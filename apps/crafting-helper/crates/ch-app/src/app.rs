//! [`App`]: the state behind one lock, the commands, the hotkey, the grab
//! and the overlay's state.
//!
//! The grab ([`crate::grab`]): a press of the global hotkey reads the
//! clipboard exactly once ([`Shell::read_clipboard`], from
//! [`App::hotkey_pressed`]); a paste in the main window sends its text with
//! `grab_text` ([`App::grab_text`]) and reads no clipboard. Nothing else
//! reads the clipboard: not the start, a timer, a focus change or another
//! command. A grab that passes replaces the last one and sets the status;
//! a refused one keeps the last grab, sets an error status and, from the
//! hotkey alone (the window may be hidden), beeps. Each grab writes one
//! `grab from the shortcut: N lines`, `grab from a paste: N lines` or
//! `grab refused: <reason>` line to `diag.log`, never the text; the text is
//! kept in memory only and never saved.
//!
//! The hotkey ([`crate::hotkey`]): [`DEFAULT_HOTKEY`] on a first launch;
//! `set_hotkey` refuses an empty key and the game's own Ctrl+Alt+C in any
//! spelling before it asks the shell, and keeps the current one. A saved
//! Ctrl+Alt+C falls back to the default at launch, and says so. A key
//! another program holds is reported and the app keeps running.
//!
//! The craft window ([`crate::craft`]): a grab that passes also goes to the
//! craft, where it becomes the planning item, is refused with the reason,
//! or, while guiding, is the result of the current step (`craft: <hit|miss|
//! unexpected> at <step id>` in `diag.log`). The commands `set_targets`,
//! `set_craft_blocked`, `choose_route`, `accept_craft` and `end_craft` each
//! return the [`View`]; a refused one changes nothing, says why in the
//! status line and logs `craft: refused: <reason>`. `open_craft_of_exile`
//! asks the shell to open [`CRAFT_OF_EXILE_URL`] and nothing else. The
//! shipped blueprints are loaded once, when the app is built.
//!
//! The overlay over the game ([`crate::overlay`]): [`View::overlay`] is
//! visible while the craft is guiding, in any guide status, and the screen
//! backend ([`crate::screen`], optional in [`AppConfig`]) last reported the
//! game's window in front; hidden otherwise, and at once when guiding ends.
//! Its content is the guide's current step and last outcome, present
//! whenever the craft is guiding. The app places it through the shell
//! ([`Shell::place_overlay`]) at the `overlay_position` setting inside the
//! game's client area, again when that area, the setting or the overlay's
//! size changes, then shows or hides it ([`Shell::set_overlay_visible`]).
//! Without a backend it never shows. The watch thread looks at the game's
//! window only while the craft is guiding; `App` sends it each change of
//! that, and of whether to read, without waiting, and never waits on it
//! with its lock held.
//!
//! The screen watch ([`crate::screen`]): while the guide is active,
//! `screen_watch` is on and the game is in front, the thread reads the
//! hovered item's tooltip and reports what it saw (`watch: tooltip <state>`
//! in `diag.log` when it changes). A change read twice alike, for the
//! current generation, is classified by the engine on a copy of the guide:
//! a hit or a miss is kept as a grab's is, with the screen as its history
//! entry's source and the read item (no copied text) as the item now
//! (`craft: <hit|miss> at <step id> (from the screen)`); an unexpected
//! change keeps nothing, says to grab the item, and is not applied again
//! (`watch: unexpected change at <step id> not applied`). Every grab and
//! every command moves the generation on, so a read in flight is dropped:
//! a grab wins. A grab after a screen read whose rarity and explicit
//! modifiers (ids and tiers) are the read item's is not an outcome: it
//! replaces the read item (`craft: grab matches the screen`). No English
//! OCR (`watch: refused: no English OCR`), `screen_watch` off or no backend
//! reads nothing, and grabs drive the guide as in M5. The guiding view and
//! the overlay carry the same watch line ([`WatchLineView`]): what the
//! watch sees and what to do.
//!
//! The hotkey, `check_updates`, the blocked currencies, `overlay_position`
//! and `screen_watch` are saved to
//! `settings.json` by a saver thread, [`SAVE_DELAY`] after the last change
//! (debounced), and at once on [`App::flush`] and [`App::shutdown`].
//!
//! The update check and install (`ep-update`'s [`Updater`], behind a lock of
//! its own that is never taken with the app's held): a check from the
//! `check_for_updates` command or, with `check_updates` on, from
//! [`App::tick_updates`] (see [`crate::updates`]); an install only from the
//! `install_update` command. A check or an install runs on a thread of its
//! own and is never waited for; the Settings window hears every change as
//! `ch:app-settings` ([`Shell::emit_app_settings`]), and the main window's
//! view names a newer version found ([`View::update_available`]). A
//! finished install asks the shell to quit ([`Shell::restart`]) through the
//! app's own quit path, whose shutdown saves; the glue then starts the new
//! exe ([`App::restart_requested`]).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, MutexGuard, Weak};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use ep_update::{
    outcome_line, started_line, turned_line, Answer, Begin, InstallEvents, TransportError, Updater,
    Version,
};
use serde_json::{json, Value};

use ch_engine::ch_item::Item;
use ch_engine::{Engine, Target};
use ch_watch::ItemKey;

use crate::craft::{CommandResult, Craft, GrabEffect, ScreenEffect, CRAFT_OF_EXILE_URL};
use crate::grab::{self, Refusal};
use crate::hotkey::{is_game_item_copy, DEFAULT_HOTKEY};
use crate::overlay::{overlay_guide, overlay_origin};
use crate::screen::{
    self, unreadable_line, ClientRect, Demand, GameWindow, OcrReadiness, Report, ScreenSetup, Seen,
    Watch, WatchFailure, WatchMsg, ANOTHER_ITEM_LINE, BLACK_LINE, CHECKING_LINE, COVERED_LINE,
    MINIMIZED_LINE, NOT_IN_FRONT_LINE, NO_BACKEND_LINE, NO_OCR_LINE, STUCK_LINE, UNCHANGED_LINE,
    UNEXPECTED_LINE, WATCHING_LINE, WATCH_OFF_LINE, WATCH_STOP_DEADLINE,
};
use crate::settings::{
    blocked_list, catalogue_groups, load_blocked, load_overlay_position, load_screen_watch,
    read_patch, CatalogueGroup, BLOCKED_CURRENCIES_KEY, CHECK_UPDATES_KEY, OVERLAY_POSITION_KEY,
    SCREEN_WATCH_KEY,
};
use crate::store::{Store, StoreError, SETTINGS_SCHEMA};
use crate::updates::{Clock, UpdateSetup};
use crate::view::{
    CraftMode, GrabSource, GrabView, GuideStatusView, OutcomeWord, OverlayPosition, OverlayView,
    SettingsView, StatusKind, StatusLine, View, WatchLineView,
};

/// How long after the last change the settings are written.
pub const SAVE_DELAY: Duration = Duration::from_millis(500);

/// Why a hotkey was not registered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HotkeyError {
    /// Another program holds the combination (Windows'
    /// `ERROR_HOTKEY_ALREADY_REGISTERED`).
    Taken(String),
    /// The text is not a hotkey.
    Invalid(String),
    /// Any other failure.
    Other(String),
}

impl HotkeyError {
    fn detail(&self) -> &str {
        match self {
            HotkeyError::Taken(e) | HotkeyError::Invalid(e) | HotkeyError::Other(e) => e,
        }
    }
}

/// One sound the shell tried for the error tone, and what the system call
/// returned. "ok" says the call ran, not that anyone heard it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeepAttempt {
    /// Which sound, as the diagnostic line names it.
    pub kind: String,
    /// The system's error when the call failed.
    pub result: Result<(), String>,
}

impl BeepAttempt {
    pub fn ok(kind: impl Into<String>) -> BeepAttempt {
        BeepAttempt {
            kind: kind.into(),
            result: Ok(()),
        }
    }

