# Quest tracker

A small always-on-top overlay for Path of Exile 2 that lists the campaign
quests, says which ones are worth doing and why, and keeps each character's
progress. It learns that progress from the game's `Client.txt` log and from
nothing else, unless you turn on the opt-in *Read the screen* setting, which
also lets it read three regions of the game's window with Windows' own OCR
("How detection works" › "Reading the screen"). In play mode over the game
it also copies small patches of the game's window, to hide the overlay
while a game panel is open under it, and reads no text ("How detection
works" › "Hiding under game panels"); that is always on since 0.1.1.

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
- **Shown only over the game.** In play mode the overlay is visible only
  while the game's window or the overlay's own is in front; switch to a
  browser or the desktop and it hides, switch back and it shows again,
  without taking focus. From the game's start until a character is in an
  area it stays hidden, so it does not cover the login and
  character-select screens. In edit mode it is always shown.
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
  character, open settings, move the window by dragging the panel's
  header (outside its buttons), and resize it by dragging the panel's
  edges and corners, as any Windows window. The global shortcut (default
  `Ctrl+Alt+Q`) switches between them. Back in play mode the panel shows
  the quest list: Settings, the character picker and a quest's detail
  close (since 0.1.1).
- **A window you size, a scale that zooms.** The window keeps the size you
  gave it by its edges in edit mode, across restarts; the *Scale* setting
  zooms the panel's contents and never resizes the window (since 0.1.1).
- **Two views.** *Next up* lists only what is available or in progress now,
  grouped by worth; *All* lists every act's quests under the act's header.
- **Skip sets a quest aside without losing it.** Any quest can be skipped
  in edit mode, one not picked up yet included, and so can the quests of a
  character still shown as "New character". A skipped quest stays
  tracked: the log keeps moving it, *Show skipped quests* lists it with the
  step it has reached, and Unskip shows it exactly where that progress is.
- **A find-it tip under a step.** A short "how to find it" hint, the
  owner's own words, shows under the step it helps with, from a `tip`
  field in the quest data that can be extended by hand ("Quest data").
- **A tray icon.** The app's icon sits in the notification area, tooltip
  "Quest Tracker", with three menu entries: *Check for updates*,
  *Settings* and, under a separator, *Quit Quest Tracker*, the way to
  close the overlay even while play mode has it hidden. Closing its window
  quits too. While a newer version is known, a fourth entry, *Update
  available: X*, sits at the top and opens that release's page in your
  browser.
- **A Settings window of its own**, opened from the tray, separate from
  the panel's settings frame. For now it holds one setting, *Automatically
  check for updates*, off by default, a *Check now* button with what the
  last check said, and the line "Created by ST4H".
- **An update check, only when you ask.** *Check for updates* in the tray,
  or *Check now* in the Settings window, asks GitHub once whether a newer
  Quest Tracker release is published; with *Automatically check for
  updates* on, it also asks once at launch and then once a day. A newer
  version is announced in the window and the tray, with a way to open its
  release page in your browser. Nothing is downloaded or installed.
- **The row itself changes** when a quest becomes available, is started,
  has a step done or is done, while you play: the row is highlighted in its
  new status colour for about 2.5 s, the completed step fades out and the
  next one fades in, and a screen reader hears one sentence saying what
  changed. The highlight shows with Windows' animation effects off too;
  only the fades stop. Lines read as catch-up change rows without either.
- **Read the screen, off by default.** With this setting on, the overlay
  also reads the game's window: while the game is in front and a quest
  step the screen can finish is open in the current area, it copies up to
  three regions of the window (the boss bar's band, the lower-left list of
  voiced lines and the dialogue window) and reads them with Windows' own
  OCR, in memory. A boss's bar going ticks the defeat step of Beira of the
  Rotten Pack, The Bloated Miller, Blackjaw and Lythara; the character's
  remark at a boss's death ticks the slaying step of the Miller, Blackjaw,
  Lythara and sixteen other bosses, for each class poe2db gives a remark;
  and the QUEST COMPLETE banner in Renly's window ticks "Take Renly's
  reward". Nothing read is stored, logged or shown ("How detection works",
  "Privacy").
- **Hide under game panels, always on.** The overlay hides itself in play
  mode while one of seven game panels is open
  under its window (the inventory, the passive tree, the shop on M, the
  options on O, the world map on U, the stash and a merchant's trade
  window), and shows again once the panel closes. The Escape menu does not
  hide the overlay. That is the owner's choice of 2026-10-27, made after a
  live session had seen it hide: with the game's menu open, the tracker
  stays in view. It
  tells a panel is open by the colour of a few small patches of the game's
  window where that panel draws its frame, measured in a live session, and
  stores, logs and shows no pixel. Since 0.1.1 it has no setting: its
  toggle is gone, and a 0.1.0 settings file that turned it off no longer
  does.
- **A replay CLI**, `quest-tracker-replay`, that runs the same engine over a
  saved log without the game, the screen reader's trackers over a kept
  research capture (`screen`), and the panel check over one (`panels`).
- **A developer-only Python tool**, `py -3 -m tools.quest_tracker.poe2db`,
  that fetches poe2db.tw's quest pages to rebuild the quest data after a
  patch.

What it is not:

- It does not read game memory, hook, inject, send input to the game or talk
  to Grinding Gear Games. It reads one log file and draws its own window;
  only with *Read the screen* on does it also copy regions of the game's
  window from the screen, as a screenshot would, and read them with
  Windows' OCR; and, to hide under game panels, it copies small patches of
  that window in play mode and compares their colour, reading no text.
- It does not track party members, endgame or Atlas quests, or a client in a
  language other than English.
- It is not an installer: there is no setup, auto-update, code signing or
  bundled WebView2 runtime. The update check only tells you a newer
  version is out; it downloads nothing and replaces nothing.
- It makes no network request unless you ask it to check for updates (the
  tray's *Check for updates*, the Settings window's *Check now*, or
  *Automatically check for updates* turned on), and then only the one
  request to GitHub's API that "Privacy" › Network describes. The links it
  opens, a quest's poe2db page and a release page, open in your default
  browser.

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
- **For *Read the screen* only: Windows' English OCR and a 16:9 window.**
  The setting reads with Windows' own OCR (`Windows.Media.Ocr`) in en-US.
  On a Windows that has no en-US OCR it refuses, with the status line
  "Screen reading is off: Windows has no English OCR.", and the rest of
  the overlay runs as before. The regions it reads are fractions of the
  game's client area, measured on one setup: a 2560×1440 borderless
  window, English client, at the in-game UI scale used then. A window that
  is not 16:9 is refused while it lasts; other 16:9 sizes and other UI
  scales are untested (Known limitations).
- **For hiding under game panels: a 16:9 window,** as for *Read the
  screen*, though hiding is always on and needs no setting. The patches it copies are fractions of the game's client area,
  measured on the same setup (2560×1440 borderless, English client). While
  the window is not 16:9 it pauses, with the status line "Hiding under game
  panels is paused while the game's window is not 16:9 (<w>x<h>).", and the
  overlay is not hidden for a panel. It needs no OCR.
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
| `--from-start` | Read the whole existing log as catch-up, ignoring the saved position. Useful once after a fresh install with an existing log; catch-up lines never play a row transition. |

Each option also takes the `--name=value` form. An unknown argument is not
fatal: the overlay starts without options and says so in its status line and
diagnostic log. A second launch while one is running is refused (single
instance).

To quit, choose *Quit Quest Tracker* from the tray icon's menu (it works
while the overlay is hidden), or close the overlay's window (a close
request, `WM_CLOSE`, ends it shown or hidden). Either writes one `quit:`
line to the diagnostic log and then stops the tracker as on any exit,
saving progress first (`stop: ...`).

The tray icon's menu, top to bottom: *Update available: X* (only while a
newer version X is known: opens its release page in your default
browser), *Check for updates* (opens or focuses the Settings window and
starts a check), *Settings* (opens or focuses the Settings window), a
separator, and *Quit Quest Tracker*. Its tooltip is `Quest Tracker`, or
`Quest Tracker (X available)` while a newer version is known. A left click
opens the menu too.

**The Settings window**, titled "Quest Tracker Settings", is an ordinary
window, decorated, dark and not resizable (420 by 280 at 1x, centred,
in the taskbar while open), separate from the panel's settings frame and
reached only from the tray. It is made the first time it is asked for and
destroyed when closed; asked for again while open, it is shown and
focused. It holds one section, *Updates*, and the line "Created by ST4H"
at the bottom; later settings are new sections. *Updates* has:

