# Quest tracker

A small always-on-top overlay for Path of Exile 2 that lists the campaign
quests, says which ones are worth doing and why, and keeps each character's
progress. It learns that progress from the game's `Client.txt` log and from
nothing else.

- Design brief (what the overlay shows, frames, icons, tokens, the export
  contract): [`ui-design-brief.md`](ui-design-brief.md)
- Research note (which quest steps leave a trace in the log, measured and
  labelled): [`quest-tracker-research.md`](quest-tracker-research.md)
- Facts about the log every mod can use:
  [`docs/RUNTIME_DATA_MODELS.md`](../RUNTIME_DATA_MODELS.md) § "Client log
  (Client.txt)"
- Harvested design (reference screenshots, component spec, reviews):
  [`design/`](design/)

## Overview

What it is:

- **One window, `quest-tracker.exe`.** A Tauri app: the log tailer, parser,
  quest engine, view-model and per-character store are Rust crates; the UI is
  Svelte and TypeScript in a transparent, undecorated, always-on-top window
  that stays off the taskbar. In play mode it never takes focus from the
  game; edit mode does, so its fields take typing.
- **A campaign quest list** (Acts 1 to 6 as poe2db numbers them, 87 quests in
  this version): each quest's status (locked, available, in progress, done,
  skipped, unknown), whether it is worth doing (`yes` for permanent stats such
  as passive points, spirit and resistances, `optional` for unlocks, gems and
  lasting items, `no` for story-only or currency-only content) and the reason,
  its rewards, its current step, and a link to its poe2db page.
- **Two modes.** *Play* is click-through: the mouse goes to the game and the
  panel shows no controls, and the window never takes focus from the game.
  *Edit* takes clicks and keyboard focus (so the Settings fields can be
  typed in): tick steps, mark quests done or skipped, switch or pin a
  character, open settings, drag the panel. The global shortcut (default
  `Ctrl+Alt+Q`) switches between them.
- **Two views.** *Next up* lists only what is available or in progress now,
  grouped by worth; *All* lists every act's quests under the act's header.
- **Toasts** when a quest becomes available, a step is done or a quest is
  done, while you play (never for lines read as catch-up).
- **A replay CLI**, `quest-tracker-replay`, that runs the same engine over a
  saved log without the game.
- **A developer-only Python tool**, `py -3 -m tools.quest_tracker.poe2db`,
  that fetches poe2db.tw's quest pages to rebuild the quest data after a
  patch.

What it is not:

- It does not read game memory, hook, inject, send input to the game or talk
  to Grinding Gear Games. It reads one log file and draws its own window.
- It does not track party members, endgame or Atlas quests, or a client in a
  language other than English.
- It is not an installer: there is no setup, auto-update, code signing or
  bundled WebView2 runtime.
- It makes no network request of its own. The one link it opens, a quest's
  poe2db page, opens in your default browser.

## Platforms and prerequisites

- **Windows 10 or 11, 64-bit.**
- **The Microsoft Edge WebView2 runtime.** Windows 11 ships it, and most
  Windows 10 machines have it through Edge. `quest-tracker.exe` does not
  bundle it; without it the window cannot start.
- **The game in windowed or borderless windowed fullscreen.** The game
  records the display mode in `Documents\My Games\Path of Exile 2\poe2_production_Config.ini`;
  the overlay needs `fullscreen=false` with
  `borderless_windowed_fullscreen=true` (or plain windowed).
- **Exclusive fullscreen cannot work.** In exclusive fullscreen the game owns
  the display output and the desktop compositor does not draw other windows
  over it, so an always-on-top window is hidden behind it or makes the game
  drop out of fullscreen. No setting of the overlay changes that.
- **A `Client.txt` the game is writing.** The standalone client's is
  `C:\Program Files (x86)\Grinding Gear Games\Path of Exile 2\logs\Client.txt`
  (measured). The Steam client's is assumed to be
  `C:\Program Files (x86)\Steam\steamapps\common\Path of Exile 2\logs\Client.txt`
  (UNVERIFIED: no Steam install was available to check). Elsewhere, pass
  `--log` or set the log path in Settings.

