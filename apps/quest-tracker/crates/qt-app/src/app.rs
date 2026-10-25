//! The shell's state and threads: one [`App`] behind one lock, the worker
//! thread that drains the tailer into the engine, and the IPC command
//! handlers.
//!
//! Locking. Everything the view is built from lives in one `State` behind
//! one mutex: the worker takes it for each batch of log lines, a command for
//! its change. A fresh view is built and emitted while the lock is held, so
//! views reach the UI in the order the state changed. Calls the shell makes
//! to its window split in two: the ones Tauri only posts to its event loop
//! (click-through, zoom) run under the lock, keeping their order; the ones
//! that wait for the main thread (placing the window, which reads the
//! monitor; registering the shortcut; opening a link) run after the lock is
//! released, so a main-thread callback waiting for the lock can never hold
//! one of them up. Disk work (saves) runs outside the lock too, under its
//! own `save_lock`, which keeps the snapshots in order.
//!
//! Saves are debounced: a change marks the state dirty, and a save runs
//! when the last one is at least [`SAVE_INTERVAL_MS`] old, from the command
//! that made the change or from the worker's next tick, and once more on
//! [`App::shutdown`]. Each file is written only when its content differs
//! from what was last written.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use qt_engine::{Engine, QuestAction, QuestData, Transition};
use qt_logline::parse_line;
use qt_store::{Loaded, Store, StoreError};
use qt_tailer::{Position, SystemClock as TailerClock, TailEvent};
use qt_viewmodel::{
    build_view, Dock, Frame, LogState, Mode, Scale, Settings, StatusKind, StatusLine, Toast,
    ToastQueue, View, ViewKind,
};
use serde_json::{json, Map, Value};

use crate::cli::Options;
use crate::diag;
use crate::discover;
use crate::seed;

/// The shortest time between two saves.
pub const SAVE_INTERVAL_MS: u64 = 2_000;

/// How long the worker waits for a log event before its housekeeping tick.
pub const WORKER_TICK: Duration = Duration::from_millis(250);

/// The most tailer events the worker handles before it emits a view.
pub const BATCH_EVENTS: usize = 2_000;

/// How long [`App::shutdown`] waits for the threads by default.
pub const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);

/// Sends `qt:view` and `qt:toast` to the `overlay` window.
pub trait Emitter: Send + Sync + 'static {
    fn view(&self, view: &View);
    fn toast(&self, toast: &Toast);
}

/// Where the window goes: docked to a side of its monitor, or where the
/// user dragged it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    Docked(Dock),
    At { x: i32, y: i32 },
}

/// What the shell does to its window and the desktop. Tauri implements it
/// in `src-tauri`; tests use a fake that records the calls.
pub trait Window: Send + Sync + 'static {
    /// `on` is play mode: the mouse passes to the game and the window never
    /// takes focus from it. `off` is edit mode: the window takes clicks and
    /// keyboard focus, so the Settings fields (log path, shortcut) can be
    /// typed in; a window that cannot be activated never receives keys.
    fn set_click_through(&self, on: bool) -> Result<(), String>;
    /// The `scale` setting as a webview zoom factor (and window size).
    fn set_scale(&self, factor: f64) -> Result<(), String>;
    fn place(&self, placement: Placement) -> Result<(), String>;
    /// Registers `hotkey` as the one global shortcut, replacing any other.
    fn register_hotkey(&self, hotkey: &str) -> Result<(), String>;
    /// Opens `url` in the user's default browser.
    fn open_url(&self, url: &str) -> Result<(), String>;
    /// The monitor the window is on, for the `auto` scale; `None` when the
    /// window has no monitor.
    fn monitor(&self) -> Result<Option<MonitorMetrics>, String>;
}

/// What the `auto` scale reads from the window's monitor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MonitorMetrics {
    /// The monitor's height in physical pixels.
    pub height: u32,
    /// Windows' DPI scale for the monitor: 1 at 100%, 1.5 at 150%.
    pub dpi_scale: f64,
}

/// Milliseconds on a monotonic clock, for the debounce and the toasts.
pub trait Clock: Send + Sync + 'static {
    fn now_ms(&self) -> u64;
}

/// The real clock: milliseconds since it was made.
#[derive(Debug, Clone, Copy)]
pub struct SystemClock(Instant);

impl Default for SystemClock {
    fn default() -> SystemClock {
        SystemClock(Instant::now())
    }
}

impl Clock for SystemClock {
    fn now_ms(&self) -> u64 {
        self.0.elapsed().as_millis() as u64
    }
}

/// The webview zoom for a `scale` setting on `monitor`. `1`, `1.33` and `2`
/// are what they say. `auto` targets 1 below 1440 physical pixels of
/// monitor height, 1.33 from 1440 and 2 from 2160, divided by the monitor's
/// DPI scale (the webview already applies that), rounded to hundredths and
/// never below 1, so Auto never shrinks the panel under what Windows' own
/// DPI setting gives. With no monitor, `auto` is 1.
pub fn scale_factor(scale: Scale, monitor: Option<MonitorMetrics>) -> f64 {
    match scale {
        Scale::Auto => monitor.map_or(1.0, auto_zoom),
        Scale::X1 => 1.0,
        Scale::X133 => 1.33,
        Scale::X2 => 2.0,
    }
}

fn auto_zoom(m: MonitorMetrics) -> f64 {
    let target = if m.height >= 2160 {
        2.0
    } else if m.height >= 1440 {
        1.33
    } else {
        1.0
    };
    // A DPI scale that is not one (zero, negative, NaN) divides nothing out.
    let dpi = if m.dpi_scale.is_finite() && m.dpi_scale > 0.0 {
        m.dpi_scale
    } else {
        1.0
    };
    ((target / dpi * 100.0).round() / 100.0).max(1.0)
}

/// The status line's remedy when the shortcut fails at launch: with no
/// shortcut, play mode could not be left, so the overlay starts in edit mode.
const LAUNCH_REMEDY: &str =
    "the overlay stays in edit mode, where it takes clicks: set another shortcut in Settings";

fn mode_str(mode: Mode) -> &'static str {
    match mode {
        Mode::Play => "play",
        Mode::Edit => "edit",
    }
}

