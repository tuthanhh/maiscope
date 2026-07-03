<script setup lang="ts">
import { computed } from "vue";
import MvJacket from "./MvJacket.vue";

// Cover art with a deterministic generative fallback. Wherever a song/sheet
// jacket is shown (browse cards & rows, the sheet dialog) the real cover →
// generative jacket fallback and the seed derivation live here, once.
const props = defineProps<{
  item: { imageUrl?: string; title?: string; artist?: string };
}>();

const seed = computed(
  () => `${props.item.title ?? ""}${props.item.artist ?? ""}`,
);
</script>

<template>
  <img
    v-if="item.imageUrl"
    :src="item.imageUrl"
    :alt="item.title"
    class="mv-cover"
  />
  <MvJacket v-else :seed="seed" />
</template>

<style scoped>
.mv-cover {
  width: 100%;
  height: 100%;
  object-fit: contain;
  display: block;
}
</style>
