//! The overlay over the game, through `App` with a recording fake shell and
//! a fake screen backend: hidden until a craft is guiding and the game's
//! window is in front, filled from the current step and the last outcome,
//! placed at the chosen position inside the game's client area, and the
//! watch thread that tracks the game's window, which calls nothing while no
//! craft is guiding and ends with the app.

mod common;

use std::path::Path;
use std::sync::Arc;

use ch_app::{
    overlay_origin, App, ClientRect, GameWindow, GuideStatusView, OutcomeWord, OverlayPosition,
    WatchFailure, OVERLAY_INSET,
};
use common::*;
use serde_json::json;

/// A started app with `screen` (manual ticks), planning the ring's start
/// with its three targets on the whittling route.
fn planned(dir: &Path, shell: &FakeShell, screen: &FakeScreen) -> Arc<App> {
    let app = start_with_screen(dir, shell, screen, true);
    app.grab_text(Some(&ring("01-start")));
    app.set_targets(&ring_targets());
    app.choose_route(WHITTLE);
    app
}

fn overlay_lines(dir: &Path) -> Vec<String> {
    diag_lines(dir)
        .into_iter()
        .filter(|l| l.starts_with("overlay") || l.starts_with("watch:"))
        .collect()
}

#[test]
fn overlay_hidden_until_a_craft_is_active() {
    // Without a screen backend (M5's shape) a craft never shows it.
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);
    app.grab_text(Some(&ring("01-start")));
    app.set_targets(&ring_targets());
    app.choose_route(WHITTLE);
    let view = app.accept_craft();
    assert!(view.overlay.guide.is_some(), "the content is there");
    assert!(!view.overlay.visible);
    app.tick_watch();
    app.grab_text(Some(&ring("02-chaos-fire")));
    assert!(!app.get_view().overlay.visible);
    assert!(shell.rec().overlay.is_empty(), "{:?}", shell.rec().overlay);
    assert!(shell.rec().placed.is_empty());
    app.shutdown();
    assert!(overlay_lines(tmp.path()).is_empty());

    // With the game in front, the start and every command but a craft
    // leave it hidden, and the shell is never asked to show it.
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::with_clipboard(ITEM_TEXT);
    let screen = FakeScreen::in_front();
    let app = start_with_screen(tmp.path(), &shell, &screen, true);
    let view = app.get_view();
    assert!(!view.overlay.visible);
    assert!(view.overlay.guide.is_none());
    app.hotkey_pressed();
    let _ = app.grab_text(Some(&ring("01-start")));
    let _ = app.set_targets(&ring_targets());
    let _ = app.set_hotkey("Ctrl+Alt+R");
    let _ = app.update_settings(&json!({ "blocked_currencies": ["divine-orb"] }));
    let _ = app.update_settings(&json!({ "overlay_position": "bottom-left" }));
    let _ = app.check_for_updates();
    app.open_settings().unwrap();
    app.tick_watch();
    assert!(!app.get_view().overlay.visible);
    assert!(shell.rec().views.iter().all(|v| !v.overlay.visible));
    assert!(shell.rec().overlay.is_empty(), "{:?}", shell.rec().overlay);

    // Target: a craft in the guide shows it, through the shell and in the
    // view, once the watch has seen the game in front.
    app.accept_craft();
    app.tick_watch();
    assert!(app.get_view().overlay.visible);
    assert!(shell.last_view().overlay.visible);
    assert_eq!(shell.rec().overlay, vec![true]);
    // Another tick changes nothing.
    app.tick_watch();
    assert_eq!(shell.rec().overlay, vec![true]);

    // The craft ends: hidden at once, with no tick.
    app.end_craft();
    assert!(!app.get_view().overlay.visible);
    assert!(!shell.last_view().overlay.visible);
    assert!(app.get_view().overlay.guide.is_none());
    assert_eq!(shell.rec().overlay, vec![true, false]);
    app.shutdown();
    let log = overlay_lines(tmp.path());
    assert!(log.contains(&"overlay: shown".to_string()), "{log:?}");
    assert!(
        log.contains(&"overlay: hidden (not guiding)".to_string()),
        "{log:?}"
    );
    assert!(
        log.contains(&"watch: idle (not guiding)".to_string()),
        "{log:?}"
    );
    assert!(!diag_lines(tmp.path())
        .iter()
        .any(|l| l.contains("a craft is active")));
}

