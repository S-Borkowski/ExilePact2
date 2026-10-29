//! The grammar of one line: a modifier header, and a modifier line's values,
//! roll ranges, markers and template. Pure functions of the line's text.

use crate::model::{Marker, ModKind, Range, Value};
use crate::vocab::{self, Adjective};

/// A part of a line that was not understood, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Note {
    pub part: String,
    pub reason: String,
}

impl Note {
    fn new(part: impl Into<String>, reason: &str) -> Note {
        Note {
            part: part.into(),
            reason: reason.to_string(),
        }
    }
}

/// `{ … }`: a line the parser reads as a modifier header.
pub(crate) fn is_header(line: &str) -> bool {
    line.starts_with('{') && line.ends_with('}')
}

/// A header that marks the advanced form: braces and the `Modifier` word, or
/// a known header phrase without it (`{ Corruption Enhancement }`).
pub(crate) fn is_modifier_header(line: &str) -> bool {
    is_header(line)
        && (line.contains(vocab::MODIFIER_WORD)
            || vocab::starts_with_header_phrase(line[1..line.len() - 1].trim()))
}

/// What a header says about its modifier.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Header {
    pub kind: ModKind,
    pub affix_name: Option<String>,
    pub tier: Option<u32>,
    pub tags: Vec<String>,
    pub fractured: bool,
    pub desecrated: bool,
    pub unrevealed: bool,
    pub crafted: bool,
    pub notes: Vec<Note>,
}

/// `{ [adjective …] Kind Modifier ["name"] [(Tier: N)] [— tag, tag] }`, or
/// a known phrase without `Modifier` (`{ Corruption Enhancement — tag }`).
pub(crate) fn parse_header(line: &str) -> Header {
    let mut h = Header {
        kind: ModKind::Unknown,
        affix_name: None,
        tier: None,
        tags: Vec::new(),
        fractured: false,
        desecrated: false,
        unrevealed: false,
        crafted: false,
        notes: Vec::new(),
    };
    let inner = line[1..line.len() - 1].trim();
    let main = match inner.split_once(vocab::TAG_SEPARATOR) {
        Some((main, tags)) => {
            h.tags = tags
                .split(',')
                .map(str::trim)
                .filter(|t| !t.is_empty())
                .map(str::to_string)
                .collect();
            main.trim()
        }
        None => inner,
    };

    // The words up to `Modifier`, stopping at a quoted name or a parenthesis.
    let mut words: Vec<&str> = Vec::new();
    let mut rest = main;
    let mut saw_modifier = false;
    loop {
        rest = rest.trim_start();
        if rest.is_empty() || rest.starts_with('"') || rest.starts_with('(') {
            break;
        }
        let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
        let word = &rest[..end];
        rest = &rest[end..];
        if word == vocab::MODIFIER_WORD {
            saw_modifier = true;
            break;
        }
        words.push(word);
    }

    if !saw_modifier {
        let phrase = words.join(" ");
        match vocab::header_phrase(&phrase) {
            Some(kind) => h.kind = kind,
            None => h
                .notes
                .push(Note::new(phrase, "header has no `Modifier` word")),
        }
    } else if let Some((&kind_word, adjectives)) = words.split_last() {
        match vocab::header_kind(kind_word) {
            Some(kind) => h.kind = kind,
            None => h
                .notes
                .push(Note::new(kind_word, "unknown header kind word")),
        }
        for &word in adjectives {
            match vocab::header_adjective(word) {
                Some(Adjective::Fractured) => h.fractured = true,
                Some(Adjective::Desecrated) => h.desecrated = true,
                Some(Adjective::Crafted) => h.crafted = true,
                Some(Adjective::Unrevealed) => {
                    h.unrevealed = true;
                    h.desecrated = true;
                }
                None => h.notes.push(Note::new(word, "unknown header word")),
            }
        }
    } else {
        h.notes.push(Note::new(
            vocab::MODIFIER_WORD,
            "header has no kind word before `Modifier`",
        ));
    }

    rest = rest.trim_start();
    if let Some(quoted) = rest.strip_prefix('"') {
        match quoted.find('"') {
            Some(close) => {
                h.affix_name = Some(quoted[..close].to_string());
                rest = quoted[close + 1..].trim_start();
            }
            None => {
                h.notes
                    .push(Note::new(rest, "affix name has no closing quote"));
                rest = "";
            }
        }
    }
    if let Some(after) = rest.strip_prefix(vocab::TIER_OPEN) {
        if let Some(close) = after.find(')') {
            if let Ok(tier) = after[..close].trim().parse::<u32>() {
                h.tier = Some(tier);
                rest = after[close + 1..].trim_start();
            }
        }
    }
    if !rest.is_empty() {
        h.notes.push(Note::new(rest, "header text not understood"));
    }
    h
}

