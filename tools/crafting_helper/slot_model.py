"""The slot model: how state-ordered crafting routes use an item's free slots.

Developer-only, stdlib only. Built from `docs/models/crafting-slots-spec.md`
alone (a gap the spec leaves is filled in the spec first, then here), and
held against the engine (`ch-engine`) through the shared case file
`apps/crafting-helper/crates/ch-engine/tests/items/slots/cases.json`:

    py -3 -m tools.crafting_helper.slot_model check [--cases FILE]
    py -3 -m tools.crafting_helper.slot_model show [--cases FILE]

`check` plans every case with the four v2 blueprints (and reads the listed
`fill-desecrate-exalt` for the baseline) and fails, naming each difference,
when a result departs from the case file. `show` prints what the model
computes, for writing a new case; a recorded expectation is never edited to
match the model (a difference is a spec or model defect).

What is here:

- the slot accounting (cap, mods, free, need, spare; blocked, reserved,
  closable, open) and what each currency and omen the v2 blueprints name
  does to the slots;
- the four v2 blueprints as rule lists (`BLUEPRINTS`), each with the
  not-established facts it relies on, the last-target desecration's steps
  among them, bound to `last-remaining` (made afresh on every item) and
  held to the case's `desecration_pool`;
- the next-step rules (certain removals first and Omen of Light waiting
  for a same-side exalt among them), the side
  rules, the `greater-omen`, `side-omen` and `lord-omen`
  placeholders (a target's lord read from the case), the removal rules and the projection (`plan`), with the
  rejections in the spec's order;
- a reader for a listed blueprint, for the baselines only
  (`listed_first_step`);
- readers for the spec's facts table and its relied-on table.

The model has no spawn weights, no floors and no odds beyond a removal's
certainty; those are the engine's (the spec's "Not covered").
"""
from __future__ import annotations

import argparse
import copy
import json
import re
import sys
import tomllib
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SPEC = ROOT / "docs" / "models" / "crafting-slots-spec.md"
CASES = ROOT / "apps" / "crafting-helper" / "crates" / "ch-engine" / "tests" / "items" / "slots" / "cases.json"
# The listed blueprint the baselines read: the engine's test copy once it
# exists, the shipped file before it does.
LISTED_PATHS = (
    ROOT / "apps" / "crafting-helper" / "crates" / "ch-engine" / "tests" / "items" / "gear"
    / "fill-desecrate-exalt" / "fill-desecrate-exalt.toml",
    ROOT / "apps" / "crafting-helper" / "blueprints" / "fill-desecrate-exalt.toml",
)

PREFIX, SUFFIX = "prefix", "suffix"
SIDES = (PREFIX, SUFFIX)
CAPS = {"normal": 0, "magic": 1, "rare": 3}
RARE_CAP = 3
STEP_CAP = 16
# Each source names the day it was read: fetched (a page) or reviewed (one
# an issue records, maxroll's crafting guide in #35).
FETCHED = re.compile(r"\b(?:fetched|reviewed) \d{4}-\d{2}-\d{2}\b")
LABELS = ("measured", "documented", "not established")

EXALTS = ("exalted-orb", "greater-exalted-orb", "perfect-exalted-orb")
AUGMENTS = ("orb-of-augmentation", "greater-orb-of-augmentation", "perfect-orb-of-augmentation")
TRANSMUTES = ("orb-of-transmutation", "greater-orb-of-transmutation", "perfect-orb-of-transmutation")
ESSENCE_FAMILIES = (
    "the-body", "the-mind", "enhancement", "insulation", "thawing", "grounding",
    "ruin", "the-infinite", "abrasion", "flames", "ice", "electricity", "battle",
    "seeking", "alacrity", "haste", "sorcery", "command", "opulence",
)
# Lesser, plain and Greater per family, cheapest first: the list
# `fill-desecrate-exalt` names (Perfect and corrupted essences are out).
ESSENCES = tuple(f"{g}essence-of-{f}" for f in ESSENCE_FAMILIES for g in ("lesser-", "", "greater-"))
ANNUL = "orb-of-annulment"
CHAOS = "chaos-orb"
DESECRATE = "desecrate"
REVEAL = "reveal"

GREATER = "omen-of-greater-exaltation"
WHITTLING = "omen-of-whittling"
LIGHT = "omen-of-light"
ECHOES = "omen-of-abyssal-echoes"
SIDE_OMENS = {
    "exalt": {PREFIX: "omen-of-sinistral-exaltation", SUFFIX: "omen-of-dextral-exaltation"},
    "annul": {PREFIX: "omen-of-sinistral-annulment", SUFFIX: "omen-of-dextral-annulment"},
    "desecrate": {PREFIX: "omen-of-sinistral-necromancy", SUFFIX: "omen-of-dextral-necromancy"},
}
REMOVALS = ("remove-unwanted", "remove-desecrated")
# A desecrated modifier's lord to the omen that guarantees one of its
# modifiers (spec § "`lord-omen`").
LORD_OMENS = {
    "ulaman": "omen-of-the-sovereign",
    "amanamu": "omen-of-the-liege",
    "kurgal": "omen-of-the-blackblooded",
}
# The classes a lord's omen works on (`lord-omen-classes`): the jawbone
# classes but Quivers, and Amulets and Rings.
LORD_CLASSES = (
    "Wands", "One Hand Maces", "Sceptres", "Spears", "Bows", "Staves",
    "Two Hand Maces", "Quarterstaves", "Crossbows", "Talismans", "Amulets", "Rings",
)

