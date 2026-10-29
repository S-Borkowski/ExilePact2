//! The matcher: a tooltip read and the item the current step was shown on
//! in, one [`ReadKind`] out, and for a change the read item.
//!
//! Every line under the name lines must be accounted for, or the read is
//! unreadable: the matcher prefers unreadable to a guess. A line is
//! accounted for when it is
//!
//! - one of the item's own non-explicit lines: the item class as the
//!   tooltip names it (`Amulet`; `Amulet: Item Level 12` with Alt held), a
//!   property, the requirements, the item level, quality or sockets by the
//!   name before the colon (their values may change), an implicit, rune,
//!   enchant or corruption line by its template, a piece of a description
//!   line (the tooltip wraps a long one) or a flag. They are matched first,
//!   each template line once, and every implicit line must be found;
//! - alone or with the next lines, an explicit template the base can roll
//!   (`valid_mods` for its variant and item level, normal and desecrated,
//!   both sides), numbers read as values, its tier read as [`tier_of`] says
//!   (an unchanged modifier keeps its tier; else the one tier of any item
//!   level whose ranges hold the values);
//! - an advanced-form header (`Prefix` or `Suffix Modifier`, a name, `Tier:
//!   n`), which gives the next modifier's side and tier;
//! - one of the fixed interface strings ([`INTERFACE_STRINGS`]), after its
//!   key caps.
//!
//! When more than one reading of the lines accounts for all of them and the
//! readings differ, the read is unreadable. An item with an unrevealed or a
//! fractured modifier, or a tooltip showing the line under an unrevealed one
//! ([`UNREVEALED_NOTE`]), is unreadable too: the tooltip draws an unrevealed
//! modifier as glyphs OCR does not read, and what it draws of a fractured
//! one is not established.
//!
//! What the shots measured (2026-10-29) and the tests pin: the plain
//! tooltip shows each modifier's line with its value and without its header
//! or range, the item class above the requirements, no item level; held
//! Alt adds the ranges, the item level beside the class, and the side and
//! tier at the box's edges (the reader drops those). A value's range may run
//! high to low (`12(15-10)%`, a reducing modifier): it is dropped like any
//! other, and a tier holds a value between its bounds in either order.

use std::collections::BTreeMap;

use ch_engine::ch_data::{self, ClassData, DataMod, Kind, Section, Tier};
use ch_engine::ch_item::{Item, ItemClass, ModKind, ModLine, Modifier, Rarity, Value};

use crate::text::{prepare, prepare_template, similarity, Prepared};
use crate::{TooltipBox, TooltipRead};

/// Two masked texts are the same line when [`similarity`] is at least this.
/// Pinned by the tests: `to fire resistance` against `to cold resistance`
/// scores 0.78, one wrong letter in a short line about 0.94.
pub const LINE_THRESHOLD: f64 = 0.85;

/// A template is a candidate for a line only when its score lies within this
/// of the best template's of the same line count: templates that differ in
/// one word (`Physical` and `Cold`) both pass [`LINE_THRESHOLD`].
pub const BEST_MARGIN: f64 = 0.05;

/// Fixed strings of the game's interface that a tooltip's box may show and
/// that say nothing of the item, read off the owner's screenshots
/// (2026-10-29): the key hints the game draws over or beside a hovered
/// item's tooltip, each after its key caps ([`KEY_CAPS`]). A line is one of
/// them when, its key caps dropped and its spaces taken out (OCR spaces the
/// small letters: `P R t C E CHEC K`), it is alike one of these.
pub const INTERFACE_STRINGS: &[&str] = &["Inspect", "Compare", "Quick Move", "Price Check", "Use"];

/// The key caps drawn before a key hint.
pub const KEY_CAPS: &[&str] = &["Alt", "Shift", "Ctrl"];

/// The line the tooltip shows below an unrevealed desecrated modifier, whose
/// own line is a row of glyphs OCR does not read (measured, 2026-10-29).
pub const UNREVEALED_NOTE: &str =
    "Take this item to the Well of Souls to reveal the Desecrated Modifier";

