//! State-ordered blueprints (docs/models/crafting-slots-spec.md, "Route
//! rules" and "Projection"): the next step picked from the item as it
//! stands, the route preview as the path the craft takes if each step hits,
//! and the rejections, over the `test-state-*` blueprints and the
//! constructed `ring-state-*` items in `tests/items/engine/` (see its
//! README). The shipped v2 blueprints and the model's cases are
//! `tests/slots.rs`'s.

mod common;

use ch_engine::ch_blueprint::{Goal, Side};
use ch_engine::{Next, Odds, OutcomeKind, Rejection, Route, Status, StepView, Target};
use common::{
    blocked, engine, item, item_text, next_step, observe, parse, shipped, start, targets,
};

const MANA: &str = "prefix-to-maximum-mana";
const LIFE: &str = "prefix-to-maximum-life";
const ACC: &str = "prefix-to-accuracy-rating";
const EVASION: &str = "prefix-to-evasion-rating";
const FIRE: &str = "suffix-to-fire-resistance";
const COLD: &str = "suffix-to-cold-resistance";
const LIGHTNING: &str = "suffix-to-lightning-resistance";
const CHAOS: &str = "suffix-to-chaos-resistance";

const CLOSE: &str = "engine/test-state-close.toml";
const CLEAN: &str = "engine/test-state-clean.toml";

/// Mana T5 and Life T4 (prefixes) and Fire Resistance T5 (suffix) on the
/// ring; Accuracy Rating T3, a third prefix, is missing. The owner's 2P/1S
/// craft: the prefix side is reserved, the suffix side closable.
const TWO_P_ONE_S: &str = "engine/ring-rare-life-hit.txt";

fn two_p_one_s_targets() -> Vec<Target> {
    targets(&[
        (MANA, Some(5)),
        (LIFE, Some(4)),
        (ACC, Some(3)),
        (FIRE, Some(5)),
    ])
}

/// Mana, a missing Life T4, Fire: on `ring-state-prefix-blocked` and
/// `ring-state-whittle` the full prefix side is blocked.
fn blocked_prefix_targets() -> Vec<Target> {
    targets(&[(MANA, Some(5)), (LIFE, Some(4)), (FIRE, Some(5))])
}

/// The route of `blueprint` on the item; panics when it is rejected.
fn plan_route(
    rels: &[&str],
    blueprint: &str,
    file: &str,
    list: &[Target],
    block: &[&str],
) -> Route {
    let plan = engine(rels)
        .plan(&item(file), list, &blocked(block))
        .unwrap();
    if let Some(r) = plan.rejected.iter().find(|r| r.blueprint == blueprint) {
        panic!("{blueprint} is rejected on {file}: {}", r.reason);
    }
    plan.routes
        .into_iter()
        .find(|r| r.blueprint == blueprint)
        .unwrap()
}

/// Why `blueprint` is rejected on the item; panics when it is not.
fn rejection(rels: &[&str], blueprint: &str, file: &str, list: &[Target]) -> Rejection {
    let plan = engine(rels).plan(&item(file), list, &[]).unwrap();
    plan.rejected
        .into_iter()
        .find(|r| r.blueprint == blueprint)
        .unwrap_or_else(|| panic!("{blueprint} is not rejected on {file}"))
        .reason
}

/// A step as the preview shows it: goal, side, currency as written, the
/// omens resolved.
fn shape(step: &StepView) -> (Goal, Option<Side>, String, Vec<String>) {
    let action = step.action.as_ref().expect("a step with an action");
    (
        step.goal,
        step.side,
        action.currency.clone(),
        action.omens.clone(),
    )
}

fn step(
    goal: Goal,
    side: Option<Side>,
    currency: &str,
    omens: &[&str],
) -> (Goal, Option<Side>, String, Vec<String>) {
    (
        goal,
        side,
        currency.to_string(),
        omens.iter().map(|o| o.to_string()).collect(),
    )
}

fn percent(step: &StepView) -> Option<f64> {
    step.odds.as_ref().and_then(Odds::percent)
}

// --- The guide -------------------------------------------------------------

