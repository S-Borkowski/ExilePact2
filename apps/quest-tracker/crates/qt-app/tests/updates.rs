//! The update check in the shell (Context "The update checker in qt-app",
//! "The privacy invariant and its proof"), driven by a fake transport that
//! records every call and can be held on a gate, a fake clock, a fake
//! emitter that records the Settings window's views and a fake window that
//! records the URLs it opens. Nothing here touches the network.
//!
//! Baseline: with the defaults, an old `settings.json` without the key, a
//! corrupt one or no store at all, the transport is never called across
//! start, commands and 48 simulated hours of ticks, and no `updates:` line is
//! written; a manual check through the same fake then makes exactly one call
//! (the positive control). Target: a manual check makes one request; with
//! `check_updates` on, one at launch and then one a day, one an hour after a
//! failed automatic check, none once it is turned off; a check never blocks
//! a command or the worker, runs one at a time, and is not waited for at
//! shutdown.

use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use qt_app::updates::{
    newest_release, Answer, TransportError, Unreadable, RELEASE_PAGE_PREFIX, TAG_PREFIX,
    TRAY_CHECK_UPDATES, TRAY_QUIT, TRAY_SETTINGS, TRAY_UPDATE_AVAILABLE,
};
use qt_app::{
    tray_menu, App, AppConfig, AppSettingsView, Clock, Emitter, Mode, MonitorMetrics, PanelConfig,
    Placement, TrayAction, TrayItem, UpdateConfig, UpdateState, UpdateTransport, Version, Window,
    SHUTDOWN_TIMEOUT,
};
use qt_store::Store;
use qt_viewmodel::{RowChange, View};
use serde_json::{json, Value};

const HOUR: u64 = 60 * 60 * 1_000;
const MINUTE: u64 = 60 * 1_000;
/// The running version every harness gives the app: synthetic, never the
/// crate's, so a release bump changes nothing here.
const RUNNING: Version = Version::new(1, 2, 0);
const NEWER: Version = Version::new(1, 3, 0);

// ---------------------------------------------------------------------------
// The fakes.

type Reply = Result<Answer, TransportError>;

/// The transport's shared record: how many calls began and returned, the
/// replies queued (else `fallback`), and the gate a call waits on while it is
/// closed.
struct Wire {
    calls: AtomicUsize,
    returned: AtomicUsize,
    replies: Mutex<VecDeque<Reply>>,
    fallback: Mutex<Reply>,
    open: Mutex<bool>,
    opened: Condvar,
}

struct FakeTransport(Arc<Wire>);

impl UpdateTransport for FakeTransport {
    fn fetch_releases(&self) -> Result<Answer, TransportError> {
        let w = &self.0;
        w.calls.fetch_add(1, Ordering::SeqCst);
        let mut open = w.open.lock().unwrap();
        while !*open {
            open = w.opened.wait(open).unwrap();
        }
        drop(open);
        let reply = w
            .replies
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| w.fallback.lock().unwrap().clone());
        w.returned.fetch_add(1, Ordering::SeqCst);
        reply
    }
}

#[derive(Default)]
struct Recorded {
    app_settings: Mutex<Vec<AppSettingsView>>,
    opened: Mutex<Vec<String>>,
}

struct FakeEmitter(Arc<Recorded>);

impl Emitter for FakeEmitter {
    fn view(&self, _: &View) {}
    fn row_change(&self, _: &RowChange) {}
    fn app_settings(&self, view: &AppSettingsView) {
        self.0.app_settings.lock().unwrap().push(view.clone());
    }
}

struct FakeWindow(Arc<Recorded>);

impl Window for FakeWindow {
    fn set_click_through(&self, _: bool) -> Result<(), String> {
        Ok(())
    }
    fn set_scale(&self, _: f64) -> Result<(), String> {
        Ok(())
    }
    fn set_resizable(&self, _: bool) -> Result<(), String> {
        Ok(())
    }
    fn set_size(&self, _: f64, _: f64) -> Result<(), String> {
        Ok(())
    }
    fn place(&self, _: Placement) -> Result<(), String> {
        Ok(())
    }
    fn register_hotkey(&self, _: &str) -> Result<(), String> {
        Ok(())
    }
    fn open_url(&self, url: &str) -> Result<(), String> {
        self.0.opened.lock().unwrap().push(url.to_string());
        Ok(())
    }
    fn monitor(&self) -> Result<Option<MonitorMetrics>, String> {
        Ok(None)
    }
    fn foreground_pid(&self) -> Option<u32> {
        None
    }
    fn set_visible(&self, _: bool) -> Result<(), String> {
        Ok(())
    }
}

/// The fake clock, which counts its reads: the worker reads it on every
/// tick, so a rising count shows the worker is still ticking.
struct FakeClock {
    now: Arc<AtomicU64>,
    reads: Arc<AtomicU64>,
}

impl Clock for FakeClock {
    fn now_ms(&self) -> u64 {
        self.reads.fetch_add(1, Ordering::SeqCst);
        self.now.load(Ordering::SeqCst)
    }
}

