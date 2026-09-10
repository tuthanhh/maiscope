mod config;
mod domain;
mod error;
mod queries;
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

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(config.database_max_connections)
        .connect(&config.database_url)
        .await?;

    let port = config.port;
    let state = AppState {
        pool,
        config: Arc::new(config),
    };

    let app = routes::router(state);

    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    println!("listening on {}", listener.local_addr()?);

    axum::serve(listener, app).await?;
    Ok(())
}
