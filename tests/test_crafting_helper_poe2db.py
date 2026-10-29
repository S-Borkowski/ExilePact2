"""tools/crafting_helper/poe2db.py: fetch, extract and check the crafting helper's data, with no network.

Every case runs against an injected fetcher that serves synthetic pages from
`tests/fixtures/crafting_helper/poe2db/` (the site's shape, invented bases,
modifiers and essences; never a saved page) and an injected clock, so nothing
here sends a request or sleeps. A small class table stands in for the real
one: Gloves with two attribute variants (`str`, `int`), Rings on a single
page and Jewels with one jewel page. Caches and output folders live in
temporary directories.

The baseline is the site as a polite client must treat it (`Fetch`): what
robots.txt disallows is never asked for, requests are spaced by the delay, a
fresh cache is not asked again, and nothing is written inside a git checkout.
The targets are the files `extract` makes from the pages (`Extract`), its
loud failure on a page whose structure it does not know, which leaves every
file alone (`Surprises`), and the rules `check` holds the files to
(`CheckData`). Each keeps a negative control beside it: an allow-all
robots.txt that does fetch, a delay at the minimum that is accepted, a stale
page that is fetched, an essence tier that is not matched, a well-formed set
of pages that extracts, and clean files the check passes.
"""
import contextlib
import copy
import datetime
import io
import json
import re
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from tools.crafting_helper import poe2db  # noqa: E402
from tools.quest_tracker import poe2db as quest_poe2db  # noqa: E402
from tools.waystone_roller import poe2db as waystone_poe2db  # noqa: E402

FIXTURES = ROOT / "tests" / "fixtures" / "crafting_helper" / "poe2db"
TODAY = datetime.date(2026, 10, 28)
TABLE = (
    poe2db._armour("Gloves", "Gloves", "Gloves", ("str", "int")),
    poe2db._single("Rings", "Ring", "Rings"),
    poe2db._jewels("Jewels", "Jewel", ("Ruby",)),
)
SLUGS = ["Gloves_str", "Gloves_int", "Rings", "Ruby"]
URLS = ["https://poe2db.tw/us/%s" % slug for slug in SLUGS]


class FakeClock:
    """A monotonic clock that only moves when slept on or told to."""

    def __init__(self, today=TODAY):
        self.t = 1000.0
        self.sleeps = []
        self._today = today

    def monotonic(self):
        return self.t

    def sleep(self, seconds):
        self.sleeps.append(seconds)
        self.t += seconds

    def today(self):
        return self._today


class FakeSite:
    """Serves fixture files at the site's URLs and records each request.

    A request takes 0.4 s of fake time, so the delay has something to subtract.
    `routes` maps a URL to a fixture name, or to an int for an HTTP error.
    """

    def __init__(self, clock, robots="robots.txt"):
        self.clock = clock
        self.calls = []
        self.routes = {url: slug + ".html" for url, slug in zip(URLS, SLUGS)}
        if robots is not None:
            self.routes[poe2db.ROBOTS_URL] = robots

    def __call__(self, url, user_agent):
        self.calls.append((self.clock.monotonic(), url, user_agent))
        self.clock.t += 0.4
        route = self.routes.get(url)
        if route is None:
            return 404, b"not found"
        if isinstance(route, int):
            return route, b"unavailable"
        return 200, (FIXTURES / route).read_bytes()

    def urls(self):
        return [url for _, url, _ in self.calls]


def run(argv, fetcher=None, clock=None, table=TABLE):
    out, err = io.StringIO(), io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
        code = poe2db.main(argv, fetcher=fetcher, clock=clock, table=table)
    return code, out.getvalue(), err.getvalue()


def fixture_text(slug):
    return (FIXTURES / (slug + ".html")).read_text(encoding="utf-8")


def make_cache(folder, pages=None, fetched="2026-10-28"):
    """A cache as fetch leaves it, holding the fixture pages or the `pages` given by slug."""
    folder.mkdir(parents=True, exist_ok=True)
    records = {}
    for slug in SLUGS:
        html = (pages or {}).get(slug, fixture_text(slug))
        (folder / (slug + ".html")).write_text(html, encoding="utf-8", newline="\n")
        records[slug] = {"slug": slug, "url": poe2db.page_url(slug), "fetched": fetched, "status": 200,
                         "file": slug + ".html"}
    (folder / poe2db.CACHE_INDEX).write_text(json.dumps({"schema": 1, "pages": records}), encoding="utf-8")
    return folder


def edit_view(html, change):
    """`html` with its ModsView JSON passed through `change(view)`."""
    start = html.index("{", html.index("new ModsView("))
    view, end = json.JSONDecoder().raw_decode(html, start)
    change(view)
    return html[:start] + json.dumps(view, ensure_ascii=False) + html[end:]


def tree(folder):
    """{relative path: bytes} of every file under `folder`."""
    return {p.relative_to(folder).as_posix(): p.read_bytes() for p in sorted(folder.rglob("*")) if p.is_file()}


class Poe2dbTestCase(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)
        # Every positive case below writes here; it must not be a checkout.
        self.assertIsNone(quest_poe2db.git_tree_of(self.tmp), "the temp folder sits inside a git checkout")

    def extract_pages(self, pages=None, name="out"):
        cache = make_cache(self.tmp / ("cache-" + name), pages)
        out_dir = self.tmp / name
        code, out, err = run(["extract", "--cache", str(cache), "--out", str(out_dir)])
        return code, out, err, out_dir


