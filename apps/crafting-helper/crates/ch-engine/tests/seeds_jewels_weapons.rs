//! The seed blueprints for jewels, weapons and boots (the owner's posts 3,
//! 6, 8, 12 and 13), each walked from its start to its end twice: a hit
//! path, where every step hits and the guide reaches done, and a miss path,
//! where a miss follows the blueprint's miss transition and the walk then
//! reaches done or abandon. The item states are constructed, under
//! `tests/items/jewels-weapons/<id>/` (each folder's README lists them).

mod common;

use ch_engine::{Engine, Guide, Next, Odds, OutcomeKind, Status, Target};
use ch_item::Item;
use common::{crate_dir, item, next_step, observe, start, targets};

const SEEDS: [&str; 5] = [
    "jewels-three-one-side",
    "time-lost-jewel-radius",
    "weapons-alloy-budget",
    "caster-cast-speed",
    "boots-runic-ward",
];

/// An engine over this item's five seed blueprints, read from
/// `apps/crafting-helper/blueprints/` (the other seeds are another test's).
fn seeds() -> Engine {
    let dir = crate_dir().join("..").join("..").join("blueprints");
    let paths: Vec<_> = SEEDS
        .iter()
        .map(|id| dir.join(format!("{id}.toml")))
        .collect();
    Engine::load(&paths).unwrap_or_else(|errors| {
        let lines: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        panic!("seed blueprints do not load:\n{}", lines.join("\n"))
    })
}

/// The item state `tests/items/jewels-weapons/<blueprint>/<file>`.
fn state(blueprint: &str, file: &str) -> Item {
    item(&format!("jewels-weapons/{blueprint}/{file}"))
}

/// One change of the walk: the step the guide must be on, the item state
/// after the player took it, and the verdict and transition expected.
struct Change {
    step: &'static str,
    after: &'static str,
    result: OutcomeKind,
    transition: &'static str,
}

const fn hit(step: &'static str, after: &'static str, transition: &'static str) -> Change {
    Change {
        step,
        after,
        result: OutcomeKind::Hit,
        transition,
    }
}

const fn miss(step: &'static str, after: &'static str, transition: &'static str) -> Change {
    Change {
        step,
        after,
        result: OutcomeKind::Miss,
        transition,
    }
}

/// Observes each change in turn from `before`, checking the step the guide
/// was on, the verdict and the transition; no outcome may lose a target
/// unless it is a miss. Returns the last outcome's next.
fn walk(
    engine: &Engine,
    guide: &mut Guide,
    blueprint: &str,
    mut before: Item,
    changes: &[Change],
) -> Next {
    let mut last = None;
    for (i, change) in changes.iter().enumerate() {
        assert_eq!(
            guide.step, change.step,
            "change {i}: the guide is on `{}`, not `{}`",
            guide.step, change.step
        );
        let after = state(blueprint, change.after);
        let out = observe(engine, guide, &before, &after);
        assert_eq!(
            out.result, change.result,
            "change {i} (`{}` to {}): {:?}",
            change.step, change.after, out.reasons
        );
        assert_eq!(
            out.transition.as_deref(),
            Some(change.transition),
            "change {i} (`{}` to {})",
            change.step,
            change.after
        );
        if out.result == OutcomeKind::Hit {
            assert!(out.lost.is_empty(), "change {i}: a hit lost {:?}", out.lost);
        }
        before = after;
        last = Some(out.next);
    }
    last.expect("at least one change")
}

fn started(
    engine: &Engine,
    blueprint: &str,
    targets: &[Target],
) -> (Guide, ch_engine::StepView, Item) {
    let first = state(blueprint, "0-start.txt");
    let (guide, step) = start(engine, blueprint, &first, targets, &[]);
    (guide, step, first)
}

fn unknown(odds: &Option<Odds>) -> &str {
    match odds {
        Some(Odds::Unknown { reason }) => reason,
        other => panic!("expected unknown odds, got {other:?}"),
    }
}

// --- jewels-three-one-side (post 6) --------------------------------------

const J3: &str = "jewels-three-one-side";
const J_SPELL: &str = "prefix-increased-spell-damage";
const J_ES: &str = "prefix-increased-maximum-energy-shield";
const J_EXPOSURE: &str = "desecrated-prefix-increased-exposure-effect";
const J_CRIT: &str = "suffix-increased-critical-hit-chance-for-spells";

