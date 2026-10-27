# ExilePact2

Tools and mods for Path of Exile 2. The first one is the quest tracker.

## Quest tracker

`quest-tracker.exe` is a small always-on-top overlay that lists the Path of
Exile 2 campaign quests (Acts 1 to 6, 87 quests in 0.1.0). It says which ones
are worth doing and why, and keeps each character's progress. It learns that
progress from the game's `Client.txt` log. Two settings are opt-in: *Read the
screen* also reads three regions of the game's window with Windows' own OCR,
and *Hide under game panels* hides the overlay while a game panel such as the
inventory is open under it. It sends no input to the game and never reads the
game's memory.

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

## Repository layout

| Path | What it holds |
|---|---|
| `apps/quest-tracker/` | The overlay: Rust crates for the log tailer, parser, quest engine, view-model and store, and the Svelte UI. |
| `tools/quest_tracker/` | Developer-only tools: the release cutter, the poe2db quest data harvester and the outside instruments used for research. |
| `.github/workflows/` | The release build, and the opt-in AI review. |
| `docs/quest-tracker/` | The quest tracker's guide, research note and design. |
| `docs/RUNTIME_DATA_MODELS.md` | Facts about the game that every mod can use, each labelled measured or read. |
| `tools/`, `tests/` | The tests for all of the above, and the agent tooling's own tools. |
| `AGENTS.md`, `.claude/`, `.codex/`, `docs/agents/`, `docs/tools/` | The rules and tooling for coding agents. |

## Tests

```sh
py -3 -m unittest discover -s tests                                              # every Python test
npm --prefix apps/quest-tracker run build                                        # needed before the workspace build
cargo test --manifest-path apps/quest-tracker/Cargo.toml --locked --workspace    # every crate
npm --prefix apps/quest-tracker test                                             # the UI, in jsdom
```

No test opens a window or reads a real `Client.txt`.

## Releases

A release is cut from the tip of `main` by tagging it:

```sh
py -3 -m tools.quest_tracker.release cut --push
```

The tag `quest-tracker-v<version>` starts
[`.github/workflows/quest-tracker-release.yml`](.github/workflows/quest-tracker-release.yml).
It builds both executables on a Windows runner and attaches them to a
**draft** release, with the version's changelog from the guide as its body.
The owner publishes the draft and sets it as the latest release by hand. The
guide's "Working on the app" › "Cutting a release" has the whole procedure.

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
