//! Baseline and target for the quest engine, over the synthetic quest data in
//! `fixtures/quests_min.json` and the sessions in `fixtures/sessions/`.
//! Baseline: chat, trade and endgame lines leave every quest locked and no
//! character known. Target: after `act1.txt`, every fixture quest has the
//! status Context "Quest state rules" gives it. The rest pin down the
//! character detection rules, idempotence, overrides and the validator.
//! All character names are invented.

use std::path::PathBuf;
use std::sync::Arc;

use qt_engine::{
    area_from_log, CharacterNote, DataError, Detection, Engine, LogArea, QuestAction, QuestData,
    Status, Timestamp, Transition,
};
use serde_json::Value;

const CORISANDE: &str = "Corisande_Fell";
const ODILE: &str = "Odile_Marsh";

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

fn data_text() -> String {
    let path = fixtures().join("quests_min.json");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn data() -> Arc<QuestData> {
    Arc::new(QuestData::parse(&data_text()).expect("the fixture data validates"))
}

/// A session's lines, split the way the tailer hands them over.
fn session(name: &str) -> Vec<String> {
    let path = fixtures().join("sessions").join(name);
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let mut lines: Vec<String> = text.split('\n').map(str::to_string).collect();
    if lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    lines
}

fn feed(engine: &mut Engine, lines: &[String], live: bool) -> Vec<Transition> {
    lines
        .iter()
        .flat_map(|l| engine.feed_line(l, live))
        .collect()
}

fn replay(name: &str) -> Engine {
    let mut engine = Engine::new(data());
    feed(&mut engine, &session(name), false);
    engine
}

fn status(engine: &Engine, character: &str, quest: &str) -> Status {
    engine
        .status(character, quest)
        .unwrap_or_else(|| panic!("no quest {quest}"))
}

fn names(engine: &Engine) -> Vec<String> {
    engine.characters().into_iter().map(|c| c.name).collect()
}

fn all_statuses(engine: &Engine, character: &str) -> Vec<(String, Status, Option<String>)> {
    engine
        .quest_states(character)
        .into_iter()
        .map(|q| (q.quest_id, q.status, q.current_step))
        .collect()
}

/// A timestamped client line around a message, for the inline cases.
fn line(message: &str) -> String {
    format!("2026/03/14 12:00:00 9000000 abcdef01 [INFO Client 2120] {message}")
}

#[test]
fn baseline_irrelevant_lines_change_nothing() {
    let fresh = Engine::new(data());
    let mut engine = Engine::new(data());
    let transitions = feed(&mut engine, &session("baseline.txt"), true);
    assert!(transitions.is_empty(), "{transitions:?}");
    assert!(engine.characters().is_empty(), "no character known");
    assert_eq!(engine.current(), None);
    assert_eq!(engine.to_json(), fresh.to_json());
    for q in &engine.data().quests {
        assert_eq!(
            status(&engine, CORISANDE, &q.id),
            Status::Locked,
            "{}",
            q.id
        );
    }

    // The same lines after a real session change nothing for its character.
    let mut played = replay("act1.txt");
    let before = played.to_json();
    let before_statuses = all_statuses(&played, CORISANDE);
    let transitions = feed(&mut played, &session("baseline.txt"), true);
    assert!(transitions.is_empty(), "{transitions:?}");
    assert_eq!(played.to_json(), before);
    assert_eq!(all_statuses(&played, CORISANDE), before_statuses);
}

#[test]
fn target_act1_fixture_states() {
    let engine = replay("act1.txt");
    let chars = engine.characters();
    assert_eq!(chars.len(), 1, "{chars:?}");
    let c = &chars[0];
    assert_eq!(
        (
            c.name.as_str(),
            c.class.as_str(),
            c.level,
            c.own,
            c.active,
            c.pinned
        ),
        (CORISANDE, "Witch", 5, true, true, false)
    );
    assert_eq!(engine.client_version(), Some("4.5.3"));
    assert_eq!(engine.held_count(), 0);

    let step = |s: &str| Some(s.to_string());
    let want = vec![
        (
            "fixture-riverbank".to_string(),
            Status::Done,
            step("reach-encampment"),
        ),
        (
            "fixture-beira".to_string(),
            Status::Done,
            step("meet-beira"),
        ),
        ("fixture-crowbell".to_string(), Status::Available, None),
        (
            "fixture-king".to_string(),
            Status::InProgress,
            step("enter-freythorn"),
        ),
        (
            "fixture-lute".to_string(),
            Status::InProgress,
            step("hear-the-lute"),
        ),
        ("fixture-candles".to_string(), Status::Unknown, None),
        ("fixture-caravan".to_string(), Status::Locked, None),
    ];
    assert_eq!(all_statuses(&engine, CORISANDE), want);

    // The area lines before the first level-up were held and given to it.
    let riverbank = engine.quest_state(CORISANDE, "fixture-riverbank").unwrap();
    assert!(
        riverbank.steps.iter().all(|s| s.fired && !s.ticked),
        "{riverbank:?}"
    );
}

#[test]
fn chunked_live_equals_whole_replay() {
    let lines = session("act1.txt");
    let whole = replay("act1.txt");
    let split = lines.len() / 2;
    for chunk in [1, 2, 3, 5, 8, lines.len()] {
        let mut engine = Engine::new(data());
        let mut transitions = Vec::new();
        for (i, part) in lines.chunks(chunk).enumerate() {
            // The tailer's catch-up lines first, then live ones.
            let live = i * chunk >= split;
            transitions.extend(feed(&mut engine, part, live));
        }
        assert_eq!(engine.to_json(), whole.to_json(), "chunk {chunk}");
        assert_eq!(
            all_statuses(&engine, CORISANDE),
            all_statuses(&whole, CORISANDE),
            "chunk {chunk}"
        );
        if chunk < lines.len() {
            // Done by a live line (the cold resistance reward) is live.
            let beira = transitions
                .iter()
                .find(|t| t.quest_id == "fixture-beira" && t.to == Status::Done)
                .expect("beira transition");
            assert!(beira.live, "chunk {chunk}: {beira:?}");
        }
        // The riverbank's first step fired in the catch-up part.
        let first = transitions
            .iter()
            .find(|t| t.quest_id == "fixture-riverbank")
            .expect("riverbank transition");
        assert!(!first.live, "chunk {chunk}: {first:?}");
    }
}

#[test]
fn applying_events_twice_is_idempotent() {
    let lines = session("act1.txt");
    let once = replay("act1.txt");

    let mut doubled = Engine::new(data());
    for l in &lines {
        doubled.feed_line(l, false);
        doubled.feed_line(l, false);
    }
    assert_eq!(doubled.to_json(), once.to_json());
    assert_eq!(
        all_statuses(&doubled, CORISANDE),
        all_statuses(&once, CORISANDE)
    );

    // The whole log again (a restart that re-reads from the start).
    let mut again = replay("act1.txt");
    let transitions = feed(&mut again, &lines, false);
    assert_eq!(again.to_json(), once.to_json());
    assert_eq!(
        all_statuses(&again, CORISANDE),
        all_statuses(&once, CORISANDE)
    );
    let finals: Vec<_> = transitions
        .iter()
        .filter(|t| t.quest_id == "fixture-riverbank" && t.to == Status::Done)
        .collect();
    assert!(
        finals.is_empty(),
        "nothing new is done the second time: {finals:?}"
    );
}

#[test]
fn received_line_marks_own_character() {
    let mut engine = Engine::new(data());
    engine.feed_line(&line(": Odile_Marsh (Mercenary) is now level 3"), false);
    let c = &engine.characters()[0];
    assert_eq!((c.name.as_str(), c.own, c.active), (ODILE, false, true));

    engine.feed_line(
        &line(": Odile_Marsh has received +20 to maximum Life."),
        false,
    );
    let c = &engine.characters()[0];
    assert_eq!((c.name.as_str(), c.own, c.active), (ODILE, true, true));

    // A received line alone makes a new name own and gives it the event.
    let mut engine = Engine::new(data());
    engine.feed_line(
        &line(r#"Generating level 2 area "G1_2" with seed 7"#),
        false,
    );
    engine.feed_line(
        &line(": Corisande_Fell has received +10% to [Resistances|Cold Resistance]."),
        false,
    );
    let c = &engine.characters()[0];
    assert_eq!(
        (c.name.as_str(), c.own, c.class.as_str(), c.level),
        (CORISANDE, true, "", 0)
    );
    assert_eq!(status(&engine, CORISANDE, "fixture-beira"), Status::Done);

    // A death and a level-up of someone in a join line make nobody own.
    let mut engine = Engine::new(data());
    engine.feed_line(&line(": Bram_Holloway has joined the area."), false);
    engine.feed_line(&line(": Bram_Holloway (Warrior) is now level 71"), false);
    engine.feed_line(&line(": Bram_Holloway has been slain."), false);
    assert!(engine.characters().is_empty());
}

#[test]
fn party_member_level_up_does_not_switch() {
    let lines = session("party.txt");
    let mut engine = Engine::new(data());
    // Up to the party member's level-up: the own character stays active.
    feed(&mut engine, &lines[..7], false);
    assert_eq!(engine.current(), Some(CORISANDE));
    assert_eq!(names(&engine), [CORISANDE]);
    assert_eq!(
        status(&engine, CORISANDE, "fixture-beira"),
        Status::Available
    );

    // An unknown name never seen joining levels up: active, unconfirmed.
    feed(&mut engine, &lines[7..12], false);
    assert_eq!(engine.current(), Some("Wren_Ashdown"));
    let wren = engine
        .characters()
        .into_iter()
        .find(|c| c.name == "Wren_Ashdown");
    assert!(wren.is_some_and(|c| !c.own));
    assert_eq!(
        status(&engine, "Wren_Ashdown", "fixture-crowbell"),
        Status::Available
    );

    // It leaves: not active, not stored, its events back to the hold, and
    // the next own level-up takes them.
    feed(&mut engine, &lines[12..13], false);
    assert_eq!(engine.current(), None);
    assert_eq!(names(&engine), [CORISANDE]);
    assert_eq!(engine.held_count(), 2);
    assert_eq!(
        status(&engine, CORISANDE, "fixture-crowbell"),
        Status::Locked
    );

    feed(&mut engine, &lines[13..], false);
    assert_eq!(engine.current(), Some(CORISANDE));
    assert_eq!(engine.held_count(), 0);
    assert_eq!(
        status(&engine, CORISANDE, "fixture-crowbell"),
        Status::Available
    );
    assert_eq!(engine.character(CORISANDE).unwrap().level, 10);

    let saved = engine.to_json().to_string();
    assert!(!saved.contains("Bram_Holloway"), "{saved}");
    assert!(!saved.contains("Wren_Ashdown"), "{saved}");
}

#[test]
fn held_events_go_to_first_identified_character() {
    let engine = replay("held.txt");
    // The first session's level-1 Riverbank entry started a new character
    // that no level-up named: parked with its two events, not dropped.
    assert_eq!(engine.dropped_count(), 0);
    assert_eq!(engine.dropped_new_character_count(), 0);
    let parked = engine.new_character_json().expect("parked");
    assert_eq!(
        (parked["open"].as_bool(), parked["events"].as_u64()),
        (Some(false), Some(2))
    );
    assert_eq!(engine.held_count(), 0);
    // The second session's held events went to the first character
    // identified, not to the party member who levelled up first.
    assert_eq!(names(&engine), [ODILE]);
    let c = &engine.characters()[0];
    assert_eq!((c.own, c.active, c.level), (false, true, 4));
    let riverbank = engine.quest_state(ODILE, "fixture-riverbank").unwrap();
    assert_eq!(riverbank.status, Status::InProgress);
    assert_eq!(riverbank.current_step.as_deref(), Some("reach-encampment"));
    assert!(
        !riverbank.steps[0].fired,
        "the parked new character gave nothing"
    );

    // A session ending with nothing identified drops what it held.
    let mut engine = Engine::new(data());
    let lines = session("held.txt");
    feed(&mut engine, &lines[..6], false);
    assert_eq!(engine.held_count(), 2);
    engine.end_session();
    assert_eq!((engine.held_count(), engine.dropped_count()), (0, 2));
    assert!(engine.characters().is_empty());
}

#[test]
fn pinned_character_overrides_detection() {
    // Unpinned control: the switch goes to the second character.
    let unpinned = replay("switch.txt");
    assert_eq!(names(&unpinned), [CORISANDE, ODILE]);
    assert_eq!(status(&unpinned, ODILE, "fixture-beira"), Status::Done);
    assert_eq!(
        status(&unpinned, CORISANDE, "fixture-beira"),
        Status::Locked
    );
    assert_eq!(unpinned.current(), Some(CORISANDE));

    let lines = session("switch.txt");
    let mut engine = Engine::new(data());
    feed(&mut engine, &lines[..3], false);
    assert!(engine.pin(Some("Nobody_Here")).is_err());
    engine.pin(Some(CORISANDE)).unwrap();
    feed(&mut engine, &lines[3..], false);
    assert_eq!(engine.current(), Some(CORISANDE));
    assert_eq!(engine.pinned(), Some(CORISANDE));
    assert_eq!(
        names(&engine),
        [CORISANDE],
        "detection did not add the other name"
    );
    assert_eq!(status(&engine, CORISANDE, "fixture-beira"), Status::Done);
    assert_eq!(
        status(&engine, CORISANDE, "fixture-riverbank"),
        Status::InProgress
    );
    assert_eq!(
        engine.character(CORISANDE).unwrap().level,
        8,
        "its level still updates"
    );
    assert!(engine.characters()[0].pinned);

    // The pin persists; unpinning hands events back to detection.
    let restored = Engine::from_json(data(), &engine.to_json()).unwrap();
    assert_eq!(restored.pinned(), Some(CORISANDE));
    engine.pin(None).unwrap();
    assert_eq!(engine.pinned(), None);
    engine.feed_line(&line(": Odile_Marsh (Mercenary) is now level 3"), false);
    assert_eq!(engine.current(), Some(ODILE));
}

#[test]
fn manual_overrides_survive_replay() {
    let lines = session("act1.txt");
    let mut engine = replay("act1.txt");
    let t = engine
        .quest_action(CORISANDE, "fixture-riverbank", QuestAction::UnmarkDone)
        .unwrap();
    assert_eq!(t.len(), 1);
    assert_eq!(
        (t[0].from, t[0].to, t[0].live),
        (Status::Done, Status::InProgress, true)
    );
    engine
        .quest_action(CORISANDE, "fixture-crowbell", QuestAction::Skip)
        .unwrap();
    engine
        .tick_step(CORISANDE, "fixture-lute", "return-the-lute", true)
        .unwrap();
    engine
        .quest_action(CORISANDE, "fixture-candles", QuestAction::MarkDone)
        .unwrap();
    assert!(engine
        .quest_action(CORISANDE, "no-such-quest", QuestAction::Skip)
        .is_err());
    assert!(engine
        .tick_step(CORISANDE, "fixture-lute", "no-such-step", true)
        .is_err());

    let expect = |e: &Engine| {
        assert_eq!(
            status(e, CORISANDE, "fixture-riverbank"),
            Status::InProgress
        );
        assert_eq!(status(e, CORISANDE, "fixture-crowbell"), Status::Skipped);
        assert_eq!(status(e, CORISANDE, "fixture-candles"), Status::Done);
        let lute = e.quest_state(CORISANDE, "fixture-lute").unwrap();
        assert_eq!(lute.status, Status::InProgress);
        assert_eq!(lute.current_step.as_deref(), Some("return-the-lute"));
    };
    expect(&engine);

    // Through the store's JSON, both whole and per character, then the
    // whole log again.
    let saved = engine.to_json();
    let mut restored = Engine::from_json(data(), &saved).unwrap();
    feed(&mut restored, &lines, false);
    expect(&restored);
    assert_eq!(restored.to_json(), saved);

    let mut fresh = Engine::new(data());
    let one: Value = engine.character_json(CORISANDE).unwrap();
    fresh.load_character(&one).unwrap();
    feed(&mut fresh, &lines, false);
    expect(&fresh);

    // Reset clears the overrides, not the evidence.
    restored
        .quest_action(CORISANDE, "fixture-riverbank", QuestAction::Reset)
        .unwrap();
    restored
        .quest_action(CORISANDE, "fixture-crowbell", QuestAction::Unskip)
        .unwrap();
    assert_eq!(
        status(&restored, CORISANDE, "fixture-riverbank"),
        Status::Done
    );
    assert_eq!(
        status(&restored, CORISANDE, "fixture-crowbell"),
        Status::Available
    );
}

#[test]
fn manual_quest_becomes_unknown_after_window() {
    let lines = session("manual_window.txt");
    let mut engine = Engine::new(data());
    feed(&mut engine, &lines[..3], false);
    assert_eq!(
        status(&engine, CORISANDE, "fixture-candles"),
        Status::Locked
    );
    feed(&mut engine, &lines[3..6], false);
    assert_eq!(
        status(&engine, CORISANDE, "fixture-candles"),
        Status::Available
    );
    let t = feed(&mut engine, &lines[6..], true);
    assert!(
        t.iter().any(|t| t.quest_id == "fixture-candles"
            && t.from == Status::Available
            && t.to == Status::Unknown
            && t.live),
        "{t:?}"
    );
    assert_eq!(
        status(&engine, CORISANDE, "fixture-candles"),
        Status::Unknown
    );

    // The player says what happened.
    engine
        .quest_action(CORISANDE, "fixture-candles", QuestAction::MarkDone)
        .unwrap();
    assert_eq!(status(&engine, CORISANDE, "fixture-candles"), Status::Done);
    engine
        .quest_action(CORISANDE, "fixture-candles", QuestAction::UnmarkDone)
        .unwrap();
    assert_eq!(
        status(&engine, CORISANDE, "fixture-candles"),
        Status::Unknown
    );

    // Control: the same quest marked `auto` is not made unknown by its window.
    let mut auto = (*data()).clone();
    auto.quests
        .iter_mut()
        .find(|q| q.id == "fixture-candles")
        .unwrap()
        .detection = Detection::Auto;
    let mut engine = Engine::new(Arc::new(auto));
    feed(&mut engine, &lines, false);
    assert_eq!(
        status(&engine, CORISANDE, "fixture-candles"),
        Status::Available
    );
}

#[test]
fn area_name_signal_matches_scene_name() {
    let start = [
        line(": Corisande_Fell has received +20 to maximum Life."),
        line(r#"Generating level 15 area "G1_town" with seed 1"#),
    ];
    let step = |engine: &Engine| {
        engine
            .quest_state(CORISANDE, "fixture-riverbank")
            .unwrap()
            .steps
            .iter()
            .find(|s| s.id == "reach-encampment")
            .unwrap()
            .fired
    };

    // The area code alone, the placeholder names and another area's name
    // do not fire the name signal.
    let mut engine = Engine::new(data());
    for l in start.iter().chain(&[
        line("[SCENE] Set Source [(null)]"),
        line("[SCENE] Set Source [Act 1]"),
        line("[SCENE] Set Source [Clearfell]"),
        line("[SCENE] Set Source [clearfell encampment]"),
    ]) {
        engine.feed_line(l, false);
    }
    assert!(!step(&engine));
    assert_eq!(
        status(&engine, CORISANDE, "fixture-riverbank"),
        Status::Available
    );

    // The display name fires it, trailing space and all.
    engine.feed_line(&line("[SCENE] Set Source [Clearfell Encampment ]"), false);
    assert!(step(&engine));
    assert_eq!(
        status(&engine, CORISANDE, "fixture-riverbank"),
        Status::InProgress
    );
}

#[test]
fn validator_rejects_malformed_data() {
    let base: Value = serde_json::from_str(&data_text()).unwrap();
    QuestData::from_value(&base).expect("the fixture is valid");

    type Mutate = fn(&mut Value);
    let cases: Vec<(&str, Option<&str>, &str, Mutate)> = vec![
        ("duplicate id", Some("fixture-riverbank"), "id", |v| {
            v["quests"][1]["id"] = "fixture-riverbank".into()
        }),
        ("id not kebab-case", Some("Fixture_Beira"), "id", |v| {
            v["quests"][1]["id"] = "Fixture_Beira".into()
        }),
        ("worth outside enum", Some("fixture-beira"), "worth", |v| {
            v["quests"][1]["worth"] = "maybe".into()
        }),
        (
            "detection outside enum",
            Some("fixture-lute"),
            "detection",
            |v| v["quests"][4]["detection"] = "sometimes".into(),
        ),
        (
            "reward category outside enum",
            Some("fixture-crowbell"),
            "rewards[0].category",
            |v| v["quests"][2]["rewards"][0]["category"] = "gold".into(),
        ),
        (
            "resistance without element",
            Some("fixture-beira"),
            "rewards[0].element",
            |v| {
                v["quests"][1]["rewards"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove("element");
            },
        ),
        ("bad source url", Some("fixture-king"), "source_url", |v| {
            v["quests"][3]["source_url"] = "http://example.com/king".into()
        }),
        ("bad fetch date", Some("fixture-king"), "fetched", |v| {
            v["quests"][3]["fetched"] = "2026-13-01".into()
        }),
        (
            "unknown signal kind",
            Some("fixture-crowbell"),
            "unlock[0].kind",
            |v| v["quests"][2]["unlock"][0]["kind"] = "telepathy".into(),
        ),
        (
            "unknown signal field",
            Some("fixture-crowbell"),
            "done[0].colour",
            |v| v["quests"][2]["done"][0]["colour"] = "red".into(),
        ),
        (
            "quest signal naming no quest",
            Some("fixture-king"),
            "unlock[0].id",
            |v| v["quests"][3]["unlock"][0]["id"] = "no-such-quest".into(),
        ),
        (
            "npc speaker not allowlisted",
            Some("fixture-beira"),
            "steps[0].signals[0].speaker",
            |v| v["quests"][1]["steps"][0]["signals"][0]["speaker"] = "Nobody".into(),
        ),
        (
            "system line not allowlisted",
            Some("fixture-lute"),
            "steps[0].signals[0].line",
            |v| v["quests"][4]["steps"][0]["signals"][0]["line"] = "Not on the list.".into(),
        ),
        (
            "area code not in areas",
            Some("fixture-riverbank"),
            "steps[0].signals[0].code",
            |v| v["quests"][0]["steps"][0]["signals"][0]["code"] = "G9_99".into(),
        ),
        (
            "signal area not in areas",
            Some("fixture-beira"),
            "done[0].area",
            |v| v["quests"][1]["done"][0]["area"] = "G9_99".into(),
        ),
        (
            "unknown quest field",
            Some("fixture-caravan"),
            "colour",
            |v| v["quests"][6]["colour"] = "red".into(),
        ),
        (
            "missing quest field",
            Some("fixture-caravan"),
            "steps",
            |v| {
                v["quests"][6].as_object_mut().unwrap().remove("steps");
            },
        ),
        ("wrong schema", None, "schema", |v| v["schema"] = 2.into()),
    ];
    for (what, quest, field, mutate) in cases {
        let mut v = base.clone();
        mutate(&mut v);
        let err: DataError = QuestData::from_value(&v)
            .err()
            .unwrap_or_else(|| panic!("{what}: accepted"));
        assert_eq!(err.quest.as_deref(), quest, "{what}: {err}");
        assert_eq!(err.field, field, "{what}: {err}");
        let text = err.to_string();
        assert!(text.contains(field), "{what}: {text}");
        if let Some(q) = quest {
            assert!(text.contains(q), "{what}: {text}");
        }
    }

    // Unparseable text is refused too, without a panic.
    assert!(QuestData::parse("{ not json").is_err());
}

// -- the area from the log before the start, and a counted refusal ----------

const MARKER: &str = "2026/03/14 11:59:00 ***** LOG FILE OPENING *****";
const COLD_REWARD: &str = ": Corisande_Fell has received +10% to [Resistances|Cold Resistance].";

fn area_line(code: &str) -> String {
    line(&format!(r#"Generating level 2 area "{code}" with seed 7"#))
}

#[test]
fn last_area_before_offset_is_found() {
    // The last area line wins; chat, NPC and reward lines after it do not
    // change it, and a marker before it does not either.
    let before = vec![
        MARKER.to_string(),
        area_line("G1_1"),
        line("Renly: Fixture line: welcome to the encampment."),
        area_line("G1_2"),
        line("#Bram_Holloway: anyone selling G1_11 maps"),
        line(": Odile_Marsh (Mercenary) is now level 3"),
    ];
    let found = area_from_log(&before);
    assert_eq!(found, LogArea::Area("G1_2".into()));
    assert_eq!(found.code(), Some("G1_2"));

    // Owned strings and string slices are both accepted, oldest first.
    let slices: Vec<&str> = before.iter().map(String::as_str).collect();
    assert_eq!(area_from_log(slices), LogArea::Area("G1_2".into()));

    // A line cut at the read window's start is just not recognised.
    let cut = vec![
        r#"area "G1_11" with seed 7"#.to_string(),
        area_line("G1_town"),
    ];
    assert_eq!(area_from_log(&cut).code(), Some("G1_town"));
}

#[test]
fn last_area_stops_at_session_marker() {
    // A marker after the last area line: the game restarted, area unknown.
    let restarted = vec![
        area_line("G1_2"),
        line(": Corisande_Fell has been slain."),
        MARKER.to_string(),
    ];
    let found = area_from_log(&restarted);
    assert_eq!(found, LogArea::MarkerAfter);
    assert_eq!(found.code(), None);

    // An area line after the marker is the new session's area again.
    let mut again = restarted.clone();
    again.push(area_line("G1_11"));
    assert_eq!(area_from_log(&again), LogArea::Area("G1_11".into()));

    // No area line at all, with or without a marker.
    let none: Vec<String> = Vec::new();
    assert_eq!(area_from_log(&none), LogArea::NoAreaLine);
    assert_eq!(area_from_log(&[MARKER.to_string()]), LogArea::NoAreaLine);
    assert_eq!(area_from_log(&[line(COLD_REWARD)]).code(), None);
}

#[test]
fn seeded_area_lets_area_tied_reward_count() {
    // Baseline: the tailer starts mid-zone in G1_2 and the engine never saw
    // the area line, so the area-tied reward is refused.
    let mut unseeded = Engine::new(data());
    assert_eq!(unseeded.current_area(), None);
    feed(&mut unseeded, &[line(COLD_REWARD)], true);
    assert_ne!(status(&unseeded, CORISANDE, "fixture-beira"), Status::Done);

    // Target: the area read back from the log before the start position.
    let before = vec![MARKER.to_string(), area_line("G1_town"), area_line("G1_2")];
    let mut engine = Engine::new(data());
    let fresh = engine.to_json();
    engine.seed_area(area_from_log(&before).code());

    // The seed sets the area and nothing else: no character, no evidence,
    // nothing persisted, no refusal.
    assert_eq!(engine.current_area(), Some("G1_2"));
    assert!(engine.characters().is_empty());
    assert_eq!(engine.to_json(), fresh);
    assert_eq!(engine.refused_count(), 0);

    let transitions = feed(&mut engine, &[line(COLD_REWARD)], true);
    assert_eq!(status(&engine, CORISANDE, "fixture-beira"), Status::Done);
    assert!(
        transitions
            .iter()
            .any(|t| t.quest_id == "fixture-beira" && t.to == Status::Done && t.live),
        "{transitions:?}"
    );
    assert_eq!(engine.refused_count(), 0);

    // A session marker after the seed clears it, as for an area read live.
    engine.feed_line(MARKER, true);
    assert_eq!(engine.current_area(), None);

    // Seeding "none" leaves (or makes) the area unknown.
    let mut engine = Engine::new(data());
    engine.seed_area(Some("G1_2"));
    engine.seed_area(area_from_log(&[area_line("G1_2"), MARKER.to_string()]).code());
    assert_eq!(engine.current_area(), None);
}

#[test]
fn unknown_area_refusal_is_counted() {
    let mut engine = Engine::new(data());
    assert_eq!(engine.refused_count(), 0);
    assert!(engine.refused_quests().is_empty());

    // Area unknown: the reward matches fixture-beira's done signal in all
    // but its area, so it is refused and counted once, with the quest id.
    feed(&mut engine, &[line(COLD_REWARD)], false);
    assert_ne!(status(&engine, CORISANDE, "fixture-beira"), Status::Done);
    assert_eq!(engine.refused_count(), 1);
    assert_eq!(engine.refused_quests(), ["fixture-beira".to_string()]);

    // A points line with the area unknown is fixture-crowbell's refusal.
    feed(
        &mut engine,
        &[line(": You have received 2 Passive Skill Points.")],
        false,
    );
    assert_eq!(engine.refused_count(), 2);
    assert_eq!(
        engine.take_refused(),
        vec!["fixture-beira".to_string(), "fixture-crowbell".to_string()]
    );
    assert!(engine.refused_quests().is_empty(), "taken");
    assert_eq!(engine.refused_count(), 2, "the count is cumulative");

    // Negative control: in a known, different area the same reward is the
    // normal refusal and is not counted.
    feed(&mut engine, &[area_line("G1_11"), line(COLD_REWARD)], false);
    assert_ne!(status(&engine, CORISANDE, "fixture-beira"), Status::Done);
    assert_eq!(engine.refused_count(), 2);
    assert!(engine.refused_quests().is_empty());

    // A line no area-tied signal matches in any area is not a refusal.
    feed(
        &mut engine,
        &[
            MARKER.to_string(),
            line(": Corisande_Fell has received +20 to maximum Life."),
        ],
        false,
    );
    assert_eq!(engine.current_area(), None);
    assert_eq!(engine.refused_count(), 2);

    // The count and the ids are session memory, never persisted.
    let saved = engine.to_json().to_string();
    assert!(!saved.contains("refused"), "{saved}");
}

// -- a new character from The Riverbank, and refusals only where they could
// matter ---------------------------------------------------------------------

const POINTS: &str = ": You have received 2 Passive Skill Points.";

/// The seed of the Riverbank instance the inline cases enter.
const RIVERBANK_SEED: u64 = 1849302211;

/// The level-1 entry to The Riverbank a new character starts with.
fn riverbank_entry() -> String {
    line(&format!(
        r#"Generating level 1 area "G1_1" with seed {RIVERBANK_SEED}"#
    ))
}

/// The walk from The Riverbank to the encampment.
fn to_the_encampment() -> Vec<String> {
    vec![
        line("[SCENE] Set Source [The Riverbank]"),
        line(r#"Generating level 15 area "G1_town" with seed 1"#),
        line("[SCENE] Set Source [Clearfell Encampment]"),
    ]
}

fn level_up(name: &str, class: &str, level: u32) -> String {
    line(&format!(": {name} ({class}) is now level {level}"))
}

/// A log time of 2026/03/14, the day every inline line is written on.
fn at(time: &str) -> Timestamp {
    Timestamp::parse(&format!("2026/03/14 {time}")).unwrap()
}

/// Closes the relog window of an entry written by `line` (12:00:00) the way
/// the shell's worker tick does when the log stays quiet: 6 s on.
fn expire(engine: &mut Engine) -> Vec<Transition> {
    engine.expire_relog_window(at("12:00:06"))
}

#[test]
fn riverbank_level1_entry_starts_new_character() {
    // Negative control: an entry to G1_1 at another area level, and a
    // level-1 entry to another area, start nothing; their events are held.
    let mut engine = Engine::new(data());
    feed(
        &mut engine,
        &[
            MARKER.to_string(),
            area_line("G1_1"),
            line(r#"Generating level 1 area "G1_2" with seed 7"#),
        ],
        true,
    );
    assert!(!engine.new_character_open());
    assert!(engine.new_character().is_none());
    assert!(engine.new_character_states().is_none());
    assert_eq!(engine.held_count(), 2);

    // Target: a level-1 entry to The Riverbank starts a new character with
    // no name, level 1, once its relog window has closed (here, the walk to
    // the encampment is another area entry); the window's events go to it.
    let mut engine = Engine::new(data());
    let mut lines = vec![MARKER.to_string(), riverbank_entry()];
    lines.extend(to_the_encampment());
    let mut transitions = feed(&mut engine, &lines[..3], true);
    assert!(engine.relog_window_open());
    assert!(!engine.new_character_open());
    assert!(engine.take_character_notes().is_empty());
    transitions.extend(feed(&mut engine, &lines[3..], true));
    assert!(!engine.relog_window_open());
    assert!(engine.new_character_open());
    assert_eq!(engine.current(), None);
    assert!(engine.characters().is_empty());
    assert_eq!(engine.held_count(), 0);
    let nc = engine.new_character().expect("open");
    assert_eq!((nc.name.as_str(), nc.class.as_str(), nc.level), ("", "", 1));
    let states = engine.new_character_states().expect("open");
    let riverbank = states
        .iter()
        .find(|q| q.quest_id == "fixture-riverbank")
        .unwrap();
    assert_eq!(riverbank.status, Status::InProgress);
    assert_eq!(riverbank.current_step.as_deref(), Some("reach-encampment"));
    assert!(
        transitions.iter().any(|t| t.character.is_empty()
            && t.quest_id == "fixture-riverbank"
            && t.from == Status::Available
            && t.to == Status::InProgress
            && t.live),
        "{transitions:?}"
    );

    // Renly's welcome finishes the quest for it, and a second level-1 entry
    // while it is open keeps its progress.
    feed(
        &mut engine,
        &[
            line("Renly: Fixture line: welcome to the encampment."),
            riverbank_entry(),
        ],
        true,
    );
    let states = engine.new_character_states().unwrap();
    assert_eq!(states[0].quest_id, "fixture-riverbank");
    assert_eq!(states[0].status, Status::Done);
}

#[test]
fn new_character_named_by_first_level_up_without_pick() {
    let mut engine = Engine::new(data());
    let mut lines = vec![MARKER.to_string(), riverbank_entry()];
    lines.extend(to_the_encampment());
    feed(&mut engine, &lines, true);
    let shown = engine.new_character_states().unwrap();

    // The first level-up names it: own and active with no pick, stored,
    // with the progress it showed, and no transition re-reports that.
    let transitions = feed(&mut engine, &[level_up(ODILE, "Mercenary", 2)], true);
    assert!(!engine.new_character_open());
    assert_eq!(engine.current(), Some(ODILE));
    let chars = engine.characters();
    assert_eq!(chars.len(), 1, "{chars:?}");
    let c = &chars[0];
    assert_eq!(
        (
            c.name.as_str(),
            c.class.as_str(),
            c.level,
            c.own,
            c.active,
            c.pinned
        ),
        (ODILE, "Mercenary", 2, true, true, false)
    );
    assert_eq!(engine.quest_states(ODILE), shown);
    assert!(transitions.is_empty(), "{transitions:?}");

    // It persists with that progress.
    let restored = Engine::from_json(data(), &engine.to_json()).unwrap();
    assert_eq!(restored.active(), Some(ODILE));
    assert_eq!(
        status(&restored, ODILE, "fixture-riverbank"),
        Status::InProgress
    );

    // A known name gains the new character's progress as its own evidence,
    // and the transitions are its own changes.
    let mut engine = Engine::new(data());
    feed(
        &mut engine,
        &[
            MARKER.to_string(),
            line(COLD_REWARD),
            level_up(CORISANDE, "Witch", 9),
        ],
        false,
    );
    let mut lines = vec![MARKER.to_string(), riverbank_entry()];
    lines.extend(to_the_encampment());
    feed(&mut engine, &lines, false);
    let transitions = feed(&mut engine, &[level_up(CORISANDE, "Witch", 10)], false);
    assert_eq!(names(&engine), [CORISANDE]);
    assert_eq!(engine.current(), Some(CORISANDE));
    assert_eq!(engine.character(CORISANDE).unwrap().level, 10);
    assert_eq!(
        status(&engine, CORISANDE, "fixture-riverbank"),
        Status::InProgress
    );
    assert!(
        transitions
            .iter()
            .any(|t| t.character == CORISANDE && t.quest_id == "fixture-riverbank"),
        "{transitions:?}"
    );
}

#[test]
fn party_member_level_up_does_not_name_new_character() {
    let mut engine = Engine::new(data());
    feed(
        &mut engine,
        &[
            MARKER.to_string(),
            riverbank_entry(),
            line(": Bram_Holloway has joined the area."),
            level_up("Bram_Holloway", "Warrior", 71),
            // A reward line before the level-up goes to the new character
            // and does not name it.
            line(": Odile_Marsh has received +20 to maximum Life."),
        ],
        true,
    );
    assert!(engine.new_character_open());
    assert_eq!(engine.current(), None);
    assert!(engine.characters().is_empty());

    // The own level-up after it names it.
    feed(&mut engine, &[level_up(ODILE, "Mercenary", 2)], true);
    assert!(!engine.new_character_open());
    assert_eq!(names(&engine), [ODILE]);
    assert_eq!(engine.current(), Some(ODILE));
    let saved = engine.to_json().to_string();
    assert!(!saved.contains("Bram_Holloway"), "{saved}");
}

#[test]
fn session_end_parks_new_character_and_restores_pin() {
    let mut engine = Engine::new(data());
    feed(
        &mut engine,
        &[
            MARKER.to_string(),
            riverbank_entry(),
            line("[SCENE] Set Source [The Riverbank]"),
        ],
        false,
    );
    assert!(engine.relog_window_open());
    assert!(!engine.new_character_open());

    // A session marker before any level-up first lets the entry take effect,
    // then parks the new character: nothing is dropped or stored, and the
    // record keeps its progress, closed.
    engine.feed_line(MARKER, false);
    assert!(!engine.relog_window_open());
    assert!(!engine.new_character_open());
    assert!(engine.new_character().is_none());
    assert!(engine.new_character_states().is_none());
    assert_eq!(engine.dropped_count(), 0);
    assert_eq!(engine.dropped_new_character_count(), 0);
    assert!(engine.characters().is_empty());
    let record = engine.new_character_json().expect("parked, not dropped");
    assert_eq!(record["open"], Value::Bool(false));
    assert_eq!(record["events"], 2);
    assert_eq!(record["seed"], RIVERBANK_SEED);

    // The next session's level-up is the old rule: unconfirmed, and the
    // parked character gave it nothing; the record stays parked.
    feed(&mut engine, &[level_up(ODILE, "Mercenary", 2)], false);
    let c = &engine.characters()[0];
    assert_eq!((c.name.as_str(), c.own, c.active), (ODILE, false, true));
    let riverbank = engine.quest_state(ODILE, "fixture-riverbank").unwrap();
    assert!(!riverbank.steps[0].fired, "{riverbank:?}");
    let parked = Engine::from_json(data(), &engine.to_json()).unwrap();
    assert!(parked.new_character_json().is_none(), "never in to_json");
    let mut back = Engine::new(data());
    back.load_new_character(&engine.new_character_json().unwrap())
        .unwrap();
    assert!(!back.new_character_open());
    assert_eq!(back.new_character_json().unwrap(), record);

    // With a pin: the entry released it, and the marker gives it back, so
    // the next session's events go to the pinned character again.
    let mut engine = Engine::new(data());
    feed(&mut engine, &[MARKER.to_string(), line(COLD_REWARD)], true);
    engine.pin(Some(CORISANDE)).unwrap();
    feed(&mut engine, &[MARKER.to_string(), riverbank_entry()], true);
    assert_eq!(engine.pinned(), Some(CORISANDE), "kept while undecided");
    expire(&mut engine);
    assert_eq!(engine.pinned(), None);
    assert_eq!(engine.new_character_released_pin(), Some(CORISANDE));
    engine.feed_line(MARKER, true);
    assert_eq!(engine.pinned(), Some(CORISANDE));
    assert_eq!(engine.current(), Some(CORISANDE));
    assert_eq!(
        engine.new_character_json().unwrap()["released_pin"],
        Value::Null
    );
    assert_eq!(
        engine.take_character_notes(),
        [
            CharacterNote::Started {
                released_pin: Some(CORISANDE.to_string())
            },
            CharacterNote::Parked {
                restored_pin: Some(CORISANDE.to_string())
            },
        ]
    );
    feed(&mut engine, &[area_line("G1_11"), line(POINTS)], true);
    assert_eq!(status(&engine, CORISANDE, "fixture-crowbell"), Status::Done);
    assert_eq!(
        Engine::from_json(data(), &engine.to_json())
            .unwrap()
            .pinned(),
        Some(CORISANDE)
    );
}

#[test]
fn level_up_without_riverbank_stays_unconfirmed() {
    // Baseline: with no level-1 G1_1 line in the session (a level-2 entry to
    // it does not count), a level-up of an unknown name leaves it active
    // and unconfirmed, as before.
    let mut engine = Engine::new(data());
    feed(
        &mut engine,
        &[
            MARKER.to_string(),
            line(r#"Generating level 15 area "G1_town" with seed 1"#),
            area_line("G1_1"),
            level_up(ODILE, "Mercenary", 2),
        ],
        true,
    );
    assert!(!engine.new_character_open());
    assert_eq!(engine.current(), Some(ODILE));
    let c = &engine.characters()[0];
    assert_eq!((c.name.as_str(), c.own, c.active), (ODILE, false, true));
}

#[test]
fn riverbank_entry_moves_pin_to_new_character() {
    let pinned_corisande = || {
        let mut engine = Engine::new(data());
        feed(
            &mut engine,
            &[
                MARKER.to_string(),
                line(": Corisande_Fell has received +20 to maximum Life."),
                level_up(CORISANDE, "Witch", 30),
            ],
            true,
        );
        engine.pin(Some(CORISANDE)).unwrap();
        engine
    };
    let riverbank_fired = |engine: &Engine, name: &str| {
        engine.quest_state(name, "fixture-riverbank").unwrap().steps[0].fired
    };

    // The level-1 entry releases the pin once its window closes, and the
    // named character takes it.
    let mut engine = pinned_corisande();
    feed(&mut engine, &[riverbank_entry()], true);
    assert_eq!(engine.pinned(), Some(CORISANDE));
    expire(&mut engine);
    assert!(engine.new_character_open());
    assert_eq!(engine.pinned(), None);
    assert_eq!(engine.new_character_released_pin(), Some(CORISANDE));
    assert_eq!(engine.current(), None);
    feed(&mut engine, &[level_up(ODILE, "Mercenary", 2)], true);
    assert_eq!(engine.pinned(), Some(ODILE));
    assert_eq!(engine.current(), Some(ODILE));
    let pins: Vec<(String, bool)> = engine
        .characters()
        .into_iter()
        .map(|c| (c.name, c.pinned))
        .collect();
    assert_eq!(
        pins,
        [(CORISANDE.to_string(), false), (ODILE.to_string(), true)]
    );
    // The Riverbank went to the new character, not to the old pin.
    assert!(riverbank_fired(&engine, ODILE));
    assert!(!riverbank_fired(&engine, CORISANDE));
    let restored = Engine::from_json(data(), &engine.to_json()).unwrap();
    assert_eq!(restored.pinned(), Some(ODILE));

    // Control: unpinning while it is open cancels the pin it would take.
    let mut engine = pinned_corisande();
    feed(&mut engine, &[riverbank_entry()], true);
    expire(&mut engine);
    engine.pin(None).unwrap();
    assert_eq!(engine.new_character_released_pin(), None);
    feed(&mut engine, &[level_up(ODILE, "Mercenary", 2)], true);
    assert_eq!((engine.pinned(), engine.active()), (None, Some(ODILE)));

    // Pinning a character while it is open gives it the new character's
    // progress, as held events go to a newly pinned character.
    let mut engine = pinned_corisande();
    feed(&mut engine, &[riverbank_entry()], true);
    expire(&mut engine);
    engine.pin(Some(CORISANDE)).unwrap();
    assert!(!engine.new_character_open());
    assert_eq!(engine.current(), Some(CORISANDE));
    assert!(riverbank_fired(&engine, CORISANDE));

    // Pinning while the entry is still undecided decides it for the pin: no
    // new character starts, no note, and the window's events go to her.
    let mut engine = pinned_corisande();
    engine.take_character_notes();
    feed(&mut engine, &[riverbank_entry()], true);
    assert!(!riverbank_fired(&engine, CORISANDE), "the entry waits");
    engine.pin(Some(CORISANDE)).unwrap();
    assert!(!engine.relog_window_open());
    assert!(!engine.new_character_open());
    assert!(riverbank_fired(&engine, CORISANDE));
    assert!(engine.take_character_notes().is_empty());
    expire(&mut engine);
    assert!(!engine.new_character_open());
    assert_eq!(engine.pinned(), Some(CORISANDE));
}

#[test]
fn new_character_saved_apart_from_characters() {
    let fresh = Engine::new(data()).to_json();
    assert!(Engine::new(data()).new_character_json().is_none());
    let mut engine = Engine::new(data());
    let mut lines = vec![MARKER.to_string(), riverbank_entry()];
    lines.extend(to_the_encampment());
    lines.push(line("Renly: Fixture line: welcome to the encampment."));
    feed(&mut engine, &lines, true);
    let shown = engine.new_character_states().unwrap();
    assert_eq!(shown[0].status, Status::Done);

    // Absent from to_json and characters()...
    assert_eq!(engine.to_json(), fresh);
    assert!(engine.characters().is_empty());
    assert!(engine.character("").is_none());
    assert!(engine.character_json("").is_none());
    assert_eq!(status(&engine, "", "fixture-riverbank"), Status::Locked);

    // ...and present in its own record.
    let record = engine.new_character_json().expect("open");
    assert_eq!(record["schema"], 1);
    assert_eq!(record["open"], Value::Bool(true));
    assert_eq!(record["seed"], RIVERBANK_SEED);
    assert_eq!(record["events"], 5);
    assert_eq!(record["released_pin"], Value::Null);
    assert_eq!(record["state"]["name"], "");
    assert_eq!(record["state"]["level"], 1);
    assert!(record["state"]["evidence"]["fixture-riverbank"].is_array());

    // A record taken back (a relaunch) reopens with the same rows.
    let mut back = Engine::from_json(data(), &engine.to_json()).unwrap();
    back.load_new_character(&record).unwrap();
    assert!(back.new_character_open());
    assert_eq!(back.current(), None);
    assert_eq!(back.new_character_states().unwrap(), shown);
    assert_eq!(back.to_json(), fresh);
    assert_eq!(back.new_character_json().unwrap(), record);
    // It goes on taking events, and its level-up names it.
    feed(&mut back, &[level_up(ODILE, "Mercenary", 2)], true);
    assert_eq!(names(&back), [ODILE]);
    assert_eq!(status(&back, ODILE, "fixture-riverbank"), Status::Done);
    assert!(back.new_character_json().is_none(), "named: record cleared");

    // A released pin comes back with the record and still moves to the
    // named character.
    let mut engine = Engine::new(data());
    feed(&mut engine, &[MARKER.to_string(), line(COLD_REWARD)], true);
    engine.pin(Some(CORISANDE)).unwrap();
    feed(&mut engine, &[MARKER.to_string(), riverbank_entry()], true);
    // No record while the entry is undecided, so a save then writes none.
    assert!(engine.new_character_json().is_none());
    expire(&mut engine);
    let record = engine.new_character_json().unwrap();
    assert_eq!(record["released_pin"], CORISANDE);
    let saved = engine.to_json();
    assert_eq!(saved["pinned"], Value::Null);
    let mut back = Engine::from_json(data(), &saved).unwrap();
    back.load_new_character(&record).unwrap();
    assert_eq!(back.new_character_released_pin(), Some(CORISANDE));
    assert_eq!(back.pinned(), None);
    feed(&mut back, &[level_up(ODILE, "Mercenary", 2)], true);
    assert_eq!(back.pinned(), Some(ODILE));

    // A record that is not one is refused, and the engine keeps none.
    let mut bad = Engine::new(data());
    let mut wrong = record.clone();
    wrong["schema"] = 2.into();
    assert!(bad.load_new_character(&wrong).is_err());
    assert!(bad
        .load_new_character(&serde_json::json!({"open": true}))
        .is_err());
    assert!(bad.new_character_json().is_none());

    // Beside a stored character, its progress is not in that character's
    // state either.
    let mut engine = Engine::new(data());
    feed(
        &mut engine,
        &[line(": Corisande_Fell has received +20 to maximum Life.")],
        true,
    );
    feed(&mut engine, &lines, true);
    assert!(engine.new_character_open());
    assert_eq!(names(&engine), [CORISANDE]);
    assert!(!engine.characters()[0].active);
    let saved = engine.to_json().to_string();
    assert!(!saved.contains("fixture-riverbank"), "{saved}");
}

#[test]
fn refusal_skips_done_and_skipped_quests() {
    // Positive control: area unknown, the reward goes to Corisande, and
    // fixture-beira is neither done nor skipped for her: counted.
    let mut engine = Engine::new(data());
    feed(&mut engine, &[line(COLD_REWARD)], true);
    assert_eq!(engine.current(), Some(CORISANDE));
    assert_eq!(engine.refused_count(), 1);

    // fixture-crowbell done by its own signal, in its area; then, with the
    // area unknown again, its points line is not a refusal.
    feed(&mut engine, &[area_line("G1_11"), line(POINTS)], true);
    assert_eq!(status(&engine, CORISANDE, "fixture-crowbell"), Status::Done);
    engine.seed_area(None);
    feed(&mut engine, &[line(POINTS)], true);
    assert_eq!(engine.refused_count(), 1);

    // Skipped by the user: not counted.
    engine
        .quest_action(CORISANDE, "fixture-beira", QuestAction::Skip)
        .unwrap();
    feed(&mut engine, &[line(COLD_REWARD)], true);
    assert_eq!(engine.refused_count(), 1);

    // Marked done by the user: not counted.
    engine
        .quest_action(CORISANDE, "fixture-beira", QuestAction::Unskip)
        .unwrap();
    engine
        .quest_action(CORISANDE, "fixture-beira", QuestAction::MarkDone)
        .unwrap();
    feed(&mut engine, &[line(COLD_REWARD)], true);
    assert_eq!(engine.refused_count(), 1);

    // Reset: open again, so counted again.
    engine
        .quest_action(CORISANDE, "fixture-beira", QuestAction::Reset)
        .unwrap();
    feed(&mut engine, &[line(COLD_REWARD)], true);
    assert_eq!(engine.refused_count(), 2);
    assert_eq!(
        engine.take_refused(),
        vec!["fixture-beira".to_string(), "fixture-beira".to_string()]
    );
}

#[test]
fn refusal_needs_a_receiving_character() {
    // No character: the points line is held, and no refusal is counted.
    let mut engine = Engine::new(data());
    feed(&mut engine, &[line(POINTS)], true);
    assert_eq!(engine.held_count(), 1);
    assert_eq!(engine.refused_count(), 0);
    assert!(engine.refused_quests().is_empty());

    // A character takes the held line later: counted then, for her, as a
    // line routed to her is.
    feed(
        &mut engine,
        &[line(": Corisande_Fell has received +20 to maximum Life.")],
        true,
    );
    assert_eq!(engine.held_count(), 0);
    assert_eq!(engine.refused_count(), 1);
    assert_eq!(engine.refused_quests(), ["fixture-crowbell".to_string()]);

    // The same line, now going to her directly, is counted the same way.
    feed(&mut engine, &[line(POINTS)], true);
    assert_eq!(engine.refused_count(), 2);
    assert_eq!(
        engine.refused_quests(),
        [
            "fixture-crowbell".to_string(),
            "fixture-crowbell".to_string()
        ]
    );

    // The new character from The Riverbank receives events, so it counts,
    // once its entry is decided and the window's events reach it.
    let mut engine = Engine::new(data());
    feed(&mut engine, &[riverbank_entry()], true);
    engine.seed_area(None);
    feed(&mut engine, &[line(POINTS)], true);
    assert_eq!(engine.refused_count(), 0);
    expire(&mut engine);
    assert_eq!(engine.refused_count(), 1);
}

// -- the Riverbank relog rule, the new character that survives, and the
// engine findings of launch 6 (Context "Launch 7 fixes (owner scope)") -------

/// A client line at a given time of 2026/03/14, for the timed cases.
fn line_at(time: &str, level: &str, message: &str) -> String {
    format!("2026/03/14 {time} 9000000 abcdef01 [{level} Client 2120] {message}")
}

/// The level-1 entry to one Riverbank instance, at a given time.
fn riverbank_entry_at(time: &str, seed: u64) -> String {
    line_at(
        time,
        "INFO",
        &format!(r#"Generating level 1 area "G1_1" with seed {seed}"#),
    )
}

/// The character build line the client wrote after a relog, at a given time.
fn character_line(time: &str, name: &str) -> String {
    line_at(
        time,
        "WARN",
        &format!("Character {name} has unknown build of path: None"),
    )
}

/// An engine that knows Corisande as an own character, level 30, from an
/// earlier session; pinned when asked.
fn with_corisande(pinned: bool) -> Engine {
    let mut engine = Engine::new(data());
    feed(
        &mut engine,
        &[
            MARKER.to_string(),
            line(": Corisande_Fell has received +20 to maximum Life."),
            level_up(CORISANDE, "Witch", 30),
        ],
        true,
    );
    if pinned {
        engine.pin(Some(CORISANDE)).unwrap();
    }
    engine.take_character_notes();
    engine
}

fn riverbank_fired(engine: &Engine, name: &str) -> bool {
    engine.quest_state(name, "fixture-riverbank").unwrap().steps[0].fired
}

#[test]
fn riverbank_relog_of_stored_character_keeps_pin_and_active() {
    // A relog from the character select: the client keeps its log open, so
    // no session marker comes first.
    let relog = |name: &str| {
        vec![
            riverbank_entry_at("12:00:00", RIVERBANK_SEED),
            line_at("12:00:01", "INFO", "[SCENE] Set Source [The Riverbank]"),
            character_line("12:00:02", name),
        ]
    };

    // Baseline: without the name line the entry, once its window has
    // closed, opens the new-character state and releases the pin.
    let mut engine = with_corisande(true);
    let mut lines = relog(CORISANDE);
    lines.pop();
    feed(&mut engine, &lines, true);
    expire(&mut engine);
    assert!(engine.new_character_open());
    assert_eq!(engine.pinned(), None);

    // Target: the line names Corisande, a stored character, 2 s after the
    // entry: a relog. The entry never took effect: the pin and the active
    // character are the same right after the entry and after the name line,
    // the only note is the relog, and the entry went to her.
    let mut engine = with_corisande(true);
    let lines = relog(CORISANDE);
    let before = (
        engine.pinned().map(str::to_string),
        engine.active().map(str::to_string),
    );
    assert_eq!(before, (Some(CORISANDE.into()), Some(CORISANDE.into())));
    let mut transitions = feed(&mut engine, &lines[..2], true);
    let now = (
        engine.pinned().map(str::to_string),
        engine.active().map(str::to_string),
    );
    assert_eq!(now, before, "unchanged right after the entry");
    assert!(engine.relog_window_open());
    assert!(!engine.new_character_open());
    transitions.extend(feed(&mut engine, &lines[2..], true));
    let now = (
        engine.pinned().map(str::to_string),
        engine.active().map(str::to_string),
    );
    assert_eq!(now, before, "unchanged after the name line");
    assert!(!engine.relog_window_open());
    assert!(!engine.new_character_open());
    assert_eq!(engine.current(), Some(CORISANDE));
    assert_eq!(names(&engine), [CORISANDE]);
    assert!(riverbank_fired(&engine, CORISANDE));
    assert!(
        transitions
            .iter()
            .any(|t| t.character == CORISANDE && t.quest_id == "fixture-riverbank"),
        "{transitions:?}"
    );
    assert!(engine.new_character_json().is_none(), "no record");
    assert_eq!(engine.dropped_count(), 0);
    assert_eq!(engine.dropped_new_character_count(), 0);
    assert_eq!(
        engine.take_character_notes(),
        [CharacterNote::Relogged {
            name: CORISANDE.to_string()
        }]
    );
    // Nothing more once the window would have expired.
    assert!(expire(&mut engine).is_empty());
    assert!(engine.take_character_notes().is_empty());
    let saved = Engine::from_json(data(), &engine.to_json()).unwrap();
    assert_eq!(saved.pinned(), Some(CORISANDE));

    // Unpinned: she stays active, nothing is pinned, and later events go
    // to her.
    let mut engine = with_corisande(false);
    feed(&mut engine, &relog(CORISANDE), true);
    assert!(!engine.new_character_open());
    assert_eq!((engine.pinned(), engine.active()), (None, Some(CORISANDE)));
    assert_eq!(
        engine.take_character_notes(),
        [CharacterNote::Relogged {
            name: CORISANDE.to_string()
        }]
    );
    assert!(riverbank_fired(&engine, CORISANDE));
    feed(
        &mut engine,
        &[line_at(
            "12:00:30",
            "INFO",
            "[SCENE] Set Source [Clearfell Encampment]",
        )],
        true,
    );
    let riverbank = engine.quest_state(CORISANDE, "fixture-riverbank").unwrap();
    assert_eq!(riverbank.current_step.as_deref(), Some("reach-encampment"));
}

#[test]
fn riverbank_entry_with_unknown_name_stays_new_character() {
    let mut engine = with_corisande(true);
    feed(
        &mut engine,
        &[
            MARKER.to_string(),
            riverbank_entry_at("12:00:00", RIVERBANK_SEED),
            character_line("12:00:02", ODILE),
        ],
        true,
    );
    // An unknown name in the window changes nothing: the window stays open.
    assert!(engine.relog_window_open());
    assert!(!engine.new_character_open());
    assert_eq!(engine.pinned(), Some(CORISANDE));
    assert!(engine.take_character_notes().is_empty());

    // Once it closes the new character is open, unnamed and unstored.
    engine.expire_relog_window(at("12:00:06"));
    assert!(engine.new_character_open());
    assert_eq!(engine.current(), None);
    assert_eq!(names(&engine), [CORISANDE]);
    assert_eq!(engine.new_character().unwrap().name, "");
    assert_eq!(engine.new_character_released_pin(), Some(CORISANDE));
    assert_eq!(
        engine.take_character_notes(),
        [CharacterNote::Started {
            released_pin: Some(CORISANDE.to_string())
        }]
    );

    // Its level-up names it, as before.
    feed(&mut engine, &[level_up(ODILE, "Mercenary", 2)], true);
    assert!(!engine.new_character_open());
    assert_eq!(engine.pinned(), Some(ODILE));
    assert!(riverbank_fired(&engine, ODILE));
    assert!(!riverbank_fired(&engine, CORISANDE));
    assert!(engine.new_character_json().is_none());
    assert_eq!(
        engine.take_character_notes(),
        [CharacterNote::Named {
            name: ODILE.to_string()
        }]
    );
}

#[test]
fn character_line_late_or_after_area_changes_nothing() {
    let opened = || {
        let mut engine = with_corisande(true);
        feed(
            &mut engine,
            &[
                MARKER.to_string(),
                riverbank_entry_at("12:00:00", RIVERBANK_SEED),
            ],
            true,
        );
        engine
    };

    // Positive control: 5 s after the entry is still in the window.
    let mut engine = opened();
    feed(&mut engine, &[character_line("12:00:05", CORISANDE)], true);
    assert!(!engine.new_character_open());
    assert_eq!(engine.pinned(), Some(CORISANDE));

    // 6 s after the entry: too late.
    let mut engine = opened();
    feed(&mut engine, &[character_line("12:00:06", CORISANDE)], true);
    assert!(engine.new_character_open());
    assert_eq!(engine.pinned(), None);

    // A line in between that is more than 5 s later closes the window too.
    let mut engine = opened();
    feed(
        &mut engine,
        &[
            line_at("12:00:07", "INFO", "[SCENE] Set Source [The Riverbank]"),
            character_line("12:00:07", CORISANDE),
        ],
        true,
    );
    assert!(engine.new_character_open());

    // After another area entry, even within 5 s: nothing.
    let mut engine = opened();
    feed(
        &mut engine,
        &[
            line_at(
                "12:00:01",
                "INFO",
                r#"Generating level 15 area "G1_town" with seed 1"#,
            ),
            character_line("12:00:02", CORISANDE),
        ],
        true,
    );
    assert!(engine.new_character_open());
    assert_eq!(engine.pinned(), None);

    // With no new character, a name line changes nothing: not the active
    // character, not the pin, not the store, and it is never held.
    let mut engine = with_corisande(false);
    feed(&mut engine, &[line(COLD_REWARD)], true);
    let before = (engine.to_json(), engine.characters(), engine.held_count());
    feed(
        &mut engine,
        &[
            character_line("12:00:01", ODILE),
            character_line("12:00:02", CORISANDE),
        ],
        true,
    );
    let after = (engine.to_json(), engine.characters(), engine.held_count());
    assert_eq!(after, before);
    let mut empty = Engine::new(data());
    feed(&mut empty, &[character_line("12:00:01", ODILE)], true);
    assert!(empty.characters().is_empty());
    assert_eq!(empty.held_count(), 0);
    assert_eq!(empty.current(), None);
}

#[test]
fn parked_new_character_resumes_on_same_seed() {
    let mut engine = with_corisande(true);
    let mut lines = vec![MARKER.to_string(), riverbank_entry()];
    lines.extend(to_the_encampment());
    feed(&mut engine, &lines, true);
    // Back in The Riverbank's instance before the session ends.
    feed(&mut engine, &[riverbank_entry()], true);
    let shown = engine.new_character_states().unwrap();
    engine.feed_line(MARKER, true);
    assert!(!engine.new_character_open());
    assert_eq!(engine.pinned(), Some(CORISANDE));
    engine.take_character_notes();

    // A relog into the same instance resumes it once the window closes, with
    // its rows, and releases the pin again.
    feed(&mut engine, &[MARKER.to_string(), riverbank_entry()], true);
    assert!(!engine.new_character_open());
    assert_eq!(engine.pinned(), Some(CORISANDE));
    expire(&mut engine);
    assert!(engine.new_character_open());
    assert_eq!(engine.new_character_states().unwrap(), shown);
    assert_eq!(engine.pinned(), None);
    assert_eq!(engine.new_character_released_pin(), Some(CORISANDE));
    assert_eq!(engine.new_character_json().unwrap()["events"], 6);
    assert_eq!(
        engine.take_character_notes(),
        [CharacterNote::Resumed {
            released_pin: Some(CORISANDE.to_string())
        }]
    );
    assert_eq!(engine.dropped_new_character_count(), 0);

    // Its level-up names it with all of it, and clears the record.
    feed(&mut engine, &[level_up(ODILE, "Mercenary", 2)], true);
    assert_eq!(engine.pinned(), Some(ODILE));
    let riverbank = engine.quest_state(ODILE, "fixture-riverbank").unwrap();
    assert_eq!(riverbank.current_step.as_deref(), Some("reach-encampment"));
    assert!(engine.new_character_json().is_none());
    assert_eq!(engine.dropped_new_character_count(), 0);

    // A relog of a stored character into the parked record's instance never
    // resumes it: the record stays parked, as it was, and the pin hers.
    let mut engine = with_corisande(true);
    feed(&mut engine, &lines, true);
    engine.feed_line(MARKER, true);
    let parked = engine.new_character_json().unwrap();
    feed(
        &mut engine,
        &[
            MARKER.to_string(),
            riverbank_entry_at("12:00:00", RIVERBANK_SEED),
            character_line("12:00:01", CORISANDE),
        ],
        true,
    );
    assert!(!engine.new_character_open());
    assert_eq!(engine.new_character_json().unwrap(), parked);
    assert_eq!(engine.pinned(), Some(CORISANDE));
    assert!(riverbank_fired(&engine, CORISANDE));
}

#[test]
fn parked_new_character_replaced_by_other_seed_counts_dropped() {
    let mut engine = with_corisande(false);
    feed(
        &mut engine,
        &[
            MARKER.to_string(),
            riverbank_entry(),
            line("[SCENE] Set Source [The Riverbank]"),
            MARKER.to_string(),
        ],
        true,
    );
    let parked = engine.new_character_json().unwrap();
    assert_eq!(parked["events"], 2);
    engine.take_character_notes();

    // Another instance: a new character starts in its place, with none of
    // the parked one's progress, and once the entry stands (here, the walk
    // to the encampment is another area entry) the parked one's events are
    // counted as dropped new-character events, apart from the held ones.
    let mut lines = vec![line(r#"Generating level 1 area "G1_1" with seed 77"#)];
    lines.extend(to_the_encampment());
    feed(&mut engine, &lines, true);
    assert!(engine.new_character_open());
    let record = engine.new_character_json().unwrap();
    assert_eq!(record["seed"], 77);
    assert_eq!(record["events"], 4);
    assert_eq!(engine.dropped_new_character_count(), 2);
    assert_eq!(engine.dropped_count(), 0);
    assert_eq!(
        engine.take_character_notes(),
        [
            CharacterNote::Started { released_pin: None },
            CharacterNote::Replaced { events: 2 },
        ]
    );

    // Held events dropped at a marker stay counted on their own.
    let mut engine = Engine::new(data());
    feed(
        &mut engine,
        &[MARKER.to_string(), line(POINTS), MARKER.to_string()],
        true,
    );
    let counts = (engine.dropped_count(), engine.dropped_new_character_count());
    assert_eq!(counts, (1, 0));

    // A relog of a stored character into the other instance never replaces
    // the parked record: it stays, and nothing is dropped.
    let mut engine = with_corisande(false);
    feed(
        &mut engine,
        &[
            MARKER.to_string(),
            riverbank_entry(),
            MARKER.to_string(),
            riverbank_entry_at("12:00:00", 77),
            character_line("12:00:02", CORISANDE),
            line_at(
                "12:00:30",
                "INFO",
                r#"Generating level 15 area "G1_town" with seed 1"#,
            ),
        ],
        true,
    );
    assert!(!engine.new_character_open());
    assert_eq!(engine.new_character_json().unwrap()["seed"], RIVERBANK_SEED);
    assert_eq!(engine.dropped_new_character_count(), 0);
    assert_eq!(engine.active(), Some(CORISANDE));
}

// -- the relog window decides before any note or pin release (Context
// "Launch 7 fixes (owner scope)", "Launch 8: wait, then decide") -----------

/// The note a level-1 entry posts when it takes effect over Corisande's pin.
fn started_over_corisande() -> Vec<CharacterNote> {
    vec![CharacterNote::Started {
        released_pin: Some(CORISANDE.to_string()),
    }]
}

#[test]
fn relog_window_posts_nothing_and_keeps_pin_until_closed() {
    let mut engine = with_corisande(true);
    let saved = engine.to_json();
    let transitions = feed(
        &mut engine,
        &[
            riverbank_entry_at("12:00:00", RIVERBANK_SEED),
            line_at("12:00:01", "INFO", "[SCENE] Set Source [The Riverbank]"),
        ],
        true,
    );
    // Before any close: no note, the pin held, nothing given to anyone.
    assert!(engine.relog_window_open());
    assert!(!engine.new_character_open());
    assert!(engine.take_character_notes().is_empty());
    assert_eq!(engine.pinned(), Some(CORISANDE));
    assert_eq!(engine.active(), Some(CORISANDE));
    assert_eq!(engine.current(), Some(CORISANDE));
    assert!(engine.new_character().is_none());
    assert!(engine.new_character_json().is_none(), "no record to save");
    assert!(transitions.is_empty(), "{transitions:?}");
    assert!(!riverbank_fired(&engine, CORISANDE), "the entry waits");
    assert_eq!(engine.to_json(), saved);

    // At 5 s it is still undecided.
    assert!(engine.expire_relog_window(at("12:00:05")).is_empty());
    assert!(engine.relog_window_open());
    assert!(engine.take_character_notes().is_empty());
    assert_eq!(engine.pinned(), Some(CORISANDE));

    // Closed without a relog: only now the note and the pin release.
    let transitions = engine.expire_relog_window(at("12:00:06"));
    assert!(!engine.relog_window_open());
    assert!(engine.new_character_open());
    assert_eq!(engine.take_character_notes(), started_over_corisande());
    assert_eq!(engine.pinned(), None);
    assert_eq!(engine.new_character_released_pin(), Some(CORISANDE));
    assert!(
        transitions
            .iter()
            .any(|t| t.character.is_empty() && t.quest_id == "fixture-riverbank" && t.live),
        "{transitions:?}"
    );
    assert_eq!(engine.new_character_json().unwrap()["events"], 2);
}

#[test]
fn relog_posts_only_relogged() {
    // A parked record of one instance, then a relog into another.
    let mut engine = with_corisande(true);
    feed(
        &mut engine,
        &[
            MARKER.to_string(),
            riverbank_entry(),
            line("[SCENE] Set Source [The Riverbank]"),
            MARKER.to_string(),
        ],
        true,
    );
    let parked = engine.new_character_json().expect("parked");
    assert_eq!(parked["open"], Value::Bool(false));
    engine.take_character_notes();

    feed(
        &mut engine,
        &[
            riverbank_entry_at("12:00:10", 77),
            line_at("12:00:11", "INFO", "[SCENE] Set Source [The Riverbank]"),
            character_line("12:00:12", CORISANDE),
        ],
        true,
    );
    // The relog is the only note: nothing started, replaced or released.
    assert_eq!(
        engine.take_character_notes(),
        [CharacterNote::Relogged {
            name: CORISANDE.to_string()
        }]
    );
    assert_eq!(engine.new_character_json().unwrap(), parked);
    assert_eq!(engine.dropped_new_character_count(), 0);
    assert_eq!(
        (engine.pinned(), engine.active()),
        (Some(CORISANDE), Some(CORISANDE))
    );

    // Later lines and the expiry add none.
    feed(
        &mut engine,
        &[line_at(
            "12:00:30",
            "INFO",
            r#"Generating level 15 area "G1_town" with seed 1"#,
        )],
        true,
    );
    assert!(engine.expire_relog_window(at("12:01:00")).is_empty());
    assert!(engine.take_character_notes().is_empty());
    assert!(!engine.new_character_open());
    assert_eq!(engine.new_character_json().unwrap(), parked);
}

#[test]
fn relog_window_expires_on_call_after_five_seconds() {
    // No window: the call changes nothing.
    let mut engine = with_corisande(true);
    assert!(engine.expire_relog_window(at("12:30:00")).is_empty());
    assert!(!engine.relog_window_open());
    assert_eq!(engine.pinned(), Some(CORISANDE));

    // At or under 5 s after the entry (or before it): nothing.
    feed(
        &mut engine,
        &[riverbank_entry_at("12:00:00", RIVERBANK_SEED)],
        true,
    );
    for t in ["11:59:58", "12:00:00", "12:00:04", "12:00:05"] {
        assert!(engine.expire_relog_window(at(t)).is_empty(), "{t}");
        assert!(engine.relog_window_open(), "{t}");
    }
    assert!(engine.take_character_notes().is_empty());
    assert_eq!(engine.pinned(), Some(CORISANDE));

    // More than 5 s after it: the entry takes effect.
    let transitions = engine.expire_relog_window(at("12:00:00").plus_seconds(6));
    assert!(!engine.relog_window_open());
    assert!(engine.new_character_open());
    assert_eq!(engine.take_character_notes(), started_over_corisande());
    assert!(
        transitions
            .iter()
            .any(|t| t.character.is_empty() && t.quest_id == "fixture-riverbank"),
        "{transitions:?}"
    );
    // A name line after that is no relog.
    feed(&mut engine, &[character_line("12:00:07", CORISANDE)], true);
    assert!(engine.new_character_open());
    assert!(engine.take_character_notes().is_empty());

    // Across a minute boundary.
    let mut engine = with_corisande(false);
    feed(
        &mut engine,
        &[riverbank_entry_at("12:00:58", RIVERBANK_SEED)],
        true,
    );
    engine.expire_relog_window(at("12:01:03"));
    assert!(engine.relog_window_open());
    engine.expire_relog_window(at("12:01:04"));
    assert!(!engine.relog_window_open());
    assert!(engine.new_character_open());
}

#[test]
fn relog_window_closed_by_area_line_level_up_or_marker() {
    let opened = || {
        let mut engine = with_corisande(true);
        feed(
            &mut engine,
            &[riverbank_entry_at("12:00:00", RIVERBANK_SEED)],
            true,
        );
        assert!(engine.relog_window_open());
        assert!(engine.take_character_notes().is_empty());
        engine
    };

    // Another area entry, within 5 s: the entry takes effect, then the new
    // area's entry goes to the new character.
    let mut engine = opened();
    feed(
        &mut engine,
        &[line_at(
            "12:00:01",
            "INFO",
            r#"Generating level 15 area "G1_town" with seed 1"#,
        )],
        true,
    );
    assert!(!engine.relog_window_open());
    assert!(engine.new_character_open());
    assert_eq!(engine.pinned(), None);
    assert_eq!(engine.current_area(), Some("G1_town"));
    assert_eq!(engine.take_character_notes(), started_over_corisande());
    assert_eq!(engine.new_character_json().unwrap()["events"], 2);

    // Any line more than 5 s after the entry: closed first, then the line
    // goes to the new character.
    let mut engine = opened();
    feed(
        &mut engine,
        &[line_at(
            "12:00:06",
            "INFO",
            "[SCENE] Set Source [The Riverbank]",
        )],
        true,
    );
    assert!(!engine.relog_window_open());
    assert!(engine.new_character_open());
    assert_eq!(engine.take_character_notes(), started_over_corisande());
    assert_eq!(engine.new_character_json().unwrap()["events"], 2);

    // A level-up: the entry takes effect, then the level-up names it, and
    // the released pin moves to it.
    let mut engine = opened();
    feed(
        &mut engine,
        &[line_at(
            "12:00:01",
            "INFO",
            &format!(": {ODILE} (Mercenary) is now level 2"),
        )],
        true,
    );
    assert!(!engine.relog_window_open());
    assert!(!engine.new_character_open());
    assert_eq!(
        (engine.pinned(), engine.active()),
        (Some(ODILE), Some(ODILE))
    );
    assert!(riverbank_fired(&engine, ODILE));
    assert!(!riverbank_fired(&engine, CORISANDE));
    let mut notes = started_over_corisande();
    notes.push(CharacterNote::Named {
        name: ODILE.to_string(),
    });
    assert_eq!(engine.take_character_notes(), notes);

    // A session marker: the entry takes effect, then the marker parks the
    // new character and gives the pin back.
    let mut engine = opened();
    engine.feed_line(MARKER, true);
    assert!(!engine.relog_window_open());
    assert!(!engine.new_character_open());
    assert_eq!(engine.pinned(), Some(CORISANDE));
    let record = engine.new_character_json().expect("parked");
    assert_eq!(
        (record["open"].as_bool(), record["events"].as_u64()),
        (Some(false), Some(1))
    );
    let mut notes = started_over_corisande();
    notes.push(CharacterNote::Parked {
        restored_pin: Some(CORISANDE.to_string()),
    });
    assert_eq!(engine.take_character_notes(), notes);
}

#[test]
fn relog_window_events_count_refusals_once() {
    // A points line held before the entry (no character yet this session)
    // and one in the window, both with the area unknown: fixture-crowbell's
    // area-tied signal is refused for each, once they reach a character.
    let setup = || {
        let mut engine = with_corisande(false);
        feed(
            &mut engine,
            &[MARKER.to_string(), line_at("11:59:30", "INFO", POINTS)],
            true,
        );
        assert_eq!(engine.held_count(), 1);
        feed(
            &mut engine,
            &[riverbank_entry_at("12:00:00", RIVERBANK_SEED)],
            true,
        );
        engine.seed_area(None);
        feed(&mut engine, &[line_at("12:00:01", "INFO", POINTS)], true);
        assert_eq!(engine.refused_count(), 0, "given to no one yet");
        engine
    };
    let twice = vec![
        "fixture-crowbell".to_string(),
        "fixture-crowbell".to_string(),
    ];

    // Decided as a relog: counted for Corisande, once each.
    let mut engine = setup();
    feed(&mut engine, &[character_line("12:00:02", CORISANDE)], true);
    assert_eq!(engine.current(), Some(CORISANDE));
    assert_eq!(engine.held_count(), 0);
    assert_eq!(engine.refused_count(), 2);
    assert_eq!(engine.take_refused(), twice);
    engine.expire_relog_window(at("12:00:30"));
    assert_eq!(engine.refused_count(), 2);
    assert!(engine.refused_quests().is_empty());

    // Decided as a new character: counted for it, once each.
    let mut engine = setup();
    expire(&mut engine);
    assert!(engine.new_character_open());
    assert_eq!(engine.held_count(), 0);
    assert_eq!(engine.refused_count(), 2);
    assert_eq!(engine.take_refused(), twice);
    expire(&mut engine);
    feed(&mut engine, &[character_line("12:00:07", CORISANDE)], true);
    assert_eq!(engine.refused_count(), 2);
    assert!(engine.refused_quests().is_empty());
}

#[test]
fn held_event_released_to_named_character_counts_refusal() {
    // Released by a `has received` line (rule 1). The points line
    // (fixture-crowbell) and then the reward itself (fixture-beira) are
    // both refused with the area unknown.
    let mut engine = Engine::new(data());
    feed(&mut engine, &[line(POINTS)], true);
    assert_eq!(engine.refused_count(), 0);
    feed(&mut engine, &[line(COLD_REWARD)], true);
    assert_eq!(engine.refused_count(), 2);
    assert_eq!(
        engine.take_refused(),
        vec!["fixture-crowbell".to_string(), "fixture-beira".to_string()]
    );

    // Released by a level-up of an unknown name (rule 4).
    let mut engine = Engine::new(data());
    feed(
        &mut engine,
        &[line(POINTS), level_up(ODILE, "Mercenary", 2)],
        true,
    );
    assert_eq!(engine.current(), Some(ODILE));
    assert_eq!(engine.refused_quests(), ["fixture-crowbell".to_string()]);

    // Released by a level-up of an own character (rule 3).
    let mut engine = with_corisande(false);
    feed(
        &mut engine,
        &[
            MARKER.to_string(),
            line(POINTS),
            level_up(CORISANDE, "Witch", 31),
        ],
        true,
    );
    assert_eq!(engine.refused_quests(), ["fixture-crowbell".to_string()]);

    // Released by the user's pin.
    let mut engine = with_corisande(false);
    feed(&mut engine, &[MARKER.to_string(), line(POINTS)], true);
    assert_eq!(engine.held_count(), 1);
    engine.pin(Some(CORISANDE)).unwrap();
    assert_eq!(engine.refused_quests(), ["fixture-crowbell".to_string()]);

    // Negative control: fixture-crowbell already done for her, so the
    // released line could not have changed it and is not counted.
    let mut engine = with_corisande(false);
    feed(&mut engine, &[area_line("G1_11"), line(POINTS)], true);
    assert_eq!(status(&engine, CORISANDE, "fixture-crowbell"), Status::Done);
    feed(&mut engine, &[MARKER.to_string(), line(POINTS)], true);
    assert_eq!(engine.held_count(), 1);
    engine.pin(Some(CORISANDE)).unwrap();
    assert_eq!(engine.refused_count(), 0);
}

#[test]
fn validator_reads_done_inferred() {
    let mut v: Value = serde_json::from_str(&data_text()).unwrap();
    v["quests"][0]
        .as_object_mut()
        .unwrap()
        .remove("done_inferred");
    v["quests"][1]["done_inferred"] = true.into();
    v["quests"][2]["done_inferred"] = false.into();
    let d = QuestData::from_value(&v).expect("done_inferred is accepted");
    // Absent is false; present is what it says.
    assert!(!d.quests[0].done_inferred);
    assert!(d.quests[1].done_inferred);
    assert!(!d.quests[2].done_inferred);

    // Anything but a boolean is refused, naming the quest and the field.
    v["quests"][1]["done_inferred"] = "yes".into();
    let err = QuestData::from_value(&v).unwrap_err();
    assert_eq!(err.quest.as_deref(), Some("fixture-beira"), "{err}");
    assert_eq!(err.field, "done_inferred", "{err}");
}

// -- a quest started by a signal (Context "Live 2 fixes (owner scope)") ------

/// The fixture data with a `start` list on fixture-beira (`quests[1]`).
fn data_with_beira_start(start: Value) -> Arc<QuestData> {
    let mut v: Value = serde_json::from_str(&data_text()).unwrap();
    v["quests"][1]["start"] = start;
    Arc::new(QuestData::from_value(&v).expect("a valid start list is accepted"))
}

/// The `start` the target cases give fixture-beira: fixture-riverbank done.
fn riverbank_done_start() -> Value {
    serde_json::json!([{ "kind": "quest", "id": "fixture-riverbank", "status": "done" }])
}

/// Corisande, an own character at level 30, over the given data.
fn corisande_over(data: Arc<QuestData>) -> Engine {
    let mut engine = Engine::new(data);
    feed(
        &mut engine,
        &[
            MARKER.to_string(),
            line(": Corisande_Fell has received +20 to maximum Life."),
            level_up(CORISANDE, "Witch", 30),
        ],
        true,
    );
    engine.take_character_notes();
    engine
}

/// The entry to G1_2, which unlocks fixture-beira.
fn beira_area() -> String {
    line(r#"Generating level 2 area "G1_2" with seed 2"#)
}

/// Renly's line in the encampment: fixture-riverbank's done signal.
fn renly_done() -> Vec<String> {
    vec![
        line(r#"Generating level 15 area "G1_town" with seed 1"#),
        line("Renly: Fixture line: welcome to the encampment."),
    ]
}

fn beira_state(engine: &Engine) -> (Status, Option<String>) {
    let q = engine.quest_state(CORISANDE, "fixture-beira").unwrap();
    (q.status, q.current_step)
}

#[test]
fn quest_without_start_stays_available_until_a_step() {
    // Baseline: the fixture as it is, with no `start` on fixture-beira.
    let d = data();
    assert!(d.quests[1].start.is_empty(), "absent start is empty");
    let mut engine = corisande_over(d);
    feed(&mut engine, &[beira_area()], true);
    assert_eq!(beira_state(&engine), (Status::Available, None));

    // fixture-riverbank done moves nothing of fixture-beira.
    let got = feed(&mut engine, &renly_done(), true);
    assert_eq!(
        status(&engine, CORISANDE, "fixture-riverbank"),
        Status::Done
    );
    assert_eq!(beira_state(&engine), (Status::Available, None));
    assert!(got.iter().all(|t| t.quest_id != "fixture-beira"), "{got:?}");

    // Only a step signal makes it in progress, at that step.
    feed(
        &mut engine,
        &[line(
            "Beira of the Rotten Pack: Fixture line: the pack gathers.",
        )],
        true,
    );
    assert_eq!(
        beira_state(&engine),
        (Status::InProgress, Some("meet-beira".to_string()))
    );
}

#[test]
fn start_signal_puts_quest_in_progress_at_first_step() {
    let mut engine = corisande_over(data_with_beira_start(riverbank_done_start()));
    // The start has not fired: the quest is only available.
    feed(&mut engine, &[beira_area()], true);
    assert_eq!(beira_state(&engine), (Status::Available, None));

    // Another quest's done fires the start: in progress, no step reached,
    // so the first step is the current one; reported as one transition.
    let got = feed(&mut engine, &renly_done(), true);
    assert_eq!(beira_state(&engine), (Status::InProgress, None));
    let beira = engine.quest_state(CORISANDE, "fixture-beira").unwrap();
    assert!(
        beira.steps.iter().all(|s| !s.fired && !s.ticked),
        "{beira:?}"
    );
    let started: Vec<&Transition> = got
        .iter()
        .filter(|t| t.quest_id == "fixture-beira")
        .collect();
    assert_eq!(
        started,
        vec![&Transition {
            character: CORISANDE.to_string(),
            quest_id: "fixture-beira".to_string(),
            from: Status::Available,
            to: Status::InProgress,
            step_from: None,
            step_to: None,
            live: true,
        }]
    );
    // Applying the same lines again changes nothing.
    assert_eq!(feed(&mut engine, &renly_done(), true), vec![]);

    // A step that fires after the start becomes the current step.
    feed(
        &mut engine,
        &[line(
            "Beira of the Rotten Pack: Fixture line: the pack gathers.",
        )],
        true,
    );
    assert_eq!(
        beira_state(&engine),
        (Status::InProgress, Some("meet-beira".to_string()))
    );

    // Done and skipped still come first (rules 1 and 2).
    engine
        .quest_action(CORISANDE, "fixture-beira", QuestAction::Skip)
        .unwrap();
    assert_eq!(beira_state(&engine).0, Status::Skipped);
    engine
        .quest_action(CORISANDE, "fixture-beira", QuestAction::Unskip)
        .unwrap();
    feed(&mut engine, &[beira_area(), line(COLD_REWARD)], true);
    assert_eq!(beira_state(&engine).0, Status::Done);

    // An event-based start (an NPC line) fires as evidence, and the same
    // line in catch-up reports its transition as catch-up.
    let npc_start = serde_json::json!([{ "kind": "npc", "speaker": "Una", "area": "G1_town" }]);
    let mut engine = corisande_over(data_with_beira_start(npc_start));
    feed(&mut engine, &[beira_area()], false);
    let got = feed(
        &mut engine,
        &[
            line(r#"Generating level 15 area "G1_town" with seed 1"#),
            line("Una: Fixture line: an invented remark."),
        ],
        false,
    );
    assert_eq!(beira_state(&engine), (Status::InProgress, None));
    assert!(
        got.iter()
            .any(|t| t.quest_id == "fixture-beira" && t.to == Status::InProgress && !t.live),
        "{got:?}"
    );
}

#[test]
fn validator_checks_start_signals() {
    let base: Value = serde_json::from_str(&data_text()).unwrap();
    // Absent is empty; a valid list is read in order.
    let d = QuestData::from_value(&base).unwrap();
    assert!(d.quests.iter().all(|q| q.start.is_empty()));
    let d = data_with_beira_start(riverbank_done_start());
    assert_eq!(d.quests[1].start.len(), 1);

    // Every check `unlock` gets, naming the quest and the field.
    type Mutate = fn(&mut Value);
    let cases: Vec<(&str, &str, Mutate)> = vec![
        ("not a list", "start", |v| {
            v["quests"][1]["start"] = "fixture-riverbank".into()
        }),
        ("unknown signal kind", "start[0].kind", |v| {
            v["quests"][1]["start"] = serde_json::json!([{ "kind": "telepathy" }])
        }),
        ("unknown signal field", "start[0].colour", |v| {
            v["quests"][1]["start"] =
                serde_json::json!([{ "kind": "area", "code": "G1_2", "colour": "red" }])
        }),
        ("quest signal naming no quest", "start[0].id", |v| {
            v["quests"][1]["start"] =
                serde_json::json!([{ "kind": "quest", "id": "no-such-quest", "status": "done" }])
        }),
        ("quest signal status outside enum", "start[0].status", |v| {
            v["quests"][1]["start"] = serde_json::json!([{ "kind": "quest", "id": "fixture-riverbank", "status": "begun" }])
        }),
        ("npc speaker not in npcs", "start[0].speaker", |v| {
            v["quests"][1]["start"] = serde_json::json!([{ "kind": "npc", "speaker": "Nobody" }])
        }),
        ("system line not in system_lines", "start[0].line", |v| {
            v["quests"][1]["start"] =
                serde_json::json!([{ "kind": "system", "line": "Not an allowlisted line." }])
        }),
        ("area code not in areas", "start[1].area", |v| {
            v["quests"][1]["start"] = serde_json::json!([
                { "kind": "area", "code": "G1_2" },
                { "kind": "npc", "speaker": "Una", "area": "G9_9" }
            ])
        }),
    ];
    for (what, field, mutate) in cases {
        let mut v = base.clone();
        mutate(&mut v);
        let err = QuestData::from_value(&v).expect_err(what);
        assert_eq!(err.quest.as_deref(), Some("fixture-beira"), "{what}: {err}");
        assert_eq!(err.field, field, "{what}: {err}");
    }
}
