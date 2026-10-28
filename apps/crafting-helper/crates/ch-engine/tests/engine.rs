//! The engine's route choice and guide, over the test blueprints and the
//! constructed items in `tests/items/engine/` (see its README), and the
//! real fixtures of `ch-item` where one fits.

mod common;

use ch_engine::{Next, Odds, OutcomeKind, Refusal, Rejection, Status, Target};
use common::{blocked, engine, item, next_step, observe, parse, real, start, targets};

const RING: &str = "engine/ring-rare-mana-fire.txt";
const LIFE: &str = "prefix-to-maximum-life";
const FIRE: &str = "suffix-to-fire-resistance";
const CHAOS: &str = "suffix-to-chaos-resistance";
const ES: &str = "prefix-increased-energy-shield";
const COLD: &str = "suffix-to-cold-resistance";
const SKILL_SPEED: &str = "desecrated-suffix-increased-skill-speed";

/// Life at T5 or better is missing; fire resistance at T5 is on the ring.
fn ring_targets() -> Vec<Target> {
    targets(&[(LIFE, Some(5)), (FIRE, Some(5))])
}

fn unknown(odds: &Option<Odds>) -> String {
    match odds {
        Some(Odds::Unknown { reason }) => reason.clone(),
        other => panic!("expected unknown odds, got {other:?}"),
    }
}

// --- Route choice ---------------------------------------------------------

#[test]
fn essence_first_when_an_essence_meets_the_floor() {
    let engine = engine(&["engine/test-essence.toml"]);
    let gloves = item("engine/gloves-magic-cold.txt");

    // The plain Essence of Enhancement adds increased Energy Shield at T4
    // on int gloves: floor T4 binds it, before any exalt.
    let plan = engine
        .plan(&gloves, &targets(&[(ES, Some(4)), (COLD, Some(5))]), &[])
        .unwrap();
    let route = &plan.routes[0];
    assert_eq!(
        route.start, 0,
        "the Magic start, which needs an essence binding"
    );
    let first = &route.steps[0];
    assert_eq!(first.id, "essence");
    assert_eq!(first.target.as_deref(), Some(ES));
    assert_eq!(
        first.action.as_ref().unwrap().uses,
        "essence-of-enhancement"
    );
    assert_eq!(first.odds.as_ref().unwrap().percent(), Some(100.0));
    assert_eq!(route.bindings.get("essence").map(String::as_str), Some(ES));
    assert_eq!(route.steps[1].id, "slam");

    // Floor T3: only the Greater one reaches it.
    let plan = engine
        .plan(&gloves, &targets(&[(ES, Some(3)), (COLD, Some(5))]), &[])
        .unwrap();
    let action = plan.routes[0].steps[0].action.as_ref().unwrap();
    assert_eq!(action.uses, "greater-essence-of-enhancement");
    assert_eq!(action.alternative, 1);
}

#[test]
fn no_essence_route_when_the_essence_tier_is_too_low() {
    let engine = engine(&["engine/test-essence.toml"]);
    let t2 = targets(&[(ES, Some(2)), (COLD, Some(5))]);

    // No essence reaches T2: the Magic start (which needs one) fails.
    let plan = engine
        .plan(&item("engine/gloves-magic-cold.txt"), &t2, &[])
        .unwrap();
    assert!(plan.routes.is_empty());
    match &plan.rejected[0].reason {
        Rejection::NoStartFits { failed } => {
            assert!(
                failed[0].iter().any(|f| f.contains("essence")),
                "{failed:?}"
            );
        }
        other => panic!("expected no start, got {other:?}"),
    }

    // A Rare start: the route runs, its essence step left out.
    let plan = engine
        .plan(&item("engine/gloves-rare-cold.txt"), &t2, &[])
        .unwrap();
    let route = &plan.routes[0];
    assert_eq!(route.steps[0].id, "slam");
    assert_eq!(route.skipped[0].step, "essence");
    assert!(route.bindings.is_empty());
}

