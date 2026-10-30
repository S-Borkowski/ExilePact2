"""tools/crafting_helper/slot_model.py: the slot model, its case file and its spec.

The model is built from `docs/models/crafting-slots-spec.md` alone and holds
the engine to the spec through the shared case file
`apps/crafting-helper/crates/ch-engine/tests/items/slots/cases.json`.

Baselines (how things stand without the v2 routes):

- an Exalted Orb with no side omen can land in the prefix slot the owner's
  2P/1S craft still needs (`test_baseline_an_unrestricted_exalt_...`);
- the listed `fill-desecrate-exalt`, read as written, names a plain
  `exalted-orb` first on that craft;
- its starts reject a rare with one target (#28);
- an item that already has its desecrated modifier keeps its slam (#32's
  second test: `test_a_desecrated_item_keeps_its_slam`).

Targets: every case projects as `cases.json` records, the records keep what
the spec's "Representative crafts" says must hold (written out here, apart
from the case file, so neither can drift to match the other), and the
spec's invariants hold over every case and route. Each invariant keeps a
control beside it: an item or a blocked list on which the rule has to act,
or a mutated record the checker has to refuse.
"""
import copy
import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from tools.crafting_helper import slot_model as sm  # noqa: E402

CASES = ROOT / "apps" / "crafting-helper" / "crates" / "ch-engine" / "tests" / "items" / "slots" / "cases.json"
SPEC = ROOT / "docs" / "models" / "crafting-slots-spec.md"

V2 = [
    "close-a-side",
    "essence-desecrate-double-slam",
    "magic-augment-essence",
    "rare-clean-then-slam",
]
OWNERS_CRAFTS = [
    "rare-2p1s-prefix-missing",
    "magic-two-good",
    "rare-fractured",
    "rare-one-target",
    "rare-no-target",
    "magic-one-good",
    "magic-good-and-bad",
]
# The whittle's positive control and its blocked pair: required cases, so
# neither can be dropped from the case file silently.
WHITTLE_CONTROLS = ["rare-whittle-certain", "rare-whittle-blocked"]
# The last-target desecration (#32): where it cannot apply (a blocked
# Necromancy omen, no pool on the side, a desecrated target already on the
# item) and its wrong-reveal loop.
LAST_TARGET_CASES = [
    "rare-2p1s-necromancy-blocked",
    "body-armour-prefix-no-pool",
    "rare-desecrated-target",
    "rare-wrong-reveal",
]
# Certain removals first (2026-10-30): a certain removal later in file
# order taken before an earlier candidate, and its control, whose removal
# may take a target.
CERTAIN_FIRST_CASES = [
    "rare-certain-removal-first",
    "rare-uncertain-removal-keeps-order",
]
# A wrong reveal waits for the same-side exalt (2026-10-30, #37): Omen of
# Light passed over while a prefix exalt can still place a target there.
LIGHT_WAITS_CASES = [
    "rare-wrong-reveal-same-side-exalt",
]

GREATER ="omen-of-greater-exaltation"
SIN_EX, DEX_EX = "omen-of-sinistral-exaltation", "omen-of-dextral-exaltation"
SIN_ANN, DEX_ANN = "omen-of-sinistral-annulment", "omen-of-dextral-annulment"
SIN_NEC, DEX_NEC = "omen-of-sinistral-necromancy", "omen-of-dextral-necromancy"
ECHOES = "omen-of-abyssal-echoes"
LIGHT = "omen-of-light"
ESSENCE = "<the case's essence>"

# The steps the last-target desecration takes on a prefix (suffix).
DESECRATE_PREFIX = [("desecrate", "prefix", "desecrate", [SIN_NEC]), ("reveal", None, "reveal", [ECHOES])]
DESECRATE_SUFFIX = [("desecrate", "suffix", "desecrate", [DEX_NEC]), ("reveal", None, "reveal", [ECHOES])]
# The owner's craft before the rule, and wherever it cannot apply.
CLOSE_THEN_SLAM = [
    ("fill", "suffix", "exalted-orb", [GREATER, DEX_EX]),
    ("add-wanted", "prefix", "perfect-exalted-orb", []),
]
SIDE_SLAM_PREFIX = [("add-wanted", "prefix", "perfect-exalted-orb", [SIN_EX])]
LIGHT_OFF = ("remove-desecrated", None, "orb-of-annulment", [LIGHT], True)

