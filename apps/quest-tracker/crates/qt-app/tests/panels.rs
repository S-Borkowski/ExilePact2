//! "Hide under game panels" in the shell (Context "Design: the panel
//! watcher"), driven by a fake backend over a synthetic panel table. The
//! fake paints a panel's frame colour into a copy when the copy is of one of
//! that panel's probes and the test has the panel showing (or, for the
//! best-read lines, stripes of the right mean but too much spread, or one
//! colour off the frame's), and grey otherwise; the fake window keeps the
//! overlay's shown state, so the
//! overlay the backend reports is hidden once the shell hides it. Nothing
//! here captures the screen or opens a window. Character names are
//! invented.
//!
//! Hiding under game panels is always on since 0.1.1: the watcher starts at
//! every start the shell gives a backend, whatever a 0.1.0 `settings.json`
//! said, and no setting stops it.
//!
//! Baseline: with no panel backend no watcher starts and the overlay never
//! hides for a panel, with a panel showing in the fake's copies.
//! Target: with a backend, a panel open under the overlay hides it, with its
//! line, and the overlay shows again once the panel closes.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use qt_app::panels::{NO_OVERLAY_STATUS, STOPPED_STATUS};
use qt_app::screen::CAPTURE_FAILURE_LIMIT;
use qt_app::{
    App, AppConfig, Clock, Emitter, Mode, MonitorMetrics, PanelConfig, Placement, ScreenBackend,
    Window, HIDDEN_LINE, PANEL_LINE, SHOWN_LINE, SHUTDOWN_TIMEOUT,
};
use qt_screen::{
    overlay_fraction, shipped_panels, BackendError, Image, OcrReadiness, OverlayState, PanelEntry,
    PanelProbe, PanelTable, PixelLine, PixelRect, Rect, WindowHandle,
};
use qt_store::Store;
use qt_tailer::{Fingerprint, Line, TailEvent};
use qt_viewmodel::{RowChange, StatusLine, View};
use serde_json::json;

const OWN: &str = "Corisande_Fell";
/// The pid in the session's headers.
const GAME: u32 = 7120;
/// Another program's window.
const OTHER: u32 = 9_999;
/// The game's client rect: 16:9 at the measured size.
const CLIENT: PixelRect = PixelRect::new(0, 0, 2560, 1440);
/// The overlay at its default place: x 0.78 to 1.0, y 0.30 to 0.97.
const OVERLAY: PixelRect = PixelRect::new(1997, 432, 2560, 1397);
/// The synthetic frame colour (blue, green, red) and the grey of the scene
/// behind it. No line, status or error may carry one of these numbers.
const FRAME: [u8; 3] = [123, 157, 189];
const GREY: [u8; 3] = [91, 91, 91];
const PIXEL_NUMBERS: [&str; 4] = ["123", "157", "189", "91"];
/// Two colours whose mean is `FRAME`, in alternate pixels: in colour, with a
/// luma spread of 30, above the probes' 4.
const STRIPES: [[u8; 3]; 2] = [[93, 127, 159], [153, 187, 219]];
/// One colour throughout, 30 off `FRAME` in each channel: within the spread,
/// off colour.
const OFF_COLOUR: [u8; 3] = [153, 187, 219];

/// How the fake paints the copies of a showing panel's probes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Paint {
    /// The frame colour: every probe matches.
    Frame,
    /// [`STRIPES`]: in colour, above the spread.
    Striped,
    /// [`OFF_COLOUR`]: within the spread, off colour.
    OffColour,
}

// ---------------------------------------------------------------------------
// The synthetic table.

fn probe(rect: Rect) -> PanelProbe {
    PanelProbe {
        rect,
        bgr: [FRAME[0] as f64, FRAME[1] as f64, FRAME[2] as f64],
        tolerance: [8.0, 8.0, 8.0],
        max_luma_std: 4.0,
    }
}

/// `right`: the right half, under the overlay's default place, with a third
/// probe under the overlay itself (read only while it is hidden). `left`:
/// the left side, beside it.
fn table() -> PanelTable {
    PanelTable::new(vec![
        PanelEntry {
            id: "right",
            opens_for: vec!["passive-tree"],
            footprint: Rect::new(0.5, 0.0, 1.0, 1.0),
            probes: vec![
                probe(Rect::new(0.55, 0.10, 0.56, 0.12)),
                probe(Rect::new(0.60, 0.90, 0.61, 0.92)),
                probe(Rect::new(0.90, 0.50, 0.91, 0.52)),
            ],
            min_match: 2,
        },
        PanelEntry {
            id: "left",
            opens_for: vec!["inventory", "stash"],
            footprint: Rect::new(0.0, 0.0, 0.45, 1.0),
            probes: vec![
                probe(Rect::new(0.02, 0.10, 0.03, 0.12)),
                probe(Rect::new(0.40, 0.90, 0.41, 0.92)),
            ],
            min_match: 2,
        },
    ])
}

const ENTRY_IDS: [&str; 2] = ["right", "left"];

