//! The crafting helper's Tauri glue around `ch-app`, which holds every
//! decision. This file only connects it:
//!
//! - **Shell**: [`TauriShell`] implements `ch_app::Shell`: the one global
//!   shortcut through `tauri-plugin-global-shortcut` (Windows'
//!   `RegisterHotKey`, system-wide and exclusive, not a keyboard hook; a
//!   combination another program holds is refused and the old one stays
//!   registered), the clipboard read once per press through Win32, the
//!   error beep, the `ch:view` event to the main and overlay windows and
//!   `ch:app-settings` to the Settings window, the overlay's show, hide,
//!   size and placement, opening the Settings window, opening Craft of
//!   Exile in the default browser, and the restart an installed update asks
//!   for (an exit through the quit path below). The shortcut's handler runs
//!   `App::hotkey_pressed` on a blocking task, so the clipboard's retries
//!   and the beep never block the main thread.
//! - **Screen**: `screen.rs` implements `ch_app::ScreenBackend` over the
//!   quest tracker's Windows capture (`qt-capture`) and `ch-watch`'s reader:
//!   the game's window when it is in front (`game_window.rs`, by its title,
//!   the one file that reads the window in front) and its client area, the
//!   OCR's readiness and the hovered item's tooltips. `ch-app` runs it on
//!   its own watch thread while a craft is guiding.
//! - **Browser**: `tauri-plugin-opener`, from Rust alone, opens the one
//!   address `ch_app::App::open_craft_of_exile` passes,
//!   `ch_app::CRAFT_OF_EXILE_URL`; no command takes an address. The plugin
//!   is registered with `open_js_links_on_click(false)`, so it injects no
//!   link handler into the pages, and no capability grants an `opener:`
//!   permission, so the pages cannot reach its own commands.
//! - **Update transport**: [`NetTransport`] implements
//!   `ch_app::UpdateTransport` over `ep-net`, the app's only network code
//!   (the release list and the two fixed assets of a crafting helper
//!   release; no caller names a host or a URL). `ch-app` calls it only on a
//!   check's or an install's own thread.
//! - **Commands**: `get_view`, `get_app_settings` and `get_catalogue`
//!   (sync) and the async `grab_text`, `open_settings`, `update_settings`,
//!   `set_hotkey`, `check_for_updates`, `install_update`, `cancel_update`,
//!   and the craft window's `set_targets`, `set_craft_blocked`,
//!   `choose_route`, `accept_craft`, `end_craft` and `open_craft_of_exile`,
//!   each forwarded to the `App` method of the same name.
//! - **Single instance**: a second launch exits and brings this window up.
//! - **Windows**: `main`, ordinary, titled "Crafting Helper", resizable, in
//!   the taskbar; closing it quits the app (even with the Settings window or
//!   the overlay open), after the settings are written. `settings`, the page
//!   `settings.html`, built when first opened on a blocking task, decorated,
//!   not resizable, centred; its capability (`capabilities/settings.json`)
//!   grants `core:event:default` alone. `overlay`, the page `overlay.html`
//!   (`tauri.conf.json`): 480 x 168 at scale 1 (`ui-design.md` §
//!   "Overlay"), transparent, undecorated, always on top, off the taskbar,
//!   not focusable (Tauri's Windows backend turns that into
//!   `WS_EX_NOACTIVATE`) and hidden; made click-through at start with
//!   `set_ignore_cursor_events`, which Tauri implements with
//!   `WS_EX_TRANSPARENT | WS_EX_LAYERED`. `ch-app` decides when it shows (a
//!   craft guiding and the game in front) and where (inside the game's
//!   client area, at the `overlay_position` setting); the shell shows it
//!   with `SW_SHOWNOACTIVATE`, hides it with `SW_HIDE`, so the game keeps
//!   focus, and moves it with Tauri's `set_position`, each posted to the
//!   main thread and never waited on. Its handle is read once at setup, on
//!   the main thread; its size is read from that handle with Win32, so no
//!   call from the watch thread waits on a Tauri window getter.
//! - **Restart**: the exe's path is captured at start, before any swap
//!   could rename it. On `RunEvent::Exit` (plugins have run their own exit
//!   first, so the single-instance mutex is released) the app shuts down,
//!   every global shortcut is unregistered, and, when a finished install
//!   asked for it, the new exe at that path starts with this process's
//!   arguments. At start a background thread removes what an install leaves
//!   (`<exe>.old`, a stale `<exe>.download`).
//!
//! The app sends the game nothing: no key or mouse input, no window
//! message, no focus change, no other process opened, no memory read. It
//! reads the clipboard once per press of its own hotkey and at no other
//! time (no watching, no polling, no read at start). While a craft is
//! guiding it looks at which window is in front and, when it is the game's,
//! copies the game's client area and reads its item tooltips with Windows'
//! OCR, in memory: no copy, crop or OCR line is written or logged. Its
//! other effects outside its windows are the beep, the update check's
//! requests to GitHub, opening Craft of Exile's one address in the default
//! browser when the user presses its button and, when the user presses the
//! install button, replacing its own exe.