# What the spec's "Representative crafts" says must hold, step by step:
# (goal, side, currency, omens[, certain]); `done` when the route must end
# there, `False` when only these first steps are pinned.
MUST_HOLD = {
    ("rare-2p1s-prefix-missing", "close-a-side"): (DESECRATE_PREFIX, True),
    ("rare-2p1s-prefix-missing", "essence-desecrate-double-slam"): (DESECRATE_PREFIX, True),
    ("rare-2p1s-prefix-missing", "rare-clean-then-slam"): (DESECRATE_PREFIX, True),
    ("rare-2p1s-prefix-missing", "magic-augment-essence"): "no-start-fits",
    ("rare-2p1s-necromancy-blocked", "close-a-side"): (CLOSE_THEN_SLAM, True),
    ("rare-2p1s-necromancy-blocked", "essence-desecrate-double-slam"): ([
        ("desecrate", "prefix", "desecrate", []),
        ("reveal", None, "reveal", [ECHOES]),
    ], True),
    ("rare-2p1s-necromancy-blocked", "rare-clean-then-slam"): (SIDE_SLAM_PREFIX, True),
    ("rare-2p1s-necromancy-blocked", "magic-augment-essence"): "no-start-fits",
    ("body-armour-prefix-no-pool", "close-a-side"): (CLOSE_THEN_SLAM, True),
    ("body-armour-prefix-no-pool", "essence-desecrate-double-slam"): (DESECRATE_PREFIX, True),
    ("body-armour-prefix-no-pool", "rare-clean-then-slam"): (SIDE_SLAM_PREFIX, True),
    ("body-armour-prefix-no-pool", "magic-augment-essence"): "no-start-fits",
    ("rare-desecrated-target", "close-a-side"): (CLOSE_THEN_SLAM, True),
    ("rare-desecrated-target", "essence-desecrate-double-slam"): (SIDE_SLAM_PREFIX, True),
    ("rare-desecrated-target", "rare-clean-then-slam"): (SIDE_SLAM_PREFIX, True),
    ("rare-desecrated-target", "magic-augment-essence"): "no-start-fits",
    ("rare-wrong-reveal", "close-a-side"): ([LIGHT_OFF] + DESECRATE_PREFIX, True),
    ("rare-wrong-reveal", "essence-desecrate-double-slam"): ([LIGHT_OFF] + DESECRATE_PREFIX, True),
    ("rare-wrong-reveal", "rare-clean-then-slam"): ([LIGHT_OFF] + DESECRATE_PREFIX, True),
    ("rare-wrong-reveal", "magic-augment-essence"): "no-start-fits",
    ("rare-wrong-reveal-same-side-exalt", "close-a-side"): (CLOSE_THEN_SLAM, True),
    ("rare-wrong-reveal-same-side-exalt", "essence-desecrate-double-slam"): (SIDE_SLAM_PREFIX, True),
    ("rare-wrong-reveal-same-side-exalt", "rare-clean-then-slam"): (SIDE_SLAM_PREFIX, True),
    ("rare-wrong-reveal-same-side-exalt", "magic-augment-essence"): "no-start-fits",
    ("magic-two-good", "essence-desecrate-double-slam"): ([
        ("add-wanted", None, ESSENCE, []),
        ("desecrate", "prefix", "desecrate", [SIN_NEC]),
        ("reveal", None, "reveal", [ECHOES]),
        ("add-wanted", "suffix", "perfect-exalted-orb", [GREATER, DEX_EX]),
    ], True),
    ("magic-two-good", "close-a-side"): "no-start-fits",
    ("magic-two-good", "rare-clean-then-slam"): "no-start-fits",
    ("magic-two-good", "magic-augment-essence"): ([
        ("add-wanted", None, ESSENCE, []),
        ("add-wanted", "suffix", "perfect-exalted-orb", [GREATER, DEX_EX]),
    ] + DESECRATE_PREFIX, True),
    ("rare-fractured", "close-a-side"): ([
        ("remove-unwanted", "prefix", "orb-of-annulment", [SIN_ANN], True),
    ] + DESECRATE_PREFIX, True),
    ("rare-fractured", "essence-desecrate-double-slam"): ([
        ("remove-unwanted", "prefix", "orb-of-annulment", [SIN_ANN], True),
    ] + DESECRATE_PREFIX, True),
    ("rare-fractured", "rare-clean-then-slam"): ([
        ("remove-unwanted", "prefix", "orb-of-annulment", [SIN_ANN], True),
    ] + DESECRATE_PREFIX, True),
    ("rare-fractured", "magic-augment-essence"): "no-start-fits",
    ("rare-one-target", "rare-clean-then-slam"): ([
        ("remove-unwanted", "prefix", "orb-of-annulment", []),
        ("add-wanted", "prefix", "perfect-exalted-orb", [GREATER, SIN_EX]),
    ] + DESECRATE_SUFFIX, True),
    ("rare-one-target", "close-a-side"): "no-start-fits",
    ("rare-no-target", "rare-clean-then-slam"): ([
        ("remove-unwanted", "prefix", "orb-of-annulment", []),
        ("remove-unwanted", "suffix", "orb-of-annulment", []),
        ("add-wanted", "prefix", "perfect-exalted-orb", [GREATER, SIN_EX]),
        ("add-wanted", "suffix", "perfect-exalted-orb", [GREATER, DEX_EX]),
    ], True),
    ("magic-one-good", "magic-augment-essence"): ([
        ("add-wanted", None, "perfect-orb-of-augmentation", []),
        ("add-wanted", None, ESSENCE, []),
    ], True),
    ("magic-good-and-bad", "magic-augment-essence"): ([
        ("remove-unwanted", "suffix", "orb-of-annulment", [DEX_ANN], True),
    ], False),
    # certain removals first: the certain suffix annulment goes before the
    # essence, which is first in file order (it used to come after it)
    ("magic-good-and-bad", "essence-desecrate-double-slam"): ([
        ("remove-unwanted", "suffix", "orb-of-annulment", [DEX_ANN], True),
        ("add-wanted", None, ESSENCE, []),
        ("desecrate", "suffix", "desecrate", [DEX_NEC]),
        ("reveal", None, "reveal", [ECHOES]),
    ], True),
    ("rare-certain-removal-first", "essence-desecrate-double-slam"): ([
        ("remove-unwanted", "prefix", "orb-of-annulment", [SIN_ANN], True),
        ("reveal", None, "reveal", [ECHOES]),
        ("add-wanted", "prefix", "perfect-exalted-orb", [SIN_EX]),
    ], True),
    ("rare-uncertain-removal-keeps-order", "essence-desecrate-double-slam"): ([
        ("reveal", None, "reveal", [ECHOES]),
        ("remove-unwanted", "prefix", "orb-of-annulment", [SIN_ANN], False),
        ("add-wanted", "prefix", "perfect-exalted-orb", [SIN_EX]),
    ], True),
    ("rare-certain-removal-first", "magic-augment-essence"): "no-start-fits",
    ("rare-uncertain-removal-keeps-order", "magic-augment-essence"): "no-start-fits",
    ("rare-whittle-certain", "close-a-side"): ([
        ("remove-unwanted", "prefix", "chaos-orb", ["omen-of-whittling"], True),
    ], False),
    ("rare-whittle-blocked", "close-a-side"): ([
        ("remove-unwanted", "prefix", "orb-of-annulment", [SIN_ANN], False),
    ], False),
}


