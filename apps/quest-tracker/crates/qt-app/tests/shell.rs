//! The shell's logic with a fake emitter, clock and window, over the engine's
//! synthetic quest data (`fixtures/quests_min.json`) and sessions
//! (`fixtures/sessions/`, `fixtures/logs/`). Every store and log lives in a
//! temporary folder; no test reads the real `Client.txt` or looks at the
//! game's install folders. All character names are invented.
//!
//! Baseline: an app with no log, no character and no command shows an empty
//! state, sends nothing to the window but its play-mode click-through, and
//! writes nothing. Target: lines reach the view through the worker, every
//! command announces the view it leaves, the mode drives click-through,
//! failures reach the status line, saves are debounced and done on exit, and
//! the diagnostic log never holds chat or a party member's name.

use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use qt_app::cli::{self, Options};
use qt_app::discover::{self, LogSource};
use qt_app::{App, AppConfig, Clock, Emitter, MonitorMetrics, Placement, Window};
use qt_engine::{QuestAction, QuestData, Status};
use qt_store::Store;
use qt_tailer::{Fingerprint, Line, TailEvent};
use qt_viewmodel::{Dock, Frame, Mode, StatusKind, Toast, View, ViewKind};
use serde_json::{json, Value};

const OWN: &str = "Corisande_Fell";

// ---------------------------------------------------------------------------
// Fakes.

#[derive(Default)]
struct Recorder {
    views: Mutex<Vec<View>>,
    toasts: Mutex<Vec<Toast>>,
}

impl Recorder {
    fn count(&self) -> usize {
        self.views.lock().unwrap().len()
    }

    fn last(&self) -> View {
        self.views.lock().unwrap().last().cloned().expect("a view was emitted")
    }

    /// Waits up to five seconds for an emitted view that satisfies `pred`.
    fn wait_for(&self, what: &str, pred: impl Fn(&View) -> bool) -> View {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(v) = self.views.lock().unwrap().iter().rev().find(|v| pred(v)) {
                return v.clone();
            }
            assert!(Instant::now() < deadline, "no view {what} within 5 s");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

struct FakeEmitter(Arc<Recorder>);

impl Emitter for FakeEmitter {
    fn view(&self, view: &View) {
        self.0.views.lock().unwrap().push(view.clone());
    }

    fn toast(&self, toast: &Toast) {
        self.0.toasts.lock().unwrap().push(toast.clone());
    }
}

#[derive(Default)]
struct WindowLog {
    calls: Mutex<Vec<String>>,
    hotkey_fails: AtomicBool,
    /// The monitor the fake window reports; `None` is no monitor.
    monitor: Mutex<Option<MonitorMetrics>>,
}

impl WindowLog {
    fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }

    fn click_through(&self) -> Vec<String> {
        self.calls()
            .into_iter()
            .filter(|c| c.starts_with("click_through"))
            .collect()
    }
}

struct FakeWindow(Arc<WindowLog>);

impl Window for FakeWindow {
    fn set_click_through(&self, on: bool) -> Result<(), String> {
        self.0.calls.lock().unwrap().push(format!("click_through {on}"));
        Ok(())
    }

    fn set_scale(&self, factor: f64) -> Result<(), String> {
        self.0.calls.lock().unwrap().push(format!("scale {factor}"));
        Ok(())
    }

    fn place(&self, placement: Placement) -> Result<(), String> {
        self.0.calls.lock().unwrap().push(format!("place {placement:?}"));
        Ok(())
    }

    fn register_hotkey(&self, hotkey: &str) -> Result<(), String> {
        self.0.calls.lock().unwrap().push(format!("hotkey {hotkey}"));
        if self.0.hotkey_fails.load(Ordering::SeqCst) {
            Err("HotKey already registered".to_string())
        } else {
            Ok(())
        }
    }

    fn open_url(&self, url: &str) -> Result<(), String> {
        self.0.calls.lock().unwrap().push(format!("open {url}"));
        Ok(())
    }

