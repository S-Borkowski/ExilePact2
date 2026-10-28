# ch-item

The crafting helper's item text parser. A standalone, std-only Rust library
(`ch_item`) that turns the text Path of Exile 2 puts on the clipboard for an
item into a typed item model. It reads text it is given and nothing else: no
clipboard, hotkey, Windows API or game input (the app shell, M3, grabs the
text; the blueprint engine, M4, consumes the model).

## The two copy forms

- **The advanced form** is the primary input. Above each modifier it puts a
  header, `{ Prefix Modifier "Hale" (Tier: 4) — Life }`, and after each
  number the roll range, `+62(60-69)`. The parser takes the kind, affix
  name, tier, family tags and roll ranges from it. **Both `Ctrl+Alt+C` and
  plain `Ctrl+C` give it**: the owner copied one rare gloves both ways on
  2026-10-28 and the two texts were identical
  (`fixtures/real/gloves-rare-rune-desecrated-*.txt`).
- **The short form** shows the modifier lines alone. No copy has been seen
  to give it; the parser reads it anyway, in case one does. There is no
  prefix/suffix, affix name, tier or range in it, each line becomes its own
  modifier (grouping a hybrid modifier's lines needs the mod data), and a
  magic item's base type stays unknown (its affix names cannot be told from
  its base without the base list).

`Item::form` says which one a text was: `Advanced` (a header or a roll range
seen), `Short` (modifier lines, neither of those) or `Undetermined` (no
modifier line at all, such as a plain normal item).

## The model in brief

`ch_item::parse(text)` returns an `Item`, or `ParseError::NotItemText` when
the first non-blank line is not `Item Class: …`. The item holds:

- the item class (an enum of the 22 equipment classes in scope, or
  `Other(text)`), rarity, the rare's name, the base type and the raw name
  lines;
- item level, quality (with its augmented flag), requirements (level and
  each attribute, which of them the text marks `(unmet)`, anything else kept
  as text), sockets (text and count),
  properties in order (`name`, value text, augmented) and description lines;
- every modifier in text order: kind (`Prefix`, `Suffix`, `Implicit`, `Rune`,
  `Enchant`, `Corruption` for a `{ Corruption Enhancement }` header,
  `Unknown`), affix name, tier, family tags, the flags
  fractured, desecrated, unrevealed and crafted, and its lines. Each line
  keeps its text, a template with every value replaced by `#` (markers and
  ` — Unscalable Value` removed, a leading `+` kept), its values with their
  roll ranges, the unscalable flag and its markers (`(implicit)`, `(rune)`,
  `(enchant)`, `(fractured)`, `(desecrated)`, `(crafted)`);
- the item flags corrupted, mirrored and unidentified, and queries for M4:
  `has_fractured`, `has_desecrated`, `has_unrevealed`, `prefix_count`,
  `suffix_count`.

## Nothing is dropped: unparsed lines

Every non-blank, non-separator input line is either consumed into the model
or listed in `item.report.unparsed` with its line index, section index, text
and a short reason, exactly once. A line consumed only in part (an unknown
header word, an unknown trailing `(word)`, a `(` after a number that is not a
roll range, a header and line marker that disagree) is listed in
`item.report.partial` with the part not understood. Every consumed element
keeps its input line index (`ch_item::split_lines` numbers the lines, the
same for CRLF and LF), and `Item::consumed_lines` collects them, which is how
`tests/accounting.rs` checks the rule over every fixture.

## Fixtures and which shapes are still guesses

- `fixtures/constructed/` holds item texts written by hand, one or more per
  item class, in the layout measured from real copied waystones plus guessed
  equipment shapes. They prove the parser is consistent, not that the
  shapes are right. `fixtures/README.md` lists each file and the guesses.
- `fixtures/real/` holds only texts the owner copied in the game, byte for
  byte, each recorded in `fixtures/real/README.md`. Every one must parse
  with no unparsed line.

Measured from copied waystones: the class/rarity/name block, `--------`
sections, `Key: value` properties with ` (augmented)`, `Item Level:`, the
`Prefix`/`Suffix` header with affix name and tier, roll ranges,
` — Unscalable Value`, the description and `Corrupted`.

Measured from the owner's six equipment pastes in `fixtures/real/`
(2026-10-28): the class strings `Gloves`, `Boots`, `Rings`, `Crossbows` and
`Jewels`; `Requires: Level 80, 101 Int` and its `108 (unmet) Dex` entry;
`Sockets: S `; a `(rune)` line in its own section; family tags after ` — `;
`{ Implicit Modifier }` (with or without tags) and the header word
`Desecrated`, neither with a marker on the line under it;
`{ Corruption Enhancement — tags }`; a hybrid two-line modifier, a two-value
line and a decimal range; and that plain `Ctrl+C` gives the advanced form.
Every one of the six parses with no unparsed and no partly understood line.

Still guesses: the other class strings, the `Requirements:` block, the line
markers other than `(rune)`, the `Enchant` header kind, the header words
`Fractured`, `Crafted` and `Unrevealed`, how an unrevealed modifier is
shown, `Mirrored` and `Unidentified`, negative ranges, and the short form
as a whole. All of these live in one table, `src/vocab.rs`, so a correction
from a real paste changes one place; `fixtures/README.md` lists them.

## Commands

```
cargo test --manifest-path apps/crafting-helper/crates/ch-item/Cargo.toml
cargo clippy --manifest-path apps/crafting-helper/crates/ch-item/Cargo.toml --all-targets -- -D warnings
cargo run --quiet --manifest-path apps/crafting-helper/crates/ch-item/Cargo.toml --example parse -- apps/crafting-helper/crates/ch-item/fixtures/constructed/gloves-rare-fractured-desecrated.txt
```

The `parse` example prints the model, then every unparsed or partly
understood line, and last `unparsed lines: <n>`; it exits non-zero only when
the file cannot be read or is not item text.

The crate declares no `[workspace]` and no dependency, so it builds on its
own and, unchanged, as a member of the crafting helper's Cargo workspace.
