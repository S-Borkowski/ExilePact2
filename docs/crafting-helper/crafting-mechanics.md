# Crafting mechanics: what each crafting item does

The crafting helper's knowledge base: every currency, omen, essence tier,
desecration bone, alloy, flux, catalyst family and liquid emotion family that
changes the modifiers of an item the helper covers, described in our own words.
The modifier data itself (every modifier, its tiers, item levels and weights)
is not here; it is the extractor's output under `apps/crafting-helper/data/`.
This doc says what the items that add, remove and reroll those modifiers do.

Every entry ends with a `Source:` line naming the poe2db page the claim was
read from and the date it was fetched. A claim the pages did not settle is
marked so in the entry and listed under [Gaps](#gaps). Nothing here is copied
from a page: no page, table or item text is stored in this repository
(AGENTS.md § "Legal").

The facts added from the crafting guides read on 2026-10-30 (issue #35) are
tagged where they stand, `(label: <label>; <source>, <date>)`, with one of
three labels: **community guide** (only a guide says it), **documented**
(Grinding Gear Games or poe2db says it) or **not established** (nothing
settles it, or it is inferred). A tag is an addition to an entry; its
`Source:` line stays. In `docs/models/crafting-slots-spec.md` a
community-guide fact is labelled not established, per that spec's own table.
The facts taken from the community *PoE2 Crafting Codex* for issue #40 are
tagged the same way, each naming the codex section it came from by its
anchor; where the codex disagrees with what is recorded here, the
disagreement is noted beside the fact and the recorded value stays.

`## Crafting patterns seen` paraphrases the crafting routes in the owner's
Discord paste. They seed the M4 blueprints; they are players' routes, not
claims about the game, and each step that names a mechanic links back to the
entry above that describes it.

`tests/test_crafting_helper_mechanics_doc.py` pins this doc's shape: its
sections, the terms a blueprint can name, a source on every entry, the
fourteen posts, the wiki gap, and a label tag at each place a guide's or the
codex's fact stands.

## Sources

Fetched 2026-10-28 from poe2db.tw, one request at a time and at least three
seconds apart, with a User-Agent naming the tool, after reading robots.txt
(which disallows only the `marked` and `markdown_history` paths). The raw
pages were kept on the fetching machine only.

- `https://poe2db.tw/us/Stackable_Currency`: every stackable currency with its
  one-line effect, including the Greater and Perfect orbs, desecration bones,
  alloys, fluxes, every essence and every catalyst.
- `https://poe2db.tw/us/Omen`: every omen with its effect.
- `https://poe2db.tw/us/Essence`: every essence, with the modifier each adds
  per item kind.
- `https://poe2db.tw/us/Desecrated_Modifiers`: the desecrated modifier lists
  and the page's own description of desecration.
- `https://poe2db.tw/us/Liquid_Emotions`: every liquid emotion and the jewel
  modifier it adds for each jewel colour.
- `https://poe2db.tw/us/Catalysts`: every catalyst and the modifier kind its
  quality enhances.
- `https://poe2db.tw/us/Minimum_Modifier_Level`: what the "Minimum Modifier
  Level" line on Greater and Perfect currency means.
- `https://poe2db.tw/us/Modifiers`: fetched to look for the affix limits of a
  rare item; it is an index page and does not state them (they are answered
  from the 2026-10-29 sources below).

Fetched 2026-10-29, for the 0.5 availability and slot facts (game version
0.5.5; facts paraphrased, nothing pasted):

- `https://www.pathofexile.com/forum/view-thread/3932540`: the official 0.5.0
  patch notes (one crafted and one desecrated modifier per item, the
  Standard-only Currency Exchange listings).
- `https://pathofexile2.wiki.fextralife.com/Patch_Notes_040`: a summary of the
  0.4.0 and older patch notes (the omens made unobtainable in 0.3.0, the
  Homogenising drops disabled in 0.4.0).
- `https://poe2db.tw/us/Greater_Orb_of_Augmentation`: a Magic item holds up
  to two random modifiers.
- `https://maxroll.gg/poe2/resources/path-of-exile-2-crafting-overview`: a
  community guide, for the one-and-one and three-and-three split.

Read 2026-10-30, for the guide facts of issue #35. The Mobalytics pages are
the owner's paste of 2026-10-30, each file starting with its source URL and
copy date, kept on the owner's machine outside the repository; only facts
are taken from them, in our own words.

- `https://mobalytics.gg/poe-2/guides/ssf-crafting`, the owner's paste of
  2026-10-30: fracturing beside a desecrated modifier, the Alchemy and
  reforge fallback.
- `https://mobalytics.gg/poe-2/guides/lightning-arrow-crafting`, the
  owner's paste of 2026-10-30: the Abyss mark turning a fracture into a
  one-in-three pick.
- `https://mobalytics.gg/poe-2/guides/isolation-crafting`, the owner's paste
  of 2026-10-30: the isolation principle, and Perfect Exalted's level floor
  keeping a top tier.
- `https://mobalytics.gg/poe-2/guides/fracturing-orbs`, the owner's paste of
  2026-10-30: fracturing as the first step of an isolated craft.
- `https://mobalytics.gg/poe-2/guides/omen-of-recombination`, the owner's
  paste of 2026-10-30: what Omen of Recombination does and when it pays.
- `https://mobalytics.gg/poe-2/guides/campaign-martial-weapon-crafting`, the
  owner's paste of 2026-10-30: the restart rule for a weak magic weapon.
- `https://mobalytics.gg/poe-2/guides/crafting-basics-part-1`, the owner's
  paste of 2026-10-30: hybrid modifiers, item-level breakpoints.
- `https://mobalytics.gg/poe-2/guides/essences`, the owner's paste of
  2026-10-30: one essence modifier per item, Perfect essence removals, the
  Abyss mark.
- `https://mobalytics.gg/poe-2/guides/0-3-crafting-changes`, the owner's
  paste of 2026-10-30: the Greater and Perfect currency levels, Perfect
  essences as a chosen Chaos Orb.
- `https://maxroll.gg/poe2/resources/how-to-craft-in-path-of-exile-2`,
  reviewed 2026-10-30 (issue #35): the rescue of a full rare by a
  side-steered desecration (boots without movement speed).

Fetched 2026-10-30, to settle two questions the guides raised:

- `https://www.pathofexile.com/forum/view-thread/3932540`: the 0.5.0 patch
  notes again, for alloys adding crafted modifiers.
- `https://poe2db.tw/us/Essence_of_the_Abyss` and
  `https://poe2db.tw/us/Essence`: Essence of the Abyss is a corrupted
  essence, and the Crystallisation omens name Perfect or corrupted essences.

Read 2026-10-30, for issue #40's review of the codex (one page, seventeen
anchored sections; the review's verdict per section is in
[the research doc](crafting-helper-research.md#crafting-codex-review-issue-40)):

- `https://domistae.github.io/poe2-leveling/poe2_crafting_codex.html`, read
  2026-10-30: a community summary that says it covers patch 0.5.5 and calls
  itself a quick reference rather than ground truth. Taken from it: modifier
  groups, the 0.5 Greater orb levels, Sanctification's effect, the size of
  Catalysing Exaltation's favour, the 0.5 catalyst sources and caps, a
  Divine Orb leaving a fractured modifier, the Abyss mark's replacement, the
  0.5 removal of Omen of Recombination, lord omens stacked on a Necromancy
  omen, Runic Ward enchants and the crafted slot, the flux count, Instant
  Leech desecrations, the 0.5 Vaal infusers, the item-level breakpoints and
  a source for the dropped league pools. Each is a short fact in our own
  words; nothing of the page is stored.

The other pages of the owner's paste (waystones, Vaal and corruption, gems,
runes, minion gear, league start, essence farming and the Homogenising
crafts) were not used: they are out of the helper's scope.

The slot rules the crafting helper plans with, and the omen-stacking
questions no page settles, are written down and labelled in
[`docs/models/crafting-slots-spec.md`](../models/crafting-slots-spec.md).

Not read: poe2wiki (behind a bot wall agents cannot pass) and craftofexile
(serves no data in its HTML). Both are listed under Gaps.

## Item basics

These are the terms the rest of the doc uses. Where a fact below comes from a
currency's own description, that currency's entry carries the source.

- **Rarity.** An item is Normal (no explicit modifiers), Magic or Rare. Orb of
  Transmutation turns Normal into Magic with one modifier; Orb of Augmentation
  adds a modifier to a Magic item; Regal Orb turns Magic into Rare adding one
  modifier; Orb of Alchemy goes straight from Normal or Magic to a Rare with
  four. Unique items are out of scope for the helper.
- **Prefixes and suffixes.** Every explicit modifier is either a prefix or a
  suffix (the extractor records which). Many omens restrict a currency to one
  side: "Sinistral" means prefixes, "Dextral" means suffixes. A Magic item
  carries at most one prefix and one suffix, a Rare at most three of each
  (poe2db gives a Magic item two modifiers; maxroll's crafting overview gives
  the per-side split; both fetched 2026-10-29). The liquid emotions that
  grant "+1 prefix/suffix modifier allowed" are the way past those limits.
- **Modifier groups.** An item holds at most one modifier of each group
  (the data files' `family`), and an essence whose group is already on the
  item fails without being used up (label: community guide;
  https://domistae.github.io/poe2-leveling/poe2_crafting_codex.html#anatomy,
  read 2026-10-30). The helper's route engine already refuses such an
  essence. The codex's rule of thumb that skill-level modifiers are prefixes
  disagrees with the data, which lists `+# to Level of all Lightning Spell
  Skills` on wands and staves, and Perfect Essence of Battle's `+# to Level
  of all Attack Skills` on every class it takes, as suffixes; the data's side
  is kept (label: documented; poe2db's class pages, fetched 2026-10-28,
  against the codex read 2026-10-30).
- **Bases that change the limits.** Some ring and amulet bases carry an
  implicit that allows one more or one fewer prefix or suffix. In the
  helper's shipped data (poe2db's base rows,
  https://poe2db.tw/us/Rings and https://poe2db.tw/us/Amulets, fetched
  2026-10-28): Dusk Ring and Dusk Amulet, `+1 Prefix Modifier allowed` and
  `-1 Suffix Modifier allowed`; Gloam Ring and Gloam Amulet, the reverse;
  Distorted Amulet, `-1 Suffix Modifier allowed`; Twisted Amulet, `-1
  Prefix Modifier allowed`. So a rare Dusk Ring takes four prefixes and two
  suffixes. The Absent, Lament and Portent Amulet rows also list such a
  line, one of many implicit lines in each row (8, 38 and 8), not their
  one fixed implicit; which of them such an amulet carries is not
  established here. The helper counts every such line on the
  item, implicit or explicit, with its sign, a side never below 0. How the
  game writes these implicits in a `Ctrl+Alt+C` copy is not measured.
- **Crafted and desecrated modifiers.** Since 0.5 an item may carry one
  crafted modifier (an essence's modifier is one) and one desecrated
  modifier, and a desecrated modifier no longer counts as crafted (0.5.0
  patch notes, https://www.pathofexile.com/forum/view-thread/3932540,
  fetched 2026-10-29). So an item holds one essence modifier at a time, and
  an alloy's modifier is that same one crafted modifier (see Alloys)
  (label: documented; the 0.5.0 notes, fetched 2026-10-29 and 2026-10-30;
  the community guide https://mobalytics.gg/poe-2/guides/essences, read
  2026-10-30, says the same of essences).
- **Hybrid modifiers.** A hybrid modifier improves two stats and still takes
  one prefix or suffix slot: poe2db lists each hybrid as one modifier, and
  the owner's real magic boots (the helper's fixture
  `ch-item/fixtures/real/boots-magic-hybrid-unmet-advanced.txt`) hold a
  two-line prefix beside a suffix (label: documented; poe2db's modifier
  pages, fetched 2026-10-28, and the community guide
  https://mobalytics.gg/poe-2/guides/crafting-basics-part-1, read
  2026-10-30). On the plain tooltip a hybrid's value is said to be summed
  into the single-stat line of the same stat, so a line reading 120% can be
  a 90% modifier plus a hybrid's 30%; this is not measured (label: community
  guide; https://mobalytics.gg/poe-2/guides/crafting-basics-part-1, read
  2026-10-30).
- **Item level and modifier level.** Each modifier tier has a level, and an
  item can only roll a tier whose level is at or below the item's level. The
  Greater and Perfect currencies raise a floor instead: they add only modifiers
  of at least their Minimum Modifier Level (see that entry).
- **Tiers.** One modifier family (for example maximum Life) has several tiers
  with rising level and value ranges. Players say "T1" for the best tier an
  item can roll; the data files keep each tier's level and range.
- **Spawn weight.** Equipment modifiers carry a relative weight that decides
  how often a random roll picks them. Desecrated modifiers and jewel modifiers
  carry no usable weight on poe2db; the data files mark those as not
  established.
- **Fractured.** A fractured modifier is locked: later currency does not
  remove or change it (Fracturing Orb).
- **Desecrated and unrevealed.** Desecration adds a hidden modifier that is
  revealed later, choosing from offered options (see Desecration).
- **Quality.** Armour and weapons take ordinary quality currency. Rings and
  amulets (and jewels) take catalyst quality instead, which strengthens one kind
  of modifier (see Catalysts).

## Currency

### Orb of Transmutation

Makes a Normal item Magic with one random modifier. The usual first step from
a white base. Greater and Perfect versions exist (see the next section).

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### Orb of Augmentation

Adds one random modifier to a Magic item that has room for one. Greater and
Perfect versions exist.

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### Regal Orb

Makes a Magic item Rare and adds one random modifier, keeping the modifiers it
had. Omens of Sinistral and Dextral Coronation restrict the added modifier to
one side, but are no longer obtainable since 0.3.0; Omen of Homogenising
Coronation makes it share a type with an existing modifier, and since 0.5 is
on the Currency Exchange only in Standard. Greater and Perfect versions exist.

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28;
https://www.pathofexile.com/forum/view-thread/3932540 and
https://pathofexile2.wiki.fextralife.com/Patch_Notes_040, fetched 2026-10-29

### Orb of Alchemy

Makes a Normal or Magic item Rare with four random modifiers. Omens of
Sinistral and Dextral Alchemy push the result to the most prefixes or the
most suffixes it can have; they are no longer obtainable since 0.3.0.

When Transmutation and Augmentation leave a poor magic item, an Alchemy on
it is a fresh four-modifier try, and a poor rare it produces is still worth
keeping: three rares of the same rarity can be reforged at the bench into one
new unidentified rare (label: community guide;
https://mobalytics.gg/poe-2/guides/ssf-crafting, read 2026-10-30). The same
guide warns against it on physical weapons, where a poor magic base is kept
for the reforge instead. The reforge is not a helper route (see Out of
scope).

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28;
https://pathofexile2.wiki.fextralife.com/Patch_Notes_040, fetched 2026-10-29

### Exalted Orb

Adds one random modifier to a Rare item. The most omen-steered currency in
the game: Sinistral or Dextral Exaltation pick the side, Greater Exaltation
adds two modifiers, Homogenising Exaltation matches an existing modifier's
type and Catalysing Exaltation spends catalyst quality to favour a modifier
kind. Players call a plain use a "slam". Greater and Perfect versions exist.
It needs a free slot: it cannot be used on a Rare that already has six
modifiers. Homogenising Exaltation is on the Currency Exchange only in
Standard since 0.5. Which side a plain slam lands on when one side is full,
and how the omens stack, are labelled in
`docs/models/crafting-slots-spec.md`.

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28;
https://poe2db.tw/us/Greater_Exalted_Orb and
https://www.pathofexile.com/forum/view-thread/3932540, fetched 2026-10-29

### Chaos Orb

On a Rare item, removes one random modifier and adds one new random modifier:
a one-for-one swap, not a full reroll. Omen of Whittling makes it remove the
lowest-level modifier; Sinistral and Dextral Erasure restrict the removal to
prefixes or suffixes. Greater and Perfect versions exist.

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### Orb of Annulment

Removes one random modifier. Sinistral and Dextral Annulment restrict it to
one side, Greater Annulment removes two, and Omen of Light makes it remove
only desecrated modifiers.

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### Divine Orb

Rerolls the numbers within their ranges on an item's modifiers, leaving which
modifiers it has alone. Omen of the Blessed limits it to implicit modifiers.
Whether fractured or desecrated modifiers are rerolled too is not stated (see
Gaps); the codex says a fractured one is left as it is (see Fracturing Orb).

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### Fracturing Orb

On a Rare item with at least four modifiers, picks one at random and fractures
it, locking it in place for the rest of the craft (label: documented;
https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28).

A desecrated modifier is never the one fractured (label: community guide;
https://mobalytics.gg/poe-2/guides/ssf-crafting, read 2026-10-30). Nor is
the Mark of the Abyssal Lord that Essence of the Abyss adds, so a
four-modifier rare carrying the mark fractures the wanted modifier one time
in three instead of one in four (label: community guide;
https://mobalytics.gg/poe-2/guides/ssf-crafting and
https://mobalytics.gg/poe-2/guides/lightning-arrow-crafting, read
2026-10-30). The guides fracture early, on a good base with exactly four
modifiers of which one is excellent, and build the rest of the item around
the locked one (label: community guide;
https://mobalytics.gg/poe-2/guides/fracturing-orbs, read 2026-10-30).

A Divine Orb does not change a fractured modifier's values either (label:
community guide;
https://domistae.github.io/poe2-leveling/poe2_crafting_codex.html#fracture,
read 2026-10-30). The codex's isolation trick fractures more than once on
the same item; whether an item can hold two fractured modifiers is not
established, and the helper's routes fracture once (label: not established;
https://domistae.github.io/poe2-leveling/poe2_crafting_codex.html#tricks,
read 2026-10-30).

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### Orb of Chance

Used on a Normal item, either turns it into a Unique or destroys it, at
random. Omen of Chance prevents the destruction. Uniques are out of scope for
the helper; the orb is listed because blueprints may name it as a way to spend
a failed base.

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### Hinekora's Lock

Lets the item show what the next currency used on it would produce, before
committing. Any change to the item cancels the preview. Whether the preview
can be refused and the currency kept is not stated (see Gaps).

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### Mirror of Kalandra

Makes a mirrored copy of an item. A mirrored item cannot be changed further
(the page does not say so; this is the long-standing meaning of "mirrored"
and is listed under Gaps).

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### Artificer's Orb

Adds an augment socket to a martial weapon, wand, staff or armour piece. The
augments themselves (runes, soul cores) are out of scope.

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### Scroll of Wisdom

Identifies an item. Unidentified items show no modifiers to read.

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### Blacksmith's Whetstone

Raises the quality of a martial weapon.

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### Arcanist's Etcher

Raises the quality of a wand, staff or sceptre.

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### Armourer's Scrap

Raises the quality of an armour piece.

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### Shards

Transmutation, Chance, Regal and Artificer's Shards are listed beside their
orbs with no effect text of their own. How many make a whole orb is not stated
on the page (see Gaps).

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

## Greater and Perfect currency

### Minimum Modifier Level

A line carried by the Greater and Perfect currencies and by the Ancient bones.
Every random modifier such an item adds is at least that level, unless a
modifier type would then be ruled out entirely. Such currency cannot be used
on an item whose item level is below the minimum. In practice: they skip the
low tiers, and they are wasted on modifier pools that only have low-level
tiers (the owner's paste notes jewel modifiers are all level 1).

Source: https://poe2db.tw/us/Minimum_Modifier_Level, fetched 2026-10-28

### Greater Orb of Transmutation

The Orb of Transmutation effect with a Minimum Modifier Level of 44. One
guide gives 55 for the Greater Transmutation and Augmentation orbs; the
helper keeps poe2db's 44 and records the disagreement (label: documented;
44 from poe2db, 55 from the community guide
https://mobalytics.gg/poe-2/guides/0-3-crafting-changes, read 2026-10-30).
The codex explains the gap: it gives 44 since 0.5 and 55 before, so the
other guide's 55 is the older value (label: community guide;
https://domistae.github.io/poe2-leveling/poe2_crafting_codex.html#tiers,
read 2026-10-30).

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### Perfect Orb of Transmutation

The Orb of Transmutation effect with a Minimum Modifier Level of 70.

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### Greater Orb of Augmentation

The Orb of Augmentation effect with a Minimum Modifier Level of 44. The same
guide gives 55 here too; the helper keeps poe2db's 44 (label: documented; 44
from poe2db, 55 from the community guide
https://mobalytics.gg/poe-2/guides/0-3-crafting-changes, read 2026-10-30).
The codex gives 44 since 0.5 and 55 before it, as for Transmutation
(label: community guide;
https://domistae.github.io/poe2-leveling/poe2_crafting_codex.html#tiers,
read 2026-10-30).

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### Perfect Orb of Augmentation

The Orb of Augmentation effect with a Minimum Modifier Level of 70.

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### Greater Regal Orb

The Regal Orb effect with a Minimum Modifier Level of 35.

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### Perfect Regal Orb

The Regal Orb effect with a Minimum Modifier Level of 50.

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### Greater Exalted Orb

The Exalted Orb effect with a Minimum Modifier Level of 35.

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### Perfect Exalted Orb

The Exalted Orb effect with a Minimum Modifier Level of 50. Players use it to
add a modifier that is likely a high tier ("perfect slam").

The level-50 floor does not rule a family out: when every tier of a family
the item can reach is below 50, the family's best reachable tier stays in the
pool. That is the Minimum Modifier Level page's own exception, and the
isolation guide's amulet example relies on it, a Perfect Exalted Orb leaving
only the top tier of a family whose lower tiers sit under 50 (label:
documented;
https://poe2db.tw/us/Minimum_Modifier_Level, fetched 2026-10-28; the
community guide https://mobalytics.gg/poe-2/guides/isolation-crafting, read
2026-10-30, agrees). The helper's modifier pool already keeps that tier
when it applies the floor.

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28;
https://poe2db.tw/us/Minimum_Modifier_Level, fetched 2026-10-28

### Greater Chaos Orb

The Chaos Orb swap with a Minimum Modifier Level of 35 on the added modifier.
Which modifier it removes is still random.

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### Perfect Chaos Orb

The Chaos Orb swap with a Minimum Modifier Level of 50 on the added modifier.

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

## Essences

Essences come in four tiers (Lesser, plain, Greater, Perfect) of the same
families, plus a set of single-tier special essences. Families seen on the
page: the Body (life), the Mind (mana), Enhancement (hybrid defences),
Abrasion, Flames, Ice and Electricity (added or extra damage), Ruin (chaos
resistance), Battle (accuracy, attack skill levels), Sorcery (spell damage,
spell skill levels), Haste (attack speed), the Infinite (attributes), Seeking
(critical hit), Insulation, Thawing and Grounding (fire, cold and lightning
resistance), Alacrity, Opulence and Command. Each essence's modifier depends
on the item kind it is used on; the data files record which essence adds
which modifier on which class.

### Lesser Essence

Turns a Magic item Rare and adds the family's guaranteed modifier at its
lowest tier, like a Regal Orb whose new modifier is chosen.

Source: https://poe2db.tw/us/Essence, fetched 2026-10-28

### Essence (plain tier)

The same Magic-to-Rare upgrade with a guaranteed modifier, at a higher range
than the Lesser tier.

Source: https://poe2db.tw/us/Essence, fetched 2026-10-28

### Greater Essence

The same Magic-to-Rare upgrade at the highest range of the three upgrade
tiers. Some families widen the item kinds at this tier (Greater Essence of
Battle also takes gloves and quivers).

Source: https://poe2db.tw/us/Essence, fetched 2026-10-28

### Perfect Essence

Works on a Rare item instead: removes one random modifier and adds the
family's guaranteed modifier. The perfect modifier is usually a different,
stronger stat than the lower tiers add, and is often limited to one item kind
(for example Perfect Essence of the Body only on body armour). Omens of
Sinistral and Dextral Crystallisation choose which side the removal comes
from. Since 0.5 the essence's modifier is the item's one crafted modifier, and
it is always added.

In effect it is a Chaos Orb whose added modifier is chosen: one random
removal, then a certain add. The removal can be narrowed like any other, by
a fractured modifier, a Crystallisation omen or a full side, so on a set-up
item it can leave a single candidate and be certain (label: community guide;
https://mobalytics.gg/poe-2/guides/0-3-crafting-changes and
https://mobalytics.gg/poe-2/guides/essences, read 2026-10-30; the owner's
reading of 2026-10-30 is the same). When the side the essence's modifier
belongs to is already full, the removal is taken from that side to make
room (label: community guide; https://mobalytics.gg/poe-2/guides/essences,
read 2026-10-30; not established in the slot spec, which labels every
community-guide fact so).

The codex adds that a Crystallisation omen for the other side cannot make
room on a full side: the essence still needs room on its own side, so the
three-and-three cap holds (label: community guide;
https://domistae.github.io/poe2-leveling/poe2_crafting_codex.html#caveats,
read 2026-10-30). The helper's essence-swap rule already refuses a swap with
no room for the essence's modifier. Its 0.5 values for Perfect Essence of
Battle (+3 on two-handed melee weapons and crossbows, +2 on one-handed
weapons and bows) agree with the data, but it calls that modifier a prefix,
where the data lists `+# to Level of all Attack Skills` as a suffix; the
data's side is kept (label: documented; poe2db's essence rows, fetched
2026-10-28, against the codex
https://domistae.github.io/poe2-leveling/poe2_crafting_codex.html#essences,
read 2026-10-30).

Source: https://poe2db.tw/us/Essence, fetched 2026-10-28;
https://www.pathofexile.com/forum/view-thread/3932540, fetched 2026-10-29

### Essence of Hysteria

A special essence that works like a Perfect Essence (remove one random
modifier, add a guaranteed one) and has a different modifier for nearly every
equipment kind: minion skill levels on helmets, critical damage bonus on
gloves, movement speed on boots, block chance on shields, and so on.

Source: https://poe2db.tw/us/Essence, fetched 2026-10-28

### Essence of Delirium

A special essence that removes a random modifier and adds one that allocates
a random notable passive, on body armour only.

Source: https://poe2db.tw/us/Essence, fetched 2026-10-28

### Essence of Horror

A special essence that removes a random modifier and adds a large increase to
the effect of socketed augments, on gloves or boots.

Source: https://poe2db.tw/us/Essence, fetched 2026-10-28

### Essence of Insanity

A special essence that removes a random modifier and adds, on belts, a
modifier that grants two enchantments when the item is later corrupted.

Source: https://poe2db.tw/us/Essence, fetched 2026-10-28

### Essence of the Abyss

A special essence that removes a random modifier and adds the Mark of the
Abyssal Lord to any equipment. What the mark then enables is not stated on
the page (see Gaps).

It is a corrupted essence: poe2db gives its internal type as one, and both
Crystallisation omens act on a Perfect or corrupted essence, so either omen
confines this essence's removal to its side (label: documented;
https://poe2db.tw/us/Essence_of_the_Abyss and https://poe2db.tw/us/Essence,
fetched 2026-10-30).

The mark cannot be fractured, which turns a four-modifier fracture into a
one-in-three pick (see Fracturing Orb) (label: community guide;
https://mobalytics.gg/poe-2/guides/ssf-crafting and
https://mobalytics.gg/poe-2/guides/lightning-arrow-crafting, read
2026-10-30). The essences guide also suggests the mark makes a later reveal
of desecrated modifiers likelier to offer a high tier; nothing confirms it
(label: not established; the community guide
https://mobalytics.gg/poe-2/guides/essences, read 2026-10-30).

The mark does not stay: the next desecration's desecrated modifier takes
its place (label: community guide;
https://domistae.github.io/poe2-leveling/poe2_crafting_codex.html#essences,
read 2026-10-30). The codex repeats the higher-tier claim, so it now has
two guides behind it and still nothing that confirms it (label: not
established; the community guides
https://mobalytics.gg/poe-2/guides/essences and
https://domistae.github.io/poe2-leveling/poe2_crafting_codex.html#essences,
read 2026-10-30).

Source: https://poe2db.tw/us/Essence, fetched 2026-10-28;
https://poe2db.tw/us/Essence_of_the_Abyss and https://poe2db.tw/us/Essence,
fetched 2026-10-30

### Essence of the Breach

A special essence that removes a random modifier and raises the maximum
quality of jewellery by 20%, which lets catalyst quality go higher.

Source: https://poe2db.tw/us/Essence, fetched 2026-10-28

## Desecration

Desecrating a Rare item adds one unrevealed desecrated modifier; if the item
has no free slot, a random existing modifier is removed to make room. The
modifier stays hidden until it is revealed at the Well of Souls, where the
player picks from offered options. An item that already has a desecrated
modifier cannot be desecrated again; removing it (Omen of Light with an Orb
of Annulment) makes room for another attempt. Desecrated modifiers carry the
names of the Abyssal lords (Ulaman's / of Ulaman, Amanamu's / of Amanamu,
Kurgal's / of Kurgal), plus "of the Abyss" on equipment and "Lightless" on
jewels. On the page fetched, every equipment desecrated modifier is level 65
and every jewel one level 1.

Each bone desecrates one item group, in three grades: Gnawed (only on items
up to item level 64), Preserved (no limit) and Ancient (Minimum Modifier
Level 40). The codex gives the Gnawed limit as a modifier level of 64 in
one section and an item level of 64 in another; the helper keeps poe2db's
item level (label: documented; poe2db, fetched 2026-10-28, against the
codex https://domistae.github.io/poe2-leveling/poe2_crafting_codex.html#breakpoints
and its #abyss section, read 2026-10-30).

Since 0.5 the Instant Leech desecrated modifiers no longer roll (label:
community guide;
https://domistae.github.io/poe2-leveling/poe2_crafting_codex.html#abyss,
read 2026-10-30). The helper's data lists none, so it agrees.

### Desecration and reveal

The mechanism above: hidden modifier added, revealed later from a choice of
options, one desecrated modifier per item. A desecration needs no free slot:
on a full item it removes a random existing modifier first. Since 0.5 the
limit is one desecrated modifier per item, and that modifier no longer
counts as the item's crafted one, so an item can carry one of each. The
offered options can include ordinary modifiers unless an omen forces a lord.
Omen of Abyssal Echoes allows one reroll of the offered options at reveal;
Sinistral and Dextral Necromancy pick the side; Sovereign, Liege and
Blackblooded force the lord.

When the side a Necromancy omen picks is already full, the modifier the
desecration removes to make room comes from that same side. No page says so;
it is inferred from the two facts above (a desecration removes first when
the item is full, and the omen fixes the side), and maxroll's rescue of a
full rare relies on it: boots with no movement speed lose a prefix to a
prefix-side desecration, and a later reveal can offer movement speed
(label: not established; inferred, used by the community guide
https://maxroll.gg/poe2/resources/how-to-craft-in-path-of-exile-2, reviewed
2026-10-30 for issue #35).

Source: https://poe2db.tw/us/Desecrated_Modifiers, fetched 2026-10-28 and 2026-10-29;
https://www.pathofexile.com/forum/view-thread/3932540, fetched 2026-10-29

### Gnawed Jawbone

Desecrates a Rare weapon or quiver of item level 64 or lower.

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### Preserved Jawbone

Desecrates a Rare weapon or quiver.

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### Ancient Jawbone

Desecrates a Rare weapon or quiver, with a Minimum Modifier Level of 40.

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### Gnawed Rib

Desecrates a Rare armour piece of item level 64 or lower.

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### Preserved Rib

Desecrates a Rare armour piece.

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### Ancient Rib

Desecrates a Rare armour piece, with a Minimum Modifier Level of 40.

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### Gnawed Collarbone

Desecrates a Rare amulet, ring or belt of item level 64 or lower.

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### Preserved Collarbone

Desecrates a Rare amulet, ring or belt.

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### Ancient Collarbone

Desecrates a Rare amulet, ring or belt, with a Minimum Modifier Level of 40.

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### Altered Collarbone

Desecrates a Rare amulet, ring or belt with a chance of an "otherworldly"
modifier. Which modifiers count as otherworldly is not stated (see Gaps).

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### Preserved Cranium

Desecrates a Rare jewel. The only bone for jewels; jewel desecrations are
the level-1 "Lightless" and lord-named jewel modifiers.

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

## Omens

An omen does nothing on its own. While it sits in the inventory it changes
the next use of one currency, and is used up by it. Only the crafting omens
are listed; the rest (logbook, shrine, strongbox, waystone and similar) are
under Out of scope.

### Omen of Whittling

The next Chaos Orb removes the lowest-level modifier on the item instead of a
random one. Players use it to replace a known weak modifier. How it breaks a
tie between equal levels is not stated (see Gaps).

Source: https://poe2db.tw/us/Omen, fetched 2026-10-28

### Omen of Sinistral Erasure

The next Chaos Orb removes only a prefix.

Source: https://poe2db.tw/us/Omen, fetched 2026-10-28

### Omen of Dextral Erasure

The next Chaos Orb removes only a suffix.

Source: https://poe2db.tw/us/Omen, fetched 2026-10-28

### Omen of Sinistral Alchemy

The next Orb of Alchemy gives the item as many prefixes as it can hold. No
longer obtainable since 0.3.0.

Source: https://poe2db.tw/us/Omen, fetched 2026-10-28;
https://pathofexile2.wiki.fextralife.com/Patch_Notes_040, fetched 2026-10-29

### Omen of Dextral Alchemy

The next Orb of Alchemy gives the item as many suffixes as it can hold. No
longer obtainable since 0.3.0.

Source: https://poe2db.tw/us/Omen, fetched 2026-10-28;
https://pathofexile2.wiki.fextralife.com/Patch_Notes_040, fetched 2026-10-29

### Omen of Sinistral Coronation

The next Regal Orb adds only a prefix. No longer obtainable since 0.3.0.

Source: https://poe2db.tw/us/Omen, fetched 2026-10-28;
https://pathofexile2.wiki.fextralife.com/Patch_Notes_040, fetched 2026-10-29

### Omen of Dextral Coronation

The next Regal Orb adds only a suffix. No longer obtainable since 0.3.0.

Source: https://poe2db.tw/us/Omen, fetched 2026-10-28;
https://pathofexile2.wiki.fextralife.com/Patch_Notes_040, fetched 2026-10-29

### Omen of Homogenising Coronation

The next Regal Orb adds a modifier of the same type as one already on the
item. What "type" means here (family, tag) is not stated (see Gaps). Its
drops were disabled in 0.4.0; since 0.5 it is on the Currency Exchange only
in Standard, and copies already owned still work.

Source: https://poe2db.tw/us/Omen, fetched 2026-10-28;
https://www.pathofexile.com/forum/view-thread/3932540 and
https://pathofexile2.wiki.fextralife.com/Patch_Notes_040, fetched 2026-10-29

### Omen of Greater Exaltation

The next Exalted Orb adds two random modifiers instead of one. Whether it
needs two free slots, and whether with a Sinistral or Dextral Exaltation omen
both modifiers land on that side, are not established; both are labelled in
`docs/models/crafting-slots-spec.md`.

Source: https://poe2db.tw/us/Omen, fetched 2026-10-28

### Omen of Sinistral Exaltation

The next Exalted Orb adds only prefixes.

Source: https://poe2db.tw/us/Omen, fetched 2026-10-28

### Omen of Dextral Exaltation

The next Exalted Orb adds only suffixes.

Source: https://poe2db.tw/us/Omen, fetched 2026-10-28

### Omen of Homogenising Exaltation

The next Exalted Orb adds a modifier of the same type as one already on the
item. Its drops were disabled in 0.4.0; since 0.5 it is on the Currency
Exchange only in Standard, and copies already owned still work.

Source: https://poe2db.tw/us/Omen, fetched 2026-10-28;
https://www.pathofexile.com/forum/view-thread/3932540 and
https://pathofexile2.wiki.fextralife.com/Patch_Notes_040, fetched 2026-10-29

### Omen of Catalysing Exaltation

The next Exalted Orb spends all of the item's catalyst quality to raise the
chance of a modifier of the kind that quality favours. How much the chance
rises per point is not stated (see Gaps).

The codex sizes it: the favoured kind's weight is about five times its
usual at 20% catalyst quality and about seven and a half times at 40%, and
40% is a data point, not a cap (label: community guide;
https://domistae.github.io/poe2-leveling/poe2_crafting_codex.html#omens,
read 2026-10-30). It also says the quality keeps its value after a slam
that misses, which disagrees with poe2db's text that the omen spends all
of it; the helper follows poe2db and counts no favour in its odds (label: not
established; poe2db, fetched 2026-10-28, against the codex
https://domistae.github.io/poe2-leveling/poe2_crafting_codex.html#tags,
read 2026-10-30).

Source: https://poe2db.tw/us/Omen, fetched 2026-10-28

### Omen of Greater Annulment

The next Orb of Annulment removes two modifiers. No longer obtainable since
0.3.0, per the old patch notes; poe2db carries no legacy note and one
community reference still lists it, so treat it as a legacy item.

Source: https://poe2db.tw/us/Omen, fetched 2026-10-28;
https://pathofexile2.wiki.fextralife.com/Patch_Notes_040, fetched 2026-10-29

### Omen of Sinistral Annulment

The next Orb of Annulment removes only a prefix.

Source: https://poe2db.tw/us/Omen, fetched 2026-10-28

### Omen of Dextral Annulment

The next Orb of Annulment removes only a suffix.

Source: https://poe2db.tw/us/Omen, fetched 2026-10-28

### Omen of Light

The next Orb of Annulment removes only desecrated modifiers. Since an item
holds one desecrated modifier, this removes exactly that one and frees the
item for another desecration.

Source: https://poe2db.tw/us/Omen, fetched 2026-10-28

### Omen of Sinistral Crystallisation

The next Perfect or corrupted essence removes only a prefix to make room for
its guaranteed modifier. The owner's paste also uses it with alloys; the omen
text names essences only (see Gaps).

Source: https://poe2db.tw/us/Omen, fetched 2026-10-28

### Omen of Dextral Crystallisation

The next Perfect or corrupted essence removes only a suffix. Same alloy
question as the Sinistral one.

Source: https://poe2db.tw/us/Omen, fetched 2026-10-28

### Omen of Sinistral Necromancy

The next desecration adds only a prefix.

Source: https://poe2db.tw/us/Omen, fetched 2026-10-28

### Omen of Dextral Necromancy

The next desecration adds only a suffix.

Source: https://poe2db.tw/us/Omen, fetched 2026-10-28

### Omen of Abyssal Echoes

The next time desecrated modifiers are revealed, the offered options can be
rerolled once.

Source: https://poe2db.tw/us/Omen, fetched 2026-10-28

### Omen of the Sovereign

The next desecration of a weapon or a piece of jewellery is guaranteed an
Ulaman modifier.

The lord omens (this one, the Liege and the Blackblooded) are used together
with a Sinistral or Dextral Necromancy omen on the same desecration: the
Necromancy omen picks the side and the lord omen the lord (label: community
guide;
https://domistae.github.io/poe2-leveling/poe2_crafting_codex.html#tricks,
read 2026-10-30). The helper's routes do the same: a desecration aimed at
one lord's desecrated modifier on a weapon, ring or amulet names that lord's
omen beside the Necromancy omen (the `lord-omen` rule; the slot spec's fact
`lord-with-side-omen`).

Source: https://poe2db.tw/us/Omen, fetched 2026-10-28

### Omen of the Liege

The next desecration of a weapon or a piece of jewellery is guaranteed an
Amanamu modifier.

Source: https://poe2db.tw/us/Omen, fetched 2026-10-28

### Omen of the Blackblooded

The next desecration of a weapon or a piece of jewellery is guaranteed a
Kurgal modifier.

Source: https://poe2db.tw/us/Omen, fetched 2026-10-28

### Omen of the Blessed

The next Divine Orb rerolls only the implicit modifiers' values.

Source: https://poe2db.tw/us/Omen, fetched 2026-10-28

### Omen of Sanctification

The next Divine Orb used on a Rare item sanctifies it. What sanctifying does
to the item is not stated on the page (see Gaps). The codex says each
modifier's value is multiplied by a random 78% to 122%, the item is marked
Sanctified, and most crafting no longer applies to it afterwards (label:
community guide;
https://domistae.github.io/poe2-leveling/poe2_crafting_codex.html#omens,
read 2026-10-30). The helper names it in no route.

Source: https://poe2db.tw/us/Omen, fetched 2026-10-28

### Omen of Chance

The next Orb of Chance does not destroy the item.

Source: https://poe2db.tw/us/Omen, fetched 2026-10-28

### Omen of Recombination

Listed on the omen page with no effect text. A guide says it makes the next
recombination Lucky: the outcome is rolled twice and the better one kept.
Since a failed first roll gets a second try, the omen is worth its price
when the recombination's chance of failing, times what its input items cost
to replace, is at least the omen's own cost (label: community guide;
https://mobalytics.gg/poe-2/guides/omen-of-recombination, read 2026-10-30).
The helper models no recombination, so it never names this omen in a route.

The codex says 0.5 removed the omen and deleted the copies players held,
and that the Recombinators are switched off for the Runes of Aldur league
(label: community guide;
https://domistae.github.io/poe2-leveling/poe2_crafting_codex.html#caveats,
read 2026-10-30).

Source: not established

## Alloys

An alloy works like a Perfect Essence: on a Rare item it removes one random
modifier and adds a guaranteed one, chosen by the item kind. The modifiers are
ones the ordinary pools do not offer (many about Runic Ward). Each entry names
the item kinds and what the alloy adds there, in short. An alloy's modifier
is the item's one crafted modifier (the 0.5 limit, see Item basics): the
0.5.0 notes describe alloys as adding crafted modifiers by replacing an
existing one, and the same notes allow one crafted modifier per item
(label: documented; https://www.pathofexile.com/forum/view-thread/3932540,
fetched 2026-10-30).

The codex warns that a Runic Ward enchantment from the Verisium Anvil also
takes the one crafted slot, so an item carrying one cannot take an alloy's
or an essence's modifier as well (label: not established; the community
guide https://domistae.github.io/poe2-leveling/poe2_crafting_codex.html#aldur,
read 2026-10-30). The helper reads no enchantment and does not model this.

### Runic Alloy

Maximum Runic Ward on rings, increased maximum Runic Ward on amulets, Runic
Ward regeneration on belts.

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### Adaptive Alloy

Extra fire damage while missing Runic Ward on staves and wands, Puppet Master
stacks for command skills on sceptres, attack speed while missing Runic Ward
on gloves.

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### Protective Alloy

Runic Ward recovery when a charm is used on belts, maximum Runic Ward on
weapons, Runic Ward recovery on block on shields.

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### Expansive Alloy

Remnant pickup range on gloves, presence area on body armour, mana cost
efficiency on helmets, temporary minion limit on boots.

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### Swift Alloy

Cast speed on gloves, attack speed on rings, flask charge gain on belts,
totem placement speed on shields and foci.

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### Cyclonic Alloy

Less slowing from debuffs on body armour, skill effect duration on boots,
damaging ailment duration on gloves, Archon buff duration on helmets.

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### Prismatic Alloy

Elemental penetration on gloves, ailment magnitude on martial weapons,
exposure effect on foci, staves and wands, minion ailment magnitude on
sceptres.

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### Mystic Alloy

Spell area on helmets, attack area on gloves, Spirit on boots, extra chains
on quivers, an extra elemental infusion on caster weapons.

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### Sovereign Alloy

Socketed augment effect on weapons, Runic Ward on armour, a boost to explicit
resistance modifiers on jewellery and belts.

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### Celestial Alloy

A two-stat modifier: maximum mana with spell skill levels on staves and
wands, accuracy with attack speed on martial weapons.

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### Transcendent Alloy

A two-stat modifier: cast speed with extra cold damage on staves, foci and
wands, physical damage with all attributes on martial weapons.

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### The Runebinder's Alloy

Build-specific weapon modifiers: plant overgrowth Archon on staves, elemental
skill limit on wands, Puppet Master stacks on sceptres, ballista totems on
crossbows, mark effect on bows.

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### The Runefather's Alloy

Build-specific weapon modifiers: Glory retention on maces, Tempest Bell hits
on quarterstaves, weapon range on spears, lightning contributing to ignite on
talismans.

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

## Fluxes

poe2db lists five fluxes, the entries below. The codex, working from the
0.5 patch notes, counts three; the helper keeps poe2db's five (label:
documented; https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28,
against the community guide
https://domistae.github.io/poe2-leveling/poe2_crafting_codex.html#aldur,
read 2026-10-30).

### Blazing Flux

Turns every cold and lightning resistance modifier on the item into the
matching fire resistance modifier.

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### Chilling Flux

Turns every fire and lightning resistance modifier into the matching cold
resistance modifier.

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### Crackling Flux

Turns every fire and cold resistance modifier into the matching lightning
resistance modifier.

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### Void Flux

Turns every fire, cold and lightning resistance modifier into the matching
chaos resistance modifier. Players roll a high-tier elemental resistance and
convert it, because chaos resistance rolls are rarer. How "matching" maps
tiers and values is not stated (see Gaps).

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### Perfect Flux

Raises the skills granted by an item to level 20.

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

## Catalysts

Catalysts drop from Breach. Each adds quality to a ring or amulet (the plain
ones) or to a jewel (the Refined ones) that strengthens one kind of modifier,
and replaces any other quality type already on the item. Essence of the Breach
raises the cap on jewellery quality; Omen of Catalysing Exaltation spends the
quality to favour that kind on the next Exalted Orb. How much quality one
catalyst adds and how quality scales a modifier are not stated (see Gaps).
One entry per family; each family has a plain and a Refined version.

The codex says that in 0.5 catalysts come only from the Genesis Tree, not
from Breach as above, and gives the quality caps: 20% normally, 50% on a
Breach Ring, and up to 65% with the Genesis Tree's nodes (label: community
guide;
https://domistae.github.io/poe2-leveling/poe2_crafting_codex.html#quality,
read 2026-10-30). Uul-Netol's Catalyst is one place it disagrees with
poe2db; see that entry.

### Flesh Catalyst

Life modifiers.

Source: https://poe2db.tw/us/Catalysts, fetched 2026-10-28

### Neural Catalyst

Mana modifiers.

Source: https://poe2db.tw/us/Catalysts, fetched 2026-10-28

### Carapace Catalyst

Armour, Evasion and Energy Shield modifiers.

Source: https://poe2db.tw/us/Catalysts, fetched 2026-10-28

### Uul-Netol's Catalyst

Physical modifiers. The codex says attack modifiers; the helper keeps
poe2db's physical (label: documented; poe2db, fetched 2026-10-28, against
the community guide
https://domistae.github.io/poe2-leveling/poe2_crafting_codex.html#quality,
read 2026-10-30).

Source: https://poe2db.tw/us/Catalysts, fetched 2026-10-28

### Xoph's Catalyst

Fire modifiers.

Source: https://poe2db.tw/us/Catalysts, fetched 2026-10-28

### Tul's Catalyst

Cold modifiers.

Source: https://poe2db.tw/us/Catalysts, fetched 2026-10-28

### Esh's Catalyst

Lightning modifiers.

Source: https://poe2db.tw/us/Catalysts, fetched 2026-10-28

### Chayula's Catalyst

Chaos modifiers.

Source: https://poe2db.tw/us/Catalysts, fetched 2026-10-28

### Reaver Catalyst

Attack modifiers.

Source: https://poe2db.tw/us/Catalysts, fetched 2026-10-28

### Sibilant Catalyst

Caster modifiers.

Source: https://poe2db.tw/us/Catalysts, fetched 2026-10-28

### Skittering Catalyst

Speed modifiers.

Source: https://poe2db.tw/us/Catalysts, fetched 2026-10-28

### Adaptive Catalyst

Attribute modifiers.

Source: https://poe2db.tw/us/Catalysts, fetched 2026-10-28

### Necrotic Catalyst

Minion modifiers.

Source: https://poe2db.tw/us/Catalysts, fetched 2026-10-28

## Liquid emotions

Liquid emotions (from Delirium) craft jewels. On a Rare jewel each one
removes a random modifier and adds a guaranteed crafted modifier that depends
on the jewel's colour (Ruby, Sapphire, Emerald, and for some Diamond). The
plain versions work on basic jewels; the "Ancient" versions work on Time-Lost
jewels and add radius-based modifiers instead. Each emotion comes in one
strength named in its title (Diluted, plain, Concentrated or Potent); the
strength is part of the item's name, not a choice. Used outside crafting they
also make an area Delirious, which is out of scope. One entry per family.

### Ire

Defences: increased Armour (Ruby), Energy Shield (Sapphire) or Evasion
(Emerald), all prefixes. Diluted strength.

Source: https://poe2db.tw/us/Liquid_Emotions, fetched 2026-10-28

### Guilt

Fire, cold or lightning damage by colour, prefixes. Diluted strength.

Source: https://poe2db.tw/us/Liquid_Emotions, fetched 2026-10-28

### Greed

Physical, chaos or elemental damage by colour, prefixes. Diluted strength.

Source: https://poe2db.tw/us/Liquid_Emotions, fetched 2026-10-28

### Paranoia

Attack damage (Ruby prefix), cast speed (Sapphire suffix) or attack speed
(Emerald suffix).

Source: https://poe2db.tw/us/Liquid_Emotions, fetched 2026-10-28

### Envy

Mana cost paid with life (Ruby suffix), spell damage (Sapphire prefix) or
cooldown recovery (Emerald suffix).

Source: https://poe2db.tw/us/Liquid_Emotions, fetched 2026-10-28

### Disgust

Life regeneration (Ruby), mana on kill (Sapphire) or life on kill (Emerald),
all suffixes.

Source: https://poe2db.tw/us/Liquid_Emotions, fetched 2026-10-28

### Despair

Rage on melee hit (Ruby), spell critical chance (Sapphire) or attack
critical chance (Emerald), all suffixes.

Source: https://poe2db.tw/us/Liquid_Emotions, fetched 2026-10-28

### Fear

Warcry speed (Ruby), spell critical damage (Sapphire) or attack critical
damage (Emerald), all suffixes. Concentrated strength.

Source: https://poe2db.tw/us/Liquid_Emotions, fetched 2026-10-28

### Suffering

Maximum rage (Ruby prefix), area of effect (Sapphire prefix) or movement
speed (Emerald suffix). Concentrated strength.

Source: https://poe2db.tw/us/Liquid_Emotions, fetched 2026-10-28

### Isolation

On basic jewels, +1% to a maximum resistance by colour (Diamond for chaos),
suffixes. The Ancient version instead converts increases to two elements into
the third within the jewel's radius. Concentrated strength.

Source: https://poe2db.tw/us/Liquid_Emotions, fetched 2026-10-28

### Melancholy

On basic jewels, an on-hit debuff that needs the other two colours socketed
in the tree, a suffix. The Ancient version upgrades a Time-Lost jewel's
radius to Very Large, a prefix: the "radius" step in the owner's jewel
routes. Potent strength.

Source: https://poe2db.tw/us/Liquid_Emotions, fetched 2026-10-28

### Ferocity

Increased effect of the jewel's prefixes (as a suffix) or of its suffixes (as
a prefix), on every colour. The Ancient version adds a resistance to notable
passives in radius. Potent strength.

Source: https://poe2db.tw/us/Liquid_Emotions, fetched 2026-10-28

### Contempt

Allows one more modifier on the other side: as a prefix it allows an extra
suffix, as a suffix an extra prefix. Both the plain and the Ancient versions
do this. The owner's jewel routes use it to fit a fourth modifier on one side
and later remove it. Potent strength.

Source: https://poe2db.tw/us/Liquid_Emotions, fetched 2026-10-28

## Out of scope

Listed so a reader knows they were seen and left out on purpose, not missed.

- **Corruption, Vaal Orb included.** Vaal Orb changes an item unpredictably
  and corrupts it; a corrupted item cannot be crafted further by the
  currencies above. Corruption outcomes are the owner's scope exclusion, and
  so are the currencies that act only on corrupted items or corruption
  enchantments: Architect's Orb, Vaal Cultivation Orb, the four Orbs of
  Sacrifice (Yaomac's, Kopec's, Kamasa's, Yugul's), the Vaal infusers that
  push quality past the cap with a chance to corrupt, Omen of Corruption,
  and Omen of Putrefaction (replaces every modifier with up to six
  unrevealed ones and corrupts the item; the owner's paste mentions it once
  as a gamble). 0.5 split the single Vaal infuser into four, the
  Armourer's, Blacksmith's, Arcanist's and Catalysing infusers, one per kind
  of quality (label: community guide;
  https://domistae.github.io/poe2-leveling/poe2_crafting_codex.html#vaal,
  read 2026-10-30); recorded only, as corruption stays out of scope.
- **Uniques**: Omen of the Ancients (turns an Orb of Chance result into a
  random unique of the class) and Vaal Cultivation Orb.
- **Skill gems**: Gemcutter's Prism, the Jeweller's Orbs, Crystallised
  Corruption, Thaumaturgic Flux.
- **Augments and sockets**: runes and soul cores, Orb of Extraction, Core
  Destabiliser.
- **Item classes the helper does not cover**: flasks and charms (Glassblower's
  Bauble), waystones (Preserved Vertebrae, the waystone Chaos omens), tablets
  (Ancient Infuser).
- **The dropped league pools**: the `chronomancy`, `decay`, `breach_caster`
  and `breach_minion` modifier groups the extractor leaves out. The codex
  says their modifiers come from meta-crafting runes and the Genesis Tree's
  branches (label: community guide;
  https://domistae.github.io/poe2-leveling/poe2_crafting_codex.html#aldur,
  read 2026-10-30). Still not read into the data; a follow-up.
- **Omens with no crafting effect**: the Sagas (logbooks), Refreshment,
  Resurgence, Amelioration, Answered Prayers, Secret Compartments, the Hunt,
  Reinforcements, Gambling, Bartering.
- **Non-crafting stackables**: splinters, artifacts, Rogue's Marker, Gold,
  Verisium, Starlit Ore, crests, Cryptic Key.
- **Delirium side effect** of liquid emotions (making an area Delirious).
- **The reforge bench**: three items of the same rarity go in and one new
  unidentified item of that rarity comes out, so three poor rares become
  one fresh try (label: community guide;
  https://mobalytics.gg/poe-2/guides/ssf-crafting, read 2026-10-30). A
  helper route follows one item, which a three-into-one step cannot, so it
  is recorded here and under Orb of Alchemy, not offered as a route.
- **Omen of Recombination as a route step**: the helper models no
  recombination; the omen's effect is recorded in its entry.

## Crafting patterns seen

Paraphrased from the owner's Discord paste (a local file, never copied into
the repository), split at its `---` lines into fourteen chunks. The owner
counted thirteen; chunk 7 holds no craft and is kept so the numbering matches
the paste. "Prefix exalt" and the like mean the currency with the omen that
restricts it to that side. Item kinds marked "inferred" are read from the
modifiers a post names, not stated in it.

### Post 1

- Item kind: not stated; an item carrying a 20% quality modifier and a global
  modifier.
- Goal: a cheaper finish (the poster counts the saving in Divine Orbs, the
  trade unit).
- Steps: desecrate before annulling and reveal the desecrated modifier; then
  use Omen of Whittling with a Chaos Orb, which the poster says reliably takes
  off the quality modifier because it is the lowest level; then add the
  global modifier.

Source: Discord post 1, owner's paste, 2026-10-28

### Post 2

- Item kind: jewellery, an amulet (inferred from defence catalyst quality and
  the global-defence perfect essence).
- Goal: finish the craft with high defence quality and a global defence
  modifier.
- Steps: Sinistral Exaltation with an Exalted Orb; Sinistral Crystallisation
  with Essence of the Breach for the extra quality cap, bringing defence
  quality to 40; Sinistral Annulment; another prefix exalt; Sinistral
  Crystallisation with the global defence essence; finally desecrate for
  what the poster calls skill quality (which desecrated modifier is meant is
  not clear from the post).

Source: Discord post 2, owner's paste, 2026-10-28

### Post 3

- Item kind: Time-Lost jewel (inferred from the Ancient liquids and radius).
- Goal: two critical modifiers, a desecrated modifier and the Very Large
  radius, with no budget limit.
- Steps:
  1. Start from a jewel that already has two critical suffixes.
  2. Ancient Potent Liquid Contempt for one extra suffix slot; restart on a
     miss.
  3. Dextral Exaltation with a plain Exalted Orb (Greater and Perfect are
     wasted: jewel modifiers are all level 1).
  4. On a bad suffix, either Dextral Annulment then exalt again, or Dextral
     Erasure with Chaos Orbs, until the suffixes are right. The poster calls
     this the first costly stage.
  5. Remove the Contempt modifier with Sinistral Erasure or Sinistral
     Annulment.
  6. Desecrate on the prefix side (Sinistral Necromancy) and leave it
     unrevealed.
  7. Ancient Potent Liquid Melancholy for the radius; if it removed the
     desecrated modifier, desecrate again, and repeat until the jewel holds
     both.
  8. Reveal with Omen of Abyssal Echoes; use Omen of Light and redesecrate
     until the result is good.

Source: Discord post 3, owner's paste, 2026-10-28

### Post 4

- Item kind: jewel (inferred, follows post 3).
- Goal: three critical modifiers plus mana, on a radius jewel.
- Steps: none given; the poster says suffix-only Chaos Orbs would be needed
  and that is very costly or very random, so gambling the five modifiers
  with Omen of Putrefaction (a corruption, out of scope) may be the better
  bet.

Source: Discord post 4, owner's paste, 2026-10-28

### Post 5

- Item kind: jewel (inferred, same thread).
- Goal: increased mana as a prefix.
- Steps: keep the desecration for the prefix side.

Source: Discord post 5, owner's paste, 2026-10-28

### Post 6

- Item kind: any jewel, basic or Time-Lost (the poster says the route is the
  same for all).
- Goal: three wanted modifiers on one side, then fill the other.
- Steps:
  1. Buy a cheap jewel with two wanted modifiers on the same side.
  2. Gamble the Potent Liquid Contempt for the extra slot on that side;
     restart on a miss.
  3. Desecrate the third modifier; use Abyssal Echoes and Omen of Light as
     often as needed until the three are right.
  4. Remove the Contempt modifier with a side-restricted annul or chaos.
  5. Exalt into any empty slot.
  6. Chaos the other side's two modifiers until satisfied (stop at one good
     one to gamble, or keep going for two).
  7. Last, the final liquid: radius on Time-Lost jewels, increased effect on
     the rest.
