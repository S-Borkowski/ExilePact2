//! Boxes as fractions of the game's client rect, boxes in physical screen
//! pixels, the conversion between them, and the 16:9 check.
//!
//! Every position the research measured is a fraction of the client rect at
//! 16:9 (2560x1440), so the trackers work in fractions and only the backend
//! and the reader deal in pixels. Window rects are physical pixels: the
//! overlay's process is per-monitor DPI aware.

/// A box as fractions of the client rect: 0,0 is its top-left corner, 1,1
/// its bottom-right.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

impl Rect {
    pub const fn new(left: f64, top: f64, right: f64, bottom: f64) -> Self {
        Rect {
            left,
            top,
            right,
            bottom,
        }
    }

    pub fn width(&self) -> f64 {
        self.right - self.left
    }

    pub fn height(&self) -> f64 {
        self.bottom - self.top
    }

    /// Whether the two boxes share any area (touching edges do not count).
    pub fn overlaps(&self, other: &Rect) -> bool {
        self.left < other.right
            && other.left < self.right
            && self.top < other.bottom
            && other.top < self.bottom
    }

    /// Whether this box spans any of the columns `left..right`.
    pub fn overlaps_columns(&self, left: f64, right: f64) -> bool {
        self.left < right && left < self.right
    }

    /// Whether `other` lies wholly inside this box.
    pub fn contains(&self, other: &Rect) -> bool {
        other.left >= self.left
            && other.right <= self.right
            && other.top >= self.top
            && other.bottom <= self.bottom
    }

    /// This box in screen pixels, for a client rect `client`. Edges round to
    /// the nearest pixel and stay inside the client rect.
    pub fn to_pixels(&self, client: PixelRect) -> PixelRect {
        let w = client.width() as f64;
        let h = client.height() as f64;
        let x = |f: f64| client.left + (f.clamp(0.0, 1.0) * w).round() as i32;
        let y = |f: f64| client.top + (f.clamp(0.0, 1.0) * h).round() as i32;
        PixelRect {
            left: x(self.left),
            top: y(self.top),
            right: x(self.right),
            bottom: y(self.bottom),
        }
    }
}

/// A box in physical screen pixels; `right` and `bottom` are exclusive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PixelRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl PixelRect {
    pub const fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        PixelRect {
            left,
            top,
            right,
            bottom,
        }
    }

    pub fn width(&self) -> i32 {
        (self.right - self.left).max(0)
    }

    pub fn height(&self) -> i32 {
        (self.bottom - self.top).max(0)
    }

    pub fn is_empty(&self) -> bool {
        self.width() == 0 || self.height() == 0
    }

    /// Whether the two boxes share at least one pixel.
    pub fn intersects(&self, other: &PixelRect) -> bool {
        !self.is_empty()
            && !other.is_empty()
            && self.left < other.right
            && other.left < self.right
            && self.top < other.bottom
            && other.top < self.bottom
    }

    /// `inner`, a box in this box's own pixels (0,0 at its top-left), as
    /// fractions of the client rect `client`. An OCR box comes back in the
    /// captured image's pixels; the region it was captured from is `self`.
    pub fn local_to_fraction(&self, inner: PixelRect, client: PixelRect) -> Rect {
        let w = client.width().max(1) as f64;
        let h = client.height().max(1) as f64;
        let fx = |px: i32| (self.left + px - client.left) as f64 / w;
        let fy = |px: i32| (self.top + px - client.top) as f64 / h;
        Rect::new(
            fx(inner.left),
            fy(inner.top),
            fx(inner.right),
            fy(inner.bottom),
        )
    }
}

/// How far a client rect's aspect ratio may stray from 16:9 before the reader
/// refuses it, as a fraction of 16:9.
pub const ASPECT_TOLERANCE: f64 = 0.01;

/// Whether a client area of `width` x `height` pixels is 16:9 within
/// [`ASPECT_TOLERANCE`]. Positions were measured only at 16:9.
pub fn aspect_is_16_9(width: i32, height: i32) -> bool {
    if width <= 0 || height <= 0 {
        return false;
    }
    let ratio = width as f64 / height as f64;
    (ratio / (16.0 / 9.0) - 1.0).abs() <= ASPECT_TOLERANCE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aspect_accepts_16_9_and_refuses_others() {
        assert!(aspect_is_16_9(2560, 1440));
        assert!(aspect_is_16_9(1920, 1080));
        assert!(aspect_is_16_9(1366, 768)); // 0.4% off
        assert!(!aspect_is_16_9(1920, 1200)); // 16:10
        assert!(!aspect_is_16_9(3440, 1440)); // 21:9
        assert!(!aspect_is_16_9(2560, 1400)); // 2.9% off
        assert!(!aspect_is_16_9(0, 0));
    }

    #[test]
    fn fractions_map_to_pixels_and_back() {
        let client = PixelRect::new(100, 50, 2660, 1490);
        let bar = Rect::new(0.25, 0.0, 0.75, 0.14);
        let px = bar.to_pixels(client);
        assert_eq!(px, PixelRect::new(740, 50, 2020, 252));
        let back = px.local_to_fraction(PixelRect::new(0, 0, px.width(), px.height()), client);
        assert!((back.left - 0.25).abs() < 1e-3 && (back.right - 0.75).abs() < 1e-3);
        assert!(back.top.abs() < 1e-3 && (back.bottom - 0.14).abs() < 1e-3);
    }

    #[test]
    fn pixels_stay_inside_the_client_rect() {
        let client = PixelRect::new(0, 0, 1920, 1080);
        let wild = Rect::new(-0.5, -1.0, 1.5, 2.0);
        assert_eq!(wild.to_pixels(client), client);
    }

    #[test]
    fn overlap_and_containment() {
        let a = Rect::new(0.40, 0.0, 0.60, 0.08);
        assert!(a.overlaps(&Rect::new(0.59, 0.07, 0.7, 0.1)));
        assert!(!a.overlaps(&Rect::new(0.60, 0.0, 0.7, 0.1)));
        assert!(a.contains(&Rect::new(0.45, 0.01, 0.55, 0.05)));
        assert!(!a.contains(&Rect::new(0.35, 0.01, 0.55, 0.05)));
        let p = PixelRect::new(0, 0, 10, 10);
        assert!(p.intersects(&PixelRect::new(9, 9, 20, 20)));
        assert!(!p.intersects(&PixelRect::new(10, 0, 20, 10)));
        assert!(!p.intersects(&PixelRect::new(5, 5, 5, 8)));
    }
}
