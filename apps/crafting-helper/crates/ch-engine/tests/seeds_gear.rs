//! The five equipment seed blueprints in `apps/crafting-helper/blueprints/`,
//! each walked twice: a hit path (every step taken hits and the guide
//! reaches done) and a miss path (a miss follows the blueprint's miss
//! transition, and the walk then reaches done). The item states are in
//! `tests/items/gear/<blueprint id>/` (its README says which are
//! constructed), plus `ch-item`'s real Sirenscale Gloves.

mod common;

use std::path::PathBuf;

use ch_engine::{Engine, Next, Outcome, OutcomeKind, Status, StepView, Target};
use ch_item::Item;
use common::{crate_dir, item, next_step, observe, real, start, targets};

use OutcomeKind::{Hit, Miss};

const IDS: [&str; 5] = [
    "fill-desecrate-exalt",
    "rings-resist-whittle",
    "rings-flat-attack",
    "gloves-crit-melee",
    "amulets-resist-spirit",
];

const LIFE: &str = "prefix-to-maximum-life";
const FIRE: &str = "suffix-to-fire-resistance";
const COLD: &str = "suffix-to-cold-resistance";
const LIGHTNING: &str = "suffix-to-lightning-resistance";
const CHAOS: &str = "suffix-to-chaos-resistance";

/// An engine over this file's five blueprints, read from the shipped
/// folder (not `Engine::shipped`, so another item's blueprint cannot
/// break these walks).
fn gear() -> Engine {
    let folder = crate_dir().join("..").join("..").join("blueprints");
    let paths: Vec<PathBuf> = IDS
        .iter()
        .map(|id| folder.join(format!("{id}.toml")))
        .collect();
    Engine::load(&paths).unwrap_or_else(|errors| {
        let lines: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        panic!("the gear blueprints do not load:\n{}", lines.join("\n"))
    })
}

/// An item state of `blueprint`'s walks: `real:<name>` is one of
/// `ch-item`'s real fixtures, anything else a file under
/// `tests/items/gear/<blueprint>/`.
fn state(blueprint: &str, file: &str) -> Item {
    match file.strip_prefix("real:") {
        Some(name) => real(name),
        None => item(&format!("gear/{blueprint}/{file}")),
    }
}

/// One change of the item and what the guide must make of it.
struct Change {
    after: &'static str,
    result: OutcomeKind,
    transition: &'static str,
    /// The step the guide moves to, or `done`.
    next: &'static str,
}

const fn change(
    after: &'static str,
    result: OutcomeKind,
    transition: &'static str,
    next: &'static str,
) -> Change {
    Change {
        after,
        result,
        transition,
        next,
    }
}

/// Starts `blueprint` on the state `first`, checks its first step is
/// `step`, then observes each change in turn and checks the verdict, the
/// transition and the next step. The walk must end done. Returns the first
/// step and every outcome.
fn walk(
    blueprint: &str,
    targets: &[Target],
    first: &str,
    step: &str,
    changes: &[Change],
) -> (StepView, Vec<Outcome>) {
    let engine = gear();
    let mut before = state(blueprint, first);
    let (mut guide, first_step) = start(&engine, blueprint, &before, targets, &[]);
    assert_eq!(first_step.id, step, "{blueprint}: the first step");
    let mut outcomes = Vec::new();
    for (i, c) in changes.iter().enumerate() {
        let at = guide.step.clone();
        let after = state(blueprint, c.after);
        let out = observe(&engine, &mut guide, &before, &after);
        let place = format!("{blueprint}, change {} (`{at}` to {})", i + 1, c.after);
        assert!(out.reasons.is_empty(), "{place}: {:?}", out.reasons);
        assert_eq!(out.result, c.result, "{place}");
        assert_eq!(out.transition.as_deref(), Some(c.transition), "{place}");
        if c.next == "done" {
            assert_eq!(out.next, Next::Done, "{place}");
            assert_eq!(guide.status, Status::Done, "{place}");
        } else {
            assert_eq!(next_step(&out.next).id, c.next, "{place}");
            assert_eq!(guide.status, Status::Active, "{place}");
        }
        outcomes.push(out);
        before = after;
    }
    assert_eq!(
        guide.status,
        Status::Done,
        "{blueprint}: the walk ends done"
    );
    (first_step, outcomes)
}

