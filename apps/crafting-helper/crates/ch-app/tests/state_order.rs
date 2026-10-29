//! The craft window on the state-ordered routes, through `App` with the
//! shipped engine: a fill step's instruction composed from its data, the
//! owner's two crafts (the 2P/1S belt never offered an unrestricted exalt,
//! issue #28's rare with one target given a route), the branch lines of a
//! route whose next step is picked from the item, the fallback line only
//! where a blocked id passed an alternative over, and a removal step naming
//! its side annulment.
//!
//! Items: the owner's 2P/1S belt (`tests/items/`, constructed), and
//! `ch-engine`'s slot case items and walk states
//! (`crates/ch-engine/tests/items/slots/`, constructed): issue #28's ring
//! (`rare-one-target`) and the magic amulet's augment and essence walk
//! (`magic-one-good`, `walks/augment-fire`, `walks/augment-essence`).

mod common;

use std::sync::Arc;

use ch_app::ch_engine::{Engine, Target};
use ch_app::craft::{source_line, PICKED_AFTER_MISS};
use ch_app::{App, CraftMode, GuideStatusView, OutcomeWord, RouteView, View};
use common::*;

const GREATER: &str = "Omen of Greater Exaltation";
const SINISTRAL_EXALT: &str = "Omen of Sinistral Exaltation";
const DEXTRAL_EXALT: &str = "Omen of Dextral Exaltation";
const SINISTRAL_ANNUL: &str = "Omen of Sinistral Annulment";
const ANNUL: &str = "Orb of Annulment";

fn guiding(view: &View) -> &ch_app::GuidingView {
    assert_eq!(view.craft.mode, CraftMode::Guiding, "{:?}", view.status);
    view.craft.guiding.as_ref().expect("the guiding part")
}

fn step(view: &View) -> &ch_app::CurrentStepView {
    guiding(view).step.as_ref().expect("a current step")
}

fn preview(view: &View) -> &ch_app::PreviewView {
    view.craft
        .planning
        .as_ref()
        .expect("planning")
        .preview
        .as_ref()
        .expect("a preview")
}

fn route(view: &View) -> &RouteView {
    preview(view).route.as_ref().expect("a route")
}

/// A started app planning `text` with `targets`.
fn planned(dir: &std::path::Path, shell: &FakeShell, text: &str, targets: &[Target]) -> Arc<App> {
    let app = start(dir, shell);
    app.grab_text(Some(text));
    app.set_targets(targets);
    app
}

/// Every route the preview offers for the planning item, each as the
/// preview shows it once chosen.
fn every_route(app: &App) -> Vec<RouteView> {
    let view = app.get_view();
    let mut ids = vec![route(&view).blueprint.clone()];
    ids.extend(
        preview(&view)
            .other_routes
            .iter()
            .map(|r| r.blueprint.clone()),
    );
    ids.iter()
        .map(|id| route(&app.choose_route(id)).clone())
        .collect()
}

/// An Exalted Orb of any grade, without Sinistral or Dextral Exaltation:
/// it may land on either side.
fn unrestricted_exalt(uses: &[String]) -> bool {
    uses.first().is_some_and(|u| u.ends_with("Exalted Orb"))
        && !uses
            .iter()
            .any(|u| u == SINISTRAL_EXALT || u == DEXTRAL_EXALT)
}

#[test]
fn a_fill_step_names_its_currency_omens_and_side() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = planned(tmp.path(), &shell, &belt("belt-2p1s"), &belt_targets());
    let view = app.choose_route(CLOSE);
    let close = &route(&view).steps[0];
    assert_eq!(close.id, "close");
    // Composed from its data: the currency, `with` each omen, the side it
    // fills; never the blueprint's text.
    assert_eq!(
        close.instruction,
        format!(
            "Exalted Orb with {GREATER} and {DEXTRAL_EXALT} on the suffix side, for any modifier"
        )
    );
    assert_eq!(close.uses, ["Exalted Orb", GREATER, DEXTRAL_EXALT]);
    assert!(!close.instruction.contains("Close the finished side"));
    // A fill places no target.
    assert!(close.targets.is_empty());
    // Guiding, the current step says the same.
    let view = app.accept_craft();
    assert_eq!(step(&view).step.instruction, close.instruction);
    app.shutdown();
}