#[test]
fn overlay_shows_the_step_while_guiding_with_the_game_in_front() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let screen = FakeScreen::in_front();
    let app = planned(tmp.path(), &shell, &screen);
    let view = app.accept_craft();
    // Not before the watch has looked at the game's window.
    assert!(!view.overlay.visible);
    app.tick_watch();
    let view = app.get_view();
    assert!(view.overlay.visible);
    assert_eq!(view.overlay.position, OverlayPosition::TopCenter);
    let guide = view.overlay.guide.as_ref().unwrap();
    let step = view
        .craft
        .guiding
        .as_ref()
        .unwrap()
        .step
        .as_ref()
        .unwrap()
        .clone();
    assert_eq!(guide.status, GuideStatusView::Active);
    assert_eq!(guide.route_name, "Rings: resistances by whittling");
    assert_eq!(guide.step_number, Some(1));
    assert_eq!(guide.instruction, step.step.instruction);
    // Placed before it was shown, inside the game's client area.
    assert_eq!(shell.rec().placed.len(), 1);
    assert_eq!(shell.rec().overlay, vec![true]);
    assert_eq!(shell.last_view(), view);

    // Every guide status keeps it shown: a miss, a hit, then done.
    for file in [
        "06-chaos-below-floor",
        "02-chaos-fire",
        "03-whittle-cold",
        "04-desecrated",
        "05-revealed-lightning",
    ] {
        app.grab_text(Some(&ring(file)));
        app.tick_watch();
        assert!(app.get_view().overlay.visible, "{file}");
    }
    assert_eq!(
        app.get_view().overlay.guide.unwrap().status,
        GuideStatusView::Done
    );
    assert_eq!(shell.rec().overlay, vec![true]);
    assert_eq!(shell.rec().placed.len(), 1, "never placed per tick");
    app.shutdown();
    // The overlay's lines once each; the watch's read lines (English OCR,
    // no tooltip, then idle once the guide is done) said once each too.
    let log = overlay_lines(tmp.path());
    assert_eq!(
        log,
        [
            "watch: game window 2560x1440",
            "overlay: placed top-center",
            "overlay: shown",
            "watch: on (OCR en-US)",
            "watch: tooltip none",
            "watch: idle (guide not active)",
        ],
        "{log:?}"
    );
}

#[test]
fn overlay_hides_when_the_game_is_not_in_front() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let screen = FakeScreen::default();
    let app = planned(tmp.path(), &shell, &screen);
    app.accept_craft();

    // Baseline: guiding with another window in front.
    app.tick_watch();
    assert_eq!(screen.calls(), 1);
    assert!(!app.get_view().overlay.visible);
    assert!(shell.rec().overlay.is_empty());

    // The game comes to the front, then another window does.
    screen.set(in_front(CLIENT));
    app.tick_watch();
    assert!(app.get_view().overlay.visible);
    screen.set(Ok(GameWindow::NotInFront));
    app.tick_watch();
    assert!(!app.get_view().overlay.visible);
    assert!(!shell.last_view().overlay.visible);
    assert_eq!(shell.rec().overlay, vec![true, false]);

    // Minimized: still hidden, said once.
    screen.set(Ok(GameWindow::Minimized));
    app.tick_watch();
    app.tick_watch();
    assert!(!app.get_view().overlay.visible);

    // Back in front: shown again, not placed again (nothing moved).
    screen.set(in_front(CLIENT));
    app.tick_watch();
    assert!(app.get_view().overlay.visible);
    assert_eq!(shell.rec().overlay, vec![true, false, true]);
    assert_eq!(shell.rec().placed.len(), 1);

    // A failed call hides it and is logged once until a call succeeds.
    let failed = || {
        Err(WatchFailure {
            call: "GetForegroundWindow".to_string(),
            error: "Access is denied. (os error 5)".to_string(),
        })
    };
    screen.set(failed());
    app.tick_watch();
    app.tick_watch();
    assert!(!app.get_view().overlay.visible);
    screen.set(in_front(CLIENT));
    app.tick_watch();
    screen.set(failed());
    app.tick_watch();
    assert_eq!(
        shell.rec().overlay,
        vec![true, false, true, false, true, false]
    );
    app.shutdown();

    let log = overlay_lines(tmp.path());
    let error = "error: watch: GetForegroundWindow failed: Access is denied. (os error 5)";
    assert_eq!(
        log.iter()
            .filter(|l| l.as_str() == "watch: idle (minimized)")
            .count(),
        1,
        "{log:?}"
    );
    assert_eq!(
        log.iter()
            .filter(|l| l.as_str() == "overlay: hidden (game not in front)")
            .count(),
        3,
        "{log:?}"
    );
    let all = diag_lines(tmp.path());
    assert_eq!(
        all.iter().filter(|l| l.as_str() == error).count(),
        2,
        "{all:?}"
    );
    assert!(log.contains(&"watch: idle (game not in front)".to_string()));
    assert!(log.contains(&"watch: game window 2560x1440".to_string()));
}

