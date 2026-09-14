import { defineConfig } from "vite";
import { fileURLToPath, URL } from "node:url";
import vue from "@vitejs/plugin-vue";
import yaml from "@modyfi/vite-plugin-yaml";
import { VitePWA } from "vite-plugin-pwa";

const r = (p: string) => fileURLToPath(new URL(p, import.meta.url));

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [
    vue(),
    yaml(),
    // A service worker is the one artifact that cannot be taken back over the
    // network. Everything below exists to keep a bad deploy recoverable:
    // index.html is never precached, so a broken build is replaced on the next
    // load rather than pinned into installed clients forever.
    VitePWA({
      // Nothing in a read-only browser can be lost by reloading, so the update
      // applies silently instead of earning an "update available" component.
      registerType: "autoUpdate",
      injectRegister: "auto",

      manifest: {
        name: "maiscope",
        short_name: "maiscope",
        description: "Browse maimai songs and visualise their charts.",
        // Hash history keeps the route in the fragment, so every entry point is
        // "/" as far as the manifest and the service worker are concerned.
        start_url: "/",
        scope: "/",
        display: "standalone",
        // The design is deliberately two-tone monochrome (assets/styles/theme.scss),
        // light by default — so the splash and status bar match the light shell.
        background_color: "#ffffff",
        theme_color: "#ffffff",
        icons: [
          {
            src: "/android-chrome-192x192.png",
            sizes: "192x192",
            type: "image/png",
          },
          {
            src: "/android-chrome-512x512.png",
            sizes: "512x512",
            type: "image/png",
          },
          // Separate asset, not the 512 relabelled: Android crops maskable icons
          // to its own shape, so the artwork needs the safe-area padding baked in.
          {
            src: "/maskable-512x512.png",
            sizes: "512x512",
            type: "image/png",
            purpose: "maskable",
          },
        ],
      },

      workbox: {
        // No `html` in the pattern, on purpose: precaching index.html is what
        // makes a bad deploy permanent. It is served by the NetworkFirst
        // navigation rule below instead. No `wasm` either — see globIgnores.
        globPatterns: ["**/*.{js,css,ico,png,svg,webp,woff2}"],
        // Belt and braces. The engine bundle is ~20MB release / 74MB debug, and
        // precaching it would charge every catalog visitor for a page most of
        // them never open.
        //
        // The sprite atlas is excluded for the same reason and is the larger
        // share of it: 105 files, 5.7MB, against a ~180KB app shell. Only the
        // visualizer ever draws them, and once web-delivery 05 gates that page
        // off mobile, phone users would have been paying for artwork they could
        // not reach. Served by the StaleWhileRevalidate rule below instead.
        globIgnores: ["**/*.wasm", "assets/sprites/**"],
        // index.html is not in the precache manifest, so there is nothing for a
        // navigation fallback to point at. The navigation rule below is the
        // whole story.
        navigateFallback: null,

        runtimeCaching: [
          // The API is HTTP's job to cache (Cache-Control) and IndexedDB's to
          // persist (web-delivery 02). Three layers, three lifetimes — this rule
          // exists to state that the service worker is not one of them.
          // Matched by path, not origin: the base URL is build-time injected
          // (VITE_API_BASE_URL), so it is localhost in dev and Fly in production.
          {
            urlPattern: ({ url }) => url.pathname.startsWith("/api/"),
            handler: "NetworkOnly",
          },
          // The escape hatch. A fresh index.html is fetched whenever the network
          // answers within the timeout, so the next deploy reaches installed
          // clients; the cached copy is only a fallback for offline and for a
          // server that has gone quiet.
          {
            urlPattern: ({ request }) => request.mode === "navigate",
            handler: "NetworkFirst",
            options: {
              cacheName: "app-shell",
              networkTimeoutSeconds: 3,
              expiration: { maxEntries: 1 },
              cacheableResponse: { statuses: [200] },
            },
          },
          // Cache-first is safe here only because the filename is content-hashed:
          // a new engine build is a new URL, so a stale entry can never shadow it.
          // Two entries, not one, so a client mid-deploy keeps the build it is
          // already running.
          {
            urlPattern: ({ url }) => url.pathname.endsWith(".wasm"),
            handler: "CacheFirst",
            options: {
              cacheName: "engine-wasm",
              expiration: { maxEntries: 2, maxAgeSeconds: 60 * 60 * 24 * 30 },
              cacheableResponse: { statuses: [0, 200] },
            },
          },
          // Stale-while-revalidate rather than cache-first, because these
          // filenames are NOT content-hashed — they are copied verbatim from
          // public/assets/sprites. Cache-first would pin a sprite change until
          // the entry expired; this serves the cached copy instantly and
          // replaces it in the background, so a deploy still lands.
          {
            urlPattern: ({ url }) => url.pathname.includes("/assets/sprites/"),
            handler: "StaleWhileRevalidate",
            options: {
              cacheName: "engine-sprites",
              expiration: { maxEntries: 150 },
              cacheableResponse: { statuses: [0, 200] },
            },
          },
        ],
      },
    }),
  ],

  resolve: {
    alias: {
      "~": r("./src"),
    },
  },

  server: {
    // Fixed port because it is an allowlisted CORS origin, in apps/server/.env
    // for local dev and in fly.toml for the deployed API. A port shuffle would
    // silently break every cross-origin request.
    port: 1420,
    strictPort: true,
  },
}));
