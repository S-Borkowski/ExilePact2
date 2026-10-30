//! The lord omen (issue #40, the codex's Trick II;
//! docs/models/crafting-slots-spec.md, "`lord-omen`"): a desecration aimed
//! at one lord's desecrated modifier on a weapon, ring or amulet names that
//! lord's omen beside the Necromancy omen, and nothing anywhere else: not
//! for a base-pool target, not on an armour class or a belt, not when the
//! omen is blocked. A whittle's desecration, whose side omen waits for the
//! last target, lets its lord omen wait too. Over the shipped blueprints and
//! the constructed items in `tests/items/lord-omen/` (see its README).

mod common;

use ch_engine::ch_blueprint::{Goal, Side};
use ch_engine::{Engine, Route, StepView, Target};
use ch_item::Item;
use common::{blocked, crate_dir, item, shipped, start, targets};

const MANA: &str = "prefix-to-maximum-mana";
const LIFE: &str = "prefix-to-maximum-life";
const ACC: &str = "prefix-to-accuracy-rating";
const ARMOUR_PCT: &str = "prefix-increased-armour";
const FIRE: &str = "suffix-to-fire-resistance";
/// `Ulaman's` on a ring, its one tier.
const LOW_LIFE_ATTACK: &str = "desecrated-prefix-increased-attack-damage-while-on-low-life";
/// `of Ulaman` on a belt and on a str body armour, its one tier.
const STR_DEX: &str = "desecrated-suffix-to-strength-and-dexterity";

const CLOSE: &str = "close-a-side";
const CLEAN: &str = "rare-clean-then-slam";
const WHITTLE: &str = "rings-resist-whittle";

const SINISTRAL: &str = "omen-of-sinistral-necromancy";
const DEXTRAL: &str = "omen-of-dextral-necromancy";
const SOVEREIGN: &str = "omen-of-the-sovereign";
const LIEGE: &str = "omen-of-the-liege";

/// `tests/items/lord-omen/<name>.txt`.
fn lord_item(name: &str) -> Item {
    item(&format!("lord-omen/{name}.txt"))
}

/// The route of `blueprint` on the item; panics when it is rejected.
fn route(engine: &Engine, blueprint: &str, it: &Item, list: &[Target], block: &[&str]) -> Route {
    let plan = engine.plan(it, list, &blocked(block)).unwrap();
    if let Some(r) = plan.rejected.iter().find(|r| r.blueprint == blueprint) {
        panic!("{blueprint} is rejected: {}", r.reason);
    }
    plan.routes
        .into_iter()
        .find(|r| r.blueprint == blueprint)
        .unwrap()
}

/// The route's one desecration step.
fn desecration(route: &Route) -> StepView {
    let steps: Vec<&StepView> = route
        .steps
        .iter()
        .filter(|s| s.goal == Goal::Desecrate)
        .collect();
    assert_eq!(steps.len(), 1, "{:?}", route.steps);
    steps[0].clone()
}

/// The alternative a step took and its omens, names checked to match.
fn taken(step: &StepView) -> (usize, Vec<String>) {
    let action = step.action.as_ref().expect("a desecration has an action");
    assert_eq!(action.omens.len(), action.omen_names.len(), "{action:?}");
    (action.alternative, action.omens.clone())
}

fn omens(ids: &[&str]) -> Vec<String> {
    ids.iter().map(|s| s.to_string()).collect()
}

/// The ring's targets: Mana T5, Life T4, `last` (the prefix missing) and
/// Fire Resistance T5.
fn ring_targets(last: (&str, Option<u32>)) -> Vec<Target> {
    targets(&[(MANA, Some(5)), (LIFE, Some(4)), last, (FIRE, Some(5))])
}

