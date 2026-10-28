# ExilePact2

Tools and mods for Path of Exile 2: the quest tracker and the waystone
roller.

## Quest tracker

`quest-tracker.exe` is a small always-on-top overlay that lists the Path of
Exile 2 campaign quests (Acts 1 to 6, 87 quests in 0.1.0). It says which ones
are worth doing and why, and keeps each character's progress. It learns that
progress from the game's `Client.txt` log. Two settings are opt-in: *Read the
screen* also reads three regions of the game's window with Windows' own OCR,
and *Hide under game panels* hides the overlay while a game panel such as the
inventory is open under it. It sends no input to the game and never reads the
game's memory. It makes no network request unless you ask it to check for
updates (the tray icon's *Check for updates*, or *Automatically check for
updates* in its *Settings*, off by default); a check only asks GitHub whether
a newer release is out, and downloads nothing. (WebView2, Microsoft's runtime
that draws its windows, connects on its own: see the guide's Known
limitations.)

- Download: the latest Quest Tracker release on the
  [releases page](https://github.com/S-Borkowski/ExilePact2/releases). Each
  release has `quest-tracker.exe`, `quest-tracker-replay.exe` and the
  version's changelog.
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
# -> apps/quest-tracker/target/release/quest-tracker.exe
# -> apps/quest-tracker/target/release/quest-tracker-replay.exe
```

Run `quest-tracker.exe`. `Ctrl+Alt+Q` switches between play mode, which is
click-through, and edit mode. To quit, use the tray icon's *Quit Quest
Tracker*.

## Waystone roller

`waystone-roller.exe` helps you roll (craft) waystones. In an ordinary
window you pick, from the waystone modifiers listed on poe2db.tw, the mods
to get (any or all of them) and the mods to avoid, a tier range, the
rarities and corrupted or not. It builds one search string from those rules
and keeps it within the game's 250-character limit; when it cannot, it says
so and copies nothing. `Ctrl+Alt+W` (changeable) puts the string on the
clipboard, and you paste it into the stash or a vendor's search box. It
never touches the game: no input, no memory read, no network.

- Guide (how the search is built, the mod data, privacy, known
  limitations): [`docs/waystone-roller/README.md`](docs/waystone-roller/README.md)
- Download: the latest Waystone Roller release on the
  [releases page](https://github.com/S-Borkowski/ExilePact2/releases?q=waystone-roller).
  Each release has `waystone-roller.exe` and `SHA256SUMS.txt`; there is no
  installer. The first release is published after this change merges.
- Requirements: Windows 10 or 11, 64-bit, with the WebView2 runtime; an
  English game client.

To build it from source instead, with the same toolchain as the quest
tracker:

```sh
npm --prefix apps/waystone-roller ci            # only when apps/waystone-roller/node_modules is missing
npm --prefix apps/waystone-roller run release
# -> apps/waystone-roller/target/release/waystone-roller.exe
```

## Repository layout

| Path | What it holds |
|---|---|
| `apps/quest-tracker/` | The overlay: Rust crates for the log tailer, parser, quest engine, view-model and store, and the Svelte UI. |
| `tools/quest_tracker/` | Developer-only tools: the release cutter, the poe2db quest data harvester and the outside instruments used for research. |
| `apps/waystone-roller/` | The waystone roller: the search-string engine and app logic as Rust crates, the Tauri glue, the Svelte UI and the mod data. |
| `tools/waystone_roller/` | Developer-only: the release cutter and the poe2db Waystones data tool. |
| `.github/workflows/` | The two release builds (the quest tracker's and the waystone roller's), and the opt-in AI review. |
| `docs/quest-tracker/` | The quest tracker's guide, research note and design. |
| `docs/waystone-roller/` | The waystone roller's guide, research note and UI design. |
| `docs/RUNTIME_DATA_MODELS.md` | Facts about the game that every mod can use, each labelled measured or read. |
| `tools/`, `tests/` | The tests for all of the above, and the agent tooling's own tools. |
| `AGENTS.md`, `.claude/`, `.codex/`, `docs/agents/`, `docs/tools/` | The rules and tooling for coding agents. |

## Tests

```sh
py -3 -m unittest discover -s tests                                              # every Python test
npm --prefix apps/quest-tracker run build                                        # needed before the workspace build
cargo test --manifest-path apps/quest-tracker/Cargo.toml --locked --workspace    # every crate
npm --prefix apps/quest-tracker test                                             # the UI, in jsdom
npm --prefix apps/waystone-roller run build                                      # the same two steps for the waystone roller
cargo test --manifest-path apps/waystone-roller/Cargo.toml --locked --workspace
npm --prefix apps/waystone-roller test
```

No test opens a window or reads a real `Client.txt`.

## Releases

Each app is released on its own, from the tip of `main`, by tagging it:

```sh
py -3 -m tools.quest_tracker.release cut --push      # the quest tracker
py -3 -m tools.waystone_roller.release cut --push    # the waystone roller
```

The tag `quest-tracker-v<version>` starts
[`.github/workflows/quest-tracker-release.yml`](.github/workflows/quest-tracker-release.yml),
which builds both of the quest tracker's executables;
`waystone-roller-v<version>` starts
[`.github/workflows/waystone-roller-release.yml`](.github/workflows/waystone-roller-release.yml),
which builds `waystone-roller.exe`. Each builds on a Windows runner and
attaches its files and `SHA256SUMS.txt` to a **draft** release, with the
version's changelog from the app's guide as its body. The owner publishes
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
