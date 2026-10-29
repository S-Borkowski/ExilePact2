//! The screen backend and the watch thread.
//!
//! [`ScreenBackend`] is what the app needs from the screen, implemented by
//! the glue over the quest tracker's Windows capture and by a fake in the
//! tests: the game's window when it is the window in front, its client
//! area in physical pixels, or why not (another window is in front, the
//! game is minimized); whether Windows' OCR reads English; and a read of
//! the item tooltips in the game's client area (`ch-watch`'s
//! [`ch_watch::Reader`] over the capture's calls). The glue reads the
//! window in front and its title and compares the title with
//! [`GAME_WINDOW_TITLE`] where it reads it; another window's title never
//! reaches this crate. It is optional in [`crate::AppConfig`]: without one
//! the overlay never shows and nothing is read, as in M5.
//!
//! The backend is moved to one watch thread, which `App::start` starts and
//! `App::shutdown` ends within [`WATCH_STOP_DEADLINE`] (a call in flight is
//! not interrupted; past the deadline the shutdown logs `error: watch: did
//! not end within <n> ms` and returns), and is called there alone. The
//! thread calls nothing while no craft is guiding.
//!
//! `App` sends it a demand, without waiting, on every change of whether a
//! craft is guiding and whether to read (the guide active, `screen_watch`
//! on, English OCR not refused), and on every grab and every change it
//! keeps, each with a new generation number and a copy of the item the
//! current step was shown on. While guiding the thread looks at the game's
//! window every [`WINDOW_EVERY`]; while it should read and the game is in
//! front, it reads the tooltips at most every [`READ_EVERY`] (asking once
//! whether the OCR reads English first), runs the matcher
//! ([`ch_watch::classify`]) on the demand's copy of the item, and reports
//! what it saw. A change is confirmed when a later read, at least
//! [`CONFIRM_AFTER`] after the first, gives the same read item (rarity,
//! explicit modifier ids and tiers); it is reported once per generation.
//! Once a minute while guiding it reports its counts and its busy time.
//!
//! `App` drops a report of an older generation, so a grab, which moves the
//! generation on, wins over a read in flight. No backend call, matcher run
//! or sleep happens with `App`'s lock held: a report takes the lock
//! briefly, applies, releases it, and only then calls the shell.
//!
//! With [`ScreenSetup::manual`] the thread looks (and reads, with no rate
//! limit) only when [`crate::App::tick_watch`] asks, and
//! [`ScreenSetup::with_clock`] gives it the clock the confirmation and the
//! minute line count with, so tests and the fixtures decide when the
//! backend is called and how much time passed.
//!
//! Copies, crops and OCR lines stay inside the backend's read; the thread
//! keeps only the matcher's verdict and the read item, and nothing here
//! formats OCR text, a pixel or a window title into a line.

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::Weak;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use ch_engine::ch_item::Item;
use ch_watch::{classify, item_key, ItemKey, ReadCounts, ReadKind, TooltipRead, Unreadable};

use crate::app::App;
use crate::updates::{Clock, SteadyClock};

/// The game's window title (measured 2026-10-28, the waystone roller's
/// Live 3; `docs/RUNTIME_DATA_MODELS.md` § "Item search box (stash and
/// vendor)"). The glue compares the window in front with it.
pub const GAME_WINDOW_TITLE: &str = "Path of Exile 2";

/// How often the watch thread looks at the game's window while a craft is
/// guiding (a starting value; a change is recorded in the research note).
pub const WINDOW_EVERY: Duration = Duration::from_millis(250);

/// The shortest time between two tooltip reads on the thread's own clock
/// (a starting value; a change is recorded in the research note).
pub const READ_EVERY: Duration = Duration::from_millis(500);

/// A changed read is confirmed by a later read of the same item at least
/// this long after the first.
pub const CONFIRM_AFTER: Duration = Duration::from_millis(400);

/// How often the thread reports its counts while guiding (`watch: minute:
/// …`).
pub const MINUTE_EVERY: Duration = Duration::from_secs(60);

/// Black copies in a row after which the watch says it cannot copy the
/// game's window.
pub const BLACK_IN_A_ROW: u32 = 10;

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

/// Whether Windows' OCR reads English.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OcrReadiness {
    /// Ready, with the language tag it reads (`en-US`).
    Ready { language: String },
    /// No English OCR is installed: the watch reads nothing.
    NoEnglish,
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
    /// Whether Windows' OCR reads English. Asked once, before the first
    /// read.
    fn ocr_readiness(&mut self) -> Result<OcrReadiness, WatchFailure>;
    /// The item tooltips in the game's client area `client` (physical
    /// screen pixels) of the window `handle` the last
    /// [`ScreenBackend::game_window`] gave: `ch-watch`'s read, which
    /// reports a tooltip under the overlay's window as covered. Its copies,
    /// crops and lines stay in memory and are dropped after the read.
    fn read_tooltips(
        &mut self,
        handle: isize,
        client: ClientRect,
    ) -> Result<TooltipRead, WatchFailure>;
    /// The reader's counts since they were last taken (copies, tooltips,
    /// OCR calls), for the minute line.
    fn take_counts(&mut self) -> ReadCounts;
}