def load_cases():
    return sm.load_cases(CASES)


def case(case_id):
    for c in load_cases():
        if c.id == case_id:
            return c
    raise AssertionError(f"cases.json has no case {case_id!r}")


def every_route():
    for c in load_cases():
        for bp_id in V2:
            yield c, bp_id, sm.plan(sm.BLUEPRINTS[bp_id], c)


def with_changes(c, **changes):
    """A copy of case `c` with some fields replaced (a control's input)."""
    other = copy.deepcopy(c)
    for key, value in changes.items():
        setattr(other, key, value)
    return other


def steps_of(record):
    """A route record's steps as (goal, side, currency, omens) tuples."""
    return [(s["goal"], s["side"], s["currency"], s["omens"]) for s in record["steps"]]


def last_target_to_desecrate(state, acc):
    """The missing target the spec's last-target desecration must take on
    this item, written from the spec apart from the model's step list: a rare
    with no desecrated modifier and exactly one target missing, whose side
    has a free slot and a desecration pool, the bone and that side's
    Necromancy omen unblocked. None when the rule does not apply."""
    item = state.item
    if item.rarity != "rare" or any(m.desecrated for m in item.mods()):
        return None
    miss = sm.missing(state)
    if len(miss) != 1:
        return None
    t = miss[0]
    if t.side not in state.desecration_pool or acc[t.side].free <= 0:
        return None
    if {"desecrate", sm.SIDE_OMENS["desecrate"][t.side]} & state.blocked:
        return None
    return t


class Baseline(unittest.TestCase):
    def test_baseline_an_unrestricted_exalt_can_fill_the_reserved_prefix_slot(self):
        c = case("rare-2p1s-prefix-missing")
        st = sm.State.from_case(c)
        acc = sm.accounting(st)
        self.assertTrue(acc["prefix"].reserved)
        self.assertTrue(acc["suffix"].closable)
        self.assertEqual(sm.landing_sides(st, "exalted-orb", []), ["prefix", "suffix"])
        # control: a side omen keeps it off the reserved slot
        self.assertEqual(sm.landing_sides(st, "exalted-orb", [DEX_EX]), ["suffix"])

    def test_baseline_the_listed_fill_step_names_a_plain_exalted_orb(self):
        listed = sm.read_listed(sm.listed_blueprint_path())
        c = case("rare-2p1s-prefix-missing")
        got = sm.listed_first_step(listed, c)
        self.assertEqual(got, {"step": "fill", "currency": "exalted-orb", "omens": []})
        self.assertEqual(c.listed, got)

    def test_baseline_listed_starts_reject_a_rare_with_one_target(self):
        listed = sm.read_listed(sm.listed_blueprint_path())
        c = case("rare-one-target")
        got = sm.listed_first_step(listed, c)
        self.assertEqual(got, {"rejected": {"kind": "no-start-fits"}})
        self.assertEqual(c.listed, got)