// ---------------------------------------------------------------------------
// Answers.

/// A 200 answer listing published Quest Tracker releases of `versions`.
fn releases(versions: &[&str]) -> Reply {
    let list: Vec<Value> = versions
        .iter()
        .map(|v| json!({ "tag_name": format!("{TAG_PREFIX}{v}"), "draft": false, "prerelease": false }))
        .collect();
    Ok(Answer {
        status: 200,
        body: serde_json::to_vec(&list).unwrap(),
    })
}

fn status(code: u16) -> Reply {
    Ok(Answer {
        status: code,
        body: br#"{"message":"a body that is never shown"}"#.to_vec(),
    })
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn releases_fixture() -> Vec<u8> {
    fs::read(root().join("fixtures/updates/releases.json")).expect("fixtures/updates/releases.json")
}

// ---------------------------------------------------------------------------
// The harness.

/// What the store holds when the app is made.
enum Saved {
    /// No `settings.json`: a first run.
    Nothing,
    /// A `settings.json` with this content.
    Settings(Value),
    /// A `settings.json` that is not JSON.
    Corrupt,
    /// No store at all (it could not be opened).
    NoStore,
}

struct Setup {
    saved: Saved,
    /// Whether the app gets a transport.
    transport: bool,
    /// The gate starts open.
    open: bool,
}

impl Default for Setup {
    fn default() -> Setup {
        Setup {
            saved: Saved::Nothing,
            transport: true,
            open: true,
        }
    }
}

struct Harness {
    _dir: tempfile::TempDir,
    root: PathBuf,
    app: Arc<App>,
    wire: Arc<Wire>,
    rec: Arc<Recorded>,
    now: Arc<AtomicU64>,
    reads: Arc<AtomicU64>,
}

fn harness(setup: Setup) -> Harness {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path().to_path_buf();
    let store_dir = root.join("store");
    let store = match &setup.saved {
        Saved::NoStore => Err("the store could not be opened (test)".to_string()),
        saved => {
            let s = Store::open(&store_dir).expect("store");
            match saved {
                Saved::Settings(v) => s.save_settings(v).expect("settings written"),
                Saved::Corrupt => {
                    fs::create_dir_all(&store_dir).expect("store folder");
                    fs::write(store_dir.join("settings.json"), "{ \"check_updates\": tru")
                        .expect("corrupt settings written")
                }
                _ => {}
            }
            Store::open(&store_dir).map_err(|e| e.to_string())
        }
    };
    let wire = Arc::new(Wire {
        calls: AtomicUsize::new(0),
        returned: AtomicUsize::new(0),
        replies: Mutex::new(VecDeque::new()),
        fallback: Mutex::new(releases(&["1.3.0"])),
        open: Mutex::new(setup.open),
        opened: Condvar::new(),
    });
    let rec = Arc::new(Recorded::default());
    let now = Arc::new(AtomicU64::new(100_000));
    let reads = Arc::new(AtomicU64::new(0));
    let config = AppConfig {
        options: Default::default(),
        options_error: None,
        store,
        data: None,
        candidates: Vec::new(),
        poll_interval: Duration::from_millis(20),
        emitter: Box::new(FakeEmitter(Arc::clone(&rec))),
        window: Box::new(FakeWindow(Arc::clone(&rec))),
        clock: Box::new(FakeClock {
            now: Arc::clone(&now),
            reads: Arc::clone(&reads),
        }),
        screen: None,
    };
    let transport: Option<Box<dyn UpdateTransport>> = if setup.transport {
        Some(Box::new(FakeTransport(Arc::clone(&wire))))
    } else {
        None
    };
    let app = App::with_updates(
        config,
        PanelConfig::default(),
        UpdateConfig {
            transport,
            running: Some(RUNNING),
        },
    );
    Harness {
        _dir: dir,
        root,
        app,
        wire,
        rec,
        now,
        reads,
    }
}

fn auto_on() -> Setup {
    Setup {
        saved: Saved::Settings(json!({ "check_updates": true })),
        ..Setup::default()
    }
}

fn wait_until(what: &str, pred: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !pred() {
        assert!(Instant::now() < deadline, "{what}: not within 5 s");
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// Long enough for the worker to tick at least once more.
fn let_the_worker_tick() {
    std::thread::sleep(Duration::from_millis(400));
}

impl Harness {
    fn calls(&self) -> usize {
        self.wire.calls.load(Ordering::SeqCst)
    }

    fn reply(&self, r: Reply) {
        self.wire.replies.lock().unwrap().push_back(r);
    }

    fn hold(&self) {
        *self.wire.open.lock().unwrap() = false;
    }

    fn release(&self) {
        *self.wire.open.lock().unwrap() = true;
        self.wire.opened.notify_all();
    }

    /// What `settings.json` holds now.
    fn saved_settings(&self) -> Value {
        Store::open(self.root.join("store"))
            .and_then(|s| s.load_settings())
            .expect("settings readable")
            .value
            .expect("settings saved")
    }

    fn diag_text(&self) -> String {
        fs::read_to_string(self.root.join("store").join("overlay.log")).unwrap_or_default()
    }

    /// The `updates:` lines, without the log's own prefix.
    fn update_lines(&self) -> Vec<String> {
        self.diag_text()
            .lines()
            .filter_map(|l| l.find("updates: ").map(|i| l[i..].to_string()))
            .collect()
    }

    fn state(&self) -> UpdateState {
        self.app.get_app_settings().update.state
    }

    /// Waits for the check running to end and its view to be sent (which
    /// comes after its `updates:` line is written); that view.
    fn wait_ended(&self) -> AppSettingsView {
        wait_until("the check ends and is announced", || {
            let view = self.app.get_app_settings();
            view.update.state != UpdateState::Checking && self.emitted().last() == Some(&view)
        });
        self.app.get_app_settings()
    }

    fn wait_calls(&self, n: usize) {
        wait_until(&format!("{n} transport calls"), || self.calls() >= n);
        assert_eq!(self.calls(), n, "more calls than expected");
    }

    /// Moves the clock to `ms` after its start and ticks once.
    fn at(&self, ms: u64) {
        self.now.store(100_000 + ms, Ordering::SeqCst);
        self.app.tick();
    }

    /// Ticks every half hour from `from` to `to` (after the clock's start).
    fn ticks(&self, from: u64, to: u64) {
        let mut t = from;
        while t <= to {
            self.at(t);
            t += 30 * MINUTE;
        }
    }

    fn emitted(&self) -> Vec<AppSettingsView> {
        self.rec.app_settings.lock().unwrap().clone()
    }

    fn opened(&self) -> Vec<String> {
        self.rec.opened.lock().unwrap().clone()
    }

    /// One manual check with `reply`, waited for; its view.
    fn manual(&self, reply: Reply) -> AppSettingsView {
        let before = self.calls();
        self.reply(reply);
        self.app.check_for_updates();
        self.wait_calls(before + 1);
        self.wait_ended()
    }
}

fn lines(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

/// Drives every command and 48 simulated hours of ticks, asserting no
/// request and no `updates:` line, then makes one manual check: the
/// positive control that the same fake counts a request.
fn assert_no_request_unless_asked(h: &Harness, readable_log: bool) {
    h.app.start();
    let _ = h.app.get_view();
    h.app.set_mode(Mode::Edit).expect("set_mode");
    h.app.update_settings(&json!({ "opacity": 0.5 })).expect("update_settings");
    h.app.toggle_mode().expect("toggle_mode");
    let view = h.app.get_app_settings();
    assert!(!view.check_updates);
    assert_eq!(view.update.state, UpdateState::Idle);
    assert_eq!(view.update.text, "Not checked yet.");
    assert_eq!(view.update.newer, None);
    assert!(h.app.open_release_page().is_err(), "no newer version is known");
    h.ticks(0, 48 * HOUR);
    let_the_worker_tick();
    assert_eq!(h.calls(), 0, "a request nobody asked for");
    assert!(h.update_lines().is_empty(), "{:?}", h.update_lines());
    assert!(h.emitted().is_empty(), "{:?}", h.emitted());
    assert!(h.opened().is_empty(), "{:?}", h.opened());

    // Positive control: the same instrument sees a request once one is asked.
    h.manual(releases(&["1.2.0"]));
    assert_eq!(h.calls(), 1);
    if readable_log {
        assert_eq!(
            h.update_lines(),
            lines(&[
                "updates: check started (manual)",
                "updates: up to date (running 1.2.0, latest 1.2.0)",
            ])
        );
    }
    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));
}

// ---------------------------------------------------------------------------
// Baseline.

#[test]
fn updates_default_settings_make_no_request_baseline() {
    let h = harness(Setup::default());
    assert_no_request_unless_asked(&h, true);
    // What the first run saved has the setting, off.
    assert_eq!(h.saved_settings()["check_updates"], false);
}

#[test]
fn updates_settings_file_without_the_key_makes_no_request_baseline() {
    // A file written before the setting existed, with other things on.
    let old = harness(Setup {
        saved: Saved::Settings(json!({ "opacity": 0.5, "hotkey": "F8", "show_skipped": true })),
        ..Setup::default()
    });
    assert_no_request_unless_asked(&old, true);
    // A corrupt file loads the defaults.
    let corrupt = harness(Setup {
        saved: Saved::Corrupt,
        ..Setup::default()
    });
    assert_no_request_unless_asked(&corrupt, true);
    // No store at all: nothing loads, nothing is logged.
    let none = harness(Setup {
        saved: Saved::NoStore,
        ..Setup::default()
    });
    assert_no_request_unless_asked(&none, false);
}

// ---------------------------------------------------------------------------
// Target.

#[test]
fn updates_manual_check_makes_one_request_target() {
    let h = harness(Setup::default());
    h.app.start();
    h.hold();
    let view = h.app.check_for_updates();
    // It returns at once, with the check running.
    assert_eq!(view.update.state, UpdateState::Checking);
    assert_eq!(view.update.text, "Checking for updates\u{2026}");
    h.wait_calls(1);
    h.release();
    let view = h.wait_ended();
    assert_eq!(view.update.state, UpdateState::Available);
    assert_eq!(view.update.text, "Quest Tracker 1.3.0 is available. You have 1.2.0.");
    assert_eq!(view.update.newer.as_deref(), Some("1.3.0"));
    assert_eq!(view.version, "1.2.0");
    assert!(!view.check_updates);
    assert_eq!(
        h.update_lines(),
        lines(&["updates: check started (manual)", "updates: 1.3.0 available (running 1.2.0)"])
    );
    // The window saw the check start and end; the last view sent is current.
    let sent = h.emitted();
    assert!(sent.iter().any(|v| v.update.state == UpdateState::Checking), "{sent:?}");
    assert_eq!(sent.last(), Some(&view));
    // A manual check schedules nothing: the setting is off.
    h.ticks(0, 48 * HOUR);
    let_the_worker_tick();
    assert_eq!(h.calls(), 1);
    assert_eq!(h.update_lines().len(), 2);
    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));
}

