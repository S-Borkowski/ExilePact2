# Outside-the-game instruments (developer-only)

`tools/quest_tracker/outside/` holds three research instruments and the
tools around them. They answer one question: can an event that the game
does not write to `Client.txt` be seen from outside the game? The first
case is Beira of the Rotten Pack's death cry, a voiced line with no words
(poe2db key `CarrionCroneDeath`). Live 4 timed one kill against the log and
found nothing logged at it
([`quest-tracker-research.md`](quest-tracker-research.md) § "Rewards after
a boss kill"). Live 6 runs these instruments during one more kill.

Nothing here ships to players or runs inside the quest tracker overlay, and
the tracker does not use any of it. The instruments are Python (standard
library only, run as `py -3`) and Windows PowerShell 5.1 scripts, with any
C# compiled through `Add-Type`. They download, install and build nothing.

What the instruments found, and what the client's code was read to do, is
in [`quest-tracker-research.md`](quest-tracker-research.md) § "Seeing a
voiced line from outside the game". This page says how the tools work and
how to run them.

## What never touches the game, and what never leaves the machine

- **The game's process is never opened.** No instrument reads or writes the
  game's memory, attaches a debugger, or calls any of the game's code,
  including the Oodle decompressor built into the client. The game's pid
  only names whose audio and window the instruments watch, and which reads
  to keep from a system-wide trace.
- **`Content.ggpk` is read only while the game is closed**, and then only
  its record and bundle headers (`ggpk walk`), or one named file's bytes
  (`ggpk copy`). Whether the client lets another process read the file
  while it runs was not tried.
- **Every output stays in the live folder**, outside any repository.
  Every command that writes refuses a path inside a git checkout
  (`common.refuse_inside_git`). That covers the traces, their XML
  exports, the read lists, the recordings and clips, the frames and their
  OCR text, the GGPK tree files, any decoded index and any reference audio
  or copied game file. None of them is tracked, not even as a test
  fixture. The tests use synthetic fixtures and temp folders outside the
  repository.
- **Why.** A copy of a game file, or anything decoded from one, is the
  game's own data. Tracking it would put an extracted game asset in the
  repository, which [`AGENTS.md`](../../AGENTS.md) § "Legal" forbids.
  Privacy matters too: a file-I/O trace names every file every process on
  the machine opened, and a screen frame can hold chat and the player's
  name.
- **What gets printed.** Printed output and `analysis.json` carry NPC
  lines, area and boss names, counts, times and GGPK-relative names.
  Chat never leaves the log reader (`clientlog`). A line whose speaker
  starts with a chat prefix, or whose speaker is not an allowlisted NPC,
  is dropped, and character names are masked as `<player>` or
  `<character>`.

## The live folder

`L` is `%USERPROFILE%\quest-tracker-live-5\beira-defeat\`. It is the
default of every Python command's `--live` below. The PowerShell scripts
have no default and are given it (or a file in it) through `-Live`,
`-OutDir`, `-Out` and `-StopFile`; `run start` fills them in for the
recorder, the capture and the marker. The
`store\` folder beside it belongs to `quest-tracker-live3`'s Live 5, and
nothing here touches it. All times are UTC, ISO 8601 with milliseconds and
`Z`.

| Path under `L` | Written by | Holds |
|---|---|---|
| `session.json` | `session prep` | `created_utc`, `client_txt`, `client_txt_offset` (the log's size at prep, where the analysis starts reading), `ggpk`, `ggpk_size`, `ggpk_mtime` |
| `clock.jsonl` | `session prep`, `session clock` | clock anchors, `{"utc", "tick_ms"}` |
| `markers.txt` | `marker.ps1` | `#` header lines (hotkey, pid, tone), then one `<n> <utc>` line per press |
| `stop-i1`, `stop-i2`, `stop-i3`, `stop-marker` | `session stop`, `run stop` | empty files. Each one asks its instrument to end |
| `run.json`, `marker.log` | `run start` | each started instrument's pid and creation time; the marker's output |
| `ggpk\tree-before.json`, `ggpk\tree-after.json` | `ggpk walk` | the GGPK directory before and after the session |
| `ggpk\index.raw`, `ggpk\index.json` | `ggpk copy`, `bundle_index read` | the bundle index as stored, and its decoded records (only once it has been decoded) |
| `i1\status.txt` | `fileio_controller.ps1` | `<utc> <state>` lines |
| `i1\rehearsal\` | the controller and `fileio probe` | `probe.bin`, `probe.json`, `rehearsal.etl`, `rehearsal.xml` |
| `i1\session.etl`, `i1\session.xml` | the controller, `fileio export` | the session trace and its XML |
| `i1\reads.jsonl`, `i1\mapped.jsonl` | `fileio reads`, `fileio map`, `bundle_index annotate` | the game's reads of `Content.ggpk`, then each read's GGPK records and bundle files |
| `i2\game.wav`, `i2\game.wav.json`, `i2\recorder.log` | `loopback.ps1` | the game's audio and its sample-to-UTC sidecar |
| `i2\kill-clip.wav`, `i2\bark-clip.wav` | `analyse` | 5 s either side of the kill press and of one of her lines |
| `i2\rehearsal\` | `audio selftest --out` | the recorder's selftest |
| `i3\frames\top\`, `i3\frames\bottom\`, `i3\frames.jsonl`, `i3\capture.log` | `capture.ps1` | the frames and their index |
| `i3\ocr.jsonl` | `ocr.ps1` through `screen ocr` or `analyse` | the OCR cache |
| `i3\rehearsal\` | `screen rehearse --out` | the capture's rehearsal |
| `ref\` | the person, `audio decode-ref` | reference audio, decoded to PCM, and an optional `refs.json` |
| `tools\<name>\` | the person | each owner-approved download, each in its own new folder |
| `analysis.json` | `analyse` | per check: name, expected, observed, verdict, details |

`session prep` refuses an `L` that holds anything but a `tools` folder,
so each session gets a fresh folder; a `tools` folder alone is kept, since
it holds the owner-approved downloads fetched before the session.

## The clock

A `Client.txt` line carries its local wall-clock second and a millisecond
counter. One comparison (2026-10-26) put the counter at the milliseconds
since Windows started, a 32-bit value that wraps after about 49.7 days. A
clock anchor pairs `GetTickCount64` with the UTC clock, keeping the best of
five samples. `common.log_time_utc` rebuilds a line's full tick count near
its wall-clock second and converts it through the nearest anchor. The line
falls back to its wall-clock second when it has no counter, when there is
no anchor, or when the result lies more than 1.5 s outside that second.
Every line says which source it used. Take an anchor at prep, and again
with `session clock` around the moments that matter.

## Instrument 1: the file-read trace

**What it is.** Windows' own Performance Recorder (`wpr`, profile `FileIO`,
file mode) records the operating system's file-I/O events for every
process. Nothing about it is specific to the game. The game's reads of
`Content.ggpk` are kept by pid and file name, and each read is mapped to the
GGPK records it touched: a raw FMOD bank under `/FMOD/Desktop/`, or a
bundle's 256 KiB blocks.

**What it sees.** Read calls (`ReadFile`, cached or not), and paging reads,
where a mapped page has to come from the disk.

**What it is blind to.** A read of a memory-mapped page that is already in
memory is a soft fault and leaves no file-I/O event. The client was read
statically to map `Content.ggpk` whole and copy a file's bytes out of the
mapping rather than call `ReadFile` (research note, § "How the client loads
its audio"). That reading predicts a trace that sees nothing at the kill.
Such a trace would not show that nothing played. The trace also cannot see
what the client decompresses or keeps in memory. The rehearsal measures
both blind spots before the session relies on the trace. Its first line
fails when the trace misses a cached `ReadFile`, and its second line shows
whether a paging read on a mapped view is seen.

**The admin prompt.** `fileio_controller.ps1` is the only elevated step in
the whole toolset, behind one admin prompt:

```
Start-Process powershell.exe -Verb RunAs -ArgumentList '-NoProfile','-ExecutionPolicy','Bypass','-File','<repo>\tools\quest_tracker\outside\fileio_controller.ps1','-Live','L'
```

| Argument | Default | Meaning |
|---|---|---|
| `-Live` | (required) | the live folder |
| `-MaxMinutes` | 180 | a session trace still running after this is stopped and saved, after an `error session cap` line, so a forgotten trace cannot fill the disk |

It writes each state to `i1\status.txt`, in this order:

1. `started`. If `wpr -status` shows a recording already running, it writes
   `error wpr busy` and exits without touching that recording.
2. `rehearsal-trace-running`: `wpr -start FileIO -filemode -recordtempto L\i1\tmp`.
3. `probe-done`: it runs `py -3 -m tools.quest_tracker.outside.fileio probe --live L`
   from the checkout's root.
4. `rehearsal-stopped`: `wpr -stop L\i1\rehearsal\rehearsal.etl`.
5. `session-trace-running`: the session trace, started the same way.
6. `session-trace-stopped`: once `L\stop-i1` exists, `wpr -stop L\i1\session.etl`.

Any failure writes `error <reason>`, cancels a recording this script started
and exits 1.

**`py -3 -m tools.quest_tracker.outside.fileio`**, unelevated except for
`probe`, which the controller runs:

| Command | Options | What it does |
|---|---|---|
| `probe` | `--live L` | Writes a 64 MiB `probe.bin` unbuffered and write-through, so none of it is in the cache. Reads 64 KiB at offset A (4 MiB) twice with `ReadFile`; the second read comes from the cache. Then it maps the file and touches one byte at offset B (32 MiB). Records its pid and the UTC before each step in `probe.json`. |
| `rehearsal-check` | `--live L` | Exports the rehearsal trace if needed and prints two capture lines. `i1-rehearsal` passes when both reads at A appear with the probe's pid, offset and size. Otherwise it fails and names what is missing: no events, a wrong pid, or only the uncached read (a cache-blind trace). `i1-rehearsal-mapped` passes when a read covering B is attributed to the probe, and is `not-observed` otherwise. |
| `export` | `--live L`, `--which session\|rehearsal` (default `session`), `--force` | `tracerpt <etl> -o <xml> -of XML -lr -y`. A long trace gives gigabytes of XML, and the readers stream it. |
| `reads` | `--live L`, `--pid P` (required), `--suffix` (default `\Content.ggpk`) | Writes `i1\reads.jsonl` (`utc`, `pid`, `pid_source`, `tid`, `op`, `file`, `offset`, `size`, `flags`): the reads by that pid of the file whose NT path ends in the suffix, case-insensitively. Also counts writes to the file, which only the patcher makes. |
| `map` | `--live L`, `--tree F` (default `tree-after.json`, else `tree-before.json`) | Writes `i1\mapped.jsonl`: each read with the GGPK `records` it touches. Prints counts per kind. |

How the parser reads tracerpt's XML was measured on a local trace on
2026-10-26; the `fileio` module's docstring holds the details. Which FileIo
fields tracerpt decodes was not established before the rehearsal. A read
with no decoded `Offset` is read through its binary layout, and the printed
counts say which route each read took. A read's file comes from its
FileObject through the last Create's `OpenPath`, or else from its FileKey
through the name and rundown events. Its pid comes from the event header,
or else from its thread through the thread events, and `pid_source` says
which.

**`py -3 -m tools.quest_tracker.outside.ggpk`**, the GGPK directory, read
with the game closed:

| Command | Options | What it does |
|---|---|---|
| `walk` | `--ggpk G` (default the install's `Content.ggpk`), `--out F` (required) | Reads only the record headers and bundle headers across the whole tree, never a file's contents. Writes a tree file listing every `FILE` (path, record offset and length, data offset and size, a bundle's header), `PDIR` and `FREE` record. |
| `lookup` | `--tree F` (required), `<offset>`, `--size n` (default 1) | Maps a byte range of `Content.ggpk` to the records it touches, as `{name, kind, rel_offset, blocks}`. The kinds are `file`, `bundle` (with the block indexes touched), `bundle-header`, `record-header`, `free` and `unknown`. |
| `diff` | `A B` | Compares two tree files. Exits 1 and lists what moved, changed, was added or removed. |
| `copy` | `--ggpk G`, `--tree F`, `--name <path>`, `--out O` (all but `--ggpk` required) | Copies one named file's data out (`/Bundles2/_.index.bin` into `ggpk\index.raw`, say), after checking that the record at the tree's offset is still that file. |
| `selfcheck` | `--ggpk G` | Walks in memory and writes nothing. Checks that the records the session needs exist, and that `lookup` maps their first and last bytes and blocks back to them. |

**`py -3 -m tools.quest_tracker.outside.bundle_index`**, which says which
game files a bundle block holds:

| Command | Options | What it does |
|---|---|---|
| `read` | `--bin B` (required), `--paths P`, `--out F` (required) | Reads an index that has already been decoded, and its decoded path section, and writes the records as JSON (`L\ggpk\index.json`). It joins paths to files by hash, trying the two hash functions public readers use. When neither matches, the paths stay unknown (`"hash": "none"`) and are never guessed. |
| `annotate` | `--live L`, `--index F`, `--paths P`, `--src` (default `i1\mapped.jsonl`), `--out` (default: rewrite the source) | Adds `files` to each mapped read: for each `bundle` entry, the files whose bytes overlap the blocks read. A bundle the index does not list goes in `files_unmatched`. |
| `files` | `--live L`, `--index F`, `--paths P`, `--bundle NAME` (required), `--block k` (repeatable; default every block) | Lists the files held in one bundle's blocks. |
| `extract-plan` | `--live L`, `--index F`, `--paths P`, `--prefix PATH` (required) | Lists the bundles, merged byte ranges and blocks that hold every path under a prefix: what an extraction would have to decompress. |

`bundle_index` never decompresses anything and calls no code of the game.
The index is stored compressed, so decoding it needs an Oodle decoder.
That decoder is an owner-approved download, kept in its own folder under
`L\tools\` and run by the person, outside these tools. Until a real index
has been decoded, this reader has seen only its synthetic test fixture, and
the field order it assumes is not verified. Without `index.json` the
analysis falls back to every dialogue bundle read near one of her lines.

## Instrument 2: the loopback recorder

**What it is.** WASAPI process loopback (Windows 10 2004 or later)
records the mix of one process tree's audio streams, and nothing else on
the desktop. Windows' audio engine supplies the mix; the recorder never
opens the game's process. No admin is needed.

**What it sees.** Whatever the game itself plays, at the volume the game
mixes it.

**What it is blind to.** Other processes' audio, by design: the marker's
own tone is the check (`i2-isolation`). Anything the game does not play,
including while it is muted. `mute_in_background=true` is set in the
owner's client config, and going by the option's name the game is silent
while it is not focused (not measured). So the analysis leaves out any
window that overlaps a focus-loss stretch in the log, and says so. The
recorded level follows the output volume: the selftest's tone at -20 dBFS
recorded at -33.5 dBFS.

**`loopback.ps1`**:

```
powershell.exe -NoProfile -ExecutionPolicy Bypass -File loopback.ps1 -TargetPid P -Out L\i2\game.wav -StopFile L\stop-i2 [-MaxMinutes 90]
powershell.exe -NoProfile -ExecutionPolicy Bypass -File loopback.ps1 -CompileOnly
```

- **The output** is 48 kHz, 2-channel, 16-bit PCM. Its sample index stays
  linear in UTC. Each packet goes where its QPC timestamp puts it, and a
  gap of more than 10 ms is filled with zeros and logged. Silent packets
  are written as zeros, and while no packet arrives for over a second,
  zeros are written up to half a second ago.
- **The file grows readably.** The header carries placeholder sizes while
  it records and is flushed at least once a second, so the WAV can be read
  while it grows. The sizes are patched at the end.
- **The sidecar** `<Out>.json` holds `pid`, `sample_rate`, `channels`,
  `bits`, `first_sample_utc`, `anchors` (`[sample, utc]` pairs, taken at
  the start, every 10 s and at the end), `gaps`, `end_utc` and
  `end_reason`.
- **Ready and stop.** It prints `ready <utc>` once the first packet
  arrives, or 2 s after the start. It stops on its stop file
  (`stop-file`), when the target exits (`target-exited`), when the device
  is invalidated (`device-invalidated`) or at `-MaxMinutes`
  (`max-minutes`), then prints `stopped <utc> <reason>` and exits 0.
- **Errors.** An activation or format failure, an `-Out` that already
  exists, or a path inside a git checkout prints one `error <reason>` line
  and exits 1.
- `-CompileOnly` only compiles its C#, which Windows PowerShell 5.1 builds
  as C# 5.

**`py -3 -m tools.quest_tracker.outside.audio`**:

| Command | Options | What it does |
|---|---|---|
| `tone` | `--hz F`, `--seconds S` (both required), `--dbfs D`, `--start-after T`, `--log J`, `--go-file G` | Plays a sine with `winsound.PlaySound` from memory, and logs its pid and start UTC to `J`. With `--go-file`, it waits for that file before `--start-after` starts counting. |
| `selftest` | `--out DIR` | The recorder's positive and negative control in one run. Two tone processes, A at 440 Hz and B at 1000 Hz, each -20 dBFS for 3 s, and the recorder on A's tree for 5 s. It passes when 440 Hz is at least 20 dB above 1000 Hz and A's onset is within 150 ms of its logged start. With `--out` it keeps the files and prints the `i2-rehearsal` line. Exit 0 on a pass, 1 otherwise. Measured on this machine (2026-10-26, five runs): B at -129 to -133 dBFS, and A's onset 95 to 115 ms late, most likely `PlaySound`'s own start-up. |
| `clip` | `--wav W`, `--at <utc>`, `--out O` (required), `--before S`, `--after S` (default 5 each) | Cuts `[at - before, at + after]` out of a recording through its sidecar, into a WAV with its own sidecar. |
| `decode-ref` | `--ogg O`, `--out W` (both required) | Decodes a reference `.ogg` to 48 kHz, 2-channel, 16-bit PCM through Windows' own Media Foundation, from Windows PowerShell's WinRT: `MediaTranscoder` first, then an `AudioGraph`. On this machine only the `AudioGraph` route opens Ogg Vorbis. When neither route works it exits 2 with `no decoder` and both reasons. It downloads nothing. |

The analysis's template checks read `L\ref\*.wav`: the one whose name holds
`death` is the death cry, and the rest are barks. `ref\refs.json` can map a
bark's file name to its line's text. Getting the `.ogg` files out of the
game's bundles needs the decoded index and the Oodle decoder above; it is
the person's step, and the copies stay in `ref\`.

## Instrument 3: screen capture, OCR and the marker

**What it is.** Two bands of the game's client area are copied from the
screen at 2 Hz: the top-centre boss bar (`top`) and the chat region
(`bottom`). `Windows.Media.Ocr` reads them afterwards, offline, and only for
the frames the analysis asks about.

**What it sees.** What is drawn on screen: subtitles and dialogue echoed to
chat (`output_all_dialogue_to_chat=true` in the owner's config), and the
boss bar's name.

**What it is blind to.**
- **Black frames.** A GDI copy of a DX12 borderless window may come back
  black; whether it does on this client was not established. Each frame's
  luma mean and spread go into its index, so a black capture shows.
- **Covered windows.** It captures the screen, not the window, so anything
  over the game is captured instead.
- **Minimized windows.** While the game is minimized no frame is taken,
  and the skip is logged.
- **A line with no words**, like the death cry, has nothing to read; the
  boss bar's disappearance is the screen's only route to the kill.
- **The boss bar as an image.** Whether OCR can read the boss bar at all
  is itself a check (`i3-bar-control`). A pixel-based bar detector is out
  of scope here.
- **DPI.** Per-monitor DPI awareness is set before any screen call; the
  client logs `[WINDOW] Dpi Scale = 1`.

**`capture.ps1`**:

```
powershell.exe -NoProfile -ExecutionPolicy Bypass -File capture.ps1 -TargetPid P -OutDir L\i3 -StopFile L\stop-i3 -Hz 2 [-Top 0.25,0,0.75,0.14] [-Bottom 0,0.35,1,1] [-MaxMinutes 90] [-WaitSeconds 10]
```

- **The window.** It takes the largest visible top-level window of P,
  waiting up to `-WaitSeconds` for one, and reads its client rect on every
  frame.
- **The bands.** Each band is a fraction `x0,y0,x1,y1` of that rect, and
  `off` leaves it out. A band is saved as a quality-90 JPEG,
  `frames\<band>\<yyyyMMddTHHmmss.fffZ>.jpg`.
- **The index.** Each frame gets a `frames.jsonl` line: `utc`, `band`,
  `path`, `rect`, `client`, `foreground`, `luma_mean` and `luma_std`. A
  skipped or failed frame gets `path` null and a reason.
- **The schedule** is fixed. A tick missed while the machine was busy is
  skipped, never caught up.
- **Ready and stop.** It prints `capture: ready ...` after the first frame
  and `capture: stopped <reason> ...` on its stop file, when the window is
  gone, or at `-MaxMinutes`.
- **Size.** About 2 GB per 30 minutes.

**`ocr.ps1`**:

```
powershell.exe -NoProfile -ExecutionPolicy Bypass -File ocr.ps1 -List frames.txt -Out L\i3\ocr.jsonl [-Scale 2] [-Language en-US]
```

- **Input.** `-List` is a UTF-8 file with one image path per line, all read
  in one process.
- **Scale.** Each image is upscaled by `-Scale` first. The scale is lowered
  when it would push a side past the engine's maximum image dimension,
  10,000 (measured).
- **Output.** It appends `{"path", "scale", "lines": [{"text", "box"}]}`
  per image, with boxes in the original image's pixels, and prints
  `ocr: done <n> images <e> errors`.
- **Exit codes.** 2 when no OCR engine exists for the language, 1 on a bad
  argument.

**`probe_window.ps1`**, the capture's positive control:

```
powershell.exe -NoProfile -ExecutionPolicy Bypass -File probe_window.ps1 [-Text "Beira of the Rotten Pack|What happened here...?"] [-Seconds 3] [-RenderTo DIR]
```

It shows the lines (`|` separates them) in a borderless, topmost window,
light on dark, and prints `probe: pid <pid> shown <utc> rect [l,t,r,b]`.
With `-RenderTo` it opens no window and draws each line to
`DIR\line-<n>.png` instead.

**`marker.ps1`**, the person's kill stamp:

```
powershell.exe -NoProfile -ExecutionPolicy Bypass -File marker.ps1 -Out L\markers.txt -StopFile L\stop-marker
```

- **The hotkey.** It registers Ctrl+Alt+M as a system hotkey, with
  `MOD_NOREPEAT`, so holding the keys counts once. It checks the stop file
  every 200 ms.
- **Its header** goes into the file, and the first header line,
  `# registered Ctrl+Alt+M ...`, is its ready line.
- **A press** appends `<n> <utc>`, flushes, prints `marker: press <n> <utc>`
  and plays a 1760 Hz, 150 ms tone from the marker's own process. The tone
  is what shows that the game's recording does not hear other processes.
- **Stop.** It appends `# stopped <utc> presses <k>` when the stop file
  appears.
- **Errors.** It exits 2 when the hotkey is already taken.

**`py -3 -m tools.quest_tracker.outside.screen`**:

| Command | Options | What it does |
|---|---|---|
| `selftest-ocr` | | Renders "Beira of the Rotten Pack" and "What happened here...?" to PNGs in a temp folder, with no window, and OCRs them. Passes when each scores at least 0.8. |
| `rehearse` | `--out DIR` (required) | Shows the probe window for about 3 s and captures it for 2 s. OCRs the frames and prints the `i3-rehearsal` line. Needs an unlocked desktop. |
| `ocr` | `--live L`, `--band top\|bottom`, `--window FROM TO` (repeatable) or `--all` | OCRs the frames of `L\i3` inside the given UTC windows, skipping those already in `ocr.jsonl`. |

**The fuzzy match.** Both strings are lowercased and kept to letters,
digits and spaces, with runs of spaces collapsed. The score is the best
`difflib` ratio of the target against any stretch of the OCR line within
±20% of the target's length. A match needs at least 0.8.

## Starting, stopping and the session analysis

**`py -3 -m tools.quest_tracker.outside.session`**:

| Command | Options | What it does |
|---|---|---|
| `prep` | `--live L`, `--client-txt P`, `--ggpk G` (defaults: the install's `logs\Client.txt` and `Content.ggpk`) | Creates `L` and its subfolders, refusing an `L` that holds anything but a `tools` folder (the approved downloads, which it keeps). Writes `session.json` and the first clock anchor. It looks only at the two files' sizes and times, and reads neither. |
| `clock` | `--live L` | Appends one anchor to `clock.jsonl`. |
| `stop` | `--live L`, `--only i1,i2,i3,marker` | Creates the stop files (all four by default). |

**`py -3 -m tools.quest_tracker.outside.run`**, which starts the three
unelevated instruments against the game's pid:

| Command | Options | What it does |
|---|---|---|
| `start` | `--live L`, `--pid P` (required, from the game lease), `--timeout S` (default 30), `--max-minutes M` (default 90) | Launches `loopback.ps1`, `capture.ps1` and `marker.ps1`, each detached with its output in its log file (`i2\recorder.log`, `i3\capture.log`, `marker.log`), so they outlive the shell. Waits for each one's ready line, prints one line per instrument and records the pids in `run.json`. Exits 0 when all three are ready, 1 otherwise; an instrument still alive keeps running until `stop`. |
| `stop` | `--live L`, `--timeout S` (default 300) | Creates every stop file, `stop-i1` included, which asks the elevated controller to save its trace. Waits for each instrument in `run.json` to exit, and for `i1\status.txt` to reach `session-trace-stopped` or `error`. Prints one line each, and exits 0 when everything ended in time. |

The file trace is started separately, behind its admin prompt, because
`run` never elevates.

**`py -3 -m tools.quest_tracker.outside.analyse`**:

```
py -3 -m tools.quest_tracker.outside.analyse [--live L] --pid P [--only NAME,...] [--listened <bark>,<kill>] [--confirmed yes|no]
```

It reads what the instruments left in `L` and the session's part of
`Client.txt` (from `client_txt_offset`, read-only). It prints one capture
line per check, in the live procedure's order,
`- <name> | expected: ... | observed: ... | <verdict>`, and writes
`analysis.json`. A full run also writes the two clips the person listens
to.

- **The checks**, in order: `control`, `marker`, `ggpk-stable`, the four
  rehearsals (`i1-rehearsal`, `i1-rehearsal-mapped`, `i2-rehearsal`,
  `i3-rehearsal`), the town checks (`i2-town`, `i2-isolation`, `i3-town`),
  then per instrument a control on her logged barks, a control on Una's
  "What happened here...?" and the target at the kill press (`i1-…`,
  `i2-…`, `i3-…`), and last `reward-split`.
- **A target can only be `not-observed`** when its control passed.
  Otherwise the instrument has shown nothing.
- **Times.** M is the first marker press after the `G1_2` area line. The
  press before it is the test press, made in town. The control windows are
  `[L - 0.5, L + 1.5]` around a logged line L, and the target window is
  `[M - 3, M + 2]`.
- **The base rate.** The fight is tiled into 2 s windows, leaving out the
  stretches around logged lines and the focus-loss stretches. A target
  also needs a base rate below 0.5: the share of those windows that hold
  the same signal by chance.
- **`--only`** computes just the named checks and the checks they depend
  on, and merges the results into `analysis.json`. Use `--only control`
  before anything else exists, and `--only i2-town,i2-isolation,i3-town`
  in town while the instruments run.
- **`--listened`** carries the person's two answers about the clips
  (`yes`, `no` or `unsure`) for `i2-listened`.
- **`--confirmed`** carries the person's answer to "was the press at her
  death?". Without it, `marker` cannot pass.
- **Failures.** A missing input makes the checks that need it
  `not-run (instrument: <why>)`, never a crash. An error inside one check
  names it, and the run exits 1 after printing every other check.

The analysis starts no program except `ocr.ps1`, for frames not yet in
`i3\ocr.jsonl`. The constants of each check are pinned in `analyse.py` and
its tests.

## Re-running the analysis on kept captures

A kept live folder is all the analysis needs. Nothing has to be recorded
again:

1. With the game closed, make sure `ggpk\tree-before.json` exists (and
   `tree-after.json` if the GGPK may have changed). If not, run
   `py -3 -m tools.quest_tracker.outside.ggpk walk --out L\ggpk\tree-after.json`.
2. If `i1\session.xml`, `i1\reads.jsonl` or `i1\mapped.jsonl` is missing,
   run `fileio export`, then `fileio reads --pid P`, then `fileio map`.
   Once a decoded `ggpk\index.json` exists, follow with
   `bundle_index annotate`.
3. Run `py -3 -m tools.quest_tracker.outside.analyse --live L --pid P`,
   with the person's `--listened` and `--confirmed` answers.

`P` is the game's pid in that session, from the `[INFO Client <pid>]`
lines and `run.json`. OCR results are cached in `ocr.jsonl`, so a re-run
reads only new frames. `--only` re-runs one check without the rest.

## What Live 6 showed

Live 6 ran these instruments during one Beira kill on 2026-10-26. Its
first attempt was blind at `run start` (the spawned instruments did not
run their command) and spent no kill; it was run again as session 7, after
a Restart, into a new live folder. Every instrument's rehearsal and
controls passed there. The verdicts and what they mean for the quest
tracker are in [`quest-tracker-research.md`](quest-tracker-research.md)
§ "Live 6: three instruments, one kill" and § "Product route"; this
section is about the tools.

- **The file-read trace.** The elevated controller has to be started by
  the person: the operator's own tool layer refused the `RunAs` launch
  before any UAC prompt, and the person ran the same command from their
  PowerShell and accepted the prompt. The rehearsal saw both the cached
  read call and the mapped page's paging read, so the trace is not blind to
  either. The session's trace of about 10 minutes of play was a 2.6 GB ETL
  that exported to a 24 GB XML in 314 s; `fileio reads` took 905 s over its
  13 million events and found 62,562 reads of `Content.ggpk` by the game,
  every one a paging read, and `fileio map` placed every one in the GGPK
  tree. Without a decoded index, R holds only the dialogue bundles read at
  her lines, so a bundle first read at the kill (`monsters.F4` in this
  session) falls outside R; and `i1-target`'s window [M − 3, M + 2] missed
  the reads at her death because the press came about 4 s after it. The
  `not-observed` wording names the cache, but this session's reason was the
  base rate: the bank was read as often during the fight as at the kill.
  A run with a decoded index (the Oodle decoder in `L\tools\` before
  `prep`) would name the bundle that holds her cry.
- **The loopback recorder.** The selftest, the isolation check (the
  marker's beep 8.3 dB above its neighbours, below the 15 dB that would
  show other processes' audio) and every control passed, with 110 burst
  onsets over the session. The kill check's one onset came about 5 s after
  her death, so the burst rule did not show that it finds a cry inside a
  fight's sound; the person's listening did. Its base rates came from only
  two line-free windows, since the fight from her first line to the press
  lasted 27 s. The template checks need reference audio in `ref\`, which
  needs the decoders, and did not run.
- **Screen capture and OCR.** 2,110 frames at 2 Hz, none skipped and none
  black, so a GDI capture of the borderless window works on this client at
  2560×1440 and DPI scale 1. The chat band read every logged line it was
  checked against at a score of 0.8 or more (1.00 where the score was
  printed). The boss bar's name read in about half the
  frames while the bar was up and in none after the kill. The text check
  around the kill counts any new text, so it passed on dropped items' labels,
  the skill bar's key hints and the character's own remark, none of them
  hers.
- **The marker.** Two presses, the test press in town and the kill press,
  both stamped; the person's press came about 4 s after her death, which is
  why the windows above that end at M + 2 s were late for the file reads.
- **`analyse`.** It needs `--confirmed` for `marker` and `--listened` for
  `i2-listened`; until both are given those checks fail or stay `not-run`.
  `reward-split` failed (the reward came 40.1 s after the press), which is a
  finding about the game, not about the tools.

## Tests

From the repository's root:

```
py -3 -m unittest tests.test_quest_tracker_outside_common tests.test_quest_tracker_outside_ggpk tests.test_quest_tracker_outside_fileio tests.test_quest_tracker_outside_index tests.test_quest_tracker_outside_audio tests.test_quest_tracker_outside_desktop tests.test_quest_tracker_outside_analyse
```

The tests are fast, and touch no audio device, no window, no network and
no real game file. Their fixtures are synthetic, under
`tests/fixtures/quest_tracker/outside/`, and their temp folders sit outside
the repository. The rehearsals and selftests above (`fileio
rehearsal-check`, `audio selftest`, `screen selftest-ocr`, `screen
rehearse`, `ggpk selfcheck`) are the checks that touch the real machine,
and they run by hand.
