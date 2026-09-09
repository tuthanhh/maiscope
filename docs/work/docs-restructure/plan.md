# Documentation Restructure Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restructure every document in this repository into one navigable tree with a single roadmap, no duplicated records of the same work, and automated checks that keep references from rotting.

**Architecture:** Documents are split by reader intent (Diátaxis-lite): `reference/` for lookup, `guides/` for tasks, `architecture.md` + `adr/` for understanding, `ROADMAP.md` for status, `work/` for in-flight tickets. README and CLAUDE.md become entry points that link out and never enumerate. A CI script fails the build when any Markdown link or file reference stops resolving.

**Tech Stack:** Markdown, `git mv` (history preservation), Node 18+ (link-checker script, no new dependencies), GitHub Actions.

**Spec:** `docs/work/docs-restructure/spec.md` — read it alongside this plan. Task 4 copies content directly from the spec's "ADRs to seed" section.

## Global Constraints

- **One question per document.** If two documents answer the same question, one is wrong. See the ownership table in the spec.
- **README never enumerates.** It links to `docs/ROADMAP.md`; it does not restate milestones or features.
- **No empty stubs.** `guides/` ships with exactly one file (`local-setup.md`). Do not create `deploy.md` or `runbook.md` — they are produced by production-v1 tickets 13, 14 and 20.
- **`docs/reference/simai-notation.md` already exists** (added outside this plan). Leave it in place and link it from the README documentation table; do not create a second simai document.
- **Use `git mv`**, never delete-and-recreate, so file history follows the move.
- **One commit per task**, message prefix `docs:`.
- **Feature status vocabulary** (in `spec.md`): `planned` → `active` → `shipped` → `superseded`.
- **Ticket status vocabulary** (in each issue file): `todo` → `in-progress` → `done` → `dropped`.
- **Never touch production.** This plan is documentation only; it changes no runtime behaviour.
- Repository is MIT licensed, `Copyright (c) 2026 Tu Thanh`.

## File Structure

| Path | Responsibility |
|---|---|
| `docs/ROADMAP.md` | Milestone definitions and the feature index with statuses. The only place work is enumerated. |
| `docs/architecture.md` | How the tiers fit together today: components, data flow, cross-tier keys. |
| `docs/adr/0000-template.md` | MADR-shaped template for new ADRs. |
| `docs/adr/0001..0008-*.md` | One decision each, append-only. |
| `docs/reference/api-contract.md` | HTTP contract (moved). |
| `docs/reference/schema.md` | Postgres schema (moved). |
| `docs/guides/local-setup.md` | Task: get the stack running locally (extracted from README). |
| `docs/agents/issue-tracker.md` | Convention: where tickets live, status strings (updated). |
| `docs/agents/domain.md` | Convention: domain docs and ADRs (updated). |
| `docs/work/<feature>/` | In-flight specs and tickets (moved from `.scratch/`). |
| `scripts/check-doc-links.mjs` | Fails when a Markdown link or file reference does not resolve. |
| `.github/workflows/docs.yml` | Runs the link checker on PRs. |
| `README.md` | Newcomer entry point. Links out. |
| `CLAUDE.md` | Agent entry point. Links out. |

---

### Task 1: Establish the tree and move existing documents

**Files:**
- Create: `docs/reference/`, `docs/guides/`, `docs/adr/`, `docs/work/`
- Move: `docs/api-contract.md` → `docs/reference/api-contract.md`
- Move: `docs/schema.md` → `docs/reference/schema.md`
- Move: `.scratch/<8 feature dirs>` → `docs/work/<feature>/`
- Modify: `.gitignore` (only if it references `.scratch/`)

**Interfaces:**
- Produces: the paths every later task writes into, and the paths Task 9's link checker validates.

**Context the executor needs:** `.scratch/` is tracked in git for five of its eight feature directories; `server-auth-github-oauth`, `server-contributions`, and `production-v1` are untracked. Untracked directories cannot be `git mv`'d — move them with `mv`, then `git add`. Mixing the two is one failure mode.

> **This task has already destroyed data once (2026-09-09).** Pre-creating
> `docs/work/<feature>/` before the move caused `git mv .scratch/X docs/work/X` to
> nest into `docs/work/X/X/`; the cleanup that followed deleted the untracked
> files permanently. Tracked files were recoverable from git, and two of the three
> untracked directories were recovered from Trash. `server-auth-github-oauth` was
> not, and its five tickets were reconstructed by hand from the surviving plan
> document.
>
> **The rule that prevents a repeat: create `docs/work` and nothing beneath it.**
> `git mv` and `mv` both move a source *into* an existing destination directory
> rather than becoming it.

- [ ] **Step 1: Record the current state so the move can be verified**

```bash
find docs .scratch -type f -name '*.md' | sort > /tmp/docs-before.txt
wc -l < /tmp/docs-before.txt   # expect 77
git ls-files .scratch docs | sort > /tmp/tracked-before.txt
```

If the count is not 77, do not proceed on the literal numbers in later steps —
compare against `/tmp/docs-before.txt` instead.

- [ ] **Step 2: Create the new directories — and no feature subdirectories**

```bash
mkdir -p docs/reference docs/guides docs/adr docs/work
```

Verify nothing exists under `docs/work` except the in-flight restructure feature:

```bash
ls docs/work    # expect exactly: docs-restructure
```

If any feature-named directory is already there, delete it **before** moving —
an existing destination is what caused the earlier data loss.

- [ ] **Step 3: Move the two reference documents with history**

```bash
git mv docs/api-contract.md docs/reference/api-contract.md
git mv docs/schema.md docs/reference/schema.md
```

- [ ] **Step 4: Move the five tracked feature directories with history**

