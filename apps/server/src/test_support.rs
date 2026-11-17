//! Fixture helpers shared by the binary's test modules.

use sqlx::PgPool;

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
