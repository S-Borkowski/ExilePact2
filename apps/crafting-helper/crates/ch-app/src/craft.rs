//! The craft window: its state ([`Craft`], kept in `App`'s state, in memory
//! only, never saved and never logged as text), what a grab does to it, its
//! commands' rules, and the view of it ([`CraftView`]) built from the
//! shipped engine's plan, guide and outcomes.
//!
//! The four modes ([`CraftMode`]): none (nothing grabbed), refused (the last
//! grab is not an item the engine plans for), planning and guiding.
//!
//! - **A grab** that passes M3's checks, in none, refused or planning,
//!   becomes the planning item when the engine plans for it: the targets
//!   still valid for it are kept, the others dropped. Otherwise the mode is
//!   refused, with the reason in words. While guiding it is the "after" of
//!   the current step: the engine classifies the change from the item the
//!   step was shown on, the history grows and the guide moves on. A grab
//!   whose text equals the item the step was shown on is not an outcome,
//!   and one that is not a plannable item leaves the guide as it is.
//! - **The targets** (`set_targets`): the whole list, each a modifier the
//!   base can roll at the item level with no floor ("as high as possible",
//!   the best tier) or one of the tiers it reaches; anything else refuses
//!   the list whole. Repeats are dropped.
//! - **The blocked list of a craft** (`set_craft_blocked`): the craft keeps
//!   its own additions to the Settings list and its own exceptions from it,
//!   so the effective list is the Settings list plus the additions minus the
//!   exceptions, and a Settings change made meanwhile still reaches it. Both
//!   clear when a grab starts planning from none or refused.
//! - **The preview**: with targets, the engine's first applicable route, or
//!   the one `choose_route` picked while it still applies, the other
//!   applicable routes, or every blueprint's rejection when none applies.
//!   `accept_craft` starts the guide on the previewed route; `end_craft`
//!   goes back to planning on the latest item, targets and blocked changes
//!   kept, history cleared.
//!
//! The copy marked Rust's in `docs/crafting-helper/ui-design.md` §
//! "Rust-produced copy" is made here: [`odds_words`], [`fallback_line`],
//! [`floor_options`], the branch lines ([`branch_lines`]) and the history
//! summary ([`summary`]).

use ch_engine::ch_blueprint::catalogue::{bones_for, Currency, ItemGroup, SIDE_OMEN};
use ch_engine::ch_blueprint::{Blueprint, Side, Step, Transition};
use ch_engine::ch_data::{self, Kind, Odds, Section, ValidMod};
use ch_engine::ch_item::{self, Item, Rarity};
use ch_engine::{
    catalogue_item, Engine, Guide, Next, Outcome, OutcomeKind, Refusal, Route, Started, Status,
    StepView, Target, WantedMod,
};

use crate::settings::CatalogueEntry;
use crate::view::{
    BlockedFrom, ChosenTarget, CraftBlockedEntry, CraftBlockedView, CraftItemView, CraftMode,
    CraftView, CurrentStepView, FloorOption, GuideStatusView, GuidingView, HistoryEntry,
    ItemModView, ModSide, OutcomeWord, PlanningView, PreviewView, RejectedLine, RouteChoice,
    RouteView, StepLineView, TargetChoice, TargetChoices,
};

/// The one address the app opens: Craft of Exile's Path of Exile 2
/// calculator, in the default browser. No other address is ever opened.
pub const CRAFT_OF_EXILE_URL: &str = "https://beta.craftofexile.com/?game=poe2";

/// An item text the engine plans for, and the item parsed from it.
#[derive(Debug, Clone)]
pub(crate) struct Grabbed {
    /// The normalised text (as the grab keeps it).
    text: String,
    item: Item,
}

/// A started route.
#[derive(Debug, Clone)]
pub(crate) struct Guiding {
    guide: Guide,
    route: Route,
    /// The item the current step was shown on: the latest one grabbed.
    item: Grabbed,
    /// What the engine said to do next, on `item`.
    next: Next,
    history: Vec<HistoryEntry>,
}

#[derive(Debug, Clone, Default)]
pub(crate) enum Mode {
    #[default]
    None,
    Refused(String),
    Planning(Box<Grabbed>),
    Guiding(Box<Guiding>),
}

/// The craft's state.
#[derive(Debug, Clone, Default)]
pub(crate) struct Craft {
    pub(crate) mode: Mode,
    /// The chosen targets, in the order chosen.
    targets: Vec<Target>,
    /// The route `choose_route` picked; previewed while it applies.
    chosen_route: Option<String>,
    /// This craft's additions to the Settings blocked list.
    added: Vec<String>,
    /// The Settings ids this craft allows.
    excepted: Vec<String>,
}

