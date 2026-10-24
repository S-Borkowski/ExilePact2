//! The diagnostic log, `overlay.log`: one timestamped line per call,
//! rotated at 1 MB into a single old copy, `overlay.log.1`.
//!
//! The log is for the person reading a bug report. Callers write events
//! (start and stop, paths, areas, decisions, errors), never chat or another
//! player's name; the log itself only guarantees one line per call.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::stamp::Utc;

/// The size at which `overlay.log` is rotated.
pub const DIAG_ROTATE_BYTES: u64 = 1024 * 1024;

/// An append-only, size-rotated text log. Safe to share between threads.
#[derive(Debug)]
pub struct DiagLog {
    path: PathBuf,
    limit: u64,
    lock: Mutex<()>,
}

impl DiagLog {
    /// A log at `path`, rotated at [`DIAG_ROTATE_BYTES`]. Nothing is created
    /// until the first line is written.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self::with_limit(path, DIAG_ROTATE_BYTES)
    }

    /// A log rotated at `limit` bytes instead.
    pub fn with_limit(path: impl Into<PathBuf>, limit: u64) -> Self {
        DiagLog {
            path: path.into(),
            limit,
            lock: Mutex::new(()),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The one old copy kept after a rotation: the log's name plus `.1`.
    pub fn old_path(&self) -> PathBuf {
        let mut name = self.path.file_name().unwrap_or_default().to_os_string();
        name.push(".1");
        self.path.with_file_name(name)
    }

    /// Appends `message` as one line, prefixed with the UTC time. Line breaks
    /// inside `message` become spaces, so one call is always one line.
    /// Rotates first when the line would take the file past the limit.
    pub fn write(&self, message: &str) -> io::Result<()> {
        let flat: String = message
            .chars()
            .map(|c| if c == '\r' || c == '\n' { ' ' } else { c })
            .collect();
        let line = format!("{} {}\n", Utc::now().readable(), flat);

        let _guard = self.lock.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        let len = match fs::metadata(&self.path) {
            Ok(m) => m.len(),
            Err(e) if e.kind() == io::ErrorKind::NotFound => 0,
            Err(e) => return Err(e),
        };
        if len > 0 && len + line.len() as u64 > self.limit {
            let old = self.old_path();
            match fs::remove_file(&old) {
                Ok(()) => {}
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                Err(e) => return Err(e),
            }
            fs::rename(&self.path, &old)?;
        }
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        f.write_all(line.as_bytes())
    }
}