#[test]
fn updates_auto_on_checks_at_start_then_daily_target() {
    let h = harness(auto_on());
    h.reply(releases(&["1.2.0", "1.1.0"]));
    h.app.start();
    // The first tick after a launch with the setting on checks once.
    h.wait_calls(1);
    let view = h.wait_ended();
    assert_eq!(view.update.state, UpdateState::UpToDate);
    assert_eq!(view.update.text, "You have the latest version (1.2.0).");
    assert!(view.check_updates);
    assert_eq!(
        h.update_lines(),
        lines(&[
            "updates: check started (automatic)",
            "updates: up to date (running 1.2.0, latest 1.2.0)",
        ])
    );
    // Not again within the day.
    h.ticks(30 * MINUTE, 24 * HOUR - MINUTE);
    let_the_worker_tick();
    assert_eq!(h.calls(), 1);
    // A day after it started, again.
    h.reply(releases(&["1.2.0"]));
    h.at(24 * HOUR);
    h.wait_calls(2);
    h.wait_ended();
    // A manual check counts as one: the next automatic one is a day after it.
    h.now.store(100_000 + 30 * HOUR, Ordering::SeqCst);
    h.manual(releases(&["1.2.0"]));
    assert_eq!(h.calls(), 3);
    h.ticks(30 * HOUR, 54 * HOUR - MINUTE);
    let_the_worker_tick();
    assert_eq!(h.calls(), 3, "due a day after the manual check, not the automatic one");
    h.reply(releases(&["1.2.0"]));
    h.at(54 * HOUR);
    h.wait_calls(4);
    h.wait_ended();
    assert_eq!(
        h.update_lines()
            .iter()
            .filter(|l| l.starts_with("updates: check started"))
            .cloned()
            .collect::<Vec<_>>(),
        lines(&[
            "updates: check started (automatic)",
            "updates: check started (automatic)",
            "updates: check started (manual)",
            "updates: check started (automatic)",
        ])
    );
    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));
}

