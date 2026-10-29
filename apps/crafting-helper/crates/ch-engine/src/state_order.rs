//! State order (docs/models/crafting-slots-spec.md, "Route rules" and
//! "Projection"): for a blueprint with `order = "state"`, the next step is
//! the first in file order that is a candidate on the item as it stands
//! (its guard holds, its binding is made, its goal is not met, its side
//! rule finds a side and an alternative fits), and the route preview is the
//! projection, the steps the craft takes if each one hits.
//!
//! Every number here is [`Slots`]'s. Listed blueprints never come here.

use std::collections::BTreeMap;

use ch_blueprint::catalogue::{
    bones_for, entry, Currency, Effect, ItemGroup, OmenChange, GREATER_OMEN, SIDE_OMEN,
};
use ch_blueprint::{Action, Binding, Goal, Rarity as BpRarity, SideRule, Spanned, Step, When};
use ch_data::{Kind, Section};

use crate::catalogue::data_entries;
use crate::context::{
    condition_failures, item_rarity, kind_of, side_of, Ctx, ModInfo, Role, State,
};
use crate::odds::{add_sides, has_omen_change, step_odds, with_basis_note};
use crate::route::{
    essence_adds, essence_binds_any, essence_fits_rarity, greater_fits, hardest, try_alternative,
    AltProblem, ChosenAction, Rejection, Skipped, Stall, StepView,
};
use crate::slots::{at_risk, removable, whittle_certain, Slots, Takes, SIDES};

/// A projection that reaches this many steps without finishing is looping
/// (the spec's "Step cap").
pub const STEP_CAP: usize = 16;

/// Step id to the target id its `essence`, `hardest` or `last-remaining`
/// binding chose.
pub(crate) type Bindings = BTreeMap<String, String>;

/// The position `Item::modifiers` gives a modifier the projection placed:
/// none.
const PROJECTED: usize = usize::MAX;

/// A step that is a candidate, as it would be taken.
pub(crate) struct Pick {
    /// Its position in the blueprint's steps.
    pub index: usize,
    /// The side its side rule resolved to; `None` for a step without one.
    pub side: Option<Kind>,
    /// The target its `essence`, `hardest` or `last-remaining` binding
    /// names.
    pub target: Option<usize>,
    /// The targets a hit may produce.
    pub aims: Vec<usize>,
    pub action: ChosenAction,
}

/// The route preview: the steps, those never taken (with why, on the item
/// as it stands), and the bindings the projection made.
pub(crate) struct Projection {
    pub steps: Vec<StepView>,
    pub skipped: Vec<Skipped>,
    pub bindings: Bindings,
}

fn binding_of(step: &Step) -> Option<Binding> {
    step.bind.as_ref().map(|b| *b.get_ref())
}

/// The target the route's `kind` binding names, when it is made. Every step
/// with that binding shares it.
fn bound(ctx: &Ctx, steps: &[Spanned<Step>], bindings: &Bindings, kind: Binding) -> Option<usize> {
    steps
        .iter()
        .map(|s| s.get_ref())
        .filter(|s| binding_of(s) == Some(kind))
        .find_map(|s| bindings.get(s.id.get_ref()))
        .and_then(|m| ctx.target_index(m))
}

/// The target a step's own `essence`, `hardest` or `last-remaining` binding
/// names.
fn step_target(ctx: &Ctx, step: &Step, bindings: &Bindings) -> Option<usize> {
    match binding_of(step) {
        Some(Binding::Essence | Binding::Hardest | Binding::LastRemaining) => bindings
            .get(step.id.get_ref())
            .and_then(|m| ctx.target_index(m)),
        _ => None,
    }
}

fn bind_all(steps: &[Spanned<Step>], bindings: &mut Bindings, kind: Binding, mod_id: &str) {
    for s in steps.iter().map(|s| s.get_ref()) {
        if binding_of(s) == Some(kind) {
            bindings.insert(s.id.get_ref().clone(), mod_id.to_string());
        }
    }
}

