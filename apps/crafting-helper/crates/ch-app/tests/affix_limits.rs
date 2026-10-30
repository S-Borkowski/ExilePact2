//! The target choices against the item's prefix and suffix limits (issue
//! #32): the limit is the item's as a rare, read from the item (a Dusk or
//! Gloam implicit changes it); a side counts its chosen targets, a
//! desecrated choice on the side its data gives, plus the item's own
//! modifiers a route cannot take off (fractured, unrevealed) that no target
//! names, an unrevealed one only when no missing target on its side takes
//! its slot. An unchosen choice carries the reason it is disabled when
//! ticking it would take a full side past its limit; a chosen one, or an
//! unticked fractured target, never does. `set_targets` refuses a list over a
//! limit whole, and a re-grab drops the targets past it in list order.
//!
//! The items are the owner's belt (`tests/items/`) and `ch-engine`'s
//! whittled ring, edited here: a modifier taken off, a header made
//! fractured, the base swapped for a Dusk or Gloam Ring with the data's
//! implicit lines.

mod common;

use ch_app::ch_engine::Target;
use ch_app::{CraftMode, ModSide, PlanningView, StatusKind, TargetChoice, View};
use common::*;

const LIFE: &str = "prefix-to-maximum-life";
const MANA: &str = "prefix-to-maximum-mana";
const ARMOUR: &str = "prefix-to-armour";
const FLASK_LIFE: &str = "prefix-increased-flask-life-recovery-rate";
const EVASION: &str = "prefix-to-evasion-rating";
const FIRE: &str = "suffix-to-fire-resistance";
const COLD: &str = "suffix-to-cold-resistance";
const LIGHTNING: &str = "suffix-to-lightning-resistance";
const STRENGTH: &str = "suffix-to-strength";
/// A belt's desecrated prefix and suffix.
const MANA_AS_ARMOUR: &str = "desecrated-prefix-gain-of-maximum-mana-as-armour";
const BLEEDING: &str = "desecrated-suffix-reduced-duration-of-bleeding-on-you";

fn planning(view: &View) -> &PlanningView {
    assert_eq!(view.craft.mode, CraftMode::Planning, "{:?}", view.status);
    view.craft.planning.as_ref().expect("the planning part")
}

fn status(view: &View) -> (StatusKind, String) {
    let s = view.status.clone().expect("a status line");
    (s.kind, s.text)
}

/// The reason a choice is disabled, read from the view as the page reads
/// it (no reason: selectable).
fn disabled(c: &TargetChoice) -> Option<String> {
    serde_json::to_value(c)
        .unwrap()
        .get("disabled")
        .and_then(|v| v.as_str())
        .map(str::to_string)
}

fn all_choices(p: &PlanningView) -> impl Iterator<Item = &TargetChoice> {
    p.choices
        .prefixes
        .iter()
        .chain(&p.choices.suffixes)
        .chain(&p.choices.desecrated)
}

fn choice<'a>(p: &'a PlanningView, id: &str) -> &'a TargetChoice {
    all_choices(p)
        .find(|c| c.mod_id == id)
        .unwrap_or_else(|| panic!("{id} is not a choice"))
}

fn chosen(p: &PlanningView) -> Vec<String> {
    p.targets.iter().map(|t| t.mod_id.clone()).collect()
}

/// Every unchosen choice on `side` carries `reason`, every chosen one and
/// every choice on the other side none.
fn assert_side_full(p: &PlanningView, side: ModSide, reason: &str) {
    assert_side_full_except(p, side, reason, &[]);
}

/// As `assert_side_full`, but the choices in `selectable` carry no reason:
/// ticking one does not raise the side's count.
fn assert_side_full_except(p: &PlanningView, side: ModSide, reason: &str, selectable: &[&str]) {
    let ids = chosen(p);
    let mut full = 0;
    for c in all_choices(p) {
        let want = (c.side == side
            && !ids.contains(&c.mod_id)
            && !selectable.contains(&c.mod_id.as_str()))
        .then(|| reason.to_string());
        assert_eq!(disabled(c), want, "{}", c.mod_id);
        full += usize::from(want.is_some());
    }
    assert!(full > 0, "no choice on the full side");
}

fn assert_none_disabled(p: &PlanningView) {
    for c in all_choices(p) {
        assert_eq!(disabled(c), None, "{}", c.mod_id);
    }
}

fn targets(ids: &[&str]) -> Vec<Target> {
    ids.iter().map(|id| Target::new(*id, None)).collect()
}

