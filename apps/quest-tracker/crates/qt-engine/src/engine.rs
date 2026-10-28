//! The per-character quest state machine.
//!
//! Events from `qt-logline` go to one character (Context "Character
//! detection rules"); the event-based signals they fire are kept as that
//! character's evidence, a set of signal ids per quest, so applying an event
//! twice changes nothing and a whole replay equals the same lines fed in any
//! chunks. A quest's status is never stored: it is computed from the
//! evidence, the character's level, the other quests' statuses and the
//! user's overrides (Context "Quest state rules"), and every change of a
//! status or current step is reported as a [`Transition`] carrying the
//! line's catch-up/live flag. A fired `start` signal counts under rule 4
//! like a step: the quest is in progress with no step reached, so its first
//! step is the current one (Context "Live 2 fixes (owner scope)"). A
//! skipped quest goes on recording evidence: its status stays skipped, and
//! [`QuestState::tracked_status`] and its current step say what that evidence
//! gives, which is what Unskip shows (Context "Skipped quests stay tracked").
//!
//! Each stored character keeps the seed of the Riverbank (`G1_1`) instance
//! it was last decided to be in. A level-1 entry whose seed is that of
//! exactly one stored character, and of no parked new character, is that
//! character's relog, decided at the entry: it becomes active, the pin
//! stays, only [`CharacterNote::Relogged`] is posted, and the entry and what
//! follows are routed as usual (Context "Riverbank relog by the instance
//! seed"). The seed passes from the new character to the name a level-up
//! gives it or to the character a pin hands its progress to, and a relog
//! records its entry's seed; a routed event never does.
//!
//! Any other level-1 entry to The Riverbank first opens a relog window and
//! changes nothing else: no note, the pin and the active character kept, and
//! the window's events wait. A character build line naming a stored
//! character within 5 s of the entry, before any other area entry, makes it
//! a relog: only [`CharacterNote::Relogged`] is posted and the events go to
//! that character. Any other close (another area entry, a line more than 5 s
//! later, a level-up, a session marker, or [`Engine::expire_relog_window`])
//! lets the entry take effect: a new character with no name yet starts (or
//! resumes), the pin is released, the events held before the entry are
//! dropped (another character's: a new character's first area is The
//! Riverbank), and the window's events go to it, so its Act 1 progress shows; the first level-up of a name not seen joining or
//! leaving names it. A session marker parks the new character, and a later
//! level-1 entry to the same instance (seed) resumes it (Context "Launch 7
//! fixes (owner scope)", "Launch 8: wait, then decide").
//!
//! A screen event ([`Engine::apply_screen`]) is routed as an NPC line is
//! and fires only `screen` signals, which no log line fires: in the current
//! area, on an open step, of one quest, the first in data order. It never
//! reads or moves the log's time. [`Engine::screen_demand`] lists the
//! screen signals that would fire now, for the reader to look for (Context
//! "Design: quest data and the engine").
//!
//! A level-up line is not routed: it names or levels a character by the
//! rules above, and only where the engine takes it as that character's own
//! does it fire the `level_up` signals of its level in the current area for
//! that character (refused and counted while the area is unknown).
//!
//! What persists ([`Engine::to_json`]) is the characters, the pin and the
//! active name. The unnamed new character is a separate record
//! ([`Engine::new_character_json`]), never among the characters; the user's
//! quest actions on it while it is open and no named character is current
//! ([`Engine::quest_target`]) are its overrides, kept in that record and
//! passed with its progress to the character it closes into. Session
//! memory (the names seen joining or leaving, the held events, the current
//! area, the count of signals refused while the area was unknown) is never
//! persisted: other players' names are kept in memory for one session only.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

use qt_logline::{parse_line, Allowlist, Event, EventKind, Timestamp};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::data::{classify_stat, Detection, QuestData, ScreenWhat, Signal, Status, SCHEMA};

/// Whether a character is known to be the player's own (rule 1, or the
/// user's confirmation) or only taken to be by a level-up (rule 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ownership {
    Own,
    Unconfirmed,
}

/// The user's manual overrides of one quest.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Overrides {
    /// `Some(true)` marked done, `Some(false)` marked not done, `None` unset.
    #[serde(default)]
    pub done: Option<bool>,
    #[serde(default)]
    pub skipped: bool,
    /// Ticked step ids.
    #[serde(default)]
    pub ticks: BTreeSet<String>,
}

impl Overrides {
    fn is_empty(&self) -> bool {
        self.done.is_none() && !self.skipped && self.ticks.is_empty()
    }
}

/// One character's persisted state: what the store writes to
/// `characters/<safe name>.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharacterState {
    pub name: String,
    /// Empty until a level-up line names it.
    #[serde(default)]
    pub class: String,
    /// 0 until a level-up line names it.
    #[serde(default)]
    pub level: u32,
    pub ownership: Ownership,
    /// Quest id -> the ids of its event-based signals that fired.
    #[serde(default)]
    pub evidence: BTreeMap<String, BTreeSet<String>>,
    /// Quest id -> the user's overrides.
    #[serde(default)]
    pub overrides: BTreeMap<String, Overrides>,
    /// The seed of the Riverbank (`G1_1`) instance this character was last
    /// decided to be in: a level-1 entry with this seed, of no other stored
    /// character and no parked new character, is its relog. Set by decisions
    /// only (the new character's seed passing to the name a level-up gives
    /// it or to the character a pin hands its progress to, and a relog),
    /// never by a routed event. Absent in older records, which load.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub riverbank_seed: Option<u64>,
}

impl CharacterState {
    pub fn new(name: &str, ownership: Ownership) -> CharacterState {
        CharacterState {
            name: name.to_string(),
            class: String::new(),
            level: 0,
            ownership,
            evidence: BTreeMap::new(),
            overrides: BTreeMap::new(),
            riverbank_seed: None,
        }
    }
}

/// A character as the picker lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharacterInfo {
    pub name: String,
    pub class: String,
    pub level: u32,
    pub own: bool,
    pub pinned: bool,
    pub active: bool,
}

/// One step of one quest for one character.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepState {
    pub id: String,
    /// One of the step's signals fired.
    pub fired: bool,
    /// The user ticked it.
    pub ticked: bool,
}

/// One quest's computed state for one character.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestState {
    pub quest_id: String,
    pub status: Status,
    /// The status the state rules give with this quest's skip left out:
    /// what Unskip would show. Equal to `status` for a quest not skipped. A
    /// skipped quest keeps recording its evidence, so this keeps moving
    /// while `status` stays skipped (Context "Skipped quests stay tracked").
    pub tracked_status: Status,
    /// The highest step that fired or was ticked, skipped or not.
    pub current_step: Option<String>,
    pub steps: Vec<StepState>,
    pub done_override: Option<bool>,
    pub skipped: bool,
}

/// A change of one quest's status or current step for one character.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transition {
    /// Empty for the new character from The Riverbank, which has no name
    /// until its first level-up.
    pub character: String,
    pub quest_id: String,
    pub from: Status,
    pub to: Status,
    pub step_from: Option<String>,
    pub step_to: Option<String>,
    /// The line was written after the tailer started (or the change is the
    /// user's own action); only live transitions make the panel animate a
    /// row (`qt:row-change`), catch-up ones only change the view.
    pub live: bool,
}

/// The user's actions on a quest (the IPC contract's `quest_action`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestAction {
    MarkDone,
    UnmarkDone,
    Skip,
    Unskip,
    /// Clears the overrides, not the evidence.
    Reset,
}

impl FromStr for QuestAction {
    type Err = String;

    fn from_str(s: &str) -> Result<QuestAction, String> {
        serde_json::from_value(Value::String(s.to_string()))
            .map_err(|_| format!("unknown quest action {s:?}"))
    }
}

/// Why a user action or a state load was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineError(pub String);

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for EngineError {}

/// Where a signal sits in its quest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    Unlock,
    Start,
    Step(usize),
    Done,
    Window,
}

/// A signal with its stable id (`unlock/0`, `start/0`, `step/<step id>/1`,
/// `done/0`, `window_closes/0`), which is what evidence stores.
#[derive(Debug, Clone)]
struct Compiled {
    id: String,
    role: Role,
    signal: Signal,
}

/// An event waiting for a character, with the area it happened in.
#[derive(Debug, Clone)]
struct Held {
    event: Happening,
    area: Option<String>,
    live: bool,
}

/// What a routed event is: a log line's, or the screen reader's.
#[derive(Debug, Clone)]
enum Happening {
    Log(Event),
    Screen(ScreenEvent),
}