To build it from this checkout you also need the Rust toolchain for the MSVC
target (`cargo`; it was built here with cargo 1.98.1), the Visual Studio C++
build tools, and Node.js with npm. Every crate and npm package is pinned
exactly; build with `--locked` and `npm ci`.

## Command reference

All commands run from the checkout root.

### `quest-tracker.exe`

```text
usage: quest-tracker [--log PATH] [--data PATH] [--from-start]
```

| Option | What it does |
|---|---|
| `--log PATH` | Follow this log. Ahead of the Settings log path and of discovery. Taken as given even if the file does not exist yet (the tracker waits for it). |
| `--data PATH` | Use this quest data file instead of the `data/quests.json` built into the executable. A file that fails validation falls back to the built-in data with a status line saying why. |
| `--from-start` | Read the whole existing log as catch-up, ignoring the saved position. Useful once after a fresh install with an existing log; catch-up lines never raise toasts. |

Each option also takes the `--name=value` form. An unknown argument is not
fatal: the overlay starts without options and says so in its status line and
diagnostic log. A second launch while one is running is refused (single
instance).

Which log it follows: `--log`, else the Settings log path, else the first of
the standalone and Steam paths above whose file exists. With none, the panel
shows the "no log found" state.

Settings (edit mode, the settings frame) and their defaults: opacity `0.9`
(0.3 to 1), dock `right` (`left`|`right`), scale `auto` (`auto`|`1`|`1.33`|`2`,
applied as the webview zoom; see below), show skipped quests (off), only worth-it quests
(off), log path (unset: discovery), hotkey `Ctrl+Alt+Q` (the global-shortcut
plugin's accelerator syntax). A hotkey that cannot be registered keeps the
old one working and says so in the status line. If the shortcut cannot be
registered at launch, the overlay starts in edit mode instead of play
mode, since without a shortcut play mode could not be left: the status line
says so, and the diagnostic log names the `settings.json` whose `"hotkey"`
you can change.

Scale `auto` sizes the panel by the monitor the window is on: 1x below
1440 px of monitor height, 1.33x from 1440 and 2x from 2160 (physical
pixels), divided by Windows' DPI scale for that monitor (which the webview
already applies), and never below 1. So a 1440p screen at 100% gets 1.33x,
a 4K screen at 150% gets 1.33x and at 200% gets 1x, and a 1080p screen at
150% stays at Windows' 150%. Auto is worked out at start, after the
window is placed (so from the monitor it was placed on), when the scale
setting changes, and again whenever the window moves to another monitor or
that monitor's DPI scale changes; the zoom is applied only when it differs
from the one already applied, and a docked window is placed again after
it. A fixed scale never reads the monitor. The diagnostic log records what
Auto chose from (`scale auto: monitor … px high at DPI scale … -> zoom …`
at start, `scale auto: the window is now on a monitor … -> zoom …` after a
move or a DPI change).

Build it:

```sh
npm --prefix apps/quest-tracker ci            # only when apps/quest-tracker/node_modules is missing
npm --prefix apps/quest-tracker run release   # tauri build --no-bundle
# -> apps/quest-tracker/target/release/quest-tracker.exe
```

### `quest-tracker-replay`

The same engine over a saved log, without the game or a window. Build it
with `cargo build --manifest-path apps/quest-tracker/Cargo.toml -p qt-replay
--locked --release` (it lands in `apps/quest-tracker/target/release/`), or
run it in place:

```sh
cargo run --manifest-path apps/quest-tracker/Cargo.toml -p qt-replay --locked -q -- state path/to/Client.txt
```

```text
Usage: quest-tracker-replay <command> <log> [--data PATH] [--json] [--speakers NAMES]
```