    pub fn failed(kind: impl Into<String>, error: impl Into<String>) -> BeepAttempt {
        BeepAttempt {
            kind: kind.into(),
            result: Err(error.into()),
        }
    }

    /// The diagnostic line: `refusal beep: <kind> ok|failed (<error>)`.
    fn diag_line(&self) -> String {
        match &self.result {
            Ok(()) => format!("refusal beep: {} ok", self.kind),
            Err(e) => format!("refusal beep: {} failed ({e})", self.kind),
        }
    }
}

/// What the app needs from outside: implemented by the Tauri glue, and by a
/// recording fake in the tests.
pub trait Shell: Send + Sync {
    /// Registers `hotkey` as the one global hotkey, replacing the one
    /// registered now. When refused, the one registered before stays.
    fn register_hotkey(&self, hotkey: &str) -> Result<(), HotkeyError>;
    /// Reads the clipboard's text once: `Ok(None)` when it holds no text,
    /// `Err` with the reason when it could not be read (the glue retries a
    /// clipboard another program holds for about 100 ms first). Called
    /// from [`App::hotkey_pressed`] alone, once per press.
    fn read_clipboard(&self) -> Result<Option<String>, String>;
    /// The error tone. Returns each sound tried, in order (a fallback
    /// follows a failed call); empty where the platform has none.
    fn beep(&self) -> Vec<BeepAttempt>;
    /// Sends the view to the main and overlay windows (`ch:view`).
    fn emit_view(&self, view: &View);
    /// Sends the Settings window's view to it (`ch:app-settings`), from
    /// whichever thread made the change, with no lock of the app held.
    fn emit_app_settings(&self, _view: &SettingsView) {}
    /// Shows the overlay window without activating it, or hides it. Posted
    /// to the main thread and never waited on: called from the watch thread
    /// and the commands, with no lock of the app held.
    fn set_overlay_visible(&self, visible: bool);
    /// The overlay window's size in physical pixels (width, height), for
    /// its placement. Called from the watch thread and the commands with no
    /// lock of the app held; it must not wait on the main thread.
    fn overlay_size(&self) -> Result<(u32, u32), String> {
        Err("this shell has no overlay window".to_string())
    }
    /// Moves the overlay window's top-left corner to `(x, y)`, physical
    /// pixels on the screen, and nothing else. Posted to the main thread and
    /// never waited on, as [`Shell::set_overlay_visible`] is.
    fn place_overlay(&self, _x: i32, _y: i32) {}
    /// Opens the Settings window, or brings it up when it is open.
    fn open_settings(&self) -> Result<(), String> {
        Err("this shell has no Settings window".to_string())
    }
    /// Opens `address` in the default browser. Called from
    /// [`App::open_craft_of_exile`] alone, always with
    /// [`CRAFT_OF_EXILE_URL`]; no command takes an address.
    fn open_in_browser(&self, _address: &str) -> Result<(), String> {
        Err("this shell cannot open a web browser".to_string())
    }
    /// A new exe is in place: quit through the app's own quit path (whose
    /// shutdown calls [`App::shutdown`]); the shell starts the new exe once
    /// the app has shut down, because [`App::restart_requested`] says so.
    fn restart(&self) {}
}

/// What [`App::new`] is built from.
pub struct AppConfig {
    pub store: Result<Store, StoreError>,
    pub shell: Box<dyn Shell>,
    /// The game's window (and the screen reads); `None`: the overlay never
    /// shows and nothing is read.
    pub screen: Option<ScreenSetup>,
}

/// The game's window, as the watch thread last reported it while guiding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Game {
    /// Not looked at since guiding began (or no backend).
    Unknown,
    NotInFront,
    Minimized,
    InFront(ClientRect),
    /// The backend's call failed.
    Failed,
}

/// The overlay window and the watch as the app last told the shell, the log
/// and the thread.
#[derive(Debug)]
struct ScreenState {
    /// The game's window, from the last report of this generation.
    game: Game,
    /// The shell was last told to show the overlay.
    shown: bool,
    /// The last placement: the position, the client area and the size.
    placed: Option<(OverlayPosition, ClientRect, (u32, u32))>,
    /// The last `watch:` state line written.
    watch_line: String,
    /// The game's window call that failed and was logged, by name; a
    /// success clears it.
    window_failed: Option<String>,
    /// The overlay's size was unknown, and that was logged.
    size_failed: bool,
    /// The last demand sent to the thread (whether guiding, whether to
    /// read), and its number; a report of another number is dropped.
    demand: Option<(bool, bool)>,
    generation: u64,
    /// A screen backend was given.
    has_screen: bool,
    /// The OCR reads English (`Some(true)`) or not, once the thread asked.
    ocr: Option<bool>,
    /// What the last read of this generation saw.
    seen: Option<Seen>,
    /// A change the guide did not expect, read and not applied: its key,
    /// never applied again until the next grab.
    unexpected: Option<ItemKey>,
    /// The read or OCR call that failed and was logged, by name; a success
    /// clears it.
    read_failed: Option<String>,
    /// The last `watch: idle (guide not active|screen watch off)` line
    /// written while guiding, and the last `watch: tooltip …` (or black)
    /// line; both forgotten when guiding ends.
    read_line: Option<String>,
    tooltip_line: Option<String>,
}

impl Default for ScreenState {
    fn default() -> ScreenState {
        ScreenState {
            game: Game::Unknown,
            shown: false,
            placed: None,
            watch_line: idle_line(NOT_GUIDING),
            window_failed: None,
            size_failed: false,
            demand: None,
            generation: 0,
            has_screen: false,
            ocr: None,
            seen: None,
            unexpected: None,
            read_failed: None,
            read_line: None,
            tooltip_line: None,
        }
    }
}

const NOT_GUIDING: &str = "not guiding";
const NOT_IN_FRONT: &str = "game not in front";

fn idle_line(reason: &str) -> String {
    format!("watch: idle ({reason})")
}

/// The main window's status line, and whether it is about the hotkey (a
/// hotkey that registers clears it).
#[derive(Debug, Clone)]
struct Status {
    line: StatusLine,
    about_hotkey: bool,
}

#[derive(Debug)]
struct State {
    hotkey: String,
    hotkey_status: Option<StatusLine>,
    grab: Option<GrabView>,
    status: Option<Status>,
    /// "Automatically check for updates"; off unless the saved settings say
    /// `true`.
    check_updates: bool,
    blocked_currencies: Vec<String>,
    /// Where the overlay sits over the game.
    overlay_position: OverlayPosition,
    /// Read the hovered item from the screen while guiding; on unless the
    /// saved settings say `false`.
    screen_watch: bool,
    /// The craft window's state, in memory only.
    craft: Craft,
    /// The overlay window and the watch.
    screen: ScreenState,
}

impl State {
    fn guiding(&self) -> bool {
        self.craft.mode() == CraftMode::Guiding
    }

    /// The overlay should show: guiding, with the game's window in front.
    fn overlay_wanted(&self) -> bool {
        self.guiding() && matches!(self.screen.game, Game::InFront(_))
    }

    /// The watch should read the tooltips (while the game is in front): a
    /// screen backend, an active guide, `screen_watch` on and English OCR
    /// not refused.
    fn reads(&self) -> bool {
        self.screen.has_screen
            && self.craft.guide_status() == Some(GuideStatusView::Active)
            && self.screen_watch
            && self.screen.ocr != Some(false)
    }

    /// Why the watch reads nothing while guiding with the game's window
    /// looked at, for its `watch: idle (<reason>)` line.
    fn read_idle(&self) -> Option<&'static str> {
        match self.craft.guide_status() {
            None => None,
            Some(GuideStatusView::Active) if self.screen_watch => None,
            Some(GuideStatusView::Active) => Some("screen watch off"),
            Some(_) => Some("guide not active"),
        }
    }
}

enum SaveMsg {
    Save(Value),
    Flush(Sender<()>),
}

