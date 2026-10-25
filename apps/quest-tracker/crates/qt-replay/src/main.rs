//! `quest-tracker-replay`: runs the overlay's engine over a saved log
//! (`events`, `state`, `import`, `scan`), without the game.
//!
//! It is the overlay's own code path with the window taken away: the log is
//! read once from 0 to EOF through the tailer's whole-file mode
//! ([`qt_tailer::read_whole_file`], read-only, every line catch-up), each line
//! goes through [`qt_engine::Engine::feed_line`], and `import` writes through
//! [`qt_store::Store`]. The quest data is the `data/quests.json` compiled in,
//! or the file `--data` names.
//!
//! Privacy: nothing printed is chat, an unknown speaker or another player's
//! name. `events` prints a name-bearing event only for a character the
//! engine stored (own or unconfirmed) after the whole log, never a join or
//! leave line; `scan` prints counts per area and no character name at all.
//!
//! Exit codes: 0 done, 1 an error (unreadable data, a store refusal, a read
//! failure), 2 the log file does not exist, 64 a usage error.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::ffi::OsString;
use std::fs;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use qt_engine::{Engine, QuestData, Status};
use qt_logline::{parse_line, Allowlist, Event, EventKind, PointPool};
use qt_store::{Loaded, Store};
use qt_tailer::{read_whole_file, Position};
use serde_json::{json, Map, Value};

/// The shipped quest data, compiled in so the CLI runs the app's data.
const EMBEDDED_DATA: &str = include_str!("../../../data/quests.json");

const EXIT_ERROR: u8 = 1;
const EXIT_NO_LOG: u8 = 2;
const EXIT_USAGE: u8 = 64;

const USAGE: &str = "\
quest-tracker-replay: the quest tracker's engine over a saved Client.txt, without the game.

Usage: quest-tracker-replay <command> <log> [--data PATH] [--json] [--speakers NAMES]

Commands:
  events   one recognised event per line (never chat, unknown speakers or
           other players' names)
  state    the engine over the whole log, no store: per own character its
           class, level and quests by status per act
  import   `state`, then written to the store (QUEST_TRACKER_DIR, else
           %LOCALAPPDATA%\\ExilePact2\\quest-tracker) with the position at the
           end of the log; running it twice leaves the same files
  scan     per area code: display name, entries, reward lines, points lines,
           allowlisted speakers and system lines, as counts (no chat, no
           character names)

Options:
  --data PATH        quest data file (default: the data/quests.json built in)
  --json             JSON output (`events`: one object per line)
  --speakers NAMES   extra speakers to allowlist for `events` and `scan`,
                     comma-separated; may be repeated
  -h, --help         this text

The log is opened read-only. Exit codes: 0 done, 1 error, 2 no such log
file, 64 usage error.
";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Command {
    Events,
    State,
    Import,
    Scan,
}

#[derive(Debug)]
struct Args {
    command: Command,
    log: PathBuf,
    data: Option<PathBuf>,
    json: bool,
    speakers: Vec<String>,
}

enum Failure {
    Usage(String),
    NoLog(PathBuf),
    Error(String),
    /// Whoever reads the output closed it (`| head`): their choice, not an
    /// error.
    Closed,
}

impl From<io::Error> for Failure {
    fn from(e: io::Error) -> Failure {
        if e.kind() == io::ErrorKind::BrokenPipe {
            Failure::Closed
        } else {
            Failure::Error(format!("writing output: {e}"))
        }
    }
}

fn main() -> ExitCode {
    let argv: Vec<OsString> = std::env::args_os().skip(1).collect();
    let args = match parse_args(argv) {
        Ok(Some(args)) => args,
        Ok(None) => {
            print!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Err(message) => return fail(Failure::Usage(message)),
    };
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    let result = run(&args, &mut out).and_then(|()| out.flush().map_err(Failure::from));
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => fail(failure),
    }
}

