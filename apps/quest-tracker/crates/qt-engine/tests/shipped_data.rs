//! The shipped quest data, `data/quests.json`, against the contract and the
//! rules the curation follows (Context "Quest data contract" and "Building
//! the quest data"). The validator in `qt_engine::data` checks the shape;
//! these tests check what the shape cannot: that the campaign is all there,
//! that every quest says where it came from, that a quest claiming a measured
//! signal names an area the owner's log actually showed, and that worth and
//! detection follow from the rewards and the signals.
//!
//! Baseline: the empty file the scaffold wrote validates but fails
//! `campaign_acts_present`. Target: the curated file passes all five.
//!
//! The last two pin the talk steps of Act 1 (research note "Town dialogue"):
//! the log records nothing of a talk in Clearfell Encampment, so a talk step
//! is either completed by a later signal the log records, labelled
//! `inferred`, or left to the player's tick.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::Arc;

use qt_engine::{
    Detection, Engine, Evidence, Quest, QuestData, RewardCategory, Signal, Status, Worth,
};

/// The campaign acts as poe2db numbers them (its Act 6 is the interludes).
const CAMPAIGN_ACTS: std::ops::RangeInclusive<u32> = 1..=6;

/// The one campaign act the owner's log never showed: poe2db's Act 5 is
/// newer than the log's last gameplay, so none of its signals is measured.
/// The interludes (poe2db's Act 6, the log's `P1_*`..`P3_*` codes) were.
const UNMEASURED_ACT: u32 = 5;

/// Campaign quest pages poe2db listed for Acts 1 to 6 on the fetch date
/// (74 on 2026-10-25); each is one entry with its own page as source.
const MIN_POE2DB_PAGES: usize = 74;

/// The poe2db index page, the source of the permanent rewards that its
/// community table lists without a quest page of their own.
const INDEX_URL: &str = "https://poe2db.tw/us/Quest";

fn data_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/quests.json")
}

