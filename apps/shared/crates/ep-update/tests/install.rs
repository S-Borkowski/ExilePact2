//! The in-app install, against the fake transport and a running exe in a
//! temporary folder, and the start-up cleanup.

mod common;

use std::fs;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use common::{payload, release_entry, wait_until, Setup, TempDir, FIXTURE, NEWER, RUNNING_BYTES};
use ep_update::swap::{cleanup, cleanup_once, Cleaned};
use ep_update::{
    cleanup_line, old_path, spawn_cleanup, staging_path, AssetKind, Begin, DownloadError,
    InstallState, TransportError, CHECK_INTERVAL_MS,
};
use serde_json::json;

const HOUR_MS: u64 = 60 * 60 * 1_000;

/// The folder holds the running exe and nothing else.
fn untouched(s: &Setup) {
    assert_eq!(s.exe_bytes(), RUNNING_BYTES, "the running exe changed");
    assert_eq!(common::names(s.dir.path()), vec!["app.exe".to_string()]);
}

#[test]
fn install_never_runs_without_the_command_baseline() {
    let s = Setup::new();
    // 48 simulated hours of worker ticks with the automatic check on, every
    // minute: the checks run, find the signed newer release, and nothing is
    // ever downloaded.
    let mut checks = 0;
    for minute in 0..=48 * 60u64 {
        let now = minute * 60 * 1_000;
        if let Begin::Run(Some(t)) = s.updater.begin_auto(now, true) {
            checks += 1;
            s.updater.finish(now, false, t.fetch_releases()).unwrap();
        }
    }
    assert_eq!(checks, 3, "at start, after 24 h and after 48 h");
    assert_eq!(48 * HOUR_MS / CHECK_INTERVAL_MS, 2);
    assert_eq!(s.fake.list_calls.load(Ordering::SeqCst), 3);
    assert_eq!(s.fake.download_count(), 0, "a tick downloaded");
    assert_eq!(s.updater.newer(), Some(NEWER));
    assert_eq!(s.updater.install_view().state, InstallState::Ready);
    assert!(s.rec.lines().is_empty());
    untouched(&s);
    // Positive control: the command does download, through the same fake.
    s.install().unwrap();
    s.wait_installed();
    assert_eq!(s.fake.download_count(), 2);
    assert_eq!(s.exe_bytes(), payload());
}

#[test]
fn install_downloads_verifies_replaces_and_asks_to_restart_target() {
    let s = Setup::new();
    s.check();
    assert_eq!(s.updater.install_view().state, InstallState::Ready);
    s.install().unwrap();
    // The command returns at once, downloading.
    assert!(matches!(
        s.updater.install_view().state,
        InstallState::Downloading | InstallState::Verifying | InstallState::Restarting
    ));
    wait_until("the restart request", || {
        s.rec.restarts.load(Ordering::SeqCst) == 1
    });
    s.wait_installed();
    // The signature first, then the exe, at most its listed size.
    assert_eq!(
        *s.fake.downloads.lock().unwrap(),
        vec![
            (NEWER, AssetKind::Signature, ep_update::MAX_SIGNATURE),
            (NEWER, AssetKind::Exe, payload().len() as u64),
        ]
    );
    // The new exe in place, the running one beside it as `.old`, no staging
    // file left.
    assert_eq!(s.exe_bytes(), payload());
    assert_eq!(fs::read(old_path(&s.exe)).unwrap(), RUNNING_BYTES);
    assert!(!staging_path(&s.exe).exists());
    assert_eq!(
        common::names(s.dir.path()),
        vec!["app.exe".to_string(), "app.exe.old".to_string()]
    );
    // What the app saw, in order: downloading, verifying, restarting.
    let states = s.rec.states();
    assert_eq!(
        states.first(),
        Some(&InstallState::Downloading),
        "{states:?}"
    );
    let at = |st| states.iter().position(|x| *x == st).unwrap();
    assert!(at(InstallState::Downloading) < at(InstallState::Verifying));
    assert!(at(InstallState::Verifying) < at(InstallState::Restarting));
    assert_eq!(states.last(), Some(&InstallState::Restarting));
    let v = s.updater.view(false);
    assert_eq!(v.install.state, InstallState::Restarting);
    assert_eq!(
        v.install.text,
        "Fixture App 1.3.0 is installed. Restarting\u{2026}"
    );
    assert_eq!(
        s.rec.lines(),
        vec![
            "updates: install of 1.3.0 started",
            "updates: signature of 1.3.0 checked",
            "updates: replaced the running exe with 1.3.0, restarting",
        ]
    );
    // Restarting: no check and no second install starts.
    assert!(matches!(s.updater.begin_manual(1), Begin::Skip));
    assert_eq!(s.install().unwrap_err().reason, "the app is restarting");
    assert_eq!(s.rec.restarts.load(Ordering::SeqCst), 1);
}

