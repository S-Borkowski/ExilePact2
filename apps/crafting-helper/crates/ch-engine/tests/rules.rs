//! The route rules from the crafting guides (issue #35;
//! docs/models/crafting-slots-spec.md, "Certain removals first" and
//! "Rejections"): a jewel never takes a Greater or Perfect currency, a
//! certain removal goes before a gamble, a magic start whose target prefix
//! is weak is sent back for a new base, and a Perfect or special essence is
//! a chaos-style swap. Each rule has its baseline beside it. Items and test
//! blueprints: `tests/items/rules/` (see its README).

mod common;

use ch_engine::{Odds, Plan, Rejection, Route, StepView, Target};
use common::{engine, item, start, targets};

const MOVEMENT: &str = "prefix-increased-movement-speed";
const BOOTS_LIFE: &str = "prefix-to-maximum-life";
const BOOTS_FIRE: &str = "suffix-to-fire-resistance";

const MANA: &str = "prefix-to-maximum-mana";
const LIFE: &str = "prefix-to-maximum-life";
const EVASION: &str = "prefix-to-evasion-rating";
const FIRE: &str = "suffix-to-fire-resistance";
const COLD: &str = "suffix-to-cold-resistance";
/// Essence of Hysteria's modifier on rings, at T2.
const MANA_REGEN: &str = "suffix-increased-mana-regeneration-rate";

const JEWEL_GRADES: &str = "rules/test-jewel-grades.toml";
const WEAK_MAGIC: &str = "rules/test-weak-magic.toml";
const WEAK_MAGIC_STATE: &str = "rules/test-weak-magic-state.toml";
const REMOVAL_ORDER: &str = "rules/test-removal-order.toml";
const SWAP_OMEN: &str = "rules/test-swap-omen.toml";
const SWAP_PLAIN: &str = "rules/test-swap-plain.toml";
const SWAP_ORDER: &str = "rules/test-swap-order.toml";

/// The 2P/2S ring of issue #28: Mana T5 and Accuracy Rating T4; Cold
/// Resistance T5 and Dexterity T7.
const RING_2P2S: &str = "slots/rare-one-target.txt";
/// Mana T5, Accuracy Rating T3 and Cold Damage T3: the prefixes full; Fire
/// Resistance T5.
const RING_PREFIX_FULL: &str = "engine/ring-state-prefix-blocked.txt";

fn planned(blueprint: &str, file: &str, list: &[Target]) -> Plan {
    engine(&[blueprint])
        .plan(&item(file), list, &[])
        .unwrap_or_else(|e| panic!("{file} is refused: {e}"))
}

/// The one route of the plan; panics with the rejection when there is none.
fn route(plan: &Plan) -> &Route {
    match (plan.routes.first(), plan.rejected.first()) {
        (Some(r), _) => r,
        (None, Some(r)) => panic!("rejected: {}", r.reason),
        (None, None) => panic!("neither routed nor rejected"),
    }
}

fn rejection(plan: &Plan) -> &Rejection {
    match plan.rejected.first() {
        Some(r) => &r.reason,
        None => panic!("not rejected: {:?}", plan.routes.first().map(|r| &r.steps)),
    }
}

fn currency(step: &StepView) -> &str {
    step.action.as_ref().map_or("", |a| a.currency.as_str())
}

fn percent(step: &StepView) -> Option<f64> {
    step.odds.as_ref().and_then(Odds::percent)
}

fn ids(route: &Route) -> Vec<&str> {
    route.steps.iter().map(|s| s.id.as_str()).collect()
}

// --- Jewels ------------------------------------------------------------------

#[test]
fn a_jewel_route_takes_the_plain_exalted_orb() {
    let plan = planned(
        JEWEL_GRADES,
        "engine/jewels-rare-sapphire.txt",
        &targets(&[("prefix-increased-spell-damage", None)]),
    );
    let slam = &route(&plan).steps[0];
    assert_eq!(currency(slam), "exalted-orb", "{slam:?}");
    // The engine says why it passed the Perfect and Greater orbs over.
    let guide = engine(&[JEWEL_GRADES]);
    let (_, step) = start(
        &guide,
        "test-jewel-grades",
        &item("engine/jewels-rare-sapphire.txt"),
        &targets(&[("prefix-increased-spell-damage", None)]),
        &[],
    );
    assert_eq!(currency(&step), "exalted-orb");
}

