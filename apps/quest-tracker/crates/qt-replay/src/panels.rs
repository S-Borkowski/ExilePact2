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
//! Two research commands check the table against the raw copies the
//! watcher reads, where `measure` and `detect` read JPEG frames:
//!
//! - `grab` copies the game's whole client area once, through `qt-capture`'s
//!   backend (the GDI route the watcher copies through), with the process
//!   made DPI-aware first, and writes it as an uncompressed 32-bit BMP of
//!   our own ([`write_frame`]). Before any screen call it refuses a file
//!   that exists and a folder inside a git checkout (a `.git` in it or in
//!   any parent): a frame is research data kept beside a session's capture,
//!   never in a repository. It is the only command here that writes.
//! - `probe` reads one frame, a grab or a kept capture's JPEG, and prints
//!   each shipped entry's probe statistics and counts. It cuts each probe at
//!   the pixel rect the watcher copies ([`PanelProbe::pixels`] for the
//!   frame's size) and takes the statistics of that cut as the watcher takes
//!   them of a copy ([`ProbeStats::of_image`]); the counts and the verdict
//!   are `qt-screen`'s own ([`PanelEntry::counts`], [`PanelEntry::verdict`]).
//!   `--overlay` marks the probes touching that rect unreadable, as the
//!   watcher does while the overlay is visible. Numbers go to stdout only.
//!
//! The module stands on its own (`tests/panels.rs` includes it): the UTC
//! parser and formatter it needs are handed in by its caller, which are
//! `screen`'s, and `grab`'s backend comes from a maker its caller passes.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use qt_screen::{
    Image, Millis, PanelRead, PanelTable, PixelRect, ProbeStats, Rect, ScreenBackend,
    DEFAULT_OVERLAY_REACH,
};
use serde_json::{json, Map, Value};

pub const USAGE: &str = "\
quest-tracker-replay panels: \"Hide under game panels\" over a kept capture.

Usage: quest-tracker-replay panels measure --live FOLDER --windows FILE
       quest-tracker-replay panels detect  --live FOLDER [--windows FILE]
       quest-tracker-replay panels grab    --pid PID --out FILE
       quest-tracker-replay panels probe   --frame FILE [--overlay X0,Y0,X1,Y1]

Reads FOLDER/i3/frames.jsonl and decodes its whole-client frames (band
`bottom`, rect equal to client); every other line is skipped and counted.
FILE holds one window per line, `<panel-id> <round> <open-utc> <close-utc>`
(UTC as the index writes it); `#` starts a comment.

  measure   per panel id in FILE: its windows, its footprint per window and
            their union, and its ranked probe candidates; then a summary
  detect    the shipped panel table's raw test per frame; with --windows,
            per window the frames it caught, and the frames outside every
            window it read open; then a summary
  grab      (research) copies the game's whole client area once, as the
            panel watcher copies, and writes it losslessly to FILE, an
            uncompressed 32-bit BMP; refused, before any screen call, when
            FILE exists or its folder is inside a git checkout; prints
            {\"grab\", \"utc\", \"client\": [w, h]}
  probe     (research) per entry of the shipped table, each probe's colour
            statistics and test on one frame (a grab's .bmp or a capture's
            .jpg) at the pixels the watcher copies, and the entry's counts;
            then a summary

Options:
  --live FOLDER        the kept live folder (`measure`, `detect`)
  --windows FILE       the windows file (required by `measure`)
  --pid PID            the game's process id (`grab`)
  --out FILE           the frame to write, outside every git checkout (`grab`)
  --frame FILE         the frame to read (`probe`)
  --overlay X0,Y0,X1,Y1
                       the overlay's rect as fractions of the client rect, as
                       the watcher's `panels: overlay at` line gives them;
                       probes touching it are not read (`probe`)
  -h, --help           this text

Exit codes: 0 done, 1 error (a refused grab included), 2 no capture index in
FOLDER, 64 usage error.
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

/// Milliseconds since the Unix epoch as the index's UTC text.
pub type FormatUtc = fn(Millis) -> String;

/// Makes `grab`'s backend; an error is the reason there is none.
pub type MakeBackend<'a> = dyn FnMut() -> Result<Box<dyn ScreenBackend>, String> + 'a;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Measure,
    Detect,
    Grab,
    Probe,
}

