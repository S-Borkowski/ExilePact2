# `caster-cast-speed` item states

What `tests/seeds_jewels_weapons.rs` walks the `caster-cast-speed` seed
over. **Every item text here is constructed**, not copied from the game: it
is written in the measured advanced layout (docs/RUNTIME_DATA_MODELS.md
§ "Item text"), with affix names, tiers and value ranges taken from
`apps/crafting-helper/data/classes/wands.json` so each modifier joins its
data modifier. The alloy modifier's header (`{ Suffix Modifier }`, no name
or tier) is ours: how the game heads an alloy's modifier is not measured. It
joins nothing, and the engine accepts it because its lines are the alloy's
own.

A Rare Volatile Wand, item level 82. Targets: increased Spell Damage at T4
or better (on the wand from the start), increased Critical Hit Chance for
Spells at T4 or better, and the desecrated increased Elemental Damage
prefix.

| File | What it is |
|---|---|
| `0-start.txt` | Increased Spell Damage at T3. |
| `1-filler.txt` | A Sinistral exalt added an extra Fire Damage prefix (filler). |
| `2-cast-speed.txt` | The Transcendent Alloy took the filler and added its cast speed and extra cold suffix. |
| `3-blocked.txt` | An unrevealed desecrated prefix holds a slot. |
| `4-double-slam.txt` | The Perfect exalt with Greater Exaltation added the Critical Hit Chance for Spells target and a Mana Regeneration suffix. |
| `5-unblocked.txt` | The Omen of Light removed the unrevealed prefix. |
| `6-desecrated.txt` | The last desecration: an unrevealed prefix again. |
| `7-revealed.txt` | Revealed as the Elemental Damage target. |
| `7-revealed-other.txt` | Miss: revealed as the extra Physical Damage desecrated prefix. |

The Omen of Light after the bad reveal gives `5-unblocked.txt` back. The
miss path also builds, in the test, a `4-double-slam.txt` whose critical
suffix is an Intelligence one instead: a double exalt that brings nothing
wanted.
