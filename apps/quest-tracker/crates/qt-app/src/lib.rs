//! Overlay shell logic without Tauri: app state behind one lock, the worker
//! thread that drains the tailer into the engine and announces a fresh view,
//! the IPC command handlers, mode and click-through state, debounced saves,
//! command-line options and log path discovery.
//!
//! `src-tauri` is glue around this crate: it implements [`Emitter`] (the
//! `qt:view` and `qt:toast` events) and [`Window`] (click-through, zoom,
//! placement, the global shortcut, opening a link, the monitor's metrics
//! for the `auto` scale) and forwards each IPC
//! command to the [`App`] method of the same name. Keeping the logic here
//! means it tests with a fake emitter, clock and window, without a WebView
//! or a built UI.
//!
//! - [`app`]: [`App`], its threads and its commands (Context "Overlay window
//!   and threads", "IPC contract").
//! - [`cli`]: `--log PATH`, `--data PATH`, `--from-start`.
//! - [`discover`]: which `Client.txt` to follow.
//! - [`diag`]: what the diagnostic log may say.

pub mod app;
pub mod cli;
pub mod diag;
pub mod discover;

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use qt_engine::QuestData;

pub use app::{
    scale_factor, App, AppConfig, Clock, Emitter, MonitorMetrics, Placement, SystemClock, Window,
    SAVE_INTERVAL_MS, SHUTDOWN_TIMEOUT, WORKER_TICK,
};
pub use cli::Options;
// What the glue names without depending on the library crates itself.
pub use qt_engine::QuestAction;
pub use qt_store::Store;
pub use qt_tailer::DEFAULT_POLL_INTERVAL;
pub use qt_viewmodel::{Dock, Frame, Mode, Toast, View, ViewKind};

/// The quest data built into the executable.
pub const EMBEDDED_DATA: &str = include_str!("../../../data/quests.json");

/// The quest data: the file `path` names, else the data built in. A file
/// that cannot be used falls back to the built-in data, and built-in data
/// that cannot be used (a broken build) to no quests at all; either way the
/// second value is one sentence for the status line.
pub fn load_data(path: Option<&Path>) -> (Arc<QuestData>, Option<String>) {
    let mut note = None;
    if let Some(p) = path {
        match QuestData::load(p) {
            Ok(d) => return (Arc::new(d), None),
            Err(e) => {
                note = Some(format!(
                    "Quest data {} could not be used ({e}); the built-in data is in use.",
                    p.display()
                ))
            }
        }
    }
    match QuestData::parse(EMBEDDED_DATA) {
        Ok(d) => (Arc::new(d), note),
        Err(e) => (
            Arc::new(empty_data()),
            Some(format!("The built-in quest data is broken ({e}); no quests are listed.")),
        ),
    }
}

/// Quest data with no quests, for a build whose own data is broken.
fn empty_data() -> QuestData {
    QuestData {
        schema: qt_engine::SCHEMA,
        game_version: String::new(),
        poe2db_fetched: String::new(),
        source: String::new(),
        source_note: String::new(),
        npcs: Vec::new(),
        system_lines: Vec::new(),
        areas: BTreeMap::new(),
        quests: Vec::new(),
    }
}