/// A line counted as part of the item's description must be at least this
/// long (masked): the tooltip wraps a long description over several lines,
/// each a piece of it, and a short piece is alike too many texts.
const MIN_DESCRIPTION_PIECE: usize = 8;

/// Readings of one tooltip past which the matcher stops looking: the lines
/// are too ambiguous to be worth reading.
const MAX_READINGS: usize = 16;

/// What one read says of the item the step was shown on.
#[derive(Clone, Debug, PartialEq)]
pub enum ReadKind {
    /// No tooltip, or a black copy.
    NoTooltip,
    /// A tooltip lies under the overlay's window.
    Covered,
    /// No tooltip's name lines carry the item's base type.
    AnotherItem,
    /// The item's tooltip, but the lines cannot be trusted.
    Unreadable(Unreadable),
    /// The item's explicit modifiers (ids and tiers) and rarity are the
    /// item's.
    Unchanged,
    /// The item as read: the step's item with the read rarity and explicit
    /// modifiers.
    Changed(Box<Item>),
}

impl ReadKind {
    /// The state as the `watch: tooltip <state>` line writes it.
    pub fn state(&self) -> &'static str {
        match self {
            ReadKind::NoTooltip => "none",
            ReadKind::Covered => "covered by the overlay",
            ReadKind::AnotherItem => "another item",
            ReadKind::Unreadable(_) => "unreadable",
            ReadKind::Unchanged => "unchanged",
            ReadKind::Changed(_) => "changed",
        }
    }
}

/// Why the item's tooltip could not be trusted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Unreadable {
    /// A line is not accounted for.
    UnaccountedLine,
    /// One of the item's implicit lines was not read.
    MissingLine,
    /// A modifier's values fit more than one tier.
    AmbiguousTier,
    /// The lines read more than one way.
    Ambiguous,
    /// The name lines' colour is not read, or does not agree with the
    /// number of name lines or of modifiers.
    Rarity,
    /// The item has no base type, or one the data does not hold.
    NoBaseType,
    /// The item has no item level.
    NoItemLevel,
    /// The tooltip touches the copy's edge and may be cut.
    Cut,
    /// More than one tooltip carries the item's base type.
    SeveralTooltips,
    /// An unrevealed or fractured modifier: what the tooltip shows of one is
    /// not established.
    NotEstablished,
}

impl Unreadable {
    /// The reason in words, for a log line.
    pub fn as_str(self) -> &'static str {
        match self {
            Unreadable::UnaccountedLine => "a line is not accounted for",
            Unreadable::MissingLine => "an implicit line is missing",
            Unreadable::AmbiguousTier => "a tier is ambiguous",
            Unreadable::Ambiguous => "the lines read more than one way",
            Unreadable::Rarity => "the rarity cannot be told",
            Unreadable::NoBaseType => "the item has no known base type",
            Unreadable::NoItemLevel => "the item has no item level",
            Unreadable::Cut => "the tooltip is cut by the window's edge",
            Unreadable::SeveralTooltips => "two tooltips carry the base type",
            Unreadable::NotEstablished => "an unrevealed or fractured modifier",
        }
    }
}

/// An item's rarity and its explicit modifiers as (data id, tier), sorted:
/// what decides whether two items are the same for the guide.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemKey {
    pub rarity: Rarity,
    pub mods: Vec<(String, u32)>,
}

/// The item's key, from `ch_data::join`; `None` when an explicit modifier
/// does not join (an unrevealed one, or a base the data does not hold).
pub fn item_key(item: &Item) -> Option<ItemKey> {
    let joined = ch_data::join(item);
    let mut mods = joined
        .mods
        .iter()
        .map(|m| m.matched().map(|found| (found.mod_id.clone(), found.tier)))
        .collect::<Option<Vec<_>>>()?;
    mods.sort();
    Some(ItemKey {
        rarity: item.rarity.clone(),
        mods,
    })
}

