//! The owner's release public key.
//!
//! Every `quest-tracker.exe`, `waystone-roller.exe`, `Crafting.Helper.exe`
//! and legacy `crafting-helper.exe` a release workflow publishes carries a
//! `<exe>.minisig` signature made with
//! this key's private half, and the app installs a downloaded update only when that signature
//! checks out against the key below. The private half is the owner's alone:
//! it lives in the repository secret `TAURI_SIGNING_PRIVATE_KEY` (with its
//! password in `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`) and is never committed.
//!
//! The value is the key line of the minisign public key file that
//! `tauri signer generate` wrote (key ID `663D6EC6E2F9813A`), decoded from
//! the base64 form that tool prints. It is a plain constant, not an
//! `Option`: there is no build without a key. Rotating the key is the
//! procedure in `docs/app-updates.md`.

/// The minisign public key every update's signature is checked against.
pub const RELEASE_PUBLIC_KEY: &str = "RWQ6gfnixm49ZvDZV2WB2BFWQ9PSqLZNWcve0BO/YOx9M8zmOzWJ+6XJ";
