//! "Apply a chart revision" (ticket 09): upsert `charts`, append to
//! `chart_revisions`, bump the revision the sync tier reads. One function so
//! `bin/seed_songs.rs` and phase 2's `POST /contributions/{id}/approve` (the
//! part of the write path that actually carries forward — see the ticket)
//! share it instead of each hand-rolling their own SQL.
//!
//! Note the asymmetry with `catalog_sync`, the other writer on
//! `catalog_meta.revision`: this module bumps it with `UPDATE ... RETURNING`,
//! taking the singleton's row lock at the moment it writes. `catalog_sync`
//! takes the same lock at the *start* of its transaction, because it has to
//! stamp rows with the next value long before it knows whether the run
//! changed anything. Either way the lock is what keeps the two from
//! overwriting each other.

use sha2::{Digest, Sha256};
use sqlx::{Postgres, Transaction};

/// Whether a call actually wrote. Callers that report counts (`seed_songs`)
/// need to tell a real revision from a re-seed of identical bytes — without
/// this they can only count attempts, which makes a no-op run look like it
/// rewrote everything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartRevisionOutcome {
    Applied,
    Unchanged,
}

/// Upserts the canonical chart for `(sheet_id, format)`, appends a
/// `chart_revisions` row, and — unless `content` is unchanged from what's
/// already stored — bumps `catalog_meta.revision` and stamps the sheet
/// with it.
///
/// Revision numbers are a shared sequence, not an independent per-row
/// counter: `bin/sync_catalog` draws a fresh `catalog_meta.revision` on every
/// run that changed something and stamps every changed row with that same
/// number. A client doing `GET /sync/delta?since=N` compares against that
/// shared sequence (`sheets.revision > $1`), so `sheets.revision` has to advance *within*
/// the same numbering space `catalog_meta.revision` uses — an independent
/// `sheets.revision = sheets.revision + 1` would put it in a different
/// space and could stay silently below whatever `since` a client already
/// holds, hiding the change from delta polls entirely.
///
/// Returns [`ChartRevisionOutcome::Unchanged`] without writing (no rows, no
/// revision bump) when `content` hashes to what's already stored —
/// re-seeding unchanged bytes shouldn't churn the revision counter or the
/// client-visible ETag (`GET /catalog`, contract §1) on every run.
///
/// The content hash is computed here rather than taken as a parameter:
/// it is derived entirely from `content`, and letting callers supply it
/// makes a mismatched pair — the wrong hash stored against the right
/// bytes — expressible at all, which silently breaks the no-op check above.
pub async fn apply_chart_revision(
    tx: &mut Transaction<'_, Postgres>,
    sheet_id: i64,
    sheet_expr: &str,
    format: &str,
    content: &str,
) -> Result<ChartRevisionOutcome, sqlx::Error> {
    let hash = content_hash(content);

    let unchanged: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM charts WHERE sheet_id = $1 AND format = $2 AND hash = $3)",
    )
    .bind(sheet_id)
    .bind(format)
    .bind(&hash)
    .fetch_one(&mut **tx)
    .await?;

    if unchanged {
        return Ok(ChartRevisionOutcome::Unchanged);
    }

    let chart_id: i64 = sqlx::query_scalar(
        "INSERT INTO charts (sheet_id, sheet_expr, format, content, hash, approved_at) \
         VALUES ($1, $2, $3, $4, $5, now()) \
         ON CONFLICT (sheet_id, format) DO UPDATE \
           SET content = EXCLUDED.content, \
               hash = EXCLUDED.hash, \
               version = charts.version + 1, \
               updated_at = now() \
         RETURNING id",
    )
    .bind(sheet_id)
    .bind(sheet_expr)
    .bind(format)
    .bind(content)
    .bind(&hash)
    .fetch_one(&mut **tx)
    .await?;

    sqlx::query("INSERT INTO chart_revisions (chart_id, content, hash) VALUES ($1, $2, $3)")
        .bind(chart_id)
        .bind(content)
        .bind(&hash)
        .execute(&mut **tx)
        .await?;

    let new_revision: i64 =
        sqlx::query_scalar("UPDATE catalog_meta SET revision = revision + 1 RETURNING revision")
            .fetch_one(&mut **tx)
            .await?;

    sqlx::query("UPDATE sheets SET revision = $1 WHERE id = $2")
        .bind(new_revision)
        .bind(sheet_id)
        .execute(&mut **tx)
        .await?;

    Ok(ChartRevisionOutcome::Applied)
}

