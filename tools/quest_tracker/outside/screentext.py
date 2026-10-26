"""Live 8's stage stamps, client-config reader and session analysis.

Developer-only. Usage:

    py -3 -m tools.quest_tracker.outside.screentext stage --live L --name NAME
    py -3 -m tools.quest_tracker.outside.screentext config --live L --label LABEL [--ini PATH]
    py -3 -m tools.quest_tracker.outside.screentext analyse --live L --pid P [--relaunch-pid P2]
          [--character NAME] [--boss NAME] [--npc NAME ...] [--dialogue-option TEXT]
          [--setting-label TEXT] [--setting-found yes|no] [--panels LIST]
          [--confirmed yes|no] [--only NAME,...]

The session asks what the screen shows of three sources (a boss's health
bar, with each game panel open and after walking away; the character's
remark at a kill; an NPC's dialogue window) and whether the client option
`output_all_dialogue_to_chat` decides whether NPC lines reach `Client.txt`.

- `stage` appends `{"utc", "name"}` to `L\\stages.jsonl`. The names are
  `STAGES`; any other is refused. A stage runs from its stamp to the next
  stamp, whatever its name, and each marker press belongs to the stage that
  holds it. A name may be stamped again (a repeat on another boss); every
  check reads only the last run of each stage, and `analyse` says how many
  earlier runs it ignored.
- `config` reads the client config (never writes it) and appends one record
  to `L\\config.jsonl`: label, utc, path, mtime, sha256, and the values of
  `output_all_dialogue_to_chat` (section `UI`) and `enable_subtitles`
  (section `GENERAL`), each `true`, `false` or `missing`. The default file
  is `My Games\\Path of Exile 2\\poe2_production_Config.ini` under the
  Documents known folder, which follows a OneDrive redirect.
- `analyse` prints one capture line per check, in `check_names` order, as
  `- <name> | expected: ... | observed: ... | <verdict>`, and writes
  `L\\analysis.json`. `--only` computes the named checks (and, silently,
  the checks their verdicts read) and merges them into an existing
  `analysis.json`.

What it reads: `session.json` (the `Client.txt` path and offset),
`clock.jsonl`, `markers.txt`, `stages.jsonl`, `config.jsonl`,
`i3\\frames.jsonl` with its OCR cache `i3\\ocr.jsonl`, and
`i3\\rehearsal`. Only the frames in the windows the asked checks need are
OCR'd (`ocr.ps1`, through `screen.ocr_frames`), once, before the checks run.

Speakers are Clearfell Encampment's vendors, the dialogue NPC (`Renly`),
each `--npc` and `--boss`. A screen line is *speaker-prefixed by NAME* when
its text before the first colon, normalised as `screen.normalize` does,
equals NAME normalised, or, for a NAME of 6 or more letters, scores at least
0.8 against it. A greeting's window is [press - 6 s, press + 1 s] for log
lines and [press - 6 s, press + 2 s] for frames. A shown-line hit
(`cfg-off-shown`, `dlg-subtitle`, `remark-shown`, `remark-kill`) must be a
new occurrence: its frame shows the text more times than the lines still on
screen from [start - 5 s, start - 1 s] of its window, so a stale chat row is
dropped while a repeated greeting or bark still counts. A window with no
frame in [start - 5 s, start - 1 s] is not judged: none of its reads is a
hit, and its check is `not-run (instrument: no frame before ...)` unless the
judged windows decide it. `dlg-log` counts only quest dialogue: an NPC line
logged in (open + 1 s, close + 2 s] that is not a greeting, i.e. that scores
under 0.8 either way against every line the NPC logged in [open - 6 s,
open + 1 s] or in a town stage (`TOWN_STAGES`). A log check is
`not-observed` only when its controls passed, and `not-run` otherwise. A
missing input makes the checks that need it `not-run (instrument: <why>)`;
an internal error in one check names it, and the analysis exits 1 after
printing every other check.

Privacy: the full-window band holds the chat and the character's name. The
printed lines carry no OCR text, only scores against known targets (an NPC,
the boss, a logged greeting, the option the person named), counts, times
and boxes as fractions of the client rect. `analysis.json` stays in `L` and
may keep OCR text, with the character's name masked as `<character>`.
Every command that writes refuses a folder inside a git checkout.
"""
from __future__ import annotations

import argparse
import datetime as _dt
import difflib
import hashlib
import json
import re
import statistics
import sys
import traceback
from pathlib import Path
from typing import Callable, NamedTuple

from tools.quest_tracker.outside import clientlog, common, screen
from tools.quest_tracker.outside.common import ToolError

STAGES = ("test", "boss", "remark", "dialogue", "town-on", "toggle-off", "town-off", "town-on-again",
          "relaunch-off", "end")
STAGES_JSONL = "stages.jsonl"
CONFIG_JSONL = "config.jsonl"
CONFIG_RELATIVE = Path("My Games") / "Path of Exile 2" / "poe2_production_Config.ini"
DIALOGUE_KEY = ("UI", "output_all_dialogue_to_chat")
SUBTITLE_KEY = ("GENERAL", "enable_subtitles")
FOLDERID_DOCUMENTS = "{FDD39AD0-238F-46AF-ADB4-6C85480369C7}"

DEFAULT_BOSS = "The Bloated Miller"
DIALOGUE_NPC = "Renly"
DEFAULT_PANELS = ("inventory", "character", "passive-tree", "world-map", "escape-menu", "map-overlay")
CHARACTER_MASK = "<character>"

# The context's rules. Tests pin them; changing one is an amendment.
OCR_SCORE = screen.MATCH_THRESHOLD       # 0.8
PREFIX_FUZZY_LETTERS = 6                 # a NAME this long may match a prefix at OCR_SCORE
GREET_LOG_BEFORE_S = 6.0                 # greeting window: [press - 6, press + 1] for log lines
GREET_LOG_AFTER_S = 1.0
GREET_FRAME_AFTER_S = 2.0                # ... and [press - 6, press + 2] for frames
TOWN_LINE_S = 4.0                        # i3-town: a logged line is looked for in [L, L + 4]
SHOWN_MIN = 2                            # cfg-off-shown: at least 2 presses shown
DLG_EDGE_S = 0.5                         # dlg-window: frames in [open + 0.5, close - 0.5] ...
DLG_BEFORE_FROM_S = 5.0                  # ... and absent in [open - 5, open - 1]
DLG_BEFORE_TO_S = 1.0                    # every shown-line check counts a hit new only past the
                                         # occurrences still on screen from [start - 5, start - 1]
GONE_FRAMES = 2                          # ... and one of those has left after this many frames in a row
                                         # without it (one missed OCR read is not leaving)
DLG_MIN_LINES = 3
DLG_MIN_WORDS = 3
DLG_MIN_FRAMES = 2
DLG_AFTER_S = 2.0                        # dlg-subtitle and dlg-log: [open, close + 2]
TOWN_STAGES = ("town-on", "town-off", "town-on-again", "relaunch-off")   # dlg-log's vendor-greeting pool
SAME_LINE_RATIO = 0.8                    # two OCR reads of one line
BAR_EDGE_S = 0.5                         # bar windows: [a + 0.5, b - 0.5]
BAR_SHARE = 0.5
BAR_MIN_FRAMES = 4
AWAY_FROM_S = 1.0                        # bar-away: [away + 1, away + 6]
AWAY_TO_S = 6.0
KILL_BEFORE_S = 2.0                      # remark-kill: [kill - 2, kill + 15]
KILL_AFTER_S = 15.0
REMARK_LOG_BEFORE_S = 8.0                # remark-log: [t - 8, t + 2] of the remark's first frame
REMARK_LOG_AFTER_S = 2.0

_PANEL_RE = re.compile(r"^[a-z0-9][a-z0-9-]*$")


def check_names(panels=DEFAULT_PANELS) -> list[str]:
    """Every check, in the order the capture lists them."""
    return (["control", "marker", "i3-rehearsal", "i3-town", "cfg-on-log", "cfg-setting-found",
             "cfg-file-written", "cfg-off-shown", "cfg-off-log", "cfg-on-again-log", "cfg-off-relaunch-log",
             "dlg-window", "dlg-option", "dlg-subtitle", "dlg-log", "bar-control"]
            + ["bar-" + p for p in panels]
            + ["bar-away", "remark-shown", "remark-kill", "remark-log"])


