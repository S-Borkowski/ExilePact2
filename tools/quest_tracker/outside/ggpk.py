"""The GGPK directory: walk it, look an offset up in it, diff two walks, copy one record.

Developer-only. Usage:

    py -3 -m tools.quest_tracker.outside.ggpk walk [--ggpk G] --out F
    py -3 -m tools.quest_tracker.outside.ggpk lookup --tree F <offset> [--size n]
    py -3 -m tools.quest_tracker.outside.ggpk diff A B
    py -3 -m tools.quest_tracker.outside.ggpk copy [--ggpk G] --tree F --name <path> --out O
    py -3 -m tools.quest_tracker.outside.ggpk selfcheck [--ggpk G]

`Content.ggpk` is one large file of records, in the public format the
community's GGPK readers document. Each record starts with its whole length
(u32) and a four-byte tag:

- `GGPK`, at offset 0: a format version (3 means UTF-16LE names, 4 UTF-32LE),
  the offset of the root `PDIR` and the head of the `FREE` list (u64 each).
- `PDIR`, a directory: its name length in characters (terminator included),
  its entry count, a SHA-256, the name, then per entry a name hash (u32) and
  the entry's record offset (u64).
- `FILE`: its name length, a SHA-256, the name, then the file's data up to the
  record's end.
- `FREE`, unused space: the offset of the next `FREE` record (u64, 0 ends it).

A file under `/Bundles2` named `*.bundle.bin` (and `_.index.bin`) is a bundle:
three u32 (uncompressed size, total payload size, head payload size), then a
head of a u32 first-file encode, a u32 unknown, a u64 uncompressed size, a u64
total payload size, a u32 block count, a u32 granularity (262,144), four u32
unknown and one u32 compressed size per block. The compressed blocks follow
back to back from 12 + the head payload size, and block k holds the
uncompressed bytes from k x granularity.

`walk` reads only those record and bundle headers, never a file's contents,
across the whole tree, and writes a tree file: every `FILE` (path, record
offset and length, data offset and size, and a bundle's header), every `PDIR`
and every `FREE` record. `lookup` maps a byte range of `Content.ggpk` to the
records it touches, as entries `{name, kind, rel_offset, blocks}`:

- `file`: a file's data; `rel_offset` is from the data's start.
- `bundle`: a bundle's compressed blocks; `blocks` lists the block indexes
  touched and `rel_offset` is from the bundle file's start.
- `bundle-header`: a bundle's size fields and head, `rel_offset` as above.
- `record-header`: a record's own header (a `FILE`'s length, tag, hash and
  name, a whole `PDIR`, the `GGPK` record); `rel_offset` is from the record's
  start.
- `free`: a `FREE` record, `rel_offset` from its start.
- `unknown`: bytes no walked record covers (past the end, or space no
  directory or free list reaches); `rel_offset` is the absolute offset.

A read that spans several records returns all of them, in offset order.
`diff` compares two tree files and exits 1 listing what moved, changed, was
added or removed. `copy` copies one named file's data out, after checking
that the record at the tree's offset is still that file. `selfcheck` walks in
memory, writes nothing, and checks that the records the live session needs
exist and that lookup maps their first and last bytes and blocks back to
them.

Every command that writes refuses a path inside a git checkout. Read the GGPK
only while the game is closed.
"""
from __future__ import annotations

import argparse
import bisect
import json
import struct
import sys
import time
from pathlib import Path

from tools.quest_tracker.outside import common
from tools.quest_tracker.outside.common import ToolError

TREE_FORMAT = "quest-tracker-ggpk-tree/1"
GRANULARITY = 262144
# The bundle head before its per-block sizes: 4 + 4 + 8 + 8 + 4 + 4 + 16 bytes.
BUNDLE_HEAD_FIXED = 48
# Bytes read at a record's start in one go; a longer header reads the rest.
HEADER_CHUNK = 4096
COPY_CHUNK = 1 << 20

