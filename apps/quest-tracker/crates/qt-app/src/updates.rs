//! The update check: whether a newer Quest Tracker release is published on
//! GitHub, asked only when the user orders it (Context "The update checker
//! in qt-app", "The privacy invariant and its proof").
//!
//! Nothing here talks to the network. The request is an
//! [`UpdateTransport`]'s one method, which the glue implements over
//! `qt-net` and hands [`crate::App::with_updates`] in an [`UpdateConfig`];
//! this module decides when it is called and what the answer means, and
//! calls it at exactly one place, on a thread of its own
//! ([`Checker::spawn`]). It is reached from two places only: the manual
//! request ([`crate::App::check_for_updates`], the tray's *Check for
//! updates* and the window's *Check now*) and the worker's tick while the
//! `check_updates` setting is on and a check is due ([`Checker::begin_auto`]).
//! With the setting off, which is the default and what a file written before
//! it existed loads, no tick ever reaches it.
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
//! skipped, never an error ([`newest_release`]). The release page opened is
//! built from that tag ([`Version::release_page`]), never taken from the
//! answer. Nothing of the answer's body is ever logged or shown.

use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The tag prefix of a Quest Tracker release (`tools/quest_tracker/release.py`'s
/// `TAG_PREFIX`); the repository's other tools tag their own.
pub const TAG_PREFIX: &str = "quest-tracker-v";

/// Where a release's page is: this, then its tag. `release.py`'s `REPO_URL`
/// followed by `/releases/tag/`.
pub const RELEASE_PAGE_PREFIX: &str = "https://github.com/S-Borkowski/ExilePact2/releases/tag/";

/// How long after a check started the next automatic one is due.
pub const CHECK_INTERVAL_MS: u64 = 24 * 60 * 60 * 1_000;

/// How long after a failed automatic check the next one is due.
pub const RETRY_AFTER_FAILURE_MS: u64 = 60 * 60 * 1_000;

/// The reason a build without a transport gives.
pub const NO_TRANSPORT: &str = "this build cannot check for updates";

/// The tray icon's tooltip with no newer version known.
pub const TRAY_TOOLTIP: &str = "Quest Tracker";

/// The tray menu's entry ids (see [`tray_menu`]).
pub const TRAY_UPDATE_AVAILABLE: &str = "update-available";
pub const TRAY_CHECK_UPDATES: &str = "check-updates";
pub const TRAY_SETTINGS: &str = "settings";
pub const TRAY_QUIT: &str = "quit";

/// A release's `<major>.<minor>.<patch>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl Version {
    pub const fn new(major: u64, minor: u64, patch: u64) -> Version {
        Version {
            major,
            minor,
            patch,
        }
    }

    /// Exactly three decimal numbers joined by dots, nothing before or after
    /// (no sign, no space, no `-rc1`); `None` for anything else.
    pub fn parse(text: &str) -> Option<Version> {
        let mut parts = text.split('.');
        let mut next = || -> Option<u64> {
            let p = parts.next()?;
            if p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            p.parse().ok()
        };
        let v = Version::new(next()?, next()?, next()?);
        parts.next().is_none().then_some(v)
    }

    /// The version a release tag names: [`TAG_PREFIX`] then [`Version::parse`].
    pub fn from_tag(tag: &str) -> Option<Version> {
        Version::parse(tag.strip_prefix(TAG_PREFIX)?)
    }

    /// This build's version: the crate's, read up to any `-` or `+` suffix
    /// (0.0.0 should it ever not parse).
    pub fn running() -> Version {
        let own = env!("CARGO_PKG_VERSION");
        let core = own.split(['-', '+']).next().unwrap_or(own);
        Version::parse(core).unwrap_or(Version::new(0, 0, 0))
    }

    /// The release's tag, `quest-tracker-v<version>`.
    pub fn tag(&self) -> String {
        format!("{TAG_PREFIX}{self}")
    }

    /// The release's page on GitHub, built from its tag.
    pub fn release_page(&self) -> String {
        format!("{RELEASE_PAGE_PREFIX}{}", self.tag())
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// The server's answer: its HTTP status and body, whatever the status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    pub status: u16,
    pub body: Vec<u8>,
}

/// Why a request produced no answer (`qt-net`'s errors, which the glue maps
/// one to one).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportError {
    /// This build has no way to make the request; nothing was tried.
    Unsupported,
    Timeout,
    Unreachable,
    NotSecure,
    TooLarge,
    /// The request stopped without an answer (its thread panicked).
    Panicked,
    /// Any other failure, with the system's error code.
    Other(u32),
}

