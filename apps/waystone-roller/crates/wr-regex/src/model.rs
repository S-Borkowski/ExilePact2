//! A model of the game's stash and vendor search box, for tests only.
//!
//! It was written from a public tool's practice and the owner's word, and is
//! the oracle the generator's tests run their strings through. Live
//! procedure 1 checked the rules the generator relies on against the game
//! on 2026-10-28 (research note `## Search syntax`); `.` not crossing a line
//! and refusing lookaround remain the model's own choices.
//!
//! The rules it applies:
//!
//! - the search splits on spaces outside double quotes, and the quotes are
//!   removed;
//! - a term starting with `!` is negated;
//! - the rest of a term is a case-insensitive regular expression searched
//!   anywhere in the item's text, with `.` not crossing a line;
//! - an item is highlighted when every positive term matches and no negated
//!   one does;
//! - a pattern with lookaround (`(?=`, `(?!`, `(?<=`, `(?<!`) is refused,
//!   since the game's support for it is not established.

use std::fmt;

use regex::{Regex, RegexBuilder};

/// Why a search string could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelError {
    /// A `"` was opened and never closed.
    UnclosedQuote,
    /// A term was `!` alone.
    EmptyNegation,
    /// A term uses lookaround, which the model does not assume the game has.
    Lookaround { term: String },
    /// A term is not a valid regular expression.
    BadPattern { term: String, message: String },
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModelError::UnclosedQuote => f.write_str("a double quote is never closed"),
            ModelError::EmptyNegation => f.write_str("a term is a lone \"!\""),
            ModelError::Lookaround { term } => write!(f, "term {term:?} uses lookaround"),
            ModelError::BadPattern { term, message } => {
                write!(f, "term {term:?} is not a valid pattern: {message}")
            }
        }
    }
}

impl std::error::Error for ModelError {}

/// One term of a search: its text after quote removal, and its pattern.
#[derive(Debug, Clone)]
pub struct Term {
    pub text: String,
    pub negated: bool,
    pattern: Regex,
}

/// A parsed search string.
#[derive(Debug, Clone)]
pub struct Search {
    pub terms: Vec<Term>,
}

impl Search {
    /// Reads a search string the way the model assumes the game does.
    pub fn parse(search: &str) -> Result<Search, ModelError> {
        let mut raw: Vec<String> = Vec::new();
        let mut current = String::new();
        let mut quoted = false;
        for ch in search.chars() {
            match ch {
                '"' => quoted = !quoted,
                ' ' if !quoted => {
                    if !current.is_empty() {
                        raw.push(std::mem::take(&mut current));
                    }
                }
                _ => current.push(ch),
            }
        }
        if quoted {
            return Err(ModelError::UnclosedQuote);
        }
        if !current.is_empty() {
            raw.push(current);
        }

        let mut terms = Vec::with_capacity(raw.len());
        for text in raw {
            let (negated, body) = match text.strip_prefix('!') {
                Some(rest) => (true, rest),
                None => (false, text.as_str()),
            };
            if body.is_empty() {
                return Err(ModelError::EmptyNegation);
            }
            if ["(?=", "(?!", "(?<=", "(?<!"]
                .iter()
                .any(|l| body.contains(l))
            {
                return Err(ModelError::Lookaround { term: text.clone() });
            }
            let pattern = RegexBuilder::new(body)
                .case_insensitive(true)
                .build()
                .map_err(|e| ModelError::BadPattern {
                    term: text.clone(),
                    message: e.to_string(),
                })?;
            terms.push(Term {
                text,
                negated,
                pattern,
            });
        }
        Ok(Search { terms })
    }

    /// Whether the search highlights an item with this text.
    pub fn highlights(&self, item_text: &str) -> bool {
        self.terms
            .iter()
            .all(|t| t.pattern.is_match(item_text) != t.negated)
    }
}

/// Parses `search` and tells whether it highlights `item_text`.
pub fn highlights(search: &str, item_text: &str) -> Result<bool, ModelError> {
    Ok(Search::parse(search)?.highlights(item_text))
}
