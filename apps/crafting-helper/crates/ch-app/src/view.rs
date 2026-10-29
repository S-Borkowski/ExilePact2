//! The views the windows render (Context "The IPC contract"): [`View`],
//! which the `ch:view` event carries to the `main` and `overlay` windows and
//! the `get_view` command returns, and [`SettingsView`], which the
//! `ch:app-settings` event carries to the `settings` window and its commands
//! return. `src/ipc.ts` mirrors these types field for field, and the
//! fixtures under `fixtures/` hold one serialised view per state the page
//! tests render (`fixtures_are_current`).
//!
//! The main view carries the craft window's part, [`CraftView`], always
//! present, in one of four modes ([`CraftMode`]); `crate::craft` builds it.
//!
//! Every type of the main view denies unknown fields, so a field either
//! side adds, drops or renames fails the fixture test. [`SettingsView`]
//! flattens `ep-update`'s [`AppSettingsView`] in, which serde cannot combine
//! with denying unknown fields; the fixture test compares its JSON instead.

use ep_update::AppSettingsView;
use serde::{Deserialize, Serialize};

/// Where a grab's text came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrabSource {
    /// The app's global hotkey read the clipboard once.
    Shortcut,
    /// A paste into the main window; no clipboard was read.
    Paste,
}

/// The last item text grabbed, kept in memory only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrabView {
    /// The text, normalised: LF line breaks, no trailing blank lines.
    pub text: String,
    pub line_count: usize,
    pub source: GrabSource,
    /// The first line starts like the game's copied item text.
    pub looks_like_item: bool,
}

/// Whether the status line reports or warns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusKind {
    Info,
    Error,
}

/// One status line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatusLine {
    pub kind: StatusKind,
    pub text: String,
}

impl StatusLine {
    pub fn info(text: impl Into<String>) -> StatusLine {
        StatusLine {
            kind: StatusKind::Info,
            text: text.into(),
        }
    }

    pub fn error(text: impl Into<String>) -> StatusLine {
        StatusLine {
            kind: StatusKind::Error,
            text: text.into(),
        }
    }
}

/// The overlay window's state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverlayView {
    /// True only while a craft is active.
    pub visible: bool,
    /// The one line the overlay shows.
    pub line: String,
}

/// Everything the main window and the overlay show.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct View {
    /// The global hotkey registered now (or wanted, when it was refused).
    pub hotkey: String,
    /// The last grab; `None` until one passes.
    pub grab: Option<GrabView>,
    pub status: Option<StatusLine>,
    pub overlay: OverlayView,
    /// The newer version a check found this run (`1.3.0`), which the
    /// header's *Settings* button names; `None` while none is known.
    pub update_available: Option<String>,
    /// The craft window: always present, in one of four modes.
    pub craft: CraftView,
}

// ---------------------------------------------------------------------------
// The craft window (built by `crate::craft`).

/// Which part of the craft window shows. The page picks its layout from
/// this alone and never infers it from the other fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CraftMode {
    /// Nothing grabbed yet.
    None,
    /// The last grab is not an item the engine plans for.
    Refused,
    /// An item, its targets and the route preview.
    Planning,
    /// A route started: the current step and the history.
    Guiding,
}

/// The craft part of the main view. Exactly the part its mode names is
/// present: `refusal` in refused, `planning` in planning, `guiding` in
/// guiding, none in none.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CraftView {
    pub mode: CraftMode,
    /// Why the last grab cannot be planned, in words.
    pub refusal: Option<String>,
    pub planning: Option<PlanningView>,
    pub guiding: Option<GuidingView>,
}

/// A modifier's side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModSide {
    Prefix,
    Suffix,
}

/// The parsed item, as the craft window shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CraftItemView {
    /// A rare item's name; `None` for a normal or magic one.
    pub name: Option<String>,
    pub base_type: Option<String>,
    /// The item class as the game writes it (`Rings`).
    pub class: String,
    /// `Normal`, `Magic` or `Rare`.
    pub rarity: String,
    pub item_level: Option<u32>,
    /// Each explicit (prefix or suffix) modifier, in text order.
    pub modifiers: Vec<ItemModView>,
}

/// One explicit modifier of the item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemModView {
    pub side: ModSide,
    /// Its lines as the item text shows them.
    pub lines: Vec<String>,
    /// The data's tier (1 is the best); `None` when the data has none.
    pub tier: Option<u32>,
    pub fractured: bool,
    pub desecrated: bool,
    pub unrevealed: bool,
    /// It is a chosen target at its floor or better.
    pub meets_target: bool,
}

/// One option of a target's floor select.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FloorOption {
    /// What `set_targets` sends: `None` for "as high as possible".
    pub floor: Option<u32>,
    /// `As high as possible (T<best>)` or `At least T<n>`.
    pub label: String,
}

