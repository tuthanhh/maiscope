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
- [x] `FLY_API_TOKEN` added as a **environment** secret on `production` (scoped
      tighter than a repo secret: only jobs declaring that environment can read it)
- [ ] Fly's GitHub auto-deploy turned **off**, so this workflow is the only path
- [x] Verified: one real dispatch deployed and smoke-checked green (`56a9412`)
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


## Comments

**Verified end to end on `56a9412`** (2026-09-13):

```
CI gate passed
flyctl deploy -> release_command 48e6e94a0d4d48 completed successfully
rolling update of machine 080762df263448
healthcheck 200 on attempt 1: {"status":"good"}
```

That release command is the first time `bin/migrate` actually ran on Fly. The four
earlier deploys ran the *server* binary there instead, because the image used
`ENTRYPOINT` and Fly implements `release_command` by setting cmd (issue 03).

**The gate proved itself twice before passing**, both times failing closed:

1. `Resource not accessible by integration (HTTP 403)` — the default `GITHUB_TOKEN`
   is contents-read only and the check-runs API needs `checks: read`. Fixed with an
   explicit job-level `permissions` block; note that naming any permission drops
   every unnamed scope to none, so `contents: read` must be listed too.
2. `Check 'rust' is 'pending'` — dispatched before CI finished on that commit. Correct
   behaviour, and an intrinsic property of gating on CI: the commit that fixes the
   deploy workflow must itself pass CI before it can be deployed.

**Deploying an older commit is supported** via the `ref` input, which is the usual
answer to that second case when the new commit only touched docs or workflow YAML.
The workflow *file* used for a dispatch always comes from the default branch, not
from `ref`.

**A failed deploy leaves a red `deploy` check run on the commit.** It is the deploy
workflow reporting itself, not a CI failure — the gate only inspects `rust` and
`wasm-web` by name, so it does not feed back on itself.

**The deploy token expires 2026-09-14.** Short-lived by default; deploys will start
failing with an auth error that reads like a broken workflow. Reissue with a longer
lifetime.