    fn monitor(&self) -> Result<Option<MonitorMetrics>, String> {
        Ok(*self.0.monitor.lock().unwrap())
    }
}

struct FakeClock(Arc<AtomicU64>);

impl Clock for FakeClock {
    fn now_ms(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}

// ---------------------------------------------------------------------------
// Helpers.

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

fn data() -> Arc<QuestData> {
    Arc::new(QuestData::load(&fixtures().join("quests_min.json")).expect("fixture data"))
}

fn fixture_lines(rel: &str) -> Vec<String> {
    let text = fs::read_to_string(fixtures().join(rel)).expect("fixture file");
    text.lines().map(|l| l.trim_end_matches('\r').to_string()).collect()
}

struct Harness {
    _dir: tempfile::TempDir,
    root: PathBuf,
    app: Arc<App>,
    views: Arc<Recorder>,
    window: Arc<WindowLog>,
    clock: Arc<AtomicU64>,
}

impl Harness {
    fn store_dir(&self) -> PathBuf {
        self.root.join("store")
    }

    fn diag_text(&self) -> String {
        fs::read_to_string(self.store_dir().join("overlay.log")).unwrap_or_default()
    }

    fn stored(&self, file: &str) -> Option<Value> {
        let text = fs::read_to_string(self.store_dir().join(file)).ok()?;
        let v: Value = serde_json::from_str(&text).expect("stored JSON");
        v.get("data").cloned()
    }

