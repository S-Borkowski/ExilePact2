//! The four blueprints from the maxroll and Mobalytics guides (issue #35)
//! in `apps/crafting-helper/blueprints/`, each walked through the guide
//! (`start`, then `observe` per change): a hit path, a miss path, and a
//! start the blueprint refuses. `fracture-first` and
//! `full-rare-desecration-rescue` are listed; `weapons-physical-generic`
//! and `body-armours-single-prefix` are state-ordered, use only the slot
//! spec's rules, and are not among the slot model's blueprints, so these
//! walks are what pins them (docs/models/crafting-slots-spec.md § "Not
//! covered"). Every item is constructed, under `tests/items/guides/<id>/`
//! (see each folder's README).

mod common;

use std::path::PathBuf;

use ch_engine::ch_blueprint::Side;
use ch_engine::{
    Engine, Next, Odds, Outcome, OutcomeKind, Rejection, Stall, Status, StepView, Target,
};
use ch_item::Item;
use common::{crate_dir, item, next_step, observe, start, targets};

use OutcomeKind::{Hit, Miss};

const IDS: [&str; 4] = [
    "body-armours-single-prefix",
    "fracture-first",
    "full-rare-desecration-rescue",
    "weapons-physical-generic",
];

const SPEED: &str = "prefix-increased-movement-speed";
const BOOTS_LIFE: &str = "prefix-to-maximum-life";
const BOOTS_EVASION: &str = "prefix-to-evasion-rating";
const FIRE: &str = "suffix-to-fire-resistance";
const COLD: &str = "suffix-to-cold-resistance";
const CHAOS: &str = "suffix-to-chaos-resistance";

const GREATER: &str = "omen-of-greater-exaltation";
const ECHOES: &str = "omen-of-abyssal-echoes";

/// An engine over the four guide blueprints, read from the shipped folder
/// (not `Engine::shipped`, so another blueprint cannot break these walks).
fn guides() -> Engine {
    let folder = crate_dir().join("..").join("..").join("blueprints");
    let paths: Vec<PathBuf> = IDS
        .iter()
        .map(|id| folder.join(format!("{id}.toml")))
        .collect();
    Engine::load(&paths).unwrap_or_else(|errors| {
        let lines: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        panic!("the guide blueprints do not load:\n{}", lines.join("\n"))
    })
}

/// `tests/items/guides/<blueprint>/<name>.txt`.
fn state(blueprint: &str, name: &str) -> Item {
    item(&format!("guides/{blueprint}/{name}.txt"))
}

/// Checks a step: its id, side, currency as written and omens.
fn is(step: &StepView, id: &str, side: Option<Side>, currency: &str, omens: &[&str]) {
    let action = step.action.as_ref().expect("a step with an action");
    assert_eq!(
        (
            step.id.as_str(),
            step.side,
            action.currency.as_str(),
            action.omens.clone()
        ),
        (
            id,
            side,
            currency,
            omens.iter().map(|o| o.to_string()).collect::<Vec<_>>()
        ),
        "{step:?}"
    );
}

/// Checks an outcome's verdict and returns the step it names next.
fn then(out: &Outcome, result: OutcomeKind) -> &StepView {
    assert_eq!(out.result, result, "{out:?}");
    assert!(out.reasons.is_empty(), "{:?}", out.reasons);
    next_step(&out.next)
}

/// Checks an outcome's verdict and that the craft is done.
fn done(out: &Outcome, guide: &ch_engine::Guide) {
    assert_eq!(out.result, Hit, "{out:?}");
    assert_eq!(out.next, Next::Done);
    assert_eq!(guide.status, Status::Done);
}

/// `(hits, total, basis)` of known odds; panics on unknown ones.
fn known(odds: &Option<Odds>) -> (u64, u64, String) {
    match odds {
        Some(Odds::Known {
            hits, total, basis, ..
        }) => (*hits, *total, basis.clone()),
        other => panic!("expected known odds, got {other:?}"),
    }
}

/// Why `blueprint` is rejected on the item.
fn rejection(engine: &Engine, it: &Item, list: &[Target], blueprint: &str) -> Rejection {
    let plan = engine.plan(it, list, &[]).unwrap();
    plan.rejected
        .iter()
        .find(|r| r.blueprint == blueprint)
        .unwrap_or_else(|| panic!("{blueprint} is not rejected: {:?}", plan.routes))
        .reason
        .clone()
}