/// The probe `area` (screen pixels) is a copy of, as `<entry>#<probe>`.
fn probe_at(area: PixelRect) -> Option<String> {
    let t = table();
    for e in &t.entries {
        for (i, p) in e.probes.iter().enumerate() {
            if p.pixels(CLIENT) == area {
                return Some(format!("{}#{i}", e.id));
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Fakes.

/// The scene both fakes share: what is in front, the overlay, the game's
/// window, which panels show, and every backend call.
struct Scene {
    foreground: Mutex<Option<u32>>,
    window_calls: Mutex<Vec<String>>,
    overlay: Mutex<Option<OverlayState>>,
    no_window: AtomicBool,
    /// `game_window` fails, which ends the watcher.
    window_fails: AtomicBool,
    minimized: AtomicBool,
    client: Mutex<PixelRect>,
    /// The panels showing, and how their probes' copies are painted.
    showing: Mutex<BTreeMap<&'static str, Paint>>,
    capture_fails: AtomicBool,
    /// Copies painted from a fixture's sample, by the probe's pixel rect.
    /// When there are any, every other copy is the scene's grey.
    painted: Mutex<Vec<(PixelRect, Image)>>,
    calls: Mutex<Vec<String>>,
}

impl Scene {
    fn new() -> Scene {
        Scene {
            foreground: Mutex::new(Some(GAME)),
            window_calls: Mutex::new(Vec::new()),
            overlay: Mutex::new(Some(OverlayState {
                rect: OVERLAY,
                visible: true,
            })),
            no_window: AtomicBool::new(false),
            window_fails: AtomicBool::new(false),
            minimized: AtomicBool::new(false),
            client: Mutex::new(CLIENT),
            showing: Mutex::new(BTreeMap::new()),
            capture_fails: AtomicBool::new(false),
            painted: Mutex::new(Vec::new()),
            calls: Mutex::new(Vec::new()),
        }
    }

    fn show(&self, id: &'static str) {
        self.paint(id, Paint::Frame);
    }

    /// Paints `id`'s probes' copies with `paint` from now on.
    fn paint(&self, id: &'static str, paint: Paint) {
        self.showing.lock().unwrap().insert(id, paint);
    }

    fn close(&self, id: &'static str) {
        self.showing.lock().unwrap().remove(id);
    }

    fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }

    /// The probes copied, in order, as `<entry>#<probe>`.
    fn copies(&self) -> Vec<String> {
        self.calls()
            .into_iter()
            .filter_map(|c| c.strip_prefix("capture ").map(str::to_string))
            .collect()
    }

    fn window_calls(&self) -> Vec<String> {
        self.window_calls.lock().unwrap().clone()
    }

    fn set_overlay(&self, overlay: Option<PixelRect>) {
        let mut o = self.overlay.lock().unwrap();
        let visible = o.map_or(true, |o| o.visible);
        *o = overlay.map(|rect| OverlayState { rect, visible });
    }

    fn record(&self, call: String) {
        self.calls.lock().unwrap().push(call);
    }
}

struct FakeWindow(Arc<Scene>);

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
    fn open_url(&self, _: &str) -> Result<(), String> {
        Ok(())
    }
    fn monitor(&self) -> Result<Option<MonitorMetrics>, String> {
        Ok(None)
    }
    fn foreground_pid(&self) -> Option<u32> {
        *self.0.foreground.lock().unwrap()
    }
    fn set_visible(&self, shown: bool) -> Result<(), String> {
        self.0.window_calls.lock().unwrap().push(format!("visible {shown}"));
        if let Some(o) = self.0.overlay.lock().unwrap().as_mut() {
            o.visible = shown;
        }
        Ok(())
    }
}

struct FakeBackend(Arc<Scene>);

impl ScreenBackend for FakeBackend {
    fn game_window(&mut self, pid: u32) -> Result<Option<WindowHandle>, BackendError> {
        self.0.record(format!("game_window {pid}"));
        if self.0.window_fails.load(Ordering::SeqCst) {
            return Err(BackendError::new("game_window: the fake failed"));
        }
        Ok((pid == GAME && !self.0.no_window.load(Ordering::SeqCst)).then_some(WindowHandle(42)))
    }

    fn foreground_pid(&mut self) -> Result<Option<u32>, BackendError> {
        self.0.record("foreground_pid".to_string());
        Ok(*self.0.foreground.lock().unwrap())
    }

    fn client_rect(&mut self, _: WindowHandle) -> Result<Option<PixelRect>, BackendError> {
        self.0.record("client_rect".to_string());
        if self.0.minimized.load(Ordering::SeqCst) {
            return Ok(None);
        }
        Ok(Some(*self.0.client.lock().unwrap()))
    }

    fn overlay(&mut self) -> Result<Option<OverlayState>, BackendError> {
        self.0.record("overlay".to_string());
        Ok(*self.0.overlay.lock().unwrap())
    }

    fn capture(&mut self, _: WindowHandle, area: PixelRect) -> Result<Image, BackendError> {
        let which = probe_at(area).unwrap_or_else(|| "elsewhere".to_string());
        self.0.record(format!("capture {which}"));
        if self.0.capture_fails.load(Ordering::SeqCst) {
            return Err(BackendError::new("capture: the fake failed"));
        }
        {
            let painted = self.0.painted.lock().unwrap();
            if !painted.is_empty() {
                let grey = Image {
                    width: 4,
                    height: 4,
                    bgra: [GREY[0], GREY[1], GREY[2], 255].repeat(16),
                };
                return Ok(painted
                    .iter()
                    .find(|(rect, _)| *rect == area)
                    .map_or(grey, |(_, image)| image.clone()));
            }
        }
        let showing = self.0.showing.lock().unwrap();
        let paint = showing
            .iter()
            .find(|(id, _)| which.starts_with(&format!("{id}#")))
            .map(|(_, p)| *p);
        // Two colours in alternate pixels, or one throughout.
        let (a, b) = match paint {
            Some(Paint::Frame) => (FRAME, FRAME),
            Some(Paint::Striped) => (STRIPES[0], STRIPES[1]),
            Some(Paint::OffColour) => (OFF_COLOUR, OFF_COLOUR),
            None => (GREY, GREY),
        };
        let bgra = (0..16)
            .flat_map(|i| {
                let c = if i % 2 == 0 { a } else { b };
                [c[0], c[1], c[2], 255]
            })
            .collect();
        Ok(Image {
            width: 4,
            height: 4,
            bgra,
        })
    }

    fn ocr_readiness(&mut self) -> Result<OcrReadiness, BackendError> {
        self.0.record("ocr_readiness".to_string());
        Err(BackendError::new("the panel watcher has no OCR"))
    }

    fn ocr(&mut self, _: &Image) -> Result<Vec<PixelLine>, BackendError> {
        self.0.record("ocr".to_string());
        Err(BackendError::new("the panel watcher has no OCR"))
    }
}

struct NoEmitter;

impl Emitter for NoEmitter {
    fn view(&self, _: &View) {}
    fn row_change(&self, _: &RowChange) {}
}

struct FakeClock(Arc<AtomicU64>);

impl Clock for FakeClock {
    fn now_ms(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}

// ---------------------------------------------------------------------------
// Log lines.

fn line(time: &str, message: &str) -> String {
    format!("2026/03/10 {time} 4000000 7e2c0001 [INFO Client {GAME}] {message}")
}

fn area(time: &str, code: &str) -> String {
    format!(
        "2026/03/10 {time} 4000000 7e2c0002 [DEBUG Client {GAME}] Generating level 2 area \"{code}\" with seed 2077351921"
    )
}

/// An own character in The Riverbank, live: the game pid is confirmed and
/// a character is in an area.
fn in_the_riverbank() -> Vec<String> {
    vec![
        "2026/03/10 19:00:00 ***** LOG FILE OPENING *****".to_string(),
        area("19:00:05", "G1_1"),
        line("19:00:20", &format!(": {OWN} has received +20 to maximum Life.")),
        line("19:00:40", &format!(": {OWN} (Witch) is now level 9")),
    ]
}

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
    scene: Arc<Scene>,
    clock: Arc<AtomicU64>,
}

struct Setup {
    /// The `hide_under_panels` key of a 0.1.0 `settings.json` in the store,
    /// or no settings file at all (`None`).
    old_key: Option<bool>,
    /// Whether the app gets a panel backend at all.
    backend: bool,
    /// The app's own table (`shipped_panels`) instead of the synthetic one.
    shipped: bool,
}

impl Default for Setup {
    fn default() -> Setup {
        Setup {
            old_key: None,
            backend: true,
            shipped: false,
        }
    }
}

fn harness(setup: Setup) -> Harness {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path().to_path_buf();
    let store = Store::open(root.join("store")).expect("store");
    if let Some(on) = setup.old_key {
        store
            .save_settings(&json!({ "opacity": 0.5, "hide_under_panels": on }))
            .expect("settings written");
    }
    let scene = Arc::new(Scene::new());
    let clock = Arc::new(AtomicU64::new(100_000));
    let config = AppConfig {
        options: Default::default(),
        options_error: None,
        store: Store::open(root.join("store")).map_err(|e| e.to_string()),
        data: None,
        candidates: Vec::new(),
        poll_interval: Duration::from_millis(20),
        emitter: Box::new(NoEmitter),
        window: Box::new(FakeWindow(Arc::clone(&scene))),
        clock: Box::new(FakeClock(Arc::clone(&clock))),
        screen: None,
    };
    let app = if setup.backend {
        App::with_panels(
            config,
            PanelConfig {
                backend: Some(Box::new(FakeBackend(Arc::clone(&scene)))),
                table: (!setup.shipped).then(table),
            },
        )
    } else {
        App::new(config)
    };
    Harness {
        _dir: dir,
        root,
        app,
        scene,
        clock,
    }
}

/// A harness started with the character in an area and the game in front.
fn playing(setup: Setup) -> Harness {
    let h = harness(setup);
    h.app.start();
    h.feed(&in_the_riverbank());
    h.app.tick();
    h
}

impl Harness {
    fn diag_text(&self) -> String {
        fs::read_to_string(self.root.join("store").join("overlay.log")).unwrap_or_default()
    }

    /// The log's lines without its own prefix, from where `prefix` starts.
    fn lines_from(&self, prefix: &str) -> Vec<String> {
        self.diag_text()
            .lines()
            .filter_map(|l| l.find(prefix).map(|i| l[i..].to_string()))
            .collect()
    }

    fn panel_lines(&self) -> Vec<String> {
        self.lines_from("panels: ")
    }

    fn visibility_lines(&self) -> Vec<String> {
        self.lines_from("visibility: ")
    }

    /// Waits up to five seconds for a log line holding every one of `parts`.
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

    /// Waits for the `n`th line holding `part` (1 is the first).
    fn wait_count(&self, part: &str, n: usize) {
        wait_until(&format!("{n} lines with {part:?}"), || {
            self.diag_text().lines().filter(|l| l.contains(part)).count() >= n
        });
    }

    fn feed(&self, lines: &[String]) {
        self.app.handle_tail_events(events(lines));
    }

    fn in_front(&self, pid: Option<u32>) {
        *self.scene.foreground.lock().unwrap() = pid;
        self.app.tick();
    }

    fn status(&self) -> Option<StatusLine> {
        self.app.get_view().status
    }
}

fn wait_until(what: &str, pred: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !pred() {
        assert!(Instant::now() < deadline, "{what}: not within 5 s");
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// `0.00` to `1.00`, two decimals.
fn fraction2(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 4
        && b[1] == b'.'
        && [b[0], b[2], b[3]].iter().all(u8::is_ascii_digit)
        && s.parse::<f64>().is_ok_and(|v| (0.0..=1.0).contains(&v))
}

fn number(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

/// Whether `text`'s words are `shape`'s, each `#` a number.
fn fits(text: &str, shape: &[&str]) -> bool {
    let words: Vec<&str> = text.split(' ').collect();
    words.len() == shape.len()
        && words
            .iter()
            .zip(shape)
            .all(|(w, s)| if *s == "#" { number(w) } else { w == s })
}

/// Whether `line` (from `panels: `) has one of the shapes the Context pins.
fn pinned(line: &str) -> bool {
    let Some(rest) = line.strip_prefix("panels: ") else {
        return false;
    };
    if ["on", "resumed", "stopped after an error"].contains(&rest) {
        return true;
    }
    if let Some(r) = rest.strip_prefix("idle (").and_then(|r| r.strip_suffix(')')) {
        return [
            "edit mode",
            "overlay hidden",
            "pid unconfirmed",
            "no game window",
            "minimized",
        ]
        .contains(&r);
    }
    if let Some(r) = rest.strip_prefix("paused (").and_then(|r| r.strip_suffix(')')) {
        if r == "no overlay window"
            || r == format!("the capture failed {CAPTURE_FAILURE_LIMIT} times in a row")
        {
            return true;
        }
        let Some(size) = r
            .strip_prefix("the game window is not 16:9 (")
            .and_then(|s| s.strip_suffix(')'))
        else {
            return false;
        };
        return size
            .split_once('x')
            .is_some_and(|(w, h)| number(w) && number(h));
    }
    if let Some(r) = rest.strip_prefix("overlay at x ") {
        let parts: Vec<&str> = r.split(' ').collect();
        return parts.len() == 7
            && fraction2(parts[0])
            && parts[1] == "to"
            && parts[2].strip_suffix(',').is_some_and(fraction2)
            && parts[3] == "y"
            && fraction2(parts[4])
            && parts[5] == "to"
            && fraction2(parts[6]);
    }
    if let Some(r) = rest.strip_prefix("minute: ") {
        return fits(
            r,
            &[
                "#", "reads,", "#", "copies,", "#", "open,", "#", "black", "copies,", "busy", "#",
                "ms",
            ],
        );
    }
    if let Some((id, r)) = rest.split_once(" best read: ") {
        return ENTRY_IDS.contains(&id)
            && fits(
                r,
                &[
                    "#", "of", "#", "probes", "read,", "#", "in", "colour,", "#", "within",
                    "spread,", "#", "match,", "#", "needed",
                ],
            );
    }
    ENTRY_IDS.iter().any(|id| {
        [" open", " closed", " unseen under the overlay"]
            .iter()
            .any(|s| rest == format!("{id}{s}"))
    })
}

/// `l` with the minute line's `busy <N> ms` figure blanked to `busy # ms`.
/// That figure is measured wall-clock time, so it may be any number,
/// including one of the fake's pixel values; it is the one number the
/// pixel scan must not read. The rest of the line is still scanned.
fn without_busy_ms(l: &str) -> String {
    let Some(at) = l.find(" busy ") else {
        return l.to_owned();
    };
    let start = at + " busy ".len();
    let digits = l[start..].bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 || !l[start + digits..].starts_with(" ms") {
        return l.to_owned();
    }
    format!("{}#{}", &l[..start], &l[start + digits..])
}

#[test]
fn pixel_scan_skips_only_the_busy_figure() {
    // The measured figure is blanked, whatever it is.
    let line = "panels: minute: 2 reads, 4 copies, 1 open, 0 black copies, busy 91 ms";
    assert_eq!(
        without_busy_ms(line),
        "panels: minute: 2 reads, 4 copies, 1 open, 0 black copies, busy # ms"
    );
    // Negative control: a pixel number anywhere else on the line still shows.
    let elsewhere = "panels: minute: 91 reads, 4 copies, 1 open, 0 black copies, busy 3 ms";
    assert!(without_busy_ms(elsewhere).contains("91"));
    // A line without the figure is left alone.
    assert_eq!(without_busy_ms("status 123"), "status 123");
}

/// Every `panels:` line has a pinned shape, and none of them, no
/// visibility line, error line or status carries a number of the fake's
/// pixels.
fn assert_pinned(h: &Harness) {
    let lines = h.panel_lines();
    for l in &lines {
        assert!(pinned(l), "not a pinned shape: {l:?}\n{lines:#?}");
    }
    let mut said: Vec<String> = lines;
    said.extend(h.visibility_lines());
    said.extend(h.lines_from("error: panel watcher: "));
    said.extend(h.status().map(|s| s.text));
    for l in &said {
        let scanned = without_busy_ms(l);
        for n in PIXEL_NUMBERS {
            assert!(!scanned.contains(n), "{l:?} carries {n}");
        }
    }
    for l in h.visibility_lines() {
        assert!(
            [SHOWN_LINE, HIDDEN_LINE, PANEL_LINE].contains(&l.as_str())
                || l.starts_with("visibility: hidden (no character"),
            "{l}"
        );
    }
}

// ---------------------------------------------------------------------------
// Tests.

#[test]
fn panels_no_backend_starts_no_watcher_and_never_hides_baseline() {
    // The same session, with a panel showing in the fake's copies, on apps
    // with no panel backend: with no settings file, and with a 0.1.0 file
    // that turned the setting on.
    let run = |old_key: Option<bool>| {
        let h = harness(Setup {
            old_key,
            backend: false,
            ..Setup::default()
        });
        h.scene.show("right");
        h.scene.show("left");
        h.app.start();
        h.feed(&in_the_riverbank());
        h.in_front(Some(GAME));
        std::thread::sleep(Duration::from_millis(600));
        h.in_front(Some(OTHER));
        h.wait_diag(&[HIDDEN_LINE]);
        h.in_front(Some(GAME));
        h.wait_diag(&[SHOWN_LINE]);
        std::thread::sleep(Duration::from_millis(300));
        assert!(!h.app.panel_watcher_running());
        h
    };
    let h = run(None);
    assert!(h.scene.calls().is_empty(), "{:?}", h.scene.calls());
    assert!(h.panel_lines().is_empty(), "{:?}", h.panel_lines());
    assert_eq!(h.visibility_lines(), vec![HIDDEN_LINE, SHOWN_LINE]);
    assert_eq!(h.scene.window_calls(), vec!["visible false", "visible true"]);

    let old = run(Some(true));
    assert!(old.scene.calls().is_empty(), "{:?}", old.scene.calls());
    assert!(old.panel_lines().is_empty(), "{:?}", old.panel_lines());
    assert_eq!(old.visibility_lines(), h.visibility_lines());
    assert_eq!(old.scene.window_calls(), h.scene.window_calls());

    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));
    assert!(old.app.shutdown(SHUTDOWN_TIMEOUT));
    for x in [&h, &old] {
        assert!(
            x.diag_text()
                .contains("stop: tailer and worker ended, progress saved"),
            "{}",
            x.diag_text()
        );
    }
    assert_pinned(&h);
}

/// The settings the store's `settings.json` holds, as JSON.
fn saved_settings(h: &Harness) -> serde_json::Value {
    let saved = Store::open(h.root.join("store"))
        .expect("store")
        .load_settings()
        .expect("settings.json read")
        .value
        .expect("settings.json saved");
    assert!(saved.is_object(), "{saved}");
    saved
}

#[test]
fn panels_watcher_starts_with_default_settings() {
    // No settings file: a first run, on the defaults.
    let h = harness(Setup::default());
    h.app.start();
    h.wait_diag(&["panels: on"]);
    assert!(h.app.panel_watcher_running());
    assert_eq!(h.app.get_view().settings, qt_viewmodel::Settings::default());
    // It hides the overlay for a panel under it, with nothing turned on.
    h.scene.show("right");
    h.feed(&in_the_riverbank());
    h.in_front(Some(GAME));
    h.wait_diag(&[PANEL_LINE]);
    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));
    assert!(h
        .diag_text()
        .contains("stop: tailer, worker and panel watcher ended, progress saved"));
    // What it saved holds the settings, and no such setting among them.
    let saved = saved_settings(&h);
    assert!(saved.get("opacity").is_some(), "{saved}");
    assert!(saved.get("hide_under_panels").is_none(), "{saved}");
    assert_pinned(&h);
}

#[test]
fn panels_watcher_starts_when_an_old_file_turned_it_off() {
    // A 0.1.0 settings.json that turned the setting off, and, as a control,
    // one that turned it on: the watcher starts at start either way, and the
    // next save drops the key while keeping the rest of the file.
    for old_key in [false, true] {
        let h = harness(Setup {
            old_key: Some(old_key),
            ..Setup::default()
        });
        assert_eq!(saved_settings(&h)["hide_under_panels"], old_key);
        h.app.start();
        h.wait_diag(&["panels: on"]);
        assert!(h.app.panel_watcher_running(), "{old_key}");
        assert_eq!(h.app.get_view().settings.opacity, 0.5);
        h.scene.show("right");
        h.feed(&in_the_riverbank());
        h.in_front(Some(GAME));
        h.wait_diag(&[PANEL_LINE]);
        wait_until("the overlay hidden", || {
            h.scene.window_calls() == vec!["visible false"]
        });
        // A change of another setting saves the file.
        h.app
            .update_settings(&json!({ "show_skipped": true }))
            .expect("a known setting");
        assert!(h.app.panel_watcher_running(), "{old_key}");
        assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));
        let saved = saved_settings(&h);
        assert!(saved.get("hide_under_panels").is_none(), "{old_key}: {saved}");
        assert_eq!(saved["opacity"].as_f64(), Some(0.5), "{saved}");
        assert_eq!(saved["show_skipped"].as_bool(), Some(true), "{saved}");
        assert_pinned(&h);
    }
}

