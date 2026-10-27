//! App-data store: directory resolution, atomic JSON writes of opaque
//! `serde_json::Value`s, quarantine of unreadable files and the rotating
//! diagnostic log. It knows nothing about quests.
//!
//! The folder holds:
//!
//! - `settings.json`, `position.json` (the tailer's path, fingerprint and
//!   offset), `characters.json` (name -> class, level, own or unconfirmed,
//!   pinned) and `characters/<safe name>.json` (one character's engine
//!   state), and `new-character.json` (the one provisional record of a new
//!   character not yet named, present only while there is one; clearing it
//!   removes the file). Each is written as `{"schema": 1, "data": <value>}`;
//!   what goes in `data` is the caller's.
//! - `overlay.log`, the diagnostic log, rotated at 1 MB into `overlay.log.1`.
//!
//! Every JSON write goes to `<file>.tmp` in the same folder, is synced, and
//! is renamed over the old file, so a failed write leaves the old file
//! whole. A file that cannot be read back (not JSON, or not the wrapper
//! above at this schema) is renamed `<file>.corrupt-<YYYYmmdd-HHMMSS>` (UTC),
//! the caller gets no value plus a [`Quarantined`] notice for its status
//! line, and the move is written to the diagnostic log.

mod diag;
mod dir;
mod stamp;

use std::fmt;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde_json::{json, Value};

pub use diag::{DiagLog, DIAG_ROTATE_BYTES};
pub use dir::{check_dir, default_dir, resolve_dir, Refusal, DIR_ENV};

/// The schema number every JSON file carries.
pub const SCHEMA: u64 = 1;

pub const SETTINGS_FILE: &str = "settings.json";
pub const POSITION_FILE: &str = "position.json";
pub const CHARACTERS_FILE: &str = "characters.json";
pub const CHARACTERS_DIR: &str = "characters";
pub const NEW_CHARACTER_FILE: &str = "new-character.json";
pub const DIAG_FILE: &str = "overlay.log";

/// What can go wrong opening or writing the store.
#[derive(Debug)]
pub enum StoreError {
    /// Neither `QUEST_TRACKER_DIR` nor `LOCALAPPDATA` is set.
    NoDirectory,
    /// The folder is one the store must not write to.
    Refused { dir: PathBuf, reason: Refusal },
    /// A character name that cannot name a file (empty).
    BadName(String),
    /// A filesystem operation failed on `path`.
    Io { path: PathBuf, source: io::Error },
}

impl StoreError {
    pub(crate) fn io(path: &Path, source: io::Error) -> Self {
        StoreError::Io {
            path: path.to_path_buf(),
            source,
        }
    }
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::NoDirectory => write!(
                f,
                "no store folder: neither {DIR_ENV} nor LOCALAPPDATA is set"
            ),
            StoreError::Refused { dir, reason } => {
                write!(f, "refusing store folder {}: {reason}", dir.display())
            }
            StoreError::BadName(name) => write!(f, "character name {name:?} cannot name a file"),
            StoreError::Io { path, source } => write!(f, "{}: {source}", path.display()),
        }
    }
}

impl std::error::Error for StoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            StoreError::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// A file that could not be read and was moved aside.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Quarantined {
    /// Where the file was.
    pub file: PathBuf,
    /// Where it is now.
    pub moved_to: PathBuf,
    /// Why it could not be read.
    pub reason: String,
}

impl Quarantined {
    /// One sentence for the overlay's status line.
    pub fn status_text(&self) -> String {
        format!(
            "{} could not be read ({}); it was moved to {} and defaults are in use",
            file_name(&self.file),
            self.reason,
            file_name(&self.moved_to)
        )
    }
}

/// The result of a load: the stored value, or `None` when the file is
/// missing or was quarantined (then `quarantined` says so).
#[derive(Debug, Clone, PartialEq)]
pub struct Loaded {
    pub value: Option<Value>,
    pub quarantined: Option<Quarantined>,
}

impl Loaded {
    /// The stored value, or `default` when there is none.
    pub fn value_or(self, default: Value) -> Value {
        self.value.unwrap_or(default)
    }
}