fn fail(failure: Failure) -> ExitCode {
    match failure {
        Failure::Usage(message) => {
            eprintln!("quest-tracker-replay: {message}\n\n{USAGE}");
            ExitCode::from(EXIT_USAGE)
        }
        Failure::NoLog(path) => {
            eprintln!("quest-tracker-replay: no log file at {}", path.display());
            ExitCode::from(EXIT_NO_LOG)
        }
        Failure::Closed => ExitCode::SUCCESS,
        Failure::Error(message) => {
            eprintln!("quest-tracker-replay: {message}");
            ExitCode::from(EXIT_ERROR)
        }
    }
}

// -- arguments ----------------------------------------------------------------

/// Parses the arguments by hand (no CLI crate is pinned). `Ok(None)` means
/// `--help`.
fn parse_args(argv: Vec<OsString>) -> Result<Option<Args>, String> {
    let mut command = None;
    let mut log = None;
    let mut data = None;
    let mut json = false;
    let mut speakers = Vec::new();
    let mut rest = argv.into_iter();
    while let Some(arg) = rest.next() {
        let text = arg.to_str().map(str::to_string);
        // `--name=value` and `--name value` both work.
        let (flag, inline) = match text.as_deref() {
            Some(t) if t.starts_with("--") => match t.split_once('=') {
                Some((f, v)) => (Some(f.to_string()), Some(OsString::from(v))),
                None => (Some(t.to_string()), None),
            },
            Some(t) if t.starts_with('-') && t.len() > 1 => (Some(t.to_string()), None),
            _ => (None, None),
        };
        let mut value = |name: &str| -> Result<OsString, String> {
            inline
                .clone()
                .or_else(|| rest.next())
                .ok_or_else(|| format!("{name} needs a value"))
        };
        match flag.as_deref() {
            Some("-h") | Some("--help") => return Ok(None),
            Some("--json") if inline.is_none() => json = true,
            Some("--data") => data = Some(PathBuf::from(value("--data")?)),
            Some("--speakers") => {
                let names = value("--speakers")?;
                let names = names
                    .to_str()
                    .ok_or("--speakers is not valid UTF-8")?
                    .to_string();
                speakers.extend(
                    names
                        .split(',')
                        .map(str::trim)
                        .filter(|n| !n.is_empty())
                        .map(str::to_string),
                );
            }
            Some(other) => return Err(format!("unknown option {other}")),
            None if command.is_none() => {
                command = Some(match text.as_deref() {
                    Some("events") => Command::Events,
                    Some("state") => Command::State,
                    Some("import") => Command::Import,
                    Some("scan") => Command::Scan,
                    _ => return Err(format!("unknown command {}", arg.to_string_lossy())),
                });
            }
            None if log.is_none() => log = Some(PathBuf::from(arg)),
            None => return Err(format!("unexpected argument {}", arg.to_string_lossy())),
        }
    }
    let command = command.ok_or("no command given")?;
    let log = log.ok_or("no log file given")?;
    if !speakers.is_empty() && !matches!(command, Command::Events | Command::Scan) {
        return Err("--speakers applies to `events` and `scan` only".to_string());
    }
    Ok(Some(Args {
        command,
        log,
        data,
        json,
        speakers,
    }))
}

// -- shared -------------------------------------------------------------------

fn run(args: &Args, out: &mut impl Write) -> Result<(), Failure> {
    let data = Arc::new(load_data(args.data.as_deref())?);
    match args.command {
        Command::Events => events(args, data, out),
        Command::State => {
            let (engine, replayed) = replay(Engine::new(data), &args.log)?;
            print_state(&engine, &replayed, args.json, None, out)
        }
        Command::Import => import(args, data, out),
        Command::Scan => scan(args, &data, out),
    }
}

fn load_data(path: Option<&Path>) -> Result<QuestData, Failure> {
    match path {
        Some(path) => QuestData::load(path)
            .map_err(|e| Failure::Error(format!("quest data {}: {e}", path.display()))),
        None => QuestData::parse(EMBEDDED_DATA)
            .map_err(|e| Failure::Error(format!("built-in data/quests.json: {e}"))),
    }
}

