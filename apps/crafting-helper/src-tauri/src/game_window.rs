//! Whether the game is the window in front, by its title: the one Rust
//! source of the app that reads the window in front and a window's title
//! (`tests/test_crafting_helper_guard.py` pins it).
//!
//! The title is compared with `ch_app::GAME_WINDOW_TITLE` here, where it is
//! read, and dropped: another window's title is never stored, returned,
//! logged or passed to `ch-app`. Only the game's handle leaves this file.
//! A window of this process (the main, Settings or overlay window) is never
//! the game, and its title is not read: reading it would send a message to
//! the main thread and wait for it. A window of another process has its
//! title read from the system's copy, which sends that program nothing.
//!
//! Not by process name (that would open the game's process) and not by
//! `Client.txt`'s pid (the app reads no game file).

/// The handle of the window in front when it is the game's (titled exactly
/// `ch_app::GAME_WINDOW_TITLE`), as an integer; `None` when another window,
/// or none, is in front.
#[cfg(windows)]
pub fn game_in_front() -> Option<isize> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
    };

    // SAFETY: takes nothing; returns the window in front, or null.
    let window = unsafe { GetForegroundWindow() };
    if window.is_null() {
        return None;
    }
    let mut pid = 0u32;
    // SAFETY: `window` is a window handle (stale at worst, when the call
    // returns 0 and leaves `pid` 0); `pid` is a local this call writes.
    unsafe { GetWindowThreadProcessId(window, &mut pid) };
    if pid == 0 || pid == std::process::id() {
        return None;
    }
    let want: Vec<u16> = ch_app::GAME_WINDOW_TITLE.encode_utf16().collect();
    // SAFETY: as above; reads the title's length only.
    let len = unsafe { GetWindowTextLengthW(window) };
    if usize::try_from(len).ok() != Some(want.len()) {
        return None;
    }
    let mut title = vec![0u16; want.len() + 1];
    let room = i32::try_from(title.len()).ok()?;
    // SAFETY: `title` holds `room` u16s, the count passed; the call writes
    // at most that many, its terminating zero included.
    let got = unsafe { GetWindowTextW(window, title.as_mut_ptr(), room) };
    let same = usize::try_from(got).ok() == Some(want.len()) && title[..want.len()] == want[..];
    same.then_some(window as isize)
}

/// No window in front off Windows: the app is a Windows app; this keeps the
/// workspace building elsewhere.
#[cfg(not(windows))]
pub fn game_in_front() -> Option<isize> {
    None
}
