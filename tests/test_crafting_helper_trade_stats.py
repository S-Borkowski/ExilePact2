"""tools/crafting_helper/trade_stats.py: map ch-data's modifiers to the trade site's stat ids, with no network.

Every case runs on a synthetic stat list shaped like the trade site's public
one (`result` groups of `{id, text, type}` entries, invented ids) and on
small synthetic class files shaped like `apps/crafting-helper/data/classes/`:
Gloves stands in for the armour classes, Bows for the martial weapons, Wands
and Rings for the classes whose modifiers are global. Fetches go through an
injected fetcher, and caches and data folders live in temporary directories.

The rules held here are the context's "The trade stat mapping": a modifier
matches by its first line with one leading `+` removed, against the
`explicit` group only; an armour class (except for Accuracy) or a martial
weapon takes the `(Local)` id when the site has one; a hybrid, and a line the
site does not list, are recorded as not searchable with their reason rather
than guessed; a text the site lists twice keeps both ids. Each rule keeps a
negative control beside it.
"""
import contextlib
import copy
import datetime
import io
import json
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from tools.crafting_helper import trade_stats  # noqa: E402

TODAY = datetime.date(2026, 10, 30)


def _entry(stat_id, text, kind="explicit"):
    return {"id": stat_id, "text": text, "type": kind}


def stat_list():
    """The site's shape with invented ids: a pseudo, an explicit and an implicit group."""
    return {"result": [
        {"id": "pseudo", "label": "Pseudo", "entries": [
            _entry("pseudo.pseudo_total_life", "+# total maximum Life", "pseudo"),
        ]},
        {"id": "explicit", "label": "Explicit", "entries": [
            _entry("explicit.stat_101", "# to maximum Life"),
            _entry("explicit.stat_102", "#% increased Armour"),
            _entry("explicit.stat_103", "#% increased Armour (Local)"),
            _entry("explicit.stat_104", "# to Accuracy Rating"),
            _entry("explicit.stat_105", "# to Accuracy Rating (Local)"),
            _entry("explicit.stat_106", "# to Spirit"),
            _entry("explicit.stat_107", "# to Spirit"),
            _entry("explicit.stat_108", "#% increased Attribute Requirements"),
            _entry("explicit.stat_109", "#% increased Cast Speed"),
            _entry("explicit.stat_110", "# to Unrelated Stat"),
        ]},
        {"id": "implicit", "label": "Implicit", "entries": [
            _entry("implicit.stat_201", "# to Evasion Rating", "implicit"),
            _entry("implicit.stat_202", "#% increased Movement Speed", "implicit"),
        ]},
    ]}


def _mod(mod_id, lines, section="normal", kind=None):
    kind = kind or mod_id.split("-")[0]
    return {"id": mod_id, "section": section, "kind": kind, "family": ["F"], "lines": lines, "tiers": {}}


COMMON = [
    _mod("prefix-to-maximum-life", ["+# to maximum Life"]),
    _mod("prefix-increased-armour", ["#% increased Armour"]),
    _mod("prefix-to-accuracy-rating", ["+# to Accuracy Rating"]),
    _mod("suffix-to-spirit", ["+# to Spirit"]),
    _mod("suffix-increased-cast-speed", ["#% increased Cast Speed"]),
    _mod("suffix-reduced-attribute-requirements", ["#% reduced Attribute Requirements"]),
    _mod("prefix-hybrid-armour-life", ["#% increased Armour", "+# to maximum Life"]),
    _mod("prefix-to-evasion-rating", ["+# to Evasion Rating"]),
    _mod("suffix-increased-movement-speed", ["#% increased Movement Speed"], section="desecrated"),
]


def class_files():
    return [{"class": name, "mods": copy.deepcopy(COMMON)} for name in ("Gloves", "Rings", "Bows", "Wands")]


def mapping():
    return trade_stats.build_mapping(stat_list(), class_files(), TODAY.isoformat())


def write_data_dir(root: Path, files=None) -> Path:
    """A data folder like apps/crafting-helper/data: index.json and classes/<class>.json."""
    files = files if files is not None else class_files()
    data = root / "data"
    (data / "classes").mkdir(parents=True)
    index = {"schema": 1, "classes": []}
    for f in files:
        name = "classes/%s.json" % f["class"].lower().replace(" ", "-")
        index["classes"].append({"class": f["class"], "file": name, "variants": ["all"]})
        (data / name).write_text(json.dumps(f), encoding="utf-8")
    (data / "index.json").write_text(json.dumps(index), encoding="utf-8")
    return data


class FakeSite:
    def __init__(self, status=200, body=None):
        self.status = status
        self.body = json.dumps(stat_list()).encode("utf-8") if body is None else body
        self.requests = []

    def __call__(self, url, user_agent):
        self.requests.append((url, user_agent))
        return self.status, self.body


