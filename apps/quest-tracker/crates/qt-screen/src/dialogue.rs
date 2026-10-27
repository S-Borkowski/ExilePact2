//! The dialogue window: an NPC's window is open while its heading reads, and
//! a "QUEST COMPLETE" banner in it is `quest_complete`.
//!
//! - *Heading*: a line in the region that matches the NPC's name and is no
//!   longer than the name's normalized length plus [`HEADING_SLACK`]. The
//!   window draws the name on its own as a heading; an `N: ...` line (the
//!   list of voiced lines, or a remark) is longer and is not a window.
//! - *Banner*: a line matching [`BANNER_TEXT`] whose box lies inside
//!   [`BANNER_AREA`]. The next quest's banner, its objective in capitals in
//!   the same place, does not match.
//!
//! A banner within [`HEADING_WINDOW_MS`] after a read with the NPC's heading
//! emits one [`ScreenEvent::QuestComplete`]. One banner shows for a few
//! reads, so reads of it less than [`BANNER_REARM_MS`] apart are the same
//! banner and fire at most once.

use crate::event::ScreenEvent;
use crate::fuzzy::{fuzzy_score, normalized_len, MATCH_THRESHOLD};
use crate::geometry::Rect;
use crate::observation::{Line, Observation};
use crate::Millis;

/// A heading may be this much longer than the NPC's name, as a fraction of
/// the name's normalized length.
pub const HEADING_SLACK: f64 = 0.2;

/// The banner's text.
pub const BANNER_TEXT: &str = "QUEST COMPLETE";

/// Where the banner shows (measured at x 0.447 to 0.553, y 0.776 to 0.800).
pub const BANNER_AREA: Rect = Rect::new(0.35, 0.72, 0.65, 0.84);

/// How long after a read with the NPC's heading a banner still counts.
pub const HEADING_WINDOW_MS: Millis = 10_000;

/// Banner reads closer than this belong to the same banner.
pub const BANNER_REARM_MS: Millis = 5_000;

/// Whether `line` is `npc`'s window heading.
pub fn is_heading(npc: &str, line: &Line) -> bool {
    let limit = (normalized_len(npc) as f64 * (1.0 + HEADING_SLACK) + 1e-9).floor() as usize;
    normalized_len(&line.text) <= limit && fuzzy_score(npc, &line.text) >= MATCH_THRESHOLD
}

/// Whether `line` is the "QUEST COMPLETE" banner, in its place.
pub fn is_banner(line: &Line) -> bool {
    BANNER_AREA.contains(&line.bbox) && fuzzy_score(BANNER_TEXT, &line.text) >= MATCH_THRESHOLD
}

/// One NPC's dialogue window over a sequence of reads of the region.
#[derive(Clone, Debug)]
pub struct DialogueTracker {
    npc: String,
    last_heading: Option<Millis>,
    /// The last read that showed a banner, and whether that banner fired.
    banner: Option<(Millis, bool)>,
}

impl DialogueTracker {
    pub fn new(npc: impl Into<String>) -> Self {
        DialogueTracker {
            npc: npc.into(),
            last_heading: None,
            banner: None,
        }
    }

    pub fn npc(&self) -> &str {
        &self.npc
    }