#[test]
fn panels_panel_under_the_overlay_hides_it_target() {
    let h = harness(Setup::default());
    h.scene.show("right");
    h.app.start();
    h.wait_diag(&["panels: on"]);
    h.feed(&in_the_riverbank());
    h.in_front(Some(GAME));

    h.wait_diag(&["panels: right open"]);
    h.wait_diag(&[PANEL_LINE]);
    wait_until("the overlay hidden", || {
        h.scene.window_calls() == vec!["visible false"]
    });
    assert!(h.app.panel_watcher_running());
    let lines = h.panel_lines();
    assert!(
        lines.contains(&"panels: overlay at x 0.78 to 1.00, y 0.30 to 0.97".to_string()),
        "{lines:?}"
    );
    // While the overlay was shown, the probe under it was not copied; only
    // the panel under the overlay is copied, never the one beside it.
    let copies = h.scene.copies();
    assert!(copies.iter().all(|c| c.starts_with("right#")), "{copies:?}");
    assert_eq!(&copies[..2], ["right#0", "right#1"]);
    // Its own backend, asked only what the watcher needs.
    assert!(h
        .scene
        .calls()
        .iter()
        .all(|c| !c.starts_with("ocr") && c != "foreground_pid"));
    // Hidden, it keeps reading, now every probe, and the panel stays open.
    wait_until("the probe under the hidden overlay copied", || {
        h.scene.copies().contains(&"right#2".to_string())
    });
    std::thread::sleep(Duration::from_millis(400));
    assert_eq!(h.scene.window_calls(), vec!["visible false"]);
    assert_eq!(h.visibility_lines(), vec![PANEL_LINE]);
    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));
    assert_pinned(&h);
}

