//! The paste after the shortcut's copy, through the recording fake shell:
//! with the game's window in front and every key up, the shortcut writes the
//! clipboard and then sends Ctrl+A and Ctrl+V, one key event at a time; with
//! auto-paste off, another window in front, keys still held, a refused copy
//! or the Copy button, it sends nothing. Pinned texts are compared as
//! literals (Context "The paste design").

mod common;

use std::sync::mpsc;
use std::time::Duration;

use common::*;
use serde_json::{json, Value};
use wr_app::{
    App, Key, StatusKind, CHORD_GAP, GAME_WINDOW_TITLE, KEY_GAP, RELEASE_POLL, RELEASE_TIMEOUT,
    SETTLE,
};

/// The sequence the paste sends.
const SEQUENCE: [(Key, bool); 8] = [
    (Key::Ctrl, true),
    (Key::A, true),
    (Key::A, false),
    (Key::Ctrl, false),
    (Key::Ctrl, true),
    (Key::V, true),
    (Key::V, false),
    (Key::Ctrl, false),
];

/// Wants the first tokened mod; returns the search that builds.
fn select_one(app: &App) -> String {
    let ids = tokened_ids(app);
    let view = app.update_selection(&json!({"want": [ids[0]]})).unwrap();
    assert!(!view.result.regex.is_empty(), "{:?}", view.result);
    view.result.regex
}

fn copied_status(n: usize) -> String {
    format!("Copied the search ({n} characters). Paste it into the stash or a vendor's search box.")
}

fn has_paste_line(diag: &[String]) -> bool {
    diag.iter().any(|l| l.starts_with("paste"))
}

fn no_keyboard_calls(shell: &FakeShell) -> bool {
    !shell.rec().calls.iter().any(|c| {
        matches!(
            c,
            Call::Title | Call::KeysDown(_) | Call::Key(..) | Call::KeyFailed(..) | Call::Pause(_)
        )
    })
}

/// Every key put down was let up again.
fn balanced(keys: &[(Key, bool)]) -> bool {
    [Key::Ctrl, Key::A, Key::V].iter().all(|k| {
        let downs = keys.iter().filter(|(x, d)| x == k && *d).count();
        let ups = keys.iter().filter(|(x, d)| x == k && !*d).count();
        downs == ups
    })
}

#[test]
fn hotkey_with_auto_paste_off_only_copies_baseline() {
    let dir = tempfile::tempdir().unwrap();
    let shell = FakeShell::with_game();
    let app = start(dir.path(), &shell);
    let regex = select_one(&app);
    let view = app.set_auto_paste(false);
    assert!(!view.auto_paste);

    app.hotkey_pressed();

    // Exactly 0.1.0's copy: one write, no key, no pause, no window read.
    assert_eq!(shell.rec().writes, vec![regex.clone()]);
    assert!(no_keyboard_calls(&shell), "{:?}", shell.rec().calls);
    assert_eq!(shell.rec().beeps, 0);
    let status = app.get_view().status.unwrap();
    assert_eq!(status.kind, StatusKind::Info);
    assert_eq!(status.text, copied_status(regex.chars().count()));
    let diag = diag_lines(dir.path());
    let tail: Vec<&str> = diag[diag.len() - 3..].iter().map(|s| s.as_str()).collect();
    assert_eq!(
        tail,
        [
            "copy from the shortcut",
            format!("copied {} characters: {regex}", regex.chars().count()).as_str(),
            "paste skipped: auto-paste is off",
        ]
    );
}

