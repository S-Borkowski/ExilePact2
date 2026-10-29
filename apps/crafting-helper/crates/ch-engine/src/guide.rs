//! The guide: a route started on an item, then, for each next item state,
//! the outcome (hit, miss or unexpected), the wanted modifiers gained and
//! lost, and the next step with its odds, or done, abandon, stuck or
//! re-plan.
//!
//! The guide's state ([`Guide`]) is small and serialisable both ways, so
//! M5 can keep it between grabs; it names its blueprint by id and the
//! engine looks the blueprint up again on every call.

use std::collections::BTreeMap;
use std::fmt;

use ch_blueprint::catalogue::{Effect, OmenChange};
use ch_blueprint::{Goal, Order, Rarity as BpRarity, SideRule, Spanned, Step, Transition};
use ch_data::Kind;
use ch_item::Item;
use serde::{Deserialize, Serialize};

use crate::catalogue::data_entries;
use crate::context::{
    condition_failures, kind_of, rarity_name, side_word, Ctx, ModInfo, Refusal, Role, State, Target,
};
use crate::engine::Engine;
use crate::odds::has_omen_change;
use crate::route::{
    essence_binds_any, pick_step, route_for, ChosenAction, Rejection, Route, Skipped, StepPick,
    StepView,
};
use crate::slots::Slots;
use crate::state_order;

/// [`Outcome::transition`] in a state-ordered route: no transition, the
/// next step was picked from the item.
pub const PICKED: &str = "picked";

/// Where a guide stands. Serialisable both ways.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub enum Status {
    /// Waiting for the next item state after `Guide::step`.
    Active,
    Done,
    Abandoned,
    /// The step's preconditions failed: what failed. The guide picks no
    /// other step.
    Stuck {
        step: String,
        failed: Vec<String>,
    },
    /// The last change was unexpected: plan again.
    Replan {
        reasons: Vec<String>,
    },
}

/// A route being followed. Serialisable both ways.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Guide {
    pub blueprint: String,
    pub targets: Vec<Target>,
    pub blocked: Vec<String>,
    /// The step the player is on.
    pub step: String,
    /// How many times in a row this step has been taken before.
    pub tries: u32,
    /// Step id to the target its `essence` or `hardest` binding chose.
    pub bindings: BTreeMap<String, String>,
    pub status: Status,
}

/// A guide just started: the route and the first step.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Started {
    pub guide: Guide,
    pub route: Route,
    pub next: Next,
}

/// What to do next.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "next", rename_all = "kebab-case")]
pub enum Next {
    Step {
        step: Box<StepView>,
    },
    Done,
    Abandon,
    /// The step's preconditions failed on the item.
    Stuck {
        step: String,
        failed: Vec<String>,
    },
    /// The change was unexpected; plan again.
    Replan {
        reasons: Vec<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum OutcomeKind {
    Hit,
    Miss,
    Unexpected,
}

/// A target modifier, as gained, lost or rolled below its floor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WantedMod {
    pub mod_id: String,
    pub tier: Option<u32>,
    pub floor: u32,
    pub side: Kind,
}

/// The verdict on one change of the item.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Outcome {
    /// The step the change was for.
    pub step: String,
    pub result: OutcomeKind,
    /// Why it was unexpected; empty otherwise.
    pub reasons: Vec<String>,
    /// Targets on the item at their floor now and not before.
    pub gained: Vec<WantedMod>,
    /// Targets on the item at their floor before and not now.
    pub lost: Vec<WantedMod>,
    /// Target modifiers that came in below their floor.
    pub below_floor: Vec<WantedMod>,
    /// The transition taken (`next`, `goto:clean`, …); none when unexpected.
    pub transition: Option<String>,
    /// Steps passed over on the way to the next one.
    pub skipped: Vec<Skipped>,
    pub next: Next,
}

/// Why the guide cannot answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "error", rename_all = "kebab-case")]
pub enum GuideError {
    Refused {
        refusal: Refusal,
    },
    UnknownBlueprint {
        blueprint: String,
    },
    UnknownStep {
        step: String,
    },
    Rejected {
        rejection: Rejection,
    },
    NotActive {
        status: Status,
    },
    /// The guide's step has nothing to do on the item given as "before".
    StepNotTakeable {
        step: String,
        why: String,
    },
}

