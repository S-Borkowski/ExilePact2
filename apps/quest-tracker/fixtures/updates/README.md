# The update check's and the install's test inputs

## `releases.json`

A release list, written by hand, never fetched: it has the shape of
GitHub's REST "List releases" answer (`tag_name`, `draft`, `prerelease`,
`html_url`), with entries chosen to take every path of the quest tracker's
reading of it (`ep_update::newest_release` for the `qt_app::updates::PRODUCT`):

- `quest-tracker-v1.10.0` is the newest published Quest Tracker release, and
  its `html_url` points elsewhere: nothing reads it, and no page is opened
  for an update;
- `quest-tracker-v1.9.3` is older, and lower than 1.10.0 only when the parts
  are compared as numbers, not text;
- a draft (1.12.0) and a prerelease (1.11.0) are newer and skipped;
- another tool's tag, and tags that are not exactly `quest-tracker-v`
  followed by `<major>.<minor>.<patch>` (two or four parts, a suffix, a sign,
  a space, an empty part, letters, another case, no prefix, a number too
  large), are skipped;
- entries with no tag, a tag that is not a string, a `draft` that is missing
  or not a bool, and entries that are not objects are skipped, never an
  error.

The tests are `releases_*` in `crates/qt-app/tests/updates.rs`. The parse of
a release's assets (the exe and its `.minisig`) is `ep-update`'s, tested
there (`apps/shared/crates/ep-update/tests/releases.rs`).

## `signing/`

What the install tests of `crates/qt-app/tests/updates.rs` download through
their fake transport: a payload, its signature and the public key that
checks it, made for these tests only. Nothing here is read by a build that
reaches a player.

| File | What it is |
|---|---|
| `test-key.pub` | The test public key (key ID `F539AB14B4D95E1B`), as minisign writes a `.pub` file: an untrusted comment, then the `RW...` key line the tests inject through `UpdateConfig::public_key`. |
| `payload.bin` | 4,047 bytes: one line of text, a NUL, then 4,000 pseudo-random bytes (Python's `random.Random(20261029)`). Not an executable. SHA-256 `95735939d51bace5061a59694661516988c848c5f49308fa8857fb9bfe9350d4`. |
| `payload.bin.minisig` | Its signature with the test key, trusted comment `file:quest-tracker.exe` and `version:1.3.0`: the real product's exe name, so the install's name check passes as it would for a release. |

These are not `apps/shared/fixtures/signing/`: that set is signed for the
file name `payload.bin`, which only `ep-update`'s own test product carries.

How they were made, 2026-10-28, with the `@tauri-apps/cli 2.12.2` the quest
tracker pins, the key and every intermediate file in a scratch folder
outside the checkout; the payload was signed under the name
`quest-tracker.exe`, then copied here as `payload.bin`:

```sh
npm --prefix apps/quest-tracker exec -- tauri signer generate --ci -p <pw> -w <scratch>/test.key
npm --prefix apps/quest-tracker exec -- tauri signer sign -f <scratch>/test.key -p <pw> --app-version 1.3.0 <scratch>/quest-tracker.exe
```

`tauri signer` writes the `.sig` and `.pub` as base64 of minisign's text;
each file here is that text decoded (`base64 -d`), the form a release's
`<exe>.minisig` asset carries. The private key was deleted after signing:
it is not in this repository and cannot be recovered, so the signature can
only be replaced by a new set, never remade.

`.gitattributes` turns off line-ending conversion for `signing/`: a
converted byte breaks a signature.
