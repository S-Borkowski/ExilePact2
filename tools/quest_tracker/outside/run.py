"""Start and stop the session's instruments together.

Developer-only. Usage:

    py -3 -m tools.quest_tracker.outside.run start [--live L] --pid P [--timeout S] [--max-minutes M]
                                                   [--top x0,y0,x1,y1|off] [--bottom x0,y0,x1,y1|off]
    py -3 -m tools.quest_tracker.outside.run stop [--live L] [--timeout S]

`start` launches the three unelevated instruments against the game's pid, each
as a detached process with its output in a log file, so they outlive the shell
that started them:

- the recorder, `loopback.ps1` (process loopback of the pid's tree into
  `i2\\game.wav`, stop file `stop-i2`, log `i2\\recorder.log`, ready line
  `ready <utc>`);
- the capture, `capture.ps1` (2 Hz frames of the game's window into `i3\\`,
  stop file `stop-i3`, log `i3\\capture.log`, ready line `capture: ready ...`;
  `--top` and `--bottom` pass its `-Top` and `-Bottom` bands, as fractions of
  the client rect or `off`, checked as the capture checks them before anything
  is spawned; without them it keeps its own bands and its command is unchanged);
- the marker, `marker.ps1` (Ctrl+Alt+M presses into `markers.txt`, stop file
  `stop-marker`, log `marker.log`, ready line `# registered ...`).

It waits for each one's ready line, prints one line per instrument, and
records each process's pid and creation time in `run.json`. It exits 0 when
all three are ready and 1 otherwise; an instrument that is not ready keeps
running if it is still alive, and `stop` ends it with the rest.

`stop` creates the stop files (`session.stop`, which also asks the elevated
file-trace controller to end through `stop-i1`), then waits for each
instrument in `run.json` to exit and for the controller's `i1\\status.txt` to
reach `session-trace-stopped` or `error` (unless it never started a session
trace). It prints one line each and exits 0 when everything ended within the
timeout.

Nothing here touches the game's process: the pid only names whose audio and
window the instruments observe. Every writer refuses a live folder inside a
git checkout.
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
import time
from pathlib import Path
from typing import Callable, NamedTuple

from tools.quest_tracker.outside import audio, common, screen, session
from tools.quest_tracker.outside.common import ToolError

ROOT = Path(__file__).resolve().parents[3]
RUN_JSON = "run.json"
READY_TIMEOUT_S = 30.0
STOP_TIMEOUT_S = 300.0
POLL_S = 0.1
MAX_MINUTES = 90.0

# Not DETACHED_PROCESS: a child started with it has no console at all, Windows
# ignores CREATE_NO_WINDOW beside it, and the instruments' powershell.exe ran
# nothing that way (Live 6, blind at G2). CREATE_NO_WINDOW gives the child a
# console of its own that is never shown.
CREATE_NO_WINDOW = 0x08000000
CREATE_NEW_PROCESS_GROUP = 0x00000200
CREATE_BREAKAWAY_FROM_JOB = 0x01000000

CONTROLLER_DONE = "session-trace-stopped"
# The controller's one error that does not end it: it goes on to stop the trace.
CONTROLLER_CAP = "error session cap"


class Instrument(NamedTuple):
    name: str
    command: list[str]
    log: Path
    ready: str          # the prefix of its ready line
    stop: str           # its stop-file instrument name


def band_arg(option: str, text: str) -> str:
    """`text` checked as `capture.ps1` checks a band (`x0,y0,x1,y1` fractions of
    the client rect, each in 0..1, with x1 > x0 and y1 > y0, or `off`), so a
    malformed band is refused here, naming `option`, rather than by a capture
    already spawned. Returns the value to pass on."""
    if text.strip().lower() == "off":
        return "off"
    parts = text.split(",")
    if len(parts) != 4:
        raise ToolError("%s needs x0,y0,x1,y1 or off, got '%s'" % (option, text))
    values = []
    for part in parts:
        try:
            value = float(part.strip())
        except ValueError:
            raise ToolError("%s has a value that is not a number: '%s'" % (option, part)) from None
        if not 0.0 <= value <= 1.0:     # also refuses nan
            raise ToolError("%s fractions lie in 0..1, got '%s'" % (option, part))
        values.append(value)
    if values[2] <= values[0] or values[3] <= values[1]:
        raise ToolError("%s needs x1 > x0 and y1 > y0, got '%s'" % (option, text))
    return text


def instruments(live: Path, pid: int, max_minutes: float = MAX_MINUTES,
                top: str | None = None, bottom: str | None = None) -> list[Instrument]:
    """The three instruments' commands. `top` and `bottom`, when given, override
    the capture's bands (`-Top`, `-Bottom`); without them the capture keeps its
    own defaults and its command is unchanged."""
    live = Path(live)
    bands = []
    for option, flag, text in (("--top", "-Top", top), ("--bottom", "-Bottom", bottom)):
        if text is not None:
            bands += [flag, band_arg(option, text)]
    if bands.count("off") == 2:
        raise ToolError("--top and --bottom are both off: the capture would have nothing to capture")
    return [
        Instrument("recorder",
                   audio.recorder_command(pid, live / "i2" / "game.wav", common.stop_file(live, "i2"),
                                          max_minutes),
                   live / "i2" / "recorder.log", "ready ", "i2"),
        Instrument("capture",
                   screen.powershell(screen.CAPTURE_PS1, "-TargetPid", str(pid), "-OutDir", str(live / "i3"),
                                     "-StopFile", str(common.stop_file(live, "i3")), "-Hz", str(screen.HZ),
                                     "-MaxMinutes", str(max_minutes), *bands),
                   live / "i3" / "capture.log", "capture: ready", "i3"),
        Instrument("marker",
                   screen.powershell(screen.MARKER_PS1, "-Out", str(live / common.MARKERS_TXT),
                                     "-StopFile", str(common.stop_file(live, "marker"))),
                   live / "marker.log", "# registered", "marker"),
    ]


# --------------------------------------------------------------------------
# Processes

def spawn_detached(command: list[str], log: Path, cwd: Path = ROOT) -> int:
    """Start `command` with a hidden console of its own and its own process
    group, its output appended to `log`, outside the caller's job when Windows
    allows it, so it keeps running after the caller's shell ends. Returns its
    pid."""
    with open(log, "ab") as out:
        kw = dict(stdin=subprocess.DEVNULL, stdout=out, stderr=subprocess.STDOUT, cwd=str(cwd),
                  close_fds=True)
        if sys.platform != "win32":
            return subprocess.Popen(command, start_new_session=True, **kw).pid
        flags = CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP
        try:
            return subprocess.Popen(command, creationflags=flags | CREATE_BREAKAWAY_FROM_JOB, **kw).pid
        except OSError:
            # A job that forbids breakaway refuses the flag; the process then
            # stays in the job, which matters only if the job is killed.
            return subprocess.Popen(command, creationflags=flags, **kw).pid


def _kernel32():
    import ctypes
    from ctypes import wintypes
    k = ctypes.WinDLL("kernel32", use_last_error=True)
    k.OpenProcess.restype = wintypes.HANDLE
    k.OpenProcess.argtypes = (wintypes.DWORD, wintypes.BOOL, wintypes.DWORD)
    k.WaitForSingleObject.restype = wintypes.DWORD
    k.WaitForSingleObject.argtypes = (wintypes.HANDLE, wintypes.DWORD)
    k.GetProcessTimes.restype = wintypes.BOOL
    k.GetProcessTimes.argtypes = (wintypes.HANDLE,) + (ctypes.POINTER(wintypes.FILETIME),) * 4
    k.CloseHandle.argtypes = (wintypes.HANDLE,)
    return ctypes, wintypes, k


_SYNCHRONIZE = 0x00100000
_QUERY_LIMITED = 0x1000
_WAIT_TIMEOUT = 0x102


def process_created(pid: int) -> int | None:
    """The creation time of a running process `pid` (FILETIME ticks), or None
    when no such process is running."""
    if sys.platform != "win32":
        import os
        try:
            os.kill(pid, 0)
        except OSError:
            return None
        return 0
    ctypes, wintypes, k = _kernel32()
    h = k.OpenProcess(_SYNCHRONIZE | _QUERY_LIMITED, False, int(pid))
    if not h:
        return None
    try:
        if k.WaitForSingleObject(h, 0) != _WAIT_TIMEOUT:
            return None
        times = [wintypes.FILETIME() for _ in range(4)]
        if not k.GetProcessTimes(h, *(ctypes.byref(t) for t in times)):
            return 0
        return (times[0].dwHighDateTime << 32) | times[0].dwLowDateTime
    finally:
        k.CloseHandle(h)


def alive(pid: int, created: int | None) -> bool:
    """Whether the process started as `pid` at `created` is still running
    (a reused pid with another creation time is not)."""
    now = process_created(pid)
    if now is None:
        return False
    return not created or not now or now == created


# --------------------------------------------------------------------------
# Log files

def log_lines(path: Path) -> list[str]:
    if not Path(path).is_file():
        return []
    text = Path(path).read_bytes().decode("utf-8", errors="replace")
    return [l.rstrip("\r") for l in text.split("\n") if l.strip()]


def ready_line(path: Path, prefix: str) -> str | None:
    return next((l for l in log_lines(path) if l.startswith(prefix)), None)


def last_line(path: Path) -> str:
    lines = log_lines(path)
    return lines[-1] if lines else "no output"


# --------------------------------------------------------------------------
# start and stop

def start(live: Path, pid: int, timeout: float = READY_TIMEOUT_S, max_minutes: float = MAX_MINUTES,
          spawn: Callable = spawn_detached, created: Callable = process_created,
          tools: list[Instrument] | None = None, out=None,
          top: str | None = None, bottom: str | None = None) -> int:
    out = out or sys.stdout
    live = Path(live)
    common.refuse_inside_git(live, "instrument logs")
    if not (live / common.SESSION_JSON).is_file():
        raise ToolError("no session.json in %s: run `session prep` first" % live)
    if (live / RUN_JSON).exists():
        raise ToolError("%s already exists: the instruments were started once. Run `run stop`, "
                        "and use a fresh live folder for a new session" % (live / RUN_JSON))
    tools = tools if tools is not None else instruments(live, pid, max_minutes, top=top, bottom=bottom)
    for t in tools:
        stop = common.stop_file(live, t.stop)
        if stop.exists():
            raise ToolError("the stop file %s already exists: the instruments would end at once" % stop)

    record = {"started_utc": common.format_utc(common.now_utc()), "pid": int(pid), "instruments": {}}
    began = time.monotonic()
    for t in tools:
        t.log.parent.mkdir(parents=True, exist_ok=True)
        child = spawn(t.command, t.log)
        record["instruments"][t.name] = {"pid": child, "created": created(child), "log": str(t.log),
                                         "command": t.command, "stop": t.stop, "ready": None}
    common.write_json(live / RUN_JSON, record)

    waiting = {t.name: t for t in tools}
    gone: dict[str, str] = {}
    while waiting and time.monotonic() - began < timeout:
        for name, t in list(waiting.items()):
            entry = record["instruments"][name]
            line = ready_line(t.log, t.ready)
            if line is not None:
                entry["ready"] = line
                entry["ready_s"] = round(time.monotonic() - began, 1)
                del waiting[name]
            elif not alive(entry["pid"], entry["created"]):
                gone[name] = last_line(t.log)
                del waiting[name]
        if waiting:
            time.sleep(POLL_S)
    common.write_json(live / RUN_JSON, record)

    ok = True
    for t in tools:
        entry = record["instruments"][t.name]
        if entry["ready"] is not None:
            print("run: %s ready after %.1f s (pid %d): %s" % (t.name, entry["ready_s"], entry["pid"],
                                                               entry["ready"]), file=out)
            continue
        ok = False
        if t.name in gone:
            print("run: %s NOT ready: it exited (pid %d): %s" % (t.name, entry["pid"], gone[t.name]), file=out)
        else:
            print("run: %s NOT ready within %.0f s (pid %d, still running): %s"
                  % (t.name, timeout, entry["pid"], last_line(t.log)), file=out)
    return 0 if ok else 1


def controller_state(live: Path) -> str | None:
    """The file-trace controller's last state in `i1\\status.txt`, or None."""
    lines = log_lines(Path(live) / "i1" / "status.txt")
    if not lines:
        return None
    parts = lines[-1].split(" ", 1)
    return parts[1] if len(parts) > 1 else parts[0]


