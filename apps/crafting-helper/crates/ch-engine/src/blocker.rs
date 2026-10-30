//! Blocker advice: which desecrated modifier to take at a reveal when the
//! target is not offered (docs/models/crafting-slots-spec.md, "A wrong
//! reveal waits for the same-side exalt").
//!
//! The app never sees the modifiers the Well of Souls offers, so the advice
//! is given at the reveal step, for every candidate it might be offered: on
//! a state-ordered route whose reveal is followed later in the projection
//! by an add on the same side with an Exalted Orb of any grade, each of the
//! class variant's desecrated modifiers on that side that the item level
//! reaches and that is not a target is scored by that exalt's chance of a
//! target with the modifier on the item. The add odds drop every family of
//! every revealed modifier on the item from the pool (desecrated ones
//! included, the reading the blocker blueprints rely on), so a modifier
//! that shares a family with a competitor keeps that competitor out. The
//! route keeps it through the exalt (Omen of Light waits). Ordinary
//! modifiers a reveal may offer are left out: which ones is not known.

use ch_blueprint::{Goal, Order, Side, Spanned, Step};
use ch_data::{DataMod, Odds};
use ch_item::Item;
use serde::Serialize;

use crate::context::{kind_of, side_of, Ctx, Refusal, Role, State, Target};
use crate::engine::Engine;
use crate::odds::add_chance;
use crate::route::route_for;
use crate::state_order::{placeable, project, Projection};

/// The most blocker modifiers one reveal step names.
pub const BLOCKER_LIMIT: usize = 3;

/// The Exalted Orbs, every grade: the adds the advice is scored against.
const EXALTS: [&str; 3] = ["exalted-orb", "greater-exalted-orb", "perfect-exalted-orb"];

/// A data modifier as the advice names it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BlockerMod {
    pub mod_id: String,
    /// The modifier's text, `#` for each value.
    pub lines: Vec<String>,
}

/// One or more desecrated modifiers that raise the exalt's chance equally.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BlockerPick {
    /// In data order.
    pub mods: Vec<BlockerMod>,
    /// The ordinary modifiers of the exalt's pool they keep out (a shared
    /// family), in data order.
    pub keeps_out: Vec<BlockerMod>,
    /// The exalt's chance of a target with one of them on the item.
    pub after: Odds,
}

/// What a reveal step tells the player to take when its target is not
/// offered.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BlockerAdvice {
    /// The reveal step's id.
    pub step: String,
    /// Its position in the route's steps (the projection), from 0.
    pub position: usize,
    /// The target the desecration was made for, when the route knows it.
    pub target: Option<BlockerMod>,
    /// The side the desecrated modifier is on.
    pub side: Side,
    /// The same-side exalt the chances are of: its step id and position.
    pub exalt_step: String,
    pub exalt_position: usize,
    /// The exalt's chance of a target with the desecrated slot taken by a
    /// modifier that keeps nothing out.
    pub before: Odds,
    /// Best first, at most [`BLOCKER_LIMIT`] modifiers in all; only those
    /// that raise the chance.
    pub picks: Vec<BlockerPick>,
}

impl Engine {
    /// The blocker advice of `blueprint`'s route on the item, or `None`:
    /// the blueprint is unknown, listed-order or rejected on the item, its
    /// projection has no reveal followed by a same-side exalt (a
    /// desecration that is the route's last add), the chances are not
    /// known, or no desecrated modifier raises them. Refuses what
    /// [`Engine::plan`] refuses.
    pub fn blocker_advice(
        &self,
        item: &Item,
        targets: &[Target],
        blocked: &[String],
        blueprint: &str,
    ) -> Result<Option<BlockerAdvice>, Refusal> {
        let ctx = Ctx::new(item, targets, blocked)?;
        let Some(resolved) = self.blueprint(blueprint) else {
            return Ok(None);
        };
        let b = resolved.blueprint();
        if b.order() != Order::State || route_for(&ctx, resolved).is_err() {
            return Ok(None);
        }
        let st = State::new(item, &ctx.targets);
        let Ok(projection) = project(&ctx, &b.steps, &st) else {
            return Ok(None);
        };
        Ok(advice(&ctx, &b.steps, &projection))
    }
}

