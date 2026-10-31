//! A Well of Souls reveal's estimated odds (issue #49;
//! docs/models/crafting-slots-spec.md, "Reveal odds"): the engine's twin of
//! the slot model's `reveal_chance` against the parity file
//! `tests/items/slots/reveal-odds.json`, and the reveal step's odds on
//! constructed items (`tests/items/reveal-odds/` and
//! `tests/items/last-desecration/`, see their READMEs) in both orders. A
//! desecration's own odds and the `hardest` binding stay as they were.

mod common;

use ch_engine::ch_blueprint::Goal;
use ch_engine::ch_data::{self, Kind};
use ch_engine::{
    reveal_chance, reveal_chance_with, Engine, Odds, Route, StepView, Target,
    EXCLUSIVE_EQUAL_WEIGHT,
};
use ch_item::Item;
use common::{crate_dir, engine, item, shipped, start, targets};
use serde_json::Value;

const LIFE: &str = "prefix-to-maximum-life";
const MANA: &str = "prefix-to-maximum-mana";
const ARMOUR: &str = "prefix-to-armour";
const FIRE: &str = "suffix-to-fire-resistance";
const COLD: &str = "suffix-to-cold-resistance";
const CHAOS: &str = "suffix-to-chaos-resistance";
const SPIRIT: &str = "prefix-to-spirit";
const ARMOUR_PERCENT: &str = "prefix-increased-armour";
const SPELL_DAMAGE: &str = "prefix-increased-spell-damage";
const SKILL_SPEED: &str = "desecrated-suffix-increased-skill-speed";
/// A belt exclusive prefix with one tier.
const CHARMS: &str = "desecrated-prefix-chance-for-charms-you-use-to-not-consume-charges";
/// The belt exclusive prefix with two tiers.
const FLASKS: &str = "desecrated-prefix-chance-for-flasks-you-use-to-not-consume-charges";

const REVEAL: &str = "reveal-odds/test-reveal.toml";
const REVEAL_ECHOES: &str = "reveal-odds/test-reveal-echoes.toml";
const PREFIX_EXALT: &str = "reveal-odds/test-prefix-exalt.toml";
const DESECRATE: &str = "engine/test-desecrate.toml";
const RING: &str = "engine/ring-rare-mana-fire.txt";

/// The basis words every estimate carries.
const ALWAYS: [&str; 3] = ["estimate", "rests on", "reveal-exclusive-row"];

// --- helpers ----------------------------------------------------------------

fn test_engine() -> Engine {
    engine(&[REVEAL, REVEAL_ECHOES, PREFIX_EXALT, DESECRATE])
}

/// The first step of `blueprint`'s route on the item; panics when it is
/// rejected.
fn first_step(engine: &Engine, blueprint: &str, it: &Item, list: &[Target]) -> StepView {
    route(engine, blueprint, it, list).steps[0].clone()
}

fn route(engine: &Engine, blueprint: &str, it: &Item, list: &[Target]) -> Route {
    let plan = engine.plan(it, list, &[]).unwrap();
    if let Some(r) = plan.rejected.iter().find(|r| r.blueprint == blueprint) {
        panic!("{blueprint} is rejected: {}", r.reason);
    }
    plan.routes
        .into_iter()
        .find(|r| r.blueprint == blueprint)
        .unwrap_or_else(|| panic!("{blueprint} has no route"))
}

/// `(percent, basis)` of known odds, after checking that `hits` and `total`
/// agree with the percent within 0.01 percentage points and the basis is
/// not a weight count; panics on unknown odds.
fn estimate(odds: &Option<Odds>) -> (f64, String) {
    match odds {
        Some(Odds::Known {
            percent,
            hits,
            total,
            basis,
        }) => {
            assert!(*total > 0 && hits <= total, "{hits} of {total}");
            let ratio = *hits as f64 * 100.0 / *total as f64;
            assert!((ratio - percent).abs() <= 0.01, "{ratio} vs {percent}");
            assert!(
                !basis.starts_with(&format!("{hits} of {total}")),
                "the basis reads as a count: {basis}"
            );
            (*percent, basis.clone())
        }
        other => panic!("expected known odds, got {other:?}"),
    }
}

fn unknown(odds: &Option<Odds>) -> String {
    match odds {
        Some(Odds::Unknown { reason }) => reason.clone(),
        other => panic!("expected unknown odds, got {other:?}"),
    }
}

