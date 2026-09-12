import { defineConfig } from "vite";
import { fileURLToPath, URL } from "node:url";
import vue from "@vitejs/plugin-vue";
import yaml from "@modyfi/vite-plugin-yaml";

const r = (p: string) => fileURLToPath(new URL(p, import.meta.url));

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [vue(), yaml()],

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
