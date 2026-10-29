//! The grab: the text of an item the player copied with the game's own
//! Ctrl+Alt+C, taken once, on demand, from one of two routes.
//!
//! - **The hotkey**: the glue reads the clipboard's text once per press
//!   ([`crate::Shell::read_clipboard`]) and hands it to
//!   [`crate::App::hotkey_pressed`].
//! - **A paste**: the main window reads the text from its own paste event
//!   and sends it with `grab_text`; nothing reads the clipboard for it.
//!
//! Both routes are checked the same way ([`check`]): text that is missing,
//! empty or only whitespace, or longer than [`MAX_GRAB_BYTES`] UTF-8 bytes,
//! is refused. Text that passes is normalised (CRLF and CR become LF, and
//! trailing blank lines go), counted in lines, and flagged as looking like
//! an item when its first line starts with [`ITEM_FIRST_LINE`]. The item
//! text parser (`ch-item`) defines the real check later; the shell keeps
//! the raw text only.

use crate::view::{GrabSource, GrabView};

/// The longest text a grab takes, in UTF-8 bytes.
pub const MAX_GRAB_BYTES: usize = 65_536;
/// How the first line of the game's copied item text starts. Not measured
/// by this crate; `ch-item` owns the real check.
pub const ITEM_FIRST_LINE: &str = "Item Class: ";

/// Why a grab was refused, as the status line and `diag.log` say it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// No text was given, or the clipboard holds none.
    Missing,
    /// The text is empty or only whitespace.
    Empty,
    /// The text is longer than [`MAX_GRAB_BYTES`]; how long it is.
    TooLong(usize),
    /// The clipboard could not be read; the glue's reason.
    Unreadable(String),
}

impl Refusal {
    /// The reason, as one clause.
    pub fn reason(&self, source: GrabSource) -> String {
        let what = match source {
            GrabSource::Shortcut => "the clipboard",
            GrabSource::Paste => "the paste",
        };
        match self {
            Refusal::Missing => format!("{what} holds no text"),
            Refusal::Empty => format!("{what} is empty"),
            Refusal::TooLong(n) => {
                format!("{what} holds {n} bytes of text, over {MAX_GRAB_BYTES}")
            }
            Refusal::Unreadable(e) => format!("the clipboard could not be read ({e})"),
        }
    }
}

/// Checks and normalises one grab's text.
pub fn check(text: Option<&str>, source: GrabSource) -> Result<GrabView, Refusal> {
    let text = text.ok_or(Refusal::Missing)?;
    if text.len() > MAX_GRAB_BYTES {
        return Err(Refusal::TooLong(text.len()));
    }
    if text.trim().is_empty() {
        return Err(Refusal::Empty);
    }
    let text = normalise(text);
    let line_count = text.split('\n').count();
    let looks_like_item = text
        .split('\n')
        .next()
        .is_some_and(|first| first.starts_with(ITEM_FIRST_LINE));
    Ok(GrabView {
        text,
        line_count,
        source,
        looks_like_item,
    })
}

/// CRLF and CR become LF, and trailing blank lines (empty or whitespace
/// only) are dropped, the last line break with them.
pub fn normalise(text: &str) -> String {
    let unified = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut lines: Vec<&str> = unified.split('\n').collect();
    while lines.last().is_some_and(|l| l.trim().is_empty()) {
        lines.pop();
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_breaks_are_unified_and_trailing_blank_lines_dropped() {
        assert_eq!(normalise("a\r\nb\rc\n\n  \r\n"), "a\nb\nc");
        assert_eq!(normalise("a\n\nb\n"), "a\n\nb");
        let g = check(
            Some("Item Class: Gloves\r\nRarity: Rare\r\n\r\n"),
            GrabSource::Paste,
        )
        .unwrap();
        assert_eq!(g.text, "Item Class: Gloves\nRarity: Rare");
        assert_eq!(g.line_count, 2);
        assert!(g.looks_like_item);
        let g = check(Some("hello"), GrabSource::Shortcut).unwrap();
        assert_eq!(g.line_count, 1);
        assert!(!g.looks_like_item);
    }

    #[test]
    fn missing_empty_and_too_long_are_refused() {
        assert_eq!(check(None, GrabSource::Paste), Err(Refusal::Missing));
        assert_eq!(check(Some(""), GrabSource::Paste), Err(Refusal::Empty));
        assert_eq!(
            check(Some(" \r\n\t"), GrabSource::Paste),
            Err(Refusal::Empty)
        );
        let at = "x".repeat(MAX_GRAB_BYTES);
        assert!(check(Some(&at), GrabSource::Paste).is_ok());
        // Bytes, not characters: 32,769 two-byte characters are too long.
        let over = "é".repeat(MAX_GRAB_BYTES / 2 + 1);
        assert_eq!(
            check(Some(&over), GrabSource::Paste),
            Err(Refusal::TooLong(MAX_GRAB_BYTES + 2))
        );
    }
}