fn assert_words(basis: &str, present: &[&str], absent: &[&str]) {
    for w in ALWAYS.iter().chain(present) {
        assert!(basis.contains(w), "`{w}` missing from: {basis}");
    }
    for w in absent {
        assert!(!basis.contains(w), "`{w}` in: {basis}");
    }
}

fn close(got: f64, want: f64) {
    assert!((got - want).abs() < 1e-6, "got {got}, want {want}");
}

/// The item's data, variant and level.
fn data_of(it: &Item) -> (&'static ch_data::ClassData, &'static str, u32) {
    let (data, variant) = ch_data::variant_of(it).unwrap();
    (data, variant, it.item_level.as_ref().unwrap().value)
}

/// The families of the item's joined modifiers.
fn families(it: &Item) -> Vec<String> {
    ch_data::join(it)
        .mods
        .iter()
        .filter_map(|m| m.matched())
        .flat_map(|m| m.family.clone())
        .collect()
}

/// One Exalted Orb draw's chance on `side` of rolling `target` at `floor`
/// or better, read from the data: the spawn weights, the item's families
/// left out.
fn exalt_chance(it: &Item, target: &str, floor: u32, side: Kind) -> f64 {
    let (data, variant, level) = data_of(it);
    let pool = data.add_pool(variant, level, &[side], &families(it), None);
    pool.odds(target, floor)
        .percent()
        .expect("known exalt odds")
        / 100.0
}

/// The exclusive modifiers a reveal on `side` can offer, read from the
/// data: the class's desecrated ones at the item's level, the item's
/// families left out.
fn exclusive_pool(it: &Item, side: Kind) -> u32 {
    let (data, variant, level) = data_of(it);
    let on_item = families(it);
    data.valid_mods(variant, level)
        .desecrated
        .iter()
        .filter(|m| m.kind == side && !m.family.iter().any(|f| on_item.contains(f)))
        .count() as u32
}

/// The owner's belt targets (`tests/items/last-desecration/README.md`).
fn belt_targets() -> Vec<Target> {
    targets(&[
        (LIFE, Some(2)),
        (MANA, Some(2)),
        (ARMOUR, Some(2)),
        (FIRE, Some(3)),
    ])
}

// --- The parity file --------------------------------------------------------

/// Each case of the parity file whose recorded chance departs from
/// `chance(case)`, in words.
fn reveal_problems(cases: &[Value], chance: impl Fn(&Value) -> Option<f64>) -> Vec<String> {
    let mut problems = Vec::new();
    for c in cases {
        let want = c["chance"].as_f64();
        let got = chance(c);
        let same = match (got, want) {
            (None, None) => true,
            (Some(g), Some(w)) => (g - w).abs() < 1e-9,
            _ => false,
        };
        if !same {
            problems.push(format!(
                "{} / reveal_chance: recorded {want:?}, engine {got:?}",
                c["id"]
            ));
        }
    }
    problems
}

fn case_chance(c: &Value, equal_weight: bool) -> Option<f64> {
    reveal_chance_with(
        c["exclusive_pool"].as_u64().expect("an integer pool") as u32,
        c["exclusive_hits"].as_f64().expect("a number of hits"),
        c["ordinary"].as_f64(),
        c["echoes"].as_bool().expect("echoes true or false"),
        equal_weight,
    )
}