#[test]
fn a_state_ordered_guide_picks_the_next_step_from_the_item() {
    let engine = engine(&[CLOSE]);
    let list = two_p_one_s_targets();
    let ring = item(TWO_P_ONE_S);

    // The suffix side holds its only target and two free slots: close it.
    let (mut guide, first) = start(&engine, "test-state-close", &ring, &list, &[]);
    assert_eq!(first.id, "close");
    assert_eq!(first.side, Some(Side::Suffix));

    // The Greater exalt filled the suffix side: with one side open, the
    // next step is the open side's slam, picked from the item, not from a
    // transition.
    let closed = item("engine/ring-state-closed-suffix.txt");
    let out = observe(&engine, &mut guide, &ring, &closed);
    assert!(out.reasons.is_empty(), "{:?}", out.reasons);
    assert_eq!(out.result, OutcomeKind::Hit);
    let next = next_step(&out.next);
    assert_eq!(next.id, "slam-open-side");
    assert_eq!(
        shape(next),
        step(
            Goal::AddWanted,
            Some(Side::Prefix),
            "perfect-exalted-orb",
            &[]
        )
    );
    assert_eq!(guide.step, "slam-open-side");
    assert_eq!(guide.status, Status::Active);

    // A craft started on the closed item starts where it is.
    let (_, mid) = start(&engine, "test-state-close", &closed, &list, &[]);
    assert_eq!(mid.id, "slam-open-side");
    assert_eq!(
        engine.current(&guide, &closed).unwrap().id,
        "slam-open-side"
    );
}

#[test]
fn a_state_ordered_guide_is_done_when_no_target_is_missing() {
    let engine = engine(&[CLOSE, CLEAN]);
    let list = two_p_one_s_targets();

    // Every target on the item: the preview has no step, the guide is done.
    // (`test-state-close` asks for an open side to start from.)
    let complete = item("engine/ring-state-complete.txt");
    let plan = engine.plan(&complete, &list, &[]).unwrap();
    assert_eq!(plan.routes.len(), 1);
    assert_eq!(plan.routes[0].blueprint, "test-state-clean");
    assert!(plan.routes[0].steps.is_empty());
    let started = engine
        .start("test-state-clean", &complete, &list, &[])
        .unwrap();
    assert_eq!(started.next, Next::Done);
    assert_eq!(started.guide.status, Status::Done);

    // The last slam hits: done, with no step left.
    let closed = item("engine/ring-state-closed-suffix.txt");
    let (mut guide, _) = start(&engine, "test-state-close", &closed, &list, &[]);
    let out = observe(&engine, &mut guide, &closed, &complete);
    assert_eq!(out.result, OutcomeKind::Hit);
    assert_eq!(out.next, Next::Done);
    assert_eq!(guide.status, Status::Done);
}

// --- The preview -----------------------------------------------------------

#[test]
fn a_state_ordered_route_previews_the_hit_path() {
    let route = plan_route(
        &[CLOSE],
        "test-state-close",
        TWO_P_ONE_S,
        &two_p_one_s_targets(),
        &[],
    );
    let shapes: Vec<_> = route.steps.iter().map(shape).collect();
    assert_eq!(
        shapes,
        [
            step(
                Goal::Fill,
                Some(Side::Suffix),
                "exalted-orb",
                &["omen-of-greater-exaltation", "omen-of-dextral-exaltation"],
            ),
            step(
                Goal::AddWanted,
                Some(Side::Prefix),
                "perfect-exalted-orb",
                &[]
            ),
        ]
    );
    // The first step's odds are read off the item; the second's off the
    // item as it stands if the first hit, and say so.
    assert_eq!(percent(&route.steps[0]), Some(100.0));
    match route.steps[1].odds.as_ref().unwrap() {
        Odds::Known { basis, percent, .. } => {
            assert!(*percent > 0.0 && *percent < 100.0, "{percent}");
            assert!(basis.contains("if the earlier steps hit"), "{basis}");
        }
        other => panic!("expected known odds, got {other:?}"),
    }

    // A rare with every target but one on a blocked side: a step may come
    // twice, and the preview takes it as far as the item is done.
    let route = plan_route(
        &[CLEAN],
        "test-state-clean",
        "engine/ring-state-no-target.txt",
        &targets(&[
            (MANA, Some(5)),
            (LIFE, Some(4)),
            (FIRE, Some(5)),
            (LIGHTNING, Some(5)),
        ]),
        &[],
    );
    let goals: Vec<(Goal, Option<Side>)> = route.steps.iter().map(|s| (s.goal, s.side)).collect();
    assert_eq!(
        goals,
        [
            (Goal::RemoveUnwanted, Some(Side::Prefix)),
            (Goal::RemoveUnwanted, Some(Side::Suffix)),
            (Goal::AddWanted, Some(Side::Prefix)),
            (Goal::AddWanted, Some(Side::Suffix)),
        ]
    );
    assert_eq!(route.steps[2].action.as_ref().unwrap().omens.len(), 2);
}

