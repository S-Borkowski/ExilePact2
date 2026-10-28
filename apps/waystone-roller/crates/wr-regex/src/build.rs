//! The assembler: a selection becomes one search string of terms joined by a
//! space, in the order rarity, tier, want, avoid, corruption, or an outcome
//! saying why there is none. Nothing is ever truncated: a string over
//! [`LIMIT`] is reported with each part's length.

use serde::{Deserialize, Serialize};

use crate::data::ModData;
use crate::tokens::Tokens;

/// The game's search box limit, in characters. Exactly this many is accepted.
pub const LIMIT: usize = 250;
/// The lowest and highest waystone tier.
pub const TIER_MIN: u8 = 1;
pub const TIER_MAX: u8 = 16;

/// Whether an item needs any or all of the wanted mods.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WantMode {
    #[default]
    Any,
    All,
}

/// Which items the corruption term keeps.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Corruption {
    #[default]
    Any,
    Corrupted,
    Uncorrupted,
}

/// The rarities to keep; none or all of them adds no term.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rarity {
    pub normal: bool,
    pub magic: bool,
    pub rare: bool,
}

impl Default for Rarity {
    fn default() -> Rarity {
        Rarity {
            normal: true,
            magic: true,
            rare: true,
        }
    }
}

/// The player's rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Selection {
    pub want: Vec<String>,
    pub want_mode: WantMode,
    pub avoid: Vec<String>,
    pub tier_min: u8,
    pub tier_max: u8,
    pub rarity: Rarity,
    pub corruption: Corruption,
}

impl Default for Selection {
    fn default() -> Selection {
        Selection {
            want: Vec::new(),
            want_mode: WantMode::Any,
            avoid: Vec::new(),
            tier_min: TIER_MIN,
            tier_max: TIER_MAX,
            rarity: Rarity::default(),
            corruption: Corruption::Any,
        }
    }
}

/// Which rule a part of the string carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PartWhat {
    Rarity,
    Tier,
    Want,
    Avoid,
    Corruption,
}

/// One rule's text in the string, and its length in characters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Part {
    pub what: PartWhat,
    pub text: String,
    pub length: usize,
}

/// What a selection builds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The string, at most [`LIMIT`] characters, and its parts.
    Ok { regex: String, parts: Vec<Part> },
    /// Nothing is selected, so there is nothing to search for.
    Empty,
    /// The whole string would be `length` characters; every part is here,
    /// none shortened.
    TooLong { length: usize, parts: Vec<Part> },
    /// These selected families cannot be told apart from others (or are not
    /// in the data).
    Unmatchable { ids: Vec<String> },
}

/// Wraps a term in double quotes when it holds a space, `|`, `[`, `(` or `\`.
pub fn quote(term: &str) -> String {
    if term.contains([' ', '|', '[', '(', '\\']) {
        format!("\"{term}\"")
    } else {
        term.to_string()
    }
}

/// The rarity term: none for none or all of normal, magic and rare.
pub fn rarity_term(rarity: &Rarity) -> Option<String> {
    let letters: String = [
        (rarity.normal, 'n'),
        (rarity.magic, 'm'),
        (rarity.rare, 'r'),
    ]
    .iter()
    .filter(|(on, _)| *on)
    .map(|(_, l)| *l)
    .collect();
    match letters.len() {
        1 => Some(quote(&format!("y: {letters}"))),
        2 => Some(quote(&format!("y: [{letters}]"))),
        _ => None,
    }
}

/// Clamps both bounds to 1–16 and puts them in order.
fn tier_bounds(min: u8, max: u8) -> (u8, u8) {
    let a = min.clamp(TIER_MIN, TIER_MAX);
    let b = max.clamp(TIER_MIN, TIER_MAX);
    (a.min(b), a.max(b))
}

