"""tools/quest_tracker/poe2db.py: fetch, extract and coverage, with no network.

Every case runs against an injected fetcher that serves synthetic pages from
`tests/fixtures/quest_tracker/poe2db/` (the site's shape, invented quests and
lines; never a saved page) and an injected clock, so nothing here sends a
request or sleeps. Caches and drafts live in temporary directories; the one
`Client.txt` is a synthetic file written by the test.

The baseline is the site as a polite client must treat it: what robots.txt
disallows is never asked for, requests are spaced by the delay, and nothing is
written inside a git checkout. The targets are the draft extract makes from a
quest page, its loud failure on a page whose structure it does not know, and
coverage's counts-only report. Each keeps a negative control beside it: an
allow-all robots.txt that does fetch the page, a delay at the minimum that is
accepted, a cache outside any checkout that is written, a well-formed page
that extracts, and log lines that would match if chat were not dropped.
"""
import contextlib
import datetime
import io
import json
import sys
import tempfile
import unittest
import urllib.robotparser
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from tools.quest_tracker import poe2db  # noqa: E402

FIXTURES = ROOT / "tests" / "fixtures" / "quest_tracker" / "poe2db"
TODAY = datetime.date(2026, 10, 20)
KETTLE = "The_Gilded_Kettle"
LANTERNS = "Lanterns_of_Weeping_Pass"
VAULT = "The_Sealed_Vault"


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
    """Serves the fixture files at the site's URLs and records each request.

    A request takes 0.4 s of fake time, so the delay has something to subtract.
    """

    def __init__(self, clock, robots="robots.txt", index="Quest.html", pages=None):
        self.clock = clock
        self.calls = []
        self.routes = {poe2db.page_url("Quest"): index}
        if robots is not None:
            self.routes[poe2db.ROBOTS_URL] = robots
        for slug, name in (pages or {KETTLE: "The_Gilded_Kettle.html",
                                     LANTERNS: "Lanterns_of_Weeping_Pass.html",
                                     VAULT: "The_Gilded_Kettle.html"}).items():
            self.routes[poe2db.page_url(slug)] = name

    def __call__(self, url, user_agent):
        self.calls.append((self.clock.monotonic(), url, user_agent))
        self.clock.t += 0.4
        name = self.routes.get(url)
        if name is None:
            return 404, b"not found"
        return 200, (FIXTURES / name).read_bytes()

    def urls(self):
        return [url for _, url, _ in self.calls]


def run(argv, fetcher=None, clock=None):
    out, err = io.StringIO(), io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
        code = poe2db.main(argv, fetcher=fetcher, clock=clock)
    return code, out.getvalue(), err.getvalue()


def make_cache(folder, pages, fetched="2026-10-20"):
    """A cache as fetch leaves it: `pages` maps a slug to the fixture it holds."""
    folder.mkdir(parents=True, exist_ok=True)
    index = {"schema": 1, "source": poe2db.page_url("Quest"), "pages": {}}
    for slug, fixture in pages.items():
        name = poe2db.cache_file_name(slug)
        (folder / name).write_bytes((FIXTURES / fixture).read_bytes())
        index["pages"][slug] = {"slug": slug, "url": poe2db.page_url(slug), "fetched": fetched,
                                "status": 200, "file": name}
    poe2db.save_cache_index(folder, index)
    return folder


GOOD_CACHE = {"Quest": "Quest.html", KETTLE: "The_Gilded_Kettle.html",
              LANTERNS: "Lanterns_of_Weeping_Pass.html"}


class Poe2dbTestCase(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)
        # Every positive case below writes here; it must not be a checkout.
        self.assertIsNone(poe2db.git_tree_of(self.tmp), "the temp folder sits inside a git checkout")


