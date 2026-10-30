//! The one-target tier relaxation ([`ch_engine::Engine::relax`]) over the
//! shipped blueprints and the constructed belts in `tests/items/relax/`
//! (see its README).

mod common;

use ch_engine::Target;
use common::{item, shipped, targets};

const MAGIC: &str = "relax/belt-magic-life.txt";
const FIRE: &str = "suffix-to-fire-resistance";
const COLD: &str = "suffix-to-cold-resistance";
const LIGHTNING: &str = "suffix-to-lightning-resistance";

/// Fire, Cold and Lightning Resistance, fire at `fire`, the others at T2.
fn resistances(fire: u32) -> Vec<Target> {
    targets(&[(FIRE, Some(fire)), (COLD, Some(2)), (LIGHTNING, Some(2))])
}

#[test]
fn belt_t2_resistances_have_no_route_baseline() {
    let plan = shipped().plan(&item(MAGIC), &resistances(2), &[]).unwrap();
    assert!(
        plan.routes.is_empty(),
        "expected no route, got {:?}",
        plan.routes.iter().map(|r| &r.blueprint).collect::<Vec<_>>()
    );
    assert!(!plan.rejected.is_empty());
}

/// Whether some route of `plan` takes a step whose chosen action uses `id`.
fn some_route_uses(plan: &ch_engine::Plan, id: &str) -> bool {
    plan.routes.iter().any(|r| {
        r.steps
            .iter()
            .any(|s| s.action.as_ref().is_some_and(|a| a.uses == id))
    })
}

#[test]
fn belt_t2_resistances_propose_greater_insulation() {
    let engine = shipped();
    let belt = item(MAGIC);
    let relax = engine
        .relax(&belt, &resistances(2), &[])
        .unwrap()
        .expect("a relaxation");
    assert_eq!(relax.target, 0);
    assert_eq!(relax.mod_id, FIRE);
    assert_eq!(relax.lines, vec!["+#% to Fire Resistance".to_string()]);
    assert_eq!((relax.from, relax.to), (2, 3));
    assert_eq!(relax.currency, "greater-essence-of-insulation");
    assert_eq!(relax.name, "Greater Essence of Insulation");
    // The list to send: fire's floor moved, the others as given.
    assert_eq!(relax.targets, resistances(3));
    assert!(!relax.routes.is_empty());

    // Re-planning with it routes, through the essence.
    let plan = engine.plan(&belt, &relax.targets, &[]).unwrap();
    assert!(some_route_uses(&plan, "greater-essence-of-insulation"));
    for blueprint in &relax.routes {
        assert!(plan.routes.iter().any(|r| &r.blueprint == blueprint));
    }
}

#[test]
fn a_blocked_essence_moves_the_relaxation_to_the_next_target() {
    // Negative control on the blocked list: with Greater Essence of
    // Insulation blocked, fire has no tier-3 essence; cold, next in target
    // order, does.
    let engine = shipped();
    let blocked = vec!["greater-essence-of-insulation".to_string()];
    let relax = engine
        .relax(&item(MAGIC), &resistances(2), &blocked)
        .unwrap()
        .expect("a relaxation");
    assert_eq!(relax.mod_id, COLD);
    assert_eq!((relax.from, relax.to), (2, 3));
    assert_eq!(relax.currency, "greater-essence-of-thawing");
}

const NORMAL: &str = "relax/belt-normal.txt";

#[test]
fn a_normal_belt_gets_no_relaxation() {
    // With every transmutation orb blocked a normal belt has no way to
    // become magic, so no floor routes on it and nothing is proposed.
    let engine = shipped();
    let belt = item(NORMAL);
    let blocked: Vec<String> = [
        "orb-of-transmutation",
        "greater-orb-of-transmutation",
        "perfect-orb-of-transmutation",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    for (fire, cold, lightning) in [(2, 2, 2), (3, 2, 2), (2, 3, 2), (2, 2, 3)] {
        let list = targets(&[
            (FIRE, Some(fire)),
            (COLD, Some(cold)),
            (LIGHTNING, Some(lightning)),
        ]);
        let plan = engine.plan(&belt, &list, &blocked).unwrap();
        assert!(
            plan.routes.is_empty(),
            "{fire}/{cold}/{lightning}: expected no route, got {:?}",
            plan.routes.iter().map(|r| &r.blueprint).collect::<Vec<_>>()
        );
    }
    assert_eq!(
        engine.relax(&belt, &resistances(2), &blocked).unwrap(),
        None
    );
}

#[test]
fn a_normal_belt_proposes_greater_thawing() {
    // Fire at T3 still finds no route on a normal belt; cold at T3 does,
    // by transmuting and then the Greater essence, so cold is proposed.
    let engine = shipped();
    let belt = item(NORMAL);
    assert!(engine
        .plan(&belt, &resistances(2), &[])
        .unwrap()
        .routes
        .is_empty());
    assert!(engine
        .plan(&belt, &resistances(3), &[])
        .unwrap()
        .routes
        .is_empty());
    let relax = engine
        .relax(&belt, &resistances(2), &[])
        .unwrap()
        .expect("a relaxation");
    assert_eq!(relax.target, 1);
    assert_eq!(relax.mod_id, COLD);
    assert_eq!((relax.from, relax.to), (2, 3));
    assert_eq!(relax.currency, "greater-essence-of-thawing");
    assert_eq!(relax.name, "Greater Essence of Thawing");
    assert_eq!(
        relax.targets,
        targets(&[(FIRE, Some(2)), (COLD, Some(3)), (LIGHTNING, Some(2))])
    );
    assert!(
        relax.routes.iter().any(|b| b == "magic-augment-essence"),
        "routes: {:?}",
        relax.routes
    );
    let plan = engine.plan(&belt, &relax.targets, &[]).unwrap();
    assert!(some_route_uses(&plan, "greater-essence-of-thawing"));
}

#[test]
fn a_routed_selection_gets_no_relaxation() {
    let engine = shipped();
    let belt = item("relax/belt-rare-life-mana.txt");
    assert!(!engine
        .plan(&belt, &resistances(2), &[])
        .unwrap()
        .routes
        .is_empty());
    assert_eq!(engine.relax(&belt, &resistances(2), &[]).unwrap(), None);
    // The magic belt already routes with fire at T3: nothing to lower.
    assert_eq!(
        engine.relax(&item(MAGIC), &resistances(3), &[]).unwrap(),
        None
    );
}

#[test]
fn a_refused_item_is_refused_by_relax_too() {
    let engine = shipped();
    let refusal = engine.relax(&item(MAGIC), &[], &[]).unwrap_err();
    assert_eq!(refusal, ch_engine::Refusal::NoTargets);
}
