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
    assert!(p.item.modifiers.iter().all(|m| !m.meets_target));

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
    // No target yet: no preview, nothing to accept.
    assert!(p.targets.is_empty() && p.preview.is_none() && !p.can_accept);
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
    assert_eq!(status(&view).1, "Route: Rings: resistances by whittling.");
    let preview = planning(&view).preview.as_ref().unwrap();
    let route = preview.route.as_ref().unwrap();
    assert_eq!(route.name, "Rings: resistances by whittling");
    assert!(route.source.contains("Discord post 10"));
    assert!(route.notes.is_some());
    let ids: Vec<&str> = route.steps.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(ids, ["chaos", "whittle", "desecrate", "reveal"]);
    let numbers: Vec<usize> = route.steps.iter().map(|s| s.number).collect();
    assert_eq!(numbers, [1, 2, 3, 4]);
    assert_eq!(
        route.steps[0].instruction,
        "Chaos Orb until one of the targets lands"
    );
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
    // With Chaos Orbs blocked no ring route has a first step.
    let view = app.set_craft_blocked(&lines(&["chaos-orb"]));
    let p = planning(&view);
    let preview = p.preview.as_ref().unwrap();
    assert!(preview.route.is_none());
    assert!(preview.other_routes.is_empty());
    assert!(!p.can_accept);
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
        (
            StatusKind::Info,
            "Started Rings: resistances by whittling.".to_string()
        )
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
    assert_eq!(
        step.on_hit,
        "On a hit: Whittle (Omen of Whittling with a Chaos Orb) until one target is left"
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
    assert_eq!(h.instruction, "Chaos Orb until one of the targets lands");
    assert_eq!(h.summary, "Hit: gained +#% to Fire Resistance (T3)");
    assert_eq!(status(&view), (StatusKind::Info, format!("{}.", h.summary)));
    let step = g.step.as_ref().unwrap();
    assert_eq!((step.step.id.as_str(), step.step.number), ("whittle", 2));
    assert_eq!(step.try_number, 1);
    assert_eq!(
        step.on_hit,
        "On a hit: this step again until its condition holds, then Desecrate for the last target"
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
    assert_eq!(
        h.summary,
        "Miss: +#% to Fire Resistance (T5) below its floor T3"
    );
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
    assert!(h.summary.starts_with("Unexpected: "), "{}", h.summary);
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
        "On a miss: Whittle (Omen of Whittling with a Chaos Orb) until one target is left"
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
        "Hit: gained +#% to Lightning Resistance (T3)"
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
fn overlay_stays_hidden_while_guiding() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = whittle_planned(tmp.path(), &shell);
    let view = app.accept_craft();
    assert_eq!(view.craft.mode, CraftMode::Guiding);
    assert!(!view.overlay.visible);
    for file in ["06-chaos-below-floor", "02-chaos-fire", "03-whittle-cold"] {
        assert!(!app.grab_text(Some(&ring(file))).overlay.visible);
    }
    app.end_craft();
    // The shell was never asked to show it, and no view sent showed it.
    let rec = shell.rec();
    assert!(rec.overlay.is_empty(), "{:?}", rec.overlay);
    assert!(rec.views.iter().all(|v| !v.overlay.visible));
    assert!(rec.views.iter().any(|v| v.craft.mode == CraftMode::Guiding));
    drop(rec);
    app.shutdown();
    assert!(!diag_lines(tmp.path())
        .iter()
        .any(|l| l.starts_with("overlay")));
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
    });
    let view = bare.open_craft_of_exile();
    assert_eq!(status(&view).0, StatusKind::Error);
    bare.shutdown();
    // Every catalogue item is a currency name, never an address.
    assert!(catalogue().iter().all(|c| !c.name.contains("://")));
}
