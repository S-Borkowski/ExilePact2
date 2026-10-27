"""One machine-wide game lease, so two sessions never drive one install at once.

Each worktree has its own `.claude/workorders/`, so nothing a checkout keeps
can tell one Claude session that another is already driving the one Path of
Exile 2 install on this machine. This module keeps that fact under
`%LOCALAPPDATA%\\ExilePact2\\` as one record, `lease.json`.

`tools/x64dbg_mcp.py` reads it: it attaches to the game only while a `held`
record names a holder that is still running. Until this repository has a game
driver of its own that holds the lease for the length of a live session, take
it by hand, naming the running game process as its holder -- the lease then
lives exactly as long as that game process does:

    py -3 -m tools.game_lease acquire --label <slug>-live-1   # finds the game
    py -3 -m tools.game_lease acquire --label <slug>-live-1 --pid 1234
    py -3 -m tools.game_lease status
    py -3 -m tools.game_lease release

**A crashed holder is detected by process identity, not by age.** A `held`
record names its holder as `(pid, pid_start)`: the PID and the moment that
process was created. The holder is alive only if a process with that PID
exists, has not exited, and was created at that same moment -- so a PID the
system has since handed to something else reads as gone.

**A record that cannot be read never reads as free.** `read_record` returns a
refusal for a present but unparseable `lease.json` -- `AGENTS.md` § "Check a
Permission Where It Is Used", a sentinel meaning *unknown* must not compare
equal to a real value -- and `acquire --force` is the way out.
"""

from __future__ import annotations

import argparse
import csv
import io
import json
import os
import re
import secrets
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

if os.name == "nt":
    import ctypes
    from ctypes import wintypes

SCHEMA = "game-lease/1"
RECORD_NAME = "lease.json"

#: Record states.
HELD = "held"
RELEASED = "released"

#: A label names a session in someone else's refusal, so it is kept short and
#: plain. `<slug>-live-<n>` fits.
LABEL_PATTERN = re.compile(r"^[A-Za-z0-9._-]{1,80}$")

#: Executable names the game ships under, most specific first. `GAME_EXE`
#: (comma-separated) overrides them.
GAME_EXES = ("PathOfExile_x64Steam.exe", "PathOfExileSteam.exe", "PathOfExile_x64.exe",
             "PathOfExile.exe")

#: `OpenProcess` access right that is enough for `GetExitCodeProcess` and
#: `GetProcessTimes`, and is granted for any process of the same user.
PROCESS_QUERY_LIMITED_INFORMATION = 0x1000
STILL_ACTIVE = 259

#: `process_identity()`, computed once per process.
_IDENTITY: dict[str, Any] | None = None


# --------------------------------------------------------------------------
# Locations. `GAME_LEASE_DIR` exists so no test touches the real one.
# --------------------------------------------------------------------------

def lease_dir() -> Path:
    """`GAME_LEASE_DIR` when set, else `%LOCALAPPDATA%\\ExilePact2`.

    The override is for test isolation. Set per worktree, it would give each
    session a lease of its own and defeat the point of having one.
    """
    override = os.environ.get("GAME_LEASE_DIR")
    if override:
        return Path(override)
    local = os.environ.get("LOCALAPPDATA", str(Path.home() / "AppData" / "Local"))
    return Path(local) / "ExilePact2"


def record_path() -> Path:
    return lease_dir() / RECORD_NAME


# --------------------------------------------------------------------------
# Process identity
# --------------------------------------------------------------------------

