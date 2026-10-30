//! The last-target desecration through the craft window (issue #32;
//! docs/models/crafting-slots-spec.md, "The last-target desecration"): on a
//! state-ordered route, a rare with no desecrated modifier and exactly one
//! target missing gets that target from a desecration on its side and a
//! reveal; where the desecration cannot be taken, the step taken instead
//! says why on its Fallback line, `<used, with its omens> instead of a
//! desecration: <reason>`.
//!
//! Items: the owner's 2P/1S belt (`tests/items/`, constructed), and
//! `ch-engine`'s last-desecration and slot case items
//! (`crates/ch-engine/tests/items/last-desecration/` and `.../slots/`,
//! constructed): the belt with a revealed desecrated target, the belt at
//! item level 64, and a body armour missing a prefix.

mod common;

use std::sync::Arc;

use ch_app::ch_engine::Target;
use ch_app::{App, CraftMode, RouteView, View};
use common::*;

const SINISTRAL_NECRO: &str = "Omen of Sinistral Necromancy";
const ECHOES: &str = "Omen of Abyssal Echoes";
const INSTEAD: &str = "instead of a desecration";

fn guiding(view: &View) -> &ch_app::GuidingView {
    assert_eq!(view.craft.mode, CraftMode::Guiding, "{:?}", view.status);
    view.craft.guiding.as_ref().expect("the guiding part")
}

fn step(view: &View) -> &ch_app::CurrentStepView {
    guiding(view).step.as_ref().expect("a current step")
}

fn route(view: &View) -> &RouteView {
    view.craft
        .planning
        .as_ref()
        .expect("planning")
        .preview
        .as_ref()
        .expect("a preview")
        .route
        .as_ref()
        .expect("a route")
}

/// `ch-engine`'s `tests/items/<dir>/<name>.txt`.
fn engine_item(dir: &str, name: &str) -> String {
    let path = app_root()
        .join("crates/ch-engine/tests/items")
        .join(dir)
        .join(format!("{name}.txt"));
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
        .replace("\r\n", "\n")
}

/// A started app planning `text` with `targets`, `blocked` for the craft,
/// on `close-a-side`.
fn planned(
    dir: &std::path::Path,
    shell: &FakeShell,
    text: &str,
    targets: &[Target],
    blocked: &[&str],
) -> Arc<App> {
    let app = start(dir, shell);
    app.grab_text(Some(text));
    app.set_targets(targets);
    if !blocked.is_empty() {
        app.set_craft_blocked(&lines(blocked));
    }
    app.choose_route(CLOSE);
    app
}

/// Every fallback line of the route and of the current step once started.
fn fallbacks(app: &App) -> Vec<Option<String>> {
    let view = app.get_view();
    let mut out: Vec<Option<String>> = route(&view)
        .steps
        .iter()
        .map(|s| s.fallback.clone())
        .collect();
    let view = app.accept_craft();
    out.push(step(&view).step.fallback.clone());
    out
}

// --- Issue #32, test 2: an item that already has its desecration ----------

/// The baseline: the belt's suffixes are Fire Resistance and a revealed
/// desecrated Strength and Dexterity, both targets, and Armour is missing.
/// The item already carries its one desecrated modifier: the route closes
/// and slams as before, and no line mentions a desecration.
#[test]
fn an_already_desecrated_item_keeps_its_route() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let mut targets = belt_targets();
    targets.push(Target::new(
        "desecrated-suffix-to-strength-and-dexterity",
        Some(1),
    ));
    let app = planned(
        tmp.path(),
        &shell,
        &engine_item("last-desecration", "belt-desecrated-target"),
        &targets,
        &[],
    );
    let view = app.get_view();
    let r = route(&view);
    let ids: Vec<&str> = r.steps.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(ids, ["close", "slam-open-side"]);
    for s in &r.steps {
        assert!(
            !s.uses.iter().any(|u| u.contains("Necromancy")),
            "{:?}",
            s.uses
        );
    }
    let lines = fallbacks(&app);
    assert!(
        lines.iter().flatten().all(|l| !l.contains(INSTEAD)),
        "{lines:?}"
    );
    app.shutdown();
}

// --- Issue #32, test 1: the last target is desecrated ---------------------

/// The owner's belt, Armour the one target missing: the route's last two
/// lines are the bone on the prefix side with Omen of Sinistral Necromancy
/// for Armour, then the reveal with Omen of Abyssal Echoes, each with its
/// odds in words, unknown. Neither has a Fallback line, and the guide's
/// first step is the desecration.
#[test]
fn the_last_step_is_a_desecration_and_reveal() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = planned(tmp.path(), &shell, &belt("belt-2p1s"), &belt_targets(), &[]);
    let view = app.get_view();
    let steps = &route(&view).steps;
    assert!(steps.len() >= 2, "{steps:?}");
    let [desecrate, reveal] = &steps[steps.len() - 2..] else {
        unreachable!()
    };
    assert_eq!(desecrate.id, "desecrate-last");
    assert!(
        desecrate.uses[0].ends_with("Collarbone"),
        "{:?}",
        desecrate.uses
    );
    assert_eq!(desecrate.uses[1..], [SINISTRAL_NECRO]);
    assert!(
        desecrate
            .instruction
            .starts_with("Use Omen of Sinistral Necromancy, then apply the ")
            && desecrate
                .instruction
                .contains("; the desecrated modifier will be a prefix"),
        "{}",
        desecrate.instruction
    );
    assert!(
        desecrate.targets.iter().any(|t| t.contains("Armour")),
        "{:?}",
        desecrate.targets
    );
    assert_eq!(reveal.id, "reveal-last");
    assert_eq!(reveal.uses.last().map(String::as_str), Some(ECHOES));
    for s in [desecrate, reveal] {
        let odds = s.odds.as_deref().unwrap_or_default();
        assert!(odds.starts_with("odds unknown: "), "{}: {odds}", s.id);
        assert_eq!(s.fallback, None, "{}", s.id);
    }

    let view = app.accept_craft();
    let current = step(&view);
    assert_eq!(current.step.id, "desecrate-last");
    assert_eq!(current.step.uses, desecrate.uses);
    assert_eq!(current.step.fallback, None);
    app.shutdown();
}

