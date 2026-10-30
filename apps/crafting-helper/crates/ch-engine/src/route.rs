//! Route choice: for a parsed item, its targets and the blocked ids, which
//! blueprints apply, and for each one the steps with their bound target,
//! chosen action and first-try odds. Not an optimiser: no cost, no
//! expected number of tries.

use std::collections::BTreeMap;
use std::fmt;

use ch_blueprint::catalogue::{
    self, bones_for, Currency, Effect, Entry, ItemGroup, GREATER_EXALTATION, GREATER_OMEN,
    LORD_CLASSES, LORD_OMEN, LORD_OMENS, SIDE_OMEN,
};
use ch_blueprint::{Action, Binding, Goal, Order, Rarity as BpRarity, Side, Spanned, Step, When};
use ch_data::{Kind, Odds, Section};
use ch_item::{Item, Rarity};
use serde::Serialize;

use crate::catalogue::data_entries;
use crate::context::{
    condition_failures, kind_of, side_of, side_word, Ctx, Refusal, Role, State, Target, TargetInfo,
};
use crate::engine::{Engine, Resolved};
use crate::odds::{step_odds, target_difficulty};
use crate::slots::SIDES;
use crate::state_order;
use crate::swap::EssenceSwap;

/// The jewel class's name in the data.
const JEWELS: &str = "Jewels";

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
    /// Its `side-omen` waits for the last target's side: a bound-only
    /// `last-remaining` desecration planned while targets on both sides are
    /// still missing. No omen id stands for it in `omens`; the guide names
    /// the omen once one target is left.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub side_omen_waits: bool,
    /// Its `lord-omen` waits for the last target too, beside a waiting
    /// `side-omen` (docs/models/crafting-slots-spec.md, "`lord-omen`"):
    /// each remaining target that would take a lord omen, with that omen.
    /// Empty otherwise; no omen id stands for it in `omens`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub lord_omens_waiting: Vec<WaitingLordOmen>,
}

/// A lord omen a desecration names once `target` is the last one left.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WaitingLordOmen {
    /// The target's data modifier id.
    pub target: String,
    /// The omen id.
    pub omen: String,
    /// Its English name.
    pub name: String,
}

/// One step as the player sees it, on a given item.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StepView {
    pub id: String,
    pub say: String,
    pub goal: Goal,
    /// The side the blueprint names; in a state-ordered route, the side its
    /// side rule resolved to on the item.
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
    /// State order: why the last-target desecration was not taken, in
    /// words, when the item is rare with no desecrated modifier and one
    /// target missing and this step was taken in its place (no pool on the
    /// target's side, the bone or the Necromancy omen blocked, or the
    /// engine's own reason; docs/models/crafting-slots-spec.md, "The
    /// last-target desecration"). The app's Fallback line.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instead_of_desecration: Option<String>,
    /// State order: an essence swap's removal, what it may take and, when
    /// that includes an unrevealed desecrated modifier, the chance it
    /// survives and the step the route takes next if it stays and if it
    /// goes (docs/models/crafting-slots-spec.md, "The essence swap and an
    /// unrevealed modifier"). `None` on every other step, and in a listed
    /// route.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub swap: Option<EssenceSwap>,
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
    /// How the guide picks the next step: by transitions (`listed`) or from
    /// the item (`state`).
    pub order: Order,
    /// The `[[start]]` alternative the item met, from 0.
    pub start: usize,
    /// Listed order: the steps taken, in blueprint order. State order: the
    /// projection, the steps the craft takes if each one hits (a step may
    /// come more than once), each on the item as the earlier ones leave it.
    pub steps: Vec<StepView>,
    pub skipped: Vec<Skipped>,
    /// Step id to the target an `essence` or `hardest` binding chose.
    pub bindings: BTreeMap<String, String>,
}

/// Why a state-ordered route's hit path cannot finish.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Stall {
    /// A target is missing and no step is a candidate.
    NoCandidate,
    /// A step's hit would leave the item as it was.
    NoChange,
    /// The projection reached its step cap without finishing.
    StepCap,
}

