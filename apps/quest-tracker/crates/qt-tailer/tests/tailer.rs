//! The tailer's contract, on temporary files. Every test but the two that need
//! real timing drives the tailer thread through an injected clock: each
//! `poll()` lets exactly one poll run and returns what it sent, so no test
//! sleeps.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use qt_tailer::{
    open_read_only, read_whole_file, spawn, Clock, Config, Fingerprint, Line, Position, Reason,
    SystemClock, TailEvent,
};

const WAIT: Duration = Duration::from_secs(10);

/// A clock whose `sleep` reports that a poll finished, then waits for the
/// test to allow the next one.
struct ManualClock {
    asleep: Sender<()>,
    ticks: Receiver<()>,
    sleeps: Arc<AtomicUsize>,
}

impl Clock for ManualClock {
    fn sleep(&self, _interval: Duration) {
        self.sleeps.fetch_add(1, Ordering::SeqCst);
        let _ = self.asleep.send(());
        let _ = self.ticks.recv_timeout(WAIT);
    }
}

struct Harness {
    events: Receiver<TailEvent>,
    ticks: Option<Sender<()>>,
    asleep: Receiver<()>,
    stop: Arc<AtomicBool>,
    sleeps: Arc<AtomicUsize>,
    thread: Option<JoinHandle<()>>,
}

impl Harness {
    /// Starts the tailer and returns what its first poll sent.
    fn start(config: Config) -> (Harness, Vec<TailEvent>) {
        let (asleep_tx, asleep) = mpsc::channel();
        let (ticks_tx, ticks) = mpsc::channel();
        let (events_tx, events) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let sleeps = Arc::new(AtomicUsize::new(0));
        let clock = ManualClock {
            asleep: asleep_tx,
            ticks,
            sleeps: sleeps.clone(),
        };
        let thread = spawn(config, clock, events_tx, stop.clone()).unwrap();
        let mut harness = Harness {
            events,
            ticks: Some(ticks_tx),
            asleep,
            stop,
            sleeps,
            thread: Some(thread),
        };
        let first = harness.finish_poll();
        (harness, first)
    }

    /// Lets one more poll run and returns what it sent.
    fn poll(&mut self) -> Vec<TailEvent> {
        self.ticks.as_ref().unwrap().send(()).unwrap();
        self.finish_poll()
    }

