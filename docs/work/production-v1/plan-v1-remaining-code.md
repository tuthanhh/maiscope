# Plan — v1.0 remaining code work

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the v1.0 work that is actually code, across three child features, and make the tracker tell the truth about the rest.

**Parent:** [`production-v1`](spec.md) — the umbrella these three children hang from. A cross-child execution plan lives here because no single child owns it.

**Scope boundary — read this before proposing extra work.** Everything in this plan is local: source files, workflow YAML already committed, and markdown. Nothing here provisions infrastructure, creates a repository, mints a secret, or connects to a production database. Items requiring those stay open and belong to the human partner:

- `prod-data-and-infra` 01 (Neon project, two roles, pooled endpoint)
- `prod-data-and-infra` 04 setup (private chart-text repo, `CHART_DATA_REPO`, `CHART_DATA_TOKEN`, `SEED_DATABASE_URL`)
- `prod-data-and-infra` 05 rehearsal (restore into a real Neon branch, measure the free-tier restore window)
- `build-and-deploy` 03/04/05 verifications (broken-migration abort, real-PR check run, Fly auto-deploy off, rollback exercised)
- `web-delivery` 02 stage 1 devtools confirmation
- `web-delivery` 04/05 (PWA, mobile gate) — deferred by the human partner this session

## Global Constraints

- **Never connect to production.** CLAUDE.md's first rule. `apps/server/.env` currently holds a **Neon production** `DATABASE_URL`; **do not `source` that file** and do not read a database URL from it. Every command in this plan that needs a database uses this URL verbatim:

  ```sh
  postgres://postgres:postgres@localhost:5432/maiscope_testbase
  ```

  That is the local `maiscope-db` container's lean, fully-migrated, zero-row database (16 tables, 8MB). `#[sqlx::test]` derives an ephemeral database per test from `DATABASE_URL` and drops it afterwards, so a data-laden or remote URL is both slow and destructive — see CLAUDE.md "Testing".

- **`queries/catalog.rs` uses the `sqlx::query_scalar!` macro**, backed by the committed `.sqlx/` cache. `.cargo/config.toml` sets `SQLX_OFFLINE=true` workspace-wide. Regenerating the cache is:

  ```sh
  DATABASE_URL=postgres://postgres:postgres@localhost:5432/maiscope_testbase \
    cargo sqlx prepare --workspace -- -p server --all-targets
  ```

  Every flag matters — see CLAUDE.md "Offline sqlx cache". CI runs the same with `--check` and fails on a stale cache.

- **Doc sync is part of the ticket that changes behaviour**, not a trailing ticket (CLAUDE.md). An endpoint change updates `docs/reference/api-contract.md` in the same commit.

- **No lint or format config exists for the frontend.** There is no `pnpm lint`. Do not add one, and do not reformat files you are editing beyond the lines the task names.

- **Ticket status vocabulary is a closed enum** (`docs/agents/issue-tracker.md:22`): tickets are `todo` → `in-progress` → `done` → `dropped`; features are `planned` → `active` → `shipped` → `superseded`. The enum token comes first on the `**Status:**` line; nuance goes in prose after an em dash. Never invent a new token.

- Rust tasks end green:
  ```sh
  cargo fmt --check
  cargo clippy -p server --all-targets -- -D warnings
  DATABASE_URL=postgres://postgres:postgres@localhost:5432/maiscope_testbase cargo test -p server
  ```

- Frontend tasks end green: `cd apps/host && pnpm build` (which runs `vue-tsc --noEmit` first).

---

### Task 1: `/catalog` serves an empty catalog on an unseeded database

Closes `prod-data-and-infra` [issue 06](../prod-data-and-infra/issues/06-empty-catalog-500.md). Detailed rationale lives in [`plan-06-empty-catalog.md`](../prod-data-and-infra/plan-06-empty-catalog.md); this task supersedes that plan's Step 4 and Step 6 database commands, which source the production `.env`.

