import type { RegionOverride, Sheet } from '~/types';

/**
 * The level and designer values a sheet shows in `region`: the region's
 * override where it sets one, the sheet's own value otherwise. v1 swapped in
 * an override object that inherited the rest through the prototype chain;
 * this reads the same values without one.
 */
export function regionalValues(sheet: Sheet, region: string | null): RegionOverride {
  const override: RegionOverride = (region != null ? sheet.regionOverrides?.[region] : undefined) ?? {};
  return {
    level: override.level ?? sheet.level,
    levelValue: override.levelValue ?? sheet.levelValue,
    internalLevel: override.internalLevel ?? sheet.internalLevel,
    internalLevelValue: override.internalLevelValue ?? sheet.internalLevelValue,
    noteDesigner: override.noteDesigner ?? sheet.noteDesigner,
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

  return true;
}
