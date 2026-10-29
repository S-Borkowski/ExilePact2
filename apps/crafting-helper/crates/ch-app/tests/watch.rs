//! Reading the item from the screen while guiding, through `App` with a
//! recording fake shell, a fake screen backend whose tooltip read the test
//! sets (tooltips built from `ch-engine`'s whittled ring texts), and a
//! clock the test moves on, the watch ticked by hand: a change read twice
//! alike, at least `CONFIRM_AFTER` apart, is a hit or a miss kept as a
//! grab's would be, an unexpected one is not applied, anything else changes
//! nothing; a grab wins over a read; no backend call holds the app's lock;
//! no English OCR stops the reads with a reason.

mod common;

use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use ch_app::{
    App, GameWindow, GuideStatusView, OcrReadiness, OutcomeSource, OutcomeWord, ReadCounts,
    StatusKind, TooltipRead, View, WatchFailure, WatchLineView,
};
use common::*;
use serde_json::json;

/// A started app with `screen` (ticked by hand, on the returned clock),
/// guiding the ring's start with its three targets on the whittling route.
fn guiding(dir: &Path, shell: &FakeShell, screen: &FakeScreen) -> (Arc<App>, Arc<AtomicU64>) {
    let (app, ms) = start_watching(dir, shell, screen);
    app.grab_text(Some(&ring("01-start")));
    app.set_targets(&ring_targets());
    app.choose_route(WHITTLE);
    let view = app.accept_craft();
    assert_eq!(
        view.craft.mode,
        ch_app::CraftMode::Guiding,
        "{:?}",
        view.status
    );
    (app, ms)
}

fn later(ms: &AtomicU64, by: u64) {
    ms.fetch_add(by, Ordering::SeqCst);
}

fn guide(view: &View) -> &ch_app::GuidingView {
    view.craft.guiding.as_ref().expect("guiding")
}

/// The watch line the guiding view carries; the overlay's is the same.
fn watch(app: &App) -> WatchLineView {
    let view = app.get_view();
    let line = guide(&view).watch.clone().expect("a watch line");
    assert_eq!(
        view.overlay.guide.as_ref().unwrap().watch.as_ref(),
        Some(&line),
        "the overlay's watch line is the guiding view's"
    );
    line
}

/// The current step's id and its try.
fn step(app: &App) -> (String, u32) {
    let view = app.get_view();
    let s = guide(&view).step.clone().expect("a current step");
    (s.step.id, s.try_number)
}

fn watch_log(dir: &Path) -> Vec<String> {
    diag_lines(dir)
        .into_iter()
        .filter(|l| l.starts_with("watch:") || l.starts_with("craft:"))
        .collect()
}

fn count(log: &[String], line: &str) -> usize {
    log.iter().filter(|l| l.as_str() == line).count()
}

/// Reads the tooltip twice, `CONFIRM_AFTER` apart.
fn read_twice(app: &App, ms: &AtomicU64) {
    app.tick_watch();
    later(ms, 400);
    app.tick_watch();
}

