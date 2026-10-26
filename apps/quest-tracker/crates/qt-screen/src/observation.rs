//! What one read of one region gave the trackers: the OCR lines with their
//! boxes as fractions of the client rect, or the reason the region was not
//! visible.
//!
//! OCR text is private: it lives only in memory and never reaches a file, a
//! log line, the UI or an error. So the `Debug` forms here print a line's
//! length, never its text, and nothing in this crate formats one.

use std::fmt;

use crate::geometry::{PixelRect, Rect};
use crate::region::Region;
use crate::Millis;

/// One OCR line, its box as fractions of the client rect.
#[derive(Clone, PartialEq)]
pub struct Line {
    pub text: String,
    pub bbox: Rect,
}

impl Line {
    pub fn new(text: impl Into<String>, bbox: Rect) -> Self {
        Line {
            text: text.into(),
            bbox,
        }
    }
}

impl fmt::Debug for Line {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Line")
            .field(
                "text",
                &format_args!("<{} chars>", self.text.chars().count()),
            )
            .field("bbox", &self.bbox)
            .finish()
    }
}

/// One OCR line as the backend returns it: its box in the captured image's
/// own pixels, 0,0 at the image's top-left, at the capture's scale (an OCR
/// that upscales maps its boxes back first).
#[derive(Clone, PartialEq)]
pub struct PixelLine {
    pub text: String,
    pub bbox: PixelRect,
}

impl PixelLine {
    pub fn new(text: impl Into<String>, bbox: PixelRect) -> Self {
        PixelLine {
            text: text.into(),
            bbox,
        }
    }
}

impl fmt::Debug for PixelLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PixelLine")
            .field(
                "text",
                &format_args!("<{} chars>", self.text.chars().count()),
            )
            .field("bbox", &self.bbox)
            .finish()
    }
}

/// Why a region was not visible this read. Every one counts the frame as
/// covered: it says nothing about what the game drew there.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Hidden {
    /// The overlay's own window lies over the region.
    Overlay,
    /// Another window is in front of the game.
    NotForeground,
    /// The game's window is minimized.
    Minimized,
    /// The copy failed.
    CaptureFailed,
    /// The copy came back black ([`Image::is_black`]).
    Black,
    /// The OCR call failed.
    OcrFailed,
}

/// Whether the region could be read, and if not, why.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Visibility {
    Visible,
    Hidden(Hidden),
}

/// One read of one region at `at`, on the caller's clock.
#[derive(Clone, Debug, PartialEq)]
pub struct Observation {
    pub region: Region,
    pub at: Millis,
    pub visibility: Visibility,
    /// Empty unless visible.
    pub lines: Vec<Line>,
}

impl Observation {
    /// A visible read with these lines.
    pub fn visible(region: Region, at: Millis, lines: Vec<Line>) -> Self {
        Observation {
            region,
            at,
            visibility: Visibility::Visible,
            lines,
        }
    }

    /// A read that saw nothing, for `why`.
    pub fn hidden(region: Region, at: Millis, why: Hidden) -> Self {
        Observation {
            region,
            at,
            visibility: Visibility::Hidden(why),
            lines: Vec::new(),
        }
    }

    /// A visible read from the backend's OCR of the image copied from `area`
    /// (screen pixels) of the client rect `client`.
    pub fn from_ocr(
        region: Region,
        at: Millis,
        client: PixelRect,
        area: PixelRect,
        lines: Vec<PixelLine>,
    ) -> Self {
        let lines = lines
            .into_iter()
            .map(|l| Line {
                bbox: area.local_to_fraction(l.bbox, client),
                text: l.text,
            })
            .collect();
        Observation::visible(region, at, lines)
    }

    pub fn is_visible(&self) -> bool {
        self.visibility == Visibility::Visible
    }
}

