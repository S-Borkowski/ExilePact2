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
//! line's catch-up/live flag.
//!
//! What persists ([`Engine::to_json`]) is the characters, the pin and the
//! active name. Session memory (the names seen joining or leaving, the held
//! events, the current area) never does: other players' names are kept in
//! memory for one session only.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

use qt_logline::{parse_line, Allowlist, Event, EventKind};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::data::{classify_stat, Detection, QuestData, Signal, Status, SCHEMA};

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
    /// The highest step that fired or was ticked.
    pub current_step: Option<String>,
    pub steps: Vec<StepState>,
    pub done_override: Option<bool>,
    pub skipped: bool,
}

/// A change of one quest's status or current step for one character.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transition {
    pub character: String,
    pub quest_id: String,
    pub from: Status,
    pub to: Status,
    pub step_from: Option<String>,
    pub step_to: Option<String>,
    /// The line was written after the tailer started (or the change is the
    /// user's own action); only live transitions produce toasts.
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
    Step(usize),
    Done,
    Window,
}

/// A signal with its stable id (`unlock/0`, `step/<step id>/1`, `done/0`,
/// `window_closes/0`), which is what evidence stores.
#[derive(Debug, Clone)]
struct Compiled {
    id: String,
    role: Role,
    signal: Signal,
}

/// An event waiting for a character, with the area it happened in.
#[derive(Debug, Clone)]
struct Held {
    event: Event,
    area: Option<String>,
    live: bool,
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
    dropped: u64,
    client_version: Option<String>,
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
            dropped: 0,
            client_version: None,
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
        match &ev.kind {
            EventKind::SessionStart => self.end_session(),
            EventKind::ClientVersion { tag } => self.client_version = Some(tag.clone()),
            EventKind::AreaEntered { code, .. } => {
                self.area = Some(code.clone());
                self.route(ev, live, &mut out);
            }
            EventKind::AreaName { .. }
            | EventKind::NpcLine { .. }
            | EventKind::SystemLine { .. }
            | EventKind::Points { .. } => self.route(ev, live, &mut out),
            EventKind::LevelUp { name, class, level } => {
                self.level_up(name, class, *level, live, &mut out)
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
    /// events still held with no character to give them to are dropped and
    /// counted, and the session's memory is cleared.
    pub fn end_session(&mut self) {
        self.dropped += self.held.len() as u64;
        self.held.clear();
        self.pending = None;
        self.active = None;
        self.excluded.clear();
        self.area = None;
    }

    /// Gives an event to the pinned or active character, or holds it.
    fn route(&mut self, ev: &Event, live: bool, out: &mut Vec<Transition>) {
        let held = Held {
            event: ev.clone(),
            area: self.area.clone(),
            live,
        };
        let target = self.pinned.clone().or_else(|| self.active.clone());
        match target {
            Some(name) => {
                if let Some(p) = self.pending.as_mut() {
                    if p.name == name {
                        p.since.push(held.clone());
                    }
                }
                self.apply_to(&name, &held, out);
            }
            None => self.held.push(held),
        }
    }

    /// Records the event-based signals an event fires as `name`'s evidence.
    fn apply_to(&mut self, name: &str, held: &Held, out: &mut Vec<Transition>) {
        let mut hits: Vec<(usize, String)> = Vec::new();
        for (qi, signals) in self.compiled.iter().enumerate() {
            for c in signals {
                if fires_on(&c.signal, &held.event.kind, held.area.as_deref()) {
                    hits.push((qi, c.id.clone()));
                }
            }
        }
        if hits.is_empty() {
            return;
        }
        self.change(name, held.live, out, |e| {
            let Some(ch) = e.chars.get_mut(name) else {
                return false;
            };
            let mut changed = false;
            for (qi, id) in hits {
                let qid = e.data.quests[qi].id.clone();
                changed |= ch.evidence.entry(qid).or_default().insert(id);
            }
            changed
        });
    }

    fn level_up(
        &mut self,
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
            }
            return;
        }
        let known = self.chars.get(name).map(|c| c.ownership);
        if known == Some(Ownership::Own) {
            // Rule 3.
            self.set_level(name, class, level, live, out);
            self.activate(name, out);
        } else if self.excluded.contains(name) {
            // Rule 2: a party member this session.
        } else if self.active.as_deref() == Some(name) {
            self.set_level(name, class, level, live, out);
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
            for h in &released {
                self.apply_to(name, h, out);
            }
        }
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
        for h in &released {
            self.apply_to(name, h, out);
        }
    }

