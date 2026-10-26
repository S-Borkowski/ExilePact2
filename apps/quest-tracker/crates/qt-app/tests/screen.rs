//! The screen reader in the shell (Context "Design: the reader thread"),
//! driven by a fake backend that replays a scripted sequence of OCR reads
//! per region and records every call. The fake's copies advance the shell's
//! clock by the region's period, so the reader's schedule runs as fast as
//! the test does. Nothing here captures the screen or opens a window; the
//! quest data is `fixtures/quests_min.json` with a fight step ended by the
//! pack leader's bar going in `G1_2`, as `qt-engine`'s screen tests add it.
//! Character names are invented.
//!
//! Baseline: with `screen_reading` off no reader starts and the backend is
//! never called. Target: with it on, in `G1_2` with the first step reached,
//! the bar's going ticks the fight step, with its row change and line.

use std::collections::{BTreeMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use qt_app::screen::NO_ENGLISH_STATUS;
use qt_app::{
    App, AppConfig, Clock, Emitter, MonitorMetrics, Placement, ScreenBackend, Window,
    SHUTDOWN_TIMEOUT,
};
use qt_engine::QuestData;
use qt_screen::{
    BackendError, Image, OcrReadiness, OverlayState, PixelLine, PixelRect, Rect, Region,
    WindowHandle, GONE_RUN, MIN_PRESENT,
};
use qt_store::Store;
use qt_tailer::{Fingerprint, Line, TailEvent};
use qt_viewmodel::{RowChange, RowChangeKind, View};
use serde_json::{json, Value};

const OWN: &str = "Corisande_Fell";
/// The pid in the session's headers.
const GAME: u32 = 7120;
const BEIRA: &str = "Beira of the Rotten Pack";
const QUEST: &str = "fixture-beira";
/// The game's client rect: 16:9 at the measured size.
const CLIENT: PixelRect = PixelRect::new(0, 0, 2560, 1440);
/// The overlay at its default place, right of every region it may read.
const OVERLAY_ASIDE: PixelRect = PixelRect::new(1800, 300, 2200, 900);
/// A string no read may let out of the reader.
const SENTINEL: &str = "SENTINEL-7Q3X";

// ---------------------------------------------------------------------------
// Fakes.

#[derive(Default)]
struct Recorder {
    views: Mutex<Vec<View>>,
    row_changes: Mutex<Vec<RowChange>>,
}

impl Recorder {
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

/// The shell's window: every call succeeds and none is of interest here.
struct QuietWindow;

impl Window for QuietWindow {
    fn set_click_through(&self, _: bool) -> Result<(), String> {
        Ok(())
    }
    fn set_scale(&self, _: f64) -> Result<(), String> {
        Ok(())
    }
    fn place(&self, _: Placement) -> Result<(), String> {
        Ok(())
    }
    fn register_hotkey(&self, _: &str) -> Result<(), String> {
        Ok(())
    }
    fn open_url(&self, _: &str) -> Result<(), String> {
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

struct FakeClock(Arc<AtomicU64>);

impl Clock for FakeClock {
    fn now_ms(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}

/// One scripted read of a region.
enum Frame {
    /// What the OCR reads: each line's text and its box as fractions of the
    /// client rect.
    Lines(Vec<(String, Rect)>),
    /// The copy waits here until the test opens the gate.
    Gate,
}

/// What the fake backend does and what it was asked, shared with the test.
struct ScreenLog {
    calls: Mutex<Vec<String>>,
    scripts: Mutex<BTreeMap<Region, VecDeque<Frame>>>,
    no_english: AtomicBool,
    overlay: Mutex<Option<OverlayState>>,
    /// The region of the last copy, which the next OCR call reads.
    pending: Mutex<Option<Region>>,
    /// A [`Frame::Gate`] holds the copy while this is closed.
    gate_open: Mutex<bool>,
    gate_moved: Condvar,
    at_gate: AtomicBool,
    /// How long each OCR call blocks, unless released.
    ocr_hold: Mutex<Option<Duration>>,
    in_ocr: AtomicBool,
    release: AtomicBool,
    /// OCR lines handed out that carried the sentinel, per region.
    sentinel_served: Mutex<BTreeMap<Region, u64>>,
    clock: Arc<AtomicU64>,
}

impl ScreenLog {
    fn new(clock: Arc<AtomicU64>) -> ScreenLog {
        ScreenLog {
            calls: Mutex::default(),
            scripts: Mutex::default(),
            no_english: AtomicBool::new(false),
            overlay: Mutex::new(Some(OverlayState {
                rect: OVERLAY_ASIDE,
                visible: true,
            })),
            pending: Mutex::default(),
            gate_open: Mutex::new(false),
            gate_moved: Condvar::new(),
            at_gate: AtomicBool::new(false),
            ocr_hold: Mutex::default(),
            in_ocr: AtomicBool::new(false),
            release: AtomicBool::new(false),
            sentinel_served: Mutex::default(),
            clock,
        }
    }

    fn record(&self, call: String) {
        self.calls.lock().unwrap().push(call);
    }

    fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }

    /// The copies and OCR calls made so far.
    fn reads(&self) -> Vec<String> {
        self.calls()
            .into_iter()
            .filter(|c| c.starts_with("capture") || c.starts_with("ocr "))
            .collect()
    }

    fn script(&self, region: Region, frames: Vec<Frame>) {
        self.scripts
            .lock()
            .unwrap()
            .entry(region)
            .or_default()
            .extend(frames);
    }

    fn wait_at_gate(&self) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !self.at_gate.load(Ordering::SeqCst) {
            assert!(Instant::now() < deadline, "the reader did not reach the gate");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn open_gate(&self) {
        *self.gate_open.lock().unwrap() = true;
        self.gate_moved.notify_all();
    }

    fn pass_gate(&self) {
        self.at_gate.store(true, Ordering::SeqCst);
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut open = self.gate_open.lock().unwrap();
        while !*open {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                break;
            }
            open = self.gate_moved.wait_timeout(open, left).unwrap().0;
        }
    }
}

struct FakeBackend(Arc<ScreenLog>);

fn region_of(area: PixelRect) -> Region {
    Region::ALL
        .into_iter()
        .find(|r| r.pixels(CLIENT) == area)
        .expect("a copy of one of the regions")
}

impl ScreenBackend for FakeBackend {
    fn game_window(&mut self, pid: u32) -> Result<Option<WindowHandle>, BackendError> {
        self.0.record(format!("game_window {pid}"));
        Ok((pid == GAME).then_some(WindowHandle(1)))
    }

    fn foreground_pid(&mut self) -> Result<Option<u32>, BackendError> {
        self.0.record("foreground_pid".to_string());
        Ok(Some(GAME))
    }

    fn client_rect(&mut self, _: WindowHandle) -> Result<Option<PixelRect>, BackendError> {
        self.0.record("client_rect".to_string());
        Ok(Some(CLIENT))
    }

    fn overlay(&mut self) -> Result<Option<OverlayState>, BackendError> {
        self.0.record("overlay".to_string());
        Ok(*self.0.overlay.lock().unwrap())
    }

    fn capture(&mut self, _: WindowHandle, area: PixelRect) -> Result<Image, BackendError> {
        let region = region_of(area);
        self.0.record(format!("capture {}", region.name()));
        loop {
            let gate = {
                let mut scripts = self.0.scripts.lock().unwrap();
                let queue = scripts.entry(region).or_default();
                match queue.front() {
                    Some(Frame::Gate) => {
                        queue.pop_front();
                        true
                    }
                    _ => false,
                }
            };
            if !gate {
                break;
            }
            self.0.pass_gate();
        }
        *self.0.pending.lock().unwrap() = Some(region);
        self.0
            .clock
            .fetch_add(region.period_ms() as u64, Ordering::SeqCst);
        Ok(Image {
            width: 8,
            height: 8,
            bgra: [90, 90, 90, 255].repeat(64),
        })
    }

    fn ocr_readiness(&mut self) -> Result<OcrReadiness, BackendError> {
        self.0.record("ocr_readiness".to_string());
        Ok(if self.0.no_english.load(Ordering::SeqCst) {
            OcrReadiness::NoEnglish
        } else {
            OcrReadiness::Ready {
                language: "en-US".to_string(),
            }
        })
    }

    fn ocr(&mut self, _: &Image) -> Result<Vec<PixelLine>, BackendError> {
        let region = self.0.pending.lock().unwrap().take().expect("a copy first");
        self.0.record(format!("ocr {}", region.name()));
        let hold = *self.0.ocr_hold.lock().unwrap();
        if let Some(hold) = hold {
            self.0.in_ocr.store(true, Ordering::SeqCst);
            let until = Instant::now() + hold;
            while Instant::now() < until && !self.0.release.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(5));
            }
        }
        let frame = self
            .0
            .scripts
            .lock()
            .unwrap()
            .entry(region)
            .or_default()
            .pop_front();
        let lines = match frame {
            Some(Frame::Lines(lines)) => lines,
            _ => Vec::new(),
        };
        let area = region.pixels(CLIENT);
        let served = lines.iter().filter(|(t, _)| t.contains(SENTINEL)).count() as u64;
        *self
            .0
            .sentinel_served
            .lock()
            .unwrap()
            .entry(region)
            .or_default() += served;
        Ok(lines
            .into_iter()
            .map(|(text, f)| {
                let px = f.to_pixels(CLIENT);
                let local = PixelRect::new(
                    px.left - area.left,
                    px.top - area.top,
                    px.right - area.left,
                    px.bottom - area.top,
                );
                PixelLine::new(text, local)
            })
            .collect())
    }
}

// ---------------------------------------------------------------------------
// Scripts.

/// The pack leader's name where the bar shows it.
fn name_line() -> (String, Rect) {
    (
        "BEIRA OF THE ROTTEN PACK".to_string(),
        Rect::new(0.456, 0.047, 0.544, 0.06),
    )
}

/// `n` reads of the bar with her name.
fn present(n: u32) -> Vec<Frame> {
    (0..n).map(|_| Frame::Lines(vec![name_line()])).collect()
}

// ---------------------------------------------------------------------------
// Data and log lines.

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

/// The fixture with a fight step after the pack leader's line, ended by her
/// bar going in `G1_2`; `remark` adds an invented kill remark for her, and
/// a `kill_remark` signal beside the bar's, so the voiced lines are read.
fn screen_data(remark: bool) -> Arc<QuestData> {
    let path = fixtures().join("quests_min.json");
    let text = fs::read_to_string(&path).expect("the fixture");
    let mut v: Value = serde_json::from_str(&text).expect("JSON");
    assert_eq!(v["quests"][1]["id"], QUEST);
    let mut signals = vec![json!({ "kind": "screen", "what": "bar_gone", "boss": BEIRA, "area": "G1_2" })];
    if remark {
        signals.push(json!({ "kind": "screen", "what": "kill_remark", "boss": BEIRA, "area": "G1_2" }));
        v["remarks"] = json!([{
            "class": "Witch",
            "boss": BEIRA,
            "key": "Fixture kill",
            "text": "Invented words for the pack.",
            "source_url": "https://poe2db.tw/us/Fixture_Witch",
            "fetched": "2026-10-26"
        }]);
    }
    v["quests"][1]["steps"].as_array_mut().unwrap().push(json!({
        "id": "fight",
        "title": "Defeat the pack leader",
        "text": "Fixture step text.",
        "signals": signals
    }));
    Arc::new(QuestData::from_value(&v).unwrap_or_else(|e| panic!("{e}")))
}

fn line(time: &str, message: &str) -> String {
    format!("2026/03/10 {time} 4000000 7e2c0001 [INFO Client {GAME}] {message}")
}

fn area(time: &str, code: &str) -> String {
    format!(
        "2026/03/10 {time} 4000000 7e2c0002 [DEBUG Client {GAME}] Generating level 2 area \"{code}\" with seed 2077351921"
    )
}

/// An own character in `G1_1`: no screen signal can fire there.
fn in_the_riverbank() -> Vec<String> {
    vec![
        "2026/03/10 19:00:00 ***** LOG FILE OPENING *****".to_string(),
        area("19:00:05", "G1_1"),
        line("19:00:20", &format!(": {OWN} has received +20 to maximum Life.")),
        line("19:00:40", &format!(": {OWN} (Witch) is now level 9")),
    ]
}

/// The character in `G1_2` with the pack leader's line heard: the fight
/// step is open.
fn at_the_fight() -> Vec<String> {
    let mut lines = in_the_riverbank();
    lines.push(area("19:04:01", "G1_2"));
    lines.push(line(
        "19:05:30",
        "Beira of the Rotten Pack: Fixture line: the pack gathers.",
    ));
    lines
}

/// Tailer events for `lines`, live.
fn events(lines: &[String]) -> Vec<TailEvent> {
    let fingerprint = Fingerprint::of(b"fixture log");
    let mut offset = 0u64;
    lines
        .iter()
        .map(|text| {
            offset += text.len() as u64 + 1;
            TailEvent::Line(Line {
                text: text.clone(),
                end_offset: offset,
                catch_up: false,
                fingerprint,
            })
        })
        .collect()
}

// ---------------------------------------------------------------------------
// The harness.

struct Harness {
    _dir: tempfile::TempDir,
    root: PathBuf,
    app: Arc<App>,
    views: Arc<Recorder>,
    screen: Arc<ScreenLog>,
    clock: Arc<AtomicU64>,
}

struct Setup {
    /// `screen_reading` in the saved settings.
    on: bool,
    /// Whether the config carries the fake backend.
    backend: bool,
    remark: bool,
}

impl Default for Setup {
    fn default() -> Setup {
        Setup {
            on: true,
            backend: true,
            remark: false,
        }
    }
}

fn harness(setup: Setup, prepare: impl FnOnce(&ScreenLog)) -> Harness {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path().to_path_buf();
    if setup.on {
        Store::open(root.join("store"))
            .expect("store")
            .save_settings(&json!({ "screen_reading": true }))
            .expect("settings written");
    }
    let views = Arc::new(Recorder::default());
    let clock = Arc::new(AtomicU64::new(100_000));
    let screen = Arc::new(ScreenLog::new(Arc::clone(&clock)));
    prepare(&screen);
    let backend: Option<Box<dyn ScreenBackend>> = setup
        .backend
        .then(|| Box::new(FakeBackend(Arc::clone(&screen))) as Box<dyn ScreenBackend>);
    let app = App::new(AppConfig {
        options: Default::default(),
        options_error: None,
        store: Store::open(root.join("store")).map_err(|e| e.to_string()),
        data: Some(screen_data(setup.remark)),
        candidates: Vec::new(),
        poll_interval: Duration::from_millis(20),
        emitter: Box::new(FakeEmitter(Arc::clone(&views))),
        window: Box::new(QuietWindow),
        clock: Box::new(FakeClock(Arc::clone(&clock))),
        screen: backend,
    });
    Harness {
        _dir: dir,
        root,
        app,
        views,
        screen,
        clock,
    }
}

impl Harness {
    fn store_dir(&self) -> PathBuf {
        self.root.join("store")
    }

    fn diag_text(&self) -> String {
        fs::read_to_string(self.store_dir().join("overlay.log")).unwrap_or_default()
    }

    /// Waits up to five seconds for a diagnostic line holding every one of
    /// `parts`, and returns it.
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

    /// The `screen:` lines, without the log's own prefix.
    fn screen_lines(&self) -> Vec<String> {
        self.diag_text()
            .lines()
            .filter_map(|l| l.find("screen: ").map(|i| l[i..].to_string()))
            .collect()
    }

    fn feed(&self, lines: &[String]) {
        self.app.handle_tail_events(events(lines));
    }

    fn row_changes(&self) -> Vec<RowChange> {
        self.views.row_changes.lock().unwrap().clone()
    }

    /// Lets the shell's clock run on by `periods` reads of the bar, for a
    /// reader that copies nothing (only the fake's copies move it).
    fn time_passes(&self, periods: u32) {
        for _ in 0..periods {
            self.clock
                .fetch_add(Region::BossBar.period_ms() as u64, Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(30));
        }
    }
}

fn wait_until(what: &str, pred: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !pred() {
        assert!(Instant::now() < deadline, "{what}: not within 5 s");
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Every file under `dir`, read as text.
fn files_under(dir: &Path) -> Vec<(PathBuf, String)> {
    let mut out = Vec::new();
    for entry in fs::read_dir(dir).expect("a folder").flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(files_under(&path));
        } else {
            let bytes = fs::read(&path).expect("a file");
            out.push((path, String::from_utf8_lossy(&bytes).into_owned()));
        }
    }
    out
}

/// Whether `line` is `bar of <boss> gone (last seen <hh:mm:ss.mmm>Z): ...`.
fn bar_line_shape(line: &str) -> bool {
    let Some(rest) = line.split("(last seen ").nth(1) else {
        return false;
    };
    let stamp: Vec<char> = rest.chars().take(13).collect();
    stamp.len() == 13
        && stamp.iter().enumerate().all(|(i, c)| match i {
            2 | 5 => *c == ':',
            8 => *c == '.',
            12 => *c == 'Z',
            _ => c.is_ascii_digit(),
        })
        && rest[13..].starts_with("): ")
}

// ---------------------------------------------------------------------------
// Tests.

#[test]
fn screen_reading_off_starts_no_reader_baseline() {
    let h = harness(Setup { on: false, ..Setup::default() }, |_| {});
    h.app.start();
    h.feed(&at_the_fight());
    h.app.tick();
    std::thread::sleep(Duration::from_millis(300));
    assert!(!h.app.screen_reader_running());
    assert!(h.screen.calls().is_empty(), "{:?}", h.screen.calls());
    let view = h.app.get_view();
    assert!(!view.settings.screen_reading);
    assert!(h.screen_lines().is_empty(), "{:?}", h.screen_lines());

    // The view is the one an app with no backend at all shows.
    let bare = harness(
        Setup {
            on: false,
            backend: false,
            ..Setup::default()
        },
        |_| {},
    );
    bare.app.start();
    bare.feed(&at_the_fight());
    assert_eq!(bare.app.get_view(), view);
    let json = serde_json::to_value(&view).unwrap();
    assert_eq!(json["settings"]["screen_reading"], false);

    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));
    assert!(bare.app.shutdown(SHUTDOWN_TIMEOUT));
    assert!(h.diag_text().contains("stop: tailer and worker ended"));
}

