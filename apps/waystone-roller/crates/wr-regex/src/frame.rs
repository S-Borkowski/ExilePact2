//! The text every waystone shows besides its mods, which a token must never
//! match.
//!
//! **Assumed, not measured**: written from poe2db's tier pages and a public
//! tool's practice (context "Item frame lines"). Live procedure 1's
//! `item-text` replaces it with what the game's item text really holds.
//! Random rare names and the magic affix names of regular mods are unknown
//! (poedb lists none), so they are not here; the minimum token length is the
//! only guard against them.

use crate::data::{Group, ModData};

/// The frame lines, with `#` for a number. Matched lower-cased.
pub const ITEM_FRAME_LINES: &[&str] = &[
    "Item Class: Waystones",
    "Rarity: Normal",
    "Rarity: Magic",
    "Rarity: Rare",
    "Rarity: Unique",
    "Waystone (Tier #)",
    "Waystone Tier: #",
    "Item Level: #",
    "Item Rarity: +#%",
    "Waystone Drop Chance: +#%",
    "Pack Size: +#%",
    "Monster Effectiveness: +#%",
    "Rare Monsters: +#%",
    "Magic Monsters: +#%",
    "Revives Available: #",
    "Corrupted",
    "Unidentified",
    "Can be used in a Map Device, allowing you to enter a Map. Waystones can only be used once.",
];

/// The lower-cased frame lines for `data`: [`ITEM_FRAME_LINES`] plus every
/// desecrated affix name, which shows in a magic waystone's name.
pub fn frame_lines(data: &ModData) -> Vec<String> {
    let mut lines: Vec<String> = ITEM_FRAME_LINES.iter().map(|l| l.to_lowercase()).collect();
    for f in &data.families {
        if f.group == Group::Desecrated {
            if let Some(name) = &f.name {
                let name = name.to_lowercase();
                if !lines.contains(&name) {
                    lines.push(name);
                }
            }
        }
    }
    lines
}
