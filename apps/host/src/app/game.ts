// Single-game configuration. This app targets maimai only; the former
// multi-game routing (/:gameCode) and data/sites.json registry were removed.
export const GAME = {
  gameCode: "maimai",
  gameTitle: "maimai",
  dataSourceUrl: "https://dp4p6x0xfi5o9.cloudfront.net/maimai",
  // Global backend (apps/server). Override per-env via VITE_API_BASE_URL.
  apiBaseUrl:
    import.meta.env.VITE_API_BASE_URL ?? "http://localhost:3000/api/v1",
} as const;
