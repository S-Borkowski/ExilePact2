"""tools/quest_tracker/outside: the session analysis (`analyse`), the log reader
(`clientlog`) and the instruments' start and stop (`run`).

Nothing here touches a game, a device, a window or a trace. Each test builds a
synthetic live folder in a temporary directory outside the repository:

- `Client.txt` is `tests/fixtures/quest_tracker/outside/session/before.txt`
  (an older session, before the offset) followed by `client.txt` (this
  session), written with CRLF line endings as the client writes them. The
  session follows Live 4's Beira timeline (her nine lines from "Sleep in
  ice..." to the last "Swell!", the kill 13 s later), with the gaps between
  areas shortened, a town vendor line, the reward line and Una's line in the
  Grelwood. The chat and local-chat lines and the player's name are invented.
- The recording is a 4 kHz mono WAV with low noise and 800 Hz bursts where
  the session speaks, with the recorder's sidecar. The frames index points at
  JPEGs that do not exist; their OCR results are already in `ocr.jsonl`, and
  the OCR runner is a stand-in that reads nothing.
- The mapped reads are a JSON-lines file of texture reads, the bank's read at
  the area entry and a dialogue bundle's read at each line.
- The rehearsal trace is the fileio tests' `rehearsal-pass.xml`.

The baseline is the session as Live 4 measured it: nothing at the kill and
every control present. It gives every control `pass` and every target
`not-observed`. The targets each add one signal at the kill (a burst, a
dialogue-bundle read, a boss-bar name that vanishes, a new chat line) and
turn one target to `pass`. The negative controls take the controls away
(the targets become `not-run`), make the fight's audio busy (`i2-target`
stays `not-observed` despite a burst at the kill), lose focus around one of
her lines (that line leaves the count), warm the cache, and play the
marker's tone into the recording.
"""
import contextlib
import datetime
import io
import json
import math
import random
import shutil
import sys
import tempfile
import threading
import time
import unittest
from array import array
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from tools.quest_tracker.outside import analyse, audio, clientlog, common, ggpk, run, screen  # noqa: E402

FIXTURES = ROOT / "tests" / "fixtures" / "quest_tracker" / "outside" / "session"
TRACERPT = ROOT / "tests" / "fixtures" / "quest_tracker" / "outside" / "tracerpt"
UTC = datetime.timezone.utc
TZ = datetime.timezone(datetime.timedelta(hours=1))   # CET, the fixture's local time
PID = 4242
RATE = 4000
TICK_AT_ORIGIN = 90_000_000                            # the counter at 10:00:00Z
ORIGIN = datetime.datetime(2026, 10, 26, 10, 0, 0, tzinfo=UTC)
PROBE = {   # the fileio tests' probe, which rehearsal-pass.xml was written for
    "pid": 3131,
    "file": r"C:\Users\example\live\i1\rehearsal\probe.bin",
    "offset_a": 4 * 2 ** 20,
    "size_a": 64 * 2 ** 10,
    "reads_utc": ["2026-10-26T10:00:02.000Z", "2026-10-26T10:00:03.000Z"],
    "offset_b": 32 * 2 ** 20,
    "mapped_utc": "2026-10-26T10:00:04.000Z",
}


def z(hms: str) -> datetime.datetime:
    """10:27:04.300 on the session's day, UTC."""
    h, m, s = hms.split(":")
    sec = float(s)
    return datetime.datetime(2026, 10, 26, int(h), int(m), int(sec), round((sec % 1) * 1e6), tzinfo=UTC)


TEST_PRESS = z("10:24:30.000")
M = z("10:28:17.000")
TOWN = z("10:24:35.400")
G1_2 = z("10:26:40.100")
HER = [z(t) for t in ("10:27:04.300", "10:27:27.100", "10:27:27.900", "10:27:37.200", "10:27:41.500",
                      "10:27:45.300", "10:27:53.600", "10:27:57.400", "10:28:04.700")]
HER_TEXT = ["Sleep in ice, wake in snow. Sleep my pretties as you grow...", "Be still!", "Invader!",
            "Feed!", "Be still!", "Rotten!", "Swell!", "Be still!", "Swell!"]
UNA = z("10:29:50.250")
REWARD = z("10:29:35.600")
WAV_START = z("10:24:25.000")
WAV_END = z("10:30:00.000")
FRAMES_FROM = z("10:24:25.000")
FRAMES_TO = z("10:30:00.000")

BANK = "/FMOD/Desktop/CarrionCrone.bank"
MONSTERS = "/Bundles2/Streaming/Folders/audio/dialogue/4.0/monsters.17.bundle.bin"
NPC_TOWN = "/Bundles2/Streaming/Folders/audio/dialogue/4.0/npc.2.bundle.bin"
NPC_UNA = "/Bundles2/Streaming/Folders/audio/dialogue/4.0/npc.5.bundle.bin"
TEXTURES = "/Bundles2/Folders/textures.5.bundle.bin"
NEW_LINE = "A strange whisper fades away"


def log_line(when: datetime.datetime, message: str, level: str = "INFO", pid: int = PID) -> str:
    local = when.astimezone(TZ)
    tick = TICK_AT_ORIGIN + round((when - ORIGIN).total_seconds() * 1000)
    return "%s %d 5b8c1d07 [%s Client %d] %s" % (local.strftime("%Y/%m/%d %H:%M:%S"), tick, level, pid, message)


def no_ocr(list_path, out_path, scale):
    """The OCR stand-in: every frame it is asked about reads as empty."""
    with open(out_path, "a", encoding="utf-8") as f:
        for p in Path(list_path).read_text(encoding="utf-8").split("\n"):
            if p.strip():
                f.write(json.dumps({"path": p.strip(), "lines": []}) + "\n")


_NOISE = {}


def noise(n: int) -> list[float]:
    if n not in _NOISE:
        rnd = random.Random(7)
        _NOISE[n] = [rnd.gauss(0.0, 0.002) for _ in range(n)]
    return list(_NOISE[n])


