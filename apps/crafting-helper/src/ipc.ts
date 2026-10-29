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

export const OVERLAY_POSITIONS = ['top-left', 'top-center', 'top-right', 'bottom-left', 'bottom-center', 'bottom-right'] as const;
/** Where the overlay sits inside the game's client area: an edge's middle or a corner (`top-center` by default). */
export type OverlayPosition = (typeof OVERLAY_POSITIONS)[number];

/** The overlay window's state. */
export interface OverlayView {
  /** True only while a craft is guiding and the game's window is in front; the shell shows or hides the window by it. */
  visible: boolean;
  /** Where it sits, so the page anchors its panel to that edge. */
  position: OverlayPosition;
  /** What it shows: present exactly while a craft is guiding, shown or not. */
  guide: OverlayGuide | null;
}

/** The overlay's content: Rust's copy, rendered as sent (the labels are the page's). */
export interface OverlayGuide {
  status: GuideStatus;
  route_name: string;
  /** The current step's place in the route, from 1, while active or stuck. */
  step_number: number | null;
  /** Which try of that step this is, from 1. */
  try_number: number | null;
  /** The step's instruction; with replan and done, what to do instead. */
  instruction: string;
  /** The currency's name, then each omen's, while active. */
  uses: string[];
  /** The odds in words, while active. */
  odds: string | null;
  /** Why it is stuck or must be planned again: the first reason. */
  why: string | null;
  /** `On a hit: …`, while active or stuck. */
  on_hit: string | null;
  /** `On a miss: …`, while active or stuck. */
  on_miss: string | null;
  /** The last observed change; null before the first. */
  last: HistoryEntry | null;
  /** The screen watch's line, the guiding card's own; null with replan and done. */
  watch: WatchLineView | null;
}

/**
 * The screen watch's line (`ui-design.md` § "The screen watch's line"): what
 * the watch sees and what to do, the same in the guiding card and the
 * overlay's Screen row.
 */
export interface WatchLineView {
  text: string;
  /** It asks the player to do something: the warning colour. */
  warning: boolean;
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
  /** The craft window: always present, in one of four modes. */
  craft: CraftView;
}

// ---------------------------------------------------------------------------
// The craft window (`ch-app`'s `craft.rs` builds it). Every string below is
// Rust's: the odds, the fallback line, the floor labels, the branch lines,
// the history summaries and every reason are rendered as sent.

export const CRAFT_MODES = ['none', 'refused', 'planning', 'guiding'] as const;
/** Which part of the craft window shows; the page picks its layout from this alone. */
export type CraftMode = (typeof CRAFT_MODES)[number];

export const MOD_SIDES = ['prefix', 'suffix'] as const;
export type ModSide = (typeof MOD_SIDES)[number];

export const BLOCKED_FROM = ['settings', 'craft'] as const;
/** Where a blocked id of this craft comes from: the Settings list, or this craft alone. */
export type BlockedFrom = (typeof BLOCKED_FROM)[number];

export const GUIDE_STATUSES = ['active', 'stuck', 'replan', 'done'] as const;
export type GuideStatus = (typeof GUIDE_STATUSES)[number];

export const OUTCOME_WORDS = ['hit', 'miss', 'unexpected'] as const;
export type OutcomeWord = (typeof OUTCOME_WORDS)[number];

export const OUTCOME_SOURCES = ['grab', 'screen'] as const;
/** Where an observed change came from: a grab of the item's text, or the screen watch reading the hovered item twice alike. */
export type OutcomeSource = (typeof OUTCOME_SOURCES)[number];

/** The craft part of the view: exactly the part its mode names is set. */
export interface CraftView {
  mode: CraftMode;
  /** Why the last grab cannot be planned, in words (mode refused). */
  refusal: string | null;
  planning: PlanningView | null;
  guiding: GuidingView | null;
}

/** The parsed item. */
export interface CraftItemView {
  /** A rare item's name; null for a normal or magic one. */
  name: string | null;
  base_type: string | null;
  /** The item class as the game writes it (`Rings`). */
  class: string;
  rarity: string;
  item_level: number | null;
  /** Each explicit modifier, in text order. */
  modifiers: ItemModView[];
}

export interface ItemModView {
  side: ModSide;
  lines: string[];
  /** The data's tier (1 is the best); null when the data has none. */
  tier: number | null;
  fractured: boolean;
  desecrated: boolean;
  unrevealed: boolean;
  /** It is a chosen target at its floor or better. */
  meets_target: boolean;
}

