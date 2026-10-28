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
//!
//! The dialogue window's tests run over the shipped data instead, in
//! Clearfell Encampment, where Renly's QUEST COMPLETE banner ticks "Take
//! Renly's reward"; the fake tells a copy of the window's heading strip from
//! a copy of the whole window by its area.
//!
//! The Tree of Souls test runs over the shipped data too, in the Grelwood,
//! where the character's remark on finding the tree ticks "Search the
//! Grelwood" from the voiced lines.

use std::collections::{BTreeMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use qt_app::screen::{CAPTURE_FAILURE_LIMIT, NO_ENGLISH_STATUS};
use qt_app::{
    App, AppConfig, Clock, Emitter, MonitorMetrics, Placement, ScreenBackend, Window,
    SHUTDOWN_TIMEOUT,
};
use qt_engine::QuestData;
use qt_screen::{
    BackendError, Image, OcrReadiness, OverlayState, PixelLine, PixelRect, Rect, Region,
    WindowHandle, BANNER_AREA, GONE_RUN, HEADING_STRIP, MIN_PRESENT,
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

/// What a copy was of: one of the regions, or the dialogue window's heading
/// strip ([`HEADING_STRIP`]), which the reader copies alone while the window's
/// gate is shut. Each has a script of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Area {
    Region(Region),
    Strip,
}

impl From<Region> for Area {
    fn from(region: Region) -> Area {
        Area::Region(region)
    }
}

impl Area {
    /// The name its calls are recorded under.
    fn name(self) -> &'static str {
        match self {
            Area::Region(r) => r.name(),
            Area::Strip => "heading strip",
        }
    }

    /// Where it sits in the client rect's pixels.
    fn pixels(self) -> PixelRect {
        match self {
            Area::Region(r) => r.pixels(CLIENT),
            Area::Strip => HEADING_STRIP.to_pixels(CLIENT),
        }
    }

    /// How far a copy of it moves the shell's clock: its region's period.
    fn period_ms(self) -> u64 {
        match self {
            Area::Region(r) => r.period_ms() as u64,
            Area::Strip => Region::DialogueWindow.period_ms() as u64,
        }
    }
}

/// One scripted read of a region.
enum Frame {
    /// What the OCR reads: each line's text and its box as fractions of the
    /// client rect.
    Lines(Vec<(String, Rect)>),
    /// The copy waits here until the test opens the gate.
    Gate,
    /// The copy comes back black (`Image::is_black`), and is not read.
    Black,
}

/// What the fake backend does and what it was asked, shared with the test.
struct ScreenLog {
    calls: Mutex<Vec<String>>,
    scripts: Mutex<BTreeMap<Area, VecDeque<Frame>>>,
    no_english: AtomicBool,
    overlay: Mutex<Option<OverlayState>>,
    /// The game's window is not found by its pid.
    no_window: AtomicBool,
    /// The game's window is minimized: it has no client rect.
    minimized: AtomicBool,
    /// The game's window is in front.
    in_front: AtomicBool,
    /// The area of the last copy, which the next OCR call reads.
    pending: Mutex<Option<Area>>,
    /// A [`Frame::Gate`] holds the copy while this is closed.
    gate_open: Mutex<bool>,
    gate_moved: Condvar,
    at_gate: AtomicBool,
    /// How long each OCR call blocks, unless released.
    ocr_hold: Mutex<Option<Duration>>,
    in_ocr: AtomicBool,
    release: AtomicBool,
    /// OCR lines handed out that carried the sentinel, per area.
    sentinel_served: Mutex<BTreeMap<Area, u64>>,
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
            no_window: AtomicBool::new(false),
            minimized: AtomicBool::new(false),
            in_front: AtomicBool::new(true),
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

