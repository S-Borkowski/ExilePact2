//! The static catalogue: every currency, omen, bone, flux and catalyst a
//! blueprint or the blocked list may name, with its id and what it does.
//!
//! Ids are the English name in lower kebab case with apostrophes dropped
//! ([`id_from_name`]): `Orb of Annulment` is `orb-of-annulment`,
//! `Hinekora's Lock` is `hinekoras-lock`.
//!
//! Essences (alloys included) and liquid emotions are data-backed: their
//! ids come from the data's `name`, and `ch-engine` resolves them against
//! the modifier data. This crate only recognises their shape
//! ([`data_backed`]) so it can tell one from a typo and check the omens
//! used with it.
//!
//! Everything here is from docs/crafting-helper/crafting-mechanics.md, which
//! read poe2db on 2026-10-28; what that doc marks not established is said
//! beside the entry.

use serde::Serialize;

use crate::model::{Goal, Side};

/// What kind of crafting item an id names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    Currency,
    Omen,
    Bone,
    Flux,
    Catalyst,
    /// Data-backed.
    Essence,
    /// Data-backed.
    Alloy,
    /// Data-backed.
    Liquid,
}

/// What using a crafting item does to an item, in the terms the engine
/// checks an outcome against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Effect {
    /// Normal to Magic with one modifier.
    Transmute,
    /// One modifier added to a Magic item.
    Augment,
    /// Magic to Rare plus one modifier (Regal Orb; Lesser, plain and
    /// Greater essences, whose modifier is chosen).
    Upgrade,
    /// Normal or Magic to Rare with four modifiers.
    Alchemy,
    /// One modifier added to a Rare item (two with Omen of Greater
    /// Exaltation).
    Add,
    /// One modifier removed and one added (Chaos Orb; Perfect and special
    /// essences, alloys and liquid emotions, whose added modifier is
    /// chosen).
    Swap,
    /// One modifier removed (two with Omen of Greater Annulment).
    Remove,
    /// Matching modifiers turned into others (the resistance fluxes).
    Convert,
    /// An unrevealed desecrated modifier added; one removed first when the
    /// item is full.
    Desecrate,
    /// The unrevealed modifier revealed, from offered options.
    Reveal,
    /// The values rerolled, the modifiers kept (Divine Orb).
    RerollValues,
    /// One modifier fractured (Fracturing Orb).
    Fracture,
    /// Nothing the engine checks: the player does it and confirms (quality,
    /// catalysts, sockets, identifying, previews and the rest).
    Manual,
    /// An omen: it changes the next use of another currency.
    Omen,
}

impl Effect {
    /// Whether a step with `goal` can be taken with something of this
    /// effect: a removal-only currency cannot add a wanted modifier, and a
    /// manual one cannot be checked by any goal but `manual`.
    pub fn serves(self, goal: Goal) -> bool {
        use Effect::*;
        match goal {
            Goal::AddWanted | Goal::Fill => {
                matches!(self, Transmute | Augment | Upgrade | Alchemy | Add | Swap)
            }
            Goal::RemoveUnwanted => matches!(self, Remove | Swap),
            Goal::Desecrate => self == Desecrate,
            Goal::Reveal => self == Reveal,
            Goal::RemoveDesecrated => self == Remove,
            Goal::Convert => self == Convert,
            Goal::Manual => self != Omen,
        }
    }
}

/// The item groups a bone desecrates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ItemGroup {
    /// Jawbones.
    WeaponsAndQuivers,
    /// Ribs.
    Armour,
    /// Collarbones: amulets, rings and belts.
    Jewellery,
    /// The Cranium.
    Jewels,
}

