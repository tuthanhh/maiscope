import { ref, computed } from 'vue';
import { NULL_SHEET } from '~/utils';
import type { Sheet } from '~/types';
import useItemDrawer from './useItemDrawer';

const isOpened = ref(false);
const isDrawMode = ref(false);
const drawingPool = ref<Sheet[]>([]);

const {
  currentItems,
  isDrawing,
  setCurrentItems,
  startDrawing,
  stopDrawing,
} = useItemDrawer<Sheet>({
  drawingPool,
  drawSize: 1,
});

// The drawer hands its items back as `readonly`; Catalog sheets are frozen at
// runtime already, and `DeepReadonly` cannot follow the sheet → song → sheets
// cycle, so the plain type is the accurate one here.
const currentSheet = computed(() => (currentItems.value[0] as Sheet | undefined) ?? NULL_SHEET);
const isStatic = computed(() => !isDrawing.value);

function viewSheet(sheet: Sheet) {
  setCurrentItems([sheet]);
  isDrawMode.value = false;
  isOpened.value = true;
}

async function startDrawingSheet(onFinish?: (resultItems: typeof currentItems.value) => void) {
  isDrawMode.value = true;
  isOpened.value = true;

  await startDrawing(onFinish);
}

async function stopDrawingSheet() {
  await stopDrawing();
}

export default function useSheetDialog() {
  return {
    currentSheet,
    isOpened,
    isDrawMode,
    isStatic,
    drawingPool,
    viewSheet,
    startDrawingSheet,
    stopDrawingSheet,
  };
}