// --- Fitting an alternative -------------------------------------------------

#[test]
fn an_unrestricted_exalt_is_passed_over_while_a_reserved_side_is_open() {
    let rels = ["engine/test-state-unrestricted.toml"];
    // Prefix reserved (one free slot, Accuracy missing), suffix open: the
    // plain Exalted Orb could take the prefix slot, so the side one.
    let route = plan_route(
        &rels,
        "test-state-unrestricted",
        TWO_P_ONE_S,
        &two_p_one_s_targets(),
        &[],
    );
    let first = route.steps[0].action.as_ref().unwrap();
    assert_eq!(first.alternative, 1);
    assert_eq!(first.omens, ["omen-of-sinistral-exaltation"]);

    // Negative control: two free prefixes for one missing prefix, so no
    // side is reserved, and the plain exalt is taken.
    let route = plan_route(
        &rels,
        "test-state-unrestricted",
        "engine/ring-rare-mana-fire.txt",
        &targets(&[(MANA, Some(5)), (ACC, Some(3)), (FIRE, Some(5))]),
        &[],
    );
    let first = route.steps[0].action.as_ref().unwrap();
    assert_eq!(first.alternative, 0);
    assert!(first.omens.is_empty());
}

#[test]
fn greater_omen_resolves_only_where_two_fit() {
    let greater = "omen-of-greater-exaltation";
    // A fill on a side with two free slots: Greater.
    let route = plan_route(
        &[CLOSE],
        "test-state-close",
        TWO_P_ONE_S,
        &two_p_one_s_targets(),
        &[],
    );
    assert!(route.steps[0]
        .action
        .as_ref()
        .unwrap()
        .omens
        .contains(&greater.to_string()));

    // One free suffix slot: the fill takes the side omen alone.
    let route = plan_route(
        &[CLOSE],
        "test-state-close",
        "engine/ring-state-one-suffix-free.txt",
        &two_p_one_s_targets(),
        &[],
    );
    assert_eq!(
        shape(&route.steps[0]),
        step(
            Goal::Fill,
            Some(Side::Suffix),
            "exalted-orb",
            &["omen-of-dextral-exaltation"]
        )
    );

    // An add with two free prefixes and two prefixes missing: Greater.
    let mana_fire = "engine/ring-rare-mana-fire.txt";
    let two_missing = targets(&[
        (MANA, Some(5)),
        (LIFE, Some(4)),
        (ACC, Some(3)),
        (FIRE, Some(5)),
    ]);
    let route = plan_route(&[CLEAN], "test-state-clean", mana_fire, &two_missing, &[]);
    assert_eq!(
        shape(&route.steps[0]),
        step(
            Goal::AddWanted,
            Some(Side::Prefix),
            "perfect-exalted-orb",
            &[greater, "omen-of-sinistral-exaltation"],
        )
    );
    assert_eq!(route.steps.len(), 1, "two targets placed by one step");

    // Two free prefixes but one missing: no Greater.
    let one_missing = targets(&[(MANA, Some(5)), (ACC, Some(3)), (FIRE, Some(5))]);
    let route = plan_route(&[CLEAN], "test-state-clean", mana_fire, &one_missing, &[]);
    assert_eq!(
        route.steps[0].action.as_ref().unwrap().omens,
        ["omen-of-sinistral-exaltation"]
    );

    // Greater blocked: nothing in its place, the same alternative, no
    // fallback; the route takes two slams instead of one.
    let route = plan_route(
        &[CLEAN],
        "test-state-clean",
        mana_fire,
        &two_missing,
        &[greater],
    );
    let first = route.steps[0].action.as_ref().unwrap();
    assert_eq!(first.alternative, 0);
    assert_eq!(first.omens, ["omen-of-sinistral-exaltation"]);
    assert_eq!(route.steps.len(), 2);
}

// --- Removals ------------------------------------------------------------------

