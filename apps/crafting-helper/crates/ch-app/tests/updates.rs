//! The update check and the in-app install, driven through `App` with the
//! recording fake shell, the fake transport, a fake clock and a store and an
//! exe in a temporary folder (`common`'s harness). Nothing here touches the
//! network.
//!
//! Baseline: with the defaults the transport is never called across start,
//! every command and 48 simulated hours of ticks; a manual check through the
//! same fake then makes exactly one call (the positive control). Target: a
//! manual check makes one request; with `check_updates` on, one at start and
//! then one a day; an install starts only from the `install_update`
//! command, never from a check or a tick; a finished install asks the shell
//! to quit, and the quit path's shutdown still saves.

mod common;

use std::fs;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use ch_app::{AssetKind, InstallState, UpdateSetup, UpdateState, CRAFTING_HELPER};
use common::*;
use serde_json::json;

#[test]
fn updates_default_settings_make_no_request_baseline() {
    let h = harness(Setup {
        shell: Some(FakeShell::with_clipboard(ITEM_TEXT)),
        ..Setup::default()
    });
    h.app.start();
    let _ = h.app.get_view();
    h.app.hotkey_pressed();
    let _ = h.app.grab_text(Some(ITEM_TEXT));
    let _ = h.app.set_hotkey("Ctrl+Alt+R");
    let _ = h
        .app
        .update_settings(&json!({ "blocked_currencies": ["divine"] }));
    h.app.open_settings().unwrap();
    let view = h.app.get_app_settings();
    assert!(!view.updates.check_updates);
    assert_eq!(view.updates.update.state, UpdateState::Idle);
    assert_eq!(view.updates.update.text, "Not checked yet.");
    assert_eq!(view.updates.install.state, InstallState::None);
    assert_eq!(
        h.app.install_update().unwrap_err(),
        "no newer version is known"
    );
    let _ = h.app.cancel_update();
    h.ticks(0, 48 * HOUR);
    assert_eq!(h.calls(), 0, "a request nobody asked for");
    assert!(h.downloads().is_empty());
    assert_eq!(
        h.update_lines(),
        lines(&["updates: install refused: no newer version is known"])
    );
    assert_eq!(h.app.get_view().update_available, None);

    // Positive control: the same instrument sees a request once one is asked.
    let view = h.manual(list(&[RUNNING], true));
    assert_eq!(view.updates.update.state, UpdateState::UpToDate);
    assert_eq!(h.calls(), 1);
    assert_eq!(
        h.update_lines()[1..],
        lines(&[
            "updates: check started (manual)",
            "updates: up to date (running 1.2.0, latest 1.2.0)",
        ])
    );
    assert_eq!(h.exe_bytes(), RUNNING_BYTES);
    h.app.shutdown();

    // The shipped default is the crafting helper, ticking on its own.
    let d = UpdateSetup::default();
    assert_eq!(d.product, CRAFTING_HELPER);
    assert!(d.tick_every.is_some());
    assert!(d.config.transport.is_none());
}

#[test]
fn updates_manual_check_makes_one_request_target() {
    let h = harness(Setup {
        list_held: true,
        ..Setup::default()
    });
    h.app.start();
    h.reply(signed_newer());
    let view = h.app.check_for_updates();
    // It returns at once, with the check running.
    assert_eq!(view.updates.update.state, UpdateState::Checking);
    h.wait_calls(1);
    // A second request while one runs starts nothing.
    let _ = h.app.check_for_updates();
    h.hold_list(false);
    let view = h.wait_ended();
    assert_eq!(h.calls(), 1);
    assert_eq!(view.updates.update.state, UpdateState::Available);
    assert_eq!(
        view.updates.update.text,
        "Crafting Helper 1.3.0 is available. You have 1.2.0."
    );
    assert_eq!(view.updates.install.state, InstallState::Ready);
    assert_eq!(view.updates.version, "1.2.0");
    assert_eq!(
        h.update_lines(),
        lines(&[
            "updates: check started (manual)",
            "updates: 1.3.0 available (running 1.2.0)"
        ])
    );
    let sent = h.emitted();
    assert!(sent
        .iter()
        .any(|v| v.updates.update.state == UpdateState::Checking));
    assert_eq!(sent.last(), Some(&view));
    // The main window's view names it.
    assert_eq!(h.app.get_view().update_available.as_deref(), Some("1.3.0"));
    assert_eq!(
        h.main_views().last().unwrap().update_available.as_deref(),
        Some("1.3.0")
    );
    // A manual check schedules nothing and installs nothing.
    h.ticks(0, 48 * HOUR);
    assert_eq!(h.calls(), 1);
    assert!(h.downloads().is_empty());
    assert_eq!(h.exe_bytes(), RUNNING_BYTES);
    h.app.shutdown();
}

