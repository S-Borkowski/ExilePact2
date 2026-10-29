# `time-lost-jewel-radius` item states

What `tests/seeds_jewels_weapons.rs` walks the `time-lost-jewel-radius` seed
over. **Every item text here is constructed**, not copied from the game: it
is written in the measured advanced layout (docs/RUNTIME_DATA_MODELS.md
§ "Item text"), with affix names, tiers and value ranges taken from
`apps/crafting-helper/data/classes/jewels.json` (the `time-lost-sapphire`
variant) so each modifier joins its data modifier. The Contempt and radius
modifiers' headers (`{ Prefix Modifier }`, no name or tier) are ours: how
the game heads a liquid emotion's modifier is not measured, and neither
joins the data.

A Rare Time-Lost Sapphire, item level 82. Targets: the three radius
critical suffixes (Critical Hit Chance for Spells, Critical Spell Damage
Bonus, Critical Damage Bonus) and the desecrated increased maximum Mana
prefix.

| File | What it is |
|---|---|
| `0-start.txt` | A small-passive Cold Damage prefix (unwanted); the two spell critical suffixes. |
| `1-contempt.txt` | The Ancient liquid took the Cold Damage prefix and added Contempt as a prefix (+1 suffix allowed). |
| `1-contempt-took-wanted.txt` | Miss: the liquid took the Critical Hit Chance for Spells suffix. |
| `2-exalted.txt` | A Dextral exalt added the Critical Damage Bonus target. |
| `2-exalted-other.txt` | Miss: the exalt added the Minions Critical Hit Chance suffix. |
| `3-contempt-removed.txt` | The Contempt prefix annulled. |
| `4-desecrated.txt` | An unrevealed desecrated prefix added. |
| `5-radius.txt` | The Melancholy liquid added the Very Large radius and left the unrevealed one. |
| `5-radius-unsealed.txt` | After a bad reveal, the Omen of Light removed the desecrated prefix. |
| `6-revealed.txt` | Revealed as the maximum Mana target. |
| `6-revealed-other.txt` | Miss: revealed as the maximum Life desecrated prefix. |

The Dextral annulment after the bad exalt gives `1-contempt.txt` back.
