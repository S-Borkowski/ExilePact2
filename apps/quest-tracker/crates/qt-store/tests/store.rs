//! The store's contract, each test against a fresh temporary folder: the
//! injected override stands in for `QUEST_TRACKER_DIR`, and no test reads
//! or sets the process environment's store folder.

use std::fs;
use std::path::{Path, PathBuf};

use qt_store::{
    resolve_dir, safe_name, DiagLog, Refusal, Store, StoreError, DIAG_ROTATE_BYTES, SCHEMA,
};
use serde_json::{json, Value};
use tempfile::TempDir;

fn tmp() -> TempDir {
    tempfile::tempdir().expect("temp dir")
}

fn on_disk(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).expect("read")).expect("json")
}

fn names_in(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = fs::read_dir(dir)
        .expect("read_dir")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

#[test]
fn default_dir_and_env_override() {
    let t = tmp();
    let lad = t.path().join("Local");
    let over = t.path().join("override");

    // The override wins whenever it is set and non-empty.
    assert_eq!(
        resolve_dir(Some(over.clone().into()), Some(lad.clone().into())).unwrap(),
        over
    );
    // Otherwise %LOCALAPPDATA%\ExilePact2\quest-tracker.
    let default = lad.join("ExilePact2").join("quest-tracker");
    assert_eq!(
        resolve_dir(None, Some(lad.clone().into())).unwrap(),
        default
    );
    assert_eq!(
        resolve_dir(Some("".into()), Some(lad.clone().into())).unwrap(),
        default
    );
    // Neither: an error, never the current directory.
    assert!(matches!(
        resolve_dir(None, None),
        Err(StoreError::NoDirectory)
    ));
    assert!(matches!(
        resolve_dir(Some("".into()), Some("".into())),
        Err(StoreError::NoDirectory)
    ));

    // Opening creates nothing; the first write creates the folder.
    let store = Store::open(&over).unwrap();
    assert_eq!(store.dir(), over.as_path());
    assert!(!over.exists());
    store.save_settings(&json!({"opacity": 0.8})).unwrap();
    assert!(over.join("settings.json").is_file());
    assert_eq!(
        store.load_settings().unwrap().value,
        Some(json!({"opacity": 0.8}))
    );
}

#[test]
fn atomic_write_keeps_old_file_on_failure() {
    let t = tmp();
    let store = Store::open(t.path()).unwrap();
    let file = t.path().join("settings.json");
    store.save_settings(&json!({"dock": "right"})).unwrap();
    let before = fs::read(&file).unwrap();

    // A folder where the temporary file must go makes the write fail
    // before the rename: the old file stays byte for byte.
    let tmp_path = t.path().join("settings.json.tmp");
    fs::create_dir(&tmp_path).unwrap();
    let err = store.save_settings(&json!({"dock": "left"})).unwrap_err();
    assert!(matches!(err, StoreError::Io { .. }), "{err}");
    assert_eq!(fs::read(&file).unwrap(), before);
    assert_eq!(
        store.load_settings().unwrap().value,
        Some(json!({"dock": "right"}))
    );

    // Once the obstacle is gone the write goes through and leaves no
    // temporary file behind.
    fs::remove_dir(&tmp_path).unwrap();
    store.save_settings(&json!({"dock": "left"})).unwrap();
    assert_eq!(
        store.load_settings().unwrap().value,
        Some(json!({"dock": "left"}))
    );
    assert!(!tmp_path.exists());
    assert_eq!(names_in(t.path()), vec!["settings.json"]);

    // Every file carries the schema number around the caller's value.
    assert_eq!(
        on_disk(&file),
        json!({"schema": SCHEMA, "data": {"dock": "left"}})
    );
}

#[test]
fn per_character_files() {
    let t = tmp();
    let store = Store::open(t.path()).unwrap();

    let a = json!({"quests": {"a1q1": "done"}});
    let b = json!({"quests": {"a1q1": "available"}});
    // Names that differ only in case get their own files, even on a
    // case-insensitive filesystem.
    store.save_character("Mira_Ranger", &a).unwrap();
    store.save_character("mira_ranger", &b).unwrap();
    assert_eq!(store.load_character("Mira_Ranger").unwrap().value, Some(a));
    assert_eq!(store.load_character("mira_ranger").unwrap().value, Some(b));

    let files = names_in(&t.path().join("characters"));
    assert_eq!(files.len(), 2, "{files:?}");
    assert!(files.contains(&"mira_ranger.json".to_string()), "{files:?}");
    assert_eq!(
        store.character_path("Mira_Ranger").unwrap(),
        t.path().join("characters").join("%4Dira_%52anger.json")
    );

    // A character never saved has no state yet; that is not an error.
    let none = store.load_character("Tavi").unwrap();
    assert_eq!(none.value, None);
    assert_eq!(none.quarantined, None);

    // No name escapes the folder or names a device.
    for (name, want) in [
        ("../x", "%2E%2E%2Fx"),
        ("a\\b", "a%5Cb"),
        ("con", "%63on"),
        ("com1", "%63om1"),
        ("conx", "conx"),
        ("é", "%C3%A9"),
    ] {
        assert_eq!(safe_name(name).unwrap(), want, "{name}");
    }
    assert!(matches!(safe_name(""), Err(StoreError::BadName(_))));
    assert!(store.save_character("", &json!({})).is_err());

    // The roster is its own file.
    let roster =
        json!({"Mira_Ranger": {"class": "Ranger", "level": 12, "own": true, "pinned": false}});
    store.save_characters(&roster).unwrap();
    assert_eq!(store.load_characters().unwrap().value, Some(roster));
    assert!(t.path().join("characters.json").is_file());
}

#[test]
fn refuses_dir_in_game_install_or_git_checkout() {
    let t = tmp();

    // A folder holding the game's executable, and anything under it.
    let game = t.path().join("Path of Exile 2");
    fs::create_dir(&game).unwrap();
    fs::write(game.join("PathOfExile.exe"), b"").unwrap();
    for dir in [game.clone(), game.join("data"), game.join("a").join("b")] {
        match Store::open(&dir) {
            Err(StoreError::Refused {
                reason: Refusal::GameInstall { marker },
                ..
            }) => assert_eq!(marker, game.join("PathOfExile.exe")),
            other => panic!("{}: {other:?}", dir.display()),
        }
    }

    // Anything under a Grinding Gear Games folder, whether or not it exists.
    let ggg = t.path().join("grinding gear games").join("Path of Exile 2");
    assert!(matches!(
        Store::open(&ggg),
        Err(StoreError::Refused {
            reason: Refusal::GameInstall { .. },
            ..
        })
    ));

    // A git checkout, by a .git folder or a worktree's .git file.
    let repo = t.path().join("repo");
    fs::create_dir_all(repo.join(".git")).unwrap();
    let worktree = t.path().join("worktree");
    fs::create_dir(&worktree).unwrap();
    fs::write(worktree.join(".git"), b"gitdir: elsewhere\n").unwrap();
    for (dir, marker) in [
        (repo.clone(), repo.join(".git")),
        (repo.join("store"), repo.join(".git")),
        (worktree.join("a"), worktree.join(".git")),
    ] {
        match Store::open(&dir) {
            Err(StoreError::Refused {
                reason: Refusal::GitCheckout { marker: m },
                ..
            }) => assert_eq!(m, marker),
            other => panic!("{}: {other:?}", dir.display()),
        }
    }

    // Refusing created nothing, and the refusal says why.
    assert!(!game.join("data").exists());
    assert!(!repo.join("store").exists());
    let msg = Store::open(repo.join("store")).unwrap_err().to_string();
    assert!(msg.contains("git checkout"), "{msg}");

    // A plain folder beside them is accepted: the negative control.
    let ok = t.path().join("plain").join("quest-tracker");
    let store = Store::open(&ok).unwrap();
    store.save_settings(&json!({})).unwrap();
    assert!(ok.join("settings.json").is_file());
}

#[test]
fn corrupt_file_is_quarantined() {
    let t = tmp();
    let store = Store::open(t.path()).unwrap();
    let file = t.path().join("settings.json");
    fs::write(&file, b"{\"schema\": 1, \"data\": {\"opa").unwrap();

    let loaded = store.load_settings().unwrap();
    let q = loaded.quarantined.clone().expect("quarantined");
    assert_eq!(loaded.value, None);
    assert_eq!(
        loaded.clone().value_or(json!({"opacity": 1.0})),
        json!({"opacity": 1.0})
    );
    assert_eq!(q.file, file);
    assert!(!file.exists());
    let moved = q
        .moved_to
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    assert!(is_quarantine_name(&moved, "settings.json"), "{moved}");
    assert_eq!(
        fs::read(&q.moved_to).unwrap(),
        b"{\"schema\": 1, \"data\": {\"opa".to_vec()
    );
    // The caller is told, in words for the status line, and so is the log.
    let status = q.status_text();
    assert!(
        status.contains("settings.json") && status.contains(&moved),
        "{status}"
    );
    let log = fs::read_to_string(t.path().join("overlay.log")).unwrap();
    assert!(log.contains(&moved), "{log}");

    // Readable JSON that is not this store's wrapper is quarantined too;
    // a second one in the same second gets its own name.
    fs::write(&file, b"{\"opacity\": 0.5}").unwrap();
    let q2 = store
        .load_settings()
        .unwrap()
        .quarantined
        .expect("quarantined");
    assert_ne!(q2.moved_to, q.moved_to);
    assert!(q2.moved_to.is_file() && q.moved_to.is_file());
    fs::write(&file, b"{\"schema\": 99, \"data\": {}}").unwrap();
    let q3 = store
        .load_settings()
        .unwrap()
        .quarantined
        .expect("quarantined");
    assert!(q3.reason.contains("99"), "{}", q3.reason);

    // A missing file is a first run, not corruption.
    let fresh = store.load_settings().unwrap();
    assert_eq!(fresh.value, None);
    assert_eq!(fresh.quarantined, None);

    // A character's file is handled the same way; saving afterwards works.
    let cpath = store.character_path("Tavi").unwrap();
    fs::create_dir_all(cpath.parent().unwrap()).unwrap();
    fs::write(&cpath, b"\xff\xfe not json").unwrap();
    assert!(store.load_character("Tavi").unwrap().quarantined.is_some());
    store.save_character("Tavi", &json!({"ok": true})).unwrap();
    assert_eq!(
        store.load_character("Tavi").unwrap().value,
        Some(json!({"ok": true}))
    );
}

/// `<file>.corrupt-YYYYmmdd-HHMMSS`, optionally with a `-<n>` suffix.
fn is_quarantine_name(name: &str, file: &str) -> bool {
    let Some(rest) = name.strip_prefix(&format!("{file}.corrupt-")) else {
        return false;
    };
    let stamp = rest.get(..15).unwrap_or("");
    let b = stamp.as_bytes();
    b.len() == 15
        && b[8] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 8 || c.is_ascii_digit())
        && (rest.len() == 15 || rest[15..].starts_with('-'))
}

