<script setup lang="ts">
import { computed } from "vue";
import MvIcon from "~/components/ui/MvIcon.vue";

defineOptions({ name: "BrowsePagination" });

const props = defineProps<{
    currentPage: number;
    totalPages: number;
    pageSize: number;
    total: number;
}>();
const emit = defineEmits<{ prev: []; next: [] }>();

const rangeStart = computed(() => (props.currentPage - 1) * props.pageSize + 1);
const rangeEnd = computed(() =>
    Math.min(props.currentPage * props.pageSize, props.total),
);
</script>

<template>
    <div class="mv-pagination">
        <span class="mv-page-info">
            {{ rangeStart }} – {{ rangeEnd }} of {{ total }}
        </span>
        <div class="mv-page-controls">
            <button
                type="button"
                class="mv-btn mv-page-btn"
                :disabled="currentPage === 1"
                @click="emit('prev')"
            >
                <MvIcon name="chevron" :size="13" class="mv-chevron-left" />
            </button>
            <span class="mv-page-current"
                >{{ currentPage }} / {{ totalPages }}</span
            >
            <button
                type="button"
                class="mv-btn mv-page-btn"
                :disabled="currentPage === totalPages"
                @click="emit('next')"
            >
                <MvIcon name="chevron" :size="13" class="mv-chevron-right" />
            </button>
        </div>
    </div>
</template>

<style scoped>
.mv-pagination {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    margin-top: 24px;
    padding-top: 20px;
    border-top: 1px solid var(--mv-bd2);
}
.mv-page-controls {
    display: flex;
    align-items: center;
    gap: 16px;
}
.mv-page-btn {
    background: var(--mv-panel2);
    border: 1px solid var(--mv-bd);
    border-radius: 4px;
    width: 36px;
    height: 36px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--mv-fg);
    transition: all 0.15s ease;
}
.mv-page-btn:hover:not(:disabled) {
    border-color: var(--mv-fg);
}
.mv-page-btn:disabled {
    opacity: 0.25;
    cursor: not-allowed;
    border-color: var(--mv-bd2);
}
.mv-page-current {
    font-size: 14px;
    font-weight: 700;
    color: var(--mv-fg);
    min-width: 56px;
    text-align: center;
    font-family: var(--mv-mono);
}
.mv-page-info {
    font-size: 12px;
    color: var(--mv-mut);
    letter-spacing: 0.05em;
}
/* The default MvIcon chevron points down, so we rotate it to face left/right */
.mv-chevron-left {
    transform: rotate(90deg);
}
.mv-chevron-right {
    transform: rotate(-90deg);
}
</style>
