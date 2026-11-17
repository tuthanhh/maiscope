import catImageUrl from '~/assets/images/cat.png';
import darkCatImageUrl from '~/assets/images/dark-cat.png';
import type { RegionOverride, Sheet, Song } from '~/types';

function placeholderSheet(song: Omit<Song, 'sheets'>, searchUrl: string): Sheet {
  const owner: Song = { ...song, sheets: [] };
  const sheet: Sheet = {
    type: '??',
    difficulty: '?',
    level: '',
    levelValue: 0,
    hasChart: false,
    song: owner,
    key: `${song.id}/??/?`,
    searchUrl,
  };
  owner.sheets.push(sheet);
  return sheet;
}

/** Shown while the sheet dialog has nothing to show. */
export const NULL_SHEET: Sheet = placeholderSheet(
  {
    id: 'null',
    category: '???',
    title: 'ฅ•ω•ฅ',
    artist: ':3',
    imageUrl: catImageUrl,
    comment: 'Oops! Nothing here :3',
  },
  'https://www.youtube.com/watch?v=W1nifh1OhI8',
);

export const VOID_SHEET: Sheet = placeholderSheet(
  {
    id: 'void',
    category: '???',
    title: 'ฅ-ω-ฅ',
    artist: ';3',
    imageUrl: darkCatImageUrl,
    comment: 'Oops! Nothing here ;3',
  },
  'https://www.youtube.com/watch?v=C9PFVo1FEwU',
);

/** The dialog's stand-ins: not Catalog sheets, so never bookmarked or played. */
export function isPlaceholderSheet(sheet: Sheet): boolean {
  return sheet === NULL_SHEET || sheet === VOID_SHEET;
}

/**
 * The level and designer values a sheet shows in `region`: the region's
 * override where it sets one, the sheet's own value otherwise. v1 swapped in
 * an override object that inherited the rest through the prototype chain;
 * this reads the same values without one.
 */
export function regionalValues(sheet: Sheet, region: string | null): Required<RegionOverride> {
  const override: RegionOverride = (region != null ? sheet.regionOverrides?.[region] : undefined) ?? {};
  return {
    level: override.level ?? sheet.level!,
    levelValue: override.levelValue ?? sheet.levelValue!,
    internalLevel: override.internalLevel ?? sheet.internalLevel!,
    internalLevelValue: override.internalLevelValue ?? sheet.internalLevelValue!,
    noteDesigner: override.noteDesigner ?? sheet.noteDesigner!,
  };
}

export function validateNoteCounts(sheet: Sheet, gameCode: string | null | undefined) {
  if (sheet.noteCounts == null) return true;

  if (gameCode === 'maimai') {
    return (
      sheet.noteCounts.total!
      - sheet.noteCounts.tap!
      - sheet.noteCounts.hold!
      - sheet.noteCounts.slide!
      - sheet.noteCounts.touch!
      - sheet.noteCounts.break!
    ) === 0;
  }
  if (gameCode === 'chunithm') {
    return (
      sheet.noteCounts.total!
      - sheet.noteCounts.tap!
      - sheet.noteCounts.hold!
      - sheet.noteCounts.slide!
      - sheet.noteCounts.air!
      - sheet.noteCounts.flick!
    ) === 0;
  }

  return true;
}
