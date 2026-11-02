// The Settings window's entry point (`settings.html`), in the main page's
// order: the theme copied from the quest tracker (Claude's tokens, then the
// Exile tokens over them), the main page's base rules (buttons, fields,
// focus ring), the window's own shared rules, the Exile rules
// (`../steel.css`), any requested art delivered, then the bridge (the Tauri shell inside the app, the fixture preview in a plain
// browser), then the root component mounted into #app.
import '../../theme/tokens.css';
import '../../theme/exile.css';
import '../app.css';
import './settings.css';
import '../steel.css';
import { mount } from 'svelte';
import { isTauri } from '@tauri-apps/api/core';
import SettingsWindow from './SettingsWindow.svelte';
import { tauriSettingsBridge } from './bridge';
import { previewSettingsBridge } from './preview';
import { applyRequestedArt } from '../requested-art';
import { applyTheme } from '../theme';

const target = document.getElementById('app');
if (!target) {
  throw new Error('crafting-helper: #app is missing from settings.html');
}

// Exile until the first view says otherwise.
applyTheme(document.documentElement, undefined);
applyRequestedArt(document.documentElement);

const bridge = isTauri() ? tauriSettingsBridge() : previewSettingsBridge(window.location.search);

export default mount(SettingsWindow, { target, props: { bridge } });
