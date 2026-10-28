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
//! "Defeat ..." step after it has no log signal (only the screen reader's
//! `bar_gone` of the boss), a "Use ..." step follows, and the reward line
//! still finishes the quest. Only Beira's step says the boss drops an item;
//! for the others that is not established.
//!
//! `shipped_screen_signals_are_the_four_named` pins the `bar_gone`,
//! `quest_complete` and `remark` signals the data ships (Context "Design:
//! quest data and the engine"; the Miller's bar joined the four on
//! 2026-10-28, and the Tree of Souls remark the same day), and
//! `beira_bar_gone_ticks_the_defeat_over_the_shipped_data` feeds one through
//! the engine. `shipped_kill_remark_signals_are_the_established_bosses` pins
//! the `kill_remark` signals, one per boss step whose kill remark poe2db's
//! class pages establish, and the `boss_kill_remark_*` tests feed two.
//!
//! The `miller_*` tests pin "Slay the Bloated Miller" (2026-10-28): it ticks
//! at his death, on the level-up to 2 the game logs at the kill, on his bar
//! going or the character's kill remark, or, when none came, on Mortimer's
//! thanks, and never twice.
//!
//! The `tree_remark_*` tests pin "Search the Grelwood" (2026-10-28, the
//! owner's Ranger at the Tree of Souls): the character's remark there is not
//! in the log, and the screen reader's `remark` for its key ends the search
//! in the Grelwood only, before Una's summon line, and never twice.
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
    Detection, Engine, Evidence, Quest, QuestData, QuestState, RewardCategory, ScreenEvent,
    ScreenOutcome, ScreenWhat, Signal, Status, Timestamp, Transition, Worth,
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
        Signal::Screen { area, .. } | Signal::LevelUp { area, .. } => vec![area.as_str()],
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
    // Every area belongs to a campaign act and every speaker is used, by a
    // signal or as a kill remark's boss (The Bloated Miller, whose own lines
    // tick nothing), so the allowlist stays the curated one and not a dump of
    // every logged name.
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
    for r in &data.remarks {
        used.extend(r.boss.as_deref());
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
/// Una is on the allowlist for her lines in the Grelwood and the Grim Tangle
/// (Live 2, 2026-10-25), and Mortimer for his line in The Riverbank after the
/// Miller's death (2026-10-27); each of their signals names its area and its
/// exact line ([`TOWN_LISTED`]).
const TOWN_VOICES: [&str; 4] = ["Una", "Finn", "Renly", "Mortimer"];

/// The town voice the shipped allowlist reads in `town_voices.txt`, the town
/// voices it lists, and the encampment's code.
const TOWN_SPOKEN: &str = "Una";
const TOWN_LISTED: [&str; 2] = [TOWN_SPOKEN, MORTIMER];
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
        if !TOWN_LISTED.contains(&voice) {
            assert!(!listed, "{voice}'s greetings are on the speaker allowlist");
            continue;
        }
        // Una and Mortimer are listed, and no signal of theirs can match a
        // line in town: each names an area outside the encampment and an
        // exact line.
        assert!(listed, "{voice} is not on the speaker allowlist");
        let signals = npc_signals(&data, voice);
        assert!(!signals.is_empty(), "{voice} is listed but no signal uses them");
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
    // step no line marks (the player ticks it, or the screen reader sees the
    // boss's bar go), then a "Use ..." step no signal can show, and the
    // reward line as `done`.
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
        assert!(
            fight.signals.iter().all(|s| matches!(
                s,
                Signal::Screen { what: ScreenWhat::BarGone | ScreenWhat::KillRemark, area, .. }
                    if q.zones.contains(area)
            )),
            "quest {id}: the defeat step has a log signal: {:?}",
            fight.signals
        );
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
    /// The tailer's flag for every line fed: false for catch-up.
    live: bool,
}

impl Replay {
    /// The log opening, The Riverbank at level 1 and, 200 s later, the
    /// level-up that names the character. That level-up is the one the game
    /// logs at the Miller's kill, so it ticks "Slay the Bloated Miller".
    fn start(data: &Arc<QuestData>, name: &'static str) -> Replay {
        let mut r = Replay::opening(data, name, false);
        r.enter("G1_1");
        r.clock += 200;
        let up = format!("{} [INFO Client 6200] : {name} (Ranger) is now level 2", stamp(r.clock));
        r.engine.feed_line(&up, false);
        assert_eq!(r.engine.active(), Some(name));
        r
    }

    /// The log opening and The Riverbank at level 1, fed live, with the
    /// entry decided: the new character from The Riverbank is open and
    /// unnamed until the test's own level-up ([`Replay::level_up`]).
    fn from_the_riverbank(data: &Arc<QuestData>, name: &'static str) -> Replay {
        let mut r = Replay::opening(data, name, true);
        r.enter("G1_1");
        r.clock += 10;
        r.engine.expire_relog_window(Timestamp::parse(&stamp(r.clock)[..19]).expect("a log time"));
        assert!(r.engine.new_character_open());
        assert_eq!(r.engine.current(), None);
        r
    }

    fn opening(data: &Arc<QuestData>, name: &'static str, live: bool) -> Replay {
        let mut r = Replay { engine: Engine::new(Arc::clone(data)), clock: 0, name, live };
        let open = format!("{} ***** LOG FILE OPENING *****", &stamp(0)[..19]);
        r.engine.feed_line(&open, live);
        r
    }

    fn feed(&mut self, lines: &[String]) -> Vec<Transition> {
        lines.iter().flat_map(|l| self.engine.feed_line(l, self.live)).collect()
    }

    /// A level-up line of `who` to `level`, a few seconds after the last line.
    fn level_up_of(&mut self, who: &str, level: u32) -> Vec<Transition> {
        self.clock += 3;
        let l = format!("{} [INFO Client 6200] : {who} (Ranger) is now level {level}", stamp(self.clock));
        self.feed(&[l])
    }

