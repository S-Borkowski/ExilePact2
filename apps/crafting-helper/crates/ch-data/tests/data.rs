//! ch-data against the embedded data (2026-10-28 extraction) and the real
//! item texts in `ch-item`'s `fixtures/real/`, read in place.

use std::fs;
use std::path::PathBuf;

use ch_data::{
    add_odds, class_data, index, join, variant_of, AddOptions, ClassData, JoinResult, Kind, Odds,
    Section, TierRule, VariantUnknown, EMBEDDED_CLASSES,
};
use ch_item::{Item, ItemClass};

fn real_fixture(name: &str) -> Item {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("ch-item")
        .join("fixtures")
        .join("real")
        .join(name);
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    ch_item::parse(&text).unwrap_or_else(|e| panic!("parse {name}: {e}"))
}

/// The five distinct real fixtures (the two gloves copies are identical).
const REAL: [&str; 5] = [
    "boots-magic-hybrid-unmet-advanced.txt",
    "crossbows-rare-desecrated-damage-advanced.txt",
    "gloves-rare-rune-desecrated-ctrl-alt-c.txt",
    "jewels-rare-corrupted-enhancement-advanced.txt",
    "rings-normal-implicit-advanced.txt",
];

/// A constructed item: class, rarity, base type and item level, nothing
/// else.
fn bare_item(class: &str, rarity: &str, base: &str, item_level: u32) -> Item {
    let name = if rarity == "Rare" { "Test Name\n" } else { "" };
    let text = format!(
        "Item Class: {class}\nRarity: {rarity}\n{name}{base}\n--------\nItem Level: {item_level}\n"
    );
    let item = ch_item::parse(&text).unwrap();
    assert_eq!(item.base_type.as_deref(), Some(base), "{text}");
    item
}

fn class(name: &str) -> &'static ClassData {
    class_data(&ItemClass::from_text(name)).unwrap_or_else(|| panic!("no data for {name}"))
}

fn assert_percent(odds: &Odds, expected: f64, basis: &str) {
    match odds {
        Odds::Known {
            percent,
            basis: got,
            ..
        } => {
            assert!(
                (percent - expected).abs() < 0.1,
                "expected {expected}%, got {percent}% ({got})"
            );
            assert_eq!(got, basis);
        }
        Odds::Unknown { reason } => panic!("expected {expected}%, got unknown: {reason}"),
    }
}

#[test]
fn embedded_classes_match_index() {
    let idx = index();
    assert_eq!(idx.classes.len(), 22);
    let embedded: Vec<&str> = EMBEDDED_CLASSES.iter().map(|(file, _)| *file).collect();
    let listed: Vec<&str> = idx.classes.iter().map(|c| c.file.as_str()).collect();
    assert_eq!(embedded, listed);
    for entry in &idx.classes {
        let data = ch_data::class_data_by_name(&entry.class)
            .unwrap_or_else(|| panic!("{} not embedded", entry.class));
        assert_eq!(data.class, entry.class, "{}", entry.file);
        let variants: Vec<&str> = data.pages.iter().map(|p| p.variant.as_str()).collect();
        assert_eq!(variants, entry.variants, "{}", entry.file);
    }
    // Every in-scope ItemClass resolves; one outside does not.
    for c in ItemClass::ALL.iter() {
        assert_eq!(class_data(c).map(|d| d.class.as_str()), Some(c.as_str()));
    }
    assert!(class_data(&ItemClass::Other("Waystones".into())).is_none());
}

#[test]
fn variant_comes_from_the_base_type() {
    for (class_name, base, variant) in [
        ("Gloves", "Sirenscale Gloves", "int"),
        ("Boots", "Drakeskin Boots", "dex"),
        ("Crossbows", "Siege Crossbow", "all"),
        ("Jewels", "Sapphire", "sapphire"),
        ("Rings", "Amethyst Ring", "all"),
    ] {
        let item = bare_item(class_name, "Normal", base, 80);
        let (data, found) = variant_of(&item).unwrap();
        assert_eq!(data.class, class_name);
        assert_eq!(found, variant, "{base}");
    }
    // Negative controls: a base name must match exactly, and a missing base
    // or an out-of-scope class is unknown, not guessed.
    let ring = bare_item("Rings", "Normal", "Amethyst", 80);
    assert_eq!(
        variant_of(&ring),
        Err(VariantUnknown::BaseNotInData {
            base_type: "Amethyst".into()
        })
    );
    let mut no_base = bare_item("Rings", "Normal", "Amethyst Ring", 80);
    no_base.base_type = None;
    assert_eq!(variant_of(&no_base), Err(VariantUnknown::NoBaseType));
    let other = bare_item("Waystones", "Normal", "Waystone (Tier 1)", 80);
    assert!(matches!(
        variant_of(&other),
        Err(VariantUnknown::ClassOutOfScope { .. })
    ));
    // An item whose variant is unknown joins nothing.
    let mut real = real_fixture("gloves-rare-rune-desecrated-ctrl-alt-c.txt");
    real.base_type = Some("Gloves".into());
    let joined = join(&real);
    assert!(joined.variant.is_err());
    assert!(joined.mods.iter().all(|m| m.matched().is_none()));
}

