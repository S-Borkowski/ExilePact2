# `boots-runic-ward` item states

What `tests/seeds_jewels_weapons.rs` walks the `boots-runic-ward` seed
over. **Every item text here is constructed**, not copied from the game: it
is written in the measured advanced layout (docs/RUNTIME_DATA_MODELS.md
§ "Item text"), with affix names, tiers and value ranges taken from
`apps/crafting-helper/data/classes/boots.json` (the `dex_int` variant) so
each modifier joins its data modifier. The Essence of Horror and Sovereign
Alloy modifiers' headers (`{ Suffix Modifier }`, `{ Prefix Modifier }`, no
name or tier) are ours: how the game heads them is not measured. They join
nothing, and the engine accepts them because their lines are the essence's
and the alloy's own.

Rare Frayed Shoes, item level 81. Targets: increased Movement Speed at T2
or better (fractured on the boots from the start), Dexterity and
Intelligence at T4 or better.

| File | What it is |
|---|---|
| `0-start.txt` | Fractured Movement Speed at T2; a Stun Threshold suffix (unwanted). |
| `1-chaos-other.txt` | Miss: the chaos swapped Stun Threshold for a Deflection suffix. |
| `1-chaos.txt` | Dexterity at T3 instead. |
| `2-filler-one.txt` | A Sinistral exalt added an Evasion and Energy Shield prefix (filler). |
| `3-augment.txt` | Essence of Horror took the filler and added its augment effect suffix. |
| `4-filler-two.txt` | A maximum Life prefix (filler). The rune step leaves it as it is. |
| `5-runic-ward.txt` | The Sovereign Alloy took it and added its Runic Ward prefix. |
| `6-desecrated.txt` | An unrevealed desecrated suffix added. |
| `7-revealed.txt` | Revealed as Intelligence at T3 (a normal-pool modifier, which a reveal may offer). |
| `7-revealed-other.txt` | Miss: revealed as the reduced Bleeding duration desecrated suffix. |
| `8-last-prefix.txt` | A last Sinistral exalt added a maximum Life prefix. |

The Omen of Light after the bad reveal gives `5-runic-ward.txt` back.