/// The saver thread and the way to it.
struct Saver {
    tx: Sender<SaveMsg>,
    thread: JoinHandle<()>,
}

/// The app: built once, shared by the glue as `Arc<App>`.
pub struct App {
    /// The app itself, for the threads it starts (which never keep it alive).
    me: Weak<App>,
    shell: Box<dyn Shell>,
    store: Option<Arc<Store>>,
    state: Mutex<State>,
    saver: Mutex<Option<Saver>>,
    /// The saved hotkey was the game's item copy and was replaced by the
    /// default: [`App::start`] saves the default.
    hotkey_replaced: bool,
    /// The update check and install, behind a lock of their own.
    updater: Updater,
    clock: Box<dyn Clock>,
    tick_every: Option<Duration>,
    /// The way to the ticker thread; dropping it stops the thread.
    ticker: Mutex<Option<Sender<()>>>,
    /// A finished install asked the shell to quit and start the new exe.
    restart_requested: AtomicBool,
    /// The shipped blueprints, loaded once; `None` when they did not load
    /// (then every grab is refused with that reason).
    engine: Option<Engine>,
    /// The screen backend, until [`App::start`] moves it to the watch
    /// thread.
    screen_setup: Mutex<Option<ScreenSetup>>,
    /// A screen backend was given: the overlay and the watch are in play.
    has_screen: bool,
    /// The watch thread, while it runs.
    watch: Mutex<Option<Watch>>,
    /// Keeps the overlay's shell calls in the order their decisions were
    /// made; never taken with the state's lock held.
    overlay_order: Mutex<()>,
    /// [`App::shutdown`] began: a report from the watch changes nothing.
    stopped: AtomicBool,
}

impl App {
    /// [`App::with_updates`] with the default [`UpdateSetup`]: no transport,
    /// so a check ends failed without a request and an install is refused.
    pub fn new(config: AppConfig) -> Arc<App> {
        App::with_updates(config, UpdateSetup::default())
    }

    /// Builds the app: reads the saved settings, makes the updater and
    /// starts the saver. Registers nothing, reads no clipboard, emits
    /// nothing and checks nothing until [`App::start`].
    pub fn with_updates(config: AppConfig, updates: UpdateSetup) -> Arc<App> {
        let AppConfig {
            store,
            shell,
            screen,
        } = config;
        let UpdateSetup {
            config: update_config,
            product,
            clock,
            tick_every,
        } = updates;
        let updater = Updater::new(
            product,
            Version::of_package(env!("CARGO_PKG_VERSION")),
            update_config,
        );

        let mut status: Option<Status> = None;
        let mut hotkey_status: Option<StatusLine> = None;
        let mut notes: Vec<String> = Vec::new();
        let mut set_status = |line: StatusLine, about_hotkey: bool, notes: &mut Vec<String>| {
            notes.push(line.text.clone());
            // An error outranks an earlier note.
            if status
                .as_ref()
                .is_none_or(|s| line.kind == StatusKind::Error || s.line.kind != StatusKind::Error)
            {
                status = Some(Status { line, about_hotkey });
            }
        };
        let mut hotkey = DEFAULT_HOTKEY.to_string();
        let mut hotkey_replaced = false;
        let mut check_updates = false;
        let mut blocked_currencies = Vec::new();
        let mut overlay_position = OverlayPosition::default();
        let mut screen_watch = true;
        let store = match store {
            Ok(s) => Some(Arc::new(s)),
            Err(e) => {
                set_status(
                    StatusLine::error(format!(
                        "Settings are not saved: {e}. Set {} to another folder.",
                        crate::store::DIR_ENV
                    )),
                    false,
                    &mut notes,
                );
                None
            }
        };
        if let Some(store) = &store {
            match store.load_settings() {
                Ok(None) => {}
                Ok(Some(saved)) => {
                    // Any value but `true` leaves it off.
                    check_updates =
                        saved.get(CHECK_UPDATES_KEY).and_then(Value::as_bool) == Some(true);
                    if let Some(h) = saved.get("hotkey").and_then(Value::as_str) {
                        let h = h.trim();
                        if is_game_item_copy(h) {
                            let line = StatusLine::info(format!(
                                "The saved hotkey {h} is the game's own item copy, so the hotkey is {DEFAULT_HOTKEY} again."
                            ));
                            hotkey_status = Some(line.clone());
                            hotkey_replaced = true;
                            set_status(line, true, &mut notes);
                        } else if !h.is_empty() {
                            hotkey = h.to_string();
                        }
                    }
                    overlay_position = load_overlay_position(saved.get(OVERLAY_POSITION_KEY));
                    screen_watch = load_screen_watch(saved.get(SCREEN_WATCH_KEY));
                    let (ids, dropped) = load_blocked(saved.get(BLOCKED_CURRENCIES_KEY));
                    blocked_currencies = ids;
                    if dropped > 0 {
                        set_status(
                            StatusLine::info(format!(
                                "Dropped {dropped} saved blocked currenc{} that {} not valid.",
                                if dropped == 1 { "y" } else { "ies" },
                                if dropped == 1 { "was" } else { "were" }
                            )),
                            false,
                            &mut notes,
                        );
                    }
                }
                Err(e) => set_status(
                    StatusLine::error(format!("The saved settings could not be read: {e}.")),
                    false,
                    &mut notes,
                ),
            }
        }

        let engine = match Engine::shipped() {
            Ok(engine) => Some(engine),
            Err(errors) => {
                set_status(
                    StatusLine::error(format!(
                        "The shipped blueprints did not load ({} error{}), so no craft can be planned.",
                        errors.len(),
                        if errors.len() == 1 { "" } else { "s" }
                    )),
                    false,
                    &mut notes,
                );
                None
            }
        };

        let saver = store.as_ref().map(|s| start_saver(Arc::clone(s)));
        let has_screen = screen.is_some();
        let app = Arc::new_cyclic(|me| App {
            me: me.clone(),
            shell,
            store,
            state: Mutex::new(State {
                hotkey,
                hotkey_status,
                grab: None,
                status,
                check_updates,
                blocked_currencies,
                overlay_position,
                screen_watch,
                craft: Craft::default(),
                screen: ScreenState {
                    has_screen,
                    ..ScreenState::default()
                },
            }),
            saver: Mutex::new(saver),
            hotkey_replaced,
            updater,
            clock,
            tick_every,
            ticker: Mutex::new(None),
            restart_requested: AtomicBool::new(false),
            engine,
            has_screen,
            screen_setup: Mutex::new(screen),
            watch: Mutex::new(None),
            overlay_order: Mutex::new(()),
            stopped: AtomicBool::new(false),
        });
        for n in notes {
            app.diag(&format!("settings: {n}"));
        }
        app
    }

    /// Writes the start line, registers the hotkey, starts the watch thread
    /// (with a screen backend), sends the first view, starts the update
    /// ticker and, with `check_updates` on, the first automatic check. Reads
    /// no clipboard.
    pub fn start(&self) {
        self.diag(&format!(
            "start: crafting-helper {}; store {}",
            env!("CARGO_PKG_VERSION"),
            self.store
                .as_ref()
                .map(|s| s.dir().display().to_string())
                .unwrap_or_else(|| "none".to_string())
        ));
        let hotkey = self.lock().hotkey.clone();
        if let Err(e) = self.shell.register_hotkey(&hotkey) {
            let base = self.hotkey_refused(&hotkey, &e);
            let mut st = self.lock();
            st.status = Some(Status {
                line: StatusLine::error(format!("{base} Choose another one in Settings.")),
                about_hotkey: true,
            });
            st.hotkey_status = Some(StatusLine::error(format!("{base} Choose another one.")));
        } else {
            self.diag(&format!("hotkey {hotkey} registered"));
        }
        if self.hotkey_replaced {
            let st = self.lock();
            self.save(&st);
        }
        self.start_watch();
        self.emit();
        self.start_ticker();
        self.tick_updates(self.clock.now_ms());
    }

