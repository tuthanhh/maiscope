import { ref, computed, watch } from "vue";
import { defineStore } from "pinia";
import LoadingStatus from "~/enums/LoadingStatus";
import { GAME } from "~/app/game";
import { buildEmptyData, preprocessData } from "~/utils";
import { fetchJson, fetchText } from "~/utils/api";
import {
  clearCatalog,
  dropRetiredCaches,
  readCatalog,
  writeCatalog,
} from "~/utils/catalogCache";
import { readBookmarkKeys, writeBookmarkKeys } from "~/utils/bookmarks";
import type { Data, Manifest, Sheet } from "~/types";

// eslint-disable-next-line import/prefer-default-export
export const useDataStore = defineStore("data", () => {
  const currentData = ref<Data>(buildEmptyData());
  const currentLoadingStatus = ref<LoadingStatus>(LoadingStatus.PENDING);
  const currentLoadingErrorMessage = ref("");
  const currentSelectedSheets = ref<Sheet[]>([]);
  // Bookmark keys no loaded Catalog has matched yet: the persisted list before
  // the first install, then whatever an install could not resolve — a sheet
  // missing from a stale cached body, or hidden upstream for now (ADR-0026).
  // They stay persisted, so a later Catalog that has them restores them.
  let unresolvedBookmarkKeys = readBookmarkKeys();
  function persistBookmarks(): void {
    writeBookmarkKeys([
      ...currentSelectedSheets.value.map((s) => s.key),
      ...unresolvedBookmarkKeys,
    ]);
  }
  watch(currentSelectedSheets, () => {
    if (currentLoadingStatus.value === LoadingStatus.LOADED) persistBookmarks();
  });

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
    () => new Map(currentData.value.difficulties.map((e) => [e.code, e])),
  );
  const difficultyIndexMap = computed(
    () => new Map(currentData.value.difficulties.map((e, i) => [e.code, i])),
  );

  // Parse a `/api/catalog` body and make it the current catalog; covers
  // still live on dataSourceUrl. The graph is replaced wholesale
  // (preprocessData freezes it), so bookmarks that point into the old graph
  // — or were only persisted keys — are re-resolved by sheet key. A key this
  // Catalog lacks is kept aside and stays persisted, not dropped.
  function install(raw: string): void {
    const data = JSON.parse(raw) as Data;
    preprocessData(data, GAME.dataSourceUrl, GAME.gameCode);

    const byKey = new Map(data.sheets.map((s) => [s.key, s]));
    const keys = [
      ...currentSelectedSheets.value.map((s) => s.key),
      ...unresolvedBookmarkKeys,
    ];
    const resolved: Sheet[] = [];
    unresolvedBookmarkKeys = [];
    for (const key of keys) {
      const sheet = byKey.get(key);
      if (sheet) resolved.push(sheet);
      else unresolvedBookmarkKeys.push(key);
    }

    currentData.value = data;
    currentLoadingStatus.value = LoadingStatus.LOADED;
    currentSelectedSheets.value = resolved;
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

  // Launch: read cache → paint → probe `/api/sync/manifest` → refetch only on a
  // revision change → swap (web-delivery 02 stage 2). The network is the
  // source of truth; the cache is the first paint.
  async function loadData() {
    // From ERROR too: the browse error state retries through here.
    const status = currentLoadingStatus.value;
    if (status !== LoadingStatus.PENDING && status !== LoadingStatus.ERROR) return;
    currentLoadingStatus.value = LoadingStatus.LOADING;

    dropRetiredCaches();
    const cachedRevision = await paintFromCache();

    try {
      const manifest = await fetchJson<Manifest>(
        `${GAME.apiBaseUrl}/sync/manifest`,
      );
      if (cachedRevision !== null && manifest.revision === cachedRevision) {
        return;
      }

      // `no-cache` makes the browser revalidate rather than serve its own
      // copy: `/api/catalog` carries `max-age=3600`, and a manifest that says
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
