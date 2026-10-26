"""The fuzzy match's parity fixture still holds against the Python original.

`apps/quest-tracker/fixtures/screen/fuzzy_parity.json` pins the scores that
`tools/quest_tracker/outside/screen.py`'s `fuzzy_score` gives a set of
synthetic pairs: upper-case bar text, OCR-noisy lines, a name inside a longer
line, lines shorter than the comparison window and non-ASCII text. The overlay's
Rust port (`crates/qt-screen`, test `fuzzy_matches_the_python_scores`) is
checked against the same file, so the two can only drift apart by one of them
failing. Nothing here touches a window, the screen or a capture.
"""

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from tools.quest_tracker.outside import screen  # noqa: E402

FIXTURE = ROOT / "apps" / "quest-tracker" / "fixtures" / "screen" / "fuzzy_parity.json"


def load():
    with open(FIXTURE, encoding="utf-8") as f:
        return json.load(f)


class FuzzyParity(unittest.TestCase):
    def test_constants_match_the_python_module(self):
        data = load()
        self.assertEqual(data["match_threshold"], screen.MATCH_THRESHOLD)
        self.assertEqual(data["window_slack"], screen.WINDOW_SLACK)

    def test_every_stored_score_is_what_python_gives(self):
        for pair in load()["pairs"]:
            with self.subTest(target=pair["target"], text=pair["text"]):
                self.assertEqual(screen.fuzzy_score(pair["target"], pair["text"]), pair["score"])

    def test_the_fixture_covers_the_cases_it_promises(self):
        pairs = load()["pairs"]
        self.assertGreaterEqual(len(pairs), 30)
        # Upper-case bar text, a line shorter than the window, a name inside a
        # longer line, non-ASCII text, and scores on both sides of the threshold.
        self.assertTrue(any(p["text"].isupper() and p["text"] != p["target"] for p in pairs))
        shorter = [p for p in pairs
                   if 0 < len(screen.normalize(p["text"])) < len(screen.normalize(p["target"])) * 0.8]
        self.assertTrue(shorter)
        self.assertTrue(any(len(screen.normalize(p["text"])) > 2 * len(screen.normalize(p["target"]))
                            and p["score"] == 1.0 for p in pairs))
        self.assertTrue(any(any(ord(c) > 127 for c in p["target"] + p["text"]) for p in pairs))
        self.assertTrue(any(p["score"] >= screen.MATCH_THRESHOLD for p in pairs))
        self.assertTrue(any(0 < p["score"] < screen.MATCH_THRESHOLD for p in pairs))


if __name__ == "__main__":
    unittest.main()
