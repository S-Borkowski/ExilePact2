//! The locator: the item tooltips in a copy of the game's client, from
//! pixels alone.
//!
//! Its features are measured on the owner's screenshots (2026-10-29: 2560 x
//! 1440 and 1920 x 1080, HUD scale middle, UI text medium; the research
//! note's `## Item tooltip on screen`). Lengths are given for a 1440-pixel
//! client and scaled by the client's height.
//!
//! - **The header.** A tooltip starts with a header band drawn opaque: a
//!   frame line along its top (a bright row, two darker rows, two more
//!   bright rows, each one colour along its length: gold for a rare,
//!   blue-grey for a magic item, grey for a normal one, pale brown for
//!   currency), a dark band holding the name lines, and a second frame line
//!   along its bottom. The distance from the top frame to the bottom one is
//!   about 42 pixels for one name line and about 69 for two
//!   ([`ONE_NAME`], [`TWO_NAMES`]). [`locate`] finds a row with a long run of
//!   bright, even pixels ([`FRAME_LUMA`], [`EVEN`], [`MIN_WIDTH`]), a second
//!   bright row two to four rows below it, the dark band under them
//!   ([`BAND_LUMA`]) and the bottom frame line, and keeps the candidate only
//!   when the header's height is one of the two.
//! - **The body.** Below the header the tooltip darkens what lies behind it
//!   to about a fifth of its brightness (measured against the same scene
//!   without a tooltip), so a row of the body with no text is dark across
//!   its width ([`PANEL_LUMA`]). The body ends at the first row where the
//!   scene shows bright again across most of the width (a bright scene), or
//!   else after the last dark row that is followed by [`BODY_GAP`] rows with
//!   none (a dark scene: the stash's grid looks much like the body, so the
//!   box may run on below the tooltip; the reader keeps only the lines
//!   that cross the box's middle).
//! - **The width.** The frame line runs nearly the whole width; the corner
//!   ornaments reach a little further, a quarter of the header's height.
//! - **The rarity** is the colour of the name lines: the one of
//!   [`RARITY_COLOURS`] most of the header band's bright pixels lie near.
//! - **The window's edge.** The game keeps a tooltip inside the window: in
//!   the shots it lies flush against the client's top or left edge, whole.
//!   A box whose body runs to the bottom of the copy may be cut.

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
    /// The box touches the copy's edge where the tooltip may be cut.
    pub touches_edge: bool,
}

/// The client height the lengths below are given for.
pub const BASE_HEIGHT: f64 = 1440.0;
/// A frame line's pixels are this bright (luma, 0-255) or brighter.
pub const FRAME_LUMA: u32 = 70;
/// The frame's second bright row: this bright, over [`FRAME_SHARE`] of the
/// line.
pub const SECOND_LUMA: u32 = 60;
pub const FRAME_SHARE: f64 = 0.8;
/// Two neighbouring pixels of a frame line differ by at most this in every
/// channel.
pub const EVEN: u32 = 12;
/// Shortest frame line, pixels at 1440: shorter runs are the game's other
/// panels' title plates.
pub const MIN_WIDTH: f64 = 250.0;
/// A run of frame pixels may skip this many pixels (at 1440) where the
/// colour steps.
pub const FRAME_SKIP: f64 = 4.0;
/// The header's dark band, 7 to 11 rows below the top (at 1440): pixels
/// this dark over [`BAND_SHARE`] of the line.
pub const BAND_LUMA: u32 = 50;
pub const BAND_SHARE: f64 = 0.85;
/// The bottom frame line: even pixels this bright over [`BOTTOM_SHARE`] of
/// the line's middle, 25 to 110 rows below the top (at 1440).
pub const BOTTOM_LUMA: u32 = 40;
pub const BOTTOM_SHARE: f64 = 0.85;
/// The distance from the top frame to the bottom one, pixels at 1440: one
/// name line, or two.
pub const ONE_NAME: (f64, f64) = (37.0, 47.0);
pub const TWO_NAMES: (f64, f64) = (63.0, 75.0);
/// The header band ends this far (at 1440) below the bottom frame's first
/// row.
pub const BOTTOM_FRAME: f64 = 9.0;
/// A body pixel with no text on it is this dark or darker.
pub const PANEL_LUMA: u32 = 50;
/// A body row with no text is dark over this share of the line.
pub const PANEL_SHARE: f64 = 0.95;
/// The body ends after its last dark row when this many rows (at 1440)
/// follow with none.
pub const BODY_GAP: f64 = 80.0;
/// The body ends where, over this share of the line, the rows below are at
/// least [`EDGE_RATIO`] times as bright as the rows above (plus
/// [`EDGE_OFFSET`]) and at least [`EDGE_LUMA`]: the scene undarkened.
pub const EDGE_SHARE: f64 = 0.6;
pub const EDGE_RATIO: f64 = 2.5;
pub const EDGE_OFFSET: f64 = 6.0;
pub const EDGE_LUMA: f64 = 55.0;
/// The corner ornaments reach this share of the header's height beyond
/// the frame line.
pub const CORNER: f64 = 0.25;
/// A name pixel is this bright or brighter.
pub const NAME_LUMA: u32 = 110;
/// The name colours (R, G, B), the most common bright pixel of the name
/// lines in the shots. Unique: not in the shots; the colour generally given.
pub const RARITY_COLOURS: [(Rarity, [u32; 3]); 4] = [
    (Rarity::Normal, [200, 200, 200]),
    (Rarity::Magic, [122, 151, 254]),
    (Rarity::Rare, [254, 224, 85]),
    (Rarity::Unique, [175, 96, 37]),
];
/// Farthest (Euclidean, RGB) a name pixel may lie from its rarity's colour
/// to count for it. Currency's pale brown (170, 158, 130) lies over 85 from
/// every one.
pub const RARITY_DISTANCE: f64 = 60.0;