/// Reads the whole log once, read-only, handing each line to `each`.
fn read_log(path: &Path, each: impl FnMut(qt_tailer::Line)) -> Result<Position, Failure> {
    let not_found = |e: &io::Error| e.kind() == io::ErrorKind::NotFound;
    match fs::metadata(path) {
        Ok(meta) if meta.is_file() => {}
        Ok(_) => return Err(Failure::Error(format!("{} is not a file", path.display()))),
        Err(e) if not_found(&e) => return Err(Failure::NoLog(path.to_path_buf())),
        Err(e) => return Err(Failure::Error(format!("{}: {e}", path.display()))),
    }
    read_whole_file(path, each).map_err(|e| {
        if not_found(&e) {
            Failure::NoLog(path.to_path_buf())
        } else {
            Failure::Error(format!("reading {}: {e}", path.display()))
        }
    })
}

/// What a whole-log replay read.
struct Replayed {
    position: Position,
    lines: u64,
}

/// Feeds every line of the log to `engine`, as catch-up.
fn replay(mut engine: Engine, log: &Path) -> Result<(Engine, Replayed), Failure> {
    let mut lines = 0u64;
    let position = read_log(log, |line| {
        lines += 1;
        engine.feed_line(&line.text, !line.catch_up);
    })?;
    Ok((engine, Replayed { position, lines }))
}

/// The parser allowlist of the data, plus `--speakers`.
fn allowlist(data: &QuestData, speakers: &[String]) -> Allowlist {
    let mut allow = data.allowlist();
    allow.add_npcs(speakers.iter().cloned());
    allow
}

// -- events -------------------------------------------------------------------

/// The character name an event carries, if any.
fn named(kind: &EventKind) -> Option<&str> {
    match kind {
        EventKind::LevelUp { name, .. }
        | EventKind::Slain { name }
        | EventKind::Received { name, .. }
        | EventKind::Lost { name, .. }
        | EventKind::Party { name, .. }
        | EventKind::CharacterNamed { name } => Some(name),
        _ => None,
    }
}

/// Two passes: the engine over the whole log decides which names are
/// stored characters, then the log is read again and printed. A join or
/// leave line always names someone else and is never printed.
fn events(args: &Args, data: Arc<QuestData>, out: &mut impl Write) -> Result<(), Failure> {
    let allow = allowlist(&data, &args.speakers);
    let (engine, _) = replay(Engine::new(data), &args.log)?;
    let stored: HashSet<String> = engine.characters().into_iter().map(|c| c.name).collect();
    let mut written = Ok(());
    read_log(&args.log, |line| {
        if written.is_err() {
            return;
        }
        let Some(ev) = parse_line(&line.text, &allow) else {
            return;
        };
        if matches!(ev.kind, EventKind::Party { .. }) {
            return;
        }
        if named(&ev.kind).is_some_and(|n| !stored.contains(n)) {
            return;
        }
        written = if args.json {
            serde_json::to_string(&ev)
                .map_err(io::Error::other)
                .and_then(|text| writeln!(out, "{text}"))
        } else {
            writeln!(out, "{}", event_text(&ev))
        };
    })?;
    Ok(written?)
}

/// `<time> <kind> key=value ...`, values JSON-encoded.
fn event_text(ev: &Event) -> String {
    let mut text = format!("{} {}", ev.time, kind_name(&ev.kind));
    if let Ok(Value::Object(fields)) = serde_json::to_value(&ev.kind) {
        for (key, value) in fields.iter().filter(|(k, _)| k.as_str() != "kind") {
            text.push_str(&format!(" {key}={value}"));
        }
    }
    text
}

fn kind_name(kind: &EventKind) -> String {
    serde_json::to_value(kind)
        .ok()
        .and_then(|v| v.get("kind").and_then(Value::as_str).map(str::to_string))
        .unwrap_or_default()
}

// -- state and import ---------------------------------------------------------

