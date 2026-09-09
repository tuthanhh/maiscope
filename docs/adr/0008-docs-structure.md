# ADR-0008 — Documentation structure: Diátaxis-lite plus `docs/work/`

**Status:** accepted
**Date:** 2026-09-09
**Amended by:** [ADR-0009](0009-umbrella-features-and-spec-timing.md)

## Context

This spec. Records the audience-split structure, the single-roadmap rule,
feature directories as the unit of work, and the retirement of
`docs/superpowers/plans/`.

## Decision

Diátaxis-lite audience split — reference / guides / explanation — over
topic-first or full Diátaxis. Milestone-first roadmap at `docs/ROADMAP.md`;
feature directory stays the unit of work; README links, never enumerates.
Feature dirs are the single tracker, promoted to `docs/work/`;
`docs/superpowers/plans/` retired.

## Rejected alternatives

Topic-first organization, and full Diátaxis (tutorials / how-to / reference /
explanation). The docs here are ~90% internal engineering — reference,
explanation, and process. There is no tutorial audience beyond README's
getting-started, so full Diátaxis would create empty folders.

## Consequences

Each document answers exactly one question; if two documents answer the same
question, one of them is wrong. Decisions go in `adr/`, behaviour changes in
`reference/`, planned steps in `work/`, and current status only in
`ROADMAP.md` — nowhere else. Milestones (v1.0 / v1.1 / phase 2), previously
defined inside `work/production-v1/spec.md`, move to `ROADMAP.md`; the feature
spec links to them instead of restating them.

Two status vocabularies replace the three strings in use before (`todo`,
`done`, `ready-for-agent`): features go `planned` → `active` → `shipped` →
`superseded`, tickets go `todo` → `in-progress` → `done` → `dropped`. The
archive rule follows from this split: when a feature ships, its real decisions
move into an ADR, its behaviour changes land in `reference/`, and
`ROADMAP.md` collapses it to one line — this is what stops `work/` from
becoming the next `.scratch/`.

`guides/` starts with a single file rather than empty stubs for `deploy.md`
and `runbook.md`, which are outputs of later production-v1 tickets, not
placeholders — empty stubs are what make docs look abandoned.