/// The one request a check makes: the repository's release list. The URL
/// is the transport's own; nothing here can name another. Blocks the
/// calling thread, which is always the check's own ([`Checker::spawn`]).
pub trait UpdateTransport: Send + Sync + 'static {
    fn fetch_releases(&self) -> Result<Answer, TransportError>;
}

/// What [`crate::App::with_updates`] needs for the update check.
#[derive(Default)]
pub struct UpdateConfig {
    /// `None`: this build cannot check, and a check ends failed with
    /// [`NO_TRANSPORT`] without a request.
    pub transport: Option<Box<dyn UpdateTransport>>,
    /// The running version a release is compared with; `None` is
    /// [`Version::running`] (tests give their own, so they do not depend on
    /// the workspace's version).
    pub running: Option<Version>,
}

/// The newest published Quest Tracker release in a release list body, or
/// `None` when it lists none. An entry that is a draft, a prerelease,
/// another tool's or a tag that is not exactly [`TAG_PREFIX`] and a version
/// is skipped; only a body that is not a JSON array is refused.
pub fn newest_release(body: &[u8]) -> Result<Option<Version>, Unreadable> {
    let Ok(Value::Array(entries)) = serde_json::from_slice::<Value>(body) else {
        return Err(Unreadable);
    };
    Ok(entries
        .iter()
        .filter(|e| {
            e.get("draft").and_then(Value::as_bool) == Some(false)
                && e.get("prerelease").and_then(Value::as_bool) == Some(false)
        })
        .filter_map(|e| Version::from_tag(e.get("tag_name")?.as_str()?))
        .max())
}

/// A release list body that is not a JSON array of releases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unreadable;

/// One line saying why a check failed (the window's status line and the
/// diagnostic log carry it).
pub fn transport_reason(e: TransportError) -> String {
    match e {
        TransportError::Unsupported => NO_TRANSPORT.to_string(),
        TransportError::Timeout => "GitHub did not answer in time".to_string(),
        TransportError::Unreachable => "GitHub could not be reached".to_string(),
        TransportError::NotSecure => "the connection to GitHub was not secure".to_string(),
        TransportError::TooLarge => "GitHub's answer was too large".to_string(),
        TransportError::Panicked => "the request stopped unexpectedly".to_string(),
        TransportError::Other(code) => format!("the request failed (error {code})"),
    }
}

/// The last check's outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// No check yet this run.
    Idle,
    Checking,
    /// The newest release is `latest`, not newer than the running build.
    UpToDate { latest: Version },
    Available { newer: Version },
    NoRelease,
    Failed { reason: String },
}

/// What one answer means against the `running` version.
pub fn outcome_of(answer: Result<Answer, TransportError>, running: Version) -> Outcome {
    let failed = |reason: String| Outcome::Failed { reason };
    match answer {
        Err(e) => failed(transport_reason(e)),
        Ok(Answer { status: 403 | 429, .. }) => failed(
            "GitHub refused the check for now (rate limit), try again later".to_string(),
        ),
        Ok(Answer { status, .. }) if status != 200 => failed(format!("GitHub answered {status}")),
        Ok(Answer { body, .. }) => match newest_release(&body) {
            Err(Unreadable) => failed("GitHub's answer could not be read".to_string()),
            Ok(None) => Outcome::NoRelease,
            Ok(Some(v)) if v > running => Outcome::Available { newer: v },
            Ok(Some(v)) => Outcome::UpToDate { latest: v },
        },
    }
}

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

