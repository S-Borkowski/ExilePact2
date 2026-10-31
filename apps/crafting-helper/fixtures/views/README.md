# Serialised main window views

One file per state the main window and the overlay render, each a `View`
exactly as `ch-app` serialises it and the shell sends it as `ch:view`:

- `empty`: a first start, nothing grabbed (craft mode none).
- `grabbed-item`: the hotkey grabbed an item's text (invented for the
  tests, not copied from the game). It has no modifier headers, so the
  craft part is refused with that reason.
- `grab-refused`: a later hotkey press found an empty clipboard; the last
  grab is kept and the status says why.
- `grabbed-not-item`: a paste of text that does not look like an item; it
  is shown, with an error status, and the craft part is refused.
- `hotkey-taken`: another program holds Ctrl+Alt+X at the start.

The craft window's states use the whittled ring of `ch-engine`'s tests
(`crates/ch-engine/tests/items/gear/rings-resist-whittle/`, constructed
item texts), with fire, cold and lightning resistance at T3 or better as
the targets and the whittling route chosen:

- `craft-prefilled`: planning right after the grab of the ring's start,
  before any target is changed: the item's own two modifiers (mana and
  strength, each at T5) ticked as targets and met, and the route the
  engine previews for them.
- `craft-planning`: planning, the route previewed with known odds for the
  orbs and `odds unknown: …` for the desecration and the reveal. Each step
  line carries the target modifiers a hit of it may place (`targets`) and
  an instruction composed from its data (the currency, its omens, the side
  and those modifiers); the route carries no blueprint notes. The
  whittle's desecration names Omen of Dextral Necromancy (all three
  targets are suffixes; 0.2.0). The preview's `routes` lists every
  applicable route with its `number` in the engine's order, and the
  whittling route chosen is `route_number` 2, where it stands, not first;
  `other_routes` is the same list without it, numbered the same.
- `craft-fallback`: as planning, with the Omen of Whittling blocked for
  this craft, so the whittle step falls back to a Perfect Exalted Orb and
  says why.
- `craft-no-route`: as planning, with Chaos Orbs and Orbs of Annulment
  blocked for this craft: no route (the state-ordered routes, which clean
  the suffix side with an annulment, included), one rejection per
  blueprint.
- `craft-refused`: the ring, corrupted: the engine's refusal in words.
- `craft-guiding`: the route started and one hit taken: on the whittle
  step, with its branch lines and one history entry.
- `craft-miss`: the first Chaos Orb rolled fire resistance below its
  floor: a miss, the same step on its second try.
- `craft-replan`: one Chaos Orb brought two targets at once: unexpected,
  plan again.
- `craft-done`: the hit path to the end: four hits, done.

Each history summary carries no outcome word (the pages show the result's
own beside it); the status line leads with it (`Miss: no target gained or
lost.`). No status line names the blueprint (`Route chosen.`, `Craft
started.`). The branch lines name the next step by its composed
instruction, never the blueprint's text.

Four craft states use the owner's 2P/1S belt (`crates/ch-app/tests/items/`,
constructed), with maximum Life T2, maximum Mana T2, Armour T2 (missing)
and Fire Resistance T3 as the targets and the state-ordered `close-a-side`
chosen:

- `craft-side-full`: planning on the belt right after those targets are
  set: Life and Mana on it and Armour chosen fill its three prefixes, so
  every other prefix choice (the desecrated ones included) carries
  `disabled`, `Prefixes full: 2 on the item + 1 chosen = 3 of 3`, the
  row's tooltip; the three chosen prefixes and every suffix choice carry
  `null`. `craft-close-side` and `craft-last-desecration` have the same
  choices. The ring's planning
  views (`craft-planning`, `craft-fallback`, `craft-no-route`) choose three
  suffixes, so their other suffix choices carry `Suffixes full: 0 on the
  item + 3 chosen = 3 of 3`; in `craft-prefilled` no side is full and every
  `disabled` is `null`. Its route is the one the engine previews first,
  not `close-a-side`: since 0.2.0 `fractured-essence-swap` plans on the
  belt too (its desecration and reveal; no swap, the belt has no
  fractured modifier) and takes route number 1, so in
  `craft-last-desecration` `close-a-side` is `route_number` 2.