```bash
for f in chart-playback-encapsulation frontend-server-side-search \
         server-catalog-typed-queries server-side-sheet-search server-sync-tier; do
  git mv ".scratch/$f" "docs/work/$f"
done
```

- [ ] **Step 5: Move the three untracked feature directories**

```bash
for f in server-auth-github-oauth server-contributions production-v1; do
  mv ".scratch/$f" "docs/work/$f"
done
git add docs/work
```

- [ ] **Step 6: Confirm `.scratch/` is empty and remove it**

```bash
find .scratch -type f | wc -l    # expect 0
rmdir .scratch
```

If this reports files remaining, stop and list them — an unaccounted-for directory means the inventory in the spec is wrong.

- [ ] **Step 7: Verify nothing was lost and history followed**

```bash
find docs -type f -name '*.md' | wc -l          # expect 77 — same total, new locations
diff <(find docs -type f -name '*.md' -printf '%f\n' | sort) \
     <(sed 's|.*/||' /tmp/docs-before.txt | sort) && echo "no file lost"
git log --follow --oneline docs/reference/api-contract.md | head -3
```

Expected: `no file lost`. Any line in the diff is a file that vanished in the
move — stop and recover it before continuing.

Expected: the log shows commits predating this task. If it shows only one commit, history did not follow — the move used `mv` instead of `git mv` on a tracked file.

- [ ] **Step 8: Check whether `.gitignore` mentions the old path**

```bash
grep -n 'scratch' .gitignore || echo "no reference — nothing to change"
```

If a line matches, delete it.

- [ ] **Step 9: Commit**

```bash
git add -A
git commit -m "docs: move reference docs and feature work into docs/ tree"
```

---

### Task 2: Write `docs/ROADMAP.md`

**Files:**
- Create: `docs/ROADMAP.md`
- Modify: `docs/work/production-v1/spec.md` (remove the milestone table; link to ROADMAP instead)

**Interfaces:**
- Consumes: `docs/work/<feature>/` paths from Task 1.
- Produces: `docs/ROADMAP.md`, referenced by README (Task 5) and CLAUDE.md (Task 6).

**Context the executor needs:** milestone definitions currently live in `docs/work/production-v1/spec.md`. They are roadmap-level, so they move here and the feature spec links to them. This is the "one question per document" rule in action — after this task, only ROADMAP answers "what is next".

- [ ] **Step 1: Write the roadmap**

Create `docs/ROADMAP.md` with exactly this content:

```markdown
# Roadmap

The single index of what is done, what is next, and in what order. Every other
document links here rather than restating it.

## Milestones

### v1.0 — public web app, browse-only

Catalog browsing, search and sheet details, deployed publicly. The visualizer is
reachable on desktop browsers and gated off mobile.

Out of scope, deliberately: audio hosting, authentication, contributions, a
desktop app, and a local SQLite cache. See [ADR-0002](adr/0002-chart-data-only-no-audio-hosting.md)
and [ADR-0003](adr/0003-web-pwa-drop-tauri.md).

### v1.1 — mobile visualizer

The Bevy visualizer running in a mobile browser and installed PWA. Gated on the
spike in `work/production-v1/issues/28-mobile-webview-spike.md`, whose outcome can
reopen [ADR-0003](adr/0003-web-pwa-drop-tauri.md).

### Phase 2 — community charts

GitHub authentication, the contribution and moderation queue, and
contributor-supplied audio. Unscheduled.

## Features

| Feature | Status | Milestone |
|---|---|---|
| [production-v1](work/production-v1/spec.md) | active | v1.0 |
| [docs-restructure](work/docs-restructure/spec.md) | active | — |
| [server-sync-tier](work/server-sync-tier/) | active | v1.0 |
| [server-auth-github-oauth](work/server-auth-github-oauth/) | planned | phase 2 |
| [server-contributions](work/server-contributions/) | planned | phase 2 |
| [server-catalog-typed-queries](work/server-catalog-typed-queries/) | shipped | — |
| [server-side-sheet-search](work/server-side-sheet-search/) | shipped | — |
| [frontend-server-side-search](work/frontend-server-side-search/) | shipped | — |
| [chart-playback-encapsulation](work/chart-playback-encapsulation/) | shipped | — |

## Conventions

- Feature status: `planned` → `active` → `shipped` → `superseded`
- Ticket status: `todo` → `in-progress` → `done` → `dropped`
- When a feature ships, its decisions move to [`adr/`](adr/), its behaviour
  changes land in [`reference/`](reference/), and this table collapses it to one
  line. Tickets stay in git but stop being roadmap surface.
```

- [ ] **Step 2: Remove the duplicated milestone definitions from the feature spec**

In `docs/work/production-v1/spec.md`, replace the "## Product shape" table and its
"### Why browse-only" subsection with:

```markdown
## Product shape

Milestones are defined in [`docs/ROADMAP.md`](../../ROADMAP.md). This feature
delivers **v1.0**.

**Explicitly out of v1**: audio hosting, auth, contributions, desktop app,
local SQLite cache, Tauri (any target).
```

- [ ] **Step 3: Verify no milestone definition survives in two places**

```bash
grep -rn "v1.1" docs/work/production-v1/spec.md
```

Expected: no output, or only references that link to ROADMAP. If the milestone
table is still there, Step 2 was not applied.

- [ ] **Step 4: Commit**

```bash
git add docs/ROADMAP.md docs/work/production-v1/spec.md
git commit -m "docs: add ROADMAP as the single work index"
```

---

### Task 3: Write `docs/architecture.md`

**Files:**
- Create: `docs/architecture.md`

**Interfaces:**
- Produces: `docs/architecture.md`, linked from README (Task 5) and CLAUDE.md (Task 6).

**Context the executor needs:** this describes the system **as it will be after production-v1**, with Tauri removed — because ADR-0003 has been decided and the README currently lies about it. Where current code contradicts the target, say so explicitly rather than describing a future as present tense.