class FetchTests(Poe2dbTestCase):
    def test_honours_robots_disallow(self):
        clock = FakeClock()
        site = FakeSite(clock)
        cache = self.tmp / "cache"
        code, out, err = run(["fetch", "--cache", str(cache)], fetcher=site, clock=clock)
        self.assertEqual(code, 0, err)
        urls = site.urls()
        self.assertEqual(urls[0], poe2db.ROBOTS_URL, "robots.txt is read before anything else")
        self.assertIn(poe2db.page_url(KETTLE), urls)
        self.assertIn(poe2db.page_url(LANTERNS), urls)
        self.assertNotIn(poe2db.page_url(VAULT), urls)
        self.assertIn("skip  %s: disallowed by robots.txt" % VAULT, out)
        self.assertNotIn(VAULT, json.loads((cache / "index.json").read_text("utf-8"))["pages"])
        for _, _, user_agent in site.calls:
            self.assertEqual(user_agent, poe2db.USER_AGENT)
        self.assertIn("ExilePact2", poe2db.USER_AGENT)

        # The site's wildcard rule (`/*/marked`), which urllib.robotparser
        # alone reads as a literal prefix and would allow.
        plain = urllib.robotparser.RobotFileParser()
        plain.parse((FIXTURES / "robots.txt").read_text("utf-8").splitlines())
        self.assertTrue(plain.can_fetch(poe2db.USER_AGENT, poe2db.page_url("marked")))
        clock = FakeClock()
        site = FakeSite(clock)
        code, out, err = run(["fetch", "--cache", str(self.tmp / "c2"), "--only", "marked", LANTERNS],
                             fetcher=site, clock=clock)
        self.assertEqual(code, 0, err)
        self.assertNotIn(poe2db.page_url("marked"), site.urls())
        self.assertIn(poe2db.page_url(LANTERNS), site.urls())

        # Negative control: with no robots.txt (404, allow all) the same page is fetched.
        clock = FakeClock()
        site = FakeSite(clock, robots=None)
        code, out, err = run(["fetch", "--cache", str(self.tmp / "c3")], fetcher=site, clock=clock)
        self.assertEqual(code, 0, err)
        self.assertIn(poe2db.page_url(VAULT), site.urls())

    def test_enforces_minimum_delay(self):
        for delay in ("1.5", "1.99", "0", "nan"):
            clock = FakeClock()
            site = FakeSite(clock)
            code, out, err = run(["fetch", "--cache", str(self.tmp / "c"), "--delay", delay],
                                 fetcher=site, clock=clock)
            self.assertEqual(code, 2, delay)
            self.assertIn("minimum", err)
            self.assertEqual(site.calls, [], "no request before the delay is accepted")
        self.assertFalse((self.tmp / "c").exists())

        for delay, expected in (("2", 2.0), (None, poe2db.DEFAULT_DELAY), ("4.5", 4.5)):
            clock = FakeClock()
            site = FakeSite(clock)
            argv = ["fetch", "--cache", str(self.tmp / ("d%s" % delay))]
            if delay is not None:
                argv += ["--delay", delay]
            code, out, err = run(argv, fetcher=site, clock=clock)
            self.assertEqual(code, 0, err)
            times = [t for t, _, _ in site.calls]
            self.assertGreaterEqual(len(times), 4)
            gaps = [b - a for a, b in zip(times, times[1:])]
            self.assertGreaterEqual(min(gaps), expected - 1e-9, (delay, gaps))
            self.assertTrue(clock.sleeps, "the 0.4 s a request took is not the whole delay")

    def test_cache_refuses_git_checkout(self):
        repo = self.tmp / "repo"
        (repo / ".git").mkdir(parents=True)
        worktree = self.tmp / "worktree"
        worktree.mkdir()
        (worktree / ".git").write_text("gitdir: elsewhere\n", encoding="utf-8")
        for cache in (repo, repo / "nested" / "cache", worktree / "cache"):
            clock = FakeClock()
            site = FakeSite(clock)
            code, out, err = run(["fetch", "--cache", str(cache)], fetcher=site, clock=clock)
            self.assertEqual(code, 1, cache)
            self.assertIn("git checkout", err)
            self.assertEqual(site.calls, [], "refused before any request")
        self.assertFalse((repo / "nested").exists())
        self.assertFalse((worktree / "cache").exists())

        cache = make_cache(self.tmp / "outside", GOOD_CACHE)
        code, out, err = run(["extract", "--cache", str(cache), "--out", str(repo / "draft.json")])
        self.assertEqual(code, 1)
        self.assertIn("git checkout", err)
        self.assertFalse((repo / "draft.json").exists())

        # Negative control: the same commands outside any checkout write.
        clock = FakeClock()
        code, out, err = run(["fetch", "--cache", str(self.tmp / "cache")], fetcher=FakeSite(clock), clock=clock)
        self.assertEqual(code, 0, err)
        self.assertTrue((self.tmp / "cache" / "index.json").is_file())
        code, out, err = run(["extract", "--cache", str(cache), "--out", str(self.tmp / "draft.json")])
        self.assertEqual(code, 0, err)
        self.assertTrue((self.tmp / "draft.json").is_file())

    def test_fetch_campaign_only_unless_endgame_asked(self):
        clock = FakeClock()
        site = FakeSite(clock)
        code, out, err = run(["fetch", "--cache", str(self.tmp / "c")], fetcher=site, clock=clock)
        self.assertEqual(code, 0, err)
        urls = site.urls()
        for slug in ("Ashen_Pilgrimage", "Trial_of_Echoes", "Rewards_Tab_Decoy"):
            self.assertNotIn(poe2db.page_url(slug), urls)
        self.assertEqual(urls.count(poe2db.page_url(LANTERNS)), 1, "a quest listed per act is fetched once")

        clock = FakeClock()
        site = FakeSite(clock)
        code, out, err = run(["fetch", "--cache", str(self.tmp / "e"), "--include-endgame"],
                             fetcher=site, clock=clock)
        self.assertEqual(code, 0, err)
        self.assertIn(poe2db.page_url("Ashen_Pilgrimage"), site.urls())
        self.assertIn(poe2db.page_url("Trial_of_Echoes"), site.urls())
        self.assertNotIn(poe2db.page_url("Rewards_Tab_Decoy"), site.urls())

    def test_fetch_skips_pages_cached_within_max_age(self):
        cache = self.tmp / "cache"
        clock = FakeClock()
        code, out, err = run(["fetch", "--cache", str(cache)], fetcher=FakeSite(clock), clock=clock)
        self.assertEqual(code, 0, err)
        index = json.loads((cache / "index.json").read_text("utf-8"))
        record = index["pages"][KETTLE]
        self.assertEqual(record, {"slug": KETTLE, "url": poe2db.page_url(KETTLE), "fetched": "2026-10-20",
                                  "status": 200, "file": "The_Gilded_Kettle.html"})
        self.assertTrue((cache / "The_Gilded_Kettle.html").is_file())

        clock = FakeClock(today=TODAY + datetime.timedelta(days=6))
        site = FakeSite(clock)
        code, out, err = run(["fetch", "--cache", str(cache)], fetcher=site, clock=clock)
        self.assertEqual(code, 0, err)
        self.assertEqual(site.urls(), [poe2db.ROBOTS_URL])

        # Negative control: an older cache, or --max-age 0, fetches again.
        for today, extra in ((TODAY + datetime.timedelta(days=7), []), (TODAY, ["--max-age", "0"])):
            clock = FakeClock(today=today)
            site = FakeSite(clock)
            code, out, err = run(["fetch", "--cache", str(cache)] + extra, fetcher=site, clock=clock)
            self.assertEqual(code, 0, err)
            self.assertIn(poe2db.page_url(KETTLE), site.urls())


