//! "Hide under game panels" in the shell (Context "Design: the panel
//! watcher"), driven by a fake backend over a synthetic panel table. The
//! fake paints a panel's frame colour into a copy when the copy is of one of
//! that panel's probes and the test has the panel showing, and grey
//! otherwise; the fake window keeps the overlay's shown state, so the
//! overlay the backend reports is hidden once the shell hides it. Nothing
//! here captures the screen or opens a window. Character names are
//! invented.
//!
//! Baseline: with `hide_under_panels` off no watcher starts, the backend is
//! never called, and the visibility lines are exactly those of an app with
//! no panel backend at all, with a panel showing in the fake's copies.
//! Target: with it on, a panel open under the overlay hides it, with its
//! line, and the overlay shows again once the panel closes.

use std::collections::BTreeSet;
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
    BackendError, Image, OcrReadiness, OverlayState, PanelEntry, PanelProbe, PanelTable,
    PixelLine, PixelRect, Rect, WindowHandle,
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
    showing: Mutex<BTreeSet<&'static str>>,
    capture_fails: AtomicBool,
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
            showing: Mutex::new(BTreeSet::new()),
            capture_fails: AtomicBool::new(false),
            calls: Mutex::new(Vec::new()),
        }
    }

    fn show(&self, id: &'static str) {
        self.showing.lock().unwrap().insert(id);
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
        let showing = self.0.showing.lock().unwrap();
        let painted = showing.iter().any(|id| which.starts_with(&format!("{id}#")));
        let c = if painted { FRAME } else { GREY };
        Ok(Image {
            width: 4,
            height: 4,
            bgra: [c[0], c[1], c[2], 255].repeat(16),
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
    /// `hide_under_panels` in the saved settings.
    on: bool,
    /// Whether the app gets a panel backend at all.
    backend: bool,
}

impl Default for Setup {
    fn default() -> Setup {
        Setup {
            on: true,
            backend: true,
        }
    }
}

fn harness(setup: Setup) -> Harness {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path().to_path_buf();
    Store::open(root.join("store"))
        .expect("store")
        .save_settings(&json!({ "hide_under_panels": setup.on }))
        .expect("settings written");
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
                table: Some(table()),
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

/// Whether `line` (from `panels: `) has one of the shapes the Context pins.
fn pinned(line: &str) -> bool {
    let Some(rest) = line.strip_prefix("panels: ") else {
        return false;
    };
    if ["on", "off", "resumed", "stopped after an error"].contains(&rest) {
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
        let words: Vec<&str> = r.split(' ').collect();
        let shape = [
            "#", "reads,", "#", "copies,", "#", "open,", "#", "black", "copies,", "busy", "#", "ms",
        ];
        return words.len() == shape.len()
            && words
                .iter()
                .zip(shape)
                .all(|(w, s)| if s == "#" { number(w) } else { *w == s });
    }
    ENTRY_IDS.iter().any(|id| {
        [" open", " closed", " unseen under the overlay"]
            .iter()
            .any(|s| rest == format!("{id}{s}"))
    })
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
        for n in PIXEL_NUMBERS {
            assert!(!l.contains(n), "{l:?} carries {n}");
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
fn panels_setting_off_starts_no_watcher_and_never_hides_baseline() {
    // The same session, on an app with the setting off and a panel showing
    // in the fake's copies, and on one with no panel backend at all.
    let run = |backend: bool| {
        let h = harness(Setup { on: false, backend });
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
        assert!(!h.app.get_view().settings.hide_under_panels);
        h
    };
    let h = run(true);
    assert!(h.scene.calls().is_empty(), "{:?}", h.scene.calls());
    assert!(h.panel_lines().is_empty(), "{:?}", h.panel_lines());
    assert_eq!(h.visibility_lines(), vec![HIDDEN_LINE, SHOWN_LINE]);
    assert_eq!(h.scene.window_calls(), vec!["visible false", "visible true"]);

    let bare = run(false);
    assert_eq!(bare.visibility_lines(), h.visibility_lines());
    assert_eq!(bare.scene.window_calls(), h.scene.window_calls());
    assert_eq!(bare.app.get_view(), h.app.get_view());

    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));
    assert!(bare.app.shutdown(SHUTDOWN_TIMEOUT));
    for x in [&h, &bare] {
        assert!(
            x.diag_text()
                .contains("stop: tailer and worker ended, progress saved"),
            "{}",
            x.diag_text()
        );
    }
    assert_pinned(&h);
}

#[test]
fn panels_panel_under_the_overlay_hides_it_target() {
    let h = harness(Setup::default());
    h.scene.show("right");
    h.app.start();
    assert!(h.app.get_view().settings.hide_under_panels);
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
fn panels_turned_off_while_hidden_shows_the_overlay() {
    let h = playing(Setup::default());
    h.scene.show("right");
    h.wait_diag(&[PANEL_LINE]);
    wait_until("the overlay hidden", || {
        h.scene.window_calls() == vec!["visible false"]
    });

    let view = h
        .app
        .update_settings(&json!({ "hide_under_panels": false }))
        .expect("the setting off");
    assert!(!view.settings.hide_under_panels);
    // Shown by the time the command returns, without waiting for a tick.
    assert_eq!(h.scene.window_calls(), vec!["visible false", "visible true"]);
    assert_eq!(h.visibility_lines(), vec![PANEL_LINE, SHOWN_LINE]);
    assert_eq!(h.panel_lines().last().map(String::as_str), Some("panels: off"));
    assert!(!h.app.panel_watcher_running());
    let copied = h.scene.copies().len();
    std::thread::sleep(Duration::from_millis(600));
    assert_eq!(h.scene.copies().len(), copied);
    assert_eq!(h.scene.window_calls(), vec!["visible false", "visible true"]);

    // A watcher that ran and was stopped leaves the stop line as it was.
    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));
    assert!(h
        .diag_text()
        .contains("stop: tailer and worker ended, progress saved"));
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

#[test]
fn panels_lines_use_only_the_pinned_shapes() {
    // One run through every line the watcher writes: on, the overlay's
    // place, open, closed, idle for each reason, resumed, each pause,
    // unseen, the minute, stopped after an error and off.
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

    h.app
        .update_settings(&json!({ "hide_under_panels": false }))
        .expect("the setting off");
    assert!(h.status().is_none());
    assert!(h.app.shutdown(SHUTDOWN_TIMEOUT));

    let lines = h.panel_lines();
    for shape in ["panels: on", "panels: off", "panels: resumed"] {
        assert!(lines.iter().any(|l| l == shape), "{shape}: {lines:#?}");
    }
    assert_pinned(&h);
}
