# Shared update fixtures

Test inputs of `crates/ep-update`. Nothing here is read by a build that
reaches a player.

## `signing/`

A minisign key pair made for these tests only, a payload, and four
signatures of it. The private half of the test key was thrown away after
signing; it is not in this repository and cannot be recovered, so the
signatures below cannot be remade, only replaced by a new set.

| File | What it is |
|---|---|
| `test-key.pub` | The test public key (key ID `84B4E31FC10E2A2D`), as minisign writes a `.pub` file: an untrusted comment, then the `RW...` key line the tests inject through `UpdateConfig`. |
| `payload.bin` | 40,067 bytes: one line of text, a NUL, then 40,000 pseudo-random bytes (Python's `random.Random(20261028)`). Not an executable. SHA-256 `eb542dfd8a197dcab4e3a27b9941eeda15656653af32574c083fd732531cb691`. |
| `payload.bin.minisig` | The good signature: `payload.bin` signed with the test key, trusted comment `file:payload.bin` and `version:1.3.0`. The positive control of every refusal test. |
| `another-file-name.minisig` | The same bytes, copied to `other-name.bin` and signed with the test key: `file:other-name.bin`, `version:1.3.0`. |
| `another-version.minisig` | `payload.bin` signed with the test key with `version:1.2.9`. |
| `another-key.minisig` | `payload.bin` signed with a second throwaway key (also discarded), `file:payload.bin`, `version:1.3.0`. |

How they were made, 2026-10-28, with the `@tauri-apps/cli 2.12.2` the quest
tracker pins, every key and intermediate file in a scratch folder outside
the checkout:

```sh
npm --prefix apps/quest-tracker exec -- tauri signer generate --ci -p <pw> -w <scratch>/test.key
npm --prefix apps/quest-tracker exec -- tauri signer generate --ci -p <pw2> -w <scratch>/other.key
npm --prefix apps/quest-tracker exec -- tauri signer sign -f <scratch>/test.key -p <pw> --app-version 1.3.0 <scratch>/payload.bin
npm --prefix apps/quest-tracker exec -- tauri signer sign -f <scratch>/test.key -p <pw> --app-version 1.3.0 <scratch>/other/other-name.bin
npm --prefix apps/quest-tracker exec -- tauri signer sign -f <scratch>/test.key -p <pw> --app-version 1.2.9 <scratch>/ver/payload.bin
npm --prefix apps/quest-tracker exec -- tauri signer sign -f <other.key> -p <pw2> --app-version 1.3.0 <scratch>/key/payload.bin
```

`tauri signer` writes each `.sig` and `.pub` as base64 of minisign's text;
each file here is that text decoded (`base64 -d`), which is the form a
release's `<exe>.minisig` asset carries. The tests read the payload as the
release's exe asset named `payload.bin`, of a version `1.3.0`.

`.gitattributes` turns off line-ending conversion for every file here: a
converted byte breaks a signature.
