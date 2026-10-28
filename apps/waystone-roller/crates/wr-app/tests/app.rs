//! The app's behaviour through a recording fake shell and a store in a
//! temporary folder: the copy path (hotkey and button alike), the shortcut
//! and its conflicts, the selection's persistence and the store's refusal of
//! a git checkout. Pinned texts are compared as literals (Context "Pinned
//! texts").

mod common;

use common::*;
use serde_json::{json, Value};
use wr_app::{
    App, AppConfig, ResultState, StatusKind, Store, StoreError, DEFAULT_HOTKEY, EMBEDDED_DATA,
};
use wr_regex::model::Search;
use wr_regex::{Builder, ModData, Outcome};

#[test]
fn default_selection_copies_nothing_baseline() {
    let dir = tempfile::tempdir().unwrap();
    let shell = FakeShell::with_clipboard("left by another program");
    let app = start(dir.path(), &shell);

    let view = app.get_view();
    assert_eq!(view.result.state, ResultState::Empty);
    assert_eq!(view.result.regex, "");
    assert_eq!(view.hotkey, DEFAULT_HOTKEY);
    assert_eq!(view.hotkey, "Ctrl+Alt+W");

    app.hotkey_pressed();
    let view = app.copy_now();
    {
        let r = shell.rec();
        assert!(
            r.writes.is_empty(),
            "nothing selected, yet wrote {:?}",
            r.writes
        );
        assert_eq!(r.clipboard.as_deref(), Some("left by another program"));
        assert_eq!(r.beeps, 2, "each refused copy beeps once");
    }
    let status = view.status.expect("a refused copy sets a status");
    assert_eq!(status.kind, StatusKind::Error);
    assert_eq!(status.text, "Not copied: nothing is selected.");
    let diag = diag_lines(dir.path());
    assert!(
        diag.contains(&"shortcut Ctrl+Alt+W registered".to_string()),
        "{diag:?}"
    );
    assert_eq!(
        diag.iter()
            .filter(|l| l.starts_with("copy refused: "))
            .count(),
        2,
        "{diag:?}"
    );
    assert!(!diag.iter().any(|l| l.starts_with("copied ")), "{diag:?}");
}

#[test]
fn hotkey_copies_the_built_regex_target() {
    let dir = tempfile::tempdir().unwrap();
    let shell = FakeShell::with_clipboard("old");
    let app = start(dir.path(), &shell);
    let ids = tokened_ids(&app);
    let (a, b) = (ids[0].clone(), ids[1].clone());
    app.update_selection(&json!({
        "want": [a],
        "avoid": [b],
        "rarity": {"normal": false, "magic": false, "rare": true},
        "corruption": "uncorrupted",
    }))
    .unwrap();

    let expected = match Builder::new(fixture_data()).build(&(&app.get_view().selection).into()) {
        Outcome::Ok { regex, .. } => regex,
        other => panic!("{other:?}"),
    };
    app.hotkey_pressed();

    let r = shell.rec();
    assert_eq!(r.writes, vec![expected.clone()]);
    assert_eq!(r.beeps, 0, "a successful copy is silent");
    drop(r);
    let view = app.get_view();
    assert_eq!(view.result.state, ResultState::Ok);
    assert_eq!(view.result.regex, expected);
    let status = view.status.unwrap();
    assert_eq!(status.kind, StatusKind::Info);
    assert_eq!(
        status.text,
        format!(
            "Copied the search ({} characters). Paste it into the stash or a vendor's search box.",
            expected.chars().count()
        )
    );
    let diag = diag_lines(dir.path());
    assert_eq!(
        diag.last().unwrap(),
        &format!("copied {} characters: {expected}", expected.chars().count())
    );

    // The copied string finds a rare uncorrupted item with the wanted mod
    // and not one that also carries the avoided mod (search model).
    let data = fixture_data();
    let fa = data.family(&ids[0]).unwrap();
    let fb = data.family(&ids[1]).unwrap();
    let search = Search::parse(&expected).unwrap();
    let wanted = waystone(&fa.lines, "Rare", 12, false);
    let both: Vec<String> = fa.lines.iter().chain(&fb.lines).cloned().collect();
    assert!(search.highlights(&wanted), "{expected} misses\n{wanted}");
    assert!(!search.highlights(&waystone(&both, "Rare", 12, false)));
    assert!(!search.highlights(&waystone(&fa.lines, "Magic", 12, false)));
    assert!(!search.highlights(&waystone(&fa.lines, "Rare", 12, true)));
}

