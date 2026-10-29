//! The saved settings, through `App` with a recording fake shell and a
//! temporary store: `check_updates` off unless saved `true`, and the blocked
//! currencies saved, normalised, refused whole when bad (an id outside the
//! catalogue included) and read back by a new app on the same store, and
//! the catalogue the blocked list picks from; the overlay's position and the
//! screen watch, saved and read back, an unknown position refused.

mod common;

use ch_app::ch_engine::catalogue;
use ch_app::{App, AppConfig, OverlayPosition, Store, StoreError};
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
            "blocked_currencies": [
                " divine-orb ", "chaos-orb", "", "  ", "divine-orb", "exalted-orb", "chaos-orb"
            ]
        }))
        .unwrap();
    let want = vec![
        "divine-orb".to_string(),
        "chaos-orb".to_string(),
        "exalted-orb".to_string(),
    ];
    assert_eq!(view.blocked_currencies, want);
    // Both windows heard it.
    assert_eq!(
        shell.rec().app_settings.last().unwrap().blocked_currencies,
        want
    );
    assert!(!shell.rec().views.is_empty());

    // Refused whole, with nothing changed: not a list, not text, too long an
    // id, an id outside the catalogue, another key, a bad value beside a
    // good one.
    for bad in [
        json!({ "blocked_currencies": "divine-orb" }),
        json!({ "blocked_currencies": ["divine-orb", 7] }),
        json!({ "blocked_currencies": ["x".repeat(65)] }),
        json!({ "blocked_currencies": ["divine-orb", "divine"] }),
        json!({ "blocked_currencies": [], "hotkey": "Ctrl+Alt+R" }),
        json!({ "blocked_currencies": [], "check_updates": "yes" }),
        json!({ "auto_install": true }),
        json!({}),
        json!(["divine-orb"]),
    ] {
        assert!(app.update_settings(&bad).is_err(), "{bad}");
        assert_eq!(app.get_app_settings().blocked_currencies, want, "{bad}");
    }
    let view = app
        .update_settings(
            &json!({ "blocked_currencies": [&want[0], &want[1], &want[2], "regal-orb"] }),
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
    assert_eq!(read[3], "regal-orb");
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
            "blocked_currencies": ["divine-orb", 3, "", " chaos-orb ", "divine-orb", "x".repeat(65)]
        }),
    );
    let app = start(tmp.path(), &FakeShell::default());
    assert_eq!(
        app.get_app_settings().blocked_currencies,
        vec!["divine-orb".to_string(), "chaos-orb".to_string()]
    );
    let status = app.get_view().status.unwrap();
    assert_eq!(
        status.text,
        "Dropped 4 saved blocked currencies that were not valid."
    );
    app.shutdown();
}

#[test]
fn catalogue_is_grouped_with_names() {
    let tmp = tempfile::tempdir().unwrap();
    let app = start(tmp.path(), &FakeShell::default());
    let groups = app.get_catalogue();
    app.shutdown();

    // One group per kind, in the catalogue's kind order, each with a label
    // and its entries sorted by name.
    let kinds: Vec<&str> = groups.iter().map(|g| g.kind.as_str()).collect();
    let mut want_kinds: Vec<String> = Vec::new();
    for c in catalogue() {
        let kind = serde_json::to_value(c.kind).unwrap();
        let kind = kind.as_str().unwrap().to_string();
        if !want_kinds.contains(&kind) {
            want_kinds.push(kind);
        }
    }
    let mut order = want_kinds.clone();
    order.sort_by_key(|k| KIND_ORDER.iter().position(|o| o == k).unwrap());
    assert_eq!(kinds, order, "one group per kind, in order");
    assert_eq!(groups[0].kind, "currency");
    assert_eq!(groups[0].label, "Currency");
    for g in &groups {
        assert!(!g.label.trim().is_empty(), "{}", g.kind);
        assert!(!g.entries.is_empty(), "{}", g.kind);
        let names: Vec<String> = g.entries.iter().map(|e| e.name.to_lowercase()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted, "{} is sorted by name", g.kind);
    }
    let labels: Vec<&str> = groups.iter().map(|g| g.label.as_str()).collect();
    assert!(labels.contains(&"Omens"), "{labels:?}");

    // Every catalogue id exactly once, with its English name, in its kind's
    // group, and nothing else.
    let mut seen = 0;
    for c in catalogue() {
        let kind = serde_json::to_value(c.kind).unwrap();
        let found: Vec<_> = groups
            .iter()
            .flat_map(|g| g.entries.iter().map(move |e| (g, e)))
            .filter(|(_, e)| e.id == c.id)
            .collect();
        assert_eq!(found.len(), 1, "{}", c.id);
        assert_eq!(found[0].1.name, c.name, "{}", c.id);
        assert_eq!(json!(found[0].0.kind), kind, "{}", c.id);
        seen += 1;
    }
    let total: usize = groups.iter().map(|g| g.entries.len()).sum();
    assert_eq!(total, seen);
    let divine = groups[0]
        .entries
        .iter()
        .find(|e| e.id == "divine-orb")
        .unwrap();
    assert_eq!(divine.name, "Divine Orb");
}

