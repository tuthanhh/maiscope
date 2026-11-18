<script setup lang="ts">
import { reactive, computed, watch, ref } from "vue";
import { useDataStore } from "~/stores/data";
import useI18n from "~/composables/useI18n";
import useGameData from "~/composables/useGameData";
import useSheetDialog from "~/composables/useSheetDialog";
import useSelectedSheets from "~/composables/useSelectedSheets";
import LoadingStatus from "~/enums/LoadingStatus";
import {
    buildEmptyFilters,
    buildFilterOptions,
    filterSheets,
    groupThousands,
    pickItem,
} from "~/utils";
import type { Filters, FilterOption } from "~/types";
import MvButton from "~/components/ui/MvButton.vue";
import MvCard from "~/components/ui/MvCard.vue";
import MvChip from "~/components/ui/MvChip.vue";
import MvDrawer from "~/components/ui/MvDrawer.vue";
import MvEmpty from "~/components/ui/MvEmpty.vue";
import MvError from "~/components/ui/MvError.vue";
import MvField from "~/components/ui/MvField.vue";
import MvPager from "~/components/ui/MvPager.vue";
import MvSelect, { type SelectOption } from "~/components/ui/MvSelect.vue";
import MvTextInput from "~/components/ui/MvTextInput.vue";
import BrowseFilters, {
    type BrowseForm,
} from "~/components/browse/BrowseFilters.vue";

// design-direction "Browse" and its worked example. Search, difficulty chips
// and the level range sit on the surface; the drawer holds the rest. Every
// filter applies as it changes.
defineOptions({ name: "BrowseView" });

const dataStore = useDataStore();
const i18n = useI18n();
const { t } = i18n;
const { getTypeAbbr, getDifficultyName } = useGameData();
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
const drawerOpen = ref(false);
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
// `difficulties` also lists every UTAGE tag, one per UTAGE sheet. Those are
// reached through the UTAGE type chip, so the difficulty chips stay the ones
// standard and DX sheets use.
const difficultyChips = computed(() => {
    const used = new Set(
        data.value.sheets
            .filter((sheet) => sheet.type !== "utage")
            .map((sheet) => sheet.difficulty),
    );
    return data.value.difficulties.filter((d) => used.has(d.code));
});
const difficultyColors = computed(
    () => new Map(data.value.difficulties.map((d) => [d.code, d.color ?? null])),
);

function toggleDifficulty(code: string): void {
    const list = form.difficulties;
    form.difficulties = list.includes(code)
        ? list.filter((v) => v !== code)
        : [...list, code];
}

// ── build Filters from UI state, then run the engine ──────────
// Typed BPM text that is not a number filters nothing, rather than everything
// (`bpm >= NaN` is false for every song).
function toNumber(text: string): number | null {
    const n = Number(text);
    return text.trim() !== "" && Number.isFinite(n) ? n : null;
}

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
    f.minBPM = toNumber(form.bpmMin);
    f.maxBPM = toNumber(form.bpmMax);
    f.region = form.region || null;
    f.useRegionOverride = form.useRegionOverride;
    return f;
});

// One Set of selected sheets, shared by the My-List filter and the cards.
const selectedSet = computed(() => new Set(selectedSheets.value.map((s) => s.key)));

const results = computed(() => {
    const sheets = filterSheets(data.value.sheets, filters.value);
    if (!myListOnly.value) return sheets;
    return sheets.filter((sheet) => selectedSet.value.has(sheet.key));
});

const bookmarkCount = computed(() => selectedSheets.value.length);

// ── pagination ────────────────────────────────────────────────
const PAGE_SIZE = 24; // divides by the 2, 3 and 4 grid columns
const currentPage = ref(1);
const top = ref<HTMLElement | null>(null);

// A filter change starts over at the first page.
watch(filters, () => {
    currentPage.value = 1;
});
watch(myListOnly, () => {
    currentPage.value = 1;
});
// Removing the last bookmark of the last page leaves the page past the end.
const pageCount = computed(() => Math.max(1, Math.ceil(results.value.length / PAGE_SIZE)));
watch(pageCount, (count) => {
    if (currentPage.value > count) currentPage.value = count;
});

const paginatedResults = computed(() => {
    const start = (currentPage.value - 1) * PAGE_SIZE;
    return results.value.slice(start, start + PAGE_SIZE);
});

function goToPage(page: number): void {
    currentPage.value = page;
    top.value?.scrollIntoView({ block: "start" });
}

// ── actions ───────────────────────────────────────────────────
function drawRandom(): void {
    if (results.value.length === 0) return;
    viewSheet(pickItem(results.value));
}
// Reset clears the drawer's filters; search, difficulties and the level range
// are on the surface and cleared there.
function reset(): void {
    const { title, difficulties, levelMin, levelMax } = form;
    Object.assign(form, emptyForm(), { title, difficulties, levelMin, levelMax });
}
function retry(): void {
    dataStore.loadData();
}
</script>