/// The `state` report: per stored character, its quests by status, per act.
fn state_json(engine: &Engine, replayed: &Replayed) -> Value {
    let data = engine.data();
    let characters: Vec<Value> = engine
        .characters()
        .into_iter()
        .map(|c| {
            let mut quests = Map::new();
            let mut by_act: BTreeMap<u32, BTreeMap<&'static str, Vec<String>>> = BTreeMap::new();
            for (quest, state) in data.quests.iter().zip(engine.quest_states(&c.name)) {
                quests.insert(state.quest_id.clone(), json!(state.status));
                by_act
                    .entry(quest.act)
                    .or_default()
                    .entry(state.status.as_str())
                    .or_default()
                    .push(state.quest_id);
            }
            let acts: Vec<Value> = by_act
                .into_iter()
                .map(|(act, statuses)| json!({ "act": act, "statuses": statuses }))
                .collect();
            json!({
                "name": c.name,
                "class": c.class,
                "level": c.level,
                "own": c.own,
                "active": c.active,
                "quests": quests,
                "acts": acts,
            })
        })
        .collect();
    json!({
        "lines": replayed.lines,
        "client_version": engine.client_version(),
        "held": engine.held_count(),
        // Two kinds of dropped event, never summed: those that came before
        // any character, and those of an unnamed new character that a
        // level-1 entry to another instance of The Riverbank replaced.
        "dropped": engine.dropped_count(),
        "dropped_new_character": engine.dropped_new_character_count(),
        "refused": engine.refused_count(),
        "characters": characters,
    })
}

fn print_state(
    engine: &Engine,
    replayed: &Replayed,
    as_json: bool,
    store: Option<Value>,
    out: &mut impl Write,
) -> Result<(), Failure> {
    let mut report = state_json(engine, replayed);
    if let Some(store) = store.clone() {
        report["store"] = store;
    }
    if as_json {
        writeln!(out, "{report}")?;
        return Ok(());
    }
    writeln!(
        out,
        "{} lines, client {}, {} events held, {} dropped before any character, \
         {} dropped with a replaced new character, {} refused",
        replayed.lines,
        engine.client_version().unwrap_or("unknown"),
        engine.held_count(),
        engine.dropped_count(),
        engine.dropped_new_character_count(),
        engine.refused_count()
    )?;
    let characters = report["characters"].as_array().cloned().unwrap_or_default();
    if characters.is_empty() {
        writeln!(out, "no character found")?;
    }
    for c in &characters {
        let class = c["class"].as_str().filter(|s| !s.is_empty()).unwrap_or("?");
        let own = if c["own"].as_bool() == Some(true) {
            "own"
        } else {
            "unconfirmed"
        };
        writeln!(
            out,
            "{} ({class}, level {}, {own})",
            c["name"].as_str().unwrap_or(""),
            c["level"]
        )?;
        for act in c["acts"].as_array().into_iter().flatten() {
            writeln!(out, "  act {}", act["act"])?;
            if let Some(statuses) = act["statuses"].as_object() {
                // Status order, not alphabetical.
                for status in Status::NAMES {
                    if let Some(ids) = statuses.get(status).and_then(Value::as_array) {
                        let ids: Vec<&str> = ids.iter().filter_map(Value::as_str).collect();
                        writeln!(out, "    {status} ({}): {}", ids.len(), ids.join(", "))?;
                    }
                }
            }
        }
    }
    if let Some(store) = store {
        writeln!(
            out,
            "written to {} (position at offset {})",
            store["dir"].as_str().unwrap_or(""),
            store["offset"]
        )?;
    }
    Ok(())
}

/// Says on stderr that the store moved an unreadable file aside.
fn note_quarantine(loaded: &Loaded) {
    if let Some(q) = &loaded.quarantined {
        eprintln!("quest-tracker-replay: {}", q.status_text());
    }
}

