//! The fuzzy match, ported from `tools/quest_tracker/outside/screen.py`
//! (`normalize` and `fuzzy_score`) so the overlay scores an OCR line exactly
//! as the research instruments did.
//!
//! [`fuzzy_score`] is the best `difflib.SequenceMatcher(None, autojunk=False)`
//! ratio of the normalized target against any window of the normalized line
//! whose length is within [`WINDOW_SLACK`] of the target's; a line shorter
//! than the shortest window is compared whole. The ratio is `2 * M / T`,
//! where `M` is the size of the matching blocks found by the same
//! longest-match recursion `difflib` uses (with its tie-breaking: the match
//! ending earliest in the window, then earliest in the target) and `T` the
//! two lengths' sum. Lengths count Unicode scalar values, as Python counts
//! code points.
//!
//! `apps/quest-tracker/fixtures/screen/fuzzy_parity.json` pins the scores the
//! Python function gives; `fuzzy_matches_the_python_scores` holds the port to
//! them within 1e-9, and `tests/test_quest_tracker_screen_parity.py` holds
//! the Python function to them.
//!
//! One known boundary: [`normalize`] keeps what Rust calls alphanumeric,
//! Python what it calls `isalnum`. The two agree on letters, digits and
//! numerals, and differ only on Unicode's `Other_Alphabetic` marks (an Indic
//! vowel sign, a circled letter), which Rust keeps and Python drops. The
//! English OCR the overlay uses emits none of them.

/// A line matches a name when [`fuzzy_score`] is at least this.
pub const MATCH_THRESHOLD: f64 = 0.8;

/// How far a compared window's length may stray from the target's, as a
/// fraction of the target's length.
pub const WINDOW_SLACK: f64 = 0.2;

/// Python's `str.isspace` beyond Rust's `char::is_whitespace`: the ASCII
/// information separators, which Python counts as space.
fn is_python_space(ch: char) -> bool {
    ch.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&ch)
}

/// Lowercase, letters, digits and spaces only, runs of spaces collapsed and
/// the ends trimmed.
pub fn normalize(text: &str) -> String {
    let lowered = text.to_lowercase();
    let mut out = String::with_capacity(lowered.len());
    let mut pending_space = false;
    for ch in lowered.chars() {
        if ch.is_alphanumeric() {
            if pending_space && !out.is_empty() {
                out.push(' ');
            }
            pending_space = false;
            out.push(ch);
        } else if is_python_space(ch) {
            pending_space = true;
        }
    }
    out
}

/// The normalized length of `text`, in Unicode scalar values.
pub fn normalized_len(text: &str) -> usize {
    normalize(text).chars().count()
}

/// How well `text` (an OCR line) contains `target`, 0..=1.
pub fn fuzzy_score(target: &str, text: &str) -> f64 {
    let t: Vec<char> = normalize(target).chars().collect();
    let s: Vec<char> = normalize(text).chars().collect();
    if t.is_empty() || s.is_empty() {
        return 0.0;
    }
    let (lo, hi) = window_bounds(t.len());
    let matcher = Matcher::new(&t);
    if s.len() < lo {
        return matcher.ratio(&s);
    }
    let mut best = 0.0_f64;
    for n in lo..=hi.min(s.len()) {
        for start in 0..=(s.len() - n) {
            let window = &s[start..start + n];
            if matcher.real_quick_ratio(window) <= best || matcher.quick_ratio(window) <= best {
                continue;
            }
            best = best.max(matcher.ratio(window));
            if best == 1.0 {
                return best;
            }
        }
    }
    best
}

/// Whether `text` matches `target` at [`MATCH_THRESHOLD`].
pub fn matches(target: &str, text: &str) -> bool {
    fuzzy_score(target, text) >= MATCH_THRESHOLD
}

/// The shortest and longest window for a target of `len` characters, as
/// Python computes them (the same float operations, in the same order).
fn window_bounds(len: usize) -> (usize, usize) {
    let len = len as f64;
    let lo = (len * (1.0 - WINDOW_SLACK) - 1e-9).ceil().max(1.0) as usize;
    let hi = (len * (1.0 + WINDOW_SLACK) + 1e-9).floor() as usize;
    (lo, hi)
}

/// `SequenceMatcher` with the target as its second sequence (`b`), which is
/// fixed, and each compared window as its first (`a`).
struct Matcher<'t> {
    b: &'t [char],
    /// Each character of `b` with its indices, ascending (`b2j`).
    b2j: Vec<(char, Vec<usize>)>,
}

impl<'t> Matcher<'t> {
    fn new(b: &'t [char]) -> Self {
        let mut b2j: Vec<(char, Vec<usize>)> = Vec::new();
        for (j, &ch) in b.iter().enumerate() {
            match b2j.iter_mut().find(|(c, _)| *c == ch) {
                Some((_, js)) => js.push(j),
                None => b2j.push((ch, vec![j])),
            }
        }
        Matcher { b, b2j }
    }

    fn indices(&self, ch: char) -> &[usize] {
        self.b2j
            .iter()
            .find(|(c, _)| *c == ch)
            .map(|(_, js)| js.as_slice())
            .unwrap_or(&[])
    }

