//! The item as the engine sees it: the refusals checked up front, the
//! targets with their floors, and each explicit modifier joined to the data
//! and marked wanted, below its floor, unwanted or unrevealed.

use std::fmt;

use ch_blueprint::{Condition, Rarity as BpRarity, Side};
use ch_data::{AffixLimits, ClassData, Join, Kind, Section, VariantUnknown};
use ch_item::{Form, Item, Rarity};
use serde::{Deserialize, Serialize};

use crate::slots::Slots;

/// A modifier the player wants: a data modifier id and the worst tier that
/// still counts (1 is the best). No floor means the best tier the item's
/// level reaches.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Target {
    pub mod_id: String,
    #[serde(default)]
    pub floor: Option<u32>,
}

impl Target {
    pub fn new(mod_id: impl Into<String>, floor: Option<u32>) -> Target {
        Target {
            mod_id: mod_id.into(),
            floor,
        }
    }
}

/// A target checked against the item, with its floor applied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TargetInfo {
    pub mod_id: String,
    /// The floor in force: the one asked for, or the best reachable tier.
    pub floor: u32,
    /// The best tier the item's level reaches on its variant.
    pub best_tier: u32,
    /// `desecrated`: only desecration can place it.
    pub section: Section,
    pub side: Kind,
    pub family: Vec<String>,
    pub lines: Vec<String>,
}

/// Why the engine will not plan for an item at all.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "refusal", rename_all = "kebab-case")]
pub enum Refusal {
    /// Not one of the 22 equipment classes.
    ClassOutOfScope {
        class: String,
    },
    /// No base type, or one the data does not have.
    VariantUnknown {
        why: VariantUnknown,
    },
    Corrupted,
    Mirrored,
    Unidentified,
    /// Modifier lines without headers: no tiers to read.
    ShortForm,
    /// Not Normal, Magic or Rare.
    Rarity {
        rarity: String,
    },
    NoItemLevel,
    NoTargets,
    /// A target the item cannot roll, or cannot roll at its floor.
    TargetNotValid {
        mod_id: String,
        why: String,
    },
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Refusal::ClassOutOfScope { class } => {
                write!(f, "{class} is not a class the helper crafts")
            }
            Refusal::VariantUnknown { why } => write!(f, "the item's base is unknown: {why:?}"),
            Refusal::Corrupted => f.write_str("the item is corrupted"),
            Refusal::Mirrored => f.write_str("the item is mirrored"),
            Refusal::Unidentified => f.write_str("the item is unidentified"),
            Refusal::ShortForm => f.write_str("the item text has no modifier headers, so no tiers"),
            Refusal::Rarity { rarity } => write!(f, "a {rarity} item cannot be crafted"),
            Refusal::NoItemLevel => f.write_str("the item text gives no item level"),
            Refusal::NoTargets => f.write_str("no target modifier was given"),
            Refusal::TargetNotValid { mod_id, why } => write!(f, "target `{mod_id}`: {why}"),
        }
    }
}

/// The blueprint's word for an item's rarity, for Normal, Magic and Rare.
pub(crate) fn bp_rarity(rarity: &Rarity) -> Option<BpRarity> {
    match rarity {
        Rarity::Normal => Some(BpRarity::Normal),
        Rarity::Magic => Some(BpRarity::Magic),
        Rarity::Rare => Some(BpRarity::Rare),
        _ => None,
    }
}

pub(crate) fn item_rarity(rarity: BpRarity) -> Rarity {
    match rarity {
        BpRarity::Normal => Rarity::Normal,
        BpRarity::Magic => Rarity::Magic,
        BpRarity::Rare => Rarity::Rare,
    }
}

pub(crate) fn rarity_name(rarity: &Rarity) -> String {
    match rarity {
        Rarity::Other(text) if text.is_empty() => "rarity-less".to_string(),
        Rarity::Other(text) => text.clone(),
        other => format!("{other:?}"),
    }
}

pub(crate) fn kind_of(side: Side) -> Kind {
    match side {
        Side::Prefix => Kind::Prefix,
        Side::Suffix => Kind::Suffix,
    }
}

pub(crate) fn side_of(kind: Kind) -> Side {
    match kind {
        Kind::Prefix => Side::Prefix,
        Kind::Suffix => Side::Suffix,
    }
}

pub(crate) fn side_word(kind: Kind) -> &'static str {
    match kind {
        Kind::Prefix => "prefix",
        Kind::Suffix => "suffix",
    }
}

/// An item that passed the refusals, its class data and its targets.
pub(crate) struct Ctx<'a> {
    pub item: &'a Item,
    pub data: &'static ClassData,
    pub variant: &'static str,
    pub item_level: u32,
    pub targets: Vec<TargetInfo>,
    pub blocked: &'a [String],
}

