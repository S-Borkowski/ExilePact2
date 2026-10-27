//! `quest-tracker-replay panels` over synthetic sessions: frames painted in
//! memory from numbers (a gradient world, a wandering bright square, a
//! panel's flat body and header), never a decoded file, a kept capture or
//! the screen.
//!
//! `qt-replay` is a binary, so the module is included here by path and
//! driven through its frame source with a painter; the refusal of a folder
//! without its index runs the binary itself.
//!
//! `grab` is driven through a fake backend handing back a painted copy, and
//! its refusals through a backend maker that records whether it was asked:
//! no test copies the screen.

use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Mutex};

use qt_screen::{
    BackendError, Image, Millis, OcrReadiness, OverlayState, PanelEntry, PanelProbe, PanelTable,
    PixelLine, PixelRect, ProbeStats, Rect, ScreenBackend, WindowHandle,
};
use serde_json::{json, Value};

#[allow(dead_code)]
#[path = "../src/panels.rs"]
mod panels;

use panels::{Frame, FrameSource, Index, PanelsError};

const BIN: &str = env!("CARGO_BIN_EXE_quest-tracker-replay");

/// Frames are 640x360 (16:9): one patch is 4 px, one footprint cell 8 px.
const W: u32 = 640;
const H: u32 = 360;

/// The inventory's frame: a flat body and a header strip along its top.
const PANEL: Rect = Rect::new(0.10, 0.20, 0.40, 0.80);
const HEADER: Rect = Rect::new(0.10, 0.20, 0.40, 0.30);
const BODY_BGR: [u8; 3] = [200, 20, 230];
const HEADER_BGR: [u8; 3] = [10, 230, 40];
/// Another panel, which the synthetic table does not cover.
const OTHER: Rect = Rect::new(0.55, 0.10, 0.90, 0.50);
const OTHER_BGR: [u8; 3] = [90, 200, 200];
/// One footprint cell that flickers while the inventory is open: changed in
/// every window frame, but alone, so it is no part of the footprint.
const SPARK: Rect = Rect::new(70.0 / 80.0, 5.0 / 45.0, 71.0 / 80.0, 6.0 / 45.0);

/// The session: a frame every 500 ms from 0 to 60 s.
const STEP: Millis = 500;
const END: Millis = 60_000;

/// `2026-10-26T20:MM:SS.mmmZ` for `ms` after 20:00.
fn utc(ms: Millis) -> String {
    let s = ms / 1000;
    format!(
        "2026-10-26T20:{:02}:{:02}.{:03}Z",
        s / 60,
        s % 60,
        ms % 1000
    )
}

/// [`utc`]'s inverse: the stand-in for `screen`'s parser the binary hands
/// the module.
fn parse_utc(text: &str) -> Option<Millis> {
    let rest = text.strip_prefix("2026-10-26T20:")?.strip_suffix('Z')?;
    let (mm, rest) = rest.split_once(':')?;
    let (ss, ms) = rest.split_once('.')?;
    let n = |t: &str| t.parse::<Millis>().ok();
    Some((n(mm)? * 60 + n(ss)?) * 1000 + n(ms)?)
}

fn within(ms: Millis, from: Millis, to: Millis) -> bool {
    (from..=to).contains(&ms)
}

/// Paints one frame: the world at `ms`, then each rect over it.
fn paint(ms: Millis, rects: &[(Rect, [u8; 3])]) -> Image {
    let mut bgra = Vec::with_capacity((W * H * 4) as usize);
    // A bright square wanders across the panel's rows, as an NPC would.
    let square_x = ((ms / STEP) * 24) as u32 % (W - 16);
    for y in 0..H {
        for x in 0..W {
            let mut c = [(x * 200 / W) as u8, (y * 200 / H) as u8, 100];
            if (square_x..square_x + 16).contains(&x) && (200..216).contains(&y) {
                c = [250, 250, 250];
            }
            let (fx, fy) = ((x as f64 + 0.5) / W as f64, (y as f64 + 0.5) / H as f64);
            for (r, colour) in rects {
                if fx >= r.left && fx < r.right && fy >= r.top && fy < r.bottom {
                    c = *colour;
                }
            }
            bgra.extend_from_slice(&[c[0], c[1], c[2], 255]);
        }
    }
    Image {
        width: W,
        height: H,
        bgra,
    }
}

