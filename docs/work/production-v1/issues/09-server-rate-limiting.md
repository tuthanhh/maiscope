# 09 — Server: per-IP rate limiting keyed on `Fly-Client-IP`

**What to build:** A generous per-IP limiter (`tower_governor`) that only trips
on pathological traffic. Caching (issue 08) is the main defence; this caps the
blast radius of one misconfigured script pulling a 4.7MB endpoint in a loop.

**The detail that bites everyone:** behind Fly's proxy every request appears to
originate from the proxy. A naive per-IP limiter therefore rate-limits the entire
userbase as a single client. The extractor must key on the `Fly-Client-IP`
header, falling back to the socket address for local dev.

**Blocked by:** 04 (layer needs `AppState`), 07

**Status:** todo

- [ ] `tower_governor` (or equivalent) added
- [ ] Custom key extractor: `Fly-Client-IP` → `X-Forwarded-For` → peer address
- [ ] Limits chosen per-route class: generous on `/catalog`, tighter is
      unnecessary elsewhere. Numbers recorded in this ticket with reasoning.
- [ ] `429` response uses the `AppError` shape (issue 03), with `Retry-After`
- [ ] Test proving two different `Fly-Client-IP` values get independent buckets —
      this is the regression that matters
- [ ] Health check exempt from limiting
