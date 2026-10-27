//! View-model: from engine state, quest data and settings to the `View` and
//! `RowChange` the UI renders, deciding in Rust which frame shows.
//!
//! Rust decides, the UI renders (Context "IPC contract"): the shell calls
//! [`build_view`] after every change and sends the result as `qt:view`; the
//! UI never re-derives a status, a filter or a frame. Field names are the
//! contract's, exactly as `serde` writes them, and `src/` mirrors them in
//! TypeScript. `fixtures/viewmodels/<frame id>.json` holds one serialised
//! view per frame id, which keeps the two languages' types in step.
//!
//! Which frame shows, first match wins:
//!
//! 1. the frame the user opened (`character-picker` or `settings`);
//! 2. `collapsed`, when the panel is folded to its badge;
//! 3. `empty-no-log` when no `Client.txt` was found;
//! 4. with no current character, `empty-waiting` while the log has nothing
//!    new, else `empty-no-character`. A new character from The Riverbank
//!    that no level-up has named yet is a current character here: it shows
//!    as `provisional`, with an empty name and level 1, and its rows;
//! 5. `quest-detail` when a quest the data knows is open;
//! 6. in the next-up view, `empty-act-done` when every quest of the act is
//!    done or skipped, else `panel-next-up`; in the campaign view
//!    `panel-campaign`.
//!
//! Only the act the character is in is listed (owner, 2026-10-25): the
//! highest act holding a quest that is in progress, done, unknown, or
//! available through an unlock signal that fired. A quest with no unlock is
//! available from the start and says nothing about where the character is.
//!
//! Steps: a step that fired or was ticked is done, and so is every step
//! before it; the step after the furthest one reached is `current`, the
//! one to do next. That is the step an in-progress row shows as its second
//! line (the brief: "its next step"); a quest whose every step is reached
//! but which is not done shows no step, and no tip, until it is done (Live
//! 9: Renly's banner ticked Reaching Clearfell's last step, and the row
//! kept showing that step, already done). A step's find-it `tip` goes
//! with it: on its `StepView`, and as the row's `current_tip` when it is the
//! step the row shows (Context "Find-it tips").
//!
//! Skipped quests stay tracked (Context "Skipped quests stay tracked"): a
//! skipped row keeps `status: skipped`, but its step, its tip and its
//! detail's step phases follow the engine's `tracked_status` and evidence,
//! exactly as the row would show them unskipped.
//!
//! Inferred: a done row whose quest data says its done is a later event
//! (`done_inferred`), and a done step whose own signal never fired and that
//! the user did not tick, carry `inferred`, the UI's "probably done" mark
//! (Context "Launch 7 fixes (owner scope)"). Neither counts when the user
//! marked the quest done, and the mark changes nothing the user can do.
//!
//! Filters: "only worth-it" hides quests that are not worth it; "show
//! skipped" shows a skipped quest whatever "only worth-it" says, in the
//! campaign view and, when its tracked status is available or in progress,
//! in next up, after the unskipped rows of its worth group. Neither changes
//! an act's counts, the badge or whether the act is done; with "show
//! skipped" off, next up lists no skipped quest.
//!
//! Live changes: a quest becoming available, a step reached, a quest started
//! by its `start` signal with no step reached, or a quest done, read from a
//! live line, is a [`RowChange`] the shell sends as
//! `qt:row-change`; the UI animates that row and announces its `text`
//! (Context "Post-live fixes (owner scope)"). There is no queue, cap or
//! expiry, and catch-up makes none.

use std::collections::BTreeSet;

use qt_engine::{
    Detection, Element, Engine, Quest, QuestData, QuestState, Reward, RewardCategory, Status,
    Transition, Worth,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The `schema` number of every `View`.
pub const SCHEMA: u32 = 1;

// ---------------------------------------------------------------------------
// The contract's types.

/// A frame id of the brief's `## Export contract`. `mode-play` and
/// `mode-edit` are the same panel in each [`Mode`], so neither is a frame
/// here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Frame {
    PanelCampaign,
    PanelNextUp,
    Collapsed,
    QuestDetail,
    CharacterPicker,
    Settings,
    EmptyNoLog,
    EmptyWaiting,
    EmptyNoCharacter,
    EmptyActDone,
}

