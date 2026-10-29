//! Slot accounting (docs/models/crafting-slots-spec.md, "Slot accounting"):
//! per side, its cap, the modifiers on it, the free slots, the targets it
//! still needs, and whether it is blocked, reserved, closable or open. The
//! conditions, the side rules, the placeholders and the projection all read
//! these numbers, and the stdlib model computes the same ones.
//!
//! Also here: which modifiers a removal may aim at ([`removable`]), which it
//! may actually take ([`at_risk`]), and the one a whittle is certain to take
//! ([`whittle_certain`]).

use ch_blueprint::Rarity as BpRarity;
use ch_data::Kind;

use crate::context::{ModInfo, Role, State, TargetInfo};

/// Both sides, prefix first: the order every tie that is not settled by
/// target order falls to.
pub(crate) const SIDES: [Kind; 2] = [Kind::Prefix, Kind::Suffix];

/// One side's numbers on the item as it stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SideSlots {
    pub side: Kind,
    /// The side's limit for the item's rarity.
    pub cap: usize,
    /// Its explicit modifiers: fractured, desecrated and unrevealed ones
    /// included.
    pub mods: usize,
    pub free: usize,
    /// Its targets not on the item at their floor, less those an
    /// unrevealed modifier on it waits for; on a Magic item at most the cap
    /// less the targets already there.
    pub need: usize,
    /// `free - need`.
    pub spare: isize,
    /// Short of slots, with a modifier a removal could take.
    pub blocked: bool,
    /// Every free slot is needed.
    pub reserved: bool,
    /// Free slots, nothing missing, and the other side needs something.
    pub closable: bool,
    pub open: bool,
}

/// Both sides' numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Slots {
    pub prefix: SideSlots,
    pub suffix: SideSlots,
}

impl Slots {
    pub fn new(st: &State, targets: &[TargetInfo]) -> Slots {
        let missing = st.missing(targets.len());
        let raw = |side: Kind| {
            let on: Vec<&ModInfo> = st.mods.iter().filter(|m| m.side == side).collect();
            let cap = st.limits.on(side);
            let free = cap.saturating_sub(on.len());
            let side_missing: Vec<usize> = missing
                .iter()
                .copied()
                .filter(|&t| targets[t].side == side)
                .collect();
            let waiting = on
                .iter()
                .filter(|m| m.unrevealed && m.waits_for.is_some_and(|t| side_missing.contains(&t)))
                .count();
            let mut need = side_missing.len().saturating_sub(waiting);
            if st.rarity == Some(BpRarity::Magic) {
                let placed = on
                    .iter()
                    .filter(|m| matches!(m.role, Role::Wanted(_)))
                    .count();
                need = need.min(cap.saturating_sub(placed));
            }
            (cap, on.len(), free, need)
        };
        let numbers = SIDES.map(raw);
        let side = |i: usize| {
            let (cap, mods, free, need) = numbers[i];
            let other_need = numbers[1 - i].3;
            let kind = SIDES[i];
            let spare = free as isize - need as isize;
            SideSlots {
                side: kind,
                cap,
                mods,
                free,
                need,
                spare,
                blocked: spare < 0 && st.mods.iter().any(|m| m.side == kind && removable(m)),
                reserved: need > 0 && spare == 0 && free > 0,
                closable: free > 0 && need == 0 && other_need > 0,
                open: free > 0,
            }
        };
        Slots {
            prefix: side(0),
            suffix: side(1),
        }
    }

    pub fn on(&self, side: Kind) -> &SideSlots {
        match side {
            Kind::Prefix => &self.prefix,
            Kind::Suffix => &self.suffix,
        }
    }

    pub fn both(&self) -> [&SideSlots; 2] {
        [&self.prefix, &self.suffix]
    }

    pub fn any(&self, f: impl Fn(&SideSlots) -> bool) -> bool {
        self.both().into_iter().any(f)
    }

    pub fn open_count(&self) -> usize {
        self.both().into_iter().filter(|s| s.open).count()
    }

    /// Both sides open and one of them reserved: an unrestricted exalt
    /// could take the reserved side's slot.
    pub fn two_open_one_reserved(&self) -> bool {
        self.open_count() == 2 && self.any(|s| s.reserved)
    }
}

/// What a removal step may aim at: a modifier that is not a target and not
/// fractured, unrevealed or desecrated (a projected filler is one).
pub(crate) fn removable(m: &ModInfo) -> bool {
    !matches!(m.role, Role::Wanted(_)) && !m.fractured && !m.unrevealed && !m.desecrated
}

/// How a removal chooses among the modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Takes {
    /// An Orb of Annulment, confined to `side` by a Sinistral or Dextral
    /// omen when it is `Some`.
    Any(Option<Kind>),
    /// With Omen of Light: a desecrated modifier.
    Desecrated,
}

/// The positions in `st.mods` of what a removal may actually take: for an
/// annulment every modifier (on its side) that is not fractured, unrevealed
/// and desecrated ones included; for Omen of Light the desecrated ones.
pub(crate) fn at_risk(st: &State, takes: Takes) -> Vec<usize> {
    st.mods
        .iter()
        .enumerate()
        .filter(|(_, m)| {
            !m.fractured
                && match takes {
                    Takes::Any(side) => side.is_none_or(|s| m.side == s),
                    Takes::Desecrated => m.desecrated,
                }
        })
        .map(|(i, _)| i)
        .collect()
}

/// The position in `st.mods` of the one modifier Omen of Whittling is
/// certain to take on `side`: on a Rare item whose every modifier's level
/// is known, the lowest level is held by exactly one, a removable
/// non-target on `side`, and no fractured modifier is at or below it.
pub(crate) fn whittle_certain(st: &State, side: Option<Kind>) -> Option<usize> {
    let side = side?;
    if st.rarity != Some(BpRarity::Rare) || st.mods.is_empty() {
        return None;
    }
    if st.mods.iter().any(|m| m.unrevealed || m.filler) {
        return None;
    }
    let levels: Vec<u32> = st.mods.iter().map(|m| m.level).collect::<Option<_>>()?;
    let lowest = *levels.iter().min()?;
    let mut holders = levels.iter().enumerate().filter(|(_, l)| **l == lowest);
    let (only, _) = holders.next()?;
    if holders.next().is_some() {
        return None;
    }
    let m = &st.mods[only];
    if m.side != side || !removable(m) {
        return None;
    }
    if st
        .mods
        .iter()
        .zip(&levels)
        .any(|(m, l)| m.fractured && *l <= lowest)
    {
        return None;
    }
    Some(only)
}