mod game_window;
mod screen;

use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use ch_app::ch_engine::Target;
use ch_app::ep_update::spawn_cleanup;
use ch_app::{
    Answer, App, AppConfig, AssetKind, BeepAttempt, CatalogueGroup, DownloadError, HotkeyError,
    SettingsView, Store, TransportError, UpdateConfig, UpdateSetup, UpdateTransport, Version,
    View,
};
use serde_json::Value;
use tauri::{
    AppHandle, Emitter as _, Manager, RunEvent, Theme, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use tauri_plugin_opener::{Builder as OpenerPlugin, OpenerExt};

/// The main window's label (`tauri.conf.json`).
const MAIN: &str = "main";
/// The overlay window's label (`tauri.conf.json`).
const OVERLAY: &str = "overlay";
/// The event that carries a fresh view to the main window and the overlay.
const VIEW_EVENT: &str = "ch:view";

/// The Settings window: its label, page, title and size at scale 1. The
/// height is an estimate of its tallest state: the hotkey with a refusal
/// over two lines, a full blocked list (its scroll box stops at 200 px) and
/// a download with its bar and *Cancel*; spare height falls below the
/// credit.
const SETTINGS: &str = "settings";
const SETTINGS_PAGE: &str = "settings.html";
const SETTINGS_TITLE: &str = "Crafting Helper Settings";
const SETTINGS_WIDTH: f64 = 440.0;
const SETTINGS_HEIGHT: f64 = 720.0;
/// The event that carries the Settings window's view to it.
const APP_SETTINGS_EVENT: &str = "ch:app-settings";

type Shared<'a> = tauri::State<'a, Arc<App>>;

// ---------------------------------------------------------------------------
// The shell, for ch-app.

struct TauriShell {
    app: AppHandle,
    /// The shortcut registered now, if any.
    hotkey: Mutex<Option<Shortcut>>,
    /// The overlay window's handle (an `HWND` as an integer), read once at
    /// setup on the main thread; `None` when it could not be.
    overlay: Option<isize>,
}

impl TauriShell {
    /// Logs a failed call on the overlay window.
    fn overlay_error(&self, e: &str) {
        if let Some(shell) = self.app.try_state::<Arc<App>>() {
            shell.diag(&format!("error: overlay window: {e}"));
        }
    }
}

/// What the overlay's calls say when its handle is not known.
const NO_OVERLAY: &str = "the overlay window is gone";

impl ch_app::Shell for TauriShell {
    fn register_hotkey(&self, hotkey: &str) -> Result<(), HotkeyError> {
        let new: Shortcut = hotkey
            .parse()
            .map_err(|e| HotkeyError::Invalid(format!("{e}")))?;
        let shortcuts = self.app.global_shortcut();
        let mut current = self.hotkey.lock().unwrap_or_else(|p| p.into_inner());
        if *current == Some(new) && shortcuts.is_registered(new) {
            return Ok(());
        }
        let old = current.take();
        if let Some(o) = old {
            let _ = shortcuts.unregister(o);
        }
        match shortcuts.register(new) {
            Ok(()) => {
                *current = Some(new);
                Ok(())
            }
            Err(e) => {
                // Keep the old shortcut working when the new one is refused.
                if let Some(o) = old {
                    if shortcuts.register(o).is_ok() {
                        *current = Some(o);
                    }
                }
                // The plugin carries global-hotkey's error as text; its
                // `AlreadyRegistered` reads "HotKey already registered".
                let text = e.to_string();
                if text.to_ascii_lowercase().contains("already registered") {
                    Err(HotkeyError::Taken(text))
                } else {
                    Err(HotkeyError::Other(text))
                }
            }
        }
    }

    /// Called by `App::hotkey_pressed` alone, once per press, on the
    /// blocking task the shortcut's handler started.
    fn read_clipboard(&self) -> Result<Option<String>, String> {
        clipboard::read()
    }

    fn beep(&self) -> Vec<BeepAttempt> {
        clipboard::beep()
    }

    fn emit_view(&self, view: &View) {
        let _ = self.app.emit_to(MAIN, VIEW_EVENT, view);
        let _ = self.app.emit_to(OVERLAY, VIEW_EVENT, view);
    }

    /// To the Settings window only; nothing when it is not open.
    fn emit_app_settings(&self, view: &SettingsView) {
        let _ = self.app.emit_to(SETTINGS, APP_SETTINGS_EVENT, view);
    }

    /// Called from the watch thread and the commands: posted to the main
    /// thread, never waited on.
    fn set_overlay_visible(&self, visible: bool) {
        if let Err(e) = overlay::show_without_activating(&self.app, self.overlay, visible) {
            self.overlay_error(&e);
        }
    }

    /// Read from the overlay's handle on the calling thread (Win32), never
    /// through Tauri's getter, which waits on the main thread.
    fn overlay_size(&self) -> Result<(u32, u32), String> {
        overlay::size(self.overlay)
    }

    /// Posted to the main thread, never waited on; `ch-app` computed the
    /// place (`ch_app::overlay_origin`).
    fn place_overlay(&self, x: i32, y: i32) {
        if let Err(e) = overlay::place(&self.app, x, y) {
            self.overlay_error(&e);
        }
    }

    fn open_settings(&self) -> Result<(), String> {
        open_settings_window(&self.app);
        Ok(())
    }

    /// Called by `App::open_craft_of_exile` alone, with its one fixed
    /// address; the default browser opens it, as the quest tracker opens a
    /// source page.
    fn open_in_browser(&self, address: &str) -> Result<(), String> {
        self.app
            .opener()
            .open_url(address, None::<&str>)
            .map_err(|e| e.to_string())
    }

    /// The quit path: `RunEvent::Exit` runs the shutdown, then starts the
    /// new exe because `App::restart_requested` says so.
    fn restart(&self) {
        self.app.exit(0);
    }
}

/// Opens the Settings window, or shows, unminimises and focuses it when it
/// is open already, on a blocking task (Tauri's window builder deadlocks on
/// Windows when it runs in a synchronous handler on the main thread). The
/// lock keeps two quick clicks from building it twice. A failure is logged.
fn open_settings_window(app: &AppHandle) {
    static OPENING: Mutex<()> = Mutex::new(());
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _one = OPENING.lock().unwrap_or_else(|p| p.into_inner());
        let result = match app.get_webview_window(SETTINGS) {
            Some(w) => w
                .show()
                .and_then(|()| w.unminimize())
                .and_then(|()| w.set_focus())
                .map_err(|e| e.to_string()),
            None => WebviewWindowBuilder::new(&app, SETTINGS, WebviewUrl::App(SETTINGS_PAGE.into()))
                .title(SETTINGS_TITLE)
                .inner_size(SETTINGS_WIDTH, SETTINGS_HEIGHT)
                .resizable(false)
                .maximizable(false)
                .decorations(true)
                .theme(Some(Theme::Dark))
                .center()
                .focused(true)
                .build()
                .map(|_| ())
                .map_err(|e| e.to_string()),
        };
        if let Err(e) = result {
            if let Some(shell) = app.try_state::<Arc<App>>() {
                shell.diag(&format!("error: settings window: {e}"));
            }
        }
    });
}

