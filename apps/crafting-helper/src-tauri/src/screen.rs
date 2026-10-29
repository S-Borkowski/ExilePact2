//! The screen watch's real backend: `ch_app::ScreenBackend` over the quest
//! tracker's Windows capture (`qt-capture`, by path and unchanged) and
//! `ch-watch`'s reader.
//!
//! - The game's window: the window in front when it is the game's
//!   ([`crate::game_window`]), and its client area in physical screen
//!   pixels from `qt-capture` (none while minimized).
//! - Whether Windows' OCR reads English, from `qt-capture`.
//! - The item tooltips in the game's client area: `ch-watch`'s reader over
//!   `qt-capture`'s calls (a GDI copy of the client, the overlay's own rect
//!   and visibility from the handle the backend is built with, and the
//!   OCR). Copies, crops and lines stay in memory, inside the read.
//!
//! `ch-app` moves it to its watch thread and calls it there alone, never
//! with the app's lock held: `qt-capture` reads the overlay's rect with
//! Win32 calls, so a read never waits on Tauri's main thread. A failed
//! call's error names the call and the system's error, never OCR text or a
//! pixel.

use ch_app::ScreenSetup;

/// The screen backend for the app, built with the overlay window's handle
/// (an `HWND` as an integer, read at setup on the main thread) so a read
/// can tell a tooltip under the overlay. `None` off Windows: the overlay
/// then never shows and nothing is read.
#[cfg(windows)]
pub fn setup(overlay: Option<isize>) -> Option<ScreenSetup> {
    Some(ScreenSetup::new(Box::new(win::GameScreen::new(overlay))))
}

#[cfg(not(windows))]
pub fn setup(_overlay: Option<isize>) -> Option<ScreenSetup> {
    None
}

#[cfg(windows)]
mod win {
    use ch_app::ch_watch::Reader;
    use ch_app::{
        ClientRect, GameWindow, OcrReadiness, ReadCounts, ScreenBackend, TooltipRead, WatchFailure,
    };
    use qt_capture::WindowsBackend;
    use qt_screen::{BackendError, PixelRect, ScreenBackend as _, WindowHandle};

    use crate::game_window::game_in_front;

    /// `qt-capture`'s backend and the reader that runs over it.
    pub struct GameScreen {
        capture: WindowsBackend,
        reader: Reader,
    }

    impl GameScreen {
        pub fn new(overlay: Option<isize>) -> Self {
            GameScreen {
                capture: WindowsBackend::new(overlay),
                reader: Reader::new(),
            }
        }
    }

    /// A `qt-capture` error as the watch logs it, `error: watch: <call>
    /// failed: <error>`: its own `<call> failed: <error>` split there, else
    /// `during` and the whole message.
    fn failure(during: &str, e: BackendError) -> WatchFailure {
        match e.message.split_once(" failed: ") {
            Some((call, error)) => WatchFailure {
                call: call.to_string(),
                error: error.to_string(),
            },
            None => WatchFailure {
                call: during.to_string(),
                error: e.message,
            },
        }
    }

    impl ScreenBackend for GameScreen {
        fn game_window(&mut self) -> Result<GameWindow, WatchFailure> {
            let Some(handle) = game_in_front() else {
                return Ok(GameWindow::NotInFront);
            };
            let client = self
                .capture
                .client_rect(WindowHandle(handle))
                .map_err(|e| failure("the game's client area", e))?;
            Ok(match client {
                None => GameWindow::Minimized,
                Some(r) => GameWindow::InFront {
                    handle,
                    client: ClientRect {
                        left: r.left,
                        top: r.top,
                        right: r.right,
                        bottom: r.bottom,
                    },
                },
            })
        }

        fn ocr_readiness(&mut self) -> Result<OcrReadiness, WatchFailure> {
            match self.capture.ocr_readiness() {
                Ok(qt_screen::OcrReadiness::Ready { language }) => Ok(OcrReadiness::Ready { language }),
                Ok(qt_screen::OcrReadiness::NoEnglish) => Ok(OcrReadiness::NoEnglish),
                Err(e) => Err(failure("the OCR", e)),
            }
        }

        fn read_tooltips(
            &mut self,
            handle: isize,
            client: ClientRect,
        ) -> Result<TooltipRead, WatchFailure> {
            let area = PixelRect::new(client.left, client.top, client.right, client.bottom);
            self.reader
                .read(&mut self.capture, WindowHandle(handle), area)
                .map_err(|e| failure("the tooltip read", e))
        }

        fn take_counts(&mut self) -> ReadCounts {
            self.reader.take_counts()
        }
    }
}
