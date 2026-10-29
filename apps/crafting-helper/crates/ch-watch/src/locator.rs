//! The provisional locator: the item tooltips in a copy of the game's
//! client, from pixels alone.
//!
//! PROVISIONAL. Its features are what is generally said of the game's
//! tooltips, proven only on synthetic images: a near-opaque dark panel, a
//! header band at its top drawn in colour with the name lines in the
//! rarity's colour, and light text below. Every constant here is UNVERIFIED
//! until the `locator` item measures the owner's screenshots and replaces
//! them; [`locate`] and [`Located`] stay as they are.
//!
//! 1. The copy is cut into [`CELL`]-pixel cells; a cell is panel when at
//!    least [`CELL_PANEL_SHARE`] of its pixels are dark ([`PANEL_LUMA_MAX`]).
//! 2. Each 4-connected group of panel cells at least [`MIN_WIDTH`] by
//!    [`MIN_HEIGHT`] pixels that fills [`MIN_FILL`] of its bounding box is a
//!    candidate, its edges moved to the pixel column or row where the panel
//!    starts.
//! 3. A candidate is a tooltip when its top rows form a header band: at
//!    least [`MIN_HEADER`] rows in a row of which [`HEADER_SHARE`] of the
//!    pixels are coloured or light. The rarity is the name text's colour in
//!    that band, the nearest of [`RARITY_COLOURS`].

use ch_engine::ch_item::Rarity;
use qt_screen::{Image, PixelRect};

/// One tooltip found in a copy, in the copy's own pixels.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Located {
    pub rect: PixelRect,
    /// The first row below the header band (the name lines lie above it).
    pub header_bottom: i32,
    /// The rarity the name lines' colour says, when it is near one.
    pub rarity: Option<Rarity>,
    /// The box touches the copy's edge: it may be cut.
    pub touches_edge: bool,
}

/// Cell size, pixels.
pub const CELL: i32 = 8;
/// A pixel this dark (luma, 0-255) or darker is panel.
pub const PANEL_LUMA_MAX: u32 = 80;
/// Share of a cell's pixels that must be panel for the cell to be.
pub const CELL_PANEL_SHARE: f64 = 0.5;
/// Smallest tooltip, pixels.
pub const MIN_WIDTH: i32 = 120;
pub const MIN_HEIGHT: i32 = 80;
/// Share of a candidate's bounding box its panel cells must fill.
pub const MIN_FILL: f64 = 0.6;
/// Share of a column or row that must be panel for the box's edge to lie
/// there.
pub const EDGE_SHARE: f64 = 0.6;
/// A header pixel is coloured (its channels this far apart) or light.
pub const HEADER_CHROMA: u32 = 20;
pub const TEXT_LUMA: u32 = 90;
/// Share of a row that must be header pixels for the row to be header band.
pub const HEADER_SHARE: f64 = 0.5;
/// Fewest header rows.
pub const MIN_HEADER: i32 = 12;
/// The name colours (R, G, B) as generally given for the game's rarities.
pub const RARITY_COLOURS: [(Rarity, [u32; 3]); 4] = [
    (Rarity::Normal, [200, 200, 200]),
    (Rarity::Magic, [136, 136, 255]),
    (Rarity::Rare, [255, 255, 119]),
    (Rarity::Unique, [175, 96, 37]),
];
/// Farthest (Euclidean, RGB) the name colour may lie from its rarity's.
pub const RARITY_DISTANCE: f64 = 70.0;

/// Every tooltip in `image`, top to bottom, then left to right.
pub fn locate(image: &Image) -> Vec<Located> {
    let px = Pixels::new(image);
    if px.w < MIN_WIDTH || px.h < MIN_HEIGHT {
        return Vec::new();
    }
    let cols = (px.w + CELL - 1) / CELL;
    let rows = (px.h + CELL - 1) / CELL;
    let mut panel = vec![false; (cols * rows) as usize];
    for cy in 0..rows {
        for cx in 0..cols {
            let cell = PixelRect::new(
                cx * CELL,
                cy * CELL,
                ((cx + 1) * CELL).min(px.w),
                ((cy + 1) * CELL).min(px.h),
            );
            panel[(cy * cols + cx) as usize] = px.panel_share(cell) >= CELL_PANEL_SHARE;
        }
    }

    let mut seen = vec![false; panel.len()];
    let mut found = Vec::new();
    for start in 0..panel.len() {
        if !panel[start] || seen[start] {
            continue;
        }
        let (count, bounds) = flood(&panel, &mut seen, cols, start);
        let (x0, y0, x1, y1) = bounds;
        let cells = ((x1 - x0 + 1) * (y1 - y0 + 1)) as f64;
        let rough = PixelRect::new(
            x0 * CELL,
            y0 * CELL,
            ((x1 + 1) * CELL).min(px.w),
            ((y1 + 1) * CELL).min(px.h),
        );
        if rough.width() < MIN_WIDTH
            || rough.height() < MIN_HEIGHT
            || (count as f64) < MIN_FILL * cells
        {
            continue;
        }
        let rect = px.refine(rough);
        if rect.width() < MIN_WIDTH || rect.height() < MIN_HEIGHT {
            continue;
        }
        let header_bottom = px.header_bottom(rect);
        let header = header_bottom - rect.top;
        if header < MIN_HEADER || header > rect.height() * 3 / 5 {
            continue;
        }
        found.push(Located {
            rect,
            header_bottom,
            rarity: px.name_rarity(rect, header_bottom),
            touches_edge: rect.left <= 0
                || rect.top <= 0
                || rect.right >= px.w
                || rect.bottom >= px.h,
        });
    }
    found.sort_by_key(|f| (f.rect.top, f.rect.left));
    found
}

