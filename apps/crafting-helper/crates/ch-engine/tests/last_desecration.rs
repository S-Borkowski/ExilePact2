//! The last-target desecration (issue #32; docs/models/crafting-slots-spec.md,
//! "The last-target desecration"): on a state-ordered route, a rare with no
//! desecrated modifier and exactly one target missing gets that target from
//! a desecration on its side and a reveal, not an exalt; where the
//! desecration cannot be taken the step taken instead says why. Over the
//! shipped v2 blueprints and the owner's 2P/1S belt and its later states in
//! `tests/items/last-desecration/` (constructed, see its README), and the
//! parity test's body armour, `tests/items/slots/`.

mod common;

use ch_engine::ch_blueprint::{Goal, Side};
use ch_engine::{Engine, Next, OutcomeKind, Route, StepView, Target};
use ch_item::Item;
use common::{blocked, item, next_step, observe, shipped, start, targets};

const LIFE: &str = "prefix-to-maximum-life";
const MANA: &str = "prefix-to-maximum-mana";
const ARMOUR: &str = "prefix-to-armour";
const FIRE: &str = "suffix-to-fire-resistance";
const COLD: &str = "suffix-to-cold-resistance";
const STR_DEX: &str = "desecrated-suffix-to-strength-and-dexterity";

const CLOSE: &str = "close-a-side";
const CLEAN: &str = "rare-clean-then-slam";

const SINISTRAL: &str = "omen-of-sinistral-necromancy";
const ECHOES: &str = "omen-of-abyssal-echoes";

/// `tests/items/last-desecration/<name>.txt`.
fn belt(name: &str) -> Item {
    item(&format!("last-desecration/{name}.txt"))
}

