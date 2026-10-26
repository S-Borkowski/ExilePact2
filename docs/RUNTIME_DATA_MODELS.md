# Runtime data models

This file collects what the mods in this repository have learned about Path
of Exile 2 itself: where a piece of game state lives, what shape it has, what
the game writes and when. Each mod's research note argues its findings out;
the facts that every mod can use are copied here, in our own words, each
labelled **measured** (with the date) or **static reading**, and linked back
to the note that holds the evidence. No decompiled or disassembled text
belongs here (AGENTS.md § "Legal").

## Client log (Client.txt)

Source: [`docs/quest-tracker/quest-tracker-research.md`](quest-tracker/quest-tracker-research.md),
which holds the counts, the negative results and the reasoning. Every fact
below was measured on 2026-10-24 from one player's log of 818,412 lines
(2025-08-04 to 2026-10-19, client versions 4.3.0 to 4.5.3), read read-only.

- **Location** (measured 2026-10-24). For the standalone client,
  `C:\Program Files (x86)\Grinding Gear Games\Path of Exile 2\logs\Client.txt`.
  `LatestClient.txt` beside it holds only the latest launch and is rewritten
  on each one. The Steam install's path is not established.
- **Encoding** (measured 2026-10-24). UTF-8, CRLF line endings. The file only
  grows (125 MB after fourteen months); a reader follows it from the end or a
  remembered offset, read-only, while the client keeps writing it.
- **Line shape** (measured 2026-10-24).
  `YYYY/MM/DD HH:MM:SS <ms counter> <hex> [<LEVEL> Client <pid>] <message>`,
  with levels `INFO`, `DEBUG`, `WARN` and `CRIT`. Lines without a timestamp
  are continuations of multi-line warnings and carry nothing to read.
- **The `<ms counter>`** (measured once, 2026-10-26, the research note's
  "Client.txt line format"). It counts the milliseconds
  since the system started: one line's value was 1.3 to 2.3 s below the
  time since the machine's boot computed for that line's second (one
  comparison), far from any since-client-start value; a tool converting it
  to UTC should pair a `GetTickCount64` reading with the UTC clock in the
  same session rather than rely on a boot time.
- **The `<pid>` in the header** (measured 2026-10-25, in Live 2). It is
  the process id of the running game client: the session's `[INFO Client
  <pid>]` lines carried the same number that Windows' task list gave the
  `PathOfExile` process (one session). The session marker line has no
  header and so no pid.
- **Session marker** (measured 2026-10-24).
  `YYYY/MM/DD HH:MM:SS ***** LOG FILE OPENING *****`, once per launch, but a
  launch can write it two or three times within seconds.
- **Client version** (measured 2026-10-24).
  `[HTTP2] User agent: PoE poe2_production/tags/<version> Windows x64`.

Event lines a mod can read, all measured 2026-10-24:

| Event | Message (after the `]` of the prefix) | Notes |
|---|---|---|
| Area entered | `Generating level <level> area "<code>" with seed <n>` (`DEBUG`) | One per instance join, as many as `Connecting to instance server`. Towns and hideouts have seed 1. Campaign codes `G1_*`..`G4_*`, `G<n>_town`, interludes `P1_*`, `P2_*`, `P3_*`. |
| Area display name | `[SCENE] Set Source [<name>]` (`INFO`) | The first one 3-16 lines after the area line, after a `[SCENE] Set Source [(null)]`. `(null)`, `(unknown)` and `Act <n>` are not names; strip trailing spaces. |
| NPC or boss voiced line | `<Speaker>: <text>` (`INFO`) | Short voiced lines; the dialogue window's text was not observed. Local chat and client lines such as `Queue file to download: ...` have the same shape: only an allowlist of speakers tells an NPC. |
| Chat | `#`, `$`, `%`, `@` (and `&`, not seen) before the speaker | Other players' messages; a mod drops them. |
| Permanent reward | `: <character> has received <stat>.` | Markup `[Group\|Shown]` or `[Shown]`; show the part after `\|`. |
| Reward swapped | `: <character> has lost <stat>.` | When a swappable boon is changed. |
| Passive points | `: You have received <n> Passive Skill Points.` / `... Weapon Set Passive Skill Points.` | Names no character. |
| Level-up | `: <character> (<class>) is now level <n>` | Party members too. |
| Death | `: <character> has been slain.` | Party members too. |
| Party | `: <name> has joined the area.` / `: <name> has left the area.` | Identifies party members, not own characters. |