# Every equipment class `fill-desecrate-exalt` names; jewels have no spawn
# weights and no v2 blueprint names them.
EQUIPMENT_CLASSES = (
    "Gloves", "Boots", "Helmets", "Body Armours", "Shields", "Quivers",
    "Bucklers", "Foci", "Wands", "One Hand Maces", "Sceptres", "Spears",
    "Bows", "Staves", "Two Hand Maces", "Quarterstaves", "Crossbows",
    "Talismans", "Belts", "Amulets", "Rings",
)


def other(side):
    return SUFFIX if side == PREFIX else PREFIX


def kind(currency):
    """What a currency id does, in the model's terms."""
    if currency in EXALTS:
        return "exalt"
    if currency in AUGMENTS:
        return "augment"
    if currency in TRANSMUTES:
        return "transmute"
    if currency in ESSENCES:
        return "essence"
    if currency == ANNUL:
        return "annul"
    if currency == CHAOS:
        return "chaos"
    if currency == DESECRATE:
        return "desecrate"
    if currency == REVEAL:
        return "reveal"
    raise ValueError(f"the model does not know the currency {currency!r}")


def omen_side(omen):
    """The side a Sinistral or Dextral omen restricts to, else None."""
    for by_side in SIDE_OMENS.values():
        for side, name in by_side.items():
            if name == omen:
                return side
    return None


# --- the item -------------------------------------------------------------


@dataclass
class Mod:
    alias: str
    role: str  # "target", "junk" or "filler" (a projected non-target)
    fractured: bool = False
    desecrated: bool = False
    unrevealed: bool = False
    level: int | None = None
    waits_for: str | None = None  # an unrevealed modifier's bound target

    @property
    def is_target(self):
        return self.role == "target" and not self.unrevealed


@dataclass
class Item:
    rarity: str
    prefix: list
    suffix: list

    def side(self, side):
        return self.prefix if side == PREFIX else self.suffix

    def mods(self):
        return self.prefix + self.suffix

    def placed(self):
        return [(s, m) for s in SIDES for m in self.side(s)]


@dataclass(frozen=True)
class Target:
    alias: str
    side: str
    pool: str  # "base" or "desecrated"
    essence: str | None
    # A desecrated-pool target's lord (a key of LORD_OMENS), when every tier
    # of its modifier names the same one; None otherwise.
    lord: str | None = None


@dataclass
class Case:
    id: str
    cls: str
    item: Item
    targets: tuple
    blocked: frozenset
    hardest: str | None
    expect: dict
    listed: dict | None = None
    about: str = ""
    # The sides on which the class has a desecrated modifier at the item's
    # level (the last-target desecration's pool condition).
    desecration_pool: frozenset = frozenset(SIDES)


def _mod_from_dict(d):
    role = d["role"]
    if role not in ("target", "junk"):
        raise ValueError(f"modifier {d['alias']!r}: role {role!r} is not target or junk")
    return Mod(
        alias=d["alias"], role=role,
        fractured=bool(d.get("fractured", False)),
        desecrated=bool(d.get("desecrated", False)),
        unrevealed=bool(d.get("unrevealed", False)),
        level=d.get("level"),
        waits_for=d.get("waits_for"),
    )


def case_from_dict(d):
    if d["rarity"] not in CAPS:
        raise ValueError(f"case {d['id']!r}: rarity {d['rarity']!r}")
    targets = tuple(
        Target(t["alias"], t["side"], t.get("pool", "base"), t.get("essence"), t.get("lord"))
        for t in d["targets"]
    )
    for t in targets:
        if t.side not in SIDES or t.pool not in ("base", "desecrated"):
            raise ValueError(f"case {d['id']!r}: target {t}")
        if t.lord is not None and (t.pool != "desecrated" or t.lord not in LORD_OMENS):
            raise ValueError(f"case {d['id']!r}: target {t.alias!r} has lord {t.lord!r}")
    pool = frozenset(d.get("desecration_pool", SIDES))
    if not pool <= set(SIDES):
        raise ValueError(f"case {d['id']!r}: desecration_pool {sorted(pool)}")
    return Case(
        id=d["id"], cls=d["class"],
        item=Item(d["rarity"], [_mod_from_dict(m) for m in d["prefixes"]],
                  [_mod_from_dict(m) for m in d["suffixes"]]),
        targets=targets,
        blocked=frozenset(d.get("blocked", [])),
        hardest=d.get("hardest"),
        expect=d.get("expect", {}),
        listed=d.get("listed"),
        about=d.get("about", ""),
        desecration_pool=pool,
    )


def load_cases(path=CASES):
    raw = json.loads(Path(path).read_text(encoding="utf-8"))
    return [case_from_dict(c) for c in raw["cases"]]


@dataclass
class State:
    """The item as it stands, the craft's targets and the route's bindings."""
    item: Item
    targets: tuple
    blocked: frozenset
    hardest_alias: str | None = None
    bindings: dict = field(default_factory=dict)
    fillers: int = 0
    desecration_pool: frozenset = frozenset(SIDES)
    cls: str = ""

    @classmethod
    def from_case(cls, case):
        return cls(copy.deepcopy(case.item), case.targets, case.blocked, case.hardest,
                   desecration_pool=case.desecration_pool, cls=case.cls)


def present(state):
    return {m.alias for m in state.item.mods() if m.is_target}


def missing(state):
    have = present(state)
    return [t for t in state.targets if t.alias not in have]


# --- slot accounting (spec § "Slot accounting") ---------------------------


@dataclass(frozen=True)
class SideNumbers:
    side: str
    cap: int
    mods: int
    free: int
    need: int
    spare: int
    blocked: bool
    reserved: bool
    closable: bool
    open: bool


def removable(mod):
    """What a removal step may aim at: a non-target that is not fractured,
    unrevealed or desecrated (a projected filler is one)."""
    return not mod.is_target and not mod.fractured and not mod.unrevealed and not mod.desecrated


