//! The fractured essence swap (docs/models/crafting-slots-spec.md, "The
//! essence swap and an unrevealed modifier"): the shipped
//! `fractured-essence-swap` on a rare whose full side holds a fractured
//! modifier. Essence of Hysteria's add is certain when the removal can take
//! only non-targets; an unrevealed desecrated modifier the removal may take
//! survives (n-1) in n, and the step carries the step the route takes next
//! if it stays and if it goes. The baseline beside it: a swap with no
//! unrevealed modifier carries no survival. Items: `tests/items/essence-swap/`
//! (see its README).

mod common;

use ch_blueprint::Side;
use ch_engine::{Engine, EssenceSwap, Odds, Plan, Route, StepView, Target};
use common::{crate_dir, engine, item, start, targets};

const BLUEPRINT: &str = "fractured-essence-swap";

const MANA: &str = "prefix-to-maximum-mana";
const COLD: &str = "suffix-to-cold-resistance";
/// Essence of Hysteria's modifier on rings, at T2.
const MANA_REGEN: &str = "suffix-increased-mana-regeneration-rate";
/// A ring desecrated suffix (level 65).
const COLD_CHAOS: &str = "desecrated-suffix-to-cold-and-chaos-resistances";
/// Essence of Hysteria's modifier on quivers, at T2.
const BOW: &str = "prefix-increased-damage-with-bow-skills";
/// A quiver desecrated prefix (level 65).
const FAR_PROJECTILES: &str =
    "desecrated-prefix-projectiles-deal-increased-damage-with-hits-against-enemies-further-than-m";
const ATTACK_SPEED: &str = "suffix-increased-attack-speed";

const RING: &str = "essence-swap/ring-swap.txt";
const RING_GOES: &str = "essence-swap/ring-swap-goes.txt";
const RING_ANNULLED_REGEN: &str = "essence-swap/ring-swap-annulled-regen.txt";
const QUIVER: &str = "essence-swap/quiver-swap.txt";
const DUSK_RING: &str = "essence-swap/dusk-ring-swap.txt";

/// The shipped blueprint alone, read from the shipped folder (not
/// `Engine::shipped`, so the other blueprints' routes do not mix in).
fn swap_engine() -> Engine {
    let path = crate_dir()
        .join("..")
        .join("..")
        .join("blueprints")
        .join(format!("{BLUEPRINT}.toml"));
    Engine::load(&[path]).unwrap_or_else(|errors| {
        let lines: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        panic!("{BLUEPRINT} does not load:\n{}", lines.join("\n"))
    })
}

