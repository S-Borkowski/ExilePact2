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
    /// A new line in the list of voiced lines carried a known remark text
    /// of the key (a quest moment's remark, by its poe2db key).
    Remark { key: String, at: Millis },
}

impl ScreenEvent {
    /// `bar_gone`, `quest_complete`, `kill_remark` or `remark`: the `what`
    /// of the screen signal it can fire.
    pub fn kind(&self) -> &'static str {
        match self {
            ScreenEvent::BarGone { .. } => "bar_gone",
            ScreenEvent::QuestComplete { .. } => "quest_complete",
            ScreenEvent::KillRemark { .. } => "kill_remark",
            ScreenEvent::Remark { .. } => "remark",
        }
    }

    /// The boss, NPC or remark key it is about.
    pub fn target(&self) -> &str {
        match self {
            ScreenEvent::BarGone { boss, .. } | ScreenEvent::KillRemark { boss, .. } => boss,
            ScreenEvent::QuestComplete { npc, .. } => npc,
            ScreenEvent::Remark { key, .. } => key,
        }
    }

    /// When the tracker saw it: the frame that completed it.
    pub fn at(&self) -> Millis {
        match self {
            ScreenEvent::BarGone { at, .. }
            | ScreenEvent::QuestComplete { at, .. }
            | ScreenEvent::KillRemark { at, .. }
            | ScreenEvent::Remark { at, .. } => *at,
        }
    }
}
