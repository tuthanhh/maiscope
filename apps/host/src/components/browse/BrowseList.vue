<script setup lang="ts">
import type { RouteLocationRaw } from "vue-router";
import useGameData from "~/composables/useGameData";
import type { Sheet } from "~/types";
import MvCover from "~/components/ui/MvCover.vue";
import MvIcon from "~/components/ui/MvIcon.vue";
import MvLevelPill from "~/components/ui/MvLevelPill.vue";

// The list view of Browse (frontend-redesign 17): one row per sheet, the
// same sheets, order and paging as the grid. Like a card, the whole row is
// one link to the song modal and the bookmark is its own control.
defineOptions({ name: "BrowseList" });

defineProps<{
    sheets: Sheet[];
    linkTo: (sheet: Sheet) => RouteLocationRaw;
    isBookmarked: (sheet: Sheet) => boolean;
    bookmarkLabel: (sheet: Sheet) => string;
    rowLabel: (sheet: Sheet) => string;
}>();
defineEmits<{ bookmark: [sheet: Sheet] }>();

const { getTypeAbbr, getDifficultyName, getDifficultyColor } = useGameData();
</script>

<template>
    <ul class="mv-list">
        <li v-for="s in sheets" :key="s.key" class="mv-list-row">
            <RouterLink :to="linkTo(s)" class="mv-list-link" :aria-label="rowLabel(s)">
                <span class="mv-list-jacket" aria-hidden="true"><MvCover :item="s.song" /></span>
                <span class="mv-list-main">
                    <span class="mv-list-title t-strong">{{ s.song.title }}</span>
                    <span class="mv-list-artist t-body t-muted">{{ s.song.artist }}</span>
                </span>
                <span class="mv-list-sheet t-meta t-muted">
                    {{ getTypeAbbr(s.type) }} · {{ getDifficultyName(s.difficulty) }}
                </span>
                <MvLevelPill
                    class="mv-list-level"
                    :level="s.level"
                    :diff="getDifficultyColor(s.difficulty)"
                />
            </RouterLink>
            <button
                type="button"
                class="mv-list-bookmark"
                :aria-label="bookmarkLabel(s)"
                :aria-pressed="isBookmarked(s)"
                :title="bookmarkLabel(s)"
                @click="$emit('bookmark', s)"
            >
                <MvIcon name="bookmark" :size="15" :filled="isBookmarked(s)" />
            </button>
        </li>
    </ul>
</template>

<style scoped lang="scss">
@use "~/assets/styles/breakpoints" as bp;

.mv-list {
    margin: 0;
    padding: 0;
    list-style: none;
    border-top: var(--line-w) solid var(--line);
}
.mv-list-row {
    display: flex;
    align-items: center;
    border-bottom: var(--line-w) solid var(--line);
    transition: background var(--dur);
}
.mv-list-row:hover {
    background: var(--bg-panel);
}
.mv-list-link {
    flex: 1;
    min-width: 0;
    // 56, not the table row's 44: the 48px jacket and its padding set it.
    min-height: 56px;
    display: grid;
    grid-template-columns: 48px minmax(0, 1fr) 11rem 3.5rem;
    align-items: center;
    gap: var(--s-4);
    padding: var(--s-2) var(--s-2) var(--s-2) 0;
    color: var(--fg);
    text-decoration: none;
}
.mv-list-link:hover .mv-list-title {
    text-decoration: underline;
    text-underline-offset: 3px;
}
.mv-list-jacket {
    display: block;
    width: 48px;
    aspect-ratio: 1;
    border-radius: var(--r);
    overflow: hidden;
    background: var(--bg-sunken);
}
.mv-list-main {
    display: flex;
    flex-direction: column;
    min-width: 0;
}
.mv-list-title {
    font-family: var(--f-display);
}
.mv-list-title,
.mv-list-artist,
.mv-list-sheet {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
}
.mv-list-level {
    justify-self: end;
}
.mv-list-bookmark {
    flex: none;
    width: 44px;
    height: 44px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    border-radius: var(--r);
    background: none;
    color: var(--fg);
    cursor: pointer;
    transition: opacity var(--dur);
}
.mv-list-bookmark:hover {
    opacity: 0.7;
}

@media (max-width: #{bp.$bp-md - 1px}) {
    // Phones: type and difficulty move under the artist; the pill stays right.
    .mv-list-link {
        grid-template-columns: 48px minmax(0, 1fr) auto;
        grid-template-areas:
            "jacket main level"
            "jacket sheet level";
        row-gap: 0;
        column-gap: var(--s-3);
    }
    .mv-list-jacket {
        grid-area: jacket;
    }
    .mv-list-main {
        grid-area: main;
    }
    .mv-list-sheet {
        grid-area: sheet;
    }
    .mv-list-level {
        grid-area: level;
    }
}
</style>