# --------------------------------------------------------------------------
# Fetch: the baseline a polite client keeps

class Fetch(Poe2dbTestCase):
    def fetch(self, *extra, site=None, clock=None, cache=None):
        clock = clock or FakeClock()
        site = site or FakeSite(clock)
        cache = cache or self.tmp / "cache"
        code, out, err = run(["fetch", "--cache", str(cache), *extra], fetcher=site, clock=clock)
        return code, out, err, site, clock, cache

    def test_reads_robots_first_then_every_page_in_table_order(self):
        code, out, err, site, _, cache = self.fetch()
        self.assertEqual(code, 0, err)
        self.assertEqual(site.urls(), [poe2db.ROBOTS_URL] + URLS)
        index = json.loads((cache / poe2db.CACHE_INDEX).read_text(encoding="utf-8"))
        for slug, url in zip(SLUGS, URLS):
            record = index["pages"][slug]
            self.assertEqual((record["url"], record["status"], record["fetched"], record["file"]),
                             (url, 200, "2026-10-28", slug + ".html"))
            self.assertTrue((cache / record["file"]).is_file())
        self.assertIn("fetched 4 page(s)", out)

    def test_user_agent_names_this_tool_and_repository(self):
        _, _, _, site, _, _ = self.fetch()
        self.assertEqual({ua for _, _, ua in site.calls}, {poe2db.USER_AGENT})
        self.assertIn("ExilePact2-crafting-helper-poe2db", poe2db.USER_AGENT)
        self.assertIn("github.com/S-Borkowski/ExilePact2", poe2db.USER_AGENT)
        # Negative control: not the agent of either sibling tool.
        self.assertNotEqual(poe2db.USER_AGENT, quest_poe2db.USER_AGENT)
        self.assertNotEqual(poe2db.USER_AGENT, waystone_poe2db.USER_AGENT)

    def test_requests_are_spaced_by_the_delay(self):
        _, _, _, site, clock, _ = self.fetch("--delay", "5")
        times = [t for t, _, _ in site.calls]
        self.assertEqual(len(times), 5)
        # The gap runs from the end of one request to the start of the next,
        # so a slow answer (0.4 s here) never shortens it.
        self.assertEqual(clock.sleeps, [5.0] * 4)
        for before, after in zip(times, times[1:]):
            self.assertGreaterEqual(after - (before + 0.4), 5.0)

    def test_default_delay_is_three_seconds(self):
        _, _, _, _, clock, _ = self.fetch()
        self.assertEqual(clock.sleeps, [3.0] * 4)

    def test_crawl_delay_longer_than_the_delay_wins(self):
        clock = FakeClock()
        _, _, err, _, clock, _ = self.fetch(site=FakeSite(clock, robots="robots_crawl_delay.txt"), clock=clock)
        self.assertEqual(clock.sleeps, [7.0] * 4, err)

    def test_delay_under_the_minimum_is_refused(self):
        code, _, err, site, _, cache = self.fetch("--delay", "1.9")
        self.assertNotEqual(code, 0)
        self.assertIn("minimum", err)
        self.assertEqual(site.calls, [])
        self.assertFalse(cache.exists())
        with self.assertRaises(poe2db.ToolError):
            poe2db.fetch(self.tmp / "direct", delay=1.0, fetcher=FakeSite(FakeClock()), clock=FakeClock(),
                         table=TABLE)

    def test_delay_at_the_minimum_is_accepted(self):
        code, _, err, site, clock, _ = self.fetch("--delay", "2")
        self.assertEqual(code, 0, err)
        self.assertEqual(len(site.calls), 5)
        self.assertEqual(clock.sleeps, [2.0] * 4)

    def test_cache_inside_a_git_checkout_is_refused(self):
        clock = FakeClock()
        site = FakeSite(clock)
        inside = ROOT / "tests" / "fixtures" / "crafting_helper" / "never-created-cache"
        code, _, err = run(["fetch", "--cache", str(inside)], fetcher=site, clock=clock)
        self.assertEqual(code, 1)
        self.assertIn("git checkout", err)
        self.assertEqual(site.calls, [])
        self.assertFalse(inside.exists())
        # Control: the same call outside a checkout writes the cache.
        code, _, err, _, _, cache = self.fetch()
        self.assertEqual(code, 0, err)
        self.assertTrue((cache / poe2db.CACHE_INDEX).is_file())

    def test_fresh_cache_is_not_fetched_again(self):
        self.fetch()
        code, out, err, site, _, _ = self.fetch(clock=FakeClock(TODAY + datetime.timedelta(days=6)))
        self.assertEqual(code, 0, err)
        # Not even robots.txt is asked when every page is fresh.
        self.assertEqual(site.calls, [])
        self.assertIn("nothing asked", out)

    def test_stale_cache_is_fetched_again(self):
        self.fetch()
        code, _, err, site, _, _ = self.fetch(clock=FakeClock(TODAY + datetime.timedelta(days=7)))
        self.assertEqual(code, 0, err)
        self.assertEqual(site.urls(), [poe2db.ROBOTS_URL] + URLS)
        # A max-age of 0 always fetches, too.
        code, _, err, site, _, _ = self.fetch("--max-age", "0")
        self.assertEqual(code, 0, err)
        self.assertEqual(len(site.calls), 5)

    def test_only_the_stale_page_is_fetched(self):
        _, _, _, _, _, cache = self.fetch()
        (cache / "Rings.html").unlink()
        code, out, err, site, _, _ = self.fetch()
        self.assertEqual(code, 0, err)
        self.assertEqual(site.urls(), [poe2db.ROBOTS_URL, "https://poe2db.tw/us/Rings"])
        self.assertIn("3 already cached", out)

    def test_robots_disallow_fetches_no_page(self):
        clock = FakeClock()
        code, _, err, site, _, cache = self.fetch(site=FakeSite(clock, robots="robots_disallow_rings.txt"),
                                                  clock=clock)
        self.assertEqual(code, 1)
        self.assertIn("robots.txt", err)
        self.assertIn("Rings", err)
        self.assertEqual(site.urls(), [poe2db.ROBOTS_URL])
        self.assertFalse((cache / "Gloves_str.html").exists())

    def test_missing_robots_allows_the_pages(self):
        # Negative control for the case above: a robots.txt that answers 404
        # (no rules at all) lets every page through.
        clock = FakeClock()
        code, _, err, site, _, _ = self.fetch(site=FakeSite(clock, robots=None), clock=clock)
        self.assertEqual(code, 0, err)
        self.assertEqual(site.urls(), [poe2db.ROBOTS_URL] + URLS)

    def test_failed_page_is_an_error_that_leaves_its_entry_unchanged(self):
        _, _, _, _, _, cache = self.fetch()
        before = json.loads((cache / poe2db.CACHE_INDEX).read_text(encoding="utf-8"))["pages"]["Rings"]
        page = (cache / "Rings.html").read_bytes()
        clock = FakeClock(TODAY + datetime.timedelta(days=8))
        site = FakeSite(clock)
        site.routes["https://poe2db.tw/us/Rings"] = 503
        code, _, err, site, _, _ = self.fetch(site=site, clock=clock)
        self.assertEqual(code, 1)
        self.assertIn("HTTP 503", err)
        # It stops there: Ruby, after Rings in the table, is not asked.
        self.assertEqual(site.urls(), [poe2db.ROBOTS_URL] + URLS[:3])
        index = json.loads((cache / poe2db.CACHE_INDEX).read_text(encoding="utf-8"))["pages"]
        self.assertEqual(index["Rings"], before)
        self.assertEqual((cache / "Rings.html").read_bytes(), page)
        # Control: the pages before it were refreshed.
        self.assertEqual(index["Gloves_str"]["fetched"], "2026-11-05")

    def test_session_only_asks_poe2db(self):
        clock = FakeClock()
        site = FakeSite(clock)
        session = poe2db.Session(site, clock, 3.0)
        for url in ("https://example.com/us/Rings", "http://poe2db.tw/us/Rings", "https://poe2db.tw.example/x"):
            with self.assertRaises(poe2db.ToolError):
                session.get(url)
        self.assertEqual(site.calls, [])
        # Control: a page of the site itself goes through.
        self.assertEqual(session.get("https://poe2db.tw/us/Rings")[0], 200)
        self.assertEqual(len(site.calls), 1)

    def test_help_names_the_three_commands(self):
        out = io.StringIO()
        with contextlib.redirect_stdout(out), self.assertRaises(SystemExit) as caught:
            poe2db.main(["--help"])
        self.assertEqual(caught.exception.code, 0)
        for command in ("fetch", "extract", "check"):
            self.assertIn(command, out.getvalue())

    def test_default_cache_is_the_tools_own_folder(self):
        path = str(poe2db.default_cache_dir())
        self.assertIn("crafting-helper", path)
        self.assertTrue(path.endswith("poe2db-cache"))