<template>
    <div ref="top" class="mv-browse">
        <MvError
            v-if="dataStore.currentLoadingStatus === LoadingStatus.ERROR"
            :label="t('page.songs.loadFailed') as string"
            :message="dataStore.currentLoadingErrorMessage"
            :retry-label="t('ui.retry') as string"
            @retry="retry"
        />

        <template v-else>
            <div class="mv-browse-search">
                <MvTextInput
                    v-model="form.title"
                    class="mv-browse-search-input"
                    icon="search"
                    :label="t('ui.search') as string"
                    :placeholder="t('page.songs.searchPlaceholder') as string"
                />
                <MvButton
                    icon="sliders"
                    aria-haspopup="dialog"
                    :aria-expanded="drawerOpen"
                    @click="drawerOpen = true"
                >
                    {{ t("page.songs.filters") }}
                </MvButton>
            </div>

            <div
                class="mv-browse-difficulties"
                role="group"
                :aria-label="t('term.difficulty') as string"
            >
                <MvChip
                    v-for="d in difficultyChips"
                    :key="d.code"
                    :diff="d.color ?? null"
                    :active="form.difficulties.includes(d.code)"
                    @click="toggleDifficulty(d.code)"
                >
                    {{ d.name }}
                </MvChip>
            </div>

            <div class="mv-browse-line">
                <MvField :label="t('term.level') as string" group class="mv-browse-level">
                    <span class="mv-browse-range">
                        <MvSelect
                            v-model="form.levelMin"
                            :options="levelOptions"
                            :label="t('page.songs.min') as string"
                            :placeholder="t('page.songs.min') as string"
                            :width="96"
                        />
                        <span class="t-muted" aria-hidden="true">–</span>
                        <MvSelect
                            v-model="form.levelMax"
                            :options="levelOptions"
                            :label="t('page.songs.max') as string"
                            :placeholder="t('page.songs.max') as string"
                            :width="96"
                        />
                    </span>
                </MvField>
                <div class="mv-browse-result">
                    <span class="t-label" aria-live="polite">
                        {{ groupThousands(results.length) }} {{ t("term.sheets") }}
                    </span>
                    <MvButton
                        kind="text"
                        icon="bookmark"
                        :filled="myListOnly"
                        :aria-pressed="myListOnly"
                        @click="myListOnly = !myListOnly"
                    >
                        {{ t("page.songs.myList") }} · {{ bookmarkCount }}
                    </MvButton>
                    <MvButton
                        kind="text"
                        icon="shuffle"
                        :disabled="results.length === 0"
                        @click="drawRandom"
                    >
                        {{ t("page.songs.drawRandom") }}
                    </MvButton>
                </div>
            </div>

            <MvEmpty v-if="results.length === 0">
                {{
                    myListOnly && bookmarkCount === 0
                        ? t("description.myListEmpty")
                        : t("description.filterResultEmpty")
                }}
            </MvEmpty>
            <ul v-else class="mv-browse-grid">
                <li v-for="s in paginatedResults" :key="s.key">
                    <MvCard
                        :cover="s.song"
                        :title="s.song.title"
                        :artist="s.song.artist"
                        :level="s.level"
                        :diff="difficultyColors.get(s.difficulty) ?? null"
                        :bookmarked="selectedSet.has(s.key)"
                        :bookmark-label="t('page.songs.bookmark') as string"
                        :label="`${s.song.title} — ${getTypeAbbr(s.type)} ${getDifficultyName(s.difficulty)} ${s.level ?? ''}`"
                        @open="viewSheet(s)"
                        @bookmark="toggleSheetSelection(s)"
                    />
                </li>
            </ul>

            <MvPager
                class="mv-browse-pager"
                :page="currentPage"
                :page-size="PAGE_SIZE"
                :total="results.length"
                :label="t('page.songs.pages') as string"
                :prev-label="t('page.songs.previousPage') as string"
                :next-label="t('page.songs.nextPage') as string"
                :range-text="(from, to, total) => t('page.songs.range', { from, to, total }) as string"
                @page="goToPage"
            />
        </template>

        <MvDrawer
            v-model="drawerOpen"
            :title="t('page.songs.filters') as string"
            :close-label="t('ui.close') as string"
        >
            <template #actions>
                <MvButton kind="text" @click="reset">{{ t("page.songs.reset") }}</MvButton>
            </template>
            <BrowseFilters
                :form="form"
                :category-options="categoryOptions"
                :version-options="versionOptions"
                :designer-options="designerOptions"
                :region-options="regionOptions"
                :type-chips="typeChips"
            />
        </MvDrawer>
    </div>
</template>

<style scoped lang="scss">
@use "~/assets/styles/breakpoints" as bp;

.mv-browse {
    max-width: var(--max-w);
    margin: 0 auto;
    padding: var(--s-6);
    scroll-margin-top: var(--nav-h);
}
.mv-browse-search {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s-3);
    margin-bottom: var(--s-4);
}
.mv-browse-search-input {
    width: 360px;
}
.mv-browse-difficulties {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s-2);
    margin-bottom: var(--s-4);
}
.mv-browse-line {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    justify-content: space-between;
    gap: var(--s-3) var(--s-6);
    margin-bottom: var(--s-6);
}
.mv-browse-level {
    margin-bottom: 0;
}
.mv-browse-range {
    display: flex;
    align-items: center;
    gap: var(--s-2);
}
.mv-browse-result {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--s-1) var(--s-4);
}
.mv-browse-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--card-gap);
    margin: 0;
    padding: 0;
    list-style: none;
}
.mv-browse-pager {
    margin-top: var(--s-8);
}

@media (min-width: bp.$bp-md) {
    .mv-browse-grid {
        grid-template-columns: repeat(3, minmax(0, 1fr));
    }
}
@media (min-width: bp.$bp-lg) {
    .mv-browse-grid {
        grid-template-columns: repeat(4, minmax(0, 1fr));
    }
}
@media (max-width: #{bp.$bp-md - 1px}) {
    .mv-browse {
        padding: var(--s-4);
        scroll-margin-top: 44px;
    }
    .mv-browse-search-input {
        flex: 1;
        width: auto;
    }
}
</style>
