//! The screen reader's real backend: [`qt_screen::ScreenBackend`] on Windows.
//!
//! - The game's window by pid (its largest visible top-level window, as the
//!   research's `capture.ps1` picks it), its client rect in physical screen
//!   pixels, and the process in front.
//! - The overlay's own rect and visibility, from the window handle the glue
//!   gives [`backend`] at setup, read with Win32 calls so the reader never
//!   waits on Tauri's main thread.
//! - A GDI copy of a region of the screen.
//! - `Windows.Media.Ocr`, en-US, one engine, on WinRT initialized
//!   multithreaded on the calling thread (the reader's). Images are upscaled
//!   as `ocr.ps1` did (scale 2, lowered to keep within the engine's maximum
//!   dimension) and the boxes mapped back to the copy's own pixels.
//!
//! OCR text is private: it is returned to the caller and nothing here
//! formats, logs or keeps it. Errors name the call and the system's error.
//!
//! [`pixels`] holds the pure arithmetic and builds everywhere; the backend
//! itself only on Windows.

pub mod pixels;

#[cfg(windows)]
mod win;

#[cfg(windows)]
pub use win::{WindowsBackend, OCR_LANGUAGE};

/// The real backend as `qt-screen`'s trait object. `overlay_hwnd` is the
/// overlay window's `HWND` as an integer, or `None` when it has none.
#[cfg(windows)]
pub fn backend(overlay_hwnd: Option<isize>) -> Box<dyn qt_screen::ScreenBackend> {
    Box::new(WindowsBackend::new(overlay_hwnd))
}
