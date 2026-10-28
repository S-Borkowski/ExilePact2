//! The waystone roller's Tauri glue around `wr-app`, which holds every
//! decision. This file only connects it:
//!
//! - **Shell**: [`TauriShell`] implements `wr_app::Shell`: the one global
//!   shortcut through `tauri-plugin-global-shortcut` (Windows'
//!   `RegisterHotKey`, system-wide and exclusive; a combination another
//!   program holds is refused and the old one stays registered), the
//!   clipboard written through Win32 with the window's handle, the error
//!   beep, and the `wr:view` event.
//! - **Commands**: `get_view` (sync) and the async `update_selection`,
//!   `copy_now` and `set_hotkey`, each forwarded to the `App` method of the
//!   same name.
//! - **Single instance**: a second launch exits and brings this window up.
//! - **Window**: ordinary, titled "Waystone Roller", resizable, in the
//!   taskbar; closing it quits, after the settings are written.
//!
//! The app sends nothing to the game: no input, no window message, no
//! memory read. Its only effect outside its window is the clipboard and the
//! beep.

use std::sync::{Arc, Mutex};

use serde_json::Value;
use tauri::{AppHandle, Emitter as _, Manager, RunEvent};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use wr_app::{App, AppConfig, BeepAttempt, HotkeyError, Store, View};

/// The window's label (`tauri.conf.json`).
const MAIN: &str = "main";
/// The event that carries a fresh view to the page.
const VIEW_EVENT: &str = "wr:view";

type Shared<'a> = tauri::State<'a, Arc<App>>;

// ---------------------------------------------------------------------------
// The shell, for wr-app.

struct TauriShell {
    app: AppHandle,
    /// The shortcut registered now, if any.
    hotkey: Mutex<Option<Shortcut>>,
}

impl wr_app::Shell for TauriShell {
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

    fn write_clipboard(&self, text: &str) -> Result<(), String> {
        let window = self
            .app
            .get_webview_window(MAIN)
            .ok_or("the window is gone")?;
        let hwnd = window.hwnd().map_err(|e| e.to_string())?.0 as isize;
        clipboard::write(hwnd, text)
    }

    fn beep(&self) -> Vec<BeepAttempt> {
        clipboard::beep()
    }

    fn emit_view(&self, view: &View) {
        let _ = self.app.emit_to(MAIN, VIEW_EVENT, view);
    }
}

/// The Win32 clipboard and the beep.
#[cfg(windows)]
mod clipboard {
    use std::time::Duration;