#[test]
fn overlay_lines_follow_the_step_and_the_last_outcome() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let screen = FakeScreen::in_front();
    let app = planned(tmp.path(), &shell, &screen);
    app.accept_craft();
    app.tick_watch();

    // Step 1, first try: what the guiding view says, no outcome yet.
    let view = app.get_view();
    let guide = view.overlay.guide.clone().unwrap();
    let step = view.craft.guiding.as_ref().unwrap().step.clone().unwrap();
    assert_eq!(guide.step_number, Some(1));
    assert_eq!(guide.try_number, Some(1));
    assert_eq!(
        guide.instruction,
        "Chaos Orb until one of the targets lands"
    );
    assert_eq!(guide.uses, ["Chaos Orb"]);
    assert_eq!(guide.odds, step.step.odds);
    assert!(guide.odds.as_deref().unwrap().starts_with('~'));
    assert_eq!(guide.on_hit.as_deref(), Some(step.on_hit.as_str()));
    assert_eq!(guide.on_miss.as_deref(), Some("On a miss: this step again"));
    assert_eq!(guide.why, None);
    assert_eq!(guide.last, None);

    // A miss: the same step, the second try, the miss as the last outcome.
    let view = app.grab_text(Some(&ring("06-chaos-below-floor")));
    let guide = view.overlay.guide.clone().unwrap();
    assert_eq!((guide.step_number, guide.try_number), (Some(1), Some(2)));
    let last = guide.last.unwrap();
    assert_eq!(last.result, OutcomeWord::Miss);
    assert_eq!(
        Some(&last),
        view.craft.guiding.as_ref().unwrap().history.last()
    );
    // The view the windows heard is the same.
    assert_eq!(shell.last_view().overlay, view.overlay);

    // A hit: the next step, its first try, the hit as the last outcome.
    let view = app.grab_text(Some(&ring("02-chaos-fire")));
    let guide = view.overlay.guide.clone().unwrap();
    assert_eq!((guide.step_number, guide.try_number), (Some(2), Some(1)));
    let last = guide.last.unwrap();
    assert_eq!(last.result, OutcomeWord::Hit);
    assert_eq!(last.summary, "Hit: gained +#% to Fire Resistance (T3)");
    assert_eq!(
        guide.on_hit.as_deref(),
        Some("On a hit: this step again until its condition holds, then Desecrate for the last target")
    );
    app.shutdown();

    // Replan: says so, with the first reason, and no step or branches.
    let tmp = tempfile::tempdir().unwrap();
    let app = planned(tmp.path(), &shell, &screen);
    app.accept_craft();
    let view = app.grab_text(Some(&ring("03-whittle-cold")));
    let guiding = view.craft.guiding.as_ref().unwrap();
    assert_eq!(guiding.status, GuideStatusView::Replan);
    let guide = view.overlay.guide.clone().unwrap();
    assert_eq!(guide.status, GuideStatusView::Replan);
    assert_eq!(
        guide.instruction,
        "End the craft in the app to plan again from this item."
    );
    assert_eq!(guide.why.as_ref(), guiding.reasons.first());
    assert!(guide.why.is_some());
    assert_eq!((guide.step_number, guide.try_number), (None, None));
    assert!(guide.on_hit.is_none() && guide.on_miss.is_none() && guide.uses.is_empty());
    assert_eq!(guide.last.unwrap().result, OutcomeWord::Unexpected);
    app.shutdown();

    // Done: says so, with the last hit.
    let tmp = tempfile::tempdir().unwrap();
    let app = planned(tmp.path(), &shell, &screen);
    app.accept_craft();
    for file in [
        "02-chaos-fire",
        "03-whittle-cold",
        "04-desecrated",
        "05-revealed-lightning",
    ] {
        app.grab_text(Some(&ring(file)));
    }
    let guide = app.get_view().overlay.guide.unwrap();
    assert_eq!(guide.status, GuideStatusView::Done);
    assert_eq!(guide.instruction, "The craft is done.");
    assert_eq!(guide.step_number, None);
    assert!(guide.on_hit.is_none() && guide.odds.is_none() && guide.why.is_none());
    assert_eq!(
        guide.last.unwrap().summary,
        "Hit: gained +#% to Lightning Resistance (T3)"
    );
    app.shutdown();
}

