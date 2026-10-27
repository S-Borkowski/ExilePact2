//! `quest-tracker-replay panels`: "Hide under game panels" over a kept
//! capture, without the game or a screen.
//!
//! The frames are a live folder's whole-client frames: the
//! `i3/frames.jsonl` lines with band `bottom`, a path, and a `rect` equal to
//! the `client` rect, each decoded from its JPEG by `qt-capture`. Every
//! other line is skipped and counted. A windows file names when each panel
//! was open, one window per line: `<panel-id> <round> <open-utc>
//! <close-utc>`, `#` starting a comment. The open press is made once the
//! panel is fully open and the close press once it has closed, so a window's
//! frames are those from half a second after its open press to half a second
//! before its close press ([`WINDOW_MARGIN_MS`]).
//!
//! - `detect` runs the shipped table's raw per-frame test (no tracker, no
//!   overlay) over every frame, and with a windows file says how many of
//!   each window's frames it caught and how often an entry read open in the
//!   frames well outside every window.
//! - `measure` finds each panel's footprint and ranks probe candidates for
//!   the table. Per window, a cell of a grid of [`CELL_PATCHES`]-patch cells
//!   (32 px at 2560 wide) has changed when its mean colour is at least
//!   [`CHANGED_DISTANCE`] from the window's closed reference on some channel
//!   in [`CHANGED_SHARE`] of the window's frames; the reference is the
//!   median, per cell, of the frames after the previous window's close plus
//!   [`REFERENCE_AFTER_CLOSE_MS`] and up to [`REFERENCE_BEFORE_OPEN_MS`]
//!   before its open press. Changed cells with no changed neighbour are
//!   dropped and the footprint is the bounding box of the rest. A probe
//!   candidate is a patch (16 px at 2560 wide) inside the panel's footprint
//!   whose mean stays within [`STABLE_SPREAD`] of its median on every
//!   channel in [`OPEN_SHARE`] of the rounds' frames; its margin is the
//!   [`CLOSED_SHARE`] quantile, over the frames well outside every window,
//!   of its largest channel distance from that median, and `least` the
//!   strict least. Candidates whose margin exceeds their spread are ranked
//!   by margin, each at least [`CANDIDATE_SPACING`] patches from a better
//!   one. Closed frames near most of them are listed as suspect: a panel
//!   open outside every window.
//!
//! Frames taken with the game not in front, and black copies, take no part
//! in `measure`'s statistics. Output is JSON lines of times, counts, rects
//! and the colour statistics `measure` exists to report; nothing is written.
//!
//! The module stands on its own (`tests/panels.rs` includes it): the UTC
//! parser it needs is handed in by its caller, which is `screen`'s.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use qt_screen::{Image, Millis, PanelRead, PanelTable, ProbeStats, Rect, DEFAULT_OVERLAY_REACH};
use serde_json::{json, Map, Value};

pub const USAGE: &str = "\
quest-tracker-replay panels: \"Hide under game panels\" over a kept capture.

Usage: quest-tracker-replay panels measure --live FOLDER --windows FILE
       quest-tracker-replay panels detect  --live FOLDER [--windows FILE]

Reads FOLDER/i3/frames.jsonl and decodes its whole-client frames (band
`bottom`, rect equal to client); every other line is skipped and counted.
FILE holds one window per line, `<panel-id> <round> <open-utc> <close-utc>`
(UTC as the index writes it); `#` starts a comment.

  measure   per panel id in FILE: its windows, its footprint per window and
            their union, and its ranked probe candidates; then a summary
  detect    the shipped panel table's raw test per frame; with --windows,
            per window the frames it caught, and the frames outside every
            window it read open; then a summary

Options:
  --live FOLDER     the kept live folder (required)
  --windows FILE    the windows file (required by `measure`)
  -h, --help        this text

Exit codes: 0 done, 1 error, 2 no capture index in FOLDER, 64 usage error.
";

/// A window's frames start this long after its open press and end this long
/// before its close press.
pub const WINDOW_MARGIN_MS: Millis = 500;

