//! A side Exaltation omen on an exalt whose other side is full
//! (docs/models/crafting-slots-spec.md, issue #42): an Exalted Orb of any
//! grade can only land on the side that has room, so Omen of Sinistral or
//! Dextral Exaltation naming that side would be spent for nothing. The
//! engine leaves it out of the chosen action, in both orders, and a blocked
//! one then passes nothing over; while the other side has room the omen is
//! named as before. Over the parity cases' rings in `tests/items/slots/`,
//! the shipped v2 blueprints and the listed `test-literal-exalt` (see
//! `tests/items/engine/README.md`).

mod common;

use ch_engine::ch_blueprint::Side;
use ch_engine::{Engine, Route, StepView, Target};
use common::{blocked, engine, item, shipped, start, targets};

const MANA: &str = "prefix-to-maximum-mana";
const LIFE: &str = "prefix-to-maximum-life";
const ACC: &str = "prefix-to-accuracy-rating";
const FIRE: &str = "suffix-to-fire-resistance";
const COLD: &str = "suffix-to-cold-resistance";
const LIGHTNING: &str = "suffix-to-lightning-resistance";
const SKILL_SPEED: &str = "desecrated-suffix-increased-skill-speed";

const SINISTRAL: &str = "omen-of-sinistral-exaltation";
const DEXTRAL: &str = "omen-of-dextral-exaltation";
const GREATER: &str = "omen-of-greater-exaltation";
const DEXTRAL_NECROMANCY: &str = "omen-of-dextral-necromancy";

const LITERAL: &str = "engine/test-literal-exalt.toml";

/// Mana T4, Life T1 and Accuracy Rating T4 (prefixes, full); Fire
/// Resistance T4 (suffix). Cold and Lightning Resistance are missing.
const PREFIXES_FULL: &str = "slots/rare-prefixes-full.txt";

/// Mana T4 (prefix); Fire T4, Cold T5 and a revealed desecrated Skill
/// Speed (suffixes, full). Life is missing.
const SUFFIXES_FULL: &str = "slots/rare-suffixes-full.txt";

/// Mana T5 and Life T4 (prefixes); Fire Resistance T5 (suffix): both
/// sides have room.
const ROOM_ON_BOTH: &str = "slots/rare-2p1s-prefix-missing.txt";

/// Issue #28's belt: two prefixes and two suffixes, none a target.
const NO_TARGET_BELT: &str = "slots/rare-no-target.txt";

fn prefixes_full_targets() -> Vec<Target> {
    targets(&[
        (MANA, Some(4)),
        (LIFE, Some(4)),
        (ACC, Some(4)),
        (FIRE, Some(4)),
        (COLD, Some(2)),
        (LIGHTNING, Some(3)),
    ])
}

fn suffixes_full_targets() -> Vec<Target> {
    targets(&[
        (MANA, Some(4)),
        (LIFE, Some(4)),
        (FIRE, Some(4)),
        (COLD, Some(5)),
        (SKILL_SPEED, Some(1)),
    ])
}

fn room_on_both_targets() -> Vec<Target> {
    targets(&[
        (MANA, Some(5)),
        (LIFE, Some(4)),
        (ACC, Some(3)),
        (FIRE, Some(5)),
        (COLD, Some(5)),
    ])
}

fn no_target_belt_targets() -> Vec<Target> {
    targets(&[
        (LIFE, Some(1)),
        (MANA, Some(2)),
        (COLD, Some(3)),
        (FIRE, Some(3)),
    ])
}

/// The route of `blueprint` on `file`; panics when it is rejected.
fn route_of(
    engine: &Engine,
    blueprint: &str,
    file: &str,
    list: &[Target],
    block: &[&str],
) -> Route {
    let plan = engine.plan(&item(file), list, &blocked(block)).unwrap();
    if let Some(r) = plan.rejected.iter().find(|r| r.blueprint == blueprint) {
        panic!("{blueprint} is rejected on {file}: {}", r.reason);
    }
    plan.routes
        .into_iter()
        .find(|r| r.blueprint == blueprint)
        .unwrap()
}

/// A step's currency as written and its omens, resolved.
fn taken(step: &StepView) -> (String, Vec<String>) {
    let action = step.action.as_ref().expect("an action");
    (action.currency.clone(), action.omens.clone())
}

fn owned(ids: &[&str]) -> Vec<String> {
    ids.iter().map(|s| s.to_string()).collect()
}

/// The exalt steps of a route: (side, currency, omens).
fn exalts(route: &Route) -> Vec<(Option<Side>, String, Vec<String>)> {
    route
        .steps
        .iter()
        .filter(|s| {
            s.action
                .as_ref()
                .is_some_and(|a| a.currency.ends_with("exalted-orb"))
        })
        .map(|s| {
            let (currency, omens) = taken(s);
            (s.side, currency, omens)
        })
        .collect()
}

