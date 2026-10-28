//! The update check and the in-app install, driven through `App` with a
//! recording fake shell, a fake transport that serves a release list and the
//! two release assets (each held on a gate when a test asks), a fake clock
//! and a store and an exe in a temporary folder. Nothing here touches the
//! network.
//!
//! Baseline: with the defaults, or a 0.1.0 `settings.json` without the key,
//! the transport is never called across start, every command and 48
//! simulated hours of ticks, and no `updates:` line is written; a manual
//! check through the same fake then makes exactly one call (the positive
//! control). Target: a manual check makes one request; with `check_updates`
//! on, one at start and then one a day; an install starts only from the
//! `install_update` command, never from a check or a tick; a finished
//! install asks the shell to quit, and the quit path's shutdown still saves.
//!
//! The signed release is the shared signature fixture
//! (`apps/shared/fixtures/signing/`): its exe asset is `payload.bin`, signed
//! for 1.3.0 with the test key, so the product here is the waystone roller
//! with that exe name, and the key is injected through `UpdateConfig`.

mod common;

use std::collections::VecDeque;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use common::*;
use serde_json::{json, Value};
use wr_app::{
    Answer, App, AppConfig, AppSettingsView, AssetKind, Clock, DownloadError, InstallState,
    Product, Store, TransportError, UpdateConfig, UpdateSetup, UpdateState, UpdateTransport,
    Version, View, WAYSTONE_ROLLER,
};

const HOUR: u64 = 60 * 60 * 1_000;
const MINUTE: u64 = 60 * 1_000;
/// The running version every harness gives the app: synthetic, never the
/// crate's, so a release bump changes nothing here.
const RUNNING: Version = Version::new(1, 2, 0);
/// The version the fixture signature names.
const NEWER: Version = Version::new(1, 3, 0);
/// The waystone roller, with the exe asset the fixture signature names.
const PRODUCT: Product = Product {
    name: WAYSTONE_ROLLER.name,
    tag_prefix: WAYSTONE_ROLLER.tag_prefix,
    exe_name: "payload.bin",
};
/// The running exe's bytes, which only a finished install replaces.
const RUNNING_BYTES: &[u8] = b"the running waystone roller, which a failed install never touches";

// ---------------------------------------------------------------------------
// The signature fixtures.

