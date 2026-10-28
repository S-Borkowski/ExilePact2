//! Tauri glue for the quest-tracker overlay: the `overlay` window, the tray's
//! Settings window, the commands as thin wrappers over `qt-app`, the emitter
//! that sends `qt:view`, `qt:row-change` and `qt:app-settings`, the update
//! check's and the install's transport, and the restart into an installed
//! update. Every decision is `qt-app`'s; this file only connects it to Tauri.
//!
//! - **Window**: the `overlay` window of `tauri.conf.json` (transparent,
//!   undecorated, always on top, off the taskbar, not focusable). At
//!   creation `WS_EX_NOACTIVATE` is added through `windows-sys` so the game
//!   keeps focus, and the handle and its extended style go to the
//!   diagnostic log. The config's `focusable: false` keeps that bit: Tauri's
//!   Windows backend rewrites the extended style from its own flags whenever
//!   one changes (click-through does), and derives `WS_EX_NOACTIVATE` from
//!   "not focusable".
//! - **Click-through**: `set_ignore_cursor_events`, which Tauri's Windows
//!   backend implements with `WS_EX_TRANSPARENT | WS_EX_LAYERED`. Edit mode
//!   also makes the window focusable (dropping `WS_EX_NOACTIVATE`), because
//!   a window that cannot be activated never receives keys and the Settings
//!   fields must be typed in; play mode makes it unfocusable again.
//! - **Visibility**: on every worker tick `qt-app` asks for the pid of the
//!   foreground window (`GetForegroundWindow`, `GetWindowThreadProcessId`;
//!   only the id is read) and, in play mode, hides the overlay unless that
//!   is the game's or its own. Its own pid is reported only for the overlay
//!   window (or a window it owns); the tray's window, in front while the
//!   tray menu is open, counts as another program's, so the menu never shows
//!   a hidden overlay. Showing uses `SW_SHOWNOACTIVATE`, so the game keeps
//!   focus; hiding `SW_HIDE`.
//! - **Shortcut**: one system hotkey through `tauri-plugin-global-shortcut`
//!   (default Ctrl+Alt+Q), which toggles play and edit mode. No keyboard
//!   hook; nothing is sent to the game.
//! - **Single instance**: a second launch is refused and noted in the log.
//! - **Commands**: the IPC contract's names. `get_view` reads memory and is
//!   a plain sync command; every other one may save (debounced) or write the
//!   diagnostic log, so it is `#[tauri::command(async)]` and runs off the
//!   WebView thread; none calls `block_on`. Argument keys follow Tauri's
//!   default, camelCase on the JavaScript side: `invoke("quest_action",
//!   { questId, action })`, `invoke("tick_step", { questId, stepId,
//!   ticked })`, `invoke("update_settings", { patch })`. The Settings
//!   window's own are `get_app_settings` (sync, like `get_view`),
//!   `check_for_updates` (starts a check and answers at once),
//!   `install_update` (starts the install and answers at once; the only way
//!   one starts) and `cancel_update`.
//! - **Auto scale**: every window move and every DPI scale change of the
//!   window has `qt-app` re-read the monitor, which re-applies the zoom only
//!   when it differs from the one applied.
//! - **Tray**: Tauri's own tray icon (feature `tray-icon`), with the app's
//!   default window icon. Its menu and tooltip are `qt-app`'s
//!   (`qt_app::tray_menu` of the app-settings view): *Update available: X*
//!   while a newer version is known, *Check for updates*, *Settings*, a
//!   separator and *Quit Quest Tracker*; the tooltip `Quest Tracker`, or
//!   `Quest Tracker (X available)`. Built in Rust at setup, before the
//!   worker starts, and rebuilt whenever the app-settings view changes the
//!   menu (the emitter's `app_settings`, called with no app lock held).
//!   *Update available: X* opens the Settings window, where the install
//!   button is; no entry installs or opens a page. Each
//!   entry's action runs on a blocking task, never on the main thread. No
//!   capability, CSP or JavaScript is involved. It logs `tray: icon shown`,
//!   or `error: tray icon: <reason>` and the overlay runs on without it.
//! - **Settings window**: label `settings`, the page `settings.html`, opened
//!   from the tray's *Settings* and *Check for updates*: created when first
//!   asked for, from a spawned task (Tauri's window builder deadlocks on
//!   Windows when called in a synchronous event handler, and the tray's menu
//!   handler is one), and shown, unminimised and focused when it is already
//!   open. Decorated, dark, not resizable, 420 by 328 at 1x, centred, in the
//!   taskbar while open; closing it destroys it. Its capability
//!   (`capabilities/settings.json`) grants `core:event:default` alone, and
//!   `qt:app-settings` is sent to it only. Moves, DPI changes and close
//!   requests of this window are not the overlay's and are left alone. In
//!   play mode, with it in front, the overlay hides as for any other window.
//! - **Update transport**: `ep-net`'s release list and release-asset
//!   download for the quest tracker, passed to `qt-app`, which alone decides
//!   when they run (a manual check, the `check_updates` setting on, or the
//!   install button). Their errors map one to one. The glue never sets the
//!   update key: `ep-update`'s built-in release key is the only one.
//! - **Quit**: the tray's Quit and a close request to the overlay's window
//!   (`WM_CLOSE`), shown or hidden, both make `qt-app`'s quit request (which
//!   writes the `quit:` line; the first request wins) and then exit the app.
//!   The close itself is held back, so the window stays until the exit;
//!   `RunEvent::Exit` then runs `shutdown` as before.
//! - **Restart into an update**: the exe's path is captured at start
//!   (`std::env::current_exe()`) and handed to `qt-app`, never read again.
//!   An install that put the new exe in place makes `qt-app`'s quit request
//!   and exits like the tray's Quit (`Window::restart`); `RunEvent::Exit`
//!   runs `shutdown`, then frees every global shortcut and starts the
//!   captured exe with this launch's arguments (`relaunch`). Not Tauri's
//!   `AppHandle::restart`.
//! - **Start-up cleanup**: a background thread removes `<exe>.old` and a
//!   stale `<exe>.download` an earlier install left, retrying for about
//!   30 s, and logs one `updates: cleanup:` line.
//! - **Screen reading**: on Windows the shell gets `qt-capture`'s backend,
//!   built with the overlay's window handle (read where the no-activate
//!   style reads it), so the reader thread checks the overlay's rect and
//!   visibility with Win32 calls and never calls into Tauri. Elsewhere it
//!   gets none, and the "Read the screen" setting starts nothing. No
//!   capability is involved.
//! - **Panel watcher**: on Windows the shell also gets a second
//!   `qt-capture` backend from the same handle, the watcher's own, so the
//!   check for game panels under the overlay never waits on the reader's
//!   lock. The watcher runs from start to shutdown (hiding under game
//!   panels is always on, with no setting). Elsewhere it gets none, and no
//!   watcher starts.
//! - **Main-thread callbacks** (the shortcut, window moves, a DPI scale
//!   change, a second launch, the tray menu, a close request)
//!   hand their work to a blocking task, so the main thread never waits for
//!   the app's lock while a command holding it waits for the main thread.
//!   The tray's rebuild waits for the main thread with no app lock held.

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use qt_app::updates::{Answer, AssetKind, DownloadError, TransportError, Version};
use qt_app::{
    App, AppConfig, AppSettingsView, Frame, Mode, MonitorMetrics, Options, PanelConfig, Placement,
    QuestAction, QuitFrom, RowChange, ScreenBackend, Store, SystemClock, TrayAction, TrayItem,
    TrayMenu, UpdateConfig, UpdateTransport, View, ViewKind, DEFAULT_POLL_INTERVAL,
    MIN_WINDOW_SIZE, SHUTDOWN_TIMEOUT,
};
use serde_json::Value;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{
    AppHandle, Emitter as _, LogicalSize, Manager, PhysicalPosition, RunEvent, Theme, WebviewUrl,
    WebviewWindow, WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use tauri_plugin_opener::OpenerExt;

/// The overlay window's label (`tauri.conf.json`).
const OVERLAY: &str = "overlay";

/// The tray's Settings window: its label, page, title and size at scale 1
/// (as tall as its content, `docs/quest-tracker/design/component-spec.md`).
const SETTINGS: &str = "settings";
const SETTINGS_PAGE: &str = "settings.html";
const SETTINGS_TITLE: &str = "Quest Tracker Settings";
const SETTINGS_WIDTH: f64 = 420.0;
const SETTINGS_HEIGHT: f64 = 328.0;

/// The tray icon's id. Its menu and tooltip are `qt_app::tray_menu`'s.
const TRAY: &str = "quest-tracker";


/// The default place, for a window with no saved position (where the
/// right-hand dock put it before the dock side was dropped): the gap to the
/// monitor's right side, in logical pixels, and how far down the monitor
/// the window's top sits, so it clears the game's minimap in the top-right
/// corner. Not measured against the game; the person moves the window in
/// edit mode and the position is kept.
const EDGE_MARGIN: f64 = 16.0;
const TOP_FRACTION: f64 = 0.30;

type Shell<'a> = tauri::State<'a, Arc<App>>;

// ---------------------------------------------------------------------------
// The emitter and the window, for qt-app.

struct TauriEmitter {
    app: AppHandle,
    /// The tray menu shown now, shared with the setup that builds the tray.
    tray: Arc<TrayShown>,
}

impl qt_app::Emitter for TauriEmitter {
    fn view(&self, view: &View) {
        let _ = self.app.emit_to(OVERLAY, "qt:view", view);
    }

    fn row_change(&self, change: &RowChange) {
        let _ = self.app.emit_to(OVERLAY, "qt:row-change", change);
    }

    /// Sends the view to the Settings window only (nothing when it is not
    /// open), then brings the tray's menu and tooltip up to date. `qt-app`
    /// calls this on whichever thread ran the change, which may be the main
    /// thread itself (a tray click handled in place), so nothing here waits
    /// for the main thread: the tray, which the main thread owns, is updated
    /// by a task posted to it and not waited for (run in place when the
    /// caller is the main thread). Posted tasks run in order, so the menu
    /// ends on the last view.
    fn app_settings(&self, view: &AppSettingsView) {
        let _ = self.app.emit_to(SETTINGS, "qt:app-settings", view);
        let app = self.app.clone();
        let tray = Arc::clone(&self.tray);
        let view = view.clone();
        let posted = self.app.run_on_main_thread(move || {
            if let Err(e) = tray.update(&app, &view) {
                if let Some(shell) = app.try_state::<Arc<App>>() {
                    shell.diag(&format!("error: tray menu: {e}"));
                }
            }
        });
        if let Err(e) = posted {
            if let Some(shell) = self.app.try_state::<Arc<App>>() {
                shell.diag(&format!("error: tray menu: {e}"));
            }
        }
    }
}

/// The update check's and the install's transport: `ep-net`'s release list
/// and release-asset download for the quest tracker, the answers and errors
/// carried over one to one. `qt-app`'s updater calls it, on a check's or an
/// install's own thread, only when one runs.
struct NetTransport;

/// `ep-net`'s request errors as the updater names them.
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
        ep_net::fetch_release_list(ep_net::Product::QuestTracker)
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
        // A tag's numbers are `u32` in `ep-net`; a version past them names
        // no release this build can fetch.
        let part = |n: u64| u32::try_from(n).ok();
        let (Some(major), Some(minor), Some(patch)) =
            (part(version.major), part(version.minor), part(version.patch))
        else {
            return Err(DownloadError::Transport(TransportError::Unsupported));
        };
        let release = ep_net::ReleaseTag::new(ep_net::Product::QuestTracker, major, minor, patch);
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

struct TauriWindow {
    app: AppHandle,
    /// The shortcut registered now, if any.
    hotkey: Mutex<Option<Shortcut>>,
}

impl TauriWindow {
    fn window(&self) -> Result<WebviewWindow, String> {
        self.app
            .get_webview_window(OVERLAY)
            .ok_or_else(|| "the overlay window is gone".to_string())
    }
}

impl qt_app::Window for TauriWindow {
    fn set_click_through(&self, on: bool) -> Result<(), String> {
        let w = self.window()?;
        // Play mode: not focusable, which Tauri's Windows backend turns into
        // `WS_EX_NOACTIVATE`, so the game keeps focus. Edit mode: focusable,
        // so a click activates the window and Settings' fields take keys.
        w.set_focusable(!on).map_err(|e| e.to_string())?;
        w.set_ignore_cursor_events(on).map_err(|e| e.to_string())
    }

    /// Zooms the content only; the window keeps the size the player gave it.
    fn set_scale(&self, factor: f64) -> Result<(), String> {
        self.window()?.set_zoom(factor).map_err(|e| e.to_string())
    }

    /// Edit mode: the panel's edge handles resize the window through
    /// `startResizeDragging`, and Tauri's own border for an undecorated
    /// window (inside the transparent margin) works too. Play mode: neither.
    fn set_resizable(&self, on: bool) -> Result<(), String> {
        self.window()?.set_resizable(on).map_err(|e| e.to_string())
    }

    fn set_size(&self, width: f64, height: f64) -> Result<(), String> {
        self.window()?
            .set_size(LogicalSize::new(width, height))
            .map_err(|e| e.to_string())
    }

    fn place(&self, placement: Placement) -> Result<(), String> {
        let w = self.window()?;
        let (x, y) = match placement {
            Placement::At { x, y } => (x, y),
            Placement::Default => {
                let monitor = match w.current_monitor().map_err(|e| e.to_string())? {
                    Some(m) => m,
                    None => w
                        .primary_monitor()
                        .map_err(|e| e.to_string())?
                        .ok_or_else(|| "no monitor".to_string())?,
                };
                let (at, size) = (monitor.position(), monitor.size());
                let outer = w.outer_size().map_err(|e| e.to_string())?;
                let margin = (EDGE_MARGIN * monitor.scale_factor()).round() as i32;
                let x = at.x + size.width as i32 - outer.width as i32 - margin;
                (x, at.y + (f64::from(size.height) * TOP_FRACTION).round() as i32)
            }
        };
        w.set_position(PhysicalPosition::new(x, y))
            .map_err(|e| e.to_string())
    }

    fn register_hotkey(&self, hotkey: &str) -> Result<(), String> {
        let new: Shortcut = hotkey
            .parse()
            .map_err(|e| format!("{hotkey:?} is not a shortcut: {e}"))?;
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
                Err(e.to_string())
            }
        }
    }

    fn open_url(&self, url: &str) -> Result<(), String> {
        self.app
            .opener()
            .open_url(url, None::<&str>)
            .map_err(|e| e.to_string())
    }

    fn monitor(&self) -> Result<Option<MonitorMetrics>, String> {
        let w = self.window()?;
        let monitor = match w.current_monitor().map_err(|e| e.to_string())? {
            Some(m) => Some(m),
            None => w.primary_monitor().map_err(|e| e.to_string())?,
        };
        // `size()` is in physical pixels; `scale_factor()` is Windows' DPI
        // scale for that monitor, which the webview already applies.
        Ok(monitor.map(|m| MonitorMetrics {
            height: m.size().height,
            dpi_scale: m.scale_factor(),
        }))
    }

    fn foreground_pid(&self) -> Option<u32> {
        // The overlay's handle, when it can be read; without it every window
        // of this process counts as the overlay, as before the tray.
        let overlay = self.window().ok().and_then(|w| w.hwnd().ok()).map(|h| h.0 as isize);
        foreground_pid(overlay)
    }

    fn set_visible(&self, shown: bool) -> Result<(), String> {
        let w = self.window()?;
        show_without_activating(&self.app, &w, shown)
    }

    /// An installed update: exit as the tray's Quit does (`qt-app` has made
    /// its quit request); `RunEvent::Exit` runs the shutdown and then
    /// relaunches the exe ([`relaunch`]).
    fn restart(&self) {
        self.app.exit(0);
    }
}

