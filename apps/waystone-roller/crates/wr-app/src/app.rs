//! [`App`]: the state behind one lock, the commands, the shortcut, the one
//! copy path the shortcut and the Copy button share, and the paste the
//! shortcut adds after it.
//!
//! The copy path: build the search for the current selection; `Ok` writes
//! the clipboard and stays silent; anything else beeps (the error tone),
//! leaves the clipboard alone and sets an error status. Each copy writes
//! `copy from the shortcut` or `copy from the Copy button`, then
//! `copied N characters: <regex>` or `copy refused: <reason>` to the
//! diagnostic log; a refusal adds one `refusal beep: <kind> ok|failed
//! (<error>)` line per sound the shell tried.
//!
//! The paste: only after the shortcut's copy wrote the clipboard (never the
//! Copy button's, never a refused copy), and only with `auto_paste` on. It
//! waits until no key is down (the shortcut's own keys still are when it
//! fires), for at most [`RELEASE_TIMEOUT`]; then, when the window in front
//! is titled exactly [`GAME_WINDOW_TITLE`], read right before the first key,
//! it sends Ctrl+A and Ctrl+V through the shell one key event at a time
//! ([`Shell::send_key`]), so the game's focused search box is replaced by
//! the search. Nothing else is ever sent: no other key, no mouse input, no
//! window message, no focus change. A failed send lets up every key it put
//! down and stops. One paste runs at a time, and the state lock is never
//! held across a pause. Each ends in one `paste: ...`, `paste skipped: ...`
//! or `paste failed: ...` line; the window's title is never written down.
//!
//! The selection, the shortcut, `auto_paste` and `check_updates` are saved
//! to `settings.json` by a saver thread, [`SAVE_DELAY`] after the last
//! change (debounced), and at once on [`App::flush`] and [`App::shutdown`].
//!
//! The update check and install (`ep-update`'s [`Updater`], behind a lock of
//! its own that is never held with the app's): a check from the
//! `check_for_updates` command or, with `check_updates` on, from
//! [`App::tick_updates`] (see [`crate::updates`]); an install only from the
//! `install_update` command. A check or an install runs on a thread of its
//! own and is never waited for; the Settings window hears every change as
//! `wr:app-settings` ([`Shell::emit_app_settings`]), and the main window's
//! view names a newer version found ([`View::update_available`]). A
//! finished install asks the shell to quit ([`Shell::restart`]) through the
//! app's own quit path, whose shutdown saves; the glue then starts the new
//! exe ([`App::restart_requested`]).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, MutexGuard, Weak};
use std::thread::JoinHandle;
use std::time::Duration;

use ep_update::{
    outcome_line, started_line, turned_line, Answer, AppSettingsView, Begin, InstallEvents,
    TransportError, Updater, Version,
};
use serde_json::{json, Map, Value};
use wr_regex::{Builder, ModData, Outcome, Selection, LIMIT, TIER_MAX, TIER_MIN};

use crate::store::{Store, StoreError, SETTINGS_SCHEMA};
use crate::updates::{check_updates_patch, Clock, UpdateSetup, CHECK_UPDATES_KEY};
use crate::view::{
    mod_views, DataInfo, ModView, SearchResult, SelectionView, StatusKind, StatusLine, View,
};

/// The shortcut on a first launch.
pub const DEFAULT_HOTKEY: &str = "Ctrl+Alt+W";
/// How long after the last change the settings are written.
pub const SAVE_DELAY: Duration = Duration::from_millis(500);

/// The only window the paste is sent to: the one in front, titled exactly
/// this (Live procedure 1 reads the game's title).
pub const GAME_WINDOW_TITLE: &str = "Path of Exile 2";
/// How often the paste asks whether a key is still down.
pub const RELEASE_POLL: Duration = Duration::from_millis(10);
/// How long the paste waits for every key to be up before it gives up,
/// counted as the sum of the pauses it asked for.
pub const RELEASE_TIMEOUT: Duration = Duration::from_millis(2000);
/// The pause before the first key event, once every key is up.
pub const SETTLE: Duration = Duration::from_millis(30);
/// The pause between two key events of one chord.
pub const KEY_GAP: Duration = Duration::from_millis(15);
/// The pause between Ctrl+A and Ctrl+V.
pub const CHORD_GAP: Duration = Duration::from_millis(50);

