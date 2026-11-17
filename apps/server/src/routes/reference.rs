use axum::{
    Json, Router,
    extract::State,
    http::HeaderMap,
    response::{IntoResponse, Response},
    routing::get,
};
use sqlx::PgPool;

use crate::error::AppError;
use crate::queries;
use crate::routes::caching::{CATALOG_CACHE_CONTROL, Freshness};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/reference", get(reference))
}

// GET /api/reference — the five reference tables, fetched at boot so any
// screen resolves names without the Catalog (ADR-0021). They change only with
// the Catalog, so they share its validator and its caching.
async fn reference(headers: HeaderMap, State(pool): State<PgPool>) -> Result<Response, AppError> {
    // Validator before body, deliberately: a sync committing in between leaves
    // a body newer than its ETag, which costs one extra 200 later. The other
    // order would pin an old body under a new ETag until the next change.
    let freshness = Freshness::load(&pool).await?;
    if freshness.is_current_for(&headers) {
        return Ok(freshness.not_modified(CATALOG_CACHE_CONTROL));
    }
    let reference = queries::fetch_reference(&pool).await?;
    Ok((freshness.headers(CATALOG_CACHE_CONTROL), Json(reference)).into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use axum::http::{StatusCode, header};
    use serde_json::{Value, json};

    async fn seed(pool: &PgPool) -> sqlx::Result<()> {
        sqlx::query!(
            "INSERT INTO categories (category, ordinal) VALUES ('maimai', 1), ('POPS', 0)"
        )
        .execute(pool)
        .await?;
        sqlx::query!(
            "INSERT INTO versions (version, abbr, release_date, ordinal)
             VALUES ('maimai DX', 'DX', '2019-07-11', 0)"
        )
        .execute(pool)
        .await?;
        sqlx::query!(
            "INSERT INTO types (type, name, abbr, icon_url, icon_height, ordinal)
             VALUES ('dx', 'DX', 'DX', 'https://icons.example/dx.png', 22, 0)"
        )
        .execute(pool)
        .await?;
        sqlx::query!(
            "INSERT INTO difficulties (difficulty, name, code, color, icon_url, ordinal)
             VALUES ('master', 'MASTER', 'master', '#9e45e2', 'https://icons.example/m.png', 0),
                    ('【協】', '【協】', '協', NULL, NULL, 1)"
        )
        .execute(pool)
        .await?;
        sqlx::query!("INSERT INTO regions (region, name, ordinal) VALUES ('jp', 'Japan', 0)")
            .execute(pool)
            .await?;
        sqlx::query!("INSERT INTO catalog_meta (id, update_time) VALUES (true, now())")
            .execute(pool)
            .await?;
        Ok(())
    }

    async fn body(response: Response) -> Value {
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap()
    }

    #[sqlx::test]
    async fn reference_carries_codes_and_read_fields_only(pool: PgPool) -> sqlx::Result<()> {
        seed(&pool).await?;
        let response = reference(HeaderMap::new(), State(pool)).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Exactly these keys: no icons, no release dates (ADR-0026), and
        // difficulties keyed by `code`, UTAGE included.
        assert_eq!(
            body(response).await,
            json!({
                "categories": [{ "category": "POPS" }, { "category": "maimai" }],
                "versions": [{ "version": "maimai DX", "abbr": "DX" }],
                "types": [{ "type": "dx", "name": "DX", "abbr": "DX" }],
                "difficulties": [
                    { "code": "master", "name": "MASTER", "color": "#9e45e2" },
                    { "code": "協", "name": "【協】", "color": null }
                ],
                "regions": [{ "region": "jp", "name": "Japan" }]
            })
        );
        Ok(())
    }

    #[sqlx::test]
    async fn reference_answers_304_while_the_catalog_has_not_changed(
        pool: PgPool,
    ) -> sqlx::Result<()> {
        seed(&pool).await?;
        let first = reference(HeaderMap::new(), State(pool.clone()))
            .await
            .unwrap();
        let etag = first.headers()[header::ETAG].clone();
        assert_eq!(
            first.headers()[header::CACHE_CONTROL],
            CATALOG_CACHE_CONTROL
        );

        let mut conditional = HeaderMap::new();
        conditional.insert(header::IF_NONE_MATCH, etag);
        let second = reference(conditional, State(pool)).await.unwrap();
        assert_eq!(second.status(), StatusCode::NOT_MODIFIED);
        // A 304 repeats the validator and the caching it revalidated.
        assert_eq!(
            second.headers()[header::ETAG],
            first.headers()[header::ETAG]
        );
        assert_eq!(
            second.headers()[header::CACHE_CONTROL],
            CATALOG_CACHE_CONTROL
        );
        Ok(())
    }

    #[sqlx::test]
    async fn a_new_catalog_revision_moves_the_reference_etag(pool: PgPool) -> sqlx::Result<()> {
        seed(&pool).await?;
        let before = reference(HeaderMap::new(), State(pool.clone()))
            .await
            .unwrap();
        let old_etag = before.headers()[header::ETAG].clone();
        sqlx::query!("UPDATE catalog_meta SET revision = revision + 1")
            .execute(&pool)
            .await?;

        let mut stale = HeaderMap::new();
        stale.insert(header::IF_NONE_MATCH, old_etag.clone());
        let after = reference(stale, State(pool)).await.unwrap();
        assert_eq!(
            after.status(),
            StatusCode::OK,
            "a stale validator gets the body"
        );
        assert_ne!(after.headers()[header::ETAG], old_etag);
        Ok(())
    }
}