fn inventory() -> Vec<(Rect, [u8; 3])> {
    vec![
        (PANEL, BODY_BGR),
        (HEADER, HEADER_BGR),
        (SPARK, [255, 255, 255]),
    ]
}

/// A frame source that paints each frame from its time.
struct Painter<F: Fn(Millis) -> Image>(F);

impl<F: Fn(Millis) -> Image> FrameSource for Painter<F> {
    fn image(&mut self, frame: &Frame) -> Result<Image, String> {
        Ok((self.0)(frame.at))
    }
}

/// Writes `lines` as `live/i3/frames.jsonl` and reads it back.
fn index_of(live: &Path, lines: &[Value]) -> Index {
    let i3 = live.join("i3");
    fs::create_dir_all(&i3).unwrap();
    let text: String = lines.iter().map(|v| format!("{v}\n")).collect();
    fs::write(i3.join("frames.jsonl"), text).unwrap();
    panels::read_index(live, parse_utc).unwrap_or_else(|e| panic!("index: {e:?}"))
}

/// One whole-client frame line at `ms`.
fn frame_line(ms: Millis, foreground: bool) -> Value {
    json!({
        "utc": utc(ms), "band": "bottom",
        "path": format!("frames/bottom/{ms}.jpg"),
        "rect": [0, 0, 2560, 1440], "client": [0, 0, 2560, 1440],
        "foreground": foreground,
    })
}

fn session_lines() -> Vec<Value> {
    (0..=END / STEP)
        .map(|i| frame_line(i * STEP, true))
        .collect()
}

fn objects(bytes: &[u8]) -> Vec<Value> {
    String::from_utf8(bytes.to_vec())
        .expect("UTF-8 output")
        .lines()
        .map(|l| serde_json::from_str(l).expect("one JSON object per line"))
        .collect()
}

fn rect_of(v: &Value) -> Rect {
    let n: Vec<f64> = v
        .as_array()
        .expect("a rect")
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect();
    Rect::new(n[0], n[1], n[2], n[3])
}

fn near(a: &Rect, b: &Rect, slack: f64) -> bool {
    (a.left - b.left).abs() <= slack
        && (a.top - b.top).abs() <= slack
        && (a.right - b.right).abs() <= slack
        && (a.bottom - b.bottom).abs() <= slack
}

