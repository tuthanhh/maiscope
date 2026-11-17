mod auth;
mod caching;
mod catalog;
mod charts;
mod health;
pub(crate) mod sheets;
mod songs;
mod sync;

use std::time::Duration;

use axum::Router;
use axum::http::{HeaderValue, Method, header};
use tower_http::cors::CorsLayer;
use tower_http::trace::{DefaultOnResponse, TraceLayer};
use tracing::Level;

use crate::rate_limit;
use crate::state::AppState;

// Generous, uniform across every rate-limited route: burst of 30 requests,
// refilling 1/second after that. A legitimate browser session rarely does
// more than a handful of these per minute — repeat /catalog visits mostly
// hit the ticket-07 cache/304 path instead of a fresh fetch, and
// search-as-you-type on /sheets/search is the busiest realistic case, which
// this burst comfortably absorbs. A script looping on /catalog starts
// getting 429s after 30 requests and is capped to ~1 req/s after that — the
// "blast radius" this ticket is about, not a defence against a determined
// abuser (that's caching, ticket 07, and this being generous by design).
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

    let limited = Router::new()
        .merge(sync::router())
        .merge(catalog::router())
        .merge(songs::router())
        .merge(sheets::router())
        .merge(charts::router())
        .layer(limit_layer.clone());

    let (github_limit_layer, github_limiter) = rate_limit::layer(
        &state.config.trusted_ip_headers,
        GITHUB_RATE_LIMIT_BURST_SIZE,
        GITHUB_RATE_LIMIT_PERIOD,
    );
    rate_limit::spawn_reaper(github_limiter, RATE_LIMIT_CLEANUP_INTERVAL);

    // v2 routes live under `/api` with no version segment; the catalog joins
    // them in api-rewrite. Refresh and me share the catalog's bucket, so one
    // IP has one budget for ordinary traffic.
    //
    // `no_store` wraps the auth routes only, not the `/api` nest: the catalog
    // is cacheable and will move in beside them.
    let auth_routes = Router::new()
        .merge(auth::github_router().layer(github_limit_layer))
        .merge(auth::session_router().layer(limit_layer))
        .layer(axum::middleware::map_response(auth::no_store));
    let api = Router::new().merge(auth_routes);

    Router::new()
        .nest(
            "/api/v1",
            Router::new().merge(health::router()).merge(limited),
        )
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
        // ignores it entirely. The real cap on abuse is ticket 08's rate
        // limiting. Credentials travel in an `Authorization` header the page
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
                .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
                // Without this the browser hides `X-Total-Count` from JS —
                // `headers.get()` returns null, no error is raised, and it only
                // happens cross-origin. A same-origin test cannot catch it, so
                // `sheets::tests` exercises it through the assembled router.
                .expose_headers([sheets::TOTAL_COUNT_HEADER]),
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
                .oneshot(get_from("/api/v1/sync/manifest", ip))
                .await
                .unwrap();
            if response.status() == StatusCode::TOO_MANY_REQUESTS {
                throttled = true;
                break;
            }
        }
        assert!(throttled, "rate-limit layer is not wired to /sync/manifest");

        // Same IP, now throttled — health must still answer.
        for _ in 0..RATE_LIMIT_BURST_SIZE + 5 {
            let response = app
                .clone()
                .oneshot(get_from("/api/v1/healthcheck", ip))
                .await
                .unwrap();
            assert_eq!(
                response.status(),
                StatusCode::OK,
                "healthcheck must never be rate limited"
            );
        }
        Ok(())
    }

    // The header is useless to the frontend unless CORS exposes it, and the
    // failure is invisible: the browser drops it, `headers.get()` returns null,
    // nothing is logged, and it only happens cross-origin. Neither a handler
    // test nor a same-origin request can catch that, so this goes through the
    // assembled router with an Origin the config allows.
    #[sqlx::test]
    async fn x_total_count_is_exposed_to_cross_origin_callers(pool: PgPool) -> sqlx::Result<()> {
        let origin = "https://maiscope.pages.dev";
        let mut state = test_state(pool);
        Arc::make_mut(&mut state.config).cors_allowed_origins = vec![origin.to_string()];

        let request = Request::builder()
            .uri("/api/v1/sheets/search")
            .header("fly-client-ip", "203.0.113.90")
            .header("origin", origin)
            .body(Body::empty())
            .unwrap();

        let response = router(state).oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let exposed = response
            .headers()
            .get("access-control-expose-headers")
            .expect("CORS must expose headers, or the browser hides X-Total-Count")
            .to_str()
            .unwrap()
            .to_ascii_lowercase();
        assert!(
            exposed.contains("x-total-count"),
            "expose-headers was {exposed:?}"
        );
        assert!(response.headers().contains_key("x-total-count"));
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
            ("/api/v1/healthcheck", StatusCode::OK),
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
}