/// The currency and omens a step view says to use.
fn uses(step: &StepView) -> (String, Vec<String>) {
    let action = step
        .action
        .as_ref()
        .unwrap_or_else(|| panic!("step `{}` has no action", step.id));
    (action.uses.clone(), action.omens.clone())
}

fn next_of(out: &Outcome) -> &StepView {
    next_step(&out.next)
}

fn skipped(out: &Outcome) -> Vec<&str> {
    out.skipped.iter().map(|s| s.step.as_str()).collect()
}

fn gained(out: &Outcome) -> Vec<&str> {
    out.gained.iter().map(|w| w.mod_id.as_str()).collect()
}

// --- fill-desecrate-exalt (roadmap) --------------------------------------

/// Life T3 (the plain Essence of the Body's tier on rings), fire and cold
/// resistance T4, chaos resistance T3 (the hardest: 250 weight a tier).
fn fill_targets() -> Vec<Target> {
    targets(&[
        (LIFE, Some(3)),
        (FIRE, Some(4)),
        (COLD, Some(4)),
        (CHAOS, Some(3)),
    ])
}

#[test]
fn fill_desecrate_exalt_hit_path() {
    let dir = "fill-desecrate-exalt";
    // A Magic ring: the essence start, essence step first.
    let plan = gear()
        .plan(&state(dir, "01-magic.txt"), &fill_targets(), &[])
        .unwrap();
    let route = plan
        .routes
        .iter()
        .find(|r| r.blueprint == dir)
        .expect("the blueprint applies to rings");
    assert_eq!(route.start, 2, "the Magic start, which needs an essence");

    let (first, outs) = walk(
        dir,
        &fill_targets(),
        "01-magic.txt",
        "essence",
        &[
            change("02-essence-life.txt", Hit, "next", "fill"),
            change("03-fill-fire.txt", Hit, "next", "desecrate"),
            change("04-desecrated.txt", Hit, "next", "reveal"),
            change("05-revealed-chaos.txt", Hit, "next", "slam"),
            change("06-slam-cold.txt", Hit, "next", "done"),
        ],
    );
    // The cheapest essence that meets life's floor: the plain one (T3).
    assert_eq!(first.target.as_deref(), Some(LIFE));
    assert_eq!(uses(&first).0, "essence-of-the-body");
    assert_eq!(gained(&outs[0]), [LIFE]);
    // Two targets left after one exalt: the desecration takes the hardest,
    // on its side.
    let desecrate = next_of(&outs[1]);
    assert_eq!(desecrate.target.as_deref(), Some(CHAOS));
    assert_eq!(
        uses(desecrate),
        (
            "preserved-collarbone".to_string(),
            vec!["omen-of-dextral-necromancy".to_string()]
        )
    );
    assert_eq!(gained(&outs[3]), [CHAOS]);
    // The last slam: the best exalt, tied to the last target's side.
    let slam = next_of(&outs[3]);
    assert_eq!(slam.target.as_deref(), Some(COLD));
    assert_eq!(
        uses(slam),
        (
            "perfect-exalted-orb".to_string(),
            vec!["omen-of-dextral-exaltation".to_string()]
        )
    );
    assert!(outs.iter().all(|o| o.result == Hit));
    assert_eq!(
        skipped(&outs[4]),
        ["light", "fill-clean", "slam-clean"],
        "the miss-only steps are left out once nothing remains"
    );
}

#[test]
fn fill_desecrate_exalt_miss_path() {
    let dir = "fill-desecrate-exalt";
    // A Rare ring with two targets on it (life, fire): the second start.
    let plan = gear()
        .plan(&state(dir, "03-fill-fire.txt"), &fill_targets(), &[])
        .unwrap();
    let route = plan.routes.iter().find(|r| r.blueprint == dir).unwrap();
    assert_eq!(route.start, 1, "Rare with two wanted, nothing fractured");
    assert_eq!(
        route.skipped[0].step, "essence",
        "no essence on a Rare item"
    );

    let (_, outs) = walk(
        dir,
        &fill_targets(),
        "03-fill-fire.txt",
        "fill",
        &[
            // The exalt rolls accuracy: whittle it.
            change("07-fill-miss.txt", Miss, "goto:fill-clean", "fill-clean"),
            change("08-whittled.txt", Hit, "goto:fill", "fill"),
            change("09-fill-cold.txt", Hit, "next", "desecrate"),
            // The suffixes are full: the desecration takes one off first.
            change("10-desecrated.txt", Hit, "next", "reveal"),
            // The reveal is not chaos resistance: Omen of Light, desecrate again.
            change("11-revealed-wrong.txt", Miss, "goto:light", "light"),
            change("12-light.txt", Hit, "goto:desecrate", "desecrate"),
            change("10-desecrated.txt", Hit, "next", "reveal"),
            change("13-revealed-chaos.txt", Hit, "next", "done"),
        ],
    );
    assert_eq!(
        uses(next_of(&outs[0])),
        (
            "chaos-orb".to_string(),
            vec!["omen-of-whittling".to_string()]
        )
    );
    assert_eq!(
        uses(next_of(&outs[4])),
        (
            "orb-of-annulment".to_string(),
            vec!["omen-of-light".to_string()]
        )
    );
    assert!(outs[4].gained.is_empty() && outs[4].lost.is_empty());
    assert_eq!(gained(&outs[7]), [CHAOS]);
    assert_eq!(
        skipped(&outs[7]),
        ["slam", "light", "fill-clean", "slam-clean"]
    );
}

