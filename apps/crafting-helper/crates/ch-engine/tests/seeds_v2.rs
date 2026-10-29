//! The four state-ordered v2 blueprints in `apps/crafting-helper/blueprints/`,
//! each walked through the guide (`start`, then `observe` per change): a hit
//! path, where each step hits and the next is picked from the item, and a
//! miss path, where the step after a miss is again picked from the item as
//! it stands (no transition), and a fresh plan of the missed item agrees.
//! The starting items are the parity test's (`tests/items/slots/`), the
//! later states are under `tests/items/slots/walks/`; all are constructed
//! (see the folder's README).

mod common;

use std::path::PathBuf;

use ch_engine::ch_blueprint::Side;
use ch_engine::{Engine, Next, Outcome, OutcomeKind, Status, StepView, Target};
use ch_item::Item;
use common::{crate_dir, item, next_step, observe, start, targets};

use OutcomeKind::{Hit, Miss};

const IDS: [&str; 4] = [
    "close-a-side",
    "essence-desecrate-double-slam",
    "magic-augment-essence",
    "rare-clean-then-slam",
];

const MANA: &str = "prefix-to-maximum-mana";
const LIFE: &str = "prefix-to-maximum-life";
const ACC: &str = "prefix-to-accuracy-rating";
const EVASION: &str = "prefix-to-evasion-rating";
const FIRE: &str = "suffix-to-fire-resistance";
const CHAOS: &str = "suffix-to-chaos-resistance";

/// An engine over the four v2 blueprints, read from the shipped folder (not
/// `Engine::shipped`, so another blueprint cannot break these walks).
fn v2() -> Engine {
    let folder = crate_dir().join("..").join("..").join("blueprints");
    let paths: Vec<PathBuf> = IDS
        .iter()
        .map(|id| folder.join(format!("{id}.toml")))
        .collect();
    Engine::load(&paths).unwrap_or_else(|errors| {
        let lines: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        panic!("the v2 blueprints do not load:\n{}", lines.join("\n"))
    })
}

/// A starting item of the parity test, `tests/items/slots/<name>.txt`.
fn case(name: &str) -> Item {
    item(&format!("slots/{name}.txt"))
}

/// A later state, `tests/items/slots/walks/<name>.txt`.
fn state(name: &str) -> Item {
    item(&format!("slots/walks/{name}.txt"))
}

/// The 2P/1S ring's targets (`rare-2p1s-prefix-missing`).
fn ring_2p1s() -> Vec<Target> {
    targets(&[
        (MANA, Some(5)),
        (LIFE, Some(4)),
        (ACC, Some(3)),
        (FIRE, Some(5)),
    ])
}

/// Issue #28's ring's targets (`rare-one-target`).
fn ring_one_target() -> Vec<Target> {
    targets(&[
        (MANA, Some(5)),
        (LIFE, Some(4)),
        (EVASION, Some(3)),
        (CHAOS, Some(3)),
    ])
}

/// The magic amulets' targets (`magic-one-good`, `magic-good-and-bad`).
fn amulet() -> Vec<Target> {
    targets(&[(MANA, Some(6)), (LIFE, Some(8)), (FIRE, Some(5))])
}

/// Checks a step: its id, resolved side, currency as written and omens.
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

/// A fresh plan of `blueprint` on the item starts with `step`, as the guide
/// does: the step is picked from the item, not from how it got there.
fn replans_to(engine: &Engine, blueprint: &str, it: &Item, list: &[Target], step: &StepView) {
    let plan = engine.plan(it, list, &[]).unwrap();
    let route = plan
        .routes
        .iter()
        .find(|r| r.blueprint == blueprint)
        .unwrap_or_else(|| panic!("{blueprint} is not routed: {:?}", plan.rejected));
    assert_eq!(route.steps[0].id, step.id);
    assert_eq!(route.steps[0].side, step.side);
    assert_eq!(route.steps[0].action, step.action);
}

