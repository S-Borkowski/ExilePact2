# `weapons-alloy-budget` item states

What `tests/seeds_jewels_weapons.rs` walks the `weapons-alloy-budget` seed
over. **Every item text here is constructed**, not copied from the game: it
is written in the measured advanced layout (docs/RUNTIME_DATA_MODELS.md
§ "Item text"), with affix names, tiers and value ranges taken from
`apps/crafting-helper/data/classes/one-hand-maces.json` so each modifier
joins its data modifier. The two alloy modifiers' headers (`{ Prefix
Modifier }`, `{ Suffix Modifier }`, no name or tier) are ours: how the game
heads an alloy's modifier is not measured. They join nothing, and the engine
accepts them because their lines are the alloys' own.

A Rare Wooden Club, item level 82. Targets: increased Physical Damage at T2
or better (the hardest, so the chaos step's), Adds Physical Damage at T3 or
better, and the desecrated Warcry Cooldown Recovery Rate suffix.

| File | What it is |
|---|---|
| `0-start.txt` | One Accuracy Rating prefix (unwanted). |
| `1-chaos-below-floor.txt` | Miss: the chaos rolled increased Physical Damage at T5, below the T2 floor. |
| `1-chaos.txt` | Increased Physical Damage at T2. |
| `2-filler-one.txt` | A Dextral exalt added a Strength suffix (filler). |
| `3-celestial.txt` | The Celestial Alloy took the filler and added its accuracy and attack speed prefix. |
| `4-filler-two.txt` | A second Strength suffix (filler). |
| `5-transcendent.txt` | The Transcendent Alloy took it and added its physical damage and attributes suffix. |
| `6-slam.txt` | A Sinistral Perfect exalt added Adds Physical Damage at T2. |
| `7-desecrated.txt` | An unrevealed desecrated suffix added. |
| `8-revealed.txt` | Revealed as the Warcry Cooldown Recovery Rate target. |
| `8-revealed-other.txt` | Miss: revealed as the Totem Placement speed desecrated suffix. |

The Omen of Light after the bad reveal gives `6-slam.txt` back.