// --- rings-resist-whittle (post 10) --------------------------------------

/// Three resistances at T3 or better (item level 76 reaches T2).
fn whittle_targets() -> Vec<Target> {
    targets(&[(FIRE, Some(3)), (COLD, Some(3)), (LIGHTNING, Some(3))])
}

#[test]
fn rings_resist_whittle_hit_path() {
    let (first, outs) = walk(
        "rings-resist-whittle",
        &whittle_targets(),
        "01-start.txt",
        "chaos",
        &[
            change("02-chaos-fire.txt", Hit, "next", "whittle"),
            // One target left: whittling stops.
            change("03-whittle-cold.txt", Hit, "next", "desecrate"),
            change("04-desecrated.txt", Hit, "next", "reveal"),
            change("05-revealed-lightning.txt", Hit, "next", "done"),
        ],
    );
    assert_eq!(uses(&first), ("chaos-orb".to_string(), Vec::new()));
    assert_eq!(
        uses(next_of(&outs[0])),
        (
            "chaos-orb".to_string(),
            vec!["omen-of-whittling".to_string()]
        )
    );
    // The last target, desecrated with no omen (the post uses none).
    let desecrate = next_of(&outs[1]);
    assert_eq!(desecrate.target.as_deref(), Some(LIGHTNING));
    assert_eq!(
        uses(desecrate),
        ("preserved-collarbone".to_string(), Vec::new())
    );
    assert_eq!(gained(&outs[3]), [LIGHTNING]);
    assert!(outs.iter().all(|o| o.result == Hit));
}

#[test]
fn rings_resist_whittle_miss_path() {
    let (_, outs) = walk(
        "rings-resist-whittle",
        &whittle_targets(),
        "01-start.txt",
        "chaos",
        &[
            // Fire resistance at T5, below the T3 floor: a miss, chaos again.
            change("06-chaos-below-floor.txt", Miss, "repeat", "chaos"),
            change("02-chaos-fire.txt", Hit, "next", "whittle"),
            change("03-whittle-cold.txt", Hit, "next", "desecrate"),
            change("04-desecrated.txt", Hit, "next", "reveal"),
            // The reveal misses: back to whittling, which lands the last one.
            change("07-revealed-wrong.txt", Miss, "goto:whittle", "whittle"),
            change("08-whittle-lightning.txt", Hit, "next", "done"),
        ],
    );
    assert_eq!(outs[0].below_floor.len(), 1);
    assert_eq!(
        (
            outs[0].below_floor[0].mod_id.as_str(),
            outs[0].below_floor[0].tier
        ),
        (FIRE, Some(5))
    );
    assert_eq!(gained(&outs[5]), [LIGHTNING]);
    assert_eq!(skipped(&outs[5]), ["desecrate", "reveal"]);
}

// --- rings-flat-attack (post 11) -----------------------------------------

const PHYS_ATTACK: &str = "prefix-adds-to-physical-damage-to-attacks";
const LIGHTNING_ATTACK: &str = "prefix-adds-to-lightning-damage-to-attacks";
const COLD_ATTACK: &str = "prefix-adds-to-cold-damage-to-attacks";

/// Three flat attack prefixes at T5 or better, fire resistance at T4.
fn flat_targets() -> Vec<Target> {
    targets(&[
        (PHYS_ATTACK, Some(5)),
        (LIGHTNING_ATTACK, Some(5)),
        (COLD_ATTACK, Some(5)),
        (FIRE, Some(4)),
    ])
}