impl Frame {
    pub const ALL: [Frame; 10] = [
        Frame::PanelCampaign,
        Frame::PanelNextUp,
        Frame::Collapsed,
        Frame::QuestDetail,
        Frame::CharacterPicker,
        Frame::Settings,
        Frame::EmptyNoLog,
        Frame::EmptyWaiting,
        Frame::EmptyNoCharacter,
        Frame::EmptyActDone,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Frame::PanelCampaign => "panel-campaign",
            Frame::PanelNextUp => "panel-next-up",
            Frame::Collapsed => "collapsed",
            Frame::QuestDetail => "quest-detail",
            Frame::CharacterPicker => "character-picker",
            Frame::Settings => "settings",
            Frame::EmptyNoLog => "empty-no-log",
            Frame::EmptyWaiting => "empty-waiting",
            Frame::EmptyNoCharacter => "empty-no-character",
            Frame::EmptyActDone => "empty-act-done",
        }
    }
}

/// Play is click-through with no controls; edit takes clicks.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    #[default]
    Play,
    Edit,
}

/// The panel's two lists. Next up is the default while playing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewKind {
    Campaign,
    #[default]
    NextUp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusKind {
    Ok,
    Info,
    Warn,
    Error,
}

/// The panel's one status line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusLine {
    pub kind: StatusKind,
    pub text: String,
}

impl StatusLine {
    pub fn new(kind: StatusKind, text: impl Into<String>) -> StatusLine {
        StatusLine {
            kind,
            text: text.into(),
        }
    }

    pub fn ok(text: impl Into<String>) -> StatusLine {
        StatusLine::new(StatusKind::Ok, text)
    }

    pub fn info(text: impl Into<String>) -> StatusLine {
        StatusLine::new(StatusKind::Info, text)
    }

    pub fn warn(text: impl Into<String>) -> StatusLine {
        StatusLine::new(StatusKind::Warn, text)
    }

    pub fn error(text: impl Into<String>) -> StatusLine {
        StatusLine::new(StatusKind::Error, text)
    }
}

/// A character as the header and the picker show it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharacterView {
    pub name: String,
    pub class: String,
    pub level: u32,
    pub own: bool,
    pub pinned: bool,
    pub active: bool,
    /// The new character from The Riverbank that no level-up has named yet:
    /// an empty name, level 1, never in `characters` (the UI labels it "New
    /// character").
    pub provisional: bool,
}

/// One act's header and its rows (empty while the act is folded).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActView {
    pub act: u32,
    pub title: String,
    /// Quests done in the act.
    pub done: u32,
    /// Quests in the act that are not skipped. Neither count follows the
    /// display filters.
    pub total: u32,
    pub expanded: bool,
    /// A row the filters show is available.
    pub has_available: bool,
    pub quests: Vec<QuestRow>,
}

