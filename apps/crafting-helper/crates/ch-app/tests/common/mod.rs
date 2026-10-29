//! Shared test support: a recording fake shell, a store in a temporary
//! folder, and the update harness (a fake transport serving a release list
//! and the two release assets, each held on a gate when a test asks, a fake
//! clock and the running exe in a folder of its own).
//!
//! The signed release is the shared signature fixture
//! (`apps/shared/fixtures/signing/`): its exe asset is `payload.bin`, signed
//! for 1.3.0 with the test key, so the product here is the crafting helper
//! with that exe name, and the key is injected through `UpdateConfig`.

#![allow(dead_code)]

use std::collections::VecDeque;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use ch_app::{
    Answer, App, AppConfig, AssetKind, BeepAttempt, Clock, DownloadError, HotkeyError, Product,
    SettingsView, Shell, Store, TransportError, UpdateConfig, UpdateSetup, UpdateState,
    UpdateTransport, Version, View, CRAFTING_HELPER,
};
use serde_json::{json, Value};

// ---------------------------------------------------------------------------
// The fake shell.

/// What the fake shell saw and holds.
#[derive(Debug, Default)]
pub struct Recorded {
    /// Hotkeys another program holds: registering one is refused.
    pub taken: Vec<String>,
    /// The hotkey registered now.
    pub registered: Option<String>,
    /// Every registration asked for, in order.
    pub register_calls: Vec<String>,
    /// The clipboard's text; `None` is a clipboard with no text.
    pub clipboard: Option<String>,
    /// The clipboard cannot be read: another program holds it.
    pub clipboard_busy: bool,
    /// How often the app read the clipboard.
    pub clipboard_reads: usize,
    pub beeps: usize,
    /// The tone fails as Windows reports a failed call, and the shell falls
    /// back to a second sound (as the Tauri shell does).
    pub tone_fails: bool,
    pub views: Vec<View>,
    /// Every Settings window view sent, in order.
    pub app_settings: Vec<SettingsView>,
    /// Every show (`true`) and hide (`false`) of the overlay, in order.
    pub overlay: Vec<bool>,
    /// How often the Settings window was asked to open.
    pub settings_opened: usize,
    /// How often the app asked to quit and start again.
    pub restarts: usize,
    /// Every address the app asked the browser to open, in order.
    pub opened: Vec<String>,
    /// Opening the browser fails with this reason.
    pub open_fails: Option<String>,
}

/// What a busy clipboard's read says, as the glue reports it after its
/// retries.
pub const BUSY: &str = "another program holds the clipboard";

/// A shell whose hotkey registration behaves as Windows' does (a taken
/// combination is refused and the old one stays), with a clipboard that
/// counts its reads, a beep counter whose tone can be made to fail, and an
/// overlay that records each show and hide.
#[derive(Clone, Default)]
pub struct FakeShell {
    pub rec: Arc<Mutex<Recorded>>,
}

impl FakeShell {
    pub fn with_taken(taken: &[&str]) -> FakeShell {
        let shell = FakeShell::default();
        shell.rec().taken = taken.iter().map(|s| s.to_string()).collect();
        shell
    }

    pub fn with_clipboard(text: &str) -> FakeShell {
        let shell = FakeShell::default();
        shell.set_clipboard(Some(text));
        shell
    }

    pub fn set_clipboard(&self, text: Option<&str>) {
        self.rec().clipboard = text.map(str::to_string);
    }

    pub fn rec(&self) -> std::sync::MutexGuard<'_, Recorded> {
        self.rec.lock().unwrap()
    }

    pub fn reads(&self) -> usize {
        self.rec().clipboard_reads
    }

    pub fn last_view(&self) -> View {
        self.rec().views.last().cloned().expect("a view was sent")
    }
}

impl Shell for FakeShell {
    fn register_hotkey(&self, hotkey: &str) -> Result<(), HotkeyError> {
        let mut r = self.rec();
        r.register_calls.push(hotkey.to_string());
        if hotkey.trim().is_empty() || !hotkey.contains('+') {
            return Err(HotkeyError::Invalid(format!(
                "{hotkey:?} is not a shortcut"
            )));
        }
        if r.taken.iter().any(|t| t == hotkey) {
            return Err(HotkeyError::Taken(format!(
                "HotKey already registered: {hotkey}"
            )));
        }
        r.registered = Some(hotkey.to_string());
        Ok(())
    }

