//! The `theme` setting, through `App` with a recording fake shell and a
//! temporary store: a missing, `null`, non-string or unknown saved value
//! loads Exile and every other setting loads as before; a patch takes
//! exactly `"exile"` or `"claude"`, alone or beside today's keys, and
//! refuses anything else whole, naming `theme`; the choice is saved and
//! read back by a new app on the same store; and the main, overlay and
//! Settings views all carry it.

mod common;

use ch_app::{OverlayPosition, Theme};
use common::*;
use serde_json::{json, Value};

#[test]
fn a_missing_or_unknown_saved_theme_loads_exile_and_keeps_the_rest() {
    for theme in [
        None,
        Some(Value::Null),
        Some(json!(7)),
        Some(json!("unknown")),
        Some(json!("Claude")),
    ] {
        let tmp = tempfile::tempdir().unwrap();
        let mut saved = json!({
            "schema": 1,
            "hotkey": "Ctrl+Alt+X",
            "check_updates": true,
            "overlay_position": "bottom-left",
            "screen_watch": false,
        });
        if let Some(t) = &theme {
            saved["theme"] = t.clone();
        }
        save_settings(tmp.path(), &saved);
        let app = start(tmp.path(), &FakeShell::default());
        let settings = app.get_app_settings();
        assert_eq!(settings.theme, Theme::Exile, "{theme:?}");
        assert_eq!(app.get_view().theme, Theme::Exile, "{theme:?}");
        // Every other setting loads as before.
        assert_eq!(settings.hotkey, "Ctrl+Alt+X", "{theme:?}");
        assert!(settings.updates.check_updates, "{theme:?}");
        assert_eq!(
            settings.overlay_position,
            OverlayPosition::BottomLeft,
            "{theme:?}"
        );
        assert!(!settings.screen_watch, "{theme:?}");
        app.shutdown();
    }

    // Positive control: a saved `"claude"` is Claude.
    let tmp = tempfile::tempdir().unwrap();
    save_settings(tmp.path(), &json!({ "schema": 1, "theme": "claude" }));
    let app = start(tmp.path(), &FakeShell::default());
    assert_eq!(app.get_app_settings().theme, Theme::Claude);
    assert_eq!(app.get_view().theme, Theme::Claude);
    app.shutdown();
}

#[test]
fn a_first_launch_is_exile() {
    let tmp = tempfile::tempdir().unwrap();
    let app = start(tmp.path(), &FakeShell::default());
    assert_eq!(app.get_app_settings().theme, Theme::Exile);
    assert_eq!(app.get_view().theme, Theme::Exile);
    let value = serde_json::to_value(app.get_view()).unwrap();
    assert_eq!(value["theme"], json!("exile"));
    app.shutdown();
}

#[test]
fn theme_patches_are_strict_and_atomic() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);

    let view = app.update_settings(&json!({ "theme": "claude" })).unwrap();
    assert_eq!(view.theme, Theme::Claude);

    // Refused whole, nothing changed, the reason names `theme`.
    for bad in [
        json!("unknown"),
        Value::Null,
        json!(true),
        json!(7),
        json!("Exile"),
        json!(" exile"),
    ] {
        let err = app
            .update_settings(&json!({ "theme": bad, "screen_watch": false }))
            .unwrap_err();
        assert!(err.contains("theme"), "{bad}: {err}");
        let now = app.get_app_settings();
        assert_eq!(now.theme, Theme::Claude, "{bad}");
        assert!(now.screen_watch, "{bad}: the good key beside it changed");
    }
    // A key the patch did not accept before still refuses the whole patch.
    assert!(app
        .update_settings(&json!({ "theme": "exile", "hotkey": "Ctrl+Alt+R" }))
        .is_err());
    assert_eq!(app.get_app_settings().theme, Theme::Claude);

    // Beside today's keys, both apply.
    let view = app
        .update_settings(&json!({ "theme": "exile", "overlay_position": "top-right" }))
        .unwrap();
    assert_eq!(view.theme, Theme::Exile);
    assert_eq!(view.overlay_position, OverlayPosition::TopRight);
    app.shutdown();
}

#[test]
fn a_theme_change_reaches_every_window_and_checks_nothing() {
    let h = harness(Setup::default());
    h.app.start();
    let shell = &h.shell;
    let views_before = shell.rec().views.len();
    let settings_before = shell.rec().app_settings.len();
    h.app
        .update_settings(&json!({ "theme": "claude" }))
        .unwrap();
    {
        let rec = shell.rec();
        // The main window and the overlay hear `ch:view` (one emit, both
        // windows); the Settings window hears `ch:app-settings`.
        assert_eq!(rec.views.len(), views_before + 1);
        assert_eq!(rec.views.last().unwrap().theme, Theme::Claude);
        assert_eq!(rec.app_settings.len(), settings_before + 1);
        assert_eq!(rec.app_settings.last().unwrap().theme, Theme::Claude);
    }
    // No update check started (the check setting is off; nothing asked).
    assert_eq!(h.calls(), 0);
    h.app.shutdown();
}

#[test]
fn the_theme_persists_across_a_restart() {
    let tmp = tempfile::tempdir().unwrap();
    let app = start(tmp.path(), &FakeShell::default());
    app.update_settings(&json!({ "theme": "claude" })).unwrap();
    app.shutdown();
    let saved = saved_settings(tmp.path());
    assert_eq!(saved["theme"], json!("claude"));
    assert_eq!(saved["schema"], json!(1));

    let again = start(tmp.path(), &FakeShell::default());
    assert_eq!(again.get_app_settings().theme, Theme::Claude);
    assert_eq!(again.get_view().theme, Theme::Claude);
    again.update_settings(&json!({ "theme": "exile" })).unwrap();
    again.shutdown();
    assert_eq!(saved_settings(tmp.path())["theme"], json!("exile"));

    let third = start(tmp.path(), &FakeShell::default());
    assert_eq!(third.get_app_settings().theme, Theme::Exile);
    third.shutdown();
}
