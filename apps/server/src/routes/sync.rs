use axum::{
    Json, Router,
    extract::State,
    http::HeaderMap,
    response::{IntoResponse, Response},
    routing::get,
};
use serde_json::json;
use sqlx::PgPool;

use crate::error::AppError;
use crate::routes::caching::{Freshness, MANIFEST_CACHE_CONTROL};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/sync/manifest", get(manifest))
}

// GET /api/sync/manifest — the freshness probe. The client compares
// `revision` with the revision its cached Catalog was fetched under and reads
// nothing else, so nothing else is sent (ADR-0026). No delta endpoint follows
// it: a changed revision means refetching the ETag'd Catalog.
async fn manifest(headers: HeaderMap, State(pool): State<PgPool>) -> Result<Response, AppError> {
    let freshness = Freshness::load(&pool).await?;
    if freshness.is_current_for(&headers) {
        return Ok(freshness.not_modified(MANIFEST_CACHE_CONTROL));
    }
    Ok((
        freshness.headers(MANIFEST_CACHE_CONTROL),
        Json(json!({ "revision": freshness.revision })),
    )
        .into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use axum::http::{StatusCode, header};
    use serde_json::Value;

    async fn body(response: Response) -> Value {
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap()
    }

    #[sqlx::test]
    async fn the_manifest_is_the_revision_alone(pool: PgPool) -> sqlx::Result<()> {
        sqlx::query!(
            "INSERT INTO catalog_meta (id, update_time, revision) VALUES (true, now(), 7)"
        )
        .execute(&pool)
        .await?;
        let response = manifest(HeaderMap::new(), State(pool)).await.unwrap();
        assert_eq!(
            response.headers()[header::CACHE_CONTROL],
            MANIFEST_CACHE_CONTROL
        );
        assert_eq!(body(response).await, json!({ "revision": 7 }));
        Ok(())
    }

    // Between `migrate` and the first sync there is no `catalog_meta` row.
    #[sqlx::test]
    async fn before_the_first_sync_the_revision_is_zero(pool: PgPool) {
        let response = manifest(HeaderMap::new(), State(pool)).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(body(response).await, json!({ "revision": 0 }));
    }

    #[sqlx::test]
    async fn an_unchanged_manifest_is_304(pool: PgPool) -> sqlx::Result<()> {
        sqlx::query!("INSERT INTO catalog_meta (id, update_time) VALUES (true, now())")
            .execute(&pool)
            .await?;
        let first = manifest(HeaderMap::new(), State(pool.clone()))
            .await
            .unwrap();
        let mut conditional = HeaderMap::new();
        conditional.insert(header::IF_NONE_MATCH, first.headers()[header::ETAG].clone());
        let second = manifest(conditional, State(pool)).await.unwrap();
        assert_eq!(second.status(), StatusCode::NOT_MODIFIED);
        assert_eq!(
            second.headers()[header::ETAG],
            first.headers()[header::ETAG]
        );
        Ok(())
    }
}