/// Frames more than this long before every window's open press, or more than
/// [`OUTSIDE_AFTER_MS`] after every window's close press, are outside them.
pub const OUTSIDE_BEFORE_MS: Millis = 3000;
pub const OUTSIDE_AFTER_MS: Millis = 2000;

/// A window's closed reference starts this long after the previous window's
/// close press...
pub const REFERENCE_AFTER_CLOSE_MS: Millis = 1500;
/// ...and ends this long before its own open press.
pub const REFERENCE_BEFORE_OPEN_MS: Millis = 3000;

/// The patch grid, as fractions of the client rect: 16 px patches at
/// 2560x1440.
pub const GRID_COLUMNS: usize = 160;
pub const GRID_ROWS: usize = 90;

/// A footprint cell is this many patches on a side (32 px at 2560 wide).
pub const CELL_PATCHES: usize = 2;

/// A cell has changed in a frame when one channel of its mean is at least
/// this far from the reference's...
pub const CHANGED_DISTANCE: f64 = 24.0;
/// ...and it is part of the footprint when that holds in at least this share
/// of the window's frames (the background moves, so not in all of them).
pub const CHANGED_SHARE: f64 = 0.9;

/// A probe candidate's open frames stay within this of their median, on
/// every channel, in [`OPEN_SHARE`] of them.
pub const STABLE_SPREAD: f64 = 16.0;

/// The share of a panel's open frames a candidate's spread covers: a window
/// may end with a frame or two after the panel closed, since the close press
/// comes once it has.
pub const OPEN_SHARE: f64 = 0.9;

/// A candidate's margin is this quantile of its distance over the closed
/// frames, so that a few frames of a panel opened outside every window do
/// not sink every candidate. `least` reports the strict least beside it, and
/// `closed_near` how many closed frames lie within half the margin.
pub const CLOSED_SHARE: f64 = 0.01;

/// A closed frame within half the margin of at least this share of a
/// panel's candidates is listed in its `suspect_closed`, at most
/// [`SUSPECT_LIMIT`] of them.
pub const SUSPECT_SHARE: f64 = 0.5;
pub const SUSPECT_LIMIT: usize = 20;

/// At most this many candidates are printed per panel...
pub const PROBE_CANDIDATES: usize = 40;
/// ...each at least this many patches (on either axis) from a better one, so
/// they spread over the panel instead of crowding its strongest edge.
pub const CANDIDATE_SPACING: usize = 3;

/// A parser of the index's UTC text into milliseconds.
pub type ParseUtc = fn(&str) -> Option<Millis>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Measure,
    Detect,
}

#[derive(Debug)]
pub struct PanelsArgs {
    pub mode: Mode,
    pub live: PathBuf,
    pub windows: Option<PathBuf>,
}

/// Why a run stopped.
#[derive(Debug)]
pub enum PanelsError {
    /// The live folder has no `i3/frames.jsonl`; the path it looked for.
    NoIndex(PathBuf),
    Error(String),
    /// Writing the output failed.
    Write(io::Error),
}

impl From<io::Error> for PanelsError {
    fn from(e: io::Error) -> Self {
        PanelsError::Write(e)
    }
}

/// Parses `panels`' arguments (the command word already taken). `Ok(None)`
/// means `--help`.
pub fn parse_args(argv: impl IntoIterator<Item = OsString>) -> Result<Option<PanelsArgs>, String> {
    let mut mode = None;
    let mut live = None;
    let mut windows = None;
    let mut rest = argv.into_iter();
    while let Some(arg) = rest.next() {
        let text = arg.to_str().map(str::to_string);
        let (flag, inline) = match text.as_deref() {
            Some(t) if t.starts_with("--") => match t.split_once('=') {
                Some((f, v)) => (f.to_string(), Some(OsString::from(v))),
                None => (t.to_string(), None),
            },
            Some(t) if t.starts_with('-') && t.len() > 1 => (t.to_string(), None),
            Some("measure") if mode.is_none() => {
                mode = Some(Mode::Measure);
                continue;
            }
            Some("detect") if mode.is_none() => {
                mode = Some(Mode::Detect);
                continue;
            }
            _ => return Err(format!("unexpected argument {}", arg.to_string_lossy())),
        };
        let mut value = |name: &str| -> Result<OsString, String> {
            inline
                .clone()
                .or_else(|| rest.next())
                .ok_or_else(|| format!("{name} needs a value"))
        };
        match flag.as_str() {
            "-h" | "--help" => return Ok(None),
            "--live" => live = Some(PathBuf::from(value("--live")?)),
            "--windows" => windows = Some(PathBuf::from(value("--windows")?)),
            other => return Err(format!("unknown option {other}")),
        }
    }
    let mode = mode.ok_or("panels needs `measure` or `detect`")?;
    let live = live.ok_or("panels needs --live FOLDER")?;
    if mode == Mode::Measure && windows.is_none() {
        return Err("panels measure needs --windows FILE".to_string());
    }
    Ok(Some(PanelsArgs {
        mode,
        live,
        windows,
    }))
}