def parse_panels(text) -> tuple[str, ...]:
    if text is None:
        return DEFAULT_PANELS
    items = text if isinstance(text, (list, tuple)) else str(text).split(",")
    panels = tuple(p.strip().lower() for p in items if p.strip())
    for p in panels:
        if not _PANEL_RE.match(p) or p in ("control", "away"):
            raise ToolError("bad panel name %r in --panels: lowercase letters, digits and dashes, "
                            "and not `control` or `away`" % p)
    if len(set(panels)) != len(panels):
        raise ToolError("a panel is named twice in --panels: %s" % ",".join(panels))
    return panels


# --------------------------------------------------------------------------
# Small helpers

class NotRun(Exception):
    """A check that cannot be computed; its verdict is `not-run (<why>)`."""

    def __init__(self, why: str):
        super().__init__(why)
        self.why = why


def missing(why: str) -> NotRun:
    return NotRun("instrument: " + why)


def _td(s: float) -> _dt.timedelta:
    return _dt.timedelta(seconds=s)


def _hms(t: _dt.datetime) -> str:
    return common.format_utc(t)[11:]


def _rel(t: _dt.datetime, ref: _dt.datetime) -> str:
    return "%+.2f s" % (t - ref).total_seconds()


def result(name: str, expected: str, observed: str, verdict: str, **details) -> dict:
    return {"name": name, "expected": expected, "observed": observed, "verdict": verdict,
            "details": details}


def clean(text) -> str:
    return " ".join(str(text).replace("|", "/").split())


def capture_line(r: dict) -> str:
    return "- %s | expected: %s | observed: %s | %s" % (
        r["name"], clean(r["expected"]), clean(r["observed"]), r["verdict"])


def letters(text: str) -> int:
    return sum(1 for ch in text if ch.isalpha())


def speaker_prefix(text: str) -> str | None:
    head, sep, _ = text.partition(":")
    return head if sep else None


def after_prefix(text: str) -> str:
    _, sep, rest = text.partition(":")
    return rest.strip() if sep else text


def prefixed_by(text: str, name: str) -> bool:
    """Whether the screen line `text` is speaker-prefixed by `name`."""
    head = speaker_prefix(text)
    target = screen.normalize(name)
    if head is None or not target:
        return False
    if screen.normalize(head) == target:
        return True
    return letters(name) >= PREFIX_FUZZY_LETTERS and screen.fuzzy_score(name, head) >= OCR_SCORE


def same_line(a: str, b: str) -> bool:
    """Two OCR reads of one line (both normalised)."""
    return difflib.SequenceMatcher(None, a, b, autojunk=False).ratio() >= SAME_LINE_RATIO


def reads_as(text: str, other: str) -> bool:
    """Whether the screen line `other` shows the text `text` (a read of the same line, or contains it)."""
    return same_line(screen.normalize(text), screen.normalize(other)) or screen.fuzzy_score(text, other) >= OCR_SCORE


def box_fraction(frame: screen.Frame, box) -> tuple | None:
    """An OCR box (pixels of the frame's band) as fractions of the client rect."""
    if frame.rect is None or frame.client is None or len(box) != 4:
        return None
    cl, ct, cr, cb = frame.client
    w, h = cr - cl, cb - ct
    if w <= 0 or h <= 0:
        return None
    x0, y0 = frame.rect[0] - cl, frame.rect[1] - ct
    return ((x0 + box[0]) / w, (y0 + box[1]) / h, (x0 + box[2]) / w, (y0 + box[3]) / h)


def union(boxes) -> tuple | None:
    boxes = [b for b in boxes if b is not None]
    if not boxes:
        return None
    return (min(b[0] for b in boxes), min(b[1] for b in boxes), max(b[2] for b in boxes), max(b[3] for b in boxes))


def _box_text(box) -> str:
    return "none" if box is None else ",".join("%.3f" % v for v in box)


# --------------------------------------------------------------------------
# Stages

class Run(NamedTuple):
    name: str
    start: _dt.datetime
    end: _dt.datetime | None      # the next stamp, or None for the last
    presses: list


def stage(live: Path, name: str, now=common.now_utc) -> dict:
    """Stamp stage `name` into `L\\stages.jsonl`; returns the record."""
    if name not in STAGES:
        raise ToolError("unknown stage %r: expected one of %s" % (name, ", ".join(STAGES)))
    live = Path(live)
    common.refuse_inside_git(live, "stage stamps")
    if not live.is_dir():
        raise ToolError("no live folder at %s" % live)
    record = {"utc": common.format_utc(now()), "name": name}
    common.append_jsonl(live / STAGES_JSONL, record)
    return record


def stage_runs(stamps: list[dict], presses) -> list[Run]:
    """The runs, in time order, each with the presses in [stamp, next stamp)."""
    ordered = sorted(((common.parse_utc(r["utc"]), str(r["name"])) for r in stamps), key=lambda p: p[0])
    runs = []
    for i, (start, name) in enumerate(ordered):
        end = ordered[i + 1][0] if i + 1 < len(ordered) else None
        held = [p for p in presses if p.utc >= start and (end is None or p.utc < end)]
        runs.append(Run(name, start, end, held))
    return runs


def last_runs(runs: list[Run]) -> tuple[dict[str, Run], dict[str, int]]:
    """(the last run of each name, how many earlier runs of each name were ignored)."""
    last, ignored = {}, {}
    for r in runs:
        if r.name in last:
            ignored[r.name] = ignored.get(r.name, 0) + 1
        last[r.name] = r
    return last, ignored


# --------------------------------------------------------------------------
# The client config

def read_ini_values(data: bytes) -> dict[tuple[str, str], str]:
    """{(SECTION, key): value} of an ini file's bytes (a BOM is allowed)."""
    text = data.decode("utf-8-sig", errors="replace")
    out, section = {}, ""
    for raw in text.splitlines():
        line = raw.strip()
        if not line or line[0] in ";#":
            continue
        if line.startswith("[") and line.endswith("]"):
            section = line[1:-1].strip().upper()
            continue
        key, sep, value = line.partition("=")
        if sep:
            out[(section, key.strip())] = value.strip()
    return out


def _value(values: dict, key: tuple[str, str]) -> str:
    v = values.get((key[0].upper(), key[1]))
    if v is None:
        return "missing"
    low = v.lower()
    return low if low in ("true", "false") else v


def documents_folder() -> Path:
    """The Documents known folder (`SHGetKnownFolderPath`), redirect included."""
    if sys.platform != "win32":
        raise ToolError("the Documents known folder is a Windows lookup; pass --ini")
    import ctypes
    import uuid

    class GUID(ctypes.Structure):
        _fields_ = [("Data1", ctypes.c_uint32), ("Data2", ctypes.c_uint16), ("Data3", ctypes.c_uint16),
                    ("Data4", ctypes.c_ubyte * 8)]

    g = uuid.UUID(FOLDERID_DOCUMENTS)
    guid = GUID(g.fields[0], g.fields[1], g.fields[2], (ctypes.c_ubyte * 8).from_buffer_copy(g.bytes[8:]))
    fn = ctypes.windll.shell32.SHGetKnownFolderPath
    fn.argtypes = [ctypes.POINTER(GUID), ctypes.c_uint32, ctypes.c_void_p, ctypes.POINTER(ctypes.c_void_p)]
    fn.restype = ctypes.c_long
    free = ctypes.windll.ole32.CoTaskMemFree
    free.argtypes = [ctypes.c_void_p]
    free.restype = None
    out = ctypes.c_void_p()
    hr = fn(ctypes.byref(guid), 0, None, ctypes.byref(out))
    try:
        if hr != 0 or not out.value:
            raise ToolError("SHGetKnownFolderPath(Documents) failed: 0x%08x" % (hr & 0xFFFFFFFF))
        return Path(ctypes.wstring_at(out.value))
    finally:
        if out.value:
            free(out)


def default_ini() -> Path:
    return documents_folder() / CONFIG_RELATIVE


