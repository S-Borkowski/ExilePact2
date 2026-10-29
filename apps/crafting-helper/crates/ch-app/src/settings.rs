//! What `settings.json` holds and what the Settings window may change, and
//! the catalogue the blocked list picks from.
//!
//! The file: `schema` 1, `hotkey`, `check_updates`, `blocked_currencies`,
//! `overlay_position` and `screen_watch`. `check_updates` is off unless it
//! is saved as `true`; `screen_watch` is on unless it is saved as `false`;
//! `overlay_position` is one of [`OverlayPosition`]'s names, and anything
//! else saved there is the default, `top-center`. The blocked currencies are ids of the engine's catalogue
//! ([`ch_engine::catalogue`]): every currency, omen, bone, flux, catalyst,
//! essence, alloy and liquid emotion a blueprint may name.
//!
//! The catalogue is the blocked list's limit: each id must be one of its
//! ids, with no repeats, so the list may hold the whole catalogue and never
//! more. There is no other count limit.
//!
//! `update_settings` takes a patch of [`CHECK_UPDATES_KEY`],
//! [`BLOCKED_CURRENCIES_KEY`], [`OVERLAY_POSITION_KEY`] and
//! [`SCREEN_WATCH_KEY`], any of the four and nothing else, read whole by
//! [`read_patch`] before anything changes; a position that is not one of
//! the six names, exactly, is refused. `blocked_currencies` replaces
//! the whole list: each id is trimmed, empty ids are dropped and repeats
//! removed, keeping the first order. A list that is not an array of
//! strings, an id over [`MAX_ID_CHARS`] characters or an id outside the
//! catalogue is refused, with nothing changed. A saved list that breaks
//! those rules keeps its valid ids and counts the rest ([`load_blocked`]).
//!
//! [`catalogue_groups`] is the `get_catalogue` command's answer: the
//! catalogue grouped by kind, for the pickers of the Settings window and the
//! craft window, which offer names and send ids.

use ch_engine::ch_blueprint::catalogue::Kind;
use ch_engine::{catalogue, catalogue_item};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::view::OverlayPosition;

/// The `check_updates` key of a patch and of the file.
pub const CHECK_UPDATES_KEY: &str = "check_updates";
/// The `blocked_currencies` key of a patch and of the file.
pub const BLOCKED_CURRENCIES_KEY: &str = "blocked_currencies";
/// The `overlay_position` key of a patch and of the file.
pub const OVERLAY_POSITION_KEY: &str = "overlay_position";
/// The `screen_watch` key of a patch and of the file.
pub const SCREEN_WATCH_KEY: &str = "screen_watch";
/// The keys a patch may hold.
const PATCH_KEYS: [&str; 4] = [
    CHECK_UPDATES_KEY,
    BLOCKED_CURRENCIES_KEY,
    OVERLAY_POSITION_KEY,
    SCREEN_WATCH_KEY,
];
/// The longest blocked currency id, in characters.
pub const MAX_ID_CHARS: usize = 64;

/// A checked `update_settings` patch: what it changes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Patch {
    pub check_updates: Option<bool>,
    pub blocked_currencies: Option<Vec<String>>,
    pub overlay_position: Option<OverlayPosition>,
    pub screen_watch: Option<bool>,
}

