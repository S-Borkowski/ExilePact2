# ExilePact2

Tools and mods for Path of Exile 2, three apps: the quest tracker, the
waystone roller and the crafting helper.

The [proposed shared UI design system](docs/ui-design-system.md) describes a
future visual direction; current app behavior and export contracts remain
authoritative.

## Quest tracker

`Quest Tracker.exe` (`quest-tracker.exe` as a release download) is a small
always-on-top overlay that lists the Path of
Exile 2 campaign quests (Acts 1 to 6, 87 quests in 0.2.0). It says which ones
are worth doing and why, and keeps each character's progress. It learns that
progress from the game's `Client.txt` log. One setting is opt-in: *Read the
screen* also reads three regions of the game's window with Windows' own OCR.
Hiding under game panels is always on: in play mode the overlay hides while a
game panel such as the inventory is open under it, told from the colour of a
few small patches of the game's window. In edit mode the window resizes by its
edges, and the scale setting zooms its contents. It sends no input to the game and never reads the
game's memory. It makes no network request unless you ask it to check for
updates (the tray icon's *Check for updates*, or *Automatically check for
updates* in its *Settings*, off by default) or to install one; a check only
asks GitHub whether a newer release is out, and downloads nothing. When one
is, the Settings window's install button downloads it, checks its signature
against the key built into the app, replaces the running exe and restarts
([`docs/app-updates.md`](docs/app-updates.md)). (WebView2, Microsoft's
runtime that draws its windows, connects on its own: see the guide's Known
limitations.)