#[test]
fn screen_watch_off_reads_nothing_and_a_grab_still_advances() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let screen = FakeScreen::in_front();
    screen.show(tooltip_of(&ring("02-chaos-fire")));
    let (app, ms) = start_watching(tmp.path(), &shell, &screen);
    app.update_settings(&json!({ "screen_watch": false }))
        .unwrap();
    app.grab_text(Some(&ring("01-start")));
    app.set_targets(&ring_targets());
    app.choose_route(WHITTLE);
    app.accept_craft();
    let (first, _) = step(&app);

    // The game's window is still looked at (the overlay), the tooltip never.
    for _ in 0..3 {
        app.tick_watch();
        later(&ms, 500);
    }
    assert_eq!(screen.calls(), 3);
    assert_eq!((screen.reads(), screen.ocr_checks()), (0, 0));
    assert!(app.get_view().overlay.visible);
    assert_eq!(
        watch(&app),
        WatchLineView {
            text: "Screen reading is off: grab the item after each step.".to_string(),
            warning: false
        }
    );
    assert!(guide(&app.get_view()).history.is_empty());

    // A grab drives the guide as in M5.
    let view = app.grab_text(Some(&ring("02-chaos-fire")));
    let last = guide(&view).history.last().unwrap().clone();
    assert_eq!(
        (last.result, last.source),
        (OutcomeWord::Hit, OutcomeSource::Grab)
    );
    assert_eq!(guide(&view).step.as_ref().unwrap().step.number, 2);

    // Positive control: turned on, the next tick reads.
    app.update_settings(&json!({ "screen_watch": true }))
        .unwrap();
    app.tick_watch();
    assert_eq!(screen.reads(), 1);
    app.shutdown();
    let log = watch_log(tmp.path());
    assert_eq!(count(&log, "watch: idle (screen watch off)"), 1, "{log:?}");
    assert!(log.contains(&format!("craft: hit at {first}")), "{log:?}");
    assert!(
        !log.iter().any(|l| l.ends_with("(from the screen)")),
        "{log:?}"
    );
}

#[test]
fn screen_hit_advances_after_two_equal_reads() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let screen = FakeScreen::in_front();
    screen.show(tooltip_of(&ring("02-chaos-fire")));
    let (app, ms) = guiding(tmp.path(), &shell, &screen);
    let (first, _) = step(&app);
    let before = guide(&app.get_view()).item.clone();

    // Read once: checking, nothing kept.
    app.tick_watch();
    assert_eq!(screen.reads(), 1);
    assert_eq!(
        screen.rec().read_areas,
        [(0x1234, CLIENT)],
        "the game's client area, from the look in the same tick"
    );
    assert_eq!(
        watch(&app).text,
        "Checking the change: keep hovering the item."
    );
    assert!(guide(&app.get_view()).history.is_empty());

    // Negative control: read again too soon, still nothing kept.
    later(&ms, 100);
    app.tick_watch();
    assert!(guide(&app.get_view()).history.is_empty());
    assert_eq!(step(&app), (first.clone(), 1));

    // Read again 400 ms after the first: the hit, kept from the screen.
    later(&ms, 300);
    app.tick_watch();
    let view = app.get_view();
    let g = guide(&view);
    assert_eq!(
        g.history.len(),
        1,
        "{:?} {:?}",
        view.status,
        watch_log(tmp.path())
    );
    let last = g.history[0].clone();
    assert_eq!(
        (last.result, last.source),
        (OutcomeWord::Hit, OutcomeSource::Screen)
    );
    assert_eq!(last.step, first);
    assert_eq!(last.summary, "gained +#% to Fire Resistance (T3)");
    assert_eq!(g.step.as_ref().unwrap().step.number, 2);
    assert_eq!(g.step.as_ref().unwrap().try_number, 1);
    assert_ne!(g.item, before, "the read item is the item now");
    assert!(g
        .item
        .modifiers
        .iter()
        .any(|m| m.lines.iter().any(|l| l.contains("Fire Resistance"))));
    assert_eq!(
        view.overlay.guide.as_ref().unwrap().last.as_ref(),
        Some(&last)
    );
    let status = view.status.unwrap();
    assert_eq!(status.kind, StatusKind::Info);
    assert_eq!(
        status.text,
        "Hit: gained +#% to Fire Resistance (T3). Read from the screen."
    );
    assert_eq!(shell.last_view(), app.get_view(), "the windows heard it");

    // The next read is the item now: unchanged, not another outcome.
    later(&ms, 500);
    app.tick_watch();
    assert_eq!(guide(&app.get_view()).history.len(), 1);
    assert_eq!(watch(&app).text, "Watching: the item has not changed yet.");

    // The next step's result grabbed, from the item the screen read (which
    // has no copied text): classified as M5 classifies a grab.
    let view = app.grab_text(Some(&ring("03-whittle-cold")));
    let g = guide(&view);
    assert_eq!(g.history.len(), 2);
    assert_eq!(
        (g.history[1].result, g.history[1].source),
        (OutcomeWord::Hit, OutcomeSource::Grab)
    );
    app.shutdown();
    let log = watch_log(tmp.path());
    let hit = format!("craft: hit at {first} (from the screen)");
    let at = |line: &str| log.iter().position(|l| l == line);
    assert_eq!(count(&log, &hit), 1, "{log:?}");
    assert!(at("watch: tooltip changed") < at(&hit), "{log:?}");
    assert!(at(&hit) < at("watch: tooltip unchanged"), "{log:?}");
    assert!(
        log.contains(&"watch: on (OCR en-US)".to_string()),
        "{log:?}"
    );
}

