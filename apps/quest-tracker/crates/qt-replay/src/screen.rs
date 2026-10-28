//! `quest-tracker-replay screen`: the screen reader's trackers over a kept
//! capture of a research session, without the game or a screen.
//!
//! A live folder's `i3/frames.jsonl` indexes the bands the research capture
//! copied (`top`, `bottom`), each frame with its band's `rect` and the
//! game's `client` rect in screen pixels; `i3/ocr.jsonl` caches the OCR of
//! some of those frames, boxes in the band image's own pixels
//! (`docs/quest-tracker/outside-instruments.md` § "Instrument 3"). Each
//! region the demand reads is observed only from the frames of the smallest
//! band whose rect contains it, and each such frame becomes one
//! [`Observation`] the way the reader makes one:
//!
//! - a frame the capture skipped (minimized, no client rect), a frame taken
//!   with the game not in front, and a black copy are hidden reads, as the
//!   reader would have them;
//! - any other frame with cached OCR is a visible read of the lines whose
//!   box lies wholly inside the region;
//! - any other frame with no cached OCR is skipped, never counted absent.
//!
//! The reader copies only the region, so a line across the region's edge
//! reaches its OCR clipped, or not at all. The cache holds the OCR of a
//! wider band, and a cached line's text cannot be cut at a pixel edge, so
//! the replay drops such a line rather than read it whole, and counts it in
//! the region's `straddling` summary field.
//!
//! The dialogue window's reads go through the reader's [`DialogueGate`]
//! first. While it is shut, a read is [`HEADING_STRIP`] alone, which the
//! reader copies by itself: the replay keeps the cached lines whose box lies
//! wholly inside the strip, and counts the ones across its edge as the
//! dialogue window's `straddling`. A strip read that opens the gate is
//! followed by the whole read of the same frame, as the reader reads the
//! whole region in the same tick.
//!
//! The reads go through [`Trackers`] in time order, on UTC milliseconds from
//! the index. One JSON object per event goes to the output, then a summary
//! line of counts. OCR text is never printed, and nothing is written.

use std::collections::{BTreeMap, HashMap};
use std::ffi::OsString;
use std::fs;
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};

use qt_screen::{
    Demand, DialogueGate, DialogueRead, Hidden, Millis, Observation, PixelLine, PixelRect, Rect,
    Region, RemarkDemand, ScreenEvent, Trackers, BLACK_LUMA_MEAN, BLACK_LUMA_STD, HEADING_STRIP,
};
use serde_json::{json, Value};

use crate::{load_data, Failure};

pub const USAGE: &str = "\
quest-tracker-replay screen: the screen reader's trackers over a kept capture.

Usage: quest-tracker-replay screen --live FOLDER [--bar NAME]... [--dialogue NPC]...
                                   [--class CLASS [--data PATH]]

Reads FOLDER/i3/frames.jsonl and FOLDER/i3/ocr.jsonl (a research session's
capture and its OCR cache) and prints one JSON object per event, then a
summary line. A frame with no cached OCR is skipped. A read keeps the cached
lines whose box lies wholly inside the region (the heading strip, for a shut
read of the dialogue window); the summary counts each line dropped across
the edge as the region's `straddling`. No OCR text is printed and nothing
is written.

Options:
  --live FOLDER     the kept live folder (required)
  --bar NAME        a boss whose bar going is `bar_gone`; may be repeated
  --dialogue NPC    an NPC whose QUEST COMPLETE banner is `quest_complete`;
                    may be repeated
  --class CLASS     read `kill_remark` for every boss the quest data has a
                    kill-remark text of CLASS for
  --data PATH       quest data file for --class (default: the data/quests.json
                    built in)
  -h, --help        this text

Exit codes: 0 done, 1 error (a missing or unreadable index), 64 usage error.
";

#[derive(Debug)]
pub struct ScreenArgs {
    pub live: PathBuf,
    pub bars: Vec<String>,
    pub dialogues: Vec<String>,
    pub class: Option<String>,
    pub data: Option<PathBuf>,
}

