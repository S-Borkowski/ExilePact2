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
//!
//! `boss_reward_shows_use_step_after_fight` pins the boss quests whose reward
//! line comes some seconds after the fight, not with it (research note
//! "Rewards after a boss kill"): the boss's lines are a "Find ..." step, the
//! "Defeat ..." step after it has no signal, a "Use ..." step follows, and
//! the reward line still finishes the quest. Only Beira's step says the boss
//! drops an item; for the others that is not established.
//!
//! The rest pin what launch 6 found the tests above could not catch: the
//! town voices' own lines, which `town_talk.txt` never had
//! (`town_voice_lines_move_no_quest`), the replay that finishes Secrets in the
//! Dark on the Cemetery entry (`secrets_in_the_dark_done_on_g1_7`), and the
//! `done_inferred` flag against the notes (`done_inferred_matches_notes`).
//!
//! Four more pin what Live 2 found (2026-10-25): Secrets in the Dark starts
//! when Reaching Clearfell is done, Una's Grelwood lines move it past the
//! tree, the Grelwood entry no longer starts The Mysterious Shade before its
//! unlock, and Una's Grim Tangle line moves the Shade past the roots.
//!
//! The last four pin what Live 3 and its owner found (2026-10-25): the
//! owner's find-it tips sit on the steps they help with
//! (`owner_tips_are_on_their_steps`), Treacherous Ground shows from the town
//! where Renly gives it (`treacherous_ground_available_in_town`), Beira's
//! defeat is the player's tick, not her first line
//! (`beira_defeat_is_ticked_by_hand`), and summoning Una leaves "Ask Una for
//! help" to do (`una_summon_alone_leaves_ask_una_for_help`).

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::Arc;

use qt_engine::{
    Detection, Engine, Evidence, Quest, QuestData, QuestState, RewardCategory, Signal, Status,
    Transition, Worth,
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

/// The town voices of Clearfell Encampment: what the log records of them in
/// the encampment is greetings and vendor lines, never a quest's talk
/// (research note "Town dialogue"), so none of their town lines is a signal.
/// Una alone is on the allowlist, for her lines in the Grelwood and the Grim
/// Tangle (Live 2, 2026-10-25); each of her signals names its area and its
/// exact line ([`TOWN_SPOKEN`]).
const TOWN_VOICES: [&str; 4] = ["Una", "Finn", "Renly", "Mortimer"];

/// The town voice the shipped allowlist reads, and the encampment's code.
const TOWN_SPOKEN: &str = "Una";
const TOWN_CODE: &str = "G1_town";

/// Every `npc` signal of `speaker` in the data, wherever a quest holds it, as
/// its area and exact line.
fn npc_signals<'a>(data: &'a QuestData, speaker: &str) -> Vec<(Option<&'a str>, Option<&'a str>)> {
    let mut out = Vec::new();
    for q in &data.quests {
        let all = q
            .unlock
            .iter()
            .chain(&q.start)
            .chain(&q.done)
            .chain(&q.window_closes)
            .chain(step_signals(q));
        for s in all {
            if let Signal::Npc { speaker: who, area, line } = s {
                if who == speaker {
                    out.push((area.as_deref(), line.as_deref()));
                }
            }
        }
    }
    out
}

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
    // Renly gives Treacherous Ground and Finding the Forge in the encampment,
    // in talks the log does not record: the town entry makes both available
    // (inferred, Live 3).
    assert_eq!(status(&engine, "treacherous-ground"), Status::Available);
    assert_eq!(status(&engine, "finding-the-forge"), Status::Available);

    // The visit itself. The Live 1 visit logged no NPC line at all, only chat
    // and client lines (left out of this fixture), so this loop sees no town
    // voice and cannot fail on one: `town_voice_lines_move_no_quest` replays
    // the voices' own lines. What it pins is that the visit's entry and scene
    // lines move no quest before Clearfell.
    let before = engine.quest_states(TOWN_TALK_CHARACTER);
    for l in &lines[town + 1..clearfell] {
        assert!(engine.feed_line(l, false).is_empty(), "{l}");
    }
    assert_eq!(engine.quest_states(TOWN_TALK_CHARACTER), before);

    // Target: entering Clearfell completes Reaching Clearfell, whose talk
    // steps it implies (inferred), and moves the quests that follow it.
    // Secrets in the Dark starts with it (its `start`, inferred: Renly gives
    // it in the same unlogged talk), no step reached yet.
    for l in &lines[clearfell..] {
        engine.feed_line(l, false);
    }
    assert_eq!(status(&engine, "reaching-clearfell"), Status::Done);
    let secrets = engine
        .quest_state(TOWN_TALK_CHARACTER, "secrets-in-the-dark")
        .expect("state");
    assert_eq!(secrets.status, Status::InProgress);
    assert_eq!(secrets.current_step, None);
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
        let listed = data.npcs.iter().any(|n| n == voice);
        if voice != TOWN_SPOKEN {
            assert!(!listed, "{voice}'s greetings are on the speaker allowlist");
            continue;
        }
        // Una is listed, and no signal of hers can match a line in town: each
        // names an area outside the encampment and an exact line.
        assert!(listed, "{voice} is not on the speaker allowlist");
        let signals = npc_signals(&data, voice);
        assert!(!signals.is_empty(), "{voice} is listed but no signal uses her");
        for (area, line) in signals {
            assert!(
                area.is_some_and(|a| a != TOWN_CODE) && line.is_some(),
                "{voice}'s signal {area:?} {line:?} does not name an area outside town and a line"
            );
        }
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