/// The `[[start]]` failures of a blueprint no start fits.
fn no_start(engine: &Engine, it: &Item, list: &[Target], blueprint: &str) -> Vec<Vec<String>> {
    match rejection(engine, it, list, blueprint) {
        Rejection::NoStartFits { failed } => failed,
        other => panic!("{blueprint}: expected no start to fit, got {other}"),
    }
}

// --- fracture-first --------------------------------------------------------

const FRACTURE: &str = "fracture-first";

/// Movement Speed T2 is on the boots; Evasion Rating T3, Fire Resistance T3
/// and Chaos Resistance T3 (the hardest, spawn weight 250) are missing.
fn fracture_targets() -> Vec<Target> {
    targets(&[
        (SPEED, Some(2)),
        (BOOTS_EVASION, Some(3)),
        (FIRE, Some(3)),
        (CHAOS, Some(3)),
    ])
}

fn ff(name: &str) -> Item {
    state(FRACTURE, name)
}

#[test]
fn fracture_first_hit_path() {
    let engine = guides();
    let list = fracture_targets();
    let four = ff("rare-four");

    // The essence first, with no side omen.
    let (mut guide, first) = start(&engine, FRACTURE, &four, &list, &[]);
    is(&first, "abyss", None, "essence-of-the-abyss", &[]);

    // The mark took Life's place. The Fracturing Orb's odds leave it out:
    // the one target among the three it can pick is one in three, not one
    // in four.
    let abyss = ff("abyss");
    let out = observe(&engine, &mut guide, &four, &abyss);
    let fracture = then(&out, Hit).clone();
    is(&fracture, "fracture", None, "fracturing-orb", &[]);
    let (hits, total, basis) = known(&fracture.odds);
    assert_eq!((hits, total), (1, 3), "{basis}");
    assert!(basis.contains("abyss-mark-unfracturable"), "{basis}");

    // Movement Speed fractured; annulled down to it and one more modifier,
    // the mark included.
    let fractured = ff("fractured");
    let out = observe(&engine, &mut guide, &abyss, &fractured);
    is(then(&out, Hit), "annul", None, "orb-of-annulment", &[]);
    let once = ff("annulled-once");
    let out = observe(&engine, &mut guide, &fractured, &once);
    assert_eq!(out.transition.as_deref(), Some("repeat"));
    is(then(&out, Hit), "annul", None, "orb-of-annulment", &[]);
    let twice = ff("annulled-twice");
    let out = observe(&engine, &mut guide, &once, &twice);
    is(then(&out, Hit), "chaos", None, "chaos-orb", &[]);

    // A Chaos Orb lands Fire Resistance, the second target; the hardest
    // missing one, Chaos Resistance, is desecrated on the suffixes.
    let chaos = ff("chaos");
    let out = observe(&engine, &mut guide, &twice, &chaos);
    is(
        then(&out, Hit),
        "desecrate",
        None,
        "desecrate",
        &["omen-of-dextral-necromancy"],
    );
    let desecrated = ff("desecrated");
    let out = observe(&engine, &mut guide, &chaos, &desecrated);
    is(then(&out, Hit), "reveal", None, "reveal", &[ECHOES]);

    // The reveal offers it. The first slam's target, Fire Resistance, came
    // from the Chaos Orb, so it is skipped; a Sinistral Perfect exalt adds
    // Evasion Rating.
    let revealed = ff("revealed");
    let out = observe(&engine, &mut guide, &desecrated, &revealed);
    assert_eq!(out.transition.as_deref(), Some("goto:slam-1"));
    assert_eq!(out.skipped.len(), 1, "{:?}", out.skipped);
    assert_eq!(out.skipped[0].step, "slam-1");
    let slam = then(&out, Hit).clone();
    is(
        &slam,
        "slam-2",
        None,
        "perfect-exalted-orb",
        &["omen-of-sinistral-exaltation"],
    );
    assert_eq!(slam.target.as_deref(), Some(BOOTS_EVASION));
    let complete = ff("complete");
    let out = observe(&engine, &mut guide, &revealed, &complete);
    done(&out, &guide);
}

#[test]
fn fracture_first_miss_path() {
    let engine = guides();
    let list = fracture_targets();
    let four = ff("rare-four");

    // With no side omen the essence took Movement Speed, the one target:
    // a miss, and the item is given up.
    let (mut guide, _) = start(&engine, FRACTURE, &four, &list, &[]);
    let lost = ff("abyss-lost-target");
    let out = observe(&engine, &mut guide, &four, &lost);
    assert_eq!(out.result, Miss, "{out:?}");
    assert_eq!(out.lost.len(), 1, "{out:?}");
    assert_eq!(out.lost[0].mod_id, SPEED);
    assert_eq!(out.transition.as_deref(), Some("abandon"));
    assert_eq!(out.next, Next::Abandon);
}