# --------------------------------------------------------------------------
# Extract: the files made from the pages

class Extract(Poe2dbTestCase):
    def setUp(self):
        super().setUp()
        self.code, self.out, self.err, self.dir = self.extract_pages()
        self.assertEqual(self.code, 0, self.err)
        self.index = json.loads((self.dir / "index.json").read_text(encoding="utf-8"))
        self.gloves = self.load("classes/gloves.json")
        self.rings = self.load("classes/rings.json")
        self.jewels = self.load("classes/jewels.json")

    def load(self, name):
        return json.loads((self.dir / name).read_text(encoding="utf-8"))

    def mod(self, data, first_line):
        found = [m for m in data["mods"] if m["lines"][0] == first_line]
        self.assertEqual(len(found), 1, "%r: %r" % (first_line, [m["lines"] for m in data["mods"]]))
        return found[0]

    def entry(self, data, name, field="essences"):
        found = [e for e in data[field] if e["name"] == name]
        self.assertEqual(len(found), 1, "%r: %r" % (name, [e["name"] for e in data[field]]))
        return found[0]

    def test_writes_the_index_and_one_file_per_class(self):
        self.assertEqual(sorted(tree(self.dir)), ["classes/gloves.json", "classes/jewels.json",
                                                   "classes/rings.json", "index.json"])
        self.assertEqual(list(self.index), ["schema", "source_note", "classes"])
        self.assertEqual(self.index["classes"], [
            {"class": "Gloves", "file": "classes/gloves.json", "variants": ["str", "int"]},
            {"class": "Rings", "file": "classes/rings.json", "variants": ["all"]},
            {"class": "Jewels", "file": "classes/jewels.json", "variants": ["ruby"]},
        ])
        for words in ("poe2db", "CC BY-NC-SA 3.0", "Grinding Gear Games", "2026-10-24"):
            self.assertIn(words, self.index["source_note"])
        self.assertIn("Gloves: 3 bases, 7 mods, 5 essences, 0 liquid", self.out)

    def test_class_file_fields_and_page_records(self):
        self.assertEqual(list(self.gloves), ["schema", "class", "code", "source_note", "pages", "bases", "mods",
                                             "essences", "liquid"])
        self.assertEqual((self.gloves["class"], self.gloves["code"]), ("Gloves", "Gloves"))
        self.assertEqual([(p["variant"], p["url"], p["fetched"]) for p in self.gloves["pages"]],
                         [("str", "https://poe2db.tw/us/Gloves_str", "2026-10-28"),
                          ("int", "https://poe2db.tw/us/Gloves_int", "2026-10-28")])
        self.assertEqual(self.jewels["pages"][0]["url"], "https://poe2db.tw/us/Ruby")

    def test_ranges_with_either_dash_negative_decimal_and_lone_numbers(self):
        swarm = self.mod(self.gloves, "# Lanternmoths circle you")
        # An em dash (0.5—1.5) and an en dash (-10–-5), negative and decimal.
        self.assertEqual(swarm["tiers"]["str"][0]["values"], [[0.5, 1.5], [-10, -5]])
        adds = self.mod(self.gloves, "Adds # to # Glimmer Damage to Attacks")
        self.assertEqual(adds["tiers"]["str"][0]["values"], [[1, 1], [4, 4]])
        lantern = self.mod(self.gloves, "+# to Level of all Lantern Skills")
        self.assertEqual(lantern["tiers"]["str"][0]["values"], [[1, 1]])
        hollowed = self.mod(self.jewels, "Lanterns in Radius also grant # Glimmer per Second")
        self.assertEqual(hollowed["tiers"]["ruby"][0]["values"], [[0.1, 0.1]])

    def test_br_starts_a_line_and_hidden_stats_are_dropped(self):
        swarm = self.mod(self.gloves, "# Lanternmoths circle you")
        self.assertEqual(swarm["lines"], ["# Lanternmoths circle you", "Lanternmoths deal #% less Damage"])
        # Control: the page does carry the hidden stat.
        self.assertIn("hidden moth orbit radius [20]", fixture_text("Gloves_str"))
        rings = {b["name"]: b for b in self.rings["bases"]}
        self.assertEqual(rings["Copper Circle"]["implicits"],
                         [{"lines": ["Adds # to # Glimmer Damage", "Lanterns burn #% longer"],
                           "values": [[1, 1], [3, 3], [10, 10]]}])
        # An implicit made only of hidden stats is no implicit.
        self.assertEqual(rings["Moth Loop"]["implicits"], [])

    def test_keyword_text_is_kept(self):
        self.mod(self.gloves, "+# to maximum Vigour")
        desecrated = self.mod(self.gloves, "+# to Vigour and Resolve")
        self.assertEqual(desecrated["section"], "desecrated")
        self.assertEqual(desecrated["family"], ["Vigour", "Resolve"])

    def test_every_template_holds_no_digit_or_markup(self):
        for data in (self.gloves, self.rings, self.jewels):
            texts = [line for m in data["mods"] for line in m["lines"]]
            texts += [line for e in data["essences"] + data["liquid"] for line in e["lines"]]
            texts += [line for b in data["bases"] for i in b["implicits"] for line in i["lines"]]
            self.assertTrue(texts)
            for line in texts:
                self.assertIsNone(re.search(r"[0-9\[\]<>]", line), line)

    def test_tiers_numbered_per_variant(self):
        vigour = self.mod(self.gloves, "+# to maximum Vigour")
        self.assertEqual(list(vigour["tiers"]), ["str", "int"])
        # Tier 1 is the highest item level, separately in each variant.
        self.assertEqual([(t["tier"], t["ilvl"], t["name"], t["values"]) for t in vigour["tiers"]["str"]],
                         [(1, 40, "Stalwart", [[15, 19]]), (2, 20, "Hardy", [[10, 14]]),
                          (3, 1, "Sturdy", [[5, 9]])])
        self.assertEqual([(t["tier"], t["ilvl"], t["name"]) for t in vigour["tiers"]["int"]],
                         [(1, 30, "Resolute"), (2, 1, "Sturdy")])
        # A mod one variant lacks has no tiers there at all.
        self.assertEqual(list(self.mod(self.gloves, "# Lanternmoths circle you")["tiers"]), ["str"])

    def test_equal_item_levels_put_higher_values_first(self):
        ward = self.mod(self.gloves, "#% increased Glimmer Ward")
        self.assertEqual([(t["tier"], t["ilvl"], t["name"], t["values"]) for t in ward["tiers"]["str"]],
                         [(1, 30, "Glowing", [[10, 12]]), (2, 10, "Gleaming", [[7, 9]]),
                          (3, 10, "Glinting", [[4, 6]])])

    def test_one_family_with_two_templates_is_two_mods(self):
        lantern = self.mod(self.gloves, "+# to Level of all Lantern Skills")
        moth = self.mod(self.gloves, "+# to Level of all Moth Skills")
        self.assertEqual(lantern["family"], moth["family"])
        self.assertEqual(lantern["family"], ["GlimmerSkillLevel"])
        self.assertNotEqual(lantern["id"], moth["id"])

    def test_ids_are_kebab_and_unique(self):
        for data in (self.gloves, self.rings, self.jewels):
            ids = [m["id"] for m in data["mods"]]
            self.assertEqual(len(ids), len(set(ids)))
            for ident in ids:
                self.assertRegex(ident, r"^[a-z0-9]+(-[a-z0-9]+)*$")
        self.assertEqual(self.mod(self.gloves, "+# to maximum Vigour")["id"], "prefix-to-maximum-vigour")
        self.assertEqual(self.mod(self.gloves, "+# to Vigour and Resolve")["id"],
                         "desecrated-suffix-to-vigour-and-resolve")

    def test_weights_not_established(self):
        weights = {(d["class"], p["variant"]): p["weights"]
                   for d in (self.gloves, self.rings, self.jewels) for p in d["pages"]}
        # Equipment normal rows carry real spawn weights ...
        self.assertEqual(weights[("Gloves", "str")], {"normal": "spawn", "desecrated": "not-established"})
        # ... desecrated rows are all 1, and a page without them says none ...
        self.assertEqual(weights[("Gloves", "int")], {"normal": "spawn", "desecrated": "none"})
        self.assertEqual(weights[("Rings", "all")], {"normal": "spawn", "desecrated": "none"})
        # ... and a jewel page's normal rows are all 1: not weights either.
        self.assertEqual(weights[("Jewels", "ruby")], {"normal": "not-established",
                                                        "desecrated": "not-established"})
        self.assertEqual(self.mod(self.gloves, "+# to maximum Vigour")["tiers"]["str"][0]["weight"], 1000)
        # A desecrated row without DropChance has a null weight, not a zero.
        hollowed = self.mod(self.jewels, "Lanterns in Radius also grant # Glimmer per Second")
        self.assertIsNone(hollowed["tiers"]["ruby"][0]["weight"])

    def test_ignored_groups_are_counted_per_page(self):
        self.assertEqual([p["ignored"] for p in self.gloves["pages"]],
                         [{"corrupted": 2, "socketable": 1}, {"bonded": 1}])
        self.assertEqual(self.rings["pages"][0]["ignored"], {"breach_caster": 1, "corrupted": 1})
        # Empty groups are not listed.
        self.assertEqual(self.jewels["pages"][0]["ignored"], {})
        for data in (self.gloves, self.rings):
            for m in data["mods"]:
                self.assertNotIn("invented", " ".join(m["lines"]))

    def test_duplicate_essence_rows_are_one_entry(self):
        # Two rows of different Code with the same display are one entry ...
        self.assertEqual(fixture_text("Gloves_str").count(">Lesser Essence of Warding<"), 2)
        self.entry(self.gloves, "Lesser Essence of Warding")
        # ... and one row on both variant pages is one entry naming both.
        vigour = self.entry(self.gloves, "Lesser Essence of Vigour")
        self.assertEqual(vigour["variants"], ["str", "int"])
        self.assertEqual(vigour["slug"], "Lesser_Essence_of_Vigour")

    def test_essence_matches_tier(self):
        vigour = self.entry(self.gloves, "Lesser Essence of Vigour")
        self.assertEqual(vigour["matches"]["str"], {"mod": "prefix-to-maximum-vigour", "tier": 2})
        # Control: the int page has no ilvl-20 tier of the family, so no match.
        self.assertIsNone(vigour["matches"]["int"])
        # Two tiers of one mod share the item level: the one whose values are equal.
        warding = self.entry(self.gloves, "Lesser Essence of Warding")
        self.assertEqual(warding["matches"], {"str": {"mod": "prefix-increased-glimmer-ward", "tier": 2}})
        # Two mods of one family share the item level: the one whose values are equal.
        moths = self.entry(self.gloves, "Essence of Moths")
        self.assertEqual(moths["matches"], {"str": {"mod": "suffix-to-level-of-all-moth-skills", "tier": 1}})
        # An essence-only mark and an alloy match nothing.
        hollow = self.entry(self.gloves, "Essence of the Hollow")
        self.assertEqual((hollow["group"], hollow["removes"], hollow["matches"]),
                         ("perfect_essence", True, {"str": None}))
        alloy = self.entry(self.gloves, "Fluttering Alloy")
        self.assertEqual((alloy["alloy"], alloy["perfect"], alloy["matches"]), (True, False, {"str": None}))
        dusk = self.entry(self.rings, "Essence of Dusk")
        self.assertEqual(dusk["matches"], {"all": {"mod": "suffix-to-dusk-resistance", "tier": 1}})
        perfect = self.entry(self.rings, "Perfect Essence of Glow")
        self.assertEqual((perfect["perfect"], perfect["matches"]), (True, {"all": None}))

    def test_liquid_emotions_are_kept_and_matched(self):
        self.assertEqual([e["name"] for e in self.jewels["liquid"]],
                         ["Diluted Liquid Wonder", "Potent Liquid Wistfulness"])
        wonder = self.entry(self.jewels, "Diluted Liquid Wonder", "liquid")
        self.assertEqual(list(wonder), poe2db.LIQUID_KEYS)
        self.assertEqual(wonder["matches"], {"ruby": {"mod": "prefix-increased-glimmer", "tier": 1}})
        self.assertIsNone(self.entry(self.jewels, "Potent Liquid Wistfulness", "liquid")["matches"]["ruby"])
        self.assertEqual(self.jewels["essences"], [])
        self.assertEqual(self.gloves["liquid"], [])

    def test_bases_per_variant_with_required_level(self):
        self.assertEqual([(b["name"], b["variant"], b["req_level"]) for b in self.gloves["bases"]],
                         [("Lanternguard Mitts", "str", 33), ("Mothsilk Wraps", "int", 12),
                          ("Tinplate Mitts", "str", None)])
        lantern = self.gloves["bases"][0]
        self.assertEqual(lantern["implicits"], [{"lines": ["#% increased Lantern Glow"], "values": [[4, 6]]}])
        self.assertEqual([(b["name"], b["req_level"]) for b in self.rings["bases"]],
                         [("Copper Circle", 5), ("Lantern Band", 12), ("Moth Loop", None)])
        # A jewel's base is the page's own item; the unique tab is not read.
        self.assertEqual(self.jewels["bases"], [{"name": "Ruby", "variant": "ruby", "req_level": None,
                                                 "implicits": []}])
        self.assertNotIn("Invented Unique", json.dumps(self.rings))

    def test_output_is_deterministic(self):
        again = self.extract_pages(name="again")[3]
        self.assertEqual(tree(self.dir), tree(again))
        # Rows and bases in another page order give the same files.
        def reverse(view):
            for key in ("normal", "essence", "perfect_essence"):
                view[key].reverse()
        html = edit_view(fixture_text("Gloves_str"), reverse)
        cards = re.findall(r'<div class="col">.*?</div></div></div></div></div>\n', html)
        self.assertEqual(len(cards), 2)
        html = html.replace(cards[0] + cards[1], cards[1] + cards[0])
        self.assertNotEqual(html, fixture_text("Gloves_str"))
        code, _, err, shuffled = self.extract_pages({"Gloves_str": html}, "shuffled")
        self.assertEqual(code, 0, err)
        self.assertEqual(tree(self.dir), tree(shuffled))

    def test_files_are_utf8_lf_with_a_trailing_newline(self):
        for name, raw in tree(self.dir).items():
            self.assertNotIn(b"\r", raw, name)
            self.assertTrue(raw.endswith(b"}\n"), name)
            self.assertEqual(raw.decode("utf-8"), poe2db.dumps(json.loads(raw)), name)

    def test_check_passes_on_what_extract_wrote(self):
        code, out, err = run(["check", "--data", str(self.dir)])
        self.assertEqual(code, 0, out + err)
        self.assertIn("clean", out)
        for data, item_class in zip((self.gloves, self.rings, self.jewels), TABLE):
            self.assertEqual(poe2db.check_data(data, item_class), [])