#[test]
fn rings_flat_attack_hit_path() {
    let (first, outs) = walk(
        "rings-flat-attack",
        &flat_targets(),
        "01-start.txt",
        "breach",
        &[
            // The essence swaps a suffix for its quality-cap modifier,
            // which joins no modifier of the data.
            change("02-breach.txt", Hit, "next", "catalyst"),
            change("03-catalysts.txt", Hit, "next", "chaos"),
            change("04-chaos-lightning.txt", Hit, "next", "last-flat"),
            change("05-slam-cold.txt", Hit, "next", "suffixes"),
            change("06-whittle-fire.txt", Hit, "next", "done"),
        ],
    );
    assert_eq!(uses(&first).0, "essence-of-the-breach");
    assert_eq!(uses(next_of(&outs[0])).0, "reaver-catalyst");
    assert_eq!(
        uses(next_of(&outs[2])),
        (
            "perfect-exalted-orb".to_string(),
            vec!["omen-of-sinistral-exaltation".to_string()]
        )
    );
    assert_eq!(gained(&outs[2]), [LIGHTNING_ATTACK]);
    assert_eq!(gained(&outs[3]), [COLD_ATTACK]);
    assert_eq!(gained(&outs[4]), [FIRE]);
    assert_eq!(skipped(&outs[4]), ["clean-prefix"]);
    assert!(outs.iter().all(|o| o.result == Hit));
}

#[test]
fn rings_flat_attack_miss_path() {
    let (_, outs) = walk(
        "rings-flat-attack",
        &flat_targets(),
        "01-start.txt",
        "breach",
        &[
            change("02-breach.txt", Hit, "next", "catalyst"),
            change("03-catalysts.txt", Hit, "next", "chaos"),
            // Accuracy, not a flat damage modifier: chaos again.
            change("07-chaos-miss.txt", Miss, "repeat", "chaos"),
            change("04-chaos-lightning.txt", Hit, "next", "last-flat"),
            // The slam lands mana: annul it from the prefix side, slam again.
            change(
                "08-slam-miss.txt",
                Miss,
                "goto:clean-prefix",
                "clean-prefix",
            ),
            change("04-chaos-lightning.txt", Hit, "goto:last-flat", "last-flat"),
            change("05-slam-cold.txt", Hit, "next", "suffixes"),
            change("06-whittle-fire.txt", Hit, "next", "done"),
        ],
    );
    assert_eq!(
        uses(next_of(&outs[4])),
        (
            "orb-of-annulment".to_string(),
            vec!["omen-of-sinistral-annulment".to_string()]
        )
    );
    assert!(outs[4].lost.is_empty() && outs[5].lost.is_empty());
}

// --- gloves-crit-melee (post 9) ------------------------------------------

const CRIT_DAMAGE: &str = "suffix-increased-critical-damage-bonus";
const ATTACK_SPEED: &str = "suffix-increased-attack-speed";
const ES_PERCENT: &str = "prefix-increased-energy-shield";
const REAL_GLOVES: &str = "real:gloves-rare-rune-desecrated-ctrl-alt-c.txt";

