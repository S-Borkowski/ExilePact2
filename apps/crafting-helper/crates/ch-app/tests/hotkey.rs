//! The hotkey's rules, through `App` with a recording fake shell and a
//! temporary store: the default, the game's own Ctrl+Alt+C refused in any
//! spelling before the shell is asked, a saved Ctrl+Alt+C replaced at
//! launch, a taken key reported with the app still running, and an empty
//! key refused.

mod common;

use ch_app::{StatusKind, DEFAULT_HOTKEY};
use common::*;
use serde_json::json;

#[test]
fn hotkey_defaults_to_ctrl_alt_x() {
    assert_eq!(DEFAULT_HOTKEY, "Ctrl+Alt+X");
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);
    assert_eq!(shell.rec().register_calls, vec!["Ctrl+Alt+X".to_string()]);
    assert_eq!(shell.rec().registered.as_deref(), Some("Ctrl+Alt+X"));
    assert_eq!(app.get_view().hotkey, "Ctrl+Alt+X");
    assert_eq!(app.get_app_settings().hotkey, "Ctrl+Alt+X");
    assert_eq!(app.get_app_settings().hotkey_status, None);
    assert_eq!(app.get_view().status, None);

    // A new hotkey registers, is saved and is read back by the next run.
    let view = app.set_hotkey(" Ctrl+Shift+G ");
    assert_eq!(view.hotkey, "Ctrl+Shift+G");
    let status = view.hotkey_status.unwrap();
    assert_eq!(status.kind, StatusKind::Info);
    assert_eq!(status.text, "The hotkey is now Ctrl+Shift+G.");
    // Both windows heard it.
    assert_eq!(
        shell.rec().app_settings.last().unwrap().hotkey,
        "Ctrl+Shift+G"
    );
    assert_eq!(shell.last_view().hotkey, "Ctrl+Shift+G");
    app.shutdown();
    assert_eq!(saved_settings(tmp.path())["hotkey"], "Ctrl+Shift+G");
    let again = start(tmp.path(), &FakeShell::default());
    assert_eq!(again.get_view().hotkey, "Ctrl+Shift+G");
    again.shutdown();
    assert!(diag_lines(tmp.path()).contains(&"hotkey Ctrl+Alt+X registered".to_string()));
}

#[test]
fn hotkey_refuses_ctrl_alt_c() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);
    let emitted = shell.rec().app_settings.len();
    for spelling in [
        "Ctrl+Alt+C",
        "ctrl+alt+c",
        "Alt+Ctrl+C",
        "Control+Alt+C",
        "CommandOrControl+Alt+C",
        "CmdOrCtrl+Alt+C",
        "Ctrl+Alt+KeyC",
        " alt + CONTROL + keyc ",
    ] {
        let view = app.set_hotkey(spelling);
        assert_eq!(view.hotkey, "Ctrl+Alt+X", "{spelling}");
        let status = view.hotkey_status.unwrap();
        assert_eq!(status.kind, StatusKind::Error);
        assert!(
            status.text.contains("the game's own item copy"),
            "{}",
            status.text
        );
        assert!(
            status.text.ends_with("The hotkey is still Ctrl+Alt+X."),
            "{}",
            status.text
        );
    }
    // Refused before the shell was asked: only the start registered.
    assert_eq!(shell.rec().register_calls, vec!["Ctrl+Alt+X".to_string()]);
    assert_eq!(shell.rec().registered.as_deref(), Some("Ctrl+Alt+X"));
    // Each refusal was sent to the Settings window.
    assert_eq!(shell.rec().app_settings.len(), emitted + 8);

    // Negative control: a near miss is not refused, and goes to the shell.
    let view = app.set_hotkey("Ctrl+Alt+Shift+C");
    assert_eq!(view.hotkey, "Ctrl+Alt+Shift+C");
    assert_eq!(shell.rec().register_calls.len(), 2);

    // An empty key is refused too, and the current one stays.
    let view = app.set_hotkey("  ");
    assert_eq!(view.hotkey, "Ctrl+Alt+Shift+C");
    assert_eq!(view.hotkey_status.unwrap().kind, StatusKind::Error);
    assert_eq!(shell.rec().register_calls.len(), 2);
    app.shutdown();
    assert_eq!(saved_settings(tmp.path())["hotkey"], "Ctrl+Alt+Shift+C");
    assert!(diag_lines(tmp.path())
        .contains(&"hotkey Ctrl+Alt+C refused: it is the game's item copy".to_string()));
}

#[test]
fn a_saved_ctrl_alt_c_falls_back_to_the_default() {
    let tmp = tempfile::tempdir().unwrap();
    save_settings(tmp.path(), &json!({ "schema": 1, "hotkey": "Ctrl+Alt+C" }));
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);
    assert_eq!(shell.rec().register_calls, vec!["Ctrl+Alt+X".to_string()]);
    assert_eq!(app.get_view().hotkey, "Ctrl+Alt+X");
    let status = app.get_view().status.unwrap();
    assert!(
        status.text.contains("game's own item copy"),
        "{}",
        status.text
    );
    assert!(app.get_app_settings().hotkey_status.is_some());
    app.shutdown();
    // The default is what the file holds now.
    assert_eq!(saved_settings(tmp.path())["hotkey"], "Ctrl+Alt+X");
}

#[test]
fn a_taken_hotkey_is_reported_and_the_app_keeps_running() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::with_taken(&["Ctrl+Alt+X", "Ctrl+Alt+R"]);
    let app = start(tmp.path(), &shell);
    let view = app.get_view();
    assert_eq!(view.hotkey, "Ctrl+Alt+X");
    let status = view.status.unwrap();
    assert_eq!(status.kind, StatusKind::Error);
    assert_eq!(
        status.text,
        "Hotkey Ctrl+Alt+X is taken by another program. Choose another one in Settings."
    );
    // The app still grabs from a paste.
    assert!(app.grab_text(Some(ITEM_TEXT)).grab.is_some());

    // Another taken key: refused, the status says so.
    let settings = app.set_hotkey("Ctrl+Alt+R");
    assert_eq!(settings.hotkey, "Ctrl+Alt+X");
    assert_eq!(
        settings.hotkey_status.unwrap().text,
        "Hotkey Ctrl+Alt+R is taken by another program. The hotkey is still Ctrl+Alt+X."
    );
    // A free one registers, and the main window's hotkey warning goes.
    app.grab_text(None);
    let _ = app.set_hotkey("Ctrl+Alt+G");
    assert_eq!(shell.rec().registered.as_deref(), Some("Ctrl+Alt+G"));
    // The grab's own status is not the hotkey's, and stays.
    assert_eq!(
        app.get_view().status.unwrap().text,
        "Not grabbed: the paste holds no text."
    );
    app.shutdown();
}

#[test]
fn a_registered_hotkey_clears_the_hotkey_warning() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::with_taken(&["Ctrl+Alt+X"]);
    let app = start(tmp.path(), &shell);
    assert!(app.get_view().status.is_some());
    let _ = app.set_hotkey("Ctrl+Alt+G");
    assert_eq!(app.get_view().status, None);
    assert_eq!(shell.last_view().status, None);
    assert_eq!(shell.last_view().hotkey, "Ctrl+Alt+G");
    app.shutdown();
}
