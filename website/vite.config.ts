import { defineConfig } from "vite";

// Relative base so the build works from any static host or sub-path
// (Netlify, Vercel, GitHub Pages project sites).
export default defineConfig({
  base: "./",
  build: { target: "es2022", cssMinify: true, assetsInlineLimit: 0 },
});
