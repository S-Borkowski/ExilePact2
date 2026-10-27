//! `quest-tracker-replay panels` over synthetic sessions: frames painted in
//! memory from numbers (a gradient world, a wandering bright square, a
//! panel's flat body and header), never a decoded file, a kept capture or
//! the screen.
//!
//! `qt-replay` is a binary, so the module is included here by path and
//! driven through its frame source with a painter; the refusal of a folder
//! without its index runs the binary itself.

use std::fs;
use std::path::Path;
use std::process::Command;

use qt_screen::{Image, Millis, PanelEntry, PanelProbe, PanelTable, Rect};
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