impl fmt::Display for GuideError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GuideError::Refused { refusal } => write!(f, "{refusal}"),
            GuideError::UnknownBlueprint { blueprint } => write!(f, "no blueprint `{blueprint}`"),
            GuideError::UnknownStep { step } => write!(f, "no step `{step}` in the blueprint"),
            GuideError::Rejected { rejection } => write!(f, "{rejection}"),
            GuideError::NotActive { status } => write!(f, "the guide is not active ({status:?})"),
            GuideError::StepNotTakeable { step, why } => write!(f, "step `{step}`: {why}"),
        }
    }
}

impl std::error::Error for GuideError {}

impl Engine {
    /// Starts `blueprint`'s route on the item: the guide and its first
    /// step, or stuck when that step's preconditions fail.
    pub fn start(
        &self,
        blueprint: &str,
        item: &Item,
        targets: &[Target],
        blocked: &[String],
    ) -> Result<Started, GuideError> {
        let resolved = self
            .blueprint(blueprint)
            .ok_or_else(|| GuideError::UnknownBlueprint {
                blueprint: blueprint.to_string(),
            })?;
        let ctx =
            Ctx::new(item, targets, blocked).map_err(|refusal| GuideError::Refused { refusal })?;
        let route =
            route_for(&ctx, resolved).map_err(|rejection| GuideError::Rejected { rejection })?;
        let st = State::new(item, &ctx.targets);
        let steps = &resolved.blueprint().steps;
        let state = resolved.blueprint().order() == Order::State;
        let mut guide = Guide {
            blueprint: blueprint.to_string(),
            targets: targets.to_vec(),
            blocked: blocked.to_vec(),
            step: String::new(),
            tries: 0,
            // A state-ordered route's bindings are made on the item itself,
            // never on the projection's.
            bindings: if state {
                BTreeMap::new()
            } else {
                route.bindings.clone()
            },
            status: Status::Active,
        };
        let next = if state {
            walk_state(&mut guide, &ctx, &st, steps, None)
        } else {
            walk_from(&mut guide, &ctx, &st, steps, 0, None).0
        };
        Ok(Started { guide, route, next })
    }

    /// The guide's current step on the item: its action and odds.
    pub fn current(&self, guide: &Guide, item: &Item) -> Result<StepView, GuideError> {
        let (steps, index, order) = self.locate(guide)?;
        let ctx = Ctx::new(item, &guide.targets, &guide.blocked)
            .map_err(|refusal| GuideError::Refused { refusal })?;
        let st = State::new(item, &ctx.targets);
        let mut bindings = guide.bindings.clone();
        take(&ctx, &st, steps, index, order, &mut bindings)
    }

