// The Settings window's browser preview: outside Tauri (`npm run dev`), the
// window renders one of the serialised views under `fixtures/app-settings/`,
// chosen by URL query, so each state can be looked at without the app:
//
//   http://127.0.0.1:1420/settings.html?state=available&scale=2
//   http://127.0.0.1:1420/settings.html?install=downloading
//
// `state` is an update state (default `idle`); `install` is an install state
// and shows its `install/` fixture instead, whatever `state` says; `scale`
// zooms the page as the webview zoom would (`1`|`1.33`|`2`). Commands are
// logged, not acted on: the preview shows a view, it does not run the app.
//
// The fixtures are reached only behind `import.meta.env.DEV`, which is false
// in `vite build`, so the release bundle carries none of them.

import { PREVIEW_SCALES } from '../preview';
import type { SettingsBridge, SettingsCommand, SettingsCommandArgs } from './bridge';
import { type AppSettingsView, type InstallState, INSTALL_STATES, type UpdateState, UPDATE_STATES } from './view';

type Loader = () => Promise<unknown>;

const FIXTURES: Record<string, Loader> = import.meta.env.DEV
  ? import.meta.glob(['../../fixtures/app-settings/*.json', '../../fixtures/app-settings/install/*.json'], {
      import: 'default',
    })
  : {};

/** The state the preview shows with no `state` in the query. */
export const DEFAULT_PREVIEW_STATE: UpdateState = 'idle';

export interface SettingsPreviewQuery {
  state: UpdateState;
  /** An install state whose `install/` fixture is shown instead of `state`'s. */
  install: InstallState | null;
  scale: string | null;
}

export function parseSettingsPreviewQuery(search: string): SettingsPreviewQuery {
  const q = new URLSearchParams(search);
  const state = q.get('state');
  const install = q.get('install');
  const scale = q.get('scale');
  return {
    state: state && (UPDATE_STATES as readonly string[]).includes(state) ? (state as UpdateState) : DEFAULT_PREVIEW_STATE,
    install: install && (INSTALL_STATES as readonly string[]).includes(install) ? (install as InstallState) : null,
    scale: scale && (PREVIEW_SCALES as readonly string[]).includes(scale) ? scale : null,
  };
}

/** The view for one fixture (`available`, `install/downloading`, ...), a fresh copy. */
export async function settingsPreviewFixture(name: UpdateState | `install/${InstallState}`): Promise<AppSettingsView> {
  const load = FIXTURES[`../../fixtures/app-settings/${name}.json`];
  if (!load) throw new Error(`quest-tracker preview: no fixture app-settings/${name}.json (the preview runs on the dev server only)`);
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
      const view = await settingsPreviewFixture(query.install ? `install/${query.install}` : query.state);
      const theme = new URLSearchParams(search).get('theme');
      if (theme === 'exile' || theme === 'claude') view.theme = theme;
      onView(view);
      return () => {};
    },
    async command<C extends SettingsCommand>(name: C, args: SettingsCommandArgs[C]) {
      calls.push({ name, args });
      console.info(`quest-tracker settings preview: ${name}`, args);
    },
  };
}
