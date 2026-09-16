# 04 — PWA: manifest + service worker

**What to build:** Make the app installable and offline-capable via
`vite-plugin-pwa` with `registerType: 'autoUpdate'`.

A service worker is the one artifact you can ship that **cannot be taken back
over the network** if it is wrong. A cache-first rule on `index.html` pins
installed users to a broken build permanently. The three specifics below matter
more than any of the configuration around them.

1. **`index.html` must be network-first (or `NetworkOnly`).** This is the escape
   hatch from a bad deploy.
2. **Do not precache the wasm.** Precaching means every visitor to the browse
   page downloads ~20MB before asking for the visualizer. Runtime cache-first on
   the content-hashed `.wasm` URL instead — first visualizer visit pays,
   later ones are instant and work offline. Vite's hashed filenames handle
   invalidation.
3. **Silent auto-update over prompt-to-update.** Nothing in a read-only app can
   be lost by reloading, so an update-prompt component is unearned complexity.

**Blocked by:** 03

**Status:** in-progress — install (desktop + Android Chrome) and offline reload
verified for real (2026-09-16); only the broken-build recovery rehearsal is
still open

- [x] `vite-plugin-pwa` added, `registerType: 'autoUpdate'`
- [x] Web manifest: name, short name, theme/background colour, maskable icons,
      `display: standalone`, `start_url`
- [x] App shell precached; `index.html` network-first
- [x] Runtime cache-first rule for `**/*.wasm`, with an explicit size limit and
      a bounded entry count — see Comments on where the size limit actually applies
- [x] API responses **not** cached by the service worker — that is HTTP's job
      (`server-restructure` issue 07) and IndexedDB's (issue 02). Three layers, three lifetimes.
- [x] Install tested on desktop Chrome and on Android Chrome (2026-09-16,
      maintainer-tested on both)
- [x] Offline test: reload with network off, app shell still loads
      (2026-09-16, maintainer-tested)
- [ ] Recovery rehearsed: deploy a broken build, confirm the next deploy actually
      reaches installed clients

## Comments

**The sprite atlas was the real precache problem, not the wasm.** First green
build precached **141 entries / 5,884 KiB**. Of that, **105 files and 5.7MB were
`assets/sprites/**`** — the engine's note, judge, slide and effect artwork. The
actual app shell is ~180KB; everything else was the visualizer's texture set,
downloaded by every catalog visitor before the first song rendered. Rule 2 above
names the wasm because that is the obvious 20MB, but the sprites were shipped the
same way and nobody had measured them.

Worse after `05-gate-visualizer-off-mobile`: a phone that is *refused* the
visualizer would still have paid for its artwork.

Excluded via `globIgnores` and served by a runtime rule instead. Precache is now
**37 entries / 517 KiB** — a 91% cut.

**Sprites use `StaleWhileRevalidate`, not `CacheFirst` like the wasm**, and the
difference is invalidation, not preference. The wasm ships as
`maiscope_viewer_bg-<hash>.wasm`, so a new build is a new URL and a stale entry
can never shadow it. The sprites are copied verbatim out of `public/assets/`, keep
their filenames forever, and would be pinned until expiry under cache-first. SWR
serves the cached copy instantly and replaces it in the background, so a sprite
change still lands on the next visit.

**`index.html` is network-first by *omission*.** It is not in `globPatterns`, and
a `NetworkFirst` rule matches `request.mode === "navigate"` instead. A file cannot
be both precached and network-first — the precache route claims the URL first — so
excluding it is what makes the escape hatch real. This forces
`navigateFallback: null`: the plugin defaults it to `index.html`, which is no
longer in the precache manifest.

Consequence for the offline test below: load once **online** so the navigation
rule populates `app-shell`, then cut the network. A cold first visit offline fails
by design.

**Where the "explicit size limit" actually lives.** Workbox applies
`maximumFileSizeToCacheInBytes` to *precaching* only; runtime caches have no
per-entry byte cap. The wasm rule is bounded by `maxEntries: 2` (current build
plus one, so a client mid-deploy keeps what it is running) and
`maxAgeSeconds: 30d`. The byte cap is left at its 2MiB default deliberately — if
a wasm ever slips past `globPatterns` and `globIgnores`, the build fails loudly
rather than quietly precaching 14MB.

**The API rule matches by path, not origin.** `VITE_API_BASE_URL` is injected at
build time (`deploy-web.yml:111`), so the origin is localhost in dev and Fly in
production. `url.pathname.startsWith("/api/")` covers both; a hardcoded
`maiscope-api.fly.dev` would have silently stopped matching in dev.

**Two landmines cleared from `public/` first.** A self-destroying service worker
(`sw.js`, the NekR unregister-on-activate pattern) inherited from arcade-songs,
carrying its own "SHOULD NOT BE VERSION CONTROLLED" header — inert, since nothing
registered it, but `vite-plugin-pwa` emits `dist/sw.js` too and Vite copies
`public/` verbatim. And `site.webmanifest`, unreferenced, with empty `name` and no
`display` or `start_url`, which would have shipped alongside the generated
`manifest.webmanifest`.

**`VitePWA` is a named export**, not a default one — `import VitePWA from` yields
`undefined` and fails at config load with `VitePWA is not a function`.

**Verified from `dist/`, not from the plugin's summary line:** no `.html` in the
precache manifest; the only `.wasm` string in `sw.js` is the runtime matcher; all
four handlers registered; `manifest.webmanifest` carries all three icons with
`purpose: "maskable"` on the padded one. `public/maskable-512x512.png` is the 512
icon scaled to 410px on an opaque white field — the source has transparent corners
that Android's mask crop would otherwise expose.