/// The tier term, matching the `(Tier N)` of the name line: none for 1–16.
/// Every alternative keeps the `r ` of "Tier": the search box also reads the
/// roll ranges, and a bare `15\)` matches `+14(11-15)%` on a tier-2 waystone
/// (measured 2026-10-28, Live 1).
pub fn tier_term(min: u8, max: u8) -> Option<String> {
    let (lo, hi) = tier_bounds(min, max);
    if (lo, hi) == (TIER_MIN, TIER_MAX) {
        return None;
    }
    let mut alts = Vec::new();
    if lo <= 9 {
        let b = hi.min(9);
        alts.push(if lo == b {
            format!("r {lo}\\)")
        } else {
            format!("r [{lo}-{b}]\\)")
        });
    }
    if hi >= 10 {
        let a = lo.max(10) - 10;
        let b = hi - 10;
        alts.push(if a == b {
            format!("r 1{a}\\)")
        } else {
            format!("r 1[{a}-{b}]\\)")
        });
    }
    Some(quote(&alts.join("|")))
}

/// The data and its tokens, picked once; builds any number of selections.
#[derive(Debug, Clone)]
pub struct Builder {
    data: ModData,
    tokens: Tokens,
}

impl Builder {
    pub fn new(data: ModData) -> Builder {
        let tokens = Tokens::pick(&data);
        Builder { data, tokens }
    }

    pub fn data(&self) -> &ModData {
        &self.data
    }

    pub fn tokens(&self) -> &Tokens {
        &self.tokens
    }

    /// Builds the search string for `selection`.
    pub fn build(&self, selection: &Selection) -> Outcome {
        let mut unknown: Vec<String> = Vec::new();
        for id in selection.want.iter().chain(&selection.avoid) {
            if self.data.family(id).is_none() && !unknown.contains(id) {
                unknown.push(id.clone());
            }
        }
        if !unknown.is_empty() {
            return Outcome::Unmatchable { ids: unknown };
        }

        let mut parts: Vec<(PartWhat, String)> = Vec::new();
        let mut bad: Vec<String> = Vec::new();
        if let Some(t) = rarity_term(&selection.rarity) {
            parts.push((PartWhat::Rarity, t));
        }
        if let Some(t) = tier_term(selection.tier_min, selection.tier_max) {
            parts.push((PartWhat::Tier, t));
        }
        if !selection.want.is_empty() {
            match selection.want_mode {
                WantMode::Any => match self.tokens.alternation(&selection.want) {
                    Ok(alts) => parts.push((PartWhat::Want, quote(&alts.join("|")))),
                    Err(ids) => bad.extend(ids),
                },
                WantMode::All => {
                    let mut positions: Vec<usize> = selection
                        .want
                        .iter()
                        .filter_map(|id| self.data.position(id))
                        .collect();
                    positions.sort_unstable();
                    positions.dedup();
                    let mut terms = Vec::new();
                    for p in positions {
                        match &self.tokens.all()[p].token {
                            Some(t) => terms.push(quote(t)),
                            None => bad.push(self.data.families[p].id.clone()),
                        }
                    }
                    parts.push((PartWhat::Want, terms.join(" ")));
                }
            }
        }
        if !selection.avoid.is_empty() {
            match self.tokens.alternation(&selection.avoid) {
                Ok(alts) => parts.push((PartWhat::Avoid, quote(&format!("!{}", alts.join("|"))))),
                Err(ids) => bad.extend(ids),
            }
        }
        match selection.corruption {
            Corruption::Any => {}
            Corruption::Corrupted => parts.push((PartWhat::Corruption, "corr".to_string())),
            Corruption::Uncorrupted => parts.push((PartWhat::Corruption, "!corr".to_string())),
        }

        if !bad.is_empty() {
            let mut ids: Vec<String> = Vec::new();
            for f in &self.data.families {
                if bad.contains(&f.id) && !ids.contains(&f.id) {
                    ids.push(f.id.clone());
                }
            }
            return Outcome::Unmatchable { ids };
        }
        if parts.is_empty() {
            return Outcome::Empty;
        }
        let parts: Vec<Part> = parts
            .into_iter()
            .map(|(what, text)| Part {
                what,
                length: text.chars().count(),
                text,
            })
            .collect();
        let regex = parts
            .iter()
            .map(|p| p.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        let length = regex.chars().count();
        if length > LIMIT {
            Outcome::TooLong { length, parts }
        } else {
            Outcome::Ok { regex, parts }
        }
    }
}