/// The diagnostic line of a paste skipped because auto-paste is off.
const PASTE_OFF: &str = "paste skipped: auto-paste is off";
/// The chords the paste sends, in order.
const CHORDS: [Key; 2] = [Key::A, Key::V];

/// A key the paste sends; there is no other.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    Ctrl,
    A,
    V,
}

/// The keys of a selection patch (`update_selection`).
const SELECTION_KEYS: [&str; 7] = [
    "want",
    "want_mode",
    "avoid",
    "tier_min",
    "tier_max",
    "rarity",
    "corruption",
];

/// Why a shortcut was not registered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HotkeyError {
    /// Another program holds the combination (Windows'
    /// `ERROR_HOTKEY_ALREADY_REGISTERED`).
    Taken(String),
    /// The text is not a shortcut.
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
    /// Registers `hotkey` as the one global shortcut, replacing the one
    /// registered now. When refused, the one registered before stays.
    fn register_hotkey(&self, hotkey: &str) -> Result<(), HotkeyError>;
    /// Puts `text` on the clipboard.
    fn write_clipboard(&self, text: &str) -> Result<(), String>;
    /// The error tone. Returns each sound tried, in order (a fallback
    /// follows a failed call); empty where the platform has none.
    fn beep(&self) -> Vec<BeepAttempt>;
    /// The title of the window in front, or `None` when there is none.
    /// Read for the paste only, and never written anywhere.
    fn foreground_title(&self) -> Option<String>;
    /// Whether any key is down now (mouse buttons not counted).
    fn any_key_down(&self) -> bool;
    /// Sends one event of `key`: down when `down`, else up. The only input
    /// the app ever sends.
    fn send_key(&self, key: Key, down: bool) -> Result<(), String>;
    /// Waits `duration`, on the thread the paste runs on.
    fn pause(&self, duration: Duration);
    /// Sends the view to the window (`wr:view`).
    fn emit_view(&self, view: &View);
    /// Sends the Settings window's view to it (`wr:app-settings`), from
    /// whichever thread made the change, with no lock of the app held.
    fn emit_app_settings(&self, _view: &AppSettingsView) {}
    /// Opens the Settings window, or brings it up when it is open.
    fn open_settings(&self) -> Result<(), String> {
        Err("this shell has no Settings window".to_string())
    }
    /// A new exe is in place: quit through the app's own quit path (whose
    /// shutdown calls [`App::shutdown`]); the shell starts the new exe once
    /// the app has shut down, because [`App::restart_requested`] says so.
    fn restart(&self) {}
}

/// What [`App::new`] is built from.
pub struct AppConfig {
    pub data: ModData,
    pub store: Result<Store, StoreError>,
    pub shell: Box<dyn Shell>,
}

/// The status line and whether a selection change keeps it: the shortcut's
/// and the store's warnings stay, a copy's outcome does not.
#[derive(Debug, Clone)]
struct Status {
    line: StatusLine,
    sticky: bool,
}

#[derive(Debug)]
struct State {
    selection: Selection,
    hotkey: String,
    /// The shortcut pastes after it copies; on unless the saved settings
    /// say `false`.
    auto_paste: bool,
    status: Option<Status>,
    /// "Automatically check for updates"; off unless the saved settings say
    /// `true`.
    check_updates: bool,
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
    builder: Builder,
    data_info: DataInfo,
    mods: Vec<ModView>,
    shell: Box<dyn Shell>,
    store: Option<Arc<Store>>,
    state: Mutex<State>,
    saver: Mutex<Option<Saver>>,
    /// The update check and install, behind a lock of their own.
    updater: Updater,
    clock: Box<dyn Clock>,
    tick_every: Option<Duration>,
    /// The way to the ticker thread; dropping it stops the thread.
    ticker: Mutex<Option<Sender<()>>>,
    /// A finished install asked the shell to quit and start the new exe.
    restart_requested: AtomicBool,
    /// A paste is running (from its wait to its last key).
    pasting: AtomicBool,
}

impl App {
    /// [`App::with_updates`] with the default [`UpdateSetup`]: no transport,
    /// so a check ends failed without a request and an install is refused.
    pub fn new(config: AppConfig) -> Arc<App> {
        App::with_updates(config, UpdateSetup::default())
    }

