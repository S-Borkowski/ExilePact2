//! The saved settings, through `App` with a recording fake shell and a
//! temporary store: `check_updates` off unless saved `true`, and the blocked
//! currencies saved, normalised, refused whole when bad and read back by a
//! new app on the same store.

mod common;

use ch_app::{App, AppConfig, Store, StoreError};
use common::*;
use serde_json::{json, Value};

#[test]
fn check_updates_off_by_default() {
    // A first launch, a file without the key, and any value but `true`.
    for saved in [
        None,
        Some(json!({ "schema": 1, "hotkey": "Ctrl+Alt+X" })),
        Some(json!({ "schema": 1, "check_updates": "true" })),
        Some(json!({ "schema": 1, "check_updates": 1 })),
        Some(json!({ "schema": 1, "check_updates": null })),
    ] {
        let tmp = tempfile::tempdir().unwrap();
        if let Some(s) = &saved {
            save_settings(tmp.path(), s);
        }
        let app = start(tmp.path(), &FakeShell::default());
        assert!(!app.get_app_settings().updates.check_updates, "{saved:?}");
        app.shutdown();
    }

    // Through the harness: off, no request across two days of ticks, and
    // saved off with the rest.
    let h = harness(Setup::default());
    h.app.start();
    assert!(!h.app.get_app_settings().updates.check_updates);
    h.ticks(0, 48 * HOUR);
    assert_eq!(h.calls(), 0, "a request nobody asked for");
    assert!(h.update_lines().is_empty(), "{:?}", h.update_lines());
    let _ = h.app.set_hotkey("Ctrl+Alt+R");
    h.app.flush();
    assert_eq!(h.saved_settings()["check_updates"], false);
    // Positive control: the same instrument counts a request once asked.
    h.manual(list(&[RUNNING], true));
    assert_eq!(h.calls(), 1);

    // Saved `true` is on; turned off is saved off.
    let view = h
        .app
        .update_settings(&json!({ "check_updates": true }))
        .unwrap();
    assert!(view.updates.check_updates);
    h.app.shutdown();
    assert_eq!(h.saved_settings()["check_updates"], true);
    let again = make(&h.store, &FakeShell::default());
    assert!(again.get_app_settings().updates.check_updates);
    again
        .update_settings(&json!({ "check_updates": false }))
        .unwrap();
    again.shutdown();
    assert_eq!(h.saved_settings()["check_updates"], false);
    let log = diag_lines(&h.store);
    assert!(log.contains(&"updates: automatic checks turned on".to_string()));
    assert!(log.contains(&"updates: automatic checks turned off".to_string()));
}

