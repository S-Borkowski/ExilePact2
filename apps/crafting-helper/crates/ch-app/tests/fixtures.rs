//! The IPC contract's fixtures: one serialised view per state the window
//! pages render, built here by driving the app and compared with the files
//! under `fixtures/` (`fixtures/app-settings/`, the Settings window's
//! `SettingsView`; `fixtures/views/`, the main and overlay windows' `View`),
//! and `fixtures/catalogue.json`, the `get_catalogue` answer. The page tests
//! render the same files and hold them against the TypeScript types, so the
//! two sides cannot drift apart.
//!
//! With `CH_FIXTURES=1` set the test rewrites the files instead; review the
//! diff. A JSON file in either folder that is not in the lists fails it.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use ch_app::{
    CatalogueGroup, CraftMode, GuideStatusView, InstallState, OutcomeWord, SettingsView,
    StatusKind, Version, View,
};
use common::*;
use serde_json::{json, Value};

fn rewrite() -> bool {
    std::env::var("CH_FIXTURES").is_ok_and(|v| v == "1")
}

/// One Settings window view per check state and per install state, from
/// runs of the app through the fake transport (the install states' names
/// start `install-`), then the hotkey refused and a blocked list.
fn app_settings_views() -> Vec<(&'static str, SettingsView)> {
    let h = harness(Setup::default());
    h.app.start();
    let mut out = vec![("idle", h.app.get_app_settings())];
    h.hold_list(true);
    h.reply(signed_newer());
    out.push(("checking", h.app.check_for_updates()));
    h.hold_list(false);
    h.wait_calls(1);
    h.wait_ended();
    // Turned on, the setting starts nothing here: a check started at 0.
    h.app
        .update_settings(&json!({ "check_updates": true }))
        .unwrap();
    out.push(("available", h.app.get_app_settings()));
    out.push(("failed", h.manual(status(500))));
    out.push((
        "up-to-date",
        h.manual(list(&[RUNNING, Version::new(1, 0, 0)], true)),
    ));
    out.push(("no-release", h.manual(list(&[], true))));
    out.push(("install-unsigned", h.manual(list(&[NEWER], false))));
    assert_eq!(h.calls(), 5);
    h.app.shutdown();

    // Downloading, verifying, restarting: one install that succeeds.
    let h = harness(Setup::default());
    h.app.start();
    h.manual(signed_newer());
    h.hold_downloads(true);
    out.push(("install-downloading", h.app.install_update().unwrap()));
    h.hold_downloads(false);
    wait_until("the restart request", || h.restarts() == 1);
    let verifying = h
        .emitted()
        .into_iter()
        .find(|v| v.updates.install.state == InstallState::Verifying)
        .expect("a verifying view was sent");
    out.push(("install-verifying", verifying));
    out.push(("install-restarting", h.app.get_app_settings()));
    h.app.shutdown();

    // Failed: a signature by another key.
    let h = harness(Setup::default());
    *h.wire.sig.lock().unwrap() = signing("another-key.minisig");
    h.app.start();
    h.manual(signed_newer());
    h.app.install_update().unwrap();
    wait_until("the install to fail", || {
        h.app.get_app_settings().updates.install.state == InstallState::Failed
    });
    out.push(("install-failed", h.app.get_app_settings()));
    h.app.shutdown();

    // The hotkey refused, and a blocked list.
    let h = harness(Setup::default());
    h.app.start();
    out.push(("hotkey-refused", h.app.set_hotkey("Ctrl+Alt+C")));
    h.app.shutdown();
    let h = harness(Setup::default());
    h.app.start();
    out.push((
        "blocked-currencies",
        h.app
            .update_settings(
                &json!({ "blocked_currencies": ["divine-orb", "chaos-orb", "exalted-orb"] }),
            )
            .unwrap(),
    ));
    h.app.shutdown();
    out
}