    fn script(&self, area: impl Into<Area>, frames: Vec<Frame>) {
        self.scripts
            .lock()
            .unwrap()
            .entry(area.into())
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

fn area_of(area: PixelRect) -> Area {
    Region::ALL
        .into_iter()
        .map(Area::Region)
        .chain([Area::Strip])
        .find(|a| a.pixels() == area)
        .expect("a copy of one of the regions or of the heading strip")
}

impl ScreenBackend for FakeBackend {
    fn game_window(&mut self, pid: u32) -> Result<Option<WindowHandle>, BackendError> {
        self.0.record(format!("game_window {pid}"));
        let found = pid == GAME && !self.0.no_window.load(Ordering::SeqCst);
        Ok(found.then_some(WindowHandle(1)))
    }

    fn foreground_pid(&mut self) -> Result<Option<u32>, BackendError> {
        self.0.record("foreground_pid".to_string());
        Ok(self.0.in_front.load(Ordering::SeqCst).then_some(GAME))
    }

    fn client_rect(&mut self, _: WindowHandle) -> Result<Option<PixelRect>, BackendError> {
        self.0.record("client_rect".to_string());
        Ok((!self.0.minimized.load(Ordering::SeqCst)).then_some(CLIENT))
    }

    fn overlay(&mut self) -> Result<Option<OverlayState>, BackendError> {
        self.0.record("overlay".to_string());
        Ok(*self.0.overlay.lock().unwrap())
    }

    fn capture(&mut self, _: WindowHandle, area: PixelRect) -> Result<Image, BackendError> {
        let region = area_of(area);
        self.0.record(format!("capture {}", region.name()));
        let black = loop {
            let front = {
                let mut scripts = self.0.scripts.lock().unwrap();
                let queue = scripts.entry(region).or_default();
                match queue.front() {
                    Some(Frame::Gate) => {
                        queue.pop_front();
                        Some(Frame::Gate)
                    }
                    // A black copy is not read, so its frame goes here.
                    Some(Frame::Black) => {
                        queue.pop_front();
                        Some(Frame::Black)
                    }
                    _ => None,
                }
            };
            match front {
                Some(Frame::Gate) => self.0.pass_gate(),
                Some(Frame::Black) => break true,
                _ => break false,
            }
        };
        *self.0.pending.lock().unwrap() = Some(region);
        self.0
            .clock
            .fetch_add(region.period_ms(), Ordering::SeqCst);
        let pixel = if black { [0, 0, 0, 255] } else { [90, 90, 90, 255] };
        let image = Image {
            width: 8,
            height: 8,
            bgra: pixel.repeat(64),
        };
        assert_eq!(image.is_black(), black, "the fake's copy is what it means");
        Ok(image)
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
        let area = region.pixels();
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

/// `n` reads of the band with no bar.
fn absent(n: u32) -> Vec<Frame> {
    (0..n).map(|_| Frame::Lines(Vec::new())).collect()
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

/// The shipped data, as the app builds it in: Reaching Clearfell's "Take
/// Renly's reward" is ended by the QUEST COMPLETE banner in his window in
/// `G1_town`.
fn shipped_data() -> Arc<QuestData> {
    Arc::new(QuestData::parse(qt_app::EMBEDDED_DATA).unwrap_or_else(|e| panic!("{e}")))
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

/// The lines [`at_the_fight`] adds to [`in_the_riverbank`].
fn into_the_fight() -> Vec<String> {
    at_the_fight()[in_the_riverbank().len()..].to_vec()
}

/// The character's death.
fn slain() -> String {
    line("19:06:00", &format!(": {OWN} has been slain."))
}

/// The character in Clearfell Encampment after The Riverbank, over the
/// shipped data: Reaching Clearfell is in progress, and its "Take Renly's
/// reward" is open there.
fn in_the_encampment() -> Vec<String> {
    let mut lines = in_the_riverbank();
    lines.push(area("19:03:00", "G1_town"));
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
    /// The shipped quest data instead of the fixture.
    shipped: bool,
}

impl Default for Setup {
    fn default() -> Setup {
        Setup {
            on: true,
            backend: true,
            remark: false,
            shipped: false,
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
        data: Some(if setup.shipped {
            shipped_data()
        } else {
            screen_data(setup.remark)
        }),
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
        h.screen_lines().last().map(String::as_str) == Some("screen: idle (no demand)")
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
    // The bar was seen, then absent; the character's death arrives while the
    // reader is inside the read that completes the absent run. That going is
    // reported before the reader can see the death in its demand, so the
    // worker's check on the bar's last present read is what drops it.
    let h = harness(Setup::default(), |s| {
        let mut frames = present(3);
        frames.extend(absent(GONE_RUN - 1));
        frames.push(Frame::Gate);
        s.script(Region::BossBar, frames);
    });
    h.app.start();
    h.feed(&at_the_fight());
    h.screen.wait_at_gate();
    h.feed(&[slain()]);
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

/// The `bar of Beira ... gone` lines so far.
fn bar_lines(h: &Harness) -> Vec<String> {
    h.screen_lines()
        .into_iter()
        .filter(|l| l.starts_with(&format!("screen: bar of {BEIRA} gone")))
        .collect()
}

#[test]
fn screen_bar_up_through_a_death_then_gone_is_not_counted() {
    // The bar is up; the character dies while it still reads present, and it
    // stays up through the death screen (two reads and more), then goes on
    // the walk back. Its last present read is after the death, so only the
    // reader's mark keeps that going from ticking the fight step.
    let h = harness(Setup::default(), |s| {
        let mut frames = present(3);
        frames.push(Frame::Gate);
        frames.extend(present(3));
        s.script(Region::BossBar, frames);
    });
    h.app.start();
    h.feed(&at_the_fight());
    h.screen.wait_at_gate();
    h.feed(&[slain()]);
    h.screen.open_gate();

    let line = h.wait_diag(&["screen: bar of Beira of the Rotten Pack gone"]);
    assert!(bar_line_shape(&line), "{line}");
    assert!(line.ends_with("): not counted (after a death)"), "{line}");
    // The reads after it are absent: nothing more goes, nothing ticks.
    h.time_passes(2 * GONE_RUN);
    assert_eq!(bar_lines(&h).len(), 1, "{:?}", bar_lines(&h));
    assert!(!h.diag_text().contains(&format!("ticked {QUEST}/fight")));
    assert!(h
        .row_changes()
        .iter()
        .all(|c| c.step_done.as_deref() != Some("Defeat the pack leader")));
    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));
}

#[test]
fn screen_bar_seen_again_after_a_death_then_gone_ticks() {
    // As above, and then the bar shows again and goes: that going counts.
    let h = harness(Setup::default(), |s| {
        let mut frames = present(3);
        frames.push(Frame::Gate);
        frames.extend(present(3));
        frames.extend(absent(GONE_RUN));
        frames.extend(present(MIN_PRESENT + 1));
        s.script(Region::BossBar, frames);
    });
    h.app.start();
    h.feed(&at_the_fight());
    h.screen.wait_at_gate();
    h.feed(&[slain()]);
    h.screen.open_gate();

    h.wait_diag(&[&format!("ticked {QUEST}/fight")]);
    let lines = bar_lines(&h);
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert!(lines[0].ends_with("): not counted (after a death)"), "{lines:?}");
    assert!(lines[1].ends_with(&format!("): ticked {QUEST}/fight")), "{lines:?}");
    wait_until("the row change", || {
        h.row_changes()
            .iter()
            .any(|c| c.step_done.as_deref() == Some("Defeat the pack leader"))
    });
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
    for region in [Region::BossBar, Region::VoicedLines] {
        let n = served.get(&Area::Region(region)).copied().unwrap_or(0);
        assert!(n > 0, "{served:?}");
    }
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
    h.wait_diag(&["screen: idle (no demand)"]);
    wait_until("the overlap check", || {
        h.screen.calls().contains(&"overlay".to_string())
    });
    std::thread::sleep(Duration::from_millis(400));
    assert!(h.screen.reads().is_empty(), "{:?}", h.screen.reads());
    assert!(!h.diag_text().contains("screen: reading"));
    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));
}

/// The `screen: idle ...` lines so far.
fn idle_lines(h: &Harness) -> Vec<String> {
    h.screen_lines()
        .into_iter()
        .filter(|l| l.starts_with("screen: idle"))
        .collect()
}

#[test]
fn screen_idle_line_names_its_reason() {
    let h = harness(Setup::default(), |_| {});
    h.app.start();
    // No live line yet: the game's pid is not confirmed.
    h.wait_diag(&["screen: idle (pid unconfirmed)"]);
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(idle_lines(&h).len(), 1, "one line while it holds");

    // A character where no screen signal can fire.
    h.feed(&in_the_riverbank());
    h.wait_diag(&["screen: idle (no demand)"]);
    std::thread::sleep(Duration::from_millis(300));

    // The game's window gone; then minimized (set first, so no tick between
    // the two sees a third reason).
    h.screen.no_window.store(true, Ordering::SeqCst);
    h.wait_diag(&["screen: idle (no game window)"]);
    std::thread::sleep(Duration::from_millis(300));
    h.screen.minimized.store(true, Ordering::SeqCst);
    h.screen.no_window.store(false, Ordering::SeqCst);
    h.wait_diag(&["screen: idle (minimized)"]);
    std::thread::sleep(Duration::from_millis(300));

    // The fight opens while it is minimized and behind another window; then
    // it is restored, still behind.
    h.screen.in_front.store(false, Ordering::SeqCst);
    h.feed(&into_the_fight());
    std::thread::sleep(Duration::from_millis(300));
    h.screen.minimized.store(false, Ordering::SeqCst);
    h.wait_diag(&["screen: idle (not in front)"]);
    std::thread::sleep(Duration::from_millis(300));
    assert!(h.screen.reads().is_empty(), "{:?}", h.screen.reads());

    // In front: it reads.
    h.screen.in_front.store(true, Ordering::SeqCst);
    h.wait_diag(&[&format!("screen: reading boss bar for {QUEST}/fight")]);

    // One line per reason, in the order they held, and never a bare `idle`.
    assert_eq!(
        idle_lines(&h),
        [
            "screen: idle (pid unconfirmed)",
            "screen: idle (no demand)",
            "screen: idle (no game window)",
            "screen: idle (minimized)",
            "screen: idle (not in front)",
        ]
    );
    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));
}

#[test]
fn screen_without_an_overlay_window_says_the_overlay_check_is_off() {
    const OFF: &str = "screen: overlay check off (no overlay window)";
    const ON: &str = "screen: overlay check on";
    let count = |h: &Harness, line: &str| h.screen_lines().iter().filter(|l| *l == line).count();
    let h = harness(Setup::default(), |s| {
        *s.overlay.lock().unwrap() = None;
    });
    h.app.start();
    h.feed(&at_the_fight());
    h.wait_diag(&[OFF]);
    // However many ticks pass, the line is written once.
    let checks = |h: &Harness| h.screen.calls().iter().filter(|c| *c == "overlay").count();
    let before = checks(&h);
    wait_until("more overlap checks", || checks(&h) >= before + 5);
    assert_eq!(count(&h, OFF), 1, "{:?}", h.screen_lines());
    assert_eq!(count(&h, ON), 0);

    // The overlay's window found again.
    *h.screen.overlay.lock().unwrap() = Some(OverlayState {
        rect: OVERLAY_ASIDE,
        visible: true,
    });
    h.wait_diag(&[ON]);
    let before = checks(&h);
    wait_until("more overlap checks", || checks(&h) >= before + 5);
    assert_eq!(count(&h, ON), 1, "{:?}", h.screen_lines());
    assert_eq!(count(&h, OFF), 1);

    // Lost again: the off line again.
    *h.screen.overlay.lock().unwrap() = None;
    wait_until("the second off line", || count(&h, OFF) == 2);
    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));
}

#[test]
fn screen_without_an_overlay_window_pauses_reading_with_a_status_line() {
    const PAUSED: &str = "Screen reading is paused: the overlay cannot find its own window.";
    let paused = |v: &View| v.status.as_ref().is_some_and(|s| s.text == PAUSED);
    let h = harness(Setup::default(), |s| {
        *s.overlay.lock().unwrap() = None;
    });
    h.app.start();
    h.feed(&at_the_fight());
    h.views.wait_for("with the pause's status line", paused);
    h.wait_diag(&["screen: overlay check off (no overlay window)"]);
    // The bar falls due again and again; nothing is copied or read, since
    // a copy would hold the overlay's own rows.
    h.time_passes(6);
    assert!(h.screen.reads().is_empty(), "{:?}", h.screen.reads());
    assert!(paused(&h.app.get_view()));

    // The overlay's window found again: the line clears and the bar is read.
    *h.screen.overlay.lock().unwrap() = Some(OverlayState {
        rect: OVERLAY_ASIDE,
        visible: true,
    });
    h.wait_diag(&["screen: overlay check on"]);
    h.views.wait_for("without it", |v| !paused(v));
    h.time_passes(2);
    wait_until("a copy of the bar", || {
        h.screen.reads().contains(&"capture boss bar".to_string())
    });
    assert!(!paused(&h.app.get_view()));
    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));
}

