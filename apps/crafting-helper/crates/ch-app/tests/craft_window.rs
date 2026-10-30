//! The craft window through `App`, with a recording fake shell and the
//! shipped engine: a grab becomes the planning item (or is refused with the
//! reason), the targets valid for its base, their floors, the blocked list
//! of a craft, the route preview with its odds and fallbacks in words, and
//! the guide over `ch-engine`'s whittled ring, whose states are a known hit
//! and miss walk (`tests/seeds_gear.rs`). Then the one address the app
//! opens.

mod common;

use ch_app::ch_data::Odds;
use ch_app::ch_engine::{catalogue, Engine, Target};
use ch_app::craft::odds_words;
use ch_app::{
    BlockedFrom, CraftMode, GuideStatusView, ModSide, OutcomeWord, StatusKind, View,
    CRAFT_OF_EXILE_URL,
};
use common::*;
use serde_json::json;

const FIRE: &str = "suffix-to-fire-resistance";
const COLD: &str = "suffix-to-cold-resistance";
const FIRE_TEXT: &str = "+#% to Fire Resistance";
const COLD_TEXT: &str = "+#% to Cold Resistance";
const LIGHTNING_TEXT: &str = "+#% to Lightning Resistance";

/// The whittling route's first step on the ring's start, composed from its
/// data (the blueprint says "Chaos Orb until one of the targets lands").
const CHAOS_STEP: &str = "Chaos Orb for +#% to Fire Resistance (suffix), +#% to Cold Resistance (suffix) or +#% to Lightning Resistance (suffix)";

fn planning(view: &View) -> &ch_app::PlanningView {
    assert_eq!(view.craft.mode, CraftMode::Planning, "{:?}", view.status);
    view.craft.planning.as_ref().expect("the planning part")
}

fn guiding(view: &View) -> &ch_app::GuidingView {
    assert_eq!(view.craft.mode, CraftMode::Guiding, "{:?}", view.status);
    view.craft.guiding.as_ref().expect("the guiding part")
}

fn status(view: &View) -> (StatusKind, String) {
    let s = view.status.clone().expect("a status line");
    (s.kind, s.text)
}

fn craft_lines(dir: &std::path::Path) -> Vec<String> {
    diag_lines(dir)
        .into_iter()
        .filter(|l| l.starts_with("craft:") || l.contains("Craft of Exile"))
        .collect()
}

/// A started app planning the ring's start with its three targets on the
/// whittling route.
fn whittle_planned(dir: &std::path::Path, shell: &FakeShell) -> std::sync::Arc<ch_app::App> {
    let app = start(dir, shell);
    app.grab_text(Some(&ring("01-start")));
    app.set_targets(&ring_targets());
    let view = app.choose_route(WHITTLE);
    let route = planning(&view)
        .preview
        .as_ref()
        .unwrap()
        .route
        .as_ref()
        .unwrap();
    assert_eq!(route.blueprint, WHITTLE);
    app
}

#[test]
fn grabbed_item_offers_valid_targets_by_side() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);

    // Baseline: nothing grabbed is mode none, with no part.
    let view = app.get_view();
    assert_eq!(view.craft.mode, CraftMode::None);
    assert!(view.craft.planning.is_none() && view.craft.guiding.is_none());
    assert!(view.craft.refusal.is_none());

    // Target: the ring is the planning item.
    let view = app.grab_text(Some(&ring("01-start")));
    assert_eq!(status(&view).1, "Grabbed an item: 16 lines.");
    let p = planning(&view);
    assert_eq!(p.item.name.as_deref(), Some("Doom Loop"));
    assert_eq!(p.item.base_type.as_deref(), Some("Amethyst Ring"));
    assert_eq!(
        (
            p.item.class.as_str(),
            p.item.rarity.as_str(),
            p.item.item_level
        ),
        ("Rings", "Rare", Some(76))
    );
    let sides: Vec<(ModSide, Option<u32>)> =
        p.item.modifiers.iter().map(|m| (m.side, m.tier)).collect();
    assert_eq!(
        sides,
        [(ModSide::Prefix, Some(5)), (ModSide::Suffix, Some(5))]
    );
    assert_eq!(p.item.modifiers[0].lines, ["+95(90-104) to maximum Mana"]);
    // Its own modifiers start ticked as targets, each met at its tier.
    assert!(p.item.modifiers.iter().all(|m| m.meets_target));

    // The choices are what the base rolls at item level 76, by side and
    // group, each with the default floor first and one per reachable tier.
    let item = ch_app::ch_item::parse(&ring("01-start")).unwrap();
    let (data, variant) = ch_app::ch_data::variant_of(&item).unwrap();
    let valid = data.valid_mods(variant, 76);
    assert_eq!(
        p.choices.prefixes.len() + p.choices.suffixes.len(),
        valid.normal.len()
    );
    assert_eq!(p.choices.desecrated.len(), valid.desecrated.len());
    assert!(!p.choices.prefixes.is_empty() && !p.choices.suffixes.is_empty());
    assert!(p.choices.prefixes.iter().all(|c| c.side == ModSide::Prefix));
    assert!(p.choices.suffixes.iter().all(|c| c.side == ModSide::Suffix));
    for c in p
        .choices
        .prefixes
        .iter()
        .chain(&p.choices.suffixes)
        .chain(&p.choices.desecrated)
    {
        let m = valid
            .normal
            .iter()
            .chain(&valid.desecrated)
            .find(|m| m.id == c.mod_id)
            .unwrap_or_else(|| panic!("{} is not valid for the ring", c.mod_id));
        assert_eq!(c.best_tier, m.best_tier);
        assert_eq!(c.floors.len(), m.tiers.len() + 1, "{}", c.mod_id);
        assert_eq!(c.floors[0].floor, None);
        assert_eq!(
            c.floors[0].label,
            format!("As high as possible (T{})", m.best_tier)
        );
        assert_eq!(c.floors[1].floor, Some(m.best_tier));
        assert_eq!(c.floors[1].label, format!("At least T{}", m.best_tier));
    }
    let fire = p
        .choices
        .suffixes
        .iter()
        .find(|c| c.mod_id == FIRE)
        .unwrap();
    assert_eq!(fire.text, "+#% to Fire Resistance");
    // Negative control: a modifier rings never roll is not offered.
    assert!(p
        .choices
        .prefixes
        .iter()
        .chain(&p.choices.suffixes)
        .all(|c| !c.mod_id.contains("runic-ward")));
    // The targets are the item's own two modifiers; cleared, there is no
    // preview and nothing to accept.
    assert_eq!(p.targets.len(), 2);
    let view = app.set_targets(&[]);
    let p = planning(&view);
    assert!(p.targets.is_empty() && p.preview.is_none() && !p.can_accept);
    app.shutdown();
}

/// The item's own explicit modifiers as targets, each at the tier it has.
fn own_targets(text: &str) -> Vec<(String, Option<u32>)> {
    let item = ch_app::ch_item::parse(text).unwrap();
    ch_app::ch_data::join(&item)
        .mods
        .iter()
        .filter_map(|jm| jm.matched())
        .map(|m| (m.mod_id.clone(), Some(m.tier)))
        .collect()
}

fn chosen(p: &ch_app::PlanningView) -> Vec<(String, Option<u32>)> {
    p.targets
        .iter()
        .map(|t| (t.mod_id.clone(), t.floor))
        .collect()
}

#[test]
fn a_grab_ticks_the_items_own_mods_as_targets() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);

    // Mana at T5 and fire resistance at T3: both ticked, each at its tier,
    // so each starts met.
    let view = app.grab_text(Some(&ring("02-chaos-fire")));
    let p = planning(&view);
    let own = own_targets(&ring("02-chaos-fire"));
    assert_eq!(own.len(), 2);
    assert_eq!(chosen(p), own);
    assert!(own.contains(&(FIRE.to_string(), Some(3))));
    for t in &p.targets {
        assert_eq!(Some(t.floor_in_force), t.floor, "{}", t.mod_id);
    }
    assert!(p.item.modifiers.iter().all(|m| m.meets_target));
    let fire = p.targets.iter().find(|t| t.mod_id == FIRE).unwrap();
    assert_eq!(fire.text, "+#% to Fire Resistance");
    // Nothing was dropped: the status line is the grab's alone.
    assert_eq!(
        status(&view),
        (StatusKind::Info, "Grabbed an item: 16 lines.".to_string())
    );

    // Negative control: an unrevealed desecrated modifier has no data
    // modifier yet, so it is not ticked; the three it has are.
    let view = app.grab_text(Some(&ring("04-desecrated")));
    let p = planning(&view);
    let ids: Vec<&str> = p.targets.iter().map(|t| t.mod_id.as_str()).collect();
    assert!(ids.contains(&FIRE) && ids.contains(&COLD), "{ids:?}");
    assert!(p
        .item
        .modifiers
        .iter()
        .filter(|m| m.unrevealed)
        .all(|m| !m.meets_target));

    // A modifier whose tier the item level does not reach is left out,
    // silently: the same ring at item level 1 starts with none ticked.
    let low = ring("01-start").replace("Item Level: 76", "Item Level: 1");
    app.set_targets(&[]);
    let view = app.grab_text(Some(&low));
    let p = planning(&view);
    assert!(p.targets.is_empty(), "{:?}", chosen(p));
    assert!(!status(&view).1.contains("Dropped"), "{:?}", view.status);
    app.shutdown();
}

