//! The matcher, over tooltip lines built from the whittled ring's texts
//! (`ch-engine/tests/items/gear/rings-resist-whittle/`): each read is
//! classified against the item the step was shown on.

mod common;

use ch_engine::ch_data;
use ch_engine::ch_item::{Item, ModKind, Rarity};
use ch_watch::{classify, item_key, ItemKey, ReadKind, TooltipBox, TooltipRead, Unreadable};
use common::{parse, ring, ring_text, tooltip, Tip};

fn read_of(tips: &[&Tip]) -> TooltipRead {
    TooltipRead::Boxes(tips.iter().map(|t| t.boxed()).collect())
}

fn changed(kind: ReadKind) -> Item {
    match kind {
        ReadKind::Changed(item) => *item,
        other => panic!("not a change: {other:?}"),
    }
}

fn key(item: &Item) -> ItemKey {
    item_key(item).expect("every explicit modifier joins")
}

fn has(key: &ItemKey, id: &str, tier: u32) -> bool {
    key.mods.iter().any(|(m, t)| m == id && *t == tier)
}

/// The start ring with its Strength suffix swapped for `with` (a header and
/// its lines).
fn start_with_suffix(with: &str) -> String {
    let text = ring_text("01-start");
    let lion = "{ Suffix Modifier \"of the Lion\" (Tier: 5) — Attribute }\n+18(17-20) to Strength";
    assert!(text.contains(lion));
    text.replace(lion, with)
}

#[test]
fn plain_tooltip_lines_match_the_items_mods() {
    let steps = [
        ("01-start", "02-chaos-fire"),
        ("02-chaos-fire", "03-whittle-cold"),
        ("03-whittle-cold", "05-revealed-lightning"),
        ("02-chaos-fire", "06-chaos-below-floor"),
        ("05-revealed-lightning", "07-revealed-wrong"),
    ];
    for (before, after) in steps {
        let tip = tooltip(&ring_text(after), false);
        let read = changed(classify(&read_of(&[&tip]), &ring(before)));
        assert_eq!(key(&read), key(&ring(after)), "{before} -> {after}");
    }
    // The plain form has no header to lean on: the sides and tiers come from
    // the templates and the values alone.
    let tip = tooltip(&ring_text("02-chaos-fire"), false);
    assert!(tip
        .lines
        .iter()
        .all(|l| !l.contains("Modifier") && !l.contains('(')));
    let read = changed(classify(&read_of(&[&tip]), &ring("01-start")));
    let k = key(&read);
    assert!(has(&k, "prefix-to-maximum-mana", 5), "{k:?}");
    assert!(has(&k, "suffix-to-fire-resistance", 3), "{k:?}");
    assert_eq!(k.mods.len(), 2);
}

#[test]
fn alt_tooltip_headers_give_side_and_tier() {
    // 13% increased Rarity fits a prefix tier and a suffix tier alike: the
    // plain form cannot tell them apart, the Alt form's header can.
    let raiding = start_with_suffix(
        "{ Suffix Modifier \"of Raiding\" (Tier: 2) — Drop }\n13(11-14)% increased Rarity of Items found",
    );
    let start = ring("01-start");
    assert_eq!(
        classify(&read_of(&[&tooltip(&raiding, false)]), &start),
        ReadKind::Unreadable(Unreadable::Ambiguous)
    );
    let read = changed(classify(&read_of(&[&tooltip(&raiding, true)]), &start));
    assert!(has(
        &key(&read),
        "suffix-increased-rarity-of-items-found",
        2
    ));
    assert_eq!(key(&read), key(&parse(&raiding)));

    let collector = raiding.replace(
        "Suffix Modifier \"of Raiding\" (Tier: 2) — Drop",
        "Prefix Modifier \"Collector's\" (Tier: 2) — Drop",
    );
    let read = changed(classify(&read_of(&[&tooltip(&collector, true)]), &start));
    assert!(has(
        &key(&read),
        "prefix-increased-rarity-of-items-found",
        2
    ));
    let side = read
        .modifiers
        .iter()
        .find(|m| m.affix_name.as_deref() == Some("Collector's"))
        .map(|m| m.kind);
    assert_eq!(side, Some(ModKind::Prefix));

    // A header's tier and the values agree: the revealed desecrated suffix
    // reads as its tier.
    let alt = tooltip(&ring_text("05-revealed-lightning"), true);
    let read = changed(classify(&read_of(&[&alt]), &ring("03-whittle-cold")));
    assert_eq!(key(&read), key(&ring("05-revealed-lightning")));

    // What an unrevealed modifier shows is not established yet: unreadable.
    let alt = tooltip(&ring_text("04-desecrated"), true);
    assert_eq!(
        classify(&read_of(&[&alt]), &ring("03-whittle-cold")),
        ReadKind::Unreadable(Unreadable::NotEstablished)
    );
}

