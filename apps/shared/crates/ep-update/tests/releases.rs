//! The release-list parse, for each product, and the view's texts.

mod common;

use common::release_entry;
use ep_update::{
    app_settings_view, newest_release, read_answer, Answer, InstallPhase, InstallState, Outcome,
    Product, Release, ReleaseAsset, TransportError, Unreadable, UpdateState, Version,
    CRAFTING_HELPER, QUEST_TRACKER, WAYSTONE_ROLLER,
};
use serde_json::{json, Value};

const PRODUCTS: [Product; 3] = [QUEST_TRACKER, WAYSTONE_ROLLER, CRAFTING_HELPER];

fn list(entries: Vec<Value>) -> Vec<u8> {
    serde_json::to_vec(&Value::Array(entries)).unwrap()
}

/// The next product in [`PRODUCTS`], round the end: each product is some
/// other product's `other`, so every pairing a test makes runs both ways.
fn other(p: &Product) -> Product {
    let i = PRODUCTS.iter().position(|q| q == p).unwrap();
    PRODUCTS[(i + 1) % PRODUCTS.len()]
}

/// Every product but `p`.
fn others(p: &Product) -> Vec<Product> {
    PRODUCTS.into_iter().filter(|q| q != p).collect()
}

fn newest(p: &Product, body: &[u8]) -> Result<Option<Version>, Unreadable> {
    newest_release(p, body).map(|r| r.map(|r| r.version))
}

#[test]
fn releases_pick_the_newest_published_tag_of_the_product() {
    for p in PRODUCTS {
        let o = other(&p);
        // 1.10.0 is newer than 1.9.3 as numbers, not as text; the other
        // product's 9.9.9 is not this product's release.
        let mut entries = vec![
            release_entry(&p, Version::new(1, 9, 3), &[]),
            release_entry(&p, Version::new(1, 10, 0), &[]),
            release_entry(&o, Version::new(9, 9, 9), &[]),
            release_entry(&p, Version::new(0, 1, 0), &[]),
        ];
        assert_eq!(
            newest(&p, &list(entries.clone())),
            Ok(Some(Version::new(1, 10, 0)))
        );
        assert_eq!(
            newest(&o, &list(entries.clone())),
            Ok(Some(Version::new(9, 9, 9)))
        );
        // The order of the list does not matter.
        entries.reverse();
        assert_eq!(newest(&p, &list(entries)), Ok(Some(Version::new(1, 10, 0))));
        // An empty list has none.
        assert_eq!(newest(&p, b"[]"), Ok(None));
        // The tag is the prefix and the version, both ways.
        let v = Version::new(1, 10, 0);
        assert_eq!(p.tag(v), format!("{}1.10.0", p.tag_prefix));
        assert_eq!(p.version_of_tag(&p.tag(v)), Some(v));
        assert_eq!(o.version_of_tag(&p.tag(v)), None);
        // Against the running version: newer is available, equal or older is
        // up to date.
        let body = list(vec![release_entry(&p, v, &[])]);
        let answer = || {
            Ok(Answer {
                status: 200,
                body: body.clone(),
            })
        };
        assert_eq!(
            read_answer(&p, answer(), Version::new(1, 9, 3)).0,
            Outcome::Available { newer: v }
        );
        assert_eq!(
            read_answer(&p, answer(), v).0,
            Outcome::UpToDate { latest: v }
        );
        assert_eq!(
            read_answer(&p, answer(), Version::new(2, 0, 0)),
            (Outcome::UpToDate { latest: v }, None)
        );
    }
    assert_eq!(
        QUEST_TRACKER.tag(Version::new(0, 2, 0)),
        "quest-tracker-v0.2.0"
    );
    assert_eq!(
        WAYSTONE_ROLLER.tag(Version::new(0, 2, 0)),
        "waystone-roller-v0.2.0"
    );
    assert_eq!(
        CRAFTING_HELPER.tag(Version::new(0, 1, 0)),
        "crafting-helper-v0.1.0"
    );
    assert_eq!(CRAFTING_HELPER.name, "Crafting Helper");
    // `Crafting Helper.exe`, under the name GitHub stores it as.
    assert_eq!(CRAFTING_HELPER.exe_name, "Crafting.Helper.exe");
    assert_eq!(
        CRAFTING_HELPER.signature_name(),
        "Crafting.Helper.exe.minisig"
    );
    // An app's own package version, read up to a suffix.
    assert_eq!(Version::of_package("0.2.0"), Version::new(0, 2, 0));
    assert_eq!(Version::of_package("0.2.0-dev+abc"), Version::new(0, 2, 0));
    assert_eq!(Version::of_package("garbage"), Version::new(0, 0, 0));
}