#[test]
fn a_grab_of_an_item_with_no_explicit_mods_ticks_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);
    // The ring's start as a Normal item: its implicit only.
    let normal = "Item Class: Rings\nRarity: Normal\nAmethyst Ring\n--------\nRequires: Level 20\n--------\nItem Level: 76\n--------\n{ Implicit Modifier — Chaos, Resistance }\n+7(7-13)% to Chaos Resistance\n";
    let view = app.grab_text(Some(normal));
    let p = planning(&view);
    assert!(p.item.modifiers.is_empty());
    assert!(p.targets.is_empty() && p.preview.is_none() && !p.can_accept);
    // Positive control: the same base with explicit modifiers ticks them.
    let view = app.grab_text(Some(&ring("01-start")));
    assert_eq!(planning(&view).targets.len(), 2);
    app.shutdown();
}

#[test]
fn a_regrab_while_planning_adds_the_new_items_own_mods() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);
    app.grab_text(Some(&ring("01-start")));
    // The player's own list: fire resistance at T2 or better.
    app.set_targets(&[Target::new(FIRE, Some(2))]);
    // The next item has fire and cold resistance at T3: the chosen fire
    // target stays first, with its own floor, and the item's other
    // modifier follows.
    let view = app.grab_text(Some(&ring("03-whittle-cold")));
    let p = planning(&view);
    let mut want = vec![(FIRE.to_string(), Some(2))];
    want.extend(
        own_targets(&ring("03-whittle-cold"))
            .into_iter()
            .filter(|(id, _)| id != FIRE),
    );
    assert_eq!(chosen(p), want);
    assert!(want.contains(&(COLD.to_string(), Some(3))));
    // Fire at T3 does not meet its T2 floor; the others do.
    let fire = p
        .item
        .modifiers
        .iter()
        .find(|m| m.lines[0].contains("Fire Resistance"))
        .unwrap();
    assert!(!fire.meets_target);
    assert!(p
        .item
        .modifiers
        .iter()
        .filter(|m| !m.lines[0].contains("Fire Resistance"))
        .all(|m| m.meets_target));
    // Unticking still replaces the whole list.
    let view = app.set_targets(&[Target::new(COLD, None)]);
    assert_eq!(chosen(planning(&view)), [(COLD.to_string(), None)]);
    app.shutdown();
}

#[test]
fn no_floor_means_the_best_tier() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);
    // Fire resistance at T3 on the ring.
    app.grab_text(Some(&ring("02-chaos-fire")));
    let view = app.set_targets(&[Target::new(FIRE, None)]);
    let p = planning(&view);
    let best = p
        .choices
        .suffixes
        .iter()
        .find(|c| c.mod_id == FIRE)
        .unwrap()
        .best_tier;
    assert!(best < 3, "the ring's level reaches better than T3");
    assert_eq!(p.targets.len(), 1);
    assert_eq!(p.targets[0].floor, None);
    assert_eq!(p.targets[0].floor_in_force, best);
    assert_eq!(p.targets[0].best_tier, best);
    // T3 is below the best tier, so it does not meet the target...
    let fire_mod = |p: &ch_app::PlanningView| {
        p.item
            .modifiers
            .iter()
            .find(|m| m.lines[0].contains("Fire Resistance"))
            .unwrap()
            .meets_target
    };
    assert!(!fire_mod(p));
    // ...and at least T3 it does.
    let view = app.set_targets(&[Target::new(FIRE, Some(3))]);
    let p = planning(&view);
    assert_eq!(
        (p.targets[0].floor, p.targets[0].floor_in_force),
        (Some(3), 3)
    );
    assert!(fire_mod(p));
    app.shutdown();
}

#[test]
fn invalid_targets_are_refused_whole() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);

    // Before any item, targets are refused.
    let view = app.set_targets(&[Target::new(FIRE, None)]);
    assert_eq!(view.craft.mode, CraftMode::None);
    assert_eq!(
        status(&view),
        (
            StatusKind::Error,
            "Targets not changed: no item is being planned; grab one first.".to_string()
        )
    );

    app.grab_text(Some(&ring("01-start")));
    let view = app.set_targets(&[Target::new(FIRE, Some(3))]);
    assert_eq!(planning(&view).targets.len(), 1);
    assert_eq!(
        status(&view),
        (StatusKind::Info, "1 target chosen.".to_string())
    );

    // An id the ring cannot roll refuses the whole list: fire stays alone.
    let view = app.set_targets(&[Target::new(COLD, None), Target::new("not-a-modifier", None)]);
    let p = planning(&view);
    assert_eq!(p.targets.len(), 1);
    assert_eq!(p.targets[0].mod_id, FIRE);
    let (kind, text) = status(&view);
    assert_eq!(kind, StatusKind::Error);
    assert_eq!(
        text,
        "Targets not changed: `not-a-modifier` is not a modifier this item can roll."
    );
    // A floor outside the tiers the item level reaches.
    let view = app.set_targets(&[Target::new(COLD, Some(99))]);
    assert_eq!(planning(&view).targets[0].mod_id, FIRE);
    assert!(
        status(&view).1.contains("T99 is not a tier"),
        "{:?}",
        view.status
    );
    // Repeats are dropped, the first kept.
    let view = app.set_targets(&[Target::new(FIRE, None), Target::new(FIRE, Some(3))]);
    let p = planning(&view);
    assert_eq!(p.targets.len(), 1);
    assert_eq!(p.targets[0].floor, None);
    // An empty list clears them.
    let view = app.set_targets(&[]);
    assert!(planning(&view).targets.is_empty());
    assert!(planning(&view).preview.is_none());

    app.shutdown();
    let log = craft_lines(tmp.path());
    assert!(log.contains(&"craft: 1 target".to_string()), "{log:?}");
    assert!(log.contains(&"craft: 0 targets".to_string()), "{log:?}");
    assert!(
        log.contains(
            &"craft: refused: `not-a-modifier` is not a modifier this item can roll".to_string()
        ),
        "{log:?}"
    );
    // No item text reaches the log.
    let all = diag_lines(tmp.path()).join("\n");
    assert!(
        !all.contains("Doom Loop") && !all.contains("Amethyst"),
        "{all}"
    );
}

#[test]
fn targets_plan_a_route_with_odds_in_words() {
    // The odds' forms.
    let known = |h, t| Odds::known(h, t, "weight");
    assert_eq!(
        odds_words(&known(8000, 69500)),
        "~12% (8000 of 69500 weight)"
    );
    assert_eq!(
        odds_words(&known(450, 10000)),
        "~4.5% (450 of 10000 weight)"
    );
    assert_eq!(odds_words(&known(5, 1000)), "under 1% (5 of 1000 weight)");
    assert_eq!(
        odds_words(&Odds::certain(
            true,
            "1 of 1 removable modifiers are not wanted"
        )),
        "certain (1 of 1 removable modifiers are not wanted)"
    );
    assert_eq!(
        odds_words(&Odds::unknown("no weights")),
        "odds unknown: no weights"
    );

    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);
    app.grab_text(Some(&ring("01-start")));
    let view = app.set_targets(&ring_targets());
    let p = planning(&view);
    assert_eq!(status(&view).1, "3 targets chosen.");
    let preview = p.preview.as_ref().expect("a preview with targets");
    let route = preview.route.as_ref().expect("a route for the ring");
    // The engine's first route, and the other one to choose.
    let engine = Engine::shipped().unwrap();
    let ring_item = ch_app::ch_item::parse(&ring("01-start")).unwrap();
    let plan = engine.plan(&ring_item, &ring_targets(), &[]).unwrap();
    assert_eq!(route.blueprint, plan.routes[0].blueprint);
    assert!(preview.other_routes.iter().any(|r| r.blueprint == WHITTLE));
    assert!(preview.rejected.is_empty());
    assert!(p.can_accept);

    // The whittling route: chaos, whittle, desecrate, reveal, with known
    // odds for the orbs and unknown odds, in words, for the desecration.
    let view = app.choose_route(WHITTLE);
    // The status line names no blueprint (the diag line names its id).
    assert_eq!(status(&view).1, "Route chosen.");
    let preview = planning(&view).preview.as_ref().unwrap();
    let route = preview.route.as_ref().unwrap();
    assert_eq!(route.name, "Rings: resistances by whittling");
    assert!(route.source.contains("Discord post 10"));
    // The blueprint's notes are not the route's (the route panel shows the
    // targets and the steps instead).
    assert!(serde_json::to_value(route).unwrap().get("notes").is_none());
    let ids: Vec<&str> = route.steps.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(ids, ["chaos", "whittle", "desecrate", "reveal"]);
    let numbers: Vec<usize> = route.steps.iter().map(|s| s.number).collect();
    assert_eq!(numbers, [1, 2, 3, 4]);
    assert_eq!(route.steps[0].instruction, CHAOS_STEP);
    assert_eq!(route.steps[0].uses, ["Chaos Orb"]);
    assert_eq!(route.steps[1].uses, ["Chaos Orb", "Omen of Whittling"]);
    let first = route.steps[0].odds.as_deref().unwrap();
    assert!(
        first.starts_with('~') && first.contains("% (") && first.contains("weight"),
        "{first}"
    );
    let desecrate = route.steps[2].odds.as_deref().unwrap();
    assert!(desecrate.starts_with("odds unknown: "), "{desecrate}");
    assert!(route.steps.iter().all(|s| s.fallback.is_none()));
    assert!(preview.other_routes.iter().all(|r| r.blueprint != WHITTLE));
    // A route the item has not is refused.
    let view = app.choose_route("jewels-three-one-side");
    assert_eq!(
        planning(&view)
            .preview
            .as_ref()
            .unwrap()
            .route
            .as_ref()
            .unwrap()
            .blueprint,
        WHITTLE
    );
    assert_eq!(status(&view).0, StatusKind::Error);
    app.shutdown();
    let log = craft_lines(tmp.path());
    assert!(log.contains(&format!("craft: route {WHITTLE}")), "{log:?}");
}

