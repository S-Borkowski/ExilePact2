//! The owner's remarks after Live 1, through `App` with the shipped engine:
//! a whittle-or-annul step names the whittle or the annulment by the
//! modifier levels and is judged against the way it named; a removal step
//! names the modifiers it removes; the branch lines name the next step by
//! its composed instruction; the status lines name no blueprint; and an
//! outcome's word is said once.
//!
//! The ring is the one `ch-engine`'s seed walk takes on the fill, desecrate,
//! slam route (`crates/ch-engine/tests/items/gear/fill-desecrate-exalt/`),
//! with its targets (life T3, fire T4, cold T4, chaos T3). Modifier levels
//! on it (`data/classes/rings.json`): mana T5 54, life T3 38, accuracy T3
//! 48, fire T4 48, accuracy T7 ("Reliable") 11.

mod common;

use ch_app::{CraftMode, GuideStatusView, OutcomeWord, StatusKind, View};
use common::*;

const ANNUL: &str = "Orb of Annulment";
const WHITTLING: [&str; 2] = ["Chaos Orb", "Omen of Whittling"];

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

/// A started app planning `03-fill-fire` with the fill targets on the fill
/// route, with `blocked` for this craft.
fn fill_planned(
    dir: &std::path::Path,
    shell: &FakeShell,
    blocked: &[&str],
) -> std::sync::Arc<ch_app::App> {
    let app = start(dir, shell);
    app.grab_text(Some(&fill("03-fill-fire")));
    app.set_targets(&fill_targets());
    app.choose_route(FILL);
    if !blocked.is_empty() {
        app.set_craft_blocked(&lines(blocked));
    }
    app
}

/// The fill route started on `03-fill-fire` and its exalt missed (it rolled
/// accuracy): the current step is `fill-clean`.
fn missed_exalt(
    dir: &std::path::Path,
    shell: &FakeShell,
    miss: &str,
) -> std::sync::Arc<ch_app::App> {
    let app = fill_planned(dir, shell, &[]);
    let view = app.accept_craft();
    assert_eq!(step(&view).step.id, "fill");
    let view = app.grab_text(Some(miss));
    assert_eq!(
        guiding(&view).history.last().unwrap().result,
        OutcomeWord::Miss
    );
    assert_eq!(step(&view).step.id, "fill-clean");
    app
}

/// `07-fill-miss` with its Accuracy as "Reliable" T7 (level 11), the lowest
/// modifier on the ring.
fn reliable_miss() -> String {
    let text = fill("07-fill-miss")
        .replace(
            "{ Prefix Modifier \"Steady\" (Tier: 3) — Attack }",
            "{ Prefix Modifier \"Reliable\" (Tier: 7) — Attack }",
        )
        .replace(
            "+200(168-236) to Accuracy Rating",
            "+45(33-60) to Accuracy Rating",
        );
    assert!(text.contains("Reliable") && text.contains("+45(33-60)"));
    text
}

#[test]
fn a_missed_exalt_above_the_lowest_level_is_annulled() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = missed_exalt(tmp.path(), &shell, &fill("07-fill-miss"));
    let view = app.get_view();
    let s = &step(&view).step;
    // Life (level 38) is the lowest and a target: a whittle would take it,
    // so the app names the annulment, with the engine's odds for it.
    assert_eq!(s.uses, [ANNUL]);
    assert!(s.instruction.starts_with(ANNUL), "{}", s.instruction);
    let odds = s.odds.as_deref().unwrap();
    assert!(odds.contains("2 of 4"), "{odds}");
    assert_eq!(s.fallback, None);
    assert_eq!(step(&view).try_number, 1);
    app.shutdown();
}

#[test]
fn a_missed_exalt_at_the_lowest_level_is_whittled() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = missed_exalt(tmp.path(), &shell, &reliable_miss());
    let view = app.get_view();
    let s = &step(&view).step;
    // The new Accuracy (level 11) is the lowest and unwanted: the whittle,
    // certain, naming it.
    assert_eq!(s.uses, WHITTLING);
    assert!(
        s.odds.as_deref().unwrap().starts_with("certain"),
        "{:?}",
        s.odds
    );
    assert!(
        s.instruction.contains("Accuracy Rating"),
        "{}",
        s.instruction
    );
    assert!(!s.instruction.contains("maximum Life"), "{}", s.instruction);
    assert_eq!(s.fallback, None);
    app.shutdown();
}

