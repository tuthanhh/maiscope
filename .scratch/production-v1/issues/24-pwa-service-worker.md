# 24 — PWA: manifest + service worker

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

**Blocked by:** 23

**Status:** todo

- [ ] `vite-plugin-pwa` added, `registerType: 'autoUpdate'`
- [ ] Web manifest: name, short name, theme/background colour, maskable icons,
      `display: standalone`, `start_url`
- [ ] App shell precached; `index.html` network-first
- [ ] Runtime cache-first rule for `**/*.wasm`, with an explicit size limit and
      a bounded entry count
- [ ] API responses **not** cached by the service worker — that is HTTP's job
      (issue 08) and IndexedDB's (issue 22). Three layers, three lifetimes.
- [ ] Install tested on desktop Chrome and on Android Chrome
- [ ] Offline test: reload with network off, app shell still loads
- [ ] Recovery rehearsed: deploy a broken build, confirm the next deploy actually
      reaches installed clients
