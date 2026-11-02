//! The update check and the in-app install in the shell (Context "The
//! update checker in qt-app", "The privacy invariant and its proof", "The
//! install flow, threads and failure states"), driven by a fake transport
//! that records every call (the release list and each download) and can be
//! held on a gate, a fake clock, a fake emitter that records the Settings
//! window's views and a fake window that records the URLs it opens and the
//! restarts it is asked for. Nothing here touches the network.
//!
//! Baseline: with the defaults, an old `settings.json` without the key, a
//! corrupt one or no store at all, the transport is never called across
//! start, commands and 48 simulated hours of ticks, and no `updates:` line is
//! written; a manual check through the same fake then makes exactly one call
//! (the positive control). Target: a manual check makes one request; with
//! `check_updates` on, one at launch and then one a day, one an hour after a
//! failed automatic check, none once it is turned off; a check never blocks
//! a command or the worker, runs one at a time, and is not waited for at
//! shutdown. An install happens only from the `install_update` command, even
//! with the automatic check on for 48 simulated hours; a verified one swaps
//! the exe and restarts through the quit path; a failed one shows its reason
//! and *Try again*, with the running exe untouched. An install keeps the
//! running exe's own name (`Quest Tracker.exe` for a build since 0.2.0)
//! while it downloads the release's `quest-tracker.exe`.
//!
//! The install's signature fixtures are `fixtures/updates/signing/`, made
//! with a throwaway key for the file name `quest-tracker.exe` and version
//! 1.3.0, so the real product's name check passes.

