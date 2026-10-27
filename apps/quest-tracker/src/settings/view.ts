// The Settings window's view, mirrored from `qt-app`'s `AppSettingsView`
// (`crates/qt-app/src/updates.rs`), field names snake_case exactly as serde
// writes them in `qt:app-settings` and the answer to `get_app_settings`. The
// window renders these values; whether a check runs, what it found and which
// version is newer are the app's to say.
//
// The checker below walks the same field lists the types declare, so the
// window's tests can hold every fixture under `fixtures/app-settings/`
// against it: a field the Rust side adds, drops or renames, or a state it
// invents, fails them. It follows `../ipc.ts`'s checker for `View`.

export const UPDATE_STATES = ['idle', 'checking', 'up-to-date', 'available', 'no-release', 'failed'] as const;
export type UpdateState = (typeof UPDATE_STATES)[number];

/** Where the update check stands. */
export interface UpdateView {
  state: UpdateState;
  /** The status line, written by the app. */
  text: string;
  /** The newer version known, if any: what *Open the release page* opens. Kept by a failed check. */
  newer: string | null;
}

/** What the Settings window renders. */
export interface AppSettingsView {
  /** The running build's version. */
  version: string;
  /** "Automatically check for updates" (off by default). */
  check_updates: boolean;
  update: UpdateView;
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
const appSettings = record<AppSettingsView>({ version: str, check_updates: bool, update });

/** Every way `value` differs from an `AppSettingsView`; empty when it is one. */
export function appSettingsProblems(value: unknown): string[] {
  const out: string[] = [];
  appSettings(value, 'app_settings', out);
  return out;
}

export function isAppSettingsView(value: unknown): value is AppSettingsView {
  return appSettingsProblems(value).length === 0;
}