class ExtractTests(Poe2dbTestCase):
    def test_extract_synthetic_quest_page(self):
        cache = make_cache(self.tmp / "cache", GOOD_CACHE)
        draft_path = self.tmp / "draft.json"
        code, out, err = run(["extract", "--cache", str(cache), "--out", str(draft_path)])
        self.assertEqual(code, 0, err)
        draft = json.loads(draft_path.read_text("utf-8"))
        self.assertEqual(draft["schema"], 1)
        self.assertEqual(draft["source"], "https://poe2db.tw/us/Quest")
        self.assertEqual([q["slug"] for q in draft["quests"]], [KETTLE, LANTERNS])
        kettle = draft["quests"][0]
        self.assertEqual(kettle["url"], "https://poe2db.tw/us/The_Gilded_Kettle")
        self.assertEqual(kettle["fetched"], "2026-10-20")
        self.assertEqual(kettle["name"], "The Gilded Kettle")
        self.assertEqual(kettle["type"], "Normal")
        self.assertEqual(kettle["act"], 1)
        self.assertEqual(kettle["rewards"], ["Brass Token", "2 Passive Skill Points"])
        self.assertEqual([s["number"] for s in kettle["steps"]], [1, 2, 3])
        first, second, last = kettle["steps"]
        self.assertEqual(first["title"], "Search the shallows")
        self.assertEqual(first["text"],
                         "The tinker says the kettle washed downstream. Search the shallows for it.")
        self.assertEqual(first["area"], "The Copper Shallows")
        self.assertEqual(first["npcs"], [])
        self.assertEqual(second["area"], "Tinker's Rest")
        self.assertEqual(second["npcs"], ["Wilhelmina Sorrel"])
        self.assertEqual(second["text"], "You found the kettle. Bring it back to the tinker.")
        self.assertIsNone(last["area"])
        self.assertEqual(last["title"], "Quest Complete")
        for step in kettle["steps"]:
            self.assertNotIn("dialogue", step, "dialogue only with --with-dialogue")
        self.assertNotIn("kettle sings", draft_path.read_text("utf-8"))

        lanterns = draft["quests"][1]
        self.assertEqual(lanterns["type"], "Optional")
        self.assertEqual(lanterns["act"], 2)
        self.assertEqual(lanterns["acts"], [2, 3], "the index lists it under two acts")
        self.assertEqual(lanterns["rewards"], ["Hollow Charm (iLv4)"])

        code, out, err = run(["extract", "--cache", str(cache), "--out", str(draft_path), "--with-dialogue"])
        self.assertEqual(code, 0, err)
        kettle = json.loads(draft_path.read_text("utf-8"))["quests"][0]
        self.assertEqual(kettle["steps"][1]["dialogue"], [{
            "npc": "Wilhelmina Sorrel",
            "topic": "The Kettle",
            "lines": ["The kettle sings when the river is low.",
                      "Bring me the brass token and I will mend it.",
                      "Go now."],
        }])
        self.assertEqual(kettle["steps"][0]["dialogue"], [])

        # Fetch then extract, end to end through the fake site: the
        # robots-disallowed page is not drafted.
        clock = FakeClock()
        fetched = self.tmp / "fetched"
        code, out, err = run(["fetch", "--cache", str(fetched)], fetcher=FakeSite(clock), clock=clock)
        self.assertEqual(code, 0, err)
        code, out, err = run(["extract", "--cache", str(fetched), "--out", str(self.tmp / "d2.json")])
        self.assertEqual(code, 0, err)
        slugs = [q["slug"] for q in json.loads((self.tmp / "d2.json").read_text("utf-8"))["quests"]]
        self.assertEqual(slugs, [KETTLE, LANTERNS])

    def test_extract_records_a_page_without_steps_table(self):
        # Some real campaign pages carry no steps table at all (11 of 74 on
        # 2026-10-25). Such a page is a quest with zero steps, named on
        # stderr, not a failure, as long as another page still has the table.
        cache = make_cache(self.tmp / "nosteps", {"Quest": "Quest.html", LANTERNS: "Lanterns_of_Weeping_Pass.html",
                                                   KETTLE: "Broken_Structure.html"})
        out_path = self.tmp / "draft.json"
        code, out, err = run(["extract", "--cache", str(cache), "--out", str(out_path)])
        self.assertEqual(code, 0, err)
        self.assertIn(KETTLE, err)
        self.assertIn("no steps table", err)
        self.assertNotIn(LANTERNS, err, "only the page without a table is named")
        draft = json.loads(out_path.read_text("utf-8"))
        by_slug = {q["slug"]: q for q in draft["quests"]}
        self.assertEqual(sorted(by_slug), sorted([KETTLE, LANTERNS]))
        kettle = by_slug[KETTLE]
        self.assertEqual(kettle["name"], "The Gilded Kettle")
        self.assertEqual(kettle["type"], "Normal")
        self.assertEqual(kettle["act"], 1)
        self.assertEqual(kettle["steps"], [])
        self.assertEqual(draft["no_steps_table"], [KETTLE])
        self.assertTrue(by_slug[LANTERNS]["steps"], "the page with a table keeps its steps")

        # coverage counts it the same way: zero snippets, the page still listed.
        log = self.tmp / "Client.txt"
        log.write_bytes(SYNTHETIC_LOG.encode("utf-8"))
        code, out, err = run(["coverage", "--log", str(log), "--cache", str(cache)])
        self.assertEqual(code, 0, err)
        self.assertIn("%s\tsnippets=0\tequal_or_contain=0\tshared_4word_run=0" % KETTLE, out.splitlines())
        self.assertIn("TOTAL\tpages=2\t", out)
        self.assertIn(KETTLE, err)
        self.assertIn("no steps table", err)

        # Negative control: with every page well-formed, stderr names nothing.
        cache = make_cache(self.tmp / "good", GOOD_CACHE)
        code, out, err = run(["extract", "--cache", str(cache), "--out", str(out_path)])
        self.assertEqual(code, 0, err)
        self.assertNotIn("no steps table", err)
        self.assertEqual(json.loads(out_path.read_text("utf-8"))["no_steps_table"], [])

    def test_extract_fails_loudly_on_unknown_structure(self):
        # No page in the cache has a steps table: the site changed, not one quest.
        cache = make_cache(self.tmp / "broken", {"Quest": "Quest.html", LANTERNS: "Broken_Structure.html",
                                                  KETTLE: "Broken_Structure.html"})
        out_path = self.tmp / "draft.json"
        code, out, err = run(["extract", "--cache", str(cache), "--out", str(out_path)])
        self.assertNotEqual(code, 0)
        self.assertIn("no page in the cache has a steps table", err)
        self.assertFalse(out_path.exists(), "no partial draft")
        log = self.tmp / "Client.txt"
        log.write_bytes(SYNTHETIC_LOG.encode("utf-8"))
        code, out, err = run(["coverage", "--log", str(log), "--cache", str(cache)])
        self.assertNotEqual(code, 0)
        self.assertIn("no page in the cache has a steps table", err)
        self.assertNotIn("TOTAL", out)

        # A steps table whose rows do not start with a step number.
        cache = make_cache(self.tmp / "malformed", {"Quest": "Quest.html", LANTERNS: "Lanterns_of_Weeping_Pass.html",
                                                     KETTLE: "Malformed_Steps.html"})
        code, out, err = run(["extract", "--cache", str(cache), "--out", str(out_path)])
        self.assertNotEqual(code, 0)
        self.assertIn(KETTLE, err)
        self.assertIn("not a step number", err)
        self.assertFalse(out_path.exists(), "no partial draft")

        # An index page that yields no quest.
        cache = make_cache(self.tmp / "noindex", {"Quest": "Empty_Index.html", KETTLE: "The_Gilded_Kettle.html"})
        code, out, err = run(["extract", "--cache", str(cache), "--out", str(out_path)])
        self.assertNotEqual(code, 0)
        self.assertIn("Quest:", err)
        self.assertIn("yields no quest", err)
        self.assertFalse(out_path.exists())

        # fetch stops the same way rather than fetching nothing quietly.
        clock = FakeClock()
        site = FakeSite(clock, index="Empty_Index.html")
        code, out, err = run(["fetch", "--cache", str(self.tmp / "c")], fetcher=site, clock=clock)
        self.assertNotEqual(code, 0)
        self.assertIn("yields no quest", err)

        # Negative control: the well-formed pages extract.
        cache = make_cache(self.tmp / "good", GOOD_CACHE)
        code, out, err = run(["extract", "--cache", str(cache), "--out", str(out_path)])
        self.assertEqual(code, 0, err)


