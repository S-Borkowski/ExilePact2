# Waystone Roller: research note

What the waystone roller relies on about the game's item search and about
poe2db's Waystones page, where each fact comes from, and why the search
string is built the way it is. Facts about the game that every mod can use
are copied to [`docs/RUNTIME_DATA_MODELS.md`](../RUNTIME_DATA_MODELS.md)
§ "Item search box (stash and vendor)". The guide is
[`README.md`](README.md).

The app reads no game memory and hooks nothing; no decompiler was used. The
sources are:

- **(P)** the public community tool poe2.re, whose waystone page builds such
  search strings: its shipped JavaScript, read 2026-10-28. A public tool's
  practice, **not measured** by this project.
- **(O)** the owner's knowledge of the game.
- poe2db.tw's pages, read 2026-10-28.

## Search syntax

The search box of the stash and of a vendor highlights the items whose text
matches. Every row is a check of Live procedure 1 (the written live session
of the workorder that built the app); until it is measured, each keeps the
label below.

| Fact | Source | Generator uses it |
|---|---|---|
| Limit 250 characters | O, P | yes |
| Case-insensitive | P (lower-case tokens) | yes |
| Space separates terms; every term must match (AND) | P | yes |
| `"..."` makes one term of text with spaces | P | yes |
| `\|` inside a term is OR | P | yes |
| `"!a\|b"` hides items matching a or b | P | yes |
| A term is a regular expression: `[..]` classes, `\)` escapes | P (`er 1[4-6]\)` tier terms) | yes |
| Rarity line reads `Rarity: Magic` (term `"y: m"`) | P | yes |
| Corrupted items carry `Corrupted` (terms `corr`, `!corr`) | P | yes |
| The name line carries `(Tier N)` | poedb base type `Waystone (Tier 15)`; P | yes |
| Summary properties (item rarity, waystone drop chance, pack size, monster effectiveness, revives) are searchable lines | P | no (out of scope) |
| Whether each mod's reward lines ("15% more Waystones found in Area") show on the item | not established | no: tokens avoid them |
| Lookahead `(?=`, `(?!` | not established; P never emits one | no |
| Counting mods | not established; no counting syntax in P | no |

`wr-regex`'s search model (`src/model.rs`) is this table written as code,
for the tests only: split on spaces outside double quotes, quotes removed, a
leading `!` negates, the rest a case-insensitive regular expression searched
anywhere in the item's text; an item is highlighted when every positive term
matches and no negated one does. A pattern with lookaround is refused,
because the game's handling of it is not established. The model is not a
measurement; the `_baseline` tests pin it and Live procedure 1 checks it.

### The text a waystone carries besides its mods

A token must never match the text every waystone shows. Assumed from the
sources above until Live procedure 1's item texts replace it (`wr-regex`
`ITEM_FRAME_LINES`, labelled assumed): `Item Class: Waystones`, the rarity
line, `Waystone (Tier #)`, `Waystone Tier: #`, `Item Level: #`, the summary
properties (`Item Rarity`, `Waystone Drop Chance`, `Pack Size`, `Monster
Effectiveness`, `Rare Monsters`, `Magic Monsters`, `Revives Available`),
`Corrupted`, `Unidentified`, the Map Device description ("Can be used in a
Map Device, allowing you to enter a Map. Waystones can only be used
once.") and the desecrated affix names, which show in a magic waystone's
name. Random rare names and the magic affix names of regular mods are not
known (poe2db lists none).

## poe2db Waystones page

Read from <https://poe2db.tw/us/Waystones> on 2026-10-28. Its robots.txt
disallows only `/*/marked` and `/*/markdown_history`.

- The page's tabs are `div.tab-pane` elements with ids. Two are used:
  `WaystonesMods` (header "Waystones Mods /107"; columns Level | Pre/Suf |
  Description; Level was 1 in every row) and `DesecratedWaystoneMods`
  ("/19"; columns Name | Pre/Suf | Description, names such as `Abyssal` and
  `of Siphoning`). The header's `/N` equals the table's row count.
- A table's `tbody` id is a random string; tables are found by their tab's
  id and their header text.
- In a Description cell, stat lines are separated by `<br>`, and one stat
  whose text has two lines by `<br/>`; both start a new line, as the item
  shows them. Values are `span.mod-value`, a range is `(5–9)` with an en
  dash; keywords are links whose text is the displayed text. Hidden internal
  stats are `span.secondary` (for example "map ground effect radius [20]")
  and are not item text.
- Outliers in the real page: a regular row with an empty description; two
  desecrated `of Union` rows with only hidden stats; four rows of `Area has
  patches of Ignited Ground` identical once the hidden stats are gone;
  `Area contains # additional Abysses`, whose every line also appears in the
  `Area is overrun by the Abyssal` family; poedb's `Monster have` (sic).
- Grouping the rows by their displayed lines, numbers as `#`, gives 52
  families (35 regular, 17 desecrated) and 3 skipped rows.
- Each regular mod carries reward lines from a small set: `#% more Waystones
  found in Area`, `#% increased Waystones found in Area`, `Monsters have #%
  more Effectiveness`, `#% more Rarity of Items found in this Area`, `#% more
  Pack size`, `#% more Magic and Rare Monsters`, `Rare Monsters have #% more
  chance of Monster Modifiers`. The data file marks them (`bonus`).
- The tier pages (`Waystone_%28Tier_15%29`) give the base type `Waystone
  (Tier 15)` and the Map Device description above.

## Token design

- **Text is lower-cased.** A candidate token is a piece of a run of letters
  and spaces inside one line: never across a `#`, `%`, `+`, comma,
  apostrophe or a line end, at least 3 characters, no leading or trailing
  space.
- **A mod's own token** is the shortest candidate (then the alphabetically
  first) from its first non-reward line, else its later non-reward lines,
  that occurs in no line of any other mod, reward lines included, and in no
  frame line. Taking tokens only from non-reward lines, but checking them
  against every line, is right whether or not the reward lines show on the
  item.
- **The minimum of 3 characters**: a scratch prototype found many 2-letter
  tokens, which are likelier to hit a random rare name.
- **A cover for want any and avoid**: pieces of the chosen mods' text that
  no other mod holds, picked greedily by mods newly covered per character,
  used only when shorter than the chosen mods' own tokens joined, so never
  longer.
- On the 2026-10-28 data, 51 of the 52 families have a token; `Area
  contains # additional Abysses` has none, since every piece of its text is
  also part of `Area is overrun by the Abyssal`.
- **The tier term** matches the name line's `(Tier N)`: `r d\)` or
  `r [a-b]\)` for single digits (the `r ` keeps `(Tier 1)` from matching a
  `1)` elsewhere), `1d\)` or `1[a-b]\)` for 10 to 16, joined by `|`.

## Negatives and choices

- **No lookahead.** `!` and `|` express want and avoid; a lookahead form
  would be a guess until the box's handling of `(?=` is measured.
- **Over 250 characters fails loudly**, rather than dropping selections
  (the owner's choice, 2026-10-28): the window shows each part's length.
- **No count of modifiers.** P has no counting syntax and none is
  established; 0.1.0 offers none (the owner's choice).
- **Desecrated mods** are offered in their own group (the owner's choice).
- **Silent on a successful copy, a beep on a refusal** (the owner's choice).
