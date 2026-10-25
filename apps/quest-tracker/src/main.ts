// The overlay UI's entry point: the default tokens, then the harvested theme
// over them, then the styled layer's shared rules, then the bridge (the Tauri
// shell inside the app, the fixture preview in a plain browser), then the
// root component mounted into #app.
import './defaults.css';
import { applyDerivedTokens } from './theme';
import './styled.css';
import { mount } from 'svelte';
import { isTauri } from '@tauri-apps/api/core';
import App from './App.svelte';
import { tauriBridge } from './bridge';
import { previewBridge } from './preview';

const target = document.getElementById('app');
if (!target) {
  throw new Error('quest-tracker: #app is missing from index.html');
}

applyDerivedTokens(document.documentElement);

const bridge = isTauri() ? tauriBridge() : previewBridge(window.location.search);

export default mount(App, { target, props: { bridge } });
