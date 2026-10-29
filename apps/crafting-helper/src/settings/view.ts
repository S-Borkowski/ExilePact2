// The Settings window's view, mirrored from `ch-app`'s `SettingsView`
// (`crates/ch-app/src/view.rs`): `ep-update`'s `AppSettingsView`
// (`apps/shared/crates/ep-update/src/view.rs`) flattened in, then the
// hotkey, what the last hotkey change said, the blocked currencies, the
// overlay's position and the screen watch.
// Field names are snake_case exactly as serde writes them in
// `ch:app-settings` and the answer to `get_app_settings`. The window renders
// these values; whether a check runs, what it found, which version is newer,
// where an install stands, which hotkey holds and what the list keeps are
// the app's to say.
//
// The checker below walks the same field lists the types declare, so the
// window's tests can hold every fixture under `fixtures/app-settings/`
// against it: a field the Rust side adds, drops or renames, or a state it
// invents, fails them. It follows `../ipc.ts`'s checker for `View`, and the
// waystone roller's copy of this file.

import {
  type Check,
  OVERLAY_POSITIONS,
  type OverlayPosition,
  type StatusLine,
  list,
  nullable,
  oneOf,
  record,
  statusLine,
  str,
} from '../ipc';

export const UPDATE_STATES = ['idle', 'checking', 'up-to-date', 'available', 'no-release', 'failed'] as const;
export type UpdateState = (typeof UPDATE_STATES)[number];

/** Where the update check stands. */
export interface UpdateView {
  state: UpdateState;
  /** The status line, written by the app. */
  text: string;
  /** The newer version known, if any: what the install button installs. Kept by a failed check. */
  newer: string | null;
}

export const INSTALL_STATES = ['none', 'unsigned', 'ready', 'downloading', 'verifying', 'restarting', 'failed'] as const;
export type InstallState = (typeof INSTALL_STATES)[number];

/** How far a download is. */
export interface InstallProgress {
  bytes: number;
  /** The size the release lists, which the download must match exactly. */
  total: number | null;
}

/**
 * Where an install of the newer version stands. `none`: no newer version;
 * `unsigned`: its release cannot be installed (no button); `ready`: the
 * install button, labelled `text`; `downloading` (with `progress`, and
 * *Cancel*), `verifying`, `restarting`: the status line; `failed`: its
 * reason, and *Try again*.
 */
export interface InstallView {
  state: InstallState;
  /** `ready`: the button's label; `none`: empty; otherwise the status line, written by the app. */
  text: string;
  /** Only while `downloading`. */
  progress: InstallProgress | null;
}

/** `ep-update`'s part of the view: the update check and the install. */
export interface AppSettingsView {
  /** The running build's version. */
  version: string;
  /** "Automatically check for updates" (off by default). */
  check_updates: boolean;
  update: UpdateView;
  install: InstallView;
}

/** What the Settings window renders: `ep-update`'s fields, then the app's own. */
export interface SettingsView extends AppSettingsView {
  /** The global hotkey registered now, e.g. `Ctrl+Alt+X`. */
  hotkey: string;
  /** What the last hotkey change, or the start, said about the hotkey; null when nothing. */
  hotkey_status: StatusLine | null;
  /** The currency ids the player blocked, in the order the app keeps them. */
  blocked_currencies: string[];
  /** Where the overlay sits over the game (`top-center` by default). */
  overlay_position: OverlayPosition;
  /** Whether the app reads the hovered item from the screen while a craft is guiding (on by default). */
  screen_watch: boolean;
}

// One field spec per type, from `../ipc.ts`'s checks: `Spec<T>` makes the
// compiler refuse a spec that misses or invents a field of its interface.

const count: Check = (v, p, out) => {
  if (typeof v !== 'number' || !Number.isInteger(v) || v < 0) out.push(`${p}: expected a whole number`);
};
const bool: Check = (v, p, out) => {
  if (typeof v !== 'boolean') out.push(`${p}: expected a boolean`);
};

const update = record<UpdateView>({ state: oneOf(UPDATE_STATES), text: str, newer: nullable(str) });
const progress = record<InstallProgress>({ bytes: count, total: nullable(count) });
const install = record<InstallView>({ state: oneOf(INSTALL_STATES), text: str, progress: nullable(progress) });
const settings = record<SettingsView>({
  version: str,
  check_updates: bool,
  update,
  install,
  hotkey: str,
  hotkey_status: nullable(statusLine),
  blocked_currencies: list(str),
  overlay_position: oneOf(OVERLAY_POSITIONS),
  screen_watch: bool,
});

/** Every way `value` differs from a `SettingsView`; empty when it is one. */
export function settingsProblems(value: unknown): string[] {
  const out: string[] = [];
  settings(value, 'app_settings', out);
  return out;
}

export function isSettingsView(value: unknown): value is SettingsView {
  return settingsProblems(value).length === 0;
}

/** A byte count in words: `39 KB` under a megabyte, `3.3 MB` from one. */
export function formatBytes(bytes: number): string {
  const MB = 1024 * 1024;
  return bytes < MB ? `${Math.round(bytes / 1024)} KB` : `${(bytes / MB).toFixed(1)} MB`;
}

/** The download's numbers in words: `20 KB of 39 KB`, or what came when the total is unknown. */
export function progressText(p: InstallProgress): string {
  return p.total === null ? formatBytes(p.bytes) : `${formatBytes(p.bytes)} of ${formatBytes(p.total)}`;
}
