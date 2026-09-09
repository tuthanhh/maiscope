# Frontend Server-Side Search Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Split `apps/host/src/utils/data.ts:preprocessData`'s fused mutate+freeze into a testable "build" step and a separate "freeze" step (candidate 4 of the 2026-09-07 architecture review), remove the never-wired `superFilter` feature, and rewire the browse page (`BrowseView.vue`) to call the new `GET /api/v1/sheets/search` backend endpoint (see the companion plan `docs/superpowers/plans/2026-09-07-server-side-sheet-search.md`) instead of running `filterSheets` over the full local catalog — including adding UI for `region`/`useRegionOverride`/exact-match/`useInternalLevel`, which exist in the filter engine today but have no UI control.

**Architecture:** `data.ts` splits into `buildCatalog` (all current field-computation logic, minus every `Object.freeze` call) + `freezeCatalog` (a second pass that walks the same shape and freezes it) + `preprocessData` (now a 2-line orchestrator, unchanged public signature so `stores/data.ts` needs no changes for this part). A new `decorateSheetFields` helper is factored out of `buildCatalog`'s per-sheet computation (imageUrl/imageUrlM/sheetExpr/notePercents/`$canonicalSheet`) so the new search flow can apply the exact same client-derived-field logic to `GET /sheets/search`'s flat `Sheet[]` results without duplicating it. A new composable `useSheetSearch.ts` wraps the endpoint call (debounced via `@vueuse/core`'s `useDebounceFn`, already a dependency) and decorates results. `BrowseView.vue` swaps its `computed(() => filterSheets(...))` + client `PAGE_SIZE` slice for a call into `useSheetSearch` + the server's `total`/`page`.

**Tech Stack:** Vue 3, Pinia, TypeScript, `@vueuse/core` (already a dependency, provides `useDebounceFn`).

**Spec:** No separate spec document — encodes decisions reached via `/grilling` in-session. Key decisions:
- `preprocessData` build/freeze split: mutate-in-place build + separate freeze pass (not a fully pure rewrite) — the freeze is what blocks testing today, not the in-place mutation.
- `superFilter` removed entirely (never had UI wiring) — not deferred as "server can't support it," just cut. Revisit "when the app is strong enough."
- `region`/`useRegionOverride`/`matchExactTitle`/`matchExactArtist`/`useInternalLevel` get both backend support (companion plan) and new UI controls here, despite having no UI today.
- `GET /catalog` (full snapshot) stays as the only source for the Pinia store's `currentData` — `GET /sheets/search` results are a separate, ephemeral, decorated-but-not-frozen `Sheet[]` used only by the browse page's results list, not merged into the store.

## Global Constraints

- Depends on the companion backend plan (`docs/superpowers/plans/2026-09-07-server-side-sheet-search.md`) having landed — needs `GET /api/v1/sheets/search` to exist.
- Server returns raw fields only (`imageUrl`, `sheetExpr`, `notePercents`, `songNo` are never sent) — every consumer of search results must run them through `decorateSheetFields` before display, same as the full catalog.
- No test suite exists yet in `apps/host` (per CLAUDE.md) — this plan verifies via `pnpm build` (type-check) and manual browser testing (`pnpm dev`), not automated tests. If a test runner gets added later, `buildCatalog`/`decorateSheetFields` are exactly the functions that split was for — they're the ones to test first.
- Never touch production — local dev only.

---

### Task 1: Split `preprocessData` into `buildCatalog` + `freezeCatalog`

**Files:**
- Modify: `apps/host/src/utils/data.ts`

**Interfaces:**
- Consumes: `computeSheetExpr`, `validateNoteCounts` from `~/utils/sheet` (unchanged), `$canonicalSheet` symbol (unchanged).
- Produces (used by Task 3):
  - `pub` (exported) `function decorateSheetFields(sheet: Sheet, dataSourceUrl: string): void` — mutates a flat `Sheet` in place: `imageUrl`, `imageUrlM`, `sheetExpr`, `notePercents`, `sheet[$canonicalSheet] = sheet`. No `gameCode`/`validateNoteCounts` call (that warning is catalog-build-only, see Step 1 rationale).
  - `export function buildCatalog(data: Data, dataSourceUrl: string, gameCode: string): void` — everything `preprocessData` did except the `Object.freeze` calls.
  - `export function freezeCatalog(data: Data): void` — every `Object.freeze` call from the original, walking the same shape.
  - `export function preprocessData(data: Data, dataSourceUrl: string, gameCode: string): void` — unchanged signature, now just `buildCatalog(...)` then `freezeCatalog(...)`.

- [ ] **Step 1: Rewrite `data.ts`**

```typescript
import { $canonicalSheet, computeSheetExpr, validateNoteCounts } from '~/utils/sheet';
import type { Data, Sheet } from '~/types';

export function buildEmptyData(): Data {
  return {
    songs: [],
    sheets: [],
    categories: [],
    versions: [],
    types: [],
    difficulties: [],
    regions: [],
    updateTime: '0000-00-00',
  };
}

function resolveUrl(filePath: string | undefined, baseUrl: string) {
  return filePath != null ? new URL(filePath, baseUrl).toString() : filePath;
}

function computeNotePercentages(noteCounts: Record<string, number | null> | undefined) {
  return noteCounts != null ? Object.fromEntries(
    Object.entries(noteCounts)
      .map(([key, value]) => [
        key,
        value != null && noteCounts.total != null ? Number(value) / noteCounts.total : null,
      ]),
  ) : noteCounts;
}

// Per-sheet client-derived fields (contract: server sends raw fields only).
// Used both by buildCatalog (full nested catalog) and by the search flow
// (flat GET /sheets/search results) — the two places a raw server Sheet
// becomes a display-ready Sheet.
export function decorateSheetFields(sheet: Sheet, dataSourceUrl: string): void {
  sheet[$canonicalSheet] = sheet;
  sheet.imageUrl = resolveUrl(sheet.imageName, `${dataSourceUrl}/img/cover/`);
  sheet.imageUrlM = resolveUrl(sheet.imageName, `${dataSourceUrl}/img/cover-m/`);
  sheet.sheetExpr = computeSheetExpr(sheet);
  sheet.notePercents = computeNotePercentages(sheet.noteCounts);
}

// Build the full catalog shape: computed fields, prototype-linked sheets,
// final song/sheet ordering. Does NOT freeze — see freezeCatalog. Mutates
// `data` in place (and returns nothing) so existing callers that relied on
// preprocessData's in-place mutation keep working unchanged.
export function buildCatalog(data: Data, dataSourceUrl: string, gameCode: string): void {
  let lastSongNo = 0;
  for (const song of data.songs) {
    lastSongNo += 1;
    song.songNo = lastSongNo;
    song.imageUrl = resolveUrl(song.imageName, `${dataSourceUrl}/img/cover/`);
    song.imageUrlM = resolveUrl(song.imageName, `${dataSourceUrl}/img/cover-m/`);

    for (const sheet of song.sheets) {
      Object.setPrototypeOf(sheet, song);

      decorateSheetFields(sheet, dataSourceUrl);

      if (!validateNoteCounts(sheet, gameCode)) {
        // eslint-disable-next-line no-console
        console.warn('Invalid note counts:', sheet.sheetExpr, sheet.noteCounts);
      }

      for (const regionOverride of Object.values(sheet.regionOverrides ?? {})) {
        Object.setPrototypeOf(regionOverride, sheet);
      }
    }
  }

  data.songs.reverse();

  // eslint-disable-next-line no-param-reassign
  data.sheets = data.songs.flatMap((song) => song.sheets);

  for (const type of data.types) {
    type.iconUrl = resolveUrl(type.iconUrl, `${dataSourceUrl}/img/`);
  }
  for (const difficulty of data.difficulties) {
    difficulty.iconUrl = resolveUrl(difficulty.iconUrl!, `${dataSourceUrl}/img/`);
  }
}

// Freeze the shape buildCatalog produced. Separate pass so buildCatalog's
// output can be asserted on directly (e.g. in a future test) without hitting
// frozen-object write errors, and so a future new computed field can never
// silently land after its object's freeze call (there is no such call to
// land after — freezing only happens here, once, at the end).
export function freezeCatalog(data: Data): void {
  for (const song of data.songs) {
    for (const sheet of song.sheets) {
      for (const regionOverride of Object.values(sheet.regionOverrides ?? {})) {
        Object.freeze(regionOverride);
      }
      Object.freeze(sheet.noteCounts);
      Object.freeze(sheet.notePercents);
      Object.freeze(sheet.regions);
      Object.freeze(sheet.regionOverrides);
      Object.freeze(sheet);
    }
    Object.freeze(song.sheets);
    Object.freeze(song);
  }

  for (const category of data.categories) Object.freeze(category);
  for (const version of data.versions) Object.freeze(version);
  for (const type of data.types) Object.freeze(type);
  for (const difficulty of data.difficulties) Object.freeze(difficulty);
  for (const region of data.regions) Object.freeze(region);

  Object.freeze(data.songs);
  Object.freeze(data.sheets);
  Object.freeze(data.categories);
  Object.freeze(data.versions);
  Object.freeze(data.types);
  Object.freeze(data.difficulties);
  Object.freeze(data.regions);

  Object.freeze(data);
}

export function preprocessData(data: Data, dataSourceUrl: string, gameCode: string): void {
  buildCatalog(data, dataSourceUrl, gameCode);
  freezeCatalog(data);
}
```

Behavior-preservation note: the original interleaved `Object.freeze(sheet.noteCounts)` etc. calls happened per-sheet, inside the same loop as the field computation. `freezeCatalog` re-walks the identical tree shape afterward and freezes in the same relative order (noteCounts → notePercents → regions → regionOverrides → sheet → song.sheets → song, then the lookup arrays, then `data` itself) — the *order* of freeze calls doesn't affect behavior (freezing is not observable to later freezes), only that everything gets frozen exactly once, which this preserves.

- [ ] **Step 2: Type-check**

Run: `cd apps/host && pnpm build`
Expected: `vue-tsc --noEmit` passes (this also compiles `stores/data.ts`, unchanged, still calling `preprocessData` with the same signature).

- [ ] **Step 3: Manual smoke test**

```bash
cd apps/host && pnpm dev
```
Open the browse page in a browser; confirm songs/sheets render exactly as before (cover images, levels, sheetExpr-dependent features like the visualizer deep-link still work). This is a pure refactor — any visible difference is a bug.

- [ ] **Step 4: Commit**

```bash
git add apps/host/src/utils/data.ts
git commit -m "refactor(host): split preprocessData into buildCatalog + freezeCatalog"
```

---

### Task 2: Remove `superFilter`

**Files:**
- Modify: `apps/host/src/types/Filters.ts`
- Modify: `apps/host/src/utils/filter.ts`
- Modify: `apps/host/src/locales/en.yaml` (and the other 8 locale files — see Step 3)

**Interfaces:**
- Consumes: none.
- Produces: `Filters` type no longer has `superFilter`; `filter.ts` no longer exports `parseSuperFilter`.

- [ ] **Step 1: Drop `superFilter` from the `Filters` type**

```diff
   noteDesigners: string[];
   region: string | null;
   useRegionOverride: boolean | null;
-
-  superFilter: string | null;
 };
```

- [ ] **Step 2: Remove `parseSuperFilter` and the `superFilter` block from `filterSheets`**

```diff
-export function parseSuperFilter(superFilterText: string) {
-  // eslint-disable-next-line no-new-func
-  return new Function(superFilterText)();
-}
-
 export function filterSheets(sheets: Sheet[], filters: Filters) {
```

```diff
   if (predicates.length !== 0) {
     result = result.filter((sheet) => predicates.every((p) => p(sheet)));
   }

-  if (filters.superFilter != null) {
-    try {
-      const superFilter = parseSuperFilter(filters.superFilter);
-
-      if (typeof superFilter !== "function")
-        throw new TypeError("Invalid super filter");
-
-      try {
-        result = result.filter(superFilter);
-      } catch (err) {
-        // eslint-disable-next-line no-console
-        console.warn(err);
-      }
-    } catch {
-      // do nothing if the super filter is invalid
-    }
-  }
-
   result = result.map((sheet) => getCanonicalSheet(sheet));

   return result;
 }
```

Also check `buildEmptyFilters()` (near the top of `filter.ts`) for a `superFilter: null` initializer and remove it if present.

- [ ] **Step 3: Remove the locale key**

Run: `grep -rn "superFilter" apps/host/src/locales/`
For each hit (expected in `en.yaml` and the 8 other locale files), delete the `superFilter: ...` line under the `term:` section.

- [ ] **Step 4: Verify no remaining references**

Run: `grep -rn "superFilter\|parseSuperFilter" apps/host/src`
Expected: no output.

- [ ] **Step 5: Type-check**

Run: `cd apps/host && pnpm build`
Expected: passes (confirms nothing else referenced the removed field/function).

- [ ] **Step 6: Commit**

```bash
git add apps/host/src/types/Filters.ts apps/host/src/utils/filter.ts apps/host/src/locales/
git commit -m "refactor(host): remove unwired superFilter feature"
```

---

### Task 3: `useSheetSearch` composable + Tauri search command

**Files:**
- Create: `apps/host/src/composables/useSheetSearch.ts`
- Create: `apps/host/src-tauri/src/search.rs`
- Modify: `apps/host/src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: `decorateSheetFields` (Task 1), `GAME` (from `~/app/game`), `Filters` type (Task 2's trimmed version).
- Produces: `export function useSheetSearch(): { results: Ref<Sheet[]>, total: Ref<number>, loading: Ref<boolean>, search: (filters: Filters, page: number, pageSize: number) => void }` — `search` is debounced 300ms internally via `useDebounceFn`.

- [ ] **Step 1: Add the Tauri search command (mirrors the existing `load_chart_data` pattern exactly)**

```rust
// apps/host/src-tauri/src/search.rs
// Native sheet search fetch — same CORS-dodge pattern as data::load_chart_data.
#[tauri::command]
pub async fn search_sheets(search_url: String) -> Result<serde_json::Value, String> {
    match reqwest::get(&search_url).await {
        Ok(resp) if resp.status().is_success() => {
            let text = resp.text().await.map_err(|e| e.to_string())?;
            serde_json::from_str(&text).map_err(|e| e.to_string())
        }
        Ok(resp) => Err(format!("sheet search fetch failed: HTTP {}", resp.status())),
        Err(e) => Err(format!("sheet search fetch failed: {e}")),
    }
}
```

```diff
+mod search;
 mod data;
```
(in `apps/host/src-tauri/src/lib.rs`, next to the existing `mod data;`)

```diff
-        .invoke_handler(tauri::generate_handler![data::load_chart_data])
+        .invoke_handler(tauri::generate_handler![data::load_chart_data, search::search_sheets])
```

- [ ] **Step 2: Build the Tauri side to catch mistakes early**

Run: `cd apps/host/src-tauri && cargo build`
Expected: compiles.

- [ ] **Step 3: Write `useSheetSearch.ts`**

```typescript
import { ref } from "vue";
import { useDebounceFn } from "@vueuse/core";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { GAME } from "~/app/game";
import { decorateSheetFields } from "~/utils/data";
import type { Filters, Sheet } from "~/types";

type SearchResponse = { sheets: Sheet[]; total: number };

function buildSearchUrl(filters: Filters, page: number, pageSize: number): string {
  const params = new URLSearchParams();
  if (filters.title) params.set("title", filters.title);
  if (filters.matchExactTitle) params.set("matchExactTitle", "true");
  if (filters.artist) params.set("artist", filters.artist);
  if (filters.matchExactArtist) params.set("matchExactArtist", "true");
  for (const c of filters.categories) params.append("categories", c);
  for (const v of filters.versions) params.append("versions", v);
  for (const ty of filters.types) params.append("types", ty);
  for (const d of filters.difficulties) params.append("difficulties", d);
  if (typeof filters.minLevelValue === "number") params.set("minLevelValue", String(filters.minLevelValue));
  if (typeof filters.maxLevelValue === "number") params.set("maxLevelValue", String(filters.maxLevelValue));
  if (filters.useInternalLevel) params.set("useInternalLevel", "true");
  if (typeof filters.minBPM === "number") params.set("minBPM", String(filters.minBPM));
  if (typeof filters.maxBPM === "number") params.set("maxBPM", String(filters.maxBPM));
  for (const nd of filters.noteDesigners) params.append("noteDesigners", nd);
  if (filters.region) params.set("region", filters.region);
  if (filters.useRegionOverride) params.set("useRegionOverride", "true");
  params.set("page", String(page));
  params.set("pageSize", String(pageSize));
  return `${GAME.apiBaseUrl}/sheets/search?${params.toString()}`;
}

export default function useSheetSearch() {
  const results = ref<Sheet[]>([]);
  const total = ref(0);
  const loading = ref(false);

  async function runSearch(filters: Filters, page: number, pageSize: number) {
    loading.value = true;
    try {
      const url = buildSearchUrl(filters, page, pageSize);
      const data: SearchResponse = isTauri()
        ? await invoke<SearchResponse>("search_sheets", { searchUrl: url })
        : await (await fetch(url)).json();

      for (const sheet of data.sheets) {
        decorateSheetFields(sheet, GAME.dataSourceUrl);
      }
      results.value = data.sheets;
      total.value = data.total;
    } finally {
      loading.value = false;
    }
  }

  const search = useDebounceFn(runSearch, 300);

  return { results, total, loading, search };
}
```

- [ ] **Step 4: Type-check**

Run: `cd apps/host && pnpm build`
Expected: passes.

- [ ] **Step 5: Commit**

```bash
git add apps/host/src/composables/useSheetSearch.ts apps/host/src-tauri/src/search.rs apps/host/src-tauri/src/lib.rs
git commit -m "feat(host): add useSheetSearch composable + Tauri search command"
```

---

### Task 4: Add region/override/exact-match/internal-level UI to `BrowseFilters.vue`

**Files:**
- Modify: `apps/host/src/components/browse/BrowseFilters.vue`
- Modify: `apps/host/src/locales/en.yaml`

**Interfaces:**
- Consumes: nothing new from other tasks.
- Produces: `BrowseForm` type gains `region: string`, `useRegionOverride: boolean`, `matchExactTitle: boolean`, `matchExactArtist: boolean`, `useInternalLevel: boolean`; component gains prop `regionOptions: SelectOption[]`.

- [ ] **Step 1: Extend `BrowseForm` and props**

```diff
 export type BrowseForm = {
     title: string;
     artist: string;
     designer: string;
     category: string;
     version: string;
     types: string[];
     difficulties: string[];
     levelMin: string;
     levelMax: string;
     bpmMin: string;
     bpmMax: string;
+    region: string;
+    useRegionOverride: boolean;
+    matchExactTitle: boolean;
+    matchExactArtist: boolean;
+    useInternalLevel: boolean;
 };
```

```diff
 const props = defineProps<{
     form: BrowseForm;
     categoryOptions: SelectOption[];
     versionOptions: SelectOption[];
     designerOptions: SelectOption[];
     levelOptions: SelectOption[];
+    regionOptions: SelectOption[];
     typeChips: { type: string }[];
     difficultyChips: { difficulty: string; name?: string }[];
 }>();
```

- [ ] **Step 2: Add the region field and exact-match/internal-level checkboxes to the template**

```diff
                 <MvField :label="t('term.version')">
                     <MvSelect
                         v-model="props.form.version"
                         :options="versionOptions"
                         :placeholder="t('ui.all')"
                         :width="160"
                     />
                 </MvField>
+                <MvField :label="t('term.region')">
+                    <MvSelect
+                        v-model="props.form.region"
+                        :options="regionOptions"
+                        :placeholder="t('ui.all')"
+                        :width="140"
+                    />
+                    <label class="mv-check">
+                        <input type="checkbox" v-model="props.form.useRegionOverride" />
+                        {{ t("page.songs.useRegionOverride") }}
+                    </label>
+                </MvField>
             </div>
```

```diff
                 <MvField :label="t('term.artist')">
                     <MvTextInput
                         v-model="props.form.artist"
                         placeholder="—"
                         :width="150"
                     />
+                    <label class="mv-check">
+                        <input type="checkbox" v-model="props.form.matchExactArtist" />
+                        {{ t("page.songs.exactMatch") }}
+                    </label>
                 </MvField>
```

```diff
                 <MvField :label="t('term.level')">
                     <div class="mv-range">
                         <MvSelect
                             v-model="props.form.levelMin"
                             :options="levelOptions"
                             :placeholder="t('page.songs.min')"
                             :width="90"
                         />
                         <span class="mv-dash">–</span>
                         <MvSelect
                             v-model="props.form.levelMax"
                             :options="levelOptions"
                             :placeholder="t('page.songs.max')"
                             :width="90"
                         />
                     </div>
+                    <label class="mv-check">
+                        <input type="checkbox" v-model="props.form.useInternalLevel" />
+                        {{ t("page.songs.useInternalLevel") }}
+                    </label>
                 </MvField>
```

For the title search bar's exact-match toggle: `BrowseSearchBar.vue` currently only exposes a single `v-model="form.title"` text input with no room for a checkbox — add `matchExactTitle` here too, next to the title field in `BrowseFilters.vue`'s own advanced panel instead of touching `BrowseSearchBar.vue` (keeps the always-visible search bar simple, matches where `matchExactArtist` just landed):

```diff
             <div class="mv-fields">
+                <MvField :label="t('term.title')">
+                    <label class="mv-check">
+                        <input type="checkbox" v-model="props.form.matchExactTitle" />
+                        {{ t("page.songs.exactMatch") }}
+                    </label>
+                </MvField>
                 <MvField :label="t('term.artist')">
```

- [ ] **Step 3: Add a `.mv-check` style**

```diff
 .mv-dash {
     color: var(--mv-mut);
 }
+.mv-check {
+    display: flex;
+    align-items: center;
+    gap: 5px;
+    font-size: 11px;
+    color: var(--mv-mut);
+    margin-top: 6px;
+    cursor: pointer;
+}
```

- [ ] **Step 4: Add the new locale keys**

```diff
   songs:
     sheetData: Sheet Data
     advancedSearch: Advanced Search
     searchPlaceholder: title / artist…
     drawRandom: Draw Random
     reset: Reset
     myList: My List
     min: min
     max: max
     openInVisualizer: Open in Visualizer
+    exactMatch: Exact match
+    useInternalLevel: Use internal level
+    useRegionOverride: Use region override
```

(Add the same three keys to the other 8 locale files — `ja.yaml`, `ko.yaml`, `zh-Hans.yaml`, `zh-Hant.yaml`, `vi.yaml`, `es.yaml`, `id.yaml`, `ru.yaml`. English text is an acceptable placeholder for non-English locales until translated — this project has no i18n-completeness check today.)

- [ ] **Step 5: Type-check + manual visual check**

Run: `cd apps/host && pnpm build && pnpm dev`
Open the browse page, expand "Advanced Search," confirm the new Region select and three checkboxes render and toggle without console errors.

- [ ] **Step 6: Commit**

```bash
git add apps/host/src/components/browse/BrowseFilters.vue apps/host/src/locales/
git commit -m "feat(host): add region/override/exact-match/internal-level filter UI"
```

---

### Task 5: Rewire `BrowseView.vue` onto server-side search

**Files:**
- Modify: `apps/host/src/components/BrowseView.vue`

**Interfaces:**
- Consumes: `useSheetSearch` (Task 3); extended `BrowseForm`/`regionOptions` (Task 4); `buildFilterOptions` (unchanged, still derives dropdown option lists from the full local catalog — that stays client-side, it's just populating `<select>` choices, not filtering results).

- [ ] **Step 1: Extend `emptyForm()` and wire `regionOptions`**

```diff
 function emptyForm(): BrowseForm {
     return {
         title: "",
         artist: "",
         designer: "",
         category: "",
         version: "",
         types: [],
         difficulties: [],
         levelMin: "",
         levelMax: "",
         bpmMin: "",
         bpmMax: "",
+        region: "",
+        useRegionOverride: false,
+        matchExactTitle: false,
+        matchExactArtist: false,
+        useInternalLevel: false,
     };
 }
```

```diff
 const levelOptions = computed(() => toSelectOptions(options.value.levels));
+const regionOptions = computed(() => toSelectOptions(options.value.regions));
```

- [ ] **Step 2: Replace the local `filterSheets` computed + pagination with `useSheetSearch`**

```diff
-import {
-    buildEmptyFilters,
-    buildFilterOptions,
-    filterSheets,
-    pickItem,
-} from "~/utils";
+import { buildEmptyFilters, buildFilterOptions, pickItem } from "~/utils";
+import useSheetSearch from "~/composables/useSheetSearch";
```

```diff
+const { results: searchResults, total: searchTotal, search } = useSheetSearch();
+
 // ── build Filters from UI state, then run the engine ──────────
 const filters = computed<Filters>(() => {
     const f = buildEmptyFilters();
     f.title = form.title || null;
+    f.matchExactTitle = form.matchExactTitle;
     f.artist = form.artist || null;
+    f.matchExactArtist = form.matchExactArtist;
     f.noteDesigners = form.designer ? [form.designer] : [];
     f.categories = form.category ? [form.category] : [];
     f.versions = form.version ? [form.version] : [];
     f.types = form.types;
     f.difficulties = form.difficulties;
     f.minLevelValue = form.levelMin ? Number(form.levelMin) : null;
     f.maxLevelValue = form.levelMax ? Number(form.levelMax) : null;
+    f.useInternalLevel = form.useInternalLevel;
     f.minBPM = form.bpmMin ? Number(form.bpmMin) : null;
     f.maxBPM = form.bpmMax ? Number(form.bpmMax) : null;
+    f.region = form.region || null;
+    f.useRegionOverride = form.useRegionOverride;
     return f;
 });
```

```diff
 // One Set of selected sheets, shared by the My-List filter and per-row checks.
 const selectedSet = computed(() => new Set(selectedSheets.value));

-const results = computed(() => {
-    const sheets = filterSheets(data.value.sheets, filters.value);
-    if (!myListOnly.value) return sheets;
-    return sheets.filter((sheet) => selectedSet.value.has(sheet));
-});
+// myListOnly (bookmarks) has no server equivalent — it's a client-only
+// concept (Pinia-stored selection), so it stays a post-filter client pass
+// over whatever page the server just returned.
+const results = computed(() => {
+    if (!myListOnly.value) return searchResults.value;
+    return searchResults.value.filter((sheet) => selectedSet.value.has(sheet));
+});

 const bookmarkCount = computed(() => selectedSheets.value.length);

 // ── pagination ────────────────────────────────────────────────
-const PAGE_SIZE = 22; // cards per page in both list and grid views
+const PAGE_SIZE = 22; // cards per page in both list and grid views — must match useSheetSearch's server call
 const currentPage = ref(1);

-// Reset to the first page anytime the underlying results change
-watch(results, () => {
+// Reset to the first page and re-run the server search anytime filters change.
+watch(filters, () => {
     currentPage.value = 1;
-});
+    search(filters.value, currentPage.value, PAGE_SIZE);
+}, { immediate: true, deep: true });

-const totalPages = computed(() => Math.ceil(results.value.length / PAGE_SIZE));
+watch(currentPage, () => {
+    search(filters.value, currentPage.value, PAGE_SIZE);
+});

-const paginatedResults = computed(() => {
-    const start = (currentPage.value - 1) * PAGE_SIZE;
-    return results.value.slice(start, start + PAGE_SIZE);
-});
+const totalPages = computed(() => Math.ceil(searchTotal.value / PAGE_SIZE));
+const paginatedResults = results; // already exactly one server-fetched page
```

`myListOnly`'s client-side filter now runs on a single already-paginated page (up to 22 sheets) rather than the full catalog — a real, small behavior change: today, toggling "my list only" filters bookmarks out of the *entire* filtered set before paginating (so a bookmark on page 3 still shows on page 1 if enough earlier items are excluded); after this change, it only ever hides bookmarked-or-not items within the current page, so the visible count can be less than `PAGE_SIZE` and won't backfill from the next page. This is an accepted, documented trade-off of moving pagination server-side — flag it in the PR/commit body if it comes up in review, but it's not a bug to fix in this plan (would need `myListOnly` to also become a server-side filter, out of scope here since bookmarks are client-only Pinia state the server has no concept of).

- [ ] **Step 3: `drawRandom` needs the full matching set, not just the current page**

```diff
 function drawRandom(): void {
-    if (results.value.length === 0) return;
-    viewSheet(pickItem(results.value));
+    if (searchResults.value.length === 0) return;
+    viewSheet(pickItem(searchResults.value));
 }
```

This is unchanged in effect from before this task in the sense that it only ever picked from the *currently loaded* page even pre-refactor's `results` was the full filtered set, not a page — so this is actually a behavior change worth flagging too: `drawRandom` used to pick from every filtered result across all pages; now it can only pick from the current page's up-to-22 sheets, since that's all the client has loaded. Note this explicitly rather than silently accept it — see Step 4.

- [ ] **Step 4: Decide `drawRandom`'s scope — flag, don't silently fix**

This plan does not fix `drawRandom`'s reduced scope (whole-filtered-set → current-page-only) because doing so properly needs a dedicated "random sheet matching these filters" server endpoint (`ORDER BY random() LIMIT 1` server-side), which is new scope beyond "migrate filtering to the server." Leave a comment marking it:

```diff
+// TODO(server-side-search): drawRandom now only picks from the current page
+// (up to PAGE_SIZE sheets) instead of the full filtered set, because pagination
+// moved server-side. Fixing this properly needs a dedicated random-pick
+// endpoint; out of scope for the server-side search migration itself.
 function drawRandom(): void {
     if (searchResults.value.length === 0) return;
     viewSheet(pickItem(searchResults.value));
 }
```

- [ ] **Step 5: Pass `regionOptions` to `BrowseFilters`**

```diff
         <BrowseFilters
             :form="form"
             :category-options="categoryOptions"
             :version-options="versionOptions"
             :designer-options="designerOptions"
             :level-options="levelOptions"
+            :region-options="regionOptions"
             :type-chips="typeChips"
             :difficulty-chips="difficultyChips"
             @draw="drawRandom"
             @reset="reset"
         />
```

- [ ] **Step 6: `BrowseResultBar`'s `count` prop should reflect total matches, not just the current page**

```diff
         <BrowseResultBar
             v-model:grid-view="gridView"
             v-model:my-list-only="myListOnly"
-            :count="results.length"
+            :count="searchTotal"
             :bookmark-count="bookmarkCount"
         />
```

- [ ] **Step 7: `BrowsePagination`'s `total` prop**

```diff
         <BrowsePagination
             v-if="totalPages > 1"
             :current-page="currentPage"
             :total-pages="totalPages"
             :page-size="PAGE_SIZE"
-            :total="results.length"
+            :total="searchTotal"
             @prev="prevPage"
             @next="nextPage"
         />
```

- [ ] **Step 8: Type-check**

Run: `cd apps/host && pnpm build`
Expected: passes. If `watch` isn't already imported from `"vue"` at the top of the file, it already is (line 2 of the original file imports `watch`).

- [ ] **Step 9: Manual end-to-end test**

```bash
cd apps/host && pnpm dev
```
In the browser:
1. Open the browse page — confirm it loads a first page of results from the server (open devtools Network tab, confirm a request to `.../sheets/search?...page=1&pageSize=22`).
2. Type in the title search box — confirm a debounced request fires ~300ms after you stop typing, not on every keystroke.
3. Toggle a category/type/difficulty chip — confirm results update and page resets to 1.
4. Select the new Region dropdown + toggle "Use region override" — confirm a request includes `region=`/`useRegionOverride=true` params and results change accordingly (needs seed data with region overrides to see a visible effect; presence of the param in the network request is sufficient verification otherwise).
5. Paginate to page 2 — confirm a new request fires with `page=2`, and the page's results differ from page 1's.
6. Toggle "My List Only" with a bookmark that exists but is outside the current page — confirm it does NOT appear (documented trade-off from Step 2), rather than silently reappearing (which would mean the old full-set filtering logic leaked back in somewhere).

- [ ] **Step 10: Commit**

```bash
git add apps/host/src/components/BrowseView.vue
git commit -m "feat(host): rewire BrowseView onto server-side sheet search"
```

---

### Task 6: Delete now-dead `filterSheets` from `filter.ts`

**Files:**
- Modify: `apps/host/src/utils/filter.ts`

**Interfaces:** none — this is cleanup once nothing calls `filterSheets` anymore.

- [ ] **Step 1: Confirm nothing else calls it**

Run: `grep -rn "filterSheets" apps/host/src`
Expected: only the definition in `filter.ts` itself (Task 5 already removed `BrowseView.vue`'s only call site).

- [ ] **Step 2: Delete `filterSheets`**

Remove the entire `export function filterSheets(sheets: Sheet[], filters: Filters) { ... }` block (what's left of it after Task 2 already removed its `superFilter` sub-block). Keep `buildFilterOptions` and `buildEmptyFilters` — both are still used (`BrowseView.vue`'s dropdown options and initial form state).

- [ ] **Step 3: Check `getRegionOverrideSheet` for remaining callers**

Run: `grep -rn "getRegionOverrideSheet" apps/host/src`
If the only remaining reference is its own definition in `sheet.ts`, it's now dead too (it was only ever called from the just-deleted `filterSheets`) — delete it from `sheet.ts`. If anything else calls it, leave it.

- [ ] **Step 4: Type-check**

Run: `cd apps/host && pnpm build`
Expected: passes.

- [ ] **Step 5: Commit**

```bash
git add apps/host/src/utils/filter.ts apps/host/src/utils/sheet.ts
git commit -m "refactor(host): remove filterSheets, now dead after server-side search migration"
```

---

## Self-Review Notes

- **Spec coverage:** build/freeze split ✓ (Task 1); `superFilter` removed ✓ (Task 2); region/override/exact-match/internal-level UI ✓ (Task 4); server-side search wiring ✓ (Task 5); dead-code cleanup ✓ (Task 6).
- **Honest trade-offs surfaced, not hidden:** `myListOnly` (Task 5 Step 2) and `drawRandom` (Task 5 Step 3-4) both have real, documented behavior narrowing from full-set to current-page — flagged explicitly with a `TODO` rather than silently shipped or silently "fixed" with scope-creeping new server endpoints.
- **Type consistency:** `decorateSheetFields` (Task 1) is the exact function name imported in `useSheetSearch.ts` (Task 3); `BrowseForm`'s five new fields (Task 4) match exactly what `BrowseView.vue`'s `emptyForm()` and `filters` computed (Task 5) read.
- **No placeholders** — every diff is literal before/after text.
- **Dependency order**: Task 3 (composable) depends on Task 1 (`decorateSheetFields`) and the companion backend plan; Task 5 depends on Tasks 3-4; Task 6 depends on Task 5 having removed the only call site. Execute in the numbered order.

---

**Plan complete and saved to `docs/superpowers/plans/2026-09-07-frontend-server-side-search.md`.** Two execution options:

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

**Which approach?**
