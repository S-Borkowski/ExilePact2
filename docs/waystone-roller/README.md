# Waystone Roller

A small window for Path of Exile 2 that helps you "roll" (craft) waystones.
You pick the waystone modifiers you want and the ones you want to avoid, a
tier range, the rarities and corrupted or not; the app builds one search
string for the game's stash or vendor search box from those rules. It never
touches the game: it only puts the string on the clipboard, and you paste it.

- UI direction, window states and accessibility:
  [`ui-design.md`](ui-design.md); its reviews: [`design/`](design/)
- Research note (the search syntax, the poe2db page, how tokens are
  chosen): [`waystone-roller-research.md`](waystone-roller-research.md)
- Facts about the search box every mod can use:
  [`docs/RUNTIME_DATA_MODELS.md`](../RUNTIME_DATA_MODELS.md) § "Item search
  box (stash and vendor)"

## Overview

- **One window, `waystone-roller.exe`.** A Tauri app: the search-string
  engine (`wr-regex`) and the app's logic (`wr-app`) are Rust crates; the UI
  is Svelte and TypeScript in an ordinary window titled "Waystone Roller",
  resizable and in the taskbar. Closing the window quits the app. Only one
  copy runs; starting a second brings the first to the front.
- **The mod list** is every waystone modifier and desecrated waystone
  modifier on poe2db.tw's Waystones page, one row per modifier (tiers of the
  same modifier are one row). For each you choose Want, Avoid or neither.
- **The rules**: want any or all of the wanted mods; avoid every avoided mod;
  a tier range from 1 to 16; normal, magic and rare; corrupted, uncorrupted
  or either.
- **The search string** is shown with its length. The game's search box
  takes at most 250 characters. When the rules need more, the app says so,
  shows each part's length, and copies nothing: it never shortens the
  string or drops a rule.
- **`Ctrl+Alt+W` copies it** while another window, the game included, is
  in front (measured with the game in front, Known limitations); if
  another program already holds the shortcut, diag.log says `could not be
  registered`. It is a global shortcut, so it does not depend on the window
  being in front; whether the window was minimised in that session was not
  recorded. The Copy button does the same. On success nothing is heard; a
  refusal (nothing selected, over 250 characters, a mod that cannot be told
  apart) beeps and leaves the clipboard as it was. You paste the string
  into the stash or a vendor's search box with `Ctrl+V`.
- **The shortcut can be changed** in the window. Windows lets one program
  hold a key combination; when another program holds `Ctrl+Alt+W`, the app
  says so and keeps running, and the old shortcut stays when a new one is
  refused.
- **A Settings window**, opened from the *Settings* button at the right of
  the main window's header. It holds one setting, *Automatically check for
  updates*, off by default and saved, a *Check now* button with what the
  last check said, the running version and the line "Created by ST4H".
  While a check has found a newer version, the *Settings* button says so
  in words (`Update <version>`, beside a green dot).
- **An update check, only when you ask.** *Check now* asks GitHub once
  whether a newer Waystone Roller release is published; with *Automatically
  check for updates* on, the app also asks at launch and then once a day.
  The setting only checks: nothing is downloaded until you press the
  install button.
- **The update installs in the app.** When a check finds a newer release,
  the Settings window's *Install <version> and restart* button downloads
  its `waystone-roller.exe`, checks its signature against the key built
  into the app, puts it in place of the running exe and restarts into it.
  The window shows the download's progress, with *Cancel*. When a step
  fails, the window says why in one line, with *Try again*, and the running
  app is untouched; a newer release that is not signed says it cannot be
  installed and offers no button. Nothing opens a release page or a
  browser. How it works and why the signature is the check:
  [`docs/app-updates.md`](../app-updates.md).

## Platforms and prerequisites

- **Windows 10 or 11, 64-bit.**
- **The Microsoft Edge WebView2 runtime.** Windows 11 ships it, and most
  Windows 10 machines have it through Edge. `waystone-roller.exe` does not
  bundle it; without it the window cannot start.
