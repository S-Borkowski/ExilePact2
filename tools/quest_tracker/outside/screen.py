"""Screen captures, OCR and the marker file: the third instrument and its readers.

Developer-only. Usage:

    py -3 -m tools.quest_tracker.outside.screen selftest-ocr
    py -3 -m tools.quest_tracker.outside.screen rehearse --out DIR
    py -3 -m tools.quest_tracker.outside.screen ocr [--live L] [--band top|bottom]
                                                    (--window FROM TO ... | --all)

The capture itself is `capture.ps1`: two bands of the game's client rect, the
top-centre boss bar (`top`) and the chat region (`bottom`), saved as JPEGs at
2 Hz with one `frames.jsonl` line each. Text is read afterwards, offline, by
`ocr.ps1` (`Windows.Media.Ocr`, en-US), only for the frames the analysis
asks about, and cached in `ocr.jsonl` beside `frames.jsonl`. `marker.ps1`
stamps each Ctrl+Alt+M press into `markers.txt`. `probe_window.ps1` shows
known lines in a window, the capture's positive control.

- `selftest-ocr` renders "Beira of the Rotten Pack" and "What happened
  here...?" to PNGs in a temporary folder (no window), reads them with
  `ocr.ps1`, and passes when each scores at least `MATCH_THRESHOLD`.
- `rehearse --out DIR` shows the probe window for about 3 s, captures it for
  2 s with the bottom band over the whole window, reads the frames and prints
  the `i3-rehearsal` capture line. It needs an unlocked desktop.
- `ocr --live L` reads the frames of `L\\i3` that fall in the given UTC
  windows (and band), skipping those already in `ocr.jsonl`.

What the analysis uses from here: `read_frames` / `frames_in_window` (the
frames index), `read_ocr` / `ocr_frames` (the OCR cache), `fuzzy_score` /
`best_score` (the fuzzy text match), `is_black`, `rehearsal_check` and
`read_markers` (the marker file).

The ready lines `run.py` waits for: the capture prints a line starting
`capture: ready`, the marker one starting `# registered`. Every command that
writes refuses a folder inside a git checkout.
"""
from __future__ import annotations

import argparse
import datetime as _dt
import difflib
import math
import os
import queue
import shutil
import subprocess
import sys
import tempfile
import threading
import time
from pathlib import Path
from typing import Callable, Iterable, NamedTuple

from tools.quest_tracker.outside import common
from tools.quest_tracker.outside.common import ToolError

HERE = Path(__file__).resolve().parent
CAPTURE_PS1 = HERE / "capture.ps1"
OCR_PS1 = HERE / "ocr.ps1"
PROBE_PS1 = HERE / "probe_window.ps1"
MARKER_PS1 = HERE / "marker.ps1"

FRAMES_JSONL = "frames.jsonl"
OCR_JSONL = "ocr.jsonl"
BANDS = ("top", "bottom")

# The capture's defaults (fractions x0, y0, x1, y1 of the client rect).
HZ = 2
TOP_BAND = (0.25, 0.0, 0.75, 0.14)
BOTTOM_BAND = (0.0, 0.35, 1.0, 1.0)
MAX_MINUTES = 90
OCR_SCALE = 2

# The fuzzy match: a score of at least this is a match, and the OCR line's
# windows compared with the target are within this fraction of its length.
MATCH_THRESHOLD = 0.8
WINDOW_SLACK = 0.2

# A frame this dark and this flat is a black capture, not a dark scene.
BLACK_LUMA_MEAN = 8.0
BLACK_LUMA_STD = 3.0

SELFTEST_LINES = ("Beira of the Rotten Pack", "What happened here...?")
PROBE_LINES = SELFTEST_LINES
PROBE_SECONDS = 3
REHEARSAL_CAPTURE_S = 2.0
# Generous: Windows PowerShell plus an Add-Type compile can take seconds cold.
READY_TIMEOUT_S = 25.0

CREATE_NO_WINDOW = 0x08000000


# --------------------------------------------------------------------------
# The fuzzy match

def normalize(text: str) -> str:
    """Lowercase, letters, digits and spaces only, runs of spaces collapsed."""
    kept = []
    for ch in text.lower():
        if ch.isalnum():
            kept.append(ch)
        elif ch.isspace():
            kept.append(" ")
    return " ".join("".join(kept).split())