impl<'a> Ctx<'a> {
    /// Checks the refusals, in the order: class, variant, corrupted,
    /// mirrored, unidentified, short form, rarity, item level, targets.
    pub fn new(
        item: &'a Item,
        targets: &[Target],
        blocked: &'a [String],
    ) -> Result<Ctx<'a>, Refusal> {
        let (data, variant) = ch_data::variant_of(item).map_err(|why| match why {
            VariantUnknown::ClassOutOfScope { class } => Refusal::ClassOutOfScope { class },
            why => Refusal::VariantUnknown { why },
        })?;
        if item.corrupted {
            return Err(Refusal::Corrupted);
        }
        if item.mirrored {
            return Err(Refusal::Mirrored);
        }
        if item.unidentified {
            return Err(Refusal::Unidentified);
        }
        if item.form == Form::Short {
            return Err(Refusal::ShortForm);
        }
        if bp_rarity(&item.rarity).is_none() {
            return Err(Refusal::Rarity {
                rarity: rarity_name(&item.rarity),
            });
        }
        let item_level = item
            .item_level
            .as_ref()
            .map(|l| l.value)
            .ok_or(Refusal::NoItemLevel)?;
        if targets.is_empty() {
            return Err(Refusal::NoTargets);
        }

        let mut infos: Vec<TargetInfo> = Vec::new();
        for t in targets {
            let invalid = |why: String| Refusal::TargetNotValid {
                mod_id: t.mod_id.clone(),
                why,
            };
            let Some(m) = data.find_mod(&t.mod_id) else {
                return Err(invalid(format!("{} has no such modifier", data.class)));
            };
            let best = m
                .tiers_on(variant)
                .iter()
                .filter(|tier| tier.ilvl <= item_level)
                .map(|tier| tier.tier)
                .min();
            let Some(best) = best else {
                return Err(invalid(format!(
                    "no tier of it on {} `{variant}` at item level {item_level}",
                    data.class
                )));
            };
            let floor = t.floor.unwrap_or(best);
            if floor < best {
                return Err(invalid(format!(
                    "floor T{floor} is better than the best tier item level {item_level} reaches (T{best})"
                )));
            }
            if let Some(other) = infos
                .iter()
                .find(|o| o.mod_id == m.id || m.shares_family(&o.family))
            {
                return Err(invalid(format!(
                    "it shares a modifier family with the target `{}`",
                    other.mod_id
                )));
            }
            infos.push(TargetInfo {
                mod_id: m.id.clone(),
                floor,
                best_tier: best,
                section: m.section,
                side: m.kind,
                family: m.family.clone(),
                lines: m.lines.clone(),
            });
        }
        Ok(Ctx {
            item,
            data,
            variant,
            item_level,
            targets: infos,
            blocked,
        })
    }

    pub fn is_blocked(&self, id: &str) -> bool {
        self.blocked.iter().any(|b| b == id)
    }

    pub fn target_index(&self, mod_id: &str) -> Option<usize> {
        self.targets.iter().position(|t| t.mod_id == mod_id)
    }
}

/// What one explicit modifier is to the craft.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Role {
    /// The target at this position, at its floor or better.
    Wanted(usize),
    /// The target at this position, below its floor: counts as unwanted.
    BelowFloor(usize),
    /// Anything else revealed, joined or not.
    Unwanted,
    /// Not yet revealed.
    Unrevealed,
}

/// One explicit (prefix or suffix) modifier of the item.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ModInfo {
    /// Its position in `Item::modifiers`.
    pub index: usize,
    pub side: Kind,
    pub fractured: bool,
    pub desecrated: bool,
    pub unrevealed: bool,
    /// The data modifier it joined to.
    pub mod_id: Option<String>,
    pub tier: Option<u32>,
    pub family: Vec<String>,
    /// The lines as shown, and with their values replaced by `#`.
    pub texts: Vec<String>,
    pub templates: Vec<String>,
    pub role: Role,
    /// Its modifier level (the joined tier's item level); `None` when it
    /// did not join, is unrevealed, or was placed by the projection.
    pub level: Option<u32>,
    /// A projected filler: a non-target of unknown level that only takes
    /// a slot (docs/models/crafting-slots-spec.md, "Projection").
    pub filler: bool,
    /// An unrevealed modifier: the target its desecration was bound to,
    /// when the route knows it.
    pub waits_for: Option<usize>,
}

impl ModInfo {
    /// The same modifier on both items: side, flags and lines as shown.
    pub fn same(&self, other: &ModInfo) -> bool {
        self.side == other.side
            && self.fractured == other.fractured
            && self.desecrated == other.desecrated
            && self.unrevealed == other.unrevealed
            && self.texts == other.texts
    }

