// The Settings window's door to the Rust shell, as `../bridge.ts` is the
// main window's. Rust decides, the window renders: the shell sends a fresh
// `AppSettingsView` on `wr:app-settings`, to this window only, after every
// change (the setting, a check starting or ending, an install's progress);
// the window calls a command and waits for the next one, never polling.
//
// The page never reaches the network itself. A check, a download and an
// install are made by the app, only when a command (or, for a check alone,
// the setting) asks for it; nothing here opens a page or a browser.

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { APP_SETTINGS_EVENT, type SettingsCommandArgs as ShellCommandArgs } from '../bridge';
import { type AppSettingsView, appSettingsProblems } from './view';

export { APP_SETTINGS_EVENT };

/** Every command the window sends, with its arguments (`../bridge.ts`'s list); `get_app_settings` is the bridge's own. */
export type SettingsCommandArgs = Omit<ShellCommandArgs, 'get_app_settings'>;
export type SettingsCommand = keyof SettingsCommandArgs;
/** How a section sends a command: fire and forget, the next view answers. */
export type SettingsSend = <C extends SettingsCommand>(name: C, args: SettingsCommandArgs[C]) => void;

export interface SettingsBridge {
  /** `tauri` inside the app, `preview` in a browser over the dev server. */
  readonly kind: 'tauri' | 'preview';
  /** Subscribes to `wr:app-settings`, then delivers the current view; resolves to an unsubscribe. */
  start(onView: (view: AppSettingsView) => void): Promise<() => void>;
  /** Sends one command. A refusal is logged; the next view shows the state either way. */
  command<C extends SettingsCommand>(name: C, args: SettingsCommandArgs[C]): Promise<void>;
}

function warnIfOff(what: string, problems: string[]): void {
  if (problems.length > 0) console.warn(`waystone-roller settings: ${what} differs from the IPC types`, problems);
}

/** The bridge inside the Tauri app. */
export function tauriSettingsBridge(): SettingsBridge {
  return {
    kind: 'tauri',
    async start(onView) {
      const off = await listen<AppSettingsView>(APP_SETTINGS_EVENT, (event) => {
        warnIfOff(`a ${APP_SETTINGS_EVENT}`, appSettingsProblems(event.payload));
        onView(event.payload);
      });
      // Subscribed first, so a view sent while this call runs is not lost;
      // the later of the two simply wins.
      const view = await invoke<AppSettingsView>('get_app_settings');
      warnIfOff('get_app_settings', appSettingsProblems(view));
      onView(view);
      return off;
    },
    async command(name, args) {
      // A command's own answer is not rendered: the events carry every
      // change in order, and an answer that arrived after a later event
      // (a check's end, an install's progress) would put an old state back.
      try {
        await invoke(name, args as unknown as Record<string, unknown>);
      } catch (error) {
        console.error(`waystone-roller settings: ${name} failed`, error);
      }
    },
  };
}
