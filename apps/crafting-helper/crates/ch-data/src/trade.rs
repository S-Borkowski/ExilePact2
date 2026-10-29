//! Starting a craft from a bought base: how rare each ordinary modifier is to
//! roll on a base (its start weight), the modifiers a base offers a trade
//! search, and the trade site's search query for a pick of them.
//!
//! Pure and offline: the query is a `serde_json::Value`; the address, the
//! league and the link are the app's (`ch-app`), and nothing here talks to
//! the trade site. The stat ids come from `data/trade-stats.json`, written by
//! `tools/crafting_helper/trade_stats.py` from the site's public stat list.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use crate::embedded::{class_data_by_name, trade_stats};
use crate::model::{ClassData, Kind, Section, WeightBasis};
use crate::odds::{Odds, Pool};

/// The most modifiers one search may carry.
pub const MAX_TRADE_PICKS: usize = 8;

/// The default weight when no modifier of the side has a spawn weight to
/// take the median of.
pub const DEFAULT_WEIGHT_WITHOUT_PEERS: f64 = 1.0;

/// `data/trade-stats.json`: per class, per modifier id, its trade stat or
/// why it has none.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeStats {
    pub schema: u32,
    /// The trade site's stat list the mapping was made from.
    pub source: String,
    /// ISO date of the fetch.
    pub fetched: String,
    pub source_note: String,
    pub classes: BTreeMap<String, BTreeMap<String, TradeStat>>,
}

/// One modifier's entry in the mapping.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TradeStat {
    /// The trade site's text and every id carrying it (a duplicate text
    /// keeps both ids).
    Mapped { text: String, ids: Vec<String> },
    /// `hybrid` (more than one line) or `no trade stat`.
    NotSearchable { searchable: bool, reason: String },
}

impl TradeStats {
    pub fn get(&self, class: &str, mod_id: &str) -> Option<&TradeStat> {
        self.classes.get(class)?.get(mod_id)
    }

    /// The stat ids of a mapped modifier.
    pub fn ids(&self, class: &str, mod_id: &str) -> Option<&Vec<String>> {
        match self.get(class, mod_id)? {
            TradeStat::Mapped { ids, .. } => Some(ids),
            TradeStat::NotSearchable { .. } => None,
        }
    }

    /// The trade site's text of a mapped modifier.
    pub fn text(&self, class: &str, mod_id: &str) -> Option<&str> {
        match self.get(class, mod_id)? {
            TradeStat::Mapped { text, .. } => Some(text),
            TradeStat::NotSearchable { .. } => None,
        }
    }
}

/// Whether a modifier can be searched for, flattened for a view.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Searchability {
    pub searchable: bool,
    pub text: Option<String>,
    pub ids: Vec<String>,
    /// Why not, when not.
    pub reason: Option<String>,
}

/// The mapping's answer for one modifier of a class. A modifier of more
/// than one line is a hybrid whatever the mapping says: the site lists
/// each line as its own stat.
pub fn searchability(class: &str, mod_id: &str, lines: usize) -> Searchability {
    let not = |reason: &str| Searchability {
        searchable: false,
        text: None,
        ids: Vec::new(),
        reason: Some(reason.to_string()),
    };
    if lines > 1 {
        return not("hybrid");
    }
    match trade_stats().get(class, mod_id) {
        Some(TradeStat::Mapped { text, ids }) if !ids.is_empty() => Searchability {
            searchable: true,
            text: Some(text.clone()),
            ids: ids.clone(),
            reason: None,
        },
        Some(TradeStat::NotSearchable { reason, .. }) => not(reason),
        _ => not("not in the trade stat mapping"),
    }
}

/// Where a start weight came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum WeightSource {
    /// From the data's spawn weights.
    Spawn,
    /// The data has no usable weight; the stated default stands in.
    Default,
}

/// How rare a modifier is to roll on a base, at a floor: a 1-in-N add,
/// higher the rarer.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StartWeight {
    pub weight: f64,
    pub source: WeightSource,
    /// The tier number the weight counts down to: the pick's floor, or for
    /// "any tier" the worst reachable tier.
    pub floor: u32,
    /// The odds of one random add on that side rolling the modifier at the
    /// floor or better, or why they are unknown.
    pub odds: Odds,
    /// For a default: why the data gave no weight, and what the default is.
    pub reason: Option<String>,
}

/// The item level that reaches every tier: no item level given.
fn level_or_all(item_level: Option<u32>) -> u32 {
    item_level.unwrap_or(u32::MAX)
}

/// One side's pool on a variant, with the spawn weight (any tier) of each
/// modifier that has one, for the default.
struct Side<'a> {
    pool: Pool<'a>,
    spawn: bool,
    basis_reason: String,
    median: Option<(f64, usize)>,
}

