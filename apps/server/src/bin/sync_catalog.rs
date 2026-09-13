//! Differential catalog sync: fetch upstream, sanity-check, apply the diff.
//!
//!   cargo run --bin sync_catalog
//!
//! Runs daily from .github/workflows/sync-catalog.yml. Unlike the `ingest` it
//! replaced, this never truncates and never deletes a song or sheet, so chart
//! text survives — see docs/work/catalog-sync/spec.md.

use std::error::Error;
use std::time::Duration;

use server::catalog_sync::{apply, sanity_check};
use server::upstream::RawData;
use sqlx::postgres::PgPoolOptions;

const DATA_URL: &str = "https://dp4p6x0xfi5o9.cloudfront.net/maimai/data.json";

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("sync_catalog: failed: {e}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn Error>> {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").map_err(|_| "DATABASE_URL is not set")?;

    println!("sync_catalog: fetching {DATA_URL}");
    let body = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()?
        .get(DATA_URL)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    let data: RawData = serde_json::from_str(&body)?;
    println!("sync_catalog: parsed {} songs", data.songs.len());

    let pool = PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(30))
        .connect(&database_url)
        .await?;

    let current: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM songs")
        .fetch_one(&pool)
        .await?;
    sanity_check(data.songs.len(), current)?;

    let stats = apply(&pool, &data).await?;
    println!(
        "sync_catalog: revision {} — songs +{} ~{} vanished {} skipped {}, sheets +{} ~{} vanished {}",
        stats.revision,
        stats.songs_inserted,
        stats.songs_updated,
        stats.songs_vanished,
        stats.songs_skipped_no_id,
        stats.sheets_inserted,
        stats.sheets_updated,
        stats.sheets_vanished,
    );

    Ok(())
}