/// Parses `screen`'s arguments (the command word already taken). `Ok(None)`
/// means `--help`.
pub fn parse_args(argv: impl IntoIterator<Item = OsString>) -> Result<Option<ScreenArgs>, String> {
    let mut live = None;
    let mut bars = Vec::new();
    let mut dialogues = Vec::new();
    let mut class = None;
    let mut data = None;
    let mut rest = argv.into_iter();
    while let Some(arg) = rest.next() {
        let text = arg.to_str().map(str::to_string);
        let (flag, inline) = match text.as_deref() {
            Some(t) if t.starts_with("--") => match t.split_once('=') {
                Some((f, v)) => (t[..f.len()].to_string(), Some(OsString::from(v))),
                None => (t.to_string(), None),
            },
            Some(t) if t.starts_with('-') && t.len() > 1 => (t.to_string(), None),
            _ => return Err(format!("unexpected argument {}", arg.to_string_lossy())),
        };
        let mut value = |name: &str| -> Result<OsString, String> {
            inline
                .clone()
                .or_else(|| rest.next())
                .ok_or_else(|| format!("{name} needs a value"))
        };
        let mut name = |name: &str| -> Result<String, String> {
            let v = value(name)?;
            let v = v
                .to_str()
                .ok_or_else(|| format!("{name} is not valid UTF-8"))?
                .trim()
                .to_string();
            if v.is_empty() {
                return Err(format!("{name} needs a value"));
            }
            Ok(v)
        };
        match flag.as_str() {
            "-h" | "--help" => return Ok(None),
            "--live" => live = Some(PathBuf::from(value("--live")?)),
            "--bar" => bars.push(name("--bar")?),
            "--dialogue" => dialogues.push(name("--dialogue")?),
            "--class" => class = Some(name("--class")?),
            "--data" => data = Some(PathBuf::from(value("--data")?)),
            other => return Err(format!("unknown option {other}")),
        }
    }
    let live = live.ok_or("screen needs --live FOLDER")?;
    if data.is_some() && class.is_none() {
        return Err("--data applies to `screen` with --class only".to_string());
    }
    if bars.is_empty() && dialogues.is_empty() && class.is_none() {
        return Err("screen needs at least one --bar, --dialogue or --class".to_string());
    }
    Ok(Some(ScreenArgs {
        live,
        bars,
        dialogues,
        class,
        data,
    }))
}

// -- the indexes ---------------------------------------------------------------

/// One `frames.jsonl` line.
struct Frame {
    at: Millis,
    band: String,
    /// The image's path key, or `None` for a frame the capture skipped.
    key: Option<String>,
    rect: Option<PixelRect>,
    client: Option<PixelRect>,
    foreground: bool,
    black: bool,
    minimized: bool,
}

fn read_index(path: &Path) -> Result<String, Failure> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(text),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Err(Failure::Error(format!(
            "no capture index at {}",
            path.display()
        ))),
        Err(e) => Err(Failure::Error(format!("{}: {e}", path.display()))),
    }
}

/// Non-empty lines of a JSONL file as objects. A line that does not parse is
/// an error naming the file and line, never its content.
fn jsonl(path: &Path, text: &str) -> Result<Vec<Value>, Failure> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut out = Vec::new();
    for (n, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let v: Value = serde_json::from_str(line).map_err(|_| {
            Failure::Error(format!(
                "{} line {}: not a JSON object",
                path.display(),
                n + 1
            ))
        })?;
        if !v.is_object() {
            return Err(Failure::Error(format!(
                "{} line {}: not a JSON object",
                path.display(),
                n + 1
            )));
        }
        out.push(v);
    }
    Ok(out)
}

/// A path as the OCR cache keys it: absolute, `.` dropped, `/` separators,
/// lowercase (Windows paths are case-insensitive).
fn path_key(path: &Path) -> String {
    let abs = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let mut clean = PathBuf::new();
    for c in abs.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                clean.pop();
            }
            other => clean.push(other.as_os_str()),
        }
    }
    clean.to_string_lossy().replace('\\', "/").to_lowercase()
}

