# Crafting Helper: writing a blueprint

A blueprint is one crafting technique written down as a short TOML file:
which items it starts from, which steps it takes, which currencies each step
may use, and where to go after a step lands or misses. The crafting helper's
engine (`ch-engine`) reads every blueprint under
[`apps/crafting-helper/blueprints/`](../../apps/crafting-helper/blueprints/),
picks the ones that fit an item and the modifiers you want on it, and guides
you through one step at a time.

The crafting helper's craft window previews a route from a blueprint and
guides a craft through it ([`README.md`](README.md) § "Overview"). A
blueprint is checked with `check_blueprints` (below) and walked in
`ch-engine`'s tests.

A blueprint runs in one of two orders. A **listed** blueprint (the
default) takes its steps in file order and moves between them by
transitions. A **state-ordered** blueprint (`order = "state"`, since
0.1.1) takes, after every change, the first step whose `guard` holds on the
item as it stands, so an item already part-way through the craft (a rare
with mods, a fractured mod, a magic item with one good mod) starts where it
is; § "State-ordered blueprints" below.

- What each currency, omen and bone does:
  [`crafting-mechanics.md`](crafting-mechanics.md)
- The rules a state-ordered route follows on an item's free slots, and the
  facts they rest on with their labels:
  [`docs/models/crafting-slots-spec.md`](../models/crafting-slots-spec.md);
  why routes are planned that way:
  [ADR 0001](../adr/0001-crafting-helper-route-design.md)
- The guide, the crates and their tests: [`README.md`](README.md)

## A short example

One file per blueprint, named `<id>.toml`. A ring brute-forced with Chaos
Orbs and whittling, then desecrated for the last target (the shipped
`rings-resist-whittle`, shortened):

```toml
id = "rings-resist-whittle"                      # the file name without .toml
name = "Rings: resistances by whittling"        # the title; the app does not show it
classes = ["Rings"]                             # data class strings
source = "Discord post 10, owner's paste, 2026-10-28"
notes = "Whether a Chaos Orb can take off a desecrated modifier is not established."

[[start]]                                       # any one start alternative suffices
rarity = ["rare"]

[[step]]
id = "whittle"
say = "Whittle (Omen of Whittling with a Chaos Orb) until one target is left"
goal = "add-wanted"
bind = "any-remaining"
until = { wanted_remaining_at_most = 1 }        # on a hit, repeat until this holds

[[step.action]]                                 # alternatives, preferred first
currency = "chaos-orb"
omens = ["omen-of-whittling"]

[[step.action]]
currency = "perfect-exalted-orb"

[[step]]
id = "desecrate"
say = "Desecrate for the last target"
goal = "desecrate"
bind = "last-remaining"
when = "bound"

[[step.action]]
currency = "desecrate"                          # the bones that fit the item
omens = ["side-omen"]                           # Necromancy of the last target's side

[[step.action]]
currency = "desecrate"                          # the plain bone, if that omen is blocked

[[step]]
id = "reveal"
say = "Reveal it at the Well of Souls"
goal = "reveal"
bind = "last-remaining"
when = "bound"
on_miss = "goto:whittle"

[[step.action]]
currency = "reveal"
```

Keys are snake case (`wanted_at_least`); the values the format enumerates
(goals, bindings, rarities, sides) are kebab case (`add-wanted`). Every
table refuses a key it does not know, so a typo is an error, not a key
silently ignored. A `[[step.action]]` list may also be written inline,
`action = [{ currency = "essence-of-haste" }, { currency = "greater-essence-of-haste" }]`.

## The top-level keys

| Key | Required | What it holds |
|---|---|---|
| `id` | yes | The file name without `.toml`, in lower kebab case. Two files may not share one. |
| `name` | yes | The blueprint's title, for this guide and the people writing blueprints. The craft window does not show it: the route is the app's to follow, and other routes are offered as `Route 1`, `Route 2` and so on. Not empty. |
| `classes` | yes | The item classes the blueprint is for, as the data spells them: `Gloves`, `Boots`, `Helmets`, `Body Armours`, `Shields`, `Quivers`, `Bucklers`, `Foci`, `Wands`, `One Hand Maces`, `Sceptres`, `Spears`, `Bows`, `Staves`, `Two Hand Maces`, `Quarterstaves`, `Crossbows`, `Talismans`, `Belts`, `Amulets`, `Rings`, `Jewels`. At least one. |
| `source` | yes | Where the technique came from: `"Discord post 10, owner's paste, 2026-10-28"`, or `"roadmap"` for the owner's own description. Not empty. |
| `notes` | no | Free text: caveats, and every mechanic the route rests on that the docs mark not established. The craft window does not show it, so these caveats live in the blueprint file and in this guide (§ "The seed blueprints"). |
| `order` | no | `"listed"` (the default) or `"state"` (§ "State-ordered blueprints"). A blueprint without it reads and runs as before 0.1.1. |
| `relies_on` | when `order = "state"` | The not-established facts the route relies on, by their id in the spec's facts table ([`crafting-slots-spec.md`](../models/crafting-slots-spec.md) § "Facts"), such as `greater-with-side-omen`. Required, possibly `[]`, in a state-ordered blueprint; allowed, and checked, in a listed one. An id that is not a not-established fact of that table, or one named twice, is an error. Like `notes`, the craft window does not show it. |
| `[[start]]` | at least one | A start alternative (below). |
| `[[step]]` | at least one | The steps: in order for a listed blueprint, in priority order for a state-ordered one (below). |

A blueprint whose classes name one class is offered before one naming many;
among equals, file order (the file name) decides.

## Start alternatives and conditions

A `[[start]]` table is a predicate on the item; the route may start when any
one of them holds. Every key left out holds. A step's `requires`, `until`
and `guard` take the same keys plus the step-only ones.

