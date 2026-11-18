<script setup lang="ts">
// A sheet's level as the song modal and the chart page show it: the
// difficulty's name, the level in its colour, the internal level after it
// (design-direction "Song modal", "Chart page"). `diff` null is no colour.
defineProps<{
  name: string;
  level?: string;
  internalLevelValue?: number;
  diff: string | null;
}>();
</script>

<template>
  <p class="mv-level-line" :style="{ '--diff': diff ?? 'var(--fg)' }">
    <span class="t-label">{{ name }}</span>
    <span class="t-heading t-num mv-level-line-value">{{ level ?? "?" }}</span>
    <span v-if="internalLevelValue != null" class="t-meta t-num t-muted">
      ({{ internalLevelValue.toFixed(1) }})
    </span>
  </p>
</template>

<style scoped>
.mv-level-line {
  display: flex;
  align-items: baseline;
  gap: var(--s-2);
  margin: 0;
}
.mv-level-line-value {
  color: var(--diff);
}
</style>
