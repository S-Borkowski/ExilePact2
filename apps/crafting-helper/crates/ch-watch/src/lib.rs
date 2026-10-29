//! The crafting helper's screen watch, its pure part: the item tooltips in a
//! copy of the game's client, their lines, and what those lines say of the
//! item the current craft step was shown on. No thread, no clock, no
//! Windows call: `ch-app`'s watch thread drives it over the calls the glue
//! gives it (`qt-capture`'s Windows backend), and the tests over fakes.
//!
//! - [`Reader`]: copy the client through [`ReadCalls`], find the tooltips
//!   ([`locate`]), report one the overlay's window overlaps as covered, OCR
//!   each other one's crop (once per distinct crop), and return a
//!   [`TooltipRead`]: no tooltip, covered, a black copy, or per box its name
//!   lines, its other lines and the rarity its name colour says.
//! - [`classify`]: a read and the item in, a [`ReadKind`] out (no tooltip,
//!   covered, another item, unreadable with an [`Unreadable`] reason,
//!   unchanged, or changed with the read item). The matcher prefers
//!   unreadable to a guess: every line must be accounted for, every tier
//!   told.
//! - [`item_key`]: an item's rarity and explicit modifiers as (data id,
//!   tier), what "the same item" means for the guide.
//!
//! **Measured on the owner's screenshots** (2026-10-29, 2560 x 1440 and
//! 1920 x 1080; `docs/crafting-helper/crafting-helper-research.md` §
//! "Item tooltip on screen"): [`locate`] finds a tooltip by its opaque
//! header (a frame line along its top and bottom around the name lines, one
//! name line or two) and follows its body, which darkens the scene behind
//! it to about a fifth, down to where the scene shows again; the reader
//! joins the pieces OCR splits one line into and keeps the lines that cross
//! the box's middle (every tooltip line is centred); the matcher accounts
//! for the class line and the key hints ([`INTERFACE_STRINGS`]). The plain
//! form is the one to read: held Alt adds ranges but loses values to OCR.
//! `tests/shots.rs` reads the screenshots themselves when
//! `CH_TOOLTIP_SHOTS` names their folder; the images are never committed.
//!
//! OCR text is private. Copies, crops and lines live in memory only while a
//! read is processed (the reader keeps the last read's crops and lines to
//! skip an unchanged one); nothing here writes a file or formats a line's
//! text, and the `Debug` forms of lines print their length, not their text.

mod locator;
mod matcher;
mod reader;
mod text;

use ch_engine::ch_item::Rarity;
use qt_screen::{PixelLine, PixelRect};

pub use locator::{locate, Located};
pub use matcher::{
    classify, item_key, ItemKey, ReadKind, Unreadable, INTERFACE_STRINGS, LINE_THRESHOLD,
};
pub use reader::{ReadCalls, ReadCounts, Reader};

/// One read of the game's client.
#[derive(Clone, Debug, PartialEq)]
pub enum TooltipRead {
    /// No tooltip in the copy.
    NoTooltip,
    /// A tooltip lies under the overlay's window: nothing was read.
    Covered,
    /// The copy came back black.
    Black,
    /// One entry per tooltip found, never empty.
    Boxes(Vec<TooltipBox>),
}

/// One tooltip and what OCR read of it.
#[derive(Clone, Debug, PartialEq)]
pub struct TooltipBox {
    /// The tooltip, in client pixels.
    pub rect: PixelRect,
    /// The lines in the header band, top to bottom; boxes in client pixels.
    pub name_lines: Vec<PixelLine>,
    /// The lines below it.
    pub lines: Vec<PixelLine>,
    /// The rarity the name lines' colour says, when it is near one.
    pub rarity: Option<Rarity>,
    /// The tooltip touches the copy's edge and may be cut.
    pub touches_edge: bool,
}

impl TooltipBox {
    /// A box with these lines and no geometry, for a caller that has the
    /// text alone (tests, a replay).
    pub fn from_text<S: AsRef<str>>(name_lines: &[S], lines: &[S], rarity: Option<Rarity>) -> Self {
        let line = |s: &S| PixelLine::new(s.as_ref(), PixelRect::new(0, 0, 0, 0));
        TooltipBox {
            rect: PixelRect::new(0, 0, 0, 0),
            name_lines: name_lines.iter().map(line).collect(),
            lines: lines.iter().map(line).collect(),
            rarity,
            touches_edge: false,
        }
    }
}
