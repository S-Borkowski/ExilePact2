//! What the diagnostic log (`overlay.log`) may say, as functions of typed
//! values, so no caller ever writes a log line's text.
//!
//! The log is for a bug report: start and stop, the log path and mode, the
//! window handle, session starts, areas, character decisions, transitions,
//! toasts, the shortcut and errors. It never holds chat, an NPC line's text,
//! or another player's name: an event is described by its kind and the
//! fields that name a place or a quest, never by its line; party lines are
//! not described at all; and a character is named only when it is known to
//! be the player's own; one only taken to be by a level-up is "an
//! unconfirmed character".

use qt_engine::{Engine, Ownership, Transition};
use qt_logline::{Event, EventKind};
use qt_tailer::Reason;
use qt_viewmodel::Toast;

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

pub fn toast(t: &Toast) -> String {
    let kind = serde_json::to_value(t.kind)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default();
    format!("toast {} {} ({})", t.id, kind, t.quest_id)
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