class Cases(unittest.TestCase):
    def test_every_case_projects_as_recorded(self):
        for c in load_cases():
            for bp_id in V2:
                with self.subTest(case=c.id, blueprint=bp_id):
                    self.assertIn(bp_id, c.expect)
                    got = sm.plan(sm.BLUEPRINTS[bp_id], c).record()
                    self.assertEqual(got, c.expect[bp_id])
            if c.listed is not None:
                with self.subTest(case=c.id, blueprint="fill-desecrate-exalt"):
                    listed = sm.read_listed(sm.listed_blueprint_path())
                    self.assertEqual(sm.listed_first_step(listed, c), c.listed)

    def test_cases_cover_the_owners_crafts(self):
        cases = {c.id: c for c in load_cases()}
        self.assertEqual(sorted(cases), sorted(OWNERS_CRAFTS + WHITTLE_CONTROLS + LAST_TARGET_CASES
                                               + CERTAIN_FIRST_CASES + LIGHT_WAITS_CASES))
        raw = json.loads(CASES.read_text(encoding="utf-8"))
        self.assertIn("crafting-slots-spec.md", raw.get("spec", ""))

        # the owner's craft of 2026-10-29: two target prefixes, one target
        # suffix, a third prefix target missing
        owner = sm.State.from_case(cases["rare-2p1s-prefix-missing"])
        self.assertEqual(owner.item.rarity, "rare")
        self.assertEqual([m.role for m in owner.item.prefix], ["target", "target"])
        self.assertEqual([m.role for m in owner.item.suffix], ["target"])
        self.assertEqual([(t.side, t.pool) for t in sm.missing(owner)], [("prefix", "base")])

        for (case_id, bp_id), must in MUST_HOLD.items():
            c = cases[case_id]
            for source, record in (
                ("cases.json", c.expect[bp_id]),
                ("model", sm.plan(sm.BLUEPRINTS[bp_id], c).record()),
            ):
                with self.subTest(case=case_id, blueprint=bp_id, source=source):
                    if isinstance(must, str):
                        self.assertEqual(record, {"rejected": {"kind": must}})
                        continue
                    steps, done = must
                    self.assertNotIn("rejected", record)
                    if done:
                        self.assertTrue(record["complete"])
                        self.assertEqual(len(record["steps"]), len(steps))
                    for want, got in zip(steps, record["steps"]):
                        goal, side, currency, omens = want[:4]
                        if currency == ESSENCE:
                            currency = next(t.essence for t in c.targets if t.essence)
                        self.assertEqual(
                            (got["goal"], got["side"], got["currency"], got["omens"]),
                            (goal, side, currency, omens),
                        )
                        if len(want) > 4:
                            self.assertEqual(got["certain"], want[4])

    def test_a_desecrated_item_keeps_its_slam(self):
        # #32's second test: the item already carries its one desecrated
        # modifier (a target), so no route desecrates and each slams as
        # before the rule
        c = case("rare-desecrated-target")
        self.assertTrue(any(m.desecrated and m.is_target for m in c.item.mods()))
        for bp_id in V2:
            record = sm.plan(sm.BLUEPRINTS[bp_id], c).record()
            with self.subTest(blueprint=bp_id):
                for s in record.get("steps", []):
                    self.assertNotEqual(s["goal"], "desecrate")
        self.assertEqual(steps_of(sm.plan(sm.BLUEPRINTS["close-a-side"], c).record()), CLOSE_THEN_SLAM)
        self.assertEqual(steps_of(sm.plan(sm.BLUEPRINTS["rare-clean-then-slam"], c).record()),
                         SIDE_SLAM_PREFIX)
        # control: the same item with that modifier not desecrated, the last
        # target is desecrated and revealed
        plain = copy.deepcopy(c)
        plain.item.prefix[0].desecrated = False
        for bp_id in ("close-a-side", "rare-clean-then-slam"):
            with self.subTest(control=bp_id):
                self.assertEqual(steps_of(sm.plan(sm.BLUEPRINTS[bp_id], plain).record()),
                                 DESECRATE_PREFIX)
        # control: an unrevealed desecrated modifier counts too; it is revealed,
        # never joined by a second desecration
        waiting = copy.deepcopy(c)
        waiting.item.prefix[0] = sm.Mod("unrevealed", "junk", desecrated=True, unrevealed=True,
                                        waits_for="p1")
        record = sm.plan(sm.BLUEPRINTS["rare-clean-then-slam"], waiting).record()
        self.assertEqual(steps_of(record), SIDE_SLAM_PREFIX + [("reveal", None, "reveal", [ECHOES])])


def candidates(c, bp_id):
    """Every candidate on case `c`'s item, in file order, as (step id,
    Choice): what the pick chooses from, apart from the pick itself."""
    bp = sm.BLUEPRINTS[bp_id]
    st = sm.State.from_case(c)
    sm.ensure_bindings(st, bp)
    acc = sm.accounting(st)
    out = []
    for step in bp.steps:
        got = sm.evaluate(st, step, acc)
        if isinstance(got, sm.Choice):
            out.append((step.id, got))
    return st, out


def first_pick(c, bp_id):
    bp = sm.BLUEPRINTS[bp_id]
    st = sm.State.from_case(c)
    sm.ensure_bindings(st, bp)
    choice, _ = sm.next_step(st, bp, sm.accounting(st))
    return choice


def annulment_certain(st, choice):
    """Written from the spec apart from the model: every modifier the
    annulment may take (at risk: not fractured, on the omen's side) is a
    non-target."""
    side = sm.omen_side(choice.omens[0]) if choice.omens else None
    return all(not m.is_target for s, m in st.item.placed()
               if not m.fractured and (side is None or s == side))


