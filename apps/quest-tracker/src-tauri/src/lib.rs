//! Tauri glue for the quest-tracker overlay: the `overlay` window, the
//! commands as thin wrappers over `qt-app`, and the emitter that sends
//! `qt:view` and `qt:row-change`. Every decision is `qt-app`'s; this file only
//! connects it to Tauri.
//!
//! - **Window**: the one `overlay` window of `tauri.conf.json` (transparent,
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
//!   is the game's or its own. Showing uses `SW_SHOWNOACTIVATE`, so the game
//!   keeps focus; hiding `SW_HIDE`.
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
//!   ticked })`, `invoke("update_settings", { patch })`.
//! - **Auto scale**: every window move and every DPI scale change of the
//!   window has `qt-app` re-read the monitor, which re-applies the zoom only
//!   when it differs from the one applied.
//! - **Main-thread callbacks** (the shortcut, window moves, a DPI scale
//!   change, a second launch)
//!   hand their work to a blocking task, so the main thread never waits for
//!   the app's lock while a command holding it waits for the main thread.

use std::sync::{Arc, Mutex};

use qt_app::{
    App, AppConfig, Frame, Mode, MonitorMetrics, Options, Placement, QuestAction, RowChange, Store,
    SystemClock, View, ViewKind, DEFAULT_POLL_INTERVAL, SHUTDOWN_TIMEOUT,
};
use serde_json::Value;
use tauri::{
    AppHandle, Emitter as _, LogicalSize, Manager, PhysicalPosition, RunEvent, WebviewWindow,
    WindowEvent,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use tauri_plugin_opener::OpenerExt;

/// The one window's label.
const OVERLAY: &str = "overlay";

/// The window's size at scale 1, as `tauri.conf.json` sets it.
const WIDTH: f64 = 360.0;
const HEIGHT: f64 = 640.0;

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

struct TauriEmitter(AppHandle);

impl qt_app::Emitter for TauriEmitter {
    fn view(&self, view: &View) {
        let _ = self.0.emit_to(OVERLAY, "qt:view", view);
    }

    fn row_change(&self, change: &RowChange) {
        let _ = self.0.emit_to(OVERLAY, "qt:row-change", change);
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

    fn set_scale(&self, factor: f64) -> Result<(), String> {
        let w = self.window()?;
        w.set_zoom(factor).map_err(|e| e.to_string())?;
        w.set_size(LogicalSize::new(WIDTH * factor, HEIGHT * factor))
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
        foreground_pid()
    }

    fn set_visible(&self, shown: bool) -> Result<(), String> {
        let w = self.window()?;
        show_without_activating(&self.app, &w, shown)
    }
}

/// The process id of the foreground window: only the id, never its title,
/// class or process name. `None` when there is no foreground window (focus
/// is moving) or its process cannot be read.
#[cfg(windows)]
fn foreground_pid() -> Option<u32> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};
    // SAFETY: both calls take no pointer but the out-parameter, a live local.
    let pid = unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return None;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        pid
    };
    (pid != 0).then_some(pid)
}

#[cfg(not(windows))]
fn foreground_pid() -> Option<u32> {
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

/// Runs `f` on a blocking task with the shell, if it is up yet.
fn later(app: &AppHandle, f: impl FnOnce(&App) + Send + 'static) {
    if let Some(shell) = app.try_state::<Arc<App>>() {
        let shell = Arc::clone(&shell);
        tauri::async_runtime::spawn_blocking(move || f(&shell));
    }
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

// ---------------------------------------------------------------------------

/// Builds and runs the Tauri application.
pub fn run() {
    let (options, options_error) = match qt_app::cli::parse(std::env::args_os().skip(1)) {
        Ok(o) => (o, None),
        Err(e) => (Options::default(), Some(e)),
    };

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
            let shell = App::new(AppConfig {
                options,
                options_error,
                store: Store::open_default().map_err(|e| e.to_string()),
                data: None,
                candidates: qt_app::discover::default_candidates(),
                poll_interval: DEFAULT_POLL_INTERVAL,
                emitter: Box::new(TauriEmitter(handle.clone())),
                window: Box::new(TauriWindow {
                    app: handle,
                    hotkey: Mutex::new(None),
                }),
                clock: Box::new(SystemClock::default()),
            });
            app.manage(Arc::clone(&shell));
            shell.start();
            no_activate(&window, &shell);
            Ok(())
        })
        .on_window_event(|window, event| match event {
            // Every move, in any mode: `auto` follows the window to its
            // monitor (Tauri has no drag-end event; the shell applies a zoom
            // only when it differs), and an edit-mode move is kept.
            WindowEvent::Moved(at) => {
                let (x, y) = (at.x, at.y);
                later(window.app_handle(), move |shell| shell.window_moved(x, y));
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                later(window.app_handle(), |shell| shell.follow_monitor());
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
        ])
        .build(tauri::generate_context!())
        .expect("quest-tracker: the Tauri application failed to start");

    app.run(|app, event| {
        if let RunEvent::Exit = event {
            if let Some(shell) = app.try_state::<Arc<App>>() {
                shell.shutdown(SHUTDOWN_TIMEOUT);
            }
        }
    });
}