    fn finish_poll(&mut self) -> Vec<TailEvent> {
        self.asleep
            .recv_timeout(WAIT)
            .expect("the tailer did not finish its poll");
        self.events.try_iter().collect()
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.ticks = None;
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn lines(events: &[TailEvent]) -> Vec<&Line> {
    events
        .iter()
        .filter_map(|e| match e {
            TailEvent::Line(line) => Some(line),
            _ => None,
        })
        .collect()
}

fn texts(events: &[TailEvent]) -> Vec<&str> {
    lines(events).iter().map(|l| l.text.as_str()).collect()
}

fn following(events: &[TailEvent]) -> Vec<(u64, Reason)> {
    events
        .iter()
        .filter_map(|e| match e {
            TailEvent::Following { offset, reason } => Some((*offset, *reason)),
            _ => None,
        })
        .collect()
}

fn append(path: &Path, bytes: &[u8]) {
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
        .unwrap();
    file.write_all(bytes).unwrap();
}

fn log_in(dir: &tempfile::TempDir) -> PathBuf {
    dir.path().join("Client.txt")
}

#[test]
fn first_run_starts_at_end() {
    let dir = tempfile::tempdir().unwrap();
    let path = log_in(&dir);
    let existing = "2026/10/25 10:00:00 one\r\n2026/10/25 10:00:01 two\r\nhalf a li";
    fs::write(&path, existing).unwrap();

    let (mut tailer, first) = Harness::start(Config::new(&path));
    assert_eq!(
        following(&first),
        vec![(existing.len() as u64, Reason::AtEnd)]
    );
    assert!(
        lines(&first).is_empty(),
        "existing lines were sent: {first:?}"
    );

    append(&path, b"ne\r\nthree\r\n");
    let events = tailer.poll();
    // The unfinished line that existed at start is dropped, not sent as "ne".
    assert_eq!(texts(&events), vec!["three"]);
    let line = lines(&events)[0];
    assert!(!line.catch_up);
    assert_eq!(line.end_offset, fs::metadata(&path).unwrap().len());

    // Negative control: nothing new, nothing sent.
    assert!(tailer.poll().is_empty());
}

#[test]
fn holds_partial_last_line() {
    let dir = tempfile::tempdir().unwrap();
    let path = log_in(&dir);
    fs::write(&path, "").unwrap();
    let (mut tailer, first) = Harness::start(Config::new(&path));
    assert_eq!(following(&first), vec![(0, Reason::AtEnd)]);

    append(&path, b"Hello, wor");
    assert!(lines(&tailer.poll()).is_empty());
    append(&path, b"ld\r\nnext");
    let events = tailer.poll();
    assert_eq!(texts(&events), vec!["Hello, world"]);
    assert_eq!(
        lines(&events)[0].end_offset,
        "Hello, world\r\n".len() as u64
    );
    assert!(lines(&tailer.poll()).is_empty());
    append(&path, b"\n");
    let events = tailer.poll();
    assert_eq!(texts(&events), vec!["next"]);
    assert_eq!(
        lines(&events)[0].end_offset,
        "Hello, world\r\nnext\n".len() as u64
    );
}

#[test]
fn utf8_split_across_polls() {
    let dir = tempfile::tempdir().unwrap();
    let path = log_in(&dir);
    fs::write(&path, "").unwrap();
    let (mut tailer, _) = Harness::start(Config::new(&path));

    // Two-, three- and four-byte sequences, appended one byte per poll so
    // every sequence is split across polls.
    let text = "Zoné → Clearfell 🗡 end";
    let mut got = Vec::new();
    for byte in text.as_bytes().iter().chain(b"\r\n") {
        append(&path, &[*byte]);
        got.extend(tailer.poll());
    }
    assert_eq!(texts(&got), vec![text]);
    assert!(!lines(&got)[0].text.contains('\u{FFFD}'));
}

#[test]
fn truncation_restarts_at_zero() {
    let dir = tempfile::tempdir().unwrap();
    let path = log_in(&dir);
    fs::write(&path, "").unwrap();
    let (mut tailer, _) = Harness::start(Config::new(&path));
    append(&path, b"a line\nb line\nc line\n");
    assert_eq!(texts(&tailer.poll()), vec!["a line", "b line", "c line"]);

    // Same file, rewritten shorter.
    fs::write(&path, "x\n").unwrap();
    let events = tailer.poll();
    assert_eq!(following(&events), vec![(0, Reason::Truncated)]);
    assert_eq!(texts(&events), vec!["x"]);
    assert_eq!(lines(&events)[0].end_offset, 2);
    assert!(!lines(&events)[0].catch_up);
}

#[test]
fn replacement_detected_by_fingerprint() {
    let dir = tempfile::tempdir().unwrap();
    let path = log_in(&dir);
    fs::write(&path, "").unwrap();
    let (mut tailer, _) = Harness::start(Config::new(&path));

    // Grow the file past the fingerprint's 1,024 bytes over several polls;
    // appends never count as a replacement (negative control).
    let row = format!("2026/10/25 10:00:00 {}\n", "x".repeat(80));
    let mut content = String::new();
    for _ in 0..15 {
        append(&path, row.as_bytes());
        content.push_str(&row);
        let events = tailer.poll();
        assert!(
            following(&events).is_empty(),
            "append read as a new file: {events:?}"
        );
        assert_eq!(lines(&events).len(), 1);
    }
    assert!(content.len() > 1024);
    let last = lines(&tailer.poll()).len();
    assert_eq!(last, 0);

    // The game rewrites the file in place: same file, longer, one early byte
    // different, so only the fingerprint can tell.
    let mut rewritten = content.clone().into_bytes();
    rewritten[3] = b'7';
    rewritten.extend_from_slice(b"fresh session\n");
    fs::write(&path, &rewritten).unwrap();
    let events = tailer.poll();
    assert_eq!(following(&events), vec![(0, Reason::Replaced)]);
    assert_eq!(lines(&events).len(), 16);
    assert_eq!(texts(&events).last(), Some(&"fresh session"));
    assert_eq!(lines(&events)[0].fingerprint, Fingerprint::of(&rewritten));
}

#[test]
fn rotation_rename_then_recreate() {
    let dir = tempfile::tempdir().unwrap();
    let path = log_in(&dir);
    let rotated = dir.path().join("Client.1.txt");
    fs::write(&path, "old 1\n").unwrap();
    let (mut tailer, _) = Harness::start(Config::new(&path));
    append(&path, b"old 2\n");
    assert_eq!(texts(&tailer.poll()), vec!["old 2"]);

    // Rename, seen by a poll, then a new file under the old name.
    fs::rename(&path, &rotated).unwrap();
    assert_eq!(tailer.poll(), vec![TailEvent::Waiting]);
    fs::write(&path, "new 1\nnew 2\n").unwrap();
    let events = tailer.poll();
    assert_eq!(following(&events), vec![(0, Reason::Appeared)]);
    assert_eq!(texts(&events), vec!["new 1", "new 2"]);
    assert!(lines(&events).iter().all(|l| !l.catch_up));

    // Rename and recreate between two polls, the new file already longer than
    // the offset read: read from 0, nothing of the old file.
    fs::remove_file(&rotated).unwrap();
    fs::rename(&path, &rotated).unwrap();
    fs::write(&path, "next 1\nnext 2\nnext 3\n").unwrap();
    let events = tailer.poll();
    let restarts = following(&events);
    assert_eq!(restarts.len(), 1, "{events:?}");
    assert_eq!(restarts[0].0, 0);
    // Windows' tunnelling may hand the new file the old creation time; then
    // the fingerprint is what notices.
    assert!(
        matches!(restarts[0].1, Reason::Rotated | Reason::Replaced),
        "{restarts:?}"
    );
    assert_eq!(texts(&events), vec!["next 1", "next 2", "next 3"]);
    // The rotated file was left as it was.
    assert_eq!(fs::read_to_string(&rotated).unwrap(), "new 1\nnew 2\n");
}

#[test]
fn waits_for_missing_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = log_in(&dir);
    let (mut tailer, first) = Harness::start(Config::new(&path));
    assert_eq!(first, vec![TailEvent::Waiting]);
    // Said once, not on every poll.
    assert!(tailer.poll().is_empty());
    assert!(tailer.poll().is_empty());

    fs::write(&path, "hello\n").unwrap();
    let events = tailer.poll();
    assert_eq!(following(&events), vec![(0, Reason::Appeared)]);
    assert_eq!(texts(&events), vec!["hello"]);
    // Written after the tailer started: live, not catch-up.
    assert!(!lines(&events)[0].catch_up);
}

#[test]
fn resumes_from_saved_position() {
    let dir = tempfile::tempdir().unwrap();
    let path = log_in(&dir);
    fs::write(&path, "a\nb\nc\n").unwrap();

    // The position an earlier run saved after handling "b".
    let mut earlier = Vec::new();
    read_whole_file(&path, |line| earlier.push(line)).unwrap();
    let saved = earlier[1].position(&path);
    assert_eq!(saved.offset, 4);
    // It survives the store's JSON round trip.
    let json = serde_json::to_string(&saved).unwrap();
    let saved: Position = serde_json::from_str(&json).unwrap();

    let mut config = Config::new(&path);
    config.saved = Some(saved.clone());
    let (mut tailer, first) = Harness::start(config);
    assert_eq!(following(&first), vec![(4, Reason::Resumed)]);
    assert_eq!(texts(&first), vec!["c"]);
    assert!(lines(&first)[0].catch_up);
    append(&path, b"d\n");
    let events = tailer.poll();
    assert_eq!(texts(&events), vec!["d"]);
    assert!(!lines(&events)[0].catch_up);
    drop(tailer);

    // Negative control: the file at that path is another one now.
    fs::write(&path, "z\ny\n").unwrap();
    let mut config = Config::new(&path);
    config.saved = Some(saved.clone());
    let (_tailer, first) = Harness::start(config);
    assert_eq!(following(&first), vec![(0, Reason::SavedMismatch)]);
    assert_eq!(texts(&first), vec!["z", "y"]);

    // A position saved for another path is not used.
    let mut other = saved;
    other.path = dir.path().join("LatestClient.txt");
    let mut config = Config::new(&path);
    config.saved = Some(other);
    let (_tailer, first) = Harness::start(config);
    assert_eq!(following(&first), vec![(4, Reason::AtEnd)]);
    assert!(lines(&first).is_empty());
}

#[test]
fn catch_up_lines_are_marked() {
    let dir = tempfile::tempdir().unwrap();
    let path = log_in(&dir);
    fs::write(&path, "a\nb\n").unwrap();
    let mut config = Config::new(&path);
    config.from_start = true;
    let (mut tailer, first) = Harness::start(config);
    assert_eq!(following(&first), vec![(0, Reason::FromStart)]);
    assert_eq!(texts(&first), vec!["a", "b"]);
    assert!(lines(&first).iter().all(|l| l.catch_up));
    append(&path, b"c\n");
    let events = tailer.poll();
    assert_eq!(texts(&events), vec!["c"]);
    assert!(!lines(&events)[0].catch_up);

    // Replay mode: every line catch-up, a last line without "\n" included,
    // the position at EOF.
    append(&path, b"d");
    let mut all = Vec::new();
    let end = read_whole_file(&path, |line| all.push(line)).unwrap();
    let all_texts: Vec<&str> = all.iter().map(|l| l.text.as_str()).collect();
    assert_eq!(all_texts, vec!["a", "b", "c", "d"]);
    assert!(all.iter().all(|l| l.catch_up));
    assert_eq!(end.offset, 7);
    assert_eq!(end.fingerprint, Fingerprint::of(b"a\nb\nc\nd"));
}

#[test]
fn delete_and_replace_succeed_while_polling() {
    let dir = tempfile::tempdir().unwrap();
    let path = log_in(&dir);
    let rotated = dir.path().join("Client.1.txt");

    // While the tailer's own read handle is open, the file can be renamed,
    // replaced and deleted.
    fs::write(&path, "held\n").unwrap();
    let held = open_read_only(&path).unwrap();
    fs::rename(&path, &rotated).unwrap();
    fs::write(&path, "replacement\n").unwrap();
    fs::remove_file(&path).unwrap();
    fs::remove_file(&rotated).unwrap();
    fs::write(&path, "recreated\n").unwrap();
    fs::write(&path, "rewritten\n").unwrap();
    drop(held);

    // The thread polling as fast as it can while the file is rewritten,
    // rotated and deleted over and over: every operation succeeds.
    let (tx, rx) = mpsc::channel();
    let stop = Arc::new(AtomicBool::new(false));
    let mut config = Config::new(&path);
    config.poll_interval = Duration::from_millis(1);
    let thread = spawn(config, SystemClock, tx, stop.clone()).unwrap();
    for i in 0..200 {
        append(&path, format!("line {i}\n").as_bytes());
        match i % 4 {
            0 => fs::write(&path, format!("rewrite {i}\n")).unwrap(),
            1 => {
                fs::rename(&path, &rotated).unwrap();
                fs::write(&path, format!("rotated {i}\n")).unwrap();
                fs::remove_file(&rotated).unwrap();
            }
            2 => {
                fs::remove_file(&path).unwrap();
                fs::write(&path, format!("deleted {i}\n")).unwrap();
            }
            _ => {}
        }
    }
    // It still follows the file afterwards.
    fs::remove_file(&path).unwrap();
    fs::write(&path, "the final marker line, longer than any before it\n").unwrap();
    let deadline = Instant::now() + WAIT;
    let mut found = false;
    while Instant::now() < deadline && !found {
        if let Ok(TailEvent::Line(line)) = rx.recv_timeout(Duration::from_millis(50)) {
            found = line.text.starts_with("the final marker");
        }
    }
    stop.store(true, Ordering::SeqCst);
    thread.join().unwrap();
    assert!(found, "the tailer lost the file after the churn");
}

#[cfg(windows)]
fn same_attributes(path: &Path) -> u32 {
    use std::os::windows::fs::MetadataExt;
    fs::metadata(path).unwrap().file_attributes()
}

#[cfg(not(windows))]
fn same_attributes(path: &Path) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path).unwrap().permissions().mode()
}