/// A reward as a row or the detail shows it; `icon` is an icon id of the brief.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RewardView {
    pub category: RewardCategory,
    pub icon: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestRow {
    pub id: String,
    pub name: String,
    pub act: u32,
    pub status: Status,
    pub worth: Worth,
    pub detection: Detection,
    /// The manual-tick marker: detection `partial` or `manual`.
    pub needs_tick: bool,
    /// The "probably done" mark: the quest shows done because of a later
    /// event its data names (`done_inferred`), not its own completion, and
    /// the user did not mark it done. A mark only: it changes neither
    /// `needs_tick` nor what the user can do with the row.
    pub inferred: bool,
    pub rewards: Vec<RewardView>,
    /// The title of the step to do next, for an in-progress quest only (a
    /// skipped one counts by its tracked status).
    pub current_step: Option<String>,
    /// The find-it tip of the step `current_step` names, if it has one.
    pub current_tip: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepPhase {
    Pending,
    Current,
    Done,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepView {
    pub id: String,
    pub title: String,
    pub text: String,
    pub state: StepPhase,
    pub ticked: bool,
    /// The "probably done" mark: the step shows done, but its own signal
    /// never fired and the user did not tick it; a later step, or the
    /// quest's done, implied it. Not set when the user marked the quest done
    /// (that is the user's word, not an inference). A mark only: the step
    /// ticks exactly as it would without it.
    pub inferred: bool,
    /// The owner's short "how to find it" text for this step, shown under
    /// its text; `None` when the data has none.
    pub tip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestDetail {
    pub row: QuestRow,
    pub giver: Option<String>,
    /// Display names: an area code the data knows becomes its name.
    pub zones: Vec<String>,
    pub worth_reason: String,
    pub steps: Vec<StepView>,
    pub source_url: String,
    pub fetched: String,
}

/// The panel's zoom: Auto (from the monitor) or a fixed factor from
/// [`Scale::MIN`] to [`Scale::MAX`], which the settings slider moves in
/// [`Scale::STEP`]s (Context "Live 2 fixes (owner scope)").
///
/// Serialised as the string `"auto"` or the factor as a JSON number
/// (`1.25`). It loads from either, and from a number in a string, so the
/// stored `"1"`, `"1.33"` and `"2"` of the old three-way choice load as the
/// same factor. A factor is kept to two decimals (1.33 stays 1.33); a value
/// out of range or of any other shape loads as Auto, never as an error.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum Scale {
    #[default]
    Auto,
    Fixed(f64),
}

impl Scale {
    pub const MIN: f64 = 0.75;
    pub const MAX: f64 = 2.0;
    /// The slider's step; a stored factor need not sit on it.
    pub const STEP: f64 = 0.05;

    /// A fixed factor, rounded to two decimals; Auto when that is not
    /// finite or outside `MIN..=MAX`.
    pub fn fixed(factor: f64) -> Scale {
        let f = (factor * 100.0).round() / 100.0;
        if f.is_finite() && (Scale::MIN..=Scale::MAX).contains(&f) {
            Scale::Fixed(f)
        } else {
            Scale::Auto
        }
    }

    /// The fixed factor, or `None` for Auto.
    pub fn factor(self) -> Option<f64> {
        match self.normalised() {
            Scale::Auto => None,
            Scale::Fixed(f) => Some(f),
        }
    }

    /// This scale with a fixed factor brought back in range (else Auto).
    pub fn normalised(self) -> Scale {
        match self {
            Scale::Auto => Scale::Auto,
            Scale::Fixed(f) => Scale::fixed(f),
        }
    }

    /// The scale a stored or patched JSON value means (type docs).
    pub fn from_value(value: &Value) -> Scale {
        let factor = match value {
            Value::String(s) if s.trim() == "auto" => return Scale::Auto,
            Value::String(s) => s.trim().parse::<f64>().ok(),
            Value::Number(n) => n.as_f64(),
            _ => None,
        };
        factor.map_or(Scale::Auto, Scale::fixed)
    }
}

impl Serialize for Scale {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self.normalised() {
            Scale::Auto => s.serialize_str("auto"),
            Scale::Fixed(f) => s.serialize_f64(f),
        }
    }
}

impl<'de> Deserialize<'de> for Scale {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Scale, D::Error> {
        Ok(Scale::from_value(&Value::deserialize(d)?))
    }
}

/// The user's settings, as `settings.json` stores them and the settings
/// frame shows them. A missing field takes its default; a key this struct
/// does not have, such as the `dock` side files written before Live 2
/// carry, is ignored on load (and refused in a patch).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// The panel background's opacity, 0-1. At 0 the background is gone;
    /// text, icons, the border and the edit-mode accent stay opaque.
    pub opacity: f64,
    pub scale: Scale,
    pub show_skipped: bool,
    pub only_worth_it: bool,
    /// `None`: discover the log (Context "Overlay window and threads").
    pub log_path: Option<String>,
    /// The global shortcut that toggles play and edit mode.
    pub hotkey: String,
    /// "Read the screen": the opt-in screen reader, which copies the game's
    /// window for the boss bar, the voiced lines and the dialogue window
    /// while a step it can finish is open, and stores nothing it reads.
    /// Off by default; a file written before it existed loads it off.
    pub screen_reading: bool,
    /// "Hide under game panels": in play mode, the overlay hides while one
    /// of the game's panels (the inventory, the passive tree, the stash...)
    /// is open under its window, told from the colour of a few small copies
    /// of the panel's frame, and shows again once it closes. Off by
    /// default; a file written before it existed loads it off.
    pub hide_under_panels: bool,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            opacity: Settings::DEFAULT_OPACITY,
            scale: Scale::Auto,
            show_skipped: false,
            only_worth_it: false,
            log_path: None,
            hotkey: Settings::DEFAULT_HOTKEY.to_string(),
            screen_reading: false,
            hide_under_panels: false,
        }
    }
}

impl Settings {
    /// The opacity slider's low end: no background at all (owner, after
    /// Live 2; it was a 30% floor before).
    pub const MIN_OPACITY: f64 = 0.0;
    pub const DEFAULT_OPACITY: f64 = 0.9;
    pub const DEFAULT_HOTKEY: &'static str = "Ctrl+Alt+Q";

