import { regionalValues } from "~/utils/sheet";
import type { I18nFacade } from "~/app/i18n";
import type { Data, Sheet, Filters, FilterOptions } from "~/types";

export function buildEmptyFilters(): Filters {
  return {
    categories: [],
    title: null,
    matchExactTitle: null,
    artist: null,
    matchExactArtist: null,

    versions: [],
    minBPM: null,
    maxBPM: null,

    types: [],
    difficulties: [],
    minLevelValue: null,
    maxLevelValue: null,
    useInternalLevel: null,

    noteDesigners: [],
    region: null,
    useRegionOverride: null,
  };
}

export function buildEmptyFilterOptions(): FilterOptions {
  return {
    categories: [],
    versions: [],
    levels: [],
    noteDesigners: [],
    regions: [],
  };
}

export function buildFilterOptions(
  data: Data,
  i18n: I18nFacade,
): FilterOptions {
  if (data.sheets.length === 0) {
    // return empty array instead of null to preserve all filters ui before the data is loaded
    return buildEmptyFilterOptions();
  }

  const nonEmptyOrNull = <T>(arr: T[]) => (arr.length !== 0 ? arr : null);

  return {
    categories: nonEmptyOrNull(
      data.categories.map(({ category }) => ({
        $type: "option",
        text: category ?? "N/A",
        value: category,
      })),
    ),

    versions: nonEmptyOrNull(
      data.versions.toReversed().map(({ version, abbr }) => ({
        $type: "option",
        text: abbr ?? version ?? "N/A",
        value: version,
      })),
    ),

    levels: nonEmptyOrNull(
      [
        ...new Map(
          data.sheets
            .filter(
              (sheet) =>
                sheet.levelValue != null &&
                sheet.level != null &&
                !sheet.level.endsWith("?"),
            )
            .map((sheet) => [sheet.levelValue!, sheet.level!]),
        ).entries(),
      ]
        .sort(([aLevelValue], [bLevelValue]) => aLevelValue - bLevelValue)
        .map(([levelValue, level]) => ({
          $type: "option",
          text: level,
          value: levelValue,
        })),
    ),

    noteDesigners: nonEmptyOrNull(
      [
        ...Map.groupBy(
          data.songs
            .toReversed()
            .flatMap((song) => song.sheets)
            .filter((sheet) => sheet.noteDesigner != null),
          (sheet) => sheet.noteDesigner!,
        ).entries(),
      ]
        .map(([noteDesigner, sheets]) => ({
          noteDesigner,
          sheetCount: sheets.length,
        }))
        .sort((a, b) => b.sheetCount - a.sheetCount)
        .map(({ noteDesigner, sheetCount }) => ({
          $type: "option",
          text: `${noteDesigner} (${sheetCount})`,
          value: noteDesigner,
        })),
    ),
    regions: nonEmptyOrNull(
      data.regions.flatMap(({ region, name }) => [
        {
          $type: "option",
          text: name,
          value: `${region}`,
        },
        {
          $type: "option",
          text: i18n.t("description.unavailableInRegion", {
            region: name,
          }) as string,
          value: `!${region}`,
        },
      ]),
    ),
  };
}

export function filterSheets(sheets: Sheet[], filters: Filters) {
  // With "use region override" on and a region selected, the level and
  // designer checks read that region's values.
  const overrideRegion =
    filters.useRegionOverride &&
    filters.region != null &&
    !filters.region.startsWith("!")
      ? filters.region
      : null;
  const values = (sheet: Sheet) => regionalValues(sheet, overrideRegion);

  // Collect the active predicates, then apply them in a single pass instead of
  // allocating a fresh array for every individual filter.
  const predicates: ((sheet: Sheet) => boolean)[] = [];

  if (filters.region != null) {
    if (filters.region.startsWith("!")) {
      const excludedRegion = filters.region.replace(/^!/, "");
      predicates.push(
        (sheet) => sheet.regions == null || !sheet.regions[excludedRegion],
      );
    } else {
      const includedRegion = filters.region;
      predicates.push(
        (sheet) => sheet.regions != null && sheet.regions[includedRegion],
      );
    }
  }
  if (filters.categories.length !== 0) {
    const categorySet = new Set(filters.categories);
    predicates.push(({ song }) =>
      categorySet.has(song.category!) ||
      (song.category != null &&
        song.category.split("|").some((category) => categorySet.has(category))),
    );
  }
  if (filters.title != null) {
    if (filters.matchExactTitle) {
      const exactTitle = filters.title;
      predicates.push(({ song }) => song.title === exactTitle);
    } else {
      const normalizedTitle = filters.title.toLowerCase();
      predicates.push(
        ({ song }) =>
          song.title != null && song.title.toLowerCase().includes(normalizedTitle),
      );
    }
  }
  if (filters.versions.length !== 0) {
    const versionSet = new Set(filters.versions);
    predicates.push(({ song }) => versionSet.has(song.version!));
  }
  if (filters.types.length !== 0) {
    const typeSet = new Set(filters.types);
    predicates.push((sheet) => typeSet.has(sheet.type));
  }
  if (filters.difficulties.length !== 0) {
    const difficultySet = new Set(filters.difficulties);
    predicates.push((sheet) => difficultySet.has(sheet.difficulty));
  }
  const levelOf = (sheet: Sheet) =>
    filters.useInternalLevel ? values(sheet).internalLevelValue : values(sheet).levelValue;
  if (typeof filters.minLevelValue === "number") {
    const minLevelValue = filters.minLevelValue;
    predicates.push((sheet) => {
      const level = levelOf(sheet);
      return level != null && level >= minLevelValue;
    });
  }
  if (typeof filters.maxLevelValue === "number") {
    const maxLevelValue = filters.maxLevelValue;
    predicates.push((sheet) => {
      const level = levelOf(sheet);
      return level != null && level <= maxLevelValue;
    });
  }
  if (typeof filters.minBPM === "number") {
    const minBPM = filters.minBPM;
    predicates.push(({ song }) => song.bpm != null && song.bpm >= minBPM);
  }
  if (typeof filters.maxBPM === "number") {
    const maxBPM = filters.maxBPM;
    predicates.push(({ song }) => song.bpm != null && song.bpm <= maxBPM);
  }
  if (filters.artist != null) {
    if (filters.matchExactArtist) {
      const exactArtist = filters.artist;
      predicates.push(({ song }) => song.artist === exactArtist);
    } else {
      const normalizedArtist = filters.artist.toLowerCase();
      predicates.push(
        ({ song }) =>
          song.artist != null && song.artist.toLowerCase().includes(normalizedArtist),
      );
    }
  }
  if (filters.noteDesigners.length !== 0) {
    const noteDesignerSet = new Set(filters.noteDesigners);
    predicates.push((sheet) => noteDesignerSet.has(values(sheet).noteDesigner!));
  }

  return predicates.length !== 0
    ? sheets.filter((sheet) => predicates.every((p) => p(sheet)))
    : sheets.slice();
}
