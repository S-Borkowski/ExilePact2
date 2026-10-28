//! The token picker and the set cover (context "Token and regex rules").
//!
//! A candidate token is a lower-cased substring of a run of `[a-z ]` inside
//! one line, at least [`MIN_TOKEN_LEN`] characters, with no space at either
//! end. Candidates are taken only from a family's own lines (never its shared
//! reward lines) and checked against every line of every other family,
//! reward lines included, and against the frame lines; that is correct
//! whether or not the game shows reward lines on the item.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::data::ModData;
use crate::frame::frame_lines;

/// The shortest token: two letters are likelier to hit a random rare name.
pub const MIN_TOKEN_LEN: usize = 3;

/// Every candidate token of one line, shortest first, then alphabetical,
/// without repeats.
pub fn candidates(line: &str) -> Vec<String> {
    let lower = line.to_lowercase();
    let mut found: BTreeSet<(usize, String)> = BTreeSet::new();
    for run in lower.split(|c: char| !(c.is_ascii_lowercase() || c == ' ')) {
        let b = run.as_bytes();
        for i in 0..b.len() {
            if b[i] == b' ' {
                continue;
            }
            for j in (i + MIN_TOKEN_LEN)..=b.len() {
                if b[j - 1] != b' ' {
                    found.insert((j - i, run[i..j].to_string()));
                }
            }
        }
    }
    found.into_iter().map(|(_, s)| s).collect()
}

/// A family's own token, or why it has none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FamilyToken {
    pub id: String,
    pub token: Option<String>,
    /// Set exactly when `token` is not.
    pub why_not: Option<String>,
}

/// A set of families, one bit per position in the data.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Bits(Vec<u64>);

impl Bits {
    fn new(n: usize) -> Bits {
        Bits(vec![0; n.div_ceil(64).max(1)])
    }
    fn set(&mut self, i: usize) {
        self.0[i / 64] |= 1 << (i % 64);
    }
    fn has(&self, i: usize) -> bool {
        self.0[i / 64] & (1 << (i % 64)) != 0
    }
    fn subset_of(&self, other: &Bits) -> bool {
        self.0.iter().zip(&other.0).all(|(a, b)| a & !b == 0)
    }
    fn and_count(&self, other: &Bits) -> u32 {
        self.0
            .iter()
            .zip(&other.0)
            .map(|(a, b)| (a & b).count_ones())
            .sum()
    }
    fn remove_all(&mut self, other: &Bits) {
        for (a, b) in self.0.iter_mut().zip(&other.0) {
            *a &= !b;
        }
    }
    fn is_empty(&self) -> bool {
        self.0.iter().all(|w| *w == 0)
    }
}

/// One candidate, and where it occurs.
#[derive(Debug, Clone)]
struct Candidate {
    text: String,
    /// Families with any line (reward lines included) holding it.
    anywhere: Bits,
    /// Families whose own lines hold it: an item with the family has it.
    own: Bits,
    in_frame: bool,
}

/// Every family's token, and what a set cover needs.
#[derive(Debug, Clone)]
pub struct Tokens {
    entries: Vec<FamilyToken>,
    index: HashMap<String, usize>,
    pool: Vec<Candidate>,
}

impl Tokens {
    /// Picks each family's token: the shortest candidate (then the
    /// alphabetically first) of its first own line that no other family's
    /// line and no frame line holds, else of its later own lines.
    pub fn pick(data: &ModData) -> Tokens {
        let n = data.families.len();
        let all_lines: Vec<Vec<String>> = data
            .families
            .iter()
            .map(|f| f.lines.iter().map(|l| l.to_lowercase()).collect())
            .collect();
        let own_lines: Vec<Vec<String>> = data
            .families
            .iter()
            .map(|f| f.own_lines().map(|l| l.to_lowercase()).collect())
            .collect();
        let frame = frame_lines(data);

        let mut texts: BTreeSet<String> = BTreeSet::new();
        for lines in &own_lines {
            for line in lines {
                texts.extend(candidates(line));
            }
        }
        let mut pool = Vec::with_capacity(texts.len());
        let mut at: HashMap<String, usize> = HashMap::with_capacity(texts.len());
        for text in texts {
            let mut anywhere = Bits::new(n);
            let mut own = Bits::new(n);
            for f in 0..n {
                if all_lines[f].iter().any(|l| l.contains(text.as_str())) {
                    anywhere.set(f);
                }
                if own_lines[f].iter().any(|l| l.contains(text.as_str())) {
                    own.set(f);
                }
            }
            let in_frame = frame.iter().any(|l| l.contains(text.as_str()));
            at.insert(text.clone(), pool.len());
            pool.push(Candidate {
                text,
                anywhere,
                own,
                in_frame,
            });
        }

        let mut entries = Vec::with_capacity(n);
        let mut index = HashMap::with_capacity(n);
        for (fi, f) in data.families.iter().enumerate() {
            let mut only_f = Bits::new(n);
            only_f.set(fi);
            let token = own_lines[fi].iter().find_map(|line| {
                candidates(line).into_iter().find(|c| {
                    let cand = &pool[at[c]];
                    !cand.in_frame && cand.anywhere.subset_of(&only_f)
                })
            });
            let why_not = match &token {
                Some(_) => None,
                None => Some(why_not(data, fi, &own_lines, &all_lines)),
            };
            index.insert(f.id.clone(), fi);
            entries.push(FamilyToken {
                id: f.id.clone(),
                token,
                why_not,
            });
        }
        Tokens {
            entries,
            index,
            pool,
        }
    }

