//! Waystone search-string engine: loads and validates the waystone mod data,
//! picks each mod's token and builds one Path of Exile 2 search string from
//! the player's rules, kept under the game's 250-character limit or refused
//! whole, never truncated.
//!
//! - [`data`]: the data contract (`data/waystone-mods.json`) and its
//!   validator, whose every refusal names the family and the field.
//! - [`frame`]: the text every waystone carries besides its mods (measured
//!   in Live procedure 1, 2026-10-28).
//! - [`tokens`]: each family's own token and the set cover for "any of".
//! - [`build`]: the rarity, tier, want, avoid and corruption terms and the
//!   assembler with its outcomes.
//! - [`model`]: a model of the game's search box, the oracle of the tests;
//!   not used to build anything.

pub mod build;
pub mod data;
pub mod frame;
pub mod model;
pub mod tokens;

pub use build::{
    quote, rarity_term, tier_term, Builder, Corruption, Outcome, Part, PartWhat, Rarity, Selection,
    WantMode, LIMIT, TIER_MAX, TIER_MIN,
};
pub use data::{corruption_collisions, DataError, Family, Group, Kind, ModData, Skipped, SCHEMA};
pub use frame::{frame_lines, ITEM_FRAME_LINES};
pub use tokens::{candidates, FamilyToken, Tokens, MIN_TOKEN_LEN};
