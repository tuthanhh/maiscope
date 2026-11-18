import { storeToRefs } from 'pinia';
import { useDataStore } from '~/stores/data';

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
  function getDifficultyIndex(difficulty: string | undefined) {
    return difficultyIndexMap.value.get(difficulty!) ?? -1;
  }


  return {
    getCategoryIndex,
    getVersionAbbr,
    getVersionIndex,
    getTypeName,
    getTypeAbbr,
    getTypeIndex,
    getDifficultyName,
    getDifficultyColor,
    getDifficultyIndex,
  };
}