def accounting(state):
    item = state.item
    cap = CAPS[item.rarity]
    miss = missing(state)
    raw = {}
    for s in SIDES:
        mods = item.side(s)
        free = cap - len(mods)
        side_missing = [t.alias for t in miss if t.side == s]
        need = len(side_missing)
        need -= sum(1 for m in mods if m.unrevealed and m.waits_for in side_missing)
        if item.rarity == "magic":
            need = min(need, cap - sum(1 for m in mods if m.is_target))
        need = max(need, 0)
        raw[s] = (cap, len(mods), free, need)
    out = {}
    for s in SIDES:
        cap, n, free, need = raw[s]
        spare = free - need
        out[s] = SideNumbers(
            side=s, cap=cap, mods=n, free=free, need=need, spare=spare,
            blocked=spare < 0 and any(removable(m) for m in item.side(s)),
            reserved=need > 0 and spare == 0 and free > 0,
            closable=free > 0 and need == 0 and raw[other(s)][3] > 0,
            open=free > 0,
        )
    return out


def two_open_one_reserved(acc):
    return all(acc[s].open for s in SIDES) and any(acc[s].reserved for s in SIDES)


def landing_sides(state, currency, omens):
    """The sides an adding currency can put its modifier on, as the item
    stands (with a transmute, as the magic item it makes)."""
    k = kind(currency)
    if k not in ("exalt", "augment", "transmute"):
        raise ValueError(f"{currency!r} adds no random modifier")
    restricted = next((omen_side(o) for o in omens if o in SIDE_OMENS["exalt"].values()), None)
    cap = CAPS["magic"] if k == "transmute" else CAPS[state.item.rarity]
    sides = [s for s in SIDES if cap - len(state.item.side(s)) > 0]
    if restricted is not None:
        return [restricted] if restricted in sides else []
    return sides


def at_risk(state, currency, omens):
    """What a removal may actually take, as (side, modifier) pairs."""
    k = kind(currency)
    if k == "annul":
        if LIGHT in omens:
            return [(s, m) for s, m in state.item.placed() if m.desecrated and not m.fractured]
        side = next((omen_side(o) for o in omens if o in SIDE_OMENS["annul"].values()), None)
        return [(s, m) for s, m in state.item.placed()
                if not m.fractured and (side is None or s == side)]
    if k == "chaos" and WHITTLING in omens:
        return []
    raise ValueError(f"{currency!r} with {omens!r} is not a removal the model knows")


def whittle_certain(state, side):
    """The one modifier Whittling is certain to take on `side`, or None.

    Certain: the lowest level on the item is held by exactly one modifier, a
    removable non-target on `side`, no fractured modifier is at or below it,
    and every modifier's level is known (no unrevealed one, no projected
    filler or projected target)."""
    if side is None or state.item.rarity != "rare":
        return None
    mods = state.item.mods()
    if not mods or any(m.unrevealed or m.level is None for m in mods):
        return None
    lowest = min(m.level for m in mods)
    holders = [m for m in mods if m.level == lowest]
    if len(holders) != 1:
        return None
    mod = holders[0]
    if mod not in state.item.side(side) or not removable(mod):
        return None
    if any(m.fractured and m.level <= lowest for m in mods):
        return None
    return mod


# --- blueprints ------------------------------------------------------------


@dataclass(frozen=True)
class Action:
    currency: str
    omens: tuple = ()


@dataclass(frozen=True)
class Step:
    id: str
    goal: str
    side: str | None = None
    actions: tuple = ()
    guard: tuple = ()  # (key, value) pairs
    bind: str | None = None
    when: str | None = None

    @property
    def guard_dict(self):
        return dict(self.guard)


@dataclass
class Blueprint:
    id: str
    classes: tuple
    starts: list
    steps: list
    relies_on: tuple = ()

    def binds(self):
        return {s.bind for s in self.steps if s.bind}


def _grades(currencies, omens=()):
    return tuple(Action(c, tuple(omens)) for c in currencies)


PERFECT_FIRST = tuple(reversed(EXALTS))
CHEAPEST_FIRST = EXALTS

REMOVE = Step("remove", "remove-unwanted", side="blocked", actions=(
    Action(CHAOS, (WHITTLING,)),
    Action(ANNUL, ("side-omen",)),
    Action(ANNUL),
))
SIDE_SLAM = Step("side-slam", "add-wanted", side="target",
                 actions=_grades(PERFECT_FIRST, ("greater-omen", "side-omen")))
ESSENCE_STEP = Step("essence", "add-wanted", bind="essence", when="bound",
                    guard=(("rarity", ("magic",)),), actions=_grades(ESSENCES))

# The last-target desecration (spec § "The last-target desecration"): on a
# rare with no desecrated modifier and one target missing, desecrate that
# target's side and reveal; a wrong reveal is taken off with Omen of Light.
REVEAL_LAST = Step("reveal-last", "reveal", bind="last-remaining", when="bound",
                   actions=(Action(REVEAL, (ECHOES,)), Action(REVEAL)))
LIGHT_LAST = Step("light-last", "remove-desecrated", bind="last-remaining", when="bound",
                  guard=(("desecrated_unwanted", True),),
                  actions=(Action(ANNUL, (LIGHT,)),))
DESECRATE_LAST = Step("desecrate-last", "desecrate", side="target", bind="last-remaining",
                      when="bound", guard=(("rarity", ("rare",)),),
                      actions=(Action(DESECRATE, ("side-omen", "lord-omen")),))

ALL_GREATER = ("greater-with-side-omen", "greater-needs-two-free",
               "whittling-mod-level", "fractured-not-removed")
# Every v2 blueprint's desecration carries `lord-omen` (spec § "`lord-omen`").
LORD_FACTS = ("lord-with-side-omen", "lord-omen-classes")