impl Held {
    fn is_screen(&self) -> bool {
        matches!(self.event, Happening::Screen(_))
    }
}

/// What the screen reader saw, as the engine takes it
/// ([`Engine::apply_screen`]): a `screen` signal's kind and the name it
/// reads, the boss for `bar_gone` and `kill_remark`, the NPC for
/// `quest_complete`. Names come from the shipped screen signals
/// ([`Engine::screen_demand`]), never from what was read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenEvent {
    pub what: ScreenWhat,
    pub name: String,
}

impl ScreenEvent {
    pub fn bar_gone(boss: &str) -> ScreenEvent {
        ScreenEvent {
            what: ScreenWhat::BarGone,
            name: boss.to_string(),
        }
    }

    pub fn kill_remark(boss: &str) -> ScreenEvent {
        ScreenEvent {
            what: ScreenWhat::KillRemark,
            name: boss.to_string(),
        }
    }

    pub fn quest_complete(npc: &str) -> ScreenEvent {
        ScreenEvent {
            what: ScreenWhat::QuestComplete,
            name: npc.to_string(),
        }
    }
}

/// What one screen event did, for the reader's diagnostic line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ScreenOutcome {
    /// It fired the screen signal on this step of this quest.
    Ticked { quest_id: String, step_id: String },
    /// The current area was unknown: refused, and counted with the log's
    /// own refusals ([`Engine::refused_count`]).
    AreaUnknown,
    /// No screen signal on an open step of an available or in-progress quest
    /// matched it in the current area.
    NoOpenStep,
    /// No character takes events now (a Riverbank entry's relog window is
    /// open, or no character is known yet), so it waits with the log's
    /// events and is applied, live, to the character they go to.
    Waiting,
}

/// [`Engine::apply_screen`]'s result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenResult {
    pub outcome: ScreenOutcome,
    pub transitions: Vec<Transition>,
}

/// A screen signal that would fire now ([`Engine::screen_demand`]): what
/// the reader should look for, and where it would go.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenDemand {
    pub quest_id: String,
    pub step_id: String,
    /// The signal's id, `step/<step id>/<i>`, as evidence stores it.
    pub signal_id: String,
    pub what: ScreenWhat,
    /// The boss or NPC the signal names.
    pub name: String,
    /// The area code it is tied to (the current area).
    pub area: String,
}

/// An unconfirmed character made active by a level-up (rule 4): what it was
/// before, and what was attributed to it since, so a later join or leave
/// line can give it all back to the hold.
#[derive(Debug, Clone)]
struct Pending {
    name: String,
    before: Option<CharacterState>,
    released: Vec<Held>,
    since: Vec<Held>,
}

/// The area code whose level-1 entry starts a new character.
const NEW_CHARACTER_AREA: &str = "G1_1";

/// How long after a level-1 Riverbank entry a character build line can still
/// make it a relog. The measured lines came within 2 s.
const RELOG_SECONDS: i64 = 5;

/// The new character a level-1 entry to The Riverbank starts: its progress
/// so far, kept until a level-up names it. At most one exists; it is open
/// (getting events) or parked (a session marker came first).
#[derive(Debug, Clone)]
struct NewCharacter {
    /// Its state, with an empty name and level 1.
    state: CharacterState,
    /// Events given to it, counted as dropped if another instance replaces
    /// it.
    events: u64,
    /// The pin its start released; the character it names is pinned in its
    /// place. Parking gives it back, so a parked record holds none.
    released_pin: Option<String>,
    /// The seed of the `G1_1` instance that opened it.
    seed: u64,
    /// False once a session marker parked it.
    open: bool,
}

/// A level-1 Riverbank entry not yet decided: while a character build line
/// can still make it a relog, it has changed nothing, and the events since
/// it wait here for the character it decides on. Session memory, never in a
/// record.
#[derive(Debug, Clone)]
struct RelogWindow {
    /// When the entry was.
    since: Timestamp,
    /// The seed of the entered instance.
    seed: u64,
    /// The events since the entry (the entry itself first), oldest first.
    taken: Vec<Held>,
}

/// The new character's record as [`Engine::new_character_json`] gives it
/// out and [`Engine::load_new_character`] takes it back.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NewCharacterRecord {
    schema: u64,
    state: CharacterState,
    events: u64,
    released_pin: Option<String>,
    seed: u64,
    open: bool,
}

/// What happened to the new character from The Riverbank, for the shell's
/// diagnostic log, taken with [`Engine::take_character_notes`]. Written for
/// catch-up lines as well as live ones.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CharacterNote {
    /// A level-1 Riverbank entry started one, releasing `released_pin`.
    Started { released_pin: Option<String> },
    /// A level-1 entry to the parked record's instance (same seed) resumed
    /// it, releasing `released_pin`.
    Resumed { released_pin: Option<String> },
    /// An entry to another instance replaced the parked record; its
    /// `events` are counted as dropped new-character events.
    Replaced { events: u64 },
    /// A session marker parked the open record and gave back its pin.
    Parked { restored_pin: Option<String> },
    /// A level-1 Riverbank entry was `name`'s relog: its seed was that of
    /// the stored character `name` alone, or a character build line named
    /// it within the relog window. The entry started nothing, the pin was
    /// never released and `name` is active. The only note of that entry.
    Relogged { name: String },
    /// A level-up named it `name`.
    Named { name: String },
    /// The user pinned `name` while it was open, which took its progress.
    Pinned { name: String },
}

/// How the open new character closes.
#[derive(Debug, Clone, Copy)]
enum Close<'a> {
    /// A level-up named it, with this class and level.
    LevelUp(&'a str, u32),
    /// The user pinned a stored character.
    Pin,
}

/// Who an event or a change goes to: a stored character, or the unnamed new
/// character.
#[derive(Debug, Clone, Copy)]
enum Target<'a> {
    Named(&'a str),
    New,
}

impl Target<'_> {
    fn name(&self) -> &str {
        match self {
            Target::Named(n) => n,
            Target::New => "",
        }
    }
}

/// The quest engine: the data, the characters and this session's memory.
#[derive(Debug, Clone)]
pub struct Engine {
    data: Arc<QuestData>,
    allow: Allowlist,
    compiled: Vec<Vec<Compiled>>,
    index: HashMap<String, usize>,
    chars: BTreeMap<String, CharacterState>,
    pinned: Option<String>,
    active: Option<String>,
    // Session memory, never persisted.
    excluded: HashSet<String>,
    area: Option<String>,
    held: Vec<Held>,
    pending: Option<Pending>,
    /// The new character's record, open or parked; given out and taken back
    /// apart from [`Engine::to_json`].
    new_char: Option<NewCharacter>,
    /// The level-1 Riverbank entry still waiting to be decided.
    relog: Option<RelogWindow>,
    /// Held events dropped at a session marker with no character, or when a
    /// new character from The Riverbank starts or resumes.
    dropped: u64,
    /// Events of a parked new character another instance replaced.
    dropped_new: u64,
    notes: Vec<CharacterNote>,
    client_version: Option<String>,
    /// Area-tied signals refused because the area was unknown: the running
    /// count, and the quest ids not yet taken by the shell.
    refused: u64,
    refused_quests: Vec<String>,
}

impl Engine {
    pub fn new(data: Arc<QuestData>) -> Engine {
        let compiled = data
            .quests
            .iter()
            .map(|q| {
                let mut out = Vec::new();
                for (i, s) in q.unlock.iter().enumerate() {
                    out.push(Compiled {
                        id: format!("unlock/{i}"),
                        role: Role::Unlock,
                        signal: s.clone(),
                    });
                }
                for (i, s) in q.start.iter().enumerate() {
                    out.push(Compiled {
                        id: format!("start/{i}"),
                        role: Role::Start,
                        signal: s.clone(),
                    });
                }
                for (k, step) in q.steps.iter().enumerate() {
                    for (i, s) in step.signals.iter().enumerate() {
                        out.push(Compiled {
                            id: format!("step/{}/{i}", step.id),
                            role: Role::Step(k),
                            signal: s.clone(),
                        });
                    }
                }
                for (i, s) in q.done.iter().enumerate() {
                    out.push(Compiled {
                        id: format!("done/{i}"),
                        role: Role::Done,
                        signal: s.clone(),
                    });
                }
                for (i, s) in q.window_closes.iter().enumerate() {
                    out.push(Compiled {
                        id: format!("window_closes/{i}"),
                        role: Role::Window,
                        signal: s.clone(),
                    });
                }
                out
            })
            .collect();
        let index = data
            .quests
            .iter()
            .enumerate()
            .map(|(i, q)| (q.id.clone(), i))
            .collect();
        Engine {
            allow: data.allowlist(),
            data,
            compiled,
            index,
            chars: BTreeMap::new(),
            pinned: None,
            active: None,
            excluded: HashSet::new(),
            area: None,
            held: Vec::new(),
            pending: None,
            new_char: None,
            relog: None,
            dropped: 0,
            dropped_new: 0,
            notes: Vec::new(),
            client_version: None,
            refused: 0,
            refused_quests: Vec::new(),
        }
    }

