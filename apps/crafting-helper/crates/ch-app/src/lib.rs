//! The crafting helper's logic without Tauri: the store, the grab of an
//! item's text on demand, the hotkey's rules, the saved settings, the craft
//! window (the planning item, its targets, the route preview and the
//! guide), the overlay over the game while a craft is guiding, the watch
//! thread that tracks the game's window, the views, the commands and the
//! diagnostic log.
//!
//! The player copies an item with the game's own Ctrl+Alt+C, then presses
//! the app's global hotkey (the clipboard is read once, for that press) or
//! pastes into the main window (the page sends the pasted text; no
//! clipboard is read). While a craft is guiding, the app can also read the
//! hovered item's tooltip from a copy of the game's window with Windows'
//! OCR, which the player may hold Alt for themselves. The app never watches
//! the clipboard, never hooks the keyboard, reads no input state and never
//! sends input.
//!
//! `src-tauri` is glue around this crate (as the waystone roller's `wr-app`
//! and `src-tauri` split it): it implements [`Shell`] (the global hotkey,
//! the Win32 clipboard read, the error beep, the `ch:view` and
//! `ch:app-settings` events, the overlay's size, placement, show and hide,
//! the Settings window, opening [`CRAFT_OF_EXILE_URL`] in the default
//! browser and the quit that restarts into an installed update) and
//! [`ScreenBackend`] (the game's window when it is in front, by
//! [`GAME_WINDOW_TITLE`], and its client area; whether Windows' OCR reads
//! English; the item tooltips read from that client area by `ch-watch`'s
//! reader over the quest tracker's capture), hands
//! [`App::with_updates`] the update transport over `ep-net` and the running
//! exe's path, and forwards each IPC command to the [`App`] method of the
//! same name. Keeping every decision here means it tests with a recording
//! fake shell, a fake screen backend, a fake transport and a temporary
//! store, without a WebView, a built UI, the game or the network.
//!
//! - [`app`]: [`App`], its commands (`get_view`, `grab_text`,
//!   `open_settings`, `get_app_settings`, `update_settings`, `set_hotkey`,
//!   `get_catalogue`, `check_for_updates`, `install_update`,
//!   `cancel_update`, and the craft window's `set_targets`,
//!   `set_craft_blocked`, `choose_route`, `accept_craft`, `end_craft` and
//!   `open_craft_of_exile`), the hotkey press, the overlay's visibility and
//!   placement from the watch thread's reports, a hit or a miss the watch
//!   read from the screen kept as a grab's would be, and the watch line
//!   ([`WatchLineView`]) ([`App::tick_watch`] ticks the thread by hand for
//!   the tests and fixtures).
//! - [`overlay`]: where the overlay sits inside the game's client area
//!   ([`overlay_origin`], at the `overlay_position` setting, inset
//!   [`OVERLAY_INSET`]) and what it shows ([`OverlayGuide`]: the current
//!   step, both branch lines, the last outcome and the watch line).
//! - [`screen`]: the screen backend ([`ScreenBackend`], optional in
//!   [`AppConfig`]; without one the overlay never shows and nothing is
//!   read) and the watch thread, which looks at the game's window every
//!   [`WINDOW_EVERY`] while a craft is guiding and calls nothing otherwise;
//!   while the guide is active, `screen_watch` is on, the OCR reads English
//!   and the game is in front, it reads the hovered item's tooltip at most
//!   every [`READ_EVERY`], matches it against the item the step was shown
//!   on ([`ch_watch`]) and reports a change read twice alike, at least
//!   [`CONFIRM_AFTER`] apart. No copy, crop or OCR line leaves the read.
//! - [`craft`]: the craft window's state (none, refused, planning,
//!   guiding; in memory only), what a grab does to it, its commands' rules
//!   and its view, built from the shipped engine's plan, guide and
//!   outcomes, with the copy the design marks Rust's (odds, fallbacks,
//!   floors, branches, history summaries).
//! - [`grab`]: the checks and the normalisation of a grab's text.
//! - [`hotkey`]: the default hotkey and the game's item copy it may not be.
//! - [`settings`]: what `update_settings` takes, the overlay's position
//!   and the screen watch, the blocked currencies (catalogue ids only: the
//!   catalogue is the list's limit) and the
//!   catalogue grouped by kind for the pickers ([`CatalogueGroup`]).
//! - [`updates`]: what the update check is built from ([`UpdateSetup`]), its
//!   clock and ticker interval. The check, the install of a signed release
//!   and the Settings window's update view are `ep-update`'s, shared with
//!   the quest tracker and the waystone roller.
//! - [`view`]: [`View`] (with its craft part, [`CraftView`]) and
//!   [`SettingsView`], the IPC contract `src/ipc.ts` mirrors.
//! - [`store`]: the store folder (refused inside a git checkout),
//!   `settings.json` written atomically, and `diag.log`.
//! - [`ch_engine`], re-exported for the craft window (M5): the blueprints,
//!   the route a craft takes, the hit, miss or unexpected outcome of each
//!   step, the next step's odds and the catalogue the blocked list offers;
//!   through it the data ([`ch_data`]), the blueprint format
//!   ([`ch_blueprint`]) and the item parser ([`ch_item`]). The craft
//!   window plans with the shipped blueprints, loaded once; the blocked
//!   list and `get_catalogue` read its catalogue.
//!
//! The one address the app opens is [`CRAFT_OF_EXILE_URL`], from
//! `open_craft_of_exile`, which takes no address.

