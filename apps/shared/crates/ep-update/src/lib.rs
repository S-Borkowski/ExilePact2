//! Both apps' update logic, kept once: whether a newer release of the app
//! is published, and installing it in the app when the user presses the
//! button.
//!
//! Nothing here talks to the network, and nothing here is `unsafe`. The
//! requests are an [`UpdateTransport`]'s two methods, which each app's glue
//! implements over `ep-net` for its own [`Product`] and hands
//! [`Updater::new`] in an [`UpdateConfig`]; this crate decides when they are
//! called and what their answers mean, and calls them only on a thread of
//! its own. The product (its display name, tag prefix and exe asset name) is
//! data ([`QUEST_TRACKER`], [`WAYSTONE_ROLLER`]); nothing here depends on
//! either app.
//!
//! - [`check`]: the transport, the release-list parse (the newest published
//!   release of the product, with its exe and `.minisig` assets) and what a
//!   check's answer means.
//! - [`updater`]: the check's state machine and the install, single-flight
//!   together, behind one lock of their own ([`Updater`]).
//! - [`signature`]: the minisign check of a download against the key built
//!   into the app ([`release_key::RELEASE_PUBLIC_KEY`]; tests inject theirs
//!   through [`UpdateConfig::public_key`]).
//! - [`swap`]: replacing the running exe by renames, and the start-up cleanup
//!   of what that leaves.
//! - [`view`]: the Settings window's view, its texts and the `updates:`
//!   diagnostic lines.
//!
//! There is no fallback to a release page or a browser anywhere: a release
//! that is not signed is shown as such with no button, and a failed install
//! is its reason and *Try again*, with the running exe untouched.

#![forbid(unsafe_code)]

pub mod check;
pub mod release_key;
pub mod signature;
pub mod swap;
pub mod updater;
pub mod version;
pub mod view;

pub use check::{
    download_reason, newest_release, outcome_of, read_answer, transport_reason, Answer, AssetKind,
    DownloadError, Outcome, Release, ReleaseAsset, TransportError, Unreadable, UpdateConfig,
    UpdateTransport, MAX_EXE, MAX_SIGNATURE, NO_TRANSPORT,
};
pub use release_key::RELEASE_PUBLIC_KEY;
pub use signature::SignatureError;
pub use swap::{cleanup_line, old_path, spawn_cleanup, staging_path, swap as swap_exe};
pub use updater::{
    Begin, InstallEvents, InstallRefused, Updater, CHECK_INTERVAL_MS, PROGRESS_EVERY,
    RETRY_AFTER_FAILURE_MS,
};
pub use version::{Product, Version, QUEST_TRACKER, SIGNATURE_SUFFIX, WAYSTONE_ROLLER};
pub use view::{
    app_settings_view, install_view, outcome_line, started_line, turned_line, unsigned_text,
    AppSettingsView, InstallPhase, InstallProgress, InstallState, InstallView, UpdateState,
    UpdateView,
};