BLUEPRINTS = {
    "close-a-side": Blueprint(
        id="close-a-side",
        classes=EQUIPMENT_CLASSES,
        starts=[{"rarity": ("rare",), "closable_side": True},
                {"rarity": ("rare",), "one_side_open": True}],
        steps=[
            REVEAL_LAST,
            LIGHT_LAST,
            REMOVE,
            DESECRATE_LAST,
            Step("close", "fill", side="close",
                 actions=_grades(CHEAPEST_FIRST, ("greater-omen", "side-omen"))),
            Step("slam-open-side", "add-wanted", side="target",
                 guard=(("one_side_open", True),),
                 actions=_grades(PERFECT_FIRST, ("greater-omen",))),
            SIDE_SLAM,
        ],
        relies_on=ALL_GREATER + LORD_FACTS + ("open-side-landing", "omens-active-before-use"),
    ),
    "essence-desecrate-double-slam": Blueprint(
        id="essence-desecrate-double-slam",
        classes=EQUIPMENT_CLASSES,
        starts=[{"rarity": ("magic",), "essence_binds": True}, {"rarity": ("rare",)}],
        steps=[
            ESSENCE_STEP,
            Step("reveal", "reveal", bind="hardest", when="bound",
                 actions=(Action(REVEAL, (ECHOES,)), Action(REVEAL))),
            Step("light", "remove-desecrated", bind="hardest", when="bound",
                 guard=(("desecrated_unwanted", True),),
                 actions=(Action(ANNUL, (LIGHT,)),)),
            REMOVE,
            Step("desecrate", "desecrate", side="target", bind="hardest", when="bound",
                 actions=(Action(DESECRATE, ("side-omen", "lord-omen")), Action(DESECRATE))),
            DESECRATE_LAST,
            SIDE_SLAM,
        ],
        relies_on=ALL_GREATER + LORD_FACTS + ("omens-active-before-use", "necromancy-without-pool"),
    ),
    "magic-augment-essence": Blueprint(
        id="magic-augment-essence",
        classes=EQUIPMENT_CLASSES,
        starts=[{"rarity": ("normal",)}, {"rarity": ("magic",)}],
        steps=[
            Step("transmute", "add-wanted", guard=(("rarity", ("normal",)),),
                 actions=_grades(tuple(reversed(TRANSMUTES)))),
            Step("remove-magic", "remove-unwanted", side="blocked",
                 guard=(("rarity", ("magic",)),),
                 actions=(Action(ANNUL, ("side-omen",)), Action(ANNUL))),
            Step("augment", "add-wanted", guard=(("rarity", ("magic",)),),
                 actions=_grades(tuple(reversed(AUGMENTS)))),
            ESSENCE_STEP,
            REVEAL_LAST,
            LIGHT_LAST,
            Step("remove", "remove-unwanted", side="blocked", guard=(("rarity", ("rare",)),),
                 actions=REMOVE.actions),
            DESECRATE_LAST,
            SIDE_SLAM,
        ],
        relies_on=ALL_GREATER + LORD_FACTS + ("open-side-landing", "omens-active-before-use"),
    ),
    "rare-clean-then-slam": Blueprint(
        id="rare-clean-then-slam",
        classes=EQUIPMENT_CLASSES,
        starts=[{"rarity": ("rare",)}],
        steps=[REVEAL_LAST, LIGHT_LAST, REMOVE, DESECRATE_LAST, SIDE_SLAM],
        relies_on=ALL_GREATER + LORD_FACTS + ("omens-active-before-use",),
    ),
}


# --- conditions and bindings -----------------------------------------------


def essence_target(state):
    """The first missing target an unblocked essence adds, else None."""
    for t in missing(state):
        if t.essence and t.essence not in state.blocked:
            return t
    return None


def hardest_target(state):
    miss = missing(state)
    for t in miss:
        if t.pool == "desecrated":
            return t
    if state.hardest_alias is not None:
        for t in state.targets:
            if t.alias == state.hardest_alias:
                return t
        raise ValueError(f"hardest {state.hardest_alias!r} is not a target")
    return miss[0] if miss else None


def ensure_bindings(state, bp):
    """Make each binding the blueprint uses as soon as the item allows it;
    `essence` and `hardest` once made are kept, `last-remaining` is made
    afresh on every item (spec § "Candidates and the next step")."""
    binds = bp.binds()
    if "essence" in binds and "essence" not in state.bindings and state.item.rarity == "magic":
        t = essence_target(state)
        if t is not None:
            state.bindings["essence"] = t.alias
    if "hardest" in binds and "hardest" not in state.bindings and missing(state):
        state.bindings["hardest"] = hardest_target(state).alias
    if "last-remaining" in binds:
        miss = missing(state)
        if len(miss) == 1:
            state.bindings["last-remaining"] = miss[0].alias
        else:
            state.bindings.pop("last-remaining", None)


def condition_failures(state, cond, acc):
    failed = []
    for key, want in dict(cond).items():
        if key == "rarity":
            ok = state.item.rarity in want
        elif key == "essence_binds":
            ok = (essence_target(state) is not None) == want
        elif key == "blocked_side":
            ok = any(acc[s].blocked for s in SIDES) == want
        elif key == "closable_side":
            ok = any(acc[s].closable for s in SIDES) == want
        elif key == "one_side_open":
            ok = (sum(acc[s].open for s in SIDES) == 1) == want
        elif key == "desecrated_unwanted":
            ok = any(m.desecrated and not m.unrevealed and not m.is_target
                     for m in state.item.mods()) == want
        else:
            raise ValueError(f"the model does not know the condition {key!r}")
        if not ok:
            failed.append(f"{key} is not {want}")
    return failed


def target_by_alias(state, alias):
    return next(t for t in state.targets if t.alias == alias)


