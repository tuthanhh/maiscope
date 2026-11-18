import { validateNoteCounts } from '~/utils/sheet';
import type { Data, Sheet, Song } from '~/types';

export function buildEmptyData(): Data {
  return {
    songs: [],
    sheets: [],

    categories: [],
    versions: [],
    types: [],
    difficulties: [],
    regions: [],
  };
}

/** A song's cover on the asset host, or undefined when it has none. */
export function coverUrl(imageName: string | undefined, dataSourceUrl: string): string | undefined {
  return imageName != null ? new URL(imageName, `${dataSourceUrl}/img/cover/`).toString() : undefined;
}

/** The client's identity for a sheet, built from minted parts only. */
export function sheetKey(songId: string, type: string, difficulty: string): string {
  return `${songId}/${type}/${difficulty}`;
}

/**
 * Point a sheet at its song and give it its key. Shared by the Catalog and
 * by the single-sheet fetch, the two places a wire sheet becomes a client one.
 */
export function attachSheet(sheet: Omit<Sheet, 'song' | 'key'>, song: Song): Sheet {
  const attached = sheet as Sheet;
  attached.song = song;
  attached.key = sheetKey(song.id, sheet.type, sheet.difficulty);
  return attached;
}

/**
 * A song as `/api/songs/{id}` or a sheet's `song` sends it, made a client
 * song in place: its cover URL, and each sheet pointing back at it.
 */
export function attachSong(song: Song, dataSourceUrl: string): Song {
  song.imageUrl = coverUrl(song.imageName, dataSourceUrl);
  for (const sheet of song.sheets) attachSheet(sheet, song);
  return song;
}

// Build the client graph from a `/api/catalog` body, in place: each song gets
// its cover URL, each sheet an explicit `song` and its `key`, and the flat
// `sheets` list is assembled newest song first. Then freeze it: the graph is
// replaced wholesale on a refresh, never edited.
export function buildCatalog(data: Data, dataSourceUrl: string, gameCode: string): void {
  for (const song of data.songs) {
    song.imageUrl = coverUrl(song.imageName, dataSourceUrl);
    for (const sheet of song.sheets) {
      attachSheet(sheet, song);
      if (!validateNoteCounts(sheet, gameCode)) {
        // eslint-disable-next-line no-console
        console.warn('Invalid note counts:', sheet.key, sheet.noteCounts);
      }
    }
  }

  data.songs.reverse();
  // eslint-disable-next-line no-param-reassign
  data.sheets = data.songs.flatMap((song) => song.sheets);
}

export function freezeCatalog(data: Data): void {
  for (const song of data.songs) {
    for (const sheet of song.sheets) {
      for (const regionOverride of Object.values(sheet.regionOverrides ?? {})) {
        Object.freeze(regionOverride);
      }
      Object.freeze(sheet.noteCounts);
      Object.freeze(sheet.regions);
      Object.freeze(sheet.regionOverrides);
      Object.freeze(sheet);
    }
    Object.freeze(song.sheets);
    Object.freeze(song);
  }

  for (const list of [data.categories, data.versions, data.types, data.difficulties, data.regions]) {
    for (const entry of list) Object.freeze(entry);
    Object.freeze(list);
  }
  Object.freeze(data.songs);
  Object.freeze(data.sheets);
  Object.freeze(data);
}

export function preprocessData(data: Data, dataSourceUrl: string, gameCode: string): void {
  buildCatalog(data, dataSourceUrl, gameCode);
  freezeCatalog(data);
}