SYNTHETIC_LOG = "\r\n".join([
    "2026/01/02 10:00:00 ***** LOG FILE OPENING *****",
    "2026/01/02 10:00:01 1000 1a2b [INFO Client 42] Wilhelmina Sorrel: Go now.",
    "2026/01/02 10:00:02 1001 1a2b [INFO Client 42] Wilhelmina Sorrel: Bring me the brass token, traveller.",
    "2026/01/02 10:00:03 1002 1a2b [INFO Client 42] Wilhelmina Sorrel: Bring me the brass token, traveller.",
    # Old Corvin's line, but said by someone else: a different NPC never counts.
    "2026/01/02 10:00:04 1003 1a2b [INFO Client 42] Wilhelmina Sorrel: Lanterns burn for the ones who walked the pass and never came down.",
    # Local chat and a whisper shaped like NPC lines: dropped.
    "2026/01/02 10:00:05 1004 1a2b [INFO Client 42] #Old Corvin: Lanterns burn for the ones who walked the pass and never came down.",
    "2026/01/02 10:00:06 1005 1a2b [INFO Client 42] @From Quillon Ashby: the kettle sings when the river is low",
    "2026/01/02 10:00:07 1006 1a2b [DEBUG Client 42] Generating level 3 area \"X_Shallows\" with seed 7",
    # A continuation line without a timestamp: ignored.
    "Old Corvin: Lanterns burn for the ones who walked the pass and never came down.",
    "",
])