/// The boss quests whose steps end at the fight and whose `done` is the
/// reward line, which the log records 5 s to 8 min after the boss's last
/// line (measured). For Beira that later action was using the item she
/// dropped (Live 1, by the owner's observation); for the other two it is an
/// inference from the timing.
///
/// Beira is first: the only one whose "Use ..." step may say the boss drops
/// an item.
const USE_ITEM_QUESTS: [&str; 3] = [
    "beira-of-the-rotten-pack",
    "blackjaw-the-remnant",
    "lythara-the-wayward-spear",
];

/// The invented character of `fixtures/sessions/act1.txt`.
const ACT1_CHARACTER: &str = "Corisande_Fell";

#[test]
fn boss_reward_shows_use_step_after_fight() {
    let data = Arc::new(data());
    // The shape: a "Find ..." step the boss's line moves, a "Defeat ..."
    // step no line marks (the player ticks it), then a "Use ..." step no
    // signal can show, and the reward line as `done`.
    for id in USE_ITEM_QUESTS {
        let q = data.quest(id).unwrap_or_else(|| panic!("no quest {id}"));
        let n = q.steps.len();
        assert!(n >= 3, "quest {id}: {n} steps");
        let (find, fight, last) = (&q.steps[n - 3], &q.steps[n - 2], &q.steps[n - 1]);
        assert_eq!((find.id.as_str(), fight.id.as_str(), last.id.as_str()), ("find", "fight", "use"), "quest {id}");
        assert!(
            find.signals.iter().any(|s| matches!(s, Signal::Npc { .. })),
            "quest {id}: step {} is not the boss's line: {:?}",
            find.id,
            find.signals
        );
        assert!(find.title.starts_with("Find "), "quest {id}: {:?}", find.title);
        assert!(fight.title.starts_with("Defeat "), "quest {id}: {:?}", fight.title);
        assert!(fight.signals.is_empty(), "quest {id}: the defeat step has a signal: {:?}", fight.signals);
        assert!(last.title.starts_with("Use "), "quest {id}: last step {:?}", last.title);
        assert!(last.signals.is_empty(), "quest {id}: the use step has a signal: {:?}", last.signals);
        assert!(
            !q.done.is_empty() && q.done.iter().all(|s| matches!(s, Signal::Reward { .. })),
            "quest {id}: done is not the reward line: {:?}",
            q.done
        );
        assert!(q.notes.contains("dropped"), "quest {id}: the notes do not say why the step exists");
        assert_eq!(q.detection, Detection::Auto, "quest {id}");
    }
    // Only Beira's drop was observed: the others' step says it is not
    // established, and neither its title nor its text states a drop.
    let beira_use = data.quest(USE_ITEM_QUESTS[0]).and_then(|q| q.steps.last()).expect("beira");
    assert!(beira_use.text.contains("leaves an item behind"), "{:?}", beira_use.text);
    for id in &USE_ITEM_QUESTS[1..] {
        let last = data.quest(id).and_then(|q| q.steps.last()).expect("use step");
        for claim in ["leaves an item behind", "is granted then", "item it dropped"] {
            assert!(
                !last.title.contains(claim) && !last.text.contains(claim),
                "quest {id}: the use step states a drop ({claim:?}): {:?} / {:?}",
                last.title,
                last.text
            );
        }
        assert!(last.text.contains("not established"), "quest {id}: {:?}", last.text);
    }

    // Beira in the act 1 fixture session, fed through the engine over the
    // shipped data.
    let lines = session("act1.txt");
    let beira = lines
        .iter()
        .position(|l| l.contains("] Beira of the Rotten Pack: "))
        .expect("the session has Beira's line");
    let reward = lines
        .iter()
        .position(|l| l.contains("has received +10% to [Resistances|Cold Resistance]"))
        .expect("the session has the cold resistance reward");
    assert!(entry_of(&lines, "G1_2") < beira && beira < reward);
    let mut engine = Engine::new(Arc::clone(&data));
    for l in &lines[..=beira] {
        engine.feed_line(l, false);
    }
    assert_eq!(engine.active(), Some(ACT1_CHARACTER));

    // Baseline: the boss's line puts the quest in progress at "Find ...", and
    // the step the row shows next is her defeat, never the use of the item
    // (Live 2 and Live 3 showed "Use ..." at her first line).
    let state = engine
        .quest_state(ACT1_CHARACTER, "beira-of-the-rotten-pack")
        .expect("state");
    assert_eq!(state.status, Status::InProgress);
    assert_eq!(state.current_step.as_deref(), Some("find"));
    assert_eq!(next_title(&data, &state), "Defeat Beira of the Rotten Pack");

    // The lines between the fight and the reward leave it there.
    for l in &lines[beira + 1..reward] {
        engine.feed_line(l, false);
    }
    let state = engine
        .quest_state(ACT1_CHARACTER, "beira-of-the-rotten-pack")
        .expect("state");
    assert_eq!(state.status, Status::InProgress);
    assert_eq!(next_title(&data, &state), "Defeat Beira of the Rotten Pack");

    // Target: the reward line, which comes once the item is used, finishes it.
    engine.feed_line(&lines[reward], false);
    assert_eq!(engine.status(ACT1_CHARACTER, "beira-of-the-rotten-pack"), Some(Status::Done));

    // The other two, built line by line: the boss's first line shows
    // "Defeat ..." next, not "Use ...", and the reward line finishes it.
    for (id, code, name, speaker, reward) in [
        (
            "blackjaw-the-remnant",
            "G3_6_1",
            "Jiquani's Machinarium",
            "BlackJaw",
            "+10% to [Resistances|Fire Resistance]",
        ),
        (
            "lythara-the-wayward-spear",
            "P3_2",
            "Kriar Village",
            "Lythara, The Wayward Spear",
            "+40 to [Spirit|Spirit]",
        ),
    ] {
        let mut r = Replay::start(&data, GRELWOOD_CHARACTER);
        let lines = entry_lines(&mut r.clock, code, 40, 2_449_489_742, name);
        r.feed(&lines);
        assert_eq!(r.state(id).current_step.as_deref(), Some("enter"), "quest {id}");
        r.say(speaker, "Fixture line: the fight begins.");
        let s = r.state(id);
        assert_eq!(s.current_step.as_deref(), Some("find"), "quest {id}");
        assert!(next_title(&data, &s).starts_with("Defeat "), "quest {id}: {:?}", next_title(&data, &s));
        r.received(reward);
        assert_eq!(r.state(id).status, Status::Done, "quest {id}");
    }
}