    pub fn data(&self) -> &Arc<QuestData> {
        &self.data
    }

    /// The parser allowlist built from the data.
    pub fn allowlist(&self) -> &Allowlist {
        &self.allow
    }

    // -- feeding ------------------------------------------------------------

    /// Parses one log line with the data's allowlist and applies it.
    pub fn feed_line(&mut self, line: &str, live: bool) -> Vec<Transition> {
        match parse_line(line, &self.allow) {
            Some(ev) => self.apply(&ev, live),
            None => Vec::new(),
        }
    }

    /// Applies one event. `live` is the tailer's flag (false for catch-up).
    pub fn apply(&mut self, ev: &Event, live: bool) -> Vec<Transition> {
        let mut out = Vec::new();
        // A line more than RELOG_SECONDS after the entry can no longer make
        // it a relog: the entry takes effect before the line is handled.
        self.close_expired_window(ev.time, &mut out);
        match &ev.kind {
            EventKind::SessionStart => self.end_session_into(&mut out),
            EventKind::ClientVersion { tag } => self.client_version = Some(tag.clone()),
            EventKind::AreaEntered { code, level, seed } => {
                // Any area entry decides the entry before it.
                self.close_relog_window(&mut out);
                self.area = Some(code.clone());
                if code == NEW_CHARACTER_AREA && *level == 1 && !self.new_open() {
                    match self.seed_owner(*seed) {
                        // The instance of exactly one stored character: its
                        // relog, decided at once with no window; the entry
                        // is then routed as usual.
                        Some(name) => self.relogged(&name, *seed, Vec::new(), &mut out),
                        None => {
                            self.relog = Some(RelogWindow {
                                since: ev.time,
                                seed: *seed,
                                taken: Vec::new(),
                            })
                        }
                    }
                }
                self.route(ev, live, &mut out);
            }
            EventKind::CharacterNamed { name } => self.character_named(name, ev.time, &mut out),
            EventKind::AreaName { .. }
            | EventKind::NpcLine { .. }
            | EventKind::SystemLine { .. }
            | EventKind::Points { .. } => self.route(ev, live, &mut out),
            EventKind::LevelUp { name, class, level } => {
                // A level-up decides the entry, then names or levels as usual,
                // and fires its `level_up` signals there.
                self.close_relog_window(&mut out);
                self.level_up(&ev.kind, name, class, *level, live, &mut out)
            }
            // Rule 7: a death never changes the active character.
            EventKind::Slain { .. } => {}
            EventKind::Received { name, .. } | EventKind::Lost { name, .. } => {
                self.received(name, ev, live, &mut out)
            }
            EventKind::Party { name, .. } => self.party(name, &mut out),
        }
        out
    }

    /// Ends the current session (a `LOG FILE OPENING` line does this itself):
    /// an undecided Riverbank entry first takes effect (a new character
    /// starts or resumes), events still held with no character to give them
    /// to are dropped and counted, an open new character is parked and gives
    /// back the pin its start released, and the session's memory is cleared.
    pub fn end_session(&mut self) {
        let mut out = Vec::new();
        self.end_session_into(&mut out);
    }

    fn end_session_into(&mut self, out: &mut Vec<Transition>) {
        self.close_relog_window(out);
        self.dropped += self.held.len() as u64;
        self.held.clear();
        if let Some(nc) = self.new_char.as_mut().filter(|nc| nc.open) {
            nc.open = false;
            let pin = nc
                .released_pin
                .take()
                .filter(|p| self.chars.contains_key(p));
            if pin.is_some() {
                self.pinned = pin.clone();
            }
            self.notes.push(CharacterNote::Parked { restored_pin: pin });
        }
        self.pending = None;
        self.active = None;
        self.excluded.clear();
        self.area = None;
    }

    /// Seeds the session's current area, as read back from the log before a
    /// tailer's start position ([`crate::area_from_log`]); `None` makes it
    /// unknown. It sets the area only: no event is routed, no evidence
    /// recorded and no transition made, and like an area read live it stays
    /// session memory, out of [`Engine::to_json`].
    pub fn seed_area(&mut self, code: Option<&str>) {
        self.area = code.map(str::to_string);
    }

    /// The session's current area code, `None` while unknown.
    pub fn current_area(&self) -> Option<&str> {
        self.area.as_deref()
    }

    // -- the screen reader ---------------------------------------------------

    /// Applies one screen event (Context "Design: quest data and the
    /// engine"). It is routed as an NPC line is (the pin, the open new
    /// character, the active character, else it waits) and is live. It
    /// fires a `screen` signal when the current area is known and is the
    /// signal's, the signal sits on an open step (after the furthest step
    /// fired or ticked) and the quest's tracked status is available or in
    /// progress; at most one quest fires, the first in data order. With the
    /// area unknown it is refused and counted. It fires no other kind of
    /// signal, and it neither reads nor moves the log's time: an undecided
    /// Riverbank entry stays undecided, and the event waits in its window.
    pub fn apply_screen(&mut self, ev: &ScreenEvent) -> ScreenResult {
        let mut out = Vec::new();
        let held = Held {
            event: Happening::Screen(ev.clone()),
            area: self.area.clone(),
            live: true,
        };
        let outcome = self
            .route_held(held, &mut out)
            .expect("a routed screen event has an outcome");
        ScreenResult {
            outcome,
            transitions: out,
        }
    }

    /// The screen signals a screen event would fire now, for the reader to
    /// look for: those of the character a screen event goes to, in the
    /// current area, on an open step of an available or in-progress quest,
    /// in data order. Empty while the area is unknown, while a Riverbank
    /// entry is undecided, or with no character to go to.
    pub fn screen_demand(&self) -> Vec<ScreenDemand> {
        let (Some(area), Some(target)) = (self.area.as_deref(), self.screen_target()) else {
            return Vec::new();
        };
        let states = self.states_of(target);
        self.open_screen_signals(&states)
            .into_iter()
            .filter_map(|(qi, k, c)| {
                let Signal::Screen { area: want, .. } = &c.signal else {
                    return None;
                };
                let (what, name) = c.signal.screen()?;
                (want == area).then(|| ScreenDemand {
                    quest_id: self.data.quests[qi].id.clone(),
                    step_id: self.data.quests[qi].steps[k].id.clone(),
                    signal_id: c.id.clone(),
                    what,
                    name: name.to_string(),
                    area: want.clone(),
                })
            })
            .collect()
    }

