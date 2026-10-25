// The one door between the UI and the Rust shell. Rust decides, the UI
// renders: the shell pushes a fresh `View` on `qt:view` after every change
// and a `Toast` on `qt:toast`; the UI calls commands and waits for the next
// `qt:view`, never polling and never patching its copy of the view.
//
// Command argument keys are camelCase (`questId`, `stepId`): Tauri 2 maps a
// Rust command's snake_case parameters to camelCase keys by default.

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import {
  type Dock,
  type Mode,
  type QuestAction,
  type Scale,
  type Toast,
  type View,
  type ViewKind,
  toastProblems,
  viewProblems,
} from './ipc';

export const VIEW_EVENT = 'qt:view';
export const TOAST_EVENT = 'qt:toast';

/** The frames `open_frame` opens over the panel. */
export type OverlayFrame = 'character-picker' | 'settings';

/** A partial settings object for `update_settings`; Rust refuses unknown keys. */
export interface SettingsPatch {
  opacity?: number;
  dock?: Dock;
  scale?: Scale;
  show_skipped?: boolean;
  only_worth_it?: boolean;
  log_path?: string | null;
  hotkey?: string;
}

/** Every command the UI sends, with its arguments. `get_view` is the bridge's own. */
export interface CommandArgs {
  set_mode: { mode: Mode };
  set_view_kind: { kind: ViewKind };
  set_collapsed: { collapsed: boolean };
  toggle_act: { act: number };
  open_detail: { questId: string | null };
  open_frame: { frame: OverlayFrame | null };
  quest_action: { questId: string; action: QuestAction };
  tick_step: { questId: string; stepId: string; ticked: boolean };
  select_character: { name: string };
  pin_character: { name: string | null };
  confirm_character: { name: string };
  update_settings: { patch: SettingsPatch };
  open_source: { questId: string };
}
export type Command = keyof CommandArgs;
export type Send = <C extends Command>(name: C, args: CommandArgs[C]) => void;

export interface Bridge {
  /** `tauri` inside the app, `preview` in a browser over the dev server. */
  readonly kind: 'tauri' | 'preview';
  /** Keep toasts up until replaced (the preview's screenshots), instead of ~3 s. */
  readonly holdToasts: boolean;
  /** Subscribes to both events, then delivers the current view; resolves to an unsubscribe. */
  start(onView: (view: View) => void, onToast: (toast: Toast) => void): Promise<() => void>;
  /** Sends one command. A failure is logged; the next `qt:view` shows the state either way. */
  command<C extends Command>(name: C, args: CommandArgs[C]): Promise<void>;
}

function warnIfOff(what: string, problems: string[]): void {
  if (problems.length > 0) console.warn(`quest-tracker: ${what} differs from the IPC types`, problems);
}

/** The bridge inside the Tauri app. */
export function tauriBridge(): Bridge {
  return {
    kind: 'tauri',
    holdToasts: false,
    async start(onView, onToast) {
      const offView = await listen<View>(VIEW_EVENT, (event) => {
        warnIfOff('a qt:view', viewProblems(event.payload));
        onView(event.payload);
      });
      const offToast = await listen<Toast>(TOAST_EVENT, (event) => {
        warnIfOff('a qt:toast', toastProblems(event.payload));
        onToast(event.payload);
      });
      // Subscribed first, so a view pushed while this call runs is not lost;
      // the later of the two simply wins.
      const view = await invoke<View>('get_view');
      warnIfOff('get_view', viewProblems(view));
      onView(view);
      return () => {
        offView();
        offToast();
      };
    },
    async command(name, args) {
      try {
        await invoke(name, args as unknown as Record<string, unknown>);
      } catch (error) {
        console.error(`quest-tracker: ${name} failed`, error);
      }
    },
  };
}
