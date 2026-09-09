# ADR-0008 — Documentation structure: Diátaxis-lite plus `docs/work/`

**Status:** accepted
**Date:** 2026-09-09

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

ADRs carry the rationale; `work/production-v1/spec.md` is reduced to scope and
sequencing and links to them, so the reasoning lives in exactly one place.
