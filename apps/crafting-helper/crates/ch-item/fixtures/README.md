# ch-item fixtures

Every file under `constructed/` is **constructed, not copied from the game**:
it was written by hand for the parser's tests, in the item-text layout measured
from real copied waystones (waystone roller Live 1, 2026-10-28;
`docs/RUNTIME_DATA_MODELS.md` § "Item search box (stash and vendor)") plus the
guessed shapes listed below. The item names, affix names, tiers and values are
invented. A constructed fixture's expectations come from the same hand as the
parser, so it proves the parser is consistent with these shapes, not that the
shapes are true. Texts the owner copies in the game go to `real/` instead
(`real/README.md`).

## Constructed files

| File | Covers |
|---|---|
| `gloves-rare-fractured-desecrated.txt` | Gloves, rare, quality (augmented), single-line requirements, one rune socket, an implicit with header and marker, a rune line, a fractured prefix, a hybrid two-line prefix, a revealed desecrated suffix, an unrevealed desecrated suffix, family tags in headers |
| `gloves-rare-short-form.txt` | the same gloves in the short (`Ctrl+C`) form: no headers, tiers or roll ranges |
| `boots-magic.txt` | Boots, magic, prefix and suffix affix names around the base type |
| `body-armours-rare-requirements-block.txt` | Body Armours, rare, the block form of requirements (`Requirements:` then `Level:` and `Str:`), two sockets, armour property |
| `helmets-normal.txt` | Helmets, normal, no modifier at all (form undetermined) |
| `quivers-magic-enchant.txt` | Quivers, magic with a suffix only, an enchant with header and marker, a two-value implicit |
| `shields-rare-corrupted.txt` | Shields, rare, `Corrupted` flag, an implicit with no number |
| `bucklers-magic.txt` | Bucklers, magic with a prefix only |
| `foci-rare-unidentified.txt` | Foci, unidentified rare: one name line, which is the base type |
| `wands-rare-crafted.txt` | Wands, rare, a crafted suffix by header and marker, a number inside an implicit |
| `one-hand-maces-rare-two-value.txt` | One Hand Maces, rare, weapon properties, a two-value line (`Adds 3(2-4) to 7(6-9)`) |
| `sceptres-rare.txt` | Sceptres, rare, three-entry requirements |
| `spears-magic.txt` | Spears, magic with a suffix only, weapon properties |
| `bows-rare-decimal.txt` | Bows, rare, decimal values and ranges |
| `staves-normal.txt` | Staves, normal with an implicit only |
| `two-hand-maces-rare-mirrored.txt` | Two Hand Maces, rare, `Mirrored` flag |
| `quarterstaves-rare.txt` | Quarterstaves, rare, a two-value elemental line |
| `crossbows-magic.txt` | Crossbows, magic, a decimal augmented property |
| `talismans-rare.txt` | Talismans, rare, three explicit modifiers |
| `jewels-rare.txt` | Jewels, rare, no requirements, a description section |
| `belts-rare-unscalable.txt` | Belts, rare, an implicit ending ` — Unscalable Value` |
| `amulets-rare-enchant.txt` | Amulets, rare, an enchant by marker alone (no header) |
| `rings-magic-unidentified.txt` | Rings, unidentified magic: the name line is the base type |

## Shapes that are guesses

Measured: `Item Class:`, `Rarity:`, the name lines, `--------` separators,
`Key: value` properties with ` (augmented)`, `Item Level:`, explicit headers
`{ Prefix Modifier "<name>" (Tier: N) }` and `Suffix`, roll ranges written
straight after the number (`12(10-14)%`), ` — Unscalable Value`, a line
with no number, a description section and a final `Corrupted`.

Guessed, to be confirmed against the owner's pastes in `real/`:

- the equipment class strings (`Gloves`, `Body Armours`, `Foci` and the rest);
- both requirement forms: `Requires: Level 45, 63 Str` and the
  `Requirements:` block;
- `Sockets: S S` and a rune line ending `(rune)` in its own section;
- the line markers `(implicit)`, `(rune)`, `(enchant)`, `(fractured)`,
  `(desecrated)`, `(crafted)`;
- the header kinds `Implicit Modifier` and `Enchant Modifier`, and the
  header words `Fractured`, `Desecrated`, `Crafted`, `Unrevealed` before
  `Prefix` or `Suffix`;
- family tags after ` — ` inside a header;
- how an unrevealed desecrated modifier is shown (here a header
  `{ Unrevealed Desecrated Suffix Modifier }` and the line
  `Unrevealed Desecrated Modifier`);
- the flag lines `Mirrored` and `Unidentified`;
- decimal roll ranges, the short form as a whole (which copy gives it is not
  established) and the order of the implicit, rune and explicit sections.
