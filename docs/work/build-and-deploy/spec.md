# Spec — Build and deploy

**Status:** shipped
**Milestone:** v1.0
**Parent:** [`production-v1`](../production-v1/spec.md)

Make the server buildable without a database, shippable as a container, and
deployable to Fly with migrations that fail safely. Then automate the checks that
keep it that way.

## Problem

`cargo build` currently **requires a reachable `DATABASE_URL`** — the server uses
62 compile-time `sqlx::query!`/`query_as!` macros and there is no `.sqlx/`
directory. A `docker build` has no database, so containerisation is blocked
outright before any Dockerfile is written.

The repo also has no CI. Nothing runs the six existing tests automatically, so
"tests green" is a claim rather than a gate.

## Scope

| # | Ticket | Note |
|---|---|---|
| 01 | [`cargo sqlx prepare`: commit `.sqlx/`, build with `SQLX_OFFLINE`](issues/01-sqlx-offline.md) | Blocked on `server-restructure` 04 — do it once the SQL has stopped moving |
| 02 | [Hermetic multi-stage `Dockerfile`](issues/02-dockerfile.md) | Build `-p server` only, or the image drags in the whole Bevy tree |
| 03 | [`bin/migrate` + `fly.toml` with `release_command`](issues/03-migrate-binary-and-flytoml.md) | |
| 04 | [CI: pull-request checks](issues/04-ci-pr-checks.md) | |
| 05 | [CD: manual-dispatch deploy workflow](issues/05-deploy-workflow.md) | |

## Decisions that constrained this work

Migrations-as-release-command and the manual-dispatch-first deploy progression
are now [ADR-0004](../../adr/0004-fly-compute-neon-postgres.md) and
[ADR-0011](../../adr/0011-manual-dispatch-deploy-first.md) — both since
verified for real against the live app, not just reasoned about (see their
Consequences sections). Expand-and-contract is the umbrella spec's rule, not
this feature's. `.sqlx/` as committed generated state remains documented in
`CLAUDE.md` and [ticket 01](issues/01-sqlx-offline.md) rather than promoted to
an ADR — a build mechanic with no seriously contested alternative, not an
architecture choice.

## Out of scope

Provisioning the database itself and everything that runs against production data —
those are [`prod-data-and-infra`](../prod-data-and-infra/spec.md). This feature
produces the pipeline; that one points it at a real Neon project.

Frontend hosting. Cloudflare Pages builds from its own integration
([`web-delivery`](../web-delivery/spec.md) 03), not from this workflow.

## Done when

The image builds on a machine with no Postgres reachable, a deliberately broken
migration aborts a deploy while the previous version keeps serving, `flyctl deploy`
runs from `workflow_dispatch` with a green-CI precondition, and every PR runs fmt,
clippy, `sqlx prepare --check`, the workspace tests, a wasm target build and
`pnpm build`.