impl fmt::Display for Stall {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Stall::NoCandidate => "no-candidate",
            Stall::NoChange => "no-change",
            Stall::StepCap => "step-cap",
        })
    }
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
    /// State order: a side's targets alone are more than its Rare cap.
    TargetsExceedSlots {
        side: Side,
        targets: usize,
        slots: usize,
    },
    /// State order: the projection does not reach an item with every
    /// target. `step` is the step concerned (the one that changes nothing);
    /// `reasons` says, for no candidate, why each step is not one.
    RouteCannotFinish {
        why: Stall,
        step: Option<String>,
        reasons: Vec<String>,
    },
    /// The item is Magic, its prefix is a target below the top two tiers
    /// its modifier can roll on this base at the item's level, and the
    /// route spends an upgrade (a Regal Orb or a Lesser, plain or Greater
    /// essence) on it: a new base is worth more
    /// (docs/models/crafting-slots-spec.md, "Rejections").
    WeakMagicStart {
        modifier: String,
        tier: u32,
        item_level: u32,
        /// The best tier, then the second best (one when only one tier is
        /// reachable).
        top: Vec<u32>,
    },
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
            Rejection::TargetsExceedSlots {
                side,
                targets,
                slots,
            } => write!(
                f,
                "{targets} {} targets, and a rare item has {slots} {} slots",
                side.name(),
                side.name()
            ),
            Rejection::RouteCannotFinish { why, step, reasons } => {
                match why {
                    Stall::NoCandidate => {
                        f.write_str("no step can be taken while a target is missing")?
                    }
                    Stall::NoChange => f.write_str("a step's hit would leave the item as it is")?,
                    Stall::StepCap => write!(
                        f,
                        "the route does not finish within {} steps",
                        state_order::STEP_CAP
                    )?,
                }
                if let Some(step) = step {
                    write!(f, " (step `{step}`)")?;
                }
                if !reasons.is_empty() {
                    write!(f, " ({})", reasons.join("; "))?;
                }
                Ok(())
            }
            Rejection::WeakMagicStart {
                modifier,
                tier,
                item_level,
                top,
            } => {
                let tops: Vec<String> = top.iter().map(|t| format!("T{t}")).collect();
                write!(
                    f,
                    "the magic item's prefix `{modifier}` is T{tier}, below its top tiers at \
                     item level {item_level} ({}): a new base is worth more than an upgrade",
                    tops.join(" and ")
                )
            }
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
    let route = |steps, skipped, bindings| Route {
        blueprint: resolved.id().to_string(),
        name: b.name.get_ref().clone(),
        source: b.source.get_ref().clone(),
        notes: b.notes.as_ref().map(|n| n.get_ref().clone()),
        classes: classes.clone(),
        order: b.order(),
        start,
        steps,
        skipped,
        bindings,
    };

    if b.order() == Order::State {
        // A side's targets alone must fit its Rare cap.
        if let Some(rare) = ch_data::item_affix_limits(ctx.item, &Rarity::Rare) {
            for side in SIDES {
                let targets = ctx.targets.iter().filter(|t| t.side == side).count();
                if targets > rare.on(side) {
                    return Err(Rejection::TargetsExceedSlots {
                        side: side_of(side),
                        targets,
                        slots: rare.on(side),
                    });
                }
            }
        }
        // Only a desecration places a desecrated-section target.
        let desecrates = b
            .steps
            .iter()
            .any(|s| *s.get_ref().goal.get_ref() == Goal::Desecrate);
        if let Some(t) = st
            .missing(ctx.targets.len())
            .into_iter()
            .find(|&t| ctx.targets[t].section == Section::Desecrated)
        {
            if !desecrates {
                return Err(Rejection::DesecratedTargetUnplaced {
                    target: ctx.targets[t].mod_id.clone(),
                });
            }
        }
        let projection = state_order::project(ctx, &b.steps, &st)?;
        if let Some(weak) = weak_magic_start(ctx, &st, &projection.steps) {
            return Err(weak);
        }
        return Ok(route(
            projection.steps,
            projection.skipped,
            projection.bindings,
        ));
    }

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
    if let Some(weak) = weak_magic_start(ctx, &st, &steps) {
        return Err(weak);
    }

    Ok(route(steps, skipped, bindings))
}

