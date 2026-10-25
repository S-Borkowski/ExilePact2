//! Baseline and target for the line parser, over the synthetic lines in
//! `fixtures/logs/`. Baseline: chat, local chat shaped like an NPC line and
//! every unrecognised line produce no event. Target: one event per recognised
//! shape, every field exact. All names in the fixtures are invented.

use std::path::PathBuf;

use qt_logline::{parse_line, Allowlist, Event, EventKind, PartyChange, PointPool, Timestamp};

/// Reads a fixture the way the tailer hands lines over: bytes decoded as UTF-8
/// lossily, split on `\n`. A checkout that converted the file to CRLF leaves
/// a `\r` on each line, which the parser must tolerate.
fn fixture(name: &str) -> Vec<String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/logs")
        .join(name);
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let mut lines: Vec<String> = text.split('\n').map(str::to_string).collect();
    if lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    lines
}

fn allow() -> Allowlist {
    Allowlist::new(
        ["Renly", "Una", "The King in the Mists", "Sekhema Asala"],
        ["Half sail!", "All stop!"],
    )
}

/// Parses every line of a fixture, keeping `None`s so positions line up.
fn parse_fixture(name: &str) -> Vec<Option<Event>> {
    let allow = allow();
    fixture(name)
        .iter()
        .map(|line| parse_line(line, &allow))
        .collect()
}

fn kinds(events: &[Option<Event>]) -> Vec<Option<EventKind>> {
    events
        .iter()
        .map(|e| e.as_ref().map(|e| e.kind.clone()))
        .collect()
}

fn ts(text: &str) -> Timestamp {
    Timestamp::parse(text).expect("test timestamp")
}

fn s(text: &str) -> String {
    text.to_string()
}

#[test]
fn parses_area_generation() {
    let events = parse_fixture("area.txt");
    assert_eq!(
        events[1],
        Some(Event {
            time: ts("2026/03/07 10:00:02"),
            kind: EventKind::AreaEntered {
                code: s("G1_1"),
                level: 1,
                seed: 2147003311
            },
        })
    );
    assert_eq!(
        events[4].as_ref().map(|e| &e.kind),
        Some(&EventKind::AreaEntered {
            code: s("G1_town"),
            level: 15,
            seed: 1
        })
    );
    // `Connecting to instance server` comes before every join but is not the
    // join itself.
    assert_eq!(events[0], None);
}

#[test]
fn parses_scene_name_and_skips_null() {
    let got = kinds(&parse_fixture("area.txt"));
    let name = |n: &str| Some(EventKind::AreaName { name: s(n) });
    assert_eq!(got[2], None, "(null) is not an area name");
    assert_eq!(got[3], name("The Riverbank"));
    assert_eq!(got[5], None, "(unknown) is not an area name");
    assert_eq!(got[6], None, "Act <n> is not an area name");
    assert_eq!(got[7], name("Clearfell Encampment"));
    assert_eq!(
        got[8],
        name("Sel Khari Sanctuary"),
        "trailing space stripped"
    );
}

#[test]
fn ignores_chat_prefixes() {
    let lines = fixture("chat.txt");
    let allow = allow();
    // Global, trade, party, whisper and guild, including a player who named
    // themselves after an allowlisted NPC.
    for line in &lines[..6] {
        assert_eq!(parse_line(line, &allow), None, "{line}");
    }
    // The same messages with the prefix removed would be NPC-shaped; the
    // prefix alone is what drops them, even with the speaker allowlisted.
    let unprefixed = lines[5].replacen("] #Renly: ", "] Renly: ", 1);
    assert!(
        parse_line(&unprefixed, &allow).is_some(),
        "control: {unprefixed}"
    );
}

#[test]
fn ignores_speaker_not_on_allowlist() {
    let got = kinds(&parse_fixture("chat.txt"));
    // Every line in the chat fixture is chat, a client notice shaped like an
    // NPC line, or otherwise unrecognised: none produces an event.
    assert_eq!(got, vec![None; got.len()]);
    assert_eq!(got.len(), 16);
    // An NPC that the data does not list is treated like a player.
    assert_eq!(kinds(&parse_fixture("npc.txt"))[3], None);
    // With an empty allowlist even a real NPC's line is nothing.
    let empty = Allowlist::default();
    for line in fixture("npc.txt") {
        assert_eq!(parse_line(&line, &empty), None, "{line}");
    }
}

