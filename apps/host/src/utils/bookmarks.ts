// Bookmarks persist per browser as a list of sheet keys
// (`${songId}/${type}/${difficulty}`, all minted parts), so they survive a
// reload and a Catalog refresh. Before v2 they lived in memory only and were
// lost on reload, so there is nothing older to migrate.
//
// Storage can be blocked or full; a failure here only means bookmarks do not
// outlive the tab, never that the page breaks.
const KEY = "mv:bookmarks";

export function readBookmarkKeys(): string[] {
  try {
    const parsed: unknown = JSON.parse(localStorage.getItem(KEY) ?? "[]");
    return Array.isArray(parsed) ? parsed.filter((k): k is string => typeof k === "string") : [];
  } catch {
    return [];
  }
}

export function writeBookmarkKeys(keys: string[]): void {
  try {
    localStorage.setItem(KEY, JSON.stringify(keys));
  } catch {
    // Not persisted; the in-memory list still works for this session.
  }
}
