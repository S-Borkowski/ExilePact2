//! "Hide under game panels": the table of game panels the overlay hides
//! under, the probe test that tells one is open from a few small copies of
//! the game's window, and the per-entry tracker that turns reads into open
//! and closed.
//!
//! - A [`PanelEntry`] is one panel frame: its id, the panel ids whose windows
//!   it stands for (`opens_for`: the stash's and a merchant's windows may
//!   share the inventory's frame), its footprint (the rect the panel changes,
//!   dimmed areas included) and its [`PanelProbe`]s, of which at least
//!   `min_match` must match for the panel to read open.
//! - A [`PanelProbe`] is a small rect with an expected mean blue, green and
//!   red, a tolerance per channel and a largest luma spread. It matches a
//!   copy's [`ProbeStats`] when each channel's mean is within its tolerance
//!   and the spread is at most the largest.
//! - Readable probes: while the overlay is visible, a probe that touches its
//!   rect is not read (the copy would hold the overlay). While it is hidden,
//!   every probe is read.
//! - Only entries whose footprint overlaps the overlay's rect are judged
//!   ([`PanelTable::under_overlay`]); the others are not copied at all.
//! - Each read of an entry is a [`PanelRead`]; a [`PanelTracker`] per entry
//!   opens after [`PANEL_OPEN_READS`] Open reads in a row and closes after
//!   [`PANEL_CLOSED_READS`] Closed reads in a row. Unseen and Black reads
//!   change nothing.
//! - Each read is decided from its [`PanelCounts`] (probes read, in colour,
//!   within spread, matching), which the panel watcher and the replay also
//!   report, so a count and a verdict never disagree.
//!
//! All geometry is fractions of the game's client rect, as for [`Region`].
//! Colour read at run time is private, like OCR text: [`ProbeStats`]' `Debug`
//! form prints none of its numbers, and nothing here formats one.
//!
//! [`Region`]: crate::Region

use std::fmt;

use crate::geometry::{PixelRect, Rect};
use crate::observation::{Image, BLACK_LUMA_MEAN, BLACK_LUMA_STD};
use crate::Millis;

/// Open reads in a row before an entry is open: at once, so the overlay gets
/// out of the way as soon as a panel shows.
pub const PANEL_OPEN_READS: u32 = 1;

/// Closed reads in a row before an open entry is closed: one stray read (a
/// spell's flash over a probe) does not bring the overlay back over a panel.
pub const PANEL_CLOSED_READS: u32 = 2;

/// The panel watcher reads at most this often (4 Hz).
pub const PANEL_TICK_MS: Millis = 250;

/// The default overlay's reach at zooms up to 1.5: the window sits at the
/// monitor's right edge, its top at 0.30 of the height. A shipped entry keeps
/// at least `min_match` probes outside it, so it is seen beside the overlay.
pub const DEFAULT_OVERLAY_REACH: Rect = Rect::new(0.78, 0.30, 1.0, 0.97);

/// The owner's eight panels, by the ids the table's `opens_for` and the
/// replay's windows file use: the inventory (I), the passive tree (P), the
/// panels on M, Escape, O and U, the stash and a merchant's trade window.
pub const PANEL_IDS: [&str; 8] = [
    "inventory",
    "passive-tree",
    "key-m",
    "escape-menu",
    "key-o",
    "key-u",
    "stash",
    "merchant",
];

/// What a copy of one probe's rect held: its mean blue, green and red
/// (0-255), its luma's standard deviation, and whether the copy was black
/// (the capture got nothing back).
///
/// These are colours read from the game at run time: the `Debug` form prints
/// none of them, so a stray `{:?}` cannot put one in a log line.
#[derive(Clone, Copy, PartialEq)]
pub struct ProbeStats {
    pub mean_bgr: [f64; 3],
    pub luma_std: f64,
    pub black: bool,
}

impl fmt::Debug for ProbeStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ProbeStats(<private>, black: {})", self.black)
    }
}

