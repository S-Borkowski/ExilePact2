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

mod catalogue;
mod context;
mod engine;
mod guide;
mod odds;
mod route;

pub use catalogue::{catalogue, catalogue_item, CatalogueItem};
pub use ch_data::Odds;
pub use context::{Refusal, Target, TargetInfo};
pub use engine::{check_path, resolve, toml_files, Engine, Resolved};
pub use guide::{Guide, GuideError, Next, Outcome, OutcomeKind, Started, Status, WantedMod};
pub use route::{ChosenAction, Plan, Rejected, Rejection, Route, Skipped, StepView};