#[test]
fn opens_read_only() {
    let dir = tempfile::tempdir().unwrap();
    let path = log_in(&dir);
    fs::write(&path, "first\nsecond\n").unwrap();
    let set_readonly = |on: bool| {
        let mut perms = fs::metadata(&path).unwrap().permissions();
        perms.set_readonly(on);
        fs::set_permissions(&path, perms).unwrap();
    };
    // A read-only file: any open that asks for write access fails.
    set_readonly(true);
    assert!(OpenOptions::new().append(true).open(&path).is_err());
    let before = fs::metadata(&path).unwrap();
    let attributes = same_attributes(&path);

    let mut config = Config::new(&path);
    config.from_start = true;
    let (mut tailer, first) = Harness::start(config);
    assert_eq!(texts(&first), vec!["first", "second"]);
    assert!(tailer.poll().is_empty());

    // Reading changed nothing: content, size, write time, attributes.
    let after = fs::metadata(&path).unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), "first\nsecond\n");
    assert_eq!(after.len(), before.len());
    assert_eq!(after.modified().unwrap(), before.modified().unwrap());
    assert_eq!(same_attributes(&path), attributes);

    // New lines in a read-only file are still read.
    set_readonly(false);
    append(&path, b"third\n");
    set_readonly(true);
    assert_eq!(texts(&tailer.poll()), vec!["third"]);
    drop(tailer);
    set_readonly(false);
}