#[test]
fn screen_reading_on_ticks_beira_from_the_bar_target() {
    let h = harness(Setup::default(), |s| s.script(Region::BossBar, present(3)));
    h.app.start();
    assert!(h.app.get_view().settings.screen_reading);
    h.wait_diag(&["screen: on (OCR en-US)"]);
    h.feed(&at_the_fight());

    let line = h.wait_diag(&["screen: bar of Beira of the Rotten Pack gone"]);
    assert!(
        line.ends_with(&format!("): ticked {QUEST}/fight")),
        "{line}"
    );
    assert!(bar_line_shape(&line), "{line}");
    let lines = h.screen_lines();
    assert!(
        lines.contains(&format!("screen: reading boss bar for {QUEST}/fight")),
        "{lines:?}"
    );

    // Its row change, live, naming the step it finished.
    wait_until("the row change", || {
        h.row_changes()
            .iter()
            .any(|c| c.quest_id == QUEST && c.step_done.as_deref() == Some("Defeat the pack leader"))
    });
    let change = h
        .row_changes()
        .into_iter()
        .find(|c| c.quest_id == QUEST && c.step_done.is_some())
        .unwrap();
    assert_ne!(change.kind, RowChangeKind::Available);
    h.wait_diag(&[&format!("row change {} ", change.id), QUEST]);

    // The step is done: nothing more to read, so the reader idles.
    wait_until("idle after the tick", || {
        h.screen_lines().last().map(String::as_str) == Some("screen: idle")
    });
    // It read at least the bar's minimum and run, and saved the tick.
    let bars = h
        .screen
        .reads()
        .iter()
        .filter(|c| *c == "ocr boss bar")
        .count();
    assert!(bars >= (MIN_PRESENT + GONE_RUN) as usize, "{bars}");
    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));
    assert!(h
        .diag_text()
        .contains("stop: tailer, worker and screen reader ended, progress saved"));
    let stored = h
        .app
        .store()
        .unwrap()
        .load_character(OWN)
        .unwrap()
        .value
        .expect("the character's file");
    assert!(
        stored.to_string().contains("step/fight/0"),
        "the screen signal is kept as evidence: {stored}"
    );
}

