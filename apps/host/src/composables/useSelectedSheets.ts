import { computed } from 'vue';
import { useDataStore } from '~/stores/data';
import type { Sheet } from '~/types';

export default function useSelectedSheets() {
  const dataStore = useDataStore();

  const selectedSheets = computed({
    get() {
      return dataStore.currentSelectedSheets;
    },
    set(value) {
      dataStore.currentSelectedSheets = value;
    },
  });

  function toggleSheetSelection(sheet: Sheet) {
    const newSelectedSheets = selectedSheets.value.slice();

    // By key, not object: a Catalog refresh replaces every sheet object, and a
    // sheet held from before it (an open dialog) must still match.
    const index = newSelectedSheets.findIndex((s) => s.key === sheet.key);
    if (index === -1) {
      newSelectedSheets.push(sheet);
    } else {
      newSelectedSheets.splice(index, 1);
    }

    dataStore.currentSelectedSheets = newSelectedSheets;
  }

  return {
    selectedSheets,
    toggleSheetSelection,
  };
}
