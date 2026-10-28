//! [`App`]: the state behind one lock, the commands, the shortcut and the
//! one copy path the shortcut and the Copy button share.
//!
//! The copy path: build the search for the current selection; `Ok` writes
//! the clipboard and stays silent; anything else beeps (the error tone),
//! leaves the clipboard alone and sets an error status. Each copy writes one
//! diagnostic line, `copied N characters: <regex>` or `copy refused:
//! <reason>`.
//!
//! The selection and the shortcut are saved to `settings.json` by a saver
//! thread, [`SAVE_DELAY`] after the last change (debounced), and at once on
//! [`App::flush`] and [`App::shutdown`].

use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::JoinHandle;
use std::time::Duration;

use serde_json::{json, Map, Value};
use wr_regex::{Builder, ModData, Outcome, Selection, LIMIT, TIER_MAX, TIER_MIN};

use crate::store::{Store, StoreError, SETTINGS_SCHEMA};
use crate::view::{
    mod_views, DataInfo, ModView, SearchResult, SelectionView, StatusKind, StatusLine, View,
};

/// The shortcut on a first launch.
pub const DEFAULT_HOTKEY: &str = "Ctrl+Alt+W";
/// How long after the last change the settings are written.
pub const SAVE_DELAY: Duration = Duration::from_millis(500);

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

/// What the app needs from outside: implemented by the Tauri glue, and by a
/// recording fake in the tests.
pub trait Shell: Send + Sync {
    /// Registers `hotkey` as the one global shortcut, replacing the one
    /// registered now. When refused, the one registered before stays.
    fn register_hotkey(&self, hotkey: &str) -> Result<(), HotkeyError>;
    /// Puts `text` on the clipboard.
    fn write_clipboard(&self, text: &str) -> Result<(), String>;
    /// The error tone.
    fn beep(&self);
    /// Sends the view to the window (`wr:view`).
    fn emit_view(&self, view: &View);
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
    status: Option<Status>,
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
    builder: Builder,
    data_info: DataInfo,
    mods: Vec<ModView>,
    shell: Box<dyn Shell>,
    store: Option<Arc<Store>>,
    state: Mutex<State>,
    saver: Mutex<Option<Saver>>,
}

impl App {
    /// Builds the app: picks the tokens, reads the saved settings (dropping
    /// saved mod ids the data no longer has) and starts the saver. Registers
    /// nothing and emits nothing until [`App::start`].
    pub fn new(config: AppConfig) -> Arc<App> {
        let AppConfig { data, store, shell } = config;
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
        let app = Arc::new(App {
            builder,
            data_info,
            mods,
            shell,
            store,
            state: Mutex::new(State {
                selection,
                hotkey,
                status,
            }),
            saver: Mutex::new(saver),
        });
        for n in notes {
            app.diag(&format!("settings: {n}"));
        }
        app
    }

    /// Writes the start lines, registers the shortcut and sends the first
    /// view.
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

    /// The Copy button: the same path as the shortcut.
    pub fn copy_now(&self) -> View {
        self.copy();
        self.get_view()
    }

    /// The global shortcut was pressed.
    pub fn hotkey_pressed(&self) {
        self.copy();
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

    /// Writes any pending settings and stops the saver. Safe to call twice.
    pub fn shutdown(&self) {
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
            status: st.status.as_ref().map(|s| s.line.clone()),
        }
    }

    fn emit(&self) {
        let view = self.get_view();
        self.shell.emit_view(&view);
    }

    /// The one copy path.
    fn copy(&self) {
        let selection = self.lock().selection.clone();
        let (line, diag) = match self.builder.build(&selection) {
            Outcome::Ok { regex, .. } => {
                let n = regex.chars().count();
                match self.shell.write_clipboard(&regex) {
                    Ok(()) => (
                        StatusLine::info(format!(
                            "Copied the search ({n} characters). Paste it into the stash or a vendor's search box."
                        )),
                        format!("copied {n} characters: {regex}"),
                    ),
                    Err(e) => {
                        self.shell.beep();
                        (
                            StatusLine::error(format!(
                                "Not copied: the clipboard could not be written ({e})."
                            )),
                            format!("copy refused: the clipboard could not be written: {e}"),
                        )
                    }
                }
            }
            Outcome::TooLong { length, .. } => {
                self.shell.beep();
                (
                    StatusLine::error(format!(
                        "Not copied: the search is {length} characters, over the game's {LIMIT}. Remove some selections."
                    )),
                    format!("copy refused: the search is {length} characters, over {LIMIT}"),
                )
            }
            Outcome::Empty => {
                self.shell.beep();
                (
                    StatusLine::error("Not copied: nothing is selected."),
                    "copy refused: nothing is selected".to_string(),
                )
            }
            Outcome::Unmatchable { ids } => {
                self.shell.beep();
                (
                    StatusLine::error("Not copied: some mods cannot be told apart from others."),
                    format!("copy refused: cannot be told apart: {}", ids.join(", ")),
                )
            }
        };
        self.diag(&diag);
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
            "selection": SelectionView::from(&st.selection),
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
