# ch-data

The crafting helper's modifier data, as a Rust library (`ch_data`). It embeds
`apps/crafting-helper/data/index.json` and the 22 class files at build time
(`include_str!`), parses each once, lazily, the first time it is asked for,
and answers questions about a parsed item (`ch_item::Item`). It reads no
file at run time and has no UI; the blueprint engine (`ch-engine`) is its
consumer. Every type a view may render derives `serde::Serialize`.

## What it answers

- **The class and variant.** `class_data(&ItemClass)` finds the class file
  whose `class` equals `ItemClass::as_str()` (`ItemClass::Other` is out of
  scope). `variant_of(&item)` takes the variant from the data base whose
  `name` equals the item's base type exactly (`Amethyst Ring` → `all`,
  `Sirenscale Gloves` → `int`, `Sapphire` → `sapphire`). No base type, or a
  base the data does not list, is reported as unknown (`VariantUnknown`),
  and every lookup that needs a variant then answers unknown, never a guess.
- **What an item can roll.** `ClassData::valid_mods(variant, item_level)`
  lists every modifier with a tier on the variant at or below the item
  level, with those tiers and the best of them; `normal`-section modifiers
  and `desecrated`-section ones (by desecration only) apart.
  `ClassData::mod_tiers(id, variant)` gives one modifier's tiers.
- **Essences and liquid emotions.** `ClassData::essences_for(mod, variant,
  floor)` and `liquids_for` return the entries whose `matches[variant]` is
  that modifier at tier `floor` or better (1 is the best tier).
- **The join.** `join(&item)` matches each prefix and suffix of a parsed
  item to a data modifier and tier (below).
- **Odds.** `add_odds(&item, target, floor, &AddOptions)` is the chance one
  random add rolls the target at the floor or better; `removal_odds(&join,
  wanted, side, Removal)` the chance a removal takes a modifier that is not
  wanted. Each is `Odds::Known` (a percent and its basis, such as `8000 of
  69500 weight`) or `Odds::Unknown` with the reason. One step only: no
  simulation, no expected cost.

## The join

For each explicit modifier (`ModKind::Prefix` or `Suffix`; implicit, rune,
enchant and corruption ones are left out):

1. Candidates are the class's modifiers of the same side whose `lines` equal
   the modifier's line templates in order, exact strings (both keep a
   leading `+`), in either section: a revealed desecrated modifier on a real
   item joins a `normal`-section one.
2. Only candidates with tiers on the item's variant stay.
3. The tier is the one whose name equals the header's affix name; when no
   name or several match, the one whose every range holds the modifier's
   values. `Matched::decided_by` says which rule did.

The result carries the data modifier id, its section and family, the derived
tier, the tier's modifier level (what whittling compares), the game's
`Tier:` and the fractured, desecrated and unrevealed flags; or `Unmatched`
with the reason. An unrevealed modifier is never joined.

Measured on the five distinct real items in `ch-item`'s `fixtures/real/`:
all 18 explicit modifiers join to one data modifier by affix name, and every
derived tier equals the game's `Tier:`; the two revealed desecrated ones
(the crossbow's `Incinerating`, the gloves' `of Exile`) join
`normal`-section modifiers.

## Slots, pools and odds

- **Affix limits** (`AFFIX_LIMITS`, one table): Normal 0 and 0, Magic 1
  prefix and 1 suffix, Rare 3 and 3, Rare jewels 2 and 2. Not established
  in the docs; the owner's default (2026-10-29). A Contempt liquid emotion
  on the item (`+# Suffix Modifier allowed` as a prefix, or the reverse)
  adds its value on the other side.
- **The add pool**: `normal`-section modifiers on the variant, of the sides
  with a free slot (narrowed by a side omen), minus any sharing a family
  with a modifier already on the item; each with its tiers at or below the
  item level. With a Minimum Modifier Level (Greater and Perfect
  currencies, Ancient bones) only tiers at or above it, except that a
  modifier whose every reachable tier is below it keeps its highest
  reachable tier.
- **Add odds**: the target's weight (only its tiers at the floor or better)
  over the pool's. Unknown when the variant's page weights are not `spawn`
  (every jewel page, `Body_Armours_str_dex_int`), when a counting target
  tier's weight is missing or poe2db's placeholder 1, when any pool tier's
  weight is missing, or when a modifier already on the item did not join (its
  family is then unknown). A 1 elsewhere in the pool counts as 1. Anchor: a
  Rare `Amethyst Ring`, item level 76, prefix side, `+# to maximum Life` at
  floor T8 is 8000 of 69500 (11.5%), at T1 1000 of 69500 (1.4%), and with a
  Minimum Modifier Level of 50 at T1 1000 of 22200 (4.5%).
