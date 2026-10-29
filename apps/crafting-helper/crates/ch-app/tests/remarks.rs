//! The owner's remarks after Live 1, through `App` with the shipped engine:
//! a removal step names the whittle only when it is certain, else the
//! annulment, and is judged as the step it named; a removal step names the
//! modifiers it removes; the branch lines name the next step by its
//! composed instruction; the status lines name no blueprint; and an
//! outcome's word is said once.
//!
//! They drive the shipped state-ordered `close-a-side` over the owner's
//! 2P/1S belt (`tests/items/`, constructed), with its targets (Life T2,
//! Mana T2 and Armour T2, the missing one, as prefixes; Fire Resistance T3),
//! after the suffix side was closed with Cold Resistance T2 and Strength T3
//! and the slam on the open prefix side missed. Modifier levels on it
//! (`data/classes/belts.json`): Mana T2 54, Life T2 60, Fire T3 60, Strength
//! T3 66, Cold T2 71; the missed slam's Flask Life Recovery T1 75, or Flask
//! Mana Recovery T4 26. Issue #28's ring (`ch-engine`'s slot case
//! `rare-one-target`) is the route list's removal and the desecration.
//!
//! These moved off the retired `fill-desecrate-exalt`. On a state-ordered
//! route the engine's pick is final: the whittle-or-annul by level the app
//! makes for a listed blueprint, and its `a_blocked_annulment_keeps_the_whittle`,
//! are `craft.rs`'s unit tests, on that blueprint's test copy.

mod common;

use ch_app::craft::PICKED_AFTER_MISS;
use ch_app::{CraftMode, GuideStatusView, OutcomeWord, StatusKind, View};
use common::*;

const ANNUL: &str = "Orb of Annulment";
const SINISTRAL: &str = "Omen of Sinistral Annulment";
const WHITTLING: [&str; 2] = ["Chaos Orb", "Omen of Whittling"];
/// The missed slam's modifier, above the lowest level.
const FLASK_LIFE: &str = "increased Flask Life Recovery rate";

fn guiding(view: &View) -> &ch_app::GuidingView {
    assert_eq!(view.craft.mode, CraftMode::Guiding, "{:?}", view.status);
    view.craft.guiding.as_ref().expect("the guiding part")
}

fn step(view: &View) -> &ch_app::CurrentStepView {
    guiding(view).step.as_ref().expect("a current step")
}

fn status(view: &View) -> (StatusKind, String) {
    let s = view.status.clone().expect("a status line");
    (s.kind, s.text)
}

fn route(view: &View) -> &ch_app::RouteView {
    view.craft
        .planning
        .as_ref()
        .unwrap()
        .preview
        .as_ref()
        .unwrap()
        .route
        .as_ref()
        .expect("a route")
}

/// A started app planning the belt with its targets on `close-a-side`.
fn belt_planned(dir: &std::path::Path, shell: &FakeShell) -> std::sync::Arc<ch_app::App> {
    let app = start(dir, shell);
    app.grab_text(Some(&belt("belt-2p1s")));
    app.set_targets(&belt_targets());
    app.choose_route(CLOSE);
    app
}

/// The belt's craft started, its suffix side closed (a hit), and the slam
/// on the open prefix side missed into `miss`: the current step is
/// `remove`.
fn missed_slam(
    dir: &std::path::Path,
    shell: &FakeShell,
    miss: &str,
) -> std::sync::Arc<ch_app::App> {
    let app = belt_planned(dir, shell);
    let view = app.accept_craft();
    assert_eq!(step(&view).step.id, "close");
    let view = app.grab_text(Some(&belt("belt-closed")));
    assert_eq!(step(&view).step.id, "slam-open-side");
    let view = app.grab_text(Some(miss));
    assert_eq!(
        guiding(&view).history.last().unwrap().result,
        OutcomeWord::Miss
    );
    assert_eq!(step(&view).step.id, "remove");
    app
}

#[test]
fn a_missed_exalt_above_the_lowest_level_is_annulled() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = missed_slam(tmp.path(), &shell, &belt("belt-slam-miss"));
    let view = app.get_view();
    let s = &step(&view).step;
    // Mana (level 54), a target, is the lowest: a whittle would take it, so
    // the engine takes the annulment, kept to the prefix side by Sinistral
    // Annulment (the suffix side holds fire), with its odds.
    assert_eq!(s.uses, [ANNUL, SINISTRAL]);
    assert!(s.instruction.starts_with(ANNUL), "{}", s.instruction);
    let odds = s.odds.as_deref().unwrap();
    assert!(odds.contains("1 of 3"), "{odds}");
    assert_eq!(s.fallback, None);
    assert_eq!(step(&view).try_number, 1);
    app.shutdown();
}

