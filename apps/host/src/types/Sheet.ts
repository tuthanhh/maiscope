import type { Song } from './Song';

/** A region's partial override of a sheet's level and designer fields. */
export type RegionOverride = {
  level?: string;
  levelValue?: number;
  internalLevel?: string;
  internalLevelValue?: number;
  noteDesigner?: string;
};

// A sheet as the v2 API sends it, with its song as an explicit reference
// instead of v1's prototype chain, and a stable `key` built from minted parts.
export type Sheet = {
  type: string;
  /** `difficulties.code`: `master`, or `協` for a UTAGE sheet. */
  difficulty: string;

  level?: string;
  levelValue?: number;

  internalLevel?: string;
  internalLevelValue?: number;

  noteDesigner?: string;
  noteCounts?: Record<string, number | null>;

  regions?: Record<string, boolean>;
  regionOverrides?: Record<string, RegionOverride>;

  /** A simai chart exists for this sheet. */
  hasChart: boolean;
} & {
  // set on the client (utils/data.ts)
  song: Song;
  /** `${song.id}/${type}/${difficulty}`: the sheet's identity in the client. */
  key: string;

  // placeholder sheets only (utils/sheet.ts)
  searchUrl?: string | null;
};