/// The screen backend [`crate::AppConfig`] takes, how its thread ticks and
/// the clock it counts with.
pub struct ScreenSetup {
    pub(crate) backend: Box<dyn ScreenBackend>,
    pub(crate) manual: bool,
    pub(crate) clock: Box<dyn Clock>,
}

impl ScreenSetup {
    /// The thread ticks on its own clock ([`WINDOW_EVERY`]).
    pub fn new(backend: Box<dyn ScreenBackend>) -> ScreenSetup {
        ScreenSetup {
            backend,
            manual: false,
            clock: Box::new(SteadyClock::new()),
        }
    }

    /// The thread ticks only on [`crate::App::tick_watch`] (tests and the
    /// fixtures), and reads on every tick it may.
    pub fn manual(backend: Box<dyn ScreenBackend>) -> ScreenSetup {
        ScreenSetup {
            manual: true,
            ..ScreenSetup::new(backend)
        }
    }

    /// The clock the confirmation, the read interval and the minute line
    /// count with, in place of the monotonic clock (tests).
    pub fn with_clock(self, clock: Box<dyn Clock>) -> ScreenSetup {
        ScreenSetup { clock, ..self }
    }
}

/// What the thread should do now, from `App`.
#[derive(Debug, Clone)]
pub(crate) struct Demand {
    pub(crate) generation: u64,
    pub(crate) guiding: bool,
    /// Read the tooltips while the game is in front.
    pub(crate) read: bool,
    /// The item the current step was shown on, while reading.
    pub(crate) item: Option<Box<Item>>,
}

impl Demand {
    fn idle() -> Demand {
        Demand {
            generation: 0,
            guiding: false,
            read: false,
            item: None,
        }
    }
}

/// What one read saw, as `App` keeps it for the watch line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Seen {
    NoTooltip,
    Covered,
    AnotherItem,
    Unreadable(Unreadable),
    Unchanged,
    /// The item, changed: the read item's key.
    Changed(ItemKey),
    /// [`BLACK_IN_A_ROW`] black copies in a row, or more.
    Black,
}

impl Seen {
    /// Its `diag.log` line.
    pub(crate) fn diag_line(&self) -> String {
        let state = match self {
            Seen::Black => return format!("watch: black: {BLACK_IN_A_ROW} copies in a row"),
            Seen::NoTooltip => "none",
            Seen::Covered => "covered by the overlay",
            Seen::AnotherItem => "another item",
            Seen::Unreadable(_) => "unreadable",
            Seen::Unchanged => "unchanged",
            Seen::Changed(_) => "changed",
        };
        format!("watch: tooltip {state}")
    }
}

/// What the thread tells `App`, with the demand's generation.
#[derive(Debug)]
pub(crate) enum Report {
    /// A look at the game's window.
    Window(Result<GameWindow, WatchFailure>),
    /// Whether the OCR reads English.
    Ocr(Result<OcrReadiness, WatchFailure>),
    /// A read: what it saw and, when a change was read twice alike, the
    /// read item and its key.
    Read {
        seen: Seen,
        confirmed: Option<(Box<Item>, ItemKey)>,
    },
    /// The read's call failed.
    ReadFailed(WatchFailure),
    /// A minute's counts while guiding.
    Minute { counts: ReadCounts, busy_ms: u64 },
}

pub(crate) enum WatchMsg {
    Demand(Demand),
    /// One tick now (manual ticks), acknowledged once its reports applied.
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
    let manual = setup.manual;
    let handle = std::thread::Builder::new()
        .name("ch-watch".to_string())
        .spawn(move || run(app, rx, setup))?;
    Ok(Watch { tx, handle, manual })
}

