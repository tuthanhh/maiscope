mod catalog;
mod charts;
mod health;
mod sheets;
mod songs;
mod sync;

use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::Level;

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
        // Both default to DEBUG; DEFAULT_LOG_FILTER (config.rs) is
        // "info,tower_http=info", so both need bumping to INFO or they're
        // silently filtered out. make_span_with's level gates the span
        // itself (which carries method/uri) — get that one wrong and the
        // on_response event fires with no span context, so method/path
        // never show up even though status/latency do.
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        // Browser dev build (Vite) hits this cross-origin; Tauri routes through
        // src-tauri so it doesn't need CORS. Permissive is fine for local dev.
        // TODO(06): swap for an allowlist built from config.cors_allowed_origins.
        .layer(CorsLayer::permissive())
        .with_state(state)
}