    /// Rule 1: a `has received` or `has lost` line names an own character.
    /// Only a player's own rewards are logged, so the line also names the
    /// character being played: it becomes active and gets the event.
    fn received(&mut self, name: &str, ev: &Event, live: bool, out: &mut Vec<Transition>) {
        if self.pinned.is_some() {
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
    /// `None`, unpins. Held events go to the newly pinned character.
    pub fn pin(&mut self, name: Option<&str>) -> Result<Vec<Transition>, EngineError> {
        let mut out = Vec::new();
        match name {
            None => self.pinned = None,
            Some(n) => {
                if !self.chars.contains_key(n) {
                    return Err(EngineError(format!("no character named {n:?}")));
                }
                self.pinned = Some(n.to_string());
                self.pending = None;
                let released = std::mem::take(&mut self.held);
                for h in &released {
                    self.apply_to(n, h, &mut out);
                }
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

    /// Applies a user action to one quest of one character.
    pub fn quest_action(
        &mut self,
        name: &str,
        quest_id: &str,
        action: QuestAction,
    ) -> Result<Vec<Transition>, EngineError> {
        self.check_target(name, quest_id)?;
        let mut out = Vec::new();
        self.change(name, true, &mut out, |e| {
            let ch = e.chars.get_mut(name).expect("checked");
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

    /// Ticks or unticks one step of one quest of one character.
    pub fn tick_step(
        &mut self,
        name: &str,
        quest_id: &str,
        step_id: &str,
        ticked: bool,
    ) -> Result<Vec<Transition>, EngineError> {
        let qi = self.check_target(name, quest_id)?;
        if !self.data.quests[qi].steps.iter().any(|s| s.id == step_id) {
            return Err(EngineError(format!(
                "quest {quest_id:?} has no step {step_id:?}"
            )));
        }
        let mut out = Vec::new();
        self.change(name, true, &mut out, |e| {
            let ch = e.chars.get_mut(name).expect("checked");
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

    fn check_target(&self, name: &str, quest_id: &str) -> Result<usize, EngineError> {
        if !self.chars.contains_key(name) {
            return Err(EngineError(format!("no character named {name:?}")));
        }
        self.index
            .get(quest_id)
            .copied()
            .ok_or_else(|| EngineError(format!("no quest {quest_id:?}")))
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

    /// Events dropped because a session ended with no character (rule 5).
    pub fn dropped_count(&self) -> u64 {
        self.dropped
    }

    pub fn client_version(&self) -> Option<&str> {
        self.client_version.as_deref()
    }

    /// Every quest's state for one character, in data order. A character the
    /// engine does not know has every quest locked.
    pub fn quest_states(&self, name: &str) -> Vec<QuestState> {
        match self.chars.get(name) {
            Some(ch) => self.compute(ch),
            None => self
                .data
                .quests
                .iter()
                .map(|q| QuestState {
                    quest_id: q.id.clone(),
                    status: Status::Locked,
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

    /// Everything that persists: `{schema, pinned, active, characters}`.
    /// Session memory (party names, held events, the area) is not in it.
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
        let before = self.quest_states(name);
        if !f(self) || !self.chars.contains_key(name) {
            return;
        }
        let after = self.quest_states(name);
        for (b, a) in before.iter().zip(&after) {
            if b.status != a.status || b.current_step != a.current_step {
                out.push(Transition {
                    character: name.to_string(),
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

    fn compute(&self, ch: &CharacterState) -> Vec<QuestState> {
        let n = self.data.quests.len();
        let mut memo = vec![None; n];
        let mut visiting = vec![false; n];
        (0..n)
            .map(|qi| {
                let status = self.status_of(ch, qi, &mut memo, &mut visiting);
                let q = &self.data.quests[qi];
                let ov = ch.overrides.get(&q.id);
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
        } else if ov.done == Some(true)
            || (ov.done != Some(false) && self.role_fired(ch, qi, Role::Done, memo, visiting))
        {
            Status::Done
        } else if q.detection != Detection::Auto
            && self.role_fired(ch, qi, Role::Window, memo, visiting)
        {
            Status::Unknown
        } else if !ov.ticks.is_empty()
            || (0..q.steps.len()).any(|k| self.role_fired(ch, qi, Role::Step(k), memo, visiting))
        {
            Status::InProgress
        } else if q.unlock.is_empty() || self.role_fired(ch, qi, Role::Unlock, memo, visiting) {
            Status::Available
        } else {
            Status::Locked
        };
        visiting[qi] = false;
        memo[qi] = Some(status);
        status
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

fn store_overrides(ch: &mut CharacterState, quest_id: &str, ov: Overrides) {
    if ov.is_empty() {
        ch.overrides.remove(quest_id);
    } else {
        ch.overrides.insert(quest_id.to_string(), ov);
    }
}

/// Whether an event-based signal fires on an event that happened in `area`.
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