/// Reads an `update_settings` patch. Anything but an object of the four
/// keys with values of the right shape is refused whole, with the reason.
pub fn read_patch(patch: &Value) -> Result<Patch, String> {
    let patch = patch
        .as_object()
        .ok_or_else(|| "the change is not an object".to_string())?;
    if let Some(k) = patch.keys().find(|k| !PATCH_KEYS.contains(&k.as_str())) {
        return Err(format!("{k:?} is not a setting of this window"));
    }
    if patch.is_empty() {
        return Err("the change names no setting".to_string());
    }
    let check_updates = match patch.get(CHECK_UPDATES_KEY) {
        None => None,
        Some(Value::Bool(on)) => Some(*on),
        Some(_) => return Err(format!("{CHECK_UPDATES_KEY} is not true or false")),
    };
    let blocked_currencies = match patch.get(BLOCKED_CURRENCIES_KEY) {
        None => None,
        Some(list) => Some(blocked_list(list)?),
    };
    let overlay_position = match patch.get(OVERLAY_POSITION_KEY) {
        None => None,
        Some(v) => Some(
            v.as_str()
                .and_then(OverlayPosition::from_name)
                .ok_or_else(|| format!("{v} is not an overlay position"))?,
        ),
    };
    let screen_watch = match patch.get(SCREEN_WATCH_KEY) {
        None => None,
        Some(Value::Bool(on)) => Some(*on),
        Some(_) => return Err(format!("{SCREEN_WATCH_KEY} is not true or false")),
    };
    Ok(Patch {
        check_updates,
        blocked_currencies,
        overlay_position,
        screen_watch,
    })
}

/// The overlay's position a saved file holds: the default unless it is one
/// of the six names.
pub fn load_overlay_position(saved: Option<&Value>) -> OverlayPosition {
    saved
        .and_then(Value::as_str)
        .and_then(OverlayPosition::from_name)
        .unwrap_or_default()
}

/// The screen watch a saved file holds: on unless it is saved `false`.
pub fn load_screen_watch(saved: Option<&Value>) -> bool {
    saved.and_then(Value::as_bool) != Some(false)
}

/// The blocked list a patch gives, normalised, or why it is refused.
pub fn blocked_list(list: &Value) -> Result<Vec<String>, String> {
    let items = list
        .as_array()
        .ok_or_else(|| format!("{BLOCKED_CURRENCIES_KEY} is not a list"))?;
    let mut ids: Vec<String> = Vec::new();
    for item in items {
        let id = item
            .as_str()
            .ok_or_else(|| format!("{BLOCKED_CURRENCIES_KEY} holds something that is not text"))?
            .trim();
        if id.chars().count() > MAX_ID_CHARS {
            return Err(format!(
                "a currency id is longer than {MAX_ID_CHARS} characters"
            ));
        }
        if !id.is_empty() && catalogue_item(id).is_none() {
            return Err(format!("{id:?} is not in the crafting catalogue"));
        }
        push_id(&mut ids, id);
    }
    Ok(ids)
}

/// The blocked list of a saved file: its catalogue ids, in order, each
/// once, and how many entries were dropped. A value that is not a list
/// keeps nothing.
pub fn load_blocked(saved: Option<&Value>) -> (Vec<String>, usize) {
    let Some(saved) = saved else {
        return (Vec::new(), 0);
    };
    let Some(items) = saved.as_array() else {
        return (Vec::new(), 1);
    };
    let mut ids: Vec<String> = Vec::new();
    let mut dropped = 0;
    for item in items {
        match item.as_str().map(str::trim) {
            Some(id) if id.chars().count() <= MAX_ID_CHARS && catalogue_item(id).is_some() => {
                if !push_id(&mut ids, id) {
                    dropped += 1;
                }
            }
            _ => dropped += 1,
        }
    }
    (ids, dropped)
}

/// Adds `id` (already trimmed) unless it is empty or already there; says
/// whether it was added.
fn push_id(ids: &mut Vec<String>, id: &str) -> bool {
    if id.is_empty() || ids.iter().any(|x| x == id) {
        return false;
    }
    ids.push(id.to_string());
    true
}

/// One kind of crafting item in the catalogue, as the pickers list it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogueGroup {
    /// The kind, as the engine serialises it (`currency`, `omen`, `bone`,
    /// `flux`, `catalyst`, `essence`, `alloy`, `liquid`).
    pub kind: String,
    /// The group's heading.
    pub label: String,
    /// Sorted by name.
    pub entries: Vec<CatalogueEntry>,
}

/// One crafting item a picker offers: the name it shows, the id it sends.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogueEntry {
    pub id: String,
    /// The English name.
    pub name: String,
}

