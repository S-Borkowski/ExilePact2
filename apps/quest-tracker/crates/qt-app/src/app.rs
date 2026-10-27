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
//! one of them up. The worker makes its window calls on a thread of its own
//! and stops waiting for one when it is told to stop, because the shell's
//! shutdown runs on the main thread such a call may be waiting for
//! (`WorkerWindow`). Disk work (saves) runs outside the lock too, under its
//! own `save_lock`, which keeps the snapshots in order.
//!
//! Saves are debounced: a change marks the state dirty, and a save runs
//! when the last one is at least [`SAVE_INTERVAL_MS`] old, from the command
//! that made the change or from the worker's next tick, and once more on
//! [`App::shutdown`]. Each file is written only when its content differs
//! from what was last written.
//!
//! The screen reader (`crate::screen`), with the `screen_reading` setting
//! on, is one more thread. It takes no state lock: the worker publishes
//! what it should look for after every batch, and it sends what it saw on
//! the worker's channel, beside the tailer's lines, each tagged with its
//! producer's generation so a stopped reader's events are dropped like a
//! stopped tailer's.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, MutexGuard, Weak};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use qt_engine::{Engine, QuestAction, QuestData, Transition};
use qt_screen::ScreenBackend;
use qt_logline::{client_pid, parse_line, EventKind, Timestamp};
use qt_store::{Loaded, Store, StoreError};
use qt_tailer::{Position, SystemClock as TailerClock, TailEvent};
use qt_viewmodel::{
    build_view, Frame, LogState, Mode, RowChange, RowChanges, Scale, Settings, StatusKind,
    StatusLine, View, ViewKind,
};
use serde_json::{json, Map, Value};

use crate::cli::Options;
use crate::diag;
use crate::discover;
use crate::screen::{self, DemandSnapshot, NotCounted, ReaderHandle, ScreenMsg, SharedBackend};
use crate::seed;

/// The shortest time between two saves.
pub const SAVE_INTERVAL_MS: u64 = 2_000;

/// How long the worker waits for a log event before its housekeeping tick.
pub const WORKER_TICK: Duration = Duration::from_millis(250);

/// The most tailer events the worker handles before it emits a view.
pub const BATCH_EVENTS: usize = 2_000;

/// How long [`App::shutdown`] waits for the threads by default.
pub const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);

/// How often the worker, waiting for one of its window calls, looks at its
/// stop flag ([`WorkerWindow`]).
const STOP_POLL: Duration = Duration::from_millis(5);

/// Sends `qt:view` and `qt:row-change` to the `overlay` window.
pub trait Emitter: Send + Sync + 'static {
    fn view(&self, view: &View);
    fn row_change(&self, change: &RowChange);
}

/// Where the window goes: its default place (the right side of its
/// monitor, below the game's minimap), or where the user dragged it. There
/// is no dock side any more (owner, after Live 2): a window with no saved
/// position goes where the right-hand dock used to put it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    Default,
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
    /// The process id of the window in front (the foreground window); `None`
    /// when there is none or its process cannot be told. Only the id: no
    /// title, class or process name is read.
    fn foreground_pid(&self) -> Option<u32>;
    /// Shows the window without activating it (`true`), so the game keeps
    /// focus, or hides it (`false`).
    fn set_visible(&self, shown: bool) -> Result<(), String>;
}

/// Whether the overlay is shown (Context "Live 2 fixes (owner scope)"):
/// always in edit mode; in play mode only while the window in front is the
/// game's (`game`, the pid of the newest log line's header, `None` while it
/// is not trusted: [`UNCONFIRMED_PID_GRACE_MS`]) or the overlay's own
/// (`own`). While the game's pid or the window in front is unknown it is
/// shown (fail open), so a log without headers never hides the overlay.
pub fn overlay_shown(mode: Mode, game: Option<u32>, foreground: Option<u32>, own: u32) -> bool {
    match (mode, game, foreground) {
        (Mode::Edit, _, _) | (_, None, _) | (_, _, None) => true,
        (Mode::Play, Some(game), Some(front)) => front == game || front == own,
    }
}

/// How long a game pid read only from catch-up or the seed may hide the
/// overlay before it is confirmed. A live line with that header, or that
/// pid's window in front within this time, confirms it. A log the running
/// game does not write to (an old install's `Client.txt`, a wrong
/// `log_path`) gives a pid no running game has, and no live line ever
/// replaces it: without this bound play mode would hide the overlay over the
/// real game for the whole session. After it, that pid is not used to hide
/// (fail open) until a live line confirms it: Windows reuses pids, so a
/// window of that pid in front no longer does.
pub const UNCONFIRMED_PID_GRACE_MS: u64 = 10_000;

/// How far the game pid in [`State::game_pid`] is trusted by the visibility
/// rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PidTrust {
    /// Read from a live line, or its window was seen in front: the running
    /// game's.
    Confirmed,
    /// Read only from catch-up or the seed, at this [`Clock`] reading: used
    /// for [`UNCONFIRMED_PID_GRACE_MS`].
    Unconfirmed { since_ms: u64 },
    /// Not confirmed within the grace: not used to hide the overlay, until a
    /// live line confirms it (its window in front no longer does).
    Stale,
}

/// What the diagnostic log says when the overlay is hidden and shown again.
/// No title, name or pid of the window in front is ever written; the game
/// pid it was compared with is in the last `game pid:` line before it.
pub const HIDDEN_LINE: &str =
    "visibility: hidden (the window in front is neither the overlay nor the game pid from the log)";
pub const SHOWN_LINE: &str = "visibility: shown";
/// What the diagnostic log says when play mode hides the overlay because
/// the session has no area line yet ([`waiting_for_area`]).
pub const NO_AREA_LINE: &str = "visibility: hidden (no character in an area yet this session)";

/// Whether play mode hides the overlay because no character is in an area
/// yet (owner, after Live 4: not over the login and character-select
/// screens). `awaiting_area` is set by a session marker (`LOG FILE OPENING`)
/// and cleared by the session's first area line, live, caught up or read
/// back before the tailer's start. It hides, whatever window is in front,
/// only while a game pid is `trusted` (confirmed, or unconfirmed within
/// [`UNCONFIRMED_PID_GRACE_MS`]): with no game pid, or one gone stale, no
/// running game is known to write this log, so the fail-open rule of
/// [`overlay_shown`] applies. Edit mode never waits.
pub fn waiting_for_area(mode: Mode, awaiting_area: bool, trusted: bool) -> bool {
    mode == Mode::Play && awaiting_area && trusted
}

/// The overlay's visibility as last applied, with why it is hidden, so each
/// change of reason is logged once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Visibility {
    Shown,
    /// Another window is in front ([`overlay_shown`]; [`HIDDEN_LINE`]).
    Hidden,
    /// No character in an area yet ([`waiting_for_area`]; [`NO_AREA_LINE`]).
    NoArea,
}

/// The diagnostic line written each time the game pid the visibility rule
/// compares with changes, and where it was read (`from`): a live line, a
/// catch-up line, or the log before the tailer's start. A pid read only from
/// catch-up or the seed may be a stale log's, which no running game has: the
/// line lets a report tell that apart from another window really in front,
/// and such a pid hides the overlay only within [`UNCONFIRMED_PID_GRACE_MS`]
/// unless it is confirmed. The pid is the user's own game client's, never
/// another program's.
fn game_pid_line(pid: u32, from: &str) -> String {
    format!("game pid: {pid} (from {from})")
}

/// What the `auto` scale reads from the window's monitor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MonitorMetrics {
    /// The monitor's height in physical pixels.
    pub height: u32,
    /// Windows' DPI scale for the monitor: 1 at 100%, 1.5 at 150%.
    pub dpi_scale: f64,
}

/// Milliseconds on a monotonic clock, for the save debounce.
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