#[test]
fn log_position_roundtrip() {
    let t = tmp();
    let store = Store::open(t.path()).unwrap();
    assert_eq!(store.load_position().unwrap().value, None);

    let pos = json!({
        "path": "C:/Games/PoE2/logs/Client.txt",
        "fingerprint": "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08",
        "offset": u64::MAX - 7,
    });
    store.save_position(&pos).unwrap();
    let back = store.load_position().unwrap();
    assert_eq!(back.quarantined, None);
    let back = back.value.unwrap();
    assert_eq!(back, pos);
    assert_eq!(back["offset"].as_u64(), Some(u64::MAX - 7));
    assert_eq!(
        on_disk(&t.path().join("position.json"))["schema"],
        json!(SCHEMA)
    );

    // A later save replaces it whole.
    let pos2 = json!({"path": "C:/Games/PoE2/logs/Client.txt", "fingerprint": "00", "offset": 0});
    store.save_position(&pos2).unwrap();
    assert_eq!(store.load_position().unwrap().value, Some(pos2));
}

/// A provisional record as the engine hands it out: what the store keeps is
/// opaque to it, so any JSON value stands in.
fn new_character_record() -> Value {
    json!({
        "state": {"quests": {"a1q1": "in_progress"}},
        "events": 3,
        "released_pin": "Mira_Ranger",
        "seed": 1234567,
        "open": true,
    })
}