/// Two prefixes on the jewel, a desecrated third prefix, a suffix to chaos.
fn j3_targets() -> Vec<Target> {
    targets(&[
        (J_SPELL, None),
        (J_ES, None),
        (J_EXPOSURE, None),
        (J_CRIT, None),
    ])
}

#[test]
fn jewels_three_one_side_hit_path() {
    let engine = seeds();
    let (mut guide, first, start_item) = started(&engine, J3, &j3_targets());
    assert_eq!(first.id, "contempt");
    assert_eq!(
        first.action.as_ref().unwrap().uses,
        "potent-liquid-contempt"
    );
    // A removal: two of the four modifiers are not wanted.
    assert_eq!(first.odds.as_ref().unwrap().percent(), Some(50.0));
    // The desecration takes the desecrated-section target, on its side;
    // the reveal takes the same one; the chaos the suffix left.
    assert_eq!(
        guide.bindings.get("desecrate").map(String::as_str),
        Some(J_EXPOSURE)
    );
    assert_eq!(
        guide.bindings.get("reveal").map(String::as_str),
        Some(J_EXPOSURE)
    );
    assert_eq!(
        guide.bindings.get("reroll").map(String::as_str),
        Some(J_CRIT)
    );

    let next = walk(
        &engine,
        &mut guide,
        J3,
        start_item.clone(),
        &[hit("contempt", "1-contempt.txt", "next")],
    );
    let desecrate = next_step(&next);
    let action = desecrate.action.as_ref().unwrap();
    assert_eq!(
        (action.uses.as_str(), action.omens.as_slice()),
        (
            "preserved-cranium",
            &["omen-of-sinistral-necromancy".to_string()][..]
        )
    );

    let next = walk(
        &engine,
        &mut guide,
        J3,
        state(J3, "1-contempt.txt"),
        &[
            hit("desecrate", "2-desecrated.txt", "next"),
            hit("reveal", "3-revealed.txt", "goto:clean-prefix"),
        ],
    );
    // Three wanted prefixes: nothing to clean there; the Contempt modifier
    // is on the suffixes, removed with a Dextral Annulment.
    let clean = next_step(&next);
    assert_eq!(clean.id, "clean-suffix");
    assert_eq!(
        clean.action.as_ref().unwrap().omens,
        ["omen-of-dextral-annulment"]
    );

    let next = walk(
        &engine,
        &mut guide,
        J3,
        state(J3, "3-revealed.txt"),
        &[
            hit("clean-suffix", "4-cleaned.txt", "next"),
            hit("fill", "5-filled.txt", "next"),
        ],
    );
    let reroll = next_step(&next);
    assert_eq!(reroll.target.as_deref(), Some(J_CRIT));
    assert_eq!(
        reroll.action.as_ref().unwrap().omens,
        ["omen-of-dextral-erasure"]
    );

    let next = walk(
        &engine,
        &mut guide,
        J3,
        state(J3, "5-filled.txt"),
        &[
            hit("reroll", "6-rerolled.txt", "next"),
            hit("finish", "6-rerolled.txt", "done"),
        ],
    );
    assert_eq!(next, Next::Done);
    assert_eq!(guide.status, Status::Done);
}

#[test]
fn jewels_three_one_side_miss_path() {
    let engine = seeds();

    // The reveal shows another desecrated modifier: Omen of Light, then
    // desecrate again; the second reveal hits and the route goes on.
    let (mut guide, _, start_item) = started(&engine, J3, &j3_targets());
    let next = walk(
        &engine,
        &mut guide,
        J3,
        start_item.clone(),
        &[
            hit("contempt", "1-contempt.txt", "next"),
            hit("desecrate", "2-desecrated.txt", "next"),
            miss("reveal", "3-revealed-other.txt", "goto:unseal"),
            hit("unseal", "1-contempt.txt", "goto:desecrate"),
            hit("desecrate", "2-desecrated.txt", "next"),
            hit("reveal", "3-revealed.txt", "goto:clean-prefix"),
            hit("clean-suffix", "4-cleaned.txt", "next"),
            hit("fill", "5-filled.txt", "next"),
            hit("reroll", "6-rerolled.txt", "next"),
            hit("finish", "6-rerolled.txt", "done"),
        ],
    );
    assert_eq!(next, Next::Done);

    // The liquid took a wanted prefix: start over with another jewel.
    let (mut guide, _, start_item) = started(&engine, J3, &j3_targets());
    let out = observe(
        &engine,
        &mut guide,
        &start_item,
        &state(J3, "1-contempt-took-wanted.txt"),
    );
    assert_eq!(out.result, OutcomeKind::Miss);
    assert_eq!(out.lost.len(), 1);
    assert_eq!(out.lost[0].mod_id, J_SPELL);
    assert_eq!(out.transition.as_deref(), Some("abandon"));
    assert_eq!(out.next, Next::Abandon);
    assert_eq!(guide.status, Status::Abandoned);
}

