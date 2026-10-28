//! The character's kill remark: a new `<prefix>: <text>` line in the list of
//! voiced lines whose text matches a known kill-remark text is
//! `kill_remark` for the boss. A remark at a quest moment is read the same
//! way, by the texts of its key, and is `remark` for the key.
//!
//! The prefix is the speaker, here the character's name; it is not checked,
//! since only the text tells a kill remark from the character's other
//! shouts. A line is new by the research instruments' rule: its read at `t`
//! shows more lines reading as the remark than any read in
//! [`t - NEW_FROM_MS`, `t - NEW_TO_MS`] did. A read with no read in that span
//! is not judged, so a line already on screen when reading starts never
//! counts. One remark stays on screen for several reads; the reads right
//! after the one that fired do not fire again for the same line.
//!
//! Only counts are kept between reads, never OCR text.

use std::collections::VecDeque;

use crate::event::ScreenEvent;
use crate::fuzzy::{fuzzy_score, normalize, MATCH_THRESHOLD};
use crate::observation::Observation;
use crate::Millis;

/// The start of the span before a read that tells a new line from a stale
/// one, in ms before the read.
pub const NEW_FROM_MS: Millis = 5_000;

/// The end of that span, in ms before the read.
pub const NEW_TO_MS: Millis = 1_000;

/// The text of a `<prefix>: <text>` line, or `None` when the line has no
/// speaker prefix or no text after it.
pub fn remark_body(text: &str) -> Option<&str> {
    let (prefix, body) = text.split_once(':')?;
    if normalize(prefix).is_empty() || normalize(body).is_empty() {
        return None;
    }
    Some(body)
}

#[derive(Clone, Debug)]
struct Read {
    at: Millis,
    /// Per known text, the lines of this read that carried it.
    counts: Vec<u32>,
    /// Per known text, whether this read fired for it.
    fired: Vec<bool>,
}

/// What a [`RemarkTracker`] reads for: a boss's kill remark, or a quest
/// moment's remark by its key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RemarkTarget {
    /// `kill_remark` for the boss.
    Boss(String),
    /// `remark` for the key.
    Key(String),
}

/// One boss's kill remarks, or one key's remarks, over a sequence of reads
/// of the voiced lines.
#[derive(Clone, Debug)]
pub struct RemarkTracker {
    target: RemarkTarget,
    texts: Vec<String>,
    reads: VecDeque<Read>,
}

impl RemarkTracker {
    /// `texts` are the known kill-remark texts for `boss` (the quest data's
    /// `remarks` rows of the boss).
    pub fn new(boss: impl Into<String>, texts: Vec<String>) -> Self {
        Self::reading(RemarkTarget::Boss(boss.into()), texts)
    }

    /// `texts` are the known remark texts of `key` (the quest data's
    /// `remarks` rows of the key, every class's).
    pub fn for_key(key: impl Into<String>, texts: Vec<String>) -> Self {
        Self::reading(RemarkTarget::Key(key.into()), texts)
    }

    fn reading(target: RemarkTarget, texts: Vec<String>) -> Self {
        RemarkTracker {
            target,
            texts,
            reads: VecDeque::new(),
        }
    }

    pub fn target(&self) -> &RemarkTarget {
        &self.target
    }

    /// The boss, for a kill remark's tracker.
    pub fn boss(&self) -> Option<&str> {
        match &self.target {
            RemarkTarget::Boss(boss) => Some(boss),
            RemarkTarget::Key(_) => None,
        }
    }

    /// The key, for a key's tracker.
    pub fn key(&self) -> Option<&str> {
        match &self.target {
            RemarkTarget::Key(key) => Some(key),
            RemarkTarget::Boss(_) => None,
        }
    }

    pub fn texts(&self) -> &[String] {
        &self.texts
    }

