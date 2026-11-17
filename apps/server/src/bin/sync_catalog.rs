//! Differential catalog sync: fetch upstream, sanity-check, apply the diff.
//!
//!   cargo run --bin sync_catalog
//!
//! Runs daily from .github/workflows/sync-catalog.yml. Unlike the `ingest` it
//! replaced, this never truncates and never deletes a song or sheet, so chart
//! text survives — see docs/work/catalog-sync/spec.md.
//!
//! Exit codes: 0 synced; 1 failed, nothing written; 3 synced and committed,
//! but the run carries the rename signature (a song vanished and a song
//! appeared), which a person should look at. Under GitHub Actions the run's
//! counts go to the job summary either way (`GITHUB_STEP_SUMMARY`).

use std::error::Error;
use std::time::Duration;

use server::catalog_sync::{SyncStats, apply, sanity_check};
use server::upstream::RawData;
use sqlx::postgres::PgPoolOptions;

const DATA_URL: &str = "https://dp4p6x0xfi5o9.cloudfront.net/maimai/data.json";

/// Whole-run budget, mirroring `bin/migrate.rs`. This job is unattended, and a
/// socket that stalls mid-transaction holds row locks — including the
/// `catalog_meta` singleton every writer takes — until GitHub's 6-hour job
/// ceiling, while the workflow's `cancel-in-progress: false` queues every later
/// run behind it. The per-request HTTP timeout and the pool's acquire timeout
/// each bound one step; neither bounds the run.
///
/// 900s is now pure headroom rather than a working limit. `catalog_sync::apply`
/// is set-based, so a first sync is ~30 statements and a no-op ~23, and the run
/// is dominated by fetching 4.5MB over HTTP. Before that it was ~119k sequential
/// round trips, which at runner-to-Neon latency overran this exact budget.
/// Override with `SYNC_CATALOG_TIMEOUT_SECS`.
const DEFAULT_TIMEOUT_SECS: u64 = 900;

#[tokio::main]
async fn main() {
    let timeout_secs = std::env::var("SYNC_CATALOG_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_TIMEOUT_SECS);

    match tokio::time::timeout(Duration::from_secs(timeout_secs), run()).await {
        Ok(Ok(stats)) => {
            write_job_summary(&stats);
            if stats.rename_signature() {
                eprintln!(
                    "sync_catalog: rename signature. This run committed, but songs vanished \
                     and appeared together, which is what an upstream title correction \
                     looks like.\n  vanished: {:?}\n  appeared: {:?}\n\
                     Nothing was paired or merged. Check by hand; the alarm will not repeat \
                     for these songs.",
                    stats.songs_vanished_ids, stats.songs_inserted_ids
                );
                std::process::exit(RENAME_SIGNATURE_EXIT);
            }
        }
        Ok(Err(e)) => {
            eprintln!("sync_catalog: failed: {e}");
            append_job_summary(&format!(
                "### Catalog sync: failed, nothing written\n\n`{e}`\n"
            ));
            std::process::exit(1);
        }
        Err(_) => {
            eprintln!(
                "sync_catalog: failed: timed out after {timeout_secs}s. The last \
                 'sync_catalog:' line above says which step hung; a stall at \
                 'applying' means the transaction was open and has been rolled \
                 back, so nothing was written."
            );
            append_job_summary(&format!(
                "### Catalog sync: timed out after {timeout_secs}s, nothing written\n"
            ));
            std::process::exit(1);
        }
    }
}

/// Distinct from 1 so a reader of the workflow can tell "look at this" from
/// "this broke".
const RENAME_SIGNATURE_EXIT: i32 = 3;

/// Append the run's counts to the GitHub job summary.
fn write_job_summary(stats: &SyncStats) {
    append_job_summary(&stats.summary_markdown());
}

/// Append to the GitHub job summary, when there is one, so every run says
/// what it did, failed runs included. A failure to write it is reported but
/// does not change the exit code.
fn append_job_summary(markdown: &str) {
    let Ok(path) = std::env::var("GITHUB_STEP_SUMMARY") else {
        return;
    };
    use std::io::Write as _;
    let written = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(&path)
        .and_then(|mut f| f.write_all(markdown.as_bytes()));
    if let Err(e) = written {
        eprintln!("sync_catalog: could not write the job summary to {path}: {e}");
    }
}

async fn run() -> Result<SyncStats, Box<dyn Error>> {
    dotenvy::dotenv().ok();
    // Empty counts as missing. GitHub Actions substitutes an absent secret with an
    // empty string, so `env::var` succeeds and the failure surfaces 30 lines later
    // as sqlx's "error with configuration: relative URL without a base" — which
    // points at URL parsing rather than at the unset secret that actually caused it.
    let database_url = match std::env::var("DATABASE_URL") {
        Ok(url) if !url.trim().is_empty() => url,
        _ => return Err("DATABASE_URL is not set (or is empty)".into()),
    };

    // Progress lines exist so a timed-out run shows *where* it stalled —
    // without them a hang is indistinguishable from a runner that never started.
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

    println!("sync_catalog: connecting");
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(30))
        .connect(&database_url)
        .await?;

    let current: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM songs")
        .fetch_one(&pool)
        .await?;
    sanity_check(data.songs.len(), current)?;

    println!("sync_catalog: applying (one transaction, rolled back on any failure)");
    let stats = apply(&pool, &data).await?;
    println!(
        "sync_catalog: revision {} ({}) — songs +{} ~{} vanished {} (+{} new) skipped {}, \
         sheets +{} ~{} vanished {} (+{} new)",
        stats.revision,
        if stats.revision_advanced {
            "advanced"
        } else {
            "nothing changed, counter left where it was"
        },
        stats.songs_inserted,
        stats.songs_updated,
        stats.songs_vanished_total,
        stats.songs_vanished,
        stats.songs_skipped_no_id,
        stats.sheets_inserted,
        stats.sheets_updated,
        stats.sheets_vanished_total,
        stats.sheets_vanished,
    );

    Ok(stats)
}