#[test]
fn parses_allowlisted_npc_line() {
    let events = parse_fixture("npc.txt");
    assert_eq!(
        events[0],
        Some(Event {
            time: ts("2026/03/07 10:03:00"),
            kind: EventKind::NpcLine {
                speaker: s("Renly"),
                text: s("Fixture line for the smith, nothing he really says."),
            },
        })
    );
    // A speaker name with spaces; the text keeps its own later colon.
    assert_eq!(
        events[1].as_ref().map(|e| &e.kind),
        Some(&EventKind::NpcLine {
            speaker: s("The King in the Mists"),
            text: s("A fixture boss line: with a colon inside."),
        })
    );
    // Speakerless lines on the system_lines allowlist.
    assert_eq!(
        events[2].as_ref().map(|e| &e.kind),
        Some(&EventKind::SystemLine {
            text: s("Half sail!")
        })
    );
    assert_eq!(
        events[4].as_ref().map(|e| &e.kind),
        Some(&EventKind::SystemLine {
            text: s("All stop!")
        })
    );
}

#[test]
fn parses_level_up_slain_and_received() {
    let events = parse_fixture("system_messages.txt");
    assert_eq!(
        events[0],
        Some(Event {
            time: ts("2026/03/07 10:04:00"),
            kind: EventKind::LevelUp {
                name: s("Tessaly_Vane"),
                class: s("Witch"),
                level: 12
            },
        })
    );
    let rest = kinds(&events[1..]);
    assert_eq!(
        rest,
        vec![
            Some(EventKind::Slain {
                name: s("Tessaly_Vane")
            }),
            Some(EventKind::Received {
                name: s("Tessaly_Vane"),
                stat: s("+30 to Spirit")
            }),
            Some(EventKind::Received {
                name: s("Tessaly_Vane"),
                stat: s("+20 to maximum Life")
            }),
            Some(EventKind::Lost {
                name: s("Tessaly_Vane"),
                stat: s("25% increased Stun Threshold"),
            }),
            Some(EventKind::Party {
                name: s("Marrowfen_Oak"),
                change: PartyChange::Joined
            }),
            Some(EventKind::LevelUp {
                name: s("Marrowfen_Oak"),
                class: s("Warrior"),
                level: 71,
            }),
            Some(EventKind::Party {
                name: s("Marrowfen_Oak"),
                change: PartyChange::Left
            }),
        ]
    );
}

#[test]
fn strips_reward_markup() {
    let got = kinds(&parse_fixture("rewards.txt"));
    let received = |stat: &str| {
        Some(EventKind::Received {
            name: s("Tessaly_Vane"),
            stat: s(stat),
        })
    };
    assert_eq!(
        got,
        vec![
            received("+10% to Cold Resistance"),
            received("+1 Charm Slot"),
            received("30% increased Life Recovery from Flasks"),
            received("+5% to all Elemental Resistances"),
            received("30% increased Global Armour, Evasion and Energy Shield"),
        ]
    );
}

#[test]
fn parses_passive_point_lines() {
    let events = parse_fixture("points.txt");
    assert_eq!(
        events[0],
        Some(Event {
            time: ts("2026/03/07 10:06:00"),
            kind: EventKind::Points {
                pool: PointPool::Passive,
                amount: 2
            },
        })
    );
    assert_eq!(
        kinds(&events[1..]),
        vec![
            Some(EventKind::Points {
                pool: PointPool::WeaponSet,
                amount: 2
            }),
            Some(EventKind::Points {
                pool: PointPool::Passive,
                amount: 1
            }),
            // Atlas points are endgame: not a campaign signal.
            None,
        ]
    );
    let json = serde_json::to_value(events[1].as_ref().unwrap()).unwrap();
    assert_eq!(
        json,
        serde_json::json!({
            "time": "2026/03/07 10:06:00",
            "kind": "points",
            "pool": "weapon_set",
            "amount": 2,
        })
    );
}

#[test]
fn parses_session_marker_and_client_version() {
    let events = parse_fixture("session.txt");
    assert_eq!(
        events,
        vec![
            Some(Event {
                time: ts("2026/03/07 09:59:50"),
                kind: EventKind::SessionStart
            }),
            Some(Event {
                time: ts("2026/03/07 09:59:51"),
                kind: EventKind::SessionStart
            }),
            Some(Event {
                time: ts("2026/03/07 09:59:52"),
                kind: EventKind::ClientVersion { tag: s("4.5.3") },
            }),
            None,
        ]
    );
    assert_eq!(
        serde_json::to_value(events[0].as_ref().unwrap()).unwrap(),
        serde_json::json!({ "time": "2026/03/07 09:59:50", "kind": "session_start" })
    );
}

#[test]
fn ignores_continuation_lines() {
    let got = kinds(&parse_fixture("continuation.txt"));
    let renly = |t: &str| {
        Some(EventKind::NpcLine {
            speaker: s("Renly"),
            text: s(t),
        })
    };
    let mut want = vec![None; got.len()];
    *want.last_mut().unwrap() = renly("The line after the warning is parsed again.");
    assert_eq!(got, want);
    assert_eq!(got.len(), 8);
}

