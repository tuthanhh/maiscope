mod auth;
mod caching;
mod catalog;
mod extract;
mod health;
mod reference;
mod songs;
mod sync;

use std::time::Duration;

use axum::Router;
use axum::extract::OriginalUri;
use axum::http::{HeaderValue, Method, header};
use tower_http::cors::CorsLayer;
use tower_http::trace::{DefaultOnResponse, TraceLayer};
use tracing::Level;

use crate::error::AppError;
use crate::rate_limit;
use crate::state::AppState;

// Generous, uniform across every rate-limited route: burst of 30 requests,
// refilling 1/second after that. A legitimate browser session rarely does
// more than a handful of these per minute — repeat Catalog visits mostly hit
// the ETag/304 path (`server-restructure` 07) instead of a fresh fetch. A
// script looping on the Catalog starts getting 429s after 30 requests and is
// capped to ~1 req/s after that — the "blast radius" `server-restructure` 08
// is about, not a defence against a determined abuser (that's caching, and
// this being generous by design).
const RATE_LIMIT_BURST_SIZE: u32 = 30;
const RATE_LIMIT_PERIOD: Duration = Duration::from_secs(1);

// How often stale per-IP buckets are collected. `governor` never does this on
// its own, so without the reaper the keyed map holds an entry for every IP
// ever seen. A minute is far longer than the quota takes to refill, so a
// reaped entry is always one that has genuinely gone idle.
const RATE_LIMIT_CLEANUP_INTERVAL: Duration = Duration::from_secs(60);

// The GitHub sign-in routes get their own bucket, tighter and separate: each
// callback is an outbound token exchange (up to 10 s) and a transaction, and a
// user who has been browsing the catalog should not find sign-in throttled.
// Ten, then one per 6 s, is several sign-ins' worth for a household or an
// arcade sharing one address.
const GITHUB_RATE_LIMIT_BURST_SIZE: u32 = 10;
const GITHUB_RATE_LIMIT_PERIOD: Duration = Duration::from_secs(6);