/// The invented character of `fixtures/sessions/town_voices.txt`.
const TOWN_VOICES_CHARACTER: &str = "Odile_Brackwater";

/// The town voices `town_voices.txt` quotes: the vendor and gambling lines
/// the owner's log recorded most often from them in Clearfell Encampment
/// (research note "Town dialogue").
const QUOTED_VOICES: [&str; 3] = ["Una", "Finn", "Renly"];

/// A visit to Clearfell Encampment in which Renly, Finn and Una speak, in
/// the shapes the owner's log recorded. Finn and Renly are not on the shipped
/// allowlist, and Una's town lines match none of her signals, so across those
/// lines no quest moves.
#[test]
fn town_voice_lines_move_no_quest() {
    let data = Arc::new(data());
    let lines = session("town_voices.txt");
    let town = entry_of(&lines, "G1_town");
    let clearfell = entry_of(&lines, "G1_2");
    let visit = &lines[town + 1..clearfell];

    // Positive control: with an allowlist that names the town voices, the
    // parser reads the visit's voice lines as NPC lines, each of the three
    // at least once, so the loop below does go over real voice lines.
    let voices = qt_logline::Allowlist::new(QUOTED_VOICES, Vec::<String>::new());
    let mut heard: BTreeMap<String, usize> = BTreeMap::new();
    for l in visit {
        if let Some(qt_logline::Event {
            kind: qt_logline::EventKind::NpcLine { speaker, .. },
            ..
        }) = qt_logline::parse_line(l, &voices)
        {
            *heard.entry(speaker).or_default() += 1;
        }
    }
    for voice in QUOTED_VOICES {
        assert!(heard.get(voice).is_some_and(|n| *n >= 1), "{voice} never speaks: {heard:?}");
    }
    assert_eq!(heard.values().sum::<usize>(), 6, "{heard:?}");

    // Negative control: the shipped allowlist reads Finn's and Renly's lines
    // as none. Una's town lines parse, since her Grelwood lines are signals
    // (Live 2), and match no signal: every one of hers names another area
    // and another exact line.
    let shipped = qt_logline::Allowlist::new(data.npcs.clone(), data.system_lines.clone());
    let una = npc_signals(&data, TOWN_SPOKEN);
    let mut una_lines = 0;
    for l in visit {
        let kind = qt_logline::parse_line(l, &shipped).map(|e| e.kind);
        let Some(qt_logline::EventKind::NpcLine { speaker, text }) = kind else {
            continue;
        };
        assert_eq!(speaker, TOWN_SPOKEN, "the shipped allowlist reads {l:?}");
        una_lines += 1;
        for (area, line) in &una {
            let here = area.is_none_or(|a| a == TOWN_CODE);
            let same = line.is_none_or(|x| x == text);
            assert!(!(here && same), "{l:?} matches {TOWN_SPOKEN}'s signal {area:?} {line:?}");
        }
    }
    assert_eq!(Some(&una_lines), heard.get(TOWN_SPOKEN), "{heard:?}");

    // Target: fed through the engine over the shipped data, the voices
    // change no quest state and give no transition.
    let mut engine = Engine::new(Arc::clone(&data));
    for l in &lines[..=town] {
        engine.feed_line(l, false);
    }
    assert_eq!(engine.active(), Some(TOWN_VOICES_CHARACTER));
    let before = engine.quest_states(TOWN_VOICES_CHARACTER);
    for l in visit {
        let moved = engine.feed_line(l, false);
        assert!(moved.is_empty(), "{l}: {moved:?}");
    }
    assert_eq!(engine.quest_states(TOWN_VOICES_CHARACTER), before);

    // And the session still moves quests where the log does record a signal:
    // entering Clearfell finishes Reaching Clearfell.
    for l in &lines[clearfell..] {
        engine.feed_line(l, false);
    }
    assert_eq!(
        engine.status(TOWN_VOICES_CHARACTER, "reaching-clearfell"),
        Some(Status::Done)
    );
}

/// An area entry in the shape the log writes it, for a session built in the
/// test (the client prints the level and seed with the code).
fn entry_lines(clock: &mut u32, code: &str, level: u32, seed: u64, name: &str) -> Vec<String> {
    *clock += 60;
    let t = stamp(*clock);
    vec![
        format!("{t} [INFO Client 6200] Connecting to instance server"),
        format!("{t} [DEBUG Client 6200] Generating level {level} area \"{code}\" with seed {seed}"),
        format!("{t} [INFO Client 6200] [SCENE] Set Source [(null)]"),
        format!("{t} [INFO Client 6200] [SCENE] Set Source [{name}]"),
    ]
}

fn stamp(seconds: u32) -> String {
    let (h, m, s) = (20 + seconds / 3600, seconds / 60 % 60, seconds % 60);
    format!("2026/03/22 {h:02}:{m:02}:{s:02} {} 8f3d{seconds:04x}", 5_000_000 + seconds * 1000)
}

/// The invented character of the Secrets in the Dark replay.
const SECRETS_CHARACTER: &str = "Ysolde_Marrow";