def fuzzy_score(target: str, text: str) -> float:
    """How well `text` (an OCR line) contains `target`, 0..1.

    Both are normalized; the score is the best `difflib` ratio of the target
    against any window of the line whose length is within `WINDOW_SLACK` of
    the target's. A line shorter than the shortest window is compared whole.
    """
    t, s = normalize(target), normalize(text)
    if not t or not s:
        return 0.0
    lo = max(1, math.ceil(len(t) * (1 - WINDOW_SLACK) - 1e-9))
    hi = math.floor(len(t) * (1 + WINDOW_SLACK) + 1e-9)
    matcher = difflib.SequenceMatcher(None, autojunk=False)
    matcher.set_seq2(t)
    if len(s) < lo:
        matcher.set_seq1(s)
        return matcher.ratio()
    best = 0.0
    for n in range(lo, min(hi, len(s)) + 1):
        for start in range(0, len(s) - n + 1):
            matcher.set_seq1(s[start:start + n])
            if matcher.real_quick_ratio() <= best or matcher.quick_ratio() <= best:
                continue
            best = max(best, matcher.ratio())
            if best == 1.0:
                return best
    return best


def best_score(target: str, texts: Iterable[str]) -> float:
    return max((fuzzy_score(target, t) for t in texts), default=0.0)


def matches(target: str, text: str, threshold: float = MATCH_THRESHOLD) -> bool:
    return fuzzy_score(target, text) >= threshold


# --------------------------------------------------------------------------
# The frames index

class Frame(NamedTuple):
    utc: _dt.datetime
    band: str
    path: Path | None          # None for a skipped frame
    rect: tuple | None
    client: tuple | None
    foreground: bool
    luma_mean: float | None
    luma_std: float | None
    skipped: str | None


def _rect(value) -> tuple | None:
    return None if value is None else tuple(int(v) for v in value)


def read_frames(where: Path) -> list[Frame]:
    """The frames in `where` (an `i3` folder or a `frames.jsonl`), in time order.

    A relative `path` is resolved against the index's folder. A skipped frame
    (minimized window, failed copy) is kept, with `path` None.
    """
    where = Path(where)
    index = where / FRAMES_JSONL if where.is_dir() else where
    if not index.is_file():
        raise ToolError("no frames index at %s" % index)
    out = []
    for n, rec in enumerate(common.read_jsonl(index), 1):
        try:
            raw = rec.get("path")
            path = None
            if raw:
                p = Path(raw)
                path = p if p.is_absolute() else index.parent / p
            mean, std = rec.get("luma_mean"), rec.get("luma_std")
            out.append(Frame(
                utc=common.parse_utc(rec["utc"]),
                band=str(rec["band"]),
                path=path,
                rect=_rect(rec.get("rect")),
                client=_rect(rec.get("client")),
                foreground=bool(rec.get("foreground", False)),
                luma_mean=None if mean is None else float(mean),
                luma_std=None if std is None else float(std),
                skipped=rec.get("skipped"),
            ))
        except (KeyError, TypeError, ValueError) as err:
            raise ToolError("%s: record %d is not a frame: %s" % (index, n, err))
    out.sort(key=lambda f: f.utc)
    return out


def frames_in_window(frames: Iterable[Frame], start: _dt.datetime, end: _dt.datetime,
                     band: str | None = None) -> list[Frame]:
    """The captured frames with `start <= utc <= end`, of `band` if given."""
    return [f for f in frames
            if f.path is not None and start <= f.utc <= end and (band is None or f.band == band)]


def is_black(frame: Frame) -> bool:
    """A frame the capture got back black: very dark and nearly uniform."""
    if frame.luma_mean is None or frame.luma_std is None:
        return False
    return frame.luma_mean < BLACK_LUMA_MEAN and frame.luma_std < BLACK_LUMA_STD


# --------------------------------------------------------------------------
# The OCR cache

class OcrLine(NamedTuple):
    text: str
    box: tuple


def path_key(path: Path | str) -> str:
    return os.path.normcase(os.path.abspath(str(path)))