class CoverageTests(Poe2dbTestCase):
    def test_coverage_prints_counts_only(self):
        cache = make_cache(self.tmp / "cache", GOOD_CACHE)
        log = self.tmp / "Client.txt"
        log.write_bytes(SYNTHETIC_LOG.encode("utf-8"))
        code, out, err = run(["coverage", "--log", str(log), "--cache", str(cache)])
        self.assertEqual(code, 0, err)
        lines = out.splitlines()
        self.assertEqual(lines, [
            "%s\tsnippets=1\tequal_or_contain=0\tshared_4word_run=0" % LANTERNS,
            "%s\tsnippets=3\tequal_or_contain=1\tshared_4word_run=1" % KETTLE,
            "TOTAL\tpages=2\tsnippets=4\tequal_or_contain=1\tshared_4word_run=1\tnpcs_with_logged_lines=1",
        ])
        lowered = out.lower()
        for text in ("kettle sings", "brass token", "go now", "lanterns burn", "never came down",
                     "traveller", "quillon", "wilhelmina", "corvin", "the kettle"):
            self.assertNotIn(text, lowered)

        # Baseline: a log where no NPC of the pages speaks counts nothing.
        quiet = self.tmp / "Quiet.txt"
        quiet.write_text("2026/01/02 10:00:01 1000 1a2b [INFO Client 42] Somebody Else: Go now.\n", encoding="utf-8")
        code, out, err = run(["coverage", "--log", str(quiet), "--cache", str(cache)])
        self.assertEqual(code, 0, err)
        self.assertIn("TOTAL\tpages=2\tsnippets=4\tequal_or_contain=0\tshared_4word_run=0", out)