/// The engine's `reveal_chance` gives every recorded chance of
/// `reveal-odds.json` within 1e-9, with the equal-weight switch on as the
/// file was worked; a changed expectation is reported; with the switch off
/// an exclusive target's chance is unknown.
#[test]
fn reveal_chance_matches_the_model_cases() {
    let path = crate_dir().join("tests/items/slots/reveal-odds.json");
    let text = std::fs::read_to_string(&path).unwrap();
    let file: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(file["spec"], "docs/models/crafting-slots-spec.md");
    let cases = file["cases"].as_array().unwrap();
    assert!(cases.len() >= 8, "{} cases", cases.len());

    assert!(
        EXCLUSIVE_EQUAL_WEIGHT,
        "the parity file is worked with equal weights"
    );
    let problems = reveal_problems(cases, |c| {
        reveal_chance(
            c["exclusive_pool"].as_u64().unwrap() as u32,
            c["exclusive_hits"].as_f64().unwrap(),
            c["ordinary"].as_f64(),
            c["echoes"].as_bool().unwrap(),
        )
    });
    assert!(problems.is_empty(), "{problems:#?}");
    let problems = reveal_problems(cases, |c| case_chance(c, true));
    assert!(problems.is_empty(), "{problems:#?}");

    // Control: a recorded chance moved off the model is reported, and so is
    // a known chance recorded as unknown.
    let mut changed: Vec<Value> = cases.clone();
    changed[0]["chance"] = Value::from(changed[0]["chance"].as_f64().unwrap() + 0.001);
    let last = changed.len() - 1;
    changed[last]["chance"] = Value::Null;
    let problems = reveal_problems(&changed, |c| case_chance(c, true));
    assert_eq!(problems.len(), 2, "{problems:#?}");
    assert!(problems[0].contains(changed[0]["id"].as_str().unwrap()));

    // The switch off: every case with an aimed exclusive modifier is
    // unknown, every other case unchanged.
    for c in cases {
        let hits = c["exclusive_hits"].as_f64().unwrap();
        let off = case_chance(c, false);
        if hits > 0.0 {
            assert_eq!(off, None, "{}", c["id"]);
        } else {
            assert_eq!(off, case_chance(c, true), "{}", c["id"]);
        }
    }
}

// --- Baseline: what must survive --------------------------------------------

/// A desecration step's own odds stay unknown; `hardest` binds as before: a desecrated-pool
/// target first, else the hardest exalt; and the owner's belt still
/// desecrates for Armour on the prefixes before revealing.
#[test]
fn baseline_desecration_odds_stay_unknown() {
    let engine = test_engine();
    let ring = item(RING);

    let r = route(
        &engine,
        "test-desecrate",
        &ring,
        &targets(&[(LIFE, Some(8)), (CHAOS, Some(3))]),
    );
    assert_eq!(r.steps[0].goal, Goal::Desecrate);
    let reason = unknown(&r.steps[0].odds);
    assert!(reason.contains("desecrat"), "{reason}");
    assert_eq!(r.steps[0].target.as_deref(), Some(CHAOS));
    assert_eq!(r.steps[1].target.as_deref(), Some(CHAOS));

    let r = route(
        &engine,
        "test-desecrate",
        &ring,
        &targets(&[(LIFE, Some(8)), (SKILL_SPEED, None)]),
    );
    assert!(r.steps[0].odds.as_ref().unwrap().percent().is_none());
    assert_eq!(r.steps[0].target.as_deref(), Some(SKILL_SPEED));
    assert_eq!(r.steps[1].target.as_deref(), Some(SKILL_SPEED));

    let shipped = shipped();
    let belt = item("last-desecration/belt-2p1s.txt");
    let r = route(&shipped, "close-a-side", &belt, &belt_targets());
    let ids: Vec<&str> = r.steps.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(ids, ["desecrate-last", "reveal-last"]);
    assert_eq!(r.steps[0].target.as_deref(), Some(ARMOUR));
    assert!(r.steps[0].odds.as_ref().unwrap().percent().is_none());
}

// --- Targets ------------------------------------------------------------------

/// The owner's belt with its unrevealed prefix, Armour the ordinary target:
/// the first option is one of the belt's exclusive prefixes (none aimed),
/// the other two each hit with the prefix exalt's chance q, and Omen of
/// Abyssal Echoes rerolls once: 1 - ((1 - q)^2)^2. q is the chance the
/// engine gives a prefix exalt on the belt before its desecration.
#[test]
fn reveal_of_an_ordinary_target_is_two_exalt_draws_and_a_reroll() {
    let engine = test_engine();
    let unrevealed = item("last-desecration/belt-unrevealed.txt");
    let before = item("last-desecration/belt-2p1s.txt");

    let exalt = first_step(&engine, "test-prefix-exalt", &before, &belt_targets());
    let q = exalt.odds.as_ref().unwrap().percent().unwrap() / 100.0;
    close(q, exalt_chance(&before, ARMOUR, 2, Kind::Prefix));
    assert!(q > 0.0 && q < 1.0, "{q}");
    assert!(exclusive_pool(&unrevealed, Kind::Prefix) > 0);

    let step = first_step(&engine, "test-reveal-echoes", &unrevealed, &belt_targets());
    assert_eq!(step.targets, [ARMOUR]);
    let (percent, basis) = estimate(&step.odds);
    close(percent / 100.0, 1.0 - (1.0 - q).powi(4));
    assert_words(
        &basis,
        &["reveal-ordinary-exalt-weights", "Abyssal Echoes"],
        &["exclusive-weights-unknown", "necromancy-without-pool"],
    );
}

