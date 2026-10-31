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
- **`Engine::relax(item, targets, blocked) -> Result<Option<Relaxation>,
  Refusal>`** (0.2.0): the one-target tier relaxation, `None` when the
  targets already route or no single lowered floor makes a route. See
  "Tier relaxation in brief" below.
- **`Engine::blocker_advice(item, targets, blocked, blueprint) ->
  Result<Option<BlockerAdvice>, Refusal>`** (0.2.0): for a state-ordered
  route whose reveal is followed by an exalt on the same side, the
  desecrated modifiers to take if the target is not offered, with that
  exalt's chance of a target before and after. See "Blocker advice in
  brief" below.
- **`StepView::swap: Option<EssenceSwap>`** (0.2.0): on a state-ordered
  essence swap step, what its removal may take, the fractured modifiers it
  leaves and, when it may take an unrevealed desecrated modifier, that
  modifier's survival (`Survival`) with the step taken next on each
  outcome. See "The essence swap's survival in brief" below.
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
  (Preserved, Ancient, Gnawed), `side-omen` to the Sinistral or Dextral
  omen of the bound target's side (or the step's `side`), and
  `greater-omen` to Omen of Greater Exaltation where two fit (below) or to
  nothing. Since 0.2.1 (issue #42), on an Exalted Orb of any grade a
  Sinistral or Dextral Exaltation omen, resolved from `side-omen` or
  written literally, is left out when its side has a free slot and the
  other side has none (`lands_there_anyway`): it goes to
  `ChosenAction::omens_left_out`, not to the action's omens, and is never
  checked against the blocked list, in both orders. Since 0.2.0, a listed step with `when = "bound"` and
  `bind = "last-remaining"` and no `side` (the whittle's desecration),
  planned while several targets are still missing, resolves `side-omen`
  to the side they all share; when they lie on both sides, the
  alternative is taken with its omen waiting for the last target's side
  (no omen id, `ChosenAction::side_omen_waits` set) and counts as blocked
  when either of its currency's two side omens is. The guide picks the
  step again once one target is left and names that side's omen. A
  `last-remaining` step without `when` (the rescue, which requires one
  target missing) keeps an unbound `side-omen` unfit, so it is still
  rejected with two targets missing. Since 0.2.0 (issue #40), `lord-omen`
  on a desecration resolves to the bound target's lord omen (Sovereign for
  Ulaman, Liege for Amanamu, Blackblooded for Kurgal) where the target is
  a desecrated modifier every tier of which, on the item's variant, names
  that one lord in its name (`Ulaman's`, `of Ulaman`), the class is in
  `LORD_CLASSES` and the omen is not blocked; otherwise to nothing, with
  no fallback line (`LordOmen`). It needs no `side-omen` beside it: the
  plain bone the whittle and `essence-desecrate-double-slam` fall back to
  when the Necromancy omen is blocked carries `lord-omen` alone and names
  the lord omen alone. On the whittle's desecration, unbound
  while several targets are missing, it waits too:
  `ChosenAction::lord_omens_waiting` lists each remaining target that
  would take one, with its omen. Every alternative blocked rejects the blueprint
  (`EveryAlternativeBlocked`); none usable, `NoAlternativeFits`. On class
  `Jewels`, a currency with a Minimum Modifier Level (the Greater and
  Perfect orbs) is unfit in either order (since 0.1.4: a jewel's modifiers
  have one level each, so it only adds cost), and the step takes its next
  alternative.
- **`weak-magic-start`** (since 0.1.4, either order, checked last on a
  planned route): `Rejection::WeakMagicStart` when the item is Magic, its
  prefix is a target at any tier, a step of the route makes it Rare while
  Magic (a Regal Orb of any grade, or a Lesser, plain or Greater
  essence), and the prefix's tier is worse than the second best its
  modifier rolls on the base's variant at the item level. It names the
  modifier, its tier, the item level and the top two tiers. No item level,
  an unknown tier or one reachable tier (the top two are that one) never
  rejects. Engine-only: the slot model has no tiers.
- **Order**: fewer classes first, then load order. No cost, no expected
  number of tries. Since 0.2.0 the two-class `jewellery-catalyst-exalt`
  (Amulets, Rings) is therefore offered before the many-class routes on a clean
  rare ring or amulet, which renumbers their routes.

That is listed order, what a blueprint without `order` keeps. A
state-ordered blueprint (`order = "state"`) is planned and guided by the
rules below instead.

## State order in brief

The mechanism is written down in
[docs/models/crafting-slots-spec.md](../../../../docs/models/crafting-slots-spec.md)
and modelled in stdlib Python (`tools/crafting_helper/slot_model.py`); the
engine computes the same numbers and picks the same steps.

- **Slot accounting** (`src/slots.rs`, the one module the conditions, the
  side rules, the placeholders and the projection all read): per side its
  cap for the rarity (`ch-data`'s affix limits), its modifiers, `free`, the
  targets it still `need`s (less one an unrevealed modifier there waits
  for; on a Magic item at most the cap less the targets there) and
  `spare = free - need`. A side is **blocked** when spare < 0 and it holds
  a removable non-target (never fractured, unrevealed or desecrated),
  **reserved** when every free slot is needed, **closable** when it needs
  nothing and the other side does, **open** with a free slot. The
  conditions `blocked_side`, `closable_side`, `one_side_open` and
  `desecrated_unwanted` read these (in listed blueprints too).
- **Next step**: the first step in file order that is a candidate on the
  item as it stands: its `guard` holds, its binding is made, its goal is not
  met, its side rule (`target`, `close`, `blocked`) finds a side and an
  alternative fits (a free slot where an add, fill or essence lands, one on
  a desecration's side). `essence` and `hardest` are made once, as soon as
  the item allows (`essence` on a Magic item, `hardest` while a target is
  missing), shared by every step with that binding and kept by the guide.
  `last-remaining` is made afresh on every item, to the one missing target
  when exactly one is missing and to nothing otherwise, and never kept: kept
  like `hardest`, a miss that leaves a second target missing would leave a
  desecration bound to a target that is no longer the last. No transitions:
  the guide picks again after every change (`Outcome::transition` is
  `picked`), is done when no target is missing, and stuck with every step's
  reason otherwise.
- **Certain removals first** (since 0.1.4; the spec's "Certain removals
  first"): when the first candidate is not a certain removal and a later
  one is a `remove-unwanted` step whose chosen alternative is certain
  (every modifier it may take a non-target, or a certain whittle), or an
  essence swap whose removal is, the first such later candidate is next
  (`certain_removal`). The projection takes the same pick. Add certainty
  stays in the odds and never reorders.
- **A wrong reveal waits for the same-side exalt** (since 0.2.0; the
  spec's section of that name): a `remove-desecrated` step guarded by
  `desecrated_unwanted = true` (`light`, `light-last`) is passed over while
  an `add-wanted` step is a candidate on the side of the revealed
  desecrated non-target with a free slot there (`light_waits`); its
  reason, and the projection's skipped reason, say it waits for that
  step. The modifier stays through the exalt, keeping its families out of
  the pool; Light comes once the side is full or nothing more can be
  added there.
- **Essence swaps** (since 0.1.4, engine-only): an `add-wanted` step whose
  chosen alternative is a Perfect or special essence (`Effect::Swap`)
  fits on a Rare when its `essence` binding (made on a Rare for such a
  step) names a missing target the essence adds at its floor, the
  removal's side holds a removable modifier, and the target's side has
  room once it is gone. The removal's side is the Crystallisation omen's;
  with none, the essence modifier's side when it is full
  (`essence-full-side-removal`); otherwise either. Its projection takes
  the first removable non-target on that side, fillers last (`swap_takes`),
  and places the target. Alloys and liquid emotions stay listed-order
  only.
- **Fitting**: an Exalted Orb without a side omen is passed over while both
  sides are open and one is reserved; a side Exaltation omen on an exalt
  whose other side is full is left out rather than named (above), so a
  blocked one passes nothing over; `greater-omen` resolves to Omen of
  Greater Exaltation only where the side has two free slots the step can
  use (two placeable targets, or a fill) and the omen is not blocked; a
  removal takes Omen of Whittling only when the whittle is certain (one
  lowest-level modifier, a removable non-target on its side, every level
  known) and the first annulment that would otherwise fit could take a
  target, and a Sinistral or Dextral Annulment only when the other side
  holds a target. A `last-remaining` desecration does not fit when the
  class has no desecrated modifier on the target's side at the item level
  (`no desecrated <side> modifier on <class> at item level <n>`). The
  engine's pick is final.
- **The last-target desecration** (spec, "The last-target desecration"):
  on a rare with no desecrated modifier (unrevealed ones included) and
  exactly one target missing, each state-ordered blueprint places it
  with a `desecrate-last` step bound to `last-remaining` (a bone
  and the side's Necromancy omen, nothing else) and a reveal with Omen of
  Abyssal Echoes; a wrong reveal is taken off with Omen of Light and an
  Orb of Annulment (once its side is done, above) and the bone used
  again. When that desecration
  is not a candidate and a later step is picked in its place, the picked
  step's `StepView::instead_of_desecration` says why in words (no pool on
  the side, the bone or the Necromancy omen blocked, or the engine's own
  reason); the app shows it as the step's Fallback line. `None` otherwise,
  and on the steps before it, which clear the way rather than replace it.
- **The projection** (`Route::steps`): the steps the craft takes if each
  one hits, from the item: an add places its targets (two with Greater), a
  fill adds fillers (non-targets of unknown level), an essence makes the
  item Rare with its target, a desecration leaves an unrevealed modifier
  waiting for its target, a reveal places it, a removal takes the
  non-target its action would. A step may come twice. The first step's
  odds are read off the item as in listed order; a later one's off the
  projected item, saying "if the earlier steps hit".
- **Rejections**, in order after the class and the start:
  `TargetsExceedSlots` (a side's targets alone exceed its Rare cap),
  `DesecratedTargetUnplaced` (no desecrate step), and `RouteCannotFinish`
  with `why` = `no-candidate` (with each step's reason), `no-change` (a
  step's hit changes nothing) or `step-cap` (the projection reaches
  `STEP_CAP`, 16 steps), then `WeakMagicStart` (above).
- **Hit, miss, unexpected** as in listed order; a `side = "blocked"`
  removal hits when a non-target went from a side that was blocked before
  it.

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
  desecrated one; a Fracturing Orb fractures one modifier; a manual step
  accepts anything); or a side omen's side
  was not the side that changed. The guide stops (`Status::Replan`).
- **Hit**: the goal is met (the bound or a remaining target is now there at
  its floor; an unwanted modifier went; a slot filled; an unrevealed
  modifier appeared; the desecrated one went; a target became fractured)
  and no wanted modifier was
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
fluxes and every jewel are unknown, with the exceptions below.

Since 0.1.4: an add whose pool's exclusions leave only the target, every
tier counting, is certain, and one that leaves it out is certain not to
hit, weights or not (`ch-data`'s `Pool::certainty`), so an isolation
shows on a jewel too. An essence swap's removal factor is over its
removal's side (the Crystallisation omen's, or the full side of the
essence's modifier, above). A Fracturing Orb is the share of targets
among the modifiers it may fracture: explicit, not fractured, not
desecrated (revealed or not) and not the Mark of the Abyssal Lord
(`fracture_odds`; the basis names `abyss-mark-unfracturable` and
`desecrated-unfracturable`, not established). A listed desecration with a
Necromancy omen on a full side is its removal's share of non-targets on
that side, not counting the reveal (`full-desecration-same-side`).

Since 0.2.0: an exalt with Omen of Catalysing Exaltation has the plain
slam's odds, its basis ending with `CATALYST_NOTE` ("the catalyst's favour
is not counted"): the favour's size is not established and the helper
does not read catalyst quality.

## Tier relaxation in brief

Since 0.2.0. An essence binds a target only at its floor or better, and a
class's best essence for a modifier is often a tier short of what the
player asks for: on belts the Greater resistance essences add tier 3, so
Fire, Cold and Lightning Resistance at T2 on a magic belt plan no route.
`Engine::relax` looks for the one floor to lower, and only when
`Engine::plan` finds no route. For each target in target order, each
data-backed currency of the class (essences, Perfect essences, alloys,
liquid emotions: the only things here that guarantee a modifier) that adds
that target's modifier at a tier worse than its floor, is not blocked and
fits the item level is a candidate. A candidate holds when re-planning
with that one floor moved to the currency's tier gives some route a step
whose chosen action uses the currency. The smallest tier drop wins, then
target order, then data order. The `Relaxation` names the target, its
modifier lines, the old and new floor, the currency (catalogue id and
English name), the target list to plan with and the blueprints that route
through the currency. Accepting it is an ordinary plan with that target
list. It never lowers two floors, never raises one and proposes nothing
when a route exists.

## Blocker advice in brief

Since 0.2.0. The app never reads the Well of Souls' offers
(`desecrate-adds-unrevealed`), so a reveal step says up front which
desecrated modifier to take if no target is offered. `Engine::blocker_advice`
projects the route (state order only; `None` for a listed route, an
unknown blueprint or a rejected one) and finds its first reveal; the
desecration's side is the side of the modifier it reveals. The advice
needs a later add on that side with an Exalted Orb of any grade; the last
one is scored. On the item as that exalt finds it, the reveal's slot is
taken first by a modifier that keeps nothing out (`before`), then by each
of the class variant's desecrated modifiers on that side that the item
level reaches and that are not targets (`after`), and the exalt's chance of
a missing target on that side is read as `add_chance` reads every add: the
families of every revealed modifier on the item, desecrated ones included,
leave the pool. Only modifiers that raise the chance are named, at most
`BLOCKER_LIMIT` (three), best first; equal chances share one
`BlockerPick`, with the ordinary modifiers they keep out (`keeps_out`).
Ordinary modifiers a reveal may offer (`reveal-offers-base`) are left out.
On the relaxed magic belt (Fire Resistance T3, Cold and Lightning T2) the
double slam's reveal names `desecrated-suffix-to-strength-and-dexterity`
and `desecrated-suffix-to-strength-and-intelligence`, keeping
`suffix-to-strength` out of the suffix exalt's pool.

## The essence swap's survival in brief

Since 0.2.0 (the shipped `fractured-essence-swap`; spec, "The essence swap
and an unrevealed modifier"). A state-ordered step whose action is a
Perfect or special essence carries `StepView::swap`, an `EssenceSwap`: the
side its removal takes from (the Crystallisation omen's, else the target's
side when it is full, else either), the modifiers it may take (`removes`,
every one there that is not fractured) and the fractured ones it leaves.
When one it may take is unrevealed, `survival` gives that modifier's
survival chance as known odds, (n-1) in n over the n modifiers it may take
(`removal-equally-likely`; that an unrevealed modifier is in the pool is
owner-stated), and for each outcome the step state order takes next on the
item that outcome leaves, by its own `next_step` (no rule of its own):
`stays` on the projection's own hit (which never takes an unrevealed
modifier), `goes` on the item with the unrevealed modifier removed and the
essence's modifier added. The step's own odds and its place under Certain
removals first are not changed. On the test ring (fractured, unrevealed and
ordinary suffixes, all three slots taken) the survival is 1 in 2; if it
stays the reveal is next, and if it goes the suffixes are full again, so
`remove` annuls with Omen of Dextral Annulment at 1 in 2 (it may take the
essence's modifier), then `desecrate-last` with Dextral Necromancy and
`reveal-last`. State order keeps no miss branch: an annulment that took the
essence's modifier re-plans from the item when it is read again, and the
essence step comes back bound to that modifier. The listed order sets no
`swap`.

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
cargo test --manifest-path apps/crafting-helper/Cargo.toml --locked -p ch-engine --test state_order
cargo test --manifest-path apps/crafting-helper/Cargo.toml --locked -p ch-engine --test last_desecration
cargo test --manifest-path apps/crafting-helper/Cargo.toml --locked -p ch-engine --test rules
cargo test --manifest-path apps/crafting-helper/Cargo.toml --locked -p ch-engine --test fracture
cargo test --manifest-path apps/crafting-helper/Cargo.toml --locked -p ch-engine --test seeds_guides
cargo test --manifest-path apps/crafting-helper/Cargo.toml --locked -p ch-engine --test relax
cargo test --manifest-path apps/crafting-helper/Cargo.toml --locked -p ch-engine --test blocker
cargo test --manifest-path apps/crafting-helper/Cargo.toml --locked -p ch-engine --test essence_swap
cargo test --manifest-path apps/crafting-helper/Cargo.toml --locked -p ch-engine --test lord_omen
cargo test --manifest-path apps/crafting-helper/Cargo.toml --locked -p ch-engine --test seeds_codex
cargo test --manifest-path apps/crafting-helper/Cargo.toml --locked -p ch-engine
cargo clippy --manifest-path apps/crafting-helper/Cargo.toml --locked -p ch-engine --all-targets -- -D warnings
```

`tests/engine.rs` covers route choice and outcomes over the test
blueprints and constructed items in `tests/items/engine/` (its README lists
them) and `ch-item`'s real fixtures, and holds the listed-order baseline:
the listed `fill-desecrate-exalt` (retired from the shipped set in 0.1.1),
kept as a test copy in
`tests/items/gear/fill-desecrate-exalt/`, still names a plain Exalted Orb on
two prefixes and a suffix and still rejects a rare with one target.
`tests/state_order.rs` covers state order over the `test-state-*`
blueprints and `ring-state-*` items there: the guide's pick, the
projection, the passed-over exalt, `greater-omen`, the removals, the
desecration's free slot and the rejections, and on the shipped blueprints
a wrong reveal kept through the same-side exalt, Light coming once the
side is full (`a_wrong_reveal_waits_for_the_same_side_exalt`).
`tests/last_desecration.rs`
covers the last-target desecration over the shipped blueprints and the
owner's two-prefix, one-suffix belt and its later states in
`tests/items/last-desecration/` (constructed; its README lists them): the
desecration and reveal on an undesecrated rare, an item with a desecrated
target keeping its route, the fallback reason with no pool or a blocked
Necromancy omen, two missing targets left alone, and a wrong reveal lit
off and desecrated again. `tests/rules.rs` (0.1.4) holds the guides'
route rules over the test blueprints and constructed items in
`tests/items/rules/` (its README lists them), each beside its baseline:
a jewel takes the plain Exalted Orb while a ring keeps the Perfect one
(`a_jewel_route_takes_the_plain_exalted_orb`,
`a_ring_route_keeps_the_perfect_exalted_orb_baseline`),
`weak-magic-start` on magic boots with movement speed T3 at item level 82
and not with T2 (`a_weak_magic_prefix_rejects_a_route_that_spends_an_upgrade`,
`a_top_two_magic_prefix_keeps_its_route_baseline`), a certain removal
before an uncertain step (`a_certain_removal_goes_before_an_uncertain_step`,
`an_uncertain_removal_keeps_file_order_baseline`), and the essence swap's
odds, side and order (`an_essence_swap_shows_its_removal_odds`,
`an_essence_swap_on_a_full_side_removes_from_that_side`,
`a_certain_essence_swap_goes_before_an_uncertain_step`,
`an_essence_swap_with_a_target_on_its_side_keeps_file_order_baseline`).
`tests/fracture.rs` (0.1.4) covers the `fracture` goal and the count
conditions over constructed boots in `tests/items/fracture/`: the
Fracturing Orb's odds leave out the Abyss mark and desecrated modifiers
(`fracture_odds_leave_out_the_abyss_mark_and_desecrated`), a fractured
target is a hit and another fractured modifier a miss, the counts take
every explicit modifier, and a desecration on a full side shows its
removal's odds (`a_desecration_on_a_full_side_shows_the_removal_odds`).
`tests/seeds_guides.rs` (0.1.4) walks the four blueprints from the guides,
`fracture-first`, `full-rare-desecration-rescue`,
`weapons-physical-generic` and `body-armours-single-prefix`, each through
`<id>_hit_path`, `<id>_miss_path` and `<id>_start_refuses`, over the
constructed items under `tests/items/guides/<id>/` (a README each): the
weapon and the armour refuse a weak magic prefix (`weak-magic-start`), and
fracture first and the rescue a rare with the wrong modifier count; the
two state-ordered ones are not in the slot model, so these walks pin
them. `tests/relax.rs` (0.2.0) covers the tier relaxation over the
shipped blueprints and the constructed Heavy Belts in `tests/items/relax/`
(its README lists them): the magic belt with three T2 resistances plans no
route (`belt_t2_resistances_have_no_route_baseline`) and is offered Fire
Resistance T2 to T3 by Greater Essence of Insulation
(`belt_t2_resistances_propose_greater_insulation`), the normal belt is
offered Cold Resistance by Greater Essence of Thawing through
`magic-augment-essence` (`a_normal_belt_proposes_greater_thawing`) and
nothing with the transmutation orbs blocked
(`a_normal_belt_gets_no_relaxation`), and a selection that routes gets
nothing (`a_routed_selection_gets_no_relaxation`). `tests/blocker.rs`
(0.2.0) covers the blocker advice over the shipped blueprints and the
relaxed magic belt in `tests/items/blocker/`: the double slam's reveal
names both Strength-sharing desecrated suffixes, after above before
(`blocker_advice_ranks_the_strength_blockers`), and
`magic-augment-essence`, whose desecration is its last add, gets none
(`no_blocker_advice_for_a_last_desecration`). `tests/essence_swap.rs`
(0.2.0) covers the essence swap's survival over the shipped
`fractured-essence-swap` and the constructed items in
`tests/items/essence-swap/` (its README lists them): the issue #28 ring
with the listed test swap keeps 2 in 4 and carries no survival
(`no_survival_without_an_unrevealed_modifier_baseline`); the ring with a
fractured, an unrevealed and an ordinary suffix gets a certain add and a
survival of 1 in 2, the reveal if it stays and the Dextral annulment at
1 in 2 if it goes (`a_ring_swap_gives_the_unrevealed_suffix_one_in_two`);
that ring after "goes" annuls, desecrates and reveals
(`a_ring_swap_goes_annuls_then_desecrates`); after an annulment that took
the essence's modifier it re-plans with the essence again
(`a_ring_swap_annulment_miss_replans`); the quiver is the prefix case with
Sinistral Annulment
(`a_quiver_swap_gives_the_unrevealed_prefix_one_in_two`); and a Dusk Ring's
one ordinary suffix goes for certain with no survival
(`a_dusk_ring_swap_removes_its_one_throwaway`).
`tests/lord_omen.rs` (0.2.0, issue #40) covers `lord-omen` over the
shipped blueprints and the constructed items in `tests/items/lord-omen/`
(its README lists them): an ordinary target takes the Necromancy omen
alone (`a_base_pool_target_takes_no_lord_omen_baseline`), a ring's
Ulaman prefix takes Omen of the Sovereign beside it
(`a_ring_desecration_for_an_ulaman_target_takes_the_sovereign_omen`), a
body armour and a belt take none (`an_armour_desecration_takes_no_lord_omen`,
`a_belt_desecration_takes_no_lord_omen`), a blocked lord omen leaves the
Necromancy omen and no fallback
(`a_blocked_lord_omen_keeps_the_necromancy_omen`), both Necromancy omens
blocked leave an ordinary target the plain bone with no omen
(`a_blocked_necromancy_omen_takes_the_plain_bone_baseline`) and a lord's
target the plain bone with its lord omen alone, in the whittle and in
`essence-desecrate-double-slam`
(`a_blocked_necromancy_omen_keeps_the_lord_omen`), a Bow's Ulaman prefix
takes the Sovereign in `weapons-physical-generic`'s preview
(`a_bow_desecration_for_an_ulaman_target_takes_the_sovereign_omen`),
and the whittle's waiting desecration lists which target would take which lord omen
(`a_waiting_whittle_desecration_waits_for_its_lord_omen`); `route.rs`'s
unit test reads a lord from tier names, two lords or none giving nothing.
`tests/seeds_codex.rs` (0.2.0) walks `jewellery-catalyst-exalt` through
`jewellery_catalyst_exalt_hit_path`, `_miss_path` (a missed suffix
annulled with Dextral Annulment, then the catalysts again) and
`_start_refuses` (a modifier that is not a target, a magic item) over the
constructed ring in `tests/items/codex/` (its README lists them), and its
slam's odds: the plain slam's numbers with the catalyst note
(`a_catalysing_slam_says_the_favour_is_not_counted`), and no note on a
slam without Catalysing (`a_plain_slam_carries_no_catalyst_note_baseline`).
`tests/common/` holds the helpers every engine test file shares. A `-p` build does not need the npm
build.

`tests/items/slots/cases.json` is the contract between the model and the
engine: each case's abstract item and targets and what each of the four
modelled state-ordered blueprints (`V2`; not the two from the guides)
must preview on it. 0.1.4 added two cases, `rare-certain-removal-first`
and `rare-uncertain-removal-keeps-order`, for Certain removals first.
0.2.0 gave each desecrated-pool target its `lord` (issue #40, the
`lord-omen` rule), which `tests/slots.rs` checks against the tier names of
the real modifier behind its alias; `magic-two-good`'s desecrations now
record Omen of the Liege, from the model's side. The model owns it
(`tests/test_crafting_helper_slot_model.py`); the engine's parity test,
`tests/slots.rs`, builds a constructed item per case and asserts every
expectation. A disagreement is settled by the spec, never by editing the
case file from the engine's side.