impl ItemGroup {
    /// The data classes in the group. Which group Sceptres, Foci and
    /// Bucklers fall in is a reading of the class names, not measured.
    pub fn classes(self) -> &'static [&'static str] {
        match self {
            Self::WeaponsAndQuivers => &[
                "Wands",
                "One Hand Maces",
                "Sceptres",
                "Spears",
                "Bows",
                "Staves",
                "Two Hand Maces",
                "Quarterstaves",
                "Crossbows",
                "Talismans",
                "Quivers",
            ],
            Self::Armour => &[
                "Gloves",
                "Boots",
                "Helmets",
                "Body Armours",
                "Shields",
                "Bucklers",
                "Foci",
            ],
            Self::Jewellery => &["Amulets", "Rings", "Belts"],
            Self::Jewels => &["Jewels"],
        }
    }

    /// The group a data class belongs to.
    pub fn of_class(class: &str) -> Option<Self> {
        [Self::WeaponsAndQuivers, Self::Armour, Self::Jewellery, Self::Jewels]
            .into_iter()
            .find(|g| g.classes().contains(&class))
    }
}

/// Every data `class` string (apps/crafting-helper/data/index.json, M1).
pub const CLASSES: &[&str] = &[
    "Gloves",
    "Boots",
    "Helmets",
    "Body Armours",
    "Shields",
    "Quivers",
    "Bucklers",
    "Foci",
    "Wands",
    "One Hand Maces",
    "Sceptres",
    "Spears",
    "Bows",
    "Staves",
    "Two Hand Maces",
    "Quarterstaves",
    "Crossbows",
    "Talismans",
    "Belts",
    "Amulets",
    "Rings",
    "Jewels",
];

/// What an omen's next use is of.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum OmenFits {
    /// One of these currency ids.
    Currencies(&'static [&'static str]),
    /// A Perfect or special ("corrupted") essence, or an alloy (the owner's
    /// posts use Crystallisation with alloys; not established).
    SwapEssences,
    /// A bone, or the `desecrate` placeholder.
    Desecration,
    /// The reveal at the Well of Souls (`reveal`).
    Reveal,
    /// Nothing known: the omen page gives no effect text.
    Nothing,
}

/// How an omen changes the use it modifies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum OmenChange {
    /// Restricts the added or removed modifier to the omen's side.
    Side,
    /// Alchemy: as many modifiers of the omen's side as the item holds.
    FillSide,
    /// Exaltation: two modifiers added instead of one.
    AddTwo,
    /// Annulment: two modifiers removed instead of one.
    RemoveTwo,
    /// Whittling: the lowest-level modifier is the one removed.
    RemoveLowestLevel,
    /// Light: only the desecrated modifier is removed.
    DesecratedOnly,
    /// Homogenising: the added modifier shares a type with one present.
    SameType,
    /// Catalysing: the catalyst quality is spent to favour its kind.
    Catalysing,
    /// Abyssal Echoes: the offered options may be rerolled once.
    RerollOptions,
    /// Sovereign, Liege, Blackblooded: a named lord's modifier.
    Lord,
    /// Blessed: only implicit values rerolled.
    ImplicitsOnly,
    /// Sanctification: what it does is not established.
    Sanctify,
    /// Chance: the item is not destroyed.
    NoDestroy,
    /// No effect text.
    Unknown,
}

/// The omen part of a catalogue entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct OmenSpec {
    pub fits: OmenFits,
    /// `Some` for a Sinistral (prefix) or Dextral (suffix) omen.
    pub side: Option<Side>,
    pub change: OmenChange,
}

/// One static catalogue entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct Entry {
    pub id: &'static str,
    /// The English name the id is made from.
    pub name: &'static str,
    pub kind: Kind,
    pub effect: Effect,
    /// Every modifier it adds is at least this level (Greater and Perfect
    /// currency, Ancient bones).
    pub min_modifier_level: Option<u32>,
    /// The item groups it applies to; empty for any.
    pub groups: &'static [ItemGroup],
    /// The highest item level it may be used on (Gnawed bones: 64).
    pub max_item_level: Option<u32>,
    /// `Some` for an omen.
    pub omen: Option<OmenSpec>,
}

const fn currency(id: &'static str, name: &'static str, effect: Effect, mml: Option<u32>) -> Entry {
    Entry {
        id,
        name,
        kind: Kind::Currency,
        effect,
        min_modifier_level: mml,
        groups: &[],
        max_item_level: None,
        omen: None,
    }
}

