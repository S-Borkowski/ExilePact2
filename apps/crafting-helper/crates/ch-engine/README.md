# ch-engine

The crafting helper's engine. A Rust library (`ch_engine`, on `ch-data`,
`ch-blueprint`, `ch-item`, `serde` and `serde_json`) that, for a parsed
item, the target modifiers with their tier floors and the player's blocked
currencies, picks a blueprint route, then classifies each next item state as
hit, miss or unexpected and gives the next step with its rough odds. No UI:
`ch-app` re-exports it for M5.

The blueprint format is `ch-blueprint`'s
([README](../ch-blueprint/README.md),
[docs/crafting-helper/blueprints.md](../../../../docs/crafting-helper/blueprints.md));
the data, the join and the add and removal odds are `ch-data`'s
([README](../ch-data/README.md)).

## The API

- **`Engine`**: the blueprints. `Engine::shipped()` loads the ones embedded
  from `apps/crafting-helper/blueprints/`; `Engine::new(checked)` and
  `Engine::load(paths)` take others. Each is structurally checked by
  `ch-blueprint`, then resolved here (`resolve`): an essence, alloy or
  liquid emotion id must exist in the data of at least one of the
  blueprint's classes, refused at its line and column otherwise. Two
  blueprints with one id are refused.
- **`Engine::plan(item, targets, blocked) -> Result<Plan, Refusal>`**: every
  applicable `Route` and every `Rejected` blueprint with its `Rejection`.
  A `Target` is a data modifier id and an optional floor (1 is the best
  tier; no floor means the best tier the item level reaches).
- **`Engine::start(blueprint, item, targets, blocked) -> Started`**: the
  route, the `Guide` and the first `Next` step.
- **`Engine::observe(&mut guide, before, after) -> Outcome`**: the verdict on
  one change and the next step. **`Engine::current(guide, item)`**: the
  guide's step on an item.
- **`catalogue()`**: every crafting item the blocked list may name, the
  static part (`ch-blueprint`) then the data-backed essences, alloys and
  liquid emotions (one id per English name, with the classes it exists
  for).

Every type M5 renders derives `Serialize`; `Guide` (with `Target` and
`Status`) derives `Deserialize` too, and names its blueprint by id, so the
engine looks the blueprint up again on every call. `Odds` is
`ch_data::Odds`: known (a percent and its basis, `5000 of 133355 weight`) or
unknown with the reason.

## Route choice in brief

- **Refused up front** (`Refusal`): a class outside the 22, an unknown
  variant, a corrupted, mirrored or unidentified item, the short form (no
  tiers), a rarity other than Normal, Magic or Rare, no item level, no
  target, or a target the item cannot roll at its floor (or two targets of
  one family).
- **Start**: the first `[[start]]` alternative that holds; none, and the
  blueprint is rejected with what each failed. `essence_binds` asks whether
  any unblocked essence or liquid emotion of the class adds a remaining
  target at its floor (a Normal item counts as Magic here, before the
  blueprint's own steps).
- **Bindings**, made once per route and kept by the guide: `essence` binds
  the first remaining target an unblocked essence among the step's actions
  adds at the floor or better, its grade fitting the rarity (Lesser, plain,
  Greater on Magic; Perfect, special, alloys and liquids on Rare);
  `hardest` the remaining target with the lowest add odds on its side
  (unknown counts as hardest, ties go to target order), a desecrated-section
  target first for a desecrate or reveal step, and a reveal step takes the
  desecrate step's target. `any-remaining` aims at every remaining target;
  `last-remaining` at the one left, or at every one while several remain.
  A desecrated-section target binds only to a desecrate or reveal step; a
  blueprint that cannot place one is rejected.
- **Skipped steps**: a `when = "bound"` step whose binding did not resolve,
  and a step whose goal is already met (the target is there, nothing
  unwanted to remove, no free slot to fill, an unrevealed modifier already
  there).
- **Actions**: the first alternative none of whose ids is blocked, with
  `desecrate` resolved to the bones that fit the class and item level
  (Preserved, Ancient, Gnawed) and `side-omen` to the Sinistral or Dextral
  omen of the bound target's side (or the step's `side`). Every alternative
  blocked rejects the blueprint (`EveryAlternativeBlocked`); none usable,
  `NoAlternativeFits`.
- **Order**: fewer classes first, then load order. No cost, no expected
  number of tries.

## Outcomes in brief

Given the item before and after the step:

- **Unexpected**: the class, base type or item level changed; the item
  became corrupted, mirrored or unidentified; a fractured modifier changed;
  a new revealed modifier does not join the data (an essence's own lines
  excepted); the change does not fit the action's effect (an exalt adds
  one, two with Greater Exaltation; a chaos, Perfect essence, alloy, liquid
  or flux swaps one; an annulment removes one, two with Greater Annulment;
  a regal or Lesser, plain or Greater essence makes Magic Rare with one
  more; a desecration adds one unrevealed; a reveal turns it into a
  desecrated one; a manual step accepts anything); or a side omen's side
  was not the side that changed. The guide stops (`Status::Replan`).
- **Hit**: the goal is met (the bound or a remaining target is now there at
  its floor; an unwanted modifier went; a slot filled; an unrevealed
  modifier appeared; the desecrated one went) and no wanted modifier was
  lost. **Miss**: the change fits and the goal is not met; a target below
  its floor is a miss, and counts as unwanted for removal.
- Every outcome lists the targets gained, lost and rolled below their
  floor. The next step follows the blueprint's transition (left out: a hit
  goes on, or repeats while `until` fails; a miss repeats), skipping steps
  the route would skip. A step whose `requires` fails stops the guide as
  stuck, naming what failed; it never picks another step.

## Odds in brief

One step, never a simulation: an add is the targets' counting weight
(tiers at or better than the floor) over the pool of the sides it can land
on (`ch-data`'s pool, Minimum Modifier Level included); a removal is the
chance the removed modifier is not a wanted one (whittling: certain when
the lowest level is unique; Omen of Light: the desecrated one); an essence
is certain, times the removal factor when it removes. Desecration, reveals,
fluxes and every jewel are unknown.

## The CLI

```text
cargo run --quiet --locked --manifest-path apps/crafting-helper/Cargo.toml -p ch-engine --example check_blueprints -- apps/crafting-helper/blueprints
```

Takes files or folders (a folder's `.toml` files, sorted), runs the
structural and the data-backed checks, prints one
`<path>:<line>:<col>: <message>` line per error and one `<path>: ok` line
per valid file, and exits 1 on any error (2 with no argument).

## Tests

```text
cargo test --manifest-path apps/crafting-helper/Cargo.toml --locked -p ch-engine --test engine
cargo test --manifest-path apps/crafting-helper/Cargo.toml --locked -p ch-engine
cargo clippy --manifest-path apps/crafting-helper/Cargo.toml --locked -p ch-engine --all-targets -- -D warnings
```

`tests/engine.rs` covers route choice and outcomes over the test
blueprints and constructed items in `tests/items/engine/` (its README lists
them) and `ch-item`'s real fixtures; `tests/common/` holds the helpers every
engine test file shares. A `-p` build does not need the npm build.