    /// The view now.
    pub fn get_view(&self) -> View {
        let newer = self.updater.newer().map(|v| v.to_string());
        let st = self.lock();
        view_of(&st, self.engine.as_ref(), newer)
    }

    /// The global hotkey was pressed: reads the clipboard once and grabs
    /// its text. A refusal beeps.
    pub fn hotkey_pressed(&self) {
        let checked = match self.shell.read_clipboard() {
            Ok(text) => grab::check(text.as_deref(), GrabSource::Shortcut),
            Err(e) => Err(Refusal::Unreadable(e)),
        };
        self.take_grab(checked, GrabSource::Shortcut);
    }

    /// `grab_text`: the text the main window's paste event carried. Reads
    /// no clipboard, and a refusal does not beep (the window is in front).
    pub fn grab_text(&self, text: Option<&str>) -> View {
        self.take_grab(grab::check(text, GrabSource::Paste), GrabSource::Paste);
        self.get_view()
    }

    // -- the overlay and the watch (see `crate::overlay`, `crate::screen`) ---

    /// With manual ticks ([`ScreenSetup::manual`]): one tick of the watch
    /// thread now, waited for (up to 10 s) with no lock of the app held.
    /// Does nothing on the thread's own clock, without a screen backend or
    /// after the shutdown. For the tests and the fixtures; no IPC command
    /// calls it.
    pub fn tick_watch(&self) {
        let tx = {
            let watch = self.watch.lock().unwrap_or_else(|p| p.into_inner());
            match watch.as_ref() {
                Some(w) if w.manual => w.tx.clone(),
                _ => return,
            }
        };
        let (ack, done) = mpsc::channel();
        if tx.send(WatchMsg::Tick(ack)).is_ok() {
            let _ = done.recv_timeout(Duration::from_secs(10));
        }
    }

    /// A report of the watch thread, for demand `generation` (see
    /// [`crate::screen`]). Nothing after the shutdown began.
    pub(crate) fn watch_report(&self, generation: u64, report: Report) {
        if self.stopped.load(Ordering::Acquire) {
            return;
        }
        match report {
            Report::Window(window) => self.window_report(generation, window),
            Report::Ocr(ready) => self.ocr_report(ready),
            Report::Read { seen, confirmed } => self.read_report(generation, seen, confirmed),
            Report::ReadFailed(failure) => self.read_failure(failure),
            Report::Minute { counts, busy_ms } => {
                if self.lock().guiding() {
                    self.diag(&format!(
                        "watch: minute: {} copies, {} tooltips, {} OCR reads, busy {busy_ms} ms",
                        counts.copies, counts.tooltips, counts.ocr_reads
                    ));
                }
            }
        }
    }

    /// The watch line the views carry now.
    fn watch_now(&self) -> Option<WatchLineView> {
        watch_line_view(&self.lock())
    }

    /// A read or OCR call failed: logged once until a call succeeds, and
    /// the watch line says the screen cannot be read and to grab the item,
    /// so a watch that never reads does not say it is watching.
    fn read_failure(&self, failure: WatchFailure) {
        let before = self.watch_now();
        let first = {
            let mut st = self.lock();
            let first = st.screen.read_failed.is_none();
            st.screen.read_failed = Some(failure.call.clone());
            first
        };
        if first {
            self.diag(&format!(
                "error: watch: {} failed: {}",
                failure.call, failure.error
            ));
        }
        if self.watch_now() != before {
            self.emit();
        }
    }

    /// Whether the OCR reads English: said once, and a refusal stops the
    /// reads (the next demand says not to read).
    fn ocr_report(&self, ready: Result<OcrReadiness, WatchFailure>) {
        let (line, changed) = {
            let mut st = self.lock();
            let was = st.screen.ocr;
            let line = match ready {
                Ok(OcrReadiness::Ready { language }) => {
                    st.screen.read_failed = None;
                    st.screen.ocr = Some(true);
                    format!("watch: on (OCR {language})")
                }
                Ok(OcrReadiness::NoEnglish) => {
                    st.screen.read_failed = None;
                    st.screen.ocr = Some(false);
                    "watch: refused: no English OCR".to_string()
                }
                Err(f) => {
                    drop(st);
                    self.read_failure(f);
                    return;
                }
            };
            let changed = was != st.screen.ocr;
            (line, changed)
        };
        if changed {
            self.diag(&line);
            self.screen_changed();
            self.emit();
        }
    }

    /// A read of the tooltips, for demand `generation`: what it saw becomes
    /// the watch's state (its line written when it changed), and a change
    /// read twice alike is applied to the craft (see `Craft::screen_change`)
    /// unless it is the unexpected change already turned down. Dropped for
    /// another generation or when the craft is not guiding.
    fn read_report(&self, generation: u64, seen: Seen, confirmed: Option<(Box<Item>, ItemKey)>) {
        let before = self.watch_now();
        let (lines, kept) = {
            let mut guard = self.lock();
            let st = &mut *guard;
            if st.screen.generation != generation || !st.guiding() {
                return;
            }
            st.screen.read_failed = None;
            let mut lines = Vec::new();
            let line = seen.diag_line();
            if st.screen.tooltip_line.as_ref() != Some(&line) {
                lines.push(line.clone());
                st.screen.tooltip_line = Some(line);
            }
            st.screen.seen = Some(seen);
            let mut kept = None;
            if let Some((item, key)) = confirmed {
                if st.screen.unexpected.as_ref() != Some(&key) {
                    match st.craft.screen_change(self.engine.as_ref(), *item) {
                        ScreenEffect::Kept {
                            result,
                            step,
                            summary,
                        } => {
                            lines.push(format!(
                                "craft: {} at {step} (from the screen)",
                                outcome_name(result)
                            ));
                            st.status = Some(Status {
                                line: StatusLine::info(format!("{summary}. Read from the screen.")),
                                about_hotkey: false,
                            });
                            kept = Some(true);
                        }
                        ScreenEffect::Unexpected { step } => {
                            lines.push(format!("watch: unexpected change at {step} not applied"));
                            st.screen.unexpected = Some(key);
                            st.status = Some(Status {
                                line: StatusLine::error(
                                    "Read from the screen: the item changed in a way the guide did not expect. Grab it to check; the craft is unchanged.",
                                ),
                                about_hotkey: false,
                            });
                            kept = Some(false);
                        }
                        ScreenEffect::Ignored => {}
                    }
                }
            }
            (lines, kept)
        };
        for line in &lines {
            self.diag(line);
        }
        match kept {
            // The item the next step is shown on changed: a new demand.
            Some(true) => {
                self.send_demand(true);
                self.sync_screen();
                self.emit();
            }
            Some(false) => {
                self.sync_screen();
                self.emit();
            }
            None => {
                if self.sync_screen() || self.watch_now() != before {
                    self.emit();
                }
            }
        }
    }

    /// The watch thread's look at the game's window, for demand
    /// `generation`: applied while that is the current demand and the craft
    /// is guiding; then the overlay follows, with no lock held.
    fn window_report(&self, generation: u64, window: Result<GameWindow, WatchFailure>) {
        let before = self.watch_now();
        let failure = {
            let mut st = self.lock();
            if st.screen.generation != generation || !st.guiding() {
                return;
            }
            let (game, failure) = match window {
                Ok(GameWindow::InFront { client, .. }) => (Game::InFront(client), None),
                Ok(GameWindow::NotInFront) => (Game::NotInFront, None),
                Ok(GameWindow::Minimized) => (Game::Minimized, None),
                Err(f) => (Game::Failed, Some(f)),
            };
            st.screen.game = game;
            // Logged once until a call succeeds.
            let first = failure.is_some() && st.screen.window_failed.is_none();
            st.screen.window_failed = failure.as_ref().map(|f| f.call.clone());
            failure.filter(|_| first)
        };
        if let Some(f) = failure {
            self.diag(&format!("error: watch: {} failed: {}", f.call, f.error));
        }
        if self.sync_screen() || self.watch_now() != before {
            self.emit();
        }
    }

