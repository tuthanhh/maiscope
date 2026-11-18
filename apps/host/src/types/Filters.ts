export type Filters = {
  categories: string[];
  title: string | null;
  matchExactTitle: boolean | null;
  artist: string | null;
  matchExactArtist: boolean | null;

  versions: string[];
  minBPM: number | null;
  maxBPM: number | null;

  types: string[];
  difficulties: string[];
  minLevelValue: number | null;
  maxLevelValue: number | null;
  useInternalLevel: boolean | null;

  noteDesigners: string[];
  region: string | null;
  useRegionOverride: boolean | null;
};
