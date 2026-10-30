# Lord omen items

What `tests/lord_omen.rs` plans and guides over (issue #40;
docs/models/crafting-slots-spec.md, "`lord-omen`"). **Every item text here
is constructed**, not copied from the game, in the measured advanced
layout (docs/RUNTIME_DATA_MODELS.md § "Item text"), with affix names, tiers
and value ranges taken from `apps/crafting-helper/data/` so each modifier
joins its data modifier. Each is a copy of an item another test already
uses, kept here so this test's inputs stay put when that one changes.

Every desecrated target the test names has one tier, at modifier level 65,
whose name names one lord: the Ring's increased Attack Damage while on Low
Life prefix is `Ulaman's`, the Belt's and the Body Armour's Strength and
Dexterity suffix is `of Ulaman`, the Bow's lightning penetration prefix is
`Ulaman's`.

| File | What it is | A copy of |
|---|---|---|
| `ring-2p1s.txt` | Rare Amethyst Ring, item level 76: Mana T5, Life T4; Fire Resistance T5. One prefix slot free. | `slots/rare-2p1s-prefix-missing.txt` |
| `belt-2p1s.txt` | Rare Heavy Belt, item level 82: Life T2, Mana T2; Fire Resistance T3. One suffix slot free for the desecrated suffix target; a Belt takes no lord omen (`lord-omen-classes`). | `last-desecration/belt-2p1s.txt` |
| `body-armour-2p1s.txt` | Rare Chieftain Cuirass (str), item level 82: Life T5, increased Armour T2; Fire Resistance T3. An armour class takes no lord omen. | `slots/body-armour-prefix-no-pool.txt` |
| `whittle-start.txt` | The whittle's start ring, item level 76: Mana T5; Strength T5, neither a target. | `gear/rings-resist-whittle/01-start.txt` |
| `whittle-fire.txt` | The same ring after its chaos: Mana T5; Fire Resistance T3, a target. | `gear/rings-resist-whittle/02-chaos-fire.txt` |
| `bow-magic.txt` | Magic Adherent Bow, item level 82: increased Physical Damage T2. `weapons-physical-generic`'s start; its preview ends with the desecration for Ulaman's lightning penetration prefix. | `guides/weapons-physical-generic/magic.txt` (a Two Hand Mace there) |