#[test]
fn close_a_side_hit_and_miss() {
    let engine = v2();
    let list = ring_2p1s();
    let ring = case("rare-2p1s-prefix-missing");
    let greater = "omen-of-greater-exaltation";

    // Hit path: close the suffix side with a Greater, Dextral exalt, then
    // the one open side takes a Perfect exalt with no side omen.
    let (mut guide, first) = start(&engine, "close-a-side", &ring, &list, &[]);
    is(
        &first,
        "close",
        Some(Side::Suffix),
        "exalted-orb",
        &[greater, "omen-of-dextral-exaltation"],
    );
    let closed = state("close-closed");
    let out = observe(&engine, &mut guide, &ring, &closed);
    let slam = then(&out, Hit).clone();
    is(
        &slam,
        "slam-open-side",
        Some(Side::Prefix),
        "perfect-exalted-orb",
        &[],
    );
    let complete = state("close-complete");
    let out = observe(&engine, &mut guide, &closed, &complete);
    done(&out, &guide);

    // Miss path: the slam lands an Evasion prefix. The prefix side is full
    // with Accuracy missing, so the next step is the removal, confined to
    // the prefix side (the suffix side holds Fire), not a whittle (the
    // lowest modifier level is the Life target's).
    let (mut guide, _) = start(&engine, "close-a-side", &closed, &list, &[]);
    let missed = state("close-slam-miss");
    let out = observe(&engine, &mut guide, &closed, &missed);
    let next = then(&out, Miss).clone();
    is(
        &next,
        "remove",
        Some(Side::Prefix),
        "orb-of-annulment",
        &["omen-of-sinistral-annulment"],
    );
    assert_eq!(engine.current(&guide, &missed).unwrap(), next);
    // A full item has no open side, so close-a-side does not start on it
    // afresh; the clean-up blueprint plans the same removal.
    replans_to(&engine, "rare-clean-then-slam", &missed, &list, &next);
    // The removal hits: back where the slam was.
    let out = observe(&engine, &mut guide, &missed, &closed);
    is(
        then(&out, Hit),
        "slam-open-side",
        Some(Side::Prefix),
        "perfect-exalted-orb",
        &[],
    );
}

#[test]
fn essence_desecrate_double_slam_hit_and_miss() {
    let engine = v2();
    let list = amulet();
    let magic = case("magic-one-good");

    // Hit path: the essence turns the amulet rare with Life, the bone goes
    // on the suffix side for Fire, the reveal is Fire.
    let (mut guide, first) = start(&engine, "essence-desecrate-double-slam", &magic, &list, &[]);
    is(&first, "essence", None, "lesser-essence-of-the-body", &[]);
    let rare = state("essence-rare");
    let out = observe(&engine, &mut guide, &magic, &rare);
    let desecrate = then(&out, Hit).clone();
    is(
        &desecrate,
        "desecrate",
        Some(Side::Suffix),
        "desecrate",
        &["omen-of-dextral-necromancy"],
    );
    assert_eq!(desecrate.target.as_deref(), Some(FIRE));
    let unrevealed = state("essence-desecrated");
    let out = observe(&engine, &mut guide, &rare, &unrevealed);
    let reveal = then(&out, Hit).clone();
    is(
        &reveal,
        "reveal",
        None,
        "reveal",
        &["omen-of-abyssal-echoes"],
    );
    let revealed = state("essence-revealed-fire");
    let out = observe(&engine, &mut guide, &unrevealed, &revealed);
    done(&out, &guide);

    // Miss path: the reveal is Cold and Chaos Resistance. Omen of Light
    // takes it off, and the bone is used again.
    let (mut guide, _) = start(&engine, "essence-desecrate-double-slam", &magic, &list, &[]);
    observe(&engine, &mut guide, &magic, &rare);
    observe(&engine, &mut guide, &rare, &unrevealed);
    let wrong = state("essence-revealed-wrong");
    let out = observe(&engine, &mut guide, &unrevealed, &wrong);
    let light = then(&out, Miss).clone();
    is(
        &light,
        "light",
        None,
        "orb-of-annulment",
        &["omen-of-light"],
    );
    replans_to(
        &engine,
        "essence-desecrate-double-slam",
        &wrong,
        &list,
        &light,
    );
    let out = observe(&engine, &mut guide, &wrong, &rare);
    is(
        then(&out, Hit),
        "desecrate",
        Some(Side::Suffix),
        "desecrate",
        &["omen-of-dextral-necromancy"],
    );
}