#[test]
fn new_character_file_roundtrip() {
    let t = tmp();
    let store = Store::open(t.path()).unwrap();
    let file = t.path().join("new-character.json");
    assert_eq!(store.new_character_path(), file);

    // No record yet: nothing, and that is not an error.
    let none = store.load_new_character().unwrap();
    assert_eq!(none.value, None);
    assert_eq!(none.quarantined, None);

    let rec = new_character_record();
    store.save_new_character(&rec).unwrap();
    let back = store.load_new_character().unwrap();
    assert_eq!(back.quarantined, None);
    assert_eq!(back.value, Some(rec.clone()));
    // The same wrapper and schema number as every other file, written
    // atomically: no temporary file is left behind.
    assert_eq!(on_disk(&file), json!({"schema": SCHEMA, "data": rec}));
    assert_eq!(names_in(t.path()), vec!["new-character.json"]);

    // A later save replaces the record whole (a parked record).
    let parked = json!({"state": {}, "events": 0, "released_pin": null, "seed": 7, "open": false});
    store.save_new_character(&parked).unwrap();
    assert_eq!(store.load_new_character().unwrap().value, Some(parked));

    // It is its own file: no character file and no roster entry appear,
    // and the roster's file is not touched.
    store.save_characters(&json!({})).unwrap();
    assert!(!t.path().join("characters").exists());
    assert_eq!(store.load_characters().unwrap().value, Some(json!({})));
}