// ---------------------------------------------------------------------------
// The update transport, over ep-net.

/// `ep-net`'s two requests for the crafting helper, their answers and
/// errors carried over one to one. `ch-app` calls them only on a check's or
/// an install's own thread.
struct NetTransport;

const PRODUCT: ep_net::Product = ep_net::Product::CraftingHelper;

fn transport_error(e: ep_net::FetchError) -> TransportError {
    match e {
        ep_net::FetchError::Unsupported => TransportError::Unsupported,
        ep_net::FetchError::Timeout => TransportError::Timeout,
        ep_net::FetchError::Unreachable => TransportError::Unreachable,
        ep_net::FetchError::NotSecure => TransportError::NotSecure,
        ep_net::FetchError::TooLarge => TransportError::TooLarge,
        ep_net::FetchError::Panicked => TransportError::Panicked,
        ep_net::FetchError::Other(code) => TransportError::Other(code),
    }
}

impl UpdateTransport for NetTransport {
    fn fetch_releases(&self) -> Result<Answer, TransportError> {
        ep_net::fetch_release_list(PRODUCT)
            .map(|r| Answer {
                status: r.status,
                body: r.body,
            })
            .map_err(transport_error)
    }

    fn download(
        &self,
        version: Version,
        asset: AssetKind,
        limit: u64,
        sink: &mut dyn std::io::Write,
        progress: &mut dyn FnMut(u64, Option<u64>),
        cancel: &AtomicBool,
    ) -> Result<u64, DownloadError> {
        let part = |n: u64| u32::try_from(n).ok();
        // A version past u32 is no tag `ep-net` can name: refused unsent.
        let (Some(major), Some(minor), Some(patch)) =
            (part(version.major), part(version.minor), part(version.patch))
        else {
            return Err(DownloadError::Transport(TransportError::Unsupported));
        };
        let release = ep_net::ReleaseTag::new(PRODUCT, major, minor, patch);
        let asset = match asset {
            AssetKind::Exe => ep_net::Asset::Exe,
            AssetKind::Signature => ep_net::Asset::Signature,
        };
        ep_net::download_asset(release, asset, limit, sink, progress, cancel).map_err(|e| match e {
            ep_net::DownloadError::Fetch(f) => DownloadError::Transport(transport_error(f)),
            ep_net::DownloadError::Status(code) => DownloadError::Status(code),
            ep_net::DownloadError::RedirectRefused => DownloadError::RedirectRefused,
            ep_net::DownloadError::TooManyRedirects => DownloadError::TooManyRedirects,
            ep_net::DownloadError::Incomplete => DownloadError::Incomplete,
            ep_net::DownloadError::Cancelled => DownloadError::Cancelled,
            ep_net::DownloadError::Write(kind) => DownloadError::Write(kind),
        })
    }
}