- **Automatically check for updates** (off), with the helper text "Asks
  GitHub once a day whether a newer version is out. Downloads nothing.
  Off: Quest Tracker never connects unless you ask it to check."
  `settings.json` stores it as `"check_updates"`; a file written before
  it existed loads it off. With it on, a check runs when the app starts
  (on the worker's first tick), then once a day after the last check
  started, and an hour after an automatic check failed. When checks
  happen is not stored: each launch with it on checks once.
- What the last check said: `Not checked yet.`, `Checking for updates…`,
  `You have the latest version (<running>).`, `Quest Tracker <new> is
  available. You have <running>.`, `No Quest Tracker release is published
  yet.` or `Could not check for updates: <reason>.`, the reason being one
  of `GitHub did not answer in time`, `GitHub could not be reached`, `the
  connection to GitHub was not secure`, `GitHub refused the check for now
  (rate limit), try again later` (status 403 or 429), `GitHub answered
  <status>`, `GitHub's answer was too large`, `GitHub's answer could not
  be read`, `the request stopped unexpectedly` (the request's own thread
  ended without an answer), `the request failed (error <code>)` or `this
  build cannot check for updates`.
- **Check now**, disabled while a check runs, and **Open the release
  page** while a newer version is known; under them, the running version.

A check never holds up the panel, the commands or the tracker's worker:
its one request runs on a thread of its own, and one check runs at a
time (a second request while one runs starts nothing). The newest release
is the highest published, non-prerelease release tagged
`quest-tracker-v<major>.<minor>.<patch>`; drafts, prereleases and other
tags are skipped. It is compared with the running build's version: newer
is announced, equal or older is up to date. The release page opened is
`https://github.com/S-Borkowski/ExilePact2/releases/tag/quest-tracker-v<version>`,
built from the version, never taken from GitHub's answer. In play mode,
with the Settings window in front, the overlay hides as it does for any
other window that is not the game's.

Which log it follows: `--log`, else the Settings log path, else the first of
the standalone and Steam paths above whose file exists. With none, the panel
shows the "no log found" state.

Settings (edit mode, the settings frame) and their defaults:

- **Opacity** `0.9`, a slider from 0 to 1 in 0.05 steps. It is the panel
  background's opacity only: at 0 the background is gone, while text,
  icons, the panel's border and the edit-mode accent stay fully opaque,
  so edit mode is still told apart. It applies while the slider is
  dragged (at most one update per animation frame, the last value always
  sent), not only on release.
- **Scale** `auto`: an *Auto scale* switch, plus a slider for a fixed
  factor from 0.75 to 2 in 0.05 steps, applied as the webview zoom (see
  below). Since 0.1.1 it zooms the panel's contents only and never resizes
  the window, whose size is yours (below); the panel's frame fills the
  window at every scale (seen live at 0.75, 1.5 and 2, Live 14, after a
  first build left its margin zooming with the content). Moving the slider turns Auto off. It too applies while dragged:
  since each step rezooms the page under the pointer, the value follows
  the pointer's travel since the press, in screen pixels, over the track's
  width at the press, so the zoom changing does not move it; the arrow
  keys still step it. `settings.json` stores
  `"auto"` or the factor as a number; the earlier `"1"`, `"1.33"` and
  `"2"` load as those factors, and a malformed or out-of-range value loads
  as Auto.
- **Show skipped quests** (off): with it on, a skipped quest is shown
  whatever *Only worth-it* says: in *All* always, and in *Next up* when its
  tracked progress is available or in progress, after the unskipped rows of
  its worth group. It keeps its skipped mark and shows the step that
  progress has reached. With it off, no skipped quest is listed. **Only
  worth-it quests** (off) hides the quests that are not worth it and not
  skipped. Neither changes an act's counts.
- **Log path** (unset: discovery) and **hotkey** `Ctrl+Alt+Q` (the
  global-shortcut plugin's accelerator syntax).
- **Read the screen** (off), the toggle under *Only worth-it quests*, with
  the helper text "Reads the game's window for boss bars and quest
  dialogue. Saves nothing it reads." With it on, the overlay reads parts
  of the game's window for the quest steps the screen can finish ("How
  detection works" › "Reading the screen"). `settings.json` stores it as
  `"screen_reading"`. Turning it on starts the screen reader at once, and
  turning it off stops it (`screen: off` in the diagnostic log); an
  overlay started with it on starts the reader too. On a Windows without
  English OCR it refuses (Platforms and prerequisites).

Two switches of 0.1.0 are gone from the settings frame in 0.1.1, in both
modes. *Click-through (play mode)*: play mode is click-through and edit
mode takes clicks, as before, and the shortcut or the edit footer's way
back switches between them. *Hide under game panels*: play mode always
hides the overlay while one of the game panels in "How detection works" ›
"Hiding under game panels" is open under the overlay's window, and shows
it again once the panel closes, on a 16:9 game window (Platforms and
prerequisites). The panel watcher starts with the overlay on Windows
(`panels: on` at every start) and runs until it exits. A
`"hide_under_panels"` key in a 0.1.0 `settings.json`, on or off, is
ignored on load and gone after the next save, and a settings change that
names it is refused as unknown.

**The window's size is yours.** In edit mode the window is resizable:
each of the panel's four edges and four corners is a resize handle, a
band a few pixels wide straddling its border that shows the matching
resize cursor and draws nothing, and a left-button press on it resizes
the window from that edge or corner, as any Windows window. The panel
fills the window, minus the 12 px transparent margin that holds its
shadow, and its body scrolls. In play mode the window is not resizable
and no handle is there, so the whole window stays click-through; the
collapsed badge has none. A resize made in edit mode is kept like a move,
in `settings.json`'s `"window"` object beside `"x"` and `"y"`, as
`"width"` and `"height"` in whole logical pixels, so a DPI change keeps
the panel's apparent size; it is applied at the next start. A resize in
play mode is not kept. The smallest window is 320 by 240 logical pixels,
whatever the zoom, at which the edit-mode header and the way back to
play mode each still fit on one line at zoom 1; a saved size below it
loads as that minimum. With no saved size, as for every 0.1.0 user, the
window opens at 344 by 408 logical pixels (the panel's 320 by 384 plus
the margin) times the starting zoom, never below the minimum, and that
size is not saved until you resize. Seen live (Live 14, 2026-10-28): the
resize cursor at the left edge, the bottom edge and the bottom-left
corner, the window resized from each, its size and place kept across a
restart, and in play mode a click just inside the left edge reached the
game.

There is no dock side. A window with no saved position goes to the right
side of its monitor, below the game's minimap; in edit mode you drag it by the panel's header
(outside its buttons) and the position is saved. A `dock` key in an older
`settings.json` is ignored on load, and a settings change that names it is
refused as unknown. A hotkey that cannot be registered keeps the
old one working and says so in the status line. If the shortcut cannot be
registered at launch, the overlay starts in edit mode instead of play
mode, since without a shortcut play mode could not be left: the status line
says so, and the diagnostic log names the `settings.json` whose `"hotkey"`
you can change.

Scale `auto` picks the zoom by the monitor the window is on (the zoom only;
the window keeps its size): 1x below
1440 px of monitor height, 1.33x from 1440 and 2x from 2160 (physical
pixels), divided by Windows' DPI scale for that monitor (which the webview
already applies), and never below 1. So a 1440p screen at 100% gets 1.33x,
a 4K screen at 150% gets 1.33x and at 200% gets 1x, and a 1080p screen at
150% stays at Windows' 150%. Auto is worked out at start, after the
window is placed (so from the monitor it was placed on), when the scale
setting changes, and again whenever the window moves to another monitor or
that monitor's DPI scale changes; the zoom is applied only when it differs
from the one already applied, and a window at its default place (never
dragged) is placed again after it. A fixed scale never reads the monitor. The diagnostic log records what
Auto chose from (`scale auto: monitor … px high at DPI scale … -> zoom …`
at start, `scale auto: the window is now on a monitor … -> zoom …` after a
move or a DPI change).

**What the diagnostic log says.** `overlay.log`, in the store folder
("Privacy"), records the tracker's decisions, never a line of the game's
log. It never names a character: where a line below shows `<name>`, the
log writes `character #<token>` for one of yours (six hex digits that stand
for one name until the overlay exits, so its lines can be followed within a
run, and are drawn at random each run, so a shared log cannot be checked
against a guessed name), `an unconfirmed character` for one only a level-up
named, or `none`. The lines to look for when a quest did not advance:

- `area from log: <area code>`: the current area, read back from the log
  before the position the tailer starts at ("How detection works"). When
  it found none: `area from log: none (no area line in the <n> bytes before
  offset <offset>)` or `area from log: none (a session marker follows the
  last area line)`. When it did not look: `area from log: not read (the
  tailer reads the log from offset 0)` or `area from log: not read (the log
  does not exist yet)`. When the read failed: `error: area from log:
  <error>; the area is unknown`.
- `area unknown: refused <quest id>`: a signal of that quest, tied to an
  area, arrived while the area was unknown and was not counted.
- `catch-up: <n> lines read, character <name>, area <code|unknown>`: after
  each batch of catch-up lines, the character and the area the catch-up
  left the tracker in (the area may have changed without a live area
  line); the character reads `a new character (The Riverbank)` while the
  new character has no name yet.
- What happened to the new character from The Riverbank ("How detection
  works"), written for catch-up lines as well as live ones, so a pin given
  up or kept while catching up is explained too. A level-1 entry to The
  Riverbank writes nothing itself: it opens a 5 s relog window, and these
  lines are written from what the window decides when it closes:
  - `character: a new character (The Riverbank)` when the relog window
    closes without a relog and a new character starts, and
    `character: the new character resumed (The Riverbank)` when it closes
    on an entry to the same instance as a parked one; either is followed
    by `character: pin on <name> released for the new character` when it
    released a pin. They come when the next line closes the window, or,
    on a quiet log, when the overlay's clock does: at most about 7.5 s
    after the entry (6 s counted from the log's whole-second timestamps,
    up to 1 s of offset between a line's stamp and when it was written,
    and two 250 ms polls);
  - `character: <name> relogged in The Riverbank, kept` when the character
    build line named a stored character in time: a relog, not a new
    character. A relog writes only this line: no new-character, pin or
    resumed line comes before or after it;
  - `character: the new character parked at a session start` (with `, pin
    on <name> restored` when it gave a pin back);
  - `character: a parked new character replaced by another instance of The
    Riverbank, <n> events dropped`;
  - `character: the new character named <name> by a level-up`, or
    `character: the new character's progress given to <name>, pinned` when
    you pinned a character while it was open.
- At start, when `new-character.json` holds a record: `character: the new
  character reopened from its record` or `character: a parked new
  character loaded from its record`; `character: the new character's
  record could not be read and was set aside` (quarantined) or `error: the
  new character's record: <error>` when it cannot be used.
- `character: <from> -> <to>` whenever the current character changes.
- `row change <id> <kind> (<quest id>)` for each live change sent to a
  row (`available`, `step_done` or `quest_done`; a quest started by its
  `start` signal is a `step_done` with no step completed).
- `visibility: hidden (the window in front is neither the overlay nor the
  game pid from the log)` when play mode hides the overlay because neither
  the game's window nor the overlay's own is in front, and `visibility:
  shown` when it shows again (or edit mode shows it). Each change is
  written once; no window title, other program's name or other pid is
  ever written. The game's window is told by its process id, the `<pid>`
  in the `[<LEVEL> Client <pid>]` header of the newest log line read (at
  start, read back from the log before the start position with the
  current area). Until a line with that header has been read, the overlay
  stays shown.
- `visibility: hidden (no character in an area yet this session)` when
  play mode hides the overlay after a session marker (`LOG FILE OPENING`),
  whatever window is in front, so it does not cover the login and
  character-select screens. It is written once per hide; a launch that
  writes the marker again only restarts the wait. The session's first area
  line (`Generating level <n> area ...`) lifts it, with `visibility: shown`,
  or the hidden line above when another program's window is in front. At
  start, the marker and area lines read back from the log count the same
  way, so a last session with an area line starts with the hide lifted.
  The hide holds only while the game pid from the log is trusted: once
  that pid is not used to hide the overlay (below), it shows as before.
  Edit mode is always shown, so the hotkey still reaches the settings.
- `visibility: hidden (a game panel is open under the overlay)` when play
  mode hides the overlay because the panel watcher reads a game panel
  open under it, and `visibility: shown` once the panel closes (or the
  watcher pauses or stops). The other two hides come first: with no
  character in an area yet, or another program's window in front, the
  line names that reason and the watcher does not read.
- The game pid it compares with, each time it changes: `game pid: <pid>
  (from <source>)`, the source being `a live line's header`, `a catch-up
  line's header` or `the log before the tailer's start`. A pid from a live
  line is the running game's. One read only from catch-up or from before
  the start may be an old log's, so it is trusted for 10 s: `game pid:
  <pid> confirmed (its window is in front)` when a window of that pid comes
  to the front within them, or `game pid: <pid> not used to hide the
  overlay (no live line and no window of it in front for 10 s: the log may
  be one the running game does not write to)` when none does. From then on
  that pid hides nothing, and only a live line with it trusts it again (a
  window in front does not, since Windows reuses pids).
- `tray: icon shown` at start, or `error: tray icon: <reason>` when the
  tray icon could not be made; the overlay then runs on without it (quit by
  closing its window). `error: tray menu: <reason>` when its menu could not
  be brought up to date.
- `updates: ...` lines: `updates: automatic checks turned on` and
  `updates: automatic checks turned off` when the setting changes;
  `updates: check started (manual)` or `updates: check started
  (automatic)`; then one of `updates: up to date (running <cur>, latest
  <new>)`, `updates: <new> available (running <cur>)`, `updates: no
  release published` and `updates: check failed: <reason>` (the reasons of
  the Settings window above); and `updates: opened the release page of
  <new>`. `command open_release_page refused: no newer version is known`
  when there is no page to open, and `error: opening the release page of
  <new>: <reason>` or `error: the update check thread did not start:
  <reason>` on those failures. Nothing of GitHub's answer is ever logged.
- `quit: from the tray` or `quit: the window was closed`: the first quit
  request (a second writes nothing), followed by the `stop: ...` line of
  the shutdown. With the screen reader running, that line names it too
  (`stop: tailer, worker and screen reader ended, progress saved`), and so
  with the panel watcher (`stop: tailer, worker, screen reader and panel
  watcher ended, progress saved` with both, `stop: tailer, worker and
  panel watcher ended, progress saved` with the watcher alone).
- `screen: ...` lines, with *Read the screen* on: when the reader starts,
  idles, reads, is refused or covered, sees a bar go or a banner, and once
  a minute. Every form is in "How detection works" › "Reading the screen".
- `panels: ...` lines, on Windows from every start: `panels: on` (since
  0.1.1 there is no `panels: off`: the watcher stops only with the
  overlay, on the `stop: ...` line); `panels: idle (<reason>)` and `panels: paused (<reason>)`,
  `panels: resumed` and `panels: stopped after an error`; `panels: overlay
  at x <a> to <b>, y <c> to <d>`, the overlay's place over the game's
  window; `panels: <entry> open`, `panels: <entry> closed` and `panels:
  <entry> unseen under the overlay`; and once a minute `panels: minute:
  <n> reads, <n> copies, <n> open, <n> black copies, busy <n> ms`,
  followed by one `panels: <entry> best read: <r> of <n> probes read, <c>
  in colour, <s> within spread, <m> match, <k> needed` line per entry read
  in that minute, which says in counts why an entry kept reading closed.
  An error's detail goes on an `error: panel watcher: ` line. Every form,
  and how to read a best-read line, is in "How detection works" › "Hiding
  under game panels".

Build it:

```sh
npm --prefix apps/quest-tracker ci            # only when apps/quest-tracker/node_modules is missing
npm --prefix apps/quest-tracker run release   # tauri build --no-bundle, then the replay CLI
# -> apps/quest-tracker/target/release/quest-tracker.exe
# -> apps/quest-tracker/target/release/quest-tracker-replay.exe
```

`npm run release` builds both executables: `tauri build --no-bundle` for
the overlay, then `cargo build --release -p qt-replay --locked` for the
replay CLI.

### `quest-tracker-replay`

The same engine over a saved log, without the game or a window.
`npm run release` builds it as
`apps/quest-tracker/target/release/quest-tracker-replay.exe`; on its own,
build it with `cargo build --manifest-path apps/quest-tracker/Cargo.toml -p
qt-replay --locked --release`, or run it in place:

```sh
cargo run --manifest-path apps/quest-tracker/Cargo.toml -p qt-replay --locked -q -- state path/to/Client.txt
```

```text
Usage: quest-tracker-replay <command> <log> [--data PATH] [--json] [--speakers NAMES]
```

| Command | Prints |
|---|---|
| `events` | One recognised event per line (never chat, unknown speakers or other players' names). |
| `state` | The engine over the whole log, no store: a summary line (`<n> lines, client <version>, <h> events held, <d> dropped before any character, <x> dropped with a replaced new character, <r> refused`), then per own character its class, level and quests by status per act. `dropped` counts events that came before any character and went to none (including those held when a new character from The Riverbank started); `dropped_new_character` counts, apart from it, the events of an unnamed new character that a level-1 entry to another instance of The Riverbank replaced; `refused` counts area-tied signals refused while the area was unknown, so a quest that did not advance because the area was unknown is told apart from a signal that never matched. A level-1 entry to The Riverbank whose relog window is still open at the end of the log is decided as the overlay would decide it on a quiet log, 6 s after the last line, and the summary then adds `a relog window was still open: closed at the end of the log, as the overlay would 6 s after the last line`. With `--json`, one object `{"lines", "client_version", "held", "dropped", "dropped_new_character", "refused", "relog_window_closed_at_end", "characters": [{"name", "class", "level", "own", "active", "quests": {<id>: <status>}, "acts"}], "new_character"}` (`new_character`: the unnamed new character's class, level, event count, quests and acts, or `null`). |
| `import` | `state`, then written to the store (`QUEST_TRACKER_DIR`, else `%LOCALAPPDATA%\ExilePact2\quest-tracker`) with the position at the end of the log, so the overlay resumes from there. Running it twice leaves the same files. |
| `scan` | Per area code: display name, entries, reward lines, points lines, allowlisted speakers and system lines, as counts (no chat, no character names). The input for building the quest data. |
| `screen` | The screen reader's trackers over a kept research capture instead of a log (below). |
| `panels` | Hiding under game panels over a kept research capture: `measure` (the panels' footprints and probe candidates) and `detect` (the shipped panel table's test per frame); and two research commands for the raw copies the app reads, `grab` and `probe` (below). |

| Option | What it does |
|---|---|
| `--data PATH` | Quest data file (default: the `data/quests.json` built in). |
| `--json` | JSON output (`events`: one object per line). |
| `--speakers NAMES` | Extra speakers to allowlist for `events` and `scan`, comma-separated; may be repeated. |
| `-h`, `--help` | The usage text. |

The log is opened read-only. Exit codes: `0` done, `1` error, `2` no such log
file (for `panels`, a capture folder without its index), `64` usage error.

**`quest-tracker-replay screen`** runs the same trackers the overlay's
screen reader uses over a research session's kept capture, without the
game or a screen:

```text
Usage: quest-tracker-replay screen --live FOLDER [--bar NAME]... [--dialogue NPC]...
                                   [--class CLASS [--data PATH]]
```

| Option | What it does |
|---|---|
| `--live FOLDER` | The kept live folder (required). It reads `FOLDER\i3\frames.jsonl`, the capture's index, and `FOLDER\i3\ocr.jsonl`, its OCR cache ([`outside-instruments.md`](outside-instruments.md) § "Instrument 3: screen capture, OCR and the marker"). |
| `--bar NAME` | A boss whose bar going is `bar_gone`; may be repeated. |
| `--dialogue NPC` | An NPC whose QUEST COMPLETE banner is `quest_complete`; may be repeated. |
| `--class CLASS` | Read `kill_remark` for every boss the quest data has a kill-remark text of that class for (the `remarks` table, "Quest data"). |
| `--data PATH` | The quest data file for `--class` (default: the one built in). |
| `-h`, `--help` | The usage text. |

Each region is observed from the frames of the smallest captured band that
contains it, and each frame becomes one read as the reader would make it:
a frame the capture skipped, one taken with the game not in front, and a
black copy are hidden reads; any other frame with cached OCR is a visible
read of the lines whose box lies wholly inside the region (the heading
strip, for a shut read of the dialogue window); a frame with no cached OCR
is skipped, never counted absent. The overlay's reader copies only the
region, so it would read a line across the region's edge clipped; the cache
holds that line whole, so the replay drops it and the summary counts it in
the region's `straddling`. It prints one JSON object per event, `{"event",
"target", "at"}` (its kind, the boss or NPC, and when, in UTC; a
`bar_gone` adds `"last_present"`), then a summary line `{"summary":
{"frames", "ocr_cached", "regions", "bar": {"present", "absent",
"covered"}, "events"}}`, where each region's object holds `"band"`,
`"visible"`, `"hidden"`, `"no_ocr"` and `"straddling"`. It prints no OCR
text and writes nothing. Exit codes: `0` done, `1` a missing or unreadable
index, `64` usage error. The kept captures of Lives 6 and 8 are what the trackers'
numbers were settled against; they stay outside the repository.

**`quest-tracker-replay panels`** runs the probe test of hiding under game panels
over a kept capture, measures where each panel draws, and, for a
developer checking the table, copies the game's window once and reports
each probe on that copy:

```text
Usage: quest-tracker-replay panels measure --live FOLDER --windows FILE
       quest-tracker-replay panels detect  --live FOLDER [--windows FILE]
       quest-tracker-replay panels grab    --pid PID --out FILE
       quest-tracker-replay panels probe   --frame FILE [--overlay X0,Y0,X1,Y1]
```

| Option | What it does |
|---|---|
| `--live FOLDER` | The kept live folder (`measure`, `detect`). It reads `FOLDER\i3\frames.jsonl` and decodes its whole-client frames (band `bottom`, with a path, `rect` equal to `client`) from their JPEGs; every other line is skipped and counted. A folder without the index is refused with exit `2` and one line naming the missing file. |
| `--windows FILE` | The windows file (required by `measure`, optional for `detect`): one window per line, `<panel-id> <round> <open-utc> <close-utc>`, the open press made once the panel is fully open and the close press once it has closed; `#` starts a comment. A window's frames are those from 0.5 s after its open press to 0.5 s before its close press. |
| `--pid PID` | The game's process id (`grab`). |
| `--out FILE` | The frame `grab` writes, outside every git checkout. |
| `--frame FILE` | The frame `probe` reads: a grab's `.bmp` or a kept capture's `.jpg`. |
| `--overlay X0,Y0,X1,Y1` | The overlay's rect as fractions of the client rect, as the watcher's `panels: overlay at` line gives them; `probe` reads none of the probes touching it, as the watcher reads none while the overlay is visible. |
| `-h`, `--help` | The usage text. |

What each prints, as JSON lines:

- **`measure`**, one line per panel id in the windows file, `{"panel",
  "windows", "frames", "footprint", "candidates", "probes",
  "suspect_closed"}`: each window's frames, closed reference frames and
  footprint, their union, and the ranked probe candidates, each with its
  `rect`, `bgr`, `spread`, `luma_std`, `margin`, `least`, `closed_near` and
  `outside_reach` (outside the default overlay's reach, x 0.78 to 1.0,
  y 0.30 to 0.97); then `{"summary": {"frames", "skipped", "black",
  "background", "closed", "panels"}}`. A footprint is the bounding box of
  the grid cells (32 px at 2560 wide) whose colour changed from the
  window's closed reference in 90% of its frames; a candidate is a 16 px
  patch stable in the panel's open frames and far from itself in every
  closed one, ranked by that margin. `suspect_closed` names closed frames
  near most candidates: a panel open outside every window.
- **`detect`**, per frame `{"utc", "foreground", "open": [<entry ids>]}`,
  the shipped table's raw test (no tracker, no overlay); with `--windows`,
  per window `{"window", "round", "from", "to", "frames", "detected",
  "covered"}` (`detected`: the window's frames with an entry open whose
  `opens_for` holds the window's panel; `covered`: whether any entry
  stands for it) and once `{"outside": {"frames", "detected": {<entry>:
  <n>}}}` over the frames more than 3 s before or 2 s after every window;
  last `{"summary": {"frames", "skipped", "black", "table": {<entry>:
  [<opens_for>]}}}`.
- **`grab`** (research) copies the game's whole client area once, through
  the same GDI route the panel watcher copies through, with the process
  made DPI-aware first, and writes it losslessly as an uncompressed 32-bit
  BMP. Before any screen call it refuses, with exit `1` and one line
  saying why, a `FILE` that exists and a folder inside a git checkout (a
  `.git` in it or in any parent): a frame is research data kept beside a
  session's capture, never in a repository. It prints `{"grab": <path>,
  "utc", "client": [<w>, <h>]}`; the client size says whether the copy is
  in physical pixels. The app never writes a frame.
- **`probe`** (research), for each entry of the shipped table, per probe
  `{"entry", "probe", "readable", "mean_bgr", "luma_std", "in_colour",
  "within_spread", "match"}` and per entry `{"entry", "read", "counts":
  {"read", "in_colour", "within_spread", "match", "needed"}}`, then
  `{"summary": {"frame", "client"}}`. It cuts each probe at the pixel rect
  the watcher copies for the frame's size, and its counts and verdict are
  the watcher's own, so an entry's counts are the ones a `best read:` line
  would give for a read of that frame.

Nothing but `grab` writes, and numbers go to stdout only: no pixel array,
crop or image. `measure` and `detect` read the quality-90 JPEG frames of
the research capture, `probe` a raw grab as the watcher reads it, which is
how a table tuned on JPEG frames is checked against raw copies.

### `py -3 -m tools.quest_tracker.poe2db` (developer-only)

The only network code in this feature, and it only asks poe2db.tw. Standard
library only.

| Command | Options | What it does |
|---|---|---|
| `fetch` | `--cache DIR`, `--delay S` (default 3, refused under 2), `--only SLUG ...`, `--max-age DAYS` (default 7), `--include-endgame` | Reads `robots.txt` and skips what it disallows, then fetches the quest index and the campaign quest pages one at a time, `--delay` seconds apart, with a User-Agent naming the tool and this repository. Pages cached within `--max-age` days are left alone. Act 10 and act-less MasterMission quests only with `--include-endgame`. |
| `fetch-classes` | `CLASS ...` (e.g. `Warrior Mercenary Ranger Huntress Witch Sorceress Monk Druid`, the eight playable classes), `--cache DIR`, `--delay S`, `--max-age DAYS` | Fetches the named poe2db class pages into the cache, politely and under the same robots.txt and delay rules as `fetch`, for their kill remarks. |
| `extract` | `--cache DIR`, `--out FILE` (default `<cache>\quests-draft.json`), `--with-dialogue` | Drafts one JSON entry per cached quest page (slug, URL, fetch date, name, type, act, rewards, steps; NPC dialogue only with `--with-dialogue`). A page without a steps table is drafted with zero steps and named on stderr; an index with no quest, a malformed steps table, or no steps table in any page stops with a non-zero exit naming the slug. |
| `remarks` | `--cache DIR`, `--data FILE` (default `apps/quest-tracker/data/quests.json`), `--class CLASS ...` | Prints a draft of the cached class pages' kill-remark rows whose boss the quest data names (`remarks` table entries), never a whole table. |
| `coverage` | `--log LOG` (required, read-only), `--cache DIR` | Per cached page, how many dialogue snippets match a logged line of the same NPC exactly, by containment, or by a shared 4-word run. Counts only, never text. |

The cache defaults to `%LOCALAPPDATA%\ExilePact2\quest-tracker\poe2db-cache\`;
both the cache and `--out` are refused inside a git checkout, because raw
pages are never tracked. `--help` on the tool and on each command prints the
same options.

### `tools/quest_tracker/outside/` (developer-only)

Research instruments that watch the game from outside its process, built
for Live 6's question whether Beira's wordless death cry can be seen at
all: a file-I/O trace of the game's reads of `Content.ggpk`, a
process-loopback recording of the game's audio, and screen captures read by
OCR afterwards, with the session analysis that checks them. The tracker
does not use them. Every command and script, the live folder, the one step
that needs an admin prompt and what never leaves the machine are in
[`outside-instruments.md`](outside-instruments.md). Live 8's on-screen
text session (the boss bar under each panel, the character's kill remark,
Renly's dialogue window and the `output_all_dialogue_to_chat` option) is
checked by the `screentext` session analysis,
`py -3 -m tools.quest_tracker.outside.screentext`, described there.

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
| `release` | `tauri build --no-bundle`, then `cargo build --release -p qt-replay --locked`: `apps/quest-tracker/target/release/quest-tracker.exe` and `quest-tracker-replay.exe`. |

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
otherwise starts at the end (live) unless `--from-start`. That start
position is decided once, by the tailer, before it starts reading, so the
area read below and the catch-up agree on where the log was entered. A
truncated, rewritten or rotated file is read again from its start. When
you change the log path in Settings, lines the old file's reader had
already queued are dropped (`log: <n> events of a stopped log reader
dropped` in the diagnostic log): they never reach the quests or the new
file's saved position.

**The current area on start.** A quest's reward or points signal counts
only in the area the quest data names for it, and the area is otherwise
known only from an area line read in this run. So whenever it starts following a log at a
position other than its start (a resume, or a first start at the end), the
tracker first reads back, read-only, at most 8 MiB of the log before that
position for the last area line, and takes it as the current area unless a
session marker follows it (the game restarted, so the area is unknown). The
diagnostic log says what it found (`area from log: …`, every form in the
Command reference). When no area line is within that window, or a marker
follows it, the area stays unknown until the next zone's area line.

**Refused signals.** While the area is unknown, a signal tied to an area
is refused: it is not counted, and you tick the quest by hand if it was
its `done`. A refusal is logged as `area unknown: refused <quest id>` and
counted in the status line ("<n> quest signals came before the area was
known and were not counted") and in the replay CLI's summary
(`refused`). Only a quest that could have changed is refused: one already
done or skipped for the character the event goes to is not, and an event
that goes to no character refuses nothing. A signal that does not count
because you are in a different, known area is the normal case and is not
logged.

Each complete line is parsed into an event: an area being generated, an NPC
line from an allowlisted speaker (quest givers and bosses), a system line, a
reward or loss line (`has received`, `has lost`), a passive or weapon-set
points line, a level-up (`is now level`), a party member joining or leaving,
a death, a session start, the client version. Chat and anything not recognised are dropped at the parser.

The quest data (`apps/quest-tracker/data/quests.json`) gives each quest
signal lists: `unlock`, `start`, per-step signals, `done`, and
`window_closes` (the point after which the log can no longer show the
quest being done, such as entering the next act's town). Status, first
match wins: skipped by you; done (by you, or a `done` signal fired);
unknown (the window closed, it is not done, and the log cannot tell); in
progress (a step fired or was ticked, or a `start` signal fired);
available (unlocked); locked. A fired `start` makes a quest in progress
with no step reached, so its first step is the current one; the row
changes once, saying the quest started. Applying the same line twice
changes nothing, so re-reading a log is safe.

**Skipped quests stay tracked.** Skipping is an override on top of the
quest's evidence, not a stop: the log's signals keep being recorded for a
skipped quest, and the engine keeps its *tracked* status, the one the rules
above give with the skip left out. A skipped row keeps its skipped mark and
muted style, but carries the step that tracked progress reaches, as an
in-progress row does, and its detail shows the steps done and current. With
*Show skipped quests* on, Next up lists a skipped quest whose tracked
status is available or in progress (whatever *Only worth-it* says, after
the unskipped rows of its worth group), and All lists every skipped quest.
Progress on a skipped quest plays no row change and is not announced.
Unskip shows exactly the status and step the evidence gives. An act's
counts and whether it is done do not change.

**Quest actions on the new character.** Mark done, Skip, Unskip, Reset and
a step's tick apply to the character the panel shows: the pinned one, else
the active one, else the unnamed new character from The Riverbank (below).
The new character's overrides are kept in its record, so they survive a
restart of the overlay, and pass to the character it becomes when a
level-up names it or a pin takes its progress. A quest command is refused
(`no character yet`) only when there is no character at all.

**Slay the Bloated Miller.** Since 0.1.1 Reaching Clearfell's "Slay the
Bloated Miller" ticks at his death, by whichever of three routes comes
first:

1. **The level-up to 2 in The Riverbank** (`G1_1`), with or without *Read
   the screen*: a `level_up` signal ("Quest data"). The game logs a new
   character's first level-up at the kill: in one player's log every
   Riverbank visit with a level-up had one, to level 2, after the Miller's
   first line and 0 to 9 s after his last (17 of 17, read on 2026-10-28).
   It fires only on the level-up line itself, for the character the
   engine takes it as (the new character it names, or yours), so a
   character already level 2 or more, or one loaded at start, never
   counts it. Seen live (Live 14, 2026-10-28, a new Warrior, *Read the
   screen* off): the step ticked in the same second as the level-up and
   the Miller's last line, 77 s before Mortimer's thanks, which then did
   not tick it again.
2. **With *Read the screen* on, his boss bar going or the character's kill
   remark** in The Riverbank, for any of the eight playable classes
   ("Reading the screen"). At the kill they come about 2 and 3 s after
   the level-up (a replay of Live 8's capture), so in practice the
   level-up has already ticked the step and the reader has stopped looking
   for him; they are the route for a level-up the engine did not take as
   the character's.
3. **Mortimer's "Well done! Please come inside."** there, when neither of
   those came: still the step's first signal. The log records it after the
   kill, before you enter the encampment (Live 13: the row moved on 15 s
   after his death); it came after the Miller's last line in all 13 of its
   occurrences in one player's log, 14 to 20 s after it in the three
   2026-10-27 sessions. Where it does not come either, entering Clearfell
   Encampment still completes the step.

The Miller's own lines do not tick it: they start with the fight ("Saw
you in half!"), and when they were the signal the row showed "Enter town"
while he was still alive (two live sessions, 2026-10-27). Mortimer is on
the speaker allowlist for that line alone: his signal names The Riverbank
and the exact line, so his gambling and farewell lines in the encampment
move nothing. Walking away from the living Miller with *Read the screen*
on can tick the step early (Known limitations).

**Secrets in the Dark, The Mysterious Shade and Una's Grelwood lines.**
Renly gives Secrets in the Dark in a talk the log does not record, so the
quest has a `start` signal instead: Reaching Clearfell being done, which
happens on entering Clearfell (`G1_2`), the earliest evidence of that talk.
It is an inference, written in the quest's notes; without it the quest
stayed *available* from Clearfell and showed in progress only from the
Grelwood (measured in Live 2). In the Grelwood (`G1_4`), Una's lines move
the quest on: her first line there, "What happened here...?", comes when
you summon her at the Tree of Souls and completes "Search the Grelwood",
leaving "Ask Una for help" current. "I'll wait here. The old magicks will
protect me.", which came a few seconds later in the sessions where the
player talked to her, completes "Ask Una for help" and leaves "Find the
Red Vale" current (that it follows the talk is inferred). Where that line
never comes, entering the Red Vale completes the steps before it. Her
"Good. Now the others!" there completes "Use the Rune Spikes" (an
inference from where it falls, after the Red Vale). In the
Grim Tangle (`G1_6`), her "The old magicks still thrive! The way is
open. Please, hurry." completes The Mysterious Shade's "Speak to Una
about the roots". The Mysterious Shade is no longer started by entering
the Grelwood: that entry comes on the way to Secrets in the Dark, before
The Mysterious Shade unlocks, and had shown it in progress too early.
Una's town lines move nothing, because each of her signals names its
area and its exact line. Measurements:
[`quest-tracker-research.md`](quest-tracker-research.md) § "Per-quest
detection".

**Characters.** The log has no "logged in as" line, so the active character
is inferred: a name in a `has received` or `has lost` line is yours; a name
seen joining or leaving the area is a party member for the rest of that
session; a level-up of yours makes it active and records class and level; a
level-up of an unknown name makes it active but *unconfirmed* until a reward
line or you confirm it in the character picker. Events before any active
character are held for the first one that becomes active in that session,
and dropped and counted in the status line if none does, or if a new
character from The Riverbank starts first (below). A character you pin
in the picker gets every event until you unpin it, except while a new
character from The Riverbank holds the pin it released (below).

**A new character from The Riverbank.** A new character's first area is
The Riverbank (`G1_1`), an area of level 1. The area's level says nothing
about the character, and a character relogging inside The Riverbank enters
`G1_1` again, with the same instance (the same seed), so the entry alone
cannot tell a new character from a relog. Two things can: the seed of the
character's Riverbank instance, and a name line inside a short window.

- **A relog by the seed.** Each stored character keeps the seed of the
  Riverbank instance it was last decided to be in (in its
  `characters/<name>.json`). A level-1 `G1_1` entry whose seed is that of
  exactly one stored character, and of no parked new character, is that
  character's relog, decided at the entry with no wait: it becomes active,
  a pin stays, and the diagnostic log says `character: <name> relogged in
  The Riverbank, kept` (the character by its token, "What the diagnostic
  log says"). A character gets its seed when a decision puts it in
  that Riverbank instance: the new character's seed passes to the name its
  level-up gives it, or to the character a pin hands its progress to, and a
  relog records the entry's seed. An event routed to a pinned character
  never does. Two characters with one seed, or a seed no character has, go
  to the window below.

Otherwise an entry to `G1_1` at area level 1, when no new character is
already open, first opens a *relog window* of 5 s and changes nothing
else: the active character and the pin
stay, nothing is written, and the events that come meanwhile wait. The
window closes on the first of a character build line that makes the entry
a relog (below), another area entry, any line more than 5 s after the
entry, a level-up, a session marker, your pin in the character picker
(which decides it for the pinned character: no new character starts, and
the window's events go to it), or, on a quiet log, the overlay's clock
passing 5 s of log time after the entry. On a quiet log the decision
comes at most about 7.5 s after the entry: 6 s counted from the log's
whole-second timestamps, up to 1 s of offset between a line's stamp and
when it was written, and two 250 ms polls. While the window is open, the
saved log position stays at the start of the entry line, so an overlay
closed inside the window reads the entry again on its next start and
decides it then. Only when it closes
without a relog does the entry start a *new character* for the session,
with no pick: the window's events go to it, so Act 1 progress shows from
the first step. Events held from before the entry are not its own (a new
character's first area is The Riverbank; they are another character's,
played earlier in the session and never named, such as one you switched
away from at the character select), so they are dropped and counted, never
given to it. Until something names it, the header shows "New character"
at level 1, and it is not in the character picker. If a character was
pinned, the pin is released then, not at the entry (the diagnostic log
says so).

- **A relog by the name line.** When the client's character build line
  (`Character <name> has unknown build of path: None`) comes no more than
  5 s after that
  entry, before any other area entry, and names a character the tracker
  already stores, the entry was a relog: the relog window closes having
  changed nothing, so no new character starts and no pin is released;
  that character is active, and the events of the window go where they
  would have gone with the pin and that character (`character: <name>
  relogged in The Riverbank, kept`, the only line written). A relog never
  shows "New character". The line is read only for this: an unknown name,
  or the line coming later, changes nothing, and the window then decides
  as above, starting a new character that stays open until its level-up.
  The client wrote that line on some relogs and not on others, so a relog
  the seed does not decide and the line does not follow still starts a new
  character once the window closes (Known limitations).
- **Named.** The first level-up after the entry, from a name not seen
  joining or leaving the area, names it: that name becomes your character,
  active, with the progress so far, and is stored; if the entry released a
  pin, the named character is pinned in its place. Pinning a character
  yourself while the new one is still unnamed gives that character the
  progress, and a pin while the relog window is still open closes the
  window for that character.
- **Saved across a relaunch.** Until it is named, the new character is one
  provisional record in `new-character.json` ("Privacy"), kept apart from
  your characters, so it never appears in the picker. Closing and
  restarting the overlay mid-session reopens it, with its rows. A session
  marker (the game restarted) *parks* it instead of dropping it, and gives
  back the pin it released. A later level-1 entry to the **same** instance
  of The Riverbank resumes it (`character: the new character resumed (The
  Riverbank)`); an entry to another instance replaces it, and the replaced
  one's events are counted as dropped, apart from the events dropped
  before any character (status line and replay summary). A level-up
  naming it, a relog, or your pin closes it and removes the file.

Measurements and reasons:
[`quest-tracker-research.md`](quest-tracker-research.md) § "Character
detection".

**Talking to NPCs in town.** A quest talk was not observed in the log: the
talk happens in a dialogue window, whose text was not seen in `Client.txt`
(one measured visit to Clearfell Encampment in which the player talked to
its NPCs logged no NPC line), and the voiced lines Clearfell Encampment's
NPCs did log are vendor, gambling and farewell lines; Live 3's visit, in
which the player picked up Treacherous Ground from Renly, logged no NPC
line either. That is not proof
that a talk is never logged, but the tracker cannot count on it, so a talk
step does not advance from the talk itself. Where the game
can only produce the next recorded signal after the talk (for example
entering Clearfell after talking to Renly in Clearfell Encampment), that
signal completes the talk step too, and the quest's notes say `inferred`;
a player who skips the talk is then shown done too early. Where nothing
recorded follows the talk (the reward talks of Treacherous Ground and
Finding the Forge, for instance), the step stays a manual tick. A quest
given in town is shown from the town entry where the giver stands:
Treacherous Ground, which Renly gives in Clearfell Encampment, is
available from the `G1_town` entry, as Finding the Forge is, and in
progress from the Clearfell entry as before (that the town entry stands
for the pickup is inferred). With *Read the screen* on, one reward talk
is seen on the screen instead: the QUEST COMPLETE banner in Renly's
dialogue window ticks Reaching Clearfell's "Take Renly's reward" ("Reading
the screen" below). See
[`quest-tracker-research.md`](quest-tracker-research.md) § "Town dialogue".

**A boss: find it, defeat it, use the item it dropped.** Beira of the
Rotten Pack's steps are "Reach Clearfell" (the `G1_2` entry), "Find Beira
of the Rotten Pack" (any of her lines in Clearfell, the first of which
comes when she is met), "Defeat Beira of the Rotten Pack" and "Use the item
it dropped". No line marking her defeat is known: Live 4 timed one kill
and the log held nothing at it (one controlled kill, nothing in the log;
[`quest-tracker-research.md`](quest-tracker-research.md) § "Rewards after
a boss kill"), so the defeat step has no log signal: **tick it by hand**
when she falls, and the row then shows "Use the item it dropped". With
*Read the screen* on, her boss bar going in Clearfell ticks it instead
("Reading the screen" below).
Until then her lines leave "Defeat Beira of the Rotten Pack" current,
whatever area comes next. Her reward line comes only after you use the
item she dropped, which can be minutes after the kill (seen once, in Live
1), and finishes the quest in any area, from either step: using the item
at once needs no tick. Blackjaw, the Remnant and Lythara, the Wayward
Spear have the same split: their lines reach "Find …", "Defeat …" is
ticked by hand (or by their bar going, with *Read the screen* on), and
their last step is "Use any item left after the
fight", worded as a hint, since whether they drop an item to use is not
established. A character saved by an older build, when a boss's lines were
its fight step, shows the step its other evidence reaches: that stored
line matches no signal now and is ignored. See
[`quest-tracker-research.md`](quest-tracker-research.md) § "Rewards after a
boss kill".

**"Probably done".** Some quests are shown done because of a later event
the log recorded, not their own completion: Reaching Clearfell (done on
entering Clearfell), Secrets in the Dark (done on entering the Cemetery of
the Eternals) and The Mad Wolf of Ogham (done on entering Vastiri
Outskirts) carry `done_inferred` in the quest data. Such a row, and any
step shown done although its own signal never fired and you did not tick
it (the "Use ..." step of a boss quest finished by its reward line, for
instance), shows a small dashed ring beside the done mark: the
"probably done" mark, with the tooltip and accessible text "Probably
done: inferred from a later step the log recorded." It differs from the done mark in
shape, not only colour. It is a mark, not a control: the row's
manual-tick marker is unchanged, and a quest you marked done yourself
never shows it. An inferred step has no tick-by-hand toggle in edit mode
either: its leading mark is plain, and clicking it does nothing. A step
you ticked yourself is not inferred and keeps its toggle, and the quest's
Mark done / Unmark done stays, to correct the inference when it is wrong.

**What needs a manual tick.** Each quest carries a `detection` value, and
the row shows the manual-tick marker for the last two:

- `auto` (32 quests in this version): its `done` signal was measured in a
  real log; the tracker finishes it on its own.
- `partial` (42): some unlock or step signal was measured, `done` was not;
  the tracker shows progress, and once the window closes without a `done`
  the quest goes to *unknown* until you tick it.
- `manual` (13): nothing measured; tick it yourself in edit mode (open the
  quest, tick steps or mark it done).

Which signals were measured, and which are static readings of poe2db, is
argued per quest in [`quest-tracker-research.md`](quest-tracker-research.md)
§ "Per-quest detection".

### Reading the screen

*Read the screen* (off by default, Command reference) adds one source
beside the log: a reader thread that copies parts of the game's window and
reads them with Windows' own OCR (`Windows.Media.Ocr`, en-US). It ticks a
step only through a `screen` signal in the quest data ("Quest data"), and
a log line never fires one. The measurements behind every number below
are in [`quest-tracker-research.md`](quest-tracker-research.md) § "What
the screen shows of each source".

**The demand.** After every batch that changed the tracker's state, the
worker publishes what the screen could finish now: the `screen` signals of
the character a screen event would go to, in the current area, on an open
step of an available or in-progress quest, and the game's process id once
a live line or its window in front has confirmed it. The demand is empty
while the area is unknown, while a Riverbank relog window is open, or with
no character. The reader takes the newest demand at each tick without
waiting for it, and copies nothing while the game's pid is unconfirmed,
the game's window is missing or minimized, the demand is empty, or the
game's window is not in front; it then looks again at most every 100 ms.
Its line names the first of these that held, in that order: `screen: idle
(pid unconfirmed)`, `(no game window)`, `(minimized)`, `(no demand)` or
`(not in front)`, written again only when the reason changes. The game's
window is the largest visible top-level window of that pid, and its
client area is read in physical pixels.

**The three regions,** as fractions of the game's client area (left to
right, top to bottom), each read only while a demanded signal needs it:

| Region | Where | Read | For |
|---|---|---|---|
| boss bar | x 0.25 to 0.75, y 0 to 0.14 | twice a second | `bar_gone` |
| voiced lines | x 0 to 0.45, y 0.35 to 0.80 | twice a second | `kill_remark` |
| dialogue window | x 0.22 to 0.70, y 0.20 to 0.85 | once a second | `quest_complete` |

A read is a GDI copy of the region, upscaled 2 times (less when that would
pass the OCR engine's largest image) and read by the OCR, its lines' boxes
mapped back to fractions of the client area. A name matches a line by the
research instruments' fuzzy match, ported with the same scores: a score of
at least 0.8 for the name against any stretch of the line of about the
name's length. So a short name, such as "Blackjaw", matches a longer bar
title that contains it. Before each read the reader checks the window's
shape (16:9 within 1%), that it can find the overlay's own window, and
whether that window lies over a region; a region the overlay covers is
not read, and counts as a covered read. While the overlay's own window
cannot be found, nothing is copied or read at all, and each due read
counts as covered (the overlay-window pause, Status lines below). A missed
read is skipped, never caught up.

**The bar rule (`bar_gone`).** Each read of the band is, for the boss:

- *present*: the boss's name matches a line anywhere in the band. Its place
  is not checked, since the inventory moves the name left;
- *covered*: the band could not be seen (the overlay over it, the
  overlay's own window not found, the game not in front or minimized, a
  failed or black copy, a failed OCR), or a line
  that is not the name lies over the name's column (x 0.40 to 0.60, its top
  at most y 0.08). The passive tree's title, the world map's headings and an
  ordinary monster's name sit there, and the bar under them cannot be
  seen;
- *absent*: neither. Other text in the band, such as a CHECKPOINT label,
  does not cover.

After at least 2 present reads, 6 absent reads in a row (3 s) with no
covered read since the last present one mean the bar went: one `bar_gone`
event, carrying the time of the last present read, and the tracker then
waits for the bar to show again. Brightness is never used: it does not
tell a band with the bar from one without. The worker then drops a
`bar_gone` whose last present read came before an area entry, or before a
`has been slain` line of the character, that it had received
(`not counted (after an area change)`, `not counted (after a death)`), so
leaving the area or dying does not tick the fight.

A death with the bar up is caught by the reader too. The demand counts the
character's live deaths, and when the count changes, each demanded boss
whose bar is up (seen present, its going not yet reported) has its next
going not counted, with the same `not counted (after a death)` line. That
holds whether the bar goes at the death or stays up through the death
screen and goes on the walk back, when its last present read is after the
death. The bar must then show again (2 present reads) before a going
counts. A boss whose bar was not up at the death is not marked, so dying
to ordinary monsters before the fight costs nothing. The worker's check
stays, for a going the reader had already reported when the death line
came. Walking away from a living boss is not caught (Known limitations).

**The QUEST COMPLETE rule (`quest_complete`).** The NPC's window is open
while a read of the dialogue window shows the NPC's name as a heading: a
line matching the name and no more than a fifth longer than it, so an
`<NPC>: ...` line is not taken for it. A line matching "QUEST COMPLETE"
whose box lies in x 0.35 to 0.65, y 0.72 to 0.84, within 10 s after a read
with the heading, is one `quest_complete` event. Reads of the banner less
than 5 s apart are the same banner and fire once. The next quest's
banner, its objective in capitals in the same place, does not match.

**The dialogue window gate.** A read of the whole dialogue window costs
about 276 ms of OCR, and in town nobody talks for most of a minute. So
the dialogue window is read through its heading strip (x 0.22 to 0.70,
y 0.24 to 0.31, about a tenth of the window, where the window draws the
NPC's name) until the NPC's heading shows there. While the gate is shut,
a read copies and reads the heading strip alone, and that read is the
window's read for the rule above. A strip read that shows a demanded
NPC's heading opens the gate, and the whole window is read at once, in
the same tick; the gate stays open while the last heading read in the
strip, by either kind of read, is less than 10 s old. It cannot miss a
banner the whole-window reads would catch: a banner counts only within
10 s after a read with the heading, and that heading opened the gate. The
NPC's name over him in the world (y 0.461 to 0.484 for Renly) is outside
the strip and opens nothing. On the live crop the strip read Renly's
heading in 23 of 57 reads in the minute before his banner, enough to open
the gate, and the banner was caught (Live 10); other NPCs' headings in the
strip are not measured.

**The remark rule (`kill_remark`).** A line of the voiced-lines list of
the form `<speaker>: <text>` whose text matches a known kill-remark text
for the boss (the quest data's `remarks` table, every class's texts) is
one `kill_remark` event. The speaker, the character's own name, is not
checked: only the text tells a kill remark from the character's other
shouts. A line counts only when it is new: the read shows more lines
reading as the remark than any read 1 s to 5 s before it did, and a read
with no read in that span is not judged, so a remark already on screen
when reading starts never counts. A boss with no text demands nothing.
Since 0.1.1 the table holds the remarks of nineteen bosses for each
playable class poe2db gives one (the Miller's for all eight classes), and
a `kill_remark` signal sits on each of those bosses' slaying steps
("Quest data"); a step with a remark and no bar still demands the
voiced-lines region, as Blackjaw's does.

**What a screen event ticks.** It goes to the character an NPC line would
go to and is live, so its row changes as for a log line. Before a
level-up has named a character from The Riverbank, that is the unnamed
new character, which keeps the step once the level-up names it (the
Miller's bar or remark coming before his level-up). Once a step is
reached, by whatever route, the reader stops looking for its signals, as
for the Miller's once his level-up ticked it. It fires a
`screen` signal when the current area is known and is the signal's, the
signal sits on an open step (any step after the furthest one reached),
and the quest's tracked status is available or in progress; at most one
quest fires, the first in data order. With the area unknown it is refused
and counted with the log's refusals (`area unknown: refused <quest id>`).
It never reads or moves the log's time.

**Status lines.** The screen reader is one source of the overlay's status
line, and the panel watcher ("Hiding under game panels", below) is
another. The most severe line shows; between equals, the screen reader's
comes after every source but the panel watcher's, and the panel
watcher's last. Each change of what the reader says has its diagnostic
line (below), and nothing behind it retries in a loop:

- "Screen reading is off: Windows has no English OCR." The reader ends.
- "Screen reading is paused while the game's window is not 16:9
  (<w>x<h>)." While it lasts.
- "Screen reading is paused: the overlay cannot find its own window."
  While it lasts; a copy would hold the overlay's own rows, which name the
  bar's boss.
- "The overlay covers the boss bar; move it to let screen reading see it."
  (or the regions it covers, named together). Nothing moves the overlay
  for you. The overlap is checked every tick the reader is on, demand or
  not.
- "Screen reading cannot copy the game's window (the capture failed 10
  times in a row)." Failed copies before that count as covered reads only.
  A black copy is not a failed one and does not count toward it.
- "Screen reading sees the game's window copied black (10 times in a
  row); a dark scene or a loading screen looks the same." A near-black copy
  stays a covered read; its run counts apart from the failed copies', and
  each run ends at a copy of the other kind or a good copy.
- "Screen reading stopped after an error; turn it off and on to retry."
  The cause goes on an `error: screen reader: ` line.

**Diagnostic lines.** Every reader line is `screen: ` and one of:

- `on (OCR en-US)`, `off`;
- `idle (<reason>)`, the reason being `pid unconfirmed`, `no game window`,
  `minimized`, `no demand` or `not in front` (The demand, above);
- `refused: <reason>`: `no English OCR`, `the game window is not 16:9
  (<w>x<h>)`, `the capture failed <n> times in a row` or `stopped after an
  error`;
- `reading <regions> for <quest>/<step>[, <quest>/<step>]...`;
- `covered: <region> by the overlay` and `uncovered: <region>`;
- `overlay check off (no overlay window)`, when the overlay's own window
  cannot be found, once until it is found again, and then
  `overlay check on`. While it is off, screen reading pauses: nothing is
  copied or read (the dialogue window's heading strip included), each due
  read counts as covered so no going fires across the pause, and the status
  line says so (Status lines, above);
- `black: <n> copies in a row`, at the black-copy status's limit;
- `bar of <boss> gone (last seen <hh:mm:ss.mmm>Z): <outcome>`,
  `quest complete in <npc>'s window (<hh:mm:ss.mmm>Z): <outcome>` and
  `kill remark for <boss> (<hh:mm:ss.mmm>Z): <outcome>`, where the outcome
  is `ticked <quest>/<step>` or `not counted (<why>)`, `<why>` being
  `after a death`, `after an area change`, `area unknown` or `no open
  step`;
- `minute: <n> frames, <n> present, <n> absent, <n> covered,
  <n> black copies, <n> OCR failures, busy <n> ms`, once a minute. The
  black copies are every region's; a black copy of the boss bar is in
  `covered` too;
- `minute: dialogue window gate, <n> full reads, <n> skipped`, with it, in
  a minute the dialogue window was read: reads of the whole window, and
  reads of its heading strip alone. A read the window was hidden for is in
  neither; frames and busy time count both kinds.

A region is `boss bar`, `voiced lines` or `dialogue window`; boss and NPC
names come from the quest data, never from what was read.

**Shutdown.** The reader stops with the tailer and the worker, by the same
deadline; an OCR call in flight is not interrupted, and a reader past the
deadline is logged as not ended, like the others.

### Hiding under game panels

Hiding under game panels is always on since 0.1.1 (in 0.1.0 it was the
*Hide under game panels* setting, off by default; Command reference). A
panel watcher, a thread of its own (`qt-panels`) beside the screen
reader, runs from the overlay's start to its shutdown on Windows. It
copies a few small patches of the game's window where a game panel draws
its frame, compares their colour with the colour measured for that panel,
and tells the worker when a panel is open under the overlay; the worker
then hides the overlay. It reads no text and uses no OCR. It copies only
in play mode, with the overlay over the game's window and shown or hidden
by a panel alone (its `panels: idle` lines below). The measurements
behind the table are in [`quest-tracker-research.md`](quest-tracker-research.md)
§ "Live 12: the game panels' footprints".

**The panel table.** Each entry is one panel frame: an id, `opens_for`
(the panels whose windows it stands for), a footprint (the part of the
client area the panel covers, as fractions, a dimmed area included), its
probes and `min_match`. The shipped table (`qt-screen`'s
`shipped_panels()`), measured at 2560×1440:

| Entry | `opens_for` | Footprint | Probes | `min_match` |
|---|---|---|---|---|
| `inventory` | `inventory`, `stash`, `merchant` | x 0.65 to 1.0, y 0 to 0.845 (the inventory's panel) | 6, on its header plate | 3 |
| `passive-tree` | `passive-tree` | the whole client | 5, on its title plate and its right frame | 3 |
| `key-m` | `key-m` (M: the shop) | the whole client | 7, on its left column and top right | 3 |
| `key-o` | `key-o` (O: the options) | x 0 to 0.345, y 0 to 0.8 | 5, on its header and its foot | 3 |
| `key-u` | `key-u` (U: the world map) | the whole client | 6, on its title plate and its frame | 3 |

The stash's and a merchant's windows open the inventory beside their own
window on the left, so the inventory's entry stands for them through
`opens_for`; their own left-hand windows have no entry (Known
limitations). The Escape menu, the character panel (C), the map overlay
(Tab) and any other panel have none and never hide the overlay. The Escape
menu does not hide the overlay. Live 12 measured it and the first table
had an entry for it, which Live 13 saw hide the overlay; the owner then
chose, on 2026-10-27, to keep the tracker shown while the game's menu is
open, and the entry left the table. Its measured footprint (the whole
client, which it dims) and its five probes on the top of the menu box stay
as data, under `not_shipped` in the fixture
(`fixtures/panels/shipped_panels.json`) and in the research note's Live 12.

**The probe test.** A probe is one patch of a 160 × 90 grid of the client
area (16 px square at 2560 wide), with an expected mean blue, green and
red, a tolerance per channel and a largest luma spread. A copy of the
patch matches when each channel's mean is within its tolerance and its
luma spread is at most the largest. While the overlay is visible, a probe
that touches the overlay's rect is not read (the copy would hold the
overlay); while it is hidden every probe is read. Per read, an entry is:

- *Open*: at least `min_match` probes read, and at least `min_match` of
  them match;
- *Closed*: at least `min_match` read, fewer matching;
- *Unseen*: fewer than `min_match` read;
- *Black*: every probe read came back black (the capture got nothing). A
  black copy among others simply does not match.

Unseen and Black reads change nothing. Only entries whose footprint
intersects the overlay's rect are judged; the others are not copied.

A probe whose largest luma spread is above 127.5 tests colour only: the
luma of an 8-bit copy lies in 0 to 255, so its spread can never exceed
127.5 and the spread limit never fails. The shipped table has one such
probe, key-o's at cell 27, 67 (largest spread 174); its best-read counts
always show it within spread, and its colour tolerance alone decides
whether it matches.

**When it reads.** Only in play mode, while the overlay is shown or hidden
by a panel alone, with the game's pid confirmed, its window found and not
minimized. Otherwise it idles, looking again at most every 100 ms, and
writes `panels: idle (<reason>)`, the first that held of `edit mode`,
`overlay hidden` (no character in an area yet, or another program in
front), `pid unconfirmed`, `no game window` and `minimized`, again only
when the reason changes. Reading, it ticks at most every 250 ms: the
game's window by pid, its shape (16:9 only), the overlay's own rect, the
entries under it, one copy of each readable probe of those entries, the
test and the trackers. A tick that overruns skips the ones it missed.

**The hide and the show.** An entry opens after 1 Open read and closes
after 2 Closed reads in a row (`PANEL_OPEN_READS`, `PANEL_CLOSED_READS`),
so the overlay hides at the first read that sees the panel and shows
again about half a second after it closes. The worker applies the hide as
soon as the watcher's message arrives. The order of the hides is: no
character in an area yet, then another program's window in front, then a
game panel, else shown; edit mode is always shown and the watcher idles
in it. **It fails open:** idle, paused or stopped, the watcher forgets
what it read and tells the worker no panel is open, so it never keeps the
overlay hidden. The screen reader needs nothing new: it already treats a
hidden overlay as covering nothing.

**Status lines,** its own source, after the screen reader's (Reading the
screen › Status lines):