/// What a grab did to the craft, for the status line and `diag.log`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GrabEffect {
    /// It is the planning item; how many targets it dropped.
    Planning { dropped: usize },
    /// It cannot be planned: mode refused.
    Refused,
    /// While guiding: the item the step was shown on, again.
    Unchanged,
    /// While guiding: not a plannable item, or the guide is not waiting
    /// for a step's result; the guide is as it was.
    Ignored(String),
    /// While guiding: the change was classified.
    Observed {
        result: OutcomeWord,
        step: String,
        summary: String,
    },
}

/// The result of a craft command: what to log and say, or why it was
/// refused (the craft unchanged).
pub(crate) type CommandResult = Result<Done, String>;

/// A command that went through: its `diag.log` line (no item text) and its
/// status line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Done {
    pub(crate) diag: String,
    pub(crate) status: String,
}

/// Parses `text` and checks the engine plans for the item: the engine's
/// refusals of the item itself, before any target is looked at.
fn plannable(engine: Option<&Engine>, text: &str) -> Result<Grabbed, String> {
    let Some(engine) = engine else {
        return Err("the shipped blueprints did not load, so nothing can be planned".to_string());
    };
    let item = ch_item::parse(text)
        .map_err(|_| "the text is not an item copied with Ctrl+Alt+C in the game".to_string())?;
    // With no target the engine checks the item alone and then stops at
    // "no target": any other refusal is the item's.
    match engine.plan(&item, &[], &[]) {
        Err(Refusal::NoTargets) | Ok(_) => Ok(Grabbed {
            text: text.to_string(),
            item,
        }),
        Err(why) => Err(why.to_string()),
    }
}

/// What the item's base can roll at its item level.
fn valid_mods(item: &Item) -> Vec<ValidMod<'static>> {
    let Ok((data, variant)) = ch_data::variant_of(item) else {
        return Vec::new();
    };
    let level = item.item_level.as_ref().map_or(0, |l| l.value);
    let valid = data.valid_mods(variant, level);
    valid.normal.into_iter().chain(valid.desecrated).collect()
}

