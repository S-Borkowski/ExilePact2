"""tools/waystone_roller/poe2db.py: fetch and extract the waystone mod data, with no network.

Every case runs against an injected fetcher that serves synthetic pages from
`tests/fixtures/waystone_roller/poe2db/` (the site's shape, invented modifier
lines; never a saved page) and an injected clock, so nothing here sends a
request or sleeps. Caches and output files live in temporary directories.

The baseline is the site as a polite client must treat it (`Fetch`): what
robots.txt disallows is never asked for, the two requests are spaced by the
delay, a fresh cache is not asked again, and nothing is written inside a git
checkout. The targets are the data file `extract` makes from the page
(`Extract`), its loud failure on a page whose structure it does not know
(`Broken`), and the checked-in file it wrote (`ShippedData`). Each keeps a
negative control beside it: an allow-all robots.txt that does fetch the
page, a delay at the minimum that is accepted, a stale cache that is
fetched, rows that look alike but are not merged, a well-formed page that
extracts, and a data file the check refuses.
"""
import contextlib
import copy
import datetime
import io
import json
import re
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from tools.quest_tracker import poe2db as quest_poe2db  # noqa: E402
from tools.waystone_roller import poe2db  # noqa: E402

FIXTURES = ROOT / "tests" / "fixtures" / "waystone_roller" / "poe2db"
SHIPPED = ROOT / "apps" / "waystone-roller" / "data" / "waystone-mods.json"
TODAY = datetime.date(2026, 10, 28)


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

    def __init__(self, clock, robots="robots.txt", page="Waystones.html"):
        self.clock = clock
        self.calls = []
        self.routes = {poe2db.PAGE_URL: page}
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


def run(argv, fetcher=None, clock=None):
    out, err = io.StringIO(), io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
        code = poe2db.main(argv, fetcher=fetcher, clock=clock)
    return code, out.getvalue(), err.getvalue()


def make_cache(folder, html, fetched="2026-10-28"):
    """A cache as fetch leaves it, holding `html` as the Waystones page."""
    folder.mkdir(parents=True, exist_ok=True)
    name = poe2db.PAGE_FILE
    (folder / name).write_text(html, encoding="utf-8", newline="\n")
    index = {"schema": 1, "source": poe2db.PAGE_URL,
             "pages": {poe2db.PAGE_SLUG: {"slug": poe2db.PAGE_SLUG, "url": poe2db.PAGE_URL,
                                          "fetched": fetched, "status": 200, "file": name}}}
    (folder / poe2db.CACHE_INDEX).write_text(json.dumps(index), encoding="utf-8")
    return folder


def fixture_text(name="Waystones.html"):
    return (FIXTURES / name).read_text(encoding="utf-8")


class Poe2dbTestCase(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)
        # Every positive case below writes here; it must not be a checkout.
        self.assertIsNone(quest_poe2db.git_tree_of(self.tmp), "the temp folder sits inside a git checkout")

    def extract_page(self, html, name="out.json"):
        cache = make_cache(self.tmp / ("cache-" + name), html)
        out_path = self.tmp / name
        code, out, err = run(["extract", "--cache", str(cache), "--out", str(out_path)])
        return code, out, err, out_path


# --------------------------------------------------------------------------
# Fetch: the baseline a polite client keeps

