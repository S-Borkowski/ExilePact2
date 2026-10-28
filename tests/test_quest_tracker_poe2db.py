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


# The remark ledger reads all eight playable classes. Warrior and Ranger hold
# the invented campaign lines; the other six are a page with one wild line.
LEDGER_PAGES = {name: "Quiet_Class.html" for name in poe2db.CLASSES}
LEDGER_PAGES.update({"Warrior": "Warrior.html", "Ranger": "Ranger.html"})
W_URL, R_URL = poe2db.page_url("Warrior"), poe2db.page_url("Ranger")
MILLER = "The Bloated Miller"


def attach(quest, step, area):
    return {"quest": quest, "step": step, "area": area}


# One entry per campaign key of the fixtures, by part. Every `why` is our own.
LEDGER = {
    "act1": [
        {"key": "MillerKillComment", "attach": [attach("mill", "step-2", "G1_1")], "boss": MILLER,
         "why": "The Miller's death ends the fight step."},
        {"key": "MillerSeenComment", "skip": "no-step", "why": "A sighting of a boss whose step ends on the kill."},
        {"key": "WitchBossKilled", "skip": "not-established", "why": "No step of the data names this boss."},
        {"key": "GroveShrineSeenComment", "attach": [attach("grove", "step-1", "G1_4")],
         "why": "Finding the shrine ends the search."},
        {"key": "GroveLanternLit", "skip": "too-short", "why": "Only a placeholder line."},
        {"key": "GroveGateOpened", "attach": [attach("grove", "step-2", "G1_7")],
         "why": "Opening the gate ends the step."},
        {"key": "MarshGateOpened", "skip": "no-step", "why": "A bark at a side gate."},
        {"key": "GroveGateSwung", "attach": [attach("grove", "step-2", "G1_7")],
         "why": "The same gate under another key."},
    ],
    "act2": [{"key": "HyenaPackCharges", "skip": "no-step", "why": "A bark in a fight."}],
    "act3": [{"key": "BlackjawKilled", "skip": "not-established", "why": "No act-3 step in this data."},
             {"key": "RavineBlackjawBossDefeated", "skip": "not-established", "why": "No act-3 step in this data."}],
    "act4": [],
    "interludes": [{"key": "VillageViperDefeated", "skip": "no-step", "why": "No interlude quest in this data."}],
}


def ledger_row(cls, key, text, boss=None):
    row = {"class": cls}
    if boss:
        row["boss"] = boss
    row.update({"key": key, "text": text, "source_url": W_URL if cls == "Warrior" else R_URL,
                "fetched": "2026-10-20"})
    return row


# The act-1 draft: parts in order, entries in file order, classes in the
# ledger's order. "Ah, there." is short; "Onward, then, to whatever waits." is
# also MarshGateOpened's (skipped) text; "The gate gives way at last" is kept
# because GroveGateSwung attaches to the same step; "The mill wheel turns no
# more, friend." is kept although Str4b_OldLine says it too, since that key has
# no sound file on any page and so is never voiced.
DRAFT = [
    ledger_row("Warrior", "MillerKillComment", "Pebbles and porridge, the mill is quiet now.", MILLER),
    ledger_row("Ranger", "MillerKillComment", "The mill wheel turns no more, friend.", MILLER),
    ledger_row("Ranger", "GroveShrineSeenComment", "What a strange shrine stands in this grove."),
    ledger_row("Ranger", "GroveGateOpened", "The gate gives way at last."),
    ledger_row("Ranger", "GroveGateSwung", "The gate gives way at last"),
]