pub mod app;
pub mod craft;
pub mod grab;
pub mod hotkey;
pub mod overlay;
pub mod screen;
pub mod settings;
pub mod store;
pub mod updates;
pub mod view;

pub use app::{App, AppConfig, BeepAttempt, HotkeyError, Shell, SAVE_DELAY};
pub use craft::CRAFT_OF_EXILE_URL;
pub use grab::{ITEM_FIRST_LINE, MAX_GRAB_BYTES};
pub use hotkey::{is_game_item_copy, DEFAULT_HOTKEY, GAME_ITEM_COPY};
pub use overlay::{overlay_origin, OVERLAY_INSET};
pub use screen::{
    ClientRect, GameWindow, OcrReadiness, ScreenBackend, ScreenSetup, WatchFailure, BLACK_IN_A_ROW,
    CONFIRM_AFTER, GAME_WINDOW_TITLE, MINUTE_EVERY, READ_EVERY, WATCH_STOP_DEADLINE, WINDOW_EVERY,
};
pub use settings::{CatalogueEntry, CatalogueGroup, MAX_ID_CHARS};
pub use store::{default_dir, resolve_dir, DiagLog, Store, StoreError, DIR_ENV};
pub use updates::{Clock, SteadyClock, UpdateSetup, TICK_EVERY};
pub use view::{
    BlockedFrom, ChosenTarget, CraftBlockedEntry, CraftBlockedView, CraftItemView, CraftMode,
    CraftView, CurrentStepView, FloorOption, GrabSource, GrabView, GuideStatusView, GuidingView,
    HistoryEntry, ItemModView, ModSide, OutcomeSource, OutcomeWord, OverlayGuide, OverlayPosition,
    OverlayView, PlanningView, PreviewView, RejectedLine, RouteChoice, RouteView, SettingsView,
    StatusKind, StatusLine, StepLineView, TargetChoice, TargetChoices, View, WatchLineView,
};

/// The crafting engine, for the craft window, and the crates it re-exports.
pub use ch_engine;
pub use ch_engine::{ch_blueprint, ch_data, ch_item};

/// The screen watch's pure part (the tooltip read, the matcher), for the
/// glue's backend, which runs its reader, and the tests' fake one.
pub use ch_watch;
pub use ch_watch::{ReadCounts, TooltipBox, TooltipRead, Unreadable};

/// The shared update crate, for the glue's transport and start-up cleanup.
pub use ep_update;
pub use ep_update::{
    Answer, AppSettingsView, AssetKind, DownloadError, InstallState, Product, TransportError,
    UpdateConfig, UpdateState, UpdateTransport, Version, CRAFTING_HELPER,
};
