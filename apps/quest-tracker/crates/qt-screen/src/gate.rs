//! The dialogue window's heading gate: while no demanded NPC's heading has
//! read lately, the reader copies and reads only a thin strip of the
//! dialogue window, where the window draws its heading, and reads the whole
//! region only once the heading shows there.
//!
//! - *Opens* on a read, of either kind, whose lines hold a demanded NPC's
//!   heading ([`is_heading`]) with its box inside [`HEADING_STRIP`]. A strip
//!   read that opens it is followed by a read of the whole region at once,
//!   so a banner's own read is never left to the next one.
//! - *Stays open* while the last such heading is within
//!   [`HEADING_WINDOW_MS`] of the read.
//! - *Shut*: the read is the strip alone, its lines mapped to client
//!   fractions through the strip's own area, and that read is the region's
//!   observation for the trackers.
//!
//! It cannot miss a banner the whole-region reads would catch:
//! [`DialogueTracker`](crate::DialogueTracker) fires a banner only within
//! [`HEADING_WINDOW_MS`] after a read with the heading, so that heading was
//! read before the banner and the gate is open for it, provided the heading
//! sits in the strip. The NPC's name over him in the world reads lower down,
//! outside the strip, and opens nothing.
//!
//! The gate keeps the last heading's time and counts of reads, never text.

use crate::dialogue::{is_heading, HEADING_WINDOW_MS};
use crate::geometry::Rect;
use crate::observation::Observation;
use crate::Millis;

/// The dialogue window's heading strip, as fractions of the client rect: the
/// region's width, and the rows where the heading reads (y 0.265 to 0.277
/// in every kept read that holds Renly's, the text block shifted or not).
/// About a tenth of the region's area.
pub const HEADING_STRIP: Rect = Rect::new(0.22, 0.24, 0.70, 0.31);

/// Which kind of read of the dialogue window an observation was.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DialogueRead {
    /// A copy of [`HEADING_STRIP`] alone, while the gate was shut.
    Strip,
    /// A copy of the whole region.
    Full,
}

/// The gate's reads since the counts were last taken. A hidden read is in
/// neither.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GateCounts {
    /// Visible reads of the whole region.
    pub full: u64,
    /// Visible reads of the heading strip alone.
    pub skipped: u64,
}

/// The heading gate before the dialogue window's full read, shared by the
/// reader and the replay.
#[derive(Clone, Debug, Default)]
pub struct DialogueGate {
    last_heading: Option<Millis>,
    counts: GateCounts,
}

impl DialogueGate {
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether a read at `at` is of the whole region: a demanded NPC's
    /// heading read in the strip within [`HEADING_WINDOW_MS`] before it.
    pub fn is_open(&self, at: Millis) -> bool {
        self.last_heading
            .is_some_and(|h| h <= at && at - h <= HEADING_WINDOW_MS)
    }

    /// Feed one read of the dialogue window, of kind `kind`, for the NPCs
    /// `npcs` the demand names; returns whether the gate is open after it.
    /// A hidden read is counted nowhere and changes nothing.
    pub fn observe(&mut self, npcs: &[String], obs: &Observation, kind: DialogueRead) -> bool {
        if obs.is_visible() {
            match kind {
                DialogueRead::Strip => self.counts.skipped += 1,
                DialogueRead::Full => self.counts.full += 1,
            }
            let heading = obs.lines.iter().any(|l| {
                HEADING_STRIP.contains(&l.bbox) && npcs.iter().any(|npc| is_heading(npc, l))
            });
            if heading {
                self.last_heading = Some(obs.at);
            }
        }
        self.is_open(obs.at)
    }

    /// Forget the last heading, as fresh trackers do; the counts stay.
    pub fn reset(&mut self) {
        self.last_heading = None;
    }

    /// The counts since the last call, reset.
    pub fn take_counts(&mut self) -> GateCounts {
        std::mem::take(&mut self.counts)
    }
}

