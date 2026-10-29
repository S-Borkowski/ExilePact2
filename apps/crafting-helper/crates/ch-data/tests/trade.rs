//! The trade module against the embedded data (2026-10-28 extraction) and
//! the embedded trade stat mapping (`data/trade-stats.json`, 2026-10-30):
//! start weights, the modifiers a base offers, and the trade query.

use ch_data::{
    class_data_by_name, start_weight, trade_offer, trade_query, trade_stats, ClassData, Kind, Odds,
    StartWeight, TradePick, TradeQuery, TradeSearch, WeightSource, MAX_TRADE_PICKS,
};
use serde_json::{json, Value};

fn class(name: &str) -> &'static ClassData {
    class_data_by_name(name).unwrap_or_else(|| panic!("no data for {name}"))
}

fn weight(
    class_name: &str,
    variant: &str,
    level: Option<u32>,
    id: &str,
    floor: Option<u32>,
) -> StartWeight {
    start_weight(class(class_name), variant, level, id, floor)
        .unwrap_or_else(|why| panic!("{class_name} {id}: {why}"))
}

fn pick(id: &str, floor: Option<u32>) -> TradePick {
    TradePick {
        id: id.to_string(),
        floor,
    }
}

fn search(class: &str, base: &str, item_level: Option<u32>, picks: Vec<TradePick>) -> TradeSearch {
    TradeSearch {
        class: class.to_string(),
        base: base.to_string(),
        item_level,
        picks,
        both_slots: true,
    }
}

fn query(s: &TradeSearch) -> TradeQuery {
    trade_query(s).unwrap_or_else(|why| panic!("refused: {why}"))
}

fn group(q: &TradeQuery, index: usize) -> &Value {
    &q.query["query"]["stats"][index]
}

fn filter_ids(group: &Value) -> Vec<&str> {
    group["filters"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["id"].as_str().unwrap())
        .collect()
}

const RING_LIFE: &str = "prefix-to-maximum-life";
const RING_ACCURACY: &str = "prefix-to-accuracy-rating";
const RING_COLD_RES: &str = "suffix-to-cold-resistance";

// Baseline -----------------------------------------------------------------

/// Pinned before the trade module existed: the Rings `all` prefix pool at
/// item level 82 weighs 69500, and maximum Life at tier 1 is 1000 of it. The
/// trade module reads this pool and must leave it as it was.
#[test]
fn trade_baseline_add_pool_odds_are_unchanged() {
    let rings = class("Rings");
    let pool = rings.add_pool("all", 82, &[Kind::Prefix], &[], None);
    match pool.odds(RING_LIFE, 1) {
        Odds::Known {
            hits, total, basis, ..
        } => {
            assert_eq!((hits, total), (1000, 69500));
            assert_eq!(basis, "1000 of 69500 weight");
        }
        other => panic!("expected known odds, got {other:?}"),
    }
    match pool.odds(RING_ACCURACY, 1) {
        Odds::Known { hits, total, .. } => assert_eq!((hits, total), (400, 69500)),
        other => panic!("expected known odds, got {other:?}"),
    }
}

// Start weights -------------------------------------------------------------

#[test]
fn start_weight_is_higher_for_a_rarer_modifier() {
    let accuracy = weight("Rings", "all", Some(82), RING_ACCURACY, Some(1));
    let life = weight("Rings", "all", Some(82), RING_LIFE, Some(1));
    assert_eq!(accuracy.source, WeightSource::Spawn);
    assert_eq!(life.source, WeightSource::Spawn);
    assert!(accuracy.reason.is_none() && life.reason.is_none());
    // A 1-in-N add: total over hits.
    assert!((life.weight - 69500.0 / 1000.0).abs() < 1e-9, "{life:?}");
    assert!(
        (accuracy.weight - 69500.0 / 400.0).abs() < 1e-9,
        "{accuracy:?}"
    );
    // Same side, same pool: tier 1 accuracy (400) is 2.5 times rarer than
    // tier 1 life (1000).
    assert!((accuracy.weight / life.weight - 2.5).abs() < 1e-9);
}

