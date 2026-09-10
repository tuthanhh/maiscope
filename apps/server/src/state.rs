//! Router state (ticket 03). Bundles the pool with `Config` so future
//! additions (an HTTP client, a cache) don't mean editing every handler
//! signature again.

use std::sync::Arc;

use axum::extract::FromRef;
use sqlx::PgPool;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Arc<Config>,
}

/// Lets handlers that only want the pool keep asking for `State<PgPool>` —
/// no signature change needed until they actually need `config` too.
impl FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> Self {
        todo!("state.pool.clone() — PgPool clones cheaply, it wraps an Arc internally")
    }
}
