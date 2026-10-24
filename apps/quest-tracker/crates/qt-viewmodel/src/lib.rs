//! View-model: from engine state, quest data and settings to the `View` and
//! `Toast` the UI renders, deciding in Rust which frame shows.
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
//!    new, else `empty-no-character`;
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
//! but which is not done shows its last step.

use std::collections::{BTreeSet, VecDeque};

use qt_engine::{
    Detection, Element, Engine, Quest, QuestData, QuestState, Reward, RewardCategory, Status,
    Transition, Worth,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The `schema` number of every `View`.
pub const SCHEMA: u32 = 1;

/// How long a toast shows, in milliseconds.
pub const TOAST_MS: u64 = 3_000;

/// The most toasts stacked at once.
pub const MAX_TOASTS: usize = 3;

// ---------------------------------------------------------------------------
// The contract's types.

/// A frame id of the brief's `## Export contract`. `mode-play` and
/// `mode-edit` are the same panel in each [`Mode`], and `toast` is its own
/// event, so neither is a frame here.
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
    pub rewards: Vec<RewardView>,
    /// The title of the step to do next, for an in-progress quest only.
    pub current_step: Option<String>,
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Dock {
    Left,
    #[default]
    Right,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Scale {
    #[default]
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "1")]
    X1,
    #[serde(rename = "1.33")]
    X133,
    #[serde(rename = "2")]
    X2,
}

/// The user's settings, as `settings.json` stores them and the settings
/// frame shows them. A missing field takes its default.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// The panel background's opacity, 0.3-1 (text and icons stay opaque).
    pub opacity: f64,
    pub dock: Dock,
    pub scale: Scale,
    pub show_skipped: bool,
    pub only_worth_it: bool,
    /// `None`: discover the log (Context "Overlay window and threads").
    pub log_path: Option<String>,
    /// The global shortcut that toggles play and edit mode.
    pub hotkey: String,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            opacity: Settings::DEFAULT_OPACITY,
            dock: Dock::Right,
            scale: Scale::Auto,
            show_skipped: false,
            only_worth_it: false,
            log_path: None,
            hotkey: Settings::DEFAULT_HOTKEY.to_string(),
        }
    }
}

impl Settings {
    /// The brief's floor for the opacity slider.
    pub const MIN_OPACITY: f64 = 0.3;
    pub const DEFAULT_OPACITY: f64 = 0.9;
    pub const DEFAULT_HOTKEY: &'static str = "Ctrl+Alt+Q";

