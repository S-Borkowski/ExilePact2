//! A hybrid modifier is one modifier and takes one slot: the constructed
//! crossbow in `ch-item`'s `fixtures/constructed/` carries the hybrid
//! `#% increased Physical Damage` with Accuracy beside the plain
//! `#% increased Physical Damage` prefix (a different family) and a flat
//! physical one, read in place.

use std::fs;
use std::path::PathBuf;

use ch_data::{item_affix_limits, join, AffixLimits, JoinResult, Kind, TierRule};
use ch_item::{Item, Rarity};

fn crossbow() -> Item {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("ch-item")
        .join("fixtures")
        .join("constructed")
        .join("crossbows-rare-hybrid-physical.txt");
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    ch_item::parse(&text).unwrap_or_else(|e| panic!("parse: {e}"))
}

#[test]
fn a_hybrid_prefix_joins_as_one_modifier_beside_its_plain_family() {
    let item = crossbow();
    let joined = join(&item);
    assert_eq!(joined.variant.as_deref(), Ok("all"));
    // One joined entry per header: six, not seven.
    assert_eq!(joined.mods.len(), 6);
    let mut prefixes = Vec::new();
    for m in &joined.mods {
        let found = match &m.result {
            JoinResult::Matched(found) => found,
            JoinResult::Unmatched { reason } => {
                panic!("{:?} did not join: {reason:?}", m.affix_name)
            }
        };
        assert_eq!(found.decided_by, TierRule::AffixName, "{:?}", m.affix_name);
        assert_eq!(Some(found.tier), m.game_tier, "{:?}", m.affix_name);
        if m.kind == Kind::Prefix {
            prefixes.push((
                found.mod_id.as_str(),
                found.family.clone(),
                found.tier,
                item.modifiers[m.index].lines.len(),
            ));
        }
    }
    assert_eq!(
        prefixes,
        vec![
            (
                "prefix-increased-physical-damage",
                vec!["LocalPhysicalDamagePercent".to_string()],
                5,
                1
            ),
            (
                "prefix-increased-physical-damage-2",
                vec!["LocalIncreasedPhysicalDamagePercentAndAccuracyRating".to_string()],
                6,
                2
            ),
            (
                "prefix-adds-to-physical-damage",
                vec!["PhysicalDamage".to_string()],
                3,
                1
            ),
        ]
    );
    // The two Physical Damage modifiers share a line, not a family: both may
    // sit on one item.
    assert_ne!(prefixes[0].1, prefixes[1].1);
}

#[test]
fn a_hybrid_prefix_takes_one_prefix_slot() {
    let item = crossbow();
    let limits = item_affix_limits(&item, &Rarity::Rare).expect("crossbow limits");
    assert_eq!(
        limits,
        AffixLimits {
            prefixes: 3,
            suffixes: 3
        }
    );
    // Three prefix headers, four prefix lines: the hybrid's two lines count
    // once, so the three prefixes fill the side and leave no free prefix.
    let prefix_lines: usize = item
        .modifiers
        .iter()
        .filter(|m| Kind::from_item(m.kind) == Some(Kind::Prefix))
        .map(|m| m.lines.len())
        .sum();
    assert_eq!(prefix_lines, 4);
    assert_eq!(item.prefix_count(), 3);
    assert_eq!(limits.on(Kind::Prefix) - item.prefix_count(), 0);
    assert_eq!(limits.on(Kind::Suffix) - item.suffix_count(), 0);
    let joined_prefixes = join(&item)
        .mods
        .iter()
        .filter(|m| m.kind == Kind::Prefix)
        .count();
    assert_eq!(joined_prefixes, limits.on(Kind::Prefix));
}