#[test]
fn screen_bar_gone_after_a_death_is_not_counted() {
    let h = harness(Setup::default(), |s| {
        let mut frames = present(3);
        frames.push(Frame::Gate);
        s.script(Region::BossBar, frames);
    });
    h.app.start();
    h.feed(&at_the_fight());
    // The bar was seen; before its run of absent reads, the character dies.
    h.screen.wait_at_gate();
    h.feed(&[line("19:06:00", &format!(": {OWN} has been slain."))]);
    h.screen.open_gate();

    let line = h.wait_diag(&["screen: bar of Beira of the Rotten Pack gone"]);
    assert!(line.ends_with("): not counted (after a death)"), "{line}");
    assert!(!h.diag_text().contains(&format!("ticked {QUEST}/fight")));
    assert!(h
        .row_changes()
        .iter()
        .all(|c| c.step_done.as_deref() != Some("Defeat the pack leader")));
    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));
}

#[test]
fn screen_bar_gone_after_an_area_change_is_not_counted() {
    let h = harness(Setup::default(), |s| {
        let mut frames = present(3);
        frames.push(Frame::Gate);
        s.script(Region::BossBar, frames);
    });
    h.app.start();
    h.feed(&at_the_fight());
    h.screen.wait_at_gate();
    // Back into the same area (a new instance): the fight step stays open,
    // so only the cancel keeps the bar's going from counting.
    h.feed(&[area("19:06:00", "G1_2")]);
    h.screen.open_gate();
    let line = h.wait_diag(&["screen: bar of Beira of the Rotten Pack gone"]);
    assert!(line.ends_with("): not counted (after an area change)"), "{line}");
    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));
}