def _windows_start(pid: int) -> int | None:
    """The process's creation `FILETIME` as one integer, or None when no live
    process has this PID. A handle alone is not proof of life, so the exit
    code is read too."""
    api = ctypes.WinDLL("kernel32", use_last_error=True)
    api.OpenProcess.restype = wintypes.HANDLE
    api.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
    api.GetExitCodeProcess.argtypes = [wintypes.HANDLE, ctypes.POINTER(wintypes.DWORD)]
    api.GetProcessTimes.argtypes = [wintypes.HANDLE] + [ctypes.POINTER(wintypes.FILETIME)] * 4
    api.CloseHandle.argtypes = [wintypes.HANDLE]
    handle = api.OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, False, int(pid))
    if not handle:
        return None
    try:
        code = wintypes.DWORD()
        if not api.GetExitCodeProcess(handle, ctypes.byref(code)):
            return None
        if code.value != STILL_ACTIVE:
            return None
        created, exited, kernel, user = (wintypes.FILETIME() for _ in range(4))
        if not api.GetProcessTimes(handle, ctypes.byref(created), ctypes.byref(exited),
                                   ctypes.byref(kernel), ctypes.byref(user)):
            return None
        return (int(created.dwHighDateTime) << 32) | int(created.dwLowDateTime)
    finally:
        api.CloseHandle(handle)


def _linux_start(pid: int) -> str | None:
    """`<boot_id>:<starttime>`, or None when no live process has this PID."""
    try:
        os.kill(int(pid), 0)
    except ProcessLookupError:
        return None
    except PermissionError:
        pass  # someone else's process, but it exists
    try:
        stat = Path(f"/proc/{int(pid)}/stat").read_text(encoding="ascii", errors="replace")
    except OSError:
        return None
    fields = stat[stat.rindex(")") + 1:].split()
    if len(fields) < 20 or fields[0] in ("Z", "X", "x"):
        return None
    try:
        boot = Path("/proc/sys/kernel/random/boot_id").read_text(encoding="ascii").strip()
    except OSError:
        boot = ""
    return f"{boot}:{fields[19]}"


def _has_start_time() -> bool:
    return os.name == "nt" or sys.platform.startswith("linux")


def process_start_for(pid: int) -> Any:
    """When process `pid` was created, or None when no live process has it."""
    if os.name == "nt":
        return _windows_start(pid)
    if sys.platform.startswith("linux"):
        return _linux_start(pid)
    return None


def process_identity() -> dict[str, Any]:
    """This process: `{pid, pid_start, platform}`, computed once."""
    global _IDENTITY
    if _IDENTITY is None:
        pid = os.getpid()
        _IDENTITY = {"pid": pid, "pid_start": process_start_for(pid), "platform": sys.platform}
    return dict(_IDENTITY)


def process_alive(identity: Any) -> bool:
    """True only when the process `identity` names is still that process."""
    if not isinstance(identity, dict):
        return False
    try:
        pid = int(identity.get("pid") or 0)
    except (TypeError, ValueError):
        return False
    if pid <= 0:
        return False
    if not _has_start_time():
        try:
            os.kill(pid, 0)
        except ProcessLookupError:
            return False
        except OSError:
            return True
        return True
    start = process_start_for(pid)
    return start is not None and start == identity.get("pid_start")


def find_game_pid() -> int | None:
    """The PID of the running game, by executable name, or None."""
    names = [n.strip() for n in os.environ.get("GAME_EXE", "").split(",") if n.strip()] or list(GAME_EXES)
    if os.name != "nt":
        return None
    out = subprocess.run(["tasklist", "/fo", "csv", "/nh"], capture_output=True, text=True,
                         check=False).stdout
    rows = {row[0].lower(): int(row[1]) for row in csv.reader(io.StringIO(out)) if len(row) > 1
            and row[1].isdigit()}
    for name in names:
        if name.lower() in rows:
            return rows[name.lower()]
    return None


# --------------------------------------------------------------------------
# The record on disk
# --------------------------------------------------------------------------

def _refuse(tool: str, detail: str) -> dict[str, Any]:
    return {"ok": False, "tool": tool, "refused": True, "reason": "lease_unavailable",
            "detail": detail, "lease_path": str(record_path())}


def _valid(record: Any) -> bool:
    return (isinstance(record, dict)
            and record.get("schema") == SCHEMA
            and record.get("state") in (HELD, RELEASED)
            and isinstance(record.get("lease_id"), str)
            and isinstance(record.get("holder"), dict))


