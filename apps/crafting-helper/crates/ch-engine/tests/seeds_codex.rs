//! The blueprint from the domistae PoE2 Crafting Codex (issue #40),
//! `jewellery-catalyst-exalt` in `apps/crafting-helper/blueprints/`, walked
//! through the guide (`start`, then `observe` per change): a hit path, a
//! miss path, and the starts it refuses; and its slam's odds, which do not
//! count the catalyst's favour and say so. Every item is constructed, under
//! `tests/items/codex/` (see its README).

mod common;

use ch_engine::ch_blueprint::Goal;
use ch_engine::CATALYST_NOTE;
use ch_engine::{Engine, Next, Odds, Outcome, OutcomeKind, Rejection, Status, StepView, Target};
use ch_item::Item;
use common::{blocked, crate_dir, item, next_step, observe, shipped, start, targets};

use OutcomeKind::{Hit, Miss};

const ID: &str = "jewellery-catalyst-exalt";

const MANA: &str = "prefix-to-maximum-mana";
const LIFE: &str = "prefix-to-maximum-life";
const FIRE: &str = "suffix-to-fire-resistance";
const COLD: &str = "suffix-to-cold-resistance";

const CATALYSING: &str = "omen-of-catalysing-exaltation";
const DEXTRAL_EXALTATION: &str = "omen-of-dextral-exaltation";
const DEXTRAL_ANNULMENT: &str = "omen-of-dextral-annulment";

/// An engine over the codex blueprint alone, read from the shipped folder.
fn codex() -> Engine {
    let path = crate_dir()
        .join("..")
        .join("..")
        .join("blueprints")
        .join(format!("{ID}.toml"));
    Engine::load(&[path]).unwrap_or_else(|errors| {
        let lines: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        panic!("the codex blueprint does not load:\n{}", lines.join("\n"))
    })
}

/// `tests/items/codex/<name>.txt`.
fn state(name: &str) -> Item {
    item(&format!("codex/{name}.txt"))
}

/// Mana T5 and Life T4 (prefixes), Fire Resistance T5 and Cold Resistance
/// T5 (suffixes); the ring lacks Cold Resistance.
fn ring_targets() -> Vec<Target> {
    targets(&[
        (MANA, Some(5)),
        (LIFE, Some(4)),
        (FIRE, Some(5)),
        (COLD, Some(5)),
    ])
}

/// The currency and omens a step says to use.
fn uses(step: &StepView) -> (String, Vec<String>) {
    let action = step.action.as_ref().expect("a step with an action");
    (action.uses.clone(), action.omens.clone())
}

fn owned(currency: &str, omens: &[&str]) -> (String, Vec<String>) {
    (
        currency.to_string(),
        omens.iter().map(|o| o.to_string()).collect(),
    )
}

/// Checks an outcome's verdict and transition; returns the step it names
/// next.
fn then<'a>(out: &'a Outcome, result: OutcomeKind, transition: &str) -> &'a StepView {
    assert!(out.reasons.is_empty(), "{:?}", out.reasons);
    assert_eq!(out.result, result, "{out:?}");
    assert_eq!(out.transition.as_deref(), Some(transition), "{out:?}");
    next_step(&out.next)
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

#[test]
fn jewellery_catalyst_exalt_hit_path() {
    let engine = codex();
    let list = ring_targets();
    let ring = state("01-start");
    let (mut guide, first) = start(&engine, ID, &ring, &list, &[]);
    // The catalysts come first: a manual step, no action, no odds.
    assert_eq!((first.id.as_str(), first.goal), ("catalyst", Goal::Manual));
    assert!(first.action.is_none() && first.odds.is_none());

    let out = observe(&engine, &mut guide, &ring, &ring);
    let slam = then(&out, Hit, "next");
    assert_eq!(slam.id, "slam");
    assert_eq!(slam.target.as_deref(), Some(COLD));
    assert_eq!(
        uses(slam),
        owned("greater-exalted-orb", &[CATALYSING, DEXTRAL_EXALTATION])
    );

    let hit = state("02-slam-cold");
    let out = observe(&engine, &mut guide, &ring, &hit);
    assert!(out.reasons.is_empty(), "{:?}", out.reasons);
    assert_eq!(out.result, Hit);
    assert_eq!(out.transition.as_deref(), Some("done"));
    assert_eq!(out.next, Next::Done);
    assert_eq!(guide.status, Status::Done);
    let gained: Vec<&str> = out.gained.iter().map(|w| w.mod_id.as_str()).collect();
    assert_eq!(gained, [COLD]);
}

