import { ref, computed } from "vue";
import { defineStore } from "pinia";
import LoadingStatus from "~/enums/LoadingStatus";
import { GAME } from "~/app/game";
import { buildEmptyData, preprocessData } from "~/utils";
import { fetchJson, fetchText } from "~/utils/api";
import { clearCatalog, readCatalog, writeCatalog } from "~/utils/catalogCache";
import type { Data, Manifest, Sheet } from "~/types";

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

  // Parse a `/catalog` body and make it the current catalog. Shape is
  // byte-identical to the legacy CloudFront data.json, so preprocessData is
  // unchanged; cover images/icons still live on dataSourceUrl. The graph is
  // replaced wholesale (preprocessData freezes it), so selections that point
  // into the old graph are re-resolved by sheetExpr, the cross-tier key.
  function install(raw: string): void {
    const data = JSON.parse(raw) as Data;
    preprocessData(data, GAME.dataSourceUrl, GAME.gameCode);

    const bySheetExpr = new Map(data.sheets.map((s) => [s.sheetExpr, s]));
    currentSelectedSheets.value = currentSelectedSheets.value.flatMap((old) => {
      const sheet = bySheetExpr.get(old.sheetExpr);
      return sheet ? [sheet] : [];
    });

    currentData.value = data;
    currentLoadingStatus.value = LoadingStatus.LOADED;
  }

  // Paint from IndexedDB if it holds a catalog. Returns the revision it was
  // stored under, or null on a miss. An entry that no longer parses is
  // dropped so the next launch does not trip on it again.
  async function paintFromCache(): Promise<number | null> {
    const cached = await readCatalog();
    if (!cached) return null;
    try {
      install(cached.raw);
      return cached.revision;
    } catch {
      await clearCatalog();
      return null;
    }
  }

  // Launch: read cache → paint → probe `/sync/manifest` → refetch only on a
  // revision change → swap (web-delivery 02 stage 2). The network is the
  // source of truth; the cache is the first paint.
  async function loadData() {
    if (currentLoadingStatus.value !== LoadingStatus.PENDING) return;
    currentLoadingStatus.value = LoadingStatus.LOADING;

    const cachedRevision = await paintFromCache();

    try {
      const manifest = await fetchJson<Manifest>(
        `${GAME.apiBaseUrl}/sync/manifest`,
      );
      if (cachedRevision !== null && manifest.revision === cachedRevision) {
        return;
      }

      // `no-cache` makes the browser revalidate rather than serve its own
      // copy: `/catalog` carries `max-age=3600`, and a manifest that says
      // "changed" must not be answered from an HTTP cache that is up to an
      // hour behind it. Unchanged is a 304 and costs nothing.
      const raw = await fetchText(`${GAME.apiBaseUrl}/catalog`, {
        cache: "no-cache",
      });
      install(raw);
      await writeCatalog(manifest.revision, raw);
    } catch (err: any) {
      if (cachedRevision !== null) {
        // Painted from cache already; a dead or slow network is not an
        // error the user needs to see. The cache is served as-is.
        console.warn("catalog refresh failed, showing cached catalog", err);
        return;
      }
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
