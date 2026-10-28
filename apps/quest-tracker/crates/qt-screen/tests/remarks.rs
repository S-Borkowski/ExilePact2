//! The shipped kill remarks (the `remarks` table of `data/quests.json`)
//! through the screen reader's [`RemarkTracker`], built the way the reader
//! builds it: one tracker per boss, with every class's texts for that boss.
//!
//! Target: every shipped remark text, as a new `<name>: <text>` line in the
//! list of voiced lines, fires one `kill_remark` for its boss. Negative
//! controls: each playable class's on-sight lines for the Miller, the
//! Executioner and Lythara, which play when the fight starts, fire nothing
//! through any shipped boss's tracker.
//!
//! The rows with no boss are a key's: every class's Tree of Souls remark,
//! as a new line, fires one `remark` for its key, and the Rust King's kill
//! remarks do not. Every key the data ships a `remark` signal for (the
//! remark ledger's attachments) is read the same way, area by area: each
//! of its texts fires its own key and no other key or boss asked for in the
//! same area.
//!
//! The on-sight lines are short user-facing game text from poe2db's class
//! pages, kept with the page and the date they were read (AGENTS.md §
//! "Legal", the owner's decision of 2026-10-24), braces markup removed. The
//! character name is invented.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use qt_screen::{Line, Observation, Rect, Region, RemarkTracker, ScreenEvent};
use serde_json::Value;

const MILLER: &str = "The Bloated Miller";

/// The eight playable classes (poe2db's Ascendancy page, 2026-10-28).
const PLAYABLE_CLASSES: [&str; 8] =
    ["Warrior", "Mercenary", "Ranger", "Huntress", "Witch", "Sorceress", "Monk", "Druid"];

/// Each class's `MillerSeenComment`: (class, text, page, date read).
const MILLER_SEEN: [(&str, &str, &str, &str); 8] = [
    ("Warrior", "Branoc, is that you?", "https://poe2db.tw/us/Warrior", "2026-10-27"),
    ("Mercenary", "Ohhh no...", "https://poe2db.tw/us/Mercenary", "2026-10-27"),
    ("Ranger", "The journey has not been kind to you, Miller.", "https://poe2db.tw/us/Ranger", "2026-10-28"),
    ("Huntress", "So, this is the 'sickness' I were warned about.", "https://poe2db.tw/us/Huntress", "2026-10-28"),
    ("Witch", "The Miller I presume?", "https://poe2db.tw/us/Witch", "2026-10-28"),
    ("Sorceress", "A sickness controls him!", "https://poe2db.tw/us/Sorceress", "2026-10-28"),
    ("Monk", "The Miller has seen better days.", "https://poe2db.tw/us/Monk", "2026-10-28"),
    ("Druid", "Talk's wasted on ye, ye bloody lump!", "https://poe2db.tw/us/Druid", "2026-10-28"),
];

/// Each class's `ExecutionerSeenComment` (the Executioner's on-sight line):
/// (class, text, page, date read).
const EXECUTIONER_SEEN: [(&str, &str, &str, &str); 8] = [
    ("Warrior", "You! Bastard! Face me!", "https://poe2db.tw/us/Warrior", "2026-10-27"),
    ("Mercenary", "This... is personal!", "https://poe2db.tw/us/Mercenary", "2026-10-27"),
    ("Ranger", "That execution will be your last.", "https://poe2db.tw/us/Ranger", "2026-10-28"),
    ("Huntress", "You had your chance to kill me. Now it's my turn!", "https://poe2db.tw/us/Huntress", "2026-10-28"),
    ("Witch", "My would-be executioner.", "https://poe2db.tw/us/Witch", "2026-10-28"),
    ("Sorceress", "The Maraketh do not forgive. Nor do they forget.", "https://poe2db.tw/us/Sorceress", "2026-10-28"),
    ("Monk", "Ogham deserves one last execution.", "https://poe2db.tw/us/Monk", "2026-10-28"),
    ("Druid", "You! You've much to answer for!", "https://poe2db.tw/us/Druid", "2026-10-28"),
];

