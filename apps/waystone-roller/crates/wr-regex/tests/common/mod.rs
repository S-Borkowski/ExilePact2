//! Shared test support: the fixture mod set, the fixture items and their
//! index, items synthesised from a family's lines, and the oracle that says
//! from an item's index entry (never from its text) whether a selection
//! should highlight it.

#![allow(dead_code)]

use std::path::PathBuf;

use serde_json::Value;
use wr_regex::model::Search;
use wr_regex::{Builder, Corruption, Family, ModData, Outcome, Selection, WantMode};

pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

pub fn fixture_json() -> String {
    std::fs::read_to_string(fixtures_dir().join("mods_min.json")).expect("mods_min.json")
}

pub fn fixture_data() -> ModData {
    ModData::load(&fixtures_dir().join("mods_min.json")).expect("mods_min.json loads")
}

pub fn fixture_builder() -> Builder {
    Builder::new(fixture_data())
}

/// One waystone and what it is.
#[derive(Debug, Clone)]
pub struct Item {
    pub name: String,
    pub text: String,
    pub rarity: String,
    pub tier: u8,
    pub corrupted: bool,
    pub mods: Vec<String>,
}

/// Replaces every `#` of a template line with a number and its roll range,
/// as the advanced item text shows it (`14(10-14)`; Live 1). The ranges end
/// in 10 to 16, so a tier term that a range can satisfy lights the wrong
/// tier in these items.
pub fn fill(line: &str, seed: usize) -> String {
    let mut out = String::new();
    let mut n = seed;
    for ch in line.chars() {
        if ch == '#' {
            let hi = 10 + n % 7;
            let lo = hi - 4;
            out.push_str(&format!("{}({lo}-{hi})", lo + n % 5));
            n += 3;
        } else {
            out.push(ch);
        }
    }
    out
}

/// The affix header the advanced item text puts above a mod's lines. A
/// regular mod's affix name is not in the data, so an invented one stands in.
pub fn header(f: &Family) -> String {
    let kind = match f.kind {
        wr_regex::Kind::Prefix => "Prefix",
        wr_regex::Kind::Suffix => "Suffix",
    };
    let name = f.name.as_deref().unwrap_or(match f.kind {
        wr_regex::Kind::Prefix => "Qorvex",
        wr_regex::Kind::Suffix => "of Qorvex",
    });
    format!("{{ {kind} Modifier \"{name}\" (Tier: 1) }}")
}

/// An item in the measured layout (wr-regex `ITEM_FRAME_LINES`, Live 1)
/// carrying `families`' lines under their headers, reward lines included
/// when `with_bonus` (the game showed none; kept as the conservative case).
pub fn synth_item(
    families: &[&Family],
    rarity: &str,
    tier: u8,
    corrupted: bool,
    with_bonus: bool,
) -> Item {
    let rarity_word = match rarity {
        "normal" => "Normal",
        "magic" => "Magic",
        "rare" => "Rare",
        other => panic!("rarity {other}"),
    };
    let mut text = format!("Item Class: Waystones\nRarity: {rarity_word}\n");
    if rarity == "rare" {
        text.push_str("Dusk Hollow\n");
    }
    text.push_str(&format!(
        "Waystone (Tier {tier})\n--------\nRevives Available: 2 (augmented)\nItem Rarity: +20% (augmented)\nPack Size: +5% (augmented)\nMonster Rarity: +18% (augmented)\n--------\nItem Level: {}\n",
        64 + tier as usize
    ));
    if !families.is_empty() {
        text.push_str("--------\n");
    }
    for (i, f) in families.iter().enumerate() {
        text.push_str(&header(f));
        text.push('\n');
        for (j, line) in f.lines.iter().enumerate() {
            if f.bonus[j] && !with_bonus {
                continue;
            }
            text.push_str(&fill(line, i * 5 + j));
            text.push('\n');
        }
    }
    text.push_str("--------\nCan be used in a Map Device, allowing you to enter a Map. Waystones can only be used once.\n");
    if corrupted {
        text.push_str("--------\nCorrupted\n");
    }
    Item {
        name: format!(
            "synth[{}] {rarity} t{tier}{}",
            families
                .iter()
                .map(|f| f.id.as_str())
                .collect::<Vec<_>>()
                .join(","),
            if corrupted { " corrupted" } else { "" }
        ),
        text,
        rarity: rarity.to_string(),
        tier,
        corrupted,
        mods: families.iter().map(|f| f.id.clone()).collect(),
    }
}

/// The fixture items under `fixtures/items/`, read through `items.json`.
/// The text is committed with LF; a Windows checkout with `core.autocrlf`
/// writes it with CRLF, so it is read back as LF either way.
pub fn fixture_items() -> Vec<Item> {
    let dir = fixtures_dir().join("items");
    let index: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("items.json")).unwrap()).unwrap();
    index["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            let file = e["file"].as_str().unwrap();
            Item {
                name: file.to_string(),
                text: std::fs::read_to_string(dir.join(file))
                    .unwrap()
                    .replace("\r\n", "\n"),
                rarity: e["rarity"].as_str().unwrap().to_string(),
                tier: e["tier"].as_u64().unwrap() as u8,
                corrupted: e["corrupted"].as_bool().unwrap(),
                mods: e["mods"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|m| m.as_str().unwrap().to_string())
                    .collect(),
            }
        })
        .collect()
}