/// Everything [`App::new`] needs.
pub struct AppConfig {
    pub options: Options,
    /// Why the command line was refused, when it was (the options are then
    /// the defaults).
    pub options_error: Option<String>,
    /// The store, or why it could not be opened (progress is then not saved).
    pub store: Result<Store, String>,
    /// The quest data; `None` loads `options.data` or the data built in.
    pub data: Option<Arc<QuestData>>,
    /// The install locations log discovery tries.
    pub candidates: Vec<PathBuf>,
    /// The tailer's poll interval.
    pub poll_interval: Duration,
    pub emitter: Box<dyn Emitter>,
    pub window: Box<dyn Window>,
    pub clock: Box<dyn Clock>,
}

/// Who set a status line. The most severe one shows; between equals, the
/// first in this order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Source {
    Hotkey,
    Store,
    Save,
    Data,
    Options,
    Window,
    Log,
}

fn severity(kind: StatusKind) -> u8 {
    match kind {
        StatusKind::Error => 3,
        StatusKind::Warn => 2,
        StatusKind::Info => 1,
        StatusKind::Ok => 0,
    }
}

struct State {
    engine: Engine,
    settings: Settings,
    /// Where the user dragged the window, saved with the settings.
    window_pos: Option<(i32, i32)>,
    ui: qt_viewmodel::UiState,
    toasts: ToastQueue,
    statuses: BTreeMap<Source, StatusLine>,
    log_path: Option<PathBuf>,
    position: Option<Position>,
    /// Something may differ from what was last written.
    dirty: bool,
    last_save_ms: Option<u64>,
    saved_settings: Option<Value>,
    saved_position: Option<Value>,
    saved_roster: Option<Value>,
    saved_chars: BTreeMap<String, Value>,
    last_view: Option<View>,
}

impl State {
    fn settings_json(&self) -> Value {
        let mut v = serde_json::to_value(&self.settings).expect("settings serialise");
        if let (Some((x, y)), Some(obj)) = (self.window_pos, v.as_object_mut()) {
            obj.insert("window".to_string(), json!({ "x": x, "y": y }));
        }
        v
    }

    fn roster_json(&self) -> Value {
        let mut roster = Map::new();
        for c in self.engine.characters() {
            roster.insert(
                c.name.clone(),
                json!({ "class": c.class, "level": c.level, "own": c.own, "pinned": c.pinned }),
            );
        }
        Value::Object(roster)
    }

    fn set_status(&mut self, source: Source, line: Option<StatusLine>) {
        match line {
            Some(l) => {
                self.statuses.insert(source, l);
            }
            None => {
                self.statuses.remove(&source);
            }
        }
        self.ui.status = self
            .statuses
            .iter()
            .max_by(|(sa, a), (sb, b)| {
                severity(a.kind)
                    .cmp(&severity(b.kind))
                    .then_with(|| sb.cmp(sa))
            })
            .map(|(_, l)| l.clone());
    }

    fn placement(&self) -> Placement {
        match self.window_pos {
            Some((x, y)) => Placement::At { x, y },
            None => Placement::Docked(self.settings.dock),
        }
    }
}

/// What a command leaves to do once the lock is released.
#[derive(Default)]
struct After {
    place: Option<Placement>,
    hotkey: Option<String>,
    retail: bool,
}

struct Tailer {
    stop: Arc<AtomicBool>,
    handle: JoinHandle<()>,
}

/// The shell: state, threads and commands. Shared as `Arc<App>`.
pub struct App {
    options: Options,
    store: Option<Store>,
    emitter: Box<dyn Emitter>,
    window: Box<dyn Window>,
    clock: Box<dyn Clock>,
    candidates: Vec<PathBuf>,
    poll_interval: Duration,
    state: Mutex<State>,
    save_lock: Mutex<()>,
    tx: Sender<TailEvent>,
    rx: Mutex<Option<Receiver<TailEvent>>>,
    tailer: Mutex<Option<Tailer>>,
    worker: Mutex<Option<JoinHandle<()>>>,
    worker_stop: Arc<AtomicBool>,
    stopped: AtomicBool,
    saves: AtomicU64,
}