#[test]
fn updates_auto_on_checks_at_start_then_daily_target() {
    let h = harness(auto_on());
    h.reply(list(&[RUNNING], true));
    h.app.start();
    h.wait_calls(1);
    let view = h.wait_ended();
    assert_eq!(view.updates.update.state, UpdateState::UpToDate);
    assert!(view.updates.check_updates);
    h.ticks(10 * MINUTE, 24 * HOUR - MINUTE);
    assert_eq!(h.calls(), 1);
    h.at(24 * HOUR);
    h.wait_calls(2);
    h.wait_ended();
    h.app
        .update_settings(&json!({ "check_updates": false }))
        .unwrap();
    h.ticks(24 * HOUR, 120 * HOUR);
    assert_eq!(h.calls(), 2);
    assert!(h.downloads().is_empty());
    h.app.shutdown();
}

#[test]
fn updates_install_is_reached_only_from_the_command_target() {
    let h = harness(auto_on());
    *h.wire.fallback.lock().unwrap() = signed_newer();
    h.app.start();
    h.wait_calls(1);
    let view = h.wait_ended();
    assert_eq!(view.updates.install.state, InstallState::Ready);
    h.ticks(0, 24 * HOUR);
    h.wait_calls(2);
    h.wait_ended();
    h.manual(signed_newer());
    let _ = h.app.cancel_update();
    h.app.open_settings().unwrap();
    h.app
        .update_settings(&json!({ "check_updates": true, "blocked_currencies": ["divine"] }))
        .unwrap();
    assert!(h.downloads().is_empty(), "{:?}", h.downloads());
    assert_eq!(h.exe_bytes(), RUNNING_BYTES);
    assert_eq!(h.folder(), vec![EXE_NAME.to_string()]);

    // The command: the signature first, then the exe.
    h.hold_downloads(true);
    let view = h.app.install_update().unwrap();
    assert_eq!(view.updates.install.state, InstallState::Downloading);
    let calls = h.calls();
    let _ = h.app.check_for_updates();
    h.at(200 * HOUR);
    assert_eq!(h.calls(), calls);
    assert_eq!(h.app.install_update().unwrap_err(), "an install is running");
    h.hold_downloads(false);
    wait_until("the restart request", || h.restarts() == 1);
    assert_eq!(h.downloads(), vec![AssetKind::Signature, AssetKind::Exe]);
    h.app.shutdown();
}