- **The game in English.** The search terms are English text from the item
  (Known limitations).

To build it from this checkout you also need the Rust toolchain for the MSVC
target (`cargo`; built here with cargo 1.98.1), the Visual Studio C++ build
tools, and Node.js with npm. Every crate and npm package is pinned exactly,
to the quest tracker's versions; build with `--locked` and `npm ci`.

## Command reference

### `waystone-roller.exe`

Takes no arguments.

| Environment variable | Effect |
|---|---|
| `WAYSTONE_ROLLER_DIR` | The folder for `settings.json` and `diag.log`. Unset or empty: `%LOCALAPPDATA%\ExilePact2\waystone-roller\`. A folder inside a git checkout is refused: the app then runs without saving, and the status line says why. |

- `settings.json`: `schema` (1), `hotkey` (default `Ctrl+Alt+W`),
  `selection` (the rules above) and `check_updates` (*Automatically check
  for updates*; a file written before it existed, or any value but `true`,
  loads it off). Written a moment after each change, through
  a temporary file renamed over the old one, and when the app quits. Saved
  mods the data no longer has are dropped at launch, and the status line
  names them.
- `diag.log`: one line per event, with the UTC time. Every line the app
  writes:
  - `settings: <n>` at launch, for each note about the store or the saved
    settings that the status line also shows: the folder refused, the
    settings or the saved selection unreadable, saved mods dropped.
  - `start: ...` (the version, the mod data and the store folder).
  - `shortcut <hk> registered` or `shortcut <hk> could not be registered:
    <error>`, at launch and when the shortcut is changed.
  - `selection refused: <reason>` when a change to the selection is
    refused (for example a mod in both Want and Avoid).
  - `copy from the shortcut` or `copy from the Copy button`, then
    `copied N characters: <regex>` or `copy refused: <reason>`.
  - After a refusal, `refusal beep: <sound> ok` or `refusal beep: <sound>
    failed (<error>)` for each sound tried, or `refusal beep: none on this
    platform` where the app has no beep (outside Windows).
  - `error: <e>`: the embedded mod data could not be read (a broken build;
    the app then stops), or `error: settings could not be saved: <e>`.
  - `settings refused: <reason>` when the Settings window sends a change
    other than `check_updates` set to true or false; `error: settings
    window: <reason>` when that window could not be opened.
  - `updates: ...` lines, the same as the quest tracker's:
    `updates: automatic checks turned on` and `updates: automatic checks
    turned off` when the setting changes; `updates: check started
    (manual)` or `updates: check started (automatic)`, then one of
    `updates: up to date (running <cur>, latest <new>)`, `updates: <new>
    available (running <cur>)`, `updates: no release published` and
    `updates: check failed: <reason>`. For an install: `updates: install of
    <new> started`, `updates: signature of <new> checked`, `updates:
    replaced the running exe with <new>, restarting`, then `stop` and
    `updates: started the new version`; or `updates: install failed:
    <reason>`, `updates: install cancelled`, `updates: install refused:
    <reason>` (a check or an install running, no newer version known, the
    release not signed, the app closing or restarting) and `updates: the
    partial download could not be removed (<error>)`. At every start, one
    `updates: cleanup: nothing to remove`, or `updates: cleanup: removed
    waystone-roller.exe.old` (and `could not remove <file> (<error>)` for a
    file still held after about 30 s). On those failures: `updates: restart
    failed: the new version did not start (<error>)`, `updates: restart
    failed: the exe's path is not known`, `updates: the exe's path is not
    known (<error>); updates cannot be installed`, `updates: cleanup did
    not start (<error>)`, `error: the update check thread did not start:
    <error>` and `error: the update ticker did not start: <error>`. No line
    holds a URL or anything of GitHub's answer.
  - `stop` when the app quits.

  Rotated at 1 MB into `diag.log.1`.

**The Settings window**, titled "Waystone Roller Settings", opens from the
main window's *Settings* button (shown and focused when it is already
open). It is decorated, dark and not resizable, 420 by 364 at 1x, centred;
closing the main window quits the app with it. It holds one section,
*Updates*, and "Created by ST4H" at the bottom. *Updates* has:

- **Automatically check for updates** (off), with the helper text "Asks
  GitHub once a day whether a newer version is out. Downloads nothing: an
  update is installed only when you press its button. Off: Waystone Roller
  never connects unless you ask it to." With it on, a check runs at
  launch, again when it is turned on, and then, from a tick every minute,
  a day after the last check started and an hour after an automatic check
  failed. When checks happen is not stored.
- What the last check said: `Not checked yet.`, `Checking for updates…`,
  `You have the latest version (<running>).`, `Waystone Roller <new> is
  available. You have <running>.`, `No Waystone Roller release is
  published yet.` or `Could not check for updates: <reason>.`, with the
  quest tracker's reasons (its guide's Command reference).
- The install's line and its one button: *Install <new> and restart* while
  a newer, signed version is known; `Downloading Waystone Roller <new>…`
  with a progress bar, the amount received and *Cancel*; `Checking the
  signature of Waystone Roller <new>…`; `Waystone Roller <new> is
  installed. Restarting…`; `Could not install Waystone Roller <new>:
  <reason>.` with *Try again*; or `<new> is available but cannot be
  installed: the release is not signed.` with no button. The reasons are
  the quest tracker's (its guide's Command reference).
- **Check now**, disabled while a check or an install runs; under it, the
  running version.

The window's commands are `get_app_settings`, `update_settings` (exactly
`{"check_updates": <bool>}`), `check_for_updates` (starts a check and
answers at once), `install_update` (starts the install and answers at once;
the only way one starts) and `cancel_update`; the main window's button
calls `open_settings`. The window is sent `wr:app-settings` with the view
(the running version, `check_updates`, the check's state and text and the
newer version, and the install's state, text and progress), and the main
window's `wr:view` carries `update_available`. The newest release is the
highest published, non-prerelease release tagged
`waystone-roller-v<major>.<minor>.<patch>`.

### `py -3 -m tools.waystone_roller.poe2db` (developer-only)

Fetches poe2db.tw's Waystones page and writes the mod data file. Standard
library only.

```sh
py -3 -m tools.waystone_roller.poe2db fetch [--cache DIR] [--delay S] [--max-age DAYS]
py -3 -m tools.waystone_roller.poe2db extract [--cache DIR] [--out FILE]
```

- `fetch` reads robots.txt, then asks only for `/us/Waystones`, one request
  at a time, at least `--delay` seconds apart (default 3, never under 2),
  with a User-Agent naming this tool. A page cached within `--max-age` days
  (default 7) is left alone and nothing is asked. The cache, by default
  `%LOCALAPPDATA%\ExilePact2\waystone-roller\poe2db-cache\`, is refused
  inside a git checkout: raw pages are never tracked.
- `extract` writes `apps/waystone-roller/data/waystone-mods.json` (or
  `--out`) from the cached page and prints the family and skipped counts. A
  missing tab, unexpected column headers, an unknown Pre/Suf value, an empty
  table or a row count that differs from the tab's header stops it with a
  non-zero exit naming the tab, and the existing file is left alone.

### npm scripts

Run from `apps/waystone-roller/` (or with `npm --prefix apps/waystone-roller`).

| Script | What it does |
|---|---|
| `npm run dev` | The page alone in a browser at `http://127.0.0.1:1421/`, over the view fixtures (`?view=too-long` and so on; [`ui-design.md`](ui-design.md) § "Browser preview"). |
| `npm run build` | Type-checks the page and writes `dist/`, which the executable embeds. Needed before any `cargo` build of the `waystone-roller` package. |
| `npm test` | The page's tests, in jsdom. |
| `npm run start` | The app under `tauri dev`. |
| `npm run release` | `tauri build --no-bundle`: `target/release/waystone-roller.exe`. |