def read_ocr(path: Path) -> dict[str, list[OcrLine]]:
    """`ocr.jsonl` as {path key: lines}. A later record for a path wins;
    a record with an `error` is left out, so the next run tries again."""
    path = Path(path)
    if not path.is_file():
        return {}
    out = {}
    for rec in common.read_jsonl(path):
        if rec.get("error") or "path" not in rec:
            continue
        p = Path(rec["path"])
        if not p.is_absolute():
            p = path.parent / p
        out[path_key(p)] = [OcrLine(str(l.get("text", "")), tuple(int(v) for v in l.get("box", ())))
                            for l in rec.get("lines") or []]
    return out


def powershell(script: Path, *args: str) -> list[str]:
    return ["powershell.exe", "-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass",
            "-File", str(script), *args]


def run_ocr_ps1(list_path: Path, out_path: Path, scale: float = OCR_SCALE) -> None:
    """Run `ocr.ps1` over the paths in `list_path`, appending to `out_path`."""
    if sys.platform != "win32":
        raise ToolError("OCR runs Windows.Media.Ocr, which needs Windows")
    proc = subprocess.run(powershell(OCR_PS1, "-List", str(list_path), "-Out", str(out_path),
                                     "-Scale", str(scale)),
                          capture_output=True, text=True, creationflags=CREATE_NO_WINDOW)
    if proc.returncode != 0:
        raise ToolError("ocr.ps1 exited %d: %s" % (proc.returncode,
                                                   (proc.stderr or proc.stdout).strip()))


def ocr_frames(folder: Path, paths: Iterable[Path], runner: Callable = run_ocr_ps1,
               scale: float = OCR_SCALE) -> tuple[dict[str, list[OcrLine]], int, int]:
    """OCR results for `paths`, reading only those not yet in `folder\\ocr.jsonl`.

    Returns ({path key: lines} for the paths asked about that have a result,
    how many came from the cache, how many were read now).
    """
    folder = Path(folder)
    common.refuse_inside_git(folder, "OCR cache")
    cache_path = folder / OCR_JSONL
    wanted = {}
    for p in paths:
        wanted.setdefault(path_key(p), Path(p))
    cache = read_ocr(cache_path)
    missing = [p for k, p in wanted.items() if k not in cache]
    if missing:
        list_path = folder / "ocr-list.txt"
        with open(list_path, "w", encoding="utf-8", newline="\n") as f:
            for p in missing:
                f.write(os.path.abspath(str(p)) + "\n")
        runner(list_path, cache_path, scale)
        cache = read_ocr(cache_path)
    found = {k: cache[k] for k in wanted if k in cache}
    return found, len(wanted) - len(missing), len(missing)


def frame_lines(frame: Frame, ocr: dict[str, list[OcrLine]]) -> list[OcrLine]:
    return [] if frame.path is None else ocr.get(path_key(frame.path), [])


# --------------------------------------------------------------------------
# The marker file

class Press(NamedTuple):
    n: int
    utc: _dt.datetime


class Markers(NamedTuple):
    header: list[str]     # the `#` lines, without the `#`
    presses: list[Press]


def parse_markers(text: str, name: str = "markers.txt") -> Markers:
    """`# ...` header lines and one `<n> <utc>` line per press.

    A last line without its newline that does not parse is a writer stopped
    mid-line and is dropped; any other bad line is an error naming it.
    """
    header, presses = [], []
    lines = text.split("\n")
    for i, raw in enumerate(lines):
        line = raw.strip()
        if not line:
            continue
        if line.startswith("#"):
            header.append(line[1:].strip())
            continue
        parts = line.split()
        try:
            if len(parts) != 2:
                raise ValueError("expected '<n> <utc>'")
            press = Press(int(parts[0]), common.parse_utc(parts[1]))
        except ValueError as err:
            if i == len(lines) - 1:
                break
            raise ToolError("%s:%d is not a press line: %s" % (name, i + 1, err))
        presses.append(press)
    return Markers(header, presses)


def read_markers(path: Path) -> Markers:
    path = Path(path)
    if not path.is_file():
        raise ToolError("no marker file at %s" % path)
    return parse_markers(path.read_text(encoding="utf-8"), str(path))


# --------------------------------------------------------------------------
# The rehearsal check

