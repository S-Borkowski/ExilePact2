# The owner's 2P/1S belt

The item states `tests/state_order.rs`, `tests/remarks.rs` and
`tests/fixtures.rs` drive the state-ordered `close-a-side` over: the
owner's craft of 2026-10-29, a rare with two target prefixes and one
target suffix, a prefix target still missing. **Every file here is
constructed**, not copied from the game: it is written in the measured
advanced layout (docs/RUNTIME_DATA_MODELS.md § "Item text"), on the Heavy
Belt, item level 82, of `ch-engine`'s slot case `rare-no-target`, with
affix names, tiers and value ranges taken from
`apps/crafting-helper/data/classes/belts.json` so each modifier joins its
data modifier. Its modifier levels (the data tier's item level) decide
what a whittle takes; only their order matters.

Targets: maximum Life T2 and maximum Mana T2 (on it), Armour T2 (missing);
Fire Resistance T3 (on it).

| File | State |
|---|---|
| `belt-2p1s.txt` | Rare: Life T2 (level 60), Mana T2 (54); Fire Resistance T3 (60). The prefix side's one free slot is reserved for Armour; the suffix side can be closed. |
| `belt-closed.txt` | The suffix side closed: plus Cold Resistance T2 (71) and Strength T3 (66). |
| `belt-slam-miss.txt` | The closed belt plus Flask Life Recovery T1 (75): the slam on the prefix side missed, above the lowest level (Mana, a target, is the lowest). |
| `belt-slam-miss-low.txt` | The closed belt plus Flask Mana Recovery T4 (26): the slam missed, at the lowest level. |
| `belt-done.txt` | The closed belt plus Armour T2: every target on it. |

`tests/craft_window.rs` also walks `essence-desecrate-double-slam` on the
magic Heavy Belt of `ch-engine`'s relaxation tests
(`crates/ch-engine/tests/items/relax/belt-magic-life.txt`: maximum Life
T2), with Fire Resistance T3, Cold and Lightning Resistance T2 as the
targets, through two more constructed states:

| File | State |
|---|---|
| `belt-magic-essence.txt` | Rare after Greater Essence of Insulation: Life T2; Fire Resistance T3. |
| `belt-magic-desecrated.txt` | The same plus an unrevealed desecrated suffix: the reveal is the current step. |

`tests/craft_window.rs` and `tests/fixtures.rs` (`craft-essence-swap`)
show the shipped `fractured-essence-swap` on two rings copied unchanged
from `ch-engine`'s `crates/ch-engine/tests/items/essence-swap/`
(constructed; that folder's README says how):

| File | State |
|---|---|
| `ring-essence-swap.txt` | Its `ring-swap.txt`: an Amethyst Ring, item level 76, three prefixes; its suffixes a fractured Fire Resistance, an unrevealed desecrated suffix and Dexterity. Targets: maximum Mana T5, Mana Regeneration Rate T2 (Essence of Hysteria's) and a desecrated Cold and Chaos Resistances suffix. |
| `dusk-ring-essence-swap.txt` | Its `dusk-ring-swap.txt`: a Dusk Ring (`-1 Suffix Modifier allowed`), its two suffixes a fractured Fire Resistance and Dexterity. Targets: maximum Mana T5 and Mana Regeneration Rate T2. |

The tests also read `ch-engine`'s slot case items and walk states by path
(`crates/ch-engine/tests/items/slots/`, constructed): issue #28's ring
(`rare-one-target`) and the magic amulet's augment and essence walk.
