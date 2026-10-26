//! The Windows backend: Win32 for the windows and the copy, WinRT's
//! `Windows.Media.Ocr` for the reading.

use std::ffi::c_void;
use std::thread::{self, ThreadId};

use qt_screen::{
    BackendError, Image, OcrReadiness, OverlayState, PixelLine, PixelRect, ScreenBackend,
    WindowHandle,
};
use windows::core::{BOOL, HSTRING};
use windows::Globalization::Language;
use windows::Graphics::Imaging::{BitmapAlphaMode, BitmapPixelFormat, SoftwareBitmap};
use windows::Media::Ocr::OcrEngine;
use windows::Storage::Streams::DataWriter;
use windows::Win32::Foundation::{HWND, LPARAM, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    BitBlt, ClientToScreen, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GdiFlush,
    GetDC, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HBITMAP,
    HDC, HGDIOBJ, SRCCOPY,
};
use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClientRect, GetForegroundWindow, GetWindowRect, GetWindowThreadProcessId,
    IsIconic, IsWindow, IsWindowVisible,
};

use crate::pixels::{ocr_scale, opaque, resize, scaled_size, union_of_words, unscale_box};

/// The language the OCR reads: the research read every frame in en-US.
pub const OCR_LANGUAGE: &str = "en-US";

/// The OCR engine, made once on first use.
struct Ocr {
    engine: OcrEngine,
    max_dimension: u32,
}

/// [`ScreenBackend`] on Win32 and `Windows.Media.Ocr`.
///
/// Built anywhere and moved to the reader's thread: WinRT is initialized
/// multithreaded on the thread that first asks for OCR (and again on any
/// other thread that later does), and the engine is made there.
pub struct WindowsBackend {
    overlay: Option<isize>,
    ocr: Option<Ocr>,
    no_english: bool,
    apartment: Option<ThreadId>,
}

fn hwnd(h: isize) -> HWND {
    HWND(h as *mut c_void)
}

fn failed(call: &str, e: windows::core::Error) -> BackendError {
    BackendError::new(format!("{call} failed: {e}"))
}

/// Runs its closure when dropped: GDI handles are released on every path.
struct OnDrop<F: FnMut()>(F);

impl<F: FnMut()> Drop for OnDrop<F> {
    fn drop(&mut self) {
        (self.0)()
    }
}

/// The largest visible top-level window of `pid` seen so far.
struct Search {
    pid: u32,
    best: Option<isize>,
    best_area: i64,
}

unsafe extern "system" fn visit(h: HWND, lparam: LPARAM) -> BOOL {
    // SAFETY: `lparam` is the `Search` `game_window` passes, alive for the
    // whole `EnumWindows` call.
    let search = unsafe { &mut *(lparam.0 as *mut Search) };
    let mut owner = 0u32;
    unsafe { GetWindowThreadProcessId(h, Some(&mut owner)) };
    if owner != search.pid || !unsafe { IsWindowVisible(h) }.as_bool() {
        return BOOL(1);
    }
    let mut r = RECT::default();
    if unsafe { GetWindowRect(h, &mut r) }.is_err() {
        return BOOL(1);
    }
    let area = i64::from((r.right - r.left).max(0)) * i64::from((r.bottom - r.top).max(0));
    if area > search.best_area {
        search.best_area = area;
        search.best = Some(h.0 as isize);
    }
    BOOL(1)
}

/// A 32-bit top-down DIB section of `width` x `height` selected into a
/// memory DC, its pixels at `bits`. Dropping it restores and frees both.
struct Canvas {
    dc: HDC,
    bitmap: HBITMAP,
    old: HGDIOBJ,
    bits: *mut c_void,
    width: i32,
    height: i32,
}

impl Canvas {
    fn new(compatible: Option<HDC>, width: i32, height: i32) -> Result<Canvas, BackendError> {
        let dc = unsafe { CreateCompatibleDC(compatible) };
        if dc.is_invalid() {
            return Err(BackendError::new("CreateCompatibleDC failed"));
        }
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits: *mut c_void = std::ptr::null_mut();
        let bitmap = match unsafe {
            CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0)
        } {
            Ok(b) if !bits.is_null() => b,
            Ok(b) => {
                unsafe {
                    let _ = DeleteObject(b.into());
                    let _ = DeleteDC(dc);
                }
                return Err(BackendError::new("CreateDIBSection returned no pixels"));
            }
            Err(e) => {
                unsafe {
                    let _ = DeleteDC(dc);
                }
                return Err(failed("CreateDIBSection", e));
            }
        };
        let old = unsafe { SelectObject(dc, bitmap.into()) };
        Ok(Canvas {
            dc,
            bitmap,
            old,
            bits,
            width,
            height,
        })
    }

    /// The canvas's pixels as an image, made opaque (GDI leaves alpha 0).
    fn image(&self) -> Image {
        unsafe {
            let _ = GdiFlush();
        }
        let len = self.width as usize * self.height as usize * 4;
        // SAFETY: the DIB section holds `width * height` 32-bit pixels at
        // `bits` for as long as `self` lives.
        let pixels = unsafe { std::slice::from_raw_parts(self.bits as *const u8, len) };
        let mut bgra = pixels.to_vec();
        bgra.chunks_exact_mut(4).for_each(|px| px[3] = 255);
        Image {
            width: self.width as u32,
            height: self.height as u32,
            bgra,
        }
    }
}

