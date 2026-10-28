//! Putting a verified download in place of the running exe, and removing
//! what an install leaves behind at the next start.
//!
//! Windows lets a running image be renamed but not deleted. So the swap is
//! two renames in the exe's own folder (one volume, so each is atomic): the
//! running exe to `<exe>.old`, replacing a stale one, then the staged file
//! to the exe's name. When the second fails, the first is undone and the
//! running exe is where it was. The `.old` file cannot be removed while the
//! old process runs; the next start removes it ([`cleanup`]), retrying while
//! the old process may still be ending.

use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

/// The suffix of the staging file an install downloads into, beside the exe.
pub const DOWNLOAD_SUFFIX: &str = ".download";
/// The suffix the replaced exe is renamed to.
pub const OLD_SUFFIX: &str = ".old";
/// How long the start-up cleanup keeps trying.
pub const CLEANUP_FOR: Duration = Duration::from_secs(30);
/// How long it waits between tries.
pub const CLEANUP_EVERY: Duration = Duration::from_millis(500);

fn with_suffix(exe: &Path, suffix: &str) -> PathBuf {
    let mut name: OsString = exe.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

/// `<exe>.download`: where an install stages the new exe.
pub fn staging_path(exe: &Path) -> PathBuf {
    with_suffix(exe, DOWNLOAD_SUFFIX)
}

/// `<exe>.old`: where the swap moves the running exe.
pub fn old_path(exe: &Path) -> PathBuf {
    with_suffix(exe, OLD_SUFFIX)
}

/// Why the swap did not happen. The running exe is in place either way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapError {
    /// The running exe could not be moved aside; nothing changed.
    MoveAside(io::ErrorKind),
    /// The new file could not be moved in; the running exe was moved back.
    MoveIn(io::ErrorKind),
    /// The new file could not be moved in, and moving the running exe back
    /// failed too: it is at `<exe>.old`. Not expected on one volume; said
    /// so that it is never silent.
    Restore(io::ErrorKind),
}

impl SwapError {
    pub fn reason(&self) -> String {
        match self {
            SwapError::MoveAside(k) => format!("the running app could not be moved aside ({k})"),
            SwapError::MoveIn(k) => format!("the new version could not be put in place ({k})"),
            SwapError::Restore(k) => format!(
                "the new version could not be put in place, and the running app was left as {OLD_SUFFIX} ({k})"
            ),
        }
    }
}

/// Renames `exe` to `<exe>.old` and `staged` to `exe`, with
/// [`std::fs::rename`] only.
pub fn swap(exe: &Path, staged: &Path) -> Result<(), SwapError> {
    let old = old_path(exe);
    fs::rename(exe, &old).map_err(|e| SwapError::MoveAside(e.kind()))?;
    if let Err(e) = fs::rename(staged, exe) {
        return match fs::rename(&old, exe) {
            Ok(()) => Err(SwapError::MoveIn(e.kind())),
            Err(back) => Err(SwapError::Restore(back.kind())),
        };
    }
    Ok(())
}

/// What one cleanup pass did with one leftover file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cleaned {
    Removed(PathBuf),
    /// It is still there; the error the last try gave.
    Kept(PathBuf, io::ErrorKind),
}

/// One pass: removes `<exe>.old` and `<exe>.download` where they exist.
/// Lists only the files that existed; nothing else is touched.
pub fn cleanup_once(exe: &Path) -> Vec<Cleaned> {
    [old_path(exe), staging_path(exe)]
        .into_iter()
        .filter_map(|p| match fs::remove_file(&p) {
            Ok(()) => Some(Cleaned::Removed(p)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => None,
            Err(e) => Some(Cleaned::Kept(p, e.kind())),
        })
        .collect()
}

/// Passes of [`cleanup_once`] until nothing is left or `give_up_after`
/// passed, `every` apart. Returns every file it removed and those it kept.
pub fn cleanup(exe: &Path, give_up_after: Duration, every: Duration) -> Vec<Cleaned> {
    let start = Instant::now();
    let mut removed = Vec::new();
    loop {
        let mut kept = Vec::new();
        for c in cleanup_once(exe) {
            match c {
                Cleaned::Removed(_) => removed.push(c),
                Cleaned::Kept(..) => kept.push(c),
            }
        }
        if kept.is_empty() || start.elapsed() >= give_up_after {
            removed.extend(kept);
            return removed;
        }
        thread::sleep(every);
    }
}

/// The diagnostic lines of a cleanup: one line, naming file names only.
pub fn cleanup_line(result: &[Cleaned]) -> String {
    let name = |p: &PathBuf| {
        p.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    };
    if result.is_empty() {
        return "updates: cleanup: nothing to remove".to_string();
    }
    let parts: Vec<String> = result
        .iter()
        .map(|c| match c {
            Cleaned::Removed(p) => format!("removed {}", name(p)),
            Cleaned::Kept(p, k) => format!("could not remove {} ({k})", name(p)),
        })
        .collect();
    format!("updates: cleanup: {}", parts.join(", "))
}

/// The start-up cleanup on a background thread of its own: [`cleanup`] for
/// up to [`CLEANUP_FOR`], then `done` hears its one line. The caller never
/// waits for it.
pub fn spawn_cleanup(exe: PathBuf, done: impl FnOnce(String) + Send + 'static) -> io::Result<()> {
    thread::Builder::new()
        .name("ep-update-cleanup".to_string())
        .spawn(move || done(cleanup_line(&cleanup(&exe, CLEANUP_FOR, CLEANUP_EVERY))))
        .map(|_| ())
}
