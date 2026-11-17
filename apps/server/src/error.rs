//! Single error type for the API (ticket 02). Every handler but `get_chart`
//! returns `Result<_, AppError>` — `get_chart` is excepted by contract §2/§6:
//! it returns plain-text errors, not this JSON shape.

use axum::{
    Json,
    http::StatusCode,
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
    /// A server-side failure that isn't a database error — currently only the
    /// rate limiter's unreachable key-extraction arm. The `&'static str` names
    /// the origin for the log line; the client never sees it.
    Internal(&'static str),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::Database(e) => {
                tracing::error!(error = %e, "database query failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": "database_error", "message": "internal error" })),
                )
                    .into_response()
            }
            AppError::NotFound { kind, key } => (
                StatusCode::NOT_FOUND,
                Json(
                    json!({ "error": "not_found", "message": format!("{kind} '{key}' not found") }),
                ),
            )
                .into_response(),
            AppError::BadRequest(message) => (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "bad_request", "message": message })),
            )
                .into_response(),
            // RFC 6750 §3.1: a request with no credential gets the bare
            // scheme; one whose token was refused also gets
            // `error="invalid_token"`. The body is the same shape for both.
            AppError::Unauthorized(message) => (
                StatusCode::UNAUTHORIZED,
                [(axum::http::header::WWW_AUTHENTICATE, "Bearer")],
                Json(json!({ "error": "unauthorized", "message": message })),
            )
                .into_response(),
            AppError::InvalidToken(message) => (
                StatusCode::UNAUTHORIZED,
                [(
                    axum::http::header::WWW_AUTHENTICATE,
                    r#"Bearer error="invalid_token""#,
                )],
                Json(json!({ "error": "unauthorized", "message": message })),
            )
                .into_response(),
            AppError::SnapshotRequired => (
                StatusCode::CONFLICT,
                Json(json!({ "error": "snapshot_required" })),
            )
                .into_response(),
            AppError::BadGateway(context) => {
                tracing::error!(context, "upstream call failed");
                (
                    StatusCode::BAD_GATEWAY,
                    Json(
                        json!({ "error": "upstream_error", "message": "upstream service failed" }),
                    ),
                )
                    .into_response()
            }
            AppError::Internal(context) => {
                tracing::error!(context, "internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": "internal_error", "message": "internal error" })),
                )
                    .into_response()
            }
            AppError::RateLimited { retry_after_secs } => (
                StatusCode::TOO_MANY_REQUESTS,
                [(
                    axum::http::header::RETRY_AFTER,
                    retry_after_secs.to_string(),
                )],
                Json(json!({
                    "error": "rate_limited",
                    "message": format!("rate limit exceeded, retry after {retry_after_secs}s")
                })),
            )
                .into_response(),
        }
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
}
