import { $canonicalSheet, computeSheetExpr, validateNoteCounts } from '~/utils/sheet';
import type { Data, Sheet } from '~/types';

export function buildEmptyData(): Data {
  return {
    songs: [],
    sheets: [],

    categories: [],

    versions: [],

    types: [],
    difficulties: [],

    regions: [],

    // mark the data as empty
    updateTime: '0000-00-00',
  };
}

function resolveUrl(filePath: string | undefined, baseUrl: string) {
  return filePath != null ? new URL(filePath, baseUrl).toString() : filePath;
}

function computeNotePercentages(noteCounts: Record<string, number | null> | undefined) {
  return noteCounts != null ? Object.fromEntries(
    Object.entries(noteCounts)
      .map(([key, value]) => [
        key,
        value != null && noteCounts.total != null ? Number(value) / noteCounts.total : null,
      ]),
  ) : noteCounts;
}

// Per-sheet client-derived fields (contract: server sends raw fields only).
// Used both by buildCatalog (full nested catalog) and by the search flow
// (flat GET /sheets/search results) — the two places a raw server Sheet
// becomes a display-ready Sheet.
export function decorateSheetFields(sheet: Sheet, dataSourceUrl: string): void {
  sheet[$canonicalSheet] = sheet;
  sheet.imageUrl = resolveUrl(sheet.imageName, `${dataSourceUrl}/img/cover/`);
  sheet.imageUrlM = resolveUrl(sheet.imageName, `${dataSourceUrl}/img/cover-m/`);
  sheet.sheetExpr = computeSheetExpr(sheet);
  sheet.notePercents = computeNotePercentages(sheet.noteCounts);
}

// Build the full catalog shape: computed fields, prototype-linked sheets,
// final song/sheet ordering. Does NOT freeze — see freezeCatalog. Mutates
// `data` in place (and returns nothing) so existing callers that relied on
// preprocessData's in-place mutation keep working unchanged.
export function buildCatalog(data: Data, dataSourceUrl: string, gameCode: string): void {
  let lastSongNo = 0;
  for (const song of data.songs) {
    lastSongNo += 1;
    song.songNo = lastSongNo;
    song.imageUrl = resolveUrl(song.imageName, `${dataSourceUrl}/img/cover/`);
    song.imageUrlM = resolveUrl(song.imageName, `${dataSourceUrl}/img/cover-m/`);

    for (const sheet of song.sheets) {
      Object.setPrototypeOf(sheet, song);

      decorateSheetFields(sheet, dataSourceUrl);

      if (!validateNoteCounts(sheet, gameCode)) {
        // eslint-disable-next-line no-console
        console.warn('Invalid note counts:', sheet.sheetExpr, sheet.noteCounts);
      }

      for (const regionOverride of Object.values(sheet.regionOverrides ?? {})) {
        Object.setPrototypeOf(regionOverride, sheet);
      }
    }
  }

  data.songs.reverse();

  // eslint-disable-next-line no-param-reassign
  data.sheets = data.songs.flatMap((song) => song.sheets);

  for (const type of data.types) {
    type.iconUrl = resolveUrl(type.iconUrl, `${dataSourceUrl}/img/`);
  }
  for (const difficulty of data.difficulties) {
    difficulty.iconUrl = resolveUrl(difficulty.iconUrl!, `${dataSourceUrl}/img/`);
  }
}

// Freeze the shape buildCatalog produced. Separate pass so buildCatalog's
// output can be asserted on directly (e.g. in a future test) without hitting
// frozen-object write errors, and so a future new computed field can never
// silently land after its object's freeze call (there is no such call to
// land after — freezing only happens here, once, at the end).
export function freezeCatalog(data: Data): void {
  for (const song of data.songs) {
    for (const sheet of song.sheets) {
      for (const regionOverride of Object.values(sheet.regionOverrides ?? {})) {
        Object.freeze(regionOverride);
      }
      Object.freeze(sheet.noteCounts);
      Object.freeze(sheet.notePercents);
      Object.freeze(sheet.regions);
      Object.freeze(sheet.regionOverrides);
      Object.freeze(sheet);
    }
    Object.freeze(song.sheets);
    Object.freeze(song);
  }

  for (const category of data.categories) Object.freeze(category);
  for (const version of data.versions) Object.freeze(version);
  for (const type of data.types) Object.freeze(type);
  for (const difficulty of data.difficulties) Object.freeze(difficulty);
  for (const region of data.regions) Object.freeze(region);

  Object.freeze(data.songs);
  Object.freeze(data.sheets);
  Object.freeze(data.categories);
  Object.freeze(data.versions);
  Object.freeze(data.types);
  Object.freeze(data.difficulties);
  Object.freeze(data.regions);

  Object.freeze(data);
}

export function preprocessData(data: Data, dataSourceUrl: string, gameCode: string): void {
  buildCatalog(data, dataSourceUrl, gameCode);
  freezeCatalog(data);
}
