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
