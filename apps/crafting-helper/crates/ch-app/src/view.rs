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

/// Where the overlay sits inside the game's client area: an edge's middle
/// or a corner (`overlay_position` in the settings, `top-center` by
/// default).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OverlayPosition {
    TopLeft,
    #[default]
    TopCenter,
    TopRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

impl OverlayPosition {
    /// Every position, in the order Settings offers them.
    pub const ALL: [OverlayPosition; 6] = [
        OverlayPosition::TopLeft,
        OverlayPosition::TopCenter,
        OverlayPosition::TopRight,
        OverlayPosition::BottomLeft,
        OverlayPosition::BottomCenter,
        OverlayPosition::BottomRight,
    ];

    /// Its name as the settings and the view write it (`top-center`).
    pub fn name(self) -> &'static str {
        match self {
            OverlayPosition::TopLeft => "top-left",
            OverlayPosition::TopCenter => "top-center",
            OverlayPosition::TopRight => "top-right",
            OverlayPosition::BottomLeft => "bottom-left",
            OverlayPosition::BottomCenter => "bottom-center",
            OverlayPosition::BottomRight => "bottom-right",
        }
    }

    /// The position `name` names exactly, or `None`.
    pub fn from_name(name: &str) -> Option<OverlayPosition> {
        OverlayPosition::ALL.into_iter().find(|p| p.name() == name)
    }
}

/// The overlay window's state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverlayView {
    /// True only while a craft is guiding and the game's window is in
    /// front.
    pub visible: bool,
    /// Where it sits, so the page anchors its panel to that edge.
    pub position: OverlayPosition,
    /// What it shows: present exactly while a craft is guiding, shown or
    /// not.
    pub guide: Option<OverlayGuide>,
}

/// The overlay's content: the guide's current step, where a hit and a miss
/// go and the last outcome (the copy is Rust's; the labels are the page's).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverlayGuide {
    pub status: GuideStatusView,
    pub route_name: String,
    /// The current step's place in the route, from 1, while active or
    /// stuck.
    pub step_number: Option<usize>,
    /// Which try of that step this is, from 1.
    pub try_number: Option<u32>,
    /// The step's instruction; with replan and done, what to do instead.
    pub instruction: String,
    /// The currency's name, then each omen's, while active.
    pub uses: Vec<String>,
    /// The odds in words, while active (as the guiding view's step).
    pub odds: Option<String>,
    /// Why it is stuck or must be planned again: the first reason.
    pub why: Option<String>,
    /// `On a hit: …`, while active or stuck.
    pub on_hit: Option<String>,
    /// `On a miss: …`, while active or stuck.
    pub on_miss: Option<String>,
    /// The last observed change; `None` before the first.
    pub last: Option<HistoryEntry>,
    /// The screen watch's line, the guiding view's own; `None` with replan
    /// and done.
    pub watch: Option<WatchLineView>,
}

/// The screen watch's line (Rust's copy, `ui-design.md` § "The screen
/// watch's line"): what the watch sees and what to do. The guiding card and
/// the overlay's Screen row show the same one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WatchLineView {
    pub text: String,
    /// It asks the player to do something: the warning colour.
    pub warning: bool,
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
    /// `None` when it can be chosen; otherwise why not, the row's tooltip:
    /// `<Prefixes|Suffixes> full: <a> on the item + <b> chosen = <a+b> of
    /// <limit>`. A chosen choice is never disabled.
    pub disabled: Option<String>,
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
    /// What to do: for a step that places one of the targets, composed from
    /// its data (the currency, its omens, the side and the target
    /// modifiers, `crate::craft::step_instruction`); for a removal step, the
    /// currency and the modifier(s) it removes (a whittle-or-annul step as
    /// the app named it by the modifier levels); otherwise the blueprint's
    /// text.
    pub instruction: String,
    /// The texts of the target modifiers a hit of this step may place, as
    /// the planning list writes them; empty for a step that places none
    /// (a removal or a manual step).
    pub targets: Vec<String>,
    /// The currency's name, then each omen's; empty for a manual step.
    pub uses: Vec<String>,
    /// The odds in words; `None` for a manual step.
    pub odds: Option<String>,
    /// Set when an earlier alternative was passed over; never on a step the
    /// app named by the modifier levels.
    pub fallback: Option<String>,
}

/// The route the preview shows: what it targets and what to do, never the
/// blueprint's `notes` (the reasoning behind it); the page shows the chosen
/// targets (`PlanningView::targets`) above the steps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteView {
    pub blueprint: String,
    pub name: String,
    /// Where the technique came from (attribution).
    pub source: String,
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
    /// The chosen targets, in the order chosen. A grab that starts planning
    /// ticks the item's own explicit modifiers, each at the tier it has.
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

/// Where an observed change came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutcomeSource {
    /// A grab of the item's text (the hotkey or a paste).
    Grab,
    /// The screen watch read the hovered item twice alike.
    Screen,
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
    /// What changed, without the outcome's word (`result` carries it).
    pub summary: String,
    pub source: OutcomeSource,
}

/// Mode guiding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GuidingView {
    /// The item as last grabbed, or as the screen watch last read a hit or
    /// a miss.
    pub item: CraftItemView,
    pub blueprint: String,
    pub route_name: String,
    pub status: GuideStatusView,
    /// The current step, while active or stuck.
    pub step: Option<CurrentStepView>,
    /// Why it is stuck or must be planned again.
    pub reasons: Vec<String>,
    pub history: Vec<HistoryEntry>,
    /// The screen watch's line, while active or stuck; `None` with replan
    /// and done.
    pub watch: Option<WatchLineView>,
}

/// Everything the Settings window shows: `ep-update`'s view of the update
/// check and install, then the hotkey, the blocked currencies, the
/// overlay's position and the screen watch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingsView {
    #[serde(flatten)]
    pub updates: AppSettingsView,
    pub hotkey: String,
    /// What the last hotkey change, or the start, said about the hotkey.
    pub hotkey_status: Option<StatusLine>,
    pub blocked_currencies: Vec<String>,
    /// Where the overlay sits over the game.
    pub overlay_position: OverlayPosition,
    /// Whether the app reads the hovered item from the screen while a craft
    /// is guiding.
    pub screen_watch: bool,
}
