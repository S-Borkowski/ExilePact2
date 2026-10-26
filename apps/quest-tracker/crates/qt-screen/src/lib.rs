//! The screen reader's pure part: what the overlay reads of the game's
//! window, and what it makes of it. No Windows dependency, no thread, no
//! clock: the reader in `qt-app` drives it live through a [`ScreenBackend`]
//! (the real one is `qt-capture`), and `quest-tracker-replay screen` drives
//! it over the kept captures of the research sessions.
//!
//! - [`Region`]: the three regions the reader may copy (the boss bar's band,
//!   the lower-left list of voiced lines and the dialogue window), as
//!   fractions of the game's client rect, with their read rates. Positions
//!   were measured only at 16:9; [`aspect_is_16_9`] is the reader's refusal.
//! - [`Observation`]: one read of one region, its OCR [`Line`]s with boxes
//!   as fractions of the client rect, or the [`Hidden`] reason it was not
//!   visible.
//! - [`fuzzy_score`]: the research instruments' fuzzy match, ported with
//!   identical scores (a parity fixture pins them).
//! - The trackers, each fed reads of its region in time order:
//!   [`BarTracker`] (`bar_gone`), [`DialogueTracker`] (`quest_complete`) and
//!   [`RemarkTracker`] (`kill_remark`), gathered for one [`Demand`] by
//!   [`Trackers`]. They report [`ScreenEvent`]s.
//!
//! Times are [`Millis`] on one clock the caller supplies: the reader passes a
//! monotonic clock, the replay UTC from the capture's index.
//!
//! OCR text is private. It lives in memory only while a read is processed;
//! the trackers keep matches and counts between reads, never text; events
//! carry names taken from the demand; and the `Debug` forms of lines print
//! their length instead of their text. Other crates' tests use the public
//! constants here, never copied numbers, because the replay settles them.

mod backend;
mod bar;
mod dialogue;
mod event;
mod fuzzy;
mod geometry;
mod observation;
mod region;
mod remark;
mod trackers;

/// Milliseconds on the caller's clock.
pub type Millis = i64;

pub use backend::{BackendError, OcrReadiness, OverlayState, ScreenBackend, WindowHandle};
pub use bar::{
    classify as classify_bar, BarClass, BarTracker, GONE_RUN, MIN_PRESENT, NAME_COLUMN_LEFT,
    NAME_COLUMN_MAX_TOP, NAME_COLUMN_RIGHT,
};
pub use dialogue::{
    is_banner, is_heading, DialogueTracker, BANNER_AREA, BANNER_REARM_MS, BANNER_TEXT,
    HEADING_SLACK, HEADING_WINDOW_MS,
};
pub use event::ScreenEvent;
pub use fuzzy::{fuzzy_score, matches, normalize, normalized_len, MATCH_THRESHOLD, WINDOW_SLACK};
pub use geometry::{aspect_is_16_9, PixelRect, Rect, ASPECT_TOLERANCE};
pub use observation::{
    Hidden, Image, Line, Observation, PixelLine, Visibility, BLACK_LUMA_MEAN, BLACK_LUMA_STD,
};
pub use region::{
    Region, BOSS_BAR_PERIOD_MS, BOSS_BAR_RECT, DIALOGUE_WINDOW_PERIOD_MS, DIALOGUE_WINDOW_RECT,
    VOICED_LINES_PERIOD_MS, VOICED_LINES_RECT,
};
pub use remark::{remark_body, RemarkTracker, NEW_FROM_MS, NEW_TO_MS};
pub use trackers::{BarCounts, Demand, RemarkDemand, Trackers};