def rehearsal_check(frames: list[Frame], ocr: dict[str, list[OcrLine]],
                    lines: Iterable[str] = PROBE_LINES) -> tuple[str, str, str]:
    """`i3-rehearsal`: every probe line scores at least `MATCH_THRESHOLD` in
    some captured frame. Returns (expected, observed, verdict)."""
    expected = "each probe line scores >= %.1f in a captured frame" % MATCH_THRESHOLD
    shot = [f for f in frames if f.path is not None]
    black = sum(1 for f in shot if is_black(f))
    texts = [l.text for f in shot for l in frame_lines(f, ocr)]
    scores = [(line, best_score(line, texts)) for line in lines]
    observed = "frames %d (black %d, read %d); %s" % (
        len(shot), black, sum(1 for f in shot if f.path is not None and path_key(f.path) in ocr),
        ", ".join('"%s" %.2f' % (line, s) for line, s in scores))
    ok = bool(shot) and all(s >= MATCH_THRESHOLD for _, s in scores)
    return expected, observed, "pass" if ok else "fail"


def check_line(name: str, expected: str, observed: str, verdict: str) -> str:
    return "- %s | expected: %s | observed: %s | %s" % (name, expected, observed, verdict)


# --------------------------------------------------------------------------
# Child processes

class _Lines:
    """A child's stdout, read on a thread so a wait can time out."""

    def __init__(self, proc: subprocess.Popen):
        self.q: queue.Queue = queue.Queue()
        self.seen: list[str] = []
        t = threading.Thread(target=self._pump, args=(proc.stdout,), daemon=True)
        t.start()

    def _pump(self, stream) -> None:
        for line in stream:
            self.q.put(line.rstrip("\r\n"))
        self.q.put(None)

    def wait_for(self, prefix: str, timeout: float) -> str | None:
        deadline = time.monotonic() + timeout
        while True:
            left = deadline - time.monotonic()
            if left <= 0:
                return None
            try:
                line = self.q.get(timeout=left)
            except queue.Empty:
                return None
            if line is None:
                return None
            self.seen.append(line)
            if line.startswith(prefix):
                return line


def _popen(cmd: list[str]) -> subprocess.Popen:
    return subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
                            encoding="utf-8", errors="replace", creationflags=CREATE_NO_WINDOW)


def _finish(proc: subprocess.Popen, timeout: float) -> int:
    try:
        return proc.wait(timeout=timeout)
    except subprocess.TimeoutExpired:
        proc.kill()
        return proc.wait()


def _band_arg(band) -> str:
    return "off" if band is None else ",".join(repr(float(v)) for v in band)


# --------------------------------------------------------------------------
# Commands

def selftest_ocr() -> tuple[bool, list[str]]:
    if sys.platform != "win32":
        raise ToolError("the OCR selftest runs Windows.Media.Ocr, which needs Windows")
    tmp = Path(tempfile.mkdtemp(prefix="qt-selftest-ocr-"))
    try:
        common.refuse_inside_git(tmp, "selftest images")
        proc = subprocess.run(powershell(PROBE_PS1, "-RenderTo", str(tmp), "-Text", "|".join(SELFTEST_LINES)),
                              capture_output=True, text=True, creationflags=CREATE_NO_WINDOW)
        if proc.returncode != 0:
            raise ToolError("probe_window.ps1 -RenderTo exited %d: %s"
                            % (proc.returncode, (proc.stderr or proc.stdout).strip()))
        images = [tmp / ("line-%d.png" % (i + 1)) for i in range(len(SELFTEST_LINES))]
        for img in images:
            if not img.is_file():
                raise ToolError("probe_window.ps1 did not render %s" % img.name)
        ocr, _, _ = ocr_frames(tmp, images)
        report, ok = [], True
        for line, img in zip(SELFTEST_LINES, images):
            read = [l.text for l in ocr.get(path_key(img), [])]
            score = best_score(line, read)
            ok = ok and score >= MATCH_THRESHOLD
            report.append('"%s" %.2f (read: %s)' % (line, score, " / ".join(read) or "nothing"))
        return ok, report
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def rehearse(out: Path) -> tuple[str, str, str]:
    out = Path(out)
    common.refuse_inside_git(out, "rehearsal frames")
    if out.exists() and any(out.iterdir()):
        raise ToolError("refusing to rehearse into %s: it is not empty" % out)
    if sys.platform != "win32":
        raise ToolError("the rehearsal shows a window and captures the screen, which needs Windows")
    out.mkdir(parents=True, exist_ok=True)
    stop = out / "stop-rehearsal"

    # The probe and the capture start together: the capture already knows the
    # probe's pid and waits for its window, so neither start-up delays the other.
    probe = _popen(powershell(PROBE_PS1, "-Text", "|".join(PROBE_LINES), "-Seconds", str(PROBE_SECONDS)))
    capture = _popen(powershell(CAPTURE_PS1, "-TargetPid", str(probe.pid), "-OutDir", str(out),
                                "-StopFile", str(stop), "-Hz", str(HZ), "-Top", "off",
                                "-Bottom", _band_arg((0.0, 0.0, 1.0, 1.0)), "-MaxMinutes", "1",
                                "-WaitSeconds", "15"))
    probe_out, capture_out = _Lines(probe), _Lines(capture)
    try:
        ready = capture_out.wait_for("capture: ready", READY_TIMEOUT_S)
        if ready is None:
            _finish(capture, 5)
            raise ToolError("the capture never got ready: %s %s" % (
                " / ".join(capture_out.seen), capture.stderr.read().strip()))
        time.sleep(REHEARSAL_CAPTURE_S)
    finally:
        stop.touch()
        _finish(capture, 15)
        _finish(probe, 15)
    shown = probe_out.wait_for("probe: pid", 1)
    common.write_json(out / "probe.json", {"lines": list(PROBE_LINES), "pid": probe.pid,
                                           "shown": shown, "capture": ready})
    frames = [f for f in read_frames(out) if f.band == "bottom"]
    ocr, _, _ = ocr_frames(out, [f.path for f in frames if f.path is not None])
    return rehearsal_check(frames, ocr)


