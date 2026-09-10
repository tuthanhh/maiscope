//! Per-IP rate limiting (ticket 08). Caching (ticket 07) is the main
//! defence against a busy free-tier budget; this caps the blast radius of
//! one misconfigured script pulling a heavy endpoint in a loop.

use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode};
use axum::response::IntoResponse;
use axum::{Json, http::HeaderMap};
use governor::middleware::NoOpMiddleware;
use serde_json::json;
use tower_governor::governor::{GovernorConfigBuilder, GovernorConfig};
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

/// Builds the rate-limit layer. `burst_size`/`period` are parameters (not
/// hardcoded) so tests can use a tiny quota instead of the production
/// numbers — see `routes::router` for those and the reasoning behind them.
pub(crate) fn layer(
    burst_size: u32,
    period: Duration,
) -> GovernorLayer<FlyClientIpKeyExtractor, NoOpMiddleware, axum::body::Body> {
    let config: GovernorConfig<_, _> = GovernorConfigBuilder::default()
        .key_extractor(FlyClientIpKeyExtractor)
        .period(period)
        .burst_size(burst_size)
        .finish()
        .expect("burst_size and period are both non-zero by construction");

    GovernorLayer::new(config).error_handler(|error| match error {
        GovernorError::TooManyRequests { wait_time, .. } => AppError::RateLimited {
            retry_after_secs: wait_time,
        }
        .into_response(),
        // Only reachable if FlyClientIpKeyExtractor itself fails, which it
        // never does in practice — it always falls back to the peer address,
        // and main.rs wires ConnectInfo so that's always available. Kept as
        // a safety net, not a path real traffic should hit.
        GovernorError::UnableToExtractKey | GovernorError::Other { .. } => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "internal_error", "message": "rate limiter failed" })),
        )
            .into_response(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::body::Body;
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
            .layer(layer(1, Duration::from_secs(60)));

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
