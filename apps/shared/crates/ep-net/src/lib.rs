//! The apps' only network code: the repository's GitHub release list and
//! a release's fixed assets, and the trade site's league list for the
//! crafting helper, fetched over HTTPS through Windows' own WinHTTP.
//!
//! No other crate of any app talks to the network. The public API reaches
//! three places only, [`fetch_release_list`], [`download_asset`] and
//! [`fetch_trade_leagues`]; the hosts, ports, paths and headers are this
//! crate's constants, and no caller can pass a host, a path or a URL. A
//! caller names its [`Product`], and for a download a [`ReleaseTag`] (the
//! product's tag prefix and three numbers) and an [`Asset`] (the product's
//! exe or its `.minisig`); the league list takes nothing. The update code in
//! `ep-update`, and for the league list the crafting helper's `ch-app`,
//! decides when a request happens and what the answer means; this crate only
//! carries the request.
//!
//! What a request sends, and nothing else:
//!
//! - The release list: `GET` [`PATH`] on [`HOST`], port [`PORT`], over TLS
//!   ([`SCHEME`]), with three headers: `Accept: `[`ACCEPT`],
//!   `X-GitHub-Api-Version: `[`API_VERSION`] and `User-Agent: ` the
//!   product's ([`QUEST_TRACKER_USER_AGENT`], [`WAYSTONE_ROLLER_USER_AGENT`]
//!   or [`CRAFTING_HELPER_USER_AGENT`]; GitHub refuses a request without one;
//!   it names no version and no machine detail).
//! - The trade site's league list: `GET` [`TRADE_LEAGUES_PATH`] on
//!   [`TRADE_LEAGUES_HOST`], port [`PORT`], over TLS, with two headers:
//!   `Accept: `[`TRADE_LEAGUES_ACCEPT`] and `User-Agent: `
//!   [`CRAFTING_HELPER_USER_AGENT`]. Only the crafting helper sends it, and
//!   only when the player asks for the list; the trade site sees the
//!   machine's IP address and those two headers.
//! - A release asset: `GET https://`[`DOWNLOAD_HOST`][`DOWNLOAD_PATH_PREFIX`]
//!   `<tag>/<asset>`, port 443, with two headers: `Accept: `
//!   [`DOWNLOAD_ACCEPT`] and the product's `User-Agent`. GitHub answers that
//!   with a redirect to a signed URL on its asset host; exactly one redirect
//!   is followed, and only to HTTPS port 443 on one of [`REDIRECT_HOSTS`],
//!   with the same two headers. Any other redirect, and a second one, is an
//!   error. The signed URL is never logged, stored or returned: an error
//!   names what went wrong, never where.
//! - GitHub sees the machine's IP address and that header set.
//! - No cookie is sent: each request opens a WinHTTP session of its own,
//!   with cookies turned off, and closes it, so a cookie a server set is gone
//!   with it (tested: a fetch after an answer that set one sends no
//!   `Cookie`). No credential is sent (WinHTTP's autologon policy is set
//!   high), and WinHTTP itself follows no redirect: the release list and the
//!   league list return a 3xx as the answer, and the download checks its one
//!   redirect here.
//! - The proxy is the system's. TLS and the certificate checks are Windows'
//!   own.
//!
//! Bounds of the release list: resolve, connect and send 5 s each, receive
//! 10 s, and the whole request ends within [`TOTAL_DEADLINE`] whatever the
//! server does (the remaining time bounds every wait and is checked between
//! reads). The body is read up to [`MAX_BODY`] bytes and refused past it.
//! The league list has the same bounds, and its body is read up to
//! [`MAX_TRADE_LEAGUES_BODY`] bytes and refused past it. Each request's path,
//! headers and body cap are its target's own ([`Target`]), so neither sends
//! anything of the other's.
//!
//! Bounds of a download: resolve, connect and send 5 s each, then at most
//! [`DOWNLOAD_INACTIVITY`] without a byte arriving, and both legs together
//! end within [`DOWNLOAD_DEADLINE`] (15 minutes: a 10 MB exe at about
//! 11 KB/s). The body streams into the caller's writer, at most the asset's
//! cap ([`MAX_EXE`], [`MAX_SIGNATURE`]) or the caller's smaller limit; a
//! longer answer is refused. The caller's cancel flag is checked before
//! each leg and between reads, and its progress callback hears every read.
//! Any final status but 200 is an error and writes nothing.
//!
//! Elsewhere than Windows every request returns [`FetchError::Unsupported`]
//! without trying.

use std::fmt;
use std::io::Write;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

#[cfg(windows)]
mod win;

/// The only host the release list is fetched from.
pub const HOST: &str = "api.github.com";
/// The only port any request goes to first.
pub const PORT: u16 = 443;
/// The only scheme any request uses.
pub const SCHEME: &str = "https";
/// GitHub's REST "List releases" of the repository, the 30 most recent.
pub const PATH: &str = "/repos/S-Borkowski/ExilePact2/releases?per_page=30";
/// The `Accept` header GitHub's REST API documents.
pub const ACCEPT: &str = "application/vnd.github+json";
/// The `X-GitHub-Api-Version` header: the API version the answer is read as.
pub const API_VERSION: &str = "2022-11-28";
/// The largest release-list body read; one byte more is refused.
pub const MAX_BODY: usize = 2 * 1024 * 1024;
/// The bound on a whole release-list request, from the first call to the
/// last read.
pub const TOTAL_DEADLINE: Duration = Duration::from_secs(20);

/// The only host the trade site's league list is fetched from.
pub const TRADE_LEAGUES_HOST: &str = "www.pathofexile.com";
/// The trade site's public list of Path of Exile 2's leagues.
pub const TRADE_LEAGUES_PATH: &str = "/api/trade2/data/leagues";
/// The league list's `Accept` header.
pub const TRADE_LEAGUES_ACCEPT: &str = "application/json";
/// The largest league-list body read; one byte more is refused.
pub const MAX_TRADE_LEAGUES_BODY: usize = 64 * 1024;

/// The quest tracker's `User-Agent`: the app's name, no version, no
/// machine detail.
pub const QUEST_TRACKER_USER_AGENT: &str = "ExilePact2-quest-tracker";
/// The waystone roller's `User-Agent`, in the same form.
pub const WAYSTONE_ROLLER_USER_AGENT: &str = "ExilePact2-waystone-roller";
/// The crafting helper's `User-Agent`, in the same form.
pub const CRAFTING_HELPER_USER_AGENT: &str = "ExilePact2-crafting-helper";
/// The quest tracker's release tags: this prefix and `<major>.<minor>.<patch>`.
pub const QUEST_TRACKER_TAG_PREFIX: &str = "quest-tracker-v";
/// The waystone roller's release tags, in the same form.
pub const WAYSTONE_ROLLER_TAG_PREFIX: &str = "waystone-roller-v";
/// The crafting helper's release tags, in the same form.
pub const CRAFTING_HELPER_TAG_PREFIX: &str = "crafting-helper-v";
/// The quest tracker's exe, as its releases name the asset.
pub const QUEST_TRACKER_EXE: &str = "quest-tracker.exe";
/// The waystone roller's exe, as its releases name the asset.
pub const WAYSTONE_ROLLER_EXE: &str = "waystone-roller.exe";
/// The crafting helper's exe, as its releases name the asset. The built
/// file is `Crafting Helper.exe`; GitHub stores an asset name's space as a
/// dot, so the release carries it as `Crafting.Helper.exe`. Releases from
/// 0.2.0 also carry the same exe as `crafting-helper.exe`, the name 0.1.x
/// copies look up; this build never fetches that one.
pub const CRAFTING_HELPER_EXE: &str = "Crafting.Helper.exe";
/// The suffix of an exe's signature asset (minisign's signature text).
pub const SIGNATURE_SUFFIX: &str = ".minisig";

