//! The updater: the check's state machine and the in-app install, behind
//! one lock of their own (never an app's state lock, and never held across
//! a request or a disk write).
//!
//! A check is reached from two places only: the app's manual request
//! ([`Updater::begin_manual`]) and its worker's tick while the setting is on
//! and a check is due ([`Updater::begin_auto`]). When an automatic check is
//! due: when no check (manual or automatic) has started yet this run, when
//! the last one started [`CHECK_INTERVAL_MS`] ago, or when the last
//! automatic one failed [`RETRY_AFTER_FAILURE_MS`] ago. Times are the app's
//! clock; nothing about checks is persisted but the app's setting.
//!
//! An install is reached from one place only, [`Updater::install`], which
//! the app calls from its `install_update` command, never from a tick or the
//! tray. Checks and installs are single-flight together: neither starts
//! while the other runs. The install runs on a thread of its own, holding
//! no lock across network or disk work:
//!
//! 1. In the exe's folder, create `<exe>.download` new (an unwritable folder
//!    fails here, before any request).
//! 2. Download the `.minisig` (at most [`MAX_SIGNATURE`] bytes) and check its
//!    trusted comment names the release's exe and version; then download
//!    the exe into the staging file, at most and exactly the size the
//!    release lists.
//! 3. Close the file and verify its bytes, as they are on disk, against the
//!    key; any mismatch deletes the staging file and fails.
//! 4. Swap it in place of the running exe ([`crate::swap::swap`]); then the
//!    state is restarting and the app is asked to restart
//!    ([`InstallEvents::restart`]).
//!
//! A failure never touches the running exe and deletes the staging file. A
//! cancel ([`Updater::cancel_install`]) goes back to ready. Progress is
//! announced at most every [`PROGRESS_EVERY`], plus every state change, with
//! no lock held. Shutdown ([`Updater::stop`]) cancels a running install,
//! which then deletes the staging file and changes, announces and logs
//! nothing; the app does not wait for it.

use std::fs::{self, File, OpenOptions};
use std::io::{self, BufWriter};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

use crate::check::{
    download_reason, read_answer, Answer, AssetKind, DownloadError, Outcome, Release,
    TransportError, UpdateConfig, UpdateTransport, MAX_EXE, MAX_SIGNATURE,
};
use crate::release_key::RELEASE_PUBLIC_KEY;
use crate::signature::{read_signature, verify_file};
use crate::swap::{staging_path, swap};
use crate::version::{Product, Version};
use crate::view::{
    app_settings_view, install_failed_line, install_refused_line, install_started_line,
    install_view, replaced_line, signature_checked_line, AppSettingsView, InstallPhase,
    InstallView, INSTALL_CANCELLED_LINE,
};

/// How long after a check started the next automatic one is due.
pub const CHECK_INTERVAL_MS: u64 = 24 * 60 * 60 * 1_000;

/// How long after a failed automatic check the next one is due.
pub const RETRY_AFTER_FAILURE_MS: u64 = 60 * 60 * 1_000;

/// The least time between two progress announcements of a download.
pub const PROGRESS_EVERY: Duration = Duration::from_millis(250);

/// What [`Updater::begin_manual`] and [`Updater::begin_auto`] decided.
pub enum Begin {
    /// No check starts: one is running, an install is running, none is
    /// due, or the app is stopping.
    Skip,
    /// A check started; `None` when this build has no transport.
    Run(Option<Arc<dyn UpdateTransport>>),
}

/// What a running install tells the app. Called from the install's thread,
/// never with the updater's lock held.
pub trait InstallEvents: Send + Sync + 'static {
    /// What the view shows changed: send it.
    fn changed(&self);
    /// One `updates:` diagnostic line.
    fn diag(&self, line: &str);
    /// The new exe is in place: quit through the app's own quit path, which
    /// relaunches the exe once the app has shut down.
    fn restart(&self);
}

