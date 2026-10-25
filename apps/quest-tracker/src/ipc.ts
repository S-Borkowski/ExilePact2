// The IPC contract, mirrored from the Rust types that `serde` serialises
// (`qt-viewmodel` and `qt-engine`). Field names are snake_case exactly as the
// shell sends them in `qt:view` and `qt:row-change`. The UI renders these values;
// it never re-derives a status, a filter or a frame from them.
//
// The runtime checkers below walk the same field lists the types declare, so
// `ipc.test.ts` can hold every fixture under `fixtures/viewmodels/` against
// them: a field the Rust side adds, drops or renames fails that test.

export const FRAMES = [
  'panel-campaign',
  'panel-next-up',
  'collapsed',
  'quest-detail',
  'character-picker',
  'settings',
  'empty-no-log',
  'empty-waiting',
  'empty-no-character',
  'empty-act-done',
] as const;
export type Frame = (typeof FRAMES)[number];

export const MODES = ['play', 'edit'] as const;
export type Mode = (typeof MODES)[number];

export const VIEW_KINDS = ['campaign', 'next_up'] as const;
export type ViewKind = (typeof VIEW_KINDS)[number];

export const STATUS_KINDS = ['ok', 'info', 'warn', 'error'] as const;
export type StatusKind = (typeof STATUS_KINDS)[number];

export const QUEST_STATUSES = ['locked', 'available', 'in_progress', 'done', 'skipped', 'unknown'] as const;
export type QuestStatus = (typeof QUEST_STATUSES)[number];

export const WORTHS = ['yes', 'optional', 'no'] as const;
export type Worth = (typeof WORTHS)[number];

export const DETECTIONS = ['auto', 'partial', 'manual'] as const;
export type Detection = (typeof DETECTIONS)[number];

export const REWARD_CATEGORIES = [
  'passive_points',
  'weapon_set_points',
  'spirit',
  'resistance',
  'life',
  'mana',
  'attribute',
  'charm',
  'flask',
  'defences',
  'other_stat',
  'skill_gem',
  'support_gem',
  'ascendancy',
  'item',
  'unlock',
  'story',
] as const;
export type RewardCategory = (typeof REWARD_CATEGORIES)[number];

export const STEP_PHASES = ['pending', 'current', 'done'] as const;
export type StepPhase = (typeof STEP_PHASES)[number];

export const DOCKS = ['left', 'right'] as const;
export type Dock = (typeof DOCKS)[number];

export const SCALES = ['auto', '1', '1.33', '2'] as const;
export type Scale = (typeof SCALES)[number];

export const ROW_CHANGE_KINDS = ['available', 'step_done', 'quest_done'] as const;
export type RowChangeKind = (typeof ROW_CHANGE_KINDS)[number];

export const QUEST_ACTIONS = ['mark_done', 'unmark_done', 'skip', 'unskip', 'reset'] as const;
export type QuestAction = (typeof QUEST_ACTIONS)[number];

export interface StatusLine {
  kind: StatusKind;
  text: string;
}

export interface CharacterView {
  name: string;
  class: string;
  level: number;
  own: boolean;
  pinned: boolean;
  active: boolean;
  /** The new character from The Riverbank no level-up has named yet: an empty name, level 1 (shown as "New character"). */
  provisional: boolean;
}

export interface RewardView {
  category: RewardCategory;
  /** An icon id of the brief's export contract. */
  icon: string;
  text: string;
}

export interface QuestRow {
  id: string;
  name: string;
  act: number;
  status: QuestStatus;
  worth: Worth;
  detection: Detection;
  /** The manual-tick marker: detection `partial` or `manual`. */
  needs_tick: boolean;
  rewards: RewardView[];
  /** The step to do next, for an in-progress quest only. */
  current_step: string | null;
  /** Done because of a later event the log recorded, not the quest's own signal, and not marked done by hand: the "probably done" mark. */
  inferred: boolean;
}

export interface ActView {
  act: number;
  title: string;
  done: number;
  total: number;
  expanded: boolean;
  has_available: boolean;
  quests: QuestRow[];
}

export interface StepView {
  id: string;
  title: string;
  text: string;
  state: StepPhase;
  ticked: boolean;
  /** Shown done though neither its own signal fired nor the player ticked it: the "probably done" mark. */
  inferred: boolean;
}