// -- the index ---------------------------------------------------------------------

/// One whole-client frame of the index.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    /// Its time as the index wrote it, printed back as it was.
    pub utc: String,
    pub at: Millis,
    pub foreground: bool,
    /// Its JPEG, resolved against the index's folder.
    pub path: PathBuf,
}

/// The whole-client frames in time order, and how many index lines were
/// something else.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Index {
    pub frames: Vec<Frame>,
    pub skipped: u64,
}

fn four_numbers(v: &Value) -> Option<[i64; 4]> {
    let a = v.as_array()?;
    if a.len() != 4 {
        return None;
    }
    let mut out = [0i64; 4];
    for (o, x) in out.iter_mut().zip(a) {
        *o = x.as_f64()?.round() as i64;
    }
    Some(out)
}

/// `live/i3/frames.jsonl`'s whole-client frames. A missing index is
/// [`PanelsError::NoIndex`]; a line that is not a JSON object, or a frame
/// line without a readable `utc`, is an error naming the file and line,
/// never its content.
pub fn read_index(live: &Path, parse_utc: ParseUtc) -> Result<Index, PanelsError> {
    let i3 = live.join("i3");
    let path = i3.join("frames.jsonl");
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Err(PanelsError::NoIndex(path)),
        Err(e) => return Err(PanelsError::Error(format!("{}: {e}", path.display()))),
    };
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    let mut index = Index::default();
    for (n, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let bad =
            |what: &str| PanelsError::Error(format!("{} line {}: {what}", path.display(), n + 1));
        let v: Value = serde_json::from_str(line).map_err(|_| bad("not a JSON object"))?;
        if !v.is_object() {
            return Err(bad("not a JSON object"));
        }
        let rect = four_numbers(&v["rect"]);
        let whole = v["band"].as_str() == Some("bottom")
            && rect.is_some()
            && rect == four_numbers(&v["client"]);
        let Some(file) = v["path"].as_str().filter(|_| whole) else {
            index.skipped += 1;
            continue;
        };
        let utc = v["utc"].as_str().ok_or_else(|| bad("no utc"))?;
        let at = parse_utc(utc).ok_or_else(|| bad("no readable utc"))?;
        index.frames.push(Frame {
            utc: utc.to_string(),
            at,
            // An index without the field was taken in front.
            foreground: v["foreground"].as_bool().unwrap_or(true),
            path: i3.join(file),
        });
    }
    index.frames.sort_by_key(|f| f.at);
    Ok(index)
}

// -- the windows file ----------------------------------------------------------------

/// One stretch a panel was open.
#[derive(Debug, Clone, PartialEq)]
pub struct Window {
    pub panel: String,
    pub round: String,
    pub from_utc: String,
    pub to_utc: String,
    /// The open press.
    pub from: Millis,
    /// The close press.
    pub to: Millis,
}

impl Window {
    /// Whether a frame at `at` is one of this window's frames.
    pub fn holds(&self, at: Millis) -> bool {
        at >= self.from + WINDOW_MARGIN_MS && at <= self.to - WINDOW_MARGIN_MS
    }
}