#[test]
fn overlay_is_placed_at_the_chosen_position() {
    // The math: each position inside the client, inset by OVERLAY_INSET.
    let client = ClientRect {
        left: 100,
        top: 50,
        right: 2020,
        bottom: 1130,
    };
    let size = (400, 150);
    let i = OVERLAY_INSET as i32;
    assert_eq!(OVERLAY_INSET, 12);
    let want = [
        (OverlayPosition::TopLeft, (100 + i, 50 + i)),
        (OverlayPosition::TopCenter, (100 + (1920 - 400) / 2, 50 + i)),
        (OverlayPosition::TopRight, (2020 - 400 - i, 50 + i)),
        (OverlayPosition::BottomLeft, (100 + i, 1130 - 150 - i)),
        (
            OverlayPosition::BottomCenter,
            (100 + (1920 - 400) / 2, 1130 - 150 - i),
        ),
        (
            OverlayPosition::BottomRight,
            (2020 - 400 - i, 1130 - 150 - i),
        ),
    ];
    for (position, at) in want {
        assert_eq!(overlay_origin(position, client, size), at, "{position:?}");
    }
    // A client too small for the overlay and its insets: its top-left.
    let small = ClientRect {
        left: -8,
        top: 20,
        right: 400,
        bottom: 600,
    };
    for (position, _) in want {
        assert_eq!(overlay_origin(position, small, size), (-8, 20));
    }

    // Through the app: the default, then a change while shown, then the
    // game's window moved; a refused position changes nothing.
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let screen = FakeScreen::in_front();
    let app = planned(tmp.path(), &shell, &screen);
    app.accept_craft();
    app.tick_watch();
    let (w, h) = OVERLAY_SIZE;
    let at = |p| overlay_origin(p, CLIENT, (w, h));
    assert_eq!(shell.rec().placed, [at(OverlayPosition::TopCenter)]);
    assert_eq!(at(OverlayPosition::TopCenter), ((2560 - 420) / 2, 12));

    let settings = app
        .update_settings(&json!({ "overlay_position": "bottom-right" }))
        .unwrap();
    assert_eq!(settings.overlay_position, OverlayPosition::BottomRight);
    assert_eq!(
        shell.rec().placed,
        [
            at(OverlayPosition::TopCenter),
            at(OverlayPosition::BottomRight)
        ]
    );
    assert_eq!(
        at(OverlayPosition::BottomRight),
        (2560 - 420 - 12, 1440 - 160 - 12)
    );
    assert_eq!(
        app.get_view().overlay.position,
        OverlayPosition::BottomRight
    );
    // The same position again places nothing.
    app.update_settings(&json!({ "overlay_position": "bottom-right" }))
        .unwrap();
    app.tick_watch();
    assert_eq!(shell.rec().placed.len(), 2);

    // An unknown position is refused whole, with nothing changed.
    for bad in [
        json!({ "overlay_position": "middle" }),
        json!({ "overlay_position": 3 }),
        json!({ "overlay_position": "top-left", "screen_watch": "no" }),
    ] {
        assert!(app.update_settings(&bad).is_err(), "{bad}");
    }
    assert_eq!(
        app.get_app_settings().overlay_position,
        OverlayPosition::BottomRight
    );
    assert_eq!(shell.rec().placed.len(), 2);

    // The game's window moved: placed again, from the new client area.
    let moved = ClientRect {
        left: 1920,
        top: 0,
        right: 3840,
        bottom: 1080,
    };
    screen.set(in_front(moved));
    app.tick_watch();
    assert_eq!(
        shell.rec().placed.last(),
        Some(&overlay_origin(OverlayPosition::BottomRight, moved, (w, h)))
    );
    assert_eq!(shell.rec().placed.len(), 3);

    // A change while hidden is placed when it shows again.
    screen.set(Ok(GameWindow::NotInFront));
    app.tick_watch();
    app.update_settings(&json!({ "overlay_position": "top-left" }))
        .unwrap();
    assert_eq!(shell.rec().placed.len(), 3);
    screen.set(in_front(moved));
    app.tick_watch();
    assert_eq!(
        shell.rec().placed.last(),
        Some(&overlay_origin(OverlayPosition::TopLeft, moved, (w, h)))
    );
    app.shutdown();
    let log = overlay_lines(tmp.path());
    for line in [
        "overlay: placed top-center",
        "overlay: placed bottom-right",
        "overlay: placed top-left",
        "watch: game window 1920x1080",
    ] {
        assert!(log.contains(&line.to_string()), "{line}: {log:?}");
    }
    assert!(diag_lines(tmp.path())
        .iter()
        .any(|l| l.starts_with("settings refused: ")));
}