/// Secrets in the Dark is done on entering the Cemetery of the Eternals
/// (`G1_7`), inferred (its notes, `done_inferred`): an engine replay over the
/// shipped data of an Act 1 route in the order every own character in the
/// owner's log took it. The Grim Tangle (`G1_6`) before it implies nothing.
#[test]
fn secrets_in_the_dark_done_on_g1_7() {
    let data = Arc::new(data());
    let mut clock = 0;
    let mut lines = vec![format!("{} ***** LOG FILE OPENING *****", &stamp(clock)[..19])];
    lines.extend(entry_lines(&mut clock, "G1_1", 1, 1_414_213_562, "The Riverbank"));
    clock += 200;
    lines.push(format!(
        "{} [INFO Client 6200] : {SECRETS_CHARACTER} (Ranger) is now level 2",
        stamp(clock)
    ));
    for (code, level, seed, name) in [
        ("G1_town", 15, 1, "Clearfell Encampment"),
        ("G1_2", 2, 1_732_050_807, "Clearfell"),
        ("G1_town", 15, 1, "Clearfell Encampment"),
        ("G1_4", 4, 2_645_751_311, "The Grelwood"),
        ("G1_5", 6, 3_162_277_660, "The Red Vale"),
    ] {
        lines.extend(entry_lines(&mut clock, code, level, seed, name));
    }
    clock += 90;
    lines.push(format!("{} [INFO Client 6200] The Rust King: For the clan!", stamp(clock)));
    for (code, level, seed, name) in [
        ("G1_town", 15, 1, "Clearfell Encampment"),
        ("G1_6", 7, 3_316_624_790, "The Grim Tangle"),
    ] {
        lines.extend(entry_lines(&mut clock, code, level, seed, name));
    }
    let cemetery = entry_lines(&mut clock, "G1_7", 8, 3_605_551_275, "Cemetery of the Eternals");

    let mut engine = Engine::new(Arc::clone(&data));
    for l in &lines {
        engine.feed_line(l, false);
    }
    assert_eq!(engine.active(), Some(SECRETS_CHARACTER));

    // Baseline: after the Rust King and the Grim Tangle the quest is still in
    // progress at the boss, the talks with Renly and Una after him unseen.
    let state = engine
        .quest_state(SECRETS_CHARACTER, "secrets-in-the-dark")
        .expect("state");
    assert_eq!(state.status, Status::InProgress);
    assert_eq!(state.current_step.as_deref(), Some("step-19"));

    // Target: the Cemetery entry finishes it, with a transition to done.
    let mut moved = Vec::new();
    for l in &cemetery {
        moved.extend(engine.feed_line(l, false));
    }
    assert_eq!(
        engine.status(SECRETS_CHARACTER, "secrets-in-the-dark"),
        Some(Status::Done)
    );
    assert!(
        moved
            .iter()
            .any(|t| t.quest_id == "secrets-in-the-dark" && t.to == Status::Done),
        "no transition to done: {moved:?}"
    );
    // The data says the done is inferred, so the panel can say so.
    assert!(data.quest("secrets-in-the-dark").expect("quest").done_inferred);
}

/// Una's lines in the Grelwood and the Grim Tangle, as the owner's log
/// recorded them (Live 2 and 6 earlier sessions, 2026-10-25).
const UNA_ASKS: &str = "What happened here...?";
const UNA_WAITS: &str = "I'll wait here. The old magicks will protect me.";
const UNA_SPIKES: &str = "Good. Now the others!";
const UNA_ROOTS: &str = "The old magicks still thrive! The way is open. Please, hurry.";

/// The invented character of the Live 2 replays below.
const GRELWOOD_CHARACTER: &str = "Brannoc_Weld";

/// An Act 1 route built line by line in the shapes the log writes, fed
/// through the engine over the shipped data.
struct Replay {
    engine: Engine,
    clock: u32,
    name: &'static str,
}

impl Replay {
    /// The log opening, The Riverbank at level 1 and, 200 s later, the
    /// level-up that names the character.
    fn start(data: &Arc<QuestData>, name: &'static str) -> Replay {
        let mut r = Replay { engine: Engine::new(Arc::clone(data)), clock: 0, name };
        let open = format!("{} ***** LOG FILE OPENING *****", &stamp(0)[..19]);
        r.engine.feed_line(&open, false);
        r.enter("G1_1");
        r.clock += 200;
        let up = format!("{} [INFO Client 6200] : {name} (Ranger) is now level 2", stamp(r.clock));
        r.engine.feed_line(&up, false);
        assert_eq!(r.engine.active(), Some(name));
        r
    }

    fn feed(&mut self, lines: &[String]) -> Vec<Transition> {
        lines.iter().flat_map(|l| self.engine.feed_line(l, false)).collect()
    }

    /// Enters an Act 1 area, with the level, seed and scene name the
    /// replays use for it.
    fn enter(&mut self, code: &str) -> Vec<Transition> {
        let (level, seed, name) = match code {
            "G1_1" => (1, 1_414_213_562, "The Riverbank"),
            "G1_town" => (15, 1, "Clearfell Encampment"),
            "G1_2" => (2, 1_732_050_807, "Clearfell"),
            "G1_3" => (3, 2_236_067_977, "Mud Burrow"),
            "G1_4" => (4, 2_645_751_311, "The Grelwood"),
            "G1_5" => (6, 3_162_277_660, "The Red Vale"),
            "G1_6" => (7, 3_316_624_790, "The Grim Tangle"),
            "G1_7" => (8, 3_605_551_275, "Cemetery of the Eternals"),
            _ => panic!("no entry for {code}"),
        };
        let lines = entry_lines(&mut self.clock, code, level, seed, name);
        self.feed(&lines)
    }

