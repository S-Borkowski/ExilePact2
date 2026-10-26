//! `quest-tracker-replay screen` end to end, over synthetic live folders in a
//! temporary directory: invented frames and OCR lines, no image, no real
//! capture. Counts and run lengths come from `qt-screen`'s public constants,
//! which the replay over the kept captures settles.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use qt_screen::{GONE_RUN, MIN_PRESENT};
use serde_json::{json, Value};

const BIN: &str = env!("CARGO_BIN_EXE_quest-tracker-replay");
const MILLER: &str = "The Bloated Miller";
const RENLY: &str = "Renly";
/// A sentinel in every OCR line that is not a name: it must never print.
const SENTINEL: &str = "zqxsentinel";

fn run(args: &[&str]) -> Output {
    Command::new(BIN)
        .args(args)
        .output()
        .expect("the replay binary runs")
}

fn objects(o: &Output) -> Vec<Value> {
    String::from_utf8(o.stdout.clone())
        .expect("UTF-8 output")
        .lines()
        .map(|l| serde_json::from_str(l).expect("one JSON object per line"))
        .collect()
}

/// `2026-10-26T20:00:00.000Z` plus `ms`.
fn utc(ms: i64) -> String {
    let s = ms / 1000;
    format!(
        "2026-10-26T20:{:02}:{:02}.{:03}Z",
        s / 60,
        s % 60,
        ms % 1000
    )
}

struct Live {
    frames: Vec<Value>,
    ocr: Vec<Value>,
    /// What [`session`] expects: the bar's last present read and the
    /// banner's first, in ms.
    bar_last_present: i64,
    banner_at: i64,
}

impl Live {
    fn new() -> Self {
        Live {
            frames: Vec::new(),
            ocr: Vec::new(),
            bar_last_present: 0,
            banner_at: 0,
        }
    }

    /// One frame of `band` at `ms`; `lines` are OCR'd lines (text, box in
    /// the band's pixels), or `None` for a frame with nothing cached.
    fn frame(&mut self, band: &str, ms: i64, lines: Option<Vec<(String, [i32; 4])>>) {
        let rect = match band {
            "top" => json!([640, 0, 1920, 202]),
            _ => json!([0, 0, 2560, 1440]),
        };
        let stamp = utc(ms).replace(['-', ':'], "");
        let path = format!("frames/{band}/{stamp}.jpg");
        self.frames.push(json!({
            "utc": utc(ms), "band": band, "path": path, "rect": rect,
            "client": [0, 0, 2560, 1440], "foreground": true,
            "luma_mean": 20.0, "luma_std": 15.0,
        }));
        if let Some(lines) = lines {
            let lines: Vec<Value> = lines
                .into_iter()
                .map(|(text, b)| json!({"text": text, "box": b}))
                .collect();
            // The OCR cache names its images by their own path, here
            // relative to `i3`.
            self.ocr
                .push(json!({"path": path.replace('/', "\\"), "scale": 2, "lines": lines}));
        }
    }

    fn write(&self, dir: &Path, with_frames: bool, with_ocr: bool) {
        let i3 = dir.join("i3");
        fs::create_dir_all(&i3).unwrap();
        let jsonl = |v: &[Value]| v.iter().map(|x| format!("{x}\n")).collect::<String>();
        if with_frames {
            fs::write(i3.join("frames.jsonl"), jsonl(&self.frames)).unwrap();
        }
        if with_ocr {
            fs::write(i3.join("ocr.jsonl"), jsonl(&self.ocr)).unwrap();
        }
    }
}

/// The Miller's name where Live 8 read it, in the top band's pixels.
fn miller() -> (String, [i32; 4]) {
    ("THE BLOATED MILLER".into(), [527, 68, 753, 86])
}

/// A scene label left of the name's column, carrying the sentinel.
fn scene_text() -> (String, [i32; 4]) {
    (format!("CHECKPOINT {SENTINEL}"), [128, 60, 256, 80])
}