export interface FloorOption {
  /** What `set_targets` sends: null for "as high as possible". */
  floor: number | null;
  label: string;
}

/** A modifier the item's base can roll at its item level. */
export interface TargetChoice {
  mod_id: string;
  side: ModSide;
  /** Its lines with each value as `#`, joined by ` / `. */
  text: string;
  best_tier: number;
  /** The default first, then one per reachable tier, best first. */
  floors: FloorOption[];
}

export interface TargetChoices {
  prefixes: TargetChoice[];
  suffixes: TargetChoice[];
  desecrated: TargetChoice[];
}

export interface ChosenTarget {
  mod_id: string;
  side: ModSide;
  desecrated: boolean;
  text: string;
  /** As chosen: null is "as high as possible". */
  floor: number | null;
  floor_in_force: number;
  best_tier: number;
}

export interface CraftBlockedEntry {
  id: string;
  name: string;
  from: BlockedFrom;
}

/** The blocked list this craft plans with. */
export interface CraftBlockedView {
  /** The effective list: the Settings ids first, then this craft's. */
  blocked: CraftBlockedEntry[];
  /** The Settings ids allowed for this craft. */
  allowed: CatalogueEntry[];
}

/** One step of a route, as the preview and the guide show it. */
export interface StepLineView {
  id: string;
  /** Its place in the route, from 1. */
  number: number;
  /**
   * What to do: for a step that places a target, Rust composes it from the
   * step's data (the currency, its omens, the side and the target
   * modifiers); otherwise the blueprint's text.
   */
  instruction: string;
  /** The target modifiers a hit of this step may place, as the planning list writes them; empty when it places none. */
  targets: string[];
  /** The currency's name, then each omen's; empty for a manual step. */
  uses: string[];
  /** The odds in words; null for a manual step. */
  odds: string | null;
  /** Set when an earlier alternative was passed over. */
  fallback: string | null;
}

/** The route the preview shows: what it targets and what to do, never the blueprint's notes. */
export interface RouteView {
  blueprint: string;
  name: string;
  /** Where the technique came from (attribution). */
  source: string;
  steps: StepLineView[];
}

export interface RouteChoice {
  blueprint: string;
  name: string;
}

export interface RejectedLine {
  blueprint: string;
  name: string;
  reason: string;
}

export interface PreviewView {
  route: RouteView | null;
  /** The other applicable routes, in the engine's order. */
  other_routes: RouteChoice[];
  /** Filled only when there is no route. */
  rejected: RejectedLine[];
}

export interface PlanningView {
  item: CraftItemView;
  choices: TargetChoices;
  /** The chosen targets, in the order chosen; a grab that starts planning ticks the item's own explicit modifiers. */
  targets: ChosenTarget[];
  blocked: CraftBlockedView;
  /** null with no target. */
  preview: PreviewView | null;
  /** `accept_craft` would start: there is a previewed route. */
  can_accept: boolean;
}

export interface CurrentStepView {
  step: StepLineView;
  /** `On a hit: …` */
  on_hit: string;
  /** `On a miss: …` */
  on_miss: string;
  /** Which try of this step this is, from 1. */
  try_number: number;
}

export interface HistoryEntry {
  number: number;
  step: string;
  instruction: string;
  result: OutcomeWord;
  /** What changed, without the outcome's word (`result` carries it). */
  summary: string;
  source: OutcomeSource;
}

export interface GuidingView {
  /** The item as last grabbed, or as the screen watch last read a hit or a miss. */
  item: CraftItemView;
  blueprint: string;
  route_name: string;
  status: GuideStatus;
  /** The current step, while active or stuck. */
  step: CurrentStepView | null;
  /** Why it is stuck or must be planned again. */
  reasons: string[];
  /** Oldest first. */
  history: HistoryEntry[];
  /** The screen watch's line, while active or stuck; null with replan and done. */
  watch: WatchLineView | null;
}

// ---------------------------------------------------------------------------
// The catalogue (`get_catalogue`, both windows): every crafting item the
// blocked list may name, grouped by kind, each group sorted by name.

export interface CatalogueEntry {
  id: string;
  /** The English name. */
  name: string;
}