/// Each class's `KriarViperSeen` (Lythara's on-sight line): (class, text,
/// page, date read).
const LYTHARA_SEEN: [(&str, &str, &str, &str); 8] = [
    ("Warrior", "There's no honour in what you've become!", "https://poe2db.tw/us/Warrior", "2026-10-27"),
    ("Mercenary", "Are all Azmeri insane, or is it just you?", "https://poe2db.tw/us/Mercenary", "2026-10-27"),
    ("Ranger", "You turned on your own. That I can't forgive!", "https://poe2db.tw/us/Ranger", "2026-10-28"),
    ("Huntress", "The Spirit would never bid you kill your own!", "https://poe2db.tw/us/Huntress", "2026-10-28"),
    ("Witch", "The Azmerian is out of her mind. Fun!", "https://poe2db.tw/us/Witch", "2026-10-28"),
    ("Sorceress", "You are truly insane!", "https://poe2db.tw/us/Sorceress", "2026-10-28"),
    ("Monk", "You shall not blunt my resolve!", "https://poe2db.tw/us/Monk", "2026-10-28"),
    ("Druid", "Whatever speaks to ye, it's no Spirit!", "https://poe2db.tw/us/Druid", "2026-10-28"),
];

/// One shipped kill-remark row (a `remarks` row naming a boss).
struct Remark {
    class: String,
    boss: String,
    text: String,
}

/// The shipped `remarks` rows, as JSON.
fn shipped_rows() -> Vec<Value> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/quests.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let data: Value = serde_json::from_str(&text).expect("the quest data is JSON");
    data["remarks"].as_array().expect("a remarks table").clone()
}

fn field(r: &Value, k: &str) -> String {
    r[k].as_str().unwrap_or_else(|| panic!("remark {r}: {k}")).to_string()
}

/// The shipped kill-remark rows; a row with no boss is a key's
/// ([`key_remarks`]).
fn shipped_remarks() -> Vec<Remark> {
    shipped_rows()
        .iter()
        .filter(|r| r.get("boss").is_some())
        .map(|r| Remark { class: field(r, "class"), boss: field(r, "boss"), text: field(r, "text") })
        .collect()
}

/// The shipped rows of `key`, as (class, text, page, date read).
fn key_remarks(key: &str) -> Vec<(String, String, String, String)> {
    shipped_rows()
        .iter()
        .filter(|r| r["key"] == key)
        .map(|r| (field(r, "class"), field(r, "text"), field(r, "source_url"), field(r, "fetched")))
        .collect()
}

/// Every boss the table names, in table order, each once.
fn bosses(rows: &[Remark]) -> Vec<&str> {
    let mut out: Vec<&str> = Vec::new();
    for r in rows {
        if !out.contains(&r.boss.as_str()) {
            out.push(&r.boss);
        }
    }
    out
}

/// The tracker the reader builds for `boss`: every class's texts for it.
fn tracker_for(rows: &[Remark], boss: &str) -> RemarkTracker {
    let texts = rows.iter().filter(|r| r.boss == boss).map(|r| r.text.clone()).collect();
    RemarkTracker::new(boss, texts)
}

/// The list's newest line, and an older one above it.
fn newest(text: &str) -> Line {
    Line::new(text, Rect::new(0.03, 0.71, 0.30, 0.73))
}
fn older(text: &str) -> Line {
    Line::new(text, Rect::new(0.03, 0.69, 0.30, 0.71))
}

/// Reads at 2 Hz from `start` ms until before `end`, each with `lines`.
fn reads(start: i64, end: i64, lines: Vec<Line>) -> Vec<Observation> {
    (start..end)
        .step_by(500)
        .map(|at| Observation::visible(Region::VoicedLines, at, lines.clone()))
        .collect()
}

/// What `tracker` reports when the character says `text`: 3 s of reads with
/// an older shout only, then 6 s with `<name>: <text>` as the newest line.
fn said(tracker: &mut RemarkTracker, text: &str) -> Vec<ScreenEvent> {
    let shout = older("Kestrella: For the encampment!");
    let mut obs = reads(0, 3_000, vec![shout.clone()]);
    obs.extend(reads(3_000, 9_000, vec![shout, newest(&format!("Kestrella: {text}"))]));
    obs.iter().filter_map(|o| tracker.observe(o)).collect()
}

#[test]
fn every_shipped_miller_remark_is_read_as_his_kill_remark() {
    let rows = shipped_remarks();
    let miller: Vec<&Remark> = rows.iter().filter(|r| r.boss == MILLER).collect();
    let mut classes: Vec<&str> = miller.iter().map(|r| r.class.as_str()).collect();
    classes.sort_unstable();
    let mut want = PLAYABLE_CLASSES.to_vec();
    want.sort_unstable();
    assert_eq!(classes, want, "one Miller remark per playable class");
    for r in miller {
        let mut t = tracker_for(&rows, MILLER);
        assert_eq!(
            said(&mut t, &r.text),
            [ScreenEvent::KillRemark { boss: MILLER.to_string(), at: 3_000 }],
            "{}: {:?}",
            r.class,
            r.text
        );
    }
}

