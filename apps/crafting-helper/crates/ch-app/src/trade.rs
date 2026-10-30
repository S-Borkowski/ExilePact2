//! The *Find a base on trade* panel (`ui-design.md` § "Find a base on
//! trade"): the selection the player makes (an item class, a base type, an
//! optional minimum item level, the modifiers wanted with their floors and
//! the *Both slots* choice), in memory only and never saved, its view, and
//! the trade site's search link built from it.
//!
//! The link is [`TRADE_SEARCH_URL`], the league percent-encoded, `?q=` and
//! the search's JSON (`ch-data`'s [`trade_query`]) percent-encoded, so that
//! only `A-Z a-z 0-9 - . _ ~` stay literal ([`percent_encode`]): a league of
//! `../x?y#z` cannot leave its path segment. A link over
//! [`MAX_TRADE_LINK_CHARS`] is refused. Nothing here talks to the trade
//! site; `App::open_trade_search` hands the link to the shell's browser.
//!
//! The picked floors suggest a minimum item level (`ch-data`'s
//! [`suggested_item_level`]: the level the most demanding floor tier
//! needs). With the item level field blank the search uses the suggestion;
//! a typed level always wins, and the suggestion is still shown beside it.
//!
//! `set_trade_search` takes the whole selection, never an address: a new
//! class starts its base and picks over; a new base type or item level keeps
//! the picks the base still offers (a floor it no longer reaches goes back to
//! "any tier") and drops the rest, and says how many; anything else wrong
//! (an unknown class or base type, an item level outside 1 to 100, a ninth
//! pick, a modifier picked twice, one the base does not offer or the site
//! cannot search, a floor not reachable) is refused with nothing changed.
//! Each refusal carries the words for the status line and a reason for the
//! log that names no league, base or modifier ([`TradeRefusal::log`]).

use ch_engine::ch_data::{
    class_data_by_name, index, start_weight, suggested_item_level, trade_offer, trade_query, Kind,
    OfferedMod, TradeOffer, TradePick, TradeQuery, TradeSearch, WeightSource, MAX_TRADE_PICKS,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::view::{FloorOption, ModSide, StatusLine};

/// The trade site's search page for Path of Exile 2; the league and the
/// query follow. The second address the app opens, from
/// `open_trade_search` alone.
pub const TRADE_SEARCH_URL: &str = "https://www.pathofexile.com/trade2/search/poe2/";
/// The longest link opened: Windows' shell URL limits sit near 2,048.
pub const MAX_TRADE_LINK_CHARS: usize = 2_000;
/// The highest minimum item level the panel takes (the lowest is 1).
pub const MAX_TRADE_ITEM_LEVEL: u32 = 100;

/// One picked modifier, as `set_trade_search` sends it: a data modifier id
/// and its floor (a tier number, 1 the best; `None` for any tier).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TradePickChoice {
    pub mod_id: String,
    pub floor: Option<u32>,
}

/// The panel's whole selection: what `set_trade_search` takes and what the
/// view sends back. No field names an address.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TradeSelection {
    /// The panel shows in place of the craft area.
    pub open: bool,
    /// As the `Item Class:` line writes it (`Rings`).
    pub class: Option<String>,
    /// The base type (`Sapphire Ring`).
    pub base: Option<String>,
    pub item_level: Option<u32>,
    pub picks: Vec<TradePickChoice>,
    /// *Both slots* (the default): ask for a prefix and a suffix when the
    /// picks have both; off is *At least one*.
    pub both_slots: bool,
}

impl Default for TradeSelection {
    fn default() -> TradeSelection {
        TradeSelection {
            open: false,
            class: None,
            base: None,
            item_level: None,
            picks: Vec::new(),
            both_slots: true,
        }
    }
}

/// Where a shown weight came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradeWeightSource {
    /// The data's spawn weights.
    Spawn,
    /// The data gives no usable weight; the stated default stands in.
    Default,
}

/// One searchable modifier the base offers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TradeModView {
    /// The data modifier id `set_trade_search` sends.
    pub mod_id: String,
    pub side: ModSide,
    /// As the data writes it (`+# to maximum Life`).
    pub text: String,
    /// `Any tier`, then `At least T<n>` per tier reachable at the item
    /// level, best first.
    pub floors: Vec<FloorOption>,
    pub picked: bool,
    /// The pick's floor; `None` for any tier (and when not picked).
    pub floor: Option<u32>,
    /// The rarity weight, one decimal (`12.4`): about 1 in that many adds
    /// to this side of the base is this modifier, at the floor or better.
    /// At the pick's floor when picked, any tier otherwise.
    pub weight: String,
    pub weight_source: TradeWeightSource,
    /// For a default weight: why the data has none here, and what the
    /// default is.
    pub default_reason: Option<String>,
}

