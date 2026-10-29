# Crafting Helper: writing a blueprint

A blueprint is one crafting technique written down as a short TOML file:
which items it starts from, which steps it takes, which currencies each step
may use, and where to go after a step lands or misses. The crafting helper's
engine (`ch-engine`) reads every blueprint under
[`apps/crafting-helper/blueprints/`](../../apps/crafting-helper/blueprints/),
picks the ones that fit an item and the modifiers you want on it, and guides
you through one step at a time.

No window uses the engine yet: the craft window is M5's work. Until then a
blueprint is checked with `check_blueprints` (below) and walked in
`ch-engine`'s tests.

- What each currency, omen and bone does:
  [`crafting-mechanics.md`](crafting-mechanics.md)
- The guide, the crates and their tests: [`README.md`](README.md)

## A short example

One file per blueprint, named `<id>.toml`. A ring brute-forced with Chaos
Orbs and whittling, then desecrated for the last target (the shipped
`rings-resist-whittle`, shortened):

```toml
id = "rings-resist-whittle"                      # the file name without .toml
name = "Rings: resistances by whittling"        # the title the player sees
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
| `name` | yes | The title the player sees. Not empty. |
| `classes` | yes | The item classes the blueprint is for, as the data spells them: `Gloves`, `Boots`, `Helmets`, `Body Armours`, `Shields`, `Quivers`, `Bucklers`, `Foci`, `Wands`, `One Hand Maces`, `Sceptres`, `Spears`, `Bows`, `Staves`, `Two Hand Maces`, `Quarterstaves`, `Crossbows`, `Talismans`, `Belts`, `Amulets`, `Rings`, `Jewels`. At least one. |
| `source` | yes | Where the technique came from: `"Discord post 10, owner's paste, 2026-10-28"`, or `"roadmap"` for the owner's own description. Not empty. |
| `notes` | no | Free text: caveats, and every mechanic the route rests on that the docs mark not established. |
| `[[start]]` | at least one | A start alternative (below). |
| `[[step]]` | at least one | The steps, in order (below). |

A blueprint whose classes name one class is offered before one naming many;
among equals, file order (the file name) decides.

## Start alternatives and conditions

A `[[start]]` table is a predicate on the item; the route may start when any
one of them holds. Every key left out holds. A step's `requires` and
`until` take the same keys plus the step-only ones.

| Key | Where | Holds when |
|---|---|---|
| `rarity` | start, step | The item's rarity is one of the list: `normal`, `magic`, `rare`. |
| `wanted_at_least` | start, step | At least n targets are on the item at their floor. |
| `wanted_prefixes_at_least`, `wanted_suffixes_at_least` | start, step | At least n of them are prefixes (suffixes). |
| `fractured_wanted` | start, step | `true`: a fractured modifier is present and every fractured one is a target. `false`: none is fractured. |
| `unwanted_at_most` | start, step | At most n modifiers on the item are not targets. |
| `essence_binds` | start, step | An unblocked essence, alloy or liquid emotion adds one of the targets at its floor on this item. |
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
side. These limits are the owner's, not measured.

## Steps

| Key | Required | What it holds |
|---|---|---|
| `id` | yes | Unique in the blueprint; `goto:` names it. |
| `say` | yes | The short instruction the player sees. |
| `goal` | yes | What the step is for, and so what the engine checks after it (below). |
| `side` | no | `prefix` or `suffix`: the side an add or remove goal is about. |
| `bind` | no | Which target the step aims at (below). A `reveal` or `convert` step needs one. |
| `requires` | no | A condition that must hold before the step is taken. When it does not, the guide says which part failed and stops ("stuck"); it never picks another step by itself. |
| `when` | no | `"bound"`: the step is part of the route only when its binding resolved (an essence step with no essence for any target is skipped). Needs `bind`. |
| `until` | no | On a hit, repeat the step until this condition holds (`{ wanted_remaining_at_most = 2 }`: fill until two targets are left). |
| `[[step.action]]` | yes, but a `manual` step may have none | The ways to take the step, preferred first. |
| `on_hit` | no | Where to go on a hit. Default: `next`, or `repeat` while `until` does not hold. |
| `on_miss` | no | Where to go on a miss. Default: `repeat`. |

A step whose goal is already met on the item, or whose `when` did not
resolve, is skipped, and the guide says so.

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
never takes a transition: it stops the guide and asks for a re-plan.

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

Three placeholders stand for a choice the engine makes on the item:

| Placeholder | Where | Means |
|---|---|---|
| `desecrate` | `currency` | The bones that fit the item's class and level, tried Preserved, Ancient, Gnawed (Gnawed only up to item level 64). |
| `reveal` | `currency` | The reveal at the Well of Souls. It names no crafting item, so nothing blocks it. |
| `side-omen` | `omens` | The Sinistral or Dextral omen of the action's currency for the bound target's side (or the step's `side`). Needs a `bind` or a `side`, and a currency that has side omens. |

Blocking applies to what a placeholder resolves to: block
`preserved-jawbone` and the next bones that fit are offered.

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

Ten blueprints ship, each a paraphrase in our own words of the source it
names (the owner's Discord paste of 2026-10-28, or the roadmap); see
[`crafting-mechanics.md`](crafting-mechanics.md) § "Crafting patterns
seen". Each is walked on a hit path and a miss path in `ch-engine`'s tests
(`tests/seeds_gear.rs`, `tests/seeds_jewels_weapons.rs`).

| id | source | classes | the route |
|---|---|---|---|
| `fill-desecrate-exalt` | roadmap | every equipment class with spawn weights | An essence when one meets a target's floor, side-restricted exalts until two targets are left (a miss whittled or annulled), a desecration for the hardest target revealed with Abyssal Echoes (a miss cleared with Omen of Light), and a last side-restricted Perfect, Greater or plain exalt. |
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
lands on) are kept; each blueprint's `notes` says so, and its catalyst and
rune steps are `manual`.

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