#[test]
fn a_missed_exalt_at_the_lowest_level_is_whittled() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = missed_slam(tmp.path(), &shell, &belt("belt-slam-miss-low"));
    let view = app.get_view();
    let s = &step(&view).step;
    // The new Flask Mana Recovery (level 26) is the lowest and unwanted, and
    // an annulment could take a target: the whittle, certain, naming it.
    assert_eq!(s.uses, WHITTLING);
    assert!(
        s.odds.as_deref().unwrap().starts_with("certain"),
        "{:?}",
        s.odds
    );
    assert!(
        s.instruction.contains("Flask Mana Recovery"),
        "{}",
        s.instruction
    );
    for target in ["maximum Life", "maximum Mana"] {
        assert!(!s.instruction.contains(target), "{}", s.instruction);
    }
    assert_eq!(s.fallback, None);
    app.shutdown();
}

#[test]
fn a_removal_step_names_the_modifier_it_removes() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    // Planning issue #28's ring on `rare-clean-then-slam`: the route's list
    // starts with the removal, a plain annulment (the suffix side holds no
    // target), naming the three unwanted modifiers and not mana, a target.
    let app = start(tmp.path(), &shell);
    app.grab_text(Some(&slots_item("rare-one-target")));
    app.set_targets(&one_target_targets());
    let view = app.choose_route("rare-clean-then-slam");
    let remove = &route(&view).steps[0];
    assert_eq!(remove.id, "remove");
    assert_eq!(remove.uses, [ANNUL]);
    for name in ["Accuracy Rating", "Cold Resistance", "Dexterity"] {
        assert!(
            remove.instruction.contains(name),
            "{name}: {}",
            remove.instruction
        );
    }
    assert!(
        !remove.instruction.contains("maximum Mana"),
        "{}",
        remove.instruction
    );
    assert!(
        remove.odds.as_deref().unwrap().contains("3 of 4"),
        "{:?}",
        remove.odds
    );
    assert_eq!(remove.fallback, None);
    assert!(remove.targets.is_empty());
    app.shutdown();

    // Guiding the belt: the current `remove` names the missed slam's
    // modifier, the orb taking one at random.
    let tmp = tempfile::tempdir().unwrap();
    let app = missed_slam(tmp.path(), &shell, &belt("belt-slam-miss"));
    let view = app.get_view();
    let s = &step(&view).step;
    for name in [FLASK_LIFE, "at random"] {
        assert!(s.instruction.contains(name), "{name}: {}", s.instruction);
    }
    for target in ["maximum Life", "maximum Mana"] {
        assert!(!s.instruction.contains(target), "{}", s.instruction);
    }
    assert!(!s.instruction.contains("else"), "{}", s.instruction);
    app.shutdown();
}

#[test]
fn a_miss_branch_to_a_removal_step_leaves_no_choice_to_the_player() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = belt_planned(tmp.path(), &shell);
    app.accept_craft();
    let view = app.grab_text(Some(&belt("belt-closed")));
    let current = step(&view);
    assert_eq!(current.step.id, "slam-open-side");
    // A miss leads to the removal, but the modifier it brings does not
    // exist yet: the line says the item picks the next step, and offers no
    // whittle-or-annul choice.
    let miss = &current.on_miss;
    assert_eq!(*miss, format!("On a miss: {PICKED_AFTER_MISS}"));
    for word in ["hittl", "nnul", "else", "Chaos Orb"] {
        assert!(!miss.contains(word), "{word}: {miss}");
    }
    app.shutdown();
}

