"""What every outside-the-game instrument shares.

- **The refusal.** Every writer refuses an output path inside a git checkout
  (`refuse_inside_git`, through `poe2db.git_tree_of`): captures, recordings,
  traces and anything decoded from the game's files are never tracked.
- **The live folder.** `default_live_dir()` is
  `%USERPROFILE%\\quest-tracker-live-5\\beira-defeat\\`; `SUBFOLDERS`, the
  file names and `stop_file()` are the layout every instrument agrees on.
- **Times.** UTC ISO 8601 with milliseconds and `Z`
  (`format_utc` / `parse_utc`).
- **JSON lines.** `append_jsonl` / `read_jsonl`, one object per line.
- **The log's counter.** A `Client.txt` line carries a local wall-clock
  second and a millisecond counter: the milliseconds since the system
  started, a 32-bit value that wraps after about 49.7 days. A clock anchor
  (`{"utc", "tick_ms"}` in `clock.jsonl`, `tick_ms` from `GetTickCount64`)
  ties that counter to UTC. `log_time_utc` reconstructs the full tick count
  near the wall clock, converts it through the nearest anchor, and falls back
  to the wall-clock second when the two disagree by more than
  `COUNTER_TOLERANCE_S`. It says which one it used.
"""
from __future__ import annotations

import datetime as _dt
import json
import os
import re
import sys
import time
from pathlib import Path
from typing import NamedTuple

from tools.quest_tracker.poe2db import git_tree_of

UTC = _dt.timezone.utc

GAME_DIR = Path(r"C:\Program Files (x86)\Grinding Gear Games\Path of Exile 2")
DEFAULT_CLIENT_TXT = GAME_DIR / "logs" / "Client.txt"
DEFAULT_GGPK = GAME_DIR / "Content.ggpk"

# The live folder's layout (the workorder's "Live folder contract").
SUBFOLDERS = (
    "ggpk",
    "i1/rehearsal",
    "i2/rehearsal",
    "i3/frames/top",
    "i3/frames/bottom",
    "i3/rehearsal",
    "ref",
    "tools",
)
SESSION_JSON = "session.json"
CLOCK_JSONL = "clock.jsonl"
MARKERS_TXT = "markers.txt"
ANALYSIS_JSON = "analysis.json"
INSTRUMENTS = ("i1", "i2", "i3", "marker")

# A 32-bit millisecond counter wraps here; GetTickCount64 does not.
TICK_WRAP = 1 << 32
# How far, in seconds, a counter's time may sit outside the wall-clock second
# its line carries before the line falls back to the wall clock.
COUNTER_TOLERANCE_S = 1.5
# Samples per clock anchor; the one read in the narrowest time window is kept.
ANCHOR_SAMPLES = 5

# Client.txt: `YYYY/MM/DD HH:MM:SS <ms counter> <hex> [<LEVEL> Client <pid>] <message>`.
LOG_PREFIX_RE = re.compile(r"^(\d{4})/(\d{2})/(\d{2}) (\d{2}):(\d{2}):(\d{2}) (\d+) ")
_UTC_RE = re.compile(r"^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})(?:\.(\d{1,6}))?Z$")


class ToolError(Exception):
    """A refusal or failure the command line reports in one line, exit 1."""


# --------------------------------------------------------------------------
# Refusing a git checkout, and the live folder

def refuse_inside_git(path: Path, what: str) -> None:
    tree = git_tree_of(Path(path))
    if tree is not None:
        raise ToolError(
            "refusing to write the %s at %s: it is inside the git checkout %s. Captures, "
            "recordings, traces and anything decoded from the game's files stay outside "
            "every repository; pass a folder elsewhere." % (what, path, tree)
        )


def default_live_dir() -> Path:
    home = os.environ.get("USERPROFILE") or str(Path.home())
    return Path(home) / "quest-tracker-live-5" / "beira-defeat"


def stop_file(live: Path, instrument: str) -> Path:
    """`L\\stop-<instrument>`: its existence asks that instrument to end."""
    if instrument not in INSTRUMENTS:
        raise ToolError("unknown instrument %r: expected one of %s"
                        % (instrument, ", ".join(INSTRUMENTS)))
    return Path(live) / ("stop-" + instrument)


# --------------------------------------------------------------------------
# UTC timestamps

def now_utc() -> _dt.datetime:
    return _dt.datetime.now(UTC)