/// A synthetic session: the Miller's bar up, a stretch of frames with no
/// OCR cached, the bar up again, then gone; and Renly's window with the
/// QUEST COMPLETE banner on the full-client band.
fn session() -> Live {
    let mut live = Live::new();
    let mut ms = 0;
    let top = |live: &mut Live, ms: &mut i64, lines| {
        live.frame("top", *ms, lines);
        *ms += 500;
    };
    for _ in 0..MIN_PRESENT + 2 {
        top(&mut live, &mut ms, Some(vec![miller()]));
    }
    // Uncached frames are skipped, not absent: this gap is longer than the
    // run and must not end the bar.
    for _ in 0..GONE_RUN * 3 {
        top(&mut live, &mut ms, None);
    }
    top(&mut live, &mut ms, Some(vec![miller()]));
    let last_present = ms - 500;
    for _ in 0..GONE_RUN + 4 {
        top(&mut live, &mut ms, Some(vec![scene_text()]));
    }
    live.bar_last_present = last_present;

    // Renly's window: the heading, a body line, then the banner (boxes in
    // the client's pixels; this band is the whole client).
    let heading = ("Renly".to_string(), [1203, 374, 1357, 403]);
    let body = (
        format!("Words of thanks {SENTINEL}"),
        [1075, 734, 1690, 763],
    );
    let banner = ("QUEST COMPLETE".to_string(), [1144, 1117, 1416, 1152]);
    let start = ms;
    live.frame("bottom", start, Some(vec![heading.clone(), body.clone()]));
    live.frame("bottom", start + 1_000, Some(vec![heading.clone(), body]));
    live.frame(
        "bottom",
        start + 2_000,
        Some(vec![heading.clone(), banner.clone()]),
    );
    live.frame("bottom", start + 3_000, Some(vec![banner]));
    live.banner_at = start + 2_000;
    live
}

#[test]
fn screen_replay_reads_a_synthetic_live_folder() {
    let live = session();
    let dir = tempfile::tempdir().unwrap();
    live.write(dir.path(), true, true);
    let folder = dir.path().to_str().unwrap();
    let o = run(&[
        "screen",
        "--live",
        folder,
        "--bar",
        MILLER,
        "--dialogue",
        RENLY,
    ]);
    assert!(
        o.status.success(),
        "exit {:?}: {}",
        o.status.code(),
        String::from_utf8_lossy(&o.stderr)
    );
    let text = String::from_utf8_lossy(&o.stdout).to_string();
    assert!(!text.contains(SENTINEL), "OCR text printed");
    assert!(!String::from_utf8_lossy(&o.stderr).contains(SENTINEL));

    let objs = objects(&o);
    let (summary, events) = objs.split_last().expect("a summary line");
    assert_eq!(
        events.to_vec(),
        vec![
            json!({
                "event": "bar_gone",
                "target": MILLER,
                "at": utc(live.bar_last_present + 500 * GONE_RUN as i64),
                "last_present": utc(live.bar_last_present),
            }),
            json!({"event": "quest_complete", "target": RENLY, "at": utc(live.banner_at)}),
        ]
    );
    let s = &summary["summary"];
    assert_eq!(s["events"], 2);
    assert_eq!(s["regions"]["boss bar"]["band"], "top");
    assert_eq!(s["regions"]["boss bar"]["no_ocr"], GONE_RUN as u64 * 3);
    assert_eq!(s["regions"]["dialogue window"]["band"], "bottom");
    assert_eq!(s["regions"]["dialogue window"]["visible"], 4);
    assert!(s.get("regions").unwrap().get("voiced lines").is_none());

    // Nothing was written into the folder.
    let names: Vec<_> = fs::read_dir(dir.path().join("i3"))
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(names.len(), 2, "{names:?}");

    // A frame taken with the game not in front is covered: the run after it
    // does not end the bar.
    let mut covered = Live::new();
    for n in 0..4 {
        covered.frame("top", n * 500, Some(vec![miller()]));
    }
    covered.frame("top", 2_000, Some(vec![]));
    covered.frames.last_mut().unwrap()["foreground"] = json!(false);
    for n in 0..GONE_RUN as i64 + 4 {
        covered.frame("top", 2_500 + n * 500, Some(vec![]));
    }
    let dir2 = tempfile::tempdir().unwrap();
    covered.write(dir2.path(), true, true);
    let o = run(&[
        "screen",
        "--live",
        dir2.path().to_str().unwrap(),
        "--bar",
        MILLER,
    ]);
    let objs = objects(&o);
    assert!(o.status.success());
    assert_eq!(objs.len(), 1, "{objs:?}");
    assert_eq!(objs[0]["summary"]["bar"]["covered"], 1);
}

