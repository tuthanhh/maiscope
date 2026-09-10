mod config;
mod domain;
mod error;
mod queries;
mod rate_limit;
mod routes;
mod state;
mod types;

use std::sync::Arc;

use state::AppState;

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("startup failed: {e}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let config = config::Config::from_env()?;

    // Built from borrows only — `config` must still be whole for the
    // `Arc::new(config)` move below.
    let filter = tracing_subscriber::EnvFilter::new(&config.log_filter);
    if config.log_json {
        tracing_subscriber::fmt().with_env_filter(filter).json().init();
    } else {
        tracing_subscriber::fmt().with_env_filter(filter).init();
    }

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(config.database_max_connections)
        .connect(&config.database_url)
        .await?;

    let port = config.port;
    let state = AppState {
        pool,
        config: Arc::new(config),
    };

    // AppState::clone is cheap (PgPool and Arc<Config> both just bump a
    // refcount) — keeps `state` around to log the resolved config below.
    let app = routes::router(state.clone());

    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    // sqlx tracks applied migrations itself; the latest version is a cheap,
    // always-available proxy for "what schema is this server running against".
    let schema_version: Option<i64> =
        sqlx::query_scalar("SELECT MAX(version) FROM _sqlx_migrations")
            .fetch_one(&state.pool)
            .await?;

    tracing::info!(
        addr = %listener.local_addr()?,
        log_filter = %state.config.log_filter,
        log_json = state.config.log_json,
        database_url = %config::redact_database_url(&state.config.database_url),
        database_max_connections = state.config.database_max_connections,
        cors_allowed_origins = ?state.config.cors_allowed_origins,
        schema_version = ?schema_version,
        "listening"
    );

    // Rate limiting's key extractor (rate_limit.rs) falls back to the peer
    // address when there's no Fly-Client-IP/X-Forwarded-For header (local
    // dev, or anything not behind Fly) — that fallback only exists if the
    // server actually records connection info per request.
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await?;
    Ok(())
}