/// What [`foreground_pid`] reports for a window of this process that is not
/// the overlay: pid 0, the System Idle Process, which owns no window and so
/// is neither the game's pid nor this process's, and the overlay is hidden.
const NOT_THE_OVERLAY: u32 = 0;

/// The process id of the foreground window: only the id, never its title,
/// class or process name. `None` when there is no foreground window (focus
/// is moving) or its process cannot be read.
///
/// This process's own pid stands for the overlay, so it is reported only
/// when the window in front is the overlay (`overlay`, its handle) or a
/// window it owns. Another window of this process — the tray's hidden
/// window, which Tauri's tray brings to the front while its menu is open —
/// is reported as [`NOT_THE_OVERLAY`]: otherwise opening the tray menu would
/// show the hidden overlay again, and its Quit would never run while it is
/// hidden. Only window handles are compared.
#[cfg(windows)]
fn foreground_pid(overlay: Option<isize>) -> Option<u32> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetAncestor, GetForegroundWindow, GetWindowThreadProcessId, GA_ROOTOWNER,
    };
    // SAFETY: the calls take no pointer but the out-parameter, a live local;
    // `GetAncestor` on a window gone since returns null, compared only.
    let (pid, root) = unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return None;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        (pid, GetAncestor(hwnd, GA_ROOTOWNER) as isize)
    };
    if pid == 0 {
        return None;
    }
    match overlay {
        Some(overlay) if pid == std::process::id() && root != overlay => Some(NOT_THE_OVERLAY),
        _ => Some(pid),
    }
}