#[test]
fn hotkey_pastes_into_the_game_target() {
    let dir = tempfile::tempdir().unwrap();
    let shell = FakeShell::with_game();
    let app = start(dir.path(), &shell);
    let regex = select_one(&app);
    assert!(app.get_view().auto_paste, "auto-paste is on by default");

    app.hotkey_pressed();

    assert_eq!(shell.keys(), SEQUENCE.to_vec());
    // The clipboard first, then the key check, the settle, the title read
    // right before the first key, and the gaps between the events.
    let calls = shell.rec().calls.clone();
    let k = |key, down| Call::Key(key, down);
    assert_eq!(
        calls,
        vec![
            Call::Clipboard(regex.clone()),
            Call::KeysDown(false),
            Call::Pause(SETTLE),
            Call::Title,
            k(Key::Ctrl, true),
            Call::Pause(KEY_GAP),
            k(Key::A, true),
            Call::Pause(KEY_GAP),
            k(Key::A, false),
            Call::Pause(KEY_GAP),
            k(Key::Ctrl, false),
            Call::Pause(CHORD_GAP),
            k(Key::Ctrl, true),
            Call::Pause(KEY_GAP),
            k(Key::V, true),
            Call::Pause(KEY_GAP),
            k(Key::V, false),
            Call::Pause(KEY_GAP),
            k(Key::Ctrl, false),
        ]
    );
    assert_eq!(GAME_WINDOW_TITLE, "Path of Exile 2");
    assert_eq!(
        (RELEASE_POLL, RELEASE_TIMEOUT, KEY_GAP, SETTLE, CHORD_GAP),
        (
            Duration::from_millis(10),
            Duration::from_millis(2000),
            Duration::from_millis(15),
            Duration::from_millis(30),
            Duration::from_millis(50)
        )
    );
    assert_eq!(shell.rec().beeps, 0);

    let n = regex.chars().count();
    let status = app.get_view().status.unwrap();
    assert_eq!(status.kind, StatusKind::Info);
    assert_eq!(
        status.text,
        format!("Copied the search ({n} characters) and pasted it into the game's search box.")
    );
    let last_view = shell.rec().views.last().cloned().unwrap();
    assert_eq!(last_view.status, Some(status), "the window hears the paste");
    let diag = diag_lines(dir.path());
    let tail: Vec<&str> = diag[diag.len() - 3..].iter().map(|s| s.as_str()).collect();
    assert_eq!(
        tail,
        [
            "copy from the shortcut",
            format!("copied {n} characters: {regex}").as_str(),
            "paste: sent Ctrl+A, Ctrl+V",
        ]
    );
    // The window's title is never written down.
    assert!(
        !diag.iter().any(|l| l.contains(GAME_WINDOW_TITLE)),
        "{diag:?}"
    );
}

#[test]
fn paste_waits_until_every_key_is_up() {
    let dir = tempfile::tempdir().unwrap();
    let shell = FakeShell::with_game();
    shell.rec().held_polls = 5;
    let app = start(dir.path(), &shell);
    select_one(&app);

    app.hotkey_pressed();

    let calls = shell.rec().calls.clone();
    let mut expected = Vec::new();
    for _ in 0..5 {
        expected.push(Call::KeysDown(true));
        expected.push(Call::Pause(RELEASE_POLL));
    }
    expected.push(Call::KeysDown(false));
    expected.push(Call::Pause(SETTLE));
    expected.push(Call::Title);
    // calls[0] is the clipboard write.
    assert_eq!(calls[1..=expected.len()].to_vec(), expected);
    // No key before the keys read as up.
    let last_poll = calls
        .iter()
        .rposition(|c| matches!(c, Call::KeysDown(_)))
        .unwrap();
    let first_key = calls
        .iter()
        .position(|c| matches!(c, Call::Key(..)))
        .unwrap();
    assert!(last_poll < first_key);
    assert_eq!(shell.keys(), SEQUENCE.to_vec());
    assert_eq!(
        diag_lines(dir.path()).last().unwrap(),
        "paste: sent Ctrl+A, Ctrl+V"
    );

    // The edge: keys let go on the last poll the timeout allows still paste.
    let dir = tempfile::tempdir().unwrap();
    let shell = FakeShell::with_game();
    let polls = (RELEASE_TIMEOUT.as_millis() / RELEASE_POLL.as_millis()) as usize;
    shell.rec().held_polls = polls;
    let app = start(dir.path(), &shell);
    select_one(&app);
    app.hotkey_pressed();
    assert_eq!(shell.keys(), SEQUENCE.to_vec());
    assert_eq!(
        diag_lines(dir.path()).last().unwrap(),
        "paste: sent Ctrl+A, Ctrl+V"
    );
}