/// Whether `target` is one the item can take: a modifier its base rolls at
/// its item level, with no floor or a floor among the tiers it reaches.
fn target_problem(valid: &[ValidMod<'_>], target: &Target) -> Option<String> {
    let Some(m) = valid.iter().find(|m| m.id == target.mod_id) else {
        return Some(format!(
            "`{}` is not a modifier this item can roll",
            target.mod_id
        ));
    };
    match target.floor {
        Some(f) if !m.tiers.iter().any(|t| t.tier == f) => Some(format!(
            "T{f} is not a tier of `{}` this item level reaches",
            target.mod_id
        )),
        _ => None,
    }
}

impl Craft {
    pub(crate) fn mode(&self) -> CraftMode {
        match self.mode {
            Mode::None => CraftMode::None,
            Mode::Refused(_) => CraftMode::Refused,
            Mode::Planning(_) => CraftMode::Planning,
            Mode::Guiding(_) => CraftMode::Guiding,
        }
    }

    /// The effective blocked list: the Settings ids this craft does not
    /// allow, then this craft's additions.
    pub(crate) fn effective_blocked(&self, settings: &[String]) -> Vec<String> {
        let mut ids: Vec<String> = settings
            .iter()
            .filter(|id| !self.excepted.contains(id))
            .cloned()
            .collect();
        for id in &self.added {
            if !ids.contains(id) {
                ids.push(id.clone());
            }
        }
        ids
    }

    /// A grab that passed M3's checks (see the module doc).
    pub(crate) fn grab(
        &mut self,
        engine: Option<&Engine>,
        text: &str,
        looks_like_item: bool,
    ) -> GrabEffect {
        let checked = if looks_like_item {
            plannable(engine, text)
        } else {
            Err("the text is not an item copied with Ctrl+Alt+C in the game".to_string())
        };
        if let Mode::Guiding(g) = &mut self.mode {
            let Some(engine) = engine else {
                return GrabEffect::Ignored("the shipped blueprints did not load".to_string());
            };
            return observe(engine, g, text, checked);
        }
        match checked {
            Ok(grabbed) => {
                if !matches!(self.mode, Mode::Planning(_)) {
                    self.added.clear();
                    self.excepted.clear();
                }
                let valid = valid_mods(&grabbed.item);
                let before = self.targets.len();
                self.targets.retain(|t| target_problem(&valid, t).is_none());
                let dropped = before - self.targets.len();
                self.mode = Mode::Planning(Box::new(grabbed));
                GrabEffect::Planning { dropped }
            }
            Err(reason) => {
                self.mode = Mode::Refused(reason);
                GrabEffect::Refused
            }
        }
    }

    /// `set_targets`: the whole list (see the module doc).
    pub(crate) fn set_targets(
        &mut self,
        engine: Option<&Engine>,
        targets: &[Target],
        settings: &[String],
    ) -> CommandResult {
        let grabbed = self.planning_item()?;
        let valid = valid_mods(&grabbed.item);
        let mut list: Vec<Target> = Vec::new();
        for t in targets {
            let t = Target::new(t.mod_id.trim(), t.floor);
            if let Some(why) = target_problem(&valid, &t) {
                return Err(why);
            }
            if !list.iter().any(|x| x.mod_id == t.mod_id) {
                list.push(t);
            }
        }
        // The engine's own checks of a target list (one per family).
        if let (Some(engine), false) = (engine, list.is_empty()) {
            if let Err(why) = engine.plan(&grabbed.item, &list, &self.effective_blocked(settings)) {
                return Err(why.to_string());
            }
        }
        let n = list.len();
        self.targets = list;
        Ok(Done {
            diag: format!("craft: {n} target{}", plural(n)),
            status: format!("{n} target{} chosen.", plural(n)),
        })
    }

    /// `set_craft_blocked`: the effective list for this craft, catalogue ids
    /// only (already checked by the caller).
    pub(crate) fn set_blocked(
        &mut self,
        blocked: Vec<String>,
        settings: &[String],
    ) -> CommandResult {
        self.planning_item()?;
        self.added = blocked
            .iter()
            .filter(|id| !settings.contains(id))
            .cloned()
            .collect();
        self.excepted = settings
            .iter()
            .filter(|id| !blocked.contains(id))
            .cloned()
            .collect();
        let n = blocked.len();
        Ok(Done {
            diag: format!("craft: {n} blocked for this craft"),
            status: format!("Blocked for this craft: {n}."),
        })
    }

    /// `choose_route`: one of the routes the preview offers.
    pub(crate) fn choose_route(
        &mut self,
        engine: Option<&Engine>,
        blueprint: &str,
        settings: &[String],
    ) -> CommandResult {
        let grabbed = self.planning_item()?;
        let engine = engine.ok_or("the shipped blueprints did not load")?;
        if self.targets.is_empty() {
            return Err("no target is chosen, so there is no route".to_string());
        }
        let plan = engine
            .plan(
                &grabbed.item,
                &self.targets,
                &self.effective_blocked(settings),
            )
            .map_err(|why| why.to_string())?;
        let Some(route) = plan.routes.iter().find(|r| r.blueprint == blueprint) else {
            return Err(format!("`{blueprint}` is not a route for this item"));
        };
        let name = route.name.clone();
        self.chosen_route = Some(blueprint.to_string());
        Ok(Done {
            diag: format!("craft: route {blueprint}"),
            status: format!("Route: {name}."),
        })
    }

    /// `accept_craft`: starts the guide on the previewed route.
    pub(crate) fn accept(&mut self, engine: Option<&Engine>, settings: &[String]) -> CommandResult {
        let grabbed = self.planning_item()?.clone();
        let engine = engine.ok_or("the shipped blueprints did not load")?;
        if self.targets.is_empty() {
            return Err("no target is chosen".to_string());
        }
        let blocked = self.effective_blocked(settings);
        let plan = engine
            .plan(&grabbed.item, &self.targets, &blocked)
            .map_err(|why| why.to_string())?;
        let Some(route) = previewed(&plan.routes, self.chosen_route.as_deref()) else {
            return Err("no route reaches these targets".to_string());
        };
        let id = route.blueprint.clone();
        let Started { guide, route, next } = engine
            .start(&id, &grabbed.item, &self.targets, &blocked)
            .map_err(|e| e.to_string())?;
        let name = route.name.clone();
        self.mode = Mode::Guiding(Box::new(Guiding {
            guide,
            route,
            item: grabbed,
            next,
            history: Vec::new(),
        }));
        Ok(Done {
            diag: format!("craft: accepted {id}"),
            status: format!("Started {name}."),
        })
    }

    /// `end_craft`: back to planning on the latest item.
    pub(crate) fn end(&mut self) -> CommandResult {
        let Mode::Guiding(g) = &self.mode else {
            return Err("no craft is in progress".to_string());
        };
        self.mode = Mode::Planning(Box::new(g.item.clone()));
        Ok(Done {
            diag: "craft: ended".to_string(),
            status: "Craft ended: plan again from the last item.".to_string(),
        })
    }

    /// The planning item, or why a planning command cannot run now.
    fn planning_item(&self) -> Result<&Grabbed, String> {
        match &self.mode {
            Mode::Planning(g) => Ok(g.as_ref()),
            Mode::Guiding(_) => Err("a craft is in progress; end it first".to_string()),
            Mode::None | Mode::Refused(_) => {
                Err("no item is being planned; grab one first".to_string())
            }
        }
    }

    /// The view of the craft, with the Settings blocked list now.
    pub(crate) fn view(&self, engine: Option<&Engine>, settings: &[String]) -> CraftView {
        let mut view = CraftView {
            mode: self.mode(),
            refusal: None,
            planning: None,
            guiding: None,
        };
        match &self.mode {
            Mode::None => {}
            Mode::Refused(why) => view.refusal = Some(why.clone()),
            Mode::Planning(g) => view.planning = Some(self.planning_view(engine, g, settings)),
            Mode::Guiding(g) => view.guiding = Some(guiding_view(engine, g)),
        }
        view
    }

    fn planning_view(
        &self,
        engine: Option<&Engine>,
        grabbed: &Grabbed,
        settings: &[String],
    ) -> PlanningView {
        let item = &grabbed.item;
        let valid = valid_mods(item);
        let mut choices = TargetChoices {
            prefixes: Vec::new(),
            suffixes: Vec::new(),
            desecrated: Vec::new(),
        };
        for m in &valid {
            let choice = TargetChoice {
                mod_id: m.id.to_string(),
                side: mod_side(m.kind),
                text: mod_text(m.lines),
                best_tier: m.best_tier,
                floors: floor_options(m.best_tier, m.tiers.iter().map(|t| t.tier)),
            };
            match (m.section, m.kind) {
                (Section::Desecrated, _) => choices.desecrated.push(choice),
                (Section::Normal, Kind::Prefix) => choices.prefixes.push(choice),
                (Section::Normal, Kind::Suffix) => choices.suffixes.push(choice),
            }
        }
        let targets: Vec<ChosenTarget> = self
            .targets
            .iter()
            .filter_map(|t| {
                let m = valid.iter().find(|m| m.id == t.mod_id)?;
                Some(ChosenTarget {
                    mod_id: t.mod_id.clone(),
                    side: mod_side(m.kind),
                    desecrated: m.section == Section::Desecrated,
                    text: mod_text(m.lines),
                    floor: t.floor,
                    floor_in_force: t.floor.unwrap_or(m.best_tier),
                    best_tier: m.best_tier,
                })
            })
            .collect();
        let floors: Vec<(String, u32)> = targets
            .iter()
            .map(|t| (t.mod_id.clone(), t.floor_in_force))
            .collect();

        let effective = self.effective_blocked(settings);
        let blocked = CraftBlockedView {
            blocked: effective
                .iter()
                .map(|id| CraftBlockedEntry {
                    id: id.clone(),
                    name: currency_name(id),
                    from: if settings.contains(id) {
                        BlockedFrom::Settings
                    } else {
                        BlockedFrom::Craft
                    },
                })
                .collect(),
            allowed: self
                .excepted
                .iter()
                .filter(|id| settings.contains(id))
                .map(|id| CatalogueEntry {
                    id: id.clone(),
                    name: currency_name(id),
                })
                .collect(),
        };

        let preview = match engine {
            Some(engine) if !self.targets.is_empty() => Some(preview(
                engine,
                item,
                &self.targets,
                &effective,
                self.chosen_route.as_deref(),
            )),
            _ => None,
        };
        let can_accept = preview.as_ref().is_some_and(|p| p.route.is_some());
        PlanningView {
            item: item_view(item, &floors),
            choices,
            targets,
            blocked,
            preview,
            can_accept,
        }
    }
}

/// While guiding, a grab is the result of the current step.
fn observe(
    engine: &Engine,
    g: &mut Guiding,
    text: &str,
    checked: Result<Grabbed, String>,
) -> GrabEffect {
    if text == g.item.text {
        return GrabEffect::Unchanged;
    }
    let after = match checked {
        Ok(after) => after,
        Err(why) => return GrabEffect::Ignored(why),
    };
    if g.guide.status != Status::Active {
        return GrabEffect::Ignored(format!(
            "the craft is {}; end it to plan from this item",
            match status_view(&g.guide.status) {
                GuideStatusView::Done => "done",
                GuideStatusView::Stuck => "stuck",
                _ => "waiting to be planned again",
            }
        ));
    }
    let mut guide = g.guide.clone();
    match engine.observe(&mut guide, &g.item.item, &after.item) {
        Ok(outcome) => {
            let blueprint = engine.blueprint(&guide.blueprint).map(|r| r.blueprint());
            let (number, instruction) = step_place(blueprint, &g.route, &outcome.step);
            let summary = summary(&outcome, &after.item);
            let result = outcome_word(outcome.result);
            g.history.push(HistoryEntry {
                number,
                step: outcome.step.clone(),
                instruction,
                result,
                summary: summary.clone(),
            });
            g.guide = guide;
            g.item = after;
            g.next = outcome.next;
            GrabEffect::Observed {
                result,
                step: outcome.step,
                summary,
            }
        }
        Err(e) => GrabEffect::Ignored(e.to_string()),
    }
}

/// The route `chosen` names while it applies, or the engine's first.
fn previewed<'a>(routes: &'a [Route], chosen: Option<&str>) -> Option<&'a Route> {
    chosen
        .and_then(|id| routes.iter().find(|r| r.blueprint == id))
        .or_else(|| routes.first())
}

