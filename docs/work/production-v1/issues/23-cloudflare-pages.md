# 23 — Frontend hosting on Cloudflare Pages

**What to build:** Deploy the Vue build to Cloudflare Pages. Two origins for now
(`*.pages.dev` for the app, `*.fly.dev` for the API), so a CORS allowlist is
required (issue 07).

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

**Blocked by:** 21

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