impl Drop for Canvas {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.dc, self.old);
            let _ = DeleteObject(self.bitmap.into());
            let _ = DeleteDC(self.dc);
        }
    }
}

impl WindowsBackend {
    /// A backend whose overlay is the window `overlay_hwnd` (an `HWND` as an
    /// integer), or none.
    pub fn new(overlay_hwnd: Option<isize>) -> Self {
        WindowsBackend {
            overlay: overlay_hwnd.filter(|&h| h != 0),
            ocr: None,
            no_english: false,
            apartment: None,
        }
    }

    /// WinRT, multithreaded, on the calling thread.
    fn ensure_apartment(&mut self) -> Result<(), BackendError> {
        let me = thread::current().id();
        if self.apartment == Some(me) {
            return Ok(());
        }
        unsafe { RoInitialize(RO_INIT_MULTITHREADED) }
            .map_err(|e| failed("RoInitialize (multithreaded)", e))?;
        self.apartment = Some(me);
        Ok(())
    }

    /// The engine, made on first use; `None` when Windows has no English OCR.
    fn engine(&mut self) -> Result<Option<&Ocr>, BackendError> {
        self.ensure_apartment()?;
        if self.ocr.is_none() && !self.no_english {
            let language = Language::CreateLanguage(&HSTRING::from(OCR_LANGUAGE))
                .map_err(|e| failed("Language::CreateLanguage", e))?;
            let supported = OcrEngine::IsLanguageSupported(&language)
                .map_err(|e| failed("OcrEngine::IsLanguageSupported", e))?;
            if supported {
                let engine = OcrEngine::TryCreateFromLanguage(&language)
                    .map_err(|e| failed("OcrEngine::TryCreateFromLanguage", e))?;
                let max_dimension = OcrEngine::MaxImageDimension()
                    .map_err(|e| failed("OcrEngine::MaxImageDimension", e))?;
                self.ocr = Some(Ocr {
                    engine,
                    max_dimension,
                });
            } else {
                self.no_english = true;
            }
        }
        Ok(self.ocr.as_ref())
    }
}

impl ScreenBackend for WindowsBackend {
    fn game_window(&mut self, pid: u32) -> Result<Option<WindowHandle>, BackendError> {
        if pid == 0 {
            return Ok(None);
        }
        let mut search = Search {
            pid,
            best: None,
            best_area: -1,
        };
        unsafe { EnumWindows(Some(visit), LPARAM(&mut search as *mut Search as isize)) }
            .map_err(|e| failed("EnumWindows", e))?;
        Ok(search.best.map(WindowHandle))
    }

