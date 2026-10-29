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

The first level-up and The Bloated Miller (read from the owner's log on
2026-10-28, its lines from 2025-08 to 2026-10; the research note's
[§ "The Bloated Miller: what ticks his step"](quest-tracker/quest-tracker-research.md#the-bloated-miller-what-ticks-his-step)):
every visit to The Riverbank (`G1_1`) that held a level-up held exactly
one, to level 2, of the character its level-up line names, and it came
after the Miller's first voiced line and 0 to 9 s after his last, 17 of
17. In all 17 it came before Mortimer's thanks ("Well done! Please come
inside.") and before the encampment entry. Timed against the kill once,
in Live 8's capture (a key press made as he died), it came at the kill.
A first level-up before his death in The Riverbank was not observed (not
proof that it cannot happen). The quest tracker ticks "Slay the Bloated
Miller" on it since its 0.1.1.

The same, in the game (measured 2026-10-28, Live 14, one new Warrior; the
research note's
[§ "Live 14: version 0.1.1 in the game"](quest-tracker/quest-tracker-research.md#live-14-version-011-in-the-game)):
the character's level-up to 2 was logged in The Riverbank in the same
second as the Miller's last voiced line (0 s after it, at the log's
one-second resolution) and 77 s before Mortimer's thanks (the player had
been asked to stay out of the encampment for a while after the kill); the
encampment loaded 3 s after the thanks. The kill itself was not timed by a press in that session.

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
A remark at a quest moment other than a kill is not observed either
(2026-10-28, the research note's
[§ "Remarks at every quest moment"](quest-tracker/quest-tracker-research.md#remarks-at-every-quest-moment)):
in a Ranger's session that reached the Tree of Souls in the Grelwood
(`G1_4`), where poe2db lists every class's remark on first seeing the tree
(key `SinTreeSeenComment`), the log from the Grelwood entry to the game's
close held only chat, window-focus lines, one shop request and the close.
That session had no control that the remark played, and none that a
voiced line there reaches the log: no voiced NPC line was logged in that
window either, as Una was never summoned; Live 15 later logged Una's line
in the Grelwood, the same-area control. Live 15 (measured
2026-10-28, a new Ranger, the research note's
[§ "Live 15: a quest-moment remark in the game"](quest-tracker/quest-tracker-research.md#live-15-a-quest-moment-remark-in-the-game))
had one: the remark showed on screen at the tree and the person heard it,
and `Client.txt` from the Grelwood entry to it held only window-focus
lines.

NPC lines and the dialogue option (measured 2026-10-26, Live 8, the
research note's
[§ "Live 8: on-screen text on a fresh character"](quest-tracker/quest-tracker-research.md#live-8-on-screen-text-on-a-fresh-character)):
Renly's vendor greetings do not depend on `output_all_dialogue_to_chat`,
as far as measured; boss combat lines and quest NPCs' lines (the quest
tracker's `npc` signals) were not measured with the option off. The
greetings, in Clearfell Encampment, were logged as `Renly: <text>` with the option on (3 of 3), turned off in the running
game (3 of 3), on again (2 of 2), and in a client started with the file
reading `false` (2 of 2), each 1 to 2.3 s before the person's press. Only
vendor greetings were measured. In the same session, with the option on,
Renly's quest talk in a dialogue window after The Bloated Miller left no
line in the log (the third visit to show it, after Lives 1 and 3), the
Miller's combat lines were logged with his name as speaker, and the
character's own shouts and two kill remarks were not logged, while the
level-up line at the kill was.

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
  machine read). `output_all_dialogue_to_chat` did not change whether
  Renly's vendor greetings reach `Client.txt` (Live 8, below); whether it
  gates boss or quest NPC lines is not established.
- **Sound** (static reading, 2026-10-26). `mute_in_background` (`true` on
  the machine read; going by the name, the game is silent while it is not
  focused, not measured), `dialogue_sound_volume2` (100) beside the
  master volume (48) and the music and effects volumes (41 each).
- **Window and chat** (static reading, 2026-10-26).
  `borderless_windowed_fullscreen` and `fullscreen` (`true` and `false`,
  at 2560×1440 on the machine read); `renderer_type` (`Vulkan` on the
  machine read); `chat_box_width`, `chat_box_height` (1040 and 640) and
  `show_chat_timestamps` (`true`). The panel key bindings sit in
  `[ACTION_KEYS]` as Windows virtual-key codes; on the file read they are
  I (inventory), C (character panel), P (passive tree), U (world map) and
  Tab (map overlay), with Options on O.
- **How the client reads the dialogue option** (static reading,
  2026-10-26; [§ "The dialogue setting (static reading)"](quest-tracker/quest-tracker-research.md#the-dialogue-setting-static-reading)).
  The file is UTF-8 with a byte-order mark and LF line endings;
  `output_all_dialogue_to_chat` is in `[UI]` and `enable_subtitles` in
  `[GENERAL]`. The client looks the option up in its in-memory options
  every time it is used rather than keeping a start-up copy, so a change
  in Options should apply from the next line (a prediction, not measured).
  With the option off, at least two places that would hand a voiced line
  to the chat skip it; whether the part that writes `Client.txt` is
  downstream of them is not established.
- **What the dialogue option changes** (measured 2026-10-26, Live 8).
  In Options it is labelled "Output Dialogue To Chat", in the UI tab. It
  did not change whether Renly's vendor greetings reach `Client.txt`, nor
  whether they show on screen: with it off they were logged and shown as
  with it on, both after the in-game toggle and in a client started with
  the file reading `false`. So whether the toggle applies from the next
  line on is not shown by greetings. Which lines the option does change is
  not established (only greetings were measured).
- **When the client writes the file** (measured 2026-10-26, Lives 7 and
  8). In Live 8 the running client wrote the file within the stage of each
  change in Options, at once and without an exit (the toggle off, read
  back as `false`; the toggle on, 3.3 s before the next greeting), and
  wrote it again at the exit, with the same contents as after the toggle
  off; no edit by hand was needed. In Live 7 the file read `mute_in_background=false` and a
  different `league_selected` after the game's exit, against the backup
  taken before the session.

## On-screen text (boss bar and subtitles)

Source: [`docs/quest-tracker/quest-tracker-research.md`](quest-tracker/quest-tracker-research.md)
§ "Live 6: three instruments, one kill" and § "Live 8: on-screen text on a
fresh character". Screen captures at 2 Hz of the borderless window
(2560×1440, English client), OCR'd offline with Windows' own OCR; the
game's process was not touched. Positions are fractions of the client
rect.

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
  the borderless window was black (the config file named
  `renderer_type=Vulkan`, last written before the session; other
  renderers not tested).
- **The character's own remark at a boss kill is shown but not logged**
  (measured 2026-10-26, one kill). About 1.5 s after Beira's death the
  player character's spoken remark appeared as a subtitle, prefixed with
  the character's name, and `Client.txt` has no line for it. Whether it
  follows every boss kill is not established.
- **The boss bar under each panel** (measured 2026-10-26, Live 8, The
  Bloated Miller alive, the player keeping its distance). The name sits
  at x 0.456 to 0.544, y 0.047 to 0.060, and read in 29 of 29 frames with
  no panel open. It still read with the inventory, the character panel,
  the Escape menu (the screen darkened) or the map overlay open. It did
  not with the passive tree open (a full-screen panel whose title takes
  the band, 0 of 7 frames) or the world map (its heading drawn over the
  band, 1 of 11). In that session the band with the bar up had a median
  luma of about 17 mean and 22 standard deviation, the band with the bar
  gone about 15 and 8 with no text, and the band under those two panels
  26 to 29 and 35 to 39 with the panel's text. After the player walked
  away, the boss alive, the name was in no frame from 1 s to 6 s after
  the press that marked it, so the bar's going is not a kill by itself.
- **Voiced lines show in one list at the lower left** (measured
  2026-10-26, Live 8). The boss's combat lines, an NPC's vendor
  greetings, the character's shouts and kill remarks and the level-up
  message each showed as one line, `<speaker>: <text>` for the voiced
  ones, from x 0.03, the newest at about y 0.72, older lines moving up
  about 0.02 per line. Greetings showed there the same with
  `output_all_dialogue_to_chat` off. Whether the list is the chat's
  recent-message display or the subtitle display is not established. The
  character's combat shouts carry the same name prefix as its kill
  remarks, so only the text tells a kill remark; at the Miller's kill the
  two remarks the person heard came 0.4 s and 3.0 s after their press.
- **A quest-moment remark shows in the same list** (measured 2026-10-28,
  Live 15, one new Ranger, the quest tracker's reader copying the
  voiced-lines region from the running window;
  [`quest-tracker-research.md`](quest-tracker/quest-tracker-research.md)
  § "Live 15: a quest-moment remark in the game"). The character's
  remark on reaching the Tree of Souls in the Grelwood (`G1_4`, poe2db
  key `SinTreeSeenComment`), not a kill remark, showed in the lower-left
  voiced-lines list as `<character name>: <text>`, the character's name
  as its prefix, the text word for word poe2db's Ranger line for that key
  (a screenshot by the person). The reader matched it at the first of
  its reads that showed it, before Una was summoned there, and stopped
  reading at the tick, so how many reads would have caught it is not
  measured. Other classes' lines and
  other quest moments are not measured.
- **A quest dialogue window** (measured 2026-10-26, Live 8, Renly's talk
  after The Bloated Miller). Drawn in the centre: the NPC's name as a
  heading, the text at x 0.42 to 0.66, y 0.51 to 0.61 (moved left while
  the inventory is open, to take a reward), a reward choice above it, a
  "QUEST COMPLETE" banner and then the next quest's banner at y 0.78, and
  the topic list at the end. OCR read the name at 1.00 and 16 lines of 3 or
  more words. Its lines carry no speaker prefix, are not repeated in the
  lower-left list as the NPC's lines, and are not in `Client.txt`.
- **The client's own quest panel** (read 2026-10-26 from Live 8's OCR
  cache after the session, not a check). The panel the `quest_tracking`
  option in `[UI]` shows sits at the right, at x 0.82 to 0.98, y 0.28 to
  0.35. It names the tracked quest ("Reaching Clearfell", later
  "Treacherous Ground") and its current objectives ("Kill the Bloated
  Miller and end his rage"; then "Search Clearfell for the entrance to the
  Mud Burrow" and "Slay the Devourer in its lair"). No tool reads it.
- **What else the bar's band shows** (read 2026-10-26 from Live 8's and
  Live 6's OCR cache and frame index after the sessions, not a check).
  With the inventory open, the boss's name read at x 0.28 instead of
  0.456, so it moves with that panel. The passive tree's title ("PASSIVE
  SKILLS", x 0.46 to 0.54, y 0.02 to 0.04) and the world map's headings
  ("WORLD" at y 0.02 to 0.04, "ACT 1" at x 0.49 to 0.51, y 0.06 to 0.07)
  sit over the name's column. The scene draws text in the band too: after
  Beira's death a "CHECKPOINT" label read at x 0.30 to 0.35, its height
  moving with the camera, so other text in the band does not by itself
  mean a panel; after the walk away from the Miller the band held no OCR
  line. The band's luma does not separate the bar from its absence: its
  spread was 11 to 16 after Beira's death, and 10 to 11 after the Miller's
  kill, rising to 26 within 10 s with no bar, so the spread of about 8
  recorded above for the bar gone was one spot. The overlay's reader classes the band by OCR text and
  position, never luma ([`quest-tracker/README.md`](quest-tracker/README.md)
  § "How detection works").
- **Beira's bar under the passive tree and at her kill, read live**
  (measured 2026-10-27, Live 9, one fight; the overlay's own reader
  copying the band about twice a second from the running window,
  [`quest-tracker-research.md`](quest-tracker/quest-tracker-research.md)
  § "Live 9: the screen reader in the overlay"). With the passive tree
  open for about 12.5 s during her fight, her bar was never read as gone,
  and her name read again once the tree closed, as the Miller's did under
  the same panel in Live 8. Her name's last read was 1.4 s before a press
  made as she died.
- **The QUEST COMPLETE banner in Renly's window, read live** (measured
  2026-10-27, Lives 9 to 11, one talk each, same source). After the reward
  was taken the banner was read once per session in the dialogue window's
  region, 0.75 s and 0.5 s before a press made as it showed (Lives 10 and
  11). A strip of the window's top, x 0.22 to 0.70, y 0.24 to 0.31, read
  Renly's name as its heading before the banner (Live 10).
- **Where the game's panels draw** (measured 2026-10-27, Live 12, a no-kill
  session in a town, each panel opened twice over two backgrounds, whole
  client frames at 4 Hz, 2560×1440;
  [`quest-tracker-research.md`](quest-tracker/quest-tracker-research.md)
  § "Live 12: the game panels' footprints"). As fractions of the client
  area, each panel's own extent as its frames show it: the inventory (I)
  a panel on the right, x 0.65 to 1.0, y 0 to 0.845; the passive tree (P),
  the microtransactions shop (M) and the world map (U) the whole client;
  the Escape menu dims the whole client (its mean luma 0.58 of the frames
  before it), its menu box in the centre, the box's top at about x 0.44
  to 0.53, y 0.28 to 0.37; the options (O) a panel on the left, x 0 to
  0.345, y 0 to 0.8; the stash and a merchant's trade window open the
  inventory on the right, as above, beside a window of their own on the
  left (within about x 0 to 0.50), the stash's drawing a header where
  the options panel draws its own; the character panel (C) sits on the
  left. A frame-difference footprint does not give these: the town's
  background and the HUD change while a panel is open, so every panel's
  changed area was the whole client or nearly. The inventory's header
  plate, the passive tree's title plate and the top of the Escape menu's
  box kept their colours in Live 8, another character's session in The
  Riverbank. All of it read from quality-90 JPEG frames. A raw GDI copy
  of the client matched them (measured 2026-10-27, Live 13, 2560×1440,
  the inventory open, another new character in Clearfell Encampment): on
  each of the inventory header plate's six 16 px patches the mean per
  channel sat within 0.8 levels of the same patch's mean over 23 open
  quality-90 JPEG frames, and its luma spread at 0.989 to 1.014 times
  theirs; with the inventory closed, none of the six held its open
  colour. Read on raw copies by the overlay's watcher, the passive tree's
  title plate and the top of the Escape menu's box also kept their
  colours in Live 13; their raw copies were not compared patch by patch.

## Item search box (stash and vendor)

Source: [`docs/waystone-roller/waystone-roller-research.md`](waystone-roller/waystone-roller-research.md)
§ "Search syntax" and § "poe2db Waystones page". The search box of the
stash and of a vendor highlights the items whose text matches what is typed.
Each fact is labelled with its source: measured in the waystone roller's
Live procedure 1 on 2026-10-28 (eight waystones in a stash tab: Normal,
Magic, Rare and corrupted Rare at a low tier and at tier 15), (P) the
public tool poe2.re's practice (its shipped JavaScript, read 2026-10-28),
or (O) the owner.

- **The box takes at most 250 characters**: a 250-character paste is
  accepted whole, a 251-character paste is refused whole (the box stays
  empty), not truncated (measured 2026-10-28; O, P).
- **Matching ignores case** (measured 2026-10-28; P).
- **A space separates terms, and an item must match every term**
  (measured 2026-10-28; P).
- **`"..."` makes one term of text that holds spaces** (measured
  2026-10-28; P).
- **`|` inside a term means or, and a leading `!` negates the term**:
  `"!a|b"` hides the items that match a or b (measured 2026-10-28; P).
- **A term is a regular expression**: character classes `[..]` and escapes
  such as `\)` work (measured 2026-10-28: `"r 2\)"` and `"y: [nr]"`; P).
- **The box searches the advanced item text, roll ranges included**: a
  mod's number shows with its range, as in `+14(11-15)%`, and `"15\)"` lit a
  tier-2 waystone through it (measured 2026-10-28). A term meant for the
  name line's `(Tier 15)` therefore needs the letters before the number
  (`"r 15\)"`).
- **The item text has a rarity line `Rarity: Magic`** (and Normal, Rare),
  so `"y: m"` finds magic items (measured 2026-10-28; P).
- **A corrupted item's text carries `Corrupted`**, so `corr` and `!corr`
  split corrupted from uncorrupted items (measured 2026-10-28; P).
- **A waystone's name line carries `(Tier N)`**: `"r 2\)"` lit exactly the
  tier-2 waystones (measured 2026-10-28; poe2db's base type `Waystone
  (Tier 15)`).
- **A waystone's item text** (measured 2026-10-28, copied in the advanced
  form): `Item Class: Waystones`, the rarity line, a rare's random name, the
  name line (a magic waystone's affix names around `Waystone (Tier N)`),
  summary lines (`Revives Available`, `Item Rarity`, `Pack Size`, `Monster
  Rarity`, `Monster Effectiveness`, `Waystone Drop Chance`, each ending in
  `(augmented)` when a mod raised it), `Item Level`, a header
  `{ Prefix Modifier "<affix name>" (Tier: N) }` (or `Suffix`) above each
  mod's lines, each number followed by its roll range, a mod line that
  cannot roll ending in `— Unscalable Value`, the Map Device description
  and, last, `Corrupted`.
- **A waystone mod's reward lines** ("15% more Waystones found in Area")
  were not observed in the item text of the measured waystones (measured
  2026-10-28): none of the copied advanced texts of the magic and rare
  waystones held a reward line, although poe2db lists reward lines on every
  regular mod they carried, and the search `"waystones found"` matched none
  of them. Other waystones, and other text forms, were not checked.
- **`Ctrl+Alt+W`, registered by another program through `RegisterHotKey`,
  reaches that program while the game is in front** (measured 2026-10-28,
  waystone roller Live 2, which registers it with
  `tauri_plugin_global_shortcut`; research note § "The built app in the
  game (Live 2)"). Not measured: other key combinations, other ways of
  catching keys (a low-level keyboard hook), and whether the game also sees
  the keystroke.
- **The game's window is titled `Path of Exile 2`**, process name
  `PathOfExile` (measured 2026-10-28, waystone roller Live 3, read from
  outside with `Get-Process`; research note § "The auto-paste in the game
  (Live 3)").
- **The focused search box takes injected `Ctrl+A` then `Ctrl+V` and
  replaces its text**: with `xyz` typed in the stash search box, keys sent
  through the Windows input queue (`Ctrl` down, `A`, `Ctrl` up, then the
  same with `V`) after another program put `corr` on the clipboard left the
  box holding exactly `corr`, and a second time still `corr` once
  (measured 2026-10-28, waystone roller Live 3). So the box treats
  `Ctrl+A` as select-all and `Ctrl+V` as a paste over the selection.
- **With no text field focused, a shortcut press that sent Ctrl+A then
  Ctrl+V was followed by game key bindings firing (not separated: Ctrl+A,
  Ctrl+V or the Ctrl+Alt+W press itself); with the chat open, the injected
  Ctrl+V pasted into the chat's input line** (measured 2026-10-28,
  waystone roller Live 3, the owner's report: "buttons were pressed", and
  the text was pasted into open chat). Which binding
  fired, and for which of `Ctrl+A`, `Ctrl+V` or the `Ctrl+Alt+W` press
  itself, was not separated.
- Not established: whether lookahead (`(?=`, `(?!`) works
  (`"way(?=stone)"` lit every waystone: accepted or ignored); any way to
  count modifiers. Plain `Ctrl+C` copies the same advanced text as
  `Ctrl+Alt+C` (measured 2026-10-28 on one rare gloves; § "Item text
  (Ctrl+Alt+C and Ctrl+C)" below).

## Item text (Ctrl+Alt+C and Ctrl+C)

Source: the owner's six in-game copies of equipment, 2026-10-28, kept byte
for byte in
[`apps/crafting-helper/crates/ch-item/fixtures/real/`](../apps/crafting-helper/crates/ch-item/fixtures/real/README.md)
(a rare gloves copied with `Ctrl+Alt+C` and again with `Ctrl+C`; a normal
ring, a magic boots, a rare crossbow and a corrupted rare jewel, each in the
advanced form, copy key not stated). The crafting helper's item text parser,
`ch-item`, reads this layout; its tests assert each file. Everything below
is measured from those texts (owner's copies, 2026-10-28) unless it says
otherwise, and adds to the waystone text layout in § "Item search box
(stash and vendor)" above.

- **Plain `Ctrl+C` gives the advanced form**: the gloves copied with
  `Ctrl+C` and with `Ctrl+Alt+C` gave identical text, headers and roll
  ranges included (one item; the owner confirmed which key each copy used).
- **Class strings**: `Item Class: Gloves`, `Boots`, `Rings`, `Crossbows`,
  `Jewels` (the plural class name, as `Waystones`).
- **Requirements are one line**: `Requires: Level 80, 101 Int`,
  `Requires: Level 79, 89 Str, 89 (unmet) Dex`, or `Requires: Level 20`
  alone, in their own section. An attribute the copying character does not
  meet carries `(unmet)` between its number and its name. A jewel's text
  has no requirements line.
- **Sockets**: `Sockets: S ` (with a trailing space) for one socket holding
  a rune, in its own section.
- **Properties**: armour pieces show their defence (`Energy Shield: 276
  (augmented)`, `Evasion Rating: 362 (augmented)`); a weapon shows
  `Physical Damage: 77-304 (augmented)`, an elemental damage line ending in
  its type, `Fire Damage: 59-104 (fire)`, `Critical Hit Chance: 8.33%
  (augmented)`, `Attacks per Second: 1.65` and, on a crossbow, `Reload Time:
  0.75`. `Quality: +20% (augmented)` is a property line too.
- **A rune's line, an implicit and a corruption's enhancement each have
  their own section**, after `Item Level` and before the explicit
  modifiers, which share one section. No text showed two of the three, so
  their order among themselves is not established. A rune's line ends
  `(rune)` and has no header.
- **`{ Implicit Modifier }`** heads an implicit, with family tags
  (`{ Implicit Modifier — Chaos, Resistance }`) or without; it has no affix
  name and no tier, and the line under it carries no `(implicit)` marker.
- **`{ Corruption Enhancement — Elemental, Cold, Resistance }`** heads a
  modifier on a corrupted jewel: a header without the word `Modifier`, in
  its own section, with a roll range on its line (`+10(5-10)%`).
- **Explicit headers carry family tags after an em dash**:
  `{ Prefix Modifier "Tyrannical" (Tier: 2) — Damage, Physical, Attack }`;
  a header may carry none (`{ Prefix Modifier "Hummingbird's" (Tier: 1) }`).
- **A revealed desecrated modifier** has the word `Desecrated` before
  `Prefix` or `Suffix` in its header (`{ Desecrated Suffix Modifier "of
  Exile" (Tier: 2) — Chaos, Resistance }`), and no marker on its line.
- **A modifier with two lines** sits under one header (`Hummingbird's`:
  evasion and stun threshold); **a line with two values** carries a range
  after each (`Adds 59(56-70) to 104(84-107) Fire Damage`); **a decimal
  value** has a decimal range (`+3.33(3.11-3.8)%`).
- **A jewel's text** ends with its description line, a separator and
  `Corrupted` when corrupted.
- **The header's `Tier:` equals poe2db's tier order** (measured, five
  items: the boots, crossbow, gloves, jewel and ring above). Numbering a
  modifier's tiers on the item's base variant from the highest item level
  down (tier 1 the best), as the crafting helper's data does, gave the same
  number as the header on all 18 explicit modifiers the five items carry;
  test `real_fixture_tiers_equal_game_tiers` in
  `apps/crafting-helper/crates/ch-data/tests/data.rs`. Other classes and
  modifiers have not been compared.
- **A revealed desecrated modifier can be a normal-pool modifier**
  (measured, five items, two carrying one): the crossbow's `Incinerating`
  (Tier 4) and the gloves' `of Exile` (Tier 2) join to modifiers in poe2db's
  normal section, not its desecrated one, at those tiers; test
  `desecrated_mods_join_to_normal_section` in the same file. So a reveal
  may yield a normal-pool modifier; how many options a reveal offers is not
  established.
- Not established (no real text has shown it): a fractured modifier, an
  unrevealed desecrated modifier, a crafted modifier, an enchant, `Mirrored`,
  `Unidentified`, the class strings of the other equipment classes, a
  requirements block, negative roll ranges, and any text form without
  headers and roll ranges (no copy key has been seen to give one).
