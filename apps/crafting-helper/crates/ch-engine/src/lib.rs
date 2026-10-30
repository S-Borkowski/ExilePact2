//! The crafting helper's engine: for a parsed item, the target modifiers and
//! the player's blocked currencies, it picks a blueprint route, classifies
//! each next item state as hit, miss or unexpected, and gives the next step
//! with its odds.
//!
//! - [`Engine`]: the blueprints, structurally checked by `ch-blueprint` and
//!   resolved against the data here ([`resolve`]: an essence, alloy or
//!   liquid emotion must exist for one of the blueprint's classes).
//!   [`Engine::shipped`] loads the ones embedded from
//!   `apps/crafting-helper/blueprints/`.
//! - [`Engine::plan`]: refuses an item it cannot plan for ([`Refusal`]),
//!   then lists every applicable [`Route`] (class-specific blueprints first)
//!   and every [`Rejected`] blueprint with its reason.
//! - [`Engine::start`], [`Engine::observe`], [`Engine::current`]: the
//!   guide. Given the item before and after a step, an [`Outcome`] (hit,
//!   miss or unexpected, the wanted modifiers gained and lost) and the
//!   [`Next`] step with its odds.
//! - State order: a blueprint with `order = "state"` has no transitions; the
//!   guide takes the first step that is a candidate on the item as it
//!   stands (a certain removal before a gamble), and its route is the
//!   projection, the steps the craft takes if each one hits
//!   (docs/models/crafting-slots-spec.md). Both read one slot accounting; a
//!   route whose hit path cannot finish is rejected
//!   ([`Rejection::RouteCannotFinish`]).
//! - Either order: a jewel never takes a currency with a Minimum Modifier
//!   Level, and a Magic item whose target prefix is below its top two tiers
//!   is sent back for a new base rather than upgraded
//!   ([`Rejection::WeakMagicStart`]).
//! - [`Engine::relax`]: when the targets plan no route, the one target
//!   whose floor, lowered to the tier an unblocked essence (or alloy, or
//!   liquid emotion) guarantees, then gives a route using that currency
//!   ([`Relaxation`]); `None` when they route already or nothing does.
//! - [`Engine::blocker_advice`]: for a state-ordered route whose reveal is
//!   followed by an exalt on the same side, the desecrated modifiers to take
//!   if the target is not offered, each with the exalt's chance of a target
//!   before and after ([`BlockerAdvice`]); the route keeps that modifier
//!   through the exalt (Omen of Light waits for it).
//! - [`catalogue`]: every crafting item a blueprint or the blocked list may
//!   name, static and data-backed.
//!
//! Every type M5 renders derives `Serialize`; the guide's state ([`Guide`])
//! derives `Deserialize` too. `ch_data`, `ch_blueprint` and `ch_item` are
//! re-exported.
//!
//! ```
//! use ch_engine::{Engine, Target};
//!
//! let text = r#"
//! id = "slam"
//! name = "Slam"
//! classes = ["Rings"]
//! source = "roadmap"
//!
//! [[start]]
//! rarity = ["rare"]
//!
//! [[step]]
//! id = "slam"
//! say = "Exalt for a target"
//! goal = "add-wanted"
//! bind = "any-remaining"
//! [[step.action]]
//! currency = "exalted-orb"
//! "#;
//! let checked = ch_blueprint::check("blueprints/slam.toml", text).unwrap();
//! let engine = Engine::new(vec![checked]).unwrap();
//!
//! let item = ch_item::parse(
//!     "Item Class: Rings\nRarity: Rare\nDoom Loop\nAmethyst Ring\n--------\nItem Level: 76\n",
//! )
//! .unwrap();
//! let targets = [Target::new("prefix-to-maximum-life", Some(8))];
//! let plan = engine.plan(&item, &targets, &[]).unwrap();
//! let first = &plan.routes[0].steps[0];
//! assert_eq!(first.action.as_ref().unwrap().uses, "exalted-orb");
//! // Either side can roll: 8000 life weight over both sides' pools.
//! assert!(first.odds.as_ref().unwrap().percent().unwrap() > 0.0);
//! ```

pub use ch_blueprint;
pub use ch_data;
pub use ch_item;

mod blocker;
mod catalogue;
mod context;
mod engine;
mod guide;
mod odds;
mod relax;
mod route;
mod slots;
mod state_order;

pub use blocker::{BlockerAdvice, BlockerMod, BlockerPick, BLOCKER_LIMIT};
pub use catalogue::{catalogue, catalogue_item, CatalogueItem};
pub use ch_data::Odds;
pub use context::{Refusal, Target, TargetInfo};
pub use engine::{check_path, resolve, toml_files, Engine, Resolved};
pub use guide::{
    Guide, GuideError, Next, Outcome, OutcomeKind, Started, Status, WantedMod, PICKED,
};
pub use relax::Relaxation;
pub use route::{ChosenAction, Plan, Rejected, Rejection, Route, Skipped, Stall, StepView};
pub use state_order::STEP_CAP;