/// The webview zoom for a `scale` setting on `monitor`. A fixed scale is
/// its factor (0.75 to 2, brought back in range by
/// [`Scale::normalised`], else Auto). `auto` targets 1 below 1440 physical
/// pixels of monitor height, 1.33 from 1440 and 2 from 2160, divided by the
/// monitor's DPI scale (the webview already applies that), rounded to
/// hundredths and never below 1, so Auto never shrinks the panel under what
/// Windows' own DPI setting gives. With no monitor, `auto` is 1.
pub fn scale_factor(scale: Scale, monitor: Option<MonitorMetrics>) -> f64 {
    match scale.factor() {
        Some(factor) => factor,
        None => monitor.map_or(1.0, auto_zoom),
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

/// Whether the window's monitor calls for a new zoom: the `auto` zoom on
/// `current` when it differs from the one worked out from `applied` (the
/// monitor the applied zoom came from; `None` when there was none), else
/// `None`. A fixed scale never changes with the monitor, and with no
/// current monitor to go on the applied zoom stays.
pub fn auto_rescale(
    scale: Scale,
    applied: Option<MonitorMetrics>,
    current: Option<MonitorMetrics>,
) -> Option<f64> {
    if scale.factor().is_some() {
        return None;
    }
    let current = current?;
    let zoom = auto_zoom(current);
    (zoom != scale_factor(Scale::Auto, applied)).then_some(zoom)
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
    /// The screen reader's backend (`qt-capture` on Windows, built with the
    /// overlay's window handle); `None` where there is none, and then the
    /// `screen_reading` setting starts nothing.
    pub screen: Option<Box<dyn ScreenBackend>>,
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
    /// The screen reader: a refusal, the overlay over a region, failing
    /// copies, or the reader stopped after an error.
    Screen,
}

/// What reaches the worker on its one channel: a tailer's event or the
/// screen reader's, each tagged with its producer's generation
/// (`State::tail_gen`, `State::screen_gen`).
#[derive(Debug)]
pub(crate) enum Inbound {
    Tail(u64, TailEvent),
    Screen(u64, ScreenMsg),
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
    row_changes: RowChanges,
    /// Which tailer's events count: raised each time a tailer is started or
    /// stopped, so lines an earlier tailer queued are dropped.
    tail_gen: u64,
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
    /// What `new-character.json` holds, as last read or written: `Some(None)`
    /// is no file, `None` unknown (a failed read or write), so the next save
    /// writes or clears it.
    saved_new_char: Option<Option<Value>>,
    last_view: Option<View>,
    /// The monitor the applied `auto` zoom was worked out from (`None`
    /// when there was none), so a move re-applies only a zoom that differs.
    scale_monitor: Option<MonitorMetrics>,
    /// The log time of the last line handled, the [`Clock`] reading when it
    /// was, and whether it was live: what "now" in log time is worked out
    /// from when the worker's tick closes a quiet relog window.
    log_clock: Option<LogClock>,
    /// The game client's pid, from the header of the newest line handled
    /// (live, catch-up, or read back before the tailer's start); `None`
    /// until one is seen. Each change is logged ([`game_pid_line`]).
    game_pid: Option<u32>,
    /// Whether [`State::game_pid`] may hide the overlay ([`PidTrust`]).
    pid_trust: PidTrust,
    /// The window's visibility, as last applied (it is created shown).
    visibility: Visibility,
    /// A session marker was read and no area line after it yet: no
    /// character is in an area this session ([`waiting_for_area`]).
    awaiting_area: bool,
    /// While a relog window is open, the position at the start of the line
    /// that opened it: what is saved instead of [`State::position`], so a
    /// relaunch reads the entry again and decides it. `None` when that start
    /// is not known (the first line of a tailer that began at the end of the
    /// log, or of another file): nothing newer than the position on disk is
    /// saved then.
    relog_entry: Option<Position>,
    /// Which screen reader's events count: raised each time one is started
    /// or stopped, so what a stopped one queued is dropped.
    screen_gen: u64,
    /// The [`Clock`] reading when the worker received the last live area
    /// entry, and the last live `has been slain` line of the current
    /// character: a bar last seen before either went for that reason, not
    /// for the boss's death (Context "Design: the reader thread" › "Cancel
    /// at the worker"). That catches a going the reader had already queued
    /// when the line came; a bar that stays up through the death screen is
    /// last seen after it, and the reader's own mark drops its going.
    screen_area_ms: Option<u64>,
    screen_death_ms: Option<u64>,
    /// The live deaths of the current character the worker received, which
    /// the demand carries so the reader can mark the bars up at a death
    /// (`DemandSnapshot::deaths`).
    screen_deaths: u64,
}

/// The last handled line's place in log time (see `State::log_clock`).
#[derive(Clone, Copy)]
struct LogClock {
    time: Timestamp,
    at_ms: u64,
    live: bool,
}

impl LogClock {
    /// Log time `now_ms` on the shell's clock: the line's timestamp
    /// advanced by the whole seconds elapsed since it was handled.
    fn now(&self, now_ms: u64) -> Timestamp {
        // A u64 of milliseconds over 1000 always fits in an i64.
        let elapsed = (now_ms.saturating_sub(self.at_ms) / 1_000) as i64;
        self.time.plus_seconds(elapsed)
    }
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
            None => Placement::Default,
        }
    }

    /// The log position to save: the end of the last line handled, except
    /// while a relog window is open (see [`State::relog_entry`]).
    fn position_to_save(&self) -> Option<&Position> {
        if self.engine.relog_window_open() {
            self.relog_entry.as_ref()
        } else {
            self.position.as_ref()
        }
    }
}

/// What a command leaves to do once the lock is released.
#[derive(Default)]
struct After {
    hotkey: Option<String>,
    retail: bool,
    /// The screen reader is to start (`true`) or stop (`false`).
    screen: Option<bool>,
}

struct Tailer {
    stop: Arc<AtomicBool>,
    handle: JoinHandle<()>,
}

/// The shell: state, threads and commands. Shared as `Arc<App>`.
pub struct App {
    /// The app itself, for the threads a command starts.
    me: Weak<App>,
    options: Options,
    store: Option<Store>,
    emitter: Box<dyn Emitter>,
    /// Shared with the worker's window thread ([`WorkerWindow`]).
    window: Arc<dyn Window>,
    clock: Box<dyn Clock>,
    candidates: Vec<PathBuf>,
    poll_interval: Duration,
    state: Mutex<State>,
    save_lock: Mutex<()>,
    /// Held from a monitor read to the zoom it leads to, so two window
    /// events cannot apply their reads out of order. Taken before the state
    /// lock, never while holding it.
    scale_lock: Mutex<()>,
    /// Every tailer's and screen reader's events, tagged with the generation
    /// it was started under ([`Inbound`]).
    tx: Sender<Inbound>,
    rx: Mutex<Option<Receiver<Inbound>>>,
    tailer: Mutex<Option<Tailer>>,
    /// The screen reader's backend, shared by each reader started.
    screen_backend: Option<SharedBackend>,
    /// The newest demand the worker published; the reader takes it with
    /// `try_lock` and never waits.
    screen_demand: Mutex<Arc<DemandSnapshot>>,
    reader: Mutex<Option<ReaderHandle>>,
    worker: Mutex<Option<JoinHandle<()>>>,
    worker_stop: Arc<AtomicBool>,
    stopped: AtomicBool,
    /// A quit was requested ([`App::request_quit`]).
    quitting: AtomicBool,
    saves: AtomicU64,
}

/// Where a request to quit the overlay came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuitFrom {
    /// The tray icon's "Quit Quest Tracker".
    Tray,
    /// A close request to the overlay's own window (`WM_CLOSE`), shown or
    /// hidden.
    WindowClosed,
}

