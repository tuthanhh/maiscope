<script setup lang="ts">
import type { Sheet } from "~/types";
import MvIcon from "~/components/ui/MvIcon.vue";
import MvCover from "~/components/ui/MvCover.vue";

defineOptions({ name: "BrowseGrid" });

const props = defineProps<{ sheets: Sheet[]; selectedSet: Set<Sheet> }>();
const emit = defineEmits<{ select: [sheet: Sheet] }>();

function isBookmarked(sheet: Sheet): boolean {
    return props.selectedSet.has(sheet);
}
</script>

<template>
    <div class="mv-grid">
        <div
            v-for="(s, i) in sheets"
            :key="s.sheetExpr ?? i"
            class="mv-card mv-btn"
            @click="emit('select', s)"
        >
            <div class="mv-card-art">
                <MvCover :item="s" />
                <span class="mv-card-level">{{ s.level }}</span>
                <span v-if="isBookmarked(s)" class="mv-card-bm">
                    <MvIcon name="bookmark" :size="15" filled />
                </span>
            </div>
            <div class="mv-card-meta">
                <div class="mv-card-title">{{ s.title }}</div>
                <div class="mv-card-artist">{{ s.artist }}</div>
            </div>
        </div>
    </div>
</template>

<style scoped>
.mv-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 12px;
}
.mv-card {
    border: 1px solid var(--mv-bd);
    border-radius: 5px;
    overflow: hidden;
    transition: border-color 0.12s;
}
.mv-card:hover {
    border-color: var(--mv-fg);
    opacity: 1;
}
.mv-card-art {
    position: relative;
    aspect-ratio: 1;
}
.mv-card-level {
    position: absolute;
    right: 6px;
    bottom: 6px;
    background: var(--mv-invbg);
    color: var(--mv-inv);
    font-size: 11px;
    font-weight: 700;
    padding: 2px 7px;
    border-radius: 3px;
}
.mv-card-bm {
    position: absolute;
    left: 6px;
    top: 4px;
    color: var(--mv-fg);
    display: flex;
}
.mv-card-meta {
    padding: 8px 9px;
}
.mv-card-title {
    font-size: 12px;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}
.mv-card-artist {
    font-size: 10px;
    color: var(--mv-mut);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}
</style>