/// Classify a tooltip read against the item the current step was shown on.
pub fn classify(read: &TooltipRead, item: &Item) -> ReadKind {
    match read {
        TooltipRead::NoTooltip | TooltipRead::Black => ReadKind::NoTooltip,
        TooltipRead::Covered => ReadKind::Covered,
        TooltipRead::Boxes(boxes) => match classify_boxes(boxes, item) {
            Ok(kind) => kind,
            Err(why) => ReadKind::Unreadable(why),
        },
    }
}

fn classify_boxes(boxes: &[TooltipBox], item: &Item) -> Result<ReadKind, Unreadable> {
    let base = item.base_type.as_deref().ok_or(Unreadable::NoBaseType)?;
    let base = prepare(base).masked;
    let carrying: Vec<&TooltipBox> = boxes.iter().filter(|b| carries(b, &base)).collect();
    let tip = match carrying.as_slice() {
        [] => return Ok(ReadKind::AnotherItem),
        [one] => *one,
        _ => return Err(Unreadable::SeveralTooltips),
    };
    if tip.touches_edge {
        return Err(Unreadable::Cut);
    }
    let (data, variant) = ch_data::variant_of(item).map_err(|_| Unreadable::NoBaseType)?;
    let item_level = item
        .item_level
        .as_ref()
        .ok_or(Unreadable::NoItemLevel)?
        .value;
    let rarity = read_rarity(tip)?;

    let lines: Vec<Prepared> = tip.lines.iter().map(|l| prepare(&l.text)).collect();
    let headers: Vec<Option<Header>> = lines.iter().map(header_of).collect();
    // An unrevealed modifier shows as glyphs OCR does not read, so neither
    // an item that has one nor a tooltip that shows one can be read; what a
    // fractured modifier looks like is not established.
    let note = prepare(UNREVEALED_NOTE).masked;
    if headers.iter().flatten().any(|h| h.not_established)
        || item.has_unrevealed()
        || item.has_fractured()
        || lines
            .iter()
            .any(|l| similarity(&l.masked, &note) >= LINE_THRESHOLD)
    {
        return Err(Unreadable::NotEstablished);
    }
    let reads = read_mods(&lines, &headers, item, data, variant, item_level)?;

    let prefixes = reads.iter().filter(|r| r.data.kind == Kind::Prefix).count();
    let suffixes = reads.len() - prefixes;
    let fits = match rarity {
        Rarity::Normal => reads.is_empty(),
        Rarity::Magic => prefixes <= 1 && suffixes <= 1,
        _ => true,
    };
    if !fits {
        return Err(Unreadable::Rarity);
    }

    let mut read_key: Vec<(String, u32)> = reads
        .iter()
        .map(|r| (r.data.id.clone(), r.tier.tier))
        .collect();
    read_key.sort();
    let key = ItemKey {
        rarity: rarity.clone(),
        mods: read_key,
    };
    if item_key(item).as_ref() == Some(&key) {
        return Ok(ReadKind::Unchanged);
    }
    Ok(ReadKind::Changed(Box::new(read_item(item, rarity, &reads))))
}

/// Whether a box's name lines carry the base type: a normal item's line is
/// it, a magic item's one line contains it, a rare's second line is it.
fn carries(tip: &TooltipBox, base: &str) -> bool {
    tip.name_lines.iter().any(|l| {
        let line = prepare(&l.text).masked;
        qt_screen::fuzzy_score(base, &line) >= LINE_THRESHOLD
    })
}

/// The rarity the name lines' colour says, when the number of name lines
/// agrees with it.
fn read_rarity(tip: &TooltipBox) -> Result<Rarity, Unreadable> {
    let lines = tip.name_lines.len();
    match &tip.rarity {
        Some(r @ (Rarity::Normal | Rarity::Magic)) if lines == 1 => Ok(r.clone()),
        Some(Rarity::Rare) if lines == 2 => Ok(Rarity::Rare),
        _ => Err(Unreadable::Rarity),
    }
}