**Why it matters:** production is serving a 500 on `GET /api/v1/catalog` right now. A freshly migrated database has schema but no rows, and `fetch_one` against the `catalog_meta` singleton returns `RowNotFound`, which the handler maps to 500.

**Files:**
- Modify: `apps/server/src/queries/catalog.rs:52-58`
- Modify: `docs/reference/api-contract.md:59`
- Test: `apps/server/src/routes/catalog.rs` (existing `#[cfg(test)]` module)

**Interfaces:**
- Consumes: nothing.
- Produces: `fetch_update_time` keeps its signature — `async fn fetch_update_time(pool: &PgPool) -> Result<String, sqlx::Error>`. It now yields `"0000-00-00"` instead of erroring when `catalog_meta` is empty. Callers unchanged.

- [ ] **Step 1: Write the failing test**

Add to the `#[cfg(test)] mod tests` block in `apps/server/src/routes/catalog.rs`. **Read the existing tests in that file first and match their style** for building the router and issuing the request, rather than inventing a new one.

```rust
    #[sqlx::test]
    async fn catalog_on_an_unseeded_database_is_empty_not_an_error(pool: PgPool) -> sqlx::Result<()> {
        // No seeding at all: migrations have run, every table is empty. This is
        // the state every freshly provisioned environment starts in.
        let response = catalog(State(state_for(pool)), Query(CatalogQuery::default()))
            .await
            .expect("an unseeded catalog must not be an error");

        let body = serde_json::to_value(&response.0).unwrap();
        assert_eq!(body["updateTime"], "0000-00-00");
        assert_eq!(body["songs"].as_array().unwrap().len(), 0);
        assert_eq!(body["categories"].as_array().unwrap().len(), 0);

        Ok(())
    }
```

If the existing tests call the handler through an assembled `Router` with `tower::ServiceExt::oneshot` instead of calling `catalog(...)` directly, use that style and assert `response.status() == StatusCode::OK` plus the same body fields. The point of the test is the status and the sentinel, not the calling convention.

- [ ] **Step 2: Run the test and watch it fail**

```sh
DATABASE_URL=postgres://postgres:postgres@localhost:5432/maiscope_testbase \
  cargo test -p server catalog_on_an_unseeded
```

Expected: FAIL — the handler returns `AppError::Database` from `RowNotFound`, so the `.expect(...)` panics (or the status is 500 in the router style). **Record the actual failure output in your report.** A test that passes here is testing nothing and means the bug is not where the plan says it is — stop and report that rather than proceeding.

- [ ] **Step 3: Write the implementation**

In `apps/server/src/queries/catalog.rs`, replace `fetch_update_time`:

```rust
/// `catalog_meta` is a singleton written only by the catalog sync, so it is
/// empty between `migrate` and the first sync — the state every new environment
/// starts in. Report the same `0000-00-00` sentinel the frontend already uses for
/// an empty catalog (`apps/host/src/utils/data.ts:19`) rather than erroring: an
/// unseeded catalog is empty, not broken.
pub async fn fetch_update_time(pool: &PgPool) -> Result<String, sqlx::Error> {
    let update_time = sqlx::query_scalar!(
        r#"SELECT to_char(update_time, 'YYYY-MM-DD') AS "update_time!" FROM catalog_meta LIMIT 1"#
    )
    .fetch_optional(pool)
    .await?;

    Ok(update_time.unwrap_or_else(|| "0000-00-00".to_string()))
}
```

The `0000-00-00` sentinel is not an invention — it matches `apps/host/src/utils/data.ts:19`, which already produces it, and `apps/host/src/utils/filter.ts:80`, which already branches on it. Use it exactly.

- [ ] **Step 4: Verify the offline query cache**

The macro's SQL string is unchanged but its call shape moved from `fetch_one` to `fetch_optional`. Check, and regenerate only if the check fails:

```sh
export DATABASE_URL=postgres://postgres:postgres@localhost:5432/maiscope_testbase
cargo sqlx prepare --check --workspace -- -p server --all-targets \
  || cargo sqlx prepare --workspace -- -p server --all-targets
```

