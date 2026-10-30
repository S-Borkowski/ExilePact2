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
//!   still valid for it are kept, the others dropped, and the item's own
//!   explicit modifiers not already among them are ticked after them, each
//!   with its floor at the tier it has (so each starts met); one the
//!   validity check below refuses, or one with no data modifier yet (an
//!   unrevealed desecrated one), is left out silently. Otherwise the mode is
//!   refused, with the reason in words. While guiding it is the "after" of
//!   the current step: the engine classifies the change from the item the
//!   step was shown on, the history grows and the guide moves on. A grab
//!   whose text equals the item the step was shown on is not an outcome,
//!   and one that is not a plannable item leaves the guide as it is.
//! - **A change read from the screen** ([`Craft::screen_change`]), with an
//!   active guide: classified the same way; a hit or a miss is kept with
//!   the screen as its source and the read item (which has no copied text)
//!   as the item now; an unexpected change keeps nothing. A grab after it
//!   whose rarity and explicit modifiers (ids and tiers) are the read
//!   item's is not an outcome and replaces it. When either item came from
//!   the screen, both are compared with their explicit lines written from
//!   template and values, since only a grab's lines carry ranges.
//! - **The targets** (`set_targets`): the whole list, each a modifier the
//!   base can roll at the item level with no floor ("as high as possible",
//!   the best tier) or one of the tiers it reaches; anything else refuses
//!   the list whole. Repeats are dropped.
//! - **A side's limit** ([`side_counts`]): the item's prefix and suffix
//!   limits as a rare, read from the item (a Dusk or Gloam implicit, a
//!   Contempt emotion). A side counts its chosen targets (a desecrated one
//!   on the side its data gives) and the item's modifiers no target names
//!   that a route cannot take off (fractured, unrevealed). Every unchosen
//!   choice on a full side carries why (`Prefixes full: 2 on the item + 1
//!   chosen = 3 of 3`); a chosen one never does, so it can be unticked. A
//!   list past a limit is refused whole (`Prefixes over the limit: ...`),
//!   and a grab keeps the targets in list order while their side has room,
//!   the others counted as dropped, then ticks the item's own modifiers
//!   that still fit.
//! - **The blocked list of a craft** (`set_craft_blocked`): the craft keeps
//!   its own additions to the Settings list and its own exceptions from it,
//!   so the effective list is the Settings list plus the additions minus the
//!   exceptions, and a Settings change made meanwhile still reaches it. Both
//!   clear when a grab starts planning from none or refused.
//! - **The preview**: with targets, the engine's first applicable route, or
//!   the one `choose_route` picked while it still applies, the other
//!   applicable routes, or every blueprint's rejection when none applies.
//!   The route view carries what the route targets and what to do, never
//!   the blueprint's `notes`.
//!   `accept_craft` starts the guide on the previewed route; `end_craft`
//!   goes back to planning on the latest item, targets and blocked changes
//!   kept, history cleared. No status line names the blueprint (the
//!   recipe is the app's); the diag lines name it by id.
//! - **Whittle or annul, by level** ([`annulment_by_level`]): a removal
//!   step that offers Omen of Whittling with a Chaos Orb and then a plain
//!   Orb of Annulment is shown, wherever a step is shown (the route's list,
//!   the current step, the overlay, History), as the whittle when the
//!   engine's whittle is a certain hit (the lowest-level modifier is
//!   unwanted), and otherwise as the annulment, which the engine takes with
//!   Omen of Whittling added to the list: its action and odds are the
//!   engine's, and it has no fallback line. The annulment blocked for the
//!   craft keeps the whittle. Only in a listed blueprint in which every step
//!   that names Whittling is such a step. The result is judged against the
//!   way the app named, on a copy of the guide with the longer list that
//!   gets the guide's own list back before anything is kept; the next step
//!   is decided again on the next item.
//! - **A state-ordered route** (`order = "state"`): the engine picks each
//!   step from the item, and its pick is final (no whittle or annul by
//!   level). The preview is the engine's projection, numbered in its order;
//!   while guiding, "Step <n>" counts the steps in the order the craft takes
//!   them ([`state_step_number`]). The hit line names the projection's next
//!   step, on the item now, and the miss line says the item picks the next
//!   step ([`state_branch_lines`]). A fallback line shows only when a
//!   blocked id passed an earlier alternative over ([`fallback_line`]), or
//!   when the step was taken in place of the last target's desecration,
//!   saying why ([`desecration_fallback_line`]).
//!
//! The copy marked Rust's in `docs/crafting-helper/ui-design.md` §
//! "Rust-produced copy" is made here: a step's instruction and target
//! modifiers ([`step_instruction`], [`step_targets`]), [`odds_words`],
//! [`fallback_line`], [`desecration_fallback_line`], [`floor_options`], the branch lines
//! ([`branch_lines`], [`state_branch_lines`]) and the history summary
//! ([`summary`]).

use ch_engine::ch_blueprint::catalogue::{
    bones_for, Currency, ItemGroup, OmenChange, GREATER_OMEN, SIDE_OMEN,
};
use ch_engine::ch_blueprint::{Blueprint, Goal, Order, Side, Step, Transition};
use ch_engine::ch_data::{self, JoinedMod, Kind, Odds, Section, ValidMod};
use ch_engine::ch_item::{self, Item, Rarity};
use ch_engine::{
    catalogue_item, ChosenAction, Engine, Guide, Next, Outcome, OutcomeKind, Refusal, Route,
    Started, Status, StepView, Target, WantedMod,
};

use crate::settings::CatalogueEntry;
use crate::view::{
    BlockedFrom, ChosenTarget, CraftBlockedEntry, CraftBlockedView, CraftItemView, CraftMode,
    CraftView, CurrentStepView, FloorOption, GuideStatusView, GuidingView, HistoryEntry,
    ItemModView, ModSide, OutcomeSource, OutcomeWord, PlanningView, PreviewView, RejectedLine,
    RouteChoice, RouteView, StepLineView, TargetChoice, TargetChoices,
};

/// The one address the app opens: Craft of Exile's Path of Exile 2
/// calculator, in the default browser. No other address is ever opened.
pub const CRAFT_OF_EXILE_URL: &str = "https://beta.craftofexile.com/?game=poe2";

/// An item the engine plans for: grabbed (its text and the item parsed from
/// it), or read from the screen (the item alone).
#[derive(Debug, Clone)]
pub(crate) struct Grabbed {
    /// The normalised text (as the grab keeps it); `None` for an item the
    /// screen watch read, which has no copied text.
    text: Option<String>,
    item: Item,
}

/// A started route.
#[derive(Debug, Clone)]
pub(crate) struct Guiding {
    guide: Guide,
    route: Route,
    /// The item the current step was shown on: the latest one grabbed, or
    /// the latest hit or miss read from the screen.
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
    /// While guiding: the item the screen watch read, grabbed: the same
    /// rarity and explicit modifiers (ids and tiers). The grabbed item
    /// replaces it; not an outcome.
    MatchesScreen,
    /// While guiding: the change was classified.
    Observed {
        result: OutcomeWord,
        step: String,
        summary: String,
    },
}

