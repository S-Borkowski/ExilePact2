# body-armours-single-prefix walk items

What `tests/seeds_guides.rs` walks `body-armours-single-prefix` over
(issue #35). **Every item text here is constructed**, not copied from the
game, in the measured advanced layout (docs/RUNTIME_DATA_MODELS.md § "Item
text"), with affix names, tiers and value ranges taken from
`apps/crafting-helper/data/` so each modifier joins its data modifier.

The base is a Slipstrike Vest (a Dexterity body armour) at item level 82,
where increased Evasion Rating reaches T1 at 75 and T2 at 65. The targets
are increased Evasion Rating T1, Life T4 and flat Evasion Rating T4
(prefixes); Fire and Cold Resistance T3 and the desecrated reduced Curse
effect suffix (suffixes).

| File | What it is |
|---|---|
| `magic.txt` | Magic: increased Evasion Rating T1; Fire Resistance T3. The start. |
| `magic-weak.txt` | Magic: increased Evasion Rating T3, below its top two: `weak-magic-start`. |
| `rare-essence.txt` | `magic` after Greater Essence of Thawing: Cold Resistance T3, rare. |
| `rare-desecrated.txt` | `rare-essence` with an unrevealed desecrated suffix: the suffixes are full. |
| `rare-isolated.txt` | The Greater exalt with no side omen landed Life T4 and Evasion Rating T4. |
| `rare-isolate-miss.txt` | The same exalt landed Spirit and Physical Thorns instead: the miss. |
| `rare-complete.txt` | `rare-isolated` after the reveal offered the Curse effect suffix. |