#[test]
fn panels_measure_finds_a_synthetic_panel_footprint() {
    let dir = tempfile::tempdir().unwrap();
    let index = index_of(dir.path(), &session_lines());
    // The inventory twice; `key-o` once with nothing drawn (the negative
    // control). The inventory shows a second before each open press; it
    // closes a second before the first close press, so the first window's
    // last frame is already closed, and half a second before the second.
    // Once more it flashes up unasked at 3 s, in a closed frame.
    let windows = panels::parse_windows(
        &format!(
            "# panel round open close\n\
             inventory 1 {} {}\n\
             inventory 2 {} {}  # the second round\n\
             \n\
             key-o 1 {} {}\n",
            utc(10_000),
            utc(16_000),
            utc(30_000),
            utc(36_000),
            utc(45_000),
            utc(50_000),
        ),
        parse_utc,
    )
    .expect("the windows file");
    assert_eq!(windows.len(), 3);
    let mut source = Painter(|ms| {
        if within(ms, 9_000, 15_000) || within(ms, 29_000, 35_500) || ms == 3_000 {
            paint(ms, &inventory())
        } else {
            paint(ms, &[])
        }
    });
    let mut out = Vec::new();
    panels::measure(&index, &windows, &mut source, &mut out).expect("measure");
    let lines = objects(&out);
    assert_eq!(lines.len(), 3, "two panels and a summary: {lines:?}");

    let inv = &lines[0];
    assert_eq!(inv["panel"], "inventory");
    let rounds = inv["windows"].as_array().unwrap();
    assert_eq!(rounds.len(), 2);
    // 10.5 s to 15.5 s, and 30.5 s to 35.5 s.
    assert_eq!(rounds[0]["frames"], 11);
    assert_eq!(rounds[1]["frames"], 11);
    assert_eq!(inv["frames"], 22);
    // The first round's reference runs to 3 s before its open press; the
    // second's from 1.5 s after the first's close press.
    assert_eq!(rounds[0]["reference"], 15);
    assert_eq!(rounds[1]["reference"], 19);
    for round in rounds {
        let fp = rect_of(&round["footprint"]);
        assert!(near(&fp, &PANEL, 0.03), "round footprint {fp:?}");
    }
    let whole = rect_of(&inv["footprint"]);
    assert!(near(&whole, &PANEL, 0.03), "footprint {whole:?}");
    assert!(
        whole.right < SPARK.left,
        "the lone spark is dropped: {whole:?}"
    );

    // The candidates sit inside the panel, read one of its two colours, and
    // stand further from the closed frames than they strayed open; the one
    // unasked frame is the least distance of each, near each, and suspect.
    let probes = inv["probes"].as_array().unwrap();
    assert!(probes.len() >= 3, "{probes:?}");
    assert!(inv["candidates"].as_u64().unwrap() >= probes.len() as u64);
    for p in probes {
        let r = rect_of(&p["rect"]);
        assert!(whole.contains(&r), "{r:?} outside {whole:?}");
        let bgr: Vec<f64> = p["bgr"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_f64().unwrap())
            .collect();
        let is = |c: [u8; 3]| (0..3).all(|k| (bgr[k] - c[k] as f64).abs() <= 2.0);
        assert!(is(BODY_BGR) || is(HEADER_BGR), "{p}");
        let spread = p["spread"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_f64().unwrap())
            .fold(0.0, f64::max);
        assert!(p["margin"].as_f64().unwrap() > spread, "{p}");
        assert!(p["least"].as_f64().unwrap() <= 1.0, "{p}");
        assert_eq!(p["closed_near"], 1, "{p}");
        assert_eq!(p["outside_reach"], true);
    }
    assert_eq!(inv["suspect_closed"], json!([utc(3_000)]));
    let margins: Vec<f64> = probes
        .iter()
        .map(|p| p["margin"].as_f64().unwrap())
        .collect();
    assert!(
        margins.windows(2).all(|m| m[0] >= m[1]),
        "ranked: {margins:?}"
    );

    // Nothing drew for `key-o`: no footprint, no candidates.
    let o = &lines[1];
    assert_eq!(o["panel"], "key-o");
    assert_eq!(o["frames"], 9);
    assert!(o["footprint"].is_null(), "{o}");
    assert!(o["windows"][0]["footprint"].is_null(), "{o}");
    assert_eq!(o["probes"].as_array().unwrap().len(), 0);

    let s = &lines[2]["summary"];
    assert_eq!(s["frames"], 121);
    assert_eq!(s["skipped"], 0);
    assert_eq!(s["black"], 0);
    assert_eq!(s["panels"], 2);
}

/// The table `detect` runs in this test: the inventory's frame, standing
/// for the stash's window too.
fn synthetic_table() -> PanelTable {
    let probe = |rect: Rect, bgr: [u8; 3]| PanelProbe {
        rect,
        bgr: bgr.map(f64::from),
        tolerance: [12.0; 3],
        max_luma_std: 6.0,
    };
    PanelTable::new(vec![PanelEntry {
        id: "inventory-frame",
        opens_for: vec!["inventory", "stash"],
        footprint: PANEL,
        probes: vec![
            probe(Rect::new(0.15, 0.21, 0.17, 0.24), HEADER_BGR),
            probe(Rect::new(0.20, 0.45, 0.22, 0.48), BODY_BGR),
            probe(Rect::new(0.35, 0.70, 0.37, 0.73), BODY_BGR),
        ],
        min_match: 2,
    }])
}

