# ADR-0019 — Which headers carry the client IP is configuration

**Status:** accepted
**Date:** 2026-11-17

## Context

The rate limiter (`server-restructure` issue 08) keys buckets by client IP.
Behind Fly's edge proxy every request's peer address is the proxy, so the
key must come from a header Fly sets, `Fly-Client-IP`. The extractor
hard-coded that header, then `X-Forwarded-For`, then the socket — the same
chain on every deployment.

That chain is wrong somewhere whatever it says. Off Fly (a bare `cargo run`,
a container, any future host) nothing sets those headers, so a client that
sends one mints itself a fresh bucket per request and the limiter protects
nothing. Behind a second proxy in front of Fly, `Fly-Client-IP` would name
that proxy and every user behind it would share one bucket. `X-Forwarded-For`
is appended to rather than set, so its first entry is client-supplied under
every topology. `tower_governor`'s own docs say to trust such headers only
when the peer is known to be the proxy. The trust chain is a fact about the
deployment, not about the code.

Issue 10 laid out three options; this records the one taken.

## Decision

**`TRUSTED_IP_HEADERS`: a comma-separated list of header names, in trust
order, read by `Config`.** The extractor takes the first listed header that
carries one IP address, and the socket address when none does. The default
is the empty list — trust nothing but the socket — which is correct for a
bare process. `fly.toml` sets `fly-client-ip`. An invalid header name fails
startup rather than silently falling back to the socket. `X-Forwarded-For`
is not read under any setting, because no setting makes its first entry
trustworthy.

## Rejected alternatives

**Keep the hand-written chain, delegate only the socket tail to
`SmartIpKeyExtractor`** (issue 10's option 1). No behaviour change, which is
the problem: the chain stays wrong off Fly.

**Delegate fully to `SmartIpKeyExtractor`** (option 3). It honours
`X-Forwarded-For`, `X-Real-IP` and `Forwarded` unconditionally. Widest trust,
least code, and exactly the configuration its own documentation warns
against.

**A boolean `TRUST_PROXY_HEADERS`.** Enough for "Fly or not", not for "which
proxy": a second proxy in front of Fly needs a different header
(`CF-Connecting-IP` for Cloudflare), so the knob has to name headers, not
switch them on.

**Drop the direct `governor` dependency** (issue 10's part 2) was weighed
and kept as-is. Its only use is naming `NoOpMiddleware` in the limiter's
type, which the reaper test needs in order to hold the limiter and assert
that stale state is collected. Hiding the type would hide the reaper, and an
unobservable reaper is how the leak issue 08 fixed got in. The cost is one
version pin to keep in step with `tower_governor`.

## Consequences

Local runs and tests that send `Fly-Client-IP` are keyed by the socket unless
they configure the header; the router tests set it explicitly. Any future
proxy in front of Fly is a one-line `fly.toml` change, not a code change.
The `api-contract.md` wording "keyed on `Fly-Client-IP`" becomes "keyed on the
configured trusted header, `Fly-Client-IP` on Fly".
