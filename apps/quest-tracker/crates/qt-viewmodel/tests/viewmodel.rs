//! Baseline and target for the view-model, over the engine's synthetic quest
//! data (`fixtures/quests_min.json`) and sessions (`fixtures/sessions/`).
//! Baseline: an engine that has seen nothing shows an empty state, never a
//! panel. Target: after `act1.txt` the panel lists the act the character is
//! in, its progress and its rows, filtered by the settings, with toasts only
//! for live transitions. `fixtures_match_serialised_views` pins one view per
//! frame id (and one toast) to `fixtures/viewmodels/<frame id>.json`, which
//! the UI tests and the browser preview read, all built from one story
//! (`fixtures/sessions/frames/story.txt`); `QT_UPDATE_FIXTURES=1` rewrites
//! them, and two more tests hold the set consistent and varied. All
//! character names are invented.

use std::path::PathBuf;
use std::sync::Arc;

use qt_engine::{Engine, QuestAction, QuestData, RewardCategory, Status, Transition, Worth};
use qt_viewmodel::{
    build_view, reward_icon, Frame, LogState, Mode, Settings, StatusKind, StepPhase, Toast,
    ToastKind, ToastQueue, UiState, View, ViewKind, MAX_TOASTS, TOAST_MS,
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
    // Next up never lists a skipped quest, whatever the setting.
    assert!(!next_ids(&view).contains(&"fixture-crowbell".to_string()));
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
fn toasts_only_for_live_transitions_max_three() {
    let data = data();
    // Catch-up (the lines were already in the log): no toast at all.
    let mut engine = Engine::new(data.clone());
    let catch_up = feed(&mut engine, &session("act1.txt"), false);
    assert!(!catch_up.is_empty());
    let mut queue = ToastQueue::new();
    assert!(queue.push(&data, &catch_up, 0).is_empty());
    assert!(queue.visible(0).is_empty());

    // One live line: the riverbank is done and that unlocks the king.
    let lines = session("act1.txt");
    let renly = lines
        .iter()
        .position(|l| l.contains("Renly: "))
        .expect("act1 has Renly's line");
    let mut engine = Engine::new(data.clone());
    feed(&mut engine, &lines[..renly], false);
    let live = engine.feed_line(&lines[renly], true);
    let mut queue = ToastQueue::new();
    let shown = queue.push(&data, &live, 1_000);
    let kinds: Vec<(ToastKind, &str, &str)> = shown
        .iter()
        .map(|t| (t.kind, t.quest_id.as_str(), t.icon.as_str()))
        .collect();
    assert_eq!(
        kinds,
        vec![
            (ToastKind::QuestDone, "fixture-riverbank", "status-done"),
            (ToastKind::Available, "fixture-king", "status-available"),
        ],
        "{live:#?}"
    );
    assert_eq!(shown[0].id + 1, shown[1].id);
    assert!(shown[1].text.contains("Fixture: The King in the Mists"));

    // A whole act live in one batch: at most three, the newest kept.
    let mut engine = Engine::new(data.clone());
    let burst = feed(&mut engine, &lines, true);
    let mut queue = ToastQueue::new();
    let shown = queue.push(&data, &burst, 5_000);
    assert_eq!(shown.len(), MAX_TOASTS);
    assert_eq!(queue.visible(5_000), shown);
    let last: Vec<String> = shown.iter().map(|t| t.quest_id.clone()).collect();
    assert_eq!(last.last().map(String::as_str), Some("fixture-king"));
    // They go after about three seconds.
    assert_eq!(queue.visible(5_000 + TOAST_MS - 1).len(), MAX_TOASTS);
    assert!(queue.visible(5_000 + TOAST_MS).is_empty());

    // Line by line, the stack never grows past three.
    let mut engine = Engine::new(data.clone());
    let mut queue = ToastQueue::new();
    let mut seen: Vec<Toast> = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let t = engine.feed_line(line, true);
        seen.extend(queue.push(&data, &t, i as u64 * 10));
        assert!(queue.visible(i as u64 * 10).len() <= MAX_TOASTS);
    }
    assert!(seen.len() > MAX_TOASTS, "{seen:#?}");
    assert!(seen.iter().any(|t| t.kind == ToastKind::StepDone));
    let mut ids: Vec<u64> = seen.iter().map(|t| t.id).collect();
    ids.dedup();
    assert_eq!(ids.len(), seen.len(), "toast ids are unique");
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

    // A session with no character drops its events and says so.
    // (Its level-up alone leaves Odile unconfirmed, a warning that would come
    // first; confirmed, the dropped count is what is left to say.)
    let mut held = replay("held.txt");
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
    let lines = session("act1.txt");
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
        opacity: 0.05,
        ..Settings::default()
    };
    let view = build_view(&replay("act1.txt"), &wild, &ui);
    assert_eq!(view.frame, Frame::Collapsed);
    assert_eq!(view.settings.opacity, Settings::MIN_OPACITY);
    assert!(Settings::default()
        .patched(&serde_json::json!({"no_such_setting": 1}))
        .is_err());
    let patched = Settings::default()
        .patched(&serde_json::json!({"dock": "left", "scale": "1.33"}))
        .unwrap();
    assert_eq!(
        serde_json::to_value(&patched).unwrap()["scale"],
        Value::String("1.33".into())
    );
}