| Command | Prints |
|---|---|
| `events` | One recognised event per line (never chat, unknown speakers or other players' names). |
| `state` | The engine over the whole log, no store: per own character its class, level and quests by status per act. With `--json`, one object `{"characters": [{"name", "class", "level", "quests": {<id>: <status>}}]}`. |
| `import` | `state`, then written to the store (`QUEST_TRACKER_DIR`, else `%LOCALAPPDATA%\ExilePact2\quest-tracker`) with the position at the end of the log, so the overlay resumes from there. Running it twice leaves the same files. |
| `scan` | Per area code: display name, entries, reward lines, points lines, allowlisted speakers and system lines, as counts (no chat, no character names). The input for building the quest data. |

| Option | What it does |
|---|---|
| `--data PATH` | Quest data file (default: the `data/quests.json` built in). |
| `--json` | JSON output (`events`: one object per line). |
| `--speakers NAMES` | Extra speakers to allowlist for `events` and `scan`, comma-separated; may be repeated. |
| `-h`, `--help` | The usage text. |

The log is opened read-only. Exit codes: `0` done, `1` error, `2` no such log
file, `64` usage error.

### `py -3 -m tools.quest_tracker.poe2db` (developer-only)

The only network code in this feature, and it only asks poe2db.tw. Standard
library only.

| Command | Options | What it does |
|---|---|---|
| `fetch` | `--cache DIR`, `--delay S` (default 3, refused under 2), `--only SLUG ...`, `--max-age DAYS` (default 7), `--include-endgame` | Reads `robots.txt` and skips what it disallows, then fetches the quest index and the campaign quest pages one at a time, `--delay` seconds apart, with a User-Agent naming the tool and this repository. Pages cached within `--max-age` days are left alone. Act 10 and act-less MasterMission quests only with `--include-endgame`. |
| `extract` | `--cache DIR`, `--out FILE` (default `<cache>\quests-draft.json`), `--with-dialogue` | Drafts one JSON entry per cached quest page (slug, URL, fetch date, name, type, act, rewards, steps; NPC dialogue only with `--with-dialogue`). A page without a steps table is drafted with zero steps and named on stderr; an index with no quest, a malformed steps table, or no steps table in any page stops with a non-zero exit naming the slug. |
| `coverage` | `--log LOG` (required, read-only), `--cache DIR` | Per cached page, how many dialogue snippets match a logged line of the same NPC exactly, by containment, or by a shared 4-word run. Counts only, never text. |

The cache defaults to `%LOCALAPPDATA%\ExilePact2\quest-tracker\poe2db-cache\`;
both the cache and `--out` are refused inside a git checkout, because raw
pages are never tracked. `--help` on the tool and on each command prints the
same options.

### npm scripts

Run as `npm --prefix apps/quest-tracker run <script>` (`test` also as
`npm --prefix apps/quest-tracker test`).

| Script | What it runs |
|---|---|
| `dev` | The Vite dev server on `http://127.0.0.1:1420`: the browser preview (below). |
| `build` | `svelte-check`, then `vite build` into `apps/quest-tracker/dist/`. |
| `test` | `vitest run` over `src/*.test.ts` in jsdom. |
| `tauri` | The Tauri CLI. |
| `start` | `tauri dev`: the real overlay against the dev server, rebuilt on change. |
| `release` | `tauri build --no-bundle`: `apps/quest-tracker/target/release/quest-tracker.exe`. |

**Browser preview.** Outside Tauri, `npm --prefix apps/quest-tracker run dev`
serves the UI over one of the serialised views in `fixtures/viewmodels/`,
chosen by query: `http://127.0.0.1:1420/?frame=quest-detail&mode=play&scale=2`.
`frame` is a frame id of the brief (default `panel-next-up`), `mode` is
`play`|`edit`, `scale` is `1`|`1.33`|`2`. Commands are logged to the console,
not acted on. The fixtures are reached only in the dev server; the release
bundle carries none of them.

## How detection works

The tracker follows `Client.txt` the way `tail -f` would, but without holding
it open: every 250 ms it checks the file's size and a fingerprint of its
first kilobyte, and only when something changed does it open the file
read-only, read the new bytes and close it again. On start it resumes from
the saved position when the same file is still there (catch-up), and
otherwise starts at the end (live) unless `--from-start`. A truncated,
rewritten or rotated file is read again from its start.

**The current area on start.** A quest's reward or points signal counts
only in the area the quest data names for it, and the area is otherwise
known only from an area line read in this run. So whenever it starts following a log at a
position other than its start (a resume, or a first start at the end), the
tracker first reads back, read-only, at most 8 MiB of the log before that
position for the last area line, and takes it as the current area unless a
session marker follows it (the game restarted, so the area is unknown). The
diagnostic log says what it found: `area from log: <area code>`, or `area
from log: none (…)` with the reason. When no area line is within that
window, or a marker follows it, the area stays unknown, and a quest whose
signal is tied to an area waits for the next zone's area line: a signal
refused before then because the area was unknown is not counted (tick the
quest by hand if it was its `done`), and each is logged as `area unknown:
refused <quest id>`. A signal refused because you are in a different, known area is the
normal case and is not logged.

Each complete line is parsed into an event: an area being generated, an NPC
line from an allowlisted speaker (quest givers and bosses), a system line, a
reward or loss line (`has received`, `has lost`), a passive or weapon-set
points line, a level-up (`is now level`), a party member joining or leaving,
a death, a session start, the client version. Chat and anything not recognised are dropped at the parser.

The quest data (`apps/quest-tracker/data/quests.json`) gives each quest
signal lists: `unlock`, per-step signals, `done`, and `window_closes` (the
point after which the log can no longer show the quest being done, such as
entering the next act's town). Status, first match wins: skipped by you;
done (by you, or a `done` signal fired); unknown (the window closed, it is
not done, and the log cannot tell); in progress (a step fired or was
ticked); available (unlocked); locked. Applying the same line twice changes
nothing, so re-reading a log is safe.

**Characters.** The log has no "logged in as" line, so the active character
is inferred: a name in a `has received` or `has lost` line is yours; a name
seen joining or leaving the area is a party member for the rest of that
session; a level-up of yours makes it active and records class and level; a
level-up of an unknown name makes it active but *unconfirmed* until a reward
line or you confirm it in the character picker. Events before any active
character are held for the first one that becomes active in that session,
and dropped and counted in the status line if none does. A character you pin
in the picker gets every event until you unpin it. Measurements and reasons:
[`quest-tracker-research.md`](quest-tracker-research.md) § "Character
detection".

**What needs a manual tick.** Each quest carries a `detection` value, and
the row shows the manual-tick marker for the last two:

- `auto` (30 quests in this version): its `done` signal was measured in a
  real log; the tracker finishes it on its own.
- `partial` (44): some unlock or step signal was measured, `done` was not;
  the tracker shows progress, and once the window closes without a `done`
  the quest goes to *unknown* until you tick it.
- `manual` (13): nothing measured; tick it yourself in edit mode (open the
  quest, tick steps or mark it done).

Which signals were measured, and which are static readings of poe2db, is
argued per quest in [`quest-tracker-research.md`](quest-tracker-research.md)
§ "Per-quest detection".

## Privacy

What is read:

- `Client.txt`, read-only, in short opens with full sharing, never held,
  locked, moved or truncated. Nothing else of the game's is opened.

What is stored, and where:

- The store folder is `QUEST_TRACKER_DIR` when that environment variable is
  set and non-empty, else `%LOCALAPPDATA%\ExilePact2\quest-tracker\`. It is
  refused (progress is then not saved, and the status line says so) when it
  is inside the game's install or a `Grinding Gear Games` folder, or inside
  a git checkout.
- `settings.json`; `position.json` (the log's path, fingerprint and read
  offset); `characters.json` (your character names with class, level, own
  or unconfirmed, pinned); `characters/<name>.json` (one character's quest
  evidence, overrides and ticks); `overlay.log`, the diagnostic log, rotated
  at 1 MB into `overlay.log.1`. JSON files are written atomically (a
  temporary file, synced, renamed over the old one); one that cannot be read
  back is renamed `<file>.corrupt-<timestamp>` and the status line says so.
- Only your own and unconfirmed-active character names are ever written.
  Names seen in join or leave lines (party members) live in memory for one
  session and are never written to disk or to the diagnostic log.
- Chat is ignored: chat lines are dropped by the parser, and no chat text,
  other player's name or account name reaches the store, the diagnostic log,
  the replay CLI's output or the UI.

Network:

- The app makes no network request. Its content security policy allows only
  the bundled page and Tauri's own IPC (`default-src 'self'`, no remote
  script, style, font, image or connection source), its capabilities grant
  only core event and window permissions, and its Rust sources use no HTTP
  or socket code. Fonts and icons are bundled. The poe2db link opens in your
  default browser, through the opener plugin, from Rust.
- The global shortcut is a system hotkey registration: no keyboard hook, and
  nothing is sent to the game.
- The only network code in this feature is the developer-only poe2db tool,
  which you run yourself.

## Quest data

`apps/quest-tracker/data/quests.json` is compiled into both executables
(`--data` overrides it). This version: schema 1, checked against client
`4.5.3`, poe2db fetched 2026-10-25, 87 campaign quests over Acts 1 to 6, 98
areas, 37 allowlisted speakers. The validator in `qt-engine` fails, naming
the quest and field, on a duplicate id, a value outside an enum, a bad URL
or date, an unknown signal kind or field, or a signal naming a quest,
speaker, system line or area the file does not define.

Each quest carries its poe2db `source_url` and `fetched` date. Quest names,
step text, NPC names and reward text are game text by Grinding Gear Games,
quoted as short strings for matching and display under the owner's decision
of 2026-10-24; worth reasons are written in our own words. Raw poe2db pages
are never tracked. The file's `source_note` says the same.

The file is curated by hand from a draft, not generated. To rebuild it after
a patch:

1. `py -3 -m tools.quest_tracker.poe2db fetch` (about 70 pages at 3 s, a few
   minutes), then `py -3 -m tools.quest_tracker.poe2db extract` into a draft
   outside the repository.
2. Run `quest-tracker-replay scan <your Client.txt> --speakers <the draft's
   NPC names and the bosses its step titles name>` over a log that reached
   the changed content.
3. Update `areas` from the scan (code and display name, evidence
   `measured`), adding poe2db area names the log lacks as `static`.
4. Per quest: `done` from its reward line and area where the log shows one
   (`measured`); `unlock` from the first area of its zone or the quest
   before it; steps from area entries and NPC or boss speakers in an area;
   `window_closes` from entering the next act's town for `partial` and
   `manual` quests; NPC `line` signals only for lines actually seen in the
   log. Set `detection` and `evidence` to what was measured.
5. Bump `game_version` and `poe2db_fetched`, run the engine's tests
   (`cargo test --manifest-path apps/quest-tracker/Cargo.toml -p qt-engine
   --locked`, which validates the shipped file), and `quest-tracker-replay
   state` over the log to see that quests still advance.

The data contract (every field, signal kind and reward category) is the
validator in `apps/quest-tracker/crates/qt-engine/`.

## Working on the app

Layout (`apps/quest-tracker/` is both the Cargo workspace and the npm
project):

| Path | What |
|---|---|
| `crates/qt-logline/` | Parser: one log line to one typed event. |
| `crates/qt-tailer/` | Log follower thread (poll, fingerprint, never holds the file). |
| `crates/qt-store/` | App-data folder, atomic JSON, quarantine, diagnostic log. |
| `crates/qt-engine/` | Quest data contract and validator, the per-character state machine. |
| `crates/qt-viewmodel/` | `View` and `Toast`: what the panel shows, built in Rust. |
| `crates/qt-replay/` | The `quest-tracker-replay` CLI (`src/main.rs`). |
| `crates/qt-app/` | Shell logic without Tauri: state lock, worker thread, commands, modes, debounced saves, command line, log discovery. Tested with a fake window, emitter and clock. |
| `src-tauri/` | Tauri glue: the `overlay` window, `WS_EX_NOACTIVATE`, click-through, the shortcut, single instance, the command wrappers. |
| `src/` | The Svelte 5 + TypeScript UI and its `*.test.ts`. |
| `theme/` | The harvested design: `theme.json`, `tokens.css`, `icons/<id>.svg`, `fonts/`. |
| `data/quests.json` | The shipped quest data. |
| `fixtures/` | Synthetic logs, sessions, minimal quest data and serialised views shared by the Rust and UI tests; every name in them is invented. |

Tests:

```sh
cargo test --manifest-path apps/quest-tracker/Cargo.toml --locked -p qt-engine   # one crate
npm --prefix apps/quest-tracker run build                                        # needed before the workspace build
cargo test --manifest-path apps/quest-tracker/Cargo.toml --locked --workspace    # every crate
npm --prefix apps/quest-tracker test                                             # the UI, in jsdom
py -3 -m unittest tests.test_quest_tracker_poe2db                                # the poe2db tool, no network
```

`src-tauri` embeds the built UI at compile time, so anything that builds the
`quest-tracker` package (the workspace test run included) needs
`npm --prefix apps/quest-tracker run build` first. No test opens a window or
reads a real `Client.txt`.

**IPC.** Rust decides and the UI renders. After every change of what the
panel shows, the shell pushes a freshly built view; the UI never re-derives
state and never polls. Events, Rust to the `overlay` window:

- `qt:view` with a `View` (frame, mode, view kind, status line, characters,
  acts, next-up rows, quest detail, settings).
- `qt:toast` with a `Toast` (shown once, about 3 s, at most three stacked).

Commands, UI to Rust: `get_view`, `set_mode`, `set_view_kind`,
`set_collapsed`, `toggle_act`, `open_detail`, `open_frame`, `quest_action`
(`mark_done`|`unmark_done`|`skip`|`unskip`|`reset`), `tick_step`,
`select_character`, `pin_character`, `confirm_character`, `update_settings`,
`open_source`. Field names in events are snake_case exactly as `serde`
writes them; command argument keys are camelCase on the JavaScript side
(`questId`, `stepId`), Tauri's default. `src/ipc.ts` mirrors the Rust types,
and `src/ipc.test.ts` holds every file in `fixtures/viewmodels/` against
them, so a field added, dropped or renamed on one side fails a test.

The view fixtures are written by `qt-viewmodel`'s tests; after a deliberate
change to the types, regenerate and review the diff:

```sh
QT_UPDATE_FIXTURES=1 cargo test --manifest-path apps/quest-tracker/Cargo.toml -p qt-viewmodel --locked
```

**Adding a command:**

1. Add the method to `App` in `crates/qt-app/src/app.rs`. If it changes
   state, it persists (debounced) and emits `qt:view` before it returns. Test
   it there, against the fakes.
2. Add a wrapper in `src-tauri/src/lib.rs`: `#[tauri::command(async)]` if it
   can touch disk (none calls `block_on`), a plain `#[tauri::command]` only
   for a memory read; add it to `tauri::generate_handler![...]`.
3. Add its arguments to `CommandArgs` in `src/bridge.ts` (camelCase keys)
   and call it through the bridge; the browser preview logs it.
4. Grant no new capability for it: app commands need none, and
   `src-tauri/capabilities/default.json` stays at core event and window
   permissions.
5. Update the command list in this guide.

**Dependencies** are pinned exactly in `[workspace.dependencies]` and
`package.json`; build with `--locked` and `npm ci`. A new crate or npm
package is a decision, not a side effect of a change.

**The look.** The design is the owner's, made in Figma from
[`ui-design-brief.md`](ui-design-brief.md) and harvested once into `theme/`
and [`design/`](design/). UI work reads those files, never Figma, and uses
only the brief's icon ids, frame ids and token keys, plus the extras the
harvest lists in [`design/harvest.md`](design/harvest.md). The picture to
match is `design/frames/` and `design/component-spec.md`;
`design/reference/*.tsx` is reference only and never imported.

The UI's CSS comes in three layers, loaded by `src/main.ts` in this order:

- `src/defaults.css`: a plain value for every custom property the UI
  reads, contract keys and the harvest's extras alike. Without
  `theme/tokens.css`, or with a key missing from it, the UI renders from
  these; a missing icon SVG falls back to a text glyph (`src/theme.ts`).
- `theme/tokens.css`: the harvested values and the `@font-face` rules for
  the bundled Alegreya Sans faces. Generated from `theme/theme.json`; never
  edited by hand.
- `src/styled.css`: the shared rules every component uses: the cut-corner
  shape (`.cut`: a `clip-path` plus an `::after` ring for the border), the
  buttons, toggles, slider, dropdowns, keycaps, group labels and countdown
  bars. Each component keeps its own layout in its `<style>` block.

Cut corners clip an element's own `box-shadow`, so the panel, the badge and
the toast stack take their shadow from `filter: var(--drop-panel)` or
`var(--drop-toast)` on a wrapper. `src/theme.ts` derives both from the
harvested `shadow.panel` and `shadow.toast` at start-up
(`applyDerivedTokens`). Motion (the toast's entrance, the countdown bars,
control transitions) stops under `prefers-reduced-motion`.
`src/styled.test.ts` checks that every contract token and icon id is
harvested, that every custom property the UI reads has a plain default and
comes from the harvest (no token invented in code), and that fonts and
assets load only from the bundle.

To preview a frame in a browser, run `npm --prefix apps/quest-tracker run
dev` and open `http://127.0.0.1:1420/?frame=<frame id>&mode=play|edit`
(`&scale=2` for the 2× zoom); see `src/preview.ts`.

## Known limitations

- **Party play.** Only your own characters are tracked. While in a party, a
  party member's level-up is excluded once they have been seen joining or
  leaving the area; a level-up seen before that can make them the active
  character as *unconfirmed* until their join or leave line arrives, which
  hands what was attributed to them back to the hold.
- **Character detection.** The log never says which character you logged
  in as. Until your character earns a reward or levels up in a session,
  events are held; if none does by the end of the session they are dropped
  and counted in the status line. Switching characters within a session is
  noticed only at the new character's first reward or level-up. Pin the
  character in the picker when this matters.
- **Patch changes.** The parser reads English templates and the quest data
  keys on area codes; a patch that changes either makes quests stop
  advancing rather than crash. The shipped data mixes poe2db's current
  patch with signals measured on client 4.5.3, and Act 5 and the Siege of
  Oriath have no measured signals at all. See
  [`quest-tracker-research.md`](quest-tracker-research.md) § "Patch
  sensitivity".
- **A deleted log.** Deleting or rotating `Client.txt` while the tracker runs
  is safe (it never holds the file); it waits for the game to write it again
  and reads the new file from its start. Progress already saved stays, but
  anything the game logged into a file that was deleted before the tracker
  read it is not seen and needs a manual tick.
- **No blur behind the panel.** CSS `backdrop-filter` sees only the page's
  own pixels, not the game behind the window, so a frosted panel would blur
  nothing; window-level acrylic over a topmost window is not established.
  The panel uses transparency, gradients and shadows instead.
- **Hover only in edit mode.** Play mode passes the mouse to the game.
- **Parts of the comps the view does not carry yet.** The design shows
  four rebindable hotkeys (F8 edit mode, F7 collapse, F6 switch view, F1
  icon key), an icon-key panel, Browse… and Retry on the missing-log
  state, a "New character" button, a log-idle header ("No log update · 5
  min") with stale rows, and a footer counting hidden quests. The shell has
  one hotkey (`Ctrl+Alt+Q` by default, toggling edit mode) and no command
  or `View` field for the rest, so the styled UI shows the one hotkey from
  the settings, "Open settings" instead of Browse…/Retry, the character
  picker instead of "New character", and the status line for log trouble.
- **Exclusive fullscreen** cannot show the overlay (see "Platforms and
  prerequisites").
- **Steam install path** is unverified.
- **Not covered:** endgame and Atlas quests, non-English clients, an
  installer, auto-update.

## Maintenance triggers

Revisit this feature when:

- **A game patch lands**: run `quest-tracker-replay scan` and `state` over a
  fresh log; update the parser fixtures if a log template changed, and the
  quest data if quests, rewards or area codes moved ("Quest data" above).
- **poe2db changes its page structure**: `extract` fails loudly naming the
  slug; fix the extractor and its synthetic fixture under
  `tests/fixtures/quest_tracker/poe2db/`.
- **A Rust type behind `View` or `Toast` changes**: regenerate the view
  fixtures, update `src/ipc.ts`, and update the brief if a frame, icon or
  status changes.
- **The brief's export contract changes** (icon ids, frame ids, token keys):
  update `theme/`, `qt-viewmodel` and the UI together.
- **A dependency is bumped**: keep `tauri`, its plugins, `@tauri-apps/api`
  and `@tauri-apps/cli` on the same minor line, refresh both lock files, and
  re-check the CSP and capabilities.
- **Someone verifies the Steam path**: update `crates/qt-app/src/discover.rs`
  and this guide.
- **A new display mode or Windows version** behaves differently over the
  game: record it under "Known limitations".

## Release notes

### 0.1.0 (first version)

- The overlay, `quest-tracker.exe`: a transparent, always-on-top Tauri
  window with play (click-through) and edit modes, toggled by
  `Ctrl+Alt+Q`; *Next up* and *All* views; quest detail with steps,
  rewards, worth reason and the poe2db link; character picker with confirm
  and pin; settings for opacity, dock, scale, filters, log path and hotkey;
  toasts for quests becoming available, steps and quests done.
- Play mode never takes focus from the game; edit mode takes keyboard
  focus, so the Settings fields accept typing. A shortcut that cannot be
  registered at launch leaves the overlay in edit mode, with the reason in
  the status line, instead of stuck in click-through play mode.
- The owner's design: Alegreya Sans bundled (no web font), 45° cut corners
  on the panel, buttons, toasts and badge, soft shadows inside the
  window's transparent margin, and the background-only opacity setting.
- Play mode: the character, class and level, and the view, with no
  control. Edit mode: the accent border, a teal-tinted header with an EDIT
  chip, the character name that opens the picker, a Next up / All switch,
  collapse and settings buttons, a visible hover outline on rows, and a
  footer line back to play mode.
- Quest rows on one line (status mark, name, manual-tick marker, short
  reward labels such as "Spirit" or "Passive · Cold res" with the game's
  full reward text on hover, worth star), the next step under an
  in-progress quest on the active band with a blue bar, finished rows
  compact and muted, skipped ones struck through. Each act's header shows
  how many of its quests are done; only an expanded act's header also
  shows how many are available, and a collapsed one keeps the mark alone.
  Next up
  groups its rows under "Worth it", "Optional" and "Not worth it".
- Quest detail with steps (empty box, in progress, done) whose leading mark
  is, in edit mode, the step's one tick-by-hand toggle; reward chips, the
  worth reason, the poe2db link, and Mark done / Skip / Reset buttons;
  Reset's confirm shows a 3 s countdown. Settings as label-and-control
  rows with toggles, a slider and dropdowns. Empty states with a glyph,
  the fix in edit mode, and the hotkey on a keycap in play mode. The
  collapsed badge shows the worth-it count in gold, or a warning when the
  log is in trouble. Toasts carry a coloured bar and icon per kind; the
  Undo toast drains a 5 s bar.
- Scale Auto sizes the panel by the monitor: 1x, 1.33x from 1440 px of
  height, 2x from 2160, after Windows' own DPI scale. It is worked out at
  start after the window is placed, when the setting changes, and when the
  window moves to another monitor or its DPI scale changes (Command
  reference).
- Motion is limited to the toast's entrance, countdown bars and control
  transitions, and stops under `prefers-reduced-motion`.
- Progress per character from `Client.txt` alone, resumed across restarts
  from the saved log position, stored under `%LOCALAPPDATA%` (or
  `QUEST_TRACKER_DIR`); on start the current area is read back from the
  log before that position (How detection works).
- Quest data for 87 campaign quests, Acts 1 to 6, from poe2db.tw (fetched
  2026-10-25), with signals measured against a real log of client 4.5.3:
  30 detected automatically, 44 partly, 13 by manual tick.
- `quest-tracker-replay` (`events`, `state`, `import`, `scan`) and the
  developer-only `py -3 -m tools.quest_tracker.poe2db` (`fetch`, `extract`,
  `coverage`).