fn signing(name: &str) -> Vec<u8> {
    let p = app_root().join("../shared/fixtures/signing").join(name);
    fs::read(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

fn payload() -> Vec<u8> {
    signing("payload.bin")
}

fn test_key() -> String {
    String::from_utf8(signing("test-key.pub"))
        .unwrap()
        .lines()
        .nth(1)
        .unwrap()
        .to_string()
}

// ---------------------------------------------------------------------------
// Answers.

type Reply = Result<Answer, TransportError>;

/// A 200 release list of `versions`; the newest lists the payload and its
/// signature when `signed`, the payload alone otherwise.
fn list(versions: &[Version], signed: bool) -> Reply {
    let newest = versions.iter().max().copied();
    let entries: Vec<Value> = versions
        .iter()
        .map(|v| {
            let mut assets = vec![json!({
                "name": PRODUCT.exe_name, "size": payload().len(), "state": "uploaded",
                "browser_download_url": "https://example.invalid/never-read",
            })];
            if signed {
                assets.push(json!({
                    "name": PRODUCT.signature_name(),
                    "size": signing("payload.bin.minisig").len(),
                    "state": "uploaded",
                }));
            }
            json!({
                "tag_name": PRODUCT.tag(*v),
                "draft": false,
                "prerelease": false,
                "assets": if Some(*v) == newest { assets } else { Vec::new() },
            })
        })
        .collect();
    Ok(Answer {
        status: 200,
        body: serde_json::to_vec(&entries).unwrap(),
    })
}

fn signed_newer() -> Reply {
    list(&[NEWER, RUNNING], true)
}

fn status(code: u16) -> Reply {
    Ok(Answer {
        status: code,
        body: br#"{"message":"a body that is never shown"}"#.to_vec(),
    })
}

// ---------------------------------------------------------------------------
// The fakes.

#[derive(Default)]
struct Gates {
    /// The release list's request waits while this is false.
    list_held: bool,
    /// Each download waits, before it writes anything, while this is true.
    downloads_held: bool,
}

/// The transport's shared record and script.
struct Wire {
    list_calls: AtomicUsize,
    list_returned: AtomicUsize,
    replies: Mutex<VecDeque<Reply>>,
    fallback: Mutex<Reply>,
    /// Every download asked for, in order.
    downloads: Mutex<Vec<AssetKind>>,
    sig: Mutex<Vec<u8>>,
    exe: Mutex<Vec<u8>>,
    /// A download is waiting at the held gate.
    download_waiting: AtomicBool,
    gates: Mutex<Gates>,
    turned: Condvar,
}

impl Wire {
    fn new() -> Arc<Wire> {
        Arc::new(Wire {
            list_calls: AtomicUsize::new(0),
            list_returned: AtomicUsize::new(0),
            replies: Mutex::new(VecDeque::new()),
            fallback: Mutex::new(list(&[RUNNING], true)),
            downloads: Mutex::new(Vec::new()),
            sig: Mutex::new(signing("payload.bin.minisig")),
            exe: Mutex::new(payload()),
            download_waiting: AtomicBool::new(false),
            gates: Mutex::new(Gates::default()),
            turned: Condvar::new(),
        })
    }

    fn gate(&self, f: impl FnOnce(&mut Gates)) {
        f(&mut self.gates.lock().unwrap());
        self.turned.notify_all();
    }
}

struct FakeTransport(Arc<Wire>);

impl UpdateTransport for FakeTransport {
    fn fetch_releases(&self) -> Result<Answer, TransportError> {
        let w = &self.0;
        w.list_calls.fetch_add(1, Ordering::SeqCst);
        let mut g = w.gates.lock().unwrap();
        while g.list_held {
            g = w.turned.wait(g).unwrap();
        }
        drop(g);
        let reply = w
            .replies
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| w.fallback.lock().unwrap().clone());
        w.list_returned.fetch_add(1, Ordering::SeqCst);
        reply
    }

    fn download(
        &self,
        version: Version,
        asset: AssetKind,
        limit: u64,
        sink: &mut dyn Write,
        progress: &mut dyn FnMut(u64, Option<u64>),
        cancel: &AtomicBool,
    ) -> Result<u64, DownloadError> {
        let w = &self.0;
        assert_eq!(version, NEWER, "a download of another version");
        w.downloads.lock().unwrap().push(asset);
        let mut g = w.gates.lock().unwrap();
        while g.downloads_held {
            w.download_waiting.store(true, Ordering::SeqCst);
            if cancel.load(Ordering::SeqCst) {
                w.download_waiting.store(false, Ordering::SeqCst);
                return Err(DownloadError::Cancelled);
            }
            g = w.turned.wait_timeout(g, Duration::from_millis(10)).unwrap().0;
        }
        w.download_waiting.store(false, Ordering::SeqCst);
        drop(g);
        let data = match asset {
            AssetKind::Signature => w.sig.lock().unwrap().clone(),
            AssetKind::Exe => w.exe.lock().unwrap().clone(),
        };
        let total = Some(data.len() as u64);
        let mut written = 0u64;
        for chunk in data.chunks(8 * 1024) {
            if cancel.load(Ordering::SeqCst) {
                return Err(DownloadError::Cancelled);
            }
            if written + chunk.len() as u64 > limit {
                return Err(DownloadError::Transport(TransportError::TooLarge));
            }
            sink.write_all(chunk)
                .map_err(|e| DownloadError::Write(e.kind()))?;
            written += chunk.len() as u64;
            progress(written, total);
        }
        Ok(written)
    }
}

struct FakeClock(Arc<AtomicU64>);

impl Clock for FakeClock {
    fn now_ms(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}

// ---------------------------------------------------------------------------
// The harness.

#[derive(Default)]
struct Setup {
    /// The `settings.json` the store holds when the app is made.
    saved: Option<Value>,
    /// The list's request starts held.
    list_held: bool,
}

fn auto_on() -> Setup {
    Setup {
        saved: Some(json!({ "schema": 1, "hotkey": "Ctrl+Alt+W", "check_updates": true })),
        ..Setup::default()
    }
}

struct Harness {
    _tmp: tempfile::TempDir,
    store: PathBuf,
    exe: PathBuf,
    shell: FakeShell,
    wire: Arc<Wire>,
    now: Arc<AtomicU64>,
    app: Arc<App>,
}

/// An app over the fixture mods, not started, with the fake transport, the
/// fake clock at 0, the test key, the running exe in a folder of its own
/// and no ticker thread: the tests tick it themselves.
fn harness(setup: Setup) -> Harness {
    let tmp = tempfile::tempdir().unwrap();
    let store = tmp.path().join("store");
    let bin = tmp.path().join("bin");
    fs::create_dir_all(&store).unwrap();
    fs::create_dir_all(&bin).unwrap();
    if let Some(saved) = &setup.saved {
        fs::write(store.join("settings.json"), serde_json::to_vec(saved).unwrap()).unwrap();
    }
    let exe = bin.join("waystone-roller.exe");
    fs::write(&exe, RUNNING_BYTES).unwrap();
    let wire = Wire::new();
    wire.gate(|g| g.list_held = setup.list_held);
    let now = Arc::new(AtomicU64::new(0));
    let shell = FakeShell::default();
    let app = App::with_updates(
        AppConfig {
            data: fixture_data(),
            store: Store::open(&store),
            shell: Box::new(shell.clone()),
        },
        UpdateSetup {
            config: UpdateConfig {
                transport: Some(Box::new(FakeTransport(Arc::clone(&wire)))),
                running: Some(RUNNING),
                exe_path: Some(exe.clone()),
                public_key: Some(test_key()),
            },
            product: PRODUCT,
            clock: Box::new(FakeClock(Arc::clone(&now))),
            tick_every: None,
        },
    );
    Harness {
        _tmp: tmp,
        store,
        exe,
        shell,
        wire,
        now,
        app,
    }
}

fn wait_until(what: &str, pred: impl Fn() -> bool) {
    let start = Instant::now();
    while !pred() {
        assert!(start.elapsed() < Duration::from_secs(10), "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn lines(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

impl Harness {
    fn calls(&self) -> usize {
        self.wire.list_calls.load(Ordering::SeqCst)
    }

    fn downloads(&self) -> Vec<AssetKind> {
        self.wire.downloads.lock().unwrap().clone()
    }

    fn reply(&self, reply: Reply) {
        self.wire.replies.lock().unwrap().push_back(reply);
    }

    fn hold_list(&self, held: bool) {
        self.wire.gate(|g| g.list_held = held);
    }

    fn hold_downloads(&self, held: bool) {
        self.wire.gate(|g| g.downloads_held = held);
    }

    /// One tick of the ticker at `t`, with the clock at `t`.
    fn at(&self, t: u64) {
        self.now.store(t, Ordering::SeqCst);
        self.app.tick_updates(t);
    }

    /// Ticks every ten minutes from `from` to `to`, both included.
    fn ticks(&self, from: u64, to: u64) {
        let mut t = from;
        while t <= to {
            self.at(t);
            t += 10 * MINUTE;
        }
        self.at(to);
    }

    fn wait_calls(&self, n: usize) {
        wait_until(&format!("{n} release list calls"), || self.calls() >= n);
    }

    /// Waits until every started request returned and its check ended: the
    /// last view sent is past checking. The app logs the outcome before it
    /// sends that view, and sends a checking view before any request.
    fn wait_ended(&self) -> AppSettingsView {
        wait_until("the check to end", || {
            self.wire.list_returned.load(Ordering::SeqCst) == self.calls()
                && self.app.get_app_settings().update.state != UpdateState::Checking
                && self
                    .emitted()
                    .last()
                    .is_some_and(|v| v.update.state != UpdateState::Checking)
        });
        self.app.get_app_settings()
    }

    /// A manual check answered with `reply`, waited for.
    fn manual(&self, reply: Reply) -> AppSettingsView {
        self.reply(reply);
        let before = self.calls();
        self.app.check_for_updates();
        self.wait_calls(before + 1);
        self.wait_ended()
    }

    fn update_lines(&self) -> Vec<String> {
        diag_lines(&self.store)
            .into_iter()
            .filter(|l| l.starts_with("updates:"))
            .collect()
    }

    fn emitted(&self) -> Vec<AppSettingsView> {
        self.shell.rec().app_settings.clone()
    }

    fn main_views(&self) -> Vec<View> {
        self.shell.rec().views.clone()
    }

    fn restarts(&self) -> usize {
        self.shell.rec().restarts
    }

    fn saved_settings(&self) -> Value {
        serde_json::from_str(&fs::read_to_string(self.store.join("settings.json")).unwrap())
            .unwrap()
    }

    fn exe_bytes(&self) -> Vec<u8> {
        fs::read(&self.exe).unwrap()
    }

    fn folder(&self) -> Vec<String> {
        let mut v: Vec<String> = fs::read_dir(self.exe.parent().unwrap())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        v.sort();
        v
    }
}

/// Drives start, every command and 48 simulated hours of ticks, asserting
/// no request, no download and no `updates:` line, then makes one manual
/// check: the positive control that the same fake counts a request.
fn assert_no_request_unless_asked(h: &Harness) {
    h.app.start();
    let _ = h.app.get_view();
    let id = tokened_ids(&h.app)[0].clone();
    h.app
        .update_selection(&json!({ "want": [id] }))
        .expect("update_selection");
    let _ = h.app.copy_now();
    let _ = h.app.set_hotkey("Ctrl+Alt+R");
    h.app.open_settings().expect("open_settings");
    let view = h.app.get_app_settings();
    assert!(!view.check_updates);
    assert_eq!(view.update.state, UpdateState::Idle);
    assert_eq!(view.update.text, "Not checked yet.");
    assert_eq!(view.update.newer, None);
    assert_eq!(view.install.state, InstallState::None);
    assert_eq!(
        h.app.install_update().unwrap_err(),
        "no newer version is known"
    );
    let _ = h.app.cancel_update();
    h.ticks(0, 48 * HOUR);
    assert_eq!(h.calls(), 0, "a request nobody asked for");
    assert!(h.downloads().is_empty());
    // The refused install is the one line; nothing else of updates.
    assert_eq!(
        h.update_lines(),
        lines(&["updates: install refused: no newer version is known"])
    );
    assert!(h.emitted().is_empty(), "{:?}", h.emitted());
    assert_eq!(h.get_view_update(), None);

    // Positive control: the same instrument sees a request once one is asked.
    let view = h.manual(list(&[RUNNING], true));
    assert_eq!(view.update.state, UpdateState::UpToDate);
    assert_eq!(h.calls(), 1);
    assert_eq!(
        h.update_lines()[1..],
        lines(&[
            "updates: check started (manual)",
            "updates: up to date (running 1.2.0, latest 1.2.0)",
        ])
    );
    assert!(h.downloads().is_empty());
    assert_eq!(h.exe_bytes(), RUNNING_BYTES);
    h.app.shutdown();
}

impl Harness {
    fn get_view_update(&self) -> Option<String> {
        self.app.get_view().update_available
    }
}

// ---------------------------------------------------------------------------
// Baseline.

#[test]
fn updates_default_settings_make_no_request_baseline() {
    let h = harness(Setup::default());
    assert_no_request_unless_asked(&h);
    // The setting was saved with the rest, off.
    assert_eq!(h.saved_settings()["check_updates"], false);
    // The shipped default is the waystone roller, ticking on its own.
    let d = UpdateSetup::default();
    assert_eq!(d.product, WAYSTONE_ROLLER);
    assert!(d.tick_every.is_some());
    assert!(d.config.transport.is_none());
}

#[test]
fn updates_settings_file_without_the_key_makes_no_request_baseline() {
    // The file 0.1.0 wrote: the schema, the shortcut and a selection, no key.
    let h = harness(Setup {
        saved: Some(json!({
            "schema": 1,
            "hotkey": "Ctrl+Alt+W",
            "selection": {
                "want": [], "want_mode": "any", "avoid": [], "tier_min": 1, "tier_max": 16,
                "rarity": { "normal": true, "magic": true, "rare": true }, "corruption": "any"
            }
        })),
        ..Setup::default()
    });
    assert_no_request_unless_asked(&h);
    assert_eq!(h.saved_settings()["check_updates"], false);
}

// ---------------------------------------------------------------------------
// The setting.

#[test]
fn check_updates_is_off_by_default_and_saved() {
    let h = harness(Setup::default());
    h.app.start();
    assert!(!h.app.get_app_settings().check_updates);

    // Turned on: saved, logged, sent to the Settings window.
    let view = h
        .app
        .update_settings(&json!({ "check_updates": true }))
        .unwrap();
    assert!(view.check_updates);
    h.app.flush();
    assert_eq!(h.saved_settings()["check_updates"], true);
    assert_eq!(h.saved_settings()["hotkey"], "Ctrl+Alt+W");
    assert!(h.emitted().last().unwrap().check_updates);
    assert!(h
        .update_lines()
        .contains(&"updates: automatic checks turned on".to_string()));

    // Refused, with the setting as it was: another key, another type, not
    // an object.
    for bad in [
        json!({ "hotkey": "Ctrl+Alt+R" }),
        json!({ "check_updates": "yes" }),
        json!({ "check_updates": false, "tier_min": 3 }),
        json!([true]),
    ] {
        assert!(h.app.update_settings(&bad).is_err(), "{bad}");
    }
    assert!(h.app.get_app_settings().check_updates);
    h.app.shutdown();

    // A new run reads it back on; turning it off is saved too.
    let shell = FakeShell::default();
    let again = App::new(AppConfig {
        data: fixture_data(),
        store: Store::open(&h.store),
        shell: Box::new(shell.clone()),
    });
    assert!(again.get_app_settings().check_updates);
    again
        .update_settings(&json!({ "check_updates": false }))
        .unwrap();
    again.shutdown();
    assert_eq!(h.saved_settings()["check_updates"], false);
    assert!(diag_lines(&h.store).contains(&"updates: automatic checks turned off".to_string()));
}

// ---------------------------------------------------------------------------
// Target: the check.

#[test]
fn updates_manual_check_makes_one_request_target() {
    let h = harness(Setup {
        list_held: true,
        ..Setup::default()
    });
    h.app.start();
    h.reply(signed_newer());
    let view = h.app.check_for_updates();
    // It returns at once, with the check running.
    assert_eq!(view.update.state, UpdateState::Checking);
    assert_eq!(view.update.text, "Checking for updates\u{2026}");
    h.wait_calls(1);
    // A second request while one runs starts nothing.
    let _ = h.app.check_for_updates();
    h.hold_list(false);
    let view = h.wait_ended();
    assert_eq!(h.calls(), 1);
    assert_eq!(view.update.state, UpdateState::Available);
    assert_eq!(
        view.update.text,
        "Waystone Roller 1.3.0 is available. You have 1.2.0."
    );
    assert_eq!(view.update.newer.as_deref(), Some("1.3.0"));
    assert_eq!(view.install.state, InstallState::Ready);
    assert_eq!(view.install.text, "Install 1.3.0 and restart");
    assert_eq!(view.version, "1.2.0");
    assert!(!view.check_updates);
    assert_eq!(
        h.update_lines(),
        lines(&[
            "updates: check started (manual)",
            "updates: 1.3.0 available (running 1.2.0)"
        ])
    );
    // The window saw the check start and end; the last view sent is current.
    let sent = h.emitted();
    assert!(sent.iter().any(|v| v.update.state == UpdateState::Checking));
    assert_eq!(sent.last(), Some(&view));
    // A manual check schedules nothing and installs nothing.
    h.ticks(0, 48 * HOUR);
    assert_eq!(h.calls(), 1);
    assert!(h.downloads().is_empty());
    assert_eq!(h.exe_bytes(), RUNNING_BYTES);
    h.app.shutdown();
}

#[test]
fn updates_auto_on_checks_at_start_then_daily_target() {
    let h = harness(auto_on());
    h.reply(list(&[RUNNING], true));
    // The start itself checks once with the setting on.
    h.app.start();
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
    h.ticks(10 * MINUTE, 24 * HOUR - MINUTE);
    assert_eq!(h.calls(), 1);
    // A day after it started, again.
    h.at(24 * HOUR);
    h.wait_calls(2);
    h.wait_ended();
    h.ticks(24 * HOUR + MINUTE, 48 * HOUR - MINUTE);
    assert_eq!(h.calls(), 2);
    h.at(48 * HOUR);
    h.wait_calls(3);
    h.wait_ended();
    // Turned off, nothing more.
    h.app
        .update_settings(&json!({ "check_updates": false }))
        .unwrap();
    h.ticks(48 * HOUR, 120 * HOUR);
    assert_eq!(h.calls(), 3);
    assert_eq!(
        h.update_lines()
            .iter()
            .filter(|l| l.starts_with("updates: check started"))
            .count(),
        3
    );
    assert!(h.downloads().is_empty());
    h.app.shutdown();
}

#[test]
fn view_marks_an_available_update() {
    let h = harness(Setup::default());
    h.app.start();
    assert_eq!(h.app.get_view().update_available, None);
    let first = h.main_views().len();

    h.manual(signed_newer());
    assert_eq!(h.app.get_view().update_available.as_deref(), Some("1.3.0"));
    // The main window was sent the view that names it.
    let sent = h.main_views();
    assert!(sent.len() > first);
    assert_eq!(sent.last().unwrap().update_available.as_deref(), Some("1.3.0"));

    // A failed check keeps it; an unsigned newer release still names it.
    h.manual(status(500));
    assert_eq!(h.app.get_view().update_available.as_deref(), Some("1.3.0"));
    let view = h.manual(list(&[NEWER], false));
    assert_eq!(view.install.state, InstallState::Unsigned);
    assert_eq!(h.app.get_view().update_available.as_deref(), Some("1.3.0"));

    // Up to date: no longer, and the main window hears it.
    h.manual(list(&[RUNNING], true));
    assert_eq!(h.app.get_view().update_available, None);
    assert_eq!(h.main_views().last().unwrap().update_available, None);
    h.app.shutdown();
}

// ---------------------------------------------------------------------------
// Target: the install.

#[test]
fn updates_install_is_reached_only_from_the_command_target() {
    let h = harness(auto_on());
    *h.wire.fallback.lock().unwrap() = signed_newer();
    h.app.start();
    h.wait_calls(1);
    let view = h.wait_ended();
    assert_eq!(view.install.state, InstallState::Ready);
    // Two days of automatic checks, a manual one and the Settings window's
    // other commands: a newer signed release is known throughout, and
    // nothing is downloaded.
    h.ticks(0, 24 * HOUR);
    h.wait_calls(2);
    h.wait_ended();
    h.ticks(24 * HOUR, 48 * HOUR);
    h.wait_calls(3);
    h.wait_ended();
    assert_eq!(h.calls(), 3);
    h.manual(signed_newer());
    let _ = h.app.get_app_settings();
    let _ = h.app.cancel_update();
    h.app.open_settings().unwrap();
    h.app
        .update_settings(&json!({ "check_updates": true }))
        .unwrap();
    assert!(h.downloads().is_empty(), "{:?}", h.downloads());
    assert_eq!(h.exe_bytes(), RUNNING_BYTES);
    assert_eq!(h.folder(), vec!["waystone-roller.exe".to_string()]);
    assert!(!h
        .update_lines()
        .iter()
        .any(|l| l.contains("install")), "{:?}", h.update_lines());

    // The command: the signature first, then the exe.
    h.hold_downloads(true);
    let view = h.app.install_update().unwrap();
    assert_eq!(view.install.state, InstallState::Downloading);
    // While it runs, neither a check nor a second install starts.
    let calls = h.calls();
    let _ = h.app.check_for_updates();
    h.at(200 * HOUR);
    assert_eq!(h.calls(), calls);
    assert_eq!(h.app.install_update().unwrap_err(), "an install is running");
    h.hold_downloads(false);
    wait_until("the restart request", || h.restarts() == 1);
    assert_eq!(h.downloads(), vec![AssetKind::Signature, AssetKind::Exe]);
    h.app.shutdown();
}

#[test]
fn updates_install_restarts_through_the_quit_path_target() {
    let h = harness(Setup::default());
    h.app.start();
    h.manual(signed_newer());
    // A change the quit path's shutdown must still write.
    h.app
        .update_settings(&json!({ "check_updates": true }))
        .unwrap();
    let id = tokened_ids(&h.app)[0].clone();
    h.app.update_selection(&json!({ "want": [id.clone()] })).unwrap();
    assert!(!h.app.restart_requested());

    let view = h.app.install_update().unwrap();
    assert!(matches!(
        view.install.state,
        InstallState::Downloading | InstallState::Verifying | InstallState::Restarting
    ));
    wait_until("the restart request", || h.restarts() == 1);
    assert!(h.app.restart_requested());
    // The new exe in place, the running one beside it, nothing staged.
    assert_eq!(h.exe_bytes(), payload());
    assert_eq!(
        h.folder(),
        vec![
            "waystone-roller.exe".to_string(),
            "waystone-roller.exe.old".to_string()
        ]
    );
    assert_eq!(
        fs::read(h.exe.with_file_name("waystone-roller.exe.old")).unwrap(),
        RUNNING_BYTES
    );
    let view = h.app.get_app_settings();
    assert_eq!(view.install.state, InstallState::Restarting);
    assert_eq!(
        view.install.text,
        "Waystone Roller 1.3.0 is installed. Restarting\u{2026}"
    );
    // The window saw downloading, verifying and restarting, in that order.
    let states: Vec<InstallState> = h.emitted().iter().map(|v| v.install.state).collect();
    let at = |s| states.iter().position(|x| *x == s).unwrap();
    assert!(at(InstallState::Downloading) < at(InstallState::Verifying));
    assert!(at(InstallState::Verifying) < at(InstallState::Restarting));

    // Restarting: nothing more starts, and asking again does not quit twice.
    let calls = h.calls();
    let _ = h.app.check_for_updates();
    assert_eq!(h.calls(), calls);
    assert_eq!(h.app.install_update().unwrap_err(), "the app is restarting");
    assert_eq!(h.restarts(), 1);

    // The quit path: the shell's exit runs the app's shutdown, which saves.
    h.app.shutdown();
    let saved = h.saved_settings();
    assert_eq!(saved["check_updates"], true);
    assert_eq!(saved["selection"]["want"], json!([id]));
    let log = diag_lines(&h.store);
    let pos = |l: &str| log.iter().position(|x| x == l).unwrap_or_else(|| panic!("{l} in {log:?}"));
    assert!(pos("updates: install of 1.3.0 started") < pos("updates: signature of 1.3.0 checked"));
    assert!(
        pos("updates: signature of 1.3.0 checked")
            < pos("updates: replaced the running exe with 1.3.0, restarting")
    );
    assert!(pos("updates: replaced the running exe with 1.3.0, restarting") < pos("stop"));
    assert!(h.app.restart_requested());
}

#[test]
fn updates_failed_install_leaves_the_exe_and_offers_try_again() {
    let h = harness(Setup::default());
    *h.wire.sig.lock().unwrap() = signing("another-key.minisig");
    h.app.start();
    h.manual(signed_newer());
    h.app.install_update().unwrap();
    wait_until("the install to fail", || {
        h.app.get_app_settings().install.state == InstallState::Failed
    });
    let view = h.app.get_app_settings();
    assert!(view.install.text.starts_with("Could not install Waystone Roller 1.3.0: "));
    assert_eq!(h.exe_bytes(), RUNNING_BYTES);
    assert_eq!(h.folder(), vec!["waystone-roller.exe".to_string()]);
    assert_eq!(h.restarts(), 0);
    assert!(!h.app.restart_requested());
    // Try again is the same command; with a good signature it installs.
    *h.wire.sig.lock().unwrap() = signing("payload.bin.minisig");
    h.app.install_update().unwrap();
    wait_until("the restart request", || h.restarts() == 1);
    assert_eq!(h.exe_bytes(), payload());
    h.app.shutdown();
}

#[test]
fn updates_cancel_goes_back_to_ready() {
    let h = harness(Setup::default());
    h.app.start();
    h.manual(signed_newer());
    h.hold_downloads(true);
    h.app.install_update().unwrap();
    wait_until("the download to wait", || {
        h.wire.download_waiting.load(Ordering::SeqCst)
    });
    h.app.cancel_update();
    // The line is the install's last step, after the state and the file.
    wait_until("the cancelled line", || {
        h.update_lines()
            .contains(&"updates: install cancelled".to_string())
    });
    assert_eq!(h.app.get_app_settings().install.state, InstallState::Ready);
    assert_eq!(h.folder(), vec!["waystone-roller.exe".to_string()]);
    h.app.shutdown();
}

#[test]
fn updates_shutdown_does_not_wait_for_an_install() {
    let h = harness(Setup::default());
    h.app.start();
    h.manual(signed_newer());
    h.hold_downloads(true);
    h.app.install_update().unwrap();
    wait_until("the download to wait", || {
        h.wire.download_waiting.load(Ordering::SeqCst)
    });
    let sent = h.emitted().len();
    let start = Instant::now();
    h.app.shutdown();
    assert!(start.elapsed() < Duration::from_secs(2));
    // The install ends cancelled, deleting its staging file, announcing and
    // writing nothing.
    wait_until("the staging file to go", || {
        h.folder() == vec!["waystone-roller.exe".to_string()]
    });
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(h.emitted().len(), sent);
    assert_eq!(h.restarts(), 0);
    assert_eq!(h.exe_bytes(), RUNNING_BYTES);
}

#[test]
fn open_settings_asks_the_shell() {
    let h = harness(Setup::default());
    h.app.start();
    h.app.open_settings().unwrap();
    h.app.open_settings().unwrap();
    assert_eq!(h.shell.rec().settings_opened, 2);
    h.app.shutdown();
}

// ---------------------------------------------------------------------------
// The app-settings fixtures.

fn update_fixtures() -> bool {
    std::env::var("WR_UPDATE_FIXTURES").is_ok_and(|v| v == "1")
}

/// One view per check state and per install state, from runs of the app
/// through the fake. The install states' names start `install-`.
fn app_settings_views() -> Vec<(&'static str, AppSettingsView)> {
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
    out.push(("up-to-date", h.manual(list(&[RUNNING, Version::new(1, 0, 0)], true))));
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
        .find(|v| v.install.state == InstallState::Verifying)
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
        h.app.get_app_settings().install.state == InstallState::Failed
    });
    out.push(("install-failed", h.app.get_app_settings()));
    h.app.shutdown();
    out
}

fn fixture_dir() -> PathBuf {
    app_root().join("fixtures/app-settings")
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
    // Each is the state it is named for, built from the synthetic versions.
    for (name, view) in &views {
        let (part, state) = match name.strip_prefix("install-") {
            Some(s) => (serde_json::to_value(view.install.state).unwrap(), s),
            None => (serde_json::to_value(view.update.state).unwrap(), *name),
        };
        assert_eq!(part, json!(state), "{name}");
        assert_eq!(view.version, RUNNING.to_string());
        // No part of a view carries a URL.
        let text = serde_json::to_string(view).unwrap();
        assert!(!text.contains("://"), "{name}: {text}");
    }
    let by = |n: &str| &views.iter().find(|(name, _)| *name == n).unwrap().1;
    assert_eq!(by("available").install.state, InstallState::Ready);
    assert_eq!(by("failed").update.newer.as_deref(), Some("1.3.0"), "failed keeps the newer");
    assert_eq!(by("up-to-date").update.newer, None);
    assert_eq!(by("idle").install.state, InstallState::None);
    assert_eq!(
        by("install-unsigned").install.text,
        "1.3.0 is available but cannot be installed: the release is not signed."
    );
    let progress = by("install-downloading").install.progress.unwrap();
    assert_eq!((progress.bytes, progress.total), (0, Some(payload().len() as u64)));

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
            .unwrap_or_else(|e| panic!("{}: {e} (run with WR_UPDATE_FIXTURES=1)", path.display()));
        let on_disk: Value = serde_json::from_str(&text).unwrap();
        if on_disk != serde_json::to_value(view).unwrap() {
            stale.push(*name);
        }
    }
    assert!(
        stale.is_empty(),
        "stale app-settings fixtures {stale:?}: run with WR_UPDATE_FIXTURES=1 and review the diff"
    );
}
