//! An essence swap's removal as a state-ordered step tells it
//! (docs/models/crafting-slots-spec.md, "The essence swap and an unrevealed
//! modifier"): what the removal may take on its side, the fractured
//! modifiers it leaves, and, when it may take an unrevealed desecrated
//! modifier, the chance that one survives and the step the route takes
//! next if it stays and if it goes.
//!
//! The add's own odds and the order "Certain removals first" gives are not
//! touched here: the projection's hit still takes the first removable
//! non-target, which is the "stays" outcome. Each outcome's next step is
//! state order's own pick ([`next_step`]) on the item that outcome leaves;
//! state order keeps no miss branch, so a later step that misses re-plans
//! when the item is read again.

use ch_blueprint::{Side, Spanned, Step};
use ch_data::Odds;
use serde::Serialize;

use crate::context::{side_of, Ctx, ModInfo, Role, State};
use crate::odds::{is_essence_swap, swap_removal_side};
use crate::route::StepView;
use crate::state_order::{
    apply_hit, ensure_bindings, mark_waiting, next_step, place, view_with, Bindings, Pick,
};

/// A modifier on the item as the swap's removal sees it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SwapMod {
    pub side: Side,
    /// Its lines as shown on the item; empty for an unrevealed modifier and
    /// for one an earlier step of the preview placed.
    pub lines: Vec<String>,
    pub desecrated: bool,
    pub unrevealed: bool,
    /// The target it is, at its floor or better.
    pub target: Option<String>,
}

/// The chance an unrevealed desecrated modifier the removal may take
/// survives the swap, and the step taken next on each outcome.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Survival {
    /// The unrevealed modifier's side.
    pub side: Side,
    /// (n-1) in n, n the modifiers the removal may take
    /// (`removal-equally-likely`, not established; that the removal may
    /// take an unrevealed modifier at all is owner-stated).
    pub odds: Odds,
    /// The step the route takes next if it stays (the preview's own hit);
    /// `None` when no target is then missing or no step is a candidate.
    pub stays: Option<Box<StepView>>,
    /// The step the route takes next if it goes and the essence's modifier
    /// takes its slot; `None` as for `stays`.
    pub goes: Option<Box<StepView>>,
}

/// What an essence swap step's removal does, on the item as the step finds
/// it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EssenceSwap {
    /// The side it removes from: the Crystallisation omen's, else the
    /// target's when that side is full (`essence-full-side-removal`, not
    /// established); `None`, either side.
    pub side: Option<Side>,
    /// The modifiers it may take, in the item's order: every one there that
    /// is not fractured (`fractured-not-removed`, not established).
    pub removes: Vec<SwapMod>,
    /// The fractured modifiers there, which it leaves.
    pub fractured: Vec<SwapMod>,
    /// When it may take an unrevealed desecrated modifier.
    pub survival: Option<Survival>,
}

/// The step's [`EssenceSwap`], or `None` when it is not an essence swap
/// with a bound target. `outcomes` false leaves the survival's next steps
/// out (`stays` and `goes` are `None`).
pub(crate) fn essence_swap(
    ctx: &Ctx,
    st: &State,
    steps: &[Spanned<Step>],
    bindings: &Bindings,
    pick: &Pick,
    outcomes: bool,
) -> Option<EssenceSwap> {
    if !is_essence_swap(&pick.action) {
        return None;
    }
    let t = pick.target?;
    let side = swap_removal_side(ctx, st, &pick.action, t);
    let there = |m: &&ModInfo| side.is_none_or(|s| s == m.side);
    let named = |m: &ModInfo| SwapMod {
        side: side_of(m.side),
        lines: m.texts.clone(),
        desecrated: m.desecrated,
        unrevealed: m.unrevealed,
        target: match m.role {
            Role::Wanted(w) => Some(ctx.targets[w].mod_id.clone()),
            _ => None,
        },
    };
    let removes: Vec<SwapMod> = st
        .mods
        .iter()
        .filter(there)
        .filter(|m| !m.fractured)
        .map(named)
        .collect();
    let fractured: Vec<SwapMod> = st
        .mods
        .iter()
        .filter(there)
        .filter(|m| m.fractured)
        .map(named)
        .collect();
    let survival = st
        .mods
        .iter()
        .position(|m| m.unrevealed && !m.fractured && side.is_none_or(|s| s == m.side))
        .map(|unrevealed| {
            let n = removes.len() as u64;
            let odds = Odds::known(
                n - 1,
                n,
                "the unrevealed modifier is not the one removed (owner-stated, not established; \
                 each equally likely: removal-equally-likely, not established)",
            );
            let (stays, goes) = if outcomes {
                (
                    (n > 1).then(|| apply_hit(ctx, st, steps, pick, bindings)),
                    Some(gone(ctx, st, unrevealed, t)),
                )
            } else {
                (None, None)
            };
            Survival {
                side: side_of(st.mods[unrevealed].side),
                odds,
                stays: stays.and_then(|after| next_on(ctx, steps, bindings, after)),
                goes: goes.and_then(|after| next_on(ctx, steps, bindings, after)),
            }
        });
    Some(EssenceSwap {
        side: side.map(side_of),
        removes,
        fractured,
        survival,
    })
}

/// The item after the swap took the unrevealed modifier at `unrevealed`
/// and added target `t`.
fn gone(ctx: &Ctx, st: &State, unrevealed: usize, t: usize) -> State {
    let mut after = st.clone();
    after.projected = true;
    after.mods.remove(unrevealed);
    place(ctx, &mut after, t);
    after
}

/// The step state order takes next on `st`, its bindings made as the
/// projection makes them; `None` when no target is missing or no step is a
/// candidate.
fn next_on(
    ctx: &Ctx,
    steps: &[Spanned<Step>],
    bindings: &Bindings,
    mut st: State,
) -> Option<Box<StepView>> {
    let mut bindings = bindings.clone();
    ensure_bindings(ctx, &st, steps, &mut bindings);
    mark_waiting(ctx, &mut st, steps, &bindings);
    if st.missing(ctx.targets.len()).is_empty() {
        return None;
    }
    let pick = next_step(ctx, &st, steps, &bindings).ok()?;
    Some(Box::new(view_with(ctx, &st, steps, &bindings, pick, false)))
}