#[test]
fn screen_reader_stops_within_the_shutdown_deadline() {
    // An OCR call that ends before the deadline: everything ends.
    let h = harness(Setup::default(), |s| {
        *s.ocr_hold.lock().unwrap() = Some(Duration::from_millis(300));
    });
    h.app.start();
    h.feed(&at_the_fight());
    wait_until("an OCR call in flight", || h.screen.in_ocr.load(Ordering::SeqCst));
    let started = Instant::now();
    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));
    assert!(started.elapsed() < SHUTDOWN_TIMEOUT);
    assert!(!h.app.screen_reader_running());
    assert!(h
        .diag_text()
        .contains("stop: tailer, worker and screen reader ended, progress saved"));

    // One that outlasts it: shutdown still returns by the deadline, and
    // says the reader did not end.
    let h = harness(Setup::default(), |s| {
        *s.ocr_hold.lock().unwrap() = Some(Duration::from_secs(10));
    });
    h.app.start();
    h.feed(&at_the_fight());
    wait_until("an OCR call in flight", || h.screen.in_ocr.load(Ordering::SeqCst));
    let deadline = Duration::from_millis(300);
    let started = Instant::now();
    assert!(!h.app.shutdown(deadline));
    assert!(started.elapsed() < deadline + Duration::from_millis(700));
    let stop = h.wait_diag(&["stop: "]);
    assert!(stop.contains("screen reader did not end within 300 ms"), "{stop}");
    // Let the call end, so the thread does too.
    h.screen.release.store(true, Ordering::SeqCst);
}

