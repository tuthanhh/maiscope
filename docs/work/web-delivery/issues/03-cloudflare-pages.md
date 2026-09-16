# 03 — Frontend hosting on Cloudflare Pages

**What to build:** Deploy the Vue build to Cloudflare Pages. Two origins for now
(`*.pages.dev` for the app, `*.fly.dev` for the API), so a CORS allowlist is
required (`server-restructure` issue 06).

Chosen over serving the SPA from Axum because Cloudflare's egress is unmetered
and Fly's is billed — with a ~20MB wasm artifact that is the difference between
free and a surprise invoice.

**Deferred, deliberately:** a custom domain with `/api/*` proxied to Fly. That
would put both on one origin (CORS stops existing entirely) and let a Cache Rule
serve `/catalog` from the edge, which would also hide most scale-to-zero cold
starts. Worth ~$10/yr later.

**The trap that comes with deferring it:** a PWA's install identity is its
origin. Anyone who installs from `*.pages.dev` before the domain move keeps a
stale app pointed at the old origin. **Do not promote installs until the custom
domain is in place.**

**Blocked by:** 01

**Status:** in-progress — the Pages deploy itself ships (`.github/workflows/deploy-web.yml`
is committed and green), but three boxes are still open: preview deployments on
PRs, Brotli confirmed on `.js`/`.wasm`, and a ticket opened for the custom-domain
migration

- [x] **Deployed by wrangler from GitHub Actions, not by Pages' git integration** —
      `.github/workflows/deploy-web.yml`. See Comments; the git-connected build was
      tried and failed
- [x] `VITE_API_BASE_URL` set to the Fly origin — in the workflow, not in Pages env
      vars, since Pages no longer runs the build
- [x] SPA fallback **not needed**: the router is on `createWebHashHistory`
      (`router.ts:40`), so every route is a fragment and the server only ever serves
      `/`. Deep links cannot 404. Revisit if the router moves to history mode
- [ ] Preview deployments on PRs (their origins must not be in the CORS allowlist
      unless deliberately added)
- [x] Production origin added to `CORS_ALLOWED_ORIGINS` in `fly.toml`
      (`https://maiscope.pages.dev`, alongside the dev server) — needs an API
      deploy to take effect, since `[env]` is baked in at deploy time
- [ ] Brotli confirmed on `.js`/`.wasm`
- [x] Recorded: the production origin is **`https://maiscope.pages.dev`**. This
      becomes the PWA install identity — do not promote installs before the custom
      domain move (see the trap above)
- [ ] Ticket opened for the custom-domain migration, with the install-orphaning
      consequence written down
- [x] **The wasm artifact fits what Pages will accept** — 17MB after the profile
      fix below, under the 25 MiB cap. Was 43MB; see Comments
- [x] `pnpm build` wired to the release wasm — the workflow runs
      `./scripts/build-wasm.sh release`, and a guard step fails the build if any
      asset exceeds 25 MiB rather than letting wrangler discover it

## Comments

**The "~20MB wasm artifact" figure is optimistic by 2×.** Measured 2026-09-13
while closing `build-and-deploy` 02:

| Artifact | Size |
|---|---|
| `apps/host/src/wasm/engine_bg.wasm` (dev build) | 140 MB |
| `apps/host/dist/assets/engine_bg-*.wasm` (what `pnpm build` emits) | 143 MB, gzip 16.4 MB |
| `target/wasm32-unknown-unknown/release/engine.wasm` | 43 MB |
| the same, gzipped | 9.5 MB |

Everything else `pnpm build` emits is negligible by comparison — the largest is
`index-*.js` at 218 kB (81 kB gzipped). The wasm is the entire problem.

**Cloudflare Pages caps a single file at 25 MiB**, and that limit applies to the
stored file, not the compressed transfer — so the 9.5 MB gzip figure does not save
it. Re-verify the cap against current Cloudflare docs before planning around it,
but at 43 MB the release build is ~70% over and the dev build is not close.

