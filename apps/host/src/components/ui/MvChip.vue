<script setup lang="ts">
// design-direction "Chip": a filter toggle or a difficulty selector. Selected
// is inverse — except a difficulty chip, which underlines in its `--diff`
// colour, so five selected difficulties never sit as five filled blocks.
withDefaults(
  defineProps<{
    active?: boolean;
    /** Difficulty form: the difficulty's colour, or null for none (UTAGE). */
    diff?: string | null;
  }>(),
  { active: false, diff: undefined },
);
</script>

<template>
  <button
    type="button"
    class="mv-chip t-meta"
    :class="{
      'mv-chip--active': active,
      'mv-chip--diff': diff !== undefined,
    }"
    :style="diff !== undefined ? { '--diff': diff ?? 'var(--fg)' } : undefined"
    :aria-pressed="active"
  >
    <slot />
  </button>
</template>

<style scoped>
.mv-chip {
  display: inline-flex;
  align-items: center;
  gap: var(--s-1);
  height: var(--control-h);
  padding: 0 var(--s-3);
  border-radius: var(--r);
  border: var(--line-w) solid var(--line);
  background: transparent;
  color: var(--fg);
  cursor: pointer;
  transition:
    border-color var(--dur),
    background var(--dur),
    color var(--dur);
}
.mv-chip:hover {
  border-color: var(--line-strong);
}
.mv-chip--active:not(.mv-chip--diff) {
  background: var(--inverse-bg);
  border-color: transparent;
  color: var(--inverse-fg);
}
.mv-chip--diff.mv-chip--active {
  text-decoration: underline 2px var(--diff);
  text-underline-offset: 5px;
}
</style>