#[test]
fn screen_miss_repeats_the_step() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let screen = FakeScreen::in_front();
    screen.show(tooltip_of(&ring("06-chaos-below-floor")));
    let (app, ms) = guiding(tmp.path(), &shell, &screen);
    let (first, _) = step(&app);
    read_twice(&app, &ms);
    let view = app.get_view();
    let last = guide(&view).history.last().unwrap().clone();
    assert_eq!(
        (last.result, last.source),
        (OutcomeWord::Miss, OutcomeSource::Screen)
    );
    assert_eq!(
        step(&app),
        (first.clone(), 2),
        "the same step, its second try"
    );
    app.shutdown();
    let log = watch_log(tmp.path());
    assert_eq!(
        count(&log, &format!("craft: miss at {first} (from the screen)")),
        1,
        "{log:?}"
    );
}

#[test]
fn screen_unexpected_change_is_not_applied() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let screen = FakeScreen::in_front();
    // One Chaos Orb brought two targets at once: a grab of it is unexpected.
    screen.show(tooltip_of(&ring("03-whittle-cold")));
    let (app, ms) = guiding(tmp.path(), &shell, &screen);
    let (first, _) = step(&app);
    let before = guide(&app.get_view()).clone();

    read_twice(&app, &ms);
    let view = app.get_view();
    let g = guide(&view);
    assert_eq!(g.status, GuideStatusView::Active);
    assert!(g.history.is_empty(), "nothing kept");
    assert_eq!(g.item, before.item, "the item stays");
    assert_eq!(step(&app), (first.clone(), 1));
    let status = view.status.unwrap();
    assert_eq!(status.kind, StatusKind::Error);
    assert!(status.text.contains("did not expect"), "{}", status.text);
    assert_eq!(
        watch(&app),
        WatchLineView {
            text: "The item changed in a way the guide did not expect: grab it to check."
                .to_string(),
            warning: true
        }
    );

    // The same read again is not reported again.
    for _ in 0..3 {
        later(&ms, 500);
        app.tick_watch();
    }
    assert!(guide(&app.get_view()).history.is_empty());
    assert!(watch(&app).warning);

    // Control: the same change grabbed is the step's result, as in M5.
    let view = app.grab_text(Some(&ring("03-whittle-cold")));
    let last = guide(&view).history.last().unwrap().clone();
    assert_eq!(
        (last.result, last.source),
        (OutcomeWord::Unexpected, OutcomeSource::Grab)
    );
    assert_eq!(guide(&view).status, GuideStatusView::Replan);
    assert_eq!(guide(&view).watch, None, "replan has no watch line");
    app.shutdown();
    let log = watch_log(tmp.path());
    assert_eq!(
        count(
            &log,
            &format!("watch: unexpected change at {first} not applied")
        ),
        1,
        "{log:?}"
    );
    assert!(
        !log.iter().any(|l| l.ends_with("(from the screen)")),
        "{log:?}"
    );
}

