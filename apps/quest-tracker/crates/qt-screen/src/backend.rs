//! The backend trait the reader thread drives. The real one is `qt-capture`
//! (Win32 and `Windows.Media.Ocr`); tests drive a fake that replays a script.
//!
//! The reader calls it only on its own thread, never under an `App` lock and
//! never through a Tauri window getter. A backend is built elsewhere (the
//! glue, at setup) and moved onto that thread, so it must be `Send`; anything
//! that has to live on the reader's thread (WinRT's apartment, the OCR
//! engine) is created on first use there.

use std::fmt;

use crate::geometry::PixelRect;
use crate::observation::{Image, PixelLine};

/// The game's top-level window, as the backend knows it (an `HWND` on
/// Windows). Opaque to everything but the backend that returned it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WindowHandle(pub isize);

/// The overlay's own window: where it is, in screen pixels, and whether it is
/// shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OverlayState {
    pub rect: PixelRect,
    pub visible: bool,
}

/// Whether the OCR can read English.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OcrReadiness {
    /// Ready, with the language tag it reads (`en-US`).
    Ready { language: String },
    /// Windows has no English OCR: the reader refuses and ends.
    NoEnglish,
}

/// A backend call that failed. Its message names the call and the system's
/// error, never OCR text or pixels.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BackendError {
    pub message: String,
}

impl BackendError {
    pub fn new(message: impl Into<String>) -> Self {
        BackendError {
            message: message.into(),
        }
    }
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for BackendError {}

/// What the reader needs from the system, one call per question.
pub trait ScreenBackend: Send {
    /// The game's window for process `pid`: its largest visible top-level
    /// window, or `None` when it has none.
    fn game_window(&mut self, pid: u32) -> Result<Option<WindowHandle>, BackendError>;

    /// The process that owns the foreground window, if any.
    fn foreground_pid(&mut self) -> Result<Option<u32>, BackendError>;

    /// The window's client area in physical screen pixels, or `None` when
    /// it is minimized or empty.
    fn client_rect(&mut self, window: WindowHandle) -> Result<Option<PixelRect>, BackendError>;

    /// The overlay's own window, from the handle the backend was given at
    /// construction; `None` when it has none.
    fn overlay(&mut self) -> Result<Option<OverlayState>, BackendError>;

    /// A copy of `area` (screen pixels, inside the window's client rect).
    fn capture(&mut self, window: WindowHandle, area: PixelRect) -> Result<Image, BackendError>;

    /// Whether the OCR can read English. Called once, before the first OCR.
    fn ocr_readiness(&mut self) -> Result<OcrReadiness, BackendError>;

    /// The lines OCR reads in `image`, their boxes in the image's own pixels.
    fn ocr(&mut self, image: &Image) -> Result<Vec<PixelLine>, BackendError>;
}
