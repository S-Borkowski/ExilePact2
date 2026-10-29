// The overlay's entry point (`overlay.html`): the theme copied from the
// quest tracker, the main page's base rules, the overlay's own (a
// transparent page), then the bridge the main window uses (`ch:view` and
// `get_view` inside the app, a view fixture in a plain browser, with an
// optional stand-in scene behind it), then the root component.
import '../../theme/tokens.css';
import '../app.css';
import './overlay.css';
import { mount } from 'svelte';
import { isTauri } from '@tauri-apps/api/core';
import Overlay from './Overlay.svelte';
import { DEFAULT_OVERLAY_PREVIEW_VIEW, previewBridge, tauriBridge } from '../bridge';
import { applyPreview } from './preview';

const target = document.getElementById('app');
if (!target) {
  throw new Error('crafting-helper: #app is missing from overlay.html');
}

const inApp = isTauri();
if (!inApp) applyPreview(window.location.search);
const bridge = inApp ? tauriBridge() : previewBridge(window.location.search, DEFAULT_OVERLAY_PREVIEW_VIEW);

export default mount(Overlay, { target, props: { bridge } });
