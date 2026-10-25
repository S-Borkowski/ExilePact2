//! What the diagnostic log (`overlay.log`) may say, as functions of typed
//! values, so no caller ever writes a log line's text.
//!
//! The log is for a bug report: start and stop, the log path and mode, the
//! window handle, session starts, areas, character decisions, transitions,
//! row changes, the shortcut and errors. It never holds chat, an NPC line's text,
//! or another player's name: an event is described by its kind and the
//! fields that name a place or a quest, never by its line; party lines are
//! not described at all; and a character is named only when it is known to
//! be the player's own; one only taken to be by a level-up is "an
//! unconfirmed character".

use qt_engine::{CharacterNote, Engine, LogArea, Ownership, Transition};
use qt_logline::{Event, EventKind};
use qt_tailer::Reason;
use qt_viewmodel::RowChange;

use crate::seed::MAX_SEED_BYTES;

/// How a character appears in the log: its name when it is own, else a
/// placeholder (a level-up alone does not prove the name is the player's).
pub fn character(engine: &Engine, name: Option<&str>) -> String {
    match name {
        None => "none".to_string(),
        Some(n) => match engine.character(n).map(|c| c.ownership) {
            Some(Ownership::Own) => n.to_string(),
            _ => "an unconfirmed character".to_string(),
        },
    }
}

/// A live event worth a line, or `None`. Only the session marker, area
/// changes and the client version are described; NPC and system lines,
/// rewards, level-ups, deaths and party lines are not (their effect shows as
/// a transition or a character decision instead).
pub fn event(ev: &Event) -> Option<String> {
    match &ev.kind {
        EventKind::SessionStart => Some("session start".to_string()),
        EventKind::ClientVersion { tag } => Some(format!("client version {tag}")),
        EventKind::AreaEntered { code, level, .. } => {
            Some(format!("area {code} (level {level})"))
        }
        EventKind::AreaName { name } => Some(format!("area name {name}")),
        _ => None,
    }
}

/// The current character changed.
pub fn decision(engine: &Engine, from: Option<&str>, to: Option<&str>) -> String {
    format!(
        "character: {} -> {}",
        character(engine, from),
        character(engine, to)
    )
}

pub fn transition(t: &Transition) -> String {
    let step = match (&t.step_from, &t.step_to) {
        (a, b) if a == b => String::new(),
        (a, b) => format!(
            ", step {} -> {}",
            a.as_deref().unwrap_or("none"),
            b.as_deref().unwrap_or("none")
        ),
    };
    format!(
        "quest {}: {} -> {}{step} ({})",
        t.quest_id,
        t.from.as_str(),
        t.to.as_str(),
        if t.live { "live" } else { "catch-up" }
    )
}

/// A live row change sent as `qt:row-change`: its id, kind and quest, never
/// its step titles or text.
pub fn row_change(c: &RowChange) -> String {
    format!("row change {} {} ({})", c.id, c.kind.as_str(), c.quest_id)
}

/// A level-1 entry to The Riverbank started a new, still unnamed character.
pub const NEW_CHARACTER: &str = "character: a new character (The Riverbank)";

/// A level-1 entry to the instance of The Riverbank a parked new character
/// was in resumed it.
pub const NEW_CHARACTER_RESUMED: &str = "character: the new character resumed (The Riverbank)";

/// The new character's start released a pin; the character its level-up
/// names is pinned in its place.
pub fn pin_released(engine: &Engine, pinned: &str) -> String {
    format!(
        "character: pin on {} released for the new character",
        character(engine, Some(pinned))
    )
}

/// What happened to the new character from The Riverbank, one line each,
/// for catch-up lines as well as live ones, so a pin given up or kept
/// during catch-up is explained.
pub fn character_note(engine: &Engine, note: &CharacterNote) -> Vec<String> {
    let opened = |line: &str, released_pin: &Option<String>| {
        let mut lines = vec![line.to_string()];
        if let Some(pin) = released_pin {
            lines.push(pin_released(engine, pin));
        }
        lines
    };
    match note {
        CharacterNote::Started { released_pin } => opened(NEW_CHARACTER, released_pin),
        CharacterNote::Resumed { released_pin } => opened(NEW_CHARACTER_RESUMED, released_pin),
        CharacterNote::Replaced { events } => vec![format!(
            "character: a parked new character replaced by another instance of The Riverbank, {events} events dropped"
        )],
        CharacterNote::Parked { restored_pin } => vec![match restored_pin {
            Some(pin) => format!(
                "character: the new character parked at a session start, pin on {} restored",
                character(engine, Some(pin))
            ),
            None => "character: the new character parked at a session start".to_string(),
        }],
        CharacterNote::Relogged { name } => vec![format!(
            "character: {} relogged in The Riverbank, kept",
            character(engine, Some(name))
        )],
        CharacterNote::Named { name } => vec![format!(
            "character: the new character named {} by a level-up",
            character(engine, Some(name))
        )],
        CharacterNote::Pinned { name } => vec![format!(
            "character: the new character's progress given to {}, pinned",
            character(engine, Some(name))
        )],
    }
}

/// The summary of a batch of catch-up lines: how many, the character they
/// left current and the area they left (the one the engine now holds,
/// which the catch-up may have changed without a live area line).
pub fn catch_up(engine: &Engine, lines: usize) -> String {
    let current = engine.current();
    let who = if current.is_none() && engine.new_character_open() {
        "a new character (The Riverbank)".to_string()
    } else {
        character(engine, current)
    };
    format!(
        "catch-up: {lines} lines read, character {who}, area {}",
        engine.current_area().unwrap_or("unknown")
    )
}

/// The area seeded from the log before a tailer's start position: its code,
/// or `none` with why. Never a line of the log.
pub fn seeded_area(found: &LogArea, offset: u64) -> String {
    match found {
        LogArea::Area(code) => format!("area from log: {code}"),
        LogArea::NoAreaLine => format!(
            "area from log: none (no area line in the {} bytes before offset {offset})",
            offset.min(MAX_SEED_BYTES)
        ),
        LogArea::MarkerAfter => {
            "area from log: none (a session marker follows the last area line)".to_string()
        }
    }
}

/// A signal refused because the area was unknown.
pub fn refused(quest_id: &str) -> String {
    format!("area unknown: refused {quest_id}")
}

/// Why reading starts where it does, in words.
pub fn reason(r: Reason) -> &'static str {
    match r {
        Reason::AtEnd => "no saved position: starting at the end of the log",
        Reason::FromStart => "--from-start: reading the whole log as catch-up",
        Reason::Resumed => "resuming from the saved position",
        Reason::SavedMismatch => "the saved position no longer matches: reading the whole log",
        Reason::Appeared => "the log appeared: reading it from the start",
        Reason::Truncated => "the log was truncated: reading it from the start",
        Reason::Replaced => "the log was rewritten: reading it from the start",
        Reason::Rotated => "the log was rotated: reading the new file from the start",
    }
}
