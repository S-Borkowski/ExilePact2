//! What `settings.json` holds and what the Settings window may change.
//!
//! The file: `schema` 1, `hotkey`, `check_updates` and
//! `blocked_currencies`. `check_updates` is off unless it is saved as
//! `true`. The blocked currencies are plain ids for now; the crafting data
//! names them later.
//!
//! `update_settings` takes a patch of [`CHECK_UPDATES_KEY`] and
//! [`BLOCKED_CURRENCIES_KEY`], any of the two and nothing else, read whole
//! by [`read_patch`] before anything changes. `blocked_currencies` replaces
//! the whole list: each id is trimmed, empty ids are dropped and repeats
//! removed, keeping the first order. A list that is not an array of
//! strings, an id over [`MAX_ID_CHARS`] characters or more than
//! [`MAX_BLOCKED`] ids is refused, with nothing changed. A saved list that
//! breaks those rules keeps its valid ids ([`load_blocked`]).

use serde_json::Value;

/// The `check_updates` key of a patch and of the file.
pub const CHECK_UPDATES_KEY: &str = "check_updates";
/// The `blocked_currencies` key of a patch and of the file.
pub const BLOCKED_CURRENCIES_KEY: &str = "blocked_currencies";
/// The longest blocked currency id, in characters.
pub const MAX_ID_CHARS: usize = 64;
/// The most blocked currency ids.
pub const MAX_BLOCKED: usize = 200;

/// A checked `update_settings` patch: what it changes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Patch {
    pub check_updates: Option<bool>,
    pub blocked_currencies: Option<Vec<String>>,
}

/// Reads an `update_settings` patch. Anything but an object of the two keys
/// with values of the right shape is refused whole, with the reason.
pub fn read_patch(patch: &Value) -> Result<Patch, String> {
    let patch = patch
        .as_object()
        .ok_or_else(|| "the change is not an object".to_string())?;
    if let Some(k) = patch
        .keys()
        .find(|k| k.as_str() != CHECK_UPDATES_KEY && k.as_str() != BLOCKED_CURRENCIES_KEY)
    {
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
    Ok(Patch {
        check_updates,
        blocked_currencies,
    })
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
        push_id(&mut ids, id);
    }
    if ids.len() > MAX_BLOCKED {
        return Err(format!(
            "{} currency ids are more than {MAX_BLOCKED}",
            ids.len()
        ));
    }
    Ok(ids)
}

/// The blocked list of a saved file: its valid ids, in order, at most
/// [`MAX_BLOCKED`] of them, and how many entries were dropped. A value that
/// is not a list keeps nothing.
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
            Some(id)
                if !id.is_empty()
                    && id.chars().count() <= MAX_ID_CHARS
                    && ids.len() < MAX_BLOCKED =>
            {
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
                blocked_currencies: None
            }
        );
        assert_eq!(
            read_patch(&json!({ "blocked_currencies": [" a ", "", "b", "a"] }))
                .unwrap()
                .blocked_currencies,
            Some(vec!["a".to_string(), "b".to_string()])
        );
        for bad in [
            json!({}),
            json!({ "hotkey": "Ctrl+Alt+R" }),
            json!({ "check_updates": "yes" }),
            json!({ "blocked_currencies": "a" }),
            json!({ "blocked_currencies": [1] }),
            json!({ "blocked_currencies": ["x".repeat(65)] }),
            json!({ "check_updates": true, "auto_install": true }),
            json!([true]),
        ] {
            assert!(read_patch(&bad).is_err(), "{bad}");
        }
        let many: Vec<String> = (0..201).map(|i| format!("c{i}")).collect();
        assert!(read_patch(&json!({ "blocked_currencies": many })).is_err());
        let enough: Vec<String> = (0..200).map(|i| format!("c{i}")).collect();
        assert!(read_patch(&json!({ "blocked_currencies": enough })).is_ok());
        assert!(read_patch(&json!({ "blocked_currencies": ["x".repeat(64)] })).is_ok());
    }

    #[test]
    fn a_bad_saved_list_keeps_its_valid_ids() {
        let saved = json!(["a", 3, "", " b ", "a", "x".repeat(65), null, "c"]);
        assert_eq!(
            load_blocked(Some(&saved)),
            (vec!["a".to_string(), "b".to_string(), "c".to_string()], 5)
        );
        assert_eq!(load_blocked(Some(&json!("a"))), (Vec::new(), 1));
        assert_eq!(load_blocked(None), (Vec::new(), 0));
    }
}
