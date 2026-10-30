# In-app updates: the release contract, the key and the shared crates

The quest tracker, the waystone roller and the crafting helper update
themselves the same way, with one copy of the code. Each app's Settings window can check GitHub for a
newer release of that app (*Check now*, or *Automatically check for
updates*, off by default, which only checks). When a check finds one, the
window's install button downloads the new exe, checks its signature against
the public key built into the app, puts it in place of the running exe and
restarts into it. Nothing is downloaded until the button is pressed. There
is no fallback to a release page or a browser: when a step fails, the
window says why in one line, with *Try again*, and the running app is
untouched; a newer release that is not signed is shown as such, with no
button.

What each app shows and logs is in its own guide:
[`quest-tracker/README.md`](quest-tracker/README.md),
[`waystone-roller/README.md`](waystone-roller/README.md) and
[`crafting-helper/README.md`](crafting-helper/README.md) (Overview, Command
reference, Privacy, Known limitations). This page is what all three share: what
a release must carry, why the signature is the check, the owner's key, and
the code.

## The release contract

A release the apps can install is a published (not draft, not prerelease)
GitHub release of `S-Borkowski/ExilePact2` tagged
`quest-tracker-v<major>.<minor>.<patch>`,
`waystone-roller-v<major>.<minor>.<patch>` or
`crafting-helper-v<major>.<minor>.<patch>`, carrying two assets:

| Asset | What it is |
|---|---|
| `quest-tracker.exe` / `waystone-roller.exe` / `crafting-helper.exe` | The app itself, as the release workflow built it. |
| `quest-tracker.exe.minisig` / `waystone-roller.exe.minisig` / `crafting-helper.exe.minisig` | The exe's minisign signature, as plain text: four lines, algorithm `ED` (Ed25519 over the file's BLAKE2b-512 hash), with the trusted comment `timestamp:<t>`, `file:<exe name>` and `version:<version>`, tab-separated. |

Since 0.2.0 the quest tracker's build is `Quest Tracker.exe`; its release
workflow publishes and signs it as `quest-tracker.exe`, and an update keeps
the installed file's own name, whichever of the two it is.

The other assets (`SHA256SUMS.txt`, the quest tracker's
`quest-tracker-replay.exe`, which is not updated in the app) stay as they
were. A release with no exe or no `.minisig` is shown as "<version> is
available but cannot be installed: the release is not signed." and cannot be
installed.

The three release workflows (`.github/workflows/quest-tracker-release.yml`,
`.github/workflows/waystone-roller-release.yml`,
`.github/workflows/crafting-helper-release.yml`) make the signature: their
"Sign the exe" step runs `tauri signer sign --app-version "$VERSION"` on
the built exe, with the repository secrets `TAURI_SIGNING_PRIVATE_KEY` and
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD` in that one step's environment,
decodes the base64 `.sig` it writes into `<exe>.minisig`, checks that the
trusted comment names the exe and the version, and attaches it to the draft
release beside the exe. When `TAURI_SIGNING_PRIVATE_KEY` is missing or
empty, that step prints a `::error::` and fails the job before "Create or
update the draft release", so the workflows never make an unsigned release.
`py -3 -m tools.<app>.release notes <version>` names the `.minisig` in the
release body.

What the app does with a release, in order:

1. **The check** reads GitHub's release list (`GET
   https://api.github.com/repos/S-Borkowski/ExilePact2/releases?per_page=30`)
   and keeps the highest published release of its own tag prefix, with the
   sizes of its exe and `.minisig` assets. It is newer, or it is not.
