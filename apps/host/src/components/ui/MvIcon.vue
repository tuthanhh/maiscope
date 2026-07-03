<script setup lang="ts">
import { computed } from "vue";
import icons, { type IconName } from "./icons";

const props = withDefaults(
  defineProps<{
    name: IconName;
    size?: number;
    // Fill is a state signal, not decoration (bookmarked / playing).
    filled?: boolean;
    strokeWidth?: number;
  }>(),
  { size: 16, filled: false, strokeWidth: 1.6 },
);

const inner = computed(() => icons[props.name]);
</script>

<template>
  <svg
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    :fill="filled ? 'currentColor' : 'none'"
    stroke="currentColor"
    :stroke-width="strokeWidth"
    stroke-linecap="round"
    stroke-linejoin="round"
    style="display: block"
    v-html="inner"
  />
</template>