#[test]
fn hotkey_over_limit_leaves_clipboard_alone_target() {
    let data = ModData::parse(EMBEDDED_DATA).expect("the shipped data loads");
    let dir = tempfile::tempdir().unwrap();
    let shell = FakeShell::with_clipboard("the previous search");
    let app = start_with(data, dir.path(), &shell);
    let every = tokened_ids(&app);
    let view = app
        .update_selection(&json!({"want": every, "want_mode": "all"}))
        .unwrap();
    assert_eq!(view.result.state, ResultState::TooLong, "{:?}", view.result);
    let length = view.result.length;
    assert!(length > 250);
    assert_eq!(view.result.regex, "");
    assert!(!view.result.parts.is_empty());
    assert_eq!(
        view.result.parts.iter().map(|p| p.length).sum::<usize>() + view.result.parts.len() - 1,
        length,
        "the parts and the spaces between them make up the length"
    );

    app.hotkey_pressed();
    let r = shell.rec();
    assert!(r.writes.is_empty(), "{:?}", r.writes);
    assert_eq!(r.clipboard.as_deref(), Some("the previous search"));
    assert_eq!(r.beeps, 1);
    drop(r);
    let status = app.get_view().status.unwrap();
    assert_eq!(status.kind, StatusKind::Error);
    assert_eq!(
        status.text,
        format!("Not copied: the search is {length} characters, over the game's 250. Remove some selections.")
    );
    let diag = diag_lines(dir.path());
    assert!(
        diag.last().unwrap().starts_with("copy refused: "),
        "{diag:?}"
    );

    // Negative control: one wanted mod copies.
    app.update_selection(&json!({"want": [every[0]]})).unwrap();
    app.hotkey_pressed();
    assert_eq!(shell.rec().writes.len(), 1);
}

#[test]
fn copy_button_and_hotkey_copy_the_same_text() {
    let dir = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(dir.path(), &shell);
    let ids = tokened_ids(&app);
    app.update_selection(&json!({
        "want": [ids[0], ids[2]],
        "tier_min": 14,
        "tier_max": 16,
        "rarity": {"normal": false, "magic": true, "rare": true},
    }))
    .unwrap();
    let by_button = app.copy_now();
    app.hotkey_pressed();
    let r = shell.rec();
    assert_eq!(r.writes.len(), 2);
    assert_eq!(r.writes[0], r.writes[1]);
    assert_eq!(r.writes[0], by_button.result.regex);
    assert_eq!(r.beeps, 0);
    drop(r);
    // Both write the same diagnostic line.
    let diag = diag_lines(dir.path());
    let copied: Vec<&String> = diag.iter().filter(|l| l.starts_with("copied ")).collect();
    assert_eq!(copied.len(), 2);
    assert_eq!(copied[0], copied[1]);
}

#[test]
fn taken_shortcut_is_reported_and_app_keeps_running() {
    let dir = tempfile::tempdir().unwrap();
    let shell = FakeShell::with_taken(&["Ctrl+Alt+W"]);
    let app = start(dir.path(), &shell);

    let view = app.get_view();
    assert_eq!(view.hotkey, "Ctrl+Alt+W");
    let status = view.status.expect("a taken shortcut is reported");
    assert_eq!(status.kind, StatusKind::Error);
    assert_eq!(
        status.text,
        "Shortcut Ctrl+Alt+W is taken by another program. Choose another one below."
    );
    let diag = diag_lines(dir.path());
    assert!(
        diag.iter()
            .any(|l| l.starts_with("shortcut Ctrl+Alt+W could not be registered: ")),
        "{diag:?}"
    );
    assert!(!diag.contains(&"shortcut Ctrl+Alt+W registered".to_string()));
    assert!(shell.rec().registered.is_none());

    // The app keeps running: the selection changes and the button copies.
    let ids = tokened_ids(&app);
    let view = app.update_selection(&json!({"want": [ids[0]]})).unwrap();
    assert_eq!(
        view.status.as_ref().map(|s| s.text.as_str()),
        Some("Shortcut Ctrl+Alt+W is taken by another program. Choose another one below."),
        "a selection change keeps the shortcut's warning"
    );
    app.copy_now();
    assert_eq!(shell.rec().writes.len(), 1);

    // Choosing a free shortcut resolves it.
    let view = app.set_hotkey("Ctrl+Alt+Y");
    assert_eq!(view.hotkey, "Ctrl+Alt+Y");
    assert_ne!(view.status.map(|s| s.kind), Some(StatusKind::Error));
    assert_eq!(shell.rec().registered.as_deref(), Some("Ctrl+Alt+Y"));
}

