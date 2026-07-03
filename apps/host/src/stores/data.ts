import { ref, computed } from "vue";
import { defineStore } from "pinia";
import LoadingStatus from "~/enums/LoadingStatus";
import { GAME } from "~/app/game";
import { buildEmptyData, preprocessData } from "~/utils";
import type { Data, Sheet } from "~/types";
import { invoke, isTauri } from "@tauri-apps/api/core";

// eslint-disable-next-line import/prefer-default-export
export const useDataStore = defineStore("data", () => {
  const currentData = ref<Data>(buildEmptyData());
  const currentLoadingStatus = ref<LoadingStatus>(LoadingStatus.PENDING);
  const currentLoadingErrorMessage = ref("");
  const currentSelectedSheets = ref<Sheet[]>([]);

  // Lookup maps for the loaded data. Computed once per load and shared across
  // all consumers (rather than rebuilt inside every useGameData() call).
  const categoryIndexMap = computed(
    () => new Map(currentData.value.categories.map((e, i) => [e.category, i])),
  );
  const versionMap = computed(
    () => new Map(currentData.value.versions.map((e) => [e.version, e])),
  );
  const versionIndexMap = computed(
    () => new Map(currentData.value.versions.map((e, i) => [e.version, i])),
  );
  const typeMap = computed(
    () => new Map(currentData.value.types.map((e) => [e.type, e])),
  );
  const typeIndexMap = computed(
    () => new Map(currentData.value.types.map((e, i) => [e.type, i])),
  );
  const difficultyMap = computed(
    () => new Map(currentData.value.difficulties.map((e) => [e.difficulty, e])),
  );
  const difficultyIndexMap = computed(
    () =>
      new Map(currentData.value.difficulties.map((e, i) => [e.difficulty, i])),
  );

  async function loadData() {
    if (currentLoadingStatus.value !== LoadingStatus.PENDING) return;

    try {
      currentLoadingStatus.value = LoadingStatus.LOADING;

      // Catalog now comes from the global backend (apps/server), not the
      // legacy CloudFront data.json. Shape is byte-identical, so preprocessData
      // is unchanged. Cover images/icons still live on dataSourceUrl.
      const catalogUrl = `${GAME.apiBaseUrl}/catalog`;
      const data = isTauri()
        ? await invoke<Data>("load_chart_data", { catalogUrl })
        : await (await fetch(catalogUrl)).json();

      preprocessData(data, GAME.dataSourceUrl, GAME.gameCode);

      currentData.value = data;
      currentLoadingStatus.value = LoadingStatus.LOADED;
    } catch (err: any) {
      currentLoadingErrorMessage.value = err.message;
      currentLoadingStatus.value = LoadingStatus.ERROR;
    }
  }

  return {
    currentData,
    currentLoadingStatus,
    currentLoadingErrorMessage,
    currentSelectedSheets,
    categoryIndexMap,
    versionMap,
    versionIndexMap,
    typeMap,
    typeIndexMap,
    difficultyMap,
    difficultyIndexMap,
    loadData,
  };
});