- Download: the latest Quest Tracker release on the
  [releases page](https://github.com/S-Borkowski/ExilePact2/releases). Each
  release has `quest-tracker.exe`, `quest-tracker-replay.exe`,
  `SHA256SUMS.txt`, the exe's signature `quest-tracker.exe.minisig` and the
  version's changelog. After that, a newer version installs from the app.
- Guide (prerequisites, commands, how detection works, privacy, known
  limitations): [`docs/quest-tracker/README.md`](docs/quest-tracker/README.md)

### Requirements

- Windows 10 or 11, 64-bit, with the Microsoft Edge WebView2 runtime.
- The game in windowed or borderless windowed fullscreen. Exclusive
  fullscreen cannot work: the game owns the display output, and no window is
  drawn over it.
- A `Client.txt` the game is writing. The standalone client's is found on
  its own; elsewhere, pass `--log` or set the log path in Settings.

### Build from source

You need the Rust toolchain for the MSVC target, the Visual Studio C++ build
tools, and Node.js with npm. Every crate and npm package is pinned; build
with `--locked` and `npm ci`.

```sh
npm --prefix apps/quest-tracker ci            # only when apps/quest-tracker/node_modules is missing
npm --prefix apps/quest-tracker run release   # the overlay, then the replay CLI
# -> apps/quest-tracker/target/release/Quest Tracker.exe
# -> apps/quest-tracker/target/release/quest-tracker-replay.exe
```

Run `Quest Tracker.exe` (a release download is `quest-tracker.exe`, the
same program). `Ctrl+Alt+Q` switches between play mode, which is
click-through, and edit mode. To quit, use the tray icon's *Quit Quest
Tracker*.

## Waystone roller

`waystone-roller.exe` helps you roll (craft) waystones. In an ordinary
window you pick, from the waystone modifiers listed on poe2db.tw, the mods
to get (any or all of them) and the mods to avoid, a tier range, the
rarities and corrupted or not. It builds one search string from those rules
and keeps it within the game's 250-character limit; when it cannot, it says
so and copies nothing. `Ctrl+Alt+W` (changeable) puts the string on the
clipboard and, when Path of Exile 2 is in front, pastes it into the stash
or vendor search box you have focused with `Ctrl+A` then `Ctrl+V`,
replacing its text (a checkbox in the window turns the paste off; the Copy
button only copies). Those two key presses are all it sends the game: no
mouse input, no click, no focus change, no memory read. Its *Settings* window,
like the quest tracker's, checks GitHub for a newer version only when you
ask (*Check now*, or *Automatically check for updates*, off by default) and
installs it in the app when you press its button, after checking its
signature ([`docs/app-updates.md`](docs/app-updates.md)); otherwise it
makes no network request.

- Guide (how the search is built, the mod data, privacy, known
  limitations): [`docs/waystone-roller/README.md`](docs/waystone-roller/README.md)
- Download: the latest Waystone Roller release on the
  [releases page](https://github.com/S-Borkowski/ExilePact2/releases?q=waystone-roller).
  Each release has `waystone-roller.exe`, `SHA256SUMS.txt` and the exe's
  signature `waystone-roller.exe.minisig`; there is no installer, and a
  newer version installs from the app.
- Requirements: Windows 10 or 11, 64-bit, with the WebView2 runtime; an
  English game client.

To build it from source instead, with the same toolchain as the quest
tracker:

```sh
npm --prefix apps/waystone-roller ci            # only when apps/waystone-roller/node_modules is missing
npm --prefix apps/waystone-roller run release
# -> apps/waystone-roller/target/release/waystone-roller.exe
```

## Crafting helper

`Crafting Helper.exe` (`crafting-helper.exe` before 0.2.0) helps you craft items. You copy an item in the game
with the game's own `Ctrl+Alt+C`, then press the app's hotkey
(`Ctrl+Alt+X`, changeable, never `Ctrl+Alt+C`) or `Ctrl+V` in its window,
and the window shows the item and its modifiers, lets you pick target
modifiers and previews the route a crafting blueprint takes to them, with
each step's currency, omens and rough odds. *Start crafting* guides you:
take the step in the game and hover the item, and the app reads the item's
tooltip from the game's window with Windows' own OCR and says whether the
step hit or missed and what is next; when it cannot read the item, you
grab it again. While a craft is guided and the game is in front, a
click-through overlay over the game shows the current step, where a hit
and a miss go, the last outcome and what the screen watch is doing, at the
place chosen in Settings. It
reads the clipboard once per grab and never watches it, keeps its copies
of the game's window in memory (never saved, logged or sent), uses no
keyboard hook and sends the game nothing: no keys, no mouse input, no
memory read. Its *Settings* window holds the hotkey, the overlay's
position, a switch for reading the screen, a saved list of blocked
currencies, the trade League (with a refresh icon that asks the trade
site for its league list, only when clicked) and, like the siblings', an
update check that runs only when you ask (*Check now*, or *Automatically
check for updates*, off by default) and an install of a signed newer
version on a button press ([`docs/app-updates.md`](docs/app-updates.md));
otherwise it makes no network request. *Find a base on trade* builds an
official trade-site search for a base with the modifiers you pick and
opens it in your browser; the app sends that search nothing itself.

- Guide (privacy, commands, known limitations, release notes):
  [`docs/crafting-helper/README.md`](docs/crafting-helper/README.md)
- Download: the latest Crafting Helper release on the
  [releases page](https://github.com/S-Borkowski/ExilePact2/releases?q=crafting-helper).
  Each release has `Crafting.Helper.exe` (GitHub's name for `Crafting
  Helper.exe`: it stores a space as a dot), `SHA256SUMS.txt` and the
  exe's signature `Crafting.Helper.exe.minisig`, and the same exe under
  its 0.1.x name, `crafting-helper.exe` with `crafting-helper.exe.minisig`,
  for copies from 0.1.x to update; there is no installer, and a newer
  version installs from the app (a 0.1.x copy keeps its old file name).
- Requirements: Windows 10 or 11, 64-bit, with the WebView2 runtime; an
  English game client; for reading the item from the screen, Windows'
  English OCR and the game windowed or borderless.

To build it from source, with the same toolchain:

```sh
npm --prefix apps/crafting-helper ci            # only when apps/crafting-helper/node_modules is missing
npm --prefix apps/crafting-helper run release
# -> apps/crafting-helper/target/release/Crafting Helper.exe
```

## Repository layout

| Path | What it holds |
|---|---|
| `apps/quest-tracker/` | The overlay: Rust crates for the log tailer, parser, quest engine, view-model and store, and the Svelte UI. |
| `tools/quest_tracker/` | Developer-only tools: the release cutter, the poe2db quest data harvester and the outside instruments used for research. |
| `apps/waystone-roller/` | The waystone roller: the search-string engine and app logic as Rust crates, the Tauri glue, the Svelte UI and the mod data. |
| `tools/waystone_roller/` | Developer-only: the release cutter and the poe2db Waystones data tool. |
| `apps/crafting-helper/` | The crafting helper: the app logic, the item text parser, the crafting engine and the screen watch as Rust crates (the screen watch takes the quest tracker's `qt-screen` and `qt-capture` by path), the Tauri glue, the Svelte UI, the blueprints and the modifier data. |
| `tools/crafting_helper/` | Developer-only: the poe2db modifier data extractor, the trade stat mapper (`trade_stats.py`, which writes `data/trade-stats.json`), the slot model (`slot_model.py`, a stdlib model of how crafting routes use an item's free slots, checked against the engine's case file), and the release cutter (`release.py`). |
| `apps/shared/` | The update code the three apps share, a Cargo workspace of its own: `ep-net`, the only network code, and `ep-update`, the check, the install and the signature check ([`docs/app-updates.md`](docs/app-updates.md)). |
| `.github/workflows/` | The three release builds (the quest tracker's, the waystone roller's and the crafting helper's), and the opt-in AI review. |
| `docs/quest-tracker/` | The quest tracker's guide, research note and design. |
| `docs/waystone-roller/` | The waystone roller's guide, research note and UI design. |
| `docs/crafting-helper/` | The crafting helper's guide, UI design, research note and crafting mechanics. |
| `docs/RUNTIME_DATA_MODELS.md` | Facts about the game that every mod can use, each labelled measured or read. |
| `tools/`, `tests/` | The tests for all of the above, and the agent tooling's own tools. |
| `AGENTS.md`, `.claude/`, `.codex/`, `docs/agents/`, `docs/tools/` | The rules and tooling for coding agents. |

## Tests

```sh
py -3 -m unittest discover -s tests                                              # every Python test
npm --prefix apps/quest-tracker run build                                        # needed before the workspace build
cargo test --manifest-path apps/quest-tracker/Cargo.toml --locked --workspace    # every crate
npm --prefix apps/quest-tracker test                                             # the UI, in jsdom
npm --prefix apps/waystone-roller run build                                      # the same three steps for the waystone roller
cargo test --manifest-path apps/waystone-roller/Cargo.toml --locked --workspace
npm --prefix apps/waystone-roller test
npm --prefix apps/crafting-helper run build                                      # the same three steps for the crafting helper
cargo test --manifest-path apps/crafting-helper/Cargo.toml --locked --workspace
npm --prefix apps/crafting-helper test
cargo test --manifest-path apps/shared/Cargo.toml --locked --workspace           # the shared update crates
```

No test opens a window, reads a real `Client.txt` or reads the real
clipboard.

## Releases

Each app is released on its own, from the tip of `main`, by tagging it:

```sh
py -3 -m tools.quest_tracker.release cut --push      # the quest tracker
py -3 -m tools.waystone_roller.release cut --push    # the waystone roller
py -3 -m tools.crafting_helper.release cut --push    # the crafting helper
```

The tag `quest-tracker-v<version>` starts
[`.github/workflows/quest-tracker-release.yml`](.github/workflows/quest-tracker-release.yml),
which builds both of the quest tracker's executables;
`waystone-roller-v<version>` starts
[`.github/workflows/waystone-roller-release.yml`](.github/workflows/waystone-roller-release.yml),
which builds `waystone-roller.exe`; `crafting-helper-v<version>` starts
[`.github/workflows/crafting-helper-release.yml`](.github/workflows/crafting-helper-release.yml),
which builds `Crafting Helper.exe` and attaches it as
`Crafting.Helper.exe` and, for 0.1.x copies, as `crafting-helper.exe`. Each
builds on a Windows runner, signs
the app's exe with the release key (the repository secret
`TAURI_SIGNING_PRIVATE_KEY`; without it the run fails and no release is
made; the crafting helper's under each of its two names), and attaches its
files, `SHA256SUMS.txt` and `<exe>.minisig` to a
**draft** release, with the version's changelog from the app's guide as its
body. [`docs/app-updates.md`](docs/app-updates.md) has the release contract
the apps' in-app update relies on and the key's set-up and rotation. The owner publishes
the draft and sets it as the latest release by hand. Each guide's "Working
on the app" › "Cutting a release" has the whole procedure.

## Working on this repository

The project is developed with Claude Code and Codex. [`AGENTS.md`](AGENTS.md)
holds the rules for both. Some of them are enforced by hooks, and the
`/workorder` pipeline runs a change through plan, implement and verify;
[`.claude/README.md`](.claude/README.md) describes the tooling.

- **One branch and one pull request per feature.** Link the issue the work
  belongs to.
- **Every pull request gets the `ai-review` label.** Its comments are fixed
  before merging.
- **Decompiled game code never reaches a tracked file.** Write down what was
  learned, in your own words, never the decompiled text itself (`AGENTS.md`,
  "Legal").
- **Build outputs stay in each checkout, so reclaim them.** Every worktree
  builds its own `apps/<app>/target/`. `py -3 tools/disk_hygiene.py report`
  shows what each checkout costs, and its `worktrees` and `trim` commands
  remove merged worktrees and oversized targets (dry runs unless `--apply`);
  see [docs/tools/disk-hygiene.md](docs/tools/disk-hygiene.md).

## Path of Exile 2 is an online game

Modifying the client or attaching a debugger can break Grinding Gear Games'
terms of service and get an account banned. The quest tracker reads only the
log and, if you turn the settings on, the screen. Research against a client
you can afford to lose. Nothing here may give one player an advantage over
another.

This project is not affiliated with or endorsed by Grinding Gear Games.

## License

Free for personal, educational and non-commercial use; commercial use needs
written permission. See [`LICENSE.md`](LICENSE.md).
