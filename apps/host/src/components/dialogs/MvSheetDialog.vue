<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { useRouter } from "vue-router";
import useI18n from "~/composables/useI18n";
import useGameData from "~/composables/useGameData";
import useSheetDialog from "~/composables/useSheetDialog";
import useSelectedSheets from "~/composables/useSelectedSheets";
import { useDataStore } from "~/stores/data";
import { isCanonicalSheet } from "~/utils";
import type { Sheet } from "~/types";
import MvIcon from "~/components/ui/MvIcon.vue";
import MvCover from "~/components/ui/MvCover.vue";
import MvChip from "~/components/ui/MvChip.vue";

defineOptions({ name: "MvSheetDialog" });

const { t } = useI18n();
const router = useRouter();
const { getDifficultyName, getTypeName, getSheetSearchLink } = useGameData();
const { isOpened, currentSheet } = useSheetDialog();
const { selectedSheets, toggleSheetSelection } = useSelectedSheets();
const dataStore = useDataStore();

// Which difficulty within the song is shown. Defaults to the opened sheet,
// switchable via the difficulty chips.
const selected = ref<Sheet>(currentSheet.value);
watch(currentSheet, (sheet) => {
    selected.value = sheet;
});

// Sibling sheets of the same song (the difficulty chips).
const siblingSheets = computed<Sheet[]>(() => {
    const song = dataStore.currentData.songs.find(
        (s) => s.songId === currentSheet.value.songId,
    );
    return song?.sheets ?? [currentSheet.value];
});

const internalLevel = computed(() =>
    selected.value.internalLevelValue != null
        ? `(${selected.value.internalLevelValue.toFixed(1)})`
        : "",
);

const metaRows = computed(() => [
    {
        key: t("term.noteDesigner"),
        value: selected.value.noteDesigner ?? "N/A",
    },
    { key: t("term.releaseDate"), value: selected.value.releaseDate ?? "N/A" },
    { key: t("term.type"), value: getTypeName(selected.value.type) },
    { key: t("term.comment"), value: selected.value.comment ?? "N/A" },
]);

const isBookmarked = computed(() =>
    selectedSheets.value.includes(selected.value),
);

function close(): void {
    isOpened.value = false;
}
function toggleBookmark(): void {
    // Bookmarks are keyed on canonical sheets; the opened sheet already is one.
    if (isCanonicalSheet(selected.value)) toggleSheetSelection(selected.value);
}
function searchOnYouTube(): void {
    const link = getSheetSearchLink(selected.value);
    if (link != null) window.open(link, "_blank", "noopener");
}
function openInVisualizer(): void {
    const { songId, type, difficulty } = selected.value;
    close();
    // Deep-link the visualizer to this sheet; it fetches the chart from the
    // backend on mount. Without a chart, it opens blank (manual-paste mode).
    router.push({
        name: "visualizer",
        query:
            songId != null && type != null && difficulty != null
                ? { songId, type, difficulty }
                : undefined,
    });
}
</script>

<template>
    <div v-if="isOpened" class="mv-root mv-overlay" @click="close">
        <div class="mv-modal" @click.stop>
            <!-- jacket header (real cover, generative fallback) -->
            <div class="mv-modal-art">
                <MvCover :item="selected" />

                <button
                    type="button"
                    class="mv-btn mv-art-bookmark"
                    @click="toggleBookmark"
                >
                    <MvIcon name="bookmark" :size="18" :filled="isBookmarked" />
                </button>
                <span class="mv-art-badge">{{ selected.category }}</span>
            </div>

            <div class="mv-modal-body">
                <div class="mv-modal-top">
                    <span class="mv-label">{{ selected.category }}</span>
                    <span class="mv-label mv-bpm"
                        >{{ selected.bpm }} {{ t("term.bpm") }}</span
                    >
                </div>
                <div class="mv-modal-title">{{ selected.title }}</div>
                <div class="mv-modal-artist">{{ selected.artist }}</div>

                <div class="mv-modal-cols">
                    <div class="mv-modal-main">
                        <div class="mv-diff-row">
                            <MvChip
                                v-for="(sheet, i) in siblingSheets"
                                :key="i"
                                :active="
                                    sheet.difficulty === selected.difficulty
                                "
                                @click="selected = sheet"
                            >
                                {{ getDifficultyName(sheet.difficulty) }}
                            </MvChip>
                        </div>

                        <div class="mv-level">
                            {{ getDifficultyName(selected.difficulty) }}
                            {{ selected.level }}
                            <span class="mv-level-internal">{{
                                internalLevel
                            }}</span>
                        </div>

                        <div
                            v-for="row in metaRows"
                            :key="row.key"
                            class="mv-meta"
                        >
                            <span class="mv-label">{{ row.key }}</span>
                            <span class="mv-meta-value">{{ row.value }}</span>
                        </div>
                    </div>

                    <div class="mv-modal-side">
                        <div
                            class="mv-side-icon"
                            :class="{ on: selected.isLocked }"
                        >
                            <MvIcon name="lock" :size="15" />
                        </div>
                        <div class="mv-side-icon">
                            <MvIcon name="globe" :size="15" />
                        </div>
                    </div>
                </div>

                <div class="mv-modal-actions">
                    <button
                        type="button"
                        class="mv-btn mv-act-close"
                        @click="close"
                    >
                        {{ t("ui.close") }}
                    </button>
                    <button
                        type="button"
                        class="mv-btn mv-act-ghost"
                        :title="t('sfc.SheetDialog.searchOnYouTube')"
                        @click="searchOnYouTube"
                    >
                        <MvIcon name="youtube" :size="16" />
                    </button>
                    <button
                        type="button"
                        class="mv-btn mv-act-primary"
                        :class="{ 'mv-act-primary--ready': selected.hasChart }"
                        :title="
                            selected.hasChart
                                ? t('page.viz.load')
                                : t('page.viz.awaiting')
                        "
                        @click="openInVisualizer"
                    >
                        <MvIcon name="ext" :size="16" />
                    </button>
                </div>
            </div>
        </div>
    </div>