fn content_hash(content: &str) -> String {
    format!("{:x}", Sha256::digest(content.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;

    async fn seed_song_with_sheet(pool: &PgPool) -> (i64, String) {
        let song_id: i64 = sqlx::query_scalar(
            "INSERT INTO songs (song_id, title, source_index) VALUES ($1, $2, $3) RETURNING id",
        )
        .bind("maimai_song")
        .bind("Example Song")
        .bind(0)
        .fetch_one(pool)
        .await
        .unwrap();

        let sheet_expr = "maimai_song|dx|master".to_string();
        let sheet_id: i64 = sqlx::query_scalar(
            "INSERT INTO sheets (song_id_fk, sheet_expr, type, difficulty, source_index) \
             VALUES ($1, $2, 'dx', 'master', 0) RETURNING id",
        )
        .bind(song_id)
        .bind(&sheet_expr)
        .fetch_one(pool)
        .await
        .unwrap();

        (sheet_id, sheet_expr)
    }

    async fn catalog_meta_revision(pool: &PgPool) -> i64 {
        sqlx::query_scalar("SELECT revision FROM catalog_meta LIMIT 1")
            .fetch_one(pool)
            .await
            .unwrap()
    }

    // The caller must be able to tell a real write from a no-op: seed_songs
    // reports "seeded N" for CI, and counting no-ops as writes makes a
    // re-run of unchanged data look like it did N times more than it did.
    #[sqlx::test]
    async fn outcome_distinguishes_a_write_from_a_no_op(pool: PgPool) -> sqlx::Result<()> {
        sqlx::query("INSERT INTO catalog_meta (id, update_time) VALUES (true, now())")
            .execute(&pool)
            .await?;
        let (sheet_id, sheet_expr) = seed_song_with_sheet(&pool).await;

        let mut tx = pool.begin().await?;
        let first =
            apply_chart_revision(&mut tx, sheet_id, &sheet_expr, "maimai-simai", "v1").await?;
        tx.commit().await?;
        assert_eq!(first, ChartRevisionOutcome::Applied);

        let mut tx = pool.begin().await?;
        let repeat =
            apply_chart_revision(&mut tx, sheet_id, &sheet_expr, "maimai-simai", "v1").await?;
        tx.commit().await?;
        assert_eq!(repeat, ChartRevisionOutcome::Unchanged);

        let mut tx = pool.begin().await?;
        let changed =
            apply_chart_revision(&mut tx, sheet_id, &sheet_expr, "maimai-simai", "v2").await?;
        tx.commit().await?;
        assert_eq!(changed, ChartRevisionOutcome::Applied);
        Ok(())
    }

    // The hash is the function's own business — callers pass content only.
    // Compared against sha256("v1") computed outside this codebase, so the
    // assertion cannot agree with a wrong implementation.
    #[sqlx::test]
    async fn content_is_hashed_with_sha256_by_the_function(pool: PgPool) -> sqlx::Result<()> {
        sqlx::query("INSERT INTO catalog_meta (id, update_time) VALUES (true, now())")
            .execute(&pool)
            .await?;
        let (sheet_id, sheet_expr) = seed_song_with_sheet(&pool).await;

        let mut tx = pool.begin().await?;
        apply_chart_revision(&mut tx, sheet_id, &sheet_expr, "maimai-simai", "v1").await?;
        tx.commit().await?;

        let stored: String = sqlx::query_scalar("SELECT hash FROM charts WHERE sheet_id = $1")
            .bind(sheet_id)
            .fetch_one(&pool)
            .await?;
        assert_eq!(
            stored,
            "3bfc269594ef649228e9a74bab00f042efc91d5acc6fbee31a382e80d42388fe"
        );
        Ok(())
    }

    #[sqlx::test]
    async fn applying_a_revision_twice_appends_two_chart_revisions_rows(
        pool: PgPool,
    ) -> sqlx::Result<()> {
        sqlx::query("INSERT INTO catalog_meta (id, update_time) VALUES (true, now())")
            .execute(&pool)
            .await?;
        let (sheet_id, sheet_expr) = seed_song_with_sheet(&pool).await;

        let mut tx = pool.begin().await?;
        apply_chart_revision(&mut tx, sheet_id, &sheet_expr, "maimai-simai", "v1").await?;
        tx.commit().await?;

        let mut tx = pool.begin().await?;
        apply_chart_revision(&mut tx, sheet_id, &sheet_expr, "maimai-simai", "v2").await?;
        tx.commit().await?;

        let chart_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM charts")
            .fetch_one(&pool)
            .await?;
        let revision_row_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM chart_revisions")
            .fetch_one(&pool)
            .await?;
        assert_eq!(
            chart_count, 1,
            "UNIQUE(sheet_id, format) — second call updates, not inserts"
        );
        assert_eq!(
            revision_row_count, 2,
            "append-only — both calls add a history row"
        );
        Ok(())
    }

    #[sqlx::test]
    async fn applying_a_revision_bumps_catalog_meta_and_sheet_revision(
        pool: PgPool,
    ) -> sqlx::Result<()> {
        sqlx::query("INSERT INTO catalog_meta (id, update_time) VALUES (true, now())")
            .execute(&pool)
            .await?;
        let (sheet_id, sheet_expr) = seed_song_with_sheet(&pool).await;
        let before = catalog_meta_revision(&pool).await;

        let mut tx = pool.begin().await?;
        apply_chart_revision(&mut tx, sheet_id, &sheet_expr, "maimai-simai", "v1").await?;
        tx.commit().await?;

        let after = catalog_meta_revision(&pool).await;
        assert_eq!(after, before + 1);

        let sheet_revision: i64 = sqlx::query_scalar("SELECT revision FROM sheets WHERE id = $1")
            .bind(sheet_id)
            .fetch_one(&pool)
            .await?;
        assert_eq!(
            sheet_revision, after,
            "sheet must share catalog_meta's numbering, not its own independent counter"
        );
        Ok(())
    }

    #[sqlx::test]
    async fn applying_unchanged_content_is_a_no_op(pool: PgPool) -> sqlx::Result<()> {
        sqlx::query("INSERT INTO catalog_meta (id, update_time) VALUES (true, now())")
            .execute(&pool)
            .await?;
        let (sheet_id, sheet_expr) = seed_song_with_sheet(&pool).await;

        let mut tx = pool.begin().await?;
        apply_chart_revision(&mut tx, sheet_id, &sheet_expr, "maimai-simai", "v1").await?;
        tx.commit().await?;
        let revision_after_first = catalog_meta_revision(&pool).await;

        // Same hash — re-seeding identical bytes shouldn't churn anything.
        let mut tx = pool.begin().await?;
        apply_chart_revision(&mut tx, sheet_id, &sheet_expr, "maimai-simai", "v1").await?;
        tx.commit().await?;

        let revision_after_second = catalog_meta_revision(&pool).await;
        let revision_row_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM chart_revisions")
            .fetch_one(&pool)
            .await?;
        assert_eq!(revision_after_second, revision_after_first);
        assert_eq!(revision_row_count, 1);
        Ok(())
    }
}