KINDS = ("file", "bundle", "bundle-header", "record-header", "free", "unknown")

# What `selfcheck` looks for (case-insensitive).
SELFCHECK_BANK = "/FMOD/Desktop/CarrionCrone.bank"
SELFCHECK_INDEX = "/Bundles2/_.index.bin"
SELFCHECK_DIALOGUE_PREFIX = "/Bundles2/Streaming/Folders/audio/dialogue/4.0/monsters."


def is_bundle_name(name: str) -> bool:
    low = name.lower()
    return low.startswith("/bundles2/") and (low.endswith(".bundle.bin") or low.endswith("/_.index.bin"))


# --------------------------------------------------------------------------
# The walk

class _Reader:
    def __init__(self, f, size: int):
        self.f = f
        self.size = size

    def read_at(self, offset: int, n: int) -> bytes:
        if offset < 0 or offset + n > self.size:
            raise ToolError("a read of %d bytes at offset %d runs past the end of the GGPK (%d bytes)"
                            % (n, offset, self.size))
        self.f.seek(offset)
        data = self.f.read(n)
        if len(data) != n:
            raise ToolError("short read of %d bytes at offset %d" % (n, offset))
        return data

    def head(self, offset: int) -> tuple[int, bytes, bytes]:
        """A record's length, tag, and up to HEADER_CHUNK bytes from its start."""
        n = min(HEADER_CHUNK, self.size - offset)
        if n < 8:
            raise ToolError("no record fits at offset %d (the GGPK is %d bytes)" % (offset, self.size))
        buf = self.read_at(offset, n)
        length, tag = struct.unpack_from("<I4s", buf, 0)
        if length < 8 or offset + length > self.size:
            raise ToolError("the %r record at offset %d claims %d bytes, past the GGPK's end"
                            % (tag, offset, length))
        return length, tag, buf[:length]

    def span(self, offset: int, buf: bytes, need: int) -> bytes:
        """`buf` (read at `offset`) extended to at least `need` bytes."""
        if len(buf) >= need:
            return buf
        return self.read_at(offset, need)


def _decode_name(raw: bytes, version: int) -> str:
    enc = "utf-32-le" if version == 4 else "utf-16-le"
    return raw.decode(enc, errors="replace").rstrip("\0")


def _bundle_header(reader: _Reader, data_offset: int, size: int) -> dict:
    """A bundle's header fields, or `{"error": ...}` when they do not add up."""
    if size < 12 + BUNDLE_HEAD_FIXED:
        return {"error": "the file is %d bytes, too small for a bundle header" % size}
    first = reader.read_at(data_offset, min(size, HEADER_CHUNK))
    _uncompressed, payload, head = struct.unpack_from("<III", first, 0)
    if head < BUNDLE_HEAD_FIXED or 12 + head > size:
        return {"error": "head payload size %d does not fit the file's %d bytes" % (head, size)}
    buf = reader.span(data_offset, first, 12 + head)
    (_encode, _unk, uncompressed, payload2, count, granularity) = struct.unpack_from("<IIQQII", buf, 12)
    if head != BUNDLE_HEAD_FIXED + 4 * count:
        return {"error": "head payload size %d does not match %d blocks" % (head, count)}
    blocks = list(struct.unpack_from("<%dI" % count, buf, 12 + BUNDLE_HEAD_FIXED))
    total = sum(blocks)
    if 12 + head + total > size:
        return {"error": "its %d blocks (%d bytes) run past the file's %d bytes" % (count, total, size)}
    out = {"head": head, "uncompressed": uncompressed, "payload": payload2,
           "granularity": granularity, "blocks": blocks}
    if total != payload or payload2 != payload:
        out["warning"] = "the block sizes sum to %d, the header says %d" % (total, payload)
    return out


