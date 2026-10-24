# Synthetic Client.txt lines

Hand-written lines in the shapes the English client writes to `Client.txt`
(`docs/quest-tracker/quest-tracker-research.md`), for the `qt-logline` tests
(`crates/qt-logline/tests/parser.rs`). Nothing here comes from a real log:
character names, chat text and timestamps are invented, there are no IP
addresses, and the NPC "lines" are placeholders, not what the NPCs say.

| File | What it holds |
|---|---|
| `area.txt` | `Generating level ... area` joins and the `[SCENE] Set Source` lines after them, including `(null)`, `(unknown)`, `Act 1` and a name with a trailing space |
| `chat.txt` | baseline: chat on every prefix (`#`, `$`, `%`, `@`, `&`), local chat shaped like an NPC line, client notices with a colon, Atlas points and other lines that must produce nothing |
| `npc.txt` | allowlisted speakers, one speaker not on the tests' allowlist, and allowlisted speakerless system lines |
| `system_messages.txt` | level-up, slain, received, lost, joined and left |
| `rewards.txt` | `has received` lines with each kind of `[Group\|Shown]` / `[Shown]` markup |
| `points.txt` | passive and weapon-set point lines, and an endgame Atlas one |
| `session.txt` | `LOG FILE OPENING` markers and the user-agent line carrying the client version |
| `continuation.txt` | a multi-line warning whose untimestamped continuations look like recognised lines |
| `non_ascii.txt` | non-ASCII text in an NPC line, an area name and a chat speaker; the test also feeds each line with CRLF and CR endings |

The files are LF; a checkout that converts them to CRLF changes nothing,
because the parser strips a trailing `\r`.