#[test]
fn desecration_takes_the_hardest_target() {
    let engine = engine(&["engine/test-desecrate.toml"]);
    let ring = item(RING);

    // Chaos resistance at T3 (500 of 75855 suffix weight) is harder than
    // life at T8 (8000 of 57500 prefix weight), wherever it is listed.
    for list in [
        [(LIFE, Some(8)), (CHAOS, Some(3))],
        [(CHAOS, Some(3)), (LIFE, Some(8))],
    ] {
        let plan = engine.plan(&ring, &targets(&list), &[]).unwrap();
        let route = &plan.routes[0];
        let desecrate = &route.steps[0];
        assert_eq!(desecrate.target.as_deref(), Some(CHAOS));
        let action = desecrate.action.as_ref().unwrap();
        assert_eq!(action.uses, "preserved-collarbone");
        assert_eq!(action.omens, ["omen-of-dextral-necromancy"]);
        assert_eq!(route.steps[1].id, "reveal");
        assert_eq!(
            route.steps[1].target.as_deref(),
            Some(CHAOS),
            "the reveal is the desecrated target"
        );
    }

    // Control: life at T1 (1000 of 57500) is harder than cold resistance
    // at T8 (7000 of 75855): the prefix side's omen.
    let plan = engine
        .plan(&ring, &targets(&[(COLD, Some(8)), (LIFE, Some(1))]), &[])
        .unwrap();
    let desecrate = &plan.routes[0].steps[0];
    assert_eq!(desecrate.target.as_deref(), Some(LIFE));
    assert_eq!(
        desecrate.action.as_ref().unwrap().omens,
        ["omen-of-sinistral-necromancy"]
    );
}

#[test]
fn desecrated_section_target_only_by_desecration() {
    let engine = engine(&["engine/test-slam.toml", "engine/test-desecrate.toml"]);
    let ring = item(RING);
    let list = targets(&[(LIFE, Some(8)), (SKILL_SPEED, None)]);
    let plan = engine.plan(&ring, &list, &[]).unwrap();

    assert_eq!(plan.routes.len(), 1);
    let route = &plan.routes[0];
    assert_eq!(route.blueprint, "test-desecrate");
    assert_eq!(route.steps[0].target.as_deref(), Some(SKILL_SPEED));
    let slam = plan
        .rejected
        .iter()
        .find(|r| r.blueprint == "test-slam")
        .unwrap();
    assert_eq!(
        slam.reason,
        Rejection::DesecratedTargetUnplaced {
            target: SKILL_SPEED.to_string()
        }
    );

    // Control: without the desecrated target, the exalt blueprint applies,
    // and its exalt never aims at a desecrated-section modifier.
    let plan = engine
        .plan(&ring, &targets(&[(LIFE, Some(8))]), &[])
        .unwrap();
    let slam = plan
        .routes
        .iter()
        .find(|r| r.blueprint == "test-slam")
        .unwrap();
    assert_eq!(slam.steps[0].targets, [LIFE]);
}

#[test]
fn blocked_currency_falls_back_to_the_next_alternative() {
    let ring = item(RING);
    let slam = engine(&["engine/test-slam.toml"]);
    let first_action = |blocked: &[String]| {
        let plan = slam.plan(&ring, &ring_targets(), blocked).unwrap();
        plan.routes[0].steps[0].action.clone().unwrap()
    };
    assert_eq!(first_action(&[]).uses, "perfect-exalted-orb");
    let action = first_action(&blocked(&["perfect-exalted-orb"]));
    assert_eq!(
        (action.alternative, action.uses.as_str()),
        (1, "greater-exalted-orb")
    );
    let action = first_action(&blocked(&["perfect-exalted-orb", "greater-exalted-orb"]));
    assert_eq!(
        (action.alternative, action.uses.as_str()),
        (2, "exalted-orb")
    );

    // Blocking applies to what `desecrate` resolves to.
    let desecrate = engine(&["engine/test-desecrate.toml"]);
    let list = targets(&[(LIFE, Some(8)), (CHAOS, Some(3))]);
    let plan = desecrate
        .plan(&ring, &list, &blocked(&["preserved-collarbone"]))
        .unwrap();
    assert_eq!(
        plan.routes[0].steps[0].action.as_ref().unwrap().uses,
        "ancient-collarbone"
    );

    // And to essences.
    let essence = engine(&["engine/test-essence.toml"]);
    let gloves = item("engine/gloves-magic-cold.txt");
    let plan = essence
        .plan(
            &gloves,
            &targets(&[(ES, Some(4)), (COLD, Some(5))]),
            &blocked(&["essence-of-enhancement"]),
        )
        .unwrap();
    let action = plan.routes[0].steps[0].action.as_ref().unwrap();
    assert_eq!(action.uses, "greater-essence-of-enhancement");
}