#[cfg(not(windows))]
fn foreground_pid(_overlay: Option<isize>) -> Option<u32> {
    None
}

/// Shows the overlay with `SW_SHOWNOACTIVATE`, so showing never takes focus
/// from the game, or hides it with `SW_HIDE`. Posted to the main thread,
/// which owns the window, so the caller (the worker, under the app's lock)
/// never waits for it.
#[cfg(windows)]
fn show_without_activating(app: &AppHandle, window: &WebviewWindow, shown: bool) -> Result<(), String> {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE, SW_SHOWNOACTIVATE};
    // A raw handle is not `Send`; its value is carried to the main thread.
    let handle = window.hwnd().map_err(|e| e.to_string())?.0 as isize;
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
fn show_without_activating(_app: &AppHandle, window: &WebviewWindow, shown: bool) -> Result<(), String> {
    if shown { window.show() } else { window.hide() }.map_err(|e| e.to_string())
}

/// Adds `WS_EX_NOACTIVATE` to the window in play mode, so clicking it never
/// takes focus from the game, and logs the handle with the extended style
/// before and after (the live check reads the bits from this line). A launch
/// that fell back to edit mode (no shortcut) leaves the bit off, so the
/// window can take keys.
#[cfg(windows)]
fn no_activate(window: &WebviewWindow, shell: &App) {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_NOACTIVATE,
    };
    let play = shell.get_view().mode == Mode::Play;
    match window.hwnd() {
        Ok(handle) => {
            let hwnd = handle.0 as HWND;
            // SAFETY: `hwnd` is this process's live overlay window, read and
            // written on the main thread that owns it.
            let (before, after) = unsafe {
                let before = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                if play {
                    SetWindowLongPtrW(hwnd, GWL_EXSTYLE, before | WS_EX_NOACTIVATE as isize);
                }
                (before, GetWindowLongPtrW(hwnd, GWL_EXSTYLE))
            };
            shell.diag(&format!(
                "window handle 0x{:x}: extended style 0x{:08x} -> 0x{:08x}",
                hwnd as usize, before, after
            ));
        }
        Err(e) => shell.diag(&format!("error: window handle: {e}")),
    }
}

