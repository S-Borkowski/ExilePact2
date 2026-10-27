//! The panel watcher's thread (Context "Design: the panel watcher"): with
//! the `hide_under_panels` setting on, it copies a few small patches of the
//! game's window where the panels under the overlay draw their frames, tells
//! from their colour whether one is open (`qt-screen`'s [`PanelTable`] and
//! [`PanelTracker`]), and tells the worker, on the worker's own channel,
//! which entry is open under the overlay. The worker hides the overlay for
//! it ([`crate::PANEL_LINE`]).
//!
//! - **When it reads.** Only in play mode while the overlay is shown, or
//!   hidden by a panel alone. The worker publishes that, with the confirmed
//!   game pid, as a [`PanelSnapshot`] after each change; the watcher takes
//!   the newest with `try_lock` and never waits for it. Otherwise it idles,
//!   and its `idle (<reason>)` line names the first reason that held:
//!   `edit mode`, `overlay hidden`, `pid unconfirmed`, `no game window`,
//!   `minimized`.
//! - **A tick,** at most every [`PANEL_TICK_MS`]: the game's window by pid,
//!   its client rect and shape (16:9 only), the overlay's own window, the
//!   entries whose footprint is under it, one copy of each of their readable
//!   probes, the classification and the trackers, then a message to the
//!   worker when the first open entry under the overlay (in table order)
//!   changes. A tick that overruns skips the ones it missed.
//! - **Fail open.** Idle, paused (no overlay window, a shape other than
//!   16:9, [`CAPTURE_FAILURE_LIMIT`] failed copies in a row) or stopped
//!   after an error, the watcher forgets what it read and tells the worker
//!   no panel is open: it never keeps the overlay hidden.
//! - **Locks.** Like the screen reader, it takes no `App` state lock and
//!   never calls the shell's `Window`. Its backend is its own (never the
//!   reader's, which holds its lock through OCR calls), behind a mutex taken
//!   with `try_lock` for one tick at a time, so a watcher started while a
//!   stopped one is still inside a copy waits for it.
//! - **Privacy.** A copy lives only for the tick that took it; its colour
//!   reaches nothing but the probe test. Every line is `panels: ` and one of
//!   the shapes the Context pins ("Diagnostic and status lines"), which hold
//!   entry ids, reasons, the overlay's place as fractions and counts, never
//!   a colour; an error's detail, which names only the backend call, goes on
//!   its own `error: panel watcher: ` line.

use std::collections::BTreeSet;
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, TryLockError};
use std::thread;
use std::time::{Duration, Instant};

use qt_screen::{
    aspect_is_16_9, overlay_fraction, BackendError, PanelChange, PanelRead,
    PanelTable, PanelTracker, PixelRect, ProbeStats, Rect, ScreenBackend, WindowHandle,
    PANEL_TICK_MS,
};
use qt_viewmodel::StatusLine;

use crate::app::{App, Inbound};
use crate::screen::{CAPTURE_FAILURE_LIMIT, IDLE_SLEEP, MINUTE_MS};

/// The shortest sleep between two ticks, or after finding the backend busy.
const MIN_SLEEP: Duration = Duration::from_millis(5);

/// How far an edge of the overlay's rect (a fraction of the client rect)
/// moves before the watcher writes its place again.
pub const OVERLAY_STEP: f64 = 0.01;

/// The status line while the overlay cannot find its own window: without
/// its rect, the watcher cannot tell which panels are under it.
pub const NO_OVERLAY_STATUS: &str =
    "Hiding under game panels is paused: the overlay cannot find its own window.";

/// The status line when the watcher ended without being stopped.
pub const STOPPED_STATUS: &str =
    "Hiding under game panels stopped after an error; turn it off and on to retry.";