export interface QuestDetail {
  row: QuestRow;
  giver: string | null;
  zones: string[];
  worth_reason: string;
  steps: StepView[];
  source_url: string;
  fetched: string;
}

export interface Settings {
  /** The panel background's opacity, 0.3-1; text and icons stay opaque. */
  opacity: number;
  dock: Dock;
  scale: Scale;
  show_skipped: boolean;
  only_worth_it: boolean;
  log_path: string | null;
  hotkey: string;
}

export interface View {
  schema: number;
  frame: Frame;
  mode: Mode;
  view_kind: ViewKind;
  status: StatusLine | null;
  character: CharacterView | null;
  characters: CharacterView[];
  acts: ActView[];
  next_up: QuestRow[];
  detail: QuestDetail | null;
  settings: Settings;
  available_worth_count: number;
}

/** One live change of one quest's row, sent as `qt:row-change`. */
export interface RowChange {
  /** Increasing, from 1, for the shell's lifetime. */
  id: number;
  kind: RowChangeKind;
  quest_id: string;
  /** The title of the step just completed, or null. */
  step_done: string | null;
  /** The title of the step now current, or null. */
  step_next: string | null;
  /** One sentence for the screen reader, naming the quest and what changed. */
  text: string;
}

/** The schema version this UI renders. */
export const VIEW_SCHEMA = 1;

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
const num: Check = (v, p, out) => {
  if (typeof v !== 'number' || !Number.isFinite(v)) out.push(`${p}: expected a number`);
};
const uint: Check = (v, p, out) => {
  if (typeof v !== 'number' || !Number.isInteger(v) || v < 0) out.push(`${p}: expected an unsigned integer`);
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

const statusLine = record<StatusLine>({ kind: oneOf(STATUS_KINDS), text: str });
const character = record<CharacterView>({
  name: str,
  class: str,
  level: uint,
  own: bool,
  pinned: bool,
  active: bool,
  provisional: bool,
});
const reward = record<RewardView>({ category: oneOf(REWARD_CATEGORIES), icon: str, text: str });
const questRow = record<QuestRow>({
  id: str,
  name: str,
  act: uint,
  status: oneOf(QUEST_STATUSES),
  worth: oneOf(WORTHS),
  detection: oneOf(DETECTIONS),
  needs_tick: bool,
  rewards: list(reward),
  current_step: nullable(str),
  inferred: bool,
});
const act = record<ActView>({
  act: uint,
  title: str,
  done: uint,
  total: uint,
  expanded: bool,
  has_available: bool,
  quests: list(questRow),
});
const step = record<StepView>({ id: str, title: str, text: str, state: oneOf(STEP_PHASES), ticked: bool, inferred: bool });
const detail = record<QuestDetail>({
  row: questRow,
  giver: nullable(str),
  zones: list(str),
  worth_reason: str,
  steps: list(step),
  source_url: str,
  fetched: str,
});
const settings = record<Settings>({
  opacity: num,
  dock: oneOf(DOCKS),
  scale: oneOf(SCALES),
  show_skipped: bool,
  only_worth_it: bool,
  log_path: nullable(str),
  hotkey: str,
});
const view = record<View>({
  schema: uint,
  frame: oneOf(FRAMES),
  mode: oneOf(MODES),
  view_kind: oneOf(VIEW_KINDS),
  status: nullable(statusLine),
  character: nullable(character),
  characters: list(character),
  acts: list(act),
  next_up: list(questRow),
  detail: nullable(detail),
  settings,
  available_worth_count: uint,
});
const rowChange = record<RowChange>({
  id: uint,
  kind: oneOf(ROW_CHANGE_KINDS),
  quest_id: str,
  step_done: nullable(str),
  step_next: nullable(str),
  text: str,
});

/** Every way `value` differs from a `View`; empty when it is one. */
export function viewProblems(value: unknown): string[] {
  const out: string[] = [];
  view(value, 'view', out);
  return out;
}

/** Every way `value` differs from a `RowChange`; empty when it is one. */
export function rowChangeProblems(value: unknown): string[] {
  const out: string[] = [];
  rowChange(value, 'row_change', out);
  return out;
}

export function isView(value: unknown): value is View {
  return viewProblems(value).length === 0;
}

export function isRowChange(value: unknown): value is RowChange {
  return rowChangeProblems(value).length === 0;
}