State in your report which branch ran. If `sqlx-cli` is not installed, report that rather than installing a different version — the pin matters.

- [ ] **Step 5: Update the contract**

`docs/reference/api-contract.md:59` — record the unseeded value:

```jsonc
  "updateTime": "YYYY-MM-DD"   // bumped whenever any canonical row changes;
                               // "0000-00-00" before the first catalog sync
```

- [ ] **Step 6: Run the full check**

```sh
cargo fmt --check
cargo clippy -p server --all-targets -- -D warnings
DATABASE_URL=postgres://postgres:postgres@localhost:5432/maiscope_testbase cargo test -p server
```

Expected: all three exit 0, including the new test and every pre-existing one. Paste the test summary line into your report.

- [ ] **Step 7: Commit**

```bash
git add apps/server/src/queries/catalog.rs apps/server/src/routes/catalog.rs docs/reference/api-contract.md .sqlx
git commit -m "fix(server): serve an empty catalog on an unseeded database

fetch_one on the catalog_meta singleton made GET /catalog return 500 for
every freshly migrated environment, including production right now.
fetch_optional with the 0000-00-00 sentinel the frontend already uses."
```

---

### Task 2: Remove the last three Tauri references from the frontend

Closes the sweep `web-delivery` [issue 01](../web-delivery/issues/01-delete-tauri.md) left behind. `src-tauri/` is gone, the `@tauri-apps` dependency is gone, and `stores/data.ts` has a single `fetch` path — but three files still describe the deleted architecture.

**Why it matters:** the `router.ts` comment is the dangerous one. It is the *only* stated reason hash history exists, and that reason is now false. A reader who checks it will conclude hash history is vestigial and remove it, breaking deep links on Cloudflare Pages — where hash history is precisely what makes an SPA fallback unnecessary (`web-delivery` [issue 03](../web-delivery/issues/03-cloudflare-pages.md)).

**Files:**
- Modify: `apps/host/.vscode/extensions.json`
- Modify: `apps/host/src/app/router.ts:37`
- Modify: `apps/host/src/pages/about.vue` (the "desktop port … wrapped in Tauri" paragraph, around lines 43-56)

**Interfaces:** none. No runtime behaviour changes in this task — only a comment, an editor recommendation, and user-facing prose.

- [ ] **Step 1: Drop the Tauri editor recommendation**

In `apps/host/.vscode/extensions.json`, remove the `"tauri-apps.tauri-vscode"` entry. Keep `"Vue.volar"` and `"rust-lang.rust-analyzer"` — `rust-analyzer` still earns its place: the repo is a Cargo workspace with `engine/` and `apps/server/`.

- [ ] **Step 2: Correct the router comment**

`apps/host/src/app/router.ts:37` currently reads:

```ts
// Hash history: robust for a packaged Tauri app served from file://-like origins.
```

Replace it with the reason that is true today. The replacement must say that hash history is what removes the need for an SPA fallback / rewrite rule on Cloudflare Pages, so that a future reader understands it is load-bearing rather than vestigial. Keep it to one or two lines and match the file's existing comment voice.

- [ ] **Step 3: Correct the about page**

In `apps/host/src/pages/about.vue`, the paragraph beginning "It is a desktop port of" claims two things that are now false: that maiscope is a desktop app, and that it is wrapped in Tauri. Rewrite it so it credits `zetaraku/arcade-songs` as the original web app that maiscope is a port of, and drop the Tauri sentence and its `<a href="https://tauri.app/">` link entirely.

Constraints on the rewrite:
- **Keep the `zetaraku/arcade-songs` link and the credit.** It is a license obligation, not a courtesy — see `repo-hygiene` [issue 01](../repo-hygiene/issues/01-ignore-songs-and-verify-upstream-license.md) and `LICENSE-THIRD-PARTY`.
- Match the surrounding markup: same `class="mv-about-p"`, same `target="_blank" rel="noopener"` on external links, same indentation style.
- This paragraph is hardcoded English, not `t()`-wrapped, unlike its neighbours. Leave it hardcoded — converting it to i18n is a separate decision and out of scope.
- **Do not touch item C3** ("GitHub login + contribution UI"). Issue 01 left it deliberately: whether v1 still advertises contributions is a product decision, not a Tauri removal.