- "Hiding under game panels is paused: the overlay cannot find its own
  window." Without the overlay's rect it cannot tell which panels are
  under it.
- "Hiding under game panels is paused while the game's window is not 16:9
  (<w>x<h>)."
- "Hiding under game panels cannot copy the game's window (the capture
  failed 10 times in a row)."
- "Hiding under game panels stopped after an error; turn it off and on to
  retry." The cause goes on an `error: panel watcher: ` line.

**Diagnostic lines.** Every watcher line is `panels: ` and one of:

- `on`, `off`, `resumed`, `stopped after an error`;
- `idle (<reason>)`, the reasons above;
- `paused (<reason>)`: `paused (no overlay window)`, `paused (the game
  window is not 16:9 (<w>x<h>))` or `paused (the capture failed 10 times in
  a row)`, each with its status line; `resumed` when it ends;
- `overlay at x <a> to <b>, y <c> to <d>`: the overlay's rect as fractions
  of the game's client rect, two decimals, written when an edge moves by
  0.01 or more;
- `<entry> open`, `<entry> closed`, and `<entry> unseen under the overlay`
  (once per stretch: the overlay over too many of its probes);
- `minute: <n> reads, <n> copies, <n> open, <n> black copies, busy <n>
  ms`, once a minute with reads in it;
