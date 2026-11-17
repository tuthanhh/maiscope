import type { Sheet } from './Sheet';

// A song as the v2 API sends it (contract §0), plus `imageUrl`, which the
// client derives because the asset host is client configuration.
export type Song = {
  /** The minted UUID: the only song key, and what links carry. */
  id: string;

  category?: string;
  title?: string;
  artist?: string;
  bpm?: number;

  imageName?: string;

  version?: string;
  releaseDate?: string;

  isLocked?: boolean;

  comment?: string;

  sheets: Sheet[];
} & {
  // derived on the client (utils/data.ts)
  imageUrl?: string;
};
