//! Path of Exile 2 item text parser: a pure function from the text the game
//! puts on the clipboard for an item to a typed [`Item`].
//!
//! Both copies are read. The advanced one (`Ctrl+Alt+C`) puts a header above
//! each modifier and the roll range after each number; the short one
//! (`Ctrl+C`) shows the modifier lines alone:
//!
//! ```text
//! Item Class: Gloves
//! Rarity: Rare
//! Grim Grasp
//! Plated Gauntlets
//! --------
//! Item Level: 78
//! --------
//! { Fractured Prefix Modifier "Hale" (Tier: 4) — Life }
//! +62(60-69) to maximum Life (fractured)
//! ```
//!
//! The layout of class, rarity, name lines, sections, properties, item level,
//! headers and roll ranges was measured from real copied waystones; the
//! equipment-specific shapes (class strings, requirements, sockets, line
//! markers, header adjectives, family tags, unrevealed modifiers) are guesses
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