    /// Who an event routed now would go to; `None` when it would wait.
    fn screen_target(&self) -> Option<Target<'_>> {
        if self.relog.is_some() {
            None
        } else if let Some(name) = self.pinned.as_deref() {
            Some(Target::Named(name))
        } else if self.new_open() {
            Some(Target::New)
        } else {
            self.active.as_deref().map(Target::Named)
        }
    }

    /// Every `screen` signal on an open step of a quest whose tracked
    /// status is available or in progress, in data order, as (quest index,
    /// step index, signal), whatever its area.
    fn open_screen_signals(&self, states: &[QuestState]) -> Vec<(usize, usize, &Compiled)> {
        let mut found = Vec::new();
        for (qi, signals) in self.compiled.iter().enumerate() {
            let st = &states[qi];
            if !matches!(st.tracked_status, Status::Available | Status::InProgress) {
                continue;
            }
            let reached = st.steps.iter().rposition(|s| s.fired || s.ticked);
            for c in signals {
                if let (Role::Step(k), Signal::Screen { .. }) = (c.role, &c.signal) {
                    if reached.is_none_or(|r| k > r) {
                        found.push((qi, k, c));
                    }
                }
            }
        }
        found
    }

    /// Fires the first open screen signal a screen event matches for the
    /// target, in `area`; with the area unknown, refuses and counts it.
    fn apply_screen_to(
        &mut self,
        target: Target,
        ev: &ScreenEvent,
        area: Option<&str>,
        live: bool,
        out: &mut Vec<Transition>,
    ) -> ScreenOutcome {
        let states = self.states_of(target);
        let open = self.open_screen_signals(&states);
        let reads = |c: &Compiled| c.signal.screen() == Some((ev.what, ev.name.as_str()));
        let Some(area) = area else {
            // Refused: counted once, for the quest it would have fired.
            if let Some(&(qi, _, _)) = open.iter().find(|(_, _, c)| reads(c)) {
                let qid = self.data.quests[qi].id.clone();
                self.refused += 1;
                self.refused_quests.push(qid);
            }
            return ScreenOutcome::AreaUnknown;
        };
        let hit = open.iter().find(|(_, _, c)| {
            reads(c) && matches!(&c.signal, Signal::Screen { area: want, .. } if want == area)
        });
        let Some(&(qi, k, c)) = hit else {
            return ScreenOutcome::NoOpenStep;
        };
        let (qid, sid) = (self.data.quests[qi].id.clone(), c.id.clone());
        let step_id = self.data.quests[qi].steps[k].id.clone();
        let quest_id = qid.clone();
        self.change_to(target, live, out, |e| {
            e.state_mut(target)
                .is_some_and(|ch| ch.evidence.entry(qid).or_default().insert(sid))
        });
        ScreenOutcome::Ticked { quest_id, step_id }
    }

    /// Gives an event to the pinned character, the new character from The
    /// Riverbank or the active character, or holds it. While a Riverbank
    /// entry is undecided, the event waits for the character it decides on.
    fn route(&mut self, ev: &Event, live: bool, out: &mut Vec<Transition>) {
        let held = Held {
            event: Happening::Log(ev.clone()),
            area: self.area.clone(),
            live,
        };
        self.route_held(held, out);
    }

    /// [`Engine::route`] for an event of either source; a screen event's
    /// outcome comes back (`Waiting` while it waits).
    fn route_held(&mut self, held: Held, out: &mut Vec<Transition>) -> Option<ScreenOutcome> {
        let waiting = held.is_screen().then_some(ScreenOutcome::Waiting);
        if let Some(w) = self.relog.as_mut() {
            // Not given to anyone yet, so nothing is refused for anyone yet.
            w.taken.push(held);
            waiting
        } else if let Some(name) = self.pinned.clone() {
            self.receive(Target::Named(&name), held, out)
        } else if self.new_open() {
            self.receive(Target::New, held, out)
        } else if let Some(name) = self.active.clone() {
            self.receive(Target::Named(&name), held, out)
        } else {
            // Held: no character gets it now, so nothing is refused for one.
            self.held.push(held);
            waiting
        }
    }

    /// Gives a routed event to the character it goes to.
    fn receive(
        &mut self,
        target: Target,
        held: Held,
        out: &mut Vec<Transition>,
    ) -> Option<ScreenOutcome> {
        match target {
            Target::Named(name) => {
                if let Some(p) = self.pending.as_mut() {
                    if p.name == name {
                        p.since.push(held.clone());
                    }
                }
            }
            Target::New => {
                if let Some(nc) = self.new_char.as_mut() {
                    nc.events += 1;
                }
            }
        }
        self.give(target, &held, out)
    }

    /// Gives released held events to a named character, counting refusals
    /// exactly as for an event routed to it.
    fn release_to(&mut self, name: &str, released: &[Held], out: &mut Vec<Transition>) {
        for h in released {
            self.give(Target::Named(name), h, out);
        }
    }

    /// Applies an event to the character it was given to: a log event's
    /// signals, counting its refusals while the area is unknown, or a screen
    /// event by the screen fire rule, whose outcome comes back.
    fn give(&mut self, target: Target, held: &Held, out: &mut Vec<Transition>) -> Option<ScreenOutcome> {
        match &held.event {
            Happening::Log(ev) => {
                if held.area.is_none() {
                    self.count_refusals(&ev.kind, target);
                }
                self.apply_to(target, &ev.kind, held.area.as_deref(), held.live, out);
                None
            }
            Happening::Screen(ev) => {
                Some(self.apply_screen_to(target, ev, held.area.as_deref(), held.live, out))
            }
        }
    }

    /// With the area unknown, counts every area-tied signal the event matches
    /// in all but its area, of a quest that is neither done nor skipped for
    /// the character the event goes to: it is refused, and the shell logs the
    /// quest id. Counted once per event, when it happens. A signal refused
    /// because the area is known and different is the normal case and is not
    /// counted, and neither is one of a quest already finished or set aside,
    /// which the signal could not have changed.
    fn count_refusals(&mut self, ev: &EventKind, target: Target) {
        let mut hits: Vec<usize> = Vec::new();
        for (qi, signals) in self.compiled.iter().enumerate() {
            for c in signals {
                let Some(want) = signal_area(&c.signal) else {
                    continue;
                };
                if fires_on(&c.signal, ev, Some(want)) {
                    hits.push(qi);
                }
            }
        }
        if hits.is_empty() {
            return;
        }
        let states = self.states_of(target);
        for qi in hits {
            if matches!(states[qi].status, Status::Done | Status::Skipped) {
                continue;
            }
            self.refused += 1;
            self.refused_quests.push(self.data.quests[qi].id.clone());
        }
    }

    /// Records the event-based signals a log event fires as the target's
    /// evidence.
    fn apply_to(
        &mut self,
        target: Target,
        ev: &EventKind,
        area: Option<&str>,
        live: bool,
        out: &mut Vec<Transition>,
    ) {
        let mut hits: Vec<(String, String)> = Vec::new();
        for (qi, signals) in self.compiled.iter().enumerate() {
            for c in signals {
                if fires_on(&c.signal, ev, area) {
                    hits.push((self.data.quests[qi].id.clone(), c.id.clone()));
                }
            }
        }
        if hits.is_empty() {
            return;
        }
        self.change_to(target, live, out, |e| {
            let Some(ch) = e.state_mut(target) else {
                return false;
            };
            let mut changed = false;
            for (qid, id) in hits {
                changed |= ch.evidence.entry(qid).or_default().insert(id);
            }
            changed
        });
    }

    /// A level-up line (`ev`): names the new character, levels a known one
    /// or makes an unknown name active as unconfirmed, by the rules below;
    /// then, only where it was taken as that character's own, fires the
    /// level-up's `level_up` signals for it ([`Engine::fire_level_up`]),
    /// after the branch's own naming and levelling, so a revoked
    /// unconfirmed character loses that evidence with the rest of its state.
    fn level_up(
        &mut self,
        ev: &EventKind,
        name: &str,
        class: &str,
        level: u32,
        live: bool,
        out: &mut Vec<Transition>,
    ) {
        if let Some(pinned) = self.pinned.clone() {
            // Rule 6: while pinned, detection only updates its level.
            if pinned == name {
                self.set_level(name, class, level, live, out);
                self.fire_level_up(ev, name, live, out);
            }
            return;
        }
        if self.new_open() {
            // The new character from The Riverbank: the first level-up of a
            // name not seen joining or leaving this session names it.
            if !self.excluded.contains(name) {
                self.close_new_character(name, Close::LevelUp(class, level), live, out);
                self.fire_level_up(ev, name, live, out);
            }
            return;
        }
        let known = self.chars.get(name).map(|c| c.ownership);
        if known == Some(Ownership::Own) {
            // Rule 3.
            self.set_level(name, class, level, live, out);
            self.activate(name, out);
            self.fire_level_up(ev, name, live, out);
        } else if self.excluded.contains(name) {
            // Rule 2: a party member this session.
        } else if self.active.as_deref() == Some(name) {
            self.set_level(name, class, level, live, out);
            self.fire_level_up(ev, name, live, out);
        } else {
            // Rule 4: active as unconfirmed, revocable by a join/leave line.
            self.pending = None;
            let before = self.chars.get(name).cloned();
            self.change(name, live, out, |e| {
                e.chars
                    .entry(name.to_string())
                    .or_insert_with(|| CharacterState::new(name, Ownership::Unconfirmed));
                true
            });
            self.set_level(name, class, level, live, out);
            self.active = Some(name.to_string());
            let released = std::mem::take(&mut self.held);
            self.pending = Some(Pending {
                name: name.to_string(),
                before,
                released: released.clone(),
                since: Vec::new(),
            });
            self.release_to(name, &released, out);
            self.fire_level_up(ev, name, live, out);
        }
    }

    /// Fires the `level_up` signals a level-up line fires for `name`, the
    /// character the engine took it as: in the current area, kept as
    /// evidence like a log signal's. The line is not routed like other
    /// lines, so this is where it fires. With the area unknown nothing fires
    /// and every signal it would have fired is refused and counted, as for
    /// any area-tied log signal.
    fn fire_level_up(&mut self, ev: &EventKind, name: &str, live: bool, out: &mut Vec<Transition>) {
        let area = self.area.clone();
        if area.is_none() {
            self.count_refusals(ev, Target::Named(name));
        }
        self.apply_to(Target::Named(name), ev, area.as_deref(), live, out);
    }

    /// Whether a level-1 Riverbank entry is still undecided: a character
    /// build line can still make it a relog. While it is, the entry has
    /// changed nothing (no note, the pin and the active character kept,
    /// [`Engine::new_character_open`] false) and the events since it wait.
    pub fn relog_window_open(&self) -> bool {
        self.relog.is_some()
    }

    /// Closes the relog window when `now` is more than [`RELOG_SECONDS`]
    /// after the entry, so a quiet log still decides: the entry then takes
    /// effect (a new character starts or resumes). At or under 5 s, or with
    /// no window, it changes nothing. `now` is log time (the log's local
    /// wall clock). Returns the transitions of the window's events.
    pub fn expire_relog_window(&mut self, now: Timestamp) -> Vec<Transition> {
        let mut out = Vec::new();
        self.close_expired_window(now, &mut out);
        out
    }

    fn close_expired_window(&mut self, now: Timestamp, out: &mut Vec<Transition>) {
        if self
            .relog
            .as_ref()
            .is_some_and(|w| now.seconds_since(&w.since) > RELOG_SECONDS)
        {
            self.close_relog_window(out);
        }
    }

    /// Closes the relog window without a relog: only now does the level-1
    /// entry to The Riverbank take effect. With no record, a new character
    /// starts, with no name. With a parked record, an entry to the same
    /// instance (seed) resumes it, and one to another instance replaces it
    /// (its events are counted as dropped new-character events). Either way
    /// the pin is released (the named character takes it), the active
    /// character stops being active, the events held before the entry are
    /// dropped and counted (another character's), and those of the window go
    /// to the new character, counted for refusals as routed events are. With
    /// no window it does nothing.
    fn close_relog_window(&mut self, out: &mut Vec<Transition>) {
        let Some(w) = self.relog.take() else {
            return;
        };
        let released_pin = self.pinned.take();
        let fresh = |released_pin: Option<String>| {
            let mut state = CharacterState::new("", Ownership::Own);
            state.level = 1;
            NewCharacter {
                state,
                events: 0,
                released_pin,
                seed: w.seed,
                open: true,
            }
        };
        match self.new_char.take() {
            Some(nc) if nc.open => {
                // One was loaded while the window was open: it keeps going.
                self.pinned = released_pin;
                self.new_char = Some(nc);
            }
            Some(mut parked) if parked.seed == w.seed => {
                parked.open = true;
                parked.released_pin = released_pin.clone();
                self.new_char = Some(parked);
                self.notes.push(CharacterNote::Resumed { released_pin });
            }
            Some(parked) => {
                self.new_char = Some(fresh(released_pin.clone()));
                self.notes.push(CharacterNote::Started { released_pin });
                self.dropped_new += parked.events;
                self.notes.push(CharacterNote::Replaced {
                    events: parked.events,
                });
            }
            None => {
                self.new_char = Some(fresh(released_pin.clone()));
                self.notes.push(CharacterNote::Started { released_pin });
            }
        }
        self.active = None;
        self.pending = None;
        // A new character's first area is The Riverbank: what was held
        // before the entry is another character's, never this one's.
        self.dropped += self.held.len() as u64;
        self.held.clear();
        for h in w.taken {
            self.receive(Target::New, h, out);
        }
    }

    /// A character build line. Within [`RELOG_SECONDS`] of an undecided
    /// level-1 Riverbank entry, before any other area entry, naming a stored
    /// character, it is a relog: the entry closes having changed nothing,
    /// the only note is [`CharacterNote::Relogged`], `name` becomes active
    /// and records the entry's seed, the pin stays, and the events held
    /// before the entry and those of the
    /// window go where routing sends them now (the pin, else `name`),
    /// counted for refusals as routed events are. Any other one changes
    /// nothing: an unknown name leaves the window open.
    fn character_named(&mut self, name: &str, time: Timestamp, out: &mut Vec<Transition>) {
        let in_window = self
            .relog
            .as_ref()
            .is_some_and(|w| (0..=RELOG_SECONDS).contains(&time.seconds_since(&w.since)));
        if !in_window || !self.chars.contains_key(name) {
            return;
        }
        let w = self.relog.take().expect("checked");
        self.relogged(name, w.seed, w.taken, out);
    }

    /// The one stored character whose Riverbank seed is `seed`, when the
    /// entry is not to a parked new character's instance (that record keeps
    /// its own resume rule, through the window) and no other stored
    /// character has the seed (two fall back to the window).
    fn seed_owner(&self, seed: u64) -> Option<String> {
        if self.new_char.as_ref().is_some_and(|nc| nc.seed == seed) {
            return None;
        }
        let mut owners = self
            .chars
            .values()
            .filter(|c| c.riverbank_seed == Some(seed));
        match (owners.next(), owners.next()) {
            (Some(c), None) => Some(c.name.clone()),
            _ => None,
        }
    }

    /// Decides a level-1 Riverbank entry as `name`'s relog: `name` becomes
    /// active and records the entry's seed, the pin stays, the only note is
    /// [`CharacterNote::Relogged`], and the events held before the entry and
    /// `taken` since it go where routing sends them now (the pin, else
    /// `name`), counted for refusals as routed events are.
    fn relogged(&mut self, name: &str, seed: u64, taken: Vec<Held>, out: &mut Vec<Transition>) {
        if let Some(ch) = self.chars.get_mut(name) {
            ch.riverbank_seed = Some(seed);
        }
        self.active = Some(name.to_string());
        self.pending = None;
        self.notes.push(CharacterNote::Relogged {
            name: name.to_string(),
        });
        let target = self.pinned.clone().unwrap_or_else(|| name.to_string());
        let mut released = std::mem::take(&mut self.held);
        released.extend(taken);
        self.release_to(&target, &released, out);
    }

    /// Ends the open new character's state, which clears its record.
    ///
    /// A level-up naming it, or the user pinning a character, gives its
    /// progress and its Riverbank seed to `name`: on a level-up the name
    /// becomes own and active, and pinned if the start released a pin. A
    /// name the engine did not know becomes the new character, so only what
    /// changes from what it showed is a transition; a known one gains its
    /// evidence, and the transitions are its own. (A relog never opens it:
    /// the entry's seed or [`Engine::character_named`] decides the entry
    /// before it does.)
    fn close_new_character(
        &mut self,
        name: &str,
        how: Close,
        live: bool,
        out: &mut Vec<Transition>,
    ) {
        let Some(nc) = self.new_char.take_if(|nc| nc.open) else {
            return;
        };
        let before = match self.chars.get(name) {
            Some(ch) => self.compute(ch),
            None => self.compute(&nc.state),
        };
        match self.chars.get_mut(name) {
            Some(ch) => {
                for (quest, ids) in nc.state.evidence {
                    ch.evidence.entry(quest).or_default().extend(ids);
                }
                for (quest, ov) in nc.state.overrides {
                    ch.overrides.entry(quest).or_insert(ov);
                }
            }
            None => {
                let mut state = nc.state;
                state.name = name.to_string();
                self.chars.insert(name.to_string(), state);
            }
        }
        let ch = self.chars.get_mut(name).expect("inserted");
        // The character it closes into is in its instance now.
        ch.riverbank_seed = Some(nc.seed);
        if let Close::LevelUp(class, level) = how {
            ch.ownership = Ownership::Own;
            ch.class = class.to_string();
            ch.level = level;
            self.active = Some(name.to_string());
            self.pending = None;
            if nc.released_pin.is_some() {
                self.pinned = Some(name.to_string());
            }
            self.notes.push(CharacterNote::Named {
                name: name.to_string(),
            });
        } else {
            self.notes.push(CharacterNote::Pinned {
                name: name.to_string(),
            });
        }
        let after = self.quest_states(name);
        push_transitions(name, &before, &after, live, out);
    }

    fn set_level(
        &mut self,
        name: &str,
        class: &str,
        level: u32,
        live: bool,
        out: &mut Vec<Transition>,
    ) {
        self.change(name, live, out, |e| match e.chars.get_mut(name) {
            Some(ch) if ch.level != level || ch.class != class => {
                ch.level = level;
                ch.class = class.to_string();
                true
            }
            _ => false,
        });
    }

    /// Makes an own character active and gives it the held events.
    fn activate(&mut self, name: &str, out: &mut Vec<Transition>) {
        if self.active.as_deref() == Some(name) {
            return;
        }
        self.pending = None;
        self.active = Some(name.to_string());
        let released = std::mem::take(&mut self.held);
        self.release_to(name, &released, out);
    }

    /// Rule 1: a `has received` or `has lost` line names an own character.
    /// Only a player's own rewards are logged, so the line also names the
    /// character being played: it becomes active and gets the event. While
    /// pinned, while a new character from The Riverbank waits for the
    /// level-up that names it, or while a Riverbank entry is undecided (the
    /// active character is kept then), the event only goes where every event
    /// goes.
    fn received(&mut self, name: &str, ev: &Event, live: bool, out: &mut Vec<Transition>) {
        if self.pinned.is_some() || self.new_open() || self.relog.is_some() {
            self.route(ev, live, out);
            return;
        }
        self.make_own(name, live, out);
        self.activate(name, out);
        self.route(ev, live, out);
    }

    fn make_own(&mut self, name: &str, live: bool, out: &mut Vec<Transition>) {
        if self.pending.as_ref().is_some_and(|p| p.name == name) {
            self.pending = None;
        }
        self.change(name, live, out, |e| match e.chars.get_mut(name) {
            Some(ch) if ch.ownership == Ownership::Own => false,
            Some(ch) => {
                ch.ownership = Ownership::Own;
                true
            }
            None => {
                e.chars
                    .insert(name.to_string(), CharacterState::new(name, Ownership::Own));
                true
            }
        });
    }

    /// Rule 2, and the second half of rule 4.
    fn party(&mut self, name: &str, out: &mut Vec<Transition>) {
        self.excluded.insert(name.to_string());
        if self.pinned.is_some() || self.active.as_deref() != Some(name) {
            return;
        }
        if self.chars.get(name).map(|c| c.ownership) == Some(Ownership::Own) {
            return;
        }
        self.active = None;
        if let Some(p) = self.pending.take() {
            if p.name == name {
                match p.before {
                    Some(before) => {
                        self.change(name, false, out, |e| {
                            e.chars.insert(name.to_string(), before);
                            true
                        });
                    }
                    None => {
                        self.chars.remove(name);
                    }
                }
                let mut back = p.released;
                back.extend(p.since);
                back.append(&mut self.held);
                self.held = back;
            }
        }
    }

    // -- user actions -------------------------------------------------------

    /// Pins a known character (it gets every event until unpinned) or, with
    /// `None`, unpins. Held events, and the progress of an open new
    /// character from The Riverbank not yet named, go to the newly pinned
    /// character; a parked one stays parked. Pinning while a Riverbank entry
    /// is undecided decides it for the pinned character: no new character
    /// starts, no note is posted, and the window's events go to it. Unpinning
    /// while a new character is open also cancels the pin its start released,
    /// so the character it names is not pinned.
    pub fn pin(&mut self, name: Option<&str>) -> Result<Vec<Transition>, EngineError> {
        let mut out = Vec::new();
        match name {
            None => {
                self.pinned = None;
                if let Some(nc) = self.new_char.as_mut() {
                    nc.released_pin = None;
                }
            }
            Some(n) => {
                if !self.chars.contains_key(n) {
                    return Err(EngineError(format!("no character named {n:?}")));
                }
                self.close_new_character(n, Close::Pin, true, &mut out);
                self.pinned = Some(n.to_string());
                self.pending = None;
                let mut released = std::mem::take(&mut self.held);
                if let Some(w) = self.relog.take() {
                    released.extend(w.taken);
                }
                self.release_to(n, &released, &mut out);
            }
        }
        Ok(out)
    }

    /// The user confirms an unconfirmed character as their own.
    pub fn confirm(&mut self, name: &str) -> Result<Vec<Transition>, EngineError> {
        if !self.chars.contains_key(name) {
            return Err(EngineError(format!("no character named {name:?}")));
        }
        let mut out = Vec::new();
        self.make_own(name, true, &mut out);
        Ok(out)
    }

    /// The name the user's quest actions take for the character the panel
    /// shows: the current character, else the empty name of the open new
    /// character from The Riverbank; `None` with neither.
    pub fn quest_target(&self) -> Option<&str> {
        self.current().or_else(|| self.new_open().then_some(""))
    }

    /// Applies a user action to one quest of one character: a stored one by
    /// its name, or, with the empty name ([`Engine::quest_target`]), the open
    /// new character while no named character is current. The new
    /// character's overrides are kept in its record and pass to the
    /// character its level-up names or a pin hands its progress to.
    pub fn quest_action(
        &mut self,
        name: &str,
        quest_id: &str,
        action: QuestAction,
    ) -> Result<Vec<Transition>, EngineError> {
        let (target, _) = self.check_target(name, quest_id)?;
        let mut out = Vec::new();
        self.change_to(target, true, &mut out, |e| {
            let ch = e.state_mut(target).expect("checked");
            let old = ch.overrides.get(quest_id).cloned().unwrap_or_default();
            let mut new = old.clone();
            match action {
                QuestAction::MarkDone => new.done = Some(true),
                QuestAction::UnmarkDone => new.done = Some(false),
                QuestAction::Skip => new.skipped = true,
                QuestAction::Unskip => new.skipped = false,
                QuestAction::Reset => new = Overrides::default(),
            }
            store_overrides(ch, quest_id, new.clone());
            new != old
        });
        Ok(out)
    }

    /// Ticks or unticks one step of one quest of one character, named as for
    /// [`Engine::quest_action`].
    pub fn tick_step(
        &mut self,
        name: &str,
        quest_id: &str,
        step_id: &str,
        ticked: bool,
    ) -> Result<Vec<Transition>, EngineError> {
        let (target, qi) = self.check_target(name, quest_id)?;
        if !self.data.quests[qi].steps.iter().any(|s| s.id == step_id) {
            return Err(EngineError(format!(
                "quest {quest_id:?} has no step {step_id:?}"
            )));
        }
        let mut out = Vec::new();
        self.change_to(target, true, &mut out, |e| {
            let ch = e.state_mut(target).expect("checked");
            let mut ov = ch.overrides.get(quest_id).cloned().unwrap_or_default();
            let changed = if ticked {
                ov.ticks.insert(step_id.to_string())
            } else {
                ov.ticks.remove(step_id)
            };
            store_overrides(ch, quest_id, ov);
            changed
        });
        Ok(out)
    }

    /// Who a user action on `name` goes to, and the quest's index.
    fn check_target<'n>(
        &self,
        name: &'n str,
        quest_id: &str,
    ) -> Result<(Target<'n>, usize), EngineError> {
        let target = if name.is_empty() {
            if self.current().is_some() || !self.new_open() {
                return Err(EngineError(
                    "no character to act on: none is current and no new character is open"
                        .to_string(),
                ));
            }
            Target::New
        } else if self.chars.contains_key(name) {
            Target::Named(name)
        } else {
            return Err(EngineError(format!("no character named {name:?}")));
        };
        let qi = self
            .index
            .get(quest_id)
            .copied()
            .ok_or_else(|| EngineError(format!("no quest {quest_id:?}")))?;
        Ok((target, qi))
    }

    // -- queries ------------------------------------------------------------

    /// Every stored character, by name.
    pub fn characters(&self) -> Vec<CharacterInfo> {
        self.chars
            .values()
            .map(|c| CharacterInfo {
                name: c.name.clone(),
                class: c.class.clone(),
                level: c.level,
                own: c.ownership == Ownership::Own,
                pinned: self.pinned.as_deref() == Some(c.name.as_str()),
                active: self.active.as_deref() == Some(c.name.as_str()),
            })
            .collect()
    }

    pub fn character(&self, name: &str) -> Option<&CharacterState> {
        self.chars.get(name)
    }

    /// The character events go to: the pinned one, else the active one.
    pub fn current(&self) -> Option<&str> {
        self.pinned.as_deref().or(self.active.as_deref())
    }

    pub fn active(&self) -> Option<&str> {
        self.active.as_deref()
    }

    pub fn pinned(&self) -> Option<&str> {
        self.pinned.as_deref()
    }

    /// Events waiting for a character in this session.
    pub fn held_count(&self) -> usize {
        self.held.len()
    }

    /// Events dropped because a session ended with no character (rule 5), or
    /// because a new character from The Riverbank started or resumed after
    /// them: those that came before any character.
    pub fn dropped_count(&self) -> u64 {
        self.dropped
    }

    /// Events of an unnamed new character that a level-1 entry to another
    /// instance of The Riverbank replaced, counted apart from
    /// [`Engine::dropped_count`].
    pub fn dropped_new_character_count(&self) -> u64 {
        self.dropped_new
    }

    /// Takes what happened to the new character since the last call, oldest
    /// first, for the shell to log each once.
    pub fn take_character_notes(&mut self) -> Vec<CharacterNote> {
        std::mem::take(&mut self.notes)
    }

    /// How many area-tied signals were refused because the area was unknown,
    /// over the engine's life (never reset by a session marker).
    pub fn refused_count(&self) -> u64 {
        self.refused
    }

    /// The quest id of each refusal not yet taken, oldest first, one entry
    /// per refused signal.
    pub fn refused_quests(&self) -> &[String] {
        &self.refused_quests
    }

    /// Takes the refusals' quest ids, for the shell to log each once; the
    /// count stays.
    pub fn take_refused(&mut self) -> Vec<String> {
        std::mem::take(&mut self.refused_quests)
    }

    pub fn client_version(&self) -> Option<&str> {
        self.client_version.as_deref()
    }

    /// Whether a level-1 entry to The Riverbank started (or resumed) a new
    /// character this session that no level-up has named yet. While it is
    /// open, events go to it (unless the user pins a character),
    /// [`Engine::current`] is `None`, and it is in neither
    /// [`Engine::characters`] nor [`Engine::to_json`]. A parked one is not
    /// open, and neither is one whose entry's relog window is still open
    /// ([`Engine::relog_window_open`]): the entry starts it only when the
    /// window closes without a relog.
    pub fn new_character_open(&self) -> bool {
        self.new_open()
    }

    fn new_open(&self) -> bool {
        self.new_char.as_ref().is_some_and(|nc| nc.open)
    }

    /// The unnamed new character's state (empty name, level 1), while open.
    pub fn new_character(&self) -> Option<&CharacterState> {
        self.new_char
            .as_ref()
            .filter(|nc| nc.open)
            .map(|nc| &nc.state)
    }

    /// Every quest's state for the unnamed new character, in data order,
    /// while it is open.
    pub fn new_character_states(&self) -> Option<Vec<QuestState>> {
        self.new_character().map(|ch| self.compute(ch))
    }

    /// The pin the new character's start released, while it is open: the
    /// character its level-up names is pinned in its place.
    pub fn new_character_released_pin(&self) -> Option<&str> {
        self.new_char
            .as_ref()
            .filter(|nc| nc.open)
            .and_then(|nc| nc.released_pin.as_deref())
    }

    /// Every quest's state for one character, in data order. A character the
    /// engine does not know has every quest locked.
    pub fn quest_states(&self, name: &str) -> Vec<QuestState> {
        self.states_of(Target::Named(name))
    }

    fn state(&self, target: Target) -> Option<&CharacterState> {
        match target {
            Target::Named(name) => self.chars.get(name),
            Target::New => self.new_character(),
        }
    }

    fn state_mut(&mut self, target: Target) -> Option<&mut CharacterState> {
        match target {
            Target::Named(name) => self.chars.get_mut(name),
            Target::New => self.new_char.as_mut().map(|nc| &mut nc.state),
        }
    }

    fn states_of(&self, target: Target) -> Vec<QuestState> {
        match self.state(target) {
            Some(ch) => self.compute(ch),
            None => self
                .data
                .quests
                .iter()
                .map(|q| QuestState {
                    quest_id: q.id.clone(),
                    status: Status::Locked,
                    tracked_status: Status::Locked,
                    current_step: None,
                    steps: q
                        .steps
                        .iter()
                        .map(|s| StepState {
                            id: s.id.clone(),
                            fired: false,
                            ticked: false,
                        })
                        .collect(),
                    done_override: None,
                    skipped: false,
                })
                .collect(),
        }
    }

    pub fn quest_state(&self, name: &str, quest_id: &str) -> Option<QuestState> {
        let qi = *self.index.get(quest_id)?;
        self.quest_states(name).into_iter().nth(qi)
    }

    pub fn status(&self, name: &str, quest_id: &str) -> Option<Status> {
        self.quest_state(name, quest_id).map(|s| s.status)
    }

    // -- persistence --------------------------------------------------------

    /// The characters, the pin and the active name: `{schema, pinned, active,
    /// characters}`. The unnamed new character is not in it (its record is
    /// [`Engine::new_character_json`]), nor is session memory (party names,
    /// held events, the area).
    pub fn to_json(&self) -> Value {
        json!({
            "schema": SCHEMA,
            "pinned": self.pinned,
            "active": self.active,
            "characters": self.chars.values().collect::<Vec<_>>(),
        })
    }

    /// Rebuilds an engine from [`Engine::to_json`]'s output.
    pub fn from_json(data: Arc<QuestData>, value: &Value) -> Result<Engine, EngineError> {
        if value.get("schema").and_then(Value::as_u64) != Some(SCHEMA) {
            return Err(EngineError(format!("engine state schema is not {SCHEMA}")));
        }
        let mut engine = Engine::new(data);
        if let Some(list) = value.get("characters") {
            let chars: Vec<CharacterState> = serde_json::from_value(list.clone())
                .map_err(|e| EngineError(format!("engine state characters: {e}")))?;
            for ch in chars {
                engine.chars.insert(ch.name.clone(), ch);
            }
        }
        let name = |key: &str| -> Option<String> {
            value
                .get(key)
                .and_then(Value::as_str)
                .filter(|n| engine.chars.contains_key(*n))
                .map(str::to_string)
        };
        let (pinned, active) = (name("pinned"), name("active"));
        engine.pinned = pinned;
        engine.active = active;
        Ok(engine)
    }

    /// The unnamed new character's record, open or parked, for
    /// `new-character.json`: `{schema, state, events, released_pin, seed,
    /// open}`. `None` when there is none (a level-up, a relog or a pin closed
    /// it, or none started), which is when the shell clears the file.
    pub fn new_character_json(&self) -> Option<Value> {
        self.new_char.as_ref().map(|nc| {
            let record = NewCharacterRecord {
                schema: SCHEMA,
                state: nc.state.clone(),
                events: nc.events,
                released_pin: nc.released_pin.clone(),
                seed: nc.seed,
                open: nc.open,
            };
            serde_json::to_value(record).expect("plain data serialises")
        })
    }

    /// Takes back [`Engine::new_character_json`]'s record, replacing any
    /// record the engine holds; load the characters first. An open record
    /// reopens: events go to it, nothing is active, and its released pin
    /// (else the pin the engine has now) waits for the character a level-up
    /// names. A released pin naming no stored character is dropped. A
    /// parked record waits for a level-1 entry to its instance.
    pub fn load_new_character(&mut self, value: &Value) -> Result<(), EngineError> {
        let record: NewCharacterRecord = serde_json::from_value(value.clone())
            .map_err(|e| EngineError(format!("new character record: {e}")))?;
        if record.schema != SCHEMA {
            return Err(EngineError(format!(
                "new character record schema is not {SCHEMA}"
            )));
        }
        let mut state = record.state;
        state.name.clear();
        let known = record.released_pin.filter(|p| self.chars.contains_key(p));
        let released_pin = if record.open {
            self.active = None;
            self.pending = None;
            let current = self.pinned.take();
            known.or(current)
        } else {
            known
        };
        self.new_char = Some(NewCharacter {
            state,
            events: record.events,
            released_pin,
            seed: record.seed,
            open: record.open,
        });
        Ok(())
    }

    /// One character's state, for `characters/<safe name>.json`.
    pub fn character_json(&self, name: &str) -> Option<Value> {
        self.chars
            .get(name)
            .map(|c| serde_json::to_value(c).expect("plain data serialises"))
    }

    /// Loads (or replaces) one character from [`Engine::character_json`]'s
    /// output.
    pub fn load_character(&mut self, value: &Value) -> Result<(), EngineError> {
        let ch: CharacterState = serde_json::from_value(value.clone())
            .map_err(|e| EngineError(format!("character state: {e}")))?;
        self.chars.insert(ch.name.clone(), ch);
        Ok(())
    }

    // -- status computation -------------------------------------------------

    /// Runs `f`, and when it reports a change, records every quest whose
    /// status or current step changed for `name`.
    fn change(
        &mut self,
        name: &str,
        live: bool,
        out: &mut Vec<Transition>,
        f: impl FnOnce(&mut Engine) -> bool,
    ) {
        self.change_to(Target::Named(name), live, out, f);
    }

    /// [`Engine::change`] for a stored character or the new one.
    fn change_to(
        &mut self,
        target: Target,
        live: bool,
        out: &mut Vec<Transition>,
        f: impl FnOnce(&mut Engine) -> bool,
    ) {
        let before = self.states_of(target);
        if !f(self) || self.state(target).is_none() {
            return;
        }
        let after = self.states_of(target);
        push_transitions(target.name(), &before, &after, live, out);
    }

    fn compute(&self, ch: &CharacterState) -> Vec<QuestState> {
        let n = self.data.quests.len();
        let mut memo = vec![None; n];
        let mut visiting = vec![false; n];
        (0..n)
            .map(|qi| {
                let status = self.status_of(ch, qi, &mut memo, &mut visiting);
                let q = &self.data.quests[qi];
                let ov = ch.overrides.get(&q.id);
                let tracked_status = match ov {
                    Some(o) if o.skipped => {
                        self.unskipped_status(ch, qi, o, &mut memo, &mut visiting)
                    }
                    _ => status,
                };
                let steps: Vec<StepState> = q
                    .steps
                    .iter()
                    .enumerate()
                    .map(|(k, s)| StepState {
                        id: s.id.clone(),
                        fired: self.role_fired(ch, qi, Role::Step(k), &mut memo, &mut visiting),
                        ticked: ov.is_some_and(|o| o.ticks.contains(&s.id)),
                    })
                    .collect();
                let current_step = steps
                    .iter()
                    .rev()
                    .find(|s| s.fired || s.ticked)
                    .map(|s| s.id.clone());
                QuestState {
                    quest_id: q.id.clone(),
                    status,
                    tracked_status,
                    current_step,
                    steps,
                    done_override: ov.and_then(|o| o.done),
                    skipped: ov.is_some_and(|o| o.skipped),
                }
            })
            .collect()
    }

    /// The status rules, first match wins.
    fn status_of(
        &self,
        ch: &CharacterState,
        qi: usize,
        memo: &mut Vec<Option<Status>>,
        visiting: &mut Vec<bool>,
    ) -> Status {
        if let Some(s) = memo[qi] {
            return s;
        }
        if visiting[qi] {
            // A cycle of `quest` signals: the quest being computed is taken
            // as locked for the purpose of the signal that refers back to it.
            return Status::Locked;
        }
        visiting[qi] = true;
        let q = &self.data.quests[qi];
        let ov = ch.overrides.get(&q.id).cloned().unwrap_or_default();
        let status = if ov.skipped {
            Status::Skipped
        } else {
            self.unskipped_status(ch, qi, &ov, memo, visiting)
        };
        visiting[qi] = false;
        memo[qi] = Some(status);
        status
    }

    /// The status rules after the skip: what a quest's evidence, level,
    /// other quests and its other overrides give. For a skipped quest this is
    /// its tracked status; the other quests' statuses it reads are their own
    /// (a skipped one counts as skipped there, as always).
    fn unskipped_status(
        &self,
        ch: &CharacterState,
        qi: usize,
        ov: &Overrides,
        memo: &mut Vec<Option<Status>>,
        visiting: &mut Vec<bool>,
    ) -> Status {
        let q = &self.data.quests[qi];
        if ov.done == Some(true)
            || (ov.done != Some(false) && self.role_fired(ch, qi, Role::Done, memo, visiting))
        {
            Status::Done
        } else if q.detection != Detection::Auto
            && self.role_fired(ch, qi, Role::Window, memo, visiting)
        {
            Status::Unknown
        } else if !ov.ticks.is_empty()
            || (0..q.steps.len()).any(|k| self.role_fired(ch, qi, Role::Step(k), memo, visiting))
            // A fired `start` is in progress with no step reached.
            || self.role_fired(ch, qi, Role::Start, memo, visiting)
        {
            Status::InProgress
        } else if q.unlock.is_empty() || self.role_fired(ch, qi, Role::Unlock, memo, visiting) {
            Status::Available
        } else {
            Status::Locked
        }
    }

    fn role_fired(
        &self,
        ch: &CharacterState,
        qi: usize,
        role: Role,
        memo: &mut Vec<Option<Status>>,
        visiting: &mut Vec<bool>,
    ) -> bool {
        let qid = &self.data.quests[qi].id;
        let evidence = ch.evidence.get(qid);
        self.compiled[qi]
            .iter()
            .filter(|c| c.role == role)
            .any(|c| match &c.signal {
                Signal::Level { min } => ch.level >= *min,
                Signal::Quest { id, status } => match self.index.get(id) {
                    Some(&other) => self.status_of(ch, other, memo, visiting) == *status,
                    None => false,
                },
                _ => evidence.is_some_and(|e| e.contains(&c.id)),
            })
    }
}

