"""The session analysis: every check of Live procedure 6, one capture line each.

Developer-only. Usage:

    py -3 -m tools.quest_tracker.outside.analyse [--live L] --pid P [--only NAME,...]
          [--listened <bark>,<kill>] [--confirmed yes|no]

It reads what the instruments left in the live folder `L` (the workorder's
"Live folder contract") and the session's part of `Client.txt`, computes each
check by the workorder's "Check rules", and prints one line per check, in the
procedure's order, in the capture's form:

    - <name> | expected: ... | observed: ... | <verdict>

It writes `L\\analysis.json` (per check: name, expected, observed, verdict and
details) and, on a full run, the two clips the person listens to:
`i2\\kill-clip.wav`, 5 s each side of the kill press, and `i2\\bark-clip.wav`,
5 s each side of one of her logged lines.

- `--only` computes the named checks (and, silently, the checks they depend
  on) and prints only those: `--only control` before anything else exists,
  `--only i2-town,i2-isolation,i3-town` in town while the instruments run. Its
  results are merged into an existing `analysis.json`.
- `--listened <bark>,<kill>` carries the person's two answers about the clips
  (`yes`, `no` or `unsure`, the first word of each counts) for `i2-listened`.
- `--confirmed yes|no` carries the person's answer to "was the press at her
  death?" for `marker`; without it `marker` cannot pass.

A missing input makes the checks that need it `not-run (instrument: <why>)`,
never a crash. An internal error in one check prints which check failed and
the analysis exits 1, after printing every other check.

Times. A log line's UTC comes from its counter through the clock anchors, or
its wall-clock second (`clientlog`). M is the first press after the `G1_2`
area line; the press before it is the test press. Her lines are the `Beira of
the Rotten Pack:` lines in `G1_2` before M; Una is the first `Una: What
happened here...?` in `G1_4`; a town line is an NPC line in `G1_town` after
the test press. The fight runs from her first line to M - 5 s. A window that
overlaps a stretch between `[WINDOW] Lost focus` and `Gained focus` is left
out of a count, and the count says so.

The base rate tiles the fight into 2 s windows, leaving out [L - 1, L + 3]
around every logged NPC line (and the focus-loss stretches): the share of the
remaining windows that hold a burst onset (I2) or a read of R (I1).

I1's region R is `/FMOD/Desktop/CarrionCrone.bank` plus the dialogue bundles
Rd: with a decoded index (`ggpk\\index.json` with paths) the bundles holding
`/carrioncrone/`, otherwise every `/audio/dialogue/` bundle read within
[L - 0.5, L + 1.5] of one of her lines. I2's bursts come from the recording's
300-3400 Hz envelope in 50 ms frames (`audio.band_envelope_db`); zeros the
recorder wrote into a gap never start one.

The template checks read `ref\\*.wav`, the references decoded to PCM (each the
sound alone): the one whose name holds `death` is the death cry, the rest are
barks. `ref\\refs.json`, when present, maps a bark's file name to its line's
text, so its L are only her lines with that text (a bark she did not say in
the fight is left out); without it every line of hers is a candidate L. The
correlation is over four band envelopes at 100 Hz as linear amplitude, each
band's mean removed, every lag of the whole recording; it costs about a
second per recorded minute per band, plus the correlation per reference.

Nothing here touches the game: it reads files the instruments wrote, and the
only programs it may start are `ocr.ps1` (for frames not yet in
`i3\\ocr.jsonl`). Chat text never reaches the output (`clientlog`).
"""
from __future__ import annotations

import argparse
import bisect
import datetime as _dt
import json
import math
import sys
import traceback
from operator import mul
from pathlib import Path
from typing import Callable

from tools.quest_tracker.outside import audio, clientlog, common, fileio, ggpk, screen
from tools.quest_tracker.outside.common import ToolError

# The procedure's order (Live procedure 6, "capture").
CHECKS = (
    "control", "marker", "ggpk-stable",
    "i1-rehearsal", "i1-rehearsal-mapped", "i2-rehearsal", "i3-rehearsal",
    "i2-town", "i2-isolation", "i3-town",
    "i1-control", "i1-control-una", "i1-read-timing", "i1-target",
    "i2-control", "i2-control-una", "i2-target", "i2-listened",
    "i2-template-control", "i2-template",
    "i3-control", "i3-control-una", "i3-bar-control", "i3-text-target", "i3-bar-target",
    "reward-split",
)

# --------------------------------------------------------------------------
# The constants of "Check rules". Tests pin them; changing one is an amendment.

LINE_BEFORE_S = 0.5          # a control window is [L - 0.5, L + 1.5]
LINE_AFTER_S = 1.5
TARGET_BEFORE_S = 3.0        # the target window is [M - 3, M + 2]
TARGET_AFTER_S = 2.0
FIGHT_END_BEFORE_M_S = 5.0   # the fight ends at M - 5
TILE_S = 2.0                 # base-rate windows
TILE_EXCLUDE_BEFORE_S = 1.0  # left out around each logged line: [L - 1, L + 3]
TILE_EXCLUDE_AFTER_S = 3.0
BASE_RATE_MAX = 0.5          # a target needs a base rate below this
I1_CONTROL_SHARE = 0.5       # reads at half or more of her lines
I1_QUIET_BEFORE_S = 1.5      # i1-target: none of her lines in [read - 1.5, read + 0.5]
I1_QUIET_AFTER_S = 0.5
FIRST_USE_S = 5.0            # i1-read-timing: within 5 s of her first line
AREA_LOAD_BEFORE_S = 2.0     # i1-read-timing: the G1_2 line -2 s to +20 s
AREA_LOAD_AFTER_S = 20.0
BURST_DB = 6.0               # a burst: >= 6 dB above the median of the preceding 2 s
BURST_HISTORY_S = 2.0
BURST_MIN_S = 0.150          # held for at least 150 ms
I2_CONTROL_SHARE = 2.0 / 3.0
I2_CONTROL_BASE_FACTOR = 2.0
ISOLATION_HZ = 1760.0        # the marker's tone
ISOLATION_SIDES_HZ = (1660.0, 1860.0)
ISOLATION_DB = 15.0
ISOLATION_S = 0.4
OCR_SCORE = screen.MATCH_THRESHOLD   # 0.8
OCR_LINE_S = 4.0             # a logged line is looked for in [L, L + 4]
I3_CONTROL_SHARE = 2.0 / 3.0
BAR_NAME = "beira of the rotten pack"
BAR_SHARE = 0.5
BAR_BEFORE_S = 10.0          # i3-bar-target: in a frame within [M - 10, M] ...
BAR_GONE_FROM_S = 1.0        # ... and in none within [M + 1, M + 10]
BAR_GONE_TO_S = 10.0
TEXT_BEFORE_S = 3.0          # i3-text-target: a new line within [M - 3, M + 4] ...
TEXT_AFTER_S = 4.0
TEXT_ABSENT_FROM_S = 13.0    # ... absent within [M - 13, M - 3]
TEXT_LOGGED_BEFORE_S = 15.0  # logged lines it is compared with: [M - 15, M + 5]
TEXT_LOGGED_AFTER_S = 5.0
TEXT_MIN_LETTERS = 4         # an OCR line with fewer letters is noise
TEMPLATE_FRAME_S = 0.01      # 100 Hz frames
TEMPLATE_BANDS = ((300.0, 700.0), (700.0, 1400.0), (1400.0, 2400.0), (2400.0, 3400.0))
TEMPLATE_NEAR_S = 1.5
TEMPLATE_FAR_S = 10.0
TEMPLATE_RATIO = 1.5
TEMPLATE_SHARE = 2.0 / 3.0
TEMPLATE_FLOOR_DB = -90.0
REWARD_SPLIT_S = 55.0
REWARD_TEXT = "+10% to [Resistances|Cold Resistance]"
CLIP_S = 5.0

BANK = "/FMOD/Desktop/CarrionCrone.bank"
DIALOGUE = "/audio/dialogue/"
CARRIONCRONE = "/carrioncrone/"
UNA_LINE = "What happened here...?"
RECORD_KINDS = ("file", "bundle", "bundle-header")

NOT_OBSERVED_I1 = ("not-observed (mapped reads: a page already in the system cache is a soft "
                   "fault and leaves no read)")