/// The `weak-magic-start` rejection (docs/models/crafting-slots-spec.md,
/// "Rejections"), checked last on a planned route: the item is Magic, its
/// prefix is a target at any tier, a step of the route makes it Rare (a
/// Regal Orb of any grade, or a Lesser, plain or Greater essence; only a
/// Magic item takes one), and the prefix's tier is worse than the second
/// best its modifier rolls on this base at the item's level. `None` when
/// any of these fails, or the prefix's tier is unknown.
fn weak_magic_start(ctx: &Ctx, st: &State, steps: &[StepView]) -> Option<Rejection> {
    if st.rarity != Some(BpRarity::Magic) {
        return None;
    }
    let upgrades = steps
        .iter()
        .filter_map(|s| s.action.as_ref())
        .any(|a| a.effect == Effect::Upgrade);
    if !upgrades {
        return None;
    }
    let prefix = st.mods.iter().find(|m| {
        m.side == Kind::Prefix && matches!(m.role, Role::Wanted(_) | Role::BelowFloor(_))
    })?;
    let modifier = prefix.mod_id.as_ref()?;
    let tier = prefix.tier?;
    let mut reachable: Vec<u32> = ctx
        .data
        .find_mod(modifier)?
        .tiers_on(ctx.variant)
        .iter()
        .filter(|t| t.ilvl <= ctx.item_level)
        .map(|t| t.tier)
        .collect();
    reachable.sort_unstable();
    reachable.dedup();
    let top: Vec<u32> = reachable.into_iter().take(2).collect();
    let second = *top.last()?;
    (tier > second).then(|| Rejection::WeakMagicStart {
        modifier: modifier.clone(),
        tier,
        item_level: ctx.item_level,
        top,
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
        let side = target
            .map(|t| side_of(ctx.targets[t].side))
            .or_else(|| step.side.as_ref().map(|s| *s.get_ref()));
        let placeable = target.map_or_else(|| eligible.clone(), |t| vec![t]);
        let greater = greater_fits(ctx, st, goal, side.map(kind_of), &placeable);
        let waits = target.is_none() && waits_for_last_target(step, binding);
        let omen_side = match side {
            Some(s) => OmenSide::Known(s),
            None if waits => shared_side(ctx, &eligible),
            None => OmenSide::Unknown,
        };
        let lord = match target {
            Some(t) => LordOmen::of_target(ctx, t),
            None if waits => LordOmen::waiting(ctx, &eligible),
            None => LordOmen::Nothing,
        };
        match choose_action(ctx, st, step, omen_side, greater, &lord, essence) {
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
        // A fracture's hit is a target already on the item.
        None if goal == Goal::Fracture => fracturable_targets(st),
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
        instead_of_desecration: None,
        swap: None,
    }))
}

