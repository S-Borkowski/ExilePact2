//! Slots, pools and the rough single-step odds of a currency outcome. Never
//! a simulation: one step, from the data's weights, or "unknown" with the
//! reason when the weights are not established.

use ch_item::{Item, ItemClass, Rarity};
use serde::Serialize;

use crate::join::{join, Join};
use crate::lookup::variant_of;
use crate::model::{ClassData, Kind, Section, Tier, WeightBasis};

/// poe2db writes a weight of 1 where it has no real one.
pub const PLACEHOLDER_WEIGHT: u32 = 1;

/// The lines a Contempt liquid emotion adds (data `jewels.json`, family
/// `MaxPrefixMaxSuffix`): as a prefix it allows one more suffix, as a suffix
/// one more prefix.
pub const CONTEMPT_EXTRA_SUFFIX_LINE: &str = "+# Suffix Modifier allowed";
pub const CONTEMPT_EXTRA_PREFIX_LINE: &str = "+# Prefix Modifier allowed";

/// How many prefixes and suffixes an item may carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct AffixLimits {
    pub prefixes: usize,
    pub suffixes: usize,
}

impl AffixLimits {
    pub fn on(&self, side: Kind) -> usize {
        match side {
            Kind::Prefix => self.prefixes,
            Kind::Suffix => self.suffixes,
        }
    }
}

/// Which classes a row of [`AFFIX_LIMITS`] covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum LimitClasses {
    Any,
    Jewels,
    NotJewels,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct LimitRow {
    pub rarity: &'static str,
    pub classes: LimitClasses,
    pub limits: AffixLimits,
}

/// The affix limits, in one table. Not established in the docs: the
/// owner's default (2026-10-29): Magic 1 and 1, Rare 3 and 3, Rare jewels 2
/// and 2. A Contempt liquid emotion on the item adds one on the other side
/// ([`item_affix_limits`]).
pub const AFFIX_LIMITS: [LimitRow; 4] = [
    LimitRow {
        rarity: "Normal",
        classes: LimitClasses::Any,
        limits: AffixLimits {
            prefixes: 0,
            suffixes: 0,
        },
    },
    LimitRow {
        rarity: "Magic",
        classes: LimitClasses::Any,
        limits: AffixLimits {
            prefixes: 1,
            suffixes: 1,
        },
    },
    LimitRow {
        rarity: "Rare",
        classes: LimitClasses::NotJewels,
        limits: AffixLimits {
            prefixes: 3,
            suffixes: 3,
        },
    },
    LimitRow {
        rarity: "Rare",
        classes: LimitClasses::Jewels,
        limits: AffixLimits {
            prefixes: 2,
            suffixes: 2,
        },
    },
];

/// The limits for a rarity and class from [`AFFIX_LIMITS`]; `None` for a
/// unique or any other rarity.
pub fn affix_limits(rarity: &Rarity, class: &ItemClass) -> Option<AffixLimits> {
    let rarity = match rarity {
        Rarity::Normal => "Normal",
        Rarity::Magic => "Magic",
        Rarity::Rare => "Rare",
        _ => return None,
    };
    let jewel = *class == ItemClass::Jewels;
    AFFIX_LIMITS
        .iter()
        .find(|row| {
            row.rarity == rarity
                && match row.classes {
                    LimitClasses::Any => true,
                    LimitClasses::Jewels => jewel,
                    LimitClasses::NotJewels => !jewel,
                }
        })
        .map(|row| row.limits)
}

/// The limits of `item` were it of `rarity`, with the slots a Contempt
/// liquid emotion on it adds.
pub fn item_affix_limits(item: &Item, rarity: &Rarity) -> Option<AffixLimits> {
    let mut limits = affix_limits(rarity, &item.class)?;
    for m in &item.modifiers {
        for line in &m.lines {
            let extra = line.values.first().map_or(1, |v| v.value.max(0.0) as usize);
            if line.template == CONTEMPT_EXTRA_SUFFIX_LINE {
                limits.suffixes += extra;
            } else if line.template == CONTEMPT_EXTRA_PREFIX_LINE {
                limits.prefixes += extra;
            }
        }
    }
    Some(limits)
}

/// A rough single-step chance: known, with its basis, or unknown, with the
/// reason.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "odds", rename_all = "kebab-case")]
pub enum Odds {
    Known {
        /// 0 to 100.
        percent: f64,
        hits: u64,
        total: u64,
        /// What `hits` and `total` count (`8000 of 69500 weight`).
        basis: String,
    },
    Unknown {
        reason: String,
    },
}

impl Odds {
    pub fn known(hits: u64, total: u64, unit: &str) -> Odds {
        let percent = if total == 0 {
            0.0
        } else {
            hits as f64 * 100.0 / total as f64
        };
        Odds::Known {
            percent,
            hits,
            total,
            basis: format!("{hits} of {total} {unit}"),
        }
    }