- `craft-close-side`: planning on the belt with Omen of Sinistral
  Necromancy blocked for the craft (its `blocked` list), so the last
  prefix is slammed rather than desecrated: the route closes the finished
  suffix side first, `Exalted Orb with Omen of Greater Exaltation and Omen
  of Dextral Exaltation, for any modifier` (a fill step's instruction,
  composed from its data; since 0.3.0 an exalt or a bone step names no
  side, which its omen or the free slots already decide), then slams the
  one open prefix side with a Perfect Exalted Orb and no side omen,
  `Perfect Exalted Orb for +# to Armour`. No step offers an Exalted
  Orb that could land on the prefix side's reserved slot. Both steps were
  taken in place of the last target's desecration, so each carries a
  Fallback line, `<its currency, with its omens> instead of a desecration:
  Omen of Sinistral Necromancy blocked`. The route's source line carries
  no web address.
- `craft-last-desecration`: the same planning with nothing blocked:
  Armour is the last target missing on a rare with no desecrated
  modifier, so the route desecrates for it (issue #32): `Use Omen of
  Sinistral Necromancy, then apply the Preserved Collarbone; the
  desecrated modifier will be a prefix, for +# to Armour` (the wording
  since 0.2.0), then the reveal with Omen of Abyssal Echoes, both with
  `odds unknown: …` and no Fallback line.
- `craft-miss-clean`: the same craft started, the suffix side closed (a
  hit) and the slam on the prefix side missed (Flask Life Recovery): on
  `remove`, numbered as the craft's third step. Mana, a target, is the
  lowest modifier, so a whittle would take it: the engine takes the Orb of
  Annulment with Omen of Sinistral Annulment (the suffix side holds fire),
  naming the unwanted Flask Life Recovery and saying the orb takes one at
  random, with its odds and no fallback line (the engine passed the
  whittle over by its own rule). Both branch lines say the next step is
  picked from the item (no route's start fits the full belt, and a step
  missed).

Two craft states use the magic Heavy Belt of `ch-engine`'s relaxation
tests (`crates/ch-engine/tests/items/relax/belt-magic-life.txt`,
constructed: maximum Life T2, no suffix), with Fire, Cold and Lightning
Resistance as the targets:

- `craft-relaxation`: all three at T2: no route, and the preview's
  `relaxation` proposes lowering fire to T3, the tier Greater Essence of
  Insulation adds.
- `craft-blocker`: the relaxation accepted (fire at T3) and
  `essence-desecrate-double-slam` chosen: the essence, the suffix
  desecration for cold, its reveal, then a Perfect Exalted Orb with Omen of
  Dextral Exaltation for lightning (`… for +#% to Lightning Resistance`, no
  side named). The reveal's line alone carries `blocker`: if cold is not
  offered, take `+# to Strength and Dexterity` or `+# to Strength and
  Intelligence`, which keep the ordinary `+# to Strength` out of the
  exalt's pool, and that exalt's chance of a target before and after.
  Every other step line of every view carries `blocker: null`.

One craft state uses the essence swap ring
(`crates/ch-app/tests/items/ring-essence-swap.txt`, constructed, copied
from `ch-engine`'s `tests/items/essence-swap/`): a rare Amethyst Ring at
item level 76 whose suffix side is full with a fractured Fire Resistance,
an unrevealed desecrated suffix and Dexterity, with maximum Mana T5,
Mana Regeneration Rate T2 and the desecrated Cold and Chaos Resistances
as the targets (0.2.0):

- `craft-essence-swap`: `fractured-essence-swap` chosen: the essence
  step's instruction names what Essence of Hysteria removes and what
  stays (`…; it removes the unrevealed desecrated suffix or +10(9-12) to
  Dexterity, one at random; +23(21-25)% to Fire Resistance is fractured
  and stays`), and its `survival` the unrevealed suffix's chance of
  surviving the swap (1 in 2), the reveal if it stays, and if it goes the
  annulment with Omen of Dextral Annulment, its chance of sparing the
  essence's modifier and the re-plan if it does not. Then the reveal of
  the desecrated target. Every other step line of every view carries
  `survival: null`.

Every guiding view carries the overlay's content (`overlay.guide`: the
current step, both branch lines, the last outcome and the screen watch's
line), and no other view does; each history entry carries its source
(`grab` or `screen`), and each guiding view the watch line (`watch`), the
same in the overlay's part. The views above have no screen backend, so
their watch line says the screen is not read. The overlay views below have
one; those with the game's window in front show the overlay:

- `overlay-active`: guiding after one hit (on the whittle step), the
  game's window in front: the overlay is visible at `top-center`, the
  watch reading and seeing no tooltip.
- `overlay-not-in-front`: the same craft with another window in front: the
  overlay is hidden, its content kept, the watch paused.

The screen watch's views: guiding from the ring's start with the game in
front, the fake backend's tooltip read set and the watch ticked by hand,
400 ms apart on its clock (the tooltips are built from the ring's texts:
the plain form, an assumption until the `locator` item reads the owner's
screenshots):

- `craft-watching`: reading, no tooltip: `Watching: after the step, hover
  the item in the game.`
- `craft-screen-hit`: the Chaos Orb's hit read twice alike: "Last:" and
  the history entry from the screen, the item now the read one, the next
  step.
- `craft-screen-unexpected`: two targets at once read twice alike: not
  applied, the step and the item kept, the status line and the watch line
  (warning) say to grab the item.
- `craft-watch-off`: `screen_watch` off: `Screen reading is off: …`; grabs
  drive the guide as in M5.
- `craft-watch-unavailable`: no English OCR: `Can't read the screen (no
  English OCR): …` (warning).

Every view carries the trade panel's part (`trade`), closed in all of them
but the `trade-*` ones (a closed part carries the selection and the league
alone). The trade views open the panel over the ring's planning craft,
which stays as `craft-planning` shows it, each through `set_trade_search`
as the page sends it (the whole selection), with the default league
`Forbidden Rites`:

- `trade-empty`: the panel just opened: no class chosen, every class
  offered, no rows, no link; the search disabled beside `Choose an item
  class.`
- `trade-picking`: Rings, Sapphire Ring, item level 75, `+# to maximum
  Life` (prefix) and `+#% to Fire Resistance` (suffix) picked at Any tier,
  Both slots: every row with its weight from the spawn weights, the hybrid
  in the not-searchable list, the count line asking for two, the link built
  and the search enabled.
- `trade-one-side`: the same base with two suffixes picked (fire and cold
  resistance): the count line says the search asks for one and why; the
  link built.
- `trade-default-weight`: a jewel base (Sapphire), whose page has no
  established spawn weights: every weight is the default (1.0) with its
  reason, the first prefix and the first suffix picked.
- `trade-floor`: Boots, Bangled Sandals, the item level left blank,
  `#% increased Movement Speed` picked at least T2: `suggested_item_level`
  is 65 (the tier's item level), the selection's `item_level` stays
  `null`, and the link's item-level filter is 65. Every other view's
  `suggested_item_level` is `null` (no floor picked, or the panel closed).
- `trade-refused`: after `trade-picking`, a `set_trade_search` naming a
  base type Rings does not have (Iron Mitts): the error status line, the
  selection and the link as they were.

Nothing else may be here: `fixtures_are_current` fails on any other JSON
file. The views are made by `crates/ch-app/tests/fixtures.rs` by driving the
app with a recording fake shell (and, for the overlay and screen watch
views, a fake screen backend whose watch thread is ticked by hand on a clock
the test moves, so they are deterministic),
and the test fails when a file differs
from what the Rust types serialise. The main view's types deny unknown
fields, so a field either side adds, drops or renames fails it too. After
a deliberate change, regenerate them and review the diff:

```sh
CH_FIXTURES=1 cargo test --manifest-path apps/crafting-helper/Cargo.toml -p ch-app --locked --test fixtures
```

The page tests render them and hold each one against the TypeScript types.