#[test]
fn a_removal_names_the_side_annulment_when_the_other_side_holds_a_target() {
    // Prefixes Mana (a target), Accuracy and Cold Damage, full, with Life
    // missing: blocked. The lowest modifier level is the Fire Resistance
    // target's, so no whittle; the suffix side holds that target, so the
    // annulment is confined to the prefix side.
    let route = plan_route(
        &[CLEAN],
        "test-state-clean",
        "engine/ring-state-prefix-blocked.txt",
        &blocked_prefix_targets(),
        &[],
    );
    let first = &route.steps[0];
    assert_eq!(
        shape(first),
        step(
            Goal::RemoveUnwanted,
            Some(Side::Prefix),
            "orb-of-annulment",
            &["omen-of-sinistral-annulment"],
        )
    );
    // Two of the three prefixes it may take are not targets.
    let p = percent(first).unwrap();
    assert!((p - 200.0 / 3.0).abs() < 0.01, "{p}");
}

#[test]
fn a_removal_takes_the_plain_annulment_when_the_other_side_holds_no_target() {
    // Issue #28's shape: one target (Mana) among two prefixes, two junk
    // suffixes; Life and Evasion missing on the prefix side, Fire on the
    // suffix side. The suffix side holds no target to protect.
    let list = targets(&[
        (MANA, Some(5)),
        (LIFE, Some(4)),
        (EVASION, Some(4)),
        (FIRE, Some(5)),
    ]);
    let route = plan_route(
        &[CLEAN],
        "test-state-clean",
        "engine/ring-state-one-target.txt",
        &list,
        &[],
    );
    let first = &route.steps[0];
    assert_eq!(
        shape(first),
        step(
            Goal::RemoveUnwanted,
            Some(Side::Prefix),
            "orb-of-annulment",
            &[]
        )
    );
    // Any of the four may go; three are not targets.
    assert_eq!(percent(first), Some(75.0));
}

#[test]
fn a_whittle_is_taken_only_when_certain_and_an_annulment_risks_a_target() {
    // The Rarity prefix is the one lowest-level modifier, and the side
    // annulment that would otherwise fit could take the Mana target: the
    // whittle, certain.
    let whittle = "engine/ring-state-whittle.txt";
    let route = plan_route(
        &[CLEAN],
        "test-state-clean",
        whittle,
        &blocked_prefix_targets(),
        &[],
    );
    let first = &route.steps[0];
    assert_eq!(
        shape(first),
        step(
            Goal::RemoveUnwanted,
            Some(Side::Prefix),
            "chaos-orb",
            &["omen-of-whittling"],
        )
    );
    assert_eq!(percent(first), Some(100.0));
    // The guide classifies the whittle that took it as a hit.
    let engine = engine(&[CLEAN]);
    let ring = item(whittle);
    let (mut guide, _) = start(
        &engine,
        "test-state-clean",
        &ring,
        &blocked_prefix_targets(),
        &[],
    );
    // The Rarity prefix went and a Cold Resistance suffix came, as a Chaos
    // Orb does.
    let after = item("engine/ring-state-whittled.txt");
    let out = observe(&engine, &mut guide, &ring, &after);
    assert!(out.reasons.is_empty(), "{:?}", out.reasons);
    assert_eq!(out.result, OutcomeKind::Hit);
    assert_eq!(next_step(&out.next).id, "side-slam");

    // Whittling blocked: the side annulment, no longer certain.
    let route = plan_route(
        &[CLEAN],
        "test-state-clean",
        whittle,
        &blocked_prefix_targets(),
        &["omen-of-whittling"],
    );
    assert_eq!(
        route.steps[0].action.as_ref().unwrap().omens,
        ["omen-of-sinistral-annulment"]
    );

    // Not certain: the lowest level is a target's (Fire Resistance).
    let route = plan_route(
        &[CLEAN],
        "test-state-clean",
        "engine/ring-state-prefix-blocked.txt",
        &blocked_prefix_targets(),
        &[],
    );
    assert_eq!(
        route.steps[0].action.as_ref().unwrap().currency,
        "orb-of-annulment"
    );

    // Certain, but no annulment could take a target (the item holds
    // none): the plain annulment, certain as well.
    let route = plan_route(
        &[CLEAN],
        "test-state-clean",
        "engine/ring-state-no-target.txt",
        &targets(&[
            (MANA, Some(5)),
            (LIFE, Some(4)),
            (FIRE, Some(5)),
            (LIGHTNING, Some(5)),
        ]),
        &[],
    );
    let first = &route.steps[0];
    assert_eq!(
        shape(first),
        step(
            Goal::RemoveUnwanted,
            Some(Side::Prefix),
            "orb-of-annulment",
            &[]
        )
    );
    assert_eq!(percent(first), Some(100.0));
}

