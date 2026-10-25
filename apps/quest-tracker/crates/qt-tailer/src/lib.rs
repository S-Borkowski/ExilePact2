//! Log follower: polls Client.txt on its own thread, opens it read-only only
//! on a poll that sees new bytes, and sends complete decoded lines with their
//! end offset and a catch-up flag over a channel. It never touches quest
//! state.
//!
//! Three ways in:
//!
//! - [`spawn`] starts the follower on its own `std::thread`. It polls every
//!   [`Config::poll_interval`] through an injected [`Clock`], sends
//!   [`TailEvent`]s over an `std::sync::mpsc` channel and ends when the stop
//!   flag is set (checked before and after every poll, and between lines), or
//!   when the receiver is dropped.
//! - [`Follower`] is the same state machine without the thread: one
//!   [`Follower::poll`] per call. The thread is a loop around it.
//! - [`read_whole_file`] is the replay mode: 0 to EOF once, every line
//!   catch-up, no thread.
//!
//! [`start_position`] answers, without starting anything, where a follower
//! started now would begin reading: the same rule its first poll applies.
//!
//! Each poll first stats the path. When the size and modification time are
//! what the last read left, nothing is opened. Otherwise the file is opened
//! read-only ([`open_read_only`]), its fingerprint checked, the new bytes
//! read, and the handle closed before the poll returns. The handle is never
//! held between polls: the game recreates `LatestClient.txt`, and a user may
//! delete or rotate `Client.txt` while the overlay runs.
//!
//! A file counts as new, and is read from offset 0, when its size drops below
//! the offset already read (truncation), when the first bytes no longer match
//! the fingerprint (the game rewriting the file), when the path names a
//! different file than before (rotation: on Unix the inode, on Windows the
//! creation time), or when it appears after having been missing. Windows'
//! file-system tunnelling gives a file created within seconds under a just
//! renamed or deleted name the old creation time, so on Windows a quick
//! rename-then-recreate is caught by the size or the fingerprint instead; a
//! new game log starts with its own timestamps, so its first bytes differ.

use std::fs::{self, File, Metadata};
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// How often the thread polls unless [`Config::poll_interval`] says otherwise.
pub const DEFAULT_POLL_INTERVAL: Duration = Duration::from_millis(250);

/// How many leading bytes the fingerprint covers once the file is that long.
pub const FINGERPRINT_BYTES: usize = 1024;

/// Read size for one `read` call while catching up.
const CHUNK: usize = 64 * 1024;

/// SHA-256 of a file's first [`FINGERPRINT_BYTES`] bytes, or of all of a
/// shorter file. `covered` says how many bytes were hashed, so a fingerprint
/// taken while the file was short still identifies it after it has grown.
///
/// Serialised as `{"covered": <n>, "sha256": "<64 hex digits>"}`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(into = "FingerprintRepr", try_from = "FingerprintRepr")]
pub struct Fingerprint {
    covered: u16,
    sha256: [u8; 32],
}

#[derive(Serialize, Deserialize)]
struct FingerprintRepr {
    covered: u64,
    sha256: String,
}

impl Fingerprint {
    /// Fingerprint of `head`, the first bytes of a file; only the first
    /// [`FINGERPRINT_BYTES`] of it are hashed.
    pub fn of(head: &[u8]) -> Self {
        let head = &head[..head.len().min(FINGERPRINT_BYTES)];
        let digest = Sha256::digest(head);
        let mut sha256 = [0u8; 32];
        sha256.copy_from_slice(&digest);
        Fingerprint {
            covered: head.len() as u16,
            sha256,
        }
    }

    /// How many leading bytes were hashed (at most [`FINGERPRINT_BYTES`]).
    pub fn covered(&self) -> u64 {
        u64::from(self.covered)
    }