| Key | Where | Holds when |
|---|---|---|
| `rarity` | start, step | The item's rarity is one of the list: `normal`, `magic`, `rare`. |
| `wanted_at_least` | start, step | At least n targets are on the item at their floor. |
| `wanted_prefixes_at_least`, `wanted_suffixes_at_least` | start, step | At least n of them are prefixes (suffixes). |
| `fractured_wanted` | start, step | `true`: a fractured modifier is present and every fractured one is a target. `false`: none is fractured. |
| `unwanted_at_most` | start, step | At most n modifiers on the item are not targets. |
| `essence_binds` | start, step | An unblocked essence, alloy or liquid emotion adds one of the targets at its floor on this item. |
| `blocked_side` | start, step | `true`: some side is blocked (below). `false`: none is. |
| `closable_side` | start, step | `true`: some side is closable. |
| `one_side_open` | start, step | `true`: exactly one side is open. |
| `desecrated_unwanted` | start, step | `true`: a revealed desecrated modifier on the item is not a target. |
| `modifiers_at_least`, `modifiers_at_most` | start, step | At least (at most) n explicit modifiers are on the item, fractured, desecrated and unrevealed ones counted (since 0.1.4: a Fracturing Orb wants a rare with four, and `fracture-first` starts from exactly four). |
| `free_prefixes_at_least`, `free_suffixes_at_least` | step only | At least n open prefix (suffix) slots. |
| `no_desecrated` | step only | `true`: no desecrated modifier, revealed or not. |
| `unrevealed` | step only | `true`: an unrevealed desecrated modifier is present. |
| `wanted_remaining_at_most`, `wanted_remaining_exactly` | step only | At most (exactly) n targets are still missing. |

Typical starts: "Rare with two targets", `rarity = ["rare"]` and
`wanted_at_least = 2`; "Rare with three, one fractured", adding
`fractured_wanted = true`; "Magic, and an essence binds",
`rarity = ["magic"]` and `essence_binds = true`. A step-only key in a
`[[start]]` is an error.