impl ProbeStats {
    /// The statistics of a whole copy, over every pixel. `black` is
    /// [`Image::is_black`]'s answer for the copy.
    pub fn of_image(image: &Image) -> ProbeStats {
        let all = PixelRect::new(0, 0, image.width as i32, image.height as i32);
        let (mean_bgr, _, luma_std) = window_stats(image, all);
        ProbeStats {
            mean_bgr,
            luma_std,
            black: image.is_black(),
        }
    }

    /// The statistics of `rect` (fractions) within `frame`, a copy of the
    /// whole client rect, as a replay reads a kept frame. `black` applies
    /// the black-copy rule ([`BLACK_LUMA_MEAN`], [`BLACK_LUMA_STD`]) to that
    /// rect's pixels.
    pub fn of_frame(frame: &Image, rect: Rect) -> ProbeStats {
        let client = PixelRect::new(0, 0, frame.width as i32, frame.height as i32);
        let area = rect.to_pixels(client);
        let (mean_bgr, luma_mean, luma_std) = window_stats(frame, area);
        ProbeStats {
            mean_bgr,
            luma_std,
            black: luma_mean < BLACK_LUMA_MEAN && luma_std < BLACK_LUMA_STD,
        }
    }
}

/// Mean blue, green and red, mean luma and luma's standard deviation over
/// every pixel of `area` (the image's own pixels). An empty area, or an
/// image shorter than its size says, reads as all zero.
fn window_stats(image: &Image, area: PixelRect) -> ([f64; 3], f64, f64) {
    let (w, h) = (image.width as usize, image.height as usize);
    if image.bgra.len() < w * h * 4 {
        return ([0.0; 3], 0.0, 0.0);
    }
    let x0 = area.left.clamp(0, w as i32) as usize;
    let x1 = area.right.clamp(0, w as i32) as usize;
    let y0 = area.top.clamp(0, h as i32) as usize;
    let y1 = area.bottom.clamp(0, h as i32) as usize;
    let (mut sum, mut luma, mut luma_sq, mut n) = ([0.0_f64; 3], 0.0_f64, 0.0_f64, 0u64);
    for y in y0..y1 {
        for x in x0..x1 {
            let i = (y * w + x) * 4;
            let px = &image.bgra[i..i + 3];
            let (b, g, r) = (px[0] as f64, px[1] as f64, px[2] as f64);
            sum[0] += b;
            sum[1] += g;
            sum[2] += r;
            let l = 0.114 * b + 0.587 * g + 0.299 * r;
            luma += l;
            luma_sq += l * l;
            n += 1;
        }
    }
    if n == 0 {
        return ([0.0; 3], 0.0, 0.0);
    }
    let n = n as f64;
    let mean = luma / n;
    let std = (luma_sq / n - mean * mean).max(0.0).sqrt();
    ([sum[0] / n, sum[1] / n, sum[2] / n], mean, std)
}

/// Whether two boxes share any point, touching edges included: a probe that
/// only touches the overlay's rect may still hold its border pixels.
fn touches(a: &Rect, b: &Rect) -> bool {
    a.left <= b.right && b.left <= a.right && a.top <= b.bottom && b.top <= a.bottom
}

/// One small patch of a panel's frame and the colour it has while the panel
/// is open.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PanelProbe {
    /// Where it sits, as fractions of the client rect.
    pub rect: Rect,
    /// The expected mean blue, green and red (0-255).
    pub bgr: [f64; 3],
    /// How far each channel's mean may stray from `bgr`.
    pub tolerance: [f64; 3],
    /// The largest luma standard deviation a matching copy may have.
    pub max_luma_std: f64,
}

impl PanelProbe {
    /// Where it sits in screen pixels, for the client rect `client`.
    pub fn pixels(&self, client: PixelRect) -> PixelRect {
        self.rect.to_pixels(client)
    }

    /// Whether a copy's colour is this probe's: each channel's mean within
    /// its tolerance of `bgr`.
    pub fn in_colour(&self, stats: &ProbeStats) -> bool {
        (0..3).all(|c| (stats.mean_bgr[c] - self.bgr[c]).abs() <= self.tolerance[c])
    }

    /// Whether a copy's luma spread is at most `max_luma_std`.
    pub fn within_spread(&self, stats: &ProbeStats) -> bool {
        stats.luma_std <= self.max_luma_std
    }