# --------------------------------------------------------------------------
# Surprises: a page the parser does not know stops it, and writes nothing

class Surprises(Poe2dbTestCase):
    def assert_refused(self, slug, html, *words):
        out_dir = self.tmp / "out"
        (out_dir / "classes").mkdir(parents=True, exist_ok=True)
        (out_dir / "index.json").write_text("untouched\n", encoding="utf-8")
        (out_dir / "classes" / "gloves.json").write_text("untouched\n", encoding="utf-8")
        before = tree(out_dir)
        cache = make_cache(self.tmp / "cache", {slug: html})
        code, _, err = run(["extract", "--cache", str(cache), "--out", str(out_dir)])
        self.assertEqual(code, 1, err)
        self.assertIn(slug, err)
        for word in words:
            self.assertIn(word, err)
        self.assertEqual(tree(out_dir), before)
        return err

    def test_good_pages_extract(self):
        # Positive control for every case below: the same harness, good pages.
        code, _, err, out_dir = self.extract_pages()
        self.assertEqual(code, 0, err)
        self.assertEqual(len(tree(out_dir)), 4)

    def test_schema_surprise_leaves_files_alone(self):
        # Rings, the second class, breaks after Gloves parsed cleanly: still
        # nothing at all is written, not even the Gloves file.
        html = edit_view(fixture_text("Rings"), lambda v: v["opt"].__setitem__("ItemClassesCode", "Amulet"))
        self.assert_refused("Rings", html, "ItemClassesCode", "Amulet")
        self.assertFalse((self.tmp / "out" / "classes" / "rings.json").exists())

    def test_no_mods_view(self):
        html = fixture_text("Ruby").replace("new ModsView(", "new ModsTable(")
        self.assert_refused("Ruby", html, "ModsView")

    def test_mods_view_that_is_not_json(self):
        html = fixture_text("Rings").replace('"opt": {', '"opt" {', 1)
        self.assert_refused("Rings", html, "not JSON")

    def test_variants_that_differ_from_the_table(self):
        html = edit_view(fixture_text("Gloves_int"),
                         lambda v: v["baseitem"]["opts"].__setitem__("dex", "dex"))
        self.assert_refused("Gloves_int", html, "opts", "dex")

    def test_normal_group_missing_or_empty(self):
        self.assert_refused("Rings", edit_view(fixture_text("Rings"), lambda v: v.pop("normal")), "normal")
        self.assert_refused("Rings", edit_view(fixture_text("Rings"), lambda v: v.__setitem__("normal", [])),
                            "normal")

    def test_kept_row_missing_a_key(self):
        html = edit_view(fixture_text("Gloves_str"), lambda v: v["essence"][1].pop("Level"))
        self.assert_refused("Gloves_str", html, "essence row 2", "Level")
        # Control: DropChance is optional; the row extracts with a null weight.
        html = edit_view(fixture_text("Rings"), lambda v: v["normal"][0].pop("DropChance"))
        code, _, err, out_dir = self.extract_pages({"Rings": html}, "no-drop-chance")
        self.assertEqual(code, 0, err)

    def test_generation_type_other_than_prefix_or_suffix(self):
        html = edit_view(fixture_text("Ruby"), lambda v: v["liquid"][0].__setitem__("ModGenerationTypeID", "3"))
        self.assert_refused("Ruby", html, "liquid row 1", "generation type")

    def test_value_that_will_not_parse(self):
        html = fixture_text("Rings").replace('(3<span class=\\"ndash\\">—</span>5)', "(3<span>—</span>?)", 1)
        self.assertNotEqual(html, fixture_text("Rings"))
        self.assert_refused("Rings", html, "normal row 1", "?")
        html = edit_view(fixture_text("Rings"), lambda v: v["normal"][1].__setitem__("Level", "forty"))
        self.assert_refused("Rings", html, "Level", "forty")

    def test_base_count_against_the_tab_header(self):
        html = fixture_text("Rings").replace("Rings Item /3", "Rings Item /4")
        self.assert_refused("Rings", html, "RingsItem", "/4", "3 bases")

    def test_unknown_non_empty_group(self):
        html = edit_view(fixture_text("Gloves_str"), lambda v: v.__setitem__("harbinger", [{"Name": "x"}]))
        self.assert_refused("Gloves_str", html, "harbinger")
        # A legacy group every page carries empty is a surprise once it is not.
        html = edit_view(fixture_text("Gloves_str"), lambda v: v.__setitem__("delve", [{"Name": "x"}]))
        self.assert_refused("Gloves_str", html, "delve")
        # Control: an empty unknown group is not a surprise.
        html = edit_view(fixture_text("Gloves_str"), lambda v: v.__setitem__("harbinger", []))
        self.assertEqual(self.extract_pages({"Gloves_str": html}, "empty-group")[0], 0)

    def test_jewel_page_for_another_jewel(self):
        html = edit_view(fixture_text("Ruby"), lambda v: v["baseitem"].__setitem__("Code", "Emerald"))
        self.assert_refused("Ruby", html, "Emerald")

    def test_empty_cache_says_fetch_first(self):
        cache = self.tmp / "empty-cache"
        cache.mkdir()
        code, _, err = run(["extract", "--cache", str(cache), "--out", str(self.tmp / "x")])
        self.assertEqual(code, 1)
        self.assertIn("fetch", err)
        self.assertFalse((self.tmp / "x").exists())


