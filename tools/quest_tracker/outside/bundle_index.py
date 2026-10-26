"""The decoded bundle index: its records, and which files a 256 KiB block of a bundle holds.

Developer-only. Usage:

    py -3 -m tools.quest_tracker.outside.bundle_index read --bin B [--paths P] --out F
    py -3 -m tools.quest_tracker.outside.bundle_index annotate [--live L] [--index F] [--paths P] [--out O]
    py -3 -m tools.quest_tracker.outside.bundle_index files [--live L] [--index F] [--paths P] --bundle NAME [--block k ...]
    py -3 -m tools.quest_tracker.outside.bundle_index extract-plan [--live L] [--index F] [--paths P] --prefix PATH
    py -3 -m tools.quest_tracker.outside.bundle_index decode [--live L] [--raw F] [--out F] [--oodle DLL]
    py -3 -m tools.quest_tracker.outside.bundle_index extract [--live L] [--index F] [--tree F] [--ggpk G]
                                                              (--path P ... | --prefix PATH) [--out-dir D] [--oodle DLL]

The game keeps most of its files inside bundles under `/Bundles2`, and
`/Bundles2/_.index.bin` says which bundle holds which file. Both are stored
compressed, block by block (`ggpk.py` describes a bundle's header). Every
command but `decode` and `extract` reads an index already decoded and
decompresses nothing. Those two decompress blocks through an Oodle decoder
the owner approved as a download (the workorder's "Oodle decoder download",
approved 2026-10-26): the `oo2core.dll` that LibGGPK3's `win-x64.zip` ships,
unpacked into its own folder under `L\\tools\\`, called through `ctypes`
(`OodleLZ_Decompress`, one block at a time). Nothing here calls any code of
the game, its own Oodle included. Without a decoder under `L\\tools\\` (or
`--oodle`), `decode` and `extract` exit 2 with `no decoder`.

`decode` turns the raw `_.index.bin` copy (`L\\ggpk\\index.raw`, from `ggpk
copy`) into the decoded index: it decompresses the index bundle, then the
path section bundle nested at its end, keeps both decoded binaries beside the
output (`index.dec.bin`, `index-paths.dec.bin`, so `read` can run again on
them if the layout below turns out wrong), and writes `read`'s JSON
(`L\\ggpk\\index.json`). `extract` copies one indexed path's bytes (or every
path under a prefix) to `L\\ref\\`, under its file name: it finds the
path's bundle in a GGPK tree (`tree-after.json`, else `tree-before.json`),
checks that the tree still matches `Content.ggpk`, and decompresses only the
blocks the file spans. Read the GGPK only while the game is closed.

The decoded index, in the public layout community bundle readers document
(the field order is UNVERIFIED until a real decode checks it; the tests pin
this reader's assumption), all little-endian:

- a bundle count (i32), then per bundle its name length (i32), its name
  (UTF-8, relative to `/Bundles2`, without `.bundle.bin`) and its
  uncompressed size (u32);
- a file count (i32), then per file its path hash (u64), its bundle's index
  (i32), and its offset and size inside that bundle's uncompressed bytes
  (u32 each);
- a directory count (i32), then per directory its path hash (u64) and the
  offset, size and recursive size (u32 each) of its paths in the path
  section;
- the path section, itself a compressed bundle, to the end.

The decoded path section is passed separately (`--paths`). Each directory's
slice of it is a run of u32 indexes, each but the zeros followed by a
NUL-terminated string. A zero toggles the base phase (entering it clears the
list of bases). Any other index n refers to base n-1 when that base exists,
which is prefixed to the string; in the base phase the result is a new base,
otherwise it is a full path. A file record carries only its path's hash, so
the paths are joined to the files by hashing them. Two hash functions are
known from public readers: MurmurHash64A (seed 0x1337B33F) of the lowercase
path, and FNV-1a 64 of the lowercase path followed by `++`. The reader tries
both on a sample and keeps the one that matches more file hashes; when
neither matches any, the paths stay unknown (`"hash": "none"`), never
guessed.

`read` writes the records as a JSON index (`L\\ggpk\\index.json`, the live
folder's decoded index); every other command loads either that JSON or the
binary directly. `annotate` adds `files` to each read in `i1\\mapped.jsonl`:
for every `bundle` entry `ggpk.TreeIndex.lookup` gave the read, the files
whose bytes overlap the entry's blocks, each with the blocks it was found in.
Block k of a bundle holds its uncompressed bytes from k x 262,144. A bundle
the index does not list (the index bundle itself, for one) goes in the
read's `files_unmatched`. `files` lists the files of one bundle's blocks, and
`extract-plan` the bundles, merged byte ranges and blocks that hold every
path under a prefix (for `metadata/.../carrioncrone/`), which is what an
extraction would have to decompress.

A GGPK bundle name is matched to an index bundle by its path under
`/Bundles2` without `.bundle.bin`, case-insensitively; whether the index
lists a streaming bundle with its `Streaming/` folder is not established, so
a name not found with it is tried without it.

Every command that writes refuses a path inside a git checkout: a decoded
index and an extracted file are the game's data and stay in the live folder.
"""
from __future__ import annotations