CLASS_PAGES = {"Warrior": "Warrior.html", "Broken_Class": "Broken_Class.html"}

# The quest data the remark draft reads: bosses come from `bar_gone` signals
# and `npcs`. Viper Napuatzi and the Bog Witch are there so a key that shares
# only a later word of their names (`VillageViperDefeated`, `WitchBossKilled`)
# is shown not to match.
REMARK_DATA = {
    "npcs": ["The Bloated Miller", "BlackJaw", "Viper Napuatzi", "Ignagduk, the Bog Witch"],
    "quests": [
        {"id": "a", "steps": [{"id": "fight", "signals": [
            {"kind": "screen", "what": "bar_gone", "boss": "Blackjaw", "area": "G3_6_1"}]}]},
        {"id": "b", "steps": [{"id": "fight", "signals": [
            {"kind": "screen", "what": "bar_gone", "boss": "Beira of the Rotten Pack", "area": "G1_2"}]}]},
    ],
}


def make_class_cache(folder, pages, fetched="2026-10-20"):
    """A cache as fetch-classes leaves it: `pages` maps a class to its fixture."""
    folder.mkdir(parents=True, exist_ok=True)
    index = {"schema": 1, "source": poe2db.page_url("Quest"), "pages": {}, "classes": {}}
    for name, fixture in pages.items():
        file_name = "class-" + poe2db.cache_file_name(name)
        (folder / file_name).write_bytes((FIXTURES / fixture).read_bytes())
        index["classes"][name] = {"slug": name, "url": poe2db.page_url(name), "fetched": fetched,
                                  "status": 200, "file": file_name}
    poe2db.save_cache_index(folder, index)
    return folder


