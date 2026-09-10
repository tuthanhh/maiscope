mod catalog;
mod charts;
mod health;
mod sheets;
mod songs;
mod sync;

use axum::Router;
use axum::http::{HeaderValue, Method};
use tower_http::cors::CorsLayer;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::Level;

use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    // Unparseable entries are dropped, not a startup error: a CORS
    // misconfiguration here is an availability nuisance for that one origin,
    // not a security hole (curl ignores CORS entirely, so this layer never
    // gates access to the data itself).
    let cors_origins: Vec<HeaderValue> = state
        .config
        .cors_allowed_origins
        .iter()
        .filter_map(|origin| origin.parse().ok())
        .collect();

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
        .layer(tower_http::compression::CompressionLayer::new())
        // CORS here is an egress control, not a security control — curl
        // ignores it entirely. The real cap on abuse is ticket 08's rate
        // limiting.
        .layer(
            CorsLayer::new()
                .allow_origin(cors_origins)
                .allow_methods([Method::GET]),
        )
        .with_state(state)
}