def walk_ggpk(path: Path, progress=None) -> dict:
    """The whole directory tree of the GGPK at `path`, as a tree dict.

    Reads record headers and bundle headers only. Raises ToolError on a
    record that is not where or what its parent says.
    """
    path = Path(path)
    if not path.is_file():
        raise ToolError("no GGPK at %s" % path)
    started = time.perf_counter()
    size = path.stat().st_size
    records: list[dict] = []
    counts = {"files": 0, "dirs": 0, "free": 0, "bundles": 0, "bundle_errors": 0,
              "bundle_warnings": 0, "duplicates": 0}
    with open(path, "rb") as f:
        reader = _Reader(f, size)
        length, tag, buf = reader.head(0)
        if tag != b"GGPK" or length < 28:
            raise ToolError("%s does not start with a GGPK record (tag %r)" % (path, tag))
        version, = struct.unpack_from("<I", buf, 8)
        root, free_head = struct.unpack_from("<QQ", buf, 12)
        if version not in (3, 4):
            raise ToolError("unknown GGPK version %d (3 and 4 are known)" % version)
        cw = 4 if version == 4 else 2
        records.append({"kind": "ggpk", "offset": 0, "length": length})

        seen: set[int] = set()
        stack = [(root, "", True)]
        while stack:
            offset, prefix, is_root = stack.pop()
            if offset in seen:
                counts["duplicates"] += 1
                continue
            seen.add(offset)
            length, tag, buf = reader.head(offset)
            if tag == b"PDIR":
                name_len, count = struct.unpack_from("<II", buf, 8)
                need = 16 + 32 + name_len * cw + 12 * count
                if need > length:
                    raise ToolError("the directory at offset %d needs %d bytes, its record has %d"
                                    % (offset, need, length))
                buf = reader.span(offset, buf, need)
                name = _decode_name(buf[48:48 + name_len * cw], version)
                here = prefix + "/" + name if name else prefix
                records.append({"kind": "dir", "name": here or "/", "offset": offset, "length": length})
                counts["dirs"] += 1
                base = 48 + name_len * cw
                for k in range(count):
                    _hash, child = struct.unpack_from("<IQ", buf, base + 12 * k)
                    stack.append((child, here, False))
                if progress and counts["dirs"] % 2000 == 0:
                    progress(counts)
            elif tag == b"FILE":
                name_len, = struct.unpack_from("<I", buf, 8)
                header = 12 + 32 + name_len * cw
                if header > length:
                    raise ToolError("the file record at offset %d has a %d-byte header in %d bytes"
                                    % (offset, header, length))
                buf = reader.span(offset, buf, header)
                name = prefix + "/" + _decode_name(buf[44:header], version)
                rec = {"kind": "file", "name": name, "offset": offset, "length": length,
                       "data": offset + header, "size": length - header}
                if is_bundle_name(name):
                    bundle = _bundle_header(reader, rec["data"], rec["size"])
                    if "error" in bundle:
                        rec["bundle_error"] = bundle["error"]
                        counts["bundle_errors"] += 1
                    else:
                        rec["bundle"] = bundle
                        counts["bundles"] += 1
                        counts["bundle_warnings"] += "warning" in bundle
                records.append(rec)
                counts["files"] += 1
            elif is_root:
                raise ToolError("the root at offset %d is a %r record, not PDIR" % (offset, tag))
            else:
                raise ToolError("the entry of %s at offset %d is a %r record, not PDIR or FILE"
                                % (prefix or "/", offset, tag))

        offset, hops = free_head, 0
        while offset:
            if offset in seen:
                raise ToolError("the FREE list loops back to offset %d" % offset)
            seen.add(offset)
            length, tag, buf = reader.head(offset)
            if tag != b"FREE":
                raise ToolError("the FREE list reaches a %r record at offset %d" % (tag, offset))
            records.append({"kind": "free", "offset": offset, "length": length})
            counts["free"] += 1
            offset, = struct.unpack_from("<Q", buf, 8)
            hops += 1

    records.sort(key=lambda r: r["offset"])
    return {
        "format": TREE_FORMAT,
        "ggpk": str(path),
        "ggpk_size": size,
        "version": version,
        "root": root,
        "free_head": free_head,
        "counts": counts,
        "walk_seconds": round(time.perf_counter() - started, 3),
        "records": records,
    }


