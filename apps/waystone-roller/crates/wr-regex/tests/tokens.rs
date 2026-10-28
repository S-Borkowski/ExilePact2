//! The token picker: each family's own token is the shortest text that only
//! its own lines hold, never crossing a number or punctuation, at least three
//! characters, and the same on every run.

mod common;

use serde_json::Value;
use wr_regex::model::highlights;
use wr_regex::{candidates, frame_lines, ModData, Tokens, ITEM_FRAME_LINES, MIN_TOKEN_LEN};

const CONTAINED: &str = "area-contains-additional-murk-pools";
const CONTAINER: &str = "area-is-overrun-by-the-murkfolk";

#[test]
fn tokens_are_unique_against_every_other_line_and_the_frame_target() {
    let data = common::fixture_data();
    let tokens = Tokens::pick(&data);
    let frame = frame_lines(&data);
    // The desecrated affix names are part of the frame, beside the constant.
    assert!(frame.iter().any(|l| l == "of whispers"));
    assert!(frame.iter().any(|l| l == "gloaming"));
    assert!(ITEM_FRAME_LINES.contains(&"Corrupted"));

    let items = common::all_items(&data);
    let mut with_token = 0;
    for t in tokens.all() {
        let Some(token) = &t.token else { continue };
        with_token += 1;
        assert_eq!(token, &token.to_lowercase());
        for other in data.families.iter().filter(|f| f.id != t.id) {
            for line in &other.lines {
                assert!(
                    !line.to_lowercase().contains(token.as_str()),
                    "{}'s token {token:?} is in {}'s line {line:?}",
                    t.id,
                    other.id
                );
            }
        }
        for line in &frame {
            assert!(
                !line.contains(token.as_str()),
                "{token:?} is in frame line {line:?}"
            );
        }
        // Through the search model: the token alone highlights exactly the
        // items that carry its family.
        let search = if token.contains(' ') {
            format!("\"{token}\"")
        } else {
            token.clone()
        };
        for item in &items {
            assert_eq!(
                highlights(&search, &item.text).unwrap(),
                item.mods.contains(&t.id),
                "token {search:?} of {} on {}",
                t.id,
                item.name
            );
        }
    }
    // Every family but the contained one has a token.
    assert_eq!(with_token, data.families.len() - 1);
    assert!(tokens.get(CONTAINED).unwrap().token.is_none());

    // Negative control: a family whose every line another family also has,
    // word for word, gets none.
    let mut doc: Value = serde_json::from_str(&common::fixture_json()).unwrap();
    doc["families"][10]["lines"] = serde_json::json!([
        "Players are cursed with Drowsiness",
        "Monsters have Hopping Speed"
    ]);
    doc["families"][10]["bonus"] = serde_json::json!([false, false]);
    doc["families"][10]["id"] = "players-are-cursed-with-drowsiness-and-hop".into();
    doc["families"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "id": "players-are-cursed-with-drowsiness", "group": "waystone", "kind": "suffix",
            "name": null, "lines": ["Players are cursed with Drowsiness"], "bonus": [false], "tiers": 1
        }));
    let data = ModData::parse(&doc.to_string()).unwrap();
    let tokens = Tokens::pick(&data);
    assert!(tokens
        .get("players-are-cursed-with-drowsiness")
        .unwrap()
        .token
        .is_none());
}

#[test]
fn tokens_never_span_a_number_or_punctuation() {
    let c = candidates("Monsters' Skills Chain # additional times");
    assert!(c.iter().any(|t| t == "skills chain"));
    assert!(c.iter().any(|t| t == "additional times"));
    for bad in [
        "s' s",
        "rs s",
        "chain additional",
        "n # a",
        "n  a",
        "chain  ",
    ] {
        assert!(!c.iter().any(|t| t == bad), "{bad:?} crosses a separator");
    }
    let c = candidates("Players have #% less Recovery Rate of Life, Mana and Glimmer");
    assert!(c.iter().any(|t| t == "mana and glimmer"));
    assert!(!c
        .iter()
        .any(|t| t.contains("life mana") || t.contains("e m")));
    let c = candidates("+#% Monster Spectral Resistances");
    assert!(c.iter().any(|t| t == "monster spectral resistances"));
    for t in candidates("Monsters deal 25% of Damage, as (Extra) Glimmer") {
        assert!(
            t.chars().all(|ch| ch.is_ascii_lowercase() || ch == ' '),
            "{t:?} holds a character outside [a-z ]"
        );
        assert!(
            !t.starts_with(' ') && !t.ends_with(' '),
            "{t:?} has an edge space"
        );
    }
    // Every picked token of the fixture is [a-z ] with letters at both ends.
    let data = common::fixture_data();
    for t in Tokens::pick(&data).all() {
        if let Some(token) = &t.token {
            let b = token.as_bytes();
            assert!(
                b.iter().all(|c| c.is_ascii_lowercase() || *c == b' '),
                "{token:?}"
            );
            assert!(b[0] != b' ' && b[b.len() - 1] != b' ', "{token:?}");
        }
    }
}