    /// Certain to happen (`true`) or certain not to (`false`).
    pub fn certain(yes: bool, basis: impl Into<String>) -> Odds {
        Odds::Known {
            percent: if yes { 100.0 } else { 0.0 },
            hits: u64::from(yes),
            total: 1,
            basis: basis.into(),
        }
    }

    pub fn unknown(reason: impl Into<String>) -> Odds {
        Odds::Unknown {
            reason: reason.into(),
        }
    }

    pub fn percent(&self) -> Option<f64> {
        match self {
            Odds::Known { percent, .. } => Some(*percent),
            Odds::Unknown { .. } => None,
        }
    }

    /// Both known: their product (an add times a removal factor). Unknown
    /// if either is.
    pub fn times(&self, other: &Odds) -> Odds {
        match (self, other) {
            (
                Odds::Known {
                    percent: a,
                    hits: ha,
                    total: ta,
                    basis: ba,
                },
                Odds::Known {
                    percent: b,
                    hits: hb,
                    total: tb,
                    basis: bb,
                },
            ) => Odds::Known {
                percent: a * b / 100.0,
                hits: ha * hb,
                total: ta * tb,
                basis: format!("({ba}) times ({bb})"),
            },
            (Odds::Unknown { reason }, _) | (_, Odds::Unknown { reason }) => {
                Odds::unknown(reason.clone())
            }
        }
    }
}

/// One modifier a random add can roll, with the tiers it can roll.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PoolEntry<'a> {
    pub id: &'a str,
    pub kind: Kind,
    pub tiers: Vec<&'a Tier>,
}

/// What a currency adding one random modifier can roll.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Pool<'a> {
    pub sides: Vec<Kind>,
    pub entries: Vec<PoolEntry<'a>>,
}

impl Pool<'_> {
    /// The chance the add rolls `target` at tier `floor` or better: the
    /// target's counting weight over the pool's. Unknown when a counting
    /// target tier's weight is missing or the placeholder 1, or any pool
    /// tier's weight is missing; a 1 elsewhere counts as 1.
    pub fn odds(&self, target: &str, floor: u32) -> Odds {
        let mut total: u64 = 0;
        for e in &self.entries {
            for t in &e.tiers {
                match t.weight {
                    Some(w) => total += u64::from(w),
                    None => {
                        return Odds::unknown(format!(
                            "{} tier {} has no weight in the data",
                            e.id, t.tier
                        ))
                    }
                }
            }
        }
        let Some(entry) = self.entries.iter().find(|e| e.id == target) else {
            return Odds::known(0, total, "weight (the target is not in the pool)");
        };
        let mut hits: u64 = 0;
        for t in entry.tiers.iter().filter(|t| t.tier <= floor) {
            match t.weight {
                Some(w) if w != PLACEHOLDER_WEIGHT => hits += u64::from(w),
                _ => {
                    return Odds::unknown(format!(
                        "{target} tier {} has no real weight in the data",
                        t.tier
                    ))
                }
            }
        }
        if total == 0 {
            return Odds::unknown("the pool is empty");
        }
        Odds::known(hits, total, "weight")
    }
}

impl ClassData {
    /// The add pool: `normal`-section modifiers on `variant` of the given
    /// sides, minus any sharing a family in `excluded_families`, each with
    /// its tiers at or below `item_level`. With a Minimum Modifier Level,
    /// only tiers at or above it, except that a modifier whose every
    /// reachable tier is below it keeps its highest reachable tier.
    pub fn add_pool(
        &self,
        variant: &str,
        item_level: u32,
        sides: &[Kind],
        excluded_families: &[String],
        min_modifier_level: Option<u32>,
    ) -> Pool<'_> {
        let entries = self
            .mods
            .iter()
            .filter(|m| {
                m.section == Section::Normal
                    && sides.contains(&m.kind)
                    && !m.shares_family(excluded_families)
            })
            .filter_map(|m| {
                let reachable: Vec<&Tier> = m
                    .tiers_on(variant)
                    .iter()
                    .filter(|t| t.ilvl <= item_level)
                    .collect();
                let best = reachable.iter().min_by_key(|t| t.tier).copied()?;
                let tiers = match min_modifier_level {
                    None => reachable,
                    Some(level) => {
                        let above: Vec<&Tier> =
                            reachable.into_iter().filter(|t| t.ilvl >= level).collect();
                        if above.is_empty() {
                            vec![best]
                        } else {
                            above
                        }
                    }
                };
                Some(PoolEntry {
                    id: &m.id,
                    kind: m.kind,
                    tiers,
                })
            })
            .collect();
        Pool {
            sides: sides.to_vec(),
            entries,
        }
    }
}

/// How a currency adding one random modifier is used (an input, not
/// rendered: `ch_item::Rarity` has no serde).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AddOptions {
    /// A side omen's side; `None` lets either side with a free slot roll.
    pub side: Option<Kind>,
    /// Greater and Perfect currencies, Ancient bones.
    pub min_modifier_level: Option<u32>,
    /// The rarity whose limits apply when it differs from the item's (a
    /// regal makes a Magic item Rare, then adds).
    pub rarity: Option<Rarity>,
}

