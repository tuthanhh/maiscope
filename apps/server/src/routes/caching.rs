//! Conditional-request handling shared by the two endpoints that report
//! catalog freshness: `GET /catalog` (contract §1) and `GET /sync/manifest`
//! (§3). Both read the same `catalog_meta` row and derive the same validator
//! from it, so the query, the hash, and the `304` path live here once rather
//! than being kept in step by hand in two handlers.

use axum::http::{HeaderMap, HeaderName, StatusCode, header};
use axum::response::{IntoResponse, Response};
use sqlx::PgPool;

use crate::error::AppError;

/// `GET /catalog`: the catalog only changes on a `bin/ingest` run, so an
/// hour of unconditional client-side caching is safe (contract §1).
pub(crate) const CATALOG_CACHE_CONTROL: &str = "public, max-age=3600";

/// `GET /sync/manifest`: this endpoint's whole job is telling the client
/// whether the catalog changed, so it always revalidates (contract §3).
pub(crate) const MANIFEST_CACHE_CONTROL: &str = "no-cache";

/// The `catalog_meta` row both endpoints need, plus the validator derived
/// from it.
pub(crate) struct Freshness {
    /// sha256 over `revision:updateTime`. `GET /sync/manifest` reports this
    /// bare as `catalogHash`; both endpoints send it quoted as their `ETag`.
    pub hash: String,
    pub update_time: String,
    pub revision: i64,
}

impl Freshness {
    pub(crate) async fn load(pool: &PgPool) -> Result<Self, AppError> {
        let row = sqlx::query!(
            r#"SELECT to_char(update_time, 'YYYY-MM-DD') AS "update_time!", revision AS "revision!"
               FROM catalog_meta LIMIT 1"#
        )
        .fetch_one(pool)
        .await?;

        Ok(Self {
            hash: catalog_hash(row.revision, &row.update_time),
            update_time: row.update_time,
            revision: row.revision,
        })
    }

    fn etag(&self) -> String {
        format!("\"{}\"", self.hash)
    }

    /// True when the client's `If-None-Match` already names this version.
    pub(crate) fn is_current_for(&self, headers: &HeaderMap) -> bool {
        headers
            .get(header::IF_NONE_MATCH)
            .and_then(|v| v.to_str().ok())
            == Some(self.etag().as_str())
    }

    /// The validator headers every response carries — the `200` and the
    /// `304` alike.
    pub(crate) fn headers(&self, cache_control: &'static str) -> [(HeaderName, String); 2] {
        [
            (header::ETAG, self.etag()),
            (header::CACHE_CONTROL, cache_control.to_string()),
        ]
    }

    /// RFC 7232 §4.1 requires a `304` to repeat the validators the `200`
    /// would have sent: the client uses them to refresh the freshness of
    /// what it already holds. Returning a bare `304` instead means the
    /// stored copy expires again immediately and the very next visit
    /// revalidates too, which gives up most of `CATALOG_CACHE_CONTROL`.
    pub(crate) fn not_modified(&self, cache_control: &'static str) -> Response {
        (StatusCode::NOT_MODIFIED, self.headers(cache_control)).into_response()
    }
}

fn catalog_hash(revision: i64, update_time: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(format!("{revision}:{update_time}").as_bytes());
    format!("{:x}", hasher.finalize())
}