impl Mode {
    fn name(self) -> &'static str {
        match self {
            Mode::Measure => "measure",
            Mode::Detect => "detect",
            Mode::Grab => "grab",
            Mode::Probe => "probe",
        }
    }
}

#[derive(Debug)]
pub struct PanelsArgs {
    pub mode: Mode,
    /// `measure`, `detect`.
    pub live: Option<PathBuf>,
    pub windows: Option<PathBuf>,
    /// `grab`.
    pub pid: Option<u32>,
    pub out: Option<PathBuf>,
    /// `probe`.
    pub frame: Option<PathBuf>,
    pub overlay: Option<Rect>,
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
    let mut pid = None;
    let mut out = None;
    let mut frame = None;
    let mut overlay = None;
    // Each option given, for the check that it belongs to the mode.
    let mut given: Vec<&'static str> = Vec::new();
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
            Some("grab") if mode.is_none() => {
                mode = Some(Mode::Grab);
                continue;
            }
            Some("probe") if mode.is_none() => {
                mode = Some(Mode::Probe);
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
            "--live" => {
                live = Some(PathBuf::from(value("--live")?));
                given.push("--live");
            }
            "--windows" => {
                windows = Some(PathBuf::from(value("--windows")?));
                given.push("--windows");
            }
            "--pid" => {
                let text = value("--pid")?;
                pid = Some(
                    text.to_str()
                        .and_then(|t| t.parse::<u32>().ok())
                        .filter(|&p| p != 0)
                        .ok_or_else(|| {
                            format!("--pid needs a process id, not {}", text.to_string_lossy())
                        })?,
                );
                given.push("--pid");
            }
            "--out" => {
                out = Some(PathBuf::from(value("--out")?));
                given.push("--out");
            }
            "--frame" => {
                frame = Some(PathBuf::from(value("--frame")?));
                given.push("--frame");
            }
            "--overlay" => {
                let text = value("--overlay")?;
                overlay = Some(text.to_str().and_then(parse_overlay).ok_or_else(|| {
                    format!(
                        "--overlay needs four fractions X0,Y0,X1,Y1 from 0 to 1, not {}",
                        text.to_string_lossy()
                    )
                })?);
                given.push("--overlay");
            }
            other => return Err(format!("unknown option {other}")),
        }
    }
    let mode = mode.ok_or("panels needs `measure`, `detect`, `grab` or `probe`")?;
    let allowed: &[&str] = match mode {
        Mode::Measure | Mode::Detect => &["--live", "--windows"],
        Mode::Grab => &["--pid", "--out"],
        Mode::Probe => &["--frame", "--overlay"],
    };
    if let Some(stray) = given.iter().find(|g| !allowed.contains(g)) {
        return Err(format!("panels {} takes no {stray}", mode.name()));
    }
    match mode {
        Mode::Measure | Mode::Detect => {
            if live.is_none() {
                return Err("panels needs --live FOLDER".to_string());
            }
            if mode == Mode::Measure && windows.is_none() {
                return Err("panels measure needs --windows FILE".to_string());
            }
        }
        Mode::Grab => {
            if pid.is_none() {
                return Err("panels grab needs --pid PID".to_string());
            }
            if out.is_none() {
                return Err("panels grab needs --out FILE".to_string());
            }
        }
        Mode::Probe => {
            if frame.is_none() {
                return Err("panels probe needs --frame FILE".to_string());
            }
        }
    }
    Ok(Some(PanelsArgs {
        mode,
        live,
        windows,
        pid,
        out,
        frame,
        overlay,
    }))
}