/// The only host a download's first request goes to.
pub const DOWNLOAD_HOST: &str = "github.com";
/// Every download's path starts with this; the tag and the asset name follow.
pub const DOWNLOAD_PATH_PREFIX: &str = "/S-Borkowski/ExilePact2/releases/download/";
/// A download's `Accept` header.
pub const DOWNLOAD_ACCEPT: &str = "application/octet-stream";
/// The only hosts a download's one redirect may lead to, over HTTPS on port
/// 443: GitHub's release-asset host, and the asset host it used before it.
pub const REDIRECT_HOSTS: [&str; 2] = [
    "release-assets.githubusercontent.com",
    "objects.githubusercontent.com",
];
/// The longest wait for the next byte of a download.
pub const DOWNLOAD_INACTIVITY: Duration = Duration::from_secs(30);
/// The bound on a whole download, both legs, from the first call to the
/// last read.
pub const DOWNLOAD_DEADLINE: Duration = Duration::from_secs(15 * 60);
/// The largest exe read; one byte more is refused.
pub const MAX_EXE: u64 = 64 * 1024 * 1024;
/// The largest signature read; one byte more is refused.
pub const MAX_SIGNATURE: u64 = 4 * 1024;

/// The app a request is made for: its `User-Agent`, its release tags and
/// its exe asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Product {
    QuestTracker,
    WaystoneRoller,
    CraftingHelper,
}

impl Product {
    /// The `User-Agent` every request of this product sends.
    pub const fn user_agent(self) -> &'static str {
        match self {
            Product::QuestTracker => QUEST_TRACKER_USER_AGENT,
            Product::WaystoneRoller => WAYSTONE_ROLLER_USER_AGENT,
            Product::CraftingHelper => CRAFTING_HELPER_USER_AGENT,
        }
    }

    /// The prefix of this product's release tags.
    pub const fn tag_prefix(self) -> &'static str {
        match self {
            Product::QuestTracker => QUEST_TRACKER_TAG_PREFIX,
            Product::WaystoneRoller => WAYSTONE_ROLLER_TAG_PREFIX,
            Product::CraftingHelper => CRAFTING_HELPER_TAG_PREFIX,
        }
    }

    /// The file name of this product's exe asset.
    pub const fn exe_name(self) -> &'static str {
        match self {
            Product::QuestTracker => QUEST_TRACKER_EXE,
            Product::WaystoneRoller => WAYSTONE_ROLLER_EXE,
            Product::CraftingHelper => CRAFTING_HELPER_EXE,
        }
    }
}

/// One of a release's two assets a download can name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Asset {
    /// The product's exe.
    Exe,
    /// The exe's minisign signature, `<exe>.minisig`.
    Signature,
}

impl Asset {
    /// The largest answer read for this asset, whatever the caller allows.
    pub const fn cap(self) -> u64 {
        match self {
            Asset::Exe => MAX_EXE,
            Asset::Signature => MAX_SIGNATURE,
        }
    }
}

/// A release of one product: its tag is the product's prefix and the three
/// numbers, so no caller can name another tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReleaseTag {
    pub product: Product,
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl ReleaseTag {
    pub const fn new(product: Product, major: u32, minor: u32, patch: u32) -> Self {
        ReleaseTag {
            product,
            major,
            minor,
            patch,
        }
    }

    /// The tag, `<prefix><major>.<minor>.<patch>`.
    pub fn tag(&self) -> String {
        format!(
            "{}{}.{}.{}",
            self.product.tag_prefix(),
            self.major,
            self.minor,
            self.patch
        )
    }

    /// The asset's file name in this release.
    pub fn asset_name(&self, asset: Asset) -> String {
        let exe = self.product.exe_name();
        match asset {
            Asset::Exe => exe.to_owned(),
            Asset::Signature => format!("{exe}{SIGNATURE_SUFFIX}"),
        }
    }

    /// The path of the asset's download on [`DOWNLOAD_HOST`].
    pub(crate) fn download_path(&self, asset: Asset) -> String {
        format!(
            "{DOWNLOAD_PATH_PREFIX}{}/{}",
            self.tag(),
            self.asset_name(asset)
        )
    }
}

/// The server's answer: its HTTP status and body, whatever the status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub status: u16,
    pub body: Vec<u8>,
}

/// Why a request produced no answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FetchError {
    /// This build has no transport (not Windows); nothing was tried.
    Unsupported,
    /// A wait ran out, or the whole request reached its deadline.
    Timeout,
    /// The name did not resolve, or the connection was refused or lost.
    Unreachable,
    /// The TLS connection or the server's certificate failed.
    NotSecure,
    /// The body was longer than the cap.
    TooLarge,
    /// The request's thread ended without an answer (it panicked).
    Panicked,
    /// Any other failure, with WinHTTP's (or the system's) error code.
    Other(u32),
}

impl fmt::Display for FetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FetchError::Unsupported => f.write_str("network requests are not supported here"),
            FetchError::Timeout => f.write_str("the request timed out"),
            FetchError::Unreachable => f.write_str("the server could not be reached"),
            FetchError::NotSecure => f.write_str("the secure connection failed"),
            FetchError::TooLarge => f.write_str("the answer was too large"),
            FetchError::Panicked => f.write_str("the request stopped unexpectedly"),
            FetchError::Other(code) => write!(f, "the request failed (error {code})"),
        }
    }
}

impl std::error::Error for FetchError {}

/// Why a download did not complete. Nothing in it names a URL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadError {
    /// No answer, or one over the cap; the same reasons as the release list.
    Fetch(FetchError),
    /// The final answer's status was not 200 (nothing was written).
    Status(u16),
    /// The redirect led somewhere other than an allowed asset host over
    /// HTTPS on port 443, or named no place at all; it was not followed.
    RedirectRefused,
    /// The redirect's target redirected again; that was not followed.
    TooManyRedirects,
    /// The connection ended before the length the server announced.
    Incomplete,
    /// The caller's cancel flag was set.
    Cancelled,
    /// The caller's writer failed.
    Write(std::io::ErrorKind),
}

impl fmt::Display for DownloadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DownloadError::Fetch(e) => e.fmt(f),
            DownloadError::Status(code) => write!(f, "the server answered {code}"),
            DownloadError::RedirectRefused => {
                f.write_str("the download was redirected to an unexpected place")
            }
            DownloadError::TooManyRedirects => f.write_str("the download was redirected twice"),
            DownloadError::Incomplete => f.write_str("the download ended early"),
            DownloadError::Cancelled => f.write_str("the download was cancelled"),
            DownloadError::Write(kind) => write!(f, "the download could not be saved ({kind})"),
        }
    }
}

impl std::error::Error for DownloadError {}

impl From<FetchError> for DownloadError {
    fn from(e: FetchError) -> Self {
        DownloadError::Fetch(e)
    }
}

/// Fetches the release list for `product`: one `GET` of [`PATH`] on
/// [`HOST`], blocking the calling thread for at most [`TOTAL_DEADLINE`]. Any
/// status is an answer (a 3xx, 403 or 500 included); only a failure to get
/// one is an error.
pub fn fetch_release_list(product: Product) -> Result<Response, FetchError> {
    get(&Target::release_list(product))
}

/// Fetches the trade site's league list: one `GET` of [`TRADE_LEAGUES_PATH`]
/// on [`TRADE_LEAGUES_HOST`] with the crafting helper's `User-Agent`,
/// blocking the calling thread for at most [`TOTAL_DEADLINE`]. Any status is
/// an answer (a 3xx included, which is not followed); only a failure to get
/// one is an error. Takes nothing: no caller names a place.
pub fn fetch_trade_leagues() -> Result<Response, FetchError> {
    get(&Target::trade_leagues())
}

/// Downloads one asset of `release` into `sink`, blocking the calling
/// thread (run it on a thread of its own) for at most [`DOWNLOAD_DEADLINE`]
/// plus WinHTTP's coarse last wait. Reads at most `limit` bytes, or the
/// asset's [`Asset::cap`] if that is smaller, and returns how many it wrote.
///
/// `progress` hears `(bytes written so far, the length the server announced)`
/// after every read. `cancel` is checked before each leg and between reads;
/// once set, the download ends with [`DownloadError::Cancelled`]. On any
/// error the sink may hold part of the asset (never for a non-200 answer,
/// which writes nothing); the caller discards it.
pub fn download_asset(
    release: ReleaseTag,
    asset: Asset,
    limit: u64,
    sink: &mut dyn Write,
    progress: &mut dyn FnMut(u64, Option<u64>),
    cancel: &AtomicBool,
) -> Result<u64, DownloadError> {
    download(
        &Target::download(release.product),
        &release.download_path(asset),
        limit.min(asset.cap()),
        sink,
        progress,
        cancel,
    )
}