#[test]
fn releases_ignore_drafts_prereleases_other_tools_and_bad_tags() {
    let p = QUEST_TRACKER;
    let one = |entry: Value| newest(&p, &list(vec![entry]));
    let published = |tag: &str| json!({ "tag_name": tag, "draft": false, "prerelease": false });
    // Positive control: a plain published tag is found.
    assert_eq!(
        one(published("quest-tracker-v2.0.0")),
        Ok(Some(Version::new(2, 0, 0)))
    );
    let skipped = [
        json!({ "tag_name": "quest-tracker-v2.0.0", "draft": true, "prerelease": false }),
        json!({ "tag_name": "quest-tracker-v2.0.0", "draft": false, "prerelease": true }),
        json!({ "tag_name": "quest-tracker-v2.0.0", "prerelease": false }),
        json!({ "tag_name": "quest-tracker-v2.0.0", "draft": "false", "prerelease": false }),
        published("waystone-roller-v2.0.0"),
        published("crafting-helper-v2.0.0"),
        published("log-reader-v2.0.0"),
        published("v2.0.0"),
        published("Quest-Tracker-v2.0.0"),
        published("quest-tracker-v2.0"),
        published("quest-tracker-v2.0.0.1"),
        published("quest-tracker-v2.0.0-rc1"),
        published("quest-tracker-v+2.0.0"),
        published("quest-tracker-v 2.0.0"),
        published("quest-tracker-v2..0"),
        published("quest-tracker-vX.Y.Z"),
        published("quest-tracker-v99999999999999999999.0.0"),
        json!({ "tag_name": 200, "draft": false, "prerelease": false }),
        json!({ "draft": false, "prerelease": false }),
        json!("quest-tracker-v2.0.0"),
        Value::Null,
    ];
    for entry in skipped {
        assert_eq!(one(entry.clone()), Ok(None), "{entry}");
    }
    // The same for the waystone roller's tags.
    let w = WAYSTONE_ROLLER;
    let wl = |tag: &str| newest(&w, &list(vec![published(tag)]));
    assert_eq!(
        wl("waystone-roller-v0.3.1"),
        Ok(Some(Version::new(0, 3, 1)))
    );
    for tag in [
        "quest-tracker-v0.3.1",
        "crafting-helper-v0.3.1",
        "waystone-roller-v0.3",
        "Waystone-Roller-v0.3.1",
    ] {
        assert_eq!(wl(tag), Ok(None), "{tag}");
    }
    // And for the crafting helper's.
    let c = CRAFTING_HELPER;
    let cl = |tag: &str| newest(&c, &list(vec![published(tag)]));
    assert_eq!(
        cl("crafting-helper-v0.1.0"),
        Ok(Some(Version::new(0, 1, 0)))
    );
    for tag in [
        "quest-tracker-v0.1.0",
        "waystone-roller-v0.1.0",
        "crafting-helper-v0.1",
        "Crafting-Helper-v0.1.0",
        "crafting-helper-0.1.0",
    ] {
        assert_eq!(cl(tag), Ok(None), "{tag}");
    }
}