/// One of `ch-engine`'s case items (`crates/ch-engine/tests/items/<dir>/`).
fn engine_item(dir: &str, name: &str) -> String {
    let path = app_root()
        .join("crates/ch-engine/tests/items")
        .join(dir)
        .join(format!("{name}.txt"));
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
        .replace("\r\n", "\n")
}

/// The belt with three prefixes on it (Life, Mana, Armour) and Fire
/// Resistance alone on the suffix side: the finished belt without its
/// Cold Resistance and Strength.
fn belt_three_prefixes() -> String {
    let done = belt("belt-done");
    without_mod(&without_mod(&done, "to Cold Resistance"), "to Strength")
}

/// The ring's `state` with its base swapped for `base` (`Dusk Ring` or
/// `Gloam Ring`) and its implicit for that base's data lines.
fn ring_as(state: &str, base: &str, implicit: &str) -> String {
    let text = ring(state).replace("Amethyst Ring", base).replace(
        "{ Implicit Modifier — Chaos, Resistance }\n+7(7-13)% to Chaos Resistance",
        &format!("{{ Implicit Modifier }}\n{implicit}"),
    );
    assert!(text.contains(base) && text.contains(implicit), "{text}");
    text
}

const DUSK: &str = "+1 Prefix Modifier allowed\n-1 Suffix Modifier allowed";
const GLOAM: &str = "-1 Prefix Modifier allowed\n+1 Suffix Modifier allowed";

#[test]
fn an_item_with_room_disables_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);
    // Two prefixes and a suffix on the belt, all three ticked: a prefix
    // and two suffixes still free.
    let view = app.grab_text(Some(&belt("belt-2p1s")));
    let p = planning(&view);
    assert_eq!(chosen(p), [LIFE, MANA, FIRE]);
    assert!(all_choices(p).count() > 0);
    assert_none_disabled(p);
    app.shutdown();
}

#[test]
fn prefixes_full_on_the_item_disable_every_other_prefix() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);
    // The grab ticks the three prefixes on the belt: the prefix side is
    // full, the suffix side has two free slots.
    let view = app.grab_text(Some(&belt_three_prefixes()));
    let p = planning(&view);
    assert_eq!(chosen(p), [LIFE, MANA, ARMOUR, FIRE]);
    assert_side_full(
        p,
        ModSide::Prefix,
        "Prefixes full: 3 on the item + 0 chosen = 3 of 3",
    );
    // The desecrated prefixes are disabled too, the desecrated suffixes not.
    assert!(disabled(choice(p, MANA_AS_ARMOUR)).is_some());
    assert_eq!(disabled(choice(p, BLEEDING)), None);
    assert_eq!(disabled(choice(p, COLD)), None);
    app.shutdown();
}

#[test]
fn two_on_the_item_and_one_chosen_disable_the_rest_with_the_reason() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);
    app.grab_text(Some(&belt("belt-2p1s")));
    // Life and Mana on the belt, Armour chosen and missing.
    let view = app.set_targets(&belt_targets());
    let p = planning(&view);
    assert_eq!(chosen(p), [LIFE, MANA, ARMOUR, FIRE]);
    assert_side_full(
        p,
        ModSide::Prefix,
        "Prefixes full: 2 on the item + 1 chosen = 3 of 3",
    );
    assert_eq!(
        disabled(choice(p, FLASK_LIFE)).as_deref(),
        Some("Prefixes full: 2 on the item + 1 chosen = 3 of 3")
    );
    app.shutdown();
}

#[test]
fn unticking_re_enables_the_side() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);
    let view = app.grab_text(Some(&belt_three_prefixes()));
    assert!(disabled(choice(planning(&view), FLASK_LIFE)).is_some());
    // Armour unticked: it is still on the belt, but a route can take it
    // off, so it no longer holds a slot.
    let view = app.set_targets(&targets(&[LIFE, MANA, FIRE]));
    let p = planning(&view);
    assert_eq!(chosen(p), [LIFE, MANA, FIRE]);
    assert_none_disabled(p);
    // Ticking it again fills the side again.
    let view = app.set_targets(&targets(&[LIFE, MANA, FIRE, ARMOUR]));
    assert_side_full(
        planning(&view),
        ModSide::Prefix,
        "Prefixes full: 3 on the item + 0 chosen = 3 of 3",
    );
    app.shutdown();
}