def write_tree(path: Path, tree: dict) -> None:
    """The tree as JSON, one record per line so two walks diff line by line."""
    path = Path(path)
    common.refuse_inside_git(path, "GGPK tree")
    path.parent.mkdir(parents=True, exist_ok=True)
    head = {k: v for k, v in tree.items() if k != "records"}
    compact = dict(separators=(",", ":"), ensure_ascii=False)
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write(json.dumps(head, **compact)[:-1] + ',"records":[\n')
        records = tree["records"]
        for i, rec in enumerate(records):
            f.write(json.dumps(rec, **compact) + (",\n" if i + 1 < len(records) else "\n"))
        f.write("]}\n")


def load_tree(path: Path) -> dict:
    path = Path(path)
    if not path.is_file():
        raise ToolError("no GGPK tree at %s: run `ggpk walk` first" % path)
    tree = json.loads(path.read_text(encoding="utf-8"))
    if tree.get("format") != TREE_FORMAT:
        raise ToolError("%s is not a GGPK tree (format %r, expected %r)"
                        % (path, tree.get("format"), TREE_FORMAT))
    return tree


# --------------------------------------------------------------------------
# Lookup

class _Span:
    __slots__ = ("start", "end", "kind", "name", "base", "cum", "block_start")

    def __init__(self, start, end, kind, name, base, cum=None, block_start=0):
        self.start, self.end, self.kind, self.name, self.base = start, end, kind, name, base
        self.cum, self.block_start = cum, block_start


class TreeIndex:
    """Offset lookup over a tree's records."""

    def __init__(self, tree: dict):
        self.size = int(tree.get("ggpk_size") or 0)
        spans: list[_Span] = []
        for rec in tree["records"]:
            off, length = rec["offset"], rec["length"]
            kind = rec["kind"]
            if kind == "ggpk":
                spans.append(_Span(off, off + length, "record-header", None, off))
            elif kind == "dir":
                spans.append(_Span(off, off + length, "record-header", rec["name"], off))
            elif kind == "free":
                spans.append(_Span(off, off + length, "free", None, off))
            elif kind == "file":
                data, size, name = rec["data"], rec["size"], rec["name"]
                spans.append(_Span(off, data, "record-header", name, off))
                bundle = rec.get("bundle")
                if bundle:
                    head_end = data + 12 + bundle["head"]
                    spans.append(_Span(data, head_end, "bundle-header", name, data))
                    cum = [0]
                    for b in bundle["blocks"]:
                        cum.append(cum[-1] + b)
                    blocks_end = head_end + cum[-1]
                    if blocks_end > head_end:
                        spans.append(_Span(head_end, blocks_end, "bundle", name, data, cum, head_end))
                    if data + size > blocks_end:
                        spans.append(_Span(blocks_end, data + size, "file", name, data))
                elif size > 0:
                    spans.append(_Span(data, data + size, "file", name, data))
        spans.sort(key=lambda s: s.start)
        self.spans = spans
        self.starts = [s.start for s in spans]

    @staticmethod
    def _blocks(span: _Span, lo: int, hi: int) -> list[int]:
        """Block indexes of `span` that the bytes [lo, hi) touch."""
        a = bisect.bisect_right(span.cum, lo - span.block_start) - 1
        b = bisect.bisect_right(span.cum, hi - 1 - span.block_start) - 1
        last = len(span.cum) - 2
        return list(range(max(0, a), min(b, last) + 1))

    def lookup(self, offset: int, size: int = 1) -> list[dict]:
        if size < 1:
            raise ToolError("a lookup needs a size of at least 1, got %d" % size)
        end = offset + size
        out: list[dict] = []
        i = max(0, bisect.bisect_right(self.starts, offset) - 1)
        cursor = offset
        while i < len(self.spans) and self.spans[i].start < end:
            s = self.spans[i]
            i += 1
            if s.end <= offset:
                continue
            lo, hi = max(offset, s.start), min(end, s.end)
            if lo > cursor:
                out.append({"name": None, "kind": "unknown", "rel_offset": cursor, "blocks": []})
            blocks = self._blocks(s, lo, hi) if s.kind == "bundle" else []
            out.append({"name": s.name, "kind": s.kind, "rel_offset": lo - s.base, "blocks": blocks})
            cursor = max(cursor, hi)
        if cursor < end:
            out.append({"name": None, "kind": "unknown", "rel_offset": cursor, "blocks": []})
        return out