/// One main view per state the main window and the overlay render.
fn main_views() -> Vec<(&'static str, View)> {
    let mut out = Vec::new();
    let tmp = tempfile::tempdir().unwrap();

    let shell = FakeShell::with_clipboard(ITEM_TEXT);
    let app = start(&tmp.path().join("a"), &shell);
    out.push(("empty", app.get_view()));
    app.hotkey_pressed();
    out.push(("grabbed-item", app.get_view()));
    shell.set_clipboard(Some(""));
    app.hotkey_pressed();
    out.push(("grab-refused", app.get_view()));
    out.push(("grabbed-not-item", app.grab_text(Some(NOT_ITEM_TEXT))));
    app.shutdown();

    let shell = FakeShell::with_taken(&["Ctrl+Alt+X"]);
    let app = start(&tmp.path().join("b"), &shell);
    out.push(("hotkey-taken", app.get_view()));
    app.shutdown();

    let shell = FakeShell::default();
    let app = start(&tmp.path().join("c"), &shell);
    app.grab_text(Some(ITEM_TEXT));
    app.set_craft_active(true);
    out.push(("overlay-active", app.get_view()));
    app.shutdown();

    // The craft window, over `ch-engine`'s whittled ring: each state from
    // an app of its own, planning the ring's start with its three targets
    // on the whittling route.
    let planned = |dir: &str| {
        let shell = FakeShell::default();
        let app = start(&tmp.path().join(dir), &shell);
        app.grab_text(Some(&ring("01-start")));
        app.set_targets(&ring_targets());
        app.choose_route(WHITTLE);
        app
    };
    let app = planned("planning");
    out.push(("craft-planning", app.get_view()));
    app.shutdown();
    let app = planned("fallback");
    out.push((
        "craft-fallback",
        app.set_craft_blocked(&lines(&["omen-of-whittling"])),
    ));
    app.shutdown();
    let app = planned("no-route");
    out.push((
        "craft-no-route",
        app.set_craft_blocked(&lines(&["chaos-orb"])),
    ));
    app.shutdown();

    let shell = FakeShell::default();
    let app = start(&tmp.path().join("refused"), &shell);
    let corrupted = format!("{}\n--------\nCorrupted", ring("01-start").trim_end());
    out.push(("craft-refused", app.grab_text(Some(&corrupted))));
    app.shutdown();

    // Guiding: one hit taken; a miss; an unexpected change; done.
    let guided = |dir: &str, files: &[&str]| {
        let app = planned(dir);
        app.accept_craft();
        let mut view = app.get_view();
        for file in files {
            view = app.grab_text(Some(&ring(file)));
        }
        app.shutdown();
        view
    };
    out.push(("craft-guiding", guided("guiding", &["02-chaos-fire"])));
    out.push(("craft-miss", guided("miss", &["06-chaos-below-floor"])));
    out.push(("craft-replan", guided("replan", &["03-whittle-cold"])));
    out.push((
        "craft-done",
        guided(
            "done",
            &[
                "02-chaos-fire",
                "03-whittle-cold",
                "04-desecrated",
                "05-revealed-lightning",
            ],
        ),
    ));
    out
}

fn json_files(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|n| n.ends_with(".json"))
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