    /// The hash as 64 lowercase hex digits.
    pub fn hex(&self) -> String {
        self.sha256.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// True when `head` (a file's first bytes) starts with the bytes this
    /// fingerprint was taken over.
    pub fn matches(&self, head: &[u8]) -> bool {
        let covered = usize::from(self.covered);
        head.len() >= covered && Fingerprint::of(&head[..covered]) == *self
    }
}

impl From<Fingerprint> for FingerprintRepr {
    fn from(f: Fingerprint) -> Self {
        FingerprintRepr {
            covered: f.covered(),
            sha256: f.hex(),
        }
    }
}

impl TryFrom<FingerprintRepr> for Fingerprint {
    type Error = String;

    fn try_from(r: FingerprintRepr) -> Result<Self, Self::Error> {
        if r.covered > FINGERPRINT_BYTES as u64 {
            return Err(format!(
                "fingerprint covers {} bytes, more than {FINGERPRINT_BYTES}",
                r.covered
            ));
        }
        let hex = r.sha256.as_bytes();
        if hex.len() != 64 {
            return Err("fingerprint sha256 is not 64 hex digits".to_string());
        }
        let mut sha256 = [0u8; 32];
        for (i, pair) in hex.chunks(2).enumerate() {
            let text = std::str::from_utf8(pair).map_err(|e| e.to_string())?;
            sha256[i] =
                u8::from_str_radix(text, 16).map_err(|e| format!("fingerprint sha256: {e}"))?;
        }
        Ok(Fingerprint {
            covered: r.covered as u16,
            sha256,
        })
    }
}

/// Where reading stopped: the store's `position.json` (path, fingerprint,
/// offset). `offset` is the end offset of the last line handled.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    pub path: PathBuf,
    pub fingerprint: Fingerprint,
    pub offset: u64,
}

/// One complete line of the log.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    /// The line decoded as UTF-8 (invalid sequences replaced), without its
    /// `\n` or `\r\n`.
    pub text: String,
    /// Byte offset just past this line's `\n` in the file.
    pub end_offset: u64,
    /// True when the line's bytes already existed when the tailer started
    /// (resume, `from_start` or replay); false for lines written since.
    pub catch_up: bool,
    /// Fingerprint of the file the line was read from.
    pub fingerprint: Fingerprint,
}

impl Line {
    /// The position to save once this line has been handled.
    pub fn position(&self, path: &Path) -> Position {
        Position {
            path: path.to_path_buf(),
            fingerprint: self.fingerprint,
            offset: self.end_offset,
        }
    }
}

/// Why reading (re)started where it did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    /// No saved position: started at the end of the existing file.
    AtEnd,
    /// `from_start`: the whole existing file, as catch-up.
    FromStart,
    /// The saved position matched the file: resumed from its offset.
    Resumed,
    /// A saved position for this path no longer matched (other fingerprint,
    /// or the file is shorter than its offset): the whole file, as catch-up.
    SavedMismatch,
    /// The file appeared after being missing: read from 0.
    Appeared,
    /// The file shrank below the offset already read: read from 0.
    Truncated,
    /// The first bytes changed (the game rewrote the file): read from 0.
    Replaced,
    /// The path names a different file than before (rename, then recreate):
    /// read from 0.
    Rotated,
}

/// What the tailer sends over its channel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TailEvent {
    /// A complete line.
    Line(Line),
    /// Reading starts, or restarts, at `offset`.
    Following { offset: u64, reason: Reason },
    /// The file does not exist; sent once each time it goes missing.
    Waiting,
    /// A stat, open or read failed; sent once per distinct message, retried
    /// on the next poll.
    Error(String),
}

/// What the tailer is started with.
#[derive(Clone, Debug)]
pub struct Config {
    pub path: PathBuf,
    /// A position saved by an earlier run. It is used when its path is this
    /// path and its fingerprint still matches the file.
    pub saved: Option<Position>,
    /// Read the whole existing file as catch-up, ignoring any saved position.
    pub from_start: bool,
    pub poll_interval: Duration,
}