/// The bounds of one request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Timeouts {
    pub(crate) resolve: Duration,
    pub(crate) connect: Duration,
    pub(crate) send: Duration,
    pub(crate) receive: Duration,
    pub(crate) total: Duration,
}

/// Where a download's one redirect may lead: these hosts only, with this
/// scheme and port. The production policy is [`REDIRECT_HOSTS`] over HTTPS on
/// port 443; the tests set their own, plain HTTP to their loopback port.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Redirects {
    pub(crate) secure: bool,
    pub(crate) port: u16,
    pub(crate) hosts: &'static [&'static str],
}

/// What a request sends besides its place and its `User-Agent`, and how
/// much of the answer it reads: the path, the `Accept` header, GitHub's
/// API version header when it has one, and the body's cap. A target's
/// request is what `get` sends; a download always sends
/// [`Request::DOWNLOAD`]'s header with the caller's path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Request {
    /// The path asked for; for a download, the prefix the caller's path
    /// starts with.
    pub(crate) path: &'static str,
    pub(crate) accept: &'static str,
    /// `X-GitHub-Api-Version`, sent only when set.
    pub(crate) api_version: Option<&'static str>,
    /// The largest body `get` reads; a download's cap is the asset's and
    /// the caller's instead.
    pub(crate) max_body: usize,
}

impl Request {
    /// The release list's request, as it has always been sent.
    pub(crate) const RELEASE_LIST: Request = Request {
        path: PATH,
        accept: ACCEPT,
        api_version: Some(API_VERSION),
        max_body: MAX_BODY,
    };
    /// The trade site's league list: its path and `Accept`, nothing of
    /// GitHub's.
    pub(crate) const TRADE_LEAGUES: Request = Request {
        path: TRADE_LEAGUES_PATH,
        accept: TRADE_LEAGUES_ACCEPT,
        api_version: None,
        max_body: MAX_TRADE_LEAGUES_BODY,
    };
    /// A release asset's download.
    pub(crate) const DOWNLOAD: Request = Request {
        path: DOWNLOAD_PATH_PREFIX,
        accept: DOWNLOAD_ACCEPT,
        api_version: None,
        max_body: 0,
    };

    /// The headers this request adds to the `User-Agent` and `Host`, as
    /// CRLF-ended lines.
    pub(crate) fn headers(&self) -> String {
        let mut headers = format!("Accept: {}\r\n", self.accept);
        if let Some(version) = self.api_version {
            headers.push_str(&format!("X-GitHub-Api-Version: {version}\r\n"));
        }
        headers
    }
}

/// Where a request goes and what it sends. Only [`Target::release_list`],
/// [`Target::download`] and [`Target::trade_leagues`] are reachable from
/// outside the crate; the tests build their own, plain HTTP to a loopback
/// port they opened, with no proxy, shorter bounds and a redirect allowlist
/// of their own, and one of the requests above.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Target {
    pub(crate) host: &'static str,
    pub(crate) port: u16,
    pub(crate) secure: bool,
    pub(crate) system_proxy: bool,
    pub(crate) user_agent: &'static str,
    pub(crate) timeouts: Timeouts,
    pub(crate) redirects: Redirects,
    pub(crate) request: Request,
}

/// The bounds of the release list and the league list.
const LIST_TIMEOUTS: Timeouts = Timeouts {
    resolve: Duration::from_secs(5),
    connect: Duration::from_secs(5),
    send: Duration::from_secs(5),
    receive: Duration::from_secs(10),
    total: TOTAL_DEADLINE,
};

/// No redirect is followed at all.
const NO_REDIRECT: Redirects = Redirects {
    secure: true,
    port: PORT,
    hosts: &[],
};

impl Target {
    pub(crate) fn release_list(product: Product) -> Self {
        Target {
            host: HOST,
            port: PORT,
            secure: SCHEME == "https",
            system_proxy: true,
            user_agent: product.user_agent(),
            timeouts: LIST_TIMEOUTS,
            // The release list follows no redirect at all.
            redirects: NO_REDIRECT,
            request: Request::RELEASE_LIST,
        }
    }

    pub(crate) fn trade_leagues() -> Self {
        Target {
            host: TRADE_LEAGUES_HOST,
            port: PORT,
            secure: SCHEME == "https",
            system_proxy: true,
            user_agent: CRAFTING_HELPER_USER_AGENT,
            timeouts: LIST_TIMEOUTS,
            // Nor does the league list.
            redirects: NO_REDIRECT,
            request: Request::TRADE_LEAGUES,
        }
    }

    pub(crate) fn download(product: Product) -> Self {
        Target {
            host: DOWNLOAD_HOST,
            port: PORT,
            secure: SCHEME == "https",
            system_proxy: true,
            user_agent: product.user_agent(),
            timeouts: Timeouts {
                resolve: Duration::from_secs(5),
                connect: Duration::from_secs(5),
                send: Duration::from_secs(5),
                receive: DOWNLOAD_INACTIVITY,
                total: DOWNLOAD_DEADLINE,
            },
            redirects: Redirects {
                secure: true,
                port: PORT,
                hosts: &REDIRECT_HOSTS,
            },
            request: Request::DOWNLOAD,
        }
    }
}

/// The longest `Location` value read; a longer one is refused.
pub(crate) const MAX_LOCATION: usize = 8 * 1024;

/// Whether a status is a redirect a download may follow.
pub(crate) fn is_redirect(status: u16) -> bool {
    matches!(status, 301 | 302 | 303 | 307 | 308)
}

/// The host and the path (with its query) a redirect's `Location` leads
/// to, when `policy` allows it: an absolute URL with the policy's scheme,
/// one of its hosts (compared without case), its port (named or implied),
/// no user part, and a path of visible ASCII. The host returned is the
/// policy's own string, never the server's. Anything else is `None`.
pub(crate) fn redirect_leg(policy: &Redirects, location: &str) -> Option<(&'static str, String)> {
    if location.len() > MAX_LOCATION {
        return None;
    }
    let scheme = if policy.secure { "https://" } else { "http://" };
    let rest = location.strip_prefix(scheme)?;
    let slash = rest.find('/')?;
    let (authority, path) = rest.split_at(slash);
    if authority.is_empty() || authority.contains(['@', '?', '#', '\\']) {
        return None;
    }
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) => {
            if port.is_empty() || !port.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            (host, port.parse::<u16>().ok()?)
        }
        None => (authority, if policy.secure { 443 } else { 80 }),
    };
    if port != policy.port {
        return None;
    }
    let allowed = policy
        .hosts
        .iter()
        .copied()
        .find(|h| h.eq_ignore_ascii_case(host))?;
    // The fragment is the client's, never sent.
    let path = path.split('#').next().unwrap_or_default();
    if !path.bytes().all(|b| b.is_ascii_graphic() && b != b'\\') {
        return None;
    }
    Some((allowed, path.to_owned()))
}

#[cfg(windows)]
pub(crate) fn get(target: &Target) -> Result<Response, FetchError> {
    win::get(target)
}

#[cfg(not(windows))]
pub(crate) fn get(_target: &Target) -> Result<Response, FetchError> {
    Err(FetchError::Unsupported)
}

#[cfg(windows)]
pub(crate) fn download(
    target: &Target,
    path: &str,
    cap: u64,
    sink: &mut dyn Write,
    progress: &mut dyn FnMut(u64, Option<u64>),
    cancel: &AtomicBool,
) -> Result<u64, DownloadError> {
    win::download(target, path, cap, sink, progress, cancel)
}