#[test]
fn a_blocked_annulment_keeps_the_whittle() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = fill_planned(tmp.path(), &shell, &["orb-of-annulment"]);
    let view = app.accept_craft();
    assert_eq!(step(&view).step.id, "fill");
    let view = app.grab_text(Some(&fill("07-fill-miss")));
    let s = &step(&view).step;
    assert_eq!(s.id, "fill-clean");
    // The annulment is blocked for this craft: the whittle stays, with its
    // odds as they are (it would take life, a target).
    assert_eq!(s.uses, WHITTLING);
    assert!(
        s.odds.as_deref().unwrap().starts_with("no chance"),
        "{:?}",
        s.odds
    );
    assert_eq!(s.fallback, None);
    app.shutdown();
}

#[test]
fn a_removal_step_names_the_modifier_it_removes() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    // Planning on `03-fill-fire`: the route's list shows `fill-clean`
    // through the same choice. Life is the lowest there too: the
    // annulment, naming mana, the one unwanted modifier.
    let app = fill_planned(tmp.path(), &shell, &[]);
    let view = app.get_view();
    let route = view
        .craft
        .planning
        .as_ref()
        .unwrap()
        .preview
        .as_ref()
        .unwrap()
        .route
        .as_ref()
        .unwrap();
    assert_eq!(route.blueprint, FILL);
    let clean = route
        .steps
        .iter()
        .find(|s| s.id == "fill-clean")
        .expect("fill-clean in the route");
    assert_eq!(clean.uses, [ANNUL]);
    assert!(
        clean.instruction.contains("maximum Mana"),
        "{}",
        clean.instruction
    );
    assert!(
        !clean.instruction.contains("maximum Life"),
        "{}",
        clean.instruction
    );
    assert!(
        clean.odds.as_deref().unwrap().contains("1 of 3"),
        "{:?}",
        clean.odds
    );
    assert_eq!(clean.fallback, None);
    assert!(clean.targets.is_empty());

    // Guiding: the current `fill-clean` names mana and accuracy, the orb
    // taking one at random.
    app.accept_craft();
    let view = app.grab_text(Some(&fill("07-fill-miss")));
    let s = &step(&view).step;
    assert_eq!(s.id, "fill-clean");
    for name in ["maximum Mana", "Accuracy Rating", "at random"] {
        assert!(s.instruction.contains(name), "{name}: {}", s.instruction);
    }
    assert!(!s.instruction.contains("maximum Life"), "{}", s.instruction);
    assert!(!s.instruction.contains("else"), "{}", s.instruction);
    app.shutdown();
}

#[test]
fn a_miss_branch_to_a_removal_step_leaves_no_choice_to_the_player() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = fill_planned(tmp.path(), &shell, &[]);
    let view = app.accept_craft();
    let current = step(&view);
    assert_eq!(current.step.id, "fill");
    // The modifier a miss brings does not exist yet: the line says it is
    // removed next, and offers no whittle-or-annul choice.
    let miss = &current.on_miss;
    assert!(miss.starts_with("On a miss: "), "{miss}");
    assert!(miss.contains("removed"), "{miss}");
    for word in ["hittl", "nnul", "else", "Chaos Orb"] {
        assert!(!miss.contains(word), "{word}: {miss}");
    }
    app.shutdown();
}

