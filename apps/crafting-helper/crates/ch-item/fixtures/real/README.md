# Real item texts

Only texts the owner copied in the game, byte for byte, belong here: one item
per `.txt` file, named `<class>-<rarity>[-<feature>]-<form>.txt`. Nothing here
is edited after it is pasted. `tests/fixtures.rs`
(`every_real_fixture_parses_without_unparsed_lines`) parses every file in this
directory; any line the parser reports is a shape the constructed fixtures
guessed wrong.

| File | Copy | Date | Owner's description |
|---|---|---|---|
| `gloves-rare-rune-desecrated-ctrl-alt-c.txt` | `Ctrl+Alt+C` | 2026-10-28 | Rare Sirenscale Gloves, item level 81, 20% quality, one rune socketed (18% increased Armour, Evasion and Energy Shield); three prefixes and three suffixes, the suffix "of Exile" (chaos resistance) desecrated and revealed; nothing fractured. |
| `gloves-rare-rune-desecrated-ctrl-c.txt` | `Ctrl+C` | 2026-10-28 | The same gloves copied with plain `Ctrl+C`: the text is identical to the `Ctrl+Alt+C` copy, so plain `Ctrl+C` also gives the advanced form (owner: "both were copied as said"). |