/// What the glue hands the app for the panel watcher, beside its
/// [`crate::AppConfig`] ([`App::with_panels`]).
#[derive(Default)]
pub struct PanelConfig {
    /// The watcher's own backend (`qt-capture` on Windows, built from the
    /// overlay's window handle), never the screen reader's; `None` where
    /// there is none, and then the setting starts nothing.
    pub backend: Option<Box<dyn ScreenBackend>>,
    /// The panels it watches; `None` is [`qt_screen::shipped_panels`]. Tests pass a
    /// synthetic table.
    pub table: Option<PanelTable>,
}

/// The watcher's backend, kept by the app across every watcher it starts
/// (one at a time).
pub(crate) type SharedPanelBackend = Arc<Mutex<Box<dyn ScreenBackend>>>;

/// What the worker publishes for the watcher: whether the overlay is in
/// play mode, whether it is hidden for a reason other than a panel, and the
/// confirmed game pid.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct PanelSnapshot {
    pub play: bool,
    /// Hidden for another reason: another window in front, or no character
    /// in an area yet.
    pub overlay_hidden: bool,
    /// The game's pid, once a live line or its window in front confirmed it.
    pub pid: Option<u32>,
}

/// What the watcher sends the worker, tagged with its generation.
#[derive(Debug)]
pub(crate) enum PanelMsg {
    /// The first open entry under the overlay, in table order, or `None`
    /// when none is (or the watcher can no longer tell).
    Open(Option<&'static str>),
    /// The `Source::Panels` status line, or `None` to clear it.
    Status(Option<StatusLine>),
}

/// A running watcher, as the app keeps it.
pub(crate) struct WatcherHandle {
    pub stop: Arc<AtomicBool>,
    pub handle: thread::JoinHandle<()>,
}

/// Why the watcher reads nothing although it would otherwise read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pause {
    NoOverlay,
    /// The client rect's size, refused for its shape.
    Aspect(i32, i32),
    Capture,
}

impl Pause {
    /// The reason its `paused (<reason>)` line gives.
    fn reason(self) -> String {
        match self {
            Pause::NoOverlay => "no overlay window".to_string(),
            Pause::Aspect(w, h) => format!("the game window is not 16:9 ({w}x{h})"),
            Pause::Capture => {
                format!("the capture failed {CAPTURE_FAILURE_LIMIT} times in a row")
            }
        }
    }

    fn status(self) -> StatusLine {
        StatusLine::warn(match self {
            Pause::NoOverlay => NO_OVERLAY_STATUS.to_string(),
            Pause::Aspect(w, h) => format!(
                "Hiding under game panels is paused while the game's window is not 16:9 ({w}x{h})."
            ),
            Pause::Capture => format!(
                "Hiding under game panels cannot copy the game's window (the capture failed {CAPTURE_FAILURE_LIMIT} times in a row)."
            ),
        })
    }
}

/// What the watcher last wrote about what it does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Activity {
    /// Nothing yet since `panels: on`.
    Unknown,
    Idle(&'static str),
    Paused(Pause),
    Watching,
}

/// The watcher's counts for its per-minute line.
#[derive(Default)]
struct Minute {
    /// Ticks that read the entries under the overlay.
    reads: u64,
    copies: u64,
    /// Reads that ended with an entry open under the overlay.
    open: u64,
    black_copies: u64,
    busy: Duration,
}

/// Runs `f` on the backend when no other watcher holds it; `None`, after a
/// short sleep, when one does (a stopped watcher still inside a call).
fn with_backend<R>(
    backend: &SharedPanelBackend,
    f: impl FnOnce(&mut dyn ScreenBackend) -> R,
) -> Option<R> {
    let mut guard = match backend.try_lock() {
        Ok(g) => g,
        Err(TryLockError::Poisoned(p)) => p.into_inner(),
        Err(TryLockError::WouldBlock) => {
            thread::sleep(MIN_SLEEP);
            return None;
        }
    };
    Some(f(&mut **guard))
}

