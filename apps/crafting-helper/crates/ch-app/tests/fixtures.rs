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
    OutcomeSource, OutcomeWord, OverlayPosition, SettingsView, StatusKind, TradePickChoice,
    TradeSelection, TradeWeightSource, Version, View, DEFAULT_TRADE_LEAGUE, SHIPPED_TRADE_LEAGUES,
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

    // The Trade section: a league other than the default, the shipped list;
    // a refresh running (held in flight); a refresh refused (the site
    // answered 503), the list and the league kept.
    let h = harness(Setup::default());
    h.app.start();
    out.push((
        "trade-league",
        h.app
            .update_settings(&json!({ "trade_league": "Runes of Aldur" }))
            .unwrap(),
    ));
    h.app.shutdown();
    let shell = FakeShell::default();
    shell.answer_leagues(Ok(site_leagues()));
    shell.hold_leagues(true);
    let h = harness(Setup {
        shell: Some(shell.clone()),
        ..Setup::default()
    });
    h.app.start();
    out.push(("trade-league-refreshing", h.app.refresh_trade_leagues()));
    shell.hold_leagues(false);
    wait_refreshed(&h.app, &shell);
    h.app.shutdown();
    let shell = FakeShell::default();
    let mut answer = site_leagues();
    answer.status = 503;
    shell.answer_leagues(Ok(answer));
    let h = harness(Setup {
        shell: Some(shell.clone()),
        ..Setup::default()
    });
    h.app.start();
    h.app.refresh_trade_leagues();
    out.push((
        "trade-league-refresh-refused",
        wait_refreshed(&h.app, &shell),
    ));
    h.app.shutdown();
    out
}

/// The trade panel's selection, open, as the page sends it.
fn trade_selection(
    class: &str,
    base: &str,
    item_level: Option<u32>,
    picks: &[&str],
) -> Vec<TradeSelection> {
    let class_only = TradeSelection {
        open: true,
        class: Some(class.to_string()),
        ..TradeSelection::default()
    };
    let with_base = TradeSelection {
        base: Some(base.to_string()),
        item_level,
        ..class_only.clone()
    };
    let with_picks = TradeSelection {
        picks: picks
            .iter()
            .map(|id| TradePickChoice {
                mod_id: id.to_string(),
                floor: None,
            })
            .collect(),
        ..with_base.clone()
    };
    vec![class_only, with_base, with_picks]
}