fn preview(
    engine: &Engine,
    item: &Item,
    targets: &[Target],
    blocked: &[String],
    chosen: Option<&str>,
) -> PreviewView {
    let plan = match engine.plan(item, targets, blocked) {
        Ok(plan) => plan,
        Err(why) => {
            return PreviewView {
                route: None,
                other_routes: Vec::new(),
                rejected: vec![RejectedLine {
                    blueprint: String::new(),
                    name: "Every blueprint".to_string(),
                    reason: why.to_string(),
                }],
            }
        }
    };
    let Some(route) = previewed(&plan.routes, chosen) else {
        return PreviewView {
            route: None,
            other_routes: Vec::new(),
            rejected: plan
                .rejected
                .iter()
                .map(|r| RejectedLine {
                    blueprint: r.blueprint.clone(),
                    name: blueprint_name(engine, &r.blueprint),
                    reason: r.reason.to_string(),
                })
                .collect(),
        };
    };
    let blueprint = engine.blueprint(&route.blueprint).map(|r| r.blueprint());
    PreviewView {
        route: Some(RouteView {
            blueprint: route.blueprint.clone(),
            name: route.name.clone(),
            source: route.source.clone(),
            notes: route.notes.clone(),
            steps: route
                .steps
                .iter()
                .enumerate()
                .map(|(i, s)| step_line(blueprint, s, i + 1, item, blocked))
                .collect(),
        }),
        other_routes: plan
            .routes
            .iter()
            .filter(|r| r.blueprint != route.blueprint)
            .map(|r| RouteChoice {
                blueprint: r.blueprint.clone(),
                name: r.name.clone(),
            })
            .collect(),
        rejected: Vec::new(),
    }
}