#[test]
fn install_unsigned_release_fails_with_a_reason() {
    let len = payload().len() as u64;
    for assets in [
        vec![("payload.bin", len)],
        vec![("payload.bin.minisig", 310)],
        vec![],
    ] {
        let s = Setup::new();
        let entry = release_entry(&FIXTURE, NEWER, &assets);
        *s.fake.list.lock().unwrap() = serde_json::to_vec(&json!([entry])).unwrap();
        s.check();
        let v = s.updater.view(false);
        assert_eq!(v.update.newer.as_deref(), Some("1.3.0"));
        assert_eq!(v.install.state, InstallState::Unsigned);
        assert_eq!(
            v.install.text,
            "1.3.0 is available but cannot be installed: the release is not signed."
        );
        // The command is refused with the reason, and nothing is requested.
        assert_eq!(s.install().unwrap_err().reason, "the release is not signed");
        assert_eq!(s.fake.download_count(), 0);
        assert_eq!(
            s.rec.lines(),
            vec!["updates: install refused: the release is not signed"]
        );
        assert_eq!(s.updater.view(false), v);
        untouched(&s);
    }
    // Listed as signed, but the signature asset is not there to download:
    // a failed install with the reason, and no exe request.
    let s = Setup::new();
    s.check();
    *s.fake.sig_error.lock().unwrap() = Some(DownloadError::Status(404));
    s.install().unwrap();
    s.wait_installed();
    let v = s.updater.install_view();
    assert_eq!(v.state, InstallState::Failed);
    assert_eq!(
        v.text,
        "Could not install Fixture App 1.3.0: the signature: GitHub answered 404."
    );
    assert_eq!(s.fake.download_count(), 1);
    untouched(&s);
    // With no newer version known, nothing to install.
    let s = Setup::new();
    assert_eq!(s.install().unwrap_err().reason, "no newer version is known");
}

#[test]
fn install_failure_view_offers_only_try_again() {
    let s = Setup::new();
    s.check();
    let mut tampered = payload();
    tampered[100] ^= 0x80;
    *s.fake.exe.lock().unwrap() = tampered;
    s.install().unwrap();
    s.wait_installed();
    let v = s.updater.view(true);
    assert_eq!(v.install.state, InstallState::Failed);
    assert_eq!(
        v.install.text,
        "Could not install Fixture App 1.3.0: the downloaded file does not match its signature."
    );
    assert_eq!(v.install.progress, None);
    // The check part stays as it was (*Check now* is the window's own), and
    // nothing in the view is an address or a page to open.
    assert_eq!(
        v.update.text,
        "Fixture App 1.3.0 is available. You have 1.2.0."
    );
    let json = serde_json::to_string(&v).unwrap();
    for word in ["http", "github.com", "page", "browser", "url"] {
        assert!(!json.to_lowercase().contains(word), "{word} in {json}");
    }
    assert_eq!(
        s.rec.lines().last().unwrap(),
        "updates: install failed: the downloaded file does not match its signature"
    );
    assert_eq!(s.rec.restarts.load(Ordering::SeqCst), 0);
    untouched(&s);
    // *Try again* is the install again: with the genuine bytes it succeeds.
    *s.fake.exe.lock().unwrap() = payload();
    s.install().unwrap();
    wait_until("the restart request", || {
        s.rec.restarts.load(Ordering::SeqCst) == 1
    });
    assert_eq!(s.exe_bytes(), payload());
    // A new check replaces a failure's reason with what it found.
    let s = Setup::new();
    s.check();
    *s.fake.exe_error.lock().unwrap() = Some(DownloadError::Incomplete);
    s.install().unwrap();
    s.wait_installed();
    assert_eq!(s.updater.install_view().state, InstallState::Failed);
    s.check();
    assert_eq!(s.updater.install_view().state, InstallState::Ready);
}

