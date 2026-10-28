//! The store folder and what it holds: `settings.json` (the shortcut and
//! the selection), written atomically, and `diag.log`, the diagnostic log.
//!
//! The folder is `WAYSTONE_ROLLER_DIR` when set and non-empty, else
//! `%LOCALAPPDATA%\ExilePact2\waystone-roller\`. A folder inside a git
//! checkout (a `.git` folder or file in it or above it) is refused, as the
//! quest tracker's store refuses one: the app must never write into a
//! repository.

use std::ffi::OsString;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

/// The environment variable that overrides the store folder.
pub const DIR_ENV: &str = "WAYSTONE_ROLLER_DIR";
/// The settings file's name and its schema number.
pub const SETTINGS_FILE: &str = "settings.json";
pub const SETTINGS_SCHEMA: u64 = 1;
/// The diagnostic log's name, and the size at which it is rotated into a
/// single old copy (`diag.log.1`).
pub const DIAG_FILE: &str = "diag.log";
pub const DIAG_ROTATE_BYTES: u64 = 1024 * 1024;

/// Why there is no usable store folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    /// Neither `WAYSTONE_ROLLER_DIR` nor `LOCALAPPDATA` is set.
    NoDirectory,
    /// The folder is inside a git checkout whose `.git` is `marker`.
    GitCheckout { dir: PathBuf, marker: PathBuf },
    /// The folder could not be resolved or created.
    Io { path: PathBuf, message: String },
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::NoDirectory => {
                write!(
                    f,
                    "no store folder: neither {DIR_ENV} nor LOCALAPPDATA is set"
                )
            }
            StoreError::GitCheckout { dir, marker } => write!(
                f,
                "the store folder {} is refused: it is inside a git checkout ({})",
                dir.display(),
                marker.display()
            ),
            StoreError::Io { path, message } => write!(f, "{}: {message}", path.display()),
        }
    }
}

impl std::error::Error for StoreError {}

/// The store folder from an override and `%LOCALAPPDATA%`, both injected so
/// tests never touch the process environment. An empty value counts as
/// unset.
pub fn resolve_dir(
    override_dir: Option<OsString>,
    local_app_data: Option<OsString>,
) -> Result<PathBuf, StoreError> {
    if let Some(d) = override_dir.filter(|d| !d.is_empty()) {
        return Ok(PathBuf::from(d));
    }
    match local_app_data.filter(|d| !d.is_empty()) {
        Some(base) => Ok(PathBuf::from(base)
            .join("ExilePact2")
            .join("waystone-roller")),
        None => Err(StoreError::NoDirectory),
    }
}

/// [`resolve_dir`] over this process's `WAYSTONE_ROLLER_DIR` and
/// `LOCALAPPDATA`.
pub fn default_dir() -> Result<PathBuf, StoreError> {
    resolve_dir(std::env::var_os(DIR_ENV), std::env::var_os("LOCALAPPDATA"))
}

/// Refuses a folder inside a git checkout. Reads only metadata; the folder
/// need not exist yet, and its nearest existing ancestor is also checked
/// through links, so a junction into a checkout is caught too.
pub fn check_dir(dir: &Path) -> Result<(), StoreError> {
    let abs = std::path::absolute(dir).map_err(|e| StoreError::Io {
        path: dir.to_path_buf(),
        message: e.to_string(),
    })?;
    for candidate in [Some(abs.clone()), resolve_existing(&abs)]
        .into_iter()
        .flatten()
    {
        for a in candidate.ancestors() {
            let git = a.join(".git");
            if git.exists() {
                return Err(StoreError::GitCheckout {
                    dir: dir.to_path_buf(),
                    marker: git,
                });
            }
        }
    }
    Ok(())
}

/// `abs` with its longest existing prefix canonicalized, or `None` when no
/// prefix resolves.
fn resolve_existing(abs: &Path) -> Option<PathBuf> {
    let mut rest: Vec<&std::ffi::OsStr> = Vec::new();
    let mut cur = abs;
    loop {
        if let Ok(mut real) = cur.canonicalize() {
            for part in rest.iter().rev() {
                real.push(part);
            }
            return Some(real);
        }
        rest.push(cur.file_name()?);
        cur = cur.parent()?;
    }
}

/// The store folder, checked; nothing is created until something is written.
#[derive(Debug)]
pub struct Store {
    dir: PathBuf,
    diag: DiagLog,
}

