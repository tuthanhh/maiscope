<script setup lang="ts">
import { computed } from "vue";
import useI18n from "~/composables/useI18n";
import useGameData from "~/composables/useGameData";
import type { Sheet } from "~/types";
import MvIcon from "~/components/ui/MvIcon.vue";
import MvCover from "~/components/ui/MvCover.vue";

// The Catalog metadata card for a deep-linked sheet: jacket and identity,
// difficulty banner, quick stats, note breakdown, meta rows.
const props = defineProps<{ sheet: Sheet }>();

const { t } = useI18n();
const { getTypeName, getDifficultyName, getSheetSearchLink } = useGameData();

const internalLevel = computed(() =>
    props.sheet.internalLevelValue != null
        ? `(${props.sheet.internalLevelValue.toFixed(1)})`
        : "",
);
const noteCountRows = computed(() =>
    Object.entries(props.sheet.noteCounts ?? {})
        .filter(([key, v]) => v != null && key !== "total")
        .map(([key, value]) => ({ key, value: value as number })),
);
const totalNotes = computed(() => props.sheet.noteCounts?.total ?? null);

// maimai difficulty accent colors (engine-agnostic; just for the pill).
const DIFFICULTY_COLORS: Record<string, string> = {
    basic: "#3ba955",
    advanced: "#e0a000",
    expert: "#e0405a",
    master: "#9a4fd0",
    remaster: "#c6a8e8",
};
const difficultyColor = computed(
    () => DIFFICULTY_COLORS[props.sheet.difficulty ?? ""] ?? "var(--mv-fg)",
);

function searchOnYouTube(): void {
    const link = getSheetSearchLink(props.sheet);
    if (link != null) window.open(link, "_blank", "noopener");
}
</script>

<template>
    <div class="mv-card">
        <!-- header: jacket + identity -->
        <div class="mv-card-head">
            <div class="mv-card-jacket">
                <MvCover :item="sheet" />
            </div>
            <div class="mv-card-id">
                <span class="mv-card-cat">{{
                    sheet.category
                }}</span>
                <div class="mv-card-title">{{ sheet.title }}</div>
                <div class="mv-card-artist">{{ sheet.artist }}</div>
            </div>
        </div>

        <!-- difficulty + level banner -->
        <div
            class="mv-card-diff"
            :style="{ '--diff': difficultyColor }"
        >
            <span class="mv-card-diff-name">
                {{ getDifficultyName(sheet.difficulty) }}
            </span>
            <span class="mv-card-diff-level">
                {{ sheet.level }}
                <span class="mv-card-diff-internal">{{
                    internalLevel
                }}</span>
            </span>
        </div>

        <!-- quick stats -->
        <div class="mv-card-stats">
            <div class="mv-stat">
                <span class="mv-stat-k">{{ t("term.type") }}</span>
                <span class="mv-stat-v">{{
                    getTypeName(sheet.type)
                }}</span>
            </div>
            <div v-if="sheet.bpm" class="mv-stat">
                <span class="mv-stat-k">{{ t("term.bpm") }}</span>
                <span class="mv-stat-v">{{ sheet.bpm }}</span>
            </div>
            <div v-if="totalNotes != null" class="mv-stat">
                <span class="mv-stat-k">{{
                    t("term.totalNotes")
                }}</span>
                <span class="mv-stat-v">{{ totalNotes }}</span>
            </div>
        </div>

        <!-- note-count breakdown -->
        <div v-if="noteCountRows.length" class="mv-notes-grid">
            <div
                v-for="row in noteCountRows"
                :key="row.key"
                class="mv-note-cell"
            >
                <span class="mv-note-v">{{ row.value }}</span>
                <span class="mv-note-k">{{ row.key }}</span>
            </div>
        </div>

        <!-- meta rows -->
        <div class="mv-card-meta">
            <div v-if="sheet.noteDesigner" class="mv-meta">
                <span class="mv-label">{{
                    t("term.noteDesigner")
                }}</span>
                <span>{{ sheet.noteDesigner }}</span>
            </div>
            <div v-if="sheet.version" class="mv-meta">
                <span class="mv-label">{{ t("term.version") }}</span>
                <span>{{ sheet.version }}</span>
            </div>
            <div v-if="sheet.releaseDate" class="mv-meta">
                <span class="mv-label">{{
                    t("term.releaseDate")
                }}</span>
                <span>{{ sheet.releaseDate }}</span>
            </div>
        </div>

        <button
            type="button"
            class="mv-btn mv-yt"
            @click="searchOnYouTube"
        >
            <MvIcon name="youtube" :size="15" />
            <span>{{ t("sfc.SheetDialog.searchOnYouTube") }}</span>
        </button>
    </div>
