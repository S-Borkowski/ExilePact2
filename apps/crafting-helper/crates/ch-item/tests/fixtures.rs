//! Every fixture parses with nothing left over, plus what each constructed
//! file is there to show. The constructed files are written by hand
//! (`fixtures/README.md`); `fixtures/real/` holds only texts the owner copied
//! in the game.

mod common;

use ch_item::{Form, ModKind, Range, Rarity};
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
    }
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
