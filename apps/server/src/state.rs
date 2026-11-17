//! Router state (ticket 03). Bundles the pool with `Config` so future
//! additions (an HTTP client, a cache) don't mean editing every handler
//! signature again.
//!
//! `http` is the one outbound client, for GitHub (server-auth-github-oauth
//! 03). `reqwest::Client` is an `Arc` inside, so cloning the state shares its
//! connection pool rather than opening a new one.

use std::sync::Arc;

use axum::extract::FromRef;
use sqlx::PgPool;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Arc<Config>,
    pub http: reqwest::Client,
}

/// How long a GitHub call may take before the login fails with a 502. The
/// browser is waiting on it.
const OUTBOUND_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

/// The outbound client. Redirects are never followed: an OAuth token endpoint
/// that redirects is a misconfiguration or an attack, and following it would
/// replay the client secret to wherever it points. GitHub's REST API refuses
/// requests without a `User-Agent`.
pub fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(OUTBOUND_TIMEOUT)
        .user_agent("maiscope")
        .build()
        .expect("static reqwest configuration is valid")
}

/// Lets handlers that only want the pool keep asking for `State<PgPool>` —
/// no signature change needed until they actually need `config` too.
impl FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.pool.clone()
    }
}