impl App {
    /// Loads the settings, the characters and the saved position, and
    /// decides which log to follow. Starts nothing: see [`App::start`].
    pub fn new(config: AppConfig) -> Arc<App> {
        let AppConfig {
            options,
            options_error,
            store,
            data,
            candidates,
            poll_interval,
            emitter,
            window,
            clock,
        } = config;
        let mut statuses: Vec<(Source, StatusLine)> = Vec::new();
        if let Some(e) = &options_error {
            statuses.push((Source::Options, StatusLine::warn(format!("Command line: {e}"))));
        }
        let data = match data {
            Some(d) => d,
            None => {
                let (d, note) = crate::load_data(options.data.as_deref());
                if let Some(n) = note {
                    statuses.push((Source::Data, StatusLine::error(n)));
                }
                d
            }
        };
        let (store, store_error) = match store {
            Ok(s) => (Some(s), None),
            Err(e) => (None, Some(e)),
        };
        if let Some(e) = &store_error {
            statuses.push((
                Source::Store,
                StatusLine::error(format!("Progress is not saved: {e}")),
            ));
        }

        let mut store_note: Option<StatusLine> = None;
        let load = |note: &mut Option<StatusLine>, what: &str, r: Result<Loaded, StoreError>| {
            match r {
                Ok(l) => {
                    if let Some(q) = &l.quarantined {
                        *note = Some(StatusLine::warn(q.status_text()));
                    }
                    l.value
                }
                Err(e) => {
                    *note = Some(StatusLine::error(format!("Cannot read {what}: {e}")));
                    None
                }
            }
        };

        let mut settings = Settings::default();
        let mut window_pos = None;
        let mut engine = Engine::new(data);
        let mut position = None;
        // Which files exist, so a first run writes its settings and roster.
        let (mut had_settings, mut had_roster) = (false, false);
        if let Some(s) = &store {
            if let Some(v) = load(&mut store_note, "the settings", s.load_settings()) {
                had_settings = true;
                match serde_json::from_value::<Settings>(v.clone()) {
                    Ok(loaded) => settings = loaded.normalised(),
                    Err(e) => {
                        store_note = Some(StatusLine::warn(format!(
                            "The settings could not be used ({e}); defaults are in use"
                        )))
                    }
                }
                window_pos = v.get("window").and_then(|w| {
                    let x = w.get("x")?.as_i64()?;
                    let y = w.get("y")?.as_i64()?;
                    Some((i32::try_from(x).ok()?, i32::try_from(y).ok()?))
                });
            }
            if let Some(Value::Object(roster)) =
                load(&mut store_note, "the character list", s.load_characters())
            {
                had_roster = true;
                let mut pinned = None;
                for (name, entry) in &roster {
                    if entry.get("pinned").and_then(Value::as_bool) == Some(true) {
                        pinned = Some(name.clone());
                    }
                    if let Some(v) = load(&mut store_note, "a character", s.load_character(name)) {
                        if let Err(e) = engine.load_character(&v) {
                            store_note = Some(StatusLine::warn(format!(
                                "A saved character could not be read: {e}"
                            )));
                        }
                    }
                }
                if let Some(p) = pinned {
                    let _ = engine.pin(Some(&p));
                }
            }
            if let Some(v) = load(&mut store_note, "the log position", s.load_position()) {
                position = serde_json::from_value::<Position>(v).ok();
            }
        }
        if let Some(n) = store_note {
            statuses.push((Source::Store, n));
        }

        let chosen = discover::log_path_on_disk(
            options.log.as_deref(),
            settings.log_path.as_deref(),
            &candidates,
        );
        let mut ui = qt_viewmodel::UiState::default();
        let log_path = match chosen {
            Some((p, source)) => {
                ui.log = LogState::Waiting;
                statuses.push((
                    Source::Log,
                    StatusLine::info("Waiting for the game to write to the log."),
                ));
                Some((std::path::absolute(&p).unwrap_or(p), source))
            }
            None => {
                ui.log = LogState::Missing;
                None
            }
        };

        let (tx, rx) = mpsc::channel();
        let mut state = State {
            engine,
            settings,
            window_pos,
            ui,
            toasts: ToastQueue::new(),
            statuses: BTreeMap::new(),
            log_path: log_path.as_ref().map(|(p, _)| p.clone()),
            position,
            dirty: false,
            last_save_ms: None,
            saved_settings: None,
            saved_position: None,
            saved_roster: None,
            saved_chars: BTreeMap::new(),
            last_view: None,
        };
        // What was just loaded is what is on disk; a first run starts dirty.
        state.dirty = !(had_settings && had_roster);
        if store.is_some() {
            if had_settings {
                state.saved_settings = Some(state.settings_json());
            }
            if had_roster {
                state.saved_roster = Some(state.roster_json());
            }
            for c in state.engine.characters() {
                if let Some(v) = state.engine.character_json(&c.name) {
                    state.saved_chars.insert(c.name, v);
                }
            }
            state.saved_position = state
                .position
                .as_ref()
                .and_then(|p| serde_json::to_value(p).ok());
        }
        for (source, line) in statuses {
            state.set_status(source, Some(line));
        }

        let app = Arc::new(App {
            options,
            store,
            emitter,
            window,
            clock,
            candidates,
            poll_interval,
            state: Mutex::new(state),
            save_lock: Mutex::new(()),
            tx,
            rx: Mutex::new(Some(rx)),
            tailer: Mutex::new(None),
            worker: Mutex::new(None),
            worker_stop: Arc::new(AtomicBool::new(false)),
            stopped: AtomicBool::new(false),
            saves: AtomicU64::new(0),
        });
        if let Some(e) = &options_error {
            app.diag(&format!("command line refused: {e}"));
        }
        if let Some(e) = &store_error {
            app.diag(&format!("store unavailable: {e}"));
        }
        app
    }

    /// Applies the window state, registers the shortcut, starts the tailer
    /// and the worker, and announces the first view.
    pub fn start(self: &Arc<Self>) {
        let (mode, scale, placement, hotkey, log) = {
            let st = self.lock();
            (
                st.ui.mode,
                st.settings.scale,
                st.placement(),
                st.settings.hotkey.clone(),
                st.log_path.clone(),
            )
        };
        let source = discover::log_path_on_disk(
            self.options.log.as_deref(),
            self.lock().settings.log_path.as_deref(),
            &self.candidates,
        )
        .map(|(_, s)| s);
        self.diag(&format!(
            "start: quest-tracker {}, store {}, {} quests, log {}, mode {}",
            env!("CARGO_PKG_VERSION"),
            self.store
                .as_ref()
                .map(|s| s.dir().display().to_string())
                .unwrap_or_else(|| "none".to_string()),
            self.lock().engine.data().quests.len(),
            match (&log, source) {
                (Some(p), Some(s)) => format!("{} (from {})", p.display(), s.as_str()),
                (Some(p), None) => p.display().to_string(),
                _ => "none found".to_string(),
            },
            mode_str(mode),
        ));
        // The shortcut is registered before the mode is applied: it is the
        // only way out of play mode, so without one the overlay starts in
        // edit mode, where it takes clicks and Settings can be reached.
        let mode = if self.register_hotkey(&hotkey, LAUNCH_REMEDY) || mode == Mode::Edit {
            mode
        } else {
            let settings = self
                .store
                .as_ref()
                .map(|s| s.dir().join("settings.json").display().to_string())
                .unwrap_or_else(|| "settings.json".to_string());
            self.diag(&format!(
                "mode edit: no shortcut to leave play mode; set one in Settings or as \"hotkey\" in {settings}"
            ));
            self.lock().ui.mode = Mode::Edit;
            Mode::Edit
        };
        if let Err(e) = self.window.set_click_through(mode == Mode::Play) {
            self.window_error("click-through", &e);
        }
        let monitor = if scale == Scale::Auto {
            self.read_monitor()
        } else {
            None
        };
        if let Err(e) = self.window.set_scale(self.zoom(scale, monitor)) {
            self.window_error("scale", &e);
        }
        if let Err(e) = self.window.place(placement) {
            self.window_error("placement", &e);
        }
        self.start_tailer();
        self.spawn_worker();
        let mut st = self.lock();
        self.announce(&mut st);
    }

