# Codex blueprint items

What `tests/seeds_codex.rs` walks `jewellery-catalyst-exalt` over (issue
#40, the domistae PoE2 Crafting Codex's catalyst-biased exalt). **Every
item text here is constructed**, not copied from the game, in the measured
advanced layout (docs/RUNTIME_DATA_MODELS.md § "Item text"), with affix
names, tiers and value ranges taken from `apps/crafting-helper/data/` so
each modifier joins its data modifier. Nothing here shows catalyst
quality: the helper does not read it, and the catalyst step is manual.

The targets: maximum Mana T5 and maximum Life T4 (prefixes), Fire
Resistance T5 and Cold Resistance T5 (suffixes); Cold Resistance is the
one missing.

| File | What it is |
|---|---|
| `01-start.txt` | Rare Amethyst Ring, item level 76: Mana T5, Life T4; Fire Resistance T5. Every modifier a target, one suffix target missing. A copy of `lord-omen/ring-2p1s.txt`. |
| `02-slam-cold.txt` | The slam hits: Cold Resistance T3 joins the suffixes. |
| `03-slam-strength.txt` | The slam misses: Strength T5, not a target, joins the suffixes. Also the start the blueprint refuses (a modifier that is not a target). |
| `04-magic-amulet.txt` | A Magic amulet, item level 82: Mana T2. Not rare: refused. A copy of `slots/magic-one-good.txt`. |