def add_tone(buf: list[float], start: datetime.datetime, origin: datetime.datetime, seconds: float,
             hz: float, amp: float, rate: int = RATE) -> None:
    a = int(round((start - origin).total_seconds() * rate))
    for i in range(max(0, a), min(len(buf), a + int(seconds * rate))):
        buf[i] += amp * math.sin(2 * math.pi * hz * (i - a) / rate)


def write_recording(path: Path, buf: list[float], origin: datetime.datetime, rate: int = RATE) -> None:
    pcm = array("h", (max(-32767, min(32767, int(round(v * 32767)))) for v in buf))
    if sys.byteorder != "little":
        pcm.byteswap()
    audio.write_wav(path, pcm.tobytes(), rate, 1, 16)
    end = origin + datetime.timedelta(seconds=len(buf) / rate)
    common.write_json(audio.sidecar_path(path), {
        "pid": PID, "sample_rate": rate, "channels": 1, "bits": 16,
        "first_sample_utc": common.format_utc(origin),
        "anchors": [[0, common.format_utc(origin)], [len(buf), common.format_utc(end)]],
        "gaps": [], "end_utc": common.format_utc(end), "end_reason": "stop-file"})


def cry(buf: list[float], start: datetime.datetime, origin: datetime.datetime, rate: int = RATE) -> None:
    """Six 0.1 s tones alternating 450 and 1600 Hz, 0.6 s: the stand-in death
    cry. (A plain sweep matches a plain 800 Hz burst at 0.73, too close to
    tell apart under the 1.5x rule; a cry with its own band pattern is the
    fair stand-in.)"""
    for i in range(6):
        add_tone(buf, start + datetime.timedelta(seconds=0.1 * i), origin, 0.1,
                 450.0 if i % 2 == 0 else 1600.0, 0.3, rate)


def bark(buf: list[float], start: datetime.datetime, origin: datetime.datetime, rate: int = RATE) -> None:
    """Three 1 kHz pips and a 500 Hz tail, 0.8 s: the stand-in bark."""
    for k in range(3):
        add_tone(buf, start + datetime.timedelta(seconds=0.2 * k), origin, 0.1, 1000.0, 0.3, rate)
    add_tone(buf, start + datetime.timedelta(seconds=0.6), origin, 0.2, 500.0, 0.3, rate)