impl Config {
    /// A config for `path`: no saved position, start at the end, poll every
    /// [`DEFAULT_POLL_INTERVAL`].
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Config {
            path: path.into(),
            saved: None,
            from_start: false,
            poll_interval: DEFAULT_POLL_INTERVAL,
        }
    }
}

/// How the tailer thread waits between polls; tests inject their own.
pub trait Clock: Send + 'static {
    fn sleep(&self, interval: Duration);
}

/// The real clock: `std::thread::sleep`.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn sleep(&self, interval: Duration) {
        thread::sleep(interval);
    }
}

/// Opens `path` for reading only. Rust's default Windows share mode shares
/// read, write and delete, so the game can keep writing, and anyone can
/// rename or delete the file, while this handle is open.
pub fn open_read_only(path: &Path) -> io::Result<File> {
    File::open(path)
}

/// Starts the follower on its own thread (named `qt-tailer`). It sends every
/// event to `events` and ends within one poll interval of `stop` being set,
/// or as soon as `events`' receiver is gone.
pub fn spawn<C: Clock>(
    config: Config,
    clock: C,
    events: Sender<TailEvent>,
    stop: Arc<AtomicBool>,
) -> io::Result<JoinHandle<()>> {
    thread::Builder::new()
        .name("qt-tailer".to_string())
        .spawn(move || run(config, clock, events, stop))
}

fn run<C: Clock>(config: Config, clock: C, events: Sender<TailEvent>, stop: Arc<AtomicBool>) {
    let interval = config.poll_interval;
    let mut follower = Follower::new(config);
    while !stop.load(Ordering::Acquire) {
        let go =
            follower.poll(&mut |event| !stop.load(Ordering::Acquire) && events.send(event).is_ok());
        if !go || stop.load(Ordering::Acquire) {
            break;
        }
        clock.sleep(interval);
    }
}

/// Replay mode: reads `path` from 0 to EOF once, read-only, and hands every
/// line to `each` as catch-up. A last line without `\n` is handed over too
/// (the file is complete). Returns the position at EOF.
pub fn read_whole_file(path: &Path, mut each: impl FnMut(Line)) -> io::Result<Position> {
    let mut file = open_read_only(path)?;
    let fingerprint = Fingerprint::of(&read_head(&mut file)?);
    file.seek(SeekFrom::Start(0))?;
    let mut reader = BufReader::with_capacity(CHUNK, file);
    let mut raw = Vec::new();
    let mut offset = 0u64;
    loop {
        raw.clear();
        let n = reader.read_until(b'\n', &mut raw)?;
        if n == 0 {
            break;
        }
        offset += n as u64;
        each(Line {
            text: decode(&raw),
            end_offset: offset,
            catch_up: true,
            fingerprint,
        });
    }
    Ok(Position {
        path: path.to_path_buf(),
        fingerprint,
        offset,
    })
}

/// The follower's state machine, one [`poll`](Follower::poll) at a time.
#[derive(Debug)]
pub struct Follower {
    path: PathBuf,
    saved: Option<Position>,
    from_start: bool,
    phase: Phase,
    last_error: Option<String>,
}

#[derive(Debug)]
enum Phase {
    /// Nothing polled yet.
    Start,
    /// The file is missing; it is read from 0 once it appears.
    Waiting,
    Following(Tracked),
}

#[derive(Debug)]
struct Tracked {
    /// Offset just past the last byte read (the partial line included).
    pos: u64,
    /// Bytes of the last line read so far, waiting for its `\n`.
    partial: Vec<u8>,
    /// Started at the end of a file whose last line was unfinished: drop the
    /// rest of that line rather than send its tail as a line.
    skip_to_newline: bool,
    fingerprint: Fingerprint,
    /// Lines ending at or before this offset existed when the tailer started.
    catch_up_end: u64,
    /// The stat a complete read left; `None` forces the next poll to open.
    seen: Option<Seen>,
    identity: Option<FileId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Seen {
    len: u64,
    modified: Option<SystemTime>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)] // one variant per platform
