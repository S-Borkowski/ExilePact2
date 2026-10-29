# 0001: Crafting Helper routes are planned from each side's free slots

## Status

Accepted, 2026-10-29. Ships with Crafting Helper 0.1.1.

## Context

A blueprint (`docs/crafting-helper/blueprints.md`) is a list of steps the
engine walks in the order they are written, moving between them with
`on_hit` and `on_miss` transitions. The owner's guided craft of 2026-10-29
(Live 1 of the M6 workorder: a rare belt with two target prefixes and one
target suffix on it and one target prefix still missing) and issue #28
showed four problems with that model, all in the shipped
`fill-desecrate-exalt` blueprint:

1. **An unrestricted exalt was offered while a target still needed the
   slot.** The fill step named a plain Exalted Orb. The item had one free
   prefix slot, which the missing prefix target needed, and two free suffix
   slots. An exalt without a side omen can land in that prefix slot, and
   then the target no longer fits.
2. **The step's text promised an omen its action did not carry.** The fill
   step said to use the side's Exaltation omen; its action had none, so the
   route preview and the instruction named a bare Exalted Orb. Where two
   slots of one side were free, nothing stacked Omen of Greater Exaltation
   with the side omen either.
3. **A removal named a plain Orb of Annulment where a side omen protects a
   target.** When the other side held a target, a plain annulment could
   take it; Omen of Sinistral or Dextral Annulment confines the removal to
   the side that holds the junk.
4. **The route could not start where the item stood.** The fixed order and
   its start conditions describe one path from one kind of item. Issue #28:
   a rare with fewer than two targets on it (the owner's belt with none)
   got no route at all, because every start alternative of
   `fill-desecrate-exalt` asks for at least two. A rare with a fractured
   modifier, or a magic item with one good modifier, has the same problem:
   there is no step to begin with that matches what is already on it.

Problems 1 and 3 are not wording: the right currency depends on how many
free slots each side has and how many targets each side still needs, which
a fixed order cannot know when it is written. The rules the fix relies on,
and which of them are documented and which are not established, are in
[`docs/models/crafting-slots-spec.md`](../models/crafting-slots-spec.md).

## Options

**(a) State-ordered blueprints.** A blueprint may say `order = "state"`.
Its steps carry guards (conditions on the item as it stands) and side rules
(`target`, `close`, `blocked`); the engine takes the first step, in file
order, whose guard holds and that can be taken on the item. One engine
module does the slot accounting (each side's free slots, the targets it
still needs, whether it is blocked, reserved, closable or open), and the
conditions, the side rules, the `greater-omen` placeholder and the route
preview all read it. Blueprints without `order` keep today's listed order.

**(b) A planner in `ch-engine`.** The engine picks the next action from the
slots itself, with no step list: blueprints shrink to target sets, or go
away, and one policy in code decides every craft.

Weighed against each other:

- **The unit of technique.** The owner asked for four to six blueprints,
  each drawn from a published method and each saying which facts it relies
  on. Under (a) a blueprint stays that unit: a source, a paraphrase of the
  method, and a `relies_on` list of the spec's not-established facts.
  Under (b) the technique is one policy in code, and there is nothing to
  attach a source or a relied-on fact to.
- **Reach of the change.** (a) is additive: `order = "state"` is opt-in,
  the nine class-specific blueprints keep listed order, and their walks
  stay green as the baseline. (b) replaces the guide's step-id model, the
  route preview and `ch-app`'s branch lines for every blueprint at once.
- **Starting from the item.** What (b) offers, picking the next step from
  what is on the item, (a) gets from state order: a rare with mods, a
  fractured mod or a magic item with one good mod starts at the first step
  whose guard holds.

## Decision

Option (a). Blueprints may be state-ordered, and the slot accounting lives
in one `ch-engine` module that both orders can read. The mechanism is
written down first, in
[`docs/models/crafting-slots-spec.md`](../models/crafting-slots-spec.md),
modelled in stdlib Python (`tools/crafting_helper/slot_model.py`) with
baseline and target tests, and the engine is held to the model through one
shared case file (`apps/crafting-helper/crates/ch-engine/tests/items/slots/cases.json`).

Four state-ordered blueprints ship in 0.1.1: `close-a-side`,
`essence-desecrate-double-slam`, `magic-augment-essence` and
`rare-clean-then-slam` (the last one for #28).

### Rejected alternatives

- **Fixing `fill-desecrate-exalt` in listed order.** Its fixed order is the
  defect: whichever order is written, some item reaches a step whose
  currency is wrong for its slots.
- **A preview that shows only the next step.** The craft window lists the
  whole route today. The projection (the path the craft takes if each step
  hits) keeps that, and it is also the oracle the model and the engine are
  compared on.
- **Homogenising omens, Greater Annulment and Chaos with the Erasure omens
  in the new routes.** The Homogenising omens are on the Currency Exchange
  only in Standard, Greater Annulment has been unobtainable since 0.3.0,
  and which side a Chaos Orb's replacement modifier lands on is not
  established.
- **Moving `ch-app`'s whittle-or-annul choice for listed blueprints into
  the engine.** State order decides the removal in the engine; listed
  blueprints keep `ch-app`'s choice, unchanged.

## Consequences

- **Listed blueprints are unchanged.** A blueprint without `order` reads,
  plans and guides as before; the nine class-specific blueprints and their
  walks are the proof.
- **`fill-desecrate-exalt` is retired** from the shipped set. Its file is
  kept as a test blueprint under
  `apps/crafting-helper/crates/ch-engine/tests/items/gear/fill-desecrate-exalt/`,
  where it pins the listed-order baseline: on the owner's belt it still
  names a plain Exalted Orb first, and it still rejects a rare with one
  target. Undo: move the file back and add it to the shipped list.
- **The preview is a projection.** For a state-ordered blueprint the route
  lists the steps taken if each one hits, from the item as it stands; a
  route whose hit path cannot finish is rejected with the reason. The first
  step's odds are as before; later steps' odds are given on the projected
  item (the spec's § "Projection").
- **`ch-app`'s whittle-or-annul stays for listed blueprints.** On a
  state-ordered step the engine's removal pick is final: it never chooses
  an uncertain whittle while an annulment can be taken, so the app has
  nothing to override.
- **Blueprints name what they rely on.** Every state-ordered blueprint has
  a `relies_on` list, and `ch-blueprint` refuses an id that is not a
  not-established fact of the spec's table; a test keeps the two lists
  equal. The craft window does not show the list (the owner's default,
  reversible): the blueprint file and `blueprints.md` carry it.
- **Two facts the double slams lean on are not established**: Greater
  Exaltation with a side omen giving two modifiers of that side, and
  Greater Exaltation needing two free slots. 0.1.1 ships with them
  labelled so and named in each blueprint's `relies_on`; a live session can
  change the label to measured.
- **A game patch that changes slots or omens** is a change to the spec
  first, then to the model and its cases, then to the engine.
