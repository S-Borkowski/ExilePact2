# Crafting Helper: research note

What the crafting helper's modifier data rests on: which poe2db.tw pages it
reads and what they carry, the developer tool that turns them into the
shipped data, the shape of those files, and what is not established. The
crafting mechanics themselves (currencies, omens, essences, desecration and
the rest) are described in [`crafting-mechanics.md`](crafting-mechanics.md).

The data is poe2db's reference data about item classes, not a structure of
the game client, so nothing here goes into `docs/RUNTIME_DATA_MODELS.md`. No
decompiler was used. The sources are poe2db.tw's item-class pages, fetched
2026-10-28 by the tool below (52 pages and robots.txt, one request at a time,
3 s apart). poe2wiki and craftofexile are not read.

## poe2db ModsView pages

Read from poe2db.tw on 2026-10-28. Its robots.txt disallows only
`/*/marked` and `/*/markdown_history`.

- **Which pages.** The class pages (`/us/Gloves`, `/us/Boots`, `/us/Helmets`,
  `/us/Body_Armours`, `/us/Shields`, `/us/Jewels`) carry text only: no
  ModsView, no weights, no levels, and no link to the attribute pages. The
  data lives on one page per attribute variant for armour
  (`/us/Gloves_str` … `/us/Gloves_dex_int`; Body Armours add
  `Body_Armours_str_dex_int`; Shields have `str`, `str_dex`, `str_int`), one
  page per class for weapons, offhands and jewellery (`/us/Quivers`,
  `/us/Rings`, …), and one page per jewel base (`/us/Ruby`,
  `/us/Time-Lost_Diamond`, …). The tool's class table names all 52; every one
  answered 200 and carried a ModsView on 2026-10-28, including the four
  Time-Lost jewels the planning run had not opened.
- **The ModsView object.** Each page calls `new ModsView({...})` exactly once;
  the argument is a JSON object, read with `json.JSONDecoder().raw_decode`
  from its opening brace. Top-level keys seen across the 52 pages: `opt`,
  `baseitem`, `config`, `gen`, `ids` (jewel pages: a list of integers, not a
  row group) and 42 row groups. `opt.ItemClassesCode` is the class's code
  (`Gloves`, `Helmet`, `Body Armour`, `Warstaff` for Quarterstaves, `Jewel`,
  …); `baseitem.opts` lists the attribute variants on armour pages and is
  absent or empty elsewhere; on a jewel page `baseitem.Code` is the jewel's
  own base name.
- **Row groups kept.** Totals over the 52 pages: `normal` 7355 rows,
  `essence` 1749, `desecrated` 827, `perfect_essence` 438, `liquid` 96
  (jewel pages only; empty on every equipment page).
- **Row groups counted and dropped.** Non-empty on some page, with their
  totals: `socketable` 4384, `bonded` 3445, `corruption_upgrade` 790,
  `corrupted` 514, `berserking` 180, `marksman` 168, `soul` 147, `decay` 138,
  `chronomancy` 102, `destruction` 90, `breach_caster` 89, `breach_minion`
  74, `breach_otherworldly` 36. Empty on every page: `bestiary`, `crusader`,
  `delve`, `eater`, `elder`, `enchant`, `graft_corrupted`, `haunted`,
  `hunter`, `incursion`, `infamous`, `master`, `redeemer`, `rotmother`,
  `scourgedown`, `scourgeup`, `searing`, `sentinel`, `shaper`, `synthesis`,
  `synthesis_corrupted`, `veiled`, `warbands`, `warlord`.
