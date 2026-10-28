//! The typed item model `parse` returns. Every element that came from the
//! text keeps the index of its input line (numbered as [`crate::split_lines`]
//! numbers them), so a caller can check that nothing was dropped.

use std::fmt;

/// Which copy the text came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form {
    /// `Ctrl+Alt+C`: a `{ … Modifier … }` header or a roll range was seen.
    Advanced,
    /// `Ctrl+C`: modifier lines, but neither headers nor roll ranges.
    Short,
    /// No modifier line at all (a plain normal item, an unidentified rare):
    /// the two forms cannot be told apart.
    Undetermined,
}

/// The `Item Class:` line: the 22 equipment classes the crafting helper
/// covers, or the text of any other class.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ItemClass {
    Gloves,
    Boots,
    BodyArmours,
    Helmets,
    Quivers,
    Shields,
    Bucklers,
    Foci,
    Wands,
    OneHandMaces,
    Sceptres,
    Spears,
    Bows,
    Staves,
    TwoHandMaces,
    Quarterstaves,
    Crossbows,
    Talismans,
    Jewels,
    Belts,
    Amulets,
    Rings,
    /// A class outside the 22 (`Waystones`, `Stackable Currency`, …).
    Other(String),
}

impl ItemClass {
    /// The 22 in-scope classes.
    pub const ALL: [ItemClass; 22] = [
        ItemClass::Gloves,
        ItemClass::Boots,
        ItemClass::BodyArmours,
        ItemClass::Helmets,
        ItemClass::Quivers,
        ItemClass::Shields,
        ItemClass::Bucklers,
        ItemClass::Foci,
        ItemClass::Wands,
        ItemClass::OneHandMaces,
        ItemClass::Sceptres,
        ItemClass::Spears,
        ItemClass::Bows,
        ItemClass::Staves,
        ItemClass::TwoHandMaces,
        ItemClass::Quarterstaves,
        ItemClass::Crossbows,
        ItemClass::Talismans,
        ItemClass::Jewels,
        ItemClass::Belts,
        ItemClass::Amulets,
        ItemClass::Rings,
    ];

    /// The class as the `Item Class:` line writes it (a guess for the
    /// equipment classes until the owner's pastes confirm them).
    pub fn as_str(&self) -> &str {
        match self {
            ItemClass::Gloves => "Gloves",
            ItemClass::Boots => "Boots",
            ItemClass::BodyArmours => "Body Armours",
            ItemClass::Helmets => "Helmets",
            ItemClass::Quivers => "Quivers",
            ItemClass::Shields => "Shields",
            ItemClass::Bucklers => "Bucklers",
            ItemClass::Foci => "Foci",
            ItemClass::Wands => "Wands",
            ItemClass::OneHandMaces => "One Hand Maces",
            ItemClass::Sceptres => "Sceptres",
            ItemClass::Spears => "Spears",
            ItemClass::Bows => "Bows",
            ItemClass::Staves => "Staves",
            ItemClass::TwoHandMaces => "Two Hand Maces",
            ItemClass::Quarterstaves => "Quarterstaves",
            ItemClass::Crossbows => "Crossbows",
            ItemClass::Talismans => "Talismans",
            ItemClass::Jewels => "Jewels",
            ItemClass::Belts => "Belts",
            ItemClass::Amulets => "Amulets",
            ItemClass::Rings => "Rings",
            ItemClass::Other(text) => text,
        }
    }

    /// The class named by `text`; `Other(text)` for anything else.
    pub fn from_text(text: &str) -> ItemClass {
        ItemClass::ALL
            .iter()
            .find(|c| c.as_str() == text)
            .cloned()
            .unwrap_or_else(|| ItemClass::Other(text.to_string()))
    }
}

/// The `Rarity:` line.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Rarity {
    Normal,
    Magic,
    Rare,
    Unique,
    /// Any other word (`Currency`, `Gem`), or empty when the text had no
    /// `Rarity:` line.
    Other(String),
}

