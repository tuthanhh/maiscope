//! Per-IP rate limiting (ticket 08). Caching (ticket 07) is the main
//! defence against a busy free-tier budget; this caps the blast radius of
//! one misconfigured script pulling a heavy endpoint in a loop.

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use axum::extract::ConnectInfo;
use axum::http::{HeaderMap, HeaderName, Request};
use axum::response::IntoResponse;
use governor::middleware::NoOpMiddleware;
use tower_governor::governor::{GovernorConfig, GovernorConfigBuilder, SharedRateLimiter};
use tower_governor::key_extractor::KeyExtractor;
use tower_governor::{GovernorError, GovernorLayer};

use crate::error::AppError;

/// Keys on the first configured header that carries an IP, in trust order,
/// and on the peer socket address when none does. Which headers are trusted
/// is deployment configuration (`Config::trusted_ip_headers`, ADR-0019), not
/// a constant: behind Fly's edge proxy every request's *peer* address is
/// the proxy itself, so `fly-client-ip` must be trusted there — while a bare
/// `cargo run` must trust no header at all, or any client can mint itself a
/// fresh bucket per request by setting one.
///
/// Only headers a proxy *sets* belong in the list. `X-Forwarded-For` is
/// appended to, so its first entry is whatever the client wrote; it is not
/// read here under any configuration.
#[derive(Debug, Clone)]
pub struct TrustedHeadersKeyExtractor {
    headers: Arc<[HeaderName]>,
}

impl TrustedHeadersKeyExtractor {
    /// `names` are lower-case header names already validated by `Config`.
    pub fn new(names: &[String]) -> Self {
        let headers: Vec<HeaderName> = names
            .iter()
            .map(|n| HeaderName::from_bytes(n.as_bytes()).expect("validated by Config"))
            .collect();
        Self {
            headers: headers.into(),
        }
    }
}

impl KeyExtractor for TrustedHeadersKeyExtractor {
    type Key = IpAddr;

    fn name(&self) -> &'static str {
        "trusted-ip-headers"
    }

    fn extract<T>(&self, req: &Request<T>) -> Result<Self::Key, GovernorError> {
        self.headers
            .iter()
            .find_map(|name| header_ip(req.headers(), name))
            .or_else(|| peer_addr(req))
            .ok_or(GovernorError::UnableToExtractKey)
    }

    fn key_name(&self, key: &Self::Key) -> Option<String> {
        Some(key.to_string())
    }
}