/// The kinds in the order the pickers show them, each with its heading.
const KINDS: [(Kind, &str); 8] = [
    (Kind::Currency, "Currency"),
    (Kind::Omen, "Omens"),
    (Kind::Bone, "Bones"),
    (Kind::Flux, "Fluxes"),
    (Kind::Catalyst, "Catalysts"),
    (Kind::Essence, "Essences"),
    (Kind::Alloy, "Alloys"),
    (Kind::Liquid, "Liquid emotions"),
];

/// The whole catalogue grouped by kind, in [`KINDS`]' order, each group's
/// entries sorted by name (case aside, then by id). A kind with no entries
/// has no group.
pub fn catalogue_groups() -> Vec<CatalogueGroup> {
    KINDS
        .iter()
        .filter_map(|(kind, label)| {
            let mut entries: Vec<CatalogueEntry> = catalogue()
                .iter()
                .filter(|c| c.kind == *kind)
                .map(|c| CatalogueEntry {
                    id: c.id.clone(),
                    name: c.name.clone(),
                })
                .collect();
            if entries.is_empty() {
                return None;
            }
            entries.sort_by(|a, b| {
                a.name
                    .to_lowercase()
                    .cmp(&b.name.to_lowercase())
                    .then_with(|| a.id.cmp(&b.id))
            });
            Some(CatalogueGroup {
                kind: kind_name(*kind),
                label: label.to_string(),
                entries,
            })
        })
        .collect()
}

/// The kind as the engine serialises it.
fn kind_name(kind: Kind) -> String {
    match serde_json::to_value(kind) {
        Ok(Value::String(s)) => s,
        _ => format!("{kind:?}").to_lowercase(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_patch_takes_the_two_keys_and_nothing_else() {
        assert_eq!(
            read_patch(&json!({ "check_updates": true })).unwrap(),
            Patch {
                check_updates: Some(true),
                ..Patch::default()
            }
        );
        assert_eq!(
            read_patch(&json!({ "overlay_position": "bottom-center", "screen_watch": false }))
                .unwrap(),
            Patch {
                overlay_position: Some(OverlayPosition::BottomCenter),
                screen_watch: Some(false),
                ..Patch::default()
            }
        );
        assert_eq!(
            read_patch(
                &json!({ "blocked_currencies": [" chaos-orb ", "", "divine-orb", "chaos-orb"] })
            )
            .unwrap()
            .blocked_currencies,
            Some(vec!["chaos-orb".to_string(), "divine-orb".to_string()])
        );
        for bad in [
            json!({}),
            json!({ "hotkey": "Ctrl+Alt+R" }),
            json!({ "check_updates": "yes" }),
            json!({ "blocked_currencies": "chaos-orb" }),
            json!({ "blocked_currencies": [1] }),
            json!({ "blocked_currencies": ["x".repeat(65)] }),
            json!({ "blocked_currencies": ["chaos"] }),
            json!({ "check_updates": true, "auto_install": true }),
            json!({ "overlay_position": "center" }),
            json!({ "overlay_position": " top-left" }),
            json!({ "overlay_position": null }),
            json!({ "screen_watch": "on" }),
            json!([true]),
        ] {
            assert!(read_patch(&bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn every_kind_has_its_heading() {
        // A kind the catalogue holds but `KINDS` misses would drop its
        // entries from the pickers.
        for c in catalogue() {
            assert!(KINDS.iter().any(|(k, _)| *k == c.kind), "{:?}", c.kind);
        }
    }

    #[test]
    fn a_bad_saved_list_keeps_its_valid_ids() {
        let saved = json!([
            "chaos-orb",
            3,
            "",
            " divine-orb ",
            "chaos-orb",
            "x".repeat(65),
            null,
            "chaos",
            "regal-orb"
        ]);
        assert_eq!(
            load_blocked(Some(&saved)),
            (
                vec![
                    "chaos-orb".to_string(),
                    "divine-orb".to_string(),
                    "regal-orb".to_string()
                ],
                6
            )
        );
        assert_eq!(load_blocked(Some(&json!("a"))), (Vec::new(), 1));
        assert_eq!(load_blocked(None), (Vec::new(), 0));
    }
}