- Limit: does not work when the desecrated modifier wanted is an unusual one
  on the two-modifier side.

Source: Discord post 6, owner's paste, 2026-10-28

### Post 7

No craft: the chunk only points to video crafting guides.

Source: Discord post 7, owner's paste, 2026-10-28

### Post 8

- Item kind: boots (inferred from movement speed and Essence of Horror).
- Goal: movement speed, Runic Ward, both attributes, augment effect, with the
  chosen runes.
- Steps: start from a base with a fractured modifier and two sockets; Chaos
  Orb until an attribute lands; Sinistral Exaltation with an Exalted Orb for
  a prefix; Sinistral Crystallisation with Essence of Horror; exalt another
  prefix; socket a +1 rune; an alloy with Sinistral Crystallisation for Runic
  Ward; desecrate a suffix until the other attribute appears; exalt a last
  random prefix; socket the wanted runes.

Source: Discord post 8, owner's paste, 2026-10-28

### Post 9

- Item kind: gloves (inferred from Essence of Hysteria for critical damage,
  flat lightning to attacks and melee skill levels).
- Goal: critical damage, melee skill levels, attack speed, a desecrated
  modifier.
- Steps: exalt a suffix; Essence of Hysteria with Dextral Crystallisation;
  desecrate a prefix for a common, heavily weighted defensive modifier that
  blocks the prefix pool; Perfect Exalted Orb with Sinistral Exaltation; Omen
  of Light with an annul, then desecrate a suffix; then repeat perfect exalts
  and desecrations until attack speed lands. If the slammed prefixes are
  poor and lower level than the ones to keep, Omen of Whittling clears them.
  The poster warns that without a large budget it is mostly luck.