#[test]
fn screen_text_never_reaches_disk_view_or_log() {
    let noise = || (format!("{SENTINEL} band"), Rect::new(0.26, 0.10, 0.35, 0.12));
    let h = harness(
        Setup {
            remark: true,
            ..Setup::default()
        },
        |s| {
            let mut bar: Vec<Frame> = (0..3)
                .map(|_| Frame::Lines(vec![name_line(), noise()]))
                .collect();
            bar.extend((0..GONE_RUN + 2).map(|_| Frame::Lines(vec![noise()])));
            s.script(Region::BossBar, bar);
            let voiced = (0..30)
                .map(|i| {
                    Frame::Lines(vec![(
                        format!("{OWN}: {SENTINEL} spoken line {i}"),
                        Rect::new(0.03, 0.71, 0.30, 0.73),
                    )])
                })
                .collect();
            s.script(Region::VoicedLines, voiced);
        },
    );
    h.app.start();
    h.feed(&at_the_fight());
    h.wait_diag(&[&format!("ticked {QUEST}/fight")]);
    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));

    // Positive control: the OCR did hand the sentinel to the reader, from
    // both regions.
    let served = h.screen.sentinel_served.lock().unwrap().clone();
    assert!(served.get(&Region::BossBar).copied().unwrap_or(0) > 0, "{served:?}");
    assert!(served.get(&Region::VoicedLines).copied().unwrap_or(0) > 0, "{served:?}");
    assert!(h
        .screen_lines()
        .iter()
        .any(|l| l.starts_with("screen: reading boss bar and voiced lines for")));

    for (path, text) in files_under(&h.store_dir()) {
        assert!(!text.contains(SENTINEL), "{} holds read text", path.display());
    }
    assert!(!h.diag_text().contains(SENTINEL));
    for view in h.views.views.lock().unwrap().iter() {
        let json = serde_json::to_string(view).unwrap();
        assert!(!json.contains(SENTINEL), "a view holds read text");
    }
    let changes = serde_json::to_string(&h.row_changes()).unwrap();
    assert!(!changes.contains(SENTINEL));
    assert!(!h.row_changes().is_empty());
}