def controller_ended(state: str | None) -> bool:
    """Whether a controller in `state` has ended (or never started)."""
    if state is None or state == CONTROLLER_DONE:
        return True
    return state.startswith("error") and not state.startswith(CONTROLLER_CAP)


def stop(live: Path, timeout: float = STOP_TIMEOUT_S, out=None) -> int:
    out = out or sys.stdout
    live = Path(live)
    for path in session.stop(live):
        print("run: stop file %s" % path.name, file=out)
    record = {}
    if (live / RUN_JSON).is_file():
        record = common.read_json(live / RUN_JSON).get("instruments", {})
    else:
        print("run: no run.json: no instrument was started by `run start`", file=out)

    began = time.monotonic()
    pending = dict(record)
    trace = not controller_ended(controller_state(live))
    while (pending or trace) and time.monotonic() - began < timeout:
        for name, entry in list(pending.items()):
            if not alive(entry["pid"], entry.get("created")):
                print("run: %s ended after %.1f s: %s" % (name, time.monotonic() - began,
                                                          last_line(Path(entry["log"]))), file=out)
                del pending[name]
        if trace and controller_ended(controller_state(live)):
            trace = False
        if pending or trace:
            time.sleep(POLL_S if pending else 1.0)

    ok = True
    for name, entry in pending.items():
        ok = False
        print("run: %s still running after %.0f s (pid %d): %s" % (name, timeout, entry["pid"],
                                                                  last_line(Path(entry["log"]))), file=out)
    state = controller_state(live)
    if state is None:
        print("run: file trace: no i1\\status.txt (the controller never started)", file=out)
    elif trace:
        ok = False
        print("run: file trace: still %s after %.0f s" % (state, timeout), file=out)
    else:
        print("run: file trace: %s" % state, file=out)
    return 0 if ok else 1


