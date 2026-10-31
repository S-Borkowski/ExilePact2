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
# A side Exaltation omen with the other side full (2026-10-31, #42): the
# exalt can only land on the open side, so the omen is left out; with the
# Dextral omen blocked the exalt is still taken.
SIDE_EXALT_FULL_CASES = [
    "rare-prefixes-full",
    "rare-prefixes-full-dextral-blocked",
    "rare-suffixes-full",
]

GREATER ="omen-of-greater-exaltation"
SIN_EX, DEX_EX = "omen-of-sinistral-exaltation", "omen-of-dextral-exaltation"
SIN_ANN, DEX_ANN = "omen-of-sinistral-annulment", "omen-of-dextral-annulment"
SIN_NEC, DEX_NEC = "omen-of-sinistral-necromancy", "omen-of-dextral-necromancy"
ECHOES = "omen-of-abyssal-echoes"
LIGHT = "omen-of-light"
SOVEREIGN, LIEGE = "omen-of-the-sovereign", "omen-of-the-liege"
ESSENCE = "<the case's essence>"
# The lord omens and the classes they work on, written from the spec's
# § "`lord-omen`" apart from the model's own tables.
LORD_OMEN = {"ulaman": SOVEREIGN, "amanamu": LIEGE, "kurgal": "omen-of-the-blackblooded"}
LORD_CLASSES = {
    "Wands", "One Hand Maces", "Sceptres", "Spears", "Bows", "Staves", "Two Hand Maces",
    "Quarterstaves", "Crossbows", "Talismans", "Amulets", "Rings",
}
# The cases whose targets include a desecrated-pool one, each with its lord
# (the lord every tier of the real modifier the case's item text holds names).
LORD_TARGETS = {
    "rare-desecrated-target": ("p1", "ulaman"),
    "magic-two-good": ("p3", "amanamu"),
    "rare-certain-removal-first": ("s2", "ulaman"),
    "rare-uncertain-removal-keeps-order": ("s2", "ulaman"),
    "rare-suffixes-full": ("s3", "ulaman"),
}

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
# The slam on the one open side when the other is full (#42): no side
# Exaltation omen; Greater where two targets fit.
FULL_PREFIXES_SLAM = [("add-wanted", "suffix", "perfect-exalted-orb", [GREATER])]
FULL_SUFFIXES_SLAM = [("add-wanted", "prefix", "perfect-exalted-orb", [])]

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
        # the desecrated prefix is Amanamu's, on an amulet: Omen of the Liege
        ("desecrate", "prefix", "desecrate", [SIN_NEC, LIEGE]),
        ("reveal", None, "reveal", [ECHOES]),
        # the prefixes are full after the desecration, so the slam names no
        # Dextral Exaltation (it used to)
        ("add-wanted", "suffix", "perfect-exalted-orb", [GREATER]),
    ], True),
    ("magic-two-good", "close-a-side"): "no-start-fits",
    ("magic-two-good", "rare-clean-then-slam"): "no-start-fits",
    ("magic-two-good", "magic-augment-essence"): ([
        ("add-wanted", None, ESSENCE, []),
        ("add-wanted", "suffix", "perfect-exalted-orb", [GREATER, DEX_EX]),
        ("desecrate", "prefix", "desecrate", [SIN_NEC, LIEGE]),
        ("reveal", None, "reveal", [ECHOES]),
    ], True),
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
        # the prefixes are full after their slam: no Dextral Exaltation (it
        # used to name it)
        ("add-wanted", "suffix", "perfect-exalted-orb", [GREATER]),
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
    # a side Exaltation omen with the other side full: left out, the side
    # and Greater kept (#42)
    ("rare-prefixes-full", "rare-clean-then-slam"): (FULL_PREFIXES_SLAM, True),
    ("rare-prefixes-full", "close-a-side"): (FULL_PREFIXES_SLAM, True),
    ("rare-prefixes-full", "essence-desecrate-double-slam"): ([
        ("desecrate", "suffix", "desecrate", [DEX_NEC]),
        ("reveal", None, "reveal", [ECHOES]),
        ("add-wanted", "suffix", "perfect-exalted-orb", []),
    ], True),
    ("rare-prefixes-full", "magic-augment-essence"): "no-start-fits",
    ("rare-prefixes-full-dextral-blocked", "rare-clean-then-slam"): (FULL_PREFIXES_SLAM, True),
    ("rare-prefixes-full-dextral-blocked", "close-a-side"): (FULL_PREFIXES_SLAM, True),
    ("rare-prefixes-full-dextral-blocked", "essence-desecrate-double-slam"): ([
        ("desecrate", "suffix", "desecrate", [DEX_NEC]),
        ("reveal", None, "reveal", [ECHOES]),
        ("add-wanted", "suffix", "perfect-exalted-orb", []),
    ], True),
    ("rare-prefixes-full-dextral-blocked", "magic-augment-essence"): "no-start-fits",
    ("rare-suffixes-full", "rare-clean-then-slam"): (FULL_SUFFIXES_SLAM, True),
    ("rare-suffixes-full", "essence-desecrate-double-slam"): (FULL_SUFFIXES_SLAM, True),
    ("rare-suffixes-full", "close-a-side"): (FULL_SUFFIXES_SLAM, True),
    ("rare-suffixes-full", "magic-augment-essence"): "no-start-fits",
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