/// A modifier line taken apart.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ModText {
    pub template: String,
    pub values: Vec<Value>,
    pub unscalable: bool,
    pub markers: Vec<Marker>,
    pub notes: Vec<Note>,
}

/// Takes the trailing markers and ` — Unscalable Value` off a modifier line,
/// then replaces each value (and its roll range) with `#`.
pub(crate) fn parse_mod_text(line: &str) -> ModText {
    let mut body = line;
    let mut markers = Vec::new();
    let mut unscalable = false;
    let mut notes = Vec::new();
    loop {
        if let Some(before) = body.strip_suffix(vocab::UNSCALABLE) {
            unscalable = true;
            body = before.trim_end();
            continue;
        }
        if let Some((before, word)) = trailing_paren_word(body) {
            if let Some(m) = vocab::marker(word) {
                markers.push(m);
                body = before;
                continue;
            }
            notes.push(Note::new(format!("({word})"), "unknown line marker"));
        }
        break;
    }
    markers.reverse();
    let (template, values, value_notes) = scan_values(body);
    notes.extend(value_notes);
    ModText {
        template,
        values,
        unscalable,
        markers,
        notes,
    }
}

/// Whether a line carries a known marker or a roll range: a modifier line
/// even without a header.
pub(crate) fn looks_like_modifier(line: &str) -> bool {
    let m = parse_mod_text(line);
    !m.markers.is_empty() || m.values.iter().any(|v| v.range.is_some())
}

/// Whether a line carries a roll range anywhere.
pub(crate) fn has_roll_range(line: &str) -> bool {
    parse_mod_text(line)
        .values
        .iter()
        .any(|v| v.range.is_some())
}

/// ` (word)` at the end of `s`, where the word is letters only.
fn trailing_paren_word(s: &str) -> Option<(&str, &str)> {
    let inner = s.strip_suffix(')')?;
    let open = inner.rfind('(')?;
    let word = &inner[open + 1..];
    if word.is_empty() || !word.chars().all(|c| c.is_ascii_alphabetic()) {
        return None;
    }
    let before = inner[..open].strip_suffix(' ')?;
    Some((before.trim_end(), word))
}

/// Replaces each number, with the roll range written straight after it, by
/// `#`. A `-` is a sign only where a number may start (not after a digit or
/// letter), so `3-5` is two values and a literal dash.
fn scan_values(body: &str) -> (String, Vec<Value>, Vec<Note>) {
    let chars: Vec<char> = body.chars().collect();
    let mut template = String::new();
    let mut values = Vec::new();
    let mut notes = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let boundary = i == 0 || !(chars[i - 1].is_alphanumeric() || chars[i - 1] == '.');
        let digit_at = |k: usize| chars.get(k).is_some_and(char::is_ascii_digit);
        let starts_number = boundary && (digit_at(i) || (chars[i] == '-' && digit_at(i + 1)));
        if !starts_number {
            template.push(chars[i]);
            i += 1;
            continue;
        }
        let (value, end) = number(&chars, i);
        template.push('#');
        if chars.get(end) == Some(&'(') {
            if let Some((range, after)) = range(&chars, end) {
                values.push(Value {
                    value,
                    range: Some(range),
                });
                i = after;
                continue;
            }
            let close = chars[end..]
                .iter()
                .position(|&c| c == ')')
                .map_or(chars.len(), |p| end + p + 1);
            notes.push(Note::new(
                chars[end..close].iter().collect::<String>(),
                "parenthesis after a number is not a roll range",
            ));
        }
        values.push(Value { value, range: None });
        i = end;
    }
    (template, values, notes)
}