/// An advanced-form header read off one line.
#[derive(Clone, Debug, PartialEq)]
struct Header {
    /// The side, for a prefix or suffix header; `None` for an implicit,
    /// rune, enchant or corruption one.
    side: Option<Kind>,
    /// The quoted affix name, masked.
    name: Option<String>,
    tier: Option<u32>,
    desecrated: bool,
    not_established: bool,
}

/// The words a header may start with before `Modifier`.
const HEADER_WORDS: &[&str] = &[
    "prefix",
    "suffix",
    "implicit",
    "rune",
    "enchant",
    "desecrated",
    "fractured",
    "unrevealed",
    "crafted",
    "corruption",
];

fn word_is(word: &str, want: &str) -> bool {
    similarity(word, want) >= LINE_THRESHOLD
}

fn header_of(line: &Prepared) -> Option<Header> {
    let words: Vec<&str> = line.masked.split(' ').collect();
    if words.len() >= 2 && word_is(words[0], "corruption") && word_is(words[1], "enhancement") {
        return Some(Header {
            side: None,
            name: None,
            tier: None,
            desecrated: false,
            not_established: false,
        });
    }
    let at = words
        .iter()
        .take(4)
        .position(|w| word_is(w, "modifier"))
        .filter(|&at| at >= 1)?;
    let lead = &words[..at];
    let is = |want: &str| lead.iter().any(|w| word_is(w, want));
    if !lead
        .iter()
        .all(|w| HEADER_WORDS.iter().any(|h| word_is(w, h)))
    {
        return None;
    }
    let side = if is("prefix") {
        Some(Kind::Prefix)
    } else if is("suffix") {
        Some(Kind::Suffix)
    } else {
        None
    };
    let after = &words[at + 1..];
    let tier_at = after.iter().position(|w| word_is(w, "tier"));
    let name = after[..tier_at.unwrap_or(after.len())].join(" ");
    let tier = tier_at
        .is_some()
        .then(|| line.values.first().copied())
        .flatten()
        .filter(|v| *v >= 1.0 && v.fract() == 0.0)
        .map(|v| v as u32);
    Some(Header {
        side,
        name: (side.is_some() && !name.is_empty()).then_some(name),
        tier,
        desecrated: is("desecrated"),
        not_established: is("unrevealed") || is("fractured"),
    })
}

/// A non-explicit line of the item: a keyed line (`Requires`, `Item
/// Level`, a property, the item class), a whole template line, or a piece
/// of the description.
struct Own {
    keys: Vec<String>,
    /// Masked templates, with whether each must be found (implicit lines).
    templates: Vec<(String, bool)>,
    /// The description's lines, masked: the tooltip wraps them.
    description: Vec<String>,
}

/// The item class as the tooltip names it under the name lines, one item of
/// it (`Amulet`, `Quarterstaff`, `Two Hand Mace` in the shots; the others by
/// the same rule). The Alt-held form writes `Amulet: Item Level 12`.
fn class_name(class: &ItemClass) -> String {
    let one = match class {
        ItemClass::BodyArmours => "Body Armour",
        ItemClass::Helmets => "Helmet",
        ItemClass::Quivers => "Quiver",
        ItemClass::Shields => "Shield",
        ItemClass::Bucklers => "Buckler",
        ItemClass::Foci => "Focus",
        ItemClass::Wands => "Wand",
        ItemClass::OneHandMaces => "One Hand Mace",
        ItemClass::Sceptres => "Sceptre",
        ItemClass::Spears => "Spear",
        ItemClass::Bows => "Bow",
        ItemClass::Staves => "Staff",
        ItemClass::TwoHandMaces => "Two Hand Mace",
        ItemClass::Quarterstaves => "Quarterstaff",
        ItemClass::Crossbows => "Crossbow",
        ItemClass::Talismans => "Talisman",
        ItemClass::Jewels => "Jewel",
        ItemClass::Belts => "Belt",
        ItemClass::Amulets => "Amulet",
        ItemClass::Rings => "Ring",
        ItemClass::Gloves | ItemClass::Boots => class.as_str(),
        ItemClass::Other(text) => text.strip_suffix('s').unwrap_or(text),
    };
    one.to_string()
}

