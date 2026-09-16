# ADR-0004 — Fly compute plus Neon Postgres, deliberately separated

**Status:** accepted
**Date:** 2026-09-09

## Context

Free-tier hosting, with learning CI/testing/deployment as an explicit goal.

## Decision

Axum in Docker on Fly.io with scale-to-zero; Neon Postgres as a separate
service.

## Rejected alternatives

Railway and Render (compute and data share a fate, and Render's free Postgres
has historically expired on a timer — disqualifying for irreplaceable data); an
Oracle Cloud always-free VM (the project's time would go to TLS renewal and
Postgres upgrades); Shuttle (restructures `main.rs` around its runtime macros
and teaches only Shuttle, where a Dockerfile transfers anywhere).

## Consequences

A bad deploy, a suspended service or a blown free tier cannot reach the data.
Scale-to-zero means cold starts, which the client cache and edge caching later
mitigate. Migrations run as a Fly `release_command` so a bad migration aborts
the deploy instead of crash-looping a new machine. Backups are Neon's restore
window plus scheduled `pg_dump`. Fly has no hard spend cap, so billing alerts
are required.

**Verified for real (2026-09-16, [build-and-deploy ticket
03](../work/build-and-deploy/issues/03-migrate-binary-and-flytoml.md)):** a
deliberately invalid migration, deployed directly against the live app, failed
the release-command probe machine and aborted the deploy; the serving machine
never restarted and kept answering on the pre-deploy commit throughout. The
abort path was previously only inferred from an unrelated `ENTRYPOINT`
misconfiguration (four accidental failed deploys during ticket 03) — this was
the first deliberate trigger of it.
