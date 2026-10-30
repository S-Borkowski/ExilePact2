# Essence swap items

What `tests/essence_swap.rs` plans over with the shipped
`fractured-essence-swap` (docs/models/crafting-slots-spec.md, "The essence
swap and an unrevealed modifier"). **Every item text here is constructed**,
not copied from the game: it is written in the measured advanced layout
(docs/RUNTIME_DATA_MODELS.md § "Item text"), with affix names, tiers and
value ranges taken from `apps/crafting-helper/data/` so each modifier joins
its data modifier. The baseline reads `../slots/rare-one-target.txt` with
`../rules/test-swap-plain.toml`.

| File | What it is |
|---|---|
| `ring-swap.txt` | Rare Amethyst Ring, item level 76: maximum Mana T5, Accuracy Rating T4 and Cold Damage T3 (the prefixes full); a fractured Fire Resistance T5, an unrevealed desecrated suffix and Dexterity T7 (the suffixes full). Essence of Hysteria takes the unrevealed suffix or Dexterity: the add is certain, the unrevealed one survives 1 in 2. |
| `ring-swap-goes.txt` | The ring after the swap took the unrevealed suffix: fractured Fire Resistance, Dexterity and Mana Regeneration Rate T2, the suffixes full again. |
| `ring-swap-annulled-regen.txt` | The ring after the annulment that should have freed a slot took Mana Regeneration Rate instead: fractured Fire Resistance and Dexterity, one suffix slot free. |
| `quiver-swap.txt` | Rare Broadhead Quiver, item level 76: a fractured Projectile Speed T4, an unrevealed desecrated prefix and Accuracy Rating T4 (the prefixes full); Attack Speed T2 and Dexterity T3. The prefix case, Hysteria adding Damage with Bow Skills T2. |
| `dusk-ring-swap.txt` | Rare Dusk Ring (`+1 Prefix Modifier allowed`, `-1 Suffix Modifier allowed`), item level 76: maximum Mana T5 and Accuracy Rating T4; a fractured Fire Resistance T5 and Dexterity T7, its two suffixes full. The removal has one modifier to take. |
