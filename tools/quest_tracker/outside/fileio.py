"""I1, the file-read trace: the probe, the trace export, the game's reads and their GGPK records.

Developer-only. Usage:

    py -3 -m tools.quest_tracker.outside.fileio probe [--live L]
    py -3 -m tools.quest_tracker.outside.fileio rehearsal-check [--live L]
    py -3 -m tools.quest_tracker.outside.fileio export [--live L] [--which session|rehearsal] [--force]
    py -3 -m tools.quest_tracker.outside.fileio reads [--live L] --pid P [--suffix \\Content.ggpk]
    py -3 -m tools.quest_tracker.outside.fileio map [--live L] [--tree F]

The trace itself is Windows' own Performance Recorder (`wpr`, profile
`FileIO`), started elevated by `fileio_controller.ps1`, the one elevated
script. It traces the operating system's file I/O log and never opens the
game's process. The controller runs `probe` inside a short rehearsal trace
first, so `rehearsal-check` can show what this trace sees before the session
relies on it:

- `probe` writes a 64 MiB `probe.bin` unbuffered and write-through (so none
  of it sits in the OS cache), reads 64 KiB at offset A (4 MiB) twice with
  `ReadFile` (the second read is served by the cache), then maps the file and
  touches one byte at offset B (32 MiB). It records its pid and the UTC
  before each step in `probe.json`, a second apart.
- `rehearsal-check` exports the rehearsal trace if needed and prints two
  capture lines. `i1-rehearsal` passes when both reads at A appear with the
  probe's pid, offset and size, one in each read's time slot; otherwise it
  fails naming what is missing (no events, a wrong pid, or only the first,
  uncached read: a cache-blind trace). `i1-rehearsal-mapped` passes when a
  read covering B is attributed to the probe, and is `not-observed`
  otherwise.

After the session, `export` turns an ETL into XML with the built-in
`tracerpt`, `reads` keeps the game's reads of `Content.ggpk` in
`i1\\reads.jsonl` (and counts writes to it, the patcher's), and `map` adds the
GGPK records each read touches (`ggpk.TreeIndex.lookup`) in
`i1\\mapped.jsonl`.

The XML is read as a stream. Its shape, as `tracerpt -of XML` writes kernel
events on this machine (measured on a local ETL, 2026-10-26): the event's
class GUID is in `ExtendedTracingInfo/EventGuid` and its class name in
`RenderingInfo/EventName` (the `Provider` GUID is the system trace's, and
`Task` is 0); the opcode is `System/Opcode`; fields are `EventData/Data`
elements found by `Name`, numbers padded with spaces or written in hex; and
`TimeCreated SystemTime` is local time with an offset that tracerpt rounds
down by a minute (`+01:59` for CEST), so the offset is rounded to the nearest
quarter hour. Which FileIo fields it decodes is not established until the
rehearsal; when a read or write carries no `Offset` field, its binary layout
is used (u64 Offset, three pointers, u32 TTID, u32 IoSize, u32 IoFlags), and
the printed counts say which route each read took.

A read's file comes from its FileObject through the last Create's
`OpenPath`, or else from its FileKey through the name events (Name,
FileCreate, FileRundown, whose `FileObject` carries the key); names that only
the end-of-trace rundown supplies are resolved after the pass. Its pid is the
header's `Execution ProcessID` when that is a real pid, or else its TTID
through the Thread Start, DCStart and DCEnd events. Paths are NT device
paths, so files are matched by a case-insensitive suffix.

Every writer refuses a path inside a git checkout, and the ETL, its XML and
the read lists stay in the live folder: they name every process's files.
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import mmap
import os
import re
import struct
import subprocess
import sys
import time
import xml.etree.ElementTree as ET
from pathlib import Path

from tools.quest_tracker.outside import common, ggpk
from tools.quest_tracker.outside.common import ToolError

MiB = 1 << 20
KiB = 1 << 10

PROBE_TOTAL = 64 * MiB
PROBE_OFFSET_A = 4 * MiB
PROBE_SIZE_A = 64 * KiB
PROBE_OFFSET_B = 32 * MiB
PROBE_PAUSE_S = 1.0
PROBE_CHUNK = 1 * MiB
# How early, in seconds, a trace event may sit before the probe's own UTC
# stamp for its step and still count as that step's.
SLOT_TOLERANCE_S = 0.25

FILEIO_GUID = "{90cbdc39-4a3e-11d1-84f4-0000f80464e3}"
THREAD_GUID = "{3d6fa8d1-fe05-11d0-9dda-00c04fd7ba7c}"
EVENTTRACE_GUID = "{68fdd900-4a3e-11d1-84f4-0000f80464e3}"
OP_CREATE, OP_READ, OP_WRITE = 64, 67, 68
NAME_OPS = (0, 32, 36)  # Name, FileCreate, FileRundown
THREAD_OPS = (1, 3, 4)  # Start, DCStart, DCEnd
# IRP_PAGING_IO in IoFlags, reported as a hint only: the rehearsal's mapped
# read shows the real flags (UNVERIFIED).
PAGING_FLAG = 0x2
# A pid the event header carries when it does not know the process.
NOT_A_PID = (0, 0xFFFFFFFF, -1)

DEFAULT_SUFFIX = "\\Content.ggpk"

_EVENT_NS = "{http://schemas.microsoft.com/win/2004/08/events/event}"
_TIME_RE = re.compile(
    r"^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})(?:\.(\d+))?(Z|[+-]\d{2}:\d{2})?$")
_FILETIME_EPOCH = _dt.datetime(1601, 1, 1, tzinfo=common.UTC)


# --------------------------------------------------------------------------
# Live folder paths

def i1_dir(live: Path) -> Path:
    return Path(live) / "i1"


def rehearsal_dir(live: Path) -> Path:
    return i1_dir(live) / "rehearsal"


def etl_and_xml(live: Path, which: str) -> tuple[Path, Path]:
    if which == "session":
        return i1_dir(live) / "session.etl", i1_dir(live) / "session.xml"
    if which == "rehearsal":
        return rehearsal_dir(live) / "rehearsal.etl", rehearsal_dir(live) / "rehearsal.xml"
    raise ToolError("unknown trace %r: expected session or rehearsal" % which)


def default_tree(live: Path) -> Path:
    """`tree-after.json` when the session's after-walk exists, else `tree-before.json`."""
    after = Path(live) / "ggpk" / "tree-after.json"
    return after if after.is_file() else Path(live) / "ggpk" / "tree-before.json"


