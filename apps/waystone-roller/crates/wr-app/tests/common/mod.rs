//! Shared test support: a recording fake shell, a store in a temporary
//! folder and the fixture mod set.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use wr_app::{App, AppConfig, AppSettingsView, BeepAttempt, HotkeyError, Key, Shell, Store, View};
use wr_regex::ModData;

/// One thing the app asked of the shell's clipboard or keyboard side, in
/// the order it asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Call {
    /// A clipboard write that succeeded, with its text.
    Clipboard(String),
    /// The title of the window in front was read.
    Title,
    /// Whether any key is down was asked, and the answer.
    KeysDown(bool),
    /// One key event sent: the key, and `true` for down.
    Key(Key, bool),
    /// A key event whose send failed.
    KeyFailed(Key, bool),
    /// A pause the app asked for.
    Pause(Duration),
}

/// Runs inside the fake's `pause`, with no lock of the fake held.
pub type PauseHook = Box<dyn FnMut(Duration) + Send>;

/// What the fake shell saw and holds.
#[derive(Debug, Default)]
pub struct Recorded {
    /// Shortcuts another program holds: registering one is refused.
    pub taken: Vec<String>,
    /// The shortcut registered now.
    pub registered: Option<String>,
    /// Every registration asked for, in order.
    pub register_calls: Vec<String>,
    /// The clipboard's text.
    pub clipboard: Option<String>,
    /// Every clipboard write, in order.
    pub writes: Vec<String>,
    pub beeps: usize,
    /// The tone fails as Windows reports a failed call, and the shell falls
    /// back to a second sound (as the Tauri shell does).
    pub tone_fails: bool,
    pub views: Vec<View>,
    /// Every Settings window view sent, in order.
    pub app_settings: Vec<AppSettingsView>,
    /// How often the Settings window was asked to open.
    pub settings_opened: usize,
    /// How often the app asked to quit and start again.
    pub restarts: usize,
    /// A clipboard write fails with this text.
    pub clipboard_fails: Option<String>,
    /// The clipboard writes, title reads, key-state reads, key events and
    /// pauses, in order.
    pub calls: Vec<Call>,
    /// The title of the window in front; `None` is no window at all.
    pub title: Option<String>,
    /// How many key-state reads answer "a key is down" before the keys
    /// read as up (`usize::MAX`: held for ever).
    pub held_polls: usize,
    /// The key send with this index (0-based, counting every send asked
    /// for) fails with [`SEND_ERROR`].
    pub fail_send: Option<usize>,
    /// Key sends asked for so far.
    pub sends: usize,
}

/// What a failed key send says, as the glue reports a refused key event.
pub const SEND_ERROR: &str = "the key could not be sent (Access is denied. (os error 5))";

/// A shell whose hotkey registration behaves as Windows' does (a taken
/// combination is refused and the old one stays), with a clipboard, a beep
/// counter whose tone can be made to fail, and a keyboard side with a
/// scriptable window title, held keys and a failing send.
#[derive(Clone, Default)]
pub struct FakeShell {
    pub rec: Arc<Mutex<Recorded>>,
    /// Called from `pause` after it is recorded, with no lock held.
    pub on_pause: Arc<Mutex<Option<PauseHook>>>,
}

impl FakeShell {
    pub fn with_taken(taken: &[&str]) -> FakeShell {
        let shell = FakeShell::default();
        shell.rec.lock().unwrap().taken = taken.iter().map(|s| s.to_string()).collect();
        shell
    }

    pub fn with_clipboard(text: &str) -> FakeShell {
        let shell = FakeShell::default();
        shell.rec.lock().unwrap().clipboard = Some(text.to_string());
        shell
    }

    /// A shell with Path of Exile 2 in front.
    pub fn with_game() -> FakeShell {
        FakeShell::with_title(Some(wr_app::GAME_WINDOW_TITLE))
    }

    /// A shell with `title` in front (`None`: no window).
    pub fn with_title(title: Option<&str>) -> FakeShell {
        let shell = FakeShell::default();
        shell.rec.lock().unwrap().title = title.map(str::to_string);
        shell
    }

    pub fn rec(&self) -> std::sync::MutexGuard<'_, Recorded> {
        self.rec.lock().unwrap()
    }

    /// Every key event sent, in order: the key, and `true` for down.
    pub fn keys(&self) -> Vec<(Key, bool)> {
        self.rec()
            .calls
            .iter()
            .filter_map(|c| match c {
                Call::Key(k, down) => Some((*k, *down)),
                _ => None,
            })
            .collect()
    }

    /// Runs `hook` inside every later `pause`.
    pub fn on_pause(&self, hook: impl FnMut(Duration) + Send + 'static) {
        *self.on_pause.lock().unwrap() = Some(Box::new(hook));
    }
}