#[test]
fn panels_detect_reports_windows_and_outside_frames() {
    let dir = tempfile::tempdir().unwrap();
    let mut lines = session_lines();
    // Lines that are no whole-client frame: skipped and counted.
    lines.push(json!({
        "utc": utc(1_250), "band": "top", "path": "frames/top/a.jpg",
        "rect": [640, 0, 1920, 202], "client": [0, 0, 2560, 1440],
    }));
    lines.push(json!({
        "utc": utc(1_750), "band": "bottom", "path": "frames/bottom/b.jpg",
        "rect": [0, 504, 2560, 1440], "client": [0, 0, 2560, 1440],
    }));
    lines.push(json!({"utc": utc(2_250), "band": "bottom", "skipped": "minimized"}));
    // A frame taken with the game behind another window.
    lines.push(frame_line(40_250, false));
    let index = index_of(dir.path(), &lines);
    assert_eq!(index.frames.len(), 122);
    assert_eq!(index.skipped, 3);

    let windows = panels::parse_windows(
        &format!(
            "inventory 1 {} {}\ninventory 2 {} {}\npassive-tree 1 {} {}\n",
            utc(10_000),
            utc(16_000),
            utc(30_000),
            utc(36_000),
            utc(45_000),
            utc(50_000),
        ),
        parse_utc,
    )
    .unwrap();
    // The inventory in both its windows and once more, unasked, at 55-57 s;
    // the passive tree in its window; a black copy at 20 s.
    let mut source = Painter(|ms| {
        if ms == 20_000 {
            Image {
                width: W,
                height: H,
                bgra: vec![0; (W * H * 4) as usize],
            }
        } else if within(ms, 9_000, 15_500)
            || within(ms, 29_000, 35_500)
            || within(ms, 55_000, 57_000)
        {
            paint(ms, &inventory())
        } else if within(ms, 44_000, 49_500) {
            paint(ms, &[(OTHER, OTHER_BGR)])
        } else {
            paint(ms, &[])
        }
    });
    let mut out = Vec::new();
    panels::detect(
        &index,
        Some(&windows),
        &synthetic_table(),
        &mut source,
        &mut out,
    )
    .expect("detect");
    let lines = objects(&out);
    let (frames, rest) = lines.split_at(index.frames.len());

    // Per frame: the raw test, open exactly while the inventory is drawn.
    for f in frames {
        let at = parse_utc(f["utc"].as_str().unwrap()).unwrap();
        let drawn =
            within(at, 9_000, 15_500) || within(at, 29_000, 35_500) || within(at, 55_000, 57_000);
        let want = if drawn {
            json!(["inventory-frame"])
        } else {
            json!([])
        };
        assert_eq!(f["open"], want, "{f}");
        assert_eq!(f["foreground"], at != 40_250, "{f}");
    }

    assert_eq!(rest.len(), 5, "three windows, outside, summary: {rest:?}");
    let window = |i: usize| &rest[i];
    for i in 0..2 {
        let w = window(i);
        assert_eq!(w["window"], "inventory");
        assert_eq!(w["frames"], 11, "{w}");
        assert_eq!(w["detected"], 11, "{w}");
        assert_eq!(w["covered"], true);
    }
    assert_eq!(window(0)["round"], "1");
    assert_eq!(window(0)["from"], utc(10_000));
    assert_eq!(window(0)["to"], utc(16_000));
    // No entry stands for the passive tree: drawn, but not detected.
    let p = window(2);
    assert_eq!(p["window"], "passive-tree");
    assert_eq!(p["frames"], 9);
    assert_eq!(p["detected"], 0);
    assert_eq!(p["covered"], false);

    // Outside: before 7 s, 18-27 s, 38-42 s (and the background frame), and
    // after 52 s; the unasked inventory at 55-57 s is all it read open.
    let outside = &rest[3]["outside"];
    assert_eq!(outside["frames"], 14 + 17 + 8 + 16, "{outside}");
    assert_eq!(outside["detected"], json!({"inventory-frame": 5}));

    let s = &rest[4]["summary"];
    assert_eq!(s["frames"], 122);
    assert_eq!(s["skipped"], 3);
    assert_eq!(s["black"], 1);
    assert_eq!(
        s["table"],
        json!({"inventory-frame": ["inventory", "stash"]})
    );

    // Without a windows file: the frames and the summary only.
    let mut out = Vec::new();
    panels::detect(&index, None, &synthetic_table(), &mut source, &mut out).unwrap();
    let lines = objects(&out);
    assert_eq!(lines.len(), index.frames.len() + 1);
    assert!(lines.last().unwrap()["summary"].is_object());
}