/// `X0,Y0,X1,Y1` as a rect of fractions: four numbers from 0 to 1, each
/// right and bottom edge past its left and top.
fn parse_overlay(text: &str) -> Option<Rect> {
    let n: Vec<f64> = text
        .split(',')
        .map(|p| p.trim().parse::<f64>().ok())
        .collect::<Option<_>>()?;
    let [x0, y0, x1, y1] = n[..] else {
        return None;
    };
    let unit = |v: f64| (0.0..=1.0).contains(&v);
    (n.iter().all(|&v| unit(v)) && x0 < x1 && y0 < y1).then(|| Rect::new(x0, y0, x1, y1))
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

/// Runs `args`: `measure` and `detect` over the capture's JPEGs, `grab`
/// through `qt-capture`'s backend, `probe` over one frame; the shipped
/// table for `detect` and `probe`.
pub fn run(
    args: &PanelsArgs,
    parse_utc: ParseUtc,
    format_utc: FormatUtc,
    out: &mut dyn Write,
) -> Result<(), PanelsError> {
    let need = |what: Option<&PathBuf>, flag: &str| {
        what.cloned()
            .ok_or_else(|| PanelsError::Error(format!("panels {} needs {flag}", args.mode.name())))
    };
    let capture = || -> Result<(Index, Option<Vec<Window>>), PanelsError> {
        let index = read_index(&need(args.live.as_ref(), "--live FOLDER")?, parse_utc)?;
        let windows = match &args.windows {
            Some(path) => Some(read_windows(path, parse_utc)?),
            None => None,
        };
        Ok((index, windows))
    };
    match args.mode {
        Mode::Detect => {
            let (index, windows) = capture()?;
            detect(
                &index,
                windows.as_deref(),
                &qt_screen::shipped_panels(),
                &mut JpegFiles,
                out,
            )
        }
        Mode::Measure => {
            let (index, windows) = capture()?;
            measure(
                &index,
                windows.as_deref().unwrap_or(&[]),
                &mut JpegFiles,
                out,
            )
        }
        Mode::Grab => {
            let path = need(args.out.as_ref(), "--out FILE")?;
            let pid = args
                .pid
                .ok_or_else(|| PanelsError::Error("panels grab needs --pid PID".to_string()))?;
            let now = || format_utc(now_ms());
            grab(&path, pid, &mut windows_backend, &now, out)
        }
        Mode::Probe => {
            let path = need(args.frame.as_ref(), "--frame FILE")?;
            let image = read_frame(&path)?;
            probe(
                &image,
                &path.display().to_string(),
                &qt_screen::shipped_panels(),
                args.overlay.as_ref(),
                out,
            )
        }
    }
}

// -- grab and probe ------------------------------------------------------------------

/// The size of a BMP's file header and its `BITMAPINFOHEADER`.
const BMP_FILE_HEADER: usize = 14;
const BMP_INFO_HEADER: usize = 40;
const BMP_PIXELS_AT: usize = BMP_FILE_HEADER + BMP_INFO_HEADER;

/// `image` as an uncompressed 32-bit BMP: a `BITMAPINFOHEADER` with a
/// negative height (rows top-down, as the copy holds them) and the copy's
/// own B, G, R, A bytes after it, unchanged.
pub fn frame_bmp(image: &Image) -> Result<Vec<u8>, String> {
    let (w, h) = (image.width as usize, image.height as usize);
    let size = w
        .checked_mul(h)
        .and_then(|n| n.checked_mul(4))
        .filter(|&n| n == image.bgra.len() && n <= (u32::MAX as usize - BMP_PIXELS_AT))
        .ok_or_else(|| format!("the copy's pixels do not fill {w}x{h}"))?;
    let width = i32::try_from(w).map_err(|_| "the copy is too wide".to_string())?;
    let height = i32::try_from(h).map_err(|_| "the copy is too tall".to_string())?;
    let mut b = Vec::with_capacity(BMP_PIXELS_AT + size);
    b.extend_from_slice(b"BM");
    b.extend_from_slice(&((BMP_PIXELS_AT + size) as u32).to_le_bytes());
    b.extend_from_slice(&[0; 4]); // reserved
    b.extend_from_slice(&(BMP_PIXELS_AT as u32).to_le_bytes());
    b.extend_from_slice(&(BMP_INFO_HEADER as u32).to_le_bytes());
    b.extend_from_slice(&width.to_le_bytes());
    b.extend_from_slice(&(-height).to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes()); // planes
    b.extend_from_slice(&32u16.to_le_bytes()); // bits a pixel
    b.extend_from_slice(&0u32.to_le_bytes()); // BI_RGB: uncompressed
    b.extend_from_slice(&(size as u32).to_le_bytes());
    b.extend_from_slice(&[0; 16]); // resolution and palette: none
    b.extend_from_slice(&image.bgra);
    Ok(b)
}

/// A BMP [`frame_bmp`] wrote, back into the image it was made from. Any
/// other BMP (another header, depth, compression or row order) is refused
/// rather than misread.
pub fn read_bmp(bytes: &[u8]) -> Result<Image, String> {
    let not_ours = || "not a frame `panels grab` wrote".to_string();
    if bytes.len() < BMP_PIXELS_AT || &bytes[..2] != b"BM" {
        return Err(not_ours());
    }
    let u32_at =
        |i: usize| u32::from_le_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]);
    let u16_at = |i: usize| u16::from_le_bytes([bytes[i], bytes[i + 1]]);
    let width = u32_at(18) as i32;
    let height = u32_at(22) as i32;
    if u32_at(10) as usize != BMP_PIXELS_AT
        || u32_at(14) as usize != BMP_INFO_HEADER
        || u16_at(26) != 1
        || u16_at(28) != 32
        || u32_at(30) != 0
        || width < 0
        || height > 0
    {
        return Err(not_ours());
    }
    let (w, h) = (width as usize, height.unsigned_abs() as usize);
    let size = w * h * 4;
    if bytes.len() != BMP_PIXELS_AT + size {
        return Err(format!("{}: the pixels do not fill {w}x{h}", not_ours()));
    }
    Ok(Image {
        width: w as u32,
        height: h as u32,
        bgra: bytes[BMP_PIXELS_AT..].to_vec(),
    })
}