export interface CatalogueGroup {
  /** The kind as the engine serialises it (`currency`, `omen`, ...). */
  kind: string;
  /** The group's heading. */
  label: string;
  entries: CatalogueEntry[];
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

const side = oneOf(MOD_SIDES);
const catalogueEntry = record<CatalogueEntry>({ id: str, name: str });
const itemMod = record<ItemModView>({
  side,
  lines: list(str),
  tier: nullable(uint),
  fractured: bool,
  desecrated: bool,
  unrevealed: bool,
  meets_target: bool,
});
const item = record<CraftItemView>({
  name: nullable(str),
  base_type: nullable(str),
  class: str,
  rarity: str,
  item_level: nullable(uint),
  modifiers: list(itemMod),
});
const floorOption = record<FloorOption>({ floor: nullable(uint), label: str });
const choice = record<TargetChoice>({ mod_id: str, side, text: str, best_tier: uint, floors: list(floorOption) });
const chosen = record<ChosenTarget>({
  mod_id: str,
  side,
  desecrated: bool,
  text: str,
  floor: nullable(uint),
  floor_in_force: uint,
  best_tier: uint,
});
const stepLine = record<StepLineView>({
  id: str,
  number: uint,
  instruction: str,
  targets: list(str),
  uses: list(str),
  odds: nullable(str),
  fallback: nullable(str),
});
const planning = record<PlanningView>({
  item,
  choices: record<TargetChoices>({ prefixes: list(choice), suffixes: list(choice), desecrated: list(choice) }),
  targets: list(chosen),
  blocked: record<CraftBlockedView>({
    blocked: list(record<CraftBlockedEntry>({ id: str, name: str, from: oneOf(BLOCKED_FROM) })),
    allowed: list(catalogueEntry),
  }),
  preview: nullable(
    record<PreviewView>({
      route: nullable(record<RouteView>({ blueprint: str, name: str, source: str, steps: list(stepLine) })),
      other_routes: list(record<RouteChoice>({ blueprint: str, name: str })),
      rejected: list(record<RejectedLine>({ blueprint: str, name: str, reason: str })),
    }),
  ),
  can_accept: bool,
});
const historyEntry = record<HistoryEntry>({
  number: uint,
  step: str,
  instruction: str,
  result: oneOf(OUTCOME_WORDS),
  summary: str,
  source: oneOf(OUTCOME_SOURCES),
});
const watchLine = record<WatchLineView>({ text: str, warning: bool });
const guiding = record<GuidingView>({
  item,
  blueprint: str,
  route_name: str,
  status: oneOf(GUIDE_STATUSES),
  step: nullable(record<CurrentStepView>({ step: stepLine, on_hit: str, on_miss: str, try_number: uint })),
  reasons: list(str),
  history: list(historyEntry),
  watch: nullable(watchLine),
});
const overlay = record<OverlayView>({
  visible: bool,
  position: oneOf(OVERLAY_POSITIONS),
  guide: nullable(
    record<OverlayGuide>({
      status: oneOf(GUIDE_STATUSES),
      route_name: str,
      step_number: nullable(uint),
      try_number: nullable(uint),
      instruction: str,
      uses: list(str),
      odds: nullable(str),
      why: nullable(str),
      on_hit: nullable(str),
      on_miss: nullable(str),
      last: nullable(historyEntry),
      watch: nullable(watchLine),
    }),
  ),
});
const craftFields = record<CraftView>({
  mode: oneOf(CRAFT_MODES),
  refusal: nullable(str),
  planning: nullable(planning),
  guiding: nullable(guiding),
});
/** The fields, then the one rule they carry together: the mode names the one part that is set. */
const craft: Check = (v, p, out) => {
  const before = out.length;
  craftFields(v, p, out);
  if (out.length > before) return;
  const c = v as CraftView;
  const set = { refused: c.refusal !== null, planning: c.planning !== null, guiding: c.guiding !== null };
  for (const [part, present] of Object.entries(set)) {
    if (present !== (c.mode === part)) out.push(`${p}.${part === 'refused' ? 'refusal' : part}: ${present ? 'set' : 'missing'} in mode ${c.mode}`);
  }
};

const view = record<View>({
  hotkey: str,
  grab: nullable(grab),
  status: nullable(statusLine),
  overlay,
  update_available: nullable(str),
  craft,
});

/** Every way `value` differs from a `View`; empty when it is one. */
export function viewProblems(value: unknown): string[] {
  const out: string[] = [];
  view(value, 'view', out);
  return out;
}

const catalogue = list(record<CatalogueGroup>({ kind: str, label: str, entries: list(catalogueEntry) }));

/** Every way `value` differs from `get_catalogue`'s answer; empty when it is one. */
export function catalogueProblems(value: unknown): string[] {
  const out: string[] = [];
  catalogue(value, 'catalogue', out);
  return out;
}

export function isView(value: unknown): value is View {
  return viewProblems(value).length === 0;
}
