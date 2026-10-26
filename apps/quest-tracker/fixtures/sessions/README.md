# Synthetic sessions for the quest engine

Hand-written `Client.txt` sessions for the `qt-engine` tests
(`crates/qt-engine/tests/engine.rs`), the `qt-viewmodel` tests
(`crates/qt-viewmodel/tests/viewmodel.rs`) and the replay CLI's tests
(`crates/qt-replay/tests/replay.rs`), read with the synthetic quest data in
`fixtures/quests_min.json`. The line shapes are the parser's
(`fixtures/logs/README.md`); nothing here is copied from a real log.
Character names, chat text and timestamps are invented, and there are no IP
addresses. The NPC "lines" are invented too, except in `town_voices.txt`,
which quotes town NPCs' voiced lines in the shapes a real log recorded (NPC
lines may be quoted; its character is invented).

| File | What it holds |
|---|---|
| `act1.txt` | one session of one character through the fixture's act 1: area lines before the first level-up (held, then given to it), every signal kind, chat and a death in between. The target test lists the status every fixture quest ends in |
| `baseline.txt` | chat on every channel, a trade whisper, local chat shaped like an NPC line, and endgame town and map lines: nothing a quest or a character reacts to. No session marker, so it can follow another session |
| `party.txt` | an own character in a party: a member who joins then levels up (never switches), and an unknown name that levels up before leaving (active as unconfirmed, then demoted, its events back to the hold) |
| `held.txt` | a session with no character (its events dropped and counted), then a session whose held events go to the first character identified, not to a party member |
| `switch.txt` | a character switch inside one session, between two own characters |
| `manual_window.txt` | a manual quest unlocked by level whose window closes on entering a later area |
| `town_talk.txt` | the shape of the Live 1 session's start (2026-10-25), with an invented character: a level-1 entry to The Riverbank with no character build line after it (so, by the relog rule, a new character, named by its level-up, not a relog), The Bloated Miller, the level-up to 2, a visit to Clearfell Encampment in which the player talked to its NPCs and the log recorded no NPC line, then Clearfell. Unlike the others it is read against the shipped `data/quests.json`, by `town_talk_session_replay` in `crates/qt-engine/tests/shipped_data.rs`: what a town talk the log did not show leaves the talk steps at, and which later area entry completes them (research note "Town dialogue"). Since that visit logged no NPC line, its "changes no quest" check cannot fail; `town_voices.txt` is the one that can |
| `town_voices.txt` | a visit to Clearfell Encampment (`G1_town`) in which Renly, Finn and Una speak, with an invented character, between a level-1 entry to The Riverbank and Clearfell. Read against the shipped data by `town_voice_lines_move_no_quest` in `crates/qt-engine/tests/shipped_data.rs`: its positive control parses each voice line as an NPC line under an allowlist that names the three speakers, then with the shipped data no quest changes across them (research note "Town dialogue") |
| `riverbank_relog.txt` | the shape of Live 3's relog (2026-10-25), with an invented character and seeds: a level-1 entry to The Riverbank, the level-up to 2 that names the character, then a new session whose level-1 entry to The Riverbank has the same seed and no character build line after it, then Clearfell. Read by `replay_keeps_a_named_character_on_a_riverbank_relog` in `crates/qt-replay/tests/replay.rs`: the seed decides the entry is the named character's relog, so Clearfell's evidence is that character's and no new character opens; its negative control gives the second entry another seed, which opens a new character instead |
| `frames/` | `story.txt`, the one session every view fixture under `fixtures/viewmodels/` is built from by `crates/qt-viewmodel/tests/viewmodel.rs`, so the frames tell one story; its `README.md` tells that story |

`quests_min.json` beside this folder is the matching quest data: seven
invented quests covering every signal kind (`area` by code and by name,
`npc`, `system`, `reward` by text and by category, `points`, `level`,
`quest`) and every detection value (`auto`, `partial`, `manual`).

The files are LF; the engine tests split on `\n` and the parser strips a
trailing `\r`, so a CRLF checkout changes nothing.
