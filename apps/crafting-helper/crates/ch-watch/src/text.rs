//! What the matcher does to one OCR line before comparing it: the OCR
//! confusions undone where the token says which was meant, the numbers taken
//! out as values (a `(min-max)` range after a number dropped), and the rest
//! compared with `qt-screen`'s fuzzy match, both ways.

use qt_screen::{fuzzy_score, normalize};

/// One line, ready to compare: its words with the numbers taken out,
/// normalized, and the numbers in order.
#[derive(Clone, PartialEq)]
pub(crate) struct Prepared {
    /// Lowercase letters, digits left inside words, and single spaces.
    pub masked: String,
    pub values: Vec<f64>,
}

/// `text` with its confusions undone, its numbers read and masked.
pub(crate) fn prepare(text: &str) -> Prepared {
    let fixed = fix_confusions(text);
    let (rest, values) = take_numbers(&fixed);
    Prepared {
        masked: normalize(&rest),
        values,
    }
}

/// A data template (`+#% to Fire Resistance`) masked as [`prepare`] masks a
/// line, and how many values it takes.
pub(crate) fn prepare_template(template: &str) -> (String, usize) {
    (
        normalize(&template.replace('#', " ")),
        template.matches('#').count(),
    )
}

/// How alike two masked texts are, 0..=1: the lower of `qt-screen`'s
/// containment score taken each way, so that neither side may carry words
/// the other lacks.
pub(crate) fn similarity(a: &str, b: &str) -> f64 {
    if a == b {
        return 1.0;
    }
    fuzzy_score(a, b).min(fuzzy_score(b, a))
}

fn is_confusable(c: char) -> bool {
    matches!(c, 'l' | 'I' | '|' | 'O' | 'o')
}

fn is_number_mark(c: char) -> bool {
    matches!(c, '.' | ',' | '%' | '+' | '-' | '(' | ')' | '\u{2013}')
}

/// Each whitespace-separated token with its confusions undone: a token made
/// of digits, number marks and letters OCR takes for digits (`l`, `I`, `|`,
/// `O`, `o`) is a number, so the letters become `1` and `0`; a word with a
/// digit inside it is a word, so `1` becomes `i` and `0` becomes `o`.
pub(crate) fn fix_confusions(text: &str) -> String {
    text.split_whitespace()
        .map(fix_token)
        .collect::<Vec<_>>()
        .join(" ")
}

fn fix_token(token: &str) -> String {
    let has_digit = token.chars().any(|c| c.is_ascii_digit());
    let signed = token.starts_with('+') || token.starts_with('-');
    let numeric = token
        .chars()
        .all(|c| c.is_ascii_digit() || is_confusable(c) || is_number_mark(c));
    if numeric && (has_digit || (signed && token.chars().any(is_confusable))) {
        token
            .chars()
            .map(|c| match c {
                'l' | 'I' | '|' => '1',
                'O' | 'o' => '0',
                c => c,
            })
            .collect()
    } else if has_digit && token.chars().any(char::is_alphabetic) {
        // Only a lone digit beside a letter (`F1re`, `R1ng`): `Level:10`
        // keeps its number.
        let chars: Vec<char> = token.chars().collect();
        let digit = |j: Option<usize>| {
            j.and_then(|j| chars.get(j))
                .is_some_and(char::is_ascii_digit)
        };
        let letter = |j: Option<usize>| {
            j.and_then(|j| chars.get(j))
                .is_some_and(|c| c.is_alphabetic())
        };
        chars
            .iter()
            .enumerate()
            .map(|(j, &c)| {
                let (prev, next) = (j.checked_sub(1), Some(j + 1));
                let lone = !digit(prev) && !digit(next);
                match c {
                    '1' | '0' if lone && (letter(prev) || letter(next)) => {
                        if c == '1' {
                            'i'
                        } else {
                            'o'
                        }
                    }
                    c => c,
                }
            })
            .collect()
    } else {
        token.to_string()
    }
}