def config(live: Path, label: str, ini: Path | None = None, now=common.now_utc) -> dict:
    """Read the client config and append its record to `L\\config.jsonl`."""
    live = Path(live)
    label = (label or "").strip()
    if not label:
        raise ToolError("a config record needs a --label")
    common.refuse_inside_git(live, "config record")
    if not live.is_dir():
        raise ToolError("no live folder at %s" % live)
    path = Path(ini) if ini is not None else default_ini()
    if not path.is_file():
        raise ToolError("no client config at %s" % path)
    with open(path, "rb") as f:
        data = f.read()
    st = path.stat()
    values = read_ini_values(data)
    record = {
        "label": label,
        "utc": common.format_utc(now()),
        "path": str(path),
        "mtime": common.format_utc(common.utc_from_epoch_ns(st.st_mtime_ns)),
        "sha256": hashlib.sha256(data).hexdigest(),
        DIALOGUE_KEY[1]: _value(values, DIALOGUE_KEY),
        SUBTITLE_KEY[1]: _value(values, SUBTITLE_KEY),
    }
    common.append_jsonl(live / CONFIG_JSONL, record)
    return record


def config_line(rec: dict) -> str:
    return "config: %s %s=%s %s=%s sha256 %s mtime %s" % (
        rec["label"], DIALOGUE_KEY[1], rec[DIALOGUE_KEY[1]], SUBTITLE_KEY[1], rec[SUBTITLE_KEY[1]],
        rec["sha256"][:12], rec["mtime"])


# --------------------------------------------------------------------------
# The session

class Session:
    """The live folder's inputs, read lazily; each accessor raises NotRun when
    its input is missing."""

    def __init__(self, live: Path, pid: int, relaunch_pid: int | None = None, character: str | None = None,
                 boss: str = DEFAULT_BOSS, npcs=(), dialogue_npc: str = DIALOGUE_NPC,
                 dialogue_option: str | None = None, setting_label: str | None = None,
                 setting_found: str | None = None, panels=DEFAULT_PANELS, confirmed: str | None = None,
                 tz: _dt.tzinfo | None = None, ocr_runner: Callable = screen.run_ocr_ps1):
        self.live = Path(live)
        self.pid = int(pid)
        self.relaunch_pid = None if relaunch_pid is None else int(relaunch_pid)
        self.character = (character or "").strip() or None
        self.boss = boss or DEFAULT_BOSS
        self.npc = dialogue_npc
        self.dialogue_option = dialogue_option
        self.setting_label = setting_label
        self.setting_found = setting_found
        self.panels = tuple(panels)
        self.confirmed = confirmed
        self.tz = tz
        self.ocr_runner = ocr_runner
        speakers = []
        for name in clientlog.TOWN_SPEAKERS + (dialogue_npc,) + tuple(npcs or ()) + (self.boss,):
            if name and name not in speakers:
                speakers.append(name)
        self.speakers = tuple(speakers)
        self._memo: dict = {}
        self._ocr: dict = {}

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

    def mask(self, text: str) -> str:
        if self.character:
            text = re.sub(re.escape(self.character), CHARACTER_MASK, text, flags=re.IGNORECASE)
        return text

    # ---- the log

    def session_json(self) -> dict:
        def make():
            path = self.live / common.SESSION_JSON
            if not path.is_file():
                raise missing("no session.json in %s (run `session prep`)" % self.live)
            return common.read_json(path)
        return self._once("session", make)

    def log(self) -> clientlog.ClientLog:
        def make():
            s = self.session_json()
            path = Path(s["client_txt"])
            if not path.is_file():
                raise missing("no Client.txt at %s" % path)
            speakers = self.speakers + ((self.character,) if self.character else ())
            return clientlog.read_log(path, int(s["client_txt_offset"]), common.load_anchors(self.live),
                                      self.tz, speakers=speakers)
        return self._once("log", make)

    def speaker_lines(self, a, b, pid: int | None) -> list:
        """The speakers' logged lines in [a, b], from `pid` when given."""
        return [l for l in self.log().npc() if l.speaker in self.speakers and a <= l.utc <= b
                and (pid is None or l.pid == pid)]

    def greetings(self, press, pid: int | None) -> list:
        return self.speaker_lines(press - _td(GREET_LOG_BEFORE_S), press + _td(GREET_LOG_AFTER_S), pid)

    # ---- markers and stages

    def markers(self) -> screen.Markers:
        def make():
            path = self.live / common.MARKERS_TXT
            if not path.is_file():
                raise missing("no markers.txt (the marker did not run)")
            return screen.read_markers(path)
        return self._once("markers", make)

    def runs(self) -> tuple[dict, dict]:
        def make():
            path = self.live / STAGES_JSONL
            if not path.is_file():
                raise missing("no stages.jsonl (no stage was stamped)")
            return last_runs(stage_runs(common.read_jsonl(path), self.markers().presses))
        return self._once("runs", make)

    def run(self, name: str) -> Run | None:
        return self.runs()[0].get(name)

    def need_run(self, name: str) -> Run:
        r = self.run(name)
        if r is None:
            raise missing("no %s stage in stages.jsonl" % name)
        return r

    def need_presses(self, name: str) -> list:
        presses = self.need_run(name).presses
        if not presses:
            raise missing("no press in stage %s" % name)
        return [p.utc for p in presses]

    # ---- config

    def config_records(self) -> list[dict]:
        def make():
            path = self.live / CONFIG_JSONL
            if not path.is_file():
                raise missing("no config.jsonl (run `screentext config`)")
            return common.read_jsonl(path)
        return self._once("config", make)

    # ---- frames and OCR

    def frames(self) -> list:
        def make():
            path = self.live / "i3" / screen.FRAMES_JSONL
            if not path.is_file():
                raise missing("no i3\\frames.jsonl (the capture did not run)")
            return screen.read_frames(self.live / "i3")
        return self._once("frames", make)

    def ensure_ocr(self, frames) -> None:
        want = {screen.path_key(f.path): f.path for f in frames if f.path is not None}
        todo = {k: p for k, p in want.items() if k not in self._ocr}
        if not todo:
            return
        try:
            found, _, _ = screen.ocr_frames(self.live / "i3", todo.values(), runner=self.ocr_runner)
        except ToolError as err:
            raise missing("OCR failed: %s" % err)
        for k in todo:
            self._ocr[k] = found.get(k, [])

    def window(self, band: str, a, b) -> list:
        """The captured `band` frames in [a, b], their OCR read."""
        frames = screen.frames_in_window(self.frames(), a, b, band)
        self.ensure_ocr(frames)
        return frames

    def lines(self, frame) -> list:
        return [] if frame.path is None else self._ocr.get(screen.path_key(frame.path), [])

    def texts(self, frames) -> list[str]:
        return [l.text for f in frames for l in self.lines(f)]

    # ---- the stages' presses

    def boss_presses(self) -> list:
        presses = [p.utc for p in self.need_run("boss").presses]
        if len(presses) != 2 + 2 * len(self.panels):
            raise missing("presses do not pair: %d" % len(presses))
        return presses

    def dialogue(self) -> tuple:
        presses = [p.utc for p in self.need_run("dialogue").presses]
        if len(presses) < 2:
            raise missing("fewer than 2 presses in stage dialogue: %d" % len(presses))
        return presses[0], presses[1]

    def need_character(self) -> str:
        if not self.character:
            raise missing("no --character")
        return self.character

    def newness(self, start, end) -> tuple:
        """(the bottom-band frames of [start, end], a test `is_new(frame, text)`, the count of
        frames in [start - 5, start - 1]).

        A line read in [start - 5, start - 1] was on screen before the window
        opened (the chat panel keeps it), so it is not the event the window
        looks for. Its text alone cannot tell it from a new line with the same
        words, which vendors' greetings and characters' barks are by design,
        so occurrences are counted instead. The stale count is the most lines
        reading as `text` in one frame before the window. It falls when that
        many have not been on screen for GONE_FRAMES frames in a row, from the
        last frame before onwards. A read is new when its frame holds more
        lines reading as it than the stale count: a second chat row with the
        same greeting, a subtitle beside the stale row, or a line back after
        the old one left the screen.
        """
        before = self.window("bottom", start - _td(DLG_BEFORE_FROM_S), start - _td(DLG_BEFORE_TO_S))
        walk = [f for f in self.window("bottom", start - _td(DLG_BEFORE_TO_S), end)
                if f.utc > start - _td(DLG_BEFORE_TO_S)]
        fresh = {}

        def is_new(frame, text) -> bool:
            key = screen.normalize(text)
            if key not in fresh:
                def count(f):
                    return sum(1 for l in self.lines(f) if reads_as(text, l.text))
                stale = max((count(f) for f in before), default=0)
                recent = [count(before[-1])] if before else []
                fresh[key] = set()
                for f in walk:
                    c = count(f)
                    if c > stale:
                        fresh[key].add(screen.path_key(f.path))
                    recent = (recent + [c])[-GONE_FRAMES:]
                    if len(recent) == GONE_FRAMES:
                        stale = min(stale, max(recent))
            return screen.path_key(frame.path) in fresh[key]
        return [f for f in walk if f.utc >= start], is_new, len(before)

    def first_remark(self, frames, is_new) -> tuple:
        """((frame, line) of the first new line speaker-prefixed by the character, or None,
        the count of such line reads that were a line still on screen from before the window)."""
        name = self.need_character()
        hit, dropped = None, 0
        for f in frames:
            for l in self.lines(f):
                if not prefixed_by(l.text, name):
                    continue
                if not is_new(f, l.text):
                    dropped += 1
                elif hit is None:
                    hit = (f, l)
        return hit, dropped