#[test]
fn unchanged_tooltip_is_not_an_outcome() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let screen = FakeScreen::in_front();
    screen.show(tooltip_of(&ring("01-start")));
    let (app, ms) = guiding(tmp.path(), &shell, &screen);
    let status = app.get_view().status;
    for _ in 0..4 {
        app.tick_watch();
        later(&ms, 500);
    }
    assert_eq!(screen.reads(), 4);
    assert!(guide(&app.get_view()).history.is_empty());
    assert_eq!(app.get_view().status, status);
    assert_eq!(
        watch(&app),
        WatchLineView {
            text: "Watching: the item has not changed yet.".to_string(),
            warning: false
        }
    );

    // Positive control: the item changed on the screen is read as a hit.
    screen.show(tooltip_of(&ring("02-chaos-fire")));
    read_twice(&app, &ms);
    assert_eq!(guide(&app.get_view()).history.len(), 1);
    app.shutdown();
    let log = watch_log(tmp.path());
    assert_eq!(count(&log, "watch: tooltip unchanged"), 1, "{log:?}");
}

#[test]
fn another_item_changes_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let screen = FakeScreen::in_front();
    // The test gloves: another base type.
    screen.show(tooltip_of(ITEM_TEXT));
    let (app, ms) = guiding(tmp.path(), &shell, &screen);
    let before = guide(&app.get_view()).clone();
    for _ in 0..3 {
        app.tick_watch();
        later(&ms, 500);
    }
    assert_eq!(guide(&app.get_view()).item, before.item);
    assert!(guide(&app.get_view()).history.is_empty());
    assert_eq!(
        watch(&app),
        WatchLineView {
            text: "Watching: that is another item; hover the one you are crafting.".to_string(),
            warning: false
        }
    );

    // An unreadable read of the item changes nothing either, and says why.
    let odd = format!("{}\nGrants a wish", ring("02-chaos-fire").trim_end());
    screen.show(tooltip_of(&odd));
    read_twice(&app, &ms);
    assert!(guide(&app.get_view()).history.is_empty());
    assert_eq!(
        watch(&app),
        WatchLineView {
            text:
                "Can't read the item (a line it does not know): hover it without Alt, or grab it."
                    .to_string(),
            warning: true
        }
    );
    app.shutdown();
    let log = watch_log(tmp.path());
    assert_eq!(count(&log, "watch: tooltip another item"), 1, "{log:?}");
    assert_eq!(count(&log, "watch: tooltip unreadable"), 1, "{log:?}");
    assert!(
        !log.iter()
            .any(|l| l.starts_with("craft: hit") || l.starts_with("craft: miss")),
        "{log:?}"
    );
}

#[test]
fn a_grab_wins_over_a_read_in_flight() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let screen = FakeScreen::in_front();
    screen.show(tooltip_of(&ring("02-chaos-fire")));
    let (app, ms) = guiding(tmp.path(), &shell, &screen);
    let (first, _) = step(&app);

    // Read once; the read that would confirm the hit is held in flight.
    app.tick_watch();
    later(&ms, 400);
    screen.hold_read(true);
    let ticker = {
        let app = Arc::clone(&app);
        std::thread::spawn(move || app.tick_watch())
    };
    wait_until("a read in flight", || screen.rec().in_read);

    // The player grabs a miss meanwhile: it is the step's result.
    let view = app.grab_text(Some(&ring("06-chaos-below-floor")));
    assert_eq!(guide(&view).history.len(), 1);
    screen.hold_read(false);
    ticker.join().unwrap();

    // The read in flight ended after the grab: dropped.
    let view = app.get_view();
    let g = guide(&view);
    assert_eq!(g.history.len(), 1);
    assert_eq!(
        (g.history[0].result, g.history[0].source),
        (OutcomeWord::Miss, OutcomeSource::Grab)
    );
    assert_eq!(step(&app), (first.clone(), 2));
    app.shutdown();
    let log = watch_log(tmp.path());
    assert!(log.contains(&format!("craft: miss at {first}")), "{log:?}");
    assert!(
        !log.iter().any(|l| l.ends_with("(from the screen)")),
        "{log:?}"
    );
}