import argparse
import bisect
import json
import os
import struct
import sys
from pathlib import Path

from tools.quest_tracker.outside import common, ggpk
from tools.quest_tracker.outside.common import ToolError

INDEX_FORMAT = "quest-tracker-bundle-index/1"
# Block k of a bundle holds its uncompressed bytes from k x GRANULARITY (the
# value `ggpk.py` reads from every bundle header).
GRANULARITY = 262144
FILE_FIELDS = ("path", "hash", "bundle", "offset", "size")

MURMUR_SEED = 0x1337B33F
_M64 = (1 << 64) - 1
_MURMUR_M = 0xC6A4A7935BD1E995
_FNV_OFFSET = 0xCBF29CE484222325
_FNV_PRIME = 0x100000001B3
# How many paths the hash choice is tried on before every path is hashed.
HASH_SAMPLE = 2000


# --------------------------------------------------------------------------
# Path hashes

def murmur64a(data: bytes, seed: int = MURMUR_SEED) -> int:
    """MurmurHash64A of `data`."""
    r = 47
    h = (seed ^ (len(data) * _MURMUR_M)) & _M64
    whole = len(data) // 8
    for i in range(whole):
        k = int.from_bytes(data[8 * i:8 * i + 8], "little")
        k = (k * _MURMUR_M) & _M64
        k ^= k >> r
        k = (k * _MURMUR_M) & _M64
        h ^= k
        h = (h * _MURMUR_M) & _M64
    tail = data[8 * whole:]
    if tail:
        h ^= int.from_bytes(tail, "little")
        h = (h * _MURMUR_M) & _M64
    h ^= h >> r
    h = (h * _MURMUR_M) & _M64
    h ^= h >> r
    return h


def fnv1a64(data: bytes) -> int:
    h = _FNV_OFFSET
    for b in data:
        h ^= b
        h = (h * _FNV_PRIME) & _M64
    return h


HASHES = {
    "murmur64a": lambda path: murmur64a(path.lower().encode("utf-8")),
    "fnv1a64": lambda path: fnv1a64((path.lower() + "++").encode("utf-8")),
}


# --------------------------------------------------------------------------
# Reading the decoded index

class _Cursor:
    def __init__(self, data: bytes, what: str):
        self.data, self.pos, self.what = data, 0, what

    def take(self, fmt: str, label: str):
        n = struct.calcsize(fmt)
        if self.pos + n > len(self.data):
            raise ToolError("the %s ends at byte %d, inside %s (%d bytes needed)"
                            % (self.what, len(self.data), label, n))
        out = struct.unpack_from(fmt, self.data, self.pos)
        self.pos += n
        return out

    def raw(self, n: int, label: str) -> bytes:
        if n < 0 or self.pos + n > len(self.data):
            raise ToolError("the %s ends at byte %d, inside %s (%d bytes needed)"
                            % (self.what, len(self.data), label, n))
        out = self.data[self.pos:self.pos + n]
        self.pos += n
        return out

    def count(self, label: str) -> int:
        (n,) = self.take("<i", label + " count")
        if n < 0:
            raise ToolError("the %s gives a negative %s count (%d)" % (self.what, label, n))
        return n


def read_paths(section: bytes, directories) -> list[str]:
    """Every full path the path section lists, directory by directory."""
    out: list[str] = []
    for d in directories:
        start, size = d["offset"], d["size"]
        if start + size > len(section):
            raise ToolError("a directory's paths run to byte %d, past the path section's %d bytes"
                            % (start + size, len(section)))
        end = start + size
        pos = start
        bases: list[str] = []
        in_base = False
        while pos < end:
            if pos + 4 > end:
                raise ToolError("a directory's paths end inside an index at byte %d" % pos)
            (n,) = struct.unpack_from("<I", section, pos)
            pos += 4
            if n == 0:
                in_base = not in_base
                if in_base:
                    bases = []
                continue
            nul = section.find(b"\0", pos, end)
            if nul < 0:
                raise ToolError("a path at byte %d of the path section has no terminator" % pos)
            text = section[pos:nul].decode("utf-8", errors="replace")
            pos = nul + 1
            if n - 1 < len(bases):
                text = bases[n - 1] + text
            if in_base:
                bases.append(text)
            else:
                out.append(text)
    return out


def _choose_hash(paths: list[str], hashes: set[int]) -> tuple[str, dict]:
    """The hash function matching most file hashes on a sample, or "none"."""
    sample = paths[:HASH_SAMPLE]
    best, best_hits = "none", 0
    for name, fn in HASHES.items():
        hits = sum(1 for p in sample if fn(p) in hashes)
        if hits > best_hits:
            best, best_hits = name, hits
    if best == "none":
        return best, {}
    fn = HASHES[best]
    return best, {fn(p): p for p in paths}


