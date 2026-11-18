import { storeToRefs } from 'pinia';
import { useDataStore } from '~/stores/data';

export default function useGameData() {
  const dataStore = useDataStore();
  // Lookup maps live on the store (built once per data load). This composable
  // only exposes the read helpers around them.
  const {
    versionMap,
    typeMap,
    difficultyMap,
  } = storeToRefs(dataStore);

  // Version
  function getVersionData(version: string | undefined) {
    return versionMap.value.get(version!);
  }
  function getVersionAbbr(version: string | undefined) {
    return getVersionData(version)?.abbr ?? version;
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

  return {
    getVersionAbbr,
    getTypeName,
    getTypeAbbr,
    getDifficultyName,
    getDifficultyColor,
  };
}