    /// Builds the app: picks the tokens, reads the saved settings (dropping
    /// saved mod ids the data no longer has), makes the updater and starts
    /// the saver. Registers nothing, emits nothing and checks nothing until
    /// [`App::start`].
    pub fn with_updates(config: AppConfig, updates: UpdateSetup) -> Arc<App> {
        let AppConfig { data, store, shell } = config;
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
        let data_info = DataInfo {
            source: data.source.clone(),
            fetched: data.fetched.clone(),
        };
        let builder = Builder::new(data);
        let mods = mod_views(&builder);

        let mut status: Option<Status> = None;
        let mut notes: Vec<String> = Vec::new();
        let mut set_status = |line: StatusLine, sticky: bool, notes: &mut Vec<String>| {
            notes.push(line.text.clone());
            // An error outranks an earlier note.
            if status
                .as_ref()
                .is_none_or(|s| line.kind == StatusKind::Error || s.line.kind != StatusKind::Error)
            {
                status = Some(Status { line, sticky });
            }
        };
        let mut selection = Selection::default();
        let mut hotkey = DEFAULT_HOTKEY.to_string();
        let mut check_updates = false;
        let mut auto_paste = true;
        let store = match store {
            Ok(s) => Some(Arc::new(s)),
            Err(e) => {
                set_status(
                    StatusLine::error(format!(
                        "Settings are not saved: {e}. Set {} to another folder.",
                        crate::store::DIR_ENV
                    )),
                    true,
                    &mut notes,
                );
                None
            }
        };
        if let Some(store) = &store {
            match store.load_settings() {
                Ok(None) => {}
                Ok(Some(saved)) => {
                    // A file written before the setting existed, or any
                    // value but `true`, leaves it off.
                    check_updates = saved.get(CHECK_UPDATES_KEY).and_then(Value::as_bool)
                        == Some(true);
                    // Missing, or any value but `false`, leaves it on.
                    auto_paste = saved.get("auto_paste").and_then(Value::as_bool) != Some(false);
                    if let Some(h) = saved.get("hotkey").and_then(Value::as_str) {
                        if !h.trim().is_empty() {
                            hotkey = h.trim().to_string();
                        }
                    }
                    if let Some(sel) = saved.get("selection") {
                        match serde_json::from_value::<SelectionView>(sel.clone()) {
                            Ok(v) => {
                                let mut s: Selection = (&v).into();
                                let dropped = keep_known(&mut s, builder.data());
                                normalise(&mut s);
                                selection = s;
                                if !dropped.is_empty() {
                                    set_status(
                                        StatusLine::info(format!(
                                            "Dropped {} saved mod{} no longer in the mod data: {}.",
                                            dropped.len(),
                                            if dropped.len() == 1 { "" } else { "s" },
                                            dropped.join(", ")
                                        )),
                                        false,
                                        &mut notes,
                                    );
                                }
                            }
                            Err(e) => set_status(
                                StatusLine::error(format!(
                                    "The saved selection could not be read ({e}); starting from none."
                                )),
                                false,
                                &mut notes,
                            ),
                        }
                    }
                }
                Err(e) => set_status(
                    StatusLine::error(format!("The saved settings could not be read: {e}.")),
                    false,
                    &mut notes,
                ),
            }
        }

        let saver = store.as_ref().map(|s| start_saver(Arc::clone(s)));
        let app = Arc::new_cyclic(|me| App {
            me: me.clone(),
            builder,
            data_info,
            mods,
            shell,
            store,
            state: Mutex::new(State {
                selection,
                hotkey,
                auto_paste,
                status,
                check_updates,
            }),
            saver: Mutex::new(saver),
            updater,
            clock,
            tick_every,
            ticker: Mutex::new(None),
            restart_requested: AtomicBool::new(false),
            pasting: AtomicBool::new(false),
        });
        for n in notes {
            app.diag(&format!("settings: {n}"));
        }
        app
    }