#[test]
fn choosing_a_route_previews_it_baseline() {
    // Today's behaviour, kept: choosing the whittling route on the ring
    // previews it.
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);
    app.grab_text(Some(&ring("01-start")));
    app.set_targets(&ring_targets());
    let view = app.choose_route(WHITTLE);
    let preview = planning(&view).preview.as_ref().unwrap();
    assert_eq!(preview.route.as_ref().unwrap().blueprint, WHITTLE);
}

/// Each route the preview lists, as (number, blueprint).
fn route_numbers(view: &View) -> Vec<(usize, String)> {
    planning(view)
        .preview
        .as_ref()
        .unwrap()
        .routes
        .iter()
        .map(|r| (r.number, r.blueprint.clone()))
        .collect()
}

/// The previewed route's number and blueprint.
fn previewed(view: &View) -> (Option<usize>, String) {
    let preview = planning(view).preview.as_ref().unwrap();
    (
        preview.route_number,
        preview.route.as_ref().unwrap().blueprint.clone(),
    )
}

#[test]
fn route_numbers_stay_with_their_routes() {
    // Every applicable route is listed with a number from 1, in the
    // engine's order, and choosing one moves only which is previewed: the
    // list, its order and its numbers stay as they were.
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);
    app.grab_text(Some(&ring("01-start")));
    let view = app.set_targets(&ring_targets());
    let engine = Engine::shipped().unwrap();
    let ring_item = ch_app::ch_item::parse(&ring("01-start")).unwrap();
    let plan = engine.plan(&ring_item, &ring_targets(), &[]).unwrap();
    let expected: Vec<(usize, String)> = plan
        .routes
        .iter()
        .enumerate()
        .map(|(i, r)| (i + 1, r.blueprint.clone()))
        .collect();
    assert!(expected.len() >= 2, "{expected:?}");
    assert_eq!(route_numbers(&view), expected);
    assert_eq!(previewed(&view), (Some(1), expected[0].1.clone()));

    // Route 2 is previewed as route 2; the others keep their numbers.
    let view = app.choose_route(&expected[1].1);
    assert_eq!(route_numbers(&view), expected);
    assert_eq!(previewed(&view), (Some(2), expected[1].1.clone()));
    let others: Vec<(usize, String)> = planning(&view)
        .preview
        .as_ref()
        .unwrap()
        .other_routes
        .iter()
        .map(|r| (r.number, r.blueprint.clone()))
        .collect();
    let mut without_two = expected.clone();
    without_two.remove(1);
    assert_eq!(others, without_two);

    // Route 2 again changes nothing.
    let again = app.choose_route(&expected[1].1);
    assert_eq!(
        planning(&again).preview,
        planning(&view).preview,
        "choosing route 2 twice"
    );

    // Route 1 is previewed as route 1.
    let view = app.choose_route(&expected[0].1);
    assert_eq!(route_numbers(&view), expected);
    assert_eq!(previewed(&view), (Some(1), expected[0].1.clone()));
}

/// A ring target's text, as the planning list writes it.
fn ring_text(id: &str) -> &'static str {
    match id {
        FIRE => FIRE_TEXT,
        COLD => COLD_TEXT,
        "suffix-to-lightning-resistance" => LIGHTNING_TEXT,
        other => panic!("`{other}` is not a ring target"),
    }
}

/// The engine's whittling route on the ring's start, and the item.
fn whittle_route() -> (ch_app::ch_engine::Route, ch_app::ch_item::Item) {
    let engine = Engine::shipped().unwrap();
    let item = ch_app::ch_item::parse(&ring("01-start")).unwrap();
    let plan = engine.plan(&item, &ring_targets(), &[]).unwrap();
    let route = plan
        .routes
        .into_iter()
        .find(|r| r.blueprint == WHITTLE)
        .unwrap();
    (route, item)
}

#[test]
fn each_step_names_the_mods_it_targets() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = whittle_planned(tmp.path(), &shell);
    let view = app.get_view();
    let p = planning(&view);
    let route = p.preview.as_ref().unwrap().route.as_ref().unwrap();
    // Each step line carries the texts of the modifiers the engine says a
    // hit of it may produce, as the planning list writes them, and its
    // instruction names each of them.
    let (engine_route, _) = whittle_route();
    assert_eq!(route.steps.len(), engine_route.steps.len());
    for (line, step) in route.steps.iter().zip(&engine_route.steps) {
        assert_eq!(line.id, step.id);
        let want: Vec<&str> = step.targets.iter().map(|id| ring_text(id)).collect();
        assert!(!want.is_empty(), "{}", line.id);
        assert_eq!(line.targets, want, "{}", line.id);
        for t in &line.targets {
            assert!(
                line.instruction.contains(t.as_str()),
                "{}",
                line.instruction
            );
            assert!(p.targets.iter().any(|c| c.text == *t), "{t}");
        }
    }
    assert_eq!(
        route.steps[0].targets,
        [FIRE_TEXT, COLD_TEXT, LIGHTNING_TEXT]
    );

    // Guiding: the current step names what is still missing. Fire landed,
    // so the whittle step is for cold and lightning.
    app.accept_craft();
    let view = app.grab_text(Some(&ring("02-chaos-fire")));
    let g = guiding(&view);
    let step = &g.step.as_ref().unwrap().step;
    assert_eq!(step.id, "whittle");
    assert_eq!(step.targets, [COLD_TEXT, LIGHTNING_TEXT]);
    assert_eq!(
        step.instruction,
        "Chaos Orb with Omen of Whittling for +#% to Cold Resistance (suffix) or +#% to Lightning Resistance (suffix)"
    );
    // The history entry says the chaos step as it was shown.
    assert_eq!(g.history[0].instruction, CHAOS_STEP);
    app.shutdown();
}

#[test]
fn a_targeted_step_says_its_currency_side_and_mods() {
    use ch_app::ch_engine::ch_blueprint::Side;
    use ch_app::craft::step_instruction;
    let (route, item) = whittle_route();

    // Not tied to a side: the currency, its omen, then each target with its
    // side. Never the blueprint's text.
    let whittle = route.steps[1].clone();
    assert_eq!(whittle.id, "whittle");
    assert_eq!(whittle.side, None);
    let said = step_instruction(&whittle, &item, &ring_targets());
    assert_eq!(
        said,
        "Chaos Orb with Omen of Whittling for +#% to Fire Resistance (suffix), +#% to Cold Resistance (suffix) or +#% to Lightning Resistance (suffix)"
    );
    assert!(!said.contains(&whittle.say));

    // Tied to a side: the side once, by name.
    let mut tied = whittle.clone();
    tied.side = Some(Side::Suffix);
    assert_eq!(
        step_instruction(&tied, &item, &ring_targets()),
        "Chaos Orb with Omen of Whittling on the suffix side for +#% to Fire Resistance, +#% to Cold Resistance or +#% to Lightning Resistance"
    );

    // One target left (the desecration's last, once two have landed): the
    // Necromancy omen of its side, the bone it resolved to, the side, then
    // that target.
    let last = vec!["suffix-to-lightning-resistance".to_string()];
    let mut desecrate = route.steps[2].clone();
    assert_eq!(desecrate.id, "desecrate");
    desecrate.targets = last.clone();
    let bone = desecrate.action.as_ref().unwrap().name.clone();
    assert_eq!(
        step_instruction(&desecrate, &item, &ring_targets()),
        format!("Use Omen of Dextral Necromancy, then apply the {bone}; the desecrated modifier will be a suffix, for +#% to Lightning Resistance")
    );
    // The reveal names what it reveals into.
    let mut reveal = route.steps[3].clone();
    reveal.targets = last;
    assert_eq!(
        step_instruction(&reveal, &item, &ring_targets()),
        "Reveal at the Well of Souls for +#% to Lightning Resistance (suffix)"
    );
}

