//! What the Settings window renders, its texts, and the `updates:`
//! diagnostic lines. Each app serialises [`AppSettingsView`] as it is and
//! mirrors it in TypeScript; no part of it carries a URL.

use serde::{Deserialize, Serialize};

use crate::check::{Outcome, Release};
use crate::version::{Product, Version};

/// The view's `update.state`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UpdateState {
    Idle,
    Checking,
    UpToDate,
    Available,
    NoRelease,
    Failed,
}

/// The check part of [`AppSettingsView`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateView {
    pub state: UpdateState,
    /// The status line, in words.
    pub text: String,
    /// The newest version known newer than the running one from any
    /// successful check this run: what the install button and the app's
    /// own update hints follow.
    pub newer: Option<String>,
}

/// The view's `install.state`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InstallState {
    /// No newer version is known: nothing to install.
    None,
    /// A newer version is known, but its release lists no exe or no
    /// `.minisig`: no button, and no *Try again* (it would fail the same way).
    Unsigned,
    /// A newer, signed version is known: the install button.
    Ready,
    /// The exe is downloading: the progress and *Cancel*.
    Downloading,
    /// The download is complete and its signature is being checked.
    Verifying,
    /// The new exe is in place; the app is restarting into it.
    Restarting,
    /// The install failed with a reason; *Try again*. The running exe is
    /// untouched.
    Failed,
}

/// How far a download is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallProgress {
    pub bytes: u64,
    /// The size the release lists, which the download must match exactly.
    pub total: Option<u64>,
}

/// The install part of [`AppSettingsView`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallView {
    pub state: InstallState,
    /// `ready`: the install button's label; every other state but `none`:
    /// the status line, in words; `none`: empty.
    pub text: String,
    /// Only while `downloading`.
    pub progress: Option<InstallProgress>,
}

/// What the Settings window renders.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppSettingsView {
    /// The running version.
    pub version: String,
    /// "Automatically check for updates".
    pub check_updates: bool,
    pub update: UpdateView,
    pub install: InstallView,
}

/// Where an install is, inside the updater.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallPhase {
    /// None runs (and none failed since the last check).
    Idle,
    Downloading {
        bytes: u64,
        total: Option<u64>,
    },
    Verifying,
    Restarting,
    Failed {
        reason: String,
    },
}

/// The check part's status line for an outcome.
pub fn update_text(
    product: &Product,
    running: Version,
    outcome: &Outcome,
) -> (UpdateState, String) {
    let name = product.name;
    let cur = running;
    match outcome {
        Outcome::Idle => (UpdateState::Idle, "Not checked yet.".to_string()),
        Outcome::Checking => (
            UpdateState::Checking,
            "Checking for updates\u{2026}".to_string(),
        ),
        Outcome::UpToDate { .. } => (
            UpdateState::UpToDate,
            format!("You have the latest version ({cur})."),
        ),
        Outcome::Available { newer } => (
            UpdateState::Available,
            format!("{name} {newer} is available. You have {cur}."),
        ),
        Outcome::NoRelease => (
            UpdateState::NoRelease,
            format!("No {name} release is published yet."),
        ),
        Outcome::Failed { reason } => (
            UpdateState::Failed,
            format!("Could not check for updates: {reason}."),
        ),
    }
}

/// The text of an unsigned newer release (no button with it).
pub fn unsigned_text(newer: Version) -> String {
    format!("{newer} is available but cannot be installed: the release is not signed.")
}

/// The install part for the newer release known and the install's phase.
pub fn install_view(
    product: &Product,
    newer: Option<&Release>,
    phase: &InstallPhase,
) -> InstallView {
    let name = product.name;
    let view = |state, text: String| InstallView {
        state,
        text,
        progress: None,
    };
    let Some(release) = newer else {
        // A phase without a newer release cannot happen: an install starts
        // only with one, and a check that drops it is refused meanwhile.
        return view(InstallState::None, String::new());
    };
    let v = release.version;
    match phase {
        InstallPhase::Idle if !release.is_signed() => {
            view(InstallState::Unsigned, unsigned_text(v))
        }
        InstallPhase::Idle => view(InstallState::Ready, format!("Install {v} and restart")),
        InstallPhase::Downloading { bytes, total } => InstallView {
            state: InstallState::Downloading,
            text: format!("Downloading {name} {v}\u{2026}"),
            progress: Some(InstallProgress {
                bytes: *bytes,
                total: *total,
            }),
        },
        InstallPhase::Verifying => view(
            InstallState::Verifying,
            format!("Checking the signature of {name} {v}\u{2026}"),
        ),
        InstallPhase::Restarting => view(
            InstallState::Restarting,
            format!("{name} {v} is installed. Restarting\u{2026}"),
        ),
        InstallPhase::Failed { reason } => view(
            InstallState::Failed,
            format!("Could not install {name} {v}: {reason}."),
        ),
    }
}

/// The whole view.
pub fn app_settings_view(
    product: &Product,
    running: Version,
    check_updates: bool,
    outcome: &Outcome,
    newer: Option<&Release>,
    phase: &InstallPhase,
) -> AppSettingsView {
    let (state, text) = update_text(product, running, outcome);
    AppSettingsView {
        version: running.to_string(),
        check_updates,
        update: UpdateView {
            state,
            text,
            newer: newer.map(|r| r.version.to_string()),
        },
        install: install_view(product, newer, phase),
    }
}

// -- diagnostic lines ---------------------------------------------------------

/// The setting was turned on or off.
pub fn turned_line(on: bool) -> &'static str {
    if on {
        "updates: automatic checks turned on"
    } else {
        "updates: automatic checks turned off"
    }
}

/// A check started.
pub fn started_line(manual: bool) -> &'static str {
    if manual {
        "updates: check started (manual)"
    } else {
        "updates: check started (automatic)"
    }
}

/// How a check ended. Never anything of the answer's body.
pub fn outcome_line(outcome: &Outcome, running: Version) -> String {
    match outcome {
        Outcome::UpToDate { latest } => {
            format!("updates: up to date (running {running}, latest {latest})")
        }
        Outcome::Available { newer } => format!("updates: {newer} available (running {running})"),
        Outcome::NoRelease => "updates: no release published".to_string(),
        Outcome::Failed { reason } => format!("updates: check failed: {reason}"),
        // A check never ends idle or checking.
        Outcome::Idle | Outcome::Checking => "updates: check ended".to_string(),
    }
}

/// An install started.
pub fn install_started_line(newer: Version) -> String {
    format!("updates: install of {newer} started")
}

/// An install was asked for and did not start.
pub fn install_refused_line(reason: &str) -> String {
    format!("updates: install refused: {reason}")
}

/// The downloaded exe's signature checked out.
pub fn signature_checked_line(newer: Version) -> String {
    format!("updates: signature of {newer} checked")
}

/// The new exe is in place and the app restarts into it.
pub fn replaced_line(newer: Version) -> String {
    format!("updates: replaced the running exe with {newer}, restarting")
}

/// An install failed.
pub fn install_failed_line(reason: &str) -> String {
    format!("updates: install failed: {reason}")
}

/// An install was cancelled by the user.
pub const INSTALL_CANCELLED_LINE: &str = "updates: install cancelled";
