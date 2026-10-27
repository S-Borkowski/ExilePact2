// The Settings window's browser preview: outside Tauri (`npm run dev`), the
// window renders one of the serialised views under `fixtures/app-settings/`,
// chosen by URL query, so each state can be looked at without the app:
//
//   http://127.0.0.1:1420/settings.html?state=available&scale=2
//
// `state` is an update state (default `idle`), `scale` zooms the page as the
// webview zoom would (`1`|`1.33`|`2`). Commands are logged, not acted on: the
// preview shows a view, it does not run the app.
//
// The fixtures are reached only behind `import.meta.env.DEV`, which is false
// in `vite build`, so the release bundle carries none of them.

import { PREVIEW_SCALES } from '../preview';
import type { SettingsBridge, SettingsCommand, SettingsCommandArgs } from './bridge';
import { type AppSettingsView, type UpdateState, UPDATE_STATES } from './view';

type Loader = () => Promise<unknown>;

const FIXTURES: Record<string, Loader> = import.meta.env.DEV
  ? import.meta.glob('../../fixtures/app-settings/*.json', { import: 'default' })
  : {};

/** The state the preview shows with no `state` in the query. */
export const DEFAULT_PREVIEW_STATE: UpdateState = 'idle';

export interface SettingsPreviewQuery {
  state: UpdateState;
  scale: string | null;
}

export function parseSettingsPreviewQuery(search: string): SettingsPreviewQuery {
  const q = new URLSearchParams(search);
  const state = q.get('state');
  const scale = q.get('scale');
  return {
    state: state && (UPDATE_STATES as readonly string[]).includes(state) ? (state as UpdateState) : DEFAULT_PREVIEW_STATE,
    scale: scale && (PREVIEW_SCALES as readonly string[]).includes(scale) ? scale : null,
  };
}

/** The view for one update state, a fresh copy. */
export async function settingsPreviewFixture(state: UpdateState): Promise<AppSettingsView> {
  const load = FIXTURES[`../../fixtures/app-settings/${state}.json`];
  if (!load) throw new Error(`quest-tracker preview: no fixture app-settings/${state}.json (the preview runs on the dev server only)`);
  return JSON.parse(JSON.stringify(await load())) as AppSettingsView;
}

/** The bridge in a browser: a fixture view from the URL query, commands logged. */
export function previewSettingsBridge(search: string, root: HTMLElement = document.documentElement): SettingsBridge & {
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
      console.info(`quest-tracker settings preview: ${name}`, args);
    },
  };
}
