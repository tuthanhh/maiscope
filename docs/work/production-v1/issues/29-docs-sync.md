# 29 — Documentation sync for v1 scope

**What to build:** Bring the docs in line with the decisions in `spec.md`. Several
are actively misleading now.

Known-stale, found during the grilling session:

- **CLAUDE.md claims there is no test suite beyond `shared`'s scaffold.** There
  are six `#[sqlx::test]` handler tests at `apps/server/src/main.rs:484+`.
- **CLAUDE.md and README describe a Tauri desktop app** as the frontend. There is
  no desktop target and no Tauri (issue 21).
- **README's "Getting started" tells you to run `cargo run --bin ingest`** to pull
  upstream `data.json`. After issue 17 that needs a snapshot path.
- **CLAUDE.md documents `src-tauri/` proxying authenticated writes** to dodge CORS
  for the contribution flow. Both the proxy and v1 contributions are gone.
- **`docs/api-contract.md` §5 includes audio upload and S3 presigning.** Audio is
  cut; contributions are phase 2.
- Docs moved from `apps/server/docs/` to `docs/` but CLAUDE.md still points at the
  old paths.

**Blocked by:** 21, 24 (write it once the frontend shape is final)

**Status:** todo

- [ ] CLAUDE.md: doc paths, test-suite claim, Tauri references, commands corrected
- [ ] README: architecture diagram, prerequisites, getting-started, roadmap
      updated to v1.0 / v1.1 / phase 2
- [ ] `docs/api-contract.md`: mark §4 auth and §5 contributions **phase 2**; strike
      audio upload; document ETag/304 and rate-limit `429` behaviour
- [ ] `docs/schema.md`: unchanged unless a migration lands, but verify
- [ ] A `deployment.md` covering Fly, Neon, Pages, the workflows and the runbook
- [ ] `.scratch/server-auth-github-oauth/` and `.scratch/server-contributions/`
      marked deferred to phase 2 rather than left looking active