def build(root: Path, *, controls=True, burst_at_kill=False, read_at_kill=False, bar_vanishes=False,
          text_at_kill=False, busy=False, focus_gap=False, warm=False, marker_tone=False,
          town_tone=True, refs=False) -> Path:
    """A synthetic live folder in `root`; returns it."""
    live = root / "live"
    for sub in common.SUBFOLDERS:
        (live / sub).mkdir(parents=True, exist_ok=True)

    # Client.txt: the older session, then this one.
    before = (FIXTURES / "before.txt").read_text(encoding="utf-8").splitlines()
    lines = (FIXTURES / "client.txt").read_text(encoding="utf-8").splitlines()
    if focus_gap:
        lines += [log_line(z("10:27:44.000"), "[WINDOW] Lost focus"),
                  log_line(z("10:27:47.000"), "[WINDOW] Gained focus")]
        lines.sort(key=lambda l: l[:19])
    client = root / "Client.txt"
    head = "".join(l + "\r\n" for l in before).encode("utf-8")
    client.write_bytes(head + "".join(l + "\r\n" for l in lines).encode("utf-8"))
    common.write_json(live / common.SESSION_JSON, {
        "created_utc": "2026-10-26T10:23:00.000Z", "client_txt": str(client), "client_txt_offset": len(head),
        "ggpk": str(root / "Content.ggpk"), "ggpk_size": 1000, "ggpk_mtime": "2026-10-20T08:00:00.000Z"})
    shift = 100_000_000 if warm else 0
    for when in ("10:23:00.000", "10:24:15.000"):
        tick = TICK_AT_ORIGIN + round((z(when) - ORIGIN).total_seconds() * 1000) + shift
        common.append_jsonl(live / common.CLOCK_JSONL, {"utc": common.format_utc(z(when)), "tick_ms": tick})
    (live / common.MARKERS_TXT).write_text(
        "# registered Ctrl+Alt+M (MOD_NOREPEAT) at 2026-10-26T10:24:20.000Z\n# pid 999\n"
        "# tone 1760 Hz 150 ms\n1 %s\n2 %s\n" % (common.format_utc(TEST_PRESS), common.format_utc(M)),
        encoding="utf-8")

    # The GGPK trees, before and after.
    tree = {"format": ggpk.TREE_FORMAT, "version": 3, "root": 100, "free_head": 0, "ggpk_size": 1000,
            "records": [{"kind": "file", "name": BANK, "offset": 200, "size": 300}]}
    ggpk.write_tree(live / "ggpk" / "tree-before.json", tree)
    ggpk.write_tree(live / "ggpk" / "tree-after.json", tree)

    # I1: the rehearsal and the session's mapped reads.
    shutil.copy(TRACERPT / "rehearsal-pass.xml", live / "i1" / "rehearsal" / "rehearsal.xml")
    common.write_json(live / "i1" / "rehearsal" / "probe.json", PROBE)
    reads = []

    def read(when, name, kind="bundle", block=0):
        reads.append({"utc": common.format_utc(when), "pid": PID, "pid_source": "header", "tid": 7,
                      "op": "read", "file": "\\Device\\HarddiskVolume3\\Games\\Content.ggpk",
                      "offset": 1000 + len(reads), "size": 4096, "flags": 0,
                      "records": [{"name": name, "kind": kind, "rel_offset": 0,
                                   "blocks": [] if kind == "file" else [block]}]})

    t = WAV_START
    while t < WAV_END:
        read(t + datetime.timedelta(seconds=0.05), TEXTURES, block=len(reads) % 40)
        t += datetime.timedelta(seconds=3)
    read(G1_2 + datetime.timedelta(seconds=3), BANK, kind="file")
    read(TOWN + datetime.timedelta(seconds=0.3), NPC_TOWN)
    if controls:
        for i, l in enumerate(HER):
            read(l + datetime.timedelta(seconds=0.3), MONSTERS, block=i)
    read(UNA + datetime.timedelta(seconds=0.4), NPC_UNA)
    if read_at_kill:
        read(M + datetime.timedelta(seconds=0.4), MONSTERS, block=9)
    reads.sort(key=lambda r: r["utc"])
    with open(live / "i1" / "mapped.jsonl", "w", encoding="utf-8", newline="\n") as f:
        for r in reads:
            f.write(json.dumps(r) + "\n")

    # I2: the rehearsal selftest and the session's recording.
    st0 = z("10:22:00.000")
    sbuf = noise(6 * RATE)
    add_tone(sbuf, st0 + datetime.timedelta(seconds=1.55), st0, 3.0, audio.TONE_A_HZ, 0.1)
    write_recording(live / "i2" / "rehearsal" / "selftest-20261026T102200Z.wav", sbuf, st0)
    common.append_jsonl(live / "i2" / "rehearsal" / "selftest-20261026T102200Z-a.jsonl",
                        {"pid": 1, "start_utc": common.format_utc(st0 + datetime.timedelta(seconds=1.5))})

    buf = noise(int(round((WAV_END - WAV_START).total_seconds() * RATE)))
    burst = lambda when: add_tone(buf, when, WAV_START, 0.4, 800.0, 0.3)
    if town_tone:
        burst(TOWN + datetime.timedelta(seconds=0.2))
    if controls:
        for l in HER:
            burst(l + datetime.timedelta(seconds=0.2))
    burst(UNA + datetime.timedelta(seconds=0.2))
    if burst_at_kill:
        burst(M + datetime.timedelta(seconds=0.3))
    if busy:
        tiles = analyse.Session(live, PID, tz=TZ).tiles()
        assert len(tiles) % 2 == 0, "the busy fixture needs an even number of base windows"
        for a, _ in tiles[0::2]:
            burst(a + datetime.timedelta(seconds=0.5))
    if marker_tone:
        add_tone(buf, TEST_PRESS, WAV_START, 0.15, 1760.0, 0.2)
    if refs:
        # The references, decoded to WAV, and the same sounds in the recording.
        cry(buf, M + datetime.timedelta(seconds=0.5), WAV_START)
        bark(buf, HER[0] + datetime.timedelta(seconds=0.7), WAV_START)
        # Each reference is the sound alone, as a decoded .ogg is.
        for name, make, seconds in (("carrioncronedeath.wav", cry, 0.6), ("carrioncronesleep.wav", bark, 0.8)):
            rbuf = [0.0] * int(seconds * RATE)
            make(rbuf, WAV_START, WAV_START)
            write_recording(live / "ref" / name, rbuf, WAV_START)
        common.write_json(live / "ref" / "refs.json", {"carrioncronesleep.wav": HER_TEXT[0]})
    write_recording(live / "i2" / "game.wav", buf, WAV_START)

    # I3: the rehearsal's frames and the session's.
    def frames_into(folder: Path, bands, start, end, texts):
        idx, ocr = [], []
        t = start
        while t <= end:
            stamp = t.strftime("%Y%m%dT%H%M%S.") + "%03dZ" % (t.microsecond // 1000)
            for band in bands:
                rel = "frames/%s/%s.jpg" % (band, stamp)
                idx.append({"utc": common.format_utc(t), "band": band, "path": rel, "rect": [0, 0, 10, 10],
                            "client": [0, 0, 100, 100], "foreground": True, "luma_mean": 60.0, "luma_std": 30.0})
                said = texts(band, t)
                if said:
                    ocr.append({"path": rel, "lines": [{"text": s, "box": [10, 20 * i, 300, 18]}
                                                       for i, s in enumerate(said)]})
            t += datetime.timedelta(seconds=0.5)
        folder.mkdir(parents=True, exist_ok=True)
        with open(folder / screen.FRAMES_JSONL, "w", encoding="utf-8", newline="\n") as f:
            f.writelines(json.dumps(r) + "\n" for r in idx)
        with open(folder / screen.OCR_JSONL, "w", encoding="utf-8", newline="\n") as f:
            f.writelines(json.dumps(r) + "\n" for r in ocr)

    frames_into(live / "i3" / "rehearsal", ("bottom",), z("10:22:30.000"), z("10:22:32.000"),
                lambda band, t: list(screen.PROBE_LINES))

    shown = [(TOWN, "Renly: Need something sharpened?"), (UNA, "Una: What happened here...?")]
    if controls:
        shown += [(l, "Beira of the Rotten Pack: " + text) for l, text in zip(HER, HER_TEXT)]
    bar_until = M if bar_vanishes else FRAMES_TO

    def texts(band, t):
        if band == "top":
            ok = controls and z("10:26:50.000") <= t <= bar_until
            return ["Beira of the Rotten Pack"] if ok else []
        out = [s for when, s in shown
               if when + datetime.timedelta(seconds=0.3) <= t <= when + datetime.timedelta(seconds=6)]
        if text_at_kill and M + datetime.timedelta(seconds=1) <= t <= M + datetime.timedelta(seconds=4):
            out.append(NEW_LINE)
        return out

    frames_into(live / "i3", ("top", "bottom"), FRAMES_FROM, FRAMES_TO, texts)
    return live


class Case(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = Path(self._tmp.name)
        self.assertIsNone(common.git_tree_of(self.tmp), "the temp folder sits inside a git checkout")

    def tearDown(self):
        self._tmp.cleanup()

    def analyse(self, live, **kw):
        out = io.StringIO()
        kw.setdefault("confirmed", "yes")
        err = io.StringIO()
        with contextlib.redirect_stderr(err):
            code, shown = analyse.analyse(live, PID, tz=TZ, ocr_runner=no_ocr, out=out, **kw)
        self.stderr = err.getvalue()
        self.printed = out.getvalue()
        self.code = code
        return {r["name"]: r for r in shown}

    def verdict(self, results, name):
        return results[name]["verdict"]


# --------------------------------------------------------------------------
# Constants and pure helpers

class Constants(unittest.TestCase):
    def test_the_check_rules_are_pinned(self):
        self.assertEqual((analyse.LINE_BEFORE_S, analyse.LINE_AFTER_S), (0.5, 1.5))
        self.assertEqual((analyse.TARGET_BEFORE_S, analyse.TARGET_AFTER_S), (3.0, 2.0))
        self.assertEqual(analyse.FIGHT_END_BEFORE_M_S, 5.0)
        self.assertEqual((analyse.TILE_S, analyse.TILE_EXCLUDE_BEFORE_S, analyse.TILE_EXCLUDE_AFTER_S),
                         (2.0, 1.0, 3.0))
        self.assertEqual(analyse.BASE_RATE_MAX, 0.5)
        self.assertEqual((analyse.I1_QUIET_BEFORE_S, analyse.I1_QUIET_AFTER_S), (1.5, 0.5))
        self.assertEqual(analyse.I1_CONTROL_SHARE, 0.5)
        self.assertEqual((analyse.BURST_DB, analyse.BURST_HISTORY_S, analyse.BURST_MIN_S), (6.0, 2.0, 0.150))
        self.assertEqual((analyse.I2_CONTROL_SHARE, analyse.I2_CONTROL_BASE_FACTOR), (2.0 / 3.0, 2.0))
        self.assertEqual((analyse.ISOLATION_HZ, analyse.ISOLATION_SIDES_HZ, analyse.ISOLATION_DB,
                          analyse.ISOLATION_S), (1760.0, (1660.0, 1860.0), 15.0, 0.4))
        self.assertEqual((analyse.OCR_SCORE, analyse.OCR_LINE_S, analyse.I3_CONTROL_SHARE), (0.8, 4.0, 2.0 / 3.0))
        self.assertEqual((analyse.BAR_SHARE, analyse.BAR_BEFORE_S, analyse.BAR_GONE_FROM_S, analyse.BAR_GONE_TO_S),
                         (0.5, 10.0, 1.0, 10.0))
        self.assertEqual((analyse.TEXT_BEFORE_S, analyse.TEXT_AFTER_S, analyse.TEXT_ABSENT_FROM_S), (3.0, 4.0, 13.0))
        self.assertEqual((analyse.TEMPLATE_FRAME_S, len(analyse.TEMPLATE_BANDS), analyse.TEMPLATE_NEAR_S,
                          analyse.TEMPLATE_FAR_S, analyse.TEMPLATE_RATIO, analyse.TEMPLATE_SHARE),
                         (0.01, 4, 1.5, 10.0, 1.5, 2.0 / 3.0))
        self.assertEqual((analyse.FIRST_USE_S, analyse.AREA_LOAD_BEFORE_S, analyse.AREA_LOAD_AFTER_S), (5.0, 2.0, 20.0))
        self.assertEqual(analyse.REWARD_SPLIT_S, 55.0)
        self.assertEqual(analyse.CLIP_S, 5.0)
        self.assertEqual(audio.ENVELOPE_FRAME_S, 0.05)

    def test_the_checks_are_in_the_procedures_order(self):
        self.assertEqual(analyse.CHECKS, (
            "control", "marker", "ggpk-stable", "i1-rehearsal", "i1-rehearsal-mapped", "i2-rehearsal",
            "i3-rehearsal", "i2-town", "i2-isolation", "i3-town", "i1-control", "i1-control-una",
            "i1-read-timing", "i1-target", "i2-control", "i2-control-una", "i2-target", "i2-listened",
            "i2-template-control", "i2-template", "i3-control", "i3-control-una", "i3-bar-control",
            "i3-text-target", "i3-bar-target", "reward-split"))


class Bursts(unittest.TestCase):
    def test_a_burst_needs_6_db_held_150_ms(self):
        quiet = [-60.0] * 50
        held = quiet + [-50.0] * 3 + quiet          # 150 ms at +10 dB: a burst
        short = quiet + [-50.0] * 2 + quiet         # 100 ms: not one
        weak = quiet + [-55.0] * 10 + quiet         # +5 dB: not one
        self.assertEqual(analyse.burst_onsets(held, 0.05), [50])
        self.assertEqual(analyse.burst_onsets(short, 0.05), [])
        self.assertEqual(analyse.burst_onsets(weak, 0.05), [])

    def test_the_median_is_of_the_preceding_2_s(self):
        loud = [-30.0] * 50 + [-60.0] * 40 + [-50.0] * 5
        self.assertEqual(analyse.burst_onsets(loud, 0.05), [90])
        # A burst with less than 2 s of history is not judged.
        self.assertEqual(analyse.burst_onsets([-60.0] * 10 + [-30.0] * 5, 0.05), [])

    def test_recorder_zeros_block_an_onset(self):
        held = [-60.0] * 50 + [-50.0] * 3 + [-60.0] * 10
        self.assertEqual(analyse.burst_onsets(held, 0.05, blocked=lambda a, b: True), [])


class Tiles(unittest.TestCase):
    def test_windows_avoid_the_lines_and_end_by_the_fight_end(self):
        t0 = z("10:00:00.000")
        at = lambda s: t0 + datetime.timedelta(seconds=s)
        tiles = analyse.base_tiles(at(0), at(11), [(at(3), at(7))])
        self.assertEqual([((a - t0).total_seconds(), (b - t0).total_seconds()) for a, b in tiles],
                         [(0, 2), (8, 10)])
        self.assertEqual(analyse.share_of_tiles(tiles, [at(1)]), 0.5)
        self.assertIsNone(analyse.share_of_tiles([], [at(1)]))


class Template(unittest.TestCase):
    def test_a_planted_pattern_peaks_where_it_was_planted(self):
        rnd = random.Random(3)
        pattern = [[rnd.uniform(-60, -20) for _ in range(30)] for _ in range(4)]
        series = [[-70.0 + rnd.uniform(-1, 1) for _ in range(3000)] for _ in range(4)]
        for b in range(4):
            series[b][1500:1530] = pattern[b]
        ncc = analyse.ncc_series(series, pattern)
        self.assertEqual(max(range(len(ncc)), key=ncc.__getitem__), 1500)
        t0 = z("10:00:00.000")
        lag = lambda k: t0 + datetime.timedelta(seconds=k * 0.01)
        near, far, at = analyse.template_peak(ncc, lag, [t0 + datetime.timedelta(seconds=15.0)])
        self.assertEqual(at, 1500)
        self.assertTrue(analyse.is_hit(near, far))
        # Expected 13 s too early: the planted peak is outside, beating the near one.
        near, far, _ = analyse.template_peak(ncc, lag, [t0 + datetime.timedelta(seconds=2.0)])
        self.assertFalse(analyse.is_hit(near, far))


# --------------------------------------------------------------------------
# clientlog

class ClientLogReading(Case):
    def setUp(self):
        super().setUp()
        self.live = build(self.tmp)
        s = common.read_json(self.live / common.SESSION_JSON)
        self.path, self.offset = Path(s["client_txt"]), s["client_txt_offset"]

    def test_lines_get_their_utc_from_the_counter(self):
        log = clientlog.read_log(self.path, self.offset, common.load_anchors(self.live), TZ)
        her = log.npc(clientlog.BEIRA, "G1_2")
        self.assertEqual([l.utc for l in her], HER)
        self.assertEqual({l.source for l in her}, {"counter"})
        self.assertEqual(log.sources["wall"], 0)

    def test_without_anchors_lines_fall_back_to_the_wall_clock(self):
        log = clientlog.read_log(self.path, self.offset, [], TZ)
        first = log.npc(clientlog.BEIRA)[0]
        self.assertEqual((first.utc, first.source), (z("10:27:04.000"), "wall"))

    def test_kinds_areas_and_pids(self):
        log = clientlog.read_log(self.path, self.offset, common.load_anchors(self.live), TZ)
        self.assertEqual(len(log.openings()), 1)
        self.assertEqual([a.text for a in log.areas()], ["G1_town", "G1_2", "G1_4"])
        self.assertEqual(log.pids, {PID: 28})
        self.assertEqual([l.speaker for l in log.npc(area="G1_town")], ["Renly"])
        self.assertEqual([l.text for l in log.npc(clientlog.UNA, "G1_4")],
                         ["What happened here...?", "I'll wait here. The old magicks will protect me."])
        self.assertEqual([(l.utc, l.speaker) for l in log.rewards()], [(REWARD, clientlog.PLAYER_MASK)])
        self.assertEqual(log.focus_losses(), [(z("10:24:40.000"), z("10:24:42.000"))])

    def test_chat_and_names_never_come_out(self):
        with open(self.path, "ab") as f:
            f.write((log_line(z("10:28:30.000"), ": TestMule has been slain.") + "\r\n").encode("utf-8"))
        log = clientlog.read_log(self.path, self.offset, common.load_anchors(self.live), TZ)
        self.assertEqual([(l.speaker, l.area) for l in log.deaths()], [(clientlog.PLAYER_MASK, "G1_4")])
        dumped = repr(log.lines)
        for secret in ("TestMule", "Wanderer", "anyone selling", "hello there"):
            self.assertNotIn(secret, dumped)
        self.assertEqual((log.skipped["chat"], log.skipped["unlisted-speaker"]), (1, 1))
        # The one use of the other lines: yes or no.
        self.assertTrue(log.repeats_logged("Wanderer: anyone selling a bow", z("10:24:00"), z("10:25:00"),
                                           screen.fuzzy_score, 0.8))
        self.assertFalse(log.repeats_logged("something nobody said", z("10:24:00"), z("10:25:00"),
                                            screen.fuzzy_score, 0.8))

    def test_the_last_opening_before_the_offset(self):
        when = clientlog.last_opening_before(self.path, self.offset, TZ)
        self.assertEqual(when, datetime.datetime(2026, 10, 24, 22, 10, 0, tzinfo=UTC))
        self.assertEqual(clientlog.last_opening_before(self.path, self.offset, TZ, chunk=37), when)
        self.assertIsNone(clientlog.last_opening_before(self.path, 10, TZ))


# --------------------------------------------------------------------------
# The analysis: baseline, targets and negative controls

TARGETS = ("i1-target", "i2-target", "i3-text-target", "i3-bar-target")
CONTROLS = ("control", "marker", "ggpk-stable", "i1-rehearsal", "i1-rehearsal-mapped", "i2-rehearsal",
            "i3-rehearsal", "i2-town", "i2-isolation", "i3-town", "i1-control", "i1-control-una",
            "i1-read-timing", "i2-control", "i2-control-una", "i3-control", "i3-control-una",
            "i3-bar-control", "reward-split")


class Baseline(Case):
    """Live 4's session: nothing at the kill, every control present."""

    @classmethod
    def setUpClass(cls):
        cls._ctmp = tempfile.TemporaryDirectory()
        cls.live = build(Path(cls._ctmp.name))

    @classmethod
    def tearDownClass(cls):
        cls._ctmp.cleanup()

    def setUp(self):
        super().setUp()
        self.results = self.analyse(self.live)

    def test_every_control_passes(self):
        for name in CONTROLS:
            self.assertEqual(self.verdict(self.results, name), "pass",
                             "%s: %s" % (name, self.results[name]["observed"]))

    def test_every_target_is_not_observed(self):
        for name in TARGETS:
            self.assertEqual(self.verdict(self.results, name).split()[0], "not-observed",
                             "%s: %s" % (name, self.results[name]["observed"]))
        self.assertEqual(self.verdict(self.results, "i1-target"), analyse.NOT_OBSERVED_I1)
        self.assertIn("uptime at prep 91380000 ms", self.results["i1-target"]["observed"])
        self.assertIn(": cold", self.results["i1-target"]["observed"])

    def test_without_references_or_answers_those_checks_are_not_run(self):
        self.assertEqual(self.verdict(self.results, "i2-template-control"), "not-run (instrument: no reference audio in ref\\)")
        self.assertEqual(self.verdict(self.results, "i2-template"), "not-run (instrument: no reference audio in ref\\)")
        self.assertTrue(self.verdict(self.results, "i2-listened").startswith("not-run (person: no answer yet"))

    def test_lines_are_printed_in_order_in_the_capture_form(self):
        lines = [l for l in self.printed.splitlines() if l.startswith("- ")]
        self.assertEqual([l[2:].split(" | ")[0] for l in lines], list(analyse.CHECKS))
        for l in lines:
            self.assertRegex(l, r"^- [a-z0-9-]+ \| expected: [^|]+ \| observed: [^|]+ \| "
                                r"(pass|fail|not-observed|not-run)")
        self.assertEqual(self.code, 0, self.stderr)

    def test_read_timing_and_the_region(self):
        self.assertIn("bank: area-load; dialogue: playback; town lines read: 1 of 1",
                      self.results["i1-read-timing"]["observed"])
        self.assertIn("9 of 9 lines", self.results["i1-control"]["observed"])
        self.assertEqual(self.results["i1-control"]["details"]["region"], sorted([BANK, MONSTERS]))

    def test_nothing_private_is_printed_or_written(self):
        written = (self.live / common.ANALYSIS_JSON).read_text(encoding="utf-8")
        for secret in ("TestMule", "Wanderer", "anyone selling", "hello there"):
            self.assertNotIn(secret, self.printed)
            self.assertNotIn(secret, written)

    def test_writes_the_analysis_and_the_clips(self):
        data = common.read_json(self.live / common.ANALYSIS_JSON)
        self.assertEqual([c["name"] for c in data["checks"]], list(analyse.CHECKS))
        for c in data["checks"]:
            self.assertEqual(set(c) - {"error"}, {"name", "expected", "observed", "verdict", "details"})
        for clip, at in (("kill-clip.wav", M), ("bark-clip.wav", HER[0])):
            side = common.read_json(audio.sidecar_path(self.live / "i2" / clip))
            self.assertEqual(side["at_utc"], common.format_utc(at))
            self.assertAlmostEqual(side["frames"] / RATE, 10.0, places=2)


class TargetsPass(Case):
    def test_a_burst_at_the_kill(self):
        r = self.analyse(build(self.tmp, burst_at_kill=True))
        self.assertEqual(self.verdict(r, "i2-target"), "pass", r["i2-target"]["observed"])
        self.assertIn("onset +0.30 s", r["i2-target"]["observed"])

    def test_a_dialogue_bundle_read_at_the_kill(self):
        r = self.analyse(build(self.tmp, read_at_kill=True))
        self.assertEqual(self.verdict(r, "i1-target"), "pass", r["i1-target"]["observed"])
        self.assertIn("read +0.40 s of %s, block(s) [9]" % MONSTERS, r["i1-target"]["observed"])
        self.assertIn("base rate 0.00", r["i1-target"]["observed"])

    def test_the_bar_name_vanishes_at_the_kill(self):
        r = self.analyse(build(self.tmp, bar_vanishes=True))
        self.assertEqual(self.verdict(r, "i3-bar-target"), "pass", r["i3-bar-target"]["observed"])
        self.assertIn("last frame with the name +0.00 s", r["i3-bar-target"]["observed"])

    def test_the_death_cry_template_at_the_kill(self):
        r = self.analyse(build(self.tmp, refs=True), only=["i2-template-control", "i2-template"])
        self.assertEqual(self.verdict(r, "i2-template-control"), "pass", r["i2-template-control"]["observed"])
        self.assertIn("1 of 1 references hit; carrioncronesleep.wav", r["i2-template-control"]["observed"])
        self.assertEqual(self.verdict(r, "i2-template"), "pass", r["i2-template"]["observed"])
        self.assertRegex(r["i2-template"]["observed"],
                         r"carrioncronedeath\.wav: top peak (0\.9\d|1\.00) at \+0\.[45]\d s, best outside "
                         r"M \+/- 10 s 0\.[0-5]\d")

    def test_a_new_line_at_the_kill_keeps_its_text_out_of_the_capture(self):
        live = build(self.tmp, text_at_kill=True)
        r = self.analyse(live)
        self.assertEqual(self.verdict(r, "i3-text-target"), "pass", r["i3-text-target"]["observed"])
        self.assertIn("1 new line(s): box [", r["i3-text-target"]["observed"])
        self.assertNotIn(NEW_LINE, self.printed)
        self.assertIn(NEW_LINE, (live / common.ANALYSIS_JSON).read_text(encoding="utf-8"))


class NegativeControls(Case):
    def test_failed_controls_make_the_targets_not_run(self):
        r = self.analyse(build(self.tmp, controls=False))
        for name in ("i1-control", "i2-control", "i3-control", "i3-bar-control"):
            self.assertEqual(self.verdict(r, name), "fail", name)
        self.assertEqual(self.verdict(r, "i1-target"), "not-run (instrument: i1-control fail)")
        self.assertEqual(self.verdict(r, "i2-target"), "not-run (instrument: i2-control fail)")
        self.assertEqual(self.verdict(r, "i3-text-target"), "not-run (instrument: i3-control fail)")
        self.assertEqual(self.verdict(r, "i3-bar-target"), "not-run (instrument: i3-bar-control fail)")

    def test_busy_fight_audio_leaves_a_burst_at_the_kill_not_observed(self):
        r = self.analyse(build(self.tmp, burst_at_kill=True, busy=True))
        self.assertEqual(self.verdict(r, "i2-control"), "pass", r["i2-control"]["observed"])
        self.assertIn("base rate 0.50", r["i2-target"]["observed"])
        self.assertIn("onset +0.30 s", r["i2-target"]["observed"])
        self.assertEqual(self.verdict(r, "i2-target"), "not-observed")

    def test_a_lost_focus_drops_that_line_from_the_count(self):
        # Focus is lost 10:27:44 to 10:27:47: "Rotten!" (45.3) falls in it, and
        # i3's window [L, L + 4] of the "Be still!" at 41.5 reaches into it too.
        r = self.analyse(build(self.tmp, focus_gap=True))
        for name, kept in (("i1-control", "8 of 8 lines (1 left out"), ("i2-control", "8 of 8 lines (1 left out"),
                           ("i3-control", "7 of 7 lines (2 left out")):
            self.assertIn(kept + ": focus lost)", r[name]["observed"], name)
            self.assertEqual(self.verdict(r, name), "pass", name)

    def test_a_warm_cache_makes_i1_target_not_run(self):
        r = self.analyse(build(self.tmp, warm=True))
        self.assertEqual(self.verdict(r, "i1-target"), "not-run (instrument: warm cache)")
        self.assertIn("uptime at prep 191380000 ms", r["i1-target"]["observed"])
        self.assertEqual(self.verdict(r, "control"), "pass")

    def test_the_marker_tone_in_the_recording_fails_isolation(self):
        r = self.analyse(build(self.tmp, marker_tone=True), only=["i2-isolation"])
        self.assertEqual(self.verdict(r, "i2-isolation"), "fail", r["i2-isolation"]["observed"])

    def test_no_town_burst_fails_i2_town(self):
        r = self.analyse(build(self.tmp, town_tone=False), only=["i2-town"])
        self.assertEqual(self.verdict(r, "i2-town"), "fail")

    def test_the_marker_needs_the_persons_confirmation(self):
        live = build(self.tmp)
        self.assertEqual(self.verdict(self.analyse(live, only=["marker"], confirmed=None), "marker"), "fail")
        self.assertEqual(self.verdict(self.analyse(live, only=["marker"], confirmed="no"), "marker"), "fail")
        self.assertEqual(self.verdict(self.analyse(live, only=["marker"]), "marker"), "pass")

    def test_a_changed_ggpk_names_the_record(self):
        live = build(self.tmp)
        tree = ggpk.load_tree(live / "ggpk" / "tree-after.json")
        tree["records"][0]["offset"] = 900
        ggpk.write_tree(live / "ggpk" / "tree-after.json", tree)
        r = self.analyse(live, only=["ggpk-stable"])
        self.assertEqual(self.verdict(r, "ggpk-stable"), "fail")
        self.assertIn(BANK, r["ggpk-stable"]["observed"])

    def test_the_listening_answers(self):
        live = build(self.tmp)
        for answer, verdict in (("yes,yes", "pass"), ("no,yes", "pass"), ("yes,no", "not-observed"),
                                ("yes,unsure", "not-observed"), ("no,no", "not-run (the listening control failed)"),
                                ("unsure,no", "not-run (the listening control failed)")):
            r = self.analyse(live, only=["i2-listened"], listened=answer)
            self.assertEqual(self.verdict(r, "i2-listened"), verdict, answer)


class MissingInputs(Case):
    def test_an_almost_empty_folder_is_not_a_crash(self):
        live = build(self.tmp)
        for path in ("markers.txt", "i1/mapped.jsonl", "i2/game.wav", "i3/frames.jsonl",
                     "ggpk/tree-after.json", "i1/rehearsal/probe.json"):
            (live / path).unlink()
        r = self.analyse(live)
        self.assertEqual(self.code, 0, self.stderr)
        self.assertEqual(len(r), len(analyse.CHECKS))
        self.assertEqual(self.verdict(r, "control"), "pass")
        for name in ("marker", "ggpk-stable", "i1-rehearsal", "i1-control", "i1-target", "i2-town",
                     "i2-control", "i2-target", "i3-town", "i3-control", "i3-bar-target"):
            self.assertTrue(self.verdict(r, name).startswith("not-run (instrument: "), name)
        self.assertIn("clips: not written", self.printed)

    def test_no_session_json_makes_everything_not_run(self):
        live = self.tmp / "live"
        live.mkdir()
        r = self.analyse(live)
        self.assertEqual(self.code, 0, self.stderr)
        self.assertTrue(self.verdict(r, "control").startswith("not-run (instrument: no session.json"))

    def test_only_control_in_town(self):
        live = build(self.tmp)
        (live / "i2" / "game.wav").unlink()
        r = self.analyse(live, only=["control"])
        self.assertEqual(list(r), ["control"])
        self.assertEqual([l[2:].split(" | ")[0] for l in self.printed.splitlines() if l.startswith("- ")],
                         ["control"])
        self.assertEqual(self.verdict(r, "control"), "pass")

    def test_the_town_step_merges_into_analysis_json(self):
        live = build(self.tmp)
        self.analyse(live, only=["control"])
        r = self.analyse(live, only=["i2-town,i2-isolation", "i3-town"])
        self.assertEqual(list(r), ["i2-town", "i2-isolation", "i3-town"])
        names = [c["name"] for c in common.read_json(live / common.ANALYSIS_JSON)["checks"]]
        self.assertEqual(names, ["control", "i2-town", "i2-isolation", "i3-town"])
        self.assertFalse((live / "i2" / "kill-clip.wav").exists())

    def test_an_internal_error_names_the_check_and_exits_1(self):
        live = build(self.tmp)

        def broken(s):
            raise ZeroDivisionError("boom")

        old = analyse.PLAIN["reward-split"]
        analyse.PLAIN["reward-split"] = broken
        try:
            r = self.analyse(live, only=["control", "reward-split"])
        finally:
            analyse.PLAIN["reward-split"] = old
        self.assertEqual(self.code, 1)
        self.assertIn("check reward-split failed: ZeroDivisionError: boom", self.stderr)
        self.assertEqual(self.verdict(r, "control"), "pass")

    def test_an_unknown_check_is_refused(self):
        with self.assertRaises(common.ToolError):
            analyse.parse_only(["i9-target"])

    def test_refuses_a_live_folder_inside_a_git_checkout(self):
        with self.assertRaises(common.ToolError):
            analyse.analyse(ROOT / "tests", PID, out=io.StringIO())


# --------------------------------------------------------------------------
# run: start and stop

FAKE = (
    "import os, sys, time\n"
    "stop, mode = sys.argv[1], sys.argv[2]\n"
    "if mode == 'fail':\n"
    "    print('error the device is gone', flush=True); sys.exit(1)\n"
    "print('ready 2026-10-26T10:00:00.000Z', flush=True)\n"
    "while not os.path.exists(stop):\n"
    "    time.sleep(0.05)\n"
    "print('stopped stop-file', flush=True)\n"
)


class Run(Case):
    def setUp(self):
        super().setUp()
        self.live = self.tmp / "live"
        self.live.mkdir()
        (self.live / "i2").mkdir()
        (self.live / "i3").mkdir()
        common.write_json(self.live / common.SESSION_JSON, {"client_txt_offset": 0})

    def fake(self, modes):
        return [run.Instrument(name, [sys.executable, "-c", FAKE, str(common.stop_file(self.live, stop)), mode],
                               self.live / ("%s.log" % name), "ready ", stop)
                for (name, stop), mode in zip((("recorder", "i2"), ("capture", "i3"), ("marker", "marker")), modes)]

    def test_the_commands_name_the_pid_and_the_stop_files(self):
        tools = {t.name: t for t in run.instruments(self.live, 1234)}
        rec = tools["recorder"].command
        self.assertIn("-TargetPid", rec)
        self.assertEqual(rec[rec.index("-TargetPid") + 1], "1234")
        self.assertEqual(rec[rec.index("-StopFile") + 1], str(self.live / "stop-i2"))
        self.assertEqual(rec[rec.index("-Out") + 1], str(self.live / "i2" / "game.wav"))
        cap = tools["capture"].command
        self.assertEqual(cap[cap.index("-OutDir") + 1], str(self.live / "i3"))
        self.assertEqual(cap[cap.index("-Hz") + 1], "2")
        mark = tools["marker"].command
        self.assertEqual(mark[mark.index("-Out") + 1], str(self.live / "markers.txt"))
        self.assertEqual([t.ready for t in tools.values()], ["ready ", "capture: ready", "# registered"])

    def test_start_detaches_waits_for_ready_and_stop_ends_them(self):
        out = io.StringIO()
        code = run.start(self.live, 1234, timeout=60, tools=self.fake(["ok", "ok", "ok"]), out=out)
        self.assertEqual(code, 0, out.getvalue())
        record = common.read_json(self.live / run.RUN_JSON)["instruments"]
        self.assertEqual(sorted(record), ["capture", "marker", "recorder"])
        for entry in record.values():
            self.assertTrue(run.alive(entry["pid"], entry["created"]))
            self.assertTrue(entry["ready"].startswith("ready "))
        self.assertEqual(out.getvalue().count(" ready after "), 3)
        with self.assertRaises(common.ToolError):   # started once already
            run.start(self.live, 1234, tools=self.fake(["ok", "ok", "ok"]), out=io.StringIO())

        out = io.StringIO()
        self.assertEqual(run.stop(self.live, timeout=60, out=out), 0, out.getvalue())
        for entry in record.values():
            self.assertFalse(run.alive(entry["pid"], entry["created"]))
        self.assertIn("recorder ended", out.getvalue())
        self.assertIn("stopped stop-file", out.getvalue())
        self.assertIn("no i1\\status.txt", out.getvalue())
        for name in ("i1", "i2", "i3", "marker"):
            self.assertTrue(common.stop_file(self.live, name).exists())

    def test_an_instrument_that_exits_is_reported_with_its_last_line(self):
        out = io.StringIO()
        code = run.start(self.live, 1234, timeout=60, tools=self.fake(["ok", "fail", "ok"]), out=out)
        self.assertEqual(code, 1)
        self.assertIn("capture NOT ready: it exited", out.getvalue())
        self.assertIn("error the device is gone", out.getvalue())
        self.assertEqual(run.stop(self.live, timeout=60, out=io.StringIO()), 0)

    def test_stop_waits_for_the_file_trace_controller(self):
        status = self.live / "i1" / "status.txt"
        status.parent.mkdir()
        status.write_text("2026-10-26T10:00:00.000Z started\n2026-10-26T10:00:05.000Z session-trace-running\n",
                          encoding="ascii")

        def controller():
            deadline = time.monotonic() + 30
            while not common.stop_file(self.live, "i1").exists() and time.monotonic() < deadline:
                time.sleep(0.05)
            time.sleep(0.3)
            with open(status, "a", encoding="ascii") as f:
                f.write("2026-10-26T10:01:00.000Z session-trace-stopped\n")

        th = threading.Thread(target=controller)
        th.start()
        out = io.StringIO()
        code = run.stop(self.live, timeout=30, out=out)
        th.join()
        self.assertEqual(code, 0, out.getvalue())
        self.assertIn("file trace: session-trace-stopped", out.getvalue())

    def test_controller_states(self):
        self.assertTrue(run.controller_ended(None))
        self.assertTrue(run.controller_ended("session-trace-stopped"))
        self.assertTrue(run.controller_ended("error wpr busy"))
        self.assertFalse(run.controller_ended("error session cap of 90 minutes reached; stopping the trace"))
        self.assertFalse(run.controller_ended("session-trace-running"))

    def test_start_refuses_a_stop_file_left_behind(self):
        common.stop_file(self.live, "i3").touch()
        with self.assertRaises(common.ToolError):
            run.start(self.live, 1234, tools=self.fake(["ok", "ok", "ok"]), out=io.StringIO())


if __name__ == "__main__":
    unittest.main()
