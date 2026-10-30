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

`## Crafting patterns seen` paraphrases the crafting routes in the owner's
Discord paste. They seed the M4 blueprints; they are players' routes, not
claims about the game, and each step that names a mechanic links back to the
entry above that describes it.

`tests/test_crafting_helper_mechanics_doc.py` pins this doc's shape: its
sections, the terms a blueprint can name, a source on every entry, the
fourteen posts and the wiki gap.

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
  fetched 2026-10-29).
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
Gaps).

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### Fracturing Orb

On a Rare item with at least four modifiers, picks one at random and fractures
it, locking it in place for the rest of the craft.

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

The Orb of Transmutation effect with a Minimum Modifier Level of 44.

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### Perfect Orb of Transmutation

The Orb of Transmutation effect with a Minimum Modifier Level of 70.

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

### Greater Orb of Augmentation

The Orb of Augmentation effect with a Minimum Modifier Level of 44.

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

Source: https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28

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

Source: https://poe2db.tw/us/Essence, fetched 2026-10-28

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
Level 40).

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
to the item is not stated on the page (see Gaps).

Source: https://poe2db.tw/us/Omen, fetched 2026-10-28

### Omen of Chance

The next Orb of Chance does not destroy the item.

Source: https://poe2db.tw/us/Omen, fetched 2026-10-28

### Omen of Recombination

Listed on the omen page with no effect text.

Source: not established

## Alloys

An alloy works like a Perfect Essence: on a Rare item it removes one random
modifier and adds a guaranteed one, chosen by the item kind. The modifiers are
ones the ordinary pools do not offer (many about Runic Ward). Each entry names
the item kinds and what the alloy adds there, in short. Whether an alloy's
modifier is the item's one crafted modifier (the 0.5 limit, see Item basics)
is not established: the 0.5.0 notes liken alloys to Perfect Essences without
saying so, and only community guides state it (see Gaps).

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

Physical modifiers.

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
  as a gamble).
- **Uniques**: Omen of the Ancients (turns an Orb of Chance result into a
  random unique of the class) and Vaal Cultivation Orb.
- **Skill gems**: Gemcutter's Prism, the Jeweller's Orbs, Crystallised
  Corruption, Thaumaturgic Flux.
- **Augments and sockets**: runes and soul cores, Orb of Extraction, Core
  Destabiliser.
- **Item classes the helper does not cover**: flasks and charms (Glassblower's
  Bauble), waystones (Preserved Vertebrae, the waystone Chaos omens), tablets
  (Ancient Infuser).
- **Omens with no crafting effect**: the Sagas (logbooks), Refreshment,
  Resurgence, Amelioration, Answered Prayers, Secret Compartments, the Hunt,
  Reinforcements, Gambling, Bartering.
- **Non-crafting stackables**: splinters, artifacts, Rogue's Marker, Gold,
  Verisium, Starlit Ore, crests, Cryptic Key.
- **Delirium side effect** of liquid emotions (making an area Delirious).

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
- **Alloys and the crafted limit.** Whether an alloy's modifier counts as the
  item's one crafted modifier since 0.5; only community guides say so.
- **Omen of Recombination**: listed with no effect text.
- **Crystallisation on alloys.** The omen text names Perfect or corrupted
  essences only; posts 8, 12 and 13 use it with alloys. Not established
  whether alloys count.
- **Which essences are "corrupted".** The Crystallisation omens name them;
  the essence page does not label any. Hysteria, Delirium, Horror, Insanity,
  the Abyss and the Breach are the single-tier ones and are the likely set.
- **Essence of the Abyss**: what the Mark of the Abyssal Lord enables.
- **Omen of Sanctification**: what sanctifying a Rare item does.
- **Homogenising omens**: what "the same type" matches (family, tag group).
- **Omen of Whittling**: how a tie between equal-level modifiers is broken,
  and whether fractured or desecrated modifiers can be its target.
- **Divine Orb** on fractured and desecrated modifiers.
- **Hinekora's Lock**: whether a previewed result can be declined.
- **Mirror of Kalandra**: that a mirrored item is locked is long-standing
  knowledge, not stated on the page.
- **Shards**: how many combine into a whole orb.
- **Catalysts**: quality added per catalyst, the normal cap, and how quality
  scales the favoured modifiers; how much Catalysing Exaltation raises the
  chance.
- **Fluxes**: how a converted resistance's tier and value map onto the new
  element.
- **Desecration levels.** Gnawed bones are limited to item level 64, yet
  every equipment desecrated modifier on the page is level 65; whether the
  page lists only one tier per modifier is not established. Desecrated and
  jewel modifiers publish no usable spawn weight.
- **Altered Collarbone**: which modifiers are "otherworldly".