# --------------------------------------------------------------------------
# The OCR windows each check reads, so OCR runs once for all of them

def _greeting_frames(s: Session, presses) -> list:
    return [("bottom", p - _td(GREET_LOG_BEFORE_S), p + _td(GREET_FRAME_AFTER_S)) for p in presses]


def _with_before(windows) -> list:
    """Each window, and the [start - 5, start] `Session.newness` reads for it."""
    out = []
    for band, a, b in windows:
        out += [(band, a, b), (band, a - _td(DLG_BEFORE_FROM_S), a)]
    return out


def windows_of(s: Session, name: str) -> list:
    if name == "i3-town":
        lines = []
        for p in s.need_presses("town-on"):
            lines += s.greetings(p, s.pid)
        return [("bottom", l.utc, l.utc + _td(TOWN_LINE_S)) for l in lines]
    if name in ("cfg-off-shown", "cfg-off-log"):
        return _with_before(_greeting_frames(s, s.need_presses("town-off")))
    if name.startswith("dlg-"):
        o, c = s.dialogue()
        return [("bottom", o - _td(DLG_BEFORE_FROM_S), c + _td(DLG_AFTER_S))]
    if name.startswith("bar-"):
        p = s.boss_presses()
        return [("top", p[0], p[-1] + _td(AWAY_TO_S))]
    if name.startswith("remark-"):
        p = [x.utc for x in s.need_run("remark").presses]
        w = _greeting_frames(s, p[1:])
        if p:
            w.append(("bottom", p[0] - _td(KILL_BEFORE_S), p[0] + _td(KILL_AFTER_S)))
        return _with_before(w)
    return []


def prefetch(s: Session, names) -> None:
    """OCR every frame the named checks read, in one runner call."""
    try:
        frames = s.frames()
    except NotRun:
        return
    picked = {}
    for name in names:
        try:
            ws = windows_of(s, name)
        except NotRun:
            continue
        for band, a, b in ws:
            for f in screen.frames_in_window(frames, a, b, band):
                picked[screen.path_key(f.path)] = f
    try:
        s.ensure_ocr(picked.values())
    except NotRun:
        pass   # each check reports it


# --------------------------------------------------------------------------
# The checks

# A shown-line check's observed: the reads `Session.newness` dropped as the same occurrence.
DROPPED = "dropped as a line still on screen from the %d frame(s) before, not a new one"
# ... and a window `Session.newness` found no frame before: none of its reads is a hit.
NOT_JUDGED = "not judged: no frame in [start - 5 s, start - 1 s]"


def _new_rule(before: str) -> str:
    return ("its frame shows the text more times than the lines still on screen from [%s], "
            "one having left after %d frames in a row without it" % (before, GONE_FRAMES))


def _unjudged_presses(n: int) -> str:
    return "not-run (instrument: no frame before %d press(es))" % n


UNJUDGED_WINDOW = "not-run (instrument: no frame before the window)"


EXPECTED = {
    "control": "after the session offset, a LOG FILE OPENING, lines from [INFO Client <pid>] with the lease's "
               "pid, and an area entry line from it",
    "marker": "the marker's # registered header, a press in stage test, and the person confirms it",
    "i3-rehearsal": "each probe line scores >= %.1f in a captured frame" % OCR_SCORE,
    "i3-town": "a speaker's line logged in a town-on greeting window scores >= %.1f in a bottom-band frame "
               "within [L, L + 4 s]" % OCR_SCORE,
    "cfg-on-log": "a speaker line logged in [press - 6 s, press + 1 s] of at least one town-on press",
    "cfg-setting-found": "the person found the option in Options (--setting-found yes)",
    "cfg-file-written": "the after-toggle-off config record reads output_all_dialogue_to_chat=false",
    "cfg-off-shown": "at least %d town-off presses each have a bottom-band line in [press - 6 s, press + 2 s] "
                     "speaker-prefixed by a speaker, or scoring >= %.1f against a line logged in town-on, "
                     "and new (%s); a press with no frame in [press - 11 s, press - 7 s] before its window is "
                     "not judged, not-run when the judged presses fall short of %d only by it"
                     % (SHOWN_MIN, OCR_SCORE, _new_rule("press - 11 s, press - 7 s"), SHOWN_MIN),
    "cfg-off-log": "a speaker line logged, or a logged line repeating the shown text, in [press - 6 s, "
                   "press + 1 s] of a shown town-off greeting (controls: cfg-on-log, cfg-off-shown)",
    "cfg-on-again-log": "a speaker line logged in [press - 6 s, press + 1 s] of at least one town-on-again press",
    "cfg-off-relaunch-log": "a speaker line from the relaunched pid logged in [press - 6 s, press + 1 s] of at "
                            "least one relaunch-off press (the press is the control)",
    "dlg-window": "between the first two dialogue presses, at least %d lines of %d or more words, each in %d or "
                  "more frames of [open + 0.5, close - 0.5] and in none of [open - 5, open - 1], plus the "
                  "NPC's name" % (DLG_MIN_LINES, DLG_MIN_WORDS, DLG_MIN_FRAMES),
    "dlg-option": "the chosen topic scores >= %.1f in a frame between open and close" % OCR_SCORE,
    "dlg-subtitle": "a line speaker-prefixed by the NPC within [open, close + 2 s], and new (%s); not-run "
                    "when no frame is in [open - 5 s, open - 1 s]" % _new_rule("open - 5 s, open - 1 s"),
    "dlg-log": "an NPC line logged in (open + 1 s, close + 2 s] that is not a greeting (>= %.1f either way "
               "against a line the NPC logged in [open - 6 s, open + 1 s] or in a town stage), or a dlg-window "
               "line repeated by such a logged line, any speaker (controls: cfg-on-log, dlg-window)" % OCR_SCORE,
    "bar-control": "the boss's name scores >= %.1f in at least half (and at least %d) of the top-band frames "
                   "in [bar-up + 0.5, first panel open - 0.5]" % (OCR_SCORE, BAR_MIN_FRAMES),
    "bar-panel": "the boss's name reads in at least half of the top-band frames in [open + 0.5, close - 0.5] "
                 "of the %s panel (fail: covered or unreadable)",
    "bar-away": "the boss's name in no top-band frame within [away + 1, away + 6]",
    "remark-shown": "a line speaker-prefixed by the character in [press - 6 s, press + 2 s] of a remark press "
                    "after the first, and new (%s); a press with no frame in [press - 11 s, press - 7 s] is "
                    "not judged, not-run when no judged press shows one and a press is not judged"
                    % _new_rule("press - 11 s, press - 7 s"),
    "remark-kill": "a line speaker-prefixed by the character in a bottom-band frame within [kill - 2 s, "
                   "kill + 15 s], and new (%s) (fail: no remark shown at the kill); not-run when no frame "
                   "is in [kill - 7 s, kill - 3 s]" % _new_rule("kill - 7 s, kill - 3 s"),
    "remark-log": "for each remark found, a logged line repeating its text after the prefix, or a line with "
                  "the character as speaker, within [t - 8 s, t + 2 s] of its first frame (controls: "
                  "cfg-on-log, a remark found)",
}