</template>

<style scoped>
/* ── info card ─────────────────────────────────────────────────────────── */
.mv-card {
    margin-top: 12px;
    border: 1px solid var(--mv-bd);
    border-radius: 8px;
    overflow: hidden;
    background: var(--mv-panel);
}

.mv-card-head {
    display: flex;
    gap: 12px;
    padding: 14px;
}
.mv-card-jacket {
    flex: 0 0 84px;
    width: 84px;
    height: 84px;
    border-radius: 6px;
    overflow: hidden;
    background: var(--mv-jacket-bg);
    border: 1px solid var(--mv-bd);
}
.mv-card-id {
    min-width: 0;
    display: flex;
    flex-direction: column;
    justify-content: center;
}
.mv-card-cat {
    align-self: flex-start;
    font-size: 9px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--mv-mut);
    border: 1px solid var(--mv-bd);
    border-radius: 3px;
    padding: 2px 6px;
    margin-bottom: 6px;
}
.mv-card-title {
    font-size: 17px;
    font-weight: 700;
    line-height: 1.2;
    word-break: break-word;
}
.mv-card-artist {
    font-size: 11px;
    color: var(--mv-mut);
    margin-top: 2px;
    word-break: break-word;
}

.mv-card-diff {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 14px;
    color: #fff;
    background: var(--diff);
}
.mv-card-diff-name {
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
}
.mv-card-diff-level {
    font-size: 20px;
    font-weight: 800;
    font-family: var(--mv-mono);
}
.mv-card-diff-internal {
    font-size: 12px;
    font-weight: 500;
    opacity: 0.85;
}

.mv-card-stats {
    display: flex;
    border-bottom: 1px solid var(--mv-bd);
}
.mv-stat {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 3px;
    align-items: center;
    padding: 10px 4px;
    border-right: 1px solid var(--mv-bd);
}
.mv-stat:last-child {
    border-right: none;
}
.mv-stat-k {
    font-size: 9px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--mv-mut);
}
.mv-stat-v {
    font-size: 14px;
    font-weight: 700;
    font-family: var(--mv-mono);
    text-align: center;
}

.mv-notes-grid {
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    gap: 1px;
    background: var(--mv-bd);
    border-bottom: 1px solid var(--mv-bd);
}
.mv-note-cell {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    padding: 9px 2px;
    background: var(--mv-panel);
}
.mv-note-v {
    font-size: 14px;
    font-weight: 700;
    font-family: var(--mv-mono);
}
.mv-note-k {
    font-size: 8px;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--mv-mut);
}

.mv-card-meta {
    padding: 6px 14px;
}
.mv-meta {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    padding: 7px 0;
    font-size: 12px;
    border-bottom: 1px solid var(--mv-bd2);
}
.mv-meta:last-child {
    border-bottom: none;
}
.mv-meta > span:last-child {
    text-align: right;
    word-break: break-word;
}

.mv-yt {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 7px;
    width: 100%;
    box-sizing: border-box;
    background: none;
    color: var(--mv-fg);
    border: none;
    border-top: 1px solid var(--mv-bd);
    font-family: var(--mv-mono);
    font-size: 11px;
    letter-spacing: 0.04em;
    padding: 11px;
}
.mv-yt:hover {
    background: var(--mv-bg);
    color: #e0405a;
}
</style>