// --- Desecration ---------------------------------------------------------------

#[test]
fn a_desecration_waits_for_a_free_slot_on_its_side() {
    let rels = ["engine/test-state-desecrate.toml"];
    // Life, the only missing target, is on the full prefix side: the
    // desecration comes first in the file but waits for the clean-up.
    let route = plan_route(
        &rels,
        "test-state-desecrate",
        "engine/ring-state-prefix-blocked.txt",
        &blocked_prefix_targets(),
        &[],
    );
    let shapes: Vec<_> = route.steps.iter().map(shape).collect();
    assert_eq!(
        shapes,
        [
            step(
                Goal::RemoveUnwanted,
                Some(Side::Prefix),
                "orb-of-annulment",
                &["omen-of-sinistral-annulment"],
            ),
            step(
                Goal::Desecrate,
                Some(Side::Prefix),
                "desecrate",
                &["omen-of-sinistral-necromancy"],
            ),
            step(Goal::Reveal, None, "reveal", &["omen-of-abyssal-echoes"]),
        ]
    );
    assert_eq!(route.steps[1].target.as_deref(), Some(LIFE));
    assert!(route.skipped.iter().any(|s| s.step == "side-slam"));

    // Negative control: with a free prefix slot the desecration is first.
    let route = plan_route(
        &rels,
        "test-state-desecrate",
        "engine/ring-rare-mana-fire.txt",
        &blocked_prefix_targets(),
        &[],
    );
    assert_eq!(route.steps[0].goal, Goal::Desecrate);
}

/// The ring after a wrong reveal with a prefix slot free (the slot case
/// `rare-wrong-reveal-same-side-exalt`): Mana T5 and a revealed desecrated
/// Remnant effect prefix, not a target; Fire Resistance T5. Life T4 missing.
const WRONG_SAME_SIDE: &str = "slots/rare-wrong-reveal-same-side-exalt.txt";

#[test]
fn a_wrong_reveal_waits_for_the_same_side_exalt() {
    let engine = shipped();
    let list = blocked_prefix_targets();
    let wrong = item(WRONG_SAME_SIDE);
    let slam = step(
        Goal::AddWanted,
        Some(Side::Prefix),
        "perfect-exalted-orb",
        &["omen-of-sinistral-exaltation"],
    );
    let light = step(
        Goal::RemoveDesecrated,
        None,
        "orb-of-annulment",
        &["omen-of-light"],
    );

    // Omen of Light waits: the prefix exalt goes first, the desecrated
    // modifier kept through it, and the route is done without Light.
    for (blueprint, light_id) in [
        ("essence-desecrate-double-slam", "light"),
        ("rare-clean-then-slam", "light-last"),
    ] {
        let route = engine
            .plan(&wrong, &list, &[])
            .unwrap()
            .routes
            .into_iter()
            .find(|r| r.blueprint == blueprint)
            .unwrap_or_else(|| panic!("{blueprint} does not route"));
        let shapes: Vec<_> = route.steps.iter().map(shape).collect();
        assert_eq!(shapes, std::slice::from_ref(&slam), "{blueprint}");
        let skipped = route
            .skipped
            .iter()
            .find(|s| s.step == light_id)
            .unwrap_or_else(|| panic!("{blueprint}: `{light_id}` not skipped"));
        assert!(
            skipped.why.contains("waits for `side-slam`"),
            "{blueprint}: {}",
            skipped.why
        );

        // The guide picks the same, and after the exalt hits the craft is
        // done; after it misses the prefix side is full and Light comes.
        let (mut guide, first) = start(&engine, blueprint, &wrong, &list, &[]);
        assert_eq!(shape(&first), slam, "{blueprint}");
        let hit = item("slots/rare-wrong-reveal.txt");
        let out = observe(&engine, &mut guide, &wrong, &hit);
        assert_eq!(out.result, OutcomeKind::Hit, "{blueprint}: {out:?}");
        assert_eq!(out.next, Next::Done, "{blueprint}");

        let (mut guide, _) = start(&engine, blueprint, &wrong, &list, &[]);
        let miss = parse(&item_text(WRONG_SAME_SIDE).replace(
            "{ Desecrated Prefix",
            "{ Prefix Modifier \"Vaporous\" (Tier: 3) — Defences, Evasion }\n\
             +160(142-174) to Evasion Rating\n{ Desecrated Prefix",
        ));
        let out = observe(&engine, &mut guide, &wrong, &miss);
        assert_eq!(out.result, OutcomeKind::Miss, "{blueprint}: {out:?}");
        let next = next_step(&out.next);
        assert_eq!(next.id, light_id);
        assert_eq!(shape(next), light, "{blueprint}");
    }

    // Baseline: the prefix side full (`rare-wrong-reveal`), Light comes
    // first, as before the rule.
    let full = item("slots/rare-wrong-reveal.txt");
    let (_, first) = start(
        &engine,
        "essence-desecrate-double-slam",
        &full,
        &two_p_one_s_targets(),
        &[],
    );
    assert_eq!(shape(&first), light);

    // Control: with Omen of Sinistral Exaltation blocked no add can place
    // the prefix, so Light does not wait.
    let (_, first) = start(
        &engine,
        "rare-clean-then-slam",
        &wrong,
        &list,
        &blocked(&["omen-of-sinistral-exaltation"]),
    );
    assert_eq!(first.id, "light-last");
}