#[test]
fn panels_panel_beside_the_overlay_does_not_hide_it() {
    let h = harness(Setup::default());
    h.scene.show("left");
    h.app.start();
    h.feed(&in_the_riverbank());
    h.in_front(Some(GAME));
    // Several ticks of reads.
    wait_until("six copies", || h.scene.copies().len() >= 6);
    std::thread::sleep(Duration::from_millis(300));
    let copies = h.scene.copies();
    assert!(copies.iter().all(|c| c.starts_with("right#")), "{copies:?}");
    assert!(h.panel_lines().iter().all(|l| !l.contains("left")));
    assert!(h.visibility_lines().is_empty(), "{:?}", h.visibility_lines());
    assert!(h.scene.window_calls().is_empty());

    // Positive control, same run: the panel under the overlay opens.
    h.scene.show("right");
    h.wait_diag(&[PANEL_LINE]);
    assert_eq!(h.scene.window_calls(), vec!["visible false"]);
    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));
    assert_pinned(&h);
}

#[test]
fn panels_closed_panel_shows_the_overlay_again() {
    let h = harness(Setup::default());
    h.scene.show("right");
    h.app.start();
    h.feed(&in_the_riverbank());
    h.in_front(Some(GAME));
    h.wait_diag(&[PANEL_LINE]);

    h.scene.close("right");
    h.wait_diag(&["panels: right closed"]);
    h.wait_diag(&[SHOWN_LINE]);
    wait_until("the overlay shown", || {
        h.scene.window_calls() == vec!["visible false", "visible true"]
    });
    assert_eq!(h.visibility_lines(), vec![PANEL_LINE, SHOWN_LINE]);
    // It closed after the closed reads in a row, read with every probe
    // while the overlay was hidden.
    let lines = h.panel_lines();
    let opened = lines.iter().position(|l| l == "panels: right open").unwrap();
    let closed = lines.iter().position(|l| l == "panels: right closed").unwrap();
    assert!(opened < closed, "{lines:?}");

    // Open again: hidden again.
    h.scene.show("right");
    h.wait_count(PANEL_LINE, 2);
    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));
    assert_pinned(&h);
}

