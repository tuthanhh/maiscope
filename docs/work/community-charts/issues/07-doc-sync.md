# 07 — Doc sync

**What to build:** bring the reference docs onto what shipped.

**Blocked by:** 02, 03, 04, 05, 06 — docs describe reality, not intent.

**Status:** todo

- [ ] `api-contract.md` §5 rewritten from the contribution queue to the
      community library: the `/community/*` routes, live-on-upload, no `pending`
      state, and an explicit statement that community content never appears in
      `/catalog` or `/sync/*`
- [ ] §5's struck-audio note kept — it still applies, and ADR-0013 reaffirms it
- [ ] `schema.md` moves the three tables out of "Planned tables"
- [ ] `ROADMAP.md` collapses this feature to one line and marks it shipped
- [ ] `server-contributions` confirmed `superseded`, with its tickets left in
      git as the record of the rejected design
