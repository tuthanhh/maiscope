# ADR-0009 — Umbrella features, and `spec.md` required at `active`

**Status:** accepted
**Date:** 2026-09-10
**Amends:** [ADR-0008](0008-docs-structure.md)

## Context

Two things had drifted apart from what [`docs/agents/issue-tracker.md`](../agents/issue-tracker.md)
claimed.

**The spec rule was at 22% compliance.** Seven of nine feature directories had no
`spec.md` against a rule that said one is required. The violations were not
laziness — they split into two groups, and the rule was wrong about both. Five
shipped without a spec and had already moved their decisions into `adr/`, exactly
as the tracker's own archive rule instructs, so a spec would have been written and
immediately superseded. Two are planned for phase 2, where writing a spec now
means guessing at constraints that v1.0 work will change.

**`production-v1` had grown to 29 tickets.** Four concrete costs followed. `ROADMAP.md`
carried one row for it, so the roadmap could not report progress on 97% of remaining
v1.0 work. Twenty `Blocked by:` lines and 36 prose references used bare ticket
numbers — an implicit global namespace that coupled every reference to the directory
never being split. The spec held cross-cutting invariants (expand-and-contract
migrations, additive-only response shapes, the three-cache-layer table) that
constrain every ticket and belong to none, which is a milestone wearing a feature's
clothes — the thing ADR-0008 set out to stop. And its "verify before relying on"
list held four blocking unknowns that no ticket owned.

## Decision

**Umbrella features.** A feature directory may hold `spec.md` and no `issues/`,
serving as the shared-rules document for named child features. `production-v1`
becomes one; its 28 live tickets move into `repo-hygiene`, `server-restructure`,
`build-and-deploy`, `prod-data-and-infra`, `web-delivery`, `test-foundation`, and
`mobile-webview-spike` (the last belonging to v1.1, not to the umbrella).

**`spec.md` is required before a feature goes `active`**, not before it exists. A
`planned` feature may be tickets only; a feature that shipped without one keeps its
record in `adr/` rather than getting one backfilled.

**Tickets renumber from `01` per directory.** Same-feature dependencies cite the bare
number; cross-feature dependencies and prose references cite the path or the feature
name, because a bare number means nothing outside its own directory.

**A size trigger:** past roughly a dozen tickets spanning unrelated phases, split
into child features rather than extending one number line.

## Rejected alternatives

**Cutting by tier** (server / infra-cicd / frontend / tests) left infra-cicd at ten
tickets — it renamed the problem rather than solving it. **Cutting by shippable
vertical slice** ("get something live", then hardening) would have given the best
milestone value but required rewriting ticket bodies, not moving them, discarding
dependency reasoning that was already correct.

The phase cut won on evidence: of 20 `Blocked by:` edges, only seven crossed a phase
boundary and every one pointed backwards to an earlier phase. The dependency graph
was already layered along the phases; the split only declared what was there.

**Dissolving `production-v1` entirely** — invariants to a new `reference/` file,
risks to `ROADMAP.md` — is the cleaner taxonomy, since a milestone would stop being
a directory under `work/`. Rejected because it scatters a set of rules that are only
comprehensible together, and orphans every external reference for no reader benefit.

**A size-based spec rule** ("required when a feature has more than one ticket")
would have demanded backfilling five shipped directories: archaeology on finished
work, producing documents whose decisions already live in ADRs.

**Keeping the global ticket numbers** (`server-restructure/issues/02..10`,
`build-and-deploy/issues/11,12,13,15,16`) would have preserved every existing
reference at zero rewrite risk. Rejected because the gaps read as lost files and it
abandons the "numbered from `01`" rule the tracker still states everywhere else.

## Consequences

Each child feature reports its own status, so `ROADMAP.md` can do the job it claims
in its first paragraph. The umbrella keeps one place that answers "what is v1.0" and
holds the invariants no single ticket owns.

The cost was paid once, at 58 reference edits against 28 file moves. That ratio is
the argument for the new citation rule: bare numbers are cheap to write and
expensive to move, so cross-feature references carry a path from now on.

`production-v1` stays `active` while its children are `planned` — an umbrella is
active for as long as any child is unfinished. Ticket `29 — docs sync` was dropped
before the split; [`docs-restructure`](../work/docs-restructure/spec.md) absorbed and
shipped it.

The prose in `docs-restructure`'s spec and plan still cites the old ticket numbers.
That is deliberate: they are the record of a decision made on 2026-09-09 and
rewriting them would falsify it. All such references are backtick spans, so
`scripts/check-doc-links.mjs` does not fail on them.
