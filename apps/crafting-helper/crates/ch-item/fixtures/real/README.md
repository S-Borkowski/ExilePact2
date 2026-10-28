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
| `rings-normal-implicit-advanced.txt` | not stated (advanced form) | 2026-10-28 | Normal Amethyst Ring, item level 76: one implicit (chaos resistance) under a nameless `{ Implicit Modifier — tags }` header. The owner pasted it without a description; this one is read off the text. |
| `boots-magic-hybrid-unmet-advanced.txt` | not stated (advanced form) | 2026-10-28 | Magic Drakeskin Boots, item level 81: prefix "Hummingbird's" with two lines (evasion and stun threshold) under one header with no tags, suffix "of the Phantom"; a Dex requirement marked `(unmet)`. Read off the text. |
| `crossbows-rare-desecrated-damage-advanced.txt` | not stated (advanced form) | 2026-10-28 | Rare Siege Crossbow, item level 81: weapon properties with a `(fire)` damage line, an `{ Implicit Modifier }` with no tags in its own section, a desecrated prefix "Incinerating" with a two-value `Adds … to …` line, a decimal range (`3.33(3.11-3.8)`), an `(unmet)` Dex requirement; nothing fractured. Read off the text. |
| `jewels-rare-corrupted-enhancement-advanced.txt` | not stated (advanced form) | 2026-10-28 | Rare Sapphire jewel, item level 82, corrupted: a `{ Corruption Enhancement — tags }` header (no `Modifier` word) in its own section, two prefixes and two suffixes, a description line, `Corrupted` last. Read off the text. |
