//! The `screen` signal kind and the screen event (Context "Design: quest
//! data and the engine"), over the synthetic quest data in
//! `fixtures/quests_min.json` with screen signals added here, so the other
//! crates' tests keep the fixture they pin.
//!
//! Baseline: every session fixture gives the same states with the screen
//! signals in the data as without them, and no log line fires one. Target:
//! in `G1_2` with the pack leader's line heard, her bar going ticks the
//! fight step. The rest pin the fire rule (the area, the open step, one
//! quest), the refusal while the area is unknown, the routing (a held event
//! waits, the relog window is left alone), the demand, and the validator,
//! for the `remark` signal by key too.
//! All character names are invented.

use std::path::PathBuf;
use std::sync::Arc;

use qt_engine::{
    Engine, QuestData, ScreenEvent, ScreenOutcome, ScreenWhat, Status, Timestamp, Transition,
};
use serde_json::{json, Value};

const CORISANDE: &str = "Corisande_Fell";
const BEIRA: &str = "fixture-beira";
const BEIRA_BOSS: &str = "Beira of the Rotten Pack";
const ECHO: &str = "fixture-beira-echo";

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

fn fixture_value() -> Value {
    let path = fixtures().join("quests_min.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).expect("the fixture is JSON")
}

/// The fixture with screen signals: fixture-beira (`quests[1]`) gains a
/// fight step after her line, ended by her bar going in `G1_2`, and
/// fixture-lute (`quests[4]`) a last step ended by the QUEST COMPLETE
/// banner in Renly's window in `G1_town`.
fn screen_value() -> Value {
    let mut v = fixture_value();
    v["quests"][1]["steps"].as_array_mut().unwrap().push(json!({
        "id": "fight",
        "title": "Defeat the pack leader",
        "text": "Fixture step text.",
        "signals": [{ "kind": "screen", "what": "bar_gone", "boss": BEIRA_BOSS, "area": "G1_2" }]
    }));
    v["quests"][4]["steps"].as_array_mut().unwrap().push(json!({
        "id": "take-the-reward",
        "title": "Take the reward",
        "text": "Fixture step text.",
        "signals": [{ "kind": "screen", "what": "quest_complete", "npc": "Renly", "area": "G1_town" }]
    }));
    v
}

fn parse(v: &Value) -> Arc<QuestData> {
    Arc::new(QuestData::from_value(v).unwrap_or_else(|e| panic!("{e}")))
}

fn screen_data() -> Arc<QuestData> {
    parse(&screen_value())
}

/// [`screen_data`] with a copy of fixture-beira after every other quest,
/// unlocked by the same area: two quests one bar's going could fire.
fn echo_data() -> Arc<QuestData> {
    let mut v = screen_value();
    let mut echo = v["quests"][1].clone();
    echo["id"] = ECHO.into();
    echo["done"] = json!([]);
    echo["detection"] = "manual".into();
    v["quests"].as_array_mut().unwrap().push(echo);
    parse(&v)
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

fn session_names() -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(fixtures().join("sessions"))
        .expect("the sessions folder")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".txt"))
        .collect();
    names.sort();
    names
}

fn feed(engine: &mut Engine, lines: &[String], live: bool) -> Vec<Transition> {
    lines.iter().flat_map(|l| engine.feed_line(l, live)).collect()
}

fn line_at(time: &str, level: &str, message: &str) -> String {
    format!("2026/03/14 {time} 9000000 abcdef01 [{level} Client 2120] {message}")
}

fn line(message: &str) -> String {
    line_at("12:00:00", "INFO", message)
}

fn at(time: &str) -> Timestamp {
    Timestamp::parse(&format!("2026/03/14 {time}")).unwrap()
}

const MARKER: &str = "2026/03/14 11:59:00 ***** LOG FILE OPENING *****";

