//! The area the player was in when a tailer starts, read back from the log.
//!
//! The engine's current area is session memory set by a `Generating level`
//! line read in this run. A tailer that starts mid-zone (a restart of the
//! app, or a first start at the end of a log) never reads that line, so the
//! shell reads the lines before the tailer's start position and seeds the
//! engine with [`area_from_log`]'s answer ([`crate::Engine::seed_area`]).
//! A session marker after the last area line means the game restarted since,
//! so the area is unknown, exactly as [`crate::Engine::end_session`] treats
//! it when the marker is read live.

use qt_logline::{parse_line, Allowlist, EventKind};

/// What the log before a start position says about the current area.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogArea {
    /// The code of the last `Generating level` line, with no session marker
    /// after it.
    Area(String),
    /// No area line among the lines read.
    NoAreaLine,
    /// A session marker (`LOG FILE OPENING`) follows the last area line.
    MarkerAfter,
}

impl LogArea {
    /// The area code to seed, or `None` when the area is unknown.
    pub fn code(&self) -> Option<&str> {
        match self {
            LogArea::Area(code) => Some(code),
            LogArea::NoAreaLine | LogArea::MarkerAfter => None,
        }
    }
}

/// Reads `lines` (the log before a start position, oldest first) for the
/// last area line. Parsed with the same parser as every other line; a line
/// cut at the start of a read window is simply not recognised. Area lines
/// and session markers need no allowlist, so none is taken.
pub fn area_from_log<I>(lines: I) -> LogArea
where
    I: IntoIterator,
    I::Item: AsRef<str>,
{
    let allow = Allowlist::default();
    let mut found = LogArea::NoAreaLine;
    for line in lines {
        match parse_line(line.as_ref(), &allow).map(|ev| ev.kind) {
            Some(EventKind::AreaEntered { code, .. }) => found = LogArea::Area(code),
            Some(EventKind::SessionStart) if found != LogArea::NoAreaLine => {
                found = LogArea::MarkerAfter
            }
            _ => {}
        }
    }
    found
}