    /// Starts the watch thread over the screen backend and sends it the
    /// first demand. Nothing without a backend.
    fn start_watch(&self) {
        let setup = self
            .screen_setup
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .take();
        let Some(setup) = setup else {
            return;
        };
        match screen::spawn(self.me.clone(), setup) {
            Ok(watch) => *self.watch.lock().unwrap_or_else(|p| p.into_inner()) = Some(watch),
            Err(e) => self.diag(&format!("error: the watch thread did not start: {e}")),
        }
        self.send_demand(false);
    }

    /// After a change that may move the craft in or out of guiding, change
    /// whether to read, or move the overlay: tells the watch thread and
    /// brings the overlay in line. The caller sends the view after.
    fn screen_changed(&self) {
        self.send_demand(false);
        self.sync_screen();
    }

    /// After a grab or a craft command: as [`App::screen_changed`], with a
    /// new generation whatever changed, so a read in flight is dropped and
    /// the thread matches against the item the step is shown on now.
    fn screen_moved(&self) {
        self.send_demand(true);
        self.sync_screen();
    }

    /// Sends the watch thread the demand when it changed, or with `force`,
    /// never waiting; a new demand has a new generation, and with `force`
    /// the watch's last read and an unexpected change turned down are
    /// forgotten. A craft that stops guiding forgets the game's window and
    /// the reads, so the next one starts from a fresh look.
    fn send_demand(&self, force: bool) {
        let demand = {
            let mut st = self.lock();
            let guiding = st.guiding();
            if !guiding {
                st.screen.game = Game::Unknown;
                st.screen.seen = None;
                st.screen.unexpected = None;
                st.screen.read_line = None;
                st.screen.tooltip_line = None;
            }
            let read = st.reads();
            if !force && st.screen.demand == Some((guiding, read)) {
                return;
            }
            st.screen.demand = Some((guiding, read));
            st.screen.generation += 1;
            if force {
                st.screen.seen = None;
                st.screen.unexpected = None;
            }
            Demand {
                generation: st.screen.generation,
                guiding,
                read,
                item: if read {
                    st.craft.step_item().cloned().map(Box::new)
                } else {
                    None
                },
            }
        };
        let watch = self.watch.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(w) = watch.as_ref() {
            let _ = w.tx.send(WatchMsg::Demand(demand));
        }
    }

    /// Brings the overlay window and the watch's state line in line with the
    /// state: the line when it changed, the placement when the position,
    /// the client area or the overlay's size changed, then the show or hide.
    /// Decided under the state's lock and carried out after it, in order
    /// (under `overlay_order`). True when the overlay's visibility changed
    /// (the views carry it). Nothing without a screen backend.
    fn sync_screen(&self) -> bool {
        if !self.has_screen || self.stopped.load(Ordering::Acquire) {
            return false;
        }
        // Read with no lock of the app held, `overlay_order` included.
        let size = if self.lock().overlay_wanted() {
            Some(self.shell.overlay_size())
        } else {
            None
        };
        let _order = self.overlay_order.lock().unwrap_or_else(|p| p.into_inner());
        let mut lines: Vec<String> = Vec::new();
        let (place, show) = {
            let mut guard = self.lock();
            let st = &mut *guard;
            if let Some(line) = watch_line(st) {
                if line != st.screen.watch_line {
                    lines.push(line.clone());
                    st.screen.watch_line = line;
                }
            }
            match st.read_idle().map(idle_line) {
                Some(line) => {
                    if st.screen.read_line.as_ref() != Some(&line) {
                        lines.push(line.clone());
                        st.screen.read_line = Some(line);
                    }
                }
                None => st.screen.read_line = None,
            }
            let mut place = None;
            let mut show_now = st.overlay_wanted();
            if let (true, Game::InFront(client)) = (show_now, st.screen.game) {
                match &size {
                    // It came to the front between the two locks: shown once
                    // its place is known, at the next sync.
                    None => show_now = st.screen.shown,
                    Some(Ok(size)) => {
                        st.screen.size_failed = false;
                        let key = (st.overlay_position, client, *size);
                        if st.screen.placed != Some(key) {
                            st.screen.placed = Some(key);
                            place = Some(overlay_origin(st.overlay_position, client, *size));
                            lines.push(format!("overlay: placed {}", st.overlay_position.name()));
                        }
                    }
                    // Shown where it is, said once.
                    Some(Err(e)) => {
                        if !st.screen.size_failed {
                            st.screen.size_failed = true;
                            lines.push(format!("error: overlay: its size is unknown: {e}"));
                        }
                    }
                }
            }
            let show = (show_now != st.screen.shown).then_some(show_now);
            if let Some(shown) = show {
                st.screen.shown = shown;
                lines.push(if shown {
                    "overlay: shown".to_string()
                } else {
                    let why = if st.guiding() {
                        NOT_IN_FRONT
                    } else {
                        NOT_GUIDING
                    };
                    format!("overlay: hidden ({why})")
                });
            }
            (place, show)
        };
        for line in &lines {
            self.diag(line);
        }
        if let Some((x, y)) = place {
            self.shell.place_overlay(x, y);
        }
        if let Some(shown) = show {
            self.shell.set_overlay_visible(shown);
        }
        show.is_some()
    }

    /// `set_hotkey`: registers `hotkey` in place of the current one. An
    /// empty key and the game's Ctrl+Alt+C are refused before the shell is
    /// asked; when the shell refuses, the current one stays (registered and
    /// saved). `hotkey_status` says what happened. Sends both views.
    pub fn set_hotkey(&self, hotkey: &str) -> SettingsView {
        let hotkey = hotkey.trim().to_string();
        let result = if hotkey.is_empty() {
            Err(HotkeyError::Invalid("no hotkey was given".to_string()))
        } else if is_game_item_copy(&hotkey) {
            self.diag(&format!(
                "hotkey {hotkey} refused: it is the game's item copy"
            ));
            let mut st = self.lock();
            st.hotkey_status = Some(StatusLine::error(format!(
                "{hotkey} is the game's own item copy, so it cannot be the hotkey. The hotkey is still {}.",
                st.hotkey
            )));
            drop(st);
            return self.settings_changed();
        } else {
            self.shell.register_hotkey(&hotkey)
        };
        match result {
            Ok(()) => {
                self.diag(&format!("hotkey {hotkey} registered"));
                let mut st = self.lock();
                st.hotkey = hotkey.clone();
                st.hotkey_status = Some(StatusLine::info(format!("The hotkey is now {hotkey}.")));
                if st.status.as_ref().is_some_and(|s| s.about_hotkey) {
                    st.status = None;
                }
                self.save(&st);
            }
            Err(e) => {
                let base = self.hotkey_refused(&hotkey, &e);
                let mut st = self.lock();
                let line = format!("{base} The hotkey is still {}.", st.hotkey);
                st.hotkey_status = Some(StatusLine::error(line));
            }
        }
        self.settings_changed()
    }

    // -- the Settings window and the update check and install (ep-update) -----

    /// `get_app_settings`: what the Settings window shows.
    pub fn get_app_settings(&self) -> SettingsView {
        let (on, hotkey, hotkey_status, blocked_currencies, overlay_position, screen_watch) = {
            let st = self.lock();
            (
                st.check_updates,
                st.hotkey.clone(),
                st.hotkey_status.clone(),
                st.blocked_currencies.clone(),
                st.overlay_position,
                st.screen_watch,
            )
        };
        SettingsView {
            updates: self.updater.view(on),
            hotkey,
            hotkey_status,
            blocked_currencies,
            overlay_position,
            screen_watch,
        }
    }

    /// `get_catalogue`: every crafting item the blocked list may name,
    /// grouped by kind with each group's heading and its entries (id, name)
    /// sorted by name ([`catalogue_groups`]). Both windows ask for it once;
    /// it never changes while the app runs.
    pub fn get_catalogue(&self) -> Vec<CatalogueGroup> {
        catalogue_groups()
    }