/// Baseline: the owner's ring with an ordinary prefix missing desecrates
/// with the Necromancy omen alone, as before the rule: a base-pool target
/// never takes a lord omen (it would rule the target out of the offer).
#[test]
fn a_base_pool_target_takes_no_lord_omen_baseline() {
    let engine = shipped();
    let ring = lord_item("ring-2p1s");
    let list = ring_targets((ACC, Some(3)));
    for blueprint in [CLOSE, CLEAN] {
        let step = desecration(&route(&engine, blueprint, &ring, &list, &[]));
        assert_eq!(step.target.as_deref(), Some(ACC), "{blueprint}");
        assert_eq!(taken(&step), (0, omens(&[SINISTRAL])), "{blueprint}");
        let action = step.action.as_ref().unwrap();
        assert!(action.lord_omens_waiting.is_empty(), "{blueprint}");
    }
}

/// The ring's last target is Ulaman's desecrated prefix: the desecration
/// names Omen of the Sovereign after Omen of Sinistral Necromancy, in the
/// preview and in the guide's first step.
#[test]
fn a_ring_desecration_for_an_ulaman_target_takes_the_sovereign_omen() {
    let engine = shipped();
    let ring = lord_item("ring-2p1s");
    let list = ring_targets((LOW_LIFE_ATTACK, Some(1)));
    for blueprint in [CLOSE, CLEAN] {
        let step = desecration(&route(&engine, blueprint, &ring, &list, &[]));
        assert_eq!(step.side, Some(Side::Prefix), "{blueprint}");
        assert_eq!(step.target.as_deref(), Some(LOW_LIFE_ATTACK), "{blueprint}");
        assert_eq!(
            taken(&step),
            (0, omens(&[SINISTRAL, SOVEREIGN])),
            "{blueprint}"
        );
        let action = step.action.as_ref().unwrap();
        assert_eq!(
            action.omen_names,
            ["Omen of Sinistral Necromancy", "Omen of the Sovereign"],
            "{blueprint}"
        );
    }
    let (_, first) = start(&engine, CLEAN, &ring, &list, &[]);
    assert_eq!(first.goal, Goal::Desecrate);
    assert_eq!(taken(&first), (0, omens(&[SINISTRAL, SOVEREIGN])));
    // Negative control: another lord's omen blocked changes nothing.
    let step = desecration(&route(&engine, CLEAN, &ring, &list, &[LIEGE]));
    assert_eq!(taken(&step), (0, omens(&[SINISTRAL, SOVEREIGN])));
}

/// The lord omen blocked: the desecration still takes its first
/// alternative, with the Necromancy omen alone, and says nothing in place
/// of the omen (no fallback line).
#[test]
fn a_blocked_lord_omen_keeps_the_necromancy_omen() {
    let engine = shipped();
    let ring = lord_item("ring-2p1s");
    let list = ring_targets((LOW_LIFE_ATTACK, Some(1)));
    for blueprint in [CLOSE, CLEAN] {
        let step = desecration(&route(&engine, blueprint, &ring, &list, &[SOVEREIGN]));
        assert_eq!(taken(&step), (0, omens(&[SINISTRAL])), "{blueprint}");
        assert_eq!(step.instead_of_desecration, None, "{blueprint}");
    }
    // The guide's step says the same.
    let (_, first) = start(&engine, CLEAN, &ring, &list, &blocked(&[SOVEREIGN]));
    assert_eq!(taken(&first), (0, omens(&[SINISTRAL])));
}

/// A desecration for a one-lord desecrated suffix on a body armour, an
/// armour class, takes the Necromancy omen alone.
#[test]
fn an_armour_desecration_takes_no_lord_omen() {
    let engine = shipped();
    let armour = lord_item("body-armour-2p1s");
    let list = targets(&[
        (LIFE, Some(5)),
        (ARMOUR_PCT, Some(2)),
        (FIRE, Some(3)),
        (STR_DEX, Some(1)),
    ]);
    for blueprint in [CLOSE, CLEAN] {
        let step = desecration(&route(&engine, blueprint, &armour, &list, &[]));
        assert_eq!(step.target.as_deref(), Some(STR_DEX), "{blueprint}");
        assert_eq!(step.side, Some(Side::Suffix), "{blueprint}");
        assert_eq!(taken(&step), (0, omens(&[DEXTRAL])), "{blueprint}");
    }
}

