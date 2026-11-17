//! Fixture helpers shared by the binary's test modules.

use std::sync::Arc;

use sqlx::PgPool;

use crate::config::{Config, GitHubConfig, Secret};
use crate::state::AppState;

/// The JWT secret every test `Config` carries.
pub const JWT_SECRET: &str = "test-secret-test-secret-test-secret";

/// A `Config` no test needs the environment for. GitHub points nowhere
/// reachable; a test that talks to GitHub overrides `config.github` with a
/// local stand-in's address.
pub fn config() -> Config {
    Config {
        database_url: "postgres://unused".to_string(),
        database_max_connections: 1,
        port: 0,
        cors_allowed_origins: vec![],
        trusted_ip_headers: vec![],
        jwt_secret: Secret::for_test(JWT_SECRET),
        github: GitHubConfig {
            client_id: "test-client-id".to_string(),
            client_secret: Secret::for_test("test-client-secret"),
            redirect_url: "http://localhost:1420/auth/callback".to_string(),
            oauth_base: "http://127.0.0.1:9".to_string(),
            api_base: "http://127.0.0.1:9".to_string(),
        },
        log_filter: "off".to_string(),
        log_json: false,
    }
}

pub fn state(pool: PgPool, config: Config) -> AppState {
    AppState {
        pool,
        config: Arc::new(config),
        http: crate::state::http_client(),
    }
}

/// `sheets.difficulty` references `difficulties` (catalog-difficulties 02), so
/// a fixture that inserts a sheet with a difficulty seeds its row first.
/// Idempotent, so fixtures that run twice can call it each time.
pub async fn seed_difficulties(pool: &PgPool, difficulties: &[&str]) {
    sqlx::query(
        "INSERT INTO difficulties (difficulty, name, code, ordinal) \
         SELECT d, upper(d), d, (o - 1)::int FROM unnest($1::text[]) WITH ORDINALITY AS t(d, o) \
         ON CONFLICT (difficulty) DO NOTHING",
    )
    .bind(difficulties)
    .execute(pool)
    .await
    .unwrap();
}

/// The five rows upstream lists.
pub async fn seed_standard_difficulties(pool: &PgPool) {
    seed_difficulties(pool, &["basic", "advanced", "expert", "master", "remaster"]).await;
}