fn guiding_view(engine: Option<&Engine>, g: &Guiding) -> GuidingView {
    let blueprint = engine
        .and_then(|e| e.blueprint(&g.guide.blueprint))
        .map(|r| r.blueprint());
    let floors: Vec<(String, u32)> = target_floors(&g.item.item, &g.guide.targets);
    let status = status_view(&g.guide.status);
    let reasons = match (&g.guide.status, &g.next) {
        (Status::Stuck { failed, .. }, _) => failed.clone(),
        (Status::Replan { reasons }, _) => reasons.clone(),
        (Status::Abandoned, _) => {
            vec!["the route gives this item up; grab another item to plan again".to_string()]
        }
        _ => Vec::new(),
    };
    let step = match &g.next {
        Next::Step { step } if status == GuideStatusView::Active => {
            let (number, _) = step_place(blueprint, &g.route, &step.id);
            Some(current_step(
                blueprint,
                step_line(blueprint, step, number, &g.item.item, &g.guide.blocked),
                g.guide.tries,
            ))
        }
        Next::Stuck { step, .. } => {
            let (number, instruction) = step_place(blueprint, &g.route, step);
            Some(current_step(
                blueprint,
                StepLineView {
                    id: step.clone(),
                    number,
                    instruction,
                    uses: Vec::new(),
                    odds: None,
                    fallback: None,
                },
                g.guide.tries,
            ))
        }
        _ => None,
    };
    GuidingView {
        item: item_view(&g.item.item, &floors),
        blueprint: g.guide.blueprint.clone(),
        route_name: g.route.name.clone(),
        status,
        step,
        reasons,
        history: g.history.clone(),
    }
}

fn current_step(blueprint: Option<&Blueprint>, step: StepLineView, tries: u32) -> CurrentStepView {
    let (on_hit, on_miss) = match blueprint {
        Some(b) => branch_lines(b, &step.id),
        None => (
            "On a hit: see the blueprint".to_string(),
            "On a miss: see the blueprint".to_string(),
        ),
    };
    CurrentStepView {
        step,
        on_hit,
        on_miss,
        try_number: tries + 1,
    }
}

/// Each target's floor in force on the item.
fn target_floors(item: &Item, targets: &[Target]) -> Vec<(String, u32)> {
    let valid = valid_mods(item);
    targets
        .iter()
        .filter_map(|t| {
            let m = valid.iter().find(|m| m.id == t.mod_id)?;
            Some((t.mod_id.clone(), t.floor.unwrap_or(m.best_tier)))
        })
        .collect()
}