class Fetch(Poe2dbTestCase):
    def fetch(self, *extra, site=None, clock=None):
        clock = clock or FakeClock()
        site = site or FakeSite(clock)
        cache = self.tmp / "cache"
        code, out, err = run(["fetch", "--cache", str(cache), *extra], fetcher=site, clock=clock)
        return code, out, err, site, clock, cache

    def test_reads_robots_first_then_only_the_waystones_page(self):
        code, out, err, site, _, cache = self.fetch()
        self.assertEqual(code, 0, err)
        self.assertEqual(site.urls(), [poe2db.ROBOTS_URL, "https://poe2db.tw/us/Waystones"])
        self.assertTrue((cache / poe2db.PAGE_FILE).is_file())
        index = json.loads((cache / poe2db.CACHE_INDEX).read_text(encoding="utf-8"))
        record = index["pages"][poe2db.PAGE_SLUG]
        self.assertEqual((record["status"], record["fetched"]), (200, "2026-10-28"))
        self.assertIn("Waystones", out)

    def test_user_agent_names_this_tool_and_repository(self):
        _, _, _, site, _, _ = self.fetch()
        agents = {ua for _, _, ua in site.calls}
        self.assertEqual(agents, {poe2db.USER_AGENT})
        self.assertIn("waystone-roller", poe2db.USER_AGENT)
        self.assertIn("github.com/S-Borkowski/ExilePact2", poe2db.USER_AGENT)
        # Negative control: not the quest tracker's agent, which the shared
        # session would have sent.
        self.assertNotEqual(poe2db.USER_AGENT, quest_poe2db.USER_AGENT)

    def test_requests_are_spaced_by_the_delay(self):
        _, _, _, site, clock, _ = self.fetch("--delay", "5")
        times = [t for t, _, _ in site.calls]
        self.assertEqual(len(times), 2)
        self.assertGreaterEqual(times[1] - times[0], 5.0)
        # The gap runs from the end of one request to the start of the next,
        # so a slow answer (0.4 s here) never shortens it.
        self.assertEqual(clock.sleeps, [5.0])
        self.assertGreaterEqual(times[1] - (times[0] + 0.4), 5.0)

    def test_delay_under_the_minimum_is_refused(self):
        code, _, err, site, _, cache = self.fetch("--delay", "1.9")
        self.assertNotEqual(code, 0)
        self.assertIn("minimum", err)
        self.assertEqual(site.calls, [])
        self.assertFalse(cache.exists())
        with self.assertRaises(poe2db.ToolError):
            poe2db.fetch(self.tmp / "direct", delay=1.0, fetcher=FakeSite(FakeClock()), clock=FakeClock())

    def test_delay_at_the_minimum_is_accepted(self):
        code, _, err, site, _, _ = self.fetch("--delay", "2")
        self.assertEqual(code, 0, err)
        self.assertEqual(len(site.calls), 2)

    def test_cache_inside_a_git_checkout_is_refused(self):
        clock = FakeClock()
        site = FakeSite(clock)
        inside = ROOT / "tests" / "fixtures" / "waystone_roller" / "never-created-cache"
        code, _, err = run(["fetch", "--cache", str(inside)], fetcher=site, clock=clock)
        self.assertEqual(code, 1)
        self.assertIn("git checkout", err)
        self.assertEqual(site.calls, [])
        self.assertFalse(inside.exists())

    def test_cache_outside_a_checkout_is_written(self):
        code, _, err, _, _, cache = self.fetch()
        self.assertEqual(code, 0, err)
        self.assertTrue((cache / poe2db.CACHE_INDEX).is_file())

    def test_fresh_cache_is_not_fetched_again(self):
        clock = FakeClock()
        self.fetch(clock=clock)
        code, out, err, site, _, _ = self.fetch(clock=FakeClock(TODAY + datetime.timedelta(days=6)))
        self.assertEqual(code, 0, err)
        self.assertEqual(site.calls, [])
        self.assertIn("cached", out)

    def test_stale_cache_is_fetched_again(self):
        self.fetch()
        code, out, err, site, _, _ = self.fetch(clock=FakeClock(TODAY + datetime.timedelta(days=7)))
        self.assertEqual(code, 0, err)
        self.assertEqual(site.urls(), [poe2db.ROBOTS_URL, poe2db.PAGE_URL])
        # A max-age of 0 always fetches, too.
        code, _, err, site, _, _ = self.fetch("--max-age", "0")
        self.assertEqual(code, 0, err)
        self.assertEqual(len(site.calls), 2)

    def test_robots_disallow_skips_the_page(self):
        clock = FakeClock()
        site = FakeSite(clock, robots="robots_disallow_waystones.txt")
        code, _, err, site, _, cache = self.fetch(site=site, clock=clock)
        self.assertEqual(code, 1)
        self.assertIn("robots.txt", err)
        self.assertEqual(site.urls(), [poe2db.ROBOTS_URL])
        self.assertFalse((cache / poe2db.PAGE_FILE).exists())

    def test_missing_robots_allows_the_page(self):
        # Negative control for the case above: a robots.txt that answers 404
        # (no rules at all) lets the page through.
        clock = FakeClock()
        site = FakeSite(clock, robots=None)
        code, _, err, site, _, cache = self.fetch(site=site, clock=clock)
        self.assertEqual(code, 0, err)
        self.assertEqual(site.urls(), [poe2db.ROBOTS_URL, poe2db.PAGE_URL])
        self.assertTrue((cache / poe2db.PAGE_FILE).is_file())

    def test_failed_page_is_an_error_and_leaves_no_page(self):
        clock = FakeClock()
        site = FakeSite(clock)
        site.routes[poe2db.PAGE_URL] = 503
        code, _, err, _, _, cache = self.fetch(site=site, clock=clock)
        self.assertEqual(code, 1)
        self.assertIn("HTTP 503", err)
        self.assertFalse((cache / poe2db.PAGE_FILE).exists())

    def test_session_only_asks_poe2db(self):
        clock = FakeClock()
        site = FakeSite(clock)
        session = poe2db.Session(site, clock, 3.0)
        for url in ("https://example.com/us/Waystones", "http://poe2db.tw/us/Waystones"):
            with self.assertRaises(poe2db.ToolError):
                session.get(url)
        self.assertEqual(site.calls, [])
        # Control: the page itself goes through.
        self.assertEqual(session.get(poe2db.PAGE_URL)[0], 200)
        self.assertEqual(len(site.calls), 1)

    def test_help_names_both_commands(self):
        out = io.StringIO()
        with contextlib.redirect_stdout(out), self.assertRaises(SystemExit) as caught:
            poe2db.main(["--help"])
        self.assertEqual(caught.exception.code, 0)
        self.assertIn("fetch", out.getvalue())
        self.assertIn("extract", out.getvalue())

    def test_default_cache_is_the_tools_own_folder(self):
        path = str(poe2db.default_cache_dir())
        self.assertIn("waystone-roller", path)
        self.assertTrue(path.endswith("poe2db-cache"))


