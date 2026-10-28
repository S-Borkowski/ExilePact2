//! The item's header block and the non-modifier sections: class, rarity,
//! name and base type, item level, quality, requirements, sockets,
//! properties, flag lines, the not-item-text error and line endings.

mod common;

use ch_item::{parse, ItemClass, ParseError, Rarity};
use common::{constructed, parse_ok, text};

#[test]
fn every_class_string_maps_to_its_class() {
    let names = [
        "Gloves",
        "Boots",
        "Body Armours",
        "Helmets",
        "Quivers",
        "Shields",
        "Bucklers",
        "Foci",
        "Wands",
        "One Hand Maces",
        "Sceptres",
        "Spears",
        "Bows",
        "Staves",
        "Two Hand Maces",
        "Quarterstaves",
        "Crossbows",
        "Talismans",
        "Jewels",
        "Belts",
        "Amulets",
        "Rings",
    ];
    assert_eq!(ItemClass::ALL.len(), names.len());
    for name in names {
        let class = ItemClass::from_text(name);
        assert!(!matches!(class, ItemClass::Other(_)), "{name}");
        assert_eq!(class.as_str(), name);
        let item = parse_ok(&text(&[
            &format!("Item Class: {name}"),
            "Rarity: Normal",
            "Some Base",
        ]));
        assert_eq!(item.class, class);
    }
    assert_eq!(ItemClass::from_text("Gloves"), ItemClass::Gloves);
    assert_eq!(ItemClass::from_text("Body Armours"), ItemClass::BodyArmours);
    assert_eq!(
        ItemClass::from_text("Two Hand Maces"),
        ItemClass::TwoHandMaces
    );
}

#[test]
fn unknown_class_is_kept_as_other() {
    let item = parse_ok(&text(&[
        "Item Class: Waystones",
        "Rarity: Normal",
        "Waystone (Tier 1)",
    ]));
    assert_eq!(item.class, ItemClass::Other("Waystones".into()));
    assert_eq!(item.class.as_str(), "Waystones");
    assert!(item.report.unparsed.is_empty());
}

#[test]
fn each_rarity_is_read() {
    for (word, rarity) in [
        ("Normal", Rarity::Normal),
        ("Magic", Rarity::Magic),
        ("Rare", Rarity::Rare),
        ("Unique", Rarity::Unique),
        ("Currency", Rarity::Other("Currency".into())),
    ] {
        let item = parse_ok(&text(&[
            "Item Class: Rings",
            &format!("Rarity: {word}"),
            "Ruby Ring",
        ]));
        assert_eq!(item.rarity, rarity, "{word}");
    }
}

#[test]
fn normal_item_name_line_is_its_base_type() {
    let item = parse_ok(&constructed("helmets-normal.txt"));
    assert_eq!(item.rarity, Rarity::Normal);
    assert_eq!(item.name, None);
    assert_eq!(item.base_type.as_deref(), Some("Iron Cap"));
    assert_eq!(item.name_lines.len(), 1);
    assert_eq!(item.name_lines[0].text, "Iron Cap");
}

#[test]
fn magic_item_base_type_has_its_affix_names_stripped() {
    let item = parse_ok(&constructed("boots-magic.txt"));
    assert_eq!(item.name, None);
    assert_eq!(item.base_type.as_deref(), Some("Leather Boots"));
    assert_eq!(
        item.name_lines[0].text,
        "Sprinter's Leather Boots of the Fox"
    );
    let suffix_only = parse_ok(&constructed("quivers-magic-enchant.txt"));
    assert_eq!(suffix_only.base_type.as_deref(), Some("Feathered Quiver"));
    let prefix_only = parse_ok(&constructed("bucklers-magic.txt"));
    assert_eq!(prefix_only.base_type.as_deref(), Some("Wooden Buckler"));
}

