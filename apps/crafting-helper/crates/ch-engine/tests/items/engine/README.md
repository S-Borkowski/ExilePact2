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
| `ring-state-closed-suffix.txt` | `ring-rare-life-hit.txt` plus Cold and Lightning Resistance T5: the suffix side closed by a Greater exalt. |
| `ring-state-one-suffix-free.txt` | `ring-rare-life-hit.txt` plus Cold Resistance T5: one free suffix slot. |
| `ring-state-complete.txt` | The closed ring plus Accuracy Rating T3: every target of the 2P/1S craft on it. |
| `ring-state-prefix-blocked.txt` | Rare ring: Mana T5, Accuracy Rating T3 and Cold Damage T3 (prefixes), Fire Resistance T5 (suffix, the lowest modifier level). With Life missing the prefix side is blocked. |
| `ring-state-whittle.txt` | The same with Rarity of Items T3 (modifier level 10) for Cold Damage: the one lowest-level modifier is a junk prefix. |
| `ring-state-whittled.txt` | The whittle's hit: the Rarity prefix gone, Cold Resistance T5 come. |
| `ring-state-one-target.txt` | Issue #28's shape: Mana T5 and Accuracy Rating T3 (prefixes), Cold Resistance T5 and Dexterity T4 (suffixes). |
| `ring-state-no-target.txt` | Accuracy Rating T3 and Rarity of Items T3 (prefixes), Cold Resistance T5 and Dexterity T4 (suffixes): no target on it. |
| `ring-state-desecrated-target.txt` | Mana T5, Life T3, Fire Resistance T4 and a revealed desecrated Chaos Resistance T3 (a copy of `gear/fill-desecrate-exalt/05-revealed-chaos.txt`). |

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
- `test-literal-exalt.toml` (Rings): a suffix exalt that writes Omen of
  Dextral Exaltation literally (Perfect, then Greater), then a manual
  finish; `tests/full_side_omen.rs` takes it on a ring whose prefixes are
  full and on one with room.

State-ordered test blueprints (`order = "state"`, `tests/state_order.rs`):

- `test-state-close.toml`: close-a-side's shape: remove (whittle, side
  annulment, annulment) on the blocked side, fill the closable side
  (`greater-omen`, `side-omen`), slam the one open side, slam a target's
  side.
- `test-state-clean.toml`: rare-clean-then-slam's shape: remove, then a
  Perfect Exalted Orb with `greater-omen` and `side-omen` on a target's
  side.
- `test-state-unrestricted.toml`: a plain Exalted Orb before the side one,
  so the passed-over plain exalt shows.
- `test-state-desecrate.toml`: reveal, desecrate the hardest target's side
  (before the clean-up, so a full side shows), remove, slam.
- `test-state-prefix-only.toml`, `test-state-loop.toml` and
  `test-state-light.toml`: a route that cannot finish, for no candidate,
  the step cap and no change.