/// What the exit needs to start the new exe: the exe's path as captured at
/// start and this process's arguments.
struct Relaunch {
    exe: Option<PathBuf>,
    args: Vec<OsString>,
}

/// Starts the new exe after the shutdown, when a finished install asked for
/// it. The global shortcuts are released first, so the new process can
/// register its own. A failure is logged; the old process ends either way.
fn relaunch(app: &AppHandle, shell: &App) {
    let _ = app.global_shortcut().unregister_all();
    if !shell.restart_requested() {
        return;
    }
    let Some(relaunch) = app.try_state::<Relaunch>() else {
        return;
    };
    let Some(exe) = &relaunch.exe else {
        shell.diag("updates: restart failed: the exe's path is not known");
        return;
    };
    match std::process::Command::new(exe).args(&relaunch.args).spawn() {
        Ok(_) => shell.diag("updates: started the new version"),
        Err(e) => shell.diag(&format!(
            "updates: restart failed: the new version did not start ({})",
            e.kind()
        )),
    }
}

/// The Win32 clipboard read and the beep.
#[cfg(windows)]
mod clipboard {
    use std::time::Duration;

    use ch_app::BeepAttempt;
    use windows_sys::Win32::Foundation::{GetLastError, HANDLE};
    use windows_sys::Win32::System::DataExchange::{
        CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
    };
    use windows_sys::Win32::System::Diagnostics::Debug::{Beep, MessageBeep};
    use windows_sys::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};
    use windows_sys::Win32::System::Ole::CF_UNICODETEXT;

    /// The refusal tone, the waystone roller's: `Beep` plays a sine tone
    /// through the default audio device and reads no entry of the sound
    /// scheme, so a muted or empty "Critical Stop" entry cannot silence it.
    /// A muted "System Sounds" level, or a default output device nobody is
    /// listening to, could (not ruled out). See
    /// docs/waystone-roller/waystone-roller-research.md § "The built app in
    /// the game (Live 2)". It blocks for its duration.
    const TONE_HZ: u32 = 880;
    const TONE_MS: u32 = 250;
    /// `MessageBeep`'s "simple beep", the fallback when `Beep` fails.
    const SIMPLE_BEEP: u32 = 0xFFFF_FFFF;

    /// How often, and how far apart, opening a clipboard another program
    /// holds is tried again: about 100 ms in all.
    const OPEN_TRIES: u32 = 5;
    const OPEN_PAUSE: Duration = Duration::from_millis(20);

    /// Reads the clipboard's `CF_UNICODETEXT` once: `Ok(None)` when it holds
    /// no text, `Err` when it could not be opened or read. Opened with no
    /// owner window: a read needs none, and nothing is written.
    pub fn read() -> Result<Option<String>, String> {
        let mut opened = false;
        for attempt in 0..OPEN_TRIES {
            if attempt > 0 {
                std::thread::sleep(OPEN_PAUSE);
            }
            // SAFETY: a null owner is allowed for a read; OpenClipboard
            // touches no memory of ours.
            if unsafe { OpenClipboard(std::ptr::null_mut()) } != 0 {
                opened = true;
                break;
            }
        }
        if !opened {
            return Err("another program is holding the clipboard".to_string());
        }

        let result = (|| {
            // SAFETY: the clipboard is open by this thread (above).
            if unsafe { IsClipboardFormatAvailable(CF_UNICODETEXT as u32) } == 0 {
                return Ok(None);
            }
            // SAFETY: the clipboard is open by this thread; the handle it
            // returns belongs to the clipboard and is only read, never freed.
            let block: HANDLE = unsafe { GetClipboardData(CF_UNICODETEXT as u32) };
            if block.is_null() {
                return Err("the clipboard's text could not be read".to_string());
            }
            // SAFETY: `block` is the clipboard's global block for
            // CF_UNICODETEXT, valid while the clipboard stays open.
            let bytes = unsafe { GlobalSize(block) };
            // SAFETY: as above; locking gives a pointer to its `bytes`
            // bytes, checked for null and unlocked below.
            let src = unsafe { GlobalLock(block) } as *const u16;
            if src.is_null() {
                return Err("the clipboard's text could not be read".to_string());
            }
            let units = bytes / std::mem::size_of::<u16>();
            // SAFETY: `src` points to `units` readable u16s while locked;
            // the slice is dropped before the unlock.
            let text = {
                let all = unsafe { std::slice::from_raw_parts(src, units) };
                let end = all.iter().position(|&u| u == 0).unwrap_or(all.len());
                String::from_utf16_lossy(&all[..end])
            };
            // SAFETY: the block locked above.
            unsafe { GlobalUnlock(block) };
            Ok(Some(text))
        })();
        // SAFETY: the clipboard was opened by this thread above.
        unsafe { CloseClipboard() };
        result
    }

    /// The error tone, then the simple beep only if the tone's call failed.
    /// Each call's result is returned for `diag.log`; a call that returns
    /// success still says nothing about whether it was heard.
    pub fn beep() -> Vec<BeepAttempt> {
        let tone = format!("tone {TONE_HZ} Hz {TONE_MS} ms");
        // SAFETY: Beep takes a frequency and a duration and touches no
        // memory of ours.
        let ok = unsafe { Beep(TONE_HZ, TONE_MS) } != 0;
        let first = attempt(tone, ok);
        if first.result.is_ok() {
            return vec![first];
        }
        // SAFETY: MessageBeep takes a sound type and touches no memory of ours.
        let ok = unsafe { MessageBeep(SIMPLE_BEEP) } != 0;
        vec![first, attempt("simple beep".to_string(), ok)]
    }

    /// The call's outcome, with the thread's last error when it failed. Read
    /// right after the call, before anything else can set it.
    fn attempt(kind: String, ok: bool) -> BeepAttempt {
        if ok {
            return BeepAttempt::ok(kind);
        }
        // SAFETY: GetLastError reads this thread's last error code only.
        let code = unsafe { GetLastError() };
        let error = std::io::Error::from_raw_os_error(code as i32);
        BeepAttempt::failed(kind, error.to_string())
    }
}

