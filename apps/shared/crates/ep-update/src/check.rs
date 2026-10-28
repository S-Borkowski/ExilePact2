//! The transport an app hands the updater, and what a release list means.
//!
//! What a check reads: GitHub's release list of the repository. The newest
//! release is the highest non-draft, non-prerelease entry whose tag is the
//! product's prefix and exactly `<major>.<minor>.<patch>`; anything else is
//! skipped, never an error ([`newest_release`]). Of that one release the
//! parse also keeps the exe asset and its `.minisig` (each by its exact
//! name, with its size), which is all an install needs to know. Nothing of
//! the answer's body is ever logged or shown, and no URL is taken from it:
//! the transport builds every download's address from the version and the
//! asset's kind.

use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use serde_json::Value;

use crate::version::{Product, Version};

/// The reason a build without a transport gives.
pub const NO_TRANSPORT: &str = "this build cannot check for updates";

/// The largest exe an install downloads; a release listing a larger one
/// fails before any request. `ep-net`'s cap for the exe asset.
pub const MAX_EXE: u64 = 64 * 1024 * 1024;

/// The largest signature an install reads. `ep-net`'s cap for the
/// signature asset; a minisign signature is about 300 bytes.
pub const MAX_SIGNATURE: u64 = 4 * 1024;

/// The server's answer to the release list: its HTTP status and body,
/// whatever the status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    pub status: u16,
    pub body: Vec<u8>,
}

/// Why a request produced no answer (`ep-net`'s `FetchError`, which the glue
/// maps one to one).
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

/// Which of a release's two assets a download names. The transport builds
/// the address from the product it was made for, the version and this; no
/// caller names a host, a path or a URL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssetKind {
    /// The product's exe.
    Exe,
    /// The exe's minisign signature, `<exe>.minisig`.
    Signature,
}

/// Why a download did not complete (`ep-net`'s `DownloadError`, which the
/// glue maps one to one). Nothing in it names a URL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadError {
    /// No answer, or one over the cap.
    Transport(TransportError),
    /// The final answer's status was not 200; nothing was written.
    Status(u16),
    /// The redirect led somewhere other than an allowed asset host.
    RedirectRefused,
    /// The redirect's target redirected again.
    TooManyRedirects,
    /// The connection ended before the length the server announced.
    Incomplete,
    /// The cancel flag was set.
    Cancelled,
    /// The writer failed.
    Write(std::io::ErrorKind),
}

/// The requests an updater makes, implemented by each app's glue over
/// `ep-net` for its own product. Every call blocks the calling thread, which
/// is always a check's or an install's own.
pub trait UpdateTransport: Send + Sync + 'static {
    /// The repository's release list.
    fn fetch_releases(&self) -> Result<Answer, TransportError>;

    /// Streams one asset of the product's release `version` into `sink`:
    /// at most `limit` bytes, `progress` hearing `(bytes so far, announced
    /// length)` after every read, `cancel` checked between reads. Returns how
    /// many bytes it wrote. A transport that cannot download keeps this
    /// default, and an install through it fails without a request.
    fn download(
        &self,
        version: Version,
        asset: AssetKind,
        limit: u64,
        sink: &mut dyn Write,
        progress: &mut dyn FnMut(u64, Option<u64>),
        cancel: &AtomicBool,
    ) -> Result<u64, DownloadError> {
        let _ = (version, asset, limit, sink, progress, cancel);
        Err(DownloadError::Transport(TransportError::Unsupported))
    }
}

/// What an app hands [`crate::Updater::new`].
#[derive(Default)]
pub struct UpdateConfig {
    /// `None`: this build cannot check, and a check ends failed with
    /// [`NO_TRANSPORT`] without a request.
    pub transport: Option<Box<dyn UpdateTransport>>,
    /// The running version a release is compared with; `None` is the
    /// version the app passes [`crate::Updater::new`] (tests give their own,
    /// so they do not depend on the workspace's version).
    pub running: Option<Version>,
    /// The running exe, captured by the glue at start
    /// (`std::env::current_exe()`) and never read again after a swap.
    /// `None`: this build cannot install, and an install is refused.
    pub exe_path: Option<PathBuf>,
    /// The minisign public key (its `RW...` line) a downloaded update must
    /// be signed with. `None`, the glue's only choice, is
    /// [`crate::release_key::RELEASE_PUBLIC_KEY`]; tests inject the fixture
    /// key here.
    pub public_key: Option<String>,
}

/// One asset of a release, as the release list names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseAsset {
    pub name: String,
    pub size: u64,
}