def expected_of(name: str) -> str:
    if name in EXPECTED:
        return EXPECTED[name]
    return EXPECTED["bar-panel"] % name[len("bar-"):]


def _controls(done: dict, names) -> str | None:
    """None when every named control passed, else why not."""
    bad = ["%s %s" % (n, done[n]["verdict"]) for n in names if done[n]["verdict"] != "pass"]
    return ("control: " + ", ".join(bad)) if bad else None


def check_control(s: Session) -> dict:
    expected = EXPECTED["control"].replace("<pid>", str(s.pid))
    log = s.log()
    openings = log.openings()
    mine = log.pids.get(s.pid, 0)
    others = sorted(p for p in log.pids if p != s.pid)
    areas = [a for a in log.areas() if a.pid == s.pid]
    observed = "LOG FILE OPENING x%d; %d lines from Client %d%s; area entries %s; times from counter %d, wall %d" % (
        len(openings), mine, s.pid, (" (other pids: %s)" % ", ".join(map(str, others))) if others else "",
        (", ".join("%s %s" % (a.text, _hms(a.utc)) for a in areas[:3]) + (" ..." if len(areas) > 3 else ""))
        if areas else "none", log.sources["counter"], log.sources["wall"])
    ok = bool(openings) and mine > 0 and bool(areas)
    return result("control", expected, observed, "pass" if ok else "fail",
                  openings=len(openings), pid_lines=mine, other_pids=others)


def check_marker(s: Session) -> dict:
    markers = s.markers()
    registered = any(h.lower().startswith("registered") for h in markers.header)
    run = s.run("test")
    presses = run.presses if run else []
    answer = (s.confirmed or "").strip().lower()
    observed = "header %s; presses %d; test stage %s; person: %s" % (
        "registered" if registered else "no # registered line", len(markers.presses),
        ("%d press(es), first %s" % (len(presses), _hms(presses[0].utc))) if presses
        else ("no press" if run else "not stamped"),
        answer or "no answer (pass --confirmed yes|no)")
    ok = registered and bool(presses) and answer.split()[:1] == ["yes"]
    return result("marker", EXPECTED["marker"], observed, "pass" if ok else "fail")


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


def check_i3_town(s: Session) -> dict:
    expected = EXPECTED["i3-town"]
    presses = s.need_presses("town-on")
    s.frames()
    lines, seen = [], set()
    for p in presses:
        for l in s.greetings(p, s.pid):
            if l.n not in seen:
                seen.add(l.n)
                lines.append(l)
    if not lines:
        return result("i3-town", expected, "%d town-on press(es); no speaker line logged in their greeting "
                      "windows" % len(presses), "fail")
    parts, ok, black = [], False, 0
    for l in lines:
        frames = s.window("bottom", l.utc, l.utc + _td(TOWN_LINE_S))
        score = screen.best_score(l.text, s.texts(frames))
        black += sum(1 for f in frames if screen.is_black(f))
        parts.append("%s %s: %.2f in %d frames" % (l.speaker, _hms(l.utc), score, len(frames)))
        ok = ok or score >= OCR_SCORE
    return result("i3-town", expected, "; ".join(parts) + "; black frames %d" % black, "pass" if ok else "fail",
                  black_frames=black)


def _logged_per_press(s: Session, stage_name: str, pid: int) -> tuple[bool, str]:
    presses = s.need_presses(stage_name)
    parts, ok = [], False
    for p in presses:
        lines = s.greetings(p, pid)
        ok = ok or bool(lines)
        parts.append("press %s: %s" % (_hms(p), ", ".join("%s %s" % (l.speaker, _rel(l.utc, p)) for l in lines)
                                       if lines else "none logged"))
    return ok, "%d of %d press(es) with a speaker line from Client %d; %s" % (
        sum(1 for p in presses if s.greetings(p, pid)), len(presses), pid, "; ".join(parts))


def check_cfg_on_log(s: Session) -> dict:
    ok, observed = _logged_per_press(s, "town-on", s.pid)
    return result("cfg-on-log", EXPECTED["cfg-on-log"], observed, "pass" if ok else "fail")


def check_cfg_on_again_log(s: Session) -> dict:
    ok, observed = _logged_per_press(s, "town-on-again", s.pid)
    return result("cfg-on-again-log", EXPECTED["cfg-on-again-log"], observed, "pass" if ok else "fail")


def check_cfg_off_relaunch_log(s: Session) -> dict:
    if s.run("relaunch-off") is None:
        raise NotRun("not needed: no relaunch-off stage")
    if s.relaunch_pid is None:
        raise missing("no --relaunch-pid")
    ok, observed = _logged_per_press(s, "relaunch-off", s.relaunch_pid)
    return result("cfg-off-relaunch-log", EXPECTED["cfg-off-relaunch-log"], observed,
                  "pass" if ok else "not-observed")


def check_cfg_setting_found(s: Session) -> dict:
    answer = (s.setting_found or "").strip().lower()
    if not answer:
        raise missing("no --setting-found answer")
    observed = "person: %s; label \"%s\"" % (answer, s.setting_label or "none given")
    return result("cfg-setting-found", EXPECTED["cfg-setting-found"], observed,
                  "pass" if answer == "yes" else "fail", label=s.setting_label)


def check_cfg_file_written(s: Session) -> dict:
    records = s.config_records()
    after = [r for r in records if r.get("label") == "after-toggle-off"]
    if not after:
        raise missing("no after-toggle-off record in config.jsonl")
    rec = after[-1]
    before = [r for r in records if r.get("label") == "before"]
    value = rec.get(DIALOGUE_KEY[1], "missing")
    changed = ("; file %s since `before`" % ("changed" if before[-1].get("sha256") != rec.get("sha256")
                                            else "unchanged")) if before else ""
    observed = "after-toggle-off at %s: %s=%s, mtime %s%s" % (
        rec.get("utc", "?")[11:], DIALOGUE_KEY[1], value, rec.get("mtime"), changed)
    verdict = {"false": "pass", "true": "not-observed"}.get(value, "fail")
    return result("cfg-file-written", EXPECTED["cfg-file-written"], observed, verdict)


def off_shown(s: Session) -> list:
    """For each town-off press: (press, frame, line, how, dropped, before) of its first new shown
    greeting, frame, line and how None when there is none. `dropped` counts the candidate line reads
    that were a line still on screen from the press's before window (`Session.newness`), `before`
    that window's frame count; a press whose before window holds no frame is not judged (`before`
    0, no hit)."""
    def make():
        presses = s.need_presses("town-off")
        s.frames()
        on_texts = []
        on = s.run("town-on")
        if on is not None:
            on_texts = [l.text for l in s.speaker_lines(on.start, on.end or _dt.datetime.max.replace(tzinfo=common.UTC),
                                                        s.pid)]
        out = []
        for p in presses:
            frames, is_new, before = s.newness(p - _td(GREET_LOG_BEFORE_S), p + _td(GREET_FRAME_AFTER_S))
            hit, dropped = None, 0
            if not before:
                frames = []
            for f in frames:
                for l in s.lines(f):
                    who = next((n for n in s.speakers if prefixed_by(l.text, n)), None)
                    if who is not None:
                        how = "%s-prefixed" % who
                    elif on_texts and max(screen.fuzzy_score(t, l.text) for t in on_texts) >= OCR_SCORE:
                        how = "repeats a town-on line"
                    else:
                        continue
                    if not is_new(f, l.text):
                        dropped += 1
                    elif hit is None:
                        hit = (f, l, how)
            out.append((p,) + (hit or (None, None, None)) + (dropped, before))
        return out
    return s._once("off-shown", make)


def check_cfg_off_shown(s: Session) -> dict:
    shown = off_shown(s)
    hits = [x for x in shown if x[1] is not None]
    unjudged = sum(1 for x in shown if not x[5])
    parts = [("press %s: %s" % (_hms(p), NOT_JUDGED)) if not nb else "press %s: %s; %d candidate read(s) %s" % (
             _hms(p), ("%s, first frame %s" % (how, _rel(f.utc, p))) if f else "none shown", dropped, DROPPED % nb)
             for p, f, _, how, dropped, nb in shown]
    observed = "%d of %d presses shown, %d not judged; %s" % (len(hits), len(shown), unjudged, "; ".join(parts))
    if len(hits) >= SHOWN_MIN:
        verdict = "pass"
    elif len(hits) + unjudged < SHOWN_MIN:
        verdict = "fail"
    else:
        verdict = _unjudged_presses(unjudged)
    return result("cfg-off-shown", EXPECTED["cfg-off-shown"], observed, verdict)


