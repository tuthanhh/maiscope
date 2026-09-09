# Spec — Documentation restructure

**Status:** planned
**Date:** 2026-09-09

Unify the project's documentation into one structure with a single roadmap, no
duplicated records, and mechanisms that keep it current.

This spec is the first artifact written under the structure it describes, at
`docs/work/<feature>/spec.md`.

## Problem

Three overlapping systems record the same work, and none of them owns the truth.

| Location | Contents | Problem |
|---|---|---|
| `README.md` | Overview + roadmap | Roadmap describes a Tauri desktop app, a local SQLite cache and contributions-next — all since reversed |
| `CLAUDE.md` | Agent instructions | Points at `apps/server/docs/*` (moved to `docs/`); claims no tests exist beyond `shared` (six exist at `apps/server/src/main.rs:484+`); documents the Tauri proxy (being deleted) |
| `docs/api-contract.md`, `docs/schema.md` | The real contracts | Contract still specifies audio upload and S3 presigning, both cut |
| `docs/agents/*.md` | Conventions for skills | Reference `CONTEXT.md`, `docs/adr/` and `triage-labels.md` — none exist |
| `docs/superpowers/plans/` | 7 implementation plans | Same work as `.scratch/*/issues/` in a second format; at least two describe deferred work |
| `.scratch/*/issues/` | 8 features, ~70 tickets | No status index. A directory named "scratch" is the durable tracker, and three of its eight features are uncommitted. |

Ranked by the user: **(A)** no single roadmap → **(B)** duplication → **(C)**
staleness. A is the cause; B and C follow from it.

**Evidence that a doc-sync *rule* is not enough:** CLAUDE.md already requires
updating `api-contract.md` in the same change. The contract is stale anyway,
because doc-sync was filed as trailing ticket `05` of the sync-tier feature and
never done. Four other features used the same pattern.

## Decisions

| # | Decision |
|---|---|
| 1 | Fix all three problems, roadmap ownership first |
| 2 | Milestone-first roadmap at `docs/ROADMAP.md`; feature directory stays the unit of work; README links, never enumerates |
| 3 | Diátaxis-lite audience split — reference / guides / explanation — over topic-first or full Diátaxis |
| 4 | Feature dirs are the single tracker, promoted to `docs/work/`; `docs/superpowers/plans/` retired |