/// A modifier the base offers that the trade site cannot search for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TradeUnsearchableView {
    pub mod_id: String,
    pub side: ModSide,
    /// Its lines, joined by ` / `.
    pub text: String,
    pub reason: String,
}

/// The trade panel's part of the main view. With the panel closed it
/// carries the selection and the league alone, every list empty and no
/// link; open, everything below.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TradeView {
    pub selection: TradeSelection,
    /// Every item class, in the data's order.
    pub classes: Vec<String>,
    /// The chosen class's base types, in the data's order, each once.
    pub bases: Vec<String>,
    pub prefixes: Vec<TradeModView>,
    pub suffixes: Vec<TradeModView>,
    /// The base's modifiers the site cannot search for, prefixes first.
    pub not_searchable: Vec<TradeUnsearchableView>,
    /// Another modifier may be picked: fewer than [`MAX_TRADE_PICKS`].
    pub can_pick: bool,
    /// `At most 8 modifiers in one search.` once 8 are picked.
    pub pick_limit: Option<String>,
    /// What the search asks for, and why, once something is picked.
    pub count_line: Option<String>,
    /// The item level the picked floors need (the most demanding floor
    /// tier's); `None` with no floor picked. The search uses it while the
    /// item level field is blank; a typed level wins.
    pub suggested_item_level: Option<u32>,
    /// The league the search opens in (Settings, Trade).
    pub league: String,
    /// The link the search button opens.
    pub link: Option<String>,
    /// `open_trade_search` would open the link.
    pub can_search: bool,
    /// Why there is no link: info while something is still to choose, an
    /// error when the selection cannot make one.
    pub why_not: Option<StatusLine>,
}

/// Why a trade command changed or opened nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TradeRefusal {
    /// For the status line and the panel, lower case, without a final stop
    /// (`Rings has no base type Iron Mitts`).
    pub text: String,
    /// For `diag.log`: what kind of refusal, naming no league, class, base
    /// or modifier (`unknown base type`).
    pub log: &'static str,
    /// Something is still to choose, rather than wrong.
    pub to_choose: bool,
}

fn refuse(text: impl Into<String>, log: &'static str) -> TradeRefusal {
    TradeRefusal {
        text: text.into(),
        log,
        to_choose: false,
    }
}

fn to_choose(text: &str, log: &'static str) -> TradeRefusal {
    TradeRefusal {
        text: text.to_string(),
        log,
        to_choose: true,
    }
}

impl TradeRefusal {
    /// As a sentence of its own: `Choose an item class.`
    pub fn sentence(&self) -> String {
        let mut chars = self.text.chars();
        match chars.next() {
            Some(first) => format!("{}{}.", first.to_uppercase(), chars.as_str()),
            None => String::new(),
        }
    }
}

/// A search ready to open: the link and what it asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TradeLink {
    pub link: String,
    /// How many modifiers are picked.
    pub picks: usize,
    /// How many of them the item must carry.
    pub at_least: u32,
}

/// `s` with every byte but `A-Z a-z 0-9 - . _ ~` written `%XX` (UTF-8,
/// upper-case hex).
pub fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// The trade search link for `league` and the search JSON `query`:
/// [`TRADE_SEARCH_URL`], the league and `?q=` and the query, both
/// percent-encoded. Refused for a league that would be a dot segment of its
/// own (`.` or `..`, which a browser resolves away) and for a link over
/// [`MAX_TRADE_LINK_CHARS`].
pub fn trade_link(league: &str, query: &Value) -> Result<String, TradeRefusal> {
    if league == "." || league == ".." {
        return Err(refuse(
            "the league name cannot be . or .. in a link: choose another in Settings",
            "the league is a dot segment",
        ));
    }
    let link = format!(
        "{TRADE_SEARCH_URL}{}?q={}",
        percent_encode(league),
        percent_encode(&query.to_string())
    );
    if link.len() > MAX_TRADE_LINK_CHARS {
        return Err(refuse(
            "too many modifiers for one link: remove one",
            "the link is too long",
        ));
    }
    Ok(link)
}