/// Whether the line is one of the [`INTERFACE_STRINGS`], after its
/// [`KEY_CAPS`].
fn is_interface(line: &str) -> bool {
    let words: Vec<&str> = line
        .split(' ')
        .skip_while(|w| KEY_CAPS.iter().any(|k| word_is(w, &k.to_lowercase())))
        .collect();
    let compact = words.concat();
    !compact.is_empty()
        && INTERFACE_STRINGS.iter().any(|s| {
            let want = prepare(s).masked.replace(' ', "");
            similarity(&compact, &want) >= INTERFACE_THRESHOLD
        })
}

/// Two interface strings with their spaces taken out are alike at this.
/// Pinned by the tests: `prtcecheck` against `pricecheck` scores 0.9.
const INTERFACE_THRESHOLD: f64 = 0.8;

fn own_lines(item: &Item) -> Own {
    let mut keys: Vec<String> = Vec::new();
    let mut key = |k: &str| keys.push(prepare(k).masked);
    key(&class_name(&item.class));
    if item.requirements.is_some() {
        for k in ["Requires", "Requirements", "Level", "Str", "Dex", "Int"] {
            key(k);
        }
    }
    if item.item_level.is_some() {
        key("Item Level");
    }
    if item.quality.is_some() {
        key("Quality");
    }
    if item.sockets.is_some() {
        key("Sockets");
    }
    for p in &item.properties {
        key(&p.name);
    }
    let mut templates = Vec::new();
    for m in &item.modifiers {
        let required = m.kind == ModKind::Implicit;
        if matches!(m.kind, ModKind::Prefix | ModKind::Suffix) {
            continue;
        }
        for l in &m.lines {
            templates.push((prepare_template(&l.template).0, required));
        }
    }
    let description = item
        .description
        .iter()
        .map(|l| prepare(&l.text).masked)
        .collect();
    for (flag, on) in [
        ("Corrupted", item.corrupted),
        ("Mirrored", item.mirrored),
        ("Unidentified", item.unidentified),
    ] {
        if on {
            templates.push((prepare(flag).masked, false));
        }
    }
    Own {
        keys,
        templates,
        description,
    }
}

/// Whether the line is a piece of one of the description's lines.
fn in_description(line: &str, description: &[String]) -> bool {
    line.chars().count() >= MIN_DESCRIPTION_PIECE
        && description
            .iter()
            .any(|d| qt_screen::fuzzy_score(line, d) >= LINE_THRESHOLD)
}

/// Whether the line's first words are `key`.
fn starts_with_key(line: &str, key: &str) -> bool {
    let n = key.split(' ').count();
    let head: Vec<&str> = line.split(' ').take(n).collect();
    head.len() == n && similarity(&head.join(" "), key) >= LINE_THRESHOLD
}

/// One explicit modifier as read.
#[derive(Clone, Debug)]
struct ReadMod {
    data: &'static DataMod,
    tier: &'static Tier,
    values: Vec<f64>,
    desecrated: bool,
}

/// A modifier that may start at a line: its data, its line count, its
/// values, and every tier it has on the variant.
struct Span {
    data: &'static DataMod,
    len: usize,
    values: Vec<f64>,
    tiers: Vec<&'static Tier>,
}

/// One of the item's explicit modifiers as the data joins it: its id, its
/// tier and its values.
type Known = (String, u32, Vec<f64>);

