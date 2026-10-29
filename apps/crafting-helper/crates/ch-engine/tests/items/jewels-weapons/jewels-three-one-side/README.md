# `jewels-three-one-side` item states

What `tests/seeds_jewels_weapons.rs` walks the `jewels-three-one-side` seed
over. **Every item text here is constructed**, not copied from the game: it
is written in the measured advanced layout (docs/RUNTIME_DATA_MODELS.md
§ "Item text"), with affix names, tiers and value ranges taken from
`apps/crafting-helper/data/classes/jewels.json` so each modifier joins its
data modifier. The Contempt modifier's header (`{ Suffix Modifier }`, no
name or tier) is ours: how the game heads a liquid emotion's modifier is not
measured. It joins nothing, and the engine accepts it because its line is
the liquid's own.

A Rare Sapphire, item level 82. Targets: increased Spell Damage and
increased maximum Energy Shield (the two prefixes on the jewel), the
desecrated increased Exposure Effect prefix, and increased Critical Hit
Chance for Spells (a suffix for the chaos step).

| File | What it is |
|---|---|
| `0-start.txt` | Spell Damage and Energy Shield prefixes; Curse Duration and Mana Regeneration suffixes (unwanted). |
| `1-contempt.txt` | The liquid took Mana Regeneration and added Contempt as a suffix (+1 prefix allowed). |
| `1-contempt-took-wanted.txt` | Miss: the liquid took Spell Damage (Contempt landed as a prefix). |
| `2-desecrated.txt` | An unrevealed desecrated prefix added. |
| `3-revealed.txt` | Revealed as the Exposure Effect target. |
| `3-revealed-other.txt` | Miss: revealed as the Armour and Attack Damage desecrated prefix. |
| `4-cleaned.txt` | The Contempt suffix annulled. |
| `5-filled.txt` | An exalt filled the free suffix slot (Damage taken from Mana before Life). |
| `6-rerolled.txt` | A chaos on the suffixes swapped Curse Duration for the Critical Hit Chance for Spells target. |

The Omen of Light after the bad reveal gives `1-contempt.txt` back.
