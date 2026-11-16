# 06 — Custom domain, with `/api/*` proxied to Fly

**What to build:** put the app and the API on one origin. Buy a domain, point
it at the Pages project, and proxy `/api/*` to `maiscope-api.fly.dev` from
Cloudflare so the browser never sees two origins.

Deferred from [ticket 03](03-cloudflare-pages.md), where the reasoning is
recorded; this ticket exists so the deferral has a home and its one real
consequence is written down before anyone promotes the app.

**Why it is worth doing** (ticket 03's estimate: ~$10/yr):

- **CORS stops existing.** `CORS_ALLOWED_ORIGINS` in `fly.toml`, the
  `CorsLayer` in `routes/mod.rs`, the `expose_headers` for `X-Total-Count`
  and the test that guards it all become same-origin no-ops. The v2 API adds
  `POST` for uploads ([`production-v2`](../../production-v2/spec.md)), which
  is a second CORS surface to get right — or none, on one origin.
- **A Cache Rule can serve `/catalog` from the edge.** Today every cold visitor
  pays a Fly scale-to-zero start for a 3MB payload that changes at most once a
  day. Cached at the edge on the server's own `Cache-Control: max-age=3600`,
  most visitors never reach Fly at all.
- **The PWA's install identity becomes permanent.** See the trap.

**The trap, from ticket 03:** a PWA's install identity is its origin. Anyone
who installs from `maiscope.pages.dev` *before* this ticket keeps a stale app
pointed at the old origin after the move — no update prompt, no redirect
reaches an installed PWA's manifest. **Do not promote installs until this
ticket is done.** Installs that already exist on `pages.dev` are orphaned by
this change; count them (Cloudflare analytics on `manifest.webmanifest`
fetches is the cheapest proxy) before deciding whether `pages.dev` needs to
keep serving a "reinstall from the new address" page for a while.

**Blocked by:** 03 (shipped). Nothing technical — this is a purchase and a
Cloudflare configuration, then one line in `deploy-web.yml`.

**Status:** dropped — 2026-11-17, maintainer's call: `maiscope.pages.dev`
is the permanent origin. See Comments for what that settles.

- [ ] Domain registered; DNS on Cloudflare
- [ ] Pages project bound to the domain; `maiscope.pages.dev` kept as an alias
      until the orphan question below is answered
- [ ] `/api/*` proxied to the Fly app. Worker route or origin rule — pick the
      one that forwards `Fly-Client-IP` (`rate_limit.rs` keys on it; behind a
      second proxy the fallback is `X-Forwarded-For`, which must then carry
      the real client, not Cloudflare's edge)
- [ ] `VITE_API_BASE_URL` in `deploy-web.yml` becomes the relative `/api/v1`
- [ ] Cache Rule on `/api/v1/catalog` honouring the server's `Cache-Control`
      and `ETag`; `/sync/manifest` is `no-cache` and must stay uncached
- [ ] `CORS_ALLOWED_ORIGINS` reduced to the dev server only; the production
      origin entry removed once nothing cross-origin remains
- [ ] Existing `pages.dev` installs counted and a decision recorded on whether
      the old origin serves a redirect page, and for how long
- [ ] ROADMAP "do not promote installs" note cleared

## Comments

**Dropped 2026-11-17.** The maintainer chose not to buy a domain. What that
settles, so nobody re-derives it:

- **The PWA install trap closes by itself.** The origin never moves, so
  installs from `maiscope.pages.dev` are never orphaned. The "do not promote
  installs" note in ticket 03 and the spec is cleared.
- **CORS stays.** `CORS_ALLOWED_ORIGINS` and the `CorsLayer` remain live
  configuration; the v2 upload endpoint adds its `POST` to the same
  allowlist.
- **No edge cache for `/catalog`.** Cold visitors keep paying Fly's
  scale-to-zero start. The IndexedDB cache (ticket 02 stage 2) makes that a
  first-visit-only cost, which is why this is acceptable.
- **The rate limiter needs no second proxy header.** `server-restructure` 10
  shipped the trust list anyway (ADR-0019); on Fly it holds `fly-client-ip`.
