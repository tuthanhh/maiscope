use axum::{Json, Router, routing::get};
use serde_json::{Value, json};

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/healthcheck", get(healthcheck))
}

async fn healthcheck() -> Json<Value> {
    Json(json!({ "status": "good" }))
}