    /// The settings with every value in range: opacity clamped to 0-1 (a
    /// non-number becomes the default), a fixed scale out of range Auto, an
    /// empty hotkey or log path unset.
    pub fn normalised(mut self) -> Settings {
        self.opacity = if self.opacity.is_finite() {
            self.opacity.clamp(Settings::MIN_OPACITY, 1.0)
        } else {
            Settings::DEFAULT_OPACITY
        };
        self.scale = self.scale.normalised();
        if self.hotkey.trim().is_empty() {
            self.hotkey = Settings::DEFAULT_HOTKEY.to_string();
        }
        if self.log_path.as_deref().is_some_and(|p| p.trim().is_empty()) {
            self.log_path = None;
        }
        self
    }

    /// These settings with `patch` (the `update_settings` command's object)
    /// applied, normalised. An unknown key or a value of the wrong shape is
    /// refused, naming it, and changes nothing.
    pub fn patched(&self, patch: &Value) -> Result<Settings, String> {
        let fields = patch
            .as_object()
            .ok_or_else(|| "a settings patch is a JSON object".to_string())?;
        let mut whole = serde_json::to_value(self).map_err(|e| e.to_string())?;
        let obj = whole.as_object_mut().expect("Settings serialises as an object");
        for (key, value) in fields {
            if !obj.contains_key(key) {
                return Err(format!("unknown setting {key:?}"));
            }
            obj.insert(key.clone(), value.clone());
        }
        let out: Settings =
            serde_json::from_value(whole).map_err(|e| format!("bad settings patch: {e}"))?;
        Ok(out.normalised())
    }
}

/// What the whole panel shows. The shell sends it as `qt:view`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct View {
    pub schema: u32,
    pub frame: Frame,
    pub mode: Mode,
    pub view_kind: ViewKind,
    pub status: Option<StatusLine>,
    /// The character progress goes to (pinned, else active).
    pub character: Option<CharacterView>,
    pub characters: Vec<CharacterView>,
    /// The current act only; empty with no character.
    pub acts: Vec<ActView>,
    /// The current act's available and in-progress quests, worth-it first;
    /// with "show skipped", the skipped ones tracked as either too.
    pub next_up: Vec<QuestRow>,
    pub detail: Option<QuestDetail>,
    pub settings: Settings,
    /// Worth-it quests of the current act available now (the collapsed badge).
    pub available_worth_count: u32,
}

/// What a live transition changed in a quest's row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RowChangeKind {
    Available,
    StepDone,
    QuestDone,
}

impl RowChangeKind {
    /// The contract's word, as `serde` writes it (and the shell's
    /// diagnostic line `row change <id> <kind> (<quest id>)` uses).
    pub fn as_str(self) -> &'static str {
        match self {
            RowChangeKind::Available => "available",
            RowChangeKind::StepDone => "step_done",
            RowChangeKind::QuestDone => "quest_done",
        }
    }
}

/// One live change of one quest's row. The shell sends it as
/// `qt:row-change`; the UI animates the row whose `id` is `quest_id` (the
/// completed step out, the next one in) and announces `text` in a polite
/// live region.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RowChange {
    /// Increasing, from 1, for the shell's lifetime.
    pub id: u64,
    pub kind: RowChangeKind,
    pub quest_id: String,
    /// The title of the step just completed, or `None`.
    pub step_done: Option<String>,
    /// The title of the step now current, or `None`.
    pub step_next: Option<String>,
    /// One sentence for the screen reader, naming the quest and what changed.
    pub text: String,
}

// ---------------------------------------------------------------------------
// What the shell keeps between views.

/// Whether the log is there and has given anything.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogState {
    /// No `Client.txt` at the configured or discovered path.
    Missing,
    /// The log exists and nothing new has been written to it yet.
    #[default]
    Waiting,
    /// Lines are being read.
    Reading,
}

/// The shell's UI state: everything a view needs beyond the engine and the
/// settings. The commands of the IPC contract change it; it is not data.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiState {
    pub mode: Mode,
    pub view_kind: ViewKind,
    pub collapsed: bool,
    /// `character-picker` or `settings` when open.
    pub open_frame: Option<Frame>,
    /// The quest whose detail is open.
    pub detail: Option<String>,
    /// Acts the user folded.
    pub folded_acts: BTreeSet<u32>,
    pub log: LogState,
    /// The shell's own status line (a shortcut that did not register, a
    /// load error). It wins over any status the view derives.
    pub status: Option<StatusLine>,
}

