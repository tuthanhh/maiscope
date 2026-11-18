import { computed } from 'vue';
import { useDataStore } from '~/stores/data';

// The bookmarks ("My list"). The store owns them: it alone knows the keys a
// loaded Catalog has not resolved yet (stores/data.ts).
export default function useSelectedSheets() {
  const dataStore = useDataStore();

  const selectedSheets = computed(() => dataStore.currentSelectedSheets);

  return {
    selectedSheets,
    isBookmarked: dataStore.hasBookmark,
    toggleSheetSelection: dataStore.toggleBookmark,
  };
}