</template>

<style scoped>
.mv-overlay {
    position: fixed;
    inset: 0;
    z-index: 50;
    background: var(--mv-ov);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 16px;
}
.mv-modal {
    width: 540px;
    max-width: 100%;
    max-height: 90vh;
    overflow: auto;
    background: var(--mv-bg);
    color: var(--mv-fg);
    border: 1px solid var(--mv-bd);
    border-radius: 8px;
    animation: mv-fade 0.18s ease;
}

.mv-modal-art {
    position: relative;
    aspect-ratio: 16 / 10;
    border-bottom: 1px solid var(--mv-bd);
    background: var(--mv-jacket-bg);
}
.mv-art-bookmark {
    position: absolute;
    top: 10px;
    right: 10px;
    width: 36px;
    height: 36px;
    border-radius: 50%;
    background: var(--mv-bg);
    border: 1px solid var(--mv-bd);
    color: var(--mv-fg);
    display: flex;
    align-items: center;
    justify-content: center;
}
.mv-art-badge {
    position: absolute;
    left: 12px;
    bottom: 12px;
    background: var(--mv-invbg);
    color: var(--mv-inv);
    font-size: 10px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    padding: 4px 10px;
    border-radius: 3px;
    font-weight: 700;
}
.mv-art-ext {
    position: absolute;
    right: 12px;
    bottom: 12px;
    background: var(--mv-bg);
    border: 1px solid var(--mv-bd);
    border-radius: 4px;
    padding: 6px;
    color: var(--mv-fg);
    display: flex;
}

.mv-modal-body {
    padding: 18px;
}
.mv-modal-top {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
}
.mv-bpm {
    color: var(--mv-fg);
}
.mv-modal-title {
    font-size: 24px;
    font-weight: 700;
    line-height: 1.15;
    margin: 6px 0 2px;
    word-break: break-word;
}
.mv-modal-artist {
    font-size: 12px;
    color: var(--mv-mut);
    margin-bottom: 16px;
}

.mv-modal-cols {
    display: flex;
    gap: 16px;
}
.mv-modal-main {
    flex: 1;
    min-width: 0;
}
.mv-diff-row {
    display: flex;
    gap: 5px;
    flex-wrap: wrap;
    margin-bottom: 16px;
}
.mv-level {
    font-size: 20px;
    font-weight: 700;
    margin-bottom: 16px;
}
.mv-level-internal {
    font-size: 13px;
    color: var(--mv-mut);
    font-weight: 400;
}
.mv-meta {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    padding: 7px 0;
    border-bottom: 1px solid var(--mv-bd2);
}
.mv-meta-value {
    font-size: 12px;
    text-align: right;
    word-break: break-word;
}

.mv-modal-side {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding-top: 4px;
}
.mv-side-icon {
    width: 32px;
    height: 32px;
    border: 1px solid var(--mv-bd);
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--mv-mut);
}
.mv-side-icon.on {
    color: var(--mv-fg);
    border-color: var(--mv-fg);
}

.mv-modal-actions {
    display: flex;
    gap: 8px;
    margin-top: 20px;
    justify-content: flex-end;
    align-items: center;
}
.mv-act-close {
    background: none;
    border: none;
    color: var(--mv-mut);
    font-family: var(--mv-mono);
    font-size: 12px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    padding: 8px 10px;
}
.mv-act-ghost {
    display: flex;
    align-items: center;
    justify-content: center;
    background: none;
    color: var(--mv-fg);
    border: 1px solid var(--mv-bd);
    border-radius: 3px;
    font-family: var(--mv-mono);
    font-size: 12px;
    padding: 9px 14px;
}
.mv-act-primary {
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--mv-invbg);
    color: var(--mv-inv);
    border: none;
    border-radius: 3px;
    font-family: var(--mv-mono);
    font-size: 12px;
    font-weight: 700;
    padding: 9px 14px;
    letter-spacing: 0.06em;
}
</style>
