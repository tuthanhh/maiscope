<script setup lang="ts">
import type { RouteLocationRaw } from "vue-router";
import MvCover from "./MvCover.vue";
import MvLevelPill from "./MvLevelPill.vue";
import MvIcon from "./MvIcon.vue";

// design-direction "Card": the grid item for both libraries. No border, no
// background, no shadow; the whole card is one link. The bookmark mark is a
// separate control on the jacket. `designer` is the community card's tell.
defineProps<{
  to: RouteLocationRaw;
  cover: { imageUrl?: string; title?: string; artist?: string };
  title?: string;
  artist?: string;
  level?: string;
  diff?: string | null;
  designer?: string;
  bookmarked?: boolean;
  bookmarkLabel?: string;
  /** The link's accessible name, when the visible text is not enough. */
  label?: string;
}>();
defineEmits<{ bookmark: [] }>();
</script>

<template>
  <div class="mv-card">
    <RouterLink :to="to" class="mv-card-link" :aria-label="label">
      <span class="mv-card-jacket">
        <MvCover :item="cover" />
        <MvLevelPill v-if="level" class="mv-card-pill" :level="level" :diff="diff" />
      </span>
      <span class="mv-card-title t-strong">{{ title }}</span>
      <span class="mv-card-artist t-body t-muted">{{ artist }}</span>
      <span v-if="designer" class="mv-card-designer t-meta t-muted">
        <MvIcon name="pen" :size="12" />
        <span class="mv-card-designer-name">{{ designer }}</span>
      </span>
    </RouterLink>
    <button
      v-if="bookmarkLabel"
      type="button"
      class="mv-card-bookmark"
      :aria-label="bookmarkLabel"
      :aria-pressed="bookmarked"
      :title="bookmarkLabel"
      @click="$emit('bookmark')"
    >
      <MvIcon name="bookmark" :size="15" :filled="bookmarked" />
    </button>
  </div>
</template>

<style scoped lang="scss">
.mv-card {
  position: relative;
  min-width: 0;
}
.mv-card-link {
  display: block;
  color: var(--fg);
  text-decoration: none;
}
.mv-card-jacket,
.mv-card-title,
.mv-card-artist {
  display: block;
}
.mv-card-jacket {
  position: relative;
  aspect-ratio: 1;
  border-radius: var(--r);
  overflow: hidden;
  background: var(--bg-sunken);
  outline: var(--line-w) solid transparent;
  outline-offset: -1px;
  transition: outline-color var(--dur);
  margin-bottom: var(--s-2);
}
.mv-card-link:hover .mv-card-jacket {
  outline-color: var(--line-strong);
}
.mv-card-link:hover .mv-card-title {
  text-decoration: underline;
  text-underline-offset: 3px;
}
.mv-card-pill {
  position: absolute;
  right: var(--s-2);
  bottom: var(--s-2);
}
.mv-card-title,
.mv-card-artist,
.mv-card-designer-name {
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}
.mv-card-designer {
  display: flex;
  align-items: center;
  gap: var(--s-1);
}
.mv-card-bookmark {
  position: absolute;
  top: var(--s-1);
  right: var(--s-1);
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  border-radius: var(--r);
  background: var(--bg);
  color: var(--fg);
  cursor: pointer;
  transition: opacity var(--dur);
}
// The worked example's 28px bookmark mark; otherwise an icon button.
.mv-card-bookmark:hover {
  opacity: 0.7;
}
</style>