- [ ] **Step 4: Confirm nothing else references Tauri**

```sh
grep -rniE 'tauri' apps/host --include='*.ts' --include='*.vue' --include='*.json' --include='*.html' \
  | grep -v node_modules | grep -v '/src/wasm/'
```

Expected: no output. `apps/host/src/wasm/` is generated by `scripts/build-wasm.sh` and gitignored — never edit it. If the grep returns a hit in a file this task does not name, report it rather than fixing it silently.

- [ ] **Step 5: Verify the build**

```sh
cd apps/host && pnpm build
```

Expected: exit 0. This runs `vue-tsc --noEmit` first, so a malformed template in `about.vue` fails here. If `src/wasm/` is missing the build fails with TS2307 — that means `scripts/build-wasm.sh` has not been run in this checkout; report it, do not run a fat-LTO Bevy build to work around a comment change.

- [ ] **Step 6: Commit**

```bash
git add apps/host/.vscode/extensions.json apps/host/src/app/router.ts apps/host/src/pages/about.vue
git commit -m "chore(host): drop the last three Tauri references

The router comment was the load-bearing one: it gave a deleted architecture
as the only reason for hash history, which is now what lets Cloudflare Pages
serve the SPA without a fallback rule."
```

---

### Task 3: Make the tracker match the code

The workflows for `prod-data-and-infra` 04 and 05 are committed and already satisfy every box in those tickets that is code. The unchecked boxes are stale, and three `**Status:**` lines use tokens outside the closed enum. `web-delivery` 01 and 03 are complete in fact but open on paper.

**Why it matters:** `grep '\*\*Status:\*\* todo'` is how the next session finds remaining work. Free-form status tokens and boxes that lie about shipped code both break that. This is the last task because Tasks 1 and 2 change what is true.

**Files:**
- Modify: `docs/work/prod-data-and-infra/issues/04-seed-workflows.md`
- Modify: `docs/work/prod-data-and-infra/issues/05-backups.md`
- Modify: `docs/work/web-delivery/issues/01-delete-tauri.md`
- Modify: `docs/work/web-delivery/issues/03-cloudflare-pages.md`
- Modify: `docs/work/prod-data-and-infra/issues/06-empty-catalog-500.md`
- Modify: `docs/work/web-delivery/spec.md`
- Modify: `docs/work/prod-data-and-infra/spec.md`
- Modify: `docs/work/repo-hygiene/spec.md`
- Modify: `docs/ROADMAP.md` (only the rows these tickets belong to)

**Interfaces:** consumes Task 2's commit — cite its short SHA where a box is ticked because of it. Task 1 produced no commit: its work was already on `master` as `a4fd0b5` (see Step 5).

**Verify before you tick.** Every box in this task is a claim about code that already exists. Read the file the claim is about and confirm it, then tick. Do not tick a box on the strength of this plan's say-so — this plan was itself written partly from stale ticket text, which is the exact failure being repaired here.

**Scope limit, deliberate:** only the five tickets and two parents above. Do **not** rewrite the whole `ROADMAP.md` table, do not touch `catalog-sync`, and do not write an ADR. Those are real and tracked separately; they are not this task.

- [ ] **Step 1: Tick what `seed-charts.yml` already delivers**

In `docs/work/prod-data-and-infra/issues/04-seed-workflows.md`, verify each claim against `.github/workflows/seed-charts.yml` before ticking — read the workflow, do not take this plan's word for it. These are true today:

