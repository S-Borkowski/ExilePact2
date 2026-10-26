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
| `start` | `--live L`, `--pid P` (required, from the game lease), `--timeout S` (default 30), `--max-minutes M` (default 90), `--top x0,y0,x1,y1\|off`, `--bottom x0,y0,x1,y1\|off` | Launches `loopback.ps1`, `capture.ps1` and `marker.ps1`, each detached with its output in its log file (`i2\recorder.log`, `i3\capture.log`, `marker.log`), so they outlive the shell. Waits for each one's ready line, prints one line per instrument and records the pids in `run.json`. Exits 0 when all three are ready, 1 otherwise; an instrument still alive keeps running until `stop`. `--top` and `--bottom` replace the capture's top and bottom bands (its `-Top` and `-Bottom`), as fractions of the client rect or `off`. Each is checked as `capture.ps1` checks a band (four numbers in 0..1, with x1 > x0 and y1 > y0) before anything is spawned, and both `off` is refused. Without them the capture keeps its own bands (top `0.25,0,0.75,0.14`, bottom `0,0.35,1,1`) and its command is unchanged. Live 8's procedure passes `--bottom 0,0,1,1`, the whole client area, because where a dialogue window draws is not known. |
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

## The on-screen text session: `screentext`

**`py -3 -m tools.quest_tracker.outside.screentext`** is Live 8's
analysis. That session asks what the screen shows of three sources (a
boss's health bar, with each game panel open and after the player walks
away; the character's remark at a kill; an NPC's quest dialogue window),
and whether the client option `output_all_dialogue_to_chat` decides
whether NPC lines reach `Client.txt`. It runs beside the three instruments
above, started with `run start --bottom 0,0,1,1`, and reuses their live
folder, the marker and the capture's frames and OCR cache.

| Command | Options | What it does |
|---|---|---|
| `stage` | `--live L`, `--name NAME` | Appends `{"utc", "name"}` to `L\stages.jsonl`. The names are `test`, `boss`, `remark`, `dialogue`, `town-on`, `toggle-off`, `town-off`, `town-on-again`, `relaunch-off` and `end`; any other is refused. |
| `config` | `--live L`, `--label LABEL`, `--ini PATH` | Reads the client config, never writing it, and appends one record to `L\config.jsonl`. Prints one line. |
| `analyse` | `--live L`, `--pid P`, `--relaunch-pid P2`, `--character NAME`, `--boss NAME` (default `The Bloated Miller`), `--npc NAME` (repeatable), `--dialogue-option TEXT`, `--setting-label TEXT`, `--setting-found yes\|no`, `--panels LIST`, `--confirmed yes\|no`, `--only NAME,...` | Prints one capture line per check, `- <name> \| expected: ... \| observed: ... \| <verdict>`, in the order below, then how many stage runs it ignored, and writes `L\analysis.json`. |

- **`stages.jsonl`.** A stage runs from its stamp to the next stamp,
  whatever its name, and each marker press belongs to the stage that holds
  it. Stages may come in any order. A name may be stamped again (a repeat
  on the fallback boss stamps `boss` or `remark` a second time): every
  check reads only the last run of each stage, and `analyse` prints how
  many earlier runs it ignored, per name.
- **The presses.** The last `boss` run's presses are, in order, the bar
  coming up, an open and a close press for each panel in `--panels` order,
  and the walk away. Any other count makes every `bar-*` check
  `not-run (instrument: presses do not pair: <n>)`. The last `remark` run's
  first press is the kill, made when the boss dies, and one press follows
  each remark heard. The last `dialogue` run's first two presses are the
  window opening and closing; fewer than two makes the `dlg-*` checks
  `not-run`. `test` holds the marker's test press.
- **`config.jsonl`.** Each record holds the label, the time, the file's
  path, mtime and sha256, and two values, each `true`, `false` or
  `missing` and each read from its own section:
  `output_all_dialogue_to_chat` (section `UI`) and `enable_subtitles`
  (section `GENERAL`). The default file is
  `My Games\Path of Exile 2\poe2_production_Config.ini` under the
  Documents known folder, which follows a OneDrive redirect. `analyse`
  reads the records labelled `before` and `after-toggle-off`.
- **Speakers** are Clearfell Encampment's vendors, the dialogue NPC
  (`Renly`), each `--npc` and `--boss`. A screen line is
  *speaker-prefixed by NAME* when its text before the first colon,
  normalised as `screen.normalize` does, equals NAME normalised, or, for a
  NAME of 6 or more letters, scores at least 0.8 against it. A greeting's
  window is [press − 6 s, press + 1 s] for logged lines and [press − 6 s,
  press + 2 s] for frames.
- **`--panels`** defaults to
  `inventory,character,passive-tree,world-map,escape-menu,map-overlay`.
  A panel name is lowercase letters, digits and dashes, named once, and
  never `control` or `away`.
- **`--only`** computes the named checks, and silently the checks their
  verdicts read, and merges the results into an existing `analysis.json`.