def check_cfg_off_log(s: Session, done: dict) -> dict:
    expected = EXPECTED["cfg-off-log"]
    why = _controls(done, ("cfg-on-log", "cfg-off-shown"))
    if why:
        return result("cfg-off-log", expected, "not computed: " + why, "not-run (%s)" % why)
    log = s.log()
    parts, logged = [], 0
    for p, f, l, *_ in off_shown(s):
        if f is None:
            continue
        a, b = p - _td(GREET_LOG_BEFORE_S), p + _td(GREET_LOG_AFTER_S)
        lines = s.greetings(p, s.pid)
        repeat = not lines and log.repeats_logged(l.text, a, b, screen.fuzzy_score, OCR_SCORE)
        hit = bool(lines) or repeat
        logged += hit
        parts.append("press %s: %s" % (_hms(p), ("%s %s" % (lines[0].speaker, _rel(lines[0].utc, p))) if lines
                                       else "a logged line repeats it" if repeat else "nothing logged"))
    shown = sum(1 for x in off_shown(s) if x[1] is not None)
    observed = "%d of %d shown greetings logged; %s" % (logged, shown, "; ".join(parts))
    return result("cfg-off-log", expected, observed, "pass" if logged else "not-observed")


def dialogue_lines(s: Session) -> dict:
    """dlg-window's measurement: the new lines inside the window, and the NPC's score."""
    def make():
        o, c = s.dialogue()
        inside = s.window("bottom", o + _td(DLG_EDGE_S), c - _td(DLG_EDGE_S))
        before = s.window("bottom", o - _td(DLG_BEFORE_FROM_S), o - _td(DLG_BEFORE_TO_S))
        earlier = [screen.normalize(t) for t in s.texts(before)]
        clusters = []
        for f in inside:
            for l in s.lines(f):
                n = screen.normalize(l.text)
                if len(n.split()) < DLG_MIN_WORDS:
                    continue
                c_ = next((c_ for c_ in clusters if same_line(c_["norm"], n)), None)
                if c_ is None:
                    c_ = {"norm": n, "text": l.text, "frames": set(), "boxes": []}
                    clusters.append(c_)
                c_["frames"].add(screen.path_key(f.path))
                c_["boxes"].append(box_fraction(f, l.box))
        new = [c_ for c_ in clusters if len(c_["frames"]) >= DLG_MIN_FRAMES
               and not any(same_line(c_["norm"], e) for e in earlier)]
        name = screen.best_score(s.npc, s.texts(inside))
        return {"open": o, "close": c, "inside": inside, "before": before, "new": new, "name": name,
                "box": union(b for c_ in new for b in c_["boxes"])}
    return s._once("dialogue-lines", make)


def check_dlg_window(s: Session) -> dict:
    d = dialogue_lines(s)
    new = d["new"]
    prefixed = sum(1 for c_ in new if any(prefixed_by(c_["text"], n) for n in s.speakers))
    observed = ("open %s, close %s (%.1f s); %d new line(s) of %d+ words in %d+ frames (%d speaker-prefixed); "
                "%s %.2f; frames %d inside, %d before; box %s") % (
        _hms(d["open"]), _hms(d["close"]), (d["close"] - d["open"]).total_seconds(), len(new), DLG_MIN_WORDS,
        DLG_MIN_FRAMES, prefixed, s.npc, d["name"], len(d["inside"]), len(d["before"]), _box_text(d["box"]))
    ok = len(new) >= DLG_MIN_LINES and d["name"] >= OCR_SCORE
    return result("dlg-window", EXPECTED["dlg-window"], observed, "pass" if ok else "fail",
                  lines=[s.mask(c_["text"]) for c_ in new], box=d["box"])


def check_dlg_option(s: Session) -> dict:
    o, c = s.dialogue()
    if not s.dialogue_option:
        raise missing("no --dialogue-option")
    frames = s.window("bottom", o, c)
    score = screen.best_score(s.dialogue_option, s.texts(frames))
    observed = "the topic the person named: %.2f in %d frames" % (score, len(frames))
    return result("dlg-option", EXPECTED["dlg-option"], observed, "pass" if score >= OCR_SCORE else "fail")


def check_dlg_subtitle(s: Session) -> dict:
    o, c = s.dialogue()
    frames, is_new, before = s.newness(o, c + _td(DLG_AFTER_S))
    if not before:
        observed = "open %s: %s; %d frames in the window" % (_hms(o), NOT_JUDGED, len(frames))
        return result("dlg-subtitle", EXPECTED["dlg-subtitle"], observed, UNJUDGED_WINDOW)
    with_line, dropped = [], 0
    for f in frames:
        new = False
        for l in s.lines(f):
            if not prefixed_by(l.text, s.npc):
                continue
            if is_new(f, l.text):
                new = True
            else:
                dropped += 1
        if new:
            with_line.append(f)
    observed = "%d of %d frames hold a new %s-prefixed line%s; %d candidate read(s) %s" % (
        len(with_line), len(frames), s.npc, (", first %s after open" % _rel(with_line[0].utc, o)) if with_line else "",
        dropped, DROPPED % before)
    return result("dlg-subtitle", EXPECTED["dlg-subtitle"], observed, "pass" if with_line else "fail")


def check_dlg_log(s: Session, done: dict) -> dict:
    expected = EXPECTED["dlg-log"]
    why = _controls(done, ("cfg-on-log", "dlg-window"))
    if why:
        return result("dlg-log", expected, "not computed: " + why, "not-run (%s)" % why)
    d = dialogue_lines(s)
    log = s.log()
    o = d["open"]
    # Quest dialogue is logged after the open press's greeting window, (open + 1, close + 2].
    a, b = o + _td(GREET_LOG_AFTER_S), d["close"] + _td(DLG_AFTER_S)
    at_open, pool = dialogue_greetings(s, o)
    greetings = [l.text for l in at_open] + pool

    def is_greeting(text: str) -> bool:
        return any(screen.fuzzy_score(g, text) >= OCR_SCORE or screen.fuzzy_score(text, g) >= OCR_SCORE
                   for g in greetings)

    later = [l for l in log.npc(s.npc) if a < l.utc <= b]
    quest = [l for l in later if not is_greeting(l.text)]
    unmatched = sum(1 for l in at_open if not any(screen.fuzzy_score(t, l.text) >= OCR_SCORE
                                                  or screen.fuzzy_score(l.text, t) >= OCR_SCORE for t in pool))

    def repeated(text: str) -> bool:
        """A logged line in (a, b] that repeats `text` and is not a greeting."""
        def score(x, y):
            said = x if y == text else y   # repeats_logged scores the logged line both ways round
            return 0.0 if is_greeting(said) else screen.fuzzy_score(x, y)
        return log.repeats_logged(text, a + _dt.timedelta(microseconds=1), b, score, OCR_SCORE)

    repeats = sum(1 for c_ in d["new"] if repeated(c_["text"]))
    observed = ("%d %s line(s) logged in (open + 1 s, close + 2 s] that are not a greeting, %d set aside as "
                "greetings; %d %s line(s) at the open press [open - 6 s, open + 1 s], never counted, %d matching "
                "nothing in the town pool (%d line(s)); %d of %d window lines repeated by a logged line that is "
                "not a greeting") % (len(quest), s.npc, len(later) - len(quest), len(at_open), s.npc, unmatched,
                                     len(pool), repeats, len(d["new"]))
    return result("dlg-log", expected, observed, "pass" if quest or repeats else "not-observed")


def dialogue_greetings(s: Session, o) -> tuple[list, list[str]]:
    """(the dialogue NPC's lines logged in the open press's greeting window [o - 6, o + 1], the texts
    it logged in the town stages' runs): what dlg-log sets aside as greetings."""
    lines = s.log().npc(s.npc)
    at_open = [l for l in lines if o - _td(GREET_LOG_BEFORE_S) <= l.utc <= o + _td(GREET_LOG_AFTER_S)]
    pool = []
    for name in TOWN_STAGES:
        r = s.run(name)
        if r is not None:
            pool += [l.text for l in lines if r.start <= l.utc and (r.end is None or l.utc < r.end)]
    return at_open, pool