/// One group of 4-connected panel cells from `start`: its size and its
/// bounds in cells (inclusive).
fn flood(
    panel: &[bool],
    seen: &mut [bool],
    cols: i32,
    start: usize,
) -> (usize, (i32, i32, i32, i32)) {
    let rows = panel.len() as i32 / cols;
    let mut stack = vec![start];
    seen[start] = true;
    let mut count = 0;
    let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
    while let Some(at) = stack.pop() {
        count += 1;
        let (x, y) = (at as i32 % cols, at as i32 / cols);
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x);
        y1 = y1.max(y);
        for (nx, ny) in [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)] {
            if nx < 0 || ny < 0 || nx >= cols || ny >= rows {
                continue;
            }
            let n = (ny * cols + nx) as usize;
            if panel[n] && !seen[n] {
                seen[n] = true;
                stack.push(n);
            }
        }
    }
    (count, (x0, y0, x1, y1))
}

struct Pixels<'a> {
    image: &'a Image,
    w: i32,
    h: i32,
}

impl<'a> Pixels<'a> {
    fn new(image: &'a Image) -> Self {
        let complete = image.bgra.len() >= (image.width as usize) * (image.height as usize) * 4;
        let (w, h) = if complete {
            (image.width as i32, image.height as i32)
        } else {
            (0, 0)
        };
        Pixels { image, w, h }
    }

    /// B, G, R at `x`, `y`.
    fn at(&self, x: i32, y: i32) -> [u32; 3] {
        let i = ((y * self.w + x) * 4) as usize;
        let p = &self.image.bgra[i..i + 3];
        [p[0] as u32, p[1] as u32, p[2] as u32]
    }

    fn luma(p: [u32; 3]) -> u32 {
        (114 * p[0] + 587 * p[1] + 299 * p[2]) / 1000
    }

    fn is_panel(&self, x: i32, y: i32) -> bool {
        Self::luma(self.at(x, y)) <= PANEL_LUMA_MAX
    }

    fn is_header(&self, x: i32, y: i32) -> bool {
        let p = self.at(x, y);
        let chroma = p.iter().max().unwrap() - p.iter().min().unwrap();
        chroma >= HEADER_CHROMA || Self::luma(p) >= TEXT_LUMA
    }

    fn share(&self, area: PixelRect, test: impl Fn(i32, i32) -> bool) -> f64 {
        if area.is_empty() {
            return 0.0;
        }
        let mut hits = 0usize;
        for y in area.top..area.bottom {
            for x in area.left..area.right {
                if test(x, y) {
                    hits += 1;
                }
            }
        }
        hits as f64 / (area.width() * area.height()) as f64
    }

    fn panel_share(&self, area: PixelRect) -> f64 {
        self.share(area, |x, y| self.is_panel(x, y))
    }

    /// The rough box (whole cells) with each edge moved to the first column
    /// or row, within a cell of it, that is mostly panel.
    fn refine(&self, rough: PixelRect) -> PixelRect {
        let inner_rows = (rough.top + CELL).min(rough.bottom)..(rough.bottom - CELL).max(rough.top);
        let inner_cols = (rough.left + CELL).min(rough.right)..(rough.right - CELL).max(rough.left);
        let col = |x: i32| {
            self.panel_share(PixelRect::new(x, inner_rows.start, x + 1, inner_rows.end))
                >= EDGE_SHARE
        };
        let row = |y: i32| {
            self.panel_share(PixelRect::new(inner_cols.start, y, inner_cols.end, y + 1))
                >= EDGE_SHARE
        };
        let left = ((rough.left - CELL).max(0)..rough.left + CELL)
            .find(|&x| col(x))
            .unwrap_or(rough.left);
        let right = ((rough.right - CELL)..(rough.right + CELL).min(self.w))
            .rev()
            .find(|&x| col(x))
            .map_or(rough.right, |x| x + 1);
        let top = ((rough.top - CELL).max(0)..rough.top + CELL)
            .find(|&y| row(y))
            .unwrap_or(rough.top);
        let bottom = ((rough.bottom - CELL)..(rough.bottom + CELL).min(self.h))
            .rev()
            .find(|&y| row(y))
            .map_or(rough.bottom, |y| y + 1);
        PixelRect::new(left, top, right, bottom)
    }

    /// The first row below the header band that starts at the box's top.
    fn header_bottom(&self, rect: PixelRect) -> i32 {
        let mut y = rect.top;
        while y < rect.bottom
            && self.share(PixelRect::new(rect.left, y, rect.right, y + 1), |x, y| {
                self.is_header(x, y)
            }) >= HEADER_SHARE
        {
            y += 1;
        }
        y
    }

    /// The rarity whose colour lies nearest the light pixels of the header
    /// band (inside the border), if one lies near enough.
    fn name_rarity(&self, rect: PixelRect, header_bottom: i32) -> Option<Rarity> {
        let (mut sum, mut n) = ([0u64; 3], 0u64);
        for y in (rect.top + 2)..header_bottom {
            for x in (rect.left + 2)..(rect.right - 2) {
                let p = self.at(x, y);
                if Self::luma(p) >= TEXT_LUMA {
                    // R, G, B
                    sum[0] += p[2] as u64;
                    sum[1] += p[1] as u64;
                    sum[2] += p[0] as u64;
                    n += 1;
                }
            }
        }
        if n == 0 {
            return None;
        }
        let mean = sum.map(|s| s as f64 / n as f64);
        RARITY_COLOURS
            .iter()
            .map(|(r, c)| {
                let d = (0..3)
                    .map(|i| (mean[i] - c[i] as f64).powi(2))
                    .sum::<f64>()
                    .sqrt();
                (r, d)
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .filter(|(_, d)| *d <= RARITY_DISTANCE)
            .map(|(r, _)| r.clone())
    }
}
