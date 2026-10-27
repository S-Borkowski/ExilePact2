"""tools/quest_tracker/outside/screentext: Live 8's stage stamps, config reader and analysis.

Nothing here touches a game, a window, the screen or the real client config.
Each test builds a synthetic live folder in a temporary directory outside the
repository:

- `Client.txt` is `tests/fixtures/quest_tracker/outside/screentext/before.txt`
  (an older session, before the offset) followed by `client.txt` (this
  session's start: a fresh character's entry to The Riverbank, `G1_1`, a
  level-up line carrying the character's name and two chat lines), then the
  lines each variant generates: the vendor greetings, the relaunched game's
  lines and, in some variants, a logged remark or dialogue line. The
  character, the chat and the chat's speakers are invented.
- `stages.jsonl` is written by the module's own `stage` command, and
  `config.jsonl` by its `config` command against synthetic `.ini` files in
  the temporary folder.
- The frames index points at JPEGs that do not exist. Their OCR results are
  already in `ocr.jsonl` (both bands, and `i3\\rehearsal`), and the OCR runner
  is a stand-in that reads nothing.

The session runs: `test`, `boss` (bar-up, an open and close pair for each of
six panels, away), `remark` (the kill and one remark press), `dialogue` (open,
close), `town-on` (3 greetings), `toggle-off`, `town-off` (3 greetings),
`town-on-again` (2), `relaunch-off` (2 greetings from the relaunched game) and
`end`.

The baseline has the option change nothing: greetings are logged in every
town stage, so `cfg-off-log` is `pass`. The target drops them from the log
while the option is off and keeps the subtitles, so `cfg-off-log` is
`not-observed` and every control still passes. The other variants are the
context's list: no `town-off` presses, a panel that hides the name, a remark
shown and not logged (and one logged), unpaired boss presses, `boss` stamped
twice, a `remark` run with the kill press alone, a `control` on `G1_1` alone,
and privacy, plus each route by which `cfg-off-relaunch-log`'s controls make
it not-run (the first pid given as the relaunch pid, no LOG FILE OPENING or
area entry from the relaunched client, no after-exit-off config read, or one
that reads the option on). `Newness` pins the shown-line checks' newness rule and the
unjudged window (no frame before it), and `DialogueLog` pins how `dlg-log`
tells quest dialogue from the greeting that opening the talk fires.
"""
import contextlib
import datetime
import hashlib
import io
import json
import re
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from tools.quest_tracker.outside import common, screen, screentext  # noqa: E402

FIXTURES = ROOT / "tests" / "fixtures" / "quest_tracker" / "outside" / "screentext"
UTC = datetime.timezone.utc
TZ = datetime.timezone(datetime.timedelta(hours=1))   # the fixture's local time
PID = 5151
PID2 = 5252
TICK_AT_ORIGIN = 50_000_000                            # the counter at 12:00:00Z
ORIGIN = datetime.datetime(2026, 10, 26, 12, 0, 0, tzinfo=UTC)
CHARACTER = "Ostrevane"
CHAT = "Zorblax: trading seven glowing turnips"
BOSS = "The Bloated Miller"
OPTION = "Tell me about the Miller"
LABEL = "Output all dialogue to chat"
PANELS = list(screentext.DEFAULT_PANELS)
CLIENT = [0, 0, 1920, 1080]
TOP_RECT = [480, 0, 1440, 151]


def z(hms: str) -> datetime.datetime:
    h, m, s = hms.split(":")
    sec = float(s)
    return datetime.datetime(2026, 10, 26, int(h), int(m), int(sec), round((sec % 1) * 1e6), tzinfo=UTC)


def td(s: float) -> datetime.timedelta:
    return datetime.timedelta(seconds=s)


def log_line(when, message, level="INFO", pid=PID) -> str:
    local = when.astimezone(TZ)
    tick = TICK_AT_ORIGIN + round((when - ORIGIN).total_seconds() * 1000)
    return "%s %d 3ef23348 [%s Client %d] %s" % (local.strftime("%Y/%m/%d %H:%M:%S"), tick, level, pid, message)


def no_ocr(list_path, out_path, scale):
    """The OCR stand-in: every frame it is asked about reads as empty."""
    with open(out_path, "a", encoding="utf-8") as f:
        for p in Path(list_path).read_text(encoding="utf-8").split("\n"):
            if p.strip():
                f.write(json.dumps({"path": p.strip(), "lines": []}) + "\n")


def ini_text(dialogue="true", subtitles="true") -> str:
    out = ["[GENERAL]"]
    if subtitles is not None:
        out.append("enable_subtitles=%s" % subtitles)
    out += ["language=en", "", "[UI]"]
    if dialogue is not None:
        out.append("output_all_dialogue_to_chat=%s" % dialogue)
    out += ["show_chat_timestamps=true", "", "[ACTION_KEYS]", "open_inventory_panel=73", ""]
    return "\n".join(out)


def write_ini(path: Path, dialogue="true", subtitles="true") -> Path:
    path.write_bytes(b"\xef\xbb\xbf" + ini_text(dialogue, subtitles).encode("utf-8"))
    return path


# The session's timeline.
TEST_STAMP, TEST_PRESS = z("12:01:00"), z("12:01:05")
BOSS_STAMP, BAR_UP = z("12:02:00"), z("12:02:10")
PANEL_OPEN = [z("12:02:20") + td(10 * k) for k in range(len(PANELS))]
PANEL_CLOSE = [o + td(4) for o in PANEL_OPEN]
AWAY = z("12:03:25")
REMARK_STAMP, KILL, REMARK_PRESS = z("12:04:00"), z("12:04:10"), z("12:04:30")
KILL_REMARK = ("The miller grinds no more", KILL + td(2), KILL + td(6))
LATER_REMARK = ("Flour and blood, how fitting", REMARK_PRESS - td(3), REMARK_PRESS + td(1))
DLG_STAMP, DLG_OPEN, DLG_CLOSE = z("12:05:00"), z("12:05:10"), z("12:05:40")
DLG_LINES = ["The Miller has been dealt with then", "You have my thanks, exile",
             "Take this for your trouble", OPTION]
