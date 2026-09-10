# 08 — Server: per-IP rate limiting keyed on `Fly-Client-IP`

**What to build:** A generous per-IP limiter (`tower_governor`) that only trips
on pathological traffic. Caching (issue 07) is the main defence; this caps the
blast radius of one misconfigured script pulling a 4.7MB endpoint in a loop.

**The detail that bites everyone:** behind Fly's proxy every request appears to
originate from the proxy. A naive per-IP limiter therefore rate-limits the entire
userbase as a single client. The extractor must key on the `Fly-Client-IP`
header, falling back to the socket address for local dev.

**Blocked by:** 03 (layer needs `AppState`), 06

**Status:** done

- [x] `tower_governor` (or equivalent) added
- [x] Custom key extractor: `Fly-Client-IP` → `X-Forwarded-For` → peer address
- [x] Limits chosen per-route class: generous on `/catalog`, tighter is
      unnecessary elsewhere. Numbers recorded in this ticket with reasoning.
- [x] `429` response uses the `AppError` shape (issue 02), with `Retry-After`
- [x] Test proving two different `Fly-Client-IP` values get independent buckets —
      this is the regression that matters
- [x] Health check exempt from limiting

## Comments

**One uniform limit, not per-route tiers.** Read "generous on `/catalog`,
tighter is unnecessary elsewhere" as: justify why a single number works for
every rate-limited route, not "build several tiers." `/catalog` is the
heaviest endpoint (largest payload); everything else is lighter, so whatever
threshold is comfortable for `/catalog` is never a bottleneck elsewhere.

**The numbers: burst 30, refill 1/second** (`routes/mod.rs`,
`RATE_LIMIT_BURST_SIZE`/`RATE_LIMIT_PERIOD`). A legitimate browser session
rarely fires more than a handful of API calls in quick succession —
`/catalog` itself is mostly a cache/`304` hit after the first visit (ticket
07), and search-as-you-type against `/sheets/search` is the busiest
realistic case, which a 30-request burst comfortably absorbs. A script
looping on `/catalog` without waiting for responses starts getting `429`s
after 30 requests and is capped to ~1 req/s after that — capping blast
radius, not acting as the primary defence (caching, ticket 07, already is).

**Key extractor** (`rate_limit.rs`): `Fly-Client-IP` → first entry of
`X-Forwarded-For` → peer socket address via `ConnectInfo` (wired in `main.rs`
via `into_make_service_with_connect_info::<SocketAddr>()` — without that, the
peer-address fallback silently never has anything to fall back to). Falling
back straight to the peer address behind Fly's proxy would rate-limit the
entire userbase as one client — the mistake this ticket exists to avoid.

**Health exemption via router structure, not `GovernorConfigBuilder::methods()`**
(which filters by HTTP method, not path — not useful here). `routes/mod.rs`
applies the rate-limit layer only to a `limited` sub-router (sync, catalog,
songs, sheets, charts), then merges `health::router()` in afterward, outside
that layer's scope entirely.

**429 body**: `AppError::RateLimited { retry_after_secs }` — new variant,
`{"error": "rate_limited", "message": "..."}` + a `Retry-After` header,
matching every other `AppError` variant's shape. `tower_governor`'s
`GovernorLayer::error_handler` hook converts its own `GovernorError` into
this on `TooManyRequests`; the `UnableToExtractKey`/`Other` arms are
effectively unreachable (the key extractor always finds a key once
`ConnectInfo` is wired) but handled as a plain 500 rather than left to panic.

**Verified live**: hammering `/sync/manifest` (small, fast — `/catalog`
itself is too slow to actually saturate a 30-burst/1-req-s-refill bucket
with sequential `curl`, since each 3MB response takes long enough that the
bucket partially refills between requests) with one `Fly-Client-IP`: first
30 requests `200`, next 5 `429` with the shape above; a second
`Fly-Client-IP` sent immediately after still got `200` (independent
bucket); 35 rapid requests against `/healthcheck` from the *throttled* IP
all returned `200` (exemption holds even while that IP is being limited
elsewhere).

**Test** (`rate_limit.rs`): an isolated `#[tokio::test]` against a minimal
one-route `Router` with just the rate-limit layer applied and a tiny
`burst_size=1` quota — no database, no full app wiring, since the behavior
under test is purely the key extractor + bucketing, not anything
handler-specific. This is deliberately a different shape from this
codebase's usual `#[sqlx::test]` handler tests: those call handler
functions directly, bypassing the router/middleware stack entirely, which
would never exercise this layer at all.