    /// Whether a copy with these statistics matches: in colour and within
    /// the spread.
    pub fn matches(&self, stats: &ProbeStats) -> bool {
        self.in_colour(stats) && self.within_spread(stats)
    }

    /// Whether it may be read with the overlay at `overlay` (fractions):
    /// while the overlay is visible, a probe touching its rect is not.
    pub fn is_readable(&self, overlay: &Rect, overlay_visible: bool) -> bool {
        !overlay_visible || !touches(&self.rect, overlay)
    }
}

/// What one read says about one entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PanelRead {
    /// At least `min_match` probes readable, and at least `min_match` of
    /// them match.
    Open,
    /// At least `min_match` probes readable, fewer match.
    Closed,
    /// Fewer than `min_match` probes readable: nothing to judge by.
    Unseen,
    /// Every readable probe's copy was black: the capture got nothing back.
    Black,
}

/// The counts behind one read of one entry: of its probes, those *read*
/// (readable and copied), those *in colour* (each channel's mean within its
/// tolerance), those *within spread* (luma spread at most `max_luma_std`),
/// those that *match* (both), and those whose copy was black. Counts only:
/// they say why an entry read as it did without holding a colour.
/// [`PanelEntry::classify`] decides from these, so a count and a verdict
/// never disagree.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PanelCounts {
    pub read: usize,
    pub in_colour: usize,
    pub within_spread: usize,
    pub matched: usize,
    pub black: usize,
}

/// One panel frame of the table.
#[derive(Clone, Debug, PartialEq)]
pub struct PanelEntry {
    /// Its name in diagnostic lines (`panels: <id> open`).
    pub id: &'static str,
    /// The panel ids (of [`PANEL_IDS`]) whose windows this frame stands for.
    pub opens_for: Vec<&'static str>,
    /// The rect the panel changes, dimmed areas included, as fractions.
    pub footprint: Rect,
    pub probes: Vec<PanelProbe>,
    /// Probes that must be readable, and match, for the entry to read open.
    pub min_match: usize,
}

impl PanelEntry {
    /// Whether its footprint overlaps the overlay's rect (fractions).
    pub fn is_under(&self, overlay: &Rect) -> bool {
        self.footprint.overlaps(overlay)
    }

    /// The indices of the probes that may be read with the overlay at
    /// `overlay`, in table order.
    pub fn readable_probes(&self, overlay: &Rect, overlay_visible: bool) -> Vec<usize> {
        self.probes
            .iter()
            .enumerate()
            .filter(|(_, p)| p.is_readable(overlay, overlay_visible))
            .map(|(i, _)| i)
            .collect()
    }

    /// The counts behind one read. `copies[i]` is the copy of probe `i`, or
    /// `None` when it was not copied (not readable, or its copy failed); a
    /// missing or unreadable probe is not read.
    pub fn counts(
        &self,
        overlay: &Rect,
        overlay_visible: bool,
        copies: &[Option<ProbeStats>],
    ) -> PanelCounts {
        let mut n = PanelCounts::default();
        for (p, c) in self
            .probes
            .iter()
            .zip(copies.iter())
            .filter(|(p, _)| p.is_readable(overlay, overlay_visible))
        {
            let Some(c) = c else { continue };
            let (colour, spread) = (p.in_colour(c), p.within_spread(c));
            n.read += 1;
            n.in_colour += colour as usize;
            n.within_spread += spread as usize;
            n.matched += (colour && spread) as usize;
            n.black += c.black as usize;
        }
        n
    }

    /// What a read with these counts says: fewer than `min_match` read is
    /// Unseen; else every copy black is Black (the capture got nothing
    /// back); else Open or Closed by the number that match. A black copy
    /// among others is no reason to hold the state: it simply does not
    /// match, as a dark scene behind a closed panel would not.
    pub fn verdict(&self, counts: &PanelCounts) -> PanelRead {
        let need = self.min_match.max(1);
        if counts.read < need {
            PanelRead::Unseen
        } else if counts.black == counts.read {
            PanelRead::Black
        } else if counts.matched >= need {
            PanelRead::Open
        } else {
            PanelRead::Closed
        }
    }

