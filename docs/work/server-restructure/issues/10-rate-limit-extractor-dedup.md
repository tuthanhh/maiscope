# 10 — Collapse the hand-rolled IP extractor onto `tower_governor`'s

**What to build:** Two related simplifications to `rate_limit.rs`, both
deferred out of the issue-08 follow-up review because the first has a
security dimension that deserves a deliberate decision rather than being
folded into a cleanup pass.

**Blocked by:** 08

**Status:** todo

- [ ] Decide whether `SmartIpKeyExtractor`'s wider trust chain is acceptable
- [ ] Collapse `FlyClientIpKeyExtractor`'s fallbacks accordingly
- [ ] Drop the direct `governor` dependency if the return type stops naming
      `NoOpMiddleware`

## 1. `FlyClientIpKeyExtractor` reimplements what the library ships

`rate_limit.rs` hand-writes the whole fallback chain:

```rust
fly_client_ip(headers)
    .or_else(|| x_forwarded_for(headers))
    .or_else(|| peer_addr(req))
    .ok_or(GovernorError::UnableToExtractKey)
```

`tower_governor::key_extractor::SmartIpKeyExtractor` already does all of it
bar the Fly header. Its chain, read from the 0.8.0 source:

```
x-forwarded-for → x-real-ip → forwarded → ConnectInfo → socket addr
```

So the whole thing could become:

```rust
fn extract<T>(&self, req: &Request<T>) -> Result<Self::Key, GovernorError> {
    match fly_client_ip(req.headers()) {
        Some(ip) => Ok(ip),
        None => SmartIpKeyExtractor.extract(req),
    }
}
```

That deletes `x_forwarded_for`, `peer_addr`, and two of the extractor's
five unit tests — roughly 40 lines.

**Why this is not a free win.** It is not a pure deletion: it starts
honouring `x-real-ip` and `Forwarded`, two headers we do not trust today.
Behind Fly this never matters — Fly's edge proxy sets `Fly-Client-IP` and
overwrites any client-supplied copy, so the first branch always wins and
nothing below it is reached. It matters *off* Fly: running the binary
directly (local dev, a bare container, any future non-Fly host), a client
can set `X-Real-IP` to whatever it likes and mint itself a fresh bucket per
request. That is already true of `X-Forwarded-For` today, so this widens an
existing hole rather than opening a new one — but it widens it.

Three ways out, in rough order of preference:

1. **Narrow delegation.** Keep `Fly-Client-IP → X-Forwarded-For` hand-written
   and use `SmartIpKeyExtractor` only for the peer-address tail. Deletes
   `peer_addr`, keeps the trust surface byte-for-byte as it is. Smallest win,
   zero behaviour change.
2. **Full delegation, and stop trusting proxy headers when not behind a
   proxy.** Gate the header branches on a config flag (`TRUST_PROXY_HEADERS`,
   default off, set true in the Fly deployment). This is the actually-correct
   shape — `tower_governor`'s own docs warn to "make absolutely sure that you
   only trust these headers when the peer IP is the IP of your reverse proxy"
   — but it is new configuration, so it belongs in a ticket, not a cleanup.
3. **Full delegation, accept the headers.** Biggest deletion, widest trust.

Option 2 is the one worth doing if this ticket gets picked up properly;
option 1 is the right move if the goal is purely to shrink the file.

## 2. The direct `governor` dependency exists only to name a type

`apps/server/Cargo.toml` declares `governor = "0.10.4"` alongside
`tower_governor`. The only thing it is used for is naming `NoOpMiddleware`
in the return type of `rate_limit::layer`:

```rust
pub(crate) fn layer(burst_size: u32, period: Duration) -> (
    GovernorLayer<FlyClientIpKeyExtractor, NoOpMiddleware, axum::body::Body>,
    SharedRateLimiter<IpAddr, NoOpMiddleware>,
)
```

Both halves of that tuple are handed straight to `Router::layer` and
`spawn_reaper`. If `layer` took the router and returned it —

```rust
pub(crate) fn limited(router: Router<AppState>, burst: u32, period: Duration)
    -> Router<AppState>
```

— nothing would need to name the middleware type, the import goes, and the
dependency can come out of `Cargo.toml` entirely. Note this also swallows
the `spawn_reaper` call, which is currently the caller's job; that is
arguably better (the reaper is not optional — forgetting it is the leak
issue 08's follow-up fixed) but it does hide a spawned task inside a
function that reads like pure construction. Worth weighing.

Keeping the version pin in step with whatever `tower_governor` resolves
internally is the ongoing cost of leaving this as-is: a `tower_governor`
bump that moves to `governor 0.11` and a stale direct pin would produce two
incompatible `NoOpMiddleware` types and a confusing type error.

## Comments

Raised by the review of the `feat/server-restructure` branch, alongside the
issue-08 defects (`Retry-After: 0`, the missing reaper, the `AppError`
bypass) and issue-09's counter bug — those were fixed in that pass; these
two were explicitly deferred here instead.