/// The tier a span reads as under `header`: by the header's affix name, else
/// its tier number; else, when the item has a modifier of the same id with
/// the same values, that modifier's tier (the modifier is unchanged); else
/// the one tier whose ranges hold the values. Every tier on the variant
/// counts, whatever its item level: an item can carry a tier above the one
/// its item level allows (shot 03, 2026-10-29: `Gleaming`, item level 46 in
/// the data, on an item of level 43), and the plain form cannot tell
/// overlapping tiers apart. `Err` when several fit, `Ok(None)` when none
/// does.
fn tier_of(
    span: &Span,
    header: Option<&Header>,
    known: &[Known],
) -> Result<Option<&'static Tier>, ()> {
    let fitting: Vec<&'static Tier> = span
        .tiers
        .iter()
        .copied()
        .filter(|t| holds(t, &span.values))
        .collect();
    if let Some(h) = header {
        if let Some(name) = &h.name {
            let named: Vec<&'static Tier> = fitting
                .iter()
                .copied()
                .filter(|t| similarity(&prepare(&t.name).masked, name) >= LINE_THRESHOLD)
                .collect();
            if let [one] = named.as_slice() {
                return Ok(Some(one));
            }
        }
        if let Some(n) = h.tier {
            return Ok(fitting.iter().copied().find(|t| t.tier == n));
        }
    }
    let same = known
        .iter()
        .find(|(id, _, values)| *id == span.data.id && same_values(values, &span.values));
    if let Some((_, n, _)) = same {
        if let Some(t) = fitting.iter().copied().find(|t| t.tier == *n) {
            return Ok(Some(t));
        }
    }
    match fitting.as_slice() {
        [] => Ok(None),
        [one] => Ok(Some(one)),
        _ => Err(()),
    }
}

/// The same values, in the same order.
fn same_values(a: &[f64], b: &[f64]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() <= 1e-6)
}

/// Every value lies within its tier's range (either bound order).
fn holds(t: &Tier, values: &[f64]) -> bool {
    const EPSILON: f64 = 1e-6;
    t.values.len() == values.len()
        && t.values.iter().zip(values).all(|([a, b], v)| {
            let (lo, hi) = if a <= b { (*a, *b) } else { (*b, *a) };
            *v >= lo - EPSILON && *v <= hi + EPSILON
        })
}

/// One reading of the lines: each modifier, and whether its tier was
/// ambiguous.
type Reading = Vec<(ReadMod, bool)>;

/// A modifier the base can roll: its data, each line's masked template and
/// value count, and every tier it has on the variant.
type Template = (&'static DataMod, Vec<(String, usize)>, Vec<&'static Tier>);

/// One way of reading every line, as it is built.
#[derive(Clone)]
struct Walk {
    mods: Reading,
    used: Vec<bool>,
}

struct Search<'a> {
    lines: &'a [Prepared],
    headers: &'a [Option<Header>],
    own: Own,
    /// For each line, the modifiers that may start there.
    spans: Vec<Vec<Span>>,
    /// The item's explicit modifiers, for [`tier_of`].
    known: Vec<Known>,
    readings: Vec<Reading>,
    missing: bool,
    overflow: bool,
}