/// The app-data folder. Cheap to share between threads behind an `Arc`;
/// writes are serialized internally.
#[derive(Debug)]
pub struct Store {
    dir: PathBuf,
    diag: DiagLog,
    write_lock: Mutex<()>,
}

impl Store {
    /// A store in `dir`, after [`check_dir`]. Creates nothing: the folder is
    /// made on the first write.
    pub fn open(dir: impl Into<PathBuf>) -> Result<Self, StoreError> {
        let dir = dir.into();
        check_dir(&dir)?;
        Ok(Store {
            diag: DiagLog::new(dir.join(DIAG_FILE)),
            dir,
            write_lock: Mutex::new(()),
        })
    }

    /// A store in [`default_dir`].
    pub fn open_default() -> Result<Self, StoreError> {
        Self::open(default_dir()?)
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The diagnostic log, `overlay.log` in this folder.
    pub fn diag(&self) -> &DiagLog {
        &self.diag
    }

    pub fn load_settings(&self) -> Result<Loaded, StoreError> {
        self.load(&self.dir.join(SETTINGS_FILE))
    }

    pub fn save_settings(&self, value: &Value) -> Result<(), StoreError> {
        self.save(&self.dir.join(SETTINGS_FILE), value)
    }

    pub fn load_position(&self) -> Result<Loaded, StoreError> {
        self.load(&self.dir.join(POSITION_FILE))
    }

    pub fn save_position(&self, value: &Value) -> Result<(), StoreError> {
        self.save(&self.dir.join(POSITION_FILE), value)
    }

    pub fn load_characters(&self) -> Result<Loaded, StoreError> {
        self.load(&self.dir.join(CHARACTERS_FILE))
    }

    pub fn save_characters(&self, value: &Value) -> Result<(), StoreError> {
        self.save(&self.dir.join(CHARACTERS_FILE), value)
    }

    /// `characters/<safe name>.json` for `name`; see [`safe_name`].
    pub fn character_path(&self, name: &str) -> Result<PathBuf, StoreError> {
        Ok(self
            .dir
            .join(CHARACTERS_DIR)
            .join(format!("{}.json", safe_name(name)?)))
    }

    pub fn load_character(&self, name: &str) -> Result<Loaded, StoreError> {
        self.load(&self.character_path(name)?)
    }

    pub fn save_character(&self, name: &str, value: &Value) -> Result<(), StoreError> {
        self.save(&self.character_path(name)?, value)
    }

    /// `new-character.json`: the provisional record of a new character that
    /// no level-up has named yet. Kept apart from `characters.json` and the
    /// `characters/` folder, so it never names a character.
    pub fn new_character_path(&self) -> PathBuf {
        self.dir.join(NEW_CHARACTER_FILE)
    }

    /// The provisional record, or no value when there is none.
    pub fn load_new_character(&self) -> Result<Loaded, StoreError> {
        self.load(&self.new_character_path())
    }

    pub fn save_new_character(&self, value: &Value) -> Result<(), StoreError> {
        self.save(&self.new_character_path(), value)
    }

    /// Removes the provisional record. No record (or no folder yet) is not
    /// an error.
    pub fn clear_new_character(&self) -> Result<(), StoreError> {
        let path = self.new_character_path();
        let _guard = self.write_lock.lock().unwrap_or_else(|p| p.into_inner());
        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(StoreError::io(&path, e)),
        }
    }

    fn save(&self, path: &Path, value: &Value) -> Result<(), StoreError> {
        let _guard = self.write_lock.lock().unwrap_or_else(|p| p.into_inner());
        write_json_atomic(path, &json!({ "schema": SCHEMA, "data": value }))
    }