def suffix_of(path: str) -> str:
    """A Win32 path without its drive, to match the NT device path a trace names."""
    rest = os.path.splitdrive(str(path))[1]
    return rest.replace("/", "\\")


# --------------------------------------------------------------------------
# The probe

def _win32():
    if sys.platform != "win32":
        raise ToolError("the probe calls Windows' file API and needs Windows")
    import ctypes
    from ctypes import wintypes
    k32 = ctypes.WinDLL("kernel32", use_last_error=True)
    k32.CreateFileW.restype = wintypes.HANDLE
    k32.CreateFileW.argtypes = [wintypes.LPCWSTR, wintypes.DWORD, wintypes.DWORD, ctypes.c_void_p,
                                wintypes.DWORD, wintypes.DWORD, wintypes.HANDLE]
    k32.WriteFile.restype = wintypes.BOOL
    k32.WriteFile.argtypes = [wintypes.HANDLE, ctypes.c_void_p, wintypes.DWORD,
                              ctypes.POINTER(wintypes.DWORD), ctypes.c_void_p]
    k32.ReadFile.restype = wintypes.BOOL
    k32.ReadFile.argtypes = [wintypes.HANDLE, ctypes.c_void_p, wintypes.DWORD,
                             ctypes.POINTER(wintypes.DWORD), ctypes.c_void_p]
    k32.SetFilePointerEx.restype = wintypes.BOOL
    k32.SetFilePointerEx.argtypes = [wintypes.HANDLE, ctypes.c_longlong,
                                     ctypes.POINTER(ctypes.c_longlong), wintypes.DWORD]
    k32.CloseHandle.restype = wintypes.BOOL
    k32.CloseHandle.argtypes = [wintypes.HANDLE]
    k32.VirtualAlloc.restype = ctypes.c_void_p
    k32.VirtualAlloc.argtypes = [ctypes.c_void_p, ctypes.c_size_t, wintypes.DWORD, wintypes.DWORD]
    k32.VirtualFree.restype = wintypes.BOOL
    k32.VirtualFree.argtypes = [ctypes.c_void_p, ctypes.c_size_t, wintypes.DWORD]
    return ctypes, wintypes, k32


