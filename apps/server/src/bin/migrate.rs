//! Migrator: apply pending migrations, then exit.
//!
//!   cargo run --bin migrate
//!
//! Runs as the Fly **release command** (see `fly.toml`) — a step that completes
//! before the new machine takes traffic. A failure here fails the deploy, Fly
//! aborts, and the previous version keeps serving. Running migrations at startup
//! instead would boot the new machine, panic, restart and panic again: a crash
//! loop rather than a clean rollback. It also keeps schema work off the
//! cold-start path, which matters under scale-to-zero.
//!
//! `sqlx::migrate!()` embeds `apps/server/migrations/` into the binary at compile
//! time, so the runtime image ships no SQL files and no `sqlx-cli` — the binary
//! and the schema it applies always travel together.
//!
//! **Destructive migrations are not deployed this way.** Expand-and-contract:
//! additive changes may ride a deploy, but `DROP COLUMN` / `DROP TABLE` are a
//! separate, deliberate, manual step. Postgres holds the only copy of chart text.

use std::collections::HashSet;
use std::error::Error;
use std::time::Duration;

use sqlx::postgres::PgPoolOptions;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!();

/// Whole-run budget. A release command that hangs is worse than one that fails:
/// Fly waits, the deploy neither aborts nor completes, and there is nothing in the
/// log to read. `MIGRATOR.run()` takes a Postgres advisory lock and waits
/// indefinitely if another session holds it — a connect timeout alone does not
/// bound that. Override with `MIGRATE_TIMEOUT_SECS`.
const DEFAULT_TIMEOUT_SECS: u64 = 120;

#[tokio::main]
async fn main() {
    let timeout_secs = std::env::var("MIGRATE_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_TIMEOUT_SECS);

    match tokio::time::timeout(Duration::from_secs(timeout_secs), run()).await {
        Ok(Ok(())) => {}
        // Never print the connection string — it carries the password.
        Ok(Err(e)) => {
            eprintln!("migrate: failed: {e}");
            std::process::exit(1);
        }
        Err(_) => {
            eprintln!(
                "migrate: failed: timed out after {timeout_secs}s. The last \
                 'migrate:' line above says which step hung; a stall at 'applying' \
                 usually means another session holds the migration advisory lock."
            );
            std::process::exit(1);
        }
    }
}

/// Host and database only — never the credentials.
fn redact(url: &str) -> String {
    match url.split_once('@') {
        Some((_, rest)) => rest.split('?').next().unwrap_or(rest).to_string(),
        None => "***".to_string(),
    }
}

async fn run() -> Result<(), Box<dyn Error>> {
    dotenvy::dotenv().ok();
    // Empty counts as missing — GitHub Actions substitutes an absent secret with an
    // empty string, and Fly does the same for an unset one, so `env::var` succeeds
    // and sqlx fails later with a URL-parsing message instead of naming the secret.
    //
    // `MIGRATE_DATABASE_URL`, when set, takes priority over `DATABASE_URL`. The
    // app should run against Neon's **pooled** endpoint, but `MIGRATOR.run()`
    // takes a Postgres advisory lock and transaction-mode pooling does not
    // guarantee the same backend across statements — sound only against the
    // **direct** endpoint. Until both secrets exist in every environment, an
    // unset `MIGRATE_DATABASE_URL` falls back to `DATABASE_URL` unchanged.
    let non_empty = |var: &str| match std::env::var(var) {
        Ok(url) if !url.trim().is_empty() => Some(url),
        _ => None,
    };
    let database_url = non_empty("MIGRATE_DATABASE_URL")
        .or_else(|| non_empty("DATABASE_URL"))
        .ok_or("MIGRATE_DATABASE_URL/DATABASE_URL are not set (or are empty)")?;

    // Progress lines exist so a stalled release command shows *where* it stalled.
    // Without them a hang is indistinguishable from a machine that never started.
    println!("migrate: connecting to {}", redact(&database_url));

    let pool = PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(30))
        .connect(&database_url)
        .await?;

    println!("migrate: connected");

    // Snapshot what is already applied, so the log names only what this run did.
    // On a fresh database the table does not exist yet; treat any read failure as
    // "nothing applied" — it only costs log precision, never correctness, since
    // `run()` below is the thing that actually decides what to apply.
    let before: HashSet<i64> = sqlx::query_scalar::<_, i64>("SELECT version FROM _sqlx_migrations")
        .fetch_all(&pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .collect();

    println!("migrate: applying (takes the migration advisory lock)");
    MIGRATOR.run(&pool).await?;

    // `migrate!()` yields both directions for reversible migrations; only the up
    // side was applied.
    let applied: Vec<_> = MIGRATOR
        .iter()
        .filter(|m| m.migration_type.is_up_migration() && !before.contains(&m.version))
        .collect();

    if applied.is_empty() {
        println!(
            "migrate: schema already current ({} migration(s) applied previously)",
            before.len()
        );
    } else {
        for m in &applied {
            println!("migrate: applied {} {}", m.version, m.description);
        }
        println!("migrate: {} migration(s) applied", applied.len());
    }

    Ok(())
}