#[test]
fn a_ring_route_keeps_the_perfect_exalted_orb_baseline() {
    let plan = planned(
        JEWEL_GRADES,
        "engine/ring-rare-mana-fire.txt",
        &targets(&[(MANA, Some(5)), (LIFE, Some(5)), (FIRE, Some(5))]),
    );
    assert_eq!(currency(&route(&plan).steps[0]), "perfect-exalted-orb");
}

// --- weak-magic-start ----------------------------------------------------------

/// Movement speed (a target at any tier counts) and Fire Resistance.
fn boots_targets(movement_floor: u32) -> Vec<Target> {
    targets(&[(MOVEMENT, Some(movement_floor)), (BOOTS_FIRE, Some(3))])
}

#[test]
fn a_weak_magic_prefix_rejects_a_route_that_spends_an_upgrade() {
    // Boots movement speed reaches T1 at item level 82 and T2 at 65: T3 on
    // a magic item level 82 pair is below the top two.
    for blueprint in [WEAK_MAGIC, WEAK_MAGIC_STATE] {
        let plan = planned(
            blueprint,
            "rules/boots-magic-movement-t3.txt",
            &boots_targets(3),
        );
        match rejection(&plan) {
            Rejection::WeakMagicStart {
                modifier,
                tier,
                item_level,
                top,
            } => {
                assert_eq!(modifier, MOVEMENT);
                assert_eq!((*tier, *item_level), (3, 82));
                assert_eq!(top, &[1, 2]);
            }
            other => panic!("{blueprint}: expected weak-magic-start, got {other:?}"),
        }
        let words = rejection(&plan).to_string();
        assert!(words.contains("T3") && words.contains("82"), "{words}");
        let kind = serde_json::to_value(rejection(&plan)).unwrap()["kind"].clone();
        assert_eq!(kind, "weak-magic-start");
    }
}

#[test]
fn a_top_two_magic_prefix_keeps_its_route_baseline() {
    for blueprint in [WEAK_MAGIC, WEAK_MAGIC_STATE] {
        let plan = planned(
            blueprint,
            "rules/boots-magic-movement-t2.txt",
            &boots_targets(3),
        );
        let route = route(&plan);
        assert!(!route.steps.is_empty(), "{blueprint}");
    }
}

#[test]
fn a_magic_prefix_that_is_not_a_target_keeps_its_route() {
    // Life T3 is below its top two at item level 82 too, but it is not a
    // target: the start is not judged by it.
    let plan = planned(
        WEAK_MAGIC,
        "rules/boots-magic-life-t3.txt",
        &boots_targets(2),
    );
    assert_eq!(currency(&route(&plan).steps[0]), "regal-orb");
    // Control: the same item with its Life a target is rejected.
    let plan = planned(
        WEAK_MAGIC,
        "rules/boots-magic-life-t3.txt",
        &targets(&[(BOOTS_LIFE, Some(3)), (BOOTS_FIRE, Some(3))]),
    );
    assert!(
        matches!(rejection(&plan), Rejection::WeakMagicStart { .. }),
        "{:?}",
        rejection(&plan)
    );
}

// --- Certain removals first ------------------------------------------------------

#[test]
fn a_certain_removal_goes_before_an_uncertain_step() {
    // The prefixes (Mana, Accuracy, Cold Damage) are all non-targets and
    // full while Life is missing: the side is blocked. A Sinistral
    // annulment can take only them, so it is certain, and goes before the
    // slam listed first.
    let list = targets(&[(LIFE, Some(4)), (FIRE, Some(5)), (COLD, Some(5))]);
    let plan = planned(REMOVAL_ORDER, RING_PREFIX_FULL, &list);
    let route = route(&plan);
    assert_eq!(ids(route)[0], "remove", "{:?}", ids(route));
    let first = &route.steps[0];
    assert_eq!(
        first.action.as_ref().unwrap().omens,
        ["omen-of-sinistral-annulment"]
    );
    assert_eq!(percent(first), Some(100.0));
    // The guide picks the same step on the item.
    let (_, step) = start(
        &engine(&[REMOVAL_ORDER]),
        "test-removal-order",
        &item(RING_PREFIX_FULL),
        &list,
        &[],
    );
    assert_eq!(step.id, "remove");
}

