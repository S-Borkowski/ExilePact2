// The Settings window's entry point (`settings.html`), in the main page's
// order: the theme copied from the quest tracker, the main page's base rules
// (buttons, focus ring), the window's own shared rules, then the bridge (the
// Tauri shell inside the app, the fixture preview in a plain browser), then
// the root component mounted into #app.
import '../../theme/tokens.css';
import '../app.css';
import './settings.css';
import { mount } from 'svelte';
import { isTauri } from '@tauri-apps/api/core';
import SettingsWindow from './SettingsWindow.svelte';
import { tauriSettingsBridge } from './bridge';
import { previewSettingsBridge } from './preview';

const target = document.getElementById('app');
if (!target) {
  throw new Error('waystone-roller: #app is missing from settings.html');
}

const bridge = isTauri() ? tauriSettingsBridge() : previewSettingsBridge(window.location.search);

export default mount(SettingsWindow, { target, props: { bridge } });