#[test]
fn changing_the_shortcut_keeps_the_old_one_when_refused() {
    let dir = tempfile::tempdir().unwrap();
    let shell = FakeShell::with_taken(&["Ctrl+Alt+Q"]);
    let app = start(dir.path(), &shell);
    assert_eq!(shell.rec().registered.as_deref(), Some("Ctrl+Alt+W"));
    // A change, so the settings file exists before the refusals.
    app.update_selection(&json!({"tier_min": 2})).unwrap();

    let view = app.set_hotkey("Ctrl+Alt+Q");
    assert_eq!(
        view.hotkey, "Ctrl+Alt+W",
        "the refused shortcut is not taken on"
    );
    assert_eq!(shell.rec().registered.as_deref(), Some("Ctrl+Alt+W"));
    let status = view.status.unwrap();
    assert_eq!(status.kind, StatusKind::Error);
    assert_eq!(
        status.text,
        "Shortcut Ctrl+Alt+Q is taken by another program. Choose another one below."
    );
    assert!(diag_lines(dir.path())
        .iter()
        .any(|l| l.starts_with("shortcut Ctrl+Alt+Q could not be registered: ")));

    // Not a shortcut at all: refused, the old one kept, a different reason.
    let view = app.set_hotkey("W");
    assert_eq!(view.hotkey, "Ctrl+Alt+W");
    assert_eq!(view.status.unwrap().kind, StatusKind::Error);

    app.flush();
    let saved: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.path().join("settings.json")).unwrap())
            .unwrap();
    assert_eq!(saved["hotkey"], "Ctrl+Alt+W");

    // Negative control: a free shortcut is taken on and saved.
    let view = app.set_hotkey("Ctrl+Shift+W");
    assert_eq!(view.hotkey, "Ctrl+Shift+W");
    assert_eq!(shell.rec().registered.as_deref(), Some("Ctrl+Shift+W"));
    assert!(diag_lines(dir.path()).contains(&"shortcut Ctrl+Shift+W registered".to_string()));
    app.flush();
    let saved: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.path().join("settings.json")).unwrap())
            .unwrap();
    assert_eq!(saved["hotkey"], "Ctrl+Shift+W");
}

#[test]
fn selection_persists_across_restart() {
    let dir = tempfile::tempdir().unwrap();
    let shell = FakeShell::default();
    let app = start(dir.path(), &shell);
    let ids = tokened_ids(&app);
    let before = app
        .update_selection(&json!({
            "want": [ids[0], ids[1]],
            "want_mode": "all",
            "avoid": [ids[3]],
            "tier_min": 5,
            "tier_max": 12,
            "rarity": {"normal": false, "magic": true, "rare": true},
            "corruption": "corrupted",
        }))
        .unwrap()
        .selection;
    app.set_hotkey("Ctrl+Alt+Z");
    app.shutdown();

    let shell2 = FakeShell::default();
    let app2 = start(dir.path(), &shell2);
    let view = app2.get_view();
    assert_eq!(view.selection, before);
    assert_eq!(view.hotkey, "Ctrl+Alt+Z");
    assert_eq!(shell2.rec().registered.as_deref(), Some("Ctrl+Alt+Z"));

    // A mod in both lists is refused and changes nothing.
    let refused = app2.update_selection(&json!({"avoid": [ids[0]]}));
    assert!(refused.is_err());
    assert_eq!(app2.get_view().selection, before);
    // So is a key the selection does not have.
    assert!(app2.update_selection(&json!({"wants": []})).is_err());
    assert_eq!(app2.get_view().selection, before);
}