fn area_line(code: &str) -> String {
    line(&format!(r#"Generating level 2 area "{code}" with seed 2"#))
}

fn beira_line() -> String {
    line("Beira of the Rotten Pack: Fixture line: the pack gathers.")
}

fn level_up(name: &str, level: u32) -> String {
    line(&format!(": {name} (Witch) is now level {level}"))
}

/// Corisande, an own character at level 30, over the given data.
fn corisande_over(data: Arc<QuestData>) -> Engine {
    let mut engine = Engine::new(data);
    feed(
        &mut engine,
        &[
            MARKER.to_string(),
            line(": Corisande_Fell has received +20 to maximum Life."),
            level_up(CORISANDE, 30),
        ],
        true,
    );
    engine.take_character_notes();
    engine
}

/// Corisande in `G1_2` with the pack leader's line heard: fixture-beira in
/// progress at its first step, the fight step open.
fn at_the_fight(data: Arc<QuestData>) -> Engine {
    let mut engine = corisande_over(data);
    feed(&mut engine, &[area_line("G1_2"), beira_line()], true);
    assert_eq!(step_of(&engine, BEIRA), (Status::InProgress, Some("meet-beira".into())));
    engine
}

fn step_of(engine: &Engine, quest: &str) -> (Status, Option<String>) {
    let q = engine.quest_state(CORISANDE, quest).unwrap();
    (q.status, q.current_step)
}

fn ticked(quest: &str, step: &str) -> ScreenOutcome {
    ScreenOutcome::Ticked {
        quest_id: quest.to_string(),
        step_id: step.to_string(),
    }
}

/// The ids evidence would store for every screen signal of the data.
fn screen_signal_ids(data: &QuestData) -> Vec<(String, String)> {
    let mut ids = Vec::new();
    for q in &data.quests {
        for step in &q.steps {
            for (i, s) in step.signals.iter().enumerate() {
                if s.screen().is_some() {
                    ids.push((q.id.clone(), format!("step/{}/{i}", step.id)));
                }
            }
        }
    }
    ids
}

/// Every character's quest states, leaving out the steps list (which the
/// screen data lengthens).
fn summary(engine: &Engine) -> Vec<(String, String, Status, Status, Option<String>)> {
    let mut names: Vec<String> = engine.characters().into_iter().map(|c| c.name).collect();
    names.push(String::new());
    let mut out = Vec::new();
    for name in names {
        let states = if name.is_empty() {
            engine.new_character_states().unwrap_or_default()
        } else {
            engine.quest_states(&name)
        };
        for q in states {
            out.push((name.clone(), q.quest_id, q.status, q.tracked_status, q.current_step));
        }
    }
    out
}

fn shipped_data() -> Arc<QuestData> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/quests.json");
    Arc::new(QuestData::load(&path).unwrap_or_else(|e| panic!("{e}")))
}

#[test]
fn screen_signal_never_fires_from_a_log_line_baseline() {
    let names = session_names();
    assert!(names.len() >= 5, "{names:?}");
    let shipped = shipped_data();
    assert!(!screen_signal_ids(&shipped).is_empty(), "the shipped data has screen signals");
    for name in &names {
        let lines = session(name);
        // The fixture with and without screen signals: the same states, the
        // same records, the same transitions.
        let mut plain = Engine::new(parse(&fixture_value()));
        let mut screen = Engine::new(screen_data());
        let a = feed(&mut plain, &lines, false);
        let b = feed(&mut screen, &lines, false);
        assert_eq!(a, b, "{name}: transitions");
        assert_eq!(plain.to_json(), screen.to_json(), "{name}: characters");
        assert_eq!(plain.new_character_json(), screen.new_character_json(), "{name}: new character");
        assert_eq!(summary(&plain), summary(&screen), "{name}: states");
        assert_eq!(plain.refused_count(), screen.refused_count(), "{name}: refusals");

        // No screen signal is in any evidence, over the fixture or the
        // shipped data.
        let mut over_shipped = Engine::new(Arc::clone(&shipped));
        feed(&mut over_shipped, &lines, false);
        for (engine, data) in [(&screen, screen_data()), (&over_shipped, Arc::clone(&shipped))] {
            let mut evidence: Vec<Value> = engine.to_json()["characters"]
                .as_array()
                .expect("characters")
                .iter()
                .map(|c| c["evidence"].clone())
                .collect();
            evidence.extend(engine.new_character_json().map(|r| r["state"]["evidence"].clone()));
            for ev in &evidence {
                for (quest, id) in screen_signal_ids(&data) {
                    let fired = ev[&quest]
                        .as_array()
                        .is_some_and(|e| e.iter().any(|x| *x == id.as_str()));
                    assert!(!fired, "{name}: {quest} {id} fired from a log line");
                }
            }
        }
    }
}

#[test]
fn screen_bar_gone_ticks_the_open_fight_step_target() {
    let mut engine = at_the_fight(screen_data());
    let got = engine.apply_screen(&ScreenEvent::bar_gone(BEIRA_BOSS));
    assert_eq!(got.outcome, ticked(BEIRA, "fight"));
    assert_eq!(
        got.transitions,
        vec![Transition {
            character: CORISANDE.to_string(),
            quest_id: BEIRA.to_string(),
            from: Status::InProgress,
            to: Status::InProgress,
            step_from: Some("meet-beira".to_string()),
            step_to: Some("fight".to_string()),
            live: true,
        }]
    );
    assert_eq!(step_of(&engine, BEIRA), (Status::InProgress, Some("fight".into())));
    // Fired, as evidence under the signal's id, not ticked by hand.
    let ch = engine.character(CORISANDE).unwrap();
    assert!(ch.evidence[BEIRA].contains("step/fight/0"), "{:?}", ch.evidence);
    assert!(ch.overrides.get(BEIRA).is_none());

    // The step is reached now: the same event again finds no open step.
    let again = engine.apply_screen(&ScreenEvent::bar_gone(BEIRA_BOSS));
    assert_eq!(again.outcome, ScreenOutcome::NoOpenStep);
    assert!(again.transitions.is_empty());

    // Her reward line still finishes the quest.
    feed(
        &mut engine,
        &[line(": Corisande_Fell has received +10% to [Resistances|Cold Resistance].")],
        true,
    );
    assert_eq!(step_of(&engine, BEIRA).0, Status::Done);

    // The quest-complete banner works the same way, on its own NPC: in the
    // encampment with the lute heard, Renly's banner ends the last step.
    let mut engine = corisande_over(screen_data());
    feed(
        &mut engine,
        &[
            line(r#"Generating level 15 area "G1_town" with seed 1"#),
            line("Una: Fixture line: have you seen my lute?"),
            line("Fixture system line: a lute string hums."),
        ],
        true,
    );
    assert_eq!(step_of(&engine, "fixture-lute").1.as_deref(), Some("hear-the-lute"));
    let wrong = engine.apply_screen(&ScreenEvent::quest_complete("Una"));
    assert_eq!(wrong.outcome, ScreenOutcome::NoOpenStep, "another NPC's banner");
    let got = engine.apply_screen(&ScreenEvent::quest_complete("Renly"));
    assert_eq!(got.outcome, ticked("fixture-lute", "take-the-reward"));
    assert_eq!(step_of(&engine, "fixture-lute").1.as_deref(), Some("take-the-reward"));
}

#[test]
fn screen_event_outside_its_area_fires_nothing() {
    let mut engine = at_the_fight(screen_data());
    let refused = engine.refused_count();

    // A known, different area: the normal case, not counted.
    feed(&mut engine, &[area_line("G1_11")], true);
    let saved_here = engine.to_json();
    let got = engine.apply_screen(&ScreenEvent::bar_gone(BEIRA_BOSS));
    assert_eq!(got.outcome, ScreenOutcome::NoOpenStep);
    assert!(got.transitions.is_empty());
    assert_eq!(engine.to_json(), saved_here);
    assert_eq!(engine.refused_count(), refused);

    // Another boss, or another kind for the same name, in her area.
    feed(&mut engine, &[area_line("G1_2")], true);
    let saved = engine.to_json();
    for ev in [
        ScreenEvent::bar_gone("The King in the Mists"),
        ScreenEvent::quest_complete(BEIRA_BOSS),
        ScreenEvent {
            what: ScreenWhat::KillRemark,
            name: BEIRA_BOSS.to_string(),
        },
    ] {
        let got = engine.apply_screen(&ev);
        assert_eq!(got.outcome, ScreenOutcome::NoOpenStep, "{ev:?}");
        assert!(got.transitions.is_empty(), "{ev:?}");
    }
    assert_eq!(engine.to_json(), saved);
    assert_eq!(step_of(&engine, BEIRA).1.as_deref(), Some("meet-beira"));

    // The area unknown: refused and counted once, with the quest it would
    // have fired; nothing changes.
    engine.seed_area(None);
    engine.take_refused();
    let got = engine.apply_screen(&ScreenEvent::bar_gone(BEIRA_BOSS));
    assert_eq!(got.outcome, ScreenOutcome::AreaUnknown);
    assert!(got.transitions.is_empty());
    assert_eq!(engine.refused_count(), refused + 1);
    assert_eq!(engine.take_refused(), vec![BEIRA.to_string()]);
    assert_eq!(engine.to_json(), saved);

    // Negative control: an event no open step reads is refused too, but
    // there is nothing to count.
    let got = engine.apply_screen(&ScreenEvent::bar_gone("The King in the Mists"));
    assert_eq!(got.outcome, ScreenOutcome::AreaUnknown);
    assert_eq!(engine.refused_count(), refused + 1);

    // Back in her area it fires.
    engine.seed_area(Some("G1_2"));
    let got = engine.apply_screen(&ScreenEvent::bar_gone(BEIRA_BOSS));
    assert_eq!(got.outcome, ticked(BEIRA, "fight"));
}

#[test]
fn screen_event_fires_one_quest_only() {
    let mut engine = corisande_over(echo_data());
    feed(&mut engine, &[area_line("G1_2")], true);
    // Both available, neither step reached: the fight step is open in each.
    assert_eq!(step_of(&engine, BEIRA), (Status::Available, None));
    assert_eq!(step_of(&engine, ECHO), (Status::Available, None));
    let demand: Vec<String> = engine.screen_demand().into_iter().map(|d| d.quest_id).collect();
    assert_eq!(demand, [BEIRA, ECHO]);

    // The first in data order fires, and only it.
    let got = engine.apply_screen(&ScreenEvent::bar_gone(BEIRA_BOSS));
    assert_eq!(got.outcome, ticked(BEIRA, "fight"));
    assert!(got.transitions.iter().all(|t| t.quest_id == BEIRA), "{:?}", got.transitions);
    assert_eq!(step_of(&engine, BEIRA), (Status::InProgress, Some("fight".into())));
    assert_eq!(step_of(&engine, ECHO), (Status::Available, None));

    // The next one is the other's.
    let got = engine.apply_screen(&ScreenEvent::bar_gone(BEIRA_BOSS));
    assert_eq!(got.outcome, ticked(ECHO, "fight"));
    assert_eq!(step_of(&engine, ECHO), (Status::InProgress, Some("fight".into())));

    // A quest that is done, or still locked, does not take it.
    let mut engine = corisande_over(echo_data());
    feed(&mut engine, &[area_line("G1_2")], true);
    engine
        .quest_action(CORISANDE, BEIRA, qt_engine::QuestAction::MarkDone)
        .unwrap();
    let got = engine.apply_screen(&ScreenEvent::bar_gone(BEIRA_BOSS));
    assert_eq!(got.outcome, ticked(ECHO, "fight"));
    let mut engine = corisande_over(echo_data());
    engine.seed_area(Some("G1_2"));
    assert_eq!(step_of(&engine, BEIRA).0, Status::Locked);
    let got = engine.apply_screen(&ScreenEvent::bar_gone(BEIRA_BOSS));
    assert_eq!(got.outcome, ScreenOutcome::NoOpenStep);
}

#[test]
fn screen_event_leaves_the_relog_window_alone() {
    // Corisande, pinned, then a level-1 entry to an instance no character
    // is known in: the relog window opens.
    let mut engine = at_the_fight(screen_data());
    engine.pin(Some(CORISANDE)).unwrap();
    feed(
        &mut engine,
        &[line_at(
            "12:00:01",
            "INFO",
            r#"Generating level 1 area "G1_1" with seed 777"#,
        )],
        true,
    );
    assert!(engine.relog_window_open());
    let saved = engine.to_json();

    // The screen event waits in the window: nothing decided, no note, no
    // change, and the demand is empty while no character takes events.
    assert!(engine.screen_demand().is_empty());
    let got = engine.apply_screen(&ScreenEvent::bar_gone(BEIRA_BOSS));
    assert_eq!(got.outcome, ScreenOutcome::Waiting);
    assert!(got.transitions.is_empty());
    assert!(engine.relog_window_open());
    assert!(!engine.new_character_open());
    assert!(engine.take_character_notes().is_empty());
    assert_eq!(engine.pinned(), Some(CORISANDE));
    assert_eq!(engine.to_json(), saved);

    // It never moved the log's time: 5 s after the entry the window is
    // still open, and only a later log time closes it.
    assert!(engine.expire_relog_window(at("12:00:06")).is_empty());
    assert!(engine.relog_window_open());
    engine.expire_relog_window(at("12:00:07"));
    assert!(!engine.relog_window_open());
    assert!(engine.new_character_open());

    // A screen event with no character yet waits too, and goes, live, to
    // the character the held events go to.
    let mut engine = Engine::new(screen_data());
    feed(&mut engine, &[area_line("G1_2"), beira_line()], false);
    assert_eq!(engine.held_count(), 2);
    let got = engine.apply_screen(&ScreenEvent::bar_gone(BEIRA_BOSS));
    assert_eq!(got.outcome, ScreenOutcome::Waiting);
    assert_eq!(engine.held_count(), 3);
    let moved = feed(&mut engine, &[level_up(CORISANDE, 12)], false);
    assert_eq!(engine.active(), Some(CORISANDE));
    assert_eq!(step_of(&engine, BEIRA), (Status::InProgress, Some("fight".into())));
    assert!(
        moved
            .iter()
            .any(|t| t.quest_id == BEIRA && t.step_to.as_deref() == Some("fight") && t.live),
        "{moved:?}"
    );
}

#[test]
fn screen_demand_lists_what_would_fire_now() {
    let data = screen_data();
    let mut engine = corisande_over(Arc::clone(&data));
    // The area unknown: nothing.
    assert!(engine.screen_demand().is_empty());
    // A known area with no screen signal: nothing.
    feed(&mut engine, &[area_line("G1_11")], true);
    assert!(engine.screen_demand().is_empty());

    // Her area, the quest available: her fight step, with its id and name.
    feed(&mut engine, &[area_line("G1_2")], true);
    let demand = engine.screen_demand();
    assert_eq!(demand.len(), 1, "{demand:?}");
    let d = &demand[0];
    assert_eq!(
        (d.quest_id.as_str(), d.step_id.as_str(), d.signal_id.as_str()),
        (BEIRA, "fight", "step/fight/0")
    );
    assert_eq!((d.what, d.name.as_str(), d.area.as_str()), (ScreenWhat::BarGone, BEIRA_BOSS, "G1_2"));

    // Ticked by hand, the step is reached: nothing left to read.
    engine.tick_step(CORISANDE, BEIRA, "fight", true).unwrap();
    assert!(engine.screen_demand().is_empty());
    engine.tick_step(CORISANDE, BEIRA, "fight", false).unwrap();
    assert_eq!(engine.screen_demand().len(), 1);

    // Done: nothing.
    engine
        .quest_action(CORISANDE, BEIRA, qt_engine::QuestAction::MarkDone)
        .unwrap();
    assert!(engine.screen_demand().is_empty());
    engine
        .quest_action(CORISANDE, BEIRA, qt_engine::QuestAction::Reset)
        .unwrap();
    assert_eq!(engine.screen_demand().len(), 1);

    // A session marker forgets the area and the active character.
    feed(&mut engine, &[MARKER.to_string()], true);
    assert!(engine.screen_demand().is_empty());

    // The demand is read-only: asking changes nothing.
    let saved = engine.to_json();
    engine.screen_demand();
    assert_eq!(engine.to_json(), saved);
}

// -- the validator ----------------------------------------------------------

/// A kill-remark entry for the pack leader, its text invented.
fn remark() -> Value {
    json!({
        "class": "Witch",
        "boss": BEIRA_BOSS,
        "key": "FixtureKillComment",
        "text": "Fixture remark: the pack is broken.",
        "source_url": "https://poe2db.tw/us/Witch",
        "fetched": "2026-10-26"
    })
}

fn refused(v: &Value) -> (Option<String>, String, String) {
    let err = QuestData::from_value(v).expect_err("refused");
    (err.quest, err.field, err.message)
}

#[test]
fn validator_accepts_and_refuses_screen_signals_and_remarks() {
    // Accepted: the screen data, with no remarks table at all.
    let data = screen_data();
    assert!(data.remarks.is_empty());
    let out = serde_json::to_value(&*data).unwrap();
    assert!(out.get("remarks").is_none(), "an empty table is not written");
    assert_eq!(QuestData::from_value(&out).unwrap(), *data, "round trip");

    // Accepted: a remarks table and a kill remark beside the bar.
    let mut v = screen_value();
    v["remarks"] = json!([remark()]);
    v["quests"][1]["steps"][1]["signals"]
        .as_array_mut()
        .unwrap()
        .push(json!({ "kind": "screen", "what": "kill_remark", "boss": BEIRA_BOSS, "area": "G1_2" }));
    let with = QuestData::from_value(&v).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(with.remark_texts(BEIRA_BOSS), ["Fixture remark: the pack is broken."]);
    assert!(with.remark_texts("Renly").is_empty());
    let out = serde_json::to_value(&with).unwrap();
    assert_eq!(QuestData::from_value(&out).unwrap(), with, "round trip with remarks");
    // An empty table is accepted.
    let mut empty = screen_value();
    empty["remarks"] = json!([]);
    QuestData::from_value(&empty).unwrap();

    // Refused, naming the quest and the field.
    let fight = "steps[1].signals[0]";
    let cases: Vec<(&str, Box<dyn Fn(&mut Value)>, String, &str)> = vec![
        (
            "a screen signal outside a step",
            Box::new(|v| {
                v["quests"][1]["done"] = json!([{ "kind": "screen", "what": "bar_gone", "boss": BEIRA_BOSS, "area": "G1_2" }]);
            }),
            "done[0].kind".to_string(),
            "step only",
        ),
        (
            "an unknown what",
            Box::new(|v| v["quests"][1]["steps"][1]["signals"][0]["what"] = "bar_seen".into()),
            format!("{fight}.what"),
            "is not one of",
        ),
        (
            "no area",
            Box::new(|v| {
                v["quests"][1]["steps"][1]["signals"][0].as_object_mut().unwrap().remove("area");
            }),
            format!("{fight}.area"),
            "is missing",
        ),
        (
            "an area not in areas",
            Box::new(|v| v["quests"][1]["steps"][1]["signals"][0]["area"] = "G9_9".into()),
            format!("{fight}.area"),
            "is not in areas",
        ),
        (
            "a bar with no boss",
            Box::new(|v| {
                v["quests"][1]["steps"][1]["signals"][0].as_object_mut().unwrap().remove("boss");
            }),
            format!("{fight}.boss"),
            "is missing",
        ),
        (
            "a bar naming an npc",
            Box::new(|v| v["quests"][1]["steps"][1]["signals"][0]["npc"] = "Renly".into()),
            format!("{fight}.npc"),
            "names a boss",
        ),
        (
            "an empty boss",
            Box::new(|v| v["quests"][1]["steps"][1]["signals"][0]["boss"] = " ".into()),
            format!("{fight}.boss"),
            "must not be empty",
        ),
        (
            "an unknown field",
            Box::new(|v| v["quests"][1]["steps"][1]["signals"][0]["line"] = "x".into()),
            format!("{fight}.line"),
            "not a field",
        ),
        (
            "a kill remark with no text in remarks",
            Box::new(|v| v["quests"][1]["steps"][1]["signals"][0]["what"] = "kill_remark".into()),
            format!("{fight}.boss"),
            "has no text in remarks",
        ),
    ];
    for (what, edit, field, message) in cases {
        let mut v = screen_value();
        edit(&mut v);
        let (quest, f, m) = refused(&v);
        assert_eq!(quest.as_deref(), Some(BEIRA), "{what}: {m}");
        assert_eq!(f, field, "{what}: {m}");
        assert!(m.contains(message), "{what}: {m}");
    }
    // A banner naming a boss, on fixture-lute.
    let mut v = screen_value();
    v["quests"][4]["steps"][2]["signals"][0]["boss"] = BEIRA_BOSS.into();
    let (quest, f, m) = refused(&v);
    assert_eq!((quest.as_deref(), f.as_str()), (Some("fixture-lute"), "steps[2].signals[0].boss"), "{m}");

    // The remarks table, outside any quest.
    let remark_cases: Vec<(&str, Value, &str, &str)> = vec![
        ("an unknown boss", json!({ "boss": "Nobody" }), "remarks[0].boss", "neither"),
        ("a bad source", json!({ "source_url": "https://example.com/Witch" }), "remarks[0].source_url", "is not a"),
        ("a bad date", json!({ "fetched": "2026-02-30" }), "remarks[0].fetched", "date"),
        ("an empty text", json!({ "text": "" }), "remarks[0].text", "must not be empty"),
        ("an extra field", json!({ "voice": "x" }), "remarks[0].voice", "not a field"),
    ];
    for (what, patch, field, message) in remark_cases {
        let mut r = remark();
        for (k, val) in patch.as_object().unwrap() {
            r[k] = val.clone();
        }
        let mut v = screen_value();
        v["remarks"] = json!([r]);
        let (quest, f, m) = refused(&v);
        assert_eq!(quest, None, "{what}: {m}");
        assert_eq!(f, field, "{what}: {m}");
        assert!(m.contains(message), "{what}: {m}");
    }
    // A boss a bar_gone signal names, not listed as a speaker, is accepted.
    let mut v = screen_value();
    v["quests"][1]["steps"][1]["signals"][0]["boss"] = "The Pack Leader".into();
    let mut r = remark();
    r["boss"] = "The Pack Leader".into();
    v["remarks"] = json!([r]);
    QuestData::from_value(&v).unwrap_or_else(|e| panic!("{e}"));
    // A remarks table that is not a list.
    let mut v = screen_value();
    v["remarks"] = json!({});
    let (_, f, _) = refused(&v);
    assert_eq!(f, "remarks");
}

const TREE_KEY: &str = "FixtureTreeComment";
const TREE_TEXT: &str = "Fixture remark: an old tree.";

/// A remark row with no boss, for [`TREE_KEY`], its text invented.
fn key_remark() -> Value {
    json!({
        "class": "Witch",
        "key": TREE_KEY,
        "text": TREE_TEXT,
        "source_url": "https://poe2db.tw/us/Witch",
        "fetched": "2026-10-28"
    })
}

/// [`screen_value`] with a kill remark row, a key's row and a `remark`
/// signal for the key appended to the fight step.
fn remark_value() -> Value {
    let mut v = screen_value();
    v["remarks"] = json!([remark(), key_remark()]);
    v["quests"][1]["steps"][1]["signals"]
        .as_array_mut()
        .unwrap()
        .push(json!({ "kind": "screen", "what": "remark", "key": TREE_KEY, "area": "G1_2" }));
    v
}

#[test]
fn validator_accepts_and_refuses_remark_signals() {
    // Accepted: a `remark` signal names a key; a row with no boss is the
    // key's, and it reads as the key's text, never a boss's.
    let data = parse(&remark_value());
    let fight = &data.quest(BEIRA).unwrap().steps[1].signals;
    assert_eq!(fight[1].screen(), Some((ScreenWhat::Remark, TREE_KEY)));
    assert_eq!(data.key_texts(TREE_KEY), [TREE_TEXT]);
    assert_eq!(data.key_texts("FixtureKillComment"), ["Fixture remark: the pack is broken."]);
    assert_eq!(data.remark_texts(BEIRA_BOSS), ["Fixture remark: the pack is broken."]);
    assert_eq!(data.remarks[1].boss, None);
    let out = serde_json::to_value(&*data).unwrap();
    assert!(out["remarks"][1].get("boss").is_none(), "no boss is not written");
    assert_eq!(out["remarks"][0]["boss"], BEIRA_BOSS);
    assert_eq!(QuestData::from_value(&out).unwrap(), *data, "round trip");

    // The engine fires it by the key, like any screen signal.
    let mut engine = at_the_fight(Arc::clone(&data));
    let demand = engine.screen_demand();
    assert!(
        demand.iter().any(|d| d.what == ScreenWhat::Remark && d.name == TREE_KEY),
        "{demand:?}"
    );
    let got = engine.apply_screen(&ScreenEvent::remark(TREE_KEY));
    assert_eq!(got.outcome, ticked(BEIRA, "fight"));

    // Refused, naming the quest and the field.
    let key = "steps[1].signals[1]";
    let cases: Vec<(&str, Box<dyn Fn(&mut Value)>, String, &str)> = vec![
        (
            "a remark with no key",
            Box::new(|v| {
                v["quests"][1]["steps"][1]["signals"][1].as_object_mut().unwrap().remove("key");
            }),
            format!("{key}.key"),
            "is missing",
        ),
        (
            "an empty key",
            Box::new(|v| v["quests"][1]["steps"][1]["signals"][1]["key"] = " ".into()),
            format!("{key}.key"),
            "must not be empty",
        ),
        (
            "a key no row has",
            Box::new(|v| v["quests"][1]["steps"][1]["signals"][1]["key"] = "NoSuchComment".into()),
            format!("{key}.key"),
            "has no row in remarks",
        ),
        (
            "a remark naming a boss",
            Box::new(|v| v["quests"][1]["steps"][1]["signals"][1]["boss"] = BEIRA_BOSS.into()),
            format!("{key}.boss"),
            "names a key and no boss",
        ),
        (
            "a remark naming an npc",
            Box::new(|v| v["quests"][1]["steps"][1]["signals"][1]["npc"] = "Renly".into()),
            format!("{key}.npc"),
            "names a key and no npc",
        ),
        (
            "a bar naming a key",
            Box::new(|v| v["quests"][1]["steps"][1]["signals"][0]["key"] = TREE_KEY.into()),
            "steps[1].signals[0].key".to_string(),
            "names a boss and no key",
        ),
    ];
    for (what, edit, field, message) in cases {
        let mut v = remark_value();
        edit(&mut v);
        let (quest, f, m) = refused(&v);
        assert_eq!(quest.as_deref(), Some(BEIRA), "{what}: {m}");
        assert_eq!(f, field, "{what}: {m}");
        assert!(m.contains(message), "{what}: {m}");
    }
    // A banner naming a key, on fixture-lute.
    let mut v = remark_value();
    v["quests"][4]["steps"][2]["signals"][0]["key"] = TREE_KEY.into();
    let (quest, f, m) = refused(&v);
    assert_eq!((quest.as_deref(), f.as_str()), (Some("fixture-lute"), "steps[2].signals[0].key"), "{m}");

    // The remarks table: a row with no boss that no `remark` signal names
    // is an orphan, and a boss, when named, is not empty.
    let mut v = screen_value();
    v["remarks"] = json!([remark(), key_remark()]);
    let (quest, f, m) = refused(&v);
    assert_eq!((quest, f.as_str()), (None, "remarks[1].key"), "{m}");
    assert!(m.contains("no remark signal names it"), "{m}");
    let mut v = remark_value();
    v["remarks"][1]["boss"] = "".into();
    let (quest, f, m) = refused(&v);
    assert_eq!((quest, f.as_str()), (None, "remarks[1].boss"), "{m}");
    assert!(m.contains("must not be empty"), "{m}");
    // A kill remark row still needs its boss known.
    let mut v = remark_value();
    v["remarks"][0]["boss"] = "Nobody".into();
    let (_, f, m) = refused(&v);
    assert_eq!(f, "remarks[0].boss", "{m}");
}