- [ ] **Step 1: Write the architecture document**

Create `docs/architecture.md` with exactly this content:

````markdown
# Architecture

How the pieces fit together. Decisions and their rationale live in [`adr/`](adr/);
this document describes the result.

## Tiers

```
apps/host/     Vue 3 + Vite web app, PWA-installable — song browser and visualizer page
apps/server/   Rust/Axum + Postgres — chart catalog and per-sheet chart API
engine/        Bevy ECS chart renderer, compiled to wasm32-unknown-unknown
shared/        Scaffolded Rust crate for cross-workspace types — not yet wired in
```

One Cargo workspace (`apps/server`, `engine`, `shared`) plus a pnpm-managed
frontend under `apps/host`.

## Data flow

```
apps/host (browser)                    apps/server (Fly.io)
   │  GET /api/v1/catalog       ──────▶  song/sheet metadata (Neon Postgres)
   │  GET /api/v1/sheets/{s}/chart ───▶  raw simai chart text
   ▼
visualizer page
   │  load_chart(text) via wasm-bindgen
   ▼
engine (Bevy, wasm)
   parses simai → spawns notes → renders on a wall-clock-driven chart clock
```

## Cross-tier identity

`sheetExpr` = `` `${songId}|${type}|${difficulty}` ``. Computed client-side in
`apps/host/src/utils/sheet.ts:computeSheetExpr`; stored as the `sheet_expr` column
server-side. Almost every per-sheet endpoint keys on it, URL-encoded (`|` → `%7C`).

## Raw versus derived fields

The server returns **raw fields only**. It never sends `songNo`, `imageUrl`,
`imageUrlM`, `sheetExpr`, `notePercents`, or `$canonicalSheet` — the frontend
computes those in `apps/host/src/utils/data.ts:preprocessData`, which rehydrates
the catalog JSON into a frozen, prototype-linked object graph inherited from the
upstream arcade-songs data model. Keep new endpoints on this split.

## Client caching

Three layers with three lifetimes. Knowing which is which is the difference
between debugging staleness in one minute and three.

| Layer | Holds | Invalidated by |
|---|---|---|
| Service worker | App shell (precached); wasm (runtime, cache-first) | Content-hashed filenames; `index.html` is network-first |
| IndexedDB | Catalog blob (v1.1) | `GET /sync/manifest` revision change; paint-then-swap |
| HTTP | Wire responses | `ETag` / `If-None-Match` / `Cache-Control` |

## Engine boundary

`engine/src/lib.rs` is the wasm entry point (`#[wasm_bindgen(start)]`); the Bevy
app boots on module init and attaches to `<canvas id="bevy">`, which must already
be in the DOM.

`engine/src/wasm_bridge.rs` is the JS↔Bevy boundary: a lock-guarded mailbox. JS
pushes `SongPayload` / `EngineCommand` into a `Mutex<Vec<_>>`; the `apply_commands`
system (`engine/src/systems/visual/spawning.rs`) drains it once per frame. Add new
engine commands this way — never call into Bevy synchronously.

The chart clock has two modes. `load_chart(text)` advances on the wall clock
(`ChartPlayback::advance`); `load_song(text, bytes)` slaves the clock to the BGM
(`ChartPlayback::sync_to_audio_position`). v1 uses the silent path only — see
[ADR-0002](adr/0002-chart-data-only-no-audio-hosting.md).

Only sprite textures go through Bevy's `AssetServer`, fetched at runtime from
`/assets/...`. Chart text is pushed in directly through `wasm_bridge`.

## Deployment

| Concern | Choice |
|---|---|
| Static hosting | Cloudflare Pages |
| API | Axum in Docker on Fly.io, scale-to-zero |
| Database | Neon Postgres, separate from compute |
| Migrations | Fly `release_command`, before traffic cutover |
| Source of truth | Postgres for chart text; a private repo holds seed input and backups |

See [ADR-0001](adr/0001-postgres-source-of-truth-for-charts.md) and
[ADR-0004](adr/0004-fly-compute-neon-postgres.md).

## Known deviations from this document

`apps/host/src-tauri/` still exists and still proxies catalog fetches through
Rust. It is deleted by `work/production-v1/issues/21-delete-tauri.md`. Until that
lands, the desktop path bypasses HTTP caching entirely.
````

- [ ] **Step 2: Verify every ADR link points at a file Task 4 will create**

```bash
grep -o 'adr/[0-9]\{4\}-[a-z0-9-]*\.md' docs/architecture.md | sort -u
```

Expected: `adr/0001-postgres-source-of-truth-for-charts.md`,
`adr/0002-chart-data-only-no-audio-hosting.md`, `adr/0003-web-pwa-drop-tauri.md`,
`adr/0004-fly-compute-neon-postgres.md`. These filenames must match Task 4 exactly.

- [ ] **Step 3: Commit**

```bash
git add docs/architecture.md
git commit -m "docs: add architecture overview"
```

---

### Task 4: Seed the ADR trail

**Files:**
- Create: `docs/adr/0000-template.md`
- Create: `docs/adr/0001-postgres-source-of-truth-for-charts.md`
- Create: `docs/adr/0002-chart-data-only-no-audio-hosting.md`
- Create: `docs/adr/0003-web-pwa-drop-tauri.md`
- Create: `docs/adr/0004-fly-compute-neon-postgres.md`
- Create: `docs/adr/0005-restructure-server-in-place.md`
- Create: `docs/adr/0006-typed-sqlx-queries.md`
- Create: `docs/adr/0007-client-side-filtering.md`
- Create: `docs/adr/0008-docs-structure.md`
- Modify: `docs/work/production-v1/spec.md` (replace inline rationale with ADR links)

