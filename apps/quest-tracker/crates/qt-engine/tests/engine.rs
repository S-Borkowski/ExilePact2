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
    area_from_log, DataError, Detection, Engine, LogArea, QuestAction, QuestData, Status,
    Transition,
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
    // The first session had no character: its two events were dropped.
    assert_eq!(engine.dropped_count(), 2);
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
        "the dropped session gave nothing"
    );

    // A session ending with nothing identified drops what it held.
    let mut engine = Engine::new(data());
    let lines = session("held.txt");
    feed(&mut engine, &lines[..6], false);
    assert_eq!(engine.held_count(), 2);
    engine.end_session();
    assert_eq!((engine.held_count(), engine.dropped_count()), (0, 4));
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
