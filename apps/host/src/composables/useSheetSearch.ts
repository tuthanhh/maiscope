import { ref } from "vue";
import { useDebounceFn } from "@vueuse/core";
import { GAME } from "~/app/game";
import { decorateSheetFields } from "~/utils/data";
import type { Filters, Sheet } from "~/types";

type SearchResponse = { sheets: Sheet[]; total: number };

function buildSearchUrl(filters: Filters, page: number, pageSize: number): string {
  const params = new URLSearchParams();
  if (filters.title) params.set("title", filters.title);
  if (filters.matchExactTitle) params.set("matchExactTitle", "true");
  if (filters.artist) params.set("artist", filters.artist);
  if (filters.matchExactArtist) params.set("matchExactArtist", "true");
  for (const c of filters.categories) params.append("categories", c);
  for (const v of filters.versions) params.append("versions", v);
  for (const ty of filters.types) params.append("types", ty);
  for (const d of filters.difficulties) params.append("difficulties", d);
  if (typeof filters.minLevelValue === "number") params.set("minLevelValue", String(filters.minLevelValue));
  if (typeof filters.maxLevelValue === "number") params.set("maxLevelValue", String(filters.maxLevelValue));
  if (filters.useInternalLevel) params.set("useInternalLevel", "true");
  if (typeof filters.minBPM === "number") params.set("minBPM", String(filters.minBPM));
  if (typeof filters.maxBPM === "number") params.set("maxBPM", String(filters.maxBPM));
  for (const nd of filters.noteDesigners) params.append("noteDesigners", nd);
  if (filters.region) params.set("region", filters.region);
  if (filters.useRegionOverride) params.set("useRegionOverride", "true");
  params.set("page", String(page));
  params.set("pageSize", String(pageSize));
  return `${GAME.apiBaseUrl}/sheets/search?${params.toString()}`;
}

export default function useSheetSearch() {
  const results = ref<Sheet[]>([]);
  const total = ref(0);
  const loading = ref(false);

  async function runSearch(filters: Filters, page: number, pageSize: number) {
    loading.value = true;
    try {
      const url = buildSearchUrl(filters, page, pageSize);
      const data: SearchResponse = await (await fetch(url)).json();

      for (const sheet of data.sheets) {
        decorateSheetFields(sheet, GAME.dataSourceUrl);
      }
      results.value = data.sheets;
      total.value = data.total;
    } finally {
      loading.value = false;
    }
  }

  const search = useDebounceFn(runSearch, 300);

  return { results, total, loading, search };
}