/// Why [`Updater::install`] started nothing. The view is unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallRefused {
    pub reason: String,
}

struct Core {
    outcome: Outcome,
    /// The newest release known newer than the running one, from any
    /// successful check this run.
    newer: Option<Release>,
    /// A check's thread is between its start and its end.
    in_flight: bool,
    /// When the last check, manual or automatic, started.
    last_start_ms: Option<u64>,
    /// When the last automatic check failed, while no check started since.
    auto_failed_ms: Option<u64>,
    phase: InstallPhase,
    /// The running install's cancel flag; `None` while none runs.
    installing: Option<Arc<AtomicBool>>,
    /// Raised by every change of what [`AppSettingsView`] shows, so an
    /// emitter can tell whether the view it sent is still the newest.
    seq: u64,
}

struct Shared {
    product: Product,
    transport: Option<Arc<dyn UpdateTransport>>,
    running: Version,
    exe_path: Option<PathBuf>,
    public_key: String,
    core: Mutex<Core>,
    /// Set when the app starts shutting down: no check or install starts,
    /// and one that ends after it changes nothing.
    stopped: AtomicBool,
}

/// The updater of one product. Cloning shares it.
#[derive(Clone)]
pub struct Updater {
    shared: Arc<Shared>,
}

/// One install, as its thread carries it out.
struct Job {
    release: Release,
    cancel: Arc<AtomicBool>,
}

/// How an install's steps ended short of the swap.
enum Stop {
    Cancelled,
    Failed(String),
}

impl Updater {
    /// The updater of `product`, whose running build is `own` unless the
    /// config names another (`Version::of_package(env!("CARGO_PKG_VERSION"))`
    /// in the app's own crate).
    pub fn new(product: Product, own: Version, config: UpdateConfig) -> Updater {
        Updater {
            shared: Arc::new(Shared {
                product,
                transport: config.transport.map(Arc::from),
                running: config.running.unwrap_or(own),
                exe_path: config.exe_path,
                public_key: config
                    .public_key
                    .unwrap_or_else(|| RELEASE_PUBLIC_KEY.to_string()),
                core: Mutex::new(Core {
                    outcome: Outcome::Idle,
                    newer: None,
                    in_flight: false,
                    last_start_ms: None,
                    auto_failed_ms: None,
                    phase: InstallPhase::Idle,
                    installing: None,
                    seq: 0,
                }),
                stopped: AtomicBool::new(false),
            }),
        }
    }

