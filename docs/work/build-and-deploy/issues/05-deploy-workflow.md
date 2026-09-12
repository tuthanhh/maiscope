# 05 — CD: manual-dispatch deploy workflow

**What to build:** A `workflow_dispatch` workflow that builds the image and
deploys to Fly. Manual for now — the pipeline gets built and exercised before it
is trusted to fire on its own.

Progression, recorded so it is a plan and not a drift: **manual dispatch → tag-triggered
→ auto-on-merge**. Auto-on-merge is only safe once the expand-and-contract
migration rule is habit, because a merge would then be able to apply schema
changes with no human in the loop.

**Blocked by:** 03, 04

**Status:** in-progress

- [x] `.github/workflows/deploy.yml` with `workflow_dispatch`, taking a `ref` input
- [x] Refuses to deploy unless CI is green **on that exact commit**
- [x] `flyctl deploy --remote-only` using `FLY_API_TOKEN` from repo secrets
- [x] Release command runs migrations first (issue 03); a failure aborts cleanly
- [x] Post-deploy smoke check: `GET /api/v1/healthcheck` must return 200
- [x] Rollback procedure written into this ticket — **not yet tested for real**
- [x] Deployed commit SHA surfaced in the startup log as `git_sha`
- [ ] `FLY_API_TOKEN` added to repo secrets, `production` environment created
- [ ] Fly's GitHub auto-deploy turned **off**, so this workflow is the only path
- [ ] Verified: one real dispatch deploys and smoke-checks green
- [ ] Verified: rollback exercised once for real

## Rollback

Fly keeps every released image, so rollback is a redeploy of the previous one —
no rebuild, no dependence on git state:

```sh
flyctl releases -a maiscope-api                     # find the last good version
flyctl deploy -a maiscope-api --image registry.fly.io/maiscope-api:deployment-<id>
```

The same is available from the dashboard's release list. Rolling back this way
still runs `release_command`, so the migrator executes again — harmless, since it
reports `schema already current` and exits 0.

**Rollback does not undo migrations, by design.** The image goes back; the schema
does not. That is exactly why expand-and-contract is a rule rather than a
preference: the previous version must still run against the newer schema. An
additive migration satisfies this automatically. A `DROP COLUMN` does not, which is
why destructive changes are never deployed through this pipeline (spec.md).

To roll back *and* undo schema, the down migration is a separate, deliberate,
manual step — run before redeploying the older image, never automated.
