//! Baseline and target for the view-model, over the engine's synthetic quest
//! data (`fixtures/quests_min.json`) and sessions (`fixtures/sessions/`).
//! Baseline: an engine that has seen nothing shows an empty state, never a
//! panel. Target: after `act1.txt` the panel lists the act the character is
//! in, its progress and its rows, filtered by the settings, with row changes
//! only for live transitions; a level-1 Riverbank entry shows the new
//! character before its first level-up names it; refused signals are counted
//! in the status. `fixtures_match_serialised_views` pins one view per
//! frame id to `fixtures/viewmodels/<frame id>.json`, which
//! the UI tests and the browser preview read, all built from one story
//! (`fixtures/sessions/frames/story.txt`); `QT_UPDATE_FIXTURES=1` rewrites
//! them, and two more tests hold the set consistent and varied. All
//! character names are invented.

use std::path::PathBuf;
use std::sync::Arc;

use qt_engine::{
    Engine, QuestAction, QuestData, RewardCategory, Status, Timestamp, Transition, Worth,
};
use qt_viewmodel::{
    build_view, reward_icon, Frame, LogState, Mode, RowChange, RowChangeKind, RowChanges, Scale,
    Settings, StatusKind, StepPhase, UiState, View, ViewKind,
};
use serde_json::Value;

const CORISANDE: &str = "Corisande_Fell";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixtures() -> PathBuf {
    root().join("fixtures")
}