#[test]
fn a_step_without_targets_keeps_the_blueprint_text() {
    use ch_app::ch_engine::ch_blueprint::Goal;
    use ch_app::craft::{step_instruction, step_targets};
    let (route, item) = whittle_route();
    let chaos = &route.steps[0];
    assert_eq!(chaos.say, "Chaos Orb until one of the targets lands");
    // Positive control: with its targets, the step is composed.
    let targets = ring_targets();
    assert_eq!(step_instruction(chaos, &item, &targets), CHAOS_STEP);

    // No target left for it: the blueprint's text, and no target line.
    let mut none = chaos.clone();
    none.targets.clear();
    assert_eq!(step_instruction(&none, &item, &targets), chaos.say);
    assert!(step_targets(&none, &item).is_empty());
    // A removal step's targets are only what its binding names: a hit takes
    // a modifier off and places none, so it has no target line. Its text is
    // composed too: the currency and the unwanted modifiers it may take
    // (both of the start's, neither a target), never the blueprint's.
    let mut removal = chaos.clone();
    removal.goal = Goal::RemoveUnwanted;
    let said = step_instruction(&removal, &item, &targets);
    assert!(said.starts_with("Chaos Orb to remove "), "{said}");
    assert!(said.contains("+95(90-104) to maximum Mana"), "{said}");
    assert!(
        said.ends_with("; it takes one modifier at random"),
        "{said}"
    );
    assert!(!said.contains(&chaos.say), "{said}");
    assert!(step_targets(&removal, &item).is_empty());
    // A manual step has no currency to name.
    let mut manual = chaos.clone();
    manual.goal = Goal::Manual;
    manual.action = None;
    assert_eq!(step_instruction(&manual, &item, &targets), chaos.say);
}

/// The Necromancy omens, by the side each makes the desecrated modifier.
const NECROMANCY: [(&str, &str, &str); 2] = [
    (
        "omen-of-sinistral-necromancy",
        "Omen of Sinistral Necromancy",
        "prefix",
    ),
    (
        "omen-of-dextral-necromancy",
        "Omen of Dextral Necromancy",
        "suffix",
    ),
];

/// The magic Heavy Belt `ch-engine`'s relaxation tests read
/// (`crates/ch-engine/tests/items/relax/belt-magic-life.txt`, constructed):
/// maximum Life T2, no suffix.
const MAGIC_BELT: &str = "Item Class: Belts\nRarity: Magic\nAthlete's Heavy Belt\n--------\nRequires: Level 40\n--------\nItem Level: 82\n--------\n{ Implicit Modifier }\nHas 2 Charm Slots \u{2014} Unscalable Value\n--------\n{ Prefix Modifier \"Athlete's\" (Tier: 2) \u{2014} Life }\n+130(120-149) to maximum Life\n";

/// Fire Resistance at floor T3, Cold and Lightning Resistance at T2: the
/// magic belt's list that routes on its Greater Essence of Insulation.
fn magic_belt_targets() -> Vec<Target> {
    [
        ("suffix-to-fire-resistance", 3),
        ("suffix-to-cold-resistance", 2),
        ("suffix-to-lightning-resistance", 2),
    ]
    .iter()
    .map(|(id, floor)| Target::new(*id, Some(*floor)))
    .collect()
}

#[test]
fn an_unsided_desecration_names_no_omen_baseline() {
    use ch_app::ch_engine::ch_blueprint::Goal;
    use ch_app::craft::step_instruction;
    // `caster-cast-speed`'s `block` step has no side and no binding: its
    // bone carries no omen, and its instruction names none.
    let engine = Engine::shipped().unwrap();
    let path = app_root()
        .join("crates/ch-engine/tests/items/jewels-weapons/caster-cast-speed/0-start.txt");
    let text = std::fs::read_to_string(&path)
        .unwrap()
        .replace("\r\n", "\n");
    let item = ch_app::ch_item::parse(&text).unwrap();
    let targets: Vec<Target> = [
        ("prefix-increased-spell-damage", Some(4)),
        ("suffix-increased-critical-hit-chance-for-spells", Some(4)),
        ("desecrated-prefix-increased-elemental-damage", None),
    ]
    .iter()
    .map(|(id, floor)| Target::new(*id, *floor))
    .collect();
    let plan = engine.plan(&item, &targets, &[]).unwrap();
    let route = plan
        .routes
        .iter()
        .find(|r| r.blueprint == "caster-cast-speed")
        .expect("the caster route plans on its start");
    let block = route.steps.iter().find(|s| s.id == "block").unwrap();
    assert_eq!(block.goal, Goal::Desecrate);
    assert_eq!(block.side, None);
    let action = block.action.as_ref().unwrap();
    assert!(action.omens.is_empty(), "{:?}", action.omens);
    let said = step_instruction(block, &item, &targets);
    assert!(!said.contains("Omen"), "{said}");
    assert!(!said.contains("Necromancy"), "{said}");
    assert!(!said.contains("desecrated modifier will be"), "{said}");
}

#[test]
fn every_side_desecration_names_its_necromancy_omen() {
    use ch_app::ch_engine::ch_blueprint::Goal;
    use ch_app::craft::step_instruction;
    let engine = Engine::shipped().unwrap();
    let cases = [
        ("ring", ring("01-start"), ring_targets()),
        ("2P/1S belt", belt("belt-2p1s"), belt_targets()),
        ("magic belt", MAGIC_BELT.to_string(), magic_belt_targets()),
    ];
    let mut checked = 0;
    let mut whittle_checked = false;
    for (what, text, targets) in &cases {
        let item = ch_app::ch_item::parse(text).unwrap();
        let plan = engine.plan(&item, targets, &[]).unwrap();
        assert!(!plan.routes.is_empty(), "{what}: no route");
        for route in &plan.routes {
            for step in route.steps.iter().filter(|s| s.goal == Goal::Desecrate) {
                let Some(action) = &step.action else { continue };
                let Some((_, omen, side)) = NECROMANCY
                    .iter()
                    .find(|(id, _, _)| action.omens.iter().any(|o| o == id))
                else {
                    continue;
                };
                let said = step_instruction(step, &item, targets);
                let at = format!("{what}, {} `{}`", route.blueprint, step.id);
                let start = format!("Use {omen}, then apply the {}", action.name);
                assert!(said.starts_with(&start), "{at}: {said}");
                assert!(
                    said.contains(&format!("the desecrated modifier will be a {side}")),
                    "{at}: {said}"
                );
                // Each target it may place is still named.
                let shown = ch_app::craft::step_targets(step, &item);
                assert!(
                    shown.iter().all(|t| said.contains(t.as_str())),
                    "{at}: {said}"
                );
                if route.blueprint == WHITTLE {
                    assert!(
                        said.starts_with(
                            "Use Omen of Dextral Necromancy, then apply the Preserved Collarbone"
                        ),
                        "{at}: {said}"
                    );
                    whittle_checked = true;
                }
                checked += 1;
            }
        }
    }
    // Positive control: the sets plan side desecrations to check, the
    // whittle's among them.
    assert!(
        whittle_checked,
        "the ring's whittle desecration was not checked"
    );
    assert!(checked >= 3, "only {checked} side desecrations planned");

    // The whittle's desecration, its last target left: the omen, the bone,
    // the side, then the target.
    let (route, item) = whittle_route();
    let mut desecrate = route.steps[2].clone();
    assert_eq!(desecrate.id, "desecrate");
    desecrate.targets = vec!["suffix-to-lightning-resistance".to_string()];
    let bone = desecrate.action.as_ref().unwrap().name.clone();
    assert_eq!(
        step_instruction(&desecrate, &item, &ring_targets()),
        format!("Use Omen of Dextral Necromancy, then apply the {bone}; the desecrated modifier will be a suffix, for +#% to Lightning Resistance")
    );
    // No target left: it ends after the side.
    desecrate.targets.clear();
    assert_eq!(
        step_instruction(&desecrate, &item, &ring_targets()),
        format!("Use Omen of Dextral Necromancy, then apply the {bone}; the desecrated modifier will be a suffix")
    );
}

/// The whittle's desecrate step as the preview shows it, the ring planned
/// with `targets` and `blocked` blocked for this craft.
fn whittle_desecration_line(targets: &[Target], blocked: &[&str]) -> ch_app::StepLineView {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);
    app.grab_text(Some(&ring("01-start")));
    app.set_targets(targets);
    let list: Vec<String> = blocked.iter().map(|id| id.to_string()).collect();
    app.set_craft_blocked(&list);
    let view = app.choose_route(WHITTLE);
    let route = planning(&view)
        .preview
        .as_ref()
        .unwrap()
        .route
        .as_ref()
        .expect("a route for the ring");
    assert_eq!(route.blueprint, WHITTLE);
    let line = route
        .steps
        .iter()
        .find(|s| s.id == "desecrate")
        .expect("the whittle desecrates")
        .clone();
    app.shutdown();
    line
}