    /// An NPC line a few seconds after the last line.
    fn say(&mut self, speaker: &str, line: &str) -> Vec<Transition> {
        self.clock += 3;
        let l = format!("{} [INFO Client 6200] {speaker}: {line}", stamp(self.clock));
        self.feed(&[l])
    }

    /// A reward line for the character, its stat in the log's markup, a few
    /// seconds after the last line.
    fn received(&mut self, stat: &str) -> Vec<Transition> {
        self.clock += 3;
        let l = format!("{} [INFO Client 6200] : {} has received {stat}.", stamp(self.clock), self.name);
        self.feed(&[l])
    }

    fn state(&self, quest: &str) -> QuestState {
        self.engine
            .quest_state(self.name, quest)
            .unwrap_or_else(|| panic!("no state for quest {quest}"))
    }

    /// Secrets in the Dark up to the Grim Tangle, in the order the owner's
    /// log has it: the tree and Una, the Red Vale and the Rust King, the
    /// encampment, the first Rune Spike, then the Grim Tangle.
    fn to_the_grim_tangle(&mut self) {
        for code in ["G1_town", "G1_2", "G1_town", "G1_4"] {
            self.enter(code);
        }
        self.say(TOWN_SPOKEN, UNA_ASKS);
        self.say(TOWN_SPOKEN, UNA_WAITS);
        self.enter("G1_5");
        self.say("The Rust King", "For the clan!");
        self.enter("G1_town");
        self.enter("G1_4");
        self.say(TOWN_SPOKEN, UNA_SPIKES);
        self.enter("G1_6");
    }
}

/// The title of the step the row shows next: the one after the highest
/// reached, or the first when none is.
fn next_title<'a>(data: &'a QuestData, state: &QuestState) -> &'a str {
    let q = data.quest(&state.quest_id).expect("quest");
    let k = match &state.current_step {
        None => 0,
        Some(id) => q.steps.iter().position(|s| &s.id == id).expect("current step") + 1,
    };
    q.steps.get(k).map_or("", |s| s.title.as_str())
}

/// Live 2: Secrets in the Dark stayed `available` from Clearfell to the
/// Grelwood, while Renly had given it in the encampment. Its `start` is
/// Reaching Clearfell done (inferred: the talk is not logged, and the
/// Clearfell entry is the earliest evidence).
#[test]
fn secrets_in_the_dark_in_progress_when_reaching_clearfell_done() {
    let data = Arc::new(data());
    let mut r = Replay::start(&data, GRELWOOD_CHARACTER);
    r.enter("G1_town");

    // Baseline: before Clearfell, Reaching Clearfell is in progress and
    // Secrets in the Dark is locked.
    assert_eq!(r.state("reaching-clearfell").status, Status::InProgress);
    assert_eq!(r.state("secrets-in-the-dark").status, Status::Locked);

    // Target: the Clearfell entry finishes Reaching Clearfell and starts
    // Secrets in the Dark with no step reached, so the step to do is its
    // first, in one transition.
    let moved = r.enter("G1_2");
    assert_eq!(r.state("reaching-clearfell").status, Status::Done);
    let s = r.state("secrets-in-the-dark");
    assert_eq!(s.status, Status::InProgress);
    assert_eq!(s.current_step, None);
    assert_eq!(next_title(&data, &s), "Travel to the Grelwood");
    let started: Vec<&Transition> = moved
        .iter()
        .filter(|t| t.quest_id == "secrets-in-the-dark")
        .collect();
    assert_eq!(started.len(), 1, "{moved:?}");
    assert_eq!(
        (started[0].from, started[0].to, started[0].step_to.as_deref()),
        (Status::Locked, Status::InProgress, None)
    );

    // The start is the data's, and its notes call it inferred.
    let q = data.quest("secrets-in-the-dark").expect("quest");
    assert!(
        matches!(q.start.as_slice(), [Signal::Quest { id, status: Status::Done }] if id == "reaching-clearfell"),
        "{:?}",
        q.start
    );
    assert!(q.notes.contains("Started (inferred)"), "{:?}", q.notes);

    // And the Grelwood entry still reaches the first step.
    r.enter("G1_town");
    r.enter("G1_4");
    assert_eq!(r.state("secrets-in-the-dark").current_step.as_deref(), Some("step-1"));
}

