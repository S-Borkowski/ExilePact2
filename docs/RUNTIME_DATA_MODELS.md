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
Live 3 none came.

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