/// Whether two places differ by at least [`OVERLAY_STEP`] at any edge.
fn moved(a: &Rect, b: &Rect) -> bool {
    [
        (a.left, b.left),
        (a.top, b.top),
        (a.right, b.right),
        (a.bottom, b.bottom),
    ]
    .iter()
    .any(|(x, y)| (x - y).abs() >= OVERLAY_STEP - 1e-9)
}

struct Watcher {
    app: Arc<App>,
    table: Arc<PanelTable>,
    generation: u64,
    stop: Arc<AtomicBool>,
    snapshot: Arc<PanelSnapshot>,
    /// One per table entry.
    trackers: Vec<PanelTracker>,
    /// The entries whose last read was Unseen, and said so: one line per
    /// stretch of them.
    unseen: BTreeSet<usize>,
    activity: Activity,
    /// The overlay's place as last written.
    overlay_at: Option<Rect>,
    /// The entry last sent to the worker.
    open: Option<&'static str>,
    /// Failed copies in a row.
    capture_failures: u32,
    minute: Minute,
    minute_start: u64,
    /// The status line last sent.
    status: Option<StatusLine>,
}

/// The watcher thread's body: runs until `stop` is set or a backend call it
/// cannot go on without fails. A failure or a panic ends it with a warning
/// in the status line, the cause on an `error: panel watcher:` line (a
/// panic's message is not written: it might hold anything), and no panel
/// open.
pub(crate) fn run_watcher(
    app: Arc<App>,
    backend: SharedPanelBackend,
    table: Arc<PanelTable>,
    generation: u64,
    stop: Arc<AtomicBool>,
) {
    let now = app.clock_ms();
    let entries = table.entries.len();
    let mut watcher = Watcher {
        app,
        table,
        generation,
        stop,
        snapshot: Arc::default(),
        trackers: vec![PanelTracker::new(); entries],
        unseen: BTreeSet::new(),
        activity: Activity::Unknown,
        overlay_at: None,
        open: None,
        capture_failures: 0,
        minute: Minute::default(),
        minute_start: now,
        status: None,
    };
    let ended = panic::catch_unwind(AssertUnwindSafe(|| watcher.run(&backend)));
    match ended {
        Ok(Ok(())) => {}
        Ok(Err(e)) => watcher.failed(&e.message),
        Err(_) => watcher.failed("the watcher thread panicked"),
    }
}

impl Watcher {
    fn stopped(&self) -> bool {
        self.stop.load(Ordering::Acquire)
    }

    fn send(&self, msg: PanelMsg) {
        let _ = self.app.sender().send(Inbound::Panels(self.generation, msg));
    }

    fn set_status(&mut self, status: Option<StatusLine>) {
        if self.status != status {
            self.status = status.clone();
            self.send(PanelMsg::Status(status));
        }
    }

    /// Tells the worker which entry is open under the overlay, when that
    /// changed.
    fn send_open(&mut self, open: Option<&'static str>) {
        if self.open != open {
            self.open = open;
            self.send(PanelMsg::Open(open));
        }
    }

    fn run(&mut self, backend: &SharedPanelBackend) -> Result<(), BackendError> {
        self.app.diag("panels: on");
        let period = Duration::from_millis(PANEL_TICK_MS as u64);
        while !self.stopped() {
            let started = Instant::now();
            let Some(watched) = with_backend(backend, |b| self.tick(b)) else {
                continue;
            };
            let watched = watched?;
            if watched {
                self.minute.busy += started.elapsed();
            }
            self.minute_line();
            let wait = if watched {
                // A tick that overran skips the ticks it missed.
                let spent = started.elapsed().as_nanos() % period.as_nanos();
                period - Duration::from_nanos(spent as u64)
            } else {
                IDLE_SLEEP
            };
            self.sleep(wait.max(MIN_SLEEP));
        }
        Ok(())
    }

    /// Sleeps `wait`, in steps of at most [`IDLE_SLEEP`], until the stop.
    fn sleep(&self, wait: Duration) {
        let until = Instant::now() + wait;
        while !self.stopped() {
            let left = until.saturating_duration_since(Instant::now());
            if left.is_zero() {
                break;
            }
            thread::sleep(left.min(IDLE_SLEEP));
        }
    }

