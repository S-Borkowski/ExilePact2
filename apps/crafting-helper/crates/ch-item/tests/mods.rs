//! Modifier sections: the header grammar, kinds and flags by header and by
//! line marker, hybrid modifiers, and the value and template grammar of a
//! modifier line.

mod common;

use ch_item::{Item, Marker, ModKind, Range};
use common::{constructed, parse_ok};

/// A rare with `mod_text` as its modifier section(s).
fn with_mods(mod_text: &str) -> Item {
    parse_ok(&format!(
        "Item Class: Gloves\nRarity: Rare\nGrim Grasp\nPlated Gauntlets\n--------\nItem Level: 50\n--------\n{mod_text}\n"
    ))
}

#[test]
fn header_grammar_reads_kind_name_tier_and_tags() {
    let item = with_mods(
        "{ Prefix Modifier \"Hale\" (Tier: 4) — Life, Defences }\n+62(60-69) to maximum Life",
    );
    assert!(item.report.unparsed.is_empty() && item.report.partial.is_empty());
    let m = &item.modifiers[0];
    assert_eq!(m.kind, ModKind::Prefix);
    assert_eq!(m.affix_name.as_deref(), Some("Hale"));
    assert_eq!(m.tier, Some(4));
    assert_eq!(m.tags, vec!["Life".to_string(), "Defences".to_string()]);
    assert_eq!(m.header_line, Some(7));
    assert_eq!(m.lines.len(), 1);
    assert_eq!(m.lines[0].index, 8);
    assert!(!m.fractured && !m.desecrated && !m.unrevealed && !m.crafted);
}

#[test]
fn header_kinds_implicit_rune_enchant() {
    let item = with_mods(
        "{ Implicit Modifier }\n+8(6-10)% to Fire Resistance (implicit)\n--------\n{ Rune Modifier }\n+12% to Cold Resistance (rune)\n--------\n{ Enchant Modifier }\n10% increased Projectile Speed (enchant)\n--------\n{ Suffix Modifier \"of the Bear\" (Tier: 5) }\n+15(13-16) to Strength",
    );
    assert!(item.report.partial.is_empty(), "{:?}", item.report);
    let kinds: Vec<ModKind> = item.modifiers.iter().map(|m| m.kind).collect();
    assert_eq!(
        kinds,
        vec![
            ModKind::Implicit,
            ModKind::Rune,
            ModKind::Enchant,
            ModKind::Suffix
        ]
    );
    assert_eq!(item.modifiers[0].affix_name, None);
    assert_eq!(item.modifiers[0].tier, None);
    assert!(item.modifiers[0].tags.is_empty());
}

#[test]
fn kind_markers_on_lines_without_a_header() {
    let item = with_mods(
        "+8(6-10)% to Fire Resistance (implicit)\n--------\n+12% to Cold Resistance (rune)\n--------\nAllocates Heavy Buffer (enchant)",
    );
    assert!(item.report.unparsed.is_empty() && item.report.partial.is_empty());
    let kinds: Vec<ModKind> = item.modifiers.iter().map(|m| m.kind).collect();
    assert_eq!(
        kinds,
        vec![ModKind::Implicit, ModKind::Rune, ModKind::Enchant]
    );
    assert!(item.modifiers.iter().all(|m| m.header_line.is_none()));
    let first = &item.modifiers[0].lines[0];
    assert_eq!(first.markers, vec![Marker::Implicit]);
    assert_eq!(first.template, "+#% to Fire Resistance");
    assert_eq!(
        item.modifiers[2].lines[0].template,
        "Allocates Heavy Buffer"
    );
}

