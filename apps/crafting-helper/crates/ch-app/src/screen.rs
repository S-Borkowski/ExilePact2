//! The screen backend and the watch thread.
//!
//! [`ScreenBackend`] is what the app needs from the screen, implemented by
//! the glue over the quest tracker's Windows capture and by a fake in the
//! tests: the game's window when it is the window in front, its client
//! area in physical pixels, or why not (another window is in front, the
//! game is minimized). The glue reads the window in front and its title
//! and compares the title with [`GAME_WINDOW_TITLE`] where it reads it;
//! another window's title never reaches this crate. It is optional in
//! [`crate::AppConfig`]: without one the overlay never shows and nothing is
//! read, as in M5.
//!
//! The backend is moved to one watch thread, which `App::start` starts and
//! `App::shutdown` ends within [`WATCH_STOP_DEADLINE`] (a call in flight is
//! not interrupted; past the deadline the shutdown logs `error: watch: did
//! not end within <n> ms` and returns), and is called there alone. The
//! thread calls nothing while no craft is guiding. `App` sends it a demand,
//! without waiting, each time whether a craft is guiding changes, with a
//! generation number; while guiding the thread looks at the game's window
//! every [`WINDOW_EVERY`] and reports to `App`, which drops a report of an
//! older generation. No backend call or sleep happens with `App`'s lock
//! held: a report takes the lock briefly, applies, releases it, and only
//! then calls the shell.
//!
//! With [`ScreenSetup::manual`] the thread looks only when
//! [`crate::App::tick_watch`] asks, so tests and the fixtures decide when
//! the backend is called.

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::Weak;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::app::App;

/// The game's window title (measured 2026-10-28, the waystone roller's
/// Live 3; `docs/RUNTIME_DATA_MODELS.md` § "Item search box (stash and
/// vendor)"). The glue compares the window in front with it.
pub const GAME_WINDOW_TITLE: &str = "Path of Exile 2";

/// How often the watch thread looks at the game's window while a craft is
/// guiding (a starting value; a change is recorded in the research note).
pub const WINDOW_EVERY: Duration = Duration::from_millis(250);

/// How long `App::shutdown` waits for the watch thread to end.
pub const WATCH_STOP_DEADLINE: Duration = Duration::from_secs(2);

/// A window's client area on the screen, in physical pixels (`right` and
/// `bottom` are exclusive).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl ClientRect {
    pub fn width(&self) -> u32 {
        (i64::from(self.right) - i64::from(self.left)).clamp(0, i64::from(u32::MAX)) as u32
    }

    pub fn height(&self) -> u32 {
        (i64::from(self.bottom) - i64::from(self.top)).clamp(0, i64::from(u32::MAX)) as u32
    }
}

/// Where the game's window is, as the backend found it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameWindow {
    /// The game is the window in front: its handle (for the backend's own
    /// later reads of it) and its client area.
    InFront { handle: isize, client: ClientRect },
    /// Another window is in front.
    NotInFront,
    /// The game is in front but minimized: it has no client area.
    Minimized,
}

/// A system call of the backend failed: which, and the system's error.
/// Logged as `error: watch: <call> failed: <error>`, once until a call
/// succeeds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchFailure {
    pub call: String,
    pub error: String,
}

/// What the app needs from the screen. `Send`: it is moved to the watch
/// thread and called there alone, never with the app's lock held.
pub trait ScreenBackend: Send {
    /// The game's window if it is the window in front, or why not.
    fn game_window(&mut self) -> Result<GameWindow, WatchFailure>;
}

/// The screen backend [`crate::AppConfig`] takes, and how its thread ticks.
pub struct ScreenSetup {
    pub(crate) backend: Box<dyn ScreenBackend>,
    pub(crate) manual: bool,
}

impl ScreenSetup {
    /// The thread ticks on its own clock ([`WINDOW_EVERY`]).
    pub fn new(backend: Box<dyn ScreenBackend>) -> ScreenSetup {
        ScreenSetup {
            backend,
            manual: false,
        }
    }

    /// The thread ticks only on [`crate::App::tick_watch`] (tests and the
    /// fixtures).
    pub fn manual(backend: Box<dyn ScreenBackend>) -> ScreenSetup {
        ScreenSetup {
            backend,
            manual: true,
        }
    }
}

/// What the thread should do now, from `App`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Demand {
    pub(crate) generation: u64,
    pub(crate) guiding: bool,
}

pub(crate) enum WatchMsg {
    Demand(Demand),
    /// One tick now (manual ticks), acknowledged once its report applied.
    Tick(Sender<()>),
}

/// The watch thread and the way to it; dropping `tx` ends the thread.
pub(crate) struct Watch {
    pub(crate) tx: Sender<WatchMsg>,
    pub(crate) handle: JoinHandle<()>,
    pub(crate) manual: bool,
}

/// Starts the watch thread over `setup`'s backend.
pub(crate) fn spawn(app: Weak<App>, setup: ScreenSetup) -> std::io::Result<Watch> {
    let (tx, rx) = mpsc::channel::<WatchMsg>();
    let ScreenSetup { backend, manual } = setup;
    let handle = std::thread::Builder::new()
        .name("ch-watch".to_string())
        .spawn(move || run(app, rx, backend, manual))?;
    Ok(Watch { tx, handle, manual })
}

/// The thread: waits for a demand; while guiding, looks at the game's
/// window each [`WINDOW_EVERY`] (or on each manual tick) and reports. Ends
/// when `App` drops its sender or is gone.
fn run(app: Weak<App>, rx: Receiver<WatchMsg>, mut backend: Box<dyn ScreenBackend>, manual: bool) {
    let mut demand = Demand {
        generation: 0,
        guiding: false,
    };
    let mut next: Option<Instant> = None;
    loop {
        let msg = match next {
            Some(at) if !manual && demand.guiding => {
                rx.recv_timeout(at.saturating_duration_since(Instant::now()))
            }
            _ => rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
        };
        match msg {
            Ok(WatchMsg::Demand(d)) => {
                next = match (demand.guiding, d.guiding) {
                    (_, false) => None,
                    (false, true) => Some(Instant::now()),
                    (true, true) => next,
                };
                demand = d;
            }
            Ok(WatchMsg::Tick(ack)) => {
                if demand.guiding && !look(&app, backend.as_mut(), demand.generation) {
                    return;
                }
                let _ = ack.send(());
                continue;
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        if manual || !demand.guiding {
            continue;
        }
        if next.is_some_and(|at| Instant::now() >= at) {
            if !look(&app, backend.as_mut(), demand.generation) {
                return;
            }
            next = Some(Instant::now() + WINDOW_EVERY);
        }
    }
}

/// One look at the game's window, reported to the app; false when the app
/// is gone.
fn look(app: &Weak<App>, backend: &mut dyn ScreenBackend, generation: u64) -> bool {
    let window = backend.game_window();
    match app.upgrade() {
        Some(app) => {
            app.window_report(generation, window);
            true
        }
        None => false,
    }
}

/// Joins `handle` if it ends before `deadline`.
pub(crate) fn join_by(handle: JoinHandle<()>, deadline: Instant) -> bool {
    while !handle.is_finished() {
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    handle.join().is_ok()
}
