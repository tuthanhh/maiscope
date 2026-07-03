//! Seed one chart's simai into the DB, keyed by sheet_expr. For testing the
//! engine path before contributions exist.
//!
//!   cargo run --bin seed_chart -- '<sheetExpr>' path/to/chart.simai
//!   cargo run --bin seed_chart -- '<sheetExpr>' -        # read simai from stdin
//!
//! sheetExpr = songId|type|difficulty, e.g. 'Oshama Scramble!|dx|master'.
//! Upserts the canonical (sheet, 'maimai-simai') chart.

use std::error::Error;
use std::io::Read;

use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenvy::dotenv().ok();

    let mut args = std::env::args().skip(1);
    let sheet_expr = args
        .next()
        .ok_or("usage: seed_chart -- '<sheetExpr>' <file|->")?;
    let source = args
        .next()
        .ok_or("usage: seed_chart -- '<sheetExpr>' <file|->")?;

    let content = if source == "-" {
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf)?;
        buf
    } else {
        std::fs::read_to_string(&source)?
    };

    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&std::env::var("DATABASE_URL")?)
        .await?;

    // Resolve the sheet; bail clearly if the expr doesn't match the catalog.
    let sheet_id: Option<i64> =
        sqlx::query_scalar("SELECT id FROM sheets WHERE sheet_expr = $1")
            .bind(&sheet_expr)
            .fetch_optional(&pool)
            .await?;
    let sheet_id = sheet_id.ok_or_else(|| {
        format!("no sheet matches sheet_expr '{sheet_expr}' (run ingest first?)")
    })?;

    // Upsert canonical chart; bump version on replace.
    sqlx::query(
        "INSERT INTO charts (sheet_id, sheet_expr, format, content, approved_at) \
         VALUES ($1, $2, 'maimai-simai', $3, now()) \
         ON CONFLICT (sheet_id, format) DO UPDATE \
           SET content = EXCLUDED.content, \
               version = charts.version + 1, \
               updated_at = now()",
    )
    .bind(sheet_id)
    .bind(&sheet_expr)
    .bind(&content)
    .execute(&pool)
    .await?;

    println!("seeded chart for '{sheet_expr}' ({} bytes)", content.len());
    Ok(())
}
