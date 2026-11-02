//! The theme setting: `settings.json`'s `theme`, `"exile"` (the default) or
//! `"claude"` (the look before the steel/masonry restyle). Loading is
//! lenient, so a missing, `null`, non-string or unknown value is Exile and
//! every other setting still loads; a patch is strict and atomic, so a bad
//! theme or a key the Settings window does not send refuses the whole patch
//! and changes nothing. A change is saved, survives a restart over the same
//! store, and reaches both windows once without starting an update check.

mod common;

use std::path::Path;

use common::*;
use serde_json::{json, Value};
use wr_app::{App, AppConfig, Store, Theme, UpdateState};

/// Writes `settings.json` into `dir` as an earlier run would have.
fn write_settings(dir: &Path, value: &Value) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(
        dir.join("settings.json"),
        serde_json::to_string_pretty(value).unwrap(),
    )
    .unwrap();
}

fn saved(dir: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(dir.join("settings.json")).unwrap()).unwrap()
}

/// An app over `dir`'s store, built but not started (nothing registered, no check).
fn built(dir: &Path, shell: &FakeShell) -> std::sync::Arc<App> {
    App::new(AppConfig {
        data: fixture_data(),
        store: Store::open(dir),
        shell: Box::new(shell.clone()),
    })
}

#[test]
fn theme_loads_tolerantly_and_other_settings_survive() {
    // Missing, null, a number, an unknown string and a wrong case: Exile.
    let cases: Vec<(&str, Option<Value>)> = vec![
        ("missing", None),
        ("null", Some(Value::Null)),
        ("a number", Some(json!(7))),
        ("unknown", Some(json!("unknown"))),
        ("wrong case", Some(json!("Claude"))),
        ("a list", Some(json!(["claude"]))),
    ];
    for (what, theme) in cases {
        let tmp = tempfile::tempdir().unwrap();
        let mut settings = json!({
            "schema": 1,
            "hotkey": "Ctrl+Alt+Q",
            "auto_paste": false,
            "check_updates": true,
        });
        if let Some(t) = theme {
            settings["theme"] = t;
        }
        write_settings(tmp.path(), &settings);
        let app = built(tmp.path(), &FakeShell::default());
        assert_eq!(app.get_view().theme, Theme::Exile, "{what}: the main view");
        assert_eq!(app.get_app_settings().theme, Theme::Exile, "{what}: the Settings view");
        // Every other setting loads as before.
        let view = app.get_view();
        assert_eq!(view.hotkey, "Ctrl+Alt+Q", "{what}");
        assert!(!view.auto_paste, "{what}");
        assert!(app.get_app_settings().check_updates, "{what}");
        app.shutdown();
    }

    // No settings file at all: Exile.
    let tmp = tempfile::tempdir().unwrap();
    let app = built(tmp.path(), &FakeShell::default());
    assert_eq!(app.get_view().theme, Theme::Exile);
    app.shutdown();

    // The positive control: "claude" loads Claude, beside the other settings.
    let tmp = tempfile::tempdir().unwrap();
    write_settings(
        tmp.path(),
        &json!({ "schema": 1, "hotkey": "Ctrl+Alt+Q", "auto_paste": false, "theme": "claude" }),
    );
    let app = built(tmp.path(), &FakeShell::default());
    assert_eq!(app.get_view().theme, Theme::Claude);
    assert_eq!(app.get_app_settings().theme, Theme::Claude);
    assert_eq!(app.get_view().hotkey, "Ctrl+Alt+Q");
    app.shutdown();
}

