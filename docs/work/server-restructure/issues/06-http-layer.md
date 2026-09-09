# 06 — Server: compression, CORS allowlist, configurable bind

**What to build:** Three HTTP-layer fixes that are currently production blockers.

1. **No compression layer exists** — `tower-http` is pulled in with only the
   `cors` feature (`apps/server/Cargo.toml:12`), and Fly's proxy does not gzip
   for you. `GET /catalog` therefore ships **4.7MB uncompressed** on every call.
   Brotli/gzip takes that to roughly 500KB.
2. **`CorsLayer::permissive()`** (`main.rs:92`) allows any origin, header and
   method. For public read-only data this is not a vulnerability — `curl`
   ignores CORS entirely — but an allowlist discourages other *sites* from
   spending your egress. Replace with the configured origin list.
3. **Bind address** comes from config (issue 01) so Fly's `PORT` is honoured.

**Blocked by:** 01

**Status:** todo

- [ ] `tower-http` features gain `compression-br`, `compression-gzip`
- [ ] `CompressionLayer` added; verified `Content-Encoding: br` on `/catalog`
- [ ] Measured before/after byte size of `/catalog` recorded in this ticket
- [ ] `CorsLayer` built from `config.cors_allowed_origins`; `permissive()` gone
- [ ] Local dev origin (`http://localhost:1420`) present in the example config
- [ ] Bind uses `config.port`
- [ ] Comment at the CORS layer stating plainly that CORS is an egress control
      here, not a security control — the real cap is issue 08
