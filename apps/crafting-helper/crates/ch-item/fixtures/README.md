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
| `gloves-rare-fractured-desecrated.txt` | Gloves, rare, quality (augmented), single-line requirements, one rune socket, an implicit by header, a rune line, a fractured prefix, a hybrid two-line prefix, a revealed desecrated suffix, an unrevealed desecrated suffix, family tags in headers |
| `gloves-rare-short-form.txt` | the same gloves in a short form: no headers, tiers or roll ranges (measured: plain `Ctrl+C` does **not** give this form, see below) |
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
| `crossbows-rare-hybrid-physical.txt` | Crossbows, rare, a hybrid prefix (`#% increased Physical Damage` with Accuracy, two lines under one header) beside the plain `#% increased Physical Damage` prefix and a flat physical one: two modifiers of different families with the same line, three prefixes in all. The values sit in the tier ranges of `data/classes/crossbows.json`; the plain value plus the hybrid's (120%) lands in another plain tier, and the hybrid's Accuracy alone in a tier of the plain Accuracy prefix |
| `talismans-rare.txt` | Talismans, rare, three explicit modifiers |
| `jewels-rare.txt` | Jewels, rare, no requirements, a description section |
| `belts-rare-unscalable.txt` | Belts, rare, an implicit ending ` — Unscalable Value` |
| `amulets-rare-enchant.txt` | Amulets, rare, an enchant by marker alone (no header) |
| `rings-magic-unidentified.txt` | Rings, unidentified magic: the name line is the base type |

## Shapes that are guesses

Measured from waystones (2026-10-28): `Item Class:`, `Rarity:`, the name
lines, `--------` separators, `Key: value` properties with ` (augmented)`,
`Item Level:`, explicit headers `{ Prefix Modifier "<name>" (Tier: N) }` and
`Suffix`, roll ranges written straight after the number (`12(10-14)%`),
` — Unscalable Value`, a line with no number, a description section and a
final `Corrupted`.

Measured from the owner's equipment pastes in `real/` (2026-10-28):

- the class strings `Gloves`, `Boots`, `Rings`, `Crossbows` and `Jewels`;
- `Requires: Level 80, 101 Int` on one line, and an entry the character
  does not meet written `108 (unmet) Dex`;
- `Sockets: S ` (with a trailing space), and a rune line ending `(rune)` in
  its own section;
- family tags after ` — ` inside a header, and a header with none;
- `{ Implicit Modifier }`, with or without tags, no name and no tier, in its
  own section; the line under it carries **no** `(implicit)` marker;
- the header word `Desecrated` before `Prefix` or `Suffix` for a revealed
  desecrated modifier; the line under it carries **no** `(desecrated)`
  marker;
- `{ Corruption Enhancement — tags }`, a header with no `Modifier` word, in
  its own section;
- a two-line modifier under one header, a two-value `Adds … to …` line, and
  a decimal roll range `3.33(3.11-3.8)`;
- a property value ending in a damage-type word, `Fire Damage: 59-104 (fire)`
  (kept in the value text);
- plain `Ctrl+C` gives the same advanced text as `Ctrl+Alt+C` (one item).

Corrected after the real pastes: every `{ Implicit Modifier }` line in the
advanced constructed files had an `(implicit)` marker, and the revealed
desecrated suffix in `gloves-rare-fractured-desecrated.txt` a
`(desecrated)` marker. The real texts show neither, so both were removed;
the kind and the flag now come from the header alone, as in the game.

Still guessed, because no real text has shown them yet:

- the class strings of the other classes (`Body Armours`, `Foci` and the
  rest);
- the `Requirements:` block form;
- the line markers `(implicit)`, `(enchant)`, `(fractured)`,
  `(desecrated)`, `(crafted)` anywhere (the short form uses them);
- the header kind `Enchant Modifier`, and the header words `Fractured`,
  `Crafted` and `Unrevealed`; whether a fractured or crafted modifier's line
  also carries a marker (kept here, though the measured `Implicit` and
  `Desecrated` headers carry none);
- how an unrevealed desecrated modifier is shown (here a header
  `{ Unrevealed Desecrated Suffix Modifier }` and the line
  `Unrevealed Desecrated Modifier`);
- a hybrid modifier beside a plain one that shares its line
  (`crossbows-rare-hybrid-physical.txt`): the advanced copy here keeps the
  two under their own headers, each line with its own value and range, and
  that is a guess; no real copy of such an item has been seen. What the
  plain tooltip shows (one summed `#% increased Physical Damage` line, as
  the `ch-watch` matcher test assumes) is not measured either;
- the flag lines `Mirrored` and `Unidentified`;
- negative roll ranges, and the order of an implicit and a rune section on
  one item;
- the short form as a whole: plain `Ctrl+C` does not give it, and which
  copy, if any, does is not established. `gloves-rare-short-form.txt` keeps
  the parser able to read one.