/// The magic Heavy Belt `ch-engine`'s relaxation tests read
/// (`crates/ch-engine/tests/items/relax/belt-magic-life.txt`, constructed):
/// maximum Life T2, no suffix.
fn relax_belt() -> String {
    let path = app_root().join("crates/ch-engine/tests/items/relax/belt-magic-life.txt");
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
        .replace("\r\n", "\n")
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
        app.set_craft_blocked(&lines(&["chaos-orb", "orb-of-annulment"])),
    ));
    app.shutdown();
    // No route for another reason: the magic belt's three resistances at
    // T2, one tier past its Greater essences, so the preview proposes
    // lowering fire to T3.
    let shell = FakeShell::default();
    let app = start(&tmp.path().join("relaxation"), &shell);
    app.grab_text(Some(&relax_belt()));
    let resistances: Vec<_> = RING_TARGETS
        .iter()
        .map(|id| ch_app::ch_engine::Target::new(*id, Some(2)))
        .collect();
    out.push(("craft-relaxation", app.set_targets(&resistances)));
    app.shutdown();
    // The relaxation accepted (fire at T3) on the route whose reveal comes
    // before the suffix exalt: the reveal step names the blocker.
    let shell = FakeShell::default();
    let app = start(&tmp.path().join("blocker"), &shell);
    app.grab_text(Some(&relax_belt()));
    let mut relaxed = resistances.clone();
    relaxed[0].floor = Some(3);
    app.set_targets(&relaxed);
    out.push((
        "craft-blocker",
        app.choose_route("essence-desecrate-double-slam"),
    ));
    app.shutdown();
    // The fractured essence swap on `ch-engine`'s ring (copied into
    // `tests/items/`): the essence step names what it removes and the
    // fractured suffix it leaves, and its survival line the unrevealed
    // suffix's chance and the step on each outcome.
    let shell = FakeShell::default();
    let app = start(&tmp.path().join("essence-swap"), &shell);
    app.grab_text(Some(&belt("ring-essence-swap")));
    app.set_targets(&[
        ch_app::ch_engine::Target::new("prefix-to-maximum-mana", Some(5)),
        ch_app::ch_engine::Target::new("suffix-increased-mana-regeneration-rate", Some(2)),
        ch_app::ch_engine::Target::new("desecrated-suffix-to-cold-and-chaos-resistances", None),
    ]);
    out.push((
        "craft-essence-swap",
        app.choose_route("fractured-essence-swap"),
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

    // The owner's 2P/1S belt on the state-ordered `close-a-side`: planned
    // (the suffix side closed first, with Greater and Dextral Exaltation);
    // then started, the suffix side closed and the slam on the open prefix
    // side missed: on `remove`, the engine's annulment kept to the prefix
    // side (mana, a target, is the lowest, so a whittle would take it).
    // Omen of Sinistral Necromancy is blocked for the craft, so the last
    // prefix is slammed rather than desecrated (each planned step's
    // Fallback line says so): the close and the clean-up are these views'
    // subject.
    let shell = FakeShell::default();
    let app = start(&tmp.path().join("close-side"), &shell);
    app.grab_text(Some(&belt("belt-2p1s")));
    app.set_targets(&belt_targets());
    app.set_craft_blocked(&lines(&["omen-of-sinistral-necromancy"]));
    out.push(("craft-close-side", app.choose_route(CLOSE)));
    app.accept_craft();
    app.grab_text(Some(&belt("belt-closed")));
    out.push((
        "craft-miss-clean",
        app.grab_text(Some(&belt("belt-slam-miss"))),
    ));
    app.shutdown();
    // The same belt planned with nothing blocked: Armour, the last target
    // missing on a rare with no desecrated modifier, is desecrated on the
    // prefix side and revealed (issue #32).
    let shell = FakeShell::default();
    let app = start(&tmp.path().join("last-desecration"), &shell);
    app.grab_text(Some(&belt("belt-2p1s")));
    app.set_targets(&belt_targets());
    out.push(("craft-last-desecration", app.choose_route(CLOSE)));
    app.shutdown();
    // The belt's prefix side full: Life and Mana on it, Armour chosen
    // beyond them; every other prefix choice is disabled with the reason.
    let shell = FakeShell::default();
    let app = start(&tmp.path().join("side-full"), &shell);
    app.grab_text(Some(&belt("belt-2p1s")));
    out.push(("craft-side-full", app.set_targets(&belt_targets())));
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

    // The trade panel, opened over the planning ring craft.
    let trade = |dir: &str, steps: &[TradeSelection]| {
        let app = planned(dir);
        let mut view = app.get_view();
        for sel in steps {
            view = app.set_trade_search(sel);
        }
        (app, view)
    };
    let (app, view) = trade(
        "trade-empty",
        &[TradeSelection {
            open: true,
            ..TradeSelection::default()
        }],
    );
    out.push(("trade-empty", view));
    app.shutdown();
    let picking = trade_selection(
        "Rings",
        "Sapphire Ring",
        Some(75),
        &["prefix-to-maximum-life", "suffix-to-fire-resistance"],
    );
    let (app, view) = trade("trade-picking", &picking);
    out.push(("trade-picking", view));
    // A base the class does not have: refused, the selection kept.
    let mut wrong = picking[2].clone();
    wrong.base = Some("Iron Mitts".to_string());
    out.push(("trade-refused", app.set_trade_search(&wrong)));
    app.shutdown();
    let (app, view) = trade(
        "trade-one-side",
        &trade_selection(
            "Rings",
            "Sapphire Ring",
            Some(75),
            &["suffix-to-fire-resistance", "suffix-to-cold-resistance"],
        ),
    );
    out.push(("trade-one-side", view));
    app.shutdown();
    // A jewel: no spawn weights, so every weight is the default.
    let jewel = trade_selection("Jewels", "Sapphire", None, &[]);
    let (app, view) = trade("trade-default-weight", &jewel);
    let first = |rows: &[ch_app::TradeModView]| rows[0].mod_id.clone();
    let mut sel = jewel[2].clone();
    sel.picks = [first(&view.trade.prefixes), first(&view.trade.suffixes)]
        .iter()
        .map(|id| TradePickChoice {
            mod_id: id.clone(),
            floor: None,
        })
        .collect();
    out.push(("trade-default-weight", app.set_trade_search(&sel)));
    app.shutdown();
    // A floor picked with the item level blank: the view suggests the level
    // the floor tier needs, and the search uses it.
    let mut floor = trade_selection(
        "Boots",
        "Bangled Sandals",
        None,
        &["prefix-increased-movement-speed"],
    );
    floor[2].picks[0].floor = Some(2);
    let (app, view) = trade("trade-floor", &floor);
    out.push(("trade-floor", view));
    app.shutdown();
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
            "hotkey-refused"
                | "blocked-currencies"
                | "overlay-settings"
                | "trade-league"
                | "trade-league-refreshing"
                | "trade-league-refresh-refused"
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
    // The Trade section: the default league and the shipped list everywhere
    // but its own states.
    assert_eq!(by("trade-league").trade_league, "Runes of Aldur");
    assert_eq!(by("trade-league").trade_leagues, SHIPPED_TRADE_LEAGUES);
    assert!(by("trade-league").trade_league_note.is_none());
    assert!(by("trade-league-refreshing").trade_leagues_refreshing);
    let refused = by("trade-league-refresh-refused");
    assert!(!refused.trade_leagues_refreshing);
    assert_eq!(refused.trade_leagues, SHIPPED_TRADE_LEAGUES);
    assert_eq!(
        refused.trade_league_note.as_ref().unwrap().kind,
        StatusKind::Error
    );
    for (name, v) in &settings {
        if *name != "trade-league" {
            assert_eq!(v.trade_league, DEFAULT_TRADE_LEAGUE, "{name}");
            assert_eq!(v.trade_leagues, SHIPPED_TRADE_LEAGUES, "{name}");
        }
        if !name.starts_with("trade-league-") {
            assert!(
                !v.trade_leagues_refreshing && v.trade_league_note.is_none(),
                "{name}"
            );
        }
    }

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
    assert!(preview("craft-no-route").relaxation.is_none());
    // No route on the magic belt's T2 resistances, and the one floor to
    // lower in its place.
    assert!(preview("craft-relaxation").route.is_none());
    assert!(!plan("craft-relaxation").can_accept);
    let relax = preview("craft-relaxation").relaxation.as_ref().unwrap();
    assert!(relax
        .text
        .starts_with("Lower +#% to Fire Resistance from T2 to T3"));
    assert_eq!((relax.from, relax.to), (2, 3));
    // The reveal names the blocker; no other step of any route does.
    let blocker = preview("craft-blocker").route.as_ref().unwrap();
    assert_eq!(blocker.blueprint, "essence-desecrate-double-slam");
    for (name, v) in &views {
        let steps = v
            .craft
            .planning
            .as_ref()
            .and_then(|p| p.preview.as_ref())
            .and_then(|p| p.route.as_ref())
            .map(|r| r.steps.clone())
            .unwrap_or_default();
        for s in steps {
            let named = *name == "craft-blocker" && s.id == "reveal";
            assert_eq!(s.blocker.is_some(), named, "{name} {}", s.id);
            let swap = *name == "craft-essence-swap" && s.id == "essence";
            assert_eq!(s.survival.is_some(), swap, "{name} {}", s.id);
        }
    }
    assert!(blocker.steps[2]
        .blocker
        .as_deref()
        .unwrap()
        .starts_with("If +#% to Cold Resistance is not offered, take +# to Strength and Dexterity or +# to Strength and Intelligence (keeps out +# to Strength)"));
    assert_eq!(view("craft-refused").craft.mode, CraftMode::Refused);
    let guide = |n: &str| view(n).craft.guiding.as_ref().unwrap();
    assert_eq!(guide("craft-guiding").status, GuideStatusView::Active);
    assert_eq!(guide("craft-guiding").history.len(), 1);
    assert_eq!(guide("craft-miss").history[0].result, OutcomeWord::Miss);
    assert_eq!(guide("craft-miss").step.as_ref().unwrap().try_number, 2);
    let clean = guide("craft-miss-clean").step.as_ref().unwrap();
    assert_eq!(clean.step.id, "remove");
    assert_eq!(
        clean.step.uses,
        ["Orb of Annulment", "Omen of Sinistral Annulment"]
    );
    assert!(clean.step.fallback.is_none());
    let close = preview("craft-close-side").route.as_ref().unwrap();
    assert_eq!(close.blueprint, CLOSE);
    assert_eq!(
        close.steps[0].uses,
        [
            "Exalted Orb",
            "Omen of Greater Exaltation",
            "Omen of Dextral Exaltation"
        ]
    );
    assert!(plan("craft-close-side").can_accept);
    // With the Necromancy omen blocked, each step says it was taken
    // instead of the desecration, and why.
    for s in &close.steps {
        let fallback = s.fallback.as_deref().unwrap_or_default();
        assert!(
            fallback.ends_with("instead of a desecration: Omen of Sinistral Necromancy blocked"),
            "{}: {fallback}",
            s.id
        );
    }
    // Nothing blocked: the route's last two steps are the desecration on
    // the prefix side and the reveal, odds unknown, no fallback line.
    let last = preview("craft-last-desecration").route.as_ref().unwrap();
    assert_eq!(last.blueprint, CLOSE);
    let ids: Vec<&str> = last.steps.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(ids, ["desecrate-last", "reveal-last"]);
    assert_eq!(last.steps[0].uses[1], "Omen of Sinistral Necromancy");
    assert_eq!(last.steps[1].uses[1], "Omen of Abyssal Echoes");
    for s in &last.steps {
        assert!(
            s.odds
                .as_deref()
                .is_some_and(|o| o.starts_with("odds unknown: ")),
            "{}: {:?}",
            s.id,
            s.odds
        );
        assert_eq!(s.fallback, None, "{}", s.id);
    }
    assert!(plan("craft-last-desecration").can_accept);
    // The belt's prefix side full: the chosen prefixes selectable, every
    // other prefix disabled with the reason, every suffix selectable.
    let full = plan("craft-side-full");
    let chosen = [
        "prefix-to-maximum-life",
        "prefix-to-maximum-mana",
        "prefix-to-armour",
    ];
    for c in &full.choices.prefixes {
        let want = (!chosen.contains(&c.mod_id.as_str()))
            .then_some("Prefixes full: 2 on the item + 1 chosen = 3 of 3");
        assert_eq!(c.disabled.as_deref(), want, "{}", c.mod_id);
    }
    assert!(full.choices.suffixes.iter().all(|c| c.disabled.is_none()));
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

    // The trade panel: closed in every view but its own, each of which is
    // the state it is named for; the craft underneath is the planning ring.
    for (name, v) in &views {
        assert_eq!(v.trade.selection.open, name.starts_with("trade-"), "{name}");
        if !v.trade.selection.open {
            assert!(
                v.trade.classes.is_empty() && v.trade.link.is_none(),
                "{name}"
            );
        }
    }
    let trade = |n: &str| &view(n).trade;
    for n in [
        "trade-empty",
        "trade-picking",
        "trade-one-side",
        "trade-default-weight",
        "trade-refused",
        "trade-floor",
    ] {
        assert_eq!(view(n).craft.mode, CraftMode::Planning, "{n}");
    }
    assert_eq!(trade("trade-empty").selection.class, None);
    assert!(!trade("trade-empty").can_search);
    assert_eq!(
        trade("trade-empty").why_not.as_ref().unwrap().text,
        "Choose an item class."
    );
    let picking = trade("trade-picking");
    assert!(picking.can_search && picking.link.is_some());
    assert_eq!(
        picking.count_line.as_deref(),
        Some("The search asks for two: one prefix and one suffix.")
    );
    assert!(!picking.not_searchable.is_empty());
    let one_side = trade("trade-one-side");
    assert!(one_side.link.is_some());
    assert_eq!(
        one_side.count_line.as_deref(),
        Some("Your picks are all suffixes and a magic item holds one, so the search asks for one.")
    );
    let jewel = trade("trade-default-weight");
    assert!(jewel
        .prefixes
        .iter()
        .chain(&jewel.suffixes)
        .all(|r| r.weight_source == TradeWeightSource::Default && r.default_reason.is_some()));
    assert_eq!(jewel.selection.picks.len(), 2);
    // Only a floor picked suggests an item level; the blank field searches
    // with it.
    let floor = trade("trade-floor");
    assert_eq!(floor.selection.item_level, None);
    assert_eq!(floor.selection.picks[0].floor, Some(2));
    assert_eq!(floor.suggested_item_level, Some(65));
    assert!(floor.can_search);
    let floor_link = floor.link.as_deref().unwrap();
    // `"ilvl":{"min":65}`, percent-encoded in the link.
    assert!(
        floor_link.contains("%22ilvl%22%3A%7B%22min%22%3A65%7D"),
        "{floor_link}"
    );
    for (name, v) in &views {
        if *name != "trade-floor" {
            assert_eq!(v.trade.suggested_item_level, None, "{name}");
        }
    }
    let refused_trade = trade("trade-refused");
    assert_eq!(refused_trade.selection, picking.selection);
    assert_eq!(refused_trade.link, picking.link);
    assert_eq!(
        view("trade-refused").status.as_ref().unwrap().kind,
        StatusKind::Error
    );

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
        let mut extra = value.clone();
        extra["trade"]["selection"]["extra"] = Value::Bool(true);
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
    assert_eq!(settings.len(), 17);
    assert_eq!(views.len(), 34);
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
