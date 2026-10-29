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

- `craft-planning`: planning, the route previewed with known odds for the
  orbs and `odds unknown: …` for the desecration and the reveal.
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

Every guiding view carries the overlay's content (`overlay.guide`: the
current step, both branch lines and the last outcome), and no other view
does; only the two overlay views below have a screen backend, so only the
first of them shows it:

- `overlay-active`: guiding after one hit (on the whittle step), the
  game's window in front: the overlay is visible at `top-center`.
- `overlay-not-in-front`: the same craft with another window in front: the
  overlay is hidden, its content kept.

Nothing else may be here: `fixtures_are_current` fails on any other JSON
file. The views are made by `crates/ch-app/tests/fixtures.rs` by driving the
app with a recording fake shell (and, for the overlay views, a fake screen
backend whose watch thread is ticked by hand, so they are deterministic),
and the test fails when a file differs
from what the Rust types serialise. The main view's types deny unknown
fields, so a field either side adds, drops or renames fails it too. After
a deliberate change, regenerate them and review the diff:

```sh
CH_FIXTURES=1 cargo test --manifest-path apps/crafting-helper/Cargo.toml -p ch-app --locked --test fixtures
```

The page tests render them and hold each one against the TypeScript types.
