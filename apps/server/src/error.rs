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
    NotFound { kind: &'static str, key: String },
    BadRequest(String),
    /// `/sync/delta`'s `since` predates the last full reload (contract §3).
    SnapshotRequired,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::Database(e) => {
                // TODO(05): replace with `tracing::error!` once the
                // subscriber lands — this is the one place every sqlx
                // failure passes through.
                eprintln!("database error: {e}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": "database_error", "message": "internal error" })),
                )
                    .into_response()
            }
            AppError::NotFound { kind, key } => (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "not_found", "message": format!("{kind} '{key}' not found") })),
            )
                .into_response(),
            AppError::BadRequest(message) => (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "bad_request", "message": message })),
            )
                .into_response(),
            AppError::SnapshotRequired => (
                StatusCode::CONFLICT,
                Json(json!({ "error": "snapshot_required" })),
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
    async fn snapshot_required_is_409_with_no_message_field() {
        let response = AppError::SnapshotRequired.into_response();
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert_eq!(
            body_json(response).await,
            json!({ "error": "snapshot_required" })
        );
    }
}