#[test]
fn header_adjectives_set_flags() {
    let item = with_mods(
        "{ Fractured Prefix Modifier \"Hale\" (Tier: 4) }\n+62(60-69) to maximum Life (fractured)\n{ Desecrated Suffix Modifier \"of the Drowned\" (Tier: 2) }\n+24(21-26)% to Cold Resistance (desecrated)\n{ Crafted Suffix Modifier \"of Craft\" (Tier: 1) }\n11(10-12)% increased Cast Speed (crafted)",
    );
    assert!(item.report.partial.is_empty(), "{:?}", item.report);
    let flags: Vec<(bool, bool, bool, bool)> = item
        .modifiers
        .iter()
        .map(|m| (m.fractured, m.desecrated, m.unrevealed, m.crafted))
        .collect();
    assert_eq!(
        flags,
        vec![
            (true, false, false, false),
            (false, true, false, false),
            (false, false, false, true)
        ]
    );
    assert!(item.has_fractured() && item.has_desecrated() && !item.has_unrevealed());
    assert_eq!((item.prefix_count(), item.suffix_count()), (1, 2));
}

#[test]
fn flag_markers_set_flags_without_a_header() {
    let item = with_mods(
        "+62 to maximum Life (fractured)\n+24% to Cold Resistance (desecrated)\n11% increased Cast Speed (crafted)",
    );
    assert!(item.report.unparsed.is_empty() && item.report.partial.is_empty());
    assert_eq!(item.modifiers.len(), 3);
    assert!(item.modifiers[0].fractured);
    assert!(item.modifiers[1].desecrated && !item.modifiers[1].unrevealed);
    assert!(item.modifiers[2].crafted);
    assert_eq!(item.modifiers[2].lines[0].markers, vec![Marker::Crafted]);
    assert_eq!(
        item.modifiers[2].lines[0].template,
        "#% increased Cast Speed"
    );
}

#[test]
fn unrevealed_desecrated_modifier_is_flagged() {
    // By header adjective, with no line under the header.
    let item = with_mods(
        "{ Prefix Modifier \"Hale\" (Tier: 4) }\n+62(60-69) to maximum Life\n{ Unrevealed Desecrated Suffix Modifier }",
    );
    assert!(item.report.unparsed.is_empty() && item.report.partial.is_empty());
    let m = &item.modifiers[1];
    assert_eq!(m.kind, ModKind::Suffix);
    assert!(m.lines.is_empty());
    assert!(m.unrevealed && m.desecrated);
    assert!(item.has_unrevealed());
    // `Unrevealed` alone implies desecrated.
    let alone = with_mods("{ Unrevealed Prefix Modifier }");
    assert!(alone.modifiers[0].unrevealed && alone.modifiers[0].desecrated);
    // By a line naming an unrevealed modifier, with no header (short form).
    let short = with_mods("+62 to maximum Life\nUnrevealed Desecrated Modifier");
    assert!(short.report.unparsed.is_empty());
    let m = &short.modifiers[1];
    assert!(m.unrevealed && m.desecrated);
    assert_eq!(m.lines[0].template, "Unrevealed Desecrated Modifier");
    assert!(!short.modifiers[0].unrevealed);
    // And in the required fixture.
    let gloves = parse_ok(&constructed("gloves-rare-fractured-desecrated.txt"));
    let last = gloves.modifiers.last().unwrap();
    assert!(last.unrevealed && last.desecrated && last.kind == ModKind::Suffix);
}

#[test]
fn corruption_enhancement_header_is_a_corruption_modifier() {
    // Measured (owner's pastes, 2026-10-28): a header with no `Modifier`
    // word, in its own section above the explicit modifiers.
    let item = with_mods(
        "{ Corruption Enhancement — Elemental, Cold, Resistance }\n+10(5-10)% to Cold Resistance\n--------\n{ Suffix Modifier \"of the Bear\" (Tier: 5) }\n+15(13-16) to Strength",
    );
    assert!(item.report.partial.is_empty(), "{:?}", item.report);
    let m = &item.modifiers[0];
    assert_eq!(m.kind, ModKind::Corruption);
    assert_eq!((m.affix_name.as_deref(), m.tier), (None, None));
    assert_eq!(m.tags.len(), 3);
    assert_eq!(m.lines[0].template, "+#% to Cold Resistance");
    assert_eq!((item.prefix_count(), item.suffix_count()), (0, 1));
    // Negative control: any other header without `Modifier` stays Unknown
    // and reported.
    let other = with_mods("{ Corruption Something }\n+10 to Strength");
    assert_eq!(other.modifiers[0].kind, ModKind::Unknown);
    assert_eq!(other.report.partial.len(), 1);
}

