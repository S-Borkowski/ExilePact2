//! The replay CLI end to end: every test runs the built `quest-tracker-replay`
//! binary over the synthetic sessions in `fixtures/`, with
//! `QUEST_TRACKER_DIR` set to a temporary folder, so no test reads a real log
//! or touches a real store.

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::Arc;

use qt_engine::{Engine, QuestData};
use qt_store::Store;
use serde_json::{json, Value};

const BIN: &str = env!("CARGO_BIN_EXE_quest-tracker-replay");
const CORISANDE: &str = "Corisande_Fell";

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

fn session(name: &str) -> PathBuf {
    fixtures().join("sessions").join(name)
}

fn quests_min() -> PathBuf {
    fixtures().join("quests_min.json")
}

/// Runs the binary with `store` as its store folder.
fn run<I, S>(store: &Path, args: I) -> Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    Command::new(BIN)
        .args(args)
        .env("QUEST_TRACKER_DIR", store)
        .output()
        .expect("the replay binary runs")
}

fn stdout(o: &Output) -> String {
    String::from_utf8(o.stdout.clone()).expect("UTF-8 output")
}

fn ok_json(o: &Output) -> Value {
    assert!(
        o.status.success(),
        "exit {:?}: {}",
        o.status.code(),
        String::from_utf8_lossy(&o.stderr)
    );
    serde_json::from_str(&stdout(o)).expect("one JSON object")
}

/// Every file under `dir`, by path relative to it, with its bytes.
fn snapshot(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                let rel = path.strip_prefix(root).unwrap().to_string_lossy();
                out.insert(rel.replace('\\', "/"), fs::read(&path).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, dir, &mut out);
    out
}

/// The engine itself over the same session, lines as the tailer hands them.
fn engine_over(name: &str) -> Engine {
    let data = Arc::new(QuestData::load(&quests_min()).unwrap());
    let mut engine = Engine::new(data);
    for line in fs::read_to_string(session(name)).unwrap().lines() {
        engine.feed_line(line, false);
    }
    engine
}

#[test]
fn state_on_fixture() {
    let store = tempfile::tempdir().unwrap();
    let report = ok_json(&run(
        store.path(),
        [
            OsStr::new("state"),
            session("act1.txt").as_os_str(),
            OsStr::new("--data"),
            quests_min().as_os_str(),
            OsStr::new("--json"),
        ],
    ));
    let chars = report["characters"].as_array().unwrap();
    assert_eq!(chars.len(), 1, "{report}");
    let c = &chars[0];
    assert_eq!(
        (&c["name"], &c["class"], &c["level"], &c["own"]),
        (&json!(CORISANDE), &json!("Witch"), &json!(5), &json!(true))
    );
    // The statuses the engine's own target test lists for this session.
    let want = json!({
        "fixture-riverbank": "done",
        "fixture-beira": "done",
        "fixture-crowbell": "available",
        "fixture-king": "in_progress",
        "fixture-lute": "in_progress",
        "fixture-candles": "unknown",
        "fixture-caravan": "locked",
    });
    assert_eq!(c["quests"], want);
    // And what the engine says when fed directly: the CLI adds nothing.
    let engine = engine_over("act1.txt");
    for state in engine.quest_states(CORISANDE) {
        assert_eq!(
            c["quests"][&state.quest_id],
            json!(state.status.as_str()),
            "{}",
            state.quest_id
        );
    }
    assert_eq!(report["client_version"], json!("4.5.3"));
    assert_eq!(report["held"], json!(0));
    // Quests by status per act.
    assert_eq!(
        c["acts"][1],
        json!({"act": 2, "statuses": {"locked": ["fixture-caravan"]}})
    );

    // The plain text names the character and its acts.
    let text = run(
        store.path(),
        [
            OsStr::new("state"),
            session("act1.txt").as_os_str(),
            OsStr::new("--data"),
            quests_min().as_os_str(),
        ],
    );
    assert!(text.status.success());
    let text = stdout(&text);
    assert!(
        text.contains("Corisande_Fell (Witch, level 5, own)"),
        "{text}"
    );
    assert!(
        text.contains("done (2): fixture-riverbank, fixture-beira"),
        "{text}"
    );

    // The data built in (whatever quests it holds) runs the same detection.
    let built_in = ok_json(&run(
        store.path(),
        [
            OsStr::new("state"),
            session("act1.txt").as_os_str(),
            OsStr::new("--json"),
        ],
    ));
    assert_eq!(built_in["characters"][0]["name"], json!(CORISANDE));

    // A missing log exits 2 and prints nothing on stdout; state never writes
    // a store.
    let missing = run(
        store.path(),
        [
            OsStr::new("state"),
            store.path().join("no-such-log.txt").as_os_str(),
            OsStr::new("--json"),
        ],
    );
    assert_eq!(missing.status.code(), Some(2));
    assert!(missing.stdout.is_empty());
    assert!(snapshot(store.path()).is_empty());

    // A usage error is not mistaken for a missing log.
    let usage = run(store.path(), ["state"]);
    assert_eq!(usage.status.code(), Some(64));
    let help = run(store.path(), ["--help"]);
    assert!(help.status.success());
    for name in [
        "events",
        "state",
        "import",
        "scan",
        "--data",
        "--json",
        "--speakers",
    ] {
        assert!(stdout(&help).contains(name), "--help lacks {name}");
    }
}