/// The strip read a read of the whole region holds: its lines whose box
/// centre lies in [`HEADING_STRIP`]. The replay, which has only whole-region
/// OCR, simulates a strip read with it.
pub fn strip_of(obs: &Observation) -> Observation {
    let mut strip = obs.clone();
    strip.lines.retain(|l| {
        let (cx, cy) = (
            (l.bbox.left + l.bbox.right) / 2.0,
            (l.bbox.top + l.bbox.bottom) / 2.0,
        );
        cx >= HEADING_STRIP.left
            && cx <= HEADING_STRIP.right
            && cy >= HEADING_STRIP.top
            && cy <= HEADING_STRIP.bottom
    });
    strip
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observation::{Hidden, Line};
    use crate::region::{Region, DIALOGUE_WINDOW_RECT};

    const RENLY: &str = "Renly";

    fn npcs() -> Vec<String> {
        vec![RENLY.into()]
    }
    /// Renly's heading where the kept captures read it, the text block not
    /// shifted.
    fn heading() -> Line {
        Line::new("Renly", Rect::new(0.536, 0.265, 0.561, 0.277))
    }
    /// The same, the text block shifted.
    fn shifted_heading() -> Line {
        Line::new("RENLY", Rect::new(0.363, 0.265, 0.388, 0.277))
    }
    /// His name over him in the world.
    fn world_label() -> Line {
        Line::new("Renly", Rect::new(0.48, 0.461, 0.52, 0.484))
    }
    fn read(at: Millis, lines: Vec<Line>) -> Observation {
        Observation::visible(Region::DialogueWindow, at, lines)
    }

    #[test]
    fn the_strip_is_pinned_inside_the_window() {
        assert_eq!(HEADING_STRIP, Rect::new(0.22, 0.24, 0.70, 0.31));
        assert!(DIALOGUE_WINDOW_RECT.contains(&HEADING_STRIP));
        assert!(HEADING_STRIP.contains(&heading().bbox));
        assert!(HEADING_STRIP.contains(&shifted_heading().bbox));
    }

    #[test]
    fn dialogue_gate_stays_open_within_the_heading_window() {
        let n = npcs();
        let mut g = DialogueGate::new();
        // Shut before any heading.
        assert!(!g.is_open(0));
        assert!(!g.observe(&n, &read(0, vec![]), DialogueRead::Strip));
        // Open on a heading in the strip.
        assert!(g.observe(&n, &read(1_000, vec![heading()]), DialogueRead::Strip));
        assert!(g.is_open(1_000));
        // A full read with no heading keeps it open from the last heading.
        assert!(g.observe(&n, &read(2_000, vec![]), DialogueRead::Full));
        assert!(g.is_open(1_000 + HEADING_WINDOW_MS));
        assert!(!g.is_open(1_000 + HEADING_WINDOW_MS + 1));
        // A full read with the (shifted) heading moves it on.
        assert!(g.observe(
            &n,
            &read(5_000, vec![shifted_heading()]),
            DialogueRead::Full
        ));
        assert!(g.is_open(5_000 + HEADING_WINDOW_MS));
        // Shut after that.
        let late = 5_000 + HEADING_WINDOW_MS + 1;
        assert!(!g.observe(&n, &read(late, vec![]), DialogueRead::Strip));
        // A hidden read counts nowhere and opens nothing.
        let hidden = Observation::hidden(Region::DialogueWindow, late + 1, Hidden::Overlay);
        assert!(!g.observe(&n, &hidden, DialogueRead::Full));
        assert_eq!(
            g.take_counts(),
            GateCounts {
                full: 2,
                skipped: 3
            }
        );
        assert_eq!(g.take_counts(), GateCounts::default());
        // A reset forgets the heading.
        assert!(g.observe(&n, &read(late + 2, vec![heading()]), DialogueRead::Strip));
        g.reset();
        assert!(!g.is_open(late + 2));
    }

    #[test]
    fn dialogue_gate_ignores_a_heading_outside_the_strip() {
        let n = npcs();
        let mut g = DialogueGate::new();
        // The label is a heading by the tracker's rule (positive control),
        // but it sits outside the strip: it opens nothing.
        assert!(is_heading(RENLY, &world_label()));
        assert!(!HEADING_STRIP.contains(&world_label().bbox));
        assert!(!g.observe(&n, &read(0, vec![world_label()]), DialogueRead::Full));
        assert!(!g.observe(&n, &read(1_000, vec![world_label()]), DialogueRead::Strip));
        // Another NPC's heading in the strip opens nothing either.
        let una = Line::new("Una", heading().bbox);
        assert!(!g.observe(&n, &read(2_000, vec![una]), DialogueRead::Strip));
        // Nor does Renly's with no NPC demanded.
        assert!(!g.observe(&[], &read(3_000, vec![heading()]), DialogueRead::Strip));
        // Renly's own heading in the strip does.
        assert!(g.observe(&n, &read(4_000, vec![heading()]), DialogueRead::Strip));
    }

    #[test]
    fn a_strip_read_keeps_the_lines_centred_in_the_strip() {
        let banner = Line::new("QUEST COMPLETE", Rect::new(0.447, 0.776, 0.553, 0.800));
        let obs = read(7, vec![world_label(), heading(), banner, shifted_heading()]);
        let strip = strip_of(&obs);
        assert_eq!(strip.at, 7);
        assert_eq!(strip.region, Region::DialogueWindow);
        assert_eq!(strip.lines, vec![heading(), shifted_heading()]);
        let hidden = Observation::hidden(Region::DialogueWindow, 9, Hidden::Black);
        assert_eq!(strip_of(&hidden), hidden);
    }
}