// --- time-lost-jewel-radius (post 3) -------------------------------------

const TL: &str = "time-lost-jewel-radius";
const TL_CHANCE: &str =
    "suffix-notable-passive-skills-in-radius-also-grant-increased-critical-hit-chance-for-spells";
const TL_BONUS: &str =
    "suffix-notable-passive-skills-in-radius-also-grant-increased-critical-spell-damage-bonus";
const TL_POTENCY: &str =
    "suffix-notable-passive-skills-in-radius-also-grant-increased-critical-damage-bonus";
const TL_MANA: &str =
    "desecrated-prefix-notable-passive-skills-in-radius-also-grant-increased-maximum-mana";

fn tl_targets() -> Vec<Target> {
    targets(&[
        (TL_CHANCE, None),
        (TL_BONUS, None),
        (TL_POTENCY, None),
        (TL_MANA, None),
    ])
}

#[test]
fn time_lost_jewel_radius_hit_path() {
    let engine = seeds();
    let (mut guide, first, start_item) = started(&engine, TL, &tl_targets());
    assert_eq!(first.id, "contempt");
    assert_eq!(
        first.action.as_ref().unwrap().uses,
        "ancient-potent-liquid-contempt"
    );

    let next = walk(
        &engine,
        &mut guide,
        TL,
        start_item,
        &[
            hit("contempt", "1-contempt.txt", "next"),
            hit("exalt", "2-exalted.txt", "next"),
        ],
    );
    // No bad suffix to fix: on to removing the Contempt prefix.
    let remove = next_step(&next);
    assert_eq!(remove.id, "remove-contempt");
    assert_eq!(
        remove.action.as_ref().unwrap().omens,
        ["omen-of-sinistral-annulment"]
    );

    let next = walk(
        &engine,
        &mut guide,
        TL,
        state(TL, "2-exalted.txt"),
        &[
            hit("remove-contempt", "3-contempt-removed.txt", "next"),
            hit("desecrate", "4-desecrated.txt", "next"),
            // The radius liquid left the unrevealed modifier: no second
            // desecration, straight to the reveal.
            hit("radius", "5-radius.txt", "next"),
        ],
    );
    let reveal = next_step(&next);
    assert_eq!(reveal.id, "reveal");
    assert_eq!(reveal.target.as_deref(), Some(TL_MANA));
    assert!(unknown(&reveal.odds).contains("desecration"));

    let next = walk(
        &engine,
        &mut guide,
        TL,
        state(TL, "5-radius.txt"),
        &[hit("reveal", "6-revealed.txt", "done")],
    );
    assert_eq!(next, Next::Done);
    assert_eq!(guide.status, Status::Done);
}

#[test]
fn time_lost_jewel_radius_miss_path() {
    let engine = seeds();

    // A bad suffix is annulled and the exalt taken again; a bad reveal is
    // removed with Omen of Light, desecrated again, the radius confirmed,
    // and revealed again.
    let (mut guide, _, start_item) = started(&engine, TL, &tl_targets());
    let next = walk(
        &engine,
        &mut guide,
        TL,
        start_item.clone(),
        &[
            hit("contempt", "1-contempt.txt", "next"),
            miss("exalt", "2-exalted-other.txt", "goto:fix-suffix"),
            hit("fix-suffix", "1-contempt.txt", "goto:exalt"),
            hit("exalt", "2-exalted.txt", "next"),
            hit("remove-contempt", "3-contempt-removed.txt", "next"),
            hit("desecrate", "4-desecrated.txt", "next"),
            hit("radius", "5-radius.txt", "next"),
            miss("reveal", "6-revealed-other.txt", "goto:unseal"),
            hit("unseal", "5-radius-unsealed.txt", "goto:redesecrate"),
            hit("redesecrate", "5-radius.txt", "goto:radius"),
            hit("radius", "5-radius.txt", "next"),
            hit("reveal", "6-revealed.txt", "done"),
        ],
    );
    assert_eq!(next, Next::Done);

    // The liquid took a wanted suffix: start over with another jewel.
    let (mut guide, _, start_item) = started(&engine, TL, &tl_targets());
    let out = observe(
        &engine,
        &mut guide,
        &start_item,
        &state(TL, "1-contempt-took-wanted.txt"),
    );
    assert_eq!(out.result, OutcomeKind::Miss);
    assert_eq!(out.lost[0].mod_id, TL_CHANCE);
    assert_eq!(out.next, Next::Abandon);
    assert_eq!(guide.status, Status::Abandoned);
}

