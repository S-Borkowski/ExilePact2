//! The join: each explicit modifier of a parsed item matched to the data
//! modifier and tier it is.
//!
//! 1. Candidates are the class's modifiers of the same side whose `lines`
//!    equal the modifier's line templates, in order, in either section (a
//!    revealed desecrated modifier on a real item joins a `normal` one).
//! 2. Only candidates with tiers on the item's variant stay.
//! 3. The tier is the one whose name equals the header's affix name; when
//!    no name or several match, the one whose every range holds the
//!    modifier's values. Which rule decided is kept.
//!
//! An unrevealed modifier is never joined; implicit, rune, enchant and
//! corruption modifiers are not explicit and are left out.

use ch_item::{Item, Modifier};
use serde::Serialize;

use crate::lookup::{variant_of, VariantUnknown};
use crate::model::{ClassData, DataMod, Kind, Section, Tier};

/// Slack for comparing a decimal value against a range bound.
const VALUE_EPSILON: f64 = 1e-6;

/// The joined explicit modifiers of one item.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Join {
    /// The item's variant, or why it is unknown (then nothing joins).
    pub variant: Result<String, VariantUnknown>,
    /// One entry per prefix or suffix modifier, in text order.
    pub mods: Vec<JoinedMod>,
}

/// One explicit modifier and what it joined to.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct JoinedMod {
    /// Its position in `Item::modifiers`.
    pub index: usize,
    pub kind: Kind,
    pub affix_name: Option<String>,
    /// The `Tier:` the header shows.
    pub game_tier: Option<u32>,
    pub fractured: bool,
    pub desecrated: bool,
    pub unrevealed: bool,
    pub result: JoinResult,
}

impl JoinedMod {
    /// The match, when there is one.
    pub fn matched(&self) -> Option<&Matched> {
        match &self.result {
            JoinResult::Matched(m) => Some(m),
            JoinResult::Unmatched { .. } => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub enum JoinResult {
    Matched(Matched),
    Unmatched { reason: Unmatched },
}

/// The data modifier and tier a modifier joined to.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Matched {
    pub mod_id: String,
    pub section: Section,
    pub family: Vec<String>,
    /// The derived tier (1 best).
    pub tier: u32,
    /// The tier's modifier level: what whittling compares.
    pub tier_ilvl: u32,
    pub decided_by: TierRule,
}

/// Which rule picked the tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TierRule {
    AffixName,
    Values,
}

/// Why a modifier did not join.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "why", rename_all = "kebab-case")]
pub enum Unmatched {
    /// Not yet revealed: its lines are not the modifier's.
    Unrevealed,
    /// The item's variant is unknown.
    VariantUnknown,
    /// No modifier of the class has these lines on this side.
    NoCandidate,
    /// Modifiers with these lines exist, none with tiers on the variant.
    NotOnVariant { candidates: Vec<String> },
    /// No tier's name or ranges fit.
    NoTier { candidates: Vec<String> },
    /// More than one modifier or tier fits.
    Ambiguous { candidates: Vec<String> },
}

/// Join every prefix and suffix modifier of `item`.
pub fn join(item: &Item) -> Join {
    let found = variant_of(item);
    let mods = item
        .modifiers
        .iter()
        .enumerate()
        .filter_map(|(index, m)| {
            let kind = Kind::from_item(m.kind)?;
            let result = match &found {
                Err(_) => JoinResult::Unmatched {
                    reason: Unmatched::VariantUnknown,
                },
                Ok((data, variant)) => join_one(data, variant, kind, m),
            };
            Some(JoinedMod {
                index,
                kind,
                affix_name: m.affix_name.clone(),
                game_tier: m.tier,
                fractured: m.fractured,
                desecrated: m.desecrated,
                unrevealed: m.unrevealed,
                result,
            })
        })
        .collect();
    Join {
        variant: found.map(|(_, v)| v.to_string()),
        mods,
    }
}

/// Join one modifier against the class data on `variant`.
pub fn join_one(data: &ClassData, variant: &str, kind: Kind, m: &Modifier) -> JoinResult {
    let unmatched = |reason| JoinResult::Unmatched { reason };
    if m.unrevealed {
        return unmatched(Unmatched::Unrevealed);
    }
    let candidates: Vec<&DataMod> = data
        .mods
        .iter()
        .filter(|d| {
            d.kind == kind
                && d.lines.len() == m.lines.len()
                && d.lines.iter().zip(&m.lines).all(|(a, b)| *a == b.template)
        })
        .collect();
    if candidates.is_empty() {
        return unmatched(Unmatched::NoCandidate);
    }
    let ids = |list: &[&DataMod]| list.iter().map(|d| d.id.clone()).collect::<Vec<_>>();
    let on_variant: Vec<&DataMod> = candidates
        .iter()
        .copied()
        .filter(|d| !d.tiers_on(variant).is_empty())
        .collect();
    if on_variant.is_empty() {
        return unmatched(Unmatched::NotOnVariant {
            candidates: ids(&candidates),
        });
    }

    let values: Vec<f64> = m
        .lines
        .iter()
        .flat_map(|l| l.values.iter().map(|v| v.value))
        .collect();

    // By affix name: one tier named so, across every candidate; when several
    // are, the one of them whose ranges hold the values.
    if let Some(name) = &m.affix_name {
        let named: Vec<(&DataMod, &Tier)> = on_variant
            .iter()
            .flat_map(|d| {
                d.tiers_on(variant)
                    .iter()
                    .filter(|t| &t.name == name)
                    .map(move |t| (*d, t))
            })
            .collect();
        if let [(d, t)] = named.as_slice() {
            return matched(d, t, TierRule::AffixName);
        }
        let named_fitting: Vec<&(&DataMod, &Tier)> =
            named.iter().filter(|(_, t)| holds(t, &values)).collect();
        if let [(d, t)] = named_fitting.as_slice() {
            return matched(d, t, TierRule::Values);
        }
    }

    // By values: the tier whose every range holds the modifier's values.
    let fitting: Vec<(&DataMod, &Tier)> = on_variant
        .iter()
        .flat_map(|d| {
            d.tiers_on(variant)
                .iter()
                .filter(|t| holds(t, &values))
                .map(move |t| (*d, t))
        })
        .collect();
    match fitting.as_slice() {
        [(d, t)] => matched(d, t, TierRule::Values),
        [] => unmatched(Unmatched::NoTier {
            candidates: ids(&on_variant),
        }),
        several => {
            let mut names: Vec<String> = several
                .iter()
                .map(|(d, t)| format!("{} T{}", d.id, t.tier))
                .collect();
            names.dedup();
            unmatched(Unmatched::Ambiguous { candidates: names })
        }
    }
}

fn matched(d: &DataMod, t: &Tier, rule: TierRule) -> JoinResult {
    JoinResult::Matched(Matched {
        mod_id: d.id.clone(),
        section: d.section,
        family: d.family.clone(),
        tier: t.tier,
        tier_ilvl: t.ilvl,
        decided_by: rule,
    })
}

/// Every value lies within its tier range (in either bound order).
fn holds(t: &Tier, values: &[f64]) -> bool {
    t.values.len() == values.len()
        && t.values.iter().zip(values).all(|([a, b], v)| {
            let (lo, hi) = if a <= b { (*a, *b) } else { (*b, *a) };
            *v >= lo - VALUE_EPSILON && *v <= hi + VALUE_EPSILON
        })
}