fn planned(engine: &Engine, file: &str, list: &[Target]) -> Plan {
    engine
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

fn ids(route: &Route) -> Vec<&str> {
    route.steps.iter().map(|s| s.id.as_str()).collect()
}

fn known(odds: Option<&Odds>) -> (u64, u64) {
    match odds {
        Some(Odds::Known { hits, total, .. }) => (*hits, *total),
        other => panic!("expected known odds, got {other:?}"),
    }
}

fn omens(step: &StepView) -> Vec<&str> {
    step.action
        .as_ref()
        .map_or_else(Vec::new, |a| a.omens.iter().map(String::as_str).collect())
}

fn swap(step: &StepView) -> &EssenceSwap {
    step.swap
        .as_ref()
        .unwrap_or_else(|| panic!("`{}` carries no essence swap: {step:?}", step.id))
}

/// Mana T5 (a prefix on the ring), Hysteria's Mana Regeneration Rate T2 and
/// a desecrated suffix.
fn ring_targets() -> Vec<Target> {
    targets(&[(MANA, Some(5)), (MANA_REGEN, Some(2)), (COLD_CHAOS, None)])
}

// --- Baseline ------------------------------------------------------------------

#[test]
fn no_survival_without_an_unrevealed_modifier_baseline() {
    // The issue #28 ring (a free suffix slot, nothing fractured or
    // unrevealed) with the listed test swap: the removal may take either
    // side, two of four non-targets, and nothing survives or not.
    let list = targets(&[(MANA, Some(5)), (COLD, Some(5)), (MANA_REGEN, Some(2))]);
    let plan = planned(
        &engine(&["rules/test-swap-plain.toml"]),
        "slots/rare-one-target.txt",
        &list,
    );
    let essence = &route(&plan).steps[0];
    assert_eq!(essence.target.as_deref(), Some(MANA_REGEN));
    assert_eq!(known(essence.odds.as_ref()), (2, 4));
    assert!(
        essence.swap.as_ref().is_none_or(|s| s.survival.is_none()),
        "{:?}",
        essence.swap
    );
}

// --- The ring: the suffix case -------------------------------------------------

#[test]
fn a_ring_swap_gives_the_unrevealed_suffix_one_in_two() {
    let engine = swap_engine();
    let plan = planned(&engine, RING, &ring_targets());
    let route = route(&plan);
    assert_eq!(ids(route), ["essence", "reveal-last"]);
    let essence = &route.steps[0];
    assert_eq!(essence.target.as_deref(), Some(MANA_REGEN));
    assert_eq!(essence.action.as_ref().unwrap().uses, "essence-of-hysteria");
    // The add is certain: the removal takes the unrevealed suffix or
    // Dexterity, neither a target.
    assert_eq!(known(essence.odds.as_ref()), (2, 2));

    let swap = swap(essence);
    assert_eq!(swap.side, Some(Side::Suffix));
    assert_eq!(swap.removes.len(), 2, "{:?}", swap.removes);
    assert!(swap.removes[0].unrevealed && swap.removes[0].desecrated);
    assert_eq!(swap.removes[1].lines, ["+10(9-12) to Dexterity"]);
    assert_eq!(swap.fractured.len(), 1);
    assert!(
        swap.fractured[0].lines[0].contains("to Fire Resistance"),
        "{:?}",
        swap.fractured
    );

    let survival = swap.survival.as_ref().expect("a survival");
    assert_eq!(survival.side, Side::Suffix);
    assert_eq!(known(Some(&survival.odds)), (1, 2));
    let stays = survival.stays.as_deref().expect("a step if it stays");
    assert_eq!(stays.id, "reveal-last");
    assert_eq!(stays.target.as_deref(), Some(COLD_CHAOS));
    let goes = survival.goes.as_deref().expect("a step if it goes");
    assert_eq!(goes.id, "remove");
    assert_eq!(omens(goes), ["omen-of-dextral-annulment"]);
    assert_eq!(known(goes.odds.as_ref()), (1, 2));
    // An outcome's step carries no outcomes of its own.
    assert!(goes.swap.is_none() && stays.swap.is_none());

    // The guide's step on the item carries the same.
    let (_, step) = start(&engine, BLUEPRINT, &item(RING), &ring_targets(), &[]);
    assert_eq!(step.id, "essence");
    assert_eq!(step.swap.as_ref(), Some(swap));
}

#[test]
fn a_ring_swap_goes_annuls_then_desecrates() {
    // The ring after the swap took the unrevealed suffix: fractured Fire
    // Resistance, Dexterity and Mana Regeneration Rate, the suffixes full
    // again. The annulment frees a slot first (it may take the essence's
    // modifier: one in two), then the desecration and its reveal.
    let plan = planned(&swap_engine(), RING_GOES, &ring_targets());
    let route = route(&plan);
    assert_eq!(ids(route), ["remove", "desecrate-last", "reveal-last"]);
    let remove = &route.steps[0];
    assert_eq!(omens(remove), ["omen-of-dextral-annulment"]);
    assert_eq!(known(remove.odds.as_ref()), (1, 2));
    let desecrate = &route.steps[1];
    assert_eq!(desecrate.target.as_deref(), Some(COLD_CHAOS));
    assert_eq!(omens(desecrate), ["omen-of-dextral-necromancy"]);
    assert!(route.steps.iter().all(|s| s.swap.is_none()));
}

#[test]
fn a_ring_swap_annulment_miss_replans() {
    // The annulment took Mana Regeneration Rate instead of Dexterity: state
    // order keeps no miss branch, the route is planned afresh from the item
    // and the essence is bound to the missing modifier again.
    let plan = planned(&swap_engine(), RING_ANNULLED_REGEN, &ring_targets());
    let route = route(&plan);
    let essence = route
        .steps
        .iter()
        .find(|s| s.id == "essence")
        .unwrap_or_else(|| panic!("no essence step: {:?}", ids(route)));
    assert_eq!(essence.target.as_deref(), Some(MANA_REGEN));
    assert_eq!(
        route.bindings.get("essence").map(String::as_str),
        Some(MANA_REGEN)
    );
}

// --- The quiver: the prefix case -------------------------------------------------

#[test]
fn a_quiver_swap_gives_the_unrevealed_prefix_one_in_two() {
    // Fractured Projectile Speed, an unrevealed desecrated prefix and
    // Accuracy Rating; Attack Speed, a target, among the suffixes (so the
    // Sinistral annulment keeps it).
    let list = targets(&[
        (BOW, Some(2)),
        (FAR_PROJECTILES, None),
        (ATTACK_SPEED, Some(2)),
    ]);
    let plan = planned(&swap_engine(), QUIVER, &list);
    let route = route(&plan);
    let essence = &route.steps[0];
    assert_eq!(essence.id, "essence");
    assert_eq!(essence.target.as_deref(), Some(BOW));
    assert_eq!(known(essence.odds.as_ref()), (2, 2));
    let swap = swap(essence);
    assert_eq!(swap.side, Some(Side::Prefix));
    let survival = swap.survival.as_ref().expect("a survival");
    assert_eq!(survival.side, Side::Prefix);
    assert_eq!(known(Some(&survival.odds)), (1, 2));
    assert_eq!(
        survival.stays.as_deref().map(|s| s.id.as_str()),
        Some("reveal-last")
    );
    let goes = survival.goes.as_deref().expect("a step if it goes");
    assert_eq!(goes.id, "remove");
    assert_eq!(omens(goes), ["omen-of-sinistral-annulment"]);
    assert_eq!(known(goes.odds.as_ref()), (1, 2));
}

// --- The Dusk Ring: one throwaway ---------------------------------------------

#[test]
fn a_dusk_ring_swap_removes_its_one_throwaway() {
    // A Dusk Ring holds two suffixes (`-1 Suffix Modifier allowed`): a
    // fractured one and Dexterity. The removal can take only Dexterity.
    let list = targets(&[(MANA, Some(5)), (MANA_REGEN, Some(2))]);
    let plan = planned(&swap_engine(), DUSK_RING, &list);
    let route = route(&plan);
    assert_eq!(ids(route), ["essence"]);
    let essence = &route.steps[0];
    assert_eq!(known(essence.odds.as_ref()), (1, 1));
    let swap = swap(essence);
    assert_eq!(swap.side, Some(Side::Suffix));
    assert_eq!(swap.removes.len(), 1);
    assert_eq!(swap.removes[0].lines, ["+10(9-12) to Dexterity"]);
    assert_eq!(swap.fractured.len(), 1);
    assert!(swap.survival.is_none(), "{:?}", swap.survival);
}