/// Every tooltip in `image`, top to bottom, then left to right.
pub fn locate(image: &Image) -> Vec<Located> {
    let px = Pixels::new(image);
    let scale = px.h as f64 / BASE_HEIGHT;
    let at = |v: f64| ((v * scale).round() as i32).max(1);
    if px.w < at(MIN_WIDTH) || px.h < at(TWO_NAMES.1 + 20.0) {
        return Vec::new();
    }
    let mut found: Vec<Located> = Vec::new();
    for y in 0..px.h - at(TWO_NAMES.1) {
        for (x0, x1) in px.frame_runs(y, at(MIN_WIDTH), at(FRAME_SKIP)) {
            let line = PixelRect::new(x0, y, x1, y + 1);
            if found.iter().any(|f| f.rect.intersects(&line)) {
                continue;
            }
            if let Some(f) = px.tooltip_at(y, x0, x1, scale) {
                found.push(f);
            }
        }
    }
    found.sort_by_key(|f| (f.rect.top, f.rect.left));
    found
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

    fn luma(&self, x: i32, y: i32) -> u32 {
        let p = self.at(x, y);
        (114 * p[0] + 587 * p[1] + 299 * p[2]) / 1000
    }

    /// The pixel at `x` is within [`EVEN`] of the one before it.
    fn even(&self, x: i32, y: i32) -> bool {
        x > 0 && {
            let (a, b) = (self.at(x - 1, y), self.at(x, y));
            (0..3).all(|i| a[i].abs_diff(b[i]) <= EVEN)
        }
    }

    /// Share of the pixels of row `y`, columns `x0..x1`, that pass `test`.
    fn share(&self, y: i32, x0: i32, x1: i32, test: impl Fn(i32, i32) -> bool) -> f64 {
        if x1 <= x0 || y < 0 || y >= self.h {
            return 0.0;
        }
        let hits = (x0..x1).filter(|&x| test(x, y)).count();
        hits as f64 / (x1 - x0) as f64
    }

    /// The runs of bright, even pixels in row `y` at least `min` long, a
    /// skip of up to `skip` pixels allowed inside one.
    fn frame_runs(&self, y: i32, min: i32, skip: i32) -> Vec<(i32, i32)> {
        let mut runs = Vec::new();
        let mut x = 0;
        while x < self.w {
            let frame = |x: i32| self.luma(x, y) >= FRAME_LUMA && self.even(x, y);
            if !frame(x) {
                x += 1;
                continue;
            }
            let start = x;
            let mut end = x + 1;
            x += 1;
            while x < self.w && x - end <= skip {
                if frame(x) {
                    end = x + 1;
                }
                x += 1;
            }
            if end - start >= min {
                runs.push((start, end));
            }
            x = end + 1;
        }
        runs
    }

    /// The tooltip whose top frame line is the run `x0..x1` of row `y`, if
    /// the rest of a header and a body follow.
    fn tooltip_at(&self, y: i32, x0: i32, x1: i32, scale: f64) -> Option<Located> {
        let at = |v: f64| ((v * scale).round() as i32).max(1);
        let second = (y + 2..=y + 4)
            .any(|r| self.share(r, x0, x1, |x, r| self.luma(x, r) >= SECOND_LUMA) >= FRAME_SHARE);
        if !second {
            return None;
        }
        let band = (y + at(7.0)..=y + at(11.0))
            .all(|r| self.share(r, x0, x1, |x, r| self.luma(x, r) <= BAND_LUMA) >= BAND_SHARE);
        if !band {
            return None;
        }
        let (m0, m1) = (x0 + (x1 - x0) / 10, x1 - (x1 - x0) / 10);
        let bottom_frame = (y + at(25.0)..(y + at(110.0)).min(self.h)).find(|&r| {
            self.share(r, m0, m1, |x, r| {
                self.luma(x, r) >= BOTTOM_LUMA && self.even(x, r)
            }) >= BOTTOM_SHARE
        })?;
        let height = (bottom_frame - y) as f64 / scale;
        let fits = |(lo, hi): (f64, f64)| height >= lo && height <= hi;
        if !fits(ONE_NAME) && !fits(TWO_NAMES) {
            return None;
        }
        let header_bottom = (bottom_frame + at(BOTTOM_FRAME)).min(self.h);
        let bottom = self.body_bottom(header_bottom, x0, x1, at(BODY_GAP));
        let corner = (CORNER * (bottom_frame - y) as f64).round() as i32;
        let rect = PixelRect::new((x0 - corner).max(0), y, (x1 + corner).min(self.w), bottom);
        Some(Located {
            rect,
            header_bottom,
            rarity: self.name_rarity(x0, x1, y + at(7.0), bottom_frame),
            touches_edge: bottom >= self.h,
        })
    }

    /// The first row below the body, scanning down from `top` over the
    /// columns `x0..x1`.
    fn body_bottom(&self, top: i32, x0: i32, x1: i32, gap: i32) -> i32 {
        let mean = |x: i32, from: i32, to: i32| {
            (from..to).map(|r| self.luma(x, r) as f64).sum::<f64>() / (to - from) as f64
        };
        let dark = |r: i32| self.share(r, x0, x1, |x, r| self.luma(x, r) <= PANEL_LUMA);
        let mut last_dark = top;
        for r in top + 3..self.h - 3 {
            let edge = self.share(r, x0, x1, |x, r| {
                let above = mean(x, r - 3, r);
                let below = mean(x, r, r + 3);
                above <= PANEL_LUMA as f64
                    && below >= (EDGE_RATIO * above + EDGE_OFFSET).max(EDGE_LUMA)
            });
            if edge >= EDGE_SHARE && dark(r - 1) >= PANEL_SHARE {
                return r;
            }
            if dark(r) >= PANEL_SHARE {
                last_dark = r;
            } else if r - last_dark > gap {
                return last_dark + 1;
            }
        }
        self.h
    }

    /// The rarity most of the bright pixels between the header's frames lie
    /// near, over the middle of the line (the corner ornaments, gold for
    /// every rarity, lie outside it): each pixel counts for the nearest
    /// rarity colour within [`RARITY_DISTANCE`], and a rarity must hold more
    /// than half of all the bright pixels.
    fn name_rarity(&self, x0: i32, x1: i32, top: i32, bottom: i32) -> Option<Rarity> {
        let (m0, m1) = (x0 + (x1 - x0) / 10, x1 - (x1 - x0) / 10);
        let mut votes = [0usize; RARITY_COLOURS.len()];
        let mut bright = 0usize;
        for y in top..bottom {
            for x in m0..m1 {
                if self.luma(x, y) < NAME_LUMA {
                    continue;
                }
                bright += 1;
                let p = self.at(x, y);
                // B, G, R against R, G, B.
                let nearest = RARITY_COLOURS
                    .iter()
                    .enumerate()
                    .map(|(n, (_, c))| {
                        let d = [(p[2], c[0]), (p[1], c[1]), (p[0], c[2])]
                            .iter()
                            .map(|&(a, b)| (a as f64 - b as f64).powi(2))
                            .sum::<f64>()
                            .sqrt();
                        (n, d)
                    })
                    .min_by(|a, b| a.1.total_cmp(&b.1))
                    .filter(|(_, d)| *d <= RARITY_DISTANCE);
                if let Some((n, _)) = nearest {
                    votes[n] += 1;
                }
            }
        }
        let (best, count) = votes.iter().enumerate().max_by_key(|(_, c)| **c)?;
        (bright > 0 && *count * 2 > bright).then(|| RARITY_COLOURS[best].0.clone())
    }
}
