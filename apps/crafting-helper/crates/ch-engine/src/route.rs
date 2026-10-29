//! Route choice: for a parsed item, its targets and the blocked ids, which
//! blueprints apply, and for each one the steps with their bound target,
//! chosen action and first-try odds. Not an optimiser: no cost, no
//! expected number of tries.

use std::collections::BTreeMap;
use std::fmt;

use ch_blueprint::catalogue::{self, bones_for, Currency, Effect, ItemGroup, SIDE_OMEN};
use ch_blueprint::{Action, Binding, Goal, Rarity as BpRarity, Side, Spanned, Step};
use ch_data::{Kind, Odds, Section};
use ch_item::Item;
use serde::Serialize;

use crate::catalogue::data_entries;
use crate::context::{
    condition_failures, kind_of, side_of, side_word, Ctx, Refusal, State, Target, TargetInfo,
};
use crate::engine::{Engine, Resolved};
use crate::odds::{step_odds, target_difficulty};

/// The way a step is taken: the first alternative none of whose ids is
/// blocked, with its placeholders resolved.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ChosenAction {
    /// Its position among the step's `[[step.action]]`s, from 0.
    pub alternative: usize,
    /// The currency as the blueprint writes it (`desecrate` included).
    pub currency: String,
    /// What to use: the currency id, or the bone `desecrate` resolved to.
    pub uses: String,
    /// Its English name.
    pub name: String,
    /// The omen ids, `side-omen` resolved.
    pub omens: Vec<String>,
    pub omen_names: Vec<String>,
    pub effect: Effect,
    /// The side a Sinistral or Dextral omen restricts the change to.
    pub side: Option<Side>,
    pub min_modifier_level: Option<u32>,
    /// An essence, alloy or liquid emotion.
    pub data_backed: bool,
    /// It removes a modifier before adding its own.
    pub removes: bool,
}

/// One step as the player sees it, on a given item.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StepView {
    pub id: String,
    pub say: String,
    pub goal: Goal,
    pub side: Option<Side>,
    pub binding: Option<Binding>,
    /// The bound target, when the binding names one.
    pub target: Option<String>,
    /// The targets a hit may produce: the bound one, or every remaining
    /// one this step can place.
    pub targets: Vec<String>,
    /// `None` for a manual step with no action.
    pub action: Option<ChosenAction>,
    /// The first try's odds; `None` for a manual step.
    pub odds: Option<Odds>,
}

/// A step left out of the route on this item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Skipped {
    pub step: String,
    pub why: String,
}

/// One applicable blueprint, planned on the item.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Route {
    pub blueprint: String,
    pub name: String,
    pub source: String,
    pub notes: Option<String>,
    pub classes: Vec<String>,
    /// The `[[start]]` alternative the item met, from 0.
    pub start: usize,
    /// The steps taken, in blueprint order.
    pub steps: Vec<StepView>,
    pub skipped: Vec<Skipped>,
    /// Step id to the target an `essence` or `hardest` binding chose.
    pub bindings: BTreeMap<String, String>,
}

/// Why a blueprint does not apply to the item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Rejection {
    /// The blueprint does not name the item's class.
    ClassNotCovered { class: String },
    /// No `[[start]]` alternative holds: what each one failed.
    NoStartFits { failed: Vec<Vec<String>> },
    /// Every alternative of a step names a blocked id.
    EveryAlternativeBlocked { step: String },
    /// No alternative of a step can be used on the item.
    NoAlternativeFits { step: String, reasons: Vec<String> },
    /// A step's binding cannot be made.
    CannotBind { step: String, why: String },
    /// A `desecrated`-section target no desecrate or reveal step can place.
    DesecratedTargetUnplaced { target: String },
}

impl fmt::Display for Rejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Rejection::ClassNotCovered { class } => write!(f, "not a blueprint for {class}"),
            Rejection::NoStartFits { failed } => {
                let parts: Vec<String> = failed.iter().map(|f| f.join("; ")).collect();
                write!(f, "no start fits the item ({})", parts.join(" | "))
            }
            Rejection::EveryAlternativeBlocked { step } => {
                write!(f, "step `{step}`: every alternative is blocked")
            }
            Rejection::NoAlternativeFits { step, reasons } => {
                write!(
                    f,
                    "step `{step}`: no alternative fits the item ({})",
                    reasons.join("; ")
                )
            }
            Rejection::CannotBind { step, why } => write!(f, "step `{step}`: {why}"),
            Rejection::DesecratedTargetUnplaced { target } => write!(
                f,
                "`{target}` is a desecrated modifier and no desecrate step can place it"
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Rejected {
    pub blueprint: String,
    pub reason: Rejection,
}

/// Every blueprint's verdict on one item.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Plan {
    pub class: String,
    pub variant: String,
    pub item_level: u32,
    pub targets: Vec<TargetInfo>,
    /// Class-specific blueprints first, then by load order.
    pub routes: Vec<Route>,
    pub rejected: Vec<Rejected>,
}