/// A windows file's text. A malformed line is an error naming its number.
pub fn parse_windows(text: &str, parse_utc: ParseUtc) -> Result<Vec<Window>, String> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut out = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let bad = |what: &str| format!("line {}: {what}", n + 1);
        let words: Vec<&str> = line.split_whitespace().collect();
        let [panel, round, from_utc, to_utc] = words[..] else {
            return Err(bad("expected `<panel-id> <round> <open-utc> <close-utc>`"));
        };
        let from = parse_utc(from_utc).ok_or_else(|| bad("unreadable open time"))?;
        let to = parse_utc(to_utc).ok_or_else(|| bad("unreadable close time"))?;
        if to <= from {
            return Err(bad("the close press is not after the open press"));
        }
        out.push(Window {
            panel: panel.to_string(),
            round: round.to_string(),
            from_utc: from_utc.to_string(),
            to_utc: to_utc.to_string(),
            from,
            to,
        });
    }
    Ok(out)
}

pub fn read_windows(path: &Path, parse_utc: ParseUtc) -> Result<Vec<Window>, PanelsError> {
    let text = fs::read_to_string(path)
        .map_err(|e| PanelsError::Error(format!("{}: {e}", path.display())))?;
    parse_windows(&text, parse_utc)
        .map_err(|e| PanelsError::Error(format!("{} {e}", path.display())))
}

/// Whether a frame at `at` lies well outside every window.
fn outside_all(windows: &[Window], at: Millis) -> bool {
    windows
        .iter()
        .all(|w| at < w.from - OUTSIDE_BEFORE_MS || at > w.to + OUTSIDE_AFTER_MS)
}

// -- frames ------------------------------------------------------------------------

/// Where a frame's image comes from: the capture's JPEGs, or a test's
/// painter.
pub trait FrameSource {
    fn image(&mut self, frame: &Frame) -> Result<Image, String>;
}

/// The capture's own JPEG files, decoded by `qt-capture` (Windows only).
pub struct JpegFiles;

impl FrameSource for JpegFiles {
    #[cfg(windows)]
    fn image(&mut self, frame: &Frame) -> Result<Image, String> {
        qt_capture::decode_jpeg_file(&frame.path).map_err(|e| e.message)
    }

    #[cfg(not(windows))]
    fn image(&mut self, _frame: &Frame) -> Result<Image, String> {
        Err("decoding a capture's JPEG frames needs Windows".to_string())
    }
}

fn load(source: &mut dyn FrameSource, frame: &Frame) -> Result<Image, PanelsError> {
    source
        .image(frame)
        .map_err(|e| PanelsError::Error(format!("frame {}: {e}", frame.path.display())))
}

fn round_to(x: f64, places: i32) -> f64 {
    let k = 10f64.powi(places);
    (x * k).round() / k
}

fn rect_json(r: &Rect) -> Value {
    json!([
        round_to(r.left, 4),
        round_to(r.top, 4),
        round_to(r.right, 4),
        round_to(r.bottom, 4)
    ])
}

fn colour_json(c: &[f64; 3]) -> Value {
    json!([round_to(c[0], 1), round_to(c[1], 1), round_to(c[2], 1)])
}

// -- detect --------------------------------------------------------------------------

