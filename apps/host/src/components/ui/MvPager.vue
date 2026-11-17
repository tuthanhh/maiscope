<script setup lang="ts">
import { computed } from "vue";
import MvButton from "./MvButton.vue";

// design-direction "Pager": `1 – 22 of 1 038` at the left; previous, `3 / 48`
// and next at the right. Hidden when there is one page.
const props = defineProps<{
  page: number;
  pageSize: number;
  total: number;
  /** The pager's accessible name, e.g. "Pages". */
  label: string;
  prevLabel: string;
  nextLabel: string;
  /** Formats the range line, e.g. (from, to, total) => `${from} – ${to} of ${total}`. */
  rangeText: (from: string, to: string, total: string) => string;
}>();
const emit = defineEmits<{ page: [page: number] }>();

const pages = computed(() => Math.max(1, Math.ceil(props.total / props.pageSize)));
// Thousands grouped with a no-break space, as the design's "1 038", so a
// narrow row never breaks inside a number.
const group = (n: number) => String(n).replace(/\B(?=(\d{3})+(?!\d))/g, "\u00a0");
const range = computed(() => {
  const from = (props.page - 1) * props.pageSize + 1;
  const to = Math.min(props.total, props.page * props.pageSize);
  return props.rangeText(group(from), group(to), group(props.total));
});
</script>

<template>
  <nav v-if="pages > 1" class="mv-pager t-meta" :aria-label="label">
    <span class="t-muted">{{ range }}</span>
    <span class="mv-pager-controls">
      <MvButton
        kind="icon"
        icon="chevronLeft"
        :label="prevLabel"
        :disabled="page <= 1"
        @click="emit('page', page - 1)"
      />
      <span>{{ page }} / {{ pages }}</span>
      <MvButton
        kind="icon"
        icon="chevronRight"
        :label="nextLabel"
        :disabled="page >= pages"
        @click="emit('page', page + 1)"
      />
    </span>
  </nav>
</template>

<style scoped>
.mv-pager {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--s-4);
}
.mv-pager-controls {
  display: inline-flex;
  align-items: center;
  gap: var(--s-2);
}
</style>