/// What a change the screen watch read twice alike did to the craft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ScreenEffect {
    /// Not guiding an active guide, or no engine: nothing changed.
    Ignored,
    /// A hit or a miss, kept as a grab's would be (with the screen as its
    /// source).
    Kept {
        result: OutcomeWord,
        step: String,
        summary: String,
    },
    /// A change the guide did not expect at `step`: nothing kept.
    Unexpected { step: String },
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
            text: Some(text.to_string()),
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

/// One side's count against its limit, for the target choices.
#[derive(Debug, Clone, Copy)]
struct SideCount {
    /// The chosen targets on the item, and the item's modifiers no target
    /// names that a route cannot take off (fractured, unrevealed).
    on_item: usize,
    /// The chosen targets not on the item.
    chosen: usize,
    limit: usize,
}

impl SideCount {
    fn total(&self) -> usize {
        self.on_item + self.chosen
    }

    /// `<Prefixes|Suffixes> <what>: <a> on the item + <b> chosen = <a+b> of
    /// <limit>`.
    fn words(&self, kind: Kind, what: &str) -> String {
        let side = match kind {
            Kind::Prefix => "Prefixes",
            Kind::Suffix => "Suffixes",
        };
        format!(
            "{side} {what}: {} on the item + {} chosen = {} of {}",
            self.on_item,
            self.chosen,
            self.total(),
            self.limit
        )
    }
}

/// Both sides' counts.
#[derive(Debug, Clone, Copy)]
struct SideCounts {
    prefixes: SideCount,
    suffixes: SideCount,
}

impl SideCounts {
    fn on(&self, kind: Kind) -> &SideCount {
        match kind {
            Kind::Prefix => &self.prefixes,
            Kind::Suffix => &self.suffixes,
        }
    }

    fn on_mut(&mut self, kind: Kind) -> &mut SideCount {
        match kind {
            Kind::Prefix => &mut self.prefixes,
            Kind::Suffix => &mut self.suffixes,
        }
    }

    /// Why an unchosen choice on `kind`'s side is disabled: the side has
    /// reached its limit.
    fn full(&self, kind: Kind) -> Option<String> {
        let c = self.on(kind);
        (c.total() >= c.limit).then(|| c.words(kind, "full"))
    }

    /// Why a list is refused: `kind`'s side is past its limit.
    fn over(&self, kind: Kind) -> Option<String> {
        let c = self.on(kind);
        (c.total() > c.limit).then(|| c.words(kind, "over the limit"))
    }
}

