import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

// See doc 10 (Repository & Build) and doc 50 (Frontend & Mobile-First).
// `vite build` output (dist/) is embedded into the autotim binary via
// rust-embed; Axum serves it with SPA fallback.
export default defineConfig({
  plugins: [vue()],
  server: {
    proxy: {
      "/api": "http://localhost:8080",
    },
  },
  build: {
    outDir: "dist",
    emptyOutDir: true,
  },
});