/// Compares (or, with `CH_FIXTURES=1`, writes) one folder's files; returns
/// the names whose file differs.
fn compare(dir: &Path, views: &[(&'static str, Value)]) -> Vec<&'static str> {
    if rewrite() {
        fs::create_dir_all(dir).unwrap();
        for (name, value) in views {
            let text = serde_json::to_string_pretty(value).unwrap();
            fs::write(dir.join(format!("{name}.json")), text + "\n").unwrap();
        }
    }
    let mut want: Vec<String> = views.iter().map(|(n, _)| format!("{n}.json")).collect();
    want.sort();
    assert_eq!(
        json_files(dir),
        want,
        "{} holds exactly one file per state",
        dir.display()
    );
    let mut stale = Vec::new();
    for (name, value) in views {
        let path = dir.join(format!("{name}.json"));
        let text = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("{}: {e} (run with CH_FIXTURES=1)", path.display()));
        let on_disk: Value = serde_json::from_str(&text).unwrap();
        if on_disk != *value {
            stale.push(*name);
        }
    }
    stale
}

fn fixture_dir(name: &str) -> PathBuf {
    app_root().join("fixtures").join(name)
}

#[test]
fn fixtures_are_current() {
    let settings = app_settings_views();
    let views = main_views();

    // Each Settings view is the state it is named for, from the synthetic
    // versions, and carries no URL.
    for (name, view) in &settings {
        let (part, state) = match name.strip_prefix("install-") {
            Some(s) => (serde_json::to_value(view.updates.install.state).unwrap(), s),
            None => (
                serde_json::to_value(view.updates.update.state).unwrap(),
                *name,
            ),
        };
        if !matches!(*name, "hotkey-refused" | "blocked-currencies") {
            assert_eq!(part, json!(state), "{name}");
        }
        assert_eq!(view.updates.version, RUNNING.to_string());
        let text = serde_json::to_string(view).unwrap();
        assert!(!text.contains("://"), "{name}: {text}");
    }
    let by = |n: &str| &settings.iter().find(|(name, _)| *name == n).unwrap().1;
    assert_eq!(by("available").updates.install.state, InstallState::Ready);
    assert_eq!(
        by("failed").updates.update.newer.as_deref(),
        Some("1.3.0"),
        "failed keeps the newer"
    );
    assert_eq!(by("idle").updates.install.state, InstallState::None);
    let refused = by("hotkey-refused");
    assert_eq!(refused.hotkey, "Ctrl+Alt+X");
    assert_eq!(
        refused.hotkey_status.as_ref().unwrap().kind,
        StatusKind::Error
    );
    assert_eq!(by("blocked-currencies").blocked_currencies.len(), 3);

    // Each main view shows what it is named for.
    let view = |n: &str| &views.iter().find(|(name, _)| *name == n).unwrap().1;
    assert!(view("empty").grab.is_none() && view("empty").status.is_none());
    assert!(view("grabbed-item").grab.as_ref().unwrap().looks_like_item);
    assert!(
        !view("grabbed-not-item")
            .grab
            .as_ref()
            .unwrap()
            .looks_like_item
    );
    assert_eq!(view("grab-refused").grab, view("grabbed-item").grab);
    assert_eq!(
        view("grab-refused").status.as_ref().unwrap().kind,
        StatusKind::Error
    );
    assert_eq!(
        view("hotkey-taken").status.as_ref().unwrap().kind,
        StatusKind::Error
    );
    assert!(view("overlay-active").overlay.visible);
    assert!(views
        .iter()
        .filter(|(n, _)| *n != "overlay-active")
        .all(|(_, v)| !v.overlay.visible));

    // Each craft view is the mode and state it is named for.
    assert_eq!(view("empty").craft.mode, CraftMode::None);
    let plan = |n: &str| view(n).craft.planning.as_ref().unwrap();
    let preview = |n: &str| plan(n).preview.as_ref().unwrap();
    let route = preview("craft-planning").route.as_ref().unwrap();
    assert!(route
        .steps
        .iter()
        .any(|s| s.odds.as_deref().is_some_and(|o| o.starts_with('~'))));
    assert!(route.steps.iter().any(|s| s
        .odds
        .as_deref()
        .is_some_and(|o| o.starts_with("odds unknown: "))));
    assert!(plan("craft-planning").can_accept);
    assert!(preview("craft-fallback")
        .route
        .as_ref()
        .unwrap()
        .steps
        .iter()
        .any(|s| s.fallback.is_some()));
    assert!(preview("craft-no-route").route.is_none());
    assert!(!preview("craft-no-route").rejected.is_empty());
    assert!(!plan("craft-no-route").can_accept);
    assert_eq!(view("craft-refused").craft.mode, CraftMode::Refused);
    let guide = |n: &str| view(n).craft.guiding.as_ref().unwrap();
    assert_eq!(guide("craft-guiding").status, GuideStatusView::Active);
    assert_eq!(guide("craft-guiding").history.len(), 1);
    assert_eq!(guide("craft-miss").history[0].result, OutcomeWord::Miss);
    assert_eq!(guide("craft-miss").step.as_ref().unwrap().try_number, 2);
    assert_eq!(guide("craft-replan").status, GuideStatusView::Replan);
    assert_eq!(guide("craft-done").status, GuideStatusView::Done);
    for (name, v) in &views {
        let mode = v.craft.mode;
        let parts = (
            v.craft.refusal.is_some(),
            v.craft.planning.is_some(),
            v.craft.guiding.is_some(),
        );
        let want = match mode {
            CraftMode::None => (false, false, false),
            CraftMode::Refused => (true, false, false),
            CraftMode::Planning => (false, true, false),
            CraftMode::Guiding => (false, false, true),
        };
        assert_eq!(parts, want, "{name}: exactly the part of its mode");
        let text = serde_json::to_string(&v.craft).unwrap();
        assert!(!text.contains("://"), "{name}: {text}");
    }

    // Every view reads back into its type and writes the same JSON; the
    // main view refuses an added and a dropped field.
    for (name, v) in &views {
        let value = serde_json::to_value(v).unwrap();
        let back: View = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(back).unwrap(), value, "{name}");
        let mut extra = value.clone();
        extra["overlay"]["extra"] = Value::Bool(true);
        assert!(serde_json::from_value::<View>(extra).is_err(), "{name}");
        let mut dropped = value.clone();
        dropped["overlay"].as_object_mut().unwrap().remove("line");
        assert!(serde_json::from_value::<View>(dropped).is_err(), "{name}");
        let mut extra = value.clone();
        extra["craft"]["extra"] = Value::Bool(true);
        assert!(serde_json::from_value::<View>(extra).is_err(), "{name}");
    }
    for (name, v) in &settings {
        let value = serde_json::to_value(v).unwrap();
        let back: SettingsView = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(back).unwrap(), value, "{name}");
        let mut dropped = value.clone();
        dropped
            .as_object_mut()
            .unwrap()
            .remove("blocked_currencies");
        assert!(
            serde_json::from_value::<SettingsView>(dropped).is_err(),
            "{name}"
        );
    }

    let settings: Vec<(&'static str, Value)> = settings
        .iter()
        .map(|(n, v)| (*n, serde_json::to_value(v).unwrap()))
        .collect();
    let views: Vec<(&'static str, Value)> = views
        .iter()
        .map(|(n, v)| (*n, serde_json::to_value(v).unwrap()))
        .collect();
    assert_eq!(settings.len(), 13);
    assert_eq!(views.len(), 14);
    let mut stale = compare(&fixture_dir("app-settings"), &settings);
    stale.extend(compare(&fixture_dir("views"), &views));
    assert!(
        stale.is_empty(),
        "stale fixtures {stale:?}: run with CH_FIXTURES=1 and review the diff"
    );
}

/// `fixtures/catalogue.json`: the `get_catalogue` answer, which both pages'
/// tests and previews read for their pickers.
#[test]
fn catalogue_fixture_is_current() {
    let tmp = tempfile::tempdir().unwrap();
    let app = start(tmp.path(), &FakeShell::default());
    let groups = app.get_catalogue();
    app.shutdown();
    assert!(!groups.is_empty());

    // It reads back into its type and writes the same JSON; an added field
    // is refused.
    let value = serde_json::to_value(&groups).unwrap();
    let back: Vec<CatalogueGroup> = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(back).unwrap(), value);
    let mut extra = value.clone();
    extra[0]["entries"][0]["extra"] = Value::Bool(true);
    assert!(serde_json::from_value::<Vec<CatalogueGroup>>(extra).is_err());

    let path = app_root().join("fixtures").join("catalogue.json");
    if rewrite() {
        let text = serde_json::to_string_pretty(&value).unwrap();
        fs::write(&path, text + "\n").unwrap();
    }
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: {e} (run with CH_FIXTURES=1)", path.display()));
    let on_disk: Value = serde_json::from_str(&text).unwrap();
    assert!(
        on_disk == value,
        "stale {}: run with CH_FIXTURES=1 and review the diff",
        path.display()
    );
}