#[test]
fn no_class_miller_seen_comment_is_read_as_a_kill_remark() {
    let rows = shipped_remarks();
    for (class, text, page, _) in MILLER_SEEN {
        assert_eq!(page, format!("https://poe2db.tw/us/{class}"));
        let mut t = tracker_for(&rows, MILLER);
        assert!(said(&mut t, text).is_empty(), "{class}: {text:?}");
    }
    // Positive control through the same reads: a shipped text fires.
    let mut t = tracker_for(&rows, MILLER);
    let text = rows.iter().find(|r| r.boss == MILLER).expect("a Miller remark").text.clone();
    assert_eq!(said(&mut t, &text).len(), 1);
}

#[test]
fn every_shipped_kill_remark_is_read_for_its_boss() {
    let rows = shipped_remarks();
    let bosses = bosses(&rows);
    // The Miller, Blackjaw and the seventeen other bosses whose remark the
    // class pages establish (Context "Kill remarks of every boss").
    assert_eq!(bosses.len(), 19, "{bosses:?}");
    for r in &rows {
        let mut t = tracker_for(&rows, &r.boss);
        assert_eq!(
            said(&mut t, &r.text),
            [ScreenEvent::KillRemark { boss: r.boss.clone(), at: 3_000 }],
            "{} / {}: {:?}",
            r.boss,
            r.class,
            r.text
        );
    }
}

#[test]
fn no_class_on_sight_line_is_read_as_a_kill_remark() {
    let rows = shipped_remarks();
    let bosses = bosses(&rows);
    for (class, text, page, _) in EXECUTIONER_SEEN.into_iter().chain(LYTHARA_SEEN) {
        assert_eq!(page, format!("https://poe2db.tw/us/{class}"));
        for boss in &bosses {
            let mut t = tracker_for(&rows, boss);
            assert!(said(&mut t, text).is_empty(), "{class}: {text:?} read as {boss}'s remark");
        }
    }
    // Positive control through the same reads: each boss's own texts fire.
    for boss in &bosses {
        let mut t = tracker_for(&rows, boss);
        let text = rows.iter().find(|r| r.boss == *boss).expect("a row").text.clone();
        assert_eq!(said(&mut t, &text).len(), 1, "{boss}");
    }
}

/// Every class's remark on finding the Tree of Souls (poe2db's
/// `SinTreeSeenComment`, its rows in `remarks` with the page and the date
/// read), which ends "Search the Grelwood".
const TREE_KEY: &str = "SinTreeSeenComment";
const RUST_KING: &str = "The Rust King";

/// The tracker the reader builds for `key`: every class's texts of it.
fn tracker_for_key(key: &str) -> RemarkTracker {
    let texts = key_remarks(key).into_iter().map(|(_, text, _, _)| text).collect();
    RemarkTracker::for_key(key, texts)
}

#[test]
fn every_shipped_tree_remark_is_read_as_the_tree_remark() {
    let tree = key_remarks(TREE_KEY);
    let mut classes: Vec<&str> = tree.iter().map(|(class, ..)| class.as_str()).collect();
    classes.sort_unstable();
    let mut want = PLAYABLE_CLASSES.to_vec();
    want.sort_unstable();
    assert_eq!(classes, want, "one tree remark per playable class");
    for (class, text, page, fetched) in &tree {
        assert_eq!(*page, format!("https://poe2db.tw/us/{class}"));
        assert!(fetched.as_str() >= "2026-10-27", "{class}: {fetched}");
        let mut t = tracker_for_key(TREE_KEY);
        assert_eq!(
            said(&mut t, text),
            [ScreenEvent::Remark { key: TREE_KEY.to_string(), at: 3_000 }],
            "{class}: {text:?}"
        );
    }
    // Negative control: the Rust King's kill remarks, the next remark of
    // the quest, are not the tree's.
    let rows = shipped_remarks();
    let king: Vec<&Remark> = rows.iter().filter(|r| r.boss == RUST_KING).collect();
    assert_eq!(king.len(), PLAYABLE_CLASSES.len(), "one Rust King remark per playable class");
    for r in king {
        let mut t = tracker_for_key(TREE_KEY);
        assert!(said(&mut t, &r.text).is_empty(), "{}: {:?} read as the tree remark", r.class, r.text);
        // Positive control through the same reads: his own tracker fires.
        let mut t = tracker_for(&rows, RUST_KING);
        assert_eq!(said(&mut t, &r.text).len(), 1, "{}", r.class);
    }
}

