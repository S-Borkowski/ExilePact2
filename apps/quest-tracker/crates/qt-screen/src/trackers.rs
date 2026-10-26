//! The trackers for one demand, routed by region, as the reader and the
//! replay run them.
//!
//! A [`Demand`] names what the screen could finish now: the bosses whose bar
//! going would tick a step, the NPCs whose banner would, and the bosses whose
//! kill remark would, with the remark texts. [`Trackers::set_demand`] keeps
//! the state of every tracker still demanded and drops the rest, so a demand
//! republished unchanged after each batch costs nothing.

use crate::bar::{BarClass, BarTracker};
use crate::dialogue::DialogueTracker;
use crate::event::ScreenEvent;
use crate::observation::Observation;
use crate::region::Region;
use crate::remark::RemarkTracker;

/// A boss whose kill remark is demanded, with the known remark texts.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RemarkDemand {
    pub boss: String,
    pub texts: Vec<String>,
}

/// What the screen could finish now.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Demand {
    /// Bosses whose bar going would tick a step (`bar_gone`).
    pub bars: Vec<String>,
    /// NPCs whose "QUEST COMPLETE" banner would tick a step
    /// (`quest_complete`).
    pub dialogues: Vec<String>,
    /// Bosses whose kill remark would tick a step (`kill_remark`). An entry
    /// with no texts demands nothing.
    pub remarks: Vec<RemarkDemand>,
}

impl Demand {
    /// The regions this demand reads, in [`Region::ALL`]'s order.
    pub fn regions(&self) -> Vec<Region> {
        Region::ALL
            .into_iter()
            .filter(|r| match r {
                Region::BossBar => !self.bars.is_empty(),
                Region::VoicedLines => self.remarks.iter().any(|r| !r.texts.is_empty()),
                Region::DialogueWindow => !self.dialogues.is_empty(),
            })
            .collect()
    }

    /// Whether it reads nothing.
    pub fn is_empty(&self) -> bool {
        self.regions().is_empty()
    }
}

/// How the bar's reads classed, summed over every demanded boss, for the
/// reader's per-minute line.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BarCounts {
    pub present: u64,
    pub absent: u64,
    pub covered: u64,
}

/// Every tracker one demand needs.
#[derive(Clone, Debug, Default)]
pub struct Trackers {
    bars: Vec<BarTracker>,
    dialogues: Vec<DialogueTracker>,
    remarks: Vec<RemarkTracker>,
    counts: BarCounts,
}

impl Trackers {
    pub fn new() -> Self {
        Self::default()
    }

    /// Trackers for `demand`, fresh.
    pub fn for_demand(demand: &Demand) -> Self {
        let mut t = Self::new();
        t.set_demand(demand);
        t
    }

    /// Follow a new demand: keep each tracker still demanded, with its
    /// state, start the new ones, drop the others.
    pub fn set_demand(&mut self, demand: &Demand) {
        let mut bars = Vec::new();
        for boss in &demand.bars {
            if bars.iter().any(|t: &BarTracker| t.boss() == boss) {
                continue;
            }
            let kept = self.bars.iter().position(|t| t.boss() == boss);
            bars.push(
                kept.map(|i| self.bars.swap_remove(i))
                    .unwrap_or_else(|| BarTracker::new(boss.clone())),
            );
        }
        self.bars = bars;

        let mut dialogues = Vec::new();
        for npc in &demand.dialogues {
            if dialogues.iter().any(|t: &DialogueTracker| t.npc() == npc) {
                continue;
            }
            let kept = self.dialogues.iter().position(|t| t.npc() == npc);
            dialogues.push(
                kept.map(|i| self.dialogues.swap_remove(i))
                    .unwrap_or_else(|| DialogueTracker::new(npc.clone())),
            );
        }
        self.dialogues = dialogues;

        let mut remarks = Vec::new();
        for want in demand.remarks.iter().filter(|r| !r.texts.is_empty()) {
            if remarks
                .iter()
                .any(|t: &RemarkTracker| t.boss() == want.boss)
            {
                continue;
            }
            let kept = self
                .remarks
                .iter()
                .position(|t| t.boss() == want.boss && t.texts() == want.texts);
            remarks.push(
                kept.map(|i| self.remarks.swap_remove(i))
                    .unwrap_or_else(|| RemarkTracker::new(want.boss.clone(), want.texts.clone())),
            );
        }
        self.remarks = remarks;
    }

