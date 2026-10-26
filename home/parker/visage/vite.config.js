import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  // The vignettes (vignettes.json, ~1.1 MB, ~70 kB gzipped) are their own
  // chunk, loaded after first paint by idle.js. Big on purpose; the limit
  // sits just above them so anything else that grows that large still warns.
  build: { chunkSizeWarningLimit: 1300 },
});
