//! Per-IP rate limiting (ticket 08). Caching (ticket 07) is the main
//! defence against a busy free-tier budget; this caps the blast radius of
//! one misconfigured script pulling a heavy endpoint in a loop.

use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use axum::extract::ConnectInfo;
use axum::http::{HeaderMap, Request};
use axum::response::IntoResponse;
use governor::middleware::NoOpMiddleware;
use tower_governor::governor::{GovernorConfig, GovernorConfigBuilder, SharedRateLimiter};
use tower_governor::key_extractor::KeyExtractor;
use tower_governor::{GovernorError, GovernorLayer};

use crate::error::AppError;

const FLY_CLIENT_IP: &str = "fly-client-ip";
const X_FORWARDED_FOR: &str = "x-forwarded-for";

/// Keys on `Fly-Client-IP` (set by Fly's edge proxy) first, `X-Forwarded-For`
/// second, and the actual peer socket address last. Behind Fly's proxy every
/// request's *peer* address is the proxy itself — keying on that naively
/// would rate-limit the entire userbase as one client. The peer-address
/// fallback exists for local dev and anything not behind Fly, where there is
/// no proxy header to trust.
#[derive(Debug, Clone, Copy)]
pub struct FlyClientIpKeyExtractor;

impl KeyExtractor for FlyClientIpKeyExtractor {
    type Key = IpAddr;

    fn name(&self) -> &'static str {
        "fly-client-ip"
    }

    fn extract<T>(&self, req: &Request<T>) -> Result<Self::Key, GovernorError> {
        let headers = req.headers();

        fly_client_ip(headers)
            .or_else(|| x_forwarded_for(headers))
            .or_else(|| peer_addr(req))
            .ok_or(GovernorError::UnableToExtractKey)
    }

    fn key_name(&self, key: &Self::Key) -> Option<String> {
        Some(key.to_string())
    }
}

fn fly_client_ip(headers: &HeaderMap) -> Option<IpAddr> {
    headers
        .get(FLY_CLIENT_IP)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.trim().parse().ok())
}

fn x_forwarded_for(headers: &HeaderMap) -> Option<IpAddr> {
    headers
        .get(X_FORWARDED_FOR)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .and_then(|s| s.trim().parse().ok())
}

fn peer_addr<T>(req: &Request<T>) -> Option<IpAddr> {
    req.extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|info| info.0.ip())
}

/// Periodically drops keyed state that has returned to a fresh quota, so the
/// limiter's per-IP map stays proportional to *active* clients rather than to
/// every IP ever seen. `governor` does no such collection itself — without
/// this the map only grows, and varying the `Fly-Client-IP` header is enough
/// to grow it on purpose.
pub(crate) fn spawn_reaper(
    limiter: SharedRateLimiter<IpAddr, NoOpMiddleware>,
    every: Duration,
) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(every).await;
            limiter.retain_recent();
            tracing::debug!(tracked_ips = limiter.len(), "reaped rate-limit state");
        }
    });
}