    /// Every family's token, in the data's order.
    pub fn all(&self) -> &[FamilyToken] {
        &self.entries
    }

    /// The token entry of the family with this id.
    pub fn get(&self, id: &str) -> Option<&FamilyToken> {
        self.index.get(id).map(|&i| &self.entries[i])
    }

    /// The alternatives of one term that matches an item exactly when it
    /// carries any family of `ids`: a greedy cover by text only those
    /// families hold, used when shorter than their own tokens joined, so
    /// never longer. `Err` lists the families (in the data's order) that no
    /// such text reaches. Ids the data lacks are ignored; the builder
    /// refuses them first.
    pub fn alternation(&self, ids: &[String]) -> Result<Vec<String>, Vec<String>> {
        let n = self.entries.len();
        let mut members: Vec<usize> = ids
            .iter()
            .filter_map(|id| self.index.get(id).copied())
            .collect();
        members.sort_unstable();
        members.dedup();
        let mut set = Bits::new(n);
        for &m in &members {
            set.set(m);
        }

        let usable: Vec<&Candidate> = self
            .pool
            .iter()
            .filter(|c| !c.in_frame && c.anywhere.subset_of(&set) && c.own.and_count(&set) > 0)
            .collect();
        let mut uncovered = set.clone();
        let mut chosen: Vec<&Candidate> = Vec::new();
        loop {
            let mut best: Option<(&Candidate, u32)> = None;
            for c in &usable {
                let gain = c.own.and_count(&uncovered);
                if gain == 0 {
                    continue;
                }
                let better = match best {
                    None => true,
                    Some((b, bg)) => {
                        let lhs = gain as usize * b.text.len();
                        let rhs = bg as usize * c.text.len();
                        lhs > rhs
                            || (lhs == rhs && (c.text.len(), &c.text) < (b.text.len(), &b.text))
                    }
                };
                if better {
                    best = Some((c, gain));
                }
            }
            match best {
                Some((c, _)) => {
                    uncovered.remove_all(&c.own);
                    chosen.push(c);
                }
                None => break,
            }
        }
        if !uncovered.is_empty() {
            return Err((0..n)
                .filter(|&i| uncovered.has(i))
                .map(|i| self.entries[i].id.clone())
                .collect());
        }
        let cover: Vec<String> = chosen.iter().map(|c| c.text.clone()).collect();
        let own: Option<Vec<String>> = members
            .iter()
            .map(|&m| self.entries[m].token.clone())
            .collect();
        match own {
            Some(own) if joined_len(&own) <= joined_len(&cover) => Ok(own),
            _ => Ok(cover),
        }
    }
}

fn joined_len(alts: &[String]) -> usize {
    alts.iter().map(String::len).sum::<usize>() + alts.len().saturating_sub(1)
}

/// The reason a family has no token of its own, naming a family that
/// contains it when there is one.
fn why_not(
    data: &ModData,
    fi: usize,
    own_lines: &[Vec<String>],
    all_lines: &[Vec<String>],
) -> String {
    if own_lines[fi].is_empty() {
        return "it has only reward lines, which other mods share".to_string();
    }
    // Of the families holding every own line of this one, the first by id,
    // so the reason does not depend on the file's order.
    let container: BTreeMap<&str, usize> = (0..data.families.len())
        .filter(|&g| g != fi)
        .filter(|&g| {
            own_lines[fi]
                .iter()
                .all(|line| all_lines[g].iter().any(|l| l.contains(line.as_str())))
        })
        .map(|g| (data.families[g].id.as_str(), g))
        .collect();
    match container.into_iter().next() {
        Some((id, g)) => format!(
            "its text is also part of \"{}\" ({id}), so a search cannot find it alone",
            data.families[g].lines[0]
        ),
        None => "every part of its text also appears in other mods or in the waystone's own text"
            .to_string(),
    }
}