#[test]
fn fracture_first_start_refuses() {
    let engine = guides();
    // Five modifiers: the orb's one-in-four (one-in-three) reading needs
    // exactly four.
    assert_eq!(
        no_start(&engine, &ff("rare-five"), &fracture_targets(), FRACTURE),
        vec![vec!["5 modifiers, at most 4 allowed".to_string()]]
    );
    // Control: the same boots with four fit.
    let plan = engine
        .plan(&ff("rare-four"), &fracture_targets(), &[])
        .unwrap();
    assert!(plan.routes.iter().any(|r| r.blueprint == FRACTURE));
}

// --- full-rare-desecration-rescue -------------------------------------------

const RESCUE: &str = "full-rare-desecration-rescue";

/// Everything on the boots but Movement Speed; Mana and Dexterity are not
/// targets.
fn rescue_targets() -> Vec<Target> {
    targets(&[
        (SPEED, Some(2)),
        (BOOTS_LIFE, Some(3)),
        (BOOTS_EVASION, Some(3)),
        (FIRE, Some(3)),
        (COLD, Some(3)),
    ])
}

fn rs(name: &str) -> Item {
    state(RESCUE, name)
}

#[test]
fn full_rare_desecration_rescue_hit_path() {
    let engine = guides();
    let list = rescue_targets();
    let six = rs("rare-six");

    // The bone goes on the prefixes, full, with Sinistral Necromancy. It
    // takes one of Life, Evasion and Mana off first: two of three keep
    // every target.
    let (mut guide, first) = start(&engine, RESCUE, &six, &list, &[]);
    is(
        &first,
        "desecrate",
        None,
        "desecrate",
        &["omen-of-sinistral-necromancy"],
    );
    let (hits, total, basis) = known(&first.odds);
    assert_eq!((hits, total), (1, 3), "{basis}");
    assert!(basis.contains("full-desecration-same-side"), "{basis}");

    // Mana went; the reveal offers Movement Speed.
    let desecrated = rs("desecrated");
    let out = observe(&engine, &mut guide, &six, &desecrated);
    is(then(&out, Hit), "reveal", None, "reveal", &[ECHOES]);
    let revealed = rs("revealed");
    let out = observe(&engine, &mut guide, &desecrated, &revealed);
    assert_eq!(out.transition.as_deref(), Some("done"));
    done(&out, &guide);
}

#[test]
fn full_rare_desecration_rescue_miss_path() {
    let engine = guides();
    let list = rescue_targets();
    let six = rs("rare-six");
    let desecrated = rs("desecrated");

    // A reveal that offered no target: Omen of Light takes it off, and the
    // bone is used again, now on a free prefix slot.
    let (mut guide, _) = start(&engine, RESCUE, &six, &list, &[]);
    observe(&engine, &mut guide, &six, &desecrated);
    let wrong = rs("revealed-wrong");
    let out = observe(&engine, &mut guide, &desecrated, &wrong);
    is(
        then(&out, Miss),
        "light",
        None,
        "orb-of-annulment",
        &["omen-of-light"],
    );
    let five = rs("rare-five");
    let out = observe(&engine, &mut guide, &wrong, &five);
    assert_eq!(out.transition.as_deref(), Some("goto:desecrate"));
    is(
        then(&out, Hit),
        "desecrate",
        None,
        "desecrate",
        &["omen-of-sinistral-necromancy"],
    );

    // The bone took Life, a target, off: the item is given up.
    let (mut guide, _) = start(&engine, RESCUE, &six, &list, &[]);
    let lost = rs("desecrated-lost-target");
    let out = observe(&engine, &mut guide, &six, &lost);
    assert_eq!(out.result, Miss, "{out:?}");
    assert_eq!(out.lost[0].mod_id, BOOTS_LIFE);
    assert_eq!(out.next, Next::Abandon);
}

#[test]
fn full_rare_desecration_rescue_start_refuses() {
    let engine = guides();
    // Five modifiers: not a full rare.
    assert_eq!(
        no_start(&engine, &rs("rare-five"), &rescue_targets(), RESCUE),
        vec![vec!["5 modifiers, at least 6 needed".to_string()]]
    );
}

// --- weapons-physical-generic -----------------------------------------------

const WEAPON: &str = "weapons-physical-generic";
const PHYS: &str = "prefix-increased-physical-damage";
const FLAT: &str = "prefix-adds-to-physical-damage";

