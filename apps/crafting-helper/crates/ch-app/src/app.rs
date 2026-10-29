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
//! The overlay: [`View::overlay`] is visible only while
//! [`App::set_craft_active`] says so; no command calls it, and the craft
//! window does not either (the overlay is M6's), so the overlay never shows
//! in 0.1.0.
//!
//! The hotkey, `check_updates` and the blocked currencies are saved to
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
use std::time::Duration;

use ep_update::{
    outcome_line, started_line, turned_line, Answer, Begin, InstallEvents, TransportError, Updater,
    Version,
};
use serde_json::{json, Value};

use ch_engine::{Engine, Target};

use crate::craft::{CommandResult, Craft, GrabEffect, CRAFT_OF_EXILE_URL};
use crate::grab::{self, Refusal};
use crate::hotkey::{is_game_item_copy, DEFAULT_HOTKEY};
use crate::settings::{
    blocked_list, catalogue_groups, load_blocked, read_patch, CatalogueGroup,
    BLOCKED_CURRENCIES_KEY, CHECK_UPDATES_KEY,
};
use crate::store::{Store, StoreError, SETTINGS_SCHEMA};
use crate::updates::{Clock, UpdateSetup};
use crate::view::{
    GrabSource, GrabView, OutcomeWord, OverlayView, SettingsView, StatusKind, StatusLine, View,
};

/// How long after the last change the settings are written.
pub const SAVE_DELAY: Duration = Duration::from_millis(500);
/// The overlay's one line until the craft UI fills it.
pub const OVERLAY_PLACEHOLDER: &str = "Next step: shown here while a craft is active.";

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
    /// Shows the overlay window without activating it, or hides it.
    fn set_overlay_visible(&self, visible: bool);
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
    /// Whether the overlay shows; nothing in 0.1.0 sets it (M6 will).
    craft_active: bool,
    /// The craft window's state, in memory only.
    craft: Craft,
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
        let AppConfig { store, shell } = config;
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
                craft_active: false,
                craft: Craft::default(),
            }),
            saver: Mutex::new(saver),
            hotkey_replaced,
            updater,
            clock,
            tick_every,
            ticker: Mutex::new(None),
            restart_requested: AtomicBool::new(false),
            engine,
        });
        for n in notes {
            app.diag(&format!("settings: {n}"));
        }
        app
    }

    /// Writes the start line, registers the hotkey, sends the first view,
    /// starts the update ticker and, with `check_updates` on, the first
    /// automatic check. Reads no clipboard.
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

    /// Whether a craft is active, and so whether the overlay shows. Public
    /// for the tests and the overlay to come (M6); no IPC command calls it,
    /// and the craft window does not either: guiding leaves it hidden.
    pub fn set_craft_active(&self, active: bool) {
        {
            let mut st = self.lock();
            if st.craft_active == active {
                return;
            }
            st.craft_active = active;
        }
        self.diag(if active {
            "overlay shown: a craft is active"
        } else {
            "overlay hidden: no craft is active"
        });
        self.shell.set_overlay_visible(active);
        self.emit();
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
        let (on, hotkey, hotkey_status, blocked_currencies) = {
            let st = self.lock();
            (
                st.check_updates,
                st.hotkey.clone(),
                st.hotkey_status.clone(),
                st.blocked_currencies.clone(),
            )
        };
        SettingsView {
            updates: self.updater.view(on),
            hotkey,
            hotkey_status,
            blocked_currencies,
        }
    }

    /// `get_catalogue`: every crafting item the blocked list may name,
    /// grouped by kind with each group's heading and its entries (id, name)
    /// sorted by name ([`catalogue_groups`]). Both windows ask for it once;
    /// it never changes while the app runs.
    pub fn get_catalogue(&self) -> Vec<CatalogueGroup> {
        catalogue_groups()
    }

    /// `update_settings`: `check_updates` and `blocked_currencies`, either
    /// or both (see [`crate::settings`]). Anything else, a value of the
    /// wrong shape or a blocked id outside the catalogue, is refused whole
    /// with nothing changed. Turning the check on ticks at once, so a first
    /// check starts now when one is due. Sends both views.
    pub fn update_settings(&self, patch: &Value) -> Result<SettingsView, String> {
        let patch = match read_patch(patch) {
            Ok(p) => p,
            Err(reason) => {
                self.diag(&format!("settings refused: {reason}"));
                return Err(reason);
            }
        };
        let (turned, blocked) = {
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
            if turned.is_some() || blocked.is_some() {
                self.save(&st);
            }
            (turned, blocked.map(|ids| ids.len()))
        };
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
                if let GrabEffect::Observed { result, step, .. } = &effect {
                    let word = match result {
                        OutcomeWord::Hit => "hit",
                        OutcomeWord::Miss => "miss",
                        OutcomeWord::Unexpected => "unexpected",
                    };
                    self.diag(&format!("craft: {word} at {step}"));
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
    View {
        craft: st.craft.view(engine, &st.blocked_currencies),
        hotkey: st.hotkey.clone(),
        grab: st.grab.clone(),
        status: st.status.as_ref().map(|s| s.line.clone()),
        overlay: OverlayView {
            visible: st.craft_active,
            line: OVERLAY_PLACEHOLDER.to_string(),
        },
        update_available,
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