class TempDirs(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = Path(self._tmp.name)
        self.cache = self.tmp / "trade-cache"

    def tearDown(self):
        self._tmp.cleanup()

    def quiet(self):
        return contextlib.redirect_stdout(io.StringIO())


class Matching(unittest.TestCase):
    def test_plus_is_stripped_and_the_text_matched(self):
        rings = mapping()["classes"]["Rings"]
        self.assertEqual(rings["prefix-to-maximum-life"], {"text": "# to maximum Life", "ids": ["explicit.stat_101"]})
        # A line without a leading + matches as it is.
        self.assertEqual(rings["suffix-increased-cast-speed"]["ids"], ["explicit.stat_109"])
        self.assertEqual(trade_stats.trade_text("+# to maximum Life"), "# to maximum Life")
        # Only one + goes, and nothing else is rewritten.
        self.assertEqual(trade_stats.trade_text("++# x"), "+# x")
        self.assertEqual(trade_stats.trade_text("#% reduced X"), "#% reduced X")
        # Negative controls: a text the site lists only in another group, or
        # in another case, is not a match.
        self.assertEqual(rings["prefix-to-evasion-rating"], {"searchable": False, "reason": "no trade stat"})
        files = class_files()
        files[1]["mods"][0]["lines"] = ["+# To Maximum Life"]
        other = trade_stats.build_mapping(stat_list(), files, TODAY.isoformat())
        self.assertEqual(other["classes"]["Rings"]["prefix-to-maximum-life"],
                         {"searchable": False, "reason": "no trade stat"})

    def test_local_stat_only_for_armour_defences_and_martial_weapons(self):
        classes = mapping()["classes"]
        # An armour class takes the (Local) id for its defences...
        self.assertEqual(classes["Gloves"]["prefix-increased-armour"],
                         {"text": "#% increased Armour (Local)", "ids": ["explicit.stat_103"]})
        # ...but its Accuracy is global.
        self.assertEqual(classes["Gloves"]["prefix-to-accuracy-rating"],
                         {"text": "# to Accuracy Rating", "ids": ["explicit.stat_104"]})
        # A martial weapon's Accuracy is local.
        self.assertEqual(classes["Bows"]["prefix-to-accuracy-rating"]["ids"], ["explicit.stat_105"])
        # Negative controls: jewellery and a caster weapon take the plain id,
        # and a text with no (Local) entry keeps its plain id on any class.
        for name in ("Rings", "Wands"):
            with self.subTest(item_class=name):
                self.assertEqual(classes[name]["prefix-increased-armour"]["ids"], ["explicit.stat_102"])
                self.assertEqual(classes[name]["prefix-to-accuracy-rating"]["ids"], ["explicit.stat_104"])
        self.assertEqual(classes["Gloves"]["prefix-to-maximum-life"]["ids"], ["explicit.stat_101"])
        self.assertTrue(trade_stats.uses_local("Body Armours", "#% increased Armour"))
        self.assertTrue(trade_stats.uses_local("Quarterstaves", "# to Accuracy Rating"))
        self.assertFalse(trade_stats.uses_local("Helmets", "# to Accuracy Rating"))
        for name in ("Rings", "Amulets", "Belts", "Quivers", "Jewels", "Wands", "Sceptres", "Staves"):
            with self.subTest(item_class=name):
                self.assertFalse(trade_stats.uses_local(name, "#% increased Armour"))

    def test_hybrid_reduced_and_unmatched_are_reported_not_guessed(self):
        gloves = mapping()["classes"]["Gloves"]
        # A hybrid's first line is a plain modifier's text; it is still not mapped by it.
        self.assertEqual(gloves["prefix-hybrid-armour-life"], {"searchable": False, "reason": "hybrid"})
        # "reduced" is not rewritten to "increased", though the site lists the latter.
        self.assertEqual(gloves["suffix-reduced-attribute-requirements"],
                         {"searchable": False, "reason": "no trade stat"})
        self.assertEqual(gloves["prefix-to-evasion-rating"], {"searchable": False, "reason": "no trade stat"})
        # Desecrated modifiers are out of the mapping altogether.
        self.assertNotIn("suffix-increased-movement-speed", gloves)
        # Negative control: the same hybrid cut to its first line is searchable.
        files = class_files()
        files[0]["mods"][6]["lines"] = ["#% increased Armour"]
        one = trade_stats.build_mapping(stat_list(), files, TODAY.isoformat())
        self.assertEqual(one["classes"]["Gloves"]["prefix-hybrid-armour-life"]["ids"], ["explicit.stat_103"])
        counts = trade_stats.coverage(mapping())
        self.assertEqual(counts, {"modifiers": 32, "searchable": 20, "local": 3, "hybrid": 4,
                                  "no trade stat": 8})

    def test_duplicate_texts_keep_every_id(self):
        classes = mapping()["classes"]
        for name in ("Gloves", "Rings"):
            with self.subTest(item_class=name):
                self.assertEqual(classes[name]["suffix-to-spirit"],
                                 {"text": "# to Spirit", "ids": ["explicit.stat_106", "explicit.stat_107"]})
        # Negative control: a text listed once keeps one id.
        self.assertEqual(classes["Rings"]["prefix-to-maximum-life"]["ids"], ["explicit.stat_101"])
        # A duplicate is still a duplicate in check's eyes only when an id repeats.
        bad = mapping()
        bad["classes"]["Rings"]["suffix-to-spirit"]["ids"] = ["explicit.stat_106", "explicit.stat_106"]
        self.assertTrue(any("repeats" in p for p in trade_stats.check_mapping(bad, class_files())))

    def test_an_explicit_group_is_required(self):
        stats = stat_list()
        stats["result"] = [g for g in stats["result"] if g["id"] != "explicit"]
        with self.assertRaisesRegex(trade_stats.ToolError, "explicit"):
            trade_stats.build_mapping(stats, class_files(), TODAY.isoformat())
        self.assertEqual(len(trade_stats.explicit_texts(stat_list())), 9)


class Fetch(TempDirs):
    def test_one_get_of_the_stat_list_into_the_cache(self):
        site = FakeSite()
        with self.quiet():
            trade_stats.fetch(self.cache, fetcher=site, today=TODAY)
        self.assertEqual(site.requests, [(trade_stats.STATS_URL, trade_stats.USER_AGENT)])
        self.assertIn("ExilePact2", trade_stats.USER_AGENT)
        record = json.loads((self.cache / trade_stats.CACHE_INDEX).read_text(encoding="utf-8"))
        self.assertEqual(record, {"url": trade_stats.STATS_URL, "fetched": "2026-10-30", "status": 200,
                                  "file": trade_stats.CACHE_FILE})
        self.assertEqual(json.loads((self.cache / trade_stats.CACHE_FILE).read_text(encoding="utf-8")), stat_list())

    def test_a_failed_or_unreadable_answer_leaves_the_cache_alone(self):
        with self.quiet():
            trade_stats.fetch(self.cache, fetcher=FakeSite(), today=TODAY)
        before = (self.cache / trade_stats.CACHE_FILE).read_bytes()
        for site in (FakeSite(status=503), FakeSite(body=b"<html>"), FakeSite(body=b'{"result": 1}')):
            with self.subTest(status=site.status, body=site.body[:12]):
                with self.quiet(), self.assertRaises(trade_stats.ToolError):
                    trade_stats.fetch(self.cache, fetcher=site, today=datetime.date(2026, 11, 2))
                self.assertEqual((self.cache / trade_stats.CACHE_FILE).read_bytes(), before)
                record = json.loads((self.cache / trade_stats.CACHE_INDEX).read_text(encoding="utf-8"))
                self.assertEqual(record["fetched"], "2026-10-30")

    def test_cache_inside_a_git_checkout_is_refused(self):
        site = FakeSite()
        with self.assertRaisesRegex(trade_stats.ToolError, "git checkout"):
            trade_stats.fetch(ROOT / "build" / "trade-cache-test", fetcher=site, today=TODAY)
        self.assertEqual(site.requests, [])
        self.assertFalse((ROOT / "build" / "trade-cache-test").exists())


class WrittenFile(TempDirs):
    def write(self):
        data = write_data_dir(self.tmp)
        with self.quiet():
            trade_stats.fetch(self.cache, fetcher=FakeSite(), today=TODAY)
        out = io.StringIO()
        trade_stats.write_map(self.cache, data, out=out)
        return data, out.getvalue()

    def test_written_file_has_source_url_iso_date_and_only_matched_entries(self):
        data, printed = self.write()
        text = (data / trade_stats.DATA_FILE).read_bytes().decode("utf-8")
        written = json.loads(text)
        self.assertEqual(text, trade_stats.dumps(written))
        self.assertNotIn("\r", text)
        self.assertEqual(written["schema"], trade_stats.SCHEMA)
        self.assertEqual(written["source"], "https://www.pathofexile.com/api/trade2/data/stats")
        self.assertEqual(datetime.date.fromisoformat(written["fetched"]), TODAY)
        self.assertIn("Grinding Gear Games", written["source_note"])
        self.assertEqual(list(written["classes"]), ["Gloves", "Rings", "Bows", "Wands"])
        self.assertEqual(written["classes"], mapping()["classes"])
        # Only matched entries' text and ids: nothing else the list carried.
        for unused in ("explicit.stat_108", "explicit.stat_110", "# to Unrelated Stat", "pseudo",
                       "implicit", "# to Evasion Rating", "Movement Speed", "label"):
            with self.subTest(unused=unused):
                self.assertNotIn(unused, text)
        # Negative control: those strings were in the cached list.
        cached = (self.cache / trade_stats.CACHE_FILE).read_text(encoding="utf-8")
        for unused in ("explicit.stat_108", "# to Unrelated Stat", "pseudo", "# to Evasion Rating"):
            self.assertIn(unused, cached)
        self.assertIn("Gloves: 8 modifiers, 5 searchable (1 local), 1 hybrid, 2 no trade stat", printed)
        self.assertEqual(trade_stats.check_dir(data), [])

    def test_map_without_a_cache_writes_nothing(self):
        data = write_data_dir(self.tmp)
        with self.assertRaisesRegex(trade_stats.ToolError, "fetch"):
            trade_stats.write_map(self.cache, data, out=io.StringIO())
        self.assertFalse((data / trade_stats.DATA_FILE).exists())


class Check(TempDirs):
    def test_check_refuses_a_bad_file(self):
        files = class_files()
        good = mapping()
        # Negative control: the file build_mapping makes is clean.
        self.assertEqual(trade_stats.check_mapping(good, files), [])

        def broken(edit):
            bad = copy.deepcopy(good)
            edit(bad)
            return trade_stats.check_mapping(bad, files)

        cases = {
            "explicit.stat_": lambda m: m["classes"]["Rings"]["prefix-to-maximum-life"].__setitem__(
                "ids", ["implicit.stat_201"]),
            "source": lambda m: m.pop("source"),
            "source URL": lambda m: m.__setitem__("source", "https://example.com/stats"),
            "fetched": lambda m: m.pop("fetched"),
            "ISO date": lambda m: m.__setitem__("fetched", "30/10/2026"),
            "source_note": lambda m: m.pop("source_note"),
            "schema": lambda m: m.__setitem__("schema", 2),
            "not in the class file": lambda m: m["classes"]["Rings"].__setitem__(
                "prefix-invented", {"text": "# x", "ids": ["explicit.stat_1"]}),
            "missing": lambda m: m["classes"]["Rings"].pop("suffix-to-spirit"),
            "desecrated": lambda m: m["classes"]["Rings"].__setitem__(
                "suffix-increased-movement-speed", {"searchable": False, "reason": "no trade stat"}),
            "classes": lambda m: m["classes"].pop("Wands"),
            "hybrid": lambda m: m["classes"]["Rings"].__setitem__(
                "prefix-hybrid-armour-life", {"text": "#% increased Armour", "ids": ["explicit.stat_102"]}),
            "reason": lambda m: m["classes"]["Rings"].__setitem__(
                "prefix-to-evasion-rating", {"searchable": False, "reason": "guessed"}),
            "text": lambda m: m["classes"]["Rings"]["prefix-to-maximum-life"].__setitem__("text", ""),
            "no ids": lambda m: m["classes"]["Rings"]["prefix-to-maximum-life"].__setitem__("ids", []),
            "keys": lambda m: m["classes"]["Rings"]["prefix-to-maximum-life"].__setitem__("value", 1),
        }
        for word, edit in cases.items():
            with self.subTest(defect=word):
                problems = broken(edit)
                self.assertTrue(problems, word)
                self.assertTrue(any(word in p for p in problems), problems)

    def test_check_command_reads_the_folder(self):
        data = write_data_dir(self.tmp)
        path = data / trade_stats.DATA_FILE
        with self.quiet():
            self.assertEqual(trade_stats.main(["check", "--data", str(data)]), 1)  # missing file
        path.write_text(trade_stats.dumps(mapping()), encoding="utf-8", newline="\n")
        with self.quiet():
            self.assertEqual(trade_stats.main(["check", "--data", str(data)]), 0)
        # Not written as map writes it.
        path.write_text(json.dumps(mapping()), encoding="utf-8")
        self.assertTrue(any("not written as map writes it" in p for p in trade_stats.check_dir(data)))
        # A class file that lost a modifier the mapping names.
        path.write_text(trade_stats.dumps(mapping()), encoding="utf-8", newline="\n")
        files = class_files()
        files[1]["mods"].pop(0)
        (data / "classes" / "rings.json").write_text(json.dumps(files[1]), encoding="utf-8")
        with self.quiet():
            self.assertEqual(trade_stats.main(["check", "--data", str(data)]), 1)


if __name__ == "__main__":
    unittest.main()