#[test]
fn ocr_confusions_are_tolerated() {
    let tip = tooltip(&ring_text("02-chaos-fire"), false)
        .replace("Doom Loop", "D00m Loop")
        .replace("Amethyst Ring", "Amethyst R1ng")
        .replace("Requires: Level 20", "REQUIRES: Level 2O")
        .replace("Item Level: 76", "ltem Level:76")
        .replace("+7% to Chaos Resistance", "7% to Chaos Resistance.")
        .replace("+95 to maximum Mana", "+95 to rnaximum  Mana")
        .replace("+33% to Fire Resistance", "+3l to F1re resistance");
    let read = changed(classify(&read_of(&[&tip]), &ring("01-start")));
    assert_eq!(key(&read), key(&ring("02-chaos-fire")));
    let fire = read
        .modifiers
        .iter()
        .find(|m| m.affix_name.as_deref() == Some("of the Volcano"))
        .expect("the fire suffix");
    assert_eq!(fire.lines[0].values[0].value, 31.0);

    // Tolerance stops short of a different modifier: Cold is not Fire.
    let cold = tooltip(&ring_text("02-chaos-fire"), false)
        .replace("+33% to Fire Resistance", "+33% to Cold Resistance");
    let read = changed(classify(&read_of(&[&cold]), &ring("01-start")));
    assert!(has(&key(&read), "suffix-to-cold-resistance", 3));
}

#[test]
fn hybrid_mod_lines_match_one_modifier() {
    let hearth = start_with_suffix(
        "{ Suffix Modifier \"of the Hearth\" (Tier: 1) — Mana }\n18(18-22)% increased Mana Regeneration Rate\n15(15-15)% increased Light Radius",
    );
    let hybrid = parse(&hearth);
    assert!(has(
        &key(&hybrid),
        "suffix-increased-mana-regeneration-rate-2",
        1
    ));
    let tip = tooltip(&hearth, false);
    let read = changed(classify(&read_of(&[&tip]), &ring("01-start")));
    assert_eq!(key(&read), key(&hybrid));
    let suffix = read
        .modifiers
        .iter()
        .find(|m| m.kind == ModKind::Suffix)
        .expect("the hybrid suffix");
    assert_eq!(suffix.lines.len(), 2);
    assert_eq!(classify(&read_of(&[&tip]), &hybrid), ReadKind::Unchanged);
}

#[test]
fn unchanged_item_reads_as_unchanged() {
    for name in ["01-start", "02-chaos-fire", "08-whittle-lightning"] {
        let item = ring(name);
        for alt in [false, true] {
            let tip = tooltip(&ring_text(name), alt);
            assert_eq!(
                classify(&read_of(&[&tip]), &item),
                ReadKind::Unchanged,
                "{name} alt={alt}"
            );
        }
    }
    // Ids, tiers and rarity decide, not the desecrated mark: the whittled
    // ring after 05's reveal reads the same.
    let tip = tooltip(&ring_text("08-whittle-lightning"), false);
    assert_eq!(
        classify(&read_of(&[&tip]), &ring("05-revealed-lightning")),
        ReadKind::Unchanged
    );
}