const fn omen(
    id: &'static str,
    name: &'static str,
    fits: OmenFits,
    side: Option<Side>,
    change: OmenChange,
) -> Entry {
    Entry {
        id,
        name,
        kind: Kind::Omen,
        effect: Effect::Omen,
        min_modifier_level: None,
        groups: &[],
        max_item_level: None,
        omen: Some(OmenSpec { fits, side, change }),
    }
}

const fn bone(
    id: &'static str,
    name: &'static str,
    group: &'static [ItemGroup],
    mml: Option<u32>,
    max_ilvl: Option<u32>,
) -> Entry {
    Entry {
        id,
        name,
        kind: Kind::Bone,
        effect: Effect::Desecrate,
        min_modifier_level: mml,
        groups: group,
        max_item_level: max_ilvl,
        omen: None,
    }
}

const fn flux(id: &'static str, name: &'static str, effect: Effect) -> Entry {
    Entry {
        id,
        name,
        kind: Kind::Flux,
        effect,
        min_modifier_level: None,
        groups: &[],
        max_item_level: None,
        omen: None,
    }
}

const fn catalyst(id: &'static str, name: &'static str, group: &'static [ItemGroup]) -> Entry {
    Entry {
        id,
        name,
        kind: Kind::Catalyst,
        effect: Effect::Manual,
        min_modifier_level: None,
        groups: group,
        max_item_level: None,
        omen: None,
    }
}

const CHAOS: &[&str] = &["chaos-orb", "greater-chaos-orb", "perfect-chaos-orb"];
const REGAL: &[&str] = &["regal-orb", "greater-regal-orb", "perfect-regal-orb"];
const EXALT: &[&str] = &["exalted-orb", "greater-exalted-orb", "perfect-exalted-orb"];
const ANNUL: &[&str] = &["orb-of-annulment"];
const ALCHEMY: &[&str] = &["orb-of-alchemy"];
const DIVINE: &[&str] = &["divine-orb"];
const CHANCE: &[&str] = &["orb-of-chance"];

const WEAPONS: &[ItemGroup] = &[ItemGroup::WeaponsAndQuivers];
const ARMOUR: &[ItemGroup] = &[ItemGroup::Armour];
const JEWELLERY: &[ItemGroup] = &[ItemGroup::Jewellery];
const JEWELS: &[ItemGroup] = &[ItemGroup::Jewels];

const P: Option<Side> = Some(Side::Prefix);
const S: Option<Side> = Some(Side::Suffix);

use Effect as E;
use OmenChange as C;
use OmenFits as F;

