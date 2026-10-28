//! The signature check against the fixtures in `apps/shared/fixtures/signing`
//! (see its README): one good signature, the positive control of every
//! refusal, and one fixture per way a genuine-looking signature can be wrong.

mod common;

use common::{fixture, fixture_text, payload, test_key, TempDir};
use ep_update::signature::{read_signature, verify_bytes, verify_file};
use ep_update::{SignatureError, RELEASE_PUBLIC_KEY};

const FILE: &str = "payload.bin";
const VERSION: &str = "1.3.0";

#[test]
fn signature_fixture_verifies_with_the_test_key_baseline() {
    let good = fixture_text("payload.bin.minisig");
    let sig = read_signature(&good, FILE, VERSION).expect("the good signature reads");
    // In memory and as a file on disk, read in chunks: the same answer.
    assert_eq!(verify_bytes(&payload(), &sig, &test_key()), Ok(()));
    let dir = TempDir::new();
    let staged = dir.path().join("app.exe.download");
    std::fs::write(&staged, payload()).unwrap();
    assert_eq!(verify_file(&staged, &sig, &test_key()), Ok(()));
    // The key is taken with surrounding white space, as a pasted line.
    assert_eq!(
        verify_bytes(&payload(), &sig, &format!(" {}\n", test_key())),
        Ok(())
    );
}

#[test]
fn signature_refuses_a_tampered_file() {
    let good = fixture_text("payload.bin.minisig");
    let sig = read_signature(&good, FILE, VERSION).unwrap();
    let original = payload();
    // Positive control first.
    assert_eq!(verify_bytes(&original, &sig, &test_key()), Ok(()));
    let mut cases: Vec<(&str, Vec<u8>)> = Vec::new();
    for at in [0, original.len() / 2, original.len() - 1] {
        let mut b = original.clone();
        b[at] ^= 0x01;
        cases.push(("one bit flipped", b));
    }
    let mut longer = original.clone();
    longer.push(0);
    cases.push(("one byte appended", longer));
    cases.push(("one byte cut", original[..original.len() - 1].to_vec()));
    cases.push(("empty", Vec::new()));
    let dir = TempDir::new();
    for (what, bytes) in cases {
        assert_eq!(
            verify_bytes(&bytes, &sig, &test_key()),
            Err(SignatureError::Mismatch),
            "{what}"
        );
        let staged = dir.path().join("staged");
        std::fs::write(&staged, &bytes).unwrap();
        assert_eq!(
            verify_file(&staged, &sig, &test_key()),
            Err(SignatureError::Mismatch),
            "{what} (on disk)"
        );
    }
    // A trusted comment edited after signing keeps its file and version
    // fields, so it reads, but the global signature no longer covers it.
    let edited = good.replacen("timestamp:1", "timestamp:2", 1);
    assert_ne!(edited, good);
    let sig = read_signature(&edited, FILE, VERSION).unwrap();
    assert_eq!(
        verify_bytes(&original, &sig, &test_key()),
        Err(SignatureError::Mismatch)
    );
    // A file that is not there is a read error, not a pass.
    let gone = dir.path().join("missing");
    assert!(matches!(
        verify_file(&gone, &sig, &test_key()),
        Err(SignatureError::Io(_))
    ));
}

#[test]
fn signature_refuses_another_key() {
    // Made with a second throwaway key: same file, same version.
    let other = fixture_text("another-key.minisig");
    let sig = read_signature(&other, FILE, VERSION).expect("its comment names the right file");
    assert_eq!(
        verify_bytes(&payload(), &sig, &test_key()),
        Err(SignatureError::AnotherKey)
    );
    // The good signature against the owner's release key: another key too,
    // so the fixtures can never pass for a release.
    let good = read_signature(&fixture_text("payload.bin.minisig"), FILE, VERSION).unwrap();
    assert_eq!(verify_bytes(&payload(), &good, &test_key()), Ok(()));
    assert_eq!(
        verify_bytes(&payload(), &good, RELEASE_PUBLIC_KEY),
        Err(SignatureError::AnotherKey)
    );
    // A key that is not a key.
    assert_eq!(
        verify_bytes(&payload(), &good, "RWnot-a-key"),
        Err(SignatureError::BadKey)
    );
    assert_eq!(
        verify_bytes(&payload(), &good, ""),
        Err(SignatureError::BadKey)
    );
    // Text that is not a signature at all.
    for text in [
        "",
        "untrusted comment: x\n",
        "not a signature\nat all\nreally\nno",
    ] {
        assert!(read_signature(text, FILE, VERSION).is_err(), "{text:?}");
    }
}

#[test]
fn signature_refuses_another_file_name() {
    let text = fixture_text("another-file-name.minisig");
    assert!(matches!(
        read_signature(&text, FILE, VERSION),
        Err(SignatureError::AnotherFile)
    ));
    // Control: it is a genuine signature of these bytes, under its own name.
    let sig = read_signature(&text, "other-name.bin", VERSION).unwrap();
    assert_eq!(verify_bytes(&payload(), &sig, &test_key()), Ok(()));
    // The name must match exactly.
    for name in ["Payload.bin", "payload.bin ", "payload", "other-name.bin"] {
        let good = fixture_text("payload.bin.minisig");
        assert!(
            matches!(
                read_signature(&good, name, VERSION),
                Err(SignatureError::AnotherFile)
            ),
            "{name:?}"
        );
    }
}

#[test]
fn signature_refuses_another_version() {
    let text = fixture_text("another-version.minisig");
    assert!(matches!(
        read_signature(&text, FILE, VERSION),
        Err(SignatureError::AnotherVersion)
    ));
    // Control: genuine, for 1.2.9.
    let sig = read_signature(&text, FILE, "1.2.9").unwrap();
    assert_eq!(verify_bytes(&payload(), &sig, &test_key()), Ok(()));
    let good = fixture_text("payload.bin.minisig");
    for v in ["1.3", "1.3.0.0", "v1.3.0", "1.3.1"] {
        assert!(
            matches!(
                read_signature(&good, FILE, v),
                Err(SignatureError::AnotherVersion)
            ),
            "{v:?}"
        );
    }
    // The raw fixture bytes are the text form a release carries.
    assert!(fixture("payload.bin.minisig").starts_with(b"untrusted comment: "));
}