#[test]
fn an_uncertain_removal_keeps_file_order_baseline() {
    // The same item with its Mana a target: the annulment may take it, so
    // the slam keeps its file-order place.
    let list = targets(&[
        (MANA, Some(5)),
        (LIFE, Some(4)),
        (FIRE, Some(5)),
        (COLD, Some(5)),
    ]);
    let plan = planned(REMOVAL_ORDER, RING_PREFIX_FULL, &list);
    let route = route(&plan);
    assert_eq!(ids(route)[0], "side-slam", "{:?}", ids(route));
    let remove = route.steps.iter().find(|s| s.id == "remove").unwrap();
    assert!(percent(remove).is_some_and(|p| p < 100.0), "{remove:?}");
}

// --- Essence swaps -----------------------------------------------------------------

#[test]
fn an_essence_swap_shows_its_removal_odds() {
    // Essence of Hysteria adds Mana Regeneration Rate at T2 for certain;
    // Omen of Dextral Crystallisation confines the removal to the suffixes,
    // Cold Resistance (a target) and Dexterity: half.
    let list = targets(&[(MANA, Some(5)), (COLD, Some(5)), (MANA_REGEN, Some(2))]);
    let plan = planned(SWAP_OMEN, RING_2P2S, &list);
    let essence = &route(&plan).steps[0];
    assert_eq!(currency(essence), "essence-of-hysteria");
    assert_eq!(essence.target.as_deref(), Some(MANA_REGEN));
    assert_eq!(percent(essence), Some(50.0), "{:?}", essence.odds);
}

#[test]
fn an_essence_swap_on_a_full_side_removes_from_that_side() {
    // No Crystallisation omen. The suffixes (Cold Resistance, a target,
    // Dexterity and Fire Resistance) are full, so the removal takes one of
    // them: two in three. The two prefixes, both non-targets, are not in
    // the share (over the five it would be four in five).
    let list = targets(&[(COLD, Some(5)), (MANA_REGEN, Some(2))]);
    let plan = planned(SWAP_PLAIN, "rules/ring-full-suffix.txt", &list);
    let essence = &route(&plan).steps[0];
    match &essence.odds {
        Some(Odds::Known { hits, total, .. }) => assert_eq!((*hits, *total), (2, 3)),
        other => panic!("expected known odds, got {other:?}"),
    }
    // Control: the issue #28 ring has a free suffix slot, so the removal
    // may take either side (Mana, a target, Accuracy; Cold, a target,
    // Dexterity): two in four.
    let list = targets(&[(MANA, Some(5)), (COLD, Some(5)), (MANA_REGEN, Some(2))]);
    let plan = planned(SWAP_PLAIN, RING_2P2S, &list);
    let essence = &route(&plan).steps[0];
    match &essence.odds {
        Some(Odds::Known { hits, total, .. }) => assert_eq!((*hits, *total), (2, 4)),
        other => panic!("expected known odds, got {other:?}"),
    }
}

/// Mana T5, Life T4 and Evasion Rating T3 (prefixes: Life and Evasion
/// missing, so the prefixes are blocked) and Hysteria's Mana Regeneration
/// Rate T2.
fn swap_targets(cold_wanted: bool) -> Vec<Target> {
    let mut list = vec![(MANA, Some(5)), (LIFE, Some(4)), (EVASION, Some(3))];
    if cold_wanted {
        list.push((COLD, Some(5)));
    }
    list.push((MANA_REGEN, Some(2)));
    targets(&list)
}

#[test]
fn a_certain_essence_swap_goes_before_an_uncertain_step() {
    // The annulment of the blocked prefixes may take Mana; the essence's
    // Dextral removal can take only Cold Resistance or Dexterity, neither a
    // target: certain, so it goes first.
    let plan = planned(SWAP_ORDER, RING_2P2S, &swap_targets(false));
    let route = route(&plan);
    assert_eq!(ids(route)[0], "essence", "{:?}", ids(route));
    let essence = &route.steps[0];
    assert_eq!(essence.target.as_deref(), Some(MANA_REGEN));
    assert_eq!(percent(essence), Some(100.0), "{:?}", essence.odds);
    assert!(ids(route).contains(&"remove"));
    let (_, step) = start(
        &engine(&[SWAP_ORDER]),
        "test-swap-order",
        &item(RING_2P2S),
        &swap_targets(false),
        &[],
    );
    assert_eq!(step.id, "essence");
}

#[test]
fn an_essence_swap_with_a_target_on_its_side_keeps_file_order_baseline() {
    // Cold Resistance a target: the essence's removal may take it, so the
    // annulment stays first.
    let plan = planned(SWAP_ORDER, RING_2P2S, &swap_targets(true));
    let route = route(&plan);
    assert_eq!(ids(route)[0], "remove", "{:?}", ids(route));
}