## How the search is built

The game's search box highlights the items whose text matches every term.
The app writes one string of terms separated by spaces, in this order:

1. **Rarity**: none or all three ticked adds nothing; one adds `"y: m"`
   (the end of `Rarity: Magic`); two add `"y: [mr]"`.
2. **Tier**: 1 to 16 adds nothing. Otherwise a term that matches the
   `(Tier N)` of the item's name line, such as `"r [5-9]\)|r 1[0-2]\)"` for
   tiers 5 to 12 or `"r 1[4-6]\)"` for 14 to 16. Every alternative keeps
   the `r ` of "Tier": the box also searches the mods' roll ranges, and a
   bare `15\)` lights a tier-2 waystone through `+14(11-15)%`.
3. **Want**: each mod is found by its own **token**, the shortest piece of
   its text (at least 3 letters) that no other mod's text and no other line
   of a waystone holds. Want any: one term `"t1|t2"`. Want all: one term per
   mod.
4. **Avoid**: one negated term, `"!t1|t2"`: an item with any of those mods
   is not highlighted.
5. **Corruption**: `corr` or `!corr`.

A term is quoted when it holds a space, `|`, `[`, `(` or `\`. For want any
and avoid, a shorter piece of text shared by several of the chosen mods,
and by no other, replaces their tokens when it makes the term shorter; it
is never longer.

The result is one of:

- **Ok**: at most 250 characters, copied by the shortcut or the button.
- **Nothing selected**: no rule, so nothing to search for.
- **Too long**: over 250 characters; each part's length is shown so you can
  see which rule to trim. Exactly 250 is accepted.
- **Cannot be told apart**: a wanted mod has no token of its own, because
  every piece of its text is also part of another mod (on the current data,
  only "Area contains # additional Abysses", inside "Area is overrun by the
  Abyssal"). Want any or avoid may still reach it when the mod that contains
  it is chosen too.

Every search-box fact this relies on was checked in the game on
2026-10-28 ([`waystone-roller-research.md`](waystone-roller-research.md)
§ "Search syntax", Live 1): the quoting, `|`, `!`, case, the rarity, tier
and corruption terms, and the limit (250 characters is accepted, 251 is
refused whole).

## Mod data

`apps/waystone-roller/data/waystone-mods.json`, built into the executable.
Source: <https://poe2db.tw/us/Waystones>, the tabs `WaystonesMods` and
`DesecratedWaystoneMods`, fetched 2026-10-28: 52 modifiers (35 waystone, 17
desecrated) and 3 skipped rows (one with no description, two desecrated
rows with only hidden stats). poe2db's wiki content is CC BY-NC-SA 3.0; the
game text is Grinding Gear Games', quoted as short user-facing text under
the owner's 2026-10-24 decision (AGENTS.md § "Legal").

The file keeps only what the app needs: each modifier's displayed lines with
every number as `#`, the desecrated affix name, prefix or suffix, how many
poe2db rows were merged into it, and which lines are the shared reward
lines (`#% more Waystones found in Area` and the like). No hidden stats, no
HTML, no value ranges. To refresh it, see "Working on the app".