use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use qt_app::updates::{
    newest_release, running_version, Answer, AssetKind, DownloadError, InstallState,
    TransportError, Unreadable, PRODUCT, TAG_PREFIX, TRAY_CHECK_UPDATES, TRAY_QUIT, TRAY_SETTINGS,
    TRAY_UPDATE_AVAILABLE,
};
use qt_app::{
    tray_menu, App, AppConfig, AppSettingsView, Clock, Emitter, Mode, MonitorMetrics, PanelConfig,
    Placement, QuitFrom, TrayAction, TrayItem, UpdateConfig, UpdateState, UpdateTransport, Version,
    Window, SHUTDOWN_TIMEOUT,
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
/// The bytes of the exe the harness's app runs from, which a failed install
/// must never touch.
const RUNNING_BYTES: &[u8] = b"the running quest-tracker.exe of the test";

// ---------------------------------------------------------------------------
// The fakes.

type Reply = Result<Answer, TransportError>;

/// The transport's shared record: how many release-list calls began and
/// returned, the replies queued (else `fallback`), and the gate a call waits
/// on while it is closed; every download asked for, what it serves, and the
/// gate the exe's download waits on before its first byte.
struct Wire {
    calls: AtomicUsize,
    returned: AtomicUsize,
    replies: Mutex<VecDeque<Reply>>,
    fallback: Mutex<Reply>,
    open: Mutex<bool>,
    opened: Condvar,
    downloads: Mutex<Vec<(Version, AssetKind)>>,
    exe: Mutex<Vec<u8>>,
    sig: Mutex<Vec<u8>>,
    exe_error: Mutex<Option<DownloadError>>,
    exe_open: Mutex<bool>,
    /// The exe's download is waiting on its closed gate.
    exe_waiting: AtomicBool,
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

    /// Serves the asset in 1 KiB reads, as `ep-net` streams one: the limit,
    /// the cancel flag between reads and a progress call after each.
    fn download(
        &self,
        version: Version,
        asset: AssetKind,
        limit: u64,
        sink: &mut dyn std::io::Write,
        progress: &mut dyn FnMut(u64, Option<u64>),
        cancel: &AtomicBool,
    ) -> Result<u64, DownloadError> {
        let w = &self.0;
        w.downloads.lock().unwrap().push((version, asset));
        let data = match asset {
            AssetKind::Signature => w.sig.lock().unwrap().clone(),
            AssetKind::Exe => {
                if let Some(e) = *w.exe_error.lock().unwrap() {
                    return Err(e);
                }
                while !*w.exe_open.lock().unwrap() {
                    w.exe_waiting.store(true, Ordering::SeqCst);
                    if cancel.load(Ordering::Acquire) {
                        w.exe_waiting.store(false, Ordering::SeqCst);
                        return Err(DownloadError::Cancelled);
                    }
                    std::thread::sleep(Duration::from_millis(2));
                }
                w.exe_waiting.store(false, Ordering::SeqCst);
                w.exe.lock().unwrap().clone()
            }
        };
        if data.len() as u64 > limit {
            return Err(DownloadError::Transport(TransportError::TooLarge));
        }
        let total = Some(data.len() as u64);
        let mut written = 0u64;
        for chunk in data.chunks(1024) {
            if cancel.load(Ordering::Acquire) {
                return Err(DownloadError::Cancelled);
            }
            sink.write_all(chunk)
                .map_err(|e| DownloadError::Write(e.kind()))?;
            written += chunk.len() as u64;
            progress(written, total);
        }
        Ok(written)
    }
}

#[derive(Default)]
struct Recorded {
    app_settings: Mutex<Vec<AppSettingsView>>,
    opened: Mutex<Vec<String>>,
    restarts: AtomicUsize,
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
    fn restart(&self) {
        self.0.restarts.fetch_add(1, Ordering::SeqCst);
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
// Answers and fixtures.

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn signing(name: &str) -> Vec<u8> {
    let p = root().join("fixtures/updates/signing").join(name);
    fs::read(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

/// The exe a signed release of 1.3.0 serves.
fn payload() -> Vec<u8> {
    signing("payload.bin")
}

/// Its good signature: `file:quest-tracker.exe`, `version:1.3.0`.
fn good_signature() -> Vec<u8> {
    signing("payload.bin.minisig")
}

/// The test public key's `RW...` line, injected through `UpdateConfig`.
fn test_key() -> String {
    String::from_utf8(signing("test-key.pub"))
        .unwrap()
        .lines()
        .nth(1)
        .unwrap()
        .to_string()
}

/// One published Quest Tracker release; a signed one lists the exe and its
/// `.minisig` as GitHub does.
fn entry(version: &str, signed: bool) -> Value {
    let mut e = json!({ "tag_name": format!("{TAG_PREFIX}{version}"), "draft": false, "prerelease": false });
    if signed {
        e["assets"] = json!([
            { "name": "quest-tracker.exe", "size": payload().len(), "state": "uploaded" },
            { "name": "quest-tracker.exe.minisig", "size": good_signature().len(), "state": "uploaded" },
        ]);
    }
    e
}

fn list(versions: &[&str], signed: bool) -> Reply {
    let list: Vec<Value> = versions.iter().map(|v| entry(v, signed)).collect();
    Ok(Answer {
        status: 200,
        body: serde_json::to_vec(&list).unwrap(),
    })
}

/// A 200 answer listing signed Quest Tracker releases of `versions`.
fn releases(versions: &[&str]) -> Reply {
    list(versions, true)
}

/// The same, with no exe or signature asset listed.
fn unsigned_releases(versions: &[&str]) -> Reply {
    list(versions, false)
}

fn status(code: u16) -> Reply {
    Ok(Answer {
        status: code,
        body: br#"{"message":"a body that is never shown"}"#.to_vec(),
    })
}

fn releases_fixture() -> Vec<u8> {
    fs::read(root().join("fixtures/updates/releases.json")).expect("fixtures/updates/releases.json")
}

/// The newest version of a release list, as the quest tracker reads it.
fn newest(body: &[u8]) -> Result<Option<Version>, Unreadable> {
    newest_release(&PRODUCT, body).map(|r| r.map(|r| r.version))
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
    /// The file name of the exe the app runs from. A release's asset is
    /// always `quest-tracker.exe`; a build since 0.2.0 is `Quest Tracker.exe`.
    exe_name: &'static str,
}

impl Default for Setup {
    fn default() -> Setup {
        Setup {
            saved: Saved::Nothing,
            transport: true,
            open: true,
            exe_name: "quest-tracker.exe",
        }
    }
}

struct Harness {
    _dir: tempfile::TempDir,
    root: PathBuf,
    /// The exe the app runs from, in a folder of its own.
    exe: PathBuf,
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
    let exe = root.join("app").join(setup.exe_name);
    fs::create_dir_all(exe.parent().unwrap()).expect("app folder");
    fs::write(&exe, RUNNING_BYTES).expect("running exe written");
    let wire = Arc::new(Wire {
        calls: AtomicUsize::new(0),
        returned: AtomicUsize::new(0),
        replies: Mutex::new(VecDeque::new()),
        fallback: Mutex::new(releases(&["1.3.0"])),
        open: Mutex::new(setup.open),
        opened: Condvar::new(),
        downloads: Mutex::new(Vec::new()),
        exe: Mutex::new(payload()),
        sig: Mutex::new(good_signature()),
        exe_error: Mutex::new(None),
        exe_open: Mutex::new(true),
        exe_waiting: AtomicBool::new(false),
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
            exe_path: Some(exe.clone()),
            public_key: Some(test_key()),
        },
    );
    Harness {
        _dir: dir,
        root,
        exe,
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

    fn downloads(&self) -> Vec<(Version, AssetKind)> {
        self.wire.downloads.lock().unwrap().clone()
    }

    /// Closes or opens the gate the exe's download waits on.
    fn exe_gate(&self, open: bool) {
        *self.wire.exe_open.lock().unwrap() = open;
    }

    fn restarts(&self) -> usize {
        self.rec.restarts.load(Ordering::SeqCst)
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

    /// The lines from `marker` on, without the log's own prefix.
    fn lines_from(&self, marker: &str) -> Vec<String> {
        self.diag_text()
            .lines()
            .filter_map(|l| l.find(marker).map(|i| l[i..].to_string()))
            .collect()
    }

    /// The `updates:` lines.
    fn update_lines(&self) -> Vec<String> {
        self.lines_from("updates: ")
    }

    /// The `updates:` lines of installs.
    fn install_lines(&self) -> Vec<String> {
        self.update_lines()
            .into_iter()
            .filter(|l| {
                l.starts_with("updates: install")
                    || l.starts_with("updates: signature")
                    || l.starts_with("updates: replaced")
            })
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

    /// Waits for the install to reach `state` and its view to be sent; that
    /// view.
    fn wait_install(&self, state: InstallState) -> AppSettingsView {
        wait_until(&format!("the install reaches {state:?} and is announced"), || {
            let view = self.app.get_app_settings();
            view.install.state == state && self.emitted().last() == Some(&view)
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

    /// The files of the exe's folder.
    fn app_folder(&self) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(self.exe.parent().unwrap())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    /// The running exe is as it was, alone in its folder.
    fn assert_untouched(&self) {
        assert_eq!(fs::read(&self.exe).unwrap(), RUNNING_BYTES, "the running exe changed");
        let name = self.exe.file_name().unwrap().to_string_lossy().into_owned();
        assert_eq!(self.app_folder(), vec![name]);
        assert_eq!(self.restarts(), 0, "a restart was asked for");
        assert!(!self.app.restart_requested());
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
    assert_eq!(view.install.state, InstallState::None);
    h.ticks(0, 48 * HOUR);
    let_the_worker_tick();
    assert_eq!(h.calls(), 0, "a request nobody asked for");
    assert!(h.downloads().is_empty(), "a download nobody asked for");
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

#[test]
fn theme_patch_updates_both_views_without_starting_updater_work() {
    let h = harness(Setup::default());
    h.app.start();
    let before = h.app.get_app_settings();
    let overlay = h.app.update_settings(&json!({"theme":"claude"})).unwrap();
    let ordinary = h.app.get_app_settings();
    assert_eq!(serde_json::to_value(overlay.settings).unwrap()["theme"], json!("claude"));
    assert_eq!(serde_json::to_value(&ordinary).unwrap()["theme"], json!("claude"));
    assert_eq!(ordinary.update, before.update);
    assert_eq!(ordinary.install, before.install);
    assert_eq!(ordinary.check_updates, before.check_updates);
    assert_eq!(h.emitted().last(), Some(&ordinary));
    let count = h.emitted().len();
    assert!(h.app.update_settings(&json!({"theme":null})).is_err());
    assert_eq!(h.emitted().len(), count);
    assert_eq!(h.app.get_app_settings(), ordinary);
    h.ticks(0, 48 * HOUR);
    assert_eq!(h.calls(), 0);
    assert!(h.downloads().is_empty());
    assert!(h.update_lines().is_empty());
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
    assert!(h.downloads().is_empty());
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
    quick("install_update", &|| {
        // Refused while a check runs.
        assert_eq!(h.app.install_update().update.state, UpdateState::Checking);
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
    assert!(h.downloads().is_empty());
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
    // And none starts any more, nor an install.
    h.app.check_for_updates();
    h.app.install_update();
    h.app.tick();
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(h.calls(), 1);
    assert!(h.downloads().is_empty());
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
    assert_eq!(TrayAction::from_id(TRAY_UPDATE_AVAILABLE), Some(TrayAction::OpenSettings));
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
// The install.

#[test]
fn updates_install_is_reached_only_from_the_command_target() {
    let h = harness(auto_on());
    h.app.start();
    // The automatic check at launch finds a signed newer release.
    h.wait_calls(1);
    let view = h.wait_ended();
    assert_eq!(view.update.state, UpdateState::Available);
    assert_eq!(view.install.state, InstallState::Ready);
    assert_eq!(view.install.text, "Install 1.3.0 and restart");
    // 48 simulated hours with the setting on: a check a day, never a download.
    h.ticks(30 * MINUTE, 24 * HOUR);
    h.wait_calls(2);
    h.wait_ended();
    h.ticks(24 * HOUR + 30 * MINUTE, 48 * HOUR);
    h.wait_calls(3);
    h.wait_ended();
    let_the_worker_tick();
    // Nor from any other command, or the tray's update entry (it opens the
    // Settings window).
    h.manual(releases(&["1.3.0"]));
    let _ = h.app.get_app_settings();
    let _ = h.app.cancel_update();
    h.app.update_settings(&json!({ "opacity": 0.6 })).unwrap();
    assert_eq!(TrayAction::from_id(TRAY_UPDATE_AVAILABLE), Some(TrayAction::OpenSettings));
    assert!(h.downloads().is_empty(), "{:?}", h.downloads());
    assert!(h.install_lines().is_empty(), "{:?}", h.install_lines());
    assert_eq!(h.app.get_app_settings().install.state, InstallState::Ready);
    h.assert_untouched();

    // Positive control: the command installs, through the same fake.
    let view = h.app.install_update();
    assert_ne!(view.install.state, InstallState::Ready, "the command started nothing");
    h.wait_install(InstallState::Restarting);
    assert_eq!(
        h.downloads(),
        vec![(NEWER, AssetKind::Signature), (NEWER, AssetKind::Exe)]
    );
    wait_until("the restart is asked for", || h.restarts() == 1);
    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));
}

#[test]
fn updates_install_restarts_through_the_quit_path_target() {
    let h = harness(Setup::default());
    h.app.start();
    h.app.update_settings(&json!({ "opacity": 0.5 })).unwrap();
    let ready = h.manual(releases(&["1.3.0"]));
    assert_eq!(ready.install.state, InstallState::Ready);
    let started = h.app.install_update();
    assert!(
        matches!(
            started.install.state,
            InstallState::Downloading | InstallState::Verifying | InstallState::Restarting
        ),
        "{started:?}"
    );
    let view = h.wait_install(InstallState::Restarting);
    assert_eq!(view.install.text, "Quest Tracker 1.3.0 is installed. Restarting\u{2026}");
    assert_eq!(view.install.progress, None);
    // The window saw it download and check the signature on the way.
    let states: Vec<InstallState> = h.emitted().iter().map(|v| v.install.state).collect();
    assert!(states.contains(&InstallState::Downloading), "{states:?}");
    assert!(states.contains(&InstallState::Verifying), "{states:?}");
    // The new exe is in place and the running one beside it; nothing staged.
    assert_eq!(fs::read(&h.exe).unwrap(), payload());
    assert_eq!(
        fs::read(h.exe.with_file_name("quest-tracker.exe.old")).unwrap(),
        RUNNING_BYTES
    );
    assert_eq!(h.app_folder(), lines(&["quest-tracker.exe", "quest-tracker.exe.old"]));
    // Through the quit path: one quit line, one restart asked of the shell,
    // and the relaunch recorded for its exit.
    wait_until("the restart is asked for", || h.restarts() == 1);
    assert!(h.app.restart_requested());
    assert_eq!(h.lines_from("quit: "), lines(&["quit: restarting into the update"]));
    assert_eq!(
        h.update_lines(),
        lines(&[
            "updates: check started (manual)",
            "updates: 1.3.0 available (running 1.2.0)",
            "updates: install of 1.3.0 started",
            "updates: signature of 1.3.0 checked",
            "updates: replaced the running exe with 1.3.0, restarting",
        ])
    );
    // The first quit request wins; nothing starts while it restarts.
    assert!(!h.app.request_quit(QuitFrom::Tray));
    h.app.install_update();
    h.app.check_for_updates();
    assert_eq!(h.calls(), 1);
    assert_eq!(h.downloads().len(), 2);
    assert_eq!(
        h.update_lines().last().map(String::as_str),
        Some("updates: install refused: the app is restarting")
    );
    // The shutdown the shell's exit runs saves, as any quit's does.
    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));
    assert_eq!(h.saved_settings()["opacity"], 0.5);
    assert_eq!(h.restarts(), 1);
    assert_eq!(h.lines_from("quit: ").len(), 1);
}

/// Baseline, before and after the build was renamed: an install keeps the
/// running exe's own name. A build since 0.2.0 runs as `Quest Tracker.exe`
/// while every release's asset stays `quest-tracker.exe`, signed under that
/// name; the update downloads the asset and swaps it in at the running path.
#[test]
fn updates_install_keeps_the_running_exe_name() {
    let h = harness(Setup {
        exe_name: "Quest Tracker.exe",
        ..Setup::default()
    });
    h.app.start();
    let ready = h.manual(releases(&["1.3.0"]));
    assert_eq!(ready.install.state, InstallState::Ready);
    h.app.install_update();
    h.wait_install(InstallState::Restarting);
    // The download asked for the release's asset and its signature, by the
    // product's asset name, not the running exe's.
    assert_eq!(
        h.downloads(),
        vec![(NEWER, AssetKind::Signature), (NEWER, AssetKind::Exe)]
    );
    assert_eq!(PRODUCT.exe_name, "quest-tracker.exe");
    assert_eq!(PRODUCT.signature_name(), "quest-tracker.exe.minisig");
    // The new bytes sit under the installed name, the old ones beside it.
    assert_eq!(h.exe.file_name().unwrap(), "Quest Tracker.exe");
    assert_eq!(fs::read(&h.exe).unwrap(), payload());
    assert_eq!(
        fs::read(h.exe.with_file_name("Quest Tracker.exe.old")).unwrap(),
        RUNNING_BYTES
    );
    assert_eq!(h.app_folder(), lines(&["Quest Tracker.exe", "Quest Tracker.exe.old"]));
    wait_until("the restart is asked for", || h.restarts() == 1);
    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));
}

#[test]
fn updates_install_failure_shows_the_reason_and_try_again() {
    let h = harness(Setup::default());
    h.app.start();
    // No newer version known: refused, nothing downloaded.
    let view = h.app.install_update();
    assert_eq!(view.install.state, InstallState::None);
    assert_eq!(view.install.text, "");
    // A newer release that is not signed: its one line, no install.
    let view = h.manual(unsigned_releases(&["1.3.0"]));
    assert_eq!(view.install.state, InstallState::Unsigned);
    assert_eq!(
        view.install.text,
        "1.3.0 is available but cannot be installed: the release is not signed."
    );
    assert_eq!(h.app.install_update().install.state, InstallState::Unsigned);
    assert!(h.downloads().is_empty());
    h.assert_untouched();

    // A failed download: its reason and Try again; the running exe untouched.
    assert_eq!(h.manual(releases(&["1.3.0"])).install.state, InstallState::Ready);
    *h.wire.exe_error.lock().unwrap() = Some(DownloadError::Status(404));
    h.app.install_update();
    let view = h.wait_install(InstallState::Failed);
    assert_eq!(view.install.text, "Could not install Quest Tracker 1.3.0: GitHub answered 404.");
    assert!(!view.install.text.contains('\n'));
    assert_eq!(view.update.state, UpdateState::Available, "the check part stays");
    h.assert_untouched();

    // Try again: the file does not match its signature.
    *h.wire.exe_error.lock().unwrap() = None;
    let mut tampered = payload();
    tampered[100] ^= 0x01;
    *h.wire.exe.lock().unwrap() = tampered;
    h.app.install_update();
    let view = h.wait_install(InstallState::Failed);
    assert_eq!(
        view.install.text,
        "Could not install Quest Tracker 1.3.0: the downloaded file does not match its signature."
    );
    h.assert_untouched();

    // A cancel goes back to the button and leaves nothing behind.
    *h.wire.exe.lock().unwrap() = payload();
    h.exe_gate(false);
    h.app.install_update();
    wait_until("the exe's download waits", || h.wire.exe_waiting.load(Ordering::SeqCst));
    let view = h.app.get_app_settings();
    assert_eq!(view.install.state, InstallState::Downloading);
    assert_eq!(view.install.text, "Downloading Quest Tracker 1.3.0\u{2026}");
    let progress = view.install.progress.expect("progress while downloading");
    assert_eq!((progress.bytes, progress.total), (0, Some(payload().len() as u64)));
    h.app.cancel_update();
    let view = h.wait_install(InstallState::Ready);
    assert_eq!(view.install.text, "Install 1.3.0 and restart");
    h.assert_untouched();
    h.exe_gate(true);

    // Try again, now with the release as it is: installed.
    h.app.install_update();
    h.wait_install(InstallState::Restarting);
    assert_eq!(fs::read(&h.exe).unwrap(), payload());
    wait_until("the restart is asked for", || h.restarts() == 1);
    assert_eq!(
        h.install_lines(),
        lines(&[
            "updates: install refused: no newer version is known",
            "updates: install refused: the release is not signed",
            "updates: install of 1.3.0 started",
            "updates: install failed: GitHub answered 404",
            "updates: install of 1.3.0 started",
            "updates: install failed: the downloaded file does not match its signature",
            "updates: install of 1.3.0 started",
            "updates: install cancelled",
            "updates: install of 1.3.0 started",
            "updates: signature of 1.3.0 checked",
            "updates: replaced the running exe with 1.3.0, restarting",
        ])
    );
    // No line names a URL, and nothing was opened.
    assert!(h.update_lines().iter().all(|l| !l.contains("://")), "{:?}", h.update_lines());
    assert!(h.opened().is_empty());
    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));
}

#[test]
fn updates_tray_update_entry_opens_settings() {
    let h = harness(Setup::default());
    let view = h.manual(releases(&["1.3.0"]));
    let menu = tray_menu(&view);
    assert_eq!(
        menu.items[0],
        TrayItem::Entry { id: "update-available", text: "Update available: 1.3.0".into() }
    );
    // The update entry opens the Settings window, where the install button is.
    assert_eq!(TrayAction::from_id(TRAY_UPDATE_AVAILABLE), Some(TrayAction::OpenSettings));
    // An unsigned release keeps the entry, and it opens the window too.
    let unsigned = h.manual(unsigned_releases(&["1.3.0"]));
    assert_eq!(ids(&unsigned)[0], TRAY_UPDATE_AVAILABLE);
    // No tray entry installs or opens a page.
    for item in tray_menu(&view).items.iter().chain(tray_menu(&unsigned).items.iter()) {
        if let TrayItem::Entry { id, .. } = item {
            match TrayAction::from_id(id) {
                Some(TrayAction::CheckForUpdates | TrayAction::OpenSettings | TrayAction::Quit) => {}
                None => panic!("tray entry {id:?} maps to nothing"),
            }
        }
    }
    assert!(h.downloads().is_empty());
    assert!(h.opened().is_empty());
    h.assert_untouched();
}

// ---------------------------------------------------------------------------
// The app-settings fixtures.

fn update_fixtures() -> bool {
    std::env::var("QT_UPDATE_FIXTURES").is_ok_and(|v| v == "1")
}

/// One view per update state, from one run of the app through the fake.
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

/// One view per install state, from another run.
fn install_views() -> Vec<(&'static str, AppSettingsView)> {
    let h = harness(Setup::default());
    let mut out = vec![("none", h.app.get_app_settings())];
    out.push(("unsigned", h.manual(unsigned_releases(&["1.3.0"]))));
    out.push(("ready", h.manual(releases(&["1.3.0"]))));
    // Held before the exe's first byte, so the progress is exact.
    h.exe_gate(false);
    h.app.install_update();
    wait_until("the exe's download waits", || h.wire.exe_waiting.load(Ordering::SeqCst));
    out.push(("downloading", h.app.get_app_settings()));
    h.app.cancel_update();
    h.wait_install(InstallState::Ready);
    h.exe_gate(true);
    *h.wire.exe_error.lock().unwrap() = Some(DownloadError::Status(404));
    h.app.install_update();
    out.push(("failed", h.wait_install(InstallState::Failed)));
    *h.wire.exe_error.lock().unwrap() = None;
    h.app.install_update();
    let restarting = h.wait_install(InstallState::Restarting);
    // The view sent between the download and the signature check.
    let verifying = h
        .emitted()
        .into_iter()
        .find(|v| v.install.state == InstallState::Verifying)
        .expect("a verifying view was sent");
    out.push(("verifying", verifying));
    out.push(("restarting", restarting));
    out
}

fn fixture_dir() -> PathBuf {
    root().join("fixtures/app-settings")
}

fn json_files(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|n| n.ends_with(".json"))
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

/// Writes (with `QT_UPDATE_FIXTURES=1`) and compares one folder of fixtures;
/// the names of the stale ones.
fn check_fixtures(dir: &Path, views: &[(&'static str, AppSettingsView)]) -> Vec<String> {
    if update_fixtures() {
        fs::create_dir_all(dir).unwrap();
        for (name, view) in views {
            let text = serde_json::to_string_pretty(&serde_json::to_value(view).unwrap()).unwrap();
            fs::write(dir.join(format!("{name}.json")), text + "\n").unwrap();
        }
    }
    let mut want: Vec<String> = views.iter().map(|(n, _)| format!("{n}.json")).collect();
    want.sort();
    assert_eq!(json_files(dir), want, "{} holds exactly one file per state", dir.display());
    let mut stale = Vec::new();
    for (name, view) in views {
        let path = dir.join(format!("{name}.json"));
        let text = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("{}: {e} (run with QT_UPDATE_FIXTURES=1)", path.display()));
        let on_disk: Value = serde_json::from_str(&text).unwrap();
        if on_disk != serde_json::to_value(view).unwrap() {
            stale.push(format!("{}/{name}", dir.file_name().unwrap().to_string_lossy()));
        }
    }
    stale
}

#[test]
fn app_settings_fixtures_are_current() {
    let views = app_settings_views();
    // One per update state, each the state it is named for, built from the
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
    assert_eq!(by("available").install.state, InstallState::Ready);
    assert_eq!(by("up-to-date").install.state, InstallState::None);

    // One per install state, in `install/`, each the state it is named for.
    let installs = install_views();
    for (name, view) in &installs {
        let state = serde_json::to_value(view.install.state).unwrap();
        assert_eq!(state, json!(name), "install/{name}");
        assert_eq!(view.version, RUNNING.to_string());
        assert_eq!(view.install.progress.is_some(), *name == "downloading", "install/{name}");
    }
    assert_eq!(installs.len(), 7);

    let mut stale = check_fixtures(&fixture_dir(), &views);
    stale.extend(check_fixtures(&fixture_dir().join("install"), &installs));
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
    assert_eq!(newest(&releases_fixture()), Ok(Some(Version::new(1, 10, 0))));
    // The order of the list does not matter.
    let mut list: Vec<Value> = serde_json::from_slice(&releases_fixture()).unwrap();
    list.reverse();
    assert_eq!(
        newest(&serde_json::to_vec(&list).unwrap()),
        Ok(Some(Version::new(1, 10, 0)))
    );
    // Against the running version: newer is available, equal or older is
    // up to date.
    assert!(Version::new(1, 10, 0) > Version::new(1, 9, 3));
    assert!(Version::new(2, 0, 0) > Version::new(1, 99, 99));
    assert_eq!(PRODUCT.tag(Version::new(1, 10, 0)), "quest-tracker-v1.10.0");
    assert_eq!(PRODUCT.version_of_tag("quest-tracker-v0.1.0"), Some(Version::new(0, 1, 0)));
    assert_eq!(PRODUCT.exe_name, "quest-tracker.exe");
    // An empty list (the repository today) has none.
    assert_eq!(newest(b"[]"), Ok(None));
    // The running build's own version parses.
    assert_eq!(running_version().to_string(), env!("CARGO_PKG_VERSION").split('-').next().unwrap());
}

#[test]
fn releases_ignore_drafts_prereleases_other_tools_and_bad_tags() {
    let one = |entry: Value| newest(&serde_json::to_vec(&json!([entry])).unwrap());
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
        published("waystone-roller-v2.0.0"),
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
    assert_eq!(newest(&releases_fixture()), Ok(Some(Version::new(1, 10, 0))));
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
        assert_eq!(newest(body), Err(Unreadable), "{}", String::from_utf8_lossy(body));
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
