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
  // Three pages: the main window (`index.html`), the Settings window its
  // header's button opens (`settings.html`) and the click-through overlay
  // (`overlay.html`). Paths are relative to the Vite root, this folder.
  build: {
    rolldownOptions: {
      input: { main: 'index.html', settings: 'settings.html', overlay: 'overlay.html' },
    },
  },
  // Keep Rust errors visible when run under `tauri dev`.
  clearScreen: false,
  server: {
    // The port `devUrl` in tauri.conf.json names; the quest tracker's is
    // 1420 and the waystone roller's 1421.
    port: 1422,
    strictPort: true,
    host: '127.0.0.1',
    watch: {
      ignored: ['**/src-tauri/**', '**/crates/**', '**/target/**'],
    },
  },
  test: {
    environment: 'jsdom',
    include: ['src/**/*.test.ts'],
    // Vitest blanks CSS by default. The components' own `<style>` blocks
    // and the shared `app.css` are kept, so a page test reads the styles
    // jsdom resolves from what ships (`src/ui-spacing.test.ts`).
    css: { include: [/\.svelte\?svelte&type=style/, /src[\\/]app\.css/] },
  },
}));