def parse_index_bin(data: bytes, paths: bytes | None = None) -> dict:
    """The decoded index's records, with each file's path when `paths` (the decoded path section) is given."""
    cur = _Cursor(data, "decoded index")
    bundles = []
    for i in range(cur.count("bundle")):
        (n,) = cur.take("<i", "bundle %d's name length" % i)
        name = cur.raw(n, "bundle %d's name" % i).decode("utf-8", errors="replace")
        (size,) = cur.take("<I", "bundle %d's size" % i)
        bundles.append({"name": name, "uncompressed": size})
    files = []
    for i in range(cur.count("file")):
        h, bundle, offset, size = cur.take("<QiII", "file %d" % i)
        if not 0 <= bundle < len(bundles):
            raise ToolError("file %d names bundle %d, but the index lists %d bundles"
                            % (i, bundle, len(bundles)))
        files.append({"path": None, "hash": h, "bundle": bundle, "offset": offset, "size": size})
    directories = []
    for i in range(cur.count("directory")):
        h, offset, size, recursive = cur.take("<QIII", "directory %d" % i)
        directories.append({"hash": "%016x" % h, "offset": offset, "size": size, "recursive": recursive})
    tail = {"offset": cur.pos, "size": len(data) - cur.pos}

    listed: list[str] = []
    hash_name = "none"
    if paths is not None:
        listed = read_paths(paths, directories)
        hash_name, by_hash = _choose_hash(listed, {f["hash"] for f in files})
        for f in files:
            f["path"] = by_hash.get(f["hash"])
    for f in files:
        f["hash"] = "%016x" % f["hash"]
    return {
        "format": INDEX_FORMAT,
        "hash": hash_name,
        "bundles": bundles,
        "files": files,
        "directories": directories,
        "path_bundle": tail,
        "counts": {
            "bundles": len(bundles),
            "files": len(files),
            "directories": len(directories),
            "paths_listed": len(listed),
            "paths_resolved": sum(1 for f in files if f["path"] is not None),
        },
    }


def index_to_json(index: dict) -> dict:
    """The JSON form `read` writes: files as rows of FILE_FIELDS, to keep a full index small."""
    out = {k: v for k, v in index.items() if k != "files"}
    out["file_fields"] = list(FILE_FIELDS)
    out["files"] = [[f[k] for k in FILE_FIELDS] for f in index["files"]]
    return out


def index_from_json(obj: dict) -> dict:
    if obj.get("format") != INDEX_FORMAT:
        raise ToolError("not a decoded index written by `bundle_index read` (format %r, expected %r)"
                        % (obj.get("format"), INDEX_FORMAT))
    fields = obj.get("file_fields") or list(FILE_FIELDS)
    out = dict(obj)
    out["files"] = [dict(zip(fields, row)) for row in obj["files"]]
    return out


def load_index(path: Path, paths: Path | None = None) -> "BundleIndex":
    """A `BundleIndex` from `read`'s JSON, or from the decoded binary (with its decoded path section)."""
    path = Path(path)
    if not path.is_file():
        raise ToolError("no decoded index at %s: decode `/Bundles2/_.index.bin` and run "
                        "`bundle_index read` first" % path)
    data = path.read_bytes()
    if data.lstrip()[:1] == b"{":
        if paths is not None:
            raise ToolError("--paths goes with a binary index; %s is already a JSON index" % path)
        return BundleIndex(index_from_json(json.loads(data.decode("utf-8"))))
    section = Path(paths).read_bytes() if paths is not None else None
    return BundleIndex(parse_index_bin(data, section))


# --------------------------------------------------------------------------
# Blocks to files

def bundle_key(name: str) -> str:
    """A bundle name as `/Bundles2/<key>.bundle.bin` and the index both reduce to."""
    key = name.replace("\\", "/").lower().lstrip("/")
    if key.startswith("bundles2/"):
        key = key[len("bundles2/"):]
    if key.endswith(".bundle.bin"):
        key = key[:-len(".bundle.bin")]
    return key


def path_key(path: str) -> str:
    return path.replace("\\", "/").lower().lstrip("/")


def _merge(ranges: list[list[int]]) -> list[list[int]]:
    out: list[list[int]] = []
    for lo, hi in sorted(ranges):
        if out and lo <= out[-1][1]:
            out[-1][1] = max(out[-1][1], hi)
        else:
            out.append([lo, hi])
    return out