#[test]
fn a_mixed_side_whittle_names_both_necromancy_omens() {
    // A suffix and a prefix: the omen is the last target's, which only the
    // guide will know. The preview names both, and each target's side.
    let phys = "prefix-adds-to-physical-damage-to-attacks";
    let targets = vec![Target::new(FIRE, Some(3)), Target::new(phys, None)];
    let line = whittle_desecration_line(&targets, &[]);
    assert!(
        line.instruction.starts_with("Use Omen of Sinistral Necromancy if the last target is a prefix, or Omen of Dextral Necromancy if it is a suffix, then apply the Preserved Collarbone"),
        "{}",
        line.instruction
    );
    assert_eq!(line.targets.len(), 2, "{:?}", line.targets);
    for text in &line.targets {
        let side = if text == FIRE_TEXT {
            "suffix"
        } else {
            "prefix"
        };
        assert!(
            line.instruction.contains(&format!("{text} ({side})")),
            "{}",
            line.instruction
        );
    }
    assert!(
        !line.instruction.contains("will be a"),
        "{}",
        line.instruction
    );
    assert_eq!(line.uses, ["Preserved Collarbone"]);
    assert_eq!(line.fallback, None);
}

#[test]
fn a_blocked_necromancy_omen_names_itself_in_the_fallback() {
    // The ring's targets are suffixes: Dextral Necromancy blocked, the
    // plain bone is used, and the fallback says which omen was.
    let line = whittle_desecration_line(&ring_targets(), &["omen-of-dextral-necromancy"]);
    assert_eq!(line.uses, ["Preserved Collarbone"]);
    assert_eq!(
        line.fallback.as_deref(),
        Some("Preserved Collarbone instead of a bone: Omen of Dextral Necromancy blocked")
    );
    assert!(
        line.instruction.starts_with("Preserved Collarbone for "),
        "{}",
        line.instruction
    );
    // Negative control: the other side's omen blocked changes nothing.
    let line = whittle_desecration_line(&ring_targets(), &["omen-of-sinistral-necromancy"]);
    assert_eq!(
        line.uses,
        ["Preserved Collarbone", "Omen of Dextral Necromancy"]
    );
    assert_eq!(line.fallback, None);
}

#[test]
fn the_route_view_drops_the_blueprint_notes() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = whittle_planned(tmp.path(), &shell);
    let view = app.get_view();
    let p = planning(&view);
    let route = p.preview.as_ref().unwrap().route.as_ref().unwrap();
    // Positive control: the blueprint has notes to drop.
    let (engine_route, _) = whittle_route();
    let notes = engine_route.notes.expect("the whittling blueprint's notes");
    // The route view has no notes field, and the notes' text is nowhere in
    // the view.
    assert!(serde_json::to_value(route).unwrap().get("notes").is_none());
    let sent = serde_json::to_string(&view).unwrap();
    let start: String = notes.chars().take(40).collect();
    assert!(!sent.contains(&start), "{start}");
    // In their place the panel has the chosen targets and the steps with
    // theirs; the name and the source (attribution) stay.
    assert_eq!(p.targets.len(), 3);
    assert!(route.steps.iter().all(|s| !s.targets.is_empty()));
    assert_eq!(route.name, "Rings: resistances by whittling");
    assert!(route.source.contains("Discord post 10"));
    app.shutdown();
}

#[test]
fn blocked_currency_shows_its_fallback() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = whittle_planned(tmp.path(), &shell);
    // Blocked in Settings: the whittle step falls back to its second way.
    let _ = app.update_settings(&json!({ "blocked_currencies": ["omen-of-whittling"] }));
    let view = app.get_view();
    let p = planning(&view);
    let route = p.preview.as_ref().unwrap().route.as_ref().unwrap();
    let whittle = &route.steps[1];
    assert_eq!(whittle.uses, ["Perfect Exalted Orb"]);
    assert_eq!(
        whittle.fallback.as_deref(),
        Some("Perfect Exalted Orb instead of Chaos Orb: Omen of Whittling blocked")
    );
    // The other steps took their first way.
    assert!(route
        .steps
        .iter()
        .filter(|s| s.id != "whittle")
        .all(|s| s.fallback.is_none()));
    assert_eq!(p.blocked.blocked.len(), 1);
    assert_eq!(p.blocked.blocked[0].name, "Omen of Whittling");
    assert_eq!(p.blocked.blocked[0].from, BlockedFrom::Settings);

    // Negative control: allowed for this craft, the first way is back.
    let view = app.set_craft_blocked(&[]);
    let route = planning(&view)
        .preview
        .as_ref()
        .unwrap()
        .route
        .as_ref()
        .unwrap();
    assert_eq!(route.steps[1].uses, ["Chaos Orb", "Omen of Whittling"]);
    assert!(route.steps[1].fallback.is_none());
    app.shutdown();
}

#[test]
fn craft_override_leaves_the_settings_list() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);
    app.update_settings(&json!({ "blocked_currencies": ["divine-orb", "chaos-orb"] }))
        .unwrap();
    app.flush();
    let saved = saved_settings(tmp.path());

    // Not while nothing is planned.
    let view = app.set_craft_blocked(&lines(&["divine-orb"]));
    assert_eq!(status(&view).0, StatusKind::Error);

    app.grab_text(Some(&ring("01-start")));
    let view = app.set_craft_blocked(&lines(&["divine-orb", "omen-of-whittling"]));
    assert_eq!(
        status(&view),
        (StatusKind::Info, "Blocked for this craft: 2.".to_string())
    );
    let b = &planning(&view).blocked;
    let entries: Vec<(&str, BlockedFrom)> =
        b.blocked.iter().map(|e| (e.id.as_str(), e.from)).collect();
    assert_eq!(
        entries,
        [
            ("divine-orb", BlockedFrom::Settings),
            ("omen-of-whittling", BlockedFrom::Craft)
        ]
    );
    assert_eq!(b.blocked[0].name, "Divine Orb");
    let allowed: Vec<&str> = b.allowed.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(allowed, ["chaos-orb"]);
    assert_eq!(b.allowed[0].name, "Chaos Orb");

    // Settings are not changed, in memory or on disk.
    assert_eq!(
        app.get_app_settings().blocked_currencies,
        lines(&["divine-orb", "chaos-orb"])
    );
    app.flush();
    assert_eq!(saved_settings(tmp.path()), saved);

    // A Settings change made meanwhile still reaches the craft, and the
    // craft's own changes hold.
    app.update_settings(
        &json!({ "blocked_currencies": ["divine-orb", "chaos-orb", "exalted-orb"] }),
    )
    .unwrap();
    let view = app.get_view();
    let b = &planning(&view).blocked;
    let ids: Vec<&str> = b.blocked.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(ids, ["divine-orb", "exalted-orb", "omen-of-whittling"]);
    assert_eq!(b.allowed.len(), 1);

    // An id outside the catalogue is refused, nothing changed.
    let view = app.set_craft_blocked(&lines(&["not-a-crafting-item"]));
    assert_eq!(status(&view).0, StatusKind::Error);
    assert_eq!(planning(&view).blocked.blocked.len(), 3);

    // Another planning grab keeps them; one from refused clears them.
    let view = app.grab_text(Some(&ring("02-chaos-fire")));
    assert_eq!(planning(&view).blocked.blocked.len(), 3);
    app.grab_text(Some(NOT_ITEM_TEXT));
    let view = app.grab_text(Some(&ring("01-start")));
    let b = &planning(&view).blocked;
    let ids: Vec<&str> = b.blocked.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(ids, ["divine-orb", "chaos-orb", "exalted-orb"]);
    assert!(b.allowed.is_empty());
    app.shutdown();
    assert!(craft_lines(tmp.path()).contains(&"craft: 2 blocked for this craft".to_string()));
}

#[test]
fn no_route_says_why_each_blueprint_was_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);
    app.grab_text(Some(&ring("01-start")));
    app.set_targets(&ring_targets());
    // With Chaos Orbs and Orbs of Annulment blocked no ring route has a
    // first step (the state-ordered ones, which clean the suffix side with
    // an annulment, included).
    let view = app.set_craft_blocked(&lines(&["chaos-orb", "orb-of-annulment"]));
    let p = planning(&view);
    let preview = p.preview.as_ref().unwrap();
    assert!(preview.route.is_none());
    assert!(preview.other_routes.is_empty());
    assert!(!p.can_accept);
    // Blocked currencies, not a floor an essence misses: nothing to lower.
    assert!(preview.relaxation.is_none(), "{:?}", preview.relaxation);
    let engine = Engine::shipped().unwrap();
    assert_eq!(preview.rejected.len(), engine.blueprints().len());
    for r in &preview.rejected {
        assert!(!r.name.is_empty() && !r.reason.is_empty(), "{r:?}");
        assert_eq!(
            r.name,
            *engine
                .blueprint(&r.blueprint)
                .unwrap()
                .blueprint()
                .name
                .get_ref()
        );
    }
    let whittle = preview
        .rejected
        .iter()
        .find(|r| r.blueprint == WHITTLE)
        .unwrap();
    assert_eq!(whittle.reason, "step `chaos`: every alternative is blocked");
    let jewels = preview
        .rejected
        .iter()
        .find(|r| r.blueprint == "jewels-three-one-side")
        .unwrap();
    assert_eq!(jewels.reason, "not a blueprint for Rings");

    // Nothing to start.
    let view = app.accept_craft();
    assert_eq!(view.craft.mode, CraftMode::Planning);
    assert_eq!(
        status(&view),
        (
            StatusKind::Error,
            "Craft not started: no route reaches these targets.".to_string()
        )
    );
    app.shutdown();
}

