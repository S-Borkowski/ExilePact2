// The Settings window's door to the Rust shell, as `../bridge.ts` is the
// overlay's. Rust decides, the window renders: the shell sends a fresh
// `AppSettingsView` on `qt:app-settings`, to this window only, after every
// change (the setting, a check starting, a check ending); the window calls a
// command and waits for the next one, never polling.
//
// The page never reaches the network itself. A check is made by the app,
// only when a command or the setting asks for it, and the release page opens
// in the default browser.

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { SettingsPatch } from '../bridge';
import { type AppSettingsView, appSettingsProblems } from './view';

export const APP_SETTINGS_EVENT = 'qt:app-settings';

/** Every command the window sends, with its arguments. `get_app_settings` is the bridge's own. */
export interface SettingsCommandArgs {
  /** The setting; the window only ever sends `check_updates`. */
  update_settings: { patch: Pick<SettingsPatch, 'check_updates'> };
  /** Starts a manual check; the views it causes arrive on `qt:app-settings`. */
  check_for_updates: Record<string, never>;
  /** Opens the newer version's release page in the default browser. */
  open_release_page: Record<string, never>;
}
export type SettingsCommand = keyof SettingsCommandArgs;
export type SettingsSend = <C extends SettingsCommand>(name: C, args: SettingsCommandArgs[C]) => void;

export interface SettingsBridge {
  /** `tauri` inside the app, `preview` in a browser over the dev server. */
  readonly kind: 'tauri' | 'preview';
  /** Subscribes to `qt:app-settings`, then delivers the current view; resolves to an unsubscribe. */
  start(onView: (view: AppSettingsView) => void): Promise<() => void>;
  /** Sends one command. A failure is logged; the next view shows the state either way. */
  command<C extends SettingsCommand>(name: C, args: SettingsCommandArgs[C]): Promise<void>;
}

function warnIfOff(what: string, problems: string[]): void {
  if (problems.length > 0) console.warn(`quest-tracker settings: ${what} differs from the IPC types`, problems);
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
      // `check_for_updates` answers with a view too; it is not rendered: the
      // events carry every change in order, and an answer that arrived after
      // the check's own end would put "Checking…" back.
      try {
        await invoke(name, args as unknown as Record<string, unknown>);
      } catch (error) {
        console.error(`quest-tracker settings: ${name} failed`, error);
      }
    },
  };
}
