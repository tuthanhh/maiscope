# 05 — Update api-contract.md §5 and schema.md for contributions

**What to build:** Doc sync: api-contract.md §5 notes the real approve-merge scope (`edit` implemented, `sheet`/`song`/`chart` return `501` pending a follow-up) and drops the stale "/revision" bump reference (no sync-tier revision counter exists in this feature). schema.md moves `contributions`/`audit_log`/`chart_revisions` out of "Planned tables" into a documented "Contribution tables (built)" section.

**Blocked by:** 03 — approve/reject, 04 — audio upload presigning (docs describe what actually shipped).

**Status:** todo

- [ ] api-contract.md §5 approve section documents the `edit`-only merge scope and the `501` behavior for other kinds, with a pointer to the follow-up
- [ ] schema.md: `contributions`, `audit_log`, `chart_revisions` rows removed from "Planned tables" (remove the whole section if now empty), documented under a new "Contribution tables (built)" section
- [ ] `grep -n "^| \`contributions\`\|^| \`audit_log\`" apps/server/docs/schema.md` returns no output