fn read_mods(
    lines: &[Prepared],
    headers: &[Option<Header>],
    item: &Item,
    data: &'static ClassData,
    variant: &str,
    item_level: u32,
) -> Result<Vec<ReadMod>, Unreadable> {
    let valid = data.valid_mods(variant, item_level);
    let templates: Vec<Template> = valid
        .normal
        .iter()
        .chain(&valid.desecrated)
        .filter_map(|v| {
            let data_mod = data.find_mod(v.id)?;
            let masked = data_mod.lines.iter().map(|l| prepare_template(l)).collect();
            Some((
                data_mod,
                masked,
                data_mod.tiers_on(variant).iter().collect(),
            ))
        })
        .collect();
    let spans = (0..lines.len())
        .map(|i| {
            let scored: Vec<(f64, Span)> = templates
                .iter()
                .filter_map(|(data_mod, masked, tiers)| {
                    let rest = lines.get(i..i + masked.len())?;
                    let mut score = 1.0_f64;
                    for (line, (template, n)) in rest.iter().zip(masked) {
                        if line.values.len() != *n {
                            return None;
                        }
                        score = score.min(similarity(&line.masked, template));
                    }
                    (score >= LINE_THRESHOLD).then(|| {
                        let span = Span {
                            data: data_mod,
                            len: masked.len(),
                            values: rest.iter().flat_map(|l| l.values.clone()).collect(),
                            tiers: tiers.clone(),
                        };
                        (score, span)
                    })
                })
                .collect();
            // Of the templates of one length, only those as alike as the
            // best: `Adds # to # Physical Damage` read exactly is not also
            // `Adds # to # Cold Damage`, which shares all but one word.
            let best = |len: usize| {
                scored
                    .iter()
                    .filter(|(_, s)| s.len == len)
                    .map(|(score, _)| *score)
                    .fold(0.0, f64::max)
            };
            let bests: Vec<(usize, f64)> =
                scored.iter().map(|(_, s)| (s.len, best(s.len))).collect();
            scored
                .into_iter()
                .zip(bests)
                .filter(|((score, _), (_, best))| *score >= best - BEST_MARGIN)
                .map(|((_, span), _)| span)
                .collect()
        })
        .collect();
    let own = own_lines(item);
    let used = vec![false; own.templates.len()];
    let joined = ch_data::join(item);
    let known = joined
        .mods
        .iter()
        .filter_map(|m| {
            let found = m.matched()?;
            let values = item.modifiers[m.index]
                .lines
                .iter()
                .flat_map(|l| l.values.iter().map(|v| v.value))
                .collect();
            Some((found.mod_id.clone(), found.tier, values))
        })
        .collect();
    let mut search = Search {
        lines,
        headers,
        own,
        spans,
        known,
        readings: Vec::new(),
        missing: false,
        overflow: false,
    };
    search.walk(
        0,
        None,
        Walk {
            mods: Vec::new(),
            used,
        },
    );

    if search.overflow {
        return Err(Unreadable::Ambiguous);
    }
    let mut distinct: BTreeMap<Vec<(String, Option<u32>)>, Reading> = BTreeMap::new();
    for reading in search.readings {
        let key = reading
            .iter()
            .map(|(r, ambiguous)| (r.data.id.clone(), (!ambiguous).then_some(r.tier.tier)))
            .collect();
        distinct.entry(key).or_insert(reading);
    }
    let mut readings = distinct.into_values();
    match (readings.next(), readings.next()) {
        (None, _) if search.missing => Err(Unreadable::MissingLine),
        (None, _) => Err(Unreadable::UnaccountedLine),
        (Some(_), Some(_)) => Err(Unreadable::Ambiguous),
        (Some(reading), None) => {
            if reading.iter().any(|(_, ambiguous)| *ambiguous) {
                return Err(Unreadable::AmbiguousTier);
            }
            Ok(reading.into_iter().map(|(r, _)| r).collect())
        }
    }
}