/// The chance that one random add on `item` rolls `target` (a data
/// modifier id) at tier `floor` or better.
pub fn add_odds(item: &Item, target: &str, floor: u32, options: &AddOptions) -> Odds {
    let (data, variant) = match variant_of(item) {
        Ok(found) => found,
        Err(why) => return Odds::unknown(format!("the item's variant is unknown: {why:?}")),
    };
    match data.page(variant).map(|p| p.weights.normal) {
        Some(WeightBasis::Spawn) => {}
        _ => {
            return Odds::unknown(format!(
                "{} {variant} has no established spawn weights",
                data.class
            ))
        }
    }
    let Some(item_level) = item.item_level.as_ref().map(|l| l.value) else {
        return Odds::unknown("the item has no item level");
    };
    if let Some(level) = options.min_modifier_level {
        if item_level < level {
            return Odds::unknown(format!(
                "item level {item_level} is below the currency's minimum modifier level {level}"
            ));
        }
    }
    if data.find_mod(target).is_none() {
        return Odds::unknown(format!("{} has no modifier {target}", data.class));
    }
    let joined = join(item);
    let rarity = options.rarity.as_ref().unwrap_or(&item.rarity);
    let Some(limits) = item_affix_limits(item, rarity) else {
        return Odds::unknown("no affix limits for this rarity");
    };
    let mut families: Vec<String> = Vec::new();
    for m in &joined.mods {
        match m.matched() {
            Some(found) => families.extend(found.family.iter().cloned()),
            None => {
                return Odds::unknown(format!(
                    "modifier {:?} on the item did not join: {:?}",
                    m.affix_name, m.result
                ))
            }
        }
    }
    let sides: Vec<Kind> = [Kind::Prefix, Kind::Suffix]
        .into_iter()
        .filter(|side| options.side.is_none_or(|s| s == *side))
        .filter(|side| count_side(&joined, *side) < limits.on(*side))
        .collect();
    if sides.is_empty() {
        return Odds::unknown("no free slot on the allowed side");
    }
    data.add_pool(
        variant,
        item_level,
        &sides,
        &families,
        options.min_modifier_level,
    )
    .odds(target, floor)
}

fn count_side(joined: &Join, side: Kind) -> usize {
    joined.mods.iter().filter(|m| m.kind == side).count()
}

/// Which modifier a removal takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Removal {
    /// A random removable one (chaos, annulment, perfect essence, liquid).
    Random,
    /// The lowest modifier level (Omen of Whittling with chaos).
    Whittle,
    /// The desecrated one (Omen of Light with annulment).
    Desecrated,
}

/// The chance the removed modifier is not a wanted one. `wanted` holds
/// positions in `Item::modifiers` (as `JoinedMod::index` does); `side`
/// narrows the removal to one side. A fractured modifier is never removed.
pub fn removal_odds(joined: &Join, wanted: &[usize], side: Option<Kind>, rule: Removal) -> Odds {
    let removable: Vec<_> = joined
        .mods
        .iter()
        .filter(|m| !m.fractured && side.is_none_or(|s| s == m.kind))
        .collect();
    let is_wanted = |index: usize| wanted.contains(&index);
    match rule {
        Removal::Random => {
            if removable.is_empty() {
                return Odds::unknown("no removable modifier");
            }
            let hits = removable.iter().filter(|m| !is_wanted(m.index)).count() as u64;
            Odds::known(
                hits,
                removable.len() as u64,
                "removable modifiers are not wanted",
            )
        }
        Removal::Whittle => {
            let mut levels = Vec::new();
            for m in &removable {
                match m.matched() {
                    Some(found) => levels.push((found.tier_ilvl, m.index)),
                    None => {
                        return Odds::unknown(format!(
                            "modifier {:?} did not join, so its level is unknown",
                            m.affix_name
                        ))
                    }
                }
            }
            let Some(lowest) = levels.iter().map(|(level, _)| *level).min() else {
                return Odds::unknown("no removable modifier");
            };
            let at_lowest: Vec<usize> = levels
                .iter()
                .filter(|(level, _)| *level == lowest)
                .map(|(_, index)| *index)
                .collect();
            match at_lowest.as_slice() {
                [only] => Odds::certain(
                    !is_wanted(*only),
                    format!("the lowest modifier level ({lowest}) is unique"),
                ),
                _ => Odds::unknown(format!(
                    "{} modifiers share the lowest modifier level {lowest}",
                    at_lowest.len()
                )),
            }
        }
        Removal::Desecrated => {
            let desecrated: Vec<_> = removable.iter().filter(|m| m.desecrated).collect();
            match desecrated.as_slice() {
                [] => Odds::unknown("no desecrated modifier"),
                [only] => Odds::certain(!is_wanted(only.index), "the one desecrated modifier"),
                several => {
                    let hits = several.iter().filter(|m| !is_wanted(m.index)).count() as u64;
                    Odds::known(
                        hits,
                        several.len() as u64,
                        "desecrated modifiers are not wanted",
                    )
                }
            }
        }
    }
}