class ClassFetchTests(Poe2dbTestCase):
    def site(self, clock):
        return FakeSite(clock, pages=CLASS_PAGES)

    def test_fetch_classes_reads_robots_first_and_keeps_classes_apart(self):
        clock = FakeClock()
        site = self.site(clock)
        cache = self.tmp / "cache"
        code, out, err = run(["fetch-classes", "Warrior", "Broken_Class", "--cache", str(cache)],
                             fetcher=site, clock=clock)
        self.assertEqual(code, 0, err)
        self.assertEqual(site.urls(), [poe2db.ROBOTS_URL, poe2db.page_url("Warrior"),
                                       poe2db.page_url("Broken_Class")],
                         "robots.txt first, then the class pages; no quest index")
        times = [t for t, _, _ in site.calls]
        self.assertGreaterEqual(min(b - a for a, b in zip(times, times[1:])), poe2db.DEFAULT_DELAY - 1e-9)
        for _, _, user_agent in site.calls:
            self.assertEqual(user_agent, poe2db.USER_AGENT)
        index = json.loads((cache / "index.json").read_text("utf-8"))
        self.assertEqual(index["classes"]["Warrior"],
                         {"slug": "Warrior", "url": poe2db.page_url("Warrior"), "fetched": "2026-10-20",
                          "status": 200, "file": "class-Warrior.html"})
        self.assertTrue((cache / "class-Warrior.html").is_file())
        # A class page is never a quest page: extract and coverage read `pages` only.
        self.assertEqual(index["pages"], {})

    def test_fetch_classes_honours_robots_cache_and_git(self):
        clock = FakeClock()
        site = self.site(clock)
        cache = self.tmp / "cache"
        code, out, err = run(["fetch-classes", "Sealed_Class", "Warrior", "--cache", str(cache)],
                             fetcher=site, clock=clock)
        self.assertEqual(code, 0, err)
        self.assertNotIn(poe2db.page_url("Sealed_Class"), site.urls())
        self.assertIn("skip  Sealed_Class: disallowed by robots.txt", out)
        self.assertIn(poe2db.page_url("Warrior"), site.urls(), "negative control: an allowed page is fetched")

        # Within --max-age nothing is asked, not even robots.txt.
        clock = FakeClock(today=TODAY + datetime.timedelta(days=6))
        site = self.site(clock)
        code, out, err = run(["fetch-classes", "Warrior", "--cache", str(cache)], fetcher=site, clock=clock)
        self.assertEqual(code, 0, err)
        self.assertEqual(site.calls, [])
        clock = FakeClock(today=TODAY + datetime.timedelta(days=7))
        site = self.site(clock)
        code, out, err = run(["fetch-classes", "Warrior", "--cache", str(cache)], fetcher=site, clock=clock)
        self.assertEqual(code, 0, err)
        self.assertEqual(site.urls(), [poe2db.ROBOTS_URL, poe2db.page_url("Warrior")])

        repo = self.tmp / "repo"
        (repo / ".git").mkdir(parents=True)
        clock = FakeClock()
        site = self.site(clock)
        code, out, err = run(["fetch-classes", "Warrior", "--cache", str(repo / "cache")], fetcher=site, clock=clock)
        self.assertEqual(code, 1)
        self.assertIn("git checkout", err)
        self.assertEqual(site.calls, [])
        for argv in (["fetch-classes", "Warrior", "--delay", "1.5"], ["fetch-classes", "../Warrior"]):
            clock = FakeClock()
            site = self.site(clock)
            code, out, err = run(argv + ["--cache", str(self.tmp / "other")], fetcher=site, clock=clock)
            self.assertNotEqual(code, 0, argv)
            self.assertEqual(site.calls, [], argv)


