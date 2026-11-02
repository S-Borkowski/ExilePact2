//! What the waystone roller's update check and install are built from
//! ([`UpdateSetup`]), the clock they read ([`Clock`]), the Settings window's
//! view ([`AppSettingsView`]) and the `check_updates` and `theme` patch the
//! window sends ([`settings_patch`]). The logic itself is `ep-update`'s
//! [`Updater`](ep_update::Updater), shared with the quest tracker; the app's
//! commands that drive it are on [`crate::App`].
//!
//! When a check runs: from the `check_for_updates` command, and, with the
//! `check_updates` setting on, from [`crate::App::tick_updates`]: once at
//! [`crate::App::start`], again whenever the setting is turned on, and from
//! the app's ticker thread every [`TICK_EVERY`], which starts one when
//! `ep-update` says one is due (a day after the last, an hour after a failed
//! automatic one). An install runs only from the `install_update` command.
//! With the setting off a tick reads one bool and does nothing else.

use std::time::{Duration, Instant};

use ep_update::{InstallView, Product, UpdateConfig, UpdateView, WAYSTONE_ROLLER};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::view::Theme;

/// How often the ticker thread asks whether an automatic check is due.
pub const TICK_EVERY: Duration = Duration::from_secs(60);

/// The update key `update_settings` takes.
pub const CHECK_UPDATES_KEY: &str = "check_updates";

/// The app's clock for the update check, in milliseconds.
pub trait Clock: Send + Sync {
    fn now_ms(&self) -> u64;
}

/// Milliseconds since the clock was made, from the monotonic clock: a
/// change of the system's time moves no check.
pub struct SteadyClock(Instant);

impl SteadyClock {
    pub fn new() -> SteadyClock {
        SteadyClock(Instant::now())
    }
}

impl Default for SteadyClock {
    fn default() -> SteadyClock {
        SteadyClock::new()
    }
}

impl Clock for SteadyClock {
    fn now_ms(&self) -> u64 {
        u64::try_from(self.0.elapsed().as_millis()).unwrap_or(u64::MAX)
    }
}

/// What [`crate::App::with_updates`] builds the update check from.
pub struct UpdateSetup {
    /// The transport, the exe and (in tests only) the running version and
    /// the key. The default has no transport: a check ends failed without a
    /// request, and an install is refused.
    pub config: UpdateConfig,
    /// The product the releases are read for: the waystone roller. Tests
    /// give one whose exe asset is the shared signature fixture.
    pub product: Product,
    pub clock: Box<dyn Clock>,
    /// How often the ticker thread ticks; `None` starts no ticker (tests
    /// call [`crate::App::tick_updates`] with times of their own).
    pub tick_every: Option<Duration>,
}

impl Default for UpdateSetup {
    fn default() -> UpdateSetup {
        UpdateSetup {
            config: UpdateConfig::default(),
            product: WAYSTONE_ROLLER,
            clock: Box::new(SteadyClock::new()),
            tick_every: Some(TICK_EVERY),
        }
    }
}

/// The other key `update_settings` takes: the look both windows draw.
pub const THEME_KEY: &str = "theme";

/// The Settings window's view: `ep-update`'s, which the Quest Tracker's
/// window shares, with the theme beside it (as the Quest Tracker's own
/// Settings view carries its theme), so the window draws the chosen look.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppSettingsView {
    pub version: String,
    /// "Automatically check for updates".
    pub check_updates: bool,
    pub update: UpdateView,
    pub install: InstallView,
    /// The look the window draws (`settings.json`'s `theme`).
    pub theme: Theme,
}

impl AppSettingsView {
    /// `ep-update`'s view with the theme beside it.
    pub fn with_theme(view: ep_update::AppSettingsView, theme: Theme) -> AppSettingsView {
        AppSettingsView {
            version: view.version,
            check_updates: view.check_updates,
            update: view.update,
            install: view.install,
            theme,
        }
    }
}

/// What one `update_settings` patch changes: either key, or both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SettingsPatch {
    pub check_updates: Option<bool>,
    pub theme: Option<Theme>,
}

/// The value of an `update_settings` patch: an object with
/// `"check_updates": <bool>`, `"theme": "exile" | "claude"`, or both.
/// Anything else (another key, another type, another theme, no key at all)
/// is refused whole, with the reason, so nothing of it is applied.
pub fn settings_patch(patch: &Value) -> Result<SettingsPatch, String> {
    let patch = patch
        .as_object()
        .ok_or_else(|| "the change is not an object".to_string())?;
    if let Some(k) = patch
        .keys()
        .find(|k| k.as_str() != CHECK_UPDATES_KEY && k.as_str() != THEME_KEY)
    {
        return Err(format!("{k:?} is not a setting of this window"));
    }
    let check_updates = match patch.get(CHECK_UPDATES_KEY) {
        Some(Value::Bool(on)) => Some(*on),
        Some(_) => return Err(format!("{CHECK_UPDATES_KEY} is not true or false")),
        None => None,
    };
    let theme = match patch.get(THEME_KEY) {
        Some(Value::String(s)) if s == "exile" => Some(Theme::Exile),
        Some(Value::String(s)) if s == "claude" => Some(Theme::Claude),
        Some(_) => return Err(format!("{THEME_KEY} is not \"exile\" or \"claude\"")),
        None => None,
    };
    if check_updates.is_none() && theme.is_none() {
        return Err(format!(
            "the change has no {CHECK_UPDATES_KEY} or {THEME_KEY}"
        ));
    }
    Ok(SettingsPatch {
        check_updates,
        theme,
    })
}