**Interfaces:**
- Consumes: the "ADRs to seed" section of `docs/work/docs-restructure/spec.md`, which contains the full body text for 0001–0008.
- Produces: the eight ADR paths that `architecture.md` and `ROADMAP.md` link to.

**Context the executor needs:** the spec already contains the Context / Decision / Rejected / Consequences prose for every one of these eight decisions, written from the sessions where they were made. **Do not paraphrase it.** Copy each spec subsection into its file under the template's headings. Inventing new rationale here would defeat the purpose — an ADR records what was actually decided and why.

- [ ] **Step 1: Write the template**

Create `docs/adr/0000-template.md`:

```markdown
# ADR-NNNN — <Title>

**Status:** proposed | accepted | superseded by [ADR-NNNN](nnnn-slug.md)
**Date:** YYYY-MM-DD

## Context

What forced a decision. The constraints and facts that were true at the time.

## Decision

What we chose, stated in one or two sentences.

## Rejected alternatives

What else was considered and the specific reason each was not chosen.

## Consequences

What this commits us to, including the costs. Note anything that would reopen
this decision.
```

- [ ] **Step 2: Write ADR-0001 as the worked example**

Create `docs/adr/0001-postgres-source-of-truth-for-charts.md`:

```markdown
# ADR-0001 — Postgres is the source of truth for chart text

**Status:** accepted
**Date:** 2026-09-09

## Context

Chart text existed only in an untracked local folder (16 songs, 196KB; the full
corpus is roughly 1500 songs, about 18MB of text). The alternative was a git
repository as the authority, with the database as a rebuildable serving layer.

## Decision

Postgres is the record of truth. A private repository holds the seed input and
the backups, never the authority.

## Rejected alternatives

- **Git repository as source of truth.** Rejected, but not for the reason first
  offered: 18MB of text is trivial for git, so scale was not the blocker.
- **The local folder as the backup.** True while all charts sit on one machine,
  false the moment a chart is added directly to production — and the loss would
  be silent during a restore.

## Consequences

- The load-bearing reason is phase 2: contributions write through `charts` /
  `chart_revisions` with a moderation queue and revision history, and git cannot
  be that write path.
- Backups become mandatory rather than optional — see
  [ADR-0004](0004-fly-compute-neon-postgres.md).
- `seed_songs` must fail loudly on unmatched titles instead of skipping silently.
- `ingest`'s `TRUNCATE` needs guarding against a malformed upstream snapshot.
```

- [ ] **Step 3: Write ADR-0002 through ADR-0008**

For each of 0002–0008, copy the correspondingly-numbered subsection from
`docs/work/docs-restructure/spec.md` § "ADRs to seed" into its file, mapping the
spec's bold labels onto the template headings:

| Spec label | ADR heading |
|---|---|
| **Context.** | `## Context` |
| **Decision.** | `## Decision` |
| **Rejected.** / **Why not rewrite.** / **Why.** | `## Rejected alternatives` |
| **Consequences.** | `## Consequences` |

All eight carry `**Status:** accepted` and `**Date:** 2026-09-09`, except:

- **0006** and **0007**, which record decisions already shipped — date them
  `2026-09-07` (the date of the plans that delivered them).
- **0003**, which must additionally carry this line at the end of its
  Consequences section, because its reversal condition is already known:

```markdown
**This decision reopens if the mobile WebView spike
(`work/production-v1/issues/28-mobile-webview-spike.md`) shows Bevy cannot hold
frame rate in an Android WebView.** In that case an APK bundling the wasm becomes
the only path to v1.1.
```

- [ ] **Step 4: Replace duplicated rationale in the production-v1 spec**

In `docs/work/production-v1/spec.md`, the "### Rejected, and why" and
"## Cross-cutting rules" sections restate reasoning that now lives in ADRs.
Replace "### Rejected, and why" entirely with:

```markdown
### Decisions behind this scope

| Decision | ADR |
|---|---|
| Postgres is the source of truth for chart text | [ADR-0001](../../adr/0001-postgres-source-of-truth-for-charts.md) |
| Chart data only, no audio hosting | [ADR-0002](../../adr/0002-chart-data-only-no-audio-hosting.md) |
| Web + PWA, Tauri dropped | [ADR-0003](../../adr/0003-web-pwa-drop-tauri.md) |
| Fly compute + Neon Postgres | [ADR-0004](../../adr/0004-fly-compute-neon-postgres.md) |
| Restructure the server in place | [ADR-0005](../../adr/0005-restructure-server-in-place.md) |
```

Keep "## Cross-cutting rules" — those are operating rules, not rationale.

- [ ] **Step 5: Verify the ADR set is complete and internally linked**

```bash
ls docs/adr/ | wc -l                                  # expect 9 (template + 8)
grep -L '^## Consequences' docs/adr/0*.md             # expect only 0000-template.md
grep -c 'reopens' docs/adr/0003-web-pwa-drop-tauri.md # expect 1
```

- [ ] **Step 6: Commit**

```bash
git add docs/adr docs/work/production-v1/spec.md
git commit -m "docs: seed ADR trail for decisions 0001-0008"
```

---

### Task 5: Rewrite `README.md` and extract the setup guide

**Files:**
- Modify: `README.md` (full rewrite)
- Create: `docs/guides/local-setup.md`

**Interfaces:**
- Consumes: `docs/ROADMAP.md` (Task 2), `docs/architecture.md` (Task 3), `docs/adr/` (Task 4).
- Produces: `docs/guides/local-setup.md`, the only file in `guides/`.

**Context the executor needs:** the current README describes a Tauri desktop app, lists a roadmap that contradicts ROADMAP.md, and credits `zetaraku/arcade-songs` in prose only. Prose credit does not satisfy an MIT licence's notice-retention requirement — that gap is tracked as `work/production-v1/issues/01-repo-hygiene-and-license.md` and is **not** fixed here; this task only stops the README from being the sole compliance mechanism.

