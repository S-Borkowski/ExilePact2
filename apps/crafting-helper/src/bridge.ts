// The one door between a page and the Rust shell. Rust decides, the page
// renders: the shell pushes a fresh `View` on `ch:view`, to the main window
// and the overlay, after every change; the page sends a command and waits
// for the next `ch:view`, never patching its copy of the view.
//
// Outside Tauri (`npm run dev`), `previewBridge` renders one of the view
// fixtures under `fixtures/views/`, chosen by the query:
//
//   http://127.0.0.1:1422/?state=grabbed-item
//   http://127.0.0.1:1422/overlay.html?state=overlay-active
//
// Commands are logged, not acted on: the preview has no app behind it, so a
// paste is not shown as a grab, a target change does not re-plan and the
// Craft of Exile button opens nothing. The currency pickers read the
// catalogue once, from `get_catalogue` in the app and from
// `fixtures/catalogue.json` in the preview. The fixtures are reached only
// behind `import.meta.env.DEV`, which is false in `vite build`, so the
// release bundle carries none of them.

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { type CatalogueGroup, type View, catalogueProblems, viewProblems } from './ipc';

export const VIEW_EVENT = 'ch:view';

/** Every command the main window sends, with its arguments. `get_view` is the bridge's own. */
export interface CommandArgs {
  /**
   * The text a paste into the main window carried, read from the paste
   * event itself; the app validates it as it does a hotkey grab, and reads
   * no clipboard for it.
   */
  grab_text: { text: string };
  /** The header's Settings button: opens the Settings window, or brings it up. */
  open_settings: Record<string, never>;
  /**
   * The whole list of targets, in the order chosen; `floor` null is "as
   * high as possible". The app refuses the list whole when an id or a floor
   * does not fit the item, and drops repeats.
   */
  set_targets: { targets: TargetPick[] };
  /** The whole effective blocked list for this craft, catalogue ids only; Settings is never changed by it. */
  set_craft_blocked: { blocked: string[] };
  /** Previews another applicable route. */
  choose_route: { blueprint: string };
  /** Starts the guide on the previewed route. */
  accept_craft: Record<string, never>;
  /** Back to planning on the latest item, targets and this craft's blocked changes kept, history cleared. */
  end_craft: Record<string, never>;
  /**
   * The header's Craft of Exile button. It takes nothing: the app holds the
   * one address it opens, and the page names none.
   */
  open_craft_of_exile: Record<string, never>;
}

/** One target as `set_targets` sends it. */
export interface TargetPick {
  mod_id: string;
  floor: number | null;
}
export type Command = keyof CommandArgs;
/** How a component sends a command: fire and forget, the next view answers. */
export type Send = <C extends Command>(name: C, args: CommandArgs[C]) => void;

/** What `update_settings` takes: either key, or both; the app refuses any other. */
export interface SettingsPatch {
  check_updates?: boolean;
  /** The whole list, which replaces the saved one. */
  blocked_currencies?: string[];
}

/**
 * The Settings window's commands, with their arguments; each answers with
 * the window's view (`SettingsView`, fixtures under `fixtures/app-settings/`),
 * and the shell also pushes it on `ch:app-settings` after every change.
 * `update_settings`, `set_hotkey` and `install_update` reject with a
 * one-line reason when refused. `install_update` is the only way an install
 * starts; no command opens a release page or a browser.
 */
export interface SettingsCommandArgs {
  get_app_settings: Record<string, never>;
  update_settings: { patch: SettingsPatch };
  /** The new global hotkey; the app refuses Ctrl+Alt+C in any spelling and keeps the old one. */
  set_hotkey: { hotkey: string };
  check_for_updates: Record<string, never>;
  install_update: Record<string, never>;
  cancel_update: Record<string, never>;
}
export type SettingsCommand = keyof SettingsCommandArgs;

/** The event that carries the Settings window's view. */
export const APP_SETTINGS_EVENT = 'ch:app-settings';

export interface Bridge {
  /** `tauri` inside the app, `preview` in a browser over the dev server. */
  readonly kind: 'tauri' | 'preview';
  /** Subscribes to `ch:view`, then delivers the current view; resolves to an unsubscribe. */
  start(onView: (view: View) => void): Promise<() => void>;
  /** Sends one command. A failure is logged; the next `ch:view` shows the state either way. */
  command<C extends Command>(name: C, args: CommandArgs[C]): Promise<void>;
  /** The catalogue the currency pickers list, read once. */
  catalogue(): Promise<CatalogueGroup[]>;
}