/// Fire, Cold and Lightning Resistance on the magic belt, fire at `fire`,
/// the others at T2.
fn belt_resistances(fire: u32) -> Vec<Target> {
    let mut list = magic_belt_targets();
    list[0].floor = Some(fire);
    list[1].floor = Some(2);
    list[2].floor = Some(2);
    list
}

#[test]
fn no_route_shows_the_tier_relaxation() {
    // The magic belt with its three resistances at T2: no route, since the
    // belt's best fire essence (Greater Essence of Insulation) adds T3. The
    // planning view proposes lowering fire to T3, and nothing starts yet.
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);
    app.grab_text(Some(MAGIC_BELT));
    let view = app.set_targets(&belt_resistances(2));
    let p = planning(&view);
    let preview = p.preview.as_ref().unwrap();
    assert!(preview.route.is_none());
    assert!(!preview.rejected.is_empty());
    assert!(!p.can_accept);
    let relax = preview.relaxation.as_ref().expect("a relaxation");
    assert!(
        relax
            .text
            .starts_with(&format!("Lower {FIRE_TEXT} from T2 to T3")),
        "{}",
        relax.text
    );
    assert!(
        relax.text.contains("Greater Essence of Insulation"),
        "{}",
        relax.text
    );
    assert!(relax.text.contains("unlocks a route"), "{}", relax.text);
    assert_eq!(relax.mod_id, FIRE);
    assert_eq!((relax.from, relax.to), (2, 3));
    assert_eq!(relax.currency, "Greater Essence of Insulation");
    // The targets are untouched until the player accepts.
    assert_eq!(chosen(p)[0], (FIRE.to_string(), Some(2)));
    app.shutdown();
}

#[test]
fn accepting_the_relaxation_previews_a_route() {
    // Accepting is an ordinary `set_targets` with fire at T3: a route
    // through the essence, and no proposal any more.
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);
    app.grab_text(Some(MAGIC_BELT));
    app.set_targets(&belt_resistances(2));
    let view = app.set_targets(&belt_resistances(3));
    let p = planning(&view);
    let preview = p.preview.as_ref().unwrap();
    assert!(preview.relaxation.is_none(), "{:?}", preview.relaxation);
    assert!(p.can_accept);
    let route = preview.route.as_ref().expect("a route");
    assert!(
        route
            .steps
            .iter()
            .any(|s| s.uses.iter().any(|u| u == "Greater Essence of Insulation")),
        "{:?}",
        route.steps
    );
    app.shutdown();
}

/// The state-ordered route whose reveal comes before a same-side exalt on
/// the relaxed magic belt: the essence (fire), a suffix desecration (cold),
/// its reveal, then a Perfect Exalted Orb on the suffix side.
const DOUBLE_SLAM: &str = "essence-desecrate-double-slam";

/// The relaxed magic belt's preview of `blueprint`: its step lines.
fn magic_belt_steps(blueprint: &str) -> Vec<ch_app::StepLineView> {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);
    app.grab_text(Some(MAGIC_BELT));
    app.set_targets(&belt_resistances(3));
    let view = app.choose_route(blueprint);
    let route = planning(&view)
        .preview
        .as_ref()
        .unwrap()
        .route
        .clone()
        .expect("a route for the relaxed belt");
    assert_eq!(route.blueprint, blueprint);
    app.shutdown();
    route.steps
}

#[test]
fn no_blocker_line_on_a_last_desecration_baseline() {
    // `magic-augment-essence` desecrates last: its reveal is followed by
    // nothing on that side, so no step names a blocker.
    let steps = magic_belt_steps("magic-augment-essence");
    assert!(
        steps.iter().any(|s| s.id.contains("reveal")),
        "a reveal: {steps:?}"
    );
    for s in &steps {
        assert_eq!(s.blocker, None, "{}", s.id);
    }
}

#[test]
fn reveal_step_names_the_blocker_with_before_and_after() {
    use ch_app::craft::{blocker_line, odds_percent};
    let steps = magic_belt_steps(DOUBLE_SLAM);
    let ids: Vec<&str> = steps.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(ids, ["essence", "desecrate", "reveal", "side-slam"]);

    // The engine's advice, in the step odds' percent forms.
    let engine = Engine::shipped().unwrap();
    let magic = ch_app::ch_item::parse(MAGIC_BELT).unwrap();
    let advice = engine
        .blocker_advice(&magic, &belt_resistances(3), &[], DOUBLE_SLAM)
        .unwrap()
        .expect("advice at the reveal");
    let before = odds_percent(&advice.before);
    let after = odds_percent(&advice.picks[0].after);
    assert!(
        before.starts_with('~') && after.starts_with('~'),
        "{before} {after}"
    );
    assert_ne!(before, after);

    let line = steps[2]
        .blocker
        .as_deref()
        .expect("the reveal's blocker line");
    assert_eq!(line, blocker_line(&advice));
    assert_eq!(
        line,
        format!(
            "If {COLD_TEXT} is not offered, take +# to Strength and Dexterity or +# to Strength and Intelligence (keeps out +# to Strength): the suffix exalt's chance of a target goes from {before} to {after}"
        )
    );
    // Every other step has none.
    for s in steps.iter().filter(|s| s.id != "reveal") {
        assert_eq!(s.blocker, None, "{}", s.id);
    }

    // Guiding: the essence lands fire, the desecration leaves an unrevealed
    // suffix; the reveal, now the current step, carries the same line, and
    // the steps before it none.
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);
    app.grab_text(Some(MAGIC_BELT));
    app.set_targets(&belt_resistances(3));
    app.choose_route(DOUBLE_SLAM);
    let view = app.accept_craft();
    let current = |view: &View| guiding(view).step.clone().expect("a current step").step;
    let step = current(&view);
    assert_eq!(step.id, "essence");
    assert_eq!(step.blocker, None);
    let view = app.grab_text(Some(&belt("belt-magic-essence")));
    let step = current(&view);
    assert_eq!(step.id, "desecrate", "{:?}", guiding(&view).history);
    assert_eq!(step.blocker, None);
    let view = app.grab_text(Some(&belt("belt-magic-desecrated")));
    let step = current(&view);
    assert_eq!(step.id, "reveal", "{:?}", guiding(&view).history);
    assert_eq!(step.blocker.as_deref(), Some(line));
    app.shutdown();
}

/// The shipped route that swaps a full side's throwaway for Essence of
/// Hysteria's modifier (`ch-engine`'s `tests/essence_swap.rs`).
const ESSENCE_SWAP: &str = "fractured-essence-swap";
const MANA_REGEN: &str = "suffix-increased-mana-regeneration-rate";

/// `ch-engine`'s essence swap ring (`tests/items/`, copied): its suffixes
/// a fractured Fire Resistance, an unrevealed desecrated suffix and
/// Dexterity; or the Dusk Ring, with the fractured one and Dexterity.
fn swap_ring(name: &str) -> String {
    belt(name)
}

/// The essence swap route's step lines on `item` with `list`.
fn essence_swap_steps(item: &str, list: &[Target]) -> Vec<ch_app::StepLineView> {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);
    app.grab_text(Some(&swap_ring(item)));
    app.set_targets(list);
    let view = app.choose_route(ESSENCE_SWAP);
    let route = planning(&view)
        .preview
        .as_ref()
        .unwrap()
        .route
        .clone()
        .unwrap_or_else(|| panic!("no route: {:?}", planning(&view).preview));
    assert_eq!(route.blueprint, ESSENCE_SWAP);
    app.shutdown();
    route.steps
}

