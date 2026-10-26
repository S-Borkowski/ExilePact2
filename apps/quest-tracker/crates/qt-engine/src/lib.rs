//! Quest engine: loads and validates the quest data file, feeds `qt-logline`
//! events into per-character evidence, applies manual overrides and computes
//! each quest's status, current step and transitions.
//!
//! - [`data`]: the data contract (`data/quests.json`) and its validator,
//!   whose every refusal names the quest and the field.
//! - [`engine`]: the state machine, its character detection and the state
//!   the store persists as `serde_json::Value`.
//! - [`area`]: the current area read back from the log before a tailer's
//!   start position, to seed the engine with.

pub mod area;
pub mod data;
pub mod engine;

pub use area::{area_from_log, LogArea};
pub use data::{
    classify_stat, Area, DataError, Detection, Element, Evidence, Quest, QuestData, Remark,
    Reward, RewardCategory, ScreenWhat, Signal, Status, Step, Worth, SCHEMA, TIP_MAX_CHARS,
};
pub use engine::{
    CharacterInfo, CharacterNote, CharacterState, Engine, EngineError, Overrides, Ownership,
    QuestAction, QuestState, ScreenDemand, ScreenEvent, ScreenOutcome, ScreenResult, StepState,
    Transition,
};
/// The log time [`Engine::expire_relog_window`] takes.
pub use qt_logline::Timestamp;