fn data() -> QuestData {
    let path = data_path();
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    QuestData::parse(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The area codes a signal names, through `code` or through `area`.
fn area_codes(signal: &Signal) -> Vec<&str> {
    match signal {
        Signal::Area { code, .. } => code.iter().map(String::as_str).collect(),
        Signal::Npc { area, .. }
        | Signal::System { area, .. }
        | Signal::Reward { area, .. }
        | Signal::Points { area, .. } => area.iter().map(String::as_str).collect(),
        Signal::Level { .. } | Signal::Quest { .. } => Vec::new(),
    }
}

/// A signal is measured when it names an area whose own evidence is
/// `measured`: the owner's log showed that area, and the curation took the
/// line or entry from what the scan counted there.
fn is_measured(data: &QuestData, signal: &Signal) -> bool {
    area_codes(signal)
        .iter()
        .any(|c| data.areas.get(*c).is_some_and(|a| a.evidence == Evidence::Measured))
}

fn any_measured(data: &QuestData, signals: &[Signal]) -> bool {
    signals.iter().any(|s| is_measured(data, s))
}

fn step_signals(quest: &Quest) -> impl Iterator<Item = &Signal> {
    quest.steps.iter().flat_map(|s| s.signals.iter())
}

#[test]
fn data_file_validates() {
    let data = data();
    assert!(!data.quests.is_empty(), "the shipped data holds no quest");
    assert!(!data.npcs.is_empty(), "the shipped data allowlists no speaker");
    assert!(!data.areas.is_empty(), "the shipped data names no area");
    assert_ne!(data.game_version, "unchecked", "game_version is not set");
    assert_eq!(data.source, "https://poe2db.tw/us/Quest");
    // Every area belongs to a campaign act and every speaker is used, so the
    // allowlist stays the curated one and not a dump of every logged name.
    for (code, area) in &data.areas {
        assert!(
            CAMPAIGN_ACTS.contains(&area.act),
            "area {code}: act {} is outside the campaign",
            area.act
        );
    }
    let mut used: BTreeSet<&str> = BTreeSet::new();
    for q in &data.quests {
        if let Some(g) = &q.giver {
            used.insert(g);
        }
        let all = q
            .unlock
            .iter()
            .chain(&q.done)
            .chain(&q.window_closes)
            .chain(step_signals(q));
        for s in all {
            if let Signal::Npc { speaker, .. } = s {
                used.insert(speaker);
            }
        }
    }
    for npc in &data.npcs {
        assert!(used.contains(npc.as_str()), "npc {npc:?} is allowlisted but no quest uses it");
    }
}

#[test]
fn every_quest_has_source_url_and_fetch_date() {
    let data = data();
    let mut pages = BTreeSet::new();
    for q in &data.quests {
        if q.source_url != INDEX_URL {
            assert!(pages.insert(q.source_url.as_str()), "two quests share {}", q.source_url);
        } else {
            assert!(
                q.notes.contains("community table"),
                "quest {}: sourced from the index page without saying why",
                q.id
            );
        }
        assert!(
            q.source_url.starts_with("https://poe2db.tw/us/"),
            "quest {}: source_url {:?}",
            q.id,
            q.source_url
        );
        assert!(
            qt_engine::data::is_quest_url(&q.source_url),
            "quest {}: source_url {:?} is not one poe2db page",
            q.id,
            q.source_url
        );
        assert!(qt_engine::data::is_date(&q.fetched), "quest {}: fetched {:?}", q.id, q.fetched);
        // ISO dates compare as strings: no quest was fetched after the file's
        // own fetch date.
        assert!(
            q.fetched <= data.poe2db_fetched,
            "quest {}: fetched {} is after poe2db_fetched {}",
            q.id,
            q.fetched,
            data.poe2db_fetched
        );
    }
    assert!(
        pages.len() >= MIN_POE2DB_PAGES,
        "only {} quests have their own poe2db page; the campaign has {MIN_POE2DB_PAGES}",
        pages.len()
    );
}

#[test]
fn campaign_acts_present() {
    let data = data();
    let mut per_act: BTreeMap<u32, usize> = BTreeMap::new();
    for q in &data.quests {
        assert!(
            CAMPAIGN_ACTS.contains(&q.act),
            "quest {}: act {} is outside the campaign (endgame is out of scope)",
            q.id,
            q.act
        );
        *per_act.entry(q.act).or_default() += 1;
    }
    for act in CAMPAIGN_ACTS {
        assert!(per_act.contains_key(&act), "no quest for act {act}: {per_act:?}");
    }
    // Representative quests: from the log's reward table, one or two per
    // measured act, finished automatically by the reward or points line
    // counted there; and from Act 5, which the log never showed, the main
    // quest, ticked by hand.
    for (id, act, detection) in [
        ("ominous-altars", 1, Detection::Auto),
        ("the-lost-lute", 1, Detection::Auto),
        ("traditions-toll", 2, Detection::Auto),
        ("tribal-vengeance", 3, Detection::Auto),
        ("trial-of-the-ancestors", 4, Detection::Auto),
        ("lythara-the-wayward-spear", 6, Detection::Auto),
        ("the-siege-of-oriath", 5, Detection::Manual),
    ] {
        let q = data.quest(id).unwrap_or_else(|| panic!("no quest {id}"));
        assert_eq!(q.act, act, "quest {id}");
        assert_eq!(q.detection, detection, "quest {id}");
    }
}

#[test]
fn measured_signals_use_measured_areas() {
    let data = data();
    // A measured area has a code-shaped key and an act the log showed it in.
    let mut measured = 0;
    for (code, area) in &data.areas {
        if area.evidence == Evidence::Measured {
            measured += 1;
            assert!(
                code.starts_with('G') || code.starts_with('P'),
                "area {code}: a measured area is keyed by its log code"
            );
        }
    }
    assert!(measured >= 60, "only {measured} measured areas");
    for q in &data.quests {
        // `detection` follows from where the measured signals are
        // (Context "Quest state rules").
        let done = any_measured(&data, &q.done);
        let other = any_measured(&data, &q.unlock) || step_signals(q).any(|s| is_measured(&data, s));
        let want = if done {
            Detection::Auto
        } else if other {
            Detection::Partial
        } else {
            Detection::Manual
        };
        assert_eq!(q.detection, want, "quest {}: detection", q.id);
        // A quest labelled measured has its completion measured; one that is
        // not, does not claim it.
        assert_eq!(
            q.evidence == Evidence::Measured,
            done,
            "quest {}: evidence {:?} but measured done signal = {done}",
            q.id,
            q.evidence
        );
        // Every signal that names an area code names one the file declares
        // (the validator checks this too; here it is pinned for the shipped
        // file), and an Act 5 quest, which the log never showed, uses no
        // measured area at all.
        let all: Vec<&Signal> = q
            .unlock
            .iter()
            .chain(&q.done)
            .chain(&q.window_closes)
            .chain(step_signals(q))
            .collect();
        for s in &all {
            for c in area_codes(s) {
                assert!(data.areas.contains_key(c), "quest {}: area {c}", q.id);
            }
        }
        if q.act == UNMEASURED_ACT {
            assert!(
                all.iter().filter(|s| !matches!(s, Signal::Quest { .. })).all(|s| !is_measured(&data, s)),
                "quest {}: act {} claims a measured signal",
                q.id,
                q.act
            );
        }
    }
}

#[test]
fn every_quest_has_worth_and_detection() {
    let data = data();
    let mut ids = BTreeSet::new();
    for q in &data.quests {
        assert!(ids.insert(q.id.as_str()), "duplicate id {}", q.id);
        assert!(!q.worth_reason.trim().is_empty(), "quest {}: no worth reason", q.id);
        assert!(!q.rewards.is_empty(), "quest {}: no reward listed", q.id);
        let permanent = q
            .rewards
            .iter()
            .any(|r| r.category.is_permanent_stat() || r.category == RewardCategory::Ascendancy);
        if permanent {
            assert_eq!(q.worth, Worth::Yes, "quest {}: a permanent reward is worth yes", q.id);
        }
        if q.worth == Worth::No {
            assert!(
                q.rewards
                    .iter()
                    .all(|r| matches!(r.category, RewardCategory::Story | RewardCategory::Item)),
                "quest {}: worth no with a reward that lasts",
                q.id
            );
        }
        // The manual-tick marker shows for partial and manual quests, so the
        // player can see which ones the log cannot finish; a quest the log
        // cannot finish names when its window closes or explains why not.
        if q.detection != Detection::Auto {
            assert!(
                !q.window_closes.is_empty() || !q.notes.trim().is_empty(),
                "quest {}: {:?} without window_closes or notes",
                q.id,
                q.detection
            );
        }
    }
}

/// The invented character of `fixtures/sessions/town_talk.txt`.
const TOWN_TALK_CHARACTER: &str = "Maelis_Thornby";

/// The town voices of Clearfell Encampment: what the log records of them is
/// greetings and vendor lines, never a quest's talk (research note "Town
/// dialogue"), so none of them is a speaker the data matches.
const TOWN_VOICES: [&str; 4] = ["Una", "Finn", "Renly", "Mortimer"];

/// A session's lines, split the way the tailer hands them over.
fn session(name: &str) -> Vec<String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/sessions")
        .join(name);
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let mut lines: Vec<String> = text.split('\n').map(str::to_string).collect();
    if lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    lines
}

fn entry_of(lines: &[String], code: &str) -> usize {
    let needle = format!("area \"{code}\"");
    lines
        .iter()
        .position(|l| l.contains(&needle))
        .unwrap_or_else(|| panic!("the session never enters {code}"))
}

fn status(engine: &Engine, quest: &str) -> Status {
    engine
        .status(TOWN_TALK_CHARACTER, quest)
        .unwrap_or_else(|| panic!("no state for quest {quest}"))
}

/// The live session's start, as the log recorded it (Live 1, 2026-10-25):
/// The Riverbank, The Bloated Miller, the level-up to 2, a visit to
/// Clearfell Encampment in which the person talked to the NPCs and the log
/// recorded no NPC line, then Clearfell. Fed through the engine over the
/// shipped data.
#[test]
fn town_talk_session_replay() {
    let data = Arc::new(data());
    let lines = session("town_talk.txt");
    let town = entry_of(&lines, "G1_town");
    let clearfell = entry_of(&lines, "G1_2");
    let mut engine = Engine::new(Arc::clone(&data));
    for l in &lines[..=town] {
        engine.feed_line(l, false);
    }
    assert_eq!(engine.active(), Some(TOWN_TALK_CHARACTER));

    // Baseline: entering the encampment reaches "Enter town", and the step to
    // do now is the talk with the Blacksmith, which no signal can show.
    let rc = data.quest("reaching-clearfell").expect("reaching-clearfell");
    let state = engine
        .quest_state(TOWN_TALK_CHARACTER, "reaching-clearfell")
        .expect("state");
    assert_eq!(state.status, Status::InProgress);
    assert_eq!(state.current_step.as_deref(), Some("step-3"));
    let reached = rc.steps.iter().position(|s| s.id == "step-3").expect("step-3");
    let talk = &rc.steps[reached + 1];
    assert_eq!(talk.title, "Talk to the Blacksmith");
    assert!(talk.signals.is_empty(), "the talk step has a signal: {:?}", talk.signals);
    assert_eq!(status(&engine, "secrets-in-the-dark"), Status::Locked);
    assert_eq!(status(&engine, "treacherous-ground"), Status::Locked);
    assert_eq!(status(&engine, "finding-the-forge"), Status::Available);

    // The visit itself: whatever the person said to whom, the lines up to
    // leaving the encampment change no quest.
    let before = engine.quest_states(TOWN_TALK_CHARACTER);
    for l in &lines[town + 1..clearfell] {
        assert!(engine.feed_line(l, false).is_empty(), "{l}");
    }
    assert_eq!(engine.quest_states(TOWN_TALK_CHARACTER), before);

    // Target: entering Clearfell completes Reaching Clearfell, whose talk
    // steps it implies (inferred), and moves the quests that follow it.
    for l in &lines[clearfell..] {
        engine.feed_line(l, false);
    }
    assert_eq!(status(&engine, "reaching-clearfell"), Status::Done);
    assert_eq!(status(&engine, "secrets-in-the-dark"), Status::Available);
    let tg = engine
        .quest_state(TOWN_TALK_CHARACTER, "treacherous-ground")
        .expect("state");
    assert_eq!(tg.status, Status::InProgress);
    assert_eq!(tg.current_step.as_deref(), Some("step-1"));
    // A talk step nothing later implies stays the player's tick: Finding the
    // Forge waits on "Speak to Renly", and its row carries the tick marker.
    let forge = engine
        .quest_state(TOWN_TALK_CHARACTER, "finding-the-forge")
        .expect("state");
    assert_eq!(forge.status, Status::Available);
    assert_eq!(forge.current_step, None);
    assert!(data.quest("finding-the-forge").expect("forge").detection.needs_tick());
}

/// In Act 1, a step with no signal (a talk, mostly) either belongs to a quest
/// the player finishes by hand, or is implied by a later step or a done
/// signal the log records. A done that is only an area entry stands for the
/// talk before it, and the quest's notes say `inferred`.
#[test]
fn talk_steps_are_inferred_or_ticks() {
    let data = data();
    for voice in TOWN_VOICES {
        assert!(
            !data.npcs.iter().any(|n| n == voice),
            "{voice}'s greetings are on the speaker allowlist"
        );
    }
    for q in data.quests.iter().filter(|q| q.act == 1) {
        let done_logged = any_measured(&data, &q.done);
        for (k, step) in q.steps.iter().enumerate() {
            if !step.signals.is_empty() {
                continue;
            }
            let later_logged = q.steps[k + 1..].iter().any(|s| any_measured(&data, &s.signals));
            assert!(
                q.detection.needs_tick() || later_logged || done_logged,
                "quest {}: step {} ({:?}) has no signal, nothing later the log records implies it, and the quest is {:?}",
                q.id,
                step.id,
                step.title,
                q.detection
            );
        }
        let area_only = !q.done.is_empty() && q.done.iter().all(|s| matches!(s, Signal::Area { .. }));
        if q.detection == Detection::Auto && area_only {
            assert!(
                q.notes.contains("inferred"),
                "quest {}: done is an area entry the notes do not label inferred",
                q.id
            );
        }
    }
    // The two talk endings the town dialogue measurement let the data infer.
    for (id, code) in [("reaching-clearfell", "G1_2"), ("secrets-in-the-dark", "G1_7")] {
        let q = data.quest(id).unwrap_or_else(|| panic!("no quest {id}"));
        assert!(
            q.done.iter().any(|s| area_codes(s).contains(&code)),
            "quest {id}: done is not the entry to {code}: {:?}",
            q.done
        );
        assert_eq!(q.detection, Detection::Auto, "quest {id}");
    }
}