    /// `update_settings`: `check_updates`, `blocked_currencies`,
    /// `overlay_position` and `screen_watch`, any of them (see
    /// [`crate::settings`]). Anything else, a value of the wrong shape, a
    /// blocked id outside the catalogue or an unknown position, is refused
    /// whole with nothing changed. Turning the check on ticks at once, so a
    /// first check starts now when one is due; a new position places the
    /// overlay again while it shows. Sends both views.
    pub fn update_settings(&self, patch: &Value) -> Result<SettingsView, String> {
        let patch = match read_patch(patch) {
            Ok(p) => p,
            Err(reason) => {
                self.diag(&format!("settings refused: {reason}"));
                return Err(reason);
            }
        };
        let (turned, blocked, moved, watch) = {
            let mut st = self.lock();
            let turned = patch.check_updates.filter(|on| *on != st.check_updates);
            if let Some(on) = turned {
                st.check_updates = on;
            }
            let blocked = patch
                .blocked_currencies
                .filter(|ids| *ids != st.blocked_currencies);
            if let Some(ids) = &blocked {
                st.blocked_currencies = ids.clone();
            }
            let moved = patch.overlay_position.filter(|p| *p != st.overlay_position);
            if let Some(p) = moved {
                st.overlay_position = p;
            }
            let watch = patch.screen_watch.filter(|on| *on != st.screen_watch);
            if let Some(on) = watch {
                st.screen_watch = on;
            }
            if turned.is_some() || blocked.is_some() || moved.is_some() || watch.is_some() {
                self.save(&st);
            }
            (turned, blocked.map(|ids| ids.len()), moved, watch)
        };
        if let Some(p) = moved {
            self.diag(&format!("overlay position: {}", p.name()));
        }
        if let Some(on) = watch {
            self.diag(if on {
                "screen watch: on"
            } else {
                "screen watch: off"
            });
        }
        if moved.is_some() || watch.is_some() {
            self.screen_changed();
        }
        if let Some(n) = blocked {
            self.diag(&format!(
                "blocked currencies: {n} id{}",
                if n == 1 { "" } else { "s" }
            ));
        }
        if let Some(on) = turned {
            self.diag(turned_line(on));
            self.updater.changed();
        }
        let view = self.settings_changed();
        if turned == Some(true) {
            self.tick_updates(self.clock.now_ms());
        }
        Ok(view)
    }

    /// `check_for_updates`: starts a manual check and returns the view at
    /// once, never waiting for the request. While a check or an install
    /// runs, it starts nothing and returns the view as it is.
    pub fn check_for_updates(&self) -> SettingsView {
        let now = self.clock.now_ms();
        self.run_check(true, self.updater.begin_manual(now));
        self.get_app_settings()
    }

    /// The ticker's tick at `now_ms` (the app's clock): with `check_updates`
    /// on, starts an automatic check when one is due. Never installs.
    pub fn tick_updates(&self, now_ms: u64) {
        // With the setting off this reads one bool and does nothing else.
        if self.lock().check_updates {
            self.run_check(false, self.updater.begin_auto(now_ms, true));
        }
    }

    /// `install_update`: starts installing the newer, signed release and
    /// returns the view at once, downloading. Refused, with the reason and
    /// the view unchanged, while a check or an install runs, while
    /// restarting, with no newer version known or for a release that is not
    /// signed. The only way an install starts.
    pub fn install_update(&self) -> Result<SettingsView, String> {
        let events: Arc<dyn InstallEvents> = Arc::new(AppEvents(self.me.clone()));
        self.updater.install(events).map_err(|r| r.reason)?;
        Ok(self.get_app_settings())
    }

    /// `cancel_update`: asks a running install to stop (it then deletes its
    /// partial download and goes back to ready) and returns the view.
    pub fn cancel_update(&self) -> SettingsView {
        self.updater.cancel_install();
        self.get_app_settings()
    }

    /// `open_settings`: the main window's *Settings* button.
    pub fn open_settings(&self) -> Result<(), String> {
        self.shell.open_settings().map_err(|e| {
            self.diag(&format!("error: settings window: {e}"));
            e
        })
    }

    // -- the craft window (see `crate::craft`) -------------------------------

    /// `set_targets`: the whole target list of the planning item. A
    /// modifier the item cannot roll, or a floor outside the tiers its item
    /// level reaches, refuses the list whole; repeats are dropped.
    pub fn set_targets(&self, targets: &[Target]) -> View {
        self.craft_command("Targets not changed", |craft, engine, settings| {
            craft.set_targets(engine, targets, settings)
        })
    }

    /// `set_craft_blocked`: the effective blocked list for this craft
    /// (catalogue ids only). Never changes `settings.json`.
    pub fn set_craft_blocked(&self, blocked: &[String]) -> View {
        let checked = blocked_list(&Value::from(blocked.to_vec()));
        self.craft_command("Blocked list not changed", |craft, _, settings| {
            craft.set_blocked(checked?, settings)
        })
    }

    /// `choose_route`: previews another applicable route.
    pub fn choose_route(&self, blueprint: &str) -> View {
        self.craft_command("Route not changed", |craft, engine, settings| {
            craft.choose_route(engine, blueprint, settings)
        })
    }

    /// `accept_craft`: starts the guide on the previewed route.
    pub fn accept_craft(&self) -> View {
        self.craft_command("Craft not started", |craft, engine, settings| {
            craft.accept(engine, settings)
        })
    }

    /// `end_craft`: from guiding, in any status, back to planning on the
    /// latest item; targets and this craft's blocked changes are kept, the
    /// history is cleared.
    pub fn end_craft(&self) -> View {
        self.craft_command("Craft not ended", |craft, _, _| craft.end())
    }

    /// `open_craft_of_exile`: asks the shell to open the one fixed address,
    /// [`CRAFT_OF_EXILE_URL`], in the default browser. Takes no address.
    pub fn open_craft_of_exile(&self) -> View {
        let line = match self.shell.open_in_browser(CRAFT_OF_EXILE_URL) {
            Ok(()) => {
                self.diag("opened Craft of Exile");
                StatusLine::info("Opened Craft of Exile in your web browser.")
            }
            Err(e) => {
                self.diag(&format!("error: opening Craft of Exile: {e}"));
                StatusLine::error(format!("Craft of Exile could not be opened: {e}."))
            }
        };
        self.lock().status = Some(Status {
            line,
            about_hotkey: false,
        });
        self.emit();
        self.get_view()
    }

    /// Runs one craft command on the state: a refusal changes nothing and
    /// says why (`<what>: <reason>.`); either way one `diag.log` line, the
    /// status line and both views.
    fn craft_command(
        &self,
        what: &str,
        command: impl FnOnce(&mut Craft, Option<&Engine>, &[String]) -> CommandResult,
    ) -> View {
        let result = {
            let mut guard = self.lock();
            let st = &mut *guard;
            let result = command(&mut st.craft, self.engine.as_ref(), &st.blocked_currencies);
            let line = match &result {
                Ok(done) => StatusLine::info(done.status.clone()),
                Err(why) => StatusLine::error(format!("{what}: {why}.")),
            };
            st.status = Some(Status {
                line,
                about_hotkey: false,
            });
            result
        };
        match result {
            Ok(done) => self.diag(&done.diag),
            Err(why) => self.diag(&format!("craft: refused: {why}")),
        }
        self.screen_moved();
        self.emit();
        self.get_view()
    }

    /// A finished install asked to quit and start the new exe: the glue's
    /// exit, after [`App::shutdown`], starts it.
    pub fn restart_requested(&self) -> bool {
        self.restart_requested.load(Ordering::Acquire)
    }