function warnIfOff(what: string, problems: string[]): void {
  if (problems.length > 0) console.warn(`crafting-helper: ${what} differs from the IPC types`, problems);
}

/**
 * The catalogue from the app (`get_catalogue`, which both windows may call):
 * every crafting item the blocked list may name, grouped by kind. The
 * Settings window's bridge reads it the same way.
 */
export async function readCatalogue(): Promise<CatalogueGroup[]> {
  const groups = await invoke<CatalogueGroup[]>('get_catalogue');
  warnIfOff('get_catalogue', catalogueProblems(groups));
  return groups;
}

const PREVIEW_CATALOGUE: Record<string, () => Promise<unknown>> = import.meta.env.DEV
  ? import.meta.glob('../fixtures/catalogue.json', { import: 'default' })
  : {};

/** The catalogue in the preview: `fixtures/catalogue.json`, which `ch-app`'s fixtures test writes. */
export async function previewCatalogue(): Promise<CatalogueGroup[]> {
  const load = PREVIEW_CATALOGUE['../fixtures/catalogue.json'];
  if (!load) throw new Error('crafting-helper preview: no catalogue fixture (the preview runs on the dev server only)');
  return JSON.parse(JSON.stringify(await load())) as CatalogueGroup[];
}

/** The bridge inside the Tauri app, for the main window and the overlay alike. */
export function tauriBridge(): Bridge {
  return {
    kind: 'tauri',
    async start(onView) {
      const off = await listen<View>(VIEW_EVENT, (event) => {
        warnIfOff(`a ${VIEW_EVENT}`, viewProblems(event.payload));
        onView(event.payload);
      });
      // Subscribed first, so a view pushed while this call runs is not lost;
      // the later of the two simply wins.
      const view = await invoke<View>('get_view');
      warnIfOff('get_view', viewProblems(view));
      onView(view);
      return off;
    },
    async command(name, args) {
      try {
        await invoke(name, args as unknown as Record<string, unknown>);
      } catch (error) {
        console.error(`crafting-helper: ${name} failed`, error);
      }
    },
    catalogue: readCatalogue,
  };
}

type Loader = () => Promise<unknown>;

const PREVIEW_FIXTURES: Record<string, Loader> = import.meta.env.DEV
  ? import.meta.glob('../fixtures/views/*.json', { import: 'default' })
  : {};

/** The fixture the main window's preview shows with no `state` in the query. */
export const DEFAULT_PREVIEW_VIEW = 'grabbed-item';
/** The fixture the overlay's preview shows with no `state` in the query. */
export const DEFAULT_OVERLAY_PREVIEW_VIEW = 'overlay-active';

/** The fixture a query's `state` names, or `fallback` for none or an unknown one. */
export function previewViewName(search: string, fallback: string = DEFAULT_PREVIEW_VIEW): string {
  const name = new URLSearchParams(search).get('state');
  return name && PREVIEW_FIXTURES[`../fixtures/views/${name}.json`] ? name : fallback;
}

/** The bridge in a browser: a fixture view from the URL query, commands logged. */
export function previewBridge(
  search: string,
  fallback: string = DEFAULT_PREVIEW_VIEW,
): Bridge & { readonly calls: Array<{ name: Command; args: unknown }> } {
  const calls: Array<{ name: Command; args: unknown }> = [];
  return {
    kind: 'preview',
    calls,
    async start(onView) {
      const load = PREVIEW_FIXTURES[`../fixtures/views/${previewViewName(search, fallback)}.json`];
      if (!load) throw new Error('crafting-helper preview: no view fixtures (the preview runs on the dev server only)');
      // A fresh copy, so nothing leaks into the module's cached JSON.
      onView(JSON.parse(JSON.stringify(await load())) as View);
      return () => {};
    },
    async command<C extends Command>(name: C, args: CommandArgs[C]) {
      calls.push({ name, args });
      console.info(`crafting-helper preview: ${name}`, args);
    },
    catalogue: previewCatalogue,
  };
}
