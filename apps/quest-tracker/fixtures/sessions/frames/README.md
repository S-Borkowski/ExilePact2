# The session behind the view fixtures

`story.txt` is the one session every view fixture under
`fixtures/viewmodels/` is built from (`crates/qt-viewmodel/tests/viewmodel.rs`,
`fixture_values`), so the frames tell one story. It is synthetic, like the
sessions beside this folder: the line shapes are the parser's
(`fixtures/logs/README.md`), and the character names, timestamps and NPC
"lines" are invented.

It holds two sessions:

1. `Odile_Marsh` reaches level 3 and receives a stat, which makes her an own
   character: the character picker's second entry.
2. `Corisande_Fell` walks the riverbank to the encampment (Renly's line
   finishes the riverbank and unlocks the king), enters Clearfell (the pack
   leader becomes available), reaches level 5 (the candles unlock), enters
   Freythorn (the king's first step; the candles' window closes) and returns
   to town, where Una's line makes the lute available. That last line is
   the one the test feeds live for the frames' row change
   (`story_row_change`).

The test then skips the pack leader, so act 1 holds one quest in each status:
riverbank done, pack leader skipped, bell locked, king in progress, lute
available, candles unknown. Worth is varied on the test's own copy of
`fixtures/quests_min.json` (king `optional`, pack leader `no`); the file
itself, and the sessions beside this folder, are what the engine, replay and
shell tests read, and stay as they are.

The file is LF.