enum FileId {
    Inode(u64, u64),
    Created(SystemTime),
}

#[cfg(unix)]
fn file_id(meta: &Metadata) -> Option<FileId> {
    use std::os::unix::fs::MetadataExt;
    Some(FileId::Inode(meta.dev(), meta.ino()))
}

#[cfg(not(unix))]
fn file_id(meta: &Metadata) -> Option<FileId> {
    meta.created().ok().map(FileId::Created)
}

impl Follower {
    pub fn new(config: Config) -> Self {
        Follower {
            path: config.path,
            saved: config.saved,
            from_start: config.from_start,
            phase: Phase::Start,
            last_error: None,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// One poll: hands every event to `sink`, in order. Returns false when
    /// `sink` returned false (stop, or nobody listening); the bytes after the
    /// last line it accepted are read again on the next poll.
    pub fn poll(&mut self, sink: &mut dyn FnMut(TailEvent) -> bool) -> bool {
        match self.poll_inner(sink) {
            Ok(go) => {
                self.last_error = None;
                go
            }
            Err(e) => {
                let message = format!("{}: {e}", self.path.display());
                if self.last_error.as_deref() == Some(message.as_str()) {
                    return true;
                }
                self.last_error = Some(message.clone());
                sink(TailEvent::Error(message))
            }
        }
    }

    fn poll_inner(&mut self, sink: &mut dyn FnMut(TailEvent) -> bool) -> io::Result<bool> {
        let meta = match fs::metadata(&self.path) {
            Ok(meta) if meta.is_file() => meta,
            Ok(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "not a regular file",
                ));
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(self.go_missing(sink)),
            Err(e) => return Err(e),
        };
        let seen = Seen {
            len: meta.len(),
            modified: meta.modified().ok(),
        };
        let identity = file_id(&meta);

        let restart = match &self.phase {
            Phase::Start => None,
            Phase::Waiting => Some(Reason::Appeared),
            Phase::Following(t) => {
                if matches!((t.identity, identity), (Some(a), Some(b)) if a != b) {
                    Some(Reason::Rotated)
                } else if seen.len < t.pos {
                    Some(Reason::Truncated)
                } else if t.seen == Some(seen) {
                    return Ok(true);
                } else {
                    None
                }
            }
        };

        let mut file = match open_read_only(&self.path) {
            Ok(file) => file,
            // Gone between the stat and the open: the next poll sees it.
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(true),
            Err(e) => return Err(e),
        };
        let head = read_head(&mut file)?;

        if matches!(self.phase, Phase::Start) {
            return self.start(file, &head, seen, identity, sink);
        }
        let reason = match (&self.phase, restart) {
            (_, Some(reason)) => Some(reason),
            (Phase::Following(t), None) if !t.fingerprint.matches(&head) => Some(Reason::Replaced),
            _ => None,
        };
        if let Some(reason) = reason {
            self.phase = Phase::Following(Tracked {
                pos: 0,
                partial: Vec::new(),
                skip_to_newline: false,
                fingerprint: Fingerprint::of(&head),
                catch_up_end: 0,
                seen: None,
                identity,
            });
            if !sink(TailEvent::Following { offset: 0, reason }) {
                return Ok(false);
            }
        } else if let Phase::Following(t) = &mut self.phase {
            // Same file; a fingerprint taken while it was short now covers
            // more of it.
            t.fingerprint = Fingerprint::of(&head);
            t.identity = identity;
        }
        self.read_on(file, seen, sink)
    }

    fn go_missing(&mut self, sink: &mut dyn FnMut(TailEvent) -> bool) -> bool {
        if matches!(self.phase, Phase::Waiting) {
            return true;
        }
        self.phase = Phase::Waiting;
        sink(TailEvent::Waiting)
    }

    fn start(
        &mut self,
        mut file: File,
        head: &[u8],
        seen: Seen,
        identity: Option<FileId>,
        sink: &mut dyn FnMut(TailEvent) -> bool,
    ) -> io::Result<bool> {
        let len = seen.len;
        let StartAt {
            offset: pos,
            reason,
        } = decide_start(&self.path, self.saved.as_ref(), self.from_start, head, len);
        let skip_to_newline =
            reason == Reason::AtEnd && len > 0 && last_byte(&mut file, len)? != b'\n';
        self.phase = Phase::Following(Tracked {
            pos,
            partial: Vec::new(),
            skip_to_newline,
            fingerprint: Fingerprint::of(head),
            catch_up_end: len,
            seen: None,
            identity,
        });
        if !sink(TailEvent::Following {
            offset: pos,
            reason,
        }) {
            return Ok(false);
        }
        self.read_on(file, seen, sink)
    }

    /// Reads from the tracked offset to EOF, sends every complete line, and
    /// drops the handle on return.
    fn read_on(
        &mut self,
        mut file: File,
        seen: Seen,
        sink: &mut dyn FnMut(TailEvent) -> bool,
    ) -> io::Result<bool> {
        let Phase::Following(t) = &mut self.phase else {
            return Ok(true);
        };
        t.seen = None;
        file.seek(SeekFrom::Start(t.pos))?;
        let mut buf = vec![0u8; CHUNK];
        loop {
            let n = match file.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            };
            t.pos += n as u64;
            let mut chunk = &buf[..n];
            if t.skip_to_newline {
                match chunk.iter().position(|&b| b == b'\n') {
                    Some(i) => {
                        chunk = &chunk[i + 1..];
                        t.skip_to_newline = false;
                    }
                    None => continue,
                }
            }
            t.partial.extend_from_slice(chunk);
            // File offset of `partial[0]`.
            let base = t.pos - t.partial.len() as u64;
            let mut start = 0;
            while let Some(i) = t.partial[start..].iter().position(|&b| b == b'\n') {
                let end = start + i + 1;
                let end_offset = base + end as u64;
                let line = Line {
                    text: decode(&t.partial[start..end]),
                    end_offset,
                    catch_up: end_offset <= t.catch_up_end,
                    fingerprint: t.fingerprint,
                };
                start = end;
                if !sink(TailEvent::Line(line)) {
                    // Keep what was delivered; the next poll reads the rest.
                    t.pos = end_offset;
                    t.partial.clear();
                    return Ok(false);
                }
            }
            t.partial.drain(..start);
        }
        t.seen = Some(seen);
        Ok(true)
    }
}

