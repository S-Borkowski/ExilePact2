//! The overlay's state, through `App` with a recording fake shell: hidden
//! at the start and through every command, shown only while a craft is
//! active, with the placeholder line.

mod common;

use ch_app::OVERLAY_PLACEHOLDER;
use common::*;
use serde_json::json;

#[test]
fn overlay_hidden_until_a_craft_is_active() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::with_clipboard(ITEM_TEXT);
    let app = start(tmp.path(), &shell);

    // Baseline: the start and every command leave it hidden, and the shell
    // is never asked to show it.
    let view = app.get_view();
    assert!(!view.overlay.visible);
    assert_eq!(view.overlay.line, OVERLAY_PLACEHOLDER);
    app.hotkey_pressed();
    let _ = app.grab_text(Some(ITEM_TEXT));
    let _ = app.set_hotkey("Ctrl+Alt+R");
    let _ = app.update_settings(&json!({ "blocked_currencies": ["divine"] }));
    let _ = app.update_settings(&json!({ "check_updates": true }));
    let _ = app.check_for_updates();
    let _ = app.install_update();
    let _ = app.cancel_update();
    app.open_settings().unwrap();
    assert!(!app.get_view().overlay.visible);
    assert!(shell.rec().views.iter().all(|v| !v.overlay.visible));
    assert!(shell.rec().overlay.is_empty(), "{:?}", shell.rec().overlay);

    // Target: a craft shows it, through the shell and in the view.
    app.set_craft_active(true);
    assert!(app.get_view().overlay.visible);
    assert!(shell.last_view().overlay.visible);
    assert_eq!(shell.rec().overlay, vec![true]);
    // Again changes nothing.
    app.set_craft_active(true);
    assert_eq!(shell.rec().overlay, vec![true]);

    // The craft ends: hidden again.
    app.set_craft_active(false);
    assert!(!app.get_view().overlay.visible);
    assert!(!shell.last_view().overlay.visible);
    assert_eq!(shell.rec().overlay, vec![true, false]);
    app.shutdown();
    let log = diag_lines(tmp.path());
    assert!(
        log.contains(&"overlay shown: a craft is active".to_string()),
        "{log:?}"
    );
    assert!(
        log.contains(&"overlay hidden: no craft is active".to_string()),
        "{log:?}"
    );
}
