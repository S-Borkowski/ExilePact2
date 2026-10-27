//! What the trackers report. An event carries a name taken from the demand
//! (the quest data), never from OCR text, and times on the caller's clock.

use crate::Millis;

/// One thing the screen showed that a quest step may count.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScreenEvent {
    /// The boss's bar went: GONE_RUN absent frames after it was present.
    BarGone {
        boss: String,
        last_present: Millis,
        at: Millis,
    },
    /// "QUEST COMPLETE" showed in the NPC's dialogue window.
    QuestComplete { npc: String, at: Millis },
    /// A new line in the list of voiced lines carried a known kill remark
    /// for the boss.
    KillRemark { boss: String, at: Millis },
}

impl ScreenEvent {
    /// `bar_gone`, `quest_complete` or `kill_remark`: the `what` of the
    /// screen signal it can fire.
    pub fn kind(&self) -> &'static str {
        match self {
            ScreenEvent::BarGone { .. } => "bar_gone",
            ScreenEvent::QuestComplete { .. } => "quest_complete",
            ScreenEvent::KillRemark { .. } => "kill_remark",
        }
    }

    /// The boss or NPC it is about.
    pub fn target(&self) -> &str {
        match self {
            ScreenEvent::BarGone { boss, .. } | ScreenEvent::KillRemark { boss, .. } => boss,
            ScreenEvent::QuestComplete { npc, .. } => npc,
        }
    }

    /// When the tracker saw it: the frame that completed it.
    pub fn at(&self) -> Millis {
        match self {
            ScreenEvent::BarGone { at, .. }
            | ScreenEvent::QuestComplete { at, .. }
            | ScreenEvent::KillRemark { at, .. } => *at,
        }
    }
}