    /// Judge one read: the [`verdict`](Self::verdict) of its
    /// [`counts`](Self::counts).
    pub fn classify(
        &self,
        overlay: &Rect,
        overlay_visible: bool,
        copies: &[Option<ProbeStats>],
    ) -> PanelRead {
        self.verdict(&self.counts(overlay, overlay_visible, copies))
    }

    /// The raw test on a copy of the whole client rect, with no overlay and
    /// no tracker: what `quest-tracker-replay panels detect` reports per
    /// frame. A probe smaller than a pixel of the frame is not readable.
    pub fn classify_frame(&self, frame: &Image) -> PanelRead {
        let client = PixelRect::new(0, 0, frame.width as i32, frame.height as i32);
        let copies: Vec<Option<ProbeStats>> = self
            .probes
            .iter()
            .map(|p| (!p.pixels(client).is_empty()).then(|| ProbeStats::of_frame(frame, p.rect)))
            .collect();
        let nowhere = Rect::new(0.0, 0.0, 0.0, 0.0);
        self.classify(&nowhere, false, &copies)
    }
}

/// The panels the overlay hides under, in the order the watcher prefers
/// them (the first open entry under the overlay names the hide).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PanelTable {
    pub entries: Vec<PanelEntry>,
}

impl PanelTable {
    pub fn new(entries: Vec<PanelEntry>) -> Self {
        PanelTable { entries }
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The entries whose footprint overlaps the overlay's rect, with their
    /// indices, in table order. Only these are copied and judged.
    pub fn under_overlay<'a>(
        &'a self,
        overlay: &'a Rect,
    ) -> impl Iterator<Item = (usize, &'a PanelEntry)> + 'a {
        self.entries
            .iter()
            .enumerate()
            .filter(move |(_, e)| e.is_under(overlay))
    }
}

/// An entry's state changing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PanelChange {
    Opened,
    Closed,
}

/// One entry's state over a sequence of reads.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PanelTracker {
    open: bool,
    open_run: u32,
    closed_run: u32,
}