## Privacy

- **The app touches nothing of the game.** It reads no game memory, hooks
  nothing, sends no input or window message to the game, and reads none of
  the game's files. Its only effects outside its windows are the text it
  puts on the clipboard, when you press the shortcut or the Copy button, a
  system beep when it refuses, the update check's requests to GitHub and,
  when you press the install button, replacing its own exe.
- **Network: only the update check and the install, only when you ask.**
  With *Automatically check for updates* off (the default) and no *Check
  now* pressed, the app sends and receives nothing. A check is one `GET
  https://api.github.com/repos/S-Borkowski/ExilePact2/releases?per_page=30`,
  port 443 only, from Rust through Windows' own WinHTTP, with three
  headers: `Accept: application/vnd.github+json`, `X-GitHub-Api-Version:
  2022-11-28` and `User-Agent: ExilePact2-waystone-roller` (no version, no
  machine detail). The answer is read up to 2 MiB within 20 s, used only to
  find the newest release, and never stored or logged.
- **An install** (the button only) makes two downloads, the signature, then
  the exe: `GET
  https://github.com/S-Borkowski/ExilePact2/releases/download/waystone-roller-v<version>/waystone-roller.exe.minisig`
  and the same path ending `waystone-roller.exe`, each with two headers,
  `Accept: application/octet-stream` and the same `User-Agent`. GitHub
  answers each with a redirect to a signed address on
  `release-assets.githubusercontent.com`; the app follows that one redirect,
  only over HTTPS on port 443 and only to that host or
  `objects.githubusercontent.com`, with the same two headers, and never logs
  or keeps the address. Each download stops at the size the release lists
  (at most 64 MiB for the exe, 4 KiB for the signature), after 30 s without
  a byte, or after 15 minutes in all.