#[test]
fn magic_augment_essence_hit_and_miss() {
    let engine = v2();
    let list = amulet();
    let bad = case("magic-good-and-bad");
    let good = case("magic-one-good");

    // Hit path: the Intelligence suffix is annulled from its side (the
    // prefix side holds Mana), a Perfect augment lands Fire on the free
    // suffix, the essence adds Life as the amulet turns rare.
    let (mut guide, first) = start(&engine, "magic-augment-essence", &bad, &list, &[]);
    is(
        &first,
        "remove-magic",
        Some(Side::Suffix),
        "orb-of-annulment",
        &["omen-of-dextral-annulment"],
    );
    let out = observe(&engine, &mut guide, &bad, &good);
    let augment = then(&out, Hit).clone();
    is(
        &augment,
        "augment",
        None,
        "perfect-orb-of-augmentation",
        &[],
    );
    let fire = state("augment-fire");
    let out = observe(&engine, &mut guide, &good, &fire);
    is(
        then(&out, Hit),
        "essence",
        None,
        "lesser-essence-of-the-body",
        &[],
    );
    let rare = state("augment-essence");
    let out = observe(&engine, &mut guide, &fire, &rare);
    done(&out, &guide);

    // Miss path: the augment lands Strength; the suffix side is blocked
    // again, so the next step is the Dextral annulment once more.
    let (mut guide, _) = start(&engine, "magic-augment-essence", &good, &list, &[]);
    let missed = state("augment-miss");
    let out = observe(&engine, &mut guide, &good, &missed);
    let next = then(&out, Miss).clone();
    is(
        &next,
        "remove-magic",
        Some(Side::Suffix),
        "orb-of-annulment",
        &["omen-of-dextral-annulment"],
    );
    replans_to(&engine, "magic-augment-essence", &missed, &list, &next);
    let out = observe(&engine, &mut guide, &missed, &good);
    is(
        then(&out, Hit),
        "augment",
        None,
        "perfect-orb-of-augmentation",
        &[],
    );
}

#[test]
fn rare_clean_then_slam_hit_and_miss() {
    let engine = v2();
    let list = ring_one_target();
    let ring = case("rare-one-target");
    let greater = "omen-of-greater-exaltation";

    // Hit path: the Accuracy prefix is annulled (a plain annulment, as the
    // suffix side holds no target), a Greater, Sinistral Perfect exalt adds
    // Life and Evasion, a Dextral one adds Chaos Resistance.
    let (mut guide, first) = start(&engine, "rare-clean-then-slam", &ring, &list, &[]);
    is(
        &first,
        "remove",
        Some(Side::Prefix),
        "orb-of-annulment",
        &[],
    );
    let cleaned = state("clean-cleaned");
    let out = observe(&engine, &mut guide, &ring, &cleaned);
    is(
        then(&out, Hit),
        "side-slam",
        Some(Side::Prefix),
        "perfect-exalted-orb",
        &[greater, "omen-of-sinistral-exaltation"],
    );
    let prefixes = state("clean-two-prefixes");
    let out = observe(&engine, &mut guide, &cleaned, &prefixes);
    is(
        then(&out, Hit),
        "side-slam",
        Some(Side::Suffix),
        "perfect-exalted-orb",
        &["omen-of-dextral-exaltation"],
    );
    let complete = state("clean-complete");
    let out = observe(&engine, &mut guide, &prefixes, &complete);
    done(&out, &guide);

    // Miss path: the annulment takes the Mana target instead. The prefix
    // side, now one junk for three missing targets, is still blocked: the
    // next step is the same removal, and a fresh plan agrees.
    let (mut guide, _) = start(&engine, "rare-clean-then-slam", &ring, &list, &[]);
    let lost = state("clean-target-annulled");
    let out = observe(&engine, &mut guide, &ring, &lost);
    assert_eq!(out.lost.len(), 1, "{out:?}");
    assert_eq!(out.lost[0].mod_id, MANA);
    let next = then(&out, Miss).clone();
    is(&next, "remove", Some(Side::Prefix), "orb-of-annulment", &[]);
    replans_to(&engine, "rare-clean-then-slam", &lost, &list, &next);
}
