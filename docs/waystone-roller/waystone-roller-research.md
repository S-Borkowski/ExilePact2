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
  practice; this project measured it in Live 1 (§ "Search syntax").
- **Live 1**, this project's own measurement in the game, 2026-10-28.
- **Live 2**, the built app in the game, 2026-10-28 (§ "The built app in
  the game (Live 2)").
- **(O)** the owner's knowledge of the game.
- poe2db.tw's pages, read 2026-10-28.

## Search syntax

The search box of the stash and of a vendor highlights the items whose text
matches. Every row was a check of Live procedure 1 (the written live
session of the workorder that built the app). **Live 1**, 2026-10-28:
eight waystones in one stash tab, Normal, Magic, Rare and corrupted Rare,
each at tier 2 or 1 and at tier 15 (no non-waystone item: the owner ruled
that not matching one is not a requirement); the owner pasted each probe
and said which lit. "M" below means measured in that session.

| Fact | Source | Generator uses it |
|---|---|---|
| Limit 250 characters: a 250-character string is accepted whole; a 251-character one is refused whole (the box stays empty), not truncated | M; O, P | yes |
| Case-insensitive (`WAYSTONE` lit all eight) | M; P | yes |
| Space separates terms; every term must match (AND) | M; P | yes |
| `"..."` makes one term of text with spaces | M; P | yes |
| `\|` inside a term is OR | M; P | yes |
| `"!a\|b"` hides items matching a or b | M; P | yes |
| A term is a regular expression: `[..]` classes, `\)` escapes | M (`"r 2\)"`, `"y: [nr]"`); P | yes |
| The box searches the advanced item text, roll ranges included: `"15\)"` lit a tier-2 waystone through `+14(11-15)%` | M | yes: every tier alternative carries `r ` |
| Rarity line reads `Rarity: Magic` (terms `"y: m"`, `"y: [nr]"`) | M; P | yes |
| Corrupted items carry `Corrupted` (terms `corr`, `!corr`) | M; P | yes |
| The name line carries `(Tier N)` (`"r 2\)"` lit exactly the tier-2 waystones) | M; poedb base type `Waystone (Tier 15)`; P | yes |
| Summary properties (item rarity, waystone drop chance, pack size, monster rarity, monster effectiveness, revives) show as lines of the item text | M (item texts); not probed as search terms | no (out of scope) |
| Per-mod reward lines ("15% more Waystones found in Area") were not observed on the measured waystones: no copied advanced text of the magic and rare ones had one, and `"waystones found"` lit none | M (these waystones only) | no: tokens avoid them anyway |
| Lookahead `(?=`, `(?!` | `"way(?=stone)"` lit all eight: accepted or ignored, not established which; P never emits one | no |
| Counting mods | not established; no counting syntax in P | no |

The item texts were copied in the advanced form (affix headers and roll
ranges), which the owner says `Ctrl+Alt+C` gives; which form plain `Ctrl+C`
gives was not separated in the session. Either way the box matched a roll
range, so it searches the advanced text.

`wr-regex`'s search model (`src/model.rs`) is this table written as code,
for the tests only: split on spaces outside double quotes, quotes removed, a
leading `!` negates, the rest a case-insensitive regular expression searched
anywhere in the item's text; an item is highlighted when every positive term
matches and no negated one does. A pattern with lookaround is refused,
because the game's handling of it is not established. The model is not a
measurement; the `_baseline` tests pin it, and one of them pins the roll
range Live 1 measured.

### The text a waystone carries besides its mods

A token must never match the text every waystone shows. Measured in Live 1
from the eight item texts (`wr-regex` `ITEM_FRAME_LINES`): `Item Class:
Waystones`, the rarity line, the name line (a rare's random name above
`Waystone (Tier #)`; a magic waystone's affix names around it, as in
`Waystone (Tier 2) of Sleet`), the summary lines `Revives Available`,
`Item Rarity`, `Pack Size`, `Monster Rarity`, `Monster Effectiveness` and
`Waystone Drop Chance` (each ending in `(augmented)` when a mod raised it),
`Item Level: #`, a header above each mod (`{ Prefix Modifier "<affix
name>" (Tier: #) }`, or `Suffix`), each number followed by its roll range
(`36(30-39)%`), a mod line that cannot roll ending in `— Unscalable Value`,
the Map Device description ("Can be used in a Map Device, allowing you to
enter a Map. Waystones can only be used once.") and, last, `Corrupted`.
`ITEM_FRAME_LINES` also keeps lines the session did not show (`Rarity:
Unique`, `Waystone Tier: #`, `Rare Monsters`, `Magic Monsters`,
`Unidentified`) as guards: a frame line only keeps a token away from more
text. The desecrated affix names are frame text too. Random rare names and
the affix names of regular mods are not known (poe2db lists none).

Adding `Monster Rarity` mattered: before it, the token of the fixture mod
"Players have #% less Whisper Resistance" was `r r`, which `Monster Rarity`
holds, so it would have lit every waystone showing that line; the fixture
tests caught it once the item fixtures took the measured layout.

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
  `r [a-b]\)` for single digits, `r 1d\)` or `r 1[a-b]\)` for 10 to 16,
  joined by `|`; 14 to 16 gives `"r 1[4-6]\)"` and 5 to 12 gives
  `"r [5-9]\)|r 1[0-2]\)"`. The `r ` of "Tier" is on every alternative
  because the box also searches roll ranges: the first design wrote `1d\)`
  for 10 to 16, and Live 1 refuted it (`"15\)"` lit a tier-2 waystone
  through `+14(11-15)%`). The `r ` also keeps `(Tier 1)` from matching a
  `1)` elsewhere; a mod header's `(Tier: 1)` has a colon, so it never
  matches.

## The built app in the game (Live 2)

**Live 2**, 2026-10-28: the release build of 0.1.0, Path of Exile 2 in
front, the same stash tab as Live 1. A watcher put a sentinel on the
clipboard before each press and read it after; the app's `diag.log` was
read beside it.

- **Each `Ctrl+Alt+W` press reached the app** with the game in front and
  replaced the sentinel with the search the `copied` diag line names: want
  any `"esh|d c"`, avoid `!col`, tier with rarity `"y: r" "r 2\)"`,
  corrupted `corr`. Pasted into the stash search box, each lit the
  waystones Live 1's item texts said it would.
- **The search box changed only when the owner pasted**: the app sent the
  game no input (the owner, asked directly: "no").
- **Over 250 characters copied nothing**: `copy refused: the search is 255
  characters, over 250` in `diag.log`, the sentinel still on the clipboard,
  and the window showing the refusal.
- **`MessageBeep(MB_ICONHAND)`, the first build's refusal sound, was not
  heard** on five refused presses. Cause not established: the build logged
  no result for the call, so "not run" and "not heard" could not be told
  apart.
- **The `Beep` tone was heard** in a follow-up check the same day: the
  rebuilt app plays an 880 Hz `Beep`, which reads no entry of the sound
  scheme, and logs `refusal beep: <sound> ok|failed (<error>)`. The owner,
  over-limit selection, game in front: "heard the tone". The System Sounds
  level, the output device and the diag line were not recorded.

## The auto-paste in the game (Live 3)

**Live 3**, 2026-10-28: the release build of 0.1.2 from this branch at
`d934ef52` (exe SHA-256 `ca6d2570…8c99`, built after the last commit under
`apps/waystone-roller/`), Path of Exile 2 in front with the owner at a
stash, a settings folder of its own through `WAYSTONE_ROLLER_DIR` whose
selection gives the search `corr`. The owner did every in-game action and
answered each question; the operator read `diag.log`, the clipboard and the
process list from outside.

- **The game's window is titled exactly `Path of Exile 2`**: read from
  outside with `Get-Process`, process name `PathOfExile`. That is the title
  the app compares before it pastes.
- **The shortcut's copy still works** (the control): with a sentinel on the
  clipboard, a press logged `copy from the shortcut` and `copied 4
  characters: corr`, and the clipboard then held `corr`.
- **The paste replaced the box's text** (pass): the owner clicked into the
  stash search box, typed `xyz` and pressed `Ctrl+Alt+W`; asked exactly
  what the box held, they answered `corr`. `diag.log` added `paste: sent
  Ctrl+A, Ctrl+V` within a second of the copy. So the game's search box
  took the injected `Ctrl+A` as select-all and the injected `Ctrl+V` as a
  paste over the selection.
- **A second press left the search once** (pass): with the box still
  focused, another press logged a second `paste: sent` line, and the owner
  confirmed the box held exactly `corr`, once.
- **With the checkbox off, the press only copied** (pass): the untick
  logged `auto-paste off` and saved `"auto_paste": false`; a press over
  newly typed `xyz` logged `paste skipped: auto-paste is off`, and the owner
  saw `xyz` unchanged.
- **Another window in front got nothing** (pass): ticked again (`auto-paste
  on`), Notepad in front with `keep-me` typed, a press logged `paste
  skipped: the window in front is not Path of Exile 2`, and the owner
  confirmed Notepad held only `keep-me`.
- **The app stopped cleanly** (pass): closing the window logged `stop`, and
  no `waystone-roller.exe` was left running.
- **With no text field focused, the keys reach the game's bindings** (the
  research check): with the stash closed, three presses each logged
  `paste: sent Ctrl+A, Ctrl+V`, and the owner reported that game key
  bindings fired ("buttons were pressed"). The owner also opened the chat
  and pressed the shortcut there: the search was pasted into the chat's
  input line. So the paste goes to whatever text field of the game has
  focus, chat included, and the app cannot tell which. Which binding fired,
  and whether it was answering `Ctrl+A`, `Ctrl+V` or `Ctrl+Alt+W` itself,
  was not separated.
- **Presses the procedure did not ask for**: between the toggle-off check
  and the Notepad check the log holds a second press while off (skipped),
  a toggle on, two presses that sent the paste and a toggle off; the owner's
  answers do not describe them. In one of those, the `paste: sent` line came
  3 seconds (by the log's whole-second stamps) after its copy, where the
  others came within a second; what was held or pressed meanwhile was not
  recorded.

## Negatives and choices

- **No lookahead.** `!` and `|` express want and avoid; a lookahead form
  would be a guess: in Live 1 `"way(?=stone)"` lit every waystone, which
  an accepted lookahead and an ignored one would both do.
- **Bare two-digit tier terms were refuted** in Live 1: `1d\)` matched a
  roll range on a waystone of another tier, so every tier alternative now
  starts with `r ` (§ "Token design").
- **Over 250 characters fails loudly**, rather than dropping selections
  (the owner's choice, 2026-10-28): the window shows each part's length.
- **No count of modifiers.** P has no counting syntax and none is
  established; 0.1.0 offers none (the owner's choice).
- **Desecrated mods** are offered in their own group (the owner's choice).
- **Silent on a successful copy, a beep on a refusal** (the owner's choice).