/// The header's value as one IP. A value that is not a single address (a
/// list, garbage) is treated as absent and the next header is consulted.
fn header_ip(headers: &HeaderMap, name: &HeaderName) -> Option<IpAddr> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
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
pub(crate) fn spawn_reaper(limiter: SharedRateLimiter<IpAddr, NoOpMiddleware>, every: Duration) {
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
    trusted_ip_headers: &[String],
    burst_size: u32,
    period: Duration,
) -> (
    GovernorLayer<TrustedHeadersKeyExtractor, NoOpMiddleware, axum::body::Body>,
    SharedRateLimiter<IpAddr, NoOpMiddleware>,
) {
    let config: GovernorConfig<_, _> = GovernorConfigBuilder::default()
        .key_extractor(TrustedHeadersKeyExtractor::new(trusted_ip_headers))
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
        // Only reachable if the extractor itself fails, which it never does
        // in practice — it always falls back to the peer address, and
        // main.rs wires ConnectInfo so that's always available. Kept as a
        // safety net, not a path real traffic should hit.
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

    const FLY_CLIENT_IP: &str = "fly-client-ip";

    fn fly() -> TrustedHeadersKeyExtractor {
        TrustedHeadersKeyExtractor::new(&[FLY_CLIENT_IP.to_string()])
    }

    fn request_from(ip: &str) -> Request<Body> {
        Request::builder()
            .uri("/")
            .header(FLY_CLIENT_IP, ip)
            .body(Body::empty())
            .unwrap()
    }

    fn with_peer(mut req: Request<Body>, ip: [u8; 4]) -> Request<Body> {
        req.extensions_mut()
            .insert(ConnectInfo(SocketAddr::from((ip, 54321))));
        req
    }

    #[test]
    fn a_trusted_header_beats_the_peer_address() {
        // Behind Fly the peer is the proxy; the header is the client.
        let req = with_peer(request_from("203.0.113.9"), [10, 0, 0, 1]);
        assert_eq!(
            fly().extract(&req).unwrap(),
            "203.0.113.9".parse::<IpAddr>().unwrap()
        );
    }

    #[test]
    fn headers_are_consulted_in_trust_order() {
        let ex = TrustedHeadersKeyExtractor::new(&[
            "cf-connecting-ip".to_string(),
            FLY_CLIENT_IP.to_string(),
        ]);
        let req = Request::builder()
            .uri("/")
            .header("cf-connecting-ip", "198.51.100.7")
            .header(FLY_CLIENT_IP, "203.0.113.9")
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            ex.extract(&req).unwrap(),
            "198.51.100.7".parse::<IpAddr>().unwrap()
        );

        // The first header absent: the second is used.
        assert_eq!(
            ex.extract(&request_from("203.0.113.9")).unwrap(),
            "203.0.113.9".parse::<IpAddr>().unwrap()
        );
    }

    // The point of ADR-0019: with nothing trusted, a header is just bytes.
    // A client that sets one on a bare deployment gets its socket address,
    // not a fresh bucket.
    #[test]
    fn an_untrusted_header_is_ignored() {
        let ex = TrustedHeadersKeyExtractor::new(&[]);
        let req = with_peer(request_from("203.0.113.9"), [127, 0, 0, 1]);
        assert_eq!(ex.extract(&req).unwrap(), IpAddr::from([127, 0, 0, 1]));
    }

    #[test]
    fn x_forwarded_for_is_never_read() {
        let req = with_peer(
            Request::builder()
                .uri("/")
                .header("x-forwarded-for", "198.51.100.7, 10.0.0.1")
                .body(Body::empty())
                .unwrap(),
            [127, 0, 0, 1],
        );
        assert_eq!(fly().extract(&req).unwrap(), IpAddr::from([127, 0, 0, 1]));
    }

    #[test]
    fn a_trusted_header_with_garbage_falls_through() {
        let req = with_peer(request_from("not-an-ip"), [127, 0, 0, 1]);
        assert_eq!(fly().extract(&req).unwrap(), IpAddr::from([127, 0, 0, 1]));
    }

    #[test]
    fn extractor_fails_when_nothing_is_available() {
        let req = Request::builder().uri("/").body(Body::empty()).unwrap();
        assert!(matches!(
            fly().extract(&req),
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
        let (limit_layer, limiter) = layer(&[FLY_CLIENT_IP.to_string()], 1, period);
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
            .layer(layer(&[FLY_CLIENT_IP.to_string()], 1, Duration::from_secs(1)).0);

        assert_eq!(
            app.clone()
                .oneshot(request_from("203.0.113.5"))
                .await
                .unwrap()
                .status(),
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
            .layer(layer(&[FLY_CLIENT_IP.to_string()], 1, Duration::from_secs(60)).0);

        let first = app
            .clone()
            .oneshot(request_from("203.0.113.1"))
            .await
            .unwrap();
        assert_eq!(first.status(), StatusCode::OK);

        let second = app
            .clone()
            .oneshot(request_from("203.0.113.1"))
            .await
            .unwrap();
        assert_eq!(second.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(
            second
                .headers()
                .get(axum::http::header::RETRY_AFTER)
                .is_some()
        );

        // A different IP has its own bucket — still succeeds even though
        // .1's bucket is exhausted.
        let third = app
            .clone()
            .oneshot(request_from("203.0.113.2"))
            .await
            .unwrap();
        assert_eq!(third.status(), StatusCode::OK);
    }
}