// --- Where the desecration cannot be taken ---------------------------------

/// The step taken instead of the desecration says why on its Fallback line,
/// in the route and on the current step: the Necromancy omen blocked, no
/// desecrated modifier on the side for the class or at the item level; with
/// a blocked alternative of the step's own passed over too, the
/// desecration's reason comes first.
#[test]
fn the_desecration_fallback_says_why() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    const CLOSE_USES: &str =
        "Exalted Orb with Omen of Greater Exaltation and Omen of Dextral Exaltation";

    // Omen of Sinistral Necromancy blocked: the close and the slam, each
    // naming the blocked omen.
    let app = planned(
        &tmp.path().join("omen"),
        &shell,
        &belt("belt-2p1s"),
        &belt_targets(),
        &["omen-of-sinistral-necromancy"],
    );
    let why = "Omen of Sinistral Necromancy blocked";
    assert_eq!(
        fallbacks(&app),
        [
            Some(format!("{CLOSE_USES} {INSTEAD}: {why}")),
            Some(format!("Perfect Exalted Orb {INSTEAD}: {why}")),
            Some(format!("{CLOSE_USES} {INSTEAD}: {why}")),
        ]
    );
    app.shutdown();

    // Negative control: the other side's Necromancy omen blocked changes
    // nothing; the belt desecrates, with no Fallback line.
    let app = planned(
        &tmp.path().join("dextral"),
        &shell,
        &belt("belt-2p1s"),
        &belt_targets(),
        &["omen-of-dextral-necromancy"],
    );
    let view = app.get_view();
    assert_eq!(route(&view).steps[0].id, "desecrate-last");
    assert!(fallbacks(&app).iter().all(Option::is_none));
    app.shutdown();

    // The belt at item level 64: no belt desecrated modifier below 65.
    let low_targets = [
        ("prefix-to-maximum-life", 2),
        ("prefix-to-maximum-mana", 2),
        ("prefix-to-armour", 4),
        ("suffix-to-fire-resistance", 3),
    ]
    .iter()
    .map(|(id, floor)| Target::new(*id, Some(*floor)))
    .collect::<Vec<_>>();
    let app = planned(
        &tmp.path().join("level"),
        &shell,
        &engine_item("last-desecration", "belt-2p1s-level-64"),
        &low_targets,
        &[],
    );
    let why = "no desecrated prefix modifier on Belts at item level 64";
    let lines = fallbacks(&app);
    assert_eq!(
        lines[0].as_deref(),
        Some(format!("{CLOSE_USES} {INSTEAD}: {why}").as_str())
    );
    assert_eq!(lines.last().unwrap(), &lines[0]);
    app.shutdown();

    // A body armour missing a prefix: the class has no desecrated prefix.
    let armour_targets = [
        ("prefix-to-maximum-life", 5),
        ("prefix-increased-armour", 2),
        ("prefix-to-spirit", 3),
        ("suffix-to-fire-resistance", 3),
    ]
    .iter()
    .map(|(id, floor)| Target::new(*id, Some(*floor)))
    .collect::<Vec<_>>();
    let app = planned(
        &tmp.path().join("class"),
        &shell,
        &slots_item("body-armour-prefix-no-pool"),
        &armour_targets,
        &[],
    );
    let why = "no desecrated prefix modifier on Body Armours at item level 82";
    let lines = fallbacks(&app);
    assert!(!lines.is_empty());
    for line in &lines {
        let line = line.as_deref().unwrap_or_default();
        assert!(line.ends_with(&format!("{INSTEAD}: {why}")), "{line}");
    }
    app.shutdown();

    // Both reasons: the Necromancy omen and the Exalted Orb blocked, so the
    // close takes a Greater Exalted Orb; the desecration's line first.
    let app = planned(
        &tmp.path().join("both"),
        &shell,
        &belt("belt-2p1s"),
        &belt_targets(),
        &["omen-of-sinistral-necromancy", "exalted-orb"],
    );
    let used = "Greater Exalted Orb with Omen of Greater Exaltation and Omen of Dextral Exaltation";
    let view = app.get_view();
    assert_eq!(
        route(&view).steps[0].fallback.as_deref(),
        Some(
            format!(
                "{used} {INSTEAD}: Omen of Sinistral Necromancy blocked; \
                 {used} instead of Exalted Orb with Omen of Dextral Exaltation: Exalted Orb blocked"
            )
            .as_str()
        )
    );
    app.shutdown();
}
