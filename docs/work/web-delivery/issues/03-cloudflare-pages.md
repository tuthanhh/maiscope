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

**Status:** todo

- [ ] Pages project connected to the repo; build command `pnpm build`, output `dist`
- [ ] `VITE_API_BASE_URL` set to the Fly origin in Pages env vars
- [ ] SPA fallback/rewrite configured so Vue Router history mode does not 404 on
      deep links
- [ ] Preview deployments on PRs (their origins must not be in the CORS allowlist
      unless deliberately added)
- [ ] Brotli confirmed on `.js`/`.wasm`
- [ ] Recorded: the exact production origin, since it becomes the PWA identity
- [ ] Ticket opened for the custom-domain migration, with the install-orphaning
      consequence written down
- [ ] **The wasm artifact fits what Pages will accept** — see Comments; at today's
      size it does not

## Comments

**The "~20MB wasm artifact" figure is optimistic by 2×.** Measured 2026-09-13
while closing `build-and-deploy` 02:

| Artifact | Size |
|---|---|
| `apps/host/src/wasm/maiscope_viewer_bg.wasm` (dev build) | 140 MB |
| `apps/host/dist/assets/maiscope_viewer_bg-*.wasm` (what `pnpm build` emits) | 143 MB, gzip 16.4 MB |
| `target/wasm32-unknown-unknown/release/maiscope_viewer.wasm` | 43 MB |
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
