// The main window's entry point: the theme copied from the quest tracker,
// the page's own base rules, then the bridge (the Tauri shell inside the
// app, the fixture preview in a plain browser), then the root component.
import '../theme/tokens.css';
import './app.css';
import { mount } from 'svelte';
import { isTauri } from '@tauri-apps/api/core';
import App from './App.svelte';
import { previewBridge, tauriBridge } from './bridge';

const target = document.getElementById('app');
if (!target) {
  throw new Error('crafting-helper: #app is missing from index.html');
}

const bridge = isTauri() ? tauriBridge() : previewBridge(window.location.search);

export default mount(App, { target, props: { bridge } });