/// Writes `image` to `path` as [`frame_bmp`] lays it out. Never over a
/// file: one that exists is an error.
pub fn write_frame(path: &Path, image: &Image) -> io::Result<()> {
    let bytes = frame_bmp(image).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(&bytes)?;
    file.sync_all()
}

/// One frame for `probe`, by its extension: a grab's `.bmp`, or a kept
/// capture's `.jpg` / `.jpeg` decoded by `qt-capture`.
pub fn read_frame(path: &Path) -> Result<Image, PanelsError> {
    let bad = |e: String| PanelsError::Error(format!("{}: {e}", path.display()));
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    match extension.as_deref() {
        Some("bmp") => {
            let bytes = fs::read(path).map_err(|e| bad(e.to_string()))?;
            read_bmp(&bytes).map_err(bad)
        }
        Some("jpg" | "jpeg") => decode_jpeg(path).map_err(bad),
        _ => Err(bad(
            "a frame is a grab's .bmp or a capture's .jpg".to_string()
        )),
    }
}

#[cfg(windows)]
fn decode_jpeg(path: &Path) -> Result<Image, String> {
    qt_capture::decode_jpeg_file(path).map_err(|e| e.message)
}

#[cfg(not(windows))]
fn decode_jpeg(_path: &Path) -> Result<Image, String> {
    Err("decoding a capture's JPEG frames needs Windows".to_string())
}