#[test]
fn unknown_header_kind_word_yields_unknown_kind_and_is_reported() {
    let item = with_mods("{ Shimmering Modifier \"Odd\" (Tier: 2) }\n+5 to Strength");
    let m = &item.modifiers[0];
    assert_eq!(m.kind, ModKind::Unknown);
    assert_eq!(m.affix_name.as_deref(), Some("Odd"));
    assert_eq!(m.tier, Some(2));
    assert_eq!(item.report.partial.len(), 1);
    assert_eq!(item.report.partial[0].part, "Shimmering");
    assert!(item.report.unparsed.is_empty());
}

#[test]
fn unknown_header_adjective_is_reported_and_the_kind_kept() {
    let item = with_mods("{ Glorious Prefix Modifier \"Hale\" (Tier: 1) }\n+5 to maximum Life");
    let m = &item.modifiers[0];
    assert_eq!(m.kind, ModKind::Prefix);
    assert_eq!(m.affix_name.as_deref(), Some("Hale"));
    assert_eq!(item.report.partial.len(), 1);
    assert_eq!(item.report.partial[0].part, "Glorious");
    assert_eq!(item.report.partial[0].line, 7);
}

#[test]
fn header_and_marker_disagreement_is_partly_understood() {
    let item = with_mods(
        "{ Prefix Modifier \"Hale\" (Tier: 1) }\n+10 to maximum Life (fractured)\n{ Suffix Modifier \"of Ice\" (Tier: 1) }\n+10% to Cold Resistance (implicit)",
    );
    assert_eq!(item.report.partial.len(), 2, "{:?}", item.report);
    assert!(item.report.unparsed.is_empty());
    assert!(item.modifiers[0].fractured);
    assert_eq!(item.modifiers[1].kind, ModKind::Suffix);
    assert_eq!(item.report.partial[0].part, "(fractured)");
    assert_eq!(item.report.partial[1].part, "(implicit)");
}

#[test]
fn hybrid_modifier_keeps_both_lines() {
    let item = with_mods(
        "{ Prefix Modifier \"Reinforced\" (Tier: 3) — Defences, Armour, Life }\n+38(35-45) to Armour\n+11(10-14) to maximum Life\n{ Suffix Modifier \"of the Bear\" (Tier: 5) }\n+15(13-16) to Strength",
    );
    assert_eq!(item.modifiers.len(), 2);
    let m = &item.modifiers[0];
    assert_eq!(m.lines.len(), 2);
    assert_eq!(m.lines[0].template, "+# to Armour");
    assert_eq!(m.lines[1].template, "+# to maximum Life");
    assert_eq!(item.modifiers[1].lines.len(), 1);
}

#[test]
fn integer_value_with_range_keeps_a_leading_plus_as_text() {
    let item = with_mods("{ Prefix Modifier \"Hale\" (Tier: 4) }\n+14(11-15)% to Fire Resistance");
    let l = &item.modifiers[0].lines[0];
    assert_eq!(l.text, "+14(11-15)% to Fire Resistance");
    assert_eq!(l.template, "+#% to Fire Resistance");
    assert_eq!(l.values.len(), 1);
    assert_eq!(l.values[0].value, 14.0);
    assert_eq!(
        l.values[0].range,
        Some(Range {
            min: 11.0,
            max: 15.0
        })
    );
    assert!(!l.unscalable);
}

#[test]
fn decimal_values_and_ranges() {
    let item = with_mods(
        "{ Suffix Modifier \"of Acclaim\" (Tier: 5) }\n+1.1(0.9-1.2)% to Critical Hit Chance\n{ Prefix Modifier \"Vampiric\" (Tier: 2) }\nLeeches 6.5(5.5-6.9)% of Physical Damage as Life",
    );
    let a = &item.modifiers[0].lines[0];
    assert_eq!(a.template, "+#% to Critical Hit Chance");
    assert_eq!(a.values[0].value, 1.1);
    assert_eq!(a.values[0].range, Some(Range { min: 0.9, max: 1.2 }));
    let b = &item.modifiers[1].lines[0];
    assert_eq!(b.template, "Leeches #% of Physical Damage as Life");
    assert_eq!(b.values[0].value, 6.5);
}