    fn foreground_pid(&mut self) -> Result<Option<u32>, BackendError> {
        let h = unsafe { GetForegroundWindow() };
        if h.is_invalid() {
            return Ok(None);
        }
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(h, Some(&mut pid)) };
        Ok((pid != 0).then_some(pid))
    }

    fn client_rect(&mut self, window: WindowHandle) -> Result<Option<PixelRect>, BackendError> {
        let h = hwnd(window.0);
        if !unsafe { IsWindow(Some(h)) }.as_bool() || unsafe { IsIconic(h) }.as_bool() {
            return Ok(None);
        }
        let mut c = RECT::default();
        unsafe { GetClientRect(h, &mut c) }.map_err(|e| failed("GetClientRect", e))?;
        let mut origin = POINT::default();
        if !unsafe { ClientToScreen(h, &mut origin) }.as_bool() {
            return Err(BackendError::new("ClientToScreen failed"));
        }
        let rect = PixelRect::new(
            origin.x,
            origin.y,
            origin.x + c.right - c.left,
            origin.y + c.bottom - c.top,
        );
        Ok((!rect.is_empty()).then_some(rect))
    }

    fn overlay(&mut self) -> Result<Option<OverlayState>, BackendError> {
        let Some(raw) = self.overlay else {
            return Ok(None);
        };
        let h = hwnd(raw);
        if !unsafe { IsWindow(Some(h)) }.as_bool() {
            return Ok(None);
        }
        let mut r = RECT::default();
        unsafe { GetWindowRect(h, &mut r) }.map_err(|e| failed("GetWindowRect (overlay)", e))?;
        let visible = unsafe { IsWindowVisible(h) }.as_bool() && !unsafe { IsIconic(h) }.as_bool();
        Ok(Some(OverlayState {
            rect: PixelRect::new(r.left, r.top, r.right, r.bottom),
            visible,
        }))
    }

    fn capture(&mut self, window: WindowHandle, area: PixelRect) -> Result<Image, BackendError> {
        let Some(client) = self.client_rect(window)? else {
            return Err(BackendError::new(
                "the game window is gone or minimized; nothing was copied",
            ));
        };
        let area = crate::pixels::clamp_to(area, client);
        if area.is_empty() {
            return Err(BackendError::new(
                "the area to copy lies outside the game's client rect",
            ));
        }
        let screen = unsafe { GetDC(None) };
        if screen.is_invalid() {
            return Err(BackendError::new("GetDC (screen) failed"));
        }
        let _screen = OnDrop(|| unsafe {
            ReleaseDC(None, screen);
        });
        let canvas = Canvas::new(Some(screen), area.width(), area.height())?;
        unsafe {
            BitBlt(
                canvas.dc,
                0,
                0,
                area.width(),
                area.height(),
                Some(screen),
                area.left,
                area.top,
                SRCCOPY,
            )
        }
        .map_err(|e| failed("BitBlt", e))?;
        Ok(canvas.image())
    }

    fn ocr_readiness(&mut self) -> Result<OcrReadiness, BackendError> {
        Ok(match self.engine()? {
            Some(_) => OcrReadiness::Ready {
                language: OCR_LANGUAGE.to_string(),
            },
            None => OcrReadiness::NoEnglish,
        })
    }

    fn ocr(&mut self, image: &Image) -> Result<Vec<PixelLine>, BackendError> {
        let Some(ocr) = self.engine()? else {
            return Err(BackendError::new("Windows has no English OCR"));
        };
        let (width, height) = (image.width, image.height);
        if image.bgra.len() != width as usize * height as usize * 4 {
            return Err(BackendError::new(format!(
                "the image's pixels do not fill {width}x{height}"
            )));
        }
        if width == 0 || height == 0 {
            return Ok(Vec::new());
        }
        let scale = ocr_scale(width, height, ocr.max_dimension);
        let (sw, sh) = scaled_size(width, height, scale);
        let pixels = if (sw, sh) == (width, height) {
            opaque(image)
        } else {
            resize(image, sw, sh).bgra
        };
        let writer = DataWriter::new().map_err(|e| failed("DataWriter::new", e))?;
        writer
            .WriteBytes(&pixels)
            .map_err(|e| failed("DataWriter::WriteBytes", e))?;
        let buffer = writer
            .DetachBuffer()
            .map_err(|e| failed("DataWriter::DetachBuffer", e))?;
        let bitmap = SoftwareBitmap::CreateCopyWithAlphaFromBuffer(
            &buffer,
            BitmapPixelFormat::Bgra8,
            sw as i32,
            sh as i32,
            BitmapAlphaMode::Premultiplied,
        )
        .map_err(|e| failed("SoftwareBitmap::CreateCopyWithAlphaFromBuffer", e))?;
        let result = ocr
            .engine
            .RecognizeAsync(&bitmap)
            .and_then(|op| op.join())
            .map_err(|e| failed("OcrEngine::RecognizeAsync", e));
        let _ = bitmap.Close();
        let result = result?;
        let lines = result.Lines().map_err(|e| failed("OcrResult::Lines", e))?;
        let count = lines
            .Size()
            .map_err(|e| failed("OcrResult::Lines.Size", e))?;
        let mut out = Vec::with_capacity(count as usize);
        for i in 0..count {
            let line = lines
                .GetAt(i)
                .map_err(|e| failed("OcrResult::Lines.GetAt", e))?;
            let words = line.Words().map_err(|e| failed("OcrLine::Words", e))?;
            let n = words.Size().map_err(|e| failed("OcrLine::Words.Size", e))?;
            let mut rects = Vec::with_capacity(n as usize);
            for j in 0..n {
                let r = words
                    .GetAt(j)
                    .and_then(|w| w.BoundingRect())
                    .map_err(|e| failed("OcrWord::BoundingRect", e))?;
                rects.push((r.X as f64, r.Y as f64, r.Width as f64, r.Height as f64));
            }
            let text = line.Text().map_err(|e| failed("OcrLine::Text", e))?;
            out.push(PixelLine::new(
                text.to_string_lossy(),
                unscale_box(union_of_words(&rects), scale, width, height),
            ));
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qt_screen::{fuzzy_score, MATCH_THRESHOLD};
    use windows::core::w;
    use windows::Win32::Foundation::COLORREF;
    use windows::Win32::Graphics::Gdi::{
        CreateFontW, SetBkMode, SetTextColor, TextOutW, ANTIALIASED_QUALITY, CLIP_DEFAULT_PRECIS,
        DEFAULT_CHARSET, OUT_DEFAULT_PRECIS, TRANSPARENT,
    };

    const WIDTH: i32 = 900;
    const ROW: i32 = 90;

    /// `lines` drawn light on dark (as the game draws its labels), one per
    /// row of [`ROW`] pixels, into an in-memory bitmap. Nothing touches the
    /// screen.
    fn render(lines: &[&str]) -> Image {
        let canvas = Canvas::new(None, WIDTH, ROW * 2).expect("canvas");
        let len = (WIDTH * ROW * 2 * 4) as usize;
        // SAFETY: the canvas's DIB holds exactly `len` bytes.
        let pixels = unsafe { std::slice::from_raw_parts_mut(canvas.bits as *mut u8, len) };
        pixels
            .chunks_exact_mut(4)
            .for_each(|px| px.copy_from_slice(&[24, 22, 20, 0]));
        unsafe {
            let font = CreateFontW(
                -40,
                0,
                0,
                0,
                400,
                0,
                0,
                0,
                DEFAULT_CHARSET,
                OUT_DEFAULT_PRECIS,
                CLIP_DEFAULT_PRECIS,
                ANTIALIASED_QUALITY,
                0,
                w!("Arial"),
            );
            assert!(!font.is_invalid(), "CreateFontW");
            let old = SelectObject(canvas.dc, font.into());
            SetBkMode(canvas.dc, TRANSPARENT);
            SetTextColor(canvas.dc, COLORREF(0x00E0_E8F0));
            for (i, line) in lines.iter().enumerate() {
                let text: Vec<u16> = line.encode_utf16().collect();
                assert!(
                    TextOutW(canvas.dc, 40, 20 + ROW * i as i32, &text).as_bool(),
                    "TextOutW"
                );
            }
            SelectObject(canvas.dc, old);
            let _ = DeleteObject(font.into());
        }
        canvas.image()
    }

    /// The best score of `name` over `lines`, and that line's index.
    fn best(name: &str, lines: &[PixelLine]) -> (f64, usize) {
        lines
            .iter()
            .enumerate()
            .map(|(i, l)| (fuzzy_score(name, &l.text), i))
            .fold((0.0, usize::MAX), |a, b| if b.0 > a.0 { b } else { a })
    }

    #[test]
    fn the_backend_is_send_and_needs_no_overlay() {
        fn send<T: Send>(_: &T) {}
        let mut backend = WindowsBackend::new(None);
        send(&backend);
        assert_eq!(backend.overlay(), Ok(None));
        assert_eq!(WindowsBackend::new(Some(0)).overlay(), Ok(None));
        assert_eq!(backend.game_window(0), Ok(None));
    }

    #[test]
    #[ignore = "reads with Windows' en-US OCR; the item check runs it with --include-ignored"]
    fn ocr_selftest_reads_rendered_names() {
        let names = ["Beira of the Rotten Pack", "QUEST COMPLETE"];
        // Its own thread, as the reader's: WinRT starts multithreaded there.
        thread::spawn(move || {
            let mut backend = WindowsBackend::new(None);
            assert_eq!(
                backend.ocr_readiness(),
                Ok(OcrReadiness::Ready {
                    language: OCR_LANGUAGE.to_string()
                }),
                "this machine needs Windows' en-US OCR"
            );

            let image = render(&names);
            let lines = backend.ocr(&image).expect("OCR of the rendered names");
            for (row, name) in names.iter().enumerate() {
                let (score, i) = best(name, &lines);
                assert!(
                    score >= MATCH_THRESHOLD,
                    "{name}: best score {score:.3} over {} lines",
                    lines.len()
                );
                // Its box is in the image's own pixels, on the row drawn.
                let b = lines[i].bbox;
                assert!(
                    b.left >= 0
                        && b.right <= image.width as i32
                        && b.bottom <= image.height as i32
                        && !b.is_empty(),
                    "{name}: {b:?}"
                );
                let middle = (b.top + b.bottom) / 2;
                assert!(
                    middle >= ROW * row as i32 && middle < ROW * (row as i32 + 1),
                    "{name}: {b:?} is not on row {row}"
                );
            }

            // Negative control: the same canvas with nothing drawn reads
            // neither name.
            let blank = backend.ocr(&render(&[])).expect("OCR of a blank image");
            for name in names {
                let (score, _) = best(name, &blank);
                assert!(
                    score < MATCH_THRESHOLD,
                    "{name} read off a blank: {score:.3}"
                );
            }
        })
        .join()
        .expect("the selftest thread");
    }
}
