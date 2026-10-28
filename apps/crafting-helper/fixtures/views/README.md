# Serialised main window views

One file per state the main window and the overlay render, each a `View`
exactly as `ch-app` serialises it and the shell sends it as `ch:view`:

- `empty`: a first start, nothing grabbed.
- `grabbed-item`: the hotkey grabbed an item's text (invented for the
  tests, not copied from the game).
- `grab-refused`: a later hotkey press found an empty clipboard; the last
  grab is kept and the status says why.
- `grabbed-not-item`: a paste of text that does not look like an item; it
  is shown, with an error status.
- `hotkey-taken`: another program holds Ctrl+Alt+X at the start.
- `overlay-active`: a craft is active, so the overlay is visible.

Nothing else may be here: `fixtures_are_current` fails on any other JSON
file. The views are made by `crates/ch-app/tests/fixtures.rs` by driving the
app with a recording fake shell, and the test fails when a file differs
from what the Rust types serialise. The main view's types deny unknown
fields, so a field either side adds, drops or renames fails it too. After
a deliberate change, regenerate them and review the diff:

```sh
CH_FIXTURES=1 cargo test --manifest-path apps/crafting-helper/Cargo.toml -p ch-app --locked --test fixtures
```

The page tests render them and hold each one against the TypeScript types.