/// `detect`: `table`'s raw test over every frame, then, with `windows`, the
/// per-window and outside lines, then the summary.
pub fn detect(
    index: &Index,
    windows: Option<&[Window]>,
    table: &PanelTable,
    source: &mut dyn FrameSource,
    out: &mut dyn Write,
) -> Result<(), PanelsError> {
    // Per frame, its time and the indices of the entries that read open.
    let mut reads: Vec<(Millis, Vec<usize>)> = Vec::with_capacity(index.frames.len());
    let mut black = 0u64;
    for frame in &index.frames {
        let image = load(source, frame)?;
        if image.is_black() {
            black += 1;
        }
        let open: Vec<usize> = table
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.classify_frame(&image) == PanelRead::Open)
            .map(|(i, _)| i)
            .collect();
        let ids: Vec<&str> = open.iter().map(|&i| table.entries[i].id).collect();
        writeln!(
            out,
            "{}",
            json!({"utc": frame.utc, "foreground": frame.foreground, "open": ids})
        )?;
        reads.push((frame.at, open));
    }
    if let Some(windows) = windows {
        let stands_for = |i: usize, panel: &str| table.entries[i].opens_for.contains(&panel);
        for w in windows {
            let inside: Vec<&Vec<usize>> = reads
                .iter()
                .filter(|(at, _)| w.holds(*at))
                .map(|(_, open)| open)
                .collect();
            let detected = inside
                .iter()
                .filter(|open| open.iter().any(|&i| stands_for(i, &w.panel)))
                .count();
            let covered = (0..table.entries.len()).any(|i| stands_for(i, &w.panel));
            writeln!(
                out,
                "{}",
                json!({
                    "window": w.panel, "round": w.round, "from": w.from_utc, "to": w.to_utc,
                    "frames": inside.len(), "detected": detected, "covered": covered,
                })
            )?;
        }
        let mut frames = 0u64;
        let mut detected = vec![0u64; table.entries.len()];
        for (at, open) in &reads {
            if outside_all(windows, *at) {
                frames += 1;
                for &i in open {
                    detected[i] += 1;
                }
            }
        }
        let detected: Map<String, Value> = table
            .entries
            .iter()
            .zip(detected)
            .map(|(e, n)| (e.id.to_string(), json!(n)))
            .collect();
        writeln!(
            out,
            "{}",
            json!({"outside": {"frames": frames, "detected": detected}})
        )?;
    }
    let table_json: Map<String, Value> = table
        .entries
        .iter()
        .map(|e| (e.id.to_string(), json!(e.opens_for)))
        .collect();
    writeln!(
        out,
        "{}",
        json!({"summary": {
            "frames": index.frames.len(), "skipped": index.skipped, "black": black,
            "table": table_json,
        }})
    )?;
    Ok(())
}

// -- measure -------------------------------------------------------------------------

/// One frame's patch grid: per patch its mean blue, green and red and its
/// luma spread, rounded to whole numbers (a session's grids stay in memory).
struct Grid(Vec<[u8; 4]>);

fn patch_rect(col: usize, row: usize) -> Rect {
    Rect::new(
        col as f64 / GRID_COLUMNS as f64,
        row as f64 / GRID_ROWS as f64,
        (col + 1) as f64 / GRID_COLUMNS as f64,
        (row + 1) as f64 / GRID_ROWS as f64,
    )
}

const CELL_COLUMNS: usize = GRID_COLUMNS / CELL_PATCHES;
const CELL_ROWS: usize = GRID_ROWS / CELL_PATCHES;

fn cell_rect(col: usize, row: usize) -> Rect {
    Rect::new(
        col as f64 / CELL_COLUMNS as f64,
        row as f64 / CELL_ROWS as f64,
        (col + 1) as f64 / CELL_COLUMNS as f64,
        (row + 1) as f64 / CELL_ROWS as f64,
    )
}

fn byte(x: f64) -> u8 {
    x.round().clamp(0.0, 255.0) as u8
}

impl Grid {
    /// The same statistics the probe test reads, per patch.
    fn of(image: &Image) -> Grid {
        let mut cells = Vec::with_capacity(GRID_COLUMNS * GRID_ROWS);
        for row in 0..GRID_ROWS {
            for col in 0..GRID_COLUMNS {
                let s = ProbeStats::of_frame(image, patch_rect(col, row));
                cells.push([
                    byte(s.mean_bgr[0]),
                    byte(s.mean_bgr[1]),
                    byte(s.mean_bgr[2]),
                    byte(s.luma_std),
                ]);
            }
        }
        Grid(cells)
    }

    fn patch(&self, col: usize, row: usize) -> [f64; 3] {
        let p = self.0[row * GRID_COLUMNS + col];
        [p[0] as f64, p[1] as f64, p[2] as f64]
    }

    fn spread(&self, col: usize, row: usize) -> f64 {
        self.0[row * GRID_COLUMNS + col][3] as f64
    }