- **Removal odds**: `Random` (chaos, annul, perfect essence, liquid): the
  removable modifiers on the side that are not wanted over all removable
  ones, a fractured one never removable. `Whittle`: the lowest modifier
  level goes, certain when it is unique, unknown on a tie or an unjoined
  modifier. `Desecrated` (Omen of Light with annul): the desecrated
  modifier goes, certain when there is one. `Odds::times` multiplies an
  essence's certain add by its removal factor.
- Desecration and reveal odds are not computed here: no weights are
  established for them (`weights.desecrated` is `not-established`).

## Trade: start weights and the search query

The `trade` module serves the app's *Find a base on trade* panel. It is pure
and offline: it builds the trade site's search JSON as a
`serde_json::Value`, and the address, the league and the link are
`ch-app`'s. The stat ids come from `apps/crafting-helper/data/trade-stats.json`
(embedded as `TRADE_STATS_JSON`, parsed by `trade_stats()`), which
`tools/crafting_helper/trade_stats.py` writes from the site's public stat
list: per class, per modifier id, the site's text and every id carrying it,
or `searchable: false` with the reason (`hybrid`, `no trade stat`).

- **Start weight** (`start_weight(data, variant, item_level, mod, floor)`):
  how rare a modifier is to roll on a base, as a 1-in-N add. The add pool
  of the modifier's own side on the variant (`add_pool`, unchanged) and
  `Pool::odds(mod, floor)`; known with `hits` of `total`, the weight is
  `total / hits`. A floor of `None` ("any tier") is the worst reachable
  tier, so a stricter floor gives a higher weight; no item level reaches
  every tier. A floor that is not a reachable tier, or a modifier outside
  the ordinary pool, is refused. Anchor: Rings (`all`) at item level 82,
  `+# to Accuracy Rating` at T1 (400) weighs 69500 / 400 = 173.75, 2.5
  times `+# to maximum Life` at T1 (1000).
- **The default weight**: where the odds are unknown (the variant's page
  weights are not `spawn`, as on every jewel page and
  `Body_Armours_str_dex_int`; a counting tier with no weight or poe2db's
  placeholder 1), the weight is the median of that side's weighed modifiers
  on the base at any tier (the lower middle for an even count), or 1.0
  (`DEFAULT_WEIGHT_WITHOUT_PEERS`) when none has one. The result says
  `source: default` and carries the reason and the rule; it is never
  silent.
- **The offer** (`trade_offer(class, base, item_level)`): every
  ordinary-pool modifier with a reachable tier, prefixes and suffixes apart,
  each with its lines, reachable tiers, any-tier weight and whether it can
  be searched for (`Searchability`: the site's text and ids, or why not; a
  modifier of more than one line is a hybrid whatever the mapping says).
- **The query** (`trade_query(&TradeSearch)`): a magic item of the base
  (`query.type`), online, not corrupted, with `ilvl.min` only when an item
  level is given, and two stat groups over every stat id of every pick: a
  `count` group asking for 2 when the picks hold a prefix and a suffix and
  *Both slots* is on, else 1 (with `count_line` saying why; a magic item
  holds one prefix and one suffix), whose filters carry `value.min` only
  for a pick whose floor is a chosen tier; and a `weight` group whose
  weight per stat is the trade weight, sorted by `statgroup.1`. The trade
  weight is the rarity weight over the reference value, the floor tier's
  minimum roll (the mean of the two minimums on an `Adds # to #` line), to
  four significant digits: the site sums value times weight, so without it
  a large-valued modifier would outrank a rarer one by its value alone.
  Refused, with the reason, for an unknown class or base, no pick, more
  than `MAX_TRADE_PICKS` (8), a modifier picked twice or outside the
  ordinary pool, an unreachable floor, or a modifier the site has no stat
  for. Each entry of the report carries the weight, its source, the
  reference value and the stat ids.

The site's query keys come from its public filter list (2026-10-29); the
`?q=` link form, the `count` and `weight` group types and the `statgroup`
sort are not confirmed statically and are checked live.

## Tests

```
cargo test --manifest-path apps/crafting-helper/Cargo.toml --locked -p ch-data
cargo clippy --manifest-path apps/crafting-helper/Cargo.toml --locked -p ch-data --all-targets -- -D warnings
```

`tests/data.rs` checks the embedded files against the index, the variant
from the base type, the join and its tiers on the real fixtures (read in
place from `crates/ch-item/fixtures/real/`), the valid modifiers, the ring
anchor above, unknown odds on a jewel and the essence lookup by floor. The
data itself is refreshed by `tools/crafting_helper/poe2db.py`; a refresh
that moves a weight or tier changes the anchors here, and the tests say so.

`tests/trade.rs` checks the trade module on the same data: the ring pool
anchor pinned before it existed, the start weights and their default, the
offer, and the query's keys, count rule, local stat ids, value floors and
refusals. A refresh of `trade-stats.json` that moves an id changes the
Gloves anchors there.