#[test]
fn magic_item_base_type_is_unknown_in_the_short_form() {
    let item = parse_ok(&text(&[
        "Item Class: Boots",
        "Rarity: Magic",
        "Sprinter's Leather Boots of the Fox",
        "--------",
        "Item Level: 20",
        "--------",
        "15% increased Movement Speed",
        "+9 to Dexterity",
    ]));
    assert_eq!(item.base_type, None);
    assert_eq!(
        item.name_lines[0].text,
        "Sprinter's Leather Boots of the Fox"
    );
    assert!(item.report.unparsed.is_empty());
}

#[test]
fn rare_item_has_a_name_and_a_base_type() {
    let item = parse_ok(&constructed("gloves-rare-fractured-desecrated.txt"));
    assert_eq!(item.rarity, Rarity::Rare);
    assert_eq!(item.name.as_deref(), Some("Grim Grasp"));
    assert_eq!(item.base_type.as_deref(), Some("Plated Gauntlets"));
    assert_eq!(item.name_lines.len(), 2);
}

#[test]
fn unidentified_rare_single_name_line_is_its_base_type() {
    let item = parse_ok(&constructed("foci-rare-unidentified.txt"));
    assert!(item.unidentified);
    assert_eq!(item.name, None);
    assert_eq!(item.base_type.as_deref(), Some("Crystal Focus"));
    let magic = parse_ok(&constructed("rings-magic-unidentified.txt"));
    assert!(magic.unidentified);
    assert_eq!(magic.base_type.as_deref(), Some("Ruby Ring"));
}

#[test]
fn item_level_is_read() {
    let item = parse_ok(&constructed("gloves-rare-fractured-desecrated.txt"));
    assert_eq!(item.item_level.as_ref().map(|l| l.value), Some(78));
}

#[test]
fn quality_is_read_with_its_augmented_flag() {
    let item = parse_ok(&constructed("gloves-rare-fractured-desecrated.txt"));
    let q = item.quality.as_ref().expect("quality");
    assert_eq!((q.value, q.augmented), (20, true));
    let plain = parse_ok(&text(&[
        "Item Class: Gloves",
        "Rarity: Normal",
        "Plated Gauntlets",
        "--------",
        "Quality: +7%",
    ]));
    let q = plain.quality.as_ref().expect("quality");
    assert_eq!((q.value, q.augmented), (7, false));
    assert!(item.properties.iter().all(|p| p.name != "Quality"));
}

#[test]
fn requirements_single_line_form() {
    let item = parse_ok(&constructed("sceptres-rare.txt"));
    let r = item.requirements.as_ref().expect("requirements");
    assert_eq!(r.level, Some(20));
    assert_eq!(r.strength, Some(18));
    assert_eq!(r.dexterity, None);
    assert_eq!(r.intelligence, Some(31));
    assert!(r.other.is_empty());
    let level_only = parse_ok(&constructed("helmets-normal.txt"));
    assert_eq!(level_only.requirements.as_ref().unwrap().level, Some(5));
}

#[test]
fn requirements_block_form() {
    let item = parse_ok(&constructed("body-armours-rare-requirements-block.txt"));
    let r = item.requirements.as_ref().expect("requirements");
    assert_eq!(r.level, Some(60));
    assert_eq!(r.strength, Some(121));
    assert_eq!(r.lines.len(), 3);
    assert!(item.properties.iter().all(|p| p.name != "Level"));
}

#[test]
fn unknown_requirement_entry_is_kept_as_text() {
    let item = parse_ok(&text(&[
        "Item Class: Gloves",
        "Rarity: Normal",
        "Plated Gauntlets",
        "--------",
        "Requires: Level 45, 63 Str, Class Warrior",
    ]));
    let r = item.requirements.as_ref().expect("requirements");
    assert_eq!(r.level, Some(45));
    assert_eq!(r.strength, Some(63));
    assert_eq!(r.other, vec!["Class Warrior".to_string()]);
    assert_eq!(item.report.partial.len(), 1);
}

