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

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        // Never print the connection string — it carries the password.
        eprintln!("migrate: failed: {e}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn Error>> {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").map_err(|_| "DATABASE_URL is not set")?;

    // One connection is enough, and a bounded timeout means an unreachable
    // database fails the deploy promptly instead of hanging the release step.
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(30))
        .connect(&database_url)
        .await?;

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