#[cfg(not(windows))]
fn no_activate(_window: &WebviewWindow, shell: &App) {
    shell.diag("window: no-activate style is Windows-only");
}

/// The screen reader's backend: `qt-capture`'s, given the overlay's window
/// handle, read here as `no_activate` reads it, so the reader can check the
/// overlay's own rect and visibility with Win32 calls instead of waiting on
/// this main thread. A handle that cannot be read is passed as none
/// (`no_activate` logs why); the backend then sees no overlay, and the
/// reader pauses: it copies and reads nothing, writes its `screen: overlay
/// check off (no overlay window)` line and shows the status "Screen reading
/// is paused: the overlay cannot find its own window."
#[cfg(windows)]
fn screen_backend(window: &WebviewWindow) -> Option<Box<dyn ScreenBackend>> {
    let overlay = window.hwnd().ok().map(|handle| handle.0 as isize);
    Some(qt_capture::backend(overlay))
}

/// Elsewhere there is no backend, and the `screen_reading` setting starts
/// nothing.
#[cfg(not(windows))]
fn screen_backend(_window: &WebviewWindow) -> Option<Box<dyn ScreenBackend>> {
    None
}

/// The panel watcher's backend: a second `qt-capture` backend from the same
/// overlay handle, never the screen reader's (the reader holds its own
/// through each OCR call). A handle that cannot be read is passed as none
/// here too; the watcher then pauses, never hiding the overlay, with its
/// `panels: paused (no overlay window)` line and the status "Hiding under
/// game panels is paused: the overlay cannot find its own window."
#[cfg(windows)]
fn panel_backend(window: &WebviewWindow) -> Option<Box<dyn ScreenBackend>> {
    let overlay = window.hwnd().ok().map(|handle| handle.0 as isize);
    Some(qt_capture::backend(overlay))
}