Slots: a Magic item has 1 prefix and 1 suffix, a Rare 3 and 3, a Rare jewel
2 and 2; a Contempt liquid emotion's modifier adds one slot on the other
side. The Magic and Rare limits are documented (poe2db's item text; the
spec's facts `magic-slots` and `rare-slots`); the jewel and Contempt ones
are the owner's; none is measured.

The four slot keys read the spec's slot accounting
([`crafting-slots-spec.md`](../models/crafting-slots-spec.md) § "Slot
accounting"), per side: its **free** slots, the targets it still
**needs** (a target waiting on an unrevealed modifier there not counted),
and spare = free - need. A side is **blocked** when it has fewer free
slots than missing targets and holds a modifier a removal may take (a
non-target that is not fractured, desecrated or unrevealed); **reserved**
when every free slot is needed; **closable** when it has a free slot and
needs nothing while the other side needs something; **open** when it has a
free slot. They work in listed blueprints too.

## Steps

| Key | Required | What it holds |
|---|---|---|
| `id` | yes | Unique in the blueprint; `goto:` names it. |
| `say` | yes | The blueprint's own short instruction. The player sees it only where the app does not compose the instruction from the step's data: a `fill`, `remove-desecrated` or `manual` step, a step that places a target when no target is left for it, and a branch line naming a step the route left out. Everywhere else the craft window names the currency, its omens and the modifiers the step places or removes, and for a step that may whittle or annul, which of the two the modifier levels call for ([`README.md`](README.md) § "Overview"; in a state-ordered blueprint, the alternative the engine's removal rule took). Whatever omen the text promises, an action of the step must carry (§ "Actions, ids and placeholders"). |
| `goal` | yes | What the step is for, and so what the engine checks after it (below). |
| `side` | no | `prefix` or `suffix`: the side an add or remove goal is about. A state-ordered step may give a side rule instead: `target`, `close` or `blocked` (§ "State-ordered blueprints"). |
| `bind` | no | Which target the step aims at (below). A `reveal` or `convert` step needs one. |
| `requires` | no, listed only | A condition that must hold before the step is taken. When it does not, the guide says which part failed and stops ("stuck"); it never picks another step by itself. |
| `guard` | no, state-ordered only | A condition: the step is a candidate only while it holds (`guard = { one_side_open = true }`). |
| `when` | no | `"bound"`: the step is part of the route only when its binding resolved (an essence step with no essence for any target is skipped). Needs `bind`. |
| `until` | no, listed only | On a hit, repeat the step until this condition holds (`{ wanted_remaining_at_most = 2 }`: fill until two targets are left). |
| `[[step.action]]` | yes, but a `manual` step may have none | The ways to take the step, preferred first. |
| `on_hit` | no, listed only | Where to go on a hit. Default: `next`, or `repeat` while `until` does not hold. |
| `on_miss` | no, listed only | Where to go on a miss. Default: `repeat`. A step that places a crafted modifier (an essence, alloy or liquid emotion) and is not `manual` must give one other than `repeat` (§ "One crafted modifier"). |

A key the blueprint's order does not take is an error at its own line and
column: `requires`, `until`, `on_hit` and `on_miss` in a state-ordered
blueprint; `guard` and the side rules in a listed one.

In a listed blueprint, a step whose goal is already met on the item, or
whose `when` did not resolve, is skipped, and the guide says so.

### Goals

| Goal | Hit when |
|---|---|
| `add-wanted` | A target the step aims at (on `side`, when given) is now on the item at its floor. |
| `remove-unwanted` | A modifier that is not a target went (from `side`, when given). |
| `fill` | Any modifier was added. |
| `desecrate` | An unrevealed desecrated modifier appeared. |
| `reveal` | The unrevealed modifier was revealed as the bound target at its floor. |
| `remove-desecrated` | The desecrated modifier went. |
| `convert` | A flux turned one modifier into the bound target. |
| `fracture` | A target became fractured (since 0.1.4). Another modifier fractured is a miss. Its actions must fracture: a Fracturing Orb (`fracturing-orb`). |
| `manual` | Always: the player does the step and confirms. No check, no odds. Catalysts, quality, rune sockets and anything the engine cannot read are manual. |

No goal is a hit when a target that was on the item is gone.

### Bindings

Resolved once, when the engine picks the route (a state-ordered
blueprint makes them differently, § "State-ordered blueprints"):

| Binding | The step aims at |
|---|---|
| `essence` | The target an unblocked essence, alloy or liquid among the step's actions adds at its floor or better. Lesser, plain and Greater essences fit a Magic item (they make it Rare); Perfect and the special ones fit a Rare (they remove one modifier first). |
| `hardest` | The target with the lowest odds of being added; unknown odds count as hardest, ties go to the order the targets were given. |
| `any-remaining` | Any target not yet on the item. |
| `last-remaining` | The one target not yet on the item; with two or more missing, whichever comes, as `any-remaining`. In state order it is different: made afresh on every item, only when exactly one target is missing (with two or more, none), and never kept. |

A target from the desecrated pool can only be placed by a `desecrate` or
`reveal` step; a blueprint that has none for it is rejected for that item.

### Transitions

`on_hit` and `on_miss` take one of `next` (the following step), `repeat`
(this step again), `restart` (the first step), `abandon` (give the item
up), `done` (the craft is finished), or `goto:<step id>`. A go-to to a step
the blueprint does not have is an error. An unexpected outcome (below)
never takes a transition: it stops the guide and asks for a re-plan. A
state-ordered blueprint has no transitions: after a hit or a miss the
guide picks the next step from the item again.

## Actions, ids and placeholders

Each `[[step.action]]` is a `currency` id and an optional list of `omens`
ids, to have in the inventory when using it. The route takes the first
action none of whose ids is on the player's blocked list; when every action
of a step is blocked, the blueprint is rejected for that item with that
reason.

**Ids are the English name in lower kebab case, apostrophes dropped**:
`Orb of Annulment` is `orb-of-annulment`, `Hinekora's Lock` is
`hinekoras-lock`, `Perfect Exalted Orb` is `perfect-exalted-orb`. The
blocked list in Settings stores the same ids.

- **Static ids** (currencies, Greater and Perfect currencies, omens, bones,
  fluxes, catalysts), each with what it does: `ch-blueprint`'s
  [`catalogue.rs`](../../apps/crafting-helper/crates/ch-blueprint/src/catalogue.rs),
  built from [`crafting-mechanics.md`](crafting-mechanics.md).
- **Data-backed ids** (essences, alloys included, and liquid emotions) come
  from the modifier data's `name`: `essence-of-haste`,
  `greater-essence-of-the-body`, `essence-of-horror`, `transcendent-alloy`,
  `ancient-potent-liquid-contempt`. One must exist for
  at least one of the blueprint's classes. The full list, static and
  data-backed together, is `ch_engine::catalogue()` (about 220 ids on
  2026-10-28's data).

An omen must modify the action's currency (Whittling goes with a Chaos Orb,
not an Exalted Orb), and one action may not carry both a Sinistral and a
Dextral omen. A goal the action cannot produce is an error (an Orb of
Annulment, which only removes, for an `add-wanted` step).

**The `say` and omen check**, in every blueprint: a step's `say` that names
an omen (by its English name, or by the name without "Omen of" when two or
more words are left, such as "Dextral Exaltation") needs an action of the
step that carries that omen, and a `say` that mentions an omen at all (the
word "omen" too) needs an action that carries one. `side-omen` counts as
its currency's Sinistral and Dextral omens, `greater-omen` as Omen of
Greater Exaltation. It catches a step whose text says "with an Exaltation
omen" while its action has none, which is how the retired
`fill-desecrate-exalt` came to offer a plain Exalted Orb.

**A sided desecration names its Necromancy omen** (since 0.2.0): a
`desecrate` step with a `side`, a side rule or a `bind` needs `side-omen`,
or a Necromancy omen by name, on its first `[[step.action]]`, so the
desecrated modifier lands on the side the step means and the instruction
can say which (`Use Omen of Dextral Necromancy, then apply the Preserved
Collarbone; the desecrated modifier will be a suffix, for …`). A later
alternative may be the plain bone, taken when the omen is blocked, with a
fallback line naming it. A desecration tied to neither (a blocker on any
side) needs no omen. `rings-resist-whittle`'s desecration was the one
shipped step this refused.

### One crafted modifier

Since 0.5 an item holds at most one crafted modifier
([`crafting-slots-spec.md`](../models/crafting-slots-spec.md) § "Facts",
`one-crafted-one-desecrated`, documented). Every essence grade, the
special essences (Hysteria, Horror, the Abyss, the Breach and the rest),
every liquid emotion and every alloy places one; an alloy counts too
(`alloy-counts-as-crafted`, documented in the 0.5.0 patch notes). A step
places one when an action of it names such a currency; a `manual` step
with no action does not. Since 0.1.4 the check holds every blueprint to
it, in either order:

- **Two crafted steps need a removal between them**: a second step that
  places a crafted modifier, with no step between the two in file order
  whose actions include an Orb of Annulment or a Chaos Orb of any grade,
  is an error at the second step's currency. This is a plausibility rule,
  not a proof: the removal between them may take another modifier and
  leave the first crafted one where it is.
- **A crafted step does not repeat** (listed order): a step that places a
  crafted modifier and is not `manual` may not have `until`, and its miss
  may not go to `repeat`, written or by default. So a listed essence step
  says where a miss goes (`on_miss = "next"`, `"abandon"`, a `goto:`): the
  second use would find the first one's modifier on the item.

The rule is checked on the blueprints, not on the item: the app cannot
tell whether an item already carries a crafted modifier from before the
craft (the copied text's crafted marker is not measured).

### Jewels and Greater or Perfect currencies

A jewel's modifiers have one level each, so a currency with a Minimum
Modifier Level (the Greater and Perfect orbs) only costs more on one.
Since 0.1.4 a blueprint whose every class is `Jewels` may not name one (an
error at the currency), and on any blueprint the route never offers one
on a jewel: the step takes its next alternative, the reason in words
(`` `perfect-exalted-orb` has a minimum modifier level, which only adds
cost on a jewel ``). A blueprint for rings and jewels keeps the Perfect
Exalted Orb on a ring and offers the plain one on a jewel.

### Placeholders

Four placeholders stand for a choice the engine makes on the item:

| Placeholder | Where | Means |
|---|---|---|
| `desecrate` | `currency` | The bones that fit the item's class and level, tried Preserved, Ancient, Gnawed (Gnawed only up to item level 64). |
| `reveal` | `currency` | The reveal at the Well of Souls. It names no crafting item, so nothing blocks it. |
| `side-omen` | `omens` | The Sinistral or Dextral omen of the action's currency for the bound target's side (or the step's `side`, or the side its side rule resolved): Sinistral for a prefix, Dextral for a suffix. Needs a `bind` or a `side`, and a currency that has side omens. A listed step with `when = "bound"`, `bind = "last-remaining"` and no `side`, planned while several targets are missing, takes the side they all share; with targets on both sides its omen waits for the last target's side (the preview names both omens, and either one blocked passes the alternative over), and the guide names the one omen once a single target is left. |
| `greater-omen` | `omens` | Omen of Greater Exaltation, on an Exalted Orb of any grade only, where the step's side (the item, with no side) has two free slots and the step can use two (an add: two missing targets it can place there; a fill: always), and the omen is not blocked; otherwise nothing, with no fallback line. It may sit beside `side-omen`, and is an error beside an explicit `omen-of-greater-exaltation`. Whether Greater with a side omen adds both to that side, and whether Greater needs two free slots, are not established (`greater-with-side-omen`, `greater-needs-two-free`). |

Blocking applies to what a placeholder resolves to: block
`preserved-jawbone` and the next bones that fit are offered.

## State-ordered blueprints

A blueprint with `order = "state"` lists its steps in priority order, each
with an optional `guard`, and has no transitions. The rules are the spec's
([`crafting-slots-spec.md`](../models/crafting-slots-spec.md) § "Route
rules"); the stdlib model and `ch-engine` both follow them, held together
by one case file (`apps/crafting-helper/crates/ch-engine/tests/items/slots/cases.json`).

**The next step** is the first step, in file order, that is a candidate on
the item as it stands: its `guard` holds, its `when` resolves, its goal is
not yet met, its side rule finds a side, an alternative fits, and there is
room for it (a free slot where an add, a fill or an essence lands; a free
slot on a desecration's side, so a bone never takes a random modifier off a
full item). With no candidate the craft is done when no target is missing,
and otherwise stuck, with every step's reason. The guide picks again after
every hit and every miss, so the same step may come several times.
Bindings (`essence`, `hardest`) are made once, as soon as the item allows,
and kept for the craft; an `any-remaining` add leaves out the target the
route's essence binding claims. `last-remaining` (since 0.1.2) is made
afresh on every item: the one missing target when exactly one is missing,
otherwise none, so a `when = "bound"` step on it is no candidate. Kept like
`hardest`, a miss that left a second target missing would keep a
desecration bound to a target that is no longer the last. An unrevealed
modifier on the item waits for the target of the route's desecration that
is still missing, `hardest` or `last-remaining`, in file order.

**The last-target desecration** (since 0.1.2, issue #32; the spec's
§ "The last-target desecration"). On a rare with no desecrated modifier
(an unrevealed one included) and exactly one target missing, each
state-ordered blueprint places that target by a desecration on its side
and a reveal, not an exalt or an essence. It is three ordinary steps, all bound to
`last-remaining` with `when = "bound"`, so the guide, outcomes and step ids
work as for any step:

- `reveal-last` (goal `reveal`): the reveal with Omen of Abyssal Echoes,
  then a plain reveal;
- `light-last` (goal `remove-desecrated`, guard `desecrated_unwanted =
  true`): Omen of Light with an Orb of Annulment, for a reveal that
  offered no target;
- `desecrate-last` (goal `desecrate`, `side = "target"`, guard rarity
  rare): one alternative, the bone with `side-omen` (Sinistral or Dextral
  Necromancy). A plain bone without the omen could land on the other
  side, so it is never the fallback.

`reveal-last` and `light-last` come before the `remove` step and
`desecrate-last` right after it, before `close`, `slam-open-side` and
`side-slam`; in `magic-augment-essence` and `weapons-physical-generic`
they are in the rare phase. `essence-desecrate-double-slam` adds only
`desecrate-last`, after its own `desecrate`: its `reveal` and `light`
already serve it, and its `hardest` desecration is unchanged (a route that
desecrated for the hardest target has its one desecrated modifier, so the
rule cannot apply after it). `body-armours-single-prefix` (0.1.4) has
only `desecrate-last` too, after its `remove` step, with its own `reveal`
and `light` earlier in the file. An
item with a revealed unwanted desecrated modifier and one target missing
gets `light-last`, then `desecrate-last`.

**The pool condition.** `desecrate-last` is a candidate only when the
item's class has at least one desecrated-section modifier on the target's
side at the item's level, the list the craft window's Desecrated group
shows. Any target on that side then qualifies, normal-pool ones included,
since a reveal may offer ordinary modifiers (`reveal-offers-base`). In the
shipped data every equipment desecrated modifier is level 65; body
armours, boots, gloves, helmets, shields and bucklers have no desecrated
prefix, sceptres none; jewels' are level 1. So the rule passes over an
armour prefix, any sceptre target and any item below level 65, and never
relies on `necromancy-without-pool`.

**Its Fallback line.** When the rule's conditions hold but
`desecrate-last` is no candidate, the step the engine takes after it
carries why (`StepView::instead_of_desecration`), and the app shows it as
that step's Fallback line, `<used, with its omens> instead of a
desecration: <reason>`: `no desecrated <side> modifier on <class> at item
level <n>`, `<names> blocked` (the class's bones, the Necromancy omen), or
the engine's own reason ([`ui-design.md`](ui-design.md) § "Rust-produced
copy"). The steps before it (a reveal, Omen of Light, a removal) clear the
way for it and carry no such line. The odds of both steps are "odds
unknown": desecration and reveals have no established weights.

**Side rules** (`side = ...`):

| Rule | The side |
|---|---|
| `target` | The bound target's side; unbound, the open side with the most missing targets the step can place there, ties to the side of the first such target in the target list. |
| `close` | The closable side. |
| `blocked` | The blocked side; with two, the larger shortfall first, ties as for `target`. |

**No unrestricted exalt next to a reserved side.** While both sides are
open and one is reserved, an Exalted Orb of any grade with no side omen
does not fit: it could take the slot the reserved side's target needs, so
the step falls to its next alternative.

**Removals.** A `remove-unwanted` step (usually `side = "blocked"`) tries
its alternatives in order: Chaos Orb with Omen of Whittling only when the
whittle is certain to take a non-target on that side (exactly one
modifier at the lowest modifier level, every level known) and the
annulment it would otherwise take could take a target; an Orb of
Annulment with `side-omen` (Sinistral or Dextral Annulment) only when the
other side holds a target; otherwise a plain annulment. A Magic item's
removal has no whittle. A `side = "blocked"` removal is a hit when a
non-target went from a side that was blocked.

**Certain removals first** (since 0.1.4; the spec's § "Certain removals
first"). The next step is the first candidate in file order, unless that
candidate is not a certain removal and a later candidate is a
`remove-unwanted` step whose chosen alternative is certain: every
modifier it may take is a non-target, or its whittle is certain. Then the
first such later candidate goes first, so a removal that cannot cost a
target is never left behind a gamble. Only removals reorder: the
certainty of an add stays in its odds. The preview takes the same pick.

**Essence swaps** (since 0.1.4, engine-only: the slot model has no
Perfect essence or Crystallisation omen). A Perfect or special essence
(Hysteria, Delirium, Horror, Insanity, the Abyss, the Breach) on a rare
removes a modifier, like a Chaos Orb, and adds its own. An `add-wanted`
step whose chosen alternative is one fits when its `essence` binding
names a missing target the essence adds at its floor (the binding is made
on a rare as on a magic item for such a step), the removal's side holds a
modifier a removal may take, and the target's side has room once it has.
The removal's side is the Crystallisation omen's; with none, the side the
essence's modifier belongs to when that side is full
(`essence-full-side-removal`, not established); otherwise either side.
The preview takes the first removable non-target on that side and places
the target, and a swap whose removal is certain counts as a certain
removal above. The engine's `tests/rules.rs` pins the rule over test
blueprints; since 0.2.0 the shipped `fractured-essence-swap` takes one
(`tests/essence_swap.rs`). When the removal may take an unrevealed
desecrated modifier (owner-stated, not established), the step also
carries that modifier's survival, (n-1) in n over the n modifiers the
removal may take, and the step the route takes next if it stays and if it
goes (spec, "The essence swap and an unrevealed modifier"). Alloys and liquid
emotions swap too, but stay listed-order only.

**The preview** is the route the craft takes if each step hits, from the
item as it stands: an add places its target (two with Greater), a fill
adds fillers to its side, an essence makes the Magic item Rare with its
target, a desecration leaves an unrevealed modifier waiting for its
target, a reveal places it, and a removal takes the non-target its action
would (a whittle's added modifier is not projected). It stops when no
target is missing. A step may appear twice. The first step's odds are read
off the item; a later step's off the projected item, saying "if the
earlier steps hit".

**Rejections**, after the class and the start: a side whose targets
alone exceed the item's limit as a rare, three unless a line on the item
changes it (`TargetsExceedSlots`), a desecrated target and
no desecrate step, and a route whose hit path cannot finish
(`RouteCannotFinish`: no candidate, a step whose hit changes nothing, or
16 steps without finishing). Last, in either order since 0.1.4,
**`weak-magic-start`** (`WeakMagicStart`): the item is magic, its prefix
is a target (at any tier), a step of the route makes it rare while it is
magic (a Regal Orb of any grade, or a Lesser, plain or Greater essence),
and the prefix's tier is worse than the second-best tier its modifier
rolls on this base at the item's level. The guides send such an item back
for a new base rather than spend an upgrade on it, and the rejection says
so, naming the modifier by its data id: ``the magic item's prefix
`prefix-increased-movement-speed` is T3, below its top tiers at item
level 82 (T1 and T2): a new base is worth more than an upgrade`` (boots'
movement speed reaches T1 at item level 82 and T2 at 65). It does not
apply with no item level, an unknown tier, or a prefix that is not a
target; with one reachable tier, that tier is the top two.

**What the guide does.** The craft window numbers a state-ordered route's
steps in the order this craft takes them; its hit line names the next step
of the preview (or says the craft is done, or, with no preview to read,
that the next step is picked from the item), and its miss line always says
the next step is picked from the item ([`ui-design.md`](ui-design.md)
§ "Rust-produced copy"). The engine's pick is final: the app does not re-decide a whittle or
an annulment by the modifier levels, as it does for a listed blueprint.
Outcomes (hit, miss, unexpected) classify as in listed order.

## Checking a blueprint: `check_blueprints`

```sh
cargo run --manifest-path apps/crafting-helper/Cargo.toml --locked -p ch-engine --example check_blueprints -- apps/crafting-helper/blueprints
```

It takes files or folders (a folder means its `.toml` files, sorted by
name), runs the format's checks (`ch-blueprint`) and the data-backed ones
(`ch-engine`: every essence, alloy and liquid id exists for one of the
blueprint's classes), and refuses two files with one `id`. It prints one
`<path>: ok` line per valid file and one line per error, every error in a
file and not just the first:

```text
<path>:<line>:<col>: <message>
```

Line and column count from 1 and point at the value at fault (a syntax
error at the parser's position). For example:

```text
.../many-errors.toml:1:6: id `not-the-stem` is not the file name `many-errors`
.../many-errors.toml:20:6: duplicate step id `slam` (first on line 10)
.../omen-must-fit.toml:17:10: step `slam`: `omen-of-whittling` does not modify `exalted-orb` (it modifies chaos-orb, greater-chaos-orb, perfect-chaos-orb)
```

Since 0.1.4 it also refuses, each at the value at fault:

- **crafted-twice**: a second step that places a crafted modifier with no
  annulment or Chaos Orb step between (§ "One crafted modifier"), at its
  currency: ``step `<id>`: `<currency>` places a second crafted modifier
  after step `<id>`, with no annulment or Chaos Orb between them; an item
  holds one crafted modifier``;
- **crafted-repeat**: a listed crafted step that is not `manual` with an
  `until`, or whose miss repeats (at its `on_miss`, or at its currency
  when the repeat is the default): ``step `<id>`: a miss repeats by
  default a step that places a crafted modifier; an item holds one, so
  say where a miss goes (`on_miss`)``;
- **jewel-grades**: a Greater or Perfect currency on a blueprint for
  `Jewels` alone (§ "Jewels and Greater or Perfect currencies");
- a `fracture` step whose action cannot fracture, through the existing
  check of a goal the action cannot produce;
- a `relies_on` id outside the spec's table, which since 0.1.4 also
  holds `abyss-mark-unfracturable`, `desecrated-unfracturable`,
  `full-desecration-same-side` and `essence-full-side-removal`
  (§ "The seed blueprints").

The test blueprints under `ch-blueprint`'s `tests/fixtures/invalid/`
(`crafted-twice.toml`, `crafted-repeat.toml`, `jewel-grades.toml`,
`fracture-currency.toml`) pin each line and column.

It exits 0 when every file is valid, 1 on any error and 2 with no argument.
The shipped blueprints are embedded in the app at build time, so a new or
changed file also needs `cargo test --manifest-path
apps/crafting-helper/Cargo.toml --locked -p ch-engine --test shipped`
(`every_shipped_blueprint_validates_and_resolves`).

## Outcomes: hit, miss and unexpected

After each step you copy the item again; the engine compares the item
before and after, on their modifiers joined to the data, and says one of:

- **Hit**: the step's goal is met (table above) and no target was lost. The
  guide takes `on_hit`.
- **Miss**: the change fits what the currency does, but the goal is not
  met. A target that came in below its floor is a miss, and counts as
  unwanted for a later removal. The guide takes `on_miss`.
- **Unexpected**: the change is not one the step's currency can make. The
  guide stops and asks you to plan again. That is when the item's class,
  base type or item level changed; it became corrupted, mirrored or
  unidentified; a fractured modifier changed; a new modifier does not join
  the data (an essence's, alloy's or liquid's own modifier excepted); the
  number of modifiers added and removed, or the rarity change, does not fit
  the currency (an Exalted Orb adds one, two with Omen of Greater
  Exaltation; a Chaos Orb, Perfect essence, liquid or flux removes one and
  adds one; an annulment removes one, two with Omen of Greater Annulment;
  a Regal Orb or a Lesser, plain or Greater essence makes a Magic item Rare
  with one more; a desecration adds one unrevealed modifier; a reveal turns
  the unrevealed one into a desecrated one on the same side; a Fracturing
  Orb fractures one modifier and changes nothing else); or a side
  omen's side is not the side that changed. After a `manual` step only the
  first group counts (class, base type, item level, corrupted, mirrored,
  unidentified): any change to the modifiers fits it.

Every outcome lists the targets gained, the targets lost, and the targets
that came in below their floor.

**Odds** are one step's rough chance, never a simulation: a known percent
with its basis ("8000 of 69500 weight"), or `unknown` with the reason.
Unknown for every jewel (no spawn weights), for a desecration and a reveal
(no established weights; a reveal may offer normal-pool modifiers too), for
a target whose weight is the site's placeholder 1, for a flux, and for a
whittle when two modifiers tie for the lowest level. An essence, alloy or
liquid's add is certain.

Since 0.1.4:

- **A certain add shows as certain, weights or not.** When an add's
  exclusions (the family already on the item, the side omen, the Minimum
  Modifier Level) leave the target alone in the pool and every one of its
  tiers counts at the floor, the step is certain to hit; when they leave
  the target out and a weight is missing, it is certain not to. So an
  isolation reads 100% on a jewel or a placeholder weight too.
- **An essence swap's odds** (a Perfect or special essence on a rare) are
  its certain add times its removal's share of non-targets, over the
  removal's side (§ "State-ordered blueprints", "Essence swaps"), and
  100% when every modifier the removal may take is a non-target.
- **A Fracturing Orb's odds** are the share of targets among the
  modifiers it may fracture: the explicit ones that are not fractured, not
  desecrated (revealed or not) and not the Mark of the Abyssal Lord
  (Essence of the Abyss's modifier). Leaving the last two out rests on
  `desecrated-unfracturable` and `abyss-mark-unfracturable`, not
  established, and the basis says so; each candidate is taken as equally
  likely. One target among three such modifiers is one in three.
- **A desecration on a full side** (listed order; state order never takes
  one) shows the odds of its removal: the share of non-targets among the
  side's modifiers a removal may take, resting on
  `full-desecration-same-side` and `removal-equally-likely`, and not
  counting the reveal's own chance.

## The seed blueprints

Seventeen blueprints ship, each a paraphrase in our own words of the
source it names, never a sentence of it:

- four state-ordered ones from published 0.5 crafting guides (their URLs
  and "fetched 2026-10-29" are in each `source`);
- eight listed ones from the owner's Discord paste of 2026-10-28; see
  [`crafting-mechanics.md`](crafting-mechanics.md) § "Crafting patterns
  seen";
- since 0.1.4 (issue #35), four from the Mobalytics and maxroll crafting
  guides: two state-ordered, `weapons-physical-generic` and
  `body-armours-single-prefix`, which start from a normal or magic item,
  and two listed, `fracture-first` and `full-rare-desecration-rescue`,
  which start from a rare. The Mobalytics pages' URLs and "fetched
  2026-10-30" are in each `source`; maxroll's page is "reviewed
  2026-10-30 (issue #35)";
- since 0.2.0, one state-ordered from the owner's statement of 2026-10-30,
  `fractured-essence-swap`, which starts from a rare whose full side holds
  a fractured modifier.

Each is walked on a hit path and a miss path in `ch-engine`'s tests
(`tests/seeds_v2.rs` for the first state-ordered four,
`tests/seeds_guides.rs` for the four from the guides, with a third walk
each, `<id>_start_refuses`, for the item it turns away;
`tests/seeds_gear.rs` and `tests/seeds_jewels_weapons.rs` for the rest).
The first four state-ordered ones are also held to the slot model on the
spec's representative crafts by the parity test (`tests/slots.rs`, over
`tests/items/slots/cases.json`). The two state-ordered guide blueprints
are not in the model's cases: they use only rules the cases already pin,
and `tests/seeds_guides.rs` pins them (the spec's § "Not covered");
`fractured-essence-swap` is not in them either, and `tests/essence_swap.rs`
plans it on a ring, a quiver and a Dusk Ring.

The first state-ordered four cover every equipment class with spawn
weights (not jewels). None names a Homogenising omen, Omen of Greater
Annulment, an Erasure omen, a Perfect essence, an alloy or a Fracturing
Orb. Since 0.1.2 each state-ordered blueprint also places the last missing
target on a rare with no desecrated modifier by a desecration and a reveal
(§ "State-ordered blueprints", "The last-target desecration"), which the
table below leaves out.

Every shipped blueprint places at most one crafted modifier
(§ "One crafted modifier"). The audit of 2026-10-30 found two that did
not: `boots-runic-ward` lost its Essence of Horror step (below) and
`weapons-alloy-budget` was retired; `gloves-crit-melee` no longer
repeats its Essence of Hysteria on a miss.

| id | source | classes | the route |
|---|---|---|---|
| `close-a-side` | maxroll's "How to Craft", method 3; the domistae codex | every equipment class with spawn weights | From a rare with a closable side, or with one side open: remove a blocker (a certain whittle, else an annulment, side-restricted when the other side holds a target); close the finished side with cheap side-restricted exalts (two at a time with Greater Exaltation where two slots are free); slam the one open side with no side omen; otherwise a side-restricted slam per target. |
| `essence-desecrate-double-slam` | timesaver's 0.5 guide; the domistae codex; maxroll and Mobalytics (methods 2 and 4) | every equipment class with spawn weights | From a magic item an essence binds on, or a rare: the essence; a desecration for the hardest target on its side (Sinistral or Dextral Necromancy), revealed with Abyssal Echoes; a wrong reveal taken off with Omen of Light; a removal when a side is blocked; then side-restricted Perfect exalts, with Greater Exaltation where two fit. |
| `magic-augment-essence` | maxroll's "How to Craft", method 1 | every equipment class with spawn weights | From a normal or magic item: transmute; annul a wrong modifier on the magic item (side-restricted when the other side holds a target); augment; an essence that makes it rare with a third target; then removals and side-restricted slams. |
| `rare-clean-then-slam` | issue #28; maxroll and the domistae codex (method 2's clean-up, method 3's side restriction) | every equipment class with spawn weights | From a rare with any number of targets, a dropped one included: clean the blocked side (a certain whittle, else an annulment, side-restricted when the other side holds a target) until each side has a free slot per missing target, then side-restricted slams. |
| `rings-resist-whittle` | post 10 | Rings | Chaos Orbs, then whittling until one target is left, then a desecration for the last, with the Necromancy omen of its side (since 0.2.0; the plain bone when that omen is blocked). |
| `rings-flat-attack` | post 11 | Rings | Attack quality raised first (manual), flat attack damage prefixes over resistance suffixes. |
| `weapons-physical-generic` | Mobalytics' SSF crafting guide, "Generic Phys Weapon" | Bows, Crossbows, One Hand Maces, Quarterstaves, Spears, Talismans, Two Hand Maces (the classes with both increased Physical Damage and a Greater Essence of Abrasion for flat physical damage) | State-ordered, from a normal or magic weapon: transmute and augment (Perfect first), annulling a wrong modifier, until the increased Physical Damage prefix is a top-two tier (a weaker one is sent back, `weak-magic-start`); Greater Essence of Abrasion as it turns rare, its one crafted modifier; a Greater, then plain, Exalted Orb with Omen of Greater Exaltation and the side omen for two suffixes; the last target by desecration. The guide's closing Perfect Essence of Battle would be a second crafted modifier and is left out. A weapon whose targets give Abrasion nothing to add cannot finish here; `magic-augment-essence` takes any essence. |
| `body-armours-single-prefix` | Mobalytics' Lightning Arrow crafting guide, the body armour, without Homogenising | Body Armours | State-ordered, from a normal or magic armour: a top single-stat defence prefix by transmute and augment (a weaker one is sent back, `weak-magic-start`); a Greater Essence of Insulation, Thawing or Grounding; a desecration on the hardest target's side (the suffixes), left unrevealed; with only the prefixes open, a Greater, then plain, Exalted Orb with Omen of Greater Exaltation and no side omen lands two prefixes (the isolation); the reveal, Omen of Light for a wrong one, a removal, the last target's desecration and side slams. |
| `fractured-essence-swap` | the owner's statement of 2026-10-30 | Amulets, Belts, Body Armours, Boots, Bucklers, Foci, Gloves, Helmets, Quivers, Rings, Shields (the classes with an Essence of Hysteria) | State-ordered, from a rare whose side is full and holds a fractured modifier: Essence of Hysteria adds its target to that side, removing one of the side's other modifiers (never the fractured one), so the add is certain when only throwaways are left to take; a Dusk or Gloam Ring needs only one. When an unrevealed desecrated modifier is there, the step gives its survival ((n-1) in n) and the next step on each outcome: if it stays, the reveal (Abyssal Echoes; Omen of Light for a wrong one); if it goes, an annulment confined to the full side (Sinistral or Dextral Annulment; it may take the essence's modifier, and the route then plans again), the last target's desecration with its side's Necromancy omen, and the reveal. |
| `fracture-first` | Mobalytics' SSF crafting guide (Fracturing Orb with Essence of the Abyss) and its Fracturing Orbs guide | every equipment class with spawn weights | Listed, from a rare with exactly four modifiers, none fractured, at least one a target: Essence of the Abyss (losing a target to it gives the item up), so the Fracturing Orb picks among three; the orb (fracturing anything else gives it up); annulments down to the fractured target and one more; Chaos Orbs until a second target; a desecration for the hardest target on its side, revealed with Abyssal Echoes, a wrong reveal lit off and the bone used again; side-restricted Perfect exalts. The guides' Crystallisation omen to guard the target, and their closing filler and Perfect essence, are left out (§ "What a blueprint cannot say"). |
| `full-rare-desecration-rescue` | maxroll's "How to Craft", its rescue of boots with no movement speed | every equipment class with spawn weights | Listed, from a rare with six modifiers and exactly one target missing, no desecrated modifier: a bone with the missing target's side Necromancy omen takes a random modifier off that full side (losing a target gives the item up); the reveal with Abyssal Echoes; a wrong reveal taken off with Omen of Light and the bone used again. |
| `gloves-crit-melee` | post 9 | Gloves | Critical damage from Essence of Hysteria, used once (a miss goes on: an item holds one crafted modifier), a desecrated prefix that blocks a common family while the prefixes are slammed, then desecrated suffixes until attack speed. |
| `amulets-resist-spirit` | post 14 | Amulets | Resistances turned into the wanted elements with fluxes, a desecrated blocker suffix for two Perfect Exalted Orbs, then a desecrated Spirit modifier. |
| `jewels-three-one-side` | post 6 (post 5's note in `notes`) | Jewels | Three targets on one side, the third from a desecration, then the other side. |
| `time-lost-jewel-radius` | post 3 | Jewels | Three suffixes, a desecrated prefix and the Very Large radius (manual) on a Time-Lost jewel. |
| `caster-cast-speed` | post 13 | Wands, Staves, Foci | Cast speed from the Transcendent Alloy, a desecration holding a slot while a Perfect Exalted Orb adds two, then the last desecration. |
| `boots-runic-ward` | post 8 | Boots | From fractured movement speed: Runic Ward (the Sovereign Alloy over a filler prefix, manual), both attributes by Chaos Orb and desecration, the runes (manual). Since 0.1.4 without the post's Essence of Horror (augment effect) and its filler: the alloy is the item's one crafted modifier. |

Steps that rest on a mechanic the docs mark not established (an alloy with
the Crystallisation omens, how a flux maps one resistance's tier to
another's, catalyst quality, rune sockets, which side a Contempt modifier
lands on) are kept, and their catalyst and rune steps are `manual`. Each
blueprint's `notes` names what it rests on, but the craft window does not
show `notes`: the caveats live in the blueprint file and in this guide.

What each blueprint relies on (`relies_on`; each fact is labelled not
established in the spec's facts table, which names its source, and its
§ "Which blueprint relies on which not-established fact" is the same
list). "The first four" are `close-a-side`,
`essence-desecrate-double-slam`, `magic-augment-essence` and
`rare-clean-then-slam`:

| Fact | What the route assumes | Relied on by |
|---|---|---|
| `greater-with-side-omen` | Greater Exaltation with a Sinistral or Dextral Exaltation omen adds two modifiers, both on that side. | the first four, `weapons-physical-generic` |
| `greater-needs-two-free` | Greater Exaltation needs two free slots (on the restricted side, with a side omen). | the first four, `weapons-physical-generic`, `body-armours-single-prefix` |
| `open-side-landing` | An exalt or augment with no side omen, on an item whose other side is full, lands on the open side. | `close-a-side`, `magic-augment-essence`, `weapons-physical-generic` (its augment), `body-armours-single-prefix` (its isolating Greater exalt) |
| `whittling-mod-level` | Whittling takes the modifier with the lowest modifier level. | the first four |
| `fractured-not-removed` | A fractured modifier is never taken by an annulment or a Chaos Orb. | the first four, `fracture-first`, `fractured-essence-swap` (its essence's removal too) |
| `omens-active-before-use` | A Necromancy omen must be active before the bone, Abyssal Echoes before the reveal. | all four state-ordered ones of 0.1.1 (the other three since 0.1.2's last-target desecration), the four of 0.1.4, and `fractured-essence-swap` |
| `necromancy-without-pool` | What Sinistral Necromancy does on a class with no desecrated prefixes. | `essence-desecrate-double-slam`, `full-rare-desecration-rescue` |
| `removal-equally-likely` | Each modifier a random removal may take is equally likely to go. | `full-rare-desecration-rescue`, `fractured-essence-swap` (the unrevealed modifier's survival and the annulment's odds) |
| `abyss-mark-unfracturable` | A Fracturing Orb never picks the Mark of the Abyssal Lord (Essence of the Abyss's modifier). | `fracture-first` |
| `desecrated-unfracturable` | A Fracturing Orb never picks a desecrated modifier, revealed or not. | `fracture-first` |
| `full-desecration-same-side` | A desecration with a side's Necromancy omen, on an item whose side is full, removes its random modifier from that side. | `full-rare-desecration-rescue` |
| `essence-full-side-removal` | A Perfect or corrupted essence with no Crystallisation omen, whose modifier's side is full, removes from that side. | `fractured-essence-swap` |

`ch-blueprint`'s list of these ids is read against the spec's table by a
test, so a fact relabelled there means editing the list, and a blueprint
naming an id that is no longer not established fails its check.

**Retired in 0.1.1**: the roadmap's fill, desecrate, slam route
(`fill-desecrate-exalt`) no longer ships. Its fill step's text promised a
side omen its action did not carry, so on a rare with two target prefixes
and one target suffix it offered a plain Exalted Orb that could take the
prefix slot the last target needed, and it refused a rare with one target
(issue #28). `close-a-side` and `rare-clean-then-slam` cover those crafts.
Its file stays in `ch-engine`'s tests
(`tests/items/gear/fill-desecrate-exalt/`) as the listed-order baseline.

**Retired in 0.1.4**: post 12's alloy budget for weapons
(`weapons-alloy-budget`) no longer ships. Its premise was two alloys on
one weapon, the Celestial and the Transcendent, with no removal between
them, and since 0.5 an item holds one crafted modifier, an alloy's
included. `weapons-physical-generic` and `caster-cast-speed` cover its
classes. Its file, walks and items were deleted; nothing tests its
shape.

Posts not made into a blueprint:

- **Post 1**: names no item kind; it is a cost-saving trick, not a route.
- **Post 2**: its goal is catalyst quality, which the engine cannot read.
- **Post 4**: no steps; a corruption gamble, out of scope.
- **Post 5**: folded into `jewels-three-one-side` (its prefix-side note is
  in that blueprint's `notes`).
- **Post 7**: no craft.

## What a blueprint cannot say

- Branching into another blueprint: a route that should change technique
  stops and plans again.
- Corruption, uniques, rune and soul-core socketing, catalyst quality and
  quality currencies, except as a `manual` step.
- Costs, expected numbers of tries or odds beyond the next step.
- A rarity skip in listed order: a listed step is skipped only when its
  goal is met or its `when = "bound"` binding did not resolve, never
  because the item is already magic or rare (`guard` is state order's).
  A route that starts from a normal or magic item is state-ordered for
  that reason, as `weapons-physical-generic` and
  `body-armours-single-prefix` are.
- A side taken from another step's binding: `side-omen` resolves only
  from the step's own bound target, its own `side`, or (a bound-only
  `last-remaining` step) the targets it may itself place, and every
  binding aims at a missing target. So no step can name the side of a target
  already on the item, which is how the guides keep a Perfect essence off
  the target they want to fracture (the other side's Crystallisation
  omen); `fracture-first` goes without it.
- A Perfect essence's modifier as a target: in the shipped data no
  Perfect essence matches a pool modifier, so an `essence` binding can
  never aim one at a target (only the special Essence of Hysteria matches
  one, in 12 classes).
- An item's crafted modifier from before the craft: one crafted modifier
  per item is checked on the blueprint's steps, not read from the item.