class RemarkTests(Poe2dbTestCase):
    def data_file(self):
        path = self.tmp / "quests.json"
        path.write_text(json.dumps(REMARK_DATA), encoding="utf-8")
        return path

    def test_remarks_drafts_the_kill_rows_of_bosses_the_data_names(self):
        cache = make_class_cache(self.tmp / "cache", {"Warrior": "Warrior.html"})
        code, out, err = run(["remarks", "--cache", str(cache), "--data", str(self.data_file())])
        self.assertEqual(code, 0, err)
        draft = json.loads(out)
        url = poe2db.page_url("Warrior")
        self.assertEqual(draft["remarks"], [
            {"class": "Warrior", "boss": "The Bloated Miller", "key": "MillerKillComment",
             "text": "Pebbles and porridge, the mill is quiet now.", "source_url": url,
             "fetched": "2026-10-20", "audio": "warrior_gameplay_a1_06.ogg"},
            {"class": "Warrior", "boss": "Blackjaw", "key": "BlackjawKilled",
             "text": "The gears stop turning at last.", "source_url": url,
             "fetched": "2026-10-20", "audio": "warrior_gameplay_a3_11.ogg"},
            {"class": "Warrior", "boss": "Blackjaw", "key": "RavineBlackjawBossDefeated",
             "text": "Down in the gully, down you stay.", "source_url": url,
             "fetched": "2026-10-20", "audio": "warrior_gameplay_i3_19.ogg"},
        ])
        # Never the whole table: a sighting, a rare kill, a league line, and the
        # keys that share only a later word of a boss's name stay out.
        for text in ("A very large fellow", "Crumbled like stale bread", "Not the general",
                     "serpent of the village", "marsh witch", "Teeth and more teeth",
                     "invented flavour line", "Skill gems"):
            self.assertNotIn(text, out)

        # Baseline: data that names none of those bosses drafts no row.
        empty = self.tmp / "none.json"
        empty.write_text(json.dumps({"npcs": ["Una"], "quests": []}), encoding="utf-8")
        code, out, err = run(["remarks", "--cache", str(cache), "--data", str(empty)])
        self.assertEqual(code, 0, err)
        self.assertEqual(json.loads(out)["remarks"], [])

    def test_remarks_fails_loudly_on_unknown_structure(self):
        cache = make_class_cache(self.tmp / "cache", CLASS_PAGES)
        code, out, err = run(["remarks", "--cache", str(cache), "--data", str(self.data_file())])
        self.assertEqual(code, 1)
        self.assertIn("Broken_Class: no Text Audio table", err)
        self.assertEqual(out, "")
        # Negative control: the same cache, asked only for the well-formed page.
        code, out, err = run(["remarks", "--cache", str(cache), "--data", str(self.data_file()),
                              "--class", "Warrior"])
        self.assertEqual(code, 0, err)
        self.assertEqual(len(json.loads(out)["remarks"]), 3)

        # A class that is not cached, or no class page at all, says so.
        code, out, err = run(["remarks", "--cache", str(cache), "--data", str(self.data_file()),
                              "--class", "Druid"])
        self.assertEqual(code, 1)
        self.assertIn("Druid", err)
        bare = make_cache(self.tmp / "bare", GOOD_CACHE)
        code, out, err = run(["remarks", "--cache", str(bare), "--data", str(self.data_file())])
        self.assertEqual(code, 1)
        self.assertIn("fetch-classes", err)


class HelpTests(unittest.TestCase):
    def test_help_names_the_commands(self):
        out = io.StringIO()
        with contextlib.redirect_stdout(out), self.assertRaises(SystemExit) as raised:
            poe2db.main(["--help"])
        self.assertEqual(raised.exception.code, 0)
        for command in ("fetch", "fetch-classes", "extract", "remarks", "coverage"):
            self.assertIn(command, out.getvalue())


if __name__ == "__main__":
    unittest.main()