#[test]
fn another_base_reads_as_another_item() {
    let start = ring("01-start");
    let ruby = tooltip(&ring_text("01-start"), false).replace("Amethyst Ring", "Ruby Ring");
    assert_eq!(classify(&read_of(&[&ruby]), &start), ReadKind::AnotherItem);

    let orb = TooltipBox::from_text(
        &["Exalted Orb"],
        &[
            "Stack Size: 3/20",
            "Augments a Rare item with a new random modifier",
        ],
        Some(Rarity::Other("Currency".into())),
    );
    let currency = TooltipRead::Boxes(vec![orb.clone()]);
    assert_eq!(classify(&currency, &start), ReadKind::AnotherItem);

    // Beside another item's tooltip, the item's own is read.
    let fire = tooltip(&ring_text("02-chaos-fire"), false);
    let both = TooltipRead::Boxes(vec![orb, fire.boxed()]);
    let read = changed(classify(&both, &start));
    assert_eq!(key(&read), key(&ring("02-chaos-fire")));

    // Two tooltips that both carry the base: which is the item is not known.
    let two = read_of(&[&fire, &tooltip(&ring_text("01-start"), false)]);
    assert_eq!(
        classify(&two, &start),
        ReadKind::Unreadable(Unreadable::SeveralTooltips)
    );

    // An item with no base type cannot be matched at all.
    let mut no_base = start.clone();
    no_base.base_type = None;
    assert_eq!(
        classify(&read_of(&[&fire]), &no_base),
        ReadKind::Unreadable(Unreadable::NoBaseType)
    );

    // No tooltip, a black copy and the overlay are their own answers.
    assert_eq!(
        classify(&TooltipRead::NoTooltip, &start),
        ReadKind::NoTooltip
    );
    assert_eq!(classify(&TooltipRead::Black, &start), ReadKind::NoTooltip);
    assert_eq!(classify(&TooltipRead::Covered, &start), ReadKind::Covered);
}

#[test]
fn an_unaccounted_line_makes_the_read_unreadable() {
    let start = ring("01-start");
    let fire = tooltip(&ring_text("02-chaos-fire"), false);
    assert!(matches!(
        classify(&read_of(&[&fire]), &start),
        ReadKind::Changed(_)
    ));

    let mut junk = fire.clone();
    junk.lines.push("Grants Skill: Level 5 Fireball".into());
    assert_eq!(
        classify(&read_of(&[&junk]), &start),
        ReadKind::Unreadable(Unreadable::UnaccountedLine)
    );

    // A known template with a value no tier holds is not accounted for.
    let too_high = fire
        .clone()
        .replace("+33% to Fire Resistance", "+80% to Fire Resistance");
    assert_eq!(
        classify(&read_of(&[&too_high]), &start),
        ReadKind::Unreadable(Unreadable::UnaccountedLine)
    );

    // A cut tooltip is never trusted.
    let mut cut = fire.boxed();
    cut.touches_edge = true;
    assert_eq!(
        classify(&TooltipRead::Boxes(vec![cut]), &start),
        ReadKind::Unreadable(Unreadable::Cut)
    );

    // A missing implicit line is not taken for a lost modifier.
    let mut no_implicit = fire;
    no_implicit.lines.retain(|l| l != "+7% to Chaos Resistance");
    assert_eq!(
        classify(&read_of(&[&no_implicit]), &start),
        ReadKind::Unreadable(Unreadable::MissingLine)
    );
}

#[test]
fn an_ambiguous_tier_makes_the_read_unreadable() {
    let start = ring("01-start");
    let base = tooltip(&ring_text("01-start"), false);
    // 11 to 20 lies in both tier 2 (10-15 to 18-26) and tier 3 (7-11 to
    // 14-20).
    let both = base.clone().replace(
        "+95 to maximum Mana",
        "Adds 11 to 20 Physical Damage to Attacks",
    );
    assert_eq!(
        classify(&read_of(&[&both]), &start),
        ReadKind::Unreadable(Unreadable::AmbiguousTier)
    );
    // 16 to 30 lies in tier 1 alone.
    let one = base.replace(
        "+95 to maximum Mana",
        "Adds 16 to 30 Physical Damage to Attacks",
    );
    let read = changed(classify(&read_of(&[&one]), &start));
    assert!(has(
        &key(&read),
        "prefix-adds-to-physical-damage-to-attacks",
        1
    ));
}

