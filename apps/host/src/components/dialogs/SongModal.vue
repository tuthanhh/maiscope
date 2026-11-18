<script setup lang="ts">
import { computed, ref, shallowRef, useId, watch } from "vue";
import { useRouter } from "vue-router";
import { useDataStore } from "~/stores/data";
import useI18n from "~/composables/useI18n";
import useGameData from "~/composables/useGameData";
import useSelectedSheets from "~/composables/useSelectedSheets";
import useSongModal from "~/composables/useSongModal";
import { GAME } from "~/app/game";
import { ApiError, fetchJson } from "~/utils/api";
import { attachSong, formatNoteCounts, formatReleaseDate } from "~/utils";
import type { Sheet, Song } from "~/types";
import MvButton from "~/components/ui/MvButton.vue";
import MvChip from "~/components/ui/MvChip.vue";
import MvCover from "~/components/ui/MvCover.vue";
import MvError from "~/components/ui/MvError.vue";
import MvLevelLine from "~/components/ui/MvLevelLine.vue";
import MvModal from "~/components/ui/MvModal.vue";

// design-direction "Song modal": information first; the difficulty chips in
// the underline form; bookmark and play as text buttons of equal weight. Open
// while the URL names a song (`useSongModal`).
defineOptions({ name: "SongModal" });

const { t } = useI18n();
const router = useRouter();
const dataStore = useDataStore();
const { getTypeAbbr, getTypeName, getDifficultyName, getDifficultyColor } = useGameData();
const { isBookmarked: hasBookmark, toggleSheetSelection } = useSelectedSheets();
const modal = useSongModal();
const titleId = useId();
const noChartId = useId();

// ── the song: from the Catalog when it is loaded, else fetched alone ──
// A shared link must not wait on the snapshot (screen-data-requirements).
const catalogSong = computed(() => {
    const id = modal.songId.value;
    return id ? (dataStore.currentData.songs.find((s) => s.id === id) ?? null) : null;
});
const fetched = shallowRef<Song | null>(null);
const failure = ref<"not-found" | "error" | null>(null);
let request = 0;

async function fetchSong(id: string): Promise<void> {
    const mine = ++request;
    failure.value = null;
    try {
        const song = await fetchJson<Song>(
            `${GAME.apiBaseUrl}/songs/${encodeURIComponent(id)}`,
        );
        if (mine !== request) return;
        fetched.value = attachSong(song, GAME.dataSourceUrl);
    } catch (err) {
        if (mine !== request) return;
        // 400 is an id that is not a UUID: no song has that link either.
        const status = err instanceof ApiError ? err.status : 0;
        failure.value = status === 404 || status === 400 ? "not-found" : "error";
    }
}

watch(
    [modal.songId, catalogSong],
    ([id, inCatalog]) => {
        if (id === null || inCatalog !== null) {
            request++;
            failure.value = null;
            return;
        }
        if (fetched.value?.id !== id) fetchSong(id);
    },
    { immediate: true },
);

const song = computed<Song | null>(() => {
    if (catalogSong.value) return catalogSong.value;
    return fetched.value?.id === modal.songId.value ? fetched.value : null;
});

// ── the selected sheet: the URL's, else the song's first ──
const selected = computed<Sheet | null>(() => {
    const s = song.value;
    if (!s) return null;
    return (
        s.sheets.find(
            (x) => x.type === modal.type.value && x.difficulty === modal.difficulty.value,
        ) ??
        s.sheets[0] ??
        null
    );
});
// A link naming no sheet of the song shows the first one; the URL is
// corrected to it, so the link always names what is on screen.
watch(
    [selected, modal.type, modal.difficulty],
    ([sheet, type, difficulty]) => {
        if (sheet && (sheet.type !== type || sheet.difficulty !== difficulty)) {
            modal.select(sheet);
        }
    },
    { immediate: true },
);

const types = computed(() => [...new Set(song.value?.sheets.map((s) => s.type) ?? [])]);
const sameType = computed(
    () => song.value?.sheets.filter((s) => s.type === selected.value?.type) ?? [],
);

function selectType(type: string): void {
    const sheets = song.value?.sheets.filter((s) => s.type === type) ?? [];
    const match =
        sheets.find((s) => s.difficulty === selected.value?.difficulty) ?? sheets[0];
    if (match) modal.select(match);
}


// ── the selected sheet's figures ──
const releaseDate = computed(() => formatReleaseDate(song.value?.releaseDate));
const noteCounts = computed(() =>
    formatNoteCounts(selected.value?.noteCounts, t("term.totalNotes") as string),
);

const isBookmarked = computed(() => !!selected.value && hasBookmark(selected.value.key));
function toggleBookmark(): void {
    if (selected.value) toggleSheetSelection(selected.value);
}
function play(): void {
    const sheet = selected.value;
    if (!sheet) return;
    router.push({
        name: "visualizer",
        query: { song: sheet.song.id, type: sheet.type, difficulty: sheet.difficulty },
    });
}
function retry(): void {
    if (modal.songId.value) fetchSong(modal.songId.value);
}
</script>