class CertainRemovalsFirst(unittest.TestCase):
    """Spec § "Certain removals first": a later `remove-unwanted` candidate
    whose chosen alternative is certain goes before an earlier candidate that
    is not a certain removal; a removal that may take a target keeps its
    file-order place. The guide and the projection pick alike."""

    def test_uncertain_removal_keeps_file_order(self):
        # baseline: what file order alone picks, and still picks when the
        # removal is not certain (the prefix target can be annulled)
        c = case("rare-uncertain-removal-keeps-order")
        st, cands = candidates(c, "essence-desecrate-double-slam")
        self.assertEqual([i for i, _ in cands], ["reveal", "remove"])
        remove = dict(cands)["remove"]
        self.assertEqual(remove.action.currency, "orb-of-annulment")
        self.assertFalse(annulment_certain(st, remove))
        self.assertEqual(first_pick(c, "essence-desecrate-double-slam").step.id, "reveal")
        record = sm.plan(sm.BLUEPRINTS["essence-desecrate-double-slam"], c).record()
        self.assertEqual(record["steps"][0]["goal"], "reveal")
        self.assertEqual(record["steps"][1]["certain"], False)

        # the same on a magic item: with Omen of Dextral Annulment blocked the
        # removal falls to a plain annulment, which may take the target
        # prefix, so the essence keeps its place before it
        mixed = with_changes(case("magic-good-and-bad"), blocked=frozenset({DEX_ANN}))
        st, cands = candidates(mixed, "essence-desecrate-double-slam")
        self.assertEqual([i for i, _ in cands], ["essence", "remove"])
        self.assertFalse(annulment_certain(st, dict(cands)["remove"]))
        self.assertEqual(first_pick(mixed, "essence-desecrate-double-slam").step.id, "essence")

    def test_certain_removal_goes_first(self):
        # the same item with the prefix target fractured: the reveal is still
        # the first candidate in file order, but the removal can only take
        # junk, so it is picked first
        c = case("rare-certain-removal-first")
        st, cands = candidates(c, "essence-desecrate-double-slam")
        self.assertEqual([i for i, _ in cands], ["reveal", "remove"])
        remove = dict(cands)["remove"]
        self.assertTrue(annulment_certain(st, remove))
        self.assertEqual(first_pick(c, "essence-desecrate-double-slam").step.id, "remove")
        record = sm.plan(sm.BLUEPRINTS["essence-desecrate-double-slam"], c).record()
        self.assertEqual([s["goal"] for s in record["steps"]],
                         ["remove-unwanted", "reveal", "add-wanted"])
        self.assertEqual(record["steps"][0]["certain"], True)

        # on a magic item: a certain Dextral annulment of the junk suffix goes
        # before the essence (it used to come after it)
        c = case("magic-good-and-bad")
        st, cands = candidates(c, "essence-desecrate-double-slam")
        self.assertEqual([i for i, _ in cands], ["essence", "remove"])
        self.assertTrue(annulment_certain(st, dict(cands)["remove"]))
        self.assertEqual(first_pick(c, "essence-desecrate-double-slam").step.id, "remove")


class LightWaits(unittest.TestCase):
    """Spec § "A wrong reveal waits for the same-side exalt": a
    `remove-desecrated` step guarded by `desecrated_unwanted` is passed over
    while an add is a candidate on the revealed desecrated junk's side with a
    free slot there; with that side full, Light comes first as before."""

    def test_wrong_reveal_waits_for_same_side_exalt(self):
        c = case("rare-wrong-reveal-same-side-exalt")
        junk = [m for m in c.item.prefix if m.desecrated]
        self.assertEqual([(m.role, m.unrevealed) for m in junk], [("junk", False)])
        acc = sm.accounting(sm.State.from_case(c))
        self.assertEqual(acc["prefix"].free, 1)

        for bp_id in ("essence-desecrate-double-slam", "rare-clean-then-slam"):
            light = "light" if bp_id == "essence-desecrate-double-slam" else "light-last"
            with self.subTest(blueprint=bp_id):
                # Light is a candidate on the item as it stands...
                _, cands = candidates(c, bp_id)
                self.assertEqual([i for i, _ in cands], [light, "side-slam"])
                # ...but waits: the exalt goes first, the junk kept through it
                self.assertEqual(first_pick(c, bp_id).step.id, "side-slam")
                route = sm.plan(sm.BLUEPRINTS[bp_id], c)
                self.assertEqual(steps_of(route.record()), SIDE_SLAM_PREFIX)
                self.assertTrue(route.complete)
                self.assertTrue(any(m.desecrated and m.alias == "junk-d1"
                                    for m in route.steps[-1].after.item.prefix))
                st = sm.State.from_case(c)
                sm.ensure_bindings(st, sm.BLUEPRINTS[bp_id])
                _, reasons = sm.next_step(st, sm.BLUEPRINTS[bp_id], sm.accounting(st))
                self.assertIn("waits for side-slam", reasons[light])
        # close-a-side closes the suffix first; Light still never comes
        record = sm.plan(sm.BLUEPRINTS["close-a-side"], c).record()
        self.assertEqual(steps_of(record), CLOSE_THEN_SLAM)

        # control: the side full (`rare-wrong-reveal`) keeps Light first
        full = case("rare-wrong-reveal")
        for bp_id in ("close-a-side", "essence-desecrate-double-slam", "rare-clean-then-slam"):
            with self.subTest(control=bp_id):
                self.assertEqual(steps_of(sm.plan(sm.BLUEPRINTS[bp_id], full).record()),
                                 [LIGHT_OFF[:4]] + DESECRATE_PREFIX)
        # control: with Sinistral Exaltation blocked no add can place the
        # prefix, so Light comes first and the bone is used again
        no_side = with_changes(c, blocked=frozenset({SIN_EX}))
        record = sm.plan(sm.BLUEPRINTS["rare-clean-then-slam"], no_side).record()
        self.assertEqual(steps_of(record), [LIGHT_OFF[:4]] + DESECRATE_PREFIX)
        # control: a desecrated target (not junk) never makes anything wait;
        # Light's guard is false
        hit = copy.deepcopy(c)
        hit.item.prefix[1].role = "target"
        hit.item.prefix[1].alias = "p2"
        self.assertTrue(sm.plan(sm.BLUEPRINTS["rare-clean-then-slam"], hit).record()["complete"])
        self.assertIsNone(sm.light_waits(sm.State.from_case(hit),
                                         sm.BLUEPRINTS["rare-clean-then-slam"],
                                         sm.LIGHT_LAST, sm.accounting(sm.State.from_case(hit))))