    /// A cell's mean: the mean of its patches.
    fn cell(&self, col: usize, row: usize) -> [f64; 3] {
        let mut sum = [0.0; 3];
        for dy in 0..CELL_PATCHES {
            for dx in 0..CELL_PATCHES {
                let p = self.patch(col * CELL_PATCHES + dx, row * CELL_PATCHES + dy);
                for c in 0..3 {
                    sum[c] += p[c];
                }
            }
        }
        let n = (CELL_PATCHES * CELL_PATCHES) as f64;
        [sum[0] / n, sum[1] / n, sum[2] / n]
    }
}

/// The largest channel distance between two colours: the probe test's own
/// measure (each channel within its tolerance).
fn distance(a: &[f64; 3], b: &[f64; 3]) -> f64 {
    (0..3).map(|c| (a[c] - b[c]).abs()).fold(0.0, f64::max)
}

fn union(a: Option<Rect>, b: Option<Rect>) -> Option<Rect> {
    match (a, b) {
        (Some(a), Some(b)) => Some(Rect::new(
            a.left.min(b.left),
            a.top.min(b.top),
            a.right.max(b.right),
            a.bottom.max(b.bottom),
        )),
        (a, None) => a,
        (None, b) => b,
    }
}

/// Whether two boxes share any point, touching edges included, as the
/// probe test's readability rule has it.
fn touches(a: &Rect, b: &Rect) -> bool {
    a.left <= b.right && b.left <= a.right && a.top <= b.bottom && b.top <= a.bottom
}

/// The middle value (the mean of the two middle ones for an even count); a
/// passer-by in a few reference frames does not move it.
fn median(mut values: Vec<f64>) -> f64 {
    values.sort_by(f64::total_cmp);
    let n = values.len();
    match n {
        0 => 0.0,
        _ if n % 2 == 1 => values[n / 2],
        _ => (values[n / 2 - 1] + values[n / 2]) / 2.0,
    }
}

/// A window's footprint: the bounding box of the cells that changed from
/// `reference` (per cell, the median of its frames) in [`CHANGED_SHARE`] of
/// `frames`, isolated cells dropped; with the number of cells kept.
fn footprint(frames: &[&Grid], reference: &[&Grid]) -> (Option<Rect>, usize) {
    if frames.is_empty() || reference.is_empty() {
        return (None, 0);
    }
    let need = (CHANGED_SHARE * frames.len() as f64).ceil() as usize;
    let mut changed = vec![false; CELL_COLUMNS * CELL_ROWS];
    for row in 0..CELL_ROWS {
        for col in 0..CELL_COLUMNS {
            let cells: Vec<[f64; 3]> = reference.iter().map(|g| g.cell(col, row)).collect();
            let base = [0, 1, 2].map(|k| median(cells.iter().map(|c| c[k]).collect()));
            let hits = frames
                .iter()
                .filter(|g| distance(&g.cell(col, row), &base) >= CHANGED_DISTANCE)
                .count();
            changed[row * CELL_COLUMNS + col] = hits >= need;
        }
    }
    let is = |col: isize, row: isize| {
        col >= 0
            && row >= 0
            && (col as usize) < CELL_COLUMNS
            && (row as usize) < CELL_ROWS
            && changed[row as usize * CELL_COLUMNS + col as usize]
    };
    let mut bounds: Option<Rect> = None;
    let mut kept = 0;
    for row in 0..CELL_ROWS as isize {
        for col in 0..CELL_COLUMNS as isize {
            if !is(col, row) {
                continue;
            }
            let neighbour = (-1..=1)
                .flat_map(|dy| (-1..=1).map(move |dx| (dx, dy)))
                .any(|(dx, dy)| (dx, dy) != (0, 0) && is(col + dx, row + dy));
            if neighbour {
                kept += 1;
                bounds = union(bounds, Some(cell_rect(col as usize, row as usize)));
            }
        }
    }
    (bounds, kept)
}

struct Candidate {
    col: usize,
    row: usize,
    /// Per channel, the median of the open frames' means.
    centre: [f64; 3],
    /// Per channel, the [`OPEN_SHARE`] quantile of the open frames'
    /// distance from `centre`.
    spread: [f64; 3],
    /// The [`OPEN_SHARE`] quantile of the open frames' luma spread.
    luma_std: f64,
    /// The [`CLOSED_SHARE`] quantile of the closed frames' distance from
    /// `centre`: what candidates are ranked by.
    margin: f64,
    /// The least distance from `centre` over every closed frame.
    least: f64,
}