impl<'a> Side<'a> {
    fn new(data: &'a ClassData, variant: &str, item_level: u32, kind: Kind) -> Side<'a> {
        let pool = data.add_pool(variant, item_level, &[kind], &[], None);
        let spawn = matches!(
            data.page(variant).map(|p| p.weights.normal),
            Some(WeightBasis::Spawn)
        );
        let basis_reason = format!("{} {variant} has no established spawn weights", data.class);
        let mut weighed: Vec<f64> = if spawn {
            pool.entries
                .iter()
                .filter_map(|e| {
                    let worst = e.tiers.iter().map(|t| t.tier).max()?;
                    rarity(&pool.odds(e.id, worst))
                })
                .collect()
        } else {
            Vec::new()
        };
        weighed.sort_by(f64::total_cmp);
        let median =
            (!weighed.is_empty()).then(|| (weighed[(weighed.len() - 1) / 2], weighed.len()));
        Side {
            pool,
            spawn,
            basis_reason,
            median,
        }
    }

    fn weight(
        &self,
        data: &ClassData,
        mod_id: &str,
        floor: Option<u32>,
    ) -> Result<StartWeight, String> {
        let Some(entry) = self.pool.entries.iter().find(|e| e.id == mod_id) else {
            return Err(format!(
                "{mod_id} has no reachable tier on this {} base at this item level",
                data.class
            ));
        };
        let worst = entry.tiers.iter().map(|t| t.tier).max().unwrap_or(1);
        let floor = match floor {
            None => worst,
            Some(n) if entry.tiers.iter().any(|t| t.tier == n) => n,
            Some(n) => {
                return Err(format!(
                    "T{n} is not a reachable tier of {mod_id} on this base at this item level"
                ))
            }
        };
        let side = match entry.kind {
            Kind::Prefix => "prefix",
            Kind::Suffix => "suffix",
        };
        let (odds, why) = if self.spawn {
            let odds = self.pool.odds(mod_id, floor);
            match rarity(&odds) {
                Some(weight) => {
                    return Ok(StartWeight {
                        weight,
                        source: WeightSource::Spawn,
                        floor,
                        odds,
                        reason: None,
                    })
                }
                None => {
                    let why = match &odds {
                        Odds::Unknown { reason } => reason.clone(),
                        Odds::Known { .. } => format!("{mod_id} adds with no weight"),
                    };
                    (odds, why)
                }
            }
        } else {
            (
                Odds::unknown(self.basis_reason.clone()),
                self.basis_reason.clone(),
            )
        };
        let (weight, rule) = match self.median {
            Some((median, count)) => (
                median,
                format!("the default is the median of the {count} {side} weights on this base"),
            ),
            None => (
                DEFAULT_WEIGHT_WITHOUT_PEERS,
                format!("the default is 1.0, as no {side} on this base has a spawn weight"),
            ),
        };
        Ok(StartWeight {
            weight,
            source: WeightSource::Default,
            floor,
            odds,
            reason: Some(format!("{why}; {rule}")),
        })
    }
}

/// A known add with a non-zero chance as a 1-in-N weight.
fn rarity(odds: &Odds) -> Option<f64> {
    match odds {
        Odds::Known { hits, total, .. } if *hits > 0 && *total > 0 => {
            Some(*total as f64 / *hits as f64)
        }
        _ => None,
    }
}

/// The start weight of `mod_id` on `variant` at `floor` (a tier number, 1
/// the best; `None` for any tier): the add pool of its own side, total over
/// the weight of its tiers at the floor or better. Where the data has no
/// usable weight, the default (the median of that side's weighed modifiers
/// on the base, or 1.0 when none is), labelled, with the reason. No item
/// level reaches every tier. Refused for a modifier that is not in the
/// ordinary pool or has no reachable tier, and for a floor that is not a
/// reachable tier.
pub fn start_weight(
    data: &ClassData,
    variant: &str,
    item_level: Option<u32>,
    mod_id: &str,
    floor: Option<u32>,
) -> Result<StartWeight, String> {
    let m = data
        .find_mod(mod_id)
        .ok_or_else(|| format!("{} has no modifier {mod_id}", data.class))?;
    if m.section != Section::Normal {
        return Err(format!("{mod_id} is not in the ordinary pool"));
    }
    Side::new(data, variant, level_or_all(item_level), m.kind).weight(data, mod_id, floor)
}

/// One modifier a base offers the search.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OfferedMod<'a> {
    pub id: &'a str,
    pub kind: Kind,
    pub lines: &'a [String],
    /// The reachable tier numbers, best first.
    pub tiers: Vec<u32>,
    /// The start weight at "any tier".
    pub weight: StartWeight,
    pub search: Searchability,
}