`dist/` currently holding the 140 MB dev artifact is its own finding: nothing in
the build pipeline distinguishes "what I test locally" from "what ships".
`scripts/build-wasm.sh` runs `wasm-bindgen` but no `wasm-opt`, so there is no
size-optimisation pass at all today.

**Resolved: 43MB → 17MB, and the cause was a config bug.** `engine/Cargo.toml`
carried a `[profile.release]` with `opt-level='z'`, `lto`, `codegen-units=1`,
`strip` and `panic='abort'` — **none of which ever applied.** Cargo profiles are
workspace-global, so a `[profile]` in a member manifest is ignored with a warning
printed on every build. The "release" wasm was therefore built at stock
`opt-level = 3` with no LTO and no strip.

Moved to the workspace root as `[profile.wasm-release]` (inheriting `release`), and
`build-wasm.sh release` now builds with `--profile wasm-release`. Deliberately a
separate profile rather than the root `[profile.release]`: `panic = 'abort'` would
otherwise apply to the server too, turning a panicking handler from "one killed
task" into "process death and a Fly restart".

| Artifact | Before | After |
|---|---|---|
| release wasm | 43 MB | **17 MB** |
| build time | ~1m | 2m55s (fat LTO) |

17MB clears the 25 MiB cap, so option (1) below is done and **option (2) is no
longer needed for v1.0**. Keep it in mind only if the engine grows: the margin is
8 MiB, and `bevy_kira_audio` plus a few more sprites could eat that.

Two ways out, not mutually exclusive:

1. **Shrink under the cap.** `wasm-opt -Oz`, `opt-level = "z"`, `lto = "fat"`,
   `panic = "abort"`, strip debug info, trim Bevy features. Bevy commonly sheds
   40–60% this way; plausible landing zone 15–25 MB. Tight, and one feature
   addition from breaking again.
2. **Serve the wasm from R2, keep the SPA on Pages.** No per-file cap, egress to
   Cloudflare's edge is free, and the SPA fetches it by URL. Costs one more service
   and a cache-busting story, but removes the ceiling permanently.

Do (1) regardless — it is also the user's download. Decide on (2) once the real
number is known.

**Note this does not argue for containerising the frontend.** Serving a 40MB+ wasm
from Fly means metered egress per cold visitor, which is the exact cost
[ADR-0003](../../../adr/0003-web-pwa-drop-tauri.md) and this ticket rejected. The
size problem makes Pages *more* attractive, not less.


## Deployment mechanism

**Pages' git integration cannot build this project.** Tried it; the build failed at
`vue-tsc` with the two `TS2307` errors for `~/wasm/engine.js`. The Pages
image provides `pnpm@10.33.0` and `nodejs@24.18.0` and no Rust, while
`apps/host/src/wasm/` is gitignored and produced by `scripts/build-wasm.sh` —
which needs Rust, `wasm-bindgen` 0.2.122, and a fat-LTO Bevy build that takes
**2m55s locally with a warm cargo cache**. Installing Rust in the build command
would work in principle but starts cold every time.

So `.github/workflows/deploy-web.yml` builds on a runner — reusing the same pinned
toolchain and the same `Swatinem/rust-cache` key as `ci.yml`'s `wasm-web` job — and
deploys with `wrangler pages deploy`. The Pages project is therefore a **Direct
Upload** project, not connected to git.

Consequences accepted: PR preview deployments are not automatic any more (wrangler
can produce them via `--branch`, not yet wired), and two secrets are needed —
`CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID`.

**Shipped artifact sizes**, measured end to end:

| Build | Largest asset |
|---|---|
| `build-wasm.sh` (dev, the default) | 139 MiB — would be rejected |
| `build-wasm.sh release` → `wasm-bindgen` → `vite build` | **13.9 MiB**, gzip 3.9 MB |

`wasm-bindgen` shrinks the 17MB `wasm-release` binary to 14MB by dropping unused
exports, so there is ~44% headroom under the 25 MiB cap.

**Noted, not acted on:** the generated `engine.js` uses `eval`, which rollup
warns about. Harmless today, but it will conflict with a strict `Content-Security-Policy`
— relevant when `web-delivery` 04 adds the service worker and headers.