- For every request: no cookie and no credential is sent, the proxy is the
  system's, and TLS and its certificate checks are Windows'. GitHub sees
  your IP address and those headers; nothing else leaves. The app's code
  outside the shared `ep-net` crate has no network code
  (`tests.test_waystone_roller_guard`). (WebView2, Microsoft's runtime
  that draws the windows, may connect on its own, as for the quest
  tracker.) The data tool asks poe2db.tw for one page, only when a
  developer runs it.
- **What it writes**: `settings.json` and `diag.log` in its folder
  (Command reference). The log holds the shortcut, the copied search strings,
  refusals and the `updates:` lines; nothing about your account or
  character, and nothing of GitHub's answers. An install also writes beside
  the exe: `waystone-roller.exe.download` while it downloads (deleted when
  the install fails or is cancelled) and `waystone-roller.exe.old`, the
  replaced exe, which the next start removes.

## Working on the app

| Path | What it holds |
|---|---|
| `apps/waystone-roller/crates/wr-regex/` | The engine: the data loader, token choice, the search builder, and a model of the search box used only by the tests. |
| `apps/waystone-roller/crates/wr-app/` | The app's logic without Tauri: the store, the selection, the view, the commands, the shortcut and copy path, and the update commands and ticker (`updates.rs`, over the shared `ep-update`; tested in `tests/updates.rs` with a recording fake transport). |
| `apps/waystone-roller/src-tauri/` | The Tauri glue: the global shortcut, single instance, the Win32 clipboard writer and beep, the commands, the Settings window (label `settings`, its capability `capabilities/settings.json`, `core:event:default` alone), the update transport over the shared `ep-net`, and the restart into an installed update. |
| `apps/waystone-roller/src/` | The Svelte page and `ipc.ts`, the view's types. The Settings window's page is `settings.html` with its own entry under `src/settings/` (`main.ts`, `SettingsWindow.svelte`, `UpdatesSection.svelte`, `sections.ts`, its view type `view.ts`, bridge and browser preview `settings.html?state=<state>`). |
| `apps/waystone-roller/data/` | The mod data file. |
| `apps/waystone-roller/fixtures/` | The invented mod set and item texts for `wr-regex`, the view fixtures shared by the page's tests and `wr-app`'s, and `app-settings/`, one Settings window view per update and install state. |
| `apps/shared/` | The update code both apps share, its own Cargo workspace: `ep-net` (the only network code) and `ep-update` (the check, the install, the signature check, the swap): [`docs/app-updates.md`](../app-updates.md). |
| `tools/waystone_roller/` | The poe2db data tool (`poe2db.py`) and the release cutter (`release.py`). |
| `.github/workflows/waystone-roller-release.yml` | The release build: from a pushed tag to a draft release. |

Tests (none opens a window or reaches the network):

