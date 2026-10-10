import { defineConfig } from "vite";

export default defineConfig({
  // Served from a sub-path on GitHub Pages: keep every URL relative.
  base: "./",
  server: { fs: { allow: [".."] } },
  build: { target: "es2022" },
});