/// Elsewhere there is no backend, and no panel watcher starts.
#[cfg(not(windows))]
fn panel_backend(_window: &WebviewWindow) -> Option<Box<dyn ScreenBackend>> {
    None
}

/// Runs `f` on a blocking task with the shell, if it is up yet.
fn later(app: &AppHandle, f: impl FnOnce(&App) + Send + 'static) {
    if let Some(shell) = app.try_state::<Arc<App>>() {
        let shell = Arc::clone(&shell);
        tauri::async_runtime::spawn_blocking(move || f(&shell));
    }
}

/// Makes the quit request on a blocking task, then exits the app, whose
/// `RunEvent::Exit` runs the shutdown. A second request writes nothing and
/// exits the same way. Before the shell is up there is nothing to log, so
/// the app exits at once.
fn quit(app: &AppHandle, from: QuitFrom) {
    match app.try_state::<Arc<App>>() {
        Some(shell) => {
            let shell = Arc::clone(&shell);
            let app = app.clone();
            tauri::async_runtime::spawn_blocking(move || {
                shell.request_quit(from);
                app.exit(0);
            });
        }
        None => app.exit(0),
    }
}

/// The tray menu shown now, so a view that leaves the menu as it is (a
/// check starting, the setting turned) rebuilds nothing, and two threads
/// never interleave a rebuild: the lock is held across one, and the main
/// thread, which the rebuild waits for, never takes it once the worker runs.
#[derive(Default)]
struct TrayShown(Mutex<Option<TrayMenu>>);