#[test]
fn new_character_file_cleared() {
    let t = tmp();
    let store = Store::open(t.path()).unwrap();
    let file = t.path().join("new-character.json");

    store.save_settings(&json!({"dock": "left"})).unwrap();
    store.save_new_character(&new_character_record()).unwrap();
    assert!(file.is_file());

    // Clearing removes the file; loading then returns no record.
    store.clear_new_character().unwrap();
    assert!(!file.exists());
    let none = store.load_new_character().unwrap();
    assert_eq!(none.value, None);
    assert_eq!(none.quarantined, None);
    // Nothing else in the folder went with it.
    assert_eq!(names_in(t.path()), vec!["settings.json"]);
    assert_eq!(
        store.load_settings().unwrap().value,
        Some(json!({"dock": "left"}))
    );

    // Clearing when there is no record (or no folder yet) is not an error.
    store.clear_new_character().unwrap();
    let fresh = Store::open(t.path().join("never-written")).unwrap();
    fresh.clear_new_character().unwrap();
    assert!(!t.path().join("never-written").exists());

    // A record saved after a clear is read back as usual.
    store.save_new_character(&json!({"open": true})).unwrap();
    assert_eq!(
        store.load_new_character().unwrap().value,
        Some(json!({"open": true}))
    );
}

#[test]
fn corrupt_new_character_file_is_quarantined() {
    let t = tmp();
    let store = Store::open(t.path()).unwrap();
    let file = t.path().join("new-character.json");
    fs::write(&file, b"{\"schema\": 1, \"data\": {\"ope").unwrap();

    let loaded = store.load_new_character().unwrap();
    assert_eq!(loaded.value, None);
    let q = loaded.quarantined.expect("quarantined");
    assert_eq!(q.file, file);
    assert!(!file.exists());
    let moved = q
        .moved_to
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    assert!(is_quarantine_name(&moved, "new-character.json"), "{moved}");
    assert_eq!(
        fs::read(&q.moved_to).unwrap(),
        b"{\"schema\": 1, \"data\": {\"ope".to_vec()
    );
    // Told in words for the status line, and in the diagnostic log.
    let status = q.status_text();
    assert!(
        status.contains("new-character.json") && status.contains(&moved),
        "{status}"
    );
    let log = fs::read_to_string(t.path().join("overlay.log")).unwrap();
    assert!(log.contains(&moved), "{log}");

    // Another schema is not read either.
    fs::write(&file, b"{\"schema\": 99, \"data\": {}}").unwrap();
    let q2 = store
        .load_new_character()
        .unwrap()
        .quarantined
        .expect("quarantined");
    assert!(q2.reason.contains("99"), "{}", q2.reason);

    // Then the slot is free: no record, and saving afterwards works.
    assert_eq!(
        store.load_new_character().unwrap(),
        qt_store::Loaded {
            value: None,
            quarantined: None,
        }
    );
    store.save_new_character(&new_character_record()).unwrap();
    assert_eq!(
        store.load_new_character().unwrap().value,
        Some(new_character_record())
    );
}

