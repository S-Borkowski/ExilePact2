//! Every fixture parses with nothing left over, plus what each constructed
//! file is there to show. The constructed files are written by hand
//! (`fixtures/README.md`); `fixtures/real/` holds only texts the owner copied
//! in the game.

mod common;

use ch_item::{Form, Item, ItemClass, ModKind, Modifier, Range, Rarity, Value};
use common::{constructed, constructed_dir, parse_ok, read, real_dir, txt_files};

#[test]
fn every_constructed_fixture_parses_without_unparsed_lines() {
    let files = txt_files(&constructed_dir());
    assert!(files.len() >= 23, "constructed fixtures: {}", files.len());
    for f in files {
        let item = parse_ok(&read(&f));
        assert!(
            item.report.unparsed.is_empty() && item.report.partial.is_empty(),
            "{}: {:?}",
            f.display(),
            item.report
        );
    }
}

#[test]
fn every_real_fixture_parses_without_unparsed_lines() {
    for f in txt_files(&real_dir()) {
        let item = parse_ok(&read(&f));
        assert!(
            item.report.unparsed.is_empty(),
            "{}: {:?}",
            f.display(),
            item.report.unparsed
        );
        // A real text is the shape the game writes: nothing in it may be
        // only partly understood either.
        assert!(
            item.report.partial.is_empty(),
            "{}: {:?}",
            f.display(),
            item.report.partial
        );
    }
}

// Per-file assertions for `fixtures/real/`. The expected values come from
// the row in `fixtures/real/README.md` (the owner's description, or for the
// four texts pasted without one, what the text visibly shows), never from
// the parser's output.

fn real(name: &str) -> Item {
    parse_ok(&read(&real_dir().join(name)))
}

fn value(v: f64, min: f64, max: f64) -> Value {
    Value {
        value: v,
        range: Some(Range { min, max }),
    }
}

#[test]
fn real_gloves_rune_and_revealed_desecrated_suffix() {
    let item = real("gloves-rare-rune-desecrated-ctrl-alt-c.txt");
    assert_eq!(item.form, Form::Advanced);
    assert_eq!(item.class, ItemClass::Gloves);
    assert_eq!(item.rarity, Rarity::Rare);
    assert_eq!(item.name.as_deref(), Some("Ghoul Mitts"));
    assert_eq!(item.base_type.as_deref(), Some("Sirenscale Gloves"));
    assert_eq!(item.item_level.as_ref().map(|l| l.value), Some(81));
    let q = item.quality.as_ref().expect("quality");
    assert_eq!((q.value, q.augmented), (20, true));
    assert_eq!(item.sockets.as_ref().map(|s| s.count), Some(1));
    let r = item.requirements.as_ref().expect("requirements");
    assert_eq!((r.level, r.intelligence), (Some(80), Some(101)));
    assert!(r.unmet.is_empty());
    let rune = &item.modifiers[0];
    assert_eq!(rune.kind, ModKind::Rune);
    assert_eq!(
        rune.lines[0].template,
        "#% increased Armour, Evasion and Energy Shield"
    );
    assert_eq!(rune.lines[0].values[0].value, 18.0);
    assert_eq!((item.prefix_count(), item.suffix_count()), (3, 3));
    let desecrated: Vec<&Modifier> = item.modifiers.iter().filter(|m| m.desecrated).collect();
    assert_eq!(desecrated.len(), 1);
    assert_eq!(desecrated[0].kind, ModKind::Suffix);
    assert_eq!(desecrated[0].affix_name.as_deref(), Some("of Exile"));
    assert_eq!(desecrated[0].lines[0].template, "+#% to Chaos Resistance");
    assert!(!item.has_fractured() && !item.has_unrevealed());
    assert!(!item.corrupted && !item.mirrored && !item.unidentified);
}

