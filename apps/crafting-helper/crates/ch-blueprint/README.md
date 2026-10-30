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
  optional `order` and `relies_on` (below), one or more `[[start]]`
  alternatives and the `[[step]]`s.
- **`order`**: `listed` (the default; `Blueprint::order`) or `state`. A
  listed blueprint runs its steps in file order, moved between by
  transitions. A state-ordered one takes the first step whose `guard`
  holds and whose goal is not met on the item as it stands, so a mid-way
  item starts where it is; the rules are
  [crafting-slots-spec.md](../../../../docs/models/crafting-slots-spec.md),
  "Route rules", and `ch-engine` runs them.
- **`relies_on`**: the not-established facts the route relies on, by their
  id in the spec's facts table. Required, possibly `[]`, when `order =
  "state"`; allowed, and checked, otherwise.
- **`[[start]]`**: a condition on the item. `rarity` (list of `normal`,
  `magic`, `rare`), `wanted_at_least`, `wanted_prefixes_at_least`,
  `wanted_suffixes_at_least`, `fractured_wanted` (true: a fractured
  modifier is present and every one is wanted; false: none is present),
  `unwanted_at_most`, `essence_binds`, `modifiers_at_least` and
  `modifiers_at_most` (since 0.1.4: the number of explicit modifiers,
  fractured, desecrated and unrevealed ones counted), and the four slot
  keys, each a bool
  (`false` asks for the opposite): `blocked_side` (some side is blocked),
  `closable_side` (some side is closable), `one_side_open` (exactly one
  side is open), `desecrated_unwanted` (a revealed desecrated modifier is
  not a target). Blocked, closable and open are the spec's "Slot
  accounting".
- **`[[step]]`**: `id`, `say` (the instruction shown), `goal`, optional
  `side`, `bind`, `requires`, `until` and `guard` (conditions: the start
  keys plus `free_prefixes_at_least`, `free_suffixes_at_least`,
  `no_desecrated`, `unrevealed`, `wanted_remaining_at_most`,
  `wanted_remaining_exactly`), `when = "bound"`, `on_hit`, `on_miss`, and
  its `[[step.action]]` alternatives in preference order.
- **Which order takes which step key**: `requires`, `until`, `on_hit` and
  `on_miss` are listed order's, and refused in a state-ordered blueprint;
  `guard` and the side rules are state order's, and refused in a listed
  one. `side` is `prefix` or `suffix` in either (`Step::side`), or, in
  state order, a rule word (`Step::side_rule`): `target` (the bound
  target's side, or the open side with the most targets the step can
  place), `close` (the closable side), `blocked` (the blocked side).
- **`goal`**: `add-wanted`, `remove-unwanted`, `fill`, `desecrate`,
  `reveal`, `remove-desecrated`, `convert`, `fracture` (since 0.1.4: a
  target became fractured; only a currency of effect `fracture`, the
  Fracturing Orb, serves it), `manual`.
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
  bound target's side (`Currency::side_omen`); `greater-omen`, on an
  Exalted Orb of any grade only, is Omen of Greater Exaltation where the
  step's side has two free slots and it can use two, and nothing
  otherwise (the spec's "`greater-omen`"). Both may go on one action.
- **Fact ids**: `NOT_ESTABLISHED` lists exactly the ids of the rows
  labelled not established in the spec's facts table, in its order; these
  are what `relies_on` may name. `facts_match_the_spec_table` reads the
  table and fails when the two differ, so a relabelled or added row means
  editing this list. 0.1.4 added `abyss-mark-unfracturable`,
  `desecrated-unfracturable`, `full-desecration-same-side` and
  `essence-full-side-removal`; `alloy-counts-as-crafted` is documented,
  so it is not among them.
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
   `goto:` to a missing step, a step with no action, no start, no step;
   a key the blueprint's order does not take (each at its own line and
   column), `order = "state"` with no `relies_on`, a `relies_on` id that
   is not a known not-established fact or is named twice, `greater-omen`
   on anything but an Exalted Orb or beside Omen of Greater Exaltation;
   and, in every blueprint, a `say` that promises an omen no action of
   the step carries. A `say` that names an omen (its English name, or the
   name without "Omen of" where two or more words are left, such as
   "Dextral Exaltation") needs an action carrying that omen, and one that
   mentions an omen at all (the word "omen" too) needs an action carrying
   one; `side-omen` counts as its currency's two side omens and
   `greater-omen` as Omen of Greater Exaltation. Since 0.1.4 pass 3
   also holds the one-crafted rules and the jewel one: a second step
   that places a crafted modifier (an action naming an essence of any
   grade, an alloy or a liquid emotion, the data-backed ids) with no
   step between the two whose actions include an Orb of Annulment or a
   Chaos Orb of any grade (`check_crafted`, at the second step's
   currency; a plausibility check, since the removal may take another
   modifier); in a listed blueprint, a crafted step that is not
   `manual` with an `until`, or whose miss goes to `repeat`, written (at
   its `on_miss`) or by default (at its currency); and, on a blueprint
   whose every class is `Jewels`, a currency with a Minimum Modifier
   Level (`check_jewel_grades`). A `fracture` step whose action cannot
   fracture is the existing goal check (`Effect::serves`).

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
`valid/` ones it must take (`every-key.toml` uses every key of a listed
blueprint, `state-every-key.toml` every key state order adds), and
`invalid/` ones whose every error it pins to its `file:line:col`
(`say-omen.toml` holds the listed `fill-desecrate-exalt`'s old fill step,
whose text promised an Exaltation omen its action did not carry;
`crafted-twice.toml`, `crafted-repeat.toml`, `jewel-grades.toml` and
`fracture-currency.toml` the 0.1.4 rules, in
`two_crafted_steps_without_a_removal_between_are_refused`,
`a_crafted_step_that_repeats_on_a_miss_is_refused`,
`greater_and_perfect_currencies_are_refused_on_a_jewel_blueprint` and
`a_fracture_step_needs_a_fracturing_orb`; `modifier_count_conditions_parse`
reads the two count keys). It also reads the spec's facts table against
`NOT_ESTABLISHED`.