/// The `q` quantile (0..1) of `values`, by nearest rank; 0 for none.
fn quantile(mut values: Vec<f64>, q: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(f64::total_cmp);
    let i = ((values.len() - 1) as f64 * q.clamp(0.0, 1.0)).round() as usize;
    values[i]
}

/// The ranked probe candidates inside `area`, from the panel's open frames
/// and the session's closed frames; with how many patches qualified.
fn candidates(area: &Rect, open: &[&Grid], closed: &[&Grid]) -> (Vec<Candidate>, usize) {
    if open.is_empty() || closed.is_empty() {
        return (Vec::new(), 0);
    }
    let mut all = Vec::new();
    for row in 0..GRID_ROWS {
        for col in 0..GRID_COLUMNS {
            if !area.contains(&patch_rect(col, row)) {
                continue;
            }
            let means: Vec<[f64; 3]> = open.iter().map(|g| g.patch(col, row)).collect();
            let centre = [0, 1, 2].map(|c| median(means.iter().map(|m| m[c]).collect()));
            let spread = [0, 1, 2].map(|c| {
                let off = means.iter().map(|m| (m[c] - centre[c]).abs()).collect();
                quantile(off, OPEN_SHARE)
            });
            let widest = spread.iter().copied().fold(0.0, f64::max);
            if widest > STABLE_SPREAD {
                continue;
            }
            let away: Vec<f64> = closed
                .iter()
                .map(|g| distance(&g.patch(col, row), &centre))
                .collect();
            let least = away.iter().copied().fold(f64::INFINITY, f64::min);
            let margin = quantile(away, CLOSED_SHARE);
            if margin <= widest {
                continue;
            }
            let luma_std = quantile(
                open.iter().map(|g| g.spread(col, row)).collect(),
                OPEN_SHARE,
            );
            all.push(Candidate {
                col,
                row,
                centre,
                spread,
                luma_std,
                margin,
                least,
            });
        }
    }
    let qualified = all.len();
    all.sort_by(|a, b| b.margin.total_cmp(&a.margin));
    let mut picked: Vec<Candidate> = Vec::new();
    for c in all {
        if picked.len() >= PROBE_CANDIDATES {
            break;
        }
        let crowded = picked.iter().any(|p| {
            p.col.abs_diff(c.col) < CANDIDATE_SPACING && p.row.abs_diff(c.row) < CANDIDATE_SPACING
        });
        if !crowded {
            picked.push(c);
        }
    }
    (picked, qualified)
}

/// Whether closed frame `g` lies within half of `c`'s margin of its centre:
/// a probe at the widest tolerance the table allows would match it there.
fn is_near(c: &Candidate, g: &Grid) -> bool {
    distance(&g.patch(c.col, c.row), &c.centre) <= c.margin / 2.0
}