#[test]
fn screen_black_copies_are_not_reported_as_failed_captures() {
    // A dark band for longer than the limit, then the bar's band again.
    let limit = CAPTURE_FAILURE_LIMIT;
    let black_copies = limit + 30;
    let h = harness(Setup::default(), |s| {
        s.script(
            Region::BossBar,
            (0..black_copies).map(|_| Frame::Black).collect(),
        );
    });
    h.app.start();
    h.feed(&at_the_fight());
    let black = format!(
        "Screen reading sees the game's window copied black ({limit} times in a row); a dark scene or a loading screen looks the same."
    );
    h.views.wait_for("with the black-copy status", |v| {
        v.status.as_ref().is_some_and(|s| s.text == black)
    });
    h.wait_diag(&[&format!("screen: black: {limit} copies in a row")]);
    // A good copy ends the run and clears it.
    wait_until("the status cleared after a good copy", || {
        let last = h.views.views.lock().unwrap().last().cloned();
        h.screen.reads().contains(&"ocr boss bar".to_string())
            && last.is_some_and(|v| v.status.as_ref().is_none_or(|s| s.text != black))
    });
    // The minute's line counts them.
    let minute = h.wait_diag(&["screen: minute: ", " frames, "]);
    assert!(
        minute.contains(&format!(", {black_copies} black copies, 0 OCR failures, busy ")),
        "{minute}"
    );

    // Not a failed capture: no refusal, no capture-failure status.
    let text = h.diag_text();
    assert!(!text.contains("refused: the capture failed"), "{text}");
    assert_eq!(text.matches("screen: black: ").count(), 1, "{text}");
    for view in h.views.views.lock().unwrap().iter() {
        let status = view.status.as_ref().map(|s| s.text.as_str()).unwrap_or("");
        assert!(!status.contains("cannot copy"), "{status}");
    }
    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));
}