#[cfg(not(windows))]
pub(crate) fn download(
    _target: &Target,
    _path: &str,
    _cap: u64,
    _sink: &mut dyn Write,
    _progress: &mut dyn FnMut(u64, Option<u64>),
    _cancel: &AtomicBool,
) -> Result<u64, DownloadError> {
    Err(DownloadError::Fetch(FetchError::Unsupported))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_is_the_release_list_over_https() {
        assert_eq!(HOST, "api.github.com");
        assert_eq!(PORT, 443);
        assert_eq!(SCHEME, "https");
        assert_eq!(PATH, "/repos/S-Borkowski/ExilePact2/releases?per_page=30");
        assert_eq!(ACCEPT, "application/vnd.github+json");
        assert_eq!(API_VERSION, "2022-11-28");
        assert_eq!(QUEST_TRACKER_USER_AGENT, "ExilePact2-quest-tracker");
        assert_eq!(WAYSTONE_ROLLER_USER_AGENT, "ExilePact2-waystone-roller");
        assert_eq!(CRAFTING_HELPER_USER_AGENT, "ExilePact2-crafting-helper");
        assert_eq!(MAX_BODY, 2 * 1024 * 1024);
        for (product, agent) in [
            (Product::QuestTracker, QUEST_TRACKER_USER_AGENT),
            (Product::WaystoneRoller, WAYSTONE_ROLLER_USER_AGENT),
            (Product::CraftingHelper, CRAFTING_HELPER_USER_AGENT),
        ] {
            let t = Target::release_list(product);
            assert_eq!(t.host, HOST);
            assert_eq!(t.port, PORT);
            assert!(t.secure, "the release list is fetched over TLS only");
            assert!(t.system_proxy);
            assert_eq!(t.user_agent, agent);
            assert_eq!(t.timeouts.resolve, Duration::from_secs(5));
            assert_eq!(t.timeouts.connect, Duration::from_secs(5));
            assert_eq!(t.timeouts.send, Duration::from_secs(5));
            assert_eq!(t.timeouts.receive, Duration::from_secs(10));
            assert_eq!(t.timeouts.total, Duration::from_secs(20));
            assert!(
                t.redirects.hosts.is_empty(),
                "the release list follows no redirect"
            );
        }
    }

    #[test]
    fn trade_leagues_endpoint_is_fixed() {
        assert_eq!(TRADE_LEAGUES_HOST, "www.pathofexile.com");
        assert_eq!(TRADE_LEAGUES_PATH, "/api/trade2/data/leagues");
        assert_eq!(TRADE_LEAGUES_ACCEPT, "application/json");
        assert_eq!(MAX_TRADE_LEAGUES_BODY, 64 * 1024);
        let t = Target::trade_leagues();
        assert_eq!(t.host, TRADE_LEAGUES_HOST);
        assert_eq!(t.port, 443);
        assert!(t.secure, "the league list is fetched over TLS only");
        assert!(t.system_proxy);
        assert_eq!(t.user_agent, CRAFTING_HELPER_USER_AGENT);
        // The release list's bounds, and no redirect followed.
        assert_eq!(
            t.timeouts,
            Target::release_list(Product::CraftingHelper).timeouts
        );
        assert_eq!(t.timeouts.total, Duration::from_secs(20));
        assert!(
            t.redirects.hosts.is_empty(),
            "the league list follows no redirect"
        );
        assert_eq!(
            t.request,
            Request {
                path: TRADE_LEAGUES_PATH,
                accept: TRADE_LEAGUES_ACCEPT,
                api_version: None,
                max_body: MAX_TRADE_LEAGUES_BODY,
            }
        );
        assert_eq!(t.request.headers(), "Accept: application/json\r\n");
        // Each request is its own: the release list's is unchanged, and
        // neither carries the other's path or headers.
        assert_eq!(
            Request::RELEASE_LIST.headers(),
            format!("Accept: {ACCEPT}\r\nX-GitHub-Api-Version: {API_VERSION}\r\n")
        );
        for product in [
            Product::QuestTracker,
            Product::WaystoneRoller,
            Product::CraftingHelper,
        ] {
            assert_eq!(Target::release_list(product).request, Request::RELEASE_LIST);
        }
        assert_eq!(
            Request::DOWNLOAD.headers(),
            format!("Accept: {DOWNLOAD_ACCEPT}\r\n")
        );
    }

    #[test]
    fn download_paths_are_built_from_the_tag_and_a_fixed_asset_name() {
        let qt = ReleaseTag::new(Product::QuestTracker, 0, 2, 13);
        assert_eq!(qt.tag(), "quest-tracker-v0.2.13");
        assert_eq!(qt.asset_name(Asset::Exe), "quest-tracker.exe");
        assert_eq!(qt.asset_name(Asset::Signature), "quest-tracker.exe.minisig");
        assert_eq!(
            format!("{SCHEME}://{DOWNLOAD_HOST}{}", qt.download_path(Asset::Exe)),
            "https://github.com/S-Borkowski/ExilePact2/releases/download/quest-tracker-v0.2.13/quest-tracker.exe"
        );
        assert_eq!(
            qt.download_path(Asset::Signature),
            "/S-Borkowski/ExilePact2/releases/download/quest-tracker-v0.2.13/quest-tracker.exe.minisig"
        );

        let wr = ReleaseTag::new(Product::WaystoneRoller, 1, 0, 0);
        assert_eq!(wr.tag(), "waystone-roller-v1.0.0");
        assert_eq!(
            wr.download_path(Asset::Exe),
            "/S-Borkowski/ExilePact2/releases/download/waystone-roller-v1.0.0/waystone-roller.exe"
        );
        assert_eq!(
            wr.download_path(Asset::Signature),
            "/S-Borkowski/ExilePact2/releases/download/waystone-roller-v1.0.0/waystone-roller.exe.minisig"
        );

        // From 0.2.0 the crafting helper's exe is `Crafting Helper.exe`,
        // which GitHub stores as `Crafting.Helper.exe` (a space becomes a
        // dot); the asset a download fetches is that stored name.
        let ch = ReleaseTag::new(Product::CraftingHelper, 0, 2, 0);
        assert_eq!(ch.tag(), "crafting-helper-v0.2.0");
        assert_eq!(ch.asset_name(Asset::Exe), "Crafting.Helper.exe");
        assert_eq!(
            ch.asset_name(Asset::Signature),
            "Crafting.Helper.exe.minisig"
        );
        assert_eq!(
            ch.download_path(Asset::Exe),
            "/S-Borkowski/ExilePact2/releases/download/crafting-helper-v0.2.0/Crafting.Helper.exe"
        );
        assert_eq!(
            ch.download_path(Asset::Signature),
            "/S-Borkowski/ExilePact2/releases/download/crafting-helper-v0.2.0/Crafting.Helper.exe.minisig"
        );
        // No asset name holds a character a request path would have to
        // escape, so the path above is sent as it stands.
        for name in [QUEST_TRACKER_EXE, WAYSTONE_ROLLER_EXE, CRAFTING_HELPER_EXE] {
            assert!(
                name.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b)),
                "{name}"
            );
        }

        assert_eq!(Asset::Exe.cap(), 64 * 1024 * 1024);
        assert_eq!(Asset::Signature.cap(), 4 * 1024);
        for (product, agent) in [
            (Product::QuestTracker, QUEST_TRACKER_USER_AGENT),
            (Product::WaystoneRoller, WAYSTONE_ROLLER_USER_AGENT),
            (Product::CraftingHelper, CRAFTING_HELPER_USER_AGENT),
        ] {
            let t = Target::download(product);
            assert_eq!(t.host, "github.com");
            assert_eq!(t.port, 443);
            assert!(t.secure, "a download starts over TLS only");
            assert!(t.system_proxy);
            assert_eq!(t.user_agent, agent);
            assert_eq!(t.timeouts.resolve, Duration::from_secs(5));
            assert_eq!(t.timeouts.connect, Duration::from_secs(5));
            assert_eq!(t.timeouts.send, Duration::from_secs(5));
            assert_eq!(t.timeouts.receive, Duration::from_secs(30));
            assert_eq!(t.timeouts.total, Duration::from_secs(900));
        }
    }

    #[test]
    fn redirect_targets_are_only_the_github_asset_hosts() {
        assert_eq!(
            REDIRECT_HOSTS,
            [
                "release-assets.githubusercontent.com",
                "objects.githubusercontent.com"
            ]
        );
        let policy = Target::download(Product::QuestTracker).redirects;
        assert!(policy.secure);
        assert_eq!(policy.port, 443);
        assert_eq!(policy.hosts, &REDIRECT_HOSTS);

        // What GitHub answers today (the shape measured while planning),
        // the earlier asset host, the port named, and the host's case.
        let signed = "/github-production-release-asset/1/2?X-Amz-Signature=ab%2Fcd&response-content-disposition=attachment%3B%20filename%3Dquest-tracker.exe";
        assert_eq!(
            redirect_leg(
                &policy,
                &format!("https://release-assets.githubusercontent.com{signed}")
            ),
            Some(("release-assets.githubusercontent.com", signed.to_owned()))
        );
        assert_eq!(
            redirect_leg(&policy, "https://objects.githubusercontent.com/a/b?c=d"),
            Some(("objects.githubusercontent.com", "/a/b?c=d".to_owned()))
        );
        assert_eq!(
            redirect_leg(
                &policy,
                "https://release-assets.githubusercontent.com:443/x"
            ),
            Some(("release-assets.githubusercontent.com", "/x".to_owned()))
        );
        assert_eq!(
            redirect_leg(
                &policy,
                "https://Release-Assets.GitHubUserContent.com/x#frag"
            ),
            Some(("release-assets.githubusercontent.com", "/x".to_owned()))
        );

        for refused in [
            // Not HTTPS, or not port 443.
            "http://release-assets.githubusercontent.com/x",
            "HTTPS://release-assets.githubusercontent.com/x",
            "https://release-assets.githubusercontent.com:8443/x",
            "https://release-assets.githubusercontent.com:/x",
            "https://release-assets.githubusercontent.com:44a/x",
            // Another host, or one that only looks like an asset host.
            "https://evil.example/x",
            "https://github.com/x",
            "https://api.github.com/x",
            "https://release-assets.githubusercontent.com.evil.example/x",
            "https://evil-release-assets.githubusercontent.com/x",
            "https://githubusercontent.com/x",
            // A user part, or a trick to hide the real host.
            "https://user@release-assets.githubusercontent.com/x",
            "https://evil.example@release-assets.githubusercontent.com/x",
            "https://release-assets.githubusercontent.com@evil.example/x",
            "https://evil.example\\@release-assets.githubusercontent.com/x",
            "https://release-assets.githubusercontent.com?/x",
            // No path, a relative place, or a path that is not plain ASCII.
            "https://release-assets.githubusercontent.com",
            "/x",
            "//release-assets.githubusercontent.com/x",
            "",
            "https://release-assets.githubusercontent.com/a b",
            "https://release-assets.githubusercontent.com/a\r\nX-Injected: 1",
            "https://release-assets.githubusercontent.com/\u{e9}",
            "https://release-assets.githubusercontent.com/a\\b",
        ] {
            assert_eq!(redirect_leg(&policy, refused), None, "followed {refused:?}");
        }
        let long = format!(
            "https://release-assets.githubusercontent.com/{}",
            "a".repeat(MAX_LOCATION)
        );
        assert_eq!(
            redirect_leg(&policy, &long),
            None,
            "followed a very long Location"
        );

        // The release list's policy allows nothing.
        let none = Target::release_list(Product::QuestTracker).redirects;
        assert_eq!(
            redirect_leg(&none, "https://release-assets.githubusercontent.com/x"),
            None
        );

        // Only 301, 302, 303, 307 and 308 are followed.
        for status in [301, 302, 303, 307, 308] {
            assert!(is_redirect(status), "{status}");
        }
        for status in [200, 300, 304, 305, 306, 404] {
            assert!(!is_redirect(status), "{status}");
        }
    }

    #[test]
    fn download_errors_never_name_a_place() {
        let all = [
            DownloadError::Fetch(FetchError::Timeout),
            DownloadError::Status(404),
            DownloadError::RedirectRefused,
            DownloadError::TooManyRedirects,
            DownloadError::Incomplete,
            DownloadError::Cancelled,
            DownloadError::Write(std::io::ErrorKind::PermissionDenied),
        ];
        for e in all {
            let text = e.to_string();
            assert!(!text.contains("http") && !text.contains('/'), "{text}");
            assert!(!text.contains('\n'), "{text}");
        }
        assert_eq!(
            DownloadError::Status(404).to_string(),
            "the server answered 404"
        );
        assert_eq!(
            DownloadError::from(FetchError::TooLarge),
            DownloadError::Fetch(FetchError::TooLarge)
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn fetch_is_unsupported_off_windows() {
        assert_eq!(
            fetch_release_list(Product::QuestTracker),
            Err(FetchError::Unsupported)
        );
        let mut sink = Vec::new();
        let cancel = AtomicBool::new(false);
        assert_eq!(
            download_asset(
                ReleaseTag::new(Product::QuestTracker, 1, 0, 0),
                Asset::Exe,
                MAX_EXE,
                &mut sink,
                &mut |_, _| {},
                &cancel
            ),
            Err(DownloadError::Fetch(FetchError::Unsupported))
        );
    }

    /// The loopback tests: a server on `127.0.0.1` (never every interface,
    /// which can raise a firewall prompt) that the test runs, reached
    /// through the same code as the real request.
    #[cfg(windows)]
    mod loopback {
        use super::super::*;
        use std::io::{ErrorKind, Read, Write};
        use std::net::{TcpListener, TcpStream};
        use std::sync::atomic::Ordering;
        use std::sync::mpsc;
        use std::thread;
        use std::time::Instant;

        fn listen() -> (TcpListener, u16) {
            let listener = TcpListener::bind("127.0.0.1:0").expect("bind 127.0.0.1");
            let port = listener.local_addr().unwrap().port();
            (listener, port)
        }

        /// Plain HTTP, no proxy, short bounds, and a redirect allowlist of
        /// this loopback host and port alone. Connect stays at 5 s because
        /// Windows retries a refused loopback connect for about 2 s.
        fn target(port: u16) -> Target {
            Target {
                host: "127.0.0.1",
                port,
                secure: false,
                system_proxy: false,
                user_agent: QUEST_TRACKER_USER_AGENT,
                timeouts: Timeouts {
                    resolve: Duration::from_secs(2),
                    connect: Duration::from_secs(5),
                    send: Duration::from_secs(2),
                    receive: Duration::from_secs(1),
                    total: Duration::from_secs(8),
                },
                redirects: Redirects {
                    secure: false,
                    port,
                    hosts: &["127.0.0.1"],
                },
                request: Request::RELEASE_LIST,
            }
        }

        /// The league list's request to the loopback server, as the
        /// crafting helper sends it.
        fn leagues_target(port: u16) -> Target {
            Target {
                user_agent: CRAFTING_HELPER_USER_AGENT,
                request: Request::TRADE_LEAGUES,
                ..target(port)
            }
        }

        /// The server's next connection, or a panic after 10 s, so a client
        /// that never connects fails the test instead of hanging it.
        fn accept(listener: &TcpListener) -> TcpStream {
            listener.set_nonblocking(true).unwrap();
            let until = Instant::now() + Duration::from_secs(10);
            loop {
                match listener.accept() {
                    Ok((stream, _)) => {
                        stream.set_nonblocking(false).unwrap();
                        return stream;
                    }
                    Err(e) if e.kind() == ErrorKind::WouldBlock && Instant::now() < until => {
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(e) => panic!("no request arrived: {e}"),
                }
            }
        }

        /// Fails the test if the client connected again: a connection the
        /// server never accepted still waits in the queue.
        fn assert_no_more_connections(listener: &TcpListener) {
            listener.set_nonblocking(true).unwrap();
            match listener.accept() {
                Err(e) if e.kind() == ErrorKind::WouldBlock => {}
                Err(e) => panic!("accept failed: {e}"),
                Ok((_, from)) => panic!("another request came from {from}"),
            }
        }

        /// Reads one request's head, up to the blank line.
        fn read_head(stream: &mut TcpStream) -> String {
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut head = Vec::new();
            let mut byte = [0u8; 1];
            while !head.ends_with(b"\r\n\r\n") {
                match stream.read(&mut byte) {
                    Ok(1) => head.push(byte[0]),
                    _ => break,
                }
            }
            String::from_utf8(head).expect("an ASCII request head")
        }

        fn header<'a>(head: &'a str, name: &str) -> Vec<&'a str> {
            head.split("\r\n")
                .skip(1)
                .filter_map(|line| line.split_once(':'))
                .filter(|(n, _)| n.trim().eq_ignore_ascii_case(name))
                .map(|(_, v)| v.trim())
                .collect()
        }

        fn request_line(head: &str) -> &str {
            head.split("\r\n").next().unwrap()
        }

        fn assert_sends_nothing_private(head: &str) {
            for absent in ["Cookie", "Authorization", "Proxy-Authorization", "Referer"] {
                assert!(header(head, absent).is_empty(), "{absent} sent: {head}");
            }
        }

        /// A 200 answer with `body`, and the connection closed.
        fn answer_ok(stream: &mut TcpStream, body: &[u8]) {
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .unwrap();
            stream.write_all(body).unwrap();
        }

        fn answer_redirect(stream: &mut TcpStream, location: &str) {
            write!(
                stream,
                "HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            )
            .unwrap();
        }

        /// The asset this crate's own path names, for the tests' downloads.
        const RELEASE: ReleaseTag = ReleaseTag::new(Product::QuestTracker, 1, 2, 3);

        /// What a download handed back: its result, the sink's bytes and
        /// every progress report.
        struct Got {
            result: Result<u64, DownloadError>,
            sink: Vec<u8>,
            progress: Vec<(u64, Option<u64>)>,
        }

        fn fetch(target: &Target, asset: Asset, cap: u64, cancel: &AtomicBool) -> Got {
            let mut sink = Vec::new();
            let mut progress = Vec::new();
            let result = download(
                target,
                &RELEASE.download_path(asset),
                cap,
                &mut sink,
                &mut |done, total| progress.push((done, total)),
                cancel,
            );
            Got {
                result,
                sink,
                progress,
            }
        }

        fn payload(len: usize) -> Vec<u8> {
            (0..len).map(|i| (i % 251) as u8).collect()
        }

        #[test]
        fn loopback_get_sends_the_pinned_request_and_returns_the_answer() {
            for (product, agent) in [
                (Product::QuestTracker, QUEST_TRACKER_USER_AGENT),
                (Product::WaystoneRoller, WAYSTONE_ROLLER_USER_AGENT),
                (Product::CraftingHelper, CRAFTING_HELPER_USER_AGENT),
            ] {
                let (listener, port) = listen();
                let server = thread::spawn(move || {
                    let mut stream = accept(&listener);
                    let head = read_head(&mut stream);
                    let body = b"[{\"tag_name\":\"quest-tracker-v0.2.0\"}]";
                    write!(
                        stream,
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nSet-Cookie: kept=no\r\nConnection: close\r\n\r\n",
                        body.len()
                    )
                    .unwrap();
                    stream.write_all(body).unwrap();
                    head
                });

                let mut t = target(port);
                t.user_agent = product.user_agent();
                let answer = get(&t).expect("an answer");
                let head = server.join().unwrap();

                assert_eq!(answer.status, 200);
                assert_eq!(answer.body, b"[{\"tag_name\":\"quest-tracker-v0.2.0\"}]");
                assert_eq!(request_line(&head), format!("GET {PATH} HTTP/1.1"));
                assert_eq!(header(&head, "Accept"), vec![ACCEPT]);
                assert_eq!(header(&head, "X-GitHub-Api-Version"), vec![API_VERSION]);
                assert_eq!(header(&head, "User-Agent"), vec![agent]);
                assert_eq!(
                    header(&head, "Host"),
                    vec![format!("127.0.0.1:{port}").as_str()]
                );
                assert_sends_nothing_private(&head);
            }
        }

        #[test]
        fn loopback_cookie_set_by_one_answer_is_not_sent_by_the_next_fetch() {
            let (listener, port) = listen();
            let server = thread::spawn(move || {
                let mut heads = Vec::new();
                for _ in 0..2 {
                    let mut stream = accept(&listener);
                    heads.push(read_head(&mut stream));
                    write!(
                        stream,
                        "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nSet-Cookie: kept=yes; Path=/\r\nConnection: close\r\n\r\n[]"
                    )
                    .unwrap();
                }
                heads
            });

            let first = get(&target(port)).expect("the first answer");
            let second = get(&target(port)).expect("the second answer");
            let heads = server.join().unwrap();

            assert_eq!((first.status, second.status), (200, 200));
            // Both requests reached the server, the second after the first
            // answer set a cookie for this host and path.
            assert_eq!(heads.len(), 2);
            for head in &heads {
                assert!(head.starts_with(&format!("GET {PATH} ")), "{head}");
                assert!(header(head, "Cookie").is_empty(), "Cookie sent: {head}");
            }
        }

        #[test]
        fn loopback_silent_server_times_out() {
            let (listener, port) = listen();
            let (done, wait) = mpsc::channel::<()>();
            let server = thread::spawn(move || {
                let mut stream = accept(&listener);
                read_head(&mut stream);
                // Holds the connection open and says nothing until the
                // client has given up.
                let _ = wait.recv_timeout(Duration::from_secs(15));
                drop(stream);
            });

            let started = Instant::now();
            let result = get(&target(port));
            let took = started.elapsed();
            done.send(()).unwrap();
            server.join().unwrap();

            // The 1 s receive bound ends it, not the 8 s total; WinHTTP
            // enforces it in steps of about 4 s (measured).
            assert_eq!(result, Err(FetchError::Timeout));
            assert!(took < Duration::from_secs(6), "took {took:?}");
        }

        #[test]
        fn loopback_trickling_server_ends_at_the_total_deadline() {
            let (listener, port) = listen();
            let (done, wait) = mpsc::channel::<()>();
            let server = thread::spawn(move || {
                let mut stream = accept(&listener);
                read_head(&mut stream);
                let _ = stream.write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 100000\r\nConnection: close\r\n\r\n",
                );
                // One byte every 300 ms: each read is inside the receive
                // bound, so only the total deadline can end it.
                while wait.recv_timeout(Duration::from_millis(300)).is_err() {
                    if stream.write_all(b"x").is_err() {
                        break;
                    }
                }
            });

            let mut short = target(port);
            short.timeouts.total = Duration::from_secs(3);
            let started = Instant::now();
            let result = get(&short);
            let took = started.elapsed();
            let _ = done.send(());
            server.join().unwrap();

            assert_eq!(result, Err(FetchError::Timeout));
            assert!(took < Duration::from_secs(5), "took {took:?}");
        }

        #[test]
        fn loopback_answer_over_the_cap_is_refused() {
            let (listener, port) = listen();
            let server = thread::spawn(move || {
                let mut stream = accept(&listener);
                read_head(&mut stream);
                let len = MAX_BODY + 1;
                let _ = write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {len}\r\nConnection: close\r\n\r\n"
                );
                // The client may close as soon as it is past the cap.
                let _ = stream.write_all(&vec![b'a'; len]);
            });

            let result = get(&target(port));
            server.join().unwrap();

            assert_eq!(result, Err(FetchError::TooLarge));
        }

        #[test]
        fn loopback_answer_at_the_cap_is_read() {
            let (listener, port) = listen();
            let server = thread::spawn(move || {
                let mut stream = accept(&listener);
                read_head(&mut stream);
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {MAX_BODY}\r\nConnection: close\r\n\r\n"
                )
                .unwrap();
                stream.write_all(&vec![b'a'; MAX_BODY]).unwrap();
            });

            let answer = get(&target(port)).expect("an answer");
            server.join().unwrap();

            assert_eq!(answer.status, 200);
            assert_eq!(answer.body.len(), MAX_BODY);
        }

        #[test]
        fn loopback_redirect_is_not_followed() {
            let (listener, port) = listen();
            let server = thread::spawn(move || {
                let mut stream = accept(&listener);
                read_head(&mut stream);
                write!(
                    stream,
                    "HTTP/1.1 301 Moved Permanently\r\nLocation: http://127.0.0.1:{port}/followed\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                )
                .unwrap();
                drop(stream);
                listener
            });

            // The test target allows this very redirect for a download; the
            // release list still returns it as the answer.
            let answer = get(&target(port)).expect("the 3xx is the answer");
            let listener = server.join().unwrap();

            assert_eq!(answer.status, 301);
            assert!(answer.body.is_empty());
            assert_no_more_connections(&listener);
        }

        #[test]
        fn loopback_trade_leagues_sends_its_path_and_two_headers_only() {
            let (listener, port) = listen();
            let body: &[u8] = br#"{"result":[{"id":"Standard","realm":"poe2","text":"Standard"}]}"#;
            let server = thread::spawn(move || {
                let mut stream = accept(&listener);
                let head = read_head(&mut stream);
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .unwrap();
                stream.write_all(body).unwrap();
                head
            });

            let answer = get(&leagues_target(port)).expect("an answer");
            let head = server.join().unwrap();

            assert_eq!(answer.status, 200);
            assert_eq!(answer.body, body);
            assert_eq!(
                request_line(&head),
                format!("GET {TRADE_LEAGUES_PATH} HTTP/1.1")
            );
            assert_eq!(header(&head, "Accept"), vec![TRADE_LEAGUES_ACCEPT]);
            assert_eq!(
                header(&head, "User-Agent"),
                vec![CRAFTING_HELPER_USER_AGENT]
            );
            assert!(header(&head, "X-GitHub-Api-Version").is_empty(), "{head}");
            assert_sends_nothing_private(&head);
            // Besides the two, only what HTTP itself needs (WinHTTP's own
            // `Host` and `Connection`).
            let mut names: Vec<String> = head
                .split("\r\n")
                .skip(1)
                .filter_map(|line| line.split_once(':'))
                .map(|(n, _)| n.trim().to_ascii_lowercase())
                .filter(|n| n != "host" && n != "connection")
                .collect();
            names.sort();
            assert_eq!(names, ["accept", "user-agent"], "{head}");
        }

        #[test]
        fn loopback_trade_leagues_answer_over_its_cap_is_refused() {
            // One byte past the league list's own cap, far under the
            // release list's: refused.
            let (listener, port) = listen();
            let server = thread::spawn(move || {
                let mut stream = accept(&listener);
                read_head(&mut stream);
                let len = MAX_TRADE_LEAGUES_BODY + 1;
                let _ = write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {len}\r\nConnection: close\r\n\r\n"
                );
                let _ = stream.write_all(&vec![b'a'; len]);
            });
            let result = get(&leagues_target(port));
            server.join().unwrap();
            assert_eq!(result, Err(FetchError::TooLarge));
            assert!(MAX_TRADE_LEAGUES_BODY + 1 < MAX_BODY);

            // Control: an answer at the cap is read whole.
            let (listener, port) = listen();
            let server = thread::spawn(move || {
                let mut stream = accept(&listener);
                read_head(&mut stream);
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {MAX_TRADE_LEAGUES_BODY}\r\nConnection: close\r\n\r\n"
                )
                .unwrap();
                stream
                    .write_all(&vec![b'a'; MAX_TRADE_LEAGUES_BODY])
                    .unwrap();
            });
            let answer = get(&leagues_target(port)).expect("an answer");
            server.join().unwrap();
            assert_eq!(answer.status, 200);
            assert_eq!(answer.body.len(), MAX_TRADE_LEAGUES_BODY);
        }

        #[test]
        fn loopback_trade_leagues_redirect_is_not_followed() {
            let (listener, port) = listen();
            let server = thread::spawn(move || {
                let mut stream = accept(&listener);
                read_head(&mut stream);
                write!(
                    stream,
                    "HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:{port}/followed\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                )
                .unwrap();
                drop(stream);
                listener
            });

            // The test target allows this very redirect for a download; the
            // league list returns it as the answer.
            let answer = get(&leagues_target(port)).expect("the 3xx is the answer");
            let listener = server.join().unwrap();

            assert_eq!(answer.status, 302);
            assert!(answer.body.is_empty());
            assert_no_more_connections(&listener);
        }

        #[test]
        fn loopback_closed_port_is_unreachable() {
            let (listener, port) = listen();
            drop(listener);

            assert_eq!(get(&target(port)), Err(FetchError::Unreachable));
            let cancel = AtomicBool::new(false);
            let got = fetch(&target(port), Asset::Exe, MAX_EXE, &cancel);
            assert_eq!(
                got.result,
                Err(DownloadError::Fetch(FetchError::Unreachable))
            );
            assert!(got.sink.is_empty());
        }

        #[test]
        fn loopback_download_follows_one_redirect_to_an_allowed_host() {
            let body = payload(70_000);
            let served = body.clone();
            let (listener, port) = listen();
            let signed = "/asset/7?X-Amz-Signature=s3cr3t%2F&X-Amz-Expires=300";
            let server = thread::spawn(move || {
                let mut first = accept(&listener);
                let head1 = read_head(&mut first);
                answer_redirect(&mut first, &format!("http://127.0.0.1:{port}{signed}"));
                drop(first);
                let mut second = accept(&listener);
                let head2 = read_head(&mut second);
                answer_ok(&mut second, &served);
                drop(second);
                (head1, head2, listener)
            });

            let cancel = AtomicBool::new(false);
            let got = fetch(&target(port), Asset::Exe, MAX_EXE, &cancel);
            let (head1, head2, listener) = server.join().unwrap();

            assert_eq!(got.result, Ok(body.len() as u64));
            assert_eq!(got.sink, body);
            assert_eq!(
                request_line(&head1),
                "GET /S-Borkowski/ExilePact2/releases/download/quest-tracker-v1.2.3/quest-tracker.exe HTTP/1.1"
            );
            assert_eq!(request_line(&head2), format!("GET {signed} HTTP/1.1"));
            for head in [&head1, &head2] {
                assert_eq!(header(head, "Accept"), vec![DOWNLOAD_ACCEPT]);
                assert_eq!(header(head, "User-Agent"), vec![QUEST_TRACKER_USER_AGENT]);
                assert!(header(head, "X-GitHub-Api-Version").is_empty(), "{head}");
                assert_sends_nothing_private(head);
            }
            assert_no_more_connections(&listener);
        }

        #[test]
        fn loopback_download_refuses_a_redirect_to_another_host() {
            // The same server, but named by a host the allowlist lacks, then
            // by the allowed host on another port and over another scheme.
            for location in [
                "http://localhost:{port}/asset",
                "http://127.0.0.1:1/asset",
                "https://127.0.0.1:{port}/asset",
                "/asset",
            ] {
                let (listener, port) = listen();
                let location = location.replace("{port}", &port.to_string());
                let server = thread::spawn(move || {
                    let mut first = accept(&listener);
                    read_head(&mut first);
                    answer_redirect(&mut first, &location);
                    drop(first);
                    listener
                });

                let cancel = AtomicBool::new(false);
                let got = fetch(&target(port), Asset::Exe, MAX_EXE, &cancel);
                let listener = server.join().unwrap();

                assert_eq!(got.result, Err(DownloadError::RedirectRefused));
                assert!(got.sink.is_empty());
                assert!(got.progress.is_empty());
                assert_no_more_connections(&listener);
            }
        }

        #[test]
        fn loopback_download_refuses_a_second_redirect() {
            let (listener, port) = listen();
            let server = thread::spawn(move || {
                let mut first = accept(&listener);
                read_head(&mut first);
                answer_redirect(&mut first, &format!("http://127.0.0.1:{port}/one"));
                drop(first);
                let mut second = accept(&listener);
                let head2 = read_head(&mut second);
                answer_redirect(&mut second, &format!("http://127.0.0.1:{port}/two"));
                drop(second);
                (head2, listener)
            });

            let cancel = AtomicBool::new(false);
            let got = fetch(&target(port), Asset::Exe, MAX_EXE, &cancel);
            let (head2, listener) = server.join().unwrap();

            assert_eq!(got.result, Err(DownloadError::TooManyRedirects));
            assert!(request_line(&head2).starts_with("GET /one "), "{head2}");
            assert!(got.sink.is_empty());
            assert_no_more_connections(&listener);
        }

        #[test]
        fn loopback_download_reports_progress_and_streams_to_the_sink() {
            let body = payload(200_000);
            let served = body.clone();
            let (listener, port) = listen();
            let server = thread::spawn(move || {
                let mut stream = accept(&listener);
                let head = read_head(&mut stream);
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    served.len()
                )
                .unwrap();
                // Four pieces with a pause between, so the client reads in
                // more than one step.
                for piece in served.chunks(50_000) {
                    stream.write_all(piece).unwrap();
                    stream.flush().unwrap();
                    thread::sleep(Duration::from_millis(100));
                }
                head
            });

            let cancel = AtomicBool::new(false);
            let got = fetch(&target(port), Asset::Signature, MAX_EXE, &cancel);
            let head = server.join().unwrap();

            assert_eq!(got.result, Ok(body.len() as u64));
            assert_eq!(got.sink, body);
            assert_eq!(
                request_line(&head),
                "GET /S-Borkowski/ExilePact2/releases/download/quest-tracker-v1.2.3/quest-tracker.exe.minisig HTTP/1.1"
            );
            let total = Some(body.len() as u64);
            assert!(got.progress.len() >= 2, "{:?}", got.progress);
            assert_eq!(got.progress.last(), Some(&(body.len() as u64, total)));
            assert!(got.progress.iter().all(|&(_, t)| t == total));
            assert!(
                got.progress.windows(2).all(|w| w[0].0 < w[1].0),
                "{:?}",
                got.progress
            );
        }

        #[test]
        fn loopback_download_over_the_cap_is_refused() {
            const CAP: u64 = 10_000;
            // Announced past the cap: refused before a byte is written.
            // Not announced: refused when the cap is passed. At the cap: read.
            for (announce, len, refused) in [
                (true, CAP + 1, true),
                (false, CAP + 1, true),
                (true, CAP, false),
            ] {
                let (listener, port) = listen();
                let server = thread::spawn(move || {
                    let mut stream = accept(&listener);
                    read_head(&mut stream);
                    let length = if announce {
                        format!("Content-Length: {len}\r\n")
                    } else {
                        String::new()
                    };
                    let _ = write!(
                        stream,
                        "HTTP/1.1 200 OK\r\n{length}Connection: close\r\n\r\n"
                    );
                    // The client may close as soon as it is past the cap.
                    let _ = stream.write_all(&vec![b'a'; len as usize]);
                });

                let cancel = AtomicBool::new(false);
                let got = fetch(&target(port), Asset::Exe, CAP, &cancel);
                server.join().unwrap();

                if refused {
                    assert_eq!(
                        got.result,
                        Err(DownloadError::Fetch(FetchError::TooLarge)),
                        "announce {announce}"
                    );
                    assert!(got.sink.len() as u64 <= CAP);
                    if announce {
                        assert!(got.sink.is_empty(), "wrote before refusing");
                    }
                } else {
                    assert_eq!(got.result, Ok(CAP));
                    assert_eq!(got.sink.len() as u64, CAP);
                }
            }
        }

        #[test]
        fn loopback_download_stops_when_cancelled() {
            // Set before the download: nothing connects.
            let (listener, port) = listen();
            let cancel = AtomicBool::new(true);
            let got = fetch(&target(port), Asset::Exe, MAX_EXE, &cancel);
            assert_eq!(got.result, Err(DownloadError::Cancelled));
            assert!(got.sink.is_empty());
            assert_no_more_connections(&listener);

            // Set by the first progress report: the next read never comes,
            // although the server would keep sending.
            let (done, wait) = mpsc::channel::<()>();
            let server = thread::spawn(move || {
                let mut stream = accept(&listener);
                read_head(&mut stream);
                let _ = stream.write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 1000000\r\nConnection: close\r\n\r\n",
                );
                let _ = stream.write_all(&[b'a'; 1000]);
                while wait.recv_timeout(Duration::from_millis(200)).is_err() {
                    if stream.write_all(&[b'a'; 1000]).is_err() {
                        break;
                    }
                }
            });

            let cancel = AtomicBool::new(false);
            let mut sink = Vec::new();
            let mut reports = 0;
            let started = Instant::now();
            let result = download(
                &target(port),
                &RELEASE.download_path(Asset::Exe),
                MAX_EXE,
                &mut sink,
                &mut |_, _| {
                    reports += 1;
                    cancel.store(true, Ordering::SeqCst);
                },
                &cancel,
            );
            let took = started.elapsed();
            let _ = done.send(());
            server.join().unwrap();

            assert_eq!(result, Err(DownloadError::Cancelled));
            assert_eq!(reports, 1);
            assert!(!sink.is_empty() && sink.len() < 1_000_000);
            assert!(took < Duration::from_secs(3), "took {took:?}");
        }

        #[test]
        fn loopback_download_non_200_writes_nothing() {
            // Straight away, and on the redirect's target.
            for redirect_first in [false, true] {
                for status in [
                    "404 Not Found",
                    "500 Internal Server Error",
                    "304 Not Modified",
                ] {
                    let (listener, port) = listen();
                    let server = thread::spawn(move || {
                        let mut stream = accept(&listener);
                        read_head(&mut stream);
                        if redirect_first {
                            answer_redirect(&mut stream, &format!("http://127.0.0.1:{port}/asset"));
                            drop(stream);
                            stream = accept(&listener);
                            read_head(&mut stream);
                        }
                        let body = b"Not the asset";
                        let _ = write!(
                            stream,
                            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            body.len()
                        );
                        let _ = stream.write_all(body);
                    });

                    let cancel = AtomicBool::new(false);
                    let got = fetch(&target(port), Asset::Exe, MAX_EXE, &cancel);
                    server.join().unwrap();

                    let code: u16 = status[..3].parse().unwrap();
                    assert_eq!(got.result, Err(DownloadError::Status(code)), "{status}");
                    assert!(got.sink.is_empty(), "{status}: wrote {:?}", got.sink);
                    assert!(got.progress.is_empty(), "{status}");
                }
            }
        }

        #[test]
        fn loopback_download_short_answer_is_incomplete() {
            let (listener, port) = listen();
            let server = thread::spawn(move || {
                let mut stream = accept(&listener);
                read_head(&mut stream);
                let _ = stream.write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 5000\r\nConnection: close\r\n\r\n",
                );
                let _ = stream.write_all(&[b'a'; 1000]);
                // Closed before the announced length.
            });

            let cancel = AtomicBool::new(false);
            let got = fetch(&target(port), Asset::Exe, MAX_EXE, &cancel);
            server.join().unwrap();

            assert!(
                matches!(
                    got.result,
                    Err(DownloadError::Incomplete) | Err(DownloadError::Fetch(_))
                ),
                "{:?}",
                got.result
            );
            assert!(got.result.is_err());
        }

        #[test]
        fn loopback_download_writer_failure_is_reported() {
            struct Full;
            impl Write for Full {
                fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                    Err(std::io::Error::new(ErrorKind::StorageFull, "full"))
                }
                fn flush(&mut self) -> std::io::Result<()> {
                    Ok(())
                }
            }
            let (listener, port) = listen();
            let server = thread::spawn(move || {
                let mut stream = accept(&listener);
                read_head(&mut stream);
                let _ = write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: 10\r\nConnection: close\r\n\r\n0123456789"
                );
            });

            let cancel = AtomicBool::new(false);
            let result = download(
                &target(port),
                &RELEASE.download_path(Asset::Exe),
                MAX_EXE,
                &mut Full,
                &mut |_, _| {},
                &cancel,
            );
            server.join().unwrap();

            assert_eq!(result, Err(DownloadError::Write(ErrorKind::StorageFull)));
        }
    }
}