    /// Stops the tailer and the worker, waits for both up to `timeout`, and
    /// saves whatever is unsaved. True when both threads ended in time.
    pub fn shutdown(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        self.worker_stop.store(true, Ordering::Release);
        let tailer = self.lock_tailer().take();
        if let Some(t) = &tailer {
            t.stop.store(true, Ordering::Release);
        }
        let worker = self
            .worker
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .take();
        let tailer_ok = tailer.map_or(true, |t| join_by(t.handle, deadline));
        let worker_ok = worker.map_or(true, |w| join_by(w, deadline));
        self.save_if_due(true);
        if !self.stopped.swap(true, Ordering::AcqRel) {
            self.diag(&if tailer_ok && worker_ok {
                "stop: tailer and worker ended, progress saved".to_string()
            } else {
                format!(
                    "stop: tailer {}, worker {} within {} ms; progress saved",
                    if tailer_ok { "ended" } else { "did not end" },
                    if worker_ok { "ended" } else { "did not end" },
                    timeout.as_millis()
                )
            });
        }
        tailer_ok && worker_ok
    }

    /// True while the worker thread runs.
    pub fn worker_running(&self) -> bool {
        self.worker
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .as_ref()
            .is_some_and(|h| !h.is_finished())
    }

    /// How many saves wrote at least one file.
    pub fn save_count(&self) -> u64 {
        self.saves.load(Ordering::Acquire)
    }

    pub fn store(&self) -> Option<&Store> {
        self.store.as_ref()
    }

    pub fn data(&self) -> Arc<QuestData> {
        self.lock().engine.data().clone()
    }

    /// The log being followed, if any.
    pub fn log_path(&self) -> Option<PathBuf> {
        self.lock().log_path.clone()
    }

    /// Writes one line to the diagnostic log. The shell passes only its own
    /// text here (the window handle, a second launch), never a log line.
    pub fn diag(&self, message: &str) {
        if let Some(s) = &self.store {
            let _ = s.diag().write(message);
        }
    }

    /// The window's monitor, for the `auto` scale. Called with no lock held:
    /// Tauri answers it on the main thread. A failed read is logged and
    /// counts as no monitor.
    fn read_monitor(&self) -> Option<MonitorMetrics> {
        match self.window.monitor() {
            Ok(m) => m,
            Err(e) => {
                self.diag(&format!("error: window monitor: {e}"));
                None
            }
        }
    }

    /// The zoom for `scale` on `monitor`; logs what `auto` chose from.
    fn zoom(&self, scale: Scale, monitor: Option<MonitorMetrics>) -> f64 {
        let zoom = scale_factor(scale, monitor);
        if scale == Scale::Auto {
            self.diag(&match monitor {
                Some(m) => format!(
                    "scale auto: monitor {} px high at DPI scale {} -> zoom {zoom}",
                    m.height, m.dpi_scale
                ),
                None => format!("scale auto: no monitor -> zoom {zoom}"),
            });
        }
        zoom
    }

    // -- the worker ----------------------------------------------------------

    fn spawn_worker(self: &Arc<Self>) {
        let Some(rx) = self.rx.lock().unwrap_or_else(|p| p.into_inner()).take() else {
            return;
        };
        let app = Arc::clone(self);
        let stop = Arc::clone(&self.worker_stop);
        let spawned = thread::Builder::new()
            .name("qt-worker".to_string())
            .spawn(move || run_worker(app, rx, stop));
        match spawned {
            Ok(h) => *self.worker.lock().unwrap_or_else(|p| p.into_inner()) = Some(h),
            Err(e) => self.diag(&format!("error: the worker thread did not start: {e}")),
        }
    }

    /// One batch of tailer events, as the worker hands them over: each line
    /// is parsed with the data's allowlist and fed to the engine, the
    /// position kept, and then one fresh view is emitted (when it differs
    /// from the last) followed by any toasts the live transitions made.
    pub fn handle_tail_events(&self, events: Vec<TailEvent>) {
        let now = self.clock.now_ms();
        let mut st = self.lock();
        let path = st.log_path.clone().unwrap_or_default();
        let mut transitions: Vec<Transition> = Vec::new();
        let mut caught_up = 0usize;
        for event in events {
            match event {
                TailEvent::Line(line) => {
                    if st.ui.log != LogState::Reading {
                        st.ui.log = LogState::Reading;
                        st.set_status(Source::Log, None);
                    }
                    let live = !line.catch_up;
                    if !live {
                        caught_up += 1;
                    }
                    if let Some(ev) = parse_line(&line.text, st.engine.allowlist()) {
                        if live {
                            if let Some(m) = diag::event(&ev) {
                                self.diag(&m);
                            }
                        }
                        let before = st.engine.current().map(str::to_string);
                        let ts = st.engine.apply(&ev, live);
                        for quest_id in st.engine.take_refused() {
                            self.diag(&diag::refused(&quest_id));
                        }
                        let after = st.engine.current().map(str::to_string);
                        if live && before != after {
                            self.diag(&diag::decision(
                                &st.engine,
                                before.as_deref(),
                                after.as_deref(),
                            ));
                        }
                        for t in &ts {
                            self.diag(&diag::transition(t));
                        }
                        transitions.extend(ts);
                    }
                    st.position = Some(line.position(&path));
                    st.dirty = true;
                }
                TailEvent::Following { offset, reason } => {
                    if st.ui.log == LogState::Missing {
                        st.ui.log = LogState::Waiting;
                        st.set_status(
                            Source::Log,
                            Some(StatusLine::info("Waiting for the game to write to the log.")),
                        );
                    }
                    self.diag(&format!(
                        "log: following {} from offset {offset} ({})",
                        path.display(),
                        diag::reason(reason)
                    ));
                }
                TailEvent::Waiting => {
                    st.ui.log = LogState::Missing;
                    st.set_status(Source::Log, None);
                    self.diag(&format!("log: waiting for {} to exist", path.display()));
                }
                TailEvent::Error(message) => {
                    st.set_status(
                        Source::Log,
                        Some(StatusLine::warn(format!("Reading the log failed: {message}"))),
                    );
                    self.diag(&format!("error: log: {message}"));
                }
            }
        }
        if caught_up > 0 {
            let current = st.engine.current().map(str::to_string);
            self.diag(&format!(
                "catch-up: {caught_up} lines read, character {}",
                diag::character(&st.engine, current.as_deref())
            ));
        }
        let data = st.engine.data().clone();
        let toasts = st.toasts.push(&data, &transitions, now);
        let view = self.build(&st);
        if st.last_view.as_ref() != Some(&view) {
            self.emitter.view(&view);
            st.last_view = Some(view);
        }
        for t in &toasts {
            self.emitter.toast(t);
            self.diag(&diag::toast(t));
        }
    }