/// Records every quest whose status or current step differs between two
/// computations for `character`.
fn push_transitions(
    character: &str,
    before: &[QuestState],
    after: &[QuestState],
    live: bool,
    out: &mut Vec<Transition>,
) {
    for (b, a) in before.iter().zip(after) {
        if b.status != a.status || b.current_step != a.current_step {
            out.push(Transition {
                character: character.to_string(),
                quest_id: a.quest_id.clone(),
                from: b.status,
                to: a.status,
                step_from: b.current_step.clone(),
                step_to: a.current_step.clone(),
                live,
            });
        }
    }
}

fn store_overrides(ch: &mut CharacterState, quest_id: &str, ov: Overrides) {
    if ov.is_empty() {
        ch.overrides.remove(quest_id);
    } else {
        ch.overrides.insert(quest_id.to_string(), ov);
    }
}

/// The area an event-based signal is tied to, if any.
fn signal_area(signal: &Signal) -> Option<&str> {
    match signal {
        Signal::Npc { area, .. }
        | Signal::System { area, .. }
        | Signal::Reward { area, .. }
        | Signal::Points { area, .. } => area.as_deref(),
        Signal::Screen { area, .. } | Signal::LevelUp { area, .. } => Some(area),
        Signal::Area { .. } | Signal::Level { .. } | Signal::Quest { .. } => None,
    }
}