/// `measure`: per panel id of `windows` (in the order they first appear),
/// its windows, footprints and probe candidates; then a summary line.
pub fn measure(
    index: &Index,
    windows: &[Window],
    source: &mut dyn FrameSource,
    out: &mut dyn Write,
) -> Result<(), PanelsError> {
    // Per usable frame: its time, its utc text and its grid.
    let mut grids: Vec<(Millis, &str, Grid)> = Vec::with_capacity(index.frames.len());
    let (mut black, mut background) = (0u64, 0u64);
    for frame in &index.frames {
        if !frame.foreground {
            background += 1;
            continue;
        }
        let image = load(source, frame)?;
        if image.is_black() {
            black += 1;
            continue;
        }
        grids.push((frame.at, &frame.utc, Grid::of(&image)));
    }
    let (closed_utc, closed): (Vec<&str>, Vec<&Grid>) = grids
        .iter()
        .filter(|(at, _, _)| outside_all(windows, *at))
        .map(|(_, utc, g)| (*utc, g))
        .unzip();

    let mut panels: Vec<&str> = Vec::new();
    for w in windows {
        if !panels.contains(&w.panel.as_str()) {
            panels.push(&w.panel);
        }
    }
    for panel in &panels {
        let mut lines = Vec::new();
        let mut open: Vec<&Grid> = Vec::new();
        let mut whole: Option<Rect> = None;
        for w in windows.iter().filter(|w| w.panel == *panel) {
            let frames: Vec<&Grid> = grids
                .iter()
                .filter(|(at, _, _)| w.holds(*at))
                .map(|(_, _, g)| g)
                .collect();
            let previous_close = windows
                .iter()
                .filter(|o| o.to <= w.from)
                .map(|o| o.to)
                .max();
            let reference: Vec<&Grid> = grids
                .iter()
                .filter(|(at, _, _)| {
                    previous_close.is_none_or(|t| *at > t + REFERENCE_AFTER_CLOSE_MS)
                        && *at <= w.from - REFERENCE_BEFORE_OPEN_MS
                })
                .map(|(_, _, g)| g)
                .collect();
            let (area, cells) = footprint(&frames, &reference);
            whole = union(whole, area);
            lines.push(json!({
                "round": w.round, "from": w.from_utc, "to": w.to_utc,
                "frames": frames.len(), "reference": reference.len(),
                "footprint": area.as_ref().map(rect_json), "changed_cells": cells,
            }));
            open.extend(frames);
        }
        let (probes, qualified) = match &whole {
            Some(area) => candidates(area, &open, &closed),
            None => (Vec::new(), 0),
        };
        // Closed frames near at least half the candidates: most likely a
        // panel open outside every window, worth a look before choosing.
        let suspect: Vec<&str> = closed
            .iter()
            .zip(&closed_utc)
            .filter(|(g, _)| {
                let near = probes.iter().filter(|c| is_near(c, g)).count();
                !probes.is_empty() && near as f64 >= SUSPECT_SHARE * probes.len() as f64
            })
            .map(|(_, utc)| *utc)
            .take(SUSPECT_LIMIT)
            .collect();
        let probes: Vec<Value> = probes
            .iter()
            .map(|c| {
                let rect = patch_rect(c.col, c.row);
                json!({
                    "rect": rect_json(&rect),
                    "bgr": colour_json(&c.centre),
                    "spread": colour_json(&c.spread),
                    "luma_std": round_to(c.luma_std, 1),
                    "margin": round_to(c.margin, 1),
                    "least": round_to(c.least, 1),
                    "closed_near": closed.iter().filter(|g| is_near(c, g)).count(),
                    "outside_reach": !touches(&rect, &DEFAULT_OVERLAY_REACH),
                })
            })
            .collect();
        writeln!(
            out,
            "{}",
            json!({
                "panel": panel, "windows": lines, "frames": open.len(),
                "footprint": whole.as_ref().map(rect_json),
                "candidates": qualified, "probes": probes, "suspect_closed": suspect,
            })
        )?;
    }
    let summary = BTreeMap::from([
        ("frames", json!(index.frames.len())),
        ("skipped", json!(index.skipped)),
        ("black", json!(black)),
        ("background", json!(background)),
        ("closed", json!(closed.len())),
        ("panels", json!(panels.len())),
    ]);
    writeln!(out, "{}", json!({ "summary": summary }))?;
    Ok(())
}

/// Runs `args` over the capture's JPEGs, with the shipped table for
/// `detect`.
pub fn run(args: &PanelsArgs, parse_utc: ParseUtc, out: &mut dyn Write) -> Result<(), PanelsError> {
    let index = read_index(&args.live, parse_utc)?;
    let windows = match &args.windows {
        Some(path) => Some(read_windows(path, parse_utc)?),
        None => None,
    };
    let mut source = JpegFiles;
    match args.mode {
        Mode::Detect => detect(
            &index,
            windows.as_deref(),
            &qt_screen::shipped_panels(),
            &mut source,
            out,
        ),
        Mode::Measure => measure(&index, windows.as_deref().unwrap_or(&[]), &mut source, out),
    }
}