#[test]
fn panels_replay_refuses_a_folder_without_its_index() {
    let dir = tempfile::tempdir().unwrap();
    let live = dir.path().join("live");
    fs::create_dir_all(live.join("i3")).unwrap();
    let windows = dir.path().join("windows.txt");
    fs::write(&windows, format!("inventory 1 {} {}\n", utc(0), utc(5_000))).unwrap();
    let live_arg = live.to_str().unwrap();
    let windows_arg = windows.to_str().unwrap();

    for args in [
        vec!["panels", "detect", "--live", live_arg],
        vec![
            "panels",
            "detect",
            "--live",
            live_arg,
            "--windows",
            windows_arg,
        ],
        vec![
            "panels",
            "measure",
            "--live",
            live_arg,
            "--windows",
            windows_arg,
        ],
    ] {
        let o = Command::new(BIN)
            .args(&args)
            .output()
            .expect("the binary runs");
        assert_eq!(o.status.code(), Some(2), "{args:?}");
        assert!(o.stdout.is_empty(), "{args:?}");
        let err = String::from_utf8(o.stderr).unwrap();
        let err_lines: Vec<&str> = err.lines().collect();
        assert_eq!(err_lines.len(), 1, "{args:?}: {err}");
        assert!(err_lines[0].contains("frames.jsonl"), "{err}");
    }

    // In the module: the path it looked for.
    match panels::read_index(&live, parse_utc) {
        Err(PanelsError::NoIndex(path)) => assert!(path.ends_with("i3/frames.jsonl")),
        other => panic!("expected NoIndex, got {other:?}"),
    }

    // `measure` without a windows file is a usage error, before any read.
    let o = Command::new(BIN)
        .args(["panels", "measure", "--live", live_arg])
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(64));
    // `--help` names both modes.
    let o = Command::new(BIN)
        .args(["panels", "--help"])
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(0));
    let help = String::from_utf8(o.stdout).unwrap();
    assert!(help.contains("panels measure") && help.contains("panels detect"));
    // The top-level usage names the command.
    let o = Command::new(BIN).arg("--help").output().unwrap();
    assert!(String::from_utf8(o.stdout).unwrap().contains("panels"));
}

// -- grab and probe ------------------------------------------------------------------

/// An image whose every byte, alpha included, depends on its place, at a
/// width that is no multiple of 4.
fn speckled(width: u32, height: u32) -> Image {
    let bgra = (0..width * height * 4)
        .map(|i| (i.wrapping_mul(2_654_435_761) >> 13) as u8)
        .collect();
    Image {
        width,
        height,
        bgra,
    }
}

/// A backend with one game window, of process `pid`, whose client rect is
/// `client` and whose copies are `image`; it records the area it was asked
/// to copy.
struct FakeGame {
    pid: u32,
    client: PixelRect,
    image: Image,
    copied: Arc<Mutex<Option<PixelRect>>>,
}

impl ScreenBackend for FakeGame {
    fn game_window(&mut self, pid: u32) -> Result<Option<WindowHandle>, BackendError> {
        Ok((pid == self.pid).then_some(WindowHandle(7)))
    }
    fn foreground_pid(&mut self) -> Result<Option<u32>, BackendError> {
        Ok(Some(self.pid))
    }
    fn client_rect(&mut self, window: WindowHandle) -> Result<Option<PixelRect>, BackendError> {
        assert_eq!(window, WindowHandle(7));
        Ok(Some(self.client))
    }
    fn overlay(&mut self) -> Result<Option<OverlayState>, BackendError> {
        Ok(None)
    }
    fn capture(&mut self, _window: WindowHandle, area: PixelRect) -> Result<Image, BackendError> {
        *self.copied.lock().unwrap() = Some(area);
        Ok(self.image.clone())
    }
    fn ocr_readiness(&mut self) -> Result<OcrReadiness, BackendError> {
        panic!("grab never reads text")
    }
    fn ocr(&mut self, _image: &Image) -> Result<Vec<PixelLine>, BackendError> {
        panic!("grab never reads text")
    }
}

fn fixed_utc() -> String {
    "2026-10-27T12:00:00.000Z".to_string()
}