# --------------------------------------------------------------------------
# Diff and copy

def _diff_key(rec: dict):
    if rec["kind"] in ("file", "dir"):
        return (rec["kind"], rec["name"])
    return (rec["kind"], rec["offset"])


def diff_trees(a: dict, b: dict) -> list[str]:
    """One line per difference; empty when the trees are identical."""
    lines = []
    for field in ("version", "root", "free_head", "ggpk_size"):
        if a.get(field) != b.get(field):
            lines.append("header %s: %s -> %s" % (field, a.get(field), b.get(field)))
    ra = {_diff_key(r): r for r in a["records"]}
    rb = {_diff_key(r): r for r in b["records"]}

    def label(key, rec):
        return rec.get("name") or "%s record at %d" % (key[0], rec["offset"])

    for key in sorted(ra.keys() - rb.keys(), key=lambda k: ra[k]["offset"]):
        lines.append("removed %s %s" % (key[0], label(key, ra[key])))
    for key in sorted(rb.keys() - ra.keys(), key=lambda k: rb[k]["offset"]):
        lines.append("added %s %s" % (key[0], label(key, rb[key])))
    for key in sorted(ra.keys() & rb.keys(), key=lambda k: ra[k]["offset"]):
        x, y = ra[key], rb[key]
        if x == y:
            continue
        if x["offset"] != y["offset"]:
            lines.append("moved %s %s: offset %d -> %d" % (key[0], label(key, x), x["offset"], y["offset"]))
        else:
            changed = sorted(k for k in x.keys() | y.keys() if x.get(k) != y.get(k))
            lines.append("changed %s %s: %s" % (key[0], label(key, x), ", ".join(changed)))
    return lines


def find_file(tree: dict, name: str) -> dict:
    files = [r for r in tree["records"] if r["kind"] == "file"]
    exact = [r for r in files if r["name"] == name]
    if exact:
        return exact[0]
    folded = [r for r in files if r["name"].lower() == name.lower()]
    if len(folded) == 1:
        return folded[0]
    if folded:
        raise ToolError("%d files match %s when case is ignored; give the exact name" % (len(folded), name))
    raise ToolError("no file %s in the tree" % name)


