"""tools/quest_tracker/outside: common helpers and the session commands.

Nothing here starts a trace, opens a device or a window, or reads a real game
file. The `Client.txt` and `Content.ggpk` that `prep` looks at are small
synthetic files in temporary folders outside the repository, and the clock
anchors come from an injected sampler.

The baseline is a writer that refuses the repository: an output path inside a
git checkout is refused, with a folder outside any checkout accepted beside it
as the negative control. The targets are the shared formats (UTC timestamps,
JSON lines), the log counter's conversion to UTC from anchors, including its
fall back to the wall clock and the 32-bit wrap, and the live folder that
`prep`, `clock` and `stop` lay out.
"""
import contextlib
import datetime
import io
import json
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from tools.quest_tracker.outside import common, session  # noqa: E402

UTC = datetime.timezone.utc
CET = datetime.timezone(datetime.timedelta(hours=1))
WRAP = 1 << 32


def utc(*args) -> datetime.datetime:
    return datetime.datetime(*args, tzinfo=UTC)


class TempDir(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = Path(self._tmp.name)
        self.assertIsNone(common.git_tree_of(self.tmp), "the temp folder sits inside a git checkout")

    def tearDown(self):
        self._tmp.cleanup()


class RefuseInsideGit(TempDir):
    def make_checkout(self) -> Path:
        tree = self.tmp / "checkout"
        (tree / ".git").mkdir(parents=True)
        return tree

    def test_output_inside_a_git_checkout_is_refused(self):
        tree = self.make_checkout()
        with self.assertRaises(common.ToolError) as ctx:
            common.refuse_inside_git(tree / "deep" / "live", "live folder")
        self.assertIn(str(tree), str(ctx.exception))

    def test_this_repository_is_refused(self):
        with self.assertRaises(common.ToolError):
            common.refuse_inside_git(ROOT / "nowhere" / "live", "live folder")

    def test_a_folder_outside_any_checkout_is_accepted(self):
        self.make_checkout()
        common.refuse_inside_git(self.tmp / "elsewhere" / "live", "live folder")

    def test_every_writer_refuses_the_checkout(self):
        tree = self.make_checkout()
        live = tree / "live"
        with self.assertRaises(common.ToolError):
            common.append_jsonl(tree / "x.jsonl", {"a": 1})
        with self.assertRaises(common.ToolError):
            common.write_json(tree / "x.json", {"a": 1})
        with self.assertRaises(common.ToolError):
            session.prep(live, self.tmp / "Client.txt", self.tmp / "Content.ggpk")
        live.mkdir()
        with self.assertRaises(common.ToolError):
            session.clock(live, sampler=lambda: {"utc": "2026-10-26T10:00:00.000Z", "tick_ms": 1})
        with self.assertRaises(common.ToolError):
            session.stop(live)
        self.assertEqual(list(live.iterdir()), [])
        self.assertFalse((tree / "x.jsonl").exists() or (tree / "x.json").exists())

    def test_default_live_folder(self):
        live = common.default_live_dir()
        self.assertEqual(live.parts[-2:], ("quest-tracker-live-5", "beira-defeat"))


class UtcTimestamps(unittest.TestCase):
    def test_round_trip(self):
        for when in (utc(2026, 10, 26, 10, 28, 17, 123000), utc(2026, 1, 1, 0, 0, 0),
                     utc(2026, 12, 31, 23, 59, 59, 999000)):
            text = common.format_utc(when)
            self.assertRegex(text, r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$")
            self.assertEqual(common.parse_utc(text), when)
            self.assertEqual(common.format_utc(common.parse_utc(text)), text)

    def test_format(self):
        self.assertEqual(common.format_utc(utc(2026, 10, 26, 10, 28, 17, 5000)),
                         "2026-10-26T10:28:17.005Z")
        local = datetime.datetime(2026, 10, 26, 11, 28, 17, 250000, tzinfo=CET)
        self.assertEqual(common.format_utc(local), "2026-10-26T10:28:17.250Z")

    def test_naive_is_refused(self):
        with self.assertRaises(ValueError):
            common.format_utc(datetime.datetime(2026, 10, 26, 10, 28, 17))

    def test_parse_variants_and_refusals(self):
        self.assertEqual(common.parse_utc("2026-10-26T10:28:17Z"), utc(2026, 10, 26, 10, 28, 17))
        self.assertEqual(common.parse_utc("2026-10-26T10:28:17.5Z"),
                         utc(2026, 10, 26, 10, 28, 17, 500000))
        for bad in ("2026-10-26T10:28:17.123", "2026-10-26 10:28:17.123Z",
                    "2026-10-26T10:28:17.123+01:00", "yesterday"):
            with self.assertRaises(ValueError, msg=bad):
                common.parse_utc(bad)


class JsonLines(TempDir):
    def test_round_trip(self):
        path = self.tmp / "x.jsonl"
        records = [{"utc": "2026-10-26T10:28:17.123Z", "n": 1}, {"text": "Swell!", "n": 2}]
        for r in records:
            common.append_jsonl(path, r)
        self.assertEqual(common.read_jsonl(path), records)
        self.assertEqual(path.read_bytes().count(b"\n"), 2)
        self.assertNotIn(b"\r", path.read_bytes())

    def test_a_partial_last_line_is_dropped(self):
        path = self.tmp / "x.jsonl"
        path.write_text('{"n": 1}\n{"n": 2}\n{"n": 3, "ut', encoding="utf-8")
        self.assertEqual(common.read_jsonl(path), [{"n": 1}, {"n": 2}])

    def test_a_bad_middle_line_is_an_error(self):
        path = self.tmp / "x.jsonl"
        path.write_text('{"n": 1}\nnot json\n{"n": 3}\n', encoding="utf-8")
        with self.assertRaises(common.ToolError) as ctx:
            common.read_jsonl(path)
        self.assertIn(":2", str(ctx.exception))


class CounterToUtc(unittest.TestCase):
    """A line's local second is `wall`; the anchor ties tick counts to UTC."""

    A = common.Anchor(utc(2026, 10, 26, 10, 0, 0), 90_000_000)

    def line(self, seconds_after_anchor: float, counter_offset_ms: int = 0):
        """The wall second and counter of a line `seconds_after_anchor` after A."""
        true = self.A.utc + datetime.timedelta(seconds=seconds_after_anchor)
        wall_utc = true.replace(microsecond=0)
        wall_local = wall_utc.astimezone(CET).replace(tzinfo=None)
        counter = (self.A.tick_ms + round(seconds_after_anchor * 1000) + counter_offset_ms) % WRAP
        return wall_local, counter, true

    def test_counter_reproduces_utc(self):
        for after in (0.0, 4.321, 1800.5, -12.75):
            wall, counter, true = self.line(after)
            got, source = common.log_time_utc(wall, counter, [self.A], tz=CET)
            self.assertEqual(source, "counter", after)
            self.assertEqual(got, true, after)

    def test_anchor_records_are_accepted(self):
        wall, counter, true = self.line(4.321)
        record = {"utc": common.format_utc(self.A.utc), "tick_ms": self.A.tick_ms}
        self.assertEqual(common.log_time_utc(wall, counter, [record], tz=CET), (true, "counter"))

    def test_the_nearest_anchor_is_used(self):
        # A second anchor an hour later, whose tick count has drifted 300 ms.
        later = common.Anchor(self.A.utc + datetime.timedelta(hours=1),
                              self.A.tick_ms + 3_600_000 + 300)
        wall, counter, true = self.line(3600 + 10.0, counter_offset_ms=300)
        got, source = common.log_time_utc(wall, counter, [self.A, later], tz=CET)
        self.assertEqual((got, source), (true, "counter"))
        # Through A alone the same counter reads 300 ms late.
        got, _ = common.log_time_utc(wall, counter, [self.A], tz=CET)
        self.assertEqual(got - true, datetime.timedelta(milliseconds=300))

    def test_fallback_to_the_wall_clock_second(self):
        wall, counter, true = self.line(20.0)
        wall_utc = true.replace(microsecond=0)
        cases = [
            (1400, "counter"),   # 1.4 s past the wall second's end
            (2600, "wall"),      # 1.6 s past its end
            (-1400, "counter"),  # 1.4 s before its start
            (-1600, "wall"),     # 1.6 s before its start
            (-3_600_000, "wall"),
        ]
        for shift, expected in cases:
            got, source = common.log_time_utc(wall, (counter + shift) % WRAP, [self.A], tz=CET)
            self.assertEqual(source, expected, shift)
            if expected == "wall":
                self.assertEqual(got, wall_utc, shift)
            else:
                self.assertEqual(got, true + datetime.timedelta(milliseconds=shift), shift)

    def test_no_counter_or_no_anchor_is_wall(self):
        wall, counter, true = self.line(7.5)
        self.assertEqual(common.log_time_utc(wall, counter, [], tz=CET),
                         (true.replace(microsecond=0), "wall"))
        self.assertEqual(common.log_time_utc(wall, None, [self.A], tz=CET),
                         (true.replace(microsecond=0), "wall"))

    def test_across_the_32_bit_wrap(self):
        # The anchor sits 2 s before the counter wraps; the line 5 s after it
        # carries a small counter.
        anchor = common.Anchor(utc(2026, 10, 26, 10, 0, 0), WRAP - 2000)
        true = anchor.utc + datetime.timedelta(milliseconds=5250)
        wall = true.replace(microsecond=0).astimezone(CET).replace(tzinfo=None)
        self.assertEqual(common.log_time_utc(wall, 3250, [anchor], tz=CET), (true, "counter"))
        # A 64-bit anchor taken after the wrap converts a line from before it.
        anchor = common.Anchor(utc(2026, 10, 26, 10, 0, 0), 3 * WRAP + 1000)
        true = anchor.utc - datetime.timedelta(milliseconds=3500)
        wall = true.replace(microsecond=0).astimezone(CET).replace(tzinfo=None)
        self.assertEqual(common.log_time_utc(wall, WRAP - 2500, [anchor], tz=CET), (true, "counter"))
        # Across the wrap, a counter 3 s off still falls back.
        got = common.log_time_utc(wall, (WRAP - 2500 + 3000) % WRAP, [anchor], tz=CET)
        self.assertEqual(got, (true.replace(microsecond=0), "wall"))

    def test_split_log_prefix(self):
        line = "2026/10/26 11:28:04 90865203 3ef2321d [INFO Client 41234] : Beira of the Rotten Pack: Swell!"
        self.assertEqual(common.split_log_prefix(line),
                         (datetime.datetime(2026, 10, 26, 11, 28, 4), 90865203))
        self.assertIsNone(common.split_log_prefix("not a log line"))

    def test_local_to_utc(self):
        self.assertEqual(common.local_to_utc(datetime.datetime(2026, 10, 26, 11, 28, 17), CET),
                         utc(2026, 10, 26, 10, 28, 17))


class SampleAnchor(unittest.TestCase):
    def test_the_narrowest_sample_wins(self):
        base = 1_793_000_000_000_000_000
        # (before, after) per sample, in ns; the third is the narrowest.
        windows = [(0, 9_000_000), (20_000_000, 26_000_000), (40_000_000, 41_000_000),
                   (60_000_000, 64_000_000), (80_000_000, 90_000_000)]
        times = iter(base + t for w in windows for t in w)
        ticks = iter([100, 120, 140, 160, 180])
        anchor = common.sample_anchor(now_ns=lambda: next(times), tick_ms=lambda: next(ticks))
        self.assertEqual(anchor["tick_ms"], 140)
        expected = common.utc_from_epoch_ns(base + 40_500_000)
        self.assertEqual(anchor["utc"], common.format_utc(expected))
        self.assertEqual(sorted(anchor), ["tick_ms", "utc"])


class Session(TempDir):
    ANCHOR = {"utc": "2026-10-26T10:00:00.000Z", "tick_ms": 90_000_000}

    def setUp(self):
        super().setUp()
        self.live = self.tmp / "quest-tracker-live-5" / "beira-defeat"
        self.client_txt = self.tmp / "game" / "logs" / "Client.txt"
        self.client_txt.parent.mkdir(parents=True)
        self.client_txt.write_bytes(b"x" * 1234)
        self.ggpk = self.tmp / "game" / "Content.ggpk"
        self.ggpk.write_bytes(b"\0" * 77)

    def prep(self):
        return session.prep(self.live, self.client_txt, self.ggpk, sampler=lambda: dict(self.ANCHOR),
                            now=lambda: utc(2026, 10, 26, 9, 59, 0))

    def test_prep_lays_out_a_new_folder(self):
        data = self.prep()
        for sub in ("ggpk", "i1/rehearsal", "i2/rehearsal", "i3/frames/top", "i3/frames/bottom",
                    "i3/rehearsal", "ref", "tools"):
            self.assertTrue((self.live / sub).is_dir(), sub)
        on_disk = json.loads((self.live / "session.json").read_text(encoding="utf-8"))
        self.assertEqual(on_disk, data)
        self.assertEqual(sorted(on_disk), ["client_txt", "client_txt_offset", "created_utc",
                                           "ggpk", "ggpk_mtime", "ggpk_size"])
        self.assertEqual(on_disk["client_txt_offset"], 1234)
        self.assertEqual(on_disk["client_txt"], str(self.client_txt))
        self.assertEqual(on_disk["ggpk_size"], 77)
        self.assertEqual(on_disk["created_utc"], "2026-10-26T09:59:00.000Z")
        common.parse_utc(on_disk["ggpk_mtime"])
        self.assertEqual(common.read_jsonl(self.live / "clock.jsonl"), [self.ANCHOR])

    def test_prep_accepts_an_existing_empty_folder(self):
        self.live.mkdir(parents=True)
        self.prep()
        self.assertTrue((self.live / "session.json").is_file())

    def test_prep_refuses_a_folder_that_is_not_empty(self):
        self.live.mkdir(parents=True)
        (self.live / "store").mkdir()
        with self.assertRaises(common.ToolError):
            self.prep()
        self.assertEqual([p.name for p in self.live.iterdir()], ["store"])

    def test_prep_refuses_a_missing_log(self):
        self.client_txt.unlink()
        with self.assertRaises(common.ToolError):
            self.prep()
        self.assertFalse(self.live.exists())

    def test_clock_appends_an_anchor(self):
        self.prep()
        session.clock(self.live, sampler=lambda: {"utc": "2026-10-26T11:00:00.000Z", "tick_ms": 93_600_000})
        anchors = common.load_anchors(self.live)
        self.assertEqual([a.tick_ms for a in anchors], [90_000_000, 93_600_000])
        self.assertEqual(anchors[1].utc, utc(2026, 10, 26, 11, 0, 0))

    def test_clock_needs_the_folder(self):
        with self.assertRaises(common.ToolError):
            session.clock(self.live, sampler=lambda: dict(self.ANCHOR))

    def test_stop_creates_the_stop_files(self):
        self.prep()
        made = session.stop(self.live)
        names = ["stop-i1", "stop-i2", "stop-i3", "stop-marker"]
        self.assertEqual([p.name for p in made], names)
        for name in names:
            self.assertEqual((self.live / name).stat().st_size, 0, name)
        # Again: the files stay, nothing fails.
        session.stop(self.live)

    def test_stop_only(self):
        self.prep()
        self.assertEqual([p.name for p in session.stop(self.live, ["i1,marker"])],
                         ["stop-i1", "stop-marker"])
        self.assertFalse((self.live / "stop-i2").exists())
        with self.assertRaises(common.ToolError):
            session.stop(self.live, ["i4"])

    def test_command_line(self):
        self.prep()
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            self.assertEqual(session.main(["stop", "--live", str(self.live), "--only", "i2", "i3"]), 0)
            self.assertEqual(session.main(["stop", "--live", str(self.live), "--only", "x"]), 1)
            self.assertEqual(session.main(["prep", "--live", str(self.live), "--client-txt",
                                           str(self.client_txt), "--ggpk", str(self.ggpk)]), 1)
        self.assertTrue((self.live / "stop-i2").exists() and (self.live / "stop-i3").exists())
        self.assertIn("not empty", err.getvalue())


if __name__ == "__main__":
    unittest.main()