impl UiState {
    /// The `open_frame` command: only the picker and settings open as frames.
    pub fn open_frame(&mut self, frame: Option<Frame>) -> Result<(), String> {
        match frame {
            None | Some(Frame::CharacterPicker) | Some(Frame::Settings) => {
                self.open_frame = frame;
                Ok(())
            }
            Some(other) => Err(format!("{} does not open as a frame", other.as_str())),
        }
    }

    /// The `toggle_act` command.
    pub fn toggle_act(&mut self, act: u32) {
        if !self.folded_acts.remove(&act) {
            self.folded_acts.insert(act);
        }
    }
}

// ---------------------------------------------------------------------------
// Building a view.

/// The icon id of the brief a reward shows.
pub fn reward_icon(reward: &Reward) -> &'static str {
    use RewardCategory::*;
    match reward.category {
        PassivePoints => "reward-passive",
        WeaponSetPoints => "reward-weapon-set",
        Spirit => "reward-spirit",
        Resistance => match reward.element {
            Some(Element::Fire) => "reward-res-fire",
            Some(Element::Cold) => "reward-res-cold",
            Some(Element::Lightning) => "reward-res-lightning",
            Some(Element::Chaos) => "reward-res-chaos",
            Some(Element::All) | None => "reward-res-all",
        },
        Life => "reward-life",
        Mana => "reward-mana",
        Attribute => "reward-attribute",
        Charm => "reward-charm",
        Flask => "reward-flask",
        Defences => "reward-defences",
        OtherStat => "reward-other",
        SkillGem => "reward-skill-gem",
        SupportGem => "reward-support-gem",
        Ascendancy => "reward-ascendancy",
        Item => "reward-item",
        Unlock => "reward-unlock",
        Story => "reward-story",
    }
}

/// The act the character is in, from its quests' states (module docs).
/// `None` when the data has no quest.
pub fn current_act(data: &QuestData, states: &[QuestState]) -> Option<u32> {
    let located = data
        .quests
        .iter()
        .zip(states)
        .filter(|(q, s)| match s.status {
            Status::InProgress | Status::Done | Status::Unknown => true,
            Status::Available => !q.unlock.is_empty(),
            Status::Locked | Status::Skipped => false,
        })
        .map(|(q, _)| q.act)
        .max();
    located.or_else(|| data.quests.iter().map(|q| q.act).min())
}

/// The status a quest's progress shows by: its own, or for a skipped quest
/// the one its tracked evidence gives (what Unskip would show).
fn progress_status(state: &QuestState) -> Status {
    if state.status == Status::Skipped {
        state.tracked_status
    } else {
        state.status
    }
}

/// Each step's phase, and the index of the step to do next.
fn step_phases(quest: &Quest, state: &QuestState) -> (Vec<StepPhase>, Option<usize>) {
    let n = quest.steps.len();
    let status = progress_status(state);
    if status == Status::Done {
        return (vec![StepPhase::Done; n], None);
    }
    let reached = state
        .current_step
        .as_deref()
        .and_then(|id| quest.steps.iter().position(|s| s.id == id));
    let next = reached.map_or(0, |r| r + 1);
    let next = (next < n).then_some(next);
    let open = matches!(
        status,
        Status::Available | Status::InProgress | Status::Unknown
    );
    let phases = (0..n)
        .map(|i| {
            if reached.is_some_and(|r| i <= r) {
                StepPhase::Done
            } else if open && Some(i) == next {
                StepPhase::Current
            } else {
                StepPhase::Pending
            }
        })
        .collect();
    (phases, next)
}

fn row(quest: &Quest, state: &QuestState) -> QuestRow {
    // The step the row shows: an in-progress quest's next one (a skipped
    // quest's by its tracked progress), else none. Once its last step is
    // reached there is no next one, and the row shows none until it is done.
    let shown = if progress_status(state) == Status::InProgress {
        let (_, next) = step_phases(quest, state);
        next.map(|i| &quest.steps[i])
    } else {
        None
    };
    QuestRow {
        id: quest.id.clone(),
        name: quest.name.clone(),
        act: quest.act,
        status: state.status,
        worth: quest.worth,
        detection: quest.detection,
        needs_tick: quest.detection.needs_tick(),
        inferred: state.status == Status::Done
            && state.done_override != Some(true)
            && quest.done_inferred,
        rewards: quest
            .rewards
            .iter()
            .map(|r| RewardView {
                category: r.category,
                icon: reward_icon(r).to_string(),
                text: r.text.clone(),
            })
            .collect(),
        current_step: shown.map(|s| s.title.clone()),
        current_tip: shown.and_then(|s| s.tip.clone()),
    }
}