#[test]
fn rarity_is_read_or_the_read_is_unreadable() {
    let start = ring("01-start");
    let fire = tooltip(&ring_text("02-chaos-fire"), false);

    // No colour read.
    let mut unknown = fire.boxed();
    unknown.rarity = None;
    assert_eq!(
        classify(&TooltipRead::Boxes(vec![unknown]), &start),
        ReadKind::Unreadable(Unreadable::Rarity)
    );

    // A rare's colour with one name line.
    let mut one_line = fire.clone();
    one_line.name_lines.remove(0);
    assert_eq!(
        classify(&read_of(&[&one_line]), &start),
        ReadKind::Unreadable(Unreadable::Rarity)
    );

    // A magic item: one name line holding the base, one prefix, one suffix.
    let magic = Tip {
        rarity: Rarity::Magic,
        name_lines: vec!["Gentian Amethyst Ring of the Volcano".into()],
        lines: fire.lines.clone(),
    };
    let read = changed(classify(&read_of(&[&magic]), &start));
    assert_eq!(read.rarity, Rarity::Magic);
    assert_eq!(key(&read).rarity, Rarity::Magic);
    assert_eq!(key(&read).mods, key(&ring("02-chaos-fire")).mods);

    // A magic item with two suffixes, a normal item with a modifier: not
    // what the colour says.
    let mut two_suffixes = magic.clone();
    two_suffixes.lines.push("+32% to Cold Resistance".into());
    assert_eq!(
        classify(&read_of(&[&two_suffixes]), &start),
        ReadKind::Unreadable(Unreadable::Rarity)
    );
    let normal_with_mods = Tip {
        rarity: Rarity::Normal,
        name_lines: vec!["Amethyst Ring".into()],
        lines: fire.lines.clone(),
    };
    assert_eq!(
        classify(&read_of(&[&normal_with_mods]), &start),
        ReadKind::Unreadable(Unreadable::Rarity)
    );

    // A normal item with no explicit modifier reads as normal.
    let normal = Tip {
        rarity: Rarity::Normal,
        name_lines: vec!["Amethyst Ring".into()],
        lines: fire.lines[..3].to_vec(),
    };
    let read = changed(classify(&read_of(&[&normal]), &start));
    assert_eq!(read.rarity, Rarity::Normal);
    assert!(key(&read).mods.is_empty());
}

#[test]
fn the_read_item_joins_to_the_read_mods() {
    let start = ring("01-start");
    let after = ring("02-chaos-fire");
    let tip = tooltip(&ring_text("02-chaos-fire"), false);
    let read = changed(classify(&read_of(&[&tip]), &start));

    let joined = ch_data::join(&read);
    assert_eq!(joined.variant, Ok("all".to_string()));
    let mut ids: Vec<(String, u32)> = joined
        .mods
        .iter()
        .map(|m| {
            let found = m.matched().expect("every read modifier joins");
            (found.mod_id.clone(), found.tier)
        })
        .collect();
    ids.sort();
    assert_eq!(ids, key(&after).mods);

    // Every other field is the step's item's.
    assert_eq!(read.rarity, Rarity::Rare);
    assert_eq!(read.base_type, start.base_type);
    assert_eq!(read.name, start.name);
    assert_eq!(read.item_level, start.item_level);
    assert_eq!(read.requirements, start.requirements);
    let implicit = |item: &Item| {
        item.modifiers
            .iter()
            .filter(|m| m.kind == ModKind::Implicit)
            .cloned()
            .collect::<Vec<_>>()
    };
    assert_eq!(implicit(&read), implicit(&start));

    // The lines are the data's templates with the read values.
    let texts: Vec<&str> = read
        .modifiers
        .iter()
        .filter(|m| matches!(m.kind, ModKind::Prefix | ModKind::Suffix))
        .flat_map(|m| m.lines.iter().map(|l| l.text.as_str()))
        .collect();
    assert_eq!(texts, ["+95 to maximum Mana", "+33% to Fire Resistance"]);
}