/// Live 2: in the Grelwood the row stayed on "Search the Grelwood" after the
/// tree and Una. Live 3: summoning her moved it to "Find the Red Vale",
/// before the talk. Her first Grelwood line, on the summon, ends the search,
/// so "Ask Una for help" is next; her wait line after it follows the talk
/// (inferred), so "Find the Red Vale" is next; her "Good. Now the others!"
/// after the Red Vale is the first Rune Spike (inferred from its place in
/// the order).
#[test]
fn una_at_the_tree_moves_secrets_to_find_the_red_vale() {
    let data = Arc::new(data());
    let mut r = Replay::start(&data, GRELWOOD_CHARACTER);
    for code in ["G1_town", "G1_2", "G1_town", "G1_4"] {
        r.enter(code);
    }

    // Baseline: the Grelwood entry alone leaves "Search the Grelwood" next.
    let s = r.state("secrets-in-the-dark");
    assert_eq!(s.current_step.as_deref(), Some("step-1"));
    assert_eq!(next_title(&data, &s), "Search the Grelwood");

    // Una's first line there, on the summon, makes "Ask Una for help" next.
    let moved = r.say(TOWN_SPOKEN, UNA_ASKS);
    let s = r.state("secrets-in-the-dark");
    assert_eq!(s.current_step.as_deref(), Some("step-2"));
    assert_eq!(next_title(&data, &s), "Ask Una for help");
    assert!(moved.iter().any(|t| t.quest_id == "secrets-in-the-dark"), "{moved:?}");

    // Target: her wait line, after the talk, makes "Find the Red Vale" next.
    let moved = r.say(TOWN_SPOKEN, UNA_WAITS);
    let s = r.state("secrets-in-the-dark");
    assert_eq!(s.current_step.as_deref(), Some("step-3"));
    assert_eq!(next_title(&data, &s), "Find the Red Vale");
    assert!(moved.iter().any(|t| t.quest_id == "secrets-in-the-dark"), "{moved:?}");

    // The same line in the encampment is no signal: a second character
    // that hears it there stays on the first step.
    let mut town = Replay::start(&data, "Ilse_Garrow");
    for code in ["G1_town", "G1_2", "G1_town", "G1_4", "G1_town"] {
        town.enter(code);
    }
    assert!(town.say(TOWN_SPOKEN, UNA_ASKS).is_empty());
    assert_eq!(town.state("secrets-in-the-dark").current_step.as_deref(), Some("step-1"));

    // After the Red Vale and the Rust King, her line back at the tree is
    // the first Rune Spike, and the last one is next.
    r.enter("G1_5");
    r.say("The Rust King", "For the clan!");
    assert_eq!(r.state("secrets-in-the-dark").current_step.as_deref(), Some("step-19"));
    r.enter("G1_town");
    r.enter("G1_4");
    r.say(TOWN_SPOKEN, UNA_SPIKES);
    let s = r.state("secrets-in-the-dark");
    assert_eq!(s.current_step.as_deref(), Some("step-23"));
    assert_eq!(next_title(&data, &s), "Use the Last Rune Spike");
    assert_eq!(s.status, Status::InProgress);
}

/// Live 2: The Mysterious Shade showed in progress from the Grelwood entry,
/// before its unlock, because "Find the Grim Tangle" carried that entry.
/// The Grelwood is on the way to the quest, not a step of it.
#[test]
fn mysterious_shade_stays_locked_through_the_grelwood() {
    let data = Arc::new(data());
    let mut r = Replay::start(&data, GRELWOOD_CHARACTER);
    for code in ["G1_town", "G1_2", "G1_town", "G1_4"] {
        r.enter(code);
    }
    r.say(TOWN_SPOKEN, UNA_ASKS);
    r.enter("G1_5");
    r.enter("G1_town");
    r.enter("G1_4");

    // Target: Secrets in the Dark is not done, and every Grelwood entry so
    // far leaves the Shade locked.
    assert_ne!(r.state("secrets-in-the-dark").status, Status::Done);
    let shade = r.state("the-mysterious-shade");
    assert_eq!(shade.status, Status::Locked);
    assert_eq!(shade.current_step, None);

    // Positive control: the Grim Tangle entry, its unlock and first step,
    // does move it, so the replay can move this quest.
    let moved = r.enter("G1_6");
    let shade = r.state("the-mysterious-shade");
    assert_eq!(shade.status, Status::InProgress);
    assert_eq!(shade.current_step.as_deref(), Some("step-1"));
    assert!(moved.iter().any(|t| t.quest_id == "the-mysterious-shade"), "{moved:?}");
}

/// Live 2 and 6 earlier sessions: Una's Grim Tangle line comes after the
/// first Rune Spike and is "Speak to Una about the roots".
#[test]
fn una_clears_the_roots_moves_the_shade() {
    let data = Arc::new(data());
    let mut r = Replay::start(&data, GRELWOOD_CHARACTER);
    r.to_the_grim_tangle();

    // Baseline: the Grim Tangle entry is the roots blocking the way.
    let shade = r.state("the-mysterious-shade");
    assert_eq!(shade.current_step.as_deref(), Some("step-1"));

    // Target: her line moves the Shade past "Speak to Una about the roots".
    let moved = r.say(TOWN_SPOKEN, UNA_ROOTS);
    let shade = r.state("the-mysterious-shade");
    assert_eq!(shade.status, Status::InProgress);
    assert_eq!(shade.current_step.as_deref(), Some("step-6"));
    assert_eq!(next_title(&data, &shade), "Continue through the Grim Tangle");
    assert!(moved.iter().any(|t| t.quest_id == "the-mysterious-shade"), "{moved:?}");

    // The same line in the Grelwood is not that step.
    let mut other = Replay::start(&data, "Ilse_Garrow");
    other.to_the_grim_tangle();
    other.enter("G1_4");
    assert!(other.say(TOWN_SPOKEN, UNA_ROOTS).is_empty());
    assert_eq!(other.state("the-mysterious-shade").current_step.as_deref(), Some("step-1"));

    // The Cemetery still finishes Secrets in the Dark and reaches the grave.
    r.enter("G1_7");
    assert_eq!(r.state("secrets-in-the-dark").status, Status::Done);
    assert_eq!(r.state("the-mysterious-shade").current_step.as_deref(), Some("step-8"));
}

/// The phrase a quest's notes open its inferred done with.
const INFERRED_DONE: &str = "Done (inferred)";

/// `done_inferred` is set on exactly the quests whose notes call their done
/// inferred, and on none whose done is a reward or points line (the quest's
/// own signal).
#[test]
fn done_inferred_matches_notes() {
    let data = data();
    let mut flagged = BTreeSet::new();
    for q in &data.quests {
        let said = q.notes.contains(INFERRED_DONE);
        assert_eq!(
            q.done_inferred, said,
            "quest {}: done_inferred {} but the notes {} call its done inferred",
            q.id,
            q.done_inferred,
            if said { "do" } else { "do not" }
        );
        if q.done_inferred {
            flagged.insert(q.id.as_str());
            assert!(
                !q.done.is_empty()
                    && q.done
                        .iter()
                        .all(|s| !matches!(s, Signal::Reward { .. } | Signal::Points { .. })),
                "quest {}: inferred, but its done is its own reward line: {:?}",
                q.id,
                q.done
            );
        }
    }
    // The three Context names, so the match above is not vacuous.
    assert_eq!(
        flagged,
        BTreeSet::from(["reaching-clearfell", "secrets-in-the-dark", "the-mad-wolf-of-ogham"])
    );
    // Negative control: a boss quest whose notes infer a step, not the done
    // (its done is the reward line), carries no flag.
    let blackjaw = data.quest("blackjaw-the-remnant").expect("blackjaw");
    assert!(blackjaw.notes.contains("inferred"), "{:?}", blackjaw.notes);
    assert!(!blackjaw.done_inferred);
}