def placeable(state, step):
    """The missing targets the step could place: its bound essence target for
    the essence step; otherwise base-pool targets, less the one the route's
    essence binding claims."""
    miss = missing(state)
    if step.bind == "essence":
        return [t for t in miss if t.alias == state.bindings.get("essence")]
    claimed = state.bindings.get("essence")
    return [t for t in miss if t.pool == "base" and t.alias != claimed]


def goal_open(state, step, acc):
    g = step.goal
    if g == "add-wanted":
        return bool(placeable(state, step))
    if g == "fill":
        return True
    if g == "desecrate":
        bound = state.bindings.get(step.bind)
        return (bound in {t.alias for t in missing(state)}
                and not any(m.desecrated for m in state.item.mods()))
    if g == "reveal":
        return any(m.unrevealed for m in state.item.mods())
    if g == "remove-unwanted":
        return any(acc[s].blocked for s in SIDES)
    if g == "remove-desecrated":
        return any(m.desecrated for m in state.item.mods())
    raise ValueError(f"the model does not know the goal {g!r}")


def _first_target_side(targets, sides):
    for t in targets:
        if t.side in sides:
            return t.side
    return None


def resolve_side(state, step, acc):
    rule = step.side
    if rule in SIDES:
        return rule
    if rule == "target":
        if step.bind and step.bind in state.bindings:
            return target_by_alias(state, state.bindings[step.bind]).side
        can = placeable(state, step)
        counts = {s: sum(1 for t in can if t.side == s) for s in SIDES if acc[s].open}
        best = max(counts.values(), default=0)
        if best == 0:
            return None
        return _first_target_side(can, [s for s, n in counts.items() if n == best])
    if rule == "close":
        return next((s for s in SIDES if acc[s].closable), None)
    if rule == "blocked":
        blocked = [s for s in SIDES if acc[s].blocked]
        if len(blocked) < 2:
            return blocked[0] if blocked else None
        deficit = {s: acc[s].need - acc[s].free for s in blocked}
        best = max(deficit.values())
        return _first_target_side(missing(state), [s for s in blocked if deficit[s] == best])
    raise ValueError(f"the model does not know the side rule {rule!r}")


# --- fitting an alternative --------------------------------------------------


def greater_fits(state, step, side, acc):
    if GREATER in state.blocked:
        return False
    free = acc[side].free if side else sum(acc[s].free for s in SIDES)
    if free < 2:
        return False
    if step.goal == "fill":
        return True
    if step.goal == "add-wanted":
        return sum(1 for t in placeable(state, step) if side is None or t.side == side) >= 2
    return False


def resolve_omens(state, step, side, action, acc):
    """The action's omens with the placeholders resolved; None when a
    `side-omen` has no side to resolve to."""
    k = kind(action.currency)
    out = []
    for o in action.omens:
        if o == "side-omen":
            if side is None or k not in SIDE_OMENS:
                return None
            out.append(SIDE_OMENS[k][side])
        elif o == "greater-omen":
            if k != "exalt":
                raise ValueError("greater-omen is allowed on an Exalted Orb only")
            if greater_fits(state, step, side, acc):
                out.append(GREATER)
        elif o == "lord-omen":
            if k != "desecrate":
                raise ValueError("lord-omen is allowed on a desecration only")
            lord = lord_omen(state, step)
            if lord is not None:
                out.append(lord)
        else:
            out.append(o)
    return out


def lord_omen(state, step):
    """What `lord-omen` resolves to on this step (spec § "`lord-omen`"): the
    lord omen of its bound target when that target is from the desecrated
    pool with one lord, the class is a lord class and the omen is not
    blocked; else None (nothing, no fallback)."""
    bound = state.bindings.get(step.bind) if step.bind else None
    if bound is None or state.cls not in LORD_CLASSES:
        return None
    t = target_by_alias(state, bound)
    if t.pool != "desecrated" or t.lord is None:
        return None
    omen = LORD_OMENS[t.lord]
    return None if omen in state.blocked else omen


def fit(state, step, side, index, acc):
    """(omens, "") when alternative `index` of `step` fits, else (None, why)."""
    action = step.actions[index]
    if action.currency in state.blocked:
        return None, "blocked"
    omens = resolve_omens(state, step, side, action, acc)
    if omens is None:
        return None, "no side for its side omen"
    if any(o in state.blocked for o in omens):
        return None, "blocked"
    item = state.item
    k = kind(action.currency)
    if k in ("exalt", "augment", "transmute"):
        needs = {"exalt": "rare", "augment": "magic", "transmute": "normal"}[k]
        if item.rarity != needs:
            return None, f"needs a {needs} item"
        restricted = next((omen_side(o) for o in omens if omen_side(o)), None)
        if k == "exalt" and restricted is None and two_open_one_reserved(acc):
            return None, "an unrestricted exalt could take the reserved slot"
        lands = landing_sides(state, action.currency, omens)
        if side is not None:
            lands = [s for s in lands if s == side]
        if not lands:
            return None, "no free slot where it lands"
        if step.goal == "add-wanted" and not any(t.side in lands for t in placeable(state, step)):
            return None, "no target it can place where it lands"
        return omens, ""
    if k == "essence":
        if item.rarity != "magic":
            return None, "needs a magic item"
        bound = state.bindings.get("essence")
        t = target_by_alias(state, bound) if bound else None
        if t is None or t.essence != action.currency:
            return None, "does not add the bound target"
        if RARE_CAP - len(item.side(t.side)) <= 0:
            return None, "no free slot where it lands"
        return omens, ""
    if k == "desecrate":
        if item.rarity != "rare" or side is None:
            return None, "needs a rare item and a side"
        if acc[side].free <= 0:
            return None, "no free slot on its side"
        if step.bind == "last-remaining" and side not in state.desecration_pool:
            return None, f"no desecrated {side} modifier for the class at the item level"
        return omens, ""
    if k == "reveal":
        return (omens, "") if any(m.unrevealed for m in item.mods()) else (None, "nothing to reveal")
    if k == "annul":
        if item.rarity not in ("magic", "rare"):
            return None, "needs a magic or rare item"
        restricted = next((omen_side(o) for o in omens if o in SIDE_OMENS["annul"].values()), None)
        if restricted is not None:
            if not any(m.is_target and not m.fractured for m in item.side(other(restricted))):
                return None, "the other side holds no target to protect"
        if not at_risk(state, action.currency, omens):
            return None, "nothing it could take"
        return omens, ""
    if k == "chaos":
        if WHITTLING not in omens:
            raise ValueError("a Chaos Orb without Omen of Whittling is not modelled")
        if whittle_certain(state, side) is None:
            return None, "the whittle is not certain"
        for j in range(index + 1, len(step.actions)):
            if kind(step.actions[j].currency) != "annul":
                continue
            ann, _ = fit(state, step, side, j, acc)
            if ann is None:
                continue
            risk = at_risk(state, step.actions[j].currency, ann)
            if any(m.is_target for _, m in risk):
                return omens, ""
            return None, "no annulment that fits could take a target"
        return None, "no annulment that fits could take a target"
    raise ValueError(f"unhandled currency kind {k!r}")