// --- weapons-alloy-budget (post 12) --------------------------------------

const WB: &str = "weapons-alloy-budget";
const W_PHYS: &str = "prefix-increased-physical-damage";
const W_FLAT: &str = "prefix-adds-to-physical-damage";
const W_WARCRY: &str = "desecrated-suffix-increased-warcry-cooldown-recovery-rate";

/// Physical damage at T2 or better (the hardest), flat physical damage at
/// T3 or better, a desecrated suffix.
fn wb_targets() -> Vec<Target> {
    targets(&[(W_FLAT, Some(3)), (W_PHYS, Some(2)), (W_WARCRY, None)])
}

#[test]
fn weapons_alloy_budget_hit_path() {
    let engine = seeds();
    let (mut guide, first, start_item) = started(&engine, WB, &wb_targets());
    // The chaos aims at the hardest target, wherever it is listed.
    assert_eq!(first.id, "chaos");
    assert_eq!(first.target.as_deref(), Some(W_PHYS));
    assert!(first.odds.as_ref().unwrap().percent().is_some());
    assert_eq!(guide.bindings.get("slam").map(String::as_str), Some(W_FLAT));
    assert_eq!(
        guide.bindings.get("desecrate").map(String::as_str),
        Some(W_WARCRY)
    );

    let next = walk(
        &engine,
        &mut guide,
        WB,
        start_item,
        &[
            hit("chaos", "1-chaos.txt", "next"),
            hit("filler-one", "2-filler-one.txt", "next"),
        ],
    );
    let celestial = next_step(&next);
    let action = celestial.action.as_ref().unwrap();
    assert_eq!(
        (action.uses.as_str(), action.omens.as_slice()),
        (
            "celestial-alloy",
            &["omen-of-dextral-crystallisation".to_string()][..]
        )
    );
    assert!(celestial.odds.is_none(), "a manual step has no odds");

    let next = walk(
        &engine,
        &mut guide,
        WB,
        state(WB, "2-filler-one.txt"),
        &[
            // The alloy's modifier joins nothing in the data; its own lines
            // are what the engine accepts.
            hit("celestial", "3-celestial.txt", "next"),
            hit("filler-two", "4-filler-two.txt", "next"),
            hit("transcendent", "5-transcendent.txt", "next"),
        ],
    );
    let slam = next_step(&next);
    let action = slam.action.as_ref().unwrap();
    assert_eq!(
        (action.uses.as_str(), action.omens.as_slice()),
        (
            "perfect-exalted-orb",
            &["omen-of-sinistral-exaltation".to_string()][..]
        )
    );

    let next = walk(
        &engine,
        &mut guide,
        WB,
        state(WB, "5-transcendent.txt"),
        &[hit("slam", "6-slam.txt", "next")],
    );
    let desecrate = next_step(&next);
    let action = desecrate.action.as_ref().unwrap();
    assert_eq!(
        (action.uses.as_str(), action.omens.as_slice()),
        (
            "preserved-jawbone",
            &["omen-of-dextral-necromancy".to_string()][..]
        )
    );

    let next = walk(
        &engine,
        &mut guide,
        WB,
        state(WB, "6-slam.txt"),
        &[
            hit("desecrate", "7-desecrated.txt", "next"),
            hit("reveal", "8-revealed.txt", "done"),
        ],
    );
    assert_eq!(next, Next::Done);
    assert_eq!(guide.status, Status::Done);
}