impl Engine {
    /// Plans every blueprint on the item. Refuses the item up front
    /// (class, variant, corrupted, mirrored, unidentified, short form,
    /// rarity, item level, a target not valid for it).
    pub fn plan(
        &self,
        item: &Item,
        targets: &[Target],
        blocked: &[String],
    ) -> Result<Plan, Refusal> {
        let ctx = Ctx::new(item, targets, blocked)?;
        let mut routes: Vec<(usize, usize, Route)> = Vec::new();
        let mut rejected = Vec::new();
        for (order, resolved) in self.blueprints().iter().enumerate() {
            match route_for(&ctx, resolved) {
                Ok(route) => routes.push((route.classes.len(), order, route)),
                Err(reason) => rejected.push(Rejected {
                    blueprint: resolved.id().to_string(),
                    reason,
                }),
            }
        }
        routes.sort_by_key(|(classes, order, _)| (*classes, *order));
        Ok(Plan {
            class: ctx.data.class.clone(),
            variant: ctx.variant.to_string(),
            item_level: ctx.item_level,
            targets: ctx.targets.clone(),
            routes: routes.into_iter().map(|(_, _, r)| r).collect(),
            rejected,
        })
    }
}

/// Plans one blueprint on the item.
pub(crate) fn route_for(ctx: &Ctx, resolved: &Resolved) -> Result<Route, Rejection> {
    let b = resolved.blueprint();
    let classes: Vec<String> = b
        .classes
        .get_ref()
        .iter()
        .map(|c| c.get_ref().clone())
        .collect();
    if !classes.contains(&ctx.data.class) {
        return Err(Rejection::ClassNotCovered {
            class: ctx.data.class.clone(),
        });
    }
    let st = State::new(ctx.item, &ctx.targets);
    let binds = || essence_binds_any(ctx, &st, true);
    let mut failed = Vec::new();
    let mut start = None;
    for (i, s) in b.starts.iter().enumerate() {
        let f = condition_failures(s.get_ref(), &st, &ctx.targets, &binds);
        if f.is_empty() {
            start = Some(i);
            break;
        }
        failed.push(f);
    }
    let Some(start) = start else {
        return Err(Rejection::NoStartFits { failed });
    };

    let mut bindings = BTreeMap::new();
    let mut steps = Vec::new();
    let mut skipped = Vec::new();
    for step in &b.steps {
        match pick_step(ctx, &st, step.get_ref(), &b.steps, &mut bindings) {
            StepPick::Skip(why) => skipped.push(Skipped {
                step: step.get_ref().id.get_ref().clone(),
                why,
            }),
            StepPick::Take(view) => steps.push(*view),
            StepPick::Reject(r) => return Err(r),
        }
    }

    // A desecrated-section target only desecration places: some desecrate
    // or reveal step must be bound to it, or take whatever remains.
    for t in st.missing(ctx.targets.len()) {
        let target = &ctx.targets[t];
        if target.section != Section::Desecrated {
            continue;
        }
        let bound = bindings.values().any(|m| *m == target.mod_id);
        let open = b.steps.iter().any(|s| {
            let s = s.get_ref();
            matches!(s.goal.get_ref(), Goal::Desecrate | Goal::Reveal)
                && !matches!(
                    s.bind.as_ref().map(|b| *b.get_ref()),
                    Some(Binding::Essence | Binding::Hardest)
                )
        });
        if !bound && !open {
            return Err(Rejection::DesecratedTargetUnplaced {
                target: target.mod_id.clone(),
            });
        }
    }

    Ok(Route {
        blueprint: resolved.id().to_string(),
        name: b.name.get_ref().clone(),
        source: b.source.get_ref().clone(),
        notes: b.notes.as_ref().map(|n| n.get_ref().clone()),
        classes,
        start,
        steps,
        skipped,
        bindings,
    })
}