@dataclass
class Choice:
    step: Step
    side: str | None
    action: Action
    omens: list


def evaluate(state, step, acc):
    """A Choice when `step` is a candidate, else the reason it is not."""
    if step.guard and condition_failures(state, step.guard, acc):
        return "guard false"
    if step.when == "bound" and step.bind not in state.bindings:
        return "cannot bind"
    if not goal_open(state, step, acc):
        return "goal met"
    side = None
    if step.side is not None:
        side = resolve_side(state, step, acc)
        if side is None:
            return "no side"
    whys = []
    for i, action in enumerate(step.actions):
        omens, why = fit(state, step, side, i, acc)
        if omens is not None:
            return Choice(step, side, action, omens)
        whys.append(why)
    if whys and all(w == "blocked" for w in whys):
        return "every alternative blocked"
    return "no alternative fits: " + "; ".join(whys)


def removal_certain(state, choice):
    """Whether a removal's chosen alternative is certain to take only
    non-targets: a whittle is chosen only when certain; an annulment is
    certain when every modifier it may take is a non-target."""
    if kind(choice.action.currency) == "chaos":
        return True
    return all(not m.is_target for _, m in at_risk(state, choice.action.currency, choice.omens))


def certain_removal(state, choice):
    return choice.step.goal in REMOVALS and removal_certain(state, choice)


def light_waits(state, bp, step, acc):
    """The add-wanted step a `remove-desecrated` step guarded by
    `desecrated_unwanted = true` waits for, or None (spec § "A wrong reveal
    waits for the same-side exalt"): one that is a candidate on the side of
    a revealed desecrated non-target, with a free slot there (a step without
    `side`: one that can place a missing target there)."""
    if step.goal != "remove-desecrated" or ("desecrated_unwanted", True) not in step.guard:
        return None
    sides = {s for s, m in state.item.placed()
             if m.desecrated and not m.unrevealed and not m.is_target}
    open_sides = {s for s in sides if acc[s].free > 0}
    if not open_sides:
        return None
    for add in bp.steps:
        if add.goal != "add-wanted":
            continue
        got = evaluate(state, add, acc)
        if not isinstance(got, Choice):
            continue
        if got.side is not None:
            if got.side in open_sides:
                return add
        elif any(t.side in open_sides for t in placeable(state, add)):
            return add
    return None


def next_step(state, bp, acc):
    """The next step (spec §§ "Candidates and the next step", "Certain
    removals first" and "A wrong reveal waits for the same-side exalt"): the
    first candidate in file order, unless it is not a certain removal and a
    later candidate is a `remove-unwanted` step whose chosen alternative is
    certain; then the first such later one. A Light step that waits is not
    a candidate."""
    reasons = {}
    first = None
    for step in bp.steps:
        got = evaluate(state, step, acc)
        if isinstance(got, Choice):
            add = light_waits(state, bp, step, acc)
            if add is not None:
                got = f"waits for {add.id}, an add on the revealed desecrated modifier's side"
        if not isinstance(got, Choice):
            reasons[step.id] = got
            continue
        if first is None:
            if certain_removal(state, got):
                return got, reasons
            first = got
            continue
        if step.goal == "remove-unwanted" and removal_certain(state, got):
            return got, reasons
    return first, reasons


# --- the projection ----------------------------------------------------------


def removal_takes(state, choice):
    """(side, modifier) the removal takes if it hits."""
    item, omens, side = state.item, choice.omens, choice.side
    if kind(choice.action.currency) == "chaos":
        return side, whittle_certain(state, side)
    if LIGHT in omens:
        for s, m in item.placed():
            if m.desecrated and not m.unrevealed and not m.is_target and not m.fractured:
                return s, m
        return None, None
    sides = [side] if side else list(SIDES)
    for s in sides:
        ordered = [m for m in item.side(s) if m.role != "filler"] + [m for m in item.side(s) if m.role == "filler"]
        for m in ordered:
            if removable(m):
                return s, m
    return None, None


def _place(state, target):
    state.item.side(target.side).append(Mod(target.alias, "target"))