#[test]
fn updates_turning_auto_on_checks_and_off_stops_target() {
    let h = harness(Setup::default());
    h.app.start();
    h.ticks(0, 2 * HOUR);
    let_the_worker_tick();
    assert_eq!(h.calls(), 0);
    // Turned on: logged, announced, and the next tick checks.
    let view = h
        .app
        .update_settings(&json!({ "check_updates": true }))
        .expect("turned on");
    assert!(view.settings.check_updates);
    assert!(h.emitted().last().is_some_and(|v| v.check_updates), "{:?}", h.emitted());
    h.reply(releases(&["1.2.0"]));
    h.at(2 * HOUR + MINUTE);
    h.wait_calls(1);
    h.wait_ended();
    // Turned off and on again within the day: no check until a day after
    // the last one started.
    h.app
        .update_settings(&json!({ "check_updates": false }))
        .expect("turned off");
    assert!(h.emitted().last().is_some_and(|v| !v.check_updates));
    h.ticks(3 * HOUR, 4 * HOUR);
    h.app
        .update_settings(&json!({ "check_updates": true }))
        .expect("turned on again");
    h.ticks(4 * HOUR + MINUTE, 26 * HOUR);
    let_the_worker_tick();
    assert_eq!(h.calls(), 1);
    h.reply(releases(&["1.2.0"]));
    h.at(26 * HOUR + 2 * MINUTE);
    h.wait_calls(2);
    h.wait_ended();
    // Off: nothing more, however long it runs.
    h.app
        .update_settings(&json!({ "check_updates": false }))
        .expect("turned off");
    h.ticks(27 * HOUR, 80 * HOUR);
    let_the_worker_tick();
    assert_eq!(h.calls(), 2);
    assert_eq!(
        h.update_lines(),
        lines(&[
            "updates: automatic checks turned on",
            "updates: check started (automatic)",
            "updates: up to date (running 1.2.0, latest 1.2.0)",
            "updates: automatic checks turned off",
            "updates: automatic checks turned on",
            "updates: check started (automatic)",
            "updates: up to date (running 1.2.0, latest 1.2.0)",
            "updates: automatic checks turned off",
        ])
    );
    // It is kept in settings.json.
    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));
    assert_eq!(h.saved_settings()["check_updates"], false);
}