#[test]
fn releases_refuse_an_unreadable_answer() {
    for body in [
        &b""[..],
        b"not json",
        b"[{\"tag_name\": \"quest-tracker-v2.0.0\"",
        b"{\"message\": \"Not Found\"}",
        b"\"quest-tracker-v2.0.0\"",
        b"null",
        b"\xff\xfe[]",
    ] {
        for p in PRODUCTS {
            assert_eq!(
                newest_release(&p, body),
                Err(Unreadable),
                "{}",
                String::from_utf8_lossy(body)
            );
        }
    }
    let p = QUEST_TRACKER;
    let running = Version::new(1, 2, 0);
    let failed = |answer: Result<Answer, TransportError>| match read_answer(&p, answer, running) {
        (Outcome::Failed { reason }, None) => reason,
        other => panic!("not a failure: {other:?}"),
    };
    let status = |status: u16| {
        Ok(Answer {
            status,
            body: b"[]".to_vec(),
        })
    };
    assert_eq!(
        failed(Ok(Answer {
            status: 200,
            body: b"{\"message\": \"Not Found\"}".to_vec()
        })),
        "GitHub's answer could not be read"
    );
    assert_eq!(
        failed(status(403)),
        "GitHub refused the check for now (rate limit), try again later"
    );
    assert_eq!(
        failed(status(429)),
        "GitHub refused the check for now (rate limit), try again later"
    );
    assert_eq!(failed(status(500)), "GitHub answered 500");
    assert_eq!(failed(status(302)), "GitHub answered 302");
    assert_eq!(
        failed(Err(TransportError::Unsupported)),
        "this build cannot check for updates"
    );
    assert_eq!(
        failed(Err(TransportError::Timeout)),
        "GitHub did not answer in time"
    );
    // Control: a readable empty list is no release, not a failure.
    assert_eq!(
        read_answer(&p, status(200), running),
        (Outcome::NoRelease, None)
    );
}

#[test]
fn releases_list_the_newest_releases_signed_assets() {
    let p = WAYSTONE_ROLLER;
    let v = Version::new(0, 2, 0);
    let exe = ("waystone-roller.exe", 7_654_321);
    let sig = ("waystone-roller.exe.minisig", 310);
    let asset = |(name, size): (&str, u64)| {
        Some(ReleaseAsset {
            name: name.to_string(),
            size,
        })
    };
    // Positive control: both assets, with their sizes, among others.
    let body = list(vec![release_entry(
        &p,
        v,
        &[
            ("waystone-roller-0.2.0-sha256.txt", 90),
            exe,
            sig,
            ("quest-tracker.exe", 5),
        ],
    )]);
    let r = newest_release(&p, &body).unwrap().unwrap();
    assert_eq!(
        r,
        Release {
            version: v,
            exe: asset(exe),
            signature: asset(sig)
        }
    );
    assert!(r.is_signed());
    // Only the newest release's assets count: an older signed one does not
    // sign the newer.
    let body = list(vec![
        release_entry(&p, Version::new(0, 1, 9), &[exe, sig]),
        release_entry(&p, v, &[exe]),
    ]);
    let r = newest_release(&p, &body).unwrap().unwrap();
    assert_eq!(
        (r.version, r.exe.clone(), r.signature.clone()),
        (v, asset(exe), None)
    );
    assert!(!r.is_signed());
    // No assets at all, or no exe: not signed.
    let r = newest_release(&p, &list(vec![release_entry(&p, v, &[])]))
        .unwrap()
        .unwrap();
    assert!(!r.is_signed());
    let r = newest_release(&p, &list(vec![release_entry(&p, v, &[sig])]))
        .unwrap()
        .unwrap();
    assert_eq!((r.exe, r.signature.is_some()), (None, true));
    // An asset counts only by its exact name, a whole size and an upload
    // that finished.
    let entry_with = |assets: Value| {
        let mut e = release_entry(&p, v, &[]);
        e["assets"] = assets;
        list(vec![e])
    };
    for bad in [
        json!([{ "name": "Waystone-Roller.exe", "size": 1 }]),
        json!([{ "name": "waystone-roller.exe ", "size": 1 }]),
        json!([{ "name": "waystone-roller.exe", "size": "1" }]),
        json!([{ "name": "waystone-roller.exe", "size": -1 }]),
        json!([{ "name": "waystone-roller.exe" }]),
        json!([{ "name": "waystone-roller.exe", "size": 1, "state": "open" }]),
        json!({ "name": "waystone-roller.exe", "size": 1 }),
        json!("waystone-roller.exe"),
    ] {
        let r = newest_release(&p, &entry_with(bad.clone()))
            .unwrap()
            .unwrap();
        assert_eq!(r.exe, None, "{bad}");
    }
    let r = newest_release(
        &p,
        &entry_with(json!([{ "name": "waystone-roller.exe", "size": 3 }])),
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        r.exe,
        asset(("waystone-roller.exe", 3)),
        "no state listed is taken"
    );
    // A `.sig` is not a `.minisig`.
    let body = list(vec![release_entry(
        &p,
        v,
        &[exe, ("waystone-roller.exe.sig", 400)],
    )]);
    assert!(!newest_release(&p, &body).unwrap().unwrap().is_signed());
    // Through the answer: only an available release carries its assets.
    let body = list(vec![release_entry(&p, v, &[exe, sig])]);
    let answer = || {
        Ok(Answer {
            status: 200,
            body: body.clone(),
        })
    };
    assert!(read_answer(&p, answer(), Version::new(0, 1, 0))
        .1
        .unwrap()
        .is_signed());
    assert_eq!(read_answer(&p, answer(), v).1, None);
}