#[test]
fn every_alternative_blocked_rejects_the_blueprint() {
    let ring = item(RING);
    let slam = engine(&["engine/test-slam.toml"]);
    let all = blocked(&["perfect-exalted-orb", "greater-exalted-orb", "exalted-orb"]);
    let plan = slam.plan(&ring, &ring_targets(), &all).unwrap();
    assert!(plan.routes.is_empty());
    assert_eq!(
        plan.rejected[0].reason,
        Rejection::EveryAlternativeBlocked {
            step: "slam".to_string()
        }
    );

    // A side omen resolved and blocked blocks its alternative too.
    let desecrate = engine(&["engine/test-desecrate.toml"]);
    let list = targets(&[(LIFE, Some(8)), (CHAOS, Some(3))]);
    let plan = desecrate
        .plan(&ring, &list, &blocked(&["omen-of-dextral-necromancy"]))
        .unwrap();
    assert_eq!(
        plan.rejected[0].reason,
        Rejection::EveryAlternativeBlocked {
            step: "desecrate".to_string()
        }
    );
}

// --- Outcomes -------------------------------------------------------------

#[test]
fn hit_advances_to_the_next_step() {
    let engine = engine(&["engine/test-slam.toml"]);
    let before = item(RING);
    let (mut guide, step) = start(&engine, "test-slam", &before, &ring_targets(), &[]);
    assert_eq!(step.id, "slam");
    assert_eq!(step.targets, [LIFE]);

    let after = item("engine/ring-rare-life-hit.txt");
    let out = observe(&engine, &mut guide, &before, &after);
    assert_eq!(out.result, OutcomeKind::Hit);
    assert_eq!(out.gained.len(), 1);
    assert_eq!(
        (out.gained[0].mod_id.as_str(), out.gained[0].tier),
        (LIFE, Some(4))
    );
    assert!(out.lost.is_empty());
    assert_eq!(out.transition.as_deref(), Some("next"));
    assert_eq!(next_step(&out.next).id, "finish");
    assert_eq!(
        (guide.step.as_str(), &guide.status),
        ("finish", &Status::Active)
    );
}

#[test]
fn miss_follows_the_blueprint_transition() {
    let engine = engine(&["engine/test-slam.toml"]);
    let before = item(RING);
    let (mut guide, _) = start(&engine, "test-slam", &before, &ring_targets(), &[]);

    let after = item("engine/ring-rare-accuracy-miss.txt");
    let out = observe(&engine, &mut guide, &before, &after);
    assert_eq!(out.result, OutcomeKind::Miss);
    assert!(out.gained.is_empty() && out.lost.is_empty());
    assert_eq!(out.transition.as_deref(), Some("goto:clean"));
    let clean = next_step(&out.next);
    assert_eq!(clean.id, "clean");
    // Mana and accuracy are unwanted, fire resistance wanted: 2 of 3.
    match clean.odds.as_ref().unwrap() {
        Odds::Known { hits, total, .. } => assert_eq!((*hits, *total), (2, 3)),
        other => panic!("expected known odds, got {other:?}"),
    }
    assert_eq!(guide.step, "clean");
}

#[test]
fn below_floor_is_a_miss() {
    let engine = engine(&["engine/test-slam.toml"]);
    let before = item(RING);
    let (mut guide, _) = start(&engine, "test-slam", &before, &ring_targets(), &[]);

    // Life came in at T7; the floor is T5.
    let after = item("engine/ring-rare-life-below-floor.txt");
    let out = observe(&engine, &mut guide, &before, &after);
    assert_eq!(out.result, OutcomeKind::Miss);
    assert!(out.gained.is_empty());
    assert_eq!(out.below_floor.len(), 1);
    assert_eq!(
        (out.below_floor[0].mod_id.as_str(), out.below_floor[0].tier),
        (LIFE, Some(7))
    );
    // It counts as unwanted for the removal: mana and the T7 life of 3.
    let clean = next_step(&out.next);
    match clean.odds.as_ref().unwrap() {
        Odds::Known { hits, total, .. } => assert_eq!((*hits, *total), (2, 3)),
        other => panic!("expected known odds, got {other:?}"),
    }
}

#[test]
fn unexpected_when_the_change_does_not_fit_the_action() {
    let engine = engine(&["engine/test-slam.toml"]);
    let before = item(RING);

    // An exalt adds one modifier; one going and one coming is a chaos.
    let (mut guide, _) = start(&engine, "test-slam", &before, &ring_targets(), &[]);
    let out = observe(
        &engine,
        &mut guide,
        &before,
        &item("engine/ring-rare-fire-swapped.txt"),
    );
    assert_eq!(out.result, OutcomeKind::Unexpected);
    assert!(
        out.reasons
            .iter()
            .any(|r| r.contains("1 modifier(s) went and 1 came")),
        "{:?}",
        out.reasons
    );
    assert!(matches!(out.next, Next::Replan { .. }));
    assert!(matches!(guide.status, Status::Replan { .. }));
    assert!(out.transition.is_none());
    // The guide stops: it answers nothing more until a re-plan.
    assert!(engine.observe(&mut guide, &before, &before).is_err());

    // No currency changes the item level.
    let (mut guide, _) = start(&engine, "test-slam", &before, &ring_targets(), &[]);
    let hit_text = common::item_text("engine/ring-rare-life-hit.txt");
    let after = parse(&hit_text.replace("Item Level: 76", "Item Level: 80"));
    let out = observe(&engine, &mut guide, &before, &after);
    assert_eq!(out.result, OutcomeKind::Unexpected);
    assert!(
        out.reasons.iter().any(|r| r.contains("item level")),
        "{:?}",
        out.reasons
    );

    // Control: the same change at the right item level is a hit.
    let (mut guide, _) = start(&engine, "test-slam", &before, &ring_targets(), &[]);
    let out = observe(&engine, &mut guide, &before, &parse(&hit_text));
    assert_eq!(out.result, OutcomeKind::Hit);
}