    /// Carries out what [`Updater::begin_manual`] or [`Updater::begin_auto`]
    /// decided: a started check is logged and announced, and its request
    /// runs on a thread of its own ([`Updater::spawn`]). Without a transport
    /// it ends failed at once, with no request. Called with no lock held.
    fn run_check(&self, manual: bool, begin: Begin) {
        let Begin::Run(transport) = begin else {
            return;
        };
        self.diag(started_line(manual));
        self.emit_app_settings();
        let Some(transport) = transport else {
            self.finish_check(manual, Err(TransportError::Unsupported));
            return;
        };
        let app = self.me.clone();
        let spawned = Updater::spawn(transport, move |answer| {
            if let Some(app) = app.upgrade() {
                app.finish_check(manual, answer);
            }
        });
        if let Err(e) = spawned {
            self.diag(&format!(
                "error: the update check thread did not start: {e}"
            ));
            let code = e.raw_os_error().map_or(0, |c| c as u32);
            self.finish_check(manual, Err(TransportError::Other(code)));
        }
    }

    /// A check's request ended: its outcome is kept, logged and announced,
    /// unless the app began shutting down first. The main window hears it
    /// when the newer version it names changed.
    fn finish_check(&self, manual: bool, answer: Result<Answer, TransportError>) {
        let before = self.updater.newer();
        let Some(outcome) = self.updater.finish(self.clock.now_ms(), manual, answer) else {
            return;
        };
        self.diag(&outcome_line(&outcome, self.updater.running()));
        self.emit_app_settings();
        if self.updater.newer() != before {
            self.emit();
        }
    }

    /// Sends the Settings window's view, with no lock held while it is sent.
    /// When something changed the view meanwhile (another thread may have
    /// sent the newer one first), the newest is sent again, so the last view
    /// sent is always the current one. The number is read first: a change
    /// raises it after it is made, so one that raised it later was not seen.
    fn emit_app_settings(&self) {
        loop {
            if self.updater.is_stopped() {
                return;
            }
            let seq = self.updater.seq();
            let view = self.get_app_settings();
            self.shell.emit_app_settings(&view);
            if self.updater.seq() == seq {
                return;
            }
        }
    }

    /// A Settings command changed (or refused to change) something: sends
    /// both views and returns the Settings window's.
    fn settings_changed(&self) -> SettingsView {
        self.emit_app_settings();
        self.emit();
        self.get_app_settings()
    }

    /// The ticker thread: [`App::tick_updates`] every `tick_every` until the
    /// app shuts down or is dropped. None when `tick_every` is `None`.
    fn start_ticker(&self) {
        let Some(every) = self.tick_every else {
            return;
        };
        let (tx, rx) = mpsc::channel::<()>();
        let me = self.me.clone();
        let spawned = std::thread::Builder::new()
            .name("ch-update-ticker".to_string())
            .spawn(move || loop {
                match rx.recv_timeout(every) {
                    Err(RecvTimeoutError::Timeout) => {
                        let Some(app) = me.upgrade() else {
                            return;
                        };
                        app.tick_updates(app.clock.now_ms());
                    }
                    // Closed by the shutdown (nothing is ever sent).
                    _ => return,
                }
            });
        match spawned {
            Ok(_) => *self.ticker.lock().unwrap_or_else(|p| p.into_inner()) = Some(tx),
            Err(e) => self.diag(&format!("error: the update ticker did not start: {e}")),
        }
    }

    /// Writes any pending settings now.
    pub fn flush(&self) {
        let guard = self.saver.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(saver) = guard.as_ref() {
            let (tx, rx) = mpsc::channel();
            if saver.tx.send(SaveMsg::Flush(tx)).is_ok() {
                let _ = rx.recv();
            }
        }
    }

    /// Stops the updater (a running check or install ends without changing
    /// anything; neither is waited for), the ticker and the watch thread
    /// (waited for up to [`WATCH_STOP_DEADLINE`]; a call in flight is not
    /// interrupted, and a thread still in one is logged as not ended),
    /// writes any pending settings and stops the saver. Safe to call twice.
    pub fn shutdown(&self) {
        self.updater.stop();
        self.stopped.store(true, Ordering::Release);
        // The ticker ends when its channel closes; it is not waited for.
        drop(self.ticker.lock().unwrap_or_else(|p| p.into_inner()).take());
        let watch = self.watch.lock().unwrap_or_else(|p| p.into_inner()).take();
        if let Some(Watch { tx, handle, .. }) = watch {
            drop(tx);
            // The last handle on the app may drop on the watch thread
            // itself, which cannot wait for its own end.
            let on_it = handle.thread().id() == std::thread::current().id();
            if !on_it && !screen::join_by(handle, Instant::now() + WATCH_STOP_DEADLINE) {
                self.diag(&format!(
                    "error: watch: did not end within {} ms",
                    WATCH_STOP_DEADLINE.as_millis()
                ));
            }
        }
        let saver = self.saver.lock().unwrap_or_else(|p| p.into_inner()).take();
        if let Some(Saver { tx, thread }) = saver {
            drop(tx);
            let _ = thread.join();
            self.diag("stop");
        }
    }

    /// Writes one line to `diag.log` (nothing without a store).
    pub fn diag(&self, message: &str) {
        if let Some(store) = &self.store {
            let _ = store.diag().write(message);
        }
    }

