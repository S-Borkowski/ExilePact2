//! The fracture goal, the modifier count conditions and a desecration on a
//! full side (issue #35): a Fracturing Orb's odds leave out what it does not
//! pick, its outcome is a hit only when a target became fractured, a count
//! condition counts every explicit modifier, and a desecration whose side
//! is full shows its removal's odds. Over the test blueprints and the
//! constructed boots in `tests/items/fracture/` (see its README).

mod common;

use ch_engine::{Next, Odds, OutcomeKind, Rejection, Target};
use ch_item::Item;
use common::{engine, item, observe, start, targets};

const SPEED: &str = "prefix-increased-movement-speed";
const FIRE: &str = "suffix-to-fire-resistance";
const EVASION: &str = "prefix-to-evasion-rating";

const FRACTURE: &str = "test-fracture";
const DESECRATE: &str = "test-full-desecration";

/// `tests/items/fracture/<name>.txt`.
fn boots(name: &str) -> Item {
    item(&format!("fracture/{name}.txt"))
}

/// Movement Speed T2 (prefix) and Fire Resistance T3 (suffix).
fn boot_targets() -> Vec<Target> {
    targets(&[(SPEED, Some(2)), (FIRE, Some(3))])
}

/// `(hits, total, percent, basis)` of known odds; panics on unknown ones.
fn known(odds: &Option<Odds>) -> (u64, u64, f64, String) {
    match odds {
        Some(Odds::Known {
            hits,
            total,
            percent,
            basis,
        }) => (*hits, *total, *percent, basis.clone()),
        other => panic!("expected known odds, got {other:?}"),
    }
}

/// The fracture step's odds on the item, as the started guide shows them.
fn fracture_odds(name: &str) -> (u64, u64, f64, String) {
    let engine = engine(&["fracture/test-fracture.toml"]);
    let (_, step) = start(&engine, FRACTURE, &boots(name), &boot_targets(), &[]);
    assert_eq!(step.id, "fracture");
    known(&step.odds)
}

#[test]
fn fracture_odds_leave_out_the_abyss_mark_and_desecrated() {
    // Nothing to leave out: two targets of four modifiers.
    let (hits, total, percent, basis) = fracture_odds("boots-four");
    assert_eq!((hits, total, percent), (2, 4, 50.0));
    assert!(basis.contains("abyss-mark-unfracturable"), "{basis}");
    assert!(basis.contains("desecrated-unfracturable"), "{basis}");

    // The Mark of the Abyssal Lord in the Life prefix's place: the orb
    // picks among the other three, one target more likely.
    let (hits, total, _, _) = fracture_odds("boots-four-mark");
    assert_eq!((hits, total), (2, 3));

    // The mark and an unrevealed desecrated suffix: what is left is the two
    // targets, so the fracture is certain. Counted as four, it would be half.
    let (hits, total, percent, basis) = fracture_odds("boots-four-mark-desecrated");
    assert_eq!((hits, total, percent), (1, 1, 100.0));
    assert!(basis.contains("every modifier the orb can fracture is a target"), "{basis}");
}

#[test]
fn a_fractured_target_is_a_fracture_hit() {
    let engine = engine(&["fracture/test-fracture.toml"]);
    let before = boots("boots-four");
    let (mut guide, _) = start(&engine, FRACTURE, &before, &boot_targets(), &[]);
    let out = observe(&engine, &mut guide, &before, &boots("boots-fractured-target"));
    assert_eq!(out.result, OutcomeKind::Hit, "{:?}", out.reasons);
    assert!(out.lost.is_empty());
    assert_eq!(out.transition.as_deref(), Some("next"));
    assert_eq!(out.next, Next::Done);
}

#[test]
fn a_fractured_non_target_is_a_fracture_miss() {
    let engine = engine(&["fracture/test-fracture.toml"]);
    let before = boots("boots-four");
    let (mut guide, _) = start(&engine, FRACTURE, &before, &boot_targets(), &[]);
    let out = observe(&engine, &mut guide, &before, &boots("boots-fractured-other"));
    // The Life prefix fractured: a fracture as expected, of the wrong one.
    assert_eq!(out.result, OutcomeKind::Miss, "{:?}", out.reasons);
    assert!(out.reasons.is_empty(), "{:?}", out.reasons);
    assert_eq!(out.transition.as_deref(), Some("abandon"));
    assert_eq!(out.next, Next::Abandon);

    // A fractured item has nothing left to fracture: the step is skipped.
    let plan = engine
        .plan(&boots("boots-fractured-target"), &boot_targets(), &[])
        .unwrap();
    let route = plan.routes.iter().find(|r| r.blueprint == FRACTURE).unwrap();
    assert!(route.steps.is_empty());
    assert_eq!(
        route.skipped[0].why,
        "a fractured modifier is already on the item"
    );
}

#[test]
fn modifier_count_conditions_count_every_explicit_modifier() {
    let engine = engine(&["fracture/test-fracture.toml"]);
    let verdict = |name: &str| -> Result<(), Vec<Vec<String>>> {
        let plan = engine.plan(&boots(name), &boot_targets(), &[]).unwrap();
        match plan.rejected.iter().find(|r| r.blueprint == FRACTURE) {
            None => Ok(()),
            Some(r) => match &r.reason {
                Rejection::NoStartFits { failed } => Err(failed.clone()),
                other => panic!("{name}: {other}"),
            },
        }
    };
    assert_eq!(verdict("boots-four"), Ok(()));
    // The mark, an unrevealed desecrated modifier and a fractured one each
    // count as one of the four.
    assert_eq!(verdict("boots-four-mark-desecrated"), Ok(()));
    assert_eq!(verdict("boots-fractured-target"), Ok(()));
    assert_eq!(
        verdict("boots-three"),
        Err(vec![vec!["3 modifiers, at least 4 needed".to_string()]])
    );
    assert_eq!(
        verdict("boots-five"),
        Err(vec![vec!["5 modifiers, at most 4 allowed".to_string()]])
    );
}

#[test]
fn a_desecration_on_a_full_side_shows_the_removal_odds() {
    let engine = engine(&["fracture/test-full-desecration.toml"]);
    // Evasion is the missing prefix target: the desecration goes on the
    // prefixes, with Sinistral Necromancy.
    let mut list = boot_targets();
    list.push(Target::new(EVASION, Some(3)));

    // Control: with a prefix slot free nothing is removed, and the
    // desecration's own odds stay unknown.
    let (_, step) = start(&engine, DESECRATE, &boots("boots-five"), &list, &[]);
    let action = step.action.as_ref().unwrap();
    assert_eq!(action.omens, ["omen-of-sinistral-necromancy"]);
    match &step.odds {
        Some(Odds::Unknown { reason }) => {
            assert_eq!(reason, "desecration and reveals have no established weights")
        }
        other => panic!("expected unknown odds, got {other:?}"),
    }

    // Three prefixes: the game removes one of them first. Life and Mana are
    // not targets, Movement Speed is.
    let (_, step) = start(&engine, DESECRATE, &boots("boots-six"), &list, &[]);
    let (hits, total, _, basis) = known(&step.odds);
    assert_eq!((hits, total), (2, 3));
    assert!(basis.contains("the reveal's chance not included"), "{basis}");
    assert!(basis.contains("full-desecration-same-side"), "{basis}");
}