/// The text with every number (and a `(min-max)` range straight after one)
/// replaced by a space, and the numbers in order. A `-` right before a
/// number that starts a word makes it negative.
fn take_numbers(text: &str) -> (String, Vec<f64>) {
    let chars: Vec<char> = text.chars().collect();
    let mut rest = String::with_capacity(text.len());
    let mut values = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if let Some((value, end)) = number_at(&chars, i) {
            let negative =
                i > 0 && chars[i - 1] == '-' && (i == 1 || !chars[i - 2].is_alphanumeric());
            if negative {
                rest.pop();
            }
            values.push(if negative { -value } else { value });
            rest.push(' ');
            i = range_after(&chars, end).unwrap_or(end);
            continue;
        }
        rest.push(chars[i]);
        i += 1;
    }
    (rest, values)
}

/// A number starting at `i`: digits, and a fraction when a `.` is followed
/// by a digit. Its value and the index after it.
fn number_at(chars: &[char], i: usize) -> Option<(f64, usize)> {
    if !chars.get(i)?.is_ascii_digit() {
        return None;
    }
    let mut end = i;
    while end < chars.len() && chars[end].is_ascii_digit() {
        end += 1;
    }
    if end + 1 < chars.len() && chars[end] == '.' && chars[end + 1].is_ascii_digit() {
        end += 1;
        while end < chars.len() && chars[end].is_ascii_digit() {
            end += 1;
        }
    }
    let text: String = chars[i..end].iter().collect();
    text.parse().ok().map(|v| (v, end))
}

/// The index after a `(min-max)` range starting at `i` (spaces allowed
/// before it and inside it), if one does.
fn range_after(chars: &[char], i: usize) -> Option<usize> {
    let skip = |mut j: usize| {
        while j < chars.len() && chars[j] == ' ' {
            j += 1;
        }
        j
    };
    let mut j = skip(i);
    if chars.get(j) != Some(&'(') {
        return None;
    }
    j = skip(j + 1);
    if chars.get(j) == Some(&'-') {
        j += 1;
    }
    let (_, end) = number_at(chars, j)?;
    j = skip(end);
    if !matches!(chars.get(j), Some('-') | Some('\u{2013}')) {
        return None;
    }
    j = skip(j + 1);
    if chars.get(j) == Some(&'-') {
        j += 1;
    }
    let (_, end) = number_at(chars, j)?;
    j = skip(end);
    (chars.get(j) == Some(&')')).then_some(j + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_and_ranges_come_out() {
        let p = prepare("+95(90-104) to maximum Mana");
        assert_eq!(p.values, [95.0]);
        assert_eq!(p.masked, "to maximum mana");
        let p = prepare("Adds 11 to 20 Physical Damage to Attacks");
        assert_eq!(p.values, [11.0, 20.0]);
        assert_eq!(p.masked, "adds to physical damage to attacks");
        let p = prepare("Leech 6.5% of Physical Attack Damage as Mana.");
        assert_eq!(p.values, [6.5]);
        let p = prepare("+33 (31 - 35)% to Fire Resistance");
        assert_eq!(p.values, [33.0]);
        assert_eq!(p.masked, "to fire resistance");
        assert_eq!(prepare("-5% to Cold Resistance").values, [-5.0]);
        assert_eq!(prepare("Prefix Modifier (Tier: 5)").values, [5.0]);
    }

    #[test]
    fn confusions_follow_the_token() {
        assert_eq!(fix_confusions("+3l% to F1re"), "+31% to Fire");
        assert_eq!(fix_confusions("Level 2O"), "Level 20");
        assert_eq!(fix_confusions("+l to Level"), "+1 to Level");
        assert_eq!(fix_confusions("I to lo"), "I to lo");
        assert_eq!(fix_confusions("Item Level:10 R1ng"), "Item Level:10 Ring");
        let (masked, n) = prepare_template("+#% to Fire Resistance");
        assert_eq!((masked.as_str(), n), ("to fire resistance", 1));
    }

    #[test]
    fn similarity_is_symmetric_and_strict_on_extra_words() {
        assert_eq!(similarity("to fire resistance", "to fire resistance"), 1.0);
        assert!(similarity("to fire resistance", "to cold resistance") < 0.85);
        assert!(
            similarity(
                "to lightning resistance",
                "to lightning and chaos resistances"
            ) < 0.85
        );
        assert!(similarity("to maximum mana", "to rnaximum mana") >= 0.85);
    }
}