impl Store {
    /// A store at `dir`, refused inside a git checkout.
    pub fn open(dir: impl Into<PathBuf>) -> Result<Store, StoreError> {
        let dir = dir.into();
        check_dir(&dir)?;
        let diag = DiagLog::new(dir.join(DIAG_FILE));
        Ok(Store { dir, diag })
    }

    /// A store at [`default_dir`].
    pub fn open_default() -> Result<Store, StoreError> {
        Store::open(default_dir()?)
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn settings_path(&self) -> PathBuf {
        self.dir.join(SETTINGS_FILE)
    }

    pub fn diag(&self) -> &DiagLog {
        &self.diag
    }

    /// The saved settings: `Ok(None)` when there is no file yet, `Err` with
    /// one sentence when the file cannot be read or is not a JSON object.
    pub fn load_settings(&self) -> Result<Option<Value>, String> {
        let path = self.settings_path();
        let text = match fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(format!("{} could not be read: {e}", path.display())),
        };
        match serde_json::from_str::<Value>(&text) {
            Ok(v) if v.is_object() => Ok(Some(v)),
            Ok(_) => Err(format!("{} is not a JSON object", path.display())),
            Err(e) => Err(format!("{} is not valid JSON: {e}", path.display())),
        }
    }

    /// Writes the settings atomically.
    pub fn save_settings(&self, value: &Value) -> Result<(), String> {
        write_json_atomic(&self.settings_path(), value).map_err(|e| e.to_string())
    }
}

/// Writes `value` to `path` atomically: `<file>.tmp` in the same folder,
/// written and synced, then renamed over `path`. Creates the folder. On any
/// failure the old `path` is untouched.
pub fn write_json_atomic(path: &Path, value: &Value) -> Result<(), StoreError> {
    let io_err = |p: &Path, e: io::Error| StoreError::Io {
        path: p.to_path_buf(),
        message: e.to_string(),
    };
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(|e| io_err(parent, e))?;
    }
    let mut tmp_name = path.file_name().unwrap_or_default().to_os_string();
    tmp_name.push(".tmp");
    let tmp = path.with_file_name(tmp_name);

    let mut bytes = serde_json::to_vec_pretty(value).map_err(|e| io_err(path, e.into()))?;
    bytes.push(b'\n');
    let written = (|| -> io::Result<()> {
        let mut f = File::create(&tmp)?;
        f.write_all(&bytes)?;
        f.sync_all()
    })();
    if let Err(e) = written {
        let _ = fs::remove_file(&tmp);
        return Err(io_err(&tmp, e));
    }
    if let Err(e) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(io_err(path, e));
    }
    Ok(())
}

/// An append-only, size-rotated text log, one UTC-stamped line per call.
/// Safe to share between threads.
#[derive(Debug)]
pub struct DiagLog {
    path: PathBuf,
    lock: Mutex<()>,
}

impl DiagLog {
    pub fn new(path: impl Into<PathBuf>) -> DiagLog {
        DiagLog {
            path: path.into(),
            lock: Mutex::new(()),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Appends `message` as one line; line breaks inside it become spaces.
    pub fn write(&self, message: &str) -> io::Result<()> {
        let flat: String = message
            .chars()
            .map(|c| if c == '\r' || c == '\n' { ' ' } else { c })
            .collect();
        let line = format!("{} {}\n", utc_now(), flat);
        let _guard = self.lock.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(parent) = self.path.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent)?;
        }
        let len = match fs::metadata(&self.path) {
            Ok(m) => m.len(),
            Err(e) if e.kind() == io::ErrorKind::NotFound => 0,
            Err(e) => return Err(e),
        };
        if len > 0 && len + line.len() as u64 > DIAG_ROTATE_BYTES {
            let mut old = self.path.file_name().unwrap_or_default().to_os_string();
            old.push(".1");
            let old = self.path.with_file_name(old);
            let _ = fs::remove_file(&old);
            fs::rename(&self.path, &old)?;
        }
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        f.write_all(line.as_bytes())
    }
}

/// The time now as `YYYY-MM-DD HH:MM:SSZ` (UTC).
pub fn utc_now() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format_utc(secs)
}

/// `secs` since the Unix epoch as `YYYY-MM-DD HH:MM:SSZ`.
pub fn format_utc(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    // Days to a civil date (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + if month <= 2 { 1 } else { 0 };
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utc_dates() {
        assert_eq!(format_utc(0), "1970-01-01 00:00:00Z");
        assert_eq!(format_utc(951_782_400), "2000-02-29 00:00:00Z");
        assert_eq!(format_utc(1_793_190_896), "2026-10-28 12:34:56Z");
    }
}
