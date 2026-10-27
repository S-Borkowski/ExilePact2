/// <reference types="vitest/config" />
import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

// The Vite root is this folder; `vite build` writes `dist/`, which
// `src-tauri/tauri.conf.json` embeds as `frontendDist`.
export default defineConfig(({ mode }) => ({
  plugins: [svelte()],
  // Under vitest, resolve Svelte's client runtime (the `browser` export
  // condition) so components mount in jsdom; the dev server and the build
  // keep Vite's default conditions.
  resolve: mode === 'test' ? { conditions: ['browser'] } : undefined,
  // Two pages: the overlay (`index.html`) and the tray's Settings window
  // (`settings.html`). Paths are relative to the Vite root, this folder.
  build: {
    rolldownOptions: {
      input: { main: 'index.html', settings: 'settings.html' },
    },
  },
  // Keep Rust errors visible when run under `tauri dev`.
  clearScreen: false,
  server: {
    // The port `devUrl` in tauri.conf.json names.
    port: 1420,
    strictPort: true,
    host: '127.0.0.1',
    // The tests read the brief's export contract from docs/; the dev server
    // keeps Vite's default (this folder only).
    fs: mode === 'test' ? { allow: ['.', '../../docs/quest-tracker'] } : undefined,
    watch: {
      ignored: ['**/src-tauri/**', '**/crates/**', '**/target/**'],
    },
  },
  test: {
    environment: 'jsdom',
    include: ['src/**/*.test.ts'],
    // Vitest blanks CSS by default, `?raw` imports too; the token-key test
    // reads the defaults file's text.
    css: { include: [/src[\\/]defaults\.css/] },
  },
}));