/// The baseline: while the other side has a free slot the side omen is
/// named. On issue #28's belt, cleaned to one prefix and one suffix, the
/// prefix slam keeps Omen of Sinistral Exaltation (the suffix side has
/// room); the listed literal omen is `listed_literal_…_kept_…`'s.
#[test]
fn baseline_side_exaltation_omen_kept_while_the_other_side_has_room() {
    let route = route_of(
        &shipped(),
        "rare-clean-then-slam",
        NO_TARGET_BELT,
        &no_target_belt_targets(),
        &[],
    );
    let slams = exalts(&route);
    assert_eq!(
        slams.first(),
        Some(&(
            Some(Side::Prefix),
            "perfect-exalted-orb".to_string(),
            owned(&[GREATER, SINISTRAL])
        )),
        "{slams:?}"
    );
    let action = route
        .steps
        .iter()
        .find_map(|s| {
            s.action
                .as_ref()
                .filter(|a| a.omens.iter().any(|o| o == SINISTRAL))
        })
        .unwrap();
    assert_eq!(action.side, Some(Side::Prefix));
    assert!(action
        .omen_names
        .contains(&"Omen of Sinistral Exaltation".to_string()));
    // Nothing left out, and the serialized action is as before.
    assert!(action.omens_left_out.is_empty());
    let json = serde_json::to_value(action).unwrap();
    assert!(json.get("omens_left_out").is_none(), "{json}");
}

/// The prefixes full: the suffix slam names no Dextral Exaltation, in the
/// preview of both state-ordered slams and in the guide's current step.
/// Greater stays (two suffixes fit, two targets there).
#[test]
fn full_prefixes_drop_dextral_exaltation() {
    let engine = shipped();
    let list = prefixes_full_targets();
    for bp in ["rare-clean-then-slam", "close-a-side"] {
        let route = route_of(&engine, bp, PREFIXES_FULL, &list, &[]);
        assert_eq!(
            exalts(&route),
            vec![(
                Some(Side::Suffix),
                "perfect-exalted-orb".to_string(),
                owned(&[GREATER])
            )],
            "{bp}"
        );
    }

    let (_, step) = start(
        &engine,
        "rare-clean-then-slam",
        &item(PREFIXES_FULL),
        &list,
        &[],
    );
    let action = step.action.as_ref().unwrap();
    assert_eq!(step.side, Some(Side::Suffix));
    assert_eq!(action.omens, owned(&[GREATER]));
    assert_eq!(action.omen_names, owned(&["Omen of Greater Exaltation"]));
    assert_eq!(action.alternative, 0);
    // What the app's fallback line reads to leave the omen out too.
    assert_eq!(action.omens_left_out, owned(&[DEXTRAL]));

    // The desecration route's last slam, one suffix free: no omen at all.
    let route = route_of(
        &engine,
        "essence-desecrate-double-slam",
        PREFIXES_FULL,
        &list,
        &[],
    );
    let last = route.steps.last().unwrap();
    assert_eq!(taken(last), ("perfect-exalted-orb".to_string(), Vec::new()));
    assert_eq!(last.side, Some(Side::Suffix));
}

/// The suffixes full: the prefix slam names no Sinistral Exaltation (and no
/// Greater: one target), in every state-ordered slam and in the guide.
#[test]
fn full_suffixes_drop_sinistral_exaltation() {
    let engine = shipped();
    let list = suffixes_full_targets();
    for bp in [
        "rare-clean-then-slam",
        "close-a-side",
        "essence-desecrate-double-slam",
    ] {
        let route = route_of(&engine, bp, SUFFIXES_FULL, &list, &[]);
        assert_eq!(
            exalts(&route),
            vec![(
                Some(Side::Prefix),
                "perfect-exalted-orb".to_string(),
                Vec::new()
            )],
            "{bp}"
        );
    }
    let (_, step) = start(
        &engine,
        "rare-clean-then-slam",
        &item(SUFFIXES_FULL),
        &list,
        &[],
    );
    let action = step.action.as_ref().unwrap();
    assert!(action.omens.is_empty(), "{:?}", action.omens);
    assert!(action.omen_names.is_empty(), "{:?}", action.omen_names);
    assert_eq!(action.omens_left_out, owned(&[SINISTRAL]));
    assert_eq!(step.side, Some(Side::Prefix));
}