#[test]
fn panels_read_nothing_while_the_overlay_is_hidden_for_another_reason() {
    let h = harness(Setup::default());
    h.scene.show("right");
    // Another program in front before the watcher starts.
    *h.scene.foreground.lock().unwrap() = Some(OTHER);
    h.feed(&in_the_riverbank());
    h.app.tick();
    h.wait_diag(&[HIDDEN_LINE]);
    h.app.start();
    h.wait_diag(&["panels: idle (overlay hidden)"]);
    std::thread::sleep(Duration::from_millis(700));
    assert!(h.scene.copies().is_empty(), "{:?}", h.scene.copies());
    assert!(h.panel_lines().iter().all(|l| !l.contains("open")));
    assert_eq!(h.visibility_lines(), vec![HIDDEN_LINE]);

    // Positive control: the game in front shows it, the watcher reads, and
    // the panel hides it.
    h.in_front(Some(GAME));
    h.wait_diag(&[SHOWN_LINE]);
    h.wait_diag(&["panels: resumed"]);
    h.wait_diag(&[PANEL_LINE]);

    // Hidden for another reason while a panel hides it: the watcher idles
    // and drops its hide, and copies nothing more.
    h.in_front(Some(OTHER));
    h.wait_count("panels: idle (overlay hidden)", 2);
    wait_until("the hidden line", || {
        h.visibility_lines().last().map(String::as_str) == Some(HIDDEN_LINE)
    });
    std::thread::sleep(Duration::from_millis(100));
    let copied = h.scene.copies().len();
    std::thread::sleep(Duration::from_millis(600));
    assert_eq!(h.scene.copies().len(), copied);
    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));
    assert_pinned(&h);
}

