# `boots-runic-ward` item states

What `tests/seeds_jewels_weapons.rs` walks the `boots-runic-ward` seed
over. **Every item text here is constructed**, not copied from the game: it
is written in the measured advanced layout (docs/RUNTIME_DATA_MODELS.md
§ "Item text"), with affix names, tiers and value ranges taken from
`apps/crafting-helper/data/classes/boots.json` (the `dex_int` variant) so
each modifier joins its data modifier. The Sovereign Alloy modifier's
header (`{ Prefix Modifier }`, no name or tier) is ours: how the game heads
it is not measured. It joins nothing, and the engine accepts it because its
line is the alloy's own. The post's Essence of Horror step is not in the
blueprint (since 0.5 an item holds one crafted modifier, and the alloy's is
it), so no state carries its augment effect suffix.

Rare Frayed Shoes, item level 81. Targets: increased Movement Speed at T2
or better (fractured on the boots from the start), Dexterity and
Intelligence at T4 or better.

| File | What it is |
|---|---|
| `0-start.txt` | Fractured Movement Speed at T2; a Stun Threshold suffix (unwanted). |
| `1-chaos-other.txt` | Miss: the chaos swapped Stun Threshold for a Deflection suffix. |
| `1-chaos.txt` | Dexterity at T3 instead. |
| `2-filler.txt` | A Sinistral exalt added a maximum Life prefix (filler). The rune step leaves it as it is. |
| `3-runic-ward.txt` | The Sovereign Alloy took it and added its Runic Ward prefix. |
| `4-desecrated.txt` | An unrevealed desecrated suffix added. |
| `5-revealed.txt` | Revealed as Intelligence at T3 (a normal-pool modifier, which a reveal may offer). |
| `5-revealed-other.txt` | Miss: revealed as the reduced Bleeding duration desecrated suffix. |
| `6-last-prefix.txt` | A last Sinistral exalt added a maximum Life prefix. |

The Omen of Light after the bad reveal gives `3-runic-ward.txt` back.
