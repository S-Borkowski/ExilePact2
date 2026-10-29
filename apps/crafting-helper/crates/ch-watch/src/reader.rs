//! The reader: one copy of the game's client, the tooltips found in it, and
//! their lines, over calls it is given.
//!
//! A tooltip the overlay's window overlaps is reported as covered and never
//! read in part, since a missing line would read as a lost modifier. A crop
//! identical to one the last read OCR'd is not OCR'd again: its lines are
//! reused. Copies, crops and lines stay in memory; the last read's crops and
//! lines are kept for that comparison alone, and dropped by a read that
//! finds no tooltip.

use std::fmt;

use qt_screen::{
    BackendError, Image, OverlayState, PixelLine, PixelRect, ScreenBackend, WindowHandle,
};

use crate::locator::{locate, Located};
use crate::{TooltipBox, TooltipRead};

/// The system calls a read needs. Every [`ScreenBackend`] (`qt-capture`'s
/// Windows backend among them) gives them; tests fake them.
pub trait ReadCalls {
    /// A copy of `area` (screen pixels) of the window.
    fn capture(&mut self, window: WindowHandle, area: PixelRect) -> Result<Image, BackendError>;
    /// The overlay's own window: where it is and whether it is shown.
    fn overlay(&mut self) -> Result<Option<OverlayState>, BackendError>;
    /// The lines OCR reads in `image`, their boxes in its own pixels.
    fn ocr(&mut self, image: &Image) -> Result<Vec<PixelLine>, BackendError>;
}

impl<B: ScreenBackend + ?Sized> ReadCalls for B {
    fn capture(&mut self, window: WindowHandle, area: PixelRect) -> Result<Image, BackendError> {
        ScreenBackend::capture(self, window, area)
    }

    fn overlay(&mut self) -> Result<Option<OverlayState>, BackendError> {
        ScreenBackend::overlay(self)
    }

    fn ocr(&mut self, image: &Image) -> Result<Vec<PixelLine>, BackendError> {
        ScreenBackend::ocr(self, image)
    }
}

/// What the reader did since its counts were last taken, for the watch's
/// minute line.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReadCounts {
    /// Copies of the client.
    pub copies: u64,
    /// Tooltips found in them.
    pub tooltips: u64,
    /// OCR calls made.
    pub ocr_reads: u64,
}

/// Reads the item tooltips in the game's client, keeping the last read's
/// crops and lines to skip an OCR of an unchanged crop.
#[derive(Default)]
pub struct Reader {
    last: Vec<(Image, Vec<PixelLine>)>,
    counts: ReadCounts,
}

impl fmt::Debug for Reader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Reader")
            .field("kept crops", &self.last.len())
            .field("counts", &self.counts)
            .finish()
    }
}

impl Reader {
    pub fn new() -> Self {
        Reader::default()
    }

    /// The counts since the last call, which resets them.
    pub fn take_counts(&mut self) -> ReadCounts {
        std::mem::take(&mut self.counts)
    }

    /// Copy the client area `client` (screen pixels) of `window`, find the
    /// tooltips in it and read them.
    pub fn read<C: ReadCalls + ?Sized>(
        &mut self,
        calls: &mut C,
        window: WindowHandle,
        client: PixelRect,
    ) -> Result<TooltipRead, BackendError> {
        let image = calls.capture(window, client)?;
        self.counts.copies += 1;
        if image.is_black() {
            self.last.clear();
            return Ok(TooltipRead::Black);
        }
        let found = locate(&image);
        if found.is_empty() {
            self.last.clear();
            return Ok(TooltipRead::NoTooltip);
        }
        self.counts.tooltips += found.len() as u64;

        if let Some(overlay) = calls.overlay()? {
            let on_screen = |r: PixelRect| {
                PixelRect::new(
                    r.left + client.left,
                    r.top + client.top,
                    r.right + client.left,
                    r.bottom + client.top,
                )
            };
            if overlay.visible
                && found
                    .iter()
                    .any(|f| overlay.rect.intersects(&on_screen(f.rect)))
            {
                return Ok(TooltipRead::Covered);
            }
        }

        let mut kept = Vec::with_capacity(found.len());
        let mut boxes = Vec::with_capacity(found.len());
        for f in found {
            let crop = crop(&image, f.rect);
            let lines = match self.last.iter().find(|(seen, _)| *seen == crop) {
                Some((_, lines)) => lines.clone(),
                None => {
                    self.counts.ocr_reads += 1;
                    calls.ocr(&crop)?
                }
            };
            boxes.push(tooltip_box(&f, &lines));
            kept.push((crop, lines));
        }
        self.last = kept;
        Ok(TooltipRead::Boxes(boxes))
    }
}

/// The `rect` part of `image`.
fn crop(image: &Image, rect: PixelRect) -> Image {
    let (w, h) = (rect.width() as u32, rect.height() as u32);
    let mut bgra = Vec::with_capacity((w * h * 4) as usize);
    let stride = image.width as usize * 4;
    for y in rect.top..rect.bottom {
        let start = y as usize * stride + rect.left as usize * 4;
        bgra.extend_from_slice(&image.bgra[start..start + w as usize * 4]);
    }
    Image {
        width: w,
        height: h,
        bgra,
    }
}

/// A located box and its crop's lines: the lines moved to client pixels and
/// split at the header band's bottom into name lines and the rest.
fn tooltip_box(found: &Located, lines: &[PixelLine]) -> TooltipBox {
    let header = found.header_bottom - found.rect.top;
    let mut name_lines = Vec::new();
    let mut rest = Vec::new();
    for l in lines {
        let moved = PixelLine::new(
            l.text.clone(),
            PixelRect::new(
                l.bbox.left + found.rect.left,
                l.bbox.top + found.rect.top,
                l.bbox.right + found.rect.left,
                l.bbox.bottom + found.rect.top,
            ),
        );
        if (l.bbox.top + l.bbox.bottom) / 2 < header {
            name_lines.push(moved);
        } else {
            rest.push(moved);
        }
    }
    TooltipBox {
        rect: found.rect,
        name_lines,
        lines: rest,
        rarity: found.rarity.clone(),
        touches_edge: found.touches_edge,
    }
}