fn status_view(status: &Status) -> GuideStatusView {
    match status {
        Status::Active => GuideStatusView::Active,
        Status::Done => GuideStatusView::Done,
        Status::Stuck { .. } => GuideStatusView::Stuck,
        // A route that gives the item up cannot go on: plan again.
        Status::Replan { .. } | Status::Abandoned => GuideStatusView::Replan,
    }
}

fn outcome_word(kind: OutcomeKind) -> OutcomeWord {
    match kind {
        OutcomeKind::Hit => OutcomeWord::Hit,
        OutcomeKind::Miss => OutcomeWord::Miss,
        OutcomeKind::Unexpected => OutcomeWord::Unexpected,
    }
}

/// A step's place in the route (from 1; its place in the blueprint when
/// the route left it out) and its instruction.
fn step_place(blueprint: Option<&Blueprint>, route: &Route, id: &str) -> (usize, String) {
    let in_route = route.steps.iter().position(|s| s.id == id);
    let in_blueprint = blueprint.and_then(|b| {
        b.steps
            .iter()
            .position(|s| s.get_ref().id.get_ref() == id)
            .map(|i| (i, b.steps[i].get_ref().say.get_ref().clone()))
    });
    let number = in_route
        .or(in_blueprint.as_ref().map(|(i, _)| *i))
        .map_or(0, |i| i + 1);
    let instruction = route
        .steps
        .iter()
        .find(|s| s.id == id)
        .map(|s| s.say.clone())
        .or(in_blueprint.map(|(_, say)| say))
        .unwrap_or_else(|| id.to_string());
    (number, instruction)
}

fn step_line(
    blueprint: Option<&Blueprint>,
    step: &StepView,
    number: usize,
    item: &Item,
    blocked: &[String],
) -> StepLineView {
    let uses = step
        .action
        .as_ref()
        .map(|a| {
            std::iter::once(a.name.clone())
                .chain(a.omen_names.iter().cloned())
                .collect()
        })
        .unwrap_or_default();
    StepLineView {
        id: step.id.clone(),
        number,
        instruction: step.say.clone(),
        uses,
        odds: step.odds.as_ref().map(odds_words),
        fallback: blueprint.and_then(|b| fallback_line(b, step, item, blocked)),
    }
}

// ---------------------------------------------------------------------------
// The item.

fn item_view(item: &Item, floors: &[(String, u32)]) -> CraftItemView {
    let join = ch_data::join(item);
    let modifiers = join
        .mods
        .iter()
        .map(|jm| {
            let m = &item.modifiers[jm.index];
            let matched = jm.matched();
            let meets_target = matched.is_some_and(|found| {
                floors
                    .iter()
                    .any(|(id, floor)| *id == found.mod_id && found.tier <= *floor)
            });
            ItemModView {
                side: mod_side(jm.kind),
                lines: m.lines.iter().map(|l| l.text.clone()).collect(),
                tier: matched.map(|found| found.tier),
                fractured: jm.fractured,
                desecrated: jm.desecrated,
                unrevealed: jm.unrevealed,
                meets_target,
            }
        })
        .collect();
    CraftItemView {
        name: item.name.clone(),
        base_type: item.base_type.clone(),
        class: item.class.as_str().to_string(),
        rarity: match &item.rarity {
            Rarity::Normal => "Normal".to_string(),
            Rarity::Magic => "Magic".to_string(),
            Rarity::Rare => "Rare".to_string(),
            Rarity::Unique => "Unique".to_string(),
            Rarity::Other(word) => word.clone(),
        },
        item_level: item.item_level.as_ref().map(|l| l.value),
        modifiers,
    }
}

fn mod_side(kind: Kind) -> ModSide {
    match kind {
        Kind::Prefix => ModSide::Prefix,
        Kind::Suffix => ModSide::Suffix,
    }
}

/// A data modifier's lines, values as `#`, joined by ` / `.
fn mod_text(lines: &[String]) -> String {
    lines.join(" / ")
}

/// A modifier id's text on the item's class, or the id.
fn mod_text_of(item: &Item, id: &str) -> String {
    ch_data::variant_of(item)
        .ok()
        .and_then(|(data, _)| data.find_mod(id))
        .map_or_else(|| id.to_string(), |m| mod_text(&m.lines))
}

/// A catalogue id's English name, or the id.
fn currency_name(id: &str) -> String {
    catalogue_item(id).map_or_else(|| id.to_string(), |c| c.name.clone())
}

fn blueprint_name(engine: &Engine, id: &str) -> String {
    engine
        .blueprint(id)
        .map_or_else(|| id.to_string(), |r| r.blueprint().name.get_ref().clone())
}

