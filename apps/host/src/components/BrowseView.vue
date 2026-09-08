<script setup lang="ts">
import { reactive, computed, watch, ref } from "vue";
import { useDataStore } from "~/stores/data";
import useI18n from "~/composables/useI18n";
import useSheetDialog from "~/composables/useSheetDialog";
import useSelectedSheets from "~/composables/useSelectedSheets";
import { buildEmptyFilters, buildFilterOptions, pickItem } from "~/utils";
import useSheetSearch from "~/composables/useSheetSearch";
import type { Filters, FilterOption, Sheet } from "~/types";
import { type SelectOption } from "~/components/ui/MvSelect.vue";
import BrowseSearchBar from "~/components/browse/BrowseSearchBar.vue";
import BrowseFilters, {
    type BrowseForm,
} from "~/components/browse/BrowseFilters.vue";
import BrowseResultBar from "~/components/browse/BrowseResultBar.vue";
import BrowseGrid from "~/components/browse/BrowseGrid.vue";
import BrowseList from "~/components/browse/BrowseList.vue";
import BrowsePagination from "~/components/browse/BrowsePagination.vue";

defineOptions({ name: "BrowseView" });

const dataStore = useDataStore();
const i18n = useI18n();
const { t } = i18n;
const { viewSheet } = useSheetDialog();
const { selectedSheets, toggleSheetSelection } = useSelectedSheets();

// ── filter form (declared, applied, and reset in one place) ───
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
        region: "",
        useRegionOverride: false,
        matchExactTitle: false,
        matchExactArtist: false,
        useInternalLevel: false,
    };
}
const form = reactive(emptyForm());

// view-only UI state
const gridView = ref(false);
const myListOnly = ref(false);

// ── data + options (reuses the existing filter engine) ────────
const data = computed(() => dataStore.currentData);
const options = computed(() => buildFilterOptions(data.value, i18n));

function toSelectOptions(raw: FilterOption<unknown>[] | null): SelectOption[] {
    return (raw ?? [])
        .filter((o) => o.$type === "option")
        .map((o) => ({ text: o.text, value: String(o.value) }));
}
const categoryOptions = computed(() =>
    toSelectOptions(options.value.categories),
);
const versionOptions = computed(() => toSelectOptions(options.value.versions));
const designerOptions = computed(() =>
    toSelectOptions(options.value.noteDesigners),
);
const levelOptions = computed(() => toSelectOptions(options.value.levels));
const regionOptions = computed(() => toSelectOptions(options.value.regions));

const typeChips = computed(() => data.value.types);
const difficultyChips = computed(() => data.value.difficulties);

const { results: searchResults, total: searchTotal, search } = useSheetSearch();

// ── build Filters from UI state, then run the engine ──────────
const filters = computed<Filters>(() => {
    const f = buildEmptyFilters();
    f.title = form.title || null;
    f.matchExactTitle = form.matchExactTitle;
    f.artist = form.artist || null;
    f.matchExactArtist = form.matchExactArtist;
    f.noteDesigners = form.designer ? [form.designer] : [];
    f.categories = form.category ? [form.category] : [];
    f.versions = form.version ? [form.version] : [];
    f.types = form.types;
    f.difficulties = form.difficulties;
    f.minLevelValue = form.levelMin ? Number(form.levelMin) : null;
    f.maxLevelValue = form.levelMax ? Number(form.levelMax) : null;
    f.useInternalLevel = form.useInternalLevel;
    f.minBPM = form.bpmMin ? Number(form.bpmMin) : null;
    f.maxBPM = form.bpmMax ? Number(form.bpmMax) : null;
    f.region = form.region || null;
    f.useRegionOverride = form.useRegionOverride;
    return f;
});

// One Set of selected sheets, shared by the My-List filter and per-row checks.
const selectedSet = computed(() => new Set(selectedSheets.value));

// myListOnly (bookmarks) has no server equivalent — it's a client-only
// concept (Pinia-stored selection), so it stays a post-filter client pass
// over whatever page the server just returned.
const results = computed(() => {
    if (!myListOnly.value) return searchResults.value;
    return searchResults.value.filter((sheet) => selectedSet.value.has(sheet));
});

const bookmarkCount = computed(() => selectedSheets.value.length);

// ── pagination ────────────────────────────────────────────────
const PAGE_SIZE = 22; // cards per page in both list and grid views — must match useSheetSearch's server call
const currentPage = ref(1);

// Reset to the first page and re-run the server search anytime filters change.
watch(filters, () => {
    currentPage.value = 1;
    search(filters.value, currentPage.value, PAGE_SIZE);
}, { immediate: true, deep: true });

watch(currentPage, () => {
    search(filters.value, currentPage.value, PAGE_SIZE);
});

const totalPages = computed(() => Math.ceil(searchTotal.value / PAGE_SIZE));
const paginatedResults = results; // already exactly one server-fetched page

function prevPage(): void {
    if (currentPage.value > 1) currentPage.value -= 1;
}
function nextPage(): void {
    if (currentPage.value < totalPages.value) currentPage.value += 1;
}

// ── actions ───────────────────────────────────────────────────
function onBookmark(sheet: Sheet, event: Event): void {
    event.stopPropagation();
    toggleSheetSelection(sheet);
}
// TODO(server-side-search): drawRandom now only picks from the current page
// (up to PAGE_SIZE sheets) instead of the full filtered set, because pagination
// moved server-side. Fixing this properly needs a dedicated random-pick
// endpoint; out of scope for the server-side search migration itself.
function drawRandom(): void {
    if (searchResults.value.length === 0) return;
    viewSheet(pickItem(searchResults.value));
}
function reset(): void {
    Object.assign(form, emptyForm());
}
</script>

<template>
    <div class="mv-root mv-browse">
        <BrowseSearchBar
            v-model="form.title"
            :placeholder="t('page.songs.searchPlaceholder')"
        />

        <BrowseFilters
            :form="form"
            :category-options="categoryOptions"
            :version-options="versionOptions"
            :designer-options="designerOptions"
            :level-options="levelOptions"
            :region-options="regionOptions"
            :type-chips="typeChips"
            :difficulty-chips="difficultyChips"
            @draw="drawRandom"
            @reset="reset"
        />

        <BrowseResultBar
            v-model:grid-view="gridView"
            v-model:my-list-only="myListOnly"
            :count="searchTotal"
            :bookmark-count="bookmarkCount"
        />

        <!-- results -->
        <div v-if="results.length === 0" class="mv-empty">
            {{
                myListOnly
                    ? t("description.myListEmpty")
                    : t("description.filterResultEmpty")
            }}
        </div>
        <BrowseGrid
            v-else-if="gridView"
            :sheets="paginatedResults"
            :selected-set="selectedSet"
            @select="viewSheet"
        />
        <BrowseList
            v-else
            :sheets="paginatedResults"
            :selected-set="selectedSet"
            @select="viewSheet"
            @bookmark="onBookmark"
        />

        <BrowsePagination
            v-if="totalPages > 1"
            :current-page="currentPage"
            :total-pages="totalPages"
            :page-size="PAGE_SIZE"
            :total="searchTotal"
            @prev="prevPage"
            @next="nextPage"
        />
    </div>
</template>

<style scoped>
.mv-browse {
    padding: 18px;
}
.mv-empty {
    padding: 32px;
    text-align: center;
    color: var(--mv-mut);
    font-size: 12px;
    border: 1px solid var(--mv-bd);
    border-radius: 4px;
}
</style>