#[test]
fn panels_grab_frame_round_trips_losslessly() {
    let dir = tempfile::tempdir().unwrap();

    // The writer and the reader, byte for byte.
    let image = speckled(37, 23);
    let path = dir.path().join("frame.bmp");
    panels::write_frame(&path, &image).expect("write");
    let bytes = fs::read(&path).unwrap();
    assert_eq!(&bytes[..2], b"BM");
    assert_eq!(bytes.len(), 54 + 37 * 23 * 4, "uncompressed 32-bit");
    assert_eq!(&bytes[54..], &image.bgra[..], "the pixels as copied");
    let back = panels::read_frame(&path).expect("read");
    assert_eq!(back, image);
    // It never writes over a file.
    assert!(panels::write_frame(&path, &speckled(2, 2)).is_err());
    assert_eq!(fs::read(&path).unwrap(), bytes);
    // A file it did not write is refused rather than misread.
    let other = dir.path().join("other.bmp");
    let mut foreign = bytes.clone();
    foreign[28] = 24; // 24 bits a pixel
    fs::write(&other, &foreign).unwrap();
    assert!(panels::read_frame(&other).is_err());

    // `grab` through a backend: the whole client rect, once, into a folder
    // it makes, and one JSON line naming the client's size.
    let client = PixelRect::new(100, 50, 164, 86);
    let painted = speckled(64, 36);
    let copied = Arc::new(Mutex::new(None));
    let mut made = 0;
    let mut make = || -> Result<Box<dyn ScreenBackend>, String> {
        made += 1;
        Ok(Box::new(FakeGame {
            pid: 4242,
            client,
            image: painted.clone(),
            copied: Arc::clone(&copied),
        }))
    };
    let target = dir.path().join("grabs").join("inventory-open.bmp");
    let mut out = Vec::new();
    panels::grab(&target, 4242, &mut make, &fixed_utc, &mut out).expect("grab");
    assert_eq!(
        *copied.lock().unwrap(),
        Some(client),
        "the whole client rect"
    );
    let lines = objects(&out);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert_eq!(lines[0]["client"], json!([64, 36]));
    assert_eq!(lines[0]["utc"], fixed_utc());
    assert!(lines[0]["grab"]
        .as_str()
        .unwrap()
        .ends_with("inventory-open.bmp"));
    assert_eq!(panels::read_frame(&target).expect("read"), painted);

    // A pid with no window is an error, and nothing is written.
    let none = dir.path().join("grabs").join("nobody.bmp");
    match panels::grab(&none, 1, &mut make, &fixed_utc, &mut Vec::new()) {
        Err(PanelsError::Error(message)) => assert!(message.contains("process 1"), "{message}"),
        other => panic!("expected an error, got {other:?}"),
    }
    assert!(!none.exists());
    assert_eq!(made, 2, "one backend per grab");
}

#[test]
fn panels_grab_refuses_a_path_inside_a_git_checkout() {
    let dir = tempfile::tempdir().unwrap();
    let mut asked = 0u32;
    let mut make = || -> Result<Box<dyn ScreenBackend>, String> {
        asked += 1;
        Err("no backend in this test".to_string())
    };

    // A `.git` folder in the frame's folder, or a `.git` file (a worktree)
    // in a parent of a folder not made yet.
    let repo = dir.path().join("repo");
    fs::create_dir_all(repo.join(".git")).unwrap();
    let worktree = dir.path().join("worktree");
    fs::create_dir_all(&worktree).unwrap();
    fs::write(worktree.join(".git"), "gitdir: elsewhere\n").unwrap();
    for target in [
        repo.join("frame.bmp"),
        worktree.join("a").join("b").join("frame.bmp"),
    ] {
        match panels::grab(&target, 4242, &mut make, &fixed_utc, &mut Vec::new()) {
            Err(PanelsError::Error(message)) => {
                assert!(message.contains("inside a git checkout"), "{message}");
                assert!(!message.contains('\n'), "one line: {message}");
            }
            other => panic!("expected a refusal, got {other:?}"),
        }
        assert!(!target.exists());
    }
    assert!(!worktree.join("a").exists(), "no folder made");

    // An existing file, outside any checkout, is never written over.
    let kept = dir.path().join("kept.bmp");
    fs::write(&kept, "keep").unwrap();
    match panels::grab(&kept, 4242, &mut make, &fixed_utc, &mut Vec::new()) {
        Err(PanelsError::Error(message)) => {
            assert!(message.contains("already exists"), "{message}")
        }
        other => panic!("expected a refusal, got {other:?}"),
    }
    assert_eq!(fs::read_to_string(&kept).unwrap(), "keep");
    assert_eq!(asked, 0, "refused before any backend was made");

    // The binary: the error exit with one line, never 2 (no capture index).
    let target = repo.join("sub").join("frame.bmp");
    let o = Command::new(BIN)
        .args(["panels", "grab", "--pid", "1", "--out"])
        .arg(&target)
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(1));
    assert!(o.stdout.is_empty());
    let err = String::from_utf8(o.stderr).unwrap();
    assert_eq!(err.lines().count(), 1, "{err}");
    assert!(err.contains("inside a git checkout"), "{err}");
    assert!(!target.exists() && !repo.join("sub").exists());
    let o = Command::new(BIN)
        .args(["panels", "grab", "--out"])
        .arg(dir.path().join("x.bmp"))
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(64), "no --pid");
    // `--help` names the research commands.
    let o = Command::new(BIN)
        .args(["panels", "--help"])
        .output()
        .unwrap();
    let help = String::from_utf8(o.stdout).unwrap();
    assert!(help.contains("panels grab") && help.contains("panels probe"));
}