#[test]
fn two_prefixes_and_a_suffix_never_get_an_unrestricted_exalt() {
    // The owner's craft: two target prefixes and a target suffix, Armour
    // still missing. Both sides are open and the prefix side's one free
    // slot is reserved for Armour.
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = planned(tmp.path(), &shell, &belt("belt-2p1s"), &belt_targets());
    let routes = every_route(&app);
    let ids: Vec<&str> = routes.iter().map(|r| r.blueprint.as_str()).collect();
    for id in [
        CLOSE,
        "rare-clean-then-slam",
        "essence-desecrate-double-slam",
    ] {
        assert!(ids.contains(&id), "{id}: {ids:?}");
    }
    // On the item as it stands, no route's first step is an exalt that may
    // land on the prefix side's reserved slot.
    for r in &routes {
        let first = &r.steps[0];
        assert!(
            !unrestricted_exalt(&first.uses),
            "{}: {:?}",
            r.blueprint,
            first.uses
        );
    }
    // `close-a-side` closes the suffix side first (Greater and Dextral
    // Exaltation on a plain Exalted Orb); only then, with one side open,
    // the slam needs no side omen.
    let close = routes.iter().find(|r| r.blueprint == CLOSE).unwrap();
    let uses: Vec<&[String]> = close.steps.iter().map(|s| s.uses.as_slice()).collect();
    assert_eq!(
        uses,
        [
            &[
                "Exalted Orb".to_string(),
                GREATER.to_string(),
                DEXTRAL_EXALT.to_string()
            ][..],
            &["Perfect Exalted Orb".to_string()][..],
        ]
    );
    // `rare-clean-then-slam` slams the prefix side with Sinistral
    // Exaltation and no Greater (one slot is free there).
    let clean = routes
        .iter()
        .find(|r| r.blueprint == "rare-clean-then-slam")
        .unwrap();
    assert_eq!(
        clean.steps[0].uses,
        ["Perfect Exalted Orb", SINISTRAL_EXALT]
    );
    app.shutdown();

    // Negative control, the same check on the listed baseline: the retired
    // `fill-desecrate-exalt` (its test copy) names a plain Exalted Orb
    // first on this belt, which the check catches.
    let listed = Engine::load(&[app_root()
        .join("crates/ch-engine/tests/items/gear/fill-desecrate-exalt/fill-desecrate-exalt.toml")])
    .unwrap_or_else(|e| panic!("{e:?}"));
    let item = ch_app::ch_engine::ch_item::parse(&belt("belt-2p1s")).unwrap();
    let plan = listed.plan(&item, &belt_targets(), &[]).unwrap();
    let first = &plan.routes[0].steps[0];
    let action = first.action.as_ref().unwrap();
    assert_eq!(
        (action.uses.as_str(), action.omens.len()),
        ("exalted-orb", 0)
    );
    let names: Vec<String> = std::iter::once(action.name.clone())
        .chain(action.omen_names.iter().cloned())
        .collect();
    assert!(unrestricted_exalt(&names), "{names:?}");
}

#[test]
fn a_rare_with_one_target_gets_a_route() {
    // Issue #28: a rare ring holding one target (Mana) among three
    // modifiers that are not, three targets missing.
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = planned(
        tmp.path(),
        &shell,
        &slots_item("rare-one-target"),
        &one_target_targets(),
    );
    let view = app.choose_route("rare-clean-then-slam");
    assert!(view.craft.planning.as_ref().unwrap().can_accept);
    // Clean the prefix side with a plain annulment (the suffix side holds
    // no target to protect), then a Greater side slam for the two prefixes
    // and a side slam for the suffix.
    let uses: Vec<Vec<String>> = route(&view).steps.iter().map(|s| s.uses.clone()).collect();
    assert_eq!(
        uses,
        [
            vec![ANNUL.to_string()],
            vec![
                "Perfect Exalted Orb".to_string(),
                GREATER.to_string(),
                SINISTRAL_EXALT.to_string()
            ],
            vec!["Perfect Exalted Orb".to_string(), DEXTRAL_EXALT.to_string()],
        ]
    );
    let view = app.accept_craft();
    assert_eq!(guiding(&view).status, GuideStatusView::Active);
    assert_eq!(step(&view).step.id, "remove");
    app.shutdown();

    // Negative control: the listed baseline rejects this ring.
    let listed = Engine::load(&[app_root()
        .join("crates/ch-engine/tests/items/gear/fill-desecrate-exalt/fill-desecrate-exalt.toml")])
    .unwrap_or_else(|e| panic!("{e:?}"));
    let item = ch_app::ch_engine::ch_item::parse(&slots_item("rare-one-target")).unwrap();
    let plan = listed.plan(&item, &one_target_targets(), &[]).unwrap();
    assert!(plan.routes.is_empty(), "{:?}", plan.routes);
    assert_eq!(plan.rejected.len(), 1);
}