/// Makes each binding the blueprint uses as soon as the item allows it, and
/// keeps one once made: `essence` on a Magic item, to the first missing
/// target (in target order) an unblocked essence of the essence step adds
/// at its floor; `hardest` while a target is missing, to a missing
/// desecrated-section target first, else the one with the lowest add odds
/// (leaving out the essence's). `last-remaining` is made afresh on every
/// item, to the one missing target when exactly one is missing: kept like
/// `hardest`, a miss that leaves a second target missing would keep a
/// desecration bound to a target that is no longer the last.
pub(crate) fn ensure_bindings(
    ctx: &Ctx,
    st: &State,
    steps: &[Spanned<Step>],
    bindings: &mut Bindings,
) {
    let uses = |kind: Binding| steps.iter().any(|s| binding_of(s.get_ref()) == Some(kind));
    let missing = st.missing(ctx.targets.len());
    if uses(Binding::Essence)
        && bound(ctx, steps, bindings, Binding::Essence).is_none()
        && st.rarity == Some(BpRarity::Magic)
    {
        let actions: Vec<&Action> = steps
            .iter()
            .map(|s| s.get_ref())
            .filter(|s| binding_of(s) == Some(Binding::Essence))
            .flat_map(|s| s.actions.iter().map(|a| a.get_ref()))
            .collect();
        let found = missing.iter().copied().find(|&t| {
            ctx.targets[t].section == Section::Normal
                && actions.iter().any(|a| essence_action_adds(ctx, st, a, t))
        });
        if let Some(t) = found {
            bind_all(
                steps,
                bindings,
                Binding::Essence,
                &ctx.targets[t].mod_id.clone(),
            );
        }
    }
    if uses(Binding::Hardest)
        && bound(ctx, steps, bindings, Binding::Hardest).is_none()
        && !missing.is_empty()
    {
        let claimed = bound(ctx, steps, bindings, Binding::Essence);
        let free: Vec<usize> = missing
            .iter()
            .copied()
            .filter(|&t| Some(t) != claimed)
            .collect();
        let pool = if free.is_empty() { &missing } else { &free };
        let chosen = pool
            .iter()
            .copied()
            .find(|&t| ctx.targets[t].section == Section::Desecrated)
            .or_else(|| hardest(ctx, st, pool));
        if let Some(t) = chosen {
            bind_all(
                steps,
                bindings,
                Binding::Hardest,
                &ctx.targets[t].mod_id.clone(),
            );
        }
    }
    if uses(Binding::LastRemaining) {
        for s in steps.iter().map(|s| s.get_ref()) {
            if binding_of(s) == Some(Binding::LastRemaining) {
                bindings.remove(s.id.get_ref());
            }
        }
        if let [last] = missing.as_slice() {
            bind_all(
                steps,
                bindings,
                Binding::LastRemaining,
                &ctx.targets[*last].mod_id.clone(),
            );
        }
    }
}

/// Whether `action` is an unblocked essence that adds target `t` to the item
/// at its floor.
fn essence_action_adds(ctx: &Ctx, st: &State, action: &Action, t: usize) -> bool {
    let id = action.currency.get_ref().as_str();
    let Some(Currency::DataBacked(shape)) = Currency::of(id) else {
        return false;
    };
    let omen_blocked = action.omens.iter().any(|o| {
        let o = o.get_ref();
        o != SIDE_OMEN && o != GREATER_OMEN && ctx.is_blocked(o)
    });
    !ctx.is_blocked(id)
        && !omen_blocked
        && essence_fits_rarity(shape.effect, st.rarity, false)
        && essence_adds(ctx, st, &data_entries(ctx.data, id), shape.effect, t).is_some()
}

/// The missing targets the step could place: for the essence step, its
/// bound target; otherwise the normal-section ones, less the one the
/// route's essence binding claims.
fn placeable(
    ctx: &Ctx,
    st: &State,
    step: &Step,
    steps: &[Spanned<Step>],
    bindings: &Bindings,
) -> Vec<usize> {
    let missing = st.missing(ctx.targets.len());
    let claimed = bound(ctx, steps, bindings, Binding::Essence);
    if binding_of(step) == Some(Binding::Essence) {
        return claimed
            .filter(|t| missing.contains(t))
            .into_iter()
            .collect();
    }
    missing
        .into_iter()
        .filter(|&t| ctx.targets[t].section == Section::Normal && Some(t) != claimed)
        .collect()
}

