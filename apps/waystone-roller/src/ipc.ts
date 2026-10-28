// The IPC contract, mirrored from `wr-app`'s `View`, which `serde`
// serialises. Field names are snake_case exactly as the shell sends them in
// the `wr:view` event and the `get_view` command. The UI renders these
// values; it never builds, measures or shortens the search itself.
//
// The runtime checker below walks the same field lists the types declare,
// so `ipc.test.ts` holds every fixture under `fixtures/views/` against it,
// and the Rust side reads the same fixtures into `View` with unknown fields
// denied (`view_fixtures_round_trip_through_the_view_type`): a field either
// side adds, drops or renames fails a test.

export const MOD_GROUPS = ['waystone', 'desecrated'] as const;
export type ModGroup = (typeof MOD_GROUPS)[number];

export const MOD_KINDS = ['prefix', 'suffix'] as const;
export type ModKind = (typeof MOD_KINDS)[number];

export const WANT_MODES = ['any', 'all'] as const;
export type WantMode = (typeof WANT_MODES)[number];

export const CORRUPTIONS = ['any', 'corrupted', 'uncorrupted'] as const;
export type Corruption = (typeof CORRUPTIONS)[number];

export const RESULT_STATES = ['ok', 'empty', 'too_long', 'unmatchable'] as const;
export type ResultState = (typeof RESULT_STATES)[number];

/** What a part of the search filters on, in the order the search writes them. */
export const PART_KINDS = ['rarity', 'tier', 'want', 'avoid', 'corruption'] as const;
export type PartKind = (typeof PART_KINDS)[number];

export const STATUS_KINDS = ['info', 'error'] as const;
export type StatusKind = (typeof STATUS_KINDS)[number];

/** The tier range a waystone can have. */
export const TIER_MIN = 1;
export const TIER_MAX = 16;

/** The game's limit on the search box, in characters. */
export const SEARCH_LIMIT = 250;

export interface DataInfo {
  /** The page the mod data was read from. */
  source: string;
  /** The fetch date, as the data file writes it (ISO). */
  fetched: string;
}

/** One modifier family the player can want or avoid. */
export interface ModView {
  id: string;
  group: ModGroup;
  kind: ModKind;
  /** The desecrated affix's name; null for an ordinary waystone mod. */
  name: string | null;
  /** The first line, numbers shown as `#`. */
  label: string;
  /** Every line of the family, numbers shown as `#`. */
  lines: string[];
  /** How many tiers the family has. */
  tiers: number;
  /** The text the search uses to find this family alone; null when none tells it apart. */
  token: string | null;
  /** Why the family has no token of its own; null when it has one. */
  why_not: string | null;
}

export interface Rarity {
  normal: boolean;
  magic: boolean;
  rare: boolean;
}

export interface Selection {
  want: string[];
  want_mode: WantMode;
  avoid: string[];
  tier_min: number;
  tier_max: number;
  rarity: Rarity;
  corruption: Corruption;
}

export interface SearchPart {
  what: PartKind;
  text: string;
  length: number;
}

export interface SearchResult {
  state: ResultState;
  /** The search string; empty unless `state` is `ok`. */
  regex: string;
  /** The string's length; when too long, the length it would have had. */
  length: number;
  limit: number;
  /** Each part with its length (shown when too long). */
  parts: SearchPart[];
  /** The wanted mods the search cannot tell apart (state `unmatchable`). */
  unmatchable: string[];
}

export interface StatusLine {
  kind: StatusKind;
  text: string;
}

export interface View {
  data: DataInfo;
  mods: ModView[];
  selection: Selection;
  result: SearchResult;
  /** The global shortcut that copies the search, e.g. `Ctrl+Alt+W`. */
  hotkey: string;
  status: StatusLine | null;
  /** The newer version a check found this run, e.g. `1.3.0`, which the header's Settings button names; null while none is known. */
  update_available: string | null;
}

// ---------------------------------------------------------------------------
// Runtime checks: one field spec per type. `Spec<T>` makes the compiler
// refuse a spec that misses or invents a field of its interface, so the
// checks and the types cannot drift apart.

type Check = (value: unknown, path: string, out: string[]) => void;
type Spec<T> = { [K in keyof T]-?: Check };

const isObject = (v: unknown): v is Record<string, unknown> =>
  typeof v === 'object' && v !== null && !Array.isArray(v);

const str: Check = (v, p, out) => {
  if (typeof v !== 'string') out.push(`${p}: expected a string`);
};
const uint: Check = (v, p, out) => {
  if (typeof v !== 'number' || !Number.isInteger(v) || v < 0) out.push(`${p}: expected an unsigned integer`);
};
const tier: Check = (v, p, out) => {
  if (typeof v !== 'number' || !Number.isInteger(v) || v < TIER_MIN || v > TIER_MAX) {
    out.push(`${p}: ${JSON.stringify(v)} is not a tier from ${TIER_MIN} to ${TIER_MAX}`);
  }
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
const list =
  (inner: Check): Check =>
  (v, p, out) => {
    if (!Array.isArray(v)) {
      out.push(`${p}: expected a list`);
      return;
    }
    v.forEach((item, i) => inner(item, `${p}[${i}]`, out));
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

const dataInfo = record<DataInfo>({ source: str, fetched: str });
const mod = record<ModView>({
  id: str,
  group: oneOf(MOD_GROUPS),
  kind: oneOf(MOD_KINDS),
  name: nullable(str),
  label: str,
  lines: list(str),
  tiers: uint,
  token: nullable(str),
  why_not: nullable(str),
});
const rarity = record<Rarity>({ normal: bool, magic: bool, rare: bool });
const selection = record<Selection>({
  want: list(str),
  want_mode: oneOf(WANT_MODES),
  avoid: list(str),
  tier_min: tier,
  tier_max: tier,
  rarity,
  corruption: oneOf(CORRUPTIONS),
});
const part = record<SearchPart>({ what: oneOf(PART_KINDS), text: str, length: uint });
const result = record<SearchResult>({
  state: oneOf(RESULT_STATES),
  regex: str,
  length: uint,
  limit: uint,
  parts: list(part),
  unmatchable: list(str),
});
const statusLine = record<StatusLine>({ kind: oneOf(STATUS_KINDS), text: str });
const view = record<View>({
  data: dataInfo,
  mods: list(mod),
  selection,
  result,
  hotkey: str,
  status: nullable(statusLine),
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
