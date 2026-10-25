//! The current area when a tailer starts, read back from the log.
//!
//! The engine learns the area from a `Generating level` line it reads. A
//! tailer that starts mid-zone (resuming a saved position, or a first start
//! at the end of the log) never reads that line, so before its first event
//! reaches the engine the shell reads the log before the tailer's start
//! position, at most [`MAX_SEED_BYTES`], read-only, and seeds the engine's
//! area with what [`qt_engine::area_from_log`] makes of it. A start at 0
//! needs no seed: the tailer reads the area lines itself.

use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

use qt_engine::{area_from_log, LogArea};

/// The most bytes read back before a start position: a zone's area line is
/// normally far inside it, and a start never reads the whole log.
pub const MAX_SEED_BYTES: u64 = 8 * 1024 * 1024;

/// What the log before `offset` says about the current area. Reads at most
/// [`MAX_SEED_BYTES`] before `offset`, read-only, and closes the file
/// before returning. A line cut at the start of that window, and an
/// unfinished line at `offset`, are left out.
pub fn area_before(path: &Path, offset: u64) -> io::Result<LogArea> {
    let mut file = qt_tailer::open_read_only(path)?;
    let start = offset.saturating_sub(MAX_SEED_BYTES);
    // One byte before the window too, so a line starting exactly at its
    // edge is recognised as whole.
    let from = start.saturating_sub(1);
    file.seek(SeekFrom::Start(from))?;
    let mut bytes = Vec::new();
    file.by_ref().take(offset - from).read_to_end(&mut bytes)?;
    drop(file);

    let first = if from > 0 {
        bytes
            .iter()
            .position(|&b| b == b'\n')
            .map_or(bytes.len(), |i| i + 1)
    } else {
        0
    };
    let end = bytes
        .iter()
        .rposition(|&b| b == b'\n')
        .map_or(0, |i| i + 1)
        .max(first);
    let lines = bytes[first..end]
        .split(|&b| b == b'\n')
        .map(|raw| String::from_utf8_lossy(raw.strip_suffix(b"\r").unwrap_or(raw)).into_owned());
    Ok(area_from_log(lines))
}