class Invariants(unittest.TestCase):
    def test_no_unrestricted_exalt_while_a_reserved_side_shares_the_open_slots(self):
        for c, bp_id, route in every_route():
            for step in route.steps:
                if sm.kind(step.record["currency"]) != "exalt":
                    continue
                with self.subTest(case=c.id, blueprint=bp_id, step=step.record):
                    if sm.two_open_one_reserved(step.accounting):
                        self.assertTrue(any(sm.omen_side(o) for o in step.record["omens"]))

        owner = case("rare-2p1s-prefix-missing")
        plain_first = sm.Blueprint(
            id="test-plain-first",
            classes=sm.EQUIPMENT_CLASSES,
            starts=[{"rarity": ["rare"]}],
            steps=[sm.Step("slam", "add-wanted", side="target", actions=(
                sm.Action("perfect-exalted-orb"),
                sm.Action("perfect-exalted-orb", ("side-omen",)),
            ))],
        )
        # the rule acts: the plain alternative is passed over for the side omen
        first = sm.plan(plain_first, owner).record()["steps"][0]
        self.assertEqual(first["omens"], [SIN_EX])
        # with no other alternative the route cannot finish
        plain_only = copy.deepcopy(plain_first)
        plain_only.steps[0] = sm.Step("slam", "add-wanted", side="target",
                                      actions=(sm.Action("perfect-exalted-orb"),))
        route = sm.plan(plain_only, owner)
        self.assertEqual(route.record(), {"rejected": {"kind": "route-cannot-finish"}})
        self.assertEqual(route.why, "no-candidate")
        # control: with one side open the plain exalt fits (close-a-side's last
        # slam, where the last-target desecration cannot apply)
        last = sm.plan(sm.BLUEPRINTS["close-a-side"], case("rare-2p1s-necromancy-blocked")).record()["steps"][-1]
        self.assertEqual((last["currency"], last["omens"]), ("perfect-exalted-orb", []))

    def test_side_annulment_only_when_the_other_side_holds_a_target(self):
        side_annuls = 0
        for c, bp_id, route in every_route():
            for step in route.steps:
                side = next((sm.omen_side(o) for o in step.record["omens"]
                             if o in (SIN_ANN, DEX_ANN)), None)
                if side is None:
                    continue
                side_annuls += 1
                with self.subTest(case=c.id, blueprint=bp_id, step=step.record):
                    self.assertEqual(side, step.record["side"])
                    other = step.before.item.side(sm.other(side))
                    self.assertTrue(any(m.is_target and not m.fractured for m in other))
        self.assertGreater(side_annuls, 0)

        # control: no target on the other side, plain annulment (#28)
        first = sm.plan(sm.BLUEPRINTS["rare-clean-then-slam"], case("rare-one-target")).record()["steps"][0]
        self.assertEqual((first["currency"], first["omens"], first["certain"]),
                         ("orb-of-annulment", [], False))
        # control: Sinistral Annulment blocked, the plain one, no longer certain
        c = case("rare-fractured")
        blocked = with_changes(c, blocked=frozenset({SIN_ANN}))
        first = sm.plan(sm.BLUEPRINTS["rare-clean-then-slam"], blocked).record()["steps"][0]
        self.assertEqual((first["side"], first["currency"], first["omens"], first["certain"]),
                         ("prefix", "orb-of-annulment", [], False))

    def test_whittle_only_when_certain_and_an_annulment_risks_a_target(self):
        for c, bp_id, route in every_route():
            for step in route.steps:
                if step.record["currency"] != "chaos-orb":
                    continue
                with self.subTest(case=c.id, blueprint=bp_id, step=step.record):
                    self.assertIsNotNone(sm.whittle_certain(step.before, step.record["side"]))

        # positive: the lowest-level modifier is a lone junk prefix, and the
        # Sinistral Annulment that would otherwise fit could take the target prefix
        whittle = sm.case_from_dict({
            "id": "test-whittle", "class": "Rings", "rarity": "rare",
            "prefixes": [
                {"alias": "p1", "role": "target", "level": 60},
                {"alias": "junk-p1", "role": "junk", "level": 5},
                {"alias": "junk-p2", "role": "junk", "level": 50},
            ],
            "suffixes": [{"alias": "s1", "role": "target", "level": 40}],
            "targets": [
                {"alias": "p1", "side": "prefix", "pool": "base", "essence": None},
                {"alias": "p2", "side": "prefix", "pool": "base", "essence": None},
                {"alias": "s1", "side": "suffix", "pool": "base", "essence": None},
            ],
            "blocked": [], "hardest": "p2", "expect": {},
        })
        route = sm.plan(sm.BLUEPRINTS["rare-clean-then-slam"], whittle)
        first = route.record()["steps"][0]
        self.assertEqual((first["side"], first["currency"], first["omens"], first["certain"]),
                         ("prefix", "chaos-orb", ["omen-of-whittling"], True))
        self.assertNotIn("junk-p1", [m.alias for m in route.steps[0].after.item.prefix])
        # Whittling blocked: the side annulment instead
        blocked = with_changes(whittle, blocked=frozenset({"omen-of-whittling"}))
        first = sm.plan(sm.BLUEPRINTS["rare-clean-then-slam"], blocked).record()["steps"][0]
        self.assertEqual((first["currency"], first["omens"]), ("orb-of-annulment", [SIN_ANN]))
        # a tie at the lowest level: not certain
        tied = copy.deepcopy(whittle)
        tied.item.prefix[2].level = 5
        first = sm.plan(sm.BLUEPRINTS["rare-clean-then-slam"], tied).record()["steps"][0]
        self.assertEqual(first["currency"], "orb-of-annulment")
        # certain, but no annulment could take a target (#28's belt): annul
        st = sm.State.from_case(case("rare-no-target"))
        self.assertIsNotNone(sm.whittle_certain(st, "prefix"))
        first = sm.plan(sm.BLUEPRINTS["rare-clean-then-slam"], case("rare-no-target")).record()["steps"][0]
        self.assertEqual(first["currency"], "orb-of-annulment")
        # the lowest-level modifier is a target: not certain
        self.assertIsNone(sm.whittle_certain(sm.State.from_case(case("rare-fractured")), "prefix"))

    def test_fractured_modifiers_are_never_removable(self):
        for c, bp_id, route in every_route():
            fractured = [m.alias for m in sm.State.from_case(c).item.mods() if m.fractured]
            for step in route.steps:
                with self.subTest(case=c.id, blueprint=bp_id, step=step.record):
                    after = [m.alias for m in step.after.item.mods()]
                    self.assertTrue(all(a in after for a in fractured))

        st = sm.State.from_case(case("rare-fractured"))
        self.assertFalse(sm.removable(st.item.prefix[0]))
        at_risk = sm.at_risk(st, "orb-of-annulment", [SIN_ANN])
        self.assertEqual([m.alias for _, m in at_risk], ["junk-p1", "junk-p2"])

        # control: a fractured junk prefix does not make its side blocked,
        # and no step offers to remove it
        stuck = sm.case_from_dict({
            "id": "test-fractured-junk", "class": "Rings", "rarity": "rare",
            "prefixes": [
                {"alias": "junk-p1", "role": "junk", "fractured": True, "level": 30},
                {"alias": "p1", "role": "target", "level": 50},
            ],
            "suffixes": [],
            "targets": [
                {"alias": "p1", "side": "prefix", "pool": "base", "essence": None},
                {"alias": "p2", "side": "prefix", "pool": "base", "essence": None},
                {"alias": "p3", "side": "prefix", "pool": "base", "essence": None},
            ],
            "blocked": [], "hardest": None, "expect": {},
        })
        acc = sm.accounting(sm.State.from_case(stuck))
        self.assertLess(acc["prefix"].spare, 0)
        self.assertFalse(acc["prefix"].blocked)
        route = sm.plan(sm.BLUEPRINTS["rare-clean-then-slam"], stuck)
        self.assertEqual(route.record(), {"rejected": {"kind": "route-cannot-finish"}})
        self.assertEqual(route.why, "no-candidate")
        self.assertTrue(all(s.record["goal"] != "remove-unwanted" for s in route.steps))

    def test_greater_exaltation_only_where_two_fit(self):
        greaters = 0
        for c, bp_id, route in every_route():
            for step in route.steps:
                if GREATER not in step.record["omens"]:
                    continue
                greaters += 1
                with self.subTest(case=c.id, blueprint=bp_id, step=step.record):
                    side = step.record["side"]
                    self.assertGreaterEqual(step.accounting[side].free, 2)
                    if step.record["goal"] == "add-wanted":
                        placeable = [t for t in sm.placeable(step.before, step.step) if t.side == side]
                        self.assertGreaterEqual(len(placeable), 2)
                    else:
                        self.assertEqual(step.record["goal"], "fill")
        self.assertGreater(greaters, 0)

        # control: one free prefix slot, no Greater
        first = sm.plan(sm.BLUEPRINTS["rare-clean-then-slam"], case("rare-2p1s-necromancy-blocked")).record()["steps"][0]
        self.assertEqual((first["goal"], first["side"]), ("add-wanted", "prefix"))
        self.assertNotIn(GREATER, first["omens"])
        # control: Greater blocked, one prefix per slam (the suffix, then the
        # last target, is desecrated)
        c = with_changes(case("rare-one-target"), blocked=frozenset({GREATER}))
        steps = sm.plan(sm.BLUEPRINTS["rare-clean-then-slam"], c).record()["steps"]
        self.assertEqual([s["omens"] for s in steps[1:]], [[SIN_EX], [SIN_EX], [DEX_NEC], [ECHOES]])

    def test_every_projected_step_lists_all_its_omens(self):
        checked = 0
        for c, bp_id, route in every_route():
            for step in route.steps:
                checked += 1
                with self.subTest(case=c.id, blueprint=bp_id, step=step.record):
                    self.assertEqual(sm.omen_problems(step), [])
        self.assertGreater(checked, 0)

        # controls: a mutated record must be refused
        owner = case("rare-2p1s-prefix-missing")
        route = sm.plan(sm.BLUEPRINTS["close-a-side"], case("rare-2p1s-necromancy-blocked"))
        fill = copy.deepcopy(route.steps[0])
        self.assertEqual(fill.record["goal"], "fill")
        fill.record["omens"] = [GREATER]
        self.assertTrue(sm.omen_problems(fill))
        for bp_id in ("essence-desecrate-double-slam", "close-a-side"):
            route = sm.plan(sm.BLUEPRINTS[bp_id], owner)
            desecrate = copy.deepcopy(route.steps[0])
            self.assertEqual(desecrate.record["goal"], "desecrate")
            desecrate.record["omens"] = []
            self.assertTrue(sm.omen_problems(desecrate))
            wrong_side = copy.deepcopy(route.steps[0])
            wrong_side.record["omens"] = [DEX_NEC]
            self.assertTrue(sm.omen_problems(wrong_side))
        # the plain bone is no problem only where the player blocked the omen
        plain = sm.plan(sm.BLUEPRINTS["essence-desecrate-double-slam"], case("rare-2p1s-necromancy-blocked"))
        self.assertEqual(plain.steps[0].record["omens"], [])
        self.assertEqual(sm.omen_problems(plain.steps[0]), [])

    def test_the_last_target_is_desecrated_when_it_can_be(self):
        applied = 0
        for c, bp_id, route in every_route():
            for step in route.steps:
                t = last_target_to_desecrate(step.before, step.accounting)
                if t is None:
                    continue
                applied += 1
                with self.subTest(case=c.id, blueprint=bp_id, step=step.record):
                    self.assertEqual(
                        (step.record["goal"], step.record["side"], step.record["omens"]),
                        ("desecrate", t.side, [sm.SIDE_OMENS["desecrate"][t.side]]),
                    )
                    # its unrevealed modifier waits for that target, and the
                    # route's next step reveals it
                    unrevealed = [m for m in step.after.item.side(t.side) if m.unrevealed]
                    self.assertEqual([m.waits_for for m in unrevealed], [t.alias])
                    later = route.steps[route.steps.index(step) + 1:]
                    self.assertTrue(later)
                    self.assertEqual(later[0].record["goal"], "reveal")
        self.assertGreater(applied, 0)
        # each blueprint that had no desecration takes the rule's step somewhere
        took = {bp_id for _, bp_id, route in every_route()
                for s in route.steps if s.step.bind == "last-remaining" and s.record["goal"] == "desecrate"}
        self.assertLessEqual({"close-a-side", "magic-augment-essence", "rare-clean-then-slam"}, took)

        owner = case("rare-2p1s-prefix-missing")
        close = sm.BLUEPRINTS["close-a-side"]
        # controls where it must not apply: the bone or the Necromancy omen
        # blocked, and a last target outside the class's desecration pool
        for blocked in ({"desecrate"}, {SIN_NEC}):
            with self.subTest(blocked=blocked):
                c = with_changes(owner, blocked=frozenset(blocked))
                self.assertEqual(steps_of(sm.plan(close, c).record()), CLOSE_THEN_SLAM)
        no_pool = with_changes(owner, desecration_pool=frozenset({"suffix"}))
        self.assertEqual(steps_of(sm.plan(close, no_pool).record()), CLOSE_THEN_SLAM)
        # the other side's Necromancy omen blocked changes nothing
        c = with_changes(owner, blocked=frozenset({DEX_NEC}))
        self.assertEqual(steps_of(sm.plan(close, c).record()), DESECRATE_PREFIX)
        # two targets missing: no desecration until one is left (#28)
        record = sm.plan(sm.BLUEPRINTS["rare-clean-then-slam"], case("rare-one-target")).record()
        self.assertEqual([s["goal"] for s in record["steps"][:2]], ["remove-unwanted", "add-wanted"])
        # a magic item: the essence, never a bone
        record = sm.plan(sm.BLUEPRINTS["magic-augment-essence"], case("magic-one-good")).record()
        self.assertNotIn("desecrate", [s["goal"] for s in record["steps"]])

        # the binding is made afresh on every item, never kept: a miss that
        # leaves a second target missing unbinds it
        st = sm.State.from_case(owner)
        sm.ensure_bindings(st, close)
        self.assertEqual(st.bindings.get("last-remaining"), "p3")
        st.item.prefix.pop(1)
        sm.ensure_bindings(st, close)
        self.assertNotIn("last-remaining", st.bindings)

        # a wrong reveal: Omen of Light takes it, then the bone again
        record = sm.plan(close, case("rare-wrong-reveal")).record()
        self.assertEqual(record["steps"][0]["certain"], True)
        self.assertEqual(steps_of(record), [LIGHT_OFF[:4]] + DESECRATE_PREFIX)