/// What a step is on the item.
pub(crate) enum StepPick {
    Skip(String),
    Take(Box<StepView>),
    Reject(Rejection),
}

/// Resolves one step on the item: its binding (an `essence` or `hardest`
/// one is kept in `bindings`, so later items keep the same target), whether
/// it is skipped, its action and its odds.
pub(crate) fn pick_step(
    ctx: &Ctx,
    st: &State,
    step: &Step,
    steps: &[Spanned<Step>],
    bindings: &mut BTreeMap<String, String>,
) -> StepPick {
    let id = step.id.get_ref().clone();
    let goal = *step.goal.get_ref();
    let binding = step.bind.as_ref().map(|b| *b.get_ref());
    let eligible = eligible_missing(ctx, st, goal);
    let bound = bind(ctx, st, step, steps, binding, &eligible, bindings);
    if let (Err(why), Some(_)) = (&bound, &step.when) {
        return StepPick::Skip(format!("its binding did not resolve: {why}"));
    }
    let target = bound.as_ref().ok().copied().flatten();
    if let Some(why) = goal_met(ctx, st, step, goal, target, &eligible) {
        return StepPick::Skip(why);
    }
    if let Err(why) = bound {
        return StepPick::Reject(Rejection::CannotBind { step: id, why });
    }
    let action = if step.actions.is_empty() {
        None
    } else {
        let essence = (binding == Some(Binding::Essence))
            .then_some(target)
            .flatten();
        match choose_action(ctx, st, step, target, essence) {
            Ok(a) => Some(a),
            Err(ActionProblem::AllBlocked) => {
                return StepPick::Reject(Rejection::EveryAlternativeBlocked { step: id })
            }
            Err(ActionProblem::NoneFits(reasons)) => {
                return StepPick::Reject(Rejection::NoAlternativeFits { step: id, reasons })
            }
        }
    };
    if let (Some(t), Some(Binding::Essence | Binding::Hardest)) = (target, binding) {
        bindings.insert(id.clone(), ctx.targets[t].mod_id.clone());
    }
    let aims: Vec<usize> = match target {
        Some(t) => vec![t],
        None => eligible,
    };
    let odds = action
        .as_ref()
        .and_then(|a| step_odds(ctx, st, goal, &aims, a));
    StepPick::Take(Box::new(StepView {
        id,
        say: step.say.get_ref().clone(),
        goal,
        side: step.side.as_ref().map(|s| *s.get_ref()),
        binding,
        target: target.map(|t| ctx.targets[t].mod_id.clone()),
        targets: aims
            .iter()
            .map(|&t| ctx.targets[t].mod_id.clone())
            .collect(),
        action,
        odds,
    }))
}

/// The missing targets a step with this goal may place: every one for a
/// desecrate or reveal step, the `normal`-section ones otherwise.
fn eligible_missing(ctx: &Ctx, st: &State, goal: Goal) -> Vec<usize> {
    let desecration = matches!(goal, Goal::Desecrate | Goal::Reveal);
    st.missing(ctx.targets.len())
        .into_iter()
        .filter(|&t| desecration || ctx.targets[t].section == Section::Normal)
        .collect()
}