    fn lock(&self) -> MutexGuard<'_, Core> {
        self.shared.core.lock().unwrap_or_else(|p| p.into_inner())
    }

    pub fn product(&self) -> Product {
        self.shared.product
    }

    pub fn running(&self) -> Version {
        self.shared.running
    }

    /// The whole view, with the app's setting.
    pub fn view(&self, check_updates: bool) -> AppSettingsView {
        let core = self.lock();
        app_settings_view(
            &self.shared.product,
            self.shared.running,
            check_updates,
            &core.outcome,
            core.newer.as_ref(),
            &core.phase,
        )
    }

    /// The install part of the view alone.
    pub fn install_view(&self) -> InstallView {
        let core = self.lock();
        install_view(&self.shared.product, core.newer.as_ref(), &core.phase)
    }

    pub fn seq(&self) -> u64 {
        self.lock().seq
    }

    /// Something the view shows changed outside the updater (the setting).
    pub fn changed(&self) {
        self.lock().seq += 1;
    }

    /// The newer version known, if any.
    pub fn newer(&self) -> Option<Version> {
        self.lock().newer.as_ref().map(|r| r.version)
    }

    /// The newer release known, with its assets.
    pub fn newer_release(&self) -> Option<Release> {
        self.lock().newer.clone()
    }

    /// An install's thread is between its start and its end.
    pub fn is_installing(&self) -> bool {
        self.lock().installing.is_some()
    }

    /// The app is shutting down: nothing starts any more, a running check
    /// or install ends without changing anything, and a running install is
    /// cancelled. Returns at once.
    pub fn stop(&self) {
        self.shared.stopped.store(true, Ordering::Release);
        if let Some(cancel) = &self.lock().installing {
            cancel.store(true, Ordering::Release);
        }
    }

    pub fn is_stopped(&self) -> bool {
        self.shared.stopped.load(Ordering::Acquire)
    }

    // -- the check ------------------------------------------------------------

    /// A manual request: starts a check unless one, or an install, runs.
    pub fn begin_manual(&self, now_ms: u64) -> Begin {
        let mut core = self.lock();
        self.begin(&mut core, now_ms)
    }

    /// The worker's tick, with the setting `on`: starts a check when one is
    /// due and neither a check nor an install runs.
    pub fn begin_auto(&self, now_ms: u64, on: bool) -> Begin {
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
        if core.in_flight
            || core.installing.is_some()
            || core.phase == InstallPhase::Restarting
            || self.is_stopped()
        {
            return Begin::Skip;
        }
        core.in_flight = true;
        core.last_start_ms = Some(now_ms);
        core.auto_failed_ms = None;
        core.outcome = Outcome::Checking;
        core.seq += 1;
        Begin::Run(self.shared.transport.clone())
    }

    /// A check ended with `answer`; the outcome, or `None` when the app
    /// began shutting down first (nothing changes then). A failed install's
    /// reason gives way to what the check found.
    pub fn finish(
        &self,
        now_ms: u64,
        manual: bool,
        answer: Result<Answer, TransportError>,
    ) -> Option<Outcome> {
        let mut core = self.lock();
        if self.is_stopped() {
            return None;
        }
        let (outcome, release) = read_answer(&self.shared.product, answer, self.shared.running);
        match &outcome {
            Outcome::Available { .. } => core.newer = release,
            Outcome::UpToDate { .. } | Outcome::NoRelease => core.newer = None,
            Outcome::Failed { .. } if !manual => core.auto_failed_ms = Some(now_ms),
            _ => {}
        }
        if matches!(core.phase, InstallPhase::Failed { .. }) {
            core.phase = InstallPhase::Idle;
        }
        core.in_flight = false;
        core.outcome = outcome.clone();
        core.seq += 1;
        Some(outcome)
    }

    /// A check's request on a thread of its own, whose answer goes to
    /// `done`. The caller never waits for it.
    pub fn spawn(
        transport: Arc<dyn UpdateTransport>,
        done: impl FnOnce(Result<Answer, TransportError>) + Send + 'static,
    ) -> io::Result<()> {
        thread::Builder::new()
            .name("ep-update-check".to_string())
            .spawn(move || done(transport.fetch_releases()))
            .map(|_| ())
    }

    // -- the install ----------------------------------------------------------

    /// Starts installing the newer release: refused while a check or an
    /// install runs, while restarting, after shutdown began, with no newer
    /// version known, and for a release that is not signed. Otherwise the
    /// state is downloading and the install runs on a thread of its own;
    /// this returns at once. A failure from then on is the view's failed
    /// state, with its reason.
    pub fn install(&self, events: Arc<dyn InstallEvents>) -> Result<(), InstallRefused> {
        let started = {
            let mut core = self.lock();
            self.may_install(&core).map(|release| {
                let cancel = Arc::new(AtomicBool::new(false));
                core.phase = InstallPhase::Downloading {
                    bytes: 0,
                    total: release.exe.as_ref().map(|a| a.size),
                };
                core.installing = Some(Arc::clone(&cancel));
                core.seq += 1;
                Job { release, cancel }
            })
        };
        let job = match started {
            Ok(job) => job,
            Err(reason) => {
                events.diag(&install_refused_line(&reason));
                return Err(InstallRefused { reason });
            }
        };
        events.diag(&install_started_line(job.release.version));
        events.changed();
        let me = self.clone();
        let thread_events = Arc::clone(&events);
        let spawned = thread::Builder::new()
            .name("ep-update-install".to_string())
            .spawn(move || me.run_install(job, thread_events));
        if let Err(e) = spawned {
            self.end_install(
                &events,
                Err(Stop::Failed(format!(
                    "the install could not start ({})",
                    e.kind()
                ))),
            );
        }
        Ok(())
    }

    /// Asks a running install to stop; it then deletes its staging file and
    /// goes back to ready. `false` when none runs.
    pub fn cancel_install(&self) -> bool {
        match &self.lock().installing {
            Some(cancel) => {
                cancel.store(true, Ordering::Release);
                true
            }
            None => false,
        }
    }

    fn may_install(&self, core: &Core) -> Result<Release, String> {
        if self.is_stopped() {
            return Err("the app is closing".to_string());
        }
        if core.installing.is_some() {
            return Err("an install is running".to_string());
        }
        if core.phase == InstallPhase::Restarting {
            return Err("the app is restarting".to_string());
        }
        if core.in_flight {
            return Err("a check is running".to_string());
        }
        let Some(release) = &core.newer else {
            return Err("no newer version is known".to_string());
        };
        if !release.is_signed() {
            return Err("the release is not signed".to_string());
        }
        Ok(release.clone())
    }

    /// The install's thread.
    fn run_install(&self, job: Job, events: Arc<dyn InstallEvents>) {
        let result = self.install_steps(&job, &*events);
        self.end_install(&events, result);
    }

    /// Records how an install ended. On any end but success the staging
    /// file is deleted; after shutdown began nothing else happens.
    fn end_install(&self, events: &Arc<dyn InstallEvents>, result: Result<(), Stop>) {
        let Err(stop) = result else {
            return;
        };
        if let Some(exe) = &self.shared.exe_path {
            let staged = staging_path(exe);
            if let Err(e) = fs::remove_file(&staged) {
                if e.kind() != io::ErrorKind::NotFound && !self.is_stopped() {
                    events.diag(&format!(
                        "updates: the partial download could not be removed ({})",
                        e.kind()
                    ));
                }
            }
        }
        if self.is_stopped() {
            return;
        }
        let line = {
            let mut core = self.lock();
            core.installing = None;
            core.seq += 1;
            match stop {
                Stop::Cancelled => {
                    core.phase = InstallPhase::Idle;
                    INSTALL_CANCELLED_LINE.to_string()
                }
                Stop::Failed(reason) => {
                    let line = install_failed_line(&reason);
                    core.phase = InstallPhase::Failed { reason };
                    line
                }
            }
        };
        events.diag(&line);
        events.changed();
    }

    /// A step of a running install; after shutdown began, nothing changes.
    fn set_phase(&self, phase: InstallPhase) {
        let mut core = self.lock();
        if self.is_stopped() {
            return;
        }
        core.phase = phase;
        core.seq += 1;
    }

    fn install_steps(&self, job: &Job, events: &dyn InstallEvents) -> Result<(), Stop> {
        let failed = |reason: String| Stop::Failed(reason);
        let cancelled = || job.cancel.load(Ordering::Acquire) || self.is_stopped();
        let version = job.release.version;
        let (Some(exe_asset), Some(_)) = (&job.release.exe, &job.release.signature) else {
            return Err(failed("the release is not signed".to_string()));
        };
        let Some(exe) = self.shared.exe_path.as_deref() else {
            return Err(failed("this build cannot install updates".to_string()));
        };
        let Some(transport) = self.shared.transport.as_deref() else {
            return Err(failed("this build cannot download updates".to_string()));
        };
        if exe_asset.size == 0 || exe_asset.size > MAX_EXE {
            return Err(failed(format!(
                "the release's exe has an unexpected size ({} bytes)",
                exe_asset.size
            )));
        }

        // 1. The staging file, before any request.
        let staged = staging_path(exe);
        let file = create_new(&staged)
            .map_err(|e| failed(format!("the app's folder cannot be written ({})", e.kind())))?;

        // 2. The signature, then the exe.
        let mut sig = Vec::new();
        let got = transport.download(
            version,
            AssetKind::Signature,
            MAX_SIGNATURE,
            &mut sig,
            &mut |_, _| {},
            &job.cancel,
        );
        match got {
            Err(DownloadError::Cancelled) => return Err(Stop::Cancelled),
            Err(e) => return Err(failed(format!("the signature: {}", download_reason(e)))),
            Ok(_) if cancelled() => return Err(Stop::Cancelled),
            Ok(_) => {}
        }
        let text = String::from_utf8(sig)
            .map_err(|_| failed("the release's signature could not be read".to_string()))?;
        let signature = read_signature(&text, &exe_asset.name, &version.to_string())
            .map_err(|e| failed(e.reason()))?;

        let total = exe_asset.size;
        let mut sink = BufWriter::new(file);
        let mut throttle = Throttle::new(PROGRESS_EVERY);
        let got = transport.download(
            version,
            AssetKind::Exe,
            total,
            &mut sink,
            &mut |bytes, _| {
                self.set_phase(InstallPhase::Downloading {
                    bytes,
                    total: Some(total),
                });
                if throttle.due(Instant::now()) && !self.is_stopped() {
                    events.changed();
                }
            },
            &job.cancel,
        );
        let written = match got {
            Err(DownloadError::Cancelled) => return Err(Stop::Cancelled),
            Err(e) => return Err(failed(download_reason(e))),
            Ok(_) if cancelled() => return Err(Stop::Cancelled),
            Ok(n) => n,
        };
        let file = sink.into_inner().map_err(|e| {
            failed(format!(
                "the download could not be saved ({})",
                e.error().kind()
            ))
        })?;
        file.sync_all()
            .map_err(|e| failed(format!("the download could not be saved ({})", e.kind())))?;
        drop(file);
        if written != total {
            return Err(failed(format!(
                "the download's size does not match the release ({written} of {total} bytes)"
            )));
        }

        // 3. The bytes as they are on disk.
        self.set_phase(InstallPhase::Verifying);
        if !self.is_stopped() {
            events.changed();
        }
        verify_file(&staged, &signature, &self.shared.public_key)
            .map_err(|e| failed(e.reason()))?;
        if cancelled() {
            return Err(Stop::Cancelled);
        }
        events.diag(&signature_checked_line(version));

        // 4. The swap, then the restart.
        swap(exe, &staged).map_err(|e| failed(e.reason()))?;
        {
            let mut core = self.lock();
            core.phase = InstallPhase::Restarting;
            core.installing = None;
            core.seq += 1;
        }
        if self.is_stopped() {
            // The app is quitting anyway; the next start runs the new exe.
            return Ok(());
        }
        events.diag(&replaced_line(version));
        events.changed();
        events.restart();
        Ok(())
    }
}

/// Creates `path` new; a stale one (from an install that did not end) is
/// removed first.
fn create_new(path: &Path) -> io::Result<File> {
    let open = || OpenOptions::new().write(true).create_new(true).open(path);
    match open() {
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
            fs::remove_file(path)?;
            open()
        }
        other => other,
    }
}

/// At most one announcement per interval, the first one interval after the
/// start (the start is announced as a state change).
struct Throttle {
    every: Duration,
    last: Instant,
}

impl Throttle {
    fn new(every: Duration) -> Throttle {
        Throttle {
            every,
            last: Instant::now(),
        }
    }

    fn due(&mut self, now: Instant) -> bool {
        if now.saturating_duration_since(self.last) >= self.every {
            self.last = now;
            true
        } else {
            false
        }
    }
}
