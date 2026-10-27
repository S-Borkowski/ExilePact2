"""`Client.txt` for the session analysis: read-only, from the session's offset.

The analysis needs a few kinds of line from the log the game writes, each with
its time in UTC:

- the session marker, `***** LOG FILE OPENING *****` (no header, no counter,
  so its time is its local wall-clock second);
- the header's pid, counted per pid (`[<LEVEL> Client <pid>]`);
- area lines, `Generating level <n> area "<code>" with seed <s>`;
- NPC lines, `<Speaker>: <text>`, for the speakers in `NPC_SPEAKERS` only;
- the reward line, `: <character> has received <stat>.`;
- focus lines, `[WINDOW] Lost focus` and `[WINDOW] Gained focus`;
- death lines, `: <character> has been slain.`.

Line shapes are `docs/RUNTIME_DATA_MODELS.md` § "Client log (Client.txt)".
Each line's UTC comes from its millisecond counter through the session's
clock anchors, or from its wall-clock second when the counter disagrees or
no anchor exists (`common.log_time_utc`); every line says which.

Chat never leaves this module. A line whose speaker starts with a chat prefix
(`#`, `$`, `%`, `@`, `&`), and a `Name: text` line whose speaker is not an
allowlisted NPC (local chat has that shape too), is not returned. Character
names are masked: the names the log gives the player (the reward and
level-up lines) become `<player>`, any other `<character>`. The one use of
the remaining lines' text is `ClientLog.repeats_logged`, which answers only
yes or no to "does this screen line repeat a line logged near this time?".

Developer-only, stdlib only; the file is opened read-only and never written.
"""
from __future__ import annotations

import datetime as _dt
import re
from pathlib import Path
from typing import Callable, NamedTuple

from tools.quest_tracker.outside import common

BEIRA = "Beira of the Rotten Pack"
UNA = "Una"
# Clearfell Encampment's vendors (the research note's "Town dialogue").
TOWN_SPEAKERS = ("Una", "Finn", "Renly", "Mortimer")
NPC_SPEAKERS = (BEIRA,) + TOWN_SPEAKERS
CHAT_PREFIXES = "#$%@&"
PLAYER_MASK = "<player>"
OTHER_MASK = "<character>"

_OPENING_RE = re.compile(r"^(\d{4})/(\d{2})/(\d{2}) (\d{2}):(\d{2}):(\d{2}) \*+ LOG FILE OPENING \*+")
_OPENING_MARK = b"LOG FILE OPENING"
_HEADER_RE = re.compile(r"^[0-9A-Fa-f]+ \[(\w+) Client (\d+)\] ?(.*)$")
_AREA_RE = re.compile(r'^Generating level (\d+) area "([^"]+)" with seed (\d+)')
_FOCUS_RE = re.compile(r"^\[WINDOW\] (Lost|Gained) focus", re.IGNORECASE)
_SPEAKER_RE = re.compile(r"^([^:\[]+?): (.+)$")
_REWARD_RE = re.compile(r"^: (.+?) has received (.+?)\.?$")
_DEATH_RE = re.compile(r"^: (.+?) has been slain\.?$")
_LEVEL_RE = re.compile(r"^: (.+?) \((\w+)\) is now level (\d+)")


class LogLine(NamedTuple):
    n: int                 # 1-based line number counted from the session offset
    utc: _dt.datetime
    source: str            # "counter" or "wall"
    kind: str              # opening, area, npc, reward, focus-lost, focus-gained, death
    area: str | None       # the area code in force (an area line carries its own)
    pid: int | None
    speaker: str | None    # NPC lines: the speaker; reward and death: the masked name
    text: str              # NPC: the line; area: the code; reward: the stat; else ""


class ClientLog:
    """What `read_log` found. Only allowlisted, masked lines are kept."""

    def __init__(self):
        self.lines: list[LogLine] = []
        self.pids: dict[int, int] = {}
        self.sources = {"counter": 0, "wall": 0}
        self.skipped = {"chat": 0, "unlisted-speaker": 0, "continuation": 0, "other": 0}
        # (utc, message) for every header line, chat included: read only by
        # repeats_logged, never returned or printed.
        self._said: list[tuple[_dt.datetime, str]] = []

    def of_kind(self, kind: str) -> list[LogLine]:
        return [l for l in self.lines if l.kind == kind]

    def openings(self) -> list[LogLine]:
        return self.of_kind("opening")

    def areas(self, code: str | None = None) -> list[LogLine]:
        return [l for l in self.lines if l.kind == "area" and (code is None or l.text == code)]

    def npc(self, speaker: str | None = None, area: str | None = None) -> list[LogLine]:
        return [l for l in self.lines if l.kind == "npc"
                and (speaker is None or l.speaker == speaker) and (area is None or l.area == area)]

    def rewards(self) -> list[LogLine]:
        return self.of_kind("reward")

    def deaths(self) -> list[LogLine]:
        return self.of_kind("death")

    def focus_losses(self, end: _dt.datetime | None = None) -> list[tuple[_dt.datetime, _dt.datetime]]:
        """Each stretch from a `Lost focus` line to the next `Gained focus`
        (or to `end`, or to the last line, while focus never came back)."""
        out = []
        lost = None
        for l in self.lines:
            if l.kind == "focus-lost" and lost is None:
                lost = l.utc
            elif l.kind == "focus-gained" and lost is not None:
                out.append((lost, l.utc))
                lost = None
        if lost is not None:
            last = end or (self.lines[-1].utc if self.lines else lost)
            out.append((lost, max(lost, last)))
        return out

    def repeats_logged(self, text: str, start: _dt.datetime, end: _dt.datetime,
                       score: Callable[[str, str], float], threshold: float) -> bool:
        """Whether `text` (a screen line) matches any line logged in
        `[start, end]`, either way round. Answers yes or no, nothing more."""
        for when, said in self._said:
            if start <= when <= end and (score(said, text) >= threshold or score(text, said) >= threshold):
                return True
        return False


