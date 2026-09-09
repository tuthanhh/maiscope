# 16 — CD: manual-dispatch deploy workflow

**What to build:** A `workflow_dispatch` workflow that builds the image and
deploys to Fly. Manual for now — the pipeline gets built and exercised before it
is trusted to fire on its own.

Progression, recorded so it is a plan and not a drift: **manual dispatch → tag-triggered
→ auto-on-merge**. Auto-on-merge is only safe once the expand-and-contract
migration rule is habit, because a merge would then be able to apply schema
changes with no human in the loop.

**Blocked by:** 13, 15

**Status:** todo

- [ ] `.github/workflows/deploy.yml` with `workflow_dispatch`
- [ ] Refuses to deploy unless CI is green on the target commit
- [ ] `flyctl deploy` using `FLY_API_TOKEN` from repo secrets
- [ ] Release command runs migrations first (issue 13); a failure aborts cleanly
- [ ] Post-deploy smoke check: `GET /api/v1/healthcheck` returns 200
- [ ] Rollback procedure written into the ticket and tested once for real
- [ ] Deployed commit SHA surfaced in the startup log (issue 06) so the running
      version is identifiable