    /// Writes the start lines, registers the shortcut, sends the first view,
    /// starts the update ticker and, with `check_updates` on, the first
    /// automatic check.
    pub fn start(&self) {
        let families = self.mods.len();
        let tokened = self.mods.iter().filter(|m| m.token.is_some()).count();
        self.diag(&format!(
            "start: waystone-roller {}; data {} fetched {}, {families} mods, {tokened} with a token; store {}",
            env!("CARGO_PKG_VERSION"),
            self.data_info.source,
            self.data_info.fetched,
            self.store
                .as_ref()
                .map(|s| s.dir().display().to_string())
                .unwrap_or_else(|| "none".to_string())
        ));
        let hotkey = self.lock().hotkey.clone();
        if let Err(e) = self.shell.register_hotkey(&hotkey) {
            self.hotkey_refused(&hotkey, &e);
        } else {
            self.diag(&format!("shortcut {hotkey} registered"));
        }
        self.emit();
        self.start_ticker();
        self.tick_updates(self.clock.now_ms());
    }

    /// The view now.
    pub fn get_view(&self) -> View {
        let st = self.lock();
        self.view_of(&st)
    }

    /// Applies `patch`, any subset of the selection's keys. Refused, with
    /// the selection unchanged and an error status: a key the selection
    /// does not have, a value of the wrong shape, a mod the data lacks, a
    /// tier outside 1–16, or a mod in both lists.
    pub fn update_selection(&self, patch: &Value) -> Result<View, String> {
        let result = {
            let mut st = self.lock();
            match self.patched(&st.selection, patch) {
                Ok(selection) => {
                    st.selection = selection;
                    if st.status.as_ref().is_some_and(|s| !s.sticky) {
                        st.status = None;
                    }
                    self.save(&st);
                    Ok(self.view_of(&st))
                }
                Err(reason) => {
                    st.status = Some(Status {
                        line: StatusLine::error(format!("Not changed: {reason}.")),
                        sticky: false,
                    });
                    Err(reason)
                }
            }
        };
        if let Err(reason) = &result {
            self.diag(&format!("selection refused: {reason}"));
        }
        self.emit();
        result
    }

    /// The Copy button: the same copy as the shortcut, and never a paste
    /// (the app's own window is in front).
    pub fn copy_now(&self) -> View {
        self.copy("the Copy button");
        self.get_view()
    }

    /// The global shortcut was pressed: the copy, then, when it wrote the
    /// clipboard, the paste. Returns when both are done; the glue calls it
    /// on a blocking task, since the paste pauses.
    pub fn hotkey_pressed(&self) {
        if let Some(n) = self.copy("the shortcut") {
            self.paste(n);
        }
    }

    /// `set_auto_paste`: whether the shortcut pastes after it copies.
    /// Saved, logged and sent to the window.
    pub fn set_auto_paste(&self, on: bool) -> View {
        {
            let mut st = self.lock();
            st.auto_paste = on;
            st.status = Some(Status {
                line: StatusLine::info(if on {
                    "The shortcut now copies and pastes into the game."
                } else {
                    "The shortcut now only copies."
                }),
                sticky: false,
            });
            self.save(&st);
        }
        self.diag(if on {
            "auto-paste on"
        } else {
            "auto-paste off"
        });
        self.emit();
        self.get_view()
    }

    /// Registers `hotkey` in place of the current shortcut. When it is
    /// refused the current one stays (registered and saved) and the status
    /// says why.
    pub fn set_hotkey(&self, hotkey: &str) -> View {
        let hotkey = hotkey.trim().to_string();
        let result = if hotkey.is_empty() {
            Err(HotkeyError::Invalid("no shortcut was given".to_string()))
        } else {
            self.shell.register_hotkey(&hotkey)
        };
        match result {
            Ok(()) => {
                self.diag(&format!("shortcut {hotkey} registered"));
                let mut st = self.lock();
                st.hotkey = hotkey.clone();
                st.status = Some(Status {
                    line: StatusLine::info(format!("The shortcut is now {hotkey}.")),
                    sticky: false,
                });
                self.save(&st);
            }
            Err(e) => self.hotkey_refused(&hotkey, &e),
        }
        self.emit();
        self.get_view()
    }

    // -- the update check and install (ep-update) ------------------------------

    /// `get_app_settings`: what the Settings window shows.
    pub fn get_app_settings(&self) -> AppSettingsView {
        let on = self.lock().check_updates;
        self.updater.view(on)
    }