/// The static catalogue. Shards are left out: they are not used on an
/// item. Catalyst names (`Refined …` for the jewel ones) follow the doc's
/// "a plain and a Refined version"; the exact in-game spelling is not
/// established.
pub const CATALOGUE: &[Entry] = &[
    // Currency (crafting-mechanics.md § "Currency").
    currency("orb-of-transmutation", "Orb of Transmutation", E::Transmute, None),
    currency("orb-of-augmentation", "Orb of Augmentation", E::Augment, None),
    currency("regal-orb", "Regal Orb", E::Upgrade, None),
    currency("orb-of-alchemy", "Orb of Alchemy", E::Alchemy, None),
    currency("exalted-orb", "Exalted Orb", E::Add, None),
    currency("chaos-orb", "Chaos Orb", E::Swap, None),
    currency("orb-of-annulment", "Orb of Annulment", E::Remove, None),
    currency("divine-orb", "Divine Orb", E::RerollValues, None),
    currency("fracturing-orb", "Fracturing Orb", E::Fracture, None),
    currency("orb-of-chance", "Orb of Chance", E::Manual, None),
    currency("hinekoras-lock", "Hinekora's Lock", E::Manual, None),
    currency("mirror-of-kalandra", "Mirror of Kalandra", E::Manual, None),
    currency("artificers-orb", "Artificer's Orb", E::Manual, None),
    currency("scroll-of-wisdom", "Scroll of Wisdom", E::Manual, None),
    currency("blacksmiths-whetstone", "Blacksmith's Whetstone", E::Manual, None),
    currency("arcanists-etcher", "Arcanist's Etcher", E::Manual, None),
    currency("armourers-scrap", "Armourer's Scrap", E::Manual, None),
    // Greater and Perfect currency, with their Minimum Modifier Level.
    currency("greater-orb-of-transmutation", "Greater Orb of Transmutation", E::Transmute, Some(44)),
    currency("perfect-orb-of-transmutation", "Perfect Orb of Transmutation", E::Transmute, Some(70)),
    currency("greater-orb-of-augmentation", "Greater Orb of Augmentation", E::Augment, Some(44)),
    currency("perfect-orb-of-augmentation", "Perfect Orb of Augmentation", E::Augment, Some(70)),
    currency("greater-regal-orb", "Greater Regal Orb", E::Upgrade, Some(35)),
    currency("perfect-regal-orb", "Perfect Regal Orb", E::Upgrade, Some(50)),
    currency("greater-exalted-orb", "Greater Exalted Orb", E::Add, Some(35)),
    currency("perfect-exalted-orb", "Perfect Exalted Orb", E::Add, Some(50)),
    currency("greater-chaos-orb", "Greater Chaos Orb", E::Swap, Some(35)),
    currency("perfect-chaos-orb", "Perfect Chaos Orb", E::Swap, Some(50)),
    // Omens (§ "Omens").
    omen("omen-of-whittling", "Omen of Whittling", F::Currencies(CHAOS), None, C::RemoveLowestLevel),
    omen("omen-of-sinistral-erasure", "Omen of Sinistral Erasure", F::Currencies(CHAOS), P, C::Side),
    omen("omen-of-dextral-erasure", "Omen of Dextral Erasure", F::Currencies(CHAOS), S, C::Side),
    omen("omen-of-sinistral-alchemy", "Omen of Sinistral Alchemy", F::Currencies(ALCHEMY), P, C::FillSide),
    omen("omen-of-dextral-alchemy", "Omen of Dextral Alchemy", F::Currencies(ALCHEMY), S, C::FillSide),
    omen("omen-of-sinistral-coronation", "Omen of Sinistral Coronation", F::Currencies(REGAL), P, C::Side),
    omen("omen-of-dextral-coronation", "Omen of Dextral Coronation", F::Currencies(REGAL), S, C::Side),
    omen("omen-of-homogenising-coronation", "Omen of Homogenising Coronation", F::Currencies(REGAL), None, C::SameType),
    omen("omen-of-greater-exaltation", "Omen of Greater Exaltation", F::Currencies(EXALT), None, C::AddTwo),
    omen("omen-of-sinistral-exaltation", "Omen of Sinistral Exaltation", F::Currencies(EXALT), P, C::Side),
    omen("omen-of-dextral-exaltation", "Omen of Dextral Exaltation", F::Currencies(EXALT), S, C::Side),
    omen("omen-of-homogenising-exaltation", "Omen of Homogenising Exaltation", F::Currencies(EXALT), None, C::SameType),
    omen("omen-of-catalysing-exaltation", "Omen of Catalysing Exaltation", F::Currencies(EXALT), None, C::Catalysing),
    omen("omen-of-greater-annulment", "Omen of Greater Annulment", F::Currencies(ANNUL), None, C::RemoveTwo),
    omen("omen-of-sinistral-annulment", "Omen of Sinistral Annulment", F::Currencies(ANNUL), P, C::Side),
    omen("omen-of-dextral-annulment", "Omen of Dextral Annulment", F::Currencies(ANNUL), S, C::Side),
    omen("omen-of-light", "Omen of Light", F::Currencies(ANNUL), None, C::DesecratedOnly),
    omen("omen-of-sinistral-crystallisation", "Omen of Sinistral Crystallisation", F::SwapEssences, P, C::Side),
    omen("omen-of-dextral-crystallisation", "Omen of Dextral Crystallisation", F::SwapEssences, S, C::Side),
    omen("omen-of-sinistral-necromancy", "Omen of Sinistral Necromancy", F::Desecration, P, C::Side),
    omen("omen-of-dextral-necromancy", "Omen of Dextral Necromancy", F::Desecration, S, C::Side),
    omen("omen-of-abyssal-echoes", "Omen of Abyssal Echoes", F::Reveal, None, C::RerollOptions),
    omen("omen-of-the-sovereign", "Omen of the Sovereign", F::Desecration, None, C::Lord),
    omen("omen-of-the-liege", "Omen of the Liege", F::Desecration, None, C::Lord),
    omen("omen-of-the-blackblooded", "Omen of the Blackblooded", F::Desecration, None, C::Lord),
    omen("omen-of-the-blessed", "Omen of the Blessed", F::Currencies(DIVINE), None, C::ImplicitsOnly),
    omen("omen-of-sanctification", "Omen of Sanctification", F::Currencies(DIVINE), None, C::Sanctify),
    omen("omen-of-chance", "Omen of Chance", F::Currencies(CHANCE), None, C::NoDestroy),
    omen("omen-of-recombination", "Omen of Recombination", F::Nothing, None, C::Unknown),
    // Bones (§ "Desecration"): Gnawed up to item level 64, Ancient with a
    // Minimum Modifier Level of 40.
    bone("gnawed-jawbone", "Gnawed Jawbone", WEAPONS, None, Some(64)),
    bone("preserved-jawbone", "Preserved Jawbone", WEAPONS, None, None),
    bone("ancient-jawbone", "Ancient Jawbone", WEAPONS, Some(40), None),
    bone("gnawed-rib", "Gnawed Rib", ARMOUR, None, Some(64)),
    bone("preserved-rib", "Preserved Rib", ARMOUR, None, None),
    bone("ancient-rib", "Ancient Rib", ARMOUR, Some(40), None),
    bone("gnawed-collarbone", "Gnawed Collarbone", JEWELLERY, None, Some(64)),
    bone("preserved-collarbone", "Preserved Collarbone", JEWELLERY, None, None),
    bone("ancient-collarbone", "Ancient Collarbone", JEWELLERY, Some(40), None),
    bone("altered-collarbone", "Altered Collarbone", JEWELLERY, None, None),
    bone("preserved-cranium", "Preserved Cranium", JEWELS, None, None),
    // Fluxes (§ "Fluxes"). How a flux maps tiers is not established.
    flux("blazing-flux", "Blazing Flux", E::Convert),
    flux("chilling-flux", "Chilling Flux", E::Convert),
    flux("crackling-flux", "Crackling Flux", E::Convert),
    flux("void-flux", "Void Flux", E::Convert),
    flux("perfect-flux", "Perfect Flux", E::Manual),
    // Catalysts (§ "Catalysts"): manual, the plain ones for rings and
    // amulets, the Refined ones for jewels.
    catalyst("flesh-catalyst", "Flesh Catalyst", JEWELLERY),
    catalyst("refined-flesh-catalyst", "Refined Flesh Catalyst", JEWELS),
    catalyst("neural-catalyst", "Neural Catalyst", JEWELLERY),
    catalyst("refined-neural-catalyst", "Refined Neural Catalyst", JEWELS),
    catalyst("carapace-catalyst", "Carapace Catalyst", JEWELLERY),
    catalyst("refined-carapace-catalyst", "Refined Carapace Catalyst", JEWELS),
    catalyst("uul-netols-catalyst", "Uul-Netol's Catalyst", JEWELLERY),
    catalyst("refined-uul-netols-catalyst", "Refined Uul-Netol's Catalyst", JEWELS),
    catalyst("xophs-catalyst", "Xoph's Catalyst", JEWELLERY),
    catalyst("refined-xophs-catalyst", "Refined Xoph's Catalyst", JEWELS),
    catalyst("tuls-catalyst", "Tul's Catalyst", JEWELLERY),
    catalyst("refined-tuls-catalyst", "Refined Tul's Catalyst", JEWELS),
    catalyst("eshs-catalyst", "Esh's Catalyst", JEWELLERY),
    catalyst("refined-eshs-catalyst", "Refined Esh's Catalyst", JEWELS),
    catalyst("chayulas-catalyst", "Chayula's Catalyst", JEWELLERY),
    catalyst("refined-chayulas-catalyst", "Refined Chayula's Catalyst", JEWELS),
    catalyst("reaver-catalyst", "Reaver Catalyst", JEWELLERY),
    catalyst("refined-reaver-catalyst", "Refined Reaver Catalyst", JEWELS),
    catalyst("sibilant-catalyst", "Sibilant Catalyst", JEWELLERY),
    catalyst("refined-sibilant-catalyst", "Refined Sibilant Catalyst", JEWELS),
    catalyst("skittering-catalyst", "Skittering Catalyst", JEWELLERY),
    catalyst("refined-skittering-catalyst", "Refined Skittering Catalyst", JEWELS),
    catalyst("adaptive-catalyst", "Adaptive Catalyst", JEWELLERY),
    catalyst("refined-adaptive-catalyst", "Refined Adaptive Catalyst", JEWELS),
    catalyst("necrotic-catalyst", "Necrotic Catalyst", JEWELLERY),
    catalyst("refined-necrotic-catalyst", "Refined Necrotic Catalyst", JEWELS),
];