#[test]
fn keys_held_too_long_skips_the_paste() {
    for held in [usize::MAX, 201] {
        let dir = tempfile::tempdir().unwrap();
        let shell = FakeShell::with_game();
        shell.rec().held_polls = held;
        let app = start(dir.path(), &shell);
        let regex = select_one(&app);

        app.hotkey_pressed();

        let calls = shell.rec().calls.clone();
        assert!(shell.keys().is_empty(), "{calls:?}");
        assert!(
            !calls.contains(&Call::Title),
            "the title is read only after the wait"
        );
        let waited: Duration = calls
            .iter()
            .filter_map(|c| match c {
                Call::Pause(d) => Some(*d),
                _ => None,
            })
            .sum();
        assert_eq!(waited, RELEASE_TIMEOUT, "held={held}");
        // The copy itself stands.
        assert_eq!(shell.rec().writes, vec![regex.clone()]);
        assert_eq!(shell.rec().beeps, 1, "the refusal beep");

        let n = regex.chars().count();
        let status = app.get_view().status.unwrap();
        assert_eq!(status.kind, StatusKind::Error);
        assert_eq!(
            status.text,
            format!(
                "Copied the search ({n} characters), but did not paste it: keys were still held."
            )
        );
        let diag = diag_lines(dir.path());
        let tail: Vec<&str> = diag[diag.len() - 2..].iter().map(|s| s.as_str()).collect();
        assert_eq!(
            tail,
            [
                "paste skipped: keys still held after 2000 ms",
                "refusal beep: tone ok"
            ]
        );
    }
}