/// The newest release of a product, with its exe and signature assets when
/// it lists them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub version: Version,
    /// The asset named exactly the product's exe.
    pub exe: Option<ReleaseAsset>,
    /// The asset named exactly the exe and `.minisig`.
    pub signature: Option<ReleaseAsset>,
}

impl Release {
    /// Both assets an install needs are listed: the release is signed.
    pub fn is_signed(&self) -> bool {
        self.exe.is_some() && self.signature.is_some()
    }
}

/// A release list body that is not a JSON array of releases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unreadable;

/// The newest published release of `product` in a release list body, or
/// `None` when it lists none. An entry that is a draft, a prerelease,
/// another tool's or a tag that is not exactly the prefix and a version is
/// skipped; only a body that is not a JSON array is refused. An asset counts
/// only with its exact name, a whole-number size and, when the list says,
/// the state `uploaded` (GitHub lists an upload in progress as `open`).
pub fn newest_release(product: &Product, body: &[u8]) -> Result<Option<Release>, Unreadable> {
    let Ok(Value::Array(entries)) = serde_json::from_slice::<Value>(body) else {
        return Err(Unreadable);
    };
    let newest = entries
        .iter()
        .filter(|e| {
            e.get("draft").and_then(Value::as_bool) == Some(false)
                && e.get("prerelease").and_then(Value::as_bool) == Some(false)
        })
        .filter_map(|e| Some((product.version_of_tag(e.get("tag_name")?.as_str()?)?, e)))
        .max_by_key(|(v, _)| *v);
    Ok(newest.map(|(version, entry)| Release {
        version,
        exe: asset_named(entry, product.exe_name),
        signature: asset_named(entry, &product.signature_name()),
    }))
}

fn asset_named(entry: &Value, name: &str) -> Option<ReleaseAsset> {
    let assets = entry.get("assets")?.as_array()?;
    assets.iter().find_map(|a| {
        if a.get("name")?.as_str()? != name {
            return None;
        }
        match a.get("state") {
            None => {}
            Some(s) if s.as_str() == Some("uploaded") => {}
            Some(_) => return None,
        }
        Some(ReleaseAsset {
            name: name.to_string(),
            size: a.get("size")?.as_u64()?,
        })
    })
}

/// One line saying why a request failed (the window's status line and the
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

/// One line saying why a download failed.
pub fn download_reason(e: DownloadError) -> String {
    match e {
        DownloadError::Transport(TransportError::Unsupported) => {
            "this build cannot download updates".to_string()
        }
        DownloadError::Transport(TransportError::TooLarge) => {
            "the download was larger than the release lists".to_string()
        }
        DownloadError::Transport(t) => transport_reason(t),
        DownloadError::Status(code) => format!("GitHub answered {code}"),
        DownloadError::RedirectRefused => {
            "the download was redirected to an unexpected place".to_string()
        }
        DownloadError::TooManyRedirects => "the download was redirected twice".to_string(),
        DownloadError::Incomplete => "the download ended early".to_string(),
        DownloadError::Cancelled => "the download was cancelled".to_string(),
        DownloadError::Write(kind) => format!("the download could not be saved ({kind})"),
    }
}

/// The last check's outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// No check yet this run.
    Idle,
    Checking,
    /// The newest release is `latest`, not newer than the running build.
    UpToDate {
        latest: Version,
    },
    Available {
        newer: Version,
    },
    NoRelease,
    Failed {
        reason: String,
    },
}

/// What one answer means against the `running` version: the outcome, and
/// the newer release itself when there is one.
pub fn read_answer(
    product: &Product,
    answer: Result<Answer, TransportError>,
    running: Version,
) -> (Outcome, Option<Release>) {
    let failed = |reason: String| (Outcome::Failed { reason }, None);
    match answer {
        Err(e) => failed(transport_reason(e)),
        Ok(Answer {
            status: 403 | 429, ..
        }) => failed("GitHub refused the check for now (rate limit), try again later".to_string()),
        Ok(Answer { status, .. }) if status != 200 => failed(format!("GitHub answered {status}")),
        Ok(Answer { body, .. }) => match newest_release(product, &body) {
            Err(Unreadable) => failed("GitHub's answer could not be read".to_string()),
            Ok(None) => (Outcome::NoRelease, None),
            Ok(Some(r)) if r.version > running => {
                (Outcome::Available { newer: r.version }, Some(r))
            }
            Ok(Some(r)) => (Outcome::UpToDate { latest: r.version }, None),
        },
    }
}

/// [`read_answer`]'s outcome alone.
pub fn outcome_of(
    product: &Product,
    answer: Result<Answer, TransportError>,
    running: Version,
) -> Outcome {
    read_answer(product, answer, running).0
}
