<script setup lang="ts">
import useGameData from "~/composables/useGameData";
import type { Sheet } from "~/types";
import MvIcon from "~/components/ui/MvIcon.vue";
import MvCover from "~/components/ui/MvCover.vue";

defineOptions({ name: "BrowseList" });

const props = defineProps<{ sheets: Sheet[]; selectedSet: Set<Sheet> }>();
const emit = defineEmits<{
    select: [sheet: Sheet];
    bookmark: [sheet: Sheet, event: Event];
}>();

const { getTypeAbbr } = useGameData();

function isBookmarked(sheet: Sheet): boolean {
    return props.selectedSet.has(sheet);
}
</script>

<template>
    <div class="mv-list">
        <div
            v-for="(s, i) in sheets"
            :key="s.sheetExpr ?? i"
            class="mv-row mv-btn"
            @click="emit('select', s)"
        >
            <div class="mv-row-art">
                <MvCover :item="s" />
            </div>
            <span class="mv-row-level">{{ s.level }}</span>
            <div class="mv-row-main">
                <div class="mv-row-title">{{ s.title }}</div>
                <div class="mv-row-artist">{{ s.artist }}</div>
            </div>
            <span class="mv-row-cat">{{ s.category }}</span>
            <span class="mv-row-type">{{ getTypeAbbr(s.type) }}</span>
            <span class="mv-row-bpm">{{ s.bpm }} BPM</span>
            <button
                type="button"
                class="mv-btn mv-icon-btn mv-row-bm"
                @click="emit('bookmark', s, $event)"
            >
                <MvIcon name="bookmark" :size="15" :filled="isBookmarked(s)" />
            </button>
        </div>
    </div>
</template>

<style scoped>
.mv-list {
    border: 1px solid var(--mv-bd);
    border-radius: 4px;
    overflow: hidden;
}
.mv-row {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 10px 14px;
    border-bottom: 1px solid var(--mv-bd2);
}
.mv-row:last-child {
    border-bottom: none;
}
.mv-row:hover {
    background: var(--mv-panel2);
    opacity: 1;
}
.mv-row-art {
    width: 40px;
    height: 40px;
    flex-shrink: 0;
    border-radius: 3px;
    overflow: hidden;
}
.mv-row-level {
    width: 48px;
    text-align: center;
    font-weight: 700;
    font-size: 15px;
    border: 1px solid var(--mv-bd);
    border-radius: 3px;
    padding: 5px 0;
}
.mv-row-main {
    flex: 1;
    min-width: 0;
}
.mv-row-title {
    font-size: 13px;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}
.mv-row-artist {
    font-size: 11px;
    color: var(--mv-mut);
}
.mv-row-cat {
    font-size: 10px;
    color: var(--mv-mut);
    letter-spacing: 0.1em;
    width: 90px;
    text-align: right;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}
.mv-row-type {
    font-size: 10px;
    color: var(--mv-mut);
    width: 40px;
    text-align: right;
}
.mv-row-bpm {
    font-size: 10px;
    color: var(--mv-mut);
    width: 70px;
    text-align: right;
}
.mv-icon-btn {
    background: none;
    border: none;
    padding: 0;
    display: flex;
    color: var(--mv-mut);
}
.mv-row-bm {
    color: var(--mv-mut);
}
</style>
