//! The views the windows render (Context "The IPC contract"): [`View`],
//! which the `ch:view` event carries to the `main` and `overlay` windows and
//! the `get_view` command returns, and [`SettingsView`], which the
//! `ch:app-settings` event carries to the `settings` window and its commands
//! return. `src/ipc.ts` mirrors these types field for field, and the
//! fixtures under `fixtures/` hold one serialised view per state the page
//! tests render (`fixtures_are_current`).
//!
//! Every type of the main view denies unknown fields, so a field either
//! side adds, drops or renames fails the fixture test. [`SettingsView`]
//! flattens `ep-update`'s [`AppSettingsView`] in, which serde cannot combine
//! with denying unknown fields; the fixture test compares its JSON instead.

use ep_update::AppSettingsView;
use serde::{Deserialize, Serialize};

/// Where a grab's text came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrabSource {
    /// The app's global hotkey read the clipboard once.
    Shortcut,
    /// A paste into the main window; no clipboard was read.
    Paste,
}

/// The last item text grabbed, kept in memory only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrabView {
    /// The text, normalised: LF line breaks, no trailing blank lines.
    pub text: String,
    pub line_count: usize,
    pub source: GrabSource,
    /// The first line starts like the game's copied item text.
    pub looks_like_item: bool,
}

/// Whether the status line reports or warns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusKind {
    Info,
    Error,
}

/// One status line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatusLine {
    pub kind: StatusKind,
    pub text: String,
}

impl StatusLine {
    pub fn info(text: impl Into<String>) -> StatusLine {
        StatusLine {
            kind: StatusKind::Info,
            text: text.into(),
        }
    }

    pub fn error(text: impl Into<String>) -> StatusLine {
        StatusLine {
            kind: StatusKind::Error,
            text: text.into(),
        }
    }
}

/// The overlay window's state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverlayView {
    /// True only while a craft is active.
    pub visible: bool,
    /// The one line the overlay shows.
    pub line: String,
}

/// Everything the main window and the overlay show.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct View {
    /// The global hotkey registered now (or wanted, when it was refused).
    pub hotkey: String,
    /// The last grab; `None` until one passes.
    pub grab: Option<GrabView>,
    pub status: Option<StatusLine>,
    pub overlay: OverlayView,
    /// The newer version a check found this run (`1.3.0`), which the
    /// header's *Settings* button names; `None` while none is known.
    pub update_available: Option<String>,
}

/// Everything the Settings window shows: `ep-update`'s view of the update
/// check and install, then the hotkey and the blocked currencies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingsView {
    #[serde(flatten)]
    pub updates: AppSettingsView,
    pub hotkey: String,
    /// What the last hotkey change, or the start, said about the hotkey.
    pub hotkey_status: Option<StatusLine>,
    pub blocked_currencies: Vec<String>,
}
