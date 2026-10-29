//! The crafting helper's blueprint format: the model of a hand-edited TOML
//! blueprint under `apps/crafting-helper/blueprints/`, its reader and its
//! structural validator, whose every error names its file, line and column.
//!
//! - [`model`]: what a blueprint file says (the format's contract with
//!   `ch-engine`), with the span of every value a check may refuse.
//! - [`catalogue`]: every currency, omen, bone, flux and catalyst with its
//!   id and effect, the shapes of the data-backed essence, alloy and
//!   liquid ids, the placeholders, and the not-established fact ids a
//!   blueprint's `relies_on` may name.
//! - [`validate`]: [`check`] a blueprint's text, or [`check_file`]; every
//!   error is one `<path>:<line>:<col>: <message>` [`Diagnostic`].
//! - [`shipped`]: the blueprints embedded at build time.
//!
//! ```
//! let text = r#"
//! id = "tiny"
//! name = "Tiny"
//! classes = ["Rings"]
//! source = "roadmap"
//!
//! [[start]]
//! rarity = ["rare"]
//!
//! [[step]]
//! id = "slam"
//! say = "Exalt"
//! goal = "fill"
//! [[step.action]]
//! currency = "exalted-orb"
//! "#;
//! let checked = ch_blueprint::check("blueprints/tiny.toml", text).unwrap();
//! assert_eq!(checked.blueprint.steps.len(), 1);
//!
//! let errors = ch_blueprint::check("blueprints/other.toml", text).unwrap_err();
//! assert_eq!(
//!     errors[0].to_string(),
//!     "blueprints/other.toml:2:6: id `tiny` is not the file name `other`"
//! );
//! ```

pub mod catalogue;
pub mod model;
pub mod validate;

pub use catalogue::{Currency, Effect, Entry, Kind, CATALOGUE};
pub use model::{
    Action, Binding, Blueprint, Condition, Goal, Order, Rarity, Side, SideRule, Span, Spanned, Step, Transition, When,
};
pub use validate::{check, check_file, line_col, Checked, Diagnostic};

/// A blueprint file embedded at build time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shipped {
    /// The file name, `<id>.toml`.
    pub file: &'static str,
    pub text: &'static str,
}

/// Where the shipped blueprints live, relative to the repository root; the
/// path their errors name.
pub const SHIPPED_FOLDER: &str = "apps/crafting-helper/blueprints";

/// Every `apps/crafting-helper/blueprints/*.toml` as built, sorted by file
/// name (`build.rs`).
pub fn shipped() -> &'static [Shipped] {
    include!(concat!(env!("OUT_DIR"), "/shipped.rs"))
}

/// Validates every shipped blueprint, in file order.
pub fn check_shipped() -> Vec<Result<Checked, Vec<Diagnostic>>> {
    shipped()
        .iter()
        .map(|s| check(&format!("{SHIPPED_FOLDER}/{}", s.file), s.text))
        .collect()
}