- **Modifier rows** (`normal`, `desecrated`): `Name` (affix name, plain
  text), `Level` (the modifier's item level, as a string),
  `ModGenerationTypeID` (`"1"` prefix, `"2"` suffix), `ModFamilyList` (list
  of family names), `DropChance` (string or integer) and `str` (the displayed
  text as HTML). `DropChance` is absent on 48 desecrated rows, all on jewel
  pages.
- **Essence rows** (`essence`, `perfect_essence`) add `Code` (the modifier's
  id), `IsPerfect`, `IsAlloy`, `Removes`, and a `Name` that is a link to the
  essence's page (slug from the link, name from its text). `perfect_essence`
  holds the perfect essences, the corrupted ones (Hysteria, Horror, the
  Abyss) and the alloys. One essence can appear as several rows with
  different `Code` but the same display.
- **Liquid rows** (`liquid`) have the essence row's shape without
  `IsPerfect`/`IsAlloy`/`Removes`, and a `Name` linking to a liquid emotion
  (`Diluted_Liquid_Ire` and the like).
- **The text.** Values are `span.mod-value`; a range is two numbers joined by
  an em dash (U+2014) in a `span.ndash`, possibly negative or decimal; lone
  numbers occur too. Keyword links keep their text. `<br>` separates lines:
  637 kept rows have more than one. 16 kept rows carry a `span.secondary`
  (a hidden internal stat, not item text); the tool drops it, as the
  waystone tool does.
- **Weights.** On equipment pages the `normal` rows carry real spawn weights
  (1000, 800, 500, 250, 125 …). Every `desecrated` row has 1 or nothing, and
  every jewel page's `normal` rows are all 1: the site's placeholder, not a
  weight. One equipment page, `Body_Armours_str_dex_int`, also has every
  `normal` row at 1. On pages that do carry weights, 26 normal tiers still
  show 1 (energy shield recharge on int body armours, the Surpassing extra
  arrow on quivers, physical leech on bows and crossbows, cast speed on
  amulets and rings).
- **Base types.** Armour variant pages hold them in a tab `#<X>BaseItem`
  whose header reads like "Gloves BaseItem /35", single-class pages in
  `#<Class>Item` ("Rings Item /29"). Each base is a card whose name link has
  class `whiteitem` (the image link has it too), with an optional
  "Requires: Level …" block and implicit lines. The header's `/N` equalled
  the card count on every page. No drop level is on these pages.
- **Families with several templates.** One family can hold modifiers with
  different text in one class (the socketed-skill levels for melee, spell,
  minion and projectile skills share one family, as do the weapon damage
  type prefixes on wands, staves and foci). Shipped counts of families with
  more than one template: 149 on jewels, 0 to 6 per other class.

Counts from this run, after grouping (`extract`'s summary): 1570 base
entries (one per base and variant page), 900 normal and 380 desecrated
modifiers holding 8180 tiers, 790 essence entries and 75 liquid-emotion
entries. Sceptres have no desecrated rows; jewels have no essences.

## Data extractor

`tools/crafting_helper/poe2db.py`, standard library only, developer-only.
Three commands:

```
py -3 -m tools.crafting_helper.poe2db fetch [--cache DIR] [--delay S] [--max-age DAYS]
py -3 -m tools.crafting_helper.poe2db extract [--cache DIR] [--out DIR]
py -3 -m tools.crafting_helper.poe2db check [--data DIR]
```

- **`fetch`** reads robots.txt, then every page of the class table not fresh
  in the cache, one request at a time, `--delay` seconds apart (default 3,
  minimum 2, raised to robots.txt's `Crawl-delay` when that is longer), with
  a User-Agent naming the tool and this repository, and only to poe2db.tw.
  A page fetched within `--max-age` days (default 7) is not asked again; when
  every page is fresh nothing is asked, robots.txt included. The cache
  defaults to `%LOCALAPPDATA%\ExilePact2\crafting-helper\poe2db-cache\` and
  holds the raw pages plus `index.json` (URL, fetch date, HTTP status, file
  per page). It refuses: a `--delay` under the minimum (exit 2), a cache
  folder inside a git checkout (raw pages are never tracked), a run with a
  page robots.txt disallows (nothing is fetched), and any answer other than HTTP 200 (the run stops
  and that page's cache entry is left as it was). The real run took 52
  requests plus robots.txt, about three minutes.
- **`extract`** works offline from the cache and writes `index.json` and
  `classes/<class>.json` under `--out` (default `apps/crafting-helper/data`).
  It builds every class, checks each with `check_data`, and writes nothing
  unless all pass. It refuses, naming the page, when: a page of the table is
  not in the cache; a page has no `new ModsView(` call, more than one, or an
  argument that is not JSON; `opt.ItemClassesCode` is not the table's code;
  `baseitem.opts` is present and differs from the table's variants;
  `normal` is missing or empty; a kept row lacks a required key
  (`DropChance` is optional); a kept row's generation type is not 1 or 2; a
  value does not parse; the base tab's card count differs from its header's
  `/N`; or a non-empty row group is neither kept nor on the known-ignored
  list. The 2026-10-28 pages extracted without a refusal and without a
  change to the tool.
- **`check`** loads `index.json` and every class file it names, prints each
  problem and exits 1 on any, 0 when the files keep the schema below and are
  byte for byte what `extract` would write. `tests/test_crafting_helper_data.py`
  holds the shipped files to the same rules plus the facts pinned below.

The tool's fixture tests (`tests/test_crafting_helper_poe2db.py`) run on
synthetic pages in the site's shape with invented modifier lines, an
injected fetcher and clock, and no network.

## Data files

As shipped under `apps/crafting-helper/data/`. Every file is UTF-8 with LF
line endings, written by one fixed JSON dump (two-space indent, a trailing
newline), with keys in a fixed order and lists sorted by content rather than
page order, so a refresh diffs cleanly.

**`index.json`**: `schema` (1), `source_note` (names poe2db, its CC BY-NC-SA
3.0 licence and GGG's game text quoted under the owner's 2026-10-24
decision), and `classes`, one `{class, file, variants}` per item class in
the class table's order: Gloves, Boots, Helmets, Body Armours, Shields
(6, 6, 6, 7 and 3 variants), Quivers, Bucklers, Foci, Wands, One Hand Maces,
Sceptres, Spears, Bows, Staves, Two Hand Maces, Quarterstaves, Crossbows,
Talismans, Belts, Amulets, Rings (variant `all`), and Jewels (8 variants,
one per jewel base, the slug in kebab case: `ruby` … `time-lost-diamond`).
`class` is the game's "Item Class:" text, the key the item parser and the
blueprints join on.

**`classes/<class>.json`**: `schema`, `class`, `code` (poe2db's class code),
`source_note`, then:

- `pages`: one `{variant, url, fetched, weights, ignored}` per page.
  `weights` has `normal` and `desecrated`, each `spawn` (some row has an
  integer weight above 1), `not-established` (rows present, none above 1) or
  `none` (no rows). `ignored` counts the dropped row groups by name.
- `bases`: `{name, variant, req_level, implicits}`; `req_level` is an
  integer or null (no requirement shown), `implicits` a list of
  `{lines, values}`.
- `mods`: `{id, section, kind, family, lines, tiers}`. `section` is `normal`
  or `desecrated`, `kind` `prefix` or `suffix`, `family` the family list.
  `tiers` maps each variant the modifier appears on to its tiers,
  `{tier, name, ilvl, weight, values}`, numbered 1..n without gaps from the
  highest item level down (equal levels: higher values first). `weight` is
  the page's number, or null when the row has none.
- `essences`: `{name, slug, group, perfect, alloy, removes, kind, ilvl,
  family, lines, values, variants, matches}`; `group` is `essence` or
  `perfect_essence`; `variants` lists the pages it appeared on; `matches`
  maps each of those variants to `{mod, tier}` (the modifier tier it adds)
  or null.
- `liquid`: the essence shape without `group`, `perfect`, `alloy` and
  `removes`.

`lines` are templates: every number or range is `#`, so no digit, `<`, `>`
or `[` is left; `values` holds one `[min, max]` per `#` across the lines, in
order (a lone number is `[n, n]`). Ids are kebab case, unique per file and
derived from the content. A modifier is one group per (section, kind,
family list, lines); two templates in one family are two modifiers sharing
`family`, which M4 needs for the one-modifier-per-family rule. An essence or
liquid emotion is matched, per variant, to the modifier with the same kind
and family that has a tier of the same item level, the values deciding
between several candidates; it is null when none fits.

Facts the tests pin from this run: 22 classes with variant counts 3, 6, 6,
6, 7 and 8 for the multi-page ones; Gloves pages all `spawn` for normal and `not-established` for desecrated; the Gloves `+# to maximum Life` prefix has 9 tiers on the
int page, tier 1 at item level 60, and Lesser Essence of the Body matches its
tier 7; every jewel page is `not-established` for normal weights, jewels
have liquid emotions and no essences; Sceptres have no desecrated rows.

## Not established and negatives

Measured since (M4, the owner's five real item texts of 2026-10-28):

- **The derived tier number equals the game's `Tier:`.** The tier number
  the data files carry (derived by the extractor from poe2db's levels, tier
  1 the best on the variant), on the tier `ch-data` joins each modifier to
  by its affix name or its values, matched the `Tier:` in the header
  on all 18 explicit modifiers the five real items carry (2, 6, 6, 4 and 0
  per item), `ch-data`'s `real_fixture_tiers_equal_game_tiers`. Five items
  of five classes; a class none of them is has not been compared.
- **Revealed desecrated modifiers join to the normal section.** The
  crossbow's `Incinerating` and the gloves' `of Exile`, both marked
  `Desecrated` in their headers, are normal-pool modifiers at tiers 4 and 2
  (`desecrated_mods_join_to_normal_section`), so a reveal may offer
  normal-pool modifiers as well as the desecrated pool's; the engine counts
  both and gives a reveal unknown odds.

Not established:

- **Desecrated and jewel weights.** The pages show 1 or nothing; shipped as
  `not-established`, with a null `weight` where the row had none (48
  desecrated jewel tiers).
- **`Body_Armours_str_dex_int` normal weights.** Every row shows 1 on
  2026-10-28; the page is labelled `not-established` while the other six
  body-armour pages carry spawn weights. Whether the site lacks the numbers
  for that base set or the game rolls them evenly is not established.
- **Weight 1 on a page that has spawn weights** (26 tiers, listed above
  under "Weights"). Shipped as the number 1, which may be a real low weight
  or the same placeholder; M4 should treat those odds as unknown until a
  second source agrees.
- **Base drop level.** Not on these pages; it would take one request per
  base. Only the required level ships.
- **Unmatched essences.** Many essence entries match no modifier tier on
  some or every variant: the corrupted essences (Essence of the Abyss's
  "Mark of the Abyssal Lord", Essence of Horror), the alloys, and the
  Essence of the Infinite family, whose generic "Strength, Dexterity or
  Intelligence" line found no modifier of the same kind, family and item
  level. Each is
  `null` rather than a guess; 41 liquid-emotion matches on jewels are null
  likewise.

Negatives and rejected alternatives:

- **Scraping the class pages** (`/us/Gloves` and the like): they carry text
  only, no weights or levels, and no links to the variant pages, so the
  variants are fixed in the tool's class table and checked against each
  page's `baseitem.opts`.
- **One data file per page**: 52 files, while M4 looks modifiers up per
  class. Per-class files with per-variant tiers instead.
- **Refusing a page on `DropChance` 1**: it is the site's placeholder, so the
  page is labelled, not refused.
- **A warning, not a refusal, for an unknown row group**: rejected, because a
  new group may be a new crafting mechanic; the developer adds it to the kept
  or the ignored list.
- **Dropped groups.** Corruption outcomes (`corrupted`,
  `corruption_upgrade`) are outside the owner's scope; rune and soul-core
  augments (`socketable`, `bonded`) are socketing, not modifier crafting;
  the league pools (`breach_*`, `marksman`, `decay`, `soul`, `destruction`,
  `berserking`, `chronomancy`, `rotmother`) have no established crafting
  source. Their counts stay under each page's `ignored` so a refresh shows
  drift. Uniques are not read.
- **poe2wiki and craftofexile**: not fetched. poe2wiki sits behind an
  anti-bot wall and craftofexile serves no data in its HTML.

## Item tooltip on screen

M6's screen watch reads the hovered item's tooltip from the game's window.
What it looks like was measured on the owner's screenshots of 2026-10-29,
static, before any live session; the game facts are folded into
[`docs/RUNTIME_DATA_MODELS.md`](../RUNTIME_DATA_MODELS.md) § "Item tooltip
(on screen)".

**Method.** The owner took fourteen PNGs of the game's window, each with
the hovered item's `Ctrl+Alt+C` copy beside it, in a folder outside every
repository (never committed): a rare amulet plain and with Alt held, rare
gloves with a rune in the stash plain and with Alt, a magic ring, a normal
belt, a rare quarterstaff, a rare jewel, rare gloves with an unrevealed
desecrated modifier plain and with Alt, the stash and inventory with
nothing hovered, an Exalted Orb, and the amulet and the gloves again at
1920 x 1080. Shots 01-11 are 2560 x 1440 fullscreen, HUD scale middle, UI
text size medium; 12a and 12b are the game windowed at 1920 x 1080,
captured with the Windows title bar. They were decoded with `qt-capture`'s
decoder and read with Windows' OCR (en-US), the backend the app uses.
`crates/ch-watch/tests/shots.txt` lists each shot's kind, its client inside
the image and its hand-measured tooltip boxes; the tests in
`crates/ch-watch/tests/shots.rs` read the folder `CH_TOOLTIP_SHOTS` names
(`#[ignore]`d, so the default run needs no image):

```text
CH_TOOLTIP_SHOTS="$USERPROFILE/crafting-helper-shots" cargo test -p ch-watch --test shots -- --include-ignored
```

A box was measured against shot 10 (the same stash, nothing hovered) where
the scene matches: the body is where a pixel is about a fifth of the same
pixel without the tooltip. Elsewhere it was read off the pixel rows and
columns where the darkening stops. The header's top is its first frame row.

**Positive control first.** `whole_image_ocr_reads_the_copied_mod_lines`:
OCR of each whole equipment shot, no locator, against its copy's explicit
modifier lines: 4 of 4, 4 of 4, 3 of 3, 3 of 3, 2 of 2, 0 of 0 (the normal
belt), 6 of 6, 4 of 4, 4 of 5, 4 of 5 (the unrevealed modifier's line is
glyphs), 4 of 4 and 3 of 3. OCR reads this tooltip; the questions below
rest on it.

The questions:

1. **Where it draws.** Beside the hovered item, kept inside the window: in
   ten of the fourteen tooltips it lies flush against the client's left
   edge, in five within ten pixels of its top, whole each time. At
   2560 x 1440 the boxes were 663 to 877 pixels wide (0.26 to 0.34 of the
   client) and 194 to 631 high; at 1920 x 1080 the amulet's box is the same
   fraction of the client as at 1440 (0.108 to 0.336 of its height, both),
   so the tooltip scales with the client. Hovering the quarterstaff (07)
   drew the equipped weapon's tooltip beside it, higher and to the right,
   with its own name lines; no other shot shows one, and their key hints
   offer `Shift Compare`. When the game draws a comparison is not
   established. `shot_tooltip_boxes_match_the_measured_ones` holds the
   locator's boxes to the measured ones within a hundredth of the client.
2. **Pixel features.** The header is drawn opaque: a frame line along its
   top (a bright row, two darker ones, two more bright ones, each one
   colour along the line; gold for a rare, blue-grey for a magic item, grey
   for a normal one, pale brown for currency), a dark band with the name
   lines, and a second frame line along its bottom, about 42 pixels below
   the top for one name line and about 69 for two at 1440 (three quarters
   of that at 1080). Corner ornaments reach a quarter of that past the line.
   The body below darkens the scene behind it to about a fifth (median 0.19
   over 8,645 pixels of shot 01 against shot 10) and has no border, so over
   the stash's dark grid its bottom shows only faintly: the locator's box
   may run on below the tooltip (01: to row 838 against a measured 484), and
   the reader keeps only the lines that cross the box's middle. The
   no-tooltip shot finds none and the currency shot reads as another item
   (`shots_without_a_tooltip_find_none`); the stash's title plate, the same
   gold, is too short and its band too thin to pass.
3. **What OCR reads of a crop.** The plain form: the name lines, the item
   class in the singular (`Amulet`, `Gloves`, `Quarterstaff`, the equipped
   weapon's `Two Hand Mace`), the properties, the requirements (without
   `(unmet)`; OCR splits the line where its colour changes, `Requires: Level
   54,` then `225` then `Dex, 90 Int`, which the reader joins), the rune's
   line, the implicit, and each explicit modifier's line with its value and
   without a range; no item level and no `Item Class:` or `Rarity:` line.
   Confusions seen: `UARTERSTAFF`, `PHYSIcAL`, `OSMAXIMUM` for `of
   maximum`, `PUCE` for `Place`, `CLICKib`, `REQUIRE*`, and at 1080 `P R t C
   E CHEC K`; small capitals come back upper case. The fixed interface
   strings are the key hints the game draws in a small panel near the item,
   over the tooltip's box or below it: `Alt Inspect`, `Shift Compare`, `Ctrl
   Quick Move`, `Shift Alt Price Check`, and `Use` on currency. The matcher
   counts them (`INTERFACE_STRINGS` after `KEY_CAPS`, spaces ignored) and
   the class line. Whether the plain tooltip sums two modifiers' identical
   stat lines into one is not established: no shot's item has two.
4. **Name colours** (the most common bright pixel of the name lines, RGB):
   rare 254, 224, 85; magic 122, 151, 254; normal 200, 200, 200; currency
   170, 158, 130. Unique: not in the shots, so not established.
5. **Which form.** Plain. Held Alt adds the ranges, writes `Amulet: Item
   Level 12` for the class line and puts `Implicit`, `Prefix` and `Suffix`
   at the box's left edge and a tier (`T12`) at its right, but OCR lost the
   value of 2 of its 11 revealed modifier lines (`TO LIGHTNING RESISTANCE`)
   and of the amulet's implicit, against none in the plain shots; the reader
   drops the edge labels. The guide does not ask for Alt.
6. **Times** (release build, this machine, `examples/shot_timings.rs`):
   locating on a whole 2560 x 1440 client takes 12 to 36 ms (median of 5),
   on 1920 x 1080 14 to 16 ms; OCR of one crop 34 to 215 ms, the most for
   the jewel's box, which runs on 1009 rows over the dark stash. OCR of a
   whole 2560 x 1440 shot took 694 to 847 ms. The copy of the window is not
   part of these times: Live 1 measures it.
7. **Unrevealed and fractured modifiers.** An unrevealed desecrated
   modifier shows, plain and with Alt, as a row of green glyphs that OCR
   reads nothing of, then the line `Take this item to the Well of Souls to
   reveal the Desecrated Modifier`; the copy writes `{ Prefix Modifier
   "Veiled" }` then `Desecrated Prefix`. The matcher calls an item with one,
   or a tooltip with that line, unreadable. A fractured modifier is in no
   shot: not established, and unreadable likewise.

**The reads** (`shots_read_the_copied_mods_or_say_unreadable`): 7 of the 9
plain-form equipment shots read as their copy (both amulets, both rune
gloves, the ring, the belt, the jewel), and the rest are unreadable, never a
different item: the quarterstaff's `of the Parasite` suffix (a suffix the
copy gives no tier) has no modifier in the quarterstaff data, so its line
is not accounted for; and the gloves with the unrevealed modifier. Of the
Alt shots, the amulet reads as its copy and the rune gloves are unreadable
(a lost value).

Two findings changed the matcher:

- **An item can carry a tier its item level does not allow in the data.**
  The rune gloves (item level 43) have `Gleaming`, tier 5 of `Adds # to #
  Physical Damage to Attacks`, whose data item level is 46; reading tiers
  from item-level-filtered data, the plain line `Adds 5 to 9` fit tier 6
  alone and the read came back as a different item. The matcher now counts
  every tier of the variant, and a modifier whose id and values are the
  item's keeps the item's tier (the modifier is unchanged); new values that
  fit two tiers stay unreadable.
- **A reducing modifier's range runs high to low** in the copy: the belt's
  `12(15-10)% reduced Charm Charges used`, best to worst (the owner,
  2026-10-29). `ch-item` reads it (the belt's copy parses and joins), the
  matcher drops it like any range, and a tier holds a value between its
  bounds in either order.

Not observed in the shots: a black copy, a tooltip cut by the window's
edge, a tooltip over a bright scene (every body ended over the dark stash
or a dark panel), and a unique item. The locator and the reader are also
tested on synthetic tooltips drawn the same way
(`locator_finds_a_synthetic_tooltip`).