#[test]
fn theme_patches_are_strict_and_atomic() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);
    assert_eq!(app.get_app_settings().theme, Theme::Exile);

    // Accepted alone, and beside the key the window sent before.
    let view = app.update_settings(&json!({ "theme": "claude" })).unwrap();
    assert_eq!(view.theme, Theme::Claude);
    assert_eq!(app.get_view().theme, Theme::Claude);
    let view = app
        .update_settings(&json!({ "theme": "exile", "check_updates": true }))
        .unwrap();
    assert_eq!(view.theme, Theme::Exile);
    assert!(view.check_updates);
    app.update_settings(&json!({ "theme": "claude", "check_updates": false }))
        .unwrap();

    // Refused whole: a theme that is not exactly "exile" or "claude", or a
    // key this window never sends, beside a good value or alone.
    for bad in [
        json!({ "theme": "Claude" }),
        json!({ "theme": "dark" }),
        json!({ "theme": null }),
        json!({ "theme": 1 }),
        json!({ "theme": ["claude"] }),
        json!({ "theme": "dark", "check_updates": true }),
        json!({ "theme": "exile", "hotkey": "Ctrl+Alt+R" }),
        json!({ "theme": "exile", "check_updates": "yes" }),
        json!({}),
        json!(["exile"]),
    ] {
        let err = app.update_settings(&bad).unwrap_err();
        assert!(!err.is_empty(), "{bad}");
        // State unchanged: the theme and the update setting as they were.
        assert_eq!(app.get_app_settings().theme, Theme::Claude, "{bad}");
        assert!(!app.get_app_settings().check_updates, "{bad}");
        assert_eq!(app.get_view().theme, Theme::Claude, "{bad}");
    }
    // A bad theme value is named in the reason.
    for bad in [json!({ "theme": "dark" }), json!({ "theme": null }), json!({ "theme": 1 })] {
        let err = app.update_settings(&bad).unwrap_err();
        assert!(err.contains("theme"), "{bad}: {err}");
    }
    app.flush();
    assert_eq!(saved(tmp.path())["theme"], "claude");
    assert_eq!(saved(tmp.path())["check_updates"], false);
    app.shutdown();
}

#[test]
fn theme_change_reaches_both_windows_once_and_starts_no_check() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);
    let views_before = shell.rec().views.len();
    let settings_before = shell.rec().app_settings.len();

    app.update_settings(&json!({ "theme": "claude" })).unwrap();
    {
        let rec = shell.rec();
        let views = &rec.views[views_before..];
        assert_eq!(views.len(), 1, "the main window hears it once");
        assert_eq!(views[0].theme, Theme::Claude);
        let sent = &rec.app_settings[settings_before..];
        assert_eq!(sent.len(), 1, "the Settings window hears it once");
        assert_eq!(sent[0].theme, Theme::Claude);
        assert!(!sent[0].check_updates, "the update setting is untouched");
    }
    // No update check started: the check is still where it was.
    assert_eq!(app.get_app_settings().update.state, UpdateState::Idle);
    assert!(!diag_lines(tmp.path()).iter().any(|l| l.starts_with("updates:")));

    // The same theme again changes nothing and tells no window.
    let views = shell.rec().views.len();
    let sent = shell.rec().app_settings.len();
    app.update_settings(&json!({ "theme": "claude" })).unwrap();
    assert_eq!(shell.rec().views.len(), views);
    assert_eq!(shell.rec().app_settings.len(), sent);

    // A refused patch tells no window either.
    app.update_settings(&json!({ "theme": "dark" })).unwrap_err();
    assert_eq!(shell.rec().views.len(), views);
    assert_eq!(shell.rec().app_settings.len(), sent);
    app.shutdown();
}

#[test]
fn theme_persists_across_a_restart_over_the_same_store() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);
    app.update_settings(&json!({ "theme": "claude" })).unwrap();
    app.shutdown();
    let file = saved(tmp.path());
    assert_eq!(file["theme"], "claude");
    assert_eq!(file["schema"], 1, "the schema stays 1");
    assert_eq!(file["hotkey"], "Ctrl+Alt+W", "the other settings are saved beside it");
    drop(app);

    let again = start(tmp.path(), &FakeShell::default());
    assert_eq!(again.get_view().theme, Theme::Claude);
    assert_eq!(again.get_app_settings().theme, Theme::Claude);
    again.update_settings(&json!({ "theme": "exile" })).unwrap();
    again.shutdown();
    assert_eq!(saved(tmp.path())["theme"], "exile");
    drop(again);

    let third = built(tmp.path(), &FakeShell::default());
    assert_eq!(third.get_view().theme, Theme::Exile);
    third.shutdown();
}

#[test]
fn theme_serialises_as_exile_or_claude_in_both_views() {
    let tmp = tempfile::tempdir().unwrap();
    let app = start(tmp.path(), &FakeShell::default());
    assert_eq!(serde_json::to_value(app.get_view()).unwrap()["theme"], "exile");
    assert_eq!(serde_json::to_value(app.get_app_settings()).unwrap()["theme"], "exile");
    app.update_settings(&json!({ "theme": "claude" })).unwrap();
    assert_eq!(serde_json::to_value(app.get_view()).unwrap()["theme"], "claude");
    assert_eq!(serde_json::to_value(app.get_app_settings()).unwrap()["theme"], "claude");
    app.shutdown();
}