/// The update part of [`AppSettingsView`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateView {
    pub state: UpdateState,
    /// The status line, in words.
    pub text: String,
    /// The newest version known newer than the running one from any
    /// successful check this run: what the window's *Open the release page*
    /// and the tray's *Update available* follow.
    pub newer: Option<String>,
}

/// What the Settings window renders, sent as `qt:app-settings` and answered
/// by `get_app_settings`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppSettingsView {
    /// The running version.
    pub version: String,
    /// "Automatically check for updates".
    pub check_updates: bool,
    pub update: UpdateView,
}

/// The view for an outcome, the newer version known and the setting.
pub fn app_settings_view(
    running: Version,
    check_updates: bool,
    outcome: &Outcome,
    newer: Option<Version>,
) -> AppSettingsView {
    let cur = running;
    let (state, text) = match outcome {
        Outcome::Idle => (UpdateState::Idle, "Not checked yet.".to_string()),
        Outcome::Checking => (UpdateState::Checking, "Checking for updates\u{2026}".to_string()),
        Outcome::UpToDate { .. } => (
            UpdateState::UpToDate,
            format!("You have the latest version ({cur})."),
        ),
        Outcome::Available { newer } => (
            UpdateState::Available,
            format!("Quest Tracker {newer} is available. You have {cur}."),
        ),
        Outcome::NoRelease => (
            UpdateState::NoRelease,
            "No Quest Tracker release is published yet.".to_string(),
        ),
        Outcome::Failed { reason } => (
            UpdateState::Failed,
            format!("Could not check for updates: {reason}."),
        ),
    };
    AppSettingsView {
        version: running.to_string(),
        check_updates,
        update: UpdateView {
            state,
            text,
            newer: newer.map(|v| v.to_string()),
        },
    }
}

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

/// What a tray entry does, by its id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayAction {
    /// Opens the newer version's release page in the default browser.
    OpenReleasePage,
    /// Opens or focuses the Settings window and starts a manual check.
    CheckForUpdates,
    /// Opens or focuses the Settings window.
    OpenSettings,
    Quit,
}