<template>
    <MvModal
        :open="modal.songId.value !== null"
        :labelledby="song ? titleId : undefined"
        :label="t('page.songs.song') as string"
        @close="modal.close"
    >
        <header class="sm-head">
            <div class="sm-jacket">
                <MvCover v-if="song" :item="song" />
            </div>
            <MvButton
                kind="icon"
                icon="x"
                :label="t('ui.close') as string"
                @click="modal.close"
            />
        </header>

        <div class="sm-body">
            <MvError
                v-if="failure === 'not-found'"
                :label="t('page.songs.notFound') as string"
                :message="t('page.songs.notFoundMessage') as string"
            />
            <MvError
                v-else-if="failure === 'error'"
                :label="t('page.songs.songLoadFailed') as string"
                :message="t('page.songs.songLoadFailedMessage') as string"
                :retry-label="t('ui.retry') as string"
                @retry="retry"
            />
            <p v-else-if="!song" class="t-body t-muted sm-loading">
                {{ t("description.loading") }}
            </p>
            <p v-else-if="!selected" class="t-body t-muted sm-loading">
                {{ t("page.songs.noSheets") }}
            </p>

            <template v-else>
                <div class="sm-top t-label t-muted">
                    <span>{{ song.category }}</span>
                    <span v-if="song.bpm != null" class="t-num">{{ song.bpm }} {{ t("term.bpm") }}</span>
                </div>
                <h2 :id="titleId" class="t-title sm-title">{{ song.title }}</h2>
                <p class="t-body t-muted sm-artist">{{ song.artist }}</p>

                <div
                    v-if="types.length > 1"
                    class="sm-chips"
                    role="group"
                    :aria-label="t('term.type') as string"
                >
                    <MvChip
                        v-for="type in types"
                        :key="type"
                        :active="type === selected.type"
                        @click="selectType(type)"
                    >
                        {{ getTypeAbbr(type) }}
                    </MvChip>
                </div>
                <div class="sm-chips" role="group" :aria-label="t('term.difficulty') as string">
                    <MvChip
                        v-for="sheet in sameType"
                        :key="sheet.key"
                        :diff="getDifficultyColor(sheet.difficulty)"
                        :active="sheet.key === selected.key"
                        @click="modal.select(sheet)"
                    >
                        {{ getDifficultyName(sheet.difficulty) }}
                    </MvChip>
                </div>

                <MvLevelLine
                    class="sm-level"
                    :name="getDifficultyName(selected.difficulty)"
                    :level="selected.level"
                    :internal-level-value="selected.internalLevelValue"
                    :diff="getDifficultyColor(selected.difficulty)"
                />

                <dl class="sm-facts">
                    <dt class="t-label t-muted">{{ t("term.noteDesigner") }}</dt>
                    <dd class="t-body">{{ selected.noteDesigner ?? "—" }}</dd>
                    <dt class="t-label t-muted">{{ t("term.releaseDate") }}</dt>
                    <dd class="t-body t-num">{{ releaseDate ?? "—" }}</dd>
                    <dt class="t-label t-muted">{{ t("term.type") }}</dt>
                    <dd class="t-body">{{ getTypeName(selected.type) }}</dd>
                </dl>
                <p v-if="noteCounts" class="t-meta t-num sm-counts">{{ noteCounts }}</p>

                <div class="sm-actions">
                    <MvButton
                        kind="text"
                        icon="bookmark"
                        :filled="isBookmarked"
                        :aria-pressed="isBookmarked"
                        @click="toggleBookmark"
                    >
                        {{ t("page.songs.bookmark") }}
                    </MvButton>
                    <MvButton
                        kind="text"
                        icon="play"
                        :disabled="!selected.hasChart"
                        :aria-describedby="selected.hasChart ? undefined : noChartId"
                        @click="play"
                    >
                        {{ t("page.songs.play") }}
                    </MvButton>
                    <span v-if="!selected.hasChart" :id="noChartId" class="t-meta t-muted">
                        {{ t("page.songs.noChart") }}
                    </span>
                </div>
            </template>
        </div>
    </MvModal>
</template>

<style scoped>
.sm-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    padding: var(--s-4);
    border-bottom: var(--line-w) solid var(--line);
}
.sm-jacket {
    width: 120px;
    aspect-ratio: 1;
    border-radius: var(--r);
    overflow: hidden;
    background: var(--bg-sunken);
}
.sm-body {
    padding: var(--s-4);
}
.sm-loading {
    margin: 0;
    padding: var(--s-8) 0;
    text-align: center;
}
.sm-top {
    display: flex;
    justify-content: space-between;
    gap: var(--s-4);
}
.sm-title {
    margin: var(--s-2) 0 0;
    overflow-wrap: anywhere;
}
.sm-artist {
    margin: var(--s-1) 0 var(--s-6);
}
.sm-chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s-2);
    margin-bottom: var(--s-3);
}
.sm-level {
    margin: var(--s-6) 0 var(--s-4);
}
.sm-facts {
    display: grid;
    grid-template-columns: max-content 1fr;
    align-items: baseline;
    gap: var(--s-2) var(--s-6);
    margin: 0 0 var(--s-4);
}
.sm-facts dd {
    margin: 0;
    overflow-wrap: anywhere;
}
.sm-counts {
    margin: 0 0 var(--s-6);
}
.sm-actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--s-2) var(--s-6);
}
</style>