# --------------------------------------------------------------------------
# Extract: the data file made from the page

class Extract(Poe2dbTestCase):
    @classmethod
    def setUpClass(cls):
        cls.html = fixture_text()

    def setUp(self):
        super().setUp()
        self.code, self.out, self.err, self.path = self.extract_page(self.html)
        self.assertEqual(self.code, 0, self.err)
        self.data = json.loads(self.path.read_text(encoding="utf-8"))
        self.by_id = {f["id"]: f for f in self.data["families"]}

    def family(self, first_line, second=None):
        found = [f for f in self.data["families"] if f["lines"][0] == first_line
                 and (second is None or f["lines"][1:2] == [second])]
        self.assertEqual(len(found), 1, "%r: %r" % (first_line, [f["lines"] for f in self.data["families"]]))
        return found[0]

    def test_counts_are_printed(self):
        self.assertIn("families: 13 (10 waystone, 3 desecrated)", self.out)
        self.assertIn("skipped rows: 3", self.out)
        self.assertEqual(len(self.data["families"]), 13)

    def test_top_level_fields(self):
        self.assertEqual(list(self.data), ["schema", "source", "sections", "fetched", "source_note",
                                           "families", "skipped"])
        self.assertEqual(self.data["schema"], 1)
        self.assertEqual(self.data["source"], "https://poe2db.tw/us/Waystones")
        self.assertEqual(self.data["sections"], ["WaystonesMods", "DesecratedWaystoneMods"])
        self.assertEqual(self.data["fetched"], "2026-10-28")
        for words in ("poe2db", "CC BY-NC-SA 3.0", "Grinding Gear Games", "2026-10-24"):
            self.assertIn(words, self.data["source_note"])

    def test_values_become_hash_and_hidden_stats_are_dropped(self):
        glimmer = self.family("Monsters deal #% of Damage as Extra Glimmer")
        self.assertEqual(glimmer["lines"], ["Monsters deal #% of Damage as Extra Glimmer",
                                            "#% more Waystones found in Area"])
        self.assertEqual(self.family("Area has patches of Glimmering Ground")["lines"],
                         ["Area has patches of Glimmering Ground", "#% more Rarity of Items found in this Area"])
        # Control: the page does carry the hidden stats and the numbers.
        self.assertIn("map ground effect radius [20]", self.html)
        self.assertIn("map monster glimmer chance [3]", self.html)
        for family in self.data["families"]:
            for line in family["lines"]:
                self.assertIsNone(re.search(r"[0-9\[\]<>]|map ", line), line)

    def test_decimal_ranges_and_single_values_become_hash(self):
        self.family("Monsters gain # Glimmer Charges per second")
        self.family("Area contains # additional Lantern Wells")
        self.family("+# to Hollow Charges")

    def test_keyword_text_is_kept(self):
        self.family("+#% Monster Murk Resistances")

    def test_both_line_breaks_split_lines(self):
        # `<br>` separates stats; `<br/>` separates the two lines of one stat.
        # Both are a line on the item, so a token never spans either.
        self.assertIn("seconds<br/>after", self.html)
        self.assertEqual(self.family("Players in Area are Haunted by Lanternmoths for # seconds")["lines"],
                         ["Players in Area are Haunted by Lanternmoths for # seconds",
                          "after opening a Chest",
                          "#% more Rarity of Items found in this Area"])
        self.assertEqual(self.family("Area is overrun by the Lanternfolk")["lines"][2:],
                         ["#% more Magic and Rare Monsters",
                          "Rare Monsters have #% more chance of Monster Modifiers"])

    def test_identical_rows_merge_with_their_tier_count(self):
        self.assertEqual(self.family("Area has patches of Glimmering Ground")["tiers"], 3)
        self.assertEqual(self.family("Monsters deal #% of Damage as Extra Glimmer")["tiers"], 2)
        self.assertEqual(self.family("Area is lit by # Hollow Lanterns")["tiers"], 2)
        self.assertEqual(self.family("+#% Monster Murk Resistances")["tiers"], 1)

    def test_rows_with_a_different_line_are_not_merged(self):
        # Negative control for the merge: the same first line with a different
        # reward line is two families.
        magic = self.family("Players have #% less Recovery Rate", "#% more Magic and Rare Monsters")
        pack = self.family("Players have #% less Recovery Rate", "#% more Pack size")
        self.assertNotEqual(magic["id"], pack["id"])
        self.assertEqual((magic["tiers"], pack["tiers"]), (1, 1))

    def test_bonus_flags_follow_the_reward_templates(self):
        self.assertEqual(self.family("Area is overrun by the Lanternfolk")["bonus"], [False, False, True, True])
        self.assertEqual(self.family("+#% Monster Murk Resistances")["bonus"], [False, True])
        self.assertEqual(self.family("Monsters gain # Glimmer Charges per second")["bonus"], [False, True])
        # Control: poedb's "Monster have" (sic) is a mod line, and only the
        # exact template "Monsters have #% more Effectiveness" is a reward.
        skitter = self.family("Monster have #% increased Skitter Speed")
        self.assertEqual(skitter["lines"][1], "Monsters have #% more Effectiveness")
        self.assertEqual(skitter["bonus"], [False, True])
        self.assertEqual(self.family("Area contains # additional Lantern Wells")["bonus"], [False])
        for family in self.data["families"]:
            self.assertEqual(len(family["bonus"]), len(family["lines"]))

    def test_a_family_contained_in_another_is_kept(self):
        wells = self.family("Area contains # additional Lantern Wells")
        overrun = self.family("Area is overrun by the Lanternfolk")
        self.assertEqual(wells["lines"], ["Area contains # additional Lantern Wells"])
        self.assertIn(wells["lines"][0], overrun["lines"])

    def test_ids_are_kebab_slugs_with_a_clash_suffix(self):
        self.assertEqual(self.family("Monsters deal #% of Damage as Extra Glimmer")["id"],
                         "monsters-deal-of-damage-as-extra-glimmer")
        self.assertEqual(self.family("+#% Monster Murk Resistances")["id"], "monster-murk-resistances")
        magic = self.family("Players have #% less Recovery Rate", "#% more Magic and Rare Monsters")
        pack = self.family("Players have #% less Recovery Rate", "#% more Pack size")
        self.assertEqual((magic["id"], pack["id"]),
                         ("players-have-less-recovery-rate", "players-have-less-recovery-rate-2"))
        self.assertEqual(len(self.by_id), len(self.data["families"]))

    def test_desecrated_group_kind_and_name(self):
        lanterned = self.family("Area is lit by # Hollow Lanterns")
        self.assertEqual((lanterned["group"], lanterned["kind"], lanterned["name"]),
                         ("desecrated", "prefix", "Lanterned"))
        whispers = self.family("Monsters Whisper # times per second")
        self.assertEqual((whispers["group"], whispers["kind"], whispers["name"]),
                         ("desecrated", "suffix", "of Whispers"))
        murk = self.family("+#% Monster Murk Resistances")
        self.assertEqual((murk["group"], murk["kind"], murk["name"]), ("waystone", "suffix", None))
        groups = [f["group"] for f in self.data["families"]]
        self.assertEqual((groups.count("waystone"), groups.count("desecrated")), (10, 3))
        self.assertEqual(list(self.by_id["monsters-deal-of-damage-as-extra-glimmer"]),
                         ["id", "group", "kind", "name", "lines", "bonus", "tiers"])

    def test_skipped_rows_name_their_section_row_and_reason(self):
        self.assertEqual(self.data["skipped"], [
            {"section": "WaystonesMods", "row": 4, "reason": "empty description"},
            {"section": "DesecratedWaystoneMods", "row": 3, "reason": "only hidden stats"},
            {"section": "DesecratedWaystoneMods", "row": 5, "reason": "only hidden stats"},
        ])

    def test_other_tabs_and_tables_are_ignored(self):
        self.assertIn("A table in another tab", self.html)
        for family in self.data["families"]:
            self.assertNotIn("another tab", " ".join(family["lines"]))
            self.assertNotIn("Scripts", " ".join(family["lines"]))

    def test_output_is_deterministic(self):
        again = self.extract_page(self.html, "again.json")[3]
        self.assertEqual(self.path.read_bytes(), again.read_bytes())
        # Rows in another order give the same families, ids included.
        body = re.search(r'(<tbody id="q7Zk2Lm">\n)(.*?)(</tbody>)', self.html, re.S)
        rows = body.group(2).splitlines(keepends=True)
        shuffled = self.html.replace(body.group(2), "".join(reversed(rows)))
        self.assertNotEqual(shuffled, self.html)
        code, _, err, path = self.extract_page(shuffled, "shuffled.json")
        self.assertEqual(code, 0, err)
        self.assertEqual(json.loads(path.read_text(encoding="utf-8"))["families"], self.data["families"])

    def test_file_is_utf8_lf_with_a_trailing_newline(self):
        raw = self.path.read_bytes()
        self.assertNotIn(b"\r", raw)
        self.assertTrue(raw.endswith(b"}\n"))
        self.assertEqual(raw.decode("utf-8"), poe2db.dumps(self.data))