# Each check's `expected`, printed with its verdict, not-run included. The
# rehearsal lines carry the expectation the instrument module prints.
EXPECTED = {
    "control": "after the session offset, a LOG FILE OPENING, lines from [INFO Client <pid>] with the "
               "lease's pid, and a G1_town area line",
    "marker": "M, the first press after the G1_2 line, after her first line and before the reward line, "
              "and the person confirms it was at her death",
    "ggpk-stable": "tree-before.json equals tree-after.json",
    "i1-rehearsal": "both probe reads at offset A by the probe's pid",
    "i1-rehearsal-mapped": "a read of offset B's page by the probe's pid",
    "i2-rehearsal": "%d Hz at least %d dB above %d Hz in A's recording, A's onset within %d ms of its "
                    "logged start" % (audio.TONE_A_HZ, audio.SEPARATION_DB, audio.TONE_B_HZ,
                                      audio.ONSET_TOLERANCE_S * 1000),
    "i3-rehearsal": "each probe line scores >= %.1f in a captured frame" % OCR_SCORE,
    "i2-town": "a burst onset within [L - 0.5, L + 1.5] of a town line",
    "i2-isolation": "over [press, press + 0.4 s] of the test press, %d Hz not >= %d dB above the mean of "
                    "%d and %d Hz" % (ISOLATION_HZ, ISOLATION_DB, ISOLATION_SIDES_HZ[0], ISOLATION_SIDES_HZ[1]),
    "i3-town": "the town line scores >= %.1f in bottom-band frames within [L, L + 4 s]" % OCR_SCORE,
    "i1-control": "a read of R within [L - 0.5, L + 1.5] for at least half of her lines",
    "i1-control-una": "a read of an /audio/dialogue/ bundle within [Una - 0.5, Una + 1.5]",
    "i1-read-timing": "bank and dialogue classed as playback, first-use, area-load, not-read or "
                      "no-ggpk-reads; town lines read",
    "i1-target": "a read of R within [M - 3, M + 2], none of her lines within [read - 1.5, read + 0.5], "
                 "base rate below %.1f" % BASE_RATE_MAX,
    "i2-control": "a burst onset within [L - 0.5, L + 1.5] for at least 2/3 of her lines, and that share at "
                  "least twice the base rate",
    "i2-control-una": "a burst onset within [Una - 0.5, Una + 1.5]",
    "i2-target": "a burst onset within [M - 3, M + 2], and a base rate below %.1f" % BASE_RATE_MAX,
    "i2-listened": "the person hears a cry in kill-clip, having heard her voice in bark-clip",
    "i2-template-control": "each bark reference: top peak within L +/- 1.5 s at least 1.5 x the best peak "
                           "outside L +/- 10 s; at least 2/3 hits",
    "i2-template": "the death cry's top peak over the whole recording within [M - 3, M + 2], at least "
                   "1.5 x the best peak outside M +/- 10 s",
    "i3-control": "her lines score >= %.1f in bottom-band frames within [L, L + 4] for at least 2/3 of them"
                  % OCR_SCORE,
    "i3-control-una": "Una's line scores >= %.1f within [Una, Una + 4]" % OCR_SCORE,
    "i3-bar-control": "\"%s\" scores >= %.1f in the top band in at least half the frames between her first "
                      "and last lines" % (BAR_NAME, OCR_SCORE),
    "i3-text-target": "a bottom-band OCR line within [M - 3, M + 4] that matches no logged line and was "
                      "absent within [M - 13, M - 3]",
    "i3-bar-target": "the name in at least one top-band frame within [M - 10, M] and in none within "
                     "[M + 1, M + 10]",
    "reward-split": "the reward line (has received %s) at least %d s after M" % (REWARD_TEXT, REWARD_SPLIT_S),
}
assert tuple(EXPECTED) == CHECKS


class NotRun(Exception):
    """A check that cannot be computed; its verdict is `not-run (<why>)`."""

    def __init__(self, why: str):
        super().__init__(why)
        self.why = why


def missing(why: str) -> NotRun:
    return NotRun("instrument: " + why)


def _secs(a: _dt.datetime, b: _dt.datetime) -> float:
    return (a - b).total_seconds()


def _rel(t: _dt.datetime, ref: _dt.datetime) -> str:
    return "%+.2f s" % _secs(t, ref)


def _hms(t: _dt.datetime) -> str:
    return common.format_utc(t)[11:]


def _td(s: float) -> _dt.timedelta:
    return _dt.timedelta(seconds=s)


def overlaps(a0, a1, b0, b1) -> bool:
    return a0 < b1 and a1 > b0


def result(name: str, expected: str, observed: str, verdict: str, **details) -> dict:
    return {"name": name, "expected": expected, "observed": observed, "verdict": verdict,
            "details": details}


def clean(text) -> str:
    return " ".join(str(text).replace("|", "/").split())


def capture_line(r: dict) -> str:
    return "- %s | expected: %s | observed: %s | %s" % (
        r["name"], clean(r["expected"]), clean(r["observed"]), r["verdict"])


def split_capture_line(line: str) -> tuple[str, str, str, str]:
    """(name, expected, observed, verdict) of a capture line another module printed."""
    body = line[2:] if line.startswith("- ") else line
    name, rest = body.split(" | ", 1)
    head, verdict = rest.rsplit(" | ", 1)
    expected, observed = head.split(" | observed: ", 1)
    if expected.startswith("expected: "):
        expected = expected[len("expected: "):]
    return name, expected, observed, verdict


# --------------------------------------------------------------------------
# Pure helpers the tests pin