class BundleIndex:
    """Lookups over a decoded index's bundles and files."""

    def __init__(self, index: dict, granularity: int = GRANULARITY):
        self.index = index
        self.granularity = granularity
        self.bundles = index["bundles"]
        self.files = index["files"]
        self._by_key = {}
        for i, b in enumerate(self.bundles):
            self._by_key.setdefault(bundle_key(b["name"]), i)
        # Per bundle: its files by offset, their offsets, and the furthest end
        # reached so far, so a block's files are found without a full scan.
        self._per_bundle: dict[int, tuple[list[dict], list[int], list[int]]] = {}
        grouped: dict[int, list[dict]] = {}
        for f in self.files:
            grouped.setdefault(f["bundle"], []).append(f)
        for b, fs in grouped.items():
            fs.sort(key=lambda f: (f["offset"], f["size"]))
            ends, furthest = [], 0
            for f in fs:
                furthest = max(furthest, f["offset"] + max(1, f["size"]))
                ends.append(furthest)
            self._per_bundle[b] = (fs, [f["offset"] for f in fs], ends)

    def bundle_for(self, ggpk_name: str) -> int | None:
        """The index's bundle for a GGPK file name, or None when the index does not list it."""
        key = bundle_key(ggpk_name)
        if key in self._by_key:
            return self._by_key[key]
        if key.startswith("streaming/"):
            return self._by_key.get(key[len("streaming/"):])
        return None

    def ggpk_name(self, bundle: int) -> str:
        return "/Bundles2/%s.bundle.bin" % self.bundles[bundle]["name"]

    def _block_files(self, bundle: int, block: int) -> list[dict]:
        if bundle not in self._per_bundle:
            return []
        lo = block * self.granularity
        hi = min((block + 1) * self.granularity, self.bundles[bundle]["uncompressed"])
        if lo >= hi:
            return []
        fs, offsets, ends = self._per_bundle[bundle]
        j = bisect.bisect_left(offsets, hi)
        out = []
        i = j - 1
        while i >= 0 and ends[i] > lo:
            f = fs[i]
            # A file of size 0 sits at its offset; any other covers [offset, offset + size).
            if f["offset"] + max(1, f["size"]) > lo:
                out.append(f)
            i -= 1
        out.reverse()
        return out

    def files_in_blocks(self, bundle: int, blocks) -> list[dict]:
        """The files overlapping `bundle`'s `blocks`, each with the blocks it was found in."""
        found: dict[int, dict] = {}
        for block in sorted(set(int(b) for b in blocks)):
            for f in self._block_files(bundle, block):
                hit = found.setdefault(id(f), {
                    "path": f["path"], "bundle": self.bundles[bundle]["name"],
                    "offset": f["offset"], "size": f["size"], "blocks": []})
                hit["blocks"].append(block)
        return sorted(found.values(), key=lambda h: (h["offset"], h["size"]))

    def extract_plan(self, prefix: str) -> list[dict]:
        """Per bundle, the merged byte ranges and blocks holding every path under `prefix`."""
        if not any(f["path"] is not None for f in self.files):
            raise ToolError("this index has no paths: decode its path section and pass it as --paths")
        want = path_key(prefix)
        per: dict[int, list[dict]] = {}
        for f in self.files:
            if f["path"] is not None and path_key(f["path"]).startswith(want):
                per.setdefault(f["bundle"], []).append(f)
        out = []
        for b in sorted(per):
            fs = sorted(per[b], key=lambda f: (f["offset"], f["size"]))
            ranges = _merge([[f["offset"], f["offset"] + f["size"]] for f in fs])
            blocks = set()
            for lo, hi in ranges:
                last = max(lo, hi - 1)
                blocks.update(range(lo // self.granularity, last // self.granularity + 1))
            out.append({
                "bundle": self.bundles[b]["name"],
                "ggpk_name": self.ggpk_name(b),
                "uncompressed": self.bundles[b]["uncompressed"],
                "ranges": ranges,
                "blocks": sorted(blocks),
                "files": [{"path": f["path"], "offset": f["offset"], "size": f["size"]} for f in fs],
            })
        return out


def annotate_read(rec: dict, index: BundleIndex) -> dict:
    """`rec` with `files` (and `files_unmatched` when a bundle is not in the index) set afresh."""
    out = {k: v for k, v in rec.items() if k not in ("files", "files_unmatched")}
    files: list[dict] = []
    unmatched: list[str] = []
    for entry in rec.get("records") or []:
        if entry.get("kind") != "bundle" or not entry.get("blocks"):
            continue
        bundle = index.bundle_for(entry.get("name") or "")
        if bundle is None:
            if entry.get("name") not in unmatched:
                unmatched.append(entry.get("name"))
            continue
        files.extend(index.files_in_blocks(bundle, entry["blocks"]))
    out["files"] = files
    if unmatched:
        out["files_unmatched"] = unmatched
    return out


def annotate(live: Path, index_path: Path | None = None, paths: Path | None = None,
             src: Path | None = None, out: Path | None = None) -> dict:
    """Rewrite `i1\\mapped.jsonl` (or write `out`) with each read's files."""
    live = Path(live)
    src = Path(src) if src else live / "i1" / "mapped.jsonl"
    out = Path(out) if out else src
    common.refuse_inside_git(out, "annotated read list")
    if not src.is_file():
        raise ToolError("no mapped read list at %s: run `fileio map` first" % src)
    index_path = Path(index_path) if index_path else live / "ggpk" / "index.json"
    index = load_index(index_path, paths)
    recs = [annotate_read(r, index) for r in common.read_jsonl(src)]
    tmp = out.with_name(out.name + ".tmp")
    with open(tmp, "w", encoding="utf-8", newline="\n") as f:
        for r in recs:
            f.write(json.dumps(r, ensure_ascii=False) + "\n")
    os.replace(tmp, out)
    unmatched = sorted({n for r in recs for n in r.get("files_unmatched", [])})
    return {
        "reads": len(recs),
        "with_files": sum(1 for r in recs if r["files"]),
        "files": len({(f["bundle"], f["offset"], f["path"]) for r in recs for f in r["files"]}),
        "unmatched": unmatched,
        "index": index_path,
        "out": out,
    }


# --------------------------------------------------------------------------
# Decoding, through the approved Oodle decoder

OODLE_DLL = "oo2core.dll"
# OodleLZ_Decompress's trailing arguments: fuzz-safe on, no CRC check, no
# logging, and a single-threaded decode of the whole block.
_FUZZ_SAFE_YES, _CHECK_CRC_NO, _VERBOSITY_NONE, _DECODE_UNTHREADED = 1, 0, 0, 3
# Spare bytes past a block's raw length, as a margin for the decoder's writes.
_DECODE_SLACK = 64


class NoDecoder(ToolError):
    """No approved Oodle decoder is present: `decode` and `extract` exit 2."""


def find_oodle(live: Path, explicit: Path | None = None) -> Path:
    """The decoder DLL: `explicit`, or the one `oo2core.dll` under `L\\tools\\`."""
    if explicit is not None:
        explicit = Path(explicit)
        if not explicit.is_file():
            raise NoDecoder("no Oodle decoder at %s" % explicit)
        return explicit
    tools = Path(live) / "tools"
    found = sorted(tools.rglob(OODLE_DLL)) if tools.is_dir() else []
    if not found:
        raise NoDecoder("no %s under %s: unpack the approved LibGGPK3 win-x64.zip into its own "
                        "folder there, or pass --oodle" % (OODLE_DLL, tools))
    if len(found) > 1:
        raise ToolError("%d copies of %s under %s (%s): pass --oodle to pick one"
                        % (len(found), OODLE_DLL, tools, ", ".join(str(p) for p in found)))
    return found[0]


class OodleDecoder:
    """`decompress(comp, raw_len)` through a downloaded `oo2core.dll` (never the game's)."""

    def __init__(self, dll: Path, loader=None):
        import ctypes
        self._ctypes = ctypes
        self.path = Path(dll).resolve()
        if loader is None:
            if sys.platform != "win32":
                raise NoDecoder("%s needs Windows" % OODLE_DLL)
            loader = ctypes.WinDLL
        try:
            lib = loader(str(self.path))
            fn = lib.OodleLZ_Decompress
        except (OSError, AttributeError) as err:
            raise NoDecoder("could not load OodleLZ_Decompress from %s: %s" % (self.path, err))
        c = ctypes
        fn.restype = c.c_ssize_t
        fn.argtypes = [c.c_void_p, c.c_ssize_t, c.c_void_p, c.c_ssize_t, c.c_int, c.c_int, c.c_int,
                       c.c_void_p, c.c_ssize_t, c.c_void_p, c.c_void_p, c.c_void_p, c.c_ssize_t, c.c_int]
        self._fn = fn

    def __call__(self, comp: bytes, raw_len: int) -> bytes:
        c = self._ctypes
        src = c.create_string_buffer(bytes(comp), len(comp))
        dst = c.create_string_buffer(raw_len + _DECODE_SLACK)
        got = self._fn(src, len(comp), dst, raw_len, _FUZZ_SAFE_YES, _CHECK_CRC_NO, _VERBOSITY_NONE,
                       None, 0, None, None, None, 0, _DECODE_UNTHREADED)
        if got != raw_len:
            raise ToolError("the Oodle decoder returned %d bytes for a block of %d (%d compressed)"
                            % (got, raw_len, len(comp)))
        return dst.raw[:raw_len]


def parse_bundle_head(buf: bytes, what: str) -> dict:
    """A bundle's header from its first bytes: the head size, sizes and per-block compressed sizes."""
    fixed = ggpk.BUNDLE_HEAD_FIXED
    if len(buf) < 12 + fixed:
        raise ToolError("the %s is %d bytes, too short for a bundle header" % (what, len(buf)))
    _size, payload, head = struct.unpack_from("<III", buf, 0)
    _encode, _unk, uncompressed, _payload2, count, granularity = struct.unpack_from("<IIQQII", buf, 12)
    if head != fixed + 4 * count or 12 + head > len(buf):
        raise ToolError("the %s's head (%d bytes, %d blocks) does not fit its %d bytes"
                        % (what, head, count, len(buf)))
    if granularity <= 0 or count != -(-uncompressed // granularity):
        raise ToolError("the %s holds %d blocks of %d bytes for %d uncompressed bytes"
                        % (what, count, granularity, uncompressed))
    blocks = list(struct.unpack_from("<%dI" % count, buf, 12 + fixed))
    cum = [0]
    for b in blocks:
        cum.append(cum[-1] + b)
    return {"head": head, "payload": payload, "uncompressed": uncompressed, "granularity": granularity,
            "blocks": blocks, "cum": cum, "data_start": 12 + head}


def _raw_len(head: dict, block: int) -> int:
    return min(head["granularity"], head["uncompressed"] - block * head["granularity"])


def decode_bundle(data: bytes, decompress, what: str = "bundle") -> bytes:
    """A whole bundle's uncompressed bytes."""
    head = parse_bundle_head(data, what)
    if head["data_start"] + head["cum"][-1] > len(data):
        raise ToolError("the %s's blocks run to byte %d, past its %d bytes"
                        % (what, head["data_start"] + head["cum"][-1], len(data)))
    parts = []
    for k, size in enumerate(head["blocks"]):
        start = head["data_start"] + head["cum"][k]
        parts.append(decompress(data[start:start + size], _raw_len(head, k)))
    out = b"".join(parts)
    if len(out) != head["uncompressed"]:
        raise ToolError("the %s decoded to %d bytes, its header says %d" % (what, len(out), head["uncompressed"]))
    return out


def decode(live: Path, raw: Path | None = None, out: Path | None = None, decompress=None,
           oodle: Path | None = None) -> dict:
    """The raw `_.index.bin` copy to `read`'s JSON, keeping the two decoded binaries beside it."""
    live = Path(live)
    raw = Path(raw) if raw else live / "ggpk" / "index.raw"
    out = Path(out) if out else live / "ggpk" / "index.json"
    dec_bin, dec_paths = out.with_name("index.dec.bin"), out.with_name("index-paths.dec.bin")
    for path, what in ((out, "decoded index"), (dec_bin, "decoded index"), (dec_paths, "decoded path section")):
        common.refuse_inside_git(path, what)
    if not raw.is_file():
        raise ToolError("no raw index at %s: run `ggpk copy --name /Bundles2/_.index.bin` first" % raw)
    if decompress is None:
        decompress = OodleDecoder(find_oodle(live, oodle))
    index_bin = decode_bundle(raw.read_bytes(), decompress, "index bundle")
    out.parent.mkdir(parents=True, exist_ok=True)
    dec_bin.write_bytes(index_bin)
    tail = parse_index_bin(index_bin)["path_bundle"]
    section = decode_bundle(index_bin[tail["offset"]:], decompress, "path section bundle")
    dec_paths.write_bytes(section)
    idx = parse_index_bin(index_bin, section)
    common.write_json(out, index_to_json(idx))
    return {"index": idx, "out": out, "index_bin": dec_bin, "paths_bin": dec_paths}


def _default_tree(live: Path) -> Path:
    after = live / "ggpk" / "tree-after.json"
    return after if after.is_file() else live / "ggpk" / "tree-before.json"


def _select(index: BundleIndex, paths, prefix) -> list[dict]:
    if not any(f["path"] is not None for f in index.files):
        raise ToolError("this index has no paths: decode it with its path section first")
    by_path = {}
    for f in index.files:
        if f["path"] is not None:
            by_path.setdefault(path_key(f["path"]), f)
    chosen: list[dict] = []
    for p in paths or []:
        f = by_path.get(path_key(p))
        if f is None:
            raise ToolError("the index lists no file %s" % p)
        if f not in chosen:
            chosen.append(f)
    if prefix:
        want = path_key(prefix)
        under = [f for k, f in sorted(by_path.items()) if k.startswith(want)]
        if not under:
            raise ToolError("the index lists no file under %s" % prefix)
        chosen.extend(f for f in under if f not in chosen)
    if not chosen:
        raise ToolError("name a --path or a --prefix to extract")
    return chosen


def _check_tree_record(f, rec: dict, ggpk_size: int) -> dict:
    """The bundle's header as read now, after checking the tree still describes this record."""
    if rec["offset"] + rec["length"] > ggpk_size:
        raise ToolError("the tree is stale: %s runs past the GGPK's end; walk the GGPK again" % rec["name"])
    f.seek(rec["offset"])
    length, tag = struct.unpack("<I4s", f.read(8))
    if tag != b"FILE" or length != rec["length"]:
        raise ToolError("the tree is stale: offset %d holds a %r record of %d bytes, not %s; walk the "
                        "GGPK again" % (rec["offset"], tag, length, rec["name"]))
    known = rec.get("bundle") or {}
    f.seek(rec["data"])
    first = f.read(min(rec["size"], 12 + int(known.get("head") or ggpk.BUNDLE_HEAD_FIXED)))
    head = parse_bundle_head(first, rec["name"])
    if head["blocks"] != known.get("blocks") or head["uncompressed"] != known.get("uncompressed"):
        raise ToolError("the tree is stale: %s's bundle header changed; walk the GGPK again" % rec["name"])
    return head


def extract(live: Path, paths=None, prefix: str | None = None, index_path: Path | None = None,
            tree_path: Path | None = None, ggpk_path: Path | None = None, out_dir: Path | None = None,
            decompress=None, oodle: Path | None = None) -> list[dict]:
    """Copy each chosen path's bytes out of its bundle in the GGPK, into `out_dir` (default `L\\ref\\`)."""
    live = Path(live)
    out_dir = Path(out_dir) if out_dir else live / "ref"
    common.refuse_inside_git(out_dir, "extracted file")
    index = load_index(Path(index_path) if index_path else live / "ggpk" / "index.json")
    chosen = _select(index, paths, prefix)
    names: dict[str, str] = {}
    for f in chosen:
        name = f["path"].replace("\\", "/").rsplit("/", 1)[-1]
        if name.lower() in names:
            raise ToolError("%s and %s share the file name %s; extract them one at a time with --out-dir"
                            % (names[name.lower()], f["path"], name))
        names[name.lower()] = f["path"]
    tree = ggpk.load_tree(Path(tree_path) if tree_path else _default_tree(live))
    by_key = {}
    for rec in tree["records"]:
        if rec["kind"] == "file" and rec.get("bundle"):
            by_key.setdefault(bundle_key(rec["name"]), rec)
    ggpk_path = Path(ggpk_path or tree.get("ggpk") or common.DEFAULT_GGPK)
    if not ggpk_path.is_file():
        raise ToolError("no GGPK at %s" % ggpk_path)
    if decompress is None:
        decompress = OodleDecoder(find_oodle(live, oodle))

    out_dir.mkdir(parents=True, exist_ok=True)
    done: list[dict] = []
    cache: dict[tuple[str, int], bytes] = {}
    with open(ggpk_path, "rb") as g:
        size = ggpk_path.stat().st_size
        heads: dict[str, dict] = {}
        for f in chosen:
            bname = index.bundles[f["bundle"]]["name"]
            key = bundle_key(bname)
            rec = by_key.get(key) or by_key.get("streaming/" + key)
            if rec is None:
                raise ToolError("the GGPK tree has no bundle %s (for %s)" % (bname, f["path"]))
            if rec["name"] not in heads:
                heads[rec["name"]] = _check_tree_record(g, rec, size)
            head = heads[rec["name"]]
            lo, hi = f["offset"], f["offset"] + f["size"]
            if hi > head["uncompressed"]:
                raise ToolError("%s runs to byte %d of %s, which holds %d" % (f["path"], hi, bname,
                                                                              head["uncompressed"]))
            gran = head["granularity"]
            first, last = lo // gran, max(lo, hi - 1) // gran
            parts = []
            for k in range(first, last + 1):
                if (rec["name"], k) not in cache:
                    g.seek(rec["data"] + head["data_start"] + head["cum"][k])
                    cache[(rec["name"], k)] = decompress(g.read(head["blocks"][k]), _raw_len(head, k))
                parts.append(cache[(rec["name"], k)])
            data = b"".join(parts)[lo - first * gran:hi - first * gran]
            target = out_dir / f["path"].replace("\\", "/").rsplit("/", 1)[-1]
            common.refuse_inside_git(target, "extracted file")
            target.write_bytes(data)
            done.append({"path": f["path"], "bundle": bname, "ggpk_name": rec["name"], "offset": f["offset"],
                         "size": f["size"], "blocks": list(range(first, last + 1)), "out": target})
    return done


# --------------------------------------------------------------------------
# Command line

def _index_args(p: argparse.ArgumentParser) -> None:
    p.add_argument("--live", type=Path, default=None)
    p.add_argument("--index", type=Path, default=None,
                   help="the decoded index (default L\\ggpk\\index.json), JSON or binary")
    p.add_argument("--paths", type=Path, default=None, help="the decoded path section, with a binary index")


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="bundle_index", description="The decoded bundle index, and the files a bundle block holds.")
    sub = parser.add_subparsers(dest="command", required=True)
    r = sub.add_parser("read", help="read a decoded index's records and write them as JSON")
    r.add_argument("--bin", type=Path, required=True, help="the decoded _.index.bin")
    r.add_argument("--paths", type=Path, default=None, help="its decoded path section")
    r.add_argument("--out", type=Path, required=True)
    a = sub.add_parser("annotate", help="add each mapped read's files to i1\\mapped.jsonl")
    _index_args(a)
    a.add_argument("--src", type=Path, default=None, help="default L\\i1\\mapped.jsonl")
    a.add_argument("--out", type=Path, default=None, help="default: rewrite the source")
    fl = sub.add_parser("files", help="list the files of one bundle's blocks")
    _index_args(fl)
    fl.add_argument("--bundle", required=True, help="the bundle's GGPK name or index name")
    fl.add_argument("--block", type=int, action="append", default=None, help="default: every block")
    e = sub.add_parser("extract-plan", help="list the bundle ranges and blocks holding a path prefix")
    _index_args(e)
    e.add_argument("--prefix", required=True)
    d = sub.add_parser("decode", help="decode the raw _.index.bin copy into the decoded index (needs a decoder)")
    d.add_argument("--live", type=Path, default=None)
    d.add_argument("--raw", type=Path, default=None, help="default L\\ggpk\\index.raw")
    d.add_argument("--out", type=Path, default=None, help="default L\\ggpk\\index.json")
    d.add_argument("--oodle", type=Path, default=None, help="default: the one %s under L\\tools\\" % OODLE_DLL)
    x = sub.add_parser("extract", help="copy indexed paths' bytes out of their bundles (needs a decoder)")
    x.add_argument("--live", type=Path, default=None)
    x.add_argument("--index", type=Path, default=None, help="default L\\ggpk\\index.json")
    x.add_argument("--tree", type=Path, default=None, help="default L\\ggpk\\tree-after.json, else tree-before.json")
    x.add_argument("--ggpk", type=Path, default=None, help="default: the tree's own GGPK path")
    x.add_argument("--path", action="append", default=None, help="an indexed path (repeatable)")
    x.add_argument("--prefix", default=None, help="every indexed path under this prefix")
    x.add_argument("--out-dir", type=Path, default=None, help="default L\\ref\\")
    x.add_argument("--oodle", type=Path, default=None, help="default: the one %s under L\\tools\\" % OODLE_DLL)
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    try:
        if args.command == "read":
            common.refuse_inside_git(args.out, "decoded index")
            section = args.paths.read_bytes() if args.paths is not None else None
            idx = parse_index_bin(args.bin.read_bytes(), section)
            common.write_json(args.out, index_to_json(idx))
            c = idx["counts"]
            print("bundle_index read: %d bundles, %d files, %d directories; %d of %d paths resolved (%s); "
                  "path bundle %d bytes at %d -> %s"
                  % (c["bundles"], c["files"], c["directories"], c["paths_resolved"], c["files"],
                     idx["hash"], idx["path_bundle"]["size"], idx["path_bundle"]["offset"], args.out))
            return 0
        live = args.live if args.live is not None else common.default_live_dir()
        if args.command == "decode":
            res = decode(live, args.raw, args.out, oodle=args.oodle)
            c = res["index"]["counts"]
            print("bundle_index decode: %d bundles, %d files, %d directories; %d of %d paths resolved (%s) "
                  "-> %s (decoded binaries %s, %s)"
                  % (c["bundles"], c["files"], c["directories"], c["paths_resolved"], c["files"],
                     res["index"]["hash"], res["out"], res["index_bin"].name, res["paths_bin"].name))
            return 0
        if args.command == "extract":
            for item in extract(live, args.path, args.prefix, args.index, args.tree, args.ggpk, args.out_dir,
                                oodle=args.oodle):
                print("bundle_index extract: %s (%d bytes, %s blocks %s) -> %s"
                      % (item["path"], item["size"], item["bundle"],
                         ",".join(str(b) for b in item["blocks"]), item["out"]))
            return 0
        index_path = args.index if args.index is not None else live / "ggpk" / "index.json"
        if args.command == "annotate":
            res = annotate(live, index_path, args.paths, args.src, args.out)
            print("bundle_index annotate: %d reads, %d with files, %d distinct files; bundles not in "
                  "the index: %s -> %s"
                  % (res["reads"], res["with_files"], res["files"],
                     ", ".join(res["unmatched"]) or "none", res["out"]))
            return 0
        index = load_index(index_path, args.paths)
        if args.command == "files":
            bundle = index.bundle_for(args.bundle)
            if bundle is None:
                raise ToolError("bundle %s is not in the index" % args.bundle)
            blocks = args.block
            if blocks is None:
                size = index.bundles[bundle]["uncompressed"]
                blocks = range(max(1, -(-size // index.granularity)))
            for f in index.files_in_blocks(bundle, blocks):
                print("%s blocks %s offset %d size %d %s"
                      % (f["bundle"], ",".join(str(b) for b in f["blocks"]), f["offset"], f["size"],
                         f["path"] if f["path"] is not None else "<unknown path>"))
        else:
            plan = index.extract_plan(args.prefix)
            print(json.dumps({"prefix": path_key(args.prefix), "bundles": plan}, indent=2))
    except NoDecoder as err:
        print("bundle_index: no decoder: %s" % err, file=sys.stderr)
        return 2
    except ToolError as err:
        print("bundle_index: error: %s" % err, file=sys.stderr)
        return 1
    except (OSError, ValueError, KeyError, struct.error, json.JSONDecodeError) as err:
        print("bundle_index: error: %s" % err, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