/// The target a step's binding names: `Ok(None)` when it names none (no
/// binding, or `any-remaining`), `Err` when it cannot be made.
fn bind(
    ctx: &Ctx,
    st: &State,
    step: &Step,
    steps: &[Spanned<Step>],
    binding: Option<Binding>,
    eligible: &[usize],
    bindings: &BTreeMap<String, String>,
) -> Result<Option<usize>, String> {
    let Some(binding) = binding else {
        return Ok(None);
    };
    let id = step.id.get_ref();
    if let Some(t) = bindings.get(id).and_then(|m| ctx.target_index(m)) {
        return Ok(Some(t));
    }
    let claimed: Vec<usize> = bindings
        .iter()
        .filter(|(s, _)| *s != id)
        .filter_map(|(_, m)| ctx.target_index(m))
        .collect();
    let free: Vec<usize> = eligible
        .iter()
        .copied()
        .filter(|t| !claimed.contains(t))
        .collect();
    let goal = *step.goal.get_ref();
    match binding {
        Binding::Essence => essence_binding(ctx, st, &step.actions, &free)
            .map(Some)
            .ok_or_else(|| {
                "no unblocked essence of the step adds a remaining target at its floor".to_string()
            }),
        Binding::Hardest => {
            if goal == Goal::Reveal {
                // The modifier being revealed is the one the desecrate step
                // aimed at.
                let position = steps.iter().position(|s| s.get_ref().id.get_ref() == id);
                let earlier = steps[..position.unwrap_or(steps.len())]
                    .iter()
                    .rev()
                    .filter(|s| *s.get_ref().goal.get_ref() == Goal::Desecrate)
                    .find_map(|s| bindings.get(s.get_ref().id.get_ref()));
                if let Some(t) = earlier.and_then(|m| ctx.target_index(m)) {
                    return Ok(Some(t));
                }
            }
            if matches!(goal, Goal::Desecrate | Goal::Reveal) {
                if let Some(&t) = free
                    .iter()
                    .find(|&&t| ctx.targets[t].section == Section::Desecrated)
                {
                    return Ok(Some(t));
                }
            }
            hardest(ctx, st, &free)
                .map(Some)
                .ok_or_else(|| "no target remains to bind".to_string())
        }
        Binding::AnyRemaining => {
            if eligible.is_empty() {
                Err("no target remains".to_string())
            } else {
                Ok(None)
            }
        }
        // One target left: that one; several: whichever comes, as
        // `any-remaining`.
        Binding::LastRemaining => match eligible {
            [only] => Ok(Some(*only)),
            [] => Err("no target remains".to_string()),
            _ => Ok(None),
        },
    }
}

/// The target with the lowest add odds; unknown odds count as hardest,
/// ties go to target order.
fn hardest(ctx: &Ctx, st: &State, candidates: &[usize]) -> Option<usize> {
    let mut best: Option<(usize, Option<f64>)> = None;
    for &t in candidates {
        let p = target_difficulty(ctx, st, t);
        let harder = match (&best, p) {
            (None, _) => true,
            (Some((_, Some(_))), None) => true,
            (Some((_, Some(b))), Some(a)) => a < *b,
            (Some((_, None)), _) => false,
        };
        if harder {
            best = Some((t, p));
        }
    }
    best.map(|(t, _)| t)
}

/// Whether a data-backed essence's grade fits the item's rarity: Lesser,
/// plain and Greater make a Magic item Rare; Perfect, the special ones,
/// alloys and liquid emotions work on a Rare one. `lenient` lets a Normal
/// item count as Magic (a start condition, before the blueprint's own
/// transmute).
fn essence_fits_rarity(effect: Effect, rarity: Option<BpRarity>, lenient: bool) -> bool {
    match effect {
        Effect::Upgrade => {
            rarity == Some(BpRarity::Magic) || (lenient && rarity == Some(BpRarity::Normal))
        }
        Effect::Swap => rarity == Some(BpRarity::Rare),
        _ => false,
    }
}

/// The tier at which one of `entries` adds target `t` on the item, at its
/// floor or better. An upgrading essence cannot add a modifier whose family
/// is already on the item.
fn essence_adds(
    ctx: &Ctx,
    st: &State,
    entries: &[crate::catalogue::DataEntry],
    effect: Effect,
    t: usize,
) -> Option<u32> {
    let target = &ctx.targets[t];
    if target.section != Section::Normal {
        return None;
    }
    if effect == Effect::Upgrade
        && st
            .mods
            .iter()
            .any(|m| m.family.iter().any(|f| target.family.contains(f)))
    {
        return None;
    }
    entries.iter().find_map(|e| {
        let m = e.essence.matches.get(ctx.variant)?.as_ref()?;
        (m.mod_id == target.mod_id && m.tier <= target.floor).then_some(m.tier)
    })
}

/// The first target (in the actions' preference order) an unblocked
/// essence among `actions` adds at its floor.
fn essence_binding(
    ctx: &Ctx,
    st: &State,
    actions: &[Spanned<Action>],
    free: &[usize],
) -> Option<usize> {
    for action in actions {
        let action = action.get_ref();
        let id = action.currency.get_ref().as_str();
        let Some(Currency::DataBacked(shape)) = Currency::of(id) else {
            continue;
        };
        let omen_blocked = action
            .omens
            .iter()
            .any(|o| o.get_ref() != SIDE_OMEN && ctx.is_blocked(o.get_ref()));
        if ctx.is_blocked(id)
            || omen_blocked
            || !essence_fits_rarity(shape.effect, st.rarity, false)
        {
            continue;
        }
        let entries = data_entries(ctx.data, id);
        if let Some(&t) = free
            .iter()
            .find(|&&t| essence_adds(ctx, st, &entries, shape.effect, t).is_some())
        {
            return Some(t);
        }
    }
    None
}

