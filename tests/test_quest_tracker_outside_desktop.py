"""tools/quest_tracker/outside/screen.py: the frames index, the fuzzy match,
the OCR cache and the marker file.

Nothing here opens a window, captures the screen, plays a sound or runs
PowerShell: the frames index, `ocr.jsonl` and `markers.txt` are small
synthetic files in temporary folders outside the repository, and the OCR
runner is a stand-in that appends the records the real `ocr.ps1` writes.

The baseline is a string that is not the target: far strings, and a line
whose letters are only partly the target's, stay under the threshold. The
targets are near strings (OCR misreads, a line with a speaker prefix) that
pass it, the frames a UTC window picks, OCR read only for frames not yet
cached, and the marker file's presses read back as UTC.
"""
import datetime
import json
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from tools.quest_tracker.outside import common, screen  # noqa: E402

UTC = datetime.timezone.utc


def utc(*args) -> datetime.datetime:
    return datetime.datetime(*args, tzinfo=UTC)


class TempDir(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = Path(self._tmp.name)
        self.assertIsNone(common.git_tree_of(self.tmp), "the temp folder sits inside a git checkout")

    def tearDown(self):
        self._tmp.cleanup()

    def write_lines(self, path: Path, records, tail: str = "") -> Path:
        with open(path, "w", encoding="utf-8", newline="\n") as f:
            for r in records:
                f.write((r if isinstance(r, str) else json.dumps(r)) + "\n")
            f.write(tail)
        return path


class Constants(unittest.TestCase):
    def test_the_workorder_constants_are_pinned(self):
        self.assertEqual(screen.MATCH_THRESHOLD, 0.8)
        self.assertEqual(screen.WINDOW_SLACK, 0.2)
        self.assertEqual(screen.HZ, 2)
        self.assertEqual(screen.TOP_BAND, (0.25, 0.0, 0.75, 0.14))
        self.assertEqual(screen.BOTTOM_BAND, (0.0, 0.35, 1.0, 1.0))
        self.assertEqual(screen.MAX_MINUTES, 90)
        self.assertEqual(screen.OCR_SCALE, 2)
        self.assertEqual(screen.SELFTEST_LINES, ("Beira of the Rotten Pack", "What happened here...?"))

    def test_the_scripts_sit_beside_the_module(self):
        for script in (screen.CAPTURE_PS1, screen.OCR_PS1, screen.PROBE_PS1, screen.MARKER_PS1):
            self.assertTrue(script.is_file(), script)
            self.assertNotIn(b"\r", script.read_bytes(), "%s has CR bytes; new files use LF" % script.name)


class FuzzyMatch(unittest.TestCase):
    def test_normalize_keeps_letters_digits_and_single_spaces(self):
        self.assertEqual(screen.normalize("  What happened\there...?  "), "what happened here")
        self.assertEqual(screen.normalize("Beira of the Rotten Pack:"), "beira of the rotten pack")
        self.assertEqual(screen.normalize("G1_4  Act 1"), "g14 act 1")
        self.assertEqual(screen.normalize("...!?"), "")

    def test_an_exact_line_scores_one(self):
        self.assertEqual(screen.fuzzy_score("Beira of the Rotten Pack", "Beira of the Rotten Pack"), 1.0)
        self.assertEqual(screen.fuzzy_score("What happened here...?", "what happened here"), 1.0)

    def test_the_target_inside_a_longer_line_scores_one(self):
        # The windows are what make this a match: the whole line's ratio is far lower.
        line = "Una: What happened here...? I heard a scream from the forest"
        self.assertEqual(screen.fuzzy_score("What happened here...?", line), 1.0)

    def test_near_strings_from_ocr_misreads_pass(self):
        for read in ("Beira of the R0tten Pack", "Beira of tbe Rotten Pack", "Beira ofthe Rotten Pack",
                     "Beira of the Rotten Pa"):
            with self.subTest(read=read):
                self.assertGreaterEqual(screen.fuzzy_score("Beira of the Rotten Pack", read), 0.8)
                self.assertTrue(screen.matches("Beira of the Rotten Pack", read))
        self.assertTrue(screen.matches("What happened here...?", "Wbat happencd here...?"))

    def test_far_strings_fail(self):
        for read in ("Una: Hello, exile", "Renly: Need anything mended?", "Clearfell Encampment",
                     "the pack", "Beira"):
            with self.subTest(read=read):
                self.assertLess(screen.fuzzy_score("Beira of the Rotten Pack", read), 0.8)
                self.assertFalse(screen.matches("Beira of the Rotten Pack", read))

    def test_the_threshold_sits_between_these_two_short_lines(self):
        # Target of 10 characters: the shortest window is 8, so these lines are compared whole.
        self.assertAlmostEqual(screen.fuzzy_score("abcdefghij", "abcdefg"), 14 / 17)
        self.assertTrue(screen.matches("abcdefghij", "abcdefg"))
        self.assertAlmostEqual(screen.fuzzy_score("abcdefghij", "abcdef"), 12 / 16)
        self.assertFalse(screen.matches("abcdefghij", "abcdef"))

    def test_windows_stay_within_twenty_percent_of_the_target(self):
        # The target's letters spread over twice its length: no window of 4..6
        # characters holds enough of them.
        self.assertLess(screen.fuzzy_score("beira", "b e i r a"), 0.8)

    def test_empty_strings_score_zero(self):
        self.assertEqual(screen.fuzzy_score("Beira", ""), 0.0)
        self.assertEqual(screen.fuzzy_score("", "Beira"), 0.0)
        self.assertEqual(screen.best_score("Beira", []), 0.0)

    def test_best_score_takes_the_best_line(self):
        self.assertEqual(screen.best_score("Beira of the Rotten Pack",
                                           ["Clearfell", "Beira of the Rotten Pack", "noise"]), 1.0)


def frame_record(when, band="bottom", path=None, mean=40.0, std=20.0, **extra):
    rec = {"utc": when, "band": band,
           "path": path if path is not None else "frames/%s/%s.jpg" % (band, when.replace("-", "").replace(":", "")),
           "rect": [0, 378, 1920, 1080], "client": [0, 0, 1920, 1080], "foreground": True,
           "luma_mean": mean, "luma_std": std}
    rec.update(extra)
    return rec


class FramesIndex(TempDir):
    def make_index(self, tail: str = "") -> Path:
        i3 = self.tmp / "i3"
        i3.mkdir()
        absolute = str(self.tmp / "elsewhere" / "x.jpg")
        self.write_lines(i3 / screen.FRAMES_JSONL, [
            frame_record("2026-10-26T10:28:17.500Z"),
            frame_record("2026-10-26T10:28:17.000Z", band="top"),
            frame_record("2026-10-26T10:28:17.000Z"),
            frame_record("2026-10-26T10:28:18.000Z", path=absolute, mean=1.2, std=0.4),
            {"utc": "2026-10-26T10:28:18.500Z", "band": "bottom", "path": None, "rect": None,
             "client": None, "foreground": False, "skipped": "minimized"},
            frame_record("2026-10-26T10:28:19.000Z", band="top"),
        ], tail=tail)
        return i3

    def test_the_index_reads_in_time_order_with_paths_resolved(self):
        i3 = self.make_index()
        frames = screen.read_frames(i3)
        self.assertEqual(len(frames), 6)
        self.assertEqual([f.utc for f in frames], sorted(f.utc for f in frames))
        first = frames[0]
        self.assertEqual(first.utc, utc(2026, 10, 26, 10, 28, 17))
        self.assertEqual(first.path.parent.parent.parent, i3)
        self.assertEqual(first.rect, (0, 378, 1920, 1080))
        self.assertTrue(first.foreground)
        by_time = {(f.utc, f.band): f for f in frames}
        self.assertEqual(by_time[(utc(2026, 10, 26, 10, 28, 18), "bottom")].path,
                         self.tmp / "elsewhere" / "x.jpg")

    def test_a_skipped_frame_is_kept_without_a_path(self):
        frames = screen.read_frames(self.make_index())
        skipped = [f for f in frames if f.skipped]
        self.assertEqual(len(skipped), 1)
        self.assertIsNone(skipped[0].path)
        self.assertEqual(skipped[0].skipped, "minimized")
        self.assertFalse(skipped[0].foreground)

    def test_the_index_file_itself_can_be_given(self):
        i3 = self.make_index()
        self.assertEqual(len(screen.read_frames(i3 / screen.FRAMES_JSONL)), 6)

    def test_a_writer_stopped_mid_line_is_tolerated(self):
        i3 = self.make_index(tail='{"utc": "2026-10-26T10:28:19.5')
        self.assertEqual(len(screen.read_frames(i3)), 6)

    def test_a_missing_index_and_a_bad_record_are_errors(self):
        with self.assertRaises(common.ToolError):
            screen.read_frames(self.tmp)
        self.write_lines(self.tmp / screen.FRAMES_JSONL, [{"band": "top", "path": "a.jpg"}])
        with self.assertRaises(common.ToolError) as ctx:
            screen.read_frames(self.tmp)
        self.assertIn("record 1", str(ctx.exception))

    def test_a_window_picks_captured_frames_inclusively(self):
        frames = screen.read_frames(self.make_index())
        start, end = utc(2026, 10, 26, 10, 28, 17), utc(2026, 10, 26, 10, 28, 18)
        picked = screen.frames_in_window(frames, start, end)
        # The sort is stable: of two frames at one time, the one written first comes first.
        self.assertEqual([(f.utc, f.band) for f in picked], [
            (utc(2026, 10, 26, 10, 28, 17), "top"), (utc(2026, 10, 26, 10, 28, 17), "bottom"),
            (utc(2026, 10, 26, 10, 28, 17, 500000), "bottom"), (utc(2026, 10, 26, 10, 28, 18), "bottom"),
        ])

    def test_a_window_filters_by_band_and_drops_skipped_frames(self):
        frames = screen.read_frames(self.make_index())
        start, end = utc(2026, 10, 26, 10, 28, 17, 1000), utc(2026, 10, 26, 10, 28, 19)
        bottom = screen.frames_in_window(frames, start, end, band="bottom")
        self.assertEqual([f.utc for f in bottom],
                         [utc(2026, 10, 26, 10, 28, 17, 500000), utc(2026, 10, 26, 10, 28, 18)])
        top = screen.frames_in_window(frames, start, end, band="top")
        self.assertEqual([f.utc for f in top], [utc(2026, 10, 26, 10, 28, 19)])
        self.assertEqual(screen.frames_in_window(frames, end, start), [])

    def test_select_frames_unions_windows_without_duplicates(self):
        frames = screen.read_frames(self.make_index())
        w1 = (utc(2026, 10, 26, 10, 28, 17), utc(2026, 10, 26, 10, 28, 17, 500000))
        w2 = (utc(2026, 10, 26, 10, 28, 17, 500000), utc(2026, 10, 26, 10, 28, 18))
        picked = screen.select_frames(frames, [w1, w2], "bottom", everything=False)
        self.assertEqual(len(picked), 3)
        self.assertEqual(len(screen.select_frames(frames, [], None, everything=True)), 5)

    def test_a_black_frame_is_dark_and_flat(self):
        frames = screen.read_frames(self.make_index())
        black = [f for f in frames if screen.is_black(f)]
        self.assertEqual([f.utc for f in black], [utc(2026, 10, 26, 10, 28, 18)])
        dark_scene = screen.Frame(utc(2026, 1, 1), "top", Path("a.jpg"), None, None, True, 5.0, 12.0, None)
        self.assertFalse(screen.is_black(dark_scene))


class OcrCache(TempDir):
    def fake_runner(self, results: dict, calls: list):
        """Stands in for ocr.ps1: appends one record per listed path, as it would."""
        def run(list_path, out_path, scale):
            listed = [l for l in Path(list_path).read_text(encoding="utf-8").split("\n") if l]
            calls.append(listed)
            with open(out_path, "a", encoding="utf-8", newline="\n") as f:
                for p in listed:
                    lines = results.get(Path(p).name, [])
                    f.write(json.dumps({"path": p, "scale": scale,
                                        "lines": [{"text": t, "box": [0, 0, 10, 10]} for t in lines]}) + "\n")
        return run

    def test_only_frames_not_yet_cached_are_read(self):
        a, b = self.tmp / "a.jpg", self.tmp / "b.jpg"
        calls = []
        runner = self.fake_runner({"a.jpg": ["Beira of the Rotten Pack"], "b.jpg": []}, calls)
        found, cached, read_now = screen.ocr_frames(self.tmp, [a], runner=runner)
        self.assertEqual((cached, read_now), (0, 1))
        self.assertEqual(found[screen.path_key(a)][0].text, "Beira of the Rotten Pack")
        found, cached, read_now = screen.ocr_frames(self.tmp, [a, b, a], runner=runner)
        self.assertEqual((cached, read_now), (1, 1))
        self.assertEqual(len(calls), 2)
        self.assertEqual([Path(p).name for p in calls[1]], ["b.jpg"])
        self.assertEqual(found[screen.path_key(b)], [])
        found, cached, read_now = screen.ocr_frames(self.tmp, [b], runner=runner)
        self.assertEqual((cached, read_now, len(calls)), (1, 0, 2))

    def test_a_failed_read_is_tried_again_and_a_later_record_wins(self):
        a = self.tmp / "a.jpg"
        self.write_lines(self.tmp / screen.OCR_JSONL, [
            {"path": str(a), "lines": [], "error": "file locked"},
            {"path": "b.jpg", "lines": [{"text": "old", "box": [0, 0, 1, 1]}]},
            {"path": "b.jpg", "lines": [{"text": "new", "box": [0, 0, 1, 1]}]},
        ])
        cache = screen.read_ocr(self.tmp / screen.OCR_JSONL)
        self.assertNotIn(screen.path_key(a), cache)
        self.assertEqual(cache[screen.path_key(self.tmp / "b.jpg")],
                         [screen.OcrLine("new", (0, 0, 1, 1))])

    def test_the_cache_refuses_a_git_checkout(self):
        tree = self.tmp / "checkout"
        (tree / ".git").mkdir(parents=True)
        calls = []
        with self.assertRaises(common.ToolError):
            screen.ocr_frames(tree / "i3", [tree / "i3" / "a.jpg"], runner=self.fake_runner({}, calls))
        self.assertEqual(calls, [])


class Rehearsal(TempDir):
    def frames(self, n=2, mean=40.0, std=20.0):
        return [screen.Frame(utc(2026, 10, 26, 10, 0, i), "bottom", self.tmp / ("%d.jpg" % i),
                             None, None, True, mean, std, None) for i in range(n)]

    def test_both_probe_lines_read_is_a_pass(self):
        frames = self.frames()
        ocr = {screen.path_key(frames[0].path): [screen.OcrLine("Beira of the Rotten Pack", (0, 0, 1, 1)),
                                                 screen.OcrLine("What happened here...?", (0, 2, 1, 3))]}
        expected, observed, verdict = screen.rehearsal_check(frames, ocr)
        self.assertEqual(verdict, "pass")
        self.assertIn("frames 2 (black 0, read 1)", observed)
        line = screen.check_line("i3-rehearsal", expected, observed, verdict)
        self.assertRegex(line, r"^- i3-rehearsal \| expected: .+ \| observed: .+ \| pass$")

    def test_a_missing_line_or_no_frames_is_a_fail(self):
        frames = self.frames(mean=0.5, std=0.1)
        ocr = {screen.path_key(frames[0].path): [screen.OcrLine("Beira of the Rotten Pack", (0, 0, 1, 1))]}
        _, observed, verdict = screen.rehearsal_check(frames, ocr)
        self.assertEqual(verdict, "fail")
        self.assertIn("black 2", observed)
        self.assertEqual(screen.rehearsal_check([], {})[2], "fail")

    def test_rehearse_refuses_a_git_checkout_and_a_used_folder(self):
        tree = self.tmp / "checkout"
        (tree / ".git").mkdir(parents=True)
        with self.assertRaises(common.ToolError):
            screen.rehearse(tree / "rehearsal")
        used = self.tmp / "used"
        used.mkdir()
        (used / "frames.jsonl").write_text("", encoding="utf-8")
        with self.assertRaises(common.ToolError):
            screen.rehearse(used)


class MarkerFile(TempDir):
    TEXT = ("# registered Ctrl+Alt+M (MOD_NOREPEAT) at 2026-10-26T10:20:00.000Z\n"
            "# pid 4242\n"
            "# tone 1760 Hz 150 ms\n"
            "1 2026-10-26T10:21:30.125Z\n"
            "2 2026-10-26T10:28:17.480Z\n"
            "# stopped 2026-10-26T10:40:00.000Z presses 2\n")

    def test_presses_and_header_read_back(self):
        m = screen.parse_markers(self.TEXT)
        self.assertEqual(m.presses, [screen.Press(1, utc(2026, 10, 26, 10, 21, 30, 125000)),
                                     screen.Press(2, utc(2026, 10, 26, 10, 28, 17, 480000))])
        self.assertEqual(m.header[0], "registered Ctrl+Alt+M (MOD_NOREPEAT) at 2026-10-26T10:20:00.000Z")
        self.assertIn("pid 4242", m.header)
        self.assertEqual(len(m.header), 4)

    def test_a_header_only_file_has_no_presses(self):
        self.assertEqual(screen.parse_markers("# registered x\n# pid 1\n").presses, [])

    def test_crlf_and_a_line_cut_mid_write_are_tolerated(self):
        m = screen.parse_markers(self.TEXT.replace("\n", "\r\n") + "3 2026-10-26T10:4")
        self.assertEqual(len(m.presses), 2)

    def test_a_bad_line_in_the_middle_is_an_error(self):
        for bad in ("x 2026-10-26T10:21:30.125Z", "1 10:21:30", "1 2026-10-26T10:21:30.125Z extra"):
            with self.subTest(bad=bad):
                with self.assertRaises(common.ToolError) as ctx:
                    screen.parse_markers("# pid 1\n" + bad + "\n2 2026-10-26T10:28:17.480Z\n", "m.txt")
                self.assertIn("m.txt:2", str(ctx.exception))

    def test_read_markers_reads_a_file_and_refuses_a_missing_one(self):
        path = self.tmp / common.MARKERS_TXT
        path.write_text(self.TEXT, encoding="utf-8")
        self.assertEqual(len(screen.read_markers(path).presses), 2)
        with self.assertRaises(common.ToolError):
            screen.read_markers(self.tmp / "none.txt")


class CommandLine(TempDir):
    def test_ocr_needs_a_window_or_all(self):
        import contextlib
        import io
        err = io.StringIO()
        with contextlib.redirect_stderr(err):
            code = screen.main(["ocr", "--live", str(self.tmp)])
        self.assertEqual(code, 1)
        self.assertIn("--window", err.getvalue())

    def test_a_window_that_ends_before_it_starts_is_refused(self):
        with self.assertRaises(common.ToolError):
            screen.parse_window(["2026-10-26T10:28:18.000Z", "2026-10-26T10:28:17.000Z"])


if __name__ == "__main__":
    unittest.main()