// ---------------------------------------------------------------------------
// The dialogue window's heading gate.

const RENLY: &str = "Renly";
const RENLY_STEP: &str = "reaching-clearfell/step-6";

/// Renly's heading where the kept captures read it, inside the strip.
fn renly_heading() -> (String, Rect) {
    (RENLY.to_string(), Rect::new(0.536, 0.265, 0.561, 0.277))
}

/// The banner in its place.
fn quest_complete() -> (String, Rect) {
    let banner = ("QUEST COMPLETE".to_string(), Rect::new(0.447, 0.776, 0.553, 0.800));
    assert!(BANNER_AREA.contains(&banner.1));
    banner
}

/// A body line of the window, below the strip.
fn window_text() -> (String, Rect) {
    ("Some words of thanks.".to_string(), Rect::new(0.42, 0.51, 0.66, 0.53))
}

#[test]
fn screen_dialogue_without_a_heading_reads_only_the_strip_baseline() {
    let h = harness(
        Setup {
            shipped: true,
            ..Setup::default()
        },
        |s| {
            // The strip shows the town's own text, never Renly's heading; the
            // whole window would show a body line and a banner, no heading.
            let strip = (0..90)
                .map(|_| {
                    Frame::Lines(vec![(
                        "Clearfell Encampment".to_string(),
                        Rect::new(0.40, 0.25, 0.60, 0.28),
                    )])
                })
                .collect();
            s.script(Area::Strip, strip);
            let window = (0..90)
                .map(|_| Frame::Lines(vec![window_text(), quest_complete()]))
                .collect();
            s.script(Region::DialogueWindow, window);
        },
    );
    h.app.start();
    h.feed(&in_the_encampment());
    h.wait_diag(&[&format!("screen: reading dialogue window for {RENLY_STEP}")]);

    // A minute of reads: every one of them the strip alone.
    let gate = h.wait_diag(&["screen: minute: dialogue window gate, "]);
    let skipped = h.screen.reads().iter().filter(|c| *c == "capture heading strip").count();
    assert!(skipped > 0);
    assert!(gate.ends_with("screen: minute: dialogue window gate, 0 full reads, 60 skipped"), "{gate}");
    // A strip read counts as a read of the window.
    h.wait_diag(&["screen: minute: dialogue window, Renly's heading in 0 of 60 reads, 0 banners in place"]);
    let reads = h.screen.reads();
    assert!(reads.contains(&"ocr heading strip".to_string()), "{reads:?}");
    assert!(!reads.iter().any(|c| c.ends_with(" dialogue window")), "{reads:?}");

    assert!(!h.diag_text().contains("screen: quest complete in"));
    assert!(h
        .row_changes()
        .iter()
        .all(|c| c.step_done.as_deref() != Some("Take Renly's reward")));
    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));
}