impl Search<'_> {
    fn walk(&mut self, i: usize, pending: Option<&Header>, walk: Walk) {
        if self.overflow {
            return;
        }
        if i == self.lines.len() {
            if pending.is_some_and(|h| h.side.is_some()) {
                return;
            }
            let all_found = self
                .own
                .templates
                .iter()
                .zip(&walk.used)
                .all(|((_, required), used)| !required || *used);
            if !all_found {
                self.missing = true;
                return;
            }
            self.readings.push(walk.mods);
            if self.readings.len() > MAX_READINGS {
                self.overflow = true;
            }
            return;
        }
        let (lines, headers) = (self.lines, self.headers);
        let line = &lines[i];
        let explicit_pending = pending.filter(|h| h.side.is_some());

        // A header after an explicit header with nothing under it accounts
        // for nothing.
        if let Some(header) = &headers[i] {
            if explicit_pending.is_none() {
                self.walk(i + 1, Some(header), walk);
            }
            return;
        }

        if explicit_pending.is_none() {
            // The item's own lines first, each template line once.
            let unused = self
                .own
                .templates
                .iter()
                .enumerate()
                .find(|(n, (t, _))| !walk.used[*n] && similarity(&line.masked, t) >= LINE_THRESHOLD)
                .map(|(n, _)| n);
            if let Some(n) = unused {
                let mut next = walk;
                next.used[n] = true;
                self.walk(i + 1, None, next);
                return;
            }
            if self
                .own
                .keys
                .iter()
                .any(|k| starts_with_key(&line.masked, k))
                || in_description(&line.masked, &self.own.description)
                || is_interface(&line.masked)
            {
                self.walk(i + 1, None, walk);
                return;
            }
        }

        let header = explicit_pending;
        let mut options = Vec::new();
        for span in &self.spans[i] {
            if header.is_some_and(|h| h.side != Some(span.data.kind)) {
                continue;
            }
            let (tier, ambiguous) = match tier_of(span, header, &self.known) {
                Ok(Some(t)) => (t, false),
                Ok(None) => continue,
                Err(()) => (span.tiers[0], true),
            };
            let desecrated =
                header.is_some_and(|h| h.desecrated) || span.data.section == Section::Desecrated;
            options.push((
                span.len,
                ReadMod {
                    data: span.data,
                    tier,
                    values: span.values.clone(),
                    desecrated,
                },
                ambiguous,
            ));
        }
        for (len, read, ambiguous) in options {
            let mut next = walk.clone();
            next.mods.push((read, ambiguous));
            self.walk(i + len, None, next);
        }
    }
}

/// The step's item with the read rarity and explicit modifiers: each built
/// from its data template and the read values, with its tier's name and
/// number, the tags and fractured and desecrated marks of the item's
/// modifier with the same id. Every other field is the item's.
fn read_item(item: &Item, rarity: Rarity, reads: &[ReadMod]) -> Item {
    let joined = ch_data::join(item);
    let same_id = |id: &str| -> Option<&Modifier> {
        joined
            .mods
            .iter()
            .find(|m| m.matched().is_some_and(|found| found.mod_id == id))
            .map(|m| &item.modifiers[m.index])
    };
    let built: Vec<Modifier> = reads
        .iter()
        .map(|r| {
            let before = same_id(&r.data.id);
            let mut values = r.values.iter();
            let lines = r
                .data
                .lines
                .iter()
                .map(|template| {
                    let mut text = String::new();
                    let mut line_values = Vec::new();
                    for (n, part) in template.split('#').enumerate() {
                        if n > 0 {
                            let v = values.next().copied().unwrap_or_default();
                            text.push_str(&format_value(v));
                            line_values.push(Value {
                                value: v,
                                range: None,
                            });
                        }
                        text.push_str(part);
                    }
                    ModLine {
                        index: 0,
                        text,
                        template: template.clone(),
                        values: line_values,
                        unscalable: false,
                        markers: Vec::new(),
                    }
                })
                .collect();
            Modifier {
                kind: match r.data.kind {
                    Kind::Prefix => ModKind::Prefix,
                    Kind::Suffix => ModKind::Suffix,
                },
                affix_name: Some(r.tier.name.clone()),
                tier: Some(r.tier.tier),
                tags: before.map(|m| m.tags.clone()).unwrap_or_default(),
                fractured: before.is_some_and(|m| m.fractured),
                desecrated: before.map_or(r.desecrated, |m| m.desecrated),
                unrevealed: false,
                crafted: before.is_some_and(|m| m.crafted),
                header_line: None,
                lines,
            }
        })
        .collect();
    let mut out = item.clone();
    out.rarity = rarity;
    let at = out
        .modifiers
        .iter()
        .position(|m| matches!(m.kind, ModKind::Prefix | ModKind::Suffix))
        .unwrap_or(out.modifiers.len());
    out.modifiers
        .retain(|m| !matches!(m.kind, ModKind::Prefix | ModKind::Suffix));
    let at = at.min(out.modifiers.len());
    for (n, m) in built.into_iter().enumerate() {
        out.modifiers.insert(at + n, m);
    }
    out
}

fn format_value(v: f64) -> String {
    if v.fract() == 0.0 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}