    /// Feed one read of the voiced lines; a hidden read is not a read.
    pub fn observe(&mut self, obs: &Observation) -> Option<ScreenEvent> {
        if !obs.is_visible() || self.texts.is_empty() {
            return None;
        }
        let t = obs.at;
        while self.reads.front().is_some_and(|r| r.at < t - NEW_FROM_MS) {
            self.reads.pop_front();
        }
        let counts: Vec<u32> = self
            .texts
            .iter()
            .map(|known| {
                obs.lines
                    .iter()
                    .filter_map(|l| remark_body(&l.text))
                    .filter(|body| fuzzy_score(known, body) >= MATCH_THRESHOLD)
                    .count() as u32
            })
            .collect();
        let judged = self.reads.iter().any(|r| r.at <= t - NEW_TO_MS);
        let fired: Vec<bool> = (0..self.texts.len())
            .map(|k| {
                if !judged || counts[k] == 0 {
                    return false;
                }
                let stale = self
                    .reads
                    .iter()
                    .filter(|r| r.at <= t - NEW_TO_MS || (r.fired[k] && r.at <= t))
                    .map(|r| r.counts[k])
                    .max()
                    .unwrap_or(0);
                counts[k] > stale
            })
            .collect();
        let fire = fired.iter().any(|&f| f);
        self.reads.push_back(Read {
            at: t,
            counts,
            fired,
        });
        fire.then(|| match &self.target {
            RemarkTarget::Boss(boss) => ScreenEvent::KillRemark {
                boss: boss.clone(),
                at: t,
            },
            RemarkTarget::Key(key) => ScreenEvent::Remark {
                key: key.clone(),
                at: t,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Rect;
    use crate::observation::{Hidden, Line};
    use crate::region::Region;

    const MILLER: &str = "The Bloated Miller";
    // Invented remark text and character name.
    const KNOWN: &str = "That one won't be grinding any more.";

    fn tracker() -> RemarkTracker {
        RemarkTracker::new(MILLER, vec![KNOWN.to_string()])
    }
    /// The list's newest line, and an older one above it.
    fn newest(text: &str) -> Line {
        Line::new(text, Rect::new(0.03, 0.71, 0.30, 0.73))
    }
    fn older(text: &str) -> Line {
        Line::new(text, Rect::new(0.03, 0.69, 0.30, 0.71))
    }
    fn read(at: Millis, lines: Vec<Line>) -> Observation {
        Observation::visible(Region::VoicedLines, at, lines)
    }
    /// Reads at 2 Hz from `start` ms until before `end`, each with `lines`.
    fn reads(start: Millis, end: Millis, lines: Vec<Line>) -> Vec<Observation> {
        (start..end)
            .step_by(500)
            .map(|at| read(at, lines.clone()))
            .collect()
    }
    fn run(t: &mut RemarkTracker, obs: &[Observation]) -> Vec<ScreenEvent> {
        obs.iter().filter_map(|o| t.observe(o)).collect()
    }

    #[test]
    fn constants_are_pinned() {
        assert_eq!((NEW_FROM_MS, NEW_TO_MS), (5_000, 1_000));
    }

    #[test]
    fn remark_known_text_is_a_kill_remark_target() {
        let mut t = tracker();
        let shout = older("Kestrella: For the encampment!");
        let mut obs = reads(0, 3_000, vec![shout.clone()]);
        obs.extend(reads(
            3_000,
            9_000,
            vec![
                shout.clone(),
                newest("Kestrella: That one wont be grlnding any rnore"),
            ],
        ));
        let events = run(&mut t, &obs);
        assert_eq!(
            events,
            vec![ScreenEvent::KillRemark {
                boss: MILLER.into(),
                at: 3_000
            }]
        );
    }

    #[test]
    fn remark_of_a_key_is_a_remark_target() {
        // Invented key; the same reads as a kill remark's.
        let mut t = RemarkTracker::for_key("FixtureTreeComment", vec![KNOWN.to_string()]);
        assert_eq!((t.key(), t.boss()), (Some("FixtureTreeComment"), None));
        let mut obs = reads(0, 3_000, Vec::new());
        obs.extend(reads(
            3_000,
            9_000,
            vec![newest("Kestrella: That one won't be grinding any more.")],
        ));
        assert_eq!(
            run(&mut t, &obs),
            vec![ScreenEvent::Remark {
                key: "FixtureTreeComment".into(),
                at: 3_000
            }]
        );
        assert_eq!(tracker().boss(), Some(MILLER));
    }

    #[test]
    fn remark_unknown_text_is_ignored_baseline() {
        let mut t = tracker();
        let mut obs = reads(0, 3_000, Vec::new());
        obs.extend(reads(
            3_000,
            6_000,
            vec![newest("Kestrella: What happened here...?")],
        ));
        assert!(run(&mut t, &obs).is_empty());
    }

    #[test]
    fn remark_stale_line_is_not_new() {
        // The remark is already on screen when reading starts.
        let mut t = tracker();
        let obs = reads(
            0,
            8_000,
            vec![newest("Kestrella: That one won't be grinding any more.")],
        );
        assert!(run(&mut t, &obs).is_empty());
    }

    #[test]
    fn remark_text_without_a_speaker_prefix_is_ignored() {
        let mut t = tracker();
        let mut obs = reads(0, 3_000, Vec::new());
        obs.extend(reads(3_000, 6_000, vec![newest(KNOWN)]));
        obs.extend(reads(
            6_000,
            8_000,
            vec![newest(": That one won't be grinding any more.")],
        ));
        assert!(run(&mut t, &obs).is_empty());
        assert_eq!(remark_body("Kestrella:"), None);
        assert_eq!(remark_body("Kestrella: hi"), Some(" hi"));
    }

    #[test]
    fn remark_said_again_after_leaving_the_screen_fires_again() {
        let mut t = tracker();
        let line = newest("Kestrella: That one won't be grinding any more.");
        let mut obs = reads(0, 2_000, Vec::new());
        obs.extend(reads(2_000, 6_000, vec![line.clone()]));
        obs.extend(reads(6_000, 14_000, Vec::new()));
        obs.extend(reads(14_000, 16_000, vec![line.clone()]));
        // A second copy beside the first is new too.
        obs.extend(reads(
            16_000,
            18_000,
            vec![
                line.clone(),
                older("Kestrella: That one won't be grinding any more."),
            ],
        ));
        let events = run(&mut t, &obs);
        assert_eq!(
            events.iter().map(ScreenEvent::at).collect::<Vec<_>>(),
            vec![2_000, 14_000, 16_000]
        );
    }

    #[test]
    fn remark_hidden_reads_are_not_reads() {
        let mut t = tracker();
        let mut obs: Vec<Observation> = (0..6_000)
            .step_by(500)
            .map(|at| Observation::hidden(Region::VoicedLines, at, Hidden::Overlay))
            .collect();
        // The first visible read has nothing before it to judge by.
        obs.extend(reads(
            6_000,
            9_000,
            vec![newest("Kestrella: That one won't be grinding any more.")],
        ));
        assert!(run(&mut t, &obs).is_empty());
    }
}