#[test]
fn screen_dialogue_heading_in_the_strip_opens_the_full_read_and_ticks_target() {
    let h = harness(
        Setup {
            shipped: true,
            ..Setup::default()
        },
        |s| {
            s.script(
                Area::Strip,
                vec![
                    Frame::Lines(vec![]),
                    Frame::Lines(vec![]),
                    Frame::Lines(vec![renly_heading()]),
                ],
            );
            s.script(
                Region::DialogueWindow,
                vec![
                    Frame::Lines(vec![renly_heading(), window_text()]),
                    Frame::Lines(vec![renly_heading(), quest_complete()]),
                ],
            );
        },
    );
    h.app.start();
    h.feed(&in_the_encampment());

    let line = h.wait_diag(&["screen: quest complete in Renly's window ("]);
    assert!(line.ends_with(&format!("): ticked {RENLY_STEP}")), "{line}");
    wait_until("the row change", || {
        h.row_changes()
            .iter()
            .any(|c| c.step_done.as_deref() == Some("Take Renly's reward"))
    });

    // Three strip reads, the third with the heading; the whole window is
    // copied right after it, in the same tick (no new look for the window).
    let calls = h.screen.calls();
    let first_full = calls
        .iter()
        .position(|c| c == "capture dialogue window")
        .unwrap_or_else(|| panic!("no copy of the whole window: {calls:?}"));
    let before = &calls[..first_full];
    assert_eq!(
        before.iter().filter(|c| *c == "ocr heading strip").count(),
        3,
        "{calls:?}"
    );
    let last_strip = before.iter().rposition(|c| c == "ocr heading strip").unwrap();
    assert_eq!(last_strip + 1, first_full, "{calls:?}");
    // At least two whole-window reads: the one the heading opened, then the
    // banner's (the gate stays open until the tick empties the demand).
    let reads = h.screen.reads();
    assert!(
        reads.iter().filter(|c| *c == "ocr dialogue window").count() >= 2,
        "{reads:?}"
    );
    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));
}

