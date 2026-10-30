# fracture-first walk items

What `tests/seeds_guides.rs` walks `fracture-first` over (issue #35).
**Every item text here is constructed**, not copied from the game, in the
measured advanced layout (docs/RUNTIME_DATA_MODELS.md § "Item text"), with
affix names, tiers and value ranges taken from `apps/crafting-helper/data/`
so each modifier joins its data modifier. How the game shows the Mark of
the Abyssal Lord (its header, and whether it is marked crafted) is not
measured; its line is the data's line for Essence of the Abyss, as in
`tests/items/fracture/`.

The base is a Cavalry Boots (a Dexterity base) at item level 82. The
targets are Movement Speed T2 (prefix, on the item), Evasion Rating T3
(prefix), Fire Resistance T3 and Chaos Resistance T3 (suffixes; Chaos,
spawn weight 250, is the hardest).

| File | What it is |
|---|---|
| `rare-four.txt` | Movement Speed T2, Life T3; Dexterity T3, Lightning Resistance T3. The start. |
| `rare-five.txt` | `rare-four` and a Cold Resistance T3 suffix: five modifiers, no start fits. |
| `abyss.txt` | `rare-four` after Essence of the Abyss: the mark in Life's place. |
| `abyss-lost-target.txt` | `rare-four` after an essence that took Movement Speed: the miss. |
| `fractured.txt` | `abyss` with Movement Speed fractured. |
| `annulled-once.txt` | `fractured` less Dexterity. |
| `annulled-twice.txt` | `annulled-once` less the mark: the fractured target and one more. |
| `chaos.txt` | A Chaos Orb turned Lightning Resistance into Fire Resistance T3. |
| `desecrated.txt` | `chaos` with an unrevealed desecrated suffix. |
| `revealed.txt` | The reveal offered Chaos Resistance T3. |
| `complete.txt` | `revealed` with Evasion Rating T3 from a Sinistral Perfect exalt. |
