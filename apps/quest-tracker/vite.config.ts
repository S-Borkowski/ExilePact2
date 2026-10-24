/// <reference types="vitest/config" />
import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

// The Vite root is this folder; `vite build` writes `dist/`, which
// `src-tauri/tauri.conf.json` embeds as `frontendDist`.
export default defineConfig({
  plugins: [svelte()],
  // Keep Rust errors visible when run under `tauri dev`.
  clearScreen: false,
  server: {
    // The port `devUrl` in tauri.conf.json names.
    port: 1420,
    strictPort: true,
    host: '127.0.0.1',
    watch: {
      ignored: ['**/src-tauri/**', '**/crates/**', '**/target/**'],
    },
  },
  test: {
    environment: 'jsdom',
    include: ['src/**/*.test.ts'],
  },
});