- `seed-charts.yml` exists, `workflow_dispatch` only, seeds from the private data repo's `maidata.txt` tree
- takes a `pg_dump` first and uploads it as a run artifact
- fails the run on anything unmatched by default (`--allow-unmatched` is an opt-in dispatch input)
- run summary posts the counts — `apps/server/src/bin/seed_songs.rs:223` emits seeded / already-up-to-date / unmatched difficulties / unmatched titles / skipped on one line, and the workflow's `Summary` step greps it into `$GITHUB_STEP_SUMMARY`
- neither workflow can be triggered by a push or a merge

Leave **unticked**, and add a short parenthetical saying it needs human setup: the private repo, `CHART_DATA_TOKEN`, `SEED_DATABASE_URL`, and the lower-privilege-role box (which is blocked on issue 01 — the workflow falls back to `DATABASE_URL` today).

- [ ] **Step 2: Record `reload-catalog.yml` as dropped**

Same file. The `reload-catalog.yml` box cannot be ticked and will never be built: `bin/ingest` no longer exists (`apps/server/src/bin/` holds only `migrate`, `seed_chart`, `seed_songs`, `sync_catalog`), having been replaced by the differential `bin/sync_catalog`, which never truncates. Replace the box with a one-line note saying it is dropped and why, citing the header comment in `.github/workflows/seed-charts.yml` that already records this.

- [ ] **Step 3: Tick what `backup-database.yml` already delivers**

In `docs/work/prod-data-and-infra/issues/05-backups.md`, verify against `.github/workflows/backup-database.yml`, then tick: the scheduled compressed `pg_dump`, the decided-and-documented destination, the enforced retention policy (7 daily + 1 per ISO week for 8 weeks, in the `Prune old backups` step), pre-mutation dumps wired into issue 04's workflow, and the recorded dump size.

Leave **unticked**: the restore rehearsal into a real Neon branch (the ticket already notes a local rehearsal happened on 2026-09-13 — do not tick that box on the strength of it, the box says Neon), and the no-secrets check.

- [ ] **Step 4: Normalise the status lines to the enum**

`docs/agents/issue-tracker.md:22` defines the only legal tokens. Fix these, keeping the existing prose as an em-dash suffix:

| File | Now | Becomes |
|---|---|---|
| `prod-data-and-infra/issues/04-seed-workflows.md` | `in progress — …` | `in-progress — …` |
| `prod-data-and-infra/issues/05-backups.md` | `in progress — …` | `in-progress — …` |

Do not touch issues 02 (`obsolete`) and 03 (`descoped`) in that directory — they need a judgement about whether they are `dropped` or `done`, which is not this task's to make.

- [ ] **Step 5: Close the finished tickets**

- `prod-data-and-infra/issues/06-empty-catalog-500.md` → `**Status:** done — a4fd0b5`.

  **This ticket was already fixed before this plan was written**, which Task 1 discovered and verified: `apps/server/src/queries/catalog.rs:fetch_update_time` already uses `fetch_optional` with the `0000-00-00` sentinel, and the test already exists at `apps/server/src/routes/catalog.rs:312`. Commit `a4fd0b5` ("fix(server): serve an empty catalog on an unseeded database") is on `master`. There is no Task 1 SHA to cite — cite `a4fd0b5`.

  While updating the ticket, also record two facts its text does not have:
  - The same commit fixed the identical `fetch_one`-on-an-empty-singleton bug in `Freshness::load` (`apps/server/src/routes/caching.rs`), which `GET /sync/manifest` hits as well as `GET /catalog`.
  - Commit `62f27b6` ("fix(server): /sync/delta no longer 500s on an unseeded database") closed the sibling case on `/sync/delta`.

  Do not re-verify by running the server or the test suite; read the two source files.
- `web-delivery/issues/01-delete-tauri.md` → `**Status:** done — sweep completed, <Task 2 short SHA>`
- `web-delivery/issues/03-cloudflare-pages.md` → `**Status:** done`. Its three boxes are already ticked and `.github/workflows/deploy-web.yml` is committed. **Preserve the "do not promote installs until the custom domain is in place" warning** — it is the live constraint on `web-delivery` 04 and must survive the ticket closing.
- `web-delivery/spec.md` → `**Status:** active` (not `shipped`: issues 04 and 05 remain open and deferred).
- `prod-data-and-infra/spec.md` → `**Status:** active`
- `repo-hygiene/spec.md` → `**Status:** shipped` (it currently says `done`, which is the *ticket* enum, not the feature one)

