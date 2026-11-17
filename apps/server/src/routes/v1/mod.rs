//! The v1 API, served at `/api/v1` until the redesigned frontend cuts over to
//! `/api` (`api-rewrite` 07 deletes this directory). Moved aside rather than
//! edited: until then it answers exactly as it did on master.

mod catalog;
mod charts;
mod health;
pub(crate) mod sheets;
mod songs;
mod sync;

use axum::Router;

use crate::state::AppState;

/// The healthcheck, which the caller mounts outside every rate limiter.
pub fn health_router() -> Router<AppState> {
    health::router()
}

/// Every other v1 route, for the caller to put behind its limiter.
pub fn limited_router() -> Router<AppState> {
    Router::new()
        .merge(sync::router())
        .merge(catalog::router())
        .merge(songs::router())
        .merge(sheets::router())
        .merge(charts::router())
}