def apply_hit(state, choice):
    new = copy.deepcopy(state)
    item = new.item
    step, omens, side = choice.step, choice.omens, choice.side
    k = kind(choice.action.currency)
    count = 2 if GREATER in omens else 1
    if k in ("exalt", "augment", "transmute"):
        if k == "transmute":
            item.rarity = "magic"
        if step.goal == "fill":
            for _ in range(count):
                new.fillers += 1
                item.side(side).append(Mod(f"filler-{new.fillers}", "filler"))
        else:
            for _ in range(count):
                lands = landing_sides(new, choice.action.currency, omens)
                if side is not None:
                    lands = [s for s in lands if s == side]
                can = [t for t in placeable(new, step) if t.side in lands]
                if not can:
                    break
                _place(new, can[0])
    elif k == "essence":
        item.rarity = "rare"
        _place(new, target_by_alias(new, new.bindings["essence"]))
    elif k == "desecrate":
        item.side(side).append(Mod("unrevealed", "junk", desecrated=True, unrevealed=True,
                                   waits_for=new.bindings.get(step.bind)))
    elif k == "reveal":
        for m in item.mods():
            if m.unrevealed:
                m.alias = m.waits_for or new.bindings.get(step.bind)
                m.role, m.unrevealed, m.level, m.waits_for = "target", False, None, None
                break
    else:
        s, taken = removal_takes(state, choice)
        if taken is not None:
            item.side(s).pop(state.item.side(s).index(taken))
    return new


@dataclass
class ProjectedStep:
    record: dict
    step: Step
    before: State
    accounting: dict
    after: State


@dataclass
class Route:
    steps: list
    complete: bool = False
    kind: str | None = None  # the rejection, kebab case, as the engine serialises it
    why: str | None = None
    detail: object = None

    def record(self):
        if self.kind is not None:
            return {"rejected": {"kind": self.kind}}
        return {"complete": self.complete, "steps": [s.record for s in self.steps]}


def _record(state, choice, acc):
    rec = {
        "goal": choice.step.goal,
        "side": choice.side,
        "currency": choice.action.currency,
        "omens": list(choice.omens),
    }
    if choice.step.goal in REMOVALS:
        if kind(choice.action.currency) == "chaos":
            rec["certain"] = True
        else:
            risk = at_risk(state, choice.action.currency, choice.omens)
            rec["certain"] = all(not m.is_target for _, m in risk)
    return rec


def plan(bp, case):
    """The route a state-ordered blueprint previews on the case's item, or
    its rejection (spec §§ "Rejections" and "Projection")."""
    state = State.from_case(case)
    if case.cls not in bp.classes:
        return Route([], kind="class-not-covered")
    acc = accounting(state)
    if not any(not condition_failures(state, s, acc) for s in bp.starts):
        return Route([], kind="no-start-fits")
    for s in SIDES:
        if sum(1 for t in state.targets if t.side == s) > RARE_CAP:
            return Route([], kind="targets-exceed-slots", detail=s)
    if (any(t.pool == "desecrated" for t in missing(state))
            and not any(st.goal == "desecrate" for st in bp.steps)):
        return Route([], kind="desecrated-target-unplaced")
    steps = []
    while True:
        ensure_bindings(state, bp)
        if not missing(state):
            return Route(steps, complete=True)
        if len(steps) >= STEP_CAP:
            return Route(steps, kind="route-cannot-finish", why="step-cap")
        acc = accounting(state)
        choice, reasons = next_step(state, bp, acc)
        if choice is None:
            return Route(steps, kind="route-cannot-finish", why="no-candidate", detail=reasons)
        after = apply_hit(state, choice)
        if after.item == state.item:
            return Route(steps, kind="route-cannot-finish", why="no-change", detail=choice.step.id)
        steps.append(ProjectedStep(_record(state, choice, acc), choice.step, state, acc, after))
        state = after


# --- invariants a projected step must keep ----------------------------------


def omen_problems(pstep):
    """What a projected step is missing or gets wrong in its omen list."""
    rec, acc = pstep.record, pstep.accounting
    k = kind(rec["currency"])
    omens, side = rec["omens"], rec["side"]
    problems = []
    sided = [o for o in omens if omen_side(o)]
    for o in sided:
        if side is not None and omen_side(o) != side:
            problems.append(f"{o} restricts to the other side than {side}")
        family = next(f for f, m in SIDE_OMENS.items() if o in m.values())
        if family != k:
            problems.append(f"{o} does not go with {rec['currency']}")
    if GREATER in omens:
        if k != "exalt":
            problems.append("Greater Exaltation on a currency that is not an Exalted Orb")
        if not sided and all(acc[s].open for s in SIDES):
            problems.append("Greater Exaltation without a side omen while both sides are open")
    if k == "exalt" and not sided and two_open_one_reserved(acc):
        problems.append("an unrestricted exalt while a reserved side shares the open slots")
    if (k == "desecrate" and side is not None and SIDE_OMENS["desecrate"][side] not in omens
            and SIDE_OMENS["desecrate"][side] not in pstep.before.blocked):
        problems.append(f"a {side} desecration without its Necromancy omen")
    lords = [o for o in omens if o in LORD_OMENS.values()]
    # the v2 blueprints write `lord-omen` beside `side-omen` only: the plain
    # bone, taken when the Necromancy omen is blocked, carries none
    necromancy = k == "desecrate" and any(o in SIDE_OMENS["desecrate"].values() for o in omens)
    want = lord_omen(pstep.before, pstep.step) if necromancy else None
    if lords != ([want] if want else []):
        problems.append(f"lord omens {lords}, the bound target asks {want or 'none'}")
    if k == "reveal" and ECHOES not in omens and ECHOES not in pstep.before.blocked:
        problems.append("a reveal without Omen of Abyssal Echoes")
    return problems


# --- the listed blueprint, for the baselines only ---------------------------