/// The panel's state: the selection, in memory only.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Trade {
    sel: TradeSelection,
}

impl Trade {
    pub fn selection(&self) -> &TradeSelection {
        &self.sel
    }

    /// `set_trade_search`: takes `next` (see the module doc). Returns what
    /// the status line should say about something dropped, or the refusal,
    /// with nothing changed.
    pub fn set(&mut self, next: &TradeSelection) -> Result<Option<String>, TradeRefusal> {
        let mut next = next.clone();
        if let Some(level) = next.item_level {
            if !(1..=MAX_TRADE_ITEM_LEVEL).contains(&level) {
                return Err(refuse(
                    format!(
                        "the minimum item level is a whole number from 1 to {MAX_TRADE_ITEM_LEVEL}"
                    ),
                    "item level out of range",
                ));
            }
        }
        if next.picks.len() > MAX_TRADE_PICKS {
            return Err(refuse(
                format!("at most {MAX_TRADE_PICKS} modifiers in one search"),
                "too many picks",
            ));
        }
        for (i, pick) in next.picks.iter().enumerate() {
            if next.picks[..i].iter().any(|p| p.mod_id == pick.mod_id) {
                return Err(refuse(
                    "a modifier is picked twice",
                    "a modifier picked twice",
                ));
            }
        }

        // A new class: its base and picks start over.
        if next.class != self.sel.class {
            if let Some(class) = &next.class {
                if class_data_by_name(class).is_none() {
                    return Err(refuse(
                        format!("{class} is not an item class the helper knows"),
                        "unknown item class",
                    ));
                }
            }
            let cleared = self.sel.picks.len();
            next.base = None;
            next.picks.clear();
            self.sel = next;
            return Ok((cleared > 0).then(|| {
                format!(
                    "Item class changed: {cleared} pick{} cleared.",
                    plural(cleared)
                )
            }));
        }

        let Some(base) = next.base.clone() else {
            if !next.picks.is_empty() {
                if self.sel.base.is_none() {
                    return Err(to_choose("choose a base type first", "no base type"));
                }
                let n = next.picks.len();
                next.picks.clear();
                self.sel = next;
                return Ok(Some(format!(
                    "No base type: {n} pick{} cleared.",
                    plural(n)
                )));
            }
            self.sel = next;
            return Ok(None);
        };
        let Some(class) = next.class.clone() else {
            return Err(to_choose("choose an item class first", "no item class"));
        };
        let data = class_data_by_name(&class).ok_or_else(|| {
            refuse(
                format!("{class} is not an item class the helper knows"),
                "unknown item class",
            )
        })?;
        if !data.bases.iter().any(|b| b.name == base) {
            return Err(refuse(
                format!("{class} has no base type {base}"),
                "unknown base type",
            ));
        }
        let offer = trade_offer(&class, &base, next.item_level)
            .map_err(|e| refuse(e, "the base's modifiers could not be read"))?;

        let moved = next.base != self.sel.base || next.item_level != self.sel.item_level;
        let mut note = None;
        if moved {
            // Keep what the base still offers at this level.
            let before = next.picks.len();
            next.picks
                .retain(|p| offered(&offer, &p.mod_id).is_some_and(|m| m.search.searchable));
            let dropped = before - next.picks.len();
            let mut reset = 0;
            for pick in &mut next.picks {
                let tiers = offered(&offer, &pick.mod_id).map(|m| &m.tiers);
                if let (Some(floor), Some(tiers)) = (pick.floor, tiers) {
                    if !tiers.contains(&floor) {
                        pick.floor = None;
                        reset += 1;
                    }
                }
            }
            let mut parts = Vec::new();
            if dropped > 0 {
                parts.push(format!(
                    "{dropped} pick{} dropped: {} cannot roll on this base at this item level.",
                    plural(dropped),
                    if dropped == 1 { "it" } else { "they" }
                ));
            }
            if reset > 0 {
                parts.push(format!(
                    "{reset} floor{} back to Any tier: not reachable at this item level.",
                    plural(reset)
                ));
            }
            if !parts.is_empty() {
                note = Some(parts.join(" "));
            }
        } else {
            for pick in &next.picks {
                let Some(m) = offered(&offer, &pick.mod_id) else {
                    return Err(refuse(
                        format!(
                            "{} is not a modifier this base offers at this item level",
                            pick.mod_id
                        ),
                        "a modifier the base does not offer",
                    ));
                };
                if !m.search.searchable {
                    return Err(refuse(
                        format!(
                            "{} cannot be searched on the trade site: {}",
                            m.lines.join(" / "),
                            unsearchable_reason(m.search.reason.as_deref())
                        ),
                        "a modifier the trade site cannot search",
                    ));
                }
                if let Some(floor) = pick.floor {
                    if !m.tiers.contains(&floor) {
                        return Err(refuse(
                            format!(
                                "T{floor} is not a tier of {} this base reaches at this item level",
                                m.lines.join(" / ")
                            ),
                            "a floor the base does not reach",
                        ));
                    }
                }
            }
        }
        self.sel = next;
        Ok(note)
    }

