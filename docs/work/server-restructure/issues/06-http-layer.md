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

**Status:** done

- [x] `tower-http` features gain `compression-br`, `compression-gzip`
- [x] `CompressionLayer` added; verified `Content-Encoding: br` on `/catalog`
- [x] Measured before/after byte size of `/catalog` recorded in this ticket —
      **3,040,454 bytes uncompressed → 309,171 bytes with `br`** (roughly a
      90% reduction; this dataset is smaller than the 4.7MB figure quoted in
      the problem statement, but the ratio holds)
- [x] `CorsLayer` built from `config.cors_allowed_origins`; `permissive()` gone
- [x] Local dev origin (`http://localhost:1420`, confirmed against
      `apps/host/vite.config.ts`'s fixed dev port) present in the example
      config — and made active by default (uncommented), not just documented,
      since ticket 01's "empty = no allowlist" now means zero origins allowed
- [x] Bind uses `config.port` (already done in ticket 01/03)
- [x] Comment at the CORS layer stating plainly that CORS is an egress control
      here, not a security control — the real cap is issue 08

## Comments

Resolves ticket 01's open question ("ticket 06 decides what an empty
`cors_allowed_origins` implies"): **no permissive fallback**. Empty means
zero origins allowed, full stop — consistent with every other `Config` field
in this codebase (explicit, no magic default that silently degrades
security). Local dev sets `CORS_ALLOWED_ORIGINS` in `.env` like everything
else; `.env.example` ships it uncommented so `cp .env.example .env` works
out of the box.

Unparseable origin strings (fail `HeaderValue::parse`) are silently dropped
in `routes/mod.rs`, not a startup error — a CORS misconfiguration here is an
availability nuisance for that one origin, not a security hole (`curl`
ignores CORS entirely, so this layer never gates access to the data itself).
Chose not to validate in `config.rs`'s `parse_cors_origins` to keep that
module's "no `tower`/`http` types, data only" boundary intact.

`allow_methods([Method::GET])` is set explicitly even though every route
here is a simple GET (no preflight triggered either way) — every endpoint in
this API is GET-only, so it costs nothing and documents that fact.
