//! The rough single-step odds of a step's first try on the item: never a
//! simulation, and "unknown" with the reason where the weights are not
//! established (every jewel page, desecration, reveals, fluxes).

use ch_blueprint::catalogue::{self, Effect, OmenChange};
use ch_blueprint::{Goal, Rarity as BpRarity};
use ch_data::{Kind, Odds, Removal, Section, WeightBasis, PLACEHOLDER_WEIGHT};

use crate::catalogue::data_entries;
use crate::context::{item_rarity, kind_of, Ctx, State};
use crate::route::ChosenAction;

/// The odds of the step's goal on its first try, or `None` for a manual
/// step (no check, no odds).
pub(crate) fn step_odds(
    ctx: &Ctx,
    st: &State,
    goal: Goal,
    aims: &[usize],
    action: &ChosenAction,
) -> Option<Odds> {
    if goal == Goal::Manual || action.effect == Effect::Manual {
        return None;
    }
    match action.effect {
        Effect::Desecrate | Effect::Reveal => {
            return Some(Odds::unknown(
                "desecration and reveals have no established weights",
            ))
        }
        Effect::Convert => return Some(Odds::unknown("how a flux maps tiers is not established")),
        _ => {}
    }
    Some(match goal {
        Goal::RemoveUnwanted => removal_chance(st, action),
        Goal::RemoveDesecrated => ch_data::removal_odds(
            &st.join,
            &st.wanted_positions(),
            action.side.map(kind_of),
            Removal::Desecrated,
        ),
        Goal::Fill => {
            if add_sides(ctx, st, action).is_empty() {
                Odds::certain(false, "no free slot on the allowed side")
            } else {
                Odds::certain(true, "any modifier fills a free slot")
            }
        }
        Goal::AddWanted => add_wanted_odds(ctx, st, aims, action),
        Goal::Desecrate | Goal::Reveal | Goal::Convert => {
            Odds::unknown("desecration, reveals and fluxes have no established weights")
        }
        Goal::Manual => return None,
    })
}

/// How the action's omens choose the removed modifier.
fn removal_rule(action: &ChosenAction) -> Removal {
    let changes: Vec<OmenChange> = action
        .omens
        .iter()
        .filter_map(|id| catalogue::entry(id).and_then(|e| e.omen).map(|o| o.change))
        .collect();
    if changes.contains(&OmenChange::RemoveLowestLevel) {
        Removal::Whittle
    } else if changes.contains(&OmenChange::DesecratedOnly) {
        Removal::Desecrated
    } else {
        Removal::Random
    }
}

pub(crate) fn has_omen_change(action: &ChosenAction, change: OmenChange) -> bool {
    action
        .omens
        .iter()
        .filter_map(|id| catalogue::entry(id).and_then(|e| e.omen))
        .any(|o| o.change == change)
}

/// The chance the modifier the action removes is not a wanted one.
pub(crate) fn removal_chance(st: &State, action: &ChosenAction) -> Odds {
    ch_data::removal_odds(
        &st.join,
        &st.wanted_positions(),
        action.side.map(kind_of),
        removal_rule(action),
    )
}

/// The sides one random add of this action can land on: the side omen's,
/// or every side with a free slot under the rarity the item has after the
/// action (a regal makes it Rare). A chaos-style swap also lands where its
/// removal opens a slot.
pub(crate) fn add_sides(ctx: &Ctx, st: &State, action: &ChosenAction) -> Vec<Kind> {
    let after = match action.effect {
        Effect::Upgrade => Some(BpRarity::Rare),
        Effect::Transmute => Some(BpRarity::Magic),
        _ => st.rarity,
    };
    let Some(limits) = after.and_then(|r| ch_data::item_affix_limits(ctx.item, &item_rarity(r)))
    else {
        return Vec::new();
    };
    let swap = action.effect == Effect::Swap;
    let omen_side = action.side.map(kind_of);
    [Kind::Prefix, Kind::Suffix]
        .into_iter()
        .filter(|side| swap || omen_side.is_none_or(|s| s == *side))
        .filter(|side| {
            let free = limits.on(*side) > st.count(*side);
            let opens = swap
                && omen_side.is_none_or(|s| s == *side)
                && st.mods.iter().any(|m| m.side == *side && !m.fractured);
            free || opens
        })
        .collect()
}