    /// Classifies the change from `before` (the item the guide's step was
    /// shown on) to `after`, and moves the guide on.
    pub fn observe(
        &self,
        guide: &mut Guide,
        before: &Item,
        after: &Item,
    ) -> Result<Outcome, GuideError> {
        if guide.status != Status::Active {
            return Err(GuideError::NotActive {
                status: guide.status.clone(),
            });
        }
        let (steps, index, order) = self.locate(guide)?;
        let state = order == Order::State;
        let step = steps[index].get_ref();
        // The contexts borrow these while the guide moves on.
        let (targets, blocked) = (guide.targets.clone(), guide.blocked.clone());
        let ctx = Ctx::new(before, &targets, &blocked)
            .map_err(|refusal| GuideError::Refused { refusal })?;
        let st_before = State::new(before, &ctx.targets);
        let mut bindings = guide.bindings.clone();
        let view = take(&ctx, &st_before, steps, index, order, &mut bindings)?;

        let st_after = State::new(after, &ctx.targets);
        let (removed, added) = diff(&st_before.mods, &st_after.mods);
        let wanted = |st: &State, t: usize| {
            let target = &ctx.targets[t];
            WantedMod {
                mod_id: target.mod_id.clone(),
                tier: st.wanted_mod(t).and_then(|m| m.tier),
                floor: target.floor,
                side: target.side,
            }
        };
        let count = ctx.targets.len();
        let gained: Vec<WantedMod> = (0..count)
            .filter(|&t| !st_before.present(t) && st_after.present(t))
            .map(|t| wanted(&st_after, t))
            .collect();
        let lost: Vec<WantedMod> = (0..count)
            .filter(|&t| st_before.present(t) && !st_after.present(t))
            .map(|t| wanted(&st_before, t))
            .collect();
        let below_floor: Vec<WantedMod> = added
            .iter()
            .filter_map(|m| match m.role {
                Role::BelowFloor(t) => Some(WantedMod {
                    mod_id: ctx.targets[t].mod_id.clone(),
                    tier: m.tier,
                    floor: ctx.targets[t].floor,
                    side: m.side,
                }),
                _ => None,
            })
            .collect();

        let mut reasons = identity_changes(before, after);
        if reasons.is_empty() {
            reasons = fit_reasons(
                &ctx,
                view.action.as_ref(),
                before,
                after,
                &st_before,
                &removed,
                &added,
            );
        }
        if !reasons.is_empty() {
            guide.status = Status::Replan {
                reasons: reasons.clone(),
            };
            return Ok(Outcome {
                step: view.id,
                result: OutcomeKind::Unexpected,
                reasons: reasons.clone(),
                gained,
                lost,
                below_floor,
                transition: None,
                skipped: Vec::new(),
                next: Next::Replan { reasons },
            });
        }

        let ctx_after = Ctx::new(after, &targets, &blocked)
            .map_err(|refusal| GuideError::Refused { refusal })?;
        let aims: Vec<usize> = view
            .targets
            .iter()
            .filter_map(|m| ctx.target_index(m))
            .collect();
        // In state order the side is the one the step's rule resolved to; a
        // `side = "blocked"` removal hits when a non-target went from a side
        // that was blocked before it.
        let side = if state {
            view.side.map(kind_of)
        } else {
            step.side.as_ref().map(|s| kind_of(*s.get_ref()))
        };
        let from_blocked = state
            && step
                .side_rule
                .as_ref()
                .is_some_and(|r| *r.get_ref() == SideRule::Blocked);
        let slots_before = Slots::new(&st_before, &ctx.targets);
        let kept = lost.is_empty();
        let hit = match view.goal {
            Goal::AddWanted | Goal::Convert | Goal::Reveal => {
                kept && aims
                    .iter()
                    .any(|&t| !st_before.present(t) && st_after.present(t))
            }
            Goal::RemoveUnwanted if from_blocked => {
                kept && removed
                    .iter()
                    .any(|m| m.is_unwanted() && slots_before.on(m.side).blocked)
            }
            Goal::RemoveUnwanted => {
                kept && removed
                    .iter()
                    .any(|m| m.is_unwanted() && side.is_none_or(|s| m.side == s))
            }
            Goal::Fill => kept && !added.is_empty(),
            Goal::Desecrate => kept && added.iter().any(|m| m.unrevealed),
            Goal::RemoveDesecrated => kept && removed.iter().any(|m| m.desecrated),
            Goal::Manual => true,
        };
        if state {
            // No transitions: the next step is picked from the item.
            guide.bindings = bindings;
            let next = walk_state(guide, &ctx_after, &st_after, steps, Some(&view.id));
            return Ok(Outcome {
                step: view.id,
                result: if hit {
                    OutcomeKind::Hit
                } else {
                    OutcomeKind::Miss
                },
                reasons: Vec::new(),
                gained,
                lost,
                below_floor,
                transition: Some(PICKED.to_string()),
                skipped: Vec::new(),
                next,
            });
        }
        let transition = if hit {
            let until_holds = step.until.as_ref().is_none_or(|u| {
                let binds = || essence_binds_any(&ctx_after, &st_after, true);
                condition_failures(u.get_ref(), &st_after, &ctx_after.targets, &binds).is_empty()
            });
            step.hit_transition(until_holds)
        } else {
            step.miss_transition()
        };

        guide.bindings = bindings;
        let from = match &transition {
            Transition::Next => index + 1,
            Transition::Repeat => index,
            Transition::Restart => 0,
            Transition::Goto(id) => steps
                .iter()
                .position(|s| s.get_ref().id.get_ref() == id)
                .ok_or_else(|| GuideError::UnknownStep { step: id.clone() })?,
            Transition::Abandon => {
                guide.status = Status::Abandoned;
                return Ok(self.settled(
                    view.id,
                    hit,
                    gained,
                    lost,
                    below_floor,
                    &transition,
                    Next::Abandon,
                ));
            }
            Transition::Done => {
                guide.status = Status::Done;
                return Ok(self.settled(
                    view.id,
                    hit,
                    gained,
                    lost,
                    below_floor,
                    &transition,
                    Next::Done,
                ));
            }
        };
        let (next, skipped) = walk_from(guide, &ctx_after, &st_after, steps, from, Some(index));
        Ok(Outcome {
            step: view.id,
            result: if hit {
                OutcomeKind::Hit
            } else {
                OutcomeKind::Miss
            },
            reasons: Vec::new(),
            gained,
            lost,
            below_floor,
            transition: Some(transition.to_string()),
            skipped,
            next,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn settled(
        &self,
        step: String,
        hit: bool,
        gained: Vec<WantedMod>,
        lost: Vec<WantedMod>,
        below_floor: Vec<WantedMod>,
        transition: &Transition,
        next: Next,
    ) -> Outcome {
        Outcome {
            step,
            result: if hit {
                OutcomeKind::Hit
            } else {
                OutcomeKind::Miss
            },
            reasons: Vec::new(),
            gained,
            lost,
            below_floor,
            transition: Some(transition.to_string()),
            skipped: Vec::new(),
            next,
        }
    }

    fn locate(&self, guide: &Guide) -> Result<(&[Spanned<Step>], usize, Order), GuideError> {
        let resolved =
            self.blueprint(&guide.blueprint)
                .ok_or_else(|| GuideError::UnknownBlueprint {
                    blueprint: guide.blueprint.clone(),
                })?;
        let steps = &resolved.blueprint().steps;
        let index = steps
            .iter()
            .position(|s| s.get_ref().id.get_ref() == &guide.step)
            .ok_or_else(|| GuideError::UnknownStep {
                step: guide.step.clone(),
            })?;
        Ok((steps, index, resolved.blueprint().order()))
    }
}

/// The step at `index` as taken on the item, or why it cannot be. In state
/// order the step must be a candidate on the item.
fn take(
    ctx: &Ctx,
    st: &State,
    steps: &[Spanned<Step>],
    index: usize,
    order: Order,
    bindings: &mut BTreeMap<String, String>,
) -> Result<StepView, GuideError> {
    let step = steps[index].get_ref();
    if order == Order::State {
        state_order::ensure_bindings(ctx, st, steps, bindings);
        let mut st = st.clone();
        state_order::mark_waiting(ctx, &mut st, steps, bindings);
        let slots = Slots::new(&st, &ctx.targets);
        return state_order::evaluate(ctx, &st, &slots, steps, index, bindings)
            .map(|pick| state_order::view(ctx, &st, steps, pick))
            .map_err(|why| GuideError::StepNotTakeable {
                step: step.id.get_ref().clone(),
                why,
            });
    }
    match pick_step(ctx, st, step, steps, bindings) {
        StepPick::Take(view) => Ok(*view),
        StepPick::Skip(why) => Err(GuideError::StepNotTakeable {
            step: step.id.get_ref().clone(),
            why,
        }),
        StepPick::Reject(r) => Err(GuideError::StepNotTakeable {
            step: step.id.get_ref().clone(),
            why: r.to_string(),
        }),
    }
}

/// Moves the guide to the first step from `from` on that is not skipped,
/// and checks its preconditions. `came_from` is the step just taken (a
/// repeat counts a try).
fn walk_from(
    guide: &mut Guide,
    ctx: &Ctx,
    st: &State,
    steps: &[Spanned<Step>],
    from: usize,
    came_from: Option<usize>,
) -> (Next, Vec<Skipped>) {
    let mut skipped = Vec::new();
    let mut index = from;
    loop {
        let Some(step) = steps.get(index).map(|s| s.get_ref()) else {
            guide.status = Status::Done;
            return (Next::Done, skipped);
        };
        let id = step.id.get_ref().clone();
        match pick_step(ctx, st, step, steps, &mut guide.bindings) {
            StepPick::Skip(why) => {
                skipped.push(Skipped { step: id, why });
                index += 1;
            }
            StepPick::Reject(r) => {
                return (stuck(guide, id, vec![r.to_string()]), skipped);
            }
            StepPick::Take(view) => {
                if let Some(requires) = &step.requires {
                    let binds = || essence_binds_any(ctx, st, true);
                    let failed = condition_failures(requires.get_ref(), st, &ctx.targets, &binds);
                    if !failed.is_empty() {
                        return (stuck(guide, id, failed), skipped);
                    }
                }
                guide.tries = if came_from == Some(index) && skipped.is_empty() {
                    guide.tries + 1
                } else {
                    0
                };
                guide.step = id;
                guide.status = Status::Active;
                return (Next::Step { step: view }, skipped);
            }
        }
    }
}

/// State order: the first candidate on the item, or done when no target is
/// missing, or stuck with every step's reason (naming the step the guide
/// was on). `came_from` is the step just taken (a repeat counts a try).
fn walk_state(
    guide: &mut Guide,
    ctx: &Ctx,
    st: &State,
    steps: &[Spanned<Step>],
    came_from: Option<&str>,
) -> Next {
    state_order::ensure_bindings(ctx, st, steps, &mut guide.bindings);
    let mut st = st.clone();
    state_order::mark_waiting(ctx, &mut st, steps, &guide.bindings);
    if st.missing(ctx.targets.len()).is_empty() {
        guide.status = Status::Done;
        return Next::Done;
    }
    match state_order::next_step(ctx, &st, steps, &guide.bindings) {
        Ok(pick) => {
            let view = state_order::view(ctx, &st, steps, pick);
            guide.tries = if came_from == Some(view.id.as_str()) {
                guide.tries + 1
            } else {
                0
            };
            guide.step = view.id.clone();
            guide.status = Status::Active;
            Next::Step {
                step: Box::new(view),
            }
        }
        Err(reasons) => {
            let step = match (guide.step.is_empty(), steps.first()) {
                (true, Some(first)) => first.get_ref().id.get_ref().clone(),
                _ => guide.step.clone(),
            };
            stuck(guide, step, reasons)
        }
    }
}

fn stuck(guide: &mut Guide, step: String, failed: Vec<String>) -> Next {
    guide.step = step.clone();
    guide.status = Status::Stuck {
        step: step.clone(),
        failed: failed.clone(),
    };
    Next::Stuck { step, failed }
}

/// The modifiers that went and came: a multiset difference on side, flags
/// and the lines as shown.
fn diff<'a>(before: &'a [ModInfo], after: &'a [ModInfo]) -> (Vec<&'a ModInfo>, Vec<&'a ModInfo>) {
    let mut used = vec![false; after.len()];
    let mut removed = Vec::new();
    for b in before {
        match (0..after.len()).find(|&i| !used[i] && after[i].same(b)) {
            Some(i) => used[i] = true,
            None => removed.push(b),
        }
    }
    let added = after
        .iter()
        .zip(&used)
        .filter(|(_, used)| !**used)
        .map(|(m, _)| m)
        .collect();
    (removed, added)
}

/// What no currency changes: class, base type, item level, and the item
/// becoming corrupted, mirrored or unidentified.
fn identity_changes(before: &Item, after: &Item) -> Vec<String> {
    let mut reasons = Vec::new();
    if before.class != after.class {
        reasons.push(format!(
            "the item class changed from {} to {}",
            before.class.as_str(),
            after.class.as_str()
        ));
    }
    if before.base_type != after.base_type {
        reasons.push(format!(
            "the base type changed from {} to {}",
            before.base_type.as_deref().unwrap_or("none"),
            after.base_type.as_deref().unwrap_or("none")
        ));
    }
    let level = |i: &Item| i.item_level.as_ref().map(|l| l.value);
    if level(before) != level(after) {
        reasons.push(format!(
            "the item level changed from {:?} to {:?}",
            level(before),
            level(after)
        ));
    }
    if after.corrupted && !before.corrupted {
        reasons.push("the item became corrupted".to_string());
    }
    if after.mirrored && !before.mirrored {
        reasons.push("the item became mirrored".to_string());
    }
    if after.unidentified && !before.unidentified {
        reasons.push("the item became unidentified".to_string());
    }
    reasons
}

/// Whether the change fits the action: fractured modifiers kept, every new
/// modifier joins (but an essence's own), the effect's shape, and a side
/// omen's side.
fn fit_reasons(
    ctx: &Ctx,
    action: Option<&ChosenAction>,
    before: &Item,
    after: &Item,
    st_before: &State,
    removed: &[&ModInfo],
    added: &[&ModInfo],
) -> Vec<String> {
    let mut reasons = Vec::new();
    let effect = action.map_or(Effect::Manual, |a| a.effect);
    if effect == Effect::Manual {
        return reasons;
    }
    let action = action.expect("an effect other than manual comes from an action");

    if effect != Effect::Fracture
        && (removed.iter().any(|m| m.fractured) || added.iter().any(|m| m.fractured))
    {
        reasons.push("a fractured modifier changed".to_string());
    }

    let essence_lines: Vec<Vec<String>> = if action.data_backed {
        data_entries(ctx.data, &action.uses)
            .iter()
            .map(|e| e.essence.lines.clone())
            .collect()
    } else {
        Vec::new()
    };
    for m in added {
        if !m.unrevealed && m.mod_id.is_none() && !essence_lines.contains(&m.templates) {
            reasons.push(format!(
                "the new modifier `{}` does not join the data",
                m.shown()
            ));
        }
    }

    if let Some(why) = effect_misfit(action, before, after, st_before, removed, added) {
        reasons.push(why);
    }

    if let Some(side) = action.side.map(kind_of) {
        let changed: &[&ModInfo] = if matches!(effect, Effect::Swap | Effect::Remove) {
            removed
        } else {
            added
        };
        if let Some(m) = changed.iter().find(|m| m.side != side) {
            reasons.push(format!(
                "the omen restricts the change to the {} side, but the {} `{}` changed",
                side_word(side),
                side_word(m.side),
                m.shown()
            ));
        }
    }
    reasons
}

fn effect_misfit(
    action: &ChosenAction,
    before: &Item,
    after: &Item,
    st_before: &State,
    removed: &[&ModInfo],
    added: &[&ModInfo],
) -> Option<String> {
    let (gone, came) = (removed.len(), added.len());
    let same_rarity = before.rarity == after.rarity;
    let rarity_went = |from: BpRarity, to: BpRarity| {
        crate::context::bp_rarity(&before.rarity) == Some(from)
            && crate::context::bp_rarity(&after.rarity) == Some(to)
    };
    let templates = |list: &[&ModInfo]| {
        let mut t: Vec<(&str, Vec<String>)> = list
            .iter()
            .map(|m| (side_word(m.side), m.templates.clone()))
            .collect();
        t.sort();
        t
    };
    let (fits, expected): (bool, String) = match action.effect {
        Effect::Manual | Effect::Omen => (true, String::new()),
        Effect::Add => {
            let n = if has_omen_change(action, OmenChange::AddTwo) {
                2
            } else {
                1
            };
            (
                same_rarity && gone == 0 && came == n,
                format!("adds {n} modifier(s)"),
            )
        }
        Effect::Augment => (
            same_rarity && gone == 0 && came == 1,
            "adds one modifier".to_string(),
        ),
        Effect::Transmute => (
            rarity_went(BpRarity::Normal, BpRarity::Magic) && gone == 0 && came >= 1,
            "makes a Normal item Magic".to_string(),
        ),
        Effect::Upgrade => (
            rarity_went(BpRarity::Magic, BpRarity::Rare) && gone == 0 && came == 1,
            "makes a Magic item Rare with one more modifier".to_string(),
        ),
        Effect::Alchemy => (
            crate::context::bp_rarity(&after.rarity) == Some(BpRarity::Rare)
                && gone == st_before.mods.len()
                && came >= 1,
            "makes the item Rare with new modifiers".to_string(),
        ),
        Effect::Swap | Effect::Convert => (
            same_rarity && gone == 1 && came == 1,
            "removes one modifier and adds one".to_string(),
        ),
        Effect::Remove => {
            let n = if has_omen_change(action, OmenChange::RemoveTwo) {
                2
            } else {
                1
            };
            let side = action.side.map(kind_of);
            let removable = st_before
                .mods
                .iter()
                .filter(|m| !m.fractured && side.is_none_or(|s| m.side == s))
                .count();
            let expect = n.min(removable);
            (
                same_rarity && came == 0 && gone >= 1 && gone == expect,
                format!("removes {n} modifier(s)"),
            )
        }
        Effect::Desecrate => (
            same_rarity && came == 1 && added[0].unrevealed && gone <= 1,
            "adds one unrevealed desecrated modifier".to_string(),
        ),
        Effect::Reveal => (
            same_rarity
                && gone == 1
                && came == 1
                && removed[0].unrevealed
                && !added[0].unrevealed
                && added[0].desecrated
                && added[0].side == removed[0].side,
            "reveals the unrevealed modifier".to_string(),
        ),
        Effect::RerollValues => (
            same_rarity && templates(removed) == templates(added),
            "rerolls the values only".to_string(),
        ),
        Effect::Fracture => (
            same_rarity
                && gone == 1
                && came == 1
                && added[0].fractured
                && removed[0].templates == added[0].templates,
            "fractures one modifier".to_string(),
        ),
    };
    if fits {
        return None;
    }
    let rarity = if same_rarity {
        String::new()
    } else {
        format!(
            ", and the rarity went from {} to {}",
            rarity_name(&before.rarity),
            rarity_name(&after.rarity)
        )
    };
    Some(format!(
        "{} {expected}, but {gone} modifier(s) went and {came} came{rarity}",
        action.name
    ))
}