    /// Feed one read of the dialogue window; a hidden read changes nothing.
    pub fn observe(&mut self, obs: &Observation) -> Option<ScreenEvent> {
        if !obs.is_visible() {
            return None;
        }
        if obs.lines.iter().any(|l| is_heading(&self.npc, l)) {
            self.last_heading = Some(obs.at);
        }
        if !obs.lines.iter().any(is_banner) {
            return None;
        }
        let fired = match self.banner {
            Some((seen, fired)) if obs.at - seen < BANNER_REARM_MS => fired,
            _ => false,
        };
        let in_window = self
            .last_heading
            .is_some_and(|h| h <= obs.at && obs.at - h <= HEADING_WINDOW_MS);
        let fire = !fired && in_window;
        self.banner = Some((obs.at, fired || fire));
        fire.then(|| ScreenEvent::QuestComplete {
            npc: self.npc.clone(),
            at: obs.at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observation::Hidden;
    use crate::region::Region;

    const RENLY: &str = "Renly";

    fn heading(text: &str) -> Line {
        Line::new(text, Rect::new(0.47, 0.26, 0.53, 0.28))
    }
    fn body(text: &str) -> Line {
        Line::new(text, Rect::new(0.42, 0.51, 0.66, 0.53))
    }
    fn banner(text: &str) -> Line {
        Line::new(text, Rect::new(0.447, 0.776, 0.553, 0.800))
    }
    fn read(at: Millis, lines: Vec<Line>) -> Observation {
        Observation::visible(Region::DialogueWindow, at, lines)
    }
    fn run(t: &mut DialogueTracker, obs: &[Observation]) -> Vec<ScreenEvent> {
        obs.iter().filter_map(|o| t.observe(o)).collect()
    }

    #[test]
    fn constants_are_pinned() {
        assert_eq!(HEADING_SLACK, 0.2);
        assert_eq!(BANNER_TEXT, "QUEST COMPLETE");
        assert_eq!(BANNER_AREA, Rect::new(0.35, 0.72, 0.65, 0.84));
        assert_eq!((HEADING_WINDOW_MS, BANNER_REARM_MS), (10_000, 5_000));
    }

    #[test]
    fn dialogue_quest_complete_in_the_npc_window_target() {
        let mut t = DialogueTracker::new(RENLY);
        let obs = vec![
            read(
                0,
                vec![
                    heading("RENLY"),
                    body("Some words of thanks."),
                    heading("Renly"),
                ],
            ),
            read(1_000, vec![heading("Renly"), body("Select One Reward")]),
            read(9_000, vec![heading("Renly"), banner("QUEST COMPLETE")]),
            read(10_000, vec![banner("QUEST COMPLETE")]),
            read(11_000, vec![heading("Renly"), banner("QUEST C0MPLETE")]),
            // The next quest's banner, in the same place.
            read(13_000, vec![heading("Renly"), banner("SLAY THE DEVOURER")]),
            read(20_000, vec![heading("Renly"), banner("SLAY THE DEVOURER")]),
        ];
        assert_eq!(
            run(&mut t, &obs),
            vec![ScreenEvent::QuestComplete {
                npc: RENLY.into(),
                at: 9_000
            }]
        );
    }

    #[test]
    fn dialogue_prefixed_list_line_is_not_a_window_baseline() {
        let mut t = DialogueTracker::new(RENLY);
        let prefixed = Line::new(
            "Renly: Well met, traveller.",
            Rect::new(0.22, 0.70, 0.45, 0.72),
        );
        assert!(!is_heading(RENLY, &prefixed));
        let obs = vec![
            read(0, vec![prefixed.clone()]),
            read(1_000, vec![prefixed.clone(), banner("QUEST COMPLETE")]),
            read(2_000, vec![prefixed, banner("QUEST COMPLETE")]),
        ];
        assert!(run(&mut t, &obs).is_empty());
    }

    #[test]
    fn dialogue_banner_needs_the_heading_within_the_window() {
        let mut t = DialogueTracker::new(RENLY);
        let obs = vec![
            read(0, vec![heading("Renly")]),
            read(10_001, vec![banner("QUEST COMPLETE")]),
        ];
        assert!(run(&mut t, &obs).is_empty());

        // Another NPC's window.
        let mut t = DialogueTracker::new(RENLY);
        let obs = vec![read(0, vec![heading("Una"), banner("QUEST COMPLETE")])];
        assert!(run(&mut t, &obs).is_empty());
    }

    #[test]
    fn dialogue_banner_out_of_its_place_does_not_count() {
        let mut t = DialogueTracker::new(RENLY);
        let elsewhere = Line::new("QUEST COMPLETE", Rect::new(0.45, 0.50, 0.55, 0.52));
        let obs = vec![read(0, vec![heading("Renly"), elsewhere])];
        assert!(run(&mut t, &obs).is_empty());
    }

    #[test]
    fn dialogue_hidden_read_changes_nothing() {
        let mut t = DialogueTracker::new(RENLY);
        assert_eq!(
            t.observe(&Observation::hidden(
                Region::DialogueWindow,
                0,
                Hidden::Overlay
            )),
            None
        );
        assert!(run(&mut t, &[read(1_000, vec![banner("QUEST COMPLETE")])]).is_empty());
    }

    #[test]
    fn dialogue_second_banner_later_fires_again() {
        let mut t = DialogueTracker::new(RENLY);
        let obs = vec![
            read(0, vec![heading("Renly"), banner("QUEST COMPLETE")]),
            read(1_000, vec![heading("Renly")]),
            read(30_000, vec![heading("Renly")]),
            read(31_000, vec![heading("Renly"), banner("QUEST COMPLETE")]),
        ];
        let events = run(&mut t, &obs);
        assert_eq!(
            events.iter().map(ScreenEvent::at).collect::<Vec<_>>(),
            vec![0, 31_000]
        );
    }
}
