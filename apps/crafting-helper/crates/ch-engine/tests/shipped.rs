//! The blueprints embedded from `apps/crafting-helper/blueprints/`: exactly
//! the eight seeds still shipped, the four state-ordered v2 blueprints, the
//! four from the maxroll and Mobalytics guides and, since 0.2.0, the
//! owner's `fractured-essence-swap` (issue #35;
//! `fill-desecrate-exalt` is retired: its file is a test blueprint under
//! `tests/items/gear/fill-desecrate-exalt/`; `weapons-alloy-budget` is
//! retired outright, its two alloys being two crafted modifiers where 0.5
//! allows one), each passing the
//! structural check (`ch-blueprint`) and the data-backed one (`ch-engine`),
//! and together loading as the shipped engine. The walks of each seed are
//! in `seeds_gear.rs` and `seeds_jewels_weapons.rs`, the v2 ones' in
//! `seeds_v2.rs`, the guide ones' in `seeds_guides.rs`, the essence swap's
//! in `essence_swap.rs`, and the v2 ones' parity with the slot model in
//! `slots.rs`.

use ch_engine::{resolve, Engine};

/// The shipped blueprint ids, sorted (the embedded list is in file-name
/// order).
const SEEDS: [&str; 18] = [
    "amulets-resist-spirit",
    "body-armours-single-prefix",
    "boots-runic-ward",
    "caster-cast-speed",
    "close-a-side",
    "essence-desecrate-double-slam",
    "fracture-first",
    "fractured-essence-swap",
    "full-rare-desecration-rescue",
    "gloves-crit-melee",
    "jewellery-catalyst-exalt",
    "jewels-three-one-side",
    "magic-augment-essence",
    "rare-clean-then-slam",
    "rings-flat-attack",
    "rings-resist-whittle",
    "time-lost-jewel-radius",
    "weapons-physical-generic",
];

#[test]
fn every_shipped_blueprint_validates_and_resolves() {
    let files: Vec<&str> = ch_blueprint::shipped().iter().map(|s| s.file).collect();
    let expected: Vec<String> = SEEDS.iter().map(|id| format!("{id}.toml")).collect();
    assert_eq!(
        files, expected,
        "the embedded list is not the shipped blueprints"
    );

    // Every error of every file, so one failing run names them all.
    let mut errors: Vec<String> = Vec::new();
    let mut ids: Vec<String> = Vec::new();
    for result in ch_blueprint::check_shipped() {
        match result.and_then(resolve) {
            Ok(resolved) => ids.push(resolved.id().to_string()),
            Err(e) => errors.extend(e.iter().map(|d| d.to_string())),
        }
    }
    assert!(
        errors.is_empty(),
        "shipped blueprints with errors:\n{}",
        errors.join("\n")
    );
    assert_eq!(ids, SEEDS, "a shipped blueprint's id is not its file stem");

    let engine = Engine::shipped().unwrap_or_else(|errors| {
        let lines: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        panic!("the shipped engine does not load:\n{}", lines.join("\n"))
    });
    let loaded: Vec<&str> = engine.blueprints().iter().map(|r| r.id()).collect();
    assert_eq!(
        loaded, SEEDS,
        "the shipped engine does not hold the shipped blueprints"
    );
}