#[test]
fn weapons_alloy_budget_miss_path() {
    let engine = seeds();
    let (mut guide, _, start_item) = started(&engine, WB, &wb_targets());

    // The first chaos rolls physical damage at T5: below its floor, a miss
    // that repeats the step.
    let below = state(WB, "1-chaos-below-floor.txt");
    let out = observe(&engine, &mut guide, &start_item, &below);
    assert_eq!(out.result, OutcomeKind::Miss);
    assert_eq!(out.below_floor.len(), 1);
    assert_eq!(
        (out.below_floor[0].mod_id.as_str(), out.below_floor[0].tier),
        (W_PHYS, Some(5))
    );
    assert_eq!(out.transition.as_deref(), Some("repeat"));
    assert_eq!((guide.step.as_str(), guide.tries), ("chaos", 1));

    // Then a bad reveal goes back through Omen of Light to the desecration.
    let next = walk(
        &engine,
        &mut guide,
        WB,
        below,
        &[
            hit("chaos", "1-chaos.txt", "next"),
            hit("filler-one", "2-filler-one.txt", "next"),
            hit("celestial", "3-celestial.txt", "next"),
            hit("filler-two", "4-filler-two.txt", "next"),
            hit("transcendent", "5-transcendent.txt", "next"),
            hit("slam", "6-slam.txt", "next"),
            hit("desecrate", "7-desecrated.txt", "next"),
            miss("reveal", "8-revealed-other.txt", "goto:unseal"),
            hit("unseal", "6-slam.txt", "goto:desecrate"),
            hit("desecrate", "7-desecrated.txt", "next"),
            hit("reveal", "8-revealed.txt", "done"),
        ],
    );
    assert_eq!(next, Next::Done);
    assert_eq!(guide.status, Status::Done);
}

// --- caster-cast-speed (post 13) -----------------------------------------

const CC: &str = "caster-cast-speed";
const C_SPELL: &str = "prefix-increased-spell-damage";
const C_CRIT: &str = "suffix-increased-critical-hit-chance-for-spells";
const C_ELEMENTAL: &str = "desecrated-prefix-increased-elemental-damage";

fn cc_targets() -> Vec<Target> {
    targets(&[(C_SPELL, Some(4)), (C_CRIT, Some(4)), (C_ELEMENTAL, None)])
}

#[test]
fn caster_cast_speed_hit_path() {
    let engine = seeds();
    let (mut guide, first, start_item) = started(&engine, CC, &cc_targets());
    assert_eq!(first.id, "filler");
    assert_eq!(
        guide.bindings.get("last").map(String::as_str),
        Some(C_ELEMENTAL)
    );

    let next = walk(
        &engine,
        &mut guide,
        CC,
        start_item,
        &[
            hit("filler", "1-filler.txt", "next"),
            hit("cast-speed", "2-cast-speed.txt", "next"),
            hit("block", "3-blocked.txt", "next"),
        ],
    );
    let double = next_step(&next);
    assert_eq!(double.targets, [C_CRIT]);
    let action = double.action.as_ref().unwrap();
    assert_eq!(action.omens, ["omen-of-greater-exaltation"]);

    let next = walk(
        &engine,
        &mut guide,
        CC,
        state(CC, "3-blocked.txt"),
        &[
            // Two modifiers at once, one of them the wanted suffix.
            hit("double-slam", "4-double-slam.txt", "next"),
            hit("unblock", "5-unblocked.txt", "next"),
        ],
    );
    let last = next_step(&next);
    assert_eq!(last.target.as_deref(), Some(C_ELEMENTAL));
    assert_eq!(
        last.action.as_ref().unwrap().omens,
        ["omen-of-sinistral-necromancy"]
    );

    let next = walk(
        &engine,
        &mut guide,
        CC,
        state(CC, "5-unblocked.txt"),
        &[
            hit("last", "6-desecrated.txt", "next"),
            hit("reveal", "7-revealed.txt", "done"),
        ],
    );
    assert_eq!(next, Next::Done);
    assert_eq!(guide.status, Status::Done);
}

#[test]
fn caster_cast_speed_miss_path() {
    let engine = seeds();
    let (mut guide, _, start_item) = started(&engine, CC, &cc_targets());
    let next = walk(
        &engine,
        &mut guide,
        CC,
        start_item,
        &[
            hit("filler", "1-filler.txt", "next"),
            hit("cast-speed", "2-cast-speed.txt", "next"),
            hit("block", "3-blocked.txt", "next"),
            hit("double-slam", "4-double-slam.txt", "next"),
            hit("unblock", "5-unblocked.txt", "next"),
            hit("last", "6-desecrated.txt", "next"),
            miss("reveal", "7-revealed-other.txt", "goto:redo"),
            hit("redo", "5-unblocked.txt", "goto:last"),
            hit("last", "6-desecrated.txt", "next"),
            hit("reveal", "7-revealed.txt", "done"),
        ],
    );
    assert_eq!(next, Next::Done);

    // The double exalt brings nothing wanted: the post says nothing more,
    // so the route moves on to removing the blocking desecration.
    let (mut guide, _, start_item) = started(&engine, CC, &cc_targets());
    walk(
        &engine,
        &mut guide,
        CC,
        start_item,
        &[
            hit("filler", "1-filler.txt", "next"),
            hit("cast-speed", "2-cast-speed.txt", "next"),
            hit("block", "3-blocked.txt", "next"),
        ],
    );
    let blocked = state(CC, "3-blocked.txt");
    let nothing = common::parse(
        // A CRLF checkout must not hide the line pair from the replace.
        &common::item_text("jewels-weapons/caster-cast-speed/4-double-slam.txt")
            .replace("\r\n", "\n")
            .replace(
                "{ Suffix Modifier \"of Ruin\" (Tier: 2) — Caster, Critical }\n56(54-59)% increased Critical Hit Chance for Spells",
                "{ Suffix Modifier \"of the Savant\" (Tier: 2) — Attribute }\n+29(28-30) to Intelligence",
            ),
    );
    let out = observe(&engine, &mut guide, &blocked, &nothing);
    assert_eq!(out.result, OutcomeKind::Miss);
    assert_eq!(out.transition.as_deref(), Some("next"));
    assert_eq!(next_step(&out.next).id, "unblock");
}

