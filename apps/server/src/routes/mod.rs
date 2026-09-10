mod catalog;
mod charts;
mod health;
mod sheets;
mod songs;
mod sync;

use std::time::Duration;

use axum::Router;
use axum::http::{HeaderValue, Method};
use tower_http::cors::CorsLayer;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::Level;

use crate::rate_limit;
use crate::state::AppState;

// Generous, uniform across every rate-limited route: burst of 30 requests,
// refilling 1/second after that. A legitimate browser session rarely does
// more than a handful of these per minute — repeat /catalog visits mostly
// hit the ticket-07 cache/304 path instead of a fresh fetch, and
// search-as-you-type on /sheets/search is the busiest realistic case, which
// this burst comfortably absorbs. A script looping on /catalog starts
// getting 429s after 30 requests and is capped to ~1 req/s after that — the
// "blast radius" this ticket is about, not a defence against a determined
// abuser (that's caching, ticket 07, and this being generous by design).
const RATE_LIMIT_BURST_SIZE: u32 = 30;
const RATE_LIMIT_PERIOD: Duration = Duration::from_secs(1);

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

    // Rate-limited routes only — healthcheck stays exempt (uptime probes
    // shouldn't compete with real traffic for a bucket) by never passing
    // through this layer at all, rather than trying to special-case it
    // inside the limiter.
    let limited = Router::new()
        .merge(sync::router())
        .merge(catalog::router())
        .merge(songs::router())
        .merge(sheets::router())
        .merge(charts::router())
        .layer(rate_limit::layer(RATE_LIMIT_BURST_SIZE, RATE_LIMIT_PERIOD));

    Router::new()
        .nest(
            "/api/v1",
            Router::new().merge(health::router()).merge(limited),
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
