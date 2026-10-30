# weapons-physical-generic walk items

What `tests/seeds_guides.rs` walks `weapons-physical-generic` over (issue
#35). **Every item text here is constructed**, not copied from the game, in
the measured advanced layout (docs/RUNTIME_DATA_MODELS.md § "Item text"),
with affix names, tiers and value ranges taken from
`apps/crafting-helper/data/` so each modifier joins its data modifier.

The base is an Aberrant Sledge (Two Hand Maces, no implicit) at item level
82, where increased Physical Damage reaches T1 at 82 and T2 at 75. The
targets are increased Physical Damage T2, flat Physical Damage T3 (Greater
Essence of Abrasion's tier) and Accuracy Rating T3 (prefixes); Attack
Speed, Critical Damage Bonus and Strength, T3 each (suffixes).

| File | What it is |
|---|---|
| `magic.txt` | Magic: increased Physical Damage T2, a top-two tier, and no suffix. The start. |
| `magic-weak.txt` | Magic: increased Physical Damage T3, below its top two: `weak-magic-start`. |
| `augmented.txt` | `magic` after an augment: Attack Speed T3. |
| `augment-miss.txt` | `magic` after an augment that landed Stun Duration: the miss. |
| `rare.txt` | `augmented` after Greater Essence of Abrasion: flat Physical Damage T3, rare. |
| `slammed.txt` | `rare` with Critical Damage Bonus and Strength from the Greater, Dextral exalt. |
| `desecrated.txt` | `slammed` with an unrevealed desecrated prefix. |
| `complete.txt` | The reveal offered Accuracy Rating T3. |