fn pixel_rect(v: &Value) -> Option<PixelRect> {
    let a = v.as_array()?;
    if a.len() != 4 {
        return None;
    }
    let n: Vec<i32> = a
        .iter()
        .map(|x| x.as_f64().map(|f| f.round() as i32))
        .collect::<Option<_>>()?;
    Some(PixelRect::new(n[0], n[1], n[2], n[3]))
}

fn read_frames(i3: &Path) -> Result<Vec<Frame>, Failure> {
    let path = i3.join("frames.jsonl");
    let text = read_index(&path)?;
    let mut frames = Vec::new();
    for (n, v) in jsonl(&path, &text)?.into_iter().enumerate() {
        let bad =
            |what: &str| Failure::Error(format!("{} record {}: {what}", path.display(), n + 1));
        let at = v["utc"]
            .as_str()
            .and_then(parse_utc)
            .ok_or_else(|| bad("no readable utc"))?;
        let band = v["band"]
            .as_str()
            .ok_or_else(|| bad("no band"))?
            .to_string();
        let key = v["path"].as_str().map(|p| path_key(&i3.join(p)));
        let luma = |k: &str| v[k].as_f64();
        let black = matches!((luma("luma_mean"), luma("luma_std")),
            (Some(m), Some(s)) if m < BLACK_LUMA_MEAN && s < BLACK_LUMA_STD);
        frames.push(Frame {
            at,
            band,
            key,
            rect: pixel_rect(&v["rect"]),
            client: pixel_rect(&v["client"]),
            // An index without the field was taken in front.
            foreground: v["foreground"].as_bool().unwrap_or(true),
            black,
            minimized: v["skipped"].as_str() == Some("minimized"),
        });
    }
    frames.sort_by_key(|f| f.at);
    Ok(frames)
}

/// `ocr.jsonl` as {path key: lines}, as the instruments read it: a later
/// record for a path wins, and a record with an `error` is left out.
fn read_ocr(i3: &Path) -> Result<HashMap<String, Vec<PixelLine>>, Failure> {
    let path = i3.join("ocr.jsonl");
    let text = read_index(&path)?;
    let mut out = HashMap::new();
    for v in jsonl(&path, &text)? {
        if v.get("error").is_some_and(|e| !e.is_null()) {
            continue;
        }
        let Some(p) = v["path"].as_str() else {
            continue;
        };
        let p = Path::new(p);
        let key = if p.is_absolute() {
            path_key(p)
        } else {
            path_key(&i3.join(p))
        };
        let lines = v["lines"]
            .as_array()
            .map(|lines| {
                lines
                    .iter()
                    .filter_map(|l| {
                        Some(PixelLine::new(
                            l["text"].as_str().unwrap_or(""),
                            pixel_rect(&l["box"])?,
                        ))
                    })
                    .collect()
            })
            .unwrap_or_default();
        out.insert(key, lines);
    }
    Ok(out)
}

// -- the replay ------------------------------------------------------------------

/// What happened to one region's frames, for the summary.
#[derive(Default)]
struct RegionCounts {
    band: Option<String>,
    visible: u64,
    hidden: u64,
    no_ocr: u64,
    /// Cached lines a read dropped because their box crosses the edge of
    /// what it copies (the region, or the heading strip).
    straddling: u64,
}

/// For each region, the band whose frames observe it: the smallest band
/// whose rect, on its first frame with one, contains the region.
fn source_bands(frames: &[Frame], regions: &[Region]) -> BTreeMap<Region, String> {
    let mut first: BTreeMap<&str, (PixelRect, PixelRect)> = BTreeMap::new();
    for f in frames {
        if let (Some(rect), Some(client)) = (f.rect, f.client) {
            first.entry(f.band.as_str()).or_insert((rect, client));
        }
    }
    let mut out = BTreeMap::new();
    for &region in regions {
        let best = first
            .iter()
            .filter(|(_, (rect, client))| contains(rect, &region.pixels(*client)))
            .min_by_key(|(_, (rect, _))| rect.width() as i64 * rect.height() as i64);
        if let Some((band, _)) = best {
            out.insert(region, band.to_string());
        }
    }
    out
}

