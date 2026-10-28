//! The signature check: a downloaded exe is installed only when its
//! `<exe>.minisig` verifies against the key built into the app.
//!
//! The release workflows sign each exe with `tauri signer sign
//! --app-version <v>`, which makes a minisign signature of algorithm `ED`
//! (Ed25519 over the file's BLAKE2b-512 hash) whose trusted comment reads
//! `timestamp:<t>\tfile:<file name>\tversion:<v>`. The trusted comment is
//! covered by the signature's second (global) half, so once the signature
//! verifies, its `file:` and `version:` are the signer's word. Both must
//! equal what the install expects: the release's exe asset name and the
//! release's version. That keeps a genuine signature of another file, or of
//! an older release, from being paired with the wrong download.
//!
//! The legacy algorithm `Ed` (a signature over the raw bytes) is refused:
//! the release workflows never make it.

use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use minisign_verify::{Error, PublicKey, Signature};

/// Why a signature did not check out. The texts are the reason line of a
/// failed install.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureError {
    /// The key the app checks against could not be read.
    BadKey,
    /// The `.minisig` is not a minisign signature.
    Unreadable,
    /// It is the legacy algorithm, or another one this check does not take.
    Unsupported,
    /// It was made with another key than the app's.
    AnotherKey,
    /// The trusted comment names another file than the release's exe.
    AnotherFile,
    /// The trusted comment names another version than the release's.
    AnotherVersion,
    /// The bytes, or the trusted comment, do not match the signature.
    Mismatch,
    /// The downloaded file could not be read back.
    Io(io::ErrorKind),
}

impl SignatureError {
    /// One line, for the window's status and the diagnostic log.
    pub fn reason(&self) -> String {
        match self {
            SignatureError::BadKey => "the app's update key could not be read".to_string(),
            SignatureError::Unreadable => "the release's signature could not be read".to_string(),
            SignatureError::Unsupported => {
                "the release's signature has an unsupported format".to_string()
            }
            SignatureError::AnotherKey => "the update is signed with another key".to_string(),
            SignatureError::AnotherFile => "the signature is for another file".to_string(),
            SignatureError::AnotherVersion => "the signature is for another version".to_string(),
            SignatureError::Mismatch => {
                "the downloaded file does not match its signature".to_string()
            }
            SignatureError::Io(kind) => {
                format!("the downloaded file could not be read back ({kind})")
            }
        }
    }
}

/// A `.minisig` read and its trusted comment checked, not yet verified
/// against any bytes.
pub struct CheckedSignature {
    signature: Signature,
}

/// Reads `text` as a minisign signature and checks that its trusted
/// comment names `file_name` and `version` (each field exactly once).
/// Done before the exe is downloaded, so a signature for something else
/// fails without the large request; the comment is only trusted once
/// [`verify_file`] verified the whole.
pub fn read_signature(
    text: &str,
    file_name: &str,
    version: &str,
) -> Result<CheckedSignature, SignatureError> {
    let signature = Signature::decode(text).map_err(|e| match e {
        Error::UnsupportedAlgorithm => SignatureError::Unsupported,
        _ => SignatureError::Unreadable,
    })?;
    let field = |key: &str| -> Option<&str> {
        let mut found = signature
            .trusted_comment()
            .split('\t')
            .filter_map(|f| f.strip_prefix(key));
        let first = found.next()?;
        found.next().is_none().then_some(first)
    };
    if field("file:") != Some(file_name) {
        return Err(SignatureError::AnotherFile);
    }
    if field("version:") != Some(version) {
        return Err(SignatureError::AnotherVersion);
    }
    Ok(CheckedSignature { signature })
}

/// Verifies the file at `path`, as it is on disk, against `signature` and
/// the minisign public key `public_key` (its `RW...` line), reading it in
/// chunks.
pub fn verify_file(
    path: &Path,
    signature: &CheckedSignature,
    public_key: &str,
) -> Result<(), SignatureError> {
    let key = PublicKey::from_base64(public_key.trim()).map_err(|_| SignatureError::BadKey)?;
    let mut verifier = key
        .verify_stream(&signature.signature)
        .map_err(map_verify_error)?;
    let mut file = File::open(path).map_err(|e| SignatureError::Io(e.kind()))?;
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = match file.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(SignatureError::Io(e.kind())),
        };
        verifier.update(&buf[..n]);
    }
    verifier.finalize().map_err(map_verify_error)
}

/// Verifies `bytes` in memory, the same way as [`verify_file`].
pub fn verify_bytes(
    bytes: &[u8],
    signature: &CheckedSignature,
    public_key: &str,
) -> Result<(), SignatureError> {
    let key = PublicKey::from_base64(public_key.trim()).map_err(|_| SignatureError::BadKey)?;
    key.verify(bytes, &signature.signature, false)
        .map_err(map_verify_error)
}

fn map_verify_error(e: Error) -> SignatureError {
    match e {
        Error::UnexpectedKeyId => SignatureError::AnotherKey,
        Error::InvalidSignature => SignatureError::Mismatch,
        Error::UnexpectedAlgorithm | Error::UnsupportedAlgorithm | Error::UnsupportedLegacyMode => {
            SignatureError::Unsupported
        }
        Error::IoError(e) => SignatureError::Io(e.kind()),
        Error::InvalidEncoding => SignatureError::Unreadable,
    }
}
