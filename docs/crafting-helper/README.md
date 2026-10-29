# Crafting Helper

A small window for Path of Exile 2 that helps you craft items. You copy an
item in the game with the game's own `Ctrl+Alt+C`, then press the app's
hotkey (`Ctrl+Alt+X` by default) or `Ctrl+V` in its window, and the window
becomes the craft window: it shows the item and its modifiers, lets you pick
the modifiers you want as targets (each with a tier floor), and previews the
route a crafting blueprint takes to them, each step with its currency, its
omens and its rough odds, or "odds unknown" and why. *Start crafting* turns
it into a guide: you take the step in the game, grab the item again, and the
app says whether the step hit or missed and what to do next. The app reads
the clipboard once per grab and never watches it. It uses no keyboard hook,
sends the game no keys, reads no game memory, moves no mouse and clicks
nothing. A button opens Craft of Exile in your web browser for the crafting
this app does not cover.

- UI direction, windows and states: [`ui-design.md`](ui-design.md)
- The design reviews of the craft window: its direction, before it was
  built ([`design/review-direction.md`](design/review-direction.md)), and
  the built pages ([`design/review-craft-window.md`](design/review-craft-window.md))
- What each crafting item does to an item's modifiers:
  [`crafting-mechanics.md`](crafting-mechanics.md)
- Research note (poe2db's modifier pages, the data extractor, the data
  files): [`crafting-helper-research.md`](crafting-helper-research.md)
- Writing and checking a crafting blueprint: [`blueprints.md`](blueprints.md)
- The item text parser the app uses:
  [`apps/crafting-helper/crates/ch-item/README.md`](../../apps/crafting-helper/crates/ch-item/README.md)
- How the app updates itself, shared with the quest tracker and the
  waystone roller: [`docs/app-updates.md`](../app-updates.md)

## Overview

- **One window, `crafting-helper.exe`.** A Tauri app: the app's logic
  (`ch-app`) is a Rust crate, and the UI is Svelte and TypeScript in an
  ordinary window titled "Crafting Helper", resizable and in the taskbar.
  Closing the window quits the app. Only one copy runs; starting a second
  brings the first to the front.
- **Grabbing an item.** In the game, hover the item and press
  `Ctrl+Alt+C`, the game's own copy of the item's text. Then either:
  - press the app's **hotkey**, `Ctrl+Alt+X` by default, a global shortcut
    that works while the game is in front. Each press reads the clipboard's
    text once; or
  - press **`Ctrl+V` in the app's window**. The window takes the text the
    paste carries. A paste into a text field of the window is that field's,
    not a grab.
