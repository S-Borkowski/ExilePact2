// The Settings window's entry point (`settings.html`), in the overlay's
// order: the default tokens, then the harvested theme over them, then the
// styled layer's shared rules, then the bridge (the Tauri shell inside the
// app, the fixture preview in a plain browser), then the root component
// mounted into #app.
import '../defaults.css';
import { applyDerivedTokens } from '../theme';
import '../styled.css';
import '../approved.css';
import { mount } from 'svelte';
import { isTauri } from '@tauri-apps/api/core';
import SettingsWindow from './SettingsWindow.svelte';
import { tauriSettingsBridge } from './bridge';
import { previewSettingsBridge } from './preview';

const target = document.getElementById('app');
if (!target) {
  throw new Error('quest-tracker: #app is missing from settings.html');
}

applyDerivedTokens(document.documentElement);

const bridge = isTauri() ? tauriSettingsBridge() : previewSettingsBridge(window.location.search);

export default mount(SettingsWindow, { target, props: { bridge } });