    /// The newest snapshot, if the worker is not writing it right now.
    fn refresh_snapshot(&mut self) {
        let newest = match self.app.panel_slot().try_lock() {
            Ok(slot) => Arc::clone(&slot),
            Err(TryLockError::Poisoned(p)) => Arc::clone(&p.into_inner()),
            Err(TryLockError::WouldBlock) => return,
        };
        self.snapshot = newest;
    }

    /// Forgets what was read, and tells the worker no panel is open.
    fn drop_hide(&mut self) {
        for t in &mut self.trackers {
            t.reset();
        }
        self.unseen.clear();
        self.send_open(None);
    }

    /// Idle, for `reason`: a line when the reason differs from the last.
    fn go_idle(&mut self, reason: &'static str) {
        if self.activity != Activity::Idle(reason) {
            self.activity = Activity::Idle(reason);
            self.app.diag(&format!("panels: idle ({reason})"));
        }
        self.drop_hide();
    }

    /// Paused, for `pause`: a line when it differs from the last.
    fn go_paused(&mut self, pause: Pause) {
        if self.activity != Activity::Paused(pause) {
            self.activity = Activity::Paused(pause);
            self.app
                .diag(&format!("panels: paused ({})", pause.reason()));
        }
        self.drop_hide();
    }

    /// Reading: `resumed` after an idle stretch or a pause.
    fn go_watching(&mut self) {
        if matches!(self.activity, Activity::Idle(_) | Activity::Paused(_)) {
            self.app.diag("panels: resumed");
        }
        self.activity = Activity::Watching;
    }

    /// Writes the overlay's place when it moved by [`OVERLAY_STEP`] or more.
    fn note_overlay(&mut self, place: Rect) {
        if self.overlay_at.is_none_or(|at| moved(&at, &place)) {
            self.app.diag(&format!(
                "panels: overlay at x {:.2} to {:.2}, y {:.2} to {:.2}",
                place.left, place.right, place.top, place.bottom
            ));
            self.overlay_at = Some(place);
        }
    }

    /// One tick; whether it read the entries under the overlay.
    fn tick(&mut self, backend: &mut dyn ScreenBackend) -> Result<bool, BackendError> {
        self.refresh_snapshot();
        let watched = self.tick_inner(backend)?;
        let status = match self.activity {
            Activity::Paused(p) => Some(p.status()),
            _ => None,
        };
        self.set_status(status);
        Ok(watched)
    }

    fn tick_inner(&mut self, backend: &mut dyn ScreenBackend) -> Result<bool, BackendError> {
        let snapshot = Arc::clone(&self.snapshot);
        if !snapshot.play {
            self.go_idle("edit mode");
            return Ok(false);
        }
        if snapshot.overlay_hidden {
            self.go_idle("overlay hidden");
            return Ok(false);
        }
        let Some(pid) = snapshot.pid else {
            self.go_idle("pid unconfirmed");
            return Ok(false);
        };
        let Some(window) = backend.game_window(pid)? else {
            self.go_idle("no game window");
            return Ok(false);
        };
        let Some(client) = backend.client_rect(window)? else {
            self.go_idle("minimized");
            return Ok(false);
        };
        // Footprints and probes were measured at 16:9 only.
        if !aspect_is_16_9(client.width(), client.height()) {
            self.go_paused(Pause::Aspect(client.width(), client.height()));
            return Ok(false);
        }
        // Without the overlay's rect there is no telling what is under it.
        let Some(overlay) = backend.overlay()? else {
            self.go_paused(Pause::NoOverlay);
            return Ok(false);
        };
        let place = overlay_fraction(client, overlay.rect);
        let (reads, error) = self.copy_entries(backend, window, client, &place, overlay.visible);
        if self.capture_failures >= CAPTURE_FAILURE_LIMIT {
            self.go_paused(Pause::Capture);
            if let Some(e) = error {
                self.app.diag(&format!("error: panel watcher: {e}"));
            }
            return Ok(true);
        }
        self.go_watching();
        self.note_overlay(place);
        self.minute.reads += 1;
        let under: BTreeSet<usize> = reads.iter().map(|(i, _)| *i).collect();
        for (i, read) in reads {
            let id = self.table.entries[i].id;
            if read == PanelRead::Unseen {
                if self.unseen.insert(i) {
                    self.app
                        .diag(&format!("panels: {id} unseen under the overlay"));
                }
            } else {
                self.unseen.remove(&i);
            }
            match self.trackers[i].feed(read) {
                Some(PanelChange::Opened) => self.app.diag(&format!("panels: {id} open")),
                Some(PanelChange::Closed) => self.app.diag(&format!("panels: {id} closed")),
                None => {}
            }
        }
        // An entry the overlay moved off is judged afresh once it is under
        // it again.
        for (i, t) in self.trackers.iter_mut().enumerate() {
            if !under.contains(&i) {
                t.reset();
                self.unseen.remove(&i);
            }
        }
        let open = under
            .iter()
            .find(|i| self.trackers[**i].is_open())
            .map(|i| self.table.entries[*i].id);
        if open.is_some() {
            self.minute.open += 1;
        }
        self.send_open(open);
        Ok(true)
    }