/// `state`, then the store: every stored character's engine state, the
/// roster and the position at EOF. Characters already in the store are
/// loaded first, so the user's overrides survive and evidence, being a set,
/// comes out the same however often the same log is imported. A pin is kept
/// in the roster but not applied to the replay: a pin is about the session
/// being played, not the whole history of the log.
fn import(args: &Args, data: Arc<QuestData>, out: &mut impl Write) -> Result<(), Failure> {
    let store_error = |e: qt_store::StoreError| Failure::Error(format!("store: {e}"));
    // A missing log must exit 2 before the store is touched.
    read_log_exists(&args.log)?;
    let store = Store::open_default().map_err(store_error)?;

    let loaded = store.load_characters().map_err(store_error)?;
    note_quarantine(&loaded);
    let mut roster = match loaded.value {
        Some(Value::Object(map)) => map,
        Some(_) => {
            return Err(Failure::Error(format!(
                "store: {} is not a name -> character object; not overwriting it",
                store.dir().join(qt_store::CHARACTERS_FILE).display()
            )))
        }
        None => Map::new(),
    };
    let mut engine = Engine::new(data);
    let mut pinned = BTreeSet::new();
    for (name, entry) in &roster {
        if entry.get("pinned").and_then(Value::as_bool) == Some(true) {
            pinned.insert(name.clone());
        }
        let loaded = store.load_character(name).map_err(store_error)?;
        note_quarantine(&loaded);
        if let Some(value) = loaded.value {
            engine.load_character(&value).map_err(|e| {
                Failure::Error(format!(
                    "store: character {name:?}: {e}; not overwriting it"
                ))
            })?;
        }
    }

    let (engine, replayed) = replay(engine, &args.log)?;

    for c in engine.characters() {
        let state = engine
            .character_json(&c.name)
            .expect("a listed character has a state");
        store.save_character(&c.name, &state).map_err(store_error)?;
        roster.insert(
            c.name.clone(),
            json!({
                "class": c.class,
                "level": c.level,
                "own": c.own,
                "pinned": pinned.contains(&c.name),
            }),
        );
    }
    store
        .save_characters(&Value::Object(roster))
        .map_err(store_error)?;
    let mut position = replayed.position.clone();
    position.path = std::path::absolute(&position.path).unwrap_or(position.path);
    let position_json = serde_json::to_value(&position)
        .map_err(|e| Failure::Error(format!("position of {}: {e}", position.path.display())))?;
    store.save_position(&position_json).map_err(store_error)?;

    let summary = json!({
        "dir": store.dir().display().to_string(),
        "offset": position.offset,
    });
    print_state(&engine, &replayed, args.json, Some(summary), out)
}

/// The missing-log check of [`read_log`], without reading.
fn read_log_exists(path: &Path) -> Result<(), Failure> {
    match fs::metadata(path) {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Err(Failure::NoLog(path.to_path_buf())),
        Err(e) => Err(Failure::Error(format!("{}: {e}", path.display()))),
    }
}

// -- scan ---------------------------------------------------------------------

/// What one area's instances logged, as counts.
#[derive(Default)]
struct AreaCounts {
    /// `[SCENE] Set Source` names seen after entering it.
    names: BTreeMap<String, u64>,
    entries: u64,
    /// `has received` lines, by stat text.
    rewards: BTreeMap<String, u64>,
    /// `You have received ... Passive Skill Points` lines, by pool.
    points: BTreeMap<&'static str, u64>,
    /// Allowlisted speakers' lines, by speaker.
    speakers: BTreeMap<String, u64>,
    /// Allowlisted system lines, by text.
    system_lines: BTreeMap<String, u64>,
}

impl AreaCounts {
    /// The name seen most often after entering it.
    fn name(&self) -> Option<&str> {
        let mut best: Option<(&str, u64)> = None;
        for (name, &n) in &self.names {
            if best.is_none_or(|(_, b)| n > b) {
                best = Some((name, n));
            }
        }
        best.map(|(name, _)| name)
    }

    fn is_empty(&self) -> bool {
        self.names.is_empty()
            && self.entries == 0
            && self.rewards.is_empty()
            && self.points.is_empty()
            && self.speakers.is_empty()
            && self.system_lines.is_empty()
    }