#[test]
fn unmet_requirement_is_read_and_listed() {
    // Measured (owner's pastes, 2026-10-28): `108 (unmet) Dex`.
    let item = parse_ok(&text(&[
        "Item Class: Crossbows",
        "Rarity: Normal",
        "Siege Crossbow",
        "--------",
        "Requires: Level 79, 89 Str, 89 (unmet) Dex",
    ]));
    let r = item.requirements.as_ref().expect("requirements");
    assert_eq!(
        (r.level, r.strength, r.dexterity),
        (Some(79), Some(89), Some(89))
    );
    assert_eq!(r.unmet, vec!["Dex".to_string()]);
    assert!(r.other.is_empty());
    assert!(item.report.partial.is_empty(), "{:?}", item.report);
    // Negative control: a met requirement lists nothing.
    let met = parse_ok(&constructed("sceptres-rare.txt"));
    assert!(met.requirements.as_ref().unwrap().unmet.is_empty());
}

#[test]
fn sockets_keep_their_text_and_count() {
    let item = parse_ok(&constructed("two-hand-maces-rare-mirrored.txt"));
    let s = item.sockets.as_ref().expect("sockets");
    assert_eq!((s.text.as_str(), s.count), ("S S", 2));
    let one = parse_ok(&constructed("gloves-rare-fractured-desecrated.txt"));
    assert_eq!(one.sockets.as_ref().unwrap().count, 1);
}

#[test]
fn properties_keep_order_value_text_and_augmented() {
    let item = parse_ok(&constructed("crossbows-magic.txt"));
    let got: Vec<(&str, &str, bool)> = item
        .properties
        .iter()
        .map(|p| (p.name.as_str(), p.value.as_str(), p.augmented))
        .collect();
    assert_eq!(
        got,
        vec![
            ("Physical Damage", "12-31", true),
            ("Critical Hit Chance", "5.00%", false),
            ("Attacks per Second", "1.65", false),
            ("Reload Time", "0.85", true),
        ]
    );
}

#[test]
fn flag_lines_set_the_item_flags() {
    let corrupted = parse_ok(&constructed("shields-rare-corrupted.txt"));
    assert!(corrupted.corrupted && !corrupted.mirrored && !corrupted.unidentified);
    let mirrored = parse_ok(&constructed("two-hand-maces-rare-mirrored.txt"));
    assert!(mirrored.mirrored && !mirrored.corrupted);
    let unidentified = parse_ok(&constructed("foci-rare-unidentified.txt"));
    assert!(unidentified.unidentified && !unidentified.corrupted);
}

#[test]
fn other_single_word_line_is_unparsed_not_a_flag() {
    let item = parse_ok(&text(&[
        "Item Class: Rings",
        "Rarity: Normal",
        "Ruby Ring",
        "--------",
        "Item Level: 10",
        "--------",
        "Shaped",
    ]));
    assert!(!item.corrupted && !item.mirrored && !item.unidentified);
    assert_eq!(item.report.unparsed.len(), 1);
    assert_eq!(item.report.unparsed[0].text, "Shaped");
    assert_eq!(item.report.unparsed[0].line, 6);
    assert_eq!(item.report.unparsed[0].section, 2);
}

#[test]
fn text_without_an_item_class_line_is_not_item_text() {
    for bad in [
        "",
        "\n\n",
        "hello world",
        "Rarity: Rare\nItem Class: Rings\n",
        "--------\nItem Class: Rings\n",
    ] {
        assert_eq!(parse(bad).unwrap_err(), ParseError::NotItemText, "{bad:?}");
    }
    let with_bom_and_blanks = "\u{feff}\n\n  \nItem Class: Rings\nRarity: Normal\nRuby Ring\n\n\n";
    let item = parse(with_bom_and_blanks).expect("item text");
    assert_eq!(item.class, ItemClass::Rings);
    assert_eq!(item.base_type.as_deref(), Some("Ruby Ring"));
}

#[test]
fn crlf_and_lf_parse_the_same() {
    let lf = constructed("gloves-rare-fractured-desecrated.txt").replace("\r\n", "\n");
    let crlf = lf.replace('\n', "\r\n");
    assert!(crlf.contains("\r\n"));
    let a = parse_ok(&lf);
    let b = parse_ok(&crlf);
    assert_eq!(a, b);
    let with_bom = format!("\u{feff}{crlf}\r\n\r\n");
    assert_eq!(parse_ok(&with_bom), a);
}
