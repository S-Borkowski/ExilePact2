//! The boss bar: each read of the band is present, covered or absent for a
//! boss, and a run of absent reads after the bar was up is `bar_gone`.
//!
//! - *Present*: the boss's name matches a line anywhere in the band. Some
//!   panels move the name (x 0.28 with the inventory open), so its place is
//!   not checked.
//! - *Covered*: the band was not visible (the overlay over it, the game not
//!   in front or minimized, a failed or black copy, a failed OCR), or a line
//!   that is not the name lies over the name's column ([`NAME_COLUMN_LEFT`]
//!   to [`NAME_COLUMN_RIGHT`]) with its top at most [`NAME_COLUMN_MAX_TOP`].
//!   The passive tree's title and the world map's headings sit there and
//!   hide the bar. So does an ordinary monster's name, which Live 8 read in
//!   the same column after the person walked away from the Miller.
//! - *Absent*: neither. Scene text elsewhere in the band (a "CHECKPOINT"
//!   label, whose height moves with the camera) does not cover.
//!
//! After at least [`MIN_PRESENT`] present reads, [`GONE_RUN`] absent reads
//! in a row, with no covered read since the last present one, emit one
//! [`ScreenEvent::BarGone`] carrying the last present read's time. The
//! tracker then waits for the bar to be present again. Luma is never used:
//! it does not separate a band with the bar from one without.

use crate::event::ScreenEvent;
use crate::fuzzy::{fuzzy_score, MATCH_THRESHOLD};
use crate::observation::Observation;
use crate::Millis;

/// Absent reads in a row that mean the bar went (3 s at 2 Hz). Settled by
/// `quest-tracker-replay screen` over the kept captures of Lives 6 and 8:
/// at 6, with the name column below, each kill gives one `bar_gone` within
/// a read of the bar's last showing, and Live 8's covering panels give none.
pub const GONE_RUN: u32 = 6;

/// Present reads the bar needs before its going counts.
pub const MIN_PRESENT: u32 = 2;

/// The left edge of the column the bar's name is drawn in.
pub const NAME_COLUMN_LEFT: f64 = 0.40;

/// The right edge of the column the bar's name is drawn in.
pub const NAME_COLUMN_RIGHT: f64 = 0.60;

/// A line over the name's column covers the bar when its top is at most this.
pub const NAME_COLUMN_MAX_TOP: f64 = 0.08;

/// What one read of the band says about one boss's bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BarClass {
    Present,
    Covered,
    Absent,
}

/// Classify one read of the band for `boss`.
pub fn classify(boss: &str, obs: &Observation) -> BarClass {
    if !obs.is_visible() {
        return BarClass::Covered;
    }
    let mut covering = false;
    for line in &obs.lines {
        if fuzzy_score(boss, &line.text) >= MATCH_THRESHOLD {
            return BarClass::Present;
        }
        if line
            .bbox
            .overlaps_columns(NAME_COLUMN_LEFT, NAME_COLUMN_RIGHT)
            && line.bbox.top <= NAME_COLUMN_MAX_TOP
        {
            covering = true;
        }
    }
    if covering {
        BarClass::Covered
    } else {
        BarClass::Absent
    }
}

/// One boss's bar over a sequence of reads of the band.
#[derive(Clone, Debug)]
pub struct BarTracker {
    boss: String,
    /// Present reads since the tracker last (re)armed.
    present: u32,
    last_present: Option<Millis>,
    /// Absent reads in a row since the last present one.
    absent_run: u32,
    /// A covered read came after the last present one.
    spoiled: bool,
}

impl BarTracker {
    pub fn new(boss: impl Into<String>) -> Self {
        BarTracker {
            boss: boss.into(),
            present: 0,
            last_present: None,
            absent_run: 0,
            spoiled: false,
        }
    }

    pub fn boss(&self) -> &str {
        &self.boss
    }

    /// Feed one read of the band; reads of other regions are ignored.
    /// Returns the read's class and the event it completed, if any.
    pub fn observe(&mut self, obs: &Observation) -> (BarClass, Option<ScreenEvent>) {
        let class = classify(&self.boss, obs);
        let mut event = None;
        match class {
            BarClass::Present => {
                self.present += 1;
                self.last_present = Some(obs.at);
                self.absent_run = 0;
                self.spoiled = false;
            }
            BarClass::Covered => {
                self.absent_run = 0;
                if self.present > 0 {
                    self.spoiled = true;
                }
            }
            BarClass::Absent => {
                if self.present >= MIN_PRESENT && !self.spoiled {
                    self.absent_run += 1;
                    if self.absent_run >= GONE_RUN {
                        if let Some(last_present) = self.last_present {
                            event = Some(ScreenEvent::BarGone {
                                boss: self.boss.clone(),
                                last_present,
                                at: obs.at,
                            });
                        }
                        self.rearm();
                    }
                }
            }
        }
        (class, event)
    }