/// The thread: waits for a demand; while guiding, ticks each
/// [`WINDOW_EVERY`] (or on each manual tick). Ends when `App` drops its
/// sender or is gone.
fn run(app: Weak<App>, rx: Receiver<WatchMsg>, setup: ScreenSetup) {
    let ScreenSetup {
        backend,
        manual,
        clock,
    } = setup;
    let mut w = Watcher {
        app,
        backend,
        clock,
        manual,
        demand: Demand::idle(),
        ocr: None,
        pending: None,
        reported: None,
        blacks: 0,
        last_read: None,
        minute: None,
        busy: Duration::ZERO,
    };
    let mut next: Option<Instant> = None;
    loop {
        let msg = match next {
            Some(at) if !manual && w.demand.guiding => {
                rx.recv_timeout(at.saturating_duration_since(Instant::now()))
            }
            _ => rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
        };
        match msg {
            Ok(WatchMsg::Demand(d)) => {
                next = match (w.demand.guiding, d.guiding) {
                    (_, false) => None,
                    (false, true) => Some(Instant::now()),
                    (true, true) => next,
                };
                w.set_demand(d);
            }
            Ok(WatchMsg::Tick(ack)) => {
                if w.demand.guiding && !w.tick() {
                    return;
                }
                let _ = ack.send(());
                continue;
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        if manual || !w.demand.guiding {
            continue;
        }
        if next.is_some_and(|at| Instant::now() >= at) {
            if !w.tick() {
                return;
            }
            next = Some(Instant::now() + WINDOW_EVERY);
        }
    }
}

/// The thread's state.
struct Watcher {
    app: Weak<App>,
    backend: Box<dyn ScreenBackend>,
    clock: Box<dyn Clock>,
    manual: bool,
    demand: Demand,
    /// The OCR reads English (`Some(true)`) or not, once asked.
    ocr: Option<bool>,
    /// A change read once in this generation: its key and when (clock ms).
    pending: Option<(ItemKey, u64)>,
    /// The change already reported in this generation.
    reported: Option<ItemKey>,
    /// Black copies in a row.
    blacks: u32,
    /// The last read (clock ms), for [`READ_EVERY`].
    last_read: Option<u64>,
    /// When the current minute began (clock ms), while guiding.
    minute: Option<u64>,
    /// Time spent in backend calls and the matcher this minute.
    busy: Duration,
}

fn ms(d: Duration) -> u64 {
    u64::try_from(d.as_millis()).unwrap_or(u64::MAX)
}

impl Watcher {
    fn set_demand(&mut self, d: Demand) {
        if d.generation != self.demand.generation {
            self.pending = None;
            self.reported = None;
        }
        if !d.guiding {
            self.minute = None;
            self.last_read = None;
            self.blacks = 0;
        }
        self.demand = d;
    }

    /// One tick while guiding: a look at the game's window, a read when one
    /// is wanted and due, the minute line when a minute passed. False when
    /// the app is gone.
    fn tick(&mut self) -> bool {
        let now = self.clock.now_ms();
        if self.minute.is_none() {
            // The counts of an earlier craft are not this minute's.
            let _ = self.backend.take_counts();
            self.busy = Duration::ZERO;
            self.minute = Some(now);
        }
        let started = Instant::now();
        let window = self.backend.game_window();
        self.busy += started.elapsed();
        let front = match &window {
            Ok(GameWindow::InFront { handle, client }) => Some((*handle, *client)),
            _ => None,
        };
        if !self.report(Report::Window(window)) {
            return false;
        }
        if let Some((handle, client)) = front {
            let due = self.manual
                || self
                    .last_read
                    .is_none_or(|at| now.saturating_sub(at) >= ms(READ_EVERY));
            if self.demand.read && due && !self.read(now, handle, client) {
                return false;
            }
        }
        if let Some(start) = self.minute {
            if now.saturating_sub(start) >= ms(MINUTE_EVERY) {
                let counts = self.backend.take_counts();
                let busy_ms = ms(std::mem::take(&mut self.busy));
                self.minute = Some(now);
                return self.report(Report::Minute { counts, busy_ms });
            }
        }
        true
    }

    /// One read of the tooltips, asking first whether the OCR reads
    /// English; the matcher's verdict reported. False when the app is gone.
    fn read(&mut self, now: u64, handle: isize, client: ClientRect) -> bool {
        if self.ocr.is_none() {
            let started = Instant::now();
            let ready = self.backend.ocr_readiness();
            self.busy += started.elapsed();
            if let Ok(r) = &ready {
                self.ocr = Some(matches!(r, OcrReadiness::Ready { .. }));
            }
            if !self.report(Report::Ocr(ready)) {
                return false;
            }
        }
        if self.ocr != Some(true) {
            return true;
        }
        let Some(item) = self.demand.item.as_deref() else {
            return true;
        };
        self.last_read = Some(now);
        let started = Instant::now();
        let read = match self.backend.read_tooltips(handle, client) {
            Ok(read) => read,
            Err(failure) => {
                self.busy += started.elapsed();
                return self.report(Report::ReadFailed(failure));
            }
        };
        self.blacks = if read == TooltipRead::Black {
            self.blacks.saturating_add(1)
        } else {
            0
        };
        let kind = classify(&read, item);
        drop(read);
        let (seen, confirmed) = self.judge(kind, now);
        self.busy += started.elapsed();
        self.report(Report::Read { seen, confirmed })
    }

    /// What a read saw, and the change it confirms: a change read again at
    /// least [`CONFIRM_AFTER`] after it was first read, not yet reported in
    /// this generation.
    fn judge(&mut self, kind: ReadKind, now: u64) -> (Seen, Option<(Box<Item>, ItemKey)>) {
        let item = match kind {
            ReadKind::Changed(item) => item,
            other => {
                self.pending = None;
                let seen = match other {
                    ReadKind::NoTooltip if self.blacks >= BLACK_IN_A_ROW => Seen::Black,
                    ReadKind::NoTooltip => Seen::NoTooltip,
                    ReadKind::Covered => Seen::Covered,
                    ReadKind::AnotherItem => Seen::AnotherItem,
                    ReadKind::Unreadable(why) => Seen::Unreadable(why),
                    _ => Seen::Unchanged,
                };
                return (seen, None);
            }
        };
        // The read item joins by construction; one that does not cannot be
        // told from another.
        let Some(key) = item_key(&item) else {
            self.pending = None;
            return (Seen::Unreadable(Unreadable::Ambiguous), None);
        };
        if self.reported.as_ref() == Some(&key) {
            return (Seen::Changed(key), None);
        }
        match &self.pending {
            Some((k, at)) if *k == key => {
                if now.saturating_sub(*at) >= ms(CONFIRM_AFTER) {
                    self.pending = None;
                    self.reported = Some(key.clone());
                    (Seen::Changed(key.clone()), Some((item, key)))
                } else {
                    (Seen::Changed(key), None)
                }
            }
            _ => {
                self.pending = Some((key.clone(), now));
                (Seen::Changed(key), None)
            }
        }
    }

    /// Hands `report` to the app; false when the app is gone.
    fn report(&self, report: Report) -> bool {
        match self.app.upgrade() {
            Some(app) => {
                app.watch_report(self.demand.generation, report);
                true
            }
            None => false,
        }
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

// ---------------------------------------------------------------------------
// The watch line (`ui-design.md` § "The screen watch's line"): Rust's copy.

pub const WATCHING_LINE: &str = "Watching: after the step, hover the item in the game.";
pub const UNCHANGED_LINE: &str = "Watching: the item has not changed yet.";
pub const ANOTHER_ITEM_LINE: &str =
    "Watching: that is another item; hover the one you are crafting.";
pub const CHECKING_LINE: &str = "Checking the change: keep hovering the item.";
pub const COVERED_LINE: &str =
    "The overlay covers the item: move it in Settings, or grab the item.";
pub const UNEXPECTED_LINE: &str =
    "The item changed in a way the guide did not expect: grab it to check.";
pub const BLACK_LINE: &str = "Can't copy the game's window: grab the item after each step.";
pub const NO_OCR_LINE: &str =
    "Can't read the screen (no English OCR): grab the item after each step.";
pub const WATCH_OFF_LINE: &str = "Screen reading is off: grab the item after each step.";
pub const NO_BACKEND_LINE: &str = "Screen reading is not available: grab the item after each step.";
pub const NOT_IN_FRONT_LINE: &str = "Paused: the game is not in front.";
pub const MINIMIZED_LINE: &str = "Paused: the game is minimized.";
pub const STUCK_LINE: &str = "Paused: the guide is stuck.";

/// The unreadable line: `Can't read the item (<reason>): hover it without
/// Alt, or grab it.` The plain tooltip reads best: with Alt held OCR lost
/// some values in the owner's shots.
pub fn unreadable_line(why: Unreadable) -> String {
    format!(
        "Can't read the item ({}): hover it without Alt, or grab it.",
        unreadable_words(why)
    )
}

/// The matcher's reason in the watch line's words.
pub fn unreadable_words(why: Unreadable) -> &'static str {
    match why {
        Unreadable::UnaccountedLine => "a line it does not know",
        Unreadable::MissingLine => "a line is missing",
        Unreadable::AmbiguousTier => "two tiers fit",
        Unreadable::Ambiguous => "the lines read more than one way",
        Unreadable::Rarity => "the rarity is unclear",
        Unreadable::NoBaseType => "the item has no base type",
        Unreadable::NoItemLevel => "the item has no item level",
        Unreadable::Cut => "it may be cut off at the window's edge",
        Unreadable::SeveralTooltips => "two tooltips show the item",
        Unreadable::NotEstablished => "an unrevealed or fractured modifier",
        Unreadable::UnknownModifier => "a modifier it does not know",
    }
}