#[test]
fn install_failure_leaves_the_running_file_untouched() {
    type Script = Box<dyn Fn(&Setup)>;
    let short = payload()[..1000].to_vec();
    let cases: Vec<(&str, Script, &str)> = vec![
        (
            "tampered",
            Box::new(|s: &Setup| s.fake.exe.lock().unwrap()[7] ^= 1),
            "the downloaded file does not match its signature",
        ),
        (
            "shorter than listed",
            Box::new(move |s: &Setup| *s.fake.exe.lock().unwrap() = short.clone()),
            "the download's size does not match the release (1000 of 40067 bytes)",
        ),
        (
            "longer than listed",
            Box::new(|s: &Setup| s.fake.exe.lock().unwrap().push(0)),
            "the download was larger than the release lists",
        ),
        (
            "unreachable",
            Box::new(|s: &Setup| {
                *s.fake.exe_error.lock().unwrap() =
                    Some(DownloadError::Transport(TransportError::Unreachable))
            }),
            "GitHub could not be reached",
        ),
        (
            "redirected elsewhere",
            Box::new(|s: &Setup| {
                *s.fake.exe_error.lock().unwrap() = Some(DownloadError::RedirectRefused)
            }),
            "the download was redirected to an unexpected place",
        ),
        (
            "another version's signature",
            Box::new(|s: &Setup| {
                *s.fake.sig.lock().unwrap() = common::fixture("another-version.minisig")
            }),
            "the signature is for another version",
        ),
        (
            "another file's signature",
            Box::new(|s: &Setup| {
                *s.fake.sig.lock().unwrap() = common::fixture("another-file-name.minisig")
            }),
            "the signature is for another file",
        ),
        (
            "another key's signature",
            Box::new(|s: &Setup| {
                *s.fake.sig.lock().unwrap() = common::fixture("another-key.minisig")
            }),
            "the update is signed with another key",
        ),
        (
            "a signature that is not one",
            Box::new(|s: &Setup| *s.fake.sig.lock().unwrap() = b"\xff\xfe not text".to_vec()),
            "the release's signature could not be read",
        ),
    ];
    for (what, script, reason) in cases {
        let s = Setup::new();
        s.check();
        script(&s);
        s.install().unwrap();
        s.wait_installed();
        let v = s.updater.install_view();
        assert_eq!(v.state, InstallState::Failed, "{what}");
        assert_eq!(
            v.text,
            format!("Could not install Fixture App 1.3.0: {reason}."),
            "{what}"
        );
        assert_eq!(s.rec.restarts.load(Ordering::SeqCst), 0, "{what}");
        untouched(&s);
    }
    // A swap that cannot move the new file in puts the running exe back.
    let s = Setup::new();
    let missing = s.dir.path().join("nothing-staged");
    assert!(matches!(
        ep_update::swap_exe(&s.exe, &missing),
        Err(ep_update::swap::SwapError::MoveIn(_))
    ));
    untouched(&s);
}

#[test]
fn install_cancel_removes_the_partial_file() {
    let s = Setup::new();
    s.check();
    s.fake.set_open(false);
    s.install().unwrap();
    wait_until("the download to block", || {
        s.fake.blocked.load(Ordering::SeqCst)
    });
    // Part of the exe is staged beside the running one.
    let staged = staging_path(&s.exe);
    assert!(staged.exists());
    assert!(fs::metadata(&staged).unwrap().len() > 0);
    assert_eq!(s.updater.install_view().state, InstallState::Downloading);
    assert!(s.updater.cancel_install());
    s.wait_installed();
    assert!(!staged.exists());
    // Back to ready, with the button.
    let v = s.updater.install_view();
    assert_eq!(
        (v.state, v.text.as_str()),
        (InstallState::Ready, "Install 1.3.0 and restart")
    );
    assert_eq!(s.rec.lines().last().unwrap(), "updates: install cancelled");
    assert_eq!(s.rec.restarts.load(Ordering::SeqCst), 0);
    untouched(&s);
    // Nothing runs: a cancel is a no-op.
    assert!(!s.updater.cancel_install());
    // A stale staging file from an earlier run is replaced, not refused.
    fs::write(&staged, b"stale").unwrap();
    s.fake.set_open(true);
    s.install().unwrap();
    s.wait_installed();
    assert_eq!(s.exe_bytes(), payload());
    assert!(!staged.exists());
}