#[test]
fn paste_skipped_when_the_game_is_not_in_front() {
    for title in [
        Some("Waystone Roller"),
        None,
        Some("Path of Exile"),
        Some("path of exile 2"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let shell = FakeShell::with_title(title);
        let app = start(dir.path(), &shell);
        let regex = select_one(&app);

        app.hotkey_pressed();

        assert!(
            shell.keys().is_empty(),
            "{title:?}: {:?}",
            shell.rec().calls
        );
        assert!(shell.rec().calls.contains(&Call::Title));
        assert_eq!(shell.rec().writes, vec![regex.clone()]);
        assert_eq!(shell.rec().beeps, 0);
        let status = app.get_view().status.unwrap();
        assert_eq!(status.kind, StatusKind::Info);
        assert_eq!(
            status.text,
            copied_status(regex.chars().count()),
            "the copy's status"
        );
        let diag = diag_lines(dir.path());
        assert_eq!(
            diag.last().unwrap(),
            "paste skipped: the window in front is not Path of Exile 2"
        );
        if let Some(t) = title {
            assert!(!diag
                .iter()
                .any(|l| l.contains(t) && !l.starts_with("paste skipped")));
        }
    }

    // Positive control: the same press with the game in front pastes.
    let dir = tempfile::tempdir().unwrap();
    let shell = FakeShell::with_game();
    let app = start(dir.path(), &shell);
    select_one(&app);
    app.hotkey_pressed();
    assert_eq!(shell.keys(), SEQUENCE.to_vec());
}

#[test]
fn copy_button_never_pastes() {
    let dir = tempfile::tempdir().unwrap();
    let shell = FakeShell::with_game();
    let app = start(dir.path(), &shell);
    let regex = select_one(&app);

    let view = app.copy_now();

    assert_eq!(shell.rec().writes, vec![regex.clone()]);
    assert!(no_keyboard_calls(&shell), "{:?}", shell.rec().calls);
    assert_eq!(
        view.status.unwrap().text,
        copied_status(regex.chars().count())
    );
    let diag = diag_lines(dir.path());
    assert!(!has_paste_line(&diag), "{diag:?}");

    // Negative control: the shortcut in the same app pastes.
    app.hotkey_pressed();
    assert_eq!(shell.keys(), SEQUENCE.to_vec());
}

#[test]
fn refused_copy_never_pastes() {
    // Nothing selected: the copy is refused before the clipboard.
    let dir = tempfile::tempdir().unwrap();
    let shell = FakeShell::with_game();
    let app = start(dir.path(), &shell);
    app.hotkey_pressed();
    assert!(shell.rec().writes.is_empty());
    assert!(no_keyboard_calls(&shell), "{:?}", shell.rec().calls);
    assert_eq!(shell.rec().beeps, 1);
    let diag = diag_lines(dir.path());
    assert!(!has_paste_line(&diag), "{diag:?}");
    assert_eq!(diag.last().unwrap(), "refusal beep: tone ok");

    // The clipboard refuses the write: it holds something else, so nothing
    // is pasted.
    let dir = tempfile::tempdir().unwrap();
    let shell = FakeShell::with_game();
    shell.rec().clipboard_fails = Some("the clipboard is open in another program".to_string());
    let app = start(dir.path(), &shell);
    select_one(&app);
    app.hotkey_pressed();
    assert!(no_keyboard_calls(&shell), "{:?}", shell.rec().calls);
    let status = app.get_view().status.unwrap();
    assert_eq!(status.kind, StatusKind::Error);
    assert!(status.text.starts_with("Not copied: "), "{status:?}");
    let diag = diag_lines(dir.path());
    assert!(!has_paste_line(&diag), "{diag:?}");
}

#[test]
fn paste_failure_releases_every_key_down() {
    // (the send that fails, the key events that went through)
    let cases: [(usize, Vec<(Key, bool)>); 4] = [
        (0, vec![]),
        (1, vec![(Key::Ctrl, true), (Key::Ctrl, false)]),
        (
            2,
            vec![
                (Key::Ctrl, true),
                (Key::A, true),
                (Key::A, false),
                (Key::Ctrl, false),
            ],
        ),
        (
            5,
            vec![
                (Key::Ctrl, true),
                (Key::A, true),
                (Key::A, false),
                (Key::Ctrl, false),
                (Key::Ctrl, true),
                (Key::Ctrl, false),
            ],
        ),
    ];
    for (fail, sent) in cases {
        let dir = tempfile::tempdir().unwrap();
        let shell = FakeShell::with_game();
        shell.rec().fail_send = Some(fail);
        let app = start(dir.path(), &shell);
        let regex = select_one(&app);

        app.hotkey_pressed();

        let keys = shell.keys();
        assert_eq!(keys, sent, "fail at {fail}");
        assert!(balanced(&keys), "fail at {fail}: {keys:?}");
        assert_eq!(
            shell
                .rec()
                .calls
                .iter()
                .filter(|c| matches!(c, Call::KeyFailed(..)))
                .count(),
            1
        );
        // Nothing is sent after the release: the paste stopped.
        let calls = shell.rec().calls.clone();
        let failed = calls
            .iter()
            .position(|c| matches!(c, Call::KeyFailed(..)))
            .unwrap();
        assert!(calls[failed + 1..]
            .iter()
            .all(|c| matches!(c, Call::Key(_, false))));
        assert_eq!(shell.rec().beeps, 1, "the refusal beep");

        let n = regex.chars().count();
        let status = app.get_view().status.unwrap();
        assert_eq!(status.kind, StatusKind::Error);
        assert_eq!(
            status.text,
            format!(
                "Copied the search ({n} characters), but it could not be pasted: {SEND_ERROR}."
            )
        );
        let diag = diag_lines(dir.path());
        let tail: Vec<&str> = diag[diag.len() - 2..].iter().map(|s| s.as_str()).collect();
        assert_eq!(
            tail,
            [
                format!("paste failed: {SEND_ERROR}").as_str(),
                "refusal beep: tone ok"
            ]
        );
    }

    // Failing on an up event: that key is let up again, then the rest.
    let dir = tempfile::tempdir().unwrap();
    let shell = FakeShell::with_game();
    shell.rec().fail_send = Some(2);
    let app = start(dir.path(), &shell);
    select_one(&app);
    app.hotkey_pressed();
    let calls = shell.rec().calls.clone();
    let failed = calls
        .iter()
        .position(|c| matches!(c, Call::KeyFailed(..)))
        .unwrap();
    assert_eq!(calls[failed], Call::KeyFailed(Key::A, false));
    assert_eq!(
        calls[failed + 1..].to_vec(),
        vec![Call::Key(Key::A, false), Call::Key(Key::Ctrl, false)]
    );
}

#[test]
fn second_press_during_a_paste_does_not_paste_twice() {
    let dir = tempfile::tempdir().unwrap();
    let shell = FakeShell::with_game();
    let app = start(dir.path(), &shell);
    let regex = select_one(&app);

    // The first press stops in its first pause until the second is done.
    let (paused_tx, paused_rx) = mpsc::channel::<()>();
    let (resume_tx, resume_rx) = mpsc::channel::<()>();
    let mut first = true;
    shell.on_pause(move |_| {
        if first {
            first = false;
            paused_tx.send(()).unwrap();
            resume_rx.recv().unwrap();
        }
    });
    let pressing = {
        let app = app.clone();
        std::thread::spawn(move || app.hotkey_pressed())
    };
    paused_rx
        .recv_timeout(Duration::from_secs(10))
        .expect("the first press reached its pause");

    // The second press copies as usual (the state lock is free during the
    // pause) and skips its paste.
    app.hotkey_pressed();
    assert_eq!(shell.rec().writes, vec![regex.clone(), regex.clone()]);
    assert!(shell.keys().is_empty(), "the first paste has not sent yet");
    assert_eq!(
        diag_lines(dir.path()).last().unwrap(),
        "paste skipped: a paste is already running"
    );

    resume_tx.send(()).unwrap();
    pressing.join().unwrap();
    assert_eq!(shell.keys(), SEQUENCE.to_vec(), "one paste, sent once");
    let diag = diag_lines(dir.path());
    assert_eq!(
        diag.iter()
            .filter(|l| *l == "paste: sent Ctrl+A, Ctrl+V")
            .count(),
        1
    );
    assert_eq!(
        diag.iter()
            .filter(|l| *l == "paste skipped: a paste is already running")
            .count(),
        1
    );

    // Once it is done, the next press pastes again.
    app.hotkey_pressed();
    assert_eq!(shell.keys().len(), 2 * SEQUENCE.len());
}

#[test]
fn auto_paste_toggle_persists_across_restart() {
    let dir = tempfile::tempdir().unwrap();
    let shell = FakeShell::with_game();
    let app = start(dir.path(), &shell);
    assert!(app.get_view().auto_paste, "on by default");

    let view = app.set_auto_paste(false);
    assert!(!view.auto_paste);
    let status = view.status.unwrap();
    assert_eq!(status.kind, StatusKind::Info);
    assert_eq!(status.text, "The shortcut now only copies.");
    assert_eq!(diag_lines(dir.path()).last().unwrap(), "auto-paste off");
    assert_eq!(shell.rec().views.last().map(|v| v.auto_paste), Some(false));
    app.flush();
    let saved: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.path().join("settings.json")).unwrap())
            .unwrap();
    assert_eq!(saved["auto_paste"], false);
    assert_eq!(saved["schema"], 1);
    app.shutdown();
    drop(app);

    let shell = FakeShell::with_game();
    let app = start(dir.path(), &shell);
    assert!(!app.get_view().auto_paste, "off survives a restart");
    select_one(&app);
    app.hotkey_pressed();
    assert!(shell.keys().is_empty());

    let view = app.set_auto_paste(true);
    assert!(view.auto_paste);
    assert_eq!(
        view.status.unwrap().text,
        "The shortcut now copies and pastes into the game."
    );
    assert_eq!(diag_lines(dir.path()).last().unwrap(), "auto-paste on");
    app.shutdown();
    drop(app);
    let app = start(dir.path(), &FakeShell::with_game());
    assert!(app.get_view().auto_paste, "on survives a restart");
    app.shutdown();
    drop(app);

    // A settings file from before the setting, or with a value that is not
    // a bool, reads as on.
    for saved in [
        json!({"schema": 1, "hotkey": "Ctrl+Alt+W"}),
        json!({"schema": 1, "hotkey": "Ctrl+Alt+W", "auto_paste": "no"}),
        json!({"schema": 1, "hotkey": "Ctrl+Alt+W", "auto_paste": 0}),
    ] {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("settings.json"), saved.to_string()).unwrap();
        let app = start(dir.path(), &FakeShell::with_game());
        assert!(app.get_view().auto_paste, "{saved}");
    }
    // Negative control: a saved `false` reads as off.
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("settings.json"),
        json!({"schema": 1, "auto_paste": false}).to_string(),
    )
    .unwrap();
    assert!(
        !start(dir.path(), &FakeShell::with_game())
            .get_view()
            .auto_paste
    );
}
