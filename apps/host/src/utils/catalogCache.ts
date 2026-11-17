// The catalog blob cache: the third client caching layer, under the service
// worker (app shell, wasm, sprites) and the browser's HTTP cache (wire
// responses) — see docs/reference/architecture.md, "Client caching".
//
// What it holds is the raw `/catalog` response *string*, not the parsed
// graph. `preprocessData` turns the 3MB JSON into a 4.7MB object graph with
// cross-references and frozen objects; structured-cloning that into
// IndexedDB is slower than re-parsing the string, and the frozen cross-links
// would not survive the clone anyway. Parse once per launch, from the string.
//
// Every function here swallows IndexedDB failure and behaves as a cache
// miss. Private windows, blocked storage, a WebView that evicted the database
// under pressure, an `open` that is blocked by an old version in another tab
// — none of these may stop the catalog from loading over the network, which
// is what the app did before this layer existed.

// The database is named for the payload shape it holds. v1's database
// ("maiscope") held `/api/v1/catalog` bodies; a v1 body read as v2 would paint
// v1 data into v2 code, and the revision alone cannot tell them apart (a v1
// client running across the v2 deploy stores v1 data under the new revision).
// A new shape gets a new name; the old database is deleted, best effort.
const DB_NAME = "maiscope-catalog-v2";
const RETIRED_DB_NAMES = ["maiscope"];
const DB_VERSION = 1;
const STORE = "catalog";
const KEY = "current";

export type CachedCatalog = {
  /** `/api/sync/manifest` revision the catalog was fetched under. */
  revision: number;
  /** The `/api/catalog` response body, verbatim. */
  raw: string;
  /** ISO timestamp, for a human reading the store in devtools. */
  storedAt: string;
};

function hasIndexedDb(): boolean {
  try {
    return typeof indexedDB !== "undefined" && indexedDB !== null;
  } catch {
    return false;
  }
}

function openDb(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(DB_NAME, DB_VERSION);
    request.onupgradeneeded = () => {
      request.result.createObjectStore(STORE);
    };
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
    request.onblocked = () => reject(new Error("IndexedDB open blocked"));
  });
}

// One transaction over the one store, closed when it settles. Resolves on
// `complete`, not on the request's `success`: a write is not durable until
// the transaction commits.
async function withStore<T>(
  mode: IDBTransactionMode,
  run: (store: IDBObjectStore) => IDBRequest<T>,
): Promise<T> {
  const db = await openDb();
  try {
    return await new Promise<T>((resolve, reject) => {
      const tx = db.transaction(STORE, mode);
      const request = run(tx.objectStore(STORE));
      tx.oncomplete = () => resolve(request.result);
      tx.onerror = () => reject(tx.error);
      tx.onabort = () => reject(tx.error);
    });
  } finally {
    db.close();
  }
}

function isCachedCatalog(value: unknown): value is CachedCatalog {
  if (typeof value !== "object" || value === null) return false;
  const v = value as Record<string, unknown>;
  return typeof v.revision === "number" && typeof v.raw === "string";
}

/** The cached catalog, or null on a miss, a corrupt entry, or no IndexedDB. */
export async function readCatalog(): Promise<CachedCatalog | null> {
  if (!hasIndexedDb()) return null;
  try {
    const value = await withStore("readonly", (store) => store.get(KEY));
    return isCachedCatalog(value) ? value : null;
  } catch {
    return null;
  }
}

/** Store the catalog body under the revision it was fetched for. */
export async function writeCatalog(
  revision: number,
  raw: string,
): Promise<void> {
  if (!hasIndexedDb()) return;
  const entry: CachedCatalog = {
    revision,
    raw,
    storedAt: new Date().toISOString(),
  };
  try {
    await withStore("readwrite", (store) => store.put(entry, KEY));
  } catch {
    // A failed write leaves the previous entry (or none); the next launch
    // fetches again. Nothing to do.
  }
}

/** Delete the databases earlier payload shapes lived in. Best effort. */
export function dropRetiredCaches(): void {
  if (!hasIndexedDb()) return;
  for (const name of RETIRED_DB_NAMES) {
    try {
      indexedDB.deleteDatabase(name);
    } catch {
      // Nothing to do: the old database only costs disk space.
    }
  }
}

/** Drop the entry, e.g. after it failed to parse. */
export async function clearCatalog(): Promise<void> {
  if (!hasIndexedDb()) return;
  try {
    await withStore("readwrite", (store) => store.delete(KEY));
  } catch {
    // Same as writeCatalog: best effort.
  }
}
