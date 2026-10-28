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
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use qt_app::cli::{self, Options};
use qt_app::discover::{self, LogSource};
use qt_app::{
    App, AppConfig, Clock, Emitter, MonitorMetrics, Placement, QuitFrom, Window, HIDDEN_LINE,
    NO_AREA_LINE, SHOWN_LINE, SHUTDOWN_TIMEOUT,
};
use qt_engine::{QuestAction, QuestData, Status};
use qt_store::Store;
use qt_tailer::{Fingerprint, Line, TailEvent};
use qt_viewmodel::{Frame, Mode, RowChange, RowChangeKind, Scale, StatusKind, View, ViewKind};
use serde_json::{json, Value};

const OWN: &str = "Corisande_Fell";

// ---------------------------------------------------------------------------
// Fakes.

#[derive(Default)]
struct Recorder {
    views: Mutex<Vec<View>>,
    row_changes: Mutex<Vec<RowChange>>,
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

    fn row_change(&self, change: &RowChange) {
        self.0.row_changes.lock().unwrap().push(change.clone());
    }
}

#[derive(Default)]
struct WindowLog {
    calls: Mutex<Vec<String>>,
    hotkey_fails: AtomicBool,
    /// The monitor the fake window reports; `None` is no monitor.
    monitor: Mutex<Option<MonitorMetrics>>,
    /// The pid of the window in front; `None` is none known.
    foreground: Mutex<Option<u32>>,
    /// The window calls that block while the gate is closed for them.
    gate: Mutex<Gate>,
    gate_moved: Condvar,
}

/// Which window calls block, and which callers are blocked now. A closed
/// call waits the way Tauri's window getters do on a thread other than the
/// main one: until the main thread answers, which it cannot while it is
/// inside `App::shutdown`. It gives up after [`GATE_LIMIT`], so a test that
/// fails cannot hang the suite.
#[derive(Default)]
struct Gate {
    closed_for: Vec<&'static str>,
    waiting: Vec<&'static str>,
}

/// The longest a closed window call blocks.
const GATE_LIMIT: Duration = Duration::from_secs(5);