_GENERIC_READ, _GENERIC_WRITE = 0x80000000, 0x40000000
_FILE_SHARE_READ = 0x1
_CREATE_ALWAYS, _OPEN_EXISTING = 2, 3
_FILE_ATTRIBUTE_NORMAL = 0x80
_FILE_FLAG_NO_BUFFERING, _FILE_FLAG_WRITE_THROUGH = 0x20000000, 0x80000000
_MEM_COMMIT_RESERVE, _MEM_RELEASE, _PAGE_READWRITE = 0x3000, 0x8000, 0x04
_INVALID_HANDLE = (1 << 64) - 1


def _check(ctypes, ok, what):
    if not ok:
        raise ToolError("%s failed: Windows error %d" % (what, ctypes.get_last_error()))


def _write_unbuffered(path: Path, total: int, chunk: int) -> None:
    ctypes, wintypes, k32 = _win32()
    h = k32.CreateFileW(str(path), _GENERIC_WRITE, 0, None, _CREATE_ALWAYS,
                        _FILE_FLAG_NO_BUFFERING | _FILE_FLAG_WRITE_THROUGH, None)
    if h is None or h == _INVALID_HANDLE:
        raise ToolError("CreateFileW(%s) for the unbuffered write failed: Windows error %d"
                        % (path, ctypes.get_last_error()))
    buf = k32.VirtualAlloc(None, chunk, _MEM_COMMIT_RESERVE, _PAGE_READWRITE)
    try:
        _check(ctypes, buf, "VirtualAlloc")
        pattern = bytes((i * 7 + 1) & 0xFF for i in range(4096)) * (chunk // 4096)
        ctypes.memmove(buf, pattern, chunk)
        written = wintypes.DWORD(0)
        for _ in range(total // chunk):
            _check(ctypes, k32.WriteFile(h, buf, chunk, ctypes.byref(written), None), "WriteFile")
            if written.value != chunk:
                raise ToolError("WriteFile wrote %d of %d bytes" % (written.value, chunk))
    finally:
        if buf:
            k32.VirtualFree(buf, 0, _MEM_RELEASE)
        k32.CloseHandle(h)


def _read_twice(path: Path, offset: int, size: int, pause: float, now) -> list[str]:
    """Two `ReadFile` calls of `size` bytes at `offset`, `pause` apart; the UTC before each."""
    ctypes, wintypes, k32 = _win32()
    h = k32.CreateFileW(str(path), _GENERIC_READ, _FILE_SHARE_READ, None, _OPEN_EXISTING,
                        _FILE_ATTRIBUTE_NORMAL, None)
    if h is None or h == _INVALID_HANDLE:
        raise ToolError("CreateFileW(%s) for the reads failed: Windows error %d"
                        % (path, ctypes.get_last_error()))
    stamps = []
    try:
        buf = ctypes.create_string_buffer(size)
        got = wintypes.DWORD(0)
        for k in range(2):
            if k:
                time.sleep(pause)
            _check(ctypes, k32.SetFilePointerEx(h, offset, None, 0), "SetFilePointerEx")
            stamps.append(common.format_utc(now()))
            _check(ctypes, k32.ReadFile(h, buf, size, ctypes.byref(got), None), "ReadFile")
            if got.value != size:
                raise ToolError("ReadFile read %d of %d bytes" % (got.value, size))
    finally:
        k32.CloseHandle(h)
    return stamps


def probe(live: Path, total: int = PROBE_TOTAL, offset_a: int = PROBE_OFFSET_A,
          size_a: int = PROBE_SIZE_A, offset_b: int = PROBE_OFFSET_B,
          pause: float = PROBE_PAUSE_S, now=common.now_utc) -> dict:
    """Write, read twice and map `probe.bin`; return and write `probe.json`."""
    folder = rehearsal_dir(live)
    common.refuse_inside_git(folder, "probe file")
    chunk = min(PROBE_CHUNK, total)
    if total % chunk or chunk % 4096 or offset_a % 4096 or size_a % 4096:
        raise ToolError("the probe's sizes and offset A must be multiples of 4096")
    if not (offset_a + size_a <= total and 0 <= offset_b < total):
        raise ToolError("the probe's offsets lie outside its %d bytes" % total)
    folder.mkdir(parents=True, exist_ok=True)
    path = folder / "probe.bin"
    _write_unbuffered(path, total, chunk)
    time.sleep(pause)
    reads_utc = _read_twice(path, offset_a, size_a, pause, now)
    time.sleep(pause)
    with open(path, "rb") as f:
        view = mmap.mmap(f.fileno(), 0, access=mmap.ACCESS_READ)
        try:
            mapped_utc = common.format_utc(now())
            _touched = view[offset_b]
        finally:
            view.close()
    record = {
        "pid": os.getpid(),
        "file": str(path),
        "offset_a": offset_a,
        "size_a": size_a,
        "reads_utc": reads_utc,
        "offset_b": offset_b,
        "mapped_utc": mapped_utc,
    }
    common.write_json(folder / "probe.json", record)
    return record


# --------------------------------------------------------------------------
# Export

def export(live: Path, which: str = "session", force: bool = False, run=subprocess.run) -> Path:
    """`tracerpt <etl> -o <xml> -of XML -lr -y`, unless the XML is already newer."""
    etl, xml = etl_and_xml(live, which)
    common.refuse_inside_git(xml, "trace export")
    if not etl.is_file():
        raise ToolError("no %s trace at %s" % (which, etl))
    if not force and xml.is_file() and xml.stat().st_mtime >= etl.stat().st_mtime:
        return xml
    proc = run(["tracerpt", str(etl), "-o", str(xml), "-of", "XML", "-lr", "-y"],
               cwd=str(etl.parent), capture_output=True, text=True)
    if proc.returncode != 0 or not xml.is_file():
        tail = " ".join(((proc.stdout or "") + " " + (proc.stderr or "")).split())[-300:]
        raise ToolError("tracerpt exited %d on %s: %s" % (proc.returncode, etl, tail))
    return xml


# --------------------------------------------------------------------------
# The trace parser

def parse_trace_time(text: str) -> _dt.datetime:
    """`TimeCreated SystemTime` as UTC, its offset rounded to the nearest quarter hour."""
    m = _TIME_RE.match(text.strip())
    if not m:
        raise ValueError("not a trace timestamp: %r" % (text,))
    year, month, day, hour, minute, second = (int(g) for g in m.groups()[:6])
    micro = int((m.group(7) or "0")[:6].ljust(6, "0"))
    zone = m.group(8) or "Z"
    if zone == "Z":
        offset_min = 0
    else:
        sign = -1 if zone[0] == "-" else 1
        offset_min = sign * (int(zone[1:3]) * 60 + int(zone[4:6]))
        offset_min = int(round(offset_min / 15.0)) * 15
    local = _dt.datetime(year, month, day, hour, minute, second, micro, tzinfo=common.UTC)
    return local - _dt.timedelta(minutes=offset_min)


def _num(text):
    if text is None:
        return None
    s = text.strip()
    if not s:
        return None
    try:
        return int(s, 16) if s[:2].lower() == "0x" else int(s, 10)
    except ValueError:
        return None


def _local(tag: str) -> str:
    return tag.rsplit("}", 1)[-1]


class _Event:
    __slots__ = ("cls", "opcode", "time", "pid", "tid", "data", "binary")


def _read_event(el) -> _Event:
    ev = _Event()
    ev.cls, ev.opcode, ev.time, ev.pid, ev.tid, ev.binary = None, None, None, None, None, None
    ev.data = {}
    guid = provider = name = None
    for child in el:
        ln = _local(child.tag)
        if ln == "System":
            for c in child:
                cl = _local(c.tag)
                if cl == "Opcode":
                    ev.opcode = _num(c.text)
                elif cl == "TimeCreated":
                    ev.time = c.get("SystemTime")
                elif cl == "Execution":
                    ev.pid = _num(c.get("ProcessID"))
                    ev.tid = _num(c.get("ThreadID"))
                elif cl == "Provider":
                    provider = (c.get("Guid") or "").lower()
        elif ln == "EventData":
            for d in child:
                key = d.get("Name")
                if key is not None:
                    ev.data[key] = d.text
        elif ln == "BinaryEventData":
            ev.binary = (child.text or "").strip()
        elif ln == "RenderingInfo":
            for c in child:
                if _local(c.tag) in ("EventName", "Task") and c.text and c.text.strip():
                    name = name or c.text.strip().lower()
        elif ln == "ExtendedTracingInfo":
            for c in child:
                if _local(c.tag) == "EventGuid" and c.text:
                    guid = c.text.strip().lower()
    if FILEIO_GUID in (guid, provider) or name == "fileio":
        ev.cls = "fileio"
    elif THREAD_GUID in (guid, provider) or name == "thread":
        ev.cls = "thread"
    elif EVENTTRACE_GUID in (guid, provider) or name == "eventtrace":
        ev.cls = "eventtrace"
    return ev


def _binary_rw(blob: str):
    """(offset, file_object, file_key, ttid, size, flags) from a read/write's raw payload."""
    hexs = blob[2:] if blob[:2].lower() == "0x" else blob
    try:
        raw = bytes.fromhex(hexs)
    except ValueError:
        return None
    if len(raw) < 44:
        return None
    offset, _irp, fo, fk, ttid, size, flags = struct.unpack_from("<QQQQIII", raw, 0)
    return offset, fo, fk, ttid, size, flags


def _rw_fields(ev: _Event):
    """(fields, route) of a read or write, or (None, None) when neither route reads it."""
    d = ev.data
    if d.get("Offset") is not None and _num(d.get("Offset")) is not None:
        return (_num(d.get("Offset")), _num(d.get("FileObject")), _num(d.get("FileKey")),
                _num(d.get("TTID")), _num(d.get("IoSize")), _num(d.get("IoFlags")) or 0), "decoded"
    blob = ev.binary
    if blob is None:
        # An undecoded payload some tracerpt builds put in a single hex Data element.
        blobs = [v for v in d.values() if v and v.strip()[:2].lower() == "0x" and len(v.strip()) >= 90]
        blob = blobs[0].strip() if len(blobs) == 1 else None
    if blob:
        fields = _binary_rw(blob)
        if fields:
            return fields, "binary"
    return None, None


class TraceScan:
    """What one pass over a tracerpt XML found."""

    def __init__(self):
        self.reads: list[dict] = []
        self.writes: list[dict] = []
        self.stats = {"events": 0, "fileio_events": 0, "fileio_reads": 0, "fileio_writes": 0,
                      "route_decoded": 0, "route_binary": 0, "undecodable": 0,
                      "unresolved_name": 0, "unresolved_pid": 0, "clock_check_ms": None}


def scan_trace(xml: Path, pid: int | None = None, suffix: str | None = None) -> TraceScan:
    """Every read and write by `pid` (any when None) of a file ending in `suffix` (any when None)."""
    xml = Path(xml)
    if not xml.is_file():
        raise ToolError("no trace export at %s: run `fileio export` first" % xml)
    want_suffix = suffix.lower() if suffix else None
    object_path: dict[int, str] = {}
    key_name: dict[int, str] = {}
    thread_pid: dict[int, int] = {}
    pending: list[dict] = []
    scan = TraceScan()
    st = scan.stats

    def name_ok(name):
        return name is None or want_suffix is None or name.lower().endswith(want_suffix)

    def pid_ok(p):
        return p is None or pid is None or p == pid

    try:
        context = ET.iterparse(str(xml), events=("start", "end"))
        root = None
        for event, el in context:
            if event == "start":
                if root is None:
                    root = el
                continue
            if el.tag != _EVENT_NS + "Event" and _local(el.tag) != "Event":
                continue
            st["events"] += 1
            ev = _read_event(el)
            el.clear()
            if root is not None:
                root.clear()
            if ev.cls == "thread":
                if ev.opcode in THREAD_OPS:
                    t, p = _num(ev.data.get("TThreadId")), _num(ev.data.get("ProcessId"))
                    if t is not None and p is not None:
                        thread_pid[t] = p
                continue
            if ev.cls == "eventtrace":
                start = _num(ev.data.get("StartTime"))
                if ev.opcode == 0 and start and ev.time and st["clock_check_ms"] is None:
                    ft = _FILETIME_EPOCH + _dt.timedelta(microseconds=start // 10)
                    st["clock_check_ms"] = round(
                        (parse_trace_time(ev.time) - ft).total_seconds() * 1000.0, 3)
                continue
            if ev.cls != "fileio":
                continue
            st["fileio_events"] += 1
            if ev.opcode == OP_CREATE:
                fo, path = _num(ev.data.get("FileObject")), ev.data.get("OpenPath")
                if fo is not None and path:
                    object_path[fo] = path.strip()
                continue
            if ev.opcode in NAME_OPS:
                fo, path = _num(ev.data.get("FileObject")), ev.data.get("FileName")
                if fo is not None and path:
                    key_name[fo] = path.strip()
                continue
            if ev.opcode not in (OP_READ, OP_WRITE):
                continue
            st["fileio_reads" if ev.opcode == OP_READ else "fileio_writes"] += 1
            fields, route = _rw_fields(ev)
            if fields is None:
                st["undecodable"] += 1
                continue
            st["route_" + route] += 1
            offset, fo, fk, ttid, size, flags = fields
            name, name_source = None, None
            if fo is not None and fo in object_path:
                name, name_source = object_path[fo], "FileObject"
            elif fk is not None and fk in key_name:
                name, name_source = key_name[fk], "FileKey"
            if ev.pid is not None and ev.pid not in NOT_A_PID:
                p, pid_source = ev.pid, "header"
            elif ttid is not None and ttid in thread_pid:
                p, pid_source = thread_pid[ttid], "thread"
            else:
                p, pid_source = None, None
            # Writes are kept from every pid: the patcher, not the game, writes the GGPK.
            if not (name_ok(name) and (ev.opcode == OP_WRITE or pid_ok(p))):
                continue
            pending.append({
                "utc": parse_trace_time(ev.time) if ev.time else None,
                "pid": p, "pid_source": pid_source,
                "tid": ttid if ttid is not None else ev.tid,
                "op": "read" if ev.opcode == OP_READ else "write",
                "file": name, "name_source": name_source,
                "offset": offset, "size": size, "flags": flags,
                "_fk": fk, "_ttid": ttid, "route": route,
            })
    except ET.ParseError as err:
        raise ToolError("the trace export %s does not parse after %d events: %s"
                        % (xml, st["events"], err))

    for rec in pending:
        if rec["file"] is None and rec["_fk"] in key_name:
            rec["file"], rec["name_source"] = key_name[rec["_fk"]], "FileKey"
        if rec["pid"] is None and rec["_ttid"] in thread_pid:
            rec["pid"], rec["pid_source"] = thread_pid[rec["_ttid"]], "thread"
        if rec["file"] is None:
            st["unresolved_name"] += 1
            continue
        is_read = rec["op"] == "read"
        if rec["pid"] is None:
            st["unresolved_pid"] += 1
            if pid is not None and is_read:
                continue
        if not (want_suffix is None or rec["file"].lower().endswith(want_suffix)):
            continue
        if pid is not None and is_read and rec["pid"] != pid:
            continue
        del rec["_fk"], rec["_ttid"]
        (scan.reads if rec["op"] == "read" else scan.writes).append(rec)
    scan.reads.sort(key=lambda r: (r["utc"] is None, r["utc"] or _FILETIME_EPOCH))
    return scan


def _jsonable(rec: dict) -> dict:
    out = dict(rec)
    out.pop("route", None)
    if isinstance(out.get("utc"), _dt.datetime):
        out["utc"] = common.format_utc(out["utc"])
    return out


def _write_jsonl(path: Path, records) -> int:
    path = Path(path)
    common.refuse_inside_git(path, "JSON-lines file")
    n = 0
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        for rec in records:
            f.write(json.dumps(rec, ensure_ascii=False) + "\n")
            n += 1
    return n


# --------------------------------------------------------------------------
# reads and map

def reads(live: Path, pid: int, suffix: str = DEFAULT_SUFFIX, xml: Path | None = None) -> dict:
    """Write `i1\\reads.jsonl`: the reads `pid` made of files ending in `suffix`."""
    out = i1_dir(live) / "reads.jsonl"
    common.refuse_inside_git(out, "read list")
    xml = Path(xml) if xml else etl_and_xml(live, "session")[1]
    scan = scan_trace(xml, pid=pid, suffix=suffix)
    mine = scan.reads
    n = _write_jsonl(out, (_jsonable(r) for r in mine))
    st = scan.stats
    by_source = {}
    for r in mine:
        by_source[r["pid_source"]] = by_source.get(r["pid_source"], 0) + 1
    return {
        "reads": n,
        "pid_sources": by_source,
        "writes": len(scan.writes),
        "write_pids": sorted({w["pid"] for w in scan.writes if w["pid"] is not None}),
        "stats": st,
        "out": out,
    }


def map_reads(live: Path, tree_path: Path | None = None) -> dict:
    """Write `i1\\mapped.jsonl`: each read with the GGPK records it touches."""
    live = Path(live)
    src = i1_dir(live) / "reads.jsonl"
    out = i1_dir(live) / "mapped.jsonl"
    common.refuse_inside_git(out, "mapped read list")
    if not src.is_file():
        raise ToolError("no read list at %s: run `fileio reads` first" % src)
    tree_path = Path(tree_path) if tree_path else default_tree(live)
    index = ggpk.TreeIndex(ggpk.load_tree(tree_path))
    kinds: dict[str, int] = {}

    def mapped():
        for rec in common.read_jsonl(src):
            entries = index.lookup(int(rec["offset"]), max(1, int(rec["size"] or 1)))
            for e in entries:
                kinds[e["kind"]] = kinds.get(e["kind"], 0) + 1
            out_rec = dict(rec)
            out_rec["records"] = entries
            yield out_rec

    n = _write_jsonl(out, mapped())
    return {"reads": n, "kinds": kinds, "tree": tree_path, "out": out}


# --------------------------------------------------------------------------
# rehearsal-check

def check_line(name: str, expected: str, observed: str, verdict: str) -> str:
    clean = lambda s: " ".join(str(s).replace("|", "/").split())
    return "- %s | expected: %s | observed: %s | %s" % (name, clean(expected), clean(observed), verdict)


def _not_run(why: str) -> list[str]:
    return [check_line("i1-rehearsal", "both probe reads at offset A by the probe's pid",
                       "nothing to read: " + why, "not-run (instrument: %s)" % why),
            check_line("i1-rehearsal-mapped", "a read of offset B's page by the probe's pid",
                       "nothing to read: " + why, "not-run (instrument: %s)" % why)]


def _rel(t: _dt.datetime, ref: _dt.datetime) -> str:
    return "%+.3f s" % (t - ref).total_seconds()


def judge_rehearsal(probe_rec: dict, scan: TraceScan) -> list[str]:
    """The two capture lines, from the probe's record and a scan of the probe file's events."""
    pid = int(probe_rec["pid"])
    a, size_a, b = int(probe_rec["offset_a"]), int(probe_rec["size_a"]), int(probe_rec["offset_b"])
    r0, r1 = (common.parse_utc(t) for t in probe_rec["reads_utc"])
    mapped_at = common.parse_utc(probe_rec["mapped_utc"])
    tol = _dt.timedelta(seconds=SLOT_TOLERANCE_S)
    lines = []

    expected = "two reads of %d bytes at offset %d by pid %d, one after each probe read starts" % (
        size_a, a, pid)
    at_a = [r for r in scan.reads if r["offset"] == a and r["size"] == size_a and r["utc"]]
    mine = [r for r in at_a if r["pid"] == pid]
    first = [r for r in mine if r0 - tol <= r["utc"] < r1 - tol]
    second = [r for r in mine if r["utc"] >= r1 - tol]
    if first and second:
        f, s = first[0], second[0]
        observed = ("first read %s after its start (flags 0x%x, pid from %s), second %s after its "
                    "start (flags 0x%x, pid from %s); FileIo events %d"
                    % (_rel(f["utc"], r0), f["flags"], f["pid_source"], _rel(s["utc"], r1),
                       s["flags"], s["pid_source"], scan.stats["fileio_events"]))
        verdict = "pass"
    elif not scan.stats["fileio_events"]:
        observed, verdict = "no FileIo events in the trace (%d events)" % scan.stats["events"], "fail"
    elif not scan.reads:
        observed = ("no reads of the probe file among %d FileIo events (decoded %d, binary %d, "
                    "undecodable %d)" % (scan.stats["fileio_events"], scan.stats["route_decoded"],
                                         scan.stats["route_binary"], scan.stats["undecodable"]))
        verdict = "fail"
    elif at_a and not mine:
        others = sorted({r["pid"] for r in at_a}, key=lambda p: (p is None, p or 0))
        observed = "reads at offset A found, but by pid %s, not the probe's pid %d" % (
            ", ".join(str(p) for p in others), pid)
        verdict = "fail"
    elif first and not second:
        observed = ("only the first, uncached read at offset A; no event for the second read, "
                    "served by the cache: the trace is cache-blind")
        verdict = "fail"
    elif second and not first:
        observed = "only the second read at offset A; the first, uncached read is missing"
        verdict = "fail"
    else:
        sizes = sorted({(r["offset"], r["size"]) for r in scan.reads})[:6]
        observed = "no read of %d bytes at offset %d; the probe file's reads were (offset, size) %s" % (
            size_a, a, sizes)
        verdict = "fail"
    lines.append(check_line("i1-rehearsal", expected, observed, verdict))

    expected = "a read covering offset %d (the mapped page) by pid %d" % (b, pid)
    covering = [r for r in scan.reads if r["offset"] <= b < r["offset"] + max(1, r["size"] or 0)
                and r["utc"] and r["utc"] >= mapped_at - tol]
    hit = [r for r in covering if r["pid"] == pid]
    if hit:
        r = hit[0]
        observed = ("read at offset %d, %d bytes, %s after the touch, flags 0x%x (paging bit %s), "
                    "pid from %s" % (r["offset"], r["size"], _rel(r["utc"], mapped_at), r["flags"],
                                     "set" if r["flags"] & PAGING_FLAG else "clear", r["pid_source"]))
        verdict = "pass"
    elif covering:
        observed = "a read covers offset B, but by pid %s" % ", ".join(
            sorted({str(r["pid"]) for r in covering}))
        verdict = "not-observed"
    else:
        observed = "no read of the probe file covers offset B after the touch"
        verdict = "not-observed"
    lines.append(check_line("i1-rehearsal-mapped", expected, observed, verdict))
    return lines


def rehearsal_check(live: Path, run=subprocess.run) -> list[str]:
    folder = rehearsal_dir(live)
    probe_json = folder / "probe.json"
    if not probe_json.is_file():
        return _not_run("no probe.json; the controller did not reach the probe")
    probe_rec = common.read_json(probe_json)
    etl, xml = etl_and_xml(live, "rehearsal")
    if not xml.is_file() or (etl.is_file() and etl.stat().st_mtime > xml.stat().st_mtime):
        if not etl.is_file():
            return _not_run("no rehearsal.etl; the rehearsal trace was not saved")
        try:
            export(live, "rehearsal", run=run)
        except ToolError as err:
            return _not_run(str(err))
    scan = scan_trace(xml, pid=None, suffix=suffix_of(probe_rec["file"]))
    return judge_rehearsal(probe_rec, scan)


# --------------------------------------------------------------------------
# Command line

def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="fileio", description="The file-read probe, the trace export, the game's reads and their records.")
    sub = parser.add_subparsers(dest="command", required=True)
    p = sub.add_parser("probe", help="write, read twice and map probe.bin (run inside the rehearsal trace)")
    p.add_argument("--live", type=Path, default=None)
    r = sub.add_parser("rehearsal-check", help="print i1-rehearsal and i1-rehearsal-mapped")
    r.add_argument("--live", type=Path, default=None)
    e = sub.add_parser("export", help="tracerpt an ETL to XML")
    e.add_argument("--live", type=Path, default=None)
    e.add_argument("--which", choices=("session", "rehearsal"), default="session")
    e.add_argument("--force", action="store_true")
    rd = sub.add_parser("reads", help="write i1\\reads.jsonl: one process's reads of one file")
    rd.add_argument("--live", type=Path, default=None)
    rd.add_argument("--pid", type=int, required=True)
    rd.add_argument("--suffix", default=DEFAULT_SUFFIX)
    m = sub.add_parser("map", help="write i1\\mapped.jsonl: each read's GGPK records")
    m.add_argument("--live", type=Path, default=None)
    m.add_argument("--tree", type=Path, default=None)
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    live = args.live if args.live is not None else common.default_live_dir()
    try:
        if args.command == "probe":
            rec = probe(live)
            print("fileio probe: pid %d, reads at %s, mapped at %s"
                  % (rec["pid"], ", ".join(rec["reads_utc"]), rec["mapped_utc"]))
        elif args.command == "rehearsal-check":
            for line in rehearsal_check(live):
                print(line)
        elif args.command == "export":
            started = time.perf_counter()
            xml = export(live, args.which, force=args.force)
            print("fileio export: %s (%d bytes) in %.1f s"
                  % (xml, xml.stat().st_size, time.perf_counter() - started))
        elif args.command == "reads":
            res = reads(live, args.pid, args.suffix)
            st = res["stats"]
            print("fileio reads: %d reads by pid %d of *%s (pid from %s); "
                  "%d writes to it (pids %s)"
                  % (res["reads"], args.pid, args.suffix,
                     ", ".join("%s %d" % kv for kv in sorted(res["pid_sources"].items())) or "none",
                     res["writes"], ", ".join(str(p) for p in res["write_pids"]) or "none"))
            print("fileio reads: %d events, %d FileIo; route decoded %d, binary %d, undecodable %d; "
                  "unresolved name %d, pid %d; clock check %s ms"
                  % (st["events"], st["fileio_events"], st["route_decoded"], st["route_binary"],
                     st["undecodable"], st["unresolved_name"], st["unresolved_pid"],
                     st["clock_check_ms"]))
        else:
            res = map_reads(live, args.tree)
            print("fileio map: %d reads through %s; entries by kind: %s"
                  % (res["reads"], res["tree"],
                     ", ".join("%s %d" % kv for kv in sorted(res["kinds"].items())) or "none"))
    except ToolError as err:
        print("fileio: error: %s" % err, file=sys.stderr)
        return 1
    except (OSError, ValueError, KeyError, json.JSONDecodeError) as err:
        print("fileio: error: %s" % err, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
