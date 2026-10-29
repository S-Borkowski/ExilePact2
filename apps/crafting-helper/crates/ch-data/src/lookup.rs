//! Lookups on one class's data: the variant from the base type, the
//! modifiers valid for a variant and item level, a modifier's tiers, and the
//! essences and liquid emotions that add a modifier at a tier or better.

use ch_item::Item;
use serde::Serialize;

use crate::embedded::class_data;
use crate::model::{ClassData, DataMod, Essence, Kind, Page, Section, Tier};

/// Why an item's variant is unknown. Lookups that need a variant answer
/// "unknown" then, never a guess.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "reason", rename_all = "kebab-case")]
pub enum VariantUnknown {
    /// The item's class is not one of the 22 (`ItemClass::Other`).
    ClassOutOfScope { class: String },
    /// The text gave no base type (a magic item in the short form).
    NoBaseType,
    /// The base type is not among the class's bases.
    BaseNotInData { base_type: String },
}

/// The class data and variant of a parsed item.
pub fn variant_of(item: &Item) -> Result<(&'static ClassData, &'static str), VariantUnknown> {
    let data = class_data(&item.class).ok_or_else(|| VariantUnknown::ClassOutOfScope {
        class: item.class.as_str().to_string(),
    })?;
    let base = item
        .base_type
        .as_deref()
        .ok_or(VariantUnknown::NoBaseType)?;
    let variant = data
        .variant_for_base(base)
        .ok_or_else(|| VariantUnknown::BaseNotInData {
            base_type: base.to_string(),
        })?;
    Ok((data, variant))
}

/// A modifier an item can roll, with the tiers its item level reaches.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ValidMod<'a> {
    pub id: &'a str,
    pub section: Section,
    pub kind: Kind,
    pub family: &'a [String],
    pub lines: &'a [String],
    /// The tiers on the variant at or below the item level, best first.
    pub tiers: Vec<&'a Tier>,
    /// The lowest tier number among `tiers`.
    pub best_tier: u32,
}

/// Every modifier valid for a variant and item level.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ValidMods<'a> {
    /// The `normal` section: what random currencies roll.
    pub normal: Vec<ValidMod<'a>>,
    /// The `desecrated` section: by desecration only.
    pub desecrated: Vec<ValidMod<'a>>,
}

/// An essence or liquid emotion that adds a given modifier, with the tier it
/// adds on the variant.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EssenceFor<'a> {
    pub essence: &'a Essence,
    pub tier: u32,
}

impl ClassData {
    /// The variant of the base whose name equals `base_type` exactly (for
    /// rings `Amethyst Ring`, not `Ring`).
    pub fn variant_for_base(&self, base_type: &str) -> Option<&str> {
        self.bases
            .iter()
            .find(|b| b.name == base_type)
            .map(|b| b.variant.as_str())
    }

    /// The page `variant`'s modifiers came from.
    pub fn page(&self, variant: &str) -> Option<&Page> {
        self.pages.iter().find(|p| p.variant == variant)
    }

    /// The modifier with this id.
    pub fn find_mod(&self, id: &str) -> Option<&DataMod> {
        self.mods.iter().find(|m| m.id == id)
    }

    /// The modifier's tiers on `variant`, best first; `None` when the class
    /// has no such modifier, empty when it has no tier on the variant.
    pub fn mod_tiers(&self, id: &str, variant: &str) -> Option<&[Tier]> {
        self.find_mod(id).map(|m| m.tiers_on(variant))
    }

    /// Every modifier with a tier on `variant` whose level is at or below
    /// `item_level`, with those tiers and the best of them.
    pub fn valid_mods(&self, variant: &str, item_level: u32) -> ValidMods<'_> {
        let mut out = ValidMods {
            normal: Vec::new(),
            desecrated: Vec::new(),
        };
        for m in &self.mods {
            let tiers: Vec<&Tier> = m
                .tiers_on(variant)
                .iter()
                .filter(|t| t.ilvl <= item_level)
                .collect();
            let Some(best_tier) = tiers.iter().map(|t| t.tier).min() else {
                continue;
            };
            let valid = ValidMod {
                id: &m.id,
                section: m.section,
                kind: m.kind,
                family: &m.family,
                lines: &m.lines,
                tiers,
                best_tier,
            };
            match m.section {
                Section::Normal => out.normal.push(valid),
                Section::Desecrated => out.desecrated.push(valid),
            }
        }
        out
    }

    /// The essences (alloys and perfect essences included) whose match on
    /// `variant` is `mod_id` at tier `floor` or better.
    pub fn essences_for(&self, mod_id: &str, variant: &str, floor: u32) -> Vec<EssenceFor<'_>> {
        matching(&self.essences, mod_id, variant, floor)
    }

    /// The liquid emotions whose match on `variant` is `mod_id` at tier
    /// `floor` or better.
    pub fn liquids_for(&self, mod_id: &str, variant: &str, floor: u32) -> Vec<EssenceFor<'_>> {
        matching(&self.liquid, mod_id, variant, floor)
    }
}

fn matching<'a>(
    entries: &'a [Essence],
    mod_id: &str,
    variant: &str,
    floor: u32,
) -> Vec<EssenceFor<'a>> {
    entries
        .iter()
        .filter_map(|e| {
            let m = e.matches.get(variant)?.as_ref()?;
            (m.mod_id == mod_id && m.tier <= floor).then_some(EssenceFor {
                essence: e,
                tier: m.tier,
            })
        })
        .collect()
}