#[test]
fn a_state_ordered_hit_line_names_the_next_projected_step() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = planned(tmp.path(), &shell, &belt("belt-2p1s"), &belt_targets());
    app.choose_route(CLOSE);
    let view = app.accept_craft();
    let current = step(&view);
    assert_eq!(
        (current.step.id.as_str(), current.step.number),
        ("close", 1)
    );
    // The projection's next step, by its composed instruction.
    assert_eq!(
        current.on_hit,
        "On a hit: Perfect Exalted Orb on the prefix side for +# to Armour"
    );
    let view = app.grab_text(Some(&belt("belt-closed")));
    let current = step(&view);
    assert_eq!(
        (current.step.id.as_str(), current.step.number),
        ("slam-open-side", 2)
    );
    // The projection's last step.
    assert_eq!(current.on_hit, "On a hit: the craft is done");
    let view = app.grab_text(Some(&belt("belt-done")));
    assert_eq!(guiding(&view).status, GuideStatusView::Done);
    let numbers: Vec<usize> = guiding(&view).history.iter().map(|h| h.number).collect();
    assert_eq!(numbers, [1, 2]);
    app.shutdown();

    // A magic amulet on `magic-augment-essence`, Cold Resistance a target
    // too: once the essence made it rare no start of the route fits it, and
    // the projection made when the craft started still names the step
    // after it, every step so far having hit.
    let tmp = tempfile::tempdir().unwrap();
    let targets = vec![
        Target::new("prefix-to-maximum-mana", Some(6)),
        Target::new("prefix-to-maximum-life", Some(8)),
        Target::new("suffix-to-fire-resistance", Some(5)),
        Target::new("suffix-to-cold-resistance", Some(5)),
    ];
    let app = planned(tmp.path(), &shell, &slots_item("magic-one-good"), &targets);
    app.choose_route("magic-augment-essence");
    let view = app.accept_craft();
    assert_eq!(step(&view).step.id, "augment");
    let view = app.grab_text(Some(&slots_item("walks/augment-fire")));
    assert_eq!(step(&view).step.id, "essence");
    let hit = &step(&view).on_hit;
    assert!(
        hit.starts_with("On a hit: Perfect Exalted Orb with "),
        "{hit}"
    );
    assert!(
        hit.contains(DEXTRAL_EXALT) && hit.contains("Cold Resistance"),
        "{hit}"
    );
    let view = app.grab_text(Some(&slots_item("walks/augment-essence")));
    let current = step(&view);
    assert_eq!(current.step.id, "side-slam");
    assert_eq!(current.on_hit, "On a hit: the craft is done");
    app.shutdown();
}

#[test]
fn a_state_ordered_miss_line_says_the_item_picks_the_next_step() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = planned(tmp.path(), &shell, &belt("belt-2p1s"), &belt_targets());
    app.choose_route(CLOSE);
    app.accept_craft();
    let view = app.grab_text(Some(&belt("belt-closed")));
    let picked = format!("On a miss: {PICKED_AFTER_MISS}");
    assert_eq!(step(&view).on_miss, picked);
    // The miss: the item picks the removal, numbered as the craft's third
    // step whatever its place in the blueprint (first).
    let view = app.grab_text(Some(&belt("belt-slam-miss")));
    let current = step(&view);
    assert_eq!(
        (
            current.step.id.as_str(),
            current.step.number,
            current.try_number
        ),
        ("remove", 3, 1)
    );
    assert_eq!(current.on_miss, picked);
    let last = guiding(&view).history.last().unwrap();
    assert_eq!((last.number, last.result), (2, OutcomeWord::Miss));
    app.shutdown();

    // Control: a listed route's miss line follows its transition.
    let tmp = tempfile::tempdir().unwrap();
    let app = planned(tmp.path(), &shell, &ring("01-start"), &ring_targets());
    app.choose_route(WHITTLE);
    let view = app.accept_craft();
    let miss = &step(&view).on_miss;
    assert!(miss.starts_with("On a miss: "), "{miss}");
    assert_ne!(*miss, picked);
    app.shutdown();
}

