//! Shared test support: tooltip lines built from the engine's ring item
//! texts (the plain form and the Alt form), and synthetic screen images (a
//! noisy scene with a tooltip-like panel drawn on it), with a fake set of
//! read calls that counts them.
//!
//! The tooltip construction is an assumption until the `locator` item reads
//! the owner's screenshots: the plain form drops the `{ … }` headers, the
//! `Item Class:` and `Rarity:` lines, the separators and every `(min-max)`
//! range; the Alt form keeps the headers (without braces) and the ranges.

#![allow(dead_code)]

use ch_engine::ch_item::{self, Item, Rarity};
use ch_watch::{ReadCalls, TooltipBox};
use qt_screen::{BackendError, Image, OverlayState, PixelLine, PixelRect, WindowHandle};

const RINGS: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../ch-engine/tests/items/gear/rings-resist-whittle/"
);

/// One of the whittled ring's texts (`01-start` … `08-whittle-lightning`).
pub fn ring_text(name: &str) -> String {
    let path = format!("{RINGS}{name}.txt");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// The ring text parsed.
pub fn ring(name: &str) -> Item {
    parse(&ring_text(name))
}

pub fn parse(text: &str) -> Item {
    ch_item::parse(text).expect("item text")
}

/// The tooltip's lines for an item text: its name lines and the lines below.
#[derive(Clone, Debug)]
pub struct Tip {
    pub rarity: Rarity,
    pub name_lines: Vec<String>,
    pub lines: Vec<String>,
}

impl Tip {
    /// As the reader would report it, with the name lines' colour.
    pub fn boxed(&self) -> TooltipBox {
        TooltipBox::from_text(&self.name_lines, &self.lines, Some(self.rarity.clone()))
    }

    /// Replace every line equal to `from` with `to`.
    pub fn replace(mut self, from: &str, to: &str) -> Tip {
        let mut found = false;
        for l in self.lines.iter_mut().chain(self.name_lines.iter_mut()) {
            if l == from {
                *l = to.to_string();
                found = true;
            }
        }
        assert!(found, "no line {from:?}");
        self
    }
}

/// The plain tooltip (`alt` false) or the Alt-held one of an item text.
pub fn tooltip(text: &str, alt: bool) -> Tip {
    let mut rarity = None;
    let mut name_lines = Vec::new();
    let mut lines = Vec::new();
    let mut in_names = false;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with("Item Class:") {
            continue;
        }
        if let Some(r) = line.strip_prefix("Rarity:") {
            rarity = Some(Rarity::from_text(r.trim()));
            in_names = true;
            continue;
        }
        if line == "--------" {
            in_names = false;
            continue;
        }
        if in_names {
            name_lines.push(line.to_string());
            continue;
        }
        if line.starts_with('{') {
            if alt {
                let inner = line.trim_start_matches('{').trim_end_matches('}').trim();
                lines.push(inner.to_string());
            }
            continue;
        }
        lines.push(if alt {
            line.to_string()
        } else {
            strip_ranges(line)
        });
    }
    Tip {
        rarity: rarity.expect("a Rarity: line"),
        name_lines,
        lines,
    }
}

/// The line with every `(min-max)` range removed.
pub fn strip_ranges(line: &str) -> String {
    let mut out = String::new();
    let mut rest = line;
    while let Some(open) = rest.find('(') {
        let Some(close) = rest[open..].find(')') else {
            break;
        };
        let inner = &rest[open + 1..open + close];
        let is_range = inner.contains('-')
            && inner
                .chars()
                .all(|c| c.is_ascii_digit() || c == '-' || c == '.');
        out.push_str(&rest[..open]);
        if !is_range {
            out.push_str(&rest[open..=open + close]);
        }
        rest = &rest[open + close + 1..];
    }
    out.push_str(rest);
    out
}

// ---------------------------------------------------------------------------
// Synthetic images.

/// A BGR colour.
pub type Bgr = [u8; 3];

/// The panel, its border and the header band, as generally described of the
/// game's tooltips (UNVERIFIED: the `locator` item measures the real ones).
pub const PANEL: Bgr = [14, 14, 16];
pub const BORDER: Bgr = [40, 70, 92];
pub const HEADER: Bgr = [20, 45, 60];
/// Modifier text: light blue.
pub const MOD_TEXT: Bgr = [255, 136, 136];

/// The name colour of a rarity, BGR.
pub fn name_colour(rarity: &Rarity) -> Bgr {
    match rarity {
        Rarity::Normal => [200, 200, 200],
        Rarity::Magic => [255, 136, 136],
        Rarity::Rare => [119, 255, 255],
        Rarity::Unique => [37, 96, 175],
        Rarity::Other(_) => [128, 128, 128],
    }
}