fn contains(outer: &PixelRect, inner: &PixelRect) -> bool {
    !outer.is_empty()
        && inner.left >= outer.left
        && inner.top >= outer.top
        && inner.right <= outer.right
        && inner.bottom <= outer.bottom
}

/// The read of `area` alone from `obs`, a read of the whole band: the lines
/// whose box lies wholly inside it. Each line across its edge is dropped and
/// counted in `straddling`; a line wholly outside it is not counted.
fn crop(obs: &Observation, area: Rect, straddling: &mut u64) -> Observation {
    let mut out = obs.clone();
    out.lines.retain(|l| {
        if area.contains(&l.bbox) {
            return true;
        }
        if area.overlaps(&l.bbox) {
            *straddling += 1;
        }
        false
    });
    out
}

/// The frame's visible read of the whole band, in client fractions, or the
/// hidden read of `region`; `None` when it gives none. A visible read is
/// cropped by its caller to what the reader copies.
fn observe(
    region: Region,
    frame: &Frame,
    ocr: &HashMap<String, Vec<PixelLine>>,
    counts: &mut RegionCounts,
) -> Option<Observation> {
    let hidden = if frame.key.is_none() {
        Some(if frame.minimized {
            Hidden::Minimized
        } else {
            Hidden::CaptureFailed
        })
    } else if !frame.foreground {
        Some(Hidden::NotForeground)
    } else if frame.black {
        Some(Hidden::Black)
    } else {
        None
    };
    if let Some(why) = hidden {
        counts.hidden += 1;
        return Some(Observation::hidden(region, frame.at, why));
    }
    let (Some(rect), Some(client)) = (frame.rect, frame.client) else {
        counts.no_ocr += 1;
        return None;
    };
    if !contains(&rect, &region.pixels(client)) {
        counts.no_ocr += 1;
        return None;
    }
    let Some(lines) = frame.key.as_ref().and_then(|k| ocr.get(k)) else {
        counts.no_ocr += 1;
        return None;
    };
    counts.visible += 1;
    Some(Observation::from_ocr(
        region,
        frame.at,
        client,
        rect,
        lines.clone(),
    ))
}

/// The reads the reader would make of the dialogue window from one frame's
/// band read `band`, through `gate`: the strip alone while it is shut, then
/// the whole region too when that strip read opens it; the whole region
/// while it is open. A hidden read is the frame's only read.
fn gated(
    gate: &mut DialogueGate,
    npcs: &[String],
    band: Observation,
    straddling: &mut u64,
) -> Vec<Observation> {
    if !band.is_visible() {
        return vec![band];
    }
    let mut reads = Vec::new();
    if !gate.is_open(band.at) {
        let strip = crop(&band, HEADING_STRIP, straddling);
        let open = gate.observe(npcs, &strip, DialogueRead::Strip);
        reads.push(strip);
        if !open {
            return reads;
        }
    }
    let whole = crop(&band, Region::DialogueWindow.rect(), straddling);
    gate.observe(npcs, &whole, DialogueRead::Full);
    reads.push(whole);
    reads
}

