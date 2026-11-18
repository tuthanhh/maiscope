import { reactive, ref, watch } from "vue";
import { defineStore } from "pinia";
import type { BrowseForm } from "~/components/browse/BrowseFilters.vue";

export function emptyBrowseForm(): BrowseForm {
  return {
    title: "",
    artist: "",
    designer: "",
    category: "",
    version: "",
    types: [],
    difficulties: [],
    levelMin: "",
    levelMax: "",
    bpmMin: "",
    bpmMax: "",
    region: "",
    useRegionOverride: false,
    matchExactTitle: false,
    matchExactArtist: false,
    useInternalLevel: false,
  };
}

// Browse's filters, page and My-list toggle, outside the page component. The
// page remounts on every visit (no keep-alive), and Play from the song modal
// leaves it: Back must land on the same filters and page, not a fresh grid.
// Per tab and per session; nothing is persisted.
export const useBrowseStore = defineStore("browse", () => {
  const form = reactive(emptyBrowseForm());
  const myListOnly = ref(false);
  const currentPage = ref(1);

  // Grid or list: the one browse setting kept across visits, per browser.
  const VIEW_KEY = "mv:browse-view";
  const view = ref<"grid" | "list">(readView());
  function readView(): "grid" | "list" {
    try {
      return localStorage.getItem(VIEW_KEY) === "list" ? "list" : "grid";
    } catch {
      return "grid";
    }
  }
  watch(view, (value) => {
    try {
      localStorage.setItem(VIEW_KEY, value);
    } catch {
      // Storage refused (private mode, quota): the choice lasts the session.
    }
  });

  return { form, myListOnly, currentPage, view };
});