/// A blocked Omen of Dextral Exaltation that would be left out passes no
/// alternative over: every route is the unblocked one, its Perfect Exalted
/// Orb (the first alternative) taken. The essence of the old behaviour was
/// a rejection: every suffix slam blocked.
#[test]
fn blocked_redundant_exaltation_omen_does_not_pass_the_exalt_over() {
    let engine = shipped();
    let list = prefixes_full_targets();
    for bp in [
        "rare-clean-then-slam",
        "close-a-side",
        "essence-desecrate-double-slam",
    ] {
        let free = route_of(&engine, bp, PREFIXES_FULL, &list, &[]);
        let blocked = route_of(&engine, bp, PREFIXES_FULL, &list, &[DEXTRAL]);
        assert_eq!(blocked.steps, free.steps, "{bp}");
    }
    let (_, step) = start(
        &engine,
        "rare-clean-then-slam",
        &item(PREFIXES_FULL),
        &list,
        &blocked(&[DEXTRAL]),
    );
    let action = step.action.as_ref().unwrap();
    assert_eq!(action.alternative, 0);
    assert_eq!(action.currency, "perfect-exalted-orb");
    assert!(!action.omens.iter().any(|o| o == DEXTRAL));

    // The listed literal omen too.
    let literal = engine_literal();
    let route = route_of(
        &literal,
        "test-literal-exalt",
        PREFIXES_FULL,
        &list,
        &[DEXTRAL],
    );
    let action = route.steps[0].action.as_ref().unwrap();
    assert_eq!(
        (action.alternative, action.currency.as_str()),
        (0, "perfect-exalted-orb")
    );
}

fn engine_literal() -> Engine {
    engine(&[LITERAL])
}

/// A listed step that writes Omen of Dextral Exaltation literally: left out
/// on a ring whose prefixes are full, in the preview and in the guide.
#[test]
fn listed_literal_exaltation_omen_dropped_when_the_other_side_is_full() {
    let literal = engine_literal();
    let list = prefixes_full_targets();
    let route = route_of(&literal, "test-literal-exalt", PREFIXES_FULL, &list, &[]);
    let step = &route.steps[0];
    assert_eq!(taken(step), ("perfect-exalted-orb".to_string(), Vec::new()));
    let action = step.action.as_ref().unwrap();
    assert!(action.omen_names.is_empty());
    assert_eq!(action.side, None);
    assert_eq!(step.side, Some(Side::Suffix));

    let (_, step) = start(
        &literal,
        "test-literal-exalt",
        &item(PREFIXES_FULL),
        &list,
        &[],
    );
    assert!(step.action.as_ref().unwrap().omens.is_empty());
}

/// The same listed step on a ring with a free prefix: the omen is named.
#[test]
fn listed_literal_exaltation_omen_kept_while_the_other_side_has_room() {
    let literal = engine_literal();
    let route = route_of(
        &literal,
        "test-literal-exalt",
        ROOM_ON_BOTH,
        &room_on_both_targets(),
        &[],
    );
    let action = route.steps[0].action.as_ref().unwrap();
    assert_eq!(action.omens, owned(&[DEXTRAL]));
    assert_eq!(action.omen_names, owned(&["Omen of Dextral Exaltation"]));
    assert_eq!(action.side, Some(Side::Suffix));

    // Blocked, it still passes the Perfect Exalted Orb over (and the
    // Greater one, which names it too).
    let plan = literal
        .plan(
            &item(ROOM_ON_BOTH),
            &room_on_both_targets(),
            &blocked(&[DEXTRAL]),
        )
        .unwrap();
    assert!(
        plan.rejected
            .iter()
            .any(|r| r.blueprint == "test-literal-exalt"),
        "{:?}",
        plan.routes
    );
}

/// The rule is the side Exaltation omens' alone: a desecration on the
/// suffix side of a ring whose prefixes are full keeps Omen of Dextral
/// Necromancy.
#[test]
fn necromancy_omen_kept_beside_a_full_side() {
    let route = route_of(
        &shipped(),
        "essence-desecrate-double-slam",
        PREFIXES_FULL,
        &prefixes_full_targets(),
        &[],
    );
    let first = &route.steps[0];
    assert_eq!(first.side, Some(Side::Suffix));
    assert_eq!(
        taken(first).1,
        owned(&[DEXTRAL_NECROMANCY]),
        "{:?}",
        first.action
    );
}

/// Leaving the omen out changes no odds: the exalt could only land on the
/// open side either way. rare-clean-then-slam's slam (its side omen left
/// out) has close-a-side's `slam-open-side` odds (no side omen written),
/// and the listed literal step its own odds with the omen blocked.
#[test]
fn dropped_omen_leaves_the_odds_of_the_open_side_slam() {
    let engine = shipped();
    let list = prefixes_full_targets();
    let clean = route_of(&engine, "rare-clean-then-slam", PREFIXES_FULL, &list, &[]);
    let close = route_of(&engine, "close-a-side", PREFIXES_FULL, &list, &[]);
    let odds = clean.steps[0].odds.clone().expect("odds");
    let percent = odds.percent().expect("a chance");
    assert!(percent > 0.0 && percent < 100.0, "{odds:?}");
    assert_eq!(Some(odds), close.steps[0].odds);

    let literal = engine_literal();
    let free = route_of(&literal, "test-literal-exalt", PREFIXES_FULL, &list, &[]);
    let open = route_of(
        &literal,
        "test-literal-exalt",
        PREFIXES_FULL,
        &list,
        &[DEXTRAL],
    );
    assert!(free.steps[0].odds.is_some());
    assert_eq!(free.steps[0].odds, open.steps[0].odds);
}