impl TrayShown {
    /// Builds the tray icon with the menu and tooltip for `view`, at setup,
    /// before the worker starts. A failure is the caller's to log; the
    /// overlay runs on without the icon.
    fn build(&self, app: &AppHandle, view: &AppSettingsView) -> Result<(), String> {
        let model = qt_app::tray_menu(view);
        let mut shown = self.0.lock().unwrap_or_else(|p| p.into_inner());
        let icon = app
            .default_window_icon()
            .cloned()
            .ok_or_else(|| "the app has no default window icon".to_string())?;
        let menu = tray_menu(app, &model)?;
        TrayIconBuilder::with_id(TRAY)
            .icon(icon)
            .tooltip(&model.tooltip)
            .menu(&menu)
            .show_menu_on_left_click(true)
            .on_menu_event(|app, event| tray_action(app, event.id().as_ref()))
            .build(app)
            .map_err(|e| e.to_string())?;
        *shown = Some(model);
        Ok(())
    }

    /// Brings the tray's menu and tooltip up to `view`, when they differ
    /// from the ones shown. Nothing when the tray icon was never built.
    fn update(&self, app: &AppHandle, view: &AppSettingsView) -> Result<(), String> {
        let model = qt_app::tray_menu(view);
        let mut shown = self.0.lock().unwrap_or_else(|p| p.into_inner());
        if shown.as_ref() == Some(&model) {
            return Ok(());
        }
        let Some(tray) = app.tray_by_id(TRAY) else {
            return Ok(());
        };
        let menu = tray_menu(app, &model)?;
        tray.set_menu(Some(menu)).map_err(|e| e.to_string())?;
        tray.set_tooltip(Some(&model.tooltip))
            .map_err(|e| e.to_string())?;
        *shown = Some(model);
        Ok(())
    }
}

