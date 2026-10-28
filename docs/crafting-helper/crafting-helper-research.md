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
- **Whether the derived tier number equals the game's `Tier:`** in the
  Ctrl+Alt+C item text. M2 and M4 compare against a captured item fixture.
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