/// The ordinary-pool modifiers a base offers, each side apart.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TradeOffer<'a> {
    pub class: &'a str,
    pub base: &'a str,
    pub variant: &'a str,
    pub item_level: Option<u32>,
    pub prefixes: Vec<OfferedMod<'a>>,
    pub suffixes: Vec<OfferedMod<'a>>,
}

fn base_of(
    class: &str,
    base: &str,
) -> Result<(&'static ClassData, &'static crate::model::Base), String> {
    let data =
        class_data_by_name(class).ok_or_else(|| format!("no data for item class {class}"))?;
    let found = data
        .bases
        .iter()
        .find(|b| b.name == base)
        .ok_or_else(|| format!("{class} has no base {base}"))?;
    Ok((data, found))
}

/// Every ordinary-pool modifier with a reachable tier on `base` at
/// `item_level` (none: every tier), with its weight and whether it can be
/// searched for.
pub fn trade_offer(
    class: &str,
    base: &str,
    item_level: Option<u32>,
) -> Result<TradeOffer<'static>, String> {
    let (data, found) = base_of(class, base)?;
    let variant = found.variant.as_str();
    let level = level_or_all(item_level);
    let mut offer = TradeOffer {
        class: data.class.as_str(),
        base: found.name.as_str(),
        variant,
        item_level,
        prefixes: Vec::new(),
        suffixes: Vec::new(),
    };
    for kind in [Kind::Prefix, Kind::Suffix] {
        let side = Side::new(data, variant, level, kind);
        let mut mods = Vec::new();
        for entry in &side.pool.entries {
            let Some(m) = data.find_mod(entry.id) else {
                continue;
            };
            let mut tiers: Vec<u32> = entry.tiers.iter().map(|t| t.tier).collect();
            tiers.sort_unstable();
            mods.push(OfferedMod {
                id: m.id.as_str(),
                kind,
                lines: &m.lines,
                tiers,
                weight: side.weight(data, entry.id, None)?,
                search: searchability(&data.class, entry.id, m.lines.len()),
            });
        }
        match kind {
            Kind::Prefix => offer.prefixes = mods,
            Kind::Suffix => offer.suffixes = mods,
        }
    }
    Ok(offer)
}

/// One modifier the player wants: a data modifier id and its floor (a tier
/// number; `None` for any tier).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TradePick {
    pub id: String,
    pub floor: Option<u32>,
}

/// The panel's selection, as the query needs it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TradeSearch {
    /// As the `Item Class:` line writes it (`Rings`).
    pub class: String,
    /// The base type (`Amethyst Ring`).
    pub base: String,
    pub item_level: Option<u32>,
    pub picks: Vec<TradePick>,
    /// *Both slots*: ask for a prefix and a suffix when the picks have both.
    pub both_slots: bool,
}

/// What the query says about one pick.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TradeEntry {
    pub id: String,
    pub kind: Kind,
    pub floor: Option<u32>,
    /// The rarity weight at the floor.
    pub weight: StartWeight,
    /// The floor tier's minimum roll as the site values it (the first `#`'s
    /// minimum; the mean of the minimums for a two-`#` line); `None` for a
    /// modifier without a value.
    pub reference: Option<f64>,
    /// The weight group's weight: the rarity weight over the reference value
    /// (four significant digits).
    pub trade_weight: f64,
    pub stat_ids: Vec<String>,
}

/// The trade query for a selection, with what it says about each pick.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TradeQuery {
    /// The site's search JSON: what goes after `?q=`, before encoding.
    pub query: Value,
    /// How many of the picks the item must carry.
    pub count: u32,
    /// Why that count, for the panel.
    pub count_line: String,
    pub entries: Vec<TradeEntry>,
}

/// The reference value of a tier: the minimum of the first line's `#`s,
/// averaged when the line has two.
fn reference_value(first_line: &str, values: &[[f64; 2]]) -> Option<f64> {
    let hashes = first_line.matches('#').count().min(values.len());
    if hashes == 0 {
        return None;
    }
    let sum: f64 = values[..hashes].iter().map(|v| v[0]).sum();
    let mean = sum / hashes as f64;
    (mean.abs() > 0.0).then_some(mean.abs())
}

fn round_significant(x: f64, digits: i32) -> f64 {
    if x == 0.0 || !x.is_finite() {
        return x;
    }
    let scale = 10f64.powi(digits - 1 - x.abs().log10().floor() as i32);
    (x * scale).round() / scale
}

/// A whole number as a JSON integer, so the link stays short.
fn number(x: f64) -> Value {
    if x.fract() == 0.0 && x.abs() < 1e15 {
        json!(x as i64)
    } else {
        json!(x)
    }
}