    /// The regions the current demand reads.
    pub fn regions(&self) -> Vec<Region> {
        Region::ALL
            .into_iter()
            .filter(|r| match r {
                Region::BossBar => !self.bars.is_empty(),
                Region::VoicedLines => !self.remarks.is_empty(),
                Region::DialogueWindow => !self.dialogues.is_empty(),
            })
            .collect()
    }

    /// Feed one read to the trackers of its region; returns their events.
    pub fn observe(&mut self, obs: &Observation) -> Vec<ScreenEvent> {
        let mut events = Vec::new();
        match obs.region {
            Region::BossBar => {
                for t in &mut self.bars {
                    let (class, event) = t.observe(obs);
                    match class {
                        BarClass::Present => self.counts.present += 1,
                        BarClass::Absent => self.counts.absent += 1,
                        BarClass::Covered => self.counts.covered += 1,
                    }
                    events.extend(event);
                }
            }
            Region::VoicedLines => {
                events.extend(self.remarks.iter_mut().filter_map(|t| t.observe(obs)))
            }
            Region::DialogueWindow => {
                events.extend(self.dialogues.iter_mut().filter_map(|t| t.observe(obs)))
            }
        }
        events
    }

    /// The bar's counts since the last call, reset.
    pub fn take_counts(&mut self) -> BarCounts {
        std::mem::take(&mut self.counts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Rect;
    use crate::observation::{Hidden, Line};

    fn demand() -> Demand {
        Demand {
            bars: vec!["Beira of the Rotten Pack".into()],
            dialogues: vec!["Renly".into()],
            remarks: vec![RemarkDemand {
                boss: "The Bloated Miller".into(),
                texts: vec!["Invented words.".into()],
            }],
        }
    }

    #[test]
    fn demand_names_its_regions() {
        assert_eq!(demand().regions(), Region::ALL.to_vec());
        assert!(Demand::default().is_empty());
        let no_texts = Demand {
            remarks: vec![RemarkDemand {
                boss: "X".into(),
                texts: vec![],
            }],
            ..Demand::default()
        };
        assert!(no_texts.is_empty());
        let bar_only = Demand {
            bars: vec!["Lythara".into()],
            ..Demand::default()
        };
        assert_eq!(bar_only.regions(), vec![Region::BossBar]);
        assert_eq!(
            Trackers::for_demand(&bar_only).regions(),
            vec![Region::BossBar]
        );
    }

    #[test]
    fn reads_go_to_their_region_and_state_survives_a_republished_demand() {
        let mut t = Trackers::for_demand(&demand());
        let name = || {
            vec![Line::new(
                "BEIRA OF THE ROTTEN PACK",
                Rect::new(0.456, 0.047, 0.544, 0.06),
            )]
        };
        let mut at = 0;
        for _ in 0..3 {
            assert!(t
                .observe(&Observation::visible(Region::BossBar, at, name()))
                .is_empty());
            at += 500;
        }
        // The worker republishes the same demand after a batch.
        t.set_demand(&demand());
        let mut events = Vec::new();
        for _ in 0..crate::bar::GONE_RUN {
            events.extend(t.observe(&Observation::visible(Region::BossBar, at, Vec::new())));
            at += 500;
        }
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind(), "bar_gone");
        assert_eq!(events[0].target(), "Beira of the Rotten Pack");
        t.observe(&Observation::hidden(Region::BossBar, at, Hidden::Overlay));
        assert_eq!(
            t.take_counts(),
            BarCounts {
                present: 3,
                absent: crate::bar::GONE_RUN as u64,
                covered: 1
            }
        );
        assert_eq!(t.take_counts(), BarCounts::default());
    }

    #[test]
    fn a_dropped_target_loses_its_state() {
        let mut t = Trackers::for_demand(&demand());
        let name = || {
            vec![Line::new(
                "BEIRA OF THE ROTTEN PACK",
                Rect::new(0.456, 0.047, 0.544, 0.06),
            )]
        };
        t.observe(&Observation::visible(Region::BossBar, 0, name()));
        t.observe(&Observation::visible(Region::BossBar, 500, name()));
        t.set_demand(&Demand::default());
        assert!(t.regions().is_empty());
        t.set_demand(&demand());
        let events: Vec<_> = (0..10)
            .flat_map(|n| {
                t.observe(&Observation::visible(
                    Region::BossBar,
                    1_000 + 500 * n,
                    Vec::new(),
                ))
            })
            .collect();
        assert!(events.is_empty());
    }
}