/// A client log line with the fixtures' prefix.
fn log_line(message: &str) -> String {
    format!("2026/03/14 12:00:00 9000000 abcdef01 [INFO Client 5120] {message}")
}

#[test]
fn summary_reports_refused() {
    const MARKER: &str = "2026/03/14 11:59:00 ***** LOG FILE OPENING *****";
    let area = log_line(r#"Generating level 2 area "G1_2" with seed 7"#);
    let reward = log_line(&format!(
        ": {CORISANDE} has received +10% to [Resistances|Cold Resistance]."
    ));
    let store = tempfile::tempdir().unwrap();
    let logs = tempfile::tempdir().unwrap();
    let data = quests_min();
    let state = |name: &str, lines: &[&str], as_json: bool| {
        let log = logs.path().join(name);
        fs::write(&log, lines.join("\n") + "\n").unwrap();
        let mut args = vec![
            OsStr::new("state"),
            log.as_os_str(),
            OsStr::new("--data"),
            data.as_os_str(),
        ];
        if as_json {
            args.push(OsStr::new("--json"));
        }
        run(store.path(), args)
    };

    // The session marker follows the last area line: the area is unknown
    // when fixture-beira's area-tied reward arrives, so it is refused.
    let refused = [MARKER, area.as_str(), MARKER, reward.as_str()];
    let report = ok_json(&state("refused.txt", &refused, true));
    assert_eq!(report["refused"], json!(1), "{report}");
    assert_ne!(
        report["characters"][0]["quests"]["fixture-beira"],
        json!("done"),
        "{report}"
    );
    let text = state("refused.txt", &refused, false);
    assert!(text.status.success());
    let text = stdout(&text);
    let want = format!(
        "{} dropped before any character, {} dropped with a replaced new character, 1 refused",
        report["dropped"], report["dropped_new_character"]
    );
    assert!(text.contains(&want), "{text}");

    // Negative control: the same reward in its area is no refusal, and the
    // quest is done.
    let in_area = [MARKER, area.as_str(), reward.as_str()];
    let report = ok_json(&state("in-area.txt", &in_area, true));
    assert_eq!(report["refused"], json!(0), "{report}");
    assert_eq!(
        report["characters"][0]["quests"]["fixture-beira"],
        json!("done"),
        "{report}"
    );
    let text = stdout(&state("in-area.txt", &in_area, false));
    let want = format!(
        "{} dropped before any character, {} dropped with a replaced new character, 0 refused",
        report["dropped"], report["dropped_new_character"]
    );
    assert!(text.contains(&want), "{text}");
}

/// The level-1 entry to The Riverbank that opens a new character.
fn riverbank(seed: u64) -> String {
    log_line(&format!(
        r#"Generating level 1 area "G1_1" with seed {seed}"#
    ))
}

#[test]
fn summary_reports_dropped_new_character() {
    const MARKER: &str = "2026/03/14 11:59:00 ***** LOG FILE OPENING *****";
    let clearfell = log_line(r#"Generating level 2 area "G1_2" with seed 7"#);
    let store = tempfile::tempdir().unwrap();
    let logs = tempfile::tempdir().unwrap();
    let data = quests_min();
    let write = |name: &str, lines: &[&str]| {
        let log = logs.path().join(name);
        fs::write(&log, lines.join("\n") + "\n").unwrap();
        log
    };
    let state = |log: &Path, as_json: bool| {
        let mut args = vec![
            OsStr::new("state"),
            log.as_os_str(),
            OsStr::new("--data"),
            data.as_os_str(),
        ];
        if as_json {
            args.push(OsStr::new("--json"));
        }
        run(store.path(), args)
    };
    // The engine itself over the same lines, as the replay feeds them.
    let engine_over = |log: &Path| {
        let mut engine = Engine::new(Arc::new(QuestData::load(&data).unwrap()));
        for line in fs::read_to_string(log).unwrap().lines() {
            engine.feed_line(line, false);
        }
        engine
    };

    // One event before any character, dropped at the next marker; then a
    // new character in instance 11 that takes two events and is parked at
    // a marker; then a level-1 entry to another instance (22), which
    // replaces the parked record once the next area entry closes its relog
    // window.
    let (first, second) = (riverbank(11), riverbank(22));
    let replaced = [
        MARKER,
        clearfell.as_str(),
        MARKER,
        first.as_str(),
        clearfell.as_str(),
        MARKER,
        second.as_str(),
        clearfell.as_str(),
    ];
    let log = write("replaced.txt", &replaced);
    let engine = engine_over(&log);
    assert_eq!(engine.dropped_count(), 1);
    assert!(engine.dropped_new_character_count() > 0);
    let report = ok_json(&state(&log, true));
    assert_eq!(report["dropped"], json!(1), "{report}");
    assert_eq!(
        report["dropped_new_character"],
        json!(engine.dropped_new_character_count()),
        "{report}"
    );
    let text = state(&log, false);
    assert!(text.status.success());
    let text = stdout(&text);
    let want = format!(
        "1 dropped before any character, {} dropped with a replaced new character,",
        engine.dropped_new_character_count()
    );
    assert!(text.contains(&want), "{text}");

    // Negative control: the second entry is to the same instance, so it
    // resumes the parked record and nothing of it is dropped; the event
    // before any character still is, and is reported apart.
    let resumed = [
        MARKER,
        clearfell.as_str(),
        MARKER,
        first.as_str(),
        clearfell.as_str(),
        MARKER,
        first.as_str(),
        clearfell.as_str(),
    ];
    let log = write("resumed.txt", &resumed);
    assert_eq!(engine_over(&log).dropped_new_character_count(), 0);
    let report = ok_json(&state(&log, true));
    assert_eq!(report["dropped"], json!(1), "{report}");
    assert_eq!(report["dropped_new_character"], json!(0), "{report}");
    let text = stdout(&state(&log, false));
    assert!(
        text.contains("1 dropped before any character, 0 dropped with a replaced new character,"),
        "{text}"
    );
}

/// A client line at a given time of 2026/03/14, for the timed cases.
fn line_at(time: &str, level: &str, message: &str) -> String {
    format!("2026/03/14 {time} 9000000 abcdef01 [{level} Client 5120] {message}")
}

#[test]
fn replay_decides_an_open_relog_window_at_end() {
    const MARKER: &str = "2026/03/14 11:59:00 ***** LOG FILE OPENING *****";
    let level_up = line_at(
        "12:00:00",
        "INFO",
        &format!(": {CORISANDE} (Witch) is now level 2"),
    );
    let entry = line_at(
        "12:00:10",
        "INFO",
        r#"Generating level 1 area "G1_1" with seed 11"#,
    );
    let beira = line_at(
        "12:00:12",
        "INFO",
        "Beira of the Rotten Pack: Fixture line: the pack gathers.",
    );
    let store = tempfile::tempdir().unwrap();
    let logs = tempfile::tempdir().unwrap();
    let data = quests_min();
    let write = |name: &str, lines: &[&str]| {
        let log = logs.path().join(name);
        fs::write(&log, lines.join("\n") + "\n").unwrap();
        log
    };
    let state = |log: &Path, as_json: bool| {
        let mut args = vec![
            OsStr::new("state"),
            log.as_os_str(),
            OsStr::new("--data"),
            data.as_os_str(),
        ];
        if as_json {
            args.push(OsStr::new("--json"));
        }
        run(store.path(), args)
    };

    // The log ends 2 s after a level-1 entry, with a line of the window
    // after it: the engine alone leaves the window undecided at EOF.
    let open = [MARKER, level_up.as_str(), entry.as_str(), beira.as_str()];
    let log = write("open.txt", &open);
    let mut engine = Engine::new(Arc::new(QuestData::load(&data).unwrap()));
    for line in fs::read_to_string(&log).unwrap().lines() {
        engine.feed_line(line, false);
    }
    assert!(engine.relog_window_open());
    assert!(!engine.new_character_open());
    // The overlay's quiet-log close, 6 s after the last line.
    engine.expire_relog_window(qt_engine::Timestamp::parse("2026/03/14 12:00:18").unwrap());
    assert!(engine.new_character_open());
    let control = engine.new_character_json().unwrap();
    let control_states = engine.new_character_states().unwrap();
    assert!(control["events"].as_u64().unwrap() > 0, "{control}");

    // The replay decides it the same way: the new character is shown,
    // with the window's events in it, and the summary says why.
    let report = ok_json(&state(&log, true));
    assert_eq!(
        report["relog_window_closed_at_end"],
        json!(true),
        "{report}"
    );
    let new = &report["new_character"];
    assert_eq!(new["events"], control["events"], "{report}");
    assert_eq!(new["level"], json!(1), "{report}");
    for s in &control_states {
        assert_eq!(new["quests"][&s.quest_id], json!(s.status), "{report}");
    }
    assert_eq!(
        new["quests"]["fixture-riverbank"],
        json!("in_progress"),
        "{report}"
    );
    let corisande = &report["characters"][0];
    assert_eq!(corisande["name"], json!(CORISANDE), "{report}");
    assert_eq!(corisande["active"], json!(false), "{report}");
    let text = stdout(&state(&log, false));
    assert!(
        text.contains("a relog window was still open: closed at the end of the log"),
        "{text}"
    );
    assert!(
        text.contains(&format!(
            "new character (unnamed, level 1, {} events",
            control["events"]
        )),
        "{text}"
    );

    // Negative control: a later area entry closes the window inside the
    // log. The new character is there all the same, and the summary does not
    // claim the end of the log closed it.
    let clearfell = line_at(
        "12:00:20",
        "INFO",
        r#"Generating level 2 area "G1_2" with seed 7"#,
    );
    let closed = [
        MARKER,
        level_up.as_str(),
        entry.as_str(),
        beira.as_str(),
        clearfell.as_str(),
    ];
    let report = ok_json(&state(&write("closed.txt", &closed), true));
    assert_eq!(
        report["relog_window_closed_at_end"],
        json!(false),
        "{report}"
    );
    assert!(report["new_character"].is_object(), "{report}");
    let text = stdout(&state(&write("closed.txt", &closed), false));
    assert!(!text.contains("closed at the end of the log"), "{text}");

    // Negative control: the build line names the stored character within
    // the window, so the entry was a relog and there is no new character.
    let relog_line = line_at(
        "12:00:11",
        "WARN",
        &format!("Character {CORISANDE} has unknown build of path: None"),
    );
    let relog = [
        MARKER,
        level_up.as_str(),
        entry.as_str(),
        relog_line.as_str(),
        beira.as_str(),
    ];
    let report = ok_json(&state(&write("relog.txt", &relog), true));
    assert_eq!(
        report["relog_window_closed_at_end"],
        json!(false),
        "{report}"
    );
    assert_eq!(report["new_character"], Value::Null, "{report}");
    assert_eq!(report["characters"][0]["active"], json!(true), "{report}");
}

/// The relog of `riverbank_relog.txt`: a character named by its level-up
/// in a Riverbank instance, then a new session whose level-1 entry has that
/// instance's seed and no build line naming anyone, then Clearfell.
#[test]
fn replay_keeps_a_named_character_on_a_riverbank_relog() {
    const NAME: &str = "Tamsin_Vodrey";
    const SEED: &str = "with seed 3141592653";
    let store = tempfile::tempdir().unwrap();
    let data = quests_min();
    let state = |log: &Path, as_json: bool| {
        let mut args = vec![
            OsStr::new("state"),
            log.as_os_str(),
            OsStr::new("--data"),
            data.as_os_str(),
        ];
        if as_json {
            args.push(OsStr::new("--json"));
        }
        run(store.path(), args)
    };
    let fixture = session("riverbank_relog.txt");
    let text = fs::read_to_string(&fixture).unwrap();
    assert_eq!(text.matches(SEED).count(), 2, "two entries, one seed");
    assert!(!text.contains("has unknown build of path"), "no name line");

    // The second entry has the seed the level-up gave the character, so it
    // is that character's relog: Clearfell after it unlocks fixture-beira
    // for the named character, and no new character is open.
    let report = ok_json(&state(&fixture, true));
    let chars = report["characters"].as_array().unwrap();
    assert_eq!(chars.len(), 1, "{report}");
    let c = &chars[0];
    assert_eq!(
        (&c["name"], &c["class"], &c["level"], &c["active"]),
        (&json!(NAME), &json!("Warrior"), &json!(2), &json!(true)),
        "{report}"
    );
    assert_eq!(c["quests"]["fixture-beira"], json!("available"), "{report}");
    assert_eq!(
        c["quests"]["fixture-riverbank"],
        json!("in_progress"),
        "{report}"
    );
    assert_eq!(report["new_character"], Value::Null, "{report}");
    assert_eq!(report["dropped_new_character"], json!(0), "{report}");
    let summary = state(&fixture, false);
    assert!(summary.status.success());
    let summary = stdout(&summary);
    assert!(
        summary.contains(&format!("{NAME} (Warrior, level 2, own)")),
        "{summary}"
    );
    assert!(!summary.contains("new character (unnamed"), "{summary}");

    // Negative control: the same session with another seed on the second
    // entry is a fresh instance, so the window opens, no line names
    // anyone, and Clearfell's evidence goes to a new character instead.
    let logs = tempfile::tempdir().unwrap();
    let at = text.rfind(SEED).unwrap();
    let other = format!(
        "{}{}",
        &text[..at],
        text[at..].replacen(SEED, "with seed 2718281828", 1)
    );
    let log = logs.path().join("fresh_instance.txt");
    fs::write(&log, other).unwrap();
    let report = ok_json(&state(&log, true));
    let c = &report["characters"][0];
    assert_eq!(c["name"], json!(NAME), "{report}");
    assert_eq!(c["quests"]["fixture-beira"], json!("locked"), "{report}");
    let new = &report["new_character"];
    assert!(new.is_object(), "{report}");
    assert_eq!(new["quests"]["fixture-beira"], json!("available"), "{report}");
    let summary = stdout(&state(&log, false));
    assert!(summary.contains("new character (unnamed"), "{summary}");
}

#[test]
fn events_hide_character_line_of_unstored_name() {
    const MARKER: &str = "2026/03/14 11:59:00 ***** LOG FILE OPENING *****";
    // An invented name the log never stores as a character.
    const STRANGER: &str = "Thessaly_Rook";
    let build_line = |name: &str| {
        format!(
            "2026/03/14 12:00:01 9000100 abcdef02 [WARN Client 5120] \
             Character {name} has unknown build of path: None"
        )
    };
    let lines = [
        MARKER.to_string(),
        log_line(r#"Generating level 2 area "G1_2" with seed 7"#),
        log_line(&format!(": {CORISANDE} (Witch) is now level 2")),
        build_line(CORISANDE),
        build_line(STRANGER),
    ];
    let logs = tempfile::tempdir().unwrap();
    let log = logs.path().join("named.txt");
    fs::write(&log, lines.join("\n") + "\n").unwrap();
    let store = tempfile::tempdir().unwrap();
    let data = quests_min();
    for json_flag in [false, true] {
        let mut args = vec![
            OsStr::new("events"),
            log.as_os_str(),
            OsStr::new("--data"),
            data.as_os_str(),
        ];
        if json_flag {
            args.push(OsStr::new("--json"));
        }
        let o = run(store.path(), args);
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        let out = stdout(&o);
        assert!(!out.contains(STRANGER), "printed {STRANGER:?}:\n{out}");
        // Positive control through the same output: the stored character's
        // line is printed.
        let want = if json_flag {
            format!(r#""kind":"character_named","name":"{CORISANDE}""#)
        } else {
            format!(r#"character_named name="{CORISANDE}""#)
        };
        assert!(out.contains(&want), "missing {want:?}:\n{out}");
    }
}

/// Every invented chat speaker, chat text and party member of the fixtures
/// the events below are read from.
const NEVER_PRINTED: [&str; 20] = [
    // fixtures/logs/chat.txt
    "Pellwick_Dunmore",
    "Corvanne",
    "Brisket_Lane",
    "Quillan_Marsh",
    "Tessaly_Vane",
    "global chat",
    "local chat",
    "party chat",
    "guild message",
    "lower-case speaker",
    // fixtures/sessions/baseline.txt and act1.txt
    "Hollis_Varne",
    "Ottoline_Brack",
    "Quillon_Reed",
    "WTB",
    "fixture ring",
    "inviting",
    "guild line",
    // fixtures/sessions/party.txt: a member who joins, then levels up, and a
    // name that levels up, then leaves.
    "Bram_Holloway",
    "Wren_Ashdown",
    "has joined",
];

#[test]
fn events_never_print_chat_or_unknown_speakers() {
    let store = tempfile::tempdir().unwrap();
    let logs = [
        fixtures().join("logs").join("chat.txt"),
        session("baseline.txt"),
        session("act1.txt"),
        session("party.txt"),
    ];
    let data = quests_min();
    let mut all = String::new();
    for log in &logs {
        for json_flag in [false, true] {
            let mut args = vec![
                OsStr::new("events"),
                log.as_os_str(),
                OsStr::new("--data"),
                data.as_os_str(),
            ];
            if json_flag {
                args.push(OsStr::new("--json"));
            }
            let o = run(store.path(), args);
            assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
            let out = stdout(&o);
            if json_flag {
                for line in out.lines() {
                    let ev: Value = serde_json::from_str(line).expect("one object per line");
                    assert!(ev["kind"].is_string() && ev["time"].is_string(), "{line}");
                    assert_ne!(ev["kind"], json!("party"), "{line}");
                }
            }
            all.push_str(&out);
        }
    }
    for needle in NEVER_PRINTED {
        assert!(!all.contains(needle), "printed {needle:?}:\n{all}");
    }

    // Positive controls through the same output: allowlisted speakers, the
    // own character's lines and its later session are printed.
    for needle in [
        "speaker=\"Renly\" text=\"Fixture line: welcome to the encampment.\"",
        "\"speaker\":\"Beira of the Rotten Pack\"",
        "level_up class=\"Witch\" level=5 name=\"Corisande_Fell\"",
        "level_up class=\"Witch\" level=10 name=\"Corisande_Fell\"",
        "received name=\"Corisande_Fell\" stat=\"+10% to Cold Resistance\"",
        "system_line text=\"Fixture system line: a lute string hums.\"",
    ] {
        assert!(all.contains(needle), "missing {needle:?}:\n{all}");
    }

    // The allowlist is what keeps local chat out: name the speaker with
    // --speakers and the same line is printed.
    let chat = fixtures().join("logs").join("chat.txt");
    let o = run(
        store.path(),
        [
            OsStr::new("events"),
            chat.as_os_str(),
            OsStr::new("--speakers"),
            OsStr::new("Pellwick_Dunmore"),
        ],
    );
    assert!(o.status.success());
    let out = stdout(&o);
    assert!(
        out.contains("local chat shaped exactly like an NPC line"),
        "{out}"
    );
    // Channel chat stays out even for an allowlisted name.
    assert!(!out.contains("spare charm"), "{out}");
}

#[test]
fn import_is_idempotent() {
    let store = tempfile::tempdir().unwrap();
    let dir = store.path().join("store");
    let act1 = session("act1.txt");
    let import = |log: &Path| {
        ok_json(&run(
            &dir,
            [
                OsStr::new("import"),
                log.as_os_str(),
                OsStr::new("--data"),
                quests_min().as_os_str(),
                OsStr::new("--json"),
            ],
        ))
    };

    // A missing log writes nothing.
    let missing = run(
        &dir,
        [
            OsStr::new("import"),
            store.path().join("no-such-log.txt").as_os_str(),
        ],
    );
    assert_eq!(missing.status.code(), Some(2));
    assert!(!dir.exists());

    let report = import(&act1);
    assert_eq!(
        report["characters"][0]["quests"]["fixture-beira"],
        json!("done")
    );
    let first = snapshot(&dir);
    let names: Vec<&str> = first.keys().map(String::as_str).collect();
    let file = format!(
        "characters/{}.json",
        qt_store::safe_name(CORISANDE).unwrap()
    );
    assert_eq!(
        names,
        ["characters.json", file.as_str(), "position.json"],
        "{names:?}"
    );

    // What was written is what the overlay reads back.
    let s = Store::open(&dir).unwrap();
    let position = s.load_position().unwrap().value.unwrap();
    let len = fs::metadata(&act1).unwrap().len();
    assert_eq!(position["offset"], json!(len));
    assert!(position["path"].as_str().unwrap().ends_with("act1.txt"));
    assert!(Path::new(position["path"].as_str().unwrap()).is_absolute());
    assert_eq!(
        position["fingerprint"]["sha256"].as_str().unwrap().len(),
        64
    );
    assert_eq!(
        s.load_characters().unwrap().value.unwrap(),
        json!({CORISANDE: {"class": "Witch", "level": 5, "own": true, "pinned": false}})
    );
    let engine = engine_over("act1.txt");
    assert_eq!(
        s.load_character(CORISANDE).unwrap().value.unwrap(),
        engine.character_json(CORISANDE).unwrap()
    );

    // The same log again: the same files, byte for byte.
    import(&act1);
    assert_eq!(snapshot(&dir), first);

    // The user's overrides and pin survive an import, and importing again
    // still changes nothing.
    let mut state = s.load_character(CORISANDE).unwrap().value.unwrap();
    state["overrides"] = json!({"fixture-caravan": {"skipped": true}});
    s.save_character(CORISANDE, &state).unwrap();
    let mut roster = s.load_characters().unwrap().value.unwrap();
    roster[CORISANDE]["pinned"] = json!(true);
    s.save_characters(&roster).unwrap();
    let report = import(&act1);
    assert_eq!(
        report["characters"][0]["quests"]["fixture-caravan"],
        json!("skipped")
    );
    let kept = snapshot(&dir);
    assert_eq!(
        s.load_characters().unwrap().value.unwrap()[CORISANDE]["pinned"],
        json!(true)
    );
    import(&act1);
    assert_eq!(snapshot(&dir), kept);

    // A later session adds to the same character.
    let report = import(&session("party.txt"));
    assert_eq!(
        report["characters"].as_array().unwrap().len(),
        1,
        "{report}"
    );
    assert_eq!(report["characters"][0]["level"], json!(10));
    assert_eq!(
        report["characters"][0]["quests"]["fixture-beira"],
        json!("done"),
        "evidence from the first import is kept"
    );
}

#[test]
fn scan_reports_counts_per_area() {
    let store = tempfile::tempdir().unwrap();
    let data = quests_min();
    let scan = |log: &Path, extra: &[&str]| {
        let mut args = vec![
            OsStr::new("scan"),
            log.as_os_str(),
            OsStr::new("--data"),
            data.as_os_str(),
            OsStr::new("--json"),
        ];
        args.extend(extra.iter().map(OsStr::new));
        let o = run(store.path(), args);
        let text = stdout(&o);
        (ok_json(&o), text)
    };
    let area = |report: &Value, code: &str| -> Value {
        report["areas"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["code"] == json!(code))
            .cloned()
            .unwrap_or_else(|| panic!("no area {code} in {report}"))
    };

    let (report, text) = scan(&session("act1.txt"), &[]);
    let codes: Vec<&str> = report["areas"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["code"].as_str().unwrap())
        .collect();
    assert_eq!(codes, ["G1_1", "G1_11", "G1_12", "G1_2", "G1_town"]);
    let town = area(&report, "G1_town");
    assert_eq!(town["name"], json!("Clearfell Encampment"));
    assert_eq!(town["entries"], json!(1));
    assert_eq!(town["speakers"], json!({"Renly": 1, "Una": 1}));
    assert_eq!(
        town["system_lines"],
        json!({"Fixture system line: a lute string hums.": 1})
    );
    let clearfell = area(&report, "G1_2");
    assert_eq!(clearfell["name"], json!("Clearfell"));
    assert_eq!(clearfell["reward_lines"], json!(1));
    assert_eq!(clearfell["rewards"], json!({"+10% to Cold Resistance": 1}));
    assert_eq!(
        clearfell["speakers"],
        json!({"Beira of the Rotten Pack": 1})
    );
    assert_eq!(area(&report, "G1_1")["name"], json!("The Riverbank"));
    assert_eq!(report["lines"], json!(31));
    // No character name and no chat, though the session has both.
    for needle in [CORISANDE, "Hollis_Varne", "Ottoline_Brack", "WTB", "chat"] {
        assert!(!text.contains(needle), "printed {needle:?}: {text}");
    }

    // Points lines by pool (Atlas points are not campaign points); this
    // fixture enters no area.
    let (report, _) = scan(&fixtures().join("logs").join("points.txt"), &[]);
    assert_eq!(report["areas"], json!([]));
    assert_eq!(
        report["outside_areas"]["points"],
        json!({"passive": 2, "weapon_set": 1})
    );

    // Entries count instances: the second session of party.txt re-enters
    // areas and its party lines name nobody in the output.
    let (report, text) = scan(&session("party.txt"), &[]);
    assert_eq!(area(&report, "G1_2")["entries"], json!(1));
    assert_eq!(area(&report, "G1_11")["entries"], json!(1));
    assert_eq!(area(&report, "G1_11")["name"], json!("Hunting Grounds"));
    for needle in ["Bram_Holloway", "Wren_Ashdown", CORISANDE] {
        assert!(!text.contains(needle), "printed {needle:?}: {text}");
    }

    // --speakers adds names to count; without it the same line is not
    // counted.
    let chat = fixtures().join("logs").join("chat.txt");
    let (without, _) = scan(&chat, &[]);
    assert_eq!(without["outside_areas"]["speakers"], json!({}));
    let (with, _) = scan(&chat, &["--speakers", "Pellwick_Dunmore"]);
    assert_eq!(
        with["outside_areas"]["speakers"],
        json!({"Pellwick_Dunmore": 1})
    );

    // The plain text form lists the same areas.
    let o = run(
        store.path(),
        [
            OsStr::new("scan"),
            session("act1.txt").as_os_str(),
            OsStr::new("--data"),
            quests_min().as_os_str(),
        ],
    );
    let text = stdout(&o);
    assert!(
        text.contains("G1_town  Clearfell Encampment  entries 1"),
        "{text}"
    );
    assert!(text.contains("speaker Renly: 1"), "{text}");
}