// --- boots-runic-ward (post 8) -------------------------------------------

const BR: &str = "boots-runic-ward";
const B_MS: &str = "prefix-increased-movement-speed";
const B_DEX: &str = "suffix-to-dexterity";
const B_INT: &str = "suffix-to-intelligence";

fn br_targets() -> Vec<Target> {
    targets(&[(B_MS, Some(2)), (B_DEX, Some(4)), (B_INT, Some(4))])
}

#[test]
fn boots_runic_ward_hit_path() {
    let engine = seeds();
    let (mut guide, first, start_item) = started(&engine, BR, &br_targets());
    // The fractured movement speed is the wanted start; either attribute
    // is a hit for the Chaos Orb.
    assert_eq!(first.id, "attribute");
    assert_eq!(first.targets, [B_DEX, B_INT]);

    let next = walk(
        &engine,
        &mut guide,
        BR,
        start_item,
        &[
            hit("attribute", "1-chaos.txt", "next"),
            hit("filler-one", "2-filler-one.txt", "next"),
            hit("augment", "3-augment.txt", "next"),
            hit("filler-two", "4-filler-two.txt", "next"),
            hit("rune", "4-filler-two.txt", "next"),
            hit("runic-ward", "5-runic-ward.txt", "next"),
        ],
    );
    // One attribute left: the desecration aims at it, on the suffixes.
    let desecrate = next_step(&next);
    assert_eq!(desecrate.target.as_deref(), Some(B_INT));
    let action = desecrate.action.as_ref().unwrap();
    assert_eq!(
        (action.uses.as_str(), action.omens.as_slice()),
        (
            "preserved-rib",
            &["omen-of-dextral-necromancy".to_string()][..]
        )
    );

    let next = walk(
        &engine,
        &mut guide,
        BR,
        state(BR, "5-runic-ward.txt"),
        &[
            hit("desecrate", "6-desecrated.txt", "next"),
            hit("reveal", "7-revealed.txt", "next"),
            hit("last-prefix", "8-last-prefix.txt", "next"),
            hit("runes", "8-last-prefix.txt", "done"),
        ],
    );
    assert_eq!(next, Next::Done);
    assert_eq!(guide.status, Status::Done);
}

#[test]
fn boots_runic_ward_miss_path() {
    let engine = seeds();
    let (mut guide, _, start_item) = started(&engine, BR, &br_targets());
    let next = walk(
        &engine,
        &mut guide,
        BR,
        start_item,
        &[
            miss("attribute", "1-chaos-other.txt", "repeat"),
            hit("attribute", "1-chaos.txt", "next"),
            hit("filler-one", "2-filler-one.txt", "next"),
            hit("augment", "3-augment.txt", "next"),
            hit("filler-two", "4-filler-two.txt", "next"),
            hit("rune", "4-filler-two.txt", "next"),
            hit("runic-ward", "5-runic-ward.txt", "next"),
            hit("desecrate", "6-desecrated.txt", "next"),
            miss("reveal", "7-revealed-other.txt", "goto:unseal"),
            hit("unseal", "5-runic-ward.txt", "goto:desecrate"),
            hit("desecrate", "6-desecrated.txt", "next"),
            hit("reveal", "7-revealed.txt", "next"),
            hit("last-prefix", "8-last-prefix.txt", "next"),
            hit("runes", "8-last-prefix.txt", "done"),
        ],
    );
    assert_eq!(next, Next::Done);
    assert_eq!(guide.status, Status::Done);
}