# --------------------------------------------------------------------------
# Command line

def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="run", description="Start and stop the session's instruments.")
    sub = parser.add_subparsers(dest="command", required=True)
    s = sub.add_parser("start", help="launch the recorder, the capture and the marker, detached")
    s.add_argument("--live", type=Path, default=None)
    s.add_argument("--pid", type=int, required=True, help="the game's pid, from the lease")
    s.add_argument("--timeout", type=float, default=READY_TIMEOUT_S, help="seconds to wait for the ready lines")
    s.add_argument("--max-minutes", type=float, default=MAX_MINUTES)
    s.add_argument("--top", default=None, metavar="x0,y0,x1,y1|off",
                   help="the capture's top band, fractions of the client rect (default: the capture's own)")
    s.add_argument("--bottom", default=None, metavar="x0,y0,x1,y1|off",
                   help="the capture's bottom band, fractions of the client rect (default: the capture's own)")
    t = sub.add_parser("stop", help="create the stop files and wait for every instrument to end")
    t.add_argument("--live", type=Path, default=None)
    t.add_argument("--timeout", type=float, default=STOP_TIMEOUT_S)
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    live = args.live if args.live is not None else common.default_live_dir()
    try:
        if args.command == "start":
            return start(live, args.pid, timeout=args.timeout, max_minutes=args.max_minutes,
                         top=args.top, bottom=args.bottom)
        return stop(live, timeout=args.timeout)
    except ToolError as err:
        print("run: error: %s" % err, file=sys.stderr)
        return 1
    except (OSError, ValueError, json.JSONDecodeError) as err:
        print("run: error: %s" % err, file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