2. **The install** (the button only) creates `<exe>.download` new in the
   exe's own folder (an unwritable folder fails here, before any request);
   downloads `https://github.com/S-Borkowski/ExilePact2/releases/download/<tag>/<exe>.minisig`
   (at most 4 KiB) and checks that its trusted comment names the release's
   exe and version; then downloads `.../<tag>/<exe>` into the staging file,
   at most 64 MiB and exactly the size the release lists. GitHub answers each
   download with a redirect to a signed URL; the app follows exactly one,
   only over HTTPS on port 443 to `release-assets.githubusercontent.com` or
   `objects.githubusercontent.com` (GitHub's earlier asset host).
3. **The signature**: the staged file's bytes, as they are on disk after it
   is closed, must verify against the built-in key, and the trusted
   comment's `file:` and `version:` must equal the asset's name and the
   release's version. Any mismatch deletes the staging file and fails the
   install.
4. **The swap**, by renames only: the running exe to `<exe>.old` (replacing
   a stale one), then the staging file to the exe's name. If the second
   rename fails, the first is undone. Windows lets a running image be
   renamed but not deleted; `ep-update`'s test
   `swap_replaces_a_running_executable` proves the swap on a running copy of
   a test binary.
5. **The restart**: the app quits through its own quit path, so its shutdown
   saves as on any quit; after the shutdown it frees its global shortcut and
   starts the exe at the path it captured when it started, with the same
   arguments. At every start a background thread removes `<exe>.old` and a
   stale `<exe>.download`, retrying for about 30 s while the old process may
   still be ending.

A failed or cancelled install deletes the staging file and leaves the
running exe where it was. Every step writes one `updates:` line to the app's
diagnostic log (each guide's Command reference lists them); no line holds a
URL, a query or anything of GitHub's answer.

## The threat model

**The release key is the trust anchor.** The app installs a file only when
its signature verifies against `RELEASE_PUBLIC_KEY` in
`apps/shared/crates/ep-update/src/release_key.rs`, a plain constant compiled
into all three apps (never an `Option`: there is no build without a key). Whoever
holds the private half can make an exe every installed copy will accept, so
it lives in one file on the owner's machine and in the repository secret,
and nowhere else.

- **A file the app downloads carries no "downloaded from the internet"
  mark.** A browser download gets one, and Windows checks a marked exe when
  it is run; the app writes the file itself and adds none, so nothing in
  Windows checks it. The signature stands in for that check.
- **The signature covers the bytes and the claim.** The trusted comment is
  signed with the file, and the app requires its `file:` and `version:` to
  match. A genuine signature of another file, or of an older release, is
  refused, so an old signed exe cannot be offered as a newer one.
- **No separate SHA-256 check.** GitHub's asset `digest` and
  `SHA256SUMS.txt` come from the same place that serves the file; the
  signature does not.
- **What a hostile answer can do.** The release list and the downloads go
  over TLS with Windows' own certificate checks, to fixed hosts; no URL
  from an answer is ever requested. A forged release list or a swapped
  asset can make an install fail; it cannot make an unsigned or
  wrongly-signed file be installed.
- **What the app replaces.** Its own exe, in its own folder, by renames;
  nothing else on the machine.
- **Not covered.** The exe carries no Authenticode signature, so how
  Windows SmartScreen or an antivirus treats the new exe, or the rename of
  a running one, is not established.

The flow is proven by tests: the signature check against fixtures signed
with a throwaway key, the install against a fake transport on a temporary
folder, the downloads against servers the tests run on `127.0.0.1`, and the
swap on this machine. An install from a real, signed release has not been
run yet: that needs the first signed release of each app.

## The release key

### One-time set-up (the owner; a prerequisite)

The release workflows fail without the secret, so the key comes before any
release. Done once, on 2026-10-28 (key ID `663D6EC6E2F9813A`):

1. Generate the key, with a password, to a path outside any checkout:

   ```sh
   npm --prefix apps/quest-tracker exec -- tauri signer generate -w C:/Users/<you>/.tauri/exilepact2-release.key
   ```

   It writes the private key file and `<that path>.pub`.
2. On GitHub, repository S-Borkowski/ExilePact2 › Settings › Secrets and
   variables › Actions, add `TAURI_SIGNING_PRIVATE_KEY` (the private key
   file's whole content) and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` (its
   password). One key signs all three apps.
3. Put the public key in `release_key.rs`: the `.pub` file is base64; its
   decoded text is an untrusted comment and the `RW...` key line, and the
   constant holds that key line. Never paste the private key anywhere.

`tests/test_app_updates_guard.py` checks that the constant is a minisign
public key line and not the test fixture's key.

### Rotating the key

Every installed copy checks against the key it was built with, so a new key
reaches users only through a release signed with the old one:

1. Generate the new key as above, to a new path.
2. Replace the key line in `release_key.rs` (and the key ID in its comment),
   and cut a release of each app **while the secrets still hold the old
   key**: copies already installed accept it, and it carries the new key.
3. Then replace both secrets with the new key and its password.

A copy that never installed that release refuses every later one ("the
update is signed with another key"), and its user downloads the new exe by
hand once. Losing the private key has the same effect for every user: the
next version is downloaded by hand, from a build that carries a new key.

## The shared crates

The update code is a Cargo workspace of its own, `apps/shared/`, that the
three apps take as path dependencies (none lists it among its own members):

| Crate | What it is |
|---|---|
| `crates/ep-net/` | The three apps' only network code, over HTTPS through Windows' own WinHTTP, with the bounds and headers each guide's Privacy section lists: the release list and a release's two fixed assets, and a third request, `fetch_trade_leagues`, the trade site's league list (`GET https://www.pathofexile.com/api/trade2/data/leagues`, two headers, at most 64 KiB), which the crafting helper alone sends, only when the player clicks the refresh icon beside its League setting. Each request's path, headers and body cap are its own. No caller can pass a host, a path or a URL; elsewhere than Windows every request is refused as unsupported. |
| `crates/ep-update/` | The update logic, with no network and no `unsafe`: the release-list parse, the check's state machine, the install (single-flight with the check, on a thread of its own), the signature check (`minisign-verify =0.2.5`, the one registry crate this added), the swap and the start-up cleanup, the Settings window's view and its texts, and the `updates:` lines. The product (name, tag prefix, exe name) is data: `QUEST_TRACKER`, `WAYSTONE_ROLLER`, `CRAFTING_HELPER` (with `ep-net`'s matching `CRAFTING_HELPER_USER_AGENT`, `CRAFTING_HELPER_TAG_PREFIX` and `CRAFTING_HELPER_EXE`). |
| `fixtures/` | The signature fixtures, made once with `tauri signer` and a throwaway key that was discarded (`fixtures/README.md` says how). |

Each app keeps its own glue (the transport over `ep-net`, the restart) and
its own Settings window UI; none depends on another.

Tests (none reaches the internet):

```sh
cargo test --manifest-path apps/shared/Cargo.toml --locked --workspace   # ep-net against 127.0.0.1, ep-update with a fake transport
py -3 -m unittest tests.test_app_updates_guard                           # the shared crates, from the files
```

`tests.test_app_updates_guard` pins that only `ep-net` names network code
and nothing in the workspace is `unsafe` outside it, that the crates name
the repository, tag prefixes and asset names the release tooling uses, that
a download's redirect may reach only the two asset hosts, that the key is
real and only the built-in one is used, and that every dependency is the
quest tracker's pin except `minisign-verify`. Each app's own guard
(`tests.test_quest_tracker_network`, `tests.test_waystone_roller_guard`,
`tests.test_crafting_helper_guard`) reads the shared workspace beside the
app.

A change here reaches the three apps at once: run every app's test suite too
(each guide's "Working on the app").

**Maintenance triggers.** GitHub changing its release flow or asset host
(the `/releases/download/` redirect, the asset hosts, the release list's
fields), the repository or a tag prefix changing, or the signing key
changing: update `ep-net`'s constants, `ep-update`'s products and this
page together, with `tests/test_app_updates_guard.py` and every app's
`release.py`.