#[test]
fn real_gloves_plain_ctrl_c_copy_is_the_advanced_form() {
    let advanced = read(&real_dir().join("gloves-rare-rune-desecrated-ctrl-alt-c.txt"));
    let plain = read(&real_dir().join("gloves-rare-rune-desecrated-ctrl-c.txt"));
    assert_eq!(plain, advanced);
    assert_eq!(parse_ok(&plain).form, Form::Advanced);
}

#[test]
fn real_ring_normal_with_a_nameless_implicit() {
    let item = real("rings-normal-implicit-advanced.txt");
    assert_eq!(item.form, Form::Advanced);
    assert_eq!(item.class, ItemClass::Rings);
    assert_eq!(item.rarity, Rarity::Normal);
    assert_eq!(item.name, None);
    assert_eq!(item.base_type.as_deref(), Some("Amethyst Ring"));
    assert_eq!(item.item_level.as_ref().map(|l| l.value), Some(76));
    assert_eq!(item.requirements.as_ref().and_then(|r| r.level), Some(20));
    assert_eq!(item.modifiers.len(), 1);
    let implicit = &item.modifiers[0];
    assert_eq!(implicit.kind, ModKind::Implicit);
    assert_eq!(
        (implicit.affix_name.as_deref(), implicit.tier),
        (None, None)
    );
    assert_eq!(
        implicit.tags,
        vec!["Chaos".to_string(), "Resistance".to_string()]
    );
    assert_eq!(implicit.lines[0].template, "+#% to Chaos Resistance");
    assert_eq!(implicit.lines[0].values, vec![value(7.0, 7.0, 13.0)]);
    assert!(implicit.lines[0].markers.is_empty());
    assert_eq!((item.prefix_count(), item.suffix_count()), (0, 0));
}

#[test]
fn real_boots_magic_hybrid_prefix_and_unmet_dex() {
    let item = real("boots-magic-hybrid-unmet-advanced.txt");
    assert_eq!(item.class, ItemClass::Boots);
    assert_eq!(item.rarity, Rarity::Magic);
    assert_eq!(item.base_type.as_deref(), Some("Drakeskin Boots"));
    assert_eq!(item.item_level.as_ref().map(|l| l.value), Some(81));
    let evasion = &item.properties[0];
    assert_eq!(
        (
            evasion.name.as_str(),
            evasion.value.as_str(),
            evasion.augmented
        ),
        ("Evasion Rating", "362", true)
    );
    let r = item.requirements.as_ref().expect("requirements");
    assert_eq!((r.level, r.dexterity), (Some(80), Some(108)));
    assert_eq!(r.unmet, vec!["Dex".to_string()]);
    assert!(r.other.is_empty());
    assert_eq!(item.modifiers.len(), 2);
    let prefix = &item.modifiers[0];
    assert_eq!(prefix.kind, ModKind::Prefix);
    assert_eq!(prefix.affix_name.as_deref(), Some("Hummingbird's"));
    assert_eq!(prefix.tier, Some(1));
    assert!(prefix.tags.is_empty());
    let templates: Vec<&str> = prefix.lines.iter().map(|l| l.template.as_str()).collect();
    assert_eq!(
        templates,
        vec!["#% increased Evasion Rating", "+# to Stun Threshold"]
    );
    assert_eq!(prefix.lines[0].values, vec![value(42.0, 39.0, 42.0)]);
    assert_eq!(prefix.lines[1].values, vec![value(108.0, 95.0, 136.0)]);
    let suffix = &item.modifiers[1];
    assert_eq!(suffix.kind, ModKind::Suffix);
    assert_eq!(suffix.affix_name.as_deref(), Some("of the Phantom"));
    assert_eq!(suffix.tags, vec!["Attribute".to_string()]);
    assert_eq!(suffix.lines[0].values, vec![value(33.0, 31.0, 33.0)]);
}