impl TrayAction {
    pub fn from_id(id: &str) -> Option<TrayAction> {
        match id {
            TRAY_UPDATE_AVAILABLE => Some(TrayAction::OpenReleasePage),
            TRAY_CHECK_UPDATES => Some(TrayAction::CheckForUpdates),
            TRAY_SETTINGS => Some(TrayAction::OpenSettings),
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

/// The release page of `newer` was opened.
pub fn opened_line(newer: Version) -> String {
    format!("updates: opened the release page of {newer}")
}

// -- the checker --------------------------------------------------------------

/// What [`Checker::begin_manual`] and [`Checker::begin_auto`] decided.
pub(crate) enum Begin {
    /// No check starts: one is running, none is due, or the app is stopping.
    Skip,
    /// A check started; `None` when this build has no transport.
    Run(Option<Arc<dyn UpdateTransport>>),
}

struct Core {
    outcome: Outcome,
    /// See [`UpdateView::newer`].
    newer: Option<Version>,
    /// A check's thread is between its start and its end.
    in_flight: bool,
    /// When the last check, manual or automatic, started.
    last_start_ms: Option<u64>,
    /// When the last automatic check failed, while no check started since.
    auto_failed_ms: Option<u64>,
    /// Raised by every change of what [`AppSettingsView`] shows, so an
    /// emitter can tell whether the view it sent is still the newest.
    seq: u64,
}

/// The update check's state, behind its own lock: never the app's state
/// lock, and never held across the request.
pub(crate) struct Checker {
    transport: Option<Arc<dyn UpdateTransport>>,
    running: Version,
    core: Mutex<Core>,
    /// Set when the app starts shutting down: no check starts, and one that
    /// ends after it changes nothing.
    stopped: AtomicBool,
}

impl Checker {
    pub(crate) fn new(config: UpdateConfig) -> Checker {
        Checker {
            transport: config.transport.map(Arc::from),
            running: config.running.unwrap_or_else(Version::running),
            core: Mutex::new(Core {
                outcome: Outcome::Idle,
                newer: None,
                in_flight: false,
                last_start_ms: None,
                auto_failed_ms: None,
                seq: 0,
            }),
            stopped: AtomicBool::new(false),
        }
    }

    fn lock(&self) -> MutexGuard<'_, Core> {
        self.core.lock().unwrap_or_else(|p| p.into_inner())
    }

    pub(crate) fn running(&self) -> Version {
        self.running
    }

    pub(crate) fn view(&self, check_updates: bool) -> AppSettingsView {
        let core = self.lock();
        app_settings_view(self.running, check_updates, &core.outcome, core.newer)
    }

    pub(crate) fn seq(&self) -> u64 {
        self.lock().seq
    }

    /// Something the view shows changed outside the checker (the setting).
    pub(crate) fn changed(&self) {
        self.lock().seq += 1;
    }

    pub(crate) fn newer(&self) -> Option<Version> {
        self.lock().newer
    }

    pub(crate) fn stop(&self) {
        self.stopped.store(true, Ordering::Release);
    }

    pub(crate) fn is_stopped(&self) -> bool {
        self.stopped.load(Ordering::Acquire)
    }

    /// A manual request: starts a check unless one is running.
    pub(crate) fn begin_manual(&self, now_ms: u64) -> Begin {
        let mut core = self.lock();
        self.begin(&mut core, now_ms)
    }

    /// The worker's tick, with the setting `on`: starts a check when one is
    /// due and none is running.
    pub(crate) fn begin_auto(&self, now_ms: u64, on: bool) -> Begin {
        let mut core = self.lock();
        let since = |t: Option<u64>| t.map(|t| now_ms.saturating_sub(t));
        let due = core.last_start_ms.is_none()
            || since(core.last_start_ms).is_some_and(|d| d >= CHECK_INTERVAL_MS)
            || since(core.auto_failed_ms).is_some_and(|d| d >= RETRY_AFTER_FAILURE_MS);
        if !on || !due {
            return Begin::Skip;
        }
        self.begin(&mut core, now_ms)
    }

    fn begin(&self, core: &mut Core, now_ms: u64) -> Begin {
        if core.in_flight || self.is_stopped() {
            return Begin::Skip;
        }
        core.in_flight = true;
        core.last_start_ms = Some(now_ms);
        core.auto_failed_ms = None;
        core.outcome = Outcome::Checking;
        core.seq += 1;
        Begin::Run(self.transport.clone())
    }

    /// A check ended with `answer`; the outcome, or `None` when the app
    /// began shutting down first (nothing changes then).
    pub(crate) fn finish(
        &self,
        now_ms: u64,
        manual: bool,
        answer: Result<Answer, TransportError>,
    ) -> Option<Outcome> {
        let mut core = self.lock();
        if self.is_stopped() {
            return None;
        }
        let outcome = outcome_of(answer, self.running);
        match &outcome {
            Outcome::Available { newer } => core.newer = Some(*newer),
            Outcome::UpToDate { .. } | Outcome::NoRelease => core.newer = None,
            Outcome::Failed { .. } if !manual => core.auto_failed_ms = Some(now_ms),
            _ => {}
        }
        core.in_flight = false;
        core.outcome = outcome.clone();
        core.seq += 1;
        Some(outcome)
    }

    /// The one call of the transport: its request on a thread of its own,
    /// whose answer goes to `done`. The caller never waits for it.
    pub(crate) fn spawn(
        transport: Arc<dyn UpdateTransport>,
        done: impl FnOnce(Result<Answer, TransportError>) + Send + 'static,
    ) -> std::io::Result<()> {
        thread::Builder::new()
            .name("qt-updates".to_string())
            .spawn(move || done(transport.fetch_releases()))
            .map(|_| ())
    }
}
