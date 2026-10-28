//! Shared test support: a recording fake shell, a store in a temporary
//! folder and the fixture mod set.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use wr_app::{App, AppConfig, HotkeyError, Shell, Store, View};
use wr_regex::ModData;

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
    pub views: Vec<View>,
}

/// A shell whose hotkey registration behaves as Windows' does (a taken
/// combination is refused and the old one stays), with a clipboard and a
/// beep counter.
#[derive(Clone, Default)]
pub struct FakeShell {
    pub rec: Arc<Mutex<Recorded>>,
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

    pub fn rec(&self) -> std::sync::MutexGuard<'_, Recorded> {
        self.rec.lock().unwrap()
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
        r.writes.push(text.to_string());
        r.clipboard = Some(text.to_string());
        Ok(())
    }

    fn beep(&self) {
        self.rec.lock().unwrap().beeps += 1;
    }

    fn emit_view(&self, view: &View) {
        self.rec.lock().unwrap().views.push(view.clone());
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