#[test]
fn no_backend_call_unless_guiding() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::with_clipboard(ITEM_TEXT);
    let screen = FakeScreen::in_front();
    let app = start_with_screen(tmp.path(), &shell, &screen, true);

    // Baseline: the start, every command and every tick while no craft is
    // guiding call nothing.
    app.tick_watch();
    app.hotkey_pressed();
    app.grab_text(Some(&ring("01-start")));
    app.tick_watch();
    app.set_targets(&ring_targets());
    app.choose_route(WHITTLE);
    app.update_settings(&json!({ "screen_watch": false }))
        .unwrap();
    app.update_settings(&json!({ "screen_watch": true }))
        .unwrap();
    app.tick_watch();
    assert_eq!(screen.calls(), 0, "a backend call while not guiding");

    // Positive control: guiding, each tick looks at the game's window once,
    // in any guide status.
    app.accept_craft();
    app.tick_watch();
    assert_eq!(screen.calls(), 1);
    app.grab_text(Some(&ring("03-whittle-cold")));
    assert_eq!(
        app.get_view().craft.guiding.unwrap().status,
        GuideStatusView::Replan
    );
    app.tick_watch();
    assert_eq!(screen.calls(), 2);

    // The craft ends: nothing again.
    app.end_craft();
    app.tick_watch();
    app.tick_watch();
    assert_eq!(screen.calls(), 2);
    app.shutdown();

    // The thread's own clock: it looks while guiding, and stops looking
    // when the craft ends.
    let tmp = tempfile::tempdir().unwrap();
    let screen = FakeScreen::in_front();
    let app = start_with_screen(tmp.path(), &shell, &screen, false);
    app.grab_text(Some(&ring("01-start")));
    app.set_targets(&ring_targets());
    app.choose_route(WHITTLE);
    assert_eq!(screen.calls(), 0);
    app.accept_craft();
    wait_until("the overlay shows", || app.get_view().overlay.visible);
    wait_until("a second look", || screen.calls() >= 2);
    app.end_craft();
    assert!(!app.get_view().overlay.visible);
    app.shutdown();
}

#[test]
fn shutdown_stops_the_watch() {
    // The thread ends with the app: it drops its backend.
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let screen = FakeScreen::in_front();
    let app = start_with_screen(tmp.path(), &shell, &screen, false);
    app.grab_text(Some(&ring("01-start")));
    app.set_targets(&ring_targets());
    app.choose_route(WHITTLE);
    app.accept_craft();
    wait_until("a look at the game", || screen.calls() >= 1);
    assert!(!screen.dropped());
    app.shutdown();
    assert!(screen.dropped(), "the watch thread did not end");
    let calls = screen.calls();
    app.tick_watch();
    app.accept_craft();
    assert_eq!(screen.calls(), calls);
    // Twice is safe.
    app.shutdown();
    assert!(!diag_lines(tmp.path())
        .iter()
        .any(|l| l.contains("did not end")));

    // A call in flight is not interrupted: past the deadline the shutdown
    // says so and returns; the thread ends when the call does, and its
    // report changes nothing.
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let screen = FakeScreen::in_front();
    screen.hold(true);
    let app = start_with_screen(tmp.path(), &shell, &screen, false);
    app.grab_text(Some(&ring("01-start")));
    app.set_targets(&ring_targets());
    app.choose_route(WHITTLE);
    app.accept_craft();
    wait_until("a call in flight", || screen.rec().in_call);
    app.shutdown();
    assert!(!screen.dropped());
    let log = diag_lines(tmp.path());
    assert!(
        log.contains(&"error: watch: did not end within 2000 ms".to_string()),
        "{log:?}"
    );
    screen.hold(false);
    wait_until("the thread to end", || screen.dropped());
    assert!(!app.get_view().overlay.visible);
    assert!(shell.rec().overlay.is_empty());
}
