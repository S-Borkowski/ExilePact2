//! The grab, through `App` with a recording fake shell and a temporary
//! store.
//!
//! Baseline: the start, every command and a paste read the clipboard zero
//! times. Target: each hotkey press reads it exactly once (the positive
//! control that the fake counts a read); a paste's text is grabbed with no
//! read; a refused grab keeps the last one, and only a refused hotkey grab
//! beeps; `diag.log` never holds the text.

mod common;

use ch_app::{GrabSource, StatusKind, MAX_GRAB_BYTES};
use common::*;
use serde_json::json;

#[test]
fn grab_reads_the_clipboard_once_per_press() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::with_clipboard(ITEM_TEXT);
    let app = start(tmp.path(), &shell);

    // Baseline: the start and every other command read nothing.
    let _ = app.get_view();
    let _ = app.get_app_settings();
    let _ = app.set_hotkey("Ctrl+Alt+R");
    let _ = app.update_settings(&json!({ "blocked_currencies": ["exalted-orb"] }));
    let _ = app.update_settings(&json!({ "check_updates": true }));
    app.open_settings().unwrap();
    let _ = app.check_for_updates();
    let _ = app.cancel_update();
    app.set_craft_active(true);
    app.set_craft_active(false);
    app.tick_updates(0);
    assert_eq!(shell.reads(), 0, "a clipboard read nobody asked for");
    assert_eq!(app.get_view().grab, None);

    // Target: one read per press, and the grab is the clipboard's text.
    app.hotkey_pressed();
    assert_eq!(shell.reads(), 1);
    let grab = app.get_view().grab.expect("a grab");
    assert_eq!(grab.text, ITEM_TEXT_NORMALISED);
    assert_eq!(grab.line_count, 10);
    assert_eq!(grab.source, GrabSource::Shortcut);
    assert!(grab.looks_like_item);
    let status = app.get_view().status.unwrap();
    assert_eq!(status.kind, StatusKind::Info);
    assert_eq!(status.text, "Grabbed an item: 10 lines.");
    // The window heard it.
    assert_eq!(shell.last_view().grab.as_ref(), Some(&grab));

    // A second press reads again, once, and replaces the grab.
    shell.set_clipboard(Some(NOT_ITEM_TEXT));
    app.hotkey_pressed();
    assert_eq!(shell.reads(), 2);
    let grab = app.get_view().grab.unwrap();
    assert_eq!(grab.text, "a shopping list\nmilk\neggs");
    assert_eq!(grab.line_count, 3);
    assert!(!grab.looks_like_item);
    // Not an item is shown, with an error status, and no beep.
    let status = app.get_view().status.unwrap();
    assert_eq!(status.kind, StatusKind::Error);
    assert!(
        status
            .text
            .contains("do not look like an item copied with Ctrl+Alt+C"),
        "{}",
        status.text
    );
    assert_eq!(shell.rec().beeps, 0);

    // Nothing reads afterwards on its own.
    let _ = app.get_view();
    app.tick_updates(48 * HOUR);
    assert_eq!(shell.reads(), 2);
    app.shutdown();

    // The log names each grab and its size, never the text.
    let log = diag_lines(tmp.path());
    assert!(
        log.contains(&"grab from the shortcut: 10 lines".to_string()),
        "{log:?}"
    );
    assert!(
        log.contains(&"grab from the shortcut: 3 lines".to_string()),
        "{log:?}"
    );
    let text = std::fs::read_to_string(tmp.path().join("diag.log")).unwrap();
    for word in ["Gale Grip", "Riveted Mitts", "shopping", "maximum Life"] {
        assert!(
            !text.contains(word),
            "diag.log holds the grab's text: {word}"
        );
    }
    // Nor is it saved.
    let saved = std::fs::read_to_string(tmp.path().join("settings.json")).unwrap();
    assert!(!saved.contains("Gale Grip"));
}