```sh
npm --prefix apps/waystone-roller run build                                          # needed before the workspace build
cargo test --manifest-path apps/waystone-roller/Cargo.toml --workspace --locked       # every crate
cargo test --manifest-path apps/waystone-roller/Cargo.toml -p wr-regex --locked       # the engine alone, no build of the page needed
npm --prefix apps/waystone-roller test                                                # the page, in jsdom
py -3 -m unittest tests.test_waystone_roller_poe2db tests.test_waystone_roller_guard  # the data tool and the guard
py -3 -m unittest tests.test_waystone_roller_release                                  # the release cutter and workflow
cargo test --manifest-path apps/waystone-roller/Cargo.toml -p wr-app --locked --test updates  # the update commands, fake transport
cargo test --manifest-path apps/shared/Cargo.toml --locked --workspace                 # the shared update crates, 127.0.0.1 only
py -3 -m unittest tests.test_app_updates_guard                                        # the shared crates' guard
```

- `wr-regex`'s `_baseline` tests pin the model of the game's search on
  hand-written strings; its `_target` tests run generated strings through
  the model over item texts and check that exactly the intended items light.
- `wr-app`'s tests drive the app through a recording fake shell and a
  temporary store; `shipped_data_every_family_token_matches_only_its_own_item`
  checks the shipped data file the same way.
- `wr-app`'s `tests/updates.rs` drives the check and the install through a
  fake transport and a temporary folder, and writes the Settings window's
  fixtures in `fixtures/app-settings/` (regenerate after a deliberate change
  with `WR_UPDATE_FIXTURES=1` before the command above, and review the
  diff); `src/settings/settings-window.test.ts` renders each one.
- `tests.test_waystone_roller_guard` reads the files only: no input, no
  game access and no network but the shared `ep-net`, reached only from
  `src-tauri`, in any source or manifest; no Tauri plugin but the global
  shortcut and single instance (no opener: nothing opens a browser); the
  pinned CSP and both windows' capabilities; one version everywhere; and
  every package already in the quest tracker's locks but
  `minisign-verify`. `tests.test_app_updates_guard` pins the shared crates
  themselves ([`docs/app-updates.md`](../app-updates.md)).

**Refreshing the mod data**: run `fetch`, then `extract`, then the tests
above. A new or renamed modifier changes the file; check its diff, and the
shipped-data test says if a token no longer tells a mod apart.

**Changing the icon**: `src-tauri/icons/source.png` (the owner's own
image) is the one source. Regenerate the icon files with
`npm --prefix apps/waystone-roller exec tauri icon apps/waystone-roller/src-tauri/icons/source.png -- --output apps/waystone-roller/src-tauri/icons`,
delete what it writes beyond the files `src-tauri/tauri.conf.json`'s
`bundle.icon` names (plus `64x64.png` and `icon.png`), then run
`cargo clean --manifest-path apps/waystone-roller/Cargo.toml --release -p waystone-roller`
before `npm --prefix apps/waystone-roller run release`: an incremental
build kept the old `icon.ico` in the exe (seen 2026-10-28).

**Cutting a release.** A release is the annotated tag
`waystone-roller-v<version>` on the tip of `main`. The version is the one in
`apps/waystone-roller/`'s `package.json`, `package-lock.json`,
`src-tauri/tauri.conf.json`, the workspace's `Cargo.toml` and `Cargo.lock`
(the members `wr-regex`, `wr-app` and `waystone-roller`). These must agree,
and "Release notes" below needs a `### <version>` section for it:

1. Bump the version in those files (build once so `Cargo.lock` follows),
   and write the version's release notes. Land both on `main` through a
   pull request.
2. On an up-to-date `main`:

   ```sh
   py -3 -m tools.waystone_roller.release check            # every version agrees; the notes exist
   py -3 -m tools.waystone_roller.release notes 0.1.0      # the release body, as it will read
   py -3 -m tools.waystone_roller.release cut --push       # tag HEAD and push the tag
   ```

   `cut` refuses uncommitted changes to tracked files, a HEAD that is not
   the tip of `origin/main`, and a tag that already exists here or on the
   remote. Without `--push` it only creates the tag and prints the push
   command.