def ledger_data(remarks=None):
    return {
        "schema": 1,
        "npcs": [MILLER],
        "areas": {"G1_1": {"name": "The Riverbank", "act": 1}, "G1_4": {"name": "The Grelwood", "act": 1},
                  "G1_7": {"name": "The Marsh", "act": 1}, "G2_1": {"name": "The Dunes", "act": 2}},
        "quests": [
            {"id": "mill", "act": 1, "zones": ["G1_1"], "steps": [
                {"id": "step-1", "title": "Find the mill", "text": "Look for the Miller by the river.",
                 "signals": [{"kind": "area", "code": "G1_1"}]},
                {"id": "step-2", "title": "Slay the Bloated Miller", "text": "Kill it.", "signals": [
                    {"kind": "screen", "what": "bar_gone", "boss": MILLER, "area": "G1_1"},
                    {"kind": "screen", "what": "kill_remark", "boss": MILLER, "area": "G1_1"}]}]},
            {"id": "grove", "act": 1, "zones": ["G1_4", "G1_7"],
             "notes": {"remarks": ["a nested key the rewrite never touches"]}, "steps": [
                {"id": "step-1", "title": "Search the Grelwood", "text": "Find the shrine in the grove.",
                 "signals": [{"kind": "screen", "what": "remark", "key": "GroveShrineSeenComment", "area": "G1_4"}]},
                {"id": "step-2", "title": "Open the gate", "text": "Open the marsh gate.", "signals": [
                    {"kind": "area", "code": "G1_7"},
                    {"kind": "screen", "what": "remark", "key": "GroveGateOpened", "area": "G1_7"},
                    {"kind": "screen", "what": "remark", "key": "GroveGateSwung", "area": "G1_7"}]}]},
            {"id": "dunes", "act": 2, "zones": ["G2_1"], "steps": [
                {"id": "step-1", "title": "Cross the dunes", "text": "Hyenas roam the dunes.",
                 "signals": [{"kind": "area", "code": "G2_1"}]}]},
        ],
        "remarks": list(DRAFT) if remarks is None else remarks,
    }


def data_text(data, eol="\r\n"):
    """The data as the app's file lays it out: two-space indent, one remark row
    per line, `remarks` before a last key, in `eol` line endings."""
    head = json.dumps({k: v for k, v in data.items() if k != "remarks"}, indent=2)
    rows = ",\n".join("    " + json.dumps(row) for row in data["remarks"])
    text = head[:-2] + ',\n  "remarks": [\n' + rows + '\n  ],\n  "tail": "kept"\n}\n'
    return text.replace("\n", eol)