Approach 3 follows [Diátaxis](https://diataxis.fr/) but collapses its four
quadrants to what this repo actually has. The docs here are ~90% internal
engineering — reference, explanation, and process. There is no tutorial audience
beyond README's getting-started, so full Diátaxis would create empty folders.
[Backstage](https://github.com/backstage/backstage/tree/master/docs) is the
closest real-world precedent: numbered decision records alongside a small number
of purposeful folders.

## Target structure

```
README.md                     newcomer entry: what it is, how to run it. Links out.
CLAUDE.md                     agent entry: read-first list, commands, rules. Links out.
docs/
  ROADMAP.md                  milestones → features → status. The only place work is enumerated.
  architecture.md             tiers, data flow, cross-tier keys
  adr/                        one numbered file per decision; append-only
  reference/                  api-contract.md, schema.md
  guides/                     local-setup.md
  agents/                     domain.md, issue-tracker.md
  work/                       <feature>/spec.md + issues/NN-*.md
```

### File mapping

| Now | Then |
|---|---|
| `docs/api-contract.md` | `docs/reference/api-contract.md` |
| `docs/schema.md` | `docs/reference/schema.md` |
| `docs/agents/{domain,issue-tracker}.md` | unchanged path, contents updated |
| `.scratch/<8 features>/` | `docs/work/<feature>/` |
| `docs/superpowers/plans/` (7 files) | deleted; git history retains them |
| README getting-started steps | `docs/guides/local-setup.md` |
| — | new: `ROADMAP.md`, `architecture.md`, `adr/` |

### Restraints

- `guides/` starts with **one** file. `deploy.md` and `runbook.md` are outputs of
  production-v1 tickets 13, 14 and 20; empty stubs are what make docs look
  abandoned.
- No `reference/simai-format.md`. That content does not exist; writing it is a
  task for the engine work, not a move.

## Ownership

One question per document. If two documents answer the same question, one is wrong.

| Doc | The one question it answers |
|---|---|
| `README.md` | What is this and how do I run it? |
| `CLAUDE.md` | What must an agent read and obey here? |
| `ROADMAP.md` | What is done, what is next, in what order? |
| `architecture.md` | How do the pieces fit together today? |
| `adr/NNNN-*.md` | Why is it this way, and what was rejected? |
| `reference/*` | What exactly does it do — contract, schema? |
| `guides/*` | How do I perform this task? |
| `work/<feature>/` | What are we building now, in what steps? |

**Information flow.** A decision is made → an ADR records why. Behaviour changes →
`reference/` records what. Work is planned → `work/` records the steps. Where we
are → `ROADMAP.md`, and nowhere else.

**Milestones move.** v1.0 / v1.1 / phase 2 are currently defined inside
`.scratch/production-v1/spec.md`. They are roadmap-level: their definitions move
to `ROADMAP.md` and the feature spec links to them.

## Lifecycle and status

Two vocabularies, replacing the three strings in use today (`todo`, `done`,
`ready-for-agent`):

- **Feature**, in `spec.md`: `planned` → `active` → `shipped` → `superseded`
- **Ticket**, in each issue file: `todo` → `in-progress` → `done` → `dropped`

`ready-for-agent` collapses into `todo`. The dangling `triage-labels.md`
reference in `agents/issue-tracker.md` is deleted rather than fulfilled — two
four-word vocabularies do not need their own document.

**Archive rule.** When a feature ships: extract any real decision into an ADR,
update `reference/`, set `Status: shipped`. `ROADMAP.md` collapses it to one line
with a link. Tickets stay in git but stop being roadmap surface. This is what
prevents `work/` from becoming the next `.scratch/`.

### Current feature states

| Feature | Tickets | New status |
|---|---|---|
| `chart-playback-encapsulation` | 1 | shipped |
| `frontend-server-side-search` | 6 | shipped |
| `server-catalog-typed-queries` | 8 | shipped |
| `server-side-sheet-search` | 3 | shipped |
| `server-sync-tier` | 5 | active — 1 open ticket (`05-update-contract-docs`) |
| `server-auth-github-oauth` | 5 | planned (phase 2) |
| `server-contributions` | 5 | planned (phase 2) |
| `production-v1` | 29 | active |
| `docs-restructure` | this spec | planned |

The single open sync-tier ticket is the doc-sync ticket. Its work is absorbed
here; it is marked `dropped` with a pointer to this spec.

## ADRs to seed

Decisions already made and currently recorded nowhere durable. Each entry below
is the substance the ADR file must carry — context, the decision, what was
rejected, and what it commits us to. Use MADR-style headings.

### 0001 — Postgres is the source of truth for chart text

**Context.** Chart text existed only in an untracked local folder (16 songs,
196KB; the full corpus is ~1500 songs, roughly 18MB of text). The alternative was
a git repository as the authority, with the database as a rebuildable serving
layer.

**Decision.** Postgres is the record of truth. A private repository holds the
seed input and the backups, never the authority.

**Why.** Not scale — 18MB of text is trivial for git. The load-bearing reason is
phase 2: contributions write through `charts` / `chart_revisions` with a
moderation queue and revision history, and git cannot be that write path.

**Consequences.** Backups become mandatory rather than optional (see 0004);
`seed_songs` must fail loudly on unmatched titles instead of skipping silently;
`ingest`'s `TRUNCATE` needs guarding.

### 0002 — Public hosted service, chart data only, no audio hosting

**Context.** Three product shapes were considered: a public hosted service, a
shipped app with contributions as pull requests against a data repository, and a
personal tool. Audio is SEGA-owned; charts are community transcriptions of
official charts — cleaner, though not risk-free.

**Decision.** Public hosted service. Chart text only. No audio crosses the server
in v1. Contributor-supplied audio is deferred to phase 2 and gets its own decision.

**Rejected.** Hosting audio. The cost is not storage — it is moderation. A
"community chart" is the obvious route for laundering official audio, and the
only defence is a human reviewing every upload.

**Consequences.** The engine's silent path (`load_chart`, wall-clock driven) is
v1's playback mode; the audio-slaved path (`load_song` → `sync_to_audio_position`)
is unused. Contract §5's audio upload and S3 presigning are struck. No object
storage enters the stack. No accounts and no PII, so no privacy policy is needed —
a property to preserve when adding client error reporting.

### 0003 — Web + PWA; Tauri dropped entirely

**Context.** The frontend shipped as a Tauri desktop app whose Rust side proxied
catalog fetches to dodge CORS. The user's targets are Android and web, not
desktop, and called adopting Tauri a mistake. `tauri android init` had never been
run — `gen/` held only `schemas`.

**Decision.** One web build, PWA-installable. `src-tauri/` deleted.

**Rejected.** Tauri Android. Its only genuine advantage was bundling the ~20MB
wasm into an APK, which a service worker largely recovers. Against that: a second
toolchain, a second release pipeline, signing key custody, Play Store review, and
scoped-storage complexity for any future local song packs.

**Consequences.** A single `fetch` path gains ETag, compression and IndexedDB
caching at once — the Rust proxy could not do any of them (it parsed 4.7MB into a
`serde_json::Value` and re-serialised it over IPC, with no gzip and no
`If-None-Match`). A CORS allowlist is needed until the custom domain lands. A
PWA's install identity is its origin, so moving from `*.pages.dev` later orphans
installed apps. **This decision reopens if the mobile WebView spike fails.**

### 0004 — Fly compute plus Neon Postgres, deliberately separated

**Context.** Free-tier hosting, with learning CI/testing/deployment as an
explicit goal.

**Decision.** Axum in Docker on Fly.io with scale-to-zero; Neon Postgres as a
separate service.

**Rejected.** Railway and Render (compute and data share a fate, and Render's free
Postgres has historically expired on a timer — disqualifying for irreplaceable
data); an Oracle Cloud always-free VM (the project's time would go to TLS renewal
and Postgres upgrades); Shuttle (restructures `main.rs` around its runtime macros
and teaches only Shuttle, where a Dockerfile transfers anywhere).

**Consequences.** A bad deploy, a suspended service or a blown free tier cannot
reach the data. Scale-to-zero means cold starts, which the client cache and edge
caching later mitigate. Migrations run as a Fly `release_command` so a bad
migration aborts the deploy instead of crash-looping a new machine. Backups are
Neon's restore window plus scheduled `pg_dump`. Fly has no hard spend cap, so
billing alerts are required.

### 0005 — Restructure the server in place rather than rewrite

**Context.** The user judged `apps/server` flawed and poorly structured, and
proposed rebuilding it. Inspection confirmed real problems: `main.rs` is 614 lines
mixing bootstrap, routing, domain enums, five query-param DTOs, error helpers,
eight handlers and tests; errors are ad-hoc `(StatusCode, Json<Value>)` tuples
with no `IntoResponse` type; state is a bare `Pool<Postgres>`; configuration is
`env::var(...).unwrap()`; `queries.rs` is 866 flat lines.

**Decision.** Restructure in place, keeping tests green throughout.

**Why not rewrite.** Everything on that list is a move-code-around problem. What
is *not* on it — the schema, the SQL, the HTTP contract, the `sheet_expr`
cross-tier key, the sync-revision design — is sound and expensive to rebuild. A
rewrite would discard six passing `#[sqlx::test]` handler tests for the duration
and require retyping 62 `query_as!` macros by hand, which is exactly where a
dropped `COALESCE` or a lost `ORDER BY` enters.

**Consequences.** Identical end state, reached incrementally. The existing tests
are the safety net, so they must keep passing at every step.

### 0006 — Typed sqlx queries over JSON-assembly views

**Context.** The catalog was originally assembled as JSON inside Postgres views
(`v_song`, `v_sheet`, `v_sheet_obj`), so response shape was defined in SQL and
unchecked by the compiler.

**Decision.** Move assembly into Rust with `sqlx::query_as!`, making `types.rs`
the compiler-checked response-shape seam. Delivered by the shipped
`server-catalog-typed-queries` feature (8 tickets); views dropped in migration
`20260907080000_drop_json_views`.

**Consequences.** 62 compile-time macros now mean `cargo build` requires a
reachable database unless an offline query cache is committed — which is why
`.sqlx/` and `SQLX_OFFLINE` are prerequisites for containerising at all.

### 0007 — Client-side filtering over server-side search

**Context.** Server-side sheet search was designed and built across two features
(`server-side-sheet-search`, `frontend-server-side-search`), then reverted by
`cd30221` — "browse page back to client-side filtering, catalog is small enough".

**Decision.** The client holds the full catalog and filters locally.
`GET /sheets/search` is retained server-side but is not the browse path.

**Consequences.** This is load-bearing for caching decisions. Because the whole
catalog is in memory anyway, a client-side SQLite cache would buy no query
benefit and would only translate rows back into the JSON shape `preprocessData`
already wants. That is why the offline cache is an IndexedDB blob plus a
`/sync/manifest` revision probe, not a relational store.

### 0008 — Documentation structure: Diátaxis-lite plus `docs/work/`

**Context and decision.** This spec. Records the audience-split structure, the
single-roadmap rule, feature directories as the unit of work, and the retirement
of `docs/superpowers/plans/`.

---

ADRs carry the rationale; `work/production-v1/spec.md` is reduced to scope and
sequencing and links to them, so the reasoning lives in exactly one place.

## Keeping it current

1. **Doc-sync is a checklist item inside the ticket that changes behaviour**,
   never a trailing ticket of its own. This is the direct fix for the observed
   failure.
2. **CI link checker.** Every relative link and file reference in Markdown must
   resolve. Catches the exact failure class present today: CLAUDE.md → moved
   `apps/server/docs/*`, `domain.md` → absent `CONTEXT.md`, `issue-tracker.md` →
   absent `triage-labels.md`.
3. **Contract snapshot test** (production-v1 ticket 27) guards `reference/api-contract.md`
   at the code level: the server cannot silently drop a promised field.

Link checking catches dead references; the snapshot catches lying ones. Neither
catches prose that is merely out of date — that is what the ADR trail is for.

## Stale content to correct during migration

- **CLAUDE.md** — doc paths, the "no test suite" claim, the Tauri proxy
  paragraph, `pnpm tauri dev`, the `.scratch/` path
- **README** — architecture (Tauri desktop), roadmap (SQLite cache,
  contributions, GitHub login as next), prerequisites, upstream license
  compliance (production-v1 ticket 01)
- **`reference/api-contract.md`** — strike audio upload; mark §4 and §5 as phase
  2; document ETag/304 and `429`
- **`agents/domain.md`** — `docs/adr/` becomes real; `CONTEXT.md` remains absent,
  so soften that wording
- **`agents/issue-tracker.md`** — `.scratch/` → `docs/work/`, drop the
  `triage-labels.md` reference, adopt the new status vocabulary
- **production-v1 tickets 01, 26, 28** — `songs/` was deleted by the user on
  2026-09-09; these assumed the 16 `maidata.txt` were present. They need
  re-acquiring a chart corpus into the private data repo instead.

## Migration steps

Each step is one commit; `git mv` so history follows.

1. Create the tree; move `reference/` and `work/` files
2. Write `ROADMAP.md` and `architecture.md`
3. Seed ADRs 0001–0008
4. Rewrite README and CLAUDE.md as entry points that link out
5. Update `agents/*.md`; normalise ticket statuses
6. Delete `docs/superpowers/plans/`
7. Add the CI link checker

## Out of scope

- Writing `reference/simai-format.md`
- `guides/deploy.md` and `guides/runbook.md` — produced by production-v1 tickets
  13, 14 and 20
- `CONTEXT.md` and a domain glossary — the domain-modeling skill creates those
  lazily, by design
- Any documentation site generator

## Absorbed work

**production-v1 ticket 29 (`docs-sync`)** is absorbed by this feature. It is
marked `dropped` with a pointer here rather than left as a second plan for the
same work. (`superseded` is a *feature*-level status; tickets use `dropped`.)

`/quality-sync-docs` is a verification pass to run *after* migration, not the
migration itself.