def burst_onsets(env: list[float], frame_s: float, db: float = BURST_DB,
                 history_s: float = BURST_HISTORY_S, min_s: float = BURST_MIN_S,
                 blocked: Callable[[int, int], bool] | None = None) -> list[int]:
    """Frame indexes where a burst starts: a frame at least `db` above the
    median of the preceding `history_s`, held for at least `min_s`. A frame
    needs a full history. `blocked(first, last)` drops an onset whose history
    or burst touches recorder-filled zeros."""
    hist = max(1, int(round(history_s / frame_s)))
    need = max(1, int(math.ceil(min_s / frame_s - 1e-9)))
    window: list[float] = []
    onsets = []
    run = 0
    for i, v in enumerate(env):
        above = False
        if i >= hist:
            k = len(window)
            med = window[k // 2] if k % 2 else 0.5 * (window[k // 2 - 1] + window[k // 2])
            above = v >= med + db
        if above:
            run += 1
            if run == need:
                first = i - need + 1
                if blocked is None or not blocked(first - hist, i):
                    onsets.append(first)
        else:
            run = 0
        bisect.insort(window, v)
        if len(window) > hist:
            del window[bisect.bisect_left(window, env[i - hist])]
    return onsets


def base_tiles(start: _dt.datetime, end: _dt.datetime, excluded) -> list[tuple[_dt.datetime, _dt.datetime]]:
    """The fight's 2 s windows [start + 2k, start + 2k + 2) that end by `end`
    and overlap none of the `excluded` (start, end) stretches."""
    out = []
    t = start
    while t + _td(TILE_S) <= end:
        u = t + _td(TILE_S)
        if not any(overlaps(t, u, a, b) for a, b in excluded):
            out.append((t, u))
        t = u
    return out


def share_of_tiles(tiles, events: list[_dt.datetime]) -> float | None:
    if not tiles:
        return None
    hit = sum(1 for a, b in tiles if any(a <= e < b for e in events))
    return hit / len(tiles)


def ncc_series(rec: list[list[float]], tpl: list[list[float]]) -> list[float]:
    """Normalized cross-correlation of a multi-band template against a
    multi-band series, at every lag where the template fits. Each band has
    its own mean removed (on both sides), and the bands' covariances and
    variances are summed, so a sound has to match the template's shape in
    every band at once, not just its on-and-off."""
    bands = len(tpl)
    n = len(tpl[0]) if tpl else 0
    total = len(rec[0]) if rec else 0
    if n < 2 or total < n:
        return []
    t0 = []
    for b in tpl:
        mean = sum(b) / n
        t0.append([v - mean for v in b])
    norm_t = math.sqrt(sum(v * v for b in t0 for v in b))
    if norm_t <= 0:
        return []
    # Per band, prefix sums of x and x^2. With a zero-mean template the
    # series' own mean drops out of the numerator.
    s1, s2 = [], []
    for b in range(bands):
        p1, p2 = [0.0] * (total + 1), [0.0] * (total + 1)
        row = rec[b]
        for j in range(total):
            p1[j + 1] = p1[j] + row[j]
            p2[j + 1] = p2[j] + row[j] * row[j]
        s1.append(p1)
        s2.append(p2)
    out = []
    for k in range(total - n + 1):
        num = 0.0
        var = 0.0
        for b in range(bands):
            num += sum(map(mul, rec[b][k:k + n], t0[b]))
            sx = s1[b][k + n] - s1[b][k]
            var += (s2[b][k + n] - s2[b][k]) - sx * sx / n
        out.append(num / (math.sqrt(var) * norm_t) if var > 1e-12 else 0.0)
    return out


def template_peak(series: list[float], lag_utc: Callable[[int], _dt.datetime],
                  near: list[_dt.datetime], near_s: float = TEMPLATE_NEAR_S,
                  far_s: float = TEMPLATE_FAR_S) -> tuple[float, float, int | None]:
    """(top peak within near_s of any `near` time, best peak farther than far_s
    from all of them, the near peak's lag)."""
    best_near, best_far, at = -1.0, -1.0, None
    for k, v in enumerate(series):
        t = lag_utc(k)
        ds = [abs(_secs(t, x)) for x in near]
        if ds and min(ds) <= near_s:
            if v > best_near:
                best_near, at = v, k
        elif not ds or min(ds) > far_s:
            best_far = max(best_far, v)
    return best_near, best_far, at


def is_hit(near: float, far: float, ratio: float = TEMPLATE_RATIO) -> bool:
    return near > 0 and near >= ratio * max(far, 0.0)


def letters(text: str) -> int:
    return sum(1 for c in text if c.isalpha())


# --------------------------------------------------------------------------
# The session

class Session:
    """The live folder's inputs, read lazily; each accessor raises NotRun when
    its input is missing."""

    def __init__(self, live: Path, pid: int, listened: str | None = None,
                 confirmed: str | None = None, tz: _dt.tzinfo | None = None,
                 ocr_runner: Callable = screen.run_ocr_ps1):
        self.live = Path(live)
        self.pid = int(pid)
        self.listened = listened
        self.confirmed = confirmed
        self.tz = tz
        self.ocr_runner = ocr_runner
        self._memo: dict = {}

    def _once(self, key, make):
        if key not in self._memo:
            try:
                self._memo[key] = ("ok", make())
            except NotRun as err:
                self._memo[key] = ("not-run", err)
        kind, value = self._memo[key]
        if kind == "not-run":
            raise value
        return value

    # ---- the session file, the clock and the log

    def session_json(self) -> dict:
        def make():
            path = self.live / common.SESSION_JSON
            if not path.is_file():
                raise missing("no session.json in %s (run `session prep`)" % self.live)
            return common.read_json(path)
        return self._once("session", make)

    def anchors(self) -> list:
        return self._once("anchors", lambda: common.load_anchors(self.live))

    def log(self) -> clientlog.ClientLog:
        def make():
            s = self.session_json()
            path = Path(s["client_txt"])
            if not path.is_file():
                raise missing("no Client.txt at %s" % path)
            return clientlog.read_log(path, int(s["client_txt_offset"]), self.anchors(), self.tz)
        return self._once("log", make)

    def markers(self) -> screen.Markers:
        def make():
            path = self.live / common.MARKERS_TXT
            if not path.is_file():
                raise missing("no markers.txt (the marker did not run)")
            return screen.read_markers(path)
        return self._once("markers", make)

    def g1_2(self):
        areas = self.log().areas("G1_2")
        return areas[0] if areas else None

    def town_area(self):
        areas = self.log().areas("G1_town")
        return areas[0] if areas else None

    def test_press(self):
        presses = self.markers().presses
        g = self.g1_2()
        before = [p for p in presses if g is None or p.utc < g.utc]
        return before[0] if before else None

    def kill(self):
        """M: the first press after the G1_2 area line."""
        try:
            presses = self.markers().presses
        except NotRun:
            return None
        g = self.g1_2()
        if g is None:
            return None
        after = [p for p in presses if p.utc > g.utc]
        return after[0] if after else None

    def need_kill(self):
        m = self.kill()
        if m is None:
            self.markers()  # a missing marker file says so first
            raise missing("no kill press after the G1_2 area line")
        return m.utc

    def her_lines(self) -> list:
        m = self.kill()
        return [l for l in self.log().npc(clientlog.BEIRA, "G1_2") if m is None or l.utc < m.utc]

    def need_her_lines(self) -> list:
        lines = self.her_lines()
        if not lines:
            raise NotRun("log: no line from Beira of the Rotten Pack in G1_2")
        return lines

    def una(self):
        for l in self.log().npc(clientlog.UNA, "G1_4"):
            if screen.normalize(l.text) == screen.normalize(UNA_LINE):
                return l
        return None

    def town_lines(self) -> list:
        tp = self.test_press()
        return [l for l in self.log().npc(area="G1_town") if l.speaker in clientlog.TOWN_SPEAKERS
                and (tp is None or l.utc > tp.utc)]

    def reward(self):
        m = self.kill()
        rewards = [l for l in self.log().rewards() if REWARD_TEXT in l.text]
        after = [l for l in rewards if m is None or l.utc > m.utc]
        return (after or rewards or [None])[0]

    def focus_losses(self):
        return self.log().focus_losses()

    def focus_lost_in(self, a, b) -> bool:
        return any(overlaps(a, b, x, y) for x, y in self.focus_losses())

    def split_focus(self, lines, before: float, after: float):
        """(lines kept, lines left out because their window overlaps a focus loss)."""
        kept, out = [], []
        for l in lines:
            (out if self.focus_lost_in(l.utc - _td(before), l.utc + _td(after)) else kept).append(l)
        return kept, out

    def fight(self) -> tuple[_dt.datetime, _dt.datetime]:
        lines = self.need_her_lines()
        return lines[0].utc, self.need_kill() - _td(FIGHT_END_BEFORE_M_S)

    def tiles(self):
        def make():
            start, end = self.fight()
            excluded = [(l.utc - _td(TILE_EXCLUDE_BEFORE_S), l.utc + _td(TILE_EXCLUDE_AFTER_S))
                        for l in self.log().npc()]
            excluded += self.focus_losses()
            return base_tiles(start, end, excluded)
        return self._once("tiles", make)

    # ---- I1

    def reads(self) -> list[tuple[_dt.datetime, dict]]:
        def make():
            path = self.live / "i1" / "mapped.jsonl"
            if not path.is_file():
                raise missing("no session trace (no i1\\mapped.jsonl)")
            out = []
            for rec in common.read_jsonl(path):
                if rec.get("utc") is None:
                    continue
                if rec.get("pid") not in (None, self.pid):
                    continue
                out.append((common.parse_utc(rec["utc"]), rec))
            out.sort(key=lambda p: p[0])
            return out
        return self._once("reads", make)

    @staticmethod
    def names_of(rec: dict) -> list[str]:
        return [e.get("name") or "" for e in rec.get("records") or [] if e.get("kind") in RECORD_KINDS]

    def dialogue_bundles(self) -> tuple[set[str], str]:
        """Rd and how it was found."""
        def make():
            index_path = self.live / "ggpk" / "index.json"
            if index_path.is_file():
                from tools.quest_tracker.outside import bundle_index
                index = bundle_index.load_index(index_path)
                keys = {bundle_index.bundle_key(index.bundles[f["bundle"]]["name"])
                        for f in index.files if f.get("path") and CARRIONCRONE in f["path"].lower()}
                if keys:
                    names = set()
                    for _, rec in self.reads():
                        for name in self.names_of(rec):
                            if bundle_index.bundle_key(name) in keys:
                                names.add(name)
                    return names, "decoded index: %d bundle(s) hold %s" % (len(keys), CARRIONCRONE)
            names = set()
            for line in self.her_lines():
                for t, rec in self.reads_between(line.utc - _td(LINE_BEFORE_S), line.utc + _td(LINE_AFTER_S)):
                    names.update(n for n in self.names_of(rec) if DIALOGUE in n.lower())
            return names, "no decoded index: dialogue bundles read at her lines"
        return self._once("rd", make)

    def region(self) -> set[str]:
        rd, _ = self.dialogue_bundles()
        return {BANK} | rd

    def reads_between(self, a, b):
        reads = self.reads()
        times = [t for t, _ in reads]
        i = bisect.bisect_left(times, a)
        out = []
        while i < len(reads) and reads[i][0] <= b:
            out.append(reads[i])
            i += 1
        return out

    def reads_of(self, names: set[str], a=None, b=None):
        src = self.reads() if a is None else self.reads_between(a, b)
        return [(t, r) for t, r in src if any(n in names for n in self.names_of(r))]

    def dialogue_reads(self, a, b):
        return [(t, r) for t, r in self.reads_between(a, b)
                if any(DIALOGUE in n.lower() for n in self.names_of(r))]

    # ---- I2

    def wav(self) -> Path:
        path = self.live / "i2" / "game.wav"
        if not path.is_file():
            raise missing("no i2\\game.wav (the recorder did not run)")
        return path

    def recording(self):
        def make():
            wav = self.wav()
            try:
                reader = audio.WavReader(wav)
                tl = audio.Timeline.of_wav(wav)
            except ToolError as err:
                raise missing("the recording cannot be read: %s" % err)
            return reader, tl
        return self._once("recording", make)

    def envelope(self):
        """(envelope dB per frame, samples per frame, onset UTCs)."""
        def make():
            reader, tl = self.recording()
            env = audio.band_envelope_db(reader)
            frame = int(round(audio.ENVELOPE_FRAME_S * reader.sample_rate))
            gaps = [(s, s + n) for s, n in tl.gaps]

            def blocked(first, last):
                lo, hi = max(0, first) * frame, (last + 1) * frame
                return any(overlaps(lo, hi, a, b) for a, b in gaps)

            onsets = burst_onsets(env, frame / float(reader.sample_rate), blocked=blocked)
            return env, frame, [tl.utc_of(i * frame) for i in onsets]
        return self._once("envelope", make)

    def onsets(self) -> list[_dt.datetime]:
        return self.envelope()[2]

    def onsets_in(self, a, b) -> list[_dt.datetime]:
        return [t for t in self.onsets() if a <= t <= b]

    # ---- I3

    def frames(self) -> list:
        def make():
            path = self.live / "i3" / screen.FRAMES_JSONL
            if not path.is_file():
                raise missing("no i3\\frames.jsonl (the capture did not run)")
            return screen.read_frames(self.live / "i3")
        return self._once("frames", make)

    def ocr_windows(self) -> list[tuple[str, _dt.datetime, _dt.datetime]]:
        """Every (band, start, end) any i3 check reads, so OCR runs once."""
        w = []
        try:
            self.log()
        except NotRun:
            return w
        try:
            town = self.town_lines()
        except NotRun:  # no marker file: no test press to count town lines from
            town = []
        her = self.her_lines()
        for l in town + her:
            w.append(("bottom", l.utc, l.utc + _td(OCR_LINE_S)))
        una = self.una()
        if una is not None:
            w.append(("bottom", una.utc, una.utc + _td(OCR_LINE_S)))
        if her:
            w.append(("top", her[0].utc, her[-1].utc))
        m = self.kill()
        if m is not None:
            w.append(("bottom", m.utc - _td(TEXT_ABSENT_FROM_S), m.utc + _td(TEXT_AFTER_S)))
            w.append(("top", m.utc - _td(BAR_BEFORE_S), m.utc + _td(BAR_GONE_TO_S)))
        return w

    def ocr(self) -> dict:
        def make():
            frames = self.frames()
            picked = {}
            for band, a, b in self.ocr_windows():
                for f in screen.frames_in_window(frames, a, b, band):
                    picked[screen.path_key(f.path)] = f.path
            try:
                found, _, _ = screen.ocr_frames(self.live / "i3", picked.values(), runner=self.ocr_runner)
            except ToolError as err:
                raise missing("OCR failed: %s" % err)
            return found
        return self._once("ocr", make)

    def window_frames(self, band: str, a, b) -> list:
        return screen.frames_in_window(self.frames(), a, b, band)

    def best_in(self, target: str, band: str, a, b) -> tuple[float, int, int]:
        """(best score of `target` in `band` frames within [a, b], frames, black frames)."""
        frames = self.window_frames(band, a, b)
        ocr = self.ocr()
        texts = [l.text for f in frames for l in screen.frame_lines(f, ocr)]
        return (screen.best_score(target, texts), len(frames),
                sum(1 for f in frames if screen.is_black(f)))


# --------------------------------------------------------------------------
# The checks

def check_control(s: Session) -> dict:
    expected = EXPECTED["control"].replace("<pid>", str(s.pid))
    log = s.log()
    openings = log.openings()
    mine = log.pids.get(s.pid, 0)
    others = sorted(p for p in log.pids if p != s.pid)
    town = [a for a in log.areas("G1_town") if a.pid == s.pid]
    observed = "LOG FILE OPENING x%d%s; %d lines from Client %d%s; G1_town %s; times from counter %d, wall %d" % (
        len(openings), (" (first %s)" % _hms(openings[0].utc)) if openings else "", mine, s.pid,
        (" (other pids: %s)" % ", ".join(str(p) for p in others)) if others else "",
        _hms(town[0].utc) if town else "none", log.sources["counter"], log.sources["wall"])
    ok = bool(openings) and mine > 0 and bool(town)
    return result("control", expected, observed, "pass" if ok else "fail",
                  openings=len(openings), pid_lines=mine, other_pids=others)


def check_marker(s: Session) -> dict:
    expected = EXPECTED["marker"]
    presses = s.markers().presses
    m = s.kill()
    her = s.her_lines()
    reward = s.reward()
    tp = s.test_press()
    parts = ["presses %d" % len(presses), "test press %s" % (_hms(tp.utc) if tp else "none")]
    ok = m is not None
    if m is None:
        parts.append("no press after the G1_2 line" if s.g1_2() else "no G1_2 line")
    else:
        parts.append("M %s" % _hms(m.utc))
        if her:
            parts.append("%s after her first line" % _rel(m.utc, her[0].utc))
            ok = ok and m.utc > her[0].utc
        else:
            parts.append("no line from her")
            ok = False
        if reward is not None:
            parts.append("%.1f s before the reward line" % _secs(reward.utc, m.utc))
            ok = ok and m.utc < reward.utc
        else:
            parts.append("no reward line")
            ok = False
    answer = (s.confirmed or "").strip().lower()
    parts.append("person: %s" % (answer or "no answer (pass --confirmed yes|no)"))
    ok = ok and answer.split()[:1] == ["yes"]
    return result("marker", expected, "; ".join(parts), "pass" if ok else "fail",
                  m=common.format_utc(m.utc) if m else None)


def check_ggpk_stable(s: Session) -> dict:
    expected = EXPECTED["ggpk-stable"]
    before, after = s.live / "ggpk" / "tree-before.json", s.live / "ggpk" / "tree-after.json"
    for p in (before, after):
        if not p.is_file():
            raise missing("no ggpk\\%s" % p.name)
    changes = ggpk.diff_trees(ggpk.load_tree(before), ggpk.load_tree(after))
    if not changes:
        return result("ggpk-stable", expected, "identical", "pass")
    shown = "; ".join(changes[:5]) + (" (and %d more)" % (len(changes) - 5) if len(changes) > 5 else "")
    return result("ggpk-stable", expected, "%d records changed: %s (mapping uses tree-after)"
                  % (len(changes), shown), "fail", changes=changes)


def _from_lines(lines: list[str]) -> dict[str, dict]:
    out = {}
    for line in lines:
        name, expected, observed, verdict = split_capture_line(line)
        out[name] = result(name, expected, observed, verdict)
    return out


def _i1_rehearsal(s: Session) -> dict[str, dict]:
    def make():
        folder = fileio.rehearsal_dir(s.live)
        probe = folder / "probe.json"
        if not probe.is_file():
            raise missing("no i1\\rehearsal\\probe.json; the controller did not reach the probe")
        xml = folder / "rehearsal.xml"
        if not xml.is_file():
            raise missing("no i1\\rehearsal\\rehearsal.xml; run `fileio rehearsal-check`")
        probe_rec = common.read_json(probe)
        scan = fileio.scan_trace(xml, pid=None, suffix=fileio.suffix_of(probe_rec["file"]))
        return _from_lines(fileio.judge_rehearsal(probe_rec, scan))
    return s._once("i1-rehearsal-lines", make)


def check_i1_rehearsal(s: Session) -> dict:
    return _i1_rehearsal(s)["i1-rehearsal"]


def check_i1_rehearsal_mapped(s: Session) -> dict:
    return _i1_rehearsal(s)["i1-rehearsal-mapped"]


def check_i2_rehearsal(s: Session) -> dict:
    folder = s.live / "i2" / "rehearsal"
    wavs = sorted(folder.glob("selftest-*.wav")) if folder.is_dir() else []
    wavs = [w for w in wavs if Path(str(w)[:-4] + "-a.jsonl").is_file()]
    if not wavs:
        raise missing("no selftest recording in i2\\rehearsal (run `audio selftest --out`)")
    wav = wavs[-1]
    starts = common.read_jsonl(Path(str(wav)[:-4] + "-a.jsonl"))
    if not starts:
        raise missing("tone A logged no start in %s" % wav.name)
    try:
        v = audio.measure_selftest(wav, common.parse_utc(starts[0]["start_utc"]))
    except ToolError as err:
        raise missing("the selftest recording cannot be read: %s" % err)
    name, expected, observed, verdict = split_capture_line(audio.rehearsal_line(v))
    return result(name, expected, observed + " (%s)" % wav.name, verdict)


def check_i3_rehearsal(s: Session) -> dict:
    folder = s.live / "i3" / "rehearsal"
    if not (folder / screen.FRAMES_JSONL).is_file():
        raise missing("no i3\\rehearsal\\frames.jsonl (run `screen rehearse`)")
    frames = [f for f in screen.read_frames(folder) if f.band == "bottom"]
    try:
        ocr, _, _ = screen.ocr_frames(folder, [f.path for f in frames if f.path is not None],
                                      runner=s.ocr_runner)
    except ToolError as err:
        raise missing("OCR failed: %s" % err)
    expected, observed, verdict = screen.rehearsal_check(frames, ocr)
    return result("i3-rehearsal", expected, observed, verdict)


def check_i2_town(s: Session) -> dict:
    expected = EXPECTED["i2-town"]
    lines = s.town_lines()
    if not lines:
        return result("i2-town", expected, "no town NPC line after the test press", "fail")
    s.recording()
    parts, ok = [], False
    for l in lines:
        hits = s.onsets_in(l.utc - _td(LINE_BEFORE_S), l.utc + _td(LINE_AFTER_S))
        parts.append("%s %s: %s" % (l.speaker, _hms(l.utc),
                                    ("onset " + _rel(hits[0], l.utc)) if hits else "no onset"))
        ok = ok or bool(hits)
    observed = "%d town line(s): %s; onsets in the recording %d" % (len(lines), "; ".join(parts), len(s.onsets()))
    return result("i2-town", expected, observed, "pass" if ok else "fail")


def check_i2_isolation(s: Session) -> dict:
    expected = EXPECTED["i2-isolation"]
    tp = s.test_press()
    if tp is None:
        raise missing("no test press before the G1_2 line")
    reader, tl = s.recording()
    a = int(round(tl.sample_of(tp.utc)))
    n = int(round(ISOLATION_S * reader.sample_rate))
    if a < 0 or a + n > reader.frames:
        raise missing("the recording does not cover the test press at %s" % _hms(tp.utc))
    mono = reader.read_mono(a, n)
    rate = reader.sample_rate
    level = audio.goertzel_db(mono, rate, ISOLATION_HZ)
    sides = [audio.goertzel_db(mono, rate, hz) for hz in ISOLATION_SIDES_HZ]
    mean = sum(sides) / len(sides)
    observed = "test press %s: %d Hz %.1f dBFS, sides %.1f and %.1f dBFS, %.1f dB above their mean" % (
        _hms(tp.utc), ISOLATION_HZ, level, sides[0], sides[1], level - mean)
    return result("i2-isolation", expected, observed, "fail" if level - mean >= ISOLATION_DB else "pass")


def check_i3_town(s: Session) -> dict:
    expected = EXPECTED["i3-town"]
    lines = s.town_lines()
    s.frames()
    if not lines:
        return result("i3-town", expected, "no town NPC line after the test press", "fail")
    parts, ok, black = [], False, 0
    for l in lines:
        score, n, b = s.best_in(l.text, "bottom", l.utc, l.utc + _td(OCR_LINE_S))
        black += b
        parts.append("%s %s: %.2f in %d frames" % (l.speaker, _hms(l.utc), score, n))
        ok = ok or score >= OCR_SCORE
    observed = "; ".join(parts) + "; black frames %d" % black
    return result("i3-town", expected, observed, "pass" if ok else "fail", black_frames=black)


def _left_out(n: int) -> str:
    return " (%d left out: focus lost)" % n if n else ""


def check_i1_control(s: Session) -> dict:
    expected = EXPECTED["i1-control"]
    s.reads()
    lines = s.need_her_lines()
    kept, out = s.split_focus(lines, LINE_BEFORE_S, LINE_AFTER_S)
    region = s.region()
    hits = [l for l in kept if s.reads_of(region, l.utc - _td(LINE_BEFORE_S), l.utc + _td(LINE_AFTER_S))]
    _, how = s.dialogue_bundles()
    share = len(hits) / len(kept) if kept else 0.0
    observed = "%d of %d lines%s; R = bank + %d dialogue bundle(s) (%s)" % (
        len(hits), len(kept), _left_out(len(out)), len(region) - 1, how)
    ok = bool(kept) and share >= I1_CONTROL_SHARE
    return result("i1-control", expected, observed, "pass" if ok else "fail",
                  region=sorted(region), hits=len(hits), counted=len(kept))


def check_i1_control_una(s: Session) -> dict:
    expected = EXPECTED["i1-control-una"]
    s.reads()
    una = s.una()
    if una is None:
        return result("i1-control-una", expected, "no \"%s\" from Una in G1_4" % UNA_LINE, "fail")
    reads = s.dialogue_reads(una.utc - _td(LINE_BEFORE_S), una.utc + _td(LINE_AFTER_S))
    if reads:
        t, rec = reads[0]
        names = [n for n in s.names_of(rec) if DIALOGUE in n.lower()]
        observed = "Una %s: read %s of %s" % (_hms(una.utc), _rel(t, una.utc), ", ".join(names))
        return result("i1-control-una", expected, observed, "pass")
    return result("i1-control-una", expected, "Una %s: no dialogue-bundle read" % _hms(una.utc), "fail")


def _classify(s: Session, names: set[str]) -> str:
    log = s.log()
    town = s.town_area()
    reward = s.reward()
    lo = town.utc if town else (s.reads()[0][0] if s.reads() else None)
    hi = reward.utc if reward else (log.lines[-1].utc if log.lines else None)
    if lo is None or hi is None or not s.reads_between(lo, hi):
        return "no-ggpk-reads"
    lines = s.her_lines()
    first = lines[0].utc
    xs = s.reads_of(names) if names else []
    at = [l for l in lines if s.reads_of(names, l.utc - _td(LINE_BEFORE_S), l.utc + _td(LINE_AFTER_S))] \
        if names else []
    if at and len(at) >= len(lines) / 2.0 and any(l.utc > first for l in at):
        return "playback"
    end = s.kill().utc if s.kill() else hi
    in_fight = [t for t, _ in xs if first - _td(LINE_BEFORE_S) <= t <= end]
    if in_fight and all(t <= first + _td(FIRST_USE_S) for t in in_fight):
        return "first-use"
    g = s.g1_2()
    if not at and g is not None and any(
            g.utc - _td(AREA_LOAD_BEFORE_S) <= t <= g.utc + _td(AREA_LOAD_AFTER_S) for t, _ in xs):
        return "area-load"
    if not xs:
        return "not-read"
    return "unclassified (reads at %d of %d lines)" % (len(at), len(lines))


def check_i1_read_timing(s: Session) -> dict:
    expected = EXPECTED["i1-read-timing"]
    s.reads()
    s.need_her_lines()
    rd, how = s.dialogue_bundles()
    bank = _classify(s, {BANK})
    dialogue = _classify(s, rd)
    town = s.town_lines()
    read = [l for l in town if s.dialogue_reads(l.utc - _td(LINE_BEFORE_S), l.utc + _td(LINE_AFTER_S))]
    observed = "bank: %s; dialogue: %s; town lines read: %d of %d (Rd: %s)" % (
        bank, dialogue, len(read), len(town), how)
    ok = not bank.startswith("unclassified") and not dialogue.startswith("unclassified")
    return result("i1-read-timing", expected, observed, "pass" if ok else "fail",
                  bank=bank, dialogue=dialogue)


def _cache_state(s: Session) -> tuple[bool | None, str]:
    """(cold or None when unknown, the text every i1-target verdict prints)."""
    anchors = s.anchors()
    if not anchors:
        return None, "uptime at prep unknown (no clock anchor)"
    first = anchors[0]
    boot = first.utc - _dt.timedelta(milliseconds=first.tick_ms)
    sess = s.session_json()
    try:
        last, unparsed = clientlog.scan_openings_before(
            Path(sess["client_txt"]), int(sess["client_txt_offset"]), s.tz)
        found = (common.format_utc(last) if last else
                 "none parsed (%d LOG FILE OPENING mark(s))" % unparsed if unparsed else "none in the log")
    except ToolError as err:
        last, found = None, "unknown (%s)" % err
    text = "uptime at prep %d ms (boot %s); last game start before the session %s" % (
        first.tick_ms, common.format_utc(boot), found)
    # Without a game start to compare the boot with, nothing says the cache
    # is cold: an absent or unreadable marker is not "no game since the boot".
    if last is None:
        return None, text + ": cache state unknown"
    cold = boot > last
    return cold, text + (": cold" if cold else ": warm")


def check_i1_target(s: Session, done: dict) -> dict:
    expected = EXPECTED["i1-target"]
    cold, cache = _cache_state(s)
    try:
        s.reads()
        m = s.need_kill()
        lines = s.need_her_lines()
    except NotRun as err:
        raise NotRun(err.why + "; " + cache)
    region = s.region()
    cands = []
    for t, rec in s.reads_of(region, m - _td(TARGET_BEFORE_S), m + _td(TARGET_AFTER_S)):
        if not any(t - _td(I1_QUIET_BEFORE_S) <= l.utc <= t + _td(I1_QUIET_AFTER_S) for l in lines):
            cands.append((t, rec))
    tiles = s.tiles()
    base = share_of_tiles(tiles, [t for t, _ in s.reads_of(region)])
    base_text = "base rate %s over %d windows" % ("n/a" if base is None else "%.2f" % base, len(tiles))
    if cands and base is not None and base < BASE_RATE_MAX:
        t, rec = cands[0]
        recs = [e for e in rec.get("records") or [] if (e.get("name") or "") in region]
        e = recs[0] if recs else {}
        files = [f.get("path") for f in rec.get("files") or []]
        observed = "read %s of %s, block(s) %s%s; %d candidate(s); %s; %s" % (
            _rel(t, m), e.get("name"), e.get("blocks"), ("; files " + ", ".join(map(str, files))) if files else "",
            len(cands), base_text, cache)
        return result("i1-target", expected, observed, "pass", record=e, files=files)
    observed = "%d read(s) of R in the window without a line of hers nearby; %s; %s" % (len(cands), base_text, cache)
    failed = [n for n in ("i1-control", "i1-rehearsal", "i1-rehearsal-mapped") if done[n]["verdict"] != "pass"]
    if not failed and cold:
        return result("i1-target", expected, observed, NOT_OBSERVED_I1)
    if failed:
        why = ", ".join("%s %s" % (n, done[n]["verdict"].split()[0]) for n in failed)
    else:
        why = "warm cache" if cold is False else "cache state unknown"
    return result("i1-target", expected, observed, "not-run (instrument: %s)" % why)


def check_i2_control(s: Session) -> dict:
    expected = EXPECTED["i2-control"]
    s.recording()
    lines = s.need_her_lines()
    kept, out = s.split_focus(lines, LINE_BEFORE_S, LINE_AFTER_S)
    hits = [l for l in kept if s.onsets_in(l.utc - _td(LINE_BEFORE_S), l.utc + _td(LINE_AFTER_S))]
    share = len(hits) / len(kept) if kept else 0.0
    tiles = s.tiles()
    base = share_of_tiles(tiles, s.onsets())
    observed = "%d of %d lines%s (%.2f); base rate %s over %d windows" % (
        len(hits), len(kept), _left_out(len(out)), share, "n/a" if base is None else "%.2f" % base, len(tiles))
    ok = bool(kept) and share >= I2_CONTROL_SHARE and base is not None and share >= I2_CONTROL_BASE_FACTOR * base
    return result("i2-control", expected, observed, "pass" if ok else "fail", share=share, base_rate=base)


def check_i2_control_una(s: Session) -> dict:
    expected = EXPECTED["i2-control-una"]
    s.recording()
    una = s.una()
    if una is None:
        return result("i2-control-una", expected, "no \"%s\" from Una in G1_4" % UNA_LINE, "fail")
    hits = s.onsets_in(una.utc - _td(LINE_BEFORE_S), una.utc + _td(LINE_AFTER_S))
    observed = "Una %s: %s" % (_hms(una.utc), ("onset " + _rel(hits[0], una.utc)) if hits else "no onset")
    return result("i2-control-una", expected, observed, "pass" if hits else "fail")


def check_i2_target(s: Session, done: dict) -> dict:
    expected = EXPECTED["i2-target"]
    s.recording()
    m = s.need_kill()
    s.need_her_lines()
    hits = s.onsets_in(m - _td(TARGET_BEFORE_S), m + _td(TARGET_AFTER_S))
    tiles = s.tiles()
    base = share_of_tiles(tiles, s.onsets())
    observed = "%s; base rate %s over %d windows" % (
        ("onset " + ", ".join(_rel(t, m) for t in hits)) if hits else "no onset in the window",
        "n/a" if base is None else "%.2f" % base, len(tiles))
    if hits and base is not None and base < BASE_RATE_MAX:
        return result("i2-target", expected, observed, "pass", onsets=[_secs(t, m) for t in hits])
    failed = [n for n in ("i2-control", "i2-rehearsal") if done[n]["verdict"] != "pass"]
    if not failed:
        return result("i2-target", expected, observed, "not-observed")
    return result("i2-target", expected, observed, "not-run (instrument: %s)" % ", ".join(
        "%s %s" % (n, done[n]["verdict"].split()[0]) for n in failed))


def _answer(text: str) -> str:
    words = text.strip().lower().replace(":", " ").split()
    return words[0] if words else ""


def check_i2_listened(s: Session) -> dict:
    expected = EXPECTED["i2-listened"]
    if not s.listened:
        raise NotRun("person: no answer yet (pass --listened <bark>,<kill>)")
    bark_text, _, kill_text = s.listened.partition(",")
    bark, kill = _answer(bark_text), _answer(kill_text)
    observed = "bark-clip: %s; kill-clip: %s" % (bark_text.strip() or "none", kill_text.strip() or "none")
    if kill == "yes":
        return result("i2-listened", expected, observed, "pass")
    if bark == "yes":
        return result("i2-listened", expected, observed, "not-observed")
    return result("i2-listened", expected, observed, "not-run (the listening control failed)")


def _amplitude(db: float) -> float:
    if not math.isfinite(db) or db <= TEMPLATE_FLOOR_DB:
        return 0.0
    return 10.0 ** (db / 20.0)


def _bands(s: Session, reader: audio.WavReader) -> list[list[float]]:
    """The four band envelopes at 100 Hz, as linear amplitude (dB would make
    any sound after silence look alike). A band a file's sample rate cannot
    carry is flat zero, the same for every file, so it adds nothing."""
    out = []
    frames = reader.frames // int(round(TEMPLATE_FRAME_S * reader.sample_rate))
    for lo, hi in TEMPLATE_BANDS:
        if lo >= 0.45 * reader.sample_rate:
            out.append([0.0] * frames)
            continue
        env = audio.band_envelope_db(reader, lo=lo, hi=hi, frame_s=TEMPLATE_FRAME_S)
        out.append([_amplitude(v) for v in env])
    n = min(len(b) for b in out) if out else 0
    return [b[:n] for b in out]


def _references(s: Session) -> tuple[list[Path], list[Path], dict]:
    folder = s.live / "ref"
    wavs = sorted(folder.glob("*.wav")) if folder.is_dir() else []
    death = [w for w in wavs if "death" in w.name.lower()]
    barks = [w for w in wavs if "death" not in w.name.lower()]
    texts = {}
    if (folder / "refs.json").is_file():
        texts = common.read_json(folder / "refs.json")
    return death, barks, texts


def _template(s: Session) -> dict:
    def make():
        death, barks, texts = _references(s)
        if not death and not barks:
            raise missing("no reference audio in ref\\")
        reader, tl = s.recording()
        rec = _bands(s, reader)
        frame = int(round(TEMPLATE_FRAME_S * reader.sample_rate))
        lag_utc = lambda k: tl.utc_of(k * frame)
        series = {}
        for w in death + barks:
            series[w.name] = ncc_series(rec, _bands(s, audio.WavReader(w)))
        return {"death": death, "barks": barks, "texts": texts, "series": series, "lag_utc": lag_utc}
    return s._once("template", make)


def check_i2_template_control(s: Session) -> dict:
    expected = EXPECTED["i2-template-control"]
    tpl = _template(s)
    if not tpl["barks"]:
        raise missing("no bark reference in ref\\")
    lines = s.need_her_lines()
    hits, counted, parts = 0, 0, []
    for w in tpl["barks"]:
        text = tpl["texts"].get(w.name)
        if text:
            near = [l.utc for l in lines if screen.fuzzy_score(text, l.text) >= OCR_SCORE]
            if not near:
                parts.append("%s: not said in this fight" % w.name)
                continue
        else:
            near = [l.utc for l in lines]
        top, far, _ = template_peak(tpl["series"][w.name], tpl["lag_utc"], near)
        counted += 1
        hit = is_hit(top, far)
        hits += hit
        parts.append("%s: %.2f vs %.2f%s" % (w.name, top, far, " hit" if hit else ""))
    ok = counted > 0 and hits / counted >= TEMPLATE_SHARE
    observed = "%d of %d references hit; %s" % (hits, counted, "; ".join(parts))
    return result("i2-template-control", expected, observed, "pass" if ok else "fail", hits=hits, counted=counted)


def check_i2_template(s: Session, done: dict) -> dict:
    expected = EXPECTED["i2-template"]
    tpl = _template(s)
    if not tpl["death"]:
        raise missing("no death-cry reference in ref\\")
    m = s.need_kill()
    w = tpl["death"][0]
    series = tpl["series"][w.name]
    if not series:
        raise missing("the reference %s is longer than the recording" % w.name)
    k = max(range(len(series)), key=series.__getitem__)
    t = tpl["lag_utc"](k)
    _, far, _ = template_peak(series, tpl["lag_utc"], [m], near_s=0.0, far_s=TEMPLATE_FAR_S)
    observed = "%s: top peak %.2f at %s, best outside M +/- 10 s %.2f" % (w.name, series[k], _rel(t, m), far)
    inside = m - _td(TARGET_BEFORE_S) <= t <= m + _td(TARGET_AFTER_S)
    if inside and is_hit(series[k], far):
        return result("i2-template", expected, observed, "pass")
    if done["i2-template-control"]["verdict"] == "pass":
        return result("i2-template", expected, observed, "not-observed")
    return result("i2-template", expected, observed, "not-run (instrument: i2-template-control %s)"
                  % done["i2-template-control"]["verdict"].split()[0])


def check_i3_control(s: Session) -> dict:
    expected = EXPECTED["i3-control"]
    s.frames()
    lines = s.need_her_lines()
    kept, out = s.split_focus(lines, 0.0, OCR_LINE_S)
    hits, black = 0, 0
    for l in kept:
        score, _, b = s.best_in(l.text, "bottom", l.utc, l.utc + _td(OCR_LINE_S))
        hits += score >= OCR_SCORE
        black += b
    ok = bool(kept) and hits / len(kept) >= I3_CONTROL_SHARE
    observed = "%d of %d lines%s; black frames %d" % (hits, len(kept), _left_out(len(out)), black)
    return result("i3-control", expected, observed, "pass" if ok else "fail")


def check_i3_control_una(s: Session) -> dict:
    expected = EXPECTED["i3-control-una"]
    s.frames()
    una = s.una()
    if una is None:
        return result("i3-control-una", expected, "no \"%s\" from Una in G1_4" % UNA_LINE, "fail")
    score, n, b = s.best_in(una.text, "bottom", una.utc, una.utc + _td(OCR_LINE_S))
    observed = "Una %s: %.2f in %d frames (black %d)" % (_hms(una.utc), score, n, b)
    return result("i3-control-una", expected, observed, "pass" if score >= OCR_SCORE else "fail")


def _has_name(s: Session, frame) -> bool:
    return screen.best_score(BAR_NAME, [l.text for l in screen.frame_lines(frame, s.ocr())]) >= OCR_SCORE


def check_i3_bar_control(s: Session) -> dict:
    expected = EXPECTED["i3-bar-control"]
    s.frames()
    lines = s.need_her_lines()
    frames = s.window_frames("top", lines[0].utc, lines[-1].utc)
    with_name = sum(1 for f in frames if _has_name(s, f))
    black = sum(1 for f in frames if screen.is_black(f))
    share = with_name / len(frames) if frames else 0.0
    observed = "%d of %d top-band frames (%.2f); black %d" % (with_name, len(frames), share, black)
    return result("i3-bar-control", expected, observed, "pass" if frames and share >= BAR_SHARE else "fail")


def check_i3_text_target(s: Session, done: dict) -> dict:
    expected = EXPECTED["i3-text-target"]
    s.frames()
    m = s.need_kill()
    log = s.log()
    ocr = s.ocr()
    earlier = [l.text for f in s.window_frames("bottom", m - _td(TEXT_ABSENT_FROM_S), m - _td(TEXT_BEFORE_S))
               for l in screen.frame_lines(f, ocr)]
    found = []
    for f in s.window_frames("bottom", m - _td(TEXT_BEFORE_S), m + _td(TEXT_AFTER_S)):
        for line in screen.frame_lines(f, ocr):
            if letters(line.text) < TEXT_MIN_LETTERS:
                continue
            if any(max(screen.fuzzy_score(line.text, x), screen.fuzzy_score(x, line.text)) >= OCR_SCORE
                   for x in [c["text"] for c in found]):
                continue
            if log.repeats_logged(line.text, m - _td(TEXT_LOGGED_BEFORE_S), m + _td(TEXT_LOGGED_AFTER_S),
                                  screen.fuzzy_score, OCR_SCORE):
                continue
            if any(max(screen.fuzzy_score(line.text, x), screen.fuzzy_score(x, line.text)) >= OCR_SCORE
                   for x in earlier):
                continue
            found.append({"text": line.text, "box": list(line.box), "utc": common.format_utc(f.utc)})
    if found:
        observed = "%d new line(s): %s" % (len(found), "; ".join(
            "box %s at %s" % (c["box"], _rel(common.parse_utc(c["utc"]), m)) for c in found))
        return result("i3-text-target", expected, observed, "pass", lines=found)
    observed = "no new line in %d frames" % len(s.window_frames("bottom", m - _td(TEXT_BEFORE_S), m + _td(TEXT_AFTER_S)))
    if done["i3-control"]["verdict"] == "pass":
        return result("i3-text-target", expected, observed, "not-observed")
    return result("i3-text-target", expected, observed, "not-run (instrument: i3-control %s)"
                  % done["i3-control"]["verdict"].split()[0])


def check_i3_bar_target(s: Session, done: dict) -> dict:
    expected = EXPECTED["i3-bar-target"]
    s.frames()
    m = s.need_kill()
    before = s.window_frames("top", m - _td(BAR_BEFORE_S), m)
    after = [f for f in s.window_frames("top", m + _td(BAR_GONE_FROM_S), m + _td(BAR_GONE_TO_S))
             if not screen.is_black(f)]
    named_before = [f for f in before if _has_name(s, f)]
    named_after = [f for f in after if _has_name(s, f)]
    observed = "name in %d of %d frames before M, %d of %d (not black) after M + 1 s" % (
        len(named_before), len(before), len(named_after), len(after))
    if named_before:
        observed += "; last frame with the name %s" % _rel(max(named_before, key=lambda f: f.utc).utc, m)
    if named_before and after and not named_after:
        return result("i3-bar-target", expected, observed, "pass",
                      last_named=_secs(max(named_before, key=lambda f: f.utc).utc, m))
    if not after:
        return result("i3-bar-target", expected, observed,
                      "not-run (instrument: no readable top-band frame within [M + 1, M + 10])")
    if done["i3-bar-control"]["verdict"] == "pass":
        return result("i3-bar-target", expected, observed, "not-observed")
    return result("i3-bar-target", expected, observed, "not-run (instrument: i3-bar-control %s)"
                  % done["i3-bar-control"]["verdict"].split()[0])


def check_reward_split(s: Session) -> dict:
    expected = EXPECTED["reward-split"]
    m = s.kill()
    reward = s.reward()
    if m is None or reward is None:
        observed = "%s; %s" % ("M %s" % _hms(m.utc) if m else "no kill press",
                               "reward %s" % _hms(reward.utc) if reward else "no reward line")
        return result("reward-split", expected, observed, "fail")
    gap = _secs(reward.utc, m.utc)
    observed = "reward %s, %.1f s after M" % (_hms(reward.utc), gap)
    return result("reward-split", expected, observed, "pass" if gap >= REWARD_SPLIT_S else "fail", gap=gap)


PLAIN = {
    "control": check_control, "marker": check_marker, "ggpk-stable": check_ggpk_stable,
    "i1-rehearsal": check_i1_rehearsal, "i1-rehearsal-mapped": check_i1_rehearsal_mapped,
    "i2-rehearsal": check_i2_rehearsal, "i3-rehearsal": check_i3_rehearsal,
    "i2-town": check_i2_town, "i2-isolation": check_i2_isolation, "i3-town": check_i3_town,
    "i1-control": check_i1_control, "i1-control-una": check_i1_control_una,
    "i1-read-timing": check_i1_read_timing,
    "i2-control": check_i2_control, "i2-control-una": check_i2_control_una,
    "i2-listened": check_i2_listened, "i2-template-control": check_i2_template_control,
    "i3-control": check_i3_control, "i3-control-una": check_i3_control_una,
    "i3-bar-control": check_i3_bar_control, "reward-split": check_reward_split,
}
# A target and the checks its verdict reads.
DEPENDENT = {
    "i1-target": (check_i1_target, ("i1-control", "i1-rehearsal", "i1-rehearsal-mapped")),
    "i2-target": (check_i2_target, ("i2-control", "i2-rehearsal")),
    "i2-template": (check_i2_template, ("i2-template-control",)),
    "i3-text-target": (check_i3_text_target, ("i3-control",)),
    "i3-bar-target": (check_i3_bar_target, ("i3-bar-control",)),
}
assert set(PLAIN) | set(DEPENDENT) == set(CHECKS)

def compute(s: Session, names) -> tuple[dict[str, dict], list[str]]:
    """Every check in `names` and what they depend on. Returns (results, errors)."""
    done: dict[str, dict] = {}
    errors: list[str] = []

    def one(name):
        if name in done:
            return done[name]
        try:
            if name in DEPENDENT:
                fn, deps = DEPENDENT[name]
                for d in deps:
                    one(d)
                r = fn(s, done)
            else:
                r = PLAIN[name](s)
        except NotRun as err:
            r = result(name, EXPECTED[name], "not computed: " + err.why, "not-run (%s)" % err.why)
        except Exception as err:  # noqa: BLE001 - reported per check, then exit 1
            errors.append("check %s failed: %s: %s" % (name, type(err).__name__, err))
            errors.append(traceback.format_exc().rstrip())
            r = result(name, EXPECTED[name], "internal error: %s" % err, "not-run (analysis error)")
            r["error"] = True
        done[name] = r
        return r

    for name in names:
        one(name)
    return done, errors


def write_clips(s: Session) -> list[str]:
    """The two clips, when the recording, M and a line of hers allow; one note each."""
    notes = []
    try:
        wav = s.wav()
        m = s.need_kill()
    except NotRun as err:
        return ["clips: not written (%s)" % err.why]
    i2 = s.live / "i2"
    try:
        audio.clip(wav, m, CLIP_S, CLIP_S, i2 / "kill-clip.wav")
        notes.append("kill-clip.wav: 5 s each side of M %s" % _hms(m))
    except ToolError as err:
        notes.append("kill-clip.wav: not written (%s)" % err)
    lines = s.her_lines()
    if not lines:
        notes.append("bark-clip.wav: not written (no line from her)")
        return notes
    try:
        pick = next((l for l in lines if s.onsets_in(l.utc - _td(LINE_BEFORE_S), l.utc + _td(LINE_AFTER_S))),
                    lines[0])
    except NotRun:
        pick = lines[0]
    try:
        audio.clip(wav, pick.utc, CLIP_S, CLIP_S, i2 / "bark-clip.wav")
        notes.append("bark-clip.wav: 5 s each side of her line at %s" % _hms(pick.utc))
    except ToolError as err:
        notes.append("bark-clip.wav: not written (%s)" % err)
    return notes


def parse_only(values) -> list[str]:
    if not values:
        return list(CHECKS)
    names = [n.strip() for v in values for n in v.split(",") if n.strip()]
    for n in names:
        if n not in CHECKS:
            raise ToolError("unknown check %r for --only: expected some of %s" % (n, ", ".join(CHECKS)))
    return [n for n in CHECKS if n in names]


def analyse(live: Path, pid: int, only=None, listened: str | None = None, confirmed: str | None = None,
            tz: _dt.tzinfo | None = None, ocr_runner: Callable = screen.run_ocr_ps1,
            out=None) -> tuple[int, list[dict]]:
    """Compute, print and write. Returns (exit code, the printed results)."""
    out = out or sys.stdout
    live = Path(live)
    common.refuse_inside_git(live, "analysis")
    if not live.is_dir():
        raise ToolError("no live folder at %s" % live)
    names = parse_only(only)
    s = Session(live, pid, listened=listened, confirmed=confirmed, tz=tz, ocr_runner=ocr_runner)
    done, errors = compute(s, names)
    shown = [done[n] for n in names]
    for r in shown:
        print(capture_line(r), file=out)

    path = live / common.ANALYSIS_JSON
    previous = {}
    if only and path.is_file():
        try:
            previous = {c["name"]: c for c in common.read_json(path).get("checks", [])}
        except (ValueError, KeyError, TypeError):
            previous = {}
    merged = dict(previous)
    merged.update({r["name"]: r for r in shown})
    common.write_json(path, {
        "created_utc": common.format_utc(common.now_utc()),
        "pid": pid,
        "only": names if only else None,
        "checks": [merged[n] for n in CHECKS if n in merged],
    })
    notes = ["wrote %s" % path.name]
    if not only:
        notes += write_clips(s)
    for note in notes:
        print("analyse: " + note, file=out)
    for e in errors:
        print("analyse: error: " + e, file=sys.stderr)
    return (1 if errors else 0), shown


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(prog="analyse", description="The session analysis: one capture line per check.")
    p.add_argument("--live", type=Path, default=None)
    p.add_argument("--pid", type=int, required=True, help="the game's pid, from the lease")
    p.add_argument("--only", nargs="+", default=None, metavar="NAME",
                   help="checks to compute and print (comma- or space-separated)")
    p.add_argument("--listened", default=None, metavar="BARK,KILL",
                   help="the person's answers about bark-clip and kill-clip: yes, no or unsure")
    p.add_argument("--confirmed", default=None, metavar="yes|no",
                   help="the person's answer: was the press at her death?")
    return p


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    live = args.live if args.live is not None else common.default_live_dir()
    try:
        code, _ = analyse(live, args.pid, only=args.only, listened=args.listened, confirmed=args.confirmed)
    except ToolError as err:
        print("analyse: error: %s" % err, file=sys.stderr)
        return 1
    except (OSError, ValueError, json.JSONDecodeError) as err:
        print("analyse: error: %s" % err, file=sys.stderr)
        return 1
    return code


if __name__ == "__main__":
    sys.exit(main())
