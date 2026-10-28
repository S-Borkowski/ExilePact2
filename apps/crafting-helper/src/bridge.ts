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
// paste is not shown as a grab. The fixtures are reached only behind
// `import.meta.env.DEV`, which is false in `vite build`, so the release
// bundle carries none of them.

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { type View, viewProblems } from './ipc';

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
}

function warnIfOff(what: string, problems: string[]): void {
  if (problems.length > 0) console.warn(`crafting-helper: ${what} differs from the IPC types`, problems);
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
  };
}