#[test]
fn diag_log_rotates_at_one_mb() {
    let t = tmp();
    let store = Store::open(t.path()).unwrap();
    let log = store.diag();
    let path: PathBuf = t.path().join("overlay.log");
    let old = t.path().join("overlay.log.1");
    assert_eq!(log.path(), path.as_path());
    assert_eq!(log.old_path(), old);
    assert_eq!(DIAG_ROTATE_BYTES, 1024 * 1024);

    log.write("first line\nwith a break").unwrap();
    let first = fs::read_to_string(&path).unwrap();
    assert_eq!(first.lines().count(), 1, "{first:?}");
    assert!(first.trim_end().ends_with("first line with a break"));

    // Just under 1 MB: no rotation yet.
    let chunk = "x".repeat(4000);
    while fs::metadata(&path).unwrap().len() + 4030 <= DIAG_ROTATE_BYTES {
        log.write(&chunk).unwrap();
    }
    assert!(!old.exists());

    // The line that would cross 1 MB starts a new file; the full one is
    // kept as the single old copy.
    let mut n = 0;
    while !old.exists() {
        log.write(&chunk).unwrap();
        n += 1;
        assert!(n < 10, "never rotated");
    }
    assert!(fs::metadata(&old).unwrap().len() <= DIAG_ROTATE_BYTES);
    assert!(fs::read_to_string(&old).unwrap().contains("first line"));
    assert!(fs::metadata(&path).unwrap().len() < 8000);

    // Fill and cross again: still one old copy, and the oldest lines gone.
    let mut rotations = 0;
    let mut last = fs::metadata(&path).unwrap().len();
    for _ in 0..1000 {
        log.write(&chunk).unwrap();
        let len = fs::metadata(&path).unwrap().len();
        assert!(len <= DIAG_ROTATE_BYTES);
        if len < last {
            rotations += 1;
            break;
        }
        last = len;
    }
    assert_eq!(rotations, 1, "never rotated a second time");
    log.write("marker").unwrap();
    assert!(!fs::read_to_string(&old).unwrap().contains("first line"));
    assert!(fs::metadata(&path).unwrap().len() <= DIAG_ROTATE_BYTES);
    assert!(fs::read_to_string(&path).unwrap().contains("marker"));
    assert_eq!(
        names_in(t.path()),
        vec!["overlay.log".to_string(), "overlay.log.1".to_string()]
    );

    // A standalone log with a small limit rotates the same way.
    let small = DiagLog::with_limit(t.path().join("small.log"), 100);
    for _ in 0..5 {
        small.write(&"y".repeat(30)).unwrap();
    }
    assert!(fs::metadata(small.path()).unwrap().len() <= 100);
    assert!(small.old_path().is_file());
}