#[test]
fn lost_wanted_mod_is_reported() {
    let engine = engine(&["engine/test-slam.toml"]);
    let ring = item(RING);
    let (mut guide, _) = start(&engine, "test-slam", &ring, &ring_targets(), &[]);
    let missed = item("engine/ring-rare-accuracy-miss.txt");
    observe(&engine, &mut guide, &ring, &missed);
    assert_eq!(guide.step, "clean");

    // The annulment took fire resistance, the wanted one: a miss that
    // reports the loss, and the step repeats.
    let annulled = item("engine/ring-rare-fire-annulled.txt");
    let out = observe(&engine, &mut guide, &missed, &annulled);
    assert_eq!(out.result, OutcomeKind::Miss);
    assert_eq!(out.lost.len(), 1);
    assert_eq!(
        (out.lost[0].mod_id.as_str(), out.lost[0].tier),
        (FIRE, Some(5))
    );
    assert_eq!(out.transition.as_deref(), Some("repeat"));
    assert_eq!((guide.step.as_str(), guide.tries), ("clean", 1));

    // An unexpected outcome reports it too.
    let (mut guide, _) = start(&engine, "test-slam", &ring, &ring_targets(), &[]);
    let out = observe(
        &engine,
        &mut guide,
        &ring,
        &item("engine/ring-rare-fire-swapped.txt"),
    );
    assert_eq!(out.result, OutcomeKind::Unexpected);
    assert_eq!(out.lost.len(), 1);
    assert_eq!(out.lost[0].mod_id, FIRE);
}

#[test]
fn odds_unknown_for_desecration_and_jewels() {
    let ring = item(RING);
    let desecrate = engine(&["engine/test-desecrate.toml"]);
    let plan = desecrate
        .plan(&ring, &targets(&[(LIFE, Some(8)), (CHAOS, Some(3))]), &[])
        .unwrap();
    let steps = &plan.routes[0].steps;
    assert!(unknown(&steps[0].odds).contains("desecration"));
    assert!(unknown(&steps[1].odds).contains("desecration"));

    // Every jewel page has no established spawn weights.
    let jewel = engine(&["engine/test-jewel.toml"]);
    let plan = jewel
        .plan(
            &item("engine/jewels-rare-sapphire.txt"),
            &targets(&[("prefix-increased-spell-damage", None)]),
            &[],
        )
        .unwrap();
    assert!(unknown(&plan.routes[0].steps[0].odds).contains("spawn weights"));

    // Control: an exalt on the ring has known odds.
    let slam = engine(&["engine/test-slam.toml"]);
    let blocked = blocked(&["perfect-exalted-orb", "greater-exalted-orb"]);
    let plan = slam.plan(&ring, &ring_targets(), &blocked).unwrap();
    let odds = plan.routes[0].steps[0].odds.as_ref().unwrap();
    // Life T5 or better, with no side omen both sides roll: the prefix
    // pool without mana (57500) and the suffix pool without fire (75855).
    match odds {
        Odds::Known { hits, total, .. } => assert_eq!((*hits, *total), (5000, 57500 + 75855)),
        other => panic!("expected known odds, got {other:?}"),
    }
}

// --- Refusals, the real fixtures, the state, the data-backed check -------

