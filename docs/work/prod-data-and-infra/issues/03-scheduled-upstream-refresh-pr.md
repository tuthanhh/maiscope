# 03 — Scheduled upstream refresh, delivered as a PR

**What to build:** A cron workflow that fetches upstream `data.json`, and if it
differs from the vendored snapshot (issue 02), opens a pull request with the
updated file. Automated detection, human approval — the review gate is the part
with the value.

The diff is the early-warning system for the failure mode that silently loses
charts: an upstream song rename that breaks `seed_songs`' title matching.

**Blocked by:** 02

**Status:** todo

- [ ] Scheduled workflow in the data repo (weekly is plenty)
- [ ] Fetches upstream, compares against the vendored snapshot
- [ ] No change → exits quietly, no PR, no noise
- [ ] Change → opens/updates a PR with the new snapshot
- [ ] PR body summarises the delta: songs added / removed / **retitled**, since
      retitles are the dangerous class
- [ ] Fails loudly if upstream is unreachable or returns a suspiciously small file
- [ ] Merging the PR does **not** auto-deploy — applying it is issue 04