/// Whether any unblocked essence or liquid emotion of the class adds a
/// remaining target at its floor: a start's `essence_binds`.
pub(crate) fn essence_binds_any(ctx: &Ctx, st: &State, lenient: bool) -> bool {
    let missing: Vec<usize> = eligible_missing(ctx, st, Goal::AddWanted);
    let all = ctx.data.essences.iter().chain(ctx.data.liquid.iter());
    for essence in all {
        let id = catalogue::id_from_name(&essence.name);
        if ctx.is_blocked(&id) {
            continue;
        }
        let Some(Currency::DataBacked(shape)) = Currency::of(&id) else {
            continue;
        };
        if !essence_fits_rarity(shape.effect, st.rarity, lenient) {
            continue;
        }
        let entry = [crate::catalogue::DataEntry { essence }];
        if missing
            .iter()
            .any(|&t| essence_adds(ctx, st, &entry, shape.effect, t).is_some())
        {
            return true;
        }
    }
    false
}

/// Why the step has nothing to do on this item, if it has not.
fn goal_met(
    ctx: &Ctx,
    st: &State,
    step: &Step,
    goal: Goal,
    target: Option<usize>,
    eligible: &[usize],
) -> Option<String> {
    let side = step.side.as_ref().map(|s| kind_of(*s.get_ref()));
    let target_present = target.filter(|&t| st.present(t)).map(|t| {
        format!(
            "`{}` is already on the item at its floor",
            ctx.targets[t].mod_id
        )
    });
    match goal {
        Goal::AddWanted => match target {
            Some(_) => target_present,
            None if eligible.is_empty() => Some("no target this step can add remains".to_string()),
            None => None,
        },
        Goal::Convert | Goal::Reveal => target_present,
        Goal::Desecrate => target_present.or_else(|| {
            st.has_unrevealed()
                .then(|| "an unrevealed desecrated modifier is already on the item".to_string())
        }),
        Goal::RemoveUnwanted => {
            let any = st
                .mods
                .iter()
                .any(|m| m.is_unwanted() && !m.fractured && side.is_none_or(|s| m.side == s));
            (!any).then(|| "no unwanted modifier to remove".to_string())
        }
        Goal::RemoveDesecrated => {
            (!st.has_desecrated()).then(|| "no desecrated modifier".to_string())
        }
        Goal::Fill => {
            let free = [Kind::Prefix, Kind::Suffix]
                .into_iter()
                .filter(|k| side.is_none_or(|s| s == *k))
                .any(|k| st.free(k) > 0);
            (!free).then(|| "no free slot".to_string())
        }
        Goal::Manual => None,
    }
}

enum ActionProblem {
    AllBlocked,
    NoneFits(Vec<String>),
}

enum AltProblem {
    Blocked,
    Unfit(String),
}

/// The first alternative none of whose ids (placeholders resolved) is
/// blocked and that fits the item.
fn choose_action(
    ctx: &Ctx,
    st: &State,
    step: &Step,
    target: Option<usize>,
    essence_for: Option<usize>,
) -> Result<ChosenAction, ActionProblem> {
    let mut blocked = 0;
    let mut reasons = Vec::new();
    for (i, action) in step.actions.iter().enumerate() {
        match try_alternative(ctx, st, step, i, action.get_ref(), target, essence_for) {
            Ok(chosen) => return Ok(chosen),
            Err(AltProblem::Blocked) => blocked += 1,
            Err(AltProblem::Unfit(why)) => reasons.push(why),
        }
    }
    if blocked == step.actions.len() {
        Err(ActionProblem::AllBlocked)
    } else {
        Err(ActionProblem::NoneFits(reasons))
    }
}