/// A `w` x `h` image of random pixels (a busy scene), from `seed`.
pub fn noisy_scene(w: u32, h: u32, seed: u64) -> Image {
    let mut state = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
    let mut bgra = Vec::with_capacity((w * h * 4) as usize);
    for _ in 0..w * h {
        for _ in 0..3 {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            bgra.push((state >> 56) as u8);
        }
        bgra.push(255);
    }
    Image {
        width: w,
        height: h,
        bgra,
    }
}

/// A `w` x `h` image of one colour.
pub fn plain_image(w: u32, h: u32, bgr: Bgr) -> Image {
    let mut image = Image {
        width: w,
        height: h,
        bgra: vec![0; (w * h * 4) as usize],
    };
    fill(&mut image, PixelRect::new(0, 0, w as i32, h as i32), bgr);
    image
}

/// Paint `rect` (clipped to the image) with `bgr`.
pub fn fill(image: &mut Image, rect: PixelRect, bgr: Bgr) {
    let (w, h) = (image.width as i32, image.height as i32);
    for y in rect.top.max(0)..rect.bottom.min(h) {
        for x in rect.left.max(0)..rect.right.min(w) {
            let i = ((y * w + x) * 4) as usize;
            image.bgra[i..i + 3].copy_from_slice(&bgr);
            image.bgra[i + 3] = 255;
        }
    }
}

/// A row of glyph-like blocks, centred in `left..right`, `width` wide.
fn text_row(image: &mut Image, left: i32, right: i32, top: i32, width: i32, bgr: Bgr) {
    let start = left + (right - left - width) / 2;
    let mut x = start;
    while x + 6 <= start + width {
        fill(image, PixelRect::new(x, top, x + 6, top + 10), bgr);
        x += 9;
    }
}

/// The header band's height for a rarity: one name line, or two.
pub fn header_height(rarity: &Rarity) -> i32 {
    match rarity {
        Rarity::Rare | Rarity::Unique => 48,
        _ => 30,
    }
}

/// Draw a tooltip-like panel at `rect`: a dark panel with a 2 px border, a
/// tinted header band with the name lines in the rarity's colour, and
/// `body_rows` rows of light text below. `wide` varies the text widths so two
/// tooltips differ.
pub fn draw_tooltip(
    image: &mut Image,
    rect: PixelRect,
    rarity: &Rarity,
    body_rows: i32,
    wide: i32,
) {
    fill(image, rect, BORDER);
    let inner = PixelRect::new(rect.left + 2, rect.top + 2, rect.right - 2, rect.bottom - 2);
    fill(image, inner, PANEL);
    let header = PixelRect::new(
        inner.left,
        inner.top,
        inner.right,
        inner.top + header_height(rarity),
    );
    fill(image, header, HEADER);
    let names = if header_height(rarity) > 30 { 2 } else { 1 };
    let text_w = (inner.right - inner.left) / 2;
    for n in 0..names {
        text_row(
            image,
            inner.left,
            inner.right,
            header.top + 8 + n * 18,
            text_w - n * 20,
            name_colour(rarity),
        );
    }
    let mut y = header.bottom + 12;
    for r in 0..body_rows {
        if y + 10 > inner.bottom - 6 {
            break;
        }
        text_row(
            image,
            inner.left,
            inner.right,
            y,
            text_w - 18 * (r % 3) + wide,
            MOD_TEXT,
        );
        y += 22;
    }
}

// ---------------------------------------------------------------------------
// The fake read calls.

/// The calls the reader is given, faked: one image, the overlay's state, and
/// the lines OCR "reads", each call counted.
pub struct FakeCalls {
    pub image: Image,
    pub overlay: Option<OverlayState>,
    pub ocr_lines: Vec<PixelLine>,
    pub captures: usize,
    pub overlay_calls: usize,
    pub ocr_calls: usize,
    /// The area each capture asked for.
    pub areas: Vec<PixelRect>,
}

impl FakeCalls {
    pub fn new(image: Image) -> Self {
        FakeCalls {
            image,
            overlay: None,
            ocr_lines: Vec::new(),
            captures: 0,
            overlay_calls: 0,
            ocr_calls: 0,
            areas: Vec::new(),
        }
    }
}

impl ReadCalls for FakeCalls {
    fn capture(&mut self, _window: WindowHandle, area: PixelRect) -> Result<Image, BackendError> {
        self.captures += 1;
        self.areas.push(area);
        Ok(self.image.clone())
    }

    fn overlay(&mut self) -> Result<Option<OverlayState>, BackendError> {
        self.overlay_calls += 1;
        Ok(self.overlay)
    }

    fn ocr(&mut self, _image: &Image) -> Result<Vec<PixelLine>, BackendError> {
        self.ocr_calls += 1;
        Ok(self.ocr_lines.clone())
    }
}