    fn rearm(&mut self) {
        self.present = 0;
        self.last_present = None;
        self.absent_run = 0;
        self.spoiled = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Rect;
    use crate::observation::{Hidden, Line};
    use crate::region::Region;

    const BEIRA: &str = "Beira of the Rotten Pack";
    const MILLER: &str = "The Bloated Miller";

    /// The name where Live 8 read it, and where the inventory moved it.
    fn name_line(text: &str) -> Line {
        Line::new(text, Rect::new(0.456, 0.047, 0.544, 0.060))
    }
    fn moved_name_line(text: &str) -> Line {
        Line::new(text, Rect::new(0.28, 0.047, 0.37, 0.060))
    }
    fn passive_title() -> Line {
        Line::new("PASSIVE SKILLS", Rect::new(0.46, 0.02, 0.54, 0.04))
    }
    fn act_label() -> Line {
        Line::new("ACT 1", Rect::new(0.49, 0.06, 0.51, 0.07))
    }
    fn checkpoint_label(top: f64) -> Line {
        Line::new("CHECKPOINT", Rect::new(0.30, top, 0.35, top + 0.015))
    }

    /// Reads at 2 Hz from `start` ms, one per entry.
    fn reads(start: Millis, frames: Vec<Vec<Line>>) -> Vec<Observation> {
        frames
            .into_iter()
            .enumerate()
            .map(|(n, lines)| {
                Observation::visible(Region::BossBar, start + 500 * n as Millis, lines)
            })
            .collect()
    }

    fn run(tracker: &mut BarTracker, obs: &[Observation]) -> Vec<ScreenEvent> {
        obs.iter().filter_map(|o| tracker.observe(o).1).collect()
    }

    fn present(n: usize, boss: &str) -> Vec<Vec<Line>> {
        (0..n)
            .map(|_| vec![name_line(&boss.to_uppercase())])
            .collect()
    }
    fn absent(n: usize) -> Vec<Vec<Line>> {
        (0..n).map(|_| Vec::new()).collect()
    }

    #[test]
    fn constants_are_pinned() {
        assert_eq!(GONE_RUN, 6);
        assert_eq!(MIN_PRESENT, 2);
        assert_eq!(
            (NAME_COLUMN_LEFT, NAME_COLUMN_RIGHT, NAME_COLUMN_MAX_TOP),
            (0.40, 0.60, 0.08)
        );
    }

    #[test]
    fn bar_present_then_absent_run_is_gone_target() {
        let mut t = BarTracker::new(MILLER);
        let mut frames = present(4, MILLER);
        frames.extend(absent(GONE_RUN as usize + 10));
        let obs = reads(1_000, frames);
        let events = run(&mut t, &obs);
        assert_eq!(
            events,
            vec![ScreenEvent::BarGone {
                boss: MILLER.into(),
                last_present: 1_000 + 3 * 500,
                at: 1_000 + (3 + GONE_RUN as Millis) * 500,
            }]
        );
    }

    #[test]
    fn bar_one_absent_short_of_the_run_is_not_gone_baseline() {
        let mut t = BarTracker::new(MILLER);
        let mut frames = present(4, MILLER);
        frames.extend(absent(GONE_RUN as usize - 1));
        frames.extend(present(1, MILLER));
        assert!(run(&mut t, &reads(0, frames)).is_empty());
    }

    #[test]
    fn bar_under_a_covering_title_is_not_gone_baseline() {
        let mut t = BarTracker::new(BEIRA);
        let mut frames = present(4, BEIRA);
        // The passive tree opens: its title over the name's column, no name.
        frames.extend((0..10).map(|_| vec![passive_title()]));
        // It closes on an empty band: the covered reads spoil the run.
        frames.extend(absent(GONE_RUN as usize + 4));
        let obs = reads(0, frames);
        assert_eq!(classify(BEIRA, &obs[5]), BarClass::Covered);
        assert!(run(&mut t, &obs).is_empty());

        // The world map's act label covers the same way.
        let map = Observation::visible(Region::BossBar, 0, vec![act_label()]);
        assert_eq!(classify(BEIRA, &map), BarClass::Covered);
    }

    #[test]
    fn bar_world_label_off_the_name_column_does_not_cover() {
        let mut t = BarTracker::new(BEIRA);
        let mut frames = present(3, BEIRA);
        // After the kill a scene label reads left of the name's column, its
        // height moving with the camera.
        frames
            .extend((0..GONE_RUN as usize).map(|n| vec![checkpoint_label(0.03 + 0.01 * n as f64)]));
        let obs = reads(0, frames);
        assert_eq!(classify(BEIRA, &obs[3]), BarClass::Absent);
        let events = run(&mut t, &obs);
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0],
            ScreenEvent::BarGone {
                last_present: 1_000,
                ..
            }
        ));

        // A line in the column but below the bar's top rows does not cover
        // either.
        let low = Observation::visible(
            Region::BossBar,
            0,
            vec![Line::new("CHECKPOINT", Rect::new(0.45, 0.10, 0.55, 0.12))],
        );
        assert_eq!(classify(BEIRA, &low), BarClass::Absent);
    }

    #[test]
    fn bar_absent_with_no_present_frame_before_it_is_not_gone() {
        let mut t = BarTracker::new(BEIRA);
        assert!(run(&mut t, &reads(0, absent(30))).is_empty());
        // One present read is not enough either.
        let mut frames = present(1, BEIRA);
        frames.extend(absent(30));
        assert!(run(&mut t, &reads(20_000, frames)).is_empty());
    }

    #[test]
    fn bar_missed_name_in_some_frames_is_not_gone() {
        let mut t = BarTracker::new(BEIRA);
        let p = || vec![name_line("BEIRA OF THE ROTTEN PACK")];
        let noisy = || vec![name_line("BElRA OF THE R0TTEN PAGK")];
        let missed = Vec::new;
        let frames = vec![
            p(),
            p(),
            missed(),
            noisy(),
            missed(),
            missed(),
            p(),
            missed(),
            missed(),
            missed(),
            missed(),
            missed(),
            p(),
        ];
        assert!(run(&mut t, &reads(0, frames)).is_empty());
        // Then the bar goes for good.
        let events = run(&mut t, &reads(10_000, absent(GONE_RUN as usize)));
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0],
            ScreenEvent::BarGone {
                last_present: 6_000,
                ..
            }
        ));
    }

    #[test]
    fn bar_with_the_name_moved_by_a_panel_is_present() {
        let obs = Observation::visible(
            Region::BossBar,
            0,
            vec![moved_name_line("BEIRA OF THE ROTTEN PACK")],
        );
        assert_eq!(classify(BEIRA, &obs), BarClass::Present);
        // Present wins over a covering line in the same read.
        let both =
            Observation::visible(Region::BossBar, 0, vec![passive_title(), name_line(BEIRA)]);
        assert_eq!(classify(BEIRA, &both), BarClass::Present);
        // A short name matches the full bar title by containment.
        let blackjaw =
            Observation::visible(Region::BossBar, 0, vec![name_line("BLACKJAW, THE REMNANT")]);
        assert_eq!(classify("Blackjaw", &blackjaw), BarClass::Present);
    }

    #[test]
    fn bar_under_the_overlay_is_covered_and_not_gone() {
        let mut t = BarTracker::new(BEIRA);
        let mut obs = reads(0, present(4, BEIRA));
        obs.push(Observation::hidden(Region::BossBar, 2_000, Hidden::Overlay));
        obs.extend(reads(2_500, absent(GONE_RUN as usize + 4)));
        assert_eq!(t.observe(&obs[0]).0, BarClass::Present);
        assert_eq!(classify(BEIRA, &obs[4]), BarClass::Covered);
        assert!(run(&mut t, &obs[1..]).is_empty());
        // Every hidden reason counts as covered.
        for why in [
            Hidden::NotForeground,
            Hidden::Minimized,
            Hidden::CaptureFailed,
            Hidden::Black,
            Hidden::OcrFailed,
        ] {
            assert_eq!(
                classify(BEIRA, &Observation::hidden(Region::BossBar, 0, why)),
                BarClass::Covered
            );
        }
    }

    #[test]
    fn bar_gone_waits_for_the_bar_to_come_back() {
        let mut t = BarTracker::new(BEIRA);
        let mut frames = present(2, BEIRA);
        frames.extend(absent(GONE_RUN as usize * 4));
        assert_eq!(run(&mut t, &reads(0, frames)).len(), 1);
        let mut again = present(2, BEIRA);
        again.extend(absent(GONE_RUN as usize));
        let events = run(&mut t, &reads(60_000, again));
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0],
            ScreenEvent::BarGone {
                last_present: 60_500,
                ..
            }
        ));
    }

    #[test]
    fn bar_of_another_boss_in_the_name_column_covers() {
        let obs = Observation::visible(Region::BossBar, 0, vec![name_line("THE BLOATED MILLER")]);
        assert_eq!(classify(BEIRA, &obs), BarClass::Covered);
        let off_column = Observation::visible(
            Region::BossBar,
            0,
            vec![Line::new(
                "THE BLOATED MILLER",
                Rect::new(0.62, 0.047, 0.70, 0.060),
            )],
        );
        assert_eq!(classify(BEIRA, &off_column), BarClass::Absent);
    }
}