/// The kinds in the order the catalogue groups them.
const KIND_ORDER: [&str; 8] = [
    "currency", "omen", "bone", "flux", "catalyst", "essence", "alloy", "liquid",
];

#[test]
fn blocking_the_whole_catalogue_is_accepted() {
    let tmp = tempfile::tempdir().unwrap();
    let app = start(tmp.path(), &FakeShell::default());
    let all: Vec<String> = catalogue().iter().map(|c| c.id.clone()).collect();
    // More than M3's literal limit of 200: the catalogue is the limit now.
    assert!(all.len() > 200, "{}", all.len());

    // The whole catalogue, each id given twice: every id kept once, in the
    // order given.
    let twice: Vec<String> = all.iter().chain(all.iter()).cloned().collect();
    let view = app
        .update_settings(&json!({ "blocked_currencies": twice }))
        .unwrap();
    assert_eq!(view.blocked_currencies, all);
    app.shutdown();
    assert_eq!(saved_settings(tmp.path())["blocked_currencies"], json!(all));

    // Read back whole by a new app, with nothing dropped.
    let again = start(tmp.path(), &FakeShell::default());
    assert_eq!(again.get_app_settings().blocked_currencies, all);
    assert!(again.get_view().status.is_none());
    again.shutdown();
}

#[test]
fn an_id_outside_the_catalogue_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);
    // Positive control: a catalogue id is taken.
    let view = app
        .update_settings(&json!({ "blocked_currencies": ["divine-orb"] }))
        .unwrap();
    assert_eq!(view.blocked_currencies, vec!["divine-orb".to_string()]);
    let sent = shell.rec().app_settings.len();

    // M3's short ids, a name instead of an id, a wrong case and a made-up
    // id, alone or beside good ones and with another good change: each
    // refused whole, with nothing changed and nothing sent.
    for outside in [
        "divine",
        "exalted",
        "Divine Orb",
        "DIVINE-ORB",
        "orb-of-nothing",
    ] {
        let patch = json!({
            "check_updates": true,
            "blocked_currencies": ["chaos-orb", outside, "exalted-orb"]
        });
        let why = app.update_settings(&patch).unwrap_err();
        assert!(why.contains(&format!("{outside:?}")), "{outside}: {why}");
        let now = app.get_app_settings();
        assert_eq!(now.blocked_currencies, vec!["divine-orb".to_string()]);
        assert!(!now.updates.check_updates, "{outside}");
    }
    assert_eq!(shell.rec().app_settings.len(), sent);
    app.shutdown();
    assert_eq!(
        saved_settings(tmp.path())["blocked_currencies"],
        json!(["divine-orb"])
    );
    let log = diag_lines(tmp.path());
    assert!(
        log.iter()
            .any(|l| l.starts_with("settings refused: ") && l.contains("\"orb-of-nothing\"")),
        "{log:?}"
    );
}