/// From 0.2.0 a crafting helper release carries its exe twice: under the
/// renamed `Crafting.Helper.exe`, which this build looks up, and under the
/// legacy `crafting-helper.exe`, which 0.1.x copies look up.
#[test]
fn a_crafting_helper_release_lists_the_renamed_exe_beside_the_legacy_one() {
    let p = CRAFTING_HELPER;
    let v = Version::new(0, 2, 0);
    let exe = ("Crafting.Helper.exe", 9_000_000);
    let sig = ("Crafting.Helper.exe.minisig", 320);
    let legacy_exe = ("crafting-helper.exe", 9_000_000);
    let legacy_sig = ("crafting-helper.exe.minisig", 318);
    let asset = |(name, size): (&str, u64)| {
        Some(ReleaseAsset {
            name: name.to_string(),
            size,
        })
    };
    let body = list(vec![release_entry(
        &p,
        v,
        &[legacy_exe, legacy_sig, exe, ("SHA256SUMS.txt", 200), sig],
    )]);
    let r = newest_release(&p, &body).unwrap().unwrap();
    assert_eq!(
        r,
        Release {
            version: v,
            exe: asset(exe),
            signature: asset(sig)
        }
    );
    assert!(r.is_signed());
    // Negative control: a release with only the legacy names (0.1.x) has
    // no asset this build installs.
    let body = list(vec![release_entry(&p, v, &[legacy_exe, legacy_sig])]);
    let r = newest_release(&p, &body).unwrap().unwrap();
    assert_eq!((r.exe, r.signature), (None, None));
    // The built file's own name, with its space, is not the stored name.
    let body = list(vec![release_entry(
        &p,
        v,
        &[
            ("Crafting Helper.exe", 1),
            ("Crafting Helper.exe.minisig", 1),
        ],
    )]);
    assert!(!newest_release(&p, &body).unwrap().unwrap().is_signed());
}