class Spec(unittest.TestCase):
    def test_spec_facts_carry_a_label_and_a_source(self):
        text = SPEC.read_text(encoding="utf-8")
        rows = sm.spec_facts(text)
        self.assertGreaterEqual(len(rows), 20)
        for row in rows:
            with self.subTest(fact=row["id"]):
                self.assertEqual(sm.fact_problems(row), [])
        not_established = {r["id"] for r in rows if r["label"] == "not established"}
        self.assertIn("greater-with-side-omen", not_established)

        # each blueprint relies on exactly the facts the spec's table says,
        # and only on not-established ones
        table = sm.relied_on_table(text)
        self.assertEqual(set(table), not_established)
        for bp_id in V2:
            with self.subTest(blueprint=bp_id):
                relies = set(sm.BLUEPRINTS[bp_id].relies_on)
                self.assertLessEqual(relies, not_established)
                self.assertEqual(relies, {f for f, bps in table.items() if bp_id in bps})

        # controls: rows the check must refuse
        good = rows[0]
        for bad in (
            dict(good, label="community"),
            dict(good, source="https://poe2db.tw/us/Omen"),
            dict(good, source="poe2db, fetched 2026-10-29"),
            dict(good, fact=""),
        ):
            with self.subTest(bad=bad):
                self.assertTrue(sm.fact_problems(bad))


if __name__ == "__main__":
    unittest.main()