fn add_wanted_odds(ctx: &Ctx, st: &State, aims: &[usize], action: &ChosenAction) -> Odds {
    if aims.is_empty() {
        return Odds::certain(false, "no remaining target");
    }
    if action.data_backed {
        let entries = data_entries(ctx.data, &action.uses);
        for &t in aims {
            let target = &ctx.targets[t];
            let tier = entries.iter().find_map(|e| {
                let m = e.essence.matches.get(ctx.variant)?.as_ref()?;
                (m.mod_id == target.mod_id && m.tier <= target.floor).then_some(m.tier)
            });
            if let Some(tier) = tier {
                let add = Odds::certain(
                    true,
                    format!("{} adds {} at T{tier}", action.name, target.mod_id),
                );
                return if action.removes {
                    add.times(&removal_chance(st, action))
                } else {
                    add
                };
            }
        }
        return Odds::certain(
            false,
            format!("{} adds no remaining target at its floor", action.name),
        );
    }
    match action.effect {
        Effect::Add | Effect::Augment | Effect::Transmute | Effect::Upgrade | Effect::Swap => {
            let sides = add_sides(ctx, st, action);
            let add = add_chance(ctx, st, aims, &sides, action.min_modifier_level);
            let add = if has_omen_change(action, OmenChange::AddTwo) {
                with_basis_note(add, "for each of the two modifiers added")
            } else {
                add
            };
            if action.effect == Effect::Swap {
                add.times(&removal_chance(st, action))
            } else {
                add
            }
        }
        Effect::Alchemy => Odds::unknown("an Orb of Alchemy adds four modifiers at once"),
        other => Odds::unknown(format!("{other:?} adds no random modifier")),
    }
}

fn with_basis_note(odds: Odds, note: &str) -> Odds {
    match odds {
        Odds::Known {
            percent,
            hits,
            total,
            basis,
        } => Odds::Known {
            percent,
            hits,
            total,
            basis: format!("{basis}, {note}"),
        },
        unknown => unknown,
    }
}

/// The chance one random add on `sides` rolls any of `aims` at its floor:
/// the aims' counting weight over the pool's. The pool and the weight
/// rules are `ch-data`'s (`ClassData::add_pool`, `Pool::odds`), widened to
/// several targets.
pub(crate) fn add_chance(
    ctx: &Ctx,
    st: &State,
    aims: &[usize],
    sides: &[Kind],
    min_modifier_level: Option<u32>,
) -> Odds {
    match ctx.data.page(ctx.variant).map(|p| p.weights.normal) {
        Some(WeightBasis::Spawn) => {}
        _ => {
            return Odds::unknown(format!(
                "{} `{}` has no established spawn weights",
                ctx.data.class, ctx.variant
            ))
        }
    }
    if aims
        .iter()
        .any(|&t| ctx.targets[t].section == Section::Desecrated)
    {
        return Odds::unknown("a desecrated modifier has no established weight");
    }
    if let Some(level) = min_modifier_level {
        if ctx.item_level < level {
            return Odds::unknown(format!(
                "item level {} is below the minimum modifier level {level}",
                ctx.item_level
            ));
        }
    }
    let mut families: Vec<String> = Vec::new();
    for m in st.mods.iter().filter(|m| !m.unrevealed) {
        if m.mod_id.is_none() {
            return Odds::unknown(format!(
                "the modifier `{}` did not join the data",
                m.shown()
            ));
        }
        families.extend(m.family.iter().cloned());
    }
    if sides.is_empty() {
        return Odds::unknown("no free slot on the allowed side");
    }
    let pool = ctx.data.add_pool(
        ctx.variant,
        ctx.item_level,
        sides,
        &families,
        min_modifier_level,
    );
    let mut total: u64 = 0;
    for entry in &pool.entries {
        for tier in &entry.tiers {
            match tier.weight {
                Some(w) => total += u64::from(w),
                None => {
                    return Odds::unknown(format!(
                        "{} tier {} has no weight in the data",
                        entry.id, tier.tier
                    ))
                }
            }
        }
    }
    let mut hits: u64 = 0;
    for &t in aims {
        let target = &ctx.targets[t];
        let Some(entry) = pool.entries.iter().find(|e| e.id == target.mod_id) else {
            continue;
        };
        for tier in entry.tiers.iter().filter(|x| x.tier <= target.floor) {
            match tier.weight {
                Some(w) if w != PLACEHOLDER_WEIGHT => hits += u64::from(w),
                _ => {
                    return Odds::unknown(format!(
                        "{} tier {} has no real weight in the data",
                        target.mod_id, tier.tier
                    ))
                }
            }
        }
    }
    if total == 0 {
        return Odds::unknown("the pool is empty");
    }
    Odds::known(hits, total, "weight")
}

/// A target's add odds on its own side, slots aside: what "hardest"
/// compares.
pub(crate) fn target_difficulty(ctx: &Ctx, st: &State, t: usize) -> Option<f64> {
    add_chance(ctx, st, &[t], &[ctx.targets[t].side], None).percent()
}