/// No clipboard reader off Windows: the app is a Windows app; this keeps
/// the workspace building elsewhere.
#[cfg(not(windows))]
mod clipboard {
    pub fn read() -> Result<Option<String>, String> {
        Err("the clipboard is read only on Windows".to_string())
    }

    pub fn beep() -> Vec<ch_app::BeepAttempt> {
        Vec::new()
    }
}

/// The overlay's show, hide, size and placement without taking focus from
/// the game, as the quest tracker's overlay does. Every call that changes
/// the window is posted to the main thread, which owns it, so the caller
/// (the watch thread or a command) never waits for it.
mod overlay {
    use std::sync::Arc;

    use ch_app::App;
    use tauri::{AppHandle, Manager, PhysicalPosition};

    use super::{NO_OVERLAY, OVERLAY};

    /// The overlay window's handle as an integer, read on the main thread
    /// (at setup): Tauri's `hwnd` waits on the main thread.
    #[cfg(windows)]
    pub fn handle(window: &tauri::WebviewWindow) -> Result<isize, String> {
        // A raw handle is not `Send`; its value is carried instead.
        Ok(window.hwnd().map_err(|e| e.to_string())?.0 as isize)
    }

    #[cfg(not(windows))]
    pub fn handle(_window: &tauri::WebviewWindow) -> Result<isize, String> {
        Err("the overlay's handle is read only on Windows".to_string())
    }

