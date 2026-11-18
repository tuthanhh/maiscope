use axum::{Json, Router, routing::get};
use serde_json::{Value, json};

use crate::state::AppState;

/// Mounted outside every rate limiter: an uptime probe must not compete with
/// real traffic for a bucket.
pub fn router() -> Router<AppState> {
    Router::new().route("/healthcheck", get(healthcheck))
}

// GET /api/healthcheck
async fn healthcheck() -> Json<Value> {
    Json(json!({ "status": "good" }))
}