    /// The item level the picked floors need, or `None` with no class, base
    /// type or floor.
    pub fn suggested_item_level(&self) -> Option<u32> {
        let sel = &self.sel;
        let picks: Vec<TradePick> = sel.picks.iter().map(trade_pick).collect();
        suggested_item_level(sel.class.as_deref()?, sel.base.as_deref()?, &picks)
    }

    /// The search `ch-data` builds for the selection, or why there is none:
    /// at the typed item level, or the suggested one when the field is
    /// blank.
    fn query(&self) -> Result<TradeQuery, TradeRefusal> {
        let sel = &self.sel;
        let Some(class) = &sel.class else {
            return Err(to_choose("choose an item class", "no item class"));
        };
        let Some(base) = &sel.base else {
            return Err(to_choose("choose a base type", "no base type"));
        };
        if sel.picks.is_empty() {
            return Err(to_choose(
                "pick at least one modifier",
                "no modifier picked",
            ));
        }
        trade_query(&TradeSearch {
            class: class.clone(),
            base: base.clone(),
            item_level: sel.item_level.or_else(|| self.suggested_item_level()),
            picks: sel.picks.iter().map(trade_pick).collect(),
            both_slots: sel.both_slots,
        })
        .map_err(|e| refuse(e, "the search could not be built"))
    }

    /// The link `open_trade_search` opens for `league`, or why there is
    /// none.
    pub fn link(&self, league: &str) -> Result<TradeLink, TradeRefusal> {
        let query = self.query()?;
        Ok(TradeLink {
            link: trade_link(league, &query.query)?,
            picks: self.sel.picks.len(),
            at_least: query.count,
        })
    }

    /// The panel's view with the search opening in `league`.
    pub fn view(&self, league: &str) -> TradeView {
        let sel = &self.sel;
        let mut view = TradeView {
            selection: sel.clone(),
            classes: Vec::new(),
            bases: Vec::new(),
            prefixes: Vec::new(),
            suffixes: Vec::new(),
            not_searchable: Vec::new(),
            can_pick: sel.picks.len() < MAX_TRADE_PICKS,
            pick_limit: (sel.picks.len() >= MAX_TRADE_PICKS)
                .then(|| format!("At most {MAX_TRADE_PICKS} modifiers in one search.")),
            count_line: None,
            suggested_item_level: None,
            league: league.to_string(),
            link: None,
            can_search: false,
            why_not: None,
        };
        if !sel.open {
            return view;
        }
        view.classes = index().classes.iter().map(|c| c.class.clone()).collect();
        view.suggested_item_level = self.suggested_item_level();
        if let Some(data) = sel.class.as_deref().and_then(class_data_by_name) {
            for b in &data.bases {
                if !view.bases.contains(&b.name) {
                    view.bases.push(b.name.clone());
                }
            }
            if let Some(offer) = sel
                .base
                .as_deref()
                .and_then(|base| trade_offer(&data.class, base, sel.item_level).ok())
            {
                for m in offer.prefixes.iter().chain(&offer.suffixes) {
                    let side = side_of(m.kind);
                    if !m.search.searchable {
                        view.not_searchable.push(TradeUnsearchableView {
                            mod_id: m.id.to_string(),
                            side,
                            text: m.lines.join(" / "),
                            reason: unsearchable_reason(m.search.reason.as_deref()),
                        });
                        continue;
                    }
                    let pick = sel.picks.iter().find(|p| p.mod_id == m.id);
                    let floor = pick.and_then(|p| p.floor);
                    // A picked floor counts down to that tier.
                    let weight = match floor {
                        Some(_) => {
                            let variant = offer.variant;
                            start_weight(data, variant, sel.item_level, m.id, floor)
                                .unwrap_or_else(|_| m.weight.clone())
                        }
                        None => m.weight.clone(),
                    };
                    let row = TradeModView {
                        mod_id: m.id.to_string(),
                        side,
                        text: m.lines.join(" / "),
                        floors: floor_options(&m.tiers),
                        picked: pick.is_some(),
                        floor,
                        weight: format!("{:.1}", weight.weight),
                        weight_source: match weight.source {
                            WeightSource::Spawn => TradeWeightSource::Spawn,
                            WeightSource::Default => TradeWeightSource::Default,
                        },
                        default_reason: weight.reason.clone(),
                    };
                    match m.kind {
                        Kind::Prefix => view.prefixes.push(row),
                        Kind::Suffix => view.suffixes.push(row),
                    }
                }
            }
        }
        match self.query() {
            Ok(query) => {
                view.count_line = Some(count_line(&query, sel.both_slots).to_string());
                match trade_link(league, &query.query) {
                    Ok(link) => {
                        view.link = Some(link);
                        view.can_search = true;
                    }
                    Err(r) => view.why_not = Some(why_not(&r)),
                }
            }
            Err(r) => view.why_not = Some(why_not(&r)),
        }
        view
    }
}