#[test]
fn branch_lines_name_the_next_step_by_its_composed_instruction() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = missed_slam(tmp.path(), &shell, &belt("belt-slam-miss"));
    // The annulment took life instead: `remove` again, and its hit goes on
    // to the slam, named by its currency, omens and the targets now
    // missing, never the blueprint's text.
    let view = app.grab_text(Some(&without_mod(
        &belt("belt-slam-miss"),
        "to maximum Life",
    )));
    assert_eq!(step(&view).step.id, "remove");
    let hit = &step(&view).on_hit;
    assert!(hit.starts_with("On a hit: Perfect Exalted Orb"), "{hit}");
    assert!(
        hit.contains("maximum Life") && hit.contains("Armour"),
        "{hit}"
    );
    assert!(!hit.contains("Exalt the one open side"), "{hit}");
    // Its miss: the item picks the next step.
    assert_eq!(
        step(&view).on_miss,
        format!("On a miss: {PICKED_AFTER_MISS}")
    );
    app.shutdown();

    // Control: the whittling ring's first step, whose hit goes to the
    // whittle, names it by its currency and omen.
    let tmp = tempfile::tempdir().unwrap();
    let app = start(tmp.path(), &shell);
    app.grab_text(Some(&ring("01-start")));
    app.set_targets(&ring_targets());
    app.choose_route(WHITTLE);
    let view = app.accept_craft();
    let hit = &step(&view).on_hit;
    assert!(
        hit.starts_with("On a hit: Chaos Orb with Omen of Whittling for "),
        "{hit}"
    );
    assert!(!hit.contains("until one target is left"), "{hit}");
    app.shutdown();
}

#[test]
fn the_desecrate_step_names_the_hardest_target() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);
    app.grab_text(Some(&slots_item("rare-one-target")));
    app.set_targets(&one_target_targets());
    let view = app.choose_route("essence-desecrate-double-slam");
    // The engine's `hardest` binding is chaos resistance (spawn weight
    // 250): the desecrate line names it, on its side, with its omen.
    let desecrate = route(&view)
        .steps
        .iter()
        .find(|s| s.id == "desecrate")
        .unwrap();
    assert!(
        desecrate.instruction.contains("Chaos Resistance"),
        "{}",
        desecrate.instruction
    );
    assert!(
        desecrate
            .uses
            .iter()
            .any(|u| u == "Omen of Dextral Necromancy"),
        "{:?}",
        desecrate.uses
    );
    // Guiding on the removal before it: its hit line names the desecration
    // by the same target, and the words "the hardest target" reach no one.
    let view = app.accept_craft();
    assert_eq!(step(&view).step.id, "remove");
    let hit = &step(&view).on_hit;
    assert!(hit.contains("Necromancy"), "{hit}");
    assert!(hit.contains("Chaos Resistance"), "{hit}");
    // (The view keeps the route's name for the diag lines; the pages do
    // not render it.)
    let sent = serde_json::to_string(&view).unwrap();
    assert!(!sent.contains("hardest target"), "{sent}");
    app.shutdown();
}

#[test]
fn status_lines_do_not_name_the_blueprint() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);
    app.grab_text(Some(&belt("belt-2p1s")));
    app.set_targets(&belt_targets());
    let view = app.choose_route(CLOSE);
    let name = route(&view).name.clone();
    // Positive control: the route has a name to leave out.
    assert!(!name.is_empty());
    let (kind, line) = status(&view);
    assert_eq!(kind, StatusKind::Info);
    assert!(!line.contains(&name) && !line.contains(CLOSE), "{line}");
    assert!(line.starts_with("Route"), "{line}");
    let view = app.accept_craft();
    let (kind, line) = status(&view);
    assert_eq!(kind, StatusKind::Info);
    assert!(!line.contains(&name) && !line.contains(CLOSE), "{line}");
    assert!(line.contains("started"), "{line}");
    app.shutdown();
    // The diag lines still name the blueprint by id.
    let log = diag_lines(tmp.path());
    assert!(log.contains(&format!("craft: route {CLOSE}")), "{log:?}");
    assert!(log.contains(&format!("craft: accepted {CLOSE}")), "{log:?}");
}

#[test]
fn the_last_outcome_says_its_word_once() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = missed_slam(tmp.path(), &shell, &belt("belt-slam-miss"));
    let view = app.get_view();
    let last = guiding(&view).history.last().unwrap();
    assert_eq!(last.result, OutcomeWord::Miss);
    // The summary carries no word: the pages show the result's own.
    for word in ["Hit", "Miss", "Unexpected"] {
        assert!(!last.summary.starts_with(word), "{}", last.summary);
    }
    // The status line, which shows the summary alone, says it once.
    let (kind, line) = status(&view);
    assert_eq!(kind, StatusKind::Info);
    assert_eq!(line, format!("Miss: {}.", last.summary));
    assert_eq!(line.matches("Miss").count(), 1, "{line}");
    app.shutdown();
}