#[test]
fn a_grab_matching_the_screen_read_is_not_an_outcome() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let screen = FakeScreen::in_front();
    screen.show(tooltip_of(&ring("02-chaos-fire")));
    let (app, ms) = guiding(tmp.path(), &shell, &screen);
    read_twice(&app, &ms);
    assert_eq!(guide(&app.get_view()).history.len(), 1);
    let (second, _) = step(&app);

    // The player grabs the item the screen read: not a new outcome.
    let view = app.grab_text(Some(&ring("02-chaos-fire")));
    assert_eq!(guide(&view).history.len(), 1);
    assert_eq!(step(&app), (second.clone(), 1));
    let status = view.status.unwrap();
    assert_eq!(status.kind, StatusKind::Info);
    assert!(status.text.contains("the screen read"), "{}", status.text);
    // The grabbed text replaced the read item: the same text again is M5's
    // unchanged grab.
    let view = app.grab_text(Some(&ring("02-chaos-fire")));
    assert_eq!(guide(&view).history.len(), 1);
    assert!(view
        .status
        .unwrap()
        .text
        .starts_with("This is the item the step was shown on"));

    // Control: another item grabbed is the step's result.
    let view = app.grab_text(Some(&ring("03-whittle-cold")));
    let g = guide(&view);
    assert_eq!(g.history.len(), 2);
    assert_eq!(
        (g.history[1].result, g.history[1].source),
        (OutcomeWord::Hit, OutcomeSource::Grab)
    );
    app.shutdown();
    let log = watch_log(tmp.path());
    assert_eq!(count(&log, "craft: grab matches the screen"), 1, "{log:?}");
    assert_eq!(
        log.iter()
            .filter(|l| l.starts_with("craft: hit at"))
            .count(),
        2,
        "the screen's and the control's: {log:?}"
    );
}

/// Runs commands that take the app's lock on another thread and waits for
/// them to return, with a deadline.
fn commands_return(app: &Arc<App>, what: &str) {
    let done = Arc::new(AtomicBool::new(false));
    let worker = {
        let (app, done) = (Arc::clone(app), Arc::clone(&done));
        std::thread::spawn(move || {
            app.get_view();
            app.get_app_settings();
            app.update_settings(&json!({ "overlay_position": "top-left" }))
                .unwrap();
            app.grab_text(Some(&ring("01-start")));
            done.store(true, Ordering::SeqCst);
        })
    };
    wait_until(what, || done.load(Ordering::SeqCst));
    worker.join().unwrap();
}

#[test]
fn backend_calls_run_without_the_app_lock() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let screen = FakeScreen::in_front();
    screen.show(tooltip_of(&ring("01-start")));
    let (app, _ms) = guiding(tmp.path(), &shell, &screen);

    // A look at the game's window in flight: the commands still run.
    screen.hold(true);
    let ticker = {
        let app = Arc::clone(&app);
        std::thread::spawn(move || app.tick_watch())
    };
    wait_until("a look in flight", || screen.rec().in_call);
    commands_return(&app, "the commands, with a look in flight");
    assert!(screen.rec().in_call, "still in flight");
    screen.hold(false);
    ticker.join().unwrap();

    // A tooltip read in flight: the same.
    screen.hold_read(true);
    let ticker = {
        let app = Arc::clone(&app);
        std::thread::spawn(move || app.tick_watch())
    };
    wait_until("a read in flight", || screen.rec().in_read);
    commands_return(&app, "the commands, with a read in flight");
    assert!(screen.rec().in_read, "still in flight");
    screen.hold_read(false);
    ticker.join().unwrap();
    assert_eq!(screen.reads(), 2);
    app.shutdown();
}

