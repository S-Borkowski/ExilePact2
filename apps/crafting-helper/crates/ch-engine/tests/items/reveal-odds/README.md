# Reveal odds items and blueprints

What `tests/reveal_odds.rs` plans over (issue #49;
docs/models/crafting-slots-spec.md, "Reveal odds"), beside the owner's belt
in `../last-desecration/` and the ring in `../engine/`. **Every item text
here is constructed**, not copied from the game, in the measured advanced
layout (docs/RUNTIME_DATA_MODELS.md § "Item text"), with affix names, tiers
and value ranges taken from `apps/crafting-helper/data/` so each modifier
joins its data modifier.

## Items (constructed)

| File | What it is |
|---|---|
| `belt-unrevealed-level-64.txt` | `../last-desecration/belt-2p1s-level-64.txt` with an unrevealed desecrated prefix: below every belt exclusive modifier's level 65, so its reveal is three exalt-like draws. |
| `body-armour-unrevealed-prefix.txt` | `../slots/body-armour-prefix-no-pool.txt` with an unrevealed desecrated prefix: body armours have no exclusive prefix at any level (`necromancy-without-pool`). |
| `jewel-unrevealed-prefix.txt` | `../engine/jewels-rare-sapphire.txt` with an unrevealed desecrated prefix: a jewel's ordinary target has no spawn weight, its exclusive one needs none. |

## Test blueprints

Valid blueprints for the tests only (never shipped), listed order:

- `test-reveal.toml`: one plain reveal, aimed at every remaining target.
- `test-reveal-echoes.toml`: the same reveal with Omen of Abyssal Echoes.
- `test-prefix-exalt.toml`: one Exalted Orb with Omen of Sinistral
  Exaltation, the prefix exalt chance an ordinary reveal draw is compared
  with.