/// The belt with its unrevealed prefix, an exclusive prefix the target:
/// with equal weights it is the first option one time in n, n the belt's
/// exclusive prefixes at item level 82 (none shares a family with the
/// item's). The exclusive prefix with two tiers, aimed at T1, counts a half.
#[test]
fn reveal_of_an_exclusive_target_is_one_in_the_side_pool() {
    let engine = test_engine();
    let it = item("last-desecration/belt-unrevealed.txt");
    let n = exclusive_pool(&it, Kind::Prefix);
    assert_eq!(n, 8);

    let list = targets(&[
        (LIFE, Some(2)),
        (MANA, Some(2)),
        (CHARMS, None),
        (FIRE, Some(3)),
    ]);
    let step = first_step(&engine, "test-reveal", &it, &list);
    assert_eq!(step.targets, [CHARMS]);
    let (percent, basis) = estimate(&step.odds);
    close(percent / 100.0, 1.0 / n as f64);
    assert_words(
        &basis,
        &["exclusive-weights-unknown", "equal"],
        &[
            "reveal-ordinary-exalt-weights",
            "Abyssal Echoes",
            "necromancy-without-pool",
        ],
    );

    let list = targets(&[
        (LIFE, Some(2)),
        (MANA, Some(2)),
        (FLASKS, Some(1)),
        (FIRE, Some(3)),
    ]);
    let step = first_step(&engine, "test-reveal", &it, &list);
    let (percent, _) = estimate(&step.odds);
    close(percent / 100.0, 0.5 / n as f64);
}

/// A body armour has no exclusive prefix at any level: its unrevealed
/// prefix (Sinistral Necromancy's, by the wiki) reveals three exalt-like
/// draws over the ordinary prefixes, and the basis names
/// `necromancy-without-pool`.
#[test]
fn reveal_without_an_exclusive_pool_is_three_exalt_draws() {
    let engine = test_engine();
    let it = item("reveal-odds/body-armour-unrevealed-prefix.txt");
    assert_eq!(exclusive_pool(&it, Kind::Prefix), 0);
    let list = targets(&[
        (LIFE, Some(5)),
        (ARMOUR_PERCENT, Some(2)),
        (SPIRIT, Some(3)),
        (FIRE, Some(3)),
    ]);
    let q = exalt_chance(&it, SPIRIT, 3, Kind::Prefix);
    assert!(q > 0.0 && q < 1.0, "{q}");

    let step = first_step(&engine, "test-reveal", &it, &list);
    assert_eq!(step.targets, [SPIRIT]);
    let (percent, basis) = estimate(&step.odds);
    close(percent / 100.0, 1.0 - (1.0 - q).powi(3));
    assert_words(
        &basis,
        &["reveal-ordinary-exalt-weights", "necromancy-without-pool"],
        &["exclusive-weights-unknown", "Abyssal Echoes"],
    );
}

/// The owner's belt at item level 64 with an unrevealed prefix: below every
/// belt exclusive modifier's level 65, so three exalt-like draws; the class
/// has a pool at a higher level, so the basis does not name
/// `necromancy-without-pool`.
#[test]
fn reveal_below_the_pool_level_is_three_exalt_draws() {
    let engine = test_engine();
    let it = item("reveal-odds/belt-unrevealed-level-64.txt");
    assert_eq!(exclusive_pool(&it, Kind::Prefix), 0);
    let list = targets(&[
        (LIFE, Some(2)),
        (MANA, Some(2)),
        (ARMOUR, Some(4)),
        (FIRE, Some(3)),
    ]);
    let q = exalt_chance(&it, ARMOUR, 4, Kind::Prefix);
    assert!(q > 0.0 && q < 1.0, "{q}");

    let step = first_step(&engine, "test-reveal", &it, &list);
    let (percent, basis) = estimate(&step.odds);
    close(percent / 100.0, 1.0 - (1.0 - q).powi(3));
    assert_words(
        &basis,
        &["reveal-ordinary-exalt-weights", "64"],
        &[
            "necromancy-without-pool",
            "exclusive-weights-unknown",
            "Abyssal Echoes",
        ],
    );
}