#[test]
fn a_state_ordered_step_shows_no_fallback_for_the_engines_own_pick() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    // The belt's slam missed: the engine passes the whittle over (it is not
    // certain) and takes the Sinistral annulment, its second way. Nothing
    // is blocked: no fallback line.
    let missed = |dir: &str, blocked: &[&str]| {
        let app = planned(
            &tmp.path().join(dir),
            &shell,
            &belt("belt-2p1s"),
            &belt_targets(),
        );
        app.choose_route(CLOSE);
        if !blocked.is_empty() {
            app.set_craft_blocked(&lines(blocked));
        }
        app.accept_craft();
        app.grab_text(Some(&belt("belt-closed")));
        let view = app.grab_text(Some(&belt("belt-slam-miss")));
        app.shutdown();
        view
    };
    let view = missed("own", &[]);
    let s = &step(&view).step;
    assert_eq!(
        (s.id.as_str(), s.uses.as_slice()),
        (
            "remove",
            &[ANNUL.to_string(), SINISTRAL_ANNUL.to_string()][..]
        )
    );
    assert_eq!(s.fallback, None);

    // Issue #28's ring: the plain annulment, its third way (the other side
    // holds no target for a side omen to protect): no fallback line.
    let app = planned(
        &tmp.path().join("ring"),
        &shell,
        &slots_item("rare-one-target"),
        &one_target_targets(),
    );
    let view = app.choose_route("rare-clean-then-slam");
    let remove = &route(&view).steps[0];
    assert_eq!(remove.uses, [ANNUL]);
    assert_eq!(remove.fallback, None);
    app.shutdown();

    // The whittled ring with Chaos Orbs blocked: the whittle names a
    // blocked Chaos Orb, but the engine would pass it over anyway (no
    // annulment could take a target there): still no fallback line.
    let app = planned(
        &tmp.path().join("chaos"),
        &shell,
        &ring("01-start"),
        &ring_targets(),
    );
    let view = app.set_craft_blocked(&lines(&["chaos-orb"]));
    let r = route(&view);
    assert_eq!(r.blueprint, CLOSE);
    assert_eq!(
        (r.steps[0].id.as_str(), r.steps[0].uses.as_slice()),
        ("remove", &[ANNUL.to_string()][..])
    );
    assert!(
        r.steps.iter().all(|s| s.fallback.is_none()),
        "{:?}",
        r.steps
    );
    app.shutdown();

    // Positive control: with Omen of Sinistral Annulment blocked for the
    // craft, the plain annulment stands in for the side one, and says why.
    let view = missed("blocked", &["omen-of-sinistral-annulment"]);
    let s = &step(&view).step;
    assert_eq!(s.uses, [ANNUL]);
    assert_eq!(
        s.fallback.as_deref(),
        Some("Orb of Annulment instead of Orb of Annulment with Omen of Sinistral Annulment: Omen of Sinistral Annulment blocked")
    );
}

#[test]
fn a_removal_step_names_its_side_annulment() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = planned(tmp.path(), &shell, &belt("belt-2p1s"), &belt_targets());
    app.choose_route(CLOSE);
    app.accept_craft();
    app.grab_text(Some(&belt("belt-closed")));
    let view = app.grab_text(Some(&belt("belt-slam-miss")));
    // The suffix side holds fire, a target: the annulment is kept to the
    // blocked prefix side, and names only what it can take there that is
    // not wanted.
    let s = &step(&view).step;
    assert_eq!(
        s.instruction,
        format!(
            "{ANNUL} with {SINISTRAL_ANNUL} to remove 38(35-40)% increased Flask Life Recovery rate; it takes one modifier at random"
        )
    );
    assert_eq!(s.uses, [ANNUL, SINISTRAL_ANNUL]);
    app.shutdown();

    // Control: on issue #28's ring the suffix side holds no target, so no
    // side omen; the plain annulment names every unwanted modifier.
    let tmp = tempfile::tempdir().unwrap();
    let app = planned(
        tmp.path(),
        &shell,
        &slots_item("rare-one-target"),
        &one_target_targets(),
    );
    let view = app.choose_route("rare-clean-then-slam");
    let remove = &route(&view).steps[0];
    assert!(
        !remove
            .uses
            .iter()
            .any(|u| u.contains("Annulment") && u != ANNUL),
        "{:?}",
        remove.uses
    );
    assert!(
        remove.instruction.contains("Dexterity"),
        "{}",
        remove.instruction
    );
    app.shutdown();
}

#[test]
fn the_route_source_carries_no_web_address() {
    // The v2 blueprints' sources name their guides' addresses; the route
    // view sends the attribution without them (no view carries an address).
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = planned(tmp.path(), &shell, &belt("belt-2p1s"), &belt_targets());
    let view = app.choose_route(CLOSE);
    let source = &route(&view).source;
    assert!(
        source.starts_with("maxroll, How to Craft in Path of Exile 2, method 3 and the domistae"),
        "{source}"
    );
    assert!(
        source.ends_with("fetched 2026-10-29") && !source.contains("://"),
        "{source}"
    );
    app.shutdown();

    assert_eq!(
        source_line(
            "maxroll, How to Craft, method 1 (https://maxroll.gg/poe2/x) and the codex \
             (https://domistae.github.io/y.html), fetched 2026-10-29"
        ),
        "maxroll, How to Craft, method 1 and the codex, fetched 2026-10-29"
    );
    // A bracket without an address stays, a bare address goes, and a
    // source with none is kept as written.
    assert_eq!(
        source_line("issue #28 (a rare with one wanted modifier); https://x.y/z maxroll"),
        "issue #28 (a rare with one wanted modifier); maxroll"
    );
    let plain = "Discord post 6 (and post 5's note), owner's paste, 2026-10-28";
    assert_eq!(source_line(plain), plain);
}