#[test]
fn panels_edit_mode_never_hides() {
    let h = playing(Setup::default());
    h.scene.show("right");
    h.wait_diag(&[PANEL_LINE]);

    h.app.set_mode(Mode::Edit).expect("edit mode");
    h.wait_diag(&["panels: idle (edit mode)"]);
    h.wait_diag(&[SHOWN_LINE]);
    wait_until("the overlay shown", || {
        h.scene.window_calls().last().map(String::as_str) == Some("visible true")
    });
    std::thread::sleep(Duration::from_millis(100));
    let copied = h.scene.copies().len();
    std::thread::sleep(Duration::from_millis(600));
    assert_eq!(h.scene.copies().len(), copied, "no copy in edit mode");
    assert_eq!(h.visibility_lines(), vec![PANEL_LINE, SHOWN_LINE]);

    // Back to play mode, the panel still open: hidden again.
    h.app.set_mode(Mode::Play).expect("play mode");
    h.wait_count(PANEL_LINE, 2);
    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));
    assert_pinned(&h);
}

#[test]
fn panels_without_an_overlay_window_pause_with_a_status_line() {
    let h = harness(Setup::default());
    h.scene.show("right");
    h.scene.set_overlay(None);
    h.app.start();
    h.feed(&in_the_riverbank());
    h.in_front(Some(GAME));
    h.wait_diag(&["panels: paused (no overlay window)"]);
    wait_until("the status line", || {
        h.status() == Some(StatusLine::warn(NO_OVERLAY_STATUS))
    });
    std::thread::sleep(Duration::from_millis(500));
    assert!(h.scene.copies().is_empty(), "{:?}", h.scene.copies());
    assert!(h.visibility_lines().is_empty(), "{:?}", h.visibility_lines());

    // The window found again: reading resumes, the status clears, and the
    // panel hides the overlay.
    h.scene.set_overlay(Some(OVERLAY));
    h.wait_diag(&["panels: resumed"]);
    h.wait_diag(&[PANEL_LINE]);
    wait_until("the status cleared", || h.status().is_none());
    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));
    assert_pinned(&h);
}

#[test]
fn panels_patch_naming_the_old_setting_keeps_the_overlay_hidden() {
    let h = playing(Setup::default());
    h.scene.show("right");
    h.wait_diag(&[PANEL_LINE]);
    wait_until("the overlay hidden", || {
        h.scene.window_calls() == vec!["visible false"]
    });

    // The 0.1.0 switch is refused as an unknown setting and stops nothing.
    let err = h
        .app
        .update_settings(&json!({ "hide_under_panels": false }))
        .unwrap_err();
    assert!(err.contains("unknown setting"), "{err}");
    assert!(h.app.panel_watcher_running());
    let copied = h.scene.copies().len();
    std::thread::sleep(Duration::from_millis(600));
    assert!(h.scene.copies().len() > copied);
    assert_eq!(h.scene.window_calls(), vec!["visible false"]);
    assert_eq!(h.visibility_lines(), vec![PANEL_LINE]);

    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));
    assert!(h
        .diag_text()
        .contains("stop: tailer, worker and panel watcher ended, progress saved"));
    assert_pinned(&h);
}

#[test]
fn panels_watcher_stops_within_the_shutdown_deadline() {
    // Reading, with the overlay hidden by a panel.
    let h = playing(Setup::default());
    h.scene.show("right");
    h.wait_diag(&[PANEL_LINE]);
    let started = Instant::now();
    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));
    assert!(started.elapsed() < SHUTDOWN_TIMEOUT, "{:?}", started.elapsed());
    assert!(!h.app.panel_watcher_running());
    assert!(
        h.diag_text()
            .contains("stop: tailer, worker and panel watcher ended, progress saved"),
        "{}",
        h.diag_text()
    );
    assert_pinned(&h);

    // Idle (edit mode): it ends as fast.
    let h = playing(Setup::default());
    h.app.set_mode(Mode::Edit).expect("edit mode");
    h.wait_diag(&["panels: idle (edit mode)"]);
    let started = Instant::now();
    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));
    assert!(started.elapsed() < Duration::from_millis(1_000), "{:?}", started.elapsed());
    assert!(h
        .diag_text()
        .contains("stop: tailer, worker and panel watcher ended, progress saved"));
    assert_pinned(&h);
}

/// The best-read line the Context pins, for `id` and its counts.
fn best_read(id: &str, [r, n, c, s, m, k]: [usize; 6]) -> String {
    format!(
        "panels: {id} best read: {r} of {n} probes read, {c} in colour, {s} within spread, {m} match, {k} needed"
    )
}

/// The panel lines from the `n`th `minute:` line (0 is the first) on.
fn after_minute(h: &Harness, n: usize) -> Vec<String> {
    let lines = h.panel_lines();
    let at = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.starts_with("panels: minute: "))
        .nth(n)
        .map(|(i, _)| i)
        .unwrap_or_else(|| panic!("no minute line {n}: {lines:#?}"));
    lines[at..].to_vec()
}