fn why_not(r: &TradeRefusal) -> StatusLine {
    if r.to_choose {
        StatusLine::info(r.sentence())
    } else {
        StatusLine::error(r.sentence())
    }
}

fn trade_pick(p: &TradePickChoice) -> TradePick {
    TradePick {
        id: p.mod_id.clone(),
        floor: p.floor,
    }
}

fn plural(n: usize) -> &'static str {
    if n == 1 {
        ""
    } else {
        "s"
    }
}

fn offered<'a>(offer: &'a TradeOffer<'static>, id: &str) -> Option<&'a OfferedMod<'static>> {
    offer
        .prefixes
        .iter()
        .chain(&offer.suffixes)
        .find(|m| m.id == id)
}

fn side_of(kind: Kind) -> ModSide {
    match kind {
        Kind::Prefix => ModSide::Prefix,
        Kind::Suffix => ModSide::Suffix,
    }
}

/// `Any tier`, then `At least T<n>` per reachable tier, best first.
fn floor_options(tiers: &[u32]) -> Vec<FloorOption> {
    let mut options = vec![FloorOption {
        floor: None,
        label: "Any tier".to_string(),
    }];
    options.extend(tiers.iter().map(|t| FloorOption {
        floor: Some(*t),
        label: format!("At least T{t}"),
    }));
    options
}

/// The mapping's reason in the panel's words.
fn unsearchable_reason(reason: Option<&str>) -> String {
    match reason {
        Some("hybrid") => "more than one line; the trade site lists each line as its own stat",
        Some("no trade stat") => "no trade site stat for this text",
        Some(other) => other,
        None => "no trade site stat for this text",
    }
    .to_string()
}

/// What the search asks for, and why when it differs from the choice.
fn count_line(query: &TradeQuery, both_slots: bool) -> &'static str {
    let has = |kind: Kind| query.entries.iter().any(|e| e.kind == kind);
    match (has(Kind::Prefix), has(Kind::Suffix), both_slots) {
        (true, true, true) => "The search asks for two: one prefix and one suffix.",
        (_, _, false) => "The search asks for at least one of your picks.",
        (true, false, true) => {
            "Your picks are all prefixes and a magic item holds one, so the search asks for one."
        }
        _ => "Your picks are all suffixes and a magic item holds one, so the search asks for one.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_unreserved_bytes_stay_literal() {
        assert_eq!(percent_encode("Forbidden Rites"), "Forbidden%20Rites");
        assert_eq!(percent_encode("AZaz09-._~"), "AZaz09-._~");
        assert_eq!(percent_encode("../x?y#z"), "..%2Fx%3Fy%23z");
        assert_eq!(percent_encode("a+b%c&d=e"), "a%2Bb%25c%26d%3De");
        assert_eq!(percent_encode("é"), "%C3%A9");
        assert_eq!(percent_encode(r#"{"a":1}"#), "%7B%22a%22%3A1%7D");
    }

    #[test]
    fn a_refusal_reads_as_a_sentence() {
        assert_eq!(
            to_choose("choose an item class", "no item class").sentence(),
            "Choose an item class."
        );
    }
}
