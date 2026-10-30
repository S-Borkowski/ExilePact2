//! The item-level breakpoints the community PoE2 Crafting Codex gives
//! (`https://domistae.github.io/poe2-leveling/poe2_crafting_codex.html#breakpoints`,
//! read 2026-10-30, issue #40), checked against the embedded data
//! (2026-10-28 extraction) and the trade link's item-level floor
//! (`suggested_item_level`).
//!
//! Five of the codex's six breakpoints agree with the data. The sixth,
//! movement speed's second tier, is 70 in the codex and 65 in the data; the
//! data is kept (`docs/crafting-helper/crafting-mechanics.md`, "Item-level
//! breakpoints"). Nothing here changes the data: these tests make a data
//! refresh that moves one of the six fail loudly instead of quietly changing
//! what the helper tells a player to buy.

use ch_data::{class_data_by_name, suggested_item_level, ClassData, TradePick};

/// One breakpoint the codex states and the data agrees with: on `class`,
/// tier `tier` of `id` needs item level `level`.
struct Breakpoint {
    class: &'static str,
    id: &'static str,
    tier: u32,
    level: u32,
}

const fn bp(class: &'static str, id: &'static str, tier: u32, level: u32) -> Breakpoint {
    Breakpoint {
        class,
        id,
        tier,
        level,
    }
}

const MOVEMENT_SPEED: &str = "prefix-increased-movement-speed";

/// The codex's agreeing rows: movement speed's top tier on boots; the top
/// fire, cold and lightning resistance tiers on boots, rings and body
/// armours; body armour's top chaos resistance tier; a wand's top lightning
/// spell level tier; a bow's top additional-arrow tier.
const AGREEING: &[Breakpoint] = &[
    bp("Boots", MOVEMENT_SPEED, 1, 82),
    bp("Boots", "suffix-to-fire-resistance", 1, 82),
    bp("Boots", "suffix-to-cold-resistance", 1, 82),
    bp("Boots", "suffix-to-lightning-resistance", 1, 82),
    bp("Rings", "suffix-to-fire-resistance", 1, 82),
    bp("Rings", "suffix-to-cold-resistance", 1, 82),
    bp("Rings", "suffix-to-lightning-resistance", 1, 82),
    bp("Body Armours", "suffix-to-fire-resistance", 1, 82),
    bp("Body Armours", "suffix-to-cold-resistance", 1, 82),
    bp("Body Armours", "suffix-to-lightning-resistance", 1, 82),
    bp("Body Armours", "suffix-to-chaos-resistance", 1, 81),
    bp(
        "Wands",
        "suffix-to-level-of-all-lightning-spell-skills",
        1,
        81,
    ),
    bp(
        "Bows",
        "suffix-surpassing-chance-to-fire-an-additional-arrow",
        1,
        82,
    ),
];

/// Movement speed's second tier: the codex's 70, and the data's 65 that the
/// helper keeps.
const CODEX_MOVEMENT_SPEED_T2: u32 = 70;
const DATA_MOVEMENT_SPEED_T2: u32 = 65;

fn class(name: &str) -> &'static ClassData {
    class_data_by_name(name).unwrap_or_else(|| panic!("no data for {name}"))
}

/// The level of tier `tier` of `id` on every variant of `class` that has the
/// modifier, with the variant's name. Panics when no variant has it, so a
/// renamed modifier fails here rather than passing on an empty list.
fn tier_levels(class_name: &str, id: &str, tier: u32) -> Vec<(String, u32)> {
    let data = class(class_name);
    let m = data
        .find_mod(id)
        .unwrap_or_else(|| panic!("{class_name} has no modifier {id}"));
    let levels: Vec<(String, u32)> = m
        .tiers
        .iter()
        .filter_map(|(variant, tiers)| {
            tiers
                .iter()
                .find(|t| t.tier == tier)
                .map(|t| (variant.clone(), t.ilvl))
        })
        .collect();
    assert!(
        !levels.is_empty(),
        "{class_name} {id}: no variant has T{tier}"
    );
    levels
}

/// One base of `class` per variant on which `id` has tiers.
fn bases_with(class_name: &str, id: &str) -> Vec<&'static str> {
    let data = class(class_name);
    let m = data.find_mod(id).expect("modifier");
    let mut seen = Vec::new();
    let mut out = Vec::new();
    for base in &data.bases {
        if !m.tiers_on(&base.variant).is_empty() && !seen.contains(&base.variant) {
            seen.push(base.variant.clone());
            out.push(base.name.as_str());
        }
    }
    assert!(!out.is_empty(), "{class_name}: no base offers {id}");
    out
}

fn floor(id: &str, tier: u32) -> Vec<TradePick> {
    vec![TradePick {
        id: id.to_string(),
        floor: Some(tier),
    }]
}

/// Each agreeing row's tier sits at the codex's level on every variant of
/// its class.
#[test]
fn codex_breakpoints_agree_with_the_data() {
    for row in AGREEING {
        for (variant, level) in tier_levels(row.class, row.id, row.tier) {
            assert_eq!(
                level, row.level,
                "{} {} T{} on {variant}: data {level}, codex {}",
                row.class, row.id, row.tier, row.level
            );
        }
    }
    // Negative control: the check reads the tier it names. Fire resistance's
    // second tier on rings is not the top tier's 82.
    let second = tier_levels("Rings", "suffix-to-fire-resistance", 2);
    assert!(second.iter().all(|(_, level)| *level != 82), "{second:?}");
}

/// The one breakpoint the codex and the data disagree on: movement speed's
/// second tier stays at the data's 65 on every boots variant.
#[test]
fn movement_speed_second_tier_keeps_the_data_level() {
    for (variant, level) in tier_levels("Boots", MOVEMENT_SPEED, 2) {
        assert_eq!(level, DATA_MOVEMENT_SPEED_T2, "boots {variant}");
        assert_ne!(level, CODEX_MOVEMENT_SPEED_T2, "boots {variant}");
    }
}

/// The trade link's item-level floor for a single pick at each breakpoint's
/// tier is the breakpoint's level, on one base of every variant of the
/// class; movement speed's second tier asks for the data's 65.
#[test]
fn the_trade_floor_follows_the_codex_breakpoints() {
    for row in AGREEING {
        for base in bases_with(row.class, row.id) {
            assert_eq!(
                suggested_item_level(row.class, base, &floor(row.id, row.tier)),
                Some(row.level),
                "{} {base} {} T{}",
                row.class,
                row.id,
                row.tier
            );
        }
    }
    for base in bases_with("Boots", MOVEMENT_SPEED) {
        assert_eq!(
            suggested_item_level("Boots", base, &floor(MOVEMENT_SPEED, 2)),
            Some(DATA_MOVEMENT_SPEED_T2),
            "{base}"
        );
    }
    // Negative controls: no floor asks for no item level, and a base the
    // class does not have suggests nothing.
    let any_tier = [TradePick {
        id: MOVEMENT_SPEED.to_string(),
        floor: None,
    }];
    assert_eq!(
        suggested_item_level("Boots", "Bangled Sandals", &any_tier),
        None
    );
    assert_eq!(
        suggested_item_level("Boots", "No Such Boots", &floor(MOVEMENT_SPEED, 1)),
        None
    );
}