/// The placeholder currency for "the bones that fit the item's class and
/// level", tried in the order Preserved, Ancient, Gnawed (Gnawed dropped
/// above item level 64).
pub const DESECRATE: &str = "desecrate";
/// The placeholder currency for the reveal at the Well of Souls. It names
/// no crafting item, so nothing blocks it.
pub const REVEAL: &str = "reveal";
/// The placeholder omen for the Sinistral or Dextral omen of the action's
/// currency for the bound target's side.
pub const SIDE_OMEN: &str = "side-omen";

/// The catalogue entry for an id.
pub fn entry(id: &str) -> Option<&'static Entry> {
    CATALOGUE.iter().find(|e| e.id == id)
}

/// The bones of a group, in the order the `desecrate` placeholder tries
/// them (Preserved, Ancient, Gnawed), Gnawed dropped above its item level.
pub fn bones_for(group: ItemGroup, item_level: u32) -> Vec<&'static Entry> {
    let mut bones: Vec<&'static Entry> = CATALOGUE
        .iter()
        .filter(|e| e.kind == Kind::Bone && e.groups.contains(&group))
        .filter(|e| e.id != "altered-collarbone")
        .filter(|e| e.max_item_level.is_none_or(|max| item_level <= max))
        .collect();
    let rank = |e: &Entry| match e.id.split('-').next() {
        Some("preserved") => 0,
        Some("ancient") => 1,
        _ => 2,
    };
    bones.sort_by_key(|e| rank(e));
    bones
}