- `<entry> best read: <r> of <n> probes read, <c> in colour, <s> within
  spread, <m> match, <k> needed`, right after each `minute:` line, one per
  entry read in that minute, in table order. `<n>` is the entry's probe
  count and `<k>` its `min_match`; the others count, for the minute's
  best read of the entry (the most matching, then the most in colour,
  then the most within spread, then the most read), the probes read,
  those whose colour was within tolerance, those within the luma spread,
  and those that did both.

When the watcher stops with reads not yet in a `minute:` line, it writes
that part minute's lines first. With the panel hide, the visibility line
is `visibility: hidden (a game panel is open under the overlay)`. No
line, status or error carries a colour, a pixel value or a probe's
numbers.

**Reading a best-read line.** For an entry whose panel was open under the
overlay while the overlay stayed shown (no `panels: <entry> open` line):

- `<r>` below `<k>`: the probes it needs were not read (the overlay over
  them, or outside the client), as the `unseen under the overlay` line
  says. The fix is the probes' places, not a colour number.
- `<r>` at least `<k>`, `<c>` below `<k>`: a colour miss, the copies'
  means outside the tolerance. The fix is the expected colour or the
  tolerance, from a raw grab of the open panel (`quest-tracker-replay
  panels probe`).
- `<c>` at least `<k>`, `<s>` below `<k>`: a spread miss, the raw copy's
  spread above the largest luma spread tuned on JPEG frames. The fix is
  that limit, from the grab's spreads.