/// Builds the rate-limit layer and hands back the limiter behind it, which
/// the caller passes to [`spawn_reaper`] — the layer alone would leave that
/// state uncollected. `burst_size`/`period` are parameters (not hardcoded)
/// so tests can use a tiny quota instead of the production numbers — see
/// `routes::router` for those and the reasoning behind them.
pub(crate) fn layer(
    burst_size: u32,
    period: Duration,
) -> (
    GovernorLayer<FlyClientIpKeyExtractor, NoOpMiddleware, axum::body::Body>,
    SharedRateLimiter<IpAddr, NoOpMiddleware>,
) {
    let config: GovernorConfig<_, _> = GovernorConfigBuilder::default()
        .key_extractor(FlyClientIpKeyExtractor)
        .period(period)
        .burst_size(burst_size)
        .finish()
        .expect("burst_size and period are both non-zero by construction");

    // Cloned before `config` is moved into the layer — this Arc is the only
    // remaining way to reach the keyed state afterwards.
    let limiter = config.limiter().clone();

    let layer = GovernorLayer::new(config).error_handler(|error| match error {
        // tower_governor reports its wait time via `Duration::as_secs()`,
        // which truncates. At RATE_LIMIT_PERIOD (1s) the real wait is always
        // sub-second, so the raw value is 0 — a `Retry-After: 0` telling a
        // throttled client to retry immediately, which is worse than sending
        // nothing. Round up to the next whole second.
        GovernorError::TooManyRequests { wait_time, .. } => AppError::RateLimited {
            retry_after_secs: wait_time.max(1),
        }
        .into_response(),
        // Only reachable if FlyClientIpKeyExtractor itself fails, which it
        // never does in practice — it always falls back to the peer address,
        // and main.rs wires ConnectInfo so that's always available. Kept as
        // a safety net, not a path real traffic should hit.
        GovernorError::UnableToExtractKey | GovernorError::Other { .. } => {
            AppError::Internal("rate limiter failed").into_response()
        }
    });

    (layer, limiter)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::body::Body;
    use axum::http::StatusCode;
    use axum::routing::get;
    use tower::ServiceExt;

    fn request_from(ip: &str) -> Request<Body> {
        Request::builder()
            .uri("/")
            .header(FLY_CLIENT_IP, ip)
            .body(Body::empty())
            .unwrap()
    }

    #[test]
    fn extractor_prefers_fly_client_ip_over_x_forwarded_for() {
        let req = Request::builder()
            .uri("/")
            .header(FLY_CLIENT_IP, "203.0.113.9")
            .header(X_FORWARDED_FOR, "198.51.100.7")
            .body(Body::empty())
            .unwrap();

        let key = FlyClientIpKeyExtractor.extract(&req).unwrap();
        assert_eq!(key, "203.0.113.9".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn extractor_falls_back_to_x_forwarded_for_when_no_fly_header() {
        // The client's own IP is the leftmost entry — everything after it
        // was appended by proxies between the client and here.
        let req = Request::builder()
            .uri("/")
            .header(X_FORWARDED_FOR, "198.51.100.7, 10.0.0.1")
            .body(Body::empty())
            .unwrap();

        let key = FlyClientIpKeyExtractor.extract(&req).unwrap();
        assert_eq!(key, "198.51.100.7".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn extractor_falls_back_to_peer_address_when_no_headers_present() {
        // Local dev's actual case: no proxy, so no Fly-Client-IP or
        // X-Forwarded-For header exists at all.
        let mut req = Request::builder().uri("/").body(Body::empty()).unwrap();
        req.extensions_mut()
            .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 54321))));

        let key = FlyClientIpKeyExtractor.extract(&req).unwrap();
        assert_eq!(key, IpAddr::from([127, 0, 0, 1]));
    }

    #[test]
    fn extractor_fails_when_nothing_is_available() {
        let req = Request::builder().uri("/").body(Body::empty()).unwrap();
        assert!(matches!(
            FlyClientIpKeyExtractor.extract(&req),
            Err(GovernorError::UnableToExtractKey)
        ));
    }

    // Without a reaper, governor's keyed state keeps one entry per distinct
    // IP forever — an unbounded leak on exactly the free-tier VM the rate
    // limiter exists to protect, and one an attacker grows just by varying
    // the header. This drives the real `spawn_reaper` task rather than
    // calling `retain_recent` inline, so it fails if the task is never
    // spawned or never loops.
    #[tokio::test]
    async fn spawned_reaper_reclaims_stale_per_ip_state() {
        // A short period keeps the test quick: governor only considers an
        // entry reclaimable once its quota has been fully back at a fresh
        // state for another whole period, so the wait below has to clear
        // 2 x period.
        let period = Duration::from_millis(200);
        let (limit_layer, limiter) = layer(1, period);
        let app = Router::new()
            .route("/", get(|| async { "ok" }))
            .layer(limit_layer);

        for octet in 1..=5u8 {
            let ip = format!("203.0.113.{octet}");
            app.clone().oneshot(request_from(&ip)).await.unwrap();
        }
        assert_eq!(limiter.len(), 5, "one keyed entry per distinct IP");

        spawn_reaper(limiter.clone(), Duration::from_millis(50));
        tokio::time::sleep(period * 4).await;

        assert_eq!(
            limiter.len(),
            0,
            "entries back at a fresh state must be reclaimed, not retained forever"
        );
    }

    // Uses the *production* period (RATE_LIMIT_PERIOD, 1s), not the long one
    // the bucket test uses. tower_governor reports its wait time as whole
    // seconds (`Duration::as_secs()`), so at a 1s refill the real sub-second
    // wait truncates to 0 — a `Retry-After: 0` telling a throttled client to
    // retry immediately. Any period under two seconds hits this; the bucket
    // test's 60s period cannot see it.
    #[tokio::test]
    async fn retry_after_is_at_least_one_second_at_the_production_period() {
        let app = Router::new()
            .route("/", get(|| async { "ok" }))
            .layer(layer(1, Duration::from_secs(1)).0);

        assert_eq!(
            app.clone().oneshot(request_from("203.0.113.5")).await.unwrap().status(),
            StatusCode::OK
        );

        let throttled = app.oneshot(request_from("203.0.113.5")).await.unwrap();
        assert_eq!(throttled.status(), StatusCode::TOO_MANY_REQUESTS);

        let retry_after: u64 = throttled
            .headers()
            .get(axum::http::header::RETRY_AFTER)
            .expect("429 must carry Retry-After")
            .to_str()
            .unwrap()
            .parse()
            .unwrap();
        assert!(
            retry_after >= 1,
            "Retry-After: {retry_after} tells the client to retry immediately"
        );
    }

    // The regression that matters: behind Fly, every request's peer address
    // is the proxy. If the key extractor ever regresses to keying on that
    // instead of Fly-Client-IP, this test starts failing because both IPs
    // would collapse into one bucket.
    #[tokio::test]
    async fn different_fly_client_ip_values_get_independent_buckets() {
        // burst_size 1: the second request from the same IP within the
        // period is rate-limited, keeping this test fast (no real waiting).
        let app = Router::new()
            .route("/", get(|| async { "ok" }))
            .layer(layer(1, Duration::from_secs(60)).0);

        let first = app.clone().oneshot(request_from("203.0.113.1")).await.unwrap();
        assert_eq!(first.status(), StatusCode::OK);

        let second = app.clone().oneshot(request_from("203.0.113.1")).await.unwrap();
        assert_eq!(second.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(second.headers().get(axum::http::header::RETRY_AFTER).is_some());

        // A different IP has its own bucket — still succeeds even though
        // .1's bucket is exhausted.
        let third = app.clone().oneshot(request_from("203.0.113.2")).await.unwrap();
        assert_eq!(third.status(), StatusCode::OK);
    }
}
