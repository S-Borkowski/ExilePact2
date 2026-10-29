// The Settings window's browser preview: outside Tauri (`npm run dev`), the
// window renders one of the serialised views under `fixtures/app-settings/`,
// chosen by URL query, so each state can be looked at without the app:
//
//   http://127.0.0.1:1422/settings.html?state=install-downloading&scale=2
//
// `state` is a fixture's name (default `idle`), `scale` zooms the page as
// the webview zoom would (`1`|`1.33`|`2`). Commands are logged, not acted
// on: the preview shows a view, it does not run the app. The blocked
// currencies' picker reads `fixtures/catalogue.json`, as the main window's
// preview does.
//
// The fixtures are reached only behind `import.meta.env.DEV`, which is false
// in `vite build`, so the release bundle carries none of them.

import { previewCatalogue } from '../bridge';
import type { SettingsCommand, SettingsCommandArgs, SettingsWindowBridge } from './bridge';
import type { SettingsView } from './view';

type Loader = () => Promise<unknown>;

const FIXTURES: Record<string, Loader> = import.meta.env.DEV
  ? import.meta.glob('../../fixtures/app-settings/*.json', { import: 'default' })
  : {};

/**
 * The fixtures: one per check state and one per install state (`install-*`;
 * the install state `none` is `idle`'s and `ready` is `available`'s), then a
 * refused hotkey, a saved blocked list and the overlay's settings changed
 * from their defaults. What
 * `fixtures/app-settings/README.md` lists.
 */
export const PREVIEW_STATES = [
  'idle',
  'checking',
  'up-to-date',
  'available',
  'no-release',
  'failed',
  'install-unsigned',
  'install-downloading',
  'install-verifying',
  'install-restarting',
  'install-failed',
  'hotkey-refused',
  'blocked-currencies',
  'overlay-settings',
] as const;
export type PreviewState = (typeof PREVIEW_STATES)[number];

/** The zoom levels the preview offers, as the webview's scale would be. */
export const PREVIEW_SCALES = ['1', '1.33', '2'] as const;

/** The state the preview shows with no `state` in the query. */
export const DEFAULT_PREVIEW_STATE: PreviewState = 'idle';

export interface SettingsPreviewQuery {
  state: PreviewState;
  scale: string | null;
}

export function parseSettingsPreviewQuery(search: string): SettingsPreviewQuery {
  const q = new URLSearchParams(search);
  const state = q.get('state');
  const scale = q.get('scale');
  return {
    state: state && (PREVIEW_STATES as readonly string[]).includes(state) ? (state as PreviewState) : DEFAULT_PREVIEW_STATE,
    scale: scale && (PREVIEW_SCALES as readonly string[]).includes(scale) ? scale : null,
  };
}

/** The view of one fixture, a fresh copy. */
export async function settingsPreviewFixture(state: PreviewState): Promise<SettingsView> {
  const load = FIXTURES[`../../fixtures/app-settings/${state}.json`];
  if (!load) throw new Error(`crafting-helper preview: no fixture app-settings/${state}.json (the preview runs on the dev server only)`);
  return JSON.parse(JSON.stringify(await load())) as SettingsView;
}

/** The bridge in a browser: a fixture view from the URL query, commands logged. */
export function previewSettingsBridge(search: string, root: HTMLElement = document.documentElement): SettingsWindowBridge & {
  readonly calls: Array<{ name: SettingsCommand; args: unknown }>;
} {
  const query = parseSettingsPreviewQuery(search);
  const calls: Array<{ name: SettingsCommand; args: unknown }> = [];
  return {
    kind: 'preview',
    calls,
    async start(onView) {
      if (query.scale) {
        root.dataset.previewScale = query.scale;
        root.style.setProperty('--preview-scale', query.scale);
      }
      onView(await settingsPreviewFixture(query.state));
      return () => {};
    },
    async command<C extends SettingsCommand>(name: C, args: SettingsCommandArgs[C]) {
      calls.push({ name, args });
      console.info(`crafting-helper settings preview: ${name}`, args);
    },
    catalogue: previewCatalogue,
  };
}