    use windows_sys::Win32::Foundation::{GetLastError, GlobalFree, HWND};
    use windows_sys::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
    };
    use windows_sys::Win32::System::Diagnostics::Debug::{Beep, MessageBeep};
    use windows_sys::Win32::System::Memory::{
        GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE,
    };
    use windows_sys::Win32::System::Ole::CF_UNICODETEXT;
    use wr_app::BeepAttempt;

    /// The refusal tone: `Beep` plays a sine tone through the default audio
    /// device and reads no entry of the sound scheme, so a muted or empty
    /// "Critical Stop" entry cannot silence it. A muted "System Sounds"
    /// level, or a default output device nobody is listening to, could
    /// (not ruled out). Live 2 heard nothing from `MessageBeep(MB_ICONHAND)`
    /// and heard this tone; see
    /// docs/waystone-roller/waystone-roller-research.md § "The built app in
    /// the game (Live 2)". It blocks for its duration.
    const TONE_HZ: u32 = 880;
    const TONE_MS: u32 = 250;
    /// `MessageBeep`'s "simple beep", the fallback when `Beep` fails.
    const SIMPLE_BEEP: u32 = 0xFFFF_FFFF;

    /// How often, and how far apart, opening a clipboard another program
    /// holds is tried again.
    const OPEN_TRIES: u32 = 10;
    const OPEN_PAUSE: Duration = Duration::from_millis(20);

    /// Puts `text` on the clipboard as `CF_UNICODETEXT`, owned by the
    /// window `hwnd` (with a null owner `SetClipboardData` fails after
    /// `EmptyClipboard`).
    pub fn write(hwnd: isize, text: &str) -> Result<(), String> {
        let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        let bytes = wide.len() * std::mem::size_of::<u16>();

        let mut opened = false;
        for _ in 0..OPEN_TRIES {
            // SAFETY: `hwnd` is this app's own live window handle, read from
            // Tauri just before; OpenClipboard only records it as the owner.
            if unsafe { OpenClipboard(hwnd as HWND) } != 0 {
                opened = true;
                break;
            }
            std::thread::sleep(OPEN_PAUSE);
        }
        if !opened {
            return Err("another program is holding the clipboard".to_string());
        }

        let result = (|| {
            // SAFETY: the clipboard is open by this thread (above).
            if unsafe { EmptyClipboard() } == 0 {
                return Err("the clipboard could not be emptied".to_string());
            }
            // SAFETY: a plain allocation of `bytes` bytes; checked for null.
            let block = unsafe { GlobalAlloc(GMEM_MOVEABLE, bytes) };
            if block.is_null() {
                return Err("no memory for the clipboard text".to_string());
            }
            // SAFETY: `block` is the moveable block just allocated; locking
            // it gives a pointer to its `bytes` bytes, checked for null.
            let dst = unsafe { GlobalLock(block) } as *mut u16;
            if dst.is_null() {
                // SAFETY: `block` is ours and was never handed to the clipboard.
                unsafe { GlobalFree(block) };
                return Err("the clipboard text could not be prepared".to_string());
            }
            // SAFETY: `dst` points to `bytes` writable bytes, which is
            // exactly `wide.len()` u16s; the two buffers do not overlap.
            unsafe {
                std::ptr::copy_nonoverlapping(wide.as_ptr(), dst, wide.len());
                GlobalUnlock(block);
            }
            // SAFETY: the clipboard is open and emptied by this thread and
            // `block` is an unlocked moveable global block holding a
            // NUL-terminated UTF-16 string. On success the system owns it.
            if unsafe { SetClipboardData(CF_UNICODETEXT as u32, block) }.is_null() {
                // SAFETY: the clipboard refused the block, so it is still ours.
                unsafe { GlobalFree(block) };
                return Err("the clipboard refused the text".to_string());
            }
            Ok(())
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

/// No clipboard writer off Windows: the app is a Windows app; this keeps
/// the workspace building elsewhere.
#[cfg(not(windows))]
mod clipboard {
    pub fn write(_hwnd: isize, _text: &str) -> Result<(), String> {
        Err("the clipboard is written only on Windows".to_string())
    }

    pub fn beep() -> Vec<wr_app::BeepAttempt> {
        Vec::new()
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

#[tauri::command(async)]
fn update_selection(app: Shared<'_>, patch: Value) -> Result<View, String> {
    app.update_selection(&patch)
}

#[tauri::command(async)]
fn copy_now(app: Shared<'_>) -> View {
    app.copy_now()
}

#[tauri::command(async)]
fn set_hotkey(app: Shared<'_>, hotkey: String) -> View {
    app.set_hotkey(&hotkey)
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
        .setup(|app| {
            let store = Store::open_default();
            let data = match wr_app::embedded_data() {
                Ok(d) => d,
                Err(e) => {
                    // A broken build: say so where a bug report can find it.
                    if let Ok(s) = &store {
                        let _ = s.diag().write(&format!("error: {e}"));
                    }
                    return Err(e.into());
                }
            };
            let shell = App::new(AppConfig {
                data,
                store,
                shell: Box::new(TauriShell {
                    app: app.handle().clone(),
                    hotkey: Mutex::new(None),
                }),
            });
            app.manage(Arc::clone(&shell));
            shell.start();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_view,
            update_selection,
            copy_now,
            set_hotkey,
        ])
        .build(tauri::generate_context!())
        .expect("waystone-roller: the Tauri application failed to start");

    app.run(|app, event| {
        if let RunEvent::Exit = event {
            if let Some(shell) = app.try_state::<Arc<App>>() {
                shell.shutdown();
            }
        }
    });
}