#[test]
fn panels_minute_reports_each_entrys_best_read_in_counts() {
    // `right` at the overlay's default place: three probes, two needed, the
    // third under the visible overlay and so not read while it shows.
    let t = table();
    let right = &t.entries[0];
    let place = overlay_fraction(CLIENT, OVERLAY);
    assert_eq!(right.readable_probes(&place, true), vec![0, 1]);
    assert_eq!((right.probes.len(), right.min_match), (3, 2));
    assert!(!t.entries[1].is_under(&place));

    // Copies of the right mean with too much spread.
    let h = harness(Setup::default());
    h.scene.paint("right", Paint::Striped);
    h.app.start();
    h.feed(&in_the_riverbank());
    h.in_front(Some(GAME));
    wait_until("six copies", || h.scene.copies().len() >= 6);
    // The game window lost, so no read crosses into the next minute, then a
    // minute on the shell's clock.
    h.scene.no_window.store(true, Ordering::SeqCst);
    h.wait_diag(&["panels: idle (no game window)"]);
    h.clock.fetch_add(60_000, Ordering::SeqCst);
    h.wait_diag(&["panels: right best read: "]);
    let minute = after_minute(&h, 0);
    assert_eq!(minute[1], best_read("right", [2, 3, 2, 0, 0, 2]), "{minute:#?}");
    assert!(minute.iter().all(|l| !l.contains("left best read")));

    // Copies within the spread, off colour.
    h.scene.paint("right", Paint::OffColour);
    let copied = h.scene.copies().len();
    h.scene.no_window.store(false, Ordering::SeqCst);
    wait_until("six more copies", || h.scene.copies().len() >= copied + 6);
    h.scene.no_window.store(true, Ordering::SeqCst);
    h.wait_count("panels: idle (no game window)", 2);
    h.clock.fetch_add(60_000, Ordering::SeqCst);
    h.wait_count("panels: right best read: ", 2);
    let minute = after_minute(&h, 1);
    assert_eq!(minute[1], best_read("right", [2, 3, 0, 2, 0, 2]), "{minute:#?}");

    // Neither read open: the overlay stayed shown throughout.
    let lines = h.panel_lines();
    assert!(lines.iter().all(|l| l != "panels: right open"), "{lines:#?}");
    assert!(h.visibility_lines().is_empty(), "{:?}", h.visibility_lines());
    assert!(h.scene.window_calls().is_empty());
    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));
    assert_pinned(&h);
}

#[test]
fn panels_stop_writes_the_pending_minute() {
    // Reads for less than a minute on the shell's clock, nothing showing.
    let h = playing(Setup::default());
    wait_until("four copies", || h.scene.copies().len() >= 4);
    assert!(h
        .panel_lines()
        .iter()
        .all(|l| !l.starts_with("panels: minute: ")));

    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));
    // The part minute's lines, then the stop line.
    let minute = after_minute(&h, 0);
    assert_eq!(
        minute,
        vec![minute[0].clone(), best_read("right", [2, 3, 0, 2, 0, 2])]
    );
    let text = h.diag_text();
    let best = text.find(&minute[1]).expect("the best-read line");
    let stop = text
        .find("stop: tailer, worker and panel watcher ended")
        .expect("the stop line");
    assert!(best < stop, "{text}");
    assert_eq!(
        h.panel_lines()
            .iter()
            .filter(|l| l.starts_with("panels: minute: "))
            .count(),
        1
    );
    assert_pinned(&h);
}

#[test]
fn panels_lines_use_only_the_pinned_shapes() {
    // One run through every line the watcher writes: on, the overlay's
    // place, open, closed, idle for each reason, resumed, each pause,
    // unseen, the minute and stopped after an error.
    let h = harness(Setup::default());
    h.app.start();
    h.wait_diag(&["panels: idle (pid unconfirmed)"]);
    h.feed(&in_the_riverbank());
    h.in_front(Some(GAME));
    h.wait_diag(&["panels: overlay at x "]);

    h.scene.show("right");
    h.wait_diag(&["panels: right open"]);
    h.scene.close("right");
    h.wait_diag(&["panels: right closed"]);
    h.wait_diag(&[SHOWN_LINE]);

    // The overlay over every probe of the right panel: nothing to judge by.
    h.scene.set_overlay(Some(PixelRect::new(1280, 0, 2560, 1440)));
    h.wait_diag(&["panels: right unseen under the overlay"]);
    h.wait_diag(&["panels: overlay at x 0.50 to 1.00, y 0.00 to 1.00"]);
    h.scene.set_overlay(Some(OVERLAY));

    h.scene.no_window.store(true, Ordering::SeqCst);
    h.wait_diag(&["panels: idle (no game window)"]);
    h.scene.no_window.store(false, Ordering::SeqCst);
    h.scene.minimized.store(true, Ordering::SeqCst);
    h.wait_diag(&["panels: idle (minimized)"]);
    h.scene.minimized.store(false, Ordering::SeqCst);
    h.wait_count("panels: resumed", 2);

    *h.scene.client.lock().unwrap() = PixelRect::new(0, 0, 2560, 1080);
    h.wait_diag(&["panels: paused (the game window is not 16:9 (2560x1080))"]);
    wait_until("the shape's status", || {
        h.status().is_some_and(|s| {
            s.text == "Hiding under game panels is paused while the game's window is not 16:9 (2560x1080)."
        })
    });
    *h.scene.client.lock().unwrap() = CLIENT;
    h.wait_count("panels: resumed", 3);

    h.scene.capture_fails.store(true, Ordering::SeqCst);
    h.wait_diag(&[&format!(
        "panels: paused (the capture failed {CAPTURE_FAILURE_LIMIT} times in a row)"
    )]);
    h.wait_diag(&["error: panel watcher: capture: the fake failed"]);
    wait_until("the capture's status", || {
        h.status().is_some_and(|s| {
            s.text
                == format!("Hiding under game panels cannot copy the game's window (the capture failed {CAPTURE_FAILURE_LIMIT} times in a row).")
        })
    });
    h.scene.capture_fails.store(false, Ordering::SeqCst);
    h.wait_count("panels: resumed", 4);
    wait_until("the status cleared", || h.status().is_none());

    // A minute on the shell's clock, with reads in it.
    h.scene.show("right");
    h.wait_count("panels: right open", 2);
    h.clock.fetch_add(60_000, Ordering::SeqCst);
    let minute = h.wait_diag(&["panels: minute: "]);
    assert!(minute.contains(" open, "), "{minute}");

    h.app.set_mode(Mode::Edit).expect("edit mode");
    h.wait_diag(&["panels: idle (edit mode)"]);
    h.app.set_mode(Mode::Play).expect("play mode");
    h.in_front(Some(OTHER));
    h.wait_diag(&["panels: idle (overlay hidden)"]);
    h.in_front(Some(GAME));

    // A backend call it cannot go on without fails: it stops, says so, and
    // drops its hide.
    h.wait_count("panels: right open", 3);
    h.wait_count(PANEL_LINE, 3);
    h.scene.window_fails.store(true, Ordering::SeqCst);
    h.wait_diag(&["error: panel watcher: game_window: the fake failed"]);
    h.wait_diag(&["panels: stopped after an error"]);
    wait_until("the stopped status", || {
        h.status() == Some(StatusLine::warn(STOPPED_STATUS))
    });
    wait_until("the overlay shown", || {
        h.visibility_lines().last().map(String::as_str) == Some(SHOWN_LINE)
    });
    assert!(!h.app.panel_watcher_running());
    assert_pinned(&h);

    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));

    let lines = h.panel_lines();
    for shape in ["panels: on", "panels: resumed"] {
        assert!(lines.iter().any(|l| l == shape), "{shape}: {lines:#?}");
    }
    assert_pinned(&h);
}

