//! The swap on a real running image: a copy of this test binary, started
//! from a temporary folder and kept running while its file is replaced.

mod common;

use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use common::{wait_until, TempDir};
use ep_update::{old_path, swap_exe};

/// Set to a folder, this test waits there until told to stop: the running
/// image the swap test replaces. Without it, it returns at once.
const HELPER_DIR: &str = "EP_UPDATE_SWAP_HELPER_DIR";

#[test]
fn swap_helper_runs_until_told_to_stop() {
    let Some(dir) = std::env::var_os(HELPER_DIR).map(PathBuf::from) else {
        return;
    };
    fs::write(dir.join("ready"), b"").unwrap();
    let deadline = Instant::now() + Duration::from_secs(60);
    while !dir.join("stop").exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn swap_replaces_a_plain_file() {
    let dir = TempDir::new();
    let exe = dir.path().join("app.exe");
    let staged = dir.path().join("app.exe.download");
    fs::write(&exe, b"old").unwrap();
    fs::write(&staged, b"new").unwrap();
    // A stale `.old` is replaced.
    fs::write(old_path(&exe), b"older").unwrap();
    swap_exe(&exe, &staged).unwrap();
    assert_eq!(fs::read(&exe).unwrap(), b"new");
    assert_eq!(fs::read(old_path(&exe)).unwrap(), b"old");
    assert!(!staged.exists());
}

#[cfg(windows)]
#[test]
fn swap_replaces_a_running_executable() {
    let dir = TempDir::new();
    let exe = dir.path().join("running-app.exe");
    fs::copy(std::env::current_exe().unwrap(), &exe).unwrap();
    let original = fs::read(&exe).unwrap();
    let mut child = std::process::Command::new(&exe)
        .args([
            "swap_helper_runs_until_told_to_stop",
            "--exact",
            "--test-threads=1",
        ])
        .env(HELPER_DIR, dir.path())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    wait_until("the copy to run", || dir.path().join("ready").exists());
    // Baseline: Windows refuses to delete a running image.
    assert!(
        fs::remove_file(&exe).is_err(),
        "a running image was deleted"
    );
    // The swap renames it aside and moves the new file in, while it runs.
    let staged = dir.path().join("running-app.exe.download");
    fs::write(&staged, b"the new version").unwrap();
    swap_exe(&exe, &staged).unwrap();
    assert_eq!(fs::read(&exe).unwrap(), b"the new version");
    assert_eq!(fs::read(old_path(&exe)).unwrap(), original);
    assert!(!staged.exists());
    // The old image is still running, and still cannot be deleted...
    assert!(child.try_wait().unwrap().is_none(), "the copy stopped");
    assert!(fs::remove_file(old_path(&exe)).is_err());
    // ...until it has ended.
    fs::write(dir.path().join("stop"), b"").unwrap();
    assert!(child.wait().unwrap().success());
    wait_until("the old image to be removable", || {
        fs::remove_file(old_path(&exe)).is_ok()
    });
    assert!(!old_path(&exe).exists());
}