# --------------------------------------------------------------------------
# CheckData: the rules the files are held to

class CheckData(Poe2dbTestCase):
    @classmethod
    def setUpClass(cls):
        tmp = tempfile.TemporaryDirectory()
        try:
            cache = make_cache(Path(tmp.name) / "cache")
            files = poe2db.extract(cache, Path(tmp.name) / "out", out=io.StringIO(), table=TABLE)
        finally:
            tmp.cleanup()
        cls.gloves = files["classes/gloves.json"]
        cls.jewels = files["classes/jewels.json"]
        cls.files = files

    def broken(self, change, data=None):
        data = copy.deepcopy(data or self.gloves)
        change(data)
        return poe2db.check_data(data, TABLE[0] if data["class"] == "Gloves" else None)

    def test_clean_file_passes(self):
        self.assertEqual(poe2db.check_data(self.gloves, TABLE[0]), [])
        self.assertEqual(poe2db.check_data(self.jewels, TABLE[2]), [])

    def test_digit_in_a_template_is_refused(self):
        problems = self.broken(lambda d: d["mods"][0]["lines"].__setitem__(0, "5% increased Glimmer Ward"))
        self.assertTrue(any("digit" in p for p in problems), problems)
        self.assertTrue(self.broken(lambda d: d["essences"][0]["lines"].__setitem__(0, "+2 to Moth Skills")))
        self.assertTrue(self.broken(lambda d: d["bases"][0]["implicits"][0]["lines"].__setitem__(0, "<b>#</b>")))
        self.assertTrue(self.broken(lambda d: d["mods"][0]["lines"].__setitem__(0, "radius [#]")))

    def test_values_that_differ_from_the_hash_count_are_refused(self):
        problems = self.broken(lambda d: d["mods"][0]["tiers"]["str"][0]["values"].append([1, 2]))
        self.assertTrue(any("values for" in p for p in problems), problems)
        self.assertTrue(self.broken(lambda d: d["essences"][0]["values"].clear()))
        self.assertTrue(self.broken(lambda d: d["mods"][0]["tiers"]["str"][0].__setitem__("values", [[1]])))

    def test_tier_gap_is_refused(self):
        problems = self.broken(lambda d: d["mods"][0]["tiers"]["str"][2].__setitem__("tier", 4))
        self.assertTrue(any("without gaps" in p for p in problems), problems)
        self.assertTrue(self.broken(lambda d: d["mods"][0]["tiers"]["str"].reverse()))

    def test_dangling_match_is_refused(self):
        def dangle(d):
            d["essences"][0]["matches"]["str"]["mod"] = "prefix-no-such-mod"
        problems = self.broken(dangle)
        self.assertTrue(any("names no mod" in p for p in problems), problems)
        # A tier the mod does not have in that variant dangles too.
        def no_tier(d):
            d["essences"][0]["matches"]["str"]["tier"] = 9
        self.assertTrue(self.broken(no_tier))

    def test_other_breaks_are_refused(self):
        self.assertTrue(self.broken(lambda d: d["pages"][0].__setitem__("fetched", "yesterday")))
        self.assertTrue(self.broken(lambda d: d["pages"][0]["weights"].__setitem__("normal", "maybe")))
        self.assertTrue(self.broken(lambda d: d["mods"][1].__setitem__("id", d["mods"][0]["id"])))
        self.assertTrue(self.broken(lambda d: d["mods"].reverse()))
        self.assertTrue(self.broken(lambda d: d["mods"][0].__setitem__("kind", "implicit")))
        self.assertTrue(self.broken(lambda d: d["essences"][0].__setitem__("variants", ["dex"])))
        self.assertTrue(self.broken(lambda d: d["bases"][0].__setitem__("variant", "dex")))
        self.assertTrue(self.broken(lambda d: d.__setitem__("code", "Glove")))
        self.assertTrue(self.broken(lambda d: d.__setitem__("extra", 1)))

    def test_check_command_reports_each_problem(self):
        out_dir = self.tmp / "data"
        for name, data in self.files.items():
            path = out_dir / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(poe2db.dumps(data), encoding="utf-8", newline="\n")
        code, out, err = run(["check", "--data", str(out_dir)])
        self.assertEqual(code, 0, out + err)
        # A checkout with CRLF line endings is the same content.
        crlf = (out_dir / "classes" / "rings.json")
        crlf.write_bytes(crlf.read_bytes().replace(b"\n", b"\r\n"))
        self.assertEqual(run(["check", "--data", str(out_dir)])[0], 0)
        broken = copy.deepcopy(self.gloves)
        broken["mods"][0]["lines"][0] = "5% increased Glimmer Ward"
        (out_dir / "classes" / "gloves.json").write_text(poe2db.dumps(broken), encoding="utf-8", newline="\n")
        (out_dir / "classes" / "jewels.json").unlink()
        code, out, _ = run(["check", "--data", str(out_dir)])
        self.assertEqual(code, 1)
        self.assertIn("classes/gloves.json", out)
        self.assertIn("digit", out)
        self.assertIn("classes/jewels.json: missing", out)
        # A hand-reformatted file is not what extract writes.
        (out_dir / "classes" / "gloves.json").write_text(json.dumps(self.gloves, indent=4) + "\n",
                                                          encoding="utf-8")
        self.assertIn("not written as extract writes it", run(["check", "--data", str(out_dir)])[1])
        shutil.rmtree(out_dir)
        code, out, _ = run(["check", "--data", str(out_dir)])
        self.assertEqual(code, 1)
        self.assertIn("extract", out)