#[test]
fn jewellery_catalyst_exalt_miss_path() {
    let engine = codex();
    let list = ring_targets();
    let ring = state("01-start");
    let missed = state("03-slam-strength");
    let (mut guide, _) = start(&engine, ID, &ring, &list, &[]);
    let out = observe(&engine, &mut guide, &ring, &ring);
    assert_eq!(then(&out, Hit, "next").id, "slam");

    // Strength instead: the miss is a suffix, so the prefix annulment is
    // passed over and the suffix one takes it with Dextral Annulment.
    let out = observe(&engine, &mut guide, &ring, &missed);
    let annul = then(&out, Miss, "next");
    assert_eq!(annul.id, "annul-suffix");
    let skipped: Vec<&str> = out.skipped.iter().map(|s| s.step.as_str()).collect();
    assert_eq!(skipped, ["annul-prefix"]);
    assert_eq!(uses(annul), owned("orb-of-annulment", &[DEXTRAL_ANNULMENT]));

    // The annulment takes it: catalysts again, then the slam lands.
    let out = observe(&engine, &mut guide, &missed, &ring);
    assert_eq!(then(&out, Hit, "goto:catalyst").id, "catalyst");
    let out = observe(&engine, &mut guide, &ring, &ring);
    assert_eq!(then(&out, Hit, "next").id, "slam");
    let out = observe(&engine, &mut guide, &ring, &state("02-slam-cold"));
    assert_eq!(out.result, Hit);
    assert_eq!(out.next, Next::Done);
    assert_eq!(guide.status, Status::Done);
}

#[test]
fn jewellery_catalyst_exalt_start_refuses() {
    let engine = codex();
    let list = ring_targets();
    let failed = |it: &Item| {
        let plan = engine.plan(it, &list, &[]).unwrap();
        match plan
            .rejected
            .iter()
            .find(|r| r.blueprint == ID)
            .unwrap_or_else(|| panic!("{ID} is not rejected: {:?}", plan.routes))
            .reason
            .clone()
        {
            Rejection::NoStartFits { failed } => failed,
            other => panic!("expected no start to fit, got {other}"),
        }
    };
    // A modifier that is not a target: a miss could not be told apart.
    assert_eq!(
        failed(&state("03-slam-strength")),
        vec![vec!["1 unwanted modifiers, at most 0 allowed".to_string()]]
    );
    // Not rare.
    assert_eq!(
        failed(&state("04-magic-amulet")),
        vec![vec!["the item is magic, not rare".to_string()]]
    );
    // Control: the clean ring is offered, and among the shipped
    // blueprints too.
    let plan = engine.plan(&state("01-start"), &list, &[]).unwrap();
    assert!(plan.routes.iter().any(|r| r.blueprint == ID));
    let plan = shipped().plan(&state("01-start"), &list, &[]).unwrap();
    assert!(plan.routes.iter().any(|r| r.blueprint == ID));
}

/// The slam on the clean ring, with `ids` blocked.
fn slam(ids: &[&str]) -> StepView {
    let plan = codex()
        .plan(&state("01-start"), &ring_targets(), &blocked(ids))
        .unwrap();
    let route = plan
        .routes
        .iter()
        .find(|r| r.blueprint == ID)
        .unwrap_or_else(|| panic!("{ID} does not plan: {:?}", plan.rejected));
    route
        .steps
        .iter()
        .find(|s| s.id == "slam")
        .expect("a slam step")
        .clone()
}

/// Baseline: a slam with no Catalysing omen (here the blueprint's third
/// alternative, Catalysing blocked) has the plain odds basis.
#[test]
fn a_plain_slam_carries_no_catalyst_note_baseline() {
    let plain = slam(&[CATALYSING]);
    assert_eq!(plain.action.as_ref().unwrap().alternative, 2);
    assert_eq!(
        uses(&plain),
        owned("greater-exalted-orb", &[DEXTRAL_EXALTATION])
    );
    let (_, _, basis) = known(&plain.odds);
    assert!(!basis.contains(CATALYST_NOTE), "{basis}");
}

/// A slam with Omen of Catalysing Exaltation has the plain slam's numbers
/// (the favour is not counted) and a basis that says so.
#[test]
fn a_catalysing_slam_says_the_favour_is_not_counted() {
    let catalysing = slam(&[]);
    assert_eq!(catalysing.action.as_ref().unwrap().alternative, 0);
    let (hits, total, basis) = known(&catalysing.odds);
    assert!(basis.ends_with(CATALYST_NOTE), "{basis}");
    let (plain_hits, plain_total, plain_basis) = known(&slam(&[CATALYSING]).odds);
    assert_eq!((hits, total), (plain_hits, plain_total));
    assert_eq!(basis, format!("{plain_basis}, {CATALYST_NOTE}"));
    // The plain Exalted Orb with Catalysing (Greater blocked) says it too.
    let cheaper = slam(&["greater-exalted-orb"]);
    assert_eq!(
        uses(&cheaper),
        owned("exalted-orb", &[CATALYSING, DEXTRAL_EXALTATION])
    );
    let (_, _, basis) = known(&cheaper.odds);
    assert!(basis.ends_with(CATALYST_NOTE), "{basis}");
}
