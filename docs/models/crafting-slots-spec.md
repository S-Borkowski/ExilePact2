# Crafting slots: the mechanism behind state-ordered routes

This spec writes down, in our own words, how the crafting items the
Crafting Helper's state-ordered blueprints use change an item's modifier
slots, and the rules a route follows on them. The stdlib model
(`tools/crafting_helper/slot_model.py`) is built from this file alone, and
the engine (`ch-engine`) is held to the model through the shared case file
`apps/crafting-helper/crates/ch-engine/tests/items/slots/cases.json`. Why
the design is shaped this way: [ADR 0001](../adr/0001-crafting-helper-route-design.md).

A gap this spec does not answer is fixed here, with a label, before the
model or the engine guesses it.

## Labels and sources

Game: Path of Exile 2 0.5.5 (the latest patch found; the 0.5 league, Runes
of Aldur). Every fact below was fetched from public pages: GGG's patch
notes, poe2db's item pages and community guides, most on 2026-10-29; the
facts from the crafting guides of issue #35 on 2026-10-30 (the Mobalytics
guides as the owner copied them that day, maxroll's crafting guide as the
issue reviewed it), each with its own date. Nothing here comes from the
game client, and no page's text is copied. The official 0.5.5
notes were not fetched; no crafting change in 0.5.5 was found, so the
rules are 0.5.0's.

Each fact carries one label:

| Label | Means | From the research's label |
|---|---|---|
| measured | Observed in the game by this project, with the session named. | (none yet; no crafting rule has been measured) |
| documented | Stated by GGG's patch notes, poe2db's item text, or the wiki. | `documented` |
| not established | Stated only by a community guide (named in the fact), inferred from other facts, or answered by no source. | `community` and `not established` |

A route may rely on a not-established fact; when it does, its blueprint
names the fact's id in `relies_on`. `ch-blueprint` knows exactly the ids of
the rows labelled not established below, and a test fails when the two
lists differ. A later live session changes a label to measured and names
the session; it does not change the id.

## Facts