// ---------------------------------------------------------------------------
// The Tree of Souls remark (2026-10-28).

const TREE_STEP: &str = "secrets-in-the-dark/step-2";

/// The Witch's `SinTreeSeenComment` (https://poe2db.tw/us/Witch, fetched
/// 2026-10-28): her remark on finding the Tree of Souls.
const WITCH_TREE: &str = "Well, well... who is this that adorns the Tree of Souls?";

/// The character in the Grelwood over the shipped data, Secrets in the Dark
/// started on the Clearfell entry: "Search the Grelwood" is open there.
fn in_the_grelwood() -> Vec<String> {
    let mut lines = in_the_riverbank();
    for (time, code) in [
        ("19:03:00", "G1_town"),
        ("19:05:00", "G1_2"),
        ("19:12:00", "G1_town"),
        ("19:14:00", "G1_4"),
    ] {
        lines.push(area(time, code));
    }
    lines
}

/// Owner, 2026-10-28: a Ranger's remark at the tree did not move Secrets in
/// the Dark. In the Grelwood the reader now reads the voiced lines for
/// "Search the Grelwood", and the character's new tree remark ticks it,
/// with its row change and its `remark` line.
#[test]
fn tree_demand_reads_the_voiced_lines_in_the_grelwood() {
    let spoken = || {
        Frame::Lines(vec![(
            format!("{OWN}: {WITCH_TREE}"),
            Rect::new(0.03, 0.71, 0.30, 0.73),
        )])
    };
    let h = harness(
        Setup {
            shipped: true,
            ..Setup::default()
        },
        |s| {
            let mut voiced: Vec<Frame> = (0..8).map(|_| Frame::Lines(Vec::new())).collect();
            voiced.extend((0..12).map(|_| spoken()));
            s.script(Region::VoicedLines, voiced);
        },
    );
    h.app.start();
    h.feed(&in_the_grelwood());

    h.wait_diag(&["screen: reading voiced lines for", TREE_STEP]);
    let line = h.wait_diag(&["screen: remark SinTreeSeenComment ("]);
    assert!(line.ends_with(&format!("): ticked {TREE_STEP}")), "{line}");
    wait_until("the row change", || {
        h.row_changes()
            .iter()
            .any(|c| c.step_done.as_deref() == Some("Search the Grelwood"))
    });
    // One tick, however long the line stays on screen.
    let remarks = h
        .screen_lines()
        .into_iter()
        .filter(|l| l.starts_with("screen: remark "))
        .count();
    assert_eq!(remarks, 1, "{:?}", h.screen_lines());
    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));
}
