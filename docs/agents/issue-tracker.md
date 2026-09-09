# Issue tracker: Local Markdown

The rules below are decided in [ADR-0008](../adr/0008-docs-structure.md) and
[ADR-0009](../adr/0009-umbrella-features-and-spec-timing.md).

Issues and specs (you may know a spec as a PRD) for this repo live as markdown files in `docs/work/`.

## Conventions

- One feature per directory: `docs/work/<feature-slug>/`
- The spec is `docs/work/<feature-slug>/spec.md`. It is **required before a
  feature goes `active`** — a `planned` feature may be tickets only until it is
  scheduled, and a feature that shipped without one keeps its record in
  [`docs/adr/`](../adr/) rather than getting a spec backfilled.
- An optional `plan.md` sits beside the spec when a feature needs a written
  implementation plan
- An **umbrella** feature owns no `issues/` — it holds the rules and risks its
  child features share and indexes them. [`production-v1`](../work/production-v1/spec.md)
  is the example.
- Implementation issues are one file per ticket at `docs/work/<feature-slug>/issues/<NN>-<slug>.md`, numbered from `01` — never a single combined tickets file
- Triage state is recorded as a `**Status:**` line near the top of each issue file.
  Ticket statuses: `todo` → `in-progress` → `done` → `dropped`.
  The parent `spec.md` carries a feature status: `planned` → `active` → `shipped` → `superseded`.
- Dependencies go on a `**Blocked by:**` line. Within the same feature, cite the
  bare number (`03, 04`); across features, cite the path
  (`` `server-restructure/issues/04-split-modules.md` ``) — a bare number means
  nothing outside its own directory. Prose references follow the same rule:
  `issue 07` inside the feature, `` `server-restructure` issue 07 `` outside it.
- Keep a feature small enough that its tickets share one dependency chain. When a
  directory grows past roughly a dozen tickets spanning unrelated phases, split it
  into child features under an umbrella instead of letting one number line grow.
- Comments and conversation history append to the bottom of the file under a `## Comments` heading
- When a feature ships, extract its decisions into `docs/adr/`, update
  `docs/reference/`, set the feature status to `shipped`, and collapse its entry
  in `docs/ROADMAP.md` to a single line.

## When a skill says "publish to the issue tracker"

Create a new file under `docs/work/<feature-slug>/` (creating the directory if needed).

## When a skill says "fetch the relevant ticket"

Read the file at the referenced path. The user will normally pass the path or the issue number directly.

## Wayfinding operations

Used by `/wayfinder`. The **map** is a file with one **child** file per ticket.

- **Map**: `docs/work/<effort>/map.md` — the Notes / Decisions-so-far / Fog body.
- **Child ticket**: `docs/work/<effort>/issues/NN-<slug>.md`, numbered from `01`, with the question in the body. A `Type:` line records the ticket type (`research`/`prototype`/`grilling`/`task`); a `Status:` line records `claimed`/`resolved`.
- **Blocking**: a `Blocked by: NN, NN` line near the top. A ticket is unblocked when every file it lists is `resolved`.
- **Frontier**: scan `docs/work/<effort>/issues/` for files that are open, unblocked, and unclaimed; first by number wins.
- **Claim**: set `Status: claimed` and save before any work.
- **Resolve**: append the answer under an `## Answer` heading, set `Status: resolved`, then append a context pointer (gist + link) to the map's Decisions-so-far in `map.md`.
