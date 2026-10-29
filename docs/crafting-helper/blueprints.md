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
| `on_miss` | no, listed only | Where to go on a miss. Default: `repeat`. |

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
| `manual` | Always: the player does the step and confirms. No check, no odds. Catalysts, quality, rune sockets and anything the engine cannot read are manual. |

No goal is a hit when a target that was on the item is gone.

### Bindings

Resolved once, when the engine picks the route:

| Binding | The step aims at |
|---|---|
| `essence` | The target an unblocked essence, alloy or liquid among the step's actions adds at its floor or better. Lesser, plain and Greater essences fit a Magic item (they make it Rare); Perfect and the special ones fit a Rare (they remove one modifier first). |
| `hardest` | The target with the lowest odds of being added; unknown odds count as hardest, ties go to the order the targets were given. |
| `any-remaining` | Any target not yet on the item. |
| `last-remaining` | The one target not yet on the item. |

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

Four placeholders stand for a choice the engine makes on the item:

| Placeholder | Where | Means |
|---|---|---|
| `desecrate` | `currency` | The bones that fit the item's class and level, tried Preserved, Ancient, Gnawed (Gnawed only up to item level 64). |
| `reveal` | `currency` | The reveal at the Well of Souls. It names no crafting item, so nothing blocks it. |
| `side-omen` | `omens` | The Sinistral or Dextral omen of the action's currency for the bound target's side (or the step's `side`, or the side its side rule resolved): Sinistral for a prefix, Dextral for a suffix. Needs a `bind` or a `side`, and a currency that has side omens. |
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
route's essence binding claims.

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
alone exceed a Rare's three (`TargetsExceedSlots`), a desecrated target and
no desecrate step, and a route whose hit path cannot finish
(`RouteCannotFinish`: no candidate, a step whose hit changes nothing, or
16 steps without finishing).

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
  the unrevealed one into a desecrated one on the same side); or a side
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

## The seed blueprints

Thirteen blueprints ship, each a paraphrase in our own words of the source
it names: four state-ordered ones from published 0.5 crafting guides
(their URLs and "fetched 2026-10-29" are in each `source`), and nine
listed ones from the owner's Discord paste of 2026-10-28; see
[`crafting-mechanics.md`](crafting-mechanics.md) § "Crafting patterns
seen". Each is walked on a hit path and a miss path in `ch-engine`'s tests
(`tests/seeds_v2.rs` for the state-ordered four, `tests/seeds_gear.rs` and
`tests/seeds_jewels_weapons.rs` for the rest), and the four state-ordered
ones are held to the slot model on the spec's representative crafts by the
parity test (`tests/slots.rs`, over `tests/items/slots/cases.json`).

The state-ordered four cover every equipment class with spawn weights
(not jewels). None names a Homogenising omen, Omen of Greater Annulment,
an Erasure omen, a Perfect essence, an alloy or a Fracturing Orb.

| id | source | classes | the route |
|---|---|---|---|
| `close-a-side` | maxroll's "How to Craft", method 3; the domistae codex | every equipment class with spawn weights | From a rare with a closable side, or with one side open: remove a blocker (a certain whittle, else an annulment, side-restricted when the other side holds a target); close the finished side with cheap side-restricted exalts (two at a time with Greater Exaltation where two slots are free); slam the one open side with no side omen; otherwise a side-restricted slam per target. |
| `essence-desecrate-double-slam` | timesaver's 0.5 guide; the domistae codex; maxroll and Mobalytics (methods 2 and 4) | every equipment class with spawn weights | From a magic item an essence binds on, or a rare: the essence; a desecration for the hardest target on its side (Sinistral or Dextral Necromancy), revealed with Abyssal Echoes; a wrong reveal taken off with Omen of Light; a removal when a side is blocked; then side-restricted Perfect exalts, with Greater Exaltation where two fit. |
| `magic-augment-essence` | maxroll's "How to Craft", method 1 | every equipment class with spawn weights | From a normal or magic item: transmute; annul a wrong modifier on the magic item (side-restricted when the other side holds a target); augment; an essence that makes it rare with a third target; then removals and side-restricted slams. |
| `rare-clean-then-slam` | issue #28; maxroll and the domistae codex (method 2's clean-up, method 3's side restriction) | every equipment class with spawn weights | From a rare with any number of targets, a dropped one included: clean the blocked side (a certain whittle, else an annulment, side-restricted when the other side holds a target) until each side has a free slot per missing target, then side-restricted slams. |
| `rings-resist-whittle` | post 10 | Rings | Chaos Orbs, then whittling until one target is left, then a desecration for the last. |
| `rings-flat-attack` | post 11 | Rings | Attack quality raised first (manual), flat attack damage prefixes over resistance suffixes. |
| `gloves-crit-melee` | post 9 | Gloves | Critical damage from Essence of Hysteria, a desecrated prefix that blocks a common family while the prefixes are slammed, then desecrated suffixes until attack speed. |
| `amulets-resist-spirit` | post 14 | Amulets | Resistances turned into the wanted elements with fluxes, a desecrated blocker suffix for two Perfect Exalted Orbs, then a desecrated Spirit modifier. |
| `jewels-three-one-side` | post 6 (post 5's note in `notes`) | Jewels | Three targets on one side, the third from a desecration, then the other side. |
| `time-lost-jewel-radius` | post 3 | Jewels | Three suffixes, a desecrated prefix and the Very Large radius (manual) on a Time-Lost jewel. |
| `weapons-alloy-budget` | post 12 | Staves, Wands and the physical weapon classes both alloys fit | A damage modifier by Chaos Orb, two alloys over exalted filler suffixes, a side-restricted Perfect Exalted Orb and a desecration. |
| `caster-cast-speed` | post 13 | Wands, Staves, Foci | Cast speed from the Transcendent Alloy, a desecration holding a slot while a Perfect Exalted Orb adds two, then the last desecration. |
| `boots-runic-ward` | post 8 | Boots | From fractured movement speed: Runic Ward and augment effect (manual), both attributes by Chaos Orb and desecration, the runes (manual). |

Steps that rest on a mechanic the docs mark not established (an alloy with
the Crystallisation omens, how a flux maps one resistance's tier to
another's, catalyst quality, rune sockets, which side a Contempt modifier
lands on) are kept, and their catalyst and rune steps are `manual`. Each
blueprint's `notes` names what it rests on, but the craft window does not
show `notes`: the caveats live in the blueprint file and in this guide.

What each state-ordered blueprint relies on (`relies_on`; each fact is
labelled not established in the spec's facts table, which names its
source):

| Fact | What the route assumes | Relied on by |
|---|---|---|
| `greater-with-side-omen` | Greater Exaltation with a Sinistral or Dextral Exaltation omen adds two modifiers, both on that side. | all four |
| `greater-needs-two-free` | Greater Exaltation needs two free slots (on the restricted side, with a side omen). | all four |
| `open-side-landing` | An exalt or augment with no side omen, on an item whose other side is full, lands on the open side. | `close-a-side`, `magic-augment-essence` |
| `whittling-mod-level` | Whittling takes the modifier with the lowest modifier level. | all four |
| `fractured-not-removed` | A fractured modifier is never taken by an annulment or a Chaos Orb. | all four |
| `omens-active-before-use` | A Necromancy omen must be active before the bone, Abyssal Echoes before the reveal. | `essence-desecrate-double-slam` |
| `necromancy-without-pool` | What Sinistral Necromancy does on a class with no desecrated prefixes. | `essence-desecrate-double-slam` |

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
