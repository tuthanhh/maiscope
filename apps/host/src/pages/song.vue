<script setup lang="ts">
import { computed } from "vue";
import { useRoute } from "vue-router";
import usePageTitle from "~/composables/usePageTitle";
import { useDataStore } from "~/stores/data";
import useI18n from "~/composables/useI18n";
import useGameData from "~/composables/useGameData";
import useSheetDialog from "~/composables/useSheetDialog";
import LoadingStatus from "~/enums/LoadingStatus";
import { GAME } from "~/app/game";
import { validateNoteCounts, PageNotFoundError } from "~/utils";
import type { Sheet } from "~/types";
import MvIcon from "~/components/ui/MvIcon.vue";

defineOptions({ name: "GameSongPage" });

const { t } = useI18n();
const route = useRoute();
const dataStore = useDataStore();
const { getTypeAbbr, getDifficultyName } = useGameData();
const { viewSheet } = useSheetDialog();

const song = computed(() => {
  const songId = route.query.id;
  const found =
    dataStore.currentData.songs.find((s) => s.songId === songId) ?? null;

  if (
    songId !== undefined &&
    found === null &&
    dataStore.currentLoadingStatus === LoadingStatus.LOADED
  ) {
    // eslint-disable-next-line no-console
    console.warn(new PageNotFoundError().message);
  }
  return found;
});

const noteKeys = ["total", "tap", "hold", "slide", "touch", "break"];
const extraSheetHeaders = computed(() =>
  noteKeys.map((key) => ({
    key,
    title: key !== "total" ? key.toUpperCase() : (t("term.totalNotes") as string),
    get: (sheet: Sheet) => sheet.noteCounts?.[key],
  })),
);

const infoRows = computed(() =>
  song.value == null
    ? []
    : [
        { key: t("term.category"), value: song.value.category },
        { key: t("term.title"), value: song.value.title },
        { key: t("term.artist"), value: song.value.artist },
        { key: t("term.bpm"), value: song.value.bpm },
        {
          key: t("term.releaseDate"),
          value: (song.value.releaseDate ?? "").replaceAll("-", "/"),
        },
        { key: t("term.version"), value: song.value.version },
      ],
);

usePageTitle(() => ({
  title: `${song.value?.title} | ${t("page-title.song")}`,
}));
</script>

<template>
  <div class="mv-root mv-song">
    <router-link :to="{ name: 'songs' }" class="mv-btn mv-back">
      <MvIcon name="back" :size="14" />
      {{ t("ui.goBack") }}
    </router-link>

    <template v-if="song != null">
      <h1 class="mv-song-title">{{ song.title }}</h1>

      <div class="mv-song-head">
        <img
          v-if="song.imageUrl"
          :src="song.imageUrl"
          :alt="song.title"
          class="mv-song-cover"
        />
        <table class="mv-table mv-info">
          <tbody>
            <tr v-for="row in infoRows" :key="row.key">
              <th class="mv-label">{{ row.key }}</th>
              <td>{{ row.value }}</td>
            </tr>
          </tbody>
        </table>
      </div>

      <h2 class="mv-song-subtitle">{{ t("page.songs.sheetData") }}</h2>

      <div class="mv-table-wrap">
        <table class="mv-table mv-sheets">
          <thead>
            <tr>
              <th class="mv-label">{{ t("term.type") }}</th>
              <th class="mv-label">{{ t("term.difficulty") }}</th>
              <th class="mv-label">{{ t("term.level") }}</th>
              <th class="mv-label">{{ t("term.internalLevel") }}</th>
              <th v-for="h in extraSheetHeaders" :key="h.key" class="mv-label">
                {{ h.title }}
              </th>
              <th class="mv-label">{{ t("term.noteDesigner") }}</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(sheet, i) in song.sheets" :key="i">
              <td>{{ getTypeAbbr(sheet.type) }}</td>
              <td>
                <button
                  type="button"
                  class="mv-btn mv-diff-btn"
                  @click="viewSheet(sheet)"
                >
                  {{ getDifficultyName(sheet.difficulty) }}
                </button>
              </td>
              <td>{{ sheet.level }}</td>
              <td>{{ sheet.internalLevel }}</td>
              <td v-for="h in extraSheetHeaders" :key="h.key">
                {{ h.get(sheet) }}
                <span
                  v-if="h.key === 'total' && !validateNoteCounts(sheet, GAME.gameCode)"
                  class="mv-warn"
                  :title="t('description.invalidNoteCounts')"
                >
                  !
                </span>
              </td>
              <td>{{ sheet.noteDesigner }}</td>
            </tr>
          </tbody>
        </table>
      </div>
    </template>
  </div>
</template>

<style scoped>
.mv-song {
  padding: 18px;
}
.mv-back {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  background: none;
  border: 1px solid var(--mv-bd);
  border-radius: 3px;
  color: var(--mv-fg);
  font-family: var(--mv-mono);
  font-size: 11px;
  padding: 7px 12px;
  text-decoration: none;
}
.mv-song-title {
  font-size: 26px;
  font-weight: 700;
  margin: 18px 0;
}
.mv-song-subtitle {
  font-size: 14px;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  margin: 28px 0 12px;
}
.mv-song-head {
  display: flex;
  flex-wrap: wrap;
  gap: 24px;
  align-items: flex-start;
}
.mv-song-cover {
  width: 240px;
  height: 240px;
  object-fit: contain;
  background: var(--mv-jacket-bg);
  border: 1px solid var(--mv-bd);
}
.mv-table {
  border-collapse: collapse;
  font-size: 12px;
}
.mv-table th,
.mv-table td {
  border: 1px solid var(--mv-bd2);
  padding: 7px 12px;
  text-align: left;
}
.mv-info th {
  text-align: left;
}
.mv-table-wrap {
  overflow-x: auto;
}
.mv-sheets th,
.mv-sheets td {
  text-align: center;
  white-space: nowrap;
}
.mv-diff-btn {
  background: none;
  border: none;
  color: var(--mv-fg);
  font-family: var(--mv-mono);
  font-weight: 700;
  font-size: 12px;
  padding: 0;
}
.mv-warn {
  color: var(--mv-fg);
  font-weight: 700;
}
</style>