#[test]
fn real_fixture_mods_join_to_data() {
    let mut total = 0;
    for name in REAL {
        let item = real_fixture(name);
        let joined = join(&item);
        assert!(joined.variant.is_ok(), "{name}: {:?}", joined.variant);
        for m in &joined.mods {
            match &m.result {
                JoinResult::Matched(found) => {
                    assert_eq!(
                        found.decided_by,
                        TierRule::AffixName,
                        "{name} {:?}",
                        m.affix_name
                    );
                    let data = variant_of(&item).unwrap().0;
                    let dm = data.find_mod(&found.mod_id).unwrap();
                    assert_eq!(dm.kind, m.kind);
                }
                JoinResult::Unmatched { reason } => {
                    panic!("{name} {:?} did not join: {reason:?}", m.affix_name)
                }
            }
        }
        total += joined.mods.len();
    }
    assert_eq!(total, 18);
    // The implicit, the rune and the corruption enhancement are not joined.
    let gloves = real_fixture("gloves-rare-rune-desecrated-ctrl-alt-c.txt");
    assert_eq!(gloves.modifiers.len(), 7);
    assert_eq!(join(&gloves).mods.len(), 6);
}

#[test]
fn real_fixture_tiers_equal_game_tiers() {
    let mut mismatches = Vec::new();
    let mut counts = Vec::new();
    for name in REAL {
        let item = real_fixture(name);
        let joined = join(&item);
        for m in &joined.mods {
            let derived = m.matched().map(|f| f.tier);
            if derived.is_none() || derived != m.game_tier {
                mismatches.push(format!(
                    "{name} {:?}: game {:?}, derived {derived:?}",
                    m.affix_name, m.game_tier
                ));
            }
        }
        counts.push(joined.mods.len());
    }
    assert!(mismatches.is_empty(), "{mismatches:#?}");
    assert_eq!(counts, [2, 6, 6, 4, 0]);
}

#[test]
fn desecrated_mods_join_to_normal_section() {
    for (name, affix, mod_id, tier) in [
        (
            "crossbows-rare-desecrated-damage-advanced.txt",
            "Incinerating",
            "prefix-adds-to-fire-damage",
            4,
        ),
        (
            "gloves-rare-rune-desecrated-ctrl-alt-c.txt",
            "of Exile",
            "suffix-to-chaos-resistance",
            2,
        ),
    ] {
        let item = real_fixture(name);
        let joined = join(&item);
        let desecrated: Vec<_> = joined.mods.iter().filter(|m| m.desecrated).collect();
        assert_eq!(desecrated.len(), 1, "{name}");
        let m = desecrated[0];
        assert_eq!(m.affix_name.as_deref(), Some(affix));
        assert!(!m.unrevealed);
        let found = m.matched().unwrap();
        assert_eq!(found.mod_id, mod_id);
        assert_eq!(found.section, Section::Normal);
        assert_eq!(found.tier, tier);
        // The others on the item are not desecrated.
        assert!(joined
            .mods
            .iter()
            .filter(|o| o.index != m.index)
            .all(|o| !o.desecrated));
    }
}

#[test]
fn valid_mods_respect_ilvl_and_variant() {
    let gloves = class("Gloves");
    // Int gloves at item level 58: energy shield tiers 1 (level 65) and 2
    // (60) are out of reach, 3 (54) is the best.
    let valid = gloves.valid_mods("int", 58);
    let es = valid
        .normal
        .iter()
        .find(|m| m.id == "prefix-increased-energy-shield")
        .expect("energy shield on int gloves");
    let tiers: Vec<u32> = es.tiers.iter().map(|t| t.tier).collect();
    assert_eq!(tiers, [3, 4, 5, 6, 7]);
    assert_eq!(es.best_tier, 3);
    assert!(es.tiers.iter().all(|t| t.ilvl <= 58));
    // At 65 the first tier is in.
    let high = gloves.valid_mods("int", 65);
    let es65 = high
        .normal
        .iter()
        .find(|m| m.id == "prefix-increased-energy-shield")
        .unwrap();
    assert_eq!(es65.best_tier, 1);
    // Armour is a str variant's modifier: absent on int, present on str.
    assert!(valid
        .normal
        .iter()
        .all(|m| m.id != "prefix-increased-armour"));
    assert!(gloves
        .valid_mods("str", 58)
        .normal
        .iter()
        .any(|m| m.id == "prefix-increased-armour"));
    // Desecrated-section modifiers are listed apart, and only there.
    assert!(valid.normal.iter().all(|m| m.section == Section::Normal));
    assert!(valid
        .desecrated
        .iter()
        .all(|m| m.section == Section::Desecrated));
    assert!(!high.desecrated.is_empty());
    // A modifier's tiers on a variant.
    assert_eq!(
        gloves
            .mod_tiers("prefix-increased-energy-shield", "int")
            .map(|t| t.len()),
        Some(7)
    );
    assert_eq!(
        gloves
            .mod_tiers("prefix-increased-energy-shield", "str")
            .map(|t| t.len()),
        Some(0)
    );
    assert!(gloves.mod_tiers("no-such-mod", "int").is_none());
}