impl QuitFrom {
    /// The diagnostic line a first request writes.
    fn line(self) -> &'static str {
        match self {
            QuitFrom::Tray => "quit: from the tray",
            QuitFrom::WindowClosed => "quit: the window was closed",
        }
    }
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
            screen,
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
        // What `new-character.json` holds (`Some(None)`: no record), and the
        // diagnostic line its load leaves.
        let mut saved_new_char: Option<Option<Value>> = None;
        let mut new_char_diag: Option<String> = None;
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
            // The unnamed new character's record, after the characters and
            // the pin: an open one reopens, so a relaunch mid-session keeps
            // showing it, and takes the pin as the one its start released.
            match s.load_new_character() {
                Ok(l) => {
                    if let Some(q) = &l.quarantined {
                        store_note = Some(StatusLine::warn(q.status_text()));
                        new_char_diag = Some(
                            "character: the new character's record could not be read and was set aside"
                                .to_string(),
                        );
                    }
                    saved_new_char = Some(None);
                    if let Some(v) = l.value {
                        match engine.load_new_character(&v) {
                            Ok(()) => {
                                new_char_diag = Some(if engine.new_character_open() {
                                    "character: the new character reopened from its record"
                                        .to_string()
                                } else {
                                    "character: a parked new character loaded from its record"
                                        .to_string()
                                });
                                saved_new_char = Some(Some(v));
                            }
                            // Left on disk as it is, like a character file
                            // the engine refuses; a new record replaces it.
                            Err(e) => {
                                store_note = Some(StatusLine::warn(format!(
                                    "The new character's saved record could not be used: {e}"
                                )));
                                new_char_diag =
                                    Some(format!("error: the new character's record: {e}"));
                            }
                        }
                    }
                }
                Err(e) => {
                    store_note = Some(StatusLine::error(format!(
                        "Cannot read the new character's record: {e}"
                    )));
                }
            }
            let _ = engine.take_character_notes();
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
            row_changes: RowChanges::new(),
            tail_gen: 0,
            statuses: BTreeMap::new(),
            log_path: log_path.as_ref().map(|(p, _)| p.clone()),
            position,
            dirty: false,
            last_save_ms: None,
            saved_settings: None,
            saved_position: None,
            saved_roster: None,
            saved_chars: BTreeMap::new(),
            saved_new_char,
            last_view: None,
            scale_monitor: None,
            log_clock: None,
            game_pid: None,
            pid_trust: PidTrust::Stale,
            visibility: Visibility::Shown,
            awaiting_area: false,
            relog_entry: None,
            screen_gen: 0,
            screen_area_ms: None,
            screen_death_ms: None,
            screen_deaths: 0,
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

        let app = Arc::new_cyclic(|me| App {
            me: me.clone(),
            options,
            store,
            emitter,
            window: Arc::from(window),
            clock,
            candidates,
            poll_interval,
            state: Mutex::new(state),
            save_lock: Mutex::new(()),
            scale_lock: Mutex::new(()),
            tx,
            rx: Mutex::new(Some(rx)),
            tailer: Mutex::new(None),
            screen_backend: screen.map(|b| Arc::new(Mutex::new(b))),
            screen_demand: Mutex::new(Arc::default()),
            reader: Mutex::new(None),
            worker: Mutex::new(None),
            worker_stop: Arc::new(AtomicBool::new(false)),
            stopped: AtomicBool::new(false),
            quitting: AtomicBool::new(false),
            saves: AtomicU64::new(0),
        });
        if let Some(e) = &options_error {
            app.diag(&format!("command line refused: {e}"));
        }
        if let Some(e) = &store_error {
            app.diag(&format!("store unavailable: {e}"));
        }
        if let Some(line) = &new_char_diag {
            app.diag(line);
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
        // Placed first, so `auto` reads the monitor the window was placed
        // on; a window at its default place is placed again once the zoom
        // has resized it, so its edge keeps the margin to the monitor's side.
        if let Err(e) = self.window.place(placement) {
            self.window_error("placement", &e);
        }
        {
            let _scale = self.scale_lock.lock().unwrap_or_else(|p| p.into_inner());
            let monitor = if scale == Scale::Auto {
                self.read_monitor()
            } else {
                None
            };
            self.lock().scale_monitor = monitor;
            if let Err(e) = self.window.set_scale(self.zoom(scale, monitor)) {
                self.window_error("scale", &e);
            }
        }
        if placement == Placement::Default {
            if let Err(e) = self.window.place(placement) {
                self.window_error("placement", &e);
            }
        }
        self.start_tailer();
        self.spawn_worker();
        if self.lock().settings.screen_reading {
            self.start_reader();
        }
        let mut st = self.lock();
        self.announce(&mut st);
    }

    /// Stops the tailer, the worker and the screen reader, waits for them up
    /// to `timeout`, and saves whatever is unsaved. True when every thread
    /// ended in time. An OCR call in flight is not interrupted: a reader
    /// inside one past the deadline is logged as not ended, like the others.
    pub fn shutdown(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        self.worker_stop.store(true, Ordering::Release);
        let tailer = self.lock_tailer().take();
        if let Some(t) = &tailer {
            t.stop.store(true, Ordering::Release);
        }
        let reader = self.lock_reader().take();
        if let Some(r) = &reader {
            r.stop.store(true, Ordering::Release);
        }
        let worker = self
            .worker
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .take();
        let tailer_ok = tailer.map_or(true, |t| join_by(t.handle, deadline));
        let worker_ok = worker.map_or(true, |w| join_by(w, deadline));
        let reader_ok = reader.map(|r| join_by(r.handle, deadline));
        self.save_if_due(true);
        let ended = |ok: bool| if ok { "ended" } else { "did not end" };
        if !self.stopped.swap(true, Ordering::AcqRel) {
            self.diag(&match reader_ok {
                None if tailer_ok && worker_ok => {
                    "stop: tailer and worker ended, progress saved".to_string()
                }
                None => format!(
                    "stop: tailer {}, worker {} within {} ms; progress saved",
                    ended(tailer_ok),
                    ended(worker_ok),
                    timeout.as_millis()
                ),
                Some(true) if tailer_ok && worker_ok => {
                    "stop: tailer, worker and screen reader ended, progress saved".to_string()
                }
                Some(reader_ok) => format!(
                    "stop: tailer {}, worker {}, screen reader {} within {} ms; progress saved",
                    ended(tailer_ok),
                    ended(worker_ok),
                    ended(reader_ok),
                    timeout.as_millis()
                ),
            });
        }
        tailer_ok && worker_ok && reader_ok.unwrap_or(true)
    }

    /// True while a screen reader thread runs.
    pub fn screen_reader_running(&self) -> bool {
        self.lock_reader()
            .as_ref()
            .is_some_and(|r| !r.handle.is_finished())
    }

    /// A request to quit, from the tray or a close of the overlay's window
    /// (Context "Tray icon and a clean exit while hidden"). The first request
    /// writes one line saying where it came from and returns true; any later
    /// one writes nothing and returns false. It stops nothing itself: the
    /// caller exits the app, whose exit runs [`App::shutdown`] as before.
    pub fn request_quit(&self, from: QuitFrom) -> bool {
        if self.quitting.swap(true, Ordering::AcqRel) {
            return false;
        }
        self.diag(from.line());
        true
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

    /// One batch of events from the tailer running now: each line is parsed
    /// with the data's allowlist and fed to the engine, the position kept,
    /// and then one fresh view is emitted (when it differs from the last)
    /// followed by one `qt:row-change` per live transition that changed a
    /// row.
    pub fn handle_tail_events(&self, events: Vec<TailEvent>) {
        let st = self.lock();
        let generation = st.tail_gen;
        self.apply_tail_events(
            st,
            generation,
            events.into_iter().map(|e| Inbound::Tail(generation, e)),
        );
    }

    /// The generation of the tailer running now. Events tagged with an
    /// earlier one came from a tailer since stopped.
    pub fn tail_generation(&self) -> u64 {
        self.lock().tail_gen
    }

    /// A batch of tailer events as the worker drains them, each tagged with
    /// the generation of the tailer that sent it. Those of a tailer since
    /// stopped (one that followed the log before a path change) are dropped
    /// under the same lock that changed the path: none reaches the engine,
    /// and none becomes the saved position of the new path.
    pub fn handle_tail_events_of(&self, events: Vec<(u64, TailEvent)>) {
        let st = self.lock();
        let generation = st.tail_gen;
        self.apply_tail_events(
            st,
            generation,
            events.into_iter().map(|(tag, e)| Inbound::Tail(tag, e)),
        );
    }

    /// A batch as the worker drains it: tailer and screen events in the
    /// order they arrived, those of a stopped producer dropped.
    fn handle_inbound(&self, items: Vec<Inbound>) {
        let st = self.lock();
        let generation = st.tail_gen;
        self.apply_tail_events(st, generation, items.into_iter());
    }

    fn apply_tail_events(
        &self,
        mut st: MutexGuard<'_, State>,
        generation: u64,
        events: impl Iterator<Item = Inbound>,
    ) {
        let path = st.log_path.clone().unwrap_or_default();
        let mut stale = 0usize;
        let mut transitions: Vec<Transition> = Vec::new();
        let mut caught_up = 0usize;
        // The game pid before this batch, and where its newest change came
        // from: logged once after the batch, so a long catch-up over many
        // game sessions writes one line, not one per session.
        let pid_before = st.game_pid;
        let mut pid_from = "";
        for item in events {
            let (tag, event) = match item {
                Inbound::Tail(tag, event) => (tag, event),
                Inbound::Screen(tag, msg) => {
                    // A stopped reader's are dropped without a line: every
                    // `screen:` line has one of the shapes Live 9 checks.
                    if tag == st.screen_gen {
                        self.apply_screen_msg(&mut st, msg, &mut transitions);
                    }
                    continue;
                }
            };
            if tag != generation {
                stale += 1;
                continue;
            }
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
                    let parsed = parse_line(&line.text, st.engine.allowlist());
                    let time = parsed
                        .as_ref()
                        .map(|ev| ev.time)
                        .or_else(|| line.text.get(..19).and_then(Timestamp::parse));
                    if let Some(time) = time {
                        st.log_clock = Some(LogClock {
                            time,
                            at_ms: self.clock.now_ms(),
                            live,
                        });
                    }
                    // The game's pid: every line with a client header,
                    // whether or not it gives an event. A live line's is the
                    // running game's; a catch-up line's is trusted only for
                    // a while ([`PidTrust`]).
                    if let Some(pid) = client_pid(&line.text) {
                        if st.game_pid != Some(pid) {
                            pid_from = if live {
                                "a live line's header"
                            } else {
                                "a catch-up line's header"
                            };
                            if !live {
                                st.pid_trust = PidTrust::Unconfirmed {
                                    since_ms: self.clock.now_ms(),
                                };
                            }
                        }
                        if live {
                            st.pid_trust = PidTrust::Confirmed;
                        }
                        st.game_pid = Some(pid);
                    }
                    let window_was_open = st.engine.relog_window_open();
                    if let Some(ev) = parsed {
                        // Live or caught up alike: a marker starts the wait
                        // for a character in an area, an area line ends it.
                        match ev.kind {
                            EventKind::SessionStart => st.awaiting_area = true,
                            EventKind::AreaEntered { .. } => st.awaiting_area = false,
                            _ => {}
                        }
                        if live {
                            if let Some(m) = diag::event(&ev) {
                                self.diag(&m);
                            }
                            // When the worker received an area entry or the
                            // current character's death, for the screen
                            // reader's cancel.
                            match &ev.kind {
                                EventKind::AreaEntered { .. } => {
                                    st.screen_area_ms = Some(self.clock.now_ms())
                                }
                                EventKind::Slain { name }
                                    if st.engine.current() == Some(name.as_str()) =>
                                {
                                    st.screen_death_ms = Some(self.clock.now_ms());
                                    st.screen_deaths += 1;
                                }
                                _ => {}
                            }
                        }
                        let before = st.engine.current().map(str::to_string);
                        let ts = st.engine.apply(&ev, live);
                        // Catch-up as well as live: a pin given up or kept
                        // while catching up is explained too.
                        self.log_character_notes(&mut st);
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
                    let end = line.position(&path);
                    if !window_was_open && st.engine.relog_window_open() {
                        // This line opened a relog window: it starts where
                        // the line before it, of the same file, ended.
                        st.relog_entry = st.position.clone().filter(|p| {
                            p.path == end.path
                                && p.fingerprint == end.fingerprint
                                && p.offset < end.offset
                        });
                    }
                    st.position = Some(end);
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
        if stale > 0 {
            self.diag(&format!("log: {stale} events of a stopped log reader dropped"));
        }
        if caught_up > 0 {
            self.diag(&diag::catch_up(&st.engine, caught_up));
        }
        match st.game_pid {
            Some(pid) if st.game_pid != pid_before => self.diag(&game_pid_line(pid, pid_from)),
            _ => {}
        }
        self.publish(&mut st, &transitions);
    }

    /// Emits a fresh view (when it differs from the last) and then one
    /// `qt:row-change` per live transition that changed a row.
    fn publish(&self, st: &mut State, transitions: &[Transition]) {
        let data = st.engine.data().clone();
        let changes = st.row_changes.make(&data, transitions);
        let view = self.build(st);
        if st.last_view.as_ref() != Some(&view) {
            self.emitter.view(&view);
            st.last_view = Some(view);
        }
        for c in &changes {
            self.emitter.row_change(c);
            self.diag(&diag::row_change(c));
        }
        self.publish_demand(st);
    }

    // -- the screen reader ---------------------------------------------------

    /// Publishes what the screen reader should look for now: nothing while
    /// the setting is off, else the engine's screen demand, the game pid
    /// once confirmed and the character's live deaths. Replaced only when it
    /// changed, so the reader keeps its trackers' state across batches.
    fn publish_demand(&self, st: &State) {
        let snapshot = if st.settings.screen_reading {
            let pid = st.game_pid.filter(|_| st.pid_trust == PidTrust::Confirmed);
            DemandSnapshot::of(&st.engine, pid, st.screen_deaths)
        } else {
            DemandSnapshot::default()
        };
        let mut slot = self.demand_slot().lock().unwrap_or_else(|p| p.into_inner());
        if **slot != snapshot {
            *slot = Arc::new(snapshot);
        }
    }

    /// One message of the running screen reader: its status line, or a
    /// tracker's event checked, handed to the engine and logged with its
    /// outcome. A bar last seen before the worker received an area entry or
    /// the character's death went for that reason and is not counted, and
    /// so is the going the reader marked as the first after a death with the
    /// bar up (Context "The review fixes" › 1); an event no character would
    /// take now (a relog window open, or no character yet) is not counted
    /// either.
    fn apply_screen_msg(&self, st: &mut State, msg: ScreenMsg, out: &mut Vec<Transition>) {
        let (event, wall_ms, after_death) = match msg {
            ScreenMsg::Status(line) => {
                st.set_status(Source::Screen, line);
                return;
            }
            ScreenMsg::Seen {
                event,
                wall_ms,
                after_death,
            } => (event, wall_ms, after_death),
        };
        let cancelled = match &event {
            qt_screen::ScreenEvent::BarGone { last_present, .. } => {
                let before = |t: Option<u64>| t.is_some_and(|t| *last_present < t as i64);
                if before(st.screen_area_ms) {
                    Some(NotCounted::AfterAreaChange)
                } else if after_death || before(st.screen_death_ms) {
                    Some(NotCounted::AfterDeath)
                } else {
                    None
                }
            }
            _ => None,
        };
        let (outcome, transitions) = match cancelled {
            Some(why) => (Err(why), Vec::new()),
            None if st.engine.relog_window_open() || st.engine.quest_target().is_none() => {
                (Err(NotCounted::NoOpenStep), Vec::new())
            }
            None => {
                let result = st.engine.apply_screen(&screen::engine_event(&event));
                (screen::engine_outcome(&result.outcome), result.transitions)
            }
        };
        // A tick is evidence to save, whether or not a row changed.
        if outcome.is_ok() {
            st.dirty = true;
        }
        self.diag(&format!(
            "{}: {}",
            screen::seen_head(&event, wall_ms),
            screen::outcome_text(outcome)
        ));
        for quest_id in st.engine.take_refused() {
            self.diag(&diag::refused(&quest_id));
        }
        for t in &transitions {
            self.diag(&diag::transition(t));
        }
        out.extend(transitions);
    }

    /// Starts the screen reader, unless one runs or there is no backend.
    fn start_reader(&self) {
        let (Some(backend), Some(app)) = (&self.screen_backend, self.me.upgrade()) else {
            return;
        };
        let mut slot = self.lock_reader();
        if slot.as_ref().is_some_and(|r| !r.handle.is_finished()) {
            return;
        }
        let generation = {
            let mut st = self.lock();
            st.screen_gen += 1;
            st.set_status(Source::Screen, None);
            self.publish_demand(&st);
            st.screen_gen
        };
        let stop = Arc::new(AtomicBool::new(false));
        let (backend, flag) = (Arc::clone(backend), Arc::clone(&stop));
        let spawned = thread::Builder::new()
            .name("qt-screen".to_string())
            .spawn(move || screen::run_reader(app, backend, generation, flag));
        match spawned {
            Ok(handle) => *slot = Some(ReaderHandle { stop, handle }),
            Err(e) => {
                self.diag(&format!("error: screen reader: the thread did not start: {e}"));
                self.diag("screen: refused: stopped after an error");
                self.lock()
                    .set_status(Source::Screen, Some(StatusLine::warn(screen::STOPPED_STATUS)));
            }
        }
    }

    /// Stops the screen reader and waits for it up to [`SHUTDOWN_TIMEOUT`],
    /// as a tailer restart does; what it queued is dropped and its status
    /// line cleared.
    fn stop_reader(&self) {
        let reader = self.lock_reader().take();
        let ended = reader.map(|r| {
            r.stop.store(true, Ordering::Release);
            join_by(r.handle, Instant::now() + SHUTDOWN_TIMEOUT)
        });
        {
            let mut st = self.lock();
            st.screen_gen += 1;
            st.set_status(Source::Screen, None);
            self.publish_demand(&st);
        }
        if ended == Some(false) {
            self.diag(&format!(
                "error: screen reader: did not end within {} ms",
                SHUTDOWN_TIMEOUT.as_millis()
            ));
        }
        self.diag("screen: off");
    }

    /// The shell's clock, for the reader's times.
    pub(crate) fn clock_ms(&self) -> u64 {
        self.clock.now_ms()
    }

    /// The worker's channel, for the reader's messages.
    pub(crate) fn sender(&self) -> &Sender<Inbound> {
        &self.tx
    }

    /// Where the worker publishes the reader's demand.
    pub(crate) fn demand_slot(&self) -> &Mutex<Arc<DemandSnapshot>> {
        &self.screen_demand
    }

    fn lock_reader(&self) -> MutexGuard<'_, Option<ReaderHandle>> {
        self.reader.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// The worker's housekeeping between batches: a quiet relog window
    /// decided, the window shown or hidden for what is in front, then a save
    /// when one is due.
    pub fn tick(&self) {
        self.tick_via(None);
    }

    /// [`App::tick`], with the window reached through `worker` when the
    /// worker runs it, so a requested stop never waits for a window call.
    fn tick_via(&self, worker: Option<&WorkerWindow>) {
        self.close_quiet_relog_window();
        self.follow_foreground_via(worker);
        // A pid its window in front confirmed reaches the reader's demand.
        {
            let st = self.lock();
            self.publish_demand(&st);
        }
        self.save_if_due(false);
    }

    /// Shows or hides the window by [`overlay_shown`]: in play mode it reads
    /// the pid of the window in front, and hides the overlay unless that is
    /// the game or the overlay itself, and before that it hides the overlay
    /// while the session has no character in an area ([`waiting_for_area`]).
    /// Each change is applied and logged once ([`HIDDEN_LINE`],
    /// [`NO_AREA_LINE`], [`SHOWN_LINE`]); showing never activates the
    /// window, so the game keeps focus. A game pid read only from catch-up
    /// or the seed is confirmed when its window is in front within
    /// [`UNCONFIRMED_PID_GRACE_MS`], and stops hiding the overlay when it is
    /// not; both are logged once. A pid gone stale is confirmed only by a
    /// live line, never by a window in front (Windows reuses pids).
    pub fn follow_foreground(&self) {
        self.follow_foreground_via(None);
    }

    /// [`App::follow_foreground`]; through `worker`, a window call the stop
    /// beats ends it without a decision.
    fn follow_foreground_via(&self, worker: Option<&WorkerWindow>) {
        let (mode, game) = {
            let st = self.lock();
            (st.ui.mode, st.game_pid)
        };
        // Only read when it can decide anything.
        let front = if mode == Mode::Play && game.is_some() {
            match worker {
                None => self.window.foreground_pid(),
                Some(w) => match w.call(|win| win.foreground_pid()) {
                    Some(front) => front,
                    None => return,
                },
            }
        } else {
            None
        };
        let now = self.clock.now_ms();
        let mut st = self.lock();
        // The decision stands only for the state it was made on.
        if st.ui.mode != mode || st.game_pid != game {
            return;
        }
        if let Some(pid) = game {
            // Only a pid within its grace is confirmed by its window: Windows
            // reuses pids, so once stale only a live line confirms it again
            // (the game writes one with its pid whenever it gains focus).
            match st.pid_trust {
                PidTrust::Confirmed | PidTrust::Stale => {}
                PidTrust::Unconfirmed { since_ms }
                    if now.saturating_sub(since_ms) >= UNCONFIRMED_PID_GRACE_MS =>
                {
                    st.pid_trust = PidTrust::Stale;
                    self.diag(&format!(
                        "game pid: {pid} not used to hide the overlay (no live line and no \
                         window of it in front for {} s: the log may be one the running game \
                         does not write to)",
                        UNCONFIRMED_PID_GRACE_MS / 1000
                    ));
                }
                PidTrust::Unconfirmed { .. } if front == Some(pid) => {
                    st.pid_trust = PidTrust::Confirmed;
                    self.diag(&format!("game pid: {pid} confirmed (its window is in front)"));
                }
                PidTrust::Unconfirmed { .. } => {}
            }
        }
        let used = if st.pid_trust == PidTrust::Stale { None } else { game };
        let visibility = if waiting_for_area(mode, st.awaiting_area, used.is_some()) {
            Visibility::NoArea
        } else if overlay_shown(mode, used, front, std::process::id()) {
            Visibility::Shown
        } else {
            Visibility::Hidden
        };
        if st.visibility == visibility {
            return;
        }
        let was_shown = st.visibility == Visibility::Shown;
        st.visibility = visibility;
        let shown = visibility == Visibility::Shown;
        let line = match visibility {
            Visibility::Shown => SHOWN_LINE,
            Visibility::Hidden => HIDDEN_LINE,
            Visibility::NoArea => NO_AREA_LINE,
        };
        if was_shown == shown {
            // Hidden either way, for another reason: only the line changes.
            self.diag(line);
            return;
        }
        // Tauri only posts this to its event loop, so it runs under the lock,
        // in order with any other change. Through the worker's window thread
        // a stop still beats it, and the lock is released.
        let applied = match worker {
            None => Some(self.window.set_visible(shown)),
            Some(w) => w.call(move |win| win.set_visible(shown)),
        };
        let Some(applied) = applied else {
            return;
        };
        if let Err(e) = applied {
            self.diag(&format!("error: window visibility: {e}"));
        }
        self.diag(line);
    }

    /// Closes a relog window the log has gone quiet on: with no line since
    /// a level-1 entry to The Riverbank, only the clock can tell that more
    /// than 5 s of log time have passed. "Now" is the last handled line's
    /// timestamp advanced by the time elapsed on the shell's [`Clock`] since
    /// it was handled. The character lines are written from the notes the
    /// engine posts when it closes (a new character started or resumed, a
    /// pin released), never from the window being open.
    fn close_quiet_relog_window(&self) {
        let mut st = self.lock();
        if !st.engine.relog_window_open() {
            return;
        }
        let Some(clock) = st.log_clock else {
            return;
        };
        let now = clock.now(self.clock.now_ms());
        let before = st.engine.current().map(str::to_string);
        let ts = st.engine.expire_relog_window(now);
        if st.engine.relog_window_open() {
            return;
        }
        self.log_character_notes(&mut st);
        for quest_id in st.engine.take_refused() {
            self.diag(&diag::refused(&quest_id));
        }
        let after = st.engine.current().map(str::to_string);
        if clock.live && before != after {
            self.diag(&diag::decision(&st.engine, before.as_deref(), after.as_deref()));
        }
        for t in &ts {
            self.diag(&diag::transition(t));
        }
        st.dirty = true;
        self.publish(&mut st, &ts);
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
            let name = quest_target(st)?;
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
            let name = quest_target(st)?;
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
    /// to the window at once; a new shortcut or log path is applied after
    /// the lock is released. A patch that names the scale has the monitor
    /// read first, before the lock, for `auto`. A key the settings do not
    /// have (the old `dock` side among them) refuses the whole patch.
    pub fn update_settings(&self, patch: &Value) -> Result<View, String> {
        let names_scale = patch.get("scale").is_some();
        let _scale = names_scale.then(|| self.scale_lock.lock().unwrap_or_else(|p| p.into_inner()));
        let monitor = if names_scale { self.read_monitor() } else { None };
        self.command("update_settings", |app, st| {
            let new = st.settings.patched(patch)?;
            let old = std::mem::replace(&mut st.settings, new.clone());
            st.dirty = true;
            let mut after = After::default();
            if new.scale != old.scale {
                st.scale_monitor = monitor;
                if let Err(e) = app.window.set_scale(app.zoom(new.scale, monitor)) {
                    app.diag(&format!("error: window scale: {e}"));
                }
            }
            if new.hotkey != old.hotkey {
                after.hotkey = Some(new.hotkey.clone());
            }
            if new.log_path != old.log_path {
                after.retail = true;
            }
            if new.screen_reading != old.screen_reading {
                after.screen = Some(new.screen_reading);
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

    /// The window was moved, in any mode (a drag in edit mode, a placement
    /// by the shell): `auto` follows it to its monitor ([`App::follow_monitor`]).
    /// Only a move in edit mode (a drag) is kept as the position.
    pub fn window_moved(&self, x: i32, y: i32) {
        let keep = {
            let mut st = self.lock();
            let keep = st.ui.mode == Mode::Edit && st.window_pos != Some((x, y));
            if keep {
                st.window_pos = Some((x, y));
                st.dirty = true;
            }
            keep
        };
        if keep {
            self.save_if_due(false);
        }
        self.follow_monitor();
    }

    /// Re-evaluates the `auto` scale on the window's monitor, after a move or
    /// a DPI scale change, and applies the zoom only when it differs from
    /// the one applied. A fixed scale never reads the monitor. A window at
    /// its default place is placed again after a new zoom has resized it.
    pub fn follow_monitor(&self) {
        let _scale = self.scale_lock.lock().unwrap_or_else(|p| p.into_inner());
        if self.lock().settings.scale != Scale::Auto {
            return;
        }
        // Read with no state lock held: Tauri answers it on the main thread.
        // A failed read changes nothing.
        let current = match self.window.monitor() {
            Ok(m) => m,
            Err(e) => {
                self.diag(&format!("error: window monitor: {e}"));
                return;
            }
        };
        let replace = {
            let mut st = self.lock();
            let rescale = auto_rescale(st.settings.scale, st.scale_monitor, current);
            if current.is_some() {
                st.scale_monitor = current;
            }
            let (Some(zoom), Some(m)) = (rescale, current) else {
                return;
            };
            self.diag(&format!(
                "scale auto: the window is now on a monitor {} px high at DPI scale {} -> zoom {zoom}",
                m.height, m.dpi_scale
            ));
            if let Err(e) = self.window.set_scale(zoom) {
                self.diag(&format!("error: window scale: {e}"));
            }
            st.window_pos.is_none().then_some(Placement::Default)
        };
        if let Some(p) = replace {
            if let Err(e) = self.window.place(p) {
                self.window_error("placement", &e);
            }
        }
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
        self.publish_demand(st);
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
        let again = after.hotkey.is_some() || after.retail || after.screen.is_some();
        if let Some(h) = after.hotkey {
            self.register_hotkey(&h, "change it in Settings");
        }
        if after.retail {
            self.start_tailer();
        }
        match after.screen {
            Some(true) => self.start_reader(),
            Some(false) => self.stop_reader(),
            None => {}
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

    /// Logs what happened to the new character from The Riverbank since the
    /// last call, one line per [`qt_engine::CharacterNote`].
    fn log_character_notes(&self, st: &mut State) {
        for note in st.engine.take_character_notes() {
            for line in diag::character_note(&st.engine, &note) {
                self.diag(&line);
            }
        }
    }

    fn engine_changed(&self, st: &mut State, ts: &[Transition]) {
        self.log_character_notes(st);
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
            // Whatever the stopped tailer queued is not this log's.
            st.tail_gen += 1;
            st.log_path = None;
            st.ui.log = LogState::Missing;
            st.set_status(Source::Log, None);
            drop(st);
            self.diag("log: no Client.txt found; set the log path in Settings");
            return;
        };
        let generation = {
            let mut st = self.lock();
            // From here on, only the new tailer's events count: whatever the
            // stopped one queued is dropped, under this lock, before it can
            // reach the engine or be saved as a position of this path.
            st.tail_gen += 1;
            if st.log_path.as_deref() != Some(path.as_path()) || st.ui.log == LogState::Missing {
                st.ui.log = LogState::Waiting;
                st.set_status(
                    Source::Log,
                    Some(StatusLine::info("Waiting for the game to write to the log.")),
                );
            }
            st.log_path = Some(path.clone());
            st.tail_gen
        };
        let config = qt_tailer::Config {
            path: path.clone(),
            saved: saved.filter(|p| p.path == path),
            from_start: self.options.from_start,
            poll_interval: self.poll_interval,
        };
        // The start is decided once, here: the seed reads the log up to
        // exactly where the tailer begins, and the tailer begins there.
        let follower = match qt_tailer::decide(&config) {
            Ok(start) => {
                self.seed_area(&config.path, start.at());
                qt_tailer::Follower::with_start(config, start)
            }
            Err(e) => {
                // The tailer decides on its first poll and reports the
                // failure itself, as it does for any later read.
                self.diag(&format!("error: area from log: {e}; the area is unknown"));
                {
                    let mut st = self.lock();
                    st.engine.seed_area(None);
                    st.awaiting_area = false;
                }
                qt_tailer::Follower::new(config)
            }
        };
        let stop = Arc::new(AtomicBool::new(false));
        let tx = self.tx.clone();
        let sink = move |event| tx.send(Inbound::Tail(generation, event)).is_ok();
        match qt_tailer::spawn_follower(follower, TailerClock, sink, Arc::clone(&stop)) {
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

    /// Seeds the engine's area from the log at `path` before `start`, where
    /// the tailer about to run begins reading (the decision it is handed,
    /// [`qt_tailer::decide`]), before the tailer exists, so before any of
    /// its lines reaches the engine. A start at 0, or a log not there yet,
    /// seeds nothing: the tailer reads the area lines itself. A read that
    /// fails leaves the area unknown.
    fn seed_area(&self, path: &std::path::Path, start: Option<qt_tailer::StartAt>) {
        // A new tailer: whether a session waits for an area is what this
        // log says, never what an earlier one did.
        self.lock().awaiting_area = false;
        let start = match start {
            Some(start) if start.offset > 0 => start,
            Some(_) => {
                self.diag("area from log: not read (the tailer reads the log from offset 0)");
                return;
            }
            None => {
                self.diag("area from log: not read (the log does not exist yet)");
                return;
            }
        };
        let found = seed::area_before(path, start.offset);
        let mut st = self.lock();
        match found {
            Ok(found) => {
                st.engine.seed_area(found.area.code());
                // The last session read back has no area line yet: hidden
                // while its pid is trusted, as if the marker were read now.
                st.awaiting_area = found.awaiting_area;
                self.diag(&diag::seeded_area(&found.area, start.offset));
                // Before any line of this tailer, so a newer line still wins.
                if let Some(pid) = found.pid {
                    if st.game_pid != Some(pid) {
                        self.diag(&game_pid_line(pid, "the log before the tailer's start"));
                        st.pid_trust = PidTrust::Unconfirmed {
                            since_ms: self.clock.now_ms(),
                        };
                    }
                    st.game_pid = Some(pid);
                }
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
            // Inside a relog window, the start of its entry line: a relaunch
            // then reads the entry again and decides it (events are
            // idempotent, so re-reading changes nothing else).
            if let Some(pos) = st.position_to_save().and_then(|p| serde_json::to_value(p).ok()) {
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
            // The new character's record: written while there is one,
            // cleared once the engine has closed it.
            let record = st.engine.new_character_json();
            if st.saved_new_char.as_ref() != Some(&record) {
                st.saved_new_char = Some(record.clone());
                writes.push(match record {
                    Some(v) => (Write::NewCharacter, v),
                    None => (Write::ClearNewCharacter, Value::Null),
                });
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
                Write::NewCharacter => store.save_new_character(value),
                Write::ClearNewCharacter => store.clear_new_character(),
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
                    Write::NewCharacter | Write::ClearNewCharacter => st.saved_new_char = None,
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
    NewCharacter,
    ClearNewCharacter,
}

/// The character a quest command applies to: the one the panel shows. That
/// is the pinned one, else the active one, else the open new character from
/// The Riverbank, which the engine names with the empty name; its overrides
/// are kept in its record and pass to the character it becomes (Context
/// "Quest actions on the new character"). Refused only with neither.
fn quest_target(st: &State) -> Result<String, String> {
    st.engine
        .quest_target()
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
/// [`App::handle_tail_events`], and ticks after each batch or each
/// [`WORKER_TICK`] without one, so a quiet log's relog window still closes.
/// Ends within one [`WORKER_TICK`] of `stop` being set, even while one of its
/// window calls waits for the main thread ([`WorkerWindow`]).
fn run_worker(app: Arc<App>, rx: Receiver<Inbound>, stop: Arc<AtomicBool>) {
    let window = match WorkerWindow::start(Arc::clone(&app.window), Arc::clone(&stop)) {
        Ok(w) => Some(w),
        Err(e) => {
            app.diag(&format!(
                "error: the window thread did not start: {e}; the worker calls the window itself"
            ));
            None
        }
    };
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
                app.handle_inbound(batch);
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
        app.tick_via(window.as_ref());
    }
}

type WindowJob = Box<dyn FnOnce(&dyn Window) + Send>;

/// The worker's way to the window (Context "Worker shutdown waits on the
/// main thread"). The shell calls [`App::shutdown`] on Tauri's main thread,
/// and a Tauri window getter called from any other thread (the overlay's
/// handle, read for the window in front) waits for the main thread to answer
/// it. So the worker hands each window call to a thread of its own and waits
/// for the answer only until `stop` is set: a stop then ends the worker
/// within [`STOP_POLL`], whatever the call is waiting for. A call left behind
/// ends with the main thread's answer or the process; the window thread ends
/// after it, once the worker has dropped its sender.
struct WorkerWindow {
    jobs: Sender<WindowJob>,
    stop: Arc<AtomicBool>,
}

impl WorkerWindow {
    fn start(window: Arc<dyn Window>, stop: Arc<AtomicBool>) -> std::io::Result<WorkerWindow> {
        let (jobs, rx) = mpsc::channel::<WindowJob>();
        thread::Builder::new()
            .name("qt-window".to_string())
            .spawn(move || {
                for job in rx {
                    job(&*window);
                }
            })?;
        Ok(WorkerWindow { jobs, stop })
    }

    /// `f` on the window, on the window thread; `None` when the stop was
    /// set before it returned.
    fn call<T: Send + 'static>(
        &self,
        f: impl FnOnce(&dyn Window) -> T + Send + 'static,
    ) -> Option<T> {
        if self.stop.load(Ordering::Acquire) {
            return None;
        }
        let (tx, rx) = mpsc::channel();
        let job: WindowJob = Box::new(move |w| {
            let _ = tx.send(f(w));
        });
        self.jobs.send(job).ok()?;
        loop {
            match rx.recv_timeout(STOP_POLL) {
                Ok(value) => return Some(value),
                Err(RecvTimeoutError::Timeout) if !self.stop.load(Ordering::Acquire) => {}
                Err(_) => return None,
            }
        }
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
        fn row_change(&self, _: &RowChange) {}
    }

    struct FakeWindow {
        calls: Arc<Mutex<Vec<String>>>,
        hotkey_fails: bool,
        /// The pid of the window in front.
        front: Arc<Mutex<Option<u32>>>,
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
        fn foreground_pid(&self) -> Option<u32> {
            *self.front.lock().unwrap()
        }
        fn set_visible(&self, shown: bool) -> Result<(), String> {
            self.calls.lock().unwrap().push(format!("visible {shown}"));
            Ok(())
        }
    }

    struct ManualClock(Arc<AtomicU64>);

    impl Clock for ManualClock {
        fn now_ms(&self) -> u64 {
            self.0.load(Ordering::SeqCst)
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
                front: Arc::default(),
            }),
            clock: Box::new(ZeroClock),
            screen: None,
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
        assert_eq!(scale_factor(Scale::fixed(1.0), at(2160, 1.0)), 1.0);
        assert_eq!(scale_factor(Scale::fixed(1.33), at(1080, 1.0)), 1.33);
        assert_eq!(scale_factor(Scale::fixed(2.0), at(1080, 1.0)), 2.0);
        assert_eq!(scale_factor(Scale::fixed(2.0), None), 2.0);
    }

    #[test]
    fn fixed_scale_is_its_factor() {
        // Any factor of the slider's range, below 1 too.
        for f in [0.75, 0.8, 1.25, 1.5, 1.95, 2.0] {
            assert_eq!(scale_factor(Scale::fixed(f), at(2160, 1.0)), f, "{f}");
        }
        // A factor out of range never reaches the window as is: it is Auto.
        assert_eq!(scale_factor(Scale::Fixed(3.0), at(1440, 1.0)), 1.33);
        assert_eq!(scale_factor(Scale::Fixed(f64::NAN), None), 1.0);
    }

    #[test]
    fn overlay_shown_rule() {
        let own = 4242;
        // Play mode: shown only with the game or the overlay in front.
        assert!(overlay_shown(Mode::Play, Some(15660), Some(15660), own));
        assert!(overlay_shown(Mode::Play, Some(15660), Some(own), own));
        assert!(!overlay_shown(Mode::Play, Some(15660), Some(999), own));
        // Fail open: no game pid yet, or nothing known in front.
        assert!(overlay_shown(Mode::Play, None, Some(999), own));
        assert!(overlay_shown(Mode::Play, Some(15660), None, own));
        // Edit mode: always.
        assert!(overlay_shown(Mode::Edit, Some(15660), Some(999), own));
    }

    #[test]
    fn waiting_for_area_rule() {
        // Play mode, a session with no area line, a trusted pid: hidden.
        assert!(waiting_for_area(Mode::Play, true, true));
        // Negative controls: an area line read, no trusted pid (fail open),
        // or edit mode.
        assert!(!waiting_for_area(Mode::Play, false, true));
        assert!(!waiting_for_area(Mode::Play, true, false));
        assert!(!waiting_for_area(Mode::Edit, true, true));

        // What the log before a start says: the newest marker or area line.
        let marker = "2026/03/11 10:00:00 ***** LOG FILE OPENING *****";
        let area = "2026/03/11 10:00:05 1 7e2c0001 [DEBUG Client 7120] Generating level 1 area \"G1_1\" with seed 9";
        let header = "2026/03/11 10:00:13 2 7e2c0002 [INFO Client 7120] Connecting to instance server";
        assert!(seed::awaiting_area([marker, header]));
        assert!(seed::awaiting_area([marker, area, marker, marker]));
        assert!(!seed::awaiting_area([marker, header, area, header]));
        assert!(!seed::awaiting_area([header]));
        assert!(!seed::awaiting_area(Vec::<&str>::new()));
    }

    #[test]
    fn game_pid_change_is_logged_with_its_source() {
        // The pid the visibility rule compares with is in the diagnostic
        // log, with where it was read, so a report can tell a stale log's
        // pid (catch-up only) from another window really in front.
        let dir = tempfile::tempdir().expect("temp dir");
        let store = dir.path().join("store");
        let app = App::new(AppConfig {
            options: Options::default(),
            options_error: None,
            store: Store::open(store.clone()).map_err(|e| e.to_string()),
            data: None,
            candidates: Vec::new(),
            poll_interval: Duration::from_millis(20),
            emitter: Box::new(NoEmitter),
            window: Box::new(FakeWindow {
                calls: Arc::new(Mutex::new(Vec::new())),
                hotkey_fails: false,
                front: Arc::default(),
            }),
            clock: Box::new(ZeroClock),
            screen: None,
        });
        let fingerprint = qt_tailer::Fingerprint::of(b"fixture log");
        let lines = |texts: &[&str], catch_up: bool| -> Vec<TailEvent> {
            texts
                .iter()
                .map(|t| {
                    TailEvent::Line(qt_tailer::Line {
                        text: t.to_string(),
                        end_offset: 1,
                        catch_up,
                        fingerprint,
                    })
                })
                .collect()
        };
        let log = || std::fs::read_to_string(store.join("overlay.log")).unwrap_or_default();
        let pid_lines = || -> Vec<String> {
            log().lines().filter(|l| l.contains("game pid:")).map(str::to_string).collect()
        };

        // Negative control: a line with no client header tells no pid.
        app.handle_tail_events(lines(&["2026/03/10 19:00:00 ***** LOG FILE OPENING *****"], true));
        assert!(pid_lines().is_empty(), "{}", log());

        // Two sessions caught up in one batch: one line, the newest pid.
        app.handle_tail_events(lines(
            &[
                "2026/03/10 19:00:01 1 7e2c0010 [INFO Client 7120] Connecting to instance server",
                "2026/03/10 20:00:01 2 7e2c0010 [INFO Client 15660] Connecting to instance server",
            ],
            true,
        ));
        assert_eq!(pid_lines().len(), 1, "{}", log());
        assert!(pid_lines()[0].ends_with("game pid: 15660 (from a catch-up line's header)"));

        // The same pid again, live: no new line.
        app.handle_tail_events(lines(
            &["2026/03/10 20:00:02 3 7e2c0010 [INFO Client 15660] Connecting to instance server"],
            false,
        ));
        assert_eq!(pid_lines().len(), 1, "{}", log());

        // A restart under another pid, live: logged, and it says so.
        app.handle_tail_events(lines(
            &["2026/03/10 21:00:01 4 7e2c0010 [INFO Client 2244] Connecting to instance server"],
            false,
        ));
        let found = pid_lines();
        assert_eq!(found.len(), 2, "{}", log());
        assert!(found[1].ends_with("game pid: 2244 (from a live line's header)"), "{found:?}");
        // The shell's visibility filters stay clean: no pid line reads as one.
        assert!(found.iter().all(|l| !l.contains("visibility")), "{found:?}");
    }

    #[test]
    fn stale_log_pid_stops_hiding_the_overlay() {
        // A followed log the running game does not write to: its newest
        // header is an old client's pid (15660), while the real game runs
        // as 7120 and writes elsewhere, so no live line ever comes.
        const STALE: u32 = 15_660;
        const REAL_GAME: u32 = 7_120;
        const OTHER: u32 = 9_999;
        let dir = tempfile::tempdir().expect("temp dir");
        let store = dir.path().join("store");
        let calls = Arc::new(Mutex::new(Vec::new()));
        let front = Arc::new(Mutex::new(None));
        let clock = Arc::new(AtomicU64::new(100_000));
        let app = App::new(AppConfig {
            options: Options::default(),
            options_error: None,
            store: Store::open(store.clone()).map_err(|e| e.to_string()),
            data: None,
            candidates: Vec::new(),
            poll_interval: Duration::from_millis(20),
            emitter: Box::new(NoEmitter),
            window: Box::new(FakeWindow {
                calls: Arc::clone(&calls),
                hotkey_fails: false,
                front: Arc::clone(&front),
            }),
            clock: Box::new(ManualClock(Arc::clone(&clock))),
            screen: None,
        });
        let fingerprint = qt_tailer::Fingerprint::of(b"fixture log");
        let line = |pid: u32, catch_up: bool| -> Vec<TailEvent> {
            vec![TailEvent::Line(qt_tailer::Line {
                text: format!(
                    "2026/03/10 19:00:01 1 7e2c0010 [INFO Client {pid}] Connecting to instance server"
                ),
                end_offset: 1,
                catch_up,
                fingerprint,
            })]
        };
        let in_front = |pid: u32| {
            *front.lock().unwrap() = Some(pid);
            app.follow_foreground();
        };
        let visible = || -> Vec<String> {
            calls.lock().unwrap().iter().filter(|c| c.starts_with("visible")).cloned().collect()
        };
        let log = || std::fs::read_to_string(store.join("overlay.log")).unwrap_or_default();

        app.handle_tail_events(line(STALE, true));
        // Within the grace the catch-up pid is used: the real game in front
        // looks like another program, and the overlay is hidden.
        in_front(REAL_GAME);
        assert_eq!(visible(), vec!["visible false"]);
        clock.fetch_add(UNCONFIRMED_PID_GRACE_MS - 1, Ordering::SeqCst);
        in_front(REAL_GAME);
        assert_eq!(visible(), vec!["visible false"]);
        // Target: not confirmed within the grace, the pid stops hiding the
        // overlay, and the log says why, once.
        clock.fetch_add(1, Ordering::SeqCst);
        in_front(REAL_GAME);
        assert_eq!(visible(), vec!["visible false", "visible true"]);
        for _ in 0..3 {
            clock.fetch_add(60_000, Ordering::SeqCst);
            in_front(REAL_GAME);
            in_front(OTHER);
        }
        assert_eq!(visible().len(), 2, "{:?}", visible());
        let text = log();
        assert_eq!(
            text.matches(&format!("game pid: {STALE} not used to hide the overlay")).count(),
            1,
            "{text}"
        );
        assert!(!text.contains(&REAL_GAME.to_string()), "{text}");

        // A window of the stale pid in front no longer confirms it (Windows
        // reuses pids): the overlay stays shown over another program.
        in_front(STALE);
        assert!(!log().contains(&format!("game pid: {STALE} confirmed")), "{}", log());
        in_front(OTHER);
        assert_eq!(visible(), vec!["visible false", "visible true"]);
        // A live line with it does: from then on the rule hides as before,
        // whatever the clock says.
        app.handle_tail_events(line(STALE, false));
        clock.fetch_add(60_000, Ordering::SeqCst);
        in_front(OTHER);
        assert_eq!(visible(), vec!["visible false", "visible true", "visible false"]);

        // Negative control: a pid from a live line is the running game's
        // from the start, and never expires.
        in_front(STALE);
        app.handle_tail_events(line(REAL_GAME, false));
        clock.fetch_add(60_000, Ordering::SeqCst);
        in_front(OTHER);
        clock.fetch_add(60_000, Ordering::SeqCst);
        in_front(OTHER);
        assert_eq!(
            visible(),
            vec!["visible false", "visible true", "visible false", "visible true", "visible false"]
        );
        // A catch-up pid that its window confirms within the grace hides
        // after it too.
        app.handle_tail_events(line(4_242, true));
        in_front(4_242);
        clock.fetch_add(60_000, Ordering::SeqCst);
        in_front(OTHER);
        assert_eq!(visible().len(), 7, "{:?}", visible());
        assert_eq!(visible().last().map(String::as_str), Some("visible false"));
    }

    /// An app with a store, a fake window whose window in front is set by
    /// the test, and a clock the test moves; no tailer and no worker.
    struct PidRig {
        _dir: tempfile::TempDir,
        store: PathBuf,
        app: Arc<App>,
        calls: Arc<Mutex<Vec<String>>>,
        front: Arc<Mutex<Option<u32>>>,
        clock: Arc<AtomicU64>,
    }

    impl PidRig {
        fn new() -> PidRig {
            let dir = tempfile::tempdir().expect("temp dir");
            let store = dir.path().join("store");
            let calls = Arc::new(Mutex::new(Vec::new()));
            let front = Arc::new(Mutex::new(None));
            let clock = Arc::new(AtomicU64::new(100_000));
            let app = App::new(AppConfig {
                options: Options::default(),
                options_error: None,
                store: Store::open(store.clone()).map_err(|e| e.to_string()),
                data: None,
                candidates: Vec::new(),
                poll_interval: Duration::from_millis(20),
                emitter: Box::new(NoEmitter),
                window: Box::new(FakeWindow {
                    calls: Arc::clone(&calls),
                    hotkey_fails: false,
                    front: Arc::clone(&front),
                }),
                clock: Box::new(ManualClock(Arc::clone(&clock))),
                screen: None,
            });
            PidRig { _dir: dir, store, app, calls, front, clock }
        }

        /// One line with `pid` in its header, read live or as catch-up.
        fn line(&self, pid: u32, catch_up: bool) {
            self.app.handle_tail_events(vec![TailEvent::Line(qt_tailer::Line {
                text: format!(
                    "2026/03/10 19:00:01 1 7e2c0010 [INFO Client {pid}] Connecting to instance server"
                ),
                end_offset: 1,
                catch_up,
                fingerprint: qt_tailer::Fingerprint::of(b"fixture log"),
            })]);
        }

        fn in_front(&self, pid: u32) {
            *self.front.lock().unwrap() = Some(pid);
            self.app.follow_foreground();
        }

        fn wait(&self, ms: u64) {
            self.clock.fetch_add(ms, Ordering::SeqCst);
        }

        fn visible(&self) -> Vec<String> {
            self.calls.lock().unwrap().iter().filter(|c| c.starts_with("visible")).cloned().collect()
        }

        fn log(&self) -> String {
            std::fs::read_to_string(self.store.join("overlay.log")).unwrap_or_default()
        }
    }

    #[test]
    fn unconfirmed_pid_is_confirmed_by_its_window_in_front() {
        // Baseline: a pid read only from catch-up whose window comes to the
        // front within the grace is the running game's, confirmed once, and
        // hides the overlay over another program long after the grace.
        const GAME: u32 = 15_660;
        const OTHER: u32 = 9_999;
        let rig = PidRig::new();
        rig.line(GAME, true);
        rig.wait(UNCONFIRMED_PID_GRACE_MS - 1);
        rig.in_front(GAME);
        rig.in_front(GAME);
        let text = rig.log();
        assert_eq!(
            text.matches(&format!("game pid: {GAME} confirmed (its window is in front)")).count(),
            1,
            "{text}"
        );
        rig.wait(60_000);
        rig.in_front(OTHER);
        assert_eq!(rig.visible(), vec!["visible false"]);
        assert!(!rig.log().contains("not used to hide the overlay"), "{}", rig.log());
    }

    #[test]
    fn stale_pid_is_not_confirmed_by_a_window_in_front() {
        // Target: once the grace has passed unconfirmed, a window of that
        // pid in front (Windows reuses pids) confirms nothing, even on the
        // tick the grace ends; the pid stays unused, so the overlay stays
        // shown over another program.
        const STALE: u32 = 15_660;
        const OTHER: u32 = 9_999;
        let rig = PidRig::new();
        rig.line(STALE, true);
        rig.wait(UNCONFIRMED_PID_GRACE_MS);
        rig.in_front(STALE);
        for _ in 0..3 {
            rig.wait(60_000);
            rig.in_front(STALE);
            rig.in_front(OTHER);
        }
        let text = rig.log();
        assert!(!text.contains("confirmed"), "{text}");
        assert_eq!(
            text.matches(&format!("game pid: {STALE} not used to hide the overlay")).count(),
            1,
            "{text}"
        );
        assert!(rig.visible().is_empty(), "{:?}", rig.visible());

        // Positive control: a live line with that pid confirms it, and the
        // same program in front then hides the overlay.
        rig.line(STALE, false);
        rig.in_front(OTHER);
        assert_eq!(rig.visible(), vec!["visible false"]);
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

    #[test]
    fn auto_scale_rescales_on_monitor_change() {
        // The window went from a 1080 px to a 1440 px monitor: 1.33, and
        // back: 1.
        assert_eq!(auto_rescale(Scale::Auto, at(1080, 1.0), at(1440, 1.0)), Some(1.33));
        assert_eq!(auto_rescale(Scale::Auto, at(1440, 1.0), at(1080, 1.0)), Some(1.0));
        // Same height, another DPI scale (Windows' setting changed, or a
        // monitor of the same size at 125%).
        assert_eq!(auto_rescale(Scale::Auto, at(2160, 1.0), at(2160, 1.25)), Some(1.6));
        // No monitor when the zoom was applied (zoom 1), one now.
        assert_eq!(auto_rescale(Scale::Auto, None, at(2160, 1.0)), Some(2.0));
    }

    #[test]
    fn auto_scale_same_monitor_keeps_zoom() {
        // A move on the same monitor changes nothing.
        assert_eq!(auto_rescale(Scale::Auto, at(1440, 1.0), at(1440, 1.0)), None);
        // Another monitor with the same zoom: nothing to apply either.
        assert_eq!(auto_rescale(Scale::Auto, at(1080, 1.0), at(1200, 1.0)), None);
        assert_eq!(auto_rescale(Scale::Auto, at(1440, 1.0), at(2160, 1.5)), None);
        assert_eq!(auto_rescale(Scale::Auto, None, at(1080, 1.0)), None);
        // No monitor to go on: the applied zoom stays.
        assert_eq!(auto_rescale(Scale::Auto, at(1440, 1.0), None), None);
    }

    #[test]
    fn fixed_scale_ignores_monitor_change() {
        for scale in [Scale::fixed(1.0), Scale::fixed(1.33), Scale::fixed(2.0), Scale::fixed(0.75)] {
            assert_eq!(auto_rescale(scale, at(1080, 1.0), at(2160, 1.0)), None, "{scale:?}");
            assert_eq!(auto_rescale(scale, None, at(1440, 1.0)), None, "{scale:?}");
        }
        // Positive control: the same move under `auto` does rescale.
        assert_eq!(auto_rescale(Scale::Auto, at(1080, 1.0), at(2160, 1.0)), Some(2.0));
    }
}