    fn read_clipboard(&self) -> Result<Option<String>, String> {
        let mut r = self.rec();
        r.clipboard_reads += 1;
        if r.clipboard_busy {
            return Err(BUSY.to_string());
        }
        Ok(r.clipboard.clone())
    }

    fn beep(&self) -> Vec<BeepAttempt> {
        let mut r = self.rec();
        r.beeps += 1;
        if r.tone_fails {
            vec![
                BeepAttempt::failed("tone", "no sound device (os error 1)"),
                BeepAttempt::ok("simple beep"),
            ]
        } else {
            vec![BeepAttempt::ok("tone")]
        }
    }

    fn emit_view(&self, view: &View) {
        self.rec().views.push(view.clone());
    }

    fn emit_app_settings(&self, view: &SettingsView) {
        self.rec().app_settings.push(view.clone());
    }

    fn set_overlay_visible(&self, visible: bool) {
        self.rec().overlay.push(visible);
    }

    fn open_settings(&self) -> Result<(), String> {
        self.rec().settings_opened += 1;
        Ok(())
    }

    fn restart(&self) {
        self.rec().restarts += 1;
    }

    fn open_in_browser(&self, address: &str) -> Result<(), String> {
        let mut r = self.rec();
        r.opened.push(address.to_string());
        match &r.open_fails {
            Some(why) => Err(why.clone()),
            None => Ok(()),
        }
    }
}

pub fn app_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// An app with its store in `dir` and `shell`, not started.
pub fn make(dir: &Path, shell: &FakeShell) -> Arc<App> {
    App::new(AppConfig {
        store: Store::open(dir),
        shell: Box::new(shell.clone()),
    })
}

/// A started app with its store in `dir` and `shell`.
pub fn start(dir: &Path, shell: &FakeShell) -> Arc<App> {
    let app = make(dir, shell);
    app.start();
    app
}

/// Writes `value` as the store's `settings.json`.
pub fn save_settings(dir: &Path, value: &Value) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("settings.json"),
        serde_json::to_vec(value).unwrap(),
    )
    .unwrap();
}

/// The store's `settings.json`.
pub fn saved_settings(dir: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(dir.join("settings.json")).unwrap()).unwrap()
}

/// The lines of `dir`'s `diag.log`, without their time stamps.
pub fn diag_lines(dir: &Path) -> Vec<String> {
    let text = fs::read_to_string(dir.join("diag.log")).unwrap_or_default();
    text.lines()
        .map(|l| {
            // "YYYY-MM-DD HH:MM:SSZ message"
            l.splitn(3, ' ').nth(2).unwrap_or("").to_string()
        })
        .collect()
}

/// An item's text as the game copies it with Ctrl+Alt+C, invented for the
/// tests (CRLF line breaks and a trailing blank line, as a clipboard may
/// hold it).
pub const ITEM_TEXT: &str = "Item Class: Gloves\r\nRarity: Rare\r\nGale Grip\r\nRiveted Mitts\r\n--------\r\nArmour: 52\r\n--------\r\nItem Level: 74\r\n--------\r\n+31 to maximum Life\r\n\r\n";
/// [`ITEM_TEXT`] as a grab keeps it.
pub const ITEM_TEXT_NORMALISED: &str = "Item Class: Gloves\nRarity: Rare\nGale Grip\nRiveted Mitts\n--------\nArmour: 52\n--------\nItem Level: 74\n--------\n+31 to maximum Life";
/// Text that is not an item.
pub const NOT_ITEM_TEXT: &str = "a shopping list\nmilk\neggs\n";