/// `Err` with why when the step's goal is met (or it has none here).
fn goal_open(
    ctx: &Ctx,
    st: &State,
    slots: &Slots,
    step: &Step,
    steps: &[Spanned<Step>],
    bindings: &Bindings,
) -> Result<(), String> {
    let missing = st.missing(ctx.targets.len());
    let (open, why) = match *step.goal.get_ref() {
        Goal::AddWanted => (
            !placeable(ctx, st, step, steps, bindings).is_empty(),
            "no target it can place is missing",
        ),
        Goal::Fill => (true, ""),
        Goal::Desecrate if st.has_desecrated() => {
            (false, "a desecrated modifier is already on the item")
        }
        Goal::Desecrate => (
            step_target(ctx, step, bindings).is_some_and(|t| missing.contains(&t)),
            "its bound target is on the item",
        ),
        Goal::Reveal => (st.has_unrevealed(), "no unrevealed modifier"),
        Goal::RemoveUnwanted => (slots.any(|s| s.blocked), "no side is blocked"),
        Goal::RemoveDesecrated => (st.has_desecrated(), "no desecrated modifier"),
        other => {
            return Err(format!(
                "a state-ordered route has no rule for a `{}` step",
                other.name()
            ))
        }
    };
    if open {
        Ok(())
    } else {
        Err(why.to_string())
    }
}

/// The side the step's side rule resolves to; `Ok(None)` for a step
/// without `side`.
fn resolve_side(
    ctx: &Ctx,
    st: &State,
    slots: &Slots,
    step: &Step,
    steps: &[Spanned<Step>],
    bindings: &Bindings,
) -> Result<Option<Kind>, String> {
    if let Some(side) = &step.side {
        return Ok(Some(kind_of(*side.get_ref())));
    }
    let Some(rule) = step.side_rule.as_ref().map(|r| *r.get_ref()) else {
        return Ok(None);
    };
    // Of `sides`, the side of the first of `targets` on one of them.
    let first_side = |targets: &[usize], sides: &[Kind]| {
        targets
            .iter()
            .map(|&t| ctx.targets[t].side)
            .find(|s| sides.contains(s))
    };
    match rule {
        SideRule::Target => {
            if let Some(t) = step_target(ctx, step, bindings) {
                return Ok(Some(ctx.targets[t].side));
            }
            let can = placeable(ctx, st, step, steps, bindings);
            let counts = SIDES.map(|s| {
                if slots.on(s).open {
                    can.iter().filter(|&&t| ctx.targets[t].side == s).count()
                } else {
                    0
                }
            });
            let best = counts.into_iter().max().unwrap_or(0);
            if best == 0 {
                return Err("no open side has a target it can place".to_string());
            }
            let tied: Vec<Kind> = SIDES
                .into_iter()
                .zip(counts)
                .filter(|(_, n)| *n == best)
                .map(|(s, _)| s)
                .collect();
            Ok(first_side(&can, &tied))
        }
        SideRule::Close => SIDES
            .into_iter()
            .find(|&s| slots.on(s).closable)
            .map(Some)
            .ok_or_else(|| "no side is closable".to_string()),
        SideRule::Blocked => {
            let blocked: Vec<Kind> = SIDES.into_iter().filter(|&s| slots.on(s).blocked).collect();
            match blocked.as_slice() {
                [] => Err("no side is blocked".to_string()),
                [only] => Ok(Some(*only)),
                _ => {
                    let deficit = |s: Kind| slots.on(s).need as isize - slots.on(s).free as isize;
                    let best = blocked.iter().map(|&s| deficit(s)).max().unwrap_or(0);
                    let tied: Vec<Kind> = blocked
                        .iter()
                        .copied()
                        .filter(|&s| deficit(s) == best)
                        .collect();
                    let missing = st.missing(ctx.targets.len());
                    Ok(first_side(&missing, &tied).or(tied.first().copied()))
                }
            }
        }
    }
}

/// Whether the item's class has a desecrated-section modifier on `side` at
/// the item's level (the craft window's Desecrated group): the
/// last-target desecration's pool condition.
fn has_desecration_pool(ctx: &Ctx, side: Kind) -> bool {
    ctx.data
        .valid_mods(ctx.variant, ctx.item_level)
        .desecrated
        .iter()
        .any(|m| m.kind == side)
}

