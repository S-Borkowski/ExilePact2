// The browser preview: outside Tauri (`npm run dev`), the UI renders one of
// the serialised views under `fixtures/viewmodels/`, chosen by URL query, so
// every frame of the brief can be screenshotted without the game:
//
//   http://127.0.0.1:1420/?frame=quest-detail&mode=play&scale=2
//
// `frame` is a frame id of the brief (default `panel-next-up`), `mode`
// overrides the fixture's mode (`play`|`edit`), `scale` zooms the page as the
// webview zoom would (`1`|`1.33`|`2`). Commands are logged, not acted on: the
// preview shows a view, it does not run the engine.
//
// The fixtures are reached only behind `import.meta.env.DEV`, which is false
// in `vite build`, so the release bundle carries none of them.

import type { Bridge, Command, CommandArgs } from './bridge';
import { type Mode, type Toast, type View, MODES } from './ipc';

type Loader = () => Promise<unknown>;

const FIXTURES: Record<string, Loader> = import.meta.env.DEV
  ? import.meta.glob('../fixtures/viewmodels/*.json', { import: 'default' })
  : {};

/** The frame the preview shows with no `frame` in the query. */
export const DEFAULT_PREVIEW_FRAME = 'panel-next-up';

/** Brief frame ids that are not a `View` frame, and the fixture each is shown on. */
const STAND_INS: Record<string, { fixture: string; mode?: Mode; toast?: boolean }> = {
  'mode-play': { fixture: 'panel-campaign', mode: 'play' },
  'mode-edit': { fixture: 'panel-campaign', mode: 'edit' },
  toast: { fixture: 'panel-next-up', toast: true },
};

export const PREVIEW_SCALES = ['1', '1.33', '2'] as const;

export interface PreviewQuery {
  frame: string;
  mode: Mode | null;
  scale: string | null;
}

export function parsePreviewQuery(search: string): PreviewQuery {
  const q = new URLSearchParams(search);
  const mode = q.get('mode');
  const scale = q.get('scale');
  return {
    frame: q.get('frame') || DEFAULT_PREVIEW_FRAME,
    mode: mode && (MODES as readonly string[]).includes(mode) ? (mode as Mode) : null,
    scale: scale && (PREVIEW_SCALES as readonly string[]).includes(scale) ? scale : null,
  };
}

async function loadFixture(name: string): Promise<unknown> {
  const load = FIXTURES[`../fixtures/viewmodels/${name}.json`];
  if (!load) throw new Error(`quest-tracker preview: no fixture ${name}.json (the preview runs on the dev server only)`);
  // A fresh copy, so an override never leaks into the module's cached JSON.
  return JSON.parse(JSON.stringify(await load()));
}

/** The view, and the toasts to show over it, for one brief frame id. */
export async function previewFixture(frame: string, mode: Mode | null): Promise<{ view: View; toasts: Toast[] }> {
  const standIn = STAND_INS[frame];
  const known = standIn || FIXTURES[`../fixtures/viewmodels/${frame}.json`];
  const view = (await loadFixture(standIn ? standIn.fixture : known ? frame : 'panel-campaign')) as View;
  const forced = mode ?? standIn?.mode ?? null;
  if (forced) view.mode = forced;
  if (!known) view.status = { kind: 'warn', text: `Preview: no frame "${frame}"` };
  const toasts = standIn?.toast ? [(await loadFixture('toast')) as Toast] : [];
  return { view, toasts };
}

/** The bridge in a browser: a fixture view from the URL query, commands logged. */
export function previewBridge(search: string, root: HTMLElement = document.documentElement): Bridge & {
  readonly calls: Array<{ name: Command; args: unknown }>;
} {
  const query = parsePreviewQuery(search);
  const calls: Array<{ name: Command; args: unknown }> = [];
  return {
    kind: 'preview',
    holdToasts: true,
    calls,
    async start(onView, onToast) {
      if (query.scale) {
        root.dataset.previewScale = query.scale;
        root.style.setProperty('--preview-scale', query.scale);
      }
      const { view, toasts } = await previewFixture(query.frame, query.mode);
      onView(view);
      for (const toast of toasts) onToast(toast);
      return () => {};
    },
    async command<C extends Command>(name: C, args: CommandArgs[C]) {
      calls.push({ name, args });
      console.info(`quest-tracker preview: ${name}`, args);
    },
  };
}