def opening_utc(line: str, tz: _dt.tzinfo | None = None) -> _dt.datetime | None:
    m = _OPENING_RE.match(line)
    if not m:
        return None
    wall = _dt.datetime(*(int(g) for g in m.groups()))
    return common.local_to_utc(wall, tz)


def _mask(name: str, own: set[str]) -> str:
    return PLAYER_MASK if name in own else OTHER_MASK


def read_log(path: Path, offset: int = 0, anchors=(), tz: _dt.tzinfo | None = None,
             speakers=NPC_SPEAKERS) -> ClientLog:
    """The session's lines of `Client.txt` from byte `offset` on."""
    path = Path(path)
    if not path.is_file():
        raise common.ToolError("no Client.txt at %s" % path)
    with open(path, "rb") as f:
        f.seek(max(0, int(offset)))
        raw = f.read()
    text = raw.decode("utf-8", errors="replace")
    speakers = set(speakers)

    # First pass: what each header line is, before names are masked.
    parsed = []
    own: set[str] = set()
    for n, line in enumerate(text.split("\n"), 1):
        line = line.rstrip("\r")
        if not line.strip():
            continue
        when = opening_utc(line, tz)
        if when is not None:
            parsed.append((n, when, "wall", "opening", None, None, None))
            continue
        prefix = common.split_log_prefix(line)
        if prefix is None:
            parsed.append((n, None, None, "continuation", None, None, None))
            continue
        wall, counter = prefix
        head = _HEADER_RE.match(line[common.LOG_PREFIX_RE.match(line).end():])
        if head is None:
            parsed.append((n, None, None, "other", None, None, None))
            continue
        utc, source = common.log_time_utc(wall, counter, anchors, tz)
        pid, message = int(head.group(2)), head.group(3)
        m = _REWARD_RE.match(message)
        if m:
            own.add(m.group(1))
        m = _LEVEL_RE.match(message)
        if m:
            own.add(m.group(1))
        parsed.append((n, utc, source, "header", pid, message, None))

    log = ClientLog()
    area = None
    for n, utc, source, what, pid, message, _ in parsed:
        if what in ("continuation", "other"):
            log.skipped[what] += 1
            continue
        if what == "opening":
            log.lines.append(LogLine(n, utc, "wall", "opening", area, None, None, ""))
            continue
        log.sources[source] += 1
        log.pids[pid] = log.pids.get(pid, 0) + 1
        log._said.append((utc, message))
        m = _AREA_RE.match(message)
        if m:
            area = m.group(2)
            log.lines.append(LogLine(n, utc, source, "area", area, pid, None, area))
            continue
        m = _FOCUS_RE.match(message)
        if m:
            kind = "focus-lost" if m.group(1).lower() == "lost" else "focus-gained"
            log.lines.append(LogLine(n, utc, source, kind, area, pid, None, ""))
            continue
        m = _REWARD_RE.match(message)
        if m:
            log.lines.append(LogLine(n, utc, source, "reward", area, pid, _mask(m.group(1), own),
                                     m.group(2)))
            continue
        m = _DEATH_RE.match(message)
        if m:
            log.lines.append(LogLine(n, utc, source, "death", area, pid, _mask(m.group(1), own), ""))
            continue
        m = _SPEAKER_RE.match(message)
        if m:
            speaker = m.group(1).strip()
            if speaker[:1] in CHAT_PREFIXES:
                log.skipped["chat"] += 1
            elif speaker in speakers:
                log.lines.append(LogLine(n, utc, source, "npc", area, pid, speaker, m.group(2).strip()))
            else:
                log.skipped["unlisted-speaker"] += 1
            continue
        log.skipped["other"] += 1
    return log


def last_opening_before(path: Path, offset: int, tz: _dt.tzinfo | None = None,
                        chunk: int = 1 << 20) -> _dt.datetime | None:
    """The UTC of the last `LOG FILE OPENING` that starts before byte `offset`,
    read backwards from it, or None when there is none."""
    return scan_openings_before(path, offset, tz, chunk)[0]


def scan_openings_before(path: Path, offset: int, tz: _dt.tzinfo | None = None,
                         chunk: int = 1 << 20) -> tuple[_dt.datetime | None, int]:
    """(`last_opening_before`'s answer, the number of `LOG FILE OPENING` marks
    passed over because their time did not parse). With no answer, a count of
    0 means no mark at all before `offset`; more means marks none of which
    parsed."""
    path = Path(path)
    if not path.is_file():
        raise common.ToolError("no Client.txt at %s" % path)
    margin = 64
    unparsed = 0
    with open(path, "rb") as f:
        end = min(int(offset), path.stat().st_size)
        while end > 0:
            start = max(0, end - chunk)
            lo = max(0, start - margin)
            f.seek(lo)
            data = f.read(end - lo)
            i = data.rfind(_OPENING_MARK)
            while i >= 0:
                # Only a mark inside [start, end) is this chunk's; one in the
                # margin is read again, whole, with the next chunk.
                if lo + i >= start or start == 0:
                    ls = data.rfind(b"\n", 0, i) + 1
                    line = data[ls:].split(b"\n", 1)[0].decode("utf-8", errors="replace").rstrip("\r")
                    when = opening_utc(line, tz)
                    if when is not None:
                        return when, unparsed
                    unparsed += 1
                i = data.rfind(_OPENING_MARK, 0, i)
            end = start
    return None, unparsed