fn demand(args: &ScreenArgs) -> Result<Demand, Failure> {
    let mut demand = Demand {
        bars: args.bars.clone(),
        dialogues: args.dialogues.clone(),
        remarks: Vec::new(),
        keys: Vec::new(),
    };
    if let Some(class) = &args.class {
        let data = load_data(args.data.as_deref())?;
        let mut by_boss: BTreeMap<&str, Vec<String>> = BTreeMap::new();
        for r in data
            .remarks
            .iter()
            .filter(|r| r.class.eq_ignore_ascii_case(class))
        {
            // A row with no boss is a key's remark, which `--class` does
            // not read.
            if let Some(boss) = &r.boss {
                by_boss.entry(boss).or_default().push(r.text.clone());
            }
        }
        if by_boss.is_empty() {
            return Err(Failure::Error(format!(
                "the quest data has no kill-remark text for class {class:?}"
            )));
        }
        demand.remarks = by_boss
            .into_iter()
            .map(|(boss, texts)| RemarkDemand {
                boss: boss.to_string(),
                texts,
            })
            .collect();
    }
    Ok(demand)
}

fn event_json(event: &ScreenEvent) -> Value {
    let mut v = json!({
        "event": event.kind(),
        "target": event.target(),
        "at": format_utc(event.at()),
    });
    if let ScreenEvent::BarGone { last_present, .. } = event {
        v["last_present"] = json!(format_utc(*last_present));
    }
    v
}

pub fn run(args: &ScreenArgs, out: &mut impl Write) -> Result<(), Failure> {
    let i3 = args.live.join("i3");
    let frames = read_frames(&i3)?;
    let ocr = read_ocr(&i3)?;
    let demand = demand(args)?;
    let regions = demand.regions();
    let sources = source_bands(&frames, &regions);
    let mut counts: BTreeMap<Region, RegionCounts> = regions
        .iter()
        .map(|&r| {
            let c = RegionCounts {
                band: sources.get(&r).cloned(),
                ..RegionCounts::default()
            };
            (r, c)
        })
        .collect();
    let mut trackers = Trackers::for_demand(&demand);
    let mut gate = DialogueGate::new();
    let mut events = 0u64;
    for frame in &frames {
        for &region in &regions {
            if sources.get(&region) != Some(&frame.band) {
                continue;
            }
            let c = counts.get_mut(&region).expect("every region counted");
            let Some(obs) = observe(region, frame, &ocr, c) else {
                continue;
            };
            let reads = if region == Region::DialogueWindow {
                gated(&mut gate, &demand.dialogues, obs, &mut c.straddling)
            } else if obs.is_visible() {
                vec![crop(&obs, region.rect(), &mut c.straddling)]
            } else {
                vec![obs]
            };
            for obs in &reads {
                for event in trackers.observe(obs) {
                    events += 1;
                    writeln!(out, "{}", event_json(&event))?;
                }
            }
        }
    }
    let bar = trackers.take_counts();
    let gate = gate.take_counts();
    let regions_json: serde_json::Map<String, Value> = counts
        .iter()
        .map(|(r, c)| {
            (
                r.name().to_string(),
                json!({
                    "band": c.band,
                    "visible": c.visible,
                    "hidden": c.hidden,
                    "no_ocr": c.no_ocr,
                    "straddling": c.straddling,
                }),
            )
        })
        .collect();
    let summary = json!({
        "summary": {
            "frames": frames.len(),
            "ocr_cached": ocr.len(),
            "regions": regions_json,
            "bar": {"present": bar.present, "absent": bar.absent, "covered": bar.covered},
            "dialogue_gate": {"full": gate.full, "skipped": gate.skipped},
            "events": events,
        }
    });
    writeln!(out, "{summary}")?;
    Ok(())
}

// -- UTC -------------------------------------------------------------------------

/// Days from 1970-01-01 to the civil date (proleptic Gregorian).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

/// `YYYY-MM-DDTHH:MM:SS[.fff]Z` as milliseconds since the Unix epoch.
pub fn parse_utc(text: &str) -> Option<Millis> {
    let t = text.strip_suffix('Z')?;
    let (date, time) = t.split_once('T')?;
    let mut d = date.splitn(3, '-').map(|p| p.parse::<i64>().ok());
    let (y, mo, da) = (d.next()??, d.next()??, d.next()??);
    let (hms, frac) = match time.split_once('.') {
        Some((hms, frac)) => (hms, frac),
        None => (time, ""),
    };
    let mut h = hms.splitn(3, ':').map(|p| p.parse::<i64>().ok());
    let (hh, mm, ss) = (h.next()??, h.next()??, h.next()??);
    if !(1..=12).contains(&mo) || !(1..=31).contains(&da) || hh > 23 || mm > 59 || ss > 60 {
        return None;
    }
    if !frac.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let ms = format!("{frac:0<3}")[..3].parse::<i64>().ok()?;
    let days = days_from_civil(y, mo, da);
    Some(((days * 24 + hh) * 60 + mm) * 60_000 + ss * 1000 + ms)
}