class RemarkLedgerTests(Poe2dbTestCase):
    def setUp(self):
        super().setUp()
        self.cache = make_class_cache(self.tmp / "cache", LEDGER_PAGES)

    def write_ledger(self, entries_by_part, folder=None):
        folder = folder or self.tmp / "ledger"
        folder.mkdir(parents=True, exist_ok=True)
        for part, entries in entries_by_part.items():
            lines = ",\n".join("    " + json.dumps(entry) for entry in entries)
            text = '{"schema": 1, "part": "%s", "keys": [\n%s\n]}\n' % (part, lines) if entries else \
                '{"schema": 1, "part": "%s", "keys": []}\n' % part
            (folder / (part + ".json")).write_text(text, encoding="utf-8", newline="\n")
        return folder

    def write_data(self, data=None, name="quests.json"):
        path = self.tmp / name
        path.write_bytes(data_text(data or ledger_data()).encode("utf-8"))
        return path

    def check(self, ledger, data, *extra):
        return run(["remarks", "--cache", str(self.cache), "--data", str(data), "--ledger", str(ledger),
                    "--check"] + list(extra))

    def test_remarks_keys_lists_every_campaign_key_of_a_part(self):
        data = self.write_data()
        code, out, err = run(["remarks", "--keys", "--part", "act1", "--cache", str(self.cache),
                              "--data", str(data)])
        self.assertEqual(code, 0, err)
        lines = [json.loads(line) for line in out.splitlines()]
        self.assertEqual([line["key"] for line in lines], [
            "MillerKillComment", "MillerSeenComment", "WitchBossKilled", "GroveShrineSeenComment",
            "GroveLanternLit", "GroveGateOpened", "MarshGateOpened", "GroveGateSwung"],
            "every act-1 key, Warrior's page first, each once")
        by_key = {line["key"]: line for line in lines}
        shrine = by_key["GroveShrineSeenComment"]
        self.assertEqual(shrine["tags"], ["a1"])
        self.assertEqual(list(shrine["texts"]), list(poe2db.CLASSES))
        self.assertEqual(shrine["texts"]["Ranger"], ["What a strange shrine stands in this grove."],
                         "braces gone, spaces collapsed, the short row dropped")
        self.assertEqual(shrine["texts"]["Warrior"], [])
        self.assertEqual(shrine["candidates"], ["grove/step-1"])
        self.assertEqual(by_key["MillerKillComment"]["candidates"], ["mill/step-2", "mill/step-1"],
                         "the step sharing most words first")
        self.assertEqual(by_key["GroveLanternLit"]["texts"]["Ranger"], [], "a [DNT] row is no text")
        self.assertEqual(by_key["MillerSeenComment"]["texts"]["Ranger"], [])
        self.assertEqual(by_key["GroveGateOpened"]["shared"], ["GroveGateSwung", "MarshGateOpened"])
        self.assertEqual(by_key["MillerKillComment"]["shared"], [],
                         "Str4b_OldLine shares its text but has no sound file on any page")

        # A key heard in two parts belongs to the first; no-sound and wild rows are no campaign key.
        code, out, err = run(["remarks", "--keys", "--part", "act3", "--cache", str(self.cache),
                              "--data", str(data)])
        self.assertEqual(code, 0, err)
        lines = [json.loads(line) for line in out.splitlines()]
        self.assertEqual([line["key"] for line in lines], ["BlackjawKilled", "RavineBlackjawBossDefeated"])
        self.assertEqual(lines[1]["tags"], ["a3", "i3"])
        code, out, err = run(["remarks", "--keys", "--part", "interludes", "--cache", str(self.cache),
                              "--data", str(data)])
        self.assertEqual([json.loads(line)["key"] for line in out.splitlines()], ["VillageViperDefeated"])
        for key in ("KillRare", "Str4b_OldLine", "IncursionDefeatBlackjawUnique"):
            self.assertNotIn(key, out)

        # All eight classes, or it says which are missing; and --keys needs a part.
        partial = make_class_cache(self.tmp / "partial", {"Warrior": "Warrior.html"})
        code, out, err = run(["remarks", "--keys", "--part", "act1", "--cache", str(partial),
                              "--data", str(data)])
        self.assertEqual(code, 1)
        self.assertIn("Mercenary, Ranger, Huntress, Witch, Sorceress, Monk, Druid", err)
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit) as raised:
            poe2db.main(["remarks", "--keys", "--cache", str(self.cache)])
        self.assertEqual(raised.exception.code, 2)

    def test_remarks_ledger_check_finds_unaccounted_keys(self):
        data = self.write_data()
        ledger = self.write_ledger(LEDGER)
        code, out, err = self.check(ledger, data, "--ledger-only")
        self.assertEqual(code, 0, out + err)
        self.assertEqual(out.splitlines()[-1],
                         "summary: parts=act1,act2,act3,act4,interludes keys=12 entries=12 attached=4 "
                         "attachments=4 log-covers=0 no-step=4 no-area=0 not-established=3 too-short=1 "
                         "rows=5 dropped=2 dropped-short=1 dropped-shared=1 gaps=27 findings=0")
        self.assertNotIn("finding:", out)
        self.assertIn("gap: act1 GroveShrineSeenComment Warrior: no usable text", out)

        # --part reads only its own file: the others may not exist yet.
        alone = self.write_ledger({"act1": LEDGER["act1"]}, self.tmp / "alone")
        code, out, err = self.check(alone, data, "--ledger-only", "--part", "act1")
        self.assertEqual(code, 0, out + err)
        code, out, err = self.check(alone, data, "--ledger-only")
        self.assertEqual(code, 1)
        self.assertIn("finding: act2: no ledger file", out)

        bad = [e for e in LEDGER["act1"] if e["key"] != "GroveGateSwung"] + [
            {"key": "NoSuchKey", "skip": "no-step", "why": "Invented."},
            {"key": "HyenaPackCharges", "skip": "no-step", "why": "Filed under the wrong act."},
            {"key": "KillRare", "skip": "no-step", "why": "A wild line."},
            {"key": "MillerSeenComment", "skip": "no-step", "why": "Twice."},
            {"key": "WitchBossKilled", "skip": "maybe", "why": "An unknown code."},
            {"key": "GroveLanternLit", "skip": "too-short", "attach": [], "why": "Both at once."},
        ]
        self.write_ledger({"act1": bad}, self.tmp / "bad")
        code, out, err = self.check(self.tmp / "bad", data, "--ledger-only", "--part", "act1")
        self.assertEqual(code, 1)
        findings = [line for line in out.splitlines() if line.startswith("finding:")]
        for expected in ("act1 GroveGateSwung: no entry",
                         "act1 NoSuchKey: not on any class page",
                         "act1 HyenaPackCharges: belongs to act2",
                         "act1 KillRare: no campaign sound",
                         "act1 MillerSeenComment: in more than one entry",
                         "act1 WitchBossKilled: unknown skip code",
                         "act1 GroveLanternLit: an entry is either attached or skipped"):
            self.assertTrue(any(expected in f for f in findings), (expected, findings))
        self.assertTrue(out.splitlines()[-1].startswith("summary: parts=act1 "))
        self.assertTrue(out.splitlines()[-1].endswith("findings=%d" % len(findings)))

    def test_remarks_ledger_draft_drops_shared_and_short_texts(self):
        data = self.write_data(ledger_data(remarks=[]))
        ledger = self.write_ledger(LEDGER)
        code, out, err = run(["remarks", "--cache", str(self.cache), "--data", str(data), "--ledger", str(ledger),
                              "--apply"])
        self.assertEqual(code, 0, out + err)
        self.assertEqual(json.loads(data.read_text("utf-8"))["remarks"], DRAFT,
                         "Ranger's MillerKillComment row stays: Str4b_OldLine, sharing it, is never voiced")

        # Negative control: once MarshGateOpened attaches to the same step,
        # their shared text is no longer told apart from nothing, and is kept.
        same = [dict(e, attach=[attach("grove", "step-2", "G1_7")], why="The marsh side of the gate.")
                if e["key"] == "MarshGateOpened" else e for e in LEDGER["act1"]]
        for e in same:
            if e["key"] == "MarshGateOpened":
                del e["skip"]
        self.write_ledger(dict(LEDGER, act1=same), self.tmp / "same")
        code, out, err = run(["remarks", "--cache", str(self.cache), "--data", str(data), "--ledger",
                              str(self.tmp / "same"), "--apply"])
        self.assertEqual(code, 0, out + err)
        rows = json.loads(data.read_text("utf-8"))["remarks"]
        self.assertIn(ledger_row("Ranger", "GroveGateOpened", "Onward, then, to whatever waits."), rows)
        self.assertIn(ledger_row("Ranger", "MarshGateOpened", "Onward, then, to whatever waits!"), rows)
        self.assertNotIn("Ah, there.", data.read_text("utf-8"))

        # A key all of whose texts are dropped cannot be attached (rule 4), and
        # a too-short skip must be true.
        wrong = [dict(e, why="Swapped.") for e in LEDGER["act1"]]
        for e in wrong:
            if e["key"] == "GroveLanternLit":
                e.pop("skip")
                e["attach"] = [attach("grove", "step-1", "G1_4")]
            elif e["key"] == "GroveShrineSeenComment":
                e.pop("attach")
                e["skip"] = "too-short"
        self.write_ledger({"act1": wrong}, self.tmp / "wrong")
        code, out, err = self.check(self.tmp / "wrong", data, "--ledger-only", "--part", "act1")
        self.assertEqual(code, 1)
        self.assertIn("finding: act1 GroveLanternLit: no class has a usable text", out)
        self.assertIn("finding: act1 GroveShrineSeenComment: skipped as too-short, but Ranger has a usable text",
                      out)

    def test_remarks_ledger_check_matches_data_signals_and_rows(self):
        ledger = self.write_ledger(LEDGER)
        data = self.write_data()
        code, out, err = self.check(ledger, data)
        self.assertEqual(code, 0, out + err)
        code, out, err = self.check(ledger, data, "--part", "act1")
        self.assertEqual(code, 0, out + err)

        # A missing signal, a signal no entry attaches, a stale row, rows out of order.
        broken = ledger_data(remarks=[DRAFT[1], DRAFT[0]] + DRAFT[2:4] +
                             [ledger_row("Ranger", "GroveGateSwung", "An invented stale line.")])
        steps = broken["quests"][1]["steps"]
        steps[1]["signals"] = steps[1]["signals"][:2]
        broken["quests"][2]["steps"][0]["signals"].append(
            {"kind": "screen", "what": "remark", "key": "HyenaPackCharges", "area": "G2_1"})
        code, out, err = self.check(ledger, self.write_data(broken, "broken.json"))
        self.assertEqual(code, 1)
        for expected in ("finding: data: signal missing: grove/step-2 remark GroveGateSwung G1_7",
                         "finding: data: signal not in the ledger: dunes/step-1 remark HyenaPackCharges G2_1",
                         "finding: data: row missing: Ranger GroveGateSwung",
                         "finding: data: row not in the draft: Ranger GroveGateSwung"):
            self.assertIn(expected, out)
        reordered = ledger_data(remarks=[DRAFT[1], DRAFT[0]] + DRAFT[2:])
        code, out, err = self.check(ledger, self.write_data(reordered, "reordered.json"))
        self.assertEqual(code, 1)
        self.assertIn("finding: data: the remarks rows are not in the draft's order", out)
        # --ledger-only leaves the data's rows and signals alone.
        code, out, err = self.check(ledger, self.write_data(broken, "broken.json"), "--ledger-only")
        self.assertEqual(code, 0, out + err)

        # Rules 2 and 3: the quest's act is the sound's, the area is the step's or named by it.
        moved = [dict(e) for e in LEDGER["act1"]]
        for e in moved:
            if e["key"] == "GroveShrineSeenComment":
                e["attach"] = [attach("dunes", "step-1", "G2_1")]
            elif e["key"] == "GroveGateOpened":
                e["attach"] = [attach("grove", "step-2", "G1_1")]
            elif e["key"] == "GroveGateSwung":
                e["attach"] = [attach("grove", "step-9", "G1_7")]
        self.write_ledger({"act1": moved}, self.tmp / "moved")
        code, out, err = self.check(self.tmp / "moved", data, "--ledger-only", "--part", "act1")
        self.assertEqual(code, 1)
        for expected in ("finding: act1 GroveShrineSeenComment: dunes is act 2, not the sound's act (rule 2)",
                         "finding: act1 GroveGateOpened: area G1_1 is not grove/step-2's (rule 3)",
                         "finding: act1 GroveGateSwung: no step grove/step-9"):
            self.assertIn(expected, out)

    def test_remarks_ledger_apply_rewrites_only_the_remarks_block(self):
        ledger = self.write_ledger(LEDGER)
        stale = [DRAFT[2], DRAFT[0], ledger_row("Ranger", "GroveGateSwung", "An invented stale line.")]
        data = self.write_data(ledger_data(remarks=stale))
        before = data.read_bytes()
        code, out, err = run(["remarks", "--cache", str(self.cache), "--data", str(data), "--ledger", str(ledger),
                              "--apply"])
        self.assertEqual(code, 0, out + err)
        after = data.read_bytes()
        self.assertEqual(after, data_text(ledger_data()).encode("utf-8"),
                         "the draft's rows, one per line, in the file's CRLF; nothing else moved")
        head = before.index(b'\r\n  "remarks": [')
        self.assertEqual(after[:head], before[:head])
        self.assertTrue(after.endswith(b'\r\n  ],\r\n  "tail": "kept"\r\n}\r\n'))
        self.assertNotIn(b"\n", after.replace(b"\r\n", b""))
        self.assertIn(b'"a nested key the rewrite never touches"', after)
        self.assertIn("unchanged", run(["remarks", "--cache", str(self.cache), "--data", str(data),
                                        "--ledger", str(ledger), "--apply"])[1])
        self.assertEqual(data.read_bytes(), after, "a second run changes nothing")

        # An LF file stays LF.
        lf = self.tmp / "lf.json"
        lf.write_bytes(data_text(ledger_data(remarks=stale), eol="\n").encode("utf-8"))
        code, out, err = run(["remarks", "--cache", str(self.cache), "--data", str(lf), "--ledger", str(ledger),
                              "--apply"])
        self.assertEqual(code, 0, out + err)
        self.assertEqual(lf.read_bytes(), data_text(ledger_data(), eol="\n").encode("utf-8"))

        # A ledger that fails its own check writes nothing; --apply takes every part.
        self.write_ledger({"act1": LEDGER["act1"][1:]}, ledger)
        data.write_bytes(before)
        code, out, err = run(["remarks", "--cache", str(self.cache), "--data", str(data), "--ledger", str(ledger),
                              "--apply"])
        self.assertEqual(code, 1)
        self.assertIn("finding: act1 MillerKillComment: no entry", out)
        self.assertEqual(data.read_bytes(), before)
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit) as raised:
            poe2db.main(["remarks", "--ledger", str(ledger), "--apply", "--part", "act1"])
        self.assertEqual(raised.exception.code, 2)


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
