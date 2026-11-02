// The one door between the UI and the Rust shell. Rust decides, the UI
// renders: the shell pushes a fresh `View` on `qt:view` after every change
// and a `RowChange` on `qt:row-change` for each live change of a quest's
// row; the UI calls commands and waits for the next
// `qt:view`, never polling and never patching its copy of the view.
//
// Command argument keys are camelCase (`questId`, `stepId`): Tauri 2 maps a
// Rust command's snake_case parameters to camelCase keys by default.

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import {
  type Mode,
  type Theme,
  type QuestAction,
  type Scale,
  type RowChange,
  type View,
  type ViewKind,
  rowChangeProblems,
  viewProblems,
} from './ipc';

export const VIEW_EVENT = 'qt:view';
export const ROW_CHANGE_EVENT = 'qt:row-change';

/**
 * The edge or corner a resize drag starts from: the panel's eight handles in
 * edit mode, named as Tauri's `startResizeDragging` names them.
 */
export const RESIZE_DIRECTIONS = [
  'North',
  'NorthEast',
  'East',
  'SouthEast',
  'South',
  'SouthWest',
  'West',
  'NorthWest',
] as const;
export type ResizeDirection = (typeof RESIZE_DIRECTIONS)[number];

/** The frames `open_frame` opens over the panel. */
export type OverlayFrame = 'character-picker' | 'settings';

/** A partial settings object for `update_settings`; Rust refuses unknown keys (`dock` among them since Live 2, and the hide-under-panels switch since 0.1.1). */
export interface SettingsPatch {
  theme?: Theme;
  opacity?: number;
  scale?: Scale;
  show_skipped?: boolean;
  only_worth_it?: boolean;
  log_path?: string | null;
  hotkey?: string;
  screen_reading?: boolean;
  check_updates?: boolean;
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
  /** Subscribes to both events, then delivers the current view; resolves to an unsubscribe. */
  start(onView: (view: View) => void, onRowChange: (change: RowChange) => void): Promise<() => void>;
  /** Sends one command. A failure is logged; the next `qt:view` shows the state either way. */
  command<C extends Command>(name: C, args: CommandArgs[C]): Promise<void>;
  /**
   * Starts moving the overlay window with the pressed mouse button (edit
   * mode's header). The shell keeps a move made in edit mode. A no-op in the
   * preview; the test bridge records it.
   */
  startDrag(): void;
  /**
   * Starts resizing the overlay window from `direction`'s edge or corner
   * with the pressed mouse button (edit mode's panel edges). The shell keeps
   * a size made in edit mode. The preview and the test bridge record it.
   */
  startResize(direction: ResizeDirection): void;
}

function warnIfOff(what: string, problems: string[]): void {
  if (problems.length > 0) console.warn(`quest-tracker: ${what} differs from the IPC types`, problems);
}

/** The bridge inside the Tauri app. */
export function tauriBridge(): Bridge {
  return {
    kind: 'tauri',
    async start(onView, onRowChange) {
      const offView = await listen<View>(VIEW_EVENT, (event) => {
        warnIfOff('a qt:view', viewProblems(event.payload));
        onView(event.payload);
      });
      const offRowChange = await listen<RowChange>(ROW_CHANGE_EVENT, (event) => {
        warnIfOff('a qt:row-change', rowChangeProblems(event.payload));
        onRowChange(event.payload);
      });
      // Subscribed first, so a view pushed while this call runs is not lost;
      // the later of the two simply wins.
      const view = await invoke<View>('get_view');
      warnIfOff('get_view', viewProblems(view));
      onView(view);
      return () => {
        offView();
        offRowChange();
      };
    },
    async command(name, args) {
      try {
        await invoke(name, args as unknown as Record<string, unknown>);
      } catch (error) {
        console.error(`quest-tracker: ${name} failed`, error);
      }
    },
    startDrag() {
      getCurrentWindow()
        .startDragging()
        .catch((error) => console.error('quest-tracker: the window drag failed', error));
    },
    startResize(direction) {
      getCurrentWindow()
        .startResizeDragging(direction)
        .catch((error) => console.error('quest-tracker: the window resize failed', error));
    },
  };
}