def _bar_stats(s: Session, frames) -> tuple[int, int, float, float | None, float | None]:
    """(frames with the name, other OCR lines, share, median luma mean, median luma std)."""
    with_name, others = 0, 0
    for f in frames:
        lines = s.lines(f)
        named = [l for l in lines if screen.fuzzy_score(s.boss, l.text) >= OCR_SCORE]
        with_name += bool(named)
        others += len(lines) - len(named)
    share = with_name / len(frames) if frames else 0.0
    means = [f.luma_mean for f in frames if f.luma_mean is not None]
    stds = [f.luma_std for f in frames if f.luma_std is not None]
    return (with_name, others, share, statistics.median(means) if means else None,
            statistics.median(stds) if stds else None)


def _luma(mean, std) -> str:
    return "luma %s/%s" % ("n/a" if mean is None else "%.1f" % mean, "n/a" if std is None else "%.1f" % std)


def bar_control_window(s: Session) -> tuple:
    p = s.boss_presses()
    return p[0] + _td(BAR_EDGE_S), p[1] - _td(BAR_EDGE_S)


def check_bar_control(s: Session) -> dict:
    a, b = bar_control_window(s)
    frames = s.window("top", a, b)
    n, others, share, mean, std = _bar_stats(s, frames)
    observed = "%d of %d top-band frames (%.2f); %s; other lines %d; black %d" % (
        n, len(frames), share, _luma(mean, std), others, sum(1 for f in frames if screen.is_black(f)))
    ok = n >= BAR_MIN_FRAMES and share >= BAR_SHARE
    return result("bar-control", EXPECTED["bar-control"], observed, "pass" if ok else "fail",
                  share=share, luma_mean=mean, luma_std=std)


def check_bar_panel(s: Session, done: dict, panel: str) -> dict:
    name = "bar-" + panel
    expected = expected_of(name)
    p = s.boss_presses()
    why = _controls(done, ("bar-control",))
    if why:
        return result(name, expected, "not computed: " + why, "not-run (%s)" % why)
    k = s.panels.index(panel)
    o, c = p[1 + 2 * k], p[2 + 2 * k]
    frames = s.window("top", o + _td(BAR_EDGE_S), c - _td(BAR_EDGE_S))
    if not frames:
        raise missing("no top-band frame in [open + 0.5, close - 0.5] of %s" % panel)
    n, others, share, mean, std = _bar_stats(s, frames)
    ctl = done["bar-control"]["details"]
    observed = "%d of %d top-band frames (%.2f); %s against bar-control's %s; other lines %d; open %.1f s" % (
        n, len(frames), share, _luma(mean, std), _luma(ctl.get("luma_mean"), ctl.get("luma_std")), others,
        (c - o).total_seconds())
    return result(name, expected, observed, "pass" if share >= BAR_SHARE else "fail",
                  share=share, luma_mean=mean, luma_std=std, other_lines=others)


def check_bar_away(s: Session, done: dict) -> dict:
    expected = EXPECTED["bar-away"]
    p = s.boss_presses()
    why = _controls(done, ("bar-control",))
    if why:
        return result("bar-away", expected, "not computed: " + why, "not-run (%s)" % why)
    away = p[-1]
    frames = s.window("top", away + _td(AWAY_FROM_S), away + _td(AWAY_TO_S))
    if not frames:
        raise missing("no top-band frame in [away + 1, away + 6]")
    n, _, _, _, _ = _bar_stats(s, frames)
    observed = "the name in %d of %d top-band frames" % (n, len(frames))
    return result("bar-away", expected, observed, "pass" if n == 0 else "fail")


def remarks(s: Session) -> dict:
    """remark-shown's and remark-kill's finds."""
    def make():
        presses = [p.utc for p in s.need_run("remark").presses]
        s.need_character()
        s.frames()
        def find(a, b):
            frames, is_new, before = s.newness(a, b)
            if not before:
                return None, len(frames), 0, 0   # not judged: no hit
            hit, dropped = s.first_remark(frames, is_new)
            return hit, len(frames), dropped, before
        shown = [(p,) + find(p - _td(GREET_LOG_BEFORE_S), p + _td(GREET_FRAME_AFTER_S)) for p in presses[1:]]
        kill = None
        if presses:
            kill = (presses[0],) + find(presses[0] - _td(KILL_BEFORE_S), presses[0] + _td(KILL_AFTER_S))
        return {"presses": presses, "shown": shown, "kill": kill}
    return s._once("remarks", make)


def check_remark_shown(s: Session) -> dict:
    r = remarks(s)
    if not r["shown"]:
        raise missing("no remark press after the first")
    parts = [("press %s: %s" % (_hms(p), NOT_JUDGED)) if not nb else "press %s: %s; %d candidate read(s) %s" % (
             _hms(p), ("first frame %s" % _rel(hit[0].utc, p)) if hit else "none in %d frames" % n, dropped,
             DROPPED % nb)
             for p, hit, n, dropped, nb in r["shown"]]
    found = sum(1 for _, hit, *_ in r["shown"] if hit)
    unjudged = sum(1 for *_, nb in r["shown"] if not nb)
    observed = "%d of %d presses with a line prefixed by %s, %d not judged; %s" % (
        found, len(r["shown"]), CHARACTER_MASK, unjudged, "; ".join(parts))
    verdict = "pass" if found else "fail" if not unjudged else _unjudged_presses(unjudged)
    return result("remark-shown", EXPECTED["remark-shown"], observed, verdict)


def check_remark_kill(s: Session) -> dict:
    r = remarks(s)
    if r["kill"] is None:
        raise missing("no press in stage remark")
    p, hit, n, dropped, nb = r["kill"]
    if not nb:
        observed = "kill press %s: %s; %d frames in the window" % (_hms(p), NOT_JUDGED, n)
        return result("remark-kill", EXPECTED["remark-kill"], observed, UNJUDGED_WINDOW, delay_s=None)
    if hit:
        observed = "kill press %s: first frame with a new line prefixed by %s %s after it (%d frames)" % (
            _hms(p), CHARACTER_MASK, _rel(hit[0].utc, p), n)
    else:
        observed = "kill press %s: no new line prefixed by %s in %d frames" % (_hms(p), CHARACTER_MASK, n)
    observed += "; %d candidate read(s) %s" % (dropped, DROPPED % nb)
    return result("remark-kill", EXPECTED["remark-kill"], observed, "pass" if hit else "fail",
                  delay_s=(hit[0].utc - p).total_seconds() if hit else None)


def check_remark_log(s: Session, done: dict) -> dict:
    expected = EXPECTED["remark-log"]
    why = _controls(done, ("cfg-on-log",))
    r = None
    if not why:
        try:
            r = remarks(s)
        except NotRun as err:
            why = err.why
    found = []
    if r is not None:
        for p, hit, *_ in r["shown"] + ([r["kill"]] if r["kill"] else []):
            if hit is None:
                continue
            f, l = hit
            rest = screen.normalize(after_prefix(l.text))
            if not any(same_line(rest, screen.normalize(after_prefix(x[1].text))) and
                       abs((f.utc - x[0].utc).total_seconds()) < REMARK_LOG_BEFORE_S for x in found):
                found.append((f, l))
        if not found:
            why = "control: no remark found"
    if why:
        return result("remark-log", expected, "not computed: " + why, "not-run (%s)" % why)
    log = s.log()
    parts, logged = [], 0
    for f, l in sorted(found, key=lambda x: x[0].utc):
        a, b = f.utc - _td(REMARK_LOG_BEFORE_S), f.utc + _td(REMARK_LOG_AFTER_S)
        own = [x for x in log.npc(s.character) if a <= x.utc <= b]
        repeat = log.repeats_logged(after_prefix(l.text), a, b, screen.fuzzy_score, OCR_SCORE)
        hit = bool(own) or repeat
        logged += hit
        parts.append("remark at %s: %s" % (_hms(f.utc), "logged" if hit else "not logged"))
    observed = "%d of %d remarks logged; %s" % (logged, len(found), "; ".join(parts))
    return result("remark-log", expected, observed, "pass" if logged == len(found) else "not-observed",
                  remarks=[s.mask(l.text) for _, l in found])