#[test]
fn refuses_what_it_cannot_plan_for() {
    let engine = engine(&["engine/test-slam.toml"]);
    let jewel = real("jewels-rare-corrupted-enhancement-advanced.txt");
    let list = targets(&[("prefix-increased-spell-damage", None)]);
    assert_eq!(
        engine.plan(&jewel, &list, &[]).unwrap_err(),
        Refusal::Corrupted
    );

    let ring = item(RING);
    let not_on_rings = engine
        .plan(&ring, &targets(&[(ES, None)]), &[])
        .unwrap_err();
    assert!(
        matches!(not_on_rings, Refusal::TargetNotValid { .. }),
        "{not_on_rings:?}"
    );
    // Chaos resistance T1 needs item level 81; the ring is 76.
    let beyond = engine
        .plan(&ring, &targets(&[(CHAOS, Some(1))]), &[])
        .unwrap_err();
    assert!(
        matches!(beyond, Refusal::TargetNotValid { .. }),
        "{beyond:?}"
    );
    assert_eq!(
        engine.plan(&ring, &[], &[]).unwrap_err(),
        Refusal::NoTargets
    );
}

#[test]
fn real_fixtures_plan() {
    // The real Amethyst Ring is Normal: a Rare start does not fit it.
    let engine = engine(&["engine/test-slam.toml", "engine/test-essence.toml"]);
    let ring = real("rings-normal-implicit-advanced.txt");
    let plan = engine.plan(&ring, &targets(&[(LIFE, None)]), &[]).unwrap();
    assert!(plan.routes.is_empty());
    let slam = plan
        .rejected
        .iter()
        .find(|r| r.blueprint == "test-slam")
        .unwrap();
    assert!(matches!(slam.reason, Rejection::NoStartFits { .. }));
    let essence = plan
        .rejected
        .iter()
        .find(|r| r.blueprint == "test-essence")
        .unwrap();
    assert!(matches!(essence.reason, Rejection::ClassNotCovered { .. }));

    // The real Sirenscale gloves already carry increased Energy Shield T2
    // and cold resistance T5: nothing is left to do.
    let gloves = real("gloves-rare-rune-desecrated-ctrl-alt-c.txt");
    let list = targets(&[(ES, Some(4)), (COLD, Some(5))]);
    let plan = engine.plan(&gloves, &list, &[]).unwrap();
    let route = &plan.routes[0];
    assert!(route.steps.is_empty());
    assert_eq!(route.skipped.len(), 2);
    let started = engine.start("test-essence", &gloves, &list, &[]).unwrap();
    assert_eq!(started.next, Next::Done);
    assert_eq!(started.guide.status, Status::Done);
}

#[test]
fn guide_state_round_trips_through_json() {
    let engine = engine(&["engine/test-slam.toml"]);
    let ring = item(RING);
    let (guide, _) = start(
        &engine,
        "test-slam",
        &ring,
        &ring_targets(),
        &blocked(&["orb-of-chance"]),
    );
    let json = serde_json::to_string(&guide).unwrap();
    let back: ch_engine::Guide = serde_json::from_str(&json).unwrap();
    assert_eq!(back, guide);
    // The step view and the plan serialise for M5.
    let plan = engine.plan(&ring, &ring_targets(), &[]).unwrap();
    assert!(serde_json::to_string(&plan)
        .unwrap()
        .contains("\"blueprint\":\"test-slam\""));
    assert_eq!(engine.current(&back, &ring).unwrap().id, "slam");
}

#[test]
fn data_backed_id_unknown_for_every_class_is_refused_with_its_line() {
    let text = "id = \"no-essence\"\nname = \"No essence\"\nclasses = [\"Rings\"]\nsource = \"tests\"\n\n[[start]]\nrarity = [\"magic\"]\n\n[[step]]\nid = \"essence\"\nsay = \"Use it\"\ngoal = \"add-wanted\"\n\n[[step.action]]\ncurrency = \"essence-of-enhancement\"\n";
    // Essence of Enhancement exists for gloves, not for rings.
    let checked = ch_blueprint::check("blueprints/no-essence.toml", text).unwrap();
    let errors = ch_engine::resolve(checked).unwrap_err();
    assert_eq!(errors.len(), 1);
    assert_eq!(
        errors[0].to_string(),
        "blueprints/no-essence.toml:15:12: step `essence`: no essence `essence-of-enhancement` in the data of Rings"
    );
    let gloves = text.replace("Rings", "Gloves");
    let checked = ch_blueprint::check("blueprints/no-essence.toml", &gloves).unwrap();
    assert!(ch_engine::resolve(checked).is_ok());
}

#[test]
fn every_catalogue_id_is_offered() {
    let all = ch_engine::catalogue();
    assert!(all
        .iter()
        .any(|c| c.id == "orb-of-annulment" && !c.data_backed));
    let enhancement = all
        .iter()
        .find(|c| c.id == "essence-of-enhancement")
        .unwrap();
    assert!(enhancement.data_backed && enhancement.classes.contains(&"Gloves".to_string()));
    assert!(all.iter().any(|c| c.id == "liquid-despair"));
}
