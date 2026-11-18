import { defineConfig } from "vitest/config";
import { fileURLToPath, URL } from "node:url";
import vue from "@vitejs/plugin-vue";
import yaml from "@modyfi/vite-plugin-yaml";

// Unit tests only: the app's plugins minus the service worker, a DOM from
// happy-dom, and the same `~` alias. Tests sit next to the code they cover
// (`*.test.ts`), as the server's do.
export default defineConfig({
  plugins: [vue(), yaml()],
  resolve: {
    alias: { "~": fileURLToPath(new URL("./src", import.meta.url)) },
  },
  test: {
    environment: "happy-dom",
    include: ["src/**/*.test.ts"],
    restoreMocks: true,
  },
});