#[test]
fn start_weight_scales_with_the_floor() {
    let any = weight("Rings", "all", Some(82), RING_LIFE, None);
    let t3 = weight("Rings", "all", Some(82), RING_LIFE, Some(3));
    let t1 = weight("Rings", "all", Some(82), RING_LIFE, Some(1));
    // "Any tier" is the worst reachable tier: all eight count.
    assert_eq!(any.floor, 8);
    assert!((any.weight - 69500.0 / 8000.0).abs() < 1e-9, "{any:?}");
    assert!(any.weight < t3.weight && t3.weight < t1.weight);
    // An item level narrows what is reachable: at 50 the best life tier is 2
    // (level 46), so tier 1 is refused rather than weighed as impossible.
    let refused = start_weight(class("Rings"), "all", Some(50), RING_LIFE, Some(1)).unwrap_err();
    assert!(refused.contains("T1"), "{refused}");
    // No item level: every tier reachable, the same as the data's highest
    // tier level.
    let open = weight("Rings", "all", None, RING_LIFE, Some(1));
    assert!((open.weight - t1.weight).abs() < 1e-9);
}

#[test]
fn start_weight_without_spawn_weights_uses_the_default_and_says_why() {
    // Every jewel page is not established: no modifier on the base has a
    // spawn weight, so the default is 1.0, labelled, with the reason.
    let jewels = class("Jewels");
    let variant = jewels.variant_for_base("Sapphire").unwrap();
    let offer = trade_offer("Jewels", "Sapphire", None).unwrap();
    let first = &offer.prefixes[0];
    let jewel = weight("Jewels", variant, None, first.id, None);
    assert_eq!(jewel.source, WeightSource::Default);
    assert_eq!(jewel.weight, 1.0);
    let reason = jewel.reason.as_deref().unwrap();
    assert!(reason.contains("no established spawn weights"), "{reason}");
    assert!(matches!(jewel.odds, Odds::Unknown { .. }));

    // A spawn page whose target tiers carry poe2db's placeholder 1: the
    // default is the median of the same side's weighed modifiers, and the
    // report says so.
    let cast_speed = weight(
        "Rings",
        "all",
        Some(82),
        "suffix-increased-cast-speed",
        None,
    );
    assert_eq!(cast_speed.source, WeightSource::Default);
    let reason = cast_speed.reason.as_deref().unwrap();
    assert!(reason.contains("no real weight"), "{reason}");
    assert!(reason.contains("median"), "{reason}");
    let rings = trade_offer("Rings", "Amethyst Ring", Some(82)).unwrap();
    let mut peers: Vec<f64> = rings
        .suffixes
        .iter()
        .filter(|m| m.weight.source == WeightSource::Spawn)
        .map(|m| m.weight.weight)
        .collect();
    peers.sort_by(f64::total_cmp);
    assert!(!peers.is_empty());
    assert_eq!(cast_speed.weight, peers[(peers.len() - 1) / 2]);

    // Negative control: an ordinary ring modifier is not defaulted.
    let life = weight("Rings", "all", Some(82), RING_LIFE, None);
    assert_eq!(life.source, WeightSource::Spawn);
}

// The offer -------------------------------------------------------------

#[test]
fn trade_offer_lists_the_ordinary_pool_with_its_searchability() {
    let offer = trade_offer("Gloves", "Ancient Mitts", Some(82)).unwrap();
    assert_eq!(offer.variant, "str");
    let gloves = class("Gloves");
    for m in offer.prefixes.iter().chain(&offer.suffixes) {
        let data = gloves.find_mod(m.id).unwrap();
        assert_eq!(data.section, ch_data::Section::Normal, "{}", m.id);
        assert!(!m.tiers.is_empty());
        assert!(m.weight.weight > 0.0);
        assert_eq!(m.search.searchable, !m.search.ids.is_empty(), "{}", m.id);
        assert_eq!(m.search.searchable, m.search.reason.is_none(), "{}", m.id);
    }
    assert!(offer.prefixes.iter().all(|m| m.kind == Kind::Prefix));
    assert!(offer.suffixes.iter().all(|m| m.kind == Kind::Suffix));
    let hybrid = offer
        .prefixes
        .iter()
        .find(|m| m.id == "prefix-increased-armour-2")
        .unwrap();
    assert!(!hybrid.search.searchable);
    assert_eq!(hybrid.search.reason.as_deref(), Some("hybrid"));

    assert!(trade_offer("Gloves", "Amethyst Ring", None).is_err());
    assert!(trade_offer("Not A Class", "Ancient Mitts", None).is_err());
}

