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
- **`Ctrl+Alt+W` copies it**, from anywhere, the game included: the window
  can be minimised. The Copy button does the same. On success nothing is
  heard; a refusal (nothing selected, over 250 characters, a mod that
  cannot be told apart) beeps and leaves the clipboard as it was. You paste
  the string into the stash or a vendor's search box with `Ctrl+V`.
- **The shortcut can be changed** in the window. Windows lets one program
  hold a key combination; when another program holds `Ctrl+Alt+W`, the app
  says so and keeps running, and the old shortcut stays when a new one is
  refused.

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

- `settings.json`: `schema` (1), `hotkey` (default `Ctrl+Alt+W`) and
  `selection` (the rules above). Written a moment after each change, through
  a temporary file renamed over the old one, and when the app quits. Saved
  mods the data no longer has are dropped at launch, and the status line
  names them.
- `diag.log`: one line per event, with the UTC time: `start: ...`,
  `shortcut <hk> registered`, `shortcut <hk> could not be registered:
  <error>`, `copy from the shortcut` or `copy from the Copy button`, then
  `copied N characters: <regex>` or `copy refused: <reason>` and, after a
  refusal, `refusal beep: <sound> ok` or `refusal beep: <sound> failed
  (<error>)` for each sound tried; `stop`. Rotated at 1 MB into
  `diag.log.1`.

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

- **The app touches nothing but its window and the clipboard.** It reads no
  game memory, hooks nothing, sends no input or window message to the game,
  and reads none of the game's files. Its only effects outside its window are
  the text it puts on the clipboard, when you press the shortcut or the Copy
  button, and a system beep when it refuses.
- **No network.** The app makes no request. (WebView2, Microsoft's runtime
  that draws the window, may connect on its own, as for the quest tracker.)
  The data tool asks poe2db.tw for one page, only when a developer runs it.
- **What it writes**: `settings.json` and `diag.log` in its folder
  (Command reference). The log holds the shortcut, the copied search strings
  and refusals; nothing about your account or character.

## Working on the app

| Path | What it holds |
|---|---|
| `apps/waystone-roller/crates/wr-regex/` | The engine: the data loader, token choice, the search builder, and a model of the search box used only by the tests. |
| `apps/waystone-roller/crates/wr-app/` | The app's logic without Tauri: the store, the selection, the view, the commands, the shortcut and copy path. |
| `apps/waystone-roller/src-tauri/` | The Tauri glue: the global shortcut, single instance, the Win32 clipboard writer and beep, the commands. |
| `apps/waystone-roller/src/` | The Svelte page and `ipc.ts`, the view's types. |
| `apps/waystone-roller/data/` | The mod data file. |
| `apps/waystone-roller/fixtures/` | The invented mod set and item texts for `wr-regex`, and the view fixtures shared by the page's tests and `wr-app`'s. |
| `tools/waystone_roller/` | The poe2db data tool. |

Tests (none opens a window or reaches the network):

```sh
npm --prefix apps/waystone-roller run build                                          # needed before the workspace build
cargo test --manifest-path apps/waystone-roller/Cargo.toml --workspace --locked       # every crate
cargo test --manifest-path apps/waystone-roller/Cargo.toml -p wr-regex --locked       # the engine alone, no build of the page needed
npm --prefix apps/waystone-roller test                                                # the page, in jsdom
py -3 -m unittest tests.test_waystone_roller_poe2db tests.test_waystone_roller_guard  # the data tool and the guard
```

- `wr-regex`'s `_baseline` tests pin the model of the game's search on
  hand-written strings; its `_target` tests run generated strings through
  the model over item texts and check that exactly the intended items light.
- `wr-app`'s tests drive the app through a recording fake shell and a
  temporary store; `shipped_data_every_family_token_matches_only_its_own_item`
  checks the shipped data file the same way.
- `tests.test_waystone_roller_guard` reads the files only: no input, no
  network and no game access in any source or manifest, the pinned CSP and
  capability, one version everywhere, and every package already in the
  quest tracker's locks.

**Refreshing the mod data**: run `fetch`, then `extract`, then the tests
above. A new or renamed modifier changes the file; check its diff, and the
shipped-data test says if a token no longer tells a mod apart.

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
- **The refusal beep is not yet confirmed audible in the game.** In the
  2026-10-28 session the old beep (Windows' "Critical Stop" sound) was not
  heard on five refused presses, cause not established. 0.1.0 now plays a
  tone that reads no entry of the sound scheme, falling back to the simple
  beep only if that call fails, and `diag.log` records `refusal beep:
  <sound> ok` or `failed (<error>)` for each one tried. "ok" says Windows
  ran the call, not that it was heard; a short live check is still to
  confirm it.
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

## Release notes

### 0.1.0 (first version)

- The waystone roller: pick waystone and desecrated waystone modifiers to
  want (any or all) or avoid, a tier range, rarities and corruption, and get
  one search string for the stash or vendor search box, at most 250
  characters or refused whole.
- `Ctrl+Alt+W` (changeable) or the Copy button puts it on the clipboard;
  a refusal beeps and leaves the clipboard alone.
- Mod data from poe2db.tw, fetched 2026-10-28.
- Built from source only; no installer and no release download.
- Measured in the built app on 2026-10-28, with Path of Exile 2 in front:
  each `Ctrl+Alt+W` press replaced the clipboard with the search the
  window showed (want any, avoid, tier with rarity, corrupted), and
  pasting it lit the waystones the rules named; over 250 characters
  nothing was copied and the window showed the refusal; the app sent
  nothing to the game. The refusal beep was not heard in that session
  (Known limitations).
- The refusal beep is now a tone that does not depend on the Windows sound
  scheme (before: the "Critical Stop" sound), and `diag.log` names the
  sound and whether the call succeeded, and whether a copy came from the
  shortcut or the Copy button.