# --------------------------------------------------------------------------
# Broken: a page the parser does not know fails loudly

class Broken(Poe2dbTestCase):
    def assert_fails(self, html, *words):
        out_path = self.tmp / "sentinel.json"
        out_path.write_text("untouched\n", encoding="utf-8")
        cache = make_cache(self.tmp / "cache", html)
        code, _, err = run(["extract", "--cache", str(cache), "--out", str(out_path)])
        self.assertEqual(code, 1, err)
        for word in words:
            self.assertIn(word, err)
        self.assertEqual(out_path.read_text(encoding="utf-8"), "untouched\n")
        return err

    def test_good_page_extracts(self):
        # Positive control for every case below: the same harness, the good page.
        code, _, err, path = self.extract_page(fixture_text())
        self.assertEqual(code, 0, err)
        self.assertTrue(path.is_file())

    def test_missing_mods_tab_names_the_tab(self):
        self.assert_fails(fixture_text("Waystones_no_mods_tab.html"), "WaystonesMods", "tab")

    def test_unexpected_headers_name_the_tab_and_the_headers(self):
        self.assert_fails(fixture_text("Waystones_bad_headers.html"), "WaystonesMods", "Affix", "Stats")

    def test_unknown_affix_kind_names_the_tab_and_row(self):
        html = fixture_text().replace("<td>1</td><td>Suffix</td>", "<td>1</td><td>Implicit</td>", 1)
        self.assert_fails(html, "WaystonesMods", "row 1", "Implicit")

    def test_row_count_against_the_tab_header(self):
        self.assert_fails(fixture_text().replace("Waystones Mods /14", "Waystones Mods /15"),
                          "WaystonesMods", "15", "14")

    def test_empty_table_names_the_tab(self):
        html = re.sub(r'(<tbody id="Rr90xW">).*?(</tbody>)', r"\1\2", fixture_text(), flags=re.S)
        html = html.replace("Desecrated Waystone Mods /6", "Desecrated Waystone Mods")
        self.assert_fails(html, "DesecratedWaystoneMods")

    def test_empty_cache_says_fetch_first(self):
        cache = self.tmp / "empty-cache"
        cache.mkdir()
        code, _, err = run(["extract", "--cache", str(cache), "--out", str(self.tmp / "x.json")])
        self.assertEqual(code, 1)
        self.assertIn("fetch", err)
        self.assertFalse((self.tmp / "x.json").exists())