// The trade query ----------------------------------------------------------

#[test]
fn trade_query_has_magic_rarity_the_base_and_one_entry_per_modifier() {
    let s = search(
        "Rings",
        "Amethyst Ring",
        Some(82),
        vec![pick(RING_LIFE, None), pick(RING_COLD_RES, Some(1))],
    );
    let q = query(&s);
    let v = &q.query;
    assert_eq!(v["query"]["status"]["option"], "online");
    assert_eq!(v["query"]["type"], "Amethyst Ring");
    let filters = &v["query"]["filters"];
    assert_eq!(
        filters["type_filters"]["filters"]["rarity"]["option"],
        "magic"
    );
    assert_eq!(filters["type_filters"]["filters"]["ilvl"]["min"], 82);
    assert_eq!(
        filters["misc_filters"]["filters"]["corrupted"]["option"],
        "false"
    );
    assert_eq!(v["sort"], json!({"statgroup.1": "desc"}));
    assert_eq!(v["query"]["stats"].as_array().unwrap().len(), 2);
    assert_eq!(group(&q, 0)["type"], "count");
    assert_eq!(group(&q, 1)["type"], "weight");
    assert_eq!(group(&q, 1)["value"], json!({}));

    let stats = trade_stats();
    let life_ids = stats.ids("Rings", RING_LIFE).unwrap();
    let cold_ids = stats.ids("Rings", RING_COLD_RES).unwrap();
    let expected: Vec<&str> = life_ids
        .iter()
        .chain(cold_ids)
        .map(String::as_str)
        .collect();
    assert_eq!(filter_ids(group(&q, 0)), expected);
    assert_eq!(filter_ids(group(&q, 1)), expected);
    assert_eq!(q.entries.len(), 2);
    assert_eq!(q.entries[0].stat_ids, *life_ids);
    assert_eq!(q.entries[0].reference, Some(10.0), "life T8's minimum");
    assert!(q
        .entries
        .iter()
        .all(|e| e.weight.source == WeightSource::Spawn));

    // No item level: no ilvl filter.
    let open = query(&search(
        "Rings",
        "Amethyst Ring",
        None,
        vec![pick(RING_LIFE, None)],
    ));
    assert!(open.query["query"]["filters"]["type_filters"]["filters"]
        .get("ilvl")
        .is_none());

    // A duplicate trade text (`# to all Attributes` on rings) keeps every
    // id, in the same group.
    let id = "suffix-to-all-attributes";
    let ids = stats.ids("Rings", id).unwrap();
    assert!(ids.len() > 1, "{ids:?}");
    let q = query(&search(
        "Rings",
        "Amethyst Ring",
        None,
        vec![pick(id, None)],
    ));
    assert_eq!(filter_ids(group(&q, 0)), filter_ids(group(&q, 1)));
    assert_eq!(
        filter_ids(group(&q, 0)),
        ids.iter().map(String::as_str).collect::<Vec<_>>()
    );
}

