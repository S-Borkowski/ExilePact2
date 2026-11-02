// The overlay's entry point (`overlay.html`): the theme copied from the
// quest tracker (Claude's tokens, then the Exile tokens over them), the main
// page's base rules, the overlay's own (a transparent page), the Exile rules
// (`../steel.css`), any requested art delivered, then the bridge the main window uses (`ch:view` and
// `get_view` inside the app, a view fixture in a plain browser, with an
// optional stand-in scene behind it), then the root component.
import '../../theme/tokens.css';
import '../../theme/exile.css';
import '../app.css';
import './overlay.css';
import '../steel.css';
import { mount } from 'svelte';
import { isTauri } from '@tauri-apps/api/core';
import Overlay from './Overlay.svelte';
import { DEFAULT_OVERLAY_PREVIEW_VIEW, previewBridge, tauriBridge } from '../bridge';
import { applyPreview } from './preview';
import { applyRequestedArt } from '../requested-art';
import { applyTheme } from '../theme';

const target = document.getElementById('app');
if (!target) {
  throw new Error('crafting-helper: #app is missing from overlay.html');
}

// Exile until the first view says otherwise.
applyTheme(document.documentElement, undefined);
applyRequestedArt(document.documentElement);

const inApp = isTauri();
if (!inApp) applyPreview(window.location.search);
const bridge = inApp ? tauriBridge() : previewBridge(window.location.search, DEFAULT_OVERLAY_PREVIEW_VIEW);

export default mount(Overlay, { target, props: { bridge } });
