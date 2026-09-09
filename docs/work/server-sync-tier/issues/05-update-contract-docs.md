# 05 — Update api-contract.md for the sync tier

**What to build:** Doc sync: precise 409 semantics (predates last full reload, not a vague staleness heuristic — incremental contribution-approve edits never trigger it), and the `charts: []` scope-cut note on `GET /sync/delta`'s response shape.

**Blocked by:** `server-contributions/issues/06-bump-revision-on-approve.md`, 04 — manifest/delta/etag (docs describe what actually shipped).

**Status:** dropped

**Dropped:** absorbed by the `docs-restructure` feature — see
[`docs/work/docs-restructure/spec.md`](../../docs-restructure/spec.md). That work
covers every item this ticket listed.

- [ ] `409 snapshot_required` semantics documented precisely against `ingest`'s full-reload behavior
- [ ] `GET /sync/delta` response shape shows `charts: []` with a note that chart-meta delta tracking is a follow-up