#[test]
fn real_crossbow_desecrated_two_value_prefix_and_decimal_range() {
    let item = real("crossbows-rare-desecrated-damage-advanced.txt");
    assert_eq!(item.class, ItemClass::Crossbows);
    assert_eq!(item.rarity, Rarity::Rare);
    assert_eq!(item.name.as_deref(), Some("Sorrow Core"));
    assert_eq!(item.base_type.as_deref(), Some("Siege Crossbow"));
    assert_eq!(item.item_level.as_ref().map(|l| l.value), Some(81));
    let r = item.requirements.as_ref().expect("requirements");
    assert_eq!(
        (r.level, r.strength, r.dexterity),
        (Some(79), Some(89), Some(89))
    );
    assert_eq!(r.unmet, vec!["Dex".to_string()]);
    let names: Vec<&str> = item.properties.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(
        names,
        vec![
            "Physical Damage",
            "Fire Damage",
            "Critical Hit Chance",
            "Attacks per Second",
            "Reload Time"
        ]
    );
    assert!(item.properties[0].augmented && !item.properties[1].augmented);
    let implicit = &item.modifiers[0];
    assert_eq!(implicit.kind, ModKind::Implicit);
    assert!(implicit.tags.is_empty());
    assert_eq!(
        implicit.lines[0].template,
        "Grenade Skills Fire an additional Projectile"
    );
    assert!(implicit.lines[0].values.is_empty());
    assert_eq!((item.prefix_count(), item.suffix_count()), (3, 3));
    let desecrated: Vec<&Modifier> = item.modifiers.iter().filter(|m| m.desecrated).collect();
    assert_eq!(desecrated.len(), 1);
    let incinerating = desecrated[0];
    assert_eq!(incinerating.kind, ModKind::Prefix);
    assert_eq!(incinerating.affix_name.as_deref(), Some("Incinerating"));
    assert_eq!(incinerating.tier, Some(4));
    assert_eq!(incinerating.lines[0].template, "Adds # to # Fire Damage");
    assert_eq!(
        incinerating.lines[0].values,
        vec![value(59.0, 56.0, 70.0), value(104.0, 84.0, 107.0)]
    );
    let calamity = item.modifiers.last().unwrap();
    assert_eq!(calamity.affix_name.as_deref(), Some("of Calamity"));
    assert_eq!(calamity.lines[0].values, vec![value(3.33, 3.11, 3.8)]);
    assert!(!item.has_fractured() && !item.corrupted);
}

#[test]
fn real_jewel_corruption_enhancement_and_corrupted() {
    let item = real("jewels-rare-corrupted-enhancement-advanced.txt");
    assert_eq!(item.class, ItemClass::Jewels);
    assert_eq!(item.rarity, Rarity::Rare);
    assert_eq!(item.name.as_deref(), Some("Glyph Breath"));
    assert_eq!(item.base_type.as_deref(), Some("Sapphire"));
    assert_eq!(item.item_level.as_ref().map(|l| l.value), Some(82));
    assert!(item.requirements.is_none());
    let enhancement = &item.modifiers[0];
    assert_eq!(enhancement.kind, ModKind::Corruption);
    assert_eq!(
        enhancement.tags,
        vec![
            "Elemental".to_string(),
            "Cold".to_string(),
            "Resistance".to_string()
        ]
    );
    assert_eq!(enhancement.lines[0].template, "+#% to Cold Resistance");
    assert_eq!(enhancement.lines[0].values, vec![value(10.0, 5.0, 10.0)]);
    assert_eq!(item.modifiers.len(), 5);
    assert_eq!((item.prefix_count(), item.suffix_count()), (2, 2));
    assert_eq!(item.description.len(), 1);
    assert!(item.corrupted && !item.mirrored);
}