/// The grade of a data-backed essence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EssenceTier {
    Lesser,
    Plain,
    Greater,
    Perfect,
    /// The single-tier essences (Hysteria, Delirium, Horror, Insanity, the
    /// Abyss, the Breach), which swap like a Perfect one.
    Special,
}

/// What the shape of a data-backed id says, before the data is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct DataBacked {
    pub kind: Kind,
    /// `Some` for an essence.
    pub tier: Option<EssenceTier>,
    pub effect: Effect,
}

impl DataBacked {
    /// Whether the Crystallisation omens fit it.
    pub fn takes_crystallisation(self) -> bool {
        self.kind == Kind::Alloy
            || matches!(self.tier, Some(EssenceTier::Perfect | EssenceTier::Special))
    }
}

/// The special essences' families, as they appear after `essence-of-`.
const SPECIAL_ESSENCES: &[&str] = &["hysteria", "delirium", "horror", "insanity", "the-abyss", "the-breach"];

/// Recognises an essence, alloy or liquid emotion id by its shape. Whether
/// one with that name exists for a class is `ch-engine`'s check.
pub fn data_backed(id: &str) -> Option<DataBacked> {
    if !is_kebab(id) {
        return None;
    }
    let essence = |tier, effect| {
        Some(DataBacked {
            kind: Kind::Essence,
            tier: Some(tier),
            effect,
        })
    };
    if let Some(family) = id.strip_prefix("essence-of-") {
        return if SPECIAL_ESSENCES.contains(&family) {
            essence(EssenceTier::Special, Effect::Swap)
        } else {
            essence(EssenceTier::Plain, Effect::Upgrade)
        };
    }
    if id.starts_with("lesser-essence-of-") {
        return essence(EssenceTier::Lesser, Effect::Upgrade);
    }
    if id.starts_with("greater-essence-of-") {
        return essence(EssenceTier::Greater, Effect::Upgrade);
    }
    if id.starts_with("perfect-essence-of-") {
        return essence(EssenceTier::Perfect, Effect::Swap);
    }
    if id.ends_with("-alloy") {
        return Some(DataBacked {
            kind: Kind::Alloy,
            tier: None,
            effect: Effect::Swap,
        });
    }
    if id.starts_with("liquid-") || id.contains("-liquid-") {
        return Some(DataBacked {
            kind: Kind::Liquid,
            tier: None,
            effect: Effect::Swap,
        });
    }
    None
}