/// The owner's find-it tips (after Live 3, 2026-10-25, with the two tidied
/// texts the owner chose), as quest, step, the step's title and the tip. The
/// titles are poe2db's except Beira's "Find ...", which is ours; pinning them
/// keeps a tip from drifting onto another step unnoticed.
const OWNER_TIPS: [(&str, &str, &str, &str); 5] = [
    ("reaching-clearfell", "step-2", "Slay the Bloated Miller", "Follow up river."),
    ("treacherous-ground", "step-2", "Find the Mud Burrow", "Follow the burrow."),
    (
        "beira-of-the-rotten-pack",
        "find",
        "Find Beira of the Rotten Pack",
        "More wolves means you are closer to Beira.",
    ),
    ("secrets-in-the-dark", "step-1", "Travel to the Grelwood", "Follow up river again."),
    ("secrets-in-the-dark", "step-2", "Search the Grelwood", "Should be close to entrance etc."),
];

/// The tip of the step the row shows next, as [`next_title`] finds it.
fn next_tip<'a>(data: &'a QuestData, state: &QuestState) -> Option<&'a str> {
    let q = data.quest(&state.quest_id).expect("quest");
    let title = next_title(data, state);
    q.steps.iter().find(|s| s.title == title).and_then(|s| s.tip.as_deref())
}

/// Each of the owner's tips is on its step, and that step is the one the row
/// shows next while the player is doing what the tip helps with.
#[test]
fn owner_tips_are_on_their_steps() {
    let data = Arc::new(data());
    for (quest, step, title, tip) in OWNER_TIPS {
        let q = data.quest(quest).unwrap_or_else(|| panic!("no quest {quest}"));
        let s = q
            .steps
            .iter()
            .find(|s| s.id == step)
            .unwrap_or_else(|| panic!("quest {quest}: no step {step}"));
        assert_eq!(s.title, title, "quest {quest}: step {step}");
        assert_eq!(s.tip.as_deref(), Some(tip), "quest {quest}: step {step}");
    }
    // Negative control: the step after each tipped one carries none of them.
    let ask = data.quest("secrets-in-the-dark").and_then(|q| q.steps.iter().find(|s| s.id == "step-3"));
    assert_eq!(ask.expect("step-3").tip, None);
    let defeat = data.quest("beira-of-the-rotten-pack").and_then(|q| q.steps.iter().find(|s| s.id == "fight"));
    assert_eq!(defeat.expect("fight").tip, None);

    // In play: The Riverbank shows the Miller's tip, Clearfell the burrow's,
    // Beira's and the way to the Grelwood, and the Grelwood the tree's.
    let tip = |quest: &str, step: &str| {
        OWNER_TIPS
            .iter()
            .find(|t| t.0 == quest && t.1 == step)
            .map(|t| t.3)
            .expect("an owner tip")
    };
    let mut r = Replay::start(&data, GRELWOOD_CHARACTER);
    let s = r.state("reaching-clearfell");
    assert_eq!(next_tip(&data, &s), Some(tip("reaching-clearfell", "step-2")));
    r.enter("G1_town");
    r.enter("G1_2");
    let s = r.state("treacherous-ground");
    assert_eq!(next_tip(&data, &s), Some(tip("treacherous-ground", "step-2")));
    let s = r.state("beira-of-the-rotten-pack");
    assert_eq!(next_tip(&data, &s), Some(tip("beira-of-the-rotten-pack", "find")));
    let s = r.state("secrets-in-the-dark");
    assert_eq!(next_tip(&data, &s), Some(tip("secrets-in-the-dark", "step-1")));
    r.enter("G1_town");
    r.enter("G1_4");
    let s = r.state("secrets-in-the-dark");
    assert_eq!(next_tip(&data, &s), Some(tip("secrets-in-the-dark", "step-2")));
}

/// Live 3: the owner took Treacherous Ground from Renly in Clearfell
/// Encampment, and it showed only from the Clearfell entry. The talk is not
/// in the log (Live 3's town visit held no NPC line), so the town entry, the
/// signal Finding the Forge unlocks on too, makes it available (inferred).
#[test]
fn treacherous_ground_available_in_town() {
    let data = Arc::new(data());
    let q = data.quest("treacherous-ground").expect("quest");
    for code in [TOWN_CODE, "G1_2"] {
        assert!(
            q.unlock.iter().any(|s| area_codes(s).contains(&code)),
            "its unlock lacks {code}: {:?}",
            q.unlock
        );
    }
    assert!(q.notes.contains("Available (inferred)"), "{:?}", q.notes);

    // Baseline: The Riverbank alone leaves it locked.
    let mut r = Replay::start(&data, GRELWOOD_CHARACTER);
    assert_eq!(r.state("treacherous-ground").status, Status::Locked);

    // Target: the encampment entry makes it available, no step reached, in
    // one transition.
    let moved = r.enter(TOWN_CODE);
    let s = r.state("treacherous-ground");
    assert_eq!((s.status, s.current_step.as_deref()), (Status::Available, None));
    let mine: Vec<&Transition> = moved.iter().filter(|t| t.quest_id == "treacherous-ground").collect();
    assert_eq!(mine.len(), 1, "{moved:?}");
    assert_eq!((mine[0].from, mine[0].to), (Status::Locked, Status::Available));

    // Clearfell puts it in progress, as before: the Mud Burrow is next.
    r.enter("G1_2");
    let s = r.state("treacherous-ground");
    assert_eq!(s.status, Status::InProgress);
    assert_eq!(s.current_step.as_deref(), Some("step-1"));
    assert_eq!(next_title(&data, &s), "Find the Mud Burrow");
}