// ---------------------------------------------------------------------------
// The shipped table, on a measured sample.

const PANEL_FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/panels/shipped_panels.json"
);

/// A 16 × 16 copy with the mean `mean` (blue, green, red) and the luma
/// spread `luma_std`: two levels, the same offset in every channel, in the
/// share of pixels that keeps both inside 0..255.
fn paint_sample(mean: [f64; 3], luma_std: f64) -> Image {
    const N: usize = 256;
    let lo = mean.iter().cloned().fold(f64::MAX, f64::min);
    let hi = mean.iter().cloned().fold(f64::MIN, f64::max);
    let (k, up, down) = (1..N)
        .map(|k| {
            let p = k as f64 / N as f64;
            (k, luma_std * ((1.0 - p) / p).sqrt(), luma_std * (p / (1.0 - p)).sqrt())
        })
        .filter(|&(_, up, down)| hi + up <= 255.0 && lo - down >= 0.0)
        .min_by_key(|&(k, _, _)| k.abs_diff(N / 2))
        .expect("a spread that fits");
    let level = |d: f64| -> [u8; 4] {
        let c = |x: f64| (x + d).round().clamp(0.0, 255.0) as u8;
        [c(mean[0]), c(mean[1]), c(mean[2]), 255]
    };
    let (a, b) = (level(up), level(-down));
    Image {
        width: 16,
        height: 16,
        bgra: (0..N).flat_map(|i| if i < k { a } else { b }).collect(),
    }
}

#[test]
fn panels_shipped_table_hides_the_overlay_on_a_measured_sample_target() {
    let raw = fs::read_to_string(PANEL_FIXTURE).expect("the panel fixture");
    let data: serde_json::Value = serde_json::from_str(&raw).expect("JSON");
    let sample = data["samples"]
        .as_array()
        .expect("samples")
        .iter()
        .find(|s| {
            s["kind"] == "open" && s["entry"] == "inventory" && s["window"]["panel"] == "inventory"
        })
        .expect("an open sample of the inventory")
        .clone();
    let table = shipped_panels();
    let inventory = table
        .entries
        .iter()
        .find(|e| e.id == "inventory")
        .expect("the inventory's entry");
    let probes = sample["probes"]["inventory"].as_array().expect("its probes");
    assert_eq!(probes.len(), inventory.probes.len());
    let painted: Vec<(PixelRect, Image)> = inventory
        .probes
        .iter()
        .zip(probes)
        .map(|(p, s)| {
            let mean: Vec<f64> = s["mean_bgr"]
                .as_array()
                .expect("a mean")
                .iter()
                .map(|v| v.as_f64().expect("a number"))
                .collect();
            let std = s["luma_std"].as_f64().expect("a spread");
            (p.pixels(CLIENT), paint_sample([mean[0], mean[1], mean[2]], std))
        })
        .collect();

    let h = harness(Setup {
        shipped: true,
        ..Setup::default()
    });
    // Negative control first: the scene's grey everywhere, nothing hides.
    *h.scene.painted.lock().unwrap() = vec![(PixelRect::new(0, 0, 1, 1), paint_sample([91.0; 3], 0.0))];
    h.app.start();
    h.feed(&in_the_riverbank());
    h.in_front(Some(GAME));
    h.wait_diag(&["panels: overlay at x 0.78 to 1.00, y 0.30 to 0.97"]);
    wait_until("several copies", || h.scene.copies().len() >= 40);
    assert!(h.scene.window_calls().is_empty(), "{:?}", h.scene.window_calls());
    assert!(h.visibility_lines().is_empty(), "{:?}", h.visibility_lines());

    // The inventory's open sample under the overlay at its default place.
    *h.scene.painted.lock().unwrap() = painted;
    h.wait_diag(&["panels: inventory open"]);
    h.wait_diag(&[PANEL_LINE]);
    wait_until("the overlay hidden", || {
        h.scene.window_calls() == vec!["visible false"]
    });
    // The panel closes: the overlay shows again.
    *h.scene.painted.lock().unwrap() = vec![(PixelRect::new(0, 0, 1, 1), paint_sample([91.0; 3], 0.0))];
    h.wait_diag(&["panels: inventory closed"]);
    wait_until("the overlay shown", || {
        h.scene.window_calls() == vec!["visible false", "visible true"]
    });
    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));
    for l in h.panel_lines() {
        for n in ["mean", "bgr"] {
            assert!(!l.contains(n), "{l}");
        }
    }
}