/// Whether an event-based signal fires on a log event that happened in
/// `area`. A `screen` signal never does: only a screen event fires it
/// ([`Engine::apply_screen`]).
fn fires_on(signal: &Signal, ev: &EventKind, area: Option<&str>) -> bool {
    let in_area = |want: &Option<String>| want.as_deref().is_none_or(|w| Some(w) == area);
    match (signal, ev) {
        (Signal::Area { code: Some(c), .. }, EventKind::AreaEntered { code, .. }) => c == code,
        (Signal::Area { name: Some(n), .. }, EventKind::AreaName { name }) => n.trim() == name,
        (
            Signal::Npc {
                speaker,
                area: a,
                line,
            },
            EventKind::NpcLine { speaker: s, text },
        ) => speaker == s && line.as_ref().is_none_or(|l| l == text) && in_area(a),
        (Signal::System { line, area: a }, EventKind::SystemLine { text }) => {
            line == text && in_area(a)
        }
        (
            Signal::Reward {
                text,
                category,
                area: a,
            },
            EventKind::Received { stat, .. },
        ) => {
            in_area(a)
                && (text.as_ref().is_some_and(|t| t == stat)
                    || category.is_some_and(|c| classify_stat(stat) == c))
        }
        (Signal::Points { pool, area: a }, EventKind::Points { pool: p, .. }) => {
            pool == p && in_area(a)
        }
        // Only [`Engine::fire_level_up`] passes a level-up here: the line is
        // not routed like the others.
        (Signal::LevelUp { level, area: a }, EventKind::LevelUp { level: l, .. }) => {
            level == l && Some(a.as_str()) == area
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quest_actions_parse_from_ipc_names() {
        for (s, a) in [
            ("mark_done", QuestAction::MarkDone),
            ("unmark_done", QuestAction::UnmarkDone),
            ("skip", QuestAction::Skip),
            ("unskip", QuestAction::Unskip),
            ("reset", QuestAction::Reset),
        ] {
            assert_eq!(s.parse::<QuestAction>().unwrap(), a);
        }
        assert!("explode".parse::<QuestAction>().is_err());
    }

    #[test]
    fn area_condition_limits_npc_signal() {
        let sig = Signal::Npc {
            speaker: "Renly".into(),
            area: Some("G1_town".into()),
            line: None,
        };
        let ev = EventKind::NpcLine {
            speaker: "Renly".into(),
            text: "anything".into(),
        };
        assert!(fires_on(&sig, &ev, Some("G1_town")));
        assert!(!fires_on(&sig, &ev, Some("G1_2")));
        assert!(!fires_on(&sig, &ev, None));
    }
}