#[test]
fn an_implicit_changes_the_count() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);
    // Baseline: the Amethyst Ring with Mana on it and Life and Evasion
    // chosen fills its three prefixes; Strength and Fire leave a suffix.
    app.grab_text(Some(&ring("01-start")));
    let list = targets(&[MANA, LIFE, EVASION, STRENGTH, FIRE]);
    let view = app.set_targets(&list);
    let p = planning(&view);
    assert_side_full(
        p,
        ModSide::Prefix,
        "Prefixes full: 1 on the item + 2 chosen = 3 of 3",
    );

    // A Dusk Ring (one more prefix, one fewer suffix): the same list
    // leaves a prefix free and fills the two suffixes.
    let view = app.grab_text(Some(&ring_as("01-start", "Dusk Ring", DUSK)));
    let p = planning(&view);
    assert_eq!(p.item.base_type.as_deref(), Some("Dusk Ring"));
    assert_eq!(chosen(p), [MANA, LIFE, EVASION, STRENGTH, FIRE]);
    assert_side_full(
        p,
        ModSide::Suffix,
        "Suffixes full: 1 on the item + 1 chosen = 2 of 2",
    );
    app.shutdown();
}

#[test]
fn desecrated_choices_count_toward_their_side() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);
    app.grab_text(Some(&belt("belt-2p1s")));
    // A desecrated prefix chosen beside Life and Mana: the prefix side is
    // full, its other desecrated prefixes and the normal ones disabled.
    let view = app.set_targets(&targets(&[LIFE, MANA, MANA_AS_ARMOUR, FIRE]));
    let p = planning(&view);
    assert_eq!(chosen(p), [LIFE, MANA, MANA_AS_ARMOUR, FIRE]);
    assert_side_full(
        p,
        ModSide::Prefix,
        "Prefixes full: 2 on the item + 1 chosen = 3 of 3",
    );
    assert_eq!(disabled(choice(p, MANA_AS_ARMOUR)), None);
    // A desecrated suffix fills the suffix side the same way.
    let view = app.set_targets(&targets(&[LIFE, FIRE, COLD, BLEEDING]));
    let p = planning(&view);
    assert_side_full(
        p,
        ModSide::Suffix,
        "Suffixes full: 1 on the item + 2 chosen = 3 of 3",
    );
    app.shutdown();
}

#[test]
fn unremovable_modifiers_hold_a_slot_unticked() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);
    // Life fractured and unticked, Mana and Armour chosen: the fractured
    // modifier stays on the belt, so it holds a prefix.
    let fractured = belt("belt-2p1s").replace(
        "{ Prefix Modifier \"Athlete's\"",
        "{ Fractured Prefix Modifier \"Athlete's\"",
    );
    let view = app.grab_text(Some(&fractured));
    assert!(planning(&view).item.modifiers[0].fractured);
    let view = app.set_targets(&targets(&[MANA, ARMOUR, FIRE]));
    // Life itself stays selectable: it already holds its slot.
    assert_side_full_except(
        planning(&view),
        ModSide::Prefix,
        "Prefixes full: 2 on the item + 1 chosen = 3 of 3",
        &[LIFE],
    );
    // Negative control: the same belt unfractured frees the slot.
    app.grab_text(Some(&belt("belt-2p1s")));
    let view = app.set_targets(&targets(&[MANA, ARMOUR, FIRE]));
    assert_none_disabled(planning(&view));

    // An unrevealed desecrated suffix holds a suffix: the ring has Fire and
    // Cold beside it, both ticked by the grab. With no suffix target
    // missing it counts; a missing suffix target takes its slot (it is
    // revealed as that target), so every suffix stays selectable.
    app.set_targets(&[]);
    let view = app.grab_text(Some(&ring("04-desecrated")));
    let p = planning(&view);
    assert_eq!(chosen(p), [FIRE, COLD]);
    assert_none_disabled(p);
    let view = app.set_targets(&targets(&[FIRE, COLD, STRENGTH]));
    assert_side_full(
        planning(&view),
        ModSide::Suffix,
        "Suffixes full: 2 on the item + 1 chosen = 3 of 3",
    );
    app.shutdown();
}