/// The git checkout `folder` lies in, if any: the nearest folder, `folder`
/// itself or a parent, holding a `.git` (a folder, or a worktree's file).
/// Both the path as given and the real path of its nearest existing folder
/// are walked, so neither a folder not made yet nor a link hides one.
pub fn git_checkout_of(folder: &Path) -> Option<PathBuf> {
    let holds_git = |a: &Path| a.join(".git").symlink_metadata().is_ok();
    if let Some(root) = folder.ancestors().find(|a| holds_git(a)) {
        return Some(root.to_path_buf());
    }
    let existing = folder.ancestors().find(|a| a.is_dir())?;
    let real = fs::canonicalize(existing).ok()?;
    real.ancestors()
        .find(|a| holds_git(a))
        .map(Path::to_path_buf)
}

/// Where `grab` may write `path`: its absolute form, or the one-line reason
/// it may not (the file exists, or its folder is inside a git checkout).
pub fn grab_target(path: &Path) -> Result<PathBuf, String> {
    let target = std::path::absolute(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if target.symlink_metadata().is_ok() {
        return Err(format!(
            "refusing to grab into {}: it already exists",
            target.display()
        ));
    }
    let folder = target.parent().ok_or_else(|| {
        format!(
            "refusing to grab into {}: it has no folder",
            target.display()
        )
    })?;
    if let Some(root) = git_checkout_of(folder) {
        return Err(format!(
            "refusing to grab into {}: its folder is inside a git checkout ({}); keep frames \
             beside the session's capture, outside every repository",
            target.display(),
            root.display()
        ));
    }
    Ok(target)
}

/// `grab`: refuses `path` as [`grab_target`] says before making a backend,
/// then copies process `pid`'s game window's whole client area once, writes
/// it with [`write_frame`] (making its folder) and prints
/// `{"grab": <path>, "utc", "client": [<w>, <h>]}`, `utc` from `now_utc`
/// right after the copy.
pub fn grab(
    path: &Path,
    pid: u32,
    make_backend: &mut MakeBackend<'_>,
    now_utc: &dyn Fn() -> String,
    out: &mut dyn Write,
) -> Result<(), PanelsError> {
    let target = grab_target(path).map_err(PanelsError::Error)?;
    let mut backend = make_backend().map_err(PanelsError::Error)?;
    let screen = |e: qt_screen::BackendError| PanelsError::Error(e.message);
    let window = backend
        .game_window(pid)
        .map_err(screen)?
        .ok_or_else(|| PanelsError::Error(format!("process {pid} has no visible window")))?;
    let client = backend
        .client_rect(window)
        .map_err(screen)?
        .ok_or_else(|| {
            PanelsError::Error(format!(
                "process {pid}'s window is minimized or has no client area; nothing was copied"
            ))
        })?;
    let image = backend.capture(window, client).map_err(screen)?;
    let utc = now_utc();
    if (image.width as i32, image.height as i32) != (client.width(), client.height()) {
        return Err(PanelsError::Error(format!(
            "the copy is {}x{}, not the client area's {}x{}; nothing was written",
            image.width,
            image.height,
            client.width(),
            client.height()
        )));
    }
    let wrote = |e: io::Error| PanelsError::Error(format!("{}: {e}", target.display()));
    if let Some(folder) = target.parent() {
        fs::create_dir_all(folder).map_err(wrote)?;
    }
    write_frame(&target, &image).map_err(wrote)?;
    writeln!(
        out,
        "{}",
        json!({
            "grab": target.display().to_string(), "utc": utc,
            "client": [image.width, image.height],
        })
    )?;
    Ok(())
}

/// The real backend for `grab`: the process made DPI-aware, so that the
/// client rect and the copy are in physical pixels, then `qt-capture`'s
/// backend with no overlay.
#[cfg(windows)]
fn windows_backend() -> Result<Box<dyn ScreenBackend>, String> {
    // A refusal leaves the awareness the process already has; the grab's
    // line names the client size it copied either way.
    let _ = qt_capture::make_process_dpi_aware();
    Ok(qt_capture::backend(None))
}

#[cfg(not(windows))]
fn windows_backend() -> Result<Box<dyn ScreenBackend>, String> {
    Err("grabbing the game's window needs Windows".to_string())
}

fn now_ms() -> Millis {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as Millis)
        .unwrap_or(0)
}