fn data_value() -> Value {
    let path = fixtures().join("quests_min.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).expect("the fixture data is JSON")
}

fn data() -> Arc<QuestData> {
    Arc::new(QuestData::from_value(&data_value()).expect("the fixture data validates"))
}

/// The fixture data with some quests' worth changed, so ordering and the
/// worth filter have more than one value to work on.
fn data_with_worth(changes: &[(&str, &str)]) -> Arc<QuestData> {
    let mut v = data_value();
    for q in v["quests"].as_array_mut().unwrap() {
        for (id, worth) in changes {
            if q["id"] == *id {
                q["worth"] = Value::String(worth.to_string());
            }
        }
    }
    Arc::new(QuestData::from_value(&v).expect("the changed data validates"))
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
    lines.iter().flat_map(|l| engine.feed_line(l, live)).collect()
}

fn replay_with(data: Arc<QuestData>, name: &str) -> Engine {
    let mut engine = Engine::new(data);
    feed(&mut engine, &session(name), false);
    engine
}

fn replay(name: &str) -> Engine {
    replay_with(data(), name)
}

/// A client line entering an area, for the inline cases.
fn area_line(code: &str) -> String {
    format!("2026/03/08 18:20:00 2800000 5c0a0040 [DEBUG Client 5120] Generating level 15 area \"{code}\" with seed 1")
}

fn reading(kind: ViewKind) -> UiState {
    UiState {
        view_kind: kind,
        log: LogState::Reading,
        ..UiState::default()
    }
}

fn row_ids(view: &View) -> Vec<String> {
    view.acts
        .iter()
        .flat_map(|a| a.quests.iter().map(|r| r.id.clone()))
        .collect()
}

fn next_ids(view: &View) -> Vec<String> {
    view.next_up.iter().map(|r| r.id.clone()).collect()
}

fn ids(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

#[test]
fn next_up_lists_available_and_in_progress_worth_first() {
    // Act 1 after act1.txt: king and lute in progress, crowbell available,
    // riverbank and beira done, candles unknown. Lute made "no" and king
    // "optional", so worth decides before status does.
    let engine = replay_with(
        data_with_worth(&[("fixture-lute", "no"), ("fixture-king", "optional")]),
        "act1.txt",
    );
    let view = build_view(&engine, &Settings::default(), &reading(ViewKind::NextUp));
    assert_eq!(view.frame, Frame::PanelNextUp);
    assert_eq!(view.view_kind, ViewKind::NextUp);
    assert_eq!(
        next_ids(&view),
        ids(&["fixture-crowbell", "fixture-king", "fixture-lute"]),
        "{view:#?}"
    );
    // Same worth: in progress before available.
    let plain = replay("act1.txt");
    let view = build_view(&plain, &Settings::default(), &reading(ViewKind::NextUp));
    assert_eq!(
        next_ids(&view),
        ids(&["fixture-king", "fixture-lute", "fixture-crowbell"])
    );
    // An in-progress row always carries its next step; an available one none.
    let by_id = |id: &str| view.next_up.iter().find(|r| r.id == id).unwrap().clone();
    assert_eq!(by_id("fixture-king").status, Status::InProgress);
    assert_eq!(by_id("fixture-king").current_step.as_deref(), Some("Face the king"));
    assert_eq!(by_id("fixture-lute").current_step.as_deref(), Some("Return the lute"));
    assert_eq!(by_id("fixture-crowbell").status, Status::Available);
    assert_eq!(by_id("fixture-crowbell").current_step, None);
    // The lute is partial: it carries the manual-tick marker.
    assert!(by_id("fixture-lute").needs_tick);
    assert!(!by_id("fixture-king").needs_tick);
    // The collapsed badge counts the worth-it quests available now.
    assert_eq!(view.available_worth_count, 1);
}

#[test]
fn campaign_view_groups_by_act_with_progress() {
    let mut engine = replay("act1.txt");
    let ui = reading(ViewKind::Campaign);
    let view = build_view(&engine, &Settings::default(), &ui);
    assert_eq!(view.frame, Frame::PanelCampaign);
    // Only the act the character is in is listed (owner, 2026-10-25).
    assert_eq!(view.acts.len(), 1, "{view:#?}");
    let act = &view.acts[0];
    assert_eq!((act.act, act.title.as_str()), (1, "Act 1"));
    assert_eq!((act.done, act.total), (2, 6));
    assert!(act.expanded && act.has_available);
    assert_eq!(
        row_ids(&view),
        ids(&[
            "fixture-riverbank",
            "fixture-beira",
            "fixture-crowbell",
            "fixture-king",
            "fixture-lute",
            "fixture-candles",
        ])
    );
    let beira = &act.quests[1];
    assert_eq!(beira.rewards.len(), 1);
    assert_eq!(beira.rewards[0].category, RewardCategory::Resistance);
    assert_eq!(beira.rewards[0].icon, "reward-res-cold");
    assert_eq!(beira.rewards[0].text, "+10% to Cold Resistance");

    // A collapsed act keeps its header and progress, not its rows.
    let mut folded = ui.clone();
    folded.toggle_act(1);
    let view = build_view(&engine, &Settings::default(), &folded);
    let act = &view.acts[0];
    assert!(!act.expanded && act.quests.is_empty());
    assert_eq!((act.done, act.total), (2, 6));
    folded.toggle_act(1);
    assert!(build_view(&engine, &Settings::default(), &folded).acts[0].expanded);

    // A skipped quest leaves the act's total.
    engine
        .quest_action(CORISANDE, "fixture-candles", QuestAction::Skip)
        .unwrap();
    let act = build_view(&engine, &Settings::default(), &ui).acts[0].clone();
    assert_eq!((act.done, act.total), (2, 5));

    // Entering act 2's town unlocks the caravan: the panel moves to act 2.
    feed(&mut engine, &[area_line("G2_town")], true);
    let view = build_view(&engine, &Settings::default(), &ui);
    assert_eq!(view.acts.len(), 1);
    let act = &view.acts[0];
    assert_eq!((act.act, act.done, act.total), (2, 0, 1));
    assert_eq!(row_ids(&view), ids(&["fixture-caravan"]));
}

#[test]
fn skipped_hidden_unless_setting() {
    let mut engine = replay("act1.txt");
    engine
        .quest_action(CORISANDE, "fixture-crowbell", QuestAction::Skip)
        .unwrap();
    let ui = reading(ViewKind::Campaign);
    let hidden = build_view(&engine, &Settings::default(), &ui);
    assert!(!row_ids(&hidden).contains(&"fixture-crowbell".to_string()));
    assert!(!next_ids(&hidden).contains(&"fixture-crowbell".to_string()));

    let shown = Settings {
        show_skipped: true,
        ..Settings::default()
    };
    let view = build_view(&engine, &shown, &ui);
    let row = view.acts[0]
        .quests
        .iter()
        .find(|r| r.id == "fixture-crowbell")
        .expect("a skipped quest shows with the setting on");
    assert_eq!(row.status, Status::Skipped);
    // With the setting on, next up lists it too: its tracked status is
    // available, and it comes after the unskipped rows of its worth group.
    assert_eq!(
        next_ids(&view),
        ids(&["fixture-king", "fixture-lute", "fixture-crowbell"])
    );
    // Unskipping brings it back without the setting.
    engine
        .quest_action(CORISANDE, "fixture-crowbell", QuestAction::Unskip)
        .unwrap();
    let back = build_view(&engine, &Settings::default(), &ui);
    assert!(row_ids(&back).contains(&"fixture-crowbell".to_string()));
}

#[test]
fn only_worth_it_filter() {
    let engine = replay_with(
        data_with_worth(&[("fixture-lute", "no"), ("fixture-king", "optional")]),
        "act1.txt",
    );
    let only = Settings {
        only_worth_it: true,
        ..Settings::default()
    };
    let campaign = build_view(&engine, &only, &reading(ViewKind::Campaign));
    assert_eq!(
        row_ids(&campaign),
        ids(&[
            "fixture-riverbank",
            "fixture-beira",
            "fixture-crowbell",
            "fixture-candles",
        ])
    );
    assert!(campaign.acts[0].quests.iter().all(|r| r.worth == Worth::Yes));
    // Progress counts the whole act, not just what the filter shows.
    assert_eq!((campaign.acts[0].done, campaign.acts[0].total), (2, 6));
    let next = build_view(&engine, &only, &reading(ViewKind::NextUp));
    assert_eq!(next_ids(&next), ids(&["fixture-crowbell"]));
    assert_eq!(next.available_worth_count, 1);
    // Off: everything again.
    let all = build_view(&engine, &Settings::default(), &reading(ViewKind::Campaign));
    assert_eq!(row_ids(&all).len(), 6);
}

#[test]
fn only_worth_it_still_hides_unskipped_optional_quests() {
    // Baseline for the filter "show skipped" leaves alone: with nothing
    // skipped, "only worth-it" hides the optional king and the not-worth-it
    // lute whether "show skipped" is on or off, and the act still counts them.
    let engine = replay_with(
        data_with_worth(&[("fixture-lute", "no"), ("fixture-king", "optional")]),
        "act1.txt",
    );
    for show_skipped in [false, true] {
        let only = Settings {
            only_worth_it: true,
            show_skipped,
            ..Settings::default()
        };
        let campaign = build_view(&engine, &only, &reading(ViewKind::Campaign));
        assert_eq!(
            row_ids(&campaign),
            ids(&[
                "fixture-riverbank",
                "fixture-beira",
                "fixture-crowbell",
                "fixture-candles",
            ]),
            "show_skipped={show_skipped}"
        );
        assert_eq!((campaign.acts[0].done, campaign.acts[0].total), (2, 6));
        let next = build_view(&engine, &only, &reading(ViewKind::NextUp));
        assert_eq!(next_ids(&next), ids(&["fixture-crowbell"]), "show_skipped={show_skipped}");
    }
}

#[test]
fn show_skipped_shows_skipped_quest_under_only_worth_it() {
    // Live 2: the only skipped quest was optional and "only worth-it" was on,
    // so "show skipped" showed nothing. Crowbell is made optional and
    // skipped here; the king, optional and not skipped, is the control.
    let mut engine = replay_with(
        data_with_worth(&[("fixture-crowbell", "optional"), ("fixture-king", "optional")]),
        "act1.txt",
    );
    engine
        .quest_action(CORISANDE, "fixture-crowbell", QuestAction::Skip)
        .unwrap();
    let ui = reading(ViewKind::Campaign);
    let both = Settings {
        only_worth_it: true,
        show_skipped: true,
        ..Settings::default()
    };
    let view = build_view(&engine, &both, &ui);
    assert_eq!(
        row_ids(&view),
        ids(&[
            "fixture-riverbank",
            "fixture-beira",
            "fixture-crowbell",
            "fixture-lute",
            "fixture-candles",
        ]),
        "the skipped optional quest shows, the unskipped optional one does not"
    );
    let crowbell = view.acts[0]
        .quests
        .iter()
        .find(|r| r.id == "fixture-crowbell")
        .unwrap();
    assert_eq!(crowbell.status, Status::Skipped);
    // Next up lists the skipped quest too (its tracked status is available),
    // whatever "only worth-it" says, after the worth-it lute; the unskipped
    // optional king stays hidden.
    let next = build_view(&engine, &both, &reading(ViewKind::NextUp));
    assert_eq!(
        next_ids(&next),
        ids(&["fixture-lute", "fixture-crowbell"]),
        "{next:#?}"
    );

    // "Show skipped" off: hidden again. The act's counts are the same either
    // way (a skipped quest leaves the total).
    let only = Settings {
        only_worth_it: true,
        ..Settings::default()
    };
    let hidden = build_view(&engine, &only, &ui);
    assert!(!row_ids(&hidden).contains(&"fixture-crowbell".to_string()));
    assert_eq!(
        (hidden.acts[0].done, hidden.acts[0].total),
        (view.acts[0].done, view.acts[0].total)
    );
    assert_eq!((view.acts[0].done, view.acts[0].total), (2, 5));
}

// ---------------------------------------------------------------------------
// Skipped quests stay tracked (Context "Skipped quests stay tracked"): a
// skipped row keeps its status and shows the step its tracked progress
// reaches; "show skipped" lists it in next up too; its progress is quiet.

/// The King in the Mists' line in Freythorn, where `act1.txt` ends: it
/// reaches the king's last step, "Face the king".
fn king_line() -> String {
    client_line("The King in the Mists: Fixture line: the mists part.")
}

#[test]
fn skipped_row_shows_its_tracked_step() {
    let mut engine = replay("act1.txt");
    let row_before = campaign_row(&engine, "fixture-king");
    let detail_before = detail_of(&engine, "fixture-king");
    assert_eq!(row_before.current_step.as_deref(), Some("Face the king"));
    assert!(
        detail_before.steps.iter().any(|s| s.state == StepPhase::Current),
        "{detail_before:#?}"
    );

    engine
        .quest_action(CORISANDE, "fixture-king", QuestAction::Skip)
        .unwrap();
    let show = Settings {
        show_skipped: true,
        ..Settings::default()
    };
    let view = build_view(&engine, &show, &reading(ViewKind::Campaign));
    let row = view.acts[0]
        .quests
        .iter()
        .find(|r| r.id == "fixture-king")
        .expect("the skipped king shows with the setting on");
    assert_eq!(row.status, Status::Skipped, "the status stays skipped");
    assert_eq!(row.current_step, row_before.current_step);
    let detail = detail_of(&engine, "fixture-king");
    assert_eq!(detail.row.status, Status::Skipped);
    assert_eq!(detail.row.current_step, row_before.current_step);
    assert_eq!(step_marks(&detail), step_marks(&detail_before));

    // Evidence keeps arriving while skipped: the king's last step is reached,
    // and the skipped detail shows it done.
    feed(&mut engine, &[king_line()], true);
    let detail = detail_of(&engine, "fixture-king");
    assert_eq!(detail.row.status, Status::Skipped);
    assert!(
        detail.steps.iter().all(|s| s.state == StepPhase::Done),
        "{detail:#?}"
    );
    let skipped_marks = step_marks(&detail);
    let skipped_step = detail.row.current_step.clone();
    assert_eq!(skipped_step.as_deref(), Some("Face the king"));

    // Unskip shows exactly the status and step that evidence gives.
    engine
        .quest_action(CORISANDE, "fixture-king", QuestAction::Unskip)
        .unwrap();
    let back = detail_of(&engine, "fixture-king");
    assert_eq!(back.row.status, Status::InProgress);
    assert_eq!(back.row.current_step, skipped_step);
    assert_eq!(step_marks(&back), skipped_marks);

    // A skipped quest not picked up yet (its tracked status available)
    // carries no step, like an available row.
    engine
        .quest_action(CORISANDE, "fixture-crowbell", QuestAction::Skip)
        .unwrap();
    let view = build_view(&engine, &show, &reading(ViewKind::Campaign));
    let crowbell = view.acts[0]
        .quests
        .iter()
        .find(|r| r.id == "fixture-crowbell")
        .unwrap();
    assert_eq!((crowbell.status, crowbell.current_step.as_deref()), (Status::Skipped, None));
}

#[test]
fn next_up_hides_skipped_without_show_skipped() {
    // Baseline: with "show skipped" off, a skipped quest is in neither list,
    // in progress (the lute) or available (crowbell), whatever "only
    // worth-it" says.
    let mut engine = replay("act1.txt");
    for id in ["fixture-lute", "fixture-crowbell"] {
        engine.quest_action(CORISANDE, id, QuestAction::Skip).unwrap();
    }
    for only_worth_it in [false, true] {
        let settings = Settings {
            only_worth_it,
            ..Settings::default()
        };
        let next = build_view(&engine, &settings, &reading(ViewKind::NextUp));
        assert_eq!(next_ids(&next), ids(&["fixture-king"]), "only_worth_it={only_worth_it}");
        let campaign = build_view(&engine, &settings, &reading(ViewKind::Campaign));
        let rows = row_ids(&campaign);
        assert!(
            !rows.iter().any(|id| id == "fixture-lute" || id == "fixture-crowbell"),
            "only_worth_it={only_worth_it}: {rows:?}"
        );
    }
}

#[test]
fn next_up_lists_skipped_quest_when_show_skipped() {
    // The king made optional: the worth groups are the lute and crowbell
    // (worth it), then the king.
    let mut engine = replay_with(data_with_worth(&[("fixture-king", "optional")]), "act1.txt");
    let show = Settings {
        show_skipped: true,
        ..Settings::default()
    };
    let hide = Settings::default();
    let before = build_view(&engine, &show, &reading(ViewKind::NextUp));
    assert_eq!(
        next_ids(&before),
        ids(&["fixture-lute", "fixture-crowbell", "fixture-king"])
    );

    // The lute (in progress) skipped, and two quests whose tracked status is
    // neither available nor in progress: beira (done) and the candles
    // (unknown).
    for id in ["fixture-lute", "fixture-beira", "fixture-candles"] {
        engine.quest_action(CORISANDE, id, QuestAction::Skip).unwrap();
    }
    let view = build_view(&engine, &show, &reading(ViewKind::NextUp));
    assert_eq!(
        next_ids(&view),
        ids(&["fixture-crowbell", "fixture-lute", "fixture-king"]),
        "the skipped lute after the unskipped rows of its worth group: {view:#?}"
    );
    let lute = &view.next_up[1];
    assert_eq!(lute.status, Status::Skipped);
    assert_eq!(lute.current_step.as_deref(), Some("Return the lute"));

    // Off: nothing changes from before.
    let off = build_view(&engine, &hide, &reading(ViewKind::NextUp));
    assert_eq!(next_ids(&off), ids(&["fixture-crowbell", "fixture-king"]));

    // The counts and the badge are the same either way.
    let counts = |v: &View| (v.acts[0].done, v.acts[0].total, v.available_worth_count);
    assert_eq!(counts(&view), counts(&off));

    // Under "only worth-it": a skipped quest shows whatever its worth; the
    // unskipped optional king does not.
    engine
        .quest_action(CORISANDE, "fixture-king", QuestAction::Skip)
        .unwrap();
    let both = Settings {
        only_worth_it: true,
        show_skipped: true,
        ..Settings::default()
    };
    let view = build_view(&engine, &both, &reading(ViewKind::NextUp));
    assert_eq!(
        next_ids(&view),
        ids(&["fixture-crowbell", "fixture-lute", "fixture-king"])
    );

    // `act_done` follows the statuses alone: with every quest of the act
    // done or skipped, next up is still the act-done frame, setting or not.
    engine
        .quest_action(CORISANDE, "fixture-crowbell", QuestAction::MarkDone)
        .unwrap();
    for settings in [&show, &hide] {
        let view = build_view(&engine, settings, &reading(ViewKind::NextUp));
        assert_eq!(view.frame, Frame::EmptyActDone, "show_skipped={}", settings.show_skipped);
    }
}

#[test]
fn skipped_quest_progress_makes_no_row_change() {
    let data = data();
    // Control: the king not skipped, the line read live reaches its last step
    // and makes one row change.
    let mut control = Engine::new(data.clone());
    feed(&mut control, &session("act1.txt"), false);
    let live = feed(&mut control, &[king_line()], true);
    assert_eq!(
        change_kinds(&RowChanges::new().make(&data, &live)),
        vec![(RowChangeKind::StepDone, "fixture-king")]
    );

    // Skipped: the same line moves the tracked step (a skipped -> skipped
    // transition), and the row does not change.
    let mut engine = Engine::new(data.clone());
    feed(&mut engine, &session("act1.txt"), false);
    engine
        .quest_action(CORISANDE, "fixture-king", QuestAction::Skip)
        .unwrap();
    let live = feed(&mut engine, &[king_line()], true);
    assert!(
        live.iter().any(|t| t.quest_id == "fixture-king"
            && (t.from, t.to) == (Status::Skipped, Status::Skipped)
            && t.step_to.as_deref() == Some("face-the-king")),
        "{live:#?}"
    );
    assert!(RowChanges::new().make(&data, &live).is_empty(), "{live:#?}");
}

// ---------------------------------------------------------------------------
// Find-it tips (Context "Find-it tips").

/// The fixture data with a `tip` on each (quest id, step id) given.
fn data_with_tips(tips: &[(&str, &str, &str)]) -> Arc<QuestData> {
    let mut v = data_value();
    add_tips(&mut v, tips);
    Arc::new(QuestData::from_value(&v).expect("the data with tips validates"))
}

fn add_tips(v: &mut Value, tips: &[(&str, &str, &str)]) {
    for q in v["quests"].as_array_mut().unwrap() {
        for (quest, step, tip) in tips {
            if q["id"] != *quest {
                continue;
            }
            for s in q["steps"].as_array_mut().unwrap() {
                if s["id"] == *step {
                    s["tip"] = Value::String(tip.to_string());
                }
            }
        }
    }
}

#[test]
fn step_tip_reaches_row_and_detail() {
    // Baseline: the fixture data has no tip; no row or step carries one.
    let plain = replay("act1.txt");
    let view = build_view(&plain, &Settings::default(), &reading(ViewKind::NextUp));
    assert!(view.next_up.iter().all(|r| r.current_tip.is_none()), "{view:#?}");
    assert!(detail_of(&plain, "fixture-king").steps.iter().all(|s| s.tip.is_none()));

    // Target: the king's row shows "Face the king" next, so that step's tip
    // is the row's; the tip of a step already done is not.
    const FACE: &str = "Fixture tip: the throne is past the second bridge.";
    const HEAR: &str = "Fixture tip: ask in the encampment.";
    let mut engine = replay_with(
        data_with_tips(&[
            ("fixture-king", "face-the-king", FACE),
            ("fixture-king", "hear-of-the-king", HEAR),
        ]),
        "act1.txt",
    );
    let view = build_view(&engine, &Settings::default(), &reading(ViewKind::NextUp));
    let king = view.next_up.iter().find(|r| r.id == "fixture-king").unwrap();
    assert_eq!(king.current_step.as_deref(), Some("Face the king"));
    assert_eq!(king.current_tip.as_deref(), Some(FACE));
    // A row with no tip on its step carries none.
    let lute = view.next_up.iter().find(|r| r.id == "fixture-lute").unwrap();
    assert_eq!(lute.current_tip, None);

    // The detail shows each step's own tip, under its text.
    let detail = detail_of(&engine, "fixture-king");
    let tips: Vec<(&str, Option<&str>)> = detail
        .steps
        .iter()
        .map(|s| (s.id.as_str(), s.tip.as_deref()))
        .collect();
    assert_eq!(
        tips,
        vec![
            ("hear-of-the-king", Some(HEAR)),
            ("enter-freythorn", None),
            ("face-the-king", Some(FACE)),
        ]
    );
    assert_eq!(detail.row.current_tip.as_deref(), Some(FACE));

    // Both are contract fields: present on every row and step, null when
    // there is none.
    let row = serde_json::to_value(king).unwrap();
    assert_eq!(row["current_tip"], Value::String(FACE.into()));
    assert!(serde_json::to_value(lute).unwrap()["current_tip"].is_null());
    let step = serde_json::to_value(&detail.steps[1]).unwrap();
    assert!(step.as_object().unwrap().contains_key("tip") && step["tip"].is_null());

    // A skipped row carries its step's tip too.
    engine
        .quest_action(CORISANDE, "fixture-king", QuestAction::Skip)
        .unwrap();
    let show = Settings {
        show_skipped: true,
        ..Settings::default()
    };
    let view = build_view(&engine, &show, &reading(ViewKind::Campaign));
    let king = view.acts[0].quests.iter().find(|r| r.id == "fixture-king").unwrap();
    assert_eq!(king.status, Status::Skipped);
    assert_eq!(king.current_tip.as_deref(), Some(FACE));
}

// ---------------------------------------------------------------------------
// Settings after Live 2: opacity to 0, a scale factor, no dock.

#[test]
fn opacity_zero_is_kept() {
    assert_eq!(Settings::MIN_OPACITY, 0.0);
    let clear = Settings {
        opacity: 0.0,
        ..Settings::default()
    };
    assert_eq!(clear.clone().normalised().opacity, 0.0);
    let view = build_view(&replay("act1.txt"), &clear, &reading(ViewKind::NextUp));
    assert_eq!(view.settings.opacity, 0.0);
    let patched = Settings::default()
        .patched(&serde_json::json!({"opacity": 0}))
        .unwrap();
    assert_eq!(patched.opacity, 0.0);
    // The old floor is an ordinary value now, and loads as it is.
    let old: Settings = serde_json::from_value(serde_json::json!({"opacity": 0.3})).unwrap();
    assert_eq!(old.normalised().opacity, 0.3);
    // Outside 0-1 is clamped; a non-number is the default.
    let at = |o: f64| Settings { opacity: o, ..Settings::default() }.normalised().opacity;
    assert_eq!(at(-0.5), 0.0);
    assert_eq!(at(1.5), 1.0);
    assert_eq!(at(f64::NAN), Settings::DEFAULT_OPACITY);
}

#[test]
fn old_settings_with_dock_and_scale_strings_load() {
    // A settings.json written before Live 2: a dock side and a scale string.
    for (stored, want) in [
        ("auto", Scale::Auto),
        ("1", Scale::Fixed(1.0)),
        ("1.33", Scale::Fixed(1.33)),
        ("2", Scale::Fixed(2.0)),
    ] {
        let old = serde_json::json!({
            "opacity": 0.3,
            "dock": "left",
            "scale": stored,
            "show_skipped": true,
            "only_worth_it": false,
            "log_path": null,
            "hotkey": "F8",
        });
        let s: Settings = serde_json::from_value(old).expect("an old file loads");
        let s = s.normalised();
        assert_eq!(s.scale, want, "{stored:?}");
        assert_eq!((s.opacity, s.show_skipped, s.hotkey.as_str()), (0.3, true, "F8"));
        // Written back, it has no dock and the scale's new form.
        let v = serde_json::to_value(&s).unwrap();
        assert!(v.get("dock").is_none(), "{v}");
        let back: Settings = serde_json::from_value(v).unwrap();
        assert_eq!(back.scale, want, "{stored:?}");
    }
    // A patch that names the dock is refused as unknown and changes nothing.
    let err = Settings::default()
        .patched(&serde_json::json!({"dock": "left"}))
        .unwrap_err();
    assert!(err.contains("dock"), "{err}");
    let err = Settings::default()
        .patched(&serde_json::json!({"dock": "right", "opacity": 0.5}))
        .unwrap_err();
    assert!(err.contains("dock"), "{err}");
}

#[test]
fn scale_fixed_factor_roundtrips_and_clamps() {
    // Serialised form: the string "auto", or the factor as a JSON number.
    assert_eq!(serde_json::to_value(Scale::Auto).unwrap(), Value::String("auto".into()));
    assert_eq!(serde_json::to_value(Scale::Fixed(1.25)).unwrap(), serde_json::json!(1.25));
    assert_eq!(Settings::default().scale, Scale::Auto);
    assert_eq!(Scale::Auto.factor(), None);
    assert_eq!(Scale::Fixed(1.5).factor(), Some(1.5));

    // Every slider position, 0.75 to 2.0 in 0.05 steps, survives a round trip.
    let steps = ((Scale::MAX - Scale::MIN) / Scale::STEP).round() as u32;
    assert_eq!(steps, 25);
    for k in 0..=steps {
        let f = Scale::MIN + Scale::STEP * k as f64;
        let s = Scale::fixed(f);
        let want = (f * 100.0).round() / 100.0;
        assert_eq!(s, Scale::Fixed(want), "{f}");
        let back: Scale = serde_json::from_value(serde_json::to_value(s).unwrap()).unwrap();
        assert_eq!(back, s, "{f}");
    }

    // The slider moved: a patch with a number turns Auto off.
    let patched = Settings::default()
        .patched(&serde_json::json!({"scale": 1.5}))
        .unwrap();
    assert_eq!(patched.scale, Scale::Fixed(1.5));
    let auto = patched.patched(&serde_json::json!({"scale": "auto"})).unwrap();
    assert_eq!(auto.scale, Scale::Auto);
    // A slider's float noise lands on the 0.01 grid, the edges included.
    assert_eq!(Scale::fixed(1.1500000000000001), Scale::Fixed(1.15));
    assert_eq!(Scale::fixed(2.0000000001), Scale::Fixed(2.0));
    assert_eq!(Scale::fixed(0.7499999999), Scale::Fixed(0.75));

    // Out of 0.75-2.0, or malformed: Auto (Context "Live 2 fixes").
    for bad in [
        serde_json::json!(0.7),
        serde_json::json!(2.05),
        serde_json::json!(5),
        serde_json::json!(-1),
        serde_json::json!("0.5"),
        serde_json::json!("NaN"),
        serde_json::json!("inf"),
        serde_json::json!("big"),
        serde_json::json!(""),
        serde_json::json!(true),
        serde_json::json!(null),
        serde_json::json!([1]),
        serde_json::json!({"factor": 1}),
    ] {
        let s: Scale = serde_json::from_value(bad.clone()).unwrap();
        assert_eq!(s, Scale::Auto, "{bad}");
        let patched = Settings::default()
            .patched(&serde_json::json!({ "scale": bad.clone() }))
            .unwrap();
        assert_eq!(patched.scale, Scale::Auto, "{bad}");
    }
    // A fixed value built out of range normalises to Auto too.
    let wild = Settings {
        scale: Scale::Fixed(3.0),
        ..Settings::default()
    };
    assert_eq!(wild.normalised().scale, Scale::Auto);
    assert_eq!(Scale::fixed(f64::NAN), Scale::Auto);
}

/// Each row change as (kind, quest id).
fn change_kinds(made: &[RowChange]) -> Vec<(RowChangeKind, &str)> {
    made.iter().map(|c| (c.kind, c.quest_id.as_str())).collect()
}

#[test]
fn row_changes_only_for_live_transitions() {
    let data = data();
    // Baseline, catch-up (the lines were already in the log): the engine
    // reports transitions, and not one of them is a row change.
    let mut engine = Engine::new(data.clone());
    let catch_up = feed(&mut engine, &session("act1.txt"), false);
    assert!(!catch_up.is_empty());
    let mut changes = RowChanges::new();
    assert!(changes.make(&data, &catch_up).is_empty());

    // One live line: the riverbank is done and that unlocks the king.
    let lines = session("act1.txt");
    let renly = lines
        .iter()
        .position(|l| l.contains("Renly: "))
        .expect("act1 has Renly's line");
    let mut engine = Engine::new(data.clone());
    feed(&mut engine, &lines[..renly], false);
    let live = engine.feed_line(&lines[renly], true);
    let made = RowChanges::new().make(&data, &live);
    assert_eq!(
        change_kinds(&made),
        vec![
            (RowChangeKind::QuestDone, "fixture-riverbank"),
            (RowChangeKind::Available, "fixture-king"),
        ],
        "{live:#?}"
    );
    assert_eq!((made[0].id, made[1].id), (1, 2));
    assert!(made[1].text.contains("Fixture: The King in the Mists"));

    // A whole act live in one batch: every change, in order, no cap of
    // three and nothing dropped.
    let mut engine = Engine::new(data.clone());
    let burst = feed(&mut engine, &lines, true);
    let mut changes = RowChanges::new();
    let all = changes.make(&data, &burst);
    assert!(all.len() > 3, "{all:#?}");
    assert!(all.iter().any(|c| c.kind == RowChangeKind::StepDone));
    assert_eq!(
        all.iter().map(|c| c.id).collect::<Vec<_>>(),
        (1..=all.len() as u64).collect::<Vec<_>>(),
        "ids increase from 1"
    );
    assert_eq!(all.last().map(|c| c.quest_id.as_str()), Some("fixture-king"));
    // The ids keep increasing across batches.
    let again = changes.make(&data, &live);
    assert_eq!(again[0].id, all.len() as u64 + 1);

    // Line by line: the same changes as the batch, one per transition.
    let mut engine = Engine::new(data.clone());
    let mut changes = RowChanges::new();
    let mut seen: Vec<RowChange> = Vec::new();
    for line in &lines {
        let t = engine.feed_line(line, true);
        seen.extend(changes.make(&data, &t));
    }
    assert_eq!(seen, all);

    // Catch-up and live mixed in one batch: only the live ones count.
    let mut mixed = catch_up.clone();
    mixed.extend(live.iter().cloned());
    assert_eq!(
        change_kinds(&RowChanges::new().make(&data, &mixed)),
        change_kinds(&made)
    );
}

#[test]
fn row_change_carries_steps_and_announcement() {
    let data = data();
    let lines = session("act1.txt");
    let at = |needle: &str| {
        lines
            .iter()
            .position(|l| l.contains(needle))
            .unwrap_or_else(|| panic!("act1 has {needle:?}"))
    };
    let live_at = |i: usize| {
        let mut engine = Engine::new(data.clone());
        feed(&mut engine, &lines[..i], false);
        let t = engine.feed_line(&lines[i], true);
        RowChanges::new().make(&data, &t)
    };

    // A step reached: the step completed goes out, the next step comes in.
    let made = live_at(at("a lute string hums"));
    let lute = made
        .iter()
        .find(|c| c.quest_id == "fixture-lute")
        .unwrap_or_else(|| panic!("the lute's step is a row change: {made:#?}"));
    assert_eq!(lute.kind, RowChangeKind::StepDone);
    assert_eq!(lute.step_done.as_deref(), Some("Hear the lute"));
    assert_eq!(lute.step_next.as_deref(), Some("Return the lute"));
    for part in ["Fixture: The Lost Lute", "Hear the lute", "Return the lute"] {
        assert!(lute.text.contains(part), "{:?} lacks {part:?}", lute.text);
    }

    // A quest done: the step its row showed goes out, nothing comes in.
    let made = live_at(at("Renly: "));
    let riverbank = &made[0];
    assert_eq!(riverbank.kind, RowChangeKind::QuestDone);
    assert_eq!(riverbank.step_done.as_deref(), Some("Reach the encampment"));
    assert_eq!(riverbank.step_next, None);
    let name = &data.quest("fixture-riverbank").unwrap().name;
    assert!(riverbank.text.contains(name.as_str()), "{:?} lacks {name:?}", riverbank.text);
    // Available: no step on either side, and the sentence names the quest.
    let king = &made[1];
    assert_eq!(king.kind, RowChangeKind::Available);
    assert_eq!((king.step_done.as_deref(), king.step_next.as_deref()), (None, None));
    assert!(king.text.contains("Fixture: The King in the Mists"));

    // The payload is the contract's, exactly as serde writes it.
    let v = serde_json::to_value(lute).unwrap();
    let mut keys: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
    keys.sort();
    assert_eq!(keys, vec!["id", "kind", "quest_id", "step_done", "step_next", "text"]);
    assert_eq!(v["kind"], Value::String("step_done".into()));
    assert_eq!(RowChangeKind::QuestDone.as_str(), "quest_done");
    assert_eq!(serde_json::to_value(&made[1]).unwrap()["step_done"], Value::Null);
}

/// The fixture data with a `start` on fixture-beira: fixture-riverbank done.
fn data_with_beira_start() -> Arc<QuestData> {
    let mut v = data_value();
    for q in v["quests"].as_array_mut().unwrap() {
        if q["id"] == "fixture-beira" {
            q["start"] = serde_json::json!([
                { "kind": "quest", "id": "fixture-riverbank", "status": "done" }
            ]);
        }
    }
    Arc::new(QuestData::from_value(&v).expect("a start list validates"))
}

#[test]
fn started_quest_makes_a_row_change() {
    let lines = session("act1.txt");
    let renly = lines
        .iter()
        .position(|l| l.contains("Renly: "))
        .expect("act1 has Renly's line");
    let live_renly = |data: &Arc<QuestData>| {
        let mut engine = Engine::new(data.clone());
        feed(&mut engine, &lines[..renly], false);
        let t = engine.feed_line(&lines[renly], true);
        (engine, RowChanges::new().make(data, &t))
    };

    // Control: with no start, Renly's line read live changes no row of the
    // pack leader's quest.
    let (_, made) = live_renly(&data());
    assert!(made.iter().all(|c| c.quest_id != "fixture-beira"), "{made:#?}");

    // The start fires: one row change, a step change with no step completed
    // and the first step now current, naming the quest as started.
    let data = data_with_beira_start();
    let (mut engine, made) = live_renly(&data);
    let beira: Vec<&RowChange> = made.iter().filter(|c| c.quest_id == "fixture-beira").collect();
    assert_eq!(beira.len(), 1, "{made:#?}");
    let started = beira[0];
    assert_eq!(started.kind, RowChangeKind::StepDone);
    assert_eq!(started.step_done, None);
    assert_eq!(started.step_next.as_deref(), Some("Find the pack leader"));
    let name = &data.quest("fixture-beira").unwrap().name;
    assert!(started.text.contains(name.as_str()), "{:?} lacks {name:?}", started.text);
    assert!(started.text.contains("started"), "{:?}", started.text);
    assert!(started.text.contains("Find the pack leader"), "{:?}", started.text);
    // The other changes of that line are the ones without the start.
    assert_eq!(
        change_kinds(&made)
            .into_iter()
            .filter(|(_, q)| *q != "fixture-beira")
            .collect::<Vec<_>>(),
        vec![
            (RowChangeKind::QuestDone, "fixture-riverbank"),
            (RowChangeKind::Available, "fixture-king"),
        ]
    );
    // The row shows what the change says: in progress at its first step.
    let view = build_view(&engine, &Settings::default(), &reading(ViewKind::Campaign));
    let row = view.acts[0].quests.iter().find(|r| r.id == "fixture-beira").unwrap();
    assert_eq!(row.status, Status::InProgress);
    assert_eq!(row.current_step.as_deref(), Some("Find the pack leader"));

    // Entering its unlock area later changes nothing of the row; its step
    // signal then makes an ordinary step change.
    let rest = &lines[renly + 1..];
    let g1_2 = rest.iter().position(|l| l.contains("area \"G1_2\"")).unwrap();
    let mut changes = RowChanges::new();
    let t = feed(&mut engine, &rest[..=g1_2], true);
    assert!(
        changes.make(&data, &t).iter().all(|c| c.quest_id != "fixture-beira"),
        "{t:#?}"
    );
    let pack = rest.iter().position(|l| l.contains("the pack gathers")).unwrap();
    let t = feed(&mut engine, &rest[g1_2 + 1..=pack], true);
    let step = changes.make(&data, &t);
    let step: Vec<&RowChange> = step.iter().filter(|c| c.quest_id == "fixture-beira").collect();
    assert_eq!(step.len(), 1, "{step:#?}");
    assert_eq!(step[0].kind, RowChangeKind::StepDone);
    assert_eq!(step[0].step_done.as_deref(), Some("Find the pack leader"));

    // Read in catch-up, the start makes no row change.
    let mut engine = Engine::new(data.clone());
    let t = feed(&mut engine, &lines[..=renly], false);
    assert!(t.iter().any(|t| t.quest_id == "fixture-beira" && t.to == Status::InProgress));
    assert!(RowChanges::new().make(&data, &t).is_empty());
}

#[test]
fn new_character_view_before_level_2() {
    let lines = session("act1.txt");
    let riverbank = lines
        .iter()
        .position(|l| l.contains("Generating level 1 area \"G1_1\""))
        .expect("act1 enters The Riverbank at level 1");
    let level2 = lines
        .iter()
        .position(|l| l.contains("is now level 2"))
        .expect("act1 has the first level-up");
    assert!(riverbank < level2);

    // Baseline: before the Riverbank entry there is no character.
    let mut engine = Engine::new(data());
    feed(&mut engine, &lines[..riverbank], false);
    let view = build_view(&engine, &Settings::default(), &reading(ViewKind::NextUp));
    assert_eq!(view.frame, Frame::EmptyNoCharacter);
    assert!(view.character.is_none());

    // While the entry's relog window is open nothing changes yet.
    feed(&mut engine, &lines[riverbank..level2], false);
    assert!(engine.relog_window_open());
    assert!(!engine.new_character_open());
    let view = build_view(&engine, &Settings::default(), &reading(ViewKind::NextUp));
    assert_eq!(view.frame, Frame::EmptyNoCharacter);

    // Target: once the window closes (here the shell's expiry call, 6 s
    // after the entry) the new character is the active one, unnamed and
    // provisional, with its act 1 rows, never in the list.
    let entry = Timestamp::parse(&lines[riverbank][..19]).unwrap();
    engine.expire_relog_window(entry.plus_seconds(6));
    assert!(engine.new_character_open());
    let view = build_view(&engine, &Settings::default(), &reading(ViewKind::Campaign));
    assert_eq!(view.frame, Frame::PanelCampaign, "{view:#?}");
    let ch = view.character.clone().expect("the new character shows");
    assert!(ch.provisional && ch.own && ch.active && !ch.pinned, "{ch:?}");
    assert_eq!((ch.name.as_str(), ch.level), ("", 1));
    assert!(view.characters.is_empty(), "{:?}", view.characters);
    assert_eq!(view.acts.len(), 1);
    assert_eq!(view.acts[0].act, 1);
    let row = view.acts[0]
        .quests
        .iter()
        .find(|r| r.id == "fixture-riverbank")
        .expect("the riverbank row");
    assert_eq!(row.status, Status::InProgress);
    assert_eq!(row.current_step.as_deref(), Some("Reach the encampment"));
    // Not "not confirmed": the new character is the player's own.
    assert_ne!(view.status.as_ref().map(|s| s.kind), Some(StatusKind::Warn));
    let v = serde_json::to_value(&view).unwrap();
    assert_eq!(v["character"]["provisional"], Value::Bool(true));
    assert_eq!(v["character"]["name"], Value::String(String::new()));
    // Its detail opens like any character's.
    let mut ui = reading(ViewKind::NextUp);
    ui.detail = Some("fixture-riverbank".into());
    let view = build_view(&engine, &Settings::default(), &ui);
    assert_eq!(view.frame, Frame::QuestDetail);

    // The first level-up names it: no longer provisional, listed, same rows.
    feed(&mut engine, &lines[level2..=level2], false);
    assert!(!engine.new_character_open());
    let view = build_view(&engine, &Settings::default(), &reading(ViewKind::Campaign));
    let ch = view.character.clone().expect("the named character");
    assert_eq!((ch.name.as_str(), ch.level, ch.provisional), (CORISANDE, 2, false));
    assert!(ch.own && ch.active);
    assert_eq!(view.characters, vec![ch]);
    let row = view.acts[0]
        .quests
        .iter()
        .find(|r| r.id == "fixture-riverbank")
        .unwrap();
    assert_eq!(row.status, Status::InProgress);
}

/// A client line, for the inline cases.
fn client_line(text: &str) -> String {
    format!("2026/03/08 18:20:00 2800000 5c0a0041 [INFO Client 5120] {text}")
}

#[test]
fn refused_count_in_status() {
    let data = data();
    // Baseline: an own character's reward with the area known is no refusal
    // and says nothing.
    let reward = client_line(": Corisande_Fell has received +10% to [Resistances|Cold Resistance].");
    let mut known = Engine::new(data.clone());
    feed(&mut known, &[area_line("G1_2"), reward.clone()], true);
    assert_eq!(known.refused_count(), 0);
    assert!(build_view(&known, &Settings::default(), &reading(ViewKind::Campaign))
        .status
        .is_none());

    // Target: the same reward before any area line. The pack leader's reward
    // is tied to its area, the area is unknown, so the signal is refused and
    // the status says how many.
    let mut engine = Engine::new(data.clone());
    feed(&mut engine, &[reward.clone()], true);
    assert_eq!((engine.refused_count(), engine.dropped_count()), (1, 0));
    let view = build_view(&engine, &Settings::default(), &reading(ViewKind::Campaign));
    let status = view.status.expect("a status line for the refused signal");
    assert_eq!(status.kind, StatusKind::Info);
    assert!(status.text.contains("1 quest signal "), "{status:?}");
    assert!(status.text.contains("area"), "{status:?}");

    // Dropped and refused together: one line names both counts.
    let opening = "2026/03/08 18:30:00 ***** LOG FILE OPENING *****".to_string();
    let npc = client_line("Renly: Fixture line: welcome to the encampment.");
    feed(&mut engine, &[opening.clone(), npc, opening], true);
    let (dropped, refused) = (engine.dropped_count(), engine.refused_count());
    assert!(dropped > 0 && refused > 0, "dropped {dropped}, refused {refused}");
    let view = build_view(&engine, &Settings::default(), &reading(ViewKind::Campaign));
    let text = view.status.expect("a status line").text;
    assert!(
        text.contains(&format!("{dropped} log event")) && text.contains(&format!("{refused} quest signal")),
        "{text}"
    );
}

/// The level-1 entry to one instance of The Riverbank.
fn riverbank_entry(seed: u64) -> String {
    client_line(&format!(r#"Generating level 1 area "G1_1" with seed {seed}"#))
}

#[test]
fn status_tells_dropped_new_character_events_apart() {
    let marker = "2026/03/08 18:30:00 ***** LOG FILE OPENING *****".to_string();
    let walk = vec![
        client_line("[SCENE] Set Source [The Riverbank]"),
        area_line("G1_town"),
        client_line("[SCENE] Set Source [Clearfell Encampment]"),
    ];

    // Baseline: events held with no character, dropped at a session marker,
    // are told as "before any character".
    let mut held = Engine::new(data());
    feed(
        &mut held,
        &[marker.clone(), area_line("G1_town"), marker.clone()],
        true,
    );
    assert_eq!(
        (held.dropped_count(), held.dropped_new_character_count()),
        (1, 0)
    );
    let text = build_view(&held, &Settings::default(), &reading(ViewKind::NextUp))
        .status
        .expect("a status line for the dropped event")
        .text;
    assert_eq!(text, "1 log event came before any character and was not counted.");

    // Target: a new character from The Riverbank parked at a marker, then
    // replaced by a level-1 entry to another instance. Its events are told
    // apart, in their own words, not as "before any character".
    let mut engine = Engine::new(data());
    let mut lines = vec![marker.clone(), riverbank_entry(11)];
    lines.push(client_line("[SCENE] Set Source [The Riverbank]"));
    lines.push(marker.clone());
    lines.push(riverbank_entry(77));
    lines.extend(walk.clone());
    feed(&mut engine, &lines, true);
    let replaced = engine.dropped_new_character_count();
    assert!(replaced > 0, "the parked record's events are dropped");
    assert_eq!(engine.dropped_count(), 0);
    let view = build_view(&engine, &Settings::default(), &reading(ViewKind::NextUp));
    let status = view.status.expect("a status line for the replaced character");
    assert_eq!(status.kind, StatusKind::Info);
    assert!(status.text.contains(&format!("{replaced} log event")), "{status:?}");
    assert!(status.text.contains("new character"), "{status:?}");
    assert!(!status.text.contains("before any character"), "{status:?}");

    // Both at once: one line, each count with its own reason.
    let mut both = Engine::new(data());
    feed(
        &mut both,
        &[marker.clone(), area_line("G1_town"), marker.clone()],
        true,
    );
    feed(&mut both, &lines, true);
    let (d, n) = (both.dropped_count(), both.dropped_new_character_count());
    assert!(d > 0 && n > 0, "dropped {d}, replaced {n}");
    let text = build_view(&both, &Settings::default(), &reading(ViewKind::NextUp))
        .status
        .expect("a status line")
        .text;
    assert!(
        text.contains(&format!("{d} log event")) && text.contains("before any character"),
        "{text}"
    );
    assert!(
        text.contains(&format!("{n} log event")) && text.contains("another new character replaced"),
        "{text}"
    );
    assert!(text.ends_with("none was counted."), "{text}");
}

// ---------------------------------------------------------------------------
// The inferred mark.

/// The row of a quest in the campaign view.
fn campaign_row(engine: &Engine, id: &str) -> qt_viewmodel::QuestRow {
    let view = build_view(engine, &Settings::default(), &reading(ViewKind::Campaign));
    view.acts
        .iter()
        .flat_map(|a| a.quests.iter())
        .find(|r| r.id == id)
        .unwrap_or_else(|| panic!("no row {id}: {view:#?}"))
        .clone()
}

/// The detail of a quest, opened.
fn detail_of(engine: &Engine, id: &str) -> qt_viewmodel::QuestDetail {
    let mut ui = reading(ViewKind::NextUp);
    ui.mode = Mode::Edit;
    ui.detail = Some(id.to_string());
    build_view(engine, &Settings::default(), &ui)
        .detail
        .unwrap_or_else(|| panic!("no detail for {id}"))
}

/// Each step's id with its inferred mark.
fn step_marks(detail: &qt_viewmodel::QuestDetail) -> Vec<(String, StepPhase, bool)> {
    detail
        .steps
        .iter()
        .map(|s| (s.id.clone(), s.state, s.inferred))
        .collect()
}

#[test]
fn inferred_done_quest_is_marked() {
    let data = data();
    let riverbank = data.quest("fixture-riverbank").unwrap();
    assert!(riverbank.done_inferred, "the fixture's riverbank carries done_inferred");

    // Baseline: before the welcome line that is its done, the row is in
    // progress and carries no mark.
    let lines = session("act1.txt");
    let welcome = lines
        .iter()
        .position(|l| l.contains("Renly: Fixture line: welcome to the encampment."))
        .expect("act1 has the welcome line");
    let mut engine = Engine::new(data.clone());
    feed(&mut engine, &lines[..welcome], false);
    let row = campaign_row(&engine, "fixture-riverbank");
    assert_eq!(row.status, Status::InProgress);
    assert!(!row.inferred, "{row:?}");

    // Target: the welcome line makes it done, and done by inference.
    feed(&mut engine, &lines[welcome..], false);
    let row = campaign_row(&engine, "fixture-riverbank");
    assert_eq!(row.status, Status::Done);
    assert!(row.inferred, "{row:?}");
    // A mark only: the manual-tick marker follows the detection alone.
    assert_eq!(row.needs_tick, riverbank.detection.needs_tick());
    assert!(!row.needs_tick);
    // The serialised field is the contract's name.
    let v = serde_json::to_value(&row).unwrap();
    assert_eq!(v["inferred"], Value::Bool(true));
    // The detail's row carries it too; its steps each fired, so none is.
    let detail = detail_of(&engine, "fixture-riverbank");
    assert!(detail.row.inferred);
    assert!(detail.steps.iter().all(|s| !s.inferred), "{:?}", step_marks(&detail));
}

#[test]
fn inferred_step_is_marked() {
    let data = data();
    let mut engine = replay("act1.txt");

    // A later step fired: the king's first step leaves no trace, so entering
    // Freythorn shows it done by inference; the step that fired is not.
    let detail = detail_of(&engine, "fixture-king");
    assert_eq!(detail.row.status, Status::InProgress);
    assert!(!detail.row.inferred, "an in-progress row is never inferred");
    assert_eq!(
        step_marks(&detail),
        vec![
            ("hear-of-the-king".to_string(), StepPhase::Done, true),
            ("enter-freythorn".to_string(), StepPhase::Done, false),
            ("face-the-king".to_string(), StepPhase::Current, false),
        ]
    );
    // The row still shows the step to do next.
    assert_eq!(detail.row.current_step.as_deref(), Some("Face the king"));
    let v = serde_json::to_value(&detail).unwrap();
    assert_eq!(v["steps"][0]["inferred"], Value::Bool(true));

    // The quest done by its own signal (a spirit reward): every step it never
    // saw is inferred, the row is not (the quest has no done_inferred).
    let reward = client_line(": Corisande_Fell has received +30 to Spirit.");
    feed(&mut engine, &[reward], true);
    let detail = detail_of(&engine, "fixture-king");
    assert_eq!(detail.row.status, Status::Done);
    assert!(!detail.row.inferred);
    assert_eq!(
        step_marks(&detail),
        vec![
            ("hear-of-the-king".to_string(), StepPhase::Done, true),
            ("enter-freythorn".to_string(), StepPhase::Done, false),
            ("face-the-king".to_string(), StepPhase::Done, true),
        ]
    );
    // The mark changes no tick: none of the steps is ticked, and the row's
    // manual-tick marker follows the detection alone.
    assert!(detail.steps.iter().all(|s| !s.ticked));
    let king = data.quest("fixture-king").unwrap();
    assert_eq!(detail.row.needs_tick, king.detection.needs_tick());

    // A step the user ticked is the user's word, not an inference; the step
    // before it, which nothing showed, is inferred.
    let mut engine = story_engine();
    let lute = detail_of(&engine, "fixture-lute");
    assert!(lute.steps.iter().all(|s| s.state != StepPhase::Done), "{lute:?}");
    engine
        .tick_step(CORISANDE, "fixture-lute", "return-the-lute", true)
        .unwrap();
    let lute = detail_of(&engine, "fixture-lute");
    assert_eq!(
        step_marks(&lute),
        vec![
            ("hear-the-lute".to_string(), StepPhase::Done, true),
            ("return-the-lute".to_string(), StepPhase::Done, false),
        ]
    );
    assert!(lute.steps[1].ticked);
}

#[test]
fn own_signal_done_is_not_inferred() {
    // The pack leader is done by its own reward line and carries no
    // done_inferred: no mark on the row, none on the step that fired.
    let engine = replay("act1.txt");
    let row = campaign_row(&engine, "fixture-beira");
    assert_eq!(row.status, Status::Done);
    assert!(!row.inferred, "{row:?}");
    let detail = detail_of(&engine, "fixture-beira");
    assert_eq!(
        step_marks(&detail),
        vec![("meet-beira".to_string(), StepPhase::Done, false)]
    );

    // Negative control: the same quest data with done_inferred set on the
    // pack leader marks the same row.
    let mut v = data_value();
    for q in v["quests"].as_array_mut().unwrap() {
        if q["id"] == "fixture-beira" {
            q["done_inferred"] = Value::Bool(true);
        }
    }
    let flagged = Arc::new(QuestData::from_value(&v).expect("the flagged data validates"));
    let engine = replay_with(flagged, "act1.txt");
    assert!(campaign_row(&engine, "fixture-beira").inferred);

    // Rows that are not done carry no mark, whatever their data says.
    let engine = replay("act1.txt");
    let view = build_view(&engine, &Settings::default(), &reading(ViewKind::Campaign));
    for row in view.acts.iter().flat_map(|a| a.quests.iter()) {
        if row.status != Status::Done {
            assert!(!row.inferred, "{row:?}");
        }
    }
}

#[test]
fn user_marked_done_is_not_inferred() {
    // A quest the user marked done is done on the user's word: no mark on
    // the row, even with done_inferred, and none on its steps.
    let mut engine = replay("act1.txt");
    engine
        .quest_action(CORISANDE, "fixture-riverbank", QuestAction::MarkDone)
        .unwrap();
    let row = campaign_row(&engine, "fixture-riverbank");
    assert_eq!(row.status, Status::Done);
    assert!(!row.inferred, "{row:?}");

    engine
        .quest_action(CORISANDE, "fixture-king", QuestAction::MarkDone)
        .unwrap();
    let detail = detail_of(&engine, "fixture-king");
    assert_eq!(detail.row.status, Status::Done);
    assert!(!detail.row.inferred);
    assert!(detail.steps.iter().all(|s| s.state == StepPhase::Done));
    assert!(detail.steps.iter().all(|s| !s.inferred), "{:?}", step_marks(&detail));

    // Positive control: reset, the evidence alone shows the riverbank done by
    // inference again.
    engine
        .quest_action(CORISANDE, "fixture-riverbank", QuestAction::Reset)
        .unwrap();
    assert!(campaign_row(&engine, "fixture-riverbank").inferred);
}

#[test]
fn empty_and_error_states() {
    let data = data();
    let fresh = Engine::new(data.clone());
    let settings = Settings::default();

    // Baseline: nothing read yet.
    let missing = UiState {
        log: LogState::Missing,
        ..UiState::default()
    };
    let view = build_view(&fresh, &settings, &missing);
    assert_eq!(view.frame, Frame::EmptyNoLog);
    assert_eq!(view.status.as_ref().map(|s| s.kind), Some(StatusKind::Error));
    assert!(view.character.is_none() && view.acts.is_empty() && view.next_up.is_empty());
    // Settings stay reachable without a log, so the path can be set.
    let mut open = missing.clone();
    open.open_frame(Some(Frame::Settings)).unwrap();
    let view = build_view(&fresh, &settings, &open);
    assert_eq!(view.frame, Frame::Settings);
    assert_eq!(view.status.as_ref().map(|s| s.kind), Some(StatusKind::Error));
    // Only the picker and settings open as frames.
    assert!(open.open_frame(Some(Frame::PanelCampaign)).is_err());

    let waiting = UiState::default();
    assert_eq!(waiting.log, LogState::Waiting);
    let view = build_view(&fresh, &settings, &waiting);
    assert_eq!(view.frame, Frame::EmptyWaiting);
    assert!(view.status.is_none());

    // Lines that name no character: no character detected yet.
    let baseline = replay("baseline.txt");
    let view = build_view(&baseline, &settings, &reading(ViewKind::NextUp));
    assert_eq!(view.frame, Frame::EmptyNoCharacter);
    assert!(view.characters.is_empty());

    // A session with no character drops its events and says so. (`held.txt`'s
    // first session parks a new character instead of dropping it, so the
    // second session is ended here, before its level-ups, with its area
    // lines held. Odile's level-up alone then leaves her unconfirmed, a
    // warning that would come first; confirmed, the dropped count is what
    // is left to say.)
    let lines = session("held.txt");
    let mut held = Engine::new(data.clone());
    feed(&mut held, &lines[..6], false);
    held.end_session();
    feed(&mut held, &lines[6..], false);
    assert!(held.dropped_count() > 0);
    held.confirm("Odile_Marsh").unwrap();
    let view = build_view(&held, &settings, &reading(ViewKind::NextUp));
    let status = view.status.expect("a status line for dropped events");
    assert_eq!(status.kind, StatusKind::Info);
    assert!(status.text.contains(&held.dropped_count().to_string()), "{status:?}");

    // Every act-1 quest done: next up shows the act-done state; the campaign
    // view still lists the done rows.
    let mut engine = replay("act1.txt");
    for id in ["fixture-crowbell", "fixture-king", "fixture-lute", "fixture-candles"] {
        engine
            .quest_action(CORISANDE, id, QuestAction::MarkDone)
            .unwrap();
    }
    let view = build_view(&engine, &settings, &reading(ViewKind::NextUp));
    assert_eq!(view.frame, Frame::EmptyActDone);
    assert!(view.next_up.is_empty());
    let view = build_view(&engine, &settings, &reading(ViewKind::Campaign));
    assert_eq!(view.frame, Frame::PanelCampaign);
    assert_eq!((view.acts[0].done, view.acts[0].total), (6, 6));

    // A detail for a quest the data does not have falls back to the panel
    // with a warning, never to an empty detail.
    let engine = replay("act1.txt");
    let mut ui = reading(ViewKind::NextUp);
    ui.detail = Some("no-such-quest".to_string());
    let view = build_view(&engine, &settings, &ui);
    assert_eq!(view.frame, Frame::PanelNextUp);
    assert!(view.detail.is_none());
    assert_eq!(view.status.as_ref().map(|s| s.kind), Some(StatusKind::Warn));
    // The shell's own status line wins over a derived one.
    ui.status = Some(qt_viewmodel::StatusLine::error("Shortcut Ctrl+Alt+Q is taken"));
    let view = build_view(&engine, &settings, &ui);
    assert_eq!(view.status.unwrap().text, "Shortcut Ctrl+Alt+Q is taken");

    // A level-up alone makes an unconfirmed character: the panel says so.
    // (Without the level-1 Riverbank entry, which would start a new
    // character that the level-up names as the player's own.)
    let lines: Vec<String> = session("act1.txt")
        .into_iter()
        .filter(|l| !l.contains("area \"G1_1\""))
        .collect();
    let first_received = lines.iter().position(|l| l.contains("has received")).unwrap();
    let mut engine = Engine::new(data.clone());
    feed(&mut engine, &lines[..first_received], false);
    let view = build_view(&engine, &settings, &reading(ViewKind::NextUp));
    let ch = view.character.expect("the levelled character is active");
    assert!(!ch.own);
    assert_eq!(view.status.map(|s| s.kind), Some(StatusKind::Warn));

    // Collapsed wins over the panel, and settings out of range are clamped.
    let mut ui = reading(ViewKind::NextUp);
    ui.collapsed = true;
    let wild = Settings {
        opacity: -0.05,
        ..Settings::default()
    };
    let view = build_view(&replay("act1.txt"), &wild, &ui);
    assert_eq!(view.frame, Frame::Collapsed);
    assert_eq!(view.settings.opacity, Settings::MIN_OPACITY);
    assert!(Settings::default()
        .patched(&serde_json::json!({"no_such_setting": 1}))
        .is_err());
    let patched = Settings::default()
        .patched(&serde_json::json!({"scale": "1.33"}))
        .unwrap();
    assert_eq!(serde_json::to_value(&patched).unwrap()["scale"], serde_json::json!(1.33));
}

// ---------------------------------------------------------------------------
// The fixtures the UI tests read.

/// The data the frames are built on: the fixture data with two quests' worth
/// (and its reason) changed on this copy, so the rows carry every worth, and
/// a find-it tip on the step the king's row shows next, so the preview
/// carries one. `fixtures/quests_min.json` itself stays as the engine and
/// replay tests read it (the engine's tip test holds it free of tips).
fn frames_data() -> Arc<QuestData> {
    let mut v = data_value();
    add_tips(
        &mut v,
        &[("fixture-king", "face-the-king", "Fixture tip: the throne is past the second bridge.")],
    );
    for q in v["quests"].as_array_mut().unwrap() {
        let (worth, reason) = match q["id"].as_str() {
            Some("fixture-king") => ("optional", "Spirit, but a long fight for it this early."),
            Some("fixture-beira") => ("no", "The same resistance comes cheaper later."),
            _ => continue,
        };
        q["worth"] = Value::String(worth.into());
        q["worth_reason"] = Value::String(reason.into());
    }
    Arc::new(QuestData::from_value(&v).expect("the frames' data validates"))
}

/// The frames' one session (`fixtures/sessions/frames/story.txt`).
fn story_lines() -> Vec<String> {
    session("frames/story.txt")
}

/// The story read to its end, the pack leader then skipped: act 1 holds one
/// quest in each status (riverbank done, beira skipped, crowbell locked, king
/// in progress, lute available, candles unknown), for Corisande at level 5,
/// with Odile as the second own character.
fn story_engine() -> Engine {
    let mut engine = Engine::new(frames_data());
    feed(&mut engine, &story_lines(), false);
    engine
        .quest_action(CORISANDE, "fixture-beira", QuestAction::Skip)
        .unwrap();
    engine
}

/// One serialised value per frame id of the brief. Every frame
/// that shows a character is the same story at the same point; only
/// `empty-act-done` moves it on, by marking the rest of act 1 done.
fn fixture_values() -> Vec<(String, Value)> {
    let to = |v: &View| serde_json::to_value(v).unwrap();
    let mut out: Vec<(String, Value)> = Vec::new();
    let story = story_engine();

    let shown = Settings {
        show_skipped: true,
        ..Settings::default()
    };
    out.push((
        "panel-campaign".into(),
        to(&build_view(&story, &shown, &reading(ViewKind::Campaign))),
    ));
    out.push((
        "panel-next-up".into(),
        to(&build_view(&story, &Settings::default(), &reading(ViewKind::NextUp))),
    ));
    let mut ui = reading(ViewKind::NextUp);
    ui.collapsed = true;
    out.push(("collapsed".into(), to(&build_view(&story, &Settings::default(), &ui))));

    let mut ui = reading(ViewKind::NextUp);
    ui.mode = Mode::Edit;
    ui.detail = Some("fixture-king".into());
    out.push(("quest-detail".into(), to(&build_view(&story, &Settings::default(), &ui))));

    let mut ui = reading(ViewKind::NextUp);
    ui.mode = Mode::Edit;
    ui.open_frame(Some(Frame::CharacterPicker)).unwrap();
    out.push(("character-picker".into(), to(&build_view(&story, &Settings::default(), &ui))));

    let mut ui = reading(ViewKind::NextUp);
    ui.mode = Mode::Edit;
    ui.open_frame(Some(Frame::Settings)).unwrap();
    let settings = Settings {
        log_path: Some(
            r"C:\Program Files (x86)\Grinding Gear Games\Path of Exile 2\logs\Client.txt".into(),
        ),
        ..Settings::default()
    };
    out.push(("settings".into(), to(&build_view(&story, &settings, &ui))));

    let fresh = Engine::new(data());
    let missing = UiState {
        log: LogState::Missing,
        ..UiState::default()
    };
    out.push(("empty-no-log".into(), to(&build_view(&fresh, &Settings::default(), &missing))));
    out.push((
        "empty-waiting".into(),
        to(&build_view(&fresh, &Settings::default(), &UiState::default())),
    ));
    out.push((
        "empty-no-character".into(),
        to(&build_view(&replay("baseline.txt"), &Settings::default(), &reading(ViewKind::NextUp))),
    ));

    // The same story with the rest of act 1 marked done (the pack leader stays
    // skipped, which leaves the act's total).
    let mut done = story_engine();
    for id in ["fixture-crowbell", "fixture-king", "fixture-lute", "fixture-candles"] {
        done.quest_action(CORISANDE, id, QuestAction::MarkDone).unwrap();
    }
    out.push((
        "empty-act-done".into(),
        to(&build_view(&done, &Settings::default(), &reading(ViewKind::NextUp))),
    ));
    out
}

/// The story's last line (Una's, which makes the lute available) read live
/// after a catch-up of the rest: the row change the frames' moment ends on.
fn story_row_change() -> RowChange {
    let data = frames_data();
    let lines = story_lines();
    let (last, before) = lines.split_last().unwrap();
    let mut engine = Engine::new(data.clone());
    feed(&mut engine, before, false);
    let live = engine.feed_line(last, true);
    RowChanges::new()
        .make(&data, &live)
        .into_iter()
        .next()
        .expect("Una's line read live makes the lute available")
}

fn update_fixtures() -> bool {
    std::env::var("QT_UPDATE_FIXTURES").is_ok_and(|v| v == "1")
}

/// The view fixtures as the UI reads them: the files on disk, or, while
/// `QT_UPDATE_FIXTURES=1` rewrites them, the values being written (which
/// `fixtures_match_serialised_views` otherwise holds equal to the files).
fn generated_fixtures() -> Vec<(String, Value)> {
    if update_fixtures() {
        return fixture_values();
    }
    let dir = fixtures().join("viewmodels");
    let mut out: Vec<(String, Value)> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .map(|p| {
            let name = p.file_stem().unwrap().to_string_lossy().into_owned();
            let text = std::fs::read_to_string(&p).unwrap();
            (name, serde_json::from_str(&text).unwrap())
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    assert!(out.len() > 1, "no view fixtures in {}", dir.display());
    out
}

/// Every quest row a view carries: the acts' rows, next up and the detail's.
fn rows_of(view: &Value) -> Vec<&Value> {
    let mut rows: Vec<&Value> = view["acts"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|a| a["quests"].as_array().unwrap())
        .collect();
    rows.extend(view["next_up"].as_array().unwrap());
    if !view["detail"].is_null() {
        rows.push(&view["detail"]["row"]);
    }
    rows
}

fn text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

#[test]
fn fixture_frames_share_one_character_state() {
    let files = generated_fixtures();
    let views: Vec<&(String, Value)> = files.iter().collect();

    // One active character in every frame that shows one, and each character
    // at one class and level wherever it is listed (the picker's other
    // character too).
    let mut active = std::collections::BTreeSet::new();
    let mut seen: std::collections::BTreeMap<String, std::collections::BTreeSet<String>> =
        Default::default();
    for (name, view) in &views {
        let listed = view["characters"].as_array().unwrap();
        if !view["character"].is_null() {
            let ch = &view["character"];
            active.insert(text(&ch["name"]));
            let marked: Vec<&Value> = listed.iter().filter(|c| c["active"] == true).collect();
            assert_eq!(marked, vec![ch], "{name}: the listed active character is the header's");
        }
        for c in listed.iter().chain(std::iter::once(&view["character"])) {
            if c.is_null() {
                continue;
            }
            let at = format!("{} {} own={}", text(&c["class"]), text(&c["level"]), c["own"]);
            seen.entry(text(&c["name"])).or_default().insert(at);
        }
    }
    assert_eq!(active.len(), 1, "one active character across the frames: {active:?}");
    for (who, at) in &seen {
        assert_eq!(at.len(), 1, "{who} differs between frames: {at:?}");
    }
    assert!(seen.len() > 1, "the picker has a second character: {seen:?}");

    // One status (and worth and next step) per quest across the frames,
    // except the frame whose scenario moves the story on.
    let mut quests: std::collections::BTreeMap<String, std::collections::BTreeSet<String>> =
        Default::default();
    for (name, view) in &views {
        if name == "empty-act-done" {
            continue;
        }
        for row in rows_of(view) {
            let state = format!(
                "{} worth={} step={}",
                text(&row["status"]),
                text(&row["worth"]),
                text(&row["current_step"])
            );
            quests.entry(text(&row["id"])).or_default().insert(state);
        }
    }
    assert!(!quests.is_empty());
    for (id, states) in &quests {
        assert_eq!(states.len(), 1, "{id} differs between frames: {states:?}");
    }

    // An act's progress is counted from its rows: done rows over every row
    // but the skipped ones (the viewmodel's rule; "show skipped" lists a
    // skipped row without counting it). Checked where no filter hides rows.
    for (name, view) in &views {
        if view["settings"]["only_worth_it"] == true {
            continue;
        }
        for act in view["acts"].as_array().unwrap() {
            if act["expanded"] != true {
                continue;
            }
            let rows = act["quests"].as_array().unwrap();
            let done = rows.iter().filter(|r| r["status"] == "done").count();
            let counted = rows.iter().filter(|r| r["status"] != "skipped").count();
            assert_eq!(act["done"], Value::from(done), "{name} act {}", act["act"]);
            assert_eq!(act["total"], Value::from(counted), "{name} act {}", act["act"]);
        }
    }

    // The row change the story ends on announces what the frames show.
    let change = story_row_change();
    let shown_as = match change.kind {
        RowChangeKind::Available => "available",
        RowChangeKind::StepDone => "in_progress",
        RowChangeKind::QuestDone => "done",
    };
    let quest = change.quest_id.clone();
    let states = quests.get(&quest).unwrap_or_else(|| panic!("no frame shows {quest}"));
    assert!(
        states.iter().all(|s| s.starts_with(&format!("{shown_as} "))),
        "the row change says {quest} is {shown_as}; the frames show {states:?}"
    );
}

#[test]
fn fixture_rows_cover_every_worth() {
    let files = generated_fixtures();
    let mut worth = std::collections::BTreeSet::new();
    let mut detection = std::collections::BTreeSet::new();
    let mut ticks = std::collections::BTreeSet::new();
    for (_, view) in &files {
        for row in rows_of(view) {
            worth.insert(text(&row["worth"]));
            detection.insert(text(&row["detection"]));
            ticks.insert(row["needs_tick"].as_bool().unwrap());
        }
    }
    let names = |list: &[&str]| list.iter().map(|s| s.to_string()).collect::<std::collections::BTreeSet<_>>();
    assert_eq!(worth, names(&Worth::NAMES), "every worth across the rows");
    assert_eq!(
        detection,
        names(&qt_engine::Detection::NAMES),
        "every detection across the rows"
    );
    assert_eq!(ticks.len(), 2, "auto and manual-tick rows: {ticks:?}");

    // The preview's default frame shows both an auto row and a manual-tick
    // one, and more than one worth.
    let next = &files.iter().find(|(n, _)| n == "panel-next-up").expect("panel-next-up.json").1;
    let rows = next["next_up"].as_array().unwrap();
    assert!(rows.iter().any(|r| r["needs_tick"] == true), "{rows:#?}");
    assert!(rows.iter().any(|r| r["needs_tick"] == false), "{rows:#?}");
    let worths: std::collections::BTreeSet<String> = rows.iter().map(|r| text(&r["worth"])).collect();
    assert!(worths.len() > 1, "{worths:?}");
}

/// The frame ids of the brief's `## Export contract`.
fn brief_list(label: &str) -> Vec<String> {
    let path = root().join("../../docs/quest-tracker/ui-design-brief.md");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let text = text.replace('\r', "");
    let line = text
        .split('\n')
        .find_map(|l| l.strip_prefix(label))
        .unwrap_or_else(|| panic!("the brief has no {label:?} line"));
    line.split(',')
        .map(|s| s.trim().trim_matches('`').to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[test]
fn fixtures_match_serialised_views() {
    let dir = fixtures().join("viewmodels");
    let values = fixture_values();

    // Every frame id of the brief has a fixture (the two mode frames are the
    // same panel in each mode, so they are a field, not a file).
    let mut want: Vec<String> = brief_list("Frame ids:")
        .into_iter()
        .filter(|f| f != "mode-play" && f != "mode-edit")
        .collect();
    want.sort();
    let mut have: Vec<String> = values.iter().map(|(n, _)| n.clone()).collect();
    have.sort();
    assert_eq!(have, want);

    let update = update_fixtures();
    if update {
        std::fs::create_dir_all(&dir).unwrap();
    }
    let mut stale = Vec::new();
    for (name, value) in &values {
        assert_eq!(value["frame"], Value::String(name.clone()), "{name}");
        assert_eq!(value["schema"], Value::from(1));
        let path = dir.join(format!("{name}.json"));
        if update {
            let text = serde_json::to_string_pretty(value).unwrap() + "\n";
            std::fs::write(&path, text).unwrap();
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!("{}: {e} (run with QT_UPDATE_FIXTURES=1)", path.display())
        });
        let on_disk: Value = serde_json::from_str(&text).unwrap();
        if &on_disk != value {
            stale.push(name.clone());
        }
    }
    assert!(
        stale.is_empty(),
        "stale view fixtures {stale:?}: run with QT_UPDATE_FIXTURES=1 and review the diff"
    );
    // No other JSON file sits beside them (`toast.json` left with the toasts).
    let mut extra: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .map(|p| p.file_stem().unwrap().to_string_lossy().into_owned())
        .filter(|n| !values.iter().any(|(v, _)| v == n))
        .collect();
    extra.sort();
    assert!(extra.is_empty(), "view fixtures no frame id names: {extra:?}");

    // The campaign fixture shows one row in each status (the brief's ask).
    let campaign = &values[0].1;
    let mut statuses: Vec<String> = campaign["acts"][0]["quests"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["status"].as_str().unwrap().to_string())
        .collect();
    statuses.sort();
    let mut all: Vec<String> = Status::NAMES.iter().map(|s| s.to_string()).collect();
    all.sort();
    assert_eq!(statuses, all);

    // The "probably done" mark is in the fixtures the UI reads: an inferred
    // done row in the campaign, an inferred step in the detail, and the
    // field on every row and step (false elsewhere).
    let campaign_rows = campaign["acts"][0]["quests"].as_array().unwrap();
    assert!(
        campaign_rows
            .iter()
            .any(|r| r["inferred"] == true && r["status"] == "done"),
        "panel-campaign has no inferred done row"
    );
    let detail = &values
        .iter()
        .find(|(n, _)| n == "quest-detail")
        .expect("quest-detail")
        .1;
    let steps = detail["detail"]["steps"].as_array().unwrap();
    assert!(
        steps.iter().any(|s| s["inferred"] == true && s["state"] == "done"),
        "quest-detail has no inferred step: {steps:#?}"
    );
    for (name, view) in &values {
        for row in rows_of(view) {
            assert!(row["inferred"].is_boolean(), "{name}: {row}");
            assert!(row.as_object().unwrap().contains_key("current_tip"), "{name}: {row}");
        }
        if !view["detail"].is_null() {
            for s in view["detail"]["steps"].as_array().unwrap() {
                assert!(s["inferred"].is_boolean(), "{name}: {s}");
                assert!(s.as_object().unwrap().contains_key("tip"), "{name}: {s}");
            }
        }
    }

    // A find-it tip is in the preview: under a row's next step in the default
    // frame, and under its step in the detail.
    let next = &values.iter().find(|(n, _)| n == "panel-next-up").unwrap().1;
    assert!(
        rows_of(next).iter().any(|r| r["current_tip"].is_string()),
        "panel-next-up has no row with a tip"
    );
    assert!(steps.iter().any(|s| s["tip"].is_string()), "quest-detail has no step with a tip");
}

#[test]
fn frames_and_icons_are_the_brief_ids() {
    let frames = brief_list("Frame ids:");
    for f in Frame::ALL {
        assert!(frames.contains(&f.as_str().to_string()), "{f:?}");
        assert_eq!(
            serde_json::to_value(f).unwrap(),
            Value::String(f.as_str().to_string())
        );
    }
    let icons = brief_list("Icon ids:");
    let data = data();
    for q in &data.quests {
        for r in &q.rewards {
            assert!(icons.contains(&reward_icon(r).to_string()), "{r:?}");
        }
    }
    // The `toast` frame left the contract with the toasts.
    assert!(!frames.contains(&"toast".to_string()), "{frames:?}");
    for kind in [RowChangeKind::Available, RowChangeKind::StepDone, RowChangeKind::QuestDone] {
        assert_eq!(
            serde_json::to_value(kind).unwrap(),
            Value::String(kind.as_str().to_string())
        );
    }
    // Every reward category and element maps to an icon of the brief.
    for name in RewardCategory::NAMES {
        let category: RewardCategory = serde_json::from_value(Value::String(name.into())).unwrap();
        for element in [None, Some("fire"), Some("cold"), Some("lightning"), Some("chaos"), Some("all")] {
            let reward = qt_engine::Reward {
                category,
                text: String::new(),
                element: element.map(|e| serde_json::from_value(Value::String(e.into())).unwrap()),
            };
            assert!(icons.contains(&reward_icon(&reward).to_string()), "{reward:?}");
        }
    }
    // Step phases serialise as the contract's words.
    assert_eq!(
        serde_json::to_value(StepPhase::Current).unwrap(),
        Value::String("current".into())
    );
}