def read_record(tool: str = "game_lease") -> dict[str, Any] | None:
    """The parsed record; None when there is none; a `lease_unavailable`
    refusal when a file is there and cannot be read as one."""
    path = record_path()
    try:
        raw = path.read_bytes()
    except FileNotFoundError:
        return None
    except OSError as exc:
        return _refuse(tool, f"The lease record {path} could not be read ({type(exc).__name__}: "
                             f"{exc}), so whether another session holds the game is unknown. "
                             "`acquire --force` replaces it.")
    try:
        record = json.loads(raw.decode("utf-8"))
    except (UnicodeDecodeError, ValueError) as exc:
        record, why = None, f"{type(exc).__name__}: {exc}"
    else:
        why = f"it is not an {SCHEMA} record"
    if record is None or not _valid(record):
        return _refuse(tool, f"The lease record {path} is present but unreadable ({why}), so "
                             "whether another session holds the game is unknown and it is not "
                             "treated as free. `acquire --force` replaces it.")
    return record


def _write(record: dict[str, Any]) -> None:
    lease_dir().mkdir(parents=True, exist_ok=True)
    tmp = record_path().with_suffix(".tmp")
    tmp.write_text(json.dumps(record, indent=2), encoding="utf-8")
    for _ in range(200):
        try:
            os.replace(tmp, record_path())
            return
        except PermissionError:  # a reader has it open on Windows
            time.sleep(0.025)
    os.replace(tmp, record_path())


def status() -> dict[str, Any]:
    record = read_record()
    if record is None:
        return {"state": "free", "detail": f"no lease record at {record_path()}"}
    if record.get("refused"):
        return {"state": "unavailable", "detail": record["detail"]}
    if record["state"] == RELEASED:
        return {**record, "state": "free", "detail": f"{record.get('label')} released it"}
    if process_alive(record["holder"]):
        return {**record, "state": "held"}
    return {**record, "state": "stale", "detail": "the holder is no longer running"}


def acquire(label: str, pid: int | None, force: bool = False) -> tuple[int, dict[str, Any]]:
    if not LABEL_PATTERN.match(label):
        return 2, {"error": f"label {label!r} must match {LABEL_PATTERN.pattern}"}
    current = status()
    if current["state"] == "held" and not force:
        return 1, {"error": "the lease is held by another session; only its owner decides a "
                            "takeover (`--force`)", **current}
    if current["state"] == "unavailable" and not force:
        return 1, {"error": current["detail"]}
    pid = pid or find_game_pid()
    if not pid:
        return 1, {"error": "no running game process found; start the game or pass --pid"}
    start = process_start_for(pid)
    if start is None:
        return 1, {"error": f"no live process has pid {pid}"}
    record = {"schema": SCHEMA, "state": HELD, "lease_id": secrets.token_hex(8), "label": label,
              "holder": {"pid": pid, "pid_start": start, "platform": sys.platform},
              "acquired_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")}
    _write(record)
    return 0, record


def release() -> tuple[int, dict[str, Any]]:
    record = read_record()
    if record is None or record.get("refused"):
        return 1, {"error": "no readable lease record to release"}
    record["state"] = RELEASED
    _write(record)
    return 0, record


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    sub = parser.add_subparsers(dest="cmd", required=True)
    sub.add_parser("status")
    acq = sub.add_parser("acquire")
    acq.add_argument("--label", required=True)
    acq.add_argument("--pid", type=int, help="holder PID (default: the running game)")
    acq.add_argument("--force", action="store_true")
    sub.add_parser("release")
    args = parser.parse_args(argv)
    if args.cmd == "status":
        rc, out = 0, status()
    elif args.cmd == "acquire":
        rc, out = acquire(args.label, args.pid, args.force)
    else:
        rc, out = release()
    print(json.dumps(out, indent=2))
    return rc


if __name__ == "__main__":
    raise SystemExit(main())