/// Where a tailer begins reading an existing file, and why.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StartAt {
    pub offset: u64,
    pub reason: Reason,
}

/// Where a tailer started now with `config` would begin reading: the
/// decision [`Follower`] makes on its first poll, for a caller that has to
/// agree with it (the shell reads the log before this offset for the
/// current area). `None` when the file does not exist: the tailer waits for
/// it and then reads it from 0. Opens the file read-only and closes it
/// before returning.
pub fn start_position(config: &Config) -> io::Result<Option<StartAt>> {
    let mut file = match open_read_only(&config.path) {
        Ok(file) => file,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    let len = file.metadata()?.len();
    let head = read_head(&mut file)?;
    Ok(Some(decide_start(
        &config.path,
        config.saved.as_ref(),
        config.from_start,
        &head,
        len,
    )))
}

/// The start rule: `from_start` reads from 0; a saved position for this
/// path resumes from its offset while its fingerprint still matches and the
/// file is not shorter, else the whole file is read; with none, reading
/// starts at the end of the file.
fn decide_start(
    path: &Path,
    saved: Option<&Position>,
    from_start: bool,
    head: &[u8],
    len: u64,
) -> StartAt {
    let (offset, reason) = if from_start {
        (0, Reason::FromStart)
    } else if let Some(saved) = saved.filter(|s| s.path == path) {
        if saved.offset <= len && saved.fingerprint.matches(head) {
            (saved.offset, Reason::Resumed)
        } else {
            (0, Reason::SavedMismatch)
        }
    } else {
        (len, Reason::AtEnd)
    };
    StartAt { offset, reason }
}

/// Up to the first [`FINGERPRINT_BYTES`] bytes of a freshly opened file.
fn read_head(file: &mut File) -> io::Result<Vec<u8>> {
    let mut head = Vec::with_capacity(FINGERPRINT_BYTES);
    file.seek(SeekFrom::Start(0))?;
    file.by_ref()
        .take(FINGERPRINT_BYTES as u64)
        .read_to_end(&mut head)?;
    Ok(head)
}

fn last_byte(file: &mut File, len: u64) -> io::Result<u8> {
    let mut byte = [0u8; 1];
    file.seek(SeekFrom::Start(len - 1))?;
    file.read_exact(&mut byte)?;
    Ok(byte[0])
}

/// One raw line, `\n` or `\r\n` stripped, decoded as UTF-8 lossily.
fn decode(raw: &[u8]) -> String {
    let raw = raw.strip_suffix(b"\n").unwrap_or(raw);
    let raw = raw.strip_suffix(b"\r").unwrap_or(raw);
    String::from_utf8_lossy(raw).into_owned()
}

#[cfg(test)]
mod tests {
    //! [`start_position`] against the follower's own first `Following`
    //! event, in every way a start can go.

    use super::*;

    /// The follower's first `Following` event for `config`.
    fn follower_start(config: &Config) -> Option<StartAt> {
        let mut follower = Follower::new(config.clone());
        let mut first = None;
        follower.poll(&mut |event| {
            if let TailEvent::Following { offset, reason } = event {
                first.get_or_insert(StartAt { offset, reason });
            }
            true
        });
        first
    }

    #[test]
    fn start_position_agrees_with_the_follower() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Client.txt");
        let config = Config::new(&path);

        // A missing file: nothing to read back; the tailer waits.
        assert_eq!(start_position(&config).unwrap(), None);

        fs::write(&path, b"one\ntwo\nthree\n").unwrap();
        let len = 14;
        let saved = |offset: u64, head: &[u8]| Position {
            path: path.clone(),
            fingerprint: Fingerprint::of(head),
            offset,
        };
        let cases = [
            (
                Config::new(&path),
                StartAt {
                    offset: len,
                    reason: Reason::AtEnd,
                },
            ),
            (
                Config {
                    from_start: true,
                    saved: Some(saved(4, b"one\ntwo\nthree\n")),
                    ..Config::new(&path)
                },
                StartAt {
                    offset: 0,
                    reason: Reason::FromStart,
                },
            ),
            (
                Config {
                    saved: Some(saved(8, b"one\ntwo\nthree\n")),
                    ..Config::new(&path)
                },
                StartAt {
                    offset: 8,
                    reason: Reason::Resumed,
                },
            ),
            (
                Config {
                    saved: Some(saved(8, b"other\n")),
                    ..Config::new(&path)
                },
                StartAt {
                    offset: 0,
                    reason: Reason::SavedMismatch,
                },
            ),
            (
                Config {
                    saved: Some(saved(99, b"one\ntwo\nthree\n")),
                    ..Config::new(&path)
                },
                StartAt {
                    offset: 0,
                    reason: Reason::SavedMismatch,
                },
            ),
            // A position saved for another path is not this file's.
            (
                Config {
                    saved: Some(Position {
                        path: dir.path().join("Other.txt"),
                        ..saved(8, b"one\ntwo\nthree\n")
                    }),
                    ..Config::new(&path)
                },
                StartAt {
                    offset: len,
                    reason: Reason::AtEnd,
                },
            ),
        ];
        for (config, want) in cases {
            assert_eq!(start_position(&config).unwrap(), Some(want), "{config:?}");
            assert_eq!(follower_start(&config), Some(want), "{config:?}");
        }
    }
}