#[test]
fn saved_ids_outside_the_catalogue_are_dropped() {
    let tmp = tempfile::tempdir().unwrap();
    // M3's short ids beside real ones: the real ones kept in order, the
    // others dropped and counted.
    save_settings(
        tmp.path(),
        &json!({
            "schema": 1,
            "blocked_currencies": ["divine", "chaos-orb", "exalted", " divine-orb ", "Regal Orb"]
        }),
    );
    let app = start(tmp.path(), &FakeShell::default());
    assert_eq!(
        app.get_app_settings().blocked_currencies,
        vec!["chaos-orb".to_string(), "divine-orb".to_string()]
    );
    assert_eq!(
        app.get_view().status.unwrap().text,
        "Dropped 3 saved blocked currencies that were not valid."
    );
    // A change saves the list as read, without the dropped ids.
    let _ = app.set_hotkey("Ctrl+Alt+R");
    app.shutdown();
    assert_eq!(
        saved_settings(tmp.path())["blocked_currencies"],
        json!(["chaos-orb", "divine-orb"])
    );

    // One outside id is counted as one.
    let tmp = tempfile::tempdir().unwrap();
    save_settings(
        tmp.path(),
        &json!({ "schema": 1, "blocked_currencies": ["chaos-orb", "chaos"] }),
    );
    let app = start(tmp.path(), &FakeShell::default());
    assert_eq!(
        app.get_app_settings().blocked_currencies,
        vec!["chaos-orb".to_string()]
    );
    assert_eq!(
        app.get_view().status.unwrap().text,
        "Dropped 1 saved blocked currency that was not valid."
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
        screen: None,
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
    let _ = app.update_settings(&json!({ "blocked_currencies": ["divine-orb"] }));
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

#[test]
fn overlay_position_and_screen_watch_are_saved() {
    // The defaults: a first launch, a file without the keys, and saved
    // values that are not one of theirs.
    for saved in [
        None,
        Some(json!({ "schema": 1, "hotkey": "Ctrl+Alt+X" })),
        Some(json!({ "schema": 1, "overlay_position": "middle", "screen_watch": "off" })),
        Some(json!({ "schema": 1, "overlay_position": 2, "screen_watch": null })),
    ] {
        let tmp = tempfile::tempdir().unwrap();
        if let Some(s) = &saved {
            save_settings(tmp.path(), s);
        }
        let app = start(tmp.path(), &FakeShell::default());
        let view = app.get_app_settings();
        assert_eq!(
            view.overlay_position,
            OverlayPosition::TopCenter,
            "{saved:?}"
        );
        assert!(view.screen_watch, "{saved:?}");
        assert_eq!(app.get_view().overlay.position, OverlayPosition::TopCenter);
        app.shutdown();
    }

    // Changed, either or both, saved with the rest, and read back.
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);
    let view = app
        .update_settings(&json!({ "overlay_position": "bottom-left" }))
        .unwrap();
    assert_eq!(view.overlay_position, OverlayPosition::BottomLeft);
    assert!(view.screen_watch);
    let view = app
        .update_settings(&json!({ "screen_watch": false, "overlay_position": "top-right" }))
        .unwrap();
    assert_eq!(view.overlay_position, OverlayPosition::TopRight);
    assert!(!view.screen_watch);
    // The Settings window and the overlay heard it.
    assert_eq!(shell.rec().app_settings.last(), Some(&view));
    assert_eq!(
        shell.last_view().overlay.position,
        OverlayPosition::TopRight
    );
    // Refused whole: an unknown position, a watch that is not true or
    // false, an unknown key beside a good one.
    for bad in [
        json!({ "overlay_position": "centre" }),
        json!({ "overlay_position": "Top-Left" }),
        json!({ "screen_watch": 0 }),
        json!({ "screen_watch": true, "overlay_scale": 2 }),
        json!({ "overlay_position": "top-left", "screen_watch": "yes" }),
    ] {
        assert!(app.update_settings(&bad).is_err(), "{bad}");
    }
    let view = app.get_app_settings();
    assert_eq!(view.overlay_position, OverlayPosition::TopRight);
    assert!(!view.screen_watch);
    app.shutdown();
    let saved = saved_settings(tmp.path());
    assert_eq!(saved["overlay_position"], "top-right");
    assert_eq!(saved["screen_watch"], false);
    assert_eq!(saved["hotkey"], "Ctrl+Alt+X");

    let again = start(tmp.path(), &FakeShell::default());
    let view = again.get_app_settings();
    assert_eq!(view.overlay_position, OverlayPosition::TopRight);
    assert!(!view.screen_watch);
    again
        .update_settings(&json!({ "screen_watch": true }))
        .unwrap();
    again.shutdown();
    assert_eq!(saved_settings(tmp.path())["screen_watch"], true);
    let log = diag_lines(tmp.path());
    for line in [
        "overlay position: bottom-left",
        "overlay position: top-right",
        "screen watch: off",
        "screen watch: on",
    ] {
        assert!(log.contains(&line.to_string()), "{line}: {log:?}");
    }
}
