//! The quest tracker's part of the update check and the in-app install
//! (Context "Per-app wiring"): which product it is, its tray menu, and the
//! shared update code (`ep-update`) re-exported under the names the app and
//! its glue use.
//!
//! Nothing here, or in `ep-update`, talks to the network. The requests are
//! an [`UpdateTransport`]'s two methods, which the glue implements over
//! `ep-net` and hands [`crate::App::with_updates`] in an [`UpdateConfig`];
//! `ep-update`'s updater decides when they are called and what their answers
//! mean, and calls them only on a thread of its own. A check is reached from
//! two places only: the manual request ([`crate::App::check_for_updates`],
//! the tray's *Check for updates* and the window's *Check now*) and the
//! worker's tick while the `check_updates` setting is on and a check is due.
//! With the setting off, which is the default and what a file written before
//! it existed loads, no tick ever reaches it. An install is reached from one
//! place only, the window's install button ([`crate::App::install_update`]):
//! never from a tick, the setting or the tray, whose *Update available: X*
//! opens the Settings window.
//!
//! When an automatic check is due: when no check (manual or automatic) has
//! started yet this run, when the last one started [`CHECK_INTERVAL_MS`]
//! ago, or when the last automatic one failed [`RETRY_AFTER_FAILURE_MS`]
//! ago. Times are the app's [`crate::Clock`]; nothing about checks is
//! persisted but the setting, so each launch with it on checks once.
//!
//! What a check reads: GitHub's release list of the repository. The newest
//! release is the highest non-draft, non-prerelease entry whose tag is
//! [`TAG_PREFIX`] and exactly `<major>.<minor>.<patch>`; anything else is
//! skipped, never an error ([`newest_release`]). Of that release the parse
//! keeps the `quest-tracker.exe` asset and its `.minisig`, which an install
//! downloads by version and name, never by a URL from the answer. A release
//! without them is shown as not signed, with no install button. No page is
//! opened for an update, and nothing of an answer's body is ever logged or
//! shown.

pub use ep_update::{
    cleanup_line, newest_release, outcome_line, outcome_of, spawn_cleanup, started_line,
    transport_reason, turned_line, Answer,
    AssetKind, DownloadError, InstallProgress, InstallState, InstallView,
    Outcome, Product, Release, ReleaseAsset, TransportError, Unreadable, UpdateConfig,
    UpdateState, UpdateTransport, UpdateView, Version, CHECK_INTERVAL_MS, NO_TRANSPORT,
    RETRY_AFTER_FAILURE_MS,
};

/// Quest Tracker's flat Settings snapshot, extended only with its local theme.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AppSettingsView {
    pub version: String,
    pub check_updates: bool,
    pub update: UpdateView,
    pub install: InstallView,
    pub theme: qt_viewmodel::Theme,
}

impl AppSettingsView {
    pub fn with_theme(view: ep_update::AppSettingsView, theme: qt_viewmodel::Theme) -> Self {
        Self { version: view.version, check_updates: view.check_updates, update: view.update, install: view.install, theme }
    }
}

/// The product the quest tracker's updater works for: "Quest Tracker", its
/// tags `quest-tracker-v<version>` and its exe asset `quest-tracker.exe`.
pub const PRODUCT: Product = ep_update::QUEST_TRACKER;

/// The tag prefix of a Quest Tracker release (`tools/quest_tracker/release.py`'s
/// `TAG_PREFIX`); the repository's other tools tag their own.
pub const TAG_PREFIX: &str = PRODUCT.tag_prefix;

/// This build's version: the crate's, read up to any `-` or `+` suffix
/// (0.0.0 should it ever not parse).
pub fn running_version() -> Version {
    Version::of_package(env!("CARGO_PKG_VERSION"))
}

/// The tray icon's tooltip with no newer version known.
pub const TRAY_TOOLTIP: &str = "Quest Tracker";

/// The tray menu's entry ids (see [`tray_menu`]).
pub const TRAY_UPDATE_AVAILABLE: &str = "update-available";
pub const TRAY_CHECK_UPDATES: &str = "check-updates";
pub const TRAY_SETTINGS: &str = "settings";
pub const TRAY_QUIT: &str = "quit";

/// One entry of the tray menu, top to bottom.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrayItem {
    Entry { id: &'static str, text: String },
    Separator,
}

/// The tray icon's menu and tooltip for an app-settings view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayMenu {
    pub items: Vec<TrayItem>,
    pub tooltip: String,
}

/// What a tray entry does, by its id. No entry installs: the install button
/// is the Settings window's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayAction {
    /// Opens or focuses the Settings window and starts a manual check.
    CheckForUpdates,
    /// Opens or focuses the Settings window (*Settings*, and *Update
    /// available: X*, whose install button is there).
    OpenSettings,
    Quit,
}

impl TrayAction {
    pub fn from_id(id: &str) -> Option<TrayAction> {
        match id {
            TRAY_UPDATE_AVAILABLE | TRAY_SETTINGS => Some(TrayAction::OpenSettings),
            TRAY_CHECK_UPDATES => Some(TrayAction::CheckForUpdates),
            TRAY_QUIT => Some(TrayAction::Quit),
            _ => None,
        }
    }
}

/// The tray menu: *Update available: X* only while a newer version is
/// known, then *Check for updates*, *Settings*, a separator and *Quit Quest
/// Tracker*; the tooltip names the newer version too.
pub fn tray_menu(view: &AppSettingsView) -> TrayMenu {
    let newer = view.update.newer.as_deref();
    let entry = |id, text: &str| TrayItem::Entry {
        id,
        text: text.to_string(),
    };
    let mut items = Vec::new();
    if let Some(v) = newer {
        items.push(entry(TRAY_UPDATE_AVAILABLE, &format!("Update available: {v}")));
    }
    items.push(entry(TRAY_CHECK_UPDATES, "Check for updates"));
    items.push(entry(TRAY_SETTINGS, "Settings"));
    items.push(TrayItem::Separator);
    items.push(entry(TRAY_QUIT, "Quit Quest Tracker"));
    TrayMenu {
        items,
        tooltip: match newer {
            Some(v) => format!("{TRAY_TOOLTIP} ({v} available)"),
            None => TRAY_TOOLTIP.to_string(),
        },
    }
}