Source: Discord post 9, owner's paste, 2026-10-28

### Post 10

- Item kind: ring.
- Goal: three top-tier resistances and flat damage (the poster notes the
  asker's suffix needs may differ).
- Steps: Chaos Orb for the first top-tier resistance; Omen of Whittling or a
  Perfect Exalted Orb for the second; whittle for the third; whittle for the
  second top-tier flat damage; desecrate for the third. No other omens: brute
  force with whittling.

Source: Discord post 10, owner's paste, 2026-10-28

### Post 11

- Item kind: ring.
- Goal: flat attack damage prefixes with resistance suffixes.
- Steps: Essence of the Breach; Reaver Catalyst to 40% attack quality; Chaos
  Orb until the second flat attack modifier; desecrate or perfect-exalt the
  last flat; whittle the suffixes until resistances land. The poster warns
  rings get costly on bad luck.

Source: Discord post 11, owner's paste, 2026-10-28

### Post 12

- Item kind: weapon, caster (staff or wand) or physical (inferred from the
  alloys named).
- Goal: a budget route to a good weapon.
- Steps: Chaos Orb for a top-two tier of spell or physical damage; exalt a
  suffix; Dextral Crystallisation with Celestial Alloy; exalt another suffix
  (if it is cast speed, annul the suffix and retry); Dextral Crystallisation
  with Transcendent Alloy; then a side-restricted Perfect Exalted Orb on one
  side and a desecration on the other.

Source: Discord post 12, owner's paste, 2026-10-28

### Post 13

- Item kind: caster weapon or focus (inferred from cast speed by alloy).
- Goal: cast speed and a filled item.
- Steps: exalt a prefix; an alloy for cast speed with Sinistral
  Crystallisation; desecrate to block the mana modifier from rolling; Omen of
  Greater Exaltation with a Perfect Exalted Orb; Omen of Light, then desecrate
  the last slot.

Source: Discord post 13, owner's paste, 2026-10-28

### Post 14

- Item kind: not stated; resistance and intelligence suffixes with life,
  mana or hybrid energy-shield prefixes and a desecrated Spirit modifier (an
  amulet fits, inferred).
- Goal: top-tier resistances, top-tier prefixes, a desecrated Spirit modifier.
- Steps: annul down to one modifier and Chaos Orb for any top-tier
  resistance; Void Flux to make it chaos resistance; side-restricted Perfect
  Exalted Orb for another top-tier resistance; Blazing Flux to make that one
  fire; if a resistance misses, annul and restart from the right step;
  desecrate a suffix for top-tier intelligence or a resistance to block the
  suffix pool, with Echoes and Light as needed; two perfect prefix slams,
  whittling a miss (the second for life, mana or hybrid energy shield); last,
  Omen of Light and desecrate again for the Spirit modifier.

Source: Discord post 14, owner's paste, 2026-10-28

## Crafting principles from the guides

The practice the crafting guides of 2026-10-30 teach, beyond single
mechanics. They are players' advice, labelled as such; the helper's route
engine follows them where `docs/models/crafting-slots-spec.md` says so.

- **Isolation.** A random step becomes a sure one when everything but the
  wanted outcome is excluded before it is taken: a fractured modifier is
  never removed, a side omen rules out the other side, Whittling picks the
  lowest-level modifier, and a Minimum Modifier Level drops the low tiers.
  Set the item up so those exclusions leave one candidate, and the step
  cannot miss (label: community guide;
  https://mobalytics.gg/poe-2/guides/isolation-crafting and
  https://mobalytics.gg/poe-2/guides/fracturing-orbs, read 2026-10-30).
  The slot spec's "Certain removals first" rule is our use of it.
- **Gamble last.** The same guides order a craft so that its certain steps
  come first and the step that can still fail comes at the end, when
  nothing more depends on it (label: community guide;
  https://mobalytics.gg/poe-2/guides/isolation-crafting, read 2026-10-30).
- **Restart a weak magic start.** For a magic martial weapon, a damage
  prefix in its family's best or second-best tier is worth a Regal Orb or
  an essence; anything lower is worth a new base instead (label: community
  guide; https://mobalytics.gg/poe-2/guides/campaign-martial-weapon-crafting,
  read 2026-10-30).
- **Item-level breakpoints.** A tier only rolls on an item whose level
  reaches the tier's, so the target tiers set the base's minimum item
  level. The guide's examples: the top elemental resistance tier needs item
  level 82, and boots' top movement speed tier 82 and the second 70 (label:
  community guide; https://mobalytics.gg/poe-2/guides/crafting-basics-part-1,
  read 2026-10-30). The helper's shipped data, from poe2db's boots pages
  fetched 2026-10-28, has the top tier at 82 but the second (Cheetah's) at
  65 on every boots base, `prefix-increased-movement-speed`; the helper keeps
  the data's 65 (label: documented; poe2db, fetched 2026-10-28, against
  the guide read 2026-10-30). The codex agrees with the data on five
  breakpoints: movement speed's top tier at 82; the top fire, cold and
  lightning resistance tiers at 82; body armour's top chaos resistance tier
  at 81; a wand's top lightning spell level tier at 81; and a bow's top
  additional-arrow tier at 82. It gives movement speed's second tier as 70,
  like the other guide, where the data says 65, and the data's 65 stays
  (label: community guide;
  https://domistae.github.io/poe2-leveling/poe2_crafting_codex.html#breakpoints,
  read 2026-10-30). `apps/crafting-helper/crates/ch-data/tests/codex_breakpoints.rs`
  pins all six against the data and the trade link's item-level floor.
- **Alchemy and the reforge fallback.** A magic start that went badly can be
  turned rare by an Orb of Alchemy for a second chance, and poor rares can
  be fed three at a time to the reforge bench for a new unidentified one
  (label: community guide; https://mobalytics.gg/poe-2/guides/ssf-crafting,
  read 2026-10-30). Neither is a helper route; see Orb of Alchemy and Out
  of scope.

## Gaps

Questions the poe2db pages did not answer. Each is open until someone reads
a better source or measures it in the game.

- **poe2wiki was not read.** Its crafting pages sit behind an Anubis bot wall
  that agents cannot pass; they need a browser. Reading them and filling the
  gaps below is the owner's to do (or a later workorder's), as an edit to
  this doc. craftofexile was not read either: its pages serve no data in HTML.
- **Affix limits** (answered 2026-10-29). A Magic item carries one prefix
  and one suffix, a Rare three and three: poe2db's Greater Orb of
  Augmentation page (https://poe2db.tw/us/Greater_Orb_of_Augmentation) gives
  a Magic item two modifiers, and maxroll's crafting overview
  (https://maxroll.gg/poe2/resources/path-of-exile-2-crafting-overview)
  gives the split. See Item basics.
- **Omen stacking and slot edge cases.** Whether Omen of Greater Exaltation
  with a Sinistral or Dextral Exaltation omen adds two modifiers of that
  side, whether Greater Exaltation needs two free slots, whether an Exalted
  Orb lands on the open side when the other is full, whether whittling
  picks by modifier level, and whether a fractured modifier is ever removed
  are not established (community guides or inference only). They are
  labelled, with their sources, in `docs/models/crafting-slots-spec.md`.
- **Omen of Recombination**: listed with no effect text on poe2db. A
  community guide (2026-10-30) describes it as making a recombination
  Lucky; no GGG or poe2db text confirms it, so its entry stays not
  established. The codex
  (https://domistae.github.io/poe2-leveling/poe2_crafting_codex.html#caveats,
  read 2026-10-30) says 0.5 removed the omen, deleting the copies players
  held, and switched the Recombinators off for the Runes of Aldur league;
  if so, the question would not matter to a 0.5 craft (community guide;
  not established).
- **Crystallisation on alloys.** The omen text names Perfect or corrupted
  essences only; posts 8, 12 and 13 use it with alloys. Not established
  whether alloys count.
- **Which essences are "corrupted".** The Crystallisation omens name them;
  the essence page does not label any. Essence of the Abyss is one (poe2db's
  Essence of the Abyss page, fetched 2026-10-30, gives its type).
  Hysteria, Delirium, Horror, Insanity and the Breach, the other single-tier
  ones, are the likely rest of the set, not checked.
- **Essence of the Abyss**: what the Mark of the Abyssal Lord enables,
  answered in part. Community guides (2026-10-30) say the mark cannot be
  fractured, which the helper's fracture route uses; the essences guide's
  hint that it raises the tier of a later reveal is not established, and
  the codex
  (https://domistae.github.io/poe2-leveling/poe2_crafting_codex.html#essences,
  read 2026-10-30) is a second guide making the same claim without
  evidence. The codex also says the next desecration's modifier replaces
  the mark.
  (Whether an alloy's modifier is the item's crafted one left this list on
  2026-10-30: the 0.5.0 patch notes settle it; see Alloys.)
- **Omen of Sanctification**: what sanctifying a Rare item does, answered
  in part. The codex
  (https://domistae.github.io/poe2-leveling/poe2_crafting_codex.html#omens,
  read 2026-10-30) gives a random 78% to 122% multiplier on each
  modifier's value and says most crafting stops applying; no GGG or poe2db
  text confirms it.
- **Homogenising omens**: what "the same type" matches (family, tag group).
- **Omen of Whittling**: how a tie between equal-level modifiers is broken,
  and whether fractured or desecrated modifiers can be its target.
- **Divine Orb** on fractured and desecrated modifiers, answered in part:
  the codex
  (https://domistae.github.io/poe2-leveling/poe2_crafting_codex.html#fracture,
  read 2026-10-30) says a fractured modifier is left unchanged; desecrated
  modifiers are still open.
- **Hinekora's Lock**: whether a previewed result can be declined.
- **Mirror of Kalandra**: that a mirrored item is locked is long-standing
  knowledge, not stated on the page.
- **Shards**: how many combine into a whole orb.
- **Catalysts**: quality added per catalyst, the normal cap, and how quality
  scales the favoured modifiers; how much Catalysing Exaltation raises the
  chance. Answered in part by the codex
  (https://domistae.github.io/poe2-leveling/poe2_crafting_codex.html#quality
  and its #omens section, read 2026-10-30): a 20% normal cap (50% on a
  Breach Ring, 65% with Genesis Tree nodes) and a favour of about five
  times at 20% quality and seven and a half at 40%. Quality per catalyst,
  and how quality scales a modifier's values, are still open.
- **Fluxes**: how a converted resistance's tier and value map onto the new
  element.
- **Desecration levels.** Gnawed bones are limited to item level 64, yet
  every equipment desecrated modifier on the page is level 65; whether the
  page lists only one tier per modifier is not established. The codex calls
  the Gnawed limit a modifier level in one section and an item level in
  another (see Desecration). Desecrated and
  jewel modifiers publish no usable spawn weight.
- **Altered Collarbone**: which modifiers are "otherworldly".
