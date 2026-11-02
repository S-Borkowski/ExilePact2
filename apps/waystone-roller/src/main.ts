// The page's entry point: the theme copied from the quest tracker (the
// Claude tokens, then the Exile overrides scoped to `data-theme='exile'`),
// the page's own base rules, the Exile rules over them, then the requested
// art any delivered file wires in, the bridge (the Tauri shell inside the
// app, the fixture preview in a plain browser), then the root component.
import '../theme/tokens.css';
import '../theme/exile.css';
import './app.css';
import './steel.css';
import { mount } from 'svelte';
import { isTauri } from '@tauri-apps/api/core';
import App from './App.svelte';
import { previewBridge, tauriBridge } from './bridge';
import { applyRequestedArt } from './requested-art';
import { applyTheme } from './theme';

const target = document.getElementById('app');
if (!target) {
  throw new Error('waystone-roller: #app is missing from index.html');
}

// Exile until the first view says otherwise.
applyTheme(document.documentElement, undefined);
applyRequestedArt(document.documentElement);

const bridge = isTauri() ? tauriBridge() : previewBridge(window.location.search);

export default mount(App, { target, props: { bridge } });