def copy_record(ggpk: Path, tree: dict, name: str, out: Path) -> dict:
    """Copy one file's data out of the GGPK, refusing a stale tree."""
    out = Path(out)
    common.refuse_inside_git(out, "copied GGPK record")
    rec = find_file(tree, name)
    ggpk = Path(ggpk)
    if not ggpk.is_file():
        raise ToolError("no GGPK at %s" % ggpk)
    version = tree["version"]
    cw = 4 if version == 4 else 2
    with open(ggpk, "rb") as f:
        reader = _Reader(f, ggpk.stat().st_size)
        try:
            length, tag, buf = reader.head(rec["offset"])
        except ToolError as err:
            raise ToolError("the tree is stale: no record of %s at offset %d (%s); walk the GGPK again"
                            % (rec["name"], rec["offset"], err))
        if tag != b"FILE" or length != rec["length"]:
            raise ToolError("the tree is stale: offset %d holds a %r record of %d bytes, not %s; "
                            "walk the GGPK again" % (rec["offset"], tag, length, rec["name"]))
        name_len, = struct.unpack_from("<I", buf, 8)
        header = 12 + 32 + name_len * cw
        buf = reader.span(rec["offset"], buf, header)
        found = _decode_name(buf[44:header], version)
        if not rec["name"].endswith("/" + found) or rec["offset"] + header != rec["data"]:
            raise ToolError("the tree is stale: offset %d holds the file %r, not %s; walk the GGPK again"
                            % (rec["offset"], found, rec["name"]))
        out.parent.mkdir(parents=True, exist_ok=True)
        remaining, pos = rec["size"], rec["data"]
        with open(out, "wb") as dst:
            while remaining:
                n = min(COPY_CHUNK, remaining)
                dst.write(reader.read_at(pos, n))
                pos += n
                remaining -= n
    return rec


# --------------------------------------------------------------------------
# Selfcheck

def _check_bytes(index: TreeIndex, rec: dict, failures: list[str]) -> None:
    for offset in (rec["data"], rec["data"] + rec["size"] - 1):
        hits = index.lookup(offset)
        if len(hits) != 1 or hits[0]["name"] != rec["name"] or hits[0]["kind"] not in (
                "file", "bundle", "bundle-header"):
            failures.append("lookup(%d) gave %r, not %s" % (offset, hits, rec["name"]))


def _check_blocks(index: TreeIndex, rec: dict, failures: list[str]) -> None:
    bundle = rec["bundle"]
    blocks = bundle["blocks"]
    start = rec["data"] + 12 + bundle["head"]
    end = start + sum(blocks)
    for offset, want in ((start, 0), (start + blocks[0] - 1, 0),
                         (end - blocks[-1], len(blocks) - 1), (end - 1, len(blocks) - 1)):
        hits = index.lookup(offset)
        if len(hits) != 1 or hits[0]["kind"] != "bundle" or hits[0]["blocks"] != [want] \
                or hits[0]["name"] != rec["name"]:
            failures.append("lookup(%d) gave %r, not block %d of %s" % (offset, hits, want, rec["name"]))


def selfcheck(ggpk: Path) -> tuple[list[str], list[str]]:
    """(report lines, failures): an empty failure list is a pass."""
    tree = walk_ggpk(ggpk)
    started = time.perf_counter()
    index = TreeIndex(tree)
    index_seconds = time.perf_counter() - started
    files = [r for r in tree["records"] if r["kind"] == "file"]
    by_lower = {r["name"].lower(): r for r in files}
    failures: list[str] = []
    report: list[str] = []
    c = tree["counts"]
    report.append("version %d, %d bytes; %d files, %d dirs, %d free records, %d bundles "
                  "(%d header errors, %d size warnings); walk %.1f s, index %.1f s"
                  % (tree["version"], tree["ggpk_size"], c["files"], c["dirs"], c["free"],
                     c["bundles"], c["bundle_errors"], c["bundle_warnings"], tree["walk_seconds"],
                     index_seconds))

    bank = by_lower.get(SELFCHECK_BANK.lower())
    if bank is None:
        failures.append("no %s" % SELFCHECK_BANK)
    else:
        report.append("%s: %d bytes at %d" % (bank["name"], bank["size"], bank["data"]))
        _check_bytes(index, bank, failures)

    idx = by_lower.get(SELFCHECK_INDEX.lower())
    if idx is None:
        failures.append("no %s" % SELFCHECK_INDEX)
    elif "bundle" not in idx:
        failures.append("%s has no readable bundle header: %s" % (idx["name"], idx.get("bundle_error")))
    else:
        report.append("%s: %d bytes, %d blocks" % (idx["name"], idx["size"], len(idx["bundle"]["blocks"])))
        _check_bytes(index, idx, failures)

    prefix = SELFCHECK_DIALOGUE_PREFIX.lower()
    dialogue = [r for r in files if r["name"].lower().startswith(prefix)]
    usable = [r for r in dialogue if r.get("bundle") and r["bundle"]["blocks"]]
    if not usable:
        failures.append("no %s* bundle with blocks (%d named)" % (SELFCHECK_DIALOGUE_PREFIX, len(dialogue)))
    else:
        d = usable[0]
        report.append("%d dialogue bundles under %s*; %s: %d bytes, %d blocks"
                      % (len(dialogue), SELFCHECK_DIALOGUE_PREFIX, d["name"], d["size"],
                         len(d["bundle"]["blocks"])))
        _check_bytes(index, d, failures)
        _check_blocks(index, d, failures)
    return report, failures


