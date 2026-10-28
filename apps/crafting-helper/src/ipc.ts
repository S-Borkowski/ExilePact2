// The IPC contract, mirrored from `ch-app`'s `View`
// (`crates/ch-app/src/view.rs`), which `serde` serialises. Field names are
// snake_case exactly as the shell sends them in the `ch:view` event and the
// `get_view` command. The pages render these values; whether a grab passed,
// how many lines it has and what the status says are the app's to decide.
//
// The runtime checker below walks the same field lists the types declare,
// so `ipc.test.ts` holds every fixture under `fixtures/views/` against it,
// and the Rust side writes the same fixtures from `View`, whose types deny
// unknown fields (`fixtures_are_current`): a field either side adds, drops
// or renames fails a test.

export const GRAB_SOURCES = ['shortcut', 'paste'] as const;
/** Where a grab's text came from: the global hotkey's one clipboard read, or a paste into the main window. */
export type GrabSource = (typeof GRAB_SOURCES)[number];

export const STATUS_KINDS = ['info', 'error'] as const;
export type StatusKind = (typeof STATUS_KINDS)[number];

/** The last item text grabbed, kept in memory only. */
export interface GrabView {
  /** The text, normalised by the app: LF line breaks, no trailing blank lines. */
  text: string;
  line_count: number;
  source: GrabSource;
  /** The first line starts like the game's copied item text. */
  looks_like_item: boolean;
}

export interface StatusLine {
  kind: StatusKind;
  text: string;
}

/** The overlay window's state. */
export interface OverlayView {
  /** True only while a craft is active; the shell shows or hides the window by it. */
  visible: boolean;
  /** The one line the overlay shows. */
  line: string;
}

/** Everything the main window and the overlay show. */
export interface View {
  /** The global hotkey, e.g. `Ctrl+Alt+X`. */
  hotkey: string;
  /** The last grab; null until one passes. */
  grab: GrabView | null;
  status: StatusLine | null;
  overlay: OverlayView;
  /** The newer version a check found this run, e.g. `1.3.0`, which the header's Settings button names; null while none is known. */
  update_available: string | null;
}

// ---------------------------------------------------------------------------
// Runtime checks: one field spec per type. `Spec<T>` makes the compiler
// refuse a spec that misses or invents a field of its interface, so the
// checks and the types cannot drift apart.

export type Check = (value: unknown, path: string, out: string[]) => void;
export type Spec<T> = { [K in keyof T]-?: Check };

const isObject = (v: unknown): v is Record<string, unknown> =>
  typeof v === 'object' && v !== null && !Array.isArray(v);

export const str: Check = (v, p, out) => {
  if (typeof v !== 'string') out.push(`${p}: expected a string`);
};
const uint: Check = (v, p, out) => {
  if (typeof v !== 'number' || !Number.isInteger(v) || v < 0) out.push(`${p}: expected an unsigned integer`);
};
const bool: Check = (v, p, out) => {
  if (typeof v !== 'boolean') out.push(`${p}: expected a boolean`);
};
export const oneOf =
  (values: readonly string[]): Check =>
  (v, p, out) => {
    if (typeof v !== 'string' || !values.includes(v)) out.push(`${p}: ${JSON.stringify(v)} is not one of ${values.join('|')}`);
  };
export const nullable =
  (inner: Check): Check =>
  (v, p, out) => {
    if (v !== null) inner(v, p, out);
  };
export const list =
  (inner: Check): Check =>
  (v, p, out) => {
    if (!Array.isArray(v)) {
      out.push(`${p}: expected a list`);
      return;
    }
    v.forEach((item, i) => inner(item, `${p}[${i}]`, out));
  };
export function record<T>(spec: Spec<T>): Check {
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

const grab = record<GrabView>({ text: str, line_count: uint, source: oneOf(GRAB_SOURCES), looks_like_item: bool });
export const statusLine = record<StatusLine>({ kind: oneOf(STATUS_KINDS), text: str });
const overlay = record<OverlayView>({ visible: bool, line: str });
const view = record<View>({
  hotkey: str,
  grab: nullable(grab),
  status: nullable(statusLine),
  overlay,
  update_available: nullable(str),
});

/** Every way `value` differs from a `View`; empty when it is one. */
export function viewProblems(value: unknown): string[] {
  const out: string[] = [];
  view(value, 'view', out);
  return out;
}

export function isView(value: unknown): value is View {
  return viewProblems(value).length === 0;
}
