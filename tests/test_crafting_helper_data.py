"""The crafting helper's shipped data: `apps/crafting-helper/data/` as `extract` wrote it from poe2db.tw.

`tests/test_crafting_helper_poe2db.py` holds the extractor to its rules on
synthetic pages; this module holds the files actually shipped to the same
rules, plus the facts about the real pages the milestone pins: the 22 item
classes of the class table and their variant counts, every page's source on
poe2db.tw with an ISO fetch date, the Gloves life prefix and the essence that
adds it, and the jewels' weights and liquid emotions. Each rule keeps a
negative control beside it: the same file with one defect fails the check.

When a refresh changes the site's content, `extract` rewrites these files and
the facts below may move; a fact that moved is a change to review, not a test
to loosen.
"""
import copy
import datetime
import json
import re
import sys
import unittest
import urllib.parse
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from tools.crafting_helper import poe2db, trade_stats  # noqa: E402

DATA = ROOT / "apps" / "crafting-helper" / "data"


def _read(path: Path) -> tuple[str, dict]:
    # A Windows checkout with core.autocrlf turns LF into CRLF; the content is
    # what is pinned, not the checkout's line endings.
    text = path.read_bytes().decode("utf-8").replace("\r\n", "\n")
    return text, json.loads(text)


class ShippedData(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.index_text, cls.index = _read(DATA / poe2db.DATA_INDEX)
        cls.texts: dict[str, str] = {}
        cls.files: dict[str, dict] = {}
        for entry in cls.index["classes"]:
            cls.texts[entry["class"]], cls.files[entry["class"]] = _read(DATA / entry["file"])

    def test_the_check_command_passes(self):
        self.assertEqual(poe2db.check_dir(DATA), [])

    def test_index_is_the_class_table(self):
        self.assertEqual(poe2db.check_index(self.index), [])
        self.assertEqual(self.index, poe2db.build_index())
        self.assertIn("CC BY-NC-SA 3.0", self.index["source_note"])
        self.assertEqual(len(self.index["classes"]), 22)
        counts = sorted(len(c["variants"]) for c in self.index["classes"] if len(c["variants"]) > 1)
        self.assertEqual(counts, [3, 6, 6, 6, 7, 8])
        names = [c["class"] for c in self.index["classes"]]
        self.assertEqual(names[:5], ["Gloves", "Boots", "Helmets", "Body Armours", "Shields"])
        self.assertEqual(names[-1], "Jewels")
        # The folder holds the files the index names, the trade stat mapping
        # (trade_stats.py writes it beside them), and nothing else.
        on_disk = sorted(p.relative_to(DATA).as_posix() for p in DATA.rglob("*") if p.is_file())
        self.assertEqual(on_disk, sorted([poe2db.DATA_INDEX, trade_stats.DATA_FILE]
                                         + [c["file"] for c in self.index["classes"]]))

    def test_every_class_file_passes_check_data(self):
        by_name = {c.name: c for c in poe2db.CLASS_TABLE}
        for name, data in self.files.items():
            with self.subTest(item_class=name):
                self.assertEqual(poe2db.check_data(data, by_name[name]), [])
                variants = [p["variant"] for p in data["pages"]]
                self.assertEqual(variants, [p.variant for p in by_name[name].pages])

    def test_every_page_is_a_poe2db_url_with_an_iso_fetch_date(self):
        for name, data in self.files.items():
            for page in data["pages"]:
                with self.subTest(item_class=name, variant=page["variant"]):
                    url = urllib.parse.urlsplit(page["url"])
                    self.assertEqual((url.scheme, url.netloc), ("https", "poe2db.tw"))
                    self.assertTrue(url.path.startswith("/us/"), page["url"])
                    self.assertEqual(datetime.date.fromisoformat(page["fetched"]).isoformat(), page["fetched"])

    def test_written_exactly_as_extract_writes_it(self):
        self.assertNotIn("\r", self.index_text)
        self.assertEqual(self.index_text, poe2db.dumps(self.index))
        for name, data in self.files.items():
            with self.subTest(item_class=name):
                self.assertNotIn("\r", self.texts[name])
                self.assertEqual(self.texts[name], poe2db.dumps(data))
        # Control: a hand-reformatted copy is not what extract writes.
        gloves = self.files["Gloves"]
        self.assertNotEqual(json.dumps(gloves, indent=4) + "\n", poe2db.dumps(gloves))

    def test_no_markup_or_digits_in_templates(self):
        for name, data in self.files.items():
            lines = [line for m in data["mods"] for line in m["lines"]]
            lines += [line for e in data["essences"] + data["liquid"] for line in e["lines"]]
            with self.subTest(item_class=name):
                self.assertTrue(lines)
                for line in lines:
                    self.assertIsNone(re.search(r"[\[<>0-9]", line), line)
                self.assertNotRegex(self.texts[name], r"data-hover|<span|<a |<br|class=")

    def test_weight_labels(self):
        # Desecrated rows never carry a real weight, and every jewel page's
        # normal rows are all 1: both are labelled, never shipped as spawn.
        for name, data in self.files.items():
            for page in data["pages"]:
                with self.subTest(item_class=name, variant=page["variant"]):
                    self.assertIn(page["weights"]["desecrated"], ("not-established", "none"))
                    if name == "Jewels":
                        self.assertEqual(page["weights"]["normal"], "not-established")
        sceptres = {p["weights"]["desecrated"] for p in self.files["Sceptres"]["pages"]}
        self.assertEqual(sceptres, {"none"})

    def test_gloves(self):
        gloves = self.files["Gloves"]
        self.assertEqual(sorted(p["variant"] for p in gloves["pages"]),
                         ["dex", "dex_int", "int", "str", "str_dex", "str_int"])
        self.assertEqual({(p["weights"]["normal"], p["weights"]["desecrated"]) for p in gloves["pages"]},
                         {("spawn", "not-established")})
        life = [m for m in gloves["mods"]
                if m["section"] == "normal" and m["kind"] == "prefix" and m["lines"] == ["+# to maximum Life"]]
        self.assertEqual(len(life), 1)
        tiers = life[0]["tiers"]["int"]
        self.assertEqual(len(tiers), 9)
        self.assertEqual((tiers[0]["tier"], tiers[0]["ilvl"]), (1, 60))
        self.assertEqual([t["tier"] for t in tiers], list(range(1, 10)))
        self.assertEqual([t["ilvl"] for t in tiers], sorted((t["ilvl"] for t in tiers), reverse=True))
        self.assertTrue(all(isinstance(t["weight"], int) and t["weight"] > 1 for t in tiers))
        body = [e for e in gloves["essences"] if e["name"] == "Lesser Essence of the Body"]
        self.assertEqual(len(body), 1)
        self.assertEqual(body[0]["matches"]["int"], {"mod": life[0]["id"], "tier": 7})
        self.assertEqual(tiers[6]["ilvl"], body[0]["ilvl"])
        self.assertEqual(tiers[6]["values"], body[0]["values"])
        self.assertEqual(gloves["liquid"], [])

    def test_jewels(self):
        jewels = self.files["Jewels"]
        self.assertEqual({p["weights"]["normal"] for p in jewels["pages"]}, {"not-established"})
        self.assertEqual(jewels["essences"], [])
        self.assertTrue(jewels["liquid"])
        self.assertEqual(len(jewels["bases"]), 8)
        mod_ids = {m["id"] for m in jewels["mods"]}
        matched = [v for e in jewels["liquid"] for v in e["matches"].values() if v is not None]
        self.assertTrue(matched)
        for match in matched:
            self.assertIn(match["mod"], mod_ids)

    def test_the_check_refuses_a_bad_file(self):
        # Negative control: the shipped Gloves file with one defect at a time fails.
        gloves = self.files["Gloves"]
        item_class = poe2db.CLASS_TABLE[0]
        life = next(n for n, m in enumerate(gloves["mods"])
                    if m["section"] == "normal" and m["lines"] == ["+# to maximum Life"])

        def broken(change):
            data = copy.deepcopy(gloves)
            change(data)
            return poe2db.check_data(data, item_class)

        self.assertEqual(broken(lambda d: None), [])
        self.assertTrue(broken(lambda d: d["mods"][life]["lines"].__setitem__(0, "+5 to maximum Life")))
        self.assertTrue(broken(lambda d: d["mods"][life]["lines"].__setitem__(0, "<span>#</span>")))
        self.assertTrue(broken(lambda d: d["mods"][life]["tiers"]["int"].pop(0)))
        self.assertTrue(broken(lambda d: d["mods"][life]["tiers"]["int"][0]["values"].append([1, 2])))
        self.assertTrue(broken(lambda d: d["pages"][0].__setitem__("fetched", "yesterday")))
        self.assertTrue(broken(lambda d: d["essences"][0]["matches"].__setitem__(
            "int", {"mod": "no-such-mod", "tier": 1})))
        self.assertTrue(broken(lambda d: d.__setitem__("class", "Boots")))


class ShippedTradeStats(unittest.TestCase):
    """`trade-stats.json` as `trade_stats.py map` wrote it from the trade site's 2026-10-30 stat list.

    The coverage floors are the counts measured in planning over the shipped
    class data and the site's list (900 ordinary-pool modifiers: 77 hybrids,
    32 lines the site does not list). A refresh that maps fewer is a change
    to review, not a floor to lower.
    """

    @classmethod
    def setUpClass(cls):
        cls.text, cls.mapping = _read(DATA / trade_stats.DATA_FILE)

    def test_the_check_command_passes(self):
        self.assertEqual(trade_stats.check_dir(DATA), [])
        self.assertNotIn("\r", self.text)
        self.assertEqual(self.text, trade_stats.dumps(self.mapping))

    def test_source_url_and_iso_fetch_date(self):
        self.assertEqual(self.mapping["source"], "https://www.pathofexile.com/api/trade2/data/stats")
        fetched = self.mapping["fetched"]
        self.assertEqual(datetime.date.fromisoformat(fetched).isoformat(), fetched)
        self.assertIn("Grinding Gear Games", self.mapping["source_note"])

    def test_coverage_is_at_least_the_measured_counts(self):
        counts = trade_stats.coverage(self.mapping)
        self.assertEqual(counts["modifiers"], 900)
        self.assertGreaterEqual(counts["searchable"], 900 - 77 - 32)
        self.assertLessEqual(counts["no trade stat"], 32)
        self.assertEqual(counts["hybrid"], 77)
        self.assertGreater(counts["local"], 0)
        # Control: the floors are not met by an empty mapping.
        empty = {"classes": {name: {} for name in self.mapping["classes"]}}
        self.assertLess(trade_stats.coverage(empty)["searchable"], 900 - 77 - 32)

    def test_anchor_modifiers(self):
        classes = self.mapping["classes"]
        gloves, rings = classes["Gloves"], classes["Rings"]
        self.assertTrue(gloves["prefix-increased-armour"]["text"].endswith(" (Local)"))
        self.assertEqual(gloves["prefix-to-accuracy-rating"]["text"], "# to Accuracy Rating")
        self.assertEqual(gloves["prefix-to-accuracy-rating"], rings["prefix-to-accuracy-rating"])
        self.assertEqual(rings["prefix-to-maximum-life"]["text"], "# to maximum Life")
        self.assertEqual(gloves["prefix-increased-armour-2"], {"searchable": False, "reason": "hybrid"})
        self.assertEqual(len(rings["suffix-to-all-attributes"]["ids"]), 2)
        self.assertTrue(all(i.startswith("explicit.stat_") for e in classes.values() for v in e.values()
                            for i in v.get("ids", [])))


if __name__ == "__main__":
    unittest.main()