#[test]
fn grab_from_paste_reads_no_clipboard() {
    let tmp = tempfile::tempdir().unwrap();
    // The clipboard holds something else: the paste's text must win.
    let shell = FakeShell::with_clipboard(NOT_ITEM_TEXT);
    let app = start(tmp.path(), &shell);
    let view = app.grab_text(Some(ITEM_TEXT));
    assert_eq!(shell.reads(), 0, "a paste read the clipboard");
    let grab = view.grab.clone().expect("a grab");
    assert_eq!(grab.text, ITEM_TEXT_NORMALISED);
    assert_eq!(grab.source, GrabSource::Paste);
    assert!(grab.looks_like_item);
    assert_eq!(shell.last_view(), view);

    // The same checks: a refused paste keeps the grab and does not beep.
    let refused = app.grab_text(Some("   \r\n"));
    assert_eq!(refused.grab.as_ref(), Some(&grab));
    let status = refused.status.unwrap();
    assert_eq!(status.kind, StatusKind::Error);
    assert_eq!(status.text, "Not grabbed: the paste is empty.");
    let refused = app.grab_text(None);
    assert_eq!(
        refused.status.unwrap().text,
        "Not grabbed: the paste holds no text."
    );
    assert_eq!(shell.rec().beeps, 0);
    assert_eq!(shell.reads(), 0);
    app.shutdown();
    let log = diag_lines(tmp.path());
    assert!(
        log.contains(&"grab from a paste: 10 lines".to_string()),
        "{log:?}"
    );
    assert!(
        log.contains(&"grab refused: the paste is empty".to_string()),
        "{log:?}"
    );
}

#[test]
fn grab_refuses_an_empty_clipboard() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::with_clipboard(ITEM_TEXT);
    let app = start(tmp.path(), &shell);
    app.hotkey_pressed();
    let kept = app.get_view().grab.expect("a grab");

    // Empty, whitespace, no text at all, a busy clipboard and too long: each
    // refused, with the grab kept, an error status and one beep.
    let too_long = "x".repeat(MAX_GRAB_BYTES + 1);
    let cases: Vec<(Option<&str>, bool, String)> = vec![
        (Some(""), false, "the clipboard is empty".to_string()),
        (
            Some(" \t\r\n\n"),
            false,
            "the clipboard is empty".to_string(),
        ),
        (None, false, "the clipboard holds no text".to_string()),
        (
            Some(ITEM_TEXT),
            true,
            format!("the clipboard could not be read ({BUSY})"),
        ),
        (
            Some(&too_long),
            false,
            format!(
                "the clipboard holds {} bytes of text, over {MAX_GRAB_BYTES}",
                MAX_GRAB_BYTES + 1
            ),
        ),
    ];
    for (i, (text, busy, reason)) in cases.iter().enumerate() {
        shell.set_clipboard(*text);
        shell.rec().clipboard_busy = *busy;
        app.hotkey_pressed();
        let view = app.get_view();
        assert_eq!(view.grab.as_ref(), Some(&kept), "case {i}");
        let status = view.status.unwrap();
        assert_eq!(status.kind, StatusKind::Error);
        assert_eq!(status.text, format!("Not grabbed: {reason}."));
        assert_eq!(shell.rec().beeps, i + 1, "one beep per refusal");
        assert_eq!(shell.reads(), i + 2, "one read per press");
    }

    // The beep's fallback is logged too.
    shell.rec().clipboard_busy = false;
    shell.rec().tone_fails = true;
    shell.set_clipboard(Some(""));
    app.hotkey_pressed();
    app.shutdown();
    let log = diag_lines(tmp.path());
    assert!(
        log.contains(&"grab refused: the clipboard is empty".to_string()),
        "{log:?}"
    );
    assert!(
        log.contains(&"refusal beep: tone ok".to_string()),
        "{log:?}"
    );
    assert!(
        log.contains(&"refusal beep: tone failed (no sound device (os error 1))".to_string()),
        "{log:?}"
    );
    assert!(
        log.contains(&"refusal beep: simple beep ok".to_string()),
        "{log:?}"
    );
}

#[test]
fn a_grab_at_the_size_limit_is_taken() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::with_clipboard(&"x".repeat(MAX_GRAB_BYTES));
    let app = start(tmp.path(), &shell);
    app.hotkey_pressed();
    let grab = app.get_view().grab.expect("a grab at the limit");
    assert_eq!(grab.text.len(), MAX_GRAB_BYTES);
    assert_eq!(shell.rec().beeps, 0);
    app.shutdown();
}