    fn json(&self) -> Value {
        json!({
            "entries": self.entries,
            "reward_lines": self.rewards.values().sum::<u64>(),
            "points_lines": self.points.values().sum::<u64>(),
            "speaker_lines": self.speakers.values().sum::<u64>(),
            "system_line_count": self.system_lines.values().sum::<u64>(),
            "rewards": self.rewards,
            "points": self.points,
            "speakers": self.speakers,
            "system_lines": self.system_lines,
        })
    }
}

/// Counts what each area code's instances logged. The area is the one the
/// last `Generating level` line entered in the same session; lines before
/// any (or after a `LOG FILE OPENING`) count as outside every area. Names
/// from level-up, death, join and leave lines are never counted.
fn scan(args: &Args, data: &QuestData, out: &mut impl Write) -> Result<(), Failure> {
    let allow = allowlist(data, &args.speakers);
    let mut areas: BTreeMap<String, AreaCounts> = BTreeMap::new();
    let mut outside = AreaCounts::default();
    let mut current: Option<String> = None;
    let mut lines = 0u64;
    read_log(&args.log, |line| {
        lines += 1;
        let Some(ev) = parse_line(&line.text, &allow) else {
            return;
        };
        if let EventKind::SessionStart = ev.kind {
            current = None;
            return;
        }
        if let EventKind::AreaEntered { code, .. } = &ev.kind {
            current = Some(code.clone());
        }
        let counts = match &current {
            Some(code) => areas.entry(code.clone()).or_default(),
            None => &mut outside,
        };
        let bump = |map: &mut BTreeMap<String, u64>, key: &str| {
            *map.entry(key.to_string()).or_default() += 1;
        };
        match &ev.kind {
            EventKind::AreaEntered { .. } => counts.entries += 1,
            EventKind::AreaName { name } => bump(&mut counts.names, name),
            EventKind::Received { stat, .. } => bump(&mut counts.rewards, stat),
            EventKind::Points { pool, .. } => {
                let pool = match pool {
                    PointPool::Passive => "passive",
                    PointPool::WeaponSet => "weapon_set",
                };
                *counts.points.entry(pool).or_default() += 1;
            }
            EventKind::NpcLine { speaker, .. } => bump(&mut counts.speakers, speaker),
            EventKind::SystemLine { text } => bump(&mut counts.system_lines, text),
            _ => {}
        }
    })?;

    let display = |code: &str, counts: &AreaCounts| -> Option<String> {
        counts
            .name()
            .map(str::to_string)
            .or_else(|| data.areas.get(code).map(|a| a.name.clone()))
    };
    if args.json {
        let list: Vec<Value> = areas
            .iter()
            .map(|(code, counts)| {
                let mut v = counts.json();
                v["code"] = json!(code);
                v["name"] = json!(display(code, counts));
                v
            })
            .collect();
        let report = json!({
            "lines": lines,
            "areas": list,
            "outside_areas": outside.json(),
        });
        writeln!(out, "{report}")?;
        return Ok(());
    }
    writeln!(out, "{lines} lines, {} area codes", areas.len())?;
    let rows = areas
        .iter()
        .map(|(code, counts)| (code.as_str(), display(code, counts), counts))
        .chain((!outside.is_empty()).then_some(("(outside any area)", None, &outside)));
    for (code, name, counts) in rows {
        writeln!(
            out,
            "{code}  {}  entries {}, reward lines {}, points lines {}, speaker lines {}, system lines {}",
            name.as_deref().unwrap_or("?"),
            counts.entries,
            counts.rewards.values().sum::<u64>(),
            counts.points.values().sum::<u64>(),
            counts.speakers.values().sum::<u64>(),
            counts.system_lines.values().sum::<u64>(),
        )?;
        for (speaker, n) in &counts.speakers {
            writeln!(out, "    speaker {speaker}: {n}")?;
        }
        for (text, n) in &counts.system_lines {
            writeln!(out, "    system {text:?}: {n}")?;
        }
        for (stat, n) in &counts.rewards {
            writeln!(out, "    reward {stat:?}: {n}")?;
        }
        for (pool, n) in &counts.points {
            writeln!(out, "    points {pool}: {n}")?;
        }
    }
    Ok(())
}