pub fn router(state: AppState) -> Router {
    // Unparseable entries are dropped, not a startup error: a CORS
    // misconfiguration here is an availability nuisance for that one origin,
    // not a security hole (curl ignores CORS entirely, so this layer never
    // gates access to the data itself).
    let cors_origins: Vec<HeaderValue> = state
        .config
        .cors_allowed_origins
        .iter()
        .filter_map(|origin| origin.parse().ok())
        .collect();

    // Rate-limited routes only — healthcheck stays exempt (uptime probes
    // shouldn't compete with real traffic for a bucket) by never passing
    // through this layer at all, rather than trying to special-case it
    // inside the limiter.
    let (limit_layer, limiter) = rate_limit::layer(
        &state.config.trusted_ip_headers,
        RATE_LIMIT_BURST_SIZE,
        RATE_LIMIT_PERIOD,
    );
    rate_limit::spawn_reaper(limiter, RATE_LIMIT_CLEANUP_INTERVAL);

    let (github_limit_layer, github_limiter) = rate_limit::layer(
        &state.config.trusted_ip_headers,
        GITHUB_RATE_LIMIT_BURST_SIZE,
        GITHUB_RATE_LIMIT_PERIOD,
    );
    rate_limit::spawn_reaper(github_limiter, RATE_LIMIT_CLEANUP_INTERVAL);

    // The routes live under `/api` with no version segment (`api-rewrite`).
    // Refresh and me share the reads' bucket, so one IP has one budget for
    // ordinary traffic.
    //
    // `no_store` wraps the auth routes only, not the `/api` nest: the Catalog
    // and reference tables beside them are cacheable.
    let auth_routes = Router::new()
        .merge(auth::github_router().layer(github_limit_layer))
        .merge(auth::session_router().layer(limit_layer.clone()))
        .layer(axum::middleware::map_response(auth::no_store));
    let reads = Router::new()
        .merge(reference::router())
        .merge(catalog::router())
        .merge(songs::router())
        .merge(sync::router())
        .layer(limit_layer);
    let api = Router::new()
        .merge(health::router())
        .merge(reads)
        .merge(auth_routes)
        // An address that matches no route, or a route asked with the wrong
        // method, answers in the contract's shape rather than axum's empty
        // body. `OriginalUri`: inside the nest the URI has lost its `/api`.
        .fallback(|uri: OriginalUri| async move {
            AppError::NotFound {
                kind: "route",
                key: extract::quoted(uri.path()),
            }
        })
        .method_not_allowed_fallback(|| async { AppError::MethodNotAllowed });

    Router::new()
        .nest("/api", api)
        // Both default to DEBUG; DEFAULT_LOG_FILTER (config.rs) is
        // "info,tower_http=info", so both need bumping to INFO or they're
        // silently filtered out. The span's level gates the span itself —
        // get that one wrong and the on_response event fires with no span
        // context, so method/path never show up even though status/latency
        // do.
        //
        // The path, never the URI: `DefaultMakeSpan` records the query
        // string, and `/api/auth/github/callback?code=…` carries a code this
        // server can still exchange for a session. Anyone reading the logs
        // could replay it within its lifetime.
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &axum::extract::Request| {
                    tracing::info_span!(
                        "request",
                        method = %request.method(),
                        path = %request.uri().path(),
                    )
                })
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .layer(tower_http::compression::CompressionLayer::new())
        // CORS here is an egress control, not a security control — curl
        // ignores it entirely. The real cap on abuse is the rate limiting
        // (`server-restructure` 08). Credentials travel in an `Authorization` header the page
        // sets, never in cookies, so no `allow_credentials`: a listed origin
        // gains nothing a script on it could not already send.
        //
        // POST (refresh) and an `Authorization` or JSON `Content-Type`
        // header make a request non-simple, so the browser preflights it;
        // anything not listed here fails that preflight before the request
        // is ever sent (server-auth-github-oauth 04).
        .layer(
            CorsLayer::new()
                .allow_origin(cors_origins)
                .allow_methods([Method::GET, Method::POST, Method::DELETE])
                // No `expose_headers` until a route sends a header the page
                // must read: `community-charts` browse will, with
                // `X-Total-Count` — and without exposing it the browser hides
                // it from JS cross-origin, silently.
                .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE]),
        )
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use sqlx::PgPool;
    use std::sync::Arc;
    use tower::ServiceExt;

    fn test_state(pool: PgPool) -> AppState {
        // The router tests key requests by this header, as production does
        // behind Fly; `oneshot` carries no socket address.
        let mut config = crate::test_support::config();
        config.trusted_ip_headers = vec!["fly-client-ip".to_string()];
        crate::test_support::state(pool, config)
    }

    fn get_from(path: &str, ip: &str) -> Request<Body> {
        Request::builder()
            .uri(path)
            .header("fly-client-ip", ip)
            .body(Body::empty())
            .unwrap()
    }

    // Exercises the assembled router, not a hand-built one-route stack: the
    // exemption is a property of how `router()` nests health *outside* the
    // rate-limited sub-router, so only the real wiring can prove it. The
    // first half also pins that the layer is attached to real routes at all
    // — a one-route test router cannot catch it being dropped from here.
    #[sqlx::test]
    async fn healthcheck_is_exempt_while_the_same_ip_is_throttled_elsewhere(
        pool: PgPool,
    ) -> sqlx::Result<()> {
        sqlx::query!("INSERT INTO catalog_meta (id, update_time) VALUES (true, now())")
            .execute(&pool)
            .await?;
        let app = router(test_state(pool));
        let ip = "203.0.113.77";

        // Burn the bucket on a limited route.
        let mut throttled = false;
        for _ in 0..RATE_LIMIT_BURST_SIZE + 5 {
            let response = app
                .clone()
                .oneshot(get_from("/api/sync/manifest", ip))
                .await
                .unwrap();
            if response.status() == StatusCode::TOO_MANY_REQUESTS {
                throttled = true;
                break;
            }
        }
        assert!(throttled, "rate-limit layer is not wired to /sync/manifest");

        // The other reads share that bucket, so they are throttled too.
        let response = app
            .clone()
            .oneshot(get_from("/api/reference", ip))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);

        // Same IP, now throttled — the healthcheck must still answer.
        for _ in 0..RATE_LIMIT_BURST_SIZE + 5 {
            let response = app
                .clone()
                .oneshot(get_from("/api/healthcheck", ip))
                .await
                .unwrap();
            assert_eq!(
                response.status(),
                StatusCode::OK,
                "the healthcheck must never be rate limited"
            );
        }
        Ok(())
    }

    // A browser sends nothing authenticated until this preflight passes, and
    // a handler test cannot see the CORS layer at all.
    #[sqlx::test]
    async fn an_authenticated_cross_origin_call_passes_preflight(pool: PgPool) {
        let origin = "https://maiscope.pages.dev";
        let mut state = test_state(pool);
        Arc::make_mut(&mut state.config).cors_allowed_origins = vec![origin.to_string()];

        let preflight = Request::builder()
            .method("OPTIONS")
            .uri("/api/auth/refresh")
            .header("fly-client-ip", "203.0.113.91")
            .header("origin", origin)
            .header("access-control-request-method", "POST")
            .header(
                "access-control-request-headers",
                "authorization,content-type",
            )
            .body(Body::empty())
            .unwrap();
        let response = router(state).oneshot(preflight).await.unwrap();

        let header = |name: &str| {
            response
                .headers()
                .get(name)
                .unwrap_or_else(|| panic!("preflight lacks {name}"))
                .to_str()
                .unwrap()
                .to_ascii_lowercase()
        };
        assert_eq!(header("access-control-allow-origin"), origin);
        assert!(header("access-control-allow-methods").contains("post"));
        assert!(header("access-control-allow-headers").contains("authorization"));
        assert!(header("access-control-allow-headers").contains("content-type"));
    }

    // Mounted under `/api`, behind the extractor: no token, no user.
    #[sqlx::test]
    async fn auth_me_is_mounted_and_refuses_an_anonymous_caller(pool: PgPool) {
        let response = router(test_state(pool))
            .oneshot(get_from("/api/auth/me", "203.0.113.92"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[sqlx::test]
    async fn sign_in_has_its_own_bucket_and_health_stays_exempt(pool: PgPool) {
        let app = router(test_state(pool));
        let ip = "203.0.113.93";
        let login = "/api/auth/github/login?state=abcdefghijklmnopqrst";

        let mut throttled = false;
        for _ in 0..GITHUB_RATE_LIMIT_BURST_SIZE + 5 {
            let response = app.clone().oneshot(get_from(login, ip)).await.unwrap();
            if response.status() == StatusCode::TOO_MANY_REQUESTS {
                throttled = true;
                break;
            }
        }
        assert!(throttled, "the sign-in limiter is not wired");

        // Same IP: the session routes and health answer from other buckets.
        for (path, expected) in [
            ("/api/auth/me", StatusCode::UNAUTHORIZED),
            ("/api/healthcheck", StatusCode::OK),
        ] {
            let response = app.clone().oneshot(get_from(path, ip)).await.unwrap();
            assert_eq!(response.status(), expected, "{path}");
        }
    }

    #[sqlx::test]
    async fn auth_responses_are_never_cached(pool: PgPool) {
        let app = router(test_state(pool));
        for path in [
            "/api/auth/me",
            "/api/auth/github/login?state=abcdefghijklmnopqrst",
        ] {
            let response = app
                .clone()
                .oneshot(get_from(path, "203.0.113.94"))
                .await
                .unwrap();
            assert_eq!(response.headers()["cache-control"], "no-store", "{path}");
        }
    }

    thread_local! {
        /// Log output for the current thread, while a test is capturing.
        static CAPTURED: std::cell::RefCell<Option<Vec<u8>>> =
            const { std::cell::RefCell::new(None) };
    }

    /// Writes into [`CAPTURED`] when the current thread is capturing, and
    /// nowhere otherwise.
    struct ThreadCapture;

    impl std::io::Write for ThreadCapture {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            CAPTURED.with_borrow_mut(|captured| {
                if let Some(captured) = captured {
                    captured.extend_from_slice(buf);
                }
            });
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// One *global* subscriber for the whole test binary. A thread-local one
    /// (`set_default`) is not enough: tests running in parallel without it can
    /// cache a callsite's interest as "never" and hide the trace span, which
    /// made a scoped version of this test fail one run in three.
    fn capture_logs() {
        static INSTALL: std::sync::Once = std::sync::Once::new();
        INSTALL.call_once(|| {
            let subscriber = tracing_subscriber::fmt()
                .with_writer(|| ThreadCapture)
                .with_max_level(Level::INFO)
                .with_ansi(false)
                .finish();
            tracing::subscriber::set_global_default(subscriber)
                .expect("no other test installs a global subscriber");
        });
        CAPTURED.set(Some(Vec::new()));
    }

    // A code in a log line is a session for whoever reads the logs.
    #[sqlx::test]
    async fn the_request_log_never_carries_the_query_string(pool: PgPool) {
        // `#[sqlx::test]` runs on one thread, so the request logs here.
        capture_logs();

        router(test_state(pool))
            .oneshot(get_from(
                "/api/auth/github/callback?code=REPLAYABLE-CODE",
                "203.0.113.95",
            ))
            .await
            .unwrap();

        let logs = String::from_utf8(CAPTURED.take().unwrap()).unwrap();
        assert!(
            logs.contains("path=/api/auth/github/callback"),
            "the request was not logged at all: {logs}"
        );
        assert!(!logs.contains("REPLAYABLE-CODE"), "{logs}");
    }

    // Mounted as POST under `/api`, and answering in the contract's shape: a
    // route typo or a missing `post()` would 404 or 405 here.
    #[sqlx::test]
    async fn auth_refresh_is_mounted_as_post(pool: PgPool) {
        let request = Request::builder()
            .method("POST")
            .uri("/api/auth/refresh")
            .header("fly-client-ip", "203.0.113.96")
            .header("content-type", "application/json")
            .body(Body::from("not json"))
            .unwrap();
        let response = router(test_state(pool)).oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    // The allowlist is exact: an origin not on it gets no grant at all.
    #[sqlx::test]
    async fn an_unlisted_origin_gets_no_cors_grant(pool: PgPool) {
        let mut state = test_state(pool);
        Arc::make_mut(&mut state.config).cors_allowed_origins =
            vec!["https://maiscope.pages.dev".to_string()];
        let preflight = Request::builder()
            .method("OPTIONS")
            .uri("/api/auth/refresh")
            .header("fly-client-ip", "203.0.113.97")
            .header("origin", "https://evil.example")
            .header("access-control-request-method", "POST")
            .header("access-control-request-headers", "authorization")
            .body(Body::empty())
            .unwrap();
        let response = router(state).oneshot(preflight).await.unwrap();
        assert!(
            !response
                .headers()
                .contains_key("access-control-allow-origin")
        );
    }

    // Deletion through the real stack: the extractor refuses an anonymous
    // caller, and a browser's preflight for DELETE passes.
    #[sqlx::test]
    async fn delete_me_is_mounted_behind_auth_and_preflights(pool: PgPool) {
        let origin = "https://maiscope.pages.dev";
        let mut state = test_state(pool);
        Arc::make_mut(&mut state.config).cors_allowed_origins = vec![origin.to_string()];
        let app = router(state);

        let anonymous = Request::builder()
            .method("DELETE")
            .uri("/api/auth/me")
            .header("fly-client-ip", "203.0.113.98")
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(anonymous).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let preflight = Request::builder()
            .method("OPTIONS")
            .uri("/api/auth/me")
            .header("fly-client-ip", "203.0.113.98")
            .header("origin", origin)
            .header("access-control-request-method", "DELETE")
            .header("access-control-request-headers", "authorization")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(preflight).await.unwrap();
        let methods = response.headers()["access-control-allow-methods"]
            .to_str()
            .unwrap()
            .to_ascii_lowercase();
        assert!(methods.contains("delete"), "{methods}");
    }

    // Through the real stack: axum's own rejections and misses would answer
    // in plain text or with an empty body; the API answers in the contract's
    // shape.
    #[sqlx::test]
    async fn path_errors_and_misses_are_json(pool: PgPool) {
        let app = router(test_state(pool));
        for (path, status, code) in [
            // Not UTF-8 once decoded: axum's `Path` rejection.
            ("/api/songs/%FF", StatusCode::BAD_REQUEST, "bad_request"),
            // A UUID, but not the canonical spelling.
            (
                "/api/songs/01A1E99A-0000-7000-8000-000000000000",
                StatusCode::BAD_REQUEST,
                "bad_request",
            ),
            ("/api/songs/x/sheets/dx", StatusCode::NOT_FOUND, "not_found"),
            ("/api/nothing-here", StatusCode::NOT_FOUND, "not_found"),
            // `/api/v1` is gone (api-rewrite 07): a miss like any other.
            ("/api/v1/healthcheck", StatusCode::NOT_FOUND, "not_found"),
        ] {
            let response = app
                .clone()
                .oneshot(get_from(path, "203.0.113.99"))
                .await
                .unwrap();
            assert_eq!(response.status(), status, "{path}");
            let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap();
            let body: serde_json::Value = serde_json::from_slice(&bytes)
                .unwrap_or_else(|_| panic!("{path} did not answer JSON"));
            assert_eq!(body["error"], code, "{path}");
        }
    }

    #[sqlx::test]
    async fn misses_name_the_full_path(pool: PgPool) {
        let app = router(test_state(pool));
        let response = app
            .clone()
            .oneshot(get_from("/api/nothing-here", "203.0.113.100"))
            .await
            .unwrap();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["message"], "route '/api/nothing-here' not found");

        // A preflight never reaches the 405 fallback: the outer CORS layer
        // answers every OPTIONS itself.
        let preflight = Request::builder()
            .method("OPTIONS")
            .uri("/api/reference")
            .header("fly-client-ip", "203.0.113.100")
            .header("origin", "https://elsewhere.example")
            .header("access-control-request-method", "GET")
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(preflight).await.unwrap();
        assert_ne!(response.status(), StatusCode::METHOD_NOT_ALLOWED);

        let post = Request::builder()
            .method("POST")
            .uri("/api/reference")
            .header("fly-client-ip", "203.0.113.100")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(post).await.unwrap();
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["error"], "method_not_allowed");
    }

    async fn body_bytes(response: axum::response::Response) -> Vec<u8> {
        axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap()
            .to_vec()
    }

    // Through the assembled stack, compression included: the strong ETag
    // survives the encoding, and a conditional request gets the 304 with its
    // validators. Handler tests cannot see the compression layer.
    #[sqlx::test]
    async fn the_catalog_revalidates_through_the_whole_stack(pool: PgPool) {
        let app = router(test_state(pool));
        let request = |etag: Option<&str>| {
            let mut builder = Request::builder()
                .uri("/api/catalog")
                .header("fly-client-ip", "203.0.113.101")
                .header("accept-encoding", "gzip");
            if let Some(etag) = etag {
                builder = builder.header("if-none-match", etag);
            }
            builder.body(Body::empty()).unwrap()
        };
        let first = app.clone().oneshot(request(None)).await.unwrap();
        assert_eq!(first.status(), StatusCode::OK);
        assert_eq!(first.headers()["content-encoding"], "gzip");
        let etag = first.headers()["etag"].to_str().unwrap().to_string();
        assert!(etag.starts_with('"'), "a strong ETag: {etag}");

        let second = app.oneshot(request(Some(&etag))).await.unwrap();
        assert_eq!(second.status(), StatusCode::NOT_MODIFIED);
        assert_eq!(second.headers()["etag"], etag.as_str());
        assert!(second.headers().contains_key("cache-control"));
    }

    // The chart route's own rejections are plain text, like its other
    // errors: the engine reads that body as text whatever the status.
    #[sqlx::test]
    async fn a_chart_path_it_cannot_parse_is_a_plain_text_400(pool: PgPool) {
        let response = router(test_state(pool))
            .oneshot(get_from(
                "/api/songs/%FF/sheets/dx/master/chart",
                "203.0.113.102",
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert!(
            response.headers()["content-type"]
                .to_str()
                .unwrap()
                .starts_with("text/plain")
        );
    }

    // A chart row holding no inline text (blob-only) is not a chart the
    // client can load: the sheet says `hasChart: false`, and the chart route
    // answers 501 rather than an empty 200.
    #[sqlx::test]
    async fn a_blob_only_chart_is_not_a_chart_to_load(pool: PgPool) -> sqlx::Result<()> {
        crate::test_support::seed_difficulties(&pool, &["master"]).await;
        let song: sqlx::types::Uuid = sqlx::query_scalar(
            "INSERT INTO songs (song_id, title, source_index) VALUES ('s', 'S', 0) RETURNING id",
        )
        .fetch_one(&pool)
        .await?;
        sqlx::query(
            "INSERT INTO sheets (song_id_fk, sheet_expr, type, difficulty, source_index)
             VALUES ($1, 's|dx|master', 'dx', 'master', 0)",
        )
        .bind(song)
        .execute(&pool)
        .await?;
        sqlx::query(
            "INSERT INTO charts (sheet_id, sheet_expr, format, blob_url)
             SELECT id, sheet_expr, 'maimai-simai', 's3://charts/s' FROM sheets",
        )
        .execute(&pool)
        .await?;
        let app = router(test_state(pool));
        let sheet = format!("/api/songs/{song}/sheets/dx/master");

        let response = app
            .clone()
            .oneshot(get_from(&sheet, "203.0.113.103"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body: serde_json::Value = serde_json::from_slice(&body_bytes(response).await).unwrap();
        assert_eq!(body["hasChart"], false);

        let response = app
            .oneshot(get_from(&format!("{sheet}/chart"), "203.0.113.103"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
        Ok(())
    }
}
