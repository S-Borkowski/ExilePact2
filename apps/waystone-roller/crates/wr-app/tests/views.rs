//! The IPC contract: every view fixture the window's tests render
//! (`fixtures/views/*.json`, shared with `src/ipc.test.ts`) reads into
//! `View` with unknown fields denied and writes back to the same JSON value,
//! and a view the app builds has the same shape.

mod common;

use common::*;
use serde_json::Value;
use wr_app::View;

#[test]
fn view_fixtures_round_trip_through_the_view_type() {
    let dir = app_root().join("fixtures").join("views");
    let mut names = Vec::new();
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap();
        let value: Value = serde_json::from_str(&text).unwrap();
        let view: View = serde_json::from_value(value.clone())
            .unwrap_or_else(|e| panic!("{} does not read into View: {e}", path.display()));
        assert_eq!(
            serde_json::to_value(&view).unwrap(),
            value,
            "{} does not write back the same",
            path.display()
        );
        names.push(path.file_stem().unwrap().to_string_lossy().into_owned());

        // Negative controls: an added field and a dropped one are refused.
        let mut extra = value.clone();
        extra["result"]["extra"] = Value::Bool(true);
        assert!(serde_json::from_value::<View>(extra).is_err());
        let mut dropped = value.clone();
        dropped["selection"]
            .as_object_mut()
            .unwrap()
            .remove("want_mode");
        assert!(serde_json::from_value::<View>(dropped).is_err());
    }
    names.sort();
    assert_eq!(
        names,
        [
            "empty",
            "hotkey-taken",
            "ok-all",
            "ok-any",
            "too-long",
            "unmatchable"
        ]
    );

    // A view the app builds round-trips too.
    let tmp = tempfile::tempdir().unwrap();
    let app = start(tmp.path(), &FakeShell::default());
    let value = serde_json::to_value(app.get_view()).unwrap();
    let back: View = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(back).unwrap(), value);
}
