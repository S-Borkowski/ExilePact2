//! The waystone roller's logic without Tauri: the mod data built into the
//! executable, the store, the selection and its persistence, the view, the
//! commands, the shortcut and the one copy path, and the diagnostic log.
//!
//! `src-tauri` is glue around this crate (as the quest tracker's `qt-app`
//! and `src-tauri` split it): it implements [`Shell`] (the global shortcut,
//! the Win32 clipboard writer, the error beep and the `wr:view` event) and
//! forwards each IPC command to the [`App`] method of the same name. Keeping
//! every decision here means it tests with a recording fake shell and a
//! temporary store, without a WebView or a built UI.
//!
//! - [`app`]: [`App`], its commands and the copy path (Context "The app
//!   shell", "Pinned texts", "Hotkey conflicts").
//! - [`view`]: [`View`] and its parts, the IPC contract `src/ipc.ts`
//!   mirrors (Context "The IPC contract").
//! - [`store`]: the store folder (refused inside a git checkout),
//!   `settings.json` written atomically, and `diag.log`.

pub mod app;
pub mod store;
pub mod view;

pub use app::{App, AppConfig, HotkeyError, Shell, DEFAULT_HOTKEY, SAVE_DELAY};
pub use store::{default_dir, resolve_dir, DiagLog, Store, StoreError, DIR_ENV};
pub use view::{
    DataInfo, ModView, PartView, RarityView, ResultState, SearchResult, SelectionView, StatusKind,
    StatusLine, View,
};

use wr_regex::ModData;

/// The mod data built into the executable.
pub const EMBEDDED_DATA: &str = include_str!("../../../data/waystone-mods.json");

/// The built-in mod data. It fails only in a broken build, which the
/// shipped-data test (`tests/shipped.rs`) stops before release.
pub fn embedded_data() -> Result<ModData, String> {
    ModData::parse(EMBEDDED_DATA).map_err(|e| format!("the built-in mod data is broken: {e}"))
}