    /// The worker's housekeeping between batches: a save when one is due.
    pub fn tick(&self) {
        self.save_if_due(false);
    }

    // -- the commands --------------------------------------------------------

    /// `get_view`: the view as it stands, from memory.
    pub fn get_view(&self) -> View {
        let st = self.lock();
        self.build(&st)
    }

    /// `set_mode`: play is click-through, edit takes clicks.
    pub fn set_mode(&self, mode: Mode) -> Result<View, String> {
        self.command("set_mode", |app, st| {
            app.apply_mode(st, mode);
            Ok(After::default())
        })
    }

    /// The global shortcut: play <-> edit.
    pub fn toggle_mode(&self) -> Result<View, String> {
        self.command("shortcut", |app, st| {
            let mode = match st.ui.mode {
                Mode::Play => Mode::Edit,
                Mode::Edit => Mode::Play,
            };
            app.diag(&format!("shortcut: mode {}", mode_str(mode)));
            app.apply_mode(st, mode);
            Ok(After::default())
        })
    }

    pub fn set_view_kind(&self, kind: ViewKind) -> Result<View, String> {
        self.command("set_view_kind", |_, st| {
            st.ui.view_kind = kind;
            Ok(After::default())
        })
    }

    pub fn set_collapsed(&self, collapsed: bool) -> Result<View, String> {
        self.command("set_collapsed", |_, st| {
            st.ui.collapsed = collapsed;
            Ok(After::default())
        })
    }

    pub fn toggle_act(&self, act: u32) -> Result<View, String> {
        self.command("toggle_act", |_, st| {
            st.ui.toggle_act(act);
            Ok(After::default())
        })
    }

    pub fn open_detail(&self, quest_id: Option<String>) -> Result<View, String> {
        self.command("open_detail", |_, st| {
            if let Some(id) = &quest_id {
                if st.engine.data().quest(id).is_none() {
                    return Err(format!("no quest {id:?}"));
                }
            }
            st.ui.detail = quest_id;
            Ok(After::default())
        })
    }

    /// `open_frame`: `character-picker`, `settings`, or `None` to close.
    pub fn open_frame(&self, frame: Option<Frame>) -> Result<View, String> {
        self.command("open_frame", |_, st| {
            st.ui.open_frame(frame)?;
            Ok(After::default())
        })
    }

    pub fn quest_action(&self, quest_id: &str, action: QuestAction) -> Result<View, String> {
        self.command("quest_action", |app, st| {
            let name = current(st)?;
            let ts = st
                .engine
                .quest_action(&name, quest_id, action)
                .map_err(|e| e.to_string())?;
            app.engine_changed(st, &ts);
            Ok(After::default())
        })
    }

    pub fn tick_step(&self, quest_id: &str, step_id: &str, ticked: bool) -> Result<View, String> {
        self.command("tick_step", |app, st| {
            let name = current(st)?;
            let ts = st
                .engine
                .tick_step(&name, quest_id, step_id, ticked)
                .map_err(|e| e.to_string())?;
            app.engine_changed(st, &ts);
            Ok(After::default())
        })
    }

    /// `select_character`: the picker's choice. The character is pinned (it
    /// gets every event until unpinned) and the picker closes.
    pub fn select_character(&self, name: &str) -> Result<View, String> {
        self.command("select_character", |app, st| {
            app.pin(st, Some(name))?;
            st.ui.open_frame = None;
            Ok(After::default())
        })
    }

    /// `pin_character`: pins a character, or with `None` unpins, and leaves
    /// the open frame as it is.
    pub fn pin_character(&self, name: Option<&str>) -> Result<View, String> {
        self.command("pin_character", |app, st| {
            app.pin(st, name)?;
            Ok(After::default())
        })
    }

    pub fn confirm_character(&self, name: &str) -> Result<View, String> {
        self.command("confirm_character", |app, st| {
            let ts = st.engine.confirm(name).map_err(|e| e.to_string())?;
            app.diag(&format!(
                "character confirmed as own: {}",
                diag::character(&st.engine, Some(name))
            ));
            app.engine_changed(st, &ts);
            Ok(After::default())
        })
    }

    /// `update_settings`: applies a patch of settings fields. The scale goes
    /// to the window at once; a new dock side, shortcut or log path is
    /// applied after the lock is released. A patch that names the scale has
    /// the monitor read first, before the lock, for `auto`.
    pub fn update_settings(&self, patch: &Value) -> Result<View, String> {
        let monitor = if patch.get("scale").is_some() {
            self.read_monitor()
        } else {
            None
        };
        self.command("update_settings", |app, st| {
            let new = st.settings.patched(patch)?;
            let old = std::mem::replace(&mut st.settings, new.clone());
            st.dirty = true;
            let mut after = After::default();
            if new.scale != old.scale {
                if let Err(e) = app.window.set_scale(app.zoom(new.scale, monitor)) {
                    app.diag(&format!("error: window scale: {e}"));
                }
            }
            if new.dock != old.dock {
                st.window_pos = None;
                after.place = Some(Placement::Docked(new.dock));
            }
            if new.hotkey != old.hotkey {
                after.hotkey = Some(new.hotkey.clone());
            }
            if new.log_path != old.log_path {
                after.retail = true;
            }
            Ok(after)
        })
    }

    /// `open_source`: opens the quest's poe2db page in the default browser.
    /// The app makes no request itself.
    pub fn open_source(&self, quest_id: &str) -> Result<(), String> {
        let url = {
            let st = self.lock();
            st.engine
                .data()
                .quest(quest_id)
                .map(|q| q.source_url.clone())
                .ok_or_else(|| format!("no quest {quest_id:?}"))?
        };
        self.window.open_url(&url).map_err(|e| {
            self.diag(&format!("error: opening the source of {quest_id}: {e}"));
            format!("could not open the quest's page: {e}")
        })?;
        self.diag(&format!("opened the source page of {quest_id}"));
        Ok(())
    }