impl Rarity {
    pub fn from_text(text: &str) -> Rarity {
        match text {
            "Normal" => Rarity::Normal,
            "Magic" => Rarity::Magic,
            "Rare" => Rarity::Rare,
            "Unique" => Rarity::Unique,
            other => Rarity::Other(other.to_string()),
        }
    }
}

/// One input line kept as text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub index: usize,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemLevel {
    pub value: u32,
    pub line: usize,
}

/// `Quality: +20% (augmented)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Quality {
    pub value: u32,
    pub augmented: bool,
    pub line: usize,
}

/// `Requires: Level 45, 63 Str`, or the block form `Requirements:` followed
/// by `Level: 45` and `Str: 63` lines.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Requirements {
    pub level: Option<u32>,
    pub strength: Option<u32>,
    pub dexterity: Option<u32>,
    pub intelligence: Option<u32>,
    /// Entries the parser did not recognise, as written (also reported as
    /// partly understood).
    pub other: Vec<String>,
    /// Every input line the requirements came from.
    pub lines: Vec<usize>,
}

/// `Sockets: S S`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sockets {
    /// The text after `Sockets:`.
    pub text: String,
    pub count: usize,
    pub line: usize,
}

/// A `Key: value` property line, in text order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Property {
    pub name: String,
    /// The value as written, without a trailing ` (augmented)`.
    pub value: String,
    pub augmented: bool,
    pub line: usize,
}

/// What a modifier is, from its header's kind word or a line marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModKind {
    Prefix,
    Suffix,
    Implicit,
    Rune,
    Enchant,
    /// The text does not say (an explicit modifier in the short form), or
    /// the header's kind word is not in the vocabulary (then reported).
    Unknown,
}

/// A trailing `(word)` on a modifier line that the vocabulary knows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Marker {
    Implicit,
    Rune,
    Enchant,
    Fractured,
    Desecrated,
    Crafted,
}

impl Marker {
    /// The modifier kind this marker states, if it states one.
    pub fn kind(self) -> Option<ModKind> {
        match self {
            Marker::Implicit => Some(ModKind::Implicit),
            Marker::Rune => Some(ModKind::Rune),
            Marker::Enchant => Some(ModKind::Enchant),
            Marker::Fractured | Marker::Desecrated | Marker::Crafted => None,
        }
    }

    /// The marker as the line writes it, without parentheses.
    pub fn as_str(self) -> &'static str {
        match self {
            Marker::Implicit => "implicit",
            Marker::Rune => "rune",
            Marker::Enchant => "enchant",
            Marker::Fractured => "fractured",
            Marker::Desecrated => "desecrated",
            Marker::Crafted => "crafted",
        }
    }
}

/// A roll range `(min-max)` written straight after a number.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Range {
    pub min: f64,
    pub max: f64,
}

/// One number on a modifier line, with its roll range when the text shows
/// one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Value {
    pub value: f64,
    pub range: Option<Range>,
}

/// One line of a modifier.
#[derive(Debug, Clone, PartialEq)]
pub struct ModLine {
    pub index: usize,
    /// The line as shown.
    pub text: String,
    /// The line with each value (and its range) replaced by `#`, the markers
    /// and ` — Unscalable Value` removed; a leading `+` stays as text.
    pub template: String,
    pub values: Vec<Value>,
    /// The line ended ` — Unscalable Value`.
    pub unscalable: bool,
    /// Known markers the line carried, in text order.
    pub markers: Vec<Marker>,
}

/// One modifier: a header and the lines under it, or a single line with no
/// header.
#[derive(Debug, Clone, PartialEq)]
pub struct Modifier {
    pub kind: ModKind,
    /// The quoted affix name in the header (`"Hale"`, `"of the Fox"`).
    pub affix_name: Option<String>,
    pub tier: Option<u32>,
    /// Family tags after ` — ` in the header.
    pub tags: Vec<String>,
    pub fractured: bool,
    pub desecrated: bool,
    /// Not yet revealed; an unrevealed modifier is always desecrated.
    pub unrevealed: bool,
    pub crafted: bool,
    pub header_line: Option<usize>,
    /// The modifier's lines; empty for a header with nothing under it.
    pub lines: Vec<ModLine>,
}