#[test]
fn an_unrevealed_modifier_holds_the_slot_of_a_missing_target() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);
    // Life and Mana on the belt and an unrevealed desecrated prefix: Armour
    // is the target it waits to become, so it counts once, as the engine
    // counts it (`ch-engine`'s `slots.rs`).
    let text = engine_item("last-desecration", "belt-unrevealed");
    let view = app.grab_text(Some(&text));
    let p = planning(&view);
    assert_eq!(chosen(p), [LIFE, MANA, FIRE]);
    assert_eq!(disabled(choice(p, ARMOUR)), None);

    let view = app.set_targets(&belt_targets());
    assert_eq!(status(&view).0, StatusKind::Info, "{:?}", view.status);
    let p = planning(&view);
    assert_eq!(chosen(p), [LIFE, MANA, ARMOUR, FIRE]);
    assert_side_full(
        p,
        ModSide::Prefix,
        "Prefixes full: 2 on the item + 1 chosen = 3 of 3",
    );
    // A re-grab keeps every target.
    let view = app.grab_text(Some(&text));
    assert_eq!(chosen(planning(&view)), [LIFE, MANA, ARMOUR, FIRE]);
    assert!(!status(&view).1.contains("Dropped"), "{:?}", view.status);
    app.shutdown();
}

#[test]
fn an_unticked_fractured_target_stays_selectable() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);
    // Life fractured: the grab ticks it with Mana and Fire, Armour is added.
    let fractured = belt("belt-2p1s").replace(
        "{ Prefix Modifier \"Athlete's\"",
        "{ Fractured Prefix Modifier \"Athlete's\"",
    );
    app.grab_text(Some(&fractured));
    let view = app.set_targets(&targets(&[LIFE, MANA, ARMOUR, FIRE]));
    assert_side_full(
        planning(&view),
        ModSide::Prefix,
        "Prefixes full: 2 on the item + 1 chosen = 3 of 3",
    );
    // Life unticked still holds its slot, so ticking it again adds nothing:
    // it stays selectable and the list with it is taken.
    let view = app.set_targets(&targets(&[MANA, ARMOUR, FIRE]));
    let p = planning(&view);
    assert_eq!(disabled(choice(p, LIFE)), None);
    assert!(disabled(choice(p, FLASK_LIFE)).is_some());
    let view = app.set_targets(&targets(&[MANA, ARMOUR, FIRE, LIFE]));
    assert_eq!(status(&view).0, StatusKind::Info, "{:?}", view.status);
    assert_eq!(chosen(planning(&view)), [MANA, ARMOUR, FIRE, LIFE]);
    app.shutdown();
}

#[test]
fn set_targets_refuses_a_side_over_its_limit() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);
    app.grab_text(Some(&belt("belt-2p1s")));
    let before = chosen(planning(&app.get_view()));

    let view = app.set_targets(&targets(&[LIFE, MANA, ARMOUR, FLASK_LIFE, FIRE]));
    assert_eq!(chosen(planning(&view)), before);
    assert_eq!(
        status(&view),
        (
            StatusKind::Error,
            "Targets not changed: Prefixes over the limit: 2 on the item + 2 chosen = 4 of 3."
                .to_string()
        )
    );
    let view = app.set_targets(&targets(&[LIFE, FIRE, COLD, LIGHTNING, STRENGTH]));
    assert_eq!(chosen(planning(&view)), before);
    assert_eq!(
        status(&view).1,
        "Targets not changed: Suffixes over the limit: 1 on the item + 3 chosen = 4 of 3."
    );
    // At the limit, the list is taken.
    let view = app.set_targets(&targets(&[LIFE, MANA, ARMOUR, FIRE]));
    assert_eq!(chosen(planning(&view)), [LIFE, MANA, ARMOUR, FIRE]);
    assert_eq!(status(&view).0, StatusKind::Info);
    app.shutdown();
}

#[test]
fn a_regrab_drops_targets_past_the_limit() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(tmp.path(), &shell);
    app.grab_text(Some(&ring("01-start")));
    app.set_targets(&targets(&[MANA, LIFE, EVASION, STRENGTH]));
    // Negative control: the same ring again keeps all four.
    let view = app.grab_text(Some(&ring("01-start")));
    assert_eq!(chosen(planning(&view)), [MANA, LIFE, EVASION, STRENGTH]);
    assert!(!status(&view).1.contains("Dropped"), "{:?}", view.status);

    // A Gloam Ring has two prefixes: Evasion, the third in the list, goes.
    let view = app.grab_text(Some(&ring_as("01-start", "Gloam Ring", GLOAM)));
    let p = planning(&view);
    assert_eq!(chosen(p), [MANA, LIFE, STRENGTH]);
    assert!(
        status(&view)
            .1
            .ends_with("Dropped 1 target this item cannot roll."),
        "{:?}",
        view.status
    );
    assert_side_full(
        p,
        ModSide::Prefix,
        "Prefixes full: 1 on the item + 1 chosen = 2 of 2",
    );
    app.shutdown();
}
