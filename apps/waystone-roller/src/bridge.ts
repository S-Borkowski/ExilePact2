// The one door between the page and the Rust shell. Rust decides, the page
// renders: the shell pushes a fresh `View` on `wr:view` after every change;
// the page sends a command and waits for the next `wr:view`, never patching
// its copy of the view.
//
// Outside Tauri (`npm run dev`), `previewBridge` renders one of the view
// fixtures under `fixtures/views/`, chosen by the query:
//
//   http://127.0.0.1:1421/?view=too-long
//
// Commands are logged. A selection patch is echoed into the shown selection,
// and the paste checkbox into the shown `auto_paste`, so the controls
// respond, but the search string, the meter and the status
// stay the fixture's: the preview has no engine. The fixtures are reached
// only behind `import.meta.env.DEV`, which is false in `vite build`, so the
// release bundle carries none of them.

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { type Selection, type View, viewProblems } from './ipc';

export const VIEW_EVENT = 'wr:view';

/** Any subset of the selection's keys; the shell refuses a mod in both lists. */
export type SelectionPatch = Partial<Selection>;

/** Every command the page sends, with its arguments. `get_view` is the bridge's own. */
export interface CommandArgs {
  update_selection: { patch: SelectionPatch };
  copy_now: Record<string, never>;
  set_hotkey: { hotkey: string };
  /** The footer's checkbox: whether the shortcut pastes into the game after it copies. */
  set_auto_paste: { on: boolean };
  /** The header's Settings button: opens the Settings window, or brings it up. */
  open_settings: Record<string, never>;
}

/**
 * The Settings window's commands, with their arguments; each answers with
 * the window's view (`ep-update`'s `AppSettingsView`, fixtures under
 * `fixtures/app-settings/`), and the shell also pushes it on
 * `wr:app-settings` after every change. `update_settings` takes exactly
 * `{ check_updates }`; it and `install_update` reject with a one-line reason
 * when refused. `install_update` is the only way an install starts; no
 * command opens a release page or a browser.
 */
export interface SettingsCommandArgs {
  get_app_settings: Record<string, never>;
  update_settings: { patch: { check_updates: boolean } };
  check_for_updates: Record<string, never>;
  install_update: Record<string, never>;
  cancel_update: Record<string, never>;
}
export type SettingsCommand = keyof SettingsCommandArgs;

/** The event that carries the Settings window's view. */
export const APP_SETTINGS_EVENT = 'wr:app-settings';
export type Command = keyof CommandArgs;
/** How a component sends a command: fire and forget, the next view answers. */
export type Send = <C extends Command>(name: C, args: CommandArgs[C]) => void;

export interface Bridge {
  /** `tauri` inside the app, `preview` in a browser over the dev server. */
  readonly kind: 'tauri' | 'preview';
  /** Subscribes to `wr:view`, then delivers the current view; resolves to an unsubscribe. */
  start(onView: (view: View) => void): Promise<() => void>;
  /** Sends one command. A failure is logged; the next `wr:view` shows the state either way. */
  command<C extends Command>(name: C, args: CommandArgs[C]): Promise<void>;
}

function warnIfOff(what: string, problems: string[]): void {
  if (problems.length > 0) console.warn(`waystone-roller: ${what} differs from the IPC types`, problems);
}

/** The bridge inside the Tauri app. */
export function tauriBridge(): Bridge {
  return {
    kind: 'tauri',
    async start(onView) {
      const off = await listen<View>(VIEW_EVENT, (event) => {
        warnIfOff('a wr:view', viewProblems(event.payload));
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
        console.error(`waystone-roller: ${name} failed`, error);
      }
    },
  };
}

type Loader = () => Promise<unknown>;

const PREVIEW_FIXTURES: Record<string, Loader> = import.meta.env.DEV
  ? import.meta.glob('../fixtures/views/*.json', { import: 'default' })
  : {};

/** The fixture the preview shows with no `view` in the query. */
export const DEFAULT_PREVIEW_VIEW = 'ok-any';

/** The fixture a query names, or the default for none or an unknown one. */
export function previewViewName(search: string): string {
  const name = new URLSearchParams(search).get('view');
  return name && PREVIEW_FIXTURES[`../fixtures/views/${name}.json`] ? name : DEFAULT_PREVIEW_VIEW;
}

/** The bridge in a browser: a fixture view from the URL query, commands logged. */
export function previewBridge(search: string): Bridge & { readonly calls: Array<{ name: Command; args: unknown }> } {
  const calls: Array<{ name: Command; args: unknown }> = [];
  let current: View | null = null;
  let push: ((view: View) => void) | null = null;
  return {
    kind: 'preview',
    calls,
    async start(onView) {
      const load = PREVIEW_FIXTURES[`../fixtures/views/${previewViewName(search)}.json`];
      if (!load) throw new Error('waystone-roller preview: no view fixtures (the preview runs on the dev server only)');
      // A fresh copy, so an echoed patch never leaks into the module's cached JSON.
      current = JSON.parse(JSON.stringify(await load())) as View;
      push = onView;
      onView(current);
      return () => {
        push = null;
      };
    },
    async command<C extends Command>(name: C, args: CommandArgs[C]) {
      calls.push({ name, args });
      console.info(`waystone-roller preview: ${name}`, args);
      if (name === 'update_selection' && current && push) {
        const { patch } = args as CommandArgs['update_selection'];
        current = { ...current, selection: { ...current.selection, ...patch } };
        push(current);
      } else if (name === 'set_auto_paste' && current && push) {
        const { on } = args as CommandArgs['set_auto_paste'];
        current = { ...current, auto_paste: on };
        push(current);
      }
    },
  };
}
