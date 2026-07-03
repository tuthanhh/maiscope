<script setup lang="ts">
// Toggle chip. Invert = active (fg/bg flip), the recurring active-state trick.
withDefaults(defineProps<{ active?: boolean }>(), { active: false });
</script>

<template>
  <button
    type="button"
    class="mv-btn mv-chip"
    :class="{ 'mv-chip--active': active }"
  >
    <slot />
  </button>
</template>

<style scoped>
.mv-chip {
  font-family: inherit;
  font-size: 11px;
  padding: 5px 10px;
  border-radius: 3px;
  border: 1px solid var(--mv-bd);
  background: transparent;
  color: var(--mv-mut);
  display: inline-flex;
  align-items: center;
  gap: 5px;
}
.mv-chip--active {
  border-color: var(--mv-fg);
  background: var(--mv-invbg);
  color: var(--mv-inv);
}
</style>
