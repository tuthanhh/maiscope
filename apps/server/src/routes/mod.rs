mod catalog;
mod charts;
mod health;
mod sheets;
mod songs;
mod sync;

use axum::Router;
use tower_http::cors::CorsLayer;

use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .nest(
            "/api/v1",
            Router::new()
                .merge(health::router())
                .merge(sync::router())
                .merge(catalog::router())
                .merge(songs::router())
                .merge(sheets::router())
                .merge(charts::router()),
        )
        // Browser dev build (Vite) hits this cross-origin; Tauri routes through
        // src-tauri so it doesn't need CORS. Permissive is fine for local dev.
        // TODO(06): swap for an allowlist built from config.cors_allowed_origins.
        .layer(CorsLayer::permissive())
        .with_state(state)
}