#[test]
fn trade_query_weights_are_ordered_by_spawn_rarity() {
    let q = query(&search(
        "Rings",
        "Amethyst Ring",
        Some(82),
        vec![pick(RING_LIFE, Some(1)), pick(RING_ACCURACY, Some(1))],
    ));
    let life = &q.entries[0];
    let accuracy = &q.entries[1];
    // The rarer modifier carries the higher rarity weight.
    assert!(accuracy.weight.weight > life.weight.weight);
    assert!((accuracy.weight.weight / life.weight.weight - 2.5).abs() < 1e-9);
    // The trade weight is the rarity weight over the reference value (the
    // floor tier's minimum roll), so a large-valued modifier is not ranked
    // by its value alone.
    assert_eq!(life.reference, Some(100.0));
    assert_eq!(accuracy.reference, Some(347.0));
    for e in [life, accuracy] {
        let expected = e.weight.weight / e.reference.unwrap();
        assert!((e.trade_weight - expected).abs() / expected < 1e-3, "{e:?}");
    }
    let weights: Vec<f64> = group(&q, 1)["filters"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["value"]["weight"].as_f64().unwrap())
        .collect();
    assert_eq!(weights, vec![life.trade_weight, accuracy.trade_weight]);

    // Two-# lines: the reference is the mean of the two minimums.
    let adds = class("Rings")
        .find_mod("prefix-adds-to-cold-damage-to-attacks")
        .unwrap();
    let t1 = &adds.tiers_on("all")[0];
    let q = query(&search(
        "Rings",
        "Amethyst Ring",
        None,
        vec![pick("prefix-adds-to-cold-damage-to-attacks", Some(t1.tier))],
    ));
    assert_eq!(
        q.entries[0].reference,
        Some((t1.values[0][0] + t1.values[1][0]) / 2.0)
    );
}

#[test]
fn trade_query_counts_two_only_when_both_sides_are_picked() {
    let both = search(
        "Rings",
        "Amethyst Ring",
        Some(82),
        vec![pick(RING_LIFE, None), pick(RING_COLD_RES, None)],
    );
    let q = query(&both);
    assert_eq!(q.count, 2);
    assert_eq!(group(&q, 0)["value"], json!({"min": 2}));
    assert!(q.count_line.contains("Both slots"), "{}", q.count_line);

    let mut loose = both.clone();
    loose.both_slots = false;
    let q = query(&loose);
    assert_eq!(q.count, 1);
    assert_eq!(group(&q, 0)["value"], json!({"min": 1}));
    assert!(q.count_line.contains("At least one"), "{}", q.count_line);

    let one_side = search(
        "Rings",
        "Amethyst Ring",
        Some(82),
        vec![pick(RING_LIFE, None), pick(RING_ACCURACY, None)],
    );
    let q = query(&one_side);
    assert_eq!(q.count, 1);
    assert_eq!(group(&q, 0)["value"], json!({"min": 1}));
    assert!(q.count_line.contains("prefixes"), "{}", q.count_line);
}

#[test]
fn trade_query_names_the_local_stat_on_armour_and_weapons() {
    let q = query(&search(
        "Gloves",
        "Ancient Mitts",
        Some(82),
        vec![
            pick("prefix-increased-armour", None),
            pick(RING_ACCURACY, None),
        ],
    ));
    let stats = trade_stats();
    let armour = &q.entries[0];
    let accuracy = &q.entries[1];
    assert_eq!(armour.stat_ids, vec!["explicit.stat_1062208444"]);
    assert!(stats
        .text("Gloves", "prefix-increased-armour")
        .unwrap()
        .ends_with("(Local)"));
    // Gloves' accuracy is global: the same plain id rings use.
    assert_eq!(accuracy.stat_ids, vec!["explicit.stat_803737631"]);
    assert_eq!(
        accuracy.stat_ids,
        *stats.ids("Rings", RING_ACCURACY).unwrap()
    );
    assert!(!stats
        .text("Gloves", RING_ACCURACY)
        .unwrap()
        .contains("(Local)"));
    assert_eq!(
        filter_ids(group(&q, 0)),
        vec!["explicit.stat_1062208444", "explicit.stat_803737631"]
    );
}