def parse_window(values: list[str]) -> tuple[_dt.datetime, _dt.datetime]:
    start, end = common.parse_utc(values[0]), common.parse_utc(values[1])
    if end < start:
        raise ToolError("a --window ends before it starts: %s %s" % tuple(values))
    return start, end


def select_frames(frames: list[Frame], windows, band: str | None, everything: bool) -> list[Frame]:
    if everything:
        return [f for f in frames if f.path is not None and (band is None or f.band == band)]
    picked = {}
    for start, end in windows:
        for f in frames_in_window(frames, start, end, band):
            picked[path_key(f.path)] = f
    return sorted(picked.values(), key=lambda f: f.utc)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="screen", description="Screen capture, OCR and the marker file.")
    sub = parser.add_subparsers(dest="command", required=True)
    sub.add_parser("selftest-ocr", help="OCR two rendered lines; no window")
    r = sub.add_parser("rehearse", help="capture the probe window and OCR it (needs an unlocked desktop)")
    r.add_argument("--out", type=Path, required=True)
    o = sub.add_parser("ocr", help="OCR the selected frames of L\\i3, caching results in ocr.jsonl")
    o.add_argument("--live", type=Path, default=None)
    o.add_argument("--band", choices=BANDS, default=None)
    o.add_argument("--window", nargs=2, action="append", default=[], metavar=("FROM", "TO"),
                   help="a UTC window, YYYY-MM-DDTHH:MM:SS.mmmZ; repeatable")
    o.add_argument("--all", action="store_true", help="every captured frame")
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    try:
        if args.command == "selftest-ocr":
            ok, report = selftest_ocr()
            for line in report:
                print("selftest-ocr: " + line)
            print("selftest-ocr: %s" % ("pass" if ok else "fail"))
            return 0 if ok else 1
        if args.command == "rehearse":
            expected, observed, verdict = rehearse(args.out)
            print(check_line("i3-rehearsal", expected, observed, verdict))
            return 0 if verdict == "pass" else 1
        live = args.live if args.live is not None else common.default_live_dir()
        if not args.window and not args.all:
            raise ToolError("give at least one --window FROM TO, or --all")
        windows = [parse_window(w) for w in args.window]
        i3 = Path(live) / "i3"
        frames = select_frames(read_frames(i3), windows, args.band, args.all)
        found, cached, read_now = ocr_frames(i3, [f.path for f in frames])
        print("ocr: %d frames selected, %d cached, %d read now, %d without a result"
              % (len(frames), cached, read_now, len(frames) - len(found)))
    except ToolError as err:
        print("screen: error: %s" % err, file=sys.stderr)
        return 1
    except (OSError, ValueError) as err:
        print("screen: error: %s" % err, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