    // -----------------------------------------------------------------------

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|p| p.into_inner())
    }

    fn emit(&self) {
        let view = self.get_view();
        self.shell.emit_view(&view);
    }

    /// Keeps a grab that passed, or reports one refused (see the module
    /// doc), then sends the view.
    fn take_grab(&self, checked: Result<GrabView, Refusal>, source: GrabSource) {
        let route = match source {
            GrabSource::Shortcut => "the shortcut",
            GrabSource::Paste => "a paste",
        };
        match checked {
            Ok(g) => {
                let n = g.line_count;
                let lines = format!("{n} line{}", if n == 1 { "" } else { "s" });
                self.diag(&format!("grab from {route}: {lines}"));
                let grabbed = if g.looks_like_item {
                    StatusLine::info(format!("Grabbed an item: {lines}."))
                } else {
                    StatusLine::error(format!(
                        "Grabbed {lines}, but they do not look like an item copied with Ctrl+Alt+C in the game."
                    ))
                };
                let mut st = self.lock();
                let effect = st
                    .craft
                    .grab(self.engine.as_ref(), &g.text, g.looks_like_item);
                let line = match &effect {
                    GrabEffect::Planning { dropped: 0 } | GrabEffect::Refused => grabbed,
                    GrabEffect::Planning { dropped } => StatusLine::info(format!(
                        "{} Dropped {dropped} target{} this item cannot roll.",
                        grabbed.text,
                        if *dropped == 1 { "" } else { "s" }
                    )),
                    GrabEffect::Unchanged => StatusLine::info(
                        "This is the item the step was shown on: take the step in the game, copy the item with Ctrl+Alt+C, then grab it again.",
                    ),
                    GrabEffect::Ignored(why) => StatusLine::error(format!(
                        "Not a step result: {why}. The craft is unchanged."
                    )),
                    GrabEffect::MatchesScreen => StatusLine::info(
                        "This is the item the screen read: the grab replaces it, and the craft is unchanged.",
                    ),
                    GrabEffect::Observed {
                        result, summary, ..
                    } => {
                        if *result == OutcomeWord::Unexpected {
                            StatusLine::error(format!("{summary}."))
                        } else {
                            StatusLine::info(format!("{summary}."))
                        }
                    }
                };
                st.grab = Some(g);
                st.status = Some(Status {
                    line,
                    about_hotkey: false,
                });
                drop(st);
                match &effect {
                    GrabEffect::Observed { result, step, .. } => {
                        self.diag(&format!("craft: {} at {step}", outcome_name(*result)));
                    }
                    GrabEffect::MatchesScreen => self.diag("craft: grab matches the screen"),
                    _ => {}
                }
            }
            Err(refusal) => {
                let reason = refusal.reason(source);
                self.diag(&format!("grab refused: {reason}"));
                if source == GrabSource::Shortcut {
                    self.refusal_beep();
                }
                self.lock().status = Some(Status {
                    line: StatusLine::error(format!("Not grabbed: {reason}.")),
                    about_hotkey: false,
                });
            }
        }
        // Every grab moves the generation on: a read in flight is dropped.
        self.screen_moved();
        self.emit();
    }

    /// A refusal beeps once, and the log says which sound ran.
    fn refusal_beep(&self) {
        let tried = self.shell.beep();
        if tried.is_empty() {
            self.diag("refusal beep: none on this platform");
        }
        for attempt in &tried {
            self.diag(&attempt.diag_line());
        }
    }

    /// Logs a hotkey the shell refused and returns the sentence that says
    /// why, for the status lines.
    fn hotkey_refused(&self, hotkey: &str, e: &HotkeyError) -> String {
        self.diag(&format!(
            "hotkey {hotkey} could not be registered: {}",
            e.detail()
        ));
        match e {
            HotkeyError::Taken(_) => format!("Hotkey {hotkey} is taken by another program."),
            HotkeyError::Invalid(d) => format!("{hotkey:?} is not a hotkey ({d})."),
            HotkeyError::Other(d) => format!("Hotkey {hotkey} could not be registered ({d})."),
        }
    }

    /// Hands the settings to the saver (written [`SAVE_DELAY`] later).
    fn save(&self, st: &State) {
        let value = json!({
            "schema": SETTINGS_SCHEMA,
            "hotkey": st.hotkey,
            "check_updates": st.check_updates,
            "blocked_currencies": st.blocked_currencies,
            "overlay_position": st.overlay_position.name(),
            "screen_watch": st.screen_watch,
        });
        let guard = self.saver.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(saver) = guard.as_ref() {
            let _ = saver.tx.send(SaveMsg::Save(value));
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// The view of `st`, with the newer version a check found.
fn view_of(st: &State, engine: Option<&Engine>, update_available: Option<String>) -> View {
    let mut craft = st.craft.view(engine, &st.blocked_currencies);
    if let Some(g) = craft.guiding.as_mut() {
        g.watch = watch_line_view(st);
    }
    let guide = craft.guiding.as_ref().map(overlay_guide);
    View {
        craft,
        hotkey: st.hotkey.clone(),
        grab: st.grab.clone(),
        status: st.status.as_ref().map(|s| s.line.clone()),
        overlay: OverlayView {
            visible: st.screen.shown,
            position: st.overlay_position,
            guide,
        },
        update_available,
    }
}

/// The watch's state line for `st`, or `None` when it has nothing new to
/// say (guiding, with no look at the game's window yet, or a failed one).
fn watch_line(st: &State) -> Option<String> {
    if !st.guiding() {
        return Some(idle_line(NOT_GUIDING));
    }
    match st.screen.game {
        Game::Unknown | Game::Failed => None,
        Game::NotInFront => Some(idle_line(NOT_IN_FRONT)),
        Game::Minimized => Some(idle_line("minimized")),
        Game::InFront(client) => Some(format!(
            "watch: game window {}x{}",
            client.width(),
            client.height()
        )),
    }
}

/// The screen watch's line for the views (`ui-design.md` § "The screen
/// watch's line"): while guiding, what the watch sees and what to do, the
/// ones that ask the player to act marked for the warning colour; `None`
/// when not guiding and with replan and done.
fn watch_line_view(st: &State) -> Option<WatchLineView> {
    let line = |text: &str, warning: bool| {
        Some(WatchLineView {
            text: text.to_string(),
            warning,
        })
    };
    match st.craft.guide_status()? {
        GuideStatusView::Replan | GuideStatusView::Done => return None,
        GuideStatusView::Stuck => return line(STUCK_LINE, false),
        GuideStatusView::Active => {}
    }
    if !st.screen.has_screen {
        return line(NO_BACKEND_LINE, false);
    }
    if !st.screen_watch {
        return line(WATCH_OFF_LINE, false);
    }
    if st.screen.ocr == Some(false) {
        return line(NO_OCR_LINE, true);
    }
    match st.screen.game {
        Game::Failed => {
            let call = st.screen.window_failed.as_deref().unwrap_or("game window");
            return line(&failed_line(call), true);
        }
        Game::NotInFront => return line(NOT_IN_FRONT_LINE, false),
        Game::Minimized => return line(MINIMIZED_LINE, false),
        Game::Unknown | Game::InFront(_) => {}
    }
    if let Some(call) = &st.screen.read_failed {
        return line(&failed_line(call), true);
    }
    match &st.screen.seen {
        None | Some(Seen::NoTooltip) => line(WATCHING_LINE, false),
        Some(Seen::Unchanged) => line(UNCHANGED_LINE, false),
        Some(Seen::AnotherItem) => line(ANOTHER_ITEM_LINE, false),
        Some(Seen::Changed(key)) if st.screen.unexpected.as_ref() == Some(key) => {
            line(UNEXPECTED_LINE, true)
        }
        Some(Seen::Changed(_)) => line(CHECKING_LINE, false),
        Some(Seen::Unreadable(why)) => line(&unreadable_line(*why), true),
        Some(Seen::Covered) => line(COVERED_LINE, true),
        Some(Seen::Black) => line(BLACK_LINE, true),
    }
}

/// The watch line while a backend call fails (the game's window, a copy of
/// it or the OCR): `Can't read the screen (<call> failed): grab the item
/// after each step.`, in the form of [`NO_OCR_LINE`]. Shown until a call
/// succeeds, so a watch that cannot read never says it is watching.
fn failed_line(call: &str) -> String {
    format!("Can't read the screen ({call} failed): grab the item after each step.")
}

/// An outcome as the `craft:` lines name it.
fn outcome_name(result: OutcomeWord) -> &'static str {
    match result {
        OutcomeWord::Hit => "hit",
        OutcomeWord::Miss => "miss",
        OutcomeWord::Unexpected => "unexpected",
    }
}

/// What a running install tells the app, on the install's thread. It holds
/// the app weakly: an install never keeps a closed app alive.
struct AppEvents(Weak<App>);

impl InstallEvents for AppEvents {
    fn changed(&self) {
        if let Some(app) = self.0.upgrade() {
            app.emit_app_settings();
        }
    }

    fn diag(&self, line: &str) {
        if let Some(app) = self.0.upgrade() {
            app.diag(line);
        }
    }

    fn restart(&self) {
        if let Some(app) = self.0.upgrade() {
            app.restart_requested.store(true, Ordering::Release);
            app.shell.restart();
        }
    }
}

/// The saver thread: keeps the last settings it was given and writes them
/// once no newer ones came for [`SAVE_DELAY`], on a flush, or when the app
/// shuts down.
fn start_saver(store: Arc<Store>) -> Saver {
    let (tx, rx) = mpsc::channel::<SaveMsg>();
    let thread = std::thread::Builder::new()
        .name("ch-saver".to_string())
        .spawn(move || {
            let write = |v: &Value| {
                if let Err(e) = store.save_settings(v) {
                    let _ = store
                        .diag()
                        .write(&format!("error: settings could not be saved: {e}"));
                }
            };
            let mut pending: Option<Value> = None;
            loop {
                let msg = if pending.is_some() {
                    rx.recv_timeout(SAVE_DELAY)
                } else {
                    rx.recv().map_err(|_| RecvTimeoutError::Disconnected)
                };
                match msg {
                    Ok(SaveMsg::Save(v)) => pending = Some(v),
                    Ok(SaveMsg::Flush(ack)) => {
                        if let Some(v) = pending.take() {
                            write(&v);
                        }
                        let _ = ack.send(());
                    }
                    Err(RecvTimeoutError::Timeout) => {
                        if let Some(v) = pending.take() {
                            write(&v);
                        }
                    }
                    Err(RecvTimeoutError::Disconnected) => {
                        if let Some(v) = pending.take() {
                            write(&v);
                        }
                        return;
                    }
                }
            }
        })
        .expect("ch-app: the saver thread could not start");
    Saver { tx, thread }
}