- [ ] **Step 1: Write the setup guide**

Create `docs/guides/local-setup.md`:

````markdown
# Get maiscope running locally

## Prerequisites

- [Rust toolchain](https://rustup.rs/) (stable, edition 2024) with the
  `wasm32-unknown-unknown` target
- `wasm-bindgen-cli` **0.2.122 exactly** — it must match the `wasm-bindgen` crate
  version pinned in `engine/Cargo.toml`, or bindgen fails with a schema-version
  mismatch
- [Node.js](https://nodejs.org/) 18+ and [pnpm](https://pnpm.io/)
- Docker (for local Postgres) and [`sqlx-cli`](https://github.com/launchbadge/sqlx/tree/main/sqlx-cli)

## 1. Backend

```sh
cd apps/server
cp .env.example .env          # values must match docker-compose.yml
docker compose up -d          # Postgres on :5432
sqlx migrate run
cargo run --bin ingest        # load the catalog
cargo run                     # API on :3000
```

## 2. Engine

```sh
./engine/build-wasm.sh        # dev build; pass `release` for a size-optimised one
```

Output lands in `apps/host/src/wasm/` (gitignored — rebuild after any change under
`engine/`). Artifacts build through the root workspace, so they appear in the root
`target/`, not `engine/target/`.

## 3. Frontend

```sh
cd apps/host
pnpm install
pnpm dev                      # Vite dev server
```

Set `VITE_API_BASE_URL` if the server is not at the default
`http://localhost:3000/api/v1` — see `apps/host/src/app/game.ts`.

## Troubleshooting

**Blank canvas on the visualizer page.** The Bevy app attaches to
`<canvas id="bevy">` on module init; the element must exist in the DOM first.

**`wasm-bindgen` schema error.** The CLI and crate versions differ. Check
`engine/Cargo.toml` and install the matching CLI version.
````

- [ ] **Step 2: Rewrite the README**

Replace the entire contents of `README.md` with:

````markdown
# maiscope

A song browser and real-time chart visualizer for **maimai** (SEGA's arcade
rhythm game), built as a Rust/TypeScript monorepo: a Vue web app, a Rust backend
serving a community chart database, and a Bevy engine compiled to WebAssembly for
in-app chart playback.

## What's here

```
apps/host/     Vue 3 + Vite web app (PWA-installable) — song browser, visualizer page
apps/server/   Rust/Axum + Postgres — chart catalog and per-sheet chart API
engine/        Bevy ECS chart renderer, compiled to wasm and imported by apps/host
shared/        Rust crate for cross-workspace types (scaffolded, not yet wired in)
```

For how these fit together, see [docs/architecture.md](docs/architecture.md).

## Getting started

See [docs/guides/local-setup.md](docs/guides/local-setup.md).

## Features

- Song gallery with search and filtering (level, category, version, BPM, region)
- Per-sheet detail view: difficulty, internal level, note designer, note counts
- Chart visualizer: taps, holds, touch notes, slides with star and path traces
- Light/dark mode; UI in EN, JA, KO, ZH-Hans, ZH-Hant, VI, ES, ID, RU

## Documentation

| | |
|---|---|
| [Roadmap](docs/ROADMAP.md) | What is done, what is next |
| [Architecture](docs/architecture.md) | How the tiers fit together |
| [Decisions](docs/adr/) | Why it is built this way |
| [API contract](docs/reference/api-contract.md) | The HTTP surface |
| [Database schema](docs/reference/schema.md) | Postgres tables |
| [simai notation](docs/reference/simai-notation.md) | The chart text format |

## Acknowledgments

- **[zetaraku/arcade-songs](https://github.com/zetaraku/arcade-songs)** — the web
  app this frontend is ported from. Data model, filtering logic and UI design
  credit belong to the upstream project.
- [mai-notes.com](https://mai-notes.com/) and [majdata.net](https://majdata.net/) —
  simai format and note-timing references
- [maimai でらっくす 公式サイト｜セガ](https://maimai.sega.jp/) — official song information

## License

MIT — see [LICENSE](./LICENSE).
````

- [ ] **Step 3: Verify the README no longer enumerates work**

```bash
grep -niE 'roadmap|- \[ \]|sqlite|tauri|contribution flow' README.md
```

Expected: exactly one match — the `[Roadmap](docs/ROADMAP.md)` table row. Any
other hit means stale content survived.

- [ ] **Step 4: Verify every README link resolves**

```bash
grep -o '(\(docs\|\./\)[^)]*)' README.md | tr -d '()' | while read -r p; do
  [ -e "$p" ] || echo "BROKEN: $p"
done
```

Expected: no output.

- [ ] **Step 5: Commit**

```bash
git add README.md docs/guides/local-setup.md
git commit -m "docs: rewrite README as an entry point, extract setup guide"
```

---

### Task 6: Rewrite `CLAUDE.md`

**Files:**
- Modify: `CLAUDE.md` (full rewrite)

**Interfaces:**
- Consumes: every path created in Tasks 1–5.

**Context the executor needs:** the current CLAUDE.md is stale in five specific ways — it points at `apps/server/docs/*` (moved in Task 1), claims no test suite exists beyond `shared` (six `#[sqlx::test]` handler tests live at `apps/server/src/main.rs:484+`), documents the Tauri proxy, lists `pnpm tauri dev`, and points at `.scratch/`. All five are corrected here.

- [ ] **Step 1: Rewrite the file**

Replace the entire contents of `CLAUDE.md` with:

````markdown
# CLAUDE.md

Guidance for Claude Code when working in this repository.

## Always read first

- [`docs/ROADMAP.md`](docs/ROADMAP.md) — what is being built now and what is deferred
- [`docs/architecture.md`](docs/architecture.md) — how the tiers fit together
- [`docs/adr/`](docs/adr/) — why decisions were made; check before proposing a change that contradicts one

Conditionally relevant:

- [`docs/reference/api-contract.md`](docs/reference/api-contract.md) — read before adding or changing any server endpoint
- [`docs/reference/schema.md`](docs/reference/schema.md) — read before touching tables or migrations
- `engine/build-wasm.sh` — only when changing anything under `engine/` or the wasm-bindgen pin

## What this is

maiscope: a web app for browsing maimai songs and visualizing charts. Rust/TypeScript
monorepo — one Cargo workspace (`apps/server`, `engine`, `shared`) plus a
pnpm-managed frontend under `apps/host`.

The frontend is a port of [zetaraku/arcade-songs](https://github.com/zetaraku/arcade-songs)
(originally Nuxt 2 + Vuetify 2), rebuilt on native Vue 3 for maimai only.

## Commands

### Backend (`apps/server`)

```sh
docker compose up -d               # Postgres on :5432
sqlx migrate run
cargo run --bin ingest             # load catalog into canonical tables
cargo run                          # API on :3000
cargo run --bin seed_songs         # load chart text, keyed by sheet_expr
```

### Engine

```sh
./engine/build-wasm.sh             # dev build; pass `release` for size-optimised
```

Output goes to `apps/host/src/wasm/` (gitignored). The `wasm-bindgen-cli` version
must match the crate version in `engine/Cargo.toml` **exactly** (pinned `=0.2.122`)
or bindgen fails on a schema mismatch.

### Frontend (`apps/host`)

```sh
pnpm install
pnpm dev                           # Vite dev server
pnpm build                         # vue-tsc --noEmit && vite build
```

### Workspace

```sh
cargo build --workspace            # excludes the engine's wasm target
cargo test --workspace
```

There is no lint or format config (no eslint/prettier). Do not assume `pnpm lint`
exists.

## Testing

Six `#[sqlx::test]` handler tests live in `apps/server/src/main.rs`. `engine/` and
`apps/host/` have no tests yet — growing that is tracked in
[`docs/work/production-v1/`](docs/work/production-v1/) issues 26 and 27.

## Conventions

**Cross-tier key.** `sheetExpr = ${songId}|${type}|${difficulty}`. Frontend:
`utils/sheet.ts:computeSheetExpr`. Backend: the `sheet_expr` column. URL-encoded
in paths (`|` → `%7C`).

**Raw versus derived.** The server returns raw fields only; the frontend computes
`songNo`, `imageUrl`, `sheetExpr`, `notePercents`, `$canonicalSheet` in
`utils/data.ts:preprocessData`. Keep new endpoints on this split.

**Engine boundary.** JS↔Bevy goes through the mailbox in `engine/src/wasm_bridge.rs`,
drained once per frame by `apply_commands`. Never call into Bevy synchronously.

## Rules

- **Never touch production.** Everything local: dev Postgres via `docker compose`,
  local wasm builds. If a task implies a production system, stop and ask.
- **Doc sync is part of the ticket that changes behaviour**, not a trailing ticket.
  Changing an endpoint means updating `docs/reference/api-contract.md` in the same
  change; changing the schema means `docs/reference/schema.md`.
- **Record decisions as ADRs.** A choice with a rejected alternative belongs in
  `docs/adr/`, not buried in a spec.

## Agent skills

**Issue tracker.** Work lives as markdown under `docs/work/<feature-slug>/`.
See [`docs/agents/issue-tracker.md`](docs/agents/issue-tracker.md).

**Domain docs.** See [`docs/agents/domain.md`](docs/agents/domain.md).
````

- [ ] **Step 2: Verify every stale claim is gone**

```bash
grep -niE 'apps/server/docs|tauri|\.scratch|only crate with tests' CLAUDE.md
```

Expected: no output. Any hit is a stale claim that survived.

- [ ] **Step 3: Verify every link resolves**

```bash
grep -o '(\(docs\|\./\)[^)]*)' CLAUDE.md | tr -d '()' | while read -r p; do
  [ -e "$p" ] || echo "BROKEN: $p"
done
```

Expected: no output.

- [ ] **Step 4: Commit**

```bash
git add CLAUDE.md
git commit -m "docs: rewrite CLAUDE.md as an entry point, correct stale claims"
```

---

### Task 7: Update agent conventions and normalise ticket statuses

**Files:**
- Modify: `docs/agents/issue-tracker.md`
- Modify: `docs/agents/domain.md`
- Modify: all `docs/work/*/issues/*.md` (status lines)
- Modify: `docs/work/production-v1/issues/29-docs-sync.md`
- Modify: `docs/work/server-sync-tier/issues/05-update-contract-docs.md`
- Modify: `docs/work/production-v1/issues/{01,26,28}-*.md`

**Interfaces:**
- Consumes: the status vocabularies from Global Constraints.

**Context the executor needs:** three status strings are in use (`todo`, `done`, `ready-for-agent`), and `issue-tracker.md` points at a `triage-labels.md` that does not exist. Separately, the user deleted `songs/` on 2026-09-09, which invalidates assumptions in three production-v1 tickets.

- [ ] **Step 1: Update the issue-tracker convention**

In `docs/agents/issue-tracker.md`, apply three changes:

Replace every occurrence of `.scratch/` with `docs/work/`.

Replace the triage line:

```markdown
- Triage state is recorded as a `Status:` line near the top of each issue file (see `triage-labels.md` for the role strings)
```

with:

```markdown
- Triage state is recorded as a `Status:` line near the top of each issue file.
  Ticket statuses: `todo` → `in-progress` → `done` → `dropped`.
  The parent `spec.md` carries a feature status: `planned` → `active` → `shipped` → `superseded`.
```

Add, after the conventions list:

```markdown
- When a feature ships, extract its decisions into `docs/adr/`, update
  `docs/reference/`, set the feature status to `shipped`, and collapse its entry
  in `docs/ROADMAP.md` to a single line.
```

- [ ] **Step 2: Soften the dead references in the domain convention**

In `docs/agents/domain.md`, replace the "## Before exploring, read these" list with:

```markdown
- **`docs/adr/`** — read the ADRs that touch the area you are about to work in.
  This directory exists and is the authoritative record of past decisions.
- **`CONTEXT.md`** at the repo root, if it exists — a domain glossary. It does not
  exist yet; the `/domain-modeling` skill creates it lazily when terms actually
  get resolved. Proceed silently if it is absent.
```

- [ ] **Step 3: Normalise `ready-for-agent` to `todo`**

```bash
grep -rl 'ready-for-agent' docs/work/ | while read -r f; do
  sed -i 's/ready-for-agent/todo/g' "$f"
done
grep -rn 'ready-for-agent' docs/work/ || echo "clean"
```

Expected final line: `clean`.

- [ ] **Step 4: Mark the two absorbed doc tickets as dropped**

In `docs/work/production-v1/issues/29-docs-sync.md`, set the status line to
`**Status:** dropped` and add immediately below it:

```markdown
**Dropped:** absorbed by the `docs-restructure` feature — see
[`docs/work/docs-restructure/spec.md`](../../docs-restructure/spec.md). That work
covers every item this ticket listed.
```

In `docs/work/server-sync-tier/issues/05-update-contract-docs.md`, set the status
line to `**Status:** dropped` and add the same pointer.

- [ ] **Step 5: Correct the three tickets invalidated by the deleted corpus**

The user deleted `songs/` on 2026-09-09, removing 16 `maidata.txt` files (196KB)
along with the mp3s. Apply these edits:

In `01-repo-hygiene-and-license.md`, replace the `songs/` gitignore checklist items with:

```markdown
- [ ] `songs/` added to `.gitignore` **pre-emptively** — the directory was deleted
      on 2026-09-09, but the same layout returns whenever a song pack is unpacked
      locally, and it must never be committed
- [ ] Confirm nothing under `songs/` was ever committed (`git log --all -- songs/`)
```

In `26-engine-parser-tests.md`, replace the corpus checklist item with:

```markdown
- [ ] Re-acquire a chart corpus (the original 16 `maidata.txt` were deleted with
      the audio on 2026-09-09) into the private data repo, chart text only
- [ ] Corpus test: parse every acquired `maidata.txt` without panicking, snapshot
      the event counts per difficulty
- [ ] Fixtures vendored into `engine/tests/fixtures/` — chart text only, never
      mp3 or bg images
```

In `28-mobile-webview-spike.md`, replace the "one of the 16 corpus charts" item with:

```markdown
- [ ] One chart loaded end to end (requires the corpus re-acquired in issue 26, or
      a single `maidata.txt` obtained ad hoc for the spike)
```

- [ ] **Step 6: Verify one vocabulary is in use**

```bash
grep -rhoE '^\*\*Status:\*\* *[a-z-]+' docs/work/ | sort -u
```

Expected: only `todo`, `in-progress`, `done`, `dropped`, `planned`, `active`,
`shipped`, `superseded`. Anything else is an unmigrated status string.

- [ ] **Step 7: Commit**

```bash
git add docs/agents docs/work
git commit -m "docs: normalise ticket statuses, update agent conventions"
```

---

### Task 8: Delete the retired plans directory

**Files:**
- Delete: `docs/superpowers/plans/` (7 files)

**Context the executor needs:** these seven plan documents describe the same work as the feature directories in `docs/work/`, and two of them (`server-auth-github-oauth`, `server-contributions`) plan work now deferred to phase 2. Git history retains them; this is recoverable.

- [ ] **Step 1: Record what is being deleted, in the commit message body**

```bash
ls docs/superpowers/plans/
```

Expected: seven files dated `2026-09-07`.

- [ ] **Step 2: Confirm no surviving document links into the directory**

```bash
grep -rn 'superpowers/plans' --include='*.md' . | grep -v '^./docs/work/docs-restructure/'
```

Expected: no output. If a feature spec references a plan, replace that reference
with a link to the feature directory before deleting.

- [ ] **Step 3: Delete**

```bash
git rm -r docs/superpowers/plans
rmdir docs/superpowers 2>/dev/null || true
```

- [ ] **Step 4: Commit**

```bash
git commit -m "docs: retire docs/superpowers/plans, superseded by docs/work

The seven plans duplicated the feature directories now in docs/work/.
Two of them planned work since deferred to phase 2. Recoverable from history."
```

---

### Task 9: Add the CI link checker

**Files:**
- Create: `scripts/check-doc-links.mjs`
- Create: `.github/workflows/docs.yml`
- Test: run the script against a deliberately broken link

**Interfaces:**
- Consumes: every path created in Tasks 1–8.
- Produces: a check that fails CI when a Markdown link stops resolving.

**Context the executor needs:** this catches the exact failure class the repo already had — CLAUDE.md pointing at moved paths, `domain.md` pointing at absent files. It checks relative links only; external URLs are out of scope (they fail for reasons unrelated to this repo, and would make CI flaky).

- [ ] **Step 1: Write the failing test fixture**

```bash
mkdir -p /tmp/link-test
printf '# Broken\n\nSee [missing](./does-not-exist.md).\n' > /tmp/link-test/broken.md
```

- [ ] **Step 2: Write the checker**

Create `scripts/check-doc-links.mjs`:

```javascript
#!/usr/bin/env node
// Fails when a relative Markdown link or image path does not resolve on disk.
// External URLs (http/https/mailto) and pure anchors are skipped: they fail for
// reasons unrelated to this repository and would make CI flaky.

import { readdir, readFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { join, dirname, resolve, extname } from "node:path";

const IGNORED_DIRS = new Set(["node_modules", "target", ".git", "dist"]);
const LINK = /\[[^\]]*\]\(([^)\s]+)(?:\s+"[^"]*")?\)/g;

async function markdownFiles(dir) {
  const found = [];
  for (const entry of await readdir(dir, { withFileTypes: true })) {
    if (entry.name.startsWith(".") && entry.name !== ".github") continue;
    if (IGNORED_DIRS.has(entry.name)) continue;
    const full = join(dir, entry.name);
    if (entry.isDirectory()) found.push(...(await markdownFiles(full)));
    else if (extname(entry.name) === ".md") found.push(full);
  }
  return found;
}

const root = process.argv[2] ?? ".";
const failures = [];

for (const file of await markdownFiles(root)) {
  const body = await readFile(file, "utf8");
  const lines = body.split("\n");

  for (const [index, line] of lines.entries()) {
    for (const match of line.matchAll(LINK)) {
      const target = match[1];
      if (/^(https?:|mailto:|#)/.test(target)) continue;

      const path = target.split("#")[0];
      if (path === "") continue;

      if (!existsSync(resolve(dirname(file), path))) {
        failures.push(`${file}:${index + 1}: broken link -> ${target}`);
      }
    }
  }
}

if (failures.length > 0) {
  console.error(`${failures.length} broken reference(s):\n`);
  for (const f of failures) console.error(`  ${f}`);
  process.exit(1);
}

console.log("All relative Markdown links resolve.");
```

- [ ] **Step 3: Run it against the broken fixture to verify it fails**

```bash
node scripts/check-doc-links.mjs /tmp/link-test
```

Expected: exit code 1, output containing
`broken.md:3: broken link -> ./does-not-exist.md`.

If it exits 0, the checker is not detecting anything and the rest of this task is
worthless — fix it before continuing.

- [ ] **Step 4: Run it against the repository**

```bash
node scripts/check-doc-links.mjs .
echo "exit: $?"
```

Expected: exit 0 and `All relative Markdown links resolve.`

If it reports failures, fix each one — these are real broken references created or
missed by Tasks 1–8, which is exactly what this task exists to catch.

- [ ] **Step 5: Add the workflow**

Create `.github/workflows/docs.yml`:

```yaml
name: docs

on:
  pull_request:
    paths:
      - "**/*.md"
      - "scripts/check-doc-links.mjs"
      - ".github/workflows/docs.yml"
  push:
    branches: [master]

jobs:
  links:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: "20"
      - name: Check Markdown links
        run: node scripts/check-doc-links.mjs .
```

- [ ] **Step 6: Clean up the fixture**

```bash
rm -rf /tmp/link-test
```

- [ ] **Step 7: Commit**

```bash
git add scripts/check-doc-links.mjs .github/workflows/docs.yml
git commit -m "docs: add CI check for broken Markdown references"
```

---

### Task 10: Close out the feature

**Files:**
- Modify: `docs/work/docs-restructure/spec.md`
- Modify: `docs/ROADMAP.md`

- [ ] **Step 1: Verify the whole structure matches the spec**

```bash
test -f docs/ROADMAP.md && test -f docs/architecture.md \
  && test -d docs/adr && test -d docs/reference \
  && test -d docs/guides && test -d docs/work \
  && test ! -d .scratch && test ! -d docs/superpowers \
  && echo "structure OK"
```

Expected: `structure OK`.

- [ ] **Step 2: Verify `guides/` has exactly one file**

```bash
ls docs/guides | wc -l    # expect 1
```

A second file means an empty stub was created against the Global Constraints.

- [ ] **Step 3: Run the link checker one final time**

```bash
node scripts/check-doc-links.mjs .
```

Expected: exit 0.

- [ ] **Step 4: Set the feature to shipped**

In `docs/work/docs-restructure/spec.md`, change `**Status:** planned` to
`**Status:** shipped`.

In `docs/ROADMAP.md`, move the `docs-restructure` row's status from `active` to
`shipped`.

- [ ] **Step 5: Commit**

```bash
git add docs/work/docs-restructure/spec.md docs/ROADMAP.md
git commit -m "docs: mark docs-restructure shipped"
```

---

## Self-Review

**Spec coverage.** Every spec section maps to a task: target structure and file
mapping → Task 1; ownership and milestone move → Task 2; architecture → Task 3;
ADRs to seed → Task 4; stale README → Task 5; stale CLAUDE.md → Task 6; lifecycle,
status vocabulary, agent conventions, `songs/` fallout → Task 7; retire plans →
Task 8; keeping-it-current mechanisms → Task 9; archive rule applied to this
feature itself → Task 10.

Two spec items are deliberately **not** tasks, and both are marked out of scope in
the spec: the contract snapshot test (production-v1 ticket 27 owns it) and
`/quality-sync-docs` (a post-migration verification pass, not migration work).

**Placeholders.** None. Every document's full text is inline except ADRs 0002–0008,
which Task 4 copies from a named spec section with an explicit label-to-heading
mapping — the spec travels with the plan by design.

**Consistency.** ADR filenames in Task 3's `architecture.md` match Task 4's
creation list exactly (verified by Task 3 Step 2). Status vocabularies in Global
Constraints match Task 7's normalisation and Task 10's close-out. The
`docs-restructure` feature row appears in Task 2's ROADMAP table and is updated by
Task 10.

**One risk worth naming:** Task 1 Step 7 expects 56 Markdown files, which assumes
no other document is added between now and execution. If the count differs, compare
against `/tmp/docs-before.txt` rather than trusting the literal number.