/// A modifier the item's base can roll at its item level.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetChoice {
    /// The data modifier id `set_targets` sends.
    pub mod_id: String,
    pub side: ModSide,
    /// Its lines with each value as `#`, joined by ` / `.
    pub text: String,
    /// The best tier the item level reaches.
    pub best_tier: u32,
    /// The default first, then one per reachable tier, best first.
    pub floors: Vec<FloorOption>,
}

/// The target choices for the item's base, in three groups.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetChoices {
    pub prefixes: Vec<TargetChoice>,
    pub suffixes: Vec<TargetChoice>,
    /// Modifiers only desecration places, either side.
    pub desecrated: Vec<TargetChoice>,
}

/// A chosen target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChosenTarget {
    pub mod_id: String,
    pub side: ModSide,
    /// In the desecrated group.
    pub desecrated: bool,
    pub text: String,
    /// As chosen: `None` is "as high as possible".
    pub floor: Option<u32>,
    /// The floor in force: the chosen one, or the best tier.
    pub floor_in_force: u32,
    pub best_tier: u32,
}

/// Where a blocked id of this craft comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockedFrom {
    /// The Settings list.
    Settings,
    /// Added for this craft only.
    Craft,
}

/// One id of the effective blocked list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CraftBlockedEntry {
    pub id: String,
    /// The English name.
    pub name: String,
    pub from: BlockedFrom,
}

/// The blocked list this craft plans with: the Settings list, plus this
/// craft's additions, minus this craft's exceptions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CraftBlockedView {
    /// The effective list: the Settings ids first, then this craft's.
    pub blocked: Vec<CraftBlockedEntry>,
    /// The Settings ids allowed for this craft.
    pub allowed: Vec<crate::settings::CatalogueEntry>,
}

/// One step of a route, as the preview and the guide show it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StepLineView {
    /// The blueprint's step id.
    pub id: String,
    /// Its place in the route, from 1.
    pub number: usize,
    /// The blueprint's instruction.
    pub instruction: String,
    /// The currency's name, then each omen's; empty for a manual step.
    pub uses: Vec<String>,
    /// The odds in words; `None` for a manual step.
    pub odds: Option<String>,
    /// Set when an earlier alternative was passed over.
    pub fallback: Option<String>,
}

/// The route the preview shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteView {
    pub blueprint: String,
    pub name: String,
    /// Where the technique came from.
    pub source: String,
    pub notes: Option<String>,
    pub steps: Vec<StepLineView>,
}

/// Another applicable route, for the route select.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteChoice {
    pub blueprint: String,
    pub name: String,
}

/// A blueprint that does not apply, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RejectedLine {
    pub blueprint: String,
    pub name: String,
    /// The rejection in words.
    pub reason: String,
}

/// The route preview: a route and the other applicable ones, or, with no
/// route, every blueprint's rejection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewView {
    pub route: Option<RouteView>,
    /// The other applicable routes, in the engine's order.
    pub other_routes: Vec<RouteChoice>,
    /// Filled only when there is no route.
    pub rejected: Vec<RejectedLine>,
}

/// Mode planning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanningView {
    pub item: CraftItemView,
    pub choices: TargetChoices,
    /// The chosen targets, in the order chosen.
    pub targets: Vec<ChosenTarget>,
    pub blocked: CraftBlockedView,
    /// `None` with no target.
    pub preview: Option<PreviewView>,
    /// `accept_craft` would start: there is a previewed route.
    pub can_accept: bool,
}

/// Where the guide stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GuideStatusView {
    Active,
    Stuck,
    Replan,
    Done,
}

/// The step the player is on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CurrentStepView {
    pub step: StepLineView,
    /// `On a hit: …`
    pub on_hit: String,
    /// `On a miss: …`
    pub on_miss: String,
    /// Which try of this step this is, from 1.
    pub try_number: u32,
}

/// What one observed change was.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutcomeWord {
    Hit,
    Miss,
    Unexpected,
}

/// One observed change, oldest first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HistoryEntry {
    /// The step the change was for: its place in the route and its id.
    pub number: usize,
    pub step: String,
    pub instruction: String,
    pub result: OutcomeWord,
    /// Starts `Hit:`, `Miss:` or `Unexpected:`.
    pub summary: String,
}

/// Mode guiding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GuidingView {
    /// The item as last grabbed.
    pub item: CraftItemView,
    pub blueprint: String,
    pub route_name: String,
    pub status: GuideStatusView,
    /// The current step, while active or stuck.
    pub step: Option<CurrentStepView>,
    /// Why it is stuck or must be planned again.
    pub reasons: Vec<String>,
    pub history: Vec<HistoryEntry>,
}

/// Everything the Settings window shows: `ep-update`'s view of the update
/// check and install, then the hotkey and the blocked currencies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingsView {
    #[serde(flatten)]
    pub updates: AppSettingsView,
    pub hotkey: String,
    /// What the last hotkey change, or the start, said about the hotkey.
    pub hotkey_status: Option<StatusLine>,
    pub blocked_currencies: Vec<String>,
}
