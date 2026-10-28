//! What the crafting helper's update check and install are built from
//! ([`UpdateSetup`]) and the clock they read ([`Clock`]). The logic itself is
//! `ep-update`'s [`Updater`](ep_update::Updater), shared with the quest
//! tracker and the waystone roller; the app's commands that drive it are on
//! [`crate::App`], and the `check_updates` patch the Settings window sends is
//! read by [`crate::settings`].
//!
//! When a check runs: from the `check_for_updates` command, and, with the
//! `check_updates` setting on, from [`crate::App::tick_updates`]: once at
//! [`crate::App::start`], again whenever the setting is turned on, and from
//! the app's ticker thread every [`TICK_EVERY`], which starts one when
//! `ep-update` says one is due (a day after the last, an hour after a failed
//! automatic one). An install runs only from the `install_update` command.
//! With the setting off a tick reads one bool and does nothing else.

use std::time::{Duration, Instant};

use ep_update::{Product, UpdateConfig, CRAFTING_HELPER};

/// How often the ticker thread asks whether an automatic check is due.
pub const TICK_EVERY: Duration = Duration::from_secs(60);

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
    /// The product the releases are read for: the crafting helper. Tests
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
            product: CRAFTING_HELPER,
            clock: Box::new(SteadyClock::new()),
            tick_every: Some(TICK_EVERY),
        }
    }
}
