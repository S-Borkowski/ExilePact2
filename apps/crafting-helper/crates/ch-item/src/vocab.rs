//! Every word and fixed string the parser recognises, in one place. Most of
//! the equipment-specific shapes here are guesses until the owner's pasted
//! texts confirm them (`fixtures/README.md` § "Shapes that are guesses"): a
//! correction should change this file, not the parser.

use crate::model::{Marker, ModKind};

/// The section separator (measured).
pub(crate) const SEPARATOR: &str = "--------";
/// The first line of every item text (measured).
pub(crate) const ITEM_CLASS: &str = "Item Class:";
/// The rarity line (measured).
pub(crate) const RARITY: &str = "Rarity:";
/// Ends a property raised by a modifier (measured).
pub(crate) const AUGMENTED: &str = " (augmented)";
/// Ends a modifier line whose value cannot roll (measured, em dash).
pub(crate) const UNSCALABLE: &str = " — Unscalable Value";
/// Separates a header's family tags from the rest (guess, em dash).
pub(crate) const TAG_SEPARATOR: &str = " — ";
/// The word every modifier header carries (measured).
pub(crate) const MODIFIER_WORD: &str = "Modifier";
/// Opens a header's tier (measured).
pub(crate) const TIER_OPEN: &str = "(Tier: ";
/// The block form of requirements (guess).
pub(crate) const REQUIREMENTS_BLOCK: &str = "Requirements:";
/// A word inside a requirement entry the character does not meet
/// (measured: `108 (unmet) Dex`).
pub(crate) const UNMET: &str = "(unmet)";

/// Property keys with their own place in the model.
pub(crate) const KEY_ITEM_LEVEL: &str = "Item Level";
pub(crate) const KEY_QUALITY: &str = "Quality";
pub(crate) const KEY_REQUIRES: &str = "Requires";
pub(crate) const KEY_SOCKETS: &str = "Sockets";
pub(crate) const KEY_LEVEL: &str = "Level";

/// A trailing `(word)` on a modifier line (guesses).
pub(crate) fn marker(word: &str) -> Option<Marker> {
    match word {
        "implicit" => Some(Marker::Implicit),
        "rune" => Some(Marker::Rune),
        "enchant" => Some(Marker::Enchant),
        "fractured" => Some(Marker::Fractured),
        "desecrated" => Some(Marker::Desecrated),
        "crafted" => Some(Marker::Crafted),
        _ => None,
    }
}

/// The header word just before `Modifier` (`Prefix`, `Suffix` measured; the
/// rest guesses).
pub(crate) fn header_kind(word: &str) -> Option<ModKind> {
    match word {
        "Prefix" => Some(ModKind::Prefix),
        "Suffix" => Some(ModKind::Suffix),
        "Implicit" => Some(ModKind::Implicit),
        "Rune" => Some(ModKind::Rune),
        "Enchant" => Some(ModKind::Enchant),
        _ => None,
    }
}

/// Whole headers with no `Modifier` word, by the words before their tags
/// (measured: `{ Corruption Enhancement — Elemental, Cold, Resistance }`).
const HEADER_PHRASES: &[(&str, ModKind)] = &[("Corruption Enhancement", ModKind::Corruption)];

pub(crate) fn header_phrase(words: &str) -> Option<ModKind> {
    HEADER_PHRASES
        .iter()
        .find(|(phrase, _)| *phrase == words)
        .map(|&(_, kind)| kind)
}

/// Whether a header's text inside the braces starts with one of those
/// phrases.
pub(crate) fn starts_with_header_phrase(inner: &str) -> bool {
    HEADER_PHRASES
        .iter()
        .any(|(phrase, _)| inner.starts_with(phrase))
}

/// A header word before the kind word (measured: `Desecrated`; the rest
/// guesses).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Adjective {
    Fractured,
    Desecrated,
    Crafted,
    /// Implies desecrated.
    Unrevealed,
}

pub(crate) fn header_adjective(word: &str) -> Option<Adjective> {
    match word {
        "Fractured" => Some(Adjective::Fractured),
        "Desecrated" => Some(Adjective::Desecrated),
        "Crafted" => Some(Adjective::Crafted),
        "Unrevealed" => Some(Adjective::Unrevealed),
        _ => None,
    }
}

/// A whole line that sets an item flag (`Corrupted` measured; the others
/// guesses).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ItemFlag {
    Corrupted,
    Mirrored,
    Unidentified,
}

pub(crate) fn item_flag(line: &str) -> Option<ItemFlag> {
    match line {
        "Corrupted" => Some(ItemFlag::Corrupted),
        "Mirrored" => Some(ItemFlag::Mirrored),
        "Unidentified" => Some(ItemFlag::Unidentified),
        _ => None,
    }
}

/// The one recogniser for a modifier line that names an unrevealed
/// modifier instead of showing it (guess: `Unrevealed Desecrated Modifier`).
/// Takes the line's template, so markers are already removed.
pub(crate) fn names_unrevealed(template: &str) -> bool {
    template.starts_with("Unrevealed ") && template.ends_with(MODIFIER_WORD)
}

/// A requirement attribute, by its short or long name (guesses).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Attribute {
    Strength,
    Dexterity,
    Intelligence,
}

pub(crate) fn attribute(word: &str) -> Option<Attribute> {
    match word {
        "Str" | "Strength" => Some(Attribute::Strength),
        "Dex" | "Dexterity" => Some(Attribute::Dexterity),
        "Int" | "Intelligence" => Some(Attribute::Intelligence),
        _ => None,
    }
}