/// A two hand mace at item level 82: increased Physical Damage T2 (its top
/// two are T1 and T2), flat Physical Damage T3 (Greater Essence of
/// Abrasion's), Accuracy T3; Attack Speed, Critical Damage Bonus and
/// Strength, T3 each.
fn weapon_targets() -> Vec<Target> {
    targets(&[
        (PHYS, Some(2)),
        (FLAT, Some(3)),
        ("prefix-to-accuracy-rating", Some(3)),
        ("suffix-increased-attack-speed", Some(3)),
        ("suffix-to-critical-damage-bonus", Some(3)),
        ("suffix-to-strength", Some(3)),
    ])
}

fn wp(name: &str) -> Item {
    state(WEAPON, name)
}

#[test]
fn weapons_physical_generic_hit_path() {
    let engine = guides();
    let list = weapon_targets();
    let magic = wp("magic");

    // The magic weapon holds a top-two Physical Damage prefix: augment.
    let (mut guide, first) = start(&engine, WEAPON, &magic, &list, &[]);
    is(&first, "augment", None, "perfect-orb-of-augmentation", &[]);

    // Attack Speed; then the one essence, Greater Essence of Abrasion.
    let augmented = wp("augmented");
    let out = observe(&engine, &mut guide, &magic, &augmented);
    let essence = then(&out, Hit).clone();
    is(
        &essence,
        "essence",
        None,
        "greater-essence-of-abrasion",
        &[],
    );
    assert_eq!(essence.target.as_deref(), Some(FLAT));

    // Rare with two suffixes missing: the guide's Greater Exalted Orb with
    // Greater and Dextral Exaltation.
    let rare = wp("rare");
    let out = observe(&engine, &mut guide, &augmented, &rare);
    is(
        then(&out, Hit),
        "side-slam",
        Some(Side::Suffix),
        "greater-exalted-orb",
        &[GREATER, "omen-of-dextral-exaltation"],
    );

    // Accuracy is the last target: desecrated on the prefixes, revealed.
    let slammed = wp("slammed");
    let out = observe(&engine, &mut guide, &rare, &slammed);
    is(
        then(&out, Hit),
        "desecrate-last",
        Some(Side::Prefix),
        "desecrate",
        &["omen-of-sinistral-necromancy"],
    );
    let desecrated = wp("desecrated");
    let out = observe(&engine, &mut guide, &slammed, &desecrated);
    is(then(&out, Hit), "reveal-last", None, "reveal", &[ECHOES]);
    let complete = wp("complete");
    let out = observe(&engine, &mut guide, &desecrated, &complete);
    done(&out, &guide);

    // The preview from the magic weapon is that path, with no second
    // essence and no Regal Orb.
    let plan = engine.plan(&magic, &list, &[]).unwrap();
    let route = plan.routes.iter().find(|r| r.blueprint == WEAPON).unwrap();
    let ids: Vec<&str> = route.steps.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "augment",
            "essence",
            "side-slam",
            "desecrate-last",
            "reveal-last"
        ]
    );
}

#[test]
fn weapons_physical_generic_miss_path() {
    let engine = guides();
    let list = weapon_targets();
    let magic = wp("magic");

    // The augment lands Stun Duration: the suffix is blocked, and the next
    // step, picked from the item, is a Dextral annulment (the prefix holds
    // a target).
    let (mut guide, _) = start(&engine, WEAPON, &magic, &list, &[]);
    let missed = wp("augment-miss");
    let out = observe(&engine, &mut guide, &magic, &missed);
    let next = then(&out, Miss).clone();
    is(
        &next,
        "remove-magic",
        Some(Side::Suffix),
        "orb-of-annulment",
        &["omen-of-dextral-annulment"],
    );
    assert_eq!(engine.current(&guide, &missed).unwrap(), next);
    // The annulment hits: back to the augment.
    let out = observe(&engine, &mut guide, &missed, &magic);
    is(
        then(&out, Hit),
        "augment",
        None,
        "perfect-orb-of-augmentation",
        &[],
    );
}

#[test]
fn weapons_physical_generic_start_refuses() {
    let engine = guides();

    // Physical Damage T3, below its top two at item level 82: a new base.
    match rejection(&engine, &wp("magic-weak"), &weapon_targets(), WEAPON) {
        Rejection::WeakMagicStart {
            modifier,
            tier,
            item_level,
            top,
        } => assert_eq!(
            (modifier.as_str(), tier, item_level, top),
            (PHYS, 3, 82, vec![1, 2])
        ),
        other => panic!("expected weak-magic-start, got {other}"),
    }

    // No flat Physical Damage among the targets: Abrasion adds nothing, so
    // no step makes the weapon rare and the route cannot finish.
    let no_flat: Vec<Target> = weapon_targets()
        .into_iter()
        .filter(|t| t.mod_id != FLAT)
        .collect();
    match rejection(&engine, &wp("magic"), &no_flat, WEAPON) {
        Rejection::RouteCannotFinish { why, .. } => assert_eq!(why, Stall::NoCandidate),
        other => panic!("expected route-cannot-finish, got {other}"),
    }
}