    /// Shows the overlay with `SW_SHOWNOACTIVATE`, so showing never takes
    /// focus from the game, or hides it with `SW_HIDE`.
    #[cfg(windows)]
    pub fn show_without_activating(
        app: &AppHandle,
        handle: Option<isize>,
        shown: bool,
    ) -> Result<(), String> {
        use windows_sys::Win32::Foundation::HWND;
        use windows_sys::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE, SW_SHOWNOACTIVATE};
        let handle = handle.ok_or_else(|| NO_OVERLAY.to_string())?;
        app.run_on_main_thread(move || {
            // SAFETY: the overlay's own window, shown or hidden on the main
            // thread that owns it.
            unsafe {
                ShowWindow(handle as HWND, if shown { SW_SHOWNOACTIVATE } else { SW_HIDE });
            }
        })
        .map_err(|e| e.to_string())
    }

    #[cfg(not(windows))]
    pub fn show_without_activating(
        app: &AppHandle,
        _handle: Option<isize>,
        shown: bool,
    ) -> Result<(), String> {
        let app2 = app.clone();
        app.run_on_main_thread(move || {
            if let Some(w) = app2.get_webview_window(OVERLAY) {
                let _ = if shown { w.show() } else { w.hide() };
            }
        })
        .map_err(|e| e.to_string())
    }

    /// The overlay window's size in physical pixels, from its handle on the
    /// calling thread (`GetWindowRect`, in the process's DPI awareness:
    /// Tauri makes it per-monitor aware, so physical pixels).
    #[cfg(windows)]
    pub fn size(handle: Option<isize>) -> Result<(u32, u32), String> {
        use windows_sys::Win32::Foundation::{HWND, RECT};
        use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect;
        let handle = handle.ok_or_else(|| NO_OVERLAY.to_string())?;
        let mut r = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        // SAFETY: the overlay's own window (stale at worst, when the call
        // fails); `r` is a local the call writes.
        if unsafe { GetWindowRect(handle as HWND, &mut r) } == 0 {
            return Err(format!("GetWindowRect failed: {}", std::io::Error::last_os_error()));
        }
        let side = |a: i32, b: i32| u32::try_from(i64::from(b) - i64::from(a)).unwrap_or(0);
        Ok((side(r.left, r.right), side(r.top, r.bottom)))
    }

    #[cfg(not(windows))]
    pub fn size(_handle: Option<isize>) -> Result<(u32, u32), String> {
        Err("the overlay's size is read only on Windows".to_string())
    }

    /// Moves the overlay's top-left corner to `(x, y)`, physical screen
    /// pixels, with Tauri's own `set_position` (which neither activates it
    /// nor changes its size or order) on the main thread. A failure there is
    /// logged.
    pub fn place(app: &AppHandle, x: i32, y: i32) -> Result<(), String> {
        let app2 = app.clone();
        app.run_on_main_thread(move || {
            let result = app2
                .get_webview_window(OVERLAY)
                .ok_or_else(|| NO_OVERLAY.to_string())
                .and_then(|w| w.set_position(PhysicalPosition::new(x, y)).map_err(|e| e.to_string()));
            if let Err(e) = result {
                if let Some(shell) = app2.try_state::<Arc<App>>() {
                    shell.diag(&format!("error: overlay window: place: {e}"));
                }
            }
        })
        .map_err(|e| e.to_string())
    }
}

