//! The mod data contract: the fixture loads, and each kind of bad file is
//! refused with the family and the field named.

mod common;

use std::io::Write;

use serde_json::Value;
use wr_regex::{Group, Kind, ModData, SCHEMA};

fn fixture_value() -> Value {
    serde_json::from_str(&common::fixture_json()).unwrap()
}

fn refused(doc: &Value) -> String {
    match ModData::parse(&doc.to_string()) {
        Ok(_) => panic!("accepted a bad file: {doc}"),
        Err(e) => e.to_string(),
    }
}

#[test]
fn data_loads_fixture_and_refuses_bad_schema() {
    // The fixture loads (positive control for every refusal below).
    let data = common::fixture_data();
    assert_eq!(data.schema, SCHEMA);
    assert_eq!(data.source, "https://poe2db.tw/us/Waystones");
    assert_eq!(data.sections, ["WaystonesMods", "DesecratedWaystoneMods"]);
    assert_eq!(data.fetched, "2026-10-28");
    assert_eq!(data.families.len(), 13);
    assert_eq!(data.skipped.len(), 2);
    let gloaming = data
        .family("monsters-have-increased-shadow-damage")
        .unwrap();
    assert_eq!(gloaming.group, Group::Desecrated);
    assert_eq!(gloaming.kind, Kind::Prefix);
    assert_eq!(gloaming.name.as_deref(), Some("Gloaming"));
    let sparkling = data.family("area-has-patches-of-sparkling-ground").unwrap();
    assert_eq!(sparkling.tiers, 4);
    assert_eq!(
        sparkling.own_lines().collect::<Vec<_>>(),
        ["Area has patches of Sparkling Ground"]
    );
    assert!(data.family("no-such-mod").is_none());

    // A file that is not JSON.
    assert!(ModData::parse("{ not json")
        .unwrap_err()
        .to_string()
        .contains("JSON"));

    // Each mutation of the fixture, with the words its refusal must carry.
    type Mutation = (&'static str, fn(&mut Value), &'static [&'static str]);
    let cases: Vec<Mutation> = vec![
        ("schema 2", |d| d["schema"] = 2.into(), &["schema"]),
        (
            "no families",
            |d| d["families"] = Value::Array(vec![]),
            &["families"],
        ),
        (
            "families missing",
            |d| {
                d.as_object_mut().unwrap().remove("families");
            },
            &["families"],
        ),
        ("unknown top field", |d| d["extra"] = 1.into(), &["extra"]),
        (
            "source not https",
            |d| d["source"] = "poe2db".into(),
            &["source"],
        ),
        (
            "fetched not a date",
            |d| d["fetched"] = "yesterday".into(),
            &["fetched"],
        ),
        (
            "bonus length differs",
            |d| d["families"][0]["bonus"] = serde_json::json!([false]),
            &["monsters-deal-of-damage-as-extra-glimmer", "bonus"],
        ),
        (
            "duplicate id",
            |d| d["families"][1]["id"] = d["families"][0]["id"].clone(),
            &["monsters-deal-of-damage-as-extra-glimmer", "id"],
        ),
        (
            "id not a slug",
            |d| d["families"][2]["id"] = "Monster Spectral".into(),
            &["Monster Spectral", "id"],
        ),
        (
            "same lines twice",
            |d| d["families"][1]["lines"] = d["families"][0]["lines"].clone(),
            &["monsters-deal-of-damage-as-extra-murk", "lines"],
        ),
        (
            "empty line",
            |d| d["families"][3]["lines"][0] = "".into(),
            &["monster-have-chance-to-bleat", "lines[0]"],
        ),
        (
            "bad group",
            |d| d["families"][0]["group"] = "abyss".into(),
            &["families"],
        ),
        (
            "regular mod with an affix name",
            |d| d["families"][0]["name"] = "Glimmering".into(),
            &["monsters-deal-of-damage-as-extra-glimmer", "name"],
        ),
        (
            "tiers zero",
            |d| d["families"][9]["tiers"] = 0.into(),
            &["area-has-patches-of-sparkling-ground", "tiers"],
        ),
        (
            "unknown family field",
            |d| d["families"][0]["value"] = 5.into(),
            &["value"],
        ),
    ];
    for (what, mutate, words) in cases {
        let mut doc = fixture_value();
        mutate(&mut doc);
        let message = refused(&doc);
        for w in words {
            assert!(
                message.contains(w),
                "{what}: {message:?} does not name {w:?}"
            );
        }
    }

    // `load` refuses a bad file on disk the same way and names the problem.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("waystone-mods.json");
    let mut doc = fixture_value();
    doc["schema"] = 3.into();
    std::fs::File::create(&path)
        .unwrap()
        .write_all(doc.to_string().as_bytes())
        .unwrap();
    assert!(ModData::load(&path)
        .unwrap_err()
        .to_string()
        .contains("schema"));
    assert!(ModData::load(&dir.path().join("missing.json")).is_err());
}
