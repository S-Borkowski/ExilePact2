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
  and those modifiers); the route carries no blueprint notes.
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

Three craft states use the owner's 2P/1S belt (`crates/ch-app/tests/items/`,
constructed), with maximum Life T2, maximum Mana T2, Armour T2 (missing)
and Fire Resistance T3 as the targets and the state-ordered `close-a-side`
chosen:

- `craft-side-full`: planning on the belt right after those targets are
  set: Life and Mana on it and Armour chosen fill its three prefixes, so
  every other prefix choice (the desecrated ones included) carries
  `disabled`, `Prefixes full: 2 on the item + 1 chosen = 3 of 3`, the
  row's tooltip; the three chosen prefixes and every suffix choice carry
  `null`. `craft-close-side` has the same choices. The ring's planning
  views (`craft-planning`, `craft-fallback`, `craft-no-route`) choose three
  suffixes, so their other suffix choices carry `Suffixes full: 0 on the
  item + 3 chosen = 3 of 3`; in `craft-prefilled` no side is full and every
  `disabled` is `null`.

- `craft-close-side`: planning on the belt: the route closes the finished
  suffix side first, `Exalted Orb with Omen of Greater Exaltation and Omen
  of Dextral Exaltation on the suffix side, for any modifier` (a fill step's
  instruction, composed from its data), then slams the one open prefix side
  with a Perfect Exalted Orb and no side omen. No step offers an Exalted
  Orb that could land on the prefix side's reserved slot. The route's
  source line carries no web address.
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
