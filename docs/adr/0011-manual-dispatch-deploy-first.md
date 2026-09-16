# ADR-0011 — Deploy starts manual-dispatch, automates later

**Status:** accepted
**Date:** 2026-09-16

## Context

`deploy.yml` needed a trigger. The three obvious options — manual dispatch,
tag push, auto-deploy on merge to `master` — are not mutually exclusive over
time, just increasingly hands-off. The project had no deploy pipeline at all
yet, so whichever came first would be exercised and trusted before the next
stage of automation could be justified.

Expand-and-contract (the umbrella spec's rule) makes additive migrations safe
to ride a deploy automatically, but a `DROP COLUMN` or `DROP TABLE` is a
separate, deliberate, manual step by design — Postgres holds the only copy of
chart text. Auto-deploy-on-merge only stays safe once expand-and-contract is
habit, not just policy: a merge could otherwise apply schema changes with no
human watching.

## Decision

Ship `deploy.yml` as `workflow_dispatch` only. The progression is recorded as
**manual dispatch → tag-triggered → auto-on-merge**, so moving to the next
stage is a deliberate step against a written plan, not drift discovered after
the fact.

## Rejected alternatives

**Auto-deploy on merge to `master`, from the start.** Fastest feedback loop,
but removes the human checkpoint before expand-and-contract discipline has
been proven in practice — a bad migration would reach production the moment
a PR merges, with no dispatch step to catch it first.

**Tag-triggered as the first stage.** Slightly more automated than manual
dispatch, but adds a second mechanism (tagging discipline) to get right before
the pipeline itself had been exercised even once. Manual dispatch is strictly
simpler to verify first.

## Consequences

Every production deploy needs a human to click dispatch — acceptable at
current traffic and team size (one), and required regardless while CI's
green-on-exact-commit gate and Fly's release-command abort path are unproven.
Revisit tag-triggered once merges to `master` are routine and reviewed;
revisit auto-on-merge only once expand-and-contract has been the only way
schema has changed for a while, not just the stated rule.

Verified for real (2026-09-16, [ticket
05](../work/build-and-deploy/issues/05-deploy-workflow.md)): rollback is a
manual `flyctl deploy --image` to a prior release, and a release-command
failure aborts the deploy while the previous machine keeps serving — both
confirmed against the live app, not just reasoned about.