# --------------------------------------------------------------------------
# The real class table

class ClassTable(unittest.TestCase):
    def test_twenty_two_classes_on_fifty_two_pages(self):
        table = poe2db.CLASS_TABLE
        self.assertEqual(len(table), 22)
        self.assertEqual(sum(len(c.pages) for c in table), 52)
        self.assertEqual(len({p.slug for c in table for p in c.pages}), 52)
        names = [c.name for c in table]
        self.assertEqual(names[:5], ["Gloves", "Boots", "Helmets", "Body Armours", "Shields"])
        self.assertEqual(names[-3:], ["Amulets", "Rings", "Jewels"])
        by_name = {c.name: c for c in table}
        self.assertEqual([p.slug for p in by_name["Body Armours"].pages][-1], "Body_Armours_str_dex_int")
        self.assertEqual([p.variant for p in by_name["Shields"].pages], ["str", "str_dex", "str_int"])
        self.assertEqual(by_name["Quarterstaves"].code, "Warstaff")
        self.assertEqual([p.variant for p in by_name["Rings"].pages], ["all"])
        self.assertEqual(by_name["Jewels"].pages[4], poe2db.Page("time-lost-ruby", "Time-Lost_Ruby"))
        self.assertTrue(by_name["Jewels"].jewel)
        self.assertEqual(poe2db.class_file(by_name["One Hand Maces"]), "classes/one-hand-maces.json")
        self.assertEqual(poe2db.build_index()["classes"][3],
                         {"class": "Body Armours", "file": "classes/body-armours.json",
                          "variants": ["str", "dex", "int", "str_dex", "str_int", "dex_int", "str_dex_int"]})

    def test_default_data_folder_is_the_apps(self):
        self.assertEqual(poe2db.DEFAULT_DATA, ROOT / "apps" / "crafting-helper" / "data")


if __name__ == "__main__":
    unittest.main()
