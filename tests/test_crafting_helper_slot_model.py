"""tools/crafting_helper/slot_model.py: the slot model, its case file and its spec.

The model is built from `docs/models/crafting-slots-spec.md` alone and holds
the engine to the spec through the shared case file
`apps/crafting-helper/crates/ch-engine/tests/items/slots/cases.json`.

Baselines (how things stand without the v2 routes):

- an Exalted Orb with no side omen can land in the prefix slot the owner's
  2P/1S craft still needs (`test_baseline_an_unrestricted_exalt_...`);
- the listed `fill-desecrate-exalt`, read as written, names a plain
  `exalted-orb` first on that craft;
- its starts reject a rare with one target (#28).

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

GREATER = "omen-of-greater-exaltation"
SIN_EX, DEX_EX = "omen-of-sinistral-exaltation", "omen-of-dextral-exaltation"
SIN_ANN, DEX_ANN = "omen-of-sinistral-annulment", "omen-of-dextral-annulment"
SIN_NEC = "omen-of-sinistral-necromancy"
ECHOES = "omen-of-abyssal-echoes"
ESSENCE = "<the case's essence>"

# What the spec's "Representative crafts" says must hold, step by step:
# (goal, side, currency, omens[, certain]); `done` when the route must end
# there, `False` when only these first steps are pinned.
MUST_HOLD = {
    ("rare-2p1s-prefix-missing", "close-a-side"): ([
        ("fill", "suffix", "exalted-orb", [GREATER, DEX_EX]),
        ("add-wanted", "prefix", "perfect-exalted-orb", []),
    ], True),
    ("rare-2p1s-prefix-missing", "essence-desecrate-double-slam"): ([
        ("desecrate", "prefix", "desecrate", [SIN_NEC]),
        ("reveal", None, "reveal", [ECHOES]),
    ], True),
    ("rare-2p1s-prefix-missing", "rare-clean-then-slam"): ([
        ("add-wanted", "prefix", "perfect-exalted-orb", [SIN_EX]),
    ], True),
    ("rare-2p1s-prefix-missing", "magic-augment-essence"): "no-start-fits",
    ("magic-two-good", "essence-desecrate-double-slam"): ([
        ("add-wanted", None, ESSENCE, []),
        ("desecrate", "prefix", "desecrate", [SIN_NEC]),
        ("reveal", None, "reveal", [ECHOES]),
        ("add-wanted", "suffix", "perfect-exalted-orb", [GREATER, DEX_EX]),
    ], True),
    ("magic-two-good", "close-a-side"): "no-start-fits",
    ("magic-two-good", "rare-clean-then-slam"): "no-start-fits",
    ("magic-two-good", "magic-augment-essence"): "desecrated-target-unplaced",
    ("rare-fractured", "close-a-side"): ([
        ("remove-unwanted", "prefix", "orb-of-annulment", [SIN_ANN], True),
    ], False),
    ("rare-fractured", "essence-desecrate-double-slam"): ([
        ("remove-unwanted", "prefix", "orb-of-annulment", [SIN_ANN], True),
    ], False),
    ("rare-fractured", "rare-clean-then-slam"): ([
        ("remove-unwanted", "prefix", "orb-of-annulment", [SIN_ANN], True),
    ], False),
    ("rare-fractured", "magic-augment-essence"): "no-start-fits",
    ("rare-one-target", "rare-clean-then-slam"): ([
        ("remove-unwanted", "prefix", "orb-of-annulment", []),
        ("add-wanted", "prefix", "perfect-exalted-orb", [GREATER, SIN_EX]),
        ("add-wanted", "suffix", "perfect-exalted-orb", [DEX_EX]),
    ], True),
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
        self.assertEqual(sorted(cases), sorted(OWNERS_CRAFTS))
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
        # control: with one side open the plain exalt fits (close-a-side's last slam)
        last = sm.plan(sm.BLUEPRINTS["close-a-side"], owner).record()["steps"][-1]
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
        first = sm.plan(sm.BLUEPRINTS["rare-clean-then-slam"], case("rare-2p1s-prefix-missing")).record()["steps"][0]
        self.assertNotIn(GREATER, first["omens"])
        # control: Greater blocked, one prefix per slam
        c = with_changes(case("rare-one-target"), blocked=frozenset({GREATER}))
        steps = sm.plan(sm.BLUEPRINTS["rare-clean-then-slam"], c).record()["steps"]
        self.assertEqual([s["omens"] for s in steps[1:]], [[SIN_EX], [SIN_EX], [DEX_EX]])

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
        route = sm.plan(sm.BLUEPRINTS["close-a-side"], owner)
        fill = copy.deepcopy(route.steps[0])
        fill.record["omens"] = [GREATER]
        self.assertTrue(sm.omen_problems(fill))
        route = sm.plan(sm.BLUEPRINTS["essence-desecrate-double-slam"], owner)
        desecrate = copy.deepcopy(route.steps[0])
        desecrate.record["omens"] = []
        self.assertTrue(sm.omen_problems(desecrate))
        wrong_side = copy.deepcopy(route.steps[0])
        wrong_side.record["omens"] = ["omen-of-dextral-necromancy"]
        self.assertTrue(sm.omen_problems(wrong_side))


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
