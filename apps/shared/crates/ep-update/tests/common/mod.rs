//! What the `ep-update` tests share: the signature fixtures, a temporary
//! folder, a fake transport that serves a release list and the two assets
//! the way `ep-net` does (in chunks, honouring the limit and the cancel
//! flag), and a recorder of what an install tells the app.

#![allow(dead_code)]

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant};

use ep_update::{
    Answer, AssetKind, DownloadError, InstallEvents, InstallState, Product, TransportError,
    UpdateConfig, UpdateTransport, Updater, Version,
};
use serde_json::{json, Value};

/// The product the fixtures were signed for: its exe asset is the payload.
pub const FIXTURE: Product = Product {
    name: "Fixture App",
    tag_prefix: "fixture-app-v",
    exe_name: "payload.bin",
};

/// The version the good fixture signature names, and the one before it.
pub const NEWER: Version = Version::new(1, 3, 0);
pub const RUNNING: Version = Version::new(1, 2, 0);

/// The bytes of the exe the tests' running app has.
pub const RUNNING_BYTES: &[u8] = b"the running exe, which a failed install must never touch";

pub fn fixture(name: &str) -> Vec<u8> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/signing")
        .join(name);
    fs::read(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

pub fn fixture_text(name: &str) -> String {
    String::from_utf8(fixture(name)).unwrap()
}

pub fn payload() -> Vec<u8> {
    fixture("payload.bin")
}

pub fn good_signature() -> Vec<u8> {
    fixture("payload.bin.minisig")
}

/// The test public key's `RW...` line.
pub fn test_key() -> String {
    fixture_text("test-key.pub")
        .lines()
        .nth(1)
        .unwrap()
        .to_string()
}

/// A folder under the system's temporary folder, removed when dropped.
pub struct TempDir(pub PathBuf);

impl TempDir {
    pub fn new() -> TempDir {
        static N: AtomicUsize = AtomicUsize::new(0);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let p = std::env::temp_dir().join(format!(
            "ep-update-test-{}-{}-{nanos}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&p).unwrap();
        TempDir(p)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Every file name in `dir`, sorted.
pub fn names(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

/// One release entry as GitHub lists it; `assets` are `(name, size)`.
pub fn release_entry(product: &Product, v: Version, assets: &[(&str, u64)]) -> Value {
    json!({
        "tag_name": product.tag(v),
        "draft": false,
        "prerelease": false,
        "html_url": "https://example.invalid/never-read",
        "assets": assets.iter().map(|(name, size)| json!({
            "name": name,
            "size": size,
            "state": "uploaded",
            "browser_download_url": "https://example.invalid/never-read",
        })).collect::<Vec<_>>(),
    })
}

/// A release list of `NEWER` of the fixture product, with the payload as its
/// exe and its signature.
pub fn signed_list() -> Vec<u8> {
    let entry = release_entry(
        &FIXTURE,
        NEWER,
        &[
            ("payload.bin", payload().len() as u64),
            ("payload.bin.minisig", good_signature().len() as u64),
        ],
    );
    serde_json::to_vec(&json!([entry])).unwrap()
}

pub fn ok(body: Vec<u8>) -> Result<Answer, TransportError> {
    Ok(Answer { status: 200, body })
}

/// The fake transport's shared record and script.
pub struct Fake {
    pub list_calls: AtomicUsize,
    pub list: Mutex<Vec<u8>>,
    /// Every download asked for, in order.
    pub downloads: Mutex<Vec<(Version, AssetKind, u64)>>,
    pub exe: Mutex<Vec<u8>>,
    pub sig: Mutex<Vec<u8>>,
    pub exe_error: Mutex<Option<DownloadError>>,
    pub sig_error: Mutex<Option<DownloadError>>,
    /// Bytes per read of the exe.
    pub chunk: Mutex<usize>,
    /// A wait between two reads of the exe.
    pub pause: Mutex<Duration>,
    /// While closed, the exe's download waits after its first read until it
    /// opens or the cancel flag is set.
    pub open: Mutex<bool>,
    pub opened: Condvar,
    /// The exe's download reached the closed gate.
    pub blocked: AtomicBool,
    /// Downloads that returned.
    pub returned: AtomicUsize,
}

impl Fake {
    pub fn new() -> Arc<Fake> {
        Arc::new(Fake {
            list_calls: AtomicUsize::new(0),
            list: Mutex::new(signed_list()),
            downloads: Mutex::new(Vec::new()),
            exe: Mutex::new(payload()),
            sig: Mutex::new(good_signature()),
            exe_error: Mutex::new(None),
            sig_error: Mutex::new(None),
            chunk: Mutex::new(8 * 1024),
            pause: Mutex::new(Duration::ZERO),
            open: Mutex::new(true),
            opened: Condvar::new(),
            blocked: AtomicBool::new(false),
            returned: AtomicUsize::new(0),
        })
    }

    pub fn set_open(&self, open: bool) {
        *self.open.lock().unwrap() = open;
        self.opened.notify_all();
    }

    pub fn download_count(&self) -> usize {
        self.downloads.lock().unwrap().len()
    }

    #[allow(clippy::too_many_arguments)]
    fn serve(
        &self,
        data: &[u8],
        chunk: usize,
        gated: bool,
        limit: u64,
        sink: &mut dyn Write,
        progress: &mut dyn FnMut(u64, Option<u64>),
        cancel: &AtomicBool,
    ) -> Result<u64, DownloadError> {
        let mut written = 0u64;
        let total = Some(data.len() as u64);
        for (i, piece) in data.chunks(chunk.max(1)).enumerate() {
            if cancel.load(Ordering::Acquire) {
                return Err(DownloadError::Cancelled);
            }
            if written + piece.len() as u64 > limit {
                return Err(DownloadError::Transport(TransportError::TooLarge));
            }
            sink.write_all(piece)
                .map_err(|e| DownloadError::Write(e.kind()))?;
            written += piece.len() as u64;
            progress(written, total);
            if gated && i == 0 {
                sink.flush().map_err(|e| DownloadError::Write(e.kind()))?;
                let mut open = self.open.lock().unwrap();
                while !*open {
                    self.blocked.store(true, Ordering::SeqCst);
                    if cancel.load(Ordering::Acquire) {
                        return Err(DownloadError::Cancelled);
                    }
                    open = self
                        .opened
                        .wait_timeout(open, Duration::from_millis(5))
                        .unwrap()
                        .0;
                }
            }
            let pause = *self.pause.lock().unwrap();
            if !pause.is_zero() {
                std::thread::sleep(pause);
            }
        }
        Ok(written)
    }
}

pub struct FakeTransport(pub Arc<Fake>);

impl UpdateTransport for FakeTransport {
    fn fetch_releases(&self) -> Result<Answer, TransportError> {
        self.0.list_calls.fetch_add(1, Ordering::SeqCst);
        ok(self.0.list.lock().unwrap().clone())
    }

    fn download(
        &self,
        version: Version,
        asset: AssetKind,
        limit: u64,
        sink: &mut dyn Write,
        progress: &mut dyn FnMut(u64, Option<u64>),
        cancel: &AtomicBool,
    ) -> Result<u64, DownloadError> {
        let f = &self.0;
        f.downloads.lock().unwrap().push((version, asset, limit));
        let result = match asset {
            AssetKind::Signature => match *f.sig_error.lock().unwrap() {
                Some(e) => Err(e),
                None => {
                    let data = f.sig.lock().unwrap().clone();
                    f.serve(&data, data.len(), false, limit, sink, progress, cancel)
                }
            },
            AssetKind::Exe => match *f.exe_error.lock().unwrap() {
                Some(e) => Err(e),
                None => {
                    let data = f.exe.lock().unwrap().clone();
                    let chunk = *f.chunk.lock().unwrap();
                    f.serve(&data, chunk, true, limit, sink, progress, cancel)
                }
            },
        };
        f.returned.fetch_add(1, Ordering::SeqCst);
        result
    }
}

/// What an install told the app, and the install state the view showed at
/// each announcement.
#[derive(Default)]
pub struct Rec {
    pub updater: OnceLock<Updater>,
    pub changed: AtomicUsize,
    pub states: Mutex<Vec<(InstallState, Option<u64>)>>,
    pub lines: Mutex<Vec<String>>,
    pub restarts: AtomicUsize,
}

impl InstallEvents for Rec {
    fn changed(&self) {
        self.changed.fetch_add(1, Ordering::SeqCst);
        if let Some(u) = self.updater.get() {
            let v = u.install_view();
            self.states
                .lock()
                .unwrap()
                .push((v.state, v.progress.map(|p| p.bytes)));
        }
    }

    fn diag(&self, line: &str) {
        self.lines.lock().unwrap().push(line.to_string());
    }

    fn restart(&self) {
        self.restarts.fetch_add(1, Ordering::SeqCst);
    }
}

impl Rec {
    pub fn lines(&self) -> Vec<String> {
        self.lines.lock().unwrap().clone()
    }

    pub fn states(&self) -> Vec<InstallState> {
        self.states.lock().unwrap().iter().map(|s| s.0).collect()
    }
}

/// An app's updater with the fake transport, the test key and a running exe
/// in a temporary folder.
pub struct Setup {
    pub dir: TempDir,
    pub exe: PathBuf,
    pub fake: Arc<Fake>,
    pub updater: Updater,
    pub rec: Arc<Rec>,
}

impl Setup {
    /// Nothing checked yet.
    pub fn new() -> Setup {
        let dir = TempDir::new();
        let exe = dir.path().join("app.exe");
        fs::write(&exe, RUNNING_BYTES).unwrap();
        Setup::with_exe(dir, exe)
    }

    pub fn with_exe(dir: TempDir, exe: PathBuf) -> Setup {
        let fake = Fake::new();
        let updater = Updater::new(
            FIXTURE,
            Version::new(0, 0, 1),
            UpdateConfig {
                transport: Some(Box::new(FakeTransport(Arc::clone(&fake)))),
                running: Some(RUNNING),
                exe_path: Some(exe.clone()),
                public_key: Some(test_key()),
            },
        );
        let rec = Arc::new(Rec::default());
        let _ = rec.updater.set(updater.clone());
        Setup {
            dir,
            exe,
            fake,
            updater,
            rec,
        }
    }

    /// A manual check whose answer is the fake's release list.
    pub fn check(&self) {
        assert!(matches!(
            self.updater.begin_manual(0),
            ep_update::Begin::Run(Some(_))
        ));
        let list = self.fake.list.lock().unwrap().clone();
        self.updater.finish(0, true, ok(list)).unwrap();
    }

    pub fn events(&self) -> Arc<dyn InstallEvents> {
        Arc::clone(&self.rec) as Arc<dyn InstallEvents>
    }

    pub fn install(&self) -> Result<(), ep_update::InstallRefused> {
        self.updater.install(self.events())
    }

    /// Waits for the running install to end.
    pub fn wait_installed(&self) {
        wait_until("the install to end", || !self.updater.is_installing());
    }

    pub fn exe_bytes(&self) -> Vec<u8> {
        fs::read(&self.exe).unwrap()
    }
}

/// Polls `cond` for up to ten seconds.
pub fn wait_until(what: &str, mut cond: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !cond() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(2));
    }
}