fn detail(data: &QuestData, quest: &Quest, state: &QuestState) -> QuestDetail {
    let (phases, _) = step_phases(quest, state);
    let user_done = state.done_override == Some(true);
    QuestDetail {
        row: row(quest, state),
        giver: quest.giver.clone(),
        zones: quest
            .zones
            .iter()
            .map(|z| data.areas.get(z).map_or_else(|| z.clone(), |a| a.name.clone()))
            .collect(),
        worth_reason: quest.worth_reason.clone(),
        steps: quest
            .steps
            .iter()
            .zip(phases)
            .map(|(s, phase)| {
                let own = state.steps.iter().find(|t| t.id == s.id);
                let ticked = own.is_some_and(|t| t.ticked);
                let fired = own.is_some_and(|t| t.fired);
                StepView {
                    id: s.id.clone(),
                    title: s.title.clone(),
                    text: s.text.clone(),
                    state: phase,
                    ticked,
                    inferred: phase == StepPhase::Done && !fired && !ticked && !user_done,
                    tip: s.tip.clone(),
                }
            })
            .collect(),
        source_url: quest.source_url.clone(),
        fetched: quest.fetched.clone(),
    }
}

fn worth_rank(w: Worth) -> u8 {
    match w {
        Worth::Yes => 0,
        Worth::Optional => 1,
        Worth::No => 2,
    }
}

/// What the panel lists for one character.
struct Panel {
    act: u32,
    acts: Vec<ActView>,
    next_up: Vec<QuestRow>,
    act_done: bool,
    available_worth_count: u32,
}

fn panel(data: &QuestData, states: &[QuestState], settings: &Settings, ui: &UiState) -> Option<Panel> {
    let act = current_act(data, states)?;
    let in_act: Vec<(&Quest, &QuestState)> = data
        .quests
        .iter()
        .zip(states)
        .filter(|(q, _)| q.act == act)
        .collect();
    // "Show skipped" shows a skipped quest whatever "only worth-it" says
    // (Live 2: the only skipped quest was optional, so the toggle showed
    // nothing); "only worth-it" still hides the rest that is not worth it.
    let shown = |q: &Quest, s: &QuestState| {
        if s.status == Status::Skipped {
            settings.show_skipped
        } else {
            !settings.only_worth_it || q.worth == Worth::Yes
        }
    };
    let rows: Vec<QuestRow> = in_act
        .iter()
        .filter(|(q, s)| shown(q, s))
        .map(|(q, s)| row(q, s))
        .collect();
    let count = |f: &dyn Fn(&Quest, &QuestState) -> bool| {
        in_act.iter().filter(|(q, s)| f(q, s)).count() as u32
    };
    let expanded = !ui.folded_acts.contains(&act);
    let header = ActView {
        act,
        title: format!("Act {act}"),
        done: count(&|_, s| s.status == Status::Done),
        total: count(&|_, s| s.status != Status::Skipped),
        expanded,
        has_available: rows.iter().any(|r| r.status == Status::Available),
        quests: if expanded { rows } else { Vec::new() },
    };
    // A skipped quest is listed by its tracked status, and only while "show
    // skipped" shows it (Context "Skipped quests stay tracked").
    let mut next: Vec<(&Quest, &QuestState)> = in_act
        .iter()
        .copied()
        .filter(|(q, s)| {
            matches!(progress_status(s), Status::Available | Status::InProgress) && shown(q, s)
        })
        .collect();
    // Stable: data order within the same worth and status; a skipped quest
    // after the unskipped rows of its worth group.
    next.sort_by_key(|(q, s)| {
        (
            worth_rank(q.worth),
            s.status == Status::Skipped,
            progress_status(s) != Status::InProgress,
        )
    });
    Some(Panel {
        act,
        acts: vec![header],
        next_up: next.into_iter().map(|(q, s)| row(q, s)).collect(),
        act_done: !in_act.is_empty()
            && in_act
                .iter()
                .all(|(_, s)| matches!(s.status, Status::Done | Status::Skipped)),
        available_worth_count: count(&|q, s| s.status == Status::Available && q.worth == Worth::Yes),
    })
}

