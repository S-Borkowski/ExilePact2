//! The waystone roller's logic without Tauri: the mod data built into the
//! executable, the store, the selection and its persistence, the view, the
//! commands, the shortcut and the one copy path, and the diagnostic log.
//!
//! `src-tauri` is glue around this crate (as the quest tracker's `qt-app`
//! and `src-tauri` split it): it implements [`Shell`] (the global shortcut,
//! the Win32 clipboard writer, the error beep, the `wr:view` and
//! `wr:app-settings` events, the Settings window and the quit that restarts
//! into an installed update), hands [`App::with_updates`] the update
//! transport over `ep-net` and the running exe's path, and forwards each IPC
//! command to the [`App`] method of the same name. Keeping every decision
//! here means it tests with a recording fake shell, a fake transport and a
//! temporary store, without a WebView, a built UI or the network.
//!
//! - [`app`]: [`App`], its commands, the copy path and the update commands
//!   (`get_app_settings`, `update_settings`, `check_for_updates`,
//!   `install_update`, `cancel_update`, `open_settings`).
//! - [`updates`]: what the update check is built from ([`UpdateSetup`]), its
//!   clock and ticker interval. The check, the install of a signed release
//!   and the Settings window's view are `ep-update`'s, shared with the quest
//!   tracker; no URL is opened anywhere.
//! - [`view`]: [`View`] and its parts, the IPC contract `src/ipc.ts`
//!   mirrors (Context "The IPC contract").
//! - [`store`]: the store folder (refused inside a git checkout),
//!   `settings.json` (the shortcut, the selection and `check_updates`)
//!   written atomically, and `diag.log`.

pub mod app;
pub mod store;
pub mod updates;
pub mod view;

pub use app::{App, AppConfig, BeepAttempt, HotkeyError, Shell, DEFAULT_HOTKEY, SAVE_DELAY};
pub use store::{default_dir, resolve_dir, DiagLog, Store, StoreError, DIR_ENV};
pub use updates::{Clock, SteadyClock, UpdateSetup, TICK_EVERY};
pub use view::{
    DataInfo, ModView, PartView, RarityView, ResultState, SearchResult, SelectionView, StatusKind,
    StatusLine, View,
};

/// The shared update crate, for the glue's transport and start-up cleanup.
pub use ep_update;
pub use ep_update::{
    Answer, AppSettingsView, AssetKind, DownloadError, InstallState, Product, TransportError,
    UpdateConfig, UpdateState, UpdateTransport, Version, WAYSTONE_ROLLER,
};

use wr_regex::ModData;

/// The mod data built into the executable.
pub const EMBEDDED_DATA: &str = include_str!("../../../data/waystone-mods.json");

/// The built-in mod data. It fails only in a broken build, which the
/// shipped-data test (`tests/shipped.rs`) stops before release.
pub fn embedded_data() -> Result<ModData, String> {
    ModData::parse(EMBEDDED_DATA).map_err(|e| format!("the built-in mod data is broken: {e}"))
}
