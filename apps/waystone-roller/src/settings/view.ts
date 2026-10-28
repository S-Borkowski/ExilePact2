// The Settings window's view, mirrored from the `AppSettingsView` the
// waystone roller sends (`ep-update`'s, `apps/shared/crates/ep-update/src/view.rs`,
// re-exported by `crates/wr-app`), field names snake_case exactly as serde
// writes them in `wr:app-settings` and the answer to `get_app_settings`. The
// window renders these values; whether a check runs, what it found, which
// version is newer and where an install stands are the app's to say.
//
// The checker below walks the same field lists the types declare, so the
// window's tests can hold every fixture under `fixtures/app-settings/`
// against it: a field the Rust side adds, drops or renames, or a state it
// invents, fails them. It follows `../ipc.ts`'s checker for `View`, and the
// quest tracker's copy of this file.

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

/** What the Settings window renders. */
export interface AppSettingsView {
  /** The running build's version. */
  version: string;
  /** "Automatically check for updates" (off by default). */
  check_updates: boolean;
  update: UpdateView;
  install: InstallView;
}

// One field spec per type: `Spec<T>` makes the compiler refuse a spec that
// misses or invents a field of its interface.

type Check = (value: unknown, path: string, out: string[]) => void;
type Spec<T> = { [K in keyof T]-?: Check };

const isObject = (v: unknown): v is Record<string, unknown> =>
  typeof v === 'object' && v !== null && !Array.isArray(v);

const str: Check = (v, p, out) => {
  if (typeof v !== 'string') out.push(`${p}: expected a string`);
};
const count: Check = (v, p, out) => {
  if (typeof v !== 'number' || !Number.isInteger(v) || v < 0) out.push(`${p}: expected a whole number`);
};
const bool: Check = (v, p, out) => {
  if (typeof v !== 'boolean') out.push(`${p}: expected a boolean`);
};
const oneOf =
  (values: readonly string[]): Check =>
  (v, p, out) => {
    if (typeof v !== 'string' || !values.includes(v)) out.push(`${p}: ${JSON.stringify(v)} is not one of ${values.join('|')}`);
  };
const nullable =
  (inner: Check): Check =>
  (v, p, out) => {
    if (v !== null) inner(v, p, out);
  };
function record<T>(spec: Spec<T>): Check {
  return (v, p, out) => {
    if (!isObject(v)) {
      out.push(`${p}: expected an object`);
      return;
    }
    const s = spec as Record<string, Check>;
    for (const key of Object.keys(s)) {
      if (!(key in v)) out.push(`${p}.${key}: missing`);
      else s[key](v[key], `${p}.${key}`, out);
    }
    for (const key of Object.keys(v)) {
      if (!(key in s)) out.push(`${p}.${key}: not in the TypeScript types`);
    }
  };
}

const update = record<UpdateView>({ state: oneOf(UPDATE_STATES), text: str, newer: nullable(str) });
const progress = record<InstallProgress>({ bytes: count, total: nullable(count) });
const install = record<InstallView>({ state: oneOf(INSTALL_STATES), text: str, progress: nullable(progress) });
const appSettings = record<AppSettingsView>({ version: str, check_updates: bool, update, install });

/** Every way `value` differs from an `AppSettingsView`; empty when it is one. */
export function appSettingsProblems(value: unknown): string[] {
  const out: string[] = [];
  appSettings(value, 'app_settings', out);
  return out;
}

export function isAppSettingsView(value: unknown): value is AppSettingsView {
  return appSettingsProblems(value).length === 0;
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