fn plural(n: usize) -> &'static str {
    if n == 1 {
        ""
    } else {
        "s"
    }
}

// ---------------------------------------------------------------------------
// Rust-produced copy (`ui-design.md` § "Rust-produced copy").

/// The odds in words: `~12% (8000 of 69500 weight)` (an integer percent
/// from 10, one decimal from 1 to under 10, `under 1%` above 0 and under 1),
/// `certain (<basis>)` at 100, `no chance (<basis>)` at 0, and
/// `odds unknown: <reason>`.
pub fn odds_words(odds: &Odds) -> String {
    match odds {
        Odds::Unknown { reason } => format!("odds unknown: {reason}"),
        Odds::Known { percent, basis, .. } => {
            let p = *percent;
            if p >= 100.0 {
                format!("certain ({basis})")
            } else if p <= 0.0 {
                format!("no chance ({basis})")
            } else if p < 1.0 {
                format!("under 1% ({basis})")
            } else if p < 10.0 {
                format!("~{p:.1}% ({basis})")
            } else {
                format!("~{p:.0}% ({basis})")
            }
        }
    }
}

/// A target's floor options: `As high as possible (T<best>)`, then
/// `At least T<n>` per tier the item level reaches, best first.
pub fn floor_options(best: u32, tiers: impl IntoIterator<Item = u32>) -> Vec<FloorOption> {
    let mut tiers: Vec<u32> = tiers.into_iter().collect();
    tiers.sort_unstable();
    tiers.dedup();
    std::iter::once(FloorOption {
        floor: None,
        label: format!("As high as possible (T{best})"),
    })
    .chain(tiers.into_iter().map(|t| FloorOption {
        floor: Some(t),
        label: format!("At least T{t}"),
    }))
    .collect()
}

/// The fallback line of a step whose chosen way is not the blueprint's
/// first: `<used name> instead of <first alternative's name>: <names>
/// blocked`, or `…: does not fit this item` when no passed-over
/// alternative names a blocked id. `None` when the first way was taken.
pub fn fallback_line(
    blueprint: &Blueprint,
    step: &StepView,
    item: &Item,
    blocked: &[String],
) -> Option<String> {
    let action = step.action.as_ref()?;
    if action.alternative == 0 {
        return None;
    }
    let bp_step = blueprint
        .steps
        .iter()
        .map(|s| s.get_ref())
        .find(|s| s.id.get_ref() == &step.id)?;
    let first = bp_step.actions.first()?.get_ref();
    let first_name = action_name(first.currency.get_ref());
    let side = target_side(item, step);
    let mut names: Vec<String> = Vec::new();
    for passed in bp_step.actions.iter().take(action.alternative) {
        for id in action_ids(passed.get_ref(), item, side) {
            if blocked.contains(&id) {
                let name = currency_name(&id);
                if !names.contains(&name) {
                    names.push(name);
                }
            }
        }
    }
    let why = if names.is_empty() {
        "does not fit this item".to_string()
    } else {
        format!("{} blocked", names.join(", "))
    };
    Some(format!("{} instead of {first_name}: {why}", action.name))
}

/// The side a step's omens are for: its bound target's, or its own.
fn target_side(item: &Item, step: &StepView) -> Option<Side> {
    let from_target = step.target.as_deref().and_then(|id| {
        let (data, _) = ch_data::variant_of(item).ok()?;
        Some(match data.find_mod(id)?.kind {
            Kind::Prefix => Side::Prefix,
            Kind::Suffix => Side::Suffix,
        })
    });
    from_target.or(step.side)
}

/// Every id a blueprint action names, its placeholders resolved as far as
/// the item allows (every bone `desecrate` may use; the side omen of the
/// target's side).
fn action_ids(
    action: &ch_engine::ch_blueprint::Action,
    item: &Item,
    side: Option<Side>,
) -> Vec<String> {
    let written = action.currency.get_ref().as_str();
    let currency = Currency::of(written);
    let mut ids = Vec::new();
    match currency {
        Some(Currency::Desecrate) => {
            let group = ItemGroup::of_class(item.class.as_str());
            let level = item.item_level.as_ref().map_or(0, |l| l.value);
            if let Some(group) = group {
                ids.extend(bones_for(group, level).iter().map(|b| b.id.to_string()));
            }
        }
        Some(Currency::Reveal) => {}
        _ => ids.push(written.to_string()),
    }
    for omen in &action.omens {
        let id = omen.get_ref().as_str();
        if id == SIDE_OMEN {
            let resolved = currency.zip(side).and_then(|(c, s)| c.side_omen(s));
            ids.extend(resolved.map(|e| e.id.to_string()));
        } else {
            ids.push(id.to_string());
        }
    }
    ids
}

