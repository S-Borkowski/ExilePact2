//! Isolation in the add odds (issue #35, docs/models/crafting-slots-spec.md
//! "Certain removals first"): when a pool's exclusions leave one outcome,
//! the add is certain whatever the weights, so a jewel (no spawn weights)
//! and a placeholder weight still show it. A pool the weights decide keeps
//! its percent.

use ch_data::{class_data_by_name, ClassData, Kind, Odds, Pool, PoolEntry, Tier};

fn class(name: &str) -> &'static ClassData {
    class_data_by_name(name).unwrap_or_else(|| panic!("no data for {name}"))
}

/// A jewel modifier's tiers on the Ruby: poe2db's placeholder weight 1 and
/// no spawn basis, so the weights decide nothing.
fn jewel_entry<'a>(jewels: &'a ClassData, id: &'a str) -> PoolEntry<'a> {
    let m = jewels
        .find_mod(id)
        .unwrap_or_else(|| panic!("Jewels has no {id}"));
    PoolEntry {
        id: &m.id,
        kind: m.kind,
        tiers: m.tiers_on("ruby").iter().collect(),
    }
}

const INCISION: &str = "prefix-chance-for-attack-hits-to-apply-incision";
const BLEEDING: &str = "prefix-chance-to-inflict-bleeding-on-hit";

fn percent(odds: &Odds) -> Option<f64> {
    odds.percent()
}

#[test]
fn a_pool_of_one_is_certain_without_weights() {
    let jewels = class("Jewels");
    let alone = Pool {
        sides: vec![Kind::Prefix],
        entries: vec![jewel_entry(jewels, INCISION)],
    };
    let weights: Vec<Option<u32>> = alone.entries[0].tiers.iter().map(|t| t.weight).collect();
    assert!(
        weights.iter().all(|w| *w == Some(1)),
        "the fixture's weights are placeholders: {weights:?}"
    );
    let odds = alone.odds(INCISION, 1);
    assert_eq!(percent(&odds), Some(100.0), "{odds:?}");

    // Negative control: a second modifier in the pool, and the placeholder
    // weight decides again (unknown).
    let two = Pool {
        sides: vec![Kind::Prefix],
        entries: vec![jewel_entry(jewels, INCISION), jewel_entry(jewels, BLEEDING)],
    };
    assert!(
        matches!(two.odds(INCISION, 1), Odds::Unknown { .. }),
        "{:?}",
        two.odds(INCISION, 1)
    );

    // A tier below the floor is not a hit: the pool is the target alone
    // but not every roll counts, so the weights decide.
    let life = class("Rings").find_mod("prefix-to-maximum-life").unwrap();
    let tiers: Vec<&Tier> = life.tiers_on("all").iter().collect();
    let worst = tiers.iter().map(|t| t.tier).max().unwrap();
    let life_alone = Pool {
        sides: vec![Kind::Prefix],
        entries: vec![PoolEntry {
            id: &life.id,
            kind: life.kind,
            tiers,
        }],
    };
    assert_eq!(percent(&life_alone.odds(&life.id, worst)), Some(100.0));
    let part = percent(&life_alone.odds(&life.id, 1)).expect("ring weights are known");
    assert!(part < 100.0, "{part}");
}

#[test]
fn a_pool_without_the_target_is_certain_not_to_hit() {
    let jewels = class("Jewels");
    let without = Pool {
        sides: vec![Kind::Prefix],
        entries: vec![jewel_entry(jewels, BLEEDING)],
    };
    // A missing weight in the pool no longer hides the miss.
    let mut missing = without.clone();
    let mut tier: Tier = (*missing.entries[0].tiers[0]).clone();
    tier.weight = None;
    let leaked: &'static Tier = Box::leak(Box::new(tier));
    missing.entries[0].tiers = vec![leaked];
    let odds = missing.odds(INCISION, 1);
    assert_eq!(percent(&odds), Some(0.0), "{odds:?}");
    match odds {
        Odds::Known { basis, .. } => assert!(basis.contains("not in the pool"), "{basis}"),
        other => panic!("expected known odds, got {other:?}"),
    }

    // Control: the target in that same pool, with the missing weight,
    // stays unknown.
    let with = Pool {
        sides: vec![Kind::Prefix],
        entries: vec![missing.entries[0].clone(), jewel_entry(jewels, INCISION)],
    };
    assert!(
        matches!(with.odds(INCISION, 1), Odds::Unknown { .. }),
        "{:?}",
        with.odds(INCISION, 1)
    );
}

/// Pinned before the change: a pool the weights decide keeps its percent,
/// for a target in it and for one left out.
#[test]
fn a_weighed_pool_keeps_its_percent_baseline() {
    let rings = class("Rings");
    let pool = rings.add_pool("all", 82, &[Kind::Prefix], &[], None);
    match pool.odds("prefix-to-maximum-life", 1) {
        Odds::Known {
            hits, total, basis, ..
        } => {
            assert_eq!((hits, total), (1000, 69500));
            assert_eq!(basis, "1000 of 69500 weight");
        }
        other => panic!("expected known odds, got {other:?}"),
    }
    match pool.odds("suffix-to-fire-resistance", 1) {
        Odds::Known {
            hits, total, basis, ..
        } => {
            assert_eq!((hits, total), (0, 69500));
            assert_eq!(basis, "0 of 69500 weight (the target is not in the pool)");
        }
        other => panic!("expected known odds, got {other:?}"),
    }
}