#[test]
fn stop_flag_ends_thread_within_one_poll() {
    let dir = tempfile::tempdir().unwrap();
    let path = log_in(&dir);
    fs::write(&path, "a\n").unwrap();

    // Injected clock: the thread is between polls; the flag is set, the
    // interval ends, and the thread exits without polling again.
    let (mut tailer, _) = Harness::start(Config::new(&path));
    tailer.poll();
    let sleeps = tailer.sleeps.load(Ordering::SeqCst);
    tailer.stop.store(true, Ordering::SeqCst);
    append(&path, b"after stop\n");
    tailer.ticks.as_ref().unwrap().send(()).unwrap();
    let thread = tailer.thread.take().unwrap();
    let (done_tx, done_rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = thread.join();
        let _ = done_tx.send(());
    });
    done_rx.recv_timeout(WAIT).expect("the tailer did not stop");
    assert_eq!(
        tailer.sleeps.load(Ordering::SeqCst),
        sleeps,
        "it slept again after the stop"
    );
    assert!(lines(&tailer.events.try_iter().collect::<Vec<_>>()).is_empty());

    // Real clock: from the flag to the thread's end takes at most one poll
    // interval, plus scheduling slack.
    let interval = Duration::from_millis(200);
    let (tx, rx) = mpsc::channel();
    let stop = Arc::new(AtomicBool::new(false));
    let mut config = Config::new(&path);
    config.poll_interval = interval;
    let thread = spawn(config, SystemClock, tx, stop.clone()).unwrap();
    assert!(matches!(
        rx.recv_timeout(WAIT),
        Ok(TailEvent::Following { .. })
    ));
    let set_at = Instant::now();
    stop.store(true, Ordering::SeqCst);
    thread.join().unwrap();
    let took = set_at.elapsed();
    assert!(
        took < interval + Duration::from_millis(250),
        "stopping took {took:?}"
    );
}