fn try_alternative(
    ctx: &Ctx,
    st: &State,
    step: &Step,
    alternative: usize,
    action: &Action,
    target: Option<usize>,
    essence_for: Option<usize>,
) -> Result<ChosenAction, AltProblem> {
    let written = action.currency.get_ref().as_str();
    let Some(currency) = Currency::of(written) else {
        return Err(AltProblem::Unfit(format!("`{written}` is not a currency")));
    };
    let class = ctx.data.class.as_str();
    let group = ItemGroup::of_class(class);
    let (uses, name, effect, mml, data_backed, removes) = match currency {
        Currency::Static(e) => {
            if ctx.is_blocked(e.id) {
                return Err(AltProblem::Blocked);
            }
            if essence_for.is_some() {
                return Err(AltProblem::Unfit(format!("`{written}` is not an essence")));
            }
            if !e.groups.is_empty() && !group.is_some_and(|g| e.groups.contains(&g)) {
                return Err(AltProblem::Unfit(format!(
                    "`{written}` is not used on {class}"
                )));
            }
            if e.max_item_level.is_some_and(|max| ctx.item_level > max) {
                return Err(AltProblem::Unfit(format!(
                    "`{written}` is for item level {} or lower",
                    e.max_item_level.unwrap_or_default()
                )));
            }
            (
                e.id.to_string(),
                e.name.to_string(),
                e.effect,
                e.min_modifier_level,
                false,
                false,
            )
        }
        Currency::DataBacked(shape) => {
            if ctx.is_blocked(written) {
                return Err(AltProblem::Blocked);
            }
            let entries = data_entries(ctx.data, written);
            let Some(first) = entries.first() else {
                return Err(AltProblem::Unfit(format!("no `{written}` for {class}")));
            };
            if let Some(t) = essence_for {
                if !essence_fits_rarity(shape.effect, st.rarity, false)
                    || essence_adds(ctx, st, &entries, shape.effect, t).is_none()
                {
                    return Err(AltProblem::Unfit(format!(
                        "`{written}` does not add `{}` at its floor here",
                        ctx.targets[t].mod_id
                    )));
                }
            }
            let removes = shape.effect == Effect::Swap
                || entries.iter().any(|e| e.essence.removes == Some(true));
            (
                written.to_string(),
                first.essence.name.clone(),
                shape.effect,
                None,
                true,
                removes,
            )
        }
        Currency::Desecrate => {
            let Some(group) = group else {
                return Err(AltProblem::Unfit(format!("no bone desecrates {class}")));
            };
            let bones = bones_for(group, ctx.item_level);
            if bones.is_empty() {
                return Err(AltProblem::Unfit(format!(
                    "no bone fits {class} at this item level"
                )));
            }
            let Some(bone) = bones.iter().find(|b| !ctx.is_blocked(b.id)) else {
                return Err(AltProblem::Blocked);
            };
            (
                bone.id.to_string(),
                bone.name.to_string(),
                Effect::Desecrate,
                bone.min_modifier_level,
                false,
                false,
            )
        }
        Currency::Reveal => (
            catalogue::REVEAL.to_string(),
            "Reveal at the Well of Souls".to_string(),
            Effect::Reveal,
            None,
            false,
            false,
        ),
    };

    let side = target
        .map(|t| side_of(ctx.targets[t].side))
        .or_else(|| step.side.as_ref().map(|s| *s.get_ref()));
    let mut omens = Vec::new();
    let mut omen_names = Vec::new();
    let mut omen_side = None;
    for omen in &action.omens {
        let id = omen.get_ref().as_str();
        let entry = if id == SIDE_OMEN {
            let Some(s) = side else {
                return Err(AltProblem::Unfit(
                    "`side-omen` with no bound target or side".to_string(),
                ));
            };
            currency.side_omen(s).ok_or_else(|| {
                AltProblem::Unfit(format!("`{written}` has no {} omen", side_word(kind_of(s))))
            })?
        } else {
            catalogue::entry(id).ok_or_else(|| AltProblem::Unfit(format!("unknown omen `{id}`")))?
        };
        if ctx.is_blocked(entry.id) {
            return Err(AltProblem::Blocked);
        }
        if let Some(s) = entry.omen.and_then(|o| o.side) {
            omen_side = Some(s);
        }
        omens.push(entry.id.to_string());
        omen_names.push(entry.name.to_string());
    }

    Ok(ChosenAction {
        alternative,
        currency: written.to_string(),
        uses,
        name,
        omens,
        omen_names,
        effect,
        side: omen_side,
        min_modifier_level: mml,
        data_backed,
        removes,
    })
}