/// The tray's menu, rendered from `qt-app`'s entries in their order.
fn tray_menu(app: &AppHandle, model: &TrayMenu) -> Result<Menu<tauri::Wry>, String> {
    let menu = Menu::new(app).map_err(|e| e.to_string())?;
    for item in &model.items {
        match item {
            TrayItem::Entry { id, text } => {
                let entry = MenuItem::with_id(app, *id, text, true, None::<&str>)
                    .map_err(|e| e.to_string())?;
                menu.append(&entry).map_err(|e| e.to_string())?;
            }
            TrayItem::Separator => {
                let line = PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?;
                menu.append(&line).map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(menu)
}

/// What a tray entry does, by its id (`TrayAction::from_id`). Called on the
/// main thread, so every action is handed on: the app's work to a blocking
/// task, the window to a spawned one.
fn tray_action(app: &AppHandle, id: &str) {
    match TrayAction::from_id(id) {
        Some(TrayAction::CheckForUpdates) => {
            open_settings(app);
            later(app, |shell| {
                shell.check_for_updates();
            });
        }
        Some(TrayAction::OpenSettings) => open_settings(app),
        Some(TrayAction::Quit) => quit(app, QuitFrom::Tray),
        None => {}
    }
}

/// Opens the Settings window, or shows, unminimises and focuses it when it
/// is open already, on a blocking task: never in the tray's synchronous
/// menu handler, where Tauri's window builder deadlocks on Windows. The
/// lock keeps two quick clicks from building it twice. A failure is logged.
fn open_settings(app: &AppHandle) {
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
                .focusable(true)
                .focused(true)
                .always_on_top(false)
                .skip_taskbar(false)
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
// The commands (Context "IPC contract").

#[tauri::command]
fn get_view(shell: Shell<'_>) -> View {
    shell.get_view()
}

#[tauri::command(async)]
fn set_mode(shell: Shell<'_>, mode: Mode) -> Result<View, String> {
    shell.set_mode(mode)
}

#[tauri::command(async)]
fn set_view_kind(shell: Shell<'_>, kind: ViewKind) -> Result<View, String> {
    shell.set_view_kind(kind)
}

#[tauri::command(async)]
fn set_collapsed(shell: Shell<'_>, collapsed: bool) -> Result<View, String> {
    shell.set_collapsed(collapsed)
}

#[tauri::command(async)]
fn toggle_act(shell: Shell<'_>, act: u32) -> Result<View, String> {
    shell.toggle_act(act)
}

#[tauri::command(async)]
fn open_detail(shell: Shell<'_>, quest_id: Option<String>) -> Result<View, String> {
    shell.open_detail(quest_id)
}

#[tauri::command(async)]
fn open_frame(shell: Shell<'_>, frame: Option<Frame>) -> Result<View, String> {
    shell.open_frame(frame)
}

#[tauri::command(async)]
fn quest_action(shell: Shell<'_>, quest_id: String, action: QuestAction) -> Result<View, String> {
    shell.quest_action(&quest_id, action)
}

#[tauri::command(async)]
fn tick_step(
    shell: Shell<'_>,
    quest_id: String,
    step_id: String,
    ticked: bool,
) -> Result<View, String> {
    shell.tick_step(&quest_id, &step_id, ticked)
}

#[tauri::command(async)]
fn select_character(shell: Shell<'_>, name: String) -> Result<View, String> {
    shell.select_character(&name)
}

#[tauri::command(async)]
fn pin_character(shell: Shell<'_>, name: Option<String>) -> Result<View, String> {
    shell.pin_character(name.as_deref())
}

#[tauri::command(async)]
fn confirm_character(shell: Shell<'_>, name: String) -> Result<View, String> {
    shell.confirm_character(&name)
}

#[tauri::command(async)]
fn update_settings(shell: Shell<'_>, patch: Value) -> Result<View, String> {
    shell.update_settings(&patch)
}

#[tauri::command(async)]
fn open_source(shell: Shell<'_>, quest_id: String) -> Result<(), String> {
    shell.open_source(&quest_id)
}

// The Settings window's commands.

#[tauri::command]
fn get_app_settings(shell: Shell<'_>) -> AppSettingsView {
    shell.get_app_settings()
}

#[tauri::command(async)]
fn check_for_updates(shell: Shell<'_>) -> AppSettingsView {
    shell.check_for_updates()
}

#[tauri::command(async)]
fn install_update(shell: Shell<'_>) -> AppSettingsView {
    shell.install_update()
}

#[tauri::command(async)]
fn cancel_update(shell: Shell<'_>) -> AppSettingsView {
    shell.cancel_update()
}

// ---------------------------------------------------------------------------
// The restart into an installed update, and the start-up cleanup.

/// After the shutdown of an exit an installed update asked for
/// (`App::restart_requested`): frees every global shortcut, so the new
/// process can register its own (a shortcut still taken would start it in
/// edit mode with an error), then starts `exe`, the path captured at start
/// and never read again, with this launch's arguments. The single-instance
/// plugin has released its lock by now: plugins hear `RunEvent::Exit`
/// before this callback. A failed start is logged; the new exe is in place
/// either way and runs at the next start.
fn relaunch(app: &AppHandle, shell: &App, exe: Option<&PathBuf>) {
    if let Err(e) = app.global_shortcut().unregister_all() {
        shell.diag(&format!("error: freeing the shortcut before the restart: {e}"));
    }
    let Some(exe) = exe else {
        shell.diag("error: restart: the app's own path is unknown");
        return;
    };
    match std::process::Command::new(exe)
        .args(std::env::args_os().skip(1))
        .spawn()
    {
        Ok(_) => shell.diag("updates: restarted into the new exe"),
        Err(e) => shell.diag(&format!("error: restart: the new exe did not start ({})", e.kind())),
    }
}

/// At start, a background thread removes what an earlier install left
/// beside `exe` (`<exe>.old`, a stale `<exe>.download`), retrying for up to
/// about 30 s while the old process may still be ending, and logs one
/// `updates: cleanup:` line. Not waited for.
fn start_cleanup(shell: &Arc<App>, exe: &PathBuf) {
    let log = Arc::clone(shell);
    if let Err(e) = qt_app::updates::spawn_cleanup(exe.clone(), move |line| log.diag(&line)) {
        shell.diag(&format!("error: the update cleanup thread did not start: {e}"));
    }
}

// ---------------------------------------------------------------------------

/// Builds and runs the Tauri application.
pub fn run() {
    let (options, options_error) = match qt_app::cli::parse(std::env::args_os().skip(1)) {
        Ok(o) => (o, None),
        Err(e) => (Options::default(), Some(e)),
    };
    // The exe an install replaces and the restart relaunches, captured once,
    // before any swap could rename it. Without it no install can run.
    let exe = std::env::current_exe();
    let exe_path = exe.as_ref().ok().cloned();
    let relaunch_exe = exe_path.clone();

    let app = tauri::Builder::default()
        // First, so a second launch exits before it sets anything up.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            later(app, |shell| shell.second_instance());
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        later(app, |shell| {
                            let _ = shell.toggle_mode();
                        });
                    }
                })
                .build(),
        )
        .setup(move |app| {
            let handle = app.handle().clone();
            let window = app
                .get_webview_window(OVERLAY)
                .ok_or("tauri.conf.json has no overlay window")?;
            // The smallest size the edges can give the window, whatever the
            // zoom (`qt_app::MIN_WINDOW_SIZE`); logged once the shell is up.
            let (min_width, min_height) = MIN_WINDOW_SIZE;
            let min_size = window.set_min_size(Some(LogicalSize::new(min_width, min_height)));
            let panels = PanelConfig {
                backend: panel_backend(&window),
                table: None,
            };
            // The key is `ep-update`'s built-in release key: the glue never
            // sets one.
            let updates = UpdateConfig {
                transport: Some(Box::new(NetTransport)),
                exe_path: exe_path.clone(),
                ..UpdateConfig::default()
            };
            let tray = Arc::new(TrayShown::default());
            let shell = App::with_updates(
                AppConfig {
                    options,
                    options_error,
                    store: Store::open_default().map_err(|e| e.to_string()),
                    data: None,
                    candidates: qt_app::discover::default_candidates(),
                    poll_interval: DEFAULT_POLL_INTERVAL,
                    emitter: Box::new(TauriEmitter {
                        app: handle.clone(),
                        tray: Arc::clone(&tray),
                    }),
                    window: Box::new(TauriWindow {
                        app: handle,
                        hotkey: Mutex::new(None),
                    }),
                    clock: Box::new(SystemClock::default()),
                    screen: screen_backend(&window),
                },
                panels,
                updates,
            );
            app.manage(Arc::clone(&shell));
            // Before the worker starts, so no check can change the menu
            // while it is built; logged after the start's own lines.
            let tray_built = tray.build(app.handle(), &shell.get_app_settings());
            if let Err(e) = min_size {
                shell.diag(&format!("error: window minimum size: {e}"));
            }
            shell.start();
            no_activate(&window, &shell);
            match tray_built {
                Ok(()) => shell.diag("tray: icon shown"),
                Err(e) => shell.diag(&format!("error: tray icon: {e}")),
            }
            match &exe {
                Ok(path) => start_cleanup(&shell, path),
                Err(e) => shell.diag(&format!(
                    "error: the app's own path is unknown ({}); updates cannot be installed",
                    e.kind()
                )),
            }
            Ok(())
        })
        .on_window_event(|window, event| match event {
            // Every move of the overlay, in any mode: `auto` follows the
            // window to its monitor (Tauri has no drag-end event; the shell
            // applies a zoom only when it differs), and an edit-mode move is
            // kept. The Settings window's moves are its own.
            WindowEvent::Moved(at) if window.label() == OVERLAY => {
                let (x, y) = (at.x, at.y);
                later(window.app_handle(), move |shell| shell.window_moved(x, y));
            }
            // Every resize of the overlay, in logical pixels, handed over off
            // the main thread as moves are: one made in edit mode (by the
            // panel's edges) is kept as the window's size.
            WindowEvent::Resized(size) if window.label() == OVERLAY => {
                let Ok(dpi) = window.scale_factor() else {
                    return;
                };
                let logical = size.to_logical::<f64>(dpi);
                let (width, height) = (logical.width, logical.height);
                later(window.app_handle(), move |shell| shell.window_resized(width, height));
            }
            WindowEvent::ScaleFactorChanged { .. } if window.label() == OVERLAY => {
                later(window.app_handle(), |shell| shell.follow_monitor());
            }
            // `WM_CLOSE` to the overlay, shown or hidden: hold the close and
            // quit the whole app, so the shutdown runs and the process ends.
            WindowEvent::CloseRequested { api, .. } if window.label() == OVERLAY => {
                api.prevent_close();
                quit(window.app_handle(), QuitFrom::WindowClosed);
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            get_view,
            set_mode,
            set_view_kind,
            set_collapsed,
            toggle_act,
            open_detail,
            open_frame,
            quest_action,
            tick_step,
            select_character,
            pin_character,
            confirm_character,
            update_settings,
            open_source,
            get_app_settings,
            check_for_updates,
            install_update,
            cancel_update,
        ])
        .build(tauri::generate_context!())
        .expect("quest-tracker: the Tauri application failed to start");

    app.run(move |app, event| {
        if let RunEvent::Exit = event {
            if let Some(shell) = app.try_state::<Arc<App>>() {
                shell.shutdown(SHUTDOWN_TIMEOUT);
                if shell.restart_requested() {
                    relaunch(app, &shell, relaunch_exe.as_ref());
                }
            }
        }
    });
}