def utc_from_epoch_ns(ns: int) -> _dt.datetime:
    return _dt.datetime(1970, 1, 1, tzinfo=UTC) + _dt.timedelta(microseconds=ns // 1000)


def format_utc(when: _dt.datetime) -> str:
    """`2026-10-26T10:28:17.123Z`. A naive datetime is refused, never guessed."""
    if when.tzinfo is None:
        raise ValueError("format_utc needs an aware datetime, got a naive one: %r" % (when,))
    u = when.astimezone(UTC)
    return "%04d-%02d-%02dT%02d:%02d:%02d.%03dZ" % (
        u.year, u.month, u.day, u.hour, u.minute, u.second, u.microsecond // 1000)


def parse_utc(text: str) -> _dt.datetime:
    m = _UTC_RE.match(text.strip())
    if not m:
        raise ValueError("not a UTC timestamp of the form YYYY-MM-DDTHH:MM:SS.mmmZ: %r" % (text,))
    year, month, day, hour, minute, second = (int(g) for g in m.groups()[:6])
    frac = m.group(7) or ""
    micro = int(frac.ljust(6, "0")) if frac else 0
    return _dt.datetime(year, month, day, hour, minute, second, micro, tzinfo=UTC)


# --------------------------------------------------------------------------
# JSON and JSON lines

def append_jsonl(path: Path, record: dict) -> None:
    path = Path(path)
    refuse_inside_git(path, "JSON-lines file")
    with open(path, "a", encoding="utf-8", newline="\n") as f:
        f.write(json.dumps(record, ensure_ascii=False) + "\n")
        f.flush()


def read_jsonl(path: Path) -> list[dict]:
    """Every object in `path`, in order.

    A last line without its newline that does not parse is a writer stopped
    mid-line and is dropped; any other line that does not parse is an error
    naming the file and line.
    """
    path = Path(path)
    text = path.read_text(encoding="utf-8")
    lines = text.split("\n")
    out = []
    for i, line in enumerate(lines):
        if not line.strip():
            continue
        try:
            out.append(json.loads(line))
        except json.JSONDecodeError as err:
            if i == len(lines) - 1:
                break
            raise ToolError("%s:%d is not a JSON line: %s" % (path, i + 1, err))
    return out


def write_json(path: Path, obj) -> None:
    path = Path(path)
    refuse_inside_git(path, "JSON file")
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write(json.dumps(obj, ensure_ascii=False, indent=2) + "\n")


def read_json(path: Path):
    return json.loads(Path(path).read_text(encoding="utf-8"))


# --------------------------------------------------------------------------
# Clock anchors and the log's counter

class Anchor(NamedTuple):
    utc: _dt.datetime
    tick_ms: int


def read_tick_ms() -> int:
    """`GetTickCount64`: milliseconds since the system started."""
    if sys.platform != "win32":
        raise ToolError("clock anchors read GetTickCount64, which needs Windows")
    import ctypes
    fn = ctypes.windll.kernel32.GetTickCount64
    fn.restype = ctypes.c_uint64
    fn.argtypes = []
    return int(fn())


def sample_anchor(samples: int = ANCHOR_SAMPLES, now_ns=time.time_ns, tick_ms=read_tick_ms) -> dict:
    """One clock anchor, `{"utc": ..., "tick_ms": ...}`.

    Each sample reads the wall clock, the tick count, then the wall clock
    again; the sample with the narrowest window wins, and its UTC is that
    window's midpoint.
    """
    best = None
    for _ in range(max(1, samples)):
        before = now_ns()
        tick = tick_ms()
        after = now_ns()
        width = after - before
        if best is None or width < best[0]:
            best = (width, before + width // 2, tick)
    return {"utc": format_utc(utc_from_epoch_ns(best[1])), "tick_ms": int(best[2])}


def anchor_of(record: dict) -> Anchor:
    return Anchor(parse_utc(record["utc"]), int(record["tick_ms"]))


def load_anchors(live: Path) -> list[Anchor]:
    """The anchors in `L\\clock.jsonl`, or none when the file is missing."""
    path = Path(live) / CLOCK_JSONL
    if not path.is_file():
        return []
    return [anchor_of(r) for r in read_jsonl(path)]


def split_log_prefix(line: str) -> tuple[_dt.datetime, int] | None:
    """A `Client.txt` line's local wall-clock second (naive) and its counter."""
    m = LOG_PREFIX_RE.match(line)
    if not m:
        return None
    year, month, day, hour, minute, second = (int(g) for g in m.groups()[:6])
    return _dt.datetime(year, month, day, hour, minute, second), int(m.group(7))


def local_to_utc(wall: _dt.datetime, tz: _dt.tzinfo | None = None) -> _dt.datetime:
    """A naive local time as UTC: through `tz` when given, else this machine's zone."""
    if wall.tzinfo is not None:
        return wall.astimezone(UTC)
    if tz is not None:
        return wall.replace(tzinfo=tz).astimezone(UTC)
    return wall.astimezone(UTC)


def counter_to_utc(counter: int, anchor: Anchor, near: _dt.datetime) -> _dt.datetime:
    """The UTC of a 32-bit millisecond counter, through `anchor`.

    The counter only knows its value modulo 2**32, so the full tick count is
    the one with that remainder nearest to the tick count `near` implies.
    """
    expected = anchor.tick_ms + (near - anchor.utc) / _dt.timedelta(milliseconds=1)
    raw = int(counter) % TICK_WRAP
    tick = raw + round((expected - raw) / TICK_WRAP) * TICK_WRAP
    return anchor.utc + _dt.timedelta(milliseconds=tick - anchor.tick_ms)


def log_time_utc(wall: _dt.datetime, counter: int | None, anchors,
                 tz: _dt.tzinfo | None = None) -> tuple[_dt.datetime, str]:
    """A log line's UTC and its source, `"counter"` or `"wall"`.

    `wall` is the line's local second. The counter is converted through the
    anchor nearest that second; its result is kept unless it lies more than
    `COUNTER_TOLERANCE_S` outside the wall-clock second, in which case (or
    with no counter or no anchor) the line gets the start of its wall-clock
    second.
    """
    wall_utc = local_to_utc(wall, tz)
    anchors = [a if isinstance(a, Anchor) else anchor_of(a) for a in (anchors or [])]
    if counter is None or not anchors:
        return wall_utc, "wall"
    anchor = min(anchors, key=lambda a: abs((a.utc - wall_utc).total_seconds()))
    utc = counter_to_utc(counter, anchor, wall_utc)
    offset = (utc - wall_utc).total_seconds()
    outside = -offset if offset < 0 else max(0.0, offset - 1.0)
    if outside > COUNTER_TOLERANCE_S:
        return wall_utc, "wall"
    return utc, "counter"