#[test]
fn crlf_and_non_ascii() {
    let allow = allow();
    let lines = fixture("non_ascii.txt");
    let want = vec![
        Some(EventKind::NpcLine {
            speaker: s("Sekhema Asala"),
            text: s("Fixture text with \u{2018}curly quotes\u{2019}, an em dash \u{2014} and \u{201c}K\u{e4}rm\u{e4}\u{201d}."),
        }),
        Some(EventKind::AreaName { name: s("Fixture Grotto of \u{d6}lm\u{e9}") }),
        Some(EventKind::Received { name: s("Tessaly_Vane"), stat: s("+10% to Fire Resistance") }),
        None,
    ];
    for (line, want) in lines.iter().zip(&want) {
        let bare = line.trim_end_matches('\r');
        let crlf = format!("{bare}\r\n");
        let cr = format!("{bare}\r");
        for variant in [bare, crlf.as_str(), cr.as_str()] {
            let got = parse_line(variant, &allow).map(|e| e.kind);
            assert_eq!(&got, want, "{variant:?}");
        }
    }
    assert_eq!(lines.len(), want.len());

    // A UTF-8 sequence the tailer decoded lossily still parses, and the
    // replacement character stays in the text rather than failing the line.
    let lossy = String::from_utf8_lossy(
        b"2026/03/07 10:08:04 51685000 3ef2a224 [INFO Client 4120] Una: broken \xE2\x80 byte\r\n",
    )
    .into_owned();
    assert_eq!(
        parse_line(&lossy, &allow).map(|e| e.kind),
        Some(EventKind::NpcLine {
            speaker: s("Una"),
            text: s("broken \u{fffd} byte")
        })
    );
    // A byte-order mark before the first line of a file.
    let bom = format!(
        "\u{feff}{}",
        fixture("session.txt")[0].trim_end_matches('\r')
    );
    assert_eq!(
        parse_line(&bom, &allow).map(|e| e.kind),
        Some(EventKind::SessionStart)
    );
    // A multi-byte character where the timestamp should be is no line at all.
    assert_eq!(parse_line("2026/03/07 10:08:0\u{e9} x", &allow), None);
}

// -- the character build line (Context "Launch 7 fixes (owner scope)") -------

/// A client line at one level, built in the test (the name is invented).
fn client_line(level: &str, message: &str) -> String {
    format!("2026/03/14 12:00:02 9000400 abcdef02 [{level} Client 2120] {message}")
}

#[test]
fn parses_character_build_line() {
    let allow = allow();
    let got = parse_line(
        &client_line(
            "WARN",
            "Character Odile_Marsh has unknown build of path: None",
        ),
        &allow,
    );
    assert_eq!(
        got,
        Some(Event {
            time: ts("2026/03/14 12:00:02"),
            kind: EventKind::CharacterNamed {
                name: s("Odile_Marsh")
            },
        })
    );
    assert_eq!(
        serde_json::to_value(got.unwrap()).unwrap(),
        serde_json::json!({
            "time": "2026/03/14 12:00:02",
            "kind": "character_named",
            "name": "Odile_Marsh"
        })
    );
    // A trailing CR, as a CRLF checkout leaves it, is tolerated.
    let crlf = client_line(
        "WARN",
        "Character Odile_Marsh has unknown build of path: None\r",
    );
    assert!(matches!(
        parse_line(&crlf, &allow).map(|e| e.kind),
        Some(EventKind::CharacterNamed { name }) if name == "Odile_Marsh"
    ));
}

#[test]
fn ignores_other_warn_lines() {
    let allow = allow();
    // Positive control beside the negatives: the exact shape parses.
    let exact = client_line(
        "WARN",
        "Character Odile_Marsh has unknown build of path: None",
    );
    assert!(parse_line(&exact, &allow).is_some());
    for line in [
        // Another warning.
        client_line("WARN", "Something the fixture made up went wrong: code 7"),
        // The same words at another level.
        client_line(
            "INFO",
            "Character Odile_Marsh has unknown build of path: None",
        ),
        client_line(
            "DEBUG",
            "Character Odile_Marsh has unknown build of path: None",
        ),
        // A build path that is not None, and text before or after the shape.
        client_line(
            "WARN",
            "Character Odile_Marsh has unknown build of path: Mercenary",
        ),
        client_line(
            "WARN",
            "Character Odile_Marsh has unknown build of path: None.",
        ),
        client_line(
            "WARN",
            "Odile_Marsh: Character Odile_Marsh has unknown build of path: None",
        ),
        // A name with a space is not one character name.
        client_line(
            "WARN",
            "Character Odile Marsh has unknown build of path: None",
        ),
        // Someone typing the text in chat.
        client_line(
            "INFO",
            "#Bram_Holloway: Character Odile_Marsh has unknown build of path: None",
        ),
        // The warning's continuation line has no timestamp.
        "Character Odile_Marsh has unknown build of path: None".to_string(),
    ] {
        assert_eq!(parse_line(&line, &allow), None, "{line}");
    }
}