#[test]
fn constructed_file_names_match_class_and_rarity() {
    for f in txt_files(&constructed_dir()) {
        let name = f.file_name().unwrap().to_string_lossy().into_owned();
        let item = parse_ok(&read(&f));
        let class = item.class.as_str().to_lowercase().replace(' ', "-");
        let rarity = match &item.rarity {
            Rarity::Normal => "normal",
            Rarity::Magic => "magic",
            Rarity::Rare => "rare",
            Rarity::Unique => "unique",
            Rarity::Other(o) => panic!("{name}: rarity {o}"),
        };
        assert!(
            name.starts_with(&format!("{class}-{rarity}")),
            "{name}: {class} {rarity}"
        );
    }
}

#[test]
fn required_gloves_fixture_has_every_feature() {
    let item = parse_ok(&constructed("gloves-rare-fractured-desecrated.txt"));
    assert_eq!(item.form, Form::Advanced);
    assert_eq!(item.rarity, Rarity::Rare);
    assert_eq!(item.quality.as_ref().map(|q| q.value), Some(20));
    assert_eq!(item.sockets.as_ref().map(|s| s.count), Some(1));
    let kinds: Vec<ModKind> = item.modifiers.iter().map(|m| m.kind).collect();
    assert_eq!(
        kinds,
        vec![
            ModKind::Implicit,
            ModKind::Rune,
            ModKind::Prefix,
            ModKind::Prefix,
            ModKind::Suffix,
            ModKind::Suffix,
            ModKind::Suffix
        ]
    );
    let fractured = &item.modifiers[2];
    assert!(fractured.fractured);
    assert_eq!(fractured.affix_name.as_deref(), Some("Hale"));
    assert_eq!(fractured.tier, Some(4));
    assert_eq!(fractured.tags, vec!["Life".to_string()]);
    let hybrid = &item.modifiers[3];
    assert_eq!(hybrid.lines.len(), 2);
    assert_eq!(hybrid.tags.len(), 3);
    let revealed = &item.modifiers[4];
    assert!(revealed.desecrated && !revealed.unrevealed);
    assert_eq!(revealed.affix_name.as_deref(), Some("of the Drowned"));
    let unrevealed = &item.modifiers[6];
    assert!(unrevealed.desecrated && unrevealed.unrevealed);
    assert!(item.has_fractured() && item.has_desecrated() && item.has_unrevealed());
    assert_eq!((item.prefix_count(), item.suffix_count()), (2, 3));
    assert!(!item.corrupted && !item.mirrored && !item.unidentified);
}

#[test]
fn constructed_crossbow_hybrid_beside_plain_physical() {
    // Two prefixes carry `#% increased Physical Damage`: the plain one and
    // the hybrid with Accuracy. Each header starts its own modifier, so the
    // hybrid is one prefix with two lines, not two prefixes.
    let item = parse_ok(&constructed("crossbows-rare-hybrid-physical.txt"));
    assert_eq!(item.form, Form::Advanced);
    assert_eq!(item.class, ItemClass::Crossbows);
    assert_eq!(item.rarity, Rarity::Rare);
    assert_eq!(item.base_type.as_deref(), Some("Siege Crossbow"));
    assert_eq!((item.prefix_count(), item.suffix_count()), (3, 3));
    let prefixes: Vec<&Modifier> = item
        .modifiers
        .iter()
        .filter(|m| m.kind == ModKind::Prefix)
        .collect();
    let line_counts: Vec<usize> = prefixes.iter().map(|m| m.lines.len()).collect();
    assert_eq!(line_counts, vec![1, 2, 1]);

    let plain = prefixes[0];
    assert_eq!(plain.affix_name.as_deref(), Some("Vicious"));
    assert_eq!(plain.lines[0].template, "#% increased Physical Damage");
    assert_eq!(plain.lines[0].values, vec![value(92.0, 85.0, 109.0)]);

    let hybrid = prefixes[1];
    assert_eq!(hybrid.affix_name.as_deref(), Some("Reaver's"));
    assert_eq!(hybrid.tier, Some(6));
    let templates: Vec<&str> = hybrid.lines.iter().map(|l| l.template.as_str()).collect();
    assert_eq!(
        templates,
        vec!["#% increased Physical Damage", "+# to Accuracy Rating"]
    );
    assert_eq!(hybrid.lines[0].values, vec![value(28.0, 25.0, 34.0)]);
    assert_eq!(hybrid.lines[1].values, vec![value(55.0, 47.0, 72.0)]);

    let flat = prefixes[2];
    assert_eq!(flat.lines[0].template, "Adds # to # Physical Damage");
    assert_eq!(
        flat.lines[0].values,
        vec![value(27.0, 23.0, 35.0), value(48.0, 39.0, 59.0)]
    );
    assert!(!item.has_fractured() && !item.has_desecrated() && !item.corrupted);
}

