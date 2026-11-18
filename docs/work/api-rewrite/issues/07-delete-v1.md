# 07 — Delete `/api/v1`

**What to build:** the cutover's server half. Once the frontend calls only
`/api/...`, the v1 tree goes, with everything only it used.

**Blocked by:** 02, 03, 04, 05, 06;
`frontend-redesign/issues/04-screens-on-new-api.md` (the client must have
stopped calling v1).

**Status:** done

- [x] `rm -r apps/server/src/routes/v1/`; `routes/mod.rs` nests `/api` only
- [x] The router tests in `routes/mod.rs` that drive `/api/v1` paths are
      repointed at v2 routes, not deleted — notably
      `healthcheck_is_exempt_while_the_same_ip_is_throttled_elsewhere`, the
      only proof the health exemption is wired, which burns its bucket on
      `/api/v1/sync/manifest`
- [x] Queries, types and domain code used only by v1 go with it: sheet search
      and its params, `sheetExpr`-keyed lookups, delta, the v1 wire types. The
      compiler's dead-code warnings are the checklist
- [x] v1-only leftovers go too: `AppError::SnapshotRequired`, `Freshness`'s
      `catalogHash` doc, the `/api/v1` empty-404 fallback in `routes/mod.rs`
- [x] CORS stops exposing `X-Total-Count` until a v2 route sends it
      (`community-charts` browse will)
- [x] `test-foundation` 02's two deferred tests (the delta 409 path, the
      search pagination boundary) are closed as moot: their endpoints are gone
- [x] `tests/snapshots/catalog.json` and the v1 snapshot test are deleted or
      replaced by 03's mapping check
- [x] Health checks move first, in their own change: `fly.toml`'s
      `[[http_service.checks]]` path and `deploy.yml`'s `HEALTHCHECK_URL`
      point at `/api/healthcheck`, not `/api/v1/healthcheck` — or the deploy
      gate and Fly's check fail the moment v1 is gone
- [x] Contract: v1 sections removed (`computeSheetExpr`, `GET /sheets/{sheetExpr}`
      and the "unchanged" notes among them); §7's wiring map describes v2
- [x] `architecture.md` and `schema.md` drop the v1-only `sheet_expr` and
      `song_no` notes that say "until api-rewrite 07"
- [x] `.sqlx/` regenerated; `--check` clean
- [x] Spec *Done when* verified: no URL in the system carries a percent-encoded
      title

## Comments

**2026-11-18 — built.** The health checks moved first (`7907d02`). Then
`routes/v1/` went, and with it everything the compiler named dead once it
had: `queries/{catalog,sheets,songs}.rs` (sheet search and its params, the
`sheetExpr` lookups, the v1 reference fetches), `domain.rs`, the v1 wire
types in `types.rs`, `AppError::SnapshotRequired` (so a rendered error
always has a message now), `Freshness::update_time`, the empty-404 v1
fallback, the CORS `X-Total-Count` exposure and its test, the
`seed_standard_difficulties` fixture only v1 tests used, and
`tests/snapshots/catalog.json` (ADR-0018 notes what replaced it). The router
tests drive v2 routes: the health exemption burns its bucket on
`/api/sync/manifest`, and `/api/v1/healthcheck` is now asserted to be an
ordinary JSON `404`. `.sqlx/` lost the 51 queries nothing runs; `--check` is
clean. The contract drops §1–3 and keeps §4–§7's numbers; code comments
that cited §1–§3 point at §0. Row revisions have no reader after
`/sync/delta`; the sync still keeps them exact, and its comments say so.
`test-foundation` 02's two deferred tests are closed as moot.

Done when, verified: no route takes a title, and the client's only
percent-encoding is of ids and difficulty codes (a UTAGE code is not ASCII).

**2026-11-18 — review.** Applied: a blank line left by the removed error
test failed `cargo fmt --check` (CI's first gate) — fixed, and fmt, clippy
`-D warnings`, `sqlx prepare --check` and the tests now all run clean
locally as CI runs them. v1's empty-database Catalog test, the guard for
`prod-data-and-infra` 06's 500, had no v2 equivalent: `/catalog` and
`/reference` now each have one, and the Catalog's 304 test checks the
validators it repeats. `scripts/compression-eval.sh`, the local-setup guide,
the prod-data runbook and web-delivery 06's open items pointed at
`/api/v1`. Docs said `sheet_expr` is only the sync's key — the read queries
also join tombstones on it — and that a snapshot test guards the wire
contract; both corrected, and schema.md's delta notes are in the past
tense. Kept: comments that contrast a v2 choice with v1 explain the choice.