- **Failures.** A missing input makes the checks that need it
  `not-run (instrument: <why>)`. A log check is `not-observed` only when
  its controls passed, and `not-run` otherwise. An error inside one check
  names it, and the run exits 1 after printing every other check.
- **OCR.** The OCR runner is `ocr.ps1`, through `screen.ocr_frames`. Only
  the frames in the windows the asked checks need are read, once, and
  cached in `i3\ocr.jsonl`.

The checks, in order (*pass* means what the line says was seen):

| Check | What it reads |
|---|---|
| `control` | After the session's offset in `Client.txt`: a `LOG FILE OPENING`, lines from `[INFO Client <pid>]` with `--pid`, and an area entry line from that pid (a fresh character starts in The Riverbank, `G1_1`, which counts). |
| `marker` | `markers.txt`'s `# registered` header, a press in stage `test`, and `--confirmed yes`. |
| `i3-rehearsal` | `screen.rehearsal_check` on `L\i3\rehearsal`: each probe line scores ≥ 0.8 in a captured frame. |
| `i3-town` | A speaker's line logged in a `town-on` greeting window scores ≥ 0.8 in a bottom-band frame within [L, L + 4 s] of it. |
| `cfg-on-log` | A speaker line logged in the greeting window of at least one `town-on` press. |
| `cfg-setting-found` | `--setting-found yes`: the person found the option in the game's Options; observed carries `--setting-label`. |
| `cfg-file-written` | The last `after-toggle-off` config record reads `output_all_dialogue_to_chat=false`; `not-observed` when it still reads `true`. Observed says whether the file changed since `before`. |
| `cfg-off-shown` | At least 2 `town-off` presses each have a bottom-band line in their window [press − 6 s, press + 2 s] that is speaker-prefixed by a speaker, or scores ≥ 0.8 against a line logged in `town-on`, and is new against the window's before frames, those of [start − 5 s, start − 1 s]: the line's frame holds more lines reading as its text than the most any one before frame held, a count that falls once that many have been missing for 2 frames in a row. So a chat row still on screen from before is dropped (observed counts the reads dropped), while a second row with the same greeting, a subtitle beside the stale row, or the line back after it left still counts. A press with no frame in its before window is not judged: none of its reads is a hit, and observed names it. *pass* when 2 judged presses are shown; *fail* when the shown and unjudged presses together are fewer than 2; otherwise `not-run (instrument: no frame before <n> press(es))`. |
| `cfg-off-log` | A speaker line logged, or a logged line repeating the shown text, in the window of a shown `town-off` greeting. Controls: `cfg-on-log` and `cfg-off-shown`. |
| `cfg-on-again-log` | As `cfg-on-log`, for `town-on-again`. |
| `cfg-off-relaunch-log` | As `cfg-on-log`, for `relaunch-off` presses and lines from `--relaunch-pid`; the press is the control. `not-run (not needed: ...)` when the stage is absent. |
| `dlg-window` | Between the dialogue's open and close: at least 3 lines of 3 or more words, each in 2 or more frames of [open + 0.5 s, close − 0.5 s] and in none of [open − 5 s, open − 1 s], plus the NPC's name. Observed: their union box. |
| `dlg-option` | `--dialogue-option`, the topic the person chose, scores ≥ 0.8 in a frame between open and close. |
| `dlg-subtitle` | A line speaker-prefixed by the NPC within [open, close + 2 s], new against the before frames of [open − 5 s, open − 1 s] by `cfg-off-shown`'s rule. With no frame in that before window the window is not judged: `not-run (instrument: no frame before the window)`. |
| `dlg-log` | Quest dialogue, told from the greeting that opening the talk fires. The *greeting set* is every line the NPC (`Renly`) logged in [open − 6 s, open + 1 s], plus every line it logged in a `town-on`, `town-off`, `town-on-again` or `relaunch-off` run; a logged line is a greeting when it scores ≥ 0.8 either way against one of them. *pass* when the NPC logged a line in (open + 1 s, close + 2 s] that is not a greeting, or a logged line there that is not a greeting (any speaker) repeats a `dlg-window` line. A line logged at the open press is never counted, even when it matches nothing in the town pool; observed counts those, and the lines set aside as greetings. Controls: `cfg-on-log` and `dlg-window`. |
| `bar-control` | `--boss` scores ≥ 0.8 in at least half (and at least 4) of the top-band frames in [bar-up + 0.5 s, first panel open − 0.5 s]. |
| `bar-<panel>`, one per `--panels` entry | The boss's name reads in at least half of the top-band frames in [open + 0.5 s, close − 0.5 s] of that panel; *fail* means covered or unreadable. Observed: the share, the band's median luma mean and standard deviation against `bar-control`'s, and the count of other top-band OCR lines. `not-run` without a passing `bar-control`. |
| `bar-away` | The boss's name in no top-band frame within [away + 1 s, away + 6 s]. |
| `remark-shown` | A line speaker-prefixed by `--character` in the window [press − 6 s, press + 2 s] of a `remark` press after the first, new against the before frames of [press − 11 s, press − 7 s] by `cfg-off-shown`'s rule. `not-run` when there is no such press. A press with no frame in its before window is not judged: *pass* when a judged press shows the line, *fail* when none does and every press was judged, otherwise `not-run (instrument: no frame before <n> press(es))`. |
| `remark-kill` | Such a line in a bottom-band frame within [kill − 2 s, kill + 15 s] of the kill press, new against the before frames of [kill − 7 s, kill − 3 s]; *fail* means no remark shown at the kill. With no frame in that before window it is not judged: `not-run (instrument: no frame before the window)`. Observed: the first frame's delay from the press. |
| `remark-log` | For each remark `remark-shown` or `remark-kill` found, a logged line repeating its text after the prefix, or a line with the character as speaker, within [t − 8 s, t + 2 s] of its first frame. Controls: `cfg-on-log` and a remark found. |