    fn calculate_ratio(matches: usize, length: usize) -> f64 {
        if length > 0 {
            2.0 * matches as f64 / length as f64
        } else {
            1.0
        }
    }

    fn real_quick_ratio(&self, a: &[char]) -> f64 {
        Self::calculate_ratio(a.len().min(self.b.len()), a.len() + self.b.len())
    }

    fn quick_ratio(&self, a: &[char]) -> f64 {
        let mut avail: Vec<(char, isize)> = self
            .b2j
            .iter()
            .map(|(c, js)| (*c, js.len() as isize))
            .collect();
        let mut matches = 0;
        for ch in a {
            if let Some((_, n)) = avail.iter_mut().find(|(c, _)| c == ch) {
                if *n > 0 {
                    matches += 1;
                }
                *n -= 1;
            }
        }
        Self::calculate_ratio(matches, a.len() + self.b.len())
    }

    fn ratio(&self, a: &[char]) -> f64 {
        let matched = self.matched(a, 0, a.len(), 0, self.b.len());
        Self::calculate_ratio(matched, a.len() + self.b.len())
    }

    /// The total size of the matching blocks of `a[alo..ahi]` and
    /// `b[blo..bhi]` (`get_matching_blocks`, summed).
    fn matched(&self, a: &[char], alo: usize, ahi: usize, blo: usize, bhi: usize) -> usize {
        let (i, j, k) = self.find_longest_match(a, alo, ahi, blo, bhi);
        if k == 0 {
            return 0;
        }
        let mut total = k;
        if alo < i && blo < j {
            total += self.matched(a, alo, i, blo, j);
        }
        if i + k < ahi && j + k < bhi {
            total += self.matched(a, i + k, ahi, j + k, bhi);
        }
        total
    }

    /// `find_longest_match` with no junk: the longest block, the earliest in
    /// `a` among equals, then the earliest in `b`.
    fn find_longest_match(
        &self,
        a: &[char],
        alo: usize,
        ahi: usize,
        blo: usize,
        bhi: usize,
    ) -> (usize, usize, usize) {
        let (mut besti, mut bestj, mut bestsize) = (alo, blo, 0);
        // prev[j]: the length of the match ending at a[i - 1] and b[j].
        let mut prev = vec![0usize; self.b.len()];
        let mut cur = vec![0usize; self.b.len()];
        for i in alo..ahi {
            for &j in self.indices(a[i]) {
                if j < blo {
                    continue;
                }
                if j >= bhi {
                    break;
                }
                let k = if j > blo { prev[j - 1] } else { 0 } + 1;
                cur[j] = k;
                if k > bestsize {
                    besti = i + 1 - k;
                    bestj = j + 1 - k;
                    bestsize = k;
                }
            }
            std::mem::swap(&mut prev, &mut cur);
            cur.iter_mut().for_each(|v| *v = 0);
        }
        // With no junk the scan already finds a maximal block; difflib's
        // extension loops cannot grow it, so they are left out.
        (besti, bestj, bestsize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/screen/fuzzy_parity.json"
    );

    #[test]
    fn fuzzy_matches_the_python_scores() {
        let raw = std::fs::read_to_string(FIXTURE).expect("the parity fixture");
        let data: serde_json::Value = serde_json::from_str(&raw).expect("JSON");
        assert_eq!(data["match_threshold"].as_f64(), Some(MATCH_THRESHOLD));
        assert_eq!(data["window_slack"].as_f64(), Some(WINDOW_SLACK));
        let pairs = data["pairs"].as_array().expect("pairs");
        assert!(pairs.len() >= 30, "{} pairs", pairs.len());
        let mut wrong = Vec::new();
        for (n, pair) in pairs.iter().enumerate() {
            let target = pair["target"].as_str().unwrap();
            let text = pair["text"].as_str().unwrap();
            let want = pair["score"].as_f64().unwrap();
            let got = fuzzy_score(target, text);
            if (got - want).abs() > 1e-9 {
                wrong.push(format!("pair {n}: python {want}, rust {got}"));
            }
        }
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    }

    #[test]
    fn normalize_keeps_letters_digits_and_single_spaces() {
        assert_eq!(
            normalize("  BEIRA, of the\tRotten-Pack!! "),
            "beira of the rottenpack"
        );
        assert_eq!(normalize("Renly: Well met."), "renly well met");
        assert_eq!(normalize("ΣΟΦΟΣ"), "σοφος");
        assert_eq!(normalize("a\u{1f}b"), "a b");
        assert_eq!(normalize("..."), "");
    }

    #[test]
    fn empty_sides_score_zero() {
        assert_eq!(fuzzy_score("", "Renly"), 0.0);
        assert_eq!(fuzzy_score("Renly", ""), 0.0);
        assert_eq!(fuzzy_score("Renly", "!!"), 0.0);
    }

    #[test]
    fn window_bounds_follow_the_slack() {
        assert_eq!(window_bounds(5), (4, 6));
        assert_eq!(window_bounds(1), (1, 1));
        assert_eq!(window_bounds(24), (20, 28));
        assert_eq!(window_bounds(10), (8, 12));
    }

    #[test]
    fn matches_uses_the_threshold() {
        assert!(matches("Renly", "Rerly")); // exactly 0.8
        assert!(!matches("CHECKPOINT", "CHECK"));
    }
}