#[test]
fn screen_replay_reads_kill_remarks_for_a_class() {
    const BOSS: &str = "Beira of the Rotten Pack";
    // An invented remark text, in a copy of the minimal quest data.
    let dir = tempfile::tempdir().unwrap();
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/quests_min.json");
    let mut data: Value = serde_json::from_str(&fs::read_to_string(fixture).unwrap()).unwrap();
    data["remarks"] = json!([{
        "class": "Witch", "boss": BOSS, "key": "kill",
        "text": "Invented words over a fallen beast.",
        "source_url": "https://poe2db.tw/us/Witch", "fetched": "2026-10-26",
    }]);
    let data_path = dir.path().join("quests.json");
    fs::write(&data_path, data.to_string()).unwrap();

    // The list of voiced lines on the whole-client band: a shout, then the
    // remark under an invented character's prefix.
    let shout = (
        format!("Kestrella: {SENTINEL} for the camp"),
        [77, 994, 768, 1022],
    );
    let remark = (
        "Kestrella: Invented words over a fallen beast.".to_string(),
        [77, 1022, 768, 1051],
    );
    let mut live = Live::new();
    for n in 0..6 {
        live.frame("bottom", n * 500, Some(vec![shout.clone()]));
    }
    for n in 6..14 {
        live.frame("bottom", n * 500, Some(vec![shout.clone(), remark.clone()]));
    }
    let folder = dir.path().join("live");
    live.write(&folder, true, true);
    let folder = folder.to_str().unwrap();
    let data_arg = data_path.to_str().unwrap();

    let o = run(&[
        "screen", "--live", folder, "--class", "witch", "--data", data_arg,
    ]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert!(!String::from_utf8_lossy(&o.stdout).contains(SENTINEL));
    let objs = objects(&o);
    assert_eq!(
        objs[..objs.len() - 1].to_vec(),
        vec![json!({"event": "kill_remark", "target": BOSS, "at": utc(3_000)})]
    );
    assert_eq!(
        objs.last().unwrap()["summary"]["regions"]["voiced lines"]["visible"],
        14
    );

    // A class the data has no text for is refused.
    let o = run(&[
        "screen", "--live", folder, "--class", "Ranger", "--data", data_arg,
    ]);
    assert_eq!(o.status.code(), Some(1));
}

#[test]
fn screen_replay_refuses_a_folder_without_its_index() {
    let live = session();
    for (with_frames, with_ocr, missing) in
        [(false, true, "frames.jsonl"), (true, false, "ocr.jsonl")]
    {
        let dir = tempfile::tempdir().unwrap();
        live.write(dir.path(), with_frames, with_ocr);
        let o = run(&[
            "screen",
            "--live",
            dir.path().to_str().unwrap(),
            "--bar",
            MILLER,
        ]);
        assert_eq!(o.status.code(), Some(1), "missing {missing}");
        let err = String::from_utf8_lossy(&o.stderr);
        assert!(err.contains(missing), "{err}");
        assert!(o.stdout.is_empty());
    }
    // And a usage error when it is given nothing to read.
    let dir = tempfile::tempdir().unwrap();
    let o = run(&["screen", "--live", dir.path().to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(64));
}
