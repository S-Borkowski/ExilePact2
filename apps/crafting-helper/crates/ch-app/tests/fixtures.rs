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
use std::sync::atomic::Ordering;

use ch_app::{
    CatalogueGroup, CraftMode, GameWindow, GuideStatusView, InstallState, OcrReadiness,
    OutcomeSource, OutcomeWord, OverlayPosition, SettingsView, StatusKind, Version, View,
};
use common::*;
use serde_json::{json, Value};

fn rewrite() -> bool {
    std::env::var("CH_FIXTURES").is_ok_and(|v| v == "1")
}

/// One Settings window view per check state and per install state, from
/// runs of the app through the fake transport (the install states' names
/// start `install-`), then the hotkey refused, a blocked list and the
/// overlay's settings changed.
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

    // The overlay's position other than the default, and the screen watch
    // off.
    let h = harness(Setup::default());
    h.app.start();
    out.push((
        "overlay-settings",
        h.app
            .update_settings(&json!({ "overlay_position": "bottom-right", "screen_watch": false }))
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
    // Right after the grab of the ring's start, before any target is
    // changed: its own two modifiers ticked, each at its tier.
    let shell = FakeShell::default();
    let app = start(&tmp.path().join("prefilled"), &shell);
    out.push(("craft-prefilled", app.grab_text(Some(&ring("01-start")))));
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

    // The fill route on `ch-engine`'s fill ring: planned on `03-fill-fire`
    // with the seed walk's targets, its exalt missed (`07-fill-miss`
    // rolled accuracy): on `fill-clean`, which the app names by the
    // modifier levels (life, a target, is the lowest: the annulment).
    let shell = FakeShell::default();
    let app = start(&tmp.path().join("miss-clean"), &shell);
    app.grab_text(Some(&fill("03-fill-fire")));
    app.set_targets(&fill_targets());
    app.choose_route(FILL);
    app.accept_craft();
    out.push((
        "craft-miss-clean",
        app.grab_text(Some(&fill("07-fill-miss"))),
    ));
    app.shutdown();

    // The overlay: guiding after one hit, the game's window in front (shown)
    // or not (hidden), the watch ticked once by hand.
    let overlay = |dir: &str, window| {
        let shell = FakeShell::default();
        let screen = FakeScreen::default();
        screen.set(window);
        let app = start_with_screen(&tmp.path().join(dir), &shell, &screen, true);
        app.grab_text(Some(&ring("01-start")));
        app.set_targets(&ring_targets());
        app.choose_route(WHITTLE);
        app.accept_craft();
        app.grab_text(Some(&ring("02-chaos-fire")));
        app.tick_watch();
        let view = app.get_view();
        app.shutdown();
        view
    };
    out.push(("overlay-active", overlay("overlay", in_front(CLIENT))));
    out.push((
        "overlay-not-in-front",
        overlay("not-in-front", Ok(GameWindow::NotInFront)),
    ));

    // The screen watch: guiding from the ring's start with the game in
    // front, the settings patched first, then `reads` ticks by hand, 400 ms
    // apart on the watch's clock.
    let watching = |dir: &str, screen: &FakeScreen, settings: Option<Value>, reads: usize| {
        let shell = FakeShell::default();
        let (app, ms) = start_watching(&tmp.path().join(dir), &shell, screen);
        if let Some(patch) = settings {
            app.update_settings(&patch).unwrap();
        }
        app.grab_text(Some(&ring("01-start")));
        app.set_targets(&ring_targets());
        app.choose_route(WHITTLE);
        app.accept_craft();
        for _ in 0..reads {
            app.tick_watch();
            ms.fetch_add(400, Ordering::SeqCst);
        }
        let view = app.get_view();
        app.shutdown();
        view
    };
    let screen = FakeScreen::in_front();
    out.push(("craft-watching", watching("watching", &screen, None, 1)));
    let screen = FakeScreen::in_front();
    screen.show(tooltip_of(&ring("02-chaos-fire")));
    out.push(("craft-screen-hit", watching("screen-hit", &screen, None, 2)));
    let screen = FakeScreen::in_front();
    screen.show(tooltip_of(&ring("03-whittle-cold")));
    out.push((
        "craft-screen-unexpected",
        watching("screen-unexpected", &screen, None, 2),
    ));
    let screen = FakeScreen::in_front();
    out.push((
        "craft-watch-off",
        watching(
            "watch-off",
            &screen,
            Some(json!({ "screen_watch": false })),
            1,
        ),
    ));
    let screen = FakeScreen::in_front();
    screen.rec().ocr = Ok(OcrReadiness::NoEnglish);
    out.push((
        "craft-watch-unavailable",
        watching("watch-unavailable", &screen, None, 1),
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
        if !matches!(
            *name,
            "hotkey-refused" | "blocked-currencies" | "overlay-settings"
        ) {
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
    assert_eq!(
        by("overlay-settings").overlay_position,
        OverlayPosition::BottomRight
    );
    assert!(!by("overlay-settings").screen_watch);
    assert!(settings
        .iter()
        .filter(|(n, _)| *n != "overlay-settings")
        .all(|(_, v)| v.screen_watch && v.overlay_position == OverlayPosition::TopCenter));

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
    // The overlay shows in the views with the game in front: the active
    // overlay and the screen watch's.
    const IN_FRONT: [&str; 6] = [
        "overlay-active",
        "craft-watching",
        "craft-screen-hit",
        "craft-screen-unexpected",
        "craft-watch-off",
        "craft-watch-unavailable",
    ];
    for (name, v) in &views {
        assert_eq!(v.overlay.visible, IN_FRONT.contains(name), "{name}");
    }
    // Both overlay views guide; only the game in front shows it. Every
    // guiding view carries the overlay's content, and no other does.
    for n in ["overlay-active", "overlay-not-in-front"] {
        assert_eq!(view(n).craft.mode, CraftMode::Guiding, "{n}");
        let guide = view(n).overlay.guide.as_ref().unwrap();
        assert_eq!(guide.last.as_ref().unwrap().result, OutcomeWord::Hit);
    }
    for (name, v) in &views {
        assert_eq!(
            v.overlay.guide.is_some(),
            v.craft.mode == CraftMode::Guiding,
            "{name}"
        );
    }

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
    // No route carries the blueprint's notes; every step line carries its
    // target modifiers.
    assert!(route.steps.iter().all(|s| !s.targets.is_empty()));
    let prefilled = plan("craft-prefilled");
    assert_eq!(prefilled.targets.len(), 2);
    assert!(prefilled.item.modifiers.iter().all(|m| m.meets_target));
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
    let clean = guide("craft-miss-clean").step.as_ref().unwrap();
    assert_eq!(clean.step.id, "fill-clean");
    assert_eq!(clean.step.uses, ["Orb of Annulment"]);
    assert!(clean.step.fallback.is_none());
    assert_eq!(guide("craft-replan").status, GuideStatusView::Replan);
    assert_eq!(guide("craft-done").status, GuideStatusView::Done);

    // The screen watch's views: the watch line each names, the hit from the
    // screen kept, the unexpected change not.
    let line = |n: &str| guide(n).watch.clone().unwrap_or_else(|| panic!("{n}"));
    assert_eq!(
        line("craft-watching").text,
        "Watching: after the step, hover the item in the game."
    );
    let hit = guide("craft-screen-hit").history.last().unwrap();
    assert_eq!(
        (hit.result, hit.source),
        (OutcomeWord::Hit, OutcomeSource::Screen)
    );
    assert!(guide("craft-screen-unexpected").history.is_empty());
    assert!(line("craft-screen-unexpected").warning);
    assert!(line("craft-screen-unexpected")
        .text
        .contains("did not expect"));
    assert!(line("craft-watch-off")
        .text
        .starts_with("Screen reading is off"));
    assert!(line("craft-watch-unavailable").warning);
    assert!(line("craft-watch-unavailable")
        .text
        .contains("(no English OCR)"));
    // Without a backend the guide says the screen is not read; replan and
    // done carry no line; the overlay's line is the guiding view's.
    assert!(line("craft-guiding")
        .text
        .starts_with("Screen reading is not available"));
    assert_eq!(guide("craft-replan").watch, None);
    assert_eq!(guide("craft-done").watch, None);
    for (name, v) in &views {
        if let (Some(g), Some(o)) = (&v.craft.guiding, &v.overlay.guide) {
            assert_eq!(g.watch, o.watch, "{name}");
        }
    }
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
        dropped["overlay"]
            .as_object_mut()
            .unwrap()
            .remove("visible");
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
    assert_eq!(settings.len(), 14);
    assert_eq!(views.len(), 22);
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
