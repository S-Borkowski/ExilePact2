//! The crafting helper's logic without Tauri: the store, the grab of an
//! item's text on demand, the hotkey's rules, the saved settings, the
//! overlay's state, the views, the commands and the diagnostic log.
//!
//! The player copies an item with the game's own Ctrl+Alt+C, then presses
//! the app's global hotkey (the clipboard is read once, for that press) or
//! pastes into the main window (the page sends the pasted text; no
//! clipboard is read). The app never watches the clipboard, never hooks the
//! keyboard and never sends input.
//!
//! `src-tauri` is glue around this crate (as the waystone roller's `wr-app`
//! and `src-tauri` split it): it implements [`Shell`] (the global hotkey,
//! the Win32 clipboard read, the error beep, the `ch:view` and
//! `ch:app-settings` events, the overlay's show and hide, the Settings
//! window and the quit that restarts into an installed update), hands
//! [`App::with_updates`] the update transport over `ep-net` and the running
//! exe's path, and forwards each IPC command to the [`App`] method of the
//! same name. Keeping every decision here means it tests with a recording
//! fake shell, a fake transport and a temporary store, without a WebView, a
//! built UI, the game or the network.
//!
//! - [`app`]: [`App`], its commands (`get_view`, `grab_text`,
//!   `open_settings`, `get_app_settings`, `update_settings`, `set_hotkey`,
//!   `check_for_updates`, `install_update`, `cancel_update`), the hotkey
//!   press and [`App::set_craft_active`].
//! - [`grab`]: the checks and the normalisation of a grab's text.
//! - [`hotkey`]: the default hotkey and the game's item copy it may not be.
//! - [`settings`]: what `update_settings` takes, and the blocked currencies.
//! - [`updates`]: what the update check is built from ([`UpdateSetup`]), its
//!   clock and ticker interval. The check, the install of a signed release
//!   and the Settings window's update view are `ep-update`'s, shared with
//!   the quest tracker and the waystone roller; no URL is opened anywhere.
//! - [`view`]: [`View`] and [`SettingsView`], the IPC contract `src/ipc.ts`
//!   mirrors (Context "The IPC contract").
//! - [`store`]: the store folder (refused inside a git checkout),
//!   `settings.json` written atomically, and `diag.log`.
//! - [`ch_engine`], re-exported for the craft window (M5): the blueprints,
//!   the route a craft takes, the hit, miss or unexpected outcome of each
//!   step, the next step's odds and the catalogue the blocked list offers;
//!   through it the data ([`ch_data`]), the blueprint format
//!   ([`ch_blueprint`]) and the item parser ([`ch_item`]). No command,
//!   view or setting uses it yet.

pub mod app;
pub mod grab;
pub mod hotkey;
pub mod settings;
pub mod store;
pub mod updates;
pub mod view;

pub use app::{App, AppConfig, BeepAttempt, HotkeyError, Shell, OVERLAY_PLACEHOLDER, SAVE_DELAY};
pub use grab::{ITEM_FIRST_LINE, MAX_GRAB_BYTES};
pub use hotkey::{is_game_item_copy, DEFAULT_HOTKEY, GAME_ITEM_COPY};
pub use settings::{MAX_BLOCKED, MAX_ID_CHARS};
pub use store::{default_dir, resolve_dir, DiagLog, Store, StoreError, DIR_ENV};
pub use updates::{Clock, SteadyClock, UpdateSetup, TICK_EVERY};
pub use view::{GrabSource, GrabView, OverlayView, SettingsView, StatusKind, StatusLine, View};

/// The crafting engine, for the craft window, and the crates it re-exports.
pub use ch_engine;
pub use ch_engine::{ch_blueprint, ch_data, ch_item};

/// The shared update crate, for the glue's transport and start-up cleanup.
pub use ep_update;
pub use ep_update::{
    Answer, AppSettingsView, AssetKind, DownloadError, InstallState, Product, TransportError,
    UpdateConfig, UpdateState, UpdateTransport, Version, CRAFTING_HELPER,
};