    /// The character's own level-up.
    fn level_up(&mut self, level: u32) -> Vec<Transition> {
        self.level_up_of(self.name, level)
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

    /// The quest's state for the character, or for the new character from
    /// The Riverbank while no level-up has named it.
    fn state(&self, quest: &str) -> QuestState {
        let named = self.engine.character(self.name).is_some();
        let state = if named {
            self.engine.quest_state(self.name, quest)
        } else {
            self.engine
                .new_character_states()
                .and_then(|s| s.into_iter().find(|q| q.quest_id == quest))
        };
        state.unwrap_or_else(|| panic!("no state for quest {quest}"))
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
    ("secrets-in-the-dark", "step-2", "Search the Grelwood", "Should be close to entrance."),
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
    // The Miller's step is open before the level-up at his kill, which
    // names the character (moved off `Replay::start` on 2026-10-28).
    let mut r = Replay::from_the_riverbank(&data, GRELWOOD_CHARACTER);
    let s = r.state("reaching-clearfell");
    assert_eq!(next_tip(&data, &s), Some(tip("reaching-clearfell", "step-2")));
    r.level_up(2);
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

const MORTIMER: &str = "Mortimer";
const MILLER: &str = "The Bloated Miller";

/// The Riverbank's fight with The Bloated Miller as the owner's log recorded
/// it on 2026-10-27: Mortimer's call, the Miller's lines from the fight's
/// start to his last, and Mortimer's line after the kill.
const MORTIMER_CALLS: &str = "Foundling! We need your strength!";
const MILLER_LINES: [&str; 4] = ["Saw you in half!", "Split you!", "Die!", "Die!"];
const MORTIMER_THANKS: &str = "Well done! Please come inside.";

/// 2026-10-27, two live sessions: "Slay the Bloated Miller" ticked on his
/// first line, at the fight's start, and the row showed "Enter town" while he
/// was alive. The step waits for his death: Mortimer's line after the kill
/// is the route when no level-up to 2 came in The Riverbank, so the new
/// character here stays unnamed (moved off [`Replay::start`], whose naming
/// level-up now ticks the step, on 2026-10-28).
#[test]
fn miller_step_waits_for_his_death() {
    let data = Arc::new(data());
    let mut r = Replay::from_the_riverbank(&data, GRELWOOD_CHARACTER);

    // Baseline: in The Riverbank the row shows the Miller's step.
    let s = r.state("reaching-clearfell");
    assert_eq!((s.status, s.current_step.as_deref()), (Status::InProgress, Some("step-1")));
    assert_eq!(next_title(&data, &s), "Slay the Bloated Miller");

    // The fight: Mortimer's call and every line of the Miller's, from the
    // first, move nothing.
    let mut moved = r.say(MORTIMER, MORTIMER_CALLS);
    for line in MILLER_LINES {
        moved.extend(r.say(MILLER, line));
    }
    assert!(moved.is_empty(), "{moved:?}");
    assert_eq!(next_title(&data, &r.state("reaching-clearfell")), "Slay the Bloated Miller");

    // Target: Mortimer's line after the kill ticks it, in one transition, and
    // "Enter town" is next.
    let moved = r.say(MORTIMER, MORTIMER_THANKS);
    let s = r.state("reaching-clearfell");
    assert_eq!(s.current_step.as_deref(), Some("step-2"));
    assert_eq!(next_title(&data, &s), "Enter town");
    assert_eq!(moved.len(), 1, "{moved:?}");
    assert_eq!(moved[0].step_to.as_deref(), Some("step-2"));

    // Negative control: the same line in the encampment, where Mortimer is a
    // gambling vendor, ticks nothing.
    let mut town = Replay::from_the_riverbank(&data, GRELWOOD_CHARACTER);
    town.enter(TOWN_CODE);
    let before = town.state("reaching-clearfell");
    assert!(town.say(MORTIMER, MORTIMER_THANKS).is_empty());
    assert_eq!(town.state("reaching-clearfell"), before);
}

/// The invented stored character of the level-up baselines.
const STORED_CHARACTER: &str = "Ottilie_Varn";
const STORED_SEED: u64 = 2_718_281_828;

/// Whether any `step-2` signal of Reaching Clearfell is in `name`'s evidence.
fn slay_evidence(engine: &Engine, name: &str) -> bool {
    engine.character(name).is_some_and(|c| {
        c.evidence
            .get("reaching-clearfell")
            .is_some_and(|e| e.iter().any(|id| id.starts_with("step/step-2/")))
    })
}

/// A stored own character of `level` loaded and in The Riverbank, active:
/// relogging by its instance's seed, or, with `by_build_line`, by the
/// character build line the client writes within 2 s of a relog into an
/// instance of no stored character. "Slay the Bloated Miller" is open.
fn stored_in_the_riverbank(data: &Arc<QuestData>, level: u32, by_build_line: bool) -> Replay {
    let mut r = Replay::opening(data, STORED_CHARACTER, true);
    let mut record = serde_json::json!({
        "name": STORED_CHARACTER, "class": "Ranger", "level": level, "ownership": "own"
    });
    if !by_build_line {
        record["riverbank_seed"] = STORED_SEED.into();
    }
    r.engine.load_character(&record).expect("a stored character");
    let seed = if by_build_line { STORED_SEED + 1 } else { STORED_SEED };
    let mut lines = entry_lines(&mut r.clock, "G1_1", 1, seed, "The Riverbank");
    if by_build_line {
        lines.push(format!(
            "{} [WARN Client 6200] Character {STORED_CHARACTER} has unknown build of path: None",
            stamp(r.clock + 1)
        ));
    }
    r.feed(&lines);
    assert_eq!(r.engine.current(), Some(STORED_CHARACTER));
    assert!(!r.engine.new_character_open());
    let s = r.state("reaching-clearfell");
    assert_eq!(s.current_step.as_deref(), Some("step-1"));
    assert_eq!(next_title(&data, &s), "Slay the Bloated Miller");
    r
}

/// Baseline (2026-10-28): the level-up ticks "Slay the Bloated Miller" only
/// as the character's first, to 2 in The Riverbank. A character already
/// level 2 or more, loaded and relogging there either way, has the step
/// open, and neither its level-up to 3 there, a level-up to 2 in another
/// area, a party member's level-up nor, while pinned, another name's
/// level-up ticks it.
#[test]
fn miller_level_up_ticks_nothing_for_a_character_already_level_2() {
    let data = Arc::new(data());
    for by_build_line in [false, true] {
        // The character's own level-up to 3 in The Riverbank.
        let mut r = stored_in_the_riverbank(&data, 2, by_build_line);
        let moved = r.level_up(3);
        assert!(moved.iter().all(|t| t.quest_id != "reaching-clearfell"), "{moved:?}");
        assert!(!slay_evidence(&r.engine, STORED_CHARACTER));
        assert_eq!(r.state("reaching-clearfell").current_step.as_deref(), Some("step-1"));
    }

    // A level-up to 2 in another area (The Mud Burrow, which Reaching
    // Clearfell does not name).
    let mut r = stored_in_the_riverbank(&data, 1, false);
    r.enter("G1_3");
    let moved = r.level_up(2);
    assert!(moved.iter().all(|t| t.quest_id != "reaching-clearfell"), "{moved:?}");
    assert!(!slay_evidence(&r.engine, STORED_CHARACTER));
    assert_eq!(r.state("reaching-clearfell").current_step.as_deref(), Some("step-1"));

    // A party member's level-up to 2 in The Riverbank.
    let mut r = stored_in_the_riverbank(&data, 2, false);
    r.clock += 1;
    r.feed(&[format!("{} [INFO Client 6200] : Bram_Holloway has joined the area.", stamp(r.clock))]);
    let moved = r.level_up_of("Bram_Holloway", 2);
    assert!(moved.is_empty(), "{moved:?}");
    assert!(!slay_evidence(&r.engine, STORED_CHARACTER));
    assert!(r.engine.character("Bram_Holloway").is_none());

    // While pinned, another name's level-up to 2 in The Riverbank.
    let mut r = stored_in_the_riverbank(&data, 2, false);
    r.engine.pin(Some(STORED_CHARACTER)).expect("pin");
    let moved = r.level_up_of("Wren_Ashdown", 2);
    assert!(moved.is_empty(), "{moved:?}");
    assert!(!slay_evidence(&r.engine, STORED_CHARACTER));
    assert!(r.engine.character("Wren_Ashdown").is_none());
    assert_eq!(r.state("reaching-clearfell").current_step.as_deref(), Some("step-1"));
}

/// The Miller's screen signals in the screen demand: (what, name) of every
/// demanded signal of Reaching Clearfell.
fn miller_demand(engine: &Engine) -> Vec<(ScreenWhat, String)> {
    engine
        .screen_demand()
        .into_iter()
        .filter(|d| d.quest_id == "reaching-clearfell")
        .map(|d| (d.what, d.name))
        .collect()
}

/// The transitions that move Reaching Clearfell to `step-2`.
fn to_step_2(moved: &[Transition]) -> Vec<&Transition> {
    moved
        .iter()
        .filter(|t| t.quest_id == "reaching-clearfell" && t.step_to.as_deref() == Some("step-2"))
        .collect()
}

/// Up to the Miller's kill: the new character from The Riverbank, unnamed,
/// with Mortimer's call and the Miller's lines heard, which move nothing.
fn at_the_miller(data: &Arc<QuestData>) -> Replay {
    let mut r = Replay::from_the_riverbank(data, GRELWOOD_CHARACTER);
    let mut moved = r.say(MORTIMER, MORTIMER_CALLS);
    for line in MILLER_LINES {
        moved.extend(r.say(MILLER, line));
    }
    assert!(moved.is_empty(), "{moved:?}");
    let s = r.state("reaching-clearfell");
    assert_eq!(s.current_step.as_deref(), Some("step-1"));
    r
}

/// Target (2026-10-28, the owner: "first level up can come only from the
/// miller"): the level-up to 2 in The Riverbank, logged at his kill, names
/// the new character and ticks "Slay the Bloated Miller" for it, in one
/// live transition; Mortimer's thanks after it adds none.
#[test]
fn miller_level_up_to_2_ticks_the_slay_step_over_the_shipped_data() {
    let data = Arc::new(data());
    let mut r = at_the_miller(&data);

    let moved = r.level_up(2);
    assert_eq!(r.engine.current(), Some(GRELWOOD_CHARACTER));
    let s = r.state("reaching-clearfell");
    assert_eq!(s.current_step.as_deref(), Some("step-2"));
    assert_eq!(next_title(&data, &s), "Enter town");
    let ticked = to_step_2(&moved);
    assert_eq!(ticked.len(), 1, "{moved:?}");
    assert_eq!(ticked[0].character, GRELWOOD_CHARACTER);
    assert!(ticked[0].live);
    assert!(slay_evidence(&r.engine, GRELWOOD_CHARACTER));

    let moved = r.say(MORTIMER, MORTIMER_THANKS);
    assert!(moved.is_empty(), "{moved:?}");
    assert_eq!(r.state("reaching-clearfell").current_step.as_deref(), Some("step-2"));
}

/// After the level-up ticked the step, the Miller's bar going and the
/// character's kill remark find no open step and add no transition, and the
/// screen demand no longer names him.
#[test]
fn miller_level_up_then_screen_events_tick_once() {
    let data = Arc::new(data());
    let mut r = at_the_miller(&data);
    assert_eq!(to_step_2(&r.level_up(2)).len(), 1);
    assert!(miller_demand(&r.engine).is_empty(), "{:?}", r.engine.screen_demand());

    for ev in [ScreenEvent::bar_gone(MILLER), ScreenEvent::kill_remark(MILLER)] {
        let got = r.engine.apply_screen(&ev);
        assert_eq!(got.outcome, ScreenOutcome::NoOpenStep, "{ev:?}");
        assert!(got.transitions.is_empty(), "{:?}", got.transitions);
    }
    assert_eq!(r.state("reaching-clearfell").current_step.as_deref(), Some("step-2"));
}

/// The other order: the bar goes while the character is still the unnamed
/// new character from The Riverbank. The screen event reaches it and ticks
/// the step; the level-up that follows names it, the named character keeps
/// the step, and the whole run moves to `step-2` exactly once.
#[test]
fn miller_screen_tick_reaches_the_new_character_before_its_name() {
    let data = Arc::new(data());
    let mut r = Replay::from_the_riverbank(&data, GRELWOOD_CHARACTER);
    let mut moved = r.say(MORTIMER, MORTIMER_CALLS);
    for line in MILLER_LINES {
        moved.extend(r.say(MILLER, line));
    }

    let got = r.engine.apply_screen(&ScreenEvent::bar_gone(MILLER));
    assert_eq!(
        got.outcome,
        ScreenOutcome::Ticked { quest_id: "reaching-clearfell".into(), step_id: "step-2".into() }
    );
    let ticked = to_step_2(&got.transitions);
    assert_eq!(ticked.len(), 1, "{:?}", got.transitions);
    assert_eq!(ticked[0].character, "", "the unnamed new character");
    assert!(ticked[0].live);
    moved.extend(got.transitions);

    moved.extend(r.level_up(2));
    assert_eq!(r.engine.current(), Some(GRELWOOD_CHARACTER));
    assert_eq!(
        r.engine.status(GRELWOOD_CHARACTER, "reaching-clearfell"),
        Some(Status::InProgress)
    );
    assert_eq!(r.state("reaching-clearfell").current_step.as_deref(), Some("step-2"));
    assert_eq!(to_step_2(&moved).len(), 1, "{moved:?}");
}

/// Before the level-up has come, either screen event of the Miller's ticks
/// "Slay the Bloated Miller" in one live transition; the demand in The
/// Riverbank names his two screen signals. `kill_remark` is the second.
fn miller_screen_event_ticks(ev: ScreenEvent) {
    let data = Arc::new(data());
    let mut r = at_the_miller(&data);
    assert_eq!(
        miller_demand(&r.engine),
        [(ScreenWhat::BarGone, MILLER.to_string()), (ScreenWhat::KillRemark, MILLER.to_string())]
    );
    let got = r.engine.apply_screen(&ev);
    assert_eq!(
        got.outcome,
        ScreenOutcome::Ticked { quest_id: "reaching-clearfell".into(), step_id: "step-2".into() },
        "{ev:?}"
    );
    let ticked = to_step_2(&got.transitions);
    assert_eq!(ticked.len(), 1, "{:?}", got.transitions);
    assert!(ticked[0].live);
    assert_eq!(next_title(&data, &r.state("reaching-clearfell")), "Enter town");
    assert!(miller_demand(&r.engine).is_empty());
}

#[test]
fn miller_bar_gone_ticks_the_slay_step_over_the_shipped_data() {
    miller_screen_event_ticks(ScreenEvent::bar_gone(MILLER));
}

#[test]
fn miller_kill_remark_ticks_the_slay_step_over_the_shipped_data() {
    miller_screen_event_ticks(ScreenEvent::kill_remark(MILLER));
}

/// The fire rule over the Miller's signals: nothing outside The Riverbank,
/// nothing once "Slay the Bloated Miller" is reached.
#[test]
fn miller_screen_events_tick_nothing_outside_the_open_step() {
    let data = Arc::new(data());
    let events = [ScreenEvent::bar_gone(MILLER), ScreenEvent::kill_remark(MILLER)];

    // In The Mud Burrow, with the step still open.
    let mut r = at_the_miller(&data);
    r.enter("G1_3");
    for ev in &events {
        let got = r.engine.apply_screen(ev);
        assert_eq!(got.outcome, ScreenOutcome::NoOpenStep, "{ev:?}");
        assert!(got.transitions.is_empty(), "{:?}", got.transitions);
    }
    assert_eq!(r.state("reaching-clearfell").current_step.as_deref(), Some("step-1"));

    // Back in The Riverbank once Mortimer's thanks reached the step.
    let mut r = at_the_miller(&data);
    assert_eq!(to_step_2(&r.say(MORTIMER, MORTIMER_THANKS)).len(), 1);
    for ev in &events {
        let got = r.engine.apply_screen(ev);
        assert_eq!(got.outcome, ScreenOutcome::NoOpenStep, "{ev:?}");
        assert!(got.transitions.is_empty(), "{:?}", got.transitions);
    }
}

/// The eight playable classes (poe2db's Ascendancy page, 2026-10-28).
const PLAYABLE_CLASSES: [&str; 8] =
    ["Warrior", "Mercenary", "Ranger", "Huntress", "Witch", "Sorceress", "Monk", "Druid"];

/// The Miller's kill remark texts are the eight classes' `MillerKillComment`
/// rows, one each, from each class's poe2db page.
#[test]
fn shipped_miller_remarks_cover_every_playable_class() {
    let data = data();
    let rows: Vec<_> = data.remarks.iter().filter(|r| r.boss.as_deref() == Some(MILLER)).collect();
    let classes: Vec<&str> = rows.iter().map(|r| r.class.as_str()).collect();
    let mut sorted = classes.clone();
    sorted.sort_unstable();
    let mut want = PLAYABLE_CLASSES.to_vec();
    want.sort_unstable();
    assert_eq!(sorted, want, "one row per playable class");
    for r in &rows {
        assert_eq!(r.key, "MillerKillComment", "{}", r.class);
        assert_eq!(r.source_url, format!("https://poe2db.tw/us/{}", r.class));
        assert_eq!(r.text, r.text.trim(), "{}", r.class);
    }
    let texts = data.remark_texts(MILLER);
    assert_eq!(texts.len(), 8, "{texts:?}");
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

// -- the Tree of Souls remark (2026-10-28) -----------------------------------

const SECRETS: &str = "secrets-in-the-dark";

/// The poe2db key of every class's remark on finding the Tree of Souls, and
/// the Ranger's text of it (https://poe2db.tw/us/Ranger, fetched
/// 2026-10-28), the line the owner's Ranger said at the tree.
const TREE_KEY: &str = "SinTreeSeenComment";
const RANGER_TREE: &str = "Has the Tree not endured enough?";

/// Up to the Grelwood: Secrets in the Dark in progress, its entry reached,
/// "Search the Grelwood" next.
fn at_the_tree(data: &Arc<QuestData>) -> Replay {
    let mut r = Replay::start(data, GRELWOOD_CHARACTER);
    for code in [TOWN_CODE, "G1_2", TOWN_CODE, "G1_4"] {
        r.enter(code);
    }
    let s = r.state(SECRETS);
    assert_eq!(s.current_step.as_deref(), Some("step-1"));
    assert_eq!(next_title(data, &s), "Search the Grelwood");
    r
}

/// The transitions that move Secrets in the Dark to `step`.
fn secrets_to<'a>(moved: &'a [Transition], step: &str) -> Vec<&'a Transition> {
    moved
        .iter()
        .filter(|t| t.quest_id == SECRETS && t.step_to.as_deref() == Some(step))
        .collect()
}

/// Secrets in the Dark's screen signals demanded now, as (step, what, name,
/// area).
fn secrets_demand(engine: &Engine) -> Vec<(String, ScreenWhat, String, String)> {
    engine
        .screen_demand()
        .into_iter()
        .filter(|d| d.quest_id == SECRETS)
        .map(|d| (d.step_id, d.what, d.name, d.area))
        .collect()
}

/// Baseline (2026-10-28, the owner's Ranger at the tree): a character's
/// remark is not in `Client.txt`, and a line carrying its text under the
/// character's name moves no quest, the character being no listed speaker.
/// Una's "What happened here...?" still ends the search.
#[test]
fn tree_remark_in_the_log_ticks_nothing() {
    let data = Arc::new(data());
    let mut r = at_the_tree(&data);
    let moved = r.say(GRELWOOD_CHARACTER, RANGER_TREE);
    assert!(moved.is_empty(), "{moved:?}");
    assert_eq!(r.state(SECRETS).current_step.as_deref(), Some("step-1"));

    let moved = r.say(TOWN_SPOKEN, UNA_ASKS);
    assert_eq!(secrets_to(&moved, "step-2").len(), 1, "{moved:?}");
    let s = r.state(SECRETS);
    assert_eq!(next_title(&data, &s), "Ask Una for help");
}

/// Target: in the Grelwood at "Search the Grelwood", the reader is asked
/// for the tree remark (every class's text of its key), and the remark
/// ticks the search in one live transition, so "Ask Una for help" is next.
/// Una's summon line afterwards moves nothing more, and her wait line still
/// reaches "Find the Red Vale".
#[test]
fn tree_remark_ticks_search_the_grelwood_over_the_shipped_data() {
    let data = Arc::new(data());
    let mut r = at_the_tree(&data);
    assert_eq!(
        secrets_demand(&r.engine),
        [("step-2".to_string(), ScreenWhat::Remark, TREE_KEY.to_string(), "G1_4".to_string())]
    );
    let texts = data.key_texts(TREE_KEY);
    assert_eq!(texts.len(), 8, "one text per playable class: {texts:?}");
    assert!(texts.contains(&RANGER_TREE), "{texts:?}");

    let got = r.engine.apply_screen(&ScreenEvent::remark(TREE_KEY));
    assert_eq!(
        got.outcome,
        ScreenOutcome::Ticked { quest_id: SECRETS.to_string(), step_id: "step-2".to_string() }
    );
    let moved = secrets_to(&got.transitions, "step-2");
    assert_eq!(moved.len(), 1, "{:?}", got.transitions);
    assert!(moved[0].live && moved[0].character == GRELWOOD_CHARACTER, "{:?}", moved[0]);
    let s = r.state(SECRETS);
    assert_eq!(s.current_step.as_deref(), Some("step-2"));
    assert_eq!(next_title(&data, &s), "Ask Una for help");
    assert!(!s.steps.iter().any(|st| st.ticked), "fired, not ticked: {:?}", s.steps);
    assert!(secrets_demand(&r.engine).is_empty());

    let moved = r.say(TOWN_SPOKEN, UNA_ASKS);
    assert!(secrets_to(&moved, "step-2").is_empty(), "{moved:?}");
    assert_eq!(r.state(SECRETS).current_step.as_deref(), Some("step-2"));

    let moved = r.say(TOWN_SPOKEN, UNA_WAITS);
    assert_eq!(secrets_to(&moved, "step-3").len(), 1, "{moved:?}");
    let s = r.state(SECRETS);
    assert_eq!(next_title(&data, &s), "Find the Red Vale");
}

/// The tree remark ticks nothing in the encampment while the search is
/// open, nothing once Una's line ended the search, nothing a second time,
/// and the key read as a kill remark's boss ticks nothing at all.
#[test]
fn tree_remark_ticks_nothing_outside_the_grelwood_or_open_step() {
    let data = Arc::new(data());
    let nothing = |r: &mut Replay, ev: &ScreenEvent, why: &str| {
        let got = r.engine.apply_screen(ev);
        assert_eq!(got.outcome, ScreenOutcome::NoOpenStep, "{why}");
        assert!(got.transitions.is_empty(), "{why}: {:?}", got.transitions);
    };
    let tree = ScreenEvent::remark(TREE_KEY);

    // In the encampment, the search still open.
    let mut r = at_the_tree(&data);
    r.enter(TOWN_CODE);
    assert!(secrets_demand(&r.engine).is_empty());
    nothing(&mut r, &tree, "in the encampment");
    assert_eq!(r.state(SECRETS).current_step.as_deref(), Some("step-1"));

    // In the Grelwood once Una's line ended the search.
    let mut r = at_the_tree(&data);
    r.say(TOWN_SPOKEN, UNA_ASKS);
    assert!(secrets_demand(&r.engine).is_empty());
    nothing(&mut r, &tree, "after Una's line");

    // Once it ticked, a second remark adds none.
    let mut r = at_the_tree(&data);
    assert_eq!(secrets_to(&r.engine.apply_screen(&tree).transitions, "step-2").len(), 1);
    nothing(&mut r, &tree, "a second remark");

    // The key as a kill remark is another signal.
    let mut r = at_the_tree(&data);
    nothing(&mut r, &ScreenEvent::kill_remark(TREE_KEY), "as a kill remark");
    assert_eq!(r.state(SECRETS).current_step.as_deref(), Some("step-1"));
}

// -- the screen reader's signals (Context "Design: quest data and the engine")

/// The `bar_gone`, `quest_complete` and `remark` signals the data ships, as
/// (quest, step, kind, name, area), in data order: the Miller's bar (its
/// going at the kill read in the Live 8 replay, 2026-10-28), Renly's QUEST
/// COMPLETE banner (Live 8), the Tree of Souls remark on "Search the
/// Grelwood" (poe2db's class pages, 2026-10-28; not measured live), Beira's
/// bar (its text measured in Live 6), and Blackjaw's and Lythara's by
/// analogy, their bar text not measured.
const SHIPPED_SCREEN: [(&str, &str, ScreenWhat, &str, &str); 6] = [
    ("reaching-clearfell", "step-2", ScreenWhat::BarGone, "The Bloated Miller", "G1_1"),
    ("reaching-clearfell", "step-6", ScreenWhat::QuestComplete, "Renly", "G1_town"),
    ("secrets-in-the-dark", "step-2", ScreenWhat::Remark, TREE_KEY, "G1_4"),
    ("beira-of-the-rotten-pack", "fight", ScreenWhat::BarGone, "Beira of the Rotten Pack", "G1_2"),
    ("blackjaw-the-remnant", "fight", ScreenWhat::BarGone, "Blackjaw", "G3_6_1"),
    ("lythara-the-wayward-spear", "fight", ScreenWhat::BarGone, "Lythara", "P3_2"),
];

#[test]
fn shipped_screen_signals_are_the_four_named() {
    let data = data();
    let mut found = Vec::new();
    for q in &data.quests {
        for step in &q.steps {
            for s in &step.signals {
                let (Some((what, name)), Signal::Screen { area, .. }) = (s.screen(), s) else {
                    continue;
                };
                if what == ScreenWhat::KillRemark {
                    // Pinned, with or without a bar beside them, by
                    // `shipped_kill_remark_signals_are_the_established_bosses`.
                    // A `remark` signal is pinned here.
                    continue;
                }
                found.push((q.id.as_str(), step.id.as_str(), what, name, area.as_str()));
            }
        }
    }
    assert_eq!(found, SHIPPED_SCREEN.to_vec());
    for (id, step, what, name, area) in SHIPPED_SCREEN {
        let q = data.quest(id).expect("quest");
        let title = &q.steps.iter().find(|s| s.id == step).expect("step").title;
        match what {
            // The short name is in the full bar title, and in the quest's
            // own name under a "Defeat ..." step; the Miller's step, in a
            // quest named for the encampment, names him in its own title
            // ("Slay the Bloated Miller").
            ScreenWhat::BarGone => {
                let boss = name.to_lowercase();
                let slays = title.to_lowercase().contains(&boss);
                assert!(q.name.to_lowercase().contains(&boss) || slays, "quest {id}: {name:?}");
                assert!(title.starts_with("Defeat ") || slays, "quest {id}: {title:?}");
                assert!(q.zones.iter().any(|z| z == area), "quest {id}: {area} is not its zone");
            }
            // A remark reads every class's text of its key, in a zone of the
            // quest, and was appended after the step's other signals (saved
            // progress names them by position).
            ScreenWhat::Remark => {
                assert_eq!(data.key_texts(name).len(), PLAYABLE_CLASSES.len(), "quest {id}: {name}");
                assert!(q.zones.iter().any(|z| z == area), "quest {id}: {area} is not its zone");
                let signals = &q.steps.iter().find(|s| s.id == step).expect("step").signals;
                assert_eq!(signals.last().and_then(Signal::screen), Some((what, name)), "quest {id}");
            }
            _ => assert!(title.contains(name), "quest {id}: {title:?}"),
        }
    }
    // Every kill remark text is for a boss the data knows (the validator
    // checks this; pinned here for the shipped file).
    for boss in data.remarks.iter().filter_map(|r| r.boss.as_ref()) {
        assert!(
            SHIPPED_SCREEN.iter().any(|s| s.3 == boss) || data.npcs.contains(boss),
            "remark for {boss:?}"
        );
    }
}

/// Beira's bar going, in Clearfell with her line heard, ticks "Defeat ..."
/// over the shipped data, and Renly's banner is what the encampment asks
/// the reader for before that.
#[test]
fn beira_bar_gone_ticks_the_defeat_over_the_shipped_data() {
    let data = Arc::new(data());
    let mut r = Replay::start(&data, GRELWOOD_CHARACTER);
    r.enter(TOWN_CODE);
    let demand: Vec<(String, String)> =
        r.engine.screen_demand().into_iter().map(|d| (d.quest_id, d.name)).collect();
    assert_eq!(demand, [("reaching-clearfell".to_string(), "Renly".to_string())]);

    r.enter("G1_2");
    r.say(BEIRA_SPEAKER, BEIRA_FIRST);
    let s = r.state(BEIRA);
    assert_eq!(s.current_step.as_deref(), Some("find"));
    let demand = r.engine.screen_demand();
    assert_eq!(demand.len(), 1, "{demand:?}");
    assert_eq!(
        (demand[0].quest_id.as_str(), demand[0].step_id.as_str(), demand[0].signal_id.as_str()),
        (BEIRA, "fight", "step/fight/0")
    );

    let got = r.engine.apply_screen(&ScreenEvent::bar_gone(BEIRA_SPEAKER));
    assert_eq!(
        got.outcome,
        ScreenOutcome::Ticked { quest_id: BEIRA.to_string(), step_id: "fight".to_string() }
    );
    assert!(
        got.transitions.iter().any(|t| t.quest_id == BEIRA
            && t.step_to.as_deref() == Some("fight")
            && t.live),
        "{:?}",
        got.transitions
    );
    let s = r.state(BEIRA);
    assert_eq!(s.status, Status::InProgress);
    assert_eq!(next_title(&data, &s), "Use the item it dropped");
    assert!(!s.steps.iter().any(|st| st.ticked), "fired, not ticked: {:?}", s.steps);
    assert!(r.engine.screen_demand().is_empty());
}

// -- every boss's kill remark (Context "Kill remarks of every boss") ---------

/// The `kill_remark` signals the data ships, as (quest, step, boss, area), in
/// data order: the Miller's and Blackjaw's beside their bars, and one on
/// each boss step whose kill remark poe2db's class pages establish (read
/// 2026-10-28; owner, 2026-10-28: "blackjaw and any other boss").
const SHIPPED_KILL_REMARKS: [(&str, &str, &str, &str); 21] = [
    ("reaching-clearfell", "step-2", "The Bloated Miller", "G1_1"),
    ("secrets-in-the-dark", "step-19", "The Rust King", "G1_5"),
    ("sorrow-among-stones", "step-4", "Asinia, the Praetor's Consort", "G1_9"),
    ("sorrow-among-stones", "step-9", "Draven, the Eternal Praetor", "G1_8"),
    ("sorrow-among-stones", "step-14", "Draven, the Eternal Praetor", "G1_8"),
    ("sorrow-among-stones", "step-19", "Asinia, the Praetor's Consort", "G1_9"),
    ("the-trail-of-corruption", "step-5", "The Executioner", "G1_13_2"),
    ("the-mad-wolf-of-ogham", "step-5", "Geonor, the Putrid Wolf", "G1_15"),
    ("a-theft-of-ivory", "step-4", "Iktab, the Deathlord", "G2_5_2"),
    ("kabala-constrictor-queen", "fight", "Kabala, Constrictor Queen", "G2_4_1"),
    ("legacy-of-the-vaal", "step-36", "Queen of Filth", "G3_11"),
    ("legacy-of-the-vaal", "step-42", "Ketzuli, High Priest of the Sun", "G3_12"),
    ("legacy-of-the-vaal", "step-50", "Viper Napuatzi", "G3_14"),
    ("tribal-vengeance", "step-3", "Ignagduk, the Bog Witch", "G3_7"),
    ("blackjaw-the-remnant", "fight", "Blackjaw", "G3_6_1"),
    ("whakapanu-island", "step-4", "Diamora, the Song of Death", "G4_3_2"),
    ("dark-mists", "step-8", "Captain Hartlin", "G4_2_2"),
    ("trial-of-the-ancestors", "step-12", "Yama The White", "G4_4_2"),
    ("recruit-the-ezomytes", "step-25", "Thane Wulfric", "P1_6"),
    ("recruit-the-vaal", "step-18", "Zolin, Blood Priest", "P3_7"),
    ("lythara-the-wayward-spear", "fight", "Lythara", "P3_2"),
];

/// The kill remark signals are exactly the table's. Each was appended after
/// every signal its step had, because saved progress names a step's signals
/// by position, so it is the step's last, except the Miller's (his
/// level-up follows it) and Blackjaw's (shipped beside his bar before). Every
/// boss they name has remark texts.
#[test]
fn shipped_kill_remark_signals_are_the_established_bosses() {
    let data = data();
    let mut found = Vec::new();
    for q in &data.quests {
        for step in &q.steps {
            for (i, s) in step.signals.iter().enumerate() {
                let (Some((ScreenWhat::KillRemark, boss)), Signal::Screen { area, .. }) = (s.screen(), s) else {
                    continue;
                };
                found.push((q.id.as_str(), step.id.as_str(), boss, area.as_str()));
                if boss != MILLER && boss != "Blackjaw" {
                    assert_eq!(i + 1, step.signals.len(), "quest {}: step {} not last", q.id, step.id);
                }
            }
        }
    }
    assert_eq!(found, SHIPPED_KILL_REMARKS.to_vec());
    for (quest, step, boss, _) in SHIPPED_KILL_REMARKS {
        assert!(!data.remark_texts(boss).is_empty(), "quest {quest}: step {step}: no text for {boss:?}");
    }
}

/// An own character of level 20, stored and made active by its level-up,
/// then through `areas` (code, scene name) in that order.
fn stored_through(data: &Arc<QuestData>, areas: &[(&str, &str)]) -> Replay {
    let mut r = Replay::opening(data, STORED_CHARACTER, true);
    let record = serde_json::json!({
        "name": STORED_CHARACTER, "class": "Ranger", "level": 20, "ownership": "own"
    });
    r.engine.load_character(&record).expect("a stored character");
    r.level_up(21);
    assert_eq!(r.engine.current(), Some(STORED_CHARACTER));
    for (k, (code, name)) in areas.iter().enumerate() {
        let lines = entry_lines(&mut r.clock, code, 20, 1_000 + k as u64, name);
        r.feed(&lines);
    }
    r
}

const EXECUTIONER: &str = "The Executioner";
const TRAIL: &str = "the-trail-of-corruption";
/// Up to Ogham Village, whose entry reaches "Search for the Seed of
/// Corruption", the step before the Executioner's.
const TO_OGHAM_VILLAGE: [(&str, &str); 3] =
    [("G1_11", "Hunting Grounds"), ("G1_13_1", "Ogham Farmlands"), ("G1_13_2", "Ogham Village")];
const LYTHARA: &str = "Lythara";
const KRIAR: &str = "lythara-the-wayward-spear";

/// The kill remark demanded in the current area for `quest`, by name.
fn remark_demand(engine: &Engine, quest: &str) -> Vec<String> {
    engine
        .screen_demand()
        .into_iter()
        .filter(|d| d.quest_id == quest && d.what == ScreenWhat::KillRemark)
        .map(|d| d.name)
        .collect()
}

/// One ordinary case, the Executioner's step (his line beside it, no bar),
/// and one outlier, Lythara's (her bar beside the remark): with the step
/// open in its area, the character's kill remark for the boss ticks it in
/// one live transition, the demand there names the boss's remark (whose
/// texts are every class's row), and a second remark adds none.
#[test]
fn boss_kill_remark_ticks_its_step_over_the_shipped_data() {
    let data = Arc::new(data());
    let cases: [(&str, &str, &str, &[(&str, &str)]); 2] = [
        (TRAIL, "step-5", EXECUTIONER, &TO_OGHAM_VILLAGE),
        (KRIAR, "fight", LYTHARA, &[("P3_2", "Kriar Village")]),
    ];
    for (quest, step, boss, areas) in cases {
        let mut r = stored_through(&data, areas);
        assert!(r.state(quest).steps.iter().all(|s| s.id != step || !s.fired), "{quest}");
        assert_eq!(remark_demand(&r.engine, quest), [boss.to_string()], "{quest}");
        assert_eq!(data.remark_texts(boss).len(), 8, "{boss}: one text per playable class");

        let got = r.engine.apply_screen(&ScreenEvent::kill_remark(boss));
        assert_eq!(
            got.outcome,
            ScreenOutcome::Ticked { quest_id: quest.to_string(), step_id: step.to_string() },
            "{quest}"
        );
        let moved: Vec<&Transition> = got
            .transitions
            .iter()
            .filter(|t| t.quest_id == quest && t.step_to.as_deref() == Some(step))
            .collect();
        assert_eq!(moved.len(), 1, "{:?}", got.transitions);
        assert!(moved[0].live && moved[0].character == STORED_CHARACTER);
        assert_eq!(r.state(quest).current_step.as_deref(), Some(step));
        assert!(remark_demand(&r.engine, quest).is_empty(), "{quest}");

        let again = r.engine.apply_screen(&ScreenEvent::kill_remark(boss));
        assert_eq!(again.outcome, ScreenOutcome::NoOpenStep, "{quest}");
        assert!(again.transitions.is_empty(), "{:?}", again.transitions);
    }
}

/// The same remarks tick nothing in another area while the step is open,
/// before the step is open (the quest still locked; every way into the
/// boss's area opens it), or once a step at or after it is reached: by the
/// boss's own line, by the player's tick on a later step, or by the bar.
#[test]
fn boss_kill_remark_ticks_nothing_outside_its_area_or_open_step() {
    let data = Arc::new(data());
    let nothing = |r: &mut Replay, boss: &str, why: &str| {
        let got = r.engine.apply_screen(&ScreenEvent::kill_remark(boss));
        assert_eq!(got.outcome, ScreenOutcome::NoOpenStep, "{boss}: {why}");
        assert!(got.transitions.is_empty(), "{boss}: {why}: {:?}", got.transitions);
    };

    // Before the step is open: the quests still locked, in the encampment.
    let mut r = stored_through(&data, &[("G1_town", "Clearfell Encampment")]);
    assert_eq!(r.state(TRAIL).status, Status::Locked);
    assert_eq!(r.state(KRIAR).status, Status::Locked);
    nothing(&mut r, EXECUTIONER, "locked");
    nothing(&mut r, LYTHARA, "locked");

    // In another area with the step open: the Executioner's from Ogham
    // Farmlands, Lythara's from the Ashen Forest after Kriar Village.
    let mut r = stored_through(&data, &TO_OGHAM_VILLAGE[..2]);
    assert_eq!(r.state(TRAIL).current_step.as_deref(), Some("step-3"));
    nothing(&mut r, EXECUTIONER, "Ogham Farmlands");
    let mut r = stored_through(&data, &[("P3_2", "Kriar Village"), ("P3_1", "Ashen Forest")]);
    assert_eq!(r.state(KRIAR).current_step.as_deref(), Some("enter"));
    nothing(&mut r, LYTHARA, "Ashen Forest");

    // In the area, once the step is reached by the boss's line.
    let mut r = stored_through(&data, &TO_OGHAM_VILLAGE);
    r.say(EXECUTIONER, "Fixture line: the rope awaits.");
    assert_eq!(r.state(TRAIL).current_step.as_deref(), Some("step-5"));
    nothing(&mut r, EXECUTIONER, "his line reached it");

    // In the area, once the player ticked a later step by hand.
    let mut r = stored_through(&data, &TO_OGHAM_VILLAGE);
    r.engine.tick_step(STORED_CHARACTER, TRAIL, "step-6", true).expect("tick");
    assert_eq!(r.state(TRAIL).current_step.as_deref(), Some("step-6"));
    nothing(&mut r, EXECUTIONER, "a later step ticked");

    // In the area, once her bar's going reached her step.
    let mut r = stored_through(&data, &[("P3_2", "Kriar Village")]);
    let got = r.engine.apply_screen(&ScreenEvent::bar_gone(LYTHARA));
    assert_eq!(got.outcome, ScreenOutcome::Ticked { quest_id: KRIAR.into(), step_id: "fight".into() });
    nothing(&mut r, LYTHARA, "her bar reached it");
}