const BEIRA: &str = "beira-of-the-rotten-pack";
const BEIRA_SPEAKER: &str = "Beira of the Rotten Pack";

/// Her first line each fight, then one of her combat barks, as the owner's
/// log recorded them (9 sessions, 2026-10-25).
const BEIRA_FIRST: &str = "Sleep in ice, wake in snow. Sleep my pretties as you grow...";
const BEIRA_BARK: &str = "Be still!";

/// Her reward line's stat, in the log's markup.
const BEIRA_COLD: &str = "+10% to [Resistances|Cold Resistance]";

/// Live 2 and Live 3 showed "Use the item it dropped" at Beira's first line.
/// No line marks her defeat as far as measured, so the owner ticks it
/// (2026-10-25): her lines are "Find ...", "Defeat ..." waits for the tick
/// whatever area comes next, and her reward line finishes the quest in any
/// area, from either step.
#[test]
fn beira_defeat_is_ticked_by_hand() {
    let data = Arc::new(data());
    let mut r = Replay::start(&data, GRELWOOD_CHARACTER);
    r.enter(TOWN_CODE);
    r.enter("G1_2");

    // Baseline: Clearfell reached, Beira not met.
    let s = r.state(BEIRA);
    assert_eq!(s.current_step.as_deref(), Some("enter"));
    assert_eq!(next_title(&data, &s), "Find Beira of the Rotten Pack");

    // Her lines find her: "Defeat ..." is next, not "Use ...".
    r.say(BEIRA_SPEAKER, BEIRA_FIRST);
    r.say(BEIRA_SPEAKER, BEIRA_BARK);
    let s = r.state(BEIRA);
    assert_eq!(s.current_step.as_deref(), Some("find"));
    assert_eq!(next_title(&data, &s), "Defeat Beira of the Rotten Pack");

    // Nothing the log records afterwards marks her defeat: the Mud Burrow,
    // Clearfell again and the Grelwood leave "Defeat ..." next.
    for code in ["G1_3", "G1_2", "G1_4"] {
        r.enter(code);
        let s = r.state(BEIRA);
        assert_eq!(s.status, Status::InProgress, "after {code}");
        assert_eq!(next_title(&data, &s), "Defeat Beira of the Rotten Pack", "after {code}");
    }

    // Target: the player's tick on "Defeat ..." makes the item's use next.
    r.engine
        .tick_step(GRELWOOD_CHARACTER, BEIRA, "fight", true)
        .expect("tick the defeat");
    let s = r.state(BEIRA);
    assert_eq!(s.status, Status::InProgress);
    assert_eq!(s.current_step.as_deref(), Some("fight"));
    assert_eq!(next_title(&data, &s), "Use the item it dropped");

    // The reward line, read in the Grelwood, finishes it.
    let moved = r.received(BEIRA_COLD);
    assert_eq!(r.state(BEIRA).status, Status::Done);
    assert!(moved.iter().any(|t| t.quest_id == BEIRA && t.to == Status::Done), "{moved:?}");

    // A character that uses the item at once goes from "Defeat ..." to done
    // with no tick.
    let mut quick = Replay::start(&data, "Ilse_Garrow");
    quick.enter(TOWN_CODE);
    quick.enter("G1_2");
    quick.say(BEIRA_SPEAKER, BEIRA_FIRST);
    let s = quick.state(BEIRA);
    assert_eq!(next_title(&data, &s), "Defeat Beira of the Rotten Pack");
    quick.received(BEIRA_COLD);
    let s = quick.state(BEIRA);
    assert_eq!(s.status, Status::Done);
    assert!(!s.steps.iter().any(|st| st.ticked), "{:?}", s.steps);
}

/// Live 3: the person summoned Una at the tree and the game closed 2.5 min
/// later without her wait line. Her first line alone ends the search and
/// leaves the talk to do; the Red Vale entry, where the wait line never came,
/// reaches the steps before it.
#[test]
fn una_summon_alone_leaves_ask_una_for_help() {
    let data = Arc::new(data());
    let mut r = Replay::start(&data, GRELWOOD_CHARACTER);
    for code in [TOWN_CODE, "G1_2", TOWN_CODE, "G1_4"] {
        r.enter(code);
    }

    // Target: the summon leaves "Ask Una for help" next.
    r.say(TOWN_SPOKEN, UNA_ASKS);
    let s = r.state("secrets-in-the-dark");
    assert_eq!(s.current_step.as_deref(), Some("step-2"));
    assert_eq!(next_title(&data, &s), "Ask Una for help");

    // A later visit to the encampment and back leaves it there.
    r.enter(TOWN_CODE);
    r.enter("G1_4");
    let s = r.state("secrets-in-the-dark");
    assert_eq!(s.current_step.as_deref(), Some("step-2"));
    assert_eq!(next_title(&data, &s), "Ask Una for help");

    // The Red Vale entry reaches "Find the Red Vale" without the wait line.
    r.enter("G1_5");
    let s = r.state("secrets-in-the-dark");
    assert_eq!(s.status, Status::InProgress);
    assert_eq!(s.current_step.as_deref(), Some("step-4"));
    assert_eq!(next_title(&data, &s), "Search for Runes of Power");
}
