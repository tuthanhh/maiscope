<script setup lang="ts">
// design-direction "Level pill": the level in `meta` 600 on `--bg`, a
// `--jacket-bar` left bar in the difficulty's colour, a 1px `--bg` ring so it
// separates from any artwork. Colour never backs the text.
defineProps<{ level?: string; diff?: string | null }>();
</script>

<template>
  <span class="mv-pill t-meta" :style="{ '--diff': diff ?? 'var(--fg)' }">
    {{ level || "?" }}
  </span>
</template>

<style scoped>
.mv-pill {
  display: inline-flex;
  align-items: center;
  height: 20px;
  padding: 0 var(--s-2);
  border-radius: var(--r);
  background: var(--bg);
  color: var(--fg);
  font-weight: 600;
  box-shadow:
    inset var(--jacket-bar) 0 0 var(--diff),
    0 0 0 1px var(--bg);
}
</style>