#[test]
fn updates_install_restarts_through_the_quit_path_target() {
    let h = harness(Setup::default());
    h.app.start();
    h.manual(signed_newer());
    // A change the quit path's shutdown must still write.
    h.app
        .update_settings(&json!({ "blocked_currencies": ["divine"] }))
        .unwrap();
    assert!(!h.app.restart_requested());

    h.app.install_update().unwrap();
    wait_until("the restart request", || h.restarts() == 1);
    assert!(h.app.restart_requested());
    assert_eq!(h.exe_bytes(), payload());
    let old = format!("{EXE_NAME}.old");
    assert_eq!(h.folder(), vec![EXE_NAME.to_string(), old.clone()]);
    assert_eq!(fs::read(h.exe.with_file_name(&old)).unwrap(), RUNNING_BYTES);
    let view = h.app.get_app_settings();
    assert_eq!(view.updates.install.state, InstallState::Restarting);
    assert_eq!(
        view.updates.install.text,
        "Crafting Helper 1.3.0 is installed. Restarting\u{2026}"
    );
    assert_eq!(h.app.install_update().unwrap_err(), "the app is restarting");
    assert_eq!(h.restarts(), 1);

    // The quit path: the shell's exit runs the app's shutdown, which saves.
    h.app.shutdown();
    assert_eq!(h.saved_settings()["blocked_currencies"], json!(["divine"]));
    let log = diag_lines(&h.store);
    let pos = |l: &str| {
        log.iter()
            .position(|x| x == l)
            .unwrap_or_else(|| panic!("{l} in {log:?}"))
    };
    assert!(pos("updates: install of 1.3.0 started") < pos("updates: signature of 1.3.0 checked"));
    assert!(pos("updates: replaced the running exe with 1.3.0, restarting") < pos("stop"));
}

#[test]
fn updates_failed_install_leaves_the_exe_and_offers_try_again() {
    let h = harness(Setup::default());
    *h.wire.sig.lock().unwrap() = signing("another-key.minisig");
    h.app.start();
    h.manual(signed_newer());
    h.app.install_update().unwrap();
    wait_until("the install to fail", || {
        h.app.get_app_settings().updates.install.state == InstallState::Failed
    });
    let view = h.app.get_app_settings();
    assert!(view
        .updates
        .install
        .text
        .starts_with("Could not install Crafting Helper 1.3.0: "));
    assert_eq!(h.exe_bytes(), RUNNING_BYTES);
    assert_eq!(h.folder(), vec![EXE_NAME.to_string()]);
    assert_eq!(h.restarts(), 0);
    *h.wire.sig.lock().unwrap() = signing("payload.bin.minisig");
    h.app.install_update().unwrap();
    wait_until("the restart request", || h.restarts() == 1);
    assert_eq!(h.exe_bytes(), payload());
    h.app.shutdown();
}

#[test]
fn updates_cancel_goes_back_to_ready() {
    let h = harness(Setup::default());
    h.app.start();
    h.manual(signed_newer());
    h.hold_downloads(true);
    h.app.install_update().unwrap();
    wait_until("the download to wait", || {
        h.wire.download_waiting.load(Ordering::SeqCst)
    });
    h.app.cancel_update();
    wait_until("the cancelled line", || {
        h.update_lines()
            .contains(&"updates: install cancelled".to_string())
    });
    assert_eq!(
        h.app.get_app_settings().updates.install.state,
        InstallState::Ready
    );
    assert_eq!(h.folder(), vec![EXE_NAME.to_string()]);
    h.app.shutdown();
}

#[test]
fn updates_shutdown_does_not_wait_for_an_install() {
    let h = harness(Setup::default());
    h.app.start();
    h.manual(signed_newer());
    h.hold_downloads(true);
    h.app.install_update().unwrap();
    wait_until("the download to wait", || {
        h.wire.download_waiting.load(Ordering::SeqCst)
    });
    let sent = h.emitted().len();
    let start = Instant::now();
    h.app.shutdown();
    assert!(start.elapsed() < Duration::from_secs(2));
    wait_until("the staging file to go", || {
        h.folder() == vec![EXE_NAME.to_string()]
    });
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(h.emitted().len(), sent);
    assert_eq!(h.restarts(), 0);
    assert_eq!(h.exe_bytes(), RUNNING_BYTES);
}

#[test]
fn open_settings_asks_the_shell() {
    let h = harness(Setup::default());
    h.app.start();
    h.app.open_settings().unwrap();
    h.app.open_settings().unwrap();
    assert_eq!(h.shell.rec().settings_opened, 2);
    h.app.shutdown();
}