3. The pushed tag starts `.github/workflows/waystone-roller-release.yml`. It
   checks the tag against the files, builds with Rust 1.98.1 and Node
   24.19.0 on a Windows runner, signs the exe with the release key, and
   creates a **draft** release. The draft holds `waystone-roller.exe`,
   `SHA256SUMS.txt`, `waystone-roller.exe.minisig` (the signature the app
   checks before it installs an update) and the body from step 2. Without
   the `TAURI_SIGNING_PRIVATE_KEY` secret the sign step fails the run and
   no release is made ([`docs/app-updates.md`](../app-updates.md)).
4. Open the draft on GitHub, check it, publish it, and tick "Set as the
   latest release". The workflow never publishes.

To rebuild a tag, for a failed run or a lost asset, run the workflow by
hand (Actions › Waystone Roller Release › Run workflow) with the tag. If the
release already exists, only its assets are replaced.

## Known limitations

- **Rare names and magic affix names.** A rare waystone has a random name
  and a magic one its affix names; poe2db lists neither for regular mods, so
  a token could match one by chance. Tokens are at least 3 characters to
  make that unlikely; it is not ruled out. The corruption term, a bare
  `corr` (or `!corr`), has the same exposure: a rare named "Corr..." or an
  affix name holding `corr` would be lit by `corr`, and hidden by `!corr`,
  whatever its corruption.
- **`Ctrl+Alt+W` in the game.** Measured in the built app on 2026-10-28:
  with Path of Exile 2 in front, every press reached the app and copied the
  search, and the stash search box changed only when the owner pasted.
  Whether the game binds `Ctrl+Alt+W` to anything, or still sees the
  keystroke while the app holds it, was not measured; if a game binding
  stops working while the app runs, choose another shortcut in the window.
- **The refusal tone.** Measured on 2026-10-28 with Path of Exile 2 in
  front: the first build's beep, `MessageBeep(MB_ICONHAND)` (Windows'
  "Critical Stop" sound), was not heard on five refused presses, cause not
  established; the owner then heard the 880 Hz `Beep` tone that 0.1.0
  plays instead, which reads no entry of the sound scheme (research note
  § "The built app in the game (Live 2)"). The simple beep is the fallback
  only if that call fails, and `diag.log` records `refusal beep: <sound> ok`
  or `failed (<error>)` for each sound tried; "ok" says Windows ran the
  call, not that it was heard. A muted "System Sounds" level or another
  output device could still silence the tone; neither was checked.
- **Search syntax measured on eight waystones** (2026-10-28, research note
  § "Search syntax"): Normal, Magic, Rare and corrupted Rare at tiers 1, 2
  and 15. What the game does with a lookahead such as `(?=` is not
  established (the app never writes one). The captured item texts were the
  advanced form (affix headers and roll ranges, as `Ctrl+Alt+C` copies);
  that the box searches the roll ranges is measured, while which form plain
  `Ctrl+C` gives was not separated.
- **One mod cannot be wanted alone**: "Area contains # additional Abysses"
  (How the search is built).
- **No count of modifiers**, and no value thresholds: 0.1.0 has no control
  for either.
- **English client only.**
- **The update check reads the 30 most recent releases** of the
  repository, both apps' together; a Waystone Roller release older than the
  30 newest is not seen. A network that needs a proxy Windows is not set up
  for, or that blocks `api.github.com`, `github.com` or GitHub's asset host,
  makes a check or an install fail with a one-line reason. GitHub answers
  at most 60 unauthenticated requests an hour from one IP address (GitHub's
  documentation, not measured here). Windows only.
- **The install needs a folder it can write.** It downloads into
  `waystone-roller.exe.download` beside the exe and renames the running exe
  to `waystone-roller.exe.old`; in a folder the app cannot write (under
  `Program Files`, for example) the install fails at once with "the app's
  folder cannot be written", before any download.