/// A probe's test colour, and the colour of a probe that is nowhere near it.
const PROBE_BGR: [u8; 3] = [60, 120, 180];
const FAR_BGR: [u8; 3] = [200, 30, 30];

/// The probe test's table: probe 0 in colour but textured, probe 1 flat but
/// in another colour, probe 2 matching.
fn probe_table() -> PanelTable {
    let probe = |rect: Rect| PanelProbe {
        rect,
        bgr: PROBE_BGR.map(f64::from),
        tolerance: [10.0; 3],
        max_luma_std: 6.0,
    };
    PanelTable::new(vec![PanelEntry {
        id: "test-frame",
        opens_for: vec!["inventory"],
        footprint: Rect::new(0.05, 0.05, 0.60, 0.40),
        probes: vec![
            probe(Rect::new(0.10, 0.10, 0.125, 0.15)),
            probe(Rect::new(0.30, 0.10, 0.325, 0.15)),
            probe(Rect::new(0.50, 0.30, 0.525, 0.35)),
        ],
        min_match: 1,
    }])
}

/// A grey frame with the three probes' patches painted: a checkerboard of
/// the probe colour plus and minus 40 (its mean in colour, its spread far
/// too wide), a flat far colour, and the flat probe colour.
fn probe_frame(table: &PanelTable) -> Image {
    let client = PixelRect::new(0, 0, W as i32, H as i32);
    let areas: Vec<PixelRect> = table.entries[0]
        .probes
        .iter()
        .map(|p| p.pixels(client))
        .collect();
    let inside =
        |a: &PixelRect, x: i32, y: i32| x >= a.left && x < a.right && y >= a.top && y < a.bottom;
    let mut bgra = Vec::with_capacity((W * H * 4) as usize);
    for y in 0..H as i32 {
        for x in 0..W as i32 {
            let c = if inside(&areas[0], x, y) {
                let d: i16 = if (x + y) % 2 == 0 { 40 } else { -40 };
                PROBE_BGR.map(|v| (v as i16 + d) as u8)
            } else if inside(&areas[1], x, y) {
                FAR_BGR
            } else if inside(&areas[2], x, y) {
                PROBE_BGR
            } else {
                [128, 128, 128]
            };
            bgra.extend_from_slice(&[c[0], c[1], c[2], 255]);
        }
    }
    Image {
        width: W,
        height: H,
        bgra,
    }
}