#[test]
fn updates_failed_auto_check_retries_after_an_hour() {
    let h = harness(auto_on());
    h.reply(Err(TransportError::Unreachable));
    h.app.start();
    h.wait_calls(1);
    let view = h.wait_ended();
    assert_eq!(view.update.state, UpdateState::Failed);
    assert_eq!(view.update.text, "Could not check for updates: GitHub could not be reached.");
    // Not within the hour after it failed.
    h.ticks(0, 59 * MINUTE);
    let_the_worker_tick();
    assert_eq!(h.calls(), 1);
    // An hour after, again; this one succeeds.
    h.reply(releases(&["1.2.0"]));
    h.at(60 * MINUTE);
    h.wait_calls(2);
    assert_eq!(h.wait_ended().update.state, UpdateState::UpToDate);
    // After a success, no hourly retry: the next is a day after it started.
    h.ticks(90 * MINUTE, 24 * HOUR + 59 * MINUTE);
    let_the_worker_tick();
    assert_eq!(h.calls(), 2);
    h.reply(releases(&["1.2.0"]));
    h.at(25 * HOUR);
    h.wait_calls(3);
    h.wait_ended();
    // Negative control: a failed manual check is not retried hourly.
    h.now.store(100_000 + 26 * HOUR, Ordering::SeqCst);
    let failed = h.manual(Err(TransportError::Timeout));
    assert_eq!(failed.update.text, "Could not check for updates: GitHub did not answer in time.");
    h.ticks(26 * HOUR + 30 * MINUTE, 50 * HOUR - MINUTE);
    let_the_worker_tick();
    assert_eq!(h.calls(), 4);
    h.reply(releases(&["1.2.0"]));
    h.at(50 * HOUR);
    h.wait_calls(5);
    h.wait_ended();
    let ends: Vec<String> = h
        .update_lines()
        .into_iter()
        .filter(|l| !l.starts_with("updates: check started"))
        .collect();
    assert_eq!(
        ends,
        lines(&[
            "updates: check failed: GitHub could not be reached",
            "updates: up to date (running 1.2.0, latest 1.2.0)",
            "updates: up to date (running 1.2.0, latest 1.2.0)",
            "updates: check failed: GitHub did not answer in time",
            "updates: up to date (running 1.2.0, latest 1.2.0)",
        ])
    );
    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));
}

#[test]
fn updates_check_never_blocks_commands_or_the_worker() {
    let h = harness(Setup {
        open: false,
        ..auto_on()
    });
    h.app.start();
    // The worker's tick started a check, which now waits on the gate.
    h.wait_calls(1);
    assert_eq!(h.state(), UpdateState::Checking);
    let quick = |what: &str, f: &dyn Fn()| {
        let t = Instant::now();
        f();
        assert!(t.elapsed() < Duration::from_secs(1), "{what} waited for the check");
    };
    quick("get_view", &|| {
        let _ = h.app.get_view();
    });
    quick("set_mode", &|| {
        h.app.set_mode(Mode::Edit).unwrap();
    });
    quick("update_settings", &|| {
        h.app.update_settings(&json!({ "opacity": 0.4 })).unwrap();
    });
    quick("get_app_settings", &|| {
        assert_eq!(h.app.get_app_settings().update.state, UpdateState::Checking);
    });
    quick("check_for_updates", &|| {
        assert_eq!(h.app.check_for_updates().update.state, UpdateState::Checking);
    });
    quick("tick", &|| h.app.tick());
    // The worker keeps ticking: it reads the clock on every tick.
    let before = h.reads.load(Ordering::SeqCst);
    let_the_worker_tick();
    assert!(h.reads.load(Ordering::SeqCst) > before, "the worker stopped ticking");
    assert!(h.app.worker_running());
    assert_eq!(h.calls(), 1);
    h.release();
    assert_eq!(h.wait_ended().update.state, UpdateState::Available);
    assert_eq!(h.calls(), 1);
    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));
}

#[test]
fn updates_request_while_checking_is_coalesced() {
    let h = harness(Setup {
        open: false,
        ..Setup::default()
    });
    h.app.start();
    for _ in 0..3 {
        assert_eq!(h.app.check_for_updates().update.state, UpdateState::Checking);
    }
    h.wait_calls(1);
    // Turned on while it runs: the tick starts no second one either.
    h.app.update_settings(&json!({ "check_updates": true })).unwrap();
    h.app.tick();
    let_the_worker_tick();
    assert_eq!(h.calls(), 1);
    h.release();
    h.wait_ended();
    let_the_worker_tick();
    assert_eq!(h.calls(), 1);
    assert_eq!(
        h.update_lines(),
        lines(&[
            "updates: check started (manual)",
            "updates: automatic checks turned on",
            "updates: 1.3.0 available (running 1.2.0)",
        ])
    );
    // Not stuck: once it ended, a request makes the next call.
    h.manual(releases(&["1.3.0"]));
    assert_eq!(h.calls(), 2);
    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));
}