/// Builds the view the UI renders now: which frame, and everything in it.
pub fn build_view(engine: &Engine, settings: &Settings, ui: &UiState) -> View {
    let data = engine.data();
    let settings = settings.clone().normalised();
    let characters: Vec<CharacterView> = engine
        .characters()
        .into_iter()
        .map(|c| CharacterView {
            name: c.name,
            class: c.class,
            level: c.level,
            own: c.own,
            pinned: c.pinned,
            active: c.active,
            provisional: false,
        })
        .collect();
    let named = engine
        .current()
        .and_then(|n| characters.iter().find(|c| c.name == n).cloned());
    // With no named character, the new character from The Riverbank, while
    // no level-up has named it, is the one progress goes to.
    let (character, states) = match (named, engine.new_character_states()) {
        (Some(c), _) => {
            let states = engine.quest_states(&c.name);
            (Some(c), states)
        }
        (None, Some(states)) => (
            Some(CharacterView {
                name: String::new(),
                class: engine
                    .new_character()
                    .map(|c| c.class.clone())
                    .unwrap_or_default(),
                level: 1,
                own: true,
                pinned: false,
                active: true,
                provisional: true,
            }),
            states,
        ),
        (None, None) => (None, Vec::new()),
    };
    let panel = character
        .as_ref()
        .and_then(|_| panel(data, &states, &settings, ui));

    let mut derived: Vec<StatusLine> = Vec::new();
    let open_detail = match (&ui.detail, &character) {
        (Some(id), Some(_)) => match data.quests.iter().position(|q| &q.id == id) {
            Some(i) => Some(detail(data, &data.quests[i], &states[i])),
            None => {
                derived.push(StatusLine::warn(format!("No quest {id:?} in the quest data.")));
                None
            }
        },
        _ => None,
    };
    if ui.log == LogState::Missing {
        derived.push(StatusLine::error(
            "No Client.txt found. Set the log path in Settings.",
        ));
    }
    if let Some(c) = character.as_ref().filter(|c| !c.own) {
        derived.push(StatusLine::warn(format!(
            "{} is not confirmed as your character: confirm it in the character picker.",
            c.name
        )));
    }
    // Evidence the engine could not use, each in its own words: events of a
    // session that ended with no character to go to, events of an unnamed
    // new character that another new character replaced, and area-tied
    // signals refused while the area was unknown.
    let events = |n: u64| format!("{n} log event{}", if n == 1 { "" } else { "s" });
    let signals = |n: u64| format!("{n} quest signal{}", if n == 1 { "" } else { "s" });
    let were = |n: u64| if n == 1 { "was" } else { "were" };
    let (dropped, dropped_new, refused) = (
        engine.dropped_count(),
        engine.dropped_new_character_count(),
        engine.refused_count(),
    );
    let reasons: Vec<(u64, String)> = [
        (dropped, format!("{} came before any character", events(dropped))),
        (
            dropped_new,
            format!(
                "{} belonged to a new character that another new character replaced",
                events(dropped_new)
            ),
        ),
        (refused, format!("{} came before the area was known", signals(refused))),
    ]
    .into_iter()
    .filter(|(n, _)| *n > 0)
    .collect();
    let uncounted = match reasons.as_slice() {
        [] => None,
        [(n, one)] => Some(format!("{one} and {} not counted.", were(*n))),
        many => Some(format!(
            "{}; none was counted.",
            many.iter().map(|(_, s)| s.as_str()).collect::<Vec<_>>().join("; ")
        )),
    };
    if let Some(text) = uncounted {
        derived.push(StatusLine::info(text));
    }

    let frame = match ui.open_frame {
        Some(f @ (Frame::CharacterPicker | Frame::Settings)) => f,
        _ if ui.collapsed => Frame::Collapsed,
        _ if ui.log == LogState::Missing => Frame::EmptyNoLog,
        _ if character.is_none() => match ui.log {
            LogState::Waiting => Frame::EmptyWaiting,
            _ => Frame::EmptyNoCharacter,
        },
        _ if open_detail.is_some() => Frame::QuestDetail,
        _ => match ui.view_kind {
            ViewKind::Campaign => Frame::PanelCampaign,
            ViewKind::NextUp if panel.as_ref().is_some_and(|p| p.act_done) => Frame::EmptyActDone,
            ViewKind::NextUp => Frame::PanelNextUp,
        },
    };
    if frame == Frame::PanelNextUp {
        if let Some(p) = panel.as_ref().filter(|p| p.next_up.is_empty()) {
            derived.push(StatusLine::info(format!(
                "Nothing available in Act {} right now.",
                p.act
            )));
        }
    }

    let (acts, next_up, available_worth_count) = match panel {
        Some(p) => (p.acts, p.next_up, p.available_worth_count),
        None => (Vec::new(), Vec::new(), 0),
    };
    View {
        schema: SCHEMA,
        frame,
        mode: ui.mode,
        view_kind: ui.view_kind,
        status: ui.status.clone().or_else(|| derived.into_iter().next()),
        character,
        characters,
        acts,
        next_up,
        detail: open_detail,
        settings,
        available_worth_count,
    }
}