#[test]
fn gloves_crit_melee_hit_path() {
    // Critical damage at Essence of Hysteria's tier (T2), lightning damage
    // to attacks at T3, attack speed at T2.
    let list = targets(&[
        (CRIT_DAMAGE, Some(2)),
        (LIGHTNING_ATTACK, Some(3)),
        (ATTACK_SPEED, Some(2)),
    ]);
    let (first, outs) = walk(
        "gloves-crit-melee",
        &list,
        "01-start.txt",
        "exalt-suffix",
        &[
            change("02-exalt-suffix.txt", Hit, "next", "hysteria"),
            change("03-hysteria.txt", Hit, "next", "block-prefix"),
            change("04-block-prefix.txt", Hit, "next", "reveal-blocker"),
            change("05-blocker-revealed.txt", Hit, "next", "slam-prefix"),
            change("06-slam-lightning.txt", Hit, "next", "light"),
            change("07-light.txt", Hit, "next", "slam-again"),
            change("08-slam-again.txt", Hit, "next", "desecrate-suffix"),
            change("09-desecrate-suffix.txt", Hit, "next", "reveal"),
            change("10-reveal-attack-speed.txt", Hit, "next", "done"),
        ],
    );
    assert_eq!(
        uses(&first),
        (
            "exalted-orb".to_string(),
            vec!["omen-of-dextral-exaltation".to_string()]
        )
    );
    let hysteria = next_of(&outs[0]);
    assert_eq!(hysteria.target.as_deref(), Some(CRIT_DAMAGE));
    assert_eq!(
        uses(hysteria),
        (
            "essence-of-hysteria".to_string(),
            vec!["omen-of-dextral-crystallisation".to_string()]
        )
    );
    assert_eq!(gained(&outs[1]), [CRIT_DAMAGE]);
    assert_eq!(
        uses(next_of(&outs[1])),
        (
            "preserved-rib".to_string(),
            vec!["omen-of-sinistral-necromancy".to_string()]
        )
    );
    // The last target is a suffix: Dextral Necromancy.
    let desecrate = next_of(&outs[6]);
    assert_eq!(desecrate.target.as_deref(), Some(ATTACK_SPEED));
    assert_eq!(
        uses(desecrate).1,
        vec!["omen-of-dextral-necromancy".to_string()]
    );
    assert_eq!(gained(&outs[8]), [ATTACK_SPEED]);
    assert!(outs.iter().all(|o| o.result == Hit));
}

#[test]
fn gloves_crit_melee_miss_path() {
    // Increased energy shield at T2 and attack speed at T2: the late states
    // are ch-item's real Sirenscale Gloves, whose desecrated suffix
    // revealed as chaos resistance ("of Exile"), not attack speed.
    let list = targets(&[(ES_PERCENT, Some(2)), (ATTACK_SPEED, Some(2))]);
    let (_, outs) = walk(
        "gloves-crit-melee",
        &list,
        "01-start.txt",
        "exalt-suffix",
        &[
            change("02-exalt-suffix.txt", Hit, "next", "block-prefix"),
            change("11-miss-block-prefix.txt", Hit, "next", "reveal-blocker"),
            change("12-miss-blocker-revealed.txt", Hit, "next", "slam-prefix"),
            // The slam lands mana: the post goes on to the Omen of Light.
            change("13-miss-slam-mana.txt", Miss, "next", "light"),
            change("14-miss-light.txt", Hit, "next", "slam-again"),
            change("15-miss-slam-again.txt", Hit, "next", "desecrate-suffix"),
            change("16-miss-desecrate-suffix.txt", Hit, "next", "reveal"),
            change(REAL_GLOVES, Miss, "goto:light", "light"),
            // No free prefix any more: the slam is left out.
            change("17-miss-light-again.txt", Hit, "next", "desecrate-suffix"),
            change("16-miss-desecrate-suffix.txt", Hit, "next", "reveal"),
            change("18-miss-reveal-attack-speed.txt", Hit, "next", "done"),
        ],
    );
    assert_eq!(skipped(&outs[0]), ["hysteria"], "no essence target");
    assert!(outs[3].gained.is_empty());
    assert_eq!(gained(&outs[5]), [ES_PERCENT]);
    assert!(outs[7].gained.is_empty() && outs[7].lost.is_empty());
    assert_eq!(skipped(&outs[8]), ["slam-again"]);
    assert_eq!(gained(&outs[10]), [ATTACK_SPEED]);
}

// --- amulets-resist-spirit (post 14) -------------------------------------

const ES_FLAT: &str = "prefix-to-maximum-energy-shield";
const SPIRIT: &str = "prefix-to-spirit";

/// Chaos and fire resistance, life, flat energy shield, and Spirit for the
/// last desecration.
fn amulet_targets() -> Vec<Target> {
    targets(&[
        (CHAOS, Some(2)),
        (FIRE, Some(2)),
        (LIFE, Some(2)),
        (ES_FLAT, Some(3)),
        (SPIRIT, Some(2)),
    ])
}