/// The trade site's search for a magic item of the selection's base
/// carrying the picked modifiers: a count group (2 when the picks have a
/// prefix and a suffix and *Both slots* is on, else 1; a value floor only on
/// a pick whose floor is a chosen tier) and a weight group ranking by the
/// trade weights, sorted by it. Refused, with the reason, for an unknown
/// class or base, no pick, more than [`MAX_TRADE_PICKS`], a modifier picked
/// twice, one not in the base's ordinary pool, a floor not reachable, or a
/// modifier the site has no stat for.
pub fn trade_query(search: &TradeSearch) -> Result<TradeQuery, String> {
    let (data, base) = base_of(&search.class, &search.base)?;
    if search.picks.is_empty() {
        return Err("pick at least one modifier".to_string());
    }
    if search.picks.len() > MAX_TRADE_PICKS {
        return Err(format!(
            "at most {MAX_TRADE_PICKS} modifiers per search; {} are picked",
            search.picks.len()
        ));
    }
    let variant = base.variant.as_str();
    let level = level_or_all(search.item_level);
    let prefixes = Side::new(data, variant, level, Kind::Prefix);
    let suffixes = Side::new(data, variant, level, Kind::Suffix);
    let mut entries: Vec<TradeEntry> = Vec::new();
    for pick in &search.picks {
        if entries.iter().any(|e| e.id == pick.id) {
            return Err(format!("{} is picked twice", pick.id));
        }
        let m = data
            .find_mod(&pick.id)
            .ok_or_else(|| format!("{} has no modifier {}", data.class, pick.id))?;
        if m.section != Section::Normal {
            return Err(format!("{} is not in the ordinary pool", pick.id));
        }
        let side = match m.kind {
            Kind::Prefix => &prefixes,
            Kind::Suffix => &suffixes,
        };
        let weight = side.weight(data, &m.id, pick.floor)?;
        let search_for = searchability(&data.class, &m.id, m.lines.len());
        if !search_for.searchable {
            return Err(format!(
                "{} cannot be searched on the trade site: {}",
                m.id,
                search_for.reason.unwrap_or_default()
            ));
        }
        let tier = m
            .tiers_on(variant)
            .iter()
            .find(|t| t.tier == weight.floor)
            .ok_or_else(|| format!("{} has no tier {} on {variant}", m.id, weight.floor))?;
        let reference = m
            .lines
            .first()
            .and_then(|line| reference_value(line, &tier.values));
        let trade_weight = round_significant(weight.weight / reference.unwrap_or(1.0), 4);
        entries.push(TradeEntry {
            id: m.id.clone(),
            kind: m.kind,
            floor: pick.floor,
            weight,
            reference,
            trade_weight,
            stat_ids: search_for.ids,
        });
    }

    let has = |kind: Kind| entries.iter().any(|e| e.kind == kind);
    let (count, count_line) = if has(Kind::Prefix) && has(Kind::Suffix) {
        if search.both_slots {
            (
                2,
                "Both slots: 2 of the chosen modifiers, so the item carries a chosen prefix and a chosen suffix.".to_string(),
            )
        } else {
            (
                1,
                "At least one: 1 of the chosen modifiers; the item may carry only one of them."
                    .to_string(),
            )
        }
    } else {
        let (sides, one) = if has(Kind::Prefix) {
            ("prefixes", "prefix")
        } else {
            ("suffixes", "suffix")
        };
        (
            1,
            format!("1 of the chosen modifiers: they are all {sides}, and a magic item holds one {one}."),
        )
    };

    let mut count_filters = Vec::new();
    let mut weight_filters = Vec::new();
    for e in &entries {
        for id in &e.stat_ids {
            let mut filter = Map::new();
            filter.insert("id".to_string(), json!(id));
            if let (Some(_), Some(reference)) = (e.floor, e.reference) {
                filter.insert("value".to_string(), json!({ "min": number(reference) }));
            }
            count_filters.push(Value::Object(filter));
            weight_filters.push(json!({ "id": id, "value": { "weight": number(e.trade_weight) } }));
        }
    }
    let mut type_filters = Map::new();
    type_filters.insert("rarity".to_string(), json!({ "option": "magic" }));
    if let Some(level) = search.item_level {
        type_filters.insert("ilvl".to_string(), json!({ "min": level }));
    }
    let query = json!({
        "query": {
            "status": { "option": "online" },
            "type": base.name,
            "stats": [
                { "type": "count", "value": { "min": count }, "filters": count_filters },
                { "type": "weight", "value": {}, "filters": weight_filters },
            ],
            "filters": {
                "type_filters": { "filters": type_filters },
                "misc_filters": { "filters": { "corrupted": { "option": "false" } } },
            },
        },
        "sort": { "statgroup.1": "desc" },
    });
    Ok(TradeQuery {
        query,
        count,
        count_line,
        entries,
    })
}
