//! The league the trade search opens in (`trade_league`, Settings, Trade)
//! and the list the Settings window offers for it.
//!
//! A league name is 1 to [`MAX_TRADE_LEAGUE_CHARS`] characters with no
//! control character; a patch is trimmed first ([`patch_league`]), a saved
//! value that breaks the rules is [`DEFAULT_TRADE_LEAGUE`]
//! ([`load_league`]). Any valid name is taken, not only one the list offers,
//! so a league saved from an older list is never refused.
//!
//! The list: [`SHIPPED_TRADE_LEAGUES`] until the player refreshes it, then,
//! for this run only (never saved), the poe2 leagues of the trade site's
//! answer ([`read_answer`]); the saved league is always offered, appended
//! last when the list lacks it ([`offered`]). The answer is refused whole,
//! keeping the list offered before, for a status other than 200, a body
//! over [`MAX_TRADE_LEAGUES_BODY`], JSON that is not `{"result": [{"id",
//! "realm", ...}, ...]}`, more than [`MAX_TRADE_LEAGUES`] entries, or no
//! entry left once the others are skipped: another realm than `poe2`, a
//! league marked private, and an id that is not a league name as it stands
//! (untrimmed, empty, too long or with a control character). Duplicates
//! collapse; the site's order is kept. Nothing here names a league in a
//! reason, so the log can carry it.

use serde_json::Value;

/// The `trade_league` key of a patch and of the file.
pub const TRADE_LEAGUE_KEY: &str = "trade_league";
/// The league the search opens in until the player chooses another: the
/// first softcore non-permanent league the trade site listed on 2026-10-29.
pub const DEFAULT_TRADE_LEAGUE: &str = "Forbidden Rites";
/// The leagues the Settings window offers before any refresh: the trade
/// site's poe2 leagues on 2026-10-29 and 2026-10-30, in its order.
pub const SHIPPED_TRADE_LEAGUES: [&str; 6] = [
    "Forbidden Rites",
    "HC Forbidden Rites",
    "Runes of Aldur",
    "HC Runes of Aldur",
    "Standard",
    "Hardcore",
];
/// The longest league name, in characters.
pub const MAX_TRADE_LEAGUE_CHARS: usize = 64;
/// The largest league list read (as `ep-net` caps it; checked again here).
pub const MAX_TRADE_LEAGUES_BODY: usize = 64 * 1024;
/// The most entries a league list may hold.
pub const MAX_TRADE_LEAGUES: usize = 50;

/// Why `name`, as it stands, is not a league name.
fn name_problem(name: &str) -> Option<String> {
    let chars = name.chars().count();
    if chars == 0 {
        return Some("the league name is empty".to_string());
    }
    if chars > MAX_TRADE_LEAGUE_CHARS {
        return Some(format!(
            "the league name is longer than {MAX_TRADE_LEAGUE_CHARS} characters"
        ));
    }
    if name.chars().any(char::is_control) {
        return Some("the league name holds a control character".to_string());
    }
    None
}

/// The league a patch gives, trimmed, or why it is refused.
pub fn patch_league(value: &Value) -> Result<String, String> {
    let name = value
        .as_str()
        .ok_or_else(|| format!("{TRADE_LEAGUE_KEY} is not text"))?
        .trim();
    match name_problem(name) {
        Some(why) => Err(why),
        None => Ok(name.to_string()),
    }
}

/// The league a saved file holds: [`DEFAULT_TRADE_LEAGUE`] unless it is a
/// league name once trimmed.
pub fn load_league(saved: Option<&Value>) -> String {
    saved
        .and_then(|v| patch_league(v).ok())
        .unwrap_or_else(|| DEFAULT_TRADE_LEAGUE.to_string())
}

/// What the Settings window offers: `refreshed` (or the shipped list), and
/// the saved league last when it lacks it.
pub fn offered(refreshed: Option<&[String]>, saved: &str) -> Vec<String> {
    let mut list: Vec<String> = match refreshed {
        Some(list) => list.to_vec(),
        None => SHIPPED_TRADE_LEAGUES
            .iter()
            .map(|s| s.to_string())
            .collect(),
    };
    if !list.iter().any(|l| l == saved) {
        list.push(saved.to_string());
    }
    list
}

/// A refresh's list: the leagues kept, in the site's order, and how many
/// entries were skipped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refreshed {
    pub leagues: Vec<String>,
    pub skipped: usize,
}

/// Reads the trade site's answer (see the module doc), or says why it is
/// refused, naming no league.
pub fn read_answer(status: u16, body: &[u8]) -> Result<Refreshed, String> {
    if status != 200 {
        return Err(format!("the trade site answered {status}"));
    }
    if body.len() > MAX_TRADE_LEAGUES_BODY {
        return Err(format!(
            "the answer is larger than {} KiB",
            MAX_TRADE_LEAGUES_BODY / 1024
        ));
    }
    let not_a_list = || "the answer is not a league list".to_string();
    let value: Value = serde_json::from_slice(body).map_err(|_| not_a_list())?;
    let entries = value
        .get("result")
        .and_then(Value::as_array)
        .ok_or_else(not_a_list)?;
    if entries.len() > MAX_TRADE_LEAGUES {
        return Err(format!(
            "the answer lists more than {MAX_TRADE_LEAGUES} leagues"
        ));
    }
    let mut leagues: Vec<String> = Vec::new();
    for entry in entries {
        let id = entry.get("id").and_then(Value::as_str);
        let realm = entry.get("realm").and_then(Value::as_str);
        let (Some(id), Some(realm)) = (id, realm) else {
            return Err(not_a_list());
        };
        let private = entry.get("private").and_then(Value::as_bool) == Some(true);
        let kept = realm == "poe2"
            && !private
            && id.trim() == id
            && name_problem(id).is_none()
            && !leagues.iter().any(|l| l == id);
        if kept {
            leagues.push(id.to_string());
        }
    }
    if leagues.is_empty() {
        return Err("the answer holds no Path of Exile 2 league".to_string());
    }
    Ok(Refreshed {
        skipped: entries.len() - leagues.len(),
        leagues,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_patch_is_trimmed_and_checked() {
        assert_eq!(patch_league(&json!("  Standard ")).unwrap(), "Standard");
        assert_eq!(
            patch_league(&json!("x".repeat(64))).unwrap(),
            "x".repeat(64)
        );
        for bad in [
            json!(""),
            json!("   "),
            json!("x".repeat(65)),
            json!("a\u{7}b"),
            json!("a\nb"),
            json!(3),
            json!(null),
        ] {
            assert!(patch_league(&bad).is_err(), "{bad}");
        }
        assert_eq!(load_league(Some(&json!(""))), DEFAULT_TRADE_LEAGUE);
        assert_eq!(load_league(None), DEFAULT_TRADE_LEAGUE);
        assert_eq!(load_league(Some(&json!(" Hardcore "))), "Hardcore");
    }

    #[test]
    fn the_saved_league_is_always_offered() {
        let shipped = offered(None, "Standard");
        assert_eq!(shipped, SHIPPED_TRADE_LEAGUES);
        let old = offered(None, "Old League");
        assert_eq!(old.len(), 7);
        assert_eq!(old.last().unwrap(), "Old League");
        let refreshed = vec!["A".to_string(), "B".to_string()];
        assert_eq!(offered(Some(&refreshed), "B"), ["A", "B"]);
        assert_eq!(offered(Some(&refreshed), "C"), ["A", "B", "C"]);
    }
}
