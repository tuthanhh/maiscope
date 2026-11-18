import type { Song } from './Song';
import type { Sheet } from './Sheet';

/** The five reference tables (`GET /api/reference`, and inside the Catalog). */
export type Reference = {
  categories: { category: string }[];
  versions: { version: string; abbr?: string }[];
  types: { type: string; name: string; abbr?: string }[];
  /** Keyed by `code`, the form a sheet's `difficulty` carries. */
  difficulties: { code: string; name: string; color?: string }[];
  regions: { region: string; name: string }[];
};

// `GET /api/catalog`, plus the flat sheet list built on the client.
export type Data = Reference & {
  songs: Song[];
} & {
  // added on the client: every sheet, each pointing at its song
  sheets: Sheet[];
};
