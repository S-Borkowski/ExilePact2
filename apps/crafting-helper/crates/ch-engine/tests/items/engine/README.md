# Engine test items and blueprints

What `tests/engine.rs` plans and guides over. **Every item text here is
constructed**, not copied from the game: it is written in the measured
advanced layout (docs/RUNTIME_DATA_MODELS.md § "Item text"), with affix
names, tiers and value ranges taken from `apps/crafting-helper/data/` so
each modifier joins its data modifier. The real copies the tests also use
are `ch-item`'s, under `crates/ch-item/fixtures/real/`.

## Items (constructed)

| File | What it is |
|---|---|
| `ring-rare-mana-fire.txt` | Rare Amethyst Ring, item level 76: maximum Mana T5 (prefix), Fire Resistance T5 (suffix). The start of the guide tests. |
| `ring-rare-life-hit.txt` | The same plus maximum Life T4: an exalt that hits a life target at floor T5. |
| `ring-rare-accuracy-miss.txt` | The same plus Accuracy Rating T3: an exalt that misses. |
| `ring-rare-life-below-floor.txt` | The same plus maximum Life T7: below a T5 floor. |
| `ring-rare-fire-swapped.txt` | Fire Resistance replaced by Cold Resistance T5: one out, one in, which no exalt does. |
| `ring-rare-fire-annulled.txt` | The accuracy miss with Fire Resistance removed: an annulment that took the wanted modifier. |
| `gloves-magic-cold.txt` | Magic Sirenscale Gloves (int), item level 81, Cold Resistance T5: an essence can make it Rare. |
| `gloves-rare-cold.txt` | The same item Rare. |
| `jewels-rare-sapphire.txt` | Rare Sapphire, item level 82, two modifiers, not corrupted (the real one is): jewel pages have no spawn weights. |

## Test blueprints

Valid blueprints for the tests only (never shipped), checked and resolved
the same way as the shipped ones:

- `test-slam.toml` (Rings): exalt for any remaining target (Perfect, then
  Greater, then plain), a miss goes to an annulment, a hit to a manual
  finish.
- `test-desecrate.toml` (Rings): desecrate on the hardest target's side
  (`side-omen`), then reveal.
- `test-essence.toml` (Gloves): an essence step bound by `essence` and
  taken only when bound, then an exalt; a Magic start needs an essence
  binding.
- `test-jewel.toml` (Jewels): one exalt.