#[test]
fn updates_shutdown_does_not_wait_for_a_check() {
    let h = harness(Setup {
        open: false,
        ..Setup::default()
    });
    h.app.start();
    h.app.check_for_updates();
    h.wait_calls(1);
    let t = Instant::now();
    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT), "every thread ended in time");
    assert!(t.elapsed() < Duration::from_secs(1), "shutdown waited for the check");
    let sent = h.emitted().len();
    let lines_then = h.update_lines();
    // The check ends after shutdown began: it writes and announces nothing.
    h.release();
    wait_until("the request returns", || h.wire.returned.load(Ordering::SeqCst) == 1);
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(h.emitted().len(), sent);
    assert_eq!(h.update_lines(), lines_then);
    assert_eq!(lines_then, lines(&["updates: check started (manual)"]));
    // And none starts any more.
    h.app.check_for_updates();
    h.app.tick();
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(h.calls(), 1);
}

#[test]
fn updates_release_page_comes_from_the_tag() {
    let h = harness(Setup::default());
    // Nothing known: refused, nothing opened.
    let err = h.app.open_release_page().unwrap_err();
    assert!(err.contains("no newer version"), "{err}");
    // The fixture's newest release points its `html_url` elsewhere.
    let view = h.manual(Ok(Answer {
        status: 200,
        body: releases_fixture(),
    }));
    assert_eq!(view.update.newer.as_deref(), Some("1.10.0"));
    h.app.open_release_page().expect("opened");
    let page = format!("{RELEASE_PAGE_PREFIX}{TAG_PREFIX}1.10.0");
    assert_eq!(page, "https://github.com/S-Borkowski/ExilePact2/releases/tag/quest-tracker-v1.10.0");
    assert_eq!(h.opened(), vec![page.clone()]);
    // A failed check keeps the newer version, and its page.
    h.manual(status(500));
    h.app.open_release_page().expect("still known");
    assert_eq!(h.opened(), vec![page.clone(), page.clone()]);
    // A later success replaces it: no release, nothing to open.
    h.manual(releases(&[]));
    assert!(h.app.open_release_page().is_err());
    assert_eq!(h.opened().len(), 2);
    assert!(h.opened().iter().all(|u| !u.contains("example.invalid")));
    assert_eq!(
        h.update_lines()
            .into_iter()
            .filter(|l| l.contains("release page"))
            .collect::<Vec<_>>(),
        lines(&[
            "updates: opened the release page of 1.10.0",
            "updates: opened the release page of 1.10.0",
        ])
    );
}

/// The entry ids of a tray menu, `-` for the separator.
fn ids(view: &AppSettingsView) -> Vec<&'static str> {
    tray_menu(view)
        .items
        .iter()
        .map(|i| match i {
            TrayItem::Entry { id, .. } => *id,
            TrayItem::Separator => "-",
        })
        .collect()
}

#[test]
fn updates_tray_menu_follows_the_outcome() {
    let h = harness(Setup::default());
    let plain = vec![TRAY_CHECK_UPDATES, TRAY_SETTINGS, "-", TRAY_QUIT];
    let idle = h.app.get_app_settings();
    let menu = tray_menu(&idle);
    assert_eq!(ids(&idle), plain);
    assert_eq!(menu.tooltip, "Quest Tracker");
    assert_eq!(
        menu.items,
        vec![
            TrayItem::Entry { id: "check-updates", text: "Check for updates".into() },
            TrayItem::Entry { id: "settings", text: "Settings".into() },
            TrayItem::Separator,
            TrayItem::Entry { id: "quit", text: "Quit Quest Tracker".into() },
        ]
    );
    // A newer version adds the entry on top and names it in the tooltip.
    let available = h.manual(releases(&["1.3.0"]));
    let menu = tray_menu(&available);
    assert_eq!(ids(&available)[0], TRAY_UPDATE_AVAILABLE);
    assert_eq!(&ids(&available)[1..], &plain[..]);
    assert_eq!(
        menu.items[0],
        TrayItem::Entry { id: "update-available", text: "Update available: 1.3.0".into() }
    );
    assert_eq!(menu.tooltip, "Quest Tracker (1.3.0 available)");
    // What the glue rebuilds the tray from is the view the app sent last.
    assert_eq!(h.emitted().last(), Some(&available));
    // A failed check keeps it; an up-to-date answer removes it.
    let failed = h.manual(Err(TransportError::NotSecure));
    assert_eq!(ids(&failed)[0], TRAY_UPDATE_AVAILABLE);
    let current = h.manual(releases(&["1.2.0"]));
    assert_eq!(ids(&current), plain);
    assert_eq!(tray_menu(&current).tooltip, "Quest Tracker");
    // Every id maps to its action, and nothing else does.
    assert_eq!(TrayAction::from_id(TRAY_UPDATE_AVAILABLE), Some(TrayAction::OpenReleasePage));
    assert_eq!(TrayAction::from_id(TRAY_CHECK_UPDATES), Some(TrayAction::CheckForUpdates));
    assert_eq!(TrayAction::from_id(TRAY_SETTINGS), Some(TrayAction::OpenSettings));
    assert_eq!(TrayAction::from_id(TRAY_QUIT), Some(TrayAction::Quit));
    assert_eq!(TrayAction::from_id("Quit Quest Tracker"), None);
}