    /// Copies each readable probe of each entry under the overlay at `place`
    /// and judges the entry; the reads in table order, and the detail of the
    /// failure that made the run of failed copies reach its limit, if one
    /// did now.
    fn copy_entries(
        &mut self,
        backend: &mut dyn ScreenBackend,
        window: WindowHandle,
        client: PixelRect,
        place: &Rect,
        overlay_visible: bool,
    ) -> (Vec<(usize, PanelRead)>, Option<BackendError>) {
        let table = Arc::clone(&self.table);
        let mut reads = Vec::new();
        let mut limit_error = None;
        for (i, entry) in table.under_overlay(place) {
            let mut copies: Vec<Option<ProbeStats>> = vec![None; entry.probes.len()];
            for p in entry.readable_probes(place, overlay_visible) {
                if self.stopped() {
                    break;
                }
                let area = entry.probes[p].pixels(client);
                if area.is_empty() {
                    continue;
                }
                self.minute.copies += 1;
                match backend.capture(window, area) {
                    Ok(image) => {
                        self.capture_failures = 0;
                        let stats = ProbeStats::of_image(&image);
                        if stats.black {
                            self.minute.black_copies += 1;
                        }
                        copies[p] = Some(stats);
                    }
                    Err(e) => {
                        self.capture_failures = self.capture_failures.saturating_add(1);
                        if self.capture_failures == CAPTURE_FAILURE_LIMIT {
                            limit_error = Some(e);
                        }
                    }
                }
            }
            reads.push((i, entry.classify(place, overlay_visible, &copies)));
        }
        (reads, limit_error)
    }

    /// Writes the per-minute line when a minute has passed with any read.
    fn minute_line(&mut self) {
        let now = self.app.clock_ms();
        if now.saturating_sub(self.minute_start) < MINUTE_MS {
            return;
        }
        let m = std::mem::take(&mut self.minute);
        self.minute_start = now;
        if m.reads == 0 {
            return;
        }
        self.app.diag(&format!(
            "panels: minute: {} reads, {} copies, {} open, {} black copies, busy {} ms",
            m.reads,
            m.copies,
            m.open,
            m.black_copies,
            m.busy.as_millis()
        ));
    }

    /// The thread ends after an error: the cause, the line, the warning, and
    /// no panel open.
    fn failed(&mut self, cause: &str) {
        self.app.diag(&format!("error: panel watcher: {cause}"));
        self.app.diag("panels: stopped after an error");
        self.set_status(Some(StatusLine::warn(STOPPED_STATUS)));
        self.drop_hide();
    }
}
