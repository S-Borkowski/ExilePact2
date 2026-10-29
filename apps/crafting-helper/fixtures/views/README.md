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
- `craft-no-route`: as planning, with Chaos Orbs blocked for this craft:
  no route, one rejection per blueprint.
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

One craft state uses the ring of `ch-engine`'s fill walk
(`crates/ch-engine/tests/items/gear/fill-desecrate-exalt/`), with life T3,
fire T4, cold T4 and chaos T3 as the targets and the fill, desecrate, slam
route chosen:

- `craft-miss-clean`: planned on `03-fill-fire` and started, the exalt
  missed (`07-fill-miss` rolled accuracy): on `fill-clean`, which the app
  names by the modifier levels. Life, a target, is the lowest, so a
  whittle would take it: the step is the Orb of Annulment, naming the
  unwanted mana and accuracy and saying the orb takes one at random, with
  the engine's odds for it and no fallback line. A miss branch to it says
  the new modifier is removed next, without offering the choice.

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
