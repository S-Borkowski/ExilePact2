//! Path of Exile 2 item text parser: a pure function from the text the game
//! puts on the clipboard for an item to a typed [`Item`].
//!
//! The advanced form puts a header above each modifier and the roll range
//! after each number; both `Ctrl+Alt+C` and plain `Ctrl+C` give it (measured
//! on one item, 2026-10-28). A short form with the modifier lines alone is
//! read too, though no copy has been seen to give it:
//!
//! ```text
//! Item Class: Gloves
//! Rarity: Rare
//! Grim Grasp
//! Plated Gauntlets
//! --------
//! Item Level: 78
//! --------
//! { Desecrated Suffix Modifier "of Exile" (Tier: 2) — Chaos, Resistance }
//! +22(20-23)% to Chaos Resistance
//! ```
//!
//! The layout of class, rarity, name lines, sections, properties, item level,
//! headers and roll ranges was measured from real copied waystones, and the
//! equipment shapes in `fixtures/real/` from the owner's pastes; the shapes
//! no real text has shown yet (fractured, crafted and unrevealed modifiers,
//! enchants, `Mirrored`, `Unidentified`, the requirements block) are guesses
//! kept in one vocabulary table until real pastes confirm them.
//!
//! Nothing is dropped: every non-blank, non-separator line is consumed into
//! the model or listed in [`Report::unparsed`], and a line consumed only in
//! part is listed in [`Report::partial`]. Each consumed element keeps its
//! input line index ([`split_lines`] numbers the lines), and
//! [`Item::consumed_lines`] collects them.
//!
//! ```
//! let text = "Item Class: Rings\nRarity: Normal\nRuby Ring\n--------\nItem Level: 12\n";
//! let item = ch_item::parse(text).unwrap();
//! assert_eq!(item.class, ch_item::ItemClass::Rings);
//! assert_eq!(item.base_type.as_deref(), Some("Ruby Ring"));
//! assert!(item.report.unparsed.is_empty());
//! assert!(ch_item::parse("not an item").is_err());
//! ```

mod line;
mod model;
mod parser;
mod vocab;

pub use model::{
    Form, Item, ItemClass, ItemLevel, Line, Marker, ModKind, ModLine, Modifier, ParseError,
    Partial, Property, Quality, Range, Rarity, Report, Requirements, Sockets, Unparsed, Value,
};
pub use parser::{parse, split_lines};
