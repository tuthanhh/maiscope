# 03 — POST /contributions/{id}/approve and /reject

**What to build:** Moderator-gated approve/reject endpoints. `edit`-kind approvals merge into `sheets` (COALESCE partial update, only fields present in payload). `sheet`/`song`/`chart`-kind approvals return `501 Not Implemented` — an explicit, stated scope cut (their merge logic needs the `charts`/`chart_revisions` schema settled further, deferred to a follow-up). Reject records a reason and is idempotent-safe (only transitions from `pending`).

**Blocked by:** 02 — submit/list contributions.

**Status:** todo

- [ ] `require_moderator` guard: non-moderator/admin gets `403`
- [ ] Approve: `edit` kind merges into `sheets` via COALESCE (only payload-present fields change), bumps `catalog_meta.update_time`, sets status `merged`, writes an `audit_log` row
- [ ] Approve: `sheet`/`song`/`chart` kinds return `501` with a clear message, do not silently no-op
- [ ] Approve/reject both reject (`409`) an already-reviewed contribution
- [ ] Reject: records `reject_reason`, sets status `rejected`, writes an `audit_log` row
- [ ] Manual smoke test: approve an `edit` contribution as moderator, confirm the sheet field changed; reject another with a reason; confirm both `403` for a plain-`user` token