#[test]
fn no_english_ocr_stops_the_watch_with_a_reason() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let screen = FakeScreen::in_front();
    screen.rec().ocr = Ok(OcrReadiness::NoEnglish);
    screen.show(tooltip_of(&ring("02-chaos-fire")));
    let (app, ms) = guiding(tmp.path(), &shell, &screen);
    for _ in 0..3 {
        app.tick_watch();
        later(&ms, 500);
    }
    assert_eq!((screen.ocr_checks(), screen.reads()), (1, 0));
    assert_eq!(
        watch(&app),
        WatchLineView {
            text: "Can't read the screen (no English OCR): grab the item after each step."
                .to_string(),
            warning: true
        }
    );
    assert!(app.get_view().overlay.visible, "the overlay still shows");

    // Grabs drive the guide as in M5.
    let view = app.grab_text(Some(&ring("02-chaos-fire")));
    let last = guide(&view).history.last().unwrap().clone();
    assert_eq!(
        (last.result, last.source),
        (OutcomeWord::Hit, OutcomeSource::Grab)
    );
    app.tick_watch();
    assert_eq!((screen.ocr_checks(), screen.reads()), (1, 0));
    app.shutdown();
    let log = watch_log(tmp.path());
    assert_eq!(count(&log, "watch: refused: no English OCR"), 1, "{log:?}");

    // Positive control: with English OCR the same craft reads.
    let tmp = tempfile::tempdir().unwrap();
    let screen = FakeScreen::in_front();
    let (app, _ms) = guiding(tmp.path(), &shell, &screen);
    app.tick_watch();
    assert_eq!((screen.ocr_checks(), screen.reads()), (1, 1));
    app.shutdown();
    let log = watch_log(tmp.path());
    assert_eq!(count(&log, "watch: on (OCR en-US)"), 1, "{log:?}");
}

#[test]
fn a_tooltip_under_the_overlay_is_reported() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let screen = FakeScreen::in_front();
    screen.show(TooltipRead::Covered);
    let (app, ms) = guiding(tmp.path(), &shell, &screen);
    app.tick_watch();
    assert!(app.get_view().overlay.visible);
    assert_eq!(
        watch(&app),
        WatchLineView {
            text: "The overlay covers the item: move it in Settings, or grab the item.".to_string(),
            warning: true
        }
    );
    assert!(guide(&app.get_view()).history.is_empty());

    // Moved off the tooltip: watching again.
    screen.show(TooltipRead::NoTooltip);
    later(&ms, 500);
    app.tick_watch();
    assert_eq!(
        watch(&app).text,
        "Watching: after the step, hover the item in the game."
    );
    app.shutdown();
    let log = watch_log(tmp.path());
    let at = |line: &str| log.iter().position(|l| l == line);
    assert!(
        at("watch: tooltip covered by the overlay").is_some(),
        "{log:?}"
    );
    assert!(
        at("watch: tooltip covered by the overlay") < at("watch: tooltip none"),
        "{log:?}"
    );
}

#[test]
fn the_watch_line_follows_the_game_and_the_guide() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let screen = FakeScreen::default();
    let (app, ms) = guiding(tmp.path(), &shell, &screen);
    app.tick_watch();
    assert_eq!(watch(&app).text, "Paused: the game is not in front.");
    assert_eq!(screen.reads(), 0);
    screen.set(Ok(GameWindow::Minimized));
    app.tick_watch();
    assert_eq!(watch(&app).text, "Paused: the game is minimized.");
    assert_eq!(shell.last_view(), app.get_view(), "the windows heard it");
    screen.set(in_front(CLIENT));
    later(&ms, 500);
    app.tick_watch();
    assert_eq!(
        watch(&app).text,
        "Watching: after the step, hover the item in the game."
    );
    assert_eq!(screen.reads(), 1);
    app.shutdown();

    // Without a screen backend (M5's shape) the line says so.
    let tmp = tempfile::tempdir().unwrap();
    let app = start(tmp.path(), &shell);
    app.grab_text(Some(&ring("01-start")));
    app.set_targets(&ring_targets());
    app.choose_route(WHITTLE);
    app.accept_craft();
    assert_eq!(
        watch(&app).text,
        "Screen reading is not available: grab the item after each step."
    );
    app.shutdown();
}