#[test]
fn install_unwritable_folder_fails_with_a_reason() {
    // The exe's "folder" is a file, so nothing can be created in it.
    let dir = TempDir::new();
    let blocker = dir.path().join("blocker");
    fs::write(&blocker, b"a file, not a folder").unwrap();
    let exe = blocker.join("app.exe");
    let s = Setup::with_exe(dir, exe);
    s.check();
    s.install().unwrap();
    s.wait_installed();
    let v = s.updater.install_view();
    assert_eq!(v.state, InstallState::Failed);
    assert!(
        v.text.starts_with(
            "Could not install Fixture App 1.3.0: the app's folder cannot be written ("
        ),
        "{}",
        v.text
    );
    // It failed before any request.
    assert_eq!(s.fake.download_count(), 0);
    assert_eq!(common::names(s.dir.path()), vec!["blocker".to_string()]);
    // Control: the same updater in a writable folder downloads.
    let ok = Setup::new();
    ok.check();
    ok.install().unwrap();
    ok.wait_installed();
    assert_eq!(ok.fake.download_count(), 2);
}

#[test]
fn install_progress_is_throttled() {
    // 40,067 reads of one byte each, as fast as they come: each moves the
    // progress, but the app hears about it at most every 250 ms.
    let s = Setup::new();
    s.check();
    *s.fake.chunk.lock().unwrap() = 1;
    let start = Instant::now();
    s.install().unwrap();
    s.wait_installed();
    let elapsed = start.elapsed();
    let states = s.rec.states.lock().unwrap().clone();
    let progress = states
        .iter()
        .filter(|(st, bytes)| *st == InstallState::Downloading && bytes.is_some_and(|b| b > 0))
        .count() as u128;
    let allowed = elapsed.as_millis() / ep_update::PROGRESS_EVERY.as_millis() + 1;
    assert!(
        progress <= allowed,
        "{progress} progress announcements in {elapsed:?}"
    );
    // Every state change is still announced: downloading, verifying,
    // restarting, whatever the throttle did.
    let changes: Vec<_> = s
        .rec
        .states()
        .into_iter()
        .filter(|st| *st != InstallState::Downloading)
        .collect();
    assert_eq!(
        changes,
        vec![InstallState::Verifying, InstallState::Restarting]
    );
    assert_eq!(s.exe_bytes(), payload());
    // Positive control: reads 300 ms apart are each announced.
    let s = Setup::new();
    s.check();
    *s.fake.chunk.lock().unwrap() = 15_000;
    *s.fake.pause.lock().unwrap() = Duration::from_millis(300);
    s.install().unwrap();
    s.wait_installed();
    let shown: Vec<u64> = s
        .rec
        .states
        .lock()
        .unwrap()
        .iter()
        .filter(|(st, _)| *st == InstallState::Downloading)
        .map(|(_, b)| b.expect("a download shows its progress"))
        .collect();
    assert!(shown.len() >= 3, "{shown:?}");
    assert_eq!(shown[0], 0, "the start is announced");
    assert!(
        shown.contains(&30_000) || shown.contains(&40_067),
        "{shown:?}"
    );
}

#[test]
fn install_and_check_are_single_flight() {
    let s = Setup::new();
    s.check();
    s.fake.set_open(false);
    s.install().unwrap();
    wait_until("the download to block", || {
        s.fake.blocked.load(Ordering::SeqCst)
    });
    // While the install runs: no check starts, by hand or by a tick, and a
    // second install is refused.
    assert!(matches!(s.updater.begin_manual(5), Begin::Skip));
    assert!(matches!(
        s.updater.begin_auto(CHECK_INTERVAL_MS * 3, true),
        Begin::Skip
    ));
    assert_eq!(s.install().unwrap_err().reason, "an install is running");
    assert_eq!(
        s.fake.download_count(),
        2,
        "only the first install's two requests"
    );
    s.fake.set_open(true);
    s.wait_installed();

    // While a check runs, no install starts.
    let s = Setup::new();
    s.check();
    let Begin::Run(Some(t)) = s.updater.begin_manual(10) else {
        panic!("the check did not start");
    };
    assert_eq!(s.install().unwrap_err().reason, "a check is running");
    assert_eq!(s.fake.download_count(), 0);
    // A second check is coalesced, as before.
    assert!(matches!(s.updater.begin_manual(11), Begin::Skip));
    s.updater.finish(12, true, t.fetch_releases()).unwrap();
    // Once it ended, the install starts.
    s.install().unwrap();
    s.wait_installed();
    assert_eq!(s.exe_bytes(), payload());
}