#[test]
fn an_annulment_the_app_named_is_judged_as_an_annulment() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = missed_slam(tmp.path(), &shell, &belt("belt-slam-miss"));
    // The annulment took the missed slam's modifier, nothing added: a hit,
    // on to the slam again.
    let annulled = without_mod(&belt("belt-slam-miss"), FLASK_LIFE);
    let view = app.grab_text(Some(&annulled));
    let g = guiding(&view);
    assert_eq!(g.status, GuideStatusView::Active);
    let h = g.history.last().unwrap();
    assert_eq!((h.step.as_str(), h.result), ("remove", OutcomeWord::Hit));
    assert!(h.instruction.starts_with(ANNUL), "{}", h.instruction);
    assert_eq!(step(&view).step.id, "slam-open-side");
    app.shutdown();

    // Control, on a craft of its own: a whittle's change (the modifier out,
    // another in) is not what the annulment it named does.
    let tmp = tempfile::tempdir().unwrap();
    let app = missed_slam(tmp.path(), &shell, &belt("belt-slam-miss"));
    let rerolled = belt("belt-slam-miss").replace(
        "{ Prefix Modifier \"Regenerating\" (Tier: 1) }\n38(35-40)% increased Flask Life Recovery rate",
        "{ Prefix Modifier \"Heartening\" (Tier: 4) }\n20(17-22)% increased Flask Mana Recovery rate",
    );
    assert!(!rerolled.contains(FLASK_LIFE));
    let view = app.grab_text(Some(&rerolled));
    let g = guiding(&view);
    assert_eq!(g.history.last().unwrap().result, OutcomeWord::Unexpected);
    assert_eq!(g.status, GuideStatusView::Replan);
    app.shutdown();
}

#[test]
fn an_annulment_read_from_the_screen_is_judged_as_an_annulment() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let screen = FakeScreen::in_front();
    let (app, ms) = start_watching(tmp.path(), &shell, &screen);
    app.grab_text(Some(&belt("belt-2p1s")));
    app.set_targets(&belt_targets());
    app.choose_route(CLOSE);
    app.accept_craft();
    app.grab_text(Some(&belt("belt-closed")));
    let view = app.grab_text(Some(&belt("belt-slam-miss")));
    let s = &step(&view).step;
    assert_eq!(
        (s.id.as_str(), s.uses.as_slice()),
        ("remove", &[ANNUL.to_string(), SINISTRAL.to_string()][..])
    );
    // The annulment's result, read twice alike from the screen: kept as a
    // hit, the next step the slam.
    screen.show(tooltip_of(&without_mod(
        &belt("belt-slam-miss"),
        FLASK_LIFE,
    )));
    for _ in 0..2 {
        app.tick_watch();
        ms.fetch_add(400, std::sync::atomic::Ordering::SeqCst);
    }
    let view = app.get_view();
    let g = guiding(&view);
    let h = g.history.last().unwrap();
    assert_eq!(
        (h.step.as_str(), h.result, h.source),
        ("remove", OutcomeWord::Hit, ch_app::OutcomeSource::Screen)
    );
    assert_eq!(step(&view).step.id, "slam-open-side");
    assert_eq!(
        status(&view).1,
        format!("Hit: {}. Read from the screen.", h.summary)
    );
    app.shutdown();
}

#[test]
fn a_missed_annulment_shows_the_step_again_without_a_fallback() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = missed_slam(tmp.path(), &shell, &belt("belt-slam-miss"));
    // The annulment took life instead: a miss, the same step again.
    let view = app.grab_text(Some(&without_mod(
        &belt("belt-slam-miss"),
        "to maximum Life",
    )));
    let g = guiding(&view);
    assert_eq!(g.status, GuideStatusView::Active);
    assert_eq!(g.history.last().unwrap().result, OutcomeWord::Miss);
    let current = step(&view);
    assert_eq!(
        (current.step.id.as_str(), current.try_number),
        ("remove", 2)
    );
    // Picked again on the new item: mana, a target, is now the lowest, so
    // the whittle is not certain: the Sinistral annulment again, with no
    // fallback line (the engine passed the whittle over by its own rule;
    // nothing is blocked).
    assert_eq!(current.step.uses, [ANNUL, SINISTRAL]);
    assert_eq!(current.step.fallback, None);
    app.shutdown();
}