A feature's `spec.md` status and its `ROADMAP.md` row must agree — Step 6 sets the rows, and these set the specs they mirror. Changing one without the other trades a stale row for a contradiction.

- [ ] **Step 6: Correct only the affected ROADMAP rows**

In `docs/ROADMAP.md`, the Features table (around lines 32-39). Change only these rows:

- `repo-hygiene` → `shipped` (its `spec.md` already says done; the enum word is `shipped`)
- `build-and-deploy` → `active` (its `spec.md` says active; 01/02 done, 03/04/05 pending human verification)
- `web-delivery` → `active`
- `prod-data-and-infra` → `active`

Leave `catalog-sync` and `test-foundation` exactly as they are. `catalog-sync`'s status is wrong too, but correcting it requires an ADR that is out of scope here — do not touch the row, and do not add a note about it.

- [ ] **Step 7: Verify the enum holds**

```sh
grep -rhoP '(?<=\*\*Status:\*\* )[a-z-]+' docs/work --include='*.md' | sort -u
```

Expected tokens only: `active`, `done`, `dropped`, `in-progress`, `planned`, `shipped`, `todo`, plus the two you were told not to touch (`obsolete`, `descoped`). Anything else is a regression you introduced — fix it.

- [ ] **Step 8: Commit**

```bash
git add docs/
git commit -m "docs(tracker): reconcile ticket state with the shipped code

The seed and backup workflows already satisfy every code box in
prod-data-and-infra 04/05; reload-catalog.yml is dropped with bin/ingest.
web-delivery 01 and 03 are complete. Status tokens back inside the enum."
```

---

### Task 4: Rewrite `apps/host/README.md` — it still documents a Tauri desktop app

Added after Tasks 1-3 were scoped. This is the largest remaining `web-delivery` [issue 01](../web-delivery/issues/01-delete-tauri.md) straggler. Task 2's sweep missed it because that task's verification grep was scoped to `*.ts`, `*.vue`, `*.json` and `*.html` — markdown was never searched. That was a defect in this plan, not in Task 2's execution.

**Why it matters:** this is the frontend's public README. It instructs a contributor to install the Tauri system prerequisites and run `pnpm tauri dev`, then `pnpm tauri build` to produce a desktop binary. None of those commands exist — `apps/host/package.json` defines exactly three scripts: `dev`, `build`, `preview`. Anyone following this file fails at step 4 of Installation. Closing issue 01 while it stands would make that closure false.

**Files:**
- Modify: `apps/host/README.md` (188 lines)

**Interfaces:** none. Documentation only; no code, no config, no build behaviour.

**Ground truth, verified against the working tree — trust this over anything the README currently says:**

`apps/host/package.json` declares the package name `maiscope` (not `maiscope-frontend`), `packageManager: pnpm@10.33.0`, and only these scripts:

```json
"dev": "vite", "build": "vue-tsc --noEmit && vite build", "preview": "vite preview"
```

Actual dependencies: `@vueuse/core`, `pinia`, `sleep-promise`, `vue`, `vue-i18n`, `vue-router`, `yaml`.
Actual devDependencies: `@modyfi/vite-plugin-yaml`, `@vitejs/plugin-vue`, `sass`, `typescript`, `vite`, `vue-tsc`.

Therefore the current "Built With" table is wrong in four of its eight rows: **`vuetify`, `echarts`, `vue-echarts` and `@tauri-apps/api` are no longer dependencies at all.** The repo was rebuilt on native Vue 3 with the Vuetify shim removed (CLAUDE.md, "What this is"). `src/data/sites.json`, cited in Usage, does not exist.

- [ ] **Step 1: Re-verify the ground truth yourself**