#[test]
fn install_shutdown_during_download_changes_nothing() {
    let s = Setup::new();
    s.check();
    s.fake.set_open(false);
    s.install().unwrap();
    wait_until("the download to block", || {
        s.fake.blocked.load(Ordering::SeqCst)
    });
    let staged = staging_path(&s.exe);
    assert!(staged.exists());
    let changed = s.rec.changed.load(Ordering::SeqCst);
    let lines = s.rec.lines();
    let view = s.updater.view(false);
    // Shutdown returns at once, with the download still blocked.
    let start = Instant::now();
    s.updater.stop();
    assert!(start.elapsed() < Duration::from_millis(200));
    // The download sees the cancel, ends, and its staging file goes.
    wait_until("the download to return", || {
        s.fake.returned.load(Ordering::SeqCst) == 2
    });
    wait_until("the staging file to go", || !staged.exists());
    std::thread::sleep(Duration::from_millis(50));
    // Nothing was announced, logged or changed, and the exe is untouched.
    assert_eq!(s.rec.changed.load(Ordering::SeqCst), changed);
    assert_eq!(s.rec.lines(), lines);
    assert_eq!(s.updater.view(false), view);
    assert_eq!(s.rec.restarts.load(Ordering::SeqCst), 0);
    untouched(&s);
    // And nothing starts after it.
    assert!(matches!(s.updater.begin_manual(1), Begin::Skip));
    s.fake.set_open(true);
    assert_eq!(s.install().unwrap_err().reason, "the app is closing");
}

#[test]
fn cleanup_removes_the_previous_version_and_partial_files() {
    let dir = TempDir::new();
    let exe = dir.path().join("app.exe");
    for (name, bytes) in [
        ("app.exe", &b"new"[..]),
        ("app.exe.old", b"previous"),
        ("app.exe.download", b"partial"),
        ("app.exe.old.txt", b"not ours"),
        ("other.exe.old", b"not ours"),
        ("settings.json", b"{}"),
    ] {
        fs::write(dir.path().join(name), bytes).unwrap();
    }
    let result = cleanup_once(&exe);
    assert_eq!(
        result,
        vec![
            Cleaned::Removed(old_path(&exe)),
            Cleaned::Removed(staging_path(&exe)),
        ]
    );
    assert_eq!(
        cleanup_line(&result),
        "updates: cleanup: removed app.exe.old, removed app.exe.download"
    );
    assert_eq!(
        common::names(dir.path()),
        vec![
            "app.exe",
            "app.exe.old.txt",
            "other.exe.old",
            "settings.json"
        ]
    );
    assert_eq!(fs::read(&exe).unwrap(), b"new");
    // A second pass finds nothing.
    assert_eq!(cleanup_once(&exe), vec![]);
    assert_eq!(cleanup_line(&[]), "updates: cleanup: nothing to remove");
    // What cannot be removed is kept, said, and retried until the bound: a
    // folder named like the old exe stands in for a file still in use.
    fs::create_dir(old_path(&exe)).unwrap();
    let start = Instant::now();
    let result = cleanup(&exe, Duration::from_millis(120), Duration::from_millis(20));
    assert!(start.elapsed() >= Duration::from_millis(120));
    assert!(
        matches!(&result[..], [Cleaned::Kept(p, _)] if *p == old_path(&exe)),
        "{result:?}"
    );
    assert!(cleanup_line(&result).starts_with("updates: cleanup: could not remove app.exe.old ("));
    fs::remove_dir(old_path(&exe)).unwrap();
    // On its own thread at start: the caller does not wait, and hears one line.
    fs::write(old_path(&exe), b"previous").unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    spawn_cleanup(exe.clone(), move |line| tx.send(line).unwrap()).unwrap();
    let line = rx.recv_timeout(Duration::from_secs(10)).unwrap();
    assert_eq!(line, "updates: cleanup: removed app.exe.old");
    assert!(!old_path(&exe).exists());
}