**Privacy.** The full-window bottom band holds the chat panel and the
character's name. The printed lines carry no OCR text: only scores against
known targets (an NPC, the boss, a logged greeting, the option the person
named), counts, times, and boxes as fractions of the client rect, taken
from each frame's `rect` and `client` in `frames.jsonl`. The character's
name is printed as `<character>`, and `--character` itself is never
printed. `analysis.json` stays in the live folder and may keep OCR text,
with the character's name masked the same way. Every command that writes
refuses a folder inside a git checkout.

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

## What Live 8 showed

Live 8 ran `screentext` beside the screen capture and the marker on
2026-10-26, on a fresh character and The Bloated Miller, with `run start
--bottom 0,0,1,1`. Every control passed. The verdicts and what they mean
for the quest tracker are in
[`quest-tracker-research.md`](quest-tracker-research.md) § "Live 8:
on-screen text on a fresh character", § "Dialogue setting route" and
§ "What the screen shows of each source"; this section is about the tools.

- **Capture.** 6,680 frames over about 28 minutes, the top band and the
  whole window, none skipped and none black in any check's window. The
  capture stopped when the game's window went at the first exit, so the
  relaunch stage (`relaunch-off`) is read from the log and the presses
  alone, as planned. The recorder ran because `run start` starts it and
  nothing read it; the file-read trace's controller was not started.
- **Stages and pairing.** The first `boss` run had 9 presses (the person
  pressed M, which opens the in-game shop, took the panels in another
  order and opened the world map after the walk away), so every `bar-*`
  check was `not-run (instrument: presses do not pair: 9)`; `boss` was
  stamped again, and the analysis read only the second run and printed
  `stage runs ignored: 1 (boss 1)`. A procedure should give the panel keys
  in the order to press them, and say that M is not a panel.
- **The remark checks pass on any line with the character's prefix.** At
  the Miller's kill the lines they credited were a combat shout from
  before the kill and a read of the name and colon alone, which OCR had
  split from its text; the two kill remarks the person heard came later
  and were not what the checks found. A remark check that is to prove a
  kill remark needs the remark's text, or at least some text after the
  prefix, and a window that starts at the kill.
- **The dialogue press came late.** The window's first page was already on
  screen 5 s before the open press, so `dlg-window` counted it as old and
  passed on the later pages alone. Its union box takes in every new line
  in the whole window, the lower-left line list and the skill bar's key
  hints included, so it is not the window's own box; the per-line boxes
  in `i3\ocr.jsonl` give that. `dlg-option` did not run, because the
  person reported the reward and the quest they accepted but not the
  topic's text.
- **The config.** The in-game toggle wrote the file at once, so
  `after-toggle-off` read `false` and the procedure's edit by hand with
  the game closed was not needed (`after-exit-off` read `false` too).
  `cfg-off-relaunch-log`'s controls, the relaunched pid's own `LOG FILE
  OPENING` and area entry and the `after-exit-off` record, all held.
- **OCR.** Only the frames in the asked checks' windows were read, 443
  of them, cached in `i3\ocr.jsonl`, so the frames around the kill in the
  top band were not read; the whole-window band covers that moment.

## Tests

From the repository's root:

```
py -3 -m unittest tests.test_quest_tracker_outside_common tests.test_quest_tracker_outside_ggpk tests.test_quest_tracker_outside_fileio tests.test_quest_tracker_outside_index tests.test_quest_tracker_outside_audio tests.test_quest_tracker_outside_desktop tests.test_quest_tracker_outside_analyse tests.test_quest_tracker_outside_screentext
```

The tests are fast, and touch no audio device, no window, no network and
no real game file. Their fixtures are synthetic, under
`tests/fixtures/quest_tracker/outside/`, and their temp folders sit outside
the repository. The rehearsals and selftests above (`fileio
rehearsal-check`, `audio selftest`, `screen selftest-ocr`, `screen
rehearse`, `ggpk selfcheck`) are the checks that touch the real machine,
and they run by hand.