/// Milliseconds since the Unix epoch as `YYYY-MM-DDTHH:MM:SS.mmmZ`.
pub fn format_utc(ms: Millis) -> String {
    let days = ms.div_euclid(86_400_000);
    let rem = ms.rem_euclid(86_400_000);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.{:03}Z",
        rem / 3_600_000,
        rem / 60_000 % 60,
        rem / 1000 % 60,
        rem % 1000
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utc_round_trips() {
        let at = parse_utc("2026-10-26T20:19:45.913Z").unwrap();
        assert_eq!(format_utc(at), "2026-10-26T20:19:45.913Z");
        assert_eq!(parse_utc("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(
            parse_utc("2000-03-01T00:00:01.5Z"),
            Some(951_868_800_000 + 1_500)
        );
        assert_eq!(format_utc(951_868_801_500), "2000-03-01T00:00:01.500Z");
        assert_eq!(parse_utc("2026-10-26 20:19:45Z"), None);
        assert_eq!(parse_utc("2026-13-26T20:19:45Z"), None);
        assert_eq!(parse_utc("2026-10-26T20:19:45.9x1Z"), None);
    }

    #[test]
    fn a_remark_prints_as_a_kill_remark_does_with_its_key() {
        let kill = ScreenEvent::KillRemark {
            boss: "The Bloated Miller".into(),
            at: 0,
        };
        let remark = ScreenEvent::Remark {
            key: "SinTreeSeenComment".into(),
            at: 0,
        };
        assert_eq!(
            event_json(&kill),
            json!({"event": "kill_remark", "target": "The Bloated Miller", "at": "1970-01-01T00:00:00.000Z"})
        );
        assert_eq!(
            event_json(&remark),
            json!({"event": "remark", "target": "SinTreeSeenComment", "at": "1970-01-01T00:00:00.000Z"})
        );
    }

    #[test]
    fn path_keys_ignore_case_separators_and_dots() {
        let a = path_key(Path::new("C:/Live/i3/./frames/top/A.jpg"));
        let b = path_key(Path::new("c:\\live\\i3\\frames\\top\\a.jpg"));
        assert_eq!(a, b);
    }

    #[test]
    fn the_smallest_containing_band_observes_a_region() {
        let client = PixelRect::new(0, 0, 2560, 1440);
        let frame = |band: &str, rect: PixelRect| Frame {
            at: 0,
            band: band.into(),
            key: Some(String::new()),
            rect: Some(rect),
            client: Some(client),
            foreground: true,
            black: false,
            minimized: false,
        };
        let frames = vec![
            frame("top", PixelRect::new(640, 0, 1920, 202)),
            frame("bottom", client),
        ];
        let all = Region::ALL.to_vec();
        let s = source_bands(&frames, &all);
        assert_eq!(s[&Region::BossBar], "top");
        assert_eq!(s[&Region::VoicedLines], "bottom");
        assert_eq!(s[&Region::DialogueWindow], "bottom");
        // Live 6's bottom band starts at y 0.35: no band holds the window.
        let frames = vec![
            frame("top", PixelRect::new(640, 0, 1920, 202)),
            frame("bottom", PixelRect::new(0, 504, 2560, 1440)),
        ];
        let s = source_bands(&frames, &all);
        assert_eq!(s[&Region::VoicedLines], "bottom");
        assert!(!s.contains_key(&Region::DialogueWindow));
    }
}