# --------------------------------------------------------------------------
# Command line

def _parse_int(text: str) -> int:
    return int(text, 0)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="ggpk", description="Walk the GGPK directory, look offsets up, diff, copy one record.")
    sub = parser.add_subparsers(dest="command", required=True)
    w = sub.add_parser("walk", help="write the whole directory tree to a tree file")
    w.add_argument("--ggpk", type=Path, default=common.DEFAULT_GGPK)
    w.add_argument("--out", type=Path, required=True)
    lk = sub.add_parser("lookup", help="map an offset (and size) to records")
    lk.add_argument("--tree", type=Path, required=True)
    lk.add_argument("offset", type=_parse_int)
    lk.add_argument("--size", type=_parse_int, default=1)
    d = sub.add_parser("diff", help="compare two tree files; exit 1 when they differ")
    d.add_argument("a", type=Path)
    d.add_argument("b", type=Path)
    c = sub.add_parser("copy", help="copy one file's data out of the GGPK")
    c.add_argument("--ggpk", type=Path, default=common.DEFAULT_GGPK)
    c.add_argument("--tree", type=Path, required=True)
    c.add_argument("--name", required=True)
    c.add_argument("--out", type=Path, required=True)
    s = sub.add_parser("selfcheck", help="walk in memory and check the records the session needs")
    s.add_argument("--ggpk", type=Path, default=common.DEFAULT_GGPK)
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    try:
        if args.command == "walk":
            common.refuse_inside_git(args.out, "GGPK tree")
            tree = walk_ggpk(args.ggpk)
            write_tree(args.out, tree)
            c = tree["counts"]
            print("ggpk walk: %d files, %d dirs, %d free records, %d bundles (%d bundle header errors) "
                  "in %.1f s -> %s" % (c["files"], c["dirs"], c["free"], c["bundles"],
                                      c["bundle_errors"], tree["walk_seconds"], args.out))
        elif args.command == "lookup":
            for entry in TreeIndex(load_tree(args.tree)).lookup(args.offset, args.size):
                print(json.dumps(entry, ensure_ascii=False))
        elif args.command == "diff":
            lines = diff_trees(load_tree(args.a), load_tree(args.b))
            for line in lines:
                print("ggpk diff: %s" % line)
            if lines:
                print("ggpk diff: %d differences" % len(lines))
                return 1
            print("ggpk diff: identical")
        elif args.command == "copy":
            rec = copy_record(args.ggpk, load_tree(args.tree), args.name, args.out)
            print("ggpk copy: %s, %d bytes -> %s" % (rec["name"], rec["size"], args.out))
        else:
            report, failures = selfcheck(args.ggpk)
            for line in report:
                print("ggpk selfcheck: %s" % line)
            for line in failures:
                print("ggpk selfcheck: FAIL %s" % line)
            if failures:
                return 1
            print("ggpk selfcheck: ok")
    except ToolError as err:
        print("ggpk: error: %s" % err, file=sys.stderr)
        return 1
    except (OSError, ValueError, KeyError, struct.error, json.JSONDecodeError) as err:
        print("ggpk: error: %s" % err, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