    /// `update_settings`: the Settings window's one setting, exactly
    /// `{"check_updates": <bool>}`. Anything else is refused whole, with the
    /// setting unchanged. Turning it on ticks at once, so a first check
    /// starts now when one is due.
    pub fn update_settings(&self, patch: &Value) -> Result<AppSettingsView, String> {
        let on = match check_updates_patch(patch) {
            Ok(on) => on,
            Err(reason) => {
                self.diag(&format!("settings refused: {reason}"));
                return Err(reason);
            }
        };
        let changed = {
            let mut st = self.lock();
            let changed = st.check_updates != on;
            if changed {
                st.check_updates = on;
                self.save(&st);
            }
            changed
        };
        if changed {
            self.diag(turned_line(on));
            self.updater.changed();
            self.emit_app_settings();
            if on {
                self.tick_updates(self.clock.now_ms());
            }
        }
        Ok(self.get_app_settings())
    }

    /// `check_for_updates`: starts a manual check and returns the view at
    /// once, never waiting for the request. While a check or an install
    /// runs, it starts nothing and returns the view as it is.
    pub fn check_for_updates(&self) -> AppSettingsView {
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
    pub fn install_update(&self) -> Result<AppSettingsView, String> {
        let events: Arc<dyn InstallEvents> = Arc::new(AppEvents(self.me.clone()));
        self.updater.install(events).map_err(|r| r.reason)?;
        Ok(self.get_app_settings())
    }

    /// `cancel_update`: asks a running install to stop (it then deletes its
    /// partial download and goes back to ready) and returns the view.
    pub fn cancel_update(&self) -> AppSettingsView {
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
            self.diag(&format!("error: the update check thread did not start: {e}"));
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

    /// The ticker thread: [`App::tick_updates`] every `tick_every` until the
    /// app shuts down or is dropped. None when `tick_every` is `None`.
    fn start_ticker(&self) {
        let Some(every) = self.tick_every else {
            return;
        };
        let (tx, rx) = mpsc::channel::<()>();
        let me = self.me.clone();
        let spawned = std::thread::Builder::new()
            .name("wr-update-ticker".to_string())
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
    /// anything; neither is waited for) and the ticker, writes any pending
    /// settings and stops the saver. Safe to call twice.
    pub fn shutdown(&self) {
        self.updater.stop();
        // The ticker ends when its channel closes; it is not waited for.
        drop(self.ticker.lock().unwrap_or_else(|p| p.into_inner()).take());
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

    fn view_of(&self, st: &State) -> View {
        View {
            data: self.data_info.clone(),
            mods: self.mods.clone(),
            selection: SelectionView::from(&st.selection),
            result: SearchResult::from_outcome(&self.builder.build(&st.selection)),
            hotkey: st.hotkey.clone(),
            auto_paste: st.auto_paste,
            status: st.status.as_ref().map(|s| s.line.clone()),
            update_available: self.updater.newer().map(|v| v.to_string()),
        }
    }

    fn emit(&self) {
        let view = self.get_view();
        self.shell.emit_view(&view);
    }

    /// The one copy path; `route` names what started it (the shortcut or
    /// the Copy button) in `diag.log`. Returns the search's length in
    /// characters when the clipboard was written, `None` when the copy was
    /// refused.
    fn copy(&self, route: &str) -> Option<usize> {
        self.diag(&format!("copy from {route}"));
        let selection = self.lock().selection.clone();
        let mut written = None;
        let (line, diag) = match self.builder.build(&selection) {
            Outcome::Ok { regex, .. } => {
                let n = regex.chars().count();
                match self.shell.write_clipboard(&regex) {
                    Ok(()) => {
                        written = Some(n);
                        (
                            StatusLine::info(format!(
                                "Copied the search ({n} characters). Paste it into the stash or a vendor's search box."
                            )),
                            format!("copied {n} characters: {regex}"),
                        )
                    }
                    Err(e) => (
                        StatusLine::error(format!(
                            "Not copied: the clipboard could not be written ({e})."
                        )),
                        format!("copy refused: the clipboard could not be written: {e}"),
                    ),
                }
            }
            Outcome::TooLong { length, .. } => (
                StatusLine::error(format!(
                    "Not copied: the search is {length} characters, over the game's {LIMIT}. Remove some selections."
                )),
                format!("copy refused: the search is {length} characters, over {LIMIT}"),
            ),
            Outcome::Empty => (
                StatusLine::error("Not copied: nothing is selected."),
                "copy refused: nothing is selected".to_string(),
            ),
            Outcome::Unmatchable { ids } => (
                StatusLine::error("Not copied: some mods cannot be told apart from others."),
                format!("copy refused: cannot be told apart: {}", ids.join(", ")),
            ),
        };
        self.diag(&diag);
        if line.kind == StatusKind::Error {
            self.refusal_beep();
        }
        self.lock().status = Some(Status {
            line,
            sticky: false,
        });
        self.emit();
        written
    }

    /// Every refusal beeps once, and the log says which sound ran.
    fn refusal_beep(&self) {
        let tried = self.shell.beep();
        if tried.is_empty() {
            self.diag("refusal beep: none on this platform");
        }
        for attempt in &tried {
            self.diag(&attempt.diag_line());
        }
    }

    /// The paste after the shortcut wrote `n` characters to the clipboard
    /// (see the module doc). Holds no lock of the app while it pauses.
    fn paste(&self, n: usize) {
        if !self.lock().auto_paste {
            self.diag(PASTE_OFF);
            return;
        }
        if self
            .pasting
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            self.diag("paste skipped: a paste is already running");
            return;
        }
        // Cleared however the paste ends.
        struct Running<'a>(&'a AtomicBool);
        impl Drop for Running<'_> {
            fn drop(&mut self) {
                self.0.store(false, Ordering::Release);
            }
        }
        let _running = Running(&self.pasting);

        // The shortcut fires on the press, its keys still down: a Ctrl+A
        // sent now would reach the game as Ctrl+Alt+A.
        let mut waited = Duration::ZERO;
        while self.shell.any_key_down() {
            if waited >= RELEASE_TIMEOUT {
                self.diag(&format!(
                    "paste skipped: keys still held after {} ms",
                    RELEASE_TIMEOUT.as_millis()
                ));
                self.paste_ended(StatusLine::error(format!(
                    "Copied the search ({n} characters), but did not paste it: keys were still held."
                )));
                self.refusal_beep();
                return;
            }
            self.shell.pause(RELEASE_POLL);
            waited += RELEASE_POLL;
        }
        self.shell.pause(SETTLE);

        // Checked where it is used: right before the first key.
        if self.shell.foreground_title().as_deref() != Some(GAME_WINDOW_TITLE) {
            self.diag("paste skipped: the window in front is not Path of Exile 2");
            return;
        }

        match self.send_chords() {
            Ok(()) => {
                self.diag("paste: sent Ctrl+A, Ctrl+V");
                self.paste_ended(StatusLine::info(format!(
                    "Copied the search ({n} characters) and pasted it into the game's search box."
                )));
            }
            Err(e) => {
                self.diag(&format!("paste failed: {e}"));
                self.paste_ended(StatusLine::error(format!(
                    "Copied the search ({n} characters), but it could not be pasted: {e}."
                )));
                self.refusal_beep();
            }
        }
    }

    /// Ctrl+A, then Ctrl+V, one key event at a time with the pinned pauses.
    /// When a send fails, every key put down is sent up (no stuck Ctrl) and
    /// nothing more is sent.
    fn send_chords(&self) -> Result<(), String> {
        let mut down: Vec<Key> = Vec::new();
        for (i, key) in CHORDS.iter().enumerate() {
            if i > 0 {
                self.shell.pause(CHORD_GAP);
            }
            let chord = [
                (Key::Ctrl, true),
                (*key, true),
                (*key, false),
                (Key::Ctrl, false),
            ];
            for (j, (k, press)) in chord.into_iter().enumerate() {
                if j > 0 {
                    self.shell.pause(KEY_GAP);
                }
                if let Err(e) = self.shell.send_key(k, press) {
                    for held in down.iter().rev() {
                        let _ = self.shell.send_key(*held, false);
                    }
                    return Err(e);
                }
                if press {
                    down.push(k);
                } else {
                    down.retain(|d| *d != k);
                }
            }
        }
        Ok(())
    }

    /// Sets the paste's outcome as the status and sends the view.
    fn paste_ended(&self, line: StatusLine) {
        self.lock().status = Some(Status {
            line,
            sticky: false,
        });
        self.emit();
    }

    fn hotkey_refused(&self, hotkey: &str, e: &HotkeyError) {
        self.diag(&format!(
            "shortcut {hotkey} could not be registered: {}",
            e.detail()
        ));
        let text = match e {
            HotkeyError::Taken(_) => {
                format!("Shortcut {hotkey} is taken by another program. Choose another one below.")
            }
            HotkeyError::Invalid(d) => {
                format!("{hotkey:?} is not a shortcut ({d}). Choose another one below.")
            }
            HotkeyError::Other(d) => format!(
                "Shortcut {hotkey} could not be registered ({d}). Choose another one below."
            ),
        };
        self.lock().status = Some(Status {
            line: StatusLine::error(text),
            sticky: true,
        });
    }

    /// `current` with `patch` applied and checked.
    fn patched(&self, current: &Selection, patch: &Value) -> Result<Selection, String> {
        let patch = patch
            .as_object()
            .ok_or_else(|| "the change is not an object".to_string())?;
        let mut merged: Map<String, Value> =
            match serde_json::to_value(SelectionView::from(current)) {
                Ok(Value::Object(m)) => m,
                _ => return Err("the selection could not be read".to_string()),
            };
        for (k, v) in patch {
            if !SELECTION_KEYS.contains(&k.as_str()) {
                return Err(format!("{k:?} is not part of the selection"));
            }
            merged.insert(k.clone(), v.clone());
        }
        let view: SelectionView = serde_json::from_value(Value::Object(merged))
            .map_err(|e| format!("the change does not fit the selection ({e})"))?;
        for t in [view.tier_min, view.tier_max] {
            if !(TIER_MIN..=TIER_MAX).contains(&t) {
                return Err(format!("{t} is not a tier from {TIER_MIN} to {TIER_MAX}"));
            }
        }
        let mut s: Selection = (&view).into();
        if let Some(id) = s
            .want
            .iter()
            .chain(&s.avoid)
            .find(|id| self.builder.data().family(id).is_none())
        {
            return Err(format!("{id} is not a mod of the data"));
        }
        if let Some(id) = s.want.iter().find(|id| s.avoid.contains(id)) {
            return Err(format!("{id} cannot be both wanted and avoided"));
        }
        normalise(&mut s);
        Ok(s)
    }

    /// Hands the settings to the saver (written [`SAVE_DELAY`] later).
    fn save(&self, st: &State) {
        let value = json!({
            "schema": SETTINGS_SCHEMA,
            "hotkey": st.hotkey,
            "auto_paste": st.auto_paste,
            "selection": SelectionView::from(&st.selection),
            "check_updates": st.check_updates,
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

/// Removes ids `data` lacks from both lists; returns them, in order.
fn keep_known(s: &mut Selection, data: &ModData) -> Vec<String> {
    let mut dropped = Vec::new();
    for list in [&mut s.want, &mut s.avoid] {
        list.retain(|id| {
            let known = data.family(id).is_some();
            if !known && !dropped.contains(id) {
                dropped.push(id.clone());
            }
            known
        });
    }
    dropped
}

/// Removes repeats, keeps a wanted mod out of the avoid list, and puts the
/// tiers in 1–16 and in order.
fn normalise(s: &mut Selection) {
    let mut seen = Vec::new();
    s.want.retain(|id| {
        let first = !seen.contains(id);
        seen.push(id.clone());
        first
    });
    let mut seen = Vec::new();
    let want = s.want.clone();
    s.avoid.retain(|id| {
        let keep = !seen.contains(id) && !want.contains(id);
        seen.push(id.clone());
        keep
    });
    let a = s.tier_min.clamp(TIER_MIN, TIER_MAX);
    let b = s.tier_max.clamp(TIER_MIN, TIER_MAX);
    s.tier_min = a.min(b);
    s.tier_max = a.max(b);
}

/// The saver thread: keeps the last settings it was given and writes them
/// once no newer ones came for [`SAVE_DELAY`], on a flush, or when the app
/// shuts down.
fn start_saver(store: Arc<Store>) -> Saver {
    let (tx, rx) = mpsc::channel::<SaveMsg>();
    let thread = std::thread::Builder::new()
        .name("wr-saver".to_string())
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
        .expect("wr-app: the saver thread could not start");
    Saver { tx, thread }
}