- `<c>` and `<s>` at least `<k>`, `<m>` below `<k>`: the probes in colour
  are not the ones within spread. The fix is the probe choice, or both
  limits.
- `<m>` at least `<k>` and still no `open` line: not a table number. Look
  at the minute line's `black copies` and at `idle` or `paused` lines in
  the same stretch.

The best read is the minute's, so in a minute that also held closed reads
it may be one of those; the order puts an open read first whenever it
matches or colours more probes.

**Where the numbers come from** (Live 12, a no-kill session in a town,
2026-10-27): each panel opened and closed twice over two backgrounds, and
`quest-tracker-replay panels measure` over the capture. Each footprint is
the panel's own extent as its frames show it. Each probe's tolerance is
the larger of 3 × its spread and 6 levels per channel, never more than half
its distance to the closed frames; its largest luma spread is 2 × its
measured spread + 8, at least 16; no probe's colour less its largest
tolerance is darker than luma 16, so a black copy never matches; and at
least 3 probes of each entry lie outside the default
overlay's reach (x 0.78 to 1.0, y 0.30 to 0.97). Every number comes from
quality-90 JPEG frames, while the watcher compares raw copies: those
margins are this version's allowance for the difference, which Live 13
measured on the inventory's probes: under a level per channel, the
spreads within 1.4% (Known limitations). Over Live 12's capture every window of the
shipped panels reads open from its first frame, the character panel's
windows read nothing open, and nothing reads open outside the windows
but three stretches where the inventory was open before its press. Over
Live 8's capture, a session the table was not chosen on, the inventory
and the passive tree are found (and the Escape menu was, by the entry it
had before it left the table) and its 464 foreground play frames read
nothing open.

**Shutdown.** The watcher stops with the others, by the same deadline;
when it ran, the `stop:` line names the panel watcher among the threads.

## Privacy

What is read:

- `Client.txt`, read-only, in short opens with full sharing, never held,
  locked, moved or truncated. Nothing else of the game's is opened.
- With *Read the screen* on, and only then: up to three regions of the
  game's window, copied from the screen as a screenshot would copy them
  (the boss bar's band, the lower-left list of voiced lines and the
  dialogue window, "How detection works" › "Reading the screen"). They are
  copied only while the game's window is in front and a quest step the
  screen can finish is open in the current area, and read by Windows' own
  OCR on this machine. The copies and the text read from them stay in the
  reader's memory for at most a few seconds. Nothing read is written to a
  file or the diagnostic log, shown in the UI, or put in an error: what
  leaves the reader is the kind of event, a boss or NPC name from the quest
  data, times, counts and region names. With the setting off, nothing is
  copied.
- Whenever the overlay is in play mode over the game (hiding under game
  panels is always on since 0.1.1, with no setting): small patches of the
  game's window (16 px squares at 2560 wide, at most 34 per read),
  copied the same way, only while the overlay is shown or hidden by a
  panel ("How detection works" › "Hiding under game panels"). In edit
  mode, or while the overlay is hidden for another reason, nothing is
  copied. A copy lives only for the read that took it, and its colour
  reaches nothing but the probe test: only counts, entry ids, reasons and
  the overlay's place as fractions are logged, and no pixel or colour is
  written, shown or put in an error.
- `quest-tracker-replay panels grab` is a developer's research command,
  not part of the overlay: it copies the game's window once and writes
  that frame only outside every git checkout, refusing any folder inside
  one. The overlay never saves a frame.

What is stored, and where:

- The store folder is `QUEST_TRACKER_DIR` when that environment variable is
  set and non-empty, else `%LOCALAPPDATA%\ExilePact2\quest-tracker\`. It is
  refused (progress is then not saved, and the status line says so) when it
  is inside the game's install or a `Grinding Gear Games` folder, or inside
  a git checkout.
- `settings.json`; `position.json` (the log's path, fingerprint and read
  offset); `characters.json` (your character names with class, level, own
  or unconfirmed, pinned); `characters/<name>.json` (one character's quest
  evidence, overrides and ticks, and the seed of the Riverbank instance it
  was last in, a number the game generated); `new-character.json` (only while a new
  character from The Riverbank has no name yet: its progress, its event
  count, the pin it released, the seed of its Riverbank instance and
  whether it is open; it has no name of its own, and it is removed once a level-up,
  a relog or your pin closes it); `overlay.log`, the diagnostic log, rotated
  at 1 MB into `overlay.log.1`. JSON files are written atomically (a
  temporary file, synced, renamed over the old one); one that cannot be read
  back is renamed `<file>.corrupt-<timestamp>` and the status line says so.
- Only your own and unconfirmed-active character names are ever written
  (the pin `new-character.json` keeps is one of yours), and only to the
  JSON files above: `overlay.log` writes no character name, yours
  included, only a token per character that changes every run ("What the
  diagnostic log says").
  Names seen in join or leave lines (party members) live in memory for one
  session and are never written to disk or to the diagnostic log.
- Chat is ignored: chat lines are dropped by the parser, and no chat text,
  other player's name or account name reaches the store, the diagnostic log,
  the replay CLI's output or the UI.
- A step the screen ticked is stored as any fired signal is, by its signal
  id (`step/<step id>/<i>`) in the character's evidence; no frame and no
  OCR text is ever stored.

Network:

- The app makes one kind of network request, the update check, and only
  when you order it: the tray's *Check for updates*, the Settings window's
  *Check now*, or *Automatically check for updates* turned on (off by
  default; then a check at launch, once a day, and an hour after a failed
  automatic check). A default install, with the setting off and no check
  clicked, sends and receives nothing.
- Measured in the built app on 2026-10-27: version 0.1.0 ran with a store
  folder of its own while a sampler listed, every 200 ms, the TCP
  connections of `quest-tracker.exe` and of every process it started.
  With the defaults (the setting off, no check clicked),
  `quest-tracker.exe` itself opened no connection in the five minutes
  from launch to turning the setting on, nor in the minute and a half
  after a relaunch with the setting turned off again. The same sampler
  saw each check's own connection: the automatic check's, to
  `api.github.com` on port 443, within a second of its `updates: check
  started` line, then the manual check's and the next launch's automatic
  check's. So the quiet stretches are a measured silence, at the
  sampler's 200 ms resolution, not a blind instrument. The WebView2
  processes that draw the windows did connect, with the setting off as
  well: that is Microsoft's runtime, not the app's code ("Known
  limitations").
- A check is one `GET
  https://api.github.com/repos/S-Borkowski/ExilePact2/releases?per_page=30`,
  port 443 only, made from Rust through Windows' own WinHTTP, with three
  headers: `Accept: application/vnd.github+json`, `X-GitHub-Api-Version:
  2022-11-28` and `User-Agent: ExilePact2-quest-tracker` (no version, no
  machine detail). No cookie is sent (each check opens a WinHTTP session
  of its own, with cookies off, and closes it), no credential is sent, no
  redirect is followed, and the proxy is the system's. GitHub sees your
  IP address and those headers; nothing else leaves. The answer is read
  up to 2 MiB within 20 s, used only to find the newest release, and
  never stored or logged. Nothing is downloaded or installed: a newer
  version is only announced, and its release page opens in your default
  browser when you choose it.
- The rest of the app stays offline. Its content security policy allows
  only the bundled page and Tauri's own IPC (`default-src 'self'`, no
  remote script, style, font, image or connection source), its
  capabilities grant only core event and window permissions (the
  overlay's `capabilities/default.json` grants four: `core:event:default`,
  `core:window:default`, `core:window:allow-start-dragging` and, since
  0.1.1, `core:window:allow-start-resize-dragging`, for the edit-mode
  resize handles; the Settings window's `capabilities/settings.json`
  grants `core:event:default` alone), and no Rust source outside `qt-net`, the transport crate, uses
  HTTP or socket code; `tests.test_quest_tracker_network` checks all of
  that from the files. Its name checks cover std's and tokio's sockets,
  Winsock through the `windows` crates, WinHTTP, WinINet, urlmon's
  `URLDownloadToFile`, the HTTP client crates and a webview on a remote
  page (`WebviewUrl::External`); its UI check reads every root `*.html`
  entry page (the overlay's and the Settings window's) beside `src/`. It
  also reads the compiled dependency graph (`cargo tree -i`, skipped when
  cargo is not installed), because `Cargo.lock` already lists HTTP crates
  Tauri can link but does not compile today: no HTTP client, socket, TLS
  or WebSocket crate is compiled into the app, except `socket2` through
  tokio, Tauri's async runtime. Fonts and icons are bundled. The poe2db link and
  the release page open in your default browser, through the opener
  plugin, from Rust. The screen reader's OCR is Windows' own, run on this
  machine.
- The global shortcut is a system hotkey registration: no keyboard hook, and
  nothing is sent to the game.
- Apart from the update check, the only network code in this feature is
  the developer-only poe2db tool, which you run yourself.

## Quest data

`apps/quest-tracker/data/quests.json` is compiled into both executables
(`--data` overrides it). This version: schema 1, checked against client
`4.5.3`, poe2db fetched 2026-10-25, 87 campaign quests over Acts 1 to 6, 98
areas, 39 allowlisted speakers. The validator in `qt-engine` fails, naming
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

**The level-up signal (`level_up`, since 0.1.1).** A log signal that fires
on a level-up line, not on the character's level:

```json
{"kind": "level_up", "level": 2, "area": "G1_1"}
```

Both fields are required: `level`, the new level the line reports, at
least 2 (no level-up reaches level 1), and `area`, a known area code. It
fires when a level-up line to exactly `level` comes while the current
area is `area`, and is kept as evidence of the character the engine takes
the level-up as: the new character from The Riverbank it names, the
pinned character's own level-up, an own or active character's, or an
unknown name made active as unconfirmed (which loses it with the rest of
its state if a join or leave line revokes it). A party member's level-up,
or another name's while a character is pinned, fires nothing. With the
area unknown it does not fire and is refused and counted, as every
area-tied log signal is. Unlike `level` (`{"kind": "level", "min": n}`,
read from the character's level whenever a status is computed, in any
area), it is never read from state, so a character already past `level`
never fires it. One step uses it: "Slay the Bloated Miller", the level-up
to 2 in The Riverbank ("How detection works" › "Slay the Bloated Miller").