#[test]
fn screen_overlay_over_a_region_sets_the_status_line() {
    let over_the_bar = OverlayState {
        rect: PixelRect::new(1000, 0, 1400, 150),
        visible: true,
    };
    let h = harness(Setup::default(), |s| {
        *s.overlay.lock().unwrap() = Some(over_the_bar);
    });
    h.app.start();
    h.feed(&at_the_fight());
    let want = "The overlay covers the boss bar; move it to let screen reading see it.";
    h.views.wait_for("with the overlay's status line", |v| {
        v.status.as_ref().is_some_and(|s| s.text == want)
    });
    h.wait_diag(&["screen: covered: boss bar by the overlay"]);
    // Covered reads copy nothing while it lasts.
    h.time_passes(6);
    assert!(h.screen.reads().is_empty(), "{:?}", h.screen.reads());
    assert!(!h.diag_text().contains("covered: dialogue window"));

    // Moved aside: the region is read again and the line clears.
    *h.screen.overlay.lock().unwrap() = Some(OverlayState {
        rect: OVERLAY_ASIDE,
        visible: true,
    });
    h.wait_diag(&["screen: uncovered: boss bar"]);
    h.views.wait_for("without it", |v| {
        v.status.as_ref().is_none_or(|s| s.text != want)
    });
    h.time_passes(2);
    wait_until("a copy of the bar", || {
        h.screen.reads().contains(&"capture boss bar".to_string())
    });

    // Negative control: a hidden overlay covers nothing.
    *h.screen.overlay.lock().unwrap() = Some(OverlayState {
        rect: over_the_bar.rect,
        visible: false,
    });
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(
        h.diag_text().matches("screen: covered: boss bar").count(),
        1
    );
    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));
}