#[test]
fn ring_life_exalt_odds_match_the_weights() {
    let ring = bare_item("Rings", "Rare", "Amethyst Ring", 76);
    let prefix = AddOptions {
        side: Some(Kind::Prefix),
        ..Default::default()
    };
    let life = "prefix-to-maximum-life";
    assert_percent(
        &add_odds(&ring, life, 8, &prefix),
        11.5,
        "8000 of 69500 weight",
    );
    assert_percent(
        &add_odds(&ring, life, 1, &prefix),
        1.4,
        "1000 of 69500 weight",
    );
    let perfect = AddOptions {
        min_modifier_level: Some(50),
        ..prefix.clone()
    };
    assert_percent(
        &add_odds(&ring, life, 1, &perfect),
        4.5,
        "1000 of 22200 weight",
    );
    // A family already on the item takes the target out of the pool.
    let with_life = ch_item::parse(
        "Item Class: Rings\nRarity: Rare\nTest Name\nAmethyst Ring\n--------\nItem Level: 76\n--------\n{ Prefix Modifier \"Hale\" (Tier: 8) — Life }\n+12(10-14) to maximum Life\n",
    )
    .unwrap();
    let joined = join(&with_life);
    assert_eq!(
        joined.mods[0].matched().map(|m| m.mod_id.as_str()),
        Some(life),
        "{:?}",
        joined.mods
    );
    assert_eq!(add_odds(&with_life, life, 8, &prefix).percent(), Some(0.0));
}

#[test]
fn odds_unknown_without_spawn_weights() {
    let jewel = bare_item("Jewels", "Rare", "Sapphire", 82);
    let (data, variant) = variant_of(&jewel).unwrap();
    let target = &data.valid_mods(variant, 82).normal[0];
    match add_odds(&jewel, target.id, target.best_tier, &AddOptions::default()) {
        Odds::Unknown { reason } => assert!(reason.contains("spawn weights"), "{reason}"),
        known => panic!("jewel odds should be unknown, got {known:?}"),
    }
    // Positive control through the same call: a ring with spawn weights.
    let ring = bare_item("Rings", "Rare", "Amethyst Ring", 76);
    assert!(
        add_odds(&ring, "prefix-to-maximum-life", 8, &AddOptions::default())
            .percent()
            .is_some()
    );
}

#[test]
fn essence_guarantee_found_for_its_tier() {
    let gloves = class("Gloves");
    let names = |floor: u32| -> Vec<String> {
        let mut names: Vec<String> = gloves
            .essences_for("prefix-increased-energy-shield", "int", floor)
            .iter()
            .map(|e| e.essence.name.clone())
            .collect();
        names.sort();
        names
    };
    assert_eq!(
        names(4),
        ["Essence of Enhancement", "Greater Essence of Enhancement"]
    );
    assert_eq!(names(3), ["Greater Essence of Enhancement"]);
    assert!(names(2).is_empty());
    // Every tier reaches the Lesser one at floor 6.
    assert_eq!(names(6).len(), 3);
    // The same essence matches armour on str, not energy shield.
    assert!(gloves
        .essences_for("prefix-increased-energy-shield", "str", 6)
        .is_empty());
    // Liquid emotions are a jewel lookup; gloves have none.
    assert!(gloves
        .liquids_for("prefix-increased-energy-shield", "int", 6)
        .is_empty());
    let jewels = class("Jewels");
    let liquid = &jewels.liquid[0];
    let (variant, matched) = liquid
        .matches
        .iter()
        .find_map(|(v, m)| m.as_ref().map(|m| (v.clone(), m.clone())))
        .unwrap();
    assert!(jewels
        .liquids_for(&matched.mod_id, &variant, matched.tier)
        .iter()
        .any(|e| e.essence.name == liquid.name));
}