#[test]
fn black_copies_and_the_minute_line_are_logged() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let screen = FakeScreen::in_front();
    screen.show(TooltipRead::Black);
    let (app, ms) = guiding(tmp.path(), &shell, &screen);
    for _ in 0..9 {
        app.tick_watch();
        later(&ms, 500);
    }
    assert_eq!(
        watch(&app).text,
        "Watching: after the step, hover the item in the game."
    );
    app.tick_watch();
    assert_eq!(
        watch(&app),
        WatchLineView {
            text: "Can't copy the game's window: grab the item after each step.".to_string(),
            warning: true
        }
    );
    screen.rec().counts = ReadCounts {
        copies: 12,
        tooltips: 3,
        ocr_reads: 2,
    };
    later(&ms, 60_000);
    app.tick_watch();
    app.shutdown();
    let log = watch_log(tmp.path());
    assert_eq!(
        count(&log, "watch: black: 10 copies in a row"),
        1,
        "{log:?}"
    );
    let minute: Vec<&String> = log
        .iter()
        .filter(|l| l.starts_with("watch: minute: "))
        .collect();
    assert_eq!(minute.len(), 1, "{log:?}");
    assert!(
        minute[0].starts_with("watch: minute: 12 copies, 3 tooltips, 2 OCR reads, busy "),
        "{minute:?}"
    );
    assert!(minute[0].ends_with(" ms"));
}

/// A backend call that failed with the system's error.
fn failure(call: &str) -> WatchFailure {
    WatchFailure {
        call: call.to_string(),
        error: "Access is denied. (0x80070005)".to_string(),
    }
}

/// The watch line while `call` fails.
fn failed(call: &str) -> WatchLineView {
    WatchLineView {
        text: format!("Can't read the screen ({call} failed): grab the item after each step."),
        warning: true,
    }
}

fn ticks(app: &App, ms: &AtomicU64, n: usize) {
    for _ in 0..n {
        app.tick_watch();
        later(ms, 500);
    }
}

#[test]
fn a_failed_backend_call_is_said_and_logged_once() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let screen = FakeScreen::in_front();
    // The game's window call fails: the line says so, not that the game is
    // not in front.
    screen.set(Err(failure("window")));
    let (app, ms) = guiding(tmp.path(), &shell, &screen);
    ticks(&app, &ms, 3);
    assert_eq!(watch(&app), failed("window"));
    assert_eq!((screen.ocr_checks(), screen.reads()), (0, 0));

    // The window call works, the OCR check fails.
    screen.set(in_front(CLIENT));
    screen.rec().ocr = Err(failure("OCR"));
    ticks(&app, &ms, 3);
    assert_eq!(watch(&app), failed("OCR"));
    assert_eq!((screen.ocr_checks(), screen.reads()), (3, 0));

    // The OCR reads English, the copy fails.
    screen.rec().ocr = Ok(OcrReadiness::Ready {
        language: "en-US".to_string(),
    });
    screen.rec().tooltip = Err(failure("copy"));
    ticks(&app, &ms, 3);
    assert_eq!(watch(&app), failed("copy"));
    assert_eq!(screen.reads(), 3);
    assert!(guide(&app.get_view()).history.is_empty());

    // Every call works again: watching.
    screen.show(TooltipRead::NoTooltip);
    ticks(&app, &ms, 1);
    assert_eq!(
        watch(&app),
        WatchLineView {
            text: "Watching: after the step, hover the item in the game.".to_string(),
            warning: false
        }
    );
    app.shutdown();
    let log = diag_lines(tmp.path());
    for call in ["window", "OCR", "copy"] {
        let line = format!("error: watch: {call} failed: Access is denied. (0x80070005)");
        assert_eq!(count(&log, &line), 1, "{call}: {log:?}");
    }
    assert_eq!(count(&log, "watch: idle (game not in front)"), 0, "{log:?}");
}