/// A blueprint action's currency, in words.
fn action_name(written: &str) -> String {
    match Currency::of(written) {
        Some(Currency::Desecrate) => "a bone".to_string(),
        Some(Currency::Reveal) => "Reveal at the Well of Souls".to_string(),
        _ => currency_name(written),
    }
}

/// The two branch lines of a step: `On a hit: …` and `On a miss: …`.
pub fn branch_lines(blueprint: &Blueprint, step_id: &str) -> (String, String) {
    let steps: Vec<&Step> = blueprint.steps.iter().map(|s| s.get_ref()).collect();
    let Some(index) = steps.iter().position(|s| s.id.get_ref() == step_id) else {
        return (
            "On a hit: see the blueprint".to_string(),
            "On a miss: see the blueprint".to_string(),
        );
    };
    let step = steps[index];
    let hit = match &step.on_hit {
        Some(t) => where_to(t.get_ref(), &steps, index),
        None if step.until.is_some() => format!(
            "this step again until its condition holds, then {}",
            where_to(&Transition::Next, &steps, index)
        ),
        None => where_to(&Transition::Next, &steps, index),
    };
    let miss = where_to(&step.miss_transition(), &steps, index);
    (format!("On a hit: {hit}"), format!("On a miss: {miss}"))
}

fn where_to(t: &Transition, steps: &[&Step], index: usize) -> String {
    match t {
        Transition::Next => steps.get(index + 1).map_or_else(
            || "the craft is done".to_string(),
            |s| s.say.get_ref().clone(),
        ),
        Transition::Repeat => "this step again".to_string(),
        Transition::Restart => "start over".to_string(),
        Transition::Abandon => "give the item up".to_string(),
        Transition::Done => "the craft is done".to_string(),
        Transition::Goto(id) => steps
            .iter()
            .find(|s| s.id.get_ref() == id)
            .map_or_else(|| id.clone(), |s| s.say.get_ref().clone()),
    }
}

/// The history summary of one outcome: `Hit:`, `Miss:` or `Unexpected:`,
/// then the targets gained or lost with their tier, a roll below its floor,
/// or the reasons.
pub fn summary(outcome: &Outcome, item: &Item) -> String {
    let word = match outcome.result {
        OutcomeKind::Hit => "Hit",
        OutcomeKind::Miss => "Miss",
        OutcomeKind::Unexpected => "Unexpected",
    };
    let wanted = |w: &WantedMod| {
        let tier = w
            .tier
            .map_or_else(|| "no tier".to_string(), |t| format!("T{t}"));
        format!("{} ({tier})", mod_text_of(item, &w.mod_id))
    };
    let mut parts: Vec<String> = Vec::new();
    if outcome.result == OutcomeKind::Unexpected && !outcome.reasons.is_empty() {
        parts.push(outcome.reasons.join("; "));
    }
    if !outcome.gained.is_empty() {
        let list: Vec<String> = outcome.gained.iter().map(wanted).collect();
        parts.push(format!("gained {}", list.join(", ")));
    }
    if !outcome.lost.is_empty() {
        let list: Vec<String> = outcome.lost.iter().map(wanted).collect();
        parts.push(format!("lost {}", list.join(", ")));
    }
    for b in &outcome.below_floor {
        parts.push(format!("{} below its floor T{}", wanted(b), b.floor));
    }
    if parts.is_empty() {
        parts.push(
            match outcome.result {
                OutcomeKind::Hit => "the step did what it is for, no target gained or lost",
                _ => "no target gained or lost",
            }
            .to_string(),
        );
    }
    format!("{word}: {}", parts.join("; "))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn known(hits: u64, total: u64) -> Odds {
        Odds::known(hits, total, "weight")
    }

    #[test]
    fn odds_forms() {
        assert_eq!(
            odds_words(&known(8000, 69500)),
            "~12% (8000 of 69500 weight)"
        );
        assert_eq!(odds_words(&known(5, 100)), "~5.0% (5 of 100 weight)");
        assert_eq!(odds_words(&known(1, 1000)), "under 1% (1 of 1000 weight)");
        assert_eq!(odds_words(&known(0, 10)), "no chance (0 of 10 weight)");
        assert_eq!(
            odds_words(&Odds::certain(true, "the only modifier")),
            "certain (the only modifier)"
        );
        assert_eq!(
            odds_words(&Odds::unknown("no weights for this base")),
            "odds unknown: no weights for this base"
        );
    }

    #[test]
    fn floor_option_forms() {
        let labels: Vec<String> = floor_options(2, [4, 2, 3])
            .into_iter()
            .map(|o| o.label)
            .collect();
        assert_eq!(
            labels,
            [
                "As high as possible (T2)",
                "At least T2",
                "At least T3",
                "At least T4"
            ]
        );
    }
}