**Screen signals.** A signal of kind `screen` is what *Read the screen*
can tick ("How detection works" › "Reading the screen"). It sits on a step
only (anywhere else fails validation), needs `what` and `area`, and names
a `boss` or an `npc` by its `what`:

- `bar_gone`, with a `boss`: the boss's name left the boss bar's band;
- `quest_complete`, with an `npc`: the QUEST COMPLETE banner showed in the
  NPC's dialogue window;
- `kill_remark`, with a `boss`: a new line in the list of voiced lines
  carried one of the boss's kill-remark texts from `remarks` (below); a
  `kill_remark` whose boss has no text there fails validation.

```json
{"kind": "screen", "what": "bar_gone", "boss": "Beira of the Rotten Pack", "area": "G1_2"}
```

The fire rule: a screen event fires a screen signal when the current area
is known and equals the signal's `area`, the signal sits on an open step
(any step after the furthest one reached), and the quest's tracked status
is available or in progress; at most one quest fires per event, the first
in data order. An unknown area refuses the event and counts it. A log line
never fires a screen signal, and a screen event fires nothing else. This
version ships five `bar_gone` and `quest_complete` signals: `bar_gone` on
Reaching Clearfell's "Slay the Bloated Miller" (`G1_1`, his bar's going at
the kill read in a replay of Live 8's capture) and on the "Defeat …" step
of Beira of the Rotten Pack (`G1_2`, the bar text measured in Live 6),
Blackjaw (`G3_6_1`) and Lythara (`P3_2`), whose bar text is not measured
and whose short name matches the full title by containment; and
`quest_complete` for Renly in `G1_town` on Reaching Clearfell's "Take
Renly's reward" (the banner seen in Live 8). Beside them, 21
`kill_remark` signals, one on each slaying step of the nineteen bosses
whose kill remark the `remarks` table holds (below; Asinia and Draven on
two steps each), each new one appended after the signals its step had, since
saved progress names a step's signals by position. The Miller's step
lists Mortimer's line, his `bar_gone`, his `kill_remark` and then his
`level_up`, in that order.

**The `remarks` table.** An optional top-level array of a class's
kill-remark texts, each `{"class", "boss", "key", "text", "source_url",
"fetched"}`: `key` is the poe2db class page's name for the moment that
plays the remark, `text` the short spoken line, stored verbatim, and
`source_url` and `fetched` are validated as a quest's are (a
`https://poe2db.tw/us/<page>` URL and a date). An entry's `boss` must be
the boss of a shipped `bar_gone` signal or a name in `npcs`. A remark
reaches the reader through a `kill_remark` signal of its boss; the reader
looks for every class's texts for that boss, so a row needs no code.

This version's table holds 142 texts, the kill remarks of nineteen bosses
for the eight playable classes (Warrior, Mercenary, Ranger, Huntress,
Witch, Sorceress, Monk and Druid, the classes with ascendancies on
poe2db's Ascendancy page, read 2026-10-28). The Warrior's and the
Mercenary's pages were fetched on 2026-10-27 and the other six on
2026-10-28, with `poe2db fetch-classes <class ...>`; `poe2db remarks`
prints a draft of the class pages' kill rows whose boss the data names,
never a whole table. Texts are stored as plain text: markup removed,
whitespace collapsed, trimmed. A page's row was mapped to a step of the
data only on evidence: the row's key names the boss or the step's area,
or a class's line names one, *and* the line's audio file is from the
step's act (`_a<n>_` for act n, `_i<n>_` or `_p<n>_` for the interlude
areas `P<n>_*`). Only a step whose signals already carry an area
qualifies, since a signal needs one. Two bosses on one step (Ekbab and
Iktab, Zolin and Zelina) take one label, the step's own npc.

| Boss | Quest › step | Area | Key | Classes |
|---|---|---|---|---|
| The Bloated Miller | reaching-clearfell › step-2 | `G1_1` | `MillerKillComment` | all eight |
| Blackjaw | blackjaw-the-remnant › fight | `G3_6_1` | `BlackjawKilled` | all but Warrior and Ranger |
| Lythara | lythara-the-wayward-spear › fight | `P3_2` | `KriarViperDefeated` | all eight |
| The Rust King | secrets-in-the-dark › step-19 | `G1_5` | `RedValeBossSlainComment` | all eight |
| Asinia, the Praetor's Consort | sorrow-among-stones › step-4, step-19 | `G1_9` | `WifeSlainComment` | all eight |
| Draven, the Eternal Praetor | sorrow-among-stones › step-9, step-14 | `G1_8` | `HusbandSlainComment` | all eight |
| The Executioner | the-trail-of-corruption › step-5 | `G1_13_2` | `ExecutionerSlainComment` | all eight |
| Geonor, the Putrid Wolf | the-mad-wolf-of-ogham › step-5 | `G1_15` | `IronCountKilledComment` | all eight |
| Kabala, Constrictor Queen | kabala-constrictor-queen › fight | `G2_4_1` | `KethKabalaBossSlain` | all eight |
| Iktab, the Deathlord | a-theft-of-ivory › step-4 | `G2_5_2` | `BonePitsBothBossesSlain` | all eight |
| Ignagduk, the Bog Witch | tribal-vengeance › step-3 | `G3_7` | `IgnagdukKilled` | all but Ranger |
| Queen of Filth | legacy-of-the-vaal › step-36 | `G3_11` | `AggoratOvergrownQueenOfFilthKilled` | all eight |
| Ketzuli, High Priest of the Sun | legacy-of-the-vaal › step-42 | `G3_12` | `SunWorshipperKilled` | all eight |
| Viper Napuatzi | legacy-of-the-vaal › step-50 | `G3_14` | `ViperNapuatziKilled` | all but Huntress |
| Diamora, the Song of Death | whakapanu-island › step-4 | `G4_3_2` | `SingingCavernsSirenKilled` | all eight |
| Yama The White | trial-of-the-ancestors › step-12 | `G4_4_2` | `HallsOfDeadYamaDefeated` | all eight |
| Captain Hartlin | dark-mists › step-8 | `G4_2_2` | `CaptainHartlinKilled` | Huntress and Monk |
| Thane Wulfric | recruit-the-ezomytes › step-25 | `P1_6` | `EstateTwinBossDefeated` | all eight |
| Zolin, Blood Priest | recruit-the-vaal › step-18 | `P3_7` | `BloodTwinsBossDefeated` | all eight |