#[test]
fn trade_query_refuses_a_modifier_without_a_trade_stat() {
    let hybrid = trade_query(&search(
        "Gloves",
        "Ancient Mitts",
        None,
        vec![
            pick("prefix-increased-armour", None),
            pick("prefix-increased-armour-2", None),
        ],
    ))
    .unwrap_err();
    assert!(hybrid.contains("prefix-increased-armour-2"), "{hybrid}");
    assert!(hybrid.contains("hybrid"), "{hybrid}");

    let stats = trade_stats();
    let unmatched = stats
        .classes
        .iter()
        .flat_map(|(class, mods)| mods.iter().map(move |(id, s)| (class, id, s)))
        .find(|(class, id, s)| {
            matches!(s, ch_data::TradeStat::NotSearchable { reason, .. } if reason == "no trade stat")
                && class_data_by_name(class)
                    .and_then(|d| d.find_mod(id))
                    .is_some_and(|m| m.section == ch_data::Section::Normal)
        })
        .map(|(class, id, _)| (class.clone(), id.clone()))
        .expect("the mapping records an unmatched modifier");
    let (class_name, id) = unmatched;
    let data = class(&class_name);
    let base = data
        .bases
        .iter()
        .find(|b| !data.find_mod(&id).unwrap().tiers_on(&b.variant).is_empty())
        .unwrap();
    let refused = trade_query(&search(
        &class_name,
        &base.name,
        None,
        vec![pick(&id, None)],
    ))
    .unwrap_err();
    assert!(refused.contains("no trade stat"), "{refused}");

    // The other refusals: nothing picked, too many, an unknown base, a
    // desecrated modifier, a modifier picked twice.
    assert!(trade_query(&search("Rings", "Amethyst Ring", None, vec![])).is_err());
    let rings = trade_offer("Rings", "Amethyst Ring", None).unwrap();
    let many: Vec<TradePick> = rings
        .prefixes
        .iter()
        .chain(&rings.suffixes)
        .filter(|m| m.search.searchable)
        .take(MAX_TRADE_PICKS + 1)
        .map(|m| pick(m.id, None))
        .collect();
    assert_eq!(many.len(), MAX_TRADE_PICKS + 1);
    assert!(trade_query(&search(
        "Rings",
        "Amethyst Ring",
        None,
        many[..MAX_TRADE_PICKS].to_vec()
    ))
    .is_ok());
    let too_many = trade_query(&search("Rings", "Amethyst Ring", None, many)).unwrap_err();
    assert!(
        too_many.contains(&MAX_TRADE_PICKS.to_string()),
        "{too_many}"
    );
    assert!(trade_query(&search(
        "Rings",
        "Ancient Mitts",
        None,
        vec![pick(RING_LIFE, None)]
    ))
    .is_err());
    let desecrated = class("Rings")
        .mods
        .iter()
        .find(|m| m.section == ch_data::Section::Desecrated)
        .unwrap();
    assert!(trade_query(&search(
        "Rings",
        "Amethyst Ring",
        None,
        vec![pick(&desecrated.id, None)]
    ))
    .is_err());
    assert!(trade_query(&search(
        "Rings",
        "Amethyst Ring",
        None,
        vec![pick(RING_LIFE, None), pick(RING_LIFE, Some(1))]
    ))
    .is_err());
}

#[test]
fn trade_query_sets_a_value_floor_only_for_a_chosen_tier() {
    let q = query(&search(
        "Rings",
        "Amethyst Ring",
        Some(82),
        vec![pick(RING_LIFE, None), pick(RING_COLD_RES, Some(2))],
    ));
    let count = group(&q, 0)["filters"].as_array().unwrap();
    assert!(
        count[0].get("value").is_none(),
        "any tier: no value floor: {}",
        count[0]
    );
    let cold = class("Rings").find_mod(RING_COLD_RES).unwrap();
    let t2 = cold.tiers_on("all").iter().find(|t| t.tier == 2).unwrap();
    assert_eq!(count[1]["value"]["min"].as_f64(), Some(t2.values[0][0]));
    assert_eq!(q.entries[1].reference, Some(t2.values[0][0]));
    // The weight group carries a weight for both, never a minimum.
    for f in group(&q, 1)["filters"].as_array().unwrap() {
        assert!(f["value"]["weight"].as_f64().unwrap() > 0.0);
        assert!(f["value"].get("min").is_none());
    }
}