impl Shell for FakeShell {
    fn register_hotkey(&self, hotkey: &str) -> Result<(), HotkeyError> {
        let mut r = self.rec.lock().unwrap();
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

    fn write_clipboard(&self, text: &str) -> Result<(), String> {
        let mut r = self.rec.lock().unwrap();
        if let Some(e) = r.clipboard_fails.clone() {
            return Err(e);
        }
        r.writes.push(text.to_string());
        r.clipboard = Some(text.to_string());
        r.calls.push(Call::Clipboard(text.to_string()));
        Ok(())
    }

    fn foreground_title(&self) -> Option<String> {
        let mut r = self.rec.lock().unwrap();
        r.calls.push(Call::Title);
        r.title.clone()
    }

    fn any_key_down(&self) -> bool {
        let mut r = self.rec.lock().unwrap();
        let down = r.held_polls > 0;
        if down && r.held_polls != usize::MAX {
            r.held_polls -= 1;
        }
        r.calls.push(Call::KeysDown(down));
        down
    }

    fn send_key(&self, key: Key, down: bool) -> Result<(), String> {
        let mut r = self.rec.lock().unwrap();
        let index = r.sends;
        r.sends += 1;
        if r.fail_send == Some(index) {
            r.calls.push(Call::KeyFailed(key, down));
            return Err(SEND_ERROR.to_string());
        }
        r.calls.push(Call::Key(key, down));
        Ok(())
    }

    fn pause(&self, duration: Duration) {
        self.rec.lock().unwrap().calls.push(Call::Pause(duration));
        // The hook is taken out while it runs, so a press it makes from
        // another thread never waits on this lock.
        let hook = self.on_pause.lock().unwrap().take();
        if let Some(mut hook) = hook {
            hook(duration);
            let mut slot = self.on_pause.lock().unwrap();
            if slot.is_none() {
                *slot = Some(hook);
            }
        }
    }

    fn beep(&self) -> Vec<BeepAttempt> {
        let mut r = self.rec.lock().unwrap();
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
        self.rec.lock().unwrap().views.push(view.clone());
    }

    fn emit_app_settings(&self, view: &AppSettingsView) {
        self.rec.lock().unwrap().app_settings.push(view.clone());
    }

    fn open_settings(&self) -> Result<(), String> {
        self.rec.lock().unwrap().settings_opened += 1;
        Ok(())
    }

    fn restart(&self) {
        self.rec.lock().unwrap().restarts += 1;
    }
}

pub fn app_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn fixture_data() -> ModData {
    ModData::load(&app_root().join("fixtures").join("mods_min.json")).expect("mods_min.json loads")
}

/// A started app over `data`, with its store in `dir` and `shell`.
pub fn start_with(data: ModData, dir: &Path, shell: &FakeShell) -> Arc<App> {
    let app = App::new(AppConfig {
        data,
        store: Store::open(dir),
        shell: Box::new(shell.clone()),
    });
    app.start();
    app
}

/// A started app over the fixture mod set.
pub fn start(dir: &Path, shell: &FakeShell) -> Arc<App> {
    start_with(fixture_data(), dir, shell)
}

/// The lines of `dir`'s `diag.log`, without their time stamps.
pub fn diag_lines(dir: &Path) -> Vec<String> {
    let text = std::fs::read_to_string(dir.join("diag.log")).unwrap_or_default();
    text.lines()
        .map(|l| {
            // "YYYY-MM-DD HH:MM:SSZ message"
            l.splitn(3, ' ').nth(2).unwrap_or("").to_string()
        })
        .collect()
}

/// The fixture families that have a token, by id.
pub fn tokened_ids(app: &App) -> Vec<String> {
    app.get_view()
        .mods
        .iter()
        .filter(|m| m.token.is_some())
        .map(|m| m.id.clone())
        .collect()
}

/// Fills every `#` of a template line with a number and its roll range, as
/// the advanced item text shows it (`14(10-14)`; Live 1).
pub fn fill(line: &str, seed: usize) -> String {
    let mut out = String::new();
    let mut n = seed;
    for ch in line.chars() {
        if ch == '#' {
            let hi = 10 + n % 7;
            let lo = hi - 4;
            out.push_str(&format!("{}({lo}-{hi})", lo + n % 5));
            n += 3;
        } else {
            out.push(ch);
        }
    }
    out
}

/// A waystone carrying `lines` (templates) under one affix header (its name
/// invented), in the item layout measured in Live 1
/// (`wr_regex::ITEM_FRAME_LINES`).
pub fn waystone(lines: &[String], rarity: &str, tier: u8, corrupted: bool) -> String {
    let mut text = format!("Item Class: Waystones\nRarity: {rarity}\n");
    if rarity == "Rare" {
        text.push_str("Dusk Hollow\n");
    }
    text.push_str(&format!(
        "Waystone (Tier {tier})\n--------\nRevives Available: 2 (augmented)\nItem Rarity: +20% (augmented)\nPack Size: +5% (augmented)\nMonster Rarity: +18% (augmented)\n--------\nItem Level: {}\n--------\n",
        64 + tier as usize
    ));
    if !lines.is_empty() {
        text.push_str("{ Prefix Modifier \"Qorvex\" (Tier: 1) }\n");
    }
    for (i, line) in lines.iter().enumerate() {
        text.push_str(&fill(line, i));
        text.push('\n');
    }
    text.push_str("--------\nCan be used in a Map Device, allowing you to enter a Map. Waystones can only be used once.\n");
    if corrupted {
        text.push_str("--------\nCorrupted\n");
    }
    text
}
