# Fracture and full-side desecration items

What `tests/fracture.rs` plans and guides over (issue #35). **Every item
text here is constructed**, not copied from the game, in the measured
advanced layout (docs/RUNTIME_DATA_MODELS.md § "Item text"), with affix
names, tiers and value ranges taken from `apps/crafting-helper/data/` so
each modifier joins its data modifier. How the game shows the Mark of the
Abyssal Lord (its header, and whether it is marked crafted) is not
measured; its line is the data's line for Essence of the Abyss.

The base is a Cavalry Boots (a Dexterity base) at item level 82. The
targets are Movement Speed T2 (prefix) and Fire Resistance T3 (suffix); the
desecration test adds Evasion Rating T3 as the missing prefix target.

| File | What it is |
|---|---|
| `boots-three.txt` | Movement Speed T2, Life T3; Fire Resistance T3. Three modifiers. |
| `boots-four.txt` | `boots-three` and a Dexterity T3 suffix: four modifiers, two of them targets. |
| `boots-four-mark.txt` | `boots-four` with the Mark of the Abyssal Lord in Life's place. |
| `boots-four-mark-desecrated.txt` | `boots-four-mark` with an unrevealed desecrated suffix in Dexterity's place: only the two targets are left to fracture. |
| `boots-fractured-target.txt` | `boots-four` after a Fracturing Orb fractured Movement Speed: the hit. |
| `boots-fractured-other.txt` | `boots-four` after a Fracturing Orb fractured Life: the miss. |
| `boots-five.txt` | `boots-four` and a Cold Resistance T3 suffix: one prefix slot free. |
| `boots-six.txt` | `boots-five` and a Mana T3 prefix: every slot full. |

The test blueprints beside them: `test-fracture.toml` (a rare with exactly
four modifiers, fractured with a Fracturing Orb) and
`test-full-desecration.toml` (a desecration on the last missing target's
side, with that side's Necromancy omen).