/// Each side's count against the item's limit as a rare (the craft's
/// destination, whatever its rarity now; the engine's `targets-exceed-slots`
/// uses the same): the chosen targets on the side (a desecrated one on the
/// side its data gives), each once, plus the item's explicit modifiers on it
/// that no target names and a route cannot take off with an ordinary
/// removal (fractured, unrevealed). `None` when the class has no limits.
fn side_counts(item: &Item, valid: &[ValidMod<'_>], targets: &[Target]) -> Option<SideCounts> {
    let limits = ch_data::item_affix_limits(item, &Rarity::Rare)?;
    let side = |limit| SideCount {
        on_item: 0,
        chosen: 0,
        limit,
    };
    let mut counts = SideCounts {
        prefixes: side(limits.prefixes),
        suffixes: side(limits.suffixes),
    };
    let join = ch_data::join(item);
    let on_item = |id: &str| {
        join.mods
            .iter()
            .any(|jm| jm.matched().is_some_and(|m| m.mod_id == id))
    };
    for t in targets {
        let Some(m) = valid.iter().find(|m| m.id == t.mod_id) else {
            continue;
        };
        let c = counts.on_mut(m.kind);
        if on_item(&t.mod_id) {
            c.on_item += 1;
        } else {
            c.chosen += 1;
        }
    }
    for jm in &join.mods {
        let named = jm
            .matched()
            .is_some_and(|m| targets.iter().any(|t| t.mod_id == m.mod_id));
        if (jm.fractured || jm.unrevealed) && !named {
            counts.on_mut(jm.kind).on_item += 1;
        }
    }
    Some(counts)
}

/// Whether `t` after `list` keeps its side within the limit. One that
/// names an unremovable modifier on the item adds nothing to the count.
fn has_room(item: &Item, valid: &[ValidMod<'_>], list: &[Target], t: &Target) -> bool {
    let Some(kind) = valid.iter().find(|m| m.id == t.mod_id).map(|m| m.kind) else {
        return true;
    };
    let mut with = list.to_vec();
    with.push(t.clone());
    match (
        side_counts(item, valid, list),
        side_counts(item, valid, &with),
    ) {
        (Some(without), Some(with)) => {
            let after = with.on(kind);
            after.total() <= after.limit || after.total() <= without.on(kind).total()
        }
        _ => true,
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
                let item = &grabbed.item;
                let valid = valid_mods(item);
                let before = self.targets.len();
                // In list order: each target the item can roll, while its
                // side has room.
                let mut kept: Vec<Target> = Vec::new();
                for t in std::mem::take(&mut self.targets) {
                    if target_problem(&valid, &t).is_none() && has_room(item, &valid, &kept, &t) {
                        kept.push(t);
                    }
                }
                let dropped = before - kept.len();
                self.targets = kept;
                for own in own_targets(item) {
                    let taken = self.targets.iter().any(|t| t.mod_id == own.mod_id);
                    if !taken
                        && target_problem(&valid, &own).is_none()
                        && has_room(item, &valid, &self.targets, &own)
                    {
                        self.targets.push(own);
                    }
                }
                self.mode = Mode::Planning(Box::new(grabbed));
                GrabEffect::Planning { dropped }
            }
            Err(reason) => {
                self.mode = Mode::Refused(reason);
                GrabEffect::Refused
            }
        }
    }

    /// While guiding: the guide's status.
    pub(crate) fn guide_status(&self) -> Option<GuideStatusView> {
        match &self.mode {
            Mode::Guiding(g) => Some(status_view(&g.guide.status)),
            _ => None,
        }
    }

    /// While guiding: the item the current step was shown on, which the
    /// screen watch matches its reads against.
    pub(crate) fn step_item(&self) -> Option<&Item> {
        match &self.mode {
            Mode::Guiding(g) => Some(&g.item.item),
            _ => None,
        }
    }

    /// A change the screen watch read twice alike (`item`: the step's item
    /// with the read rarity and explicit modifiers): with an active guide,
    /// the engine classifies it on a copy of the guide. A hit or a miss is
    /// kept as a grab's is, with the screen as its source, and `item` (no
    /// copied text) becomes the next step's item; an unexpected change keeps
    /// nothing.
    pub(crate) fn screen_change(&mut self, engine: Option<&Engine>, item: Item) -> ScreenEffect {
        let (Mode::Guiding(g), Some(engine)) = (&mut self.mode, engine) else {
            return ScreenEffect::Ignored;
        };
        if g.guide.status != Status::Active {
            return ScreenEffect::Ignored;
        }
        let after = Grabbed { text: None, item };
        let shown = shown_step(Some(engine), g);
        match classify(engine, g, &after, shown.as_ref()) {
            Ok((outcome, _)) if outcome.result == OutcomeKind::Unexpected => {
                ScreenEffect::Unexpected { step: outcome.step }
            }
            Ok((outcome, guide)) => {
                let (result, step, summary) = keep(
                    engine,
                    g,
                    outcome,
                    guide,
                    after,
                    shown.as_ref(),
                    OutcomeSource::Screen,
                );
                ScreenEffect::Kept {
                    result,
                    step,
                    summary,
                }
            }
            Err(_) => ScreenEffect::Ignored,
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
        if let Some(counts) = side_counts(&grabbed.item, &valid, &list) {
            if let Some(why) = counts
                .over(Kind::Prefix)
                .or_else(|| counts.over(Kind::Suffix))
            {
                return Err(why);
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
        if !plan.routes.iter().any(|r| r.blueprint == blueprint) {
            return Err(format!("`{blueprint}` is not a route for this item"));
        }
        self.chosen_route = Some(blueprint.to_string());
        // The blueprint's name is the app's recipe, not the player's: the
        // status line says what happened, the diag line names it by id.
        Ok(Done {
            diag: format!("craft: route {blueprint}"),
            status: "Route chosen.".to_string(),
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
        self.mode = Mode::Guiding(Box::new(Guiding {
            guide,
            route,
            item: grabbed,
            next,
            history: Vec::new(),
        }));
        Ok(Done {
            diag: format!("craft: accepted {id}"),
            status: "Craft started.".to_string(),
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
        let counts = side_counts(item, &valid, &self.targets);
        let mut choices = TargetChoices {
            prefixes: Vec::new(),
            suffixes: Vec::new(),
            desecrated: Vec::new(),
        };
        for m in &valid {
            let is_chosen = self.targets.iter().any(|t| t.mod_id == m.id);
            let disabled = match &counts {
                Some(counts) if !is_chosen => counts.full(m.kind),
                _ => None,
            };
            let choice = TargetChoice {
                mod_id: m.id.to_string(),
                side: mod_side(m.kind),
                text: mod_text(m.lines),
                best_tier: m.best_tier,
                floors: floor_options(m.best_tier, m.tiers.iter().map(|t| t.tier)),
                disabled,
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

/// While guiding, a grab is the result of the current step. A grab whose
/// text equals the grabbed item's the step was shown on is not an outcome;
/// nor is one of the item the screen watch read (no text to compare), when
/// its rarity and explicit modifiers (ids and tiers) are the read item's:
/// the grab then replaces the read item.
fn observe(
    engine: &Engine,
    g: &mut Guiding,
    text: &str,
    checked: Result<Grabbed, String>,
) -> GrabEffect {
    match (&g.item.text, &checked) {
        (Some(before), _) if before == text => return GrabEffect::Unchanged,
        (None, Ok(after)) if same_key(&g.item.item, &after.item) => {
            g.item = after.clone();
            return GrabEffect::MatchesScreen;
        }
        _ => {}
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
    let shown = shown_step(Some(engine), g);
    match classify(engine, g, &after, shown.as_ref()) {
        Ok((outcome, guide)) => {
            let (result, step, summary) = keep(
                engine,
                g,
                outcome,
                guide,
                after,
                shown.as_ref(),
                OutcomeSource::Grab,
            );
            GrabEffect::Observed {
                result,
                step,
                summary,
            }
        }
        Err(why) => GrabEffect::Ignored(why),
    }
}

/// Two items with the same rarity and explicit modifiers (ids and tiers);
/// false when either has a modifier that does not join.
fn same_key(a: &Item, b: &Item) -> bool {
    ch_watch::item_key(a).is_some_and(|k| Some(k) == ch_watch::item_key(b))
}

/// The engine's outcome of the current step for `after`, run on a copy of
/// the guide, and that copy moved on.
///
/// The engine tells the same modifier on both items by its lines as shown.
/// A grabbed item's lines carry the advanced copy's `(min-max)` ranges and
/// an item read from the screen's do not, so when either item came from the
/// screen both are compared in one form: each explicit modifier line
/// written from its template and values ([`as_read`]). Two grabs are
/// compared as M5 compared them.
///
/// A step the app named by level ([`Shown::by_level`]) is judged against the
/// annulment it named: on a copy of the guide whose blocked list also holds
/// Omen of Whittling. The copy gets the guide's own list back in every
/// outcome, and a next step is taken again with it, so what is kept (the
/// step, its tries, bindings and status, and the step `Guiding::next`
/// holds) is what the craft's own list gives; [`shown_step`] decides again
/// on the next item.
fn classify(
    engine: &Engine,
    g: &Guiding,
    after: &Grabbed,
    shown: Option<&Shown>,
) -> Result<(Outcome, Guide), String> {
    let by_level = shown.is_some_and(|s| s.by_level);
    let mut guide = g.guide.clone();
    if by_level {
        guide.blocked.push(WHITTLING.to_string());
    }
    let (before, after) = if g.item.text.is_some() && after.text.is_some() {
        (g.item.item.clone(), after.item.clone())
    } else {
        (as_read(&g.item.item), as_read(&after.item))
    };
    let observed = engine.observe(&mut guide, &before, &after);
    guide.blocked.clone_from(&g.guide.blocked);
    let mut outcome = observed.map_err(|e| e.to_string())?;
    if by_level {
        if let Next::Step { step } = &mut outcome.next {
            if let Ok(own) = engine.current(&guide, &after) {
                **step = own;
            }
        }
    }
    Ok((outcome, guide))
}

/// The item with each explicit modifier line's text written from its
/// template, each `#` its value, and no range: the form a screen read gives.
/// A line whose template does not hold one `#` per value is left as it is.
fn as_read(item: &Item) -> Item {
    let mut out = item.clone();
    for m in &mut out.modifiers {
        if !matches!(m.kind, ch_item::ModKind::Prefix | ch_item::ModKind::Suffix) {
            continue;
        }
        for line in &mut m.lines {
            let parts: Vec<&str> = line.template.split('#').collect();
            if line.values.is_empty() || parts.len() != line.values.len() + 1 {
                continue;
            }
            let mut text = String::from(parts[0]);
            for (value, part) in line.values.iter().zip(&parts[1..]) {
                let v = value.value;
                if v.fract() == 0.0 {
                    text.push_str(&format!("{}", v as i64));
                } else {
                    text.push_str(&format!("{v}"));
                }
                text.push_str(part);
            }
            line.text = text;
        }
    }
    out
}

/// Keeps an outcome: its history entry (with its source), the guide moved
/// on and `after` as the item the next step is shown on. Returns the result,
/// the step and the summary.
#[allow(clippy::too_many_arguments)]
fn keep(
    engine: &Engine,
    g: &mut Guiding,
    outcome: Outcome,
    guide: Guide,
    after: Grabbed,
    shown: Option<&Shown>,
    source: OutcomeSource,
) -> (OutcomeWord, String, String) {
    let blueprint = engine.blueprint(&guide.blueprint).map(|r| r.blueprint());
    let targets = &g.guide.targets;
    let (number, mut instruction) = guide_place(blueprint, g, &outcome.step);
    // The step as it was shown, on the item it was shown on.
    if let Some(shown) = shown.filter(|s| s.step.id == outcome.step) {
        instruction = step_instruction(&shown.step, &g.item.item, targets);
    }
    let summary = summary(&outcome, &after.item);
    let result = outcome_word(outcome.result);
    g.history.push(HistoryEntry {
        number,
        step: outcome.step.clone(),
        instruction,
        result,
        summary: summary.clone(),
        source,
    });
    g.guide = guide;
    g.item = after;
    g.next = outcome.next;
    (result, outcome.step, summary)
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
    // The same blueprint's route with Omen of Whittling added to the list,
    // taken once, the first time a step asks for its annulment.
    let mut longer: Option<Option<Route>> = None;
    let mut steps = Vec::new();
    for (i, s) in route.steps.iter().enumerate() {
        let named = annulment_by_level(blueprint, s, blocked, |list| {
            longer
                .get_or_insert_with(|| {
                    engine
                        .start(&route.blueprint, item, targets, list)
                        .ok()
                        .map(|started| started.route)
                })
                .as_ref()?
                .steps
                .iter()
                .find(|x| x.id == s.id)
                .cloned()
        });
        let shown = match named {
            Some(step) => Shown {
                step,
                by_level: true,
            },
            None => Shown {
                step: s.clone(),
                by_level: false,
            },
        };
        // A state-ordered step's pick with another list: the projection
        // made with it, at the same place.
        let without = |list: &[String]| {
            engine
                .start(&route.blueprint, item, targets, list)
                .ok()
                .and_then(|started| started.route.steps.get(i).cloned())
        };
        steps.push(step_line(
            blueprint,
            &shown,
            i + 1,
            item,
            blocked,
            targets,
            without,
        ));
    }
    PreviewView {
        route: Some(RouteView {
            blueprint: route.blueprint.clone(),
            name: route.name.clone(),
            source: source_line(&route.source),
            steps,
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
    let item = &g.item.item;
    let own = &g.guide.targets;
    let step = match (&g.next, shown_step(engine, g)) {
        (Next::Step { .. }, Some(shown)) if status == GuideStatusView::Active => {
            let (number, _) = guide_place(blueprint, g, &shown.step.id);
            // The current step's pick with another list, on the same item.
            let without = |list: &[String]| {
                let mut copy = g.guide.clone();
                copy.blocked = list.to_vec();
                engine?.current(&copy, item).ok()
            };
            Some(current_step(
                engine,
                blueprint,
                g,
                step_line(
                    blueprint,
                    &shown,
                    number,
                    item,
                    &g.guide.blocked,
                    own,
                    without,
                ),
            ))
        }
        (Next::Stuck { step, .. }, _) => {
            let (number, instruction) = guide_place(blueprint, g, step);
            let targets = g
                .route
                .steps
                .iter()
                .find(|s| s.id == *step)
                .map(|s| step_targets(s, &g.item.item))
                .unwrap_or_default();
            Some(current_step(
                engine,
                blueprint,
                g,
                StepLineView {
                    id: step.clone(),
                    number,
                    instruction,
                    targets,
                    uses: Vec::new(),
                    odds: None,
                    fallback: None,
                },
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
        // The app's: the screen watch's state is not the craft's.
        watch: None,
    }
}

fn current_step(
    engine: Option<&Engine>,
    blueprint: Option<&Blueprint>,
    g: &Guiding,
    step: StepLineView,
) -> CurrentStepView {
    let (on_hit, on_miss) = match blueprint {
        Some(b) if b.order() == Order::State => state_branch_lines(engine, g, &step.id),
        Some(b) => branch_lines(b, &g.route.steps, &step.id, &g.item.item, &g.guide.targets),
        None => (
            "On a hit: see the blueprint".to_string(),
            "On a miss: see the blueprint".to_string(),
        ),
    };
    CurrentStepView {
        step,
        on_hit,
        on_miss,
        try_number: g.guide.tries + 1,
    }
}

/// A step as the app shows it: the engine's pick, or the annulment the app
/// named by level in its place ([`annulment_by_level`]).
#[derive(Debug, Clone)]
struct Shown {
    step: StepView,
    /// The app named the annulment where the engine picked the whittle.
    by_level: bool,
}

/// While guiding an active step: that step as shown, decided on the item it
/// is shown on (`Guiding.item`, as kept) with the craft's own list.
fn shown_step(engine: Option<&Engine>, g: &Guiding) -> Option<Shown> {
    let Next::Step { step } = &g.next else {
        return None;
    };
    let named = engine.and_then(|engine| {
        let blueprint = engine.blueprint(&g.guide.blueprint).map(|r| r.blueprint());
        annulment_by_level(blueprint, step, &g.guide.blocked, |list| {
            let mut copy = g.guide.clone();
            copy.blocked = list.to_vec();
            engine.current(&copy, &g.item.item).ok()
        })
    });
    Some(match named {
        Some(step) => Shown {
            step,
            by_level: true,
        },
        None => Shown {
            step: (**step).clone(),
            by_level: false,
        },
    })
}

/// Omen of Whittling: the first way of a whittle-or-annul step.
const WHITTLING: &str = "omen-of-whittling";
/// Orb of Annulment: its later, plain way.
const ANNULMENT: &str = "orb-of-annulment";

fn names_whittling(action: &ch_engine::ch_blueprint::Action) -> bool {
    action.omens.iter().any(|o| o.get_ref() == WHITTLING)
}

/// A removal step one of whose ways names Omen of Whittling and a later one
/// is a plain Orb of Annulment.
fn whittle_or_annul(step: &Step) -> bool {
    if *step.goal.get_ref() != Goal::RemoveUnwanted {
        return false;
    }
    let Some(at) = step
        .actions
        .iter()
        .position(|a| names_whittling(a.get_ref()))
    else {
        return false;
    };
    step.actions[at + 1..].iter().any(|a| {
        let a = a.get_ref();
        a.currency.get_ref() == ANNULMENT && a.omens.is_empty()
    })
}

/// The whittle or the annulment of a whittle-or-annul step, by the
/// modifier levels: `Some` with the annulment's step, as the engine takes
/// it with Omen of Whittling added to the list (`annul` takes the step with
/// the list it is given), where the engine picked the whittle and it is not
/// a certain hit, unless `blocked` (the craft's own list) blocks the
/// annulment. `None` keeps the engine's pick: any step of a state-ordered
/// blueprint (the engine already took the whittle only when it is certain
/// and an annulment could take a target), a step of another kind, a
/// whittle that is a certain hit, a step the craft's own list already moved
/// to the annulment, an annulment that cannot be taken, and any step of a
/// blueprint in which a step names Omen of Whittling without the annulment
/// after it (the longer list could leave that one with every way blocked).
fn annulment_by_level(
    blueprint: Option<&Blueprint>,
    step: &StepView,
    blocked: &[String],
    annul: impl FnOnce(&[String]) -> Option<StepView>,
) -> Option<StepView> {
    let blueprint = blueprint.filter(|b| b.order() == Order::Listed)?;
    let steps = || blueprint.steps.iter().map(|s| s.get_ref());
    let guard = steps()
        .filter(|s| s.actions.iter().any(|a| names_whittling(a.get_ref())))
        .all(whittle_or_annul);
    let own = steps().find(|s| s.id.get_ref() == &step.id)?;
    if !guard || !whittle_or_annul(own) {
        return None;
    }
    let action = step.action.as_ref()?;
    let whittle = action.omens.iter().any(|o| o == WHITTLING);
    let certain = step
        .odds
        .as_ref()
        .and_then(Odds::percent)
        .is_some_and(|p| p >= 100.0);
    if !whittle || certain || blocked.iter().any(|id| id == ANNULMENT) {
        return None;
    }
    let mut list = blocked.to_vec();
    list.push(WHITTLING.to_string());
    let taken = annul(&list)?;
    let a = taken.action.as_ref()?;
    (taken.id == step.id && a.currency == ANNULMENT && a.omens.is_empty()).then_some(taken)
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
/// the route left it out) and its instruction on `item`
/// ([`step_instruction`] of the route's step; the blueprint's text for a
/// step the route left out).
fn step_place(
    blueprint: Option<&Blueprint>,
    route: &Route,
    id: &str,
    item: &Item,
    targets: &[Target],
) -> (usize, String) {
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
        .map(|s| step_instruction(s, item, targets))
        .or(in_blueprint.map(|(_, say)| say))
        .unwrap_or_else(|| id.to_string());
    (number, instruction)
}

/// A step's number and instruction while guiding, on the item the step is
/// shown on: [`step_place`] in a listed route; in a state-ordered one the
/// number is [`state_step_number`], since the projection made at the start
/// no longer says where the craft stands once a step missed.
fn guide_place(blueprint: Option<&Blueprint>, g: &Guiding, id: &str) -> (usize, String) {
    let (number, instruction) = step_place(blueprint, &g.route, id, &g.item.item, &g.guide.targets);
    if blueprint.is_some_and(|b| b.order() == Order::State) {
        (state_step_number(&g.history, id), instruction)
    } else {
        (number, instruction)
    }
}

/// A state-ordered step's number: the steps counted in the order the craft
/// takes them. The first is 1; a step taken again right after itself keeps
/// its number (the try count says which time), and any other step is the
/// next number, wherever it stands in the blueprint.
pub fn state_step_number(history: &[HistoryEntry], id: &str) -> usize {
    match history.last() {
        None => 1,
        Some(last) if last.step == id => last.number,
        Some(last) => last.number + 1,
    }
}

/// One step's line. A step the app named by level has no fallback line:
/// the whittle it passed over is not blocked. A state-ordered step's
/// fallback line asks `without` for the engine's pick of the step with
/// another list ([`state_fallback_line`]), after the line of a desecration
/// it was taken in place of ([`desecration_fallback_line`]).
fn step_line(
    blueprint: Option<&Blueprint>,
    shown: &Shown,
    number: usize,
    item: &Item,
    blocked: &[String],
    targets: &[Target],
    without: impl FnOnce(&[String]) -> Option<StepView>,
) -> StepLineView {
    let step = &shown.step;
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
        instruction: step_instruction(step, item, targets),
        targets: step_targets(step, item),
        uses,
        odds: step.odds.as_ref().map(odds_words),
        fallback: joined_fallback(
            desecration_fallback_line(step),
            blueprint
                .filter(|_| !shown.by_level)
                .and_then(|b| match b.order() {
                    Order::State => state_fallback_line(b, step, item, blocked, without),
                    Order::Listed => fallback_line(b, step, item, blocked),
                }),
        ),
    }
}

/// The fallback line of a state-ordered step taken in place of the last
/// target's desecration (docs/models/crafting-slots-spec.md, "The
/// last-target desecration"): `<used, with its omens> instead of a
/// desecration: <reason>`, the reason the engine's
/// (`StepView::instead_of_desecration`). `None` when the step replaced no
/// desecration.
pub fn desecration_fallback_line(step: &StepView) -> Option<String> {
    let why = step.instead_of_desecration.as_deref()?;
    let used = step
        .action
        .as_ref()
        .map_or_else(|| "this step".to_string(), uses_words);
    Some(format!("{used} instead of a desecration: {why}"))
}

/// A step's one fallback line: the desecration's first, then the passed-over
/// alternative's, joined by `; `.
fn joined_fallback(desecration: Option<String>, alternative: Option<String>) -> Option<String> {
    match (desecration, alternative) {
        (Some(d), Some(a)) => Some(format!("{d}; {a}")),
        (d, a) => d.or(a),
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

/// The item's own explicit modifiers as targets, in text order, each with
/// its floor at the tier it has; a modifier with no data modifier (an
/// unrevealed desecrated one, or one the data does not know) is left out.
fn own_targets(item: &Item) -> Vec<Target> {
    ch_data::join(item)
        .mods
        .iter()
        .filter_map(|jm| jm.matched())
        .map(|m| Target::new(m.mod_id.as_str(), Some(m.tier)))
        .collect()
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

/// A blueprint's source line as the route view sends it: without its web
/// addresses (a bracketed one goes with its brackets), so no view carries
/// an address; the blueprint file keeps them.
pub fn source_line(source: &str) -> String {
    let mut kept = String::new();
    let mut rest = source;
    while let Some(open) = rest.find('(') {
        let Some(close) = rest[open..].find(')').map(|c| open + c) else {
            break;
        };
        kept.push_str(&rest[..open]);
        if !rest[open..close].contains("://") {
            kept.push_str(&rest[open..=close]);
        }
        rest = &rest[close + 1..];
    }
    kept.push_str(rest);
    let words: Vec<&str> = kept
        .split_whitespace()
        .filter(|w| !w.contains("://"))
        .collect();
    words.join(" ").replace(" ,", ",").replace(" ;", ";")
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

/// Whether a hit of a step with this goal places one of the targets: an
/// added, desecrated, revealed or converted modifier. A removal or fill
/// step's targets are only what its binding names, and a manual step does
/// what the blueprint says.
fn places_a_target(goal: Goal) -> bool {
    matches!(
        goal,
        Goal::AddWanted | Goal::Desecrate | Goal::Reveal | Goal::Convert
    )
}

/// The texts of the target modifiers a hit of `step` may place on `item`,
/// as the planning list writes them; empty for a step that places none.
pub fn step_targets(step: &StepView, item: &Item) -> Vec<String> {
    if !places_a_target(step.goal) {
        return Vec::new();
    }
    step.targets
        .iter()
        .map(|id| mod_text_of(item, id))
        .collect()
}

/// A step's instruction. A step with a currency that places one of its
/// targets is composed from its data, never from the blueprint's text: the
/// currency, `with` each omen, the side when the step is tied to one, and
/// the targets it may place, `or` before the last:
///
/// - `Chaos Orb with Omen of Whittling for +#% to Cold Resistance (suffix)
///   or +#% to Lightning Resistance (suffix)`: not tied to a side, so each
///   target says its own;
/// - `Exalted Orb on the prefix side for +# to maximum Life or +# to
///   maximum Mana`: tied to a side, named once.
///
/// A removal step with a currency names the modifiers it removes
/// ([`removal_instruction`]); `wanted` (the craft's targets) tells which are
/// unwanted. A fill step with a currency names it, `with` each omen, and the
/// side it fills, any modifier being its aim:
///
/// - `Exalted Orb with Omen of Greater Exaltation and Omen of Dextral
///   Exaltation on the suffix side, for any modifier`.
///
/// Any other step (no target left, or a manual step) keeps the blueprint's
/// `say`.
pub fn step_instruction(step: &StepView, item: &Item, wanted: &[Target]) -> String {
    if step.goal == Goal::RemoveUnwanted {
        if let Some(action) = &step.action {
            return removal_instruction(action, item, wanted);
        }
    }
    if step.goal == Goal::Fill {
        if let Some(action) = &step.action {
            let uses = uses_words(action);
            return match step.side.or(action.side) {
                Some(side) => format!("{uses} on the {} side, for any modifier", side_word(side)),
                None => format!("{uses}, for any modifier"),
            };
        }
    }
    let targets = step_targets(step, item);
    let Some(action) = step.action.as_ref().filter(|_| !targets.is_empty()) else {
        return step.say.clone();
    };
    let uses = uses_words(action);
    match step.side {
        Some(side) => format!(
            "{uses} on the {} side for {}",
            side_word(side),
            word_list(&targets, "or")
        ),
        None => {
            let each: Vec<String> = step
                .targets
                .iter()
                .zip(&targets)
                .map(|(id, text)| match target_kind(item, id) {
                    Some(kind) => format!("{text} ({})", side_word(kind)),
                    None => text.clone(),
                })
                .collect();
            format!("{uses} for {}", word_list(&each, "or"))
        }
    }
}

/// The currency, `with` each omen.
fn uses_words(action: &ChosenAction) -> String {
    let mut uses = action.name.clone();
    if !action.omen_names.is_empty() {
        uses.push_str(" with ");
        uses.push_str(&word_list(&action.omen_names, "and"));
    }
    uses
}

/// A removal step's instruction: the currency with its omens and the
/// modifier it removes, by its text as the item shows it. A whittle (an
/// omen that removes the lowest-level modifier) names the removable
/// modifier of the lowest level, or each that shares it; any other removal
/// names the unwanted removable modifiers and says the orb takes one at
/// random. Fractured modifiers, and those off the action's side, are not
/// removable.
///
/// - `Chaos Orb with Omen of Whittling to remove +45(33-60) to Accuracy
///   Rating, the lowest-level modifier`;
/// - `Orb of Annulment to remove +95(90-104) to maximum Mana or
///   +200(168-236) to Accuracy Rating; it takes one modifier at random`.
fn removal_instruction(action: &ChosenAction, item: &Item, wanted: &[Target]) -> String {
    let uses = uses_words(action);
    let join = ch_data::join(item);
    let side = action.side.map(|s| match s {
        Side::Prefix => Kind::Prefix,
        Side::Suffix => Kind::Suffix,
    });
    let removable: Vec<&JoinedMod> = join
        .mods
        .iter()
        .filter(|m| !m.fractured && side.is_none_or(|s| s == m.kind))
        .collect();
    let shown = |m: &JoinedMod| {
        let lines: Vec<&str> = item.modifiers[m.index]
            .lines
            .iter()
            .map(|l| l.text.as_str())
            .collect();
        lines.join(" / ")
    };
    let whittle = action.omens.iter().any(|id| {
        catalogue_item(id)
            .and_then(|c| c.omen.as_ref())
            .is_some_and(|o| o.change == OmenChange::RemoveLowestLevel)
    });
    if whittle {
        let levels: Option<Vec<(u32, &JoinedMod)>> = removable
            .iter()
            .map(|m| m.matched().map(|found| (found.tier_ilvl, *m)))
            .collect();
        let lowest = levels
            .as_ref()
            .and_then(|l| l.iter().map(|(level, _)| *level).min());
        return match (levels, lowest) {
            (Some(levels), Some(lowest)) => {
                let at: Vec<String> = levels
                    .iter()
                    .filter(|(level, _)| *level == lowest)
                    .map(|(_, m)| shown(m))
                    .collect();
                match at.as_slice() {
                    [one] => format!("{uses} to remove {one}, the lowest-level modifier"),
                    _ => format!(
                        "{uses} to remove the lowest-level modifier: {}",
                        word_list(&at, "or")
                    ),
                }
            }
            _ => format!("{uses} to remove the lowest-level modifier"),
        };
    }
    let floors = target_floors(item, wanted);
    let unwanted: Vec<String> = removable
        .iter()
        .filter(|m| {
            !m.matched().is_some_and(|found| {
                floors
                    .iter()
                    .any(|(id, floor)| *id == found.mod_id && found.tier <= *floor)
            })
        })
        .map(|m| shown(m))
        .collect();
    if unwanted.is_empty() {
        format!("{uses}; it takes one modifier at random")
    } else {
        format!(
            "{uses} to remove {}; it takes one modifier at random",
            word_list(&unwanted, "or")
        )
    }
}

/// A modifier id's side on the item's class.
fn target_kind(item: &Item, id: &str) -> Option<Side> {
    let (data, _) = ch_data::variant_of(item).ok()?;
    Some(match data.find_mod(id)?.kind {
        Kind::Prefix => Side::Prefix,
        Kind::Suffix => Side::Suffix,
    })
}

fn side_word(side: Side) -> &'static str {
    match side {
        Side::Prefix => "prefix",
        Side::Suffix => "suffix",
    }
}

/// `a`, `a <last> b`, `a, b <last> c`.
fn word_list(items: &[String], last: &str) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., end] => format!("{} {last} {end}", rest.join(", ")),
    }
}

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

/// The fallback line of a step of a listed blueprint whose chosen way is
/// not the blueprint's first: `<used name> instead of <first alternative's
/// name>: <names> blocked`, or `…: does not fit this item` when no
/// passed-over alternative names a blocked id. `None` when the first way
/// was taken. A state-ordered step's line is [`state_fallback_line`].
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
    let bp_step = blueprint_step(blueprint, &step.id)?;
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

/// The fallback line of a state-ordered step. The engine passes
/// alternatives over by its own rules too (a whittle that is not certain, a
/// side omen with nothing on the other side to protect), and that is no
/// fallback: the line shows only when a blocked id made it pass one over.
/// `without` gives the engine's pick of the same step, on the same item,
/// with a list that leaves out the blocked ids the passed-over alternatives
/// name; when it then takes an earlier alternative, the line is `<used, with
/// its omens> instead of <that alternative, with its omens>: <names>
/// blocked`. `None` otherwise.
pub fn state_fallback_line(
    blueprint: &Blueprint,
    step: &StepView,
    item: &Item,
    blocked: &[String],
    without: impl FnOnce(&[String]) -> Option<StepView>,
) -> Option<String> {
    let action = step.action.as_ref()?;
    let used = action.alternative;
    if used == 0 {
        return None;
    }
    let bp_step = blueprint_step(blueprint, &step.id)?;
    let side = target_side(item, step);
    // The blocked ids each passed-over alternative names.
    let named: Vec<Vec<String>> = bp_step
        .actions
        .iter()
        .take(used)
        .map(|a| {
            let mut ids = action_ids(a.get_ref(), item, side);
            ids.retain(|id| blocked.contains(id));
            ids
        })
        .collect();
    let mut passed: Vec<String> = Vec::new();
    for id in named.iter().flatten() {
        if !passed.contains(id) {
            passed.push(id.clone());
        }
    }
    if passed.is_empty() {
        return None;
    }
    let list: Vec<String> = blocked
        .iter()
        .filter(|id| !passed.contains(id))
        .cloned()
        .collect();
    let other = without(&list).filter(|s| s.id == step.id)?;
    let taken = other.action.as_ref()?.alternative;
    if taken >= used {
        return None;
    }
    // A whittle can be kept from fitting by its annulments being blocked:
    // then the ids it names itself are not the reason, the others are.
    let ids = if named[taken].is_empty() {
        &passed
    } else {
        &named[taken]
    };
    let mut names: Vec<String> = Vec::new();
    for id in ids {
        let name = currency_name(id);
        if !names.contains(&name) {
            names.push(name);
        }
    }
    Some(format!(
        "{} instead of {}: {} blocked",
        uses_words(action),
        alternative_words(bp_step.actions[taken].get_ref(), side),
        names.join(", ")
    ))
}

/// The blueprint's step with id `id`.
fn blueprint_step<'a>(blueprint: &'a Blueprint, id: &str) -> Option<&'a Step> {
    blueprint
        .steps
        .iter()
        .map(|s| s.get_ref())
        .find(|s| s.id.get_ref() == id)
}

/// A blueprint action in words: its currency, `with` each omen it names,
/// the side omen resolved for `side`. Omen of Greater Exaltation's
/// placeholder is left out: it names the omen only where two slots are free.
fn alternative_words(action: &ch_engine::ch_blueprint::Action, side: Option<Side>) -> String {
    let written = action.currency.get_ref().as_str();
    let currency = Currency::of(written);
    let omens: Vec<String> = action
        .omens
        .iter()
        .filter_map(|o| match o.get_ref().as_str() {
            SIDE_OMEN => currency
                .zip(side)
                .and_then(|(c, s)| c.side_omen(s))
                .map(|e| currency_name(e.id)),
            GREATER_OMEN => None,
            id => Some(currency_name(id)),
        })
        .collect();
    let mut words = action_name(written);
    if !omens.is_empty() {
        words.push_str(" with ");
        words.push_str(&word_list(&omens, "and"));
    }
    words
}

/// The side a step's omens are for: its bound target's, or its own.
fn target_side(item: &Item, step: &StepView) -> Option<Side> {
    let from_target = step.target.as_deref().and_then(|id| target_kind(item, id));
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

/// What a miss branch says when it leads to a removal step: the modifier
/// the miss brings does not exist yet, so the step says how once it is
/// shown (whittle or annul, decided then by the modifier levels).
pub const REMOVED_NEXT: &str =
    "the new modifier is removed next; the step says how once it is shown";

/// The two branch lines of a step: `On a hit: …` and `On a miss: …`. The
/// next step is named by its composed instruction ([`step_instruction`] of
/// the route's step, for the targets still missing on `item`); a step the
/// route left out by the blueprint's text. A miss that leads to a removal
/// step says [`REMOVED_NEXT`].
pub fn branch_lines(
    blueprint: &Blueprint,
    route: &[StepView],
    step_id: &str,
    item: &Item,
    targets: &[Target],
) -> (String, String) {
    let steps: Vec<&Step> = blueprint.steps.iter().map(|s| s.get_ref()).collect();
    let Some(index) = steps.iter().position(|s| s.id.get_ref() == step_id) else {
        return (
            "On a hit: see the blueprint".to_string(),
            "On a miss: see the blueprint".to_string(),
        );
    };
    let to = Where {
        steps: &steps,
        index,
        route,
        item,
        targets,
    };
    let step = steps[index];
    let hit = match &step.on_hit {
        Some(t) => to.say(t.get_ref(), false),
        None if step.until.is_some() => format!(
            "this step again until its condition holds, then {}",
            to.say(&Transition::Next, false)
        ),
        None => to.say(&Transition::Next, false),
    };
    let miss = to.say(&step.miss_transition(), true);
    (format!("On a hit: {hit}"), format!("On a miss: {miss}"))
}

/// What a state-ordered route's miss line says: a miss leaves an item the
/// projection did not foresee, and the engine picks the next step from it.
pub const PICKED_AFTER_MISS: &str = "the next step is picked from the item after the miss";

/// What a state-ordered route's hit line says when there is no projection
/// to name the next step from: the route's starts no longer fit the item
/// (both sides full, or a magic route's item made rare), and a step missed
/// since the projection made when the craft started.
pub const PICKED_AFTER_HIT: &str = "the next step is picked from the item after the hit";

/// The two branch lines of a state-ordered route's step `step_id`, on the
/// item it is shown on. The hit line names the projection's next step by
/// its composed instruction ([`step_instruction`]), or says the craft is
/// done when the step is the projection's last. The projection is the route
/// the engine previews from that item with the craft's targets and blocked
/// list; when the engine previews none from it (no start fits the item as
/// it stands), the one made when the craft started, while every step so far
/// hit and the craft stands where it said; else [`PICKED_AFTER_HIT`]. The
/// miss line says [`PICKED_AFTER_MISS`].
pub(crate) fn state_branch_lines(
    engine: Option<&Engine>,
    g: &Guiding,
    step_id: &str,
) -> (String, String) {
    let item = &g.item.item;
    let targets = &g.guide.targets;
    let starts_here = |steps: &[StepView]| steps.first().is_some_and(|s| s.id == step_id);
    let now = engine
        .and_then(|e| {
            e.start(&g.guide.blueprint, item, targets, &g.guide.blocked)
                .ok()
        })
        .map(|started| started.route.steps)
        .filter(|steps| starts_here(steps));
    let followed = || {
        let at = g.history.len();
        let all_hit = g.history.iter().all(|h| h.result == OutcomeWord::Hit);
        g.route
            .steps
            .get(at..)
            .filter(|steps| all_hit && starts_here(steps))
            .map(<[StepView]>::to_vec)
    };
    let projection = now.or_else(followed);
    let hit = match projection.as_deref() {
        Some([_, next, ..]) => step_instruction(next, item, targets),
        Some(_) => "the craft is done".to_string(),
        None => PICKED_AFTER_HIT.to_string(),
    };
    (
        format!("On a hit: {hit}"),
        format!("On a miss: {PICKED_AFTER_MISS}"),
    )
}

/// Where a branch goes, from the step at `index`.
struct Where<'a> {
    steps: &'a [&'a Step],
    index: usize,
    route: &'a [StepView],
    item: &'a Item,
    targets: &'a [Target],
}

impl Where<'_> {
    fn say(&self, t: &Transition, miss: bool) -> String {
        let to = match t {
            Transition::Next => self.steps.get(self.index + 1),
            Transition::Goto(id) => match self.steps.iter().find(|s| s.id.get_ref() == id) {
                Some(s) => Some(s),
                None => return id.clone(),
            },
            Transition::Repeat => return "this step again".to_string(),
            Transition::Restart => return "start over".to_string(),
            Transition::Abandon => return "give the item up".to_string(),
            Transition::Done => return "the craft is done".to_string(),
        };
        match to {
            Some(step) => self.name(step, miss),
            None => "the craft is done".to_string(),
        }
    }

    /// A step by its composed instruction on the item, for the targets the
    /// item still misses (all of them when it misses none).
    fn name(&self, step: &Step, miss: bool) -> String {
        if miss && *step.goal.get_ref() == Goal::RemoveUnwanted {
            return REMOVED_NEXT.to_string();
        }
        let Some(view) = self.route.iter().find(|s| s.id == *step.id.get_ref()) else {
            return step.say.get_ref().clone();
        };
        let mut view = view.clone();
        let floors = target_floors(self.item, self.targets);
        let met: Vec<String> = ch_data::join(self.item)
            .mods
            .iter()
            .filter_map(|m| m.matched())
            .filter(|found| {
                floors
                    .iter()
                    .any(|(id, floor)| *id == found.mod_id && found.tier <= *floor)
            })
            .map(|found| found.mod_id.clone())
            .collect();
        if view.targets.iter().any(|id| !met.contains(id)) {
            view.targets.retain(|id| !met.contains(id));
        }
        step_instruction(&view, self.item, self.targets)
    }
}

/// The history summary of one outcome: the targets gained or lost with
/// their tier, a roll below its floor, or the reasons. It carries no
/// outcome word: the pages show the result's own beside it, and the status
/// line adds it (`Hit: gained +#% to Fire Resistance (T3).`).
pub fn summary(outcome: &Outcome, item: &Item) -> String {
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
    parts.join("; ")
}

/// An outcome's word, as the status line leads with it.
pub fn outcome_title(word: OutcomeWord) -> &'static str {
    match word {
        OutcomeWord::Hit => "Hit",
        OutcomeWord::Miss => "Miss",
        OutcomeWord::Unexpected => "Unexpected",
    }
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

    /// The retired `fill-desecrate-exalt`'s folder: its test copy, a listed
    /// blueprint whose two clean-up steps are whittle-or-annul steps, and the
    /// ring states `ch-engine`'s seed walk takes on it.
    fn fill_dir() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../ch-engine/tests/items/gear/fill-desecrate-exalt")
    }

    /// A state of the ring `ch-engine`'s fill walk takes.
    fn fill_item(name: &str) -> String {
        let path = fill_dir().join(format!("{name}.txt"));
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
    }

    /// The test copy's text.
    fn fill_blueprint() -> String {
        let path = fill_dir().join("fill-desecrate-exalt.toml");
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
    }

    /// An engine of the one listed blueprint `text`.
    fn listed_engine(text: &str) -> Engine {
        let checked =
            ch_engine::ch_blueprint::check("blueprints/fill-desecrate-exalt.toml", text).unwrap();
        Engine::new(vec![checked]).unwrap()
    }

    /// The seed walk's targets: life T3, fire T4, cold T4, chaos T3.
    fn fill_targets() -> Vec<Target> {
        [
            ("prefix-to-maximum-life", 3),
            ("suffix-to-fire-resistance", 4),
            ("suffix-to-cold-resistance", 4),
            ("suffix-to-chaos-resistance", 3),
        ]
        .iter()
        .map(|(id, floor)| Target::new(*id, Some(*floor)))
        .collect()
    }

    /// A craft on `engine` started on `03-fill-fire` with `blocked` for the
    /// craft, whose exalt missed (`07-fill-miss`): its current step.
    fn after_the_miss(engine: &Engine, blocked: &[String]) -> StepLineView {
        let mut craft = Craft::default();
        let start = fill_item("03-fill-fire");
        craft.grab(Some(engine), &start, true);
        craft
            .set_targets(Some(engine), &fill_targets(), &[])
            .unwrap();
        if !blocked.is_empty() {
            craft.set_blocked(blocked.to_vec(), &[]).unwrap();
        }
        craft.accept(Some(engine), &[]).unwrap();
        let effect = craft.grab(Some(engine), &fill_item("07-fill-miss"), true);
        assert!(
            matches!(
                effect,
                GrabEffect::Observed {
                    result: OutcomeWord::Miss,
                    ..
                }
            ),
            "{effect:?}"
        );
        let view = craft.view(Some(engine), &[]);
        let step = view.guiding.unwrap().step.unwrap().step;
        assert_eq!(step.id, "fill-clean");
        step
    }

    #[test]
    fn a_lone_whittle_elsewhere_keeps_the_whittle() {
        let listed = fill_blueprint();
        // Positive control: the listed blueprint, where only the
        // whittle-or-annul steps name Whittling: the app names the
        // annulment (life, a target, is the lowest).
        let engine = listed_engine(&listed);
        assert_eq!(after_the_miss(&engine, &[]).uses, ["Orb of Annulment"]);

        // The same blueprint with one more step that names Omen of Whittling
        // with no annulment after it: the longer list could leave that step
        // with every alternative blocked, so the whittle stays.
        let lone = format!(
            "{listed}\n[[step]]\nid = \"lone-whittle\"\nsay = \"Whittle the lowest modifier\"\n\
             goal = \"remove-unwanted\"\nbind = \"last-remaining\"\nwhen = \"bound\"\n\n\
             [[step.action]]\ncurrency = \"chaos-orb\"\nomens = [\"omen-of-whittling\"]\n"
        );
        let engine = listed_engine(&lone);
        assert_eq!(
            after_the_miss(&engine, &[]).uses,
            ["Chaos Orb", "Omen of Whittling"]
        );
    }

    /// The listed whittle-or-annul rule (a state-ordered step is the
    /// engine's pick, which takes no whittle while an annulment that fits
    /// could not take a target).
    #[test]
    fn a_blocked_annulment_keeps_the_whittle() {
        let engine = listed_engine(&fill_blueprint());
        // The annulment is blocked for this craft: the whittle stays, with
        // its odds as they are (it would take life, a target).
        let step = after_the_miss(&engine, &["orb-of-annulment".to_string()]);
        assert_eq!(step.uses, ["Chaos Orb", "Omen of Whittling"]);
        assert!(
            step.odds.as_deref().unwrap().starts_with("no chance"),
            "{:?}",
            step.odds
        );
        assert_eq!(step.fallback, None);
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
