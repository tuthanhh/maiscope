//! Single error type for the API (ticket 02). Every handler returns
//! `Result<_, AppError>` except the chart endpoints: v1's `get_chart`, which
//! predates this and returns its own plain-text errors, and v2's, which
//! returns [`PlainText`] — the same errors, rendered as text.

use axum::{
    Json,
    http::{HeaderName, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use serde_json::json;

#[derive(Debug)]
pub enum AppError {
    /// Any sqlx failure. The client gets an opaque message; the real error
    /// is logged server-side and never put in the response body.
    Database(sqlx::Error),
    NotFound {
        kind: &'static str,
        key: String,
    },

    BadRequest(String),
    /// `/sync/delta`'s `since` predates the last full reload (contract §3).
    SnapshotRequired,
    /// The request is valid but this server cannot serve it yet: a chart row
    /// holding no inline text (contract §2).
    NotImplemented(String),
    /// No credential presented (contract §4).
    Unauthorized(&'static str),
    /// A credential presented and refused: an expired or invalid JWT, a dead
    /// refresh token, a deleted account. The message distinguishes only what
    /// changes the client's next move; it never says *why* a token is invalid.
    InvalidToken(&'static str),
    /// Per-IP rate limit tripped (ticket 08). `retry_after_secs` comes from
    /// `tower_governor`'s own wait-time calculation.
    RateLimited {
        retry_after_secs: u64,
    },
    /// A service this server depends on (GitHub) failed or answered in a way
    /// it should not have. The `&'static str` names which call, for the log;
    /// the client gets an opaque body, since the detail is not theirs to act on.
    BadGateway(&'static str),
    /// A server-side failure that isn't a database error or an upstream one: a
    /// state the code believes unreachable (the rate limiter's key-extraction
    /// arm, a `users.role` outside the enum, a JWT that will not encode). The
    /// `&'static str` names the origin for the log line; the client never
    /// sees it.
    Internal(&'static str),
}

/// What an error becomes on the wire, whatever the body's format. Built in one
/// place, so the JSON and plain-text renderings cannot disagree on a status
/// or on what is redacted; the logging happens here too, once.
struct Rendered {
    status: StatusCode,
    code: &'static str,
    /// `None` only for `snapshot_required`, whose JSON body has no message.
    message: Option<String>,
    header: Option<(HeaderName, String)>,
}

impl AppError {
    fn render(self) -> Rendered {
        let plain = |status, code, message: String| Rendered {
            status,
            code,
            message: Some(message),
            header: None,
        };
        match self {
            AppError::Database(e) => {
                tracing::error!(error = %e, "database query failed");
                plain(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "database_error",
                    "internal error".into(),
                )
            }
            AppError::NotFound { kind, key } => plain(
                StatusCode::NOT_FOUND,
                "not_found",
                format!("{kind} '{key}' not found"),
            ),
            AppError::BadRequest(message) => plain(StatusCode::BAD_REQUEST, "bad_request", message),
            // RFC 6750 §3.1: a request with no credential gets the bare
            // scheme; one whose token was refused also gets
            // `error="invalid_token"`. The body is the same shape for both.
            AppError::Unauthorized(message) => Rendered {
                header: Some((header::WWW_AUTHENTICATE, "Bearer".into())),
                ..plain(StatusCode::UNAUTHORIZED, "unauthorized", message.into())
            },
            AppError::InvalidToken(message) => Rendered {
                header: Some((
                    header::WWW_AUTHENTICATE,
                    r#"Bearer error="invalid_token""#.into(),
                )),
                ..plain(StatusCode::UNAUTHORIZED, "unauthorized", message.into())
            },
            AppError::SnapshotRequired => Rendered {
                status: StatusCode::CONFLICT,
                code: "snapshot_required",
                message: None,
                header: None,
            },
            AppError::NotImplemented(message) => {
                plain(StatusCode::NOT_IMPLEMENTED, "not_implemented", message)
            }
            AppError::BadGateway(context) => {
                tracing::error!(context, "upstream call failed");
                plain(
                    StatusCode::BAD_GATEWAY,
                    "upstream_error",
                    "upstream service failed".into(),
                )
            }
            AppError::Internal(context) => {
                tracing::error!(context, "internal error");
                plain(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    "internal error".into(),
                )
            }
            AppError::RateLimited { retry_after_secs } => Rendered {
                header: Some((header::RETRY_AFTER, retry_after_secs.to_string())),
                ..plain(
                    StatusCode::TOO_MANY_REQUESTS,
                    "rate_limited",
                    format!("rate limit exceeded, retry after {retry_after_secs}s"),
                )
            },
        }
    }
}

impl Rendered {
    fn into_response_with(self, body: impl IntoResponse) -> Response {
        let mut response = (self.status, body).into_response();
        if let Some((name, value)) = self.header {
            response.headers_mut().insert(
                name,
                HeaderValue::from_str(&value).expect("error headers are ASCII"),
            );
        }
        response
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let rendered = self.render();
        let body = match &rendered.message {
            Some(message) => json!({ "error": rendered.code, "message": message }),
            None => json!({ "error": rendered.code }),
        };
        rendered.into_response_with(Json(body))
    }
}

/// An [`AppError`] rendered as `text/plain`: the message alone, with the same
/// status, headers and redaction as the JSON form. The one variant without a
/// message, `SnapshotRequired`, falls back to its code. For the chart endpoint,
/// whose success body is simai text the engine reads as text whatever the
/// status (`api-rewrite` 05). Built from an `AppError`, so a handler cannot
/// put a raw database error in front of the client by returning one.
#[derive(Debug)]
pub struct PlainText(pub AppError);

impl IntoResponse for PlainText {
    fn into_response(self) -> Response {
        let rendered = self.0.render();
        let body = rendered
            .message
            .clone()
            .unwrap_or_else(|| rendered.code.to_string());
        rendered.into_response_with(body)
    }
}

impl From<AppError> for PlainText {
    fn from(e: AppError) -> Self {
        PlainText(e)
    }
}

impl From<sqlx::Error> for PlainText {
    fn from(e: sqlx::Error) -> Self {
        PlainText(AppError::Database(e))
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        AppError::Database(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use serde_json::Value;

    async fn body_json(response: Response) -> Value {
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn database_error_hides_the_underlying_message() {
        let response = AppError::Database(sqlx::Error::RowNotFound).into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            body_json(response).await,
            json!({ "error": "database_error", "message": "internal error" })
        );
    }

    #[tokio::test]
    async fn internal_hides_its_context_from_the_client() {
        let response = AppError::Internal("rate limiter failed").into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        // Same policy as Database: the context is for the operator's logs,
        // the client gets an opaque body in the standard error shape.
        assert_eq!(
            body_json(response).await,
            json!({ "error": "internal_error", "message": "internal error" })
        );
    }

    #[tokio::test]
    async fn bad_gateway_hides_which_upstream_failed() {
        let response = AppError::BadGateway("github token exchange").into_response();
        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
        assert_eq!(
            body_json(response).await,
            json!({ "error": "upstream_error", "message": "upstream service failed" })
        );
    }

    #[tokio::test]
    async fn not_found_names_kind_and_key() {
        let response = AppError::NotFound {
            kind: "song",
            key: "nonexistent".to_string(),
        }
        .into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            body_json(response).await,
            json!({ "error": "not_found", "message": "song 'nonexistent' not found" })
        );
    }

    #[tokio::test]
    async fn bad_request_carries_the_message() {
        let response = AppError::BadRequest("bad input".to_string()).into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            body_json(response).await,
            json!({ "error": "bad_request", "message": "bad input" })
        );
    }

    #[tokio::test]
    async fn unauthorized_is_401_with_a_bearer_challenge() {
        let response = AppError::Unauthorized("missing bearer token").into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            response
                .headers()
                .get(axum::http::header::WWW_AUTHENTICATE)
                .unwrap(),
            "Bearer"
        );
        assert_eq!(
            body_json(response).await,
            json!({ "error": "unauthorized", "message": "missing bearer token" })
        );
    }

    #[tokio::test]
    async fn invalid_token_names_the_error_in_its_challenge() {
        let response = AppError::InvalidToken("token expired").into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            response
                .headers()
                .get(axum::http::header::WWW_AUTHENTICATE)
                .unwrap(),
            r#"Bearer error="invalid_token""#
        );
        assert_eq!(
            body_json(response).await,
            json!({ "error": "unauthorized", "message": "token expired" })
        );
    }

    #[tokio::test]
    async fn snapshot_required_is_409_with_no_message_field() {
        let response = AppError::SnapshotRequired.into_response();
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert_eq!(
            body_json(response).await,
            json!({ "error": "snapshot_required" })
        );
    }

    #[tokio::test]
    async fn rate_limited_is_429_with_retry_after_header_and_body() {
        let response = AppError::RateLimited {
            retry_after_secs: 42,
        }
        .into_response();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(
            response
                .headers()
                .get(axum::http::header::RETRY_AFTER)
                .unwrap(),
            "42"
        );
        assert_eq!(
            body_json(response).await,
            json!({ "error": "rate_limited", "message": "rate limit exceeded, retry after 42s" })
        );
    }

    async fn body_text(response: Response) -> String {
        String::from_utf8(
            to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap()
                .to_vec(),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn plain_text_keeps_the_status_and_the_redaction() {
        let response = PlainText(AppError::Database(sqlx::Error::PoolTimedOut)).into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert!(
            response.headers()[header::CONTENT_TYPE]
                .to_str()
                .unwrap()
                .starts_with("text/plain")
        );
        assert_eq!(body_text(response).await, "internal error");
    }

    #[tokio::test]
    async fn plain_text_carries_the_message_and_any_challenge() {
        let response = PlainText(AppError::NotFound {
            kind: "chart",
            key: "x".into(),
        })
        .into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(body_text(response).await, "chart 'x' not found");

        let response = PlainText(AppError::Unauthorized("missing bearer token")).into_response();
        assert_eq!(response.headers()[header::WWW_AUTHENTICATE], "Bearer");
        let response = PlainText(AppError::RateLimited {
            retry_after_secs: 7,
        })
        .into_response();
        assert_eq!(response.headers()[header::RETRY_AFTER], "7");

        let response = PlainText(AppError::SnapshotRequired).into_response();
        assert_eq!(body_text(response).await, "snapshot_required");
    }

    #[tokio::test]
    async fn not_implemented_is_501() {
        let response = AppError::NotImplemented("no inline text".into()).into_response();
        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
        assert_eq!(
            body_json(response).await,
            json!({ "error": "not_implemented", "message": "no inline text" })
        );
    }
}