// ---------------------------------------------------------------------------
// The fixtures the UI tests read.

/// The data the frames are built on: the fixture data with two quests' worth
/// (and its reason) changed on this copy, so the rows carry every worth.
/// `fixtures/quests_min.json` itself stays as the engine and replay tests
/// read it.
fn frames_data() -> Arc<QuestData> {
    let mut v = data_value();
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

/// One serialised value per frame id of the brief, plus `toast`. Every frame
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

    // The toast: the story's last line (Una's, which makes the lute
    // available) read live after a catch-up of the rest.
    let data = frames_data();
    let lines = story_lines();
    let (last, before) = lines.split_last().unwrap();
    let mut engine = Engine::new(data.clone());
    feed(&mut engine, before, false);
    let live = engine.feed_line(last, true);
    let toast = ToastQueue::new()
        .push(&data, &live, 0)
        .into_iter()
        .next()
        .expect("Una's line read live makes the lute available");
    out.push(("toast".into(), serde_json::to_value(&toast).unwrap()));
    out
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
    let views: Vec<&(String, Value)> = files.iter().filter(|(n, _)| n != "toast").collect();

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

    // The toast announces what the frames show.
    let toast = &files.iter().find(|(n, _)| n == "toast").expect("toast.json").1;
    let shown_as = match toast["kind"].as_str().unwrap() {
        "available" => "available",
        "step_done" => "in_progress",
        "quest_done" => "done",
        other => panic!("toast kind {other}"),
    };
    let quest = text(&toast["quest_id"]);
    let states = quests.get(&quest).unwrap_or_else(|| panic!("no frame shows {quest}"));
    assert!(
        states.iter().all(|s| s.starts_with(&format!("{shown_as} "))),
        "the toast says {quest} is {shown_as}; the frames show {states:?}"
    );
}

#[test]
fn fixture_rows_cover_every_worth() {
    let files = generated_fixtures();
    let mut worth = std::collections::BTreeSet::new();
    let mut detection = std::collections::BTreeSet::new();
    let mut ticks = std::collections::BTreeSet::new();
    for (name, view) in &files {
        if name == "toast" {
            continue;
        }
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
        if name != "toast" {
            assert_eq!(value["frame"], Value::String(name.clone()), "{name}");
            assert_eq!(value["schema"], Value::from(1));
        }
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
    for kind in [ToastKind::Available, ToastKind::StepDone, ToastKind::QuestDone] {
        assert!(icons.contains(&kind.icon().to_string()), "{kind:?}");
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
