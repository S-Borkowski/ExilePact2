# Synthetic sessions for the quest engine

Hand-written `Client.txt` sessions for the `qt-engine` tests
(`crates/qt-engine/tests/engine.rs`) and the `qt-viewmodel` tests
(`crates/qt-viewmodel/tests/viewmodel.rs`), read with the synthetic quest data in
`fixtures/quests_min.json`. The line shapes are the parser's
(`fixtures/logs/README.md`); nothing here comes from a real log. Character
names, chat text, timestamps and the NPC "lines" are invented, and there are
no IP addresses.

| File | What it holds |
|---|---|
| `act1.txt` | one session of one character through the fixture's act 1: area lines before the first level-up (held, then given to it), every signal kind, chat and a death in between. The target test lists the status every fixture quest ends in |
| `baseline.txt` | chat on every channel, a trade whisper, local chat shaped like an NPC line, and endgame town and map lines: nothing a quest or a character reacts to. No session marker, so it can follow another session |
| `party.txt` | an own character in a party: a member who joins then levels up (never switches), and an unknown name that levels up before leaving (active as unconfirmed, then demoted, its events back to the hold) |
| `held.txt` | a session with no character (its events dropped and counted), then a session whose held events go to the first character identified, not to a party member |
| `switch.txt` | a character switch inside one session, between two own characters |
| `manual_window.txt` | a manual quest unlocked by level whose window closes on entering a later area |
| `frames/` | `story.txt`, the one session every view fixture under `fixtures/viewmodels/` is built from by `crates/qt-viewmodel/tests/viewmodel.rs`, so the frames tell one story; its `README.md` tells that story |

`quests_min.json` beside this folder is the matching quest data: seven
invented quests covering every signal kind (`area` by code and by name,
`npc`, `system`, `reward` by text and by category, `points`, `level`,
`quest`) and every detection value (`auto`, `partial`, `manual`).

The files are LF; the engine tests split on `\n` and the parser strips a
trailing `\r`, so a CRLF checkout changes nothing.
