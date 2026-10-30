# Route rule items and test blueprints

What `tests/rules.rs` plans over: the rules from the crafting guides
(issue #35; docs/models/crafting-slots-spec.md, "Certain removals first"
and "Rejections"). **Every item text here is constructed**, not copied
from the game: it is written in the measured advanced layout
(docs/RUNTIME_DATA_MODELS.md § "Item text"), with affix names, tiers and
value ranges taken from `apps/crafting-helper/data/` so each modifier joins
its data modifier. The tests also read `../engine/` and `../slots/` items.

## Items (constructed)

| File | What it is |
|---|---|
| `boots-magic-movement-t3.txt` | Magic Cavalry Boots (dex), item level 82: increased Movement Speed T3. Its top two tiers at that level are T1 (level 82) and T2 (level 65), so a route that upgrades it is rejected (`weak-magic-start`). |
| `boots-magic-movement-t2.txt` | The same with Movement Speed T2, a top-two tier: the route stays. |
| `boots-magic-life-t3.txt` | The same boots with maximum Life T3 instead: below its top two too, but judged only when it is a target. |
| `ring-full-suffix.txt` | Rare Amethyst Ring, item level 76: maximum Mana T5 and Accuracy Rating T4; Cold Resistance T5, Dexterity T7 and Fire Resistance T5, the suffixes full. |

## Test blueprints

| File | What it tests |
|---|---|
| `test-jewel-grades.toml` | Perfect, Greater, then plain Exalted Orb for jewels and rings: a jewel takes the plain orb, a ring keeps the Perfect one. |
| `test-weak-magic.toml` | Listed: a Regal Orb on a magic pair of boots. |
| `test-weak-magic-state.toml` | State-ordered: Greater Essence of Insulation on a magic pair of boots, then a slam. |
| `test-removal-order.toml` | State-ordered: a slam listed before a clean-up, so Certain removals first decides the order. |
| `test-swap-omen.toml` | Listed: Essence of Hysteria (a special essence) with Omen of Dextral Crystallisation on a rare ring. |
| `test-swap-plain.toml` | Listed: the same essence with no Crystallisation omen. |
| `test-swap-order.toml` | State-ordered: an annulment, then the crystallised Essence of Hysteria, then a slam. |