/// One state of the whittled ring `ch-engine`'s tests walk
/// (`crates/ch-engine/tests/items/gear/rings-resist-whittle/`): `01-start`
/// is the start, `02-chaos-fire` … `05-revealed-lightning` the hit path,
/// `06-chaos-below-floor`, `07-revealed-wrong` and `08-whittle-lightning`
/// the miss path's other states.
pub fn ring(name: &str) -> String {
    let path = app_root()
        .join("crates/ch-engine/tests/items/gear/rings-resist-whittle")
        .join(format!("{name}.txt"));
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The ring's targets: fire, cold and lightning resistance at T3 or better.
pub const RING_TARGETS: [&str; 3] = [
    "suffix-to-fire-resistance",
    "suffix-to-cold-resistance",
    "suffix-to-lightning-resistance",
];

/// [`RING_TARGETS`] at floor 3, as `set_targets` takes them.
pub fn ring_targets() -> Vec<ch_app::ch_engine::Target> {
    RING_TARGETS
        .iter()
        .map(|id| ch_app::ch_engine::Target::new(*id, Some(3)))
        .collect()
}

/// The blueprint the ring's walk follows.
pub const WHITTLE: &str = "rings-resist-whittle";

pub fn wait_until(what: &str, pred: impl Fn() -> bool) {
    let start = Instant::now();
    while !pred() {
        assert!(
            start.elapsed() < Duration::from_secs(10),
            "timed out waiting for {what}"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
}

pub fn lines(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

// ---------------------------------------------------------------------------
// The update harness.

pub const HOUR: u64 = 60 * 60 * 1_000;
pub const MINUTE: u64 = 60 * 1_000;
/// The running version every harness gives the app: synthetic, never the
/// crate's, so a release bump changes nothing here.
pub const RUNNING: Version = Version::new(1, 2, 0);
/// The version the fixture signature names.
pub const NEWER: Version = Version::new(1, 3, 0);
/// The crafting helper, with the exe asset the fixture signature names.
pub const PRODUCT: Product = Product {
    name: CRAFTING_HELPER.name,
    tag_prefix: CRAFTING_HELPER.tag_prefix,
    exe_name: "payload.bin",
};
/// The running exe's bytes, which only a finished install replaces.
pub const RUNNING_BYTES: &[u8] =
    b"the running crafting helper, which a failed install never touches";
/// The running exe's file name in the harness.
pub const EXE_NAME: &str = "crafting-helper.exe";

pub fn signing(name: &str) -> Vec<u8> {
    let p = app_root().join("../shared/fixtures/signing").join(name);
    fs::read(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

pub fn payload() -> Vec<u8> {
    signing("payload.bin")
}

pub fn test_key() -> String {
    String::from_utf8(signing("test-key.pub"))
        .unwrap()
        .lines()
        .nth(1)
        .unwrap()
        .to_string()
}

pub type Reply = Result<Answer, TransportError>;

/// A 200 release list of `versions`; the newest lists the payload and its
/// signature when `signed`, the payload alone otherwise.
pub fn list(versions: &[Version], signed: bool) -> Reply {
    let newest = versions.iter().max().copied();
    let entries: Vec<Value> = versions
        .iter()
        .map(|v| {
            let mut assets = vec![json!({
                "name": PRODUCT.exe_name, "size": payload().len(), "state": "uploaded",
                "browser_download_url": "https://example.invalid/never-read",
            })];
            if signed {
                assets.push(json!({
                    "name": PRODUCT.signature_name(),
                    "size": signing("payload.bin.minisig").len(),
                    "state": "uploaded",
                }));
            }
            json!({
                "tag_name": PRODUCT.tag(*v),
                "draft": false,
                "prerelease": false,
                "assets": if Some(*v) == newest { assets } else { Vec::new() },
            })
        })
        .collect();
    Ok(Answer {
        status: 200,
        body: serde_json::to_vec(&entries).unwrap(),
    })
}

pub fn signed_newer() -> Reply {
    list(&[NEWER, RUNNING], true)
}

pub fn status(code: u16) -> Reply {
    Ok(Answer {
        status: code,
        body: br#"{"message":"a body that is never shown"}"#.to_vec(),
    })
}

#[derive(Default)]
pub struct Gates {
    /// The release list's request waits while this is true.
    pub list_held: bool,
    /// Each download waits, before it writes anything, while this is true.
    pub downloads_held: bool,
}

/// The transport's shared record and script.
pub struct Wire {
    pub list_calls: AtomicUsize,
    pub list_returned: AtomicUsize,
    pub replies: Mutex<VecDeque<Reply>>,
    pub fallback: Mutex<Reply>,
    /// Every download asked for, in order.
    pub downloads: Mutex<Vec<AssetKind>>,
    pub sig: Mutex<Vec<u8>>,
    pub exe: Mutex<Vec<u8>>,
    /// A download is waiting at the held gate.
    pub download_waiting: AtomicBool,
    pub gates: Mutex<Gates>,
    pub turned: Condvar,
}

impl Wire {
    pub fn new() -> Arc<Wire> {
        Arc::new(Wire {
            list_calls: AtomicUsize::new(0),
            list_returned: AtomicUsize::new(0),
            replies: Mutex::new(VecDeque::new()),
            fallback: Mutex::new(list(&[RUNNING], true)),
            downloads: Mutex::new(Vec::new()),
            sig: Mutex::new(signing("payload.bin.minisig")),
            exe: Mutex::new(payload()),
            download_waiting: AtomicBool::new(false),
            gates: Mutex::new(Gates::default()),
            turned: Condvar::new(),
        })
    }

    pub fn gate(&self, f: impl FnOnce(&mut Gates)) {
        f(&mut self.gates.lock().unwrap());
        self.turned.notify_all();
    }
}

pub struct FakeTransport(pub Arc<Wire>);

impl UpdateTransport for FakeTransport {
    fn fetch_releases(&self) -> Result<Answer, TransportError> {
        let w = &self.0;
        w.list_calls.fetch_add(1, Ordering::SeqCst);
        let mut g = w.gates.lock().unwrap();
        while g.list_held {
            g = w.turned.wait(g).unwrap();
        }
        drop(g);
        let reply = w
            .replies
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| w.fallback.lock().unwrap().clone());
        w.list_returned.fetch_add(1, Ordering::SeqCst);
        reply
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
        let w = &self.0;
        assert_eq!(version, NEWER, "a download of another version");
        w.downloads.lock().unwrap().push(asset);
        let mut g = w.gates.lock().unwrap();
        while g.downloads_held {
            w.download_waiting.store(true, Ordering::SeqCst);
            if cancel.load(Ordering::SeqCst) {
                w.download_waiting.store(false, Ordering::SeqCst);
                return Err(DownloadError::Cancelled);
            }
            g = w
                .turned
                .wait_timeout(g, Duration::from_millis(10))
                .unwrap()
                .0;
        }
        w.download_waiting.store(false, Ordering::SeqCst);
        drop(g);
        let data = match asset {
            AssetKind::Signature => w.sig.lock().unwrap().clone(),
            AssetKind::Exe => w.exe.lock().unwrap().clone(),
        };
        let total = Some(data.len() as u64);
        let mut written = 0u64;
        for chunk in data.chunks(8 * 1024) {
            if cancel.load(Ordering::SeqCst) {
                return Err(DownloadError::Cancelled);
            }
            if written + chunk.len() as u64 > limit {
                return Err(DownloadError::Transport(TransportError::TooLarge));
            }
            sink.write_all(chunk)
                .map_err(|e| DownloadError::Write(e.kind()))?;
            written += chunk.len() as u64;
            progress(written, total);
        }
        Ok(written)
    }
}

pub struct FakeClock(pub Arc<AtomicU64>);

impl Clock for FakeClock {
    fn now_ms(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}

#[derive(Default)]
pub struct Setup {
    /// The `settings.json` the store holds when the app is made.
    pub saved: Option<Value>,
    /// The list's request starts held.
    pub list_held: bool,
    /// The fake shell to use; a default one when `None`.
    pub shell: Option<FakeShell>,
}

pub fn auto_on() -> Setup {
    Setup {
        saved: Some(json!({ "schema": 1, "hotkey": "Ctrl+Alt+X", "check_updates": true })),
        ..Setup::default()
    }
}

pub struct Harness {
    pub _tmp: tempfile::TempDir,
    pub store: PathBuf,
    pub exe: PathBuf,
    pub shell: FakeShell,
    pub wire: Arc<Wire>,
    pub now: Arc<AtomicU64>,
    pub app: Arc<App>,
}

/// An app, not started, with the fake transport, the fake clock at 0, the
/// test key, the running exe in a folder of its own and no ticker thread:
/// the tests tick it themselves.
pub fn harness(setup: Setup) -> Harness {
    let tmp = tempfile::tempdir().unwrap();
    let store = tmp.path().join("store");
    let bin = tmp.path().join("bin");
    fs::create_dir_all(&store).unwrap();
    fs::create_dir_all(&bin).unwrap();
    if let Some(saved) = &setup.saved {
        save_settings(&store, saved);
    }
    let exe = bin.join(EXE_NAME);
    fs::write(&exe, RUNNING_BYTES).unwrap();
    let wire = Wire::new();
    wire.gate(|g| g.list_held = setup.list_held);
    let now = Arc::new(AtomicU64::new(0));
    let shell = setup.shell.unwrap_or_default();
    let app = App::with_updates(
        AppConfig {
            store: Store::open(&store),
            shell: Box::new(shell.clone()),
        },
        UpdateSetup {
            config: UpdateConfig {
                transport: Some(Box::new(FakeTransport(Arc::clone(&wire)))),
                running: Some(RUNNING),
                exe_path: Some(exe.clone()),
                public_key: Some(test_key()),
            },
            product: PRODUCT,
            clock: Box::new(FakeClock(Arc::clone(&now))),
            tick_every: None,
        },
    );
    Harness {
        _tmp: tmp,
        store,
        exe,
        shell,
        wire,
        now,
        app,
    }
}

impl Harness {
    pub fn calls(&self) -> usize {
        self.wire.list_calls.load(Ordering::SeqCst)
    }

    pub fn downloads(&self) -> Vec<AssetKind> {
        self.wire.downloads.lock().unwrap().clone()
    }

    pub fn reply(&self, reply: Reply) {
        self.wire.replies.lock().unwrap().push_back(reply);
    }

    pub fn hold_list(&self, held: bool) {
        self.wire.gate(|g| g.list_held = held);
    }

    pub fn hold_downloads(&self, held: bool) {
        self.wire.gate(|g| g.downloads_held = held);
    }

    /// One tick of the ticker at `t`, with the clock at `t`.
    pub fn at(&self, t: u64) {
        self.now.store(t, Ordering::SeqCst);
        self.app.tick_updates(t);
    }

    /// Ticks every ten minutes from `from` to `to`, both included.
    pub fn ticks(&self, from: u64, to: u64) {
        let mut t = from;
        while t <= to {
            self.at(t);
            t += 10 * MINUTE;
        }
        self.at(to);
    }

    pub fn wait_calls(&self, n: usize) {
        wait_until(&format!("{n} release list calls"), || self.calls() >= n);
    }

    /// Waits until every started request returned and its check ended: the
    /// last view sent is past checking. The app logs the outcome before it
    /// sends that view, and sends a checking view before any request.
    pub fn wait_ended(&self) -> SettingsView {
        wait_until("the check to end", || {
            self.wire.list_returned.load(Ordering::SeqCst) == self.calls()
                && self.app.get_app_settings().updates.update.state != UpdateState::Checking
                && self
                    .emitted()
                    .last()
                    .is_some_and(|v| v.updates.update.state != UpdateState::Checking)
        });
        self.app.get_app_settings()
    }

    /// A manual check answered with `reply`, waited for.
    pub fn manual(&self, reply: Reply) -> SettingsView {
        self.reply(reply);
        let before = self.calls();
        self.app.check_for_updates();
        self.wait_calls(before + 1);
        self.wait_ended()
    }

    pub fn update_lines(&self) -> Vec<String> {
        diag_lines(&self.store)
            .into_iter()
            .filter(|l| l.starts_with("updates:"))
            .collect()
    }

    pub fn emitted(&self) -> Vec<SettingsView> {
        self.shell.rec().app_settings.clone()
    }

    pub fn main_views(&self) -> Vec<View> {
        self.shell.rec().views.clone()
    }

    pub fn restarts(&self) -> usize {
        self.shell.rec().restarts
    }

    pub fn saved_settings(&self) -> Value {
        saved_settings(&self.store)
    }

    pub fn exe_bytes(&self) -> Vec<u8> {
        fs::read(&self.exe).unwrap()
    }

    pub fn folder(&self) -> Vec<String> {
        let mut v: Vec<String> = fs::read_dir(self.exe.parent().unwrap())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        v.sort();
        v
    }
}
