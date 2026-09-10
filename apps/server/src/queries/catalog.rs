use crate::types::{CategoryEntry, DifficultyEntry, RegionEntry, TypeEntry, VersionEntry};
use sqlx::PgPool;

pub async fn fetch_categories(pool: &PgPool) -> Result<Vec<CategoryEntry>, sqlx::Error> {
    sqlx::query_as!(
        CategoryEntry,
        r#"SELECT category FROM categories ORDER BY ordinal"#
    )
    .fetch_all(pool)
    .await
}

pub async fn fetch_versions(pool: &PgPool) -> Result<Vec<VersionEntry>, sqlx::Error> {
    sqlx::query_as!(
        VersionEntry,
        r#"SELECT version, abbr, to_char(release_date, 'YYYY-MM-DD') AS release_date
           FROM versions ORDER BY ordinal"#
    )
    .fetch_all(pool)
    .await
}

pub async fn fetch_types(pool: &PgPool) -> Result<Vec<TypeEntry>, sqlx::Error> {
    sqlx::query_as!(
        TypeEntry,
        r#"SELECT type AS "type: String", name, abbr, icon_url, icon_height
           FROM types ORDER BY ordinal"#
    )
    .fetch_all(pool)
    .await
}

pub async fn fetch_difficulties(pool: &PgPool) -> Result<Vec<DifficultyEntry>, sqlx::Error> {
    sqlx::query_as!(
        DifficultyEntry,
        r#"SELECT difficulty, name, color, icon_url, icon_height
           FROM difficulties ORDER BY ordinal"#
    )
    .fetch_all(pool)
    .await
}

pub async fn fetch_regions(pool: &PgPool) -> Result<Vec<RegionEntry>, sqlx::Error> {
    sqlx::query_as!(
        RegionEntry,
        r#"SELECT region, name FROM regions ORDER BY ordinal"#
    )
    .fetch_all(pool)
    .await
}

pub async fn fetch_update_time(pool: &PgPool) -> Result<String, sqlx::Error> {
    sqlx::query_scalar!(
        r#"SELECT to_char(update_time, 'YYYY-MM-DD') AS "update_time!" FROM catalog_meta LIMIT 1"#
    )
    .fetch_one(pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test]
    async fn fetch_categories_orders_by_ordinal(pool: PgPool) -> sqlx::Result<()> {
        sqlx::query!("INSERT INTO categories (category, ordinal) VALUES ($1, $2)", "maimai", 1)
            .execute(&pool)
            .await?;
        sqlx::query!("INSERT INTO categories (category, ordinal) VALUES ($1, $2)", "pops", 0)
            .execute(&pool)
            .await?;

        let result = fetch_categories(&pool).await.unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].category, "pops");
        assert_eq!(result[1].category, "maimai");
        Ok(())
    }

    #[sqlx::test]
    async fn fetch_versions_formats_release_date(pool: PgPool) -> sqlx::Result<()> {
        sqlx::query!(
            "INSERT INTO versions (version, abbr, release_date, ordinal) VALUES ($1, $2, $3, $4)",
            "maimai DX",
            Some("DX"),
            Some(sqlx::types::chrono::NaiveDate::from_ymd_opt(2019, 7, 11).unwrap()),
            0
        )
        .execute(&pool)
        .await?;

        let result = fetch_versions(&pool).await.unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].version, "maimai DX");
        assert_eq!(result[0].abbr.as_deref(), Some("DX"));
        assert_eq!(result[0].release_date.as_deref(), Some("2019-07-11"));
        Ok(())
    }

    #[sqlx::test]
    async fn fetch_update_time_reads_catalog_meta(pool: PgPool) -> sqlx::Result<()> {
        sqlx::query!(
            "INSERT INTO catalog_meta (id, update_time) VALUES (true, $1)",
            sqlx::types::chrono::DateTime::parse_from_rfc3339("2026-09-01T00:00:00Z")
                .unwrap()
                .with_timezone(&sqlx::types::chrono::Utc)
        )
        .execute(&pool)
        .await?;

        let result = fetch_update_time(&pool).await.unwrap();

        assert_eq!(result, "2026-09-01");
        Ok(())
    }
}