/// A copy whose mean luma (0-255) is below this, and whose spread is below
/// [`BLACK_LUMA_STD`], is black: the capture got nothing back.
pub const BLACK_LUMA_MEAN: f64 = 8.0;
/// The spread half of the black-copy rule; see [`BLACK_LUMA_MEAN`].
pub const BLACK_LUMA_STD: f64 = 3.0;

/// A copied region: `width` x `height` pixels, top-down rows, 4 bytes a
/// pixel in B, G, R, A order (a GDI 32-bit DIB).
#[derive(Clone, PartialEq, Eq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub bgra: Vec<u8>,
}

impl fmt::Debug for Image {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Image({}x{})", self.width, self.height)
    }
}

impl Image {
    /// Mean and standard deviation of luma (0-255) over every fourth pixel of
    /// every fourth row, as the research's capture computed them.
    pub fn luma_stats(&self) -> (f64, f64) {
        let (w, h) = (self.width as usize, self.height as usize);
        if self.bgra.len() < w * h * 4 {
            return (0.0, 0.0);
        }
        let (mut sum, mut sum_sq, mut n) = (0.0_f64, 0.0_f64, 0u64);
        for y in (0..h).step_by(4) {
            for x in (0..w).step_by(4) {
                let i = (y * w + x) * 4;
                let px = &self.bgra[i..i + 3];
                let l = 0.114 * px[0] as f64 + 0.587 * px[1] as f64 + 0.299 * px[2] as f64;
                sum += l;
                sum_sq += l * l;
                n += 1;
            }
        }
        if n == 0 {
            return (0.0, 0.0);
        }
        let mean = sum / n as f64;
        (mean, (sum_sq / n as f64 - mean * mean).max(0.0).sqrt())
    }

    /// Very dark and nearly uniform: a copy that got nothing back.
    pub fn is_black(&self) -> bool {
        let (mean, std) = self.luma_stats();
        mean < BLACK_LUMA_MEAN && std < BLACK_LUMA_STD
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_never_prints_ocr_text() {
        let line = Line::new("SENTINEL-TEXT", Rect::new(0.1, 0.2, 0.3, 0.4));
        let obs = Observation::visible(Region::VoicedLines, 5, vec![line]);
        let px = PixelLine::new("SENTINEL-TEXT", PixelRect::new(0, 0, 4, 4));
        let shown = format!("{obs:?} {px:?}");
        assert!(!shown.contains("SENTINEL"), "{shown}");
        assert!(shown.contains("<13 chars>"));
    }

    #[test]
    fn ocr_boxes_become_client_fractions() {
        let client = PixelRect::new(0, 0, 2560, 1440);
        let area = Region::BossBar.pixels(client);
        // The Miller's name box, in the band's own pixels.
        let line = PixelLine::new("THE BLOATED MILLER", PixelRect::new(527, 68, 753, 86));
        let obs = Observation::from_ocr(Region::BossBar, 0, client, area, vec![line]);
        let b = obs.lines[0].bbox;
        assert!((b.left - 0.4559).abs() < 1e-3, "{b:?}");
        assert!((b.right - 0.5442).abs() < 1e-3, "{b:?}");
        assert!((b.top - 0.0472).abs() < 1e-3, "{b:?}");
        assert!((b.bottom - 0.0597).abs() < 1e-3, "{b:?}");
        assert!(obs.is_visible());
    }

    #[test]
    fn black_is_dark_and_flat() {
        let image = |px: [u8; 4]| Image {
            width: 8,
            height: 8,
            bgra: px.repeat(64),
        };
        assert!(image([0, 0, 0, 255]).is_black());
        assert!(image([5, 5, 5, 255]).is_black());
        assert!(!image([40, 40, 40, 255]).is_black());
        let mut speckled = image([0, 0, 0, 255]);
        speckled.bgra[0..3].copy_from_slice(&[255, 255, 255]);
        assert!(!speckled.is_black());
        let empty = Image {
            width: 0,
            height: 0,
            bgra: Vec::new(),
        };
        assert!(empty.is_black());
    }
}