- **The craft window** is the main window's craft area, in one of four
  modes:
  - **Nothing grabbed**: how to grab.
  - **Can't plan this item**: the last grab is not an item a blueprint
    plans for (a text that is not an item copied with `Ctrl+Alt+C`, or the
    engine's refusal, such as a corrupted item), with the reason in words
    and the grabbed text shown.
  - **Planning**: the item (name, rarity, base, class, item level, and its
    prefixes and suffixes with their tiers), the **targets** (the
    modifiers this base can roll at its item level, as prefixes, suffixes
    and desecrated, filterable by text; each chosen one with a floor, "As
    high as possible" by default or "At least T<n>"), **Blocked for this
    craft** (the Settings list, which this craft can add to or allow
    from, without changing Settings), and the **route** the engine chose:
    the blueprint, each step's instruction, currency and omens, its rough
    odds or "odds unknown" with the reason, a fallback line where a
    blocked item made it take another, and other applicable routes to
    choose from. With no route, one line per blueprint says why it does
    not apply. *Start crafting* starts the guide.
  - **Guiding**: the current step with its odds and two lines, where a
    hit and where a miss go; the item as last grabbed; and the history of
    each step's outcome. After taking the step in the game, copy the item
    and grab it again: the app classifies the change as a hit, a miss or
    unexpected and moves on. *End craft* goes back to planning on the
    latest item, keeping the targets.

  Each grab's text, as copied, is under a *Grabbed text* disclosure with
  its line count and where it came from ("from the hotkey" or "from a
  paste"). A text whose first line does not start with `Item Class: ` is
  still shown, with a warning that it does not look like an item copied
  with `Ctrl+Alt+C`. A grab and the craft are kept in memory only: nothing
  of either is saved, and the next grab replaces the item.
- **A grab is refused** when the clipboard or the paste holds no text, only
  whitespace, or more than 65,536 bytes (UTF-8), or when the clipboard could
  not be read. The status line says why and the last grab stays. A refused
  grab from the hotkey also plays a short tone, since the window may be
  behind the game; a refused paste does not.
- **The hotkey can be changed** in the Settings window. It can never be
  `Ctrl+Alt+C`, in any spelling, because that is the game's own item copy.
  Windows lets one program hold a key combination: when another program
  holds the one you chose, the app says so, keeps running and keeps the old
  hotkey.
- **Explore more crafting on Craft of Exile**, a button in the main
  window's header, opens one fixed address,
  `https://beta.craftofexile.com/?game=poe2`, in your default web browser.
  It is the only address the app opens (Privacy).
- **A Settings window**, opened from the *Settings* button at the right of
  the main window's header, holds three sections, *Grab* (the hotkey),
  *Blocked currencies* and *Updates*, and the line "Created by ST4H". While a
  check has found a newer version, the *Settings* button says so in words
  (`Update <version>`, beside a green dot).
- **Blocked currencies**: a saved list of crafting items (currencies,
  omens, bones, fluxes, catalysts, essences, alloys and liquid emotions)
  the helper is never to suggest, picked by name from the whole catalogue.
  A route avoids them: a step whose first choice is blocked falls back to
  an alternative and says so, and a blueprint that cannot do without one
  does not apply. Each craft starts from this list and can change it for
  itself alone.
- **An update check, only when you ask.** *Check now* asks GitHub once
  whether a newer Crafting Helper release is published; with *Automatically
  check for updates* on (off by default), the app also asks at launch and
  then once a day. A check only checks: nothing is downloaded until you
  press the install button.
- **The update installs in the app.** *Install <version> and restart*
  downloads the release's `crafting-helper.exe`, checks its signature
  against the key built into the app, puts it in place of the running exe
  and restarts into it, with a progress bar and *Cancel*. A failed step says
  why in one line, with *Try again*, and leaves the running app untouched;
  a newer release that is not signed says it cannot be installed and offers
  no button. An update never opens a release page or a browser
  ([`docs/app-updates.md`](../app-updates.md)).
- **An overlay**, a click-through, always-on-top window for the next step of
  a craft. Nothing in 0.1.0 shows it, a craft in the guide included (Known
  limitations).
- **What the craft window is built on**: the modifier data's lookups and
  odds (`ch-data`), the hand-edited blueprints under
  `apps/crafting-helper/blueprints/` and their format
  ([`blueprints.md`](blueprints.md)), and the engine that picks a blueprint
  route for an item and classifies each step as hit, miss or unexpected
  (`ch-engine`), all built into the exe.

## Platforms and prerequisites

- **Windows 10 or 11, 64-bit.**
- **The Microsoft Edge WebView2 runtime.** Windows 11 ships it, and most
  Windows 10 machines have it through Edge. `crafting-helper.exe` does not
  bundle it; without it the window cannot start.
- **The game in English.** The app's check of the first line,
  `Item Class: `, is English text.

To build it from this checkout you also need the Rust toolchain for the MSVC
target (`cargo`; built here with cargo 1.98.1), the Visual Studio C++ build
tools, and Node.js with npm. Every crate and npm package is pinned exactly,
to the quest tracker's versions; build with `--locked` and `npm ci`.

## Command reference

### `crafting-helper.exe`

Takes no arguments.

| Environment variable | Effect |
|---|---|
| `CRAFTING_HELPER_DIR` | The folder for `settings.json` and `diag.log`. Unset or empty: `%LOCALAPPDATA%\ExilePact2\crafting-helper\`. A folder inside a git checkout is refused: the app then runs without saving, and the status line says why. |

- `settings.json`: `schema` (1), `hotkey` (default `Ctrl+Alt+X`),
  `check_updates` (*Automatically check for updates*; any value but `true`,
  or none, loads it off) and `blocked_currencies` (a list of catalogue
  ids). A craft's own changes to the blocked list are never saved. Written a
  moment after each change, through a temporary file renamed over the old
  one, and when the app quits. A saved hotkey of `Ctrl+Alt+C` is replaced by
  the default at launch, and the status line says so. A saved list of
  blocked currencies keeps its valid ids and drops the rest (see the
  Settings window below for what is valid), and the status line says how
  many were dropped.
- `diag.log`: one line per event, with the UTC time. Every line the app
  writes:
  - `settings: <n>` at launch, for each note about the store or the saved
    settings that the status line also shows: the folder refused, the
    settings unreadable, the saved hotkey replaced, saved ids dropped, or
    the shipped blueprints not loading (then no craft can be planned).
  - `start: crafting-helper <version>; store <folder>` (`store none` when
    the folder was refused).
  - `hotkey <hk> registered` or `hotkey <hk> could not be registered:
    <error>`, at launch and when the hotkey is changed; `hotkey <hk>
    refused: it is the game's item copy` for `Ctrl+Alt+C`.
  - `grab from the shortcut: N lines`, `grab from a paste: N lines` or
    `grab refused: <reason>` (for example `the clipboard holds no text`,
    `the paste is empty`, `the clipboard holds N bytes of text, over
    65536`, `the clipboard could not be read (<error>)`). **The grabbed
    text is never written**, to the log or anywhere else.
  - After a refused grab from the hotkey, `refusal beep: <sound> ok` or
    `refusal beep: <sound> failed (<error>)` for each sound tried, or
    `refusal beep: none on this platform` where the app has no beep
    (outside Windows).
  - `overlay: click-through` at launch, or `error: overlay window: <e>`
    when the overlay could not be made click-through, shown or hidden;
    `overlay shown: a craft is active` and `overlay hidden: no craft is
    active` (neither happens in 0.1.0).
  - The craft window's commands, one line each: `craft: <n> targets` (`1 target`),
    `craft: <n> blocked for this craft`, `craft: route <blueprint id>`,
    `craft: accepted <blueprint id>`, `craft: ended`, or `craft: refused:
    <reason>` for one the app did not take. While guiding, each grab that
    is a step's result adds `craft: hit at <step id>`, `craft: miss at
    <step id>` or `craft: unexpected at <step id>`. Only counts, ids
    (blueprints, steps, modifiers) and reasons: no item text.
  - `opened Craft of Exile` after the button, or `error: opening Craft of
    Exile: <e>` when the browser could not be asked to open it.
  - `blocked currencies: N ids` when the list changes; `settings refused:
    <reason>` when the Settings window sends a change the app does not
    take; `error: settings window: <reason>` when that window could not be
    opened; `error: settings could not be saved: <e>`.
  - `updates: ...` lines, the same as the waystone roller's (its guide's
    Command reference): the setting turned on or off, each check's start
    and outcome, each install step, failure, cancel or refusal, the restart,
    and at every start one `updates: cleanup: ...` line
    (`removed crafting-helper.exe.old` when an install left one). No line
    holds a URL or anything of GitHub's answer.
  - `stop` when the app quits.

  Rotated at 1 MB into `diag.log.1`.

**The main window**, titled "Crafting Helper", 960 by 680 at 1x by default
and usable from 600 by 480. Its header holds the title, the *Explore more
crafting on Craft of Exile* button and the *Settings* button; under it, the
craft area, in two columns from 880 px wide and one below; last, the status
line. With nothing grabbed, the craft area says how to grab (`Ctrl+Alt+C`
in the game, then the current hotkey or `Ctrl+V` here). The status line is
the app's last message: `Grabbed an item: N lines.` (with how many targets
the new item dropped, when it cannot roll them), `Not grabbed: <reason>.`,
the warning for a text that does not look like an item, what a craft
command did or why it was refused (`Targets not changed: <reason>.` and the
like, with nothing changed), a step's outcome while guiding, why a grab
while guiding is not a step's result (the same item again, or not an item a
blueprint plans for; the craft is unchanged), or a problem with the hotkey,
the saved settings or opening Craft of Exile. Each target refers to a
modifier this base can roll at its item level, with no floor ("As high as
possible", the best tier) or a tier the item level reaches; a list with
anything else is refused whole. The window's parts and their words are
[`ui-design.md`](ui-design.md) § "Craft window".

**The Settings window**, titled "Crafting Helper Settings", opens from the
main window's *Settings* button (shown and focused when it is already
open). It is decorated, dark and not resizable, 440 by 720 at 1x, centred;
closing the main window quits the app with it. From the top:

- **Grab**: the current hotkey, a field to type a new one (`Ctrl+Alt+G`,
  for example) with *Set*, and what the app said about the last change:
  `The hotkey is now <hk>.`, `<hk> is the game's own item copy, so it
  cannot be the hotkey. The hotkey is still <old>.`, `Hotkey <hk> is taken
  by another program. The hotkey is still <old>.` and the like.
- **Blocked currencies**: the count, then the catalogue as checkboxes by
  name, grouped by kind (Currency, Omens, Bones, Fluxes, Catalysts,
  Essences, Alloys, Liquid emotions), with a *Filter currencies* field and a
  *Blocked only* checkbox; there is no field to type an id. Each change
  sends the whole list of ids; the app trims each id, drops empty ones and
  repeats, and saves the list it kept. The catalogue is the list's limit:
  every id must be one of its ids (about 220), so the list can hold the
  whole catalogue and never more. An id outside the catalogue, an id over
  64 characters, or anything but a list of text is refused whole, with
  nothing changed. A saved id the catalogue no longer holds is dropped at
  launch and counted in the status line. The craft window's *Blocked for
  this craft* uses the same picker over the craft's own list.
- **Updates**, the waystone roller's section: *Automatically check for
  updates* (off), with the helper text "Asks GitHub once a day whether a
  newer version is out. Downloads nothing: an update is installed only when
  you press its button. Off: Crafting Helper never connects unless you ask
  it to."; what the last check said (`Not checked yet.`, `Checking for
  updates…`, `You have the latest version (<running>).`, `Crafting Helper
  <new> is available. You have <running>.`, `No Crafting Helper release is
  published yet.` or `Could not check for updates: <reason>.`); the
  install's line and its one button (*Install <new> and restart*, the
  download with its bar and *Cancel*, *Try again* after a failure, or none
  for an unsigned release); **Check now**, disabled while a check or an
  install runs; and the running version. The reasons are the quest
  tracker's (its guide's Command reference). With the check on, it runs at
  launch, again when it is turned on, and then a day after the last check
  started, or an hour after an automatic check failed.
- "Created by ST4H" at the bottom.

The main window's commands are `get_view`, `grab_text` (the paste's text),
`open_settings`, and the craft window's `set_targets` (the whole list of
targets, each a modifier id and a floor or none), `set_craft_blocked` (the
craft's whole blocked list, catalogue ids only; never saved),
`choose_route` (a blueprint id the preview offers), `accept_craft`,
`end_craft` and `open_craft_of_exile`, which takes no argument: the
address is the app's own. Each craft command answers with the view. Both
windows may send `get_catalogue`, which answers the catalogue grouped by
kind, each entry's id and English name. The main window is sent `ch:view`,
which carries the craft part (its mode and what the mode shows), the
hotkey, the last grab, the status line, the overlay's state and
`update_available`. The Settings window's commands are `get_app_settings`,
`set_hotkey`,
`update_settings` (an object of `check_updates` and `blocked_currencies`,
either or both, and nothing else), `check_for_updates`, `install_update`
(the only way an install starts) and `cancel_update`; it is sent
`ch:app-settings`. The overlay listens to `ch:view` and sends nothing but
`get_view`. The newest release is the highest published, non-prerelease
release tagged `crafting-helper-v<major>.<minor>.<patch>`.

### npm scripts

Run from `apps/crafting-helper/` (or with `npm --prefix apps/crafting-helper`).

| Script | What it does |
|---|---|
| `npm run dev` | The pages alone in a browser at `http://127.0.0.1:1422/`, over the view fixtures (`?state=empty`, `?state=craft-planning`, `settings.html?state=install-downloading`, `overlay.html`; [`ui-design.md`](ui-design.md) § "Browser preview"). |
| `npm run build` | Type-checks the pages and writes `dist/`, which the executable embeds. Needed before any `cargo` build of the `crafting-helper` package. |
| `npm test` | The pages' tests, in jsdom. |
| `npm run start` | The app under `tauri dev`. |
| `npm run release` | `tauri build --no-bundle`: `target/release/crafting-helper.exe`. |

The data extractor, `py -3 -m tools.crafting_helper.poe2db`, is described in
the research note:
[`crafting-helper-research.md#data-extractor`](crafting-helper-research.md#data-extractor).

## Privacy

- **The clipboard is read only when you grab.** A press of the app's hotkey
  reads the clipboard's text once (if another program holds the clipboard,
  the app tries again for about 100 ms, then refuses). A `Ctrl+V` in the
  app's window hands the page the text of that paste, which it sends to the
  app; nothing else reads the clipboard for it. The app **never watches the
  clipboard**: no read at launch, on a timer, on focus or from any other
  command, no clipboard listener and no change counter. It never writes to
  the clipboard.
- **No keyboard hook.** The hotkey is a registered system hotkey (Windows'
  `RegisterHotKey`, through Tauri's global-shortcut plugin): Windows tells
  the app when that one combination is pressed, and the app sees no other
  key. It does not poll the keyboard's state either.
- **It sends the game nothing.** No key, no mouse input or movement, no
  click, no typed text, no window message, no focus or activation change,
  no other process opened (but the web browser Windows starts when you
  press the Craft of Exile button, below), no game memory read and none of
  the game's files. The overlay never takes focus and lets every click
  through.
- **What it keeps of a grab**: the text, in memory, until the next grab or
  until the app quits, and the craft built on it (the item, the targets,
  the craft's blocked changes, the guide and its history), in memory until
  the app quits. None of it is saved, and none of the text is logged;
  `diag.log` gets only the line count or the reason a grab was refused,
  and the craft's counts, ids, refusal reasons and outcome words.
- **Its other effects outside its windows** are the refusal tone, the
  update check's requests to GitHub, opening Craft of Exile in the default
  browser when you press its button and, when you press the install
  button, replacing its own exe.
- **One web address, opened only by its button.** *Explore more crafting
  on Craft of Exile* asks Windows to open
  `https://beta.craftofexile.com/?game=poe2` in your default browser, and
  nothing else: the address is fixed in the app (`ch-app`'s
  `CRAFT_OF_EXILE_URL`), no command takes an address, and the pages hold
  none and cannot open one themselves (no link, no `window.open`; the
  opener plugin's own commands are granted to no window). The app itself
  sends nothing for it, and nothing of your item, targets or craft goes
  with it; the page loads in your browser, which connects to Craft of
  Exile as for any page you open (Craft of Exile is not part of this
  project). The guard pins each of these
  (`tests.test_crafting_helper_guard`).
- **Network: only the update check and the install, only when you ask.**
  With *Automatically check for updates* off (the default) and no *Check
  now* pressed, the app sends and receives nothing (the Craft of Exile page
  is your browser's connection, not the app's). A check is one `GET
  https://api.github.com/repos/S-Borkowski/ExilePact2/releases?per_page=30`,
  port 443 only, from Rust through Windows' own WinHTTP, with three
  headers: `Accept: application/vnd.github+json`, `X-GitHub-Api-Version:
  2022-11-28` and `User-Agent: ExilePact2-crafting-helper` (no version, no
  machine detail). The answer is read up to 2 MiB within 20 s, used only to
  find the newest release, and never stored or logged.
- **An install** (the button only) makes two downloads, the signature, then
  the exe: `GET
  https://github.com/S-Borkowski/ExilePact2/releases/download/crafting-helper-v<version>/crafting-helper.exe.minisig`
  and the same path ending `crafting-helper.exe`, each with two headers,
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
  (`tests.test_crafting_helper_guard`). (WebView2, Microsoft's runtime that
  draws the windows, may connect on its own, as for the quest tracker.) The
  data extractor asks poe2db.tw for pages only when a developer runs it.
- **What it writes**: `settings.json` and `diag.log` in its folder (Command
  reference). The log holds the hotkey, the grabs' line counts, refusals,
  the blocked list's length, the `craft:` lines, the Craft of Exile line
  and the `updates:` lines; nothing about your account or character, no
  item text, and nothing of GitHub's answers. An
  install also writes beside the exe: `crafting-helper.exe.download` while
  it downloads (deleted when the install fails or is cancelled) and
  `crafting-helper.exe.old`, the replaced exe, which the next start removes.

## Working on the app

| Path | What it holds |
|---|---|
| `apps/crafting-helper/crates/ch-app/` | The app's logic without Tauri: the store, the grab's checks and normalisation (`grab.rs`), the hotkey's rules (`hotkey.rs`), the saved settings, the blocked currencies and the catalogue grouped for the pickers (`settings.rs`), the craft window (`craft.rs`: its four modes in memory, what a grab does to it, its commands' rules, its view from the engine's plan, guide and outcomes, the copy the design marks Rust's, and `CRAFT_OF_EXILE_URL`), the views (`view.rs`, the IPC contract `src/ipc.ts` mirrors), the commands, the overlay's state (`App::set_craft_active`, which nothing calls yet), and the update commands and ticker (`updates.rs`, over the shared `ep-update`). Tested through a recording fake shell, a fake transport and a temporary store. |
| `apps/crafting-helper/crates/ch-item/` | The item text parser, a std-only crate; the craft window parses each grab with it: [`crates/ch-item/README.md`](../../apps/crafting-helper/crates/ch-item/README.md). |
| `apps/crafting-helper/crates/ch-data/` | The modifier data under `data/`, embedded at build time: an item's variant from its base type, what it can roll at its item level, the join of each parsed modifier to its data modifier and tier, the affix limits (`AFFIX_LIMITS`), and one step's add and removal odds, or `unknown` with the reason. |
| `apps/crafting-helper/crates/ch-blueprint/` | The blueprint format: the model (`model.rs`), the static catalogue of currencies, omens, bones, fluxes and catalysts with their ids (`catalogue.rs`), the structural validator whose every error is `<path>:<line>:<col>: <message>` (`validate.rs`), and the shipped blueprints, embedded by `build.rs`. |
| `apps/crafting-helper/crates/ch-engine/` | The engine: blueprints resolved against the data, route choice for an item, its targets and the blocked currencies (`route.rs`), the guide's outcomes (`guide.rs`), the full catalogue with the data-backed essences, alloys and liquids, and the `check_blueprints` example. `ch-app` re-exports it and plans, guides and fills the blocked list's catalogue with it. |
| `apps/crafting-helper/blueprints/` | The shipped blueprints, one TOML file each: how to write one is [`blueprints.md`](blueprints.md). |
| `apps/crafting-helper/src-tauri/` | The Tauri glue: the global hotkey, single instance, the Win32 clipboard read and the refusal tone, the commands, the Settings window (label `settings`) and the overlay (label `overlay`, made click-through at start and shown without taking focus), each with its capability under `capabilities/`, opening Craft of Exile's address through `tauri-plugin-opener` from Rust alone (registered without its link handler, no `opener:` permission in any capability), the update transport over the shared `ep-net`, and the restart into an installed update. |
| `apps/crafting-helper/src/` | The main page (`App.svelte`; the shared parts under `components/`: the grab prompt, the grabbed text, the status line and `CurrencyPicker.svelte`; the craft area under `craft/`, one component per part, from `CraftArea.svelte`; the commands `bridge.ts`, the paste route `paste.ts`, the view's types `ipc.ts`), the Settings window's page under `src/settings/` (`settings.html`, one component per section, listed in `sections.ts`) and the overlay's under `src/overlay/` (`overlay.html`). |
| `apps/crafting-helper/theme/` | The quest tracker's theme, copied byte for byte (`theme.test.ts`). |
| `apps/crafting-helper/data/` | The modifier data the extractor writes ([research note](crafting-helper-research.md#data-files)), embedded by `ch-data` when the app is built. |
| `apps/crafting-helper/fixtures/` | `views/`, one main window view per state (the craft window's included, [`views/README.md`](../../apps/crafting-helper/fixtures/views/README.md)), `app-settings/`, one Settings window view per update, install and settings state, and `catalogue.json`, the `get_catalogue` answer, shared by the page's tests and `ch-app`'s. |
| `apps/shared/` | The update code the three apps share, its own Cargo workspace: `ep-net` (the only network code) and `ep-update`: [`docs/app-updates.md`](../app-updates.md). |
| `tools/crafting_helper/` | The poe2db data extractor (`poe2db.py`) and the release cutter (`release.py`). |
| `.github/workflows/crafting-helper-release.yml` | The release build: from a pushed tag to a draft release. |

Tests (none opens a window, reads a real clipboard or reaches the network):

```sh
npm --prefix apps/crafting-helper run build                                            # needed before the workspace build
cargo test --manifest-path apps/crafting-helper/Cargo.toml --workspace --locked         # every crate, ch-item included
cargo test --manifest-path apps/crafting-helper/Cargo.toml -p ch-app --locked --test grab     # the grab, fake shell
cargo test --manifest-path apps/crafting-helper/Cargo.toml -p ch-app --locked --test hotkey   # the hotkey's rules
cargo test --manifest-path apps/crafting-helper/Cargo.toml -p ch-app --locked --test updates  # the update commands, fake transport
cargo test --manifest-path apps/crafting-helper/Cargo.toml -p ch-app --locked --test craft_window  # the craft window, fake shell
cargo test --manifest-path apps/crafting-helper/Cargo.toml -p ch-app --locked --test settings # the saved settings, the blocked list, the catalogue
cargo test --manifest-path apps/crafting-helper/Cargo.toml -p ch-data --locked         # the data: join, valid modifiers, odds
cargo test --manifest-path apps/crafting-helper/Cargo.toml -p ch-blueprint --locked    # the format, the catalogue, the validator
cargo test --manifest-path apps/crafting-helper/Cargo.toml -p ch-engine --locked       # route choice, outcomes, the seeds' paths
cargo run --manifest-path apps/crafting-helper/Cargo.toml -p ch-engine --locked --example check_blueprints -- apps/crafting-helper/blueprints  # check blueprints
npm --prefix apps/crafting-helper test                                                  # the pages, in jsdom
py -3 -m unittest tests.test_crafting_helper_guard                                      # the guard
py -3 -m unittest tests.test_crafting_helper_release                                    # the release cutter and workflow
py -3 -m unittest tests.test_crafting_helper_poe2db tests.test_crafting_helper_data     # the data extractor and the shipped data
cargo test --manifest-path apps/shared/Cargo.toml --locked --workspace                   # the shared update crates, 127.0.0.1 only
py -3 -m unittest tests.test_app_updates_guard                                          # the shared crates' guard
```

- `ch-app`'s `tests/grab.rs` counts the fake shell's clipboard reads: one
  per hotkey press (`grab_reads_the_clipboard_once_per_press`) and none for
  a paste (`grab_from_paste_reads_no_clipboard`). `tests/hotkey.rs` covers
  the default and `Ctrl+Alt+C` in every spelling; `tests/settings.rs` the
  saved settings, the blocked list (catalogue ids only, the whole
  catalogue allowed) and the catalogue's groups; `tests/overlay.rs` the
  overlay's visibility.
- `ch-app`'s `tests/craft_window.rs` drives the craft window over the
  whittled ring of `ch-engine`'s tests
  (`crates/ch-engine/tests/items/gear/rings-resist-whittle/`, read by
  path): the targets a base offers and their floors, refusals, a route
  with odds in words, a blocked currency's fallback and a craft's own
  blocked list, no route, accepting, a hit, a miss, an unexpected change,
  an unchanged re-grab and the walk to done; that the overlay stays hidden
  while guiding (`overlay_stays_hidden_while_guiding`); and that
  `open_craft_of_exile` asks the shell for the one fixed address
  (`open_craft_of_exile_opens_only_the_fixed_address`).
- `ch-app`'s `tests/fixtures.rs` (`fixtures_are_current`) writes the
  views under `fixtures/views/` and `fixtures/app-settings/` and
  `fixtures/catalogue.json`, and fails when a file differs. After a deliberate change, regenerate them with
  `CH_FIXTURES=1` before `cargo test --manifest-path
  apps/crafting-helper/Cargo.toml -p ch-app --locked --test fixtures`, and
  review the diff. The page's tests render each one.
- `tests.test_crafting_helper_guard` reads the files only: no clipboard
  watching (no clipboard listener, viewer or sequence number) and no
  keyboard polling in any Rust source; no `navigator.clipboard` in the UI
  outside its tests; no input sent, no game access and no network but the
  shared `ep-net`, reached only from `src-tauri`; no Tauri plugin but the
  global shortcut, single instance and opener; one web address in the Rust
  sources, Craft of Exile's, once, in `ch-app`, the opener named and called
  once in the glue and registered without its link handler, and no command
  taking an address or a path (`open_craft_of_exile` takes only the app
  state); no address, `window.open`, change of `location`, new-window link
  or opener command in the pages; the pinned CSP, the three windows'
  capabilities and the overlay window's settings; one version everywhere
  (`ch-item`'s included); every member crate but `ch-item` taking its
  package keys and external dependencies from the workspace; and every
  package already in the quest tracker's locks but `minisign-verify`. The
  one workspace pin the quest tracker's workspace lacks, `toml` (for the
  blueprint files), must be a version its `Cargo.lock` already holds.
- The item parser's own tests and fixtures are described in
  [`crates/ch-item/README.md`](../../apps/crafting-helper/crates/ch-item/README.md).
- `ch-data`'s `tests/data.rs` joins the owner's five real items (the
  parser's `fixtures/real/`) to the data and checks the anchored odds
  (`ring_life_exalt_odds_match_the_weights`); the rest of its items are
  constructed in the test. `ch-blueprint`'s `tests/format.rs` reads the
  files under `tests/fixtures/valid/` and `tests/fixtures/invalid/` and
  checks each error's line and column. `ch-engine`'s `tests/engine.rs`
  covers route choice and outcomes; `tests/seeds_gear.rs` and
  `tests/seeds_jewels_weapons.rs` walk each shipped blueprint on a hit and
  a miss path, over the item texts under `tests/items/` (each folder's
  `README.md` says which are real and which constructed); and
  `tests/shipped.rs` validates and resolves every file under
  `blueprints/`. After editing a blueprint, run `check_blueprints` (above)
  and the `ch-engine` tests: [`blueprints.md`](blueprints.md).

**Refreshing the modifier data**: see the research note's
[Data extractor](crafting-helper-research.md#data-extractor).

**Changing the icon**: `src-tauri/icons/source.png` is the one source.
Regenerate the icon files with
`npm --prefix apps/crafting-helper exec tauri icon apps/crafting-helper/src-tauri/icons/source.png -- --output apps/crafting-helper/src-tauri/icons`,
delete what it writes beyond the files `src-tauri/tauri.conf.json`'s
`bundle.icon` names (plus `64x64.png` and `icon.png`), then run
`cargo clean --manifest-path apps/crafting-helper/Cargo.toml --release -p crafting-helper`
before `npm --prefix apps/crafting-helper run release`: an incremental build
can keep the old `icon.ico` in the exe (seen for the waystone roller).

**Cutting a release.** A release is the annotated tag
`crafting-helper-v<version>` on the tip of `main`. The version is the one in
`apps/crafting-helper/`'s `package.json`, `package-lock.json`,
`src-tauri/tauri.conf.json`, the workspace's `Cargo.toml`,
`crates/ch-item/Cargo.toml` (which writes its version literally) and
`Cargo.lock` (every workspace member: `ch-app`, `ch-item` and
`crafting-helper`). These must agree, and "Release notes" below needs a
`### <version>` section for it:

1. Bump the version in those files (build once so `Cargo.lock` follows),
   and write the version's release notes. Land both on `main` through a
   pull request.
2. On an up-to-date `main`:

   ```sh
   py -3 -m tools.crafting_helper.release check            # every version agrees; the notes exist
   py -3 -m tools.crafting_helper.release notes 0.1.0      # the release body, as it will read
   py -3 -m tools.crafting_helper.release cut --push       # tag HEAD and push the tag
   ```

   `cut` refuses uncommitted changes to tracked files, a HEAD that is not
   the tip of `origin/main`, and a tag that already exists here or on the
   remote. Without `--push` it only creates the tag and prints the push
   command.
3. The pushed tag starts `.github/workflows/crafting-helper-release.yml`. It
   checks the tag against the files, builds with Rust 1.98.1 and Node
   24.19.0 on a Windows runner, signs the exe with the release key, and
   creates a **draft** release. The draft holds `crafting-helper.exe`,
   `SHA256SUMS.txt`, `crafting-helper.exe.minisig` (the signature the app
   checks before it installs an update) and the body from step 2. Without
   the `TAURI_SIGNING_PRIVATE_KEY` secret the sign step fails the run and
   no release is made ([`docs/app-updates.md`](../app-updates.md)).
4. Open the draft on GitHub, check it, publish it, and tick "Set as the
   latest release". The workflow never publishes.

To rebuild a tag, for a failed run or a lost asset, run the workflow by
hand (Actions › Crafting Helper Release › Run workflow) with the tag. If the
release already exists, only its assets are replaced.

## Known limitations

- **You grab the item again by hand after each step.** The guide learns a
  step's result only from your next grab (`Ctrl+Alt+C` in the game, then
  the hotkey or `Ctrl+V`); it does not read the item's tooltip by itself.
  Reading the tooltip on screen is a later version's.
- **A craft is not saved.** The item, the targets, the craft's blocked
  changes, the guide and its history live in memory: quitting the app ends
  the craft, and the next start begins with nothing grabbed.
- **The crafting engine's limits** show in the craft window:
  - **Odds are often unknown.** Every jewel (poe2db shows no spawn weights
    for jewels), every desecration and reveal (no established weights; a
    reveal may also offer normal-pool modifiers, whose share is not
    established), every flux (how it maps tiers is not established), and a
    target whose weight is the site's placeholder 1 give `unknown` with the
    reason. Known odds are one step's rough chance, never a simulation or
    an expected cost.
  - **The affix limits are assumed**: Magic 1 prefix and 1 suffix, Rare 3
    and 3, Rare jewel 2 and 2, one more on the other side with a Contempt
    liquid emotion. The owner's figures, not measured from the game.
  - **The tier join is measured on five items.** The tier the engine
    derives from poe2db's levels equalled the game's `Tier:` on all 18
    joined modifiers of the owner's five real items
    (`real_fixture_tiers_equal_game_tiers`); a class or modifier none of
    them carries has not been compared.
  - **A blueprint cannot branch into another**, and steps resting on a
    mechanic not established (alloys with Crystallisation omens, flux tier
    mapping, catalyst quality, rune sockets) are kept and said so in the
    blueprint's `notes`; catalyst and rune steps are manual.
- **The overlay never shows in 0.1.0.** It appears only while a craft is
  active, and nothing marks one active: a craft in the guide leaves it
  hidden, and the current step is read in the main window. It is not
  placed over the game and does not hide by itself when the game is not in
  front.
- **The "looks like an item" check is the first line only.** A text is
  taken for an item when its first line starts with `Item Class: `; every
  item text the owner copied for the parser (`ch-item`'s
  `fixtures/real/`) does, but the check reads nothing else. A text that
  fails it is still shown, with the warning.
- **`Ctrl+Alt+X` with the game in front was not measured.** A hotkey
  registered this way reached the waystone roller with Path of Exile 2 in
  front (`Ctrl+Alt+W`, measured 2026-10-28;
  [`docs/RUNTIME_DATA_MODELS.md`](../RUNTIME_DATA_MODELS.md) § "Item search
  box (stash and vendor)"). The crafting helper's own key, and whether the
  game binds `Ctrl+Alt+X` or still sees it while the app holds it, have not
  been tried in the game. If a game binding stops working while the app
  runs, choose another hotkey in Settings.
- **A grab from the hotkey needs the copy first.** The hotkey reads what is
  on the clipboard now: press it after the game's `Ctrl+Alt+C` has copied
  the item. Pressed before, it shows whatever text was there, with the
  warning when that is not an item.
- **The refusal tone** is the waystone roller's: an 880 Hz `Beep`, with
  Windows' simple beep as the fallback. A muted "System Sounds" level or
  another output device could silence it; that was not checked.
- **English client only.**
- **The update check reads the 30 most recent releases** of the
  repository, all three apps' together; a Crafting Helper release older
  than the 30 newest is not seen. A network that needs a proxy Windows is
  not set up for, or that blocks `api.github.com`, `github.com` or GitHub's
  asset host, makes a check or an install fail with a one-line reason.
  GitHub answers at most 60 unauthenticated requests an hour from one IP
  address (GitHub's documentation, not measured here). Windows only.
- **The install needs a folder it can write.** It downloads into
  `crafting-helper.exe.download` beside the exe and renames the running exe
  to `crafting-helper.exe.old`; in a folder the app cannot write (under
  `Program Files`, for example) the install fails at once with "the app's
  folder cannot be written", before any download.
- **The `.old` file stays until the next start.** Windows does not let a
  running exe be deleted, so the replaced exe is left as
  `crafting-helper.exe.old` and removed by the next start, which retries
  for about 30 s while the old process ends. Until then, to roll back by
  hand: quit the app, rename `crafting-helper.exe` away and
  `crafting-helper.exe.old` back to `crafting-helper.exe`.
- **Antivirus and SmartScreen behaviour is not established.** The exe has
  no Authenticode signature, and how an antivirus treats an exe that
  downloads and renames its own file was not measured.
- **The built exe has not been run with the game.** 0.1.0 is proven by
  tests with fakes (a fake shell for the clipboard, the hotkey and the
  browser, a fake transport and loopback servers for updates); no live
  session was part of it. The craft window's walks are the engine's test
  items (constructed ring texts), not crafts taken in the game.
- **Craft of Exile's page is outside this project.** The button opens its
  beta address with `?game=poe2`; if the site moves or drops that address,
  the button opens whatever the site serves there until the app is
  updated.

## Maintenance triggers

- **The game's copied item text changing** (its first line, or the
  `Ctrl+Alt+C` binding): the "looks like an item" check in `ch-app`'s
  `grab.rs` and the parser in `ch-item`.
- **A game patch** that adds, removes or rewords modifiers or crafting
  items: refresh the data with the extractor and update
  [`crafting-mechanics.md`](crafting-mechanics.md).
- **A data refresh**: an essence, alloy or liquid emotion renamed or
  removed breaks every blueprint that names its id, and a new crafting item
  needs a catalogue entry in `ch-blueprint`'s `catalogue.rs`. Run
  `check_blueprints` on `apps/crafting-helper/blueprints/` and the
  `ch-data` and `ch-engine` tests ([`blueprints.md`](blueprints.md)); the
  odds anchors in `ch-data`'s tests change with the weights. A catalogue
  id that goes away is also dropped from saved blocked lists at the next
  start, and `fixtures/catalogue.json` and the craft window's fixtures
  change with it (`CH_FIXTURES=1`, Working on the app).
- **poe2db's pages changing**: the extractor stops with an error naming the
  page; see the research note.
- **Craft of Exile's address changing** (the site leaving
  `beta.craftofexile.com`, or `?game=poe2` no longer choosing Path of
  Exile 2): change `CRAFT_OF_EXILE_URL` in `ch-app`'s `craft.rs` and the
  guard's `CRAFT_OF_EXILE_URL` in `tests/test_crafting_helper_guard.py`
  together, with this guide's Privacy section; it stays the one address.
- **A dependency bump**: keep the versions equal to the quest tracker's
  (`LockedToQuestTrackerPins`); bump them together.
- **GitHub changes its API or its release flow**: the release list's
  endpoint or fields, the `/releases/download/` redirect, or its asset host;
  or the repository, the `crafting-helper-v` tag prefix or the asset names
  change. Update the shared `ep-net` and `ep-update` with
  `tests/test_app_updates_guard.py` and `tools/crafting_helper/release.py`
  together ([`docs/app-updates.md`](../app-updates.md)).
- **The signing key changes** (rotated or lost): follow
  [`docs/app-updates.md`](../app-updates.md) § "Rotating the key"; copies
  built with the old key cannot install a release signed with the new one.

## Release notes

### 0.1.0 (first version)

- The crafting helper's first version: copy an item in the game with
  `Ctrl+Alt+C`, then press `Ctrl+Alt+X` (changeable, never `Ctrl+Alt+C`)
  or `Ctrl+V` in the window, and the window becomes the craft window.
- The craft window shows the item and its modifiers with their tiers. Pick
  the modifiers you want as a target, from the prefixes, suffixes and
  desecrated modifiers the item's base can roll, each "as high as
  possible" or "at least" a tier, and it previews the route a blueprint
  takes: each step's currency and omens, its rough odds or "odds unknown"
  and why, the fallbacks taken because something is blocked, or why no
  blueprint applies.
- *Start crafting* guides you step by step: take the step in the game,
  grab the item again, and the app says whether it was a hit, a miss or
  unexpected, moves to the next step and keeps a history. A craft is kept
  only until the app quits.
- An *Explore more crafting on Craft of Exile* button opens Craft of Exile
  in your web browser, the only page the app ever opens.
- The app reads the clipboard once per grab and only then. It never watches
  the clipboard, uses no keyboard hook and sends the game nothing.
- A Settings window with the hotkey, a saved list of blocked currencies
  picked by name from every crafting item (each craft can change it for
  itself), *Automatically check for updates* (off by default; it only
  checks), *Check now*, the in-app install of a signed newer version, the
  version and "Created by ST4H".
- Download `crafting-helper.exe` from the repository's
  [crafting helper releases](https://github.com/S-Borkowski/ExilePact2/releases?q=crafting-helper)
  and run it; there is no installer.
