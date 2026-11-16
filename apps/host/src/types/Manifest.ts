// `GET /sync/manifest` — docs/reference/api-contract.md §3. The cheap
// freshness probe: one small object, `Cache-Control: no-cache`, so every
// launch asks the server whether the catalog it holds is still current.
export type Manifest = {
  updateTime: string;
  /** Monotonic; 0 before the first catalog sync. */
  revision: number;
  /** The `ETag` of `GET /catalog`. */
  catalogHash: string;
  counts: { songs: number; sheets: number; charts: number };
};
