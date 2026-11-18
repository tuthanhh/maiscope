<script setup lang="ts">
import { computed, ref, watch } from "vue";
import useI18n from "~/composables/useI18n";
import useGameData from "~/composables/useGameData";
import type { Sheet } from "~/types";
import MvButton from "~/components/ui/MvButton.vue";
import MvCover from "~/components/ui/MvCover.vue";

// design-direction "Chart page": the title block under the playback surface.
// Title and level, the meta line, the actions as text buttons, the jacket
// with the sheet's figures, then the song's other difficulties. Names and
// the level colour come from the reference tables (`useGameData`).
const props = defineProps<{
  sheet: Sheet;
  /** The song's other sheets, from the Catalog when it is loaded. */
  siblings: Sheet[];
  bookmarked: boolean;
  /** Feedback after Share copied the link, shown beside the actions. */
  shareNote?: string;
}>();
defineEmits<{ bookmark: []; share: [] }>();

const { t } = useI18n();

// Following an "other difficulty" link removes that link (it becomes the
// current sheet), dropping focus to <body>. Move it to the new title, which
// also announces the new sheet.
const heading = ref<HTMLElement | null>(null);
// After the DOM update, when the followed link is already gone.
watch(
  () => props.sheet.key,
  () => {
    if (document.activeElement === document.body) heading.value?.focus();
  },
  { flush: "post" },
);
const { getTypeAbbr, getDifficultyName, getDifficultyColor, getVersionAbbr } = useGameData();

function colorOf(sheet: Sheet): string {
  const color = getDifficultyColor(sheet.difficulty);
  return color === "unset" ? "var(--fg)" : color;
}

const meta = computed(() =>
  [
    props.sheet.song.artist,
    props.sheet.song.category,
    props.sheet.song.bpm != null ? `${props.sheet.song.bpm} ${t("term.bpm")}` : null,
  ].filter(Boolean),
);
const NOTE_KEYS = ["total", "tap", "hold", "slide", "touch", "break"];
const counts = computed(() => {
  const c = props.sheet.noteCounts;
  if (!c) return "";
  return NOTE_KEYS.filter((k) => c[k] != null)
    .map((k) => `${k === "total" ? t("term.totalNotes") : k.toUpperCase()} ${c[k]}`)
    .join(" · ");
});
const facts = computed(() =>
  [
    getTypeAbbr(props.sheet.type),
    props.sheet.song.version ? getVersionAbbr(props.sheet.song.version) : null,
    props.sheet.song.releaseDate?.replaceAll("-", "/"),
  ].filter(Boolean),
);
// Type in the name only when the song has more than one chart type.
const mixedTypes = computed(
  () => new Set([props.sheet, ...props.siblings].map((s) => s.type)).size > 1,
);
function siblingName(sheet: Sheet): string {
  const name = getDifficultyName(sheet.difficulty);
  return mixedTypes.value ? `${getTypeAbbr(sheet.type)} ${name}` : name;
}
</script>

<template>
  <section class="viz-title">
    <div class="viz-title-head">
      <h1 ref="heading" class="t-title viz-title-name" tabindex="-1">{{ sheet.song.title }}</h1>
      <p class="viz-title-level" :style="{ '--diff': colorOf(sheet) }">
        <span class="t-label">{{ siblingName(sheet) }}</span>
        <span class="t-heading t-num viz-title-level-value">{{ sheet.level ?? "?" }}</span>
        <span v-if="sheet.internalLevelValue != null" class="t-meta t-num t-muted">
          ({{ sheet.internalLevelValue.toFixed(1) }})
        </span>
      </p>
    </div>
    <p class="t-body t-muted viz-title-meta">{{ meta.join(" · ") }}</p>

    <div class="viz-title-actions">
      <MvButton
        kind="text"
        icon="bookmark"
        :filled="bookmarked"
        :aria-pressed="bookmarked"
        @click="$emit('bookmark')"
      >
        {{ t("page.songs.bookmark") }}
      </MvButton>
      <MvButton kind="text" icon="share" @click="$emit('share')">
        {{ t("page.viz.share") }}
      </MvButton>
      <span class="t-meta t-muted" aria-live="polite">{{ shareNote }}</span>
    </div>

    <div class="viz-title-sheet">
      <div class="viz-title-jacket"><MvCover :item="sheet.song" /></div>
      <div class="viz-title-facts">
        <dl class="viz-title-designer">
          <dt class="t-label t-muted">{{ t("term.noteDesigner") }}</dt>
          <dd class="t-body">{{ sheet.noteDesigner ?? "—" }}</dd>
        </dl>
        <p v-if="counts" class="t-meta t-num">{{ counts }}</p>
        <p class="t-meta t-num t-muted">{{ facts.join(" · ") }}</p>
      </div>
    </div>

    <div v-if="siblings.length" class="viz-title-others">
      <h2 class="t-label t-muted viz-title-others-head">{{ t("page.viz.otherDifficulties") }}</h2>
      <ul class="viz-title-others-list">
        <li v-for="s in siblings" :key="s.key">
          <RouterLink
            class="t-body viz-title-other"
            :to="{
              name: 'visualizer',
              query: { song: s.song.id, type: s.type, difficulty: s.difficulty },
            }"
          >
            {{ siblingName(s) }}
            <span class="t-num">{{ s.level }}</span>
            <span v-if="s.noteCounts?.total != null" class="t-meta t-num t-muted">
              · {{ t("page.viz.notes", { count: s.noteCounts.total }) }}
            </span>
          </RouterLink>
        </li>
      </ul>
    </div>
  </section>
</template>

<style scoped>
.viz-title-head {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  justify-content: space-between;
  gap: var(--s-2) var(--s-6);
}
.viz-title-name {
  margin: 0;
  outline: none;
  overflow-wrap: anywhere;
}
.viz-title-level {
  display: flex;
  align-items: baseline;
  gap: var(--s-2);
  margin: 0;
}
.viz-title-level-value {
  color: var(--diff);
}
.viz-title-meta {
  margin: var(--s-1) 0 var(--s-4);
}
.viz-title-actions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--s-2) var(--s-6);
  margin-bottom: var(--s-6);
}
.viz-title-sheet {
  display: flex;
  gap: var(--s-4);
  align-items: flex-start;
}
.viz-title-jacket {
  flex: 0 0 96px;
  aspect-ratio: 1;
  border-radius: var(--r);
  overflow: hidden;
  background: var(--bg-sunken);
}
.viz-title-facts {
  display: flex;
  flex-direction: column;
  gap: var(--s-2);
  min-width: 0;
}
.viz-title-facts p,
.viz-title-designer,
.viz-title-designer dd {
  margin: 0;
}
.viz-title-designer {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: var(--s-3);
}
.viz-title-others {
  margin-top: var(--s-8);
  padding-top: var(--s-6);
  border-top: var(--line-w) solid var(--line);
}
.viz-title-others-head {
  margin: 0 0 var(--s-2);
}
.viz-title-others-list {
  margin: 0;
  padding: 0;
  list-style: none;
}
.viz-title-other {
  display: inline-flex;
  align-items: center;
  gap: var(--s-2);
  min-height: var(--control-h);
  color: var(--fg);
  text-decoration: none;
}
.viz-title-other:hover {
  text-decoration: underline;
  text-underline-offset: 3px;
}
</style>