#[test]
fn a_failure_is_forgotten_when_guiding_ends() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let screen = FakeScreen::in_front();
    let watching = WatchLineView {
        text: "Watching: after the step, hover the item in the game.".to_string(),
        warning: false,
    };
    let again = |app: &App| {
        app.end_craft();
        app.choose_route(WHITTLE);
        let view = app.accept_craft();
        assert_eq!(view.craft.mode, ch_app::CraftMode::Guiding);
    };

    // The copy fails in one craft; the next starts from a fresh look.
    screen.rec().tooltip = Err(failure("copy"));
    let (app, ms) = guiding(tmp.path(), &shell, &screen);
    ticks(&app, &ms, 2);
    assert_eq!(watch(&app), failed("copy"));
    again(&app);
    assert_eq!(watch(&app), watching);

    // So does the game's window call.
    screen.set(Err(failure("window")));
    ticks(&app, &ms, 2);
    assert_eq!(watch(&app), failed("window"));
    again(&app);
    assert_eq!(watch(&app), watching);
    // Still failing in the new craft: said and logged again.
    ticks(&app, &ms, 1);
    assert_eq!(watch(&app), failed("window"));
    app.shutdown();
    let log = diag_lines(tmp.path());
    let line = |call: &str| format!("error: watch: {call} failed: Access is denied. (0x80070005)");
    assert_eq!(count(&log, &line("copy")), 1, "{log:?}");
    assert_eq!(count(&log, &line("window")), 2, "{log:?}");
}

#[test]
fn an_unreadable_item_says_to_grab_after_each_step() {
    let hover_or_grab = |why: &str| WatchLineView {
        text: format!("Can't read the item ({why}): hover it without Alt, or grab it."),
        warning: true,
    };
    let grab_each_step = |why: &str| WatchLineView {
        text: format!("Can't read the item ({why}): grab the item after each step."),
        warning: true,
    };

    // Negative controls, on the plain start ring: a reason the read
    // brought keeps the line that says to hover again, both a line the
    // matcher does not know and the note under an unrevealed modifier.
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let screen = FakeScreen::in_front();
    let (app, ms) = guiding(tmp.path(), &shell, &screen);
    let odd = format!("{}\nGrants a wish", ring("02-chaos-fire").trim_end());
    screen.show(tooltip_of(&odd));
    read_twice(&app, &ms);
    assert_eq!(watch(&app), hover_or_grab("a line it does not know"));
    let veiled = format!(
        "{}\nTake this item to the Well of Souls to reveal the Desecrated Modifier",
        ring("01-start").trim_end()
    );
    screen.show(tooltip_of(&veiled));
    read_twice(&app, &ms);
    assert_eq!(
        watch(&app),
        hover_or_grab("an unrevealed or fractured modifier")
    );

    // Target: the step's item has an unrevealed modifier itself (the ring
    // after the desecrate step), so no hover can read it.
    for file in ["02-chaos-fire", "03-whittle-cold", "04-desecrated"] {
        app.grab_text(Some(&ring(file)));
    }
    assert_eq!(guide(&app.get_view()).history.len(), 3);
    screen.show(tooltip_of(&ring("04-desecrated")));
    read_twice(&app, &ms);
    assert_eq!(
        watch(&app),
        grab_each_step("an unrevealed or fractured modifier")
    );
    assert_eq!(guide(&app.get_view()).history.len(), 3, "nothing applied");
    app.shutdown();

    // Target: one of the step item's modifiers is not in the data.
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let screen = FakeScreen::in_front();
    let (app, ms) = start_watching(tmp.path(), &shell, &screen);
    let unknown = ring("01-start").replace(
        "{ Suffix Modifier \"of the Lion\" (Tier: 5) — Attribute }\n+18(17-20) to Strength",
        "{ Suffix Modifier \"of Nowhere\" (Tier: 5) — Attribute }\n+18(17-20) to Nonsense",
    );
    assert_ne!(unknown, ring("01-start"));
    app.grab_text(Some(&unknown));
    app.set_targets(&ring_targets());
    app.choose_route(WHITTLE);
    let view = app.accept_craft();
    assert_eq!(
        view.craft.mode,
        ch_app::CraftMode::Guiding,
        "{:?}",
        view.status
    );
    screen.show(tooltip_of(&ring("02-chaos-fire")));
    read_twice(&app, &ms);
    assert_eq!(watch(&app), grab_each_step("a modifier it does not know"));
    assert!(guide(&app.get_view()).history.is_empty());
    app.shutdown();
    let log = watch_log(tmp.path());
    assert_eq!(count(&log, "watch: tooltip unreadable"), 1, "{log:?}");
}