fn no_desecration_pool(ctx: &Ctx, side: Kind) -> String {
    format!(
        "no desecrated {} modifier on {} at item level {}",
        side_of(side).name(),
        ctx.data.class.as_str(),
        ctx.item_level
    )
}

/// The names of what the player blocked that the step's alternatives need
/// for a desecration on `side`: every bone that fits the item, and each
/// omen (`side-omen` as that side's Necromancy omen).
fn blocked_names(ctx: &Ctx, step: &Step, side: Kind) -> Vec<&'static str> {
    let mut names: Vec<&'static str> = Vec::new();
    let mut add = |name: &'static str| {
        if !names.contains(&name) {
            names.push(name);
        }
    };
    for action in step.actions.iter().map(|a| a.get_ref()) {
        let id = action.currency.get_ref().as_str();
        let currency = Currency::of(id);
        match currency {
            Some(Currency::Desecrate) => {
                let bones = ItemGroup::of_class(ctx.data.class.as_str())
                    .map(|g| bones_for(g, ctx.item_level))
                    .unwrap_or_default();
                if !bones.is_empty() && bones.iter().all(|b| ctx.is_blocked(b.id)) {
                    bones.iter().for_each(|b| add(b.name));
                }
            }
            Some(Currency::Static(e)) if ctx.is_blocked(e.id) => add(e.name),
            _ => {}
        }
        for omen in action.omens.iter().map(|o| o.get_ref().as_str()) {
            let resolved = if omen == SIDE_OMEN {
                currency.and_then(|c| c.side_omen(side_of(side)))
            } else {
                entry(omen)
            };
            if let Some(e) = resolved.filter(|e| ctx.is_blocked(e.id)) {
                add(e.name);
            }
        }
    }
    names
}