impl Modifier {
    pub(crate) fn bare(kind: ModKind) -> Modifier {
        Modifier {
            kind,
            affix_name: None,
            tier: None,
            tags: Vec::new(),
            fractured: false,
            desecrated: false,
            unrevealed: false,
            crafted: false,
            header_line: None,
            lines: Vec::new(),
        }
    }
}

/// A line the parser did not consume.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unparsed {
    /// Index of the input line.
    pub line: usize,
    /// Index of its `--------` section (0 is the class/rarity/name block).
    pub section: usize,
    pub text: String,
    pub reason: String,
}

/// A line the parser consumed but did not understand all of.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Partial {
    pub line: usize,
    pub section: usize,
    pub text: String,
    /// The part of the line not understood.
    pub part: String,
    pub reason: String,
}

/// Everything the parser did not fully understand.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    pub unparsed: Vec<Unparsed>,
    pub partial: Vec<Partial>,
}

/// One item, as its copied text describes it.
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub form: Form,
    pub class: ItemClass,
    pub class_line: usize,
    pub rarity: Rarity,
    pub rarity_line: Option<usize>,
    /// A rare or unique item's random name.
    pub name: Option<String>,
    /// The base type when the text lets it be derived: not for a magic item
    /// in the short form, whose affix names cannot be told from its base.
    pub base_type: Option<String>,
    /// The name lines as written.
    pub name_lines: Vec<Line>,
    pub item_level: Option<ItemLevel>,
    pub quality: Option<Quality>,
    pub requirements: Option<Requirements>,
    pub sockets: Option<Sockets>,
    pub properties: Vec<Property>,
    pub description: Vec<Line>,
    /// Every modifier, in text order.
    pub modifiers: Vec<Modifier>,
    pub corrupted: bool,
    pub mirrored: bool,
    pub unidentified: bool,
    /// The `Corrupted` / `Mirrored` / `Unidentified` lines.
    pub flag_lines: Vec<usize>,
    pub report: Report,
}

impl Item {
    pub fn has_fractured(&self) -> bool {
        self.modifiers.iter().any(|m| m.fractured)
    }

    pub fn has_desecrated(&self) -> bool {
        self.modifiers.iter().any(|m| m.desecrated)
    }

    pub fn has_unrevealed(&self) -> bool {
        self.modifiers.iter().any(|m| m.unrevealed)
    }

    pub fn prefix_count(&self) -> usize {
        self.count_kind(ModKind::Prefix)
    }

    pub fn suffix_count(&self) -> usize {
        self.count_kind(ModKind::Suffix)
    }

    fn count_kind(&self, kind: ModKind) -> usize {
        self.modifiers.iter().filter(|m| m.kind == kind).count()
    }

    /// The index of every input line consumed into the model, sorted. With
    /// `report.unparsed` it covers every non-blank, non-separator line
    /// exactly once.
    pub fn consumed_lines(&self) -> Vec<usize> {
        let mut out = vec![self.class_line];
        out.extend(self.rarity_line);
        out.extend(self.name_lines.iter().map(|l| l.index));
        out.extend(self.item_level.as_ref().map(|l| l.line));
        out.extend(self.quality.as_ref().map(|q| q.line));
        if let Some(r) = &self.requirements {
            out.extend(&r.lines);
        }
        out.extend(self.sockets.as_ref().map(|s| s.line));
        out.extend(self.properties.iter().map(|p| p.line));
        out.extend(self.description.iter().map(|l| l.index));
        for m in &self.modifiers {
            out.extend(m.header_line);
            out.extend(m.lines.iter().map(|l| l.index));
        }
        out.extend(&self.flag_lines);
        out.sort_unstable();
        out
    }
}

/// Why `parse` returned no item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// The first non-blank line is not `Item Class: …`.
    NotItemText,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::NotItemText => {
                f.write_str("not item text: the first line is not `Item Class: …`")
            }
        }
    }
}

impl std::error::Error for ParseError {}