The Miller's rows were the replay's control before 0.1.1
(`quest-tracker-replay screen --class Warrior` over Live 8's captures);
since 0.1.1 his step reads them. Not established, so nothing is added
(Known limitations): Blackjaw for the Warrior and the Ranger (their
`RavineBlackjawBossDefeated` lines are the Interlude 3 Etched Ravine
fight, another fight than the data's Act 3 one), Ignagduk for the Ranger
(no row), Viper Napuatzi for the Huntress (her line is three letters, too
short to tell from other speech; a row under 12 letters is never added),
Captain Hartlin for the other six classes (their
`TempleSandHartlinBossDefeated` is an interlude fight), Beira of the
Rotten Pack (no row on any page), the bosses whose step no key ties to by
the rule above (The King in the Mists, Azarian, Mektul, Benedictus,
Tavakai, Krutog, The Prisoner, Siora, Elzarah, Azmadi, Stormgore, Oswin,
and Heldra and Isolde), and every boss step with no area in the data.
The Marauder, Duelist, Shadow and Templar pages are not playable classes
and are not used.

**Find-it tips.** A step may carry a `"tip"`: a short "how to find it"
hint, shown on its own line under the step in the row (when it is the step
the row shows, a skipped row included) and under the step's text in the
quest detail. The tips are the owner's own words; poe2db's `extract` never
writes one, and the file is curated by hand, so a rebuild keeps them. To
add one, find the quest by its `id`, then the step by its `title`, and add
a field after `text`:

```json
{
  "id": "step-2",
  "title": "Slay the Bloated Miller",
  "text": "...",
  "tip": "Follow up river.",
  "signals": [...]
}
```

A tip is one line, not empty after trimming, and at most 120 characters;
anything else fails validation naming the quest, the step and the field
(`qt-engine`'s tests validate the shipped file; a `--data` file that fails
falls back to the built-in data, as for any invalid file). The shipped
file is compiled in, so rebuild after the edit, or pass the edited file
with `--data` to see it at once. This version carries five: Slay the Bloated Miller, Find the Mud
Burrow, Find Beira of the Rotten Pack, Travel to the Grelwood and Search
the Grelwood.

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
| `crates/qt-engine/` | Quest data contract and validator (`data`), the per-character state machine with character detection, the new character from The Riverbank and the refused-signal count (`engine`), and the area read back from the log before a tailer's start (`area`: `area_from_log`, `LogArea`, seeded with `Engine::seed_area`). |
| `crates/qt-viewmodel/` | `View` and `RowChange`: what the panel shows and what changed in a row, built in Rust. |
| `crates/qt-screen/` | *Read the screen*'s pure logic: the regions and geometry, the fuzzy match (parity with the research instruments, `fixtures/screen/fuzzy_parity.json`), the dialogue window's heading gate, and the bar, QUEST COMPLETE and kill-remark trackers behind a backend trait. *Hide under game panels*' too: the panel table type, the probe test and its counts, and the panel tracker (`panels`), and the shipped table (`panel_table`, its numbers in `fixtures/panels/shipped_panels.json`). |
| `crates/qt-capture/` | The Windows backend for `qt-screen`: GDI copy of a client-area region and `Windows.Media.Ocr` (en-US) in memory, through the `windows` crate. Also the JPEG decode of a kept capture's frames through WinRT's bitmap decoder (`decode`), and the call that makes a process DPI-aware, for `quest-tracker-replay panels`. |
| `crates/qt-replay/` | The `quest-tracker-replay` CLI (`src/main.rs`), with `screen` (`src/screen.rs`) and `panels` (`src/panels.rs`: `measure`, `detect`, `grab`, `probe`); it depends on `qt-capture` for the decode and the grab. |
| `crates/qt-app/` | Shell logic without Tauri: state lock, worker thread, commands, modes, debounced saves, command line, log discovery, the screen reader thread and its demand (`screen`), and the panel watcher thread (`panels`), and the update check (`updates.rs`: when a check is due, the newest release in GitHub's answer, the Settings window's view and the tray menu, behind an `UpdateTransport` trait; tested in `tests/updates.rs` with a recording fake transport). Tested with a fake window, emitter and clock. |
| `crates/qt-net/` | The app's only network code: one fixed HTTPS `GET` of the repository's GitHub release list (`api.github.com`, port 443) through Windows' WinHTTP, with its bounds and no caller-given URL. Elsewhere than Windows every fetch is refused as unsupported. Its tests talk only to a server they run on `127.0.0.1`. |
| `src-tauri/` | Tauri glue: the `overlay` window (its capability `capabilities/default.json`: core events, core window, `allow-start-dragging` and `allow-start-resize-dragging`), `WS_EX_NOACTIVATE`, click-through, edit mode's resizing and the window's minimum size, the shortcut, single instance, the tray icon and its menu, the Settings window (label `settings`, its capability `capabilities/settings.json`), the update transport over `qt-net`, the close request, the command wrappers. |
| `src/` | The Svelte 5 + TypeScript UI and its `*.test.ts`. The Settings window's page is `settings.html` with its own entry under `src/settings/` (`main.ts`, `SettingsWindow.svelte`, one component per section listed in `sections.ts`, `UpdatesSection.svelte`, its view type `view.ts`, bridge and browser preview `settings.html?state=<state>`). |
| `theme/` | The harvested design: `theme.json`, `tokens.css`, `icons/<id>.svg`, `fonts/`. |
| `data/quests.json` | The shipped quest data. |
| `fixtures/` | Synthetic logs, sessions, minimal quest data and serialised views shared by the Rust and UI tests; every name in them is invented. |

Tests:

```sh
cargo test --manifest-path apps/quest-tracker/Cargo.toml --locked -p qt-engine   # one crate
cargo test --manifest-path apps/quest-tracker/Cargo.toml --locked -p qt-net      # the transport, against 127.0.0.1 only
cargo test --manifest-path apps/quest-tracker/Cargo.toml --locked -p qt-app --test updates  # the update check, fake transport
npm --prefix apps/quest-tracker run build                                        # needed before the workspace build
cargo test --manifest-path apps/quest-tracker/Cargo.toml --locked --workspace    # every crate
npm --prefix apps/quest-tracker test                                             # the UI, in jsdom
py -3 -m unittest tests.test_quest_tracker_poe2db                                # the poe2db tool, no network
py -3 -m unittest tests.test_quest_tracker_network                               # no network door but the update check
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
- `qt:row-change` with a `RowChange`, one per **live** quest transition (a
  quest becomes available, a step is reached, a quest is started by its
  `start` signal (a `step_done` with `step_done` null), a quest is done;
  catch-up sends none): `id` (increasing), `kind`
  (`available`|`step_done`|`quest_done`), `quest_id`, `step_done` (the
  title of the step just completed, or null), `step_next` (the title of the
  step now current, or null) and `text` (one sentence for the screen
  reader). There is no queue, cap or expiry. The row whose `id` is
  `quest_id` plays it (`src/components/QuestRow.svelte`,
  `src/rowchange.ts`): the completed step fades out over 150 ms, then the
  step now current fades in from 0.2 opacity, rising 5 px, over 220 ms,
  ease-out. Every change also highlights the row, motion or not: a tint
  of the row's new status colour comes on at once, holds 2.5 s
  (`ROW_HIGHLIGHT_MS`) and fades out by opacity over 220 ms; in the
  collapsed badge the same highlight tints the badge. A second change to
  the same row restarts the transition and the hold. A
  row not on screen (its act collapsed, or filtered out) does not play it.
  `src/App.svelte` announces every change's `text` in a polite live
  region, shown row or not.

Commands, UI to Rust: `get_view`, `set_mode`, `set_view_kind`,
`set_collapsed`, `toggle_act`, `open_detail`, `open_frame`, `quest_action`
(`mark_done`|`unmark_done`|`skip`|`unskip`|`reset`), `tick_step`,
`select_character`, `pin_character`, `confirm_character`, `update_settings`,
`open_source`. The Settings window has its own: it is sent
`qt:app-settings` with an `AppSettingsView` (the running version, the
`check_updates` setting and the update state, text and newer version;
serialised in `fixtures/app-settings/`), and calls `get_app_settings`,
`check_for_updates` (starts a check and answers at once),
`open_release_page` and `update_settings` (for `check_updates`). Field names in events are snake_case exactly as `serde`
writes them; command argument keys are camelCase on the JavaScript side
(`questId`, `stepId`), Tauri's default. `src/ipc.ts` mirrors the Rust types,
and `src/ipc.test.ts` holds every file in `fixtures/viewmodels/` against
them, so a field added, dropped or renamed on one side fails a test.

The view fixtures are written by `qt-viewmodel`'s tests; after a deliberate
change to the types, regenerate and review the diff:

```sh
QT_UPDATE_FIXTURES=1 cargo test --manifest-path apps/quest-tracker/Cargo.toml -p qt-viewmodel --locked
```

The Settings window's fixtures, one `AppSettingsView` per update state in
`fixtures/app-settings/`, are written the same way by `qt-app`'s `updates`
test, and `src/settings/settings-window.test.ts` renders each one:

```sh
QT_UPDATE_FIXTURES=1 cargo test --manifest-path apps/quest-tracker/Cargo.toml -p qt-app --locked --test updates
```

`fixtures/updates/releases.json` is the release list that test feeds the
update check: written by hand in the shape of GitHub's answer, never
fetched (its `README.md` says which entry takes which path).

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
package is a decision, not a side effect of a change. A Windows API
feature is a dependency too: the `Win32_Networking_WinHttp` feature of
`windows-sys` is turned on in `crates/qt-net/Cargo.toml` alone, never in
the workspace's list, so no other member compiles WinHTTP. The network
guard (`tests/test_quest_tracker_network.py`) depends on that: it allows
WinHTTP names only in `qt-net`'s sources, and a network crate in a
manifest or in the compiled graph fails it, so turning on a network
feature (Tauri's HTTP client, Winsock) is a decision for the guard too.

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

Cut corners clip an element's own `box-shadow`, so the panel and the badge
take their shadow from `filter: var(--drop-panel)` on a wrapper.
`src/theme.ts` derives it from the harvested `shadow.panel` at start-up
(`applyDerivedTokens`). Motion (a row's transition, the countdown bar,
control transitions) stops under `prefers-reduced-motion: reduce`
(Windows' "Animation effects" off): a row's step change is then instant,
and still highlighted (the highlight is colour and opacity only, with no
movement) and announced. The transition's design check is
[`design/review-row-transition.md`](design/review-row-transition.md), and
the Settings window's is
[`design/review-settings-window.md`](design/review-settings-window.md).
`src/styled.test.ts` checks that every contract token and icon id is
harvested, that every custom property the UI reads has a plain default and
comes from the harvest (no token invented in code), and that fonts and
assets load only from the bundle.

To preview a frame in a browser, run `npm --prefix apps/quest-tracker run
dev` and open `http://127.0.0.1:1420/?frame=<frame id>&mode=play|edit`
(`&scale=2` for the 2× zoom); see `src/preview.ts`.

**Cutting a release.** A release is the annotated tag
`quest-tracker-v<version>` on the tip of `main`. The version is the one in
`package.json`, `package-lock.json`, `src-tauri/tauri.conf.json`, the
workspace's `Cargo.toml` and `Cargo.lock`. These must agree, and "Release
notes" below needs a `### <version>` section for it:

1. Bump the version in those files (build once so `Cargo.lock` follows),
   and write the version's release notes. Land both on `main` through a
   pull request.
2. On an up-to-date `main`:

   ```sh
   py -3 -m tools.quest_tracker.release check            # every version agrees; the notes exist
   py -3 -m tools.quest_tracker.release notes 0.1.0      # the release body, as it will read
   py -3 -m tools.quest_tracker.release cut --push       # tag HEAD and push the tag
   ```

   `cut` refuses uncommitted changes to tracked files, a HEAD that is not
   the tip of `origin/main`, and a tag that already exists here or on the
   remote. Without `--push` it only creates the tag and prints the push
   command.
3. The pushed tag starts `.github/workflows/quest-tracker-release.yml`. It
   checks the tag against the files, builds with Rust 1.98.1 and Node
   24.19.0 on a Windows runner, and creates a **draft** release. The draft
   holds `quest-tracker.exe`, `quest-tracker-replay.exe`, `SHA256SUMS.txt`
   and the body from step 2.
4. Open the draft on GitHub, check it, publish it, and tick "Set as the
   latest release". The workflow never publishes.

To rebuild a tag, for a failed run or a lost asset, run the workflow by
hand (Actions › Quest Tracker Release › Run workflow) with the tag. If the
release already exists, only its assets are replaced.

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
  character in the picker when this matters. A brand-new character is the
  exception: its entry to The Riverbank (an area of level 1) starts it as
  soon as the 5 s relog window after the entry closes without a relog
  (How detection works). The entry line alone cannot tell a new character
  from one re-entering The Riverbank; its seed can. A stored character
  relogging into the Riverbank instance it was last in is recognised by
  that seed at once, and otherwise when the client's character build line
  names it within 5 s of the entry. Both rest on one player's log: every
  one of its 6 relogs came back to the same seed, and no new instance was a
  stored character's return; the build line followed only 3 of those
  relogs, all by one character, so it is not a general signal. What is
  left: a stored character returning to The Riverbank in a **fresh
  instance** (its old one closed), or one the tracker stored before it had
  a seed, still starts a "New character" (not observed in the log); its
  next level-up hands the progress back to it by name (and takes over a pin
  the entry released). A character relogging before its first level-up has
  no name stored: there each session that ends parks the new character
  rather than dropping it, and the next entry to the same instance resumes
  it.
- **"Probably done" is an inference.** A row or step with the
  "probably done" mark was finished by a later signal, not its own, so a player who
  skipped that step is shown it done too early. Secrets in the Dark's
  start on entering Clearfell is an inference of the same kind.
- **Character remarks are not read from the log.** Your character's own voiced
  remarks (poe2db lists each class's, keyed by the moment that plays them,
  such as killing a quest boss) were not observed in `Client.txt`: none
  of the Mercenary's or the Warrior's remark texts was found in the whole
  of one player's log, with a speaker or without, while the same search
  found Una's lines; no speaker is named after a class; and the minute
  after a Mercenary killed The Bloated Miller held no remark. So no quest
  moves on a remark from the log; *Read the screen* reads them instead. That is "not observed", not "never
  logged" ([`quest-tracker-research.md`](quest-tracker-research.md) §
  "Character remarks"). The remarks do show on screen, which *Read the
  screen*'s remark rule reads (its limit is below).
- **Shown until the game's process is known.** Play mode hides the
  overlay when another program's window is in front, telling the game's
  window by the process id in the log lines' `Client <pid>` header. Until
  the overlay has read a line with that header (at start it reads back
  the lines before its start position), it does not know the game's id
  and stays shown over every window. An id read only from catch-up or from
  the log before the start position is trusted for 10 s: if no live line
  and no window of that id comes to the front by then, it hides nothing
  until a live line confirms it (`game pid: … not used to hide the
  overlay` in the diagnostic log). So after the game restarts, when the id
  read back is the previous launch's, the overlay is hidden over the new
  game's window for at most 10 s, or until the new client writes its first
  line. The same bound covers the wrong log: an old standalone
  `Client.txt` found before the Steam one, or a `log_path` setting naming
  a log the running game does not write to, hides the overlay for at most
  10 s, and it is then shown over every window, the game's or not, until
  the right log is set. If the window in front cannot be told, it is shown
  too.
- **Shown again at character select.** In play mode the overlay stays
  hidden from the game's start until a character is in an area, so it
  does not cover the login and character-select screens. Going back to
  character select later in the same session leaves it shown there: no
  log line marks character select (not observed), so the tracker cannot
  tell. The next launch of the game hides it again until a character is
  in an area. With no running game writing the log (its pid no longer
  trusted), the overlay shows, as when the game's process is unknown.
- **The area after a start mid-zone.** The read-back for the current area
  covers at most 8 MiB of the log before the start position. That is a
  bound, not a guarantee: a zone's area line is normally well inside it,
  but this was not measured. When the last area line is further back, or a
  session marker follows it, the area stays unknown until the next zone,
  and an area-tied step in the current zone needs a manual tick (the
  diagnostic log names each refused quest).
- **Town talk.** Talking to an NPC in a dialogue window was not observed
  in the log (one measured visit; not proof that it is never logged), so
  the tracker does not rely on it. A talk step is completed by the next
  recorded signal only where the game cannot produce that signal without
  the talk; otherwise it is a manual tick ("How detection works").
- **A boss's defeat is ticked by hand.** No log line marking the defeat of
  Beira of the Rotten Pack is known: over 9 sessions with her lines,
  nothing but chat, window-focus and client lines came between her last
  line and the next area or her reward. Live 4 timed one kill and the log
  held nothing at it: one controlled negative, so still "not observed",
  not "never logged".
  Her "Defeat Beira of the Rotten Pack", and Blackjaw's and Lythara's
  "Defeat …" steps, have no log signal: tick them when the boss falls, let
  the reward line finish the quest, or turn on *Read the screen*, whose
  bar rule ticks them, and since 0.1.1 the remark rule for most classes
  (with the limits below). Without either the row
  keeps showing the defeat step after the boss's lines.
- **Read the screen: walking away ticks early.** The boss bar also goes
  when you walk out of a living boss's range. The reader cannot tell that
  from a kill, so staying out of range for 3 s ticks the defeat step and
  the row shows the next step early; only an area entry or your death
  cancels it (the owner's choice, 2026-10-26). Since 0.1.1 this holds for
  The Bloated Miller too (the owner's choice, 2026-10-28): with *Read the
  screen* on, walking away from him while he lives can tick "Slay the
  Bloated Miller" early, as a replay of Live 8's capture showed twice for
  a first run in which the player walked away. An area entry cancels a
  going whose bar was last seen before it. A death cancels the next going
  of each bar that was up at the death, and the bar must show again
  before a going counts. Whether the bar stays up through the
  death screen is not measured; the rule covers both cases, a bar that
  goes at the death and one that goes on the walk back.
- **Read the screen: panels that hide the bar.** The passive tree and the
  world map draw their own titles over the bar's name; the bar then counts
  as not visible, and a kill while one of them is open is not seen
  (tick it by hand, or let the reward line finish the quest). The
  inventory, the character panel, the Escape menu and the map overlay
  leave the name readable. Live 9 opened the passive tree for about 12.5 s
  during Beira's fight: no going was reported while it was open, her name
  read again once it closed, and the going came at the kill.
- **Read the screen: dark scenes.** A near-black copy of a region is a
  covered read, with its own status line after 10 in a row; it is not
  taken for a failed copy. So a band dark enough to copy black can leave
  a bar's going unseen, as a panel does. Beira's arena was read in a
  session; Blackjaw's and Lythara's arenas are not measured.
- **Read the screen: Blackjaw's and Lythara's bars are not measured.** Only
  Beira's bar text was read in a session. Blackjaw's and Lythara's
  signals use their short names, matched by containment against the full
  bar title, by analogy with hers.
- **Read the screen: kill remarks.** The remark rule ticks a step only for
  a boss and class the quest data's `remarks` table has a text for. Since
  0.1.1 that is nineteen bosses' slaying steps for the eight playable
  classes ("Quest data" › "The `remarks` table"), with these not
  established and so not ticked by a remark: Blackjaw for the Warrior and
  the Ranger, Ignagduk for the Ranger, Viper Napuatzi for the Huntress,
  Captain Hartlin for every class but the Huntress and the Monk, Beira of
  the Rotten Pack for every class, the bosses no poe2db key ties to their
  step (The King in the Mists, Azarian, Mektul, Benedictus, Tavakai,
  Krutog, The Prisoner, Siora, Elzarah, Azmadi, Stormgore, Oswin, and
  Heldra and Isolde), and every boss step with no area in the data. Where
  a boss has a bar signal (Beira, the Miller, Blackjaw, Lythara) the bar
  still ticks it. No boss's remark has been seen ticking a step live: the
  texts are proved by the matcher's tests, and by the Warrior's Miller
  remark in a replay of Live 8's capture. The Miller's remark will rarely
  tick his step live, because his level-up at the kill ticks it first and
  the reader then stops looking.
- **Slay the Bloated Miller: a first level-up before his death.** The
  step ticks on the character's level-up to 2 in The Riverbank, which in
  one player's log came at the Miller's kill every time (17 of 17). A
  character whose first level-up comes in The Riverbank before he dies
  would tick the step early; not observed in that log.
- **Read the screen: one measured setup.** The regions were measured on a
  2560×1440 borderless window, English client, at one in-game UI scale.
  A window that is not 16:9 is refused; other 16:9 resolutions, other UI
  scales and other client languages are untested. The replay settles the
  trackers, but it reads cached OCR of a wider band, while the live crop
  is OCR'd on its own and can split or clip a line differently. On the
  live crop, Live 9 read Beira's bar to her kill (its last read 1.4 s
  before a press made as she died), and Lives 9 to 11 caught Renly's
  QUEST COMPLETE banner once in each session (0.75 s and 0.5 s before a
  press made as it showed, in Lives 10 and 11); the heading strip read
  Renly's name well enough to open the dialogue window's gate (Live 10).
- **Read the screen: OCR time.** Each minute line's `busy` is the time
  the reader spent copying and reading that minute, measured on one
  machine. Waiting in town for Renly's step with no window open, the
  heading gate shut, it was about 1.9 to 2.0 s a minute at 60 reads
  (Live 10; 16.6 s before the gate, Live 9). With Renly's window open it
  rose to 10.2 s (Live 10) and 10.5 s (Live 11) in the minute of the talk.
  During Beira's fight, with only the bar's band read, it was 5.5 to 8.0
  s a minute at 120 reads (Live 9). Other machines are not measured.
- **Read the screen: keep the overlay off the regions.** A region the
  overlay's window lies over is not read, and its status line says so;
  nothing moves the overlay for you. Keep it at the right of the screen,
  where its default place is, clear of the regions: the bar's band spans
  x 0.25 to 0.75 of the game's window at the top, the dialogue window
  x 0.22 to 0.70, the voiced lines x 0 to 0.45 at the lower left.
- **Hide under game panels: confirmed in the game on three panels.** The
  table's numbers come from quality-90 JPEG frames of a research capture
  (Live 12), while the watcher compares raw GDI copies, so the tolerances
  and spread limits carry margins for raw copies. Live 13 saw the overlay
  hide under the inventory, the passive tree and the stash and come back
  once each closed, and a raw copy of the inventory's six patches sat
  within 0.8 levels per channel of the JPEG numbers, its spreads 0.989 to
  1.014 times theirs, far inside those margins. The other panels' raw
  copies were not grabbed, and the shop (M), the options (O), the world
  map (U) and a merchant's window are replayed over JPEG frames, not seen
  in the game.
- **Hide under game panels: copy time.** Each `panels: minute:` line's
  `busy` is the time the watcher spent copying and testing that minute,
  measured on one machine: about 34.4 s at 120 reads (2 a second, about
  29 copies each), whether a panel was open or not, and 29.3 s at 102
  reads in the busiest minute of the panel cases (Live 13). Other
  machines are not measured.
- **Hide under game panels: only the owner's seven game panels.** The
  inventory, the passive tree, M (the shop), O (the options), U (the world
  map), the stash and a merchant's trade window. The Escape menu does not
  hide the overlay. It was measured and once had an entry, but the owner
  chose on 2026-10-27 to keep the tracker shown while the game's menu is
  open. The character panel (C), the map overlay (Tab) and every other
  panel leave the overlay shown too. The stash and a merchant share the inventory's
  entry, found on the inventory they open on the right; their own windows
  on the left have no entry, so an overlay moved to the left of the screen
  does not hide under a merchant's window. Over Live 12's capture the
  options' entry also read open whenever the stash was, the stash's window
  drawing its header where the options panel draws its own, so an overlay
  over the options' place hides under the stash too (a replay reading,
  not seen live).
- **Hide under game panels: a panel whose probes the overlay covers.**
  While the overlay is shown, a probe under its window is not read, and an
  entry with fewer than 3 probes read is unseen: it neither hides nor
  shows the overlay, and the log says `<entry> unseen under the overlay`.
  Each entry keeps at least 3 probes outside the default place's reach
  (x 0.78 to 1.0, y 0.30 to 0.97 of the game's window); an overlay moved
  or scaled over a panel's probes may not hide under that panel.
- **Hide under game panels: a brief show when the game comes back in
  front.** While another program is in front the watcher does not read
  and forgets what it read, so when the game comes back in front with a
  panel open the overlay shows until the watcher's next read, which comes
  within about one 250 ms tick, and then hides again. Live 13 saw it
  once: under 2 s by the log's whole-second times.
- **Hide under game panels: dark spots.** A probe whose copy is black (a
  dark spot of the scene, a loading screen) does not match. When every
  copy of a read is black, the read changes nothing, so a panel opened or
  closed during it is seen only at the next read with colour. Each entry
  keeps one probe that was never black in a closed frame of either
  session, so a dark scene alone does not make a whole read black.
- **Hide under game panels: one measured setup.** The patches were
  measured on a 2560×1440 borderless window, English client, at one
  in-game UI scale. A window that is not 16:9 pauses the watcher with a
  status line; other 16:9 sizes, other UI scales and other client
  languages are untested, and exclusive fullscreen cannot show the
  overlay at all.
- **Una's wait line.** "Ask Una for help" moves on Una's "I'll wait here.
  The old magicks will protect me.", which the log recorded only from
  December 2025 on (older clients' sessions never have it) and only, as far
  as measured, after a talk with her. Where it does not come, the quest
  shows "Ask Una for help" until you enter the Red Vale, which completes
  it.
- **The "Use" step after Blackjaw and Lythara.** Only Beira's reward was
  seen to come on using an item she dropped. That Blackjaw, the Remnant
  and Lythara, the Wayward Spear work the same way is an inference from
  their reward lines coming seconds after the fight, not a measurement, so
  their last step only suggests using any item left where the boss fell.
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
- **The update check reads the 30 most recent releases** of the
  repository, all its tools' together. A Quest Tracker release older than
  the 30 newest is not seen, so when the repository has published more
  than 30 releases since the last Quest Tracker one, a check says no
  release is published. The check uses Windows' own network stack and
  its proxy settings: a network that needs a proxy Windows is not set up
  for, or blocks `api.github.com`, makes it fail with a one-line reason.
  GitHub answers at most 60 unauthenticated requests an hour from one IP
  address (GitHub's documentation, not measured here); past that a check
  says so. It runs on Windows only.
- **WebView2 connects on its own.** The WebView2 processes the app starts
  to draw its windows are Microsoft's runtime, outside the app's code but
  under its name, and they open connections of their own. In the
  measured session ("Privacy" › Network) they held connections on port
  443 from 3 s after launch until the quit, with *Automatically check for
  updates* off as well as on: 5 WebView2 processes over three launches,
  to 5 addresses, all in IPv6 ranges registered to Microsoft
  (`2603:1026:…` and `2620:1ec:33::11`), seen in 1394 sampler lines over
  about ten minutes. Which Microsoft services they are, and what they
  send, was not established. The app sets no WebView2 arguments today; a
  later change could, while the content security policy already keeps the
  page itself from reaching any remote source.
- **Not covered:** endgame and Atlas quests, non-English clients, an
  installer, auto-update (the update check announces a newer version and
  downloads nothing).

## Maintenance triggers

Revisit this feature when:

- **A game patch lands**: run `quest-tracker-replay scan` and `state` over a
  fresh log; update the parser fixtures if a log template changed, and the
  quest data if quests, rewards or area codes moved ("Quest data" above).
- **A patch moves the game's UI** (the boss bar, the list of voiced lines,
  the dialogue window or its QUEST COMPLETE banner) or changes their text:
  capture a fresh session with the research instruments
  ([`outside-instruments.md`](outside-instruments.md)), run
  `quest-tracker-replay screen` over it, and retune the regions and the
  trackers' constants in `qt-screen` until it gives one event per kill or
  banner again and none under the covering panels.
- **A patch moves or redraws a game panel** (one of *Hide under game
  panels*' seven): capture a no-kill session that opens and closes each
  panel with the research instruments, as Live 12 did, run
  `quest-tracker-replay panels measure` over it with its windows file,
  retune the panel table in `qt-screen` (`panel_table.rs` and its fixture)
  until `quest-tracker-replay panels detect` finds each window and nothing
  outside them, and then check it on the raw route: `panels grab` the game
  with each retuned panel open and closed, and `panels probe` each grab.
- **poe2db changes its page structure**: `extract` fails loudly naming the
  slug; fix the extractor and its synthetic fixture under
  `tests/fixtures/quest_tracker/poe2db/`.
- **A Rust type behind `View` or `RowChange` changes**: regenerate the view
  fixtures, update `src/ipc.ts`, and update the brief if a frame, icon or
  status changes.
- **The brief's export contract changes** (icon ids, frame ids, token keys):
  update `theme/`, `qt-viewmodel` and the UI together.
- **A dependency is bumped**: keep `tauri`, its plugins, `@tauri-apps/api`
  and `@tauri-apps/cli` on the same minor line, refresh both lock files, and
  re-check the CSP and capabilities.
- **GitHub changes its API** (the "List releases" endpoint, its fields
  `tag_name`, `draft` and `prerelease`, its headers or `api.github.com`
  itself), **or the repository, the `quest-tracker-v` tag prefix or the
  release flow changes**: update `qt-net` (its host, path and headers),
  `crates/qt-app/src/updates.rs` (the tag prefix, the release page and
  what counts as a release) and `tests/test_quest_tracker_network.py`
  together, and keep them in step with `tools/quest_tracker/release.py`'s
  `TAG_PREFIX` and `REPO_URL`.
- **Someone verifies the Steam path**: update `crates/qt-app/src/discover.rs`
  and this guide.
- **A new display mode or Windows version** behaves differently over the
  game: record it under "Known limitations".

## Release notes

### 0.1.1

- Hiding under game panels is always on: in play mode the overlay hides
  while the inventory, the passive tree, the stash or another of the seven
  game panels is open under it, and shows again once it closes. Its
  *Hide under game panels* toggle is gone from Settings, and a 0.1.0
  settings file that turned it off no longer does.
- Scale zooms the panel's contents and never resizes the window. In edit
  mode the window resizes by dragging the panel's edges and corners, like
  any Windows window, down to 320 by 240; the size you give it is kept
  across restarts (`"width"` and `"height"` in `settings.json`'s
  `"window"`). In play mode the window cannot be resized and stays
  click-through.
- The click-through toggle is gone from Settings. Play mode is still
  click-through, and edit mode still takes clicks.
- Leaving edit mode shows the quest list: Settings, the character picker
  and a quest's detail close on the way back to play mode.
- "Slay the Bloated Miller" ticks at his death instead of 10 to 20 s later
  on Mortimer's line: on the character's first level-up, to 2 in The
  Riverbank, with or without *Read the screen*, and, with it on, also on
  his boss bar going or the character's kill remark (not yet seen live,
  below), for every playable class. Mortimer's line still ticks it when none of those came. With
  *Read the screen* on, walking away from the living Miller can tick it
  early, as for the other bosses with a bar.
- With *Read the screen* on, the character's kill remark ticks the slaying
  step of Blackjaw, Lythara and sixteen more bosses (The Rust King,
  Asinia, Draven, the Executioner, Geonor, Kabala, Iktab, Ignagduk, the
  Queen of Filth, Ketzuli, Viper Napuatzi, Diamora, Yama, Captain Hartlin,
  Thane Wulfric and Zolin), for each class whose remark poe2db's class
  pages give. The bosses and classes with no established remark are in
  Known limitations.
- The kill-remark route is not yet seen live: no boss's remark has ticked
  a step in a live session. The texts are poe2db's, checked against the
  screen for the Warrior's Miller remark only, in a replay of Live 8's
  capture (Known limitations, "Read the screen: kill remarks").
- The quest data gains a `level_up` signal kind: a level-up line to a
  given level in a given area, kept as evidence when it happens, never
  read from the character's level.

### 0.1.0 (first version)

- The overlay, `quest-tracker.exe`: a transparent, always-on-top Tauri
  window with play (click-through) and edit modes, toggled by
  `Ctrl+Alt+Q`; *Next up* and *All* views; quest detail with steps,
  rewards, worth reason and the poe2db link; character picker with confirm
  and pin; settings for opacity, scale, filters, log path and hotkey.
- In play mode the overlay is shown only while the game's window or its
  own is in front, and hidden over any other program (`visibility:`
  lines in the diagnostic log); edit mode always shows it. It stays shown
  until it has read the game's process id from a log line. An id read only
  from an older part of the log hides it for at most 10 s unless the game
  confirms it, so a wrong log (an old standalone `Client.txt`, or a wrong
  log path) leaves the overlay shown over every window instead of hidden
  over the game.
- In play mode the overlay stays hidden from the game's start until a
  character is in an area (the session's first area line), so it does not
  cover the login and character-select screens; edit mode still shows it.
- A tray icon (the app's icon, tooltip "Quest Tracker") with *Check for
  updates*, *Settings* and *Quit Quest Tracker*, which closes the overlay
  cleanly even while play mode has it hidden; closing the overlay's
  window quits the same way. Each writes a `quit:` line, then progress is
  saved and the tracker stops.
- *Check for updates*: the tray's entry, or *Check now* in a new Settings
  window opened from the tray, asks GitHub's API once whether a newer
  Quest Tracker release is published, and announces one in the window and
  the tray (*Update available: X*), with a way to open its release page in
  your browser. Nothing is downloaded or installed. *Automatically check
  for updates* in that window, off by default, checks at launch and once a
  day. With it off and no check clicked, the app's own code makes no
  network request (WebView2, Microsoft's runtime that draws its windows,
  connects on its own: see Known limitations).
  The window also carries the line "Created by ST4H".
- In edit mode the window moves by dragging the panel's header, and the
  position is kept; there is no dock side.
- When a quest becomes available, is started, has a step done or is done
  while you play, its own row changes in place: it is highlighted in its
  new status colour for about 2.5 s, the completed step fades out, then
  the next step fades in from low opacity with a small upward move (about
  150 ms and 220 ms). When the step done is the quest's last one and the
  quest is not done yet, the completed step fades out and no next step
  comes in. A second change restarts it; a row that is not on
  screen does not play it, and the collapsed badge is highlighted the same
  way. With reduced motion (Windows' animation effects off) the step
  change is instant and the highlight still shows. Every change is
  announced to a screen reader in one sentence naming the quest and what
  changed.
- Settings: opacity from 0 to 1 (at 0 only the panel's background goes;
  text, icons, the border and the edit accent stay opaque), scale as an
  Auto switch plus a slider for a fixed factor from 0.75x to 2x, both
  applied while dragged, not only on release; "Show skipped quests", which
  shows skipped quests even with "Only worth-it" on, in Next up as well as
  All; and "Only worth-it quests". Settings files from before load as they
  were: their `1`, `1.33` or `2` scale as that factor, and their dock side
  ignored.
- Skip works on any quest, one not picked up yet included, and on the
  quests of the unnamed "New character", whose overrides pass to the
  character it becomes. A skipped quest stays tracked: the log keeps moving
  it, its row shows the step it has reached, and Unskip shows it where its
  progress is.
- *Read the screen*, a setting off by default: with it on, the overlay
  copies up to three regions of the game's window (the boss bar's band,
  the lower-left list of voiced lines and the dialogue window), only while
  the game is in front and a step the screen can finish is open in the
  current area, and reads them with Windows' own OCR, in memory. Beira of
  the Rotten Pack's defeat step ticks when her bar goes (seen live);
  Blackjaw's and Lythara's are set up the same way by analogy, not yet
  seen. A Mercenary's "Defeat Blackjaw, the Remnant" also ticks on the
  character's kill remark (not yet seen on screen). "Take Renly's reward"
  ticks when the QUEST COMPLETE banner shows in his window. The passive tree or world map over the bar ticks
  nothing; neither does the overlay over a region, a missing English OCR,
  a window that is not 16:9, or the overlay when it cannot find its own
  window (reading then pauses, with the status line "Screen reading is
  paused: the overlay cannot find its own window."), and the status line
  says which. Nothing read is stored, logged or shown.
  `quest-tracker-replay screen` runs the same trackers over a kept
  capture.
- *Hide under game panels*, a setting off by default: with it on, play
  mode hides the overlay while one of seven game panels (the inventory,
  the passive tree, the shop on M, the options on O, the world map on U,
  the stash and a merchant's trade window) is open under it, and shows it
  again once the panel closes, by the colour of a few small patches of
  the game's window where the panel draws its frame. The Escape menu does
  not hide the overlay. It pauses, with a status line, on a window that
  is not 16:9 or when the overlay cannot find its own window, and never
  keeps the overlay hidden while paused. Nothing it copies is stored,
  logged or shown. Measured over a research capture and replayed;
  confirmed in the game on the inventory, the passive tree and the stash
  (Live 13). `quest-tracker-replay panels` measures the panels and runs
  the same test over a kept capture, and its `grab` and `probe` check the
  table on a raw copy.
- A find-it tip under a step: the owner's short "how to find it" hints,
  from a `tip` field in the quest data, shown under the step they help
  with in the row and the quest detail; five in this version, more added
  by hand.
- Play mode never takes focus from the game; edit mode takes keyboard
  focus, so the Settings fields accept typing. A shortcut that cannot be
  registered at launch leaves the overlay in edit mode, with the reason in
  the status line, instead of stuck in click-through play mode.
- The owner's design: Alegreya Sans bundled (no web font), 45° cut corners
  on the panel, buttons and badge, soft shadows inside the
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
  Next up groups its rows under "Worth it", "Optional" and "Not worth it".
- Quest detail with steps (empty box, in progress, done) whose leading mark
  is, in edit mode, the step's one tick-by-hand toggle, except on a step
  shown "probably done", which has none; reward chips, the
  worth reason, the poe2db link, and Mark done / Skip / Reset buttons;
  Reset's confirm shows a 3 s countdown. Settings as label-and-control
  rows with toggles and two sliders. Empty states with a glyph,
  the fix in edit mode, and the hotkey on a keycap in play mode. The
  collapsed badge shows the worth-it count in gold, or a warning when the
  log is in trouble.
- Scale Auto sizes the panel by the monitor: 1x, 1.33x from 1440 px of
  height, 2x from 2160, after Windows' own DPI scale. It is worked out at
  start after the window is placed, when the setting changes, and when the
  window moves to another monitor or its DPI scale changes (Command
  reference).
- Motion is limited to a row's transition, the Reset countdown bar and
  control transitions, and stops under reduced motion
  (`prefers-reduced-motion: reduce`); a row's highlight is colour only and
  stays.
- Progress per character from `Client.txt` alone (plus the screen, with
  *Read the screen* on), resumed across restarts
  from the saved log position, stored under `%LOCALAPPDATA%` (or
  `QUEST_TRACKER_DIR`); on start the current area is read back from the
  log before that position (How detection works).
- A new character is picked up when it enters The Riverbank, with no
  pick: once the 5 s relog window after the entry closes without a relog
  (at most about 7.5 s after the entry on a quiet log), Act 1 progress shows
  under "New character", and its first level-up names it and stores it. A
  pinned character's pin passes to the new one then, not at the entry. A
  named character's relog into The Riverbank is told apart by the seed of
  its Riverbank instance, at the entry, or when the client's character
  build line names a stored character within that window: that character
  and its pin are kept, nothing else changes, and "New character" never
  shows (Known limitations says when it is not told apart). Events held
  from before its entry (another character's, never named this session)
  are dropped and counted, never given to it, so a new character no longer
  starts with an earlier character's Act 2 progress.
- The diagnostic log `overlay.log` never names a character: it writes
  `character #<token>`, a token that changes every run.
- The unnamed new character is saved in `new-character.json`: a relaunch
  of the overlay mid-session shows it again with its rows, a game restart
  parks it (and gives a released pin back), and a later entry to the same
  Riverbank instance resumes it. Events of a new character that another
  one replaced are counted apart from those that came before any
  character, in the status line and the replay summary.
- A quest or step shown done because of a later signal, not its own,
  carries a "probably done" mark (a dashed ring beside the done mark) with
  a tooltip and the same words as accessible text; it is not a control,
  and such a step cannot be ticked by hand.
- Secrets in the Dark shows in progress from Clearfell (a start inferred
  from Reaching Clearfell). Summoning Una in the Grelwood ends "Search the
  Grelwood", her wait line after the talk ends "Ask Una for help", so "Find
  the Red Vale" shows after the talk, not at the summon; her later line
  ends "Use the Rune Spikes". Her Grim Tangle line completes The
  Mysterious Shade's "Speak to Una about the roots", which no longer shows
  in progress on the way through the Grelwood.
- Treacherous Ground shows from Clearfell Encampment, where Renly gives it,
  not only from Clearfell.
- Signals tied to an area that arrive while the area is unknown are
  refused, logged per quest and counted in the status line; only quests
  that could still change for that character count.
- Quest data for 87 campaign quests, Acts 1 to 6, from poe2db.tw (fetched
  2026-10-25), with signals measured against a real log of client 4.5.3:
  32 detected automatically, 42 partly, 13 by manual tick. Talk steps the
  log cannot show are completed by the next signal the game can only
  produce after them where the order is supported, and otherwise stay a
  tick. Beira of the Rotten Pack's steps are find her (her lines), defeat
  her (ticked by hand, or by her bar going with *Read the screen* on: no
  log line marks the defeat as far as measured)
  and "Use the item it dropped", since her reward came when that item was
  used; the reward finishes the quest in any area. Blackjaw, the Remnant
  and Lythara, the Wayward Spear get the same find and defeat steps and end
  with "Use any item left after the fight", a hint, since that they work
  the same way is inferred, not observed.
- Slay the Bloated Miller ticks after his death, on Mortimer's "Well done!
  Please come inside." in The Riverbank, not on the Miller's first line:
  before, the row showed "Enter town" while he was still alive (two live
  sessions). Where Mortimer's line does not come, entering Clearfell
  Encampment still completes the step.
- `quest-tracker-replay` (`events`, `state`, `import`, `scan`, `screen`, `panels`; the summary
  counts held events, the two kinds of dropped event and refused signals,
  and decides a relog window still open at the end of the log as the
  overlay would) and the developer-only `py -3 -m
  tools.quest_tracker.poe2db` (`fetch`, `fetch-classes`, `extract`,
  `remarks`, `coverage`).
  `npm run release` builds both `quest-tracker.exe` and
  `quest-tracker-replay.exe`.