    /// The window was moved. Only a move in edit mode (a drag) is kept.
    pub fn window_moved(&self, x: i32, y: i32) {
        {
            let mut st = self.lock();
            if st.ui.mode != Mode::Edit || st.window_pos == Some((x, y)) {
                return;
            }
            st.window_pos = Some((x, y));
            st.dirty = true;
        }
        self.save_if_due(false);
    }

    /// A second launch was refused by the single-instance guard.
    pub fn second_instance(&self) {
        self.diag("a second launch was refused; this instance keeps running");
    }

    // -- internals -----------------------------------------------------------

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|p| p.into_inner())
    }

    fn lock_tailer(&self) -> MutexGuard<'_, Option<Tailer>> {
        self.tailer.lock().unwrap_or_else(|p| p.into_inner())
    }

    fn build(&self, st: &State) -> View {
        build_view(&st.engine, &st.settings, &st.ui)
    }

    /// Builds and emits the view (always: a command announces its change).
    fn announce(&self, st: &mut State) -> View {
        let view = self.build(st);
        self.emitter.view(&view);
        st.last_view = Some(view.clone());
        view
    }

    /// Runs one command: the change under the lock, the view announced
    /// before the lock is released, then a save if one is due and the
    /// effects that wait for the main thread. A refused command changes
    /// nothing, announces nothing and is logged.
    fn command(
        &self,
        name: &str,
        f: impl FnOnce(&App, &mut State) -> Result<After, String>,
    ) -> Result<View, String> {
        let (mut view, after) = {
            let mut st = self.lock();
            match f(self, &mut st) {
                Ok(after) => (self.announce(&mut st), after),
                Err(e) => {
                    drop(st);
                    self.diag(&format!("command {name} refused: {e}"));
                    return Err(e);
                }
            }
        };
        self.save_if_due(false);
        let again = after.hotkey.is_some() || after.retail || after.place.is_some();
        if let Some(p) = after.place {
            if let Err(e) = self.window.place(p) {
                self.window_error("placement", &e);
            }
        }
        if let Some(h) = after.hotkey {
            self.register_hotkey(&h, "change it in Settings");
        }
        if after.retail {
            self.start_tailer();
        }
        if again {
            let mut st = self.lock();
            view = self.announce(&mut st);
        }
        Ok(view)
    }

    fn apply_mode(&self, st: &mut State, mode: Mode) {
        st.ui.mode = mode;
        match self.window.set_click_through(mode == Mode::Play) {
            Ok(()) => st.set_status(Source::Window, None),
            Err(e) => {
                self.diag(&format!("error: click-through: {e}"));
                st.set_status(
                    Source::Window,
                    Some(StatusLine::error(format!("Click-through could not be set: {e}"))),
                );
            }
        }
        self.diag(&format!("mode {}", mode_str(mode)));
    }

    fn pin(&self, st: &mut State, name: Option<&str>) -> Result<(), String> {
        let before = st.engine.current().map(str::to_string);
        let ts = st.engine.pin(name).map_err(|e| e.to_string())?;
        let after = st.engine.current().map(str::to_string);
        if before != after {
            self.diag(&diag::decision(&st.engine, before.as_deref(), after.as_deref()));
        }
        self.engine_changed(st, &ts);
        Ok(())
    }

    fn engine_changed(&self, st: &mut State, ts: &[Transition]) {
        for t in ts {
            self.diag(&diag::transition(t));
        }
        st.dirty = true;
    }

    fn window_error(&self, what: &str, e: &str) {
        self.diag(&format!("error: window {what}: {e}"));
        let mut st = self.lock();
        st.set_status(
            Source::Window,
            Some(StatusLine::error(format!("The window {what} could not be set: {e}"))),
        );
    }

    /// Registers the shortcut; a failure shows in the status line, ending
    /// with `remedy`, and in the diagnostic log until a later registration
    /// succeeds. True when it registered.
    fn register_hotkey(&self, hotkey: &str, remedy: &str) -> bool {
        let result = self.window.register_hotkey(hotkey);
        let mut st = self.lock();
        match result {
            Ok(()) => {
                self.diag(&format!("shortcut {hotkey} registered"));
                st.set_status(Source::Hotkey, None);
                true
            }
            Err(e) => {
                self.diag(&format!("error: shortcut {hotkey} could not be registered: {e}"));
                st.set_status(
                    Source::Hotkey,
                    Some(StatusLine::error(format!(
                        "The shortcut {hotkey} could not be registered ({e}); {remedy}."
                    ))),
                );
                false
            }
        }
    }

    /// Starts the tailer on the log the option, setting or discovery names,
    /// first stopping one that follows another path. With no log, none runs
    /// and the view shows `empty-no-log`.
    fn start_tailer(&self) {
        let (chosen, saved) = {
            let st = self.lock();
            (
                discover::log_path_on_disk(
                    self.options.log.as_deref(),
                    st.settings.log_path.as_deref(),
                    &self.candidates,
                ),
                st.position.clone(),
            )
        };
        let chosen = chosen.map(|(p, s)| (std::path::absolute(&p).unwrap_or(p), s));
        let mut slot = self.lock_tailer();
        let running_same = slot.is_some() && self.lock().log_path == chosen.as_ref().map(|c| c.0.clone());
        if running_same {
            return;
        }
        if let Some(old) = slot.take() {
            old.stop.store(true, Ordering::Release);
            let _ = join_by(old.handle, Instant::now() + SHUTDOWN_TIMEOUT);
        }
        let Some((path, source)) = chosen else {
            let mut st = self.lock();
            st.log_path = None;
            st.ui.log = LogState::Missing;
            st.set_status(Source::Log, None);
            drop(st);
            self.diag("log: no Client.txt found; set the log path in Settings");
            return;
        };
        {
            let mut st = self.lock();
            if st.log_path.as_deref() != Some(path.as_path()) || st.ui.log == LogState::Missing {
                st.ui.log = LogState::Waiting;
                st.set_status(
                    Source::Log,
                    Some(StatusLine::info("Waiting for the game to write to the log.")),
                );
            }
            st.log_path = Some(path.clone());
        }
        let config = qt_tailer::Config {
            path: path.clone(),
            saved: saved.filter(|p| p.path == path),
            from_start: self.options.from_start,
            poll_interval: self.poll_interval,
        };
        self.seed_area(&config);
        let stop = Arc::new(AtomicBool::new(false));
        match qt_tailer::spawn(config, TailerClock, self.tx.clone(), Arc::clone(&stop)) {
            Ok(handle) => {
                self.diag(&format!(
                    "log: tailer started on {} (from {}{})",
                    path.display(),
                    source.as_str(),
                    if self.options.from_start { ", --from-start" } else { "" }
                ));
                *slot = Some(Tailer { stop, handle });
            }
            Err(e) => {
                self.diag(&format!("error: the tailer thread did not start: {e}"));
                let mut st = self.lock();
                st.set_status(
                    Source::Log,
                    Some(StatusLine::error(format!("The log reader did not start: {e}"))),
                );
            }
        }
    }

    /// Seeds the engine's area from the log before where a tailer started
    /// with `config` begins reading (the tailer's own decision,
    /// [`qt_tailer::start_position`]), before the tailer exists, so before
    /// any of its lines reaches the engine. A start at 0, or a log not there
    /// yet, seeds nothing: the tailer reads the area lines itself. A read
    /// that fails leaves the area unknown.
    fn seed_area(&self, config: &qt_tailer::Config) {
        let start = match qt_tailer::start_position(config) {
            Ok(Some(start)) if start.offset > 0 => start,
            Ok(Some(_)) => {
                self.diag("area from log: not read (the tailer reads the log from offset 0)");
                return;
            }
            Ok(None) => {
                self.diag("area from log: not read (the log does not exist yet)");
                return;
            }
            Err(e) => {
                self.diag(&format!("error: area from log: {e}; the area is unknown"));
                self.lock().engine.seed_area(None);
                return;
            }
        };
        let found = seed::area_before(&config.path, start.offset);
        let mut st = self.lock();
        match found {
            Ok(found) => {
                st.engine.seed_area(found.code());
                self.diag(&diag::seeded_area(&found, start.offset));
            }
            Err(e) => {
                st.engine.seed_area(None);
                self.diag(&format!("error: area from log: {e}; the area is unknown"));
            }
        }
    }

    /// Saves what changed, when the last save is old enough (or `force`).
    /// Returns whether a file was written.
    fn save_if_due(&self, force: bool) -> bool {
        let Some(store) = &self.store else {
            return false;
        };
        let _order = self.save_lock.lock().unwrap_or_else(|p| p.into_inner());
        let now = self.clock.now_ms();
        let mut writes: Vec<(Write, Value)> = Vec::new();
        {
            let mut st = self.lock();
            if !st.dirty {
                return false;
            }
            if !force
                && st
                    .last_save_ms
                    .is_some_and(|t| now.saturating_sub(t) < SAVE_INTERVAL_MS)
            {
                return false;
            }
            st.dirty = false;
            let settings = st.settings_json();
            if st.saved_settings.as_ref() != Some(&settings) {
                st.saved_settings = Some(settings.clone());
                writes.push((Write::Settings, settings));
            }
            if let Some(pos) = st.position.as_ref().and_then(|p| serde_json::to_value(p).ok()) {
                if st.saved_position.as_ref() != Some(&pos) {
                    st.saved_position = Some(pos.clone());
                    writes.push((Write::Position, pos));
                }
            }
            for c in st.engine.characters() {
                if let Some(v) = st.engine.character_json(&c.name) {
                    if st.saved_chars.get(&c.name) != Some(&v) {
                        st.saved_chars.insert(c.name.clone(), v.clone());
                        writes.push((Write::Character(c.name), v));
                    }
                }
            }
            let roster = st.roster_json();
            if st.saved_roster.as_ref() != Some(&roster) {
                st.saved_roster = Some(roster.clone());
                writes.push((Write::Roster, roster));
            }
            if !writes.is_empty() {
                st.last_save_ms = Some(now);
            }
        }
        if writes.is_empty() {
            return false;
        }
        let mut failed: Vec<(Write, String)> = Vec::new();
        for (what, value) in &writes {
            let r = match what {
                Write::Settings => store.save_settings(value),
                Write::Position => store.save_position(value),
                Write::Roster => store.save_characters(value),
                Write::Character(name) => store.save_character(name, value),
            };
            if let Err(e) = r {
                failed.push((what.clone(), e.to_string()));
            }
        }
        let mut st = self.lock();
        if failed.is_empty() {
            if st.statuses.contains_key(&Source::Save) {
                st.set_status(Source::Save, None);
                self.announce(&mut st);
            }
        } else {
            // Forget what did not reach the disk, so the next save retries.
            for (what, e) in &failed {
                self.diag(&format!("error: save: {e}"));
                match what {
                    Write::Settings => st.saved_settings = None,
                    Write::Position => st.saved_position = None,
                    Write::Roster => st.saved_roster = None,
                    Write::Character(n) => {
                        st.saved_chars.remove(n);
                    }
                }
            }
            st.dirty = true;
            st.set_status(
                Source::Save,
                Some(StatusLine::error(format!(
                    "Progress could not be saved: {}",
                    failed[0].1
                ))),
            );
            self.announce(&mut st);
        }
        drop(st);
        self.saves.fetch_add(1, Ordering::AcqRel);
        true
    }
}