/// The pixels of `area` (inside `image`) as an image of their own: what a
/// copy of that area holds.
fn cut(image: &Image, area: PixelRect) -> Image {
    let w = image.width as usize;
    let (x0, x1) = (area.left.max(0) as usize, area.right.max(0) as usize);
    let (y0, y1) = (area.top.max(0) as usize, area.bottom.max(0) as usize);
    let mut bgra = Vec::with_capacity((x1 - x0) * (y1 - y0) * 4);
    for y in y0..y1 {
        bgra.extend_from_slice(&image.bgra[(y * w + x0) * 4..(y * w + x1) * 4]);
    }
    Image {
        width: (x1 - x0) as u32,
        height: (y1 - y0) as u32,
        bgra,
    }
}

fn read_name(read: PanelRead) -> &'static str {
    match read {
        PanelRead::Open => "open",
        PanelRead::Closed => "closed",
        PanelRead::Unseen => "unseen",
        PanelRead::Black => "black",
    }
}

/// `probe`: for each entry of `table`, one line per probe with its
/// statistics and test on `image` (a copy of the whole client rect), then
/// the entry's read and counts; then `{"summary": {"frame", "client"}}`.
/// A probe is read as the watcher reads it: cut at its pixel rect for the
/// frame's size, its statistics those of the cut, and not read when
/// `overlay` (fractions, the overlay visible) touches it or its rect is
/// empty. An unread probe's statistics are null.
pub fn probe(
    image: &Image,
    frame: &str,
    table: &PanelTable,
    overlay: Option<&Rect>,
    out: &mut dyn Write,
) -> Result<(), PanelsError> {
    let (w, h) = (image.width, image.height);
    if image.bgra.len() != w as usize * h as usize * 4 {
        return Err(PanelsError::Error(format!(
            "{frame}: the pixels do not fill {w}x{h}"
        )));
    }
    let client = PixelRect::new(0, 0, w as i32, h as i32);
    let nowhere = Rect::new(0.0, 0.0, 0.0, 0.0);
    let (place, visible) = match overlay {
        Some(r) => (*r, true),
        None => (nowhere, false),
    };
    for entry in &table.entries {
        let mut copies: Vec<Option<ProbeStats>> = vec![None; entry.probes.len()];
        for (i, p) in entry.probes.iter().enumerate() {
            let area = p.pixels(client);
            if p.is_readable(&place, visible) && !area.is_empty() {
                copies[i] = Some(ProbeStats::of_image(&cut(image, area)));
            }
            let line = match &copies[i] {
                Some(s) => json!({
                    "entry": entry.id, "probe": i, "readable": true,
                    "mean_bgr": colour_json(&s.mean_bgr), "luma_std": round_to(s.luma_std, 2),
                    "in_colour": p.in_colour(s), "within_spread": p.within_spread(s),
                    "match": p.matches(s),
                }),
                None => json!({
                    "entry": entry.id, "probe": i, "readable": false,
                    "mean_bgr": null, "luma_std": null,
                    "in_colour": false, "within_spread": false, "match": false,
                }),
            };
            writeln!(out, "{line}")?;
        }
        let counts = entry.counts(&place, visible, &copies);
        writeln!(
            out,
            "{}",
            json!({
                "entry": entry.id, "read": read_name(entry.verdict(&counts)),
                "counts": {
                    "read": counts.read, "in_colour": counts.in_colour,
                    "within_spread": counts.within_spread, "match": counts.matched,
                    "needed": entry.min_match,
                },
            })
        )?;
    }
    writeln!(
        out,
        "{}",
        json!({"summary": {"frame": frame, "client": [w, h]}})
    )?;
    Ok(())
}