/// `[-]digits[.digits]` starting at `start`, which must begin a number.
fn number(chars: &[char], start: usize) -> (f64, usize) {
    let mut end = start;
    if chars[end] == '-' {
        end += 1;
    }
    while chars.get(end).is_some_and(char::is_ascii_digit) {
        end += 1;
    }
    if chars.get(end) == Some(&'.') && chars.get(end + 1).is_some_and(char::is_ascii_digit) {
        end += 1;
        while chars.get(end).is_some_and(char::is_ascii_digit) {
            end += 1;
        }
    }
    let text: String = chars[start..end].iter().collect();
    // Only digits, at most one dot and a leading sign were taken.
    (text.parse().unwrap_or(0.0), end)
}

/// Whether a number starts at `at`: an optional `-` and a digit.
fn number_at(chars: &[char], at: usize) -> bool {
    let digit = |k: usize| chars.get(k).is_some_and(char::is_ascii_digit);
    digit(at) || (chars.get(at) == Some(&'-') && digit(at + 1))
}

/// `(min-max)` at `open`, each end with its own optional `-`.
fn range(chars: &[char], open: usize) -> Option<(Range, usize)> {
    let mut at = open + 1;
    if !number_at(chars, at) {
        return None;
    }
    let (min, end) = number(chars, at);
    if chars.get(end) != Some(&'-') {
        return None;
    }
    at = end + 1;
    if !number_at(chars, at) {
        return None;
    }
    let (max, end) = number(chars, at);
    if chars.get(end) != Some(&')') {
        return None;
    }
    Some((Range { min, max }, end + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measured_waystone_line() {
        let m = parse_mod_text("Monsters deal 12(10-14)% of Damage as Extra Glimmer");
        assert_eq!(m.template, "Monsters deal #% of Damage as Extra Glimmer");
        assert_eq!(
            m.values,
            vec![Value {
                value: 12.0,
                range: Some(Range {
                    min: 10.0,
                    max: 14.0
                })
            }]
        );
        assert!(m.notes.is_empty());
    }

    #[test]
    fn header_with_everything() {
        let h = parse_header("{ Fractured Prefix Modifier \"Hale\" (Tier: 4) — Life, Defences }");
        assert_eq!(h.kind, ModKind::Prefix);
        assert!(h.fractured);
        assert_eq!(h.affix_name.as_deref(), Some("Hale"));
        assert_eq!(h.tier, Some(4));
        assert_eq!(h.tags, vec!["Life", "Defences"]);
        assert!(h.notes.is_empty());
    }

    #[test]
    fn header_without_modifier_word_is_noted() {
        let h = parse_header("{ Something Else }");
        assert_eq!(h.kind, ModKind::Unknown);
        assert_eq!(h.notes.len(), 1);
    }

    #[test]
    fn markers_are_taken_in_text_order() {
        let m = parse_mod_text("+10 to Life (crafted) (fractured)");
        assert_eq!(m.markers, vec![Marker::Crafted, Marker::Fractured]);
        assert_eq!(m.template, "+# to Life");
    }

    #[test]
    fn dash_after_a_word_is_not_a_sign() {
        let m = parse_mod_text("Non-Channelling Skills cost -2 Mana");
        assert_eq!(m.template, "Non-Channelling Skills cost # Mana");
        assert_eq!(m.values[0].value, -2.0);
    }
}