/// The owner's belt: Life T2, Mana T2 and a missing Armour T2 (prefixes),
/// Fire Resistance T3 (suffix).
fn belt_targets() -> Vec<Target> {
    targets(&[
        (LIFE, Some(2)),
        (MANA, Some(2)),
        (ARMOUR, Some(2)),
        (FIRE, Some(3)),
    ])
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

/// A step as the preview shows it: id, side, currency as written, omens.
fn shape(step: &StepView) -> (String, Option<Side>, String, Vec<String>) {
    let action = step.action.as_ref().expect("a step with an action");
    (
        step.id.clone(),
        step.side,
        action.currency.clone(),
        action.omens.clone(),
    )
}

fn step(
    id: &str,
    side: Option<Side>,
    currency: &str,
    omens: &[&str],
) -> (String, Option<Side>, String, Vec<String>) {
    (
        id.to_string(),
        side,
        currency.to_string(),
        omens.iter().map(|o| o.to_string()).collect(),
    )
}

fn shapes(route: &Route) -> Vec<(String, Option<Side>, String, Vec<String>)> {
    route.steps.iter().map(shape).collect()
}

// --- Issue #32, test 2: an item that already has its desecration ----------

/// The baseline: a belt whose suffixes are a target and a revealed
/// desecrated target, its prefix Armour missing. The item already carries
/// its one desecrated modifier, so the route closes the finished suffix
/// side and slams the prefix, as before the rule.
#[test]
fn an_item_with_a_desecrated_target_keeps_its_route() {
    let engine = shipped();
    let it = belt("belt-desecrated-target");
    let mut list = belt_targets();
    list.push(Target::new(STR_DEX, Some(1)));

    let close = route(&engine, CLOSE, &it, &list, &[]);
    assert_eq!(
        shapes(&close),
        [
            step(
                "close",
                Some(Side::Suffix),
                "exalted-orb",
                &["omen-of-dextral-exaltation"]
            ),
            step(
                "slam-open-side",
                Some(Side::Prefix),
                "perfect-exalted-orb",
                &[]
            ),
        ]
    );
    assert!(close.steps.iter().all(|s| s.goal != Goal::Desecrate));

    let clean = route(&engine, CLEAN, &it, &list, &[]);
    assert_eq!(
        shapes(&clean),
        [step(
            "side-slam",
            Some(Side::Prefix),
            "perfect-exalted-orb",
            &["omen-of-sinistral-exaltation"]
        )]
    );

    let (_, first) = start(&engine, CLOSE, &it, &list, &[]);
    assert_eq!(first.id, "close");
    assert_eq!(first.instead_of_desecration, None);
}

// --- Issue #32, test 1: the last target is desecrated ---------------------

/// The owner's belt, Armour the one target missing: both rare routes
/// desecrate on the prefix side with Omen of Sinistral Necromancy and
/// reveal with Omen of Abyssal Echoes (the desecration's odds unknown, the
/// reveal's an estimate: tests/reveal_odds.rs), instead of closing the
/// suffix side and slamming; the guide walks the same two steps to done.
#[test]
fn the_last_target_on_an_undesecrated_rare_is_desecrated() {
    let engine = shipped();
    let it = belt("belt-2p1s");
    let list = belt_targets();

    for blueprint in [CLOSE, CLEAN] {
        let r = route(&engine, blueprint, &it, &list, &[]);
        assert_eq!(
            shapes(&r),
            [
                step(
                    "desecrate-last",
                    Some(Side::Prefix),
                    "desecrate",
                    &[SINISTRAL]
                ),
                step("reveal-last", None, "reveal", &[ECHOES]),
            ],
            "{blueprint}"
        );
        let desecrate = &r.steps[0];
        assert_eq!(desecrate.goal, Goal::Desecrate);
        assert_eq!(desecrate.target.as_deref(), Some(ARMOUR));
        assert!(
            desecrate
                .odds
                .as_ref()
                .is_some_and(|o| o.percent().is_none()),
            "{desecrate:?}"
        );
        let reveal = &r.steps[1];
        assert!(
            reveal.odds.as_ref().is_some_and(|o| o.percent().is_some()),
            "{reveal:?}"
        );
        for s in &r.steps {
            assert_eq!(s.instead_of_desecration, None, "{s:?}");
        }
    }

    let (mut guide, first) = start(&engine, CLOSE, &it, &list, &[]);
    assert_eq!(
        shape(&first),
        step(
            "desecrate-last",
            Some(Side::Prefix),
            "desecrate",
            &[SINISTRAL]
        )
    );
    let unrevealed = belt("belt-unrevealed");
    let out = observe(&engine, &mut guide, &it, &unrevealed);
    assert_eq!(out.result, OutcomeKind::Hit, "{out:?}");
    assert_eq!(
        shape(next_step(&out.next)),
        step("reveal-last", None, "reveal", &[ECHOES])
    );
    let out = observe(
        &engine,
        &mut guide,
        &unrevealed,
        &belt("belt-revealed-armour"),
    );
    assert_eq!(out.result, OutcomeKind::Hit, "{out:?}");
    assert_eq!(out.next, Next::Done);
}

// --- Where the desecration cannot be taken ---------------------------------

/// No desecrated modifier on the target's side for the class at the item's
/// level: a body armour has no desecrated prefix, and a belt below item
/// level 65 none at all. The route closes and slams as before, and each
/// step taken instead of the desecration says why.
#[test]
fn a_last_target_outside_the_desecration_pool_falls_back_with_why() {
    let engine = shipped();

    let armour = item("slots/body-armour-prefix-no-pool.txt");
    let list = targets(&[
        (LIFE, Some(5)),
        ("prefix-increased-armour", Some(2)),
        ("prefix-to-spirit", Some(3)),
        (FIRE, Some(3)),
    ]);
    let why = "no desecrated prefix modifier on Body Armours at item level 82";
    let r = route(&engine, CLOSE, &armour, &list, &[]);
    let ids: Vec<&str> = r.steps.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(ids, ["close", "slam-open-side"]);
    for s in &r.steps {
        assert_eq!(s.instead_of_desecration.as_deref(), Some(why), "{s:?}");
    }
    let r = route(&engine, CLEAN, &armour, &list, &[]);
    assert_eq!(
        shapes(&r),
        [step(
            "side-slam",
            Some(Side::Prefix),
            "perfect-exalted-orb",
            &["omen-of-sinistral-exaltation"]
        )]
    );
    assert_eq!(r.steps[0].instead_of_desecration.as_deref(), Some(why));

    // The guide's step says the same.
    let (_, first) = start(&engine, CLOSE, &armour, &list, &[]);
    assert_eq!(first.id, "close");
    assert_eq!(first.instead_of_desecration.as_deref(), Some(why));

    // The owner's belt at item level 64, Armour at a floor that level
    // reaches.
    let low = belt("belt-2p1s-level-64");
    let list = targets(&[
        (LIFE, Some(2)),
        (MANA, Some(2)),
        (ARMOUR, Some(4)),
        (FIRE, Some(3)),
    ]);
    let r = route(&engine, CLOSE, &low, &list, &[]);
    assert_eq!(r.steps[0].id, "close");
    assert_eq!(
        r.steps[0].instead_of_desecration.as_deref(),
        Some("no desecrated prefix modifier on Belts at item level 64")
    );
}

/// The Necromancy omen for the target's side, or every bone that fits the
/// belt, on the blocked list: a bone without the side omen could land on
/// the other side, so the route keeps the close and the slam, and the
/// close names what is blocked. Blocking the other side's omen changes
/// nothing.
#[test]
fn a_blocked_necromancy_omen_falls_back_with_why() {
    let engine = shipped();
    let it = belt("belt-2p1s");
    let list = belt_targets();
    let why = "Omen of Sinistral Necromancy blocked";

    let r = route(&engine, CLOSE, &it, &list, &[SINISTRAL]);
    assert_eq!(
        shapes(&r),
        [
            step(
                "close",
                Some(Side::Suffix),
                "exalted-orb",
                &["omen-of-greater-exaltation", "omen-of-dextral-exaltation"]
            ),
            step(
                "slam-open-side",
                Some(Side::Prefix),
                "perfect-exalted-orb",
                &[]
            ),
        ]
    );
    assert_eq!(r.steps[0].instead_of_desecration.as_deref(), Some(why));
    let r = route(&engine, CLEAN, &it, &list, &[SINISTRAL]);
    assert_eq!(r.steps[0].id, "side-slam");
    assert_eq!(r.steps[0].instead_of_desecration.as_deref(), Some(why));

    let bones = ["preserved-collarbone", "ancient-collarbone"];
    let r = route(&engine, CLOSE, &it, &list, &bones);
    assert_eq!(r.steps[0].id, "close");
    assert_eq!(
        r.steps[0].instead_of_desecration.as_deref(),
        Some("Preserved Collarbone and Ancient Collarbone blocked")
    );

    let r = route(&engine, CLOSE, &it, &list, &["omen-of-dextral-necromancy"]);
    assert_eq!(r.steps[0].id, "desecrate-last");
    assert_eq!(r.steps[0].instead_of_desecration, None);
}

/// Two targets missing: the binding is not made, so the first step is the
/// slam; once the slam has placed one, the other is the last and is
/// desecrated on its side.
#[test]
fn two_missing_targets_are_not_desecrated() {
    let engine = shipped();
    let it = belt("belt-2p1s");
    let mut list = belt_targets();
    list.push(Target::new(COLD, Some(3)));

    let r = route(&engine, CLEAN, &it, &list, &[]);
    assert_eq!(
        shapes(&r),
        [
            step(
                "side-slam",
                Some(Side::Prefix),
                "perfect-exalted-orb",
                &["omen-of-sinistral-exaltation"]
            ),
            step(
                "desecrate-last",
                Some(Side::Suffix),
                "desecrate",
                &["omen-of-dextral-necromancy"]
            ),
            step("reveal-last", None, "reveal", &[ECHOES]),
        ]
    );
    assert_eq!(r.steps[0].instead_of_desecration, None);
    assert_eq!(r.steps[1].target.as_deref(), Some(COLD));

    let (_, first) = start(&engine, CLEAN, &it, &list, &[]);
    assert_eq!(first.id, "side-slam");
    assert_eq!(first.instead_of_desecration, None);
}

/// A reveal that offered no target leaves a revealed desecrated modifier
/// that is not one: Omen of Light with an Orb of Annulment takes it,
/// certain, and the bone is used again; a fresh plan of that item agrees.
#[test]
fn a_wrong_reveal_is_lit_off_and_desecrated_again() {
    let engine = shipped();
    let it = belt("belt-2p1s");
    let list = belt_targets();
    let unrevealed = belt("belt-unrevealed");
    let wrong = belt("belt-wrong-reveal");
    let light = step("light-last", None, "orb-of-annulment", &["omen-of-light"]);
    let desecrate = step(
        "desecrate-last",
        Some(Side::Prefix),
        "desecrate",
        &[SINISTRAL],
    );

    let (mut guide, _) = start(&engine, CLOSE, &it, &list, &[]);
    let out = observe(&engine, &mut guide, &it, &unrevealed);
    assert_eq!(out.result, OutcomeKind::Hit, "{out:?}");
    let out = observe(&engine, &mut guide, &unrevealed, &wrong);
    assert_eq!(out.result, OutcomeKind::Miss, "{out:?}");
    let next = next_step(&out.next);
    assert_eq!(shape(next), light);
    assert_eq!(next.odds.as_ref().and_then(|o| o.percent()), Some(100.0));
    let out = observe(&engine, &mut guide, &wrong, &it);
    assert_eq!(out.result, OutcomeKind::Hit, "{out:?}");
    assert_eq!(shape(next_step(&out.next)), desecrate);

    for blueprint in [CLOSE, CLEAN] {
        let r = route(&engine, blueprint, &wrong, &list, &[]);
        assert_eq!(
            shapes(&r),
            [
                light.clone(),
                desecrate.clone(),
                step("reveal-last", None, "reveal", &[ECHOES]),
            ],
            "{blueprint}"
        );
    }
}
