# Last-target desecration items

What `tests/last_desecration.rs` plans and guides over (issue #32;
docs/models/crafting-slots-spec.md, "The last-target desecration"). **Every
item text here is constructed**, not copied from the game, in the measured
advanced layout (docs/RUNTIME_DATA_MODELS.md § "Item text"), with affix
names, tiers and value ranges taken from `apps/crafting-helper/data/` so
each modifier joins its data modifier.

The targets are the owner's belt's: maximum Life T2, maximum Mana T2,
Armour T2 (prefixes) and Fire Resistance T3 (suffix).

| File | What it is |
|---|---|
| `belt-2p1s.txt` | The owner's Heavy Belt, item level 82: Life T2, Mana T2; Fire Resistance T3. Armour is the one target missing. A copy of `ch-app`'s `tests/items/belt-2p1s.txt`. |
| `belt-unrevealed.txt` | The belt after the desecration: an unrevealed desecrated prefix. |
| `belt-revealed-armour.txt` | The reveal as Armour T2 (desecrated): done. |
| `belt-wrong-reveal.txt` | The reveal as the desecrated Charm charges prefix, not a target: Omen of Light's case. |
| `belt-desecrated-target.txt` | The belt with a revealed desecrated Strength and Dexterity suffix, a target: the item already has its desecration (issue #32's second test). |
| `belt-2p1s-level-64.txt` | The belt at item level 64, below every belt desecrated modifier's level 65: no desecration pool. |