/// A plain reveal has no reroll: its chance p is one set's, and the same
/// reveal with Omen of Abyssal Echoes is 1 - (1 - p)^2.
#[test]
fn reveal_without_echoes_has_no_reroll() {
    let engine = test_engine();
    let it = item("last-desecration/belt-unrevealed.txt");
    let q = exalt_chance(
        &item("last-desecration/belt-2p1s.txt"),
        ARMOUR,
        2,
        Kind::Prefix,
    );

    let plain = first_step(&engine, "test-reveal", &it, &belt_targets());
    let (p, basis) = estimate(&plain.odds);
    close(p / 100.0, 1.0 - (1.0 - q).powi(2));
    assert_words(
        &basis,
        &["reveal-ordinary-exalt-weights"],
        &["Abyssal Echoes"],
    );

    let echoes = first_step(&engine, "test-reveal-echoes", &it, &belt_targets());
    let (e, _) = estimate(&echoes.odds);
    close(e / 100.0, 1.0 - (1.0 - p / 100.0).powi(2));
}

/// A jewel's ordinary target: an Exalted Orb's odds are unknown on every
/// jewel page (no spawn weights), so the reveal's are, with that reason.
/// Control: a jewel's exclusive target needs no ordinary weight.
#[test]
fn reveal_with_no_spawn_weights_stays_unknown() {
    let engine = test_engine();
    let it = item("reveal-odds/jewel-unrevealed-prefix.txt");
    let step = first_step(
        &engine,
        "test-reveal",
        &it,
        &targets(&[(SPELL_DAMAGE, None)]),
    );
    let reason = unknown(&step.odds);
    assert!(reason.contains("spawn weights"), "{reason}");

    let (data, variant, level) = data_of(&it);
    let exclusive = data
        .valid_mods(variant, level)
        .desecrated
        .iter()
        .find(|m| m.kind == Kind::Prefix && m.tiers.len() == 1)
        .map(|m| m.id.to_string())
        .expect("a jewel exclusive prefix");
    let step = first_step(&engine, "test-reveal", &it, &targets(&[(&exclusive, None)]));
    let (percent, basis) = estimate(&step.odds);
    close(
        percent / 100.0,
        1.0 / exclusive_pool(&it, Kind::Prefix) as f64,
    );
    assert_words(
        &basis,
        &["exclusive-weights-unknown"],
        &["reveal-ordinary-exalt-weights"],
    );
}

/// A listed route evaluates its reveal on the starting item, before the
/// desecration: no unrevealed modifier, so the reveal takes the side its
/// aims are on (the ring's suffixes: one of its exclusive suffixes first,
/// then two suffix exalt draws, rerolled with Echoes). Control: aims on
/// both sides leave the side unknown.
#[test]
fn listed_reveal_before_its_desecration_takes_the_aims_side() {
    let engine = test_engine();
    let ring = item(RING);
    let list = targets(&[
        (MANA, Some(5)),
        (FIRE, Some(5)),
        (COLD, Some(5)),
        (CHAOS, Some(3)),
    ]);
    let r = route(&engine, "test-desecrate", &ring, &list);
    let reveal = &r.steps[1];
    assert_eq!(reveal.goal, Goal::Reveal);
    let target = reveal.target.clone().expect("a bound reveal");
    let floor = if target == COLD { 5 } else { 3 };
    assert!(exclusive_pool(&ring, Kind::Suffix) > 0);
    let q = exalt_chance(&ring, &target, floor, Kind::Suffix);
    let (percent, basis) = estimate(&reveal.odds);
    close(percent / 100.0, 1.0 - (1.0 - q).powi(4));
    assert_words(
        &basis,
        &["suffix", "reveal-ordinary-exalt-weights", "Abyssal Echoes"],
        &["exclusive-weights-unknown", "necromancy-without-pool"],
    );

    let both = targets(&[(MANA, Some(5)), (LIFE, Some(8)), (COLD, Some(5))]);
    let step = first_step(&engine, "test-reveal", &ring, &both);
    let reason = unknown(&step.odds);
    assert!(
        reason.contains("not known before the desecration"),
        "{reason}"
    );

    // The guide on the starting item gives the same estimate.
    let (_, first) = start(
        &engine,
        "test-reveal-echoes",
        &ring,
        &targets(&[(MANA, Some(5)), (COLD, Some(5))]),
        &[],
    );
    let q = exalt_chance(&ring, COLD, 5, Kind::Suffix);
    let (percent, _) = estimate(&first.odds);
    close(percent / 100.0, 1.0 - (1.0 - q).powi(4));
}