/// Makes the overlay click-through: every mouse event passes to the window
/// below (the game). A failure is logged; the overlay still shows only
/// while a craft is guiding and the game is in front.
fn click_through(window: &WebviewWindow, shell: &App) {
    match window.set_ignore_cursor_events(true) {
        Ok(()) => shell.diag("overlay: click-through"),
        Err(e) => shell.diag(&format!("error: overlay window: click-through: {e}")),
    }
}

/// Runs `f` with the app on a blocking task, off the main thread, when the
/// app is up.
fn later(app: &AppHandle, f: impl FnOnce(&App) + Send + 'static) {
    if let Some(shell) = app.try_state::<Arc<App>>() {
        let shell = Arc::clone(&shell);
        tauri::async_runtime::spawn_blocking(move || f(&shell));
    }
}

/// Brings the window up: shown, restored and focused.
fn bring_up(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(MAIN) {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

// ---------------------------------------------------------------------------
// The commands (Context "The IPC contract").

#[tauri::command]
fn get_view(app: Shared<'_>) -> View {
    app.get_view()
}

/// The paste event's own text; no clipboard is read for it.
#[tauri::command(async)]
fn grab_text(app: Shared<'_>, text: Option<String>) -> View {
    app.grab_text(text.as_deref())
}

#[tauri::command(async)]
fn open_settings(app: Shared<'_>) -> Result<(), String> {
    app.open_settings()
}

// The craft window's commands: each returns the view, and a refused one
// changes nothing and says why in its status line.

/// The whole target list of the planning item.
#[tauri::command(async)]
fn set_targets(app: Shared<'_>, targets: Vec<Target>) -> View {
    app.set_targets(&targets)
}

/// This craft's effective blocked list (catalogue ids only); never saved.
#[tauri::command(async)]
fn set_craft_blocked(app: Shared<'_>, blocked: Vec<String>) -> View {
    app.set_craft_blocked(&blocked)
}

#[tauri::command(async)]
fn choose_route(app: Shared<'_>, blueprint: String) -> View {
    app.choose_route(&blueprint)
}

#[tauri::command(async)]
fn accept_craft(app: Shared<'_>) -> View {
    app.accept_craft()
}

#[tauri::command(async)]
fn end_craft(app: Shared<'_>) -> View {
    app.end_craft()
}

/// Takes no address: `ch-app` opens its one fixed address.
#[tauri::command(async)]
fn open_craft_of_exile(app: Shared<'_>) -> View {
    app.open_craft_of_exile()
}

// Both windows' command.

/// Every crafting item the blocked list may name, grouped by kind.
#[tauri::command]
fn get_catalogue(app: Shared<'_>) -> Vec<CatalogueGroup> {
    app.get_catalogue()
}

// The Settings window's commands.

#[tauri::command]
fn get_app_settings(app: Shared<'_>) -> SettingsView {
    app.get_app_settings()
}

#[tauri::command(async)]
fn update_settings(app: Shared<'_>, patch: Value) -> Result<SettingsView, String> {
    app.update_settings(&patch)
}

#[tauri::command(async)]
fn set_hotkey(app: Shared<'_>, hotkey: String) -> SettingsView {
    app.set_hotkey(&hotkey)
}

#[tauri::command(async)]
fn check_for_updates(app: Shared<'_>) -> SettingsView {
    app.check_for_updates()
}

#[tauri::command(async)]
fn install_update(app: Shared<'_>) -> Result<SettingsView, String> {
    app.install_update()
}

#[tauri::command(async)]
fn cancel_update(app: Shared<'_>) -> SettingsView {
    app.cancel_update()
}

// ---------------------------------------------------------------------------

/// Builds and runs the Tauri application.
pub fn run() {
    let app = tauri::Builder::default()
        // First, so a second launch exits before it sets anything up.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            bring_up(app);
        }))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        later(app, |shell| shell.hotkey_pressed());
                    }
                })
                .build(),
        )
        // Opened from Rust alone: no link handler injected into the pages.
        .plugin(OpenerPlugin::new().open_js_links_on_click(false).build())
        .setup(|app| {
            let store = Store::open_default();
            // Captured now and never read again: a swap renames the file.
            let (exe, exe_error) = match std::env::current_exe() {
                Ok(p) => (Some(p), None),
                Err(e) => (None, Some(e)),
            };
            // Setup runs on the main thread: the overlay's handle is read
            // here once, for the shell and the screen backend.
            let overlay_window = app.get_webview_window(OVERLAY);
            let overlay_handle = overlay_window.as_ref().map(overlay::handle);
            let handle = overlay_handle.as_ref().and_then(|h| h.as_ref().ok().copied());
            let shell = App::with_updates(
                AppConfig {
                    store,
                    shell: Box::new(TauriShell {
                        app: app.handle().clone(),
                        hotkey: Mutex::new(None),
                        overlay: handle,
                    }),
                    screen: screen::setup(handle),
                },
                UpdateSetup {
                    config: UpdateConfig {
                        transport: Some(Box::new(NetTransport)),
                        exe_path: exe.clone(),
                        ..UpdateConfig::default()
                    },
                    ..UpdateSetup::default()
                },
            );
            // Managed the moment it exists: the windows tauri.conf.json
            // declares are already loading their pages, and every command
            // they send before this line rejects with "state not managed"
            // (the pages ask again for their first view; `bridge.ts`'s
            // `untilManaged`). Nothing before it can wait: the screen
            // backend only stores the overlay's handle and makes its OCR
            // engine on first use, and the rest builds the App itself.
            app.manage(Arc::clone(&shell));
            app.manage(Relaunch {
                exe: exe.clone(),
                args: std::env::args_os().skip(1).collect(),
            });
            match &overlay_window {
                Some(w) => click_through(w, &shell),
                None => shell.diag("error: overlay window: not created"),
            }
            if let Some(Err(e)) = &overlay_handle {
                shell.diag(&format!("error: overlay window: handle: {e}"));
            }
            shell.start();
            match exe {
                Some(exe) => {
                    let weak = Arc::downgrade(&shell);
                    let spawned = spawn_cleanup(exe, move |line| {
                        if let Some(shell) = weak.upgrade() {
                            shell.diag(&line);
                        }
                    });
                    if let Err(e) = spawned {
                        shell.diag(&format!("updates: cleanup did not start ({})", e.kind()));
                    }
                }
                None => shell.diag(&format!(
                    "updates: the exe's path is not known ({}); updates cannot be installed",
                    exe_error.map_or_else(String::new, |e| e.kind().to_string())
                )),
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the main window quits the app, even with the Settings
            // window or the overlay still open.
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == MAIN {
                    api.prevent_close();
                    window.app_handle().exit(0);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_view,
            grab_text,
            open_settings,
            set_targets,
            set_craft_blocked,
            choose_route,
            accept_craft,
            end_craft,
            open_craft_of_exile,
            get_catalogue,
            get_app_settings,
            update_settings,
            set_hotkey,
            check_for_updates,
            install_update,
            cancel_update,
        ])
        .build(tauri::generate_context!())
        .expect("crafting-helper: the Tauri application failed to start");

    app.run(|app, event| {
        if let RunEvent::Exit = event {
            if let Some(shell) = app.try_state::<Arc<App>>() {
                shell.shutdown();
                relaunch(app, &shell);
            }
        }
    });
}