Read `apps/host/package.json` and confirm the lists above before writing anything. If they disagree with this plan, the file wins — say so in your report. Also run:

```sh
ls apps/host/src/data/sites.json 2>&1 || echo "absent, as expected"
```

- [ ] **Step 2: Rewrite the stale sections**

Rewrite `apps/host/README.md` so every factual claim matches the working tree. The sections that are wrong:

- **Header tagline** (line ~15) and **About The Project** (~50-52) — describe a desktop app built with Tauri. maiscope v1 is a browse-only **web app** deployed to Cloudflare Pages ([ADR-0003](../../adr/0003-web-pwa-drop-tauri.md)). Keep the `zetaraku/arcade-songs` credit and link — it is a **license obligation** (`LICENSE-THIRD-PARTY`), not a courtesy — but it is no longer a "desktop port"; it is a port.
- **Built With** (~65-81) — drop the Vuetify and Tauri badges and the four dead table rows; add the real dependencies. Keep the table's existing two-column `| Package | Role |` shape.
- **Prerequisites** (~89-92) — the Rust toolchain requirement stays, but it is **not** for Tauri: it is for `scripts/build-wasm.sh`, which builds the Bevy visualizer to wasm with `wasm-bindgen-cli` pinned to `=0.2.122`. Drop the Tauri prerequisites link.
- **Installation** (~96-115) — the clone URL points at a standalone `maiscope-frontend` repo that no longer exists; this is `apps/host/` inside the monorepo. Remove the `pnpm tauri dev` step.
- **Usage** (~123-132) — `src/data/sites.json` does not exist; data comes from the server API (`VITE_API_BASE_URL`, set at build time — see `.github/workflows/deploy-web.yml`). Remove the `pnpm tauri build` desktop-binary instructions.
- **Roadmap** (~140-147) — every item is stale. C1 (backend read API) shipped. C2's `tauri-plugin-sql` SQLite cache was replaced by ETag caching now, IndexedDB later (`web-delivery` [issue 02](../web-delivery/issues/02-client-catalog-freshness.md)). C3 (GitHub login + contribution UI) was **cut** by [ADR-0002](../../adr/0002-chart-data-only-no-audio-hosting.md). C4's visualizer is built and wired. Replace the list with a pointer to [`docs/ROADMAP.md`](../../ROADMAP.md) rather than restating it — the roadmap is the single index and every other document links to it rather than duplicating it (`docs/ROADMAP.md:3-4`).
- **Any `github.com/tuthanhh/maiscope-frontend` link** — that repo is not this one. Point at the monorepo or remove the link.

**Preserve:** the badge/shield block at the top if the URLs still resolve to this repo, the table of contents structure, the License and Acknowledgments sections, and the "personal project, not open for contributions" note.

**Do not** invent features. The Features list (~55-60) describes the song gallery, filtering, per-sheet details, i18n and My List — verify each against `apps/host/src/` before keeping it, and drop any you cannot confirm. In particular the README claims "Data table, grid, and chart (echarts) views"; `echarts` is not a dependency.

- [ ] **Step 3: Verify no stale references survive**

```sh
grep -niE 'tauri|vuetify|echarts|desktop|maiscope-frontend|sites\.json' apps/host/README.md
```

Expected: no output. If a hit is a deliberate, accurate historical reference, keep it and justify it in your report.

- [ ] **Step 4: Confirm the documented commands actually work**

Every command the README tells a reader to run must exist. Check each against `apps/host/package.json`'s `scripts` block. Do not run `pnpm build` — Task 2 already proved it green and nothing here touches source.

- [ ] **Step 5: Commit**

```bash
git add apps/host/README.md
git commit -m "docs(host): README described a Tauri desktop app that no longer exists

Told readers to install Tauri prerequisites and run \`pnpm tauri dev\`,
neither of which exists; listed vuetify, echarts and @tauri-apps/api as
dependencies when none are; and pointed at a standalone frontend repo.
Roadmap now defers to docs/ROADMAP.md instead of restating it stale."
```
