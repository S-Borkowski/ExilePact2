# Slot cases and v2 walks

What `tests/slots.rs` (the parity test) and `tests/seeds_v2.rs` (the v2
blueprints' walks) read.

- `cases.json` is the contract between the slot model
  (`tools/crafting_helper/slot_model.py`) and the engine
  (docs/models/crafting-slots-spec.md). The model's side owns it; the
  engine's tests never edit it. A difference is settled by the spec.
- **Every item text here is constructed**, not copied from the game: it is
  written in the measured advanced layout (docs/RUNTIME_DATA_MODELS.md §
  "Item text"), with affix names, tiers and value ranges taken from
  `apps/crafting-helper/data/` so each modifier joins its data modifier.
  What a case calls a modifier's level is the data tier's item level; only
  their order matters (which modifier is the lowest).

## Case items

One per case in `cases.json`, named by the case id. `slots.rs` keeps the
real target ids and floors behind each case's aliases.

| File | Item | Targets (floor) |
|---|---|---|
| `rare-2p1s-prefix-missing.txt` | Rare Amethyst Ring, item level 76: Mana T5, Life T4; Fire Resistance T5. | Mana T5, Life T4, Accuracy Rating T3 (missing), Fire Resistance T5. |
| `magic-two-good.txt` | Magic Jade Amulet, item level 82: Mana T2 (a top-two tier, so `weak-magic-start` leaves its routes alone); Cold Resistance T5. | Mana T6, Life T8 (Lesser Essence of the Body), the desecrated Global Defences prefix, Cold T5, Fire T3, Lightning T3. |
| `rare-fractured.txt` | Rare ring: a fractured Mana T4, Accuracy Rating T3, Cold Damage T3; Fire Resistance T6, the lowest level. | Mana T4, Life T4 (missing), Fire T6. |
| `rare-one-target.txt` | Issue #28's ring: Mana T5, Accuracy Rating T4; Cold Resistance T5, Dexterity T7, the lowest level. | Mana T5, Life T4, Evasion Rating T3, Chaos Resistance T3 (the hardest, spawn weight 250). |
| `rare-no-target.txt` | Issue #28's belt, Heavy Belt, item level 82: Charm Effect Duration T3, Flask Mana Recovery T5, the lowest level; Stun Threshold T5, Life Regeneration T5. | Life T1 (the hardest), Mana T2, Cold T3, Fire T3. |
| `magic-one-good.txt` | Magic amulet: Mana T2 (a top-two tier), no suffix. | Mana T6, Life T8 (the essence's), Fire T5. |
| `magic-good-and-bad.txt` | Magic amulet: Mana T2 (a top-two tier); Intelligence T5. | As `magic-one-good`. |
| `rare-whittle-certain.txt`, `rare-whittle-blocked.txt` | Rare ring: Mana T5, Rarity of Items T3, the one lowest level, Accuracy Rating T3; Fire Resistance T5. The same item twice; the second case blocks Omen of Whittling. | Mana T5, Life T4 (missing), Fire T5. |
| `rare-2p1s-necromancy-blocked.txt` | `rare-2p1s-prefix-missing`'s ring again; the case blocks Omen of Sinistral Necromancy. | As `rare-2p1s-prefix-missing`. |
| `body-armour-prefix-no-pool.txt` | Rare Chieftain Cuirass (str), item level 82, no implicit: Life T5, increased Armour T2; Fire Resistance T3. The class has desecrated suffixes only. | Life T5, increased Armour T2, Spirit T3 (missing), Fire T3. |
| `rare-desecrated-target.txt` | The 2P/1S ring with a revealed desecrated Attack Damage while on Low Life prefix (its one tier, level 65) in Mana's place, then Life T4; Fire Resistance T5. | The desecrated prefix T1, Life T4, Accuracy Rating T3 (missing), Fire T5. |
| `rare-wrong-reveal.txt` | The 2P/1S ring plus a revealed desecrated Remnant effect prefix (level 65), not a target: a reveal that offered none. | As `rare-2p1s-prefix-missing`. |
| `rare-wrong-reveal-same-side-exalt.txt` | `rare-wrong-reveal`'s ring without its Life prefix: Mana T5 and the desecrated Remnant effect prefix; Fire Resistance T5. One prefix slot free. | Mana T5, Life T4 (missing, the hardest), Fire T5. |
| `rare-certain-removal-first.txt` | `rare-fractured`'s ring with Fire Resistance T5, the lowest level, and an unrevealed desecrated suffix. | Mana T4, Life T4 (missing), Fire T5, the desecrated Skill Speed suffix T1 (missing, the hardest). |
| `rare-uncertain-removal-keeps-order.txt` | The same ring with its Mana not fractured. | As `rare-certain-removal-first`. |
| `rare-prefixes-full.txt`, `rare-prefixes-full-dextral-blocked.txt` | Rare Amethyst Ring, item level 76: Mana T4, Life T1, Accuracy Rating T4, the lowest level; Fire Resistance T4. The prefixes are full. The same item twice; the second case blocks Omen of Dextral Exaltation. | Mana T4, Life T4, Accuracy Rating T4, Fire T4, Cold Resistance T2 (missing, the hardest), Lightning Resistance T3 (missing). |
| `rare-suffixes-full.txt` | Rare ring: Mana T4; Fire Resistance T4, Cold Resistance T5, the lowest level, and a revealed desecrated Skill Speed suffix (`of Ulaman`). The suffixes are full. | Mana T4, Life T4 (missing, the hardest), Fire T4, Cold T5, the desecrated Skill Speed suffix T1. |

## Walk states (`walks/`)

The states after each step of `seeds_v2.rs`'s walks, starting from the
case items above.

| File | What it is |
|---|---|
| `close-closed.txt` | The 2P/1S ring with Cold and Lightning Resistance T5: the suffix side closed. |
| `close-complete.txt` | The closed ring plus Accuracy Rating T2: done. |
| `close-slam-miss.txt` | The closed ring plus Evasion Rating T3: the slam on the open side missed. |
| `clean-cleaned.txt` | Issue #28's ring with Accuracy Rating annulled. |
| `clean-two-prefixes.txt` | The cleaned ring plus Life T1 and Evasion Rating T2: a Greater slam's two prefixes. |
| `clean-complete.txt` | That ring plus Chaos Resistance T2: done. |
| `clean-target-annulled.txt` | Issue #28's ring with the Mana target annulled instead. |
| `essence-rare.txt` | `magic-one-good` after the Lesser Essence of the Body: Rare, Mana T2 and Life T8. |
| `essence-desecrated.txt` | The same with an unrevealed desecrated suffix. |
| `essence-revealed-fire.txt` | The reveal as Fire Resistance T3 (desecrated): done. |
| `essence-revealed-wrong.txt` | The reveal as Cold and Chaos Resistances: a miss. |
| `essence-wrong-kept-fire.txt` | That miss with Fire Resistance T2 from the suffix exalt, the desecrated Cold and Chaos Resistances kept (Omen of Light waits for the same-side exalt): done. |
| `augment-fire.txt` | `magic-one-good` with Fire Resistance T2 from a Perfect augment. |
| `augment-miss.txt` | `magic-one-good` with Strength T1 instead. |
| `augment-essence.txt` | `augment-fire` after the essence: Rare, Mana, Life T8, Fire T2: done. |
