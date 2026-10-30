//! Blocker advice ([`ch_engine::Engine::blocker_advice`]) over the shipped
//! blueprints and the constructed belt in `tests/items/blocker/` (see its
//! README): which desecrated modifier a reveal step tells the player to take
//! when the target is not offered, scored by the same-side exalt that
//! follows it (docs/models/crafting-slots-spec.md, "A wrong reveal waits
//! for the same-side exalt").

mod common;

use ch_engine::ch_blueprint::{Goal, Side};
use ch_engine::{BlockerAdvice, Target, BLOCKER_LIMIT};
use common::{item, shipped, targets};

const BELT: &str = "blocker/belt-magic-life.txt";
const FIRE: &str = "suffix-to-fire-resistance";
const COLD: &str = "suffix-to-cold-resistance";
const LIGHTNING: &str = "suffix-to-lightning-resistance";
const STRENGTH: &str = "suffix-to-strength";
const STR_DEX: &str = "desecrated-suffix-to-strength-and-dexterity";
const STR_INT: &str = "desecrated-suffix-to-strength-and-intelligence";

/// The relaxed list: Fire Resistance at T3 (Greater Essence of
/// Insulation's tier), Cold and Lightning Resistance at T2.
fn relaxed() -> Vec<Target> {
    targets(&[(FIRE, Some(3)), (COLD, Some(2)), (LIGHTNING, Some(2))])
}

fn advice(blueprint: &str) -> Option<BlockerAdvice> {
    shipped()
        .blocker_advice(&item(BELT), &relaxed(), &[], blueprint)
        .unwrap()
}

fn ids(mods: &[ch_engine::BlockerMod]) -> Vec<&str> {
    mods.iter().map(|m| m.mod_id.as_str()).collect()
}

#[test]
fn blocker_advice_ranks_the_strength_blockers() {
    let engine = shipped();
    let belt = item(BELT);
    let plan = engine.plan(&belt, &relaxed(), &[]).unwrap();
    let route = plan
        .routes
        .iter()
        .find(|r| r.blueprint == "essence-desecrate-double-slam")
        .expect("the double slam routes the relaxed belt");
    // The route this advice is for: the essence, a suffix desecration, its
    // reveal, then a suffix exalt.
    let goals: Vec<Goal> = route.steps.iter().map(|s| s.goal).collect();
    assert_eq!(
        goals,
        [
            Goal::AddWanted,
            Goal::Desecrate,
            Goal::Reveal,
            Goal::AddWanted
        ],
        "{:?}",
        route.steps
    );

    let advice = advice("essence-desecrate-double-slam").expect("advice at the reveal");
    assert_eq!(advice.step, "reveal");
    assert_eq!(advice.position, 2);
    assert_eq!(advice.side, Side::Suffix);
    assert_eq!(advice.exalt_step, "side-slam");
    assert_eq!(advice.exalt_position, 3);
    assert_eq!(
        advice.target.as_ref().map(|t| t.mod_id.as_str()),
        route.steps[1].target.as_deref()
    );
    assert_eq!(
        route.steps[3].action.as_ref().map(|a| a.uses.as_str()),
        Some("perfect-exalted-orb")
    );

    // Both Strength-sharing desecrated suffixes, named together (the same
    // chance), keeping the ordinary Strength suffix out; nothing else.
    assert_eq!(advice.picks.len(), 1, "{:?}", advice.picks);
    let pick = &advice.picks[0];
    assert_eq!(ids(&pick.mods), [STR_DEX, STR_INT]);
    assert_eq!(ids(&pick.keeps_out), [STRENGTH]);
    assert_eq!(pick.keeps_out[0].lines, ["+# to Strength"]);
    let before = advice.before.percent().expect("a known chance before");
    let after = pick.after.percent().expect("a known chance after");
    assert!(after > before, "after {after} is not above before {before}");
    assert!(before > 0.0);
    let total = |o: &ch_engine::Odds| match o {
        ch_engine::Odds::Known { total, .. } => *total,
        other => panic!("not known: {other:?}"),
    };
    // Strength's tiers leave the pool, so the total weight drops and the
    // targets' weight stays.
    assert!(total(&pick.after) < total(&advice.before));
    let count: usize = advice.picks.iter().map(|p| p.mods.len()).sum();
    assert!(count <= BLOCKER_LIMIT);
}

#[test]
fn no_blocker_advice_for_a_last_desecration() {
    let engine = shipped();
    let belt = item(BELT);
    let plan = engine.plan(&belt, &relaxed(), &[]).unwrap();
    let route = plan
        .routes
        .iter()
        .find(|r| r.blueprint == "magic-augment-essence")
        .expect("magic-augment-essence routes the relaxed belt");
    // Its desecration is the route's last add: nothing follows the reveal.
    let goals: Vec<Goal> = route.steps.iter().map(|s| s.goal).collect();
    let reveal = goals
        .iter()
        .position(|g| *g == Goal::Reveal)
        .expect("a reveal");
    assert_eq!(reveal, goals.len() - 1, "{goals:?}");
    assert_eq!(goals[reveal - 1], Goal::Desecrate);
    assert_eq!(advice("magic-augment-essence"), None);

    // Neither a listed-order route nor an unknown blueprint gets advice.
    assert_eq!(advice("no-such-blueprint"), None);
    let listed = plan
        .routes
        .iter()
        .find(|r| r.order == ch_engine::ch_blueprint::Order::Listed)
        .map(|r| r.blueprint.clone());
    if let Some(listed) = listed {
        assert_eq!(advice(&listed), None, "{listed}");
    }
}