#[test]
fn panels_probe_reports_each_probe_on_a_synthetic_frame() {
    let table = probe_table();
    let entry = &table.entries[0];
    let frame = probe_frame(&table);
    // `qt-screen`'s own counts for the same copies, the reference.
    let copies: Vec<Option<ProbeStats>> = entry
        .probes
        .iter()
        .map(|p| Some(ProbeStats::of_frame(&frame, p.rect)))
        .collect();
    let check = |lines: &[Value], overlay: &Rect, visible: bool| {
        let want = entry.counts(overlay, visible, &copies);
        let counts = &lines[3]["counts"];
        assert_eq!(lines[3]["entry"], "test-frame");
        assert_eq!(counts["read"], want.read);
        assert_eq!(counts["in_colour"], want.in_colour);
        assert_eq!(counts["within_spread"], want.within_spread);
        assert_eq!(counts["match"], want.matched);
        assert_eq!(counts["needed"], entry.min_match);
        assert_eq!(lines[4]["summary"]["client"], json!([W, H]));
        assert_eq!(lines[4]["summary"]["frame"], "synthetic");
    };
    let flags = |l: &Value| {
        (
            l["readable"].as_bool().unwrap(),
            l["in_colour"].as_bool().unwrap(),
            l["within_spread"].as_bool().unwrap(),
            l["match"].as_bool().unwrap(),
        )
    };

    // No overlay: every probe is read.
    let mut out = Vec::new();
    panels::probe(&frame, "synthetic", &table, None, &mut out).expect("probe");
    let lines = objects(&out);
    assert_eq!(
        lines.len(),
        5,
        "three probes, the entry, a summary: {lines:?}"
    );
    for (i, l) in lines[..3].iter().enumerate() {
        assert_eq!(l["entry"], "test-frame");
        assert_eq!(l["probe"], i);
    }
    assert_eq!(flags(&lines[0]), (true, true, false, false), "colour only");
    assert_eq!(flags(&lines[1]), (true, false, true, false), "spread only");
    assert_eq!(flags(&lines[2]), (true, true, true, true), "a match");
    assert_eq!(lines[2]["mean_bgr"], json!([60.0, 120.0, 180.0]));
    assert_eq!(lines[2]["luma_std"], json!(0.0));
    assert!(lines[0]["luma_std"].as_f64().unwrap() > 6.0);
    assert_eq!(lines[3]["read"], "open");
    let nowhere = Rect::new(0.0, 0.0, 0.0, 0.0);
    check(&lines, &nowhere, false);
    assert_eq!(lines[3]["counts"]["match"], 1);

    // The overlay over the matching probe: it is not read, and the entry
    // reads closed on the other two.
    let overlay = Rect::new(0.45, 0.25, 1.0, 1.0);
    let mut out = Vec::new();
    panels::probe(&frame, "synthetic", &table, Some(&overlay), &mut out).expect("probe");
    let lines = objects(&out);
    assert_eq!(lines.len(), 5);
    assert_eq!(flags(&lines[2]), (false, false, false, false));
    assert!(lines[2]["mean_bgr"].is_null() && lines[2]["luma_std"].is_null());
    assert_eq!(flags(&lines[0]), (true, true, false, false));
    assert_eq!(lines[3]["read"], "closed");
    check(&lines, &overlay, true);
    assert_eq!(lines[3]["counts"]["read"], 2);

    // The binary reads a grab's file and the overlay's four fractions; the
    // table it runs is whatever `qt-screen` ships, so only the summary is
    // checked here.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("open.bmp");
    panels::write_frame(&path, &frame).unwrap();
    let o = Command::new(BIN)
        .args(["panels", "probe", "--frame"])
        .arg(&path)
        .args(["--overlay", "0.78,0.30,1.0,0.97"])
        .output()
        .unwrap();
    assert_eq!(
        o.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
    let lines = objects(&o.stdout);
    assert_eq!(lines.last().unwrap()["summary"]["client"], json!([W, H]));
    let o = Command::new(BIN)
        .args(["panels", "probe", "--frame"])
        .arg(&path)
        .args(["--overlay", "0.78,0.30"])
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(64), "four fractions");
    let png = dir.path().join("open.png");
    fs::write(&png, "not a frame").unwrap();
    let o = Command::new(BIN)
        .args(["panels", "probe", "--frame"])
        .arg(&png)
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(1), "neither a grab nor a JPEG");
    let o = Command::new(BIN)
        .args(["panels", "probe", "--frame"])
        .arg(dir.path().join("missing.bmp"))
        .output()
        .unwrap();
    assert_eq!(
        o.status.code(),
        Some(1),
        "a missing frame is an error, never 2"
    );
}