PLAIN = {
    "control": check_control, "marker": check_marker, "i3-rehearsal": check_i3_rehearsal,
    "i3-town": check_i3_town, "cfg-on-log": check_cfg_on_log, "cfg-setting-found": check_cfg_setting_found,
    "cfg-file-written": check_cfg_file_written, "cfg-off-shown": check_cfg_off_shown,
    "cfg-on-again-log": check_cfg_on_again_log, "cfg-off-relaunch-log": check_cfg_off_relaunch_log,
    "dlg-window": check_dlg_window, "dlg-option": check_dlg_option, "dlg-subtitle": check_dlg_subtitle,
    "bar-control": check_bar_control, "remark-shown": check_remark_shown, "remark-kill": check_remark_kill,
}
# A check whose verdict reads others: (function(s, done), the checks it reads).
DEPENDENT = {
    "cfg-off-log": (check_cfg_off_log, ("cfg-on-log", "cfg-off-shown")),
    "dlg-log": (check_dlg_log, ("cfg-on-log", "dlg-window")),
    "bar-away": (check_bar_away, ("bar-control",)),
    "remark-log": (check_remark_log, ("cfg-on-log", "remark-shown", "remark-kill")),
}


def _dispatch(name: str):
    if name in DEPENDENT:
        return DEPENDENT[name]
    if name in PLAIN:
        return PLAIN[name], ()
    if name.startswith("bar-"):
        panel = name[len("bar-"):]
        return (lambda s, done: check_bar_panel(s, done, panel)), ("bar-control",)
    raise KeyError(name)


def compute(s: Session, names) -> tuple[dict[str, dict], list[str]]:
    """Every check in `names` and what they depend on. Returns (results, errors)."""
    done: dict[str, dict] = {}
    errors: list[str] = []

    def one(name):
        if name in done:
            return done[name]
        try:
            fn, deps = _dispatch(name)
            for d in deps:
                one(d)
            r = fn(s, done) if deps or name in DEPENDENT else fn(s)
        except NotRun as err:
            r = result(name, expected_of(name), "not computed: " + err.why, "not-run (%s)" % err.why)
        except Exception as err:  # noqa: BLE001 - reported per check, then exit 1
            errors.append("check %s failed: %s: %s" % (name, type(err).__name__, s.mask(str(err))))
            errors.append(s.mask(traceback.format_exc().rstrip()))
            r = result(name, expected_of(name), "internal error: %s" % s.mask(str(err)), "not-run (analysis error)")
            r["error"] = True
        done[name] = r
        return r

    for name in names:
        one(name)
    return done, errors


def parse_only(values, panels) -> list[str]:
    allowed = check_names(panels)
    if not values:
        return allowed
    names = [n.strip() for v in values for n in v.split(",") if n.strip()]
    for n in names:
        if n not in allowed:
            raise ToolError("unknown check %r for --only: expected some of %s" % (n, ", ".join(allowed)))
    return [n for n in allowed if n in names]


def analyse(live: Path, pid: int, only=None, relaunch_pid: int | None = None, character: str | None = None,
            boss: str | None = None, npcs=(), dialogue_option: str | None = None,
            setting_label: str | None = None, setting_found: str | None = None, panels=None,
            confirmed: str | None = None, tz: _dt.tzinfo | None = None,
            ocr_runner: Callable = screen.run_ocr_ps1, out=None) -> tuple[int, list[dict]]:
    """Compute, print and write. Returns (exit code, the printed results)."""
    out = out or sys.stdout
    live = Path(live)
    common.refuse_inside_git(live, "analysis")
    if not live.is_dir():
        raise ToolError("no live folder at %s" % live)
    panels = parse_panels(panels)
    names = parse_only(only, panels)
    s = Session(live, pid, relaunch_pid=relaunch_pid, character=character, boss=boss or DEFAULT_BOSS,
                npcs=npcs, dialogue_option=dialogue_option, setting_label=setting_label,
                setting_found=setting_found, panels=panels, confirmed=confirmed, tz=tz, ocr_runner=ocr_runner)
    deps = set(names)
    for n in names:
        try:
            deps.update(_dispatch(n)[1])
        except KeyError:
            pass
    prefetch(s, [n for n in check_names(panels) if n in deps])
    done, errors = compute(s, names)
    shown = [done[n] for n in names]
    for r in shown:
        print(capture_line(r), file=out)

    try:
        ignored = s.runs()[1]
    except NotRun:
        ignored = {}
    path = live / common.ANALYSIS_JSON
    previous = {}
    if only and path.is_file():
        try:
            previous = {c["name"]: c for c in common.read_json(path).get("checks", [])}
        except (ValueError, KeyError, TypeError, AttributeError):
            previous = {}
    merged = dict(previous)
    merged.update({r["name"]: r for r in shown})
    order = check_names(panels)
    common.write_json(path, {
        "created_utc": common.format_utc(common.now_utc()),
        "pid": pid,
        "relaunch_pid": relaunch_pid,
        "panels": list(panels),
        "only": names if only else None,
        "stage_runs_ignored": ignored,
        "checks": [merged[n] for n in order if n in merged] + [merged[n] for n in merged if n not in order],
    })
    total = sum(ignored.values())
    print("screentext: stage runs ignored: %d%s" % (
        total, (" (%s)" % ", ".join("%s %d" % kv for kv in sorted(ignored.items()))) if total else ""), file=out)
    print("screentext: wrote %s" % path.name, file=out)
    for e in errors:
        print("screentext: error: " + e, file=sys.stderr)
    return (1 if errors else 0), shown


# --------------------------------------------------------------------------
# The command line

def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(prog="screentext", description="Live 8: stage stamps, the client config and "
                                "the session analysis.")
    sub = p.add_subparsers(dest="command", required=True, metavar="{stage,config,analyse}")
    st = sub.add_parser("stage", help="stamp a stage into L\\stages.jsonl")
    st.add_argument("--live", type=Path, required=True)
    st.add_argument("--name", required=True, help="one of " + ", ".join(STAGES))
    cf = sub.add_parser("config", help="read the client config into L\\config.jsonl (never writes it)")
    cf.add_argument("--live", type=Path, required=True)
    cf.add_argument("--label", required=True)
    cf.add_argument("--ini", type=Path, default=None, help="default: Documents\\" + str(CONFIG_RELATIVE))
    an = sub.add_parser("analyse", help="one capture line per check; writes L\\analysis.json")
    an.add_argument("--live", type=Path, required=True)
    an.add_argument("--pid", type=int, required=True, help="the game's pid, from the lease")
    an.add_argument("--relaunch-pid", type=int, default=None, help="the relaunched game's pid (stage relaunch-off)")
    an.add_argument("--character", default=None, help="the character's name; never printed")
    an.add_argument("--boss", default=DEFAULT_BOSS, help="the boss's name as the bar shows it")
    an.add_argument("--npc", action="append", default=[], help="another speaker; repeatable")
    an.add_argument("--dialogue-option", default=None, help="the topic the person chose")
    an.add_argument("--setting-label", default=None, help="the option's label, as the person gave it")
    an.add_argument("--setting-found", choices=("yes", "no"), default=None)
    an.add_argument("--panels", default=None, help="comma-separated; default " + ",".join(DEFAULT_PANELS))
    an.add_argument("--confirmed", default=None, metavar="yes|no",
                    help="the person's answer: did the test press beep and change nothing in the game?")
    an.add_argument("--only", nargs="+", default=None, metavar="NAME",
                    help="checks to compute and print (comma- or space-separated)")
    return p


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    try:
        if args.command == "stage":
            rec = stage(args.live, args.name)
            print("stage: %s at %s" % (rec["name"], rec["utc"]))
            return 0
        if args.command == "config":
            print(config_line(config(args.live, args.label, args.ini)))
            return 0
        code, _ = analyse(args.live, args.pid, only=args.only, relaunch_pid=args.relaunch_pid,
                          character=args.character, boss=args.boss, npcs=args.npc,
                          dialogue_option=args.dialogue_option, setting_label=args.setting_label,
                          setting_found=args.setting_found, panels=args.panels, confirmed=args.confirmed)
        return code
    except ToolError as err:
        print("screentext: error: %s" % err, file=sys.stderr)
        return 1
    except (OSError, ValueError, json.JSONDecodeError) as err:
        print("screentext: error: %s" % err, file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