fn advice(ctx: &Ctx, steps: &[Spanned<Step>], projection: &Projection) -> Option<BlockerAdvice> {
    let views = &projection.steps;
    let states = &projection.states;
    let reveal = views.iter().position(|v| v.goal == Goal::Reveal)?;
    let unrevealed = states[reveal].mods.iter().find(|m| m.unrevealed)?;
    let side = unrevealed.side;
    let target = unrevealed
        .waits_for
        .or_else(|| {
            views[reveal]
                .target
                .as_deref()
                .and_then(|id| ctx.target_index(id))
        })
        .map(|t| BlockerMod {
            mod_id: ctx.targets[t].mod_id.clone(),
            lines: ctx.targets[t].lines.clone(),
        });

    // The last add on the same side with an Exalted Orb, after the reveal.
    let exalt = (reveal + 1..views.len()).rev().find(|&i| {
        let v = &views[i];
        v.goal == Goal::AddWanted
            && v.side.map(kind_of) == Some(side)
            && v.action
                .as_ref()
                .is_some_and(|a| EXALTS.contains(&a.uses.as_str()))
    })?;
    let view = &views[exalt];
    let action = view.action.as_ref()?;
    let step = steps
        .iter()
        .map(|s| s.get_ref())
        .find(|s| *s.id.get_ref() == view.id)?;

    // The item as the exalt finds it, with the reveal's slot taken by `with`
    // (`None`: a modifier that keeps nothing out) instead of the target.
    let at_exalt = &states[exalt];
    let slot = at_exalt
        .mods
        .iter()
        .position(|m| m.side == side && m.desecrated && !m.unrevealed)?;
    let wrong = |with: Option<&DataMod>| {
        let mut st = at_exalt.clone();
        let m = &mut st.mods[slot];
        m.role = Role::Unwanted;
        m.filler = with.is_none();
        m.mod_id = with.map(|d| d.id.clone());
        m.family = with.map(|d| d.family.clone()).unwrap_or_default();
        m.tier = None;
        m.level = None;
        st
    };
    let chance = |st: &State| {
        let aims: Vec<usize> = placeable(ctx, st, step, steps, &projection.bindings)
            .into_iter()
            .filter(|&t| ctx.targets[t].side == side)
            .collect();
        if aims.is_empty() {
            return None;
        }
        Some(add_chance(
            ctx,
            st,
            &aims,
            &[side],
            action.min_modifier_level,
        ))
    };
    let before_state = wrong(None);
    let before = chance(&before_state)?;
    let floor = before.percent()?;

    let named = |d: &DataMod| BlockerMod {
        mod_id: d.id.clone(),
        lines: d.lines.clone(),
    };
    let item_families: Vec<String> = before_state
        .mods
        .iter()
        .filter(|m| !m.unrevealed && !m.filler)
        .flat_map(|m| m.family.iter().cloned())
        .collect();
    let pool = ctx.data.add_pool(
        ctx.variant,
        ctx.item_level,
        &[side],
        &item_families,
        action.min_modifier_level,
    );

    let mut scored: Vec<(f64, BlockerMod, Vec<BlockerMod>, Odds)> = Vec::new();
    for valid in ctx
        .data
        .valid_mods(ctx.variant, ctx.item_level)
        .desecrated
        .iter()
        .filter(|m| m.kind == side && ctx.target_index(m.id).is_none())
    {
        let Some(data) = ctx.data.find_mod(valid.id) else {
            continue;
        };
        let Some(after) = chance(&wrong(Some(data))) else {
            continue;
        };
        let Some(percent) = after.percent() else {
            continue;
        };
        if percent <= floor {
            continue;
        }
        let keeps_out = pool
            .entries
            .iter()
            .filter_map(|e| ctx.data.find_mod(e.id))
            .filter(|competitor| competitor.shares_family(&data.family))
            .map(named)
            .collect();
        scored.push((percent, named(data), keeps_out, after));
    }
    // Best first; equal chances keep data order.
    scored.sort_by(|a, b| b.0.total_cmp(&a.0));
    scored.truncate(BLOCKER_LIMIT);
    if scored.is_empty() {
        return None;
    }

    let mut picks: Vec<BlockerPick> = Vec::new();
    for (_, m, keeps_out, after) in scored {
        match picks.last_mut() {
            Some(last) if last.after == after || same_chance(&last.after, &after) => {
                last.mods.push(m);
                for k in keeps_out {
                    if !last.keeps_out.contains(&k) {
                        last.keeps_out.push(k);
                    }
                }
            }
            _ => picks.push(BlockerPick {
                mods: vec![m],
                keeps_out,
                after,
            }),
        }
    }
    Some(BlockerAdvice {
        step: views[reveal].id.clone(),
        position: reveal,
        target,
        side: side_of(side),
        exalt_step: view.id.clone(),
        exalt_position: exalt,
        before,
        picks,
    })
}

/// The same chance: the same weights hit of the same total.
fn same_chance(a: &Odds, b: &Odds) -> bool {
    match (a, b) {
        (
            Odds::Known {
                hits: h1,
                total: t1,
                ..
            },
            Odds::Known {
                hits: h2,
                total: t2,
                ..
            },
        ) => u128::from(*h1) * u128::from(*t2) == u128::from(*h2) * u128::from(*t1),
        _ => false,
    }
}