// ---------------------------------------------------------------------------
// Row changes.

/// The index of a step id in a quest's steps.
fn step_index(quest: &Quest, id: Option<&String>) -> Option<usize> {
    id.and_then(|id| quest.steps.iter().position(|s| &s.id == id))
}

/// The step title an in-progress row shows with `reached` as its furthest
/// step reached: the one after it, or none when every step is reached (the
/// same rule as the row's `current_step`).
fn shown_step(quest: &Quest, reached: Option<&String>) -> Option<String> {
    let next = step_index(quest, reached).map_or(0, |r| r + 1);
    quest.steps.get(next).map(|s| s.title.clone())
}

/// What a transition changes in its row, if anything the UI shows as a
/// change: only a live one, and only a quest becoming available, a step
/// reached, a quest started by its `start` signal (a `step_done` change with
/// no step completed), or a quest done. Returns the kind,
/// the step completed, the step now current and the announcement.
fn row_change_for(
    data: &QuestData,
    t: &Transition,
) -> Option<(RowChangeKind, Option<String>, Option<String>, String)> {
    if !t.live {
        return None;
    }
    let quest = data.quest(&t.quest_id)?;
    if t.to == Status::Done && t.from != Status::Done {
        // The step the row showed, if it showed one, is the one completed.
        let done = (t.from == Status::InProgress)
            .then(|| shown_step(quest, t.step_from.as_ref()))
            .flatten();
        return Some((
            RowChangeKind::QuestDone,
            done,
            None,
            format!("Quest done: {}.", quest.name),
        ));
    }
    if t.to == Status::Available && t.from == Status::Locked {
        return Some((
            RowChangeKind::Available,
            None,
            None,
            format!("Now available: {}.", quest.name),
        ));
    }
    if t.to == Status::InProgress
        && matches!(t.from, Status::Locked | Status::Available)
        && t.step_to.is_none()
    {
        // Started by its `start` signal, no step reached: nothing completed,
        // and the first step is the one the row now shows.
        let next = quest.steps.first().map(|s| s.title.clone());
        let text = match &next {
            Some(next) => format!("Quest started: {}; next: {next}.", quest.name),
            None => format!("Quest started: {}.", quest.name),
        };
        return Some((RowChangeKind::StepDone, None, next, text));
    }
    if t.to == Status::InProgress && t.step_to.is_some() && t.step_to != t.step_from {
        let reached = step_index(quest, t.step_to.as_ref())?;
        let done = quest.steps[reached].title.clone();
        let next = quest.steps.get(reached + 1).map(|s| s.title.clone());
        let text = match &next {
            Some(next) => format!("{}: {done} done; next: {next}.", quest.name),
            None => format!("{}: {done} done.", quest.name),
        };
        return Some((RowChangeKind::StepDone, Some(done), next, text));
    }
    None
}

/// Numbers the row changes: one [`RowChange`] per live transition that
/// changes a row, each sent once. No queue, no cap and no expiry: the UI
/// restarts a row's animation when a second change for it arrives.
#[derive(Debug, Clone)]
pub struct RowChanges {
    next_id: u64,
}

impl Default for RowChanges {
    fn default() -> RowChanges {
        RowChanges::new()
    }
}

impl RowChanges {
    pub fn new() -> RowChanges {
        RowChanges { next_id: 1 }
    }

    /// The row changes a batch of transitions makes, in order, for the shell
    /// to send. Catch-up transitions make none.
    pub fn make(&mut self, data: &QuestData, transitions: &[Transition]) -> Vec<RowChange> {
        transitions
            .iter()
            .filter_map(|t| {
                let (kind, step_done, step_next, text) = row_change_for(data, t)?;
                let id = self.next_id;
                self.next_id += 1;
                Some(RowChange {
                    id,
                    kind,
                    quest_id: t.quest_id.clone(),
                    step_done,
                    step_next,
                    text,
                })
            })
            .collect()
    }
}