#[test]
fn app_settings_view_texts_name_the_product() {
    let running = Version::new(1, 2, 0);
    let newer = Version::new(1, 3, 0);
    for p in PRODUCTS {
        let name = p.name;
        let others = others(&p);
        let signed = Release {
            version: newer,
            exe: Some(ReleaseAsset {
                name: p.exe_name.to_string(),
                size: 100,
            }),
            signature: Some(ReleaseAsset {
                name: p.signature_name(),
                size: 10,
            }),
        };
        let unsigned = Release {
            signature: None,
            ..signed.clone()
        };
        let view = |outcome: &Outcome, newer: Option<&Release>, phase: &InstallPhase| {
            app_settings_view(&p, running, false, outcome, newer, phase)
        };
        let available = Outcome::Available { newer };
        let idle = InstallPhase::Idle;

        let v = view(&Outcome::Idle, None, &idle);
        assert_eq!(
            (v.update.state, v.update.text.as_str()),
            (UpdateState::Idle, "Not checked yet.")
        );
        assert_eq!(
            (v.install.state, v.install.text.as_str()),
            (InstallState::None, "")
        );
        assert_eq!(v.version, "1.2.0");
        let v = view(&Outcome::Checking, None, &idle);
        assert_eq!(v.update.text, "Checking for updates\u{2026}");
        let v = view(&Outcome::UpToDate { latest: running }, None, &idle);
        assert_eq!(v.update.text, "You have the latest version (1.2.0).");
        let v = view(&Outcome::NoRelease, None, &idle);
        assert_eq!(
            v.update.text,
            format!("No {name} release is published yet.")
        );
        let v = view(
            &Outcome::Failed {
                reason: "GitHub answered 500".into(),
            },
            None,
            &idle,
        );
        assert_eq!(
            v.update.text,
            "Could not check for updates: GitHub answered 500."
        );

        let v = view(&available, Some(&signed), &idle);
        assert_eq!(v.update.state, UpdateState::Available);
        assert_eq!(
            v.update.text,
            format!("{name} 1.3.0 is available. You have 1.2.0.")
        );
        assert_eq!(v.update.newer.as_deref(), Some("1.3.0"));
        assert_eq!(
            (v.install.state, v.install.text.as_str()),
            (InstallState::Ready, "Install 1.3.0 and restart")
        );

        let v = view(&available, Some(&unsigned), &idle);
        assert_eq!(v.install.state, InstallState::Unsigned);
        assert_eq!(
            v.install.text,
            "1.3.0 is available but cannot be installed: the release is not signed."
        );

        let phases = [
            (
                InstallPhase::Downloading {
                    bytes: 40,
                    total: Some(100),
                },
                InstallState::Downloading,
                format!("Downloading {name} 1.3.0\u{2026}"),
            ),
            (
                InstallPhase::Verifying,
                InstallState::Verifying,
                format!("Checking the signature of {name} 1.3.0\u{2026}"),
            ),
            (
                InstallPhase::Restarting,
                InstallState::Restarting,
                format!("{name} 1.3.0 is installed. Restarting\u{2026}"),
            ),
            (
                InstallPhase::Failed {
                    reason: "the download ended early".into(),
                },
                InstallState::Failed,
                format!("Could not install {name} 1.3.0: the download ended early."),
            ),
        ];
        for (phase, state, text) in phases {
            let v = view(&available, Some(&signed), &phase);
            assert_eq!((v.install.state, &v.install.text), (state, &text));
            assert_eq!(
                v.install.progress.is_some(),
                state == InstallState::Downloading
            );
            // Every text names this product, never another, and no part of
            // the view carries an address.
            assert!(v.install.text.contains(name), "{text}");
            let json = serde_json::to_string(&v).unwrap();
            for o in &others {
                assert!(!json.contains(o.name), "{json}");
            }
            assert!(
                !json.contains("http") && !json.contains("github.com"),
                "{json}"
            );
        }
        let v = view(
            &available,
            Some(&signed),
            &InstallPhase::Downloading {
                bytes: 40,
                total: Some(100),
            },
        );
        assert_eq!(
            serde_json::to_value(&v).unwrap(),
            json!({
                "version": "1.2.0",
                "check_updates": false,
                "update": {
                    "state": "available",
                    "text": format!("{name} 1.3.0 is available. You have 1.2.0."),
                    "newer": "1.3.0",
                },
                "install": {
                    "state": "downloading",
                    "text": format!("Downloading {name} 1.3.0\u{2026}"),
                    "progress": { "bytes": 40, "total": 100 },
                },
            })
        );
    }
}