#[test]
fn essence_swap_step_names_its_removal_and_survival() {
    // Mana T5 on the ring's prefixes, Hysteria's Mana Regeneration Rate T2
    // and a desecrated suffix: the essence first, its add certain.
    let list = vec![
        Target::new("prefix-to-maximum-mana", Some(5)),
        Target::new(MANA_REGEN, Some(2)),
        Target::new("desecrated-suffix-to-cold-and-chaos-resistances", None),
    ];
    let steps = essence_swap_steps("ring-essence-swap", &list);
    let essence = &steps[0];
    assert_eq!(essence.id, "essence");
    assert_eq!(essence.uses, ["Essence of Hysteria"]);
    let said = &essence.instruction;
    assert!(said.starts_with("Essence of Hysteria for "), "{said}");
    assert!(said.contains("Mana Regeneration Rate"), "{said}");
    // What it can remove, by its text on the item, and the fractured one.
    assert!(
        said.contains(
            "it removes the unrevealed desecrated suffix or +10(9-12) to Dexterity, one at random"
        ),
        "{said}"
    );
    assert!(
        said.contains("+23(21-25)% to Fire Resistance is fractured and stays"),
        "{said}"
    );

    // The survival line, each outcome followed by the instruction of the
    // step the engine names for it.
    let engine = Engine::shipped().unwrap();
    let item = ch_app::ch_item::parse(&swap_ring("ring-essence-swap")).unwrap();
    let started = engine.start(ESSENCE_SWAP, &item, &list, &[]).unwrap();
    let survival = started.route.steps[0]
        .swap
        .as_ref()
        .and_then(|s| s.survival.as_ref())
        .expect("the engine's survival");
    let stays = survival.stays.as_deref().unwrap();
    let goes = survival.goes.as_deref().unwrap();
    assert_eq!(
        (stays.id.as_str(), goes.id.as_str()),
        ("reveal-last", "remove")
    );
    let line = essence.survival.as_deref().expect("a survival line");
    assert!(line.contains("1 in 2"), "{line}");
    assert!(line.contains("unrevealed desecrated suffix"), "{line}");
    let stays_at = line.find("If it stays, ").expect("If it stays,");
    let goes_at = line.find("If it goes, ").expect("If it goes,");
    assert!(stays_at < goes_at, "{line}");
    let stays_said = &line[stays_at + "If it stays, ".len()..goes_at];
    let goes_said = &line[goes_at + "If it goes, ".len()..];
    let stays_instruction = ch_app::craft::step_instruction(stays, &item, &list);
    assert!(
        stays_said.starts_with(&stays_instruction),
        "{stays_said} / {stays_instruction}"
    );
    assert!(stays_said.contains("Cold and Chaos"), "{stays_said}");
    // On "goes" the annulment confined to the suffixes: Dextral, what it
    // removes on the item the swap left, its chance of sparing the
    // essence's modifier, and the re-plan if it takes it.
    assert!(
        goes_said.starts_with(
            "Orb of Annulment with Omen of Dextral Annulment to remove +10(9-12) to Dexterity"
        ),
        "{goes_said}"
    );
    assert!(
        goes_said.contains("spares #% increased Mana Regeneration Rate 1 in 2"),
        "{goes_said}"
    );
    assert!(
        goes_said.contains("the route re-plans when the item is read again"),
        "{goes_said}"
    );
    // No other step carries one.
    for s in &steps[1..] {
        assert_eq!(s.survival, None, "{}", s.id);
    }

    // The Dusk Ring: one ordinary suffix beside the fractured one, so the
    // removal is certain and nothing survives or not.
    let dusk = vec![
        Target::new("prefix-to-maximum-mana", Some(5)),
        Target::new(MANA_REGEN, Some(2)),
    ];
    let steps = essence_swap_steps("dusk-ring-essence-swap", &dusk);
    let essence = &steps[0];
    assert_eq!(essence.id, "essence");
    assert!(
        essence
            .instruction
            .contains("it removes +10(9-12) to Dexterity; +23(21-25)% to Fire Resistance is fractured and stays"),
        "{}",
        essence.instruction
    );
    assert_eq!(essence.survival, None);

    // Control: a route without an essence swap carries no survival line.
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = whittle_planned(tmp.path(), &shell);
    let view = app.get_view();
    let route = planning(&view)
        .preview
        .as_ref()
        .unwrap()
        .route
        .clone()
        .unwrap();
    assert!(route.steps.iter().all(|s| s.survival.is_none()));
    app.shutdown();
}

#[test]
fn refused_item_says_why() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);

    // A corrupted ring: the engine's refusal.
    let corrupted = format!("{}\n--------\nCorrupted", ring("01-start").trim_end());
    let view = app.grab_text(Some(&corrupted));
    assert_eq!(view.craft.mode, CraftMode::Refused);
    assert_eq!(view.craft.refusal.as_deref(), Some("the item is corrupted"));
    assert!(view.craft.planning.is_none() && view.craft.guiding.is_none());
    // The grab itself is kept, as M3 keeps it.
    assert!(view.grab.as_ref().unwrap().looks_like_item);

    // Text that is not an item copied with Ctrl+Alt+C.
    let view = app.grab_text(Some(NOT_ITEM_TEXT));
    assert_eq!(
        view.craft.refusal.as_deref(),
        Some("the text is not an item copied with Ctrl+Alt+C in the game")
    );
    // The invented gloves have no modifier headers.
    let view = app.grab_text(Some(ITEM_TEXT));
    assert_eq!(
        view.craft.refusal.as_deref(),
        Some("the item text has no modifier headers, so no tiers")
    );

    // Commands are refused with the reason, and change nothing.
    let view = app.accept_craft();
    assert_eq!(view.craft.mode, CraftMode::Refused);
    assert_eq!(
        status(&view).1,
        "Craft not started: no item is being planned; grab one first."
    );
    // A plannable grab plans.
    let view = app.grab_text(Some(&ring("01-start")));
    assert_eq!(view.craft.mode, CraftMode::Planning);
    assert!(view.craft.refusal.is_none());
    app.shutdown();
    assert!(craft_lines(tmp.path())
        .contains(&"craft: refused: no item is being planned; grab one first".to_string()));
}

#[test]
fn accept_starts_the_guide_on_the_previewed_route() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = whittle_planned(tmp.path(), &shell);
    let view = app.accept_craft();
    assert_eq!(
        status(&view),
        (StatusKind::Info, "Craft started.".to_string())
    );
    let g = guiding(&view);
    assert_eq!(g.blueprint, WHITTLE);
    assert_eq!(g.route_name, "Rings: resistances by whittling");
    assert_eq!(g.status, GuideStatusView::Active);
    assert!(g.history.is_empty() && g.reasons.is_empty());
    assert_eq!(g.item.name.as_deref(), Some("Doom Loop"));
    let step = g.step.as_ref().unwrap();
    assert_eq!((step.step.id.as_str(), step.step.number), ("chaos", 1));
    assert_eq!(step.try_number, 1);
    assert_eq!(step.step.uses, ["Chaos Orb"]);
    assert!(step.step.odds.as_deref().unwrap().starts_with('~'));
    // The next step by its composed instruction, never the blueprint's text.
    assert_eq!(
        step.on_hit,
        "On a hit: Chaos Orb with Omen of Whittling for +#% to Fire Resistance (suffix), +#% to Cold Resistance (suffix) or +#% to Lightning Resistance (suffix)"
    );
    assert_eq!(step.on_miss, "On a miss: this step again");

    // Planning commands wait for the craft to end.
    let view = app.set_targets(&[]);
    assert_eq!(view.craft.mode, CraftMode::Guiding);
    assert_eq!(
        status(&view).1,
        "Targets not changed: a craft is in progress; end it first."
    );
    let view = app.accept_craft();
    assert_eq!(status(&view).0, StatusKind::Error);
    app.shutdown();
    let log = craft_lines(tmp.path());
    assert!(
        log.contains(&format!("craft: accepted {WHITTLE}")),
        "{log:?}"
    );
}

#[test]
fn regrab_hit_advances_the_guide() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::with_clipboard(&ring("02-chaos-fire"));
    let app = whittle_planned(tmp.path(), &shell);
    app.accept_craft();
    // From the hotkey, as in the game.
    app.hotkey_pressed();
    let view = app.get_view();
    let g = guiding(&view);
    assert_eq!(g.history.len(), 1);
    let h = &g.history[0];
    assert_eq!(
        (h.number, h.step.as_str(), h.result),
        (1, "chaos", OutcomeWord::Hit)
    );
    assert_eq!(h.instruction, CHAOS_STEP);
    // The summary carries no word; the status line says it once.
    assert_eq!(h.summary, "gained +#% to Fire Resistance (T3)");
    assert_eq!(
        status(&view),
        (StatusKind::Info, format!("Hit: {}.", h.summary))
    );
    let step = g.step.as_ref().unwrap();
    assert_eq!((step.step.id.as_str(), step.step.number), ("whittle", 2));
    assert_eq!(step.try_number, 1);
    assert_eq!(
        step.on_hit,
        "On a hit: this step again until its condition holds, then use Omen of Dextral Necromancy, then apply the Preserved Collarbone; the desecrated modifier will be a suffix, for +#% to Cold Resistance or +#% to Lightning Resistance"
    );
    // The item now is the one grabbed, its fire resistance a target met.
    let fire = g
        .item
        .modifiers
        .iter()
        .find(|m| m.lines[0].contains("Fire Resistance"))
        .unwrap();
    assert!(fire.meets_target);
    assert_eq!(shell.rec().beeps, 0);
    app.shutdown();
    assert!(craft_lines(tmp.path()).contains(&"craft: hit at chaos".to_string()));
}

