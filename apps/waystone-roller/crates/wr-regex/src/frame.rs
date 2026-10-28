//! The text every waystone shows besides its mods, which a token must never
//! match.
//!
//! **Measured 2026-10-28** (Live 1, `item-text`): the advanced item texts of
//! eight waystones (Normal, Magic, Rare and corrupted Rare; tiers 1, 2 and
//! 15). The text carries an affix header above each mod and a roll range
//! after each number (`+14(11-15)%`); a summary line a mod raised ends in
//! `(augmented)`; no per-mod reward line showed. The lines under "not seen"
//! did not show on those eight and stay as guards: a frame line only keeps a
//! token away from more text. Random rare names and the affix names of
//! regular mods are unknown (poedb lists none), so they are not here; the
//! minimum token length is the only guard against them.

use crate::data::{Group, ModData};

/// The frame lines, with `#` for a number or a name. Matched lower-cased.
pub const ITEM_FRAME_LINES: &[&str] = &[
    "Item Class: Waystones",
    "Rarity: Normal",
    "Rarity: Magic",
    "Rarity: Rare",
    "Waystone (Tier #)",
    "Revives Available: # (augmented)",
    "Item Rarity: +#% (augmented)",
    "Waystone Drop Chance: +#% (augmented)",
    "Pack Size: +#% (augmented)",
    "Monster Rarity: +#% (augmented)",
    "Monster Effectiveness: +#% (augmented)",
    "Item Level: #",
    "{ Prefix Modifier \"#\" (Tier: #) }",
    "{ Suffix Modifier \"#\" (Tier: #) }",
    "Unscalable Value",
    "Corrupted",
    "Can be used in a Map Device, allowing you to enter a Map. Waystones can only be used once.",
    // Not seen in Live 1; kept as guards.
    "Rarity: Unique",
    "Waystone Tier: #",
    "Rare Monsters: +#%",
    "Magic Monsters: +#%",
    "Unidentified",
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