Not observed in that log (measured 2026-10-24, a negative, not proof of
absence): any line naming a quest, its start, a step or its completion; any
line marking character select; the dialogue window's text.

The Riverbank (`G1_1`) and new characters (measured 2026-10-25, the research
note's "Character detection"): every one of the log's 12 entries to
The Riverbank is generated at area level 1. Level 1 is that area's level,
not a sign of a new character. The 12 entries generate 8 instances, one per
character. Seven are a single entry each. The eighth, on 2026-05-29, was
entered five times in five consecutive sessions, all by the same character:
its first entry is the new character, and entries 2 to 5 are four relogs at
level 1. Eight entries (the seven, and that instance's fifth) are followed
in the same session by the character's first level-up to level 2, 23 to 110
lines later. A relog inside The Riverbank generates `G1_1` again, so an
existing character re-entering `G1_1` was observed, four times. A
`[WARN] Character <name> has unknown build of path: None` line named the
character 1 to 2 s after that instance's entries 1 to 4: its first entry and
3 of the 4 relogs, not the fifth entry. For the 7 other new characters no
line but the character's own chat named it before the level-up. That
`[WARN]` line is **neither a new-character nor a relog signal**: the log's
27 such lines all name that one character, it did not follow the fourth
relog, and it never came for the 7 other new characters (nor, below, for
Live 3's two relogs). Where it comes, it names who entered; its absence says
nothing. A `G1_1` entry alone therefore does not tell a new character from a
relog.

The Riverbank's seed on a relog (measured 2026-10-25, the same log up to
the end of Live 3; the research note's
[§ "Character detection"](quest-tracker/quest-tracker-research.md#character-detection)): its 16
`G1_1` entries generate 10 distinct seeds, one instance per character, each
followed by that character's first level-up. The 6 other entries are
relogs that come back to an earlier entry's instance with the **same
seed**, each straight after a session marker: four in one character's
consecutive sessions at level 1, and Live 3's two, one at level 1 and one
after the level-up to 2, the first relog past level 1 observed. No relog
into a new seed was observed. The `[WARN] ... unknown build of path` line
followed 3 of the 6 relogs (and that one instance's first entry) and
neither of Live 3's. The client line `Requesting to complete all tutorials
due to the client setting` followed 8 of the 10 new instances (0 to 2 s
after the entry in 7 of them) and none of the relogs; it names no
character and follows the client's setting, so it is no character signal.
The `level 1` in the entry line is the area's level in every case,
including Live 3's level-2 relog, and the entry line names no character.

A boss's reward (measured 2026-10-25, the research note's "Rewards after a
boss kill"): a quest boss's `has received` reward line never came with the
boss's last voiced line. It came 5 to 465 s later, twice in the next area.
For Beira of the Rotten Pack it came when the player used the item she
dropped (measured, one session: 465 s after her last line, across an area
change), and no line about picking that item up or using it was observed
(one session, not proof that it is never logged).
For other bosses the last voiced line can be a combat bark during the
fight, so a gap of seconds does not show by itself whether the reward
waits on a later action. Beira's first line in each of the 9 sessions
with her lines is "Sleep in ice, wake in snow. Sleep my pretties as you
grow...", and the rest are combat barks. No line marking her defeat was
observed (between her last line and the next area or the reward, only
chat, window-focus and client lines), but that negative is not
controlled: no reading was timed against a known kill, and the owner
reports she was not defeated in the live sessions. Her reward came 7 to
44 s after her last line in 6 sessions and in the Mud Burrow in 2; in
Live 3 none came. Live 4 (measured 2026-10-26, one kill) controls that
negative: against the person's kill time, 11:28:17 local, her last line
was the bark "Swell!" at 11:28:04, and the 30 s after the kill held only a
window focus loss, a kind seen earlier in her fight, while the controls in
the same reading (the area entry, her first line, the next area's entry)
were all present. The reward came at 11:30:53, after the item was used.
Her defeat is therefore not observed in the log, with a control, from one
kill; her death cry has no text (poe2db's `CarrionCroneDeath`, the
research note's "Seeing a voiced line from outside the game").

Town dialogue (measured 2026-10-25, the research note's "Town dialogue"): in
Clearfell Encampment (`G1_town`), all 44 distinct lines logged from Una,
Finn, Renly and Mortimer are vendor, gambling or farewell lines; no quest's
talk was observed among them (the other towns' NPC lines were counted, not
classified). A 3.5-minute visit to Clearfell Encampment in which the player
talked to its NPCs recorded no NPC line at all: a talk in a dialogue window
was not observed to leave anything in the log. Live 3 repeated it: a visit
of about 3 minutes in which the player picked up a quest from Renly held
only chat, client, scene and window-focus lines (two visits, not proof that
a talk is never logged).

Una in the Grelwood and the Grim Tangle (measured 2026-10-25, the
research note's "Per-quest detection"; 7 sessions of the log up to Live 2
hold her Grelwood lines, and Live 3 an eighth with her first line only).
Her voiced lines there came in the same order in each of the 7 sessions. In the
Grelwood (`G1_4`), her first line each time is "What happened here...?",
when she is summoned at the Tree of Souls; in 4 sessions (from December
2025 on) "I'll wait here. The old magicks will protect me." followed it 2
to 8 s later. That wait line came after a talk with her (Live 2, where the
person had talked to her) and did not come after a summon alone (Live 3:
the game closed about 2.5 minutes after her first line with no wait line),
two sessions read against what the person did. "Good.
Now the others!" came in the Grelwood in 6 sessions, each time after that
session's visits to the Red Vale (`G1_5`). In the Grim Tangle (`G1_6`),
"The old magicks still thrive! The way is open. Please, hurry." came in 6
sessions, each time after "Good. Now the others!". Which quest step each
line stands for is the quest tracker's reading of that order (research
note), not part of the measurement.

Character remarks (measured 2026-10-25, the research note's
[§ "Character remarks"](quest-tracker/quest-tracker-research.md#character-remarks)):
poe2db lists each class's voiced remarks in a "Text Audio" table on the class
page, each keyed by the moment that plays it (entering an area, killing a
quest boss). None of the Mercenary's or the Warrior's remark texts was found
in the log, with a speaker or without one, while the same match found 19 of
Una's lines from her poe2db table (positive control). No speaker in the log
is named after a class, and a character's name appears as a speaker only in
typed local chat. When a Mercenary killed The Bloated Miller, a moment
poe2db lists a remark for, the next 60 s held no line but a window-focus
line. In Live 3 the person named four remarks they heard by the minute;
the log for those minutes held, besides chat, only window-focus lines, the
lines of the NPCs nearby (the Wounded Man's three, heard by the person,
are the positive control), a level-up and client lines, and every voiced
speaker in that session was an NPC or a boss. The own character's remarks
were therefore not observed in the log (the whole log and two sessions
with a person listening; not proof that they are never logged).

## Game files (Content.ggpk)

Source: [`docs/quest-tracker/quest-tracker-research.md`](quest-tracker/quest-tracker-research.md)
§ "Where the client keeps its audio (static reading)" and § "Live 6: three
instruments, one kill". The static readings are of 2026-10-26: a read-only
walk of the GGPK's directory records, headers only, with the game closed.
The last bullet was measured against the running game, from outside it,
in Live 6.

- **Where it is** (static reading, 2026-10-26). For the standalone client,
  `Content.ggpk` sits in the install folder beside `PathOfExile.exe` and
  `logs\`. On the machine read it was 154,759,947,271 bytes, GGPK version
  3, whose names are UTF-16.
- **Bundles** (static reading, 2026-10-26). `/Bundles2` holds 67,533 files
  (131.2 GB), with the bundle index in `_.index.bin` (115,154,591 bytes)
  and two 74-byte files, `_.index.low.bin` and `_.index.high.bin`. A
  bundle is split into 256 KiB blocks, compressed, with a table of the
  blocks' sizes in its header. Which bundle holds a given game file is
  written only in the index; bundle names do not say. Offsets move with
  each patch.
- **Audio** (static reading, 2026-10-26). The FMOD banks are plain GGPK
  files under `/FMOD/Desktop/`, not in bundles: 2,937 of them (15.5 GB),
  among them `CarrionCrone.bank` (15,376,384 bytes, Beira of the Rotten
  Pack's), `Una.bank`, `Master Bank.bank` and `Master Bank.strings.bank`.
  Voiced dialogue is in bundles under
  `/Bundles2/Streaming/Folders/audio/dialogue/`: `4.0/` holds 965 bundles
  named only by category (`npc.*`, `characters.*`, `monsters.*`,
  `mtx.*`), and `npc/` 88 bundles named by speaker.
- **Nothing loose** (static reading, 2026-10-26). The install folder holds
  no loose `Bundles2` folder and no Oodle DLL; `fmod.dll` and
  `fmodstudio.dll` sit beside the exe.
- **How the client reads it** (static reading of the exe, not of the GGPK,
  2026-10-26): on the paths followed, through a memory mapping; see
  § "Audio (FMOD)" below.
- **Read timing** (measured 2026-10-26, Live 6, a Windows file-I/O trace
  of one session from a cold boot, about 10 minutes in town, Clearfell and
  the Grelwood). All 62,562 reads of `Content.ggpk` by the game's process
  were paging reads, with no read call among them, so the game reaches
  the file through its memory map and a page shows in a trace only when it
  comes from disk. Most reads are of 64 KiB (52,196), and they
  map to bundle data (59,951), plain GGPK files such as the FMOD banks
  (3,236) and record headers. Audio is read when it plays, not at an
  area's load: see § "Audio (FMOD)". The GGPK's directory did not change
  over the session.

## Audio (FMOD)

Source: [`docs/quest-tracker/quest-tracker-research.md`](quest-tracker/quest-tracker-research.md)
§ "How the client loads its audio (static reading)", which names the build
read and how the call sites were found, and § "Live 6: three instruments,
one kill". Each bullet below is labelled: the static readings are of the
client's executable (2026-10-26); the Live 6 bullet was measured from
outside the game.

- **Banks reach FMOD through the client's callbacks** (static reading,
  2026-10-26). The client loads banks with
  `Studio::System::loadBankCustom`, the only bank loader it links, from
  one call site, passing the bank's
  path `FMOD/Desktop/<name>` as the bank's user data and its own open,
  close, read and seek callbacks. The read callback copies the bytes out of
  the client's memory map of `Content.ggpk`; it does not end in `ReadFile`.
- **Voiced lines are streamed through the same callbacks** (static reading,
  2026-10-26). The one `System::createStream` call site is the client's
  handler for FMOD's programmer-sound callback: it looks the event's name
  up in the audio table, opens it as a stream with the client's file
  callbacks; the call site sets no open-from-memory flag itself, but the
  mode comes from the audio table's answer (`getSoundInfo`), and whether
  that carries a memory flag is not established, and a flag in the event's
  user data makes the client pass no file callbacks at all, leaving the
  file to FMOD. Where neither applies, FMOD is expected (not established)
  to pull the line's bytes as it plays. A line inside a bundle is served
  by decompressing the 256 KiB
  blocks each read covers.
- **`Content.ggpk` is memory-mapped on the path followed** (static
  reading, 2026-10-26). On the one construction followed, the client opens
  a file (`Content.ggpk` going by its context, the path argument not
  traced), asking for read and write access and sharing read only, creates
  one file mapping over the whole file and maps views of it; on the bank
  and stream paths followed, a file's bytes are read through a pointer to
  its `FILE` record in that mapping. So a file trace is expected (not
  established) to see the game's audio reads only as paging reads of pages
  not already in memory. The exe's `ReadFile` call sites were not traced,
  so a `ReadFile` route to the GGPK is not excluded.
- **Decompressed bundle blocks are not kept** on the read path (static
  reading, 2026-10-26): each read decompresses into a temporary buffer and
  frees it. Whether the decompressor caches blocks itself is not
  established.
- **What the exe imports for audio** (static reading, 2026-10-26, the
  import tables and a string search of `PathOfExile.exe` on disk, names
  only). `fmod.dll` and `fmodstudio.dll` are delay-loaded. Besides
  `loadBankCustom` and `createStream` it links
  `EventDescription::isStream` and `getSoundInfo`; `loadBankMemory`,
  `loadBankFile`, `createSound`, `setFileSystem` and `loadSampleData` are
  neither imported nor present as strings (a name built at run time would
  not show). From kernel32 it imports both `ReadFile` and
  `CreateFileMappingW`/`MapViewOfFile(Ex)`.
- **Beira's death cry** (static reading of poe2db, fetched 2026-10-26).
  Its key is `CarrionCroneDeath`, its text field holds only that key, and
  its audio path ends in
  `monsters/part1/act1/carrioncrone/carrioncronedeath.ogg`. A line with no
  text was not observed in `Client.txt` (§ "Client log (Client.txt)").
- **When the audio is read** (measured 2026-10-26, Live 6, one Beira kill
  after a cold boot, a file-I/O trace of the game's paging reads of
  `Content.ggpk`). `CarrionCrone.bank` was read during play, not at the
  area's load or at her first line: 136 paging reads from her second line
  until about 1 s after her death, in bursts every 1 to 4 s, and none
  after. Her `monsters.D1` dialogue bundle was read at her first three
  lines, an `npc.12` dialogue bundle 0.02 s after Una's line, and a read
  followed both of a town NPC's lines. At her death, and at no other time in
  the session, the client read `BossKill.bank` and the dialogue bundle
  `monsters.F4`, then a `characters.*` bundle 0.1 s before the player
  character's spoken remark showed as a subtitle. Which bundle holds
  `carrioncronedeath.ogg` is not established (no decoded index), and
  whether a second kill in the same boot reads anything from disk is not
  measured: a page still in memory leaves no read. The game's own audio,
  recorded alone by process loopback, held a burst at 3 of her 4 lines
  and at Una's line, and the person heard a cry at her death; whether the
  burst rule found the cry itself is not established (the one onset it
  found was about 5 s after her death).

## Client configuration file

Source: [`docs/quest-tracker/quest-tracker-research.md`](quest-tracker/quest-tracker-research.md)
§ "The line the log does not carry". A static reading of one player's file
on 2026-10-26; what each option does in the game is read from its name,
not measured, unless a bullet says otherwise.

- **Location** (static reading, 2026-10-26).
  `Documents\My Games\Path of Exile 2\poe2_production_Config.ini` under the
  user's profile; where Windows redirects `Documents` (to OneDrive, on the
  machine read), the file follows it. Plain `key=value` lines.
- **Dialogue and subtitles** (static reading, 2026-10-26).
  `enable_subtitles` and `output_all_dialogue_to_chat` (both `true` on the
  machine read). Whether `output_all_dialogue_to_chat` is what puts NPC
  lines into `Client.txt` is not established.
- **Sound** (static reading, 2026-10-26). `mute_in_background` (`true` on
  the machine read; going by the name, the game is silent while it is not
  focused, not measured), `dialogue_sound_volume2` (100) beside the
  master volume (48) and the music and effects volumes (41 each).
- **Window and chat** (static reading, 2026-10-26).
  `borderless_windowed_fullscreen` and `fullscreen` (`true` and `false`,
  at 2560×1440 on the machine read); `chat_box_width`, `chat_box_height`
  (1040 and 640) and `show_chat_timestamps` (`true`).

## On-screen text (boss bar and subtitles)

Source: [`docs/quest-tracker/quest-tracker-research.md`](quest-tracker/quest-tracker-research.md)
§ "Live 6: three instruments, one kill". Screen captures at 2 Hz of the
borderless window (2560×1440, English client), OCR'd offline with
Windows' own OCR; the game's process was not touched.

- **The boss bar names the boss, and the name goes at the kill**
  (measured 2026-10-26, Live 6, one Beira kill). In the top-centre band,
  "Beira of the Rotten Pack" OCR'd in 12 of 23 frames during her fight
  and in 12 of the 20 frames in the 10 s before the kill marker, and in
  none of the 18 frames from 1 s to 10 s after it. The last frame with it
  was within about 0.5 s of her death as the level-up line and the file
  reads place it. Only about half the frames read the name while the bar
  is up.
- **Subtitles OCR** (measured 2026-10-26, Live 6). Her four lines, two
  town NPC lines and Una's line read at a score of at least 0.8 in the
  chat band within 4 s of their log lines. None of 2,110 GDI captures of
  the borderless DX12 window was black.
- **The character's own remark at a boss kill is shown but not logged**
  (measured 2026-10-26, one kill). About 1.5 s after Beira's death the
  player character's spoken remark appeared as a subtitle, prefixed with
  the character's name, and `Client.txt` has no line for it. Whether it
  follows every boss kill is not established.