/// What the shipped steps ask the reader for in each area: the keys of
/// their `remark` signals and the bosses of their `kill_remark` signals,
/// each once, in data order.
#[derive(Default)]
struct AreaDemand {
    keys: Vec<String>,
    bosses: Vec<String>,
}

/// A key's attachment: the (quest, step) pairs its `remark` signals sit on.
type Attachment = BTreeSet<(String, String)>;

/// The shipped demand per area, and each remark key's attachment.
fn shipped_demand_by_area() -> (BTreeMap<String, AreaDemand>, BTreeMap<String, Attachment>) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/quests.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let data: Value = serde_json::from_str(&text).expect("the quest data is JSON");
    let mut out: BTreeMap<String, AreaDemand> = BTreeMap::new();
    let mut attached: BTreeMap<String, Attachment> = BTreeMap::new();
    for q in data["quests"].as_array().expect("quests") {
        for step in q["steps"].as_array().expect("steps") {
            for s in step["signals"].as_array().expect("signals") {
                if s["kind"] != "screen" {
                    continue;
                }
                let (list, name) = match s["what"].as_str() {
                    Some("remark") => (true, field(s, "key")),
                    Some("kill_remark") => (false, field(s, "boss")),
                    _ => continue,
                };
                if list {
                    attached.entry(name.clone()).or_default().insert((field(q, "id"), field(step, "id")));
                }
                let demand = out.entry(field(s, "area")).or_default();
                let names = if list { &mut demand.keys } else { &mut demand.bosses };
                if !names.contains(&name) {
                    names.push(name);
                }
            }
        }
    }
    (out, attached)
}

/// Every remark key the data ships (the remark ledger's attachments), per
/// area: each class's text of the key, as a new line, fires one `remark`
/// for that key through the tracker the reader builds for it, and nothing
/// through the tracker of any other key, or of any boss's kill remark,
/// demanded in the same area. The one exception is a key attached to the
/// very same steps (the ledger's text rule keeps a text two such keys
/// share, as both end the same step): its tracker may fire too, and ticks
/// nothing more. The rows are poe2db's class pages' (each row's
/// `source_url` and `fetched`, 2026-10-27 and 2026-10-28).
#[test]
fn every_shipped_remark_is_read_for_its_key_only() {
    let rows = shipped_remarks();
    let (by_area, attached) = shipped_demand_by_area();
    let mut keys_read = 0;
    let mut shared = 0;
    for (area, demand) in &by_area {
        for key in &demand.keys {
            let texts = key_remarks(key);
            assert!(!texts.is_empty(), "{area}: {key}: no text");
            keys_read += 1;
            for (class, text, page, fetched) in &texts {
                assert_eq!(*page, format!("https://poe2db.tw/us/{class}"), "{key}");
                assert!(fetched.as_str() >= "2026-10-27", "{key}: {class}: {fetched}");
                let mut t = tracker_for_key(key);
                assert_eq!(
                    said(&mut t, text),
                    [ScreenEvent::Remark { key: key.clone(), at: 3_000 }],
                    "{area}: {key} / {class}: {text:?}"
                );
                for other in demand.keys.iter().filter(|k| *k != key) {
                    let mut t = tracker_for_key(other);
                    if attached[other] == attached[key] {
                        shared += said(&mut t, text).len();
                        continue;
                    }
                    assert!(said(&mut t, text).is_empty(), "{area}: {key} / {class}: {text:?} read as {other}");
                }
                for boss in &demand.bosses {
                    let mut t = tracker_for(&rows, boss);
                    assert!(said(&mut t, text).is_empty(), "{area}: {key} / {class}: {text:?} read as {boss}'s kill remark");
                }
            }
        }
    }
    // The tree's key is among them, in the Grelwood.
    assert!(by_area.get("G1_4").is_some_and(|d| d.keys.iter().any(|k| k == TREE_KEY)));
    assert!(keys_read > 1, "{keys_read} keys read");
    // Counted, not asserted: the shared texts of same-step keys.
    println!("{keys_read} keys read; {shared} reads by a same-step key");
}