/// The targets on the item a Fracturing Orb could fracture: present, not
/// fractured, not desecrated (revealed or not).
pub(crate) fn fracturable_targets(st: &State) -> Vec<usize> {
    st.mods
        .iter()
        .filter(|m| !m.fractured && !m.desecrated && !m.unrevealed)
        .filter_map(|m| match m.role {
            Role::Wanted(t) => Some(t),
            _ => None,
        })
        .collect()
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
pub(crate) fn hardest(ctx: &Ctx, st: &State, candidates: &[usize]) -> Option<usize> {
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
pub(crate) fn essence_fits_rarity(effect: Effect, rarity: Option<BpRarity>, lenient: bool) -> bool {
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
pub(crate) fn essence_adds(
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
        Goal::Fracture => {
            if st.mods.iter().any(|m| m.fractured) {
                Some("a fractured modifier is already on the item".to_string())
            } else if fracturable_targets(st).is_empty() {
                Some("no target is on the item to fracture".to_string())
            } else {
                None
            }
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

pub(crate) enum AltProblem {
    Blocked,
    Unfit(String),
}

/// Whether `greater-omen` resolves to Omen of Greater Exaltation
/// (docs/models/crafting-slots-spec.md, "`greater-omen`"): the step's side
/// (the item, with none) has two free slots, the step can use two (a fill
/// always; an add when two of `placeable` are on that side), and the omen
/// is not blocked.
pub(crate) fn greater_fits(
    ctx: &Ctx,
    st: &State,
    goal: Goal,
    side: Option<Kind>,
    placeable: &[usize],
) -> bool {
    if ctx.is_blocked(GREATER_EXALTATION) {
        return false;
    }
    let free = match side {
        Some(s) => st.free(s),
        None => st.free(Kind::Prefix) + st.free(Kind::Suffix),
    };
    if free < 2 {
        return false;
    }
    match goal {
        Goal::Fill => true,
        Goal::AddWanted => {
            placeable
                .iter()
                .filter(|&&t| side.is_none_or(|s| ctx.targets[t].side == s))
                .count()
                >= 2
        }
        _ => false,
    }
}

/// The side a `side-omen` resolves to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OmenSide {
    /// The bound target's, the step's `side`, or the side every target it
    /// may place shares.
    Known(Side),
    /// The last target's, not known yet: the targets it may place lie on
    /// both sides.
    WaitsForLastTarget,
    /// None: `side-omen` does not fit.
    Unknown,
}

/// A step whose `side-omen` may take its side from the targets it may
/// place while its binding names none: `when = "bound"` and
/// `bind = "last-remaining"`, no `side` (the whittle's desecration). The
/// guide picks such a step again once one target is left, and its omen is
/// then that target's. A `last-remaining` step without `when` (the rescue,
/// which requires one target missing) keeps an unbound `side-omen` unfit.
fn waits_for_last_target(step: &Step, binding: Option<Binding>) -> bool {
    binding == Some(Binding::LastRemaining)
        && step.when.as_ref().map(|w| *w.get_ref()) == Some(When::Bound)
        && step.side.is_none()
}

/// The side every target in `eligible` shares, or, with targets on both
/// sides, the last one's to come.
fn shared_side(ctx: &Ctx, eligible: &[usize]) -> OmenSide {
    let mut sides = eligible.iter().map(|&t| ctx.targets[t].side);
    match sides.next() {
        Some(first) if sides.all(|k| k == first) => OmenSide::Known(side_of(first)),
        Some(_) => OmenSide::WaitsForLastTarget,
        None => OmenSide::Unknown,
    }
}

/// What `lord-omen` resolves to on a step
/// (docs/models/crafting-slots-spec.md, "`lord-omen`").
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum LordOmen {
    /// Nothing: no omen, no fallback line.
    Nothing,
    /// The bound target's lord omen.
    Omen(&'static Entry),
    /// The last target's, not known yet: each remaining target that would
    /// take one, with its omen.
    Waits(Vec<WaitingLordOmen>),
}

impl LordOmen {
    /// For a desecration bound to target `t`: its lord's omen when `t` is
    /// from the desecrated pool and every tier of it on the item's variant
    /// names that one lord, the class is a lord class and the omen is not
    /// blocked; nothing otherwise.
    pub(crate) fn of_target(ctx: &Ctx, t: usize) -> Self {
        lord_omen_of(ctx, t).map_or(Self::Nothing, Self::Omen)
    }

    /// For a desecration waiting for the last of `eligible`: what each
    /// would take. Nothing when none would take one.
    fn waiting(ctx: &Ctx, eligible: &[usize]) -> Self {
        let each: Vec<WaitingLordOmen> = eligible
            .iter()
            .filter_map(|&t| {
                lord_omen_of(ctx, t).map(|e| WaitingLordOmen {
                    target: ctx.targets[t].mod_id.clone(),
                    omen: e.id.to_string(),
                    name: e.name.to_string(),
                })
            })
            .collect();
        if each.is_empty() {
            Self::Nothing
        } else {
            Self::Waits(each)
        }
    }
}

/// The lord omen a desecration bound to target `t` takes, or `None`.
fn lord_omen_of(ctx: &Ctx, t: usize) -> Option<&'static Entry> {
    let target = &ctx.targets[t];
    if target.section != Section::Desecrated || !LORD_CLASSES.contains(&ctx.data.class.as_str()) {
        return None;
    }
    let tiers = ctx.data.mod_tiers(&target.mod_id, ctx.variant)?;
    let omen = single_lord(tiers.iter().map(|tier| tier.name.as_str()))?;
    catalogue::entry(omen).filter(|e| !ctx.is_blocked(e.id))
}

/// The omen of the one lord every name names (`Ulaman's`, `of Ulaman`), or
/// `None` when a name names no lord, two names name different lords, or
/// there is no name.
fn single_lord<'a>(names: impl IntoIterator<Item = &'a str>) -> Option<&'static str> {
    let mut lord = None;
    for name in names {
        let omen = tier_lord(name)?;
        if lord.is_some_and(|l| l != omen) {
            return None;
        }
        lord = Some(omen);
    }
    lord
}

/// The lord omen a desecrated tier's name names: its last word, without a
/// possessive `'s`, is the lord.
fn tier_lord(name: &str) -> Option<&'static str> {
    let word = name.split_whitespace().last()?;
    let word = word
        .strip_suffix("'s")
        .or_else(|| word.strip_suffix("\u{2019}s"))
        .unwrap_or(word);
    LORD_OMENS
        .iter()
        .find(|(lord, _)| *lord == word)
        .map(|(_, omen)| *omen)
}

/// The first alternative none of whose ids (placeholders resolved) is
/// blocked and that fits the item.
fn choose_action(
    ctx: &Ctx,
    st: &State,
    step: &Step,
    side: OmenSide,
    greater: bool,
    lord: &LordOmen,
    essence_for: Option<usize>,
) -> Result<ChosenAction, ActionProblem> {
    let mut blocked = 0;
    let mut reasons = Vec::new();
    for (i, action) in step.actions.iter().enumerate() {
        match alternative_on(
            ctx,
            st,
            i,
            action.get_ref(),
            side,
            greater,
            lord,
            essence_for,
        ) {
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

/// One alternative with its placeholders resolved: `side-omen` to the
/// currency's omen for `side` (the bound target's, or the step's), and
/// `greater-omen` to Omen of Greater Exaltation when `greater` says it
/// fits, else to nothing, and `lord-omen` to `lord`. `Blocked` when an id
/// it uses is blocked.
#[allow(clippy::too_many_arguments)]
pub(crate) fn try_alternative(
    ctx: &Ctx,
    st: &State,
    alternative: usize,
    action: &Action,
    side: Option<Side>,
    greater: bool,
    lord: &LordOmen,
    essence_for: Option<usize>,
) -> Result<ChosenAction, AltProblem> {
    let side = side.map_or(OmenSide::Unknown, OmenSide::Known);
    alternative_on(
        ctx,
        st,
        alternative,
        action,
        side,
        greater,
        lord,
        essence_for,
    )
}

/// [`try_alternative`] with `side-omen`'s side as [`OmenSide`]: when it
/// waits for the last target, the alternative is taken with no omen id for
/// it and `side_omen_waits` set, and is blocked when either of the
/// currency's two side omens is. A lord omen never blocks an alternative:
/// a blocked one resolves to nothing ([`LordOmen`]).
#[allow(clippy::too_many_arguments)]
fn alternative_on(
    ctx: &Ctx,
    st: &State,
    alternative: usize,
    action: &Action,
    side: OmenSide,
    greater: bool,
    lord: &LordOmen,
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
            // A jewel's modifiers have one level each: a Minimum Modifier
            // Level only adds cost, so the next alternative (the plain orb)
            // is taken.
            if e.min_modifier_level.is_some() && class == JEWELS {
                return Err(AltProblem::Unfit(format!(
                    "`{written}` has a minimum modifier level, which only adds cost on a jewel"
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

    let mut omens = Vec::new();
    let mut omen_names = Vec::new();
    let mut omen_side = None;
    let mut side_omen_waits = false;
    let mut lord_omens_waiting = Vec::new();
    let side_omen = |s: Side| {
        currency.side_omen(s).ok_or_else(|| {
            AltProblem::Unfit(format!("`{written}` has no {} omen", side_word(kind_of(s))))
        })
    };
    for omen in &action.omens {
        let id = omen.get_ref().as_str();
        if id == GREATER_OMEN && !greater {
            continue;
        }
        if id == LORD_OMEN {
            match lord {
                LordOmen::Omen(entry) => {
                    omens.push(entry.id.to_string());
                    omen_names.push(entry.name.to_string());
                }
                LordOmen::Waits(each) => lord_omens_waiting = each.clone(),
                LordOmen::Nothing => {}
            }
            continue;
        }
        let entry = if id == GREATER_OMEN {
            catalogue::entry(GREATER_EXALTATION)
                .ok_or_else(|| AltProblem::Unfit(format!("unknown omen `{GREATER_EXALTATION}`")))?
        } else if id == SIDE_OMEN {
            match side {
                OmenSide::Known(s) => side_omen(s)?,
                OmenSide::WaitsForLastTarget => {
                    // Either side's omen may be the one used.
                    let both = [side_omen(Side::Prefix)?, side_omen(Side::Suffix)?];
                    if both.iter().any(|e| ctx.is_blocked(e.id)) {
                        return Err(AltProblem::Blocked);
                    }
                    side_omen_waits = true;
                    continue;
                }
                OmenSide::Unknown => {
                    return Err(AltProblem::Unfit(
                        "`side-omen` with no bound target or side".to_string(),
                    ))
                }
            }
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
        side_omen_waits,
        lord_omens_waiting,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lord_is_read_from_every_tier_name() {
        assert_eq!(single_lord(["Ulaman's"]), Some("omen-of-the-sovereign"));
        assert_eq!(
            single_lord(["of Amanamu", "Amanamu's"]),
            Some("omen-of-the-liege")
        );
        assert_eq!(single_lord(["of Kurgal"]), Some("omen-of-the-blackblooded"));
        // Two lords (the Belt's flask charges prefix), no lord (a jewel's
        // `Lightless`, the Abyss mark's `of the Abyss`), no tier: nothing.
        assert_eq!(single_lord(["Ulaman's", "Kurgal's"]), None);
        assert_eq!(single_lord(["Lightless"]), None);
        assert_eq!(single_lord(["of the Abyss"]), None);
        assert_eq!(single_lord(["Ulaman's", "Lightless"]), None);
        assert_eq!(single_lord(std::iter::empty::<&str>()), None);
        // A lord's name inside another word is not that lord.
        assert_eq!(single_lord(["Ulamanic"]), None);
    }
}