#[test]
fn updates_failure_reasons_are_one_line_each() {
    let h = harness(Setup::default());
    let cases: Vec<(Reply, &str)> = vec![
        (Err(TransportError::Timeout), "GitHub did not answer in time"),
        (Err(TransportError::Unreachable), "GitHub could not be reached"),
        (Err(TransportError::NotSecure), "the connection to GitHub was not secure"),
        (Err(TransportError::TooLarge), "GitHub's answer was too large"),
        (Err(TransportError::Panicked), "the request stopped unexpectedly"),
        (Err(TransportError::Other(12_175)), "the request failed (error 12175)"),
        (Err(TransportError::Unsupported), "this build cannot check for updates"),
        (status(403), "GitHub refused the check for now (rate limit), try again later"),
        (status(429), "GitHub refused the check for now (rate limit), try again later"),
        (status(500), "GitHub answered 500"),
        (status(301), "GitHub answered 301"),
        (
            Ok(Answer { status: 200, body: b"<html>not json</html>".to_vec() }),
            "GitHub's answer could not be read",
        ),
    ];
    for (reply, reason) in cases {
        let view = h.manual(reply);
        assert_eq!(view.update.state, UpdateState::Failed);
        assert_eq!(view.update.text, format!("Could not check for updates: {reason}."));
        assert!(!view.update.text.contains('\n'));
        assert_eq!(
            h.update_lines().last().map(String::as_str),
            Some(format!("updates: check failed: {reason}").as_str())
        );
    }
    // Nothing of a body is ever logged.
    let log = h.diag_text();
    assert!(!log.contains("never shown") && !log.contains("<html>"), "{log}");
}

#[test]
fn updates_without_a_transport_fail_without_a_request() {
    let h = harness(Setup {
        transport: false,
        ..auto_on()
    });
    h.app.start();
    let view = h.app.check_for_updates();
    assert_eq!(view.update.state, UpdateState::Failed);
    assert_eq!(view.update.text, "Could not check for updates: this build cannot check for updates.");
    assert_eq!(h.calls(), 0);
    assert!(h.update_lines().contains(&"updates: check failed: this build cannot check for updates".to_string()));
    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));
}

// ---------------------------------------------------------------------------
// The app-settings fixtures.

fn update_fixtures() -> bool {
    std::env::var("QT_UPDATE_FIXTURES").is_ok_and(|v| v == "1")
}

/// One view per state, from one run of the app through the fake.
fn app_settings_views() -> Vec<(&'static str, AppSettingsView)> {
    let h = harness(Setup::default());
    let mut out = vec![("idle", h.app.get_app_settings())];
    h.hold();
    out.push(("checking", h.app.check_for_updates()));
    h.release();
    h.wait_calls(1);
    h.wait_ended();
    h.app.update_settings(&json!({ "check_updates": true })).unwrap();
    // Not started: no worker ticks, so the setting starts nothing here.
    out.push(("available", h.app.get_app_settings()));
    out.push(("failed", h.manual(status(500))));
    out.push(("up-to-date", h.manual(releases(&["1.2.0", "1.0.0"]))));
    out.push(("no-release", h.manual(releases(&[]))));
    assert_eq!(h.calls(), 4);
    out
}

fn fixture_dir() -> PathBuf {
    root().join("fixtures/app-settings")
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

#[test]
fn app_settings_fixtures_are_current() {
    let views = app_settings_views();
    // One per state, each the state it is named for, built from the
    // synthetic version.
    for (name, view) in &views {
        let state = serde_json::to_value(view.update.state).unwrap();
        assert_eq!(state, json!(name), "{name}");
        assert_eq!(view.version, RUNNING.to_string());
    }
    let by = |n: &str| &views.iter().find(|(name, _)| *name == n).unwrap().1;
    assert_eq!(by("available").update.newer.as_deref(), Some(NEWER.to_string().as_str()));
    assert_eq!(by("failed").update.newer.as_deref(), Some("1.3.0"), "failed keeps the newer");
    assert_eq!(by("up-to-date").update.newer, None);

    let dir = fixture_dir();
    if update_fixtures() {
        fs::create_dir_all(&dir).unwrap();
        for (name, view) in &views {
            let text = serde_json::to_string_pretty(&serde_json::to_value(view).unwrap()).unwrap();
            fs::write(dir.join(format!("{name}.json")), text + "\n").unwrap();
        }
    }
    let mut want: Vec<String> = views.iter().map(|(n, _)| format!("{n}.json")).collect();
    want.sort();
    assert_eq!(json_files(&dir), want, "fixtures/app-settings holds exactly one file per state");
    let mut stale = Vec::new();
    for (name, view) in &views {
        let path = dir.join(format!("{name}.json"));
        let text = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("{}: {e} (run with QT_UPDATE_FIXTURES=1)", path.display()));
        let on_disk: Value = serde_json::from_str(&text).unwrap();
        if on_disk != serde_json::to_value(view).unwrap() {
            stale.push(*name);
        }
    }
    assert!(
        stale.is_empty(),
        "stale app-settings fixtures {stale:?}: run with QT_UPDATE_FIXTURES=1 and review the diff"
    );
}