| id | label | fact | source |
|---|---|---|---|
| `magic-slots` | documented | A magic item holds at most one prefix and one suffix. | https://poe2db.tw/us/Greater_Orb_of_Augmentation, fetched 2026-10-29 |
| `rare-slots` | documented | A rare item holds at most three prefixes and three suffixes; an Exalted Orb cannot be used on a rare with six. | https://poe2db.tw/us/Greater_Exalted_Orb, fetched 2026-10-29 |
| `transmute-adds-one` | documented | An Orb of Transmutation (any grade) makes a normal item magic with one modifier. | https://poe2db.tw/us/Greater_Orb_of_Transmutation, fetched 2026-10-29 |
| `augment-adds-one` | documented | An Orb of Augmentation (any grade) adds one random modifier to a magic item with a free slot. | https://poe2db.tw/us/Greater_Orb_of_Augmentation, fetched 2026-10-29 |
| `exalt-adds-one` | documented | An Exalted Orb (any grade) adds one random modifier to a rare item. | https://poe2db.tw/us/Greater_Exalted_Orb, fetched 2026-10-29 |
| `currency-min-level` | documented | Greater and Perfect Exalted Orbs add a modifier of level at least 35 and 50; Greater and Perfect Transmutation and Augmentation at least 44 and 70. A modifier family whose every tier the item can roll is below the minimum keeps its best such tier in the pool, so a Perfect Exalted Orb can still add it (poe2db's own exception to the minimum; Mobalytics' isolation guide relies on it for an amulet). Mobalytics' 0.3 crafting-changes guide gives the Greater Transmutation and Augmentation a minimum of 55; poe2db says 44, and 44 is kept. | https://poe2db.tw/us/Perfect_Exalted_Orb and https://poe2db.tw/us/Perfect_Orb_of_Augmentation, fetched 2026-10-29; https://poe2db.tw/us/Minimum_Modifier_Level, fetched 2026-10-28; https://mobalytics.gg/poe-2/guides/isolation-crafting and https://mobalytics.gg/poe-2/guides/0-3-crafting-changes, fetched 2026-10-30 |
| `greater-exaltation-adds-two` | documented | Omen of Greater Exaltation makes the next Exalted Orb add two random modifiers. | https://poe2db.tw/us/Omen_of_Greater_Exaltation, fetched 2026-10-29 |
| `side-exaltation` | documented | Omen of Sinistral (Dextral) Exaltation makes the next Exalted Orb add only a prefix (suffix). | https://poe2db.tw/us/Omen, fetched 2026-10-29 |
| `annulment-removes-one` | documented | An Orb of Annulment removes one random modifier from a magic or rare item. | https://poe2db.tw/us/Orb_of_Annulment, fetched 2026-10-29 |
| `side-annulment` | documented | Omen of Sinistral (Dextral) Annulment makes the next Orb of Annulment remove only a prefix (suffix). | https://poe2db.tw/us/Omen, fetched 2026-10-29 |
| `light-removes-desecrated` | documented | Omen of Light makes the next Orb of Annulment remove only a desecrated modifier. | https://poe2db.tw/us/Omen, fetched 2026-10-29 |
| `whittling-lowest` | documented | Omen of Whittling makes the next Chaos Orb remove the item's lowest-level modifier before it adds a random one. | https://poe2db.tw/us/Omen_of_Whittling, fetched 2026-10-29 |
| `omen-activation` | documented | An omen works while it is active in the inventory and is used up by the one currency its text names. | https://poe2db.tw/us/Omen_of_Greater_Exaltation and https://poe2db.tw/us/Omen, fetched 2026-10-29 |
| `essence-magic-to-rare` | documented | A Lesser, plain or Greater essence makes a magic item rare and adds its modifier; Perfect and corrupted essences work on a rare by replacing a modifier. | https://poe2db.tw/us/Essence, fetched 2026-10-29 |
| `one-crafted-one-desecrated` | documented | Since 0.5 an item has at most one crafted modifier (an essence's among them) and at most one desecrated modifier. Mobalytics' essences guide agrees: one essence modifier per item at a time. | https://www.pathofexile.com/forum/view-thread/3932540, fetched 2026-10-29; https://mobalytics.gg/poe-2/guides/essences, fetched 2026-10-30 |
| `alloy-counts-as-crafted` | documented | An alloy's modifier is the item's one crafted modifier: the 0.5.0 notes have alloys add crafted modifiers by replacing a modifier already on the item, and an item holds at most one crafted modifier. | https://www.pathofexile.com/forum/view-thread/3932540, fetched 2026-10-30 |
| `side-crystallisation` | documented | Omen of Sinistral (Dextral) Crystallisation makes the next Perfect or corrupted essence remove only a prefix (suffix). Essence of the Abyss is a corrupted essence (poe2db's internal type for it), so both omens apply to it. | https://poe2db.tw/us/Omen, fetched 2026-10-28; https://poe2db.tw/us/Essence_of_the_Abyss and https://poe2db.tw/us/Essence, fetched 2026-10-30 |
| `hybrid-one-slot` | documented | A hybrid modifier, one modifier with two stat lines, takes one slot on its side like any other. A community guide adds that the plain tooltip folds its value into a matching single-stat line, so one shown line may be two modifiers summed (not measured here). | https://poe2db.tw/us/Boots_dex, fetched 2026-10-28 (a hybrid is listed as one modifier); https://mobalytics.gg/poe-2/guides/crafting-basics-part-1, fetched 2026-10-30 |
| `fracture-four-mods` | documented | A Fracturing Orb needs a rare item with at least four modifiers and fractures one of them, picked at random. | https://poe2db.tw/us/Stackable_Currency, fetched 2026-10-28 |
| `desecrate-adds-unrevealed` | documented | A bone adds one unrevealed desecrated modifier to a rare; the Well of Souls reveals it as one of three offered modifiers. | https://poe2db.tw/us/Desecrated_Modifiers, fetched 2026-10-29 |
| `desecrate-no-free-slot` | documented | A desecration needs no free slot: on a full item it first removes a random modifier. | https://poe2db.tw/us/Desecrated_Modifiers, fetched 2026-10-29 |
| `side-necromancy` | documented | Omen of Sinistral (Dextral) Necromancy makes the next desecration add only a prefix (suffix). | https://poe2db.tw/us/Omen, fetched 2026-10-29 |
| `abyssal-echoes` | documented | Omen of Abyssal Echoes rerolls the offered modifiers once at the reveal. | https://poe2db.tw/us/Omen, fetched 2026-10-29 |
| `reveal-offers-base` | documented | A reveal may offer ordinary (non-desecrated) modifiers unless an omen guarantees a named one. | https://poe2db.tw/us/Desecrated_Modifiers, fetched 2026-10-29 |
| `armour-no-desecrated-prefix` | documented | Body armours, gloves, boots and helmets have no desecrated prefixes. | https://poe2db.tw/us/Desecrated_Modifiers, fetched 2026-10-29 |
| `homogenising-standard-only` | documented | The Homogenising Exaltation and Coronation omens stopped dropping in 0.4.0 and since 0.5 are on the Currency Exchange only in Standard. | https://www.pathofexile.com/forum/view-thread/3932540, fetched 2026-10-29 |
| `legacy-omens` | documented | Omen of Greater Annulment and the Sinistral and Dextral Coronation and Alchemy omens have been unobtainable since 0.3.0. | https://pathofexile2.wiki.fextralife.com/Patch_Notes_040, fetched 2026-10-29 |
| `greater-with-side-omen` | not established | Omen of Greater Exaltation together with a Sinistral or Dextral Exaltation omen on one Exalted Orb adds two modifiers, both on that side (maxroll's crafting guide, the domistae codex and a Mobalytics omen guide say so; GGG publishes no stacking rule). | https://maxroll.gg/poe2/resources/how-to-craft-in-path-of-exile-2 and https://domistae.github.io/poe2-leveling/poe2_crafting_codex.html, fetched 2026-10-29 |
| `greater-needs-two-free` | not established | Omen of Greater Exaltation needs two free slots, on the restricted side when a side omen is active too (maxroll's crafting guide states the first half; no source states what happens with one). | https://maxroll.gg/poe2/resources/how-to-craft-in-path-of-exile-2, fetched 2026-10-29 |
| `open-side-landing` | not established | An Exalted Orb or Orb of Augmentation with no side omen, on an item whose one side is full, adds to the other side (inferred from the slot limits; maxroll's crafting guide says a Sinistral exalt cannot add to three prefixes). | https://maxroll.gg/poe2/resources/how-to-craft-in-path-of-exile-2, fetched 2026-10-29 |
| `whittling-mod-level` | not established | The modifier Whittling takes is the one with the lowest modifier level (its required item level), not the lowest tier or value; ties between equal levels are unresolved (the domistae codex and maxroll's crafting guide). | https://domistae.github.io/poe2-leveling/poe2_crafting_codex.html, fetched 2026-10-29 |
| `fractured-not-removed` | not established | A fractured modifier is never taken by an Orb of Annulment or a Chaos Orb, Whittling included (the domistae codex; maxroll's overview says only that it cannot be changed). | https://domistae.github.io/poe2-leveling/poe2_crafting_codex.html and https://maxroll.gg/poe2/resources/path-of-exile-2-crafting-overview, fetched 2026-10-29 |
| `omens-active-before-use` | not established | A Necromancy omen must be active before the bone is used and Abyssal Echoes before the reveal (maxroll's Abyss and crafting guides). | https://maxroll.gg/poe2/resources/abyss, fetched 2026-10-29 |
| `necromancy-without-pool` | not established | What Sinistral Necromancy does on a class with no desecrated prefixes (body armour, gloves, boots, helmets): presumably a reveal of ordinary prefixes, but no source tests it. | https://poe2db.tw/us/Desecrated_Modifiers, fetched 2026-10-29 |
| `removal-equally-likely` | not established | Each modifier an Orb of Annulment may take is equally likely to go (poe2db says only "random"). | https://poe2db.tw/us/Orb_of_Annulment, fetched 2026-10-29 |
| `abyss-mark-unfracturable` | not established | The Mark of the Abyssal Lord, the modifier Essence of the Abyss adds, is never the one a Fracturing Orb picks, so on a four-modifier rare holding it the orb picks among the other three: one chance in three for a given wanted one instead of one in four (Mobalytics' SSF and Lightning Arrow crafting guides; its essences guide also says the mark seems to raise the tier of a later reveal, which nothing here relies on). | https://mobalytics.gg/poe-2/guides/ssf-crafting, https://mobalytics.gg/poe-2/guides/lightning-arrow-crafting and https://mobalytics.gg/poe-2/guides/essences, fetched 2026-10-30 |
| `desecrated-unfracturable` | not established | A Fracturing Orb never picks a desecrated modifier (Mobalytics' SSF crafting guide); that an unrevealed one is skipped too is our reading. | https://mobalytics.gg/poe-2/guides/ssf-crafting, fetched 2026-10-30 |
| `full-desecration-same-side` | not established | On an item whose side is full, a desecration with that side's Necromancy omen removes its random modifier from that side (inferred from `desecrate-no-free-slot` and `side-necromancy`; maxroll's rescue of a full rare relies on it). | https://poe2db.tw/us/Desecrated_Modifiers and https://poe2db.tw/us/Omen, fetched 2026-10-29; https://maxroll.gg/poe2/resources/how-to-craft-in-path-of-exile-2, reviewed 2026-10-30 (issue #35) |
| `essence-full-side-removal` | not established | With no Crystallisation omen, when the side a Perfect or corrupted essence's modifier belongs to is full, the modifier the essence removes comes from that side (Mobalytics' essences guide). | https://mobalytics.gg/poe-2/guides/essences, fetched 2026-10-30 |

### Which blueprint relies on which not-established fact

| Fact | Relied on by | Where |
|---|---|---|
| `greater-with-side-omen` | `close-a-side`, `essence-desecrate-double-slam`, `magic-augment-essence`, `rare-clean-then-slam`, `weapons-physical-generic` | The side slam (and `close-a-side`'s close) resolves `greater-omen` beside `side-omen`; `weapons-physical-generic`'s two suffixes come from Greater with Dextral Exaltation. |
| `greater-needs-two-free` | `close-a-side`, `essence-desecrate-double-slam`, `magic-augment-essence`, `rare-clean-then-slam`, `weapons-physical-generic`, `body-armours-single-prefix` | `greater-omen` resolves only where two slots are free; the two listed blueprints name Omen of Greater Exaltation where two are. |
| `open-side-landing` | `close-a-side`, `magic-augment-essence`, `body-armours-single-prefix`, `weapons-physical-generic` | `close-a-side`'s slam on the one open side has no side omen; `magic-augment-essence`'s augment lands on the magic item's free side; `body-armours-single-prefix`'s Greater exalt with no side omen adds its two prefixes because the suffixes are full; `weapons-physical-generic`'s augment lands on the magic weapon's free side. |
| `whittling-mod-level` | `close-a-side`, `essence-desecrate-double-slam`, `magic-augment-essence`, `rare-clean-then-slam` | The removal step's whittle alternative is certain only by this reading of "lowest level". |
| `fractured-not-removed` | `close-a-side`, `essence-desecrate-double-slam`, `magic-augment-essence`, `rare-clean-then-slam`, `fracture-first` | A removal's odds and the blocked-side test leave fractured modifiers out; `fracture-first` annuls and Chaos-rolls around its fractured target. |
| `omens-active-before-use` | `close-a-side`, `essence-desecrate-double-slam`, `magic-augment-essence`, `rare-clean-then-slam`, `fracture-first`, `weapons-physical-generic`, `body-armours-single-prefix`, `full-rare-desecration-rescue` | The desecrations' Necromancy omen and the reveals' Abyssal Echoes; the last-target desecration (§ "The last-target desecration") brings them to the three blueprints that had none. |
| `necromancy-without-pool` | `essence-desecrate-double-slam`, `full-rare-desecration-rescue` | Desecrating for an ordinary prefix target on an armour class. |
| `removal-equally-likely` | `full-rare-desecration-rescue` | The odds shown for an uncertain removal assume it, and so do the rescue's odds of keeping its targets on a full side. |
| `abyss-mark-unfracturable` | `fracture-first` | The Fracturing Orb's odds after Essence of the Abyss leave the mark out. |
| `desecrated-unfracturable` | `fracture-first` | The Fracturing Orb's odds leave a desecrated modifier out. |
| `full-desecration-same-side` | `full-rare-desecration-rescue` | The rescue desecrates a full side; its random removal is taken from that side. |
| `essence-full-side-removal` | none | Only an essence swap's removal side with no Crystallisation omen assumes it (§ "What each crafting item does to the slots"); no shipped blueprint takes such a swap. |

## Slot accounting

Every rule below reads these numbers, computed per side s (prefix, suffix)
on the item as it stands. The model and the engine compute the same ones.

- **cap**: the side's limit for the item's rarity: Normal 0, Magic 1, Rare
  3, a Rare jewel 2 (`ch-data`'s affix limits; `magic-slots`,
  `rare-slots`).
- **mods**: the explicit modifiers on s, fractured, desecrated and
  unrevealed ones included.
- **free** = cap - mods.
- **need**: the targets of side s that are not on the item at their floor,
  from both pools (base and desecrated), less one while an unrevealed
  modifier on s waits for the target its desecration was bound to. On a
  Magic item need is at most cap minus the targets already on s (the rest
  wait for the item to turn rare).
- **spare** = free - need.

A side is:

- **blocked** when spare < 0 and s holds a removable non-target (below);
- **reserved** when need > 0, spare = 0 and free > 0;
- **closable** when free > 0, need = 0 and the other side has need > 0;
- **open** when free > 0.

The words overlap: a reserved or closable side is also open.

**Removable** (what a removal step may aim at): a modifier that is not a
target, not fractured (`fractured-not-removed`), not unrevealed, and not
desecrated. A desecrated non-target is aimed at only by Omen of Light with
an Orb of Annulment, which takes nothing else (`light-removes-desecrated`).
A projected filler (§ "Projection") is a removable non-target.

**At risk** (what a removal may actually take, for its odds and for "could
take a target"):

| Removal | May take |
|---|---|
| Orb of Annulment | Every modifier on the item that is not fractured, desecrated and unrevealed ones included (whether an unrevealed one can go is not established, so it is counted). |
| with Omen of Sinistral or Dextral Annulment | The same, on that side only. |
| with Omen of Light | The desecrated modifiers. |
| Chaos Orb with Omen of Whittling | The one modifier § "Route rules" finds certain, or else it is not taken. |

**Not fitting**: a state-ordered route is rejected (`targets-exceed-slots`,
naming the side) when a side's targets alone are more than its Rare cap.

## What each crafting item does to the slots

Ids are `ch-blueprint`'s (`docs/crafting-helper/blueprints.md`). "Lands"
is the side the added modifier takes; "free there" is checked with the caps
of the rarity the item has after the currency.

| Item | Needs | Does to the slots | Facts |
|---|---|---|---|
| `orb-of-transmutation`, Greater, Perfect | A normal item. | The item turns magic with one modifier, on either side. | `transmute-adds-one`, `currency-min-level` |
| `orb-of-augmentation`, Greater, Perfect | A magic item with a free slot. | One modifier on a free side; with one side full it lands on the other. | `augment-adds-one`, `open-side-landing` |
| `exalted-orb`, Greater, Perfect | A rare item with a free slot. | One modifier on an open side; with one side full it lands on the other. | `exalt-adds-one`, `rare-slots`, `open-side-landing` |
| + `omen-of-sinistral-exaltation` / `omen-of-dextral-exaltation` | A free slot on that side. | The modifier lands on that side. | `side-exaltation` |
| + `omen-of-greater-exaltation` | Two free slots (on the restricted side, with a side omen). | Two modifiers instead of one. | `greater-exaltation-adds-two`, `greater-needs-two-free`, `greater-with-side-omen` |
| Lesser, plain or Greater essence | A magic item with no crafted modifier. | The item turns rare with its modifiers kept, plus the essence's modifier on that modifier's side. | `essence-magic-to-rare`, `one-crafted-one-desecrated` |
| `desecrate` (a bone that fits the class) | A rare item with no desecrated modifier. The routes also ask a free slot on the resolved side, so the game never removes a random modifier for it; a `last-remaining` desecration also asks the class's desecration pool on that side (§ "The last-target desecration"). | One unrevealed desecrated modifier, on the side its omen names. | `desecrate-adds-unrevealed`, `desecrate-no-free-slot`, `one-crafted-one-desecrated` |
| + `omen-of-sinistral-necromancy` / `omen-of-dextral-necromancy` | Active before the bone. | The unrevealed modifier is a prefix (suffix). | `side-necromancy`, `omens-active-before-use`, `necromancy-without-pool` |
| `reveal` | An unrevealed modifier on the item. | It becomes the chosen one of the offered modifiers, in place: no slot changes. | `desecrate-adds-unrevealed`, `reveal-offers-base` |
| + `omen-of-abyssal-echoes` | Active before the reveal. | One reroll of the offer; no slot changes. | `abyssal-echoes`, `omens-active-before-use` |
| `orb-of-annulment` | A magic or rare item with a modifier at risk. | One modifier at risk goes; its slot frees. | `annulment-removes-one`, `fractured-not-removed`, `removal-equally-likely` |
| + `omen-of-sinistral-annulment` / `omen-of-dextral-annulment` | A modifier at risk on that side. | The removal is confined to that side. | `side-annulment` |
| + `omen-of-light` | A desecrated modifier. | The desecrated modifier goes. | `light-removes-desecrated` |
| `chaos-orb` + `omen-of-whittling` | A rare item. | The lowest-level modifier goes, then one random modifier is added, on a side not established. | `whittling-lowest`, `whittling-mod-level` |
| `fracturing-orb` (listed blueprints only) | A rare item with at least four modifiers. | No slot changes: one modifier, picked at random among the explicit ones that are not fractured, not desecrated (revealed or not) and not the Mark of the Abyssal Lord, becomes fractured. | `fracture-four-mods`, `abyss-mark-unfracturable`, `desecrated-unfracturable`, `fractured-not-removed` |
| Perfect and special essences (Hysteria, Delirium, Horror, Insanity, the Abyss, the Breach): the essence swap (engine-only in state order) | A rare item, a modifier the removal may take, and room on the added modifier's side once the removal is made. | Like a Chaos Orb whose added modifier is chosen: one modifier goes, then the essence's modifier is added on its side. The removal is taken from the Crystallisation omen's side; with no omen, from the side the essence's modifier belongs to when that side is full; otherwise from either side. It may take any modifier that is not fractured, the Abyss mark and desecrated ones included (they are exclusions of a Fracturing Orb only). Certain when every modifier it may take is a non-target. | `essence-magic-to-rare`, `one-crafted-one-desecrated`, `side-crystallisation`, `essence-full-side-removal`, `fractured-not-removed`, `removal-equally-likely` |

No v2 route uses a Homogenising omen (`homogenising-standard-only`),
Greater Annulment (`legacy-omens`), the Erasure omens, a Perfect or
corrupted essence, the Crystallisation omens, an alloy or a Fracturing Orb.
The two rows above are for the listed blueprints that do (`fracture-first`)
and for the engine's state-ordered essence swap (§ "Certain removals
first"); the model has neither. An alloy's modifier counts as the item's one
crafted modifier (`alloy-counts-as-crafted`), and a hybrid modifier takes
one slot (`hybrid-one-slot`), so the accounting above already counts it as
one modifier.

## Route rules

These rules apply to a blueprint with `order = "state"`. The guide (which
step now) and the projection (the preview) use the same ones.

### Candidates and the next step

A step is a **candidate** when, on the item as it stands, all of these
hold:

1. its `guard`, when it has one, holds (the conditions below);
2. its `when` resolves (`when = "bound"`: its binding is made);
3. its goal is not met:
   - `add-wanted`: a target it can place is missing;
   - `fill`: always (its side rule decides);
   - `desecrate`: its bound target is missing and the item has no
     desecrated modifier;
   - `reveal`: an unrevealed modifier is on the item;
   - `remove-unwanted`: a side is blocked;
   - `remove-desecrated`: a desecrated modifier is on the item;
4. its side rule finds a side (below; a step without `side` needs none);
5. it can be taken: an alternative fits (below), and for an add, a fill or
   an essence a free slot exists where the action lands; for a desecration
   a free slot exists on its resolved side, and for a `last-remaining`
   desecration the pool condition holds (§ "The last-target desecration").

The **next step** is the first candidate in file order, unless § "Certain
removals first" moves a later one ahead of it, and leaving out a
`remove-desecrated` step that waits (§ "A wrong reveal waits for the
same-side exalt"). With none, the
craft is **done** when no target is missing, and otherwise **stuck**, and
the guide names each step's reason (guard false, goal met, no side, no free
slot, every alternative blocked, no alternative fits).

A target "the step can place": a missing target whose side the action can
land on, from the base pool for an add (a desecrated-pool target is placed
only by a desecration and its reveal), and not the target the route's
`essence` binding claims (an `any-remaining` add leaves that one out).
Bindings (`essence`, `hardest`) are made once and kept across items, as in
listed order; `last-remaining` is not (below).

When a binding is made (a gap the model met; added 2026-10-29): before each
pick, each binding the blueprint uses that is not yet made is made as soon
as the item allows it: `essence` on a magic item, to the first missing
target in target order that an unblocked essence adds; `hardest` while a
target is missing. A binding whose target is later placed stays made, and
the step it binds then has its goal met.

`last-remaining` in state order (added 2026-10-30) is made afresh before
every pick, on the item as it stands: to the one missing target when
exactly one target is missing, and otherwise not made. It is never kept
from an earlier item: kept like `hardest`, a miss that leaves a second
target missing would keep a desecration bound to a target that is no longer
the last. An unrevealed modifier a `last-remaining` desecration left waits
for that target (§ "Slot accounting", need).

`hardest` in the model: a missing desecrated-pool target first (its odds
are unknown, so it counts as hardest), the first in target order; else the
case's `hardest` alias when the case gives one; else the first missing
target in target order. The engine computes it from odds as in listed
order; a case whose route depends on it names it.

### Conditions

The four new condition keys, each a bool, true when:

| Key | Holds when |
|---|---|
| `blocked_side` | some side is blocked |
| `closable_side` | some side is closable |
| `one_side_open` | exactly one side is open |
| `desecrated_unwanted` | a revealed desecrated modifier on the item is not a target |

`false` asks for the opposite. The existing keys (`rarity`,
`wanted_at_least`, `essence_binds` and the rest) keep their meaning.

### Side rules

| `side` | Resolves to |
|---|---|
| `prefix`, `suffix` | That side. |
| `target` | The bound target's side; unbound, the open side with the most missing targets the step can place there, ties to the side of the first such target in target order. None when no open side has one. |
| `close` | The closable side. |
| `blocked` | The blocked side; with two, the larger deficit (need - free) first, ties to the side of the first missing target in target order. |

`side-omen` resolves to the currency's Sinistral omen on the prefix side
and its Dextral omen on the suffix side.

### The unrestricted exalt

While two sides are open and one of them is reserved, an Exalted Orb of any
grade without a side omen does not fit: the alternative is passed over as
not fitting, and the step falls to its next alternative. An unrestricted
exalt could take the slot the reserved side's target needs.

### `greater-omen`

Allowed on an Exalted Orb of any grade only. It resolves to Omen of Greater
Exaltation when all of these hold, and otherwise to nothing (no omen, no
fallback line):

- the step's resolved side has two free slots (with no resolved side, the
  item has two free slots);
- the step can use two: an add has two missing targets there it can place;
  a fill always can;
- `omen-of-greater-exaltation` is not blocked.

### Removals

A `remove-unwanted` step resolves its side with `blocked` and tries its
alternatives in file order:

- **Whittle** (Chaos Orb with Omen of Whittling) fits only when it is
  certain to take a removable non-target on the resolved side **and** the
  first annulment alternative that would otherwise fit could take a target.
  Certain means: among every modifier on the item, the lowest level is held
  by exactly one, that one is a removable non-target on the resolved side,
  no fractured modifier is at or below its level (whether Whittling skips a
  fractured one is not established), and the item holds no unrevealed
  modifier and no projected filler (their levels are not known). A target
  the projection placed has an unknown level too (added 2026-10-29), so a
  whittle is never certain after a projected add.
- **Annulment with `side-omen`** fits only when the other side holds a
  target an Orb of Annulment could take (a non-fractured target); a side
  omen that protects nothing is not worth naming.
- **Plain annulment** fits on a magic or rare item with a modifier at risk.

A Magic item's removal has no whittle (a Chaos Orb needs a rare).

A step with `side = "blocked"` is a **hit** when a non-target went from a
side that was blocked before it; other outcomes classify as in listed order
(hit, miss, unexpected).

The removal's odds: certain when every modifier it may take is a
non-target; otherwise the share of non-targets among them
(`removal-equally-likely`).

### Certain removals first

Added 2026-10-30 (issue #35). Our rule, from the guides' isolation
principle (Mobalytics' isolation and fracturing guides): arrange a random
step so that its exclusions leave one outcome, and leave the gamble for
last. The guide (which step now) and the projection pick by it alike.

The next step is the first candidate in file order, unless that candidate
is not a certain removal and a later candidate is a `remove-unwanted` step
whose chosen alternative is certain. Then the first such later candidate is
the next step.

- A **certain removal** is a candidate whose goal is `remove-unwanted` or
  `remove-desecrated` and whose chosen alternative (the first that fits,
  § "Removals") is certain: every modifier it may take (§ "Slot
  accounting", at risk) is a non-target, or it is a whittle, which fits
  only when certain.
- Only removals count. The model has no pools, so an add's certainty is
  shown in its odds and never reorders.
- The essence swap (engine-only; § "What each crafting item does to the
  slots"): an `add-wanted` step whose chosen alternative is a Perfect or
  special essence counts as a certain removal here when its removal is
  certain, and otherwise keeps its file-order place. It fits in state order
  on a rare when the step's `essence` binding names a missing target the
  essence adds at its floor (the binding is made on a rare as on a magic
  item), the removal's side holds a removable modifier, and the target's
  side has room once the removal is made. Its projection takes the
  non-target the removal would take (§ "Projection", a removal's order)
  and places the target. The model has no Perfect essence, Crystallisation
  omen or crafted-modifier rule, and no state-ordered shipped blueprint
  takes a swap, so no case covers it.
- Nothing else moves: the other candidates keep file order, and the rule
  is applied afresh before every pick.

### The last-target desecration

Added 2026-10-30 (issue #32). When the item is rare, carries no desecrated
modifier (an unrevealed one included) and exactly one target is missing,
the route places that last target with a desecration on its side and a
reveal, instead of an exalt or an essence. It is three blueprint steps,
bound to `last-remaining`, not a hidden rule, so the guide, the outcomes
and the step ids work as for every other step:

| Step | Goal | Bind, `when` | Side | Guard | Alternatives, in order |
|---|---|---|---|---|---|
| `reveal-last` | `reveal` | `last-remaining`, `bound` | none | none | `reveal` with `omen-of-abyssal-echoes`; plain `reveal` |
| `light-last` | `remove-desecrated` | `last-remaining`, `bound` | none | `desecrated_unwanted = true` | `orb-of-annulment` with `omen-of-light` |
| `desecrate-last` | `desecrate` | `last-remaining`, `bound` | `target` | `rarity = ["rare"]` | `desecrate` with `side-omen` |

Where each blueprint has them:

- `close-a-side`: `reveal-last`, `light-last`, `remove`, `desecrate-last`,
  `close`, `slam-open-side`, `side-slam`.
- `rare-clean-then-slam`: `reveal-last`, `light-last`, `remove`,
  `desecrate-last`, `side-slam`.
- `magic-augment-essence`, in its rare phase: `transmute`, `remove-magic`,
  `augment`, `essence`, `reveal-last`, `light-last`, `remove`,
  `desecrate-last`, `side-slam`.
- `essence-desecrate-double-slam`: `desecrate-last` only, right after its
  `desecrate`: `essence`, `reveal`, `light`, `remove`, `desecrate`,
  `desecrate-last`, `side-slam`. Its `reveal` and `light` already take an
  unrevealed modifier and an unwanted desecrated one, whichever binding left
  it. Its own `hardest` desecration is unchanged and still comes first; once
  it has desecrated, the item has a desecrated modifier and the rule cannot
  apply (`one-crafted-one-desecrated`).

`desecrate-last` has one alternative. A bone without the side omen could
land on the other side, so with the Necromancy omen (or the bone) on the
player's blocked list the step is not a candidate, and the route takes the
step it took before the rule.

**The pool condition.** A `last-remaining` desecration is a candidate only
when the item's class has at least one desecrated-section modifier on the
target's side at the item's level (the list the craft window's Desecrated
group shows). Any target on that side then qualifies, a base-pool one
included (`reveal-offers-base`), so the rule never relies on
`necromancy-without-pool`. The engine reads the pool from the shipped class
data; the model takes it from the case (§ "Representative crafts"). The
`hardest` desecration of `essence-desecrate-double-slam` does not ask it.

**A wrong reveal.** A reveal that offers no target leaves a revealed
desecrated modifier that is not a target, with the last target still
missing: `light-last` takes it with Omen of Light and an Orb of Annulment
(certain: it takes nothing else), then `desecrate-last` and `reveal-last`
run again. When an exalt on the same side still has a free slot and a
target to add there, Omen of Light waits for it first (§ "A wrong reveal
waits for the same-side exalt").

**The fallback reason** (engine only). When the item is rare, carries no
desecrated modifier and exactly one target is missing, but `desecrate-last`
is not a candidate, the engine attaches why, in words, to the step it picks
instead: no pool (`no desecrated <prefix|suffix> modifier on <class> at
item level <n>`), the bone or the Necromancy omen blocked (`<names>
blocked`), or otherwise the engine's own reason. The model does not compute
it and the parity test does not compare it, as with odds.

**Odds.** A desecration and a reveal have no established weights, so their
odds are unknown, with that reason, as in listed order.

### A wrong reveal waits for the same-side exalt

Added 2026-10-30 (issue #37). A route rule of ours, not a game fact: the
guide (which step now) and the projection pick by it alike.

A `remove-desecrated` step whose guard asks `desecrated_unwanted = true`
(`light`, `light-last`) **waits** while the item holds a revealed
desecrated modifier that is not a target and some `add-wanted` step of the
blueprint is a candidate on that modifier's side with a free slot there: a
candidate whose resolved side is that side, or, for a step without
`side`, one that can place a missing target on that side. A step that
waits is not taken; the next step is picked from the other candidates as
above, and the guide names its reason (it waits for the add). Once that
side is full, or no add-wanted step can place a target there, the step is
taken as before.

Why: the modifier a wrong reveal leaves stays on the item through the
exalt that fills the rest of its side, and the engine's add odds drop from
the pool every family of every revealed modifier on the item, desecrated
ones included (the reading the blocker blueprints, `gloves-crit-melee` and
`amulets-resist-spirit`, rely on). Keeping it never makes the exalt worse
and can make it better; taking it off first throws that away, and Omen of
Light still comes once the side is done. It applies to every wrong reveal
in a state-ordered route, not only one the player chose on purpose
(the owner's default, reversible). The existing `rare-wrong-reveal` case
(its side full) keeps Light first.

**The blocker advice** (engine only). Because the reveal's three offers
are never read (`desecrate-adds-unrevealed`), the engine names, for a
reveal step whose desecration is followed later in the projection by an
add on the same side with an Exalted Orb of any grade, which desecrated
modifiers to take if no target is offered: the class variant's
desecrated-section modifiers on that side at the item level that are not
targets, each scored by that exalt's chance of a target with the
candidate on the item (its families out of the pool), against the same
exalt with the slot taken by a modifier that excludes nothing. Only those
that raise the chance are named, at most three, best first. Ordinary
modifiers a reveal may offer (`reveal-offers-base`) are left out: which
ones is not known. The model has no pools and does not compute it.

### Rejections

Checked in this order; the first that applies is the route's rejection, as
the engine serialises it (`kind`, kebab case):

1. `class-not-covered`: the blueprint does not name the item's class.
2. `no-start-fits`: no `[[start]]` alternative holds.
3. `targets-exceed-slots` (new): a side's targets alone exceed its Rare cap.
4. `desecrated-target-unplaced`: a desecrated-pool target is missing and
   the blueprint has no `desecrate` step. Since every v2 blueprint has
   `desecrate-last` (added 2026-10-30), none of them is rejected this way;
   a desecrated-pool target that no `hardest` desecration binds and that
   never becomes the last missing one ends in `route-cannot-finish`.
5. `route-cannot-finish` (new), with `why`: `no-candidate` (no step is a
   candidate while a target is missing; the reasons name each step's, so a
   step whose every alternative is on the player's blocked list shows up
   here, since it is not a candidate), `no-change` (a step's hit leaves the
   item as it was) or `step-cap` (the cap below is reached), and the step
   concerned when there is one.
6. `weak-magic-start` (new, 2026-10-30; engine-only, both orders: the
   model has no tiers): checked last, after a route is planned. The route
   is rejected when all of these hold: the item is magic; its prefix is a
   target, at any tier; the route has a step, taken while the item is
   magic, whose chosen action makes it rare (a Regal Orb of any grade, or a
   Lesser, plain or Greater essence); and the prefix's tier is worse than
   the second-best tier its modifier can roll on this base variant at the
   item's level. With one reachable tier, the top two is that one tier. It
   does not apply when the item has no item level or the tier is unknown.
   The rejection names the modifier, its tier, the item level and the top
   two tiers. Why: a magic start whose wanted prefix is weak is worth a new
   base, not an upgrade (Mobalytics' campaign martial weapon guide; Boots'
   movement speed reaches T1 at item level 82 and T2 at 65 in the shipped
   data, so a magic level-82 boot with T3 movement speed as a target is
   rejected and one with T2 is not).

`every-alternative-blocked`, `no-alternative-fits` and `cannot-bind` stay
listed order's; in state order a step they would describe is simply not a
candidate.

### Listed order (the baseline only)

A blueprint without `order` keeps listed order and is not planned by these
rules. The model reads one only for the baselines: its starts, as written,
and the first fitting action of its first step that applies, as written
(the listed `fill-desecrate-exalt` names a plain `exalted-orb` with no
omen).

## Projection

The route preview of a state-ordered blueprint lists the steps the craft
takes if each one hits, starting from the item as it stands:

1. Stop when no target is missing (done; checked before the pick, added
   2026-10-29). Otherwise pick the next step by the rules above; with none,
   the route is rejected `route-cannot-finish`, `no-candidate`.
2. Record it: its goal, its resolved side (or none), its currency as
   written, and its omens resolved, in the order the action lists them
   (the v2 blueprints write `greater-omen` before `side-omen`).
3. Apply its hit to a copy of the item:
   - an add places the first missing target it can place on its resolved
     side, two with Greater, in target order (a transmute first makes the
     item magic);
   - a fill adds one filler (a non-target of unknown level) to its side,
     two with Greater;
   - an essence makes the item rare and places its bound target;
   - a desecration leaves an unrevealed modifier on its side, waiting for
     its bound target;
   - a reveal turns that modifier into the bound target (desecrated,
     revealed);
   - a removal takes the non-target its action would take: the whittle's
     certain one, otherwise the first removable non-target on the resolved
     side in the item's order (fillers last); Omen of Light takes the
     desecrated non-target. A whittle's added modifier is not projected.
4. Repeat from 1.

The projection assumes every step hits and nothing else changes: no
modifier lands where the step did not aim, no omen is wasted, and every
fact the blueprint relies on holds. A step may appear more than once (a
fill without Greater fills one slot at a time).

**Step cap: 16.** A rare holds six explicit modifiers; the longest honest
route (a normal item transmuted, a magic removal, an augment, an essence,
removals and adds up to the six slots) stays well under it. A projection
that reaches 16 steps without finishing is looping, and the route is
rejected `route-cannot-finish`, `step-cap`. A step whose hit leaves the
projected item unchanged is rejected `no-change`.

**Odds.** The first step's odds are computed as in listed order, on the
item. Each later step's odds are computed the same way on the projected
item (the item as it would stand if every earlier step hit), where a
filler takes its slot and excludes no modifier family; they read as "if the
earlier steps hit". A step whose odds cannot be computed there is unknown,
with the reason, as in listed order. The parity test does not compare odds;
the model gives only a removal's certainty.

## Representative crafts

Each case is an abstract item (rarity; per side the modifiers, each with an
alias, its role, target or junk, and whether it is fractured, desecrated or
unrevealed, and its level), its ordered targets (alias, side, pool, the
essence that adds it or none) and what must hold for each v2 blueprint.
"Junk" is a modifier that is not a target. `cases.json` writes the full
expected step lists, or only the first steps where spawn weights decide,
and each rejection's `kind`. A removal step there also records `certain`,
whether every modifier it may take is a non-target. Every case names its
`hardest` target, so a route that desecrates does not depend on odds the
model does not have; a target's essence is the essence id the engine's
floor picks, which the model takes as given (added 2026-10-29). A case
may name the sides on which its class has a desecrated modifier at the
item's level (`desecration_pool`, a list of `prefix` and `suffix`; absent,
both), for the last-target desecration's pool condition (added
2026-10-30). A difference between this list and the model
is a spec or model defect, never a reason to change the expectation.

- **`rare-2p1s-prefix-missing`** (the owner's craft of 2026-10-29): a rare
  with two target prefixes and one target suffix; a third prefix target is
  missing. The prefix side is reserved, the suffix side closable.
  - `close-a-side`: the last target, so desecrate on the prefix side with
    Omen of Sinistral Necromancy; then reveal with Omen of Abyssal Echoes.
    Done (issue #32's first test; it used to close the suffix side and
    slam).
  - `essence-desecrate-double-slam`: desecrate on the prefix side with Omen
    of Sinistral Necromancy; then reveal with Omen of Abyssal Echoes. Done.
  - `rare-clean-then-slam`: the same desecration and reveal. Done (it used
    to exalt with Sinistral Exaltation).
  - `magic-augment-essence`: rejected, `no-start-fits`.
  - Baseline: the listed `fill-desecrate-exalt` names a plain `exalted-orb`
    with no omen first, and an unrestricted exalt can land in the reserved
    prefix slot.
- **`rare-2p1s-necromancy-blocked`**: the owner's craft with
  `omen-of-sinistral-necromancy` on the blocked list. The last-target
  desecration is not a candidate, so each route is as it was before the
  rule.
  - `close-a-side`: fill the suffix side with an Exalted Orb, Omen of
    Greater Exaltation and Omen of Dextral Exaltation; then add the prefix
    with a Perfect Exalted Orb and no omen (one side open). Done.
  - `essence-desecrate-double-slam`: its `hardest` desecration falls to the
    plain bone, on the prefix side with no omen; then reveal with Abyssal
    Echoes. Done.
  - `rare-clean-then-slam`: add the prefix with a Perfect Exalted Orb and
    Omen of Sinistral Exaltation (one free slot, so no Greater). Done.
  - `magic-augment-essence`: rejected, `no-start-fits`.
- **`body-armour-prefix-no-pool`**: the owner's craft on a body armour,
  whose class has no desecrated prefix (`armour-no-desecrated-prefix`;
  `desecration_pool` is suffix only). The last target is a prefix, outside
  the pool.
  - `close-a-side`: fill the suffix side, then add the prefix with a
    Perfect Exalted Orb and no omen, as above. Done.
  - `essence-desecrate-double-slam`: desecrate on the prefix side with
    Sinistral Necromancy, then reveal with Abyssal Echoes (its `hardest`
    desecration does not ask the pool; `necromancy-without-pool`). Done.
  - `rare-clean-then-slam`: add the prefix with a Perfect Exalted Orb and
    Sinistral Exaltation. Done.
  - `magic-augment-essence`: rejected, `no-start-fits`.
- **`rare-desecrated-target`** (issue #32's second test): a rare whose
  prefixes are a revealed desecrated target and a target, and whose one
  suffix is a target; a third prefix target is missing. The item already
  has its desecrated modifier, so no route desecrates.
  - `close-a-side`: fill the suffix side, then add the prefix with a
    Perfect Exalted Orb and no omen. Done.
  - `essence-desecrate-double-slam`, `rare-clean-then-slam`: add the prefix
    with a Perfect Exalted Orb and Sinistral Exaltation. Done.
  - `magic-augment-essence`: rejected, `no-start-fits`.
- **`rare-wrong-reveal`**: a rare whose prefixes are two targets and a
  revealed desecrated junk (a reveal that offered no target), and whose one
  suffix is a target; a third prefix target is missing.
  - `close-a-side`, `essence-desecrate-double-slam`,
    `rare-clean-then-slam`: an Orb of Annulment with Omen of Light, odds
    certain; desecrate on the prefix side with Sinistral Necromancy; reveal
    with Abyssal Echoes. Done.
  - `magic-augment-essence`: rejected, `no-start-fits`.
- **`rare-wrong-reveal-same-side-exalt`** (2026-10-30, issue #37): a rare
  whose prefixes are a target and a revealed desecrated junk (a reveal that
  offered no target), and whose one suffix is a target; a second prefix
  target is missing, and the prefix side has one free slot. Omen of Light
  waits for the prefix exalt (§ "A wrong reveal waits for the same-side
  exalt").
  - `essence-desecrate-double-slam`, `rare-clean-then-slam`: add the prefix
    with a Perfect Exalted Orb and Sinistral Exaltation, the desecrated junk
    kept on the item. Done (Light used to come first).
  - `close-a-side`: the suffix side is closable, so fill it with an Exalted
    Orb, Greater and Dextral Exaltation; then add the prefix with a Perfect
    Exalted Orb and no omen (one side open). Done; Light never comes.
  - `magic-augment-essence`: rejected, `no-start-fits`.
- **`magic-two-good`**: a magic item with a target prefix and a target
  suffix; missing a prefix an essence adds, a desecrated-pool prefix and
  two more suffixes.
  - `essence-desecrate-double-slam`: the essence; desecrate on the prefix
    side with Sinistral Necromancy; reveal with Abyssal Echoes; add the two
    suffixes with a Perfect Exalted Orb, Greater and Dextral Exaltation.
    Done.
  - `magic-augment-essence`: the essence; add the two suffixes with a
    Perfect Exalted Orb, Greater and Dextral Exaltation; the desecrated
    prefix is then the last target, so desecrate on the prefix side with
    Sinistral Necromancy and reveal with Abyssal Echoes. Done (it used to
    be rejected `desecrated-target-unplaced`).
  - `close-a-side`, `rare-clean-then-slam`: rejected, `no-start-fits`.
- **`rare-fractured`**: a rare whose prefixes are a fractured target and
  two junk, whose one suffix is a target and the item's lowest-level
  modifier; a prefix target is missing. The prefix side is blocked.
  - `close-a-side`, `essence-desecrate-double-slam`,
    `rare-clean-then-slam`: each starts with an Orb of Annulment and Omen
    of Sinistral Annulment on the prefix side, odds certain (the fractured
    target cannot go; the whittle would take the suffix target); then
    desecrate on the prefix side with Sinistral Necromancy and reveal with
    Abyssal Echoes. Done.
  - `magic-augment-essence`: rejected, `no-start-fits`.
- **`rare-one-target`** (#28): a rare whose prefixes are a target and a
  junk and whose suffixes are two junk, the lowest-level modifier a junk
  suffix; missing two prefixes and a suffix. The prefix side is blocked,
  the suffix side reserved.
  - `rare-clean-then-slam`: a plain Orb of Annulment on the prefix side
    (the suffix side holds no target to protect); add the two prefixes with
    a Perfect Exalted Orb, Greater and Sinistral Exaltation; the suffix is
    then the last target, so desecrate on the suffix side with Omen of
    Dextral Necromancy and reveal with Abyssal Echoes. Done (it used to
    exalt the suffix with Dextral Exaltation).
  - `close-a-side`: rejected, `no-start-fits` (no side closable, both
    open).
- **`rare-no-target`** (#28's belt): a rare with two junk prefixes and two
  junk suffixes; two prefix targets and two suffix targets.
  - `rare-clean-then-slam`: a plain annulment on the prefix side, then one
    on the suffix side; then the two prefixes with a Perfect Exalted Orb,
    Greater and Sinistral Exaltation, and the two suffixes with a Perfect
    Exalted Orb, Greater and Dextral Exaltation. Done.
- **`magic-one-good`**: a magic item with a target prefix and an open
  suffix; missing a suffix and a prefix an essence adds.
  - `magic-augment-essence`: augment (the suffix), then the essence. Done.
- **`magic-good-and-bad`**: a magic item with a target prefix and a junk
  suffix; missing a suffix and a prefix an essence adds.
  - `magic-augment-essence`: starts with an Orb of Annulment and Omen of
    Dextral Annulment on the suffix side, odds certain.
  - `essence-desecrate-double-slam`: the same certain annulment first
    (§ "Certain removals first": the essence is the first candidate in file
    order, the removal a later certain one; it used to come after the
    essence); then the essence; desecrate on the suffix side with Dextral
    Necromancy; reveal with Abyssal Echoes. Done. With Omen of Dextral
    Annulment blocked the removal is a plain annulment that may take the
    target prefix, and the essence keeps its place.
- **`rare-certain-removal-first`** (2026-10-30): a rare whose prefixes are
  a fractured target and two junk, and whose suffixes are a target and an
  unrevealed desecrated modifier left by an earlier desecration (waiting
  for no known target); a prefix target and a desecrated-pool suffix
  target, the hardest, are missing. The prefix side is blocked.
  - `essence-desecrate-double-slam`: its `reveal` is the first candidate in
    file order, and its removal, an Orb of Annulment with Omen of Sinistral
    Annulment, is certain (the fractured target cannot go), so the removal
    goes first; then the reveal; then the prefix with a Perfect Exalted Orb
    and Sinistral Exaltation. Done.
  - `close-a-side`, `rare-clean-then-slam`: the removal is already the
    first candidate; then the prefix slam; the hardest is then the last
    target, so the waiting modifier is revealed with Abyssal Echoes. Done.
  - `magic-augment-essence`: rejected, `no-start-fits`.
- **`rare-uncertain-removal-keeps-order`**: the control, the same item with
  the prefix target not fractured, so the annulment may take it.
  - `essence-desecrate-double-slam`: the reveal keeps its file-order place;
    then the annulment, odds not certain; then the prefix slam. Done.
  - `close-a-side`, `rare-clean-then-slam`: as above, the annulment not
    certain.
  - `magic-augment-essence`: rejected, `no-start-fits`.

Invariants over every case and every route:

- no Exalted Orb without a side omen while two sides are open and one is
  reserved;
- Omen of Greater Exaltation only where two free slots fit two placeable
  targets (an add) or two fillers (a fill);
- a side annulment only where the other side holds a target an annulment
  could take;
- a whittle only when it is certain and an annulment could take a target;
- a fractured modifier is never projected as removed;
- every omen a step needs is in its list (the side omen with Greater, the
  Necromancy omen with a side desecration), unless the player blocked it
  and the step took its plain alternative (a reveal without Abyssal
  Echoes, `essence-desecrate-double-slam`'s plain bone);
- the last target is desecrated when it can be: on a rare with no
  desecrated modifier and exactly one target missing, whose side has a
  free slot and a desecration pool, with neither the bone nor that side's
  Necromancy omen blocked, the step taken is a desecration on that side
  with its Necromancy omen.

## Not covered

- Spawn weights, costs, the expected number of tries, and odds beyond one
  step: the model has no weights; where they decide, the cases record only
  the first steps.
- Floors and minimum modifier levels: the model treats a target as on the
  item or missing; checking floors and which currency grade meets them is
  the engine's.
- Modifier groups (an essence refused for a group already on the item) and
  the one-crafted-modifier cap beyond the essence step's "magic item". The
  one-crafted rule (`one-crafted-one-desecrated`, `alloy-counts-as-crafted`)
  is held by `ch-blueprint`'s validator, statically, on the blueprints: no
  two crafted steps without a removal between them, no crafted step that
  repeats on a miss.
- Tiers, and so the `weak-magic-start` rejection (engine-only).
- The Fracturing Orb: listed order only (`fracture-first`); no state-ordered
  blueprint fractures.
- The essence swap: the model has no Perfect or special essence and no
  Crystallisation omen, so no case covers the swap or its place under
  § "Certain removals first" (engine-only, tested over the engine's own
  test blueprints).
- What a whittle adds, where a Chaos Orb's replacement lands, and ties
  between equal-level modifiers: a whittle is taken only when certain, and
  its added modifier is not projected.
- Whether an unrevealed modifier can be annulled, and Putrefaction.
- Class pools: which desecrated modifiers a class has at which level (a
  case names only the sides its pool covers, `desecration_pool`; the
  engine reads the shipped class data), what a desecration without a pool
  does (`necromancy-without-pool`) and which essences fit a class.
- Two desecrated-pool targets on one item: the second finds no step, and
  the route is rejected `route-cannot-finish`.
- Jewels (no spawn weights; no v2 blueprint names them), Homogenising and
  Catalysing omens, Greater Annulment, the Erasure and Crystallisation
  omens, Perfect and corrupted essences, alloys, Fracturing Orbs, Regal,
  Alchemy, Vaal and Divine (the two rows of § "What each crafting item
  does to the slots" for the Fracturing Orb and the essence swap are for
  the engine and the listed blueprints, not the model).
- Listed-order blueprints beyond the baseline reading above.
- The two state-ordered blueprints from the guides,
  `weapons-physical-generic` and `body-armours-single-prefix`: they use only
  the rules above, which the cases already pin, so they are not among the
  model's blueprints and `cases.json` records no route for them. The
  engine's walks pin them instead
  (`apps/crafting-helper/crates/ch-engine/tests/seeds_guides.rs`).