    pub fn is_unwanted(&self) -> bool {
        matches!(self.role, Role::Unwanted | Role::BelowFloor(_))
    }

    pub fn shown(&self) -> String {
        self.texts.join(" / ")
    }
}

/// The joined item, or the item as a state-ordered route's projection
/// would leave it.
#[derive(Clone)]
pub(crate) struct State {
    pub join: Join,
    pub mods: Vec<ModInfo>,
    pub limits: AffixLimits,
    pub rarity: Option<BpRarity>,
    /// Changed by the projection: `join` no longer describes `mods`.
    pub projected: bool,
}

impl State {
    pub fn new(item: &Item, targets: &[TargetInfo]) -> State {
        let join = ch_data::join(item);
        let mods = join
            .mods
            .iter()
            .map(|jm| {
                let m = &item.modifiers[jm.index];
                let matched = jm.matched();
                let role = if jm.unrevealed {
                    Role::Unrevealed
                } else if let Some(found) = matched {
                    match targets.iter().position(|t| t.mod_id == found.mod_id) {
                        Some(t) if found.tier <= targets[t].floor => Role::Wanted(t),
                        Some(t) => Role::BelowFloor(t),
                        None => Role::Unwanted,
                    }
                } else {
                    Role::Unwanted
                };
                ModInfo {
                    index: jm.index,
                    side: jm.kind,
                    fractured: jm.fractured,
                    desecrated: jm.desecrated,
                    unrevealed: jm.unrevealed,
                    mod_id: matched.map(|f| f.mod_id.clone()),
                    tier: matched.map(|f| f.tier),
                    family: matched.map(|f| f.family.clone()).unwrap_or_default(),
                    texts: m.lines.iter().map(|l| l.text.clone()).collect(),
                    templates: m.lines.iter().map(|l| l.template.clone()).collect(),
                    role,
                    level: matched.map(|f| f.tier_ilvl),
                    filler: false,
                    waits_for: None,
                }
            })
            .collect();
        let limits = ch_data::item_affix_limits(item, &item.rarity).unwrap_or(AffixLimits {
            prefixes: 0,
            suffixes: 0,
        });
        State {
            join,
            mods,
            limits,
            rarity: bp_rarity(&item.rarity),
            projected: false,
        }
    }

    /// The target at position `t` is on the item at its floor.
    pub fn present(&self, t: usize) -> bool {
        self.mods.iter().any(|m| m.role == Role::Wanted(t))
    }

    pub fn wanted_mod(&self, t: usize) -> Option<&ModInfo> {
        self.mods.iter().find(|m| m.role == Role::Wanted(t))
    }

    /// The targets not on the item at their floor, in target order.
    pub fn missing(&self, count: usize) -> Vec<usize> {
        (0..count).filter(|t| !self.present(*t)).collect()
    }

    pub fn count(&self, side: Kind) -> usize {
        self.mods.iter().filter(|m| m.side == side).count()
    }

    pub fn free(&self, side: Kind) -> usize {
        self.limits.on(side).saturating_sub(self.count(side))
    }

    /// The `Item::modifiers` positions of the wanted modifiers.
    pub fn wanted_positions(&self) -> Vec<usize> {
        self.mods
            .iter()
            .filter(|m| matches!(m.role, Role::Wanted(_)))
            .map(|m| m.index)
            .collect()
    }

    pub fn has_unrevealed(&self) -> bool {
        self.mods.iter().any(|m| m.unrevealed)
    }

    pub fn has_desecrated(&self) -> bool {
        self.mods.iter().any(|m| m.desecrated)
    }
}