// --- body-armours-single-prefix ---------------------------------------------

const ARMOUR: &str = "body-armours-single-prefix";

/// A Slipstrike Vest (Dexterity) at item level 82: increased Evasion Rating
/// T1, Life T4 and flat Evasion Rating T4; Fire and Cold Resistance T3, and
/// the desecrated reduced Curse effect suffix.
fn armour_targets() -> Vec<Target> {
    targets(&[
        ("prefix-increased-evasion-rating", Some(1)),
        ("prefix-to-maximum-life", Some(4)),
        ("prefix-to-evasion-rating", Some(4)),
        (FIRE, Some(3)),
        (COLD, Some(3)),
        ("desecrated-suffix-reduced-effect-of-curses-on-you", Some(1)),
    ])
}

fn ba(name: &str) -> Item {
    state(ARMOUR, name)
}

#[test]
fn body_armours_single_prefix_hit_path() {
    let engine = guides();
    let list = armour_targets();
    let magic = ba("magic");

    // Fire Resistance is on: Greater Essence of Thawing adds Cold.
    let (mut guide, first) = start(&engine, ARMOUR, &magic, &list, &[]);
    is(&first, "essence", None, "greater-essence-of-thawing", &[]);
    let rare = ba("rare-essence");
    let out = observe(&engine, &mut guide, &magic, &rare);
    // The hardest target is the desecrated suffix: the bone fills the
    // suffixes, with Dextral Necromancy.
    is(
        then(&out, Hit),
        "desecrate",
        Some(Side::Suffix),
        "desecrate",
        &["omen-of-dextral-necromancy"],
    );

    // The isolation: with only the prefixes open, a Greater Exalted Orb
    // with Greater Exaltation and no side omen adds both prefixes.
    let desecrated = ba("rare-desecrated");
    let out = observe(&engine, &mut guide, &rare, &desecrated);
    let isolate = then(&out, Hit).clone();
    is(&isolate, "isolate", None, "greater-exalted-orb", &[GREATER]);
    let isolated = ba("rare-isolated");
    let out = observe(&engine, &mut guide, &desecrated, &isolated);
    is(then(&out, Hit), "reveal", None, "reveal", &[ECHOES]);
    let complete = ba("rare-complete");
    let out = observe(&engine, &mut guide, &isolated, &complete);
    done(&out, &guide);

    // The preview from the magic armour is that path.
    let plan = engine.plan(&magic, &list, &[]).unwrap();
    let route = plan.routes.iter().find(|r| r.blueprint == ARMOUR).unwrap();
    let ids: Vec<&str> = route.steps.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(ids, ["essence", "desecrate", "isolate", "reveal"]);
}

#[test]
fn body_armours_single_prefix_miss_path() {
    let engine = guides();
    let list = armour_targets();
    let magic = ba("magic");
    let rare = ba("rare-essence");
    let desecrated = ba("rare-desecrated");

    // The isolation lands Spirit and Thorns: no target, and the prefixes
    // are full. The next step, picked from the item, is the reveal.
    let (mut guide, _) = start(&engine, ARMOUR, &magic, &list, &[]);
    observe(&engine, &mut guide, &magic, &rare);
    observe(&engine, &mut guide, &rare, &desecrated);
    let missed = ba("rare-isolate-miss");
    let out = observe(&engine, &mut guide, &desecrated, &missed);
    let next = then(&out, Miss).clone();
    is(&next, "reveal", None, "reveal", &[ECHOES]);
    assert_eq!(engine.current(&guide, &missed).unwrap(), next);
}

#[test]
fn body_armours_single_prefix_start_refuses() {
    let engine = guides();
    // Increased Evasion Rating T3, below its top two at item level 82.
    match rejection(&engine, &ba("magic-weak"), &armour_targets(), ARMOUR) {
        Rejection::WeakMagicStart { modifier, tier, .. } => {
            assert_eq!(
                (modifier.as_str(), tier),
                ("prefix-increased-evasion-rating", 3)
            )
        }
        other => panic!("expected weak-magic-start, got {other}"),
    }
}