#[test]
fn amulets_resist_spirit_hit_path() {
    let (first, outs) = walk(
        "amulets-resist-spirit",
        &amulet_targets(),
        "01-start.txt",
        "annul-down",
        &[
            change("02-annulled.txt", Hit, "next", "chaos-resistance"),
            change("03-chaos-cold.txt", Hit, "next", "void-flux"),
            change("04-void-chaos.txt", Hit, "next", "slam-resistance"),
            change("05-slam-lightning.txt", Hit, "next", "blazing-flux"),
            change(
                "06-blazing-fire.txt",
                Hit,
                "goto:desecrate-block",
                "desecrate-block",
            ),
            change("07-desecrate-block.txt", Hit, "next", "reveal-block"),
            change(
                "08-blocker-revealed.txt",
                Hit,
                "goto:slam-prefix",
                "slam-prefix",
            ),
            // Two prefixes are wanted: the slam repeats once.
            change("09-slam-life.txt", Hit, "repeat", "slam-prefix"),
            change("10-slam-es.txt", Hit, "next", "light"),
            change("11-light.txt", Hit, "next", "desecrate-spirit"),
            change("12-desecrate-spirit.txt", Hit, "next", "reveal-spirit"),
            change("13-spirit.txt", Hit, "next", "done"),
        ],
    );
    assert_eq!(uses(&first), ("orb-of-annulment".to_string(), Vec::new()));
    assert_eq!(gained(&outs[2]), [CHAOS]);
    assert_eq!(gained(&outs[4]), [FIRE]);
    assert_eq!(
        uses(next_of(&outs[2])),
        (
            "perfect-exalted-orb".to_string(),
            vec!["omen-of-dextral-exaltation".to_string()]
        )
    );
    // Spirit is a prefix on amulets: Sinistral Necromancy.
    let desecrate = next_of(&outs[9]);
    assert_eq!(desecrate.target.as_deref(), Some(SPIRIT));
    assert_eq!(
        uses(desecrate),
        (
            "preserved-collarbone".to_string(),
            vec!["omen-of-sinistral-necromancy".to_string()]
        )
    );
    assert_eq!(gained(&outs[11]), [SPIRIT]);
    assert!(outs.iter().all(|o| o.result == Hit));
}

#[test]
fn amulets_resist_spirit_miss_path() {
    let (_, outs) = walk(
        "amulets-resist-spirit",
        &amulet_targets(),
        "01-start.txt",
        "annul-down",
        &[
            change("02-annulled.txt", Hit, "next", "chaos-resistance"),
            change("03-chaos-cold.txt", Hit, "next", "void-flux"),
            // The flux lands chaos resistance at T5: back to the Chaos Orb.
            change(
                "14-void-below-floor.txt",
                Miss,
                "goto:chaos-resistance",
                "chaos-resistance",
            ),
            change("03-chaos-cold.txt", Hit, "next", "void-flux"),
            change("04-void-chaos.txt", Hit, "next", "slam-resistance"),
            change("05-slam-lightning.txt", Hit, "next", "blazing-flux"),
            change(
                "06-blazing-fire.txt",
                Hit,
                "goto:desecrate-block",
                "desecrate-block",
            ),
            change("07-desecrate-block.txt", Hit, "next", "reveal-block"),
            change(
                "08-blocker-revealed.txt",
                Hit,
                "goto:slam-prefix",
                "slam-prefix",
            ),
            // The slam lands mana: whittle it away, slam again.
            change(
                "15-slam-miss.txt",
                Miss,
                "goto:whittle-prefix",
                "whittle-prefix",
            ),
            change("09-slam-life.txt", Hit, "goto:slam-prefix", "slam-prefix"),
            change("10-slam-es.txt", Hit, "next", "light"),
            change("11-light.txt", Hit, "next", "desecrate-spirit"),
            change("12-desecrate-spirit.txt", Hit, "next", "reveal-spirit"),
            // A defence prefix, not Spirit: Omen of Light, desecrate again.
            change("16-spirit-wrong.txt", Miss, "goto:light", "light"),
            change("11-light.txt", Hit, "next", "desecrate-spirit"),
            change("12-desecrate-spirit.txt", Hit, "next", "reveal-spirit"),
            change("13-spirit.txt", Hit, "next", "done"),
        ],
    );
    assert_eq!(outs[2].below_floor.len(), 1);
    assert_eq!(
        (
            outs[2].below_floor[0].mod_id.as_str(),
            outs[2].below_floor[0].tier
        ),
        (CHAOS, Some(5))
    );
    assert_eq!(
        uses(next_of(&outs[9])),
        (
            "chaos-orb".to_string(),
            vec!["omen-of-whittling".to_string()]
        )
    );
    assert_eq!(gained(&outs[10]), [LIFE]);
    assert!(outs[14].gained.is_empty() && outs[14].lost.is_empty());
    assert_eq!(gained(&outs[17]), [SPIRIT]);
}