// --- Rejections ----------------------------------------------------------------

#[test]
fn targets_beyond_the_slots_reject_a_state_ordered_route() {
    // Four prefix targets for a ring's three prefix slots.
    let list = targets(&[
        (MANA, Some(5)),
        (LIFE, Some(4)),
        (ACC, Some(3)),
        (EVASION, Some(4)),
        (FIRE, Some(5)),
    ]);
    match rejection(
        &[CLEAN],
        "test-state-clean",
        "engine/ring-rare-mana-fire.txt",
        &list,
    ) {
        Rejection::TargetsExceedSlots {
            side,
            targets,
            slots,
        } => {
            assert_eq!(side, Side::Prefix);
            assert_eq!((targets, slots), (4, 3));
        }
        other => panic!("expected targets-exceed-slots, got {other:?}"),
    }
}

#[test]
fn a_route_whose_hit_path_cannot_finish_is_rejected() {
    // No candidate: only a prefix step, and a suffix is missing.
    let list = targets(&[(MANA, Some(5)), (FIRE, Some(5)), (COLD, Some(5))]);
    let rels = ["engine/test-state-prefix-only.toml"];
    match rejection(
        &rels,
        "test-state-prefix-only",
        "engine/ring-rare-mana-fire.txt",
        &list,
    ) {
        Rejection::RouteCannotFinish { why, reasons, .. } => {
            assert_eq!(why.to_string(), "no-candidate");
            assert!(
                reasons.iter().any(|r| r.contains("prefix-slam")),
                "{reasons:?}"
            );
        }
        other => panic!("expected route-cannot-finish, got {other:?}"),
    }

    // The step cap: a fill that blocks its own side and an annulment that
    // takes the filler again.
    let rels = ["engine/test-state-loop.toml"];
    match rejection(
        &rels,
        "test-state-loop",
        TWO_P_ONE_S,
        &two_p_one_s_targets(),
    ) {
        Rejection::RouteCannotFinish { why, .. } => assert_eq!(why.to_string(), "step-cap"),
        other => panic!("expected route-cannot-finish, got {other:?}"),
    }

    // No change: Omen of Light on a desecrated target takes nothing the
    // projection can remove.
    let rels = ["engine/test-state-light.toml"];
    let list = targets(&[(MANA, Some(5)), (CHAOS, Some(3)), (LIGHTNING, Some(5))]);
    match rejection(
        &rels,
        "test-state-light",
        "engine/ring-state-desecrated-target.txt",
        &list,
    ) {
        Rejection::RouteCannotFinish { why, step, .. } => {
            assert_eq!(why.to_string(), "no-change");
            assert_eq!(step.as_deref(), Some("light"));
        }
        other => panic!("expected route-cannot-finish, got {other:?}"),
    }

    // The rejections serialise with their kind.
    let json = serde_json::to_string(&rejection(
        &rels,
        "test-state-light",
        "engine/ring-state-desecrated-target.txt",
        &list,
    ))
    .unwrap();
    assert!(json.contains("\"kind\":\"route-cannot-finish\""), "{json}");
    assert!(json.contains("\"why\":\"no-change\""), "{json}");
}