def listed_blueprint_path():
    for p in LISTED_PATHS:
        if p.is_file():
            return p
    raise FileNotFoundError("no fill-desecrate-exalt.toml at " + ", ".join(map(str, LISTED_PATHS)))


def read_listed(path):
    with open(path, "rb") as f:
        bp = tomllib.load(f)
    if bp.get("order", "listed") != "listed":
        raise ValueError(f"{path} is not a listed blueprint")
    return bp


def _listed_start_holds(state, cond):
    for key, want in cond.items():
        if key == "rarity":
            ok = state.item.rarity in want
        elif key == "wanted_at_least":
            ok = len(present(state)) >= want
        elif key == "fractured_wanted":
            fractured = [m for m in state.item.mods() if m.fractured]
            ok = bool(fractured) and all(m.is_target for m in fractured) if want else not fractured
        elif key == "essence_binds":
            ok = (essence_target(state) is not None) == want
        else:
            raise ValueError(f"the listed reading does not model the start key {key!r}")
        if not ok:
            return False
    return True


def listed_first_step(bp, case):
    """A listed blueprint's starts as written, and the first fitting action
    of its first step that applies, as written (spec § "Listed order")."""
    state = State.from_case(case)
    if case.cls not in bp.get("classes", []):
        return {"rejected": {"kind": "class-not-covered"}}
    if not any(_listed_start_holds(state, s) for s in bp.get("start", [])):
        return {"rejected": {"kind": "no-start-fits"}}
    if not missing(state):
        return {"done": True}
    for step in bp.get("step", []):
        bound = None
        if step.get("when") == "bound":
            if step.get("bind") == "essence":
                t = essence_target(state) if state.item.rarity == "magic" else None
            elif step.get("bind") == "hardest":
                t = hardest_target(state)
            else:
                raise ValueError(f"the listed reading does not model bind {step.get('bind')!r}")
            if t is None:
                continue
            bound = t
        if step["goal"] != "add-wanted":
            raise ValueError(f"the listed reading reached a {step['goal']!r} step")
        for action in step.get("action", []):
            currency, omens = action["currency"], list(action.get("omens", []))
            if any(o in ("side-omen", "greater-omen") for o in omens):
                raise ValueError("the listed reading does not resolve placeholders")
            if currency in state.blocked or any(o in state.blocked for o in omens):
                continue
            k = kind(currency)
            if k == "essence" and (bound is None or bound.essence != currency):
                continue
            if k == "exalt" and (state.item.rarity != "rare"
                                 or not landing_sides(state, currency, omens)):
                continue
            return {"step": step["id"], "currency": currency, "omens": omens}
    return {"rejected": {"kind": "no-alternative-fits"}}


# --- the spec's tables --------------------------------------------------------


def _section(text, heading):
    start = text.find("\n" + heading + "\n")
    if start < 0:
        raise ValueError(f"the spec has no {heading!r}")
    rest = text[start + len(heading) + 2:]
    ends = [i for i in (rest.find("\n## "), rest.find("\n### ")) if i >= 0]
    return rest[:min(ends)] if ends else rest


def _rows(section):
    for line in section.splitlines():
        if line.startswith("| `"):
            yield [c.strip() for c in line.strip().strip("|").split("|")]


def spec_facts(text):
    """The facts table's rows: id, label, fact, source."""
    out = []
    for cells in _rows(_section(text, "## Facts")):
        if len(cells) != 4:
            raise ValueError(f"a facts row has {len(cells)} cells: {cells}")
        out.append({"id": cells[0].strip("`"), "label": cells[1], "fact": cells[2], "source": cells[3]})
    return out


def fact_problems(row):
    problems = []
    if row["label"] not in LABELS:
        problems.append(f"label {row['label']!r} is not one of {LABELS}")
    if "https://" not in row["source"]:
        problems.append("the source names no URL")
    if not FETCHED.search(row["source"]):
        problems.append("the source does not say when it was fetched")
    if not row["fact"].strip():
        problems.append("the fact is empty")
    return problems


def relied_on_table(text):
    """Fact id -> the blueprint ids the spec says rely on it."""
    out = {}
    for cells in _rows(_section(text, "### Which blueprint relies on which not-established fact")):
        names = [n.strip().strip("`") for n in cells[1].split(",")]
        out[cells[0].strip("`")] = [n for n in names if n and n != "none"]
    return out


# --- command line -----------------------------------------------------------


def computed(case):
    out = {bp_id: plan(bp, case).record() for bp_id, bp in BLUEPRINTS.items()}
    result = {"expect": out}
    if case.listed is not None:
        result["listed"] = listed_first_step(read_listed(listed_blueprint_path()), case)
    return result


def check(path=CASES):
    problems = []
    for case in load_cases(path):
        got = computed(case)
        for bp_id in BLUEPRINTS:
            want = case.expect.get(bp_id)
            if want != got["expect"][bp_id]:
                problems.append(f"{case.id} / {bp_id}: recorded {json.dumps(want)}, "
                                f"model {json.dumps(got['expect'][bp_id])}")
        if case.listed is not None and case.listed != got["listed"]:
            problems.append(f"{case.id} / fill-desecrate-exalt: recorded {json.dumps(case.listed)}, "
                            f"model {json.dumps(got['listed'])}")
    return problems


def main(argv=None):
    ap = argparse.ArgumentParser(prog="py -3 -m tools.crafting_helper.slot_model", description=__doc__.split("\n")[0])
    ap.add_argument("command", choices=["check", "show"])
    ap.add_argument("--cases", default=str(CASES))
    args = ap.parse_args(argv)
    if args.command == "show":
        print(json.dumps({c.id: computed(c) for c in load_cases(args.cases)}, indent=2))
        return 0
    problems = check(args.cases)
    for p in problems:
        print(p)
    print("ok" if not problems else f"{len(problems)} difference(s)")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