    /// Waits up to five seconds for the diagnostic log to hold every one of
    /// `parts` on one line, and returns that line.
    fn wait_diag(&self, parts: &[&str]) -> String {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let text = self.diag_text();
            if let Some(l) = text.lines().find(|l| parts.iter().all(|p| l.contains(p))) {
                return l.to_string();
            }
            assert!(
                Instant::now() < deadline,
                "no diagnostic line with {parts:?} within 5 s:\n{text}"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// A second run over the same folder (store and log), once this one has
    /// shut down and saved.
    fn rerun(self, setup: Setup) -> Harness {
        assert!(self.app.shutdown(Duration::from_secs(2)), "first run ended");
        harness_in(self._dir, setup)
    }
}

struct Setup {
    options: Options,
    candidates: Vec<PathBuf>,
    hotkey_fails: bool,
    /// The monitor the window reports; `None` is no monitor.
    monitor: Option<MonitorMetrics>,
    /// Files to write before the app starts: (relative path, content).
    files: Vec<(&'static str, String)>,
}

impl Default for Setup {
    fn default() -> Setup {
        Setup {
            options: Options::default(),
            candidates: Vec::new(),
            hotkey_fails: false,
            monitor: None,
            files: Vec::new(),
        }
    }
}

/// An app over a fresh temporary folder. `options.log` and the candidates
/// may name paths relative to that folder.
fn harness(setup: Setup) -> Harness {
    harness_in(tempfile::tempdir().expect("temp dir"), setup)
}

/// An app over `dir`, which may hold an earlier run's store and log.
fn harness_in(dir: tempfile::TempDir, setup: Setup) -> Harness {
    let root = dir.path().to_path_buf();
    for (rel, content) in &setup.files {
        fs::write(root.join(rel), content).expect("write fixture file");
    }
    let views = Arc::new(Recorder::default());
    let window = Arc::new(WindowLog::default());
    window.hotkey_fails.store(setup.hotkey_fails, Ordering::SeqCst);
    *window.monitor.lock().unwrap() = setup.monitor;
    let clock = Arc::new(AtomicU64::new(100_000));
    let mut options = setup.options;
    options.log = options.log.map(|p| root.join(p));
    let app = App::new(AppConfig {
        options,
        options_error: None,
        store: Store::open(root.join("store")).map_err(|e| e.to_string()),
        data: Some(data()),
        candidates: setup.candidates.iter().map(|c| root.join(c)).collect(),
        poll_interval: Duration::from_millis(20),
        emitter: Box::new(FakeEmitter(Arc::clone(&views))),
        window: Box::new(FakeWindow(Arc::clone(&window))),
        clock: Box::new(FakeClock(Arc::clone(&clock))),
    });
    Harness {
        _dir: dir,
        root,
        app,
        views,
        window,
        clock,
    }
}

/// Tailer events for `lines`, as one file read them.
fn events(lines: &[String], catch_up: bool) -> Vec<TailEvent> {
    let fingerprint = Fingerprint::of(b"fixture log");
    let mut offset = 0u64;
    lines
        .iter()
        .map(|text| {
            offset += text.len() as u64 + 1;
            TailEvent::Line(Line {
                text: text.clone(),
                end_offset: offset,
                catch_up,
                fingerprint,
            })
        })
        .collect()
}

fn own_character_lines() -> Vec<String> {
    vec![
        "2026/03/10 19:00:00 ***** LOG FILE OPENING *****".to_string(),
        format!("2026/03/10 19:00:20 4020000 7e2c0001 [INFO Client 7120] : {OWN} has received +20 to maximum Life."),
        format!("2026/03/10 19:00:40 4040000 7e2c0002 [INFO Client 7120] : {OWN} (Witch) is now level 9"),
        "2026/03/10 19:02:01 4121000 7e2c0006 [DEBUG Client 7120] Generating level 1 area \"G1_1\" with seed 2077351921".to_string(),
    ]
}

fn append(path: &Path, lines: &[String]) {
    let mut f = OpenOptions::new().append(true).open(path).expect("open log for append");
    for l in lines {
        f.write_all(l.as_bytes()).unwrap();
        f.write_all(b"\n").unwrap();
    }
}

// ---------------------------------------------------------------------------
// Tests.

#[test]
fn click_through_follows_mode() {
    let h = harness(Setup::default());
    h.app.start();
    // Baseline: the app starts in play mode, click-through on.
    assert_eq!(h.window.click_through(), vec!["click_through true"]);
    assert_eq!(h.views.last().mode, Mode::Play);

    let view = h.app.set_mode(Mode::Edit).unwrap();
    assert_eq!(view.mode, Mode::Edit);
    assert_eq!(h.window.click_through().last().unwrap(), "click_through false");

    // The shortcut toggles back.
    let view = h.app.toggle_mode().unwrap();
    assert_eq!(view.mode, Mode::Play);
    assert_eq!(h.window.click_through().last().unwrap(), "click_through true");

    // Negative control: a command that is not about the mode leaves the
    // window alone.
    let before = h.window.click_through().len();
    h.app.set_view_kind(ViewKind::Campaign).unwrap();
    assert_eq!(h.window.click_through().len(), before);
    assert!(h.app.shutdown(Duration::from_secs(2)));
}

#[test]
fn worker_emits_view_after_new_lines() {
    let h = harness(Setup {
        options: Options {
            log: Some(PathBuf::from("Client.txt")),
            ..Options::default()
        },
        files: vec![(
            "Client.txt",
            "2026/03/10 18:59:00 1 1 [INFO Client 7120] Connecting to instance server\n".to_string(),
        )],
        ..Setup::default()
    });
    h.app.start();
    // Baseline: the log exists and nothing new is in it.
    assert_eq!(h.views.last().frame, Frame::EmptyWaiting);
    assert!(h.views.last().character.is_none());

    // The tailer starts at the end of the log; append only once it follows.
    let deadline = Instant::now() + Duration::from_secs(5);
    while !h.diag_text().contains("log: following") {
        assert!(Instant::now() < deadline, "the tailer did not start following");
        std::thread::sleep(Duration::from_millis(10));
    }
    append(&h.root.join("Client.txt"), &own_character_lines());
    let view = h.views.wait_for("naming the character", |v| {
        v.character.as_ref().is_some_and(|c| c.name == OWN && c.level == 9)
    });
    assert_ne!(view.frame, Frame::EmptyWaiting);
    assert_eq!(view.character.unwrap().class, "Witch");
    assert!(h.app.worker_running());
    assert!(h.app.shutdown(Duration::from_secs(2)));
}

#[test]
fn commands_announce_after_state_change() {
    let h = harness(Setup::default());
    h.app.handle_tail_events(events(&own_character_lines(), false));
    assert_eq!(h.views.last().character.as_ref().unwrap().name, OWN);

    let check = |what: &str, result: Result<View, String>| -> View {
        let returned = result.unwrap_or_else(|e| panic!("{what}: {e}"));
        assert_eq!(h.views.last(), returned, "{what}: the returned view is the one announced");
        returned
    };

    let n = h.views.count();
    let v = check("set_view_kind", h.app.set_view_kind(ViewKind::Campaign));
    assert_eq!(v.view_kind, ViewKind::Campaign);
    assert_eq!(v.frame, Frame::PanelCampaign);
    assert_eq!(h.views.count(), n + 1);

    let v = check("open_frame", h.app.open_frame(Some(Frame::Settings)));
    assert_eq!(v.frame, Frame::Settings);
    let v = check("open_frame none", h.app.open_frame(None));
    assert_eq!(v.frame, Frame::PanelCampaign);

    let n = h.views.count();
    check(
        "quest_action",
        h.app.quest_action("fixture-riverbank", QuestAction::Skip),
    );
    assert_eq!(h.views.count(), n + 1);
    let v = check("open_detail", h.app.open_detail(Some("fixture-riverbank".to_string())));
    assert_eq!(v.frame, Frame::QuestDetail);
    assert_eq!(v.detail.as_ref().unwrap().row.status, Status::Skipped);

    let v = check("update_settings", h.app.update_settings(&json!({ "opacity": 0.5 })));
    assert_eq!(v.settings.opacity, 0.5);
    let v = check("set_collapsed", h.app.set_collapsed(true));
    assert_eq!(v.frame, Frame::Collapsed);
    let v = check("set_mode", h.app.set_mode(Mode::Edit));
    assert_eq!(v.mode, Mode::Edit);
    let v = check("pin_character", h.app.pin_character(Some(OWN)));
    assert!(v.character.as_ref().unwrap().pinned);
    let v = check("select_character", h.app.select_character(OWN));
    assert!(v.character.unwrap().pinned);

    // Negative controls: a refused command and a read announce nothing.
    let n = h.views.count();
    assert!(h.app.open_frame(Some(Frame::PanelCampaign)).is_err());
    assert!(h.app.quest_action("no-such-quest", QuestAction::Skip).is_err());
    assert!(h.app.update_settings(&json!({ "no_such_setting": 1 })).is_err());
    let _ = h.app.get_view();
    assert_eq!(h.views.count(), n);

    // open_source opens the quest's page through the window, not a request.
    h.app.open_source("fixture-riverbank").unwrap();
    let source = h.app.data().quest("fixture-riverbank").unwrap().source_url.clone();
    assert!(h.window.calls().contains(&format!("open {source}")));
}

#[test]
fn hotkey_failure_is_reported() {
    let h = harness(Setup {
        hotkey_fails: true,
        ..Setup::default()
    });
    h.app.start();
    let status = h.views.last().status.expect("a status line");
    assert_eq!(status.kind, StatusKind::Error);
    assert!(status.text.contains("Ctrl+Alt+Q"), "{}", status.text);
    assert!(h.diag_text().contains("shortcut Ctrl+Alt+Q could not be registered"));

    // Another shortcut that registers clears it.
    h.window.hotkey_fails.store(false, Ordering::SeqCst);
    let view = h.app.update_settings(&json!({ "hotkey": "Ctrl+Alt+W" })).unwrap();
    assert!(h.window.calls().contains(&"hotkey Ctrl+Alt+W".to_string()));
    assert!(
        view.status.as_ref().map_or(true, |s| !s.text.contains("shortcut")),
        "{:?}",
        view.status
    );
    assert!(h.diag_text().contains("shortcut Ctrl+Alt+W registered"));
    assert!(h.app.shutdown(Duration::from_secs(2)));
}

#[test]
fn missing_log_shows_status() {
    // Nothing named, nothing found.
    let h = harness(Setup {
        candidates: vec![PathBuf::from("absent/Client.txt")],
        ..Setup::default()
    });
    let view = h.app.get_view();
    assert_eq!(view.frame, Frame::EmptyNoLog);
    let status = view.status.expect("a status line");
    assert_eq!(status.kind, StatusKind::Error);
    assert!(status.text.contains("Client.txt"), "{}", status.text);
    assert!(h.app.log_path().is_none());

    // A log the option names but that is not there yet: waiting, then the
    // tailer says it is missing.
    let h = harness(Setup {
        options: Options {
            log: Some(PathBuf::from("later/Client.txt")),
            ..Options::default()
        },
        ..Setup::default()
    });
    assert_eq!(h.app.get_view().frame, Frame::EmptyWaiting);
    h.app.handle_tail_events(vec![TailEvent::Waiting]);
    assert_eq!(h.views.last().frame, Frame::EmptyNoLog);

    // Positive control: a candidate that exists is found.
    let h = harness(Setup {
        candidates: vec![PathBuf::from("absent.txt"), PathBuf::from("Client.txt")],
        files: vec![("Client.txt", String::new())],
        ..Setup::default()
    });
    assert_eq!(h.app.get_view().frame, Frame::EmptyWaiting);
    assert_eq!(
        h.app.log_path().unwrap().file_name().unwrap(),
        "Client.txt"
    );
}

#[test]
fn shutdown_stops_worker_and_saves() {
    let session = fixture_lines("sessions/party.txt").join("\n") + "\n";
    let len = session.len() as u64;
    let h = harness(Setup {
        options: Options {
            log: Some(PathBuf::from("Client.txt")),
            from_start: true,
            ..Options::default()
        },
        files: vec![("Client.txt", session)],
        ..Setup::default()
    });
    h.app.start();
    h.views.wait_for("at level 10", |v| {
        v.character.as_ref().is_some_and(|c| c.name == OWN && c.level == 10)
    });
    assert!(h.app.worker_running());

    assert!(h.app.shutdown(Duration::from_secs(2)), "threads ended in time");
    assert!(!h.app.worker_running());
    let roster = h.stored("characters.json").expect("characters.json");
    assert_eq!(roster[OWN]["level"], 10);
    assert_eq!(roster[OWN]["own"], true);
    let state = h
        .app
        .store()
        .unwrap()
        .load_character(OWN)
        .unwrap()
        .value
        .expect("the character's file");
    assert_eq!(state["name"], OWN);
    let position = h.stored("position.json").expect("position.json");
    assert_eq!(position["offset"], len);
    assert!(h.stored("settings.json").is_some());
    assert!(h.diag_text().contains("stop: tailer and worker ended"));
}

#[test]
fn saves_are_debounced() {
    let h = harness(Setup::default());
    let opacity = |h: &Harness| h.stored("settings.json").map(|s| s["opacity"].clone());
    // Baseline: a first run writes its default settings on the first tick,
    // and then nothing until something changes.
    assert_eq!(opacity(&h), None);
    h.app.tick();
    assert_eq!(h.app.save_count(), 1);
    assert_eq!(opacity(&h), Some(json!(0.9)));
    h.clock.fetch_add(5_000, Ordering::SeqCst);
    h.app.tick();
    assert_eq!(h.app.save_count(), 1);

    // A change after a quiet spell saves at once.
    h.app.update_settings(&json!({ "opacity": 0.5 })).unwrap();
    assert_eq!(h.app.save_count(), 2);
    assert_eq!(opacity(&h), Some(json!(0.5)));

    // A change within 2 s waits for the worker's tick after the interval.
    h.clock.fetch_add(500, Ordering::SeqCst);
    h.app.update_settings(&json!({ "opacity": 0.6 })).unwrap();
    assert_eq!(h.app.save_count(), 2);
    assert_eq!(opacity(&h), Some(json!(0.5)));
    h.clock.fetch_add(1_000, Ordering::SeqCst);
    h.app.tick();
    assert_eq!(opacity(&h), Some(json!(0.5)));
    h.clock.fetch_add(600, Ordering::SeqCst);
    h.app.tick();
    assert_eq!(h.app.save_count(), 3);
    assert_eq!(opacity(&h), Some(json!(0.6)));

    // Nothing new: no write.
    h.clock.fetch_add(5_000, Ordering::SeqCst);
    h.app.tick();
    assert_eq!(h.app.save_count(), 3);

    // A change inside the interval is written on exit.
    h.clock.fetch_add(100, Ordering::SeqCst);
    h.app.update_settings(&json!({ "opacity": 0.7 })).unwrap();
    h.clock.fetch_add(100, Ordering::SeqCst);
    h.app.update_settings(&json!({ "opacity": 0.8 })).unwrap();
    assert_eq!(opacity(&h), Some(json!(0.7)));
    assert!(h.app.shutdown(Duration::from_secs(1)));
    assert_eq!(opacity(&h), Some(json!(0.8)));
    assert_eq!(h.app.save_count(), 5);
}

#[test]
fn cli_options_parse() {
    assert_eq!(cli::parse(Vec::<String>::new()).unwrap(), Options::default());
    let o = cli::parse(["--log", "C:/logs/Client.txt", "--from-start", "--data=q.json"]).unwrap();
    assert_eq!(o.log, Some(PathBuf::from("C:/logs/Client.txt")));
    assert_eq!(o.data, Some(PathBuf::from("q.json")));
    assert!(o.from_start);
    let o = cli::parse(["--log=D:/x/Client.txt"]).unwrap();
    assert_eq!(o.log, Some(PathBuf::from("D:/x/Client.txt")));
    assert!(!o.from_start);

    for bad in [
        vec!["--log"],
        vec!["--data"],
        vec!["--log="],
        vec!["--verbose"],
        vec!["Client.txt"],
        vec!["--from-start=yes"],
    ] {
        let err = cli::parse(bad.clone()).expect_err(&format!("{bad:?} is refused"));
        assert!(err.contains("usage:"), "{err}");
    }
}

#[test]
fn log_path_discovery_order() {
    let a = PathBuf::from("A/Client.txt");
    let b = PathBuf::from("B/Client.txt");
    let candidates = vec![a.clone(), b.clone()];
    let only_b = |p: &Path| p == Path::new("B/Client.txt");
    let all = |_: &Path| true;
    let none = |_: &Path| false;

    // The option wins, even over an existing candidate and when it is missing.
    assert_eq!(
        discover::log_path(Some(Path::new("opt.txt")), Some("set.txt"), &candidates, none),
        Some((PathBuf::from("opt.txt"), LogSource::Option))
    );
    // Then the setting (blank counts as unset).
    assert_eq!(
        discover::log_path(None, Some("set.txt"), &candidates, all),
        Some((PathBuf::from("set.txt"), LogSource::Setting))
    );
    assert_eq!(
        discover::log_path(None, Some("  "), &candidates, all),
        Some((a.clone(), LogSource::Discovered))
    );
    // Then the first candidate that exists.
    assert_eq!(
        discover::log_path(None, None, &candidates, only_b),
        Some((b, LogSource::Discovered))
    );
    assert_eq!(discover::log_path(None, None, &candidates, none), None);
    // The built-in candidates: the standalone client first, then Steam.
    let defaults = discover::default_candidates();
    assert_eq!(defaults[0], PathBuf::from(discover::STANDALONE_LOG));
    assert_eq!(defaults[1], PathBuf::from(discover::STEAM_LOG));
    assert!(discover::STANDALONE_LOG.ends_with(r"Path of Exile 2\logs\Client.txt"));
}

#[test]
fn diag_log_never_holds_chat_or_party_names() {
    let h = harness(Setup::default());
    let mut lines = fixture_lines("sessions/party.txt");
    lines.extend(fixture_lines("logs/chat.txt"));
    lines.extend(fixture_lines("logs/npc.txt"));
    // Catch-up first, then the same lines live: both paths write the log.
    h.app.handle_tail_events(events(&lines, true));
    h.app.handle_tail_events(vec![TailEvent::Error("fixture read error".to_string())]);
    h.app.handle_tail_events(events(&lines, false));
    let _ = h.app.pin_character(Some(OWN));
    let _ = h.app.pin_character(None);
    let log = h.diag_text();

    // Positive controls: the log was written, with places and the own name.
    assert!(log.contains("session start"), "{log}");
    assert!(log.contains("area G1_2"), "{log}");
    assert!(log.contains("catch-up:"), "{log}");
    assert!(log.contains(OWN), "{log}");
    assert!(log.contains("an unconfirmed character"), "{log}");

    for forbidden in [
        // party members (fixtures/sessions/party.txt)
        "Bram_Holloway",
        "Wren_Ashdown",
        // chat speakers and text (fixtures/logs/chat.txt)
        "Pellwick_Dunmore",
        "Corvanne",
        "Brisket_Lane",
        "Quillan_Marsh",
        "Tessaly_Vane",
        "anyone selling",
        "WTB",
        "still for sale",
        "local chat shaped",
        "global chat from a player",
    ] {
        assert!(!log.contains(forbidden), "overlay.log holds {forbidden:?}:\n{log}");
    }
}

// Dock changes move the window and drop a dragged position.
#[test]
fn dock_change_places_the_window() {
    let h = harness(Setup::default());
    h.app.set_mode(Mode::Edit).unwrap();
    h.clock.fetch_add(5_000, Ordering::SeqCst);
    h.app.window_moved(10, 20);
    assert!(h.stored("settings.json").unwrap().get("window").is_some());
    h.clock.fetch_add(5_000, Ordering::SeqCst);
    h.app.update_settings(&json!({ "dock": "left" })).unwrap();
    assert!(h
        .window
        .calls()
        .contains(&format!("place {:?}", Placement::Docked(Dock::Left))));
    assert!(h.stored("settings.json").unwrap().get("window").is_none());
}

// The `auto` scale is worked out from the window's monitor wherever the
// shell applies the scale: at start and when the setting changes.
#[test]
fn auto_scale_applied_at_start_and_on_change() {
    fn scales(h: &Harness) -> Vec<String> {
        h.window
            .calls()
            .into_iter()
            .filter(|c| c.starts_with("scale"))
            .collect()
    }

    // Baseline: with no monitor, `auto` (the default) is a zoom of 1.
    let h = harness(Setup::default());
    h.app.start();
    assert_eq!(scales(&h), vec!["scale 1"]);
    assert!(h.app.shutdown(Duration::from_secs(2)));

    // Target: a 2560x1440 monitor at 100% DPI starts at 1.33.
    let h = harness(Setup {
        monitor: Some(MonitorMetrics {
            height: 1440,
            dpi_scale: 1.0,
        }),
        ..Setup::default()
    });
    h.app.start();
    assert_eq!(scales(&h), vec!["scale 1.33"]);
    assert!(h.diag_text().contains("scale auto"), "{}", h.diag_text());

    // A fixed setting ignores the monitor; going back to `auto` asks it again.
    h.app.update_settings(&json!({ "scale": "2" })).unwrap();
    h.app.update_settings(&json!({ "scale": "1" })).unwrap();
    *h.window.monitor.lock().unwrap() = Some(MonitorMetrics {
        height: 2160,
        dpi_scale: 1.5,
    });
    h.app.update_settings(&json!({ "scale": "auto" })).unwrap();
    assert_eq!(
        scales(&h),
        vec!["scale 1.33", "scale 2", "scale 1", "scale 1.33"]
    );

    // Negative control: a settings change that is not the scale leaves the
    // zoom alone.
    h.app.update_settings(&json!({ "opacity": 0.5 })).unwrap();
    assert_eq!(scales(&h).len(), 4);
    assert!(h.app.shutdown(Duration::from_secs(2)));
}

// ---------------------------------------------------------------------------
// The area after a restart: read back from the log before the tailer's start
// position, and a refusal while it is unknown leaves a trace.

const BEIRA: &str = "fixture-beira";

/// A session that ends mid-zone in `G1_2`, before the zone's reward.
fn mid_zone_lines() -> Vec<String> {
    let mut lines = own_character_lines();
    lines.extend([
        "2026/03/10 19:04:00 4240000 7e2c0007 [INFO Client 7120] Connecting to instance server".to_string(),
        "2026/03/10 19:04:01 4241000 7e2c0008 [DEBUG Client 7120] Generating level 2 area \"G1_2\" with seed 2077351921".to_string(),
        "2026/03/10 19:04:02 4242000 7e2c0009 [INFO Client 7120] [SCENE] Set Source [Clearfell]".to_string(),
        "2026/03/10 19:05:30 4330000 7e2c000a [INFO Client 7120] Beira of the Rotten Pack: Fixture line: the pack gathers.".to_string(),
    ]);
    lines
}

/// The zone's reward, `+10% to Cold Resistance`, which finishes
/// `fixture-beira` only in `G1_2`.
fn cold_reward() -> Vec<String> {
    vec![format!(
        "2026/03/10 19:06:15 4375000 7e2c000b [INFO Client 7120] : {OWN} has received +10% to [Resistances|Cold Resistance]."
    )]
}

fn log_setup(content: String, from_start: bool) -> Setup {
    Setup {
        options: Options {
            log: Some(PathBuf::from("Client.txt")),
            from_start,
            ..Options::default()
        },
        files: vec![("Client.txt", content)],
        ..Setup::default()
    }
}

fn text(lines: &[String]) -> String {
    lines.join("\n") + "\n"
}

/// Beira's status for the current character, through the view.
fn beira_status(h: &Harness) -> Status {
    let view = h
        .app
        .open_detail(Some(BEIRA.to_string()))
        .expect("Beira's detail");
    view.detail.expect("a detail").row.status
}

#[test]
fn restart_mid_zone_then_reward_marks_done() {
    // First run: the whole session up to mid-zone in G1_2, read from the
    // start; reading from 0 needs no seed.
    let h = harness(log_setup(text(&mid_zone_lines()), true));
    h.app.start();
    h.views.wait_for("naming the character", |v| {
        v.character
            .as_ref()
            .is_some_and(|c| c.name == OWN && c.level == 9)
    });
    h.wait_diag(&["area from log: not read", "offset 0"]);
    // Every line read: Beira's line in G1_2 put her quest in progress, and
    // nothing finished it.
    let deadline = Instant::now() + Duration::from_secs(5);
    while beira_status(&h) != Status::InProgress {
        assert!(Instant::now() < deadline, "Beira's quest never went in progress");
        std::thread::sleep(Duration::from_millis(10));
    }
    let log = h.root.join("Client.txt");
    let len = fs::metadata(&log).unwrap().len();

    // Second run over the same store and log: it resumes from the saved
    // position, mid-zone, and never reads the area line again.
    let setup = Setup {
        options: Options {
            log: Some(PathBuf::from("Client.txt")),
            ..Options::default()
        },
        ..Setup::default()
    };
    let h = h.rerun(setup);
    assert_eq!(
        h.stored("position.json").expect("position.json")["offset"],
        len
    );
    h.app.start();
    h.wait_diag(&["log: following", &format!("from offset {len}"), "resuming"]);
    h.wait_diag(&["area from log: G1_2"]);

    append(&log, &cold_reward());
    h.wait_diag(&["quest fixture-beira:", "-> done", "(live)"]);
    assert_eq!(beira_status(&h), Status::Done);
    let diag = h.diag_text();
    assert!(!diag.contains("area unknown: refused"), "{diag}");
    assert!(!diag.contains("Cold Resistance"), "{diag}");
    assert!(h.app.shutdown(Duration::from_secs(2)));
}

#[test]
fn first_start_mid_zone_then_reward_marks_done() {
    // A first run (no saved position) starts at the end of a log that ends
    // mid-zone in G1_2: the area line is before the start.
    let h = harness(log_setup(text(&mid_zone_lines()), false));
    h.app.start();
    h.wait_diag(&["log: following", "no saved position"]);
    let line = h.wait_diag(&["area from log: "]);
    assert!(line.ends_with("area from log: G1_2"), "{line}");

    append(&h.root.join("Client.txt"), &cold_reward());
    h.wait_diag(&["quest fixture-beira:", "-> done", "(live)"]);
    assert_eq!(beira_status(&h), Status::Done);
    assert!(
        !h.diag_text().contains("area unknown: refused"),
        "{}",
        h.diag_text()
    );
    assert!(h.app.shutdown(Duration::from_secs(2)));
}

#[test]
fn area_unknown_refusal_is_logged() {
    // The game restarted after the last area line: the area is unknown.
    let mut lines = mid_zone_lines();
    lines.push("2026/03/10 20:00:00 ***** LOG FILE OPENING *****".to_string());
    let h = harness(log_setup(text(&lines), false));
    h.app.start();
    h.wait_diag(&["log: following", "no saved position"]);
    h.wait_diag(&[
        "area from log: none",
        "a session marker follows the last area line",
    ]);

    // Baseline: the zone's reward with the area unknown is refused, and the
    // refusal names the quest, not the line.
    let log = h.root.join("Client.txt");
    append(&log, &cold_reward());
    h.wait_diag(&["area unknown: refused fixture-beira"]);
    assert_ne!(beira_status(&h), Status::Done);
    assert!(
        !h.diag_text().contains("Cold Resistance"),
        "{}",
        h.diag_text()
    );

    // Positive control, same run: once an area line names G1_2, the same
    // reward finishes the quest and nothing more is refused.
    append(
        &log,
        &["2026/03/10 20:01:01 4900000 7e2c0010 [DEBUG Client 7120] Generating level 2 area \"G1_2\" with seed 7".to_string()],
    );
    append(&log, &cold_reward());
    h.wait_diag(&["quest fixture-beira:", "-> done", "(live)"]);
    assert_eq!(beira_status(&h), Status::Done);
    assert_eq!(
        h.diag_text().matches("area unknown: refused").count(),
        1,
        "{}",
        h.diag_text()
    );
    assert!(h.app.shutdown(Duration::from_secs(2)));
}