#[test]
fn saved_unknown_mod_ids_are_dropped_and_reported() {
    let dir = tempfile::tempdir().unwrap();
    let data = fixture_data();
    let real = data.families[0].id.clone();
    let real2 = data.families[2].id.clone();
    std::fs::write(
        dir.path().join("settings.json"),
        serde_json::to_string_pretty(&json!({
            "schema": 1,
            "hotkey": "Ctrl+Alt+W",
            "selection": {
                "want": ["gone-from-poe2db", real],
                "want_mode": "any",
                "avoid": [real2, "also-gone"],
                "tier_min": 3,
                "tier_max": 9,
                "rarity": {"normal": true, "magic": false, "rare": true},
                "corruption": "any",
            },
        }))
        .unwrap(),
    )
    .unwrap();
    let shell = FakeShell::default();
    let app = start(dir.path(), &shell);
    let view = app.get_view();
    assert_eq!(view.selection.want, vec![real.clone()]);
    assert_eq!(view.selection.avoid, vec![real2.clone()]);
    assert_eq!((view.selection.tier_min, view.selection.tier_max), (3, 9));
    let status = view.status.expect("dropped ids are reported");
    assert!(status.text.contains("gone-from-poe2db"), "{}", status.text);
    assert!(status.text.contains("also-gone"), "{}", status.text);
    let diag = diag_lines(dir.path());
    assert!(
        diag.iter()
            .any(|l| l.contains("gone-from-poe2db") && l.contains("also-gone")),
        "{diag:?}"
    );

    // Negative control: settings naming only known ids report nothing.
    let dir2 = tempfile::tempdir().unwrap();
    std::fs::write(
        dir2.path().join("settings.json"),
        json!({"schema": 1, "hotkey": "Ctrl+Alt+W", "selection": {
            "want": [real], "want_mode": "any", "avoid": [], "tier_min": 1, "tier_max": 16,
            "rarity": {"normal": true, "magic": true, "rare": true}, "corruption": "any"}})
        .to_string(),
    )
    .unwrap();
    let app2 = start(dir2.path(), &FakeShell::default());
    assert_eq!(app2.get_view().status, None);
    assert_eq!(app2.get_view().selection.want, vec![real]);
}

#[test]
fn store_refuses_a_git_checkout() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join(".git")).unwrap();
    let inside = root.path().join("sub").join("store");
    match Store::open(&inside) {
        Err(StoreError::GitCheckout { marker, .. }) => {
            assert_eq!(marker, root.path().join(".git"))
        }
        other => panic!("expected a refusal, got {other:?}"),
    }
    // A `.git` file (a worktree's) is refused too.
    let wt = tempfile::tempdir().unwrap();
    std::fs::write(wt.path().join(".git"), "gitdir: elsewhere\n").unwrap();
    assert!(matches!(
        Store::open(wt.path().join("store")),
        Err(StoreError::GitCheckout { .. })
    ));
    // Negative control: a plain folder is accepted.
    let plain = tempfile::tempdir().unwrap();
    assert!(Store::open(plain.path().join("store")).is_ok());

    // The app runs without a store, says so, and writes nothing there.
    let shell = FakeShell::default();
    let app = App::new(AppConfig {
        data: fixture_data(),
        store: Store::open(&inside),
        shell: Box::new(shell.clone()),
    });
    app.start();
    let status = app.get_view().status.expect("the refusal is reported");
    assert_eq!(status.kind, StatusKind::Error);
    assert!(status.text.contains("git checkout"), "{}", status.text);
    let ids = tokened_ids(&app);
    app.update_selection(&json!({"want": [ids[0]]})).unwrap();
    app.copy_now();
    assert_eq!(shell.rec().writes.len(), 1);
    app.shutdown();
    assert!(!inside.exists(), "nothing is written inside the checkout");
}