/// The fixture items plus, for every family, one rare item with its reward
/// lines and one corrupted magic item without them, and two bare items.
pub fn all_items(data: &ModData) -> Vec<Item> {
    let mut items = fixture_items();
    for (i, f) in data.families.iter().enumerate() {
        let tier = 1 + (i % 16) as u8;
        items.push(synth_item(&[f], "rare", tier, false, true));
        items.push(synth_item(&[f], "magic", 17 - tier, true, false));
    }
    items.push(synth_item(&[], "normal", 7, false, false));
    items.push(synth_item(&[], "rare", 12, true, false));
    items
}

/// Whether `selection` should highlight `item`, from what the item is.
pub fn expected(selection: &Selection, item: &Item) -> bool {
    let r = &selection.rarity;
    let picked = [r.normal, r.magic, r.rare];
    let rarity_ok = picked.iter().all(|p| *p)
        || picked.iter().all(|p| !*p)
        || match item.rarity.as_str() {
            "normal" => r.normal,
            "magic" => r.magic,
            "rare" => r.rare,
            _ => false,
        };
    let tier_ok = selection.tier_min <= item.tier && item.tier <= selection.tier_max;
    let has = |id: &String| item.mods.contains(id);
    let want_ok = selection.want.is_empty()
        || match selection.want_mode {
            WantMode::Any => selection.want.iter().any(has),
            WantMode::All => selection.want.iter().all(has),
        };
    let avoid_ok = !selection.avoid.iter().any(has);
    let corruption_ok = match selection.corruption {
        Corruption::Any => true,
        Corruption::Corrupted => item.corrupted,
        Corruption::Uncorrupted => !item.corrupted,
    };
    rarity_ok && tier_ok && want_ok && avoid_ok && corruption_ok
}

/// Builds `selection`, expects `Ok`, and runs the string through the search
/// model over every item; returns the string.
pub fn check(builder: &Builder, selection: &Selection, items: &[Item]) -> String {
    let regex = match builder.build(selection) {
        Outcome::Ok { regex, parts } => {
            let joined: Vec<&str> = parts.iter().map(|p| p.text.as_str()).collect();
            assert_eq!(
                regex,
                joined.join(" "),
                "parts joined differ from the string"
            );
            for p in &parts {
                assert_eq!(p.length, p.text.chars().count());
            }
            assert!(regex.chars().count() <= wr_regex::LIMIT);
            regex
        }
        other => panic!("{selection:?} gave {other:?}"),
    };
    let search = Search::parse(&regex).unwrap_or_else(|e| panic!("model refused {regex:?}: {e}"));
    for item in items {
        let got = search.highlights(&item.text);
        assert_eq!(
            got,
            expected(selection, item),
            "search {regex:?} on item {} (mods {:?})",
            item.name,
            item.mods
        );
    }
    regex
}

/// Ids of every family that has a token of its own.
pub fn matchable_ids(builder: &Builder) -> Vec<String> {
    builder
        .tokens()
        .all()
        .iter()
        .filter(|t| t.token.is_some())
        .map(|t| t.id.clone())
        .collect()
}

/// Every subset of `ids` with `min..=max` members, in a stable order.
pub fn subsets(ids: &[String], min: usize, max: usize) -> Vec<Vec<String>> {
    let mut out = Vec::new();
    let n = ids.len();
    for mask in 0u32..(1u32 << n) {
        let k = mask.count_ones() as usize;
        if k >= min && k <= max {
            out.push(
                (0..n)
                    .filter(|i| mask & (1 << i) != 0)
                    .map(|i| ids[i].clone())
                    .collect(),
            );
        }
    }
    out
}

/// A mod data file of `n` invented families whose only line is `#% <w>`, `w`
/// a three-letter word no other line or frame line holds, so each family's
/// token is exactly its word.
pub fn three_letter_data(n: usize) -> (ModData, Vec<String>) {
    let firsts = ["qx", "qz", "xq", "zq", "jq", "qj", "xz", "zx"];
    let mut words = Vec::new();
    'outer: for f in firsts {
        for c in 'a'..='z' {
            words.push(format!("{f}{c}"));
            if words.len() == n {
                break 'outer;
            }
        }
    }
    assert_eq!(words.len(), n, "not enough invented words");
    let families: Vec<Value> = words
        .iter()
        .map(|w| {
            serde_json::json!({
                "id": format!("fam-{w}"), "group": "waystone", "kind": "prefix",
                "name": null, "lines": [format!("#% {w}")], "bonus": [false], "tiers": 1
            })
        })
        .collect();
    let doc = serde_json::json!({
        "schema": 1, "source": "https://poe2db.tw/us/Waystones",
        "sections": ["WaystonesMods"], "fetched": "2026-10-28",
        "source_note": "synthetic", "families": families, "skipped": []
    });
    let data = ModData::parse(&doc.to_string()).expect("three-letter data loads");
    (data, words)
}
