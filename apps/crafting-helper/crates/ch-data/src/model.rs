//! The shape of `data/index.json` and of each `data/classes/*.json` file, as
//! the extractor (`tools/crafting_helper/poe2db.py`) writes them. Every type
//! here both reads the file and serialises for M5's views.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// `data/index.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Index {
    pub schema: u32,
    pub source_note: String,
    pub classes: Vec<IndexClass>,
}

/// One class entry of the index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexClass {
    /// As the `Item Class:` line writes it (`ch_item::ItemClass::as_str`).
    pub class: String,
    /// Relative to `data/` (`classes/gloves.json`).
    pub file: String,
    pub variants: Vec<String>,
}

/// One `data/classes/*.json` file: every modifier, base, essence and liquid
/// emotion of one item class.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassData {
    pub schema: u32,
    pub class: String,
    /// poe2db's code for the class (`Ring`).
    pub code: String,
    pub source_note: String,
    /// One page per variant, with whether its weights are usable.
    pub pages: Vec<Page>,
    pub bases: Vec<Base>,
    pub mods: Vec<DataMod>,
    /// Essences, perfect essences and alloys (empty for jewels).
    pub essences: Vec<Essence>,
    /// Liquid emotions (jewels only).
    pub liquid: Vec<Essence>,
}

/// The poe2db page a variant's modifiers came from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Page {
    pub variant: String,
    pub url: String,
    pub fetched: String,
    pub weights: PageWeights,
    /// Modifier groups the extractor left out, with their counts.
    pub ignored: BTreeMap<String, u32>,
}

/// What the weights of each section on a page mean.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageWeights {
    pub normal: WeightBasis,
    pub desecrated: WeightBasis,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WeightBasis {
    /// The tiers' `weight` is the game's spawn weight.
    Spawn,
    /// The page shows no usable weight (every jewel page, desecrated
    /// modifiers, `Body_Armours_str_dex_int`).
    NotEstablished,
    /// The page has no modifier of this section.
    None,
}

/// One base type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Base {
    /// As the item text writes the base type (`Amethyst Ring`).
    pub name: String,
    pub variant: String,
    pub req_level: Option<u32>,
    pub implicits: Vec<Implicit>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Implicit {
    pub lines: Vec<String>,
    pub values: Vec<[f64; 2]>,
}

/// Which part of a page a modifier is listed in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Section {
    /// The ordinary pool: what exalts, chaos, regals and the rest roll.
    Normal,
    /// The desecrated pool: rolled by desecration only.
    Desecrated,
}

/// Prefix or suffix: an explicit modifier's side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Prefix,
    Suffix,
}

impl Kind {
    pub fn other(self) -> Kind {
        match self {
            Kind::Prefix => Kind::Suffix,
            Kind::Suffix => Kind::Prefix,
        }
    }

    /// The side `ch_item` parsed, when it is one.
    pub fn from_item(kind: ch_item::ModKind) -> Option<Kind> {
        match kind {
            ch_item::ModKind::Prefix => Some(Kind::Prefix),
            ch_item::ModKind::Suffix => Some(Kind::Suffix),
            _ => None,
        }
    }
}

/// One modifier of the class.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataMod {
    /// Unique within the class file (`prefix-to-maximum-life`).
    pub id: String,
    pub section: Section,
    pub kind: Kind,
    /// The game's modifier families: one modifier per family on an item.
    pub family: Vec<String>,
    /// Each line with its values replaced by `#`; a leading `+` kept.
    pub lines: Vec<String>,
    /// Tiers per variant, best (tier 1) first.
    pub tiers: BTreeMap<String, Vec<Tier>>,
}

impl DataMod {
    /// This modifier's tiers on `variant`; empty when it has none there.
    pub fn tiers_on(&self, variant: &str) -> &[Tier] {
        self.tiers.get(variant).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn shares_family(&self, families: &[String]) -> bool {
        self.family.iter().any(|f| families.contains(f))
    }
}

/// One tier of a modifier on one variant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tier {
    /// 1 is the best.
    pub tier: u32,
    /// The affix name the item text shows in the header.
    pub name: String,
    /// The modifier level: the lowest item level that can roll it.
    pub ilvl: u32,
    /// Spawn weight where the page's basis is `spawn`; poe2db writes 1 as a
    /// placeholder and some tiers have none.
    pub weight: Option<u32>,
    /// One `[min, max]` per `#` across the modifier's lines, in order.
    pub values: Vec<[f64; 2]>,
}

/// An essence, perfect essence, alloy or liquid emotion: an item that adds
/// one known modifier.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Essence {
    pub name: String,
    pub slug: String,
    /// `essence` or `perfect_essence`; absent for liquid emotions.
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub perfect: Option<bool>,
    #[serde(default)]
    pub alloy: Option<bool>,
    /// Removes a random modifier before adding its own.
    #[serde(default)]
    pub removes: Option<bool>,
    pub kind: Kind,
    pub ilvl: u32,
    pub family: Vec<String>,
    pub lines: Vec<String>,
    pub values: Vec<[f64; 2]>,
    pub variants: Vec<String>,
    /// Per variant, the data modifier and tier the added modifier equals, or
    /// null when it equals none of the class's modifiers.
    pub matches: BTreeMap<String, Option<EssenceMatch>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EssenceMatch {
    #[serde(rename = "mod")]
    pub mod_id: String,
    pub tier: u32,
}