#[test]
fn tokens_have_minimum_length_three() {
    assert_eq!(MIN_TOKEN_LEN, 3);
    let data = common::fixture_data();
    for t in Tokens::pick(&data).all() {
        if let Some(token) = &t.token {
            assert!(token.len() >= MIN_TOKEN_LEN, "{}: {token:?}", t.id);
        }
    }
    for line in [
        "Monster have #% chance to Bleat",
        "Area contains # additional Murk Pools",
    ] {
        assert!(candidates(line).iter().all(|t| t.len() >= 3));
        assert!(candidates(line).iter().any(|t| t.len() == 3));
    }
    // A line of two-letter words between numbers yields nothing, so its
    // family is unmatchable, with a reason; one three-letter word is enough
    // (positive control).
    assert!(candidates("Qx #% Zy +# Jq").is_empty());
    assert_eq!(candidates("Qx #% Zyw"), ["zyw"]);
    let mut doc: Value = serde_json::from_str(&common::fixture_json()).unwrap();
    doc["families"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "id": "qx-zy", "group": "waystone", "kind": "prefix", "name": null,
            "lines": ["Qx #% Zy"], "bonus": [false], "tiers": 1
        }));
    let data = ModData::parse(&doc.to_string()).unwrap();
    let tokens = Tokens::pick(&data);
    let short = tokens.get("qx-zy").unwrap();
    assert!(short.token.is_none());
    assert!(!short.why_not.as_deref().unwrap_or("").is_empty());
}

#[test]
fn tokens_report_a_family_contained_in_another() {
    let data = common::fixture_data();
    let tokens = Tokens::pick(&data);
    let contained = tokens.get(CONTAINED).unwrap();
    assert!(contained.token.is_none());
    let why = contained.why_not.as_deref().expect("a reason");
    assert!(
        why.contains(CONTAINER),
        "{why:?} does not name the container"
    );
    // The container itself keeps a token, taken from its first line, and
    // every family with a token has no reason.
    let container = tokens.get(CONTAINER).unwrap();
    let token = container.token.as_deref().expect("container token");
    assert!(
        "area is overrun by the murkfolk".contains(token),
        "{token:?}"
    );
    for t in tokens.all() {
        assert_eq!(t.token.is_some(), t.why_not.is_none(), "{}", t.id);
    }
    // A family with only reward lines has nothing of its own.
    let mut doc: Value = serde_json::from_str(&common::fixture_json()).unwrap();
    doc["families"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "id": "more-pack-size", "group": "waystone", "kind": "prefix", "name": null,
            "lines": ["#% more Pack size", "Monsters have #% more Effectiveness"],
            "bonus": [true, true], "tiers": 1
        }));
    let data = ModData::parse(&doc.to_string()).unwrap();
    let t = Tokens::pick(&data);
    let only_bonus = t.get("more-pack-size").unwrap();
    assert!(only_bonus.token.is_none() && only_bonus.why_not.is_some());
}

#[test]
fn tokens_are_deterministic() {
    let data = common::fixture_data();
    let a = Tokens::pick(&data);
    let b = Tokens::pick(&data);
    assert_eq!(a.all(), b.all());
    // The order of families in the file changes nothing.
    let mut doc: Value = serde_json::from_str(&common::fixture_json()).unwrap();
    doc["families"].as_array_mut().unwrap().reverse();
    let reversed = Tokens::pick(&ModData::parse(&doc.to_string()).unwrap());
    for t in a.all() {
        assert_eq!(reversed.get(&t.id).unwrap(), t);
    }
    // Shortest first, then alphabetical.
    assert_eq!(candidates("Bbb, Aaa"), ["aaa", "bbb"]);
    let c = candidates("Monsters deal #% of Damage as Extra Murk");
    assert!(c
        .windows(2)
        .all(|w| (w[0].len(), &w[0]) < (w[1].len(), &w[1])));
}