- **The `.old` file stays until the next start.** Windows does not let a
  running exe be deleted, so the replaced exe is left as
  `waystone-roller.exe.old` and removed by the next start, which retries
  for about 30 s while the old process ends. Until then, to roll back by
  hand: quit the app, rename `waystone-roller.exe` away and
  `waystone-roller.exe.old` back to `waystone-roller.exe`. After that, an
  older version is the earlier release's `waystone-roller.exe`, downloaded
  by hand.
- **An unsigned release cannot be installed.** A newer release without
  `waystone-roller.exe` and `waystone-roller.exe.minisig` shows "<version>
  is available but cannot be installed: the release is not signed." with no
  button; the release workflow refuses to make such a release. Releases
  made before the in-app update (0.1.0) are older than every build that
  has it, so they are never offered.
- **Antivirus and SmartScreen behaviour is not established.** The exe has
  no Authenticode signature, and how an antivirus treats an exe that
  downloads and renames its own file was not measured.
- **The install has not yet run from a real release.** It is proven by
  `ep-update`'s tests (the swap on a running copy of a test binary
  included) and by fakes and loopback servers; the first in-app install
  from a signed, published release is still to be measured.

## Maintenance triggers

- **A game patch** that adds, removes or rewords a waystone modifier: refresh
  the data from poe2db and rerun the tests.
- **poe2db's page changing** (tab ids, column headers, the markup of a
  description cell): `extract` stops with an error naming the tab; adapt
  `tools/waystone_roller/poe2db.py` and its fixture.
- **The game's search box changing** (its limit, quoting, negation, or the
  item text it searches): the research note's search syntax and
  `wr-regex`'s model and terms.
- **A dependency bump**: keep the versions equal to the quest tracker's
  (`LockedToQuestTrackerPins`); bump both together.
- **GitHub changes its API or its release flow**: the release list's
  endpoint or fields, the `/releases/download/` redirect, or its asset host
  (`release-assets.githubusercontent.com` today); or the repository, the
  `waystone-roller-v` tag prefix or the asset names change. Update the
  shared `ep-net` and `ep-update` with `tests/test_app_updates_guard.py`
  and `tools/waystone_roller/release.py` together
  ([`docs/app-updates.md`](../app-updates.md)).
- **The signing key changes** (rotated or lost): follow
  [`docs/app-updates.md`](../app-updates.md) § "Rotating the key"; copies
  built with the old key cannot install a release signed with the new one.

## Release notes

### Unreleased

- A *Settings* button in the main window opens a Settings window with
  *Automatically check for updates* (off by default; it only checks), *Check
  now*, the version and "Created by ST4H". The button says when a newer
  version is out.
- A newer version installs in the app: *Install <version> and restart*
  downloads it, checks its signature against the key built into the app,
  replaces the running exe and restarts. A failure says why, with *Try
  again*, and leaves the app as it was.
- Each release now carries `waystone-roller.exe.minisig`, the exe's
  signature.
- The app now makes network requests, to GitHub only, and only when you
  check for updates or install one ("Privacy").

### 0.1.0 (first version)

- The waystone roller: pick waystone and desecrated waystone modifiers to
  want (any or all) or avoid, a tier range, rarities and corruption, and get
  one search string for the stash or vendor search box, at most 250
  characters or refused whole.
- `Ctrl+Alt+W` (changeable) or the Copy button puts it on the clipboard.
- A refused copy (nothing selected, over 250 characters, a mod that cannot
  be told apart) plays a short tone that does not depend on your Windows
  sound scheme, and leaves the clipboard as it was.
- Mod data from poe2db.tw, fetched 2026-10-28.
- Download `waystone-roller.exe` from the repository's
  [waystone roller releases](https://github.com/S-Borkowski/ExilePact2/releases?q=waystone-roller)
  and run it; there is no installer.
- Measured in the built app on 2026-10-28: with Path of Exile 2 in front,
  `Ctrl+Alt+W` copied the search and pasting it lit the waystones the rules
  named.