#[test]
fn blocked_currencies_persist() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);
    assert!(app.get_app_settings().blocked_currencies.is_empty());

    // Trimmed, empty ids dropped, repeats removed keeping the first order.
    let view = app
        .update_settings(&json!({
            "blocked_currencies": [" divine ", "chaos", "", "  ", "divine", "exalted", "chaos"]
        }))
        .unwrap();
    let want = vec![
        "divine".to_string(),
        "chaos".to_string(),
        "exalted".to_string(),
    ];
    assert_eq!(view.blocked_currencies, want);
    // Both windows heard it.
    assert_eq!(
        shell.rec().app_settings.last().unwrap().blocked_currencies,
        want
    );
    assert!(!shell.rec().views.is_empty());

    // Refused whole, with nothing changed: not a list, not text, too long an
    // id, too many ids, another key, a bad value beside a good one.
    let many: Vec<String> = (0..201).map(|i| format!("currency-{i}")).collect();
    for bad in [
        json!({ "blocked_currencies": "divine" }),
        json!({ "blocked_currencies": ["divine", 7] }),
        json!({ "blocked_currencies": ["x".repeat(65)] }),
        json!({ "blocked_currencies": many }),
        json!({ "blocked_currencies": [], "hotkey": "Ctrl+Alt+R" }),
        json!({ "blocked_currencies": [], "check_updates": "yes" }),
        json!({ "auto_install": true }),
        json!({}),
        json!(["divine"]),
    ] {
        assert!(app.update_settings(&bad).is_err(), "{bad}");
        assert_eq!(app.get_app_settings().blocked_currencies, want, "{bad}");
    }
    // The limits themselves are taken.
    let two_hundred: Vec<String> = (0..200).map(|i| format!("currency-{i}")).collect();
    let view = app
        .update_settings(&json!({ "blocked_currencies": two_hundred }))
        .unwrap();
    assert_eq!(view.blocked_currencies.len(), 200);
    let view = app
        .update_settings(
            &json!({ "blocked_currencies": [&want[0], &want[1], &want[2], "x".repeat(64)] }),
        )
        .unwrap();
    assert_eq!(view.blocked_currencies.len(), 4);
    app.shutdown();

    // Saved, and read back by a new app on the same store.
    let saved = saved_settings(tmp.path());
    assert_eq!(saved["schema"], 1);
    assert_eq!(saved["hotkey"], "Ctrl+Alt+X");
    assert_eq!(saved["check_updates"], false);
    assert_eq!(saved["blocked_currencies"].as_array().unwrap().len(), 4);
    let again = start(tmp.path(), &FakeShell::default());
    let read = again.get_app_settings().blocked_currencies;
    assert_eq!(read[..3], want[..]);
    assert_eq!(read[3], "x".repeat(64));
    // The empty list clears it.
    let view = again
        .update_settings(&json!({ "blocked_currencies": [] }))
        .unwrap();
    assert!(view.blocked_currencies.is_empty());
    again.shutdown();
    assert_eq!(saved_settings(tmp.path())["blocked_currencies"], json!([]));
    assert!(diag_lines(tmp.path()).contains(&"blocked currencies: 3 ids".to_string()));
}

#[test]
fn a_bad_saved_list_keeps_its_valid_ids() {
    let tmp = tempfile::tempdir().unwrap();
    save_settings(
        tmp.path(),
        &json!({
            "schema": 1,
            "blocked_currencies": ["divine", 3, "", " chaos ", "divine", "x".repeat(65)]
        }),
    );
    let app = start(tmp.path(), &FakeShell::default());
    assert_eq!(
        app.get_app_settings().blocked_currencies,
        vec!["divine".to_string(), "chaos".to_string()]
    );
    let status = app.get_view().status.unwrap();
    assert_eq!(
        status.text,
        "Dropped 4 saved blocked currencies that were not valid."
    );
    app.shutdown();
}

#[test]
fn a_store_inside_a_git_checkout_is_refused_and_nothing_is_saved() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join(".git")).unwrap();
    let dir = tmp.path().join("store");
    let store = Store::open(&dir);
    assert!(matches!(store, Err(StoreError::GitCheckout { .. })));
    let shell = FakeShell::default();
    let app = App::new(AppConfig {
        store,
        shell: Box::new(shell.clone()),
    });
    app.start();
    let status = app.get_view().status.unwrap();
    assert!(
        status.text.starts_with("Settings are not saved: "),
        "{}",
        status.text
    );
    assert!(status
        .text
        .ends_with("Set CRAFTING_HELPER_DIR to another folder."));
    let _ = app.update_settings(&json!({ "blocked_currencies": ["divine"] }));
    app.shutdown();
    assert!(!dir.exists());
}

#[test]
fn an_unreadable_settings_file_starts_from_the_defaults() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("settings.json"), b"{ not json").unwrap();
    let app = start(tmp.path(), &FakeShell::default());
    let view = app.get_app_settings();
    assert_eq!(view.hotkey, "Ctrl+Alt+X");
    assert!(view.blocked_currencies.is_empty());
    assert!(!view.updates.check_updates);
    assert!(app
        .get_view()
        .status
        .unwrap()
        .text
        .starts_with("The saved settings could not be read: "));
    // A change writes a good file over the broken one.
    let _ = app.set_hotkey("Ctrl+Alt+R");
    app.shutdown();
    let saved: Value = saved_settings(tmp.path());
    assert_eq!(saved["hotkey"], "Ctrl+Alt+R");
}