impl PanelTracker {
    /// Closed, with no reads.
    pub fn new() -> Self {
        PanelTracker::default()
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Feed one read; the change it made, if any. Open after
    /// [`PANEL_OPEN_READS`] Open reads in a row, closed after
    /// [`PANEL_CLOSED_READS`] Closed reads in a row; Unseen and Black reads
    /// leave everything, runs included, as it was.
    pub fn feed(&mut self, read: PanelRead) -> Option<PanelChange> {
        match read {
            PanelRead::Open => {
                self.closed_run = 0;
                self.open_run = self.open_run.saturating_add(1);
                if !self.open && self.open_run >= PANEL_OPEN_READS {
                    self.open = true;
                    return Some(PanelChange::Opened);
                }
            }
            PanelRead::Closed => {
                self.open_run = 0;
                self.closed_run = self.closed_run.saturating_add(1);
                if self.open && self.closed_run >= PANEL_CLOSED_READS {
                    self.open = false;
                    return Some(PanelChange::Closed);
                }
            }
            PanelRead::Unseen | PanelRead::Black => {}
        }
        None
    }

    /// Back to closed with no reads, as when the watcher pauses or idles.
    pub fn reset(&mut self) {
        *self = PanelTracker::default();
    }
}

/// The overlay's window `overlay` (screen pixels) as fractions of the client
/// rect `client`, clamped to 0..1.
pub fn overlay_fraction(client: PixelRect, overlay: PixelRect) -> Rect {
    let w = client.width().max(1) as f64;
    let h = client.height().max(1) as f64;
    let fx = |px: i32| ((px - client.left) as f64 / w).clamp(0.0, 1.0);
    let fy = |px: i32| ((px - client.top) as f64 / h).clamp(0.0, 1.0);
    Rect::new(
        fx(overlay.left),
        fy(overlay.top),
        fx(overlay.right),
        fy(overlay.bottom),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOLD: [f64; 3] = [40.0, 110.0, 150.0];

    fn flat(bgr: [u8; 3], w: u32, h: u32) -> Image {
        Image {
            width: w,
            height: h,
            bgra: [bgr[0], bgr[1], bgr[2], 255].repeat((w * h) as usize),
        }
    }

    fn probe(rect: Rect) -> PanelProbe {
        PanelProbe {
            rect,
            bgr: GOLD,
            tolerance: [12.0, 12.0, 12.0],
            max_luma_std: 10.0,
        }
    }

    /// An entry on the left half with three probes, two of them needed.
    fn left_panel() -> PanelEntry {
        PanelEntry {
            id: "left",
            opens_for: vec!["inventory", "stash"],
            footprint: Rect::new(0.0, 0.0, 0.5, 1.0),
            probes: vec![
                probe(Rect::new(0.02, 0.10, 0.03, 0.12)),
                probe(Rect::new(0.45, 0.10, 0.46, 0.12)),
                probe(Rect::new(0.20, 0.90, 0.21, 0.92)),
            ],
            min_match: 2,
        }
    }

    fn stats(bgr: [u8; 3]) -> Option<ProbeStats> {
        Some(ProbeStats::of_image(&flat(bgr, 16, 16)))
    }

    const NO_OVERLAY: Rect = Rect::new(0.0, 0.0, 0.0, 0.0);

    #[test]
    fn panel_probe_matches_its_colour_within_tolerance() {
        let p = probe(Rect::new(0.1, 0.1, 0.11, 0.12));
        assert!(p.matches(&stats([40, 110, 150]).unwrap()));
        // Each channel at its tolerance's edge, one way and the other.
        assert!(p.matches(&stats([52, 98, 162]).unwrap()));
        assert!(p.matches(&stats([28, 122, 138]).unwrap()));
        // One channel past it is enough to miss.
        assert!(!p.matches(&stats([53, 110, 150]).unwrap()));
        assert!(!p.matches(&stats([40, 97, 150]).unwrap()));
        assert!(!p.matches(&stats([40, 110, 163]).unwrap()));

        // The right mean with too much spread (a checkerboard averaging to
        // the colour) does not match.
        let mut board = flat([0, 0, 0], 16, 16);
        for (i, px) in board.bgra.chunks_mut(4).enumerate() {
            let c: [u8; 3] = if (i + i / 16) % 2 == 0 {
                [0, 60, 100]
            } else {
                [80, 160, 200]
            };
            px[..3].copy_from_slice(&c);
        }
        let s = ProbeStats::of_image(&board);
        assert!((s.mean_bgr[0] - 40.0).abs() < 1e-9 && (s.mean_bgr[2] - 150.0).abs() < 1e-9);
        assert!(s.luma_std > p.max_luma_std, "{}", s.luma_std);
        assert!(!p.matches(&s));

        // The same rule over a probe's rect in a whole frame.
        let mut frame = flat([10, 60, 20], 100, 50);
        for y in 10..20 {
            for x in 30..40 {
                let i = (y * 100 + x) * 4;
                frame.bgra[i..i + 3].copy_from_slice(&[40, 110, 150]);
            }
        }
        let inside = ProbeStats::of_frame(&frame, Rect::new(0.31, 0.22, 0.38, 0.38));
        let outside = ProbeStats::of_frame(&frame, Rect::new(0.60, 0.22, 0.68, 0.38));
        assert!(p.matches(&inside) && !p.matches(&outside));
        // Debug never prints a colour read at run time.
        let shown = format!("{inside:?}");
        assert!(!shown.contains("110") && !shown.contains("150"), "{shown}");
    }

    #[test]
    fn panel_tracker_opens_at_once_and_closes_after_the_closed_reads() {
        assert_eq!(PANEL_OPEN_READS, 1);
        assert_eq!(PANEL_CLOSED_READS, 2);
        let mut t = PanelTracker::new();
        assert!(!t.is_open());
        assert_eq!(t.feed(PanelRead::Closed), None);
        // The first open read opens it.
        assert_eq!(t.feed(PanelRead::Open), Some(PanelChange::Opened));
        assert!(t.is_open());
        assert_eq!(t.feed(PanelRead::Open), None);
        // One stray closed read, then open again: the run starts over.
        assert_eq!(t.feed(PanelRead::Closed), None);
        assert_eq!(t.feed(PanelRead::Open), None);
        for _ in 1..PANEL_CLOSED_READS {
            assert_eq!(t.feed(PanelRead::Closed), None);
            assert!(t.is_open());
        }
        assert_eq!(t.feed(PanelRead::Closed), Some(PanelChange::Closed));
        assert!(!t.is_open());
        assert_eq!(t.feed(PanelRead::Closed), None);
        // Unseen between two closed reads neither breaks the run nor counts.
        t.feed(PanelRead::Open);
        assert_eq!(t.feed(PanelRead::Closed), None);
        assert_eq!(t.feed(PanelRead::Unseen), None);
        assert_eq!(t.feed(PanelRead::Closed), Some(PanelChange::Closed));
        // Reset forgets an open state.
        t.feed(PanelRead::Open);
        t.reset();
        assert!(!t.is_open());
        assert_eq!(t, PanelTracker::new());
    }

    #[test]
    fn panel_probes_under_the_visible_overlay_are_not_read() {
        let entry = left_panel();
        let open = [
            stats([40, 110, 150]),
            stats([40, 110, 150]),
            stats([40, 110, 150]),
        ];
        // The overlay over the second probe only (touching it counts).
        let over_one = Rect::new(0.46, 0.05, 0.60, 0.50);
        assert_eq!(entry.readable_probes(&over_one, true), vec![0, 2]);
        assert_eq!(entry.classify(&over_one, true, &open), PanelRead::Open);
        // The probe under the overlay holds the overlay, not the panel: it is
        // not read, so it neither matches nor misses.
        let mut through = open;
        through[2] = stats([200, 200, 200]);
        assert_eq!(entry.classify(&over_one, true, &through), PanelRead::Closed);
        through = open;
        through[1] = stats([200, 200, 200]);
        assert_eq!(entry.classify(&over_one, true, &through), PanelRead::Open);

        // Over two probes: fewer than `min_match` readable is Unseen, even
        // with copies handed in for them.
        let over_two = Rect::new(0.0, 0.0, 0.5, 0.5);
        assert_eq!(entry.readable_probes(&over_two, true), vec![2]);
        assert_eq!(entry.classify(&over_two, true, &open), PanelRead::Unseen);
        // Hidden, every probe is read.
        assert_eq!(entry.readable_probes(&over_two, false), vec![0, 1, 2]);
        assert_eq!(entry.classify(&over_two, false, &open), PanelRead::Open);
        // A probe that was not copied (or whose copy failed) is not readable.
        let missing = [stats([40, 110, 150]), None, None];
        assert_eq!(
            entry.classify(&NO_OVERLAY, false, &missing),
            PanelRead::Unseen
        );
    }

    /// Two colours in alternate pixels whose mean is `GOLD`: in colour, but
    /// with a luma spread of 30, above the probes' 10.
    fn striped() -> Option<ProbeStats> {
        let mut image = flat([0, 0, 0], 16, 16);
        for (i, px) in image.bgra.chunks_mut(4).enumerate() {
            let c: [u8; 3] = if i % 2 == 0 {
                [10, 80, 120]
            } else {
                [70, 140, 180]
            };
            px[..3].copy_from_slice(&c);
        }
        Some(ProbeStats::of_image(&image))
    }

    #[test]
    fn panel_entry_counts_probes_by_colour_and_spread() {
        let entry = left_panel();
        let p = &entry.probes[0];
        let (gold, off, stripes) = (stats([40, 110, 150]), stats([90, 90, 90]), striped());
        // The three kinds of copy, one probe at a time.
        let s = stripes.unwrap();
        assert!(p.in_colour(&s) && !p.within_spread(&s) && !p.matches(&s));
        let o = off.unwrap();
        assert!(!p.in_colour(&o) && p.within_spread(&o) && !p.matches(&o));
        let g = gold.unwrap();
        assert!(p.in_colour(&g) && p.within_spread(&g) && p.matches(&g));

        let counts = |read, in_colour, within_spread, matched, black| PanelCounts {
            read,
            in_colour,
            within_spread,
            matched,
            black,
        };
        let check = |overlay: &Rect, visible: bool, copies: &[Option<ProbeStats>]| {
            let n = entry.counts(overlay, visible, copies);
            // `classify` decides from the same counts.
            assert_eq!(entry.classify(overlay, visible, copies), entry.verdict(&n));
            (n, entry.verdict(&n))
        };

        // In colour above the spread, within the spread off colour, and one
        // matching both: one match of the two needed.
        assert_eq!(
            check(&NO_OVERLAY, false, &[stripes, off, gold]),
            (counts(3, 2, 2, 1, 0), PanelRead::Closed)
        );
        // Two matching, the third in colour only: open.
        assert_eq!(
            check(&NO_OVERLAY, false, &[gold, stripes, gold]),
            (counts(3, 3, 2, 2, 0), PanelRead::Open)
        );
        // The probe under the visible overlay is not read, so its matching
        // copy counts nowhere; hidden, it is read and the entry opens.
        let over_one = Rect::new(0.46, 0.05, 0.60, 0.50);
        assert_eq!(
            check(&over_one, true, &[stripes, gold, gold]),
            (counts(2, 2, 1, 1, 0), PanelRead::Closed)
        );
        assert_eq!(
            check(&over_one, false, &[stripes, gold, gold]),
            (counts(3, 3, 2, 2, 0), PanelRead::Open)
        );
        // Fewer read than needed: Unseen, whatever the one read holds.
        let over_two = Rect::new(0.0, 0.0, 0.5, 0.5);
        assert_eq!(
            check(&over_two, true, &[gold, gold, gold]),
            (counts(1, 1, 1, 1, 0), PanelRead::Unseen)
        );
        assert_eq!(
            check(&NO_OVERLAY, false, &[gold, None, None]),
            (counts(1, 1, 1, 1, 0), PanelRead::Unseen)
        );
        // A black copy is counted like any other: among matching ones it
        // simply does not match; only an all-black read is Black.
        let black = stats([0, 0, 0]);
        let (n, read) = check(&NO_OVERLAY, false, &[black, gold, gold]);
        assert_eq!(
            (n.read, n.matched, n.black, read),
            (3, 2, 1, PanelRead::Open)
        );
        let (n, read) = check(&NO_OVERLAY, false, &[black, black, black]);
        assert_eq!(
            (n.read, n.matched, n.black, read),
            (3, 0, 3, PanelRead::Black)
        );
        // No copies at all: every count zero.
        assert_eq!(
            check(&NO_OVERLAY, false, &[]),
            (PanelCounts::default(), PanelRead::Unseen)
        );
    }

    #[test]
    fn panel_footprint_overlap_selects_the_panels_under_the_overlay() {
        let screen = PanelEntry {
            id: "whole",
            opens_for: vec!["escape-menu"],
            footprint: Rect::new(0.0, 0.0, 1.0, 1.0),
            probes: vec![probe(Rect::new(0.4, 0.2, 0.41, 0.22))],
            min_match: 1,
        };
        let right = PanelEntry {
            id: "right",
            opens_for: vec!["passive-tree"],
            footprint: Rect::new(0.55, 0.0, 1.0, 1.0),
            probes: vec![probe(Rect::new(0.6, 0.2, 0.61, 0.22))],
            min_match: 1,
        };
        let table = PanelTable::new(vec![left_panel(), screen, right.clone()]);
        let ids = |overlay: Rect| -> Vec<&str> {
            table.under_overlay(&overlay).map(|(_, e)| e.id).collect()
        };
        // The default place: the whole-screen and right-hand panels.
        assert_eq!(ids(DEFAULT_OVERLAY_REACH), vec!["whole", "right"]);
        let indices: Vec<usize> = table
            .under_overlay(&DEFAULT_OVERLAY_REACH)
            .map(|(i, _)| i)
            .collect();
        assert_eq!(indices, vec![1, 2]);
        // Dragged to the left edge.
        assert_eq!(ids(Rect::new(0.0, 0.3, 0.2, 0.9)), vec!["left", "whole"]);
        // Touching a footprint's edge is not under it.
        assert_eq!(ids(Rect::new(0.5, 0.3, 0.55, 0.9)), vec!["whole"]);
        assert!(PanelTable::default()
            .under_overlay(&DEFAULT_OVERLAY_REACH)
            .next()
            .is_none());

        // The overlay's window in pixels becomes fractions, clamped.
        let client = PixelRect::new(100, 0, 2660, 1440);
        let place = overlay_fraction(client, PixelRect::new(2100, 432, 2700, 1400));
        assert!((place.left - 0.78125).abs() < 1e-9, "{place:?}");
        assert!(
            (place.top - 0.30).abs() < 1e-9 && place.right == 1.0,
            "{place:?}"
        );
        assert!(right.is_under(&place) && !left_panel().is_under(&place));
    }

    #[test]
    fn panel_black_copy_leaves_the_state_alone() {
        let entry = left_panel();
        let black = stats([0, 0, 0]);
        assert!(black.unwrap().black);
        let open = [
            stats([40, 110, 150]),
            stats([40, 110, 150]),
            stats([40, 110, 150]),
        ];
        // One black copy beside two matching ones: open at `min_match` 2.
        let mut blacked = open;
        blacked[0] = black;
        assert_eq!(
            entry.classify(&NO_OVERLAY, false, &blacked),
            PanelRead::Open
        );
        // One black copy among copies that do not match: closed, not held.
        let off = stats([90, 90, 90]);
        assert_eq!(
            entry.classify(&NO_OVERLAY, false, &[black, off, off]),
            PanelRead::Closed
        );
        assert_eq!(
            entry.classify(&NO_OVERLAY, false, &[black, black, off]),
            PanelRead::Closed
        );
        // Only when every copy is black is the read Black.
        assert_eq!(
            entry.classify(&NO_OVERLAY, false, &[black, black, black]),
            PanelRead::Black
        );
        // A black frame, read whole by the replay, is black too.
        assert_eq!(
            entry.classify_frame(&flat([0, 0, 0], 640, 360)),
            PanelRead::Black
        );
        assert_eq!(
            entry.classify_frame(&flat([40, 110, 150], 640, 360)),
            PanelRead::Open
        );
        assert_eq!(
            entry.classify_frame(&flat([90, 90, 90], 640, 360)),
            PanelRead::Closed
        );

        // Open stays open through black reads, and closed stays closed.
        let mut t = PanelTracker::new();
        assert_eq!(t.feed(PanelRead::Black), None);
        assert!(!t.is_open());
        t.feed(entry.classify(&NO_OVERLAY, false, &open));
        assert!(t.is_open());
        for _ in 0..10 {
            assert_eq!(t.feed(PanelRead::Black), None);
        }
        assert!(t.is_open());
        // A black read between two closed reads does not break their run.
        assert_eq!(t.feed(PanelRead::Closed), None);
        assert_eq!(t.feed(PanelRead::Black), None);
        assert_eq!(t.feed(PanelRead::Closed), Some(PanelChange::Closed));
    }

    #[test]
    fn panel_dark_scene_probe_does_not_hold_an_open_entry() {
        // A panel closes over a dark scene: one probe now sits on ground
        // dark enough to copy as black, the others on the scene's colours.
        let entry = left_panel();
        let gold = stats([40, 110, 150]);
        let mut t = PanelTracker::new();
        assert_eq!(
            t.feed(entry.classify(&NO_OVERLAY, false, &[gold, gold, gold])),
            Some(PanelChange::Opened)
        );
        let dark = [stats([0, 0, 0]), stats([60, 70, 50]), stats([90, 90, 90])];
        assert_eq!(entry.classify(&NO_OVERLAY, false, &dark), PanelRead::Closed);
        for _ in 1..PANEL_CLOSED_READS {
            assert_eq!(t.feed(entry.classify(&NO_OVERLAY, false, &dark)), None);
            assert!(t.is_open());
        }
        assert_eq!(
            t.feed(entry.classify(&NO_OVERLAY, false, &dark)),
            Some(PanelChange::Closed)
        );
        assert!(!t.is_open());
    }

    #[test]
    fn panel_ids_are_the_owners_eight() {
        assert_eq!(PANEL_IDS.len(), 8);
        for id in [
            "inventory",
            "passive-tree",
            "escape-menu",
            "stash",
            "merchant",
        ] {
            assert!(PANEL_IDS.contains(&id), "{id}");
        }
        assert!(!PANEL_IDS.contains(&"character"));
        assert_eq!(PANEL_TICK_MS, 250);
    }
}