/// `a`, `a and b`, `a, b and c`.
fn and_list(names: &[&str]) -> String {
    match names {
        [] => String::new(),
        [one] => one.to_string(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// Why the last-target desecration is not the step taken, in words, when
/// the rule applies (a rare with no desecrated modifier, unrevealed ones
/// included, and exactly one target missing) and the step at `picked`
/// comes after the blueprint's `last-remaining` desecration, so was taken
/// in its place (docs/models/crafting-slots-spec.md, "The last-target
/// desecration"): no pool on the target's side, the bone or the
/// Necromancy omen blocked, or the engine's own reason. `None` otherwise;
/// the steps before it (a reveal, Omen of Light, a removal) clear the way
/// for it rather than replace it.
fn desecration_passed_over(
    ctx: &Ctx,
    st: &State,
    steps: &[Spanned<Step>],
    bindings: &Bindings,
    picked: usize,
) -> Option<String> {
    if st.rarity != Some(BpRarity::Rare) || st.has_desecrated() {
        return None;
    }
    let missing = st.missing(ctx.targets.len());
    let [last] = missing.as_slice() else {
        return None;
    };
    let index = steps.iter().position(|s| {
        let s = s.get_ref();
        *s.goal.get_ref() == Goal::Desecrate && binding_of(s) == Some(Binding::LastRemaining)
    })?;
    if picked <= index {
        return None;
    }
    let slots = Slots::new(st, &ctx.targets);
    let why = evaluate(ctx, st, &slots, steps, index, bindings).err()?;
    let side = ctx.targets[*last].side;
    if !has_desecration_pool(ctx, side) {
        return Some(no_desecration_pool(ctx, side));
    }
    let blocked = blocked_names(ctx, steps[index].get_ref(), side);
    if !blocked.is_empty() {
        return Some(format!("{} blocked", and_list(&blocked)));
    }
    Some(why)
}

/// What a removal alternative takes.
fn takes(action: &ChosenAction) -> Takes {
    if has_omen_change(action, OmenChange::DesecratedOnly) {
        Takes::Desecrated
    } else {
        Takes::Any(action.side.map(kind_of))
    }
}

fn is_whittle(action: &ChosenAction) -> bool {
    has_omen_change(action, OmenChange::RemoveLowestLevel)
}

/// Whether alternative `index` fits the item (the spec's "Fitting" rules on
/// top of the blocked list and the placeholders).
#[allow(clippy::too_many_arguments)]
fn fit(
    ctx: &Ctx,
    st: &State,
    slots: &Slots,
    step: &Step,
    index: usize,
    side: Option<Kind>,
    essence_for: Option<usize>,
    can: &[usize],
) -> Result<ChosenAction, AltProblem> {
    let goal = *step.goal.get_ref();
    let action = step.actions[index].get_ref();
    let greater = greater_fits(ctx, st, goal, side, can);
    let chosen = try_alternative(
        ctx,
        st,
        index,
        action,
        side.map(side_of),
        greater,
        essence_for,
    )?;
    let unfit = |why: &str| Err(AltProblem::Unfit(format!("`{}`: {why}", chosen.currency)));
    let rarity = st.rarity;
    match chosen.effect {
        Effect::Upgrade if chosen.data_backed => {
            let Some(t) = essence_for else {
                return unfit("an essence needs the step's essence binding");
            };
            if rarity != Some(BpRarity::Magic) {
                return unfit("an essence needs a magic item");
            }
            let side = ctx.targets[t].side;
            let rare = ch_data::item_affix_limits(ctx.item, &item_rarity(BpRarity::Rare));
            if rare.is_none_or(|l| l.on(side) <= st.count(side)) {
                return unfit("no free slot where its modifier lands");
            }
            Ok(chosen)
        }
        Effect::Add | Effect::Augment | Effect::Transmute if !chosen.data_backed => {
            let needs = match chosen.effect {
                Effect::Add => BpRarity::Rare,
                Effect::Augment => BpRarity::Magic,
                _ => BpRarity::Normal,
            };
            if rarity != Some(needs) {
                return unfit(&format!("it needs a {} item", needs.name()));
            }
            if chosen.effect == Effect::Add
                && chosen.side.is_none()
                && slots.two_open_one_reserved()
            {
                return unfit("an exalt without a side omen could take the reserved side's slot");
            }
            let lands: Vec<Kind> = add_sides(ctx, st, &chosen)
                .into_iter()
                .filter(|s| side.is_none_or(|r| r == *s))
                .collect();
            if lands.is_empty() {
                return unfit("no free slot where it lands");
            }
            if goal == Goal::AddWanted && !can.iter().any(|&t| lands.contains(&ctx.targets[t].side))
            {
                return unfit("no target it can place where it lands");
            }
            Ok(chosen)
        }
        Effect::Desecrate => {
            if rarity != Some(BpRarity::Rare) {
                return unfit("it needs a rare item");
            }
            match side {
                None => unfit("it needs a side"),
                Some(s) if slots.on(s).free == 0 => unfit("no free slot on its side"),
                Some(s)
                    if binding_of(step) == Some(Binding::LastRemaining)
                        && !has_desecration_pool(ctx, s) =>
                {
                    unfit(&no_desecration_pool(ctx, s))
                }
                Some(_) => Ok(chosen),
            }
        }
        Effect::Reveal => {
            if st.has_unrevealed() {
                Ok(chosen)
            } else {
                unfit("nothing to reveal")
            }
        }
        Effect::Remove if !chosen.data_backed => {
            if !matches!(rarity, Some(BpRarity::Magic | BpRarity::Rare)) {
                return unfit("it needs a magic or rare item");
            }
            if has_omen_change(&chosen, OmenChange::RemoveTwo) {
                return unfit("a state-ordered route has no rule for removing two");
            }
            if let Some(omen_side) = chosen.side.map(kind_of) {
                let protects = st.mods.iter().any(|m| {
                    m.side == omen_side.other() && !m.fractured && matches!(m.role, Role::Wanted(_))
                });
                if !protects {
                    return unfit("the other side holds no target to protect");
                }
            }
            if at_risk(st, takes(&chosen)).is_empty() {
                return unfit("nothing it could take");
            }
            Ok(chosen)
        }
        Effect::Swap if !chosen.data_backed && is_whittle(&chosen) => {
            if whittle_certain(st, side).is_none() {
                return unfit("the whittle is not certain");
            }
            // The first annulment after it that fits decides: the whittle
            // is worth it only when that annulment could take a target.
            for later in index + 1..step.actions.len() {
                let currency = step.actions[later].get_ref().currency.get_ref().as_str();
                if !matches!(Currency::of(currency), Some(Currency::Static(e)) if e.effect == Effect::Remove)
                {
                    continue;
                }
                let Ok(annul) = fit(ctx, st, slots, step, later, side, essence_for, can) else {
                    continue;
                };
                let risks = at_risk(st, takes(&annul))
                    .into_iter()
                    .any(|i| matches!(st.mods[i].role, Role::Wanted(_)));
                return if risks {
                    Ok(chosen)
                } else {
                    unfit("no annulment that fits could take a target")
                };
            }
            unfit("no annulment that fits could take a target")
        }
        _ => unfit("a state-ordered route has no rule for it"),
    }
}

/// The step at `index` as a candidate on the item, or why it is not one.
pub(crate) fn evaluate(
    ctx: &Ctx,
    st: &State,
    slots: &Slots,
    steps: &[Spanned<Step>],
    index: usize,
    bindings: &Bindings,
) -> Result<Pick, String> {
    let step = steps[index].get_ref();
    if let Some(guard) = &step.guard {
        let binds = || essence_binds_any(ctx, st, true);
        let failed = condition_failures(guard.get_ref(), st, &ctx.targets, &binds);
        if !failed.is_empty() {
            return Err(format!("its guard does not hold ({})", failed.join("; ")));
        }
    }
    let target = step_target(ctx, step, bindings);
    let binds_one = matches!(
        binding_of(step),
        Some(Binding::Essence | Binding::Hardest | Binding::LastRemaining)
    );
    if step.when.as_ref().map(|w| *w.get_ref()) == Some(When::Bound)
        && binds_one
        && target.is_none()
    {
        return Err("its binding is not made".to_string());
    }
    goal_open(ctx, st, slots, step, steps, bindings)?;
    let side = resolve_side(ctx, st, slots, step, steps, bindings)?;
    let essence_for = (binding_of(step) == Some(Binding::Essence))
        .then_some(target)
        .flatten();
    let can = placeable(ctx, st, step, steps, bindings);
    let mut blocked = 0;
    let mut reasons = Vec::new();
    for i in 0..step.actions.len() {
        match fit(ctx, st, slots, step, i, side, essence_for, &can) {
            Ok(action) => {
                let aims = target.map_or(can, |t| vec![t]);
                return Ok(Pick {
                    index,
                    side,
                    target,
                    aims,
                    action,
                });
            }
            Err(AltProblem::Blocked) => blocked += 1,
            Err(AltProblem::Unfit(why)) => reasons.push(why),
        }
    }
    if step.actions.is_empty() {
        Err("it has no action".to_string())
    } else if blocked == step.actions.len() {
        Err("every alternative is blocked".to_string())
    } else {
        Err(format!("no alternative fits ({})", reasons.join("; ")))
    }
}

/// The first candidate in file order, or every step's reason (`` `id`:
/// why ``).
pub(crate) fn next_step(
    ctx: &Ctx,
    st: &State,
    steps: &[Spanned<Step>],
    bindings: &Bindings,
) -> Result<Pick, Vec<String>> {
    let slots = Slots::new(st, &ctx.targets);
    let mut reasons = Vec::new();
    for index in 0..steps.len() {
        match evaluate(ctx, st, &slots, steps, index, bindings) {
            Ok(pick) => return Ok(pick),
            Err(why) => reasons.push(format!("`{}`: {why}", steps[index].get_ref().id.get_ref())),
        }
    }
    Err(reasons)
}

/// The step as the player sees it. On an item the projection changed, the
/// odds are read off it and say "if the earlier steps hit". Taken in place
/// of the last-target desecration, it carries why.
pub(crate) fn view(
    ctx: &Ctx,
    st: &State,
    steps: &[Spanned<Step>],
    bindings: &Bindings,
    pick: Pick,
) -> StepView {
    let step = steps[pick.index].get_ref();
    let goal = *step.goal.get_ref();
    let odds = step_odds(ctx, st, goal, &pick.aims, &pick.action).map(|o| {
        if st.projected {
            with_basis_note(o, "if the earlier steps hit")
        } else {
            o
        }
    });
    let instead_of_desecration = desecration_passed_over(ctx, st, steps, bindings, pick.index);
    StepView {
        id: step.id.get_ref().clone(),
        say: step.say.get_ref().clone(),
        goal,
        side: pick.side.map(side_of),
        binding: binding_of(step),
        target: pick.target.map(|t| ctx.targets[t].mod_id.clone()),
        targets: pick
            .aims
            .iter()
            .map(|&t| ctx.targets[t].mod_id.clone())
            .collect(),
        action: Some(pick.action),
        odds,
        instead_of_desecration,
    }
}

fn set_rarity(ctx: &Ctx, st: &mut State, rarity: BpRarity) {
    st.rarity = Some(rarity);
    if let Some(limits) = ch_data::item_affix_limits(ctx.item, &item_rarity(rarity)) {
        st.limits = limits;
    }
}

/// A modifier the projection adds: a target (at its floor, level unknown),
/// a filler, or an unrevealed desecrated one.
fn projected_mod(side: Kind, role: Role) -> ModInfo {
    ModInfo {
        index: PROJECTED,
        side,
        fractured: false,
        desecrated: false,
        unrevealed: false,
        mod_id: None,
        tier: None,
        family: Vec::new(),
        texts: Vec::new(),
        templates: Vec::new(),
        role,
        level: None,
        filler: false,
        waits_for: None,
    }
}

fn place(ctx: &Ctx, st: &mut State, t: usize) {
    let target = &ctx.targets[t];
    st.mods.push(ModInfo {
        mod_id: Some(target.mod_id.clone()),
        tier: Some(target.floor),
        family: target.family.clone(),
        ..projected_mod(target.side, Role::Wanted(t))
    });
}

/// The position in `st.mods` of the modifier a removal takes if it hits:
/// the whittle's certain one; with Omen of Light the revealed desecrated
/// non-target; otherwise the first removable non-target on the resolved
/// side (both sides, prefix first, with none) in the item's order, fillers
/// last.
fn removal_takes(st: &State, pick: &Pick) -> Option<usize> {
    if is_whittle(&pick.action) {
        return whittle_certain(st, pick.side);
    }
    if takes(&pick.action) == Takes::Desecrated {
        return st.mods.iter().position(|m| {
            m.desecrated && !m.unrevealed && !m.fractured && !matches!(m.role, Role::Wanted(_))
        });
    }
    let sides: Vec<Kind> = pick.side.map_or(SIDES.to_vec(), |s| vec![s]);
    for side in sides {
        let on_side = |filler: bool| {
            st.mods
                .iter()
                .position(|m| m.side == side && m.filler == filler && removable(m))
        };
        if let Some(i) = on_side(false).or_else(|| on_side(true)) {
            return Some(i);
        }
    }
    None
}

/// The item as the step's hit would leave it (the spec's "Projection",
/// step 3).
fn apply_hit(
    ctx: &Ctx,
    st: &State,
    steps: &[Spanned<Step>],
    pick: &Pick,
    bindings: &Bindings,
) -> State {
    let step = steps[pick.index].get_ref();
    let action = &pick.action;
    let mut new = st.clone();
    new.projected = true;
    let count = if has_omen_change(action, OmenChange::AddTwo) {
        2
    } else {
        1
    };
    match action.effect {
        Effect::Upgrade if action.data_backed => {
            set_rarity(ctx, &mut new, BpRarity::Rare);
            if let Some(t) = pick.target {
                place(ctx, &mut new, t);
            }
        }
        Effect::Add | Effect::Augment | Effect::Transmute => {
            if action.effect == Effect::Transmute {
                set_rarity(ctx, &mut new, BpRarity::Magic);
            }
            let lands = |st: &State| -> Vec<Kind> {
                add_sides(ctx, st, action)
                    .into_iter()
                    .filter(|s| pick.side.is_none_or(|r| r == *s))
                    .collect()
            };
            for _ in 0..count {
                let landing = lands(&new);
                if *step.goal.get_ref() == Goal::Fill {
                    let Some(&side) = landing.first() else {
                        break;
                    };
                    new.mods.push(ModInfo {
                        filler: true,
                        ..projected_mod(side, Role::Unwanted)
                    });
                } else {
                    let can = placeable(ctx, &new, step, steps, bindings);
                    let Some(&t) = can
                        .iter()
                        .find(|&&t| landing.contains(&ctx.targets[t].side))
                    else {
                        break;
                    };
                    place(ctx, &mut new, t);
                }
            }
        }
        Effect::Desecrate => {
            if let Some(side) = pick.side {
                new.mods.push(ModInfo {
                    desecrated: true,
                    unrevealed: true,
                    waits_for: pick.target,
                    ..projected_mod(side, Role::Unrevealed)
                });
            }
        }
        Effect::Reveal => {
            if let Some(m) = new.mods.iter_mut().find(|m| m.unrevealed) {
                if let Some(t) = m.waits_for.or(pick.target) {
                    let target = &ctx.targets[t];
                    m.unrevealed = false;
                    m.waits_for = None;
                    m.level = None;
                    m.role = Role::Wanted(t);
                    m.mod_id = Some(target.mod_id.clone());
                    m.tier = Some(target.floor);
                    m.family = target.family.clone();
                }
            }
        }
        _ => {
            if let Some(i) = removal_takes(st, pick) {
                new.mods.remove(i);
            }
        }
    }
    new
}

/// Ties each unrevealed modifier on the item to the missing target a
/// desecration of the route is bound to (`hardest` or `last-remaining`, in
/// file order), when that target belongs on its side.
pub(crate) fn mark_waiting(
    ctx: &Ctx,
    st: &mut State,
    steps: &[Spanned<Step>],
    bindings: &Bindings,
) {
    let missing = st.missing(ctx.targets.len());
    let desecration = steps
        .iter()
        .map(|s| s.get_ref())
        .filter(|s| *s.goal.get_ref() == Goal::Desecrate)
        .filter_map(|s| step_target(ctx, s, bindings))
        .find(|t| missing.contains(t));
    for m in st
        .mods
        .iter_mut()
        .filter(|m| m.unrevealed && m.waits_for.is_none())
    {
        m.waits_for = desecration.filter(|&t| ctx.targets[t].side == m.side);
    }
}

/// The route preview from the item as it stands (the spec's
/// "Projection"), or the rejection when its hit path cannot finish.
pub(crate) fn project(
    ctx: &Ctx,
    steps: &[Spanned<Step>],
    item: &State,
) -> Result<Projection, Rejection> {
    let mut bindings = Bindings::new();
    let mut st = item.clone();
    let mut views: Vec<StepView> = Vec::new();
    let mut first_reasons: Option<Vec<String>> = None;
    loop {
        ensure_bindings(ctx, &st, steps, &mut bindings);
        mark_waiting(ctx, &mut st, steps, &bindings);
        if st.missing(ctx.targets.len()).is_empty() {
            break;
        }
        if views.len() >= STEP_CAP {
            return Err(Rejection::RouteCannotFinish {
                why: Stall::StepCap,
                step: None,
                reasons: Vec::new(),
            });
        }
        if first_reasons.is_none() {
            let slots = Slots::new(&st, &ctx.targets);
            first_reasons = Some(
                (0..steps.len())
                    .map(|i| match evaluate(ctx, &st, &slots, steps, i, &bindings) {
                        Ok(_) => "not needed on the hit path".to_string(),
                        Err(why) => why,
                    })
                    .collect(),
            );
        }
        let pick = next_step(ctx, &st, steps, &bindings).map_err(|reasons| {
            Rejection::RouteCannotFinish {
                why: Stall::NoCandidate,
                step: None,
                reasons,
            }
        })?;
        let after = apply_hit(ctx, &st, steps, &pick, &bindings);
        if after.mods == st.mods && after.rarity == st.rarity {
            return Err(Rejection::RouteCannotFinish {
                why: Stall::NoChange,
                step: Some(steps[pick.index].get_ref().id.get_ref().clone()),
                reasons: Vec::new(),
            });
        }
        views.push(view(ctx, &st, steps, &bindings, pick));
        st = after;
    }
    let skipped = steps
        .iter()
        .enumerate()
        .filter(|(_, s)| !views.iter().any(|v| v.id == *s.get_ref().id.get_ref()))
        .map(|(i, s)| Skipped {
            step: s.get_ref().id.get_ref().clone(),
            why: first_reasons
                .as_ref()
                .map_or_else(|| "no target is missing".to_string(), |r| r[i].clone()),
        })
        .collect();
    Ok(Projection {
        steps: views,
        skipped,
        bindings,
    })
}