TOWN_ON = (z("12:06:00"), [z("12:06:10"), z("12:06:20"), z("12:06:30")])
TOGGLE_OFF = z("12:07:00")
TOWN_OFF = (z("12:08:00"), [z("12:08:10"), z("12:08:20"), z("12:08:30")])
TOWN_ON_AGAIN = (z("12:09:00"), [z("12:09:10"), z("12:09:20")])
RELAUNCH = (z("12:10:00"), [z("12:12:10"), z("12:12:20")])
RELAUNCH_OPENING = z("12:11:00")
GREETINGS = ["Need something mended?", "Back again, exile?", "Steel needs tending, does it not?"]
FRAMES_FROM, FRAMES_TO = z("12:01:00"), z("12:09:30")
CHAT_FROM = z("12:04:50")


def build(root: Path, *, off_logged=True, relaunch=True, relaunch_logged=None, town_off_presses=3,
          town_off_at=None, off_shown="prefixed", covered=None, boss_presses=None, boss_twice=False,
          remark_presses_after=1, remark_logged=False, dlg_logged=False, dlg_subtitle=True, extra_shown=(),
          drop_bottom=(), toggled_in_file=True, frames=True, stages=True, relaunch_opening=True,
          relaunch_area=True, relaunch_config="false") -> Path:
    """A synthetic live folder in `root`; returns it.

    `relaunch_opening` and `relaunch_area` False leave the relaunched game's
    LOG FILE OPENING or its area entry out of the log; `relaunch_config` is
    what the after-exit-off read with the game closed finds the option set
    to, or None for no such read.

    `covered` names a panel that hides the boss's name while it is open;
    `boss_presses` cuts the last boss run's presses to that many;
    `boss_twice` stamps an unpaired `boss` run before the real one.
    `town_off_at` replaces the town-off presses; `off_shown` is how their
    greetings show: `prefixed` (`Renly: <g>`), `subtitle` (`<g>`) or None.
    `dlg_logged` is True (a dialogue line logged 2 s after open) or a list of
    (seconds after open, text) Renly logs; `dlg_subtitle` False drops Renly's
    prefixed subtitle from the dialogue window. `extra_shown` adds (text,
    from, to) rows to the bottom band, and `drop_bottom` leaves out the
    bottom-band frames of each (from, to)."""
    if relaunch_logged is None:
        relaunch_logged = off_logged
    if dlg_logged is True:
        dlg_logged = [(2.0, DLG_LINES[0])]
    live = root / "live"
    for sub in common.SUBFOLDERS:
        (live / sub).mkdir(parents=True, exist_ok=True)

    # Stamps and presses.
    stamps, presses = [], []
    stamps.append((TEST_STAMP, "test"))
    presses.append(TEST_PRESS)
    if boss_twice:
        stamps.append((z("12:01:30"), "boss"))
        presses += [z("12:01:35"), z("12:01:40"), z("12:01:45")]
    stamps.append((BOSS_STAMP, "boss"))
    boss = [BAR_UP] + [t for pair in zip(PANEL_OPEN, PANEL_CLOSE) for t in pair] + [AWAY]
    if boss_presses is not None:
        boss = boss[:boss_presses]
    presses += boss
    stamps.append((REMARK_STAMP, "remark"))
    presses += [KILL] + [REMARK_PRESS][:remark_presses_after]
    stamps.append((DLG_STAMP, "dialogue"))
    presses += [DLG_OPEN, DLG_CLOSE]
    stamps.append((TOWN_ON[0], "town-on"))
    presses += TOWN_ON[1]
    stamps.append((TOGGLE_OFF, "toggle-off"))
    stamps.append((TOWN_OFF[0], "town-off"))
    off_presses = list(town_off_at) if town_off_at is not None else TOWN_OFF[1][:town_off_presses]
    presses += off_presses
    stamps.append((TOWN_ON_AGAIN[0], "town-on-again"))
    presses += TOWN_ON_AGAIN[1]
    if relaunch:
        stamps.append((RELAUNCH[0], "relaunch-off"))
        presses += RELAUNCH[1]
    stamps.append((z("12:13:00"), "end"))
    if stages:
        for when, name in stamps:
            screentext.stage(live, name, now=lambda when=when: when)
    presses.sort()
    (live / common.MARKERS_TXT).write_text(
        "# registered Ctrl+Alt+M (MOD_NOREPEAT) at 2026-10-26T12:00:50.000Z\n# pid 999\n"
        + "".join("%d %s\n" % (i + 1, common.format_utc(p)) for i, p in enumerate(presses)),
        encoding="utf-8")

    # Client.txt: the older session, this session's start, then what the variant logs.
    head = (FIXTURES / "before.txt").read_bytes().replace(b"\n", b"\r\n")
    lines = (FIXTURES / "client.txt").read_text(encoding="utf-8").splitlines()
    gen = []

    def greet(press_list, logged, pid=PID):
        for i, p in enumerate(press_list):
            if logged:
                gen.append((p - td(1), log_line(p - td(1), "Renly: " + GREETINGS[i % 3], pid=pid)))

    greet(TOWN_ON[1], True)
    greet(off_presses, off_logged)
    greet(TOWN_ON_AGAIN[1], True)
    if relaunch:
        local = RELAUNCH_OPENING.astimezone(TZ).strftime("%Y/%m/%d %H:%M:%S")
        if relaunch_opening:
            gen.append((RELAUNCH_OPENING, "%s ***** LOG FILE OPENING *****" % local))
        if relaunch_area:
            gen.append((RELAUNCH_OPENING + td(8), log_line(RELAUNCH_OPENING + td(8),
                                                             'Generating level 15 area "G1_town" with seed 1',
                                                             "DEBUG", pid=PID2)))
        greet(RELAUNCH[1], relaunch_logged, pid=PID2)
    if remark_logged:
        for text, start, _ in (KILL_REMARK, LATER_REMARK):
            gen.append((start + td(0.5), log_line(start + td(0.5), "%s: %s" % (CHARACTER, text))))
    for after, text in dlg_logged or ():
        gen.append((DLG_OPEN + td(after), log_line(DLG_OPEN + td(after), "Renly: " + text)))
    gen.sort(key=lambda p: p[0])
    client = root / "Client.txt"
    client.write_bytes(head + "".join(l + "\r\n" for l in lines + [g for _, g in gen]).encode("utf-8"))
    common.write_json(live / common.SESSION_JSON, {
        "created_utc": "2026-10-26T11:59:00.000Z", "client_txt": str(client), "client_txt_offset": len(head)})
    for when in (z("11:59:00"), z("12:10:00")):
        tick = TICK_AT_ORIGIN + round((when - ORIGIN).total_seconds() * 1000)
        common.append_jsonl(live / common.CLOCK_JSONL, {"utc": common.format_utc(when), "tick_ms": tick})

    # The config, read at P3, after the toggle, with the game closed before the
    # relaunch (step F's after-exit-off) and at the restore.
    ini = root / "poe2_production_Config.ini"
    write_ini(ini)
    screentext.config(live, "before", ini, now=lambda: z("11:59:30"))
    write_ini(ini, dialogue="false" if toggled_in_file else "true")
    screentext.config(live, "after-toggle-off", ini, now=lambda: z("12:07:30"))
    if relaunch and relaunch_config is not None:
        write_ini(ini, dialogue=relaunch_config)
        screentext.config(live, "after-exit-off", ini, now=lambda: z("12:10:30"))
    write_ini(ini)
    screentext.config(live, "restored", ini, now=lambda: z("12:14:00"))

    # I3: the rehearsal's frames and the session's.
    def frames_into(folder, bands, start, end, texts, luma=lambda band, t: (60.0, 30.0), keep=lambda band, t: True):
        idx, ocr = [], []
        t = start
        while t <= end:
            stamp = t.strftime("%Y%m%dT%H%M%S.") + "%03dZ" % (t.microsecond // 1000)
            for band in bands:
                if not keep(band, t):
                    continue
                rel = "frames/%s/%s.jpg" % (band, stamp)
                mean, std = luma(band, t)
                idx.append({"utc": common.format_utc(t), "band": band, "path": rel,
                            "rect": TOP_RECT if band == "top" else CLIENT, "client": CLIENT,
                            "foreground": True, "luma_mean": mean, "luma_std": std})
                said = texts(band, t)
                if said:
                    ocr.append({"path": rel, "lines": [{"text": s, "box": [600, 300 + 40 * i, 1300, 330 + 40 * i]}
                                                       for i, s in enumerate(said)]})
            t += td(0.5)
        folder.mkdir(parents=True, exist_ok=True)
        with open(folder / screen.FRAMES_JSONL, "w", encoding="utf-8", newline="\n") as f:
            f.writelines(json.dumps(r) + "\n" for r in idx)
        with open(folder / screen.OCR_JSONL, "w", encoding="utf-8", newline="\n") as f:
            f.writelines(json.dumps(r) + "\n" for r in ocr)

    frames_into(live / "i3" / "rehearsal", ("bottom",), z("11:58:30"), z("11:58:32"),
                lambda band, t: list(screen.PROBE_LINES))
    if not frames:
        return live

    shown = []   # (text, from, to) in the bottom band
    for group, form in ((TOWN_ON[1], "prefixed"), (off_presses, off_shown), (TOWN_ON_AGAIN[1], "prefixed")):
        # The subtitle stays whether or not the line was logged.
        if form is None:
            continue
        for i, p in enumerate(group):
            shown.append((("Renly: " if form == "prefixed" else "") + GREETINGS[i % 3], p - td(0.7), p + td(3.5)))
    for text, a, b in (KILL_REMARK, LATER_REMARK):
        shown.append(("%s: %s" % (CHARACTER, text), a, b))
    if dlg_subtitle:
        shown.append(("Renly: " + DLG_LINES[0], DLG_OPEN + td(1), DLG_OPEN + td(6)))
    shown.append((CHAT, CHAT_FROM, FRAMES_TO))
    shown += list(extra_shown)
    covered_at = None
    if covered is not None:
        k = PANELS.index(covered)
        covered_at = (PANEL_OPEN[k], PANEL_CLOSE[k] + td(0.3))
    bar_from = z("12:01:35") if boss_twice else z("12:02:05")

    def texts(band, t):
        if band == "top":
            if not (bar_from <= t < AWAY + td(0.5)):
                return []
            if covered_at and covered_at[0] <= t <= covered_at[1]:
                return ["Inventory"]
            return [BOSS]
        out = [s for s, a, b in shown if a <= t <= b]
        if DLG_OPEN + td(0.3) <= t <= DLG_CLOSE:
            out += ["Renly"] + DLG_LINES
        return out

    def luma(band, t):
        if band == "top" and covered_at and covered_at[0] <= t <= covered_at[1]:
            return 20.0, 8.0
        return 60.0, 30.0

    def keep(band, t):
        return band != "bottom" or not any(a <= t <= b for a, b in drop_bottom)

    frames_into(live / "i3", ("top", "bottom"), FRAMES_FROM, FRAMES_TO, texts, luma, keep)
    return live


class Case(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = Path(self._tmp.name)
        self.assertIsNone(common.git_tree_of(self.tmp), "the temp folder sits inside a git checkout")

    def tearDown(self):
        self._tmp.cleanup()

    def analyse(self, live, **kw):
        kw.setdefault("confirmed", "yes")
        kw.setdefault("character", CHARACTER)
        kw.setdefault("boss", BOSS)
        kw.setdefault("dialogue_option", OPTION)
        kw.setdefault("setting_label", LABEL)
        kw.setdefault("setting_found", "yes")
        kw.setdefault("relaunch_pid", PID2)
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stderr(err):
            code, shown = screentext.analyse(live, PID, tz=TZ, ocr_runner=no_ocr, out=out, **kw)
        self.out, self.err = out.getvalue(), err.getvalue()
        return code, {r["name"]: r for r in shown}

    def verdicts(self, results):
        return {n: r["verdict"] for n, r in results.items()}


CONTROLS = ("control", "marker", "i3-rehearsal", "i3-town", "cfg-on-log", "cfg-setting-found",
            "cfg-file-written", "cfg-off-shown", "cfg-on-again-log", "dlg-window", "bar-control")


class Constants(unittest.TestCase):
    def test_the_checks_are_in_the_contexts_order(self):
        self.assertEqual(screentext.check_names(PANELS), [
            "control", "marker", "i3-rehearsal", "i3-town", "cfg-on-log", "cfg-setting-found",
            "cfg-file-written", "cfg-off-shown", "cfg-off-log", "cfg-on-again-log", "cfg-off-relaunch-log",
            "dlg-window", "dlg-option", "dlg-subtitle", "dlg-log", "bar-control", "bar-inventory",
            "bar-character", "bar-passive-tree", "bar-world-map", "bar-escape-menu", "bar-map-overlay",
            "bar-away", "remark-shown", "remark-kill", "remark-log"])

    def test_the_rules_are_pinned(self):
        self.assertEqual(screentext.STAGES, ("test", "boss", "remark", "dialogue", "town-on", "toggle-off",
                                             "town-off", "town-on-again", "relaunch-off", "end"))
        self.assertEqual(screentext.DEFAULT_PANELS, ("inventory", "character", "passive-tree", "world-map",
                                                     "escape-menu", "map-overlay"))
        self.assertEqual(screentext.DEFAULT_BOSS, "The Bloated Miller")
        self.assertEqual(screentext.DIALOGUE_NPC, "Renly")
        self.assertEqual((screentext.GREET_LOG_BEFORE_S, screentext.GREET_LOG_AFTER_S,
                          screentext.GREET_FRAME_AFTER_S), (6.0, 1.0, 2.0))
        self.assertEqual((screentext.KILL_BEFORE_S, screentext.KILL_AFTER_S), (2.0, 15.0))
        self.assertEqual((screentext.REMARK_LOG_BEFORE_S, screentext.REMARK_LOG_AFTER_S), (8.0, 2.0))
        self.assertEqual((screentext.AWAY_FROM_S, screentext.AWAY_TO_S), (1.0, 6.0))
        self.assertEqual(screentext.OCR_SCORE, 0.8)
        self.assertEqual((screentext.DLG_BEFORE_FROM_S, screentext.DLG_BEFORE_TO_S), (5.0, 1.0))
        self.assertEqual(screentext.GONE_FRAMES, 2)
        self.assertEqual(screentext.SHOWN_MIN, 2)
        self.assertEqual(screentext.TOWN_STAGES, ("town-on", "town-off", "town-on-again", "relaunch-off"))

    def test_speaker_prefix_matching(self):
        self.assertTrue(screentext.prefixed_by("Renly: Need something mended?", "Renly"))
        self.assertTrue(screentext.prefixed_by("RENLY : hello there", "Renly"))
        # Under 6 letters a near miss does not count; at 6 or more a score of 0.8 does.
        self.assertFalse(screentext.prefixed_by("Renty: hello there", "Renly"))
        self.assertTrue(screentext.prefixed_by("Ostrevanc: the mill", "Ostrevane"))
        self.assertFalse(screentext.prefixed_by("Renly says hello", "Renly"))
        self.assertFalse(screentext.prefixed_by("Zorblax: Renly: hi", "Renly"))


class Stage(Case):
    def test_a_stamp_appends_one_record(self):
        live = self.tmp / "live"
        live.mkdir()
        screentext.stage(live, "boss", now=lambda: z("12:02:00"))
        screentext.stage(live, "boss", now=lambda: z("12:03:00"))
        recs = common.read_jsonl(live / screentext.STAGES_JSONL)
        self.assertEqual(recs, [{"utc": "2026-10-26T12:02:00.000Z", "name": "boss"},
                                {"utc": "2026-10-26T12:03:00.000Z", "name": "boss"}])

    def test_an_unknown_name_is_refused(self):
        live = self.tmp / "live"
        live.mkdir()
        with self.assertRaises(common.ToolError):
            screentext.stage(live, "fight")
        self.assertFalse((live / screentext.STAGES_JSONL).exists())

    def test_a_folder_inside_a_git_checkout_is_refused(self):
        inside = ROOT / "tests"
        with self.assertRaises(common.ToolError):
            screentext.stage(inside, "test")
        self.assertFalse((inside / screentext.STAGES_JSONL).exists())

    def test_runs_take_their_presses_and_the_last_run_wins(self):
        stamps = [{"utc": "2026-10-26T12:02:00.000Z", "name": "boss"},
                  {"utc": "2026-10-26T12:01:00.000Z", "name": "test"},
                  {"utc": "2026-10-26T12:03:00.000Z", "name": "boss"}]
        presses = [screen.Press(1, z("12:00:30")), screen.Press(2, z("12:01:10")),
                   screen.Press(3, z("12:02:10")), screen.Press(4, z("12:03:10"))]
        last, ignored = screentext.last_runs(screentext.stage_runs(stamps, presses))
        self.assertEqual([p.n for p in last["test"].presses], [2])
        self.assertEqual([p.n for p in last["boss"].presses], [4])
        self.assertEqual(ignored, {"boss": 1})


class Config(Case):
    def test_reads_both_values_from_their_sections_and_never_writes(self):
        live = self.tmp / "live"
        live.mkdir()
        ini = write_ini(self.tmp / "c.ini", dialogue="false")
        before = ini.read_bytes()
        rec = screentext.config(live, "after-toggle-off", ini, now=lambda: z("12:07:30"))
        self.assertEqual(ini.read_bytes(), before)
        self.assertEqual(rec["output_all_dialogue_to_chat"], "false")
        self.assertEqual(rec["enable_subtitles"], "true")
        self.assertEqual(rec["sha256"], hashlib.sha256(before).hexdigest())
        self.assertEqual(rec["label"], "after-toggle-off")
        self.assertEqual(common.read_jsonl(live / screentext.CONFIG_JSONL), [rec])

    def test_a_key_in_another_section_or_absent_is_missing(self):
        live = self.tmp / "live"
        live.mkdir()
        ini = self.tmp / "c.ini"
        ini.write_text("[GENERAL]\noutput_all_dialogue_to_chat=true\n[UI]\nfoo=1\n", encoding="utf-8")
        rec = screentext.config(live, "before", ini)
        self.assertEqual(rec["output_all_dialogue_to_chat"], "missing")
        self.assertEqual(rec["enable_subtitles"], "missing")

    def test_the_command_prints_one_line(self):
        live = self.tmp / "live"
        live.mkdir()
        ini = write_ini(self.tmp / "c.ini")
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            code = screentext.main(["config", "--live", str(live), "--label", "before", "--ini", str(ini)])
        self.assertEqual(code, 0)
        lines = out.getvalue().splitlines()
        self.assertEqual(len(lines), 1)
        self.assertIn("output_all_dialogue_to_chat=true enable_subtitles=true", lines[0])

    def test_a_missing_file_is_an_error(self):
        live = self.tmp / "live"
        live.mkdir()
        with self.assertRaises(common.ToolError):
            screentext.config(live, "before", self.tmp / "nope.ini")


class Baseline(Case):
    def test_baseline_the_option_changes_nothing(self):
        live = build(self.tmp)
        code, results = self.analyse(live)
        v = self.verdicts(results)
        self.assertEqual(code, 0, self.err)
        self.assertEqual(list(results), screentext.check_names(PANELS))
        for name in CONTROLS:
            self.assertEqual(v[name], "pass", "%s: %s" % (name, results[name]["observed"]))
        self.assertEqual(v["cfg-off-log"], "pass")
        self.assertEqual(v["cfg-off-relaunch-log"], "pass")
        for p in PANELS:
            self.assertEqual(v["bar-" + p], "pass", results["bar-" + p]["observed"])
        self.assertEqual(v["bar-away"], "pass")
        self.assertEqual(v["dlg-option"], "pass")
        self.assertEqual(v["dlg-subtitle"], "pass")
        self.assertEqual(v["dlg-log"], "not-observed")
        self.assertEqual(v["remark-shown"], "pass")
        self.assertEqual(v["remark-kill"], "pass")
        self.assertEqual(v["remark-log"], "not-observed")
        self.assertIn("+2.00 s", results["remark-kill"]["observed"])
        self.assertIn("stage runs ignored: 0", self.out)
        lines = [l for l in self.out.splitlines() if l.startswith("- ")]
        self.assertEqual(len(lines), len(screentext.check_names(PANELS)))
        self.assertTrue(lines[0].startswith("- control | expected: "))
        saved = common.read_json(live / common.ANALYSIS_JSON)
        self.assertEqual([c["name"] for c in saved["checks"]], screentext.check_names(PANELS))

    def test_dialogue_window_box_is_a_fraction_of_the_client_rect(self):
        live = build(self.tmp)
        _, results = self.analyse(live, only=["dlg-window"])
        r = results["dlg-window"]
        self.assertEqual(r["verdict"], "pass")
        self.assertIn("box 0.312,0.278,0.677,", r["observed"])


class Target(Case):
    def test_target_the_option_drops_greetings_from_the_log(self):
        live = build(self.tmp, off_logged=False)
        code, results = self.analyse(live)
        v = self.verdicts(results)
        self.assertEqual(code, 0, self.err)
        for name in CONTROLS:
            self.assertEqual(v[name], "pass", "%s: %s" % (name, results[name]["observed"]))
        self.assertEqual(v["cfg-off-log"], "not-observed")
        self.assertEqual(v["cfg-on-again-log"], "pass")
        self.assertEqual(v["cfg-off-relaunch-log"], "not-observed")

    def test_target_file_not_written_is_not_observed(self):
        live = build(self.tmp, off_logged=False, toggled_in_file=False)
        _, results = self.analyse(live, only=["cfg-file-written"])
        self.assertEqual(results["cfg-file-written"]["verdict"], "not-observed")


class Variants(Case):
    def test_no_town_off_presses(self):
        live = build(self.tmp, town_off_presses=0)
        _, results = self.analyse(live, only=["cfg-off-shown", "cfg-off-log"])
        self.assertTrue(results["cfg-off-shown"]["verdict"].startswith("not-run (instrument:"))
        self.assertTrue(results["cfg-off-log"]["verdict"].startswith("not-run"))

    def test_a_panel_that_hides_the_name_fails(self):
        live = build(self.tmp, covered="inventory")
        _, results = self.analyse(live, only=["bar-control"] + ["bar-" + p for p in PANELS] + ["bar-away"])
        v = self.verdicts(results)
        self.assertEqual(v["bar-control"], "pass")
        self.assertEqual(v["bar-inventory"], "fail")
        self.assertIn("luma", results["bar-inventory"]["observed"])
        self.assertIn("other lines 7", results["bar-inventory"]["observed"])
        for p in PANELS[1:]:
            self.assertEqual(v["bar-" + p], "pass")
        self.assertEqual(v["bar-away"], "pass")

    def test_a_remark_shown_at_the_kill_and_not_logged(self):
        live = build(self.tmp)
        _, results = self.analyse(live, only=["remark-kill", "remark-log"])
        self.assertEqual(results["remark-kill"]["verdict"], "pass")
        self.assertEqual(results["remark-log"]["verdict"], "not-observed")
        self.assertIn("0 of 2", results["remark-log"]["observed"])

    def test_a_remark_logged_passes(self):
        live = build(self.tmp, remark_logged=True)
        _, results = self.analyse(live, only=["remark-log"])
        self.assertEqual(results["remark-log"]["verdict"], "pass")

    def test_a_dialogue_line_logged_passes(self):
        live = build(self.tmp, dlg_logged=True)
        _, results = self.analyse(live, only=["dlg-log"])
        self.assertEqual(results["dlg-log"]["verdict"], "pass")

    def test_unpaired_boss_presses(self):
        live = build(self.tmp, boss_presses=9)
        _, results = self.analyse(live, only=["bar-control", "bar-inventory", "bar-away"])
        for name in ("bar-control", "bar-inventory", "bar-away"):
            self.assertEqual(results[name]["verdict"], "not-run (instrument: presses do not pair: 9)")

    def test_boss_stamped_twice_reads_the_second_run(self):
        live = build(self.tmp, boss_twice=True)
        _, results = self.analyse(live, only=["bar-control"] + ["bar-" + p for p in PANELS] + ["bar-away"])
        for name, r in results.items():
            self.assertEqual(r["verdict"], "pass", "%s: %s" % (name, r["observed"]))
        self.assertIn("stage runs ignored: 1 (boss 1)", self.out)

    def test_a_remark_run_with_the_kill_press_alone(self):
        live = build(self.tmp, remark_presses_after=0)
        _, results = self.analyse(live, only=["remark-shown", "remark-kill"])
        self.assertTrue(results["remark-shown"]["verdict"].startswith("not-run"))
        self.assertEqual(results["remark-kill"]["verdict"], "pass")

    def test_a_control_on_g1_1_alone_passes(self):
        live = build(self.tmp, relaunch=False)
        _, results = self.analyse(live, only=["control"])
        self.assertEqual(results["control"]["verdict"], "pass", results["control"]["observed"])
        self.assertIn("G1_1", results["control"]["observed"])
        self.assertNotIn("G1_town", results["control"]["observed"])

    def test_no_relaunch_stage_is_not_needed(self):
        live = build(self.tmp, relaunch=False)
        _, results = self.analyse(live, only=["cfg-off-relaunch-log"])
        self.assertTrue(results["cfg-off-relaunch-log"]["verdict"].startswith("not-run (not needed:"))

    def relaunch_verdict(self, live, **kw):
        _, results = self.analyse(live, only=["cfg-off-relaunch-log"], **kw)
        return results["cfg-off-relaunch-log"]

    def test_relaunch_pid_equal_to_the_first_pid_is_not_run(self):
        # The target: without the control, the first client's greetings would
        # read as a relaunched client that logs nothing.
        r = self.relaunch_verdict(build(self.tmp, off_logged=False), relaunch_pid=PID)
        self.assertTrue(r["verdict"].startswith("not-run (control:"), r["verdict"])
        self.assertIn("first client's pid %d" % PID, r["verdict"])

    def test_no_log_file_opening_after_the_stamp_is_not_run(self):
        r = self.relaunch_verdict(build(self.tmp, off_logged=False, relaunch_opening=False))
        self.assertTrue(r["verdict"].startswith("not-run (control:"), r["verdict"])
        self.assertIn("no LOG FILE OPENING from Client %d" % PID2, r["verdict"])

    def test_no_area_entry_from_the_relaunch_pid_is_not_run(self):
        r = self.relaunch_verdict(build(self.tmp, off_logged=False, relaunch_area=False))
        self.assertTrue(r["verdict"].startswith("not-run (control:"), r["verdict"])
        self.assertIn("no area entry from Client %d" % PID2, r["verdict"])

    def test_no_after_exit_off_record_is_not_run(self):
        r = self.relaunch_verdict(build(self.tmp, off_logged=False, relaunch_config=None))
        self.assertTrue(r["verdict"].startswith("not-run (control:"), r["verdict"])
        self.assertIn("no after-exit-off or after-edit-off config record", r["verdict"])

    def test_the_option_read_true_before_the_relaunch_is_not_run(self):
        r = self.relaunch_verdict(build(self.tmp, off_logged=False, relaunch_config="true"))
        self.assertTrue(r["verdict"].startswith("not-run (control:"), r["verdict"])
        self.assertIn("option read true before the relaunch", r["verdict"])
        self.assertIn("after-exit-off", r["observed"])

    def test_a_marker_without_the_persons_answer_fails(self):
        live = build(self.tmp)
        _, results = self.analyse(live, only=["marker"], confirmed=None)
        self.assertEqual(results["marker"]["verdict"], "fail")

    def test_missing_inputs_are_not_run(self):
        live = build(self.tmp, frames=False, stages=False)
        (live / "i3" / screen.FRAMES_JSONL).unlink(missing_ok=True)
        code, results = self.analyse(live, setting_found=None)
        self.assertEqual(code, 0, self.err)
        self.assertEqual(results["control"]["verdict"], "pass")
        self.assertEqual(results["i3-rehearsal"]["verdict"], "pass")
        self.assertTrue(results["marker"]["verdict"].startswith("not-run (instrument: no stages.jsonl"))
        self.assertTrue(results["cfg-setting-found"]["verdict"].startswith("not-run (instrument:"))
        self.assertTrue(results["bar-control"]["verdict"].startswith("not-run (instrument:"))

    def test_only_merges_into_analysis_json(self):
        live = build(self.tmp)
        self.analyse(live, only=["control"])
        self.analyse(live, only=["marker"])
        saved = common.read_json(live / common.ANALYSIS_JSON)
        self.assertEqual([c["name"] for c in saved["checks"]], ["control", "marker"])

    def test_an_unknown_only_name_is_refused(self):
        live = build(self.tmp, frames=False)
        with self.assertRaises(common.ToolError):
            self.analyse(live, only=["bar-sideways"])

    def test_an_error_in_one_check_names_it_and_exits_1(self):
        live = build(self.tmp, frames=False)

        def boom(s):
            raise RuntimeError("synthetic")
        with mock.patch.dict(screentext.PLAIN, {"control": boom}):
            code, results = self.analyse(live, only=["control", "marker"])
        self.assertEqual(code, 1)
        self.assertEqual(results["control"]["verdict"], "not-run (analysis error)")
        self.assertEqual(results["marker"]["verdict"], "pass")
        self.assertIn("check control failed", self.err)


STALE = "Renly: The mill wheel turns again"
SPACED_OFF = [z("12:08:10"), z("12:08:30"), z("12:08:50")]   # town-off presses whose windows do not overlap


def before_of(start) -> tuple:
    """The before window `Session.newness` reads for a window opening at `start`."""
    return start - td(5), start - td(1)


def dropped(r) -> int:
    """The candidate reads a shown-line check's observed counts as dropped."""
    return sum(int(n) for n in re.findall(r"(\d+) candidate read\(s\) dropped", r["observed"]))


class Newness(Case):
    """A shown-line hit must be a new occurrence against [start - 5 s, start - 1 s]."""

    def test_newness_a_stale_row_alone_is_dropped(self):
        live = build(self.tmp, dlg_subtitle=False, extra_shown=[(STALE, DLG_OPEN - td(8), DLG_CLOSE + td(3))])
        _, results = self.analyse(live, only=["dlg-subtitle"])
        r = results["dlg-subtitle"]
        self.assertEqual(r["verdict"], "fail", r["observed"])
        self.assertGreaterEqual(dropped(r), 1, r["observed"])

    def test_newness_a_second_identical_row_beside_the_stale_row_counts(self):
        live = build(self.tmp, dlg_subtitle=False, extra_shown=[
            (STALE, DLG_OPEN - td(8), DLG_CLOSE + td(3)), (STALE, DLG_OPEN + td(2), DLG_OPEN + td(6))])
        _, results = self.analyse(live, only=["dlg-subtitle"])
        r = results["dlg-subtitle"]
        self.assertEqual(r["verdict"], "pass", r["observed"])
        self.assertIn("first +2.00 s after open", r["observed"])

    def test_newness_a_subtitle_beside_a_stale_prefixed_row_counts(self):
        # Spaced presses, so one press's stale row never appears inside an earlier press's window.
        stale = [("Renly: " + GREETINGS[i % 3], p - td(12), p + td(4)) for i, p in enumerate(SPACED_OFF)]
        # The stale prefixed rows alone (the negative control), then with the bare subtitle beside each.
        for form, verdict in ((None, "fail"), ("subtitle", "pass")):
            with self.subTest(off_shown=form):
                root = self.tmp / str(form)
                root.mkdir()
                live = build(root, town_off_at=SPACED_OFF, off_shown=form, extra_shown=stale)
                _, results = self.analyse(live, only=["cfg-off-shown"])
                r = results["cfg-off-shown"]
                self.assertEqual(r["verdict"], verdict, r["observed"])
                self.assertGreaterEqual(dropped(r), 1, r["observed"])
                if form:
                    self.assertIn("3 of 3 presses shown", r["observed"])

    def test_newness_a_line_back_after_gone_frames_counts(self):
        # Frames are 0.5 s apart: gone at open + 0.5 and + 1.0 is GONE_FRAMES (2) frames in a row.
        for until, back, verdict in ((0.2, 2.0, "pass"), (0.7, 1.5, "fail")):
            with self.subTest(gone_frames=round((back - until) / 0.5) - 1):
                root = self.tmp / str(until)
                root.mkdir()
                live = build(root, dlg_subtitle=False, extra_shown=[
                    (STALE, DLG_OPEN - td(8), DLG_OPEN + td(until)), (STALE, DLG_OPEN + td(back), DLG_OPEN + td(6))])
                _, results = self.analyse(live, only=["dlg-subtitle"])
                r = results["dlg-subtitle"]
                self.assertEqual(r["verdict"], verdict, r["observed"])

    def test_an_empty_before_window_is_not_judged(self):
        cases = {
            "cfg-off-shown": dict(town_off_at=SPACED_OFF,
                                  drop_bottom=[before_of(p - td(6)) for p in SPACED_OFF]),
            "remark-shown": dict(drop_bottom=[before_of(REMARK_PRESS - td(6))]),
            "remark-kill": dict(drop_bottom=[before_of(KILL - td(2))]),
            "dlg-subtitle": dict(drop_bottom=[before_of(DLG_OPEN)]),
        }
        for name, kw in cases.items():
            with self.subTest(check=name):
                # The same window with its before window kept passes (the positive control).
                for dropping, root in ((False, self.tmp / (name + "-kept")), (True, self.tmp / name)):
                    root.mkdir()
                    args = dict(kw) if dropping else {k: v for k, v in kw.items() if k != "drop_bottom"}
                    live = build(root, **args)
                    _, results = self.analyse(live, only=[name])
                    r = results[name]
                    if not dropping:
                        self.assertEqual(r["verdict"], "pass", r["observed"])
                        continue
                    self.assertTrue(r["verdict"].startswith("not-run (instrument: no frame before"),
                                    "%s: %s" % (r["verdict"], r["observed"]))
                    self.assertIn("no frame in [start - 5 s, start - 1 s]", r["observed"])

    def test_cfg_off_shown_passes_on_two_judged_presses_beside_an_unjudged_one(self):
        live = build(self.tmp, town_off_at=SPACED_OFF, drop_bottom=[before_of(SPACED_OFF[2] - td(6))])
        _, results = self.analyse(live, only=["cfg-off-shown"])
        r = results["cfg-off-shown"]
        self.assertEqual(r["verdict"], "pass", r["observed"])
        self.assertIn("2 of 3 presses shown, 1 not judged", r["observed"])
        self.assertIn("press 12:08:50.000Z: not judged: no frame in [start - 5 s, start - 1 s]", r["observed"])

    def test_cfg_off_shown_fails_when_the_unjudged_presses_could_not_reach_two(self):
        # Nothing is shown, and one unjudged press beside two judged ones could not make 2.
        live = build(self.tmp, town_off_at=SPACED_OFF, off_shown=None,
                     drop_bottom=[before_of(SPACED_OFF[2] - td(6))])
        _, results = self.analyse(live, only=["cfg-off-shown"])
        self.assertEqual(results["cfg-off-shown"]["verdict"], "fail", results["cfg-off-shown"]["observed"])


class DialogueLog(Case):
    """dlg-log tells Renly's quest dialogue from the greeting that opening the talk fires."""

    def dlg_log(self, live):
        _, results = self.analyse(live, only=["cfg-on-log", "dlg-window", "dlg-log"])
        self.assertEqual(results["cfg-on-log"]["verdict"], "pass")
        self.assertEqual(results["dlg-window"]["verdict"], "pass", results["dlg-window"]["observed"])
        return results["dlg-log"]

    def test_dlg_log_a_greeting_at_the_open_press_alone_is_not_observed(self):
        r = self.dlg_log(build(self.tmp, dlg_logged=[(0.0, GREETINGS[0])]))
        self.assertEqual(r["verdict"], "not-observed", r["observed"])
        self.assertIn("1 Renly line(s) at the open press", r["observed"])
        self.assertIn("0 matching nothing in the town pool", r["observed"])

    def test_dlg_log_a_quest_line_after_the_greeting_passes(self):
        r = self.dlg_log(build(self.tmp, dlg_logged=[(0.0, GREETINGS[0]), (4.0, DLG_LINES[0])]))
        self.assertEqual(r["verdict"], "pass", r["observed"])
        self.assertIn("1 Renly line(s) logged in (open + 1 s, close + 2 s] that are not a greeting", r["observed"])

    def test_dlg_log_a_town_greeting_inside_the_window_is_set_aside(self):
        r = self.dlg_log(build(self.tmp, dlg_logged=[(4.0, GREETINGS[1])]))
        self.assertEqual(r["verdict"], "not-observed", r["observed"])
        self.assertIn("1 set aside as greetings", r["observed"])

    def test_dlg_log_a_line_at_the_open_press_never_counts(self):
        # A line logged at the open press may be quest dialogue, but the rule never counts it.
        r = self.dlg_log(build(self.tmp, dlg_logged=[(0.5, DLG_LINES[0])]))
        self.assertEqual(r["verdict"], "not-observed", r["observed"])
        self.assertIn("1 matching nothing in the town pool", r["observed"])


class Privacy(Case):
    def test_the_character_and_chat_never_reach_stdout(self):
        live = build(self.tmp, remark_logged=True)
        self.analyse(live)
        for secret in (CHARACTER, "Zorblax", "turnips", "Quillfeather", "grinds", "Flour"):
            self.assertNotIn(secret, self.out)
            self.assertNotIn(secret, self.err)
        self.assertNotIn(CHARACTER, (live / common.ANALYSIS_JSON).read_text(encoding="utf-8"))


class Cli(unittest.TestCase):
    def test_help_exits_0(self):
        out = io.StringIO()
        with contextlib.redirect_stdout(out), self.assertRaises(SystemExit) as cm:
            screentext.main(["--help"])
        self.assertEqual(cm.exception.code, 0)
        for sub in ("stage", "config", "analyse"):
            self.assertIn(sub, out.getvalue())


if __name__ == "__main__":
    unittest.main()