# --------------------------------------------------------------------------
# ShippedData: the checked-in file is what extract writes, and only templates

class ShippedData(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        # A Windows checkout with core.autocrlf turns the file's LF into CRLF;
        # the content is what is pinned, not the checkout's line endings.
        cls.text = SHIPPED.read_bytes().decode("utf-8").replace("\r\n", "\n")
        cls.data = json.loads(cls.text)

    def test_passes_the_data_check(self):
        self.assertEqual(poe2db.check_data(self.data), [])

    def test_source_and_fetch_date(self):
        self.assertEqual(self.data["schema"], 1)
        self.assertEqual(self.data["source"], "https://poe2db.tw/us/Waystones")
        self.assertEqual(self.data["sections"], ["WaystonesMods", "DesecratedWaystoneMods"])
        datetime.date.fromisoformat(self.data["fetched"])
        self.assertIn("CC BY-NC-SA 3.0", self.data["source_note"])

    def test_lines_are_short_templates_only(self):
        lines = [line for f in self.data["families"] for line in f["lines"]]
        self.assertTrue(lines)
        for line in lines:
            self.assertIsNone(re.search(r"[\[<0-9]", line), line)
            self.assertLessEqual(len(line), 200, line)
            self.assertEqual(line, line.strip())
        names = [f["name"] for f in self.data["families"] if f["name"] is not None]
        for name in names:
            self.assertIsNone(re.search(r"[\[<0-9]", name), name)

    def test_both_groups_are_present(self):
        groups = {f["group"] for f in self.data["families"]}
        self.assertEqual(groups, {"waystone", "desecrated"})

    def test_written_exactly_as_extract_writes_it(self):
        self.assertNotIn("\r", self.text)
        self.assertEqual(self.text, poe2db.dumps(self.data))
        # Control: a hand-reformatted copy is not what extract writes.
        self.assertNotEqual(json.dumps(self.data, indent=4) + "\n", poe2db.dumps(self.data))
        self.assertEqual(self.data["families"], poe2db.sorted_families(self.data["families"]))

    def test_the_check_refuses_a_bad_file(self):
        # Negative control: the same file with one defect at a time fails.
        def broken(change):
            data = copy.deepcopy(self.data)
            change(data)
            return poe2db.check_data(data)

        self.assertTrue(broken(lambda d: d["families"][0]["lines"].append("Area has 5 more Lanterns")))
        self.assertTrue(broken(lambda d: d["families"][0]["lines"].__setitem__(0, "map radius [x]")))
        self.assertTrue(broken(lambda d: d["families"][0]["lines"].__setitem__(0, "<span>x</span>")))
        self.assertTrue(broken(lambda d: d["families"][0]["lines"].__setitem__(0, "a" * 201)))
        self.assertTrue(broken(lambda d: d["families"][1].__setitem__("id", d["families"][0]["id"])))
        self.assertTrue(broken(lambda d: d["families"][0]["bonus"].append(True)))
        self.assertTrue(broken(lambda d: d["families"][0].__setitem__("group", "unique")))
        self.assertTrue(broken(lambda d: d.__setitem__("source", "https://example.com/")))
        self.assertTrue(broken(lambda d: d.__setitem__("fetched", "yesterday")))
        self.assertTrue(broken(lambda d: d["families"][0].__setitem__("extra", 1)))


if __name__ == "__main__":
    unittest.main()