    /// The settings with every value in range: opacity clamped to 0.3-1 (a
    /// non-number becomes the default), an empty hotkey or log path unset.
    pub fn normalised(mut self) -> Settings {
        self.opacity = if self.opacity.is_finite() {
            self.opacity.clamp(Settings::MIN_OPACITY, 1.0)
        } else {
            Settings::DEFAULT_OPACITY
        };
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
    /// The current act's available and in-progress quests, worth-it first.
    pub next_up: Vec<QuestRow>,
    pub detail: Option<QuestDetail>,
    pub settings: Settings,
    /// Worth-it quests of the current act available now (the collapsed badge).
    pub available_worth_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToastKind {
    Available,
    StepDone,
    QuestDone,
}

impl ToastKind {
    /// The icon id of the brief a toast of this kind shows.
    pub fn icon(self) -> &'static str {
        match self {
            ToastKind::Available => "status-available",
            ToastKind::StepDone => "status-in-progress",
            ToastKind::QuestDone => "status-done",
        }
    }
}

/// A short notification. The shell sends it as `qt:toast`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Toast {
    pub id: u64,
    pub kind: ToastKind,
    pub quest_id: String,
    pub text: String,
    pub icon: String,
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

/// Each step's phase, and the index of the step to do next.
fn step_phases(quest: &Quest, state: &QuestState) -> (Vec<StepPhase>, Option<usize>) {
    let n = quest.steps.len();
    if state.status == Status::Done {
        return (vec![StepPhase::Done; n], None);
    }
    let reached = state
        .current_step
        .as_deref()
        .and_then(|id| quest.steps.iter().position(|s| s.id == id));
    let next = reached.map_or(0, |r| r + 1);
    let next = (next < n).then_some(next);
    let open = matches!(
        state.status,
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
    let current_step = if state.status == Status::InProgress {
        let (_, next) = step_phases(quest, state);
        let reached = state
            .current_step
            .as_deref()
            .and_then(|id| quest.steps.iter().position(|s| s.id == id));
        next.or(reached).map(|i| quest.steps[i].title.clone())
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
        rewards: quest
            .rewards
            .iter()
            .map(|r| RewardView {
                category: r.category,
                icon: reward_icon(r).to_string(),
                text: r.text.clone(),
            })
            .collect(),
        current_step,
    }
}

fn detail(data: &QuestData, quest: &Quest, state: &QuestState) -> QuestDetail {
    let (phases, _) = step_phases(quest, state);
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
            .map(|(s, phase)| StepView {
                id: s.id.clone(),
                title: s.title.clone(),
                text: s.text.clone(),
                state: phase,
                ticked: state.steps.iter().any(|t| t.id == s.id && t.ticked),
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
    let shown = |q: &Quest, s: &QuestState| {
        (s.status != Status::Skipped || settings.show_skipped)
            && (!settings.only_worth_it || q.worth == Worth::Yes)
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
    let mut next: Vec<(&Quest, &QuestState)> = in_act
        .iter()
        .copied()
        .filter(|(q, s)| {
            matches!(s.status, Status::Available | Status::InProgress) && shown(q, s)
        })
        .collect();
    // Stable: data order within the same worth and status.
    next.sort_by_key(|(q, s)| (worth_rank(q.worth), s.status != Status::InProgress));
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
        })
        .collect();
    let character = engine
        .current()
        .and_then(|n| characters.iter().find(|c| c.name == n).cloned());

    let states = character
        .as_ref()
        .map(|c| engine.quest_states(&c.name))
        .unwrap_or_default();
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
    let dropped = engine.dropped_count();
    if dropped > 0 {
        derived.push(StatusLine::info(format!(
            "{dropped} log events came before any character and were not counted."
        )));
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
// Toasts.

/// The toast a transition makes, if any: only a live one, and only a quest
/// becoming available, a step reached, or a quest done.
fn toast_for(data: &QuestData, t: &Transition) -> Option<(ToastKind, String)> {
    if !t.live {
        return None;
    }
    let quest = data.quest(&t.quest_id)?;
    if t.to == Status::Done && t.from != Status::Done {
        return Some((ToastKind::QuestDone, format!("Quest done: {}", quest.name)));
    }
    if t.to == Status::Available && t.from == Status::Locked {
        return Some((ToastKind::Available, format!("Now available: {}", quest.name)));
    }
    if t.to == Status::InProgress && t.step_to.is_some() && t.step_to != t.step_from {
        let step = quest
            .steps
            .iter()
            .find(|s| Some(&s.id) == t.step_to.as_ref())?;
        return Some((
            ToastKind::StepDone,
            format!("{}: {}", quest.name, step.title),
        ));
    }
    None
}

/// The toast stack: each toast shows once, for [`TOAST_MS`], at most
/// [`MAX_TOASTS`] at a time; a newer toast pushes the oldest out.
#[derive(Debug, Clone, Default)]
pub struct ToastQueue {
    next_id: u64,
    shown: VecDeque<(Toast, u64)>,
}

impl ToastQueue {
    pub fn new() -> ToastQueue {
        ToastQueue {
            next_id: 1,
            shown: VecDeque::new(),
        }
    }

    /// Turns a batch of transitions into toasts at `now_ms` and returns the
    /// new ones for the shell to send: the newest [`MAX_TOASTS`] at most.
    /// Catch-up transitions make none.
    pub fn push(&mut self, data: &QuestData, transitions: &[Transition], now_ms: u64) -> Vec<Toast> {
        self.expire(now_ms);
        let mut made: Vec<(ToastKind, &Transition, String)> = transitions
            .iter()
            .filter_map(|t| toast_for(data, t).map(|(k, text)| (k, t, text)))
            .collect();
        if made.len() > MAX_TOASTS {
            made.drain(..made.len() - MAX_TOASTS);
        }
        let mut out = Vec::with_capacity(made.len());
        for (kind, t, text) in made {
            let toast = Toast {
                id: self.next_id.max(1),
                kind,
                quest_id: t.quest_id.clone(),
                text,
                icon: kind.icon().to_string(),
            };
            self.next_id = toast.id + 1;
            self.shown.push_back((toast.clone(), now_ms + TOAST_MS));
            while self.shown.len() > MAX_TOASTS {
                self.shown.pop_front();
            }
            out.push(toast);
        }
        out
    }

    /// Drops the toasts whose time is up.
    pub fn expire(&mut self, now_ms: u64) {
        self.shown.retain(|(_, until)| *until > now_ms);
    }

    /// The toasts on screen at `now_ms`, oldest first.
    pub fn visible(&self, now_ms: u64) -> Vec<Toast> {
        self.shown
            .iter()
            .filter(|(_, until)| *until > now_ms)
            .map(|(t, _)| t.clone())
            .collect()
    }
}