/// A belt is jewellery by its bones, but not a lord class: the same `of
/// Ulaman` suffix takes no lord omen there.
#[test]
fn a_belt_desecration_takes_no_lord_omen() {
    let engine = shipped();
    let belt = lord_item("belt-2p1s");
    let list = targets(&[
        (LIFE, Some(2)),
        (MANA, Some(2)),
        (FIRE, Some(3)),
        (STR_DEX, Some(1)),
    ]);
    for blueprint in [CLOSE, CLEAN] {
        let step = desecration(&route(&engine, blueprint, &belt, &list, &[]));
        assert_eq!(step.target.as_deref(), Some(STR_DEX), "{blueprint}");
        assert_eq!(taken(&step), (0, omens(&[DEXTRAL])), "{blueprint}");
    }
}

/// An engine over the shipped whittle alone.
fn whittle_engine() -> Engine {
    let path = crate_dir()
        .join("..")
        .join("..")
        .join("blueprints")
        .join(format!("{WHITTLE}.toml"));
    Engine::load(&[path]).unwrap_or_else(|errors| {
        let lines: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        panic!("the whittle does not load:\n{}", lines.join("\n"))
    })
}

/// The whittle's `desecrate` step on `it`.
fn whittle_desecration(it: &Item, list: &[Target], block: &[&str]) -> StepView {
    let r = route(&whittle_engine(), WHITTLE, it, list, block);
    r.steps
        .iter()
        .find(|s| s.id == "desecrate")
        .unwrap_or_else(|| panic!("no `desecrate` step: {:?}", r.steps))
        .clone()
}

/// A whittle aiming at a suffix and Ulaman's prefix: while both are
/// missing its Necromancy omen waits for the last target's side, and its
/// lord omen waits too, named for the target that would take it. Once the
/// Ulaman prefix is the one left, the step names both omens.
#[test]
fn a_waiting_whittle_desecration_waits_for_its_lord_omen() {
    let list = targets(&[(FIRE, Some(3)), (LOW_LIFE_ATTACK, Some(1))]);
    let start_ring = lord_item("whittle-start");
    let step = whittle_desecration(&start_ring, &list, &[]);
    assert_eq!(step.target, None);
    let action = step.action.as_ref().unwrap();
    assert_eq!((action.alternative, action.omens.clone()), (0, Vec::new()));
    assert!(action.side_omen_waits);
    let waiting: Vec<(&str, &str, &str)> = action
        .lord_omens_waiting
        .iter()
        .map(|w| (w.target.as_str(), w.omen.as_str(), w.name.as_str()))
        .collect();
    assert_eq!(
        waiting,
        [(LOW_LIFE_ATTACK, SOVEREIGN, "Omen of the Sovereign")]
    );
    // The Sovereign blocked: nothing waits, the side omen still does.
    let step = whittle_desecration(&start_ring, &list, &[SOVEREIGN]);
    let action = step.action.as_ref().unwrap();
    assert!(action.side_omen_waits);
    assert!(action.lord_omens_waiting.is_empty());
    // Negative control: two ordinary targets have no lord omen to wait for.
    let plain = targets(&[(FIRE, Some(3)), (ACC, None)]);
    let step = whittle_desecration(&start_ring, &plain, &[]);
    let action = step.action.as_ref().unwrap();
    assert!(action.side_omen_waits);
    assert!(action.lord_omens_waiting.is_empty());

    // Fire Resistance landed: the Ulaman prefix is the last target, and the
    // desecration names both omens, nothing waiting.
    let fire_ring = lord_item("whittle-fire");
    let step = whittle_desecration(&fire_ring, &list, &[]);
    assert_eq!(step.target.as_deref(), Some(LOW_LIFE_ATTACK));
    assert_eq!(taken(&step), (0, omens(&[SINISTRAL, SOVEREIGN])));
    let action = step.action.as_ref().unwrap();
    assert!(!action.side_omen_waits);
    assert!(action.lord_omens_waiting.is_empty());
}