#[test]
fn branch_lines_name_the_next_step_by_its_composed_instruction() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = missed_exalt(tmp.path(), &shell, &fill("07-fill-miss"));
    let view = app.get_view();
    // `fill-clean`'s hit goes back to the exalt, named by its currency and
    // the targets still missing, never the blueprint's text.
    let hit = &step(&view).on_hit;
    assert!(hit.starts_with("On a hit: Exalted Orb"), "{hit}");
    assert!(
        hit.contains("Cold Resistance") && hit.contains("Chaos Resistance"),
        "{hit}"
    );
    assert!(!hit.contains("Exalt a side"), "{hit}");
    // Its miss repeats it.
    assert_eq!(step(&view).on_miss, "On a miss: this step again");
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
    let app = fill_planned(tmp.path(), &shell, &[]);
    let view = app.get_view();
    let route = view
        .craft
        .planning
        .as_ref()
        .unwrap()
        .preview
        .as_ref()
        .unwrap()
        .route
        .as_ref()
        .unwrap();
    // The engine's `hardest` binding is chaos resistance (the seed walk's):
    // the desecrate line names it.
    let desecrate = route.steps.iter().find(|s| s.id == "desecrate").unwrap();
    assert!(
        desecrate.instruction.contains("Chaos Resistance"),
        "{}",
        desecrate.instruction
    );
    // Guiding on the exalt: its hit line names the desecration by the same
    // target, and the words "the hardest target" reach no one.
    let view = app.accept_craft();
    let hit = &step(&view).on_hit;
    assert!(
        hit.starts_with("On a hit: this step again until its condition holds, then "),
        "{hit}"
    );
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
    app.grab_text(Some(&fill("03-fill-fire")));
    app.set_targets(&fill_targets());
    let view = app.choose_route(FILL);
    let name = view
        .craft
        .planning
        .as_ref()
        .unwrap()
        .preview
        .as_ref()
        .unwrap()
        .route
        .as_ref()
        .unwrap()
        .name
        .clone();
    // Positive control: the route has a name to leave out.
    assert!(!name.is_empty());
    let (kind, line) = status(&view);
    assert_eq!(kind, StatusKind::Info);
    assert!(!line.contains(&name) && !line.contains(FILL), "{line}");
    assert!(line.starts_with("Route"), "{line}");
    let view = app.accept_craft();
    let (kind, line) = status(&view);
    assert_eq!(kind, StatusKind::Info);
    assert!(!line.contains(&name) && !line.contains(FILL), "{line}");
    assert!(line.contains("started"), "{line}");
    app.shutdown();
    // The diag lines still name the blueprint by id.
    let log = diag_lines(tmp.path());
    assert!(log.contains(&format!("craft: route {FILL}")), "{log:?}");
    assert!(log.contains(&format!("craft: accepted {FILL}")), "{log:?}");
}

#[test]
fn the_last_outcome_says_its_word_once() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = missed_exalt(tmp.path(), &shell, &fill("07-fill-miss"));
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
    let app = missed_exalt(tmp.path(), &shell, &fill("07-fill-miss"));
    // The annulment took accuracy, nothing added: a hit, back to the exalt.
    let annulled = without_mod(&fill("07-fill-miss"), "to Accuracy Rating");
    let view = app.grab_text(Some(&annulled));
    let g = guiding(&view);
    assert_eq!(g.status, GuideStatusView::Active);
    let h = g.history.last().unwrap();
    assert_eq!(
        (h.step.as_str(), h.result),
        ("fill-clean", OutcomeWord::Hit)
    );
    assert!(h.instruction.starts_with(ANNUL), "{}", h.instruction);
    assert_eq!(step(&view).step.id, "fill");
    app.shutdown();

    // Control, on a craft of its own: a whittle's change (accuracy out,
    // strength in; the whittle's hit in the engine's own seed walk) is not
    // what the annulment the app named does.
    let tmp = tempfile::tempdir().unwrap();
    let app = missed_exalt(tmp.path(), &shell, &fill("07-fill-miss"));
    let view = app.grab_text(Some(&fill("08-whittled")));
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
    app.grab_text(Some(&fill("03-fill-fire")));
    app.set_targets(&fill_targets());
    app.choose_route(FILL);
    app.accept_craft();
    let view = app.grab_text(Some(&fill("07-fill-miss")));
    let s = &step(&view).step;
    assert_eq!(
        (s.id.as_str(), s.uses.as_slice()),
        ("fill-clean", &[ANNUL.to_string()][..])
    );
    // The annulment's result, read twice alike from the screen: kept as a
    // hit, the next step the exalt.
    screen.show(tooltip_of(&without_mod(
        &fill("07-fill-miss"),
        "to Accuracy Rating",
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
        (
            "fill-clean",
            OutcomeWord::Hit,
            ch_app::OutcomeSource::Screen
        )
    );
    assert_eq!(step(&view).step.id, "fill");
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
    let app = missed_exalt(tmp.path(), &shell, &fill("07-fill-miss"));
    // The annulment took life instead: a miss, the same step again.
    let view = app.grab_text(Some(&without_mod(&fill("07-fill-miss"), "to maximum Life")));
    let g = guiding(&view);
    assert_eq!(g.status, GuideStatusView::Active);
    assert_eq!(g.history.last().unwrap().result, OutcomeWord::Miss);
    let current = step(&view);
    assert_eq!(
        (current.step.id.as_str(), current.try_number),
        ("fill-clean", 2)
    );
    // Decided again on the new item: accuracy and fire share level 48, so
    // the whittle is not certain: the annulment again, with no fallback
    // line (a list left holding Whittling would say "Orb of Annulment
    // instead of Chaos Orb: Omen of Whittling blocked").
    assert_eq!(current.step.uses, [ANNUL]);
    assert_eq!(current.step.fallback, None);
    app.shutdown();
}