#[test]
fn screen_no_english_ocr_is_refused_with_a_status_line() {
    let h = harness(Setup::default(), |s| {
        s.no_english.store(true, Ordering::SeqCst);
    });
    h.app.start();
    h.views.wait_for("with the refusal", |v| {
        v.status.as_ref().is_some_and(|s| s.text == NO_ENGLISH_STATUS)
    });
    h.wait_diag(&["screen: refused: no English OCR"]);
    wait_until("the reader ended", || !h.app.screen_reader_running());
    h.feed(&at_the_fight());
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(h.screen.calls(), vec!["ocr_readiness"]);
    assert!(!h.diag_text().contains("screen: on"));

    // Off and on again asks again (and is refused again).
    h.app.update_settings(&json!({ "screen_reading": false })).unwrap();
    assert!(h.app.get_view().status.is_none());
    h.app.update_settings(&json!({ "screen_reading": true })).unwrap();
    wait_until("asked twice", || {
        h.screen.calls() == vec!["ocr_readiness", "ocr_readiness"]
    });
    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));
}

#[test]
fn screen_reading_turns_on_and_off_at_run_time() {
    let h = harness(Setup { on: false, ..Setup::default() }, |_| {});
    h.app.start();
    h.feed(&at_the_fight());
    assert!(h.screen.calls().is_empty());

    let view = h
        .app
        .update_settings(&json!({ "screen_reading": true }))
        .unwrap();
    assert!(view.settings.screen_reading);
    h.wait_diag(&["screen: on (OCR en-US)"]);
    wait_until("a copy of the bar", || {
        h.screen.reads().contains(&"capture boss bar".to_string())
    });
    assert!(h.app.screen_reader_running());

    let view = h
        .app
        .update_settings(&json!({ "screen_reading": false }))
        .unwrap();
    assert!(!view.settings.screen_reading);
    assert!(!h.app.screen_reader_running());
    assert_eq!(h.screen_lines().last().map(String::as_str), Some("screen: off"));
    let calls = h.screen.calls().len();
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(h.screen.calls().len(), calls, "no call after the stop");
    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));
    let stored = h.app.store().unwrap().load_settings().unwrap().value.unwrap();
    assert_eq!(stored["screen_reading"], false);
}

#[test]
fn screen_empty_demand_captures_nothing() {
    let h = harness(Setup::default(), |_| {});
    h.app.start();
    // An own character in an area no screen signal names.
    h.feed(&in_the_riverbank());
    h.wait_diag(&["screen: idle"]);
    wait_until("the overlap check", || {
        h.screen.calls().contains(&"overlay".to_string())
    });
    std::thread::sleep(Duration::from_millis(400));
    assert!(h.screen.reads().is_empty(), "{:?}", h.screen.reads());
    assert!(!h.diag_text().contains("screen: reading"));
    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));
}