/// What a currency id an action names is, as far as this crate can tell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Currency {
    /// A static catalogue entry that is not an omen.
    Static(&'static Entry),
    /// An essence, alloy or liquid id by its shape.
    DataBacked(DataBacked),
    /// `desecrate`.
    Desecrate,
    /// `reveal`.
    Reveal,
}

impl Currency {
    /// Reads an action's currency id; `None` for an unknown id or an omen.
    pub fn of(id: &str) -> Option<Self> {
        match id {
            DESECRATE => Some(Self::Desecrate),
            REVEAL => Some(Self::Reveal),
            _ => match entry(id) {
                Some(e) if e.kind == Kind::Omen => None,
                Some(e) => Some(Self::Static(e)),
                None => data_backed(id).map(Self::DataBacked),
            },
        }
    }

    /// What using it does.
    pub fn effect(self) -> Effect {
        match self {
            Self::Static(e) => e.effect,
            Self::DataBacked(d) => d.effect,
            Self::Desecrate => Effect::Desecrate,
            Self::Reveal => Effect::Reveal,
        }
    }

    /// Whether `omen`'s next use can be this currency.
    pub fn takes(self, omen: &OmenSpec) -> bool {
        match (omen.fits, self) {
            (OmenFits::Currencies(ids), Self::Static(e)) => ids.contains(&e.id),
            (OmenFits::SwapEssences, Self::DataBacked(d)) => d.takes_crystallisation(),
            (OmenFits::Desecration, Self::Desecrate) => true,
            (OmenFits::Desecration, Self::Static(e)) => e.kind == Kind::Bone,
            (OmenFits::Reveal, Self::Reveal) => true,
            _ => false,
        }
    }

    /// The Sinistral (prefix) or Dextral (suffix) omen of this currency, if
    /// it has one: what `side-omen` resolves to.
    pub fn side_omen(self, side: Side) -> Option<&'static Entry> {
        CATALOGUE.iter().find(|e| {
            e.omen
                .as_ref()
                .is_some_and(|o| o.side == Some(side) && self.takes(o))
        })
    }
}

/// The id of an English name: lower kebab case, apostrophes dropped.
pub fn id_from_name(name: &str) -> String {
    let mut id = String::new();
    for c in name.chars() {
        if c == '\'' || c == '\u{2019}' {
            continue;
        }
        if c.is_alphanumeric() {
            id.extend(c.to_lowercase());
        } else if !id.is_empty() && !id.ends_with('-') {
            id.push('-');
        }
    }
    while id.ends_with('-') {
        id.pop();
    }
    id
}

/// Whether `id` is lower kebab case: `a-z`, `0-9` and single inner dashes.
pub fn is_kebab(id: &str) -> bool {
    !id.is_empty()
        && id
            .split('-')
            .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()))
}