// ---------------------------------------------------------------------------
// The release list.

#[test]
fn releases_pick_the_newest_published_quest_tracker_tag() {
    // 1.10.0 is newer than 1.9.3 as numbers (not as text), and its
    // `html_url` points elsewhere, which nothing reads.
    assert_eq!(newest_release(&releases_fixture()), Ok(Some(Version::new(1, 10, 0))));
    // The order of the list does not matter.
    let mut list: Vec<Value> = serde_json::from_slice(&releases_fixture()).unwrap();
    list.reverse();
    assert_eq!(
        newest_release(&serde_json::to_vec(&list).unwrap()),
        Ok(Some(Version::new(1, 10, 0)))
    );
    // Against the running version: newer is available, equal or older is
    // up to date.
    assert!(Version::new(1, 10, 0) > Version::new(1, 9, 3));
    assert!(Version::new(2, 0, 0) > Version::new(1, 99, 99));
    assert_eq!(Version::new(1, 10, 0).tag(), "quest-tracker-v1.10.0");
    assert_eq!(Version::from_tag("quest-tracker-v0.1.0"), Some(Version::new(0, 1, 0)));
    // An empty list (the repository today) has none.
    assert_eq!(newest_release(b"[]"), Ok(None));
    // The running build's own version parses.
    assert_eq!(Version::running().to_string(), env!("CARGO_PKG_VERSION").split('-').next().unwrap());
}

#[test]
fn releases_ignore_drafts_prereleases_other_tools_and_bad_tags() {
    let one = |entry: Value| newest_release(&serde_json::to_vec(&json!([entry])).unwrap());
    let published = |tag: &str| json!({ "tag_name": tag, "draft": false, "prerelease": false });
    // Positive control: a plain published tag is found.
    assert_eq!(one(published("quest-tracker-v2.0.0")), Ok(Some(Version::new(2, 0, 0))));
    // Each of these is skipped, never an error.
    let skipped = [
        json!({ "tag_name": "quest-tracker-v2.0.0", "draft": true, "prerelease": false }),
        json!({ "tag_name": "quest-tracker-v2.0.0", "draft": false, "prerelease": true }),
        json!({ "tag_name": "quest-tracker-v2.0.0", "prerelease": false }),
        json!({ "tag_name": "quest-tracker-v2.0.0", "draft": "false", "prerelease": false }),
        published("log-reader-v2.0.0"),
        published("v2.0.0"),
        published("Quest-Tracker-v2.0.0"),
        published("quest-tracker-v2.0"),
        published("quest-tracker-v2.0.0.1"),
        published("quest-tracker-v2.0.0-rc1"),
        published("quest-tracker-v+2.0.0"),
        published("quest-tracker-v 2.0.0"),
        published("quest-tracker-v2..0"),
        published("quest-tracker-vX.Y.Z"),
        published("quest-tracker-v99999999999999999999.0.0"),
        json!({ "tag_name": 200, "draft": false, "prerelease": false }),
        json!({ "draft": false, "prerelease": false }),
        json!("quest-tracker-v2.0.0"),
        Value::Null,
    ];
    for entry in skipped {
        assert_eq!(one(entry.clone()), Ok(None), "{entry}");
    }
    // The fixture's draft (1.12.0) and prerelease (1.11.0) are newer than
    // what it picks, and are not picked.
    assert_eq!(newest_release(&releases_fixture()), Ok(Some(Version::new(1, 10, 0))));
}

#[test]
fn releases_refuse_an_unreadable_answer() {
    for body in [
        &b""[..],
        b"not json",
        b"[{\"tag_name\": \"quest-tracker-v2.0.0\"",
        b"{\"message\": \"Not Found\"}",
        b"\"quest-tracker-v2.0.0\"",
        b"null",
        b"\xff\xfe[]",
    ] {
        assert_eq!(newest_release(body), Err(Unreadable), "{}", String::from_utf8_lossy(body));
    }
    // Through the app: a failed check, with one line saying so.
    let h = harness(Setup::default());
    let view = h.manual(Ok(Answer {
        status: 200,
        body: b"{\"message\": \"Not Found\"}".to_vec(),
    }));
    assert_eq!(view.update.state, UpdateState::Failed);
    assert_eq!(view.update.text, "Could not check for updates: GitHub's answer could not be read.");
}
