//! The five reference tables in their v2 shape (`api-rewrite` 02). One call
//! for all five: `GET /api/reference` and `GET /api/catalog` both want every
//! table, and never one alone.

use sqlx::PgPool;

use crate::types::{
    Reference, ReferenceCategory, ReferenceDifficulty, ReferenceRegion, ReferenceType,
    ReferenceVersion,
};

pub async fn fetch_reference(pool: &PgPool) -> Result<Reference, sqlx::Error> {
    let categories = sqlx::query_as!(
        ReferenceCategory,
        "SELECT category FROM categories ORDER BY ordinal"
    )
    .fetch_all(pool)
    .await?;
    let versions = sqlx::query_as!(
        ReferenceVersion,
        "SELECT version, abbr FROM versions ORDER BY ordinal"
    )
    .fetch_all(pool)
    .await?;
    let types = sqlx::query_as!(
        ReferenceType,
        r#"SELECT type AS "type: String", name, abbr FROM types ORDER BY ordinal"#
    )
    .fetch_all(pool)
    .await?;
    let difficulties = sqlx::query_as!(
        ReferenceDifficulty,
        "SELECT code, name, color FROM difficulties ORDER BY ordinal"
    )
    .fetch_all(pool)
    .await?;
    let regions = sqlx::query_as!(
        ReferenceRegion,
        "SELECT region, name FROM regions ORDER BY ordinal"
    )
    .fetch_all(pool)
    .await?;
    Ok(Reference {
        categories,
        versions,
        types,
        difficulties,
        regions,
    })
}
