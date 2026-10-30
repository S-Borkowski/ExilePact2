//! The engine as the craft window (M5) will reach it: through `ch_app`'s
//! re-exports, the shipped blueprints load and plan a route for a real
//! item, and the whole catalogue, the blocked list's limit, passes the
//! settings' rules unchanged.

use std::path::PathBuf;

use ch_app::ch_engine::{catalogue, Engine, Target};
use ch_app::settings::blocked_list;
use ch_app::{ch_item, MAX_ID_CHARS};
use serde_json::Value;

/// The real Amethyst Ring (`ch-item/fixtures/real/`), made Rare: the
/// rarity line changed and a name line added, nothing else.
fn amethyst_ring_made_rare() -> ch_item::Item {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("ch-item")
        .join("fixtures")
        .join("real")
        .join("rings-normal-implicit-advanced.txt");
    // CRLF read as LF: a checkout with `core.autocrlf` writes it CRLF.
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
        .replace("\r\n", "\n");
    assert!(text.contains("Rarity: Normal\nAmethyst Ring\n"), "{text}");
    let rare = text.replace(
        "Rarity: Normal\nAmethyst Ring\n",
        "Rarity: Rare\nDoom Loop\nAmethyst Ring\n",
    );
    ch_item::parse(&rare).unwrap_or_else(|e| panic!("{e:?}"))
}

#[test]
fn engine_is_reexported_with_the_shipped_blueprints() {
    let engine = Engine::shipped().unwrap_or_else(|errors| {
        let lines: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        panic!("shipped blueprints do not load:\n{}", lines.join("\n"))
    });
    // The nine seeds still shipped and the four state-ordered blueprints.
    assert_eq!(engine.blueprints().len(), 13);

    let ring = amethyst_ring_made_rare();
    let targets = [
        Target::new("prefix-to-maximum-life", None),
        Target::new("suffix-to-fire-resistance", None),
    ];
    let plan = engine
        .plan(&ring, &targets, &[])
        .unwrap_or_else(|why| panic!("the ring is refused: {why:?}"));
    assert_eq!(plan.class, "Rings");
    assert!(
        !plan.routes.is_empty(),
        "no route for the ring; rejected: {:?}",
        plan.rejected
    );
    assert!(plan
        .routes
        .iter()
        .all(|r| engine.blueprint(&r.blueprint).is_some()));
    // The plan is what M5 renders: it serialises.
    assert!(serde_json::to_string(&plan).is_ok());
}

#[test]
fn catalogue_ids_fit_the_settings_limits() {
    let ids: Vec<&str> = catalogue().iter().map(|c| c.id.as_str()).collect();
    assert!(!ids.is_empty());
    for id in &ids {
        assert!(
            id.chars().count() <= MAX_ID_CHARS,
            "{id} is longer than {MAX_ID_CHARS} characters"
        );
    }
    // The catalogue is the blocked list's limit: the whole of it is one
    // list, saved as given, so no id is trimmed, merged with another or
    // dropped by the normalisation.
    let list = Value::from(ids.clone());
    let saved = blocked_list(&list).unwrap_or_else(|why| panic!("{why}"));
    assert_eq!(saved, ids, "the normalisation changed an id");
    // Negative control: one id more than the catalogue is refused.
    let mut more = ids.clone();
    more.push("not-a-crafting-item");
    assert!(blocked_list(&Value::from(more)).is_err());
}
