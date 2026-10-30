# full-rare-desecration-rescue walk items

What `tests/seeds_guides.rs` walks `full-rare-desecration-rescue` over
(issue #35). **Every item text here is constructed**, not copied from the
game, in the measured advanced layout (docs/RUNTIME_DATA_MODELS.md § "Item
text"), with affix names, tiers and value ranges taken from
`apps/crafting-helper/data/` so each modifier joins its data modifier.
Boots have no desecrated prefix, so a revealed prefix is an ordinary one
shown as desecrated, as `tests/items/last-desecration/` writes it.

The base is a Cavalry Boots (a Dexterity base) at item level 82. The
targets are Movement Speed T2 (missing), Life T3, Evasion Rating T3, Fire
Resistance T3 and Cold Resistance T3; Mana and Dexterity are not targets.

| File | What it is |
|---|---|
| `rare-six.txt` | Life T3, Evasion Rating T3, Mana T3; Fire, Cold Resistance and Dexterity T3. The start. |
| `rare-five.txt` | `rare-six` less Mana: no start fits; also the item after Omen of Light. |
| `desecrated.txt` | The bone took Mana off the full prefixes: an unrevealed desecrated prefix in its place. |
| `desecrated-lost-target.txt` | The bone took Life, a target, off instead: the miss. |
| `revealed.txt` | The reveal offered Movement Speed T2. |
| `revealed-wrong.txt` | The reveal offered Mana, not a target. |