#[derive(Debug, Clone)]
enum Write {
    Settings,
    Position,
    Roster,
    Character(String),
}

/// The character a quest command applies to: the pinned one, else the
/// active one.
fn current(st: &State) -> Result<String, String> {
    st.engine
        .current()
        .map(str::to_string)
        .ok_or_else(|| "no character yet: play until one is detected, or pick one".to_string())
}

/// Joins `handle` if it ends before `deadline`.
fn join_by(handle: JoinHandle<()>, deadline: Instant) -> bool {
    while !handle.is_finished() {
        if Instant::now() >= deadline {
            return false;
        }
        thread::sleep(Duration::from_millis(5));
    }
    handle.join().is_ok()
}

/// The worker: drains the tailer's channel in batches into
/// [`App::handle_tail_events`], and ticks between them. Ends within one
/// [`WORKER_TICK`] of `stop` being set.
fn run_worker(app: Arc<App>, rx: Receiver<TailEvent>, stop: Arc<AtomicBool>) {
    while !stop.load(Ordering::Acquire) {
        match rx.recv_timeout(WORKER_TICK) {
            Ok(first) => {
                let mut batch = vec![first];
                while batch.len() < BATCH_EVENTS {
                    match rx.try_recv() {
                        Ok(e) => batch.push(e),
                        Err(_) => break,
                    }
                }
                app.handle_tail_events(batch);
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
        app.tick();
    }
}


#[cfg(test)]
mod tests {
    //! The launch path of the shortcut, which `tests/shell.rs` drives only
    //! through its status line: here, what the window is told.

    use super::*;

    struct NoEmitter;

    impl Emitter for NoEmitter {
        fn view(&self, _: &View) {}
        fn toast(&self, _: &Toast) {}
    }

    struct FakeWindow {
        calls: Arc<Mutex<Vec<String>>>,
        hotkey_fails: bool,
    }

    impl Window for FakeWindow {
        fn set_click_through(&self, on: bool) -> Result<(), String> {
            self.calls.lock().unwrap().push(format!("click_through {on}"));
            Ok(())
        }
        fn set_scale(&self, _: f64) -> Result<(), String> {
            Ok(())
        }
        fn place(&self, _: Placement) -> Result<(), String> {
            Ok(())
        }
        fn register_hotkey(&self, hotkey: &str) -> Result<(), String> {
            self.calls.lock().unwrap().push(format!("hotkey {hotkey}"));
            if self.hotkey_fails {
                Err("HotKey already registered".to_string())
            } else {
                Ok(())
            }
        }
        fn open_url(&self, _: &str) -> Result<(), String> {
            Ok(())
        }
        fn monitor(&self) -> Result<Option<MonitorMetrics>, String> {
            Ok(None)
        }
    }

    struct ZeroClock;

    impl Clock for ZeroClock {
        fn now_ms(&self) -> u64 {
            0
        }
    }

    /// Starts an app with no store and no log; returns its view after start
    /// and the window calls in order.
    fn launch(hotkey_fails: bool) -> (View, Vec<String>) {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let app = App::new(AppConfig {
            options: Options::default(),
            options_error: None,
            store: Err("no store in this test".to_string()),
            data: None,
            candidates: Vec::new(),
            poll_interval: Duration::from_millis(20),
            emitter: Box::new(NoEmitter),
            window: Box::new(FakeWindow {
                calls: Arc::clone(&calls),
                hotkey_fails,
            }),
            clock: Box::new(ZeroClock),
        });
        app.start();
        let view = app.get_view();
        assert!(app.shutdown(Duration::from_secs(2)));
        let calls = calls.lock().unwrap().clone();
        (view, calls)
    }

    #[test]
    fn launch_hotkey_failure_starts_in_edit_mode() {
        // Negative control: a shortcut that registers leaves play mode,
        // click-through on.
        let (view, calls) = launch(false);
        assert_eq!(view.mode, Mode::Play);
        assert_eq!(calls, vec!["hotkey Ctrl+Alt+Q", "click_through true"]);

        // No shortcut: play mode could never be left, so the window takes
        // clicks from the start and the status line says why.
        let (view, calls) = launch(true);
        assert_eq!(view.mode, Mode::Edit);
        assert_eq!(calls, vec!["hotkey Ctrl+Alt+Q", "click_through false"]);
        let status = view.status.expect("a status line");
        assert_eq!(status.kind, StatusKind::Error);
        assert!(status.text.contains("Ctrl+Alt+Q"), "{}", status.text);
        assert!(status.text.contains("stays in edit mode"), "{}", status.text);
    }

    fn at(height: u32, dpi_scale: f64) -> Option<MonitorMetrics> {
        Some(MonitorMetrics { height, dpi_scale })
    }

    #[test]
    fn auto_scale_by_monitor_height() {
        // At 100% DPI the target is the zoom: 1 below 1440, 1.33 from 1440,
        // 2 from 2160 physical pixels.
        for (height, zoom) in [
            (1080, 1.0),
            (1439, 1.0),
            (1440, 1.33),
            (1600, 1.33),
            (2159, 1.33),
            (2160, 2.0),
            (2880, 2.0),
        ] {
            assert_eq!(scale_factor(Scale::Auto, at(height, 1.0)), zoom, "{height}");
        }
        // Negative control: a fixed setting ignores the monitor.
        assert_eq!(scale_factor(Scale::X1, at(2160, 1.0)), 1.0);
        assert_eq!(scale_factor(Scale::X133, at(1080, 1.0)), 1.33);
        assert_eq!(scale_factor(Scale::X2, at(1080, 1.0)), 2.0);
        assert_eq!(scale_factor(Scale::X2, None), 2.0);
    }

    #[test]
    fn auto_scale_divides_out_dpi() {
        // The webview already applies Windows' DPI scale, so Auto adds only
        // what is missing: a 4K screen at 150% gets 1.33, not 2 on top of
        // 1.5; at 125% it gets 1.6.
        assert_eq!(scale_factor(Scale::Auto, at(2160, 1.5)), 1.33);
        assert_eq!(scale_factor(Scale::Auto, at(2160, 1.25)), 1.6);
        assert_eq!(scale_factor(Scale::Auto, at(1440, 1.25)), 1.06);
        assert_eq!(scale_factor(Scale::Auto, at(2160, 2.0)), 1.0);
    }

    #[test]
    fn auto_scale_never_below_one() {
        // Auto never shrinks the panel under what the DPI setting gives.
        assert_eq!(scale_factor(Scale::Auto, at(1080, 1.5)), 1.0);
        assert_eq!(scale_factor(Scale::Auto, at(1440, 1.5)), 1.0);
        assert_eq!(scale_factor(Scale::Auto, at(2160, 2.5)), 1.0);
        // No monitor, or a DPI scale that is not a scale: zoom 1, or the
        // height rule alone.
        assert_eq!(scale_factor(Scale::Auto, None), 1.0);
        assert_eq!(scale_factor(Scale::Auto, at(1440, 0.0)), 1.33);
        assert_eq!(scale_factor(Scale::Auto, at(1440, f64::NAN)), 1.33);
        assert_eq!(scale_factor(Scale::Auto, at(0, 1.0)), 1.0);
    }
}
