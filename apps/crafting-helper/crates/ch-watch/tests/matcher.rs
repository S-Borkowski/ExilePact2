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

/// `ch-item`'s constructed crossbow with a hybrid Physical Damage and
/// Accuracy prefix beside the plain Physical Damage prefix, LF endings.
fn hybrid_crossbow_text() -> String {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../ch-item/fixtures/constructed/crossbows-rare-hybrid-physical.txt"
    );
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    text.replace("\r\n", "\n")
}

/// The plain tooltip of a crossbow text with the plain and the hybrid
/// `#% increased Physical Damage` lines shown as one line of their sum, in
/// the plain one's place, and the hybrid's Accuracy line on its own.
fn summed_tooltip(text: &str, plain: &str, hybrid: &str, sum: &str) -> Tip {
    let mut tip = tooltip(text, false);
    let at = tip.lines.iter().position(|l| l == plain).expect("plain line");
    tip.lines[at] = sum.to_string();
    let hybrid_at = tip
        .lines
        .iter()
        .position(|l| l == hybrid)
        .expect("hybrid line");
    tip.lines.remove(hybrid_at);
    tip
}

#[test]
fn a_summed_hybrid_line_never_reads_as_another_item() {
    // What the plain tooltip shows for two modifiers with the same line is
    // not measured; if it sums them, the sum (92 + 28) is a tier of the plain
    // modifier and the hybrid's Accuracy alone a tier of the plain Accuracy
    // prefix, three prefixes the item could hold. That reading is not the
    // item: unreadable, never a change.
    let text = hybrid_crossbow_text();
    let crossbow = parse(&text);
    let own = key(&crossbow);
    assert!(has(&own, "prefix-increased-physical-damage", 5), "{own:?}");
    assert!(has(&own, "prefix-increased-physical-damage-2", 6), "{own:?}");

    // Positive control: each modifier on its own line reads unchanged.
    let separate = tooltip(&text, false);
    assert_eq!(
        classify(&read_of(&[&separate]), &crossbow),
        ReadKind::Unchanged
    );

    let summed = summed_tooltip(
        &text,
        "92% increased Physical Damage",
        "28% increased Physical Damage",
        "120% increased Physical Damage",
    );
    assert!(summed.lines.contains(&"+55 to Accuracy Rating".to_string()));
    // The same lines, read as an item of their own, are a different item:
    // the summed line is the plain prefix's tier 4, the Accuracy line the
    // Accuracy prefix's tier 9.
    let read = classify(&read_of(&[&summed]), &crossbow);
    assert!(
        matches!(read, ReadKind::Unreadable(_)),
        "summed line read as {read:?}"
    );

    // A sum past every plain tier (154 + 28) is unreadable too.
    let high = text.replace(
        "{ Prefix Modifier \"Vicious\" (Tier: 5) — Damage, Physical, Attack }\n92(85-109)%",
        "{ Prefix Modifier \"Cruel\" (Tier: 3) — Damage, Physical, Attack }\n154(135-154)%",
    );
    assert_ne!(high, text);
    let high_item = parse(&high);
    let summed = summed_tooltip(
        &high,
        "154% increased Physical Damage",
        "28% increased Physical Damage",
        "182% increased Physical Damage",
    );
    let read = classify(&read_of(&[&summed]), &high_item);
    assert!(
        matches!(read, ReadKind::Unreadable(_)),
        "summed line read as {read:?}"
    );
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
fn an_item_whose_key_does_not_join_never_reads_as_a_change() {
    // The start ring with its Strength suffix swapped for one the data does
    // not hold: the item has no key, so no read can say it changed.
    let unknown = parse(&start_with_suffix(
        "{ Suffix Modifier \"of Nowhere\" (Tier: 5) — Attribute }\n+18(17-20) to Nonsense",
    ));
    assert!(!unknown.has_unrevealed() && !unknown.has_fractured());
    assert_eq!(item_key(&unknown), None);
    for after in ["01-start", "02-chaos-fire"] {
        for alt in [false, true] {
            let tip = tooltip(&ring_text(after), alt);
            assert_eq!(
                classify(&read_of(&[&tip]), &unknown),
                ReadKind::Unreadable(Unreadable::UnknownModifier),
                "{after} alt={alt}"
            );
        }
    }
    // Positive control: the same reads against the start ring itself.
    let start = ring("01-start");
    let tip = tooltip(&ring_text("01-start"), false);
    assert_eq!(classify(&read_of(&[&tip]), &start), ReadKind::Unchanged);
    let tip = tooltip(&ring_text("02-chaos-fire"), false);
    assert_eq!(
        key(&changed(classify(&read_of(&[&tip]), &start))),
        key(&ring("02-chaos-fire"))
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

#[test]
fn measured_tooltip_lines_are_accounted_for() {
    // What the owner's screenshots show besides the item text (2026-10-29):
    // the class above the requirements, and the key hints the game draws
    // over the box, OCR spacing their small letters.
    let start = ring("01-start");
    let mut tip = tooltip(&ring_text("01-start"), false);
    tip.lines.insert(0, "RING".to_string());
    tip.lines.push("ALT INSPECT".to_string());
    tip.lines.push("P R t C E CHEC K".to_string());
    assert_eq!(classify(&read_of(&[&tip]), &start), ReadKind::Unchanged);

    // A line that only looks like a hint is still unaccounted for.
    let mut odd = tip.clone();
    odd.lines.push("ALT INSPECTION OF RINGS".to_string());
    assert_eq!(
        classify(&read_of(&[&odd]), &start),
        ReadKind::Unreadable(Unreadable::UnaccountedLine)
    );

    // The note under an unrevealed modifier, whose own line OCR does not
    // read: never read as the item without it.
    let mut veiled = tip.clone();
    veiled
        .lines
        .push("TAKE THIS ITEM TO THE WELL OF SOULS TO REVEAL THE DESECRATED MODIFIER".to_string());
    assert_eq!(
        classify(&read_of(&[&veiled]), &start),
        ReadKind::Unreadable(Unreadable::NotEstablished)
    );
    // Nor is an item that has one, whatever its tooltip shows.
    let plain = tooltip(&ring_text("04-desecrated"), false);
    assert_eq!(
        classify(&read_of(&[&plain]), &ring("04-desecrated")),
        ReadKind::Unreadable(Unreadable::NotEstablished)
    );
}

#[test]
fn a_belt_with_charm_slots_and_a_flask_implicit_reads_unchanged() {
    // A rare belt of the shape Live 1 could not read (2026-10-29): a charm
    // slot block, a requirement, a flask implicit, two prefixes and a
    // suffix. Its name and values are its own.
    let text = "Item Class: Belts\n\
        Rarity: Rare\n\
        Bramble Clasp\n\
        Utility Belt\n\
        --------\n\
        Charm Slots: 1\n\
        --------\n\
        Requires: Level 55\n\
        --------\n\
        Item Level: 79\n\
        --------\n\
        { Implicit Modifier }\n\
        20% of Flask Recovery applied Instantly\n\
        --------\n\
        { Prefix Modifier \"Athlete's\" (Tier: 2) — Life }\n\
        +131(120-149) to maximum Life\n\
        { Prefix Modifier \"Cobalt\" (Tier: 8) — Mana }\n\
        +19(15-24) to maximum Mana\n\
        { Suffix Modifier \"of the Goliath\" (Tier: 4) — Attribute }\n\
        +26(25-27) to Strength\n";
    let belt = parse(text);
    let k = key(&belt);
    assert!(has(&k, "prefix-to-maximum-life", 2), "{k:?}");
    assert!(has(&k, "prefix-to-maximum-mana", 8), "{k:?}");
    assert!(has(&k, "suffix-to-strength", 4), "{k:?}");

    // The plain tooltip names the class above the charm slots, as the shot
    // shows it.
    let mut tip = tooltip(text, false);
    tip.lines.insert(0, "BELT".to_string());
    assert_eq!(classify(&read_of(&[&tip]), &belt), ReadKind::Unchanged);
    assert_eq!(
        classify(&read_of(&[&tooltip(text, true)]), &belt),
        ReadKind::Unchanged
    );

    // Shot 13's requirement line as OCR read it over the stash (measured,
    // 2026-10-29): its first word split in two and mangled, the colon, the
    // level and its value intact. The part after the colon says what it is.
    let mangled = tip
        .clone()
        .replace("Requires: Level 55", "RQUEl S: LEVEL 55");
    assert_eq!(classify(&read_of(&[&mangled]), &belt), ReadKind::Unchanged);
    let with_attributes = tip
        .clone()
        .replace("Requires: Level 55", "RQUEl S: Level 55, 40 Str, 12 Int");
    assert_eq!(
        classify(&read_of(&[&with_attributes]), &belt),
        ReadKind::Unchanged
    );
    // Without the colon, or with more than a level and attributes after it,
    // the line is still unaccounted for.
    for odd in [
        "RQUEl S LEVEL 55",
        "RQUEl S: LEVEL 55 of all Spell Skills",
        "RQUEl S: 55",
    ] {
        let line = tip.clone().replace("Requires: Level 55", odd);
        assert_eq!(
            classify(&read_of(&[&line]), &belt),
            ReadKind::Unreadable(Unreadable::UnaccountedLine),
            "{odd}"
        );
    }

    // Negative control: one more Strength reads as the next tier up, a change.
    let stronger = tip.clone().replace("+26 to Strength", "+31 to Strength");
    let read = changed(classify(&read_of(&[&stronger]), &belt));
    assert!(has(&key(&read), "suffix-to-strength", 2));
}