impl WindowLog {
    /// Blocks `call` while the gate is closed for it, up to [`GATE_LIMIT`].
    fn pass_gate(&self, call: &'static str) {
        let deadline = Instant::now() + GATE_LIMIT;
        let mut gate = self.gate.lock().unwrap();
        if !gate.closed_for.contains(&call) {
            return;
        }
        gate.waiting.push(call);
        self.gate_moved.notify_all();
        while gate.closed_for.contains(&call) {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                break;
            }
            gate = self.gate_moved.wait_timeout(gate, left).unwrap().0;
        }
        if let Some(i) = gate.waiting.iter().position(|c| *c == call) {
            gate.waiting.remove(i);
        }
    }

    fn close_gate(&self, calls: &[&'static str]) {
        self.gate.lock().unwrap().closed_for = calls.to_vec();
    }

    /// Opens the gate and releases every blocked call.
    fn open_gate(&self) {
        self.gate.lock().unwrap().closed_for.clear();
        self.gate_moved.notify_all();
    }

    /// Waits up to five seconds for a caller blocked in `call`.
    fn wait_blocked_in(&self, call: &'static str) {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut gate = self.gate.lock().unwrap();
        while !gate.waiting.contains(&call) {
            let left = deadline.saturating_duration_since(Instant::now());
            assert!(!left.is_zero(), "no caller blocked in {call} within 5 s");
            gate = self.gate_moved.wait_timeout(gate, left).unwrap().0;
        }
    }

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
        // Recorded, so a test can see when the monitor was read.
        self.0.calls.lock().unwrap().push("monitor".to_string());
        Ok(*self.0.monitor.lock().unwrap())
    }

    // Not recorded: the worker reads it on every tick.
    fn foreground_pid(&self) -> Option<u32> {
        self.0.pass_gate("foreground_pid");
        *self.0.foreground.lock().unwrap()
    }

    fn set_visible(&self, shown: bool) -> Result<(), String> {
        self.0.pass_gate("set_visible");
        self.0.calls.lock().unwrap().push(format!("visible {shown}"));
        Ok(())
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
        screen: None,
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

/// A new character's first minutes: The Riverbank at area level 1 starts
/// it, and its first level-up names it.
fn own_character_lines() -> Vec<String> {
    vec![
        "2026/03/10 19:00:00 ***** LOG FILE OPENING *****".to_string(),
        "2026/03/10 19:00:05 4005000 7e2c0006 [DEBUG Client 7120] Generating level 1 area \"G1_1\" with seed 2077351921".to_string(),
        format!("2026/03/10 19:00:20 4020000 7e2c0001 [INFO Client 7120] : {OWN} has received +20 to maximum Life."),
        format!("2026/03/10 19:00:40 4040000 7e2c0002 [INFO Client 7120] : {OWN} (Witch) is now level 9"),
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

/// What can be open over the quest list: the two frames and a quest's
/// detail.
#[derive(Debug, Clone, Copy)]
enum Opened {
    Settings,
    Picker,
    Detail,
}

impl Opened {
    const ALL: [Opened; 3] = [Opened::Settings, Opened::Picker, Opened::Detail];

    fn open(self, app: &App) -> View {
        match self {
            Opened::Settings => app.open_frame(Some(Frame::Settings)),
            Opened::Picker => app.open_frame(Some(Frame::CharacterPicker)),
            Opened::Detail => app.open_detail(Some("fixture-riverbank".to_string())),
        }
        .unwrap()
    }

    fn frame(self) -> Frame {
        match self {
            Opened::Settings => Frame::Settings,
            Opened::Picker => Frame::CharacterPicker,
            Opened::Detail => Frame::QuestDetail,
        }
    }
}

/// The quest list's frame for a view kind (the fixture's act is not done).
fn list_frame(kind: ViewKind) -> Frame {
    match kind {
        ViewKind::NextUp => Frame::PanelNextUp,
        ViewKind::Campaign => Frame::PanelCampaign,
    }
}

#[test]
fn entering_edit_mode_keeps_the_open_frame_baseline() {
    // Baseline, true before and after the change: going from play to edit
    // changes no frame, view kind or collapse, by either route.
    for opened in Opened::ALL {
        for shortcut in [false, true] {
            let h = harness(Setup::default());
            h.app.handle_tail_events(events(&own_character_lines(), false));
            h.app.set_view_kind(ViewKind::Campaign).unwrap();
            assert_eq!(opened.open(&h.app).frame, opened.frame());

            let v = if shortcut {
                h.app.toggle_mode().unwrap()
            } else {
                h.app.set_mode(Mode::Edit).unwrap()
            };
            assert_eq!(v.mode, Mode::Edit);
            assert_eq!(v.frame, opened.frame(), "{opened:?}, shortcut {shortcut}");
            assert_eq!(v.view_kind, ViewKind::Campaign);
        }
    }

    // The collapsed badge stays collapsed, and what was open under it stays
    // open once it is unfolded.
    let h = harness(Setup::default());
    h.app.handle_tail_events(events(&own_character_lines(), false));
    h.app.open_frame(Some(Frame::Settings)).unwrap();
    assert_eq!(h.app.set_collapsed(true).unwrap().frame, Frame::Settings);
    h.app.open_frame(None).unwrap();
    h.app.open_detail(Some("fixture-riverbank".to_string())).unwrap();
    assert_eq!(h.views.last().frame, Frame::Collapsed);
    let v = h.app.set_mode(Mode::Edit).unwrap();
    assert_eq!(v.frame, Frame::Collapsed);
    assert_eq!(h.app.set_collapsed(false).unwrap().frame, Frame::QuestDetail);
}

#[test]
fn leaving_edit_mode_shows_the_quest_list() {
    // Target: with Settings, the character picker or a quest's detail open
    // in edit mode, every switch to play (the command, the shortcut, the
    // footer's "back to play", which sends the same `set_mode`) shows the
    // quest list of the view kind in force.
    #[derive(Debug, Clone, Copy)]
    enum Leave {
        SetMode,
        Shortcut,
        Footer,
    }
    for kind in [ViewKind::NextUp, ViewKind::Campaign] {
        for opened in Opened::ALL {
            for leave in [Leave::SetMode, Leave::Shortcut, Leave::Footer] {
                let h = harness(Setup::default());
                h.app.handle_tail_events(events(&own_character_lines(), false));
                h.app.set_view_kind(kind).unwrap();
                let act = h.views.last().acts[0].act;
                h.app.toggle_act(act).unwrap();
                h.app.set_mode(Mode::Edit).unwrap();
                assert_eq!(opened.open(&h.app).frame, opened.frame());

                let v = match leave {
                    Leave::SetMode | Leave::Footer => h.app.set_mode(Mode::Play).unwrap(),
                    Leave::Shortcut => h.app.toggle_mode().unwrap(),
                };
                let what = format!("{kind:?}, {opened:?}, {leave:?}");
                assert_eq!(v.mode, Mode::Play, "{what}");
                assert_eq!(v.frame, list_frame(kind), "{what}");
                assert!(v.detail.is_none(), "{what}: the detail closed");
                assert_eq!(v.view_kind, kind, "{what}: the view kind is kept");
                assert!(!v.acts[0].expanded, "{what}: the folded act stays folded");
                assert_eq!(h.views.last(), v, "{what}: the returned view is the one announced");
            }
        }
    }

    // The collapsed badge stays collapsed; what edit mode had open under it
    // is closed once it is unfolded.
    for opened in Opened::ALL {
        let h = harness(Setup::default());
        h.app.handle_tail_events(events(&own_character_lines(), false));
        h.app.set_mode(Mode::Edit).unwrap();
        opened.open(&h.app);
        h.app.set_collapsed(true).unwrap();
        let v = h.app.set_mode(Mode::Play).unwrap();
        assert_eq!(v.frame, Frame::Collapsed, "{opened:?}: the badge is kept");
        let v = h.app.set_collapsed(false).unwrap();
        assert_eq!(v.frame, Frame::PanelNextUp, "{opened:?}");
    }

    // Negative control: in edit mode, commands that are not a switch to play
    // leave the open frame and the detail alone.
    let h = harness(Setup::default());
    h.app.handle_tail_events(events(&own_character_lines(), false));
    h.app.set_mode(Mode::Edit).unwrap();
    h.app.open_detail(Some("fixture-riverbank".to_string())).unwrap();
    h.app.open_frame(Some(Frame::Settings)).unwrap();
    assert_eq!(h.app.set_view_kind(ViewKind::Campaign).unwrap().frame, Frame::Settings);
    assert_eq!(h.app.set_mode(Mode::Edit).unwrap().frame, Frame::Settings);
    assert_eq!(h.app.open_frame(None).unwrap().frame, Frame::QuestDetail);
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

    // Positive controls: the log was written, with places and the own
    // character by its token.
    assert!(log.contains("session start"), "{log}");
    assert!(log.contains("area G1_2"), "{log}");
    assert!(log.contains("catch-up:"), "{log}");
    assert!(log.contains(&logged(OWN)), "{log}");
    assert!(log.contains("an unconfirmed character"), "{log}");

    for forbidden in [
        // the player's own name
        OWN,
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

// ---------------------------------------------------------------------------
// No dock side (owner, after Live 2): a window with no saved position goes
// where dock-right put it, old settings files that carry `dock` still load,
// and a fixed scale is any factor of the slider.

fn places(h: &Harness) -> Vec<String> {
    h.window
        .calls()
        .into_iter()
        .filter(|c| c.starts_with("place"))
        .collect()
}

/// A store folder holding a `settings.json` with `settings`, written by the
/// store itself, as an earlier version would have left it.
fn store_with_settings(settings: Value) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("temp dir");
    Store::open(dir.path().join("store"))
        .expect("store")
        .save_settings(&settings)
        .expect("settings written");
    dir
}

#[test]
fn no_saved_position_places_at_the_default() {
    let default = format!("place {:?}", Placement::Default);
    // Baseline: a first run has no position: the default place (the right
    // side, below the minimap), before and after the zoom.
    let h = harness(Setup::default());
    h.app.start();
    assert_eq!(places(&h), vec![default.clone(), default.clone()]);

    // An edit-mode drag is kept, and the next run goes there, once.
    h.app.set_mode(Mode::Edit).unwrap();
    h.clock.fetch_add(5_000, Ordering::SeqCst);
    h.app.window_moved(10, 20);
    let h = h.rerun(Setup::default());
    h.app.start();
    assert_eq!(places(&h), vec![format!("place {:?}", Placement::At { x: 10, y: 20 })]);
    assert!(h.app.shutdown(Duration::from_secs(2)));

    // An old file that docked left, with no position: the default place
    // all the same; the old side is not kept.
    let h = harness_in(store_with_settings(json!({ "dock": "left" })), Setup::default());
    h.app.start();
    assert_eq!(places(&h), vec![default.clone(), default]);
    assert!(h.app.shutdown(Duration::from_secs(2)));
}

#[test]
fn old_settings_with_dock_load() {
    // A settings file from before Live 2: a dock side, the old three-way
    // scale as a string, a dragged position.
    let dir = store_with_settings(json!({
        "opacity": 0.4,
        "dock": "left",
        "scale": "1.33",
        "show_skipped": true,
        "only_worth_it": false,
        "window": { "x": 10, "y": 20 }
    }));
    let h = harness_in(dir, Setup::default());
    let v = h.app.get_view();
    // Loaded whole: nothing refused, nothing reset to its default.
    // (The status line says only that this test has no log.)
    assert!(
        v.status.as_ref().is_none_or(|s| s.text.contains("No Client.txt")),
        "{:?}",
        v.status
    );
    assert_eq!(
        (v.settings.opacity, v.settings.scale, v.settings.show_skipped),
        (0.4, Scale::fixed(1.33), true)
    );
    h.app.start();
    let calls = h.window.calls();
    assert!(calls.contains(&format!("place {:?}", Placement::At { x: 10, y: 20 })), "{calls:?}");
    assert!(calls.contains(&"scale 1.33".to_string()), "{calls:?}");

    // A patch that names the dock side is refused as unknown, and changes
    // nothing.
    let err = h.app.update_settings(&json!({ "dock": "right" })).unwrap_err();
    assert!(err.contains("unknown setting"), "{err}");
    let err = h
        .app
        .update_settings(&json!({ "opacity": 0.9, "dock": "left" }))
        .unwrap_err();
    assert!(err.contains("dock"), "{err}");
    assert_eq!(h.app.get_view().settings.opacity, 0.4);

    // The next save writes no dock key, and keeps the position.
    h.clock.fetch_add(5_000, Ordering::SeqCst);
    h.app.update_settings(&json!({ "opacity": 0.5 })).unwrap();
    let stored = h.stored("settings.json").expect("settings.json");
    assert!(stored.get("dock").is_none(), "{stored}");
    assert_eq!(stored["window"], json!({ "x": 10, "y": 20 }));
    assert!(h.app.shutdown(Duration::from_secs(2)));
}

#[test]
fn fixed_scale_sets_that_zoom() {
    let h = harness(Setup {
        monitor: monitor_at(1440, 1.0),
        ..Setup::default()
    });
    h.app.start();
    // Baseline: Auto on a 1440 px monitor, as before.
    assert_eq!(scales(&h), vec!["scale 1.33"]);

    // Target: a fixed factor of the slider is that zoom, below 1 too, as a
    // number or as the old stored strings.
    for (patch, zoom) in [
        (json!(1.25), "scale 1.25"),
        (json!(0.75), "scale 0.75"),
        (json!(2.0), "scale 2"),
        (json!("1"), "scale 1"),
        (json!(1.85), "scale 1.85"),
    ] {
        h.app.update_settings(&json!({ "scale": patch })).unwrap();
        assert_eq!(scales(&h).last().map(String::as_str), Some(zoom), "{patch}");
    }
    assert_eq!(h.app.get_view().settings.scale, Scale::fixed(1.85));
    // A fixed factor never reads the monitor, and a move leaves it.
    let before = h.window.calls().len();
    *h.window.monitor.lock().unwrap() = monitor_at(2160, 1.0);
    h.app.window_moved(50, 50);
    assert_eq!(h.window.calls().len(), before);

    // Out of range: Auto, from the monitor (2160 px: 2).
    h.app.update_settings(&json!({ "scale": 5 })).unwrap();
    assert_eq!(h.app.get_view().settings.scale, Scale::Auto);
    assert_eq!(scales(&h).last().map(String::as_str), Some("scale 2"));
    // Auto is unchanged: back on 1440 px it follows the window.
    *h.window.monitor.lock().unwrap() = monitor_at(1440, 1.0);
    h.app.window_moved(60, 60);
    assert_eq!(scales(&h).last().map(String::as_str), Some("scale 1.33"));

    // The factor survives a relaunch as the same zoom.
    h.app.update_settings(&json!({ "scale": 1.4 })).unwrap();
    let h = h.rerun(Setup::default());
    h.app.start();
    assert_eq!(scales(&h), vec!["scale 1.4"]);
    assert!(h.app.shutdown(Duration::from_secs(2)));
}

fn scales(h: &Harness) -> Vec<String> {
    h.window
        .calls()
        .into_iter()
        .filter(|c| c.starts_with("scale"))
        .collect()
}

fn monitor_at(height: u32, dpi_scale: f64) -> Option<MonitorMetrics> {
    Some(MonitorMetrics { height, dpi_scale })
}

// The `auto` scale is worked out from the window's monitor wherever the
// shell applies the scale: at start, when the setting changes, and when the
// window moves or its DPI scale changes (below).
#[test]
fn auto_scale_applied_at_start_and_on_change() {

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

// At start the monitor is read after the window is placed, so the zoom
// comes from the monitor the window was placed on; a window at its default
// place is placed again once the zoom has resized it.
#[test]
fn auto_scale_read_after_placement() {
    let h = harness(Setup {
        monitor: monitor_at(1440, 1.0),
        ..Setup::default()
    });
    h.app.start();
    let calls = h.window.calls();
    let index = |what: &str| {
        calls
            .iter()
            .position(|c| c == what)
            .unwrap_or_else(|| panic!("no {what:?} in {calls:?}"))
    };
    let docked = format!("place {:?}", Placement::Default);
    let (place, read, scale) = (index(&docked), index("monitor"), index("scale 1.33"));
    assert!(place < read && read < scale, "{calls:?}");
    assert!(
        calls[scale + 1..].contains(&docked),
        "docked again after the zoom: {calls:?}"
    );
    assert!(h.app.shutdown(Duration::from_secs(2)));

    // Negative control: a fixed scale never reads the monitor.
    let h = harness(Setup {
        monitor: monitor_at(1440, 1.0),
        ..Setup::default()
    });
    h.app.update_settings(&json!({ "scale": "2" })).unwrap();
    h.window.calls.lock().unwrap().clear();
    h.app.start();
    let calls = h.window.calls();
    assert!(!calls.contains(&"monitor".to_string()), "{calls:?}");
    assert_eq!(scales(&h), vec!["scale 2"]);
    assert!(h.app.shutdown(Duration::from_secs(2)));
}

// A move re-evaluates `auto` on the window's monitor, in any mode, and
// applies a zoom only when it differs; a DPI scale change does the same. A
// fixed scale never follows.
#[test]
fn auto_scale_follows_window_to_other_monitor() {
    let h = harness(Setup {
        monitor: monitor_at(1080, 1.0),
        ..Setup::default()
    });
    h.app.start();
    assert_eq!(scales(&h), vec!["scale 1"]);

    // Baseline: a move on the same monitor (the shell's own placement, in
    // play mode) applies nothing.
    h.app.window_moved(1500, 300);
    assert_eq!(scales(&h), vec!["scale 1"]);

    // Target: the window is now on a 1440 px monitor; one move gives one
    // new zoom, and the window at its default place is placed again.
    *h.window.monitor.lock().unwrap() = monitor_at(1440, 1.0);
    let before = h.window.calls().len();
    h.app.window_moved(3000, 300);
    assert_eq!(scales(&h), vec!["scale 1", "scale 1.33"]);
    let after: Vec<String> = h.window.calls()[before..].to_vec();
    assert_eq!(
        after,
        vec![
            "monitor".to_string(),
            "scale 1.33".to_string(),
            format!("place {:?}", Placement::Default),
        ]
    );
    assert!(h.diag_text().contains("scale auto: the window is now on a monitor 1440 px high"));
    // A second move on the same monitor gives none.
    h.app.window_moved(3010, 300);
    assert_eq!(scales(&h), vec!["scale 1", "scale 1.33"]);
    // Play-mode moves are not saved as a position.
    h.clock.fetch_add(5_000, Ordering::SeqCst);
    h.app.update_settings(&json!({ "opacity": 0.5 })).unwrap();
    assert!(h.stored("settings.json").unwrap().get("window").is_none());

    // A DPI scale change on the same monitor (Windows' setting to 125%).
    *h.window.monitor.lock().unwrap() = monitor_at(1440, 1.25);
    h.app.follow_monitor();
    assert_eq!(scales(&h), vec!["scale 1", "scale 1.33", "scale 1.06"]);

    // An edit-mode drag to a 2160 px monitor: rescaled, the position kept,
    // and not placed again.
    h.app.set_mode(Mode::Edit).unwrap();
    *h.window.monitor.lock().unwrap() = monitor_at(2160, 1.0);
    let before = h.window.calls().len();
    h.clock.fetch_add(5_000, Ordering::SeqCst);
    h.app.window_moved(4000, 200);
    assert_eq!(h.window.calls()[before..].to_vec(), vec!["monitor", "scale 2"]);
    assert_eq!(
        h.stored("settings.json").unwrap().get("window"),
        Some(&json!({ "x": 4000, "y": 200 }))
    );

    // Negative control: a fixed scale ignores the monitor and never reads it.
    h.app.update_settings(&json!({ "scale": "1" })).unwrap();
    *h.window.monitor.lock().unwrap() = monitor_at(1080, 1.0);
    let before = h.window.calls().len();
    h.app.window_moved(100, 100);
    h.app.follow_monitor();
    assert_eq!(h.window.calls().len(), before);
    assert_eq!(scales(&h).last().map(String::as_str), Some("scale 1"));
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

// ---------------------------------------------------------------------------
// After Live 1: row changes, no old-path lines, the area after catch-up, and
// the new character. (The start decided once is `qt-tailer`'s
// `seed_reads_to_the_tailers_own_start`.)

fn row_changes(h: &Harness) -> Vec<RowChange> {
    h.views.row_changes.lock().unwrap().clone()
}

fn g1_2_entry() -> String {
    "2026/03/10 19:04:01 4241000 7e2c0008 [DEBUG Client 7120] Generating level 2 area \"G1_2\" with seed 2077351921".to_string()
}

#[test]
fn live_transition_emits_row_change() {
    let h = harness(Setup::default());
    // Baseline: catch-up moves quests but changes no row on screen.
    h.app.handle_tail_events(events(&own_character_lines(), true));
    assert_eq!(h.views.last().character.as_ref().unwrap().name, OWN);
    assert!(row_changes(&h).is_empty(), "{:?}", row_changes(&h));

    // Live: entering Clearfell makes Beira's quest available, and her
    // reward line there finishes it.
    h.app.handle_tail_events(events(&[g1_2_entry()], false));
    let changes = row_changes(&h);
    let first = changes
        .iter()
        .find(|c| c.quest_id == BEIRA)
        .cloned()
        .unwrap_or_else(|| panic!("no row change for {BEIRA}: {changes:?}"));
    assert_eq!(first.kind, RowChangeKind::Available);
    assert!(!first.text.is_empty());

    h.app.handle_tail_events(events(&cold_reward(), false));
    let changes = row_changes(&h);
    let done = changes.last().cloned().expect("a row change for the reward");
    assert_eq!((done.kind, done.quest_id.as_str()), (RowChangeKind::QuestDone, BEIRA));
    assert!(done.id > first.id, "ids increase: {changes:?}");

    // Each one is in the diagnostic log by id, kind and quest only.
    let diag = h.diag_text();
    assert!(
        diag.contains(&format!("row change {} available ({BEIRA})", first.id)),
        "{diag}"
    );
    assert!(
        diag.contains(&format!("row change {} quest_done ({BEIRA})", done.id)),
        "{diag}"
    );
    assert!(!diag.contains("toast"), "{diag}");

    // Negative control: the same reward again, as catch-up, sends nothing.
    let n = row_changes(&h).len();
    h.app.handle_tail_events(events(&cold_reward(), true));
    assert_eq!(row_changes(&h).len(), n);
}

#[test]
fn path_change_drops_old_tailer_lines() {
    let h = harness(Setup {
        candidates: vec![PathBuf::from("Client.txt")],
        files: vec![("Client.txt", String::new()), ("Other.txt", String::new())],
        ..Setup::default()
    });
    h.app.start();
    h.wait_diag(&["log: following", "Client.txt"]);
    let old = h.app.tail_generation();

    // The log path changes: the first tailer is stopped and another starts.
    let other = h.root.join("Other.txt");
    h.app
        .update_settings(&json!({ "log_path": other.to_string_lossy() }))
        .unwrap();
    h.wait_diag(&["log: following", "Other.txt"]);
    assert!(h.app.tail_generation() > old);

    // What the first tailer had queued before it stopped arrives now.
    let queued: Vec<(u64, TailEvent)> = events(&own_character_lines(), false)
        .into_iter()
        .map(|e| (old, e))
        .collect();
    h.app.handle_tail_events_of(queued.clone());
    assert!(
        h.app.get_view().character.is_none(),
        "an old line reached the engine"
    );
    h.wait_diag(&["events of a stopped log reader dropped"]);
    // Nothing became the new path's position: a save now writes none.
    h.clock
        .fetch_add(10 * qt_app::SAVE_INTERVAL_MS, Ordering::SeqCst);
    h.app.set_view_kind(ViewKind::Campaign).unwrap();
    assert!(
        h.stored("position.json").is_none(),
        "{:?}",
        h.stored("position.json")
    );

    // Positive control: the same lines from the tailer running now count,
    // and their position is the new path's.
    let current = h.app.tail_generation();
    h.app
        .handle_tail_events_of(queued.into_iter().map(|(_, e)| (current, e)).collect());
    assert_eq!(h.app.get_view().character.unwrap().name, OWN);
    assert!(h.app.shutdown(Duration::from_secs(2)));
    let pos = h.stored("position.json").expect("position.json");
    assert_eq!(
        Path::new(pos["path"].as_str().unwrap()).file_name().unwrap(),
        "Other.txt"
    );
}

#[test]
fn catch_up_logs_the_area_it_left() {
    // First run: the session up to mid-zone in G1_2, read from the start.
    let h = harness(log_setup(text(&mid_zone_lines()), true));
    h.app.start();
    h.views.wait_for("naming the character", |v| {
        v.character.as_ref().is_some_and(|c| c.name == OWN)
    });
    let deadline = Instant::now() + Duration::from_secs(5);
    while beira_status(&h) != Status::InProgress {
        assert!(Instant::now() < deadline, "Beira's quest never went in progress");
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(h.app.shutdown(Duration::from_secs(2)));
    let log = h.root.join("Client.txt");
    let len = fs::metadata(&log).unwrap().len();

    // While the overlay is closed, the player walks on to G1_3 (Live 1).
    append(
        &log,
        &["2026/03/10 19:07:30 4450000 7e2c000c [DEBUG Client 7120] Generating level 3 area \"G1_3\" with seed 1234".to_string()],
    );
    let setup = Setup {
        options: Options {
            log: Some(PathBuf::from("Client.txt")),
            ..Options::default()
        },
        ..Setup::default()
    };
    let h = harness_in(h._dir, setup);
    assert_eq!(h.stored("position.json").expect("position.json")["offset"], len);
    h.app.start();
    // The seed reads up to the saved position: G1_2.
    h.wait_diag(&["area from log: G1_2"]);
    // The catch-up reads the G1_3 line, and its summary says so.
    let line = h.wait_diag(&["catch-up: 1 lines read"]);
    assert!(line.contains(", character "), "{line}");
    assert!(line.ends_with(", area G1_3"), "{line}");
    // No live area line was written for it: the summary is the trace.
    assert!(!h.diag_text().contains("area G1_3 (level"), "{}", h.diag_text());
    assert!(h.app.shutdown(Duration::from_secs(2)));
}

#[test]
fn riverbank_new_character_is_logged() {
    const NEW: &str = "character: a new character (The Riverbank)";
    let riverbank = || {
        vec![
            "2026/03/12 10:00:00 ***** LOG FILE OPENING *****".to_string(),
            "2026/03/12 10:00:05 6005000 9a1c0001 [DEBUG Client 7120] Generating level 1 area \"G1_1\" with seed 31".to_string(),
        ]
    };

    // Read as catch-up, the start is logged too (launch 7), once the relog
    // window has closed (launch 8): the catch-up summary, written while it
    // is open, names no character yet.
    let h = harness(Setup::default());
    h.app.handle_tail_events(events(&riverbank(), true));
    let diag = h.diag_text();
    assert_eq!(diag.matches(NEW).count(), 0, "{diag}");
    assert!(diag.contains("character none, area G1_1"), "{diag}");
    quiet_for(&h, 6_000);
    let diag = h.diag_text();
    assert_eq!(diag.matches(NEW).count(), 1, "{diag}");
    assert!(h.app.get_view().character.expect("the new character").provisional);

    // Live: the line is logged when the window closes on the worker's tick,
    // the view shows the provisional character, and the level-up names it.
    let h = harness(Setup::default());
    h.app.handle_tail_events(events(&riverbank(), false));
    assert_eq!(h.diag_text().matches(NEW).count(), 0, "{}", h.diag_text());
    quiet_for(&h, 6_000);
    assert_eq!(h.diag_text().matches(NEW).count(), 1, "{}", h.diag_text());
    let c = h.views.last().character.expect("the new character");
    assert!(c.provisional && c.name.is_empty(), "{c:?}");
    h.app.handle_tail_events(events(
        &[format!(
            "2026/03/12 10:01:00 6060000 9a1c0002 [INFO Client 7120] : {OWN} (Witch) is now level 2"
        )],
        false,
    ));
    let c = h.views.last().character.expect("the named character");
    assert_eq!((c.name.as_str(), c.provisional), (OWN, false));

    // A pinned character: the next Riverbank start, in another instance
    // (the same seed is the named character's relog), releases the pin, and
    // says so, when its window closes.
    h.app.pin_character(Some(OWN)).unwrap();
    h.app.handle_tail_events(events(&riverbank_at("10:02", 32), false));
    assert_eq!(h.diag_text().matches(NEW).count(), 1, "{}", h.diag_text());
    quiet_for(&h, 6_000);
    let diag = h.diag_text();
    assert_eq!(diag.matches(NEW).count(), 2, "{diag}");
    assert!(
        diag.contains(&format!("character: pin on {} released for the new character", logged(OWN))),
        "{diag}"
    );

    // Negative control: another area at level 1, or The Riverbank at a
    // higher area level, starts nothing.
    let h = harness(Setup::default());
    h.app.handle_tail_events(events(
        &[
            "2026/03/12 11:00:00 ***** LOG FILE OPENING *****".to_string(),
            "2026/03/12 11:00:05 7005000 9a1c0003 [DEBUG Client 7120] Generating level 1 area \"G1_town\" with seed 32".to_string(),
            "2026/03/12 11:00:09 7009000 9a1c0004 [DEBUG Client 7120] Generating level 40 area \"G1_1\" with seed 33".to_string(),
        ],
        false,
    ));
    quiet_for(&h, 6_000);
    assert!(!h.diag_text().contains(NEW), "{}", h.diag_text());
}

/// The log stays quiet for `ms` on the shell's test clock, and then the
/// worker's housekeeping tick runs.
fn quiet_for(h: &Harness, ms: u64) {
    h.clock.fetch_add(ms, Ordering::SeqCst);
    h.app.tick();
}

// ---------------------------------------------------------------------------
// After launch 6: the new character's record across a relaunch, the
// character lines in catch-up, the Riverbank relog, and what else a
// relaunch keeps.

const NEW_LINE: &str = "character: a new character (The Riverbank)";

/// A session start at `at` (`HH:MM`) and, 5 s later, a level-1 entry to The
/// Riverbank instance `seed`.
fn riverbank_at(at: &str, seed: u64) -> Vec<String> {
    vec![
        format!("2026/03/12 {at}:00 ***** LOG FILE OPENING *****"),
        format!("2026/03/12 {at}:05 6005000 9a1c0001 [DEBUG Client 7120] Generating level 1 area \"G1_1\" with seed {seed}"),
    ]
}

/// The character build line the client writes for `name` at `at`
/// (`HH:MM:SS`).
fn build_line(at: &str, name: &str) -> String {
    format!(
        "2026/03/12 {at} 6006000 9a1c0002 [WARN Client 7120] Character {name} has unknown build of path: None"
    )
}

/// How the diagnostic log writes the own character `name`: by its token for
/// this run, never by name.
fn logged(name: &str) -> String {
    format!("character #{}", qt_app::diag::token(name))
}

/// Every quest row's id and status in the view, act by act.
fn rows(v: &View) -> Vec<(String, Status)> {
    v.acts
        .iter()
        .flat_map(|a| a.quests.iter().map(|q| (q.id.clone(), q.status)))
        .collect()
}

#[test]
fn provisional_character_survives_relaunch() {
    // First run: a level-1 entry to The Riverbank and no level-up yet.
    let h = harness(log_setup(text(&riverbank_at("10:00", 31)), true));
    // Baseline: nothing saved before the run.
    assert!(h.stored("new-character.json").is_none());
    h.app.start();
    h.wait_diag(&["catch-up:", "area G1_1"]);
    // The log stays quiet past the relog window: the worker's tick starts
    // the new character.
    h.clock.fetch_add(6_000, Ordering::SeqCst);
    h.wait_diag(&[NEW_LINE]);
    let before = h.app.get_view();
    let c = before.character.clone().expect("the new character");
    assert!(c.provisional && c.name.is_empty(), "{c:?}");
    let before_rows = rows(&before);
    assert!(
        before_rows.contains(&("fixture-riverbank".to_string(), Status::InProgress)),
        "{before_rows:?}"
    );

    // Closed with no level-up: the record is saved, open, with its seed.
    let setup = || Setup {
        options: Options {
            log: Some(PathBuf::from("Client.txt")),
            ..Options::default()
        },
        ..Setup::default()
    };
    let h = h.rerun(setup());
    let record = h.stored("new-character.json").expect("new-character.json");
    assert_eq!((record["open"].clone(), record["seed"].clone()), (json!(true), json!(31)));
    // It never names a character: the roster does not hold it.
    assert_eq!(h.stored("characters.json"), Some(json!({})));

    // Second run over the same store and log, before any line is read: the
    // view shows the provisional character with the same rows.
    let after = h.app.get_view();
    let c = after.character.clone().expect("the new character after relaunch");
    assert!(c.provisional && c.name.is_empty(), "{c:?}");
    assert_eq!(rows(&after), before_rows);
    assert!(after.characters.is_empty(), "{:?}", after.characters);
    h.app.start();
    h.wait_diag(&["character: the new character reopened from its record"]);

    // Negative control: a level-up names it, and the record is cleared.
    append(
        &h.root.join("Client.txt"),
        &[format!(
            "2026/03/12 10:01:00 6060000 9a1c0003 [INFO Client 7120] : {OWN} (Witch) is now level 2"
        )],
    );
    h.views.wait_for("the named character", |v| {
        v.character
            .as_ref()
            .is_some_and(|c| c.name == OWN && !c.provisional)
    });
    h.wait_diag(&[&format!("character: the new character named {} by a level-up", logged(OWN))]);
    assert!(h.app.shutdown(Duration::from_secs(2)));
    assert!(h.stored("new-character.json").is_none());
    assert!(!h.store_dir().join("new-character.json").exists());
    // The named character is an ordinary saved one: in the roster, with its
    // progress, and in the picker after a relaunch.
    assert_eq!(h.stored("characters.json").expect("roster")[OWN]["own"], true);
    let h = harness_in(h._dir, setup());
    let v = h.app.get_view();
    assert!(v.characters.iter().any(|c| c.name == OWN && !c.provisional), "{:?}", v.characters);
    assert!(v.character.as_ref().is_none_or(|c| !c.provisional), "{:?}", v.character);

    // A record that cannot be read is set aside and told in the status line.
    let dir = tempfile::tempdir().expect("temp dir");
    fs::create_dir_all(dir.path().join("store")).unwrap();
    fs::write(dir.path().join("store/new-character.json"), "{ not json").unwrap();
    let h = harness_in(dir, Setup::default());
    let status = h.app.get_view().status.expect("a status line");
    assert_eq!(status.kind, StatusKind::Warn, "{status:?}");
    assert!(status.text.contains("could not be read"), "{status:?}");
    assert!(h.app.get_view().character.is_none());
    assert!(!h.store_dir().join("new-character.json").exists());
}

#[test]
fn catch_up_logs_new_character_and_released_pin() {
    let h = harness(Setup::default());
    h.app.handle_tail_events(events(&own_character_lines(), true));
    h.app.pin_character(Some(OWN)).unwrap();
    let base = h.diag_text().matches(NEW_LINE).count();

    // A later Riverbank start read as catch-up: the start and the pin it
    // released are logged, as they are for live lines, once the relog
    // window closes on the worker's tick.
    h.app.handle_tail_events(events(&riverbank_at("10:00", 31), true));
    assert_eq!(h.diag_text().matches(NEW_LINE).count(), base, "{}", h.diag_text());
    quiet_for(&h, 6_000);
    let diag = h.diag_text();
    assert_eq!(diag.matches(NEW_LINE).count(), base + 1, "{diag}");
    let released = format!("character: pin on {} released for the new character", logged(OWN));
    assert_eq!(diag.matches(&released).count(), 1, "{diag}");
    assert!(h.app.get_view().character.unwrap().provisional);

    // A session marker in catch-up parks it and gives the pin back.
    h.app.handle_tail_events(events(
        &["2026/03/12 11:00:00 ***** LOG FILE OPENING *****".to_string()],
        true,
    ));
    let diag = h.diag_text();
    assert!(
        diag.contains(&format!(
            "character: the new character parked at a session start, pin on {} restored", logged(OWN)
        )),
        "{diag}"
    );
    let c = h.app.get_view().character.expect("the pinned character");
    assert_eq!((c.name.as_str(), c.pinned), (OWN, true));

    // The same instance again, in catch-up: resumed, the pin released again.
    h.app.handle_tail_events(events(&riverbank_at("11:01", 31)[1..], true));
    quiet_for(&h, 6_000);
    let diag = h.diag_text();
    assert_eq!(
        diag.matches("character: the new character resumed (The Riverbank)").count(),
        1,
        "{diag}"
    );
    assert_eq!(diag.matches(&released).count(), 2, "{diag}");
    // Resuming is not a second start.
    assert_eq!(diag.matches(NEW_LINE).count(), base + 1, "{diag}");

    // Negative control: a Riverbank entry at a higher area level, in
    // catch-up, writes no character line.
    let h = harness(Setup::default());
    h.app.handle_tail_events(events(
        &["2026/03/12 12:00:05 7005000 9a1c0004 [DEBUG Client 7120] Generating level 40 area \"G1_1\" with seed 33".to_string()],
        true,
    ));
    quiet_for(&h, 6_000);
    assert!(!h.diag_text().contains("character:"), "{}", h.diag_text());
}

#[test]
fn riverbank_relog_is_logged() {
    let relogged = format!("character: {} relogged in The Riverbank, kept", logged(OWN));
    for catch_up in [false, true] {
        let h = harness(Setup::default());
        h.app.handle_tail_events(events(&own_character_lines(), true));
        h.app.pin_character(Some(OWN)).unwrap();

        // A relog inside The Riverbank: the entry, then the build line
        // naming the stored character 1 s later. The pin is kept.
        let base = h.diag_text().matches(NEW_LINE).count();
        let mut lines = riverbank_at("10:00", 31);
        lines.push(build_line("10:00:06", OWN));
        h.app.handle_tail_events(events(&lines, catch_up));
        // A quiet log after it decides nothing more.
        quiet_for(&h, 6_000);
        let diag = h.diag_text();
        assert_eq!(diag.matches(&relogged).count(), 1, "catch-up {catch_up}:\n{diag}");
        assert_eq!(diag.matches(NEW_LINE).count(), base, "catch-up {catch_up}:\n{diag}");
        assert!(!diag.contains("released for the new character"), "{diag}");
        let c = h.app.get_view().character.expect("the kept character");
        assert_eq!(
            (c.name.as_str(), c.pinned, c.provisional),
            (OWN, true, false),
            "catch-up {catch_up}"
        );

        // Negative control: a build line with a name not stored leaves the
        // entry undecided, logs no relog, and never writes the name; the
        // quiet log then starts the new character.
        let mut lines = riverbank_at("11:00", 32);
        lines.push(build_line("11:00:06", "Odile_Marsh"));
        h.app.handle_tail_events(events(&lines, catch_up));
        let diag = h.diag_text();
        assert_eq!(diag.matches(&relogged).count(), 1, "catch-up {catch_up}:\n{diag}");
        assert!(!diag.contains("Odile_Marsh"), "{diag}");
        assert!(!h.app.get_view().character.unwrap().provisional);
        quiet_for(&h, 6_000);
        let diag = h.diag_text();
        assert_eq!(diag.matches(NEW_LINE).count(), base + 1, "catch-up {catch_up}:\n{diag}");
        assert!(h.app.get_view().character.unwrap().provisional);

        // A build line naming the stored character 10 s after the entry is
        // past the window: no relog either.
        let mut lines = riverbank_at("12:00", 33);
        lines.push(build_line("12:00:15", OWN));
        h.app.handle_tail_events(events(&lines, catch_up));
        assert_eq!(h.diag_text().matches(&relogged).count(), 1);
        assert!(h.app.get_view().character.unwrap().provisional);
    }
}

/// Waits up to five seconds for the diagnostic log to hold `needle` exactly
/// `n` times, and returns the log.
fn wait_count(h: &Harness, needle: &str, n: usize) -> String {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let text = h.diag_text();
        if text.matches(needle).count() == n {
            return text;
        }
        assert!(
            Instant::now() < deadline,
            "{needle:?} not {n} times within 5 s:\n{text}"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn relog_window_expires_on_worker_tick() {
    let released = format!("character: pin on {} released for the new character", logged(OWN));
    for catch_up in [false, true] {
        let h = harness(Setup::default());
        h.app.handle_tail_events(events(&own_character_lines(), true));
        h.app.pin_character(Some(OWN)).unwrap();
        let base = h.diag_text().matches(NEW_LINE).count();

        // A level-1 entry to The Riverbank, and no further line.
        h.app.handle_tail_events(events(&riverbank_at("10:00", 31), catch_up));

        // Baseline: up to 5 s after the entry on the test clock (5.999 s,
        // the log's one-second resolution), the tick decides nothing: no
        // new-character line, the pin and the active character kept.
        for ms in [0, 2_000, 3_999] {
            quiet_for(&h, ms);
            let diag = h.diag_text();
            assert_eq!(diag.matches(NEW_LINE).count(), base, "catch-up {catch_up}:\n{diag}");
            assert!(!diag.contains(&released), "catch-up {catch_up}:\n{diag}");
            let c = h.app.get_view().character.expect("the kept character");
            assert_eq!(
                (c.name.as_str(), c.pinned, c.provisional),
                (OWN, true, false),
                "catch-up {catch_up}"
            );
        }

        // Target: past 5 s the tick closes the window, and the new
        // character starts, releasing the pin, with its lines written once.
        quiet_for(&h, 1);
        let diag = h.diag_text();
        assert_eq!(diag.matches(NEW_LINE).count(), base + 1, "catch-up {catch_up}:\n{diag}");
        assert_eq!(diag.matches(&released).count(), 1, "catch-up {catch_up}:\n{diag}");
        let v = h.views.last();
        assert!(v.character.as_ref().is_some_and(|c| c.provisional), "{:?}", v.character);
        assert!(v.characters.iter().all(|c| !c.pinned), "{:?}", v.characters);

        // Later ticks write it no second time.
        quiet_for(&h, 10_000);
        quiet_for(&h, 10_000);
        let diag = h.diag_text();
        assert_eq!(diag.matches(NEW_LINE).count(), base + 1, "catch-up {catch_up}:\n{diag}");
        assert_eq!(diag.matches(&released).count(), 1, "catch-up {catch_up}:\n{diag}");
    }

    // Through the running worker: a log that goes quiet after a live
    // Riverbank entry is decided by the worker's own tick once the test
    // clock passes 5 s, not before.
    let h = harness(log_setup(text(&own_character_lines()), true));
    h.app.start();
    h.wait_diag(&["catch-up:"]);
    // The catch-up may reach the worker in more than one batch, each with
    // its own summary line: wait for the level-up that names the character.
    h.views.wait_for("the named character", |v| {
        v.characters.iter().any(|c| c.name == OWN)
    });
    h.app.pin_character(Some(OWN)).unwrap();
    let base = h.diag_text().matches(NEW_LINE).count();
    append(&h.root.join("Client.txt"), &riverbank_at("10:00", 31));
    h.wait_diag(&["area G1_1 (level 1)"]);
    // Several worker ticks (every 250 ms) with the clock standing still.
    std::thread::sleep(Duration::from_millis(600));
    assert_eq!(h.diag_text().matches(NEW_LINE).count(), base, "{}", h.diag_text());
    h.clock.fetch_add(6_000, Ordering::SeqCst);
    wait_count(&h, NEW_LINE, base + 1);
    std::thread::sleep(Duration::from_millis(600));
    let diag = h.diag_text();
    assert_eq!(diag.matches(NEW_LINE).count(), base + 1, "{diag}");
    assert_eq!(diag.matches(&released).count(), 1, "{diag}");
    h.views.wait_for("the new character", |v| {
        v.character.as_ref().is_some_and(|c| c.provisional)
    });
    assert!(h.app.shutdown(Duration::from_secs(2)));
}

#[test]
fn riverbank_relog_writes_no_new_character_line() {
    let relogged = format!("character: {} relogged in The Riverbank, kept", logged(OWN));
    for catch_up in [false, true] {
        let h = harness(Setup::default());
        h.app.handle_tail_events(events(&own_character_lines(), true));
        h.app.pin_character(Some(OWN)).unwrap();
        let base = h.diag_text().matches(NEW_LINE).count();

        // The entry, then within 5 s the build line naming the stored
        // character, in separate batches with the worker ticking between.
        let entry = riverbank_at("10:00", 31);
        h.app.handle_tail_events(events(&entry, catch_up));
        quiet_for(&h, 2_000);
        h.app.handle_tail_events(events(&[build_line("10:00:07", OWN)], catch_up));
        quiet_for(&h, 10_000);
        quiet_for(&h, 10_000);

        let diag = h.diag_text();
        assert_eq!(diag.matches(&relogged).count(), 1, "catch-up {catch_up}:\n{diag}");
        assert_eq!(diag.matches(NEW_LINE).count(), base, "catch-up {catch_up}:\n{diag}");
        assert!(
            !diag.contains(&format!("pin on {} released for the new character", logged(OWN))),
            "catch-up {catch_up}:\n{diag}"
        );
        let v = h.app.get_view();
        let c = v.character.expect("the kept character");
        assert_eq!(
            (c.name.as_str(), c.pinned, c.active, c.provisional),
            (OWN, true, true, false),
            "catch-up {catch_up}"
        );
        // No view in between showed a new character.
        assert!(
            h.views
                .views
                .lock()
                .unwrap()
                .iter()
                .all(|v| v.character.as_ref().is_none_or(|c| !c.provisional)),
            "catch-up {catch_up}"
        );
    }
}

#[test]
fn ticks_skips_and_settings_survive_relaunch() {
    const CANDLES: &str = "fixture-candles";
    const CARAVAN: &str = "fixture-caravan";
    let h = harness(Setup::default());
    h.app.handle_tail_events(events(&own_character_lines(), true));
    // Pinned, so the character is current after the relaunch before any log
    // line names it (the active character is not saved; the pin is).
    h.app.pin_character(Some(OWN)).unwrap();
    // Baseline: defaults, nothing ticked or skipped.
    let v = h.app.get_view();
    assert_eq!(
        (v.settings.scale, v.settings.show_skipped, v.settings.only_worth_it),
        (Scale::Auto, false, false)
    );
    let step = |h: &Harness| {
        h.app.open_detail(Some(CANDLES.to_string())).unwrap().detail.unwrap().steps[0].ticked
    };
    let status = |h: &Harness, id: &str| {
        h.app.open_detail(Some(id.to_string())).unwrap().detail.unwrap().row.status
    };
    assert!(!step(&h));
    assert_ne!(status(&h, CARAVAN), Status::Skipped);

    h.app.tick_step(CANDLES, "light-candles", true).unwrap();
    h.app.quest_action(CARAVAN, QuestAction::Skip).unwrap();
    h.app.set_mode(Mode::Edit).unwrap();
    h.app
        .update_settings(&json!({
            "opacity": 0.5,
            "scale": "2",
            "show_skipped": true,
            "only_worth_it": true
        }))
        .unwrap();
    h.app.window_moved(10, 20);
    assert!(step(&h));

    let h = h.rerun(Setup::default());
    let v = h.app.get_view();
    assert_eq!(v.character.as_ref().map(|c| c.name.as_str()), Some(OWN));
    assert_eq!(v.settings.opacity, 0.5);
    assert_eq!(
        (v.settings.scale, v.settings.show_skipped, v.settings.only_worth_it),
        (Scale::fixed(2.0), true, true)
    );
    assert!(step(&h), "the tick survived");
    assert_eq!(status(&h, CARAVAN), Status::Skipped);
    h.app.start();
    let at = format!("place {:?}", Placement::At { x: 10, y: 20 });
    assert!(h.window.calls().contains(&at), "{:?}", h.window.calls());
    assert!(h.window.calls().contains(&"scale 2".to_string()), "{:?}", h.window.calls());
    assert!(h.app.shutdown(Duration::from_secs(2)));
}

#[test]
fn quest_action_reaches_the_new_character() {
    // Live 3: the panel showed the unnamed new character, and every Skip was
    // refused with "no character yet".
    const CANDLES: &str = "fixture-candles";
    const CARAVAN: &str = "fixture-caravan";
    let detail = |h: &Harness, id: &str| {
        h.app.open_detail(Some(id.to_string())).unwrap().detail.unwrap()
    };

    // Negative control: with no character at all, and no new one, the
    // commands are refused as before and the refusal is logged.
    let h = harness(Setup::default());
    let err = h.app.quest_action(CARAVAN, QuestAction::Skip).expect_err("no character");
    assert!(err.contains("no character yet"), "{err}");
    let err = h.app.tick_step(CANDLES, "light-candles", true).expect_err("no character");
    assert!(err.contains("no character yet"), "{err}");
    assert!(h.diag_text().contains("command quest_action refused: no character yet"));
    assert!(h.diag_text().contains("command tick_step refused: no character yet"));

    // A level-1 entry to The Riverbank, the log quiet past the relog
    // window: the panel shows the new character.
    h.app.handle_tail_events(events(&riverbank_at("10:00", 31), false));
    quiet_for(&h, 6_000);
    let c = h.app.get_view().character.expect("the new character");
    assert!(c.provisional && c.name.is_empty(), "{c:?}");
    // Baseline: nothing skipped or ticked.
    assert_ne!(detail(&h, CARAVAN).row.status, Status::Skipped);
    assert!(!detail(&h, CANDLES).steps[0].ticked);

    // Target: Skip, on a quest not picked up yet, and a tick reach it.
    h.app.quest_action(CARAVAN, QuestAction::Skip).unwrap();
    h.app.tick_step(CANDLES, "light-candles", true).unwrap();
    assert_eq!(detail(&h, CARAVAN).row.status, Status::Skipped);
    assert!(detail(&h, CANDLES).steps[0].ticked);
    // No refusal beyond the negative control's two.
    assert_eq!(h.diag_text().matches(" refused: ").count(), 2, "{}", h.diag_text());

    // The save writes them into its record, and a relaunch keeps them.
    let h = h.rerun(Setup::default());
    let record = h.stored("new-character.json").expect("new-character.json");
    assert_eq!(record["open"], json!(true));
    assert_eq!(record["state"]["overrides"][CARAVAN]["skipped"], json!(true));
    assert_eq!(record["state"]["overrides"][CANDLES]["ticks"], json!(["light-candles"]));
    assert_eq!(h.stored("characters.json"), Some(json!({})));
    let c = h.app.get_view().character.expect("the new character after relaunch");
    assert!(c.provisional, "{c:?}");
    assert_eq!(detail(&h, CARAVAN).row.status, Status::Skipped);
    // Unskip works on it too.
    h.app.quest_action(CARAVAN, QuestAction::Unskip).unwrap();
    assert_ne!(detail(&h, CARAVAN).row.status, Status::Skipped);
    h.app.quest_action(CARAVAN, QuestAction::Skip).unwrap();

    // A level-up names it: the overrides pass to the named character, whose
    // file holds them, and the record is cleared.
    h.app.handle_tail_events(events(
        &[format!(
            "2026/03/12 10:01:00 6060000 9a1c0003 [INFO Client 7120] : {OWN} (Witch) is now level 2"
        )],
        false,
    ));
    let c = h.app.get_view().character.expect("the named character");
    assert_eq!((c.name.as_str(), c.provisional), (OWN, false));
    assert_eq!(detail(&h, CARAVAN).row.status, Status::Skipped);
    assert!(detail(&h, CANDLES).steps[0].ticked);
    assert!(h.app.shutdown(Duration::from_secs(2)));
    assert!(h.stored("new-character.json").is_none());
    let state = h
        .app
        .store()
        .unwrap()
        .load_character(OWN)
        .unwrap()
        .value
        .expect("the character's file");
    assert_eq!(state["overrides"][CARAVAN]["skipped"], json!(true));
    assert_eq!(state["overrides"][CANDLES]["ticks"], json!(["light-candles"]));
}

// ---------------------------------------------------------------------------
// Visible only while the game or the overlay is in front (owner, after Live
// 2). The game's pid is the one in the newest log line's header; the fake
// window reports the pid of the window in front.

/// The pid in `own_character_lines()`' headers.
const GAME: u32 = 7120;
/// Another program's window.
const OTHER: u32 = 9_999;

fn visibility(h: &Harness) -> Vec<String> {
    h.window
        .calls()
        .into_iter()
        .filter(|c| c.starts_with("visible"))
        .collect()
}

fn in_front(h: &Harness, pid: Option<u32>) {
    *h.window.foreground.lock().unwrap() = pid;
    h.app.tick();
}

/// An app that has read the game's lines (pid [`GAME`]), with no worker.
fn game_running() -> Harness {
    let h = harness(Setup::default());
    h.app.handle_tail_events(events(&own_character_lines(), false));
    h
}

#[test]
fn play_mode_hides_when_another_app_is_in_front() {
    let h = game_running();
    // Baseline: the game in front, in play mode: shown, and nothing sent
    // (the window is created shown).
    in_front(&h, Some(GAME));
    assert!(visibility(&h).is_empty());
    assert_eq!(h.app.get_view().mode, Mode::Play);

    // Target: another program in front hides the overlay.
    in_front(&h, Some(OTHER));
    assert_eq!(visibility(&h), vec!["visible false"]);
    // The game in front again shows it.
    in_front(&h, Some(GAME));
    assert_eq!(visibility(&h), vec!["visible false", "visible true"]);
}

#[test]
fn shown_when_the_game_or_the_overlay_is_in_front() {
    let h = game_running();
    in_front(&h, Some(OTHER));
    assert_eq!(visibility(&h), vec!["visible false"]);
    // The overlay's own window in front (this process): shown.
    in_front(&h, Some(std::process::id()));
    assert_eq!(visibility(&h), vec!["visible false", "visible true"]);
    in_front(&h, Some(OTHER));
    in_front(&h, Some(GAME));
    assert_eq!(
        visibility(&h),
        vec!["visible false", "visible true", "visible false", "visible true"]
    );
    // Negative control: with the game in front, ticks change nothing.
    in_front(&h, Some(GAME));
    assert_eq!(visibility(&h).len(), 4);
}

#[test]
fn edit_mode_is_always_shown() {
    let h = game_running();
    in_front(&h, Some(OTHER));
    assert_eq!(visibility(&h), vec!["visible false"]);
    // The shortcut to edit mode: shown, whatever is in front.
    h.app.toggle_mode().unwrap();
    in_front(&h, Some(OTHER));
    assert_eq!(visibility(&h), vec!["visible false", "visible true"]);
    in_front(&h, None);
    in_front(&h, Some(OTHER));
    assert_eq!(visibility(&h).len(), 2);
    // Back to play mode with another program in front: hidden again.
    h.app.set_mode(Mode::Play).unwrap();
    in_front(&h, Some(OTHER));
    assert_eq!(
        visibility(&h),
        vec!["visible false", "visible true", "visible false"]
    );
}

#[test]
fn unknown_game_pid_keeps_the_overlay_shown() {
    let h = harness(Setup::default());
    // No line read: the game's pid is unknown, so the overlay stays shown
    // (fail open) whatever is in front.
    in_front(&h, Some(OTHER));
    // Lines with no client header (a session marker) tell no pid either.
    h.app.handle_tail_events(events(
        &["2026/03/10 19:00:00 ***** LOG FILE OPENING *****".to_string()],
        false,
    ));
    in_front(&h, Some(OTHER));
    assert!(visibility(&h).is_empty());
    assert!(!h.diag_text().contains("visibility:"), "{}", h.diag_text());

    // Nothing known in front, with the game's pid known: shown too.
    h.app.handle_tail_events(events(&own_character_lines(), false));
    in_front(&h, None);
    assert!(visibility(&h).is_empty());
    // Positive control: the same program in front, now that the game's pid
    // is known, hides it.
    in_front(&h, Some(OTHER));
    assert_eq!(visibility(&h), vec!["visible false"]);
}

#[test]
fn game_pid_comes_from_the_newest_log_line() {
    let h = game_running();
    in_front(&h, Some(GAME));
    assert!(visibility(&h).is_empty());
    // The game restarted under another pid: the newest header wins, live or
    // catch-up, even on a line that gives no event.
    h.app.handle_tail_events(events(
        &[
            "2026/03/10 20:00:00 ***** LOG FILE OPENING *****".to_string(),
            "2026/03/10 20:00:01 5000000 7e2c0010 [INFO Client 15660] Connecting to instance server".to_string(),
            // A character in an area, so the session's wait for one is over.
            "2026/03/10 20:00:02 5001000 7e2c0011 [DEBUG Client 15660] Generating level 2 area \"G1_2\" with seed 7".to_string(),
        ],
        true,
    ));
    in_front(&h, Some(GAME));
    assert_eq!(visibility(&h), vec!["visible false"]);
    in_front(&h, Some(15_660));
    assert_eq!(visibility(&h), vec!["visible false", "visible true"]);

    // A start mid-log: the pid is read back from the log before the
    // tailer's start, with the area, before any new line.
    let h = harness(log_setup(text(&own_character_lines()), false));
    h.app.start();
    h.wait_diag(&["area from log:"]);
    *h.window.foreground.lock().unwrap() = Some(OTHER);
    h.wait_diag(&[HIDDEN_LINE]);
    *h.window.foreground.lock().unwrap() = Some(GAME);
    h.wait_diag(&[SHOWN_LINE]);
    assert!(h.app.shutdown(Duration::from_secs(2)));
}

#[test]
fn visibility_change_is_logged_once() {
    // What the fake program in front is called. The window reports only its
    // pid, so this must never reach the log.
    const TITLE: &str = "Notepad - shopping list.txt";
    let h = game_running();
    let lines = |h: &Harness| -> Vec<String> {
        h.diag_text()
            .lines()
            .filter(|l| l.contains("visibility"))
            .map(str::to_string)
            .collect()
    };
    // Baseline: shown from the start, nothing logged.
    in_front(&h, Some(GAME));
    assert!(lines(&h).is_empty());

    for _ in 0..4 {
        in_front(&h, Some(OTHER));
    }
    let log = lines(&h);
    assert_eq!(log.len(), 1, "{log:?}");
    assert!(log[0].ends_with(HIDDEN_LINE), "{log:?}");
    for _ in 0..4 {
        in_front(&h, Some(GAME));
    }
    in_front(&h, Some(std::process::id()));
    let log = lines(&h);
    assert_eq!(log.len(), 2, "{log:?}");
    assert!(log[1].ends_with(SHOWN_LINE), "{log:?}");
    in_front(&h, Some(OTHER));
    let text = h.diag_text();
    assert_eq!(text.matches(HIDDEN_LINE).count(), 2, "{text}");
    assert_eq!(text.matches(SHOWN_LINE).count(), 1, "{text}");
    // Each line is the fixed text: no title, no other program's pid.
    assert!(!text.contains(TITLE), "{text}");
    for l in lines(&h) {
        assert!(l.ends_with(HIDDEN_LINE) || l.ends_with(SHOWN_LINE), "{l}");
        assert!(!l.contains(&OTHER.to_string()), "{l}");
    }
}

// ---------------------------------------------------------------------------
// Hidden until a character is in an area (owner, after Live 4): in play mode,
// from a session marker until that session's first area line, the overlay is
// hidden whatever is in front, so it does not cover the login and
// character-select screens. The lines are built here; the real log is never
// read.

/// A launch up to character select: the session marker and a line with the
/// game's client header (`pid`), but no area line.
fn character_select_lines(at: &str, pid: u32) -> Vec<String> {
    vec![
        format!("2026/03/11 {at}:00 ***** LOG FILE OPENING *****"),
        format!("2026/03/11 {at}:13 6013000 7e2c0020 [INFO Client {pid}] Connecting to instance server"),
    ]
}

/// The session's first area line: a character is in an area.
fn first_area_line(at: &str, pid: u32) -> String {
    format!(
        "2026/03/11 {at}:05 6065000 7e2c0021 [DEBUG Client {pid}] Generating level 15 area \"G1_town\" with seed 4242"
    )
}

/// The visibility lines of the diagnostic log, oldest first.
fn visibility_lines(h: &Harness) -> Vec<String> {
    h.diag_text()
        .lines()
        .filter(|l| l.contains("visibility:"))
        .map(str::to_string)
        .collect()
}

#[test]
fn edit_mode_shows_before_a_character_is_in_an_area() {
    // Baseline: edit mode is shown whatever is in front, so Ctrl+Alt+Q
    // still reaches the settings before a character loads.
    let h = harness(Setup::default());
    h.app.toggle_mode().unwrap();
    assert_eq!(h.app.get_view().mode, Mode::Edit);
    h.app.handle_tail_events(events(&character_select_lines("10:00", GAME), false));
    for front in [Some(GAME), Some(OTHER), None, Some(std::process::id())] {
        in_front(&h, front);
    }
    assert!(visibility(&h).is_empty(), "{:?}", visibility(&h));
    assert!(visibility_lines(&h).is_empty(), "{}", h.diag_text());

    // A character in an area, then play mode with the game in front: shown,
    // as before; another program in front hides it with the existing line.
    h.app.handle_tail_events(events(&[first_area_line("10:01", GAME)], false));
    h.app.set_mode(Mode::Play).unwrap();
    in_front(&h, Some(GAME));
    assert!(visibility(&h).is_empty(), "{:?}", visibility(&h));
    in_front(&h, Some(OTHER));
    assert_eq!(visibility(&h), vec!["visible false"]);
    assert!(visibility_lines(&h)[0].ends_with(HIDDEN_LINE), "{:?}", visibility_lines(&h));
}

#[test]
fn stale_pid_without_an_area_keeps_the_fail_open_rule() {
    // Baseline: started on a log whose last session has a marker and no
    // area line (a game closed at character select, or a log the running
    // game does not write to). Its pid is caught up only; once it goes stale
    // (no live line, no window of it in front within the grace), the
    // overlay shows whatever is in front, as it does today.
    const LOG_PID: u32 = 15_660;
    let h = harness(Setup::default());
    h.app.handle_tail_events(events(&character_select_lines("10:00", LOG_PID), true));
    in_front(&h, Some(OTHER));
    assert_eq!(visibility(&h), vec!["visible false"]);
    h.clock.fetch_add(10_000, Ordering::SeqCst);
    in_front(&h, Some(OTHER));
    assert_eq!(visibility(&h), vec!["visible false", "visible true"]);
    for front in [Some(GAME), Some(OTHER), None, Some(LOG_PID), Some(OTHER)] {
        h.clock.fetch_add(60_000, Ordering::SeqCst);
        in_front(&h, front);
    }
    assert_eq!(visibility(&h), vec!["visible false", "visible true"]);
    let text = h.diag_text();
    assert_eq!(
        text.matches(&format!("game pid: {LOG_PID} not used to hide the overlay")).count(),
        1,
        "{text}"
    );
    assert!(visibility_lines(&h).last().unwrap().ends_with(SHOWN_LINE), "{text}");
}

#[test]
fn overlay_hidden_until_a_character_is_in_an_area() {
    let h = harness(Setup::default());
    // Target: a live session marker, the game in front: hidden whatever is
    // in front, with the new line, once.
    h.app.handle_tail_events(events(&character_select_lines("10:00", GAME), false));
    for front in [Some(GAME), Some(GAME), Some(std::process::id()), None, Some(OTHER)] {
        in_front(&h, front);
    }
    assert_eq!(visibility(&h), vec!["visible false"]);
    assert_eq!(visibility_lines(&h).len(), 1, "{}", h.diag_text());
    assert!(visibility_lines(&h)[0].ends_with(NO_AREA_LINE), "{}", h.diag_text());
    // A launch that writes the marker again only restarts the wait.
    h.app.handle_tail_events(events(&character_select_lines("10:00", GAME), false));
    in_front(&h, Some(GAME));
    assert_eq!(visibility(&h), vec!["visible false"]);
    assert_eq!(visibility_lines(&h).len(), 1, "{}", h.diag_text());

    // The first area line lifts it: the game in front shows it, without
    // activating the window (the fake records only `visible true`).
    h.app.handle_tail_events(events(&[first_area_line("10:01", GAME)], false));
    in_front(&h, Some(GAME));
    assert_eq!(visibility(&h), vec!["visible false", "visible true"]);
    assert!(visibility_lines(&h)[1].ends_with(SHOWN_LINE), "{:?}", visibility_lines(&h));
    // Negative control: from then on the existing rule decides, as before.
    in_front(&h, Some(OTHER));
    in_front(&h, Some(GAME));
    assert_eq!(
        visibility(&h),
        vec!["visible false", "visible true", "visible false", "visible true"]
    );
    assert!(visibility_lines(&h)[2].ends_with(HIDDEN_LINE), "{:?}", visibility_lines(&h));

    // The next session's marker hides it again, the game in front.
    h.app.handle_tail_events(events(&character_select_lines("11:00", GAME), false));
    in_front(&h, Some(GAME));
    assert_eq!(visibility(&h).len(), 5, "{:?}", visibility(&h));
    assert_eq!(h.diag_text().matches(NO_AREA_LINE).count(), 2, "{}", h.diag_text());
    // Lifted while another program is in front: still hidden, now by the
    // existing rule, which its own line says; no window call.
    in_front(&h, Some(OTHER));
    h.app.handle_tail_events(events(&[first_area_line("11:01", GAME)], false));
    in_front(&h, Some(OTHER));
    assert_eq!(visibility(&h).len(), 5, "{:?}", visibility(&h));
    assert!(visibility_lines(&h).last().unwrap().ends_with(HIDDEN_LINE), "{}", h.diag_text());
    in_front(&h, Some(GAME));
    assert_eq!(visibility(&h).last().map(String::as_str), Some("visible true"));
    // No line names the program in front.
    for l in visibility_lines(&h) {
        assert!(!l.contains(&OTHER.to_string()), "{l}");
    }
}

#[test]
fn overlay_hidden_at_character_select_after_a_catch_up_session_marker() {
    // The log's last session reached character select: a marker, a header
    // with the game's pid, and no area line.
    let mut lines = own_character_lines();
    lines.extend(character_select_lines("20:00", GAME));

    // Target: started at the end of that log, the marker is read back
    // before the tailer's start, and with the pid not stale the overlay is
    // hidden with the new line; a live area line shows it.
    let h = harness(log_setup(text(&lines), false));
    *h.window.foreground.lock().unwrap() = Some(GAME);
    h.app.start();
    h.wait_diag(&[NO_AREA_LINE]);
    assert_eq!(visibility(&h), vec!["visible false"]);
    append(&h.root.join("Client.txt"), &[first_area_line("20:01", GAME)]);
    h.wait_diag(&[SHOWN_LINE]);
    assert_eq!(visibility(&h), vec!["visible false", "visible true"]);
    assert!(h.app.shutdown(Duration::from_secs(2)));

    // The same lines read as catch-up (a start at offset 0): the same.
    let h = harness(Setup::default());
    h.app.handle_tail_events(events(&lines, true));
    in_front(&h, Some(GAME));
    assert_eq!(visibility(&h), vec!["visible false"]);
    assert!(visibility_lines(&h)[0].ends_with(NO_AREA_LINE), "{}", h.diag_text());
    h.app.handle_tail_events(events(&[first_area_line("20:01", GAME)], false));
    in_front(&h, Some(GAME));
    assert_eq!(visibility(&h), vec!["visible false", "visible true"]);

    // Negative control: a last session with an area line starts with the
    // hide lifted, and the existing rule decides.
    let h = harness(log_setup(text(&own_character_lines()), false));
    *h.window.foreground.lock().unwrap() = Some(OTHER);
    h.app.start();
    h.wait_diag(&[HIDDEN_LINE]);
    *h.window.foreground.lock().unwrap() = Some(GAME);
    h.wait_diag(&[SHOWN_LINE]);
    assert!(!h.diag_text().contains(NO_AREA_LINE), "{}", h.diag_text());
    assert!(h.app.shutdown(Duration::from_secs(2)));
}

// ---------------------------------------------------------------------------
// A quit request, from the tray or a close of the overlay's window, while the
// overlay is hidden (Live 3's teardown: the process stayed, and nothing was
// logged after `visibility: hidden`).

#[test]
fn quit_is_logged_once_then_stops() {
    const CARAVAN: &str = "fixture-caravan";
    let h = game_running();
    // Hidden in play mode: another program is in front.
    in_front(&h, Some(OTHER));
    assert_eq!(visibility(&h), vec!["visible false"]);
    assert_eq!(h.app.get_view().mode, Mode::Play);
    // A change inside the save interval: not on disk until the exit.
    h.app.quest_action(CARAVAN, QuestAction::Skip).unwrap();
    let skipped = |h: &Harness| {
        h.app
            .store()
            .unwrap()
            .load_character(OWN)
            .unwrap()
            .value
            .is_some_and(|s| s["overrides"][CARAVAN]["skipped"] == json!(true))
    };
    assert!(!skipped(&h));
    // Baseline: no quit line before a request.
    assert!(!h.diag_text().contains("quit:"), "{}", h.diag_text());

    // Two tray requests and a window-closed one: only the first is the
    // caller's to act on, and only it is logged.
    assert!(h.app.request_quit(QuitFrom::Tray));
    assert!(!h.app.request_quit(QuitFrom::Tray));
    assert!(!h.app.request_quit(QuitFrom::WindowClosed));
    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));

    let text = h.diag_text();
    assert_eq!(text.matches("quit: from the tray").count(), 1, "{text}");
    assert!(!text.contains("quit: the window was closed"), "{text}");
    let hidden = text.find(HIDDEN_LINE).expect("the hidden line");
    let quit = text.find("quit: from the tray").unwrap();
    let stop = text.find("stop: tailer and worker ended, progress saved").expect("the stop line");
    assert!(hidden < quit && quit < stop, "{text}");
    // Still hidden: the quit changed nothing on the window.
    assert_eq!(visibility(&h), vec!["visible false"]);
    assert!(skipped(&h), "the store was saved on exit");

    // The window-closed line, for a first request that is a close.
    let h = game_running();
    assert!(h.app.request_quit(QuitFrom::WindowClosed));
    assert!(!h.app.request_quit(QuitFrom::Tray));
    let text = h.diag_text();
    assert_eq!(text.matches("quit: the window was closed").count(), 1, "{text}");
    assert!(!text.contains("quit: from the tray"), "{text}");
}

// Live 4's exits: `stop: tailer ended, worker did not end within 2000 ms`.
// The shell calls `shutdown` on Tauri's main thread, and a window call from
// the worker that waits for the main thread could not end before it.

/// A running app in play mode whose game pid (from the log) is confirmed by
/// the game's window in front, so the worker's ticks reach the window.
fn game_confirmed_by_worker() -> Harness {
    let h = harness(log_setup(text(&own_character_lines()), false));
    h.app.start();
    h.wait_diag(&["area from log:"]);
    *h.window.foreground.lock().unwrap() = Some(GAME);
    h.wait_diag(&[&format!("game pid: {GAME} confirmed")]);
    assert_eq!(h.app.get_view().mode, Mode::Play);
    h
}

/// Shuts `h` down while the worker is blocked in `call`, then releases the
/// fake's waiters: whether `shutdown` reported both threads ended, and how
/// long it took.
fn shutdown_while_blocked_in(h: &Harness, call: &'static str) -> (bool, Duration) {
    h.window.wait_blocked_in(call);
    let started = Instant::now();
    let ended = h.app.shutdown(SHUTDOWN_TIMEOUT);
    let took = started.elapsed();
    h.window.open_gate();
    (ended, took)
}

#[test]
fn shutdown_ends_worker_while_window_calls_wait_for_the_main_thread() {
    // Baseline: with nothing blocked, the worker ticks and the window
    // follows what is in front.
    let h = game_confirmed_by_worker();
    *h.window.foreground.lock().unwrap() = Some(OTHER);
    h.wait_diag(&[HIDDEN_LINE]);
    *h.window.foreground.lock().unwrap() = Some(GAME);
    h.wait_diag(&[SHOWN_LINE]);

    // Target: the worker is inside the read of the window in front, which
    // waits for the main thread, when the main thread enters `shutdown`.
    h.window.close_gate(&["foreground_pid", "set_visible"]);
    let (ended, took) = shutdown_while_blocked_in(&h, "foreground_pid");
    assert!(ended, "shutdown waited {} ms:\n{}", took.as_millis(), h.diag_text());
    assert!(took < SHUTDOWN_TIMEOUT, "{} ms", took.as_millis());
    assert!(!h.app.worker_running());
    let text = h.diag_text();
    assert!(text.contains("stop: tailer and worker ended, progress saved"), "{text}");
    assert!(!text.contains("did not end"), "{text}");

    // The same with the worker inside showing or hiding the window, which it
    // does holding the app's lock that `shutdown`'s save needs too.
    let h = game_confirmed_by_worker();
    h.window.close_gate(&["set_visible"]);
    *h.window.foreground.lock().unwrap() = Some(OTHER);
    let (ended, took) = shutdown_while_blocked_in(&h, "set_visible");
    assert!(ended, "shutdown waited {} ms:\n{}", took.as_millis(), h.diag_text());
    assert!(took < SHUTDOWN_TIMEOUT, "{} ms", took.as_millis());
    let text = h.diag_text();
    assert!(text.contains("stop: tailer and worker ended, progress saved"), "{text}");
    assert!(!text.contains("did not end"), "{text}");
}

// ---------------------------------------------------------------------------
// While a relog window is open the saved log position stays at the start of
// the entry line, so a relaunch reads the entry again and decides it (Round
// 3's finding).

/// Waits up to five seconds for `position.json` to hold `offset`.
fn wait_position(h: &Harness, offset: u64) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let saved = h.stored("position.json").map(|p| p["offset"].clone());
        if saved == Some(json!(offset)) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "position.json holds {saved:?}, not offset {offset}, within 5 s"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn shutdown_inside_relog_window_saves_the_entry_position() {
    // The character's lines, then, live, a new session and a level-1 entry
    // to The Riverbank, the last line of the log.
    let head = text(&own_character_lines());
    let riverbank = riverbank_at("10:00", 31);
    let entry_start = (head.len() + riverbank[0].len() + 1) as u64;
    let end = (head.len() + text(&riverbank).len()) as u64;

    let h = harness(log_setup(head.clone(), true));
    h.app.start();
    h.wait_diag(&["catch-up:"]);
    // (The head's own Riverbank entry started a character, named since.)
    let started = h.diag_text().matches(NEW_LINE).count();
    append(&h.root.join("Client.txt"), &riverbank);
    h.wait_diag(&["area G1_1 (level 1)"]);
    // A debounced save inside the window (2.5 s on the test clock: due, and
    // still inside the 5 s window) keeps the entry's start.
    h.clock.fetch_add(2_500, Ordering::SeqCst);
    wait_position(&h, entry_start);
    assert_eq!(h.diag_text().matches(NEW_LINE).count(), started, "{}", h.diag_text());
    // So does the save on exit.
    let setup = || Setup {
        options: Options {
            log: Some(PathBuf::from("Client.txt")),
            ..Options::default()
        },
        ..Setup::default()
    };
    let h = h.rerun(setup());
    assert_eq!(h.stored("position.json").unwrap()["offset"], json!(entry_start));

    // The relaunch resumes there, reads the entry again (as catch-up) and,
    // the log quiet, decides it: the new character starts, and the position
    // moves to the end of the log.
    let read = h.diag_text().matches("catch-up:").count();
    h.app.start();
    h.wait_diag(&["resuming from the saved position"]);
    wait_count(&h, "catch-up:", read + 1);
    assert_eq!(h.diag_text().matches(NEW_LINE).count(), started, "{}", h.diag_text());
    h.clock.fetch_add(6_000, Ordering::SeqCst);
    wait_count(&h, NEW_LINE, started + 1);
    wait_position(&h, end);
    assert!(h.app.shutdown(Duration::from_secs(2)));
    assert_eq!(h.stored("position.json").unwrap()["offset"], json!(end));

    // Negative control: a log that ends with no relog window open saves its
    // end at once.
    let h = harness(log_setup(head.clone(), true));
    h.app.start();
    h.wait_diag(&["catch-up:"]);
    assert!(h.app.shutdown(Duration::from_secs(2)));
    assert_eq!(h.stored("position.json").unwrap()["offset"], json!(head.len() as u64));
}

#[test]
fn diag_log_never_names_the_player() {
    // A new character from The Riverbank, named by its first level-up: the
    // two character lines the 2026-10-27 live session saw with the name.
    let h = harness(log_setup(text(&riverbank_at("10:00", 31)), true));
    h.app.start();
    h.wait_diag(&["catch-up:", "area G1_1"]);
    h.clock.fetch_add(6_000, Ordering::SeqCst);
    h.wait_diag(&[NEW_LINE]);
    append(
        &h.root.join("Client.txt"),
        &[format!(
            "2026/03/12 10:01:00 6060000 9a1c0003 [INFO Client 7120] : {OWN} (Witch) is now level 2"
        )],
    );
    // Positive control: both lines are written, the character by its token.
    let own = logged(OWN);
    assert!(own.starts_with("character #") && own.len() == "character #".len() + 6, "{own}");
    h.wait_diag(&[&format!("character: the new character named {own} by a level-up")]);
    h.wait_diag(&[&format!("character: none -> {own}")]);
    assert!(h.app.shutdown(Duration::from_secs(2)));
    // The roster holds the name: it was known, and the log still never has it.
    assert_eq!(h.stored("characters.json").expect("roster")[OWN]["own"], true);
    let diag = h.diag_text();
    assert!(!diag.contains(OWN), "overlay.log names the player:
{diag}");
}
