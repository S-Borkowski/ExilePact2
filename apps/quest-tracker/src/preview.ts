// The browser preview: outside Tauri (`npm run dev`), the UI renders one of
// the serialised views under `fixtures/viewmodels/`, chosen by URL query, so
// every frame of the brief can be screenshotted without the game:
//
//   http://127.0.0.1:1420/?frame=quest-detail&mode=play&scale=2
//
// `frame` is a frame id of the brief (default `panel-next-up`), `mode`
// overrides the fixture's mode (`play`|`edit`), `scale` zooms the page as the
// webview zoom would (`0.75`|`1`|`1.33`|`2`; any other value is ignored).
// Commands are logged, not acted on: the preview shows a view, it does not
// run the engine.
//
// `frame=next-up-sections` is a dev-only Next up view with all three worth
// sections and two quests in each (Worth it, Optional, Not worth it), built
// from the rows of `panel-campaign.json` with each row's worth set to its
// section (`sectionsView`). The frozen fixtures have no section with two
// quests and no "Not worth it" one, and the section layout needs both:
//
//   http://127.0.0.1:1420/?frame=next-up-sections&mode=play&scale=1&theme=exile
//
// The fixtures are reached only behind `import.meta.env.DEV`, which is false
// in `vite build`, so the release bundle carries none of them.

import type { Bridge, Command, CommandArgs, ResizeDirection } from './bridge';
import { type Mode, type View, type Worth, MODES } from './ipc';

type Loader = () => Promise<unknown>;

const FIXTURES: Record<string, Loader> = import.meta.env.DEV
  ? import.meta.glob('../fixtures/viewmodels/*.json', { import: 'default' })
  : {};

/** The frame the preview shows with no `frame` in the query. */
export const DEFAULT_PREVIEW_FRAME = 'panel-next-up';

/** Brief frame ids that are not a `View` frame, and the fixture each is shown on. */
const STAND_INS: Record<string, { fixture: string; mode?: Mode }> = {
  'mode-play': { fixture: 'panel-campaign', mode: 'play' },
  'mode-edit': { fixture: 'panel-campaign', mode: 'edit' },
};

/** The zooms the preview accepts; `0.75` is the smallest the Appearance slider reaches. */
export const PREVIEW_SCALES = ['0.75', '1', '1.33', '2'] as const;

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

/** The preview frame id of the three-section Next up view. */
export const SECTIONS_PREVIEW_FRAME = 'next-up-sections';

/** The three-section view's rows: a `panel-campaign.json` quest id and the section it goes in, in order. */
export const SECTION_ROWS: ReadonlyArray<readonly [string, Worth]> = [
  ['fixture-lute', 'yes'],
  ['fixture-crowbell', 'yes'],
  ['fixture-king', 'optional'],
  ['fixture-candles', 'optional'],
  ['fixture-beira', 'no'],
  ['fixture-riverbank', 'no'],
];

/**
 * The Next up view `nextUp` with its rows replaced by `SECTION_ROWS`, each
 * taken from `campaign`'s acts with its worth set to its section.
 */
export function sectionsView(nextUp: View, campaign: View): View {
  const quests = new Map(campaign.acts.flatMap((act) => act.quests).map((quest) => [quest.id, quest]));
  const next_up = SECTION_ROWS.map(([id, worth]) => {
    const quest = quests.get(id);
    if (!quest) throw new Error(`quest-tracker preview: panel-campaign.json has no quest ${id}`);
    return { ...quest, worth };
  });
  return { ...nextUp, frame: 'panel-next-up', next_up };
}

/** The view for one brief frame id. */
export async function previewFixture(frame: string, mode: Mode | null): Promise<{ view: View }> {
  if (frame === SECTIONS_PREVIEW_FRAME) {
    const view = sectionsView((await loadFixture('panel-next-up')) as View, (await loadFixture('panel-campaign')) as View);
    if (mode) view.mode = mode;
    return { view };
  }
  const standIn = STAND_INS[frame];
  const known = standIn || FIXTURES[`../fixtures/viewmodels/${frame}.json`];
  const view = (await loadFixture(standIn ? standIn.fixture : known ? frame : 'panel-campaign')) as View;
  const forced = mode ?? standIn?.mode ?? null;
  if (forced) view.mode = forced;
  if (!known) view.status = { kind: 'warn', text: `Preview: no frame "${frame}"` };
  return { view };
}

/** The bridge in a browser: a fixture view from the URL query, commands logged. */
export function previewBridge(search: string, root: HTMLElement = document.documentElement): Bridge & {
  readonly calls: Array<{ name: Command; args: unknown }>;
  /** The resize drags the panel's edge handles asked for, in order. */
  readonly resizes: ResizeDirection[];
} {
  const query = parsePreviewQuery(search);
  const calls: Array<{ name: Command; args: unknown }> = [];
  const resizes: ResizeDirection[] = [];
  return {
    kind: 'preview',
    calls,
    resizes,
    async start(onView) {
      if (query.scale) {
        root.dataset.previewScale = query.scale;
        root.style.setProperty('--preview-scale', query.scale);
        // The overlay's margin divides by the page's zoom, as in the app.
        root.style.setProperty('--page-zoom', query.scale);
      }
      const { view } = await previewFixture(query.frame, query.mode);
      const theme = new URLSearchParams(search).get('theme');
      if (theme === 'exile' || theme === 'claude') view.settings.theme = theme;
      onView(view);
      return () => {};
    },
    async command<C extends Command>(name: C, args: CommandArgs[C]) {
      calls.push({ name, args });
      console.info(`quest-tracker preview: ${name}`, args);
    },
    // A browser tab has no overlay window to move.
    startDrag() {},
    // Nor one to resize: the press is recorded and logged, so the handles
    // can be checked in the preview.
    startResize(direction) {
      resizes.push(direction);
      console.info(`quest-tracker preview: resize from ${direction}`);
    },
  };
}
