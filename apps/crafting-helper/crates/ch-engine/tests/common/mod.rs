//! Helpers the engine's integration tests share: item texts under
//! `tests/items/`, the real fixtures under `ch-item/fixtures/real/`, and an
//! engine over test blueprints. Each test file uses only some of them.
#![allow(dead_code)]

use std::path::PathBuf;

use ch_engine::{Engine, Guide, Next, Outcome, StepView, Target};
use ch_item::Item;

/// `crates/ch-engine/`.
pub fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `tests/items/<rel>`.
pub fn items_path(rel: &str) -> PathBuf {
    crate_dir().join("tests").join("items").join(rel)
}

/// The text of `tests/items/<rel>`.
pub fn item_text(rel: &str) -> String {
    let path = items_path(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// Parses item text; panics on a text that is not one.
pub fn parse(text: &str) -> Item {
    ch_item::parse(text).unwrap_or_else(|e| panic!("not item text: {e}"))
}

/// The item in `tests/items/<rel>`.
pub fn item(rel: &str) -> Item {
    parse(&item_text(rel))
}

/// The text of a real fixture, `ch-item/fixtures/real/<name>`.
pub fn real_text(name: &str) -> String {
    let path = crate_dir()
        .join("..")
        .join("ch-item")
        .join("fixtures")
        .join("real")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// A real fixture, parsed.
pub fn real(name: &str) -> Item {
    parse(&real_text(name))
}

/// An engine over `tests/items/<rel>` blueprint files, in that order.
pub fn engine(rels: &[&str]) -> Engine {
    let paths: Vec<PathBuf> = rels.iter().map(|r| items_path(r)).collect();
    Engine::load(&paths).unwrap_or_else(|errors| {
        let lines: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        panic!("blueprints do not load:\n{}", lines.join("\n"))
    })
}

/// The shipped engine; panics with every error when it does not load.
pub fn shipped() -> Engine {
    Engine::shipped().unwrap_or_else(|errors| {
        let lines: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        panic!("shipped blueprints do not load:\n{}", lines.join("\n"))
    })
}

/// Targets from `(data modifier id, floor)` pairs.
pub fn targets(list: &[(&str, Option<u32>)]) -> Vec<Target> {
    list.iter()
        .map(|(id, floor)| Target::new(*id, *floor))
        .collect()
}

/// A blocked list from ids.
pub fn blocked(ids: &[&str]) -> Vec<String> {
    ids.iter().map(|s| s.to_string()).collect()
}

/// The step `next` names; panics on any other next.
pub fn next_step(next: &Next) -> &StepView {
    match next {
        Next::Step { step } => step,
        other => panic!("expected a next step, got {other:?}"),
    }
}

/// Starts `blueprint` on `item`; panics when it cannot start.
pub fn start(
    engine: &Engine,
    blueprint: &str,
    item: &Item,
    targets: &[Target],
    blocked: &[String],
) -> (Guide, StepView) {
    let started = engine
        .start(blueprint, item, targets, blocked)
        .unwrap_or_else(|e| panic!("{blueprint} does not start: {e}"));
    let step = next_step(&started.next).clone();
    (started.guide, step)
}

/// Observes one change; panics when the guide cannot answer.
pub fn observe(engine: &Engine, guide: &mut Guide, before: &Item, after: &Item) -> Outcome {
    engine
        .observe(guide, before, after)
        .unwrap_or_else(|e| panic!("observe failed: {e}"))
}
