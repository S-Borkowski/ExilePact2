# ch-blueprint

The crafting helper's blueprint format. A Rust library (`ch_blueprint`, on
`serde` and `toml` alone, no modifier data) that reads a hand-edited TOML
blueprint under `apps/crafting-helper/blueprints/` into a model, checks its
structure, and refuses what is wrong with one line per error naming the
file, line and column. `ch-engine` resolves a checked blueprint against the
data and runs it.

How to write a blueprint, with every key and the seed blueprints, is in
[docs/crafting-helper/blueprints.md](../../../../docs/crafting-helper/blueprints.md).

## The model in brief

`src/model.rs` is the contract with `ch-engine`. Every value a check may
refuse is a `toml::Spanned` (re-exported), so an error can point at it.
Keys are snake case, enumerated values kebab case; every table refuses a key
it does not know.

- **Top level**: `id` (the file stem), `name`, `classes` (data `class`
  strings), `source` (where the technique came from), optional `notes`,
  one or more `[[start]]` alternatives and the ordered `[[step]]`s.
- **`[[start]]`**: a condition on the item. `rarity` (list of `normal`,
  `magic`, `rare`), `wanted_at_least`, `wanted_prefixes_at_least`,
  `wanted_suffixes_at_least`, `fractured_wanted` (true: a fractured
  modifier is present and every one is wanted; false: none is present),
  `unwanted_at_most`, `essence_binds`.
- **`[[step]]`**: `id`, `say` (the instruction shown), `goal`, optional
  `side`, `bind`, `requires` and `until` (conditions: the start keys plus
  `free_prefixes_at_least`, `free_suffixes_at_least`, `no_desecrated`,
  `unrevealed`, `wanted_remaining_at_most`, `wanted_remaining_exactly`),
  `when = "bound"`, `on_hit`, `on_miss`, and its `[[step.action]]`
  alternatives in preference order.
- **`goal`**: `add-wanted`, `remove-unwanted`, `fill`, `desecrate`,
  `reveal`, `remove-desecrated`, `convert`, `manual`.
- **`bind`**: `essence`, `hardest`, `any-remaining`, `last-remaining`. A
  `reveal` or `convert` step needs one.
- **`[[step.action]]`**: `currency` and optional `omens`. The route takes
  the first action none of whose ids is blocked. Only a `manual` step may
  have no action.
- **Transitions**: `next`, `repeat`, `restart`, `abandon`, `done` or
  `goto:<step id>`. Left out, a hit goes to the next step (or repeats while
  `until` does not hold) and a miss repeats (`Step::hit_transition`,
  `Step::miss_transition`). An unexpected outcome always stops the guide;
  that is the engine's, not the file's.

## The catalogue

`src/catalogue.rs` lists every currency, Greater and Perfect currency,
crafting omen, bone, flux and catalyst from
[crafting-mechanics.md](../../../../docs/crafting-helper/crafting-mechanics.md),
each with its id, name, kind, effect (`transmute`, `augment`, `upgrade`,
`alchemy`, `add`, `swap`, `remove`, `convert`, `desecrate`, `reveal`,
`reroll-values`, `fracture`, `manual`), Minimum Modifier Level, the item
groups it applies to, its highest item level (Gnawed bones: 64) and, for an
omen, the currency it modifies, its side and its change.

- **Ids**: the English name in lower kebab case, apostrophes dropped
  (`Hinekora's Lock` is `hinekoras-lock`; `id_from_name`). The settings'
  blocked list stores these ids.
- **Data-backed ids**: essences, alloys and liquid emotions are not listed;
  their ids come from the data's `name`, and `ch-engine` checks that one
  exists for the blueprint's classes. This crate recognises them by shape
  (`data_backed`: `lesser-`/`greater-`/`perfect-essence-of-…`,
  `essence-of-…`, `…-alloy`, `…liquid-…`), which also gives their effect:
  Lesser, plain and Greater essences upgrade Magic to Rare; Perfect and the
  six special essences, alloys and liquids swap.
- **Placeholders**: `desecrate` as a currency is the bones that fit the
  item, tried Preserved, Ancient, Gnawed (`bones_for`); `reveal` is the
  reveal at the Well of Souls, which nothing blocks; `side-omen` among the
  omens is the Sinistral or Dextral omen of the action's currency for the
  bound target's side (`Currency::side_omen`).
- **Readings, not measurements**: which group Sceptres, Foci and Bucklers
  belong to for the bones, the Refined catalysts' spelling, and
  Crystallisation fitting alloys (the doc marks it not established; the
  owner's posts use it) are this crate's readings. Shards are left out:
  they are not used on an item.

## The error format

Every error is one line, `<path>:<line>:<col>: <message>`, line and column
1-based (the column in characters) from the value's span, or the parser's
for a syntax error. `check(path, text)` and `check_file(path)` report every
error they find, in file order, in three passes, each finishing before the
next starts:

1. TOML syntax.
2. The shape: unknown and missing keys, wrong types, a word that is not one
   of a key's values, a step condition in a `[[start]]`.
3. The meaning: `id` not the file stem, unknown class, unknown currency or
   omen, an omen in the currency's place, an omen that does not modify the
   action's currency (whittling on an exalt), both side omens on one
   action, `side-omen` on a currency with no side omens or on a step with
   no `bind` or `side`, a goal the action cannot produce (an annul for an
   add), a `reveal` or `convert` step with no `bind`, duplicate step ids, a
   `goto:` to a missing step, a step with no action, no start, no step.

```text
tests/fixtures/invalid/unknown-currency.toml:15:12: step `slam`: unknown currency `exalted-orbb`
```

A `Checked` blueprint keeps its path and text, and `Checked::at(span,
message)` makes `ch-engine`'s data-backed errors in the same format.

## The shipped blueprints

`build.rs` embeds every `apps/crafting-helper/blueprints/*.toml`, sorted by
file name, and reruns when the folder changes; an empty or missing folder
gives an empty list. `shipped()` returns them as `(file, text)` and
`check_shipped()` checks each as `apps/crafting-helper/blueprints/<file>`.
The owner's command for checking blueprints, structure and data together, is
`ch-engine`'s `check_blueprints` example.

## Tests

```text
cargo test --manifest-path apps/crafting-helper/Cargo.toml --locked -p ch-blueprint
cargo clippy --manifest-path apps/crafting-helper/Cargo.toml --locked -p ch-blueprint --all-targets -- -D warnings
```

`tests/format.rs` checks the test blueprints under `tests/fixtures/`:
`valid/` ones it must take (`every-key.toml` uses every key), and `invalid/`
ones whose every error it pins to its `file:line:col`.