#[test]
fn negative_values_and_ranges() {
    let item = with_mods(
        "{ Suffix Modifier \"of Doubt\" (Tier: 1) }\n-8(-10--6)% to Chaos Resistance\n{ Suffix Modifier \"of Mixed\" (Tier: 1) }\n5(-3-7)% increased Effect",
    );
    let a = &item.modifiers[0].lines[0];
    assert_eq!(a.template, "#% to Chaos Resistance");
    assert_eq!(a.values[0].value, -8.0);
    assert_eq!(
        a.values[0].range,
        Some(Range {
            min: -10.0,
            max: -6.0
        })
    );
    let b = &item.modifiers[1].lines[0];
    assert_eq!(
        b.values[0].range,
        Some(Range {
            min: -3.0,
            max: 7.0
        })
    );
    assert!(item.report.partial.is_empty());
}

#[test]
fn two_values_on_one_line_both_count() {
    let item = with_mods(
        "{ Prefix Modifier \"Glinting\" (Tier: 7) }\nAdds 3(2-4) to 7(6-9) Physical Damage\n--------\nAdds 3-5 Fire Damage",
    );
    let a = &item.modifiers[0].lines[0];
    assert_eq!(a.template, "Adds # to # Physical Damage");
    let got: Vec<(f64, Option<Range>)> = a.values.iter().map(|v| (v.value, v.range)).collect();
    assert_eq!(
        got,
        vec![
            (3.0, Some(Range { min: 2.0, max: 4.0 })),
            (7.0, Some(Range { min: 6.0, max: 9.0 }))
        ]
    );
    // Without ranges, `3-5` is two values separated by a literal dash.
    let short = with_mods("Adds 3-5 Fire Damage");
    let b = &short.modifiers[0].lines[0];
    assert_eq!(b.template, "Adds #-# Fire Damage");
    assert_eq!(b.values.len(), 2);
    assert_eq!((b.values[0].value, b.values[1].value), (3.0, 5.0));
}

#[test]
fn line_without_a_number_has_no_value() {
    let item = with_mods(
        "{ Suffix Modifier \"of Slumber\" (Tier: 1) }\nPlayers are cursed with Drowsiness",
    );
    let l = &item.modifiers[0].lines[0];
    assert!(l.values.is_empty());
    assert_eq!(l.template, "Players are cursed with Drowsiness");
    assert!(item.report.partial.is_empty());
}

#[test]
fn unscalable_value_is_flagged_and_left_out_of_the_template() {
    let item = with_mods("{ Implicit Modifier }\nHas 2 Charm Slots — Unscalable Value (implicit)");
    let l = &item.modifiers[0].lines[0];
    assert!(l.unscalable);
    assert_eq!(l.template, "Has # Charm Slots");
    assert_eq!(l.values[0].value, 2.0);
    assert_eq!(l.values[0].range, None);
    assert_eq!(l.markers, vec![Marker::Implicit]);
    assert!(item.report.partial.is_empty());
}

#[test]
fn paren_after_a_number_that_is_not_a_range_is_partly_understood() {
    let item = with_mods("{ Prefix Modifier \"Odd\" (Tier: 1) }\nGain 5(x) Charges");
    let l = &item.modifiers[0].lines[0];
    assert_eq!(l.template, "Gain #(x) Charges");
    assert_eq!(l.values.len(), 1);
    assert_eq!(l.values[0].range, None);
    assert_eq!(item.report.partial.len(), 1);
    assert_eq!(item.report.partial[0].part, "(x)");
}

#[test]
fn unknown_trailing_marker_stays_in_the_template_and_is_reported() {
    let item = with_mods("{ Prefix Modifier \"Hale\" (Tier: 1) }\n+10 to maximum Life (veiled)");
    let l = &item.modifiers[0].lines[0];
    assert_eq!(l.template, "+# to maximum Life (veiled)");
    assert!(l.markers.is_empty());
    assert_eq!(item.report.partial.len(), 1);
    assert_eq!(item.report.partial[0].part, "(veiled)");
}