def lord_omen_for(state, t):
    """The lord omen the spec's `lord-omen` gives a desecration bound to
    target `t` on this item, written from the spec apart from the model:
    a desecrated-pool target with one lord, a lord class, the omen not
    blocked. None otherwise."""
    if t.pool != "desecrated" or t.lord is None or state.cls not in LORD_CLASSES:
        return None
    omen = LORD_OMEN[t.lord]
    return None if omen in state.blocked else omen


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
                                               + CERTAIN_FIRST_CASES + LIGHT_WAITS_CASES
                                               + SIDE_EXALT_FULL_CASES))
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


def side_exalt_beside_full(acc, omens):
    """The side Exaltation omens in `omens` that name a side with a free
    slot while the other side has none, written from the spec's § "A side
    Exaltation omen with the other side full" apart from the model."""
    names = {SIN_EX: "prefix", DEX_EX: "suffix"}
    other = {"prefix": "suffix", "suffix": "prefix"}
    return [o for o in omens
            if o in names and acc[names[o]].free > 0 and acc[other[names[o]]].free <= 0]


class SideExaltationFullOtherSide(unittest.TestCase):
    """Spec § "A side Exaltation omen with the other side full" (#42): on an
    Exalted Orb, a Sinistral or Dextral Exaltation omen is left out while the
    side it names has a free slot and the other side has none; the exalt
    lands on the open side anyway. The side and Greater stay."""

    def test_baseline_side_exaltation_omen_kept_while_the_other_side_has_room(self):
        # the owner's craft with Necromancy blocked: one free prefix slot,
        # two free suffix slots; both slams keep their side omen
        c = case("rare-2p1s-necromancy-blocked")
        acc = sm.accounting(sm.State.from_case(c))
        self.assertEqual((acc["prefix"].free, acc["suffix"].free), (1, 2))
        self.assertEqual(steps_of(sm.plan(sm.BLUEPRINTS["close-a-side"], c).record())[0],
                         ("fill", "suffix", "exalted-orb", [GREATER, DEX_EX]))
        self.assertEqual(steps_of(sm.plan(sm.BLUEPRINTS["rare-clean-then-slam"], c).record()),
                         SIDE_SLAM_PREFIX)
        # #28's belt: the prefix slam comes while the suffixes still have room
        steps = steps_of(sm.plan(sm.BLUEPRINTS["rare-clean-then-slam"], case("rare-no-target")).record())
        self.assertEqual(steps[2], ("add-wanted", "prefix", "perfect-exalted-orb", [GREATER, SIN_EX]))
        # the suffix slam on the magic start, the prefixes not yet full
        steps = steps_of(sm.plan(sm.BLUEPRINTS["magic-augment-essence"], case("magic-two-good")).record())
        self.assertEqual(steps[1], ("add-wanted", "suffix", "perfect-exalted-orb", [GREATER, DEX_EX]))
        # a literal side omen is kept the same way (a fill: the one missing
        # target is a prefix)
        st = sm.State.from_case(c)
        literal = sm.Step("literal", "fill", side="suffix",
                          actions=(sm.Action("exalted-orb", (DEX_EX,)),))
        got = sm.evaluate(st, literal, sm.accounting(st))
        self.assertIsInstance(got, sm.Choice)
        self.assertEqual(got.omens, [DEX_EX])

    def test_full_other_side_drops_the_side_exaltation_omen(self):
        full = case("rare-prefixes-full")
        st = sm.State.from_case(full)
        acc = sm.accounting(st)
        self.assertEqual((acc["prefix"].free, acc["suffix"].free), (0, 2))
        for bp_id in ("rare-clean-then-slam", "close-a-side"):
            with self.subTest(blueprint=bp_id):
                route = sm.plan(sm.BLUEPRINTS[bp_id], full)
                self.assertTrue(route.complete)
                self.assertEqual(steps_of(route.record()), FULL_PREFIXES_SLAM)
                # both suffixes land on the open side
                self.assertEqual({m.alias for m in route.steps[-1].after.item.suffix},
                                 {"s1", "s2", "s3"})
        # the last suffix slam after the desecration and its reveal; the
        # desecration beside the full side keeps its Necromancy omen
        steps = steps_of(sm.plan(sm.BLUEPRINTS["essence-desecrate-double-slam"], full).record())
        self.assertEqual(steps[0], ("desecrate", "suffix", "desecrate", [DEX_NEC]))
        self.assertEqual(steps[-1], ("add-wanted", "suffix", "perfect-exalted-orb", []))
        # the other way round: the suffixes full, one prefix missing, no Greater
        for bp_id in ("rare-clean-then-slam", "essence-desecrate-double-slam", "close-a-side"):
            with self.subTest(case="rare-suffixes-full", blueprint=bp_id):
                record = sm.plan(sm.BLUEPRINTS[bp_id], case("rare-suffixes-full")).record()
                self.assertEqual(steps_of(record), FULL_SUFFIXES_SLAM)
        # a later slam whose other side an earlier step filled
        steps = steps_of(sm.plan(sm.BLUEPRINTS["rare-clean-then-slam"], case("rare-no-target")).record())
        self.assertEqual(steps[3], ("add-wanted", "suffix", "perfect-exalted-orb", [GREATER]))
        steps = steps_of(sm.plan(sm.BLUEPRINTS["essence-desecrate-double-slam"], case("magic-two-good")).record())
        self.assertEqual(steps[3], ("add-wanted", "suffix", "perfect-exalted-orb", [GREATER]))
        # a literal side omen is left out too
        literal = sm.Step("literal", "add-wanted", side="suffix",
                          actions=(sm.Action("exalted-orb", (DEX_EX,)),))
        got = sm.evaluate(st, literal, acc)
        self.assertIsInstance(got, sm.Choice)
        self.assertEqual(got.omens, [])
        # control: an omen naming the full side itself is not dropped; the
        # exalt has nowhere to land (today's behaviour)
        wrong = sm.Step("wrong", "add-wanted", actions=(sm.Action("exalted-orb", (SIN_EX,)),))
        self.assertEqual(sm.fit(st, wrong, None, 0, acc), (None, "no free slot where it lands"))

    def test_blocked_redundant_exaltation_omen_does_not_pass_the_exalt_over(self):
        blocked = case("rare-prefixes-full-dextral-blocked")
        self.assertIn(DEX_EX, blocked.blocked)
        for bp_id in ("rare-clean-then-slam", "close-a-side"):
            with self.subTest(blueprint=bp_id):
                route = sm.plan(sm.BLUEPRINTS[bp_id], blocked)
                self.assertTrue(route.complete)
                self.assertEqual(steps_of(route.record()), FULL_PREFIXES_SLAM)
        steps = steps_of(sm.plan(sm.BLUEPRINTS["essence-desecrate-double-slam"], blocked).record())
        self.assertEqual(steps[-1], ("add-wanted", "suffix", "perfect-exalted-orb", []))
        # the same routes as on the item with nothing blocked
        free = case("rare-prefixes-full")
        for bp_id in sm.BLUEPRINTS:
            with self.subTest(same=bp_id):
                self.assertEqual(sm.plan(sm.BLUEPRINTS[bp_id], blocked).record(),
                                 sm.plan(sm.BLUEPRINTS[bp_id], free).record())
        # control: while the other side has room the blocked omen is needed,
        # so every alternative of the suffix close is blocked
        roomy = with_changes(case("rare-2p1s-necromancy-blocked"),
                             blocked=frozenset({SIN_NEC, DEX_EX}))
        st = sm.State.from_case(roomy)
        bp = sm.BLUEPRINTS["close-a-side"]
        sm.ensure_bindings(st, bp)
        close = next(s for s in bp.steps if s.id == "close")
        self.assertEqual(sm.evaluate(st, close, sm.accounting(st)), "every alternative blocked")

    def test_side_exaltation_omen_beside_a_full_side_is_an_omen_problem(self):
        exalts = kept = 0
        for c, bp_id, route in every_route():
            for step in route.steps:
                if step.record["currency"] not in sm.EXALTS:
                    continue
                exalts += 1
                kept += any(o in (SIN_EX, DEX_EX) for o in step.record["omens"])
                with self.subTest(case=c.id, blueprint=bp_id, step=step.record):
                    self.assertEqual(side_exalt_beside_full(step.accounting, step.record["omens"]), [])
                    # today's records are not refused
                    self.assertEqual(sm.omen_problems(step), [])
        self.assertGreater(exalts, 0)
        self.assertGreater(kept, 0)

        # controls: the omen put back beside a full side must be refused
        for case_id, bp_id, index, omen in (
            ("rare-prefixes-full", "rare-clean-then-slam", 0, DEX_EX),
            ("rare-suffixes-full", "rare-clean-then-slam", 0, SIN_EX),
            ("rare-no-target", "rare-clean-then-slam", 3, DEX_EX),
            ("magic-two-good", "essence-desecrate-double-slam", 3, DEX_EX),
        ):
            with self.subTest(control=(case_id, bp_id)):
                step = copy.deepcopy(sm.plan(sm.BLUEPRINTS[bp_id], case(case_id)).steps[index])
                self.assertEqual(sm.omen_problems(step), [])
                step.record["omens"] = step.record["omens"] + [omen]
                self.assertEqual(side_exalt_beside_full(step.accounting, step.record["omens"]), [omen])
                self.assertTrue(sm.omen_problems(step))
        # and the same omen beside a side with room is no problem
        fill = sm.plan(sm.BLUEPRINTS["close-a-side"], case("rare-2p1s-necromancy-blocked")).steps[0]
        self.assertEqual(fill.record["omens"], [GREATER, DEX_EX])
        self.assertEqual(sm.omen_problems(fill), [])


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
                    lord = lord_omen_for(step.before, t)
                    self.assertEqual(
                        (step.record["goal"], step.record["side"], step.record["omens"]),
                        ("desecrate", t.side, [sm.SIDE_OMENS["desecrate"][t.side]] + ([lord] if lord else [])),
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

    def test_lord_omen_for_a_desecrated_target(self):
        # the case file names each desecrated-pool target's lord, and no
        # base-pool target has one
        for case_id, (alias, lord) in LORD_TARGETS.items():
            with self.subTest(case=case_id):
                c = case(case_id)
                self.assertEqual({t.alias: t.lord for t in c.targets if t.pool == "desecrated"}, {alias: lord})
        for c in load_cases():
            self.assertEqual([t.alias for t in c.targets if t.pool == "base" and t.lord], [], c.id)

        # every projected desecration carries exactly the lord omen the spec
        # gives its bound target, after the Necromancy omen
        lorded = 0
        for c, bp_id, route in every_route():
            for step in route.steps:
                if step.record["goal"] != "desecrate":
                    continue
                bound = step.before.bindings.get(step.step.bind)
                t = sm.target_by_alias(step.before, bound) if bound else None
                lord = lord_omen_for(step.before, t) if t else None
                lorded += lord is not None
                with self.subTest(case=c.id, blueprint=bp_id, step=step.record):
                    lords = [o for o in step.record["omens"] if o in LORD_OMEN.values()]
                    self.assertEqual(lords, [lord] if lord else [])
                    if lord:
                        self.assertEqual(step.record["omens"][-1], lord)
                    self.assertEqual(sm.omen_problems(step), [])
        self.assertGreater(lorded, 0)

        two_good = case("magic-two-good")
        edds = sm.BLUEPRINTS["essence-desecrate-double-slam"]

        def desecration(c, bp=edds):
            steps = [s for s in steps_of(sm.plan(bp, c).record()) if s[0] == "desecrate"]
            self.assertEqual(len(steps), 1, c.id)
            return steps[0]

        self.assertEqual(desecration(two_good), ("desecrate", "prefix", "desecrate", [SIN_NEC, LIEGE]))
        self.assertEqual(desecration(two_good, sm.BLUEPRINTS["magic-augment-essence"])[3], [SIN_NEC, LIEGE])
        # controls: the omen blocked, a class outside the lord classes (a
        # belt, a body armour), a target whose tiers name no single lord:
        # the Necromancy omen alone, the step itself unchanged
        controls = {
            "liege blocked": with_changes(two_good, blocked=frozenset({LIEGE})),
            "belt": with_changes(two_good, cls="Belts"),
            "body armour": with_changes(two_good, cls="Body Armours"),
            "mixed lords": with_changes(two_good, targets=tuple(
                sm.Target(t.alias, t.side, t.pool, t.essence, None) for t in two_good.targets)),
        }
        for name, c in controls.items():
            with self.subTest(control=name):
                self.assertEqual(desecration(c), ("desecrate", "prefix", "desecrate", [SIN_NEC]))
        # another lord's omen blocked changes nothing
        c = with_changes(two_good, blocked=frozenset({SOVEREIGN}))
        self.assertEqual(desecration(c)[3], [SIN_NEC, LIEGE])
        # the Necromancy omen blocked: the fallback bone still names the
        # lord omen, alone; with the lord omen blocked too, no omen at all
        c = with_changes(two_good, blocked=frozenset({SIN_NEC}))
        self.assertEqual(desecration(c), ("desecrate", "prefix", "desecrate", [LIEGE]))
        c = with_changes(two_good, blocked=frozenset({SIN_NEC, LIEGE}))
        self.assertEqual(desecration(c), ("desecrate", "prefix", "desecrate", []))
        # a base-pool target on a ring never takes one (the owner's craft)
        owner = sm.plan(sm.BLUEPRINTS["close-a-side"], case("rare-2p1s-prefix-missing")).record()
        self.assertEqual(steps_of(owner), DESECRATE_PREFIX)

        # the checker refuses a lord omen dropped or added
        route = sm.plan(edds, two_good)
        step = next(s for s in route.steps if s.record["goal"] == "desecrate")
        for omens in ([SIN_NEC], [SIN_NEC, LIEGE, SOVEREIGN], [SIN_NEC, SOVEREIGN]):
            mutated = copy.deepcopy(step)
            mutated.record["omens"] = omens
            self.assertTrue(sm.omen_problems(mutated), omens)
        # and the case reader refuses a lord on a base-pool target or an
        # unknown lord
        raw = json.loads(CASES.read_text(encoding="utf-8"))["cases"]
        good = next(d for d in raw if d["id"] == "rare-2p1s-prefix-missing")
        for lord in ("ulaman", "xesht"):
            bad = copy.deepcopy(good)
            bad["targets"][0]["lord"] = lord
            with self.assertRaises(ValueError):
                sm.case_from_dict(bad)


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


# The reveal's estimate (#49, the spec's "Reveal odds"): hand values worked
# from the spec's formula, written here apart from the parity file so
# neither can drift to match the other. Each is (exclusive pool n, aimed
# exclusive h, ordinary chance q or None, Abyssal Echoes, chance or None).
REVEAL_ANCHORS = [
    (7, 1, 0.0, False, 1 / 7),
    (7, 1, 0.0, True, 13 / 49),
    (0, 0, 0.1, False, 0.271),
    (0, 0, 0.1, True, 0.468559),
    (10, 0, 0.1, False, 0.19),
    (10, 1, 0.2, False, 0.424),
    (10, 0, 0.0, False, 0.0),
    (10, 0, None, False, None),
]
# The facts the estimate rests on, with the source each must cite.
REVEAL_FACTS = {
    "reveal-exclusive-row": ["poe2wiki.net", "exiledesk.com"],
    "reveal-ordinary-exalt-weights": ["poe2wiki.net"],
    "exclusive-weights-unknown": ["exiledesk.com"],
    "necromancy-without-pool": ["poe2wiki.net"],
}


def assert_chance(test, got, want):
    if want is None:
        test.assertIsNone(got)
    else:
        test.assertIsNotNone(got)
        test.assertAlmostEqual(got, want, places=9)


class RevealOdds(unittest.TestCase):
    def test_reveal_chance_matches_the_recorded_cases(self):
        data = json.loads(sm.REVEAL_ODDS.read_text(encoding="utf-8"))
        self.assertEqual(data["spec"], "docs/models/crafting-slots-spec.md")
        cases = data["cases"]
        self.assertGreaterEqual(len(cases), 8)
        self.assertEqual(len({c["id"] for c in cases}), len(cases))
        for c in cases:
            with self.subTest(case=c["id"]):
                got = sm.reveal_chance(c["exclusive_pool"], c["exclusive_hits"], c["ordinary"], c["echoes"])
                assert_chance(self, got, c["chance"])
        self.assertEqual(sm.reveal_problems(), [])
        # the kinds the spec asks the file to cover
        self.assertTrue(any(c["chance"] is None for c in cases), "an unknown")
        self.assertTrue(any(c["echoes"] for c in cases), "a reroll")
        self.assertTrue(any(c["exclusive_pool"] == 0 for c in cases), "no pool")
        self.assertTrue(any(c["exclusive_hits"] and c["ordinary"] for c in cases),
                        "a mixed exclusive and ordinary aim")
        for anchor in REVEAL_ANCHORS:
            with self.subTest(anchor=anchor):
                self.assertTrue(any(
                    (c["exclusive_pool"], c["exclusive_hits"], c["ordinary"], c["echoes"]) == anchor[:4]
                    for c in cases))

    def test_reveal_chance_anchors(self):
        for n, h, q, echoes, want in REVEAL_ANCHORS:
            with self.subTest(n=n, h=h, q=q, echoes=echoes):
                assert_chance(self, sm.reveal_chance(n, h, q, echoes), want)
        self.assertIs(sm.EXCLUSIVE_EQUAL_WEIGHT, True)
        # control: with the equal-weight switch off an exclusive aim is
        # unknown, and an estimate with no exclusive aim is unchanged
        saved = sm.EXCLUSIVE_EQUAL_WEIGHT
        sm.EXCLUSIVE_EQUAL_WEIGHT = False
        try:
            self.assertIsNone(sm.reveal_chance(7, 1, 0.0, False))
            self.assertIsNone(sm.reveal_chance(10, 1, 0.2, True))
            assert_chance(self, sm.reveal_chance(10, 0, 0.1, False), 0.19)
            assert_chance(self, sm.reveal_chance(0, 0, 0.1, False), 0.271)
        finally:
            sm.EXCLUSIVE_EQUAL_WEIGHT = saved
        # controls: inputs no item can give are refused, not estimated
        for bad in ((0, 1, 0.0, False), (3, 4, 0.0, False), (5, 1, 1.5, False), (-1, 0, 0.1, False)):
            with self.subTest(bad=bad):
                with self.assertRaises(ValueError):
                    sm.reveal_chance(*bad)

    def test_reveal_case_check_reports_a_changed_expectation(self):
        import tempfile
        data = json.loads(sm.REVEAL_ODDS.read_text(encoding="utf-8"))
        mutations = []
        known = next(c for c in data["cases"] if c["chance"] is not None and c["chance"] > 0)
        unknown = next(c for c in data["cases"] if c["chance"] is None)
        for case_id, chance in ((known["id"], known["chance"] + 0.01),
                                (known["id"], None),
                                (unknown["id"], 0.5)):
            mutated = copy.deepcopy(data)
            next(c for c in mutated["cases"] if c["id"] == case_id)["chance"] = chance
            mutations.append((case_id, mutated))
        with tempfile.TemporaryDirectory() as tmp:
            same = Path(tmp) / "same.json"
            same.write_text(json.dumps(data), encoding="utf-8")
            self.assertEqual(sm.reveal_problems(same), [])
            for i, (case_id, mutated) in enumerate(mutations):
                with self.subTest(case=case_id, chance=next(
                        c for c in mutated["cases"] if c["id"] == case_id)["chance"]):
                    path = Path(tmp) / f"mutated-{i}.json"
                    path.write_text(json.dumps(mutated), encoding="utf-8")
                    problems = sm.reveal_problems(path)
                    self.assertEqual(len(problems), 1, problems)
                    self.assertIn(case_id, problems[0])
                    self.assertTrue(sm.check(CASES, path))

    def test_reveal_facts_are_not_established(self):
        text = SPEC.read_text(encoding="utf-8")
        rows = {r["id"]: r for r in sm.spec_facts(text)}
        for fact_id, sources in REVEAL_FACTS.items():
            with self.subTest(fact=fact_id):
                self.assertIn(fact_id, rows)
                self.assertEqual(rows[fact_id]["label"], "not established")
                for s in sources:
                    self.assertIn(s, rows[fact_id]["source"])
        # the three new rows close the table, in this order
        self.assertEqual([r["id"] for r in sm.spec_facts(text)][-3:],
                         ["reveal-exclusive-row", "reveal-ordinary-exalt-weights", "exclusive-weights-unknown"])
        # no blueprint relies on them: the odds' basis names them instead
        table = sm.relied_on_table(text)
        for fact_id in list(REVEAL_FACTS)[:3]:
            with self.subTest(relied_on=fact_id):
                self.assertEqual(table.get(fact_id), [])
        self.assertIn("\n## Reveal odds\n", text.replace("\r\n", "\n"))


if __name__ == "__main__":
    unittest.main()
