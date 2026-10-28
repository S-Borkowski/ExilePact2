//! The paste's four primitives, and the only input the app sends: one key
//! event of Ctrl, A or V, whether any key is down, the title of the window
//! in front, and a pause. `wr-app` decides when, to which window, in what
//! order and with what pauses; each function here does one thing once.
//!
//! Nothing here sends mouse input, a window message or text, changes the
//! focus, or opens another process. The title is read for `wr-app`'s check
//! alone and never written anywhere. The repository guard
//! (`tests/test_waystone_roller_guard.py`) allows the keyboard calls in this
//! file and nowhere else.

use std::time::Duration;

pub use imp::{any_key_down, foreground_title, send_key};

/// Waits `duration` on the calling thread: the blocking task the shortcut's
/// copy and paste run on, never the main thread.
pub fn pause(duration: Duration) {
    std::thread::sleep(duration);
}

#[cfg(windows)]
mod imp {
    use windows_sys::Win32::Foundation::GetLastError;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        self, GetAsyncKeyState, MapVirtualKeyW, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT,
        KEYEVENTF_KEYUP, MAPVK_VK_TO_VSC, VIRTUAL_KEY, VK_A, VK_CONTROL, VK_V,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW,
    };
    use wr_app::Key;

    /// The virtual keys `any_key_down` reads: everything from Backspace up,
    /// so the mouse buttons (0x01 to 0x06) are not counted.
    const FIRST_KEY: i32 = 0x08;
    const LAST_KEY: i32 = 0xFE;

    /// The virtual key of `key`; there are only these three.
    fn virtual_key(key: Key) -> VIRTUAL_KEY {
        match key {
            Key::Ctrl => VK_CONTROL,
            Key::A => VK_A,
            Key::V => VK_V,
        }
    }

    /// Sends one keyboard event of `key`, down or up, with its virtual key
    /// and its scan code (so a game reading scan codes sees a real key).
    pub fn send_key(key: Key, down: bool) -> Result<(), String> {
        let vk = virtual_key(key);
        // SAFETY: MapVirtualKeyW maps a number to a number and touches no
        // memory of ours.
        let scan = unsafe { MapVirtualKeyW(u32::from(vk), MAPVK_VK_TO_VSC) } as u16;
        let event = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: scan,
                    dwFlags: if down { 0 } else { KEYEVENTF_KEYUP },
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        let size = std::mem::size_of::<INPUT>() as i32;
        // SAFETY: `event` is one fully initialised keyboard INPUT that lives
        // across the call, the count is 1 and `size` is INPUT's own size.
        let sent = unsafe { KeyboardAndMouse::SendInput(1, &event, size) };
        if sent == 1 {
            return Ok(());
        }
        // SAFETY: GetLastError reads this thread's last error code only,
        // right after the call that set it.
        let code = unsafe { GetLastError() };
        let error = std::io::Error::from_raw_os_error(code as i32);
        Err(format!("the key event was refused ({error})"))
    }

    /// Whether any key is down now: the high bit of each virtual key's
    /// asynchronous state, mouse buttons excluded.
    pub fn any_key_down() -> bool {
        (FIRST_KEY..=LAST_KEY).any(|vk| {
            // SAFETY: GetAsyncKeyState reads one key's state by number and
            // touches no memory of ours.
            let state = unsafe { GetAsyncKeyState(vk) };
            state < 0
        })
    }

    /// The title of the window in front, or `None` when there is none.
    pub fn foreground_title() -> Option<String> {
        // SAFETY: GetForegroundWindow takes nothing and returns a handle,
        // checked for null; it changes no window.
        let hwnd = unsafe { GetForegroundWindow() };
        if hwnd.is_null() {
            return None;
        }
        // SAFETY: `hwnd` was returned just above; the call only reads the
        // length of its title (0 for none, or a window that is gone).
        let len = unsafe { GetWindowTextLengthW(hwnd) };
        if len <= 0 {
            return Some(String::new());
        }
        let mut buf = vec![0u16; len as usize + 1];
        // SAFETY: `buf` holds `buf.len()` writable u16s and the count passed
        // is that length, so the call writes at most that many, NUL included.
        let copied = unsafe { GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
        let copied = usize::try_from(copied).unwrap_or(0).min(buf.len());
        Some(String::from_utf16_lossy(&buf[..copied]))
    }
}

/// No paste off Windows: the app is a Windows app; this keeps the workspace
/// building elsewhere. "No window in front" means `wr-app` sends nothing.
#[cfg(not(windows))]
mod imp {
    use wr_app::Key;

    pub fn send_key(_key: Key, _down: bool) -> Result<(), String> {
        Err("keys are sent only on Windows".to_string())
    }

    pub fn any_key_down() -> bool {
        false
    }

    pub fn foreground_title() -> Option<String> {
        None
    }
}