/// Every part of `cond` the item fails, in words; empty when it holds.
/// `essence_binds` is asked only when the condition names it.
pub(crate) fn condition_failures(
    cond: &Condition,
    st: &State,
    targets: &[TargetInfo],
    essence_binds: &dyn Fn() -> bool,
) -> Vec<String> {
    let mut failed = Vec::new();
    let wanted = |side: Option<Kind>| {
        st.mods
            .iter()
            .filter(|m| matches!(m.role, Role::Wanted(_)) && side.is_none_or(|s| m.side == s))
            .count() as u32
    };
    let at_least = |failed: &mut Vec<String>, have: u32, need: Option<u32>, what: &str| {
        if let Some(need) = need {
            if have < need {
                failed.push(format!("{have} {what}, at least {need} needed"));
            }
        }
    };
    let num = |v: &Option<ch_blueprint::Spanned<u32>>| v.as_ref().map(|n| *n.get_ref());
    let flag = |v: &Option<ch_blueprint::Spanned<bool>>| v.as_ref().map(|b| *b.get_ref());

    if let Some(list) = &cond.rarity {
        let list = list.get_ref();
        if !st.rarity.is_some_and(|r| list.contains(&r)) {
            let names: Vec<&str> = list.iter().map(|r| r.name()).collect();
            let is = st.rarity.map_or("of another rarity", |r| r.name());
            failed.push(format!("the item is {is}, not {}", names.join(" or ")));
        }
    }
    at_least(
        &mut failed,
        wanted(None),
        num(&cond.wanted_at_least),
        "wanted modifiers",
    );
    at_least(
        &mut failed,
        wanted(Some(Kind::Prefix)),
        num(&cond.wanted_prefixes_at_least),
        "wanted prefixes",
    );
    at_least(
        &mut failed,
        wanted(Some(Kind::Suffix)),
        num(&cond.wanted_suffixes_at_least),
        "wanted suffixes",
    );
    match flag(&cond.fractured_wanted) {
        Some(true) => {
            let fractured: Vec<&ModInfo> = st.mods.iter().filter(|m| m.fractured).collect();
            if fractured.is_empty() {
                failed.push("no fractured modifier".to_string());
            } else if fractured.iter().any(|m| !matches!(m.role, Role::Wanted(_))) {
                failed.push("a fractured modifier is not a wanted one".to_string());
            }
        }
        Some(false) if st.mods.iter().any(|m| m.fractured) => {
            failed.push("a fractured modifier is present".to_string());
        }
        _ => {}
    }
    if let Some(most) = num(&cond.unwanted_at_most) {
        let have = st.mods.iter().filter(|m| m.is_unwanted()).count() as u32;
        if have > most {
            failed.push(format!("{have} unwanted modifiers, at most {most} allowed"));
        }
    }
    if let Some(want) = flag(&cond.essence_binds) {
        let binds = essence_binds();
        if binds != want {
            failed.push(if want {
                "no unblocked essence adds a remaining target at its floor".to_string()
            } else {
                "an essence adds a remaining target at its floor".to_string()
            });
        }
    }
    at_least(
        &mut failed,
        st.free(Kind::Prefix) as u32,
        num(&cond.free_prefixes_at_least),
        "free prefix slots",
    );
    at_least(
        &mut failed,
        st.free(Kind::Suffix) as u32,
        num(&cond.free_suffixes_at_least),
        "free suffix slots",
    );
    if flag(&cond.no_desecrated) == Some(true) && st.has_desecrated() {
        failed.push("a desecrated modifier is present".to_string());
    }
    match flag(&cond.unrevealed) {
        Some(true) if !st.has_unrevealed() => failed.push("no unrevealed modifier".to_string()),
        Some(false) if st.has_unrevealed() => {
            failed.push("an unrevealed modifier is present".to_string())
        }
        _ => {}
    }
    let remaining = st.missing(targets.len()).len() as u32;
    if let Some(most) = num(&cond.wanted_remaining_at_most) {
        if remaining > most {
            failed.push(format!(
                "{remaining} targets remain, at most {most} allowed"
            ));
        }
    }
    if let Some(exactly) = num(&cond.wanted_remaining_exactly) {
        if remaining != exactly {
            failed.push(format!("{remaining} targets remain, not {exactly}"));
        }
    }

    // The slot conditions (docs/models/crafting-slots-spec.md,
    // "Conditions"); `false` asks for the opposite.
    let slots = Slots::new(st, targets);
    let open = slots.open_count();
    let one_open = format!("{open} sides are open, not one");
    let checks: [(&Option<ch_blueprint::Spanned<bool>>, bool, &str, &str); 4] = [
        (
            &cond.blocked_side,
            slots.any(|s| s.blocked),
            "no side is blocked",
            "a side is blocked",
        ),
        (
            &cond.closable_side,
            slots.any(|s| s.closable),
            "no side is closable",
            "a side is closable",
        ),
        (
            &cond.one_side_open,
            open == 1,
            &one_open,
            "exactly one side is open",
        ),
        (
            &cond.desecrated_unwanted,
            st.mods
                .iter()
                .any(|m| m.desecrated && !m.unrevealed && !matches!(m.role, Role::Wanted(_))),
            "no revealed desecrated modifier is unwanted",
            "a revealed desecrated modifier is not a target",
        ),
    ];
    for (key, holds, when_wanted, when_not) in checks {
        match flag(key) {
            Some(true) if !holds => failed.push(when_wanted.to_string()),
            Some(false) if holds => failed.push(when_not.to_string()),
            _ => {}
        }
    }
    failed
}
