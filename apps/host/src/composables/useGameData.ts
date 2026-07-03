import { storeToRefs } from 'pinia';
import { useDataStore } from '~/stores/data';
import { GAME } from '~/app/game';
import type { Sheet } from '~/types';

export default function useGameData() {
  const dataStore = useDataStore();
  // Lookup maps live on the store (built once per data load). This composable
  // only exposes the read helpers around them.
  const {
    categoryIndexMap,
    versionMap,
    versionIndexMap,
    typeMap,
    typeIndexMap,
    difficultyMap,
    difficultyIndexMap,
  } = storeToRefs(dataStore);

  // Lock icon
  function getLockedIconUrl() {
    return new URL('locked.png', `${GAME.dataSourceUrl}/img/`).toString();
  }
  function getLockedIconHeight() {
    return 40;
  }

  // Category
  function getCategoryIndex(category: string | undefined) {
    return categoryIndexMap.value.get(category!) ?? -1;
  }

  // Version
  function getVersionData(version: string | undefined) {
    return versionMap.value.get(version!);
  }
  function getVersionAbbr(version: string | undefined) {
    return getVersionData(version)?.abbr ?? version;
  }
  function getVersionIndex(version: string | undefined) {
    return versionIndexMap.value.get(version!) ?? -1;
  }

  // Type
  function getTypeData(type: string | undefined) {
    return typeMap.value.get(type!);
  }
  function getTypeName(type: string | undefined) {
    return getTypeData(type)?.name ?? String(type).toUpperCase();
  }
  function getTypeAbbr(type: string | undefined) {
    return getTypeData(type)?.abbr ?? String(type).toUpperCase();
  }
  function getTypeIconUrl(type: string | undefined) {
    return getTypeData(type)?.iconUrl ?? undefined;
  }
  function getTypeIconHeight(type: string | undefined) {
    return getTypeData(type)?.iconHeight ?? undefined;
  }
  function getTypeIndex(type: string | undefined) {
    return typeIndexMap.value.get(type!) ?? -1;
  }

  // Difficulty
  function getDifficultyData(difficulty: string | undefined) {
    return difficultyMap.value.get(difficulty!);
  }
  function getDifficultyName(difficulty: string | undefined) {
    return getDifficultyData(difficulty)?.name ?? String(difficulty).toUpperCase();
  }
  function getDifficultyColor(difficulty: string | undefined) {
    return getDifficultyData(difficulty)?.color ?? 'unset';
  }
  function getDifficultyIconUrl(difficulty: string | undefined) {
    return getDifficultyData(difficulty)?.iconUrl ?? undefined;
  }
  function getDifficultyIconHeight(difficulty: string | undefined) {
    return getDifficultyData(difficulty)?.iconHeight ?? undefined;
  }
  function getDifficultyIndex(difficulty: string | undefined) {
    return difficultyIndexMap.value.get(difficulty!) ?? -1;
  }

  // Search link
  function getSheetSearchLinkIcon(sheet: Sheet) {
    if (sheet.searchUrl === null) return null;
    if (sheet.searchUrl === undefined) return 'mdi-youtube';
    if (sheet.searchUrl.includes('https://www.youtube.com/')) return 'mdi-youtube';

    return 'mdi-link-variant';
  }
  function getSheetSearchLinkColor(sheet: Sheet) {
    if (sheet.searchUrl === null) return null;
    if (sheet.searchUrl === undefined) return 'red';
    if (sheet.searchUrl.includes('https://www.youtube.com/')) return 'red';

    return 'primary';
  }
  function getSheetSearchLink(sheet: Sheet) {
    if (sheet.searchUrl === null) return null;
    if (sheet.searchUrl !== undefined) return sheet.searchUrl;

    const escapedGameTitle = GAME.gameTitle;
    const escapedTitle = sheet.title ?? '';
    const escapedDifficulty = getDifficultyName(sheet.difficulty);

    const url = new URL('https://www.youtube.com/results');
    url.searchParams.set(
      'search_query',
      `${escapedGameTitle} ${escapedTitle} ${escapedDifficulty}`,
    );

    return url.toString();
  }

  return {
    getLockedIconUrl,
    getLockedIconHeight,
    getCategoryIndex,
    getVersionAbbr,
    getVersionIndex,
    getTypeName,
    getTypeAbbr,
    getTypeIconUrl,
    getTypeIconHeight,
    getTypeIndex,
    getDifficultyName,
    getDifficultyColor,
    getDifficultyIconUrl,
    getDifficultyIconHeight,
    getDifficultyIndex,
    getSheetSearchLinkIcon,
    getSheetSearchLinkColor,
    getSheetSearchLink,
  };
}