#[test]
fn short_form_gloves_fixture() {
    let item = parse_ok(&constructed("gloves-rare-short-form.txt"));
    assert_eq!(item.form, Form::Short);
    assert_eq!(item.modifiers.len(), 8);
    assert_eq!((item.prefix_count(), item.suffix_count()), (0, 0));
    assert!(item.has_fractured() && item.has_desecrated() && item.has_unrevealed());
}

#[test]
fn decimal_fixture_values() {
    let item = parse_ok(&constructed("bows-rare-decimal.txt"));
    let leech = &item.modifiers[0].lines[0].values[0];
    assert_eq!(leech.value, 6.5);
    assert_eq!(leech.range, Some(Range { min: 5.5, max: 6.9 }));
    let crit = &item.modifiers[1].lines[0].values[0];
    assert_eq!(crit.value, 1.1);
    assert_eq!(crit.range, Some(Range { min: 0.9, max: 1.2 }));
}

#[test]
fn two_value_fixture_values() {
    for file in [
        "one-hand-maces-rare-two-value.txt",
        "quarterstaves-rare.txt",
    ] {
        let item = parse_ok(&constructed(file));
        let l = &item.modifiers[0].lines[0];
        assert_eq!(l.values.len(), 2, "{file}");
        assert!(l.template.starts_with("Adds # to # "), "{file}");
    }
}

#[test]
fn unscalable_and_enchant_fixtures() {
    let belt = parse_ok(&constructed("belts-rare-unscalable.txt"));
    let implicit = &belt.modifiers[0];
    assert_eq!(implicit.kind, ModKind::Implicit);
    assert!(implicit.lines[0].unscalable);
    let amulet = parse_ok(&constructed("amulets-rare-enchant.txt"));
    let enchant = &amulet.modifiers[0];
    assert_eq!(enchant.kind, ModKind::Enchant);
    assert_eq!(enchant.header_line, None);
    let quiver = parse_ok(&constructed("quivers-magic-enchant.txt"));
    assert_eq!(quiver.modifiers[0].kind, ModKind::Enchant);
    assert!(quiver.modifiers[0].header_line.is_some());
    let wand = parse_ok(&constructed("wands-rare-crafted.txt"));
    assert!(wand
        .modifiers
        .iter()
        .any(|m| m.crafted && m.kind == ModKind::Suffix));
}

#[test]
fn magic_fixtures_strip_their_affix_names() {
    for (file, base) in [
        ("boots-magic.txt", "Leather Boots"),
        ("quivers-magic-enchant.txt", "Feathered Quiver"),
        ("bucklers-magic.txt", "Wooden Buckler"),
        ("spears-magic.txt", "Hunting Spear"),
        ("crossbows-magic.txt", "Makeshift Crossbow"),
        ("rings-magic-unidentified.txt", "Ruby Ring"),
    ] {
        let item = parse_ok(&constructed(file));
        assert_eq!(item.base_type.as_deref(), Some(base), "{file}");
    }
}

#[test]
fn jewel_fixture_has_a_description_and_no_requirements() {
    let item = parse_ok(&constructed("jewels-rare.txt"));
    assert!(item.requirements.is_none());
    assert_eq!(item.description.len(), 1);
    assert!(item.description[0]
        .text
        .starts_with("Place into an allocated Jewel Socket"));
}