#[test]
fn regrab_miss_repeats_and_counts_the_try() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = whittle_planned(tmp.path(), &shell);
    app.accept_craft();
    // Fire resistance at T5, below the T3 floor.
    let view = app.grab_text(Some(&ring("06-chaos-below-floor")));
    let g = guiding(&view);
    assert_eq!(g.status, GuideStatusView::Active);
    let h = &g.history[0];
    assert_eq!(h.result, OutcomeWord::Miss);
    assert_eq!(h.summary, "+#% to Fire Resistance (T5) below its floor T3");
    let step = g.step.as_ref().unwrap();
    assert_eq!(step.step.id, "chaos");
    assert_eq!(step.try_number, 2);
    // The next try lands it: on to whittling, the history oldest first.
    let view = app.grab_text(Some(&ring("02-chaos-fire")));
    let g = guiding(&view);
    let results: Vec<OutcomeWord> = g.history.iter().map(|h| h.result).collect();
    assert_eq!(results, [OutcomeWord::Miss, OutcomeWord::Hit]);
    assert_eq!(g.step.as_ref().unwrap().step.id, "whittle");
    assert_eq!(g.step.as_ref().unwrap().try_number, 1);
    app.shutdown();
    let log = craft_lines(tmp.path());
    assert!(log.contains(&"craft: miss at chaos".to_string()), "{log:?}");
}

#[test]
fn unexpected_change_offers_plan_again() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = whittle_planned(tmp.path(), &shell);
    app.accept_craft();
    // Two targets at once from one Chaos Orb: not what the step does.
    let view = app.grab_text(Some(&ring("03-whittle-cold")));
    let g = guiding(&view);
    assert_eq!(g.status, GuideStatusView::Replan);
    assert!(g.step.is_none());
    assert!(!g.reasons.is_empty());
    let h = &g.history[0];
    assert_eq!(h.result, OutcomeWord::Unexpected);
    assert!(!h.summary.starts_with("Unexpected"), "{}", h.summary);
    assert_eq!(status(&view).1, format!("Unexpected: {}.", h.summary));
    assert!(
        h.summary.contains(&g.reasons[0]),
        "{} / {:?}",
        h.summary,
        g.reasons
    );
    assert_eq!(status(&view).0, StatusKind::Error);
    // Another grab is not a step's result now.
    let view = app.grab_text(Some(&ring("02-chaos-fire")));
    assert_eq!(guiding(&view).history.len(), 1);
    assert!(
        status(&view).1.starts_with("Not a step result: "),
        "{:?}",
        view.status
    );

    // Ending the craft plans again from the latest item, targets kept.
    let view = app.end_craft();
    let p = planning(&view);
    assert_eq!(p.targets.len(), 3);
    let cold = p
        .item
        .modifiers
        .iter()
        .find(|m| m.lines[0].contains("Cold Resistance"));
    assert!(cold.is_some(), "the latest item is 03");
    assert!(p.can_accept);
    // Not twice.
    let view = app.end_craft();
    assert_eq!(status(&view).1, "Craft not ended: no craft is in progress.");
    app.shutdown();
    let log = craft_lines(tmp.path());
    assert!(
        log.contains(&"craft: unexpected at chaos".to_string()),
        "{log:?}"
    );
    assert!(log.contains(&"craft: ended".to_string()), "{log:?}");
}

#[test]
fn unchanged_regrab_is_not_an_outcome() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = whittle_planned(tmp.path(), &shell);
    let before = guiding(&app.accept_craft()).clone();

    // The same item, with the clipboard's CRLF line breaks: not an outcome.
    let crlf = ring("01-start").replace('\n', "\r\n");
    shell.set_clipboard(Some(&crlf));
    app.hotkey_pressed();
    let view = app.get_view();
    assert_eq!(guiding(&view), &before);
    assert_eq!(
        status(&view),
        (
            StatusKind::Info,
            "This is the item the step was shown on: take the step in the game, copy the item with Ctrl+Alt+C, then grab it again.".to_string()
        )
    );
    // Not an item: the guide is as it was, with the reason.
    let view = app.grab_text(Some(NOT_ITEM_TEXT));
    assert_eq!(guiding(&view), &before);
    assert_eq!(
        status(&view),
        (
            StatusKind::Error,
            "Not a step result: the text is not an item copied with Ctrl+Alt+C in the game. The craft is unchanged.".to_string()
        )
    );
    app.shutdown();
    assert!(!craft_lines(tmp.path())
        .iter()
        .any(|l| l.contains(" at chaos")));
}

#[test]
fn craft_reaches_done() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = whittle_planned(tmp.path(), &shell);
    app.accept_craft();
    let walk = [
        ("02-chaos-fire", "whittle"),
        ("03-whittle-cold", "desecrate"),
        ("04-desecrated", "reveal"),
    ];
    for (file, next) in walk {
        let view = app.grab_text(Some(&ring(file)));
        let g = guiding(&view);
        assert_eq!(g.history.last().unwrap().result, OutcomeWord::Hit, "{file}");
        assert_eq!(g.step.as_ref().unwrap().step.id, next, "{file}");
    }
    let view = app.get_view();
    let reveal = guiding(&view).step.as_ref().unwrap();
    assert_eq!(reveal.on_hit, "On a hit: the craft is done");
    assert_eq!(
        reveal.on_miss,
        "On a miss: Chaos Orb with Omen of Whittling for +#% to Lightning Resistance (suffix)"
    );
    let view = app.grab_text(Some(&ring("05-revealed-lightning")));
    let g = guiding(&view);
    assert_eq!(g.status, GuideStatusView::Done);
    assert!(g.step.is_none() && g.reasons.is_empty());
    assert_eq!(g.history.len(), 4);
    assert!(g.history.iter().all(|h| h.result == OutcomeWord::Hit));
    let numbers: Vec<usize> = g.history.iter().map(|h| h.number).collect();
    assert_eq!(numbers, [1, 2, 3, 4]);
    assert_eq!(
        g.history[3].summary,
        "gained +#% to Lightning Resistance (T3)"
    );
    assert_eq!(
        g.item.modifiers.iter().filter(|m| m.meets_target).count(),
        3
    );
    // Done: a grab is not a step's result.
    let view = app.grab_text(Some(&ring("01-start")));
    assert_eq!(guiding(&view).status, GuideStatusView::Done);
    assert_eq!(
        status(&view).1,
        "Not a step result: the craft is done; end it to plan from this item. The craft is unchanged."
    );
    let view = app.end_craft();
    assert_eq!(view.craft.mode, CraftMode::Planning);
    assert!(planning(&view).targets.len() == 3);
    app.shutdown();
}

#[test]
fn open_craft_of_exile_opens_only_the_fixed_address() {
    assert_eq!(
        CRAFT_OF_EXILE_URL,
        "https://beta.craftofexile.com/?game=poe2"
    );
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = whittle_planned(tmp.path(), &shell);
    app.accept_craft();
    app.grab_text(Some(&ring("02-chaos-fire")));
    app.end_craft();
    // Baseline: no other command opened anything.
    assert!(shell.rec().opened.is_empty());

    let view = app.open_craft_of_exile();
    assert_eq!(shell.rec().opened, [CRAFT_OF_EXILE_URL]);
    assert_eq!(
        status(&view),
        (
            StatusKind::Info,
            "Opened Craft of Exile in your web browser.".to_string()
        )
    );
    // The craft is untouched.
    assert_eq!(view.craft.mode, CraftMode::Planning);

    // A failure says why, in the status line and the log.
    shell.rec().open_fails = Some("no default browser".to_string());
    let view = app.open_craft_of_exile();
    assert_eq!(
        status(&view),
        (
            StatusKind::Error,
            "Craft of Exile could not be opened: no default browser.".to_string()
        )
    );
    assert_eq!(shell.rec().opened, [CRAFT_OF_EXILE_URL, CRAFT_OF_EXILE_URL]);
    app.shutdown();
    let log = diag_lines(tmp.path());
    assert!(
        log.contains(&"opened Craft of Exile".to_string()),
        "{log:?}"
    );
    assert!(
        log.contains(&"error: opening Craft of Exile: no default browser".to_string()),
        "{log:?}"
    );

    // A shell that cannot open a browser refuses by default.
    struct Bare;
    impl ch_app::Shell for Bare {
        fn register_hotkey(&self, _: &str) -> Result<(), ch_app::HotkeyError> {
            Ok(())
        }
        fn read_clipboard(&self) -> Result<Option<String>, String> {
            Ok(None)
        }
        fn beep(&self) -> Vec<ch_app::BeepAttempt> {
            Vec::new()
        }
        fn emit_view(&self, _: &View) {}
        fn set_overlay_visible(&self, _: bool) {}
    }
    let bare_dir = tempfile::tempdir().unwrap();
    let bare = ch_app::App::new(ch_app::AppConfig {
        store: ch_app::Store::open(bare_dir.path()),
        shell: Box::new(Bare),
        screen: None,
    });
    let view = bare.open_craft_of_exile();
    assert_eq!(status(&view).0, StatusKind::Error);
    bare.shutdown();
    // Every catalogue item is a currency name, never an address.
    assert!(catalogue().iter().all(|c| !c.name.contains("://")));
}