    fn load(&self, path: &Path) -> Result<Loaded, StoreError> {
        // Held across the read and any quarantine, so a concurrent save
        // cannot land in between and be moved aside as corrupt.
        let _guard = self.write_lock.lock().unwrap_or_else(|p| p.into_inner());
        let bytes = match fs::read(path) {
            Ok(b) => b,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                return Ok(Loaded {
                    value: None,
                    quarantined: None,
                })
            }
            Err(e) => return Err(StoreError::io(path, e)),
        };
        let reason = match serde_json::from_slice::<Value>(&bytes) {
            Ok(Value::Object(mut map)) => match map.get("schema").and_then(Value::as_u64) {
                Some(SCHEMA) if map.contains_key("data") => {
                    return Ok(Loaded {
                        value: map.remove("data"),
                        quarantined: None,
                    })
                }
                Some(SCHEMA) => "no data".to_string(),
                Some(n) => format!("schema {n}, this build reads {SCHEMA}"),
                None => "no schema number".to_string(),
            },
            Ok(_) => "not a JSON object".to_string(),
            Err(e) => format!("not valid JSON: {e}"),
        };

        let moved_to = quarantine(path)?;
        let q = Quarantined {
            file: path.to_path_buf(),
            moved_to,
            reason,
        };
        // The notice reaches the caller either way; a log that cannot be
        // written must not turn a recovered load into a failure.
        let _ = self.diag.write(&format!("store: {}", q.status_text()));
        Ok(Loaded {
            value: None,
            quarantined: Some(q),
        })
    }
}

/// Writes `value` to `path` atomically: `<file>.tmp` in the same folder,
/// written and synced, then renamed over `path`. Creates the folder. On any
/// failure the old `path` is untouched.
pub fn write_json_atomic(path: &Path, value: &Value) -> Result<(), StoreError> {
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty());
    if let Some(parent) = parent {
        fs::create_dir_all(parent).map_err(|e| StoreError::io(parent, e))?;
    }
    let mut tmp_name = path.file_name().unwrap_or_default().to_os_string();
    tmp_name.push(".tmp");
    let tmp = path.with_file_name(tmp_name);

    let mut bytes = serde_json::to_vec_pretty(value).map_err(|e| StoreError::io(path, e.into()))?;
    bytes.push(b'\n');
    let written = (|| -> io::Result<()> {
        let mut f = File::create(&tmp)?;
        f.write_all(&bytes)?;
        f.sync_all()
    })();
    if let Err(e) = written {
        if tmp.is_file() {
            let _ = fs::remove_file(&tmp);
        }
        return Err(StoreError::io(&tmp, e));
    }
    if let Err(e) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(StoreError::io(path, e));
    }
    Ok(())
}

/// Renames `path` to `<file>.corrupt-<YYYYmmdd-HHMMSS>`, adding `-2`, `-3`
/// and so on when that name is taken (two in the same second).
fn quarantine(path: &Path) -> Result<PathBuf, StoreError> {
    let base = format!(
        "{}.corrupt-{}",
        path.file_name().unwrap_or_default().to_string_lossy(),
        stamp::Utc::now().compact()
    );
    let mut target = path.with_file_name(&base);
    let mut n = 2;
    while target.exists() {
        target = path.with_file_name(format!("{base}-{n}"));
        n += 1;
    }
    fs::rename(path, &target).map_err(|e| StoreError::io(path, e))?;
    Ok(target)
}

/// The file stem for a character's name: ASCII lower-case letters, digits,
/// `_` and `-` are kept; every other byte of the UTF-8 name, upper-case
/// letters included, is written `%XX`. That keeps names apart that differ
/// only in case on a case-insensitive filesystem, and never yields a path
/// separator or a Windows device name (`con`, `nul`, `com1` and the like
/// get their first byte encoded). An empty name is refused.
pub fn safe_name(name: &str) -> Result<String, StoreError> {
    if name.is_empty() {
        return Err(StoreError::BadName(name.to_string()));
    }
    let mut out = String::with_capacity(name.len());
    for b in name.bytes() {
        if b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-' {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    if is_device_name(&out) {
        let first = out.as_bytes()[0];
        out.replace_range(0..1, &format!("%{first:02X}"));
    }
    Ok(out)
}

fn is_device_name(stem: &str) -> bool {
    const FIXED: [&str; 4] = ["con", "prn", "aux", "nul"];
    if FIXED.contains(&stem) {
        return true;
    }
    let b = stem.as_bytes();
    b.len() == 4 && (stem.starts_with("com") || stem.starts_with("lpt")) && b[3].is_ascii_digit()
}

fn file_name(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| p.display().to_string())
}
