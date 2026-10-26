"""tools/quest_tracker/outside/ggpk: the GGPK walk, lookup, diff, copy and selfcheck.

Nothing here reads the real `Content.ggpk`. Each test builds a small
synthetic GGPK in a temporary folder outside the repository, in the public
record format (`GGPK`, `PDIR`, `FILE` and `FREE` records), with one bundle of
three blocks, so every byte's owner is known in advance.

The baseline is what a walk must see in any GGPK: every file the directories
reach, with its data offset and size, every directory and every free record.
The targets are the lookups the session's mapper relies on: the first and
last data byte of a file, a record header, a free record, a bundle's header
and a byte in each of its blocks, and a read that spans two records; a diff
that names a moved record; a copy that refuses a destination inside a git
checkout; and a selfcheck that passes on a tree holding the session's records
and names what is missing otherwise.
"""
import contextlib
import hashlib
import io
import json
import struct
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from tools.quest_tracker.outside import common, ggpk  # noqa: E402


# --------------------------------------------------------------------------
# A synthetic GGPK

def _name_bytes(name: str, version: int) -> tuple[int, bytes]:
    enc = "utf-32-le" if version == 4 else "utf-16-le"
    return len(name) + 1, (name + "\0").encode(enc)


class Builder:
    """Lays out records back to back; directories point at their children by offset."""

    def __init__(self, version: int = 3):
        self.version = version
        self.items = []  # (kind, payload) in file order

    def file(self, name: str, data: bytes):
        node = {"kind": "file", "name": name, "data": data}
        self.items.append(node)
        return node

    def dir(self, name: str, children):
        node = {"kind": "dir", "name": name, "children": children}
        self.items.append(node)
        return node

    def free(self, size: int):
        node = {"kind": "free", "size": size}
        self.items.append(node)
        return node

    def _length(self, node) -> int:
        if node["kind"] == "file":
            n, raw = _name_bytes(node["name"], self.version)
            return 12 + 32 + len(raw) + len(node["data"])
        if node["kind"] == "dir":
            n, raw = _name_bytes(node["name"], self.version)
            return 16 + 32 + len(raw) + 12 * len(node["children"])
        return node["size"]

    def build(self, root, path: Path) -> dict:
        offset = 28
        for node in self.items:
            node["offset"] = offset
            node["length"] = self._length(node)
            offset += node["length"]
        frees = [n for n in self.items if n["kind"] == "free"]
        out = bytearray()
        out += struct.pack("<I4sIQQ", 28, b"GGPK", self.version, root["offset"],
                           frees[0]["offset"] if frees else 0)
        for node in self.items:
            assert len(out) == node["offset"]
            if node["kind"] == "file":
                n, raw = _name_bytes(node["name"], self.version)
                out += struct.pack("<I4sI", node["length"], b"FILE", n)
                out += hashlib.sha256(node["data"]).digest() + raw + node["data"]
            elif node["kind"] == "dir":
                n, raw = _name_bytes(node["name"], self.version)
                out += struct.pack("<I4sII", node["length"], b"PDIR", n, len(node["children"]))
                out += b"\0" * 32 + raw
                for child in node["children"]:
                    out += struct.pack("<IQ", 0x1234, child["offset"])
            else:
                nxt = frees[frees.index(node) + 1]["offset"] if node is not frees[-1] else 0
                out += struct.pack("<I4sQ", node["length"], b"FREE", nxt)
                out += b"\0" * (node["length"] - 16)
        path.write_bytes(bytes(out))
        return {n.get("name") or ("free@%d" % n["offset"]): n for n in self.items}


def bundle_bytes(blocks) -> bytes:
    """A bundle file: the three sizes, the head, then the blocks' (fake) compressed bytes."""
    head = struct.pack("<IIQQII4I", 0, 0, len(blocks) * ggpk.GRANULARITY, sum(blocks),
                       len(blocks), ggpk.GRANULARITY, 0, 0, 0, 0)
    head += struct.pack("<%dI" % len(blocks), *blocks)
    body = b"".join(bytes([k + 1]) * size for k, size in enumerate(blocks))
    return struct.pack("<III", len(blocks) * ggpk.GRANULARITY, sum(blocks), len(head)) + head + body


BLOCKS = [100, 50, 70]


def build_sample(path: Path, version: int = 3, moved: bool = False) -> dict:
    """/readme.txt, /FMOD/Desktop/CarrionCrone.bank, /Bundles2/_.index.bin and a dialogue bundle.

    With `moved`, an extra free record in front of the readme moves it.
    """
    b = Builder(version)
    if moved:
        b.free(64)
    readme = b.file("readme.txt", b"hello, ggpk")
    b.free(40)
    bank = b.file("CarrionCrone.bank", b"B" * 300)
    desktop = b.dir("Desktop", [bank])
    fmod = b.dir("FMOD", [desktop])
    dialogue = b.file("monsters.7.bundle.bin", bundle_bytes(BLOCKS))
    index = b.file("_.index.bin", bundle_bytes([30]))
    folder = b.dir("4.0", [dialogue])
    d = b.dir("dialogue", [folder])
    audio = b.dir("audio", [d])
    folders = b.dir("Folders", [audio])
    streaming = b.dir("Streaming", [folders])
    bundles = b.dir("Bundles2", [index, streaming])
    root = b.dir("", [readme, fmod, bundles])
    return b.build(root, path)


class TempDir(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = Path(self._tmp.name)
        self.assertIsNone(common.git_tree_of(self.tmp), "the temp folder sits inside a git checkout")
        self.path = self.tmp / "Content.ggpk"
        self.nodes = build_sample(self.path)
        self.tree = ggpk.walk_ggpk(self.path)
        self.files = {r["name"]: r for r in self.tree["records"] if r["kind"] == "file"}
        self.index = ggpk.TreeIndex(self.tree)

    def tearDown(self):
        self._tmp.cleanup()

    def run_main(self, argv):
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = ggpk.main(argv)
        return code, out.getvalue(), err.getvalue()


DIALOGUE = "/Bundles2/Streaming/Folders/audio/dialogue/4.0/monsters.7.bundle.bin"
BANK = "/FMOD/Desktop/CarrionCrone.bank"
INDEX = "/Bundles2/_.index.bin"


# --------------------------------------------------------------------------
# Baseline: the walk

class Walk(TempDir):
    def test_finds_every_file_with_its_data(self):
        self.assertEqual(sorted(self.files), sorted(["/readme.txt", BANK, INDEX, DIALOGUE]))
        data = self.path.read_bytes()
        readme = self.files["/readme.txt"]
        self.assertEqual(data[readme["data"]:readme["data"] + readme["size"]], b"hello, ggpk")
        bank = self.files[BANK]
        self.assertEqual(data[bank["data"]:bank["data"] + bank["size"]], b"B" * 300)

    def test_records_dirs_free_and_bundle_headers(self):
        kinds = {}
        for r in self.tree["records"]:
            kinds[r["kind"]] = kinds.get(r["kind"], 0) + 1
        self.assertEqual(kinds, {"ggpk": 1, "dir": 9, "free": 1, "file": 4})
        dirs = {r["name"] for r in self.tree["records"] if r["kind"] == "dir"}
        self.assertIn("/", dirs)
        self.assertIn("/Bundles2/Streaming/Folders/audio/dialogue/4.0", dirs)
        bundle = self.files[DIALOGUE]["bundle"]
        self.assertEqual(bundle["blocks"], BLOCKS)
        self.assertEqual(bundle["granularity"], ggpk.GRANULARITY)
        self.assertNotIn("bundle", self.files["/readme.txt"])
        self.assertEqual(self.tree["counts"]["bundles"], 2)
        self.assertEqual(self.tree["counts"]["bundle_errors"], 0)

    def test_version_4_names_are_utf32(self):
        path = self.tmp / "v4.ggpk"
        build_sample(path, version=4)
        tree = ggpk.walk_ggpk(path)
        self.assertEqual(tree["version"], 4)
        self.assertIn(BANK, {r.get("name") for r in tree["records"]})

    def test_not_a_ggpk_is_refused(self):
        bad = self.tmp / "bad.ggpk"
        bad.write_bytes(struct.pack("<I4s", 64, b"NOPE") + b"\0" * 56)
        with self.assertRaises(common.ToolError):
            ggpk.walk_ggpk(bad)

    def test_walk_writes_a_tree_that_loads_back(self):
        out = self.tmp / "live" / "ggpk" / "tree-before.json"
        code, stdout, _ = self.run_main(["walk", "--ggpk", str(self.path), "--out", str(out)])
        self.assertEqual(code, 0, stdout)
        self.assertIn("4 files", stdout)
        loaded = ggpk.load_tree(out)
        self.assertEqual(loaded["records"], self.tree["records"])

    def test_walk_refuses_an_output_inside_a_git_checkout(self):
        checkout = self.tmp / "checkout"
        (checkout / ".git").mkdir(parents=True)
        code, _, err = self.run_main(["walk", "--ggpk", str(self.path), "--out", str(checkout / "t.json")])
        self.assertEqual(code, 1)
        self.assertIn("git checkout", err)
        self.assertFalse((checkout / "t.json").exists())


# --------------------------------------------------------------------------
# Target: lookup

class Lookup(TempDir):
    def one(self, offset, size=1):
        hits = self.index.lookup(offset, size)
        self.assertEqual(len(hits), 1, hits)
        return hits[0]

    def test_first_and_last_data_byte_of_a_file(self):
        bank = self.files[BANK]
        first = self.one(bank["data"])
        self.assertEqual((first["name"], first["kind"], first["rel_offset"]), (BANK, "file", 0))
        last = self.one(bank["data"] + bank["size"] - 1)
        self.assertEqual((last["name"], last["kind"], last["rel_offset"]), (BANK, "file", 299))

    def test_a_record_header_byte(self):
        bank = self.files[BANK]
        hit = self.one(bank["offset"] + 4)  # the FILE tag
        self.assertEqual((hit["name"], hit["kind"], hit["rel_offset"]), (BANK, "record-header", 4))
        root = [r for r in self.tree["records"] if r["kind"] == "dir" and r["name"] == "/"][0]
        hit = self.one(root["offset"] + 20)
        self.assertEqual((hit["name"], hit["kind"]), ("/", "record-header"))
        self.assertEqual(self.one(5)["kind"], "record-header")  # the GGPK record

    def test_a_free_byte(self):
        free = [r for r in self.tree["records"] if r["kind"] == "free"][0]
        hit = self.one(free["offset"] + 20)
        self.assertEqual((hit["name"], hit["kind"], hit["rel_offset"]), (None, "free", 20))

    def test_a_bundle_header_byte(self):
        d = self.files[DIALOGUE]
        hit = self.one(d["data"] + 30)
        self.assertEqual((hit["name"], hit["kind"], hit["rel_offset"], hit["blocks"]),
                         (DIALOGUE, "bundle-header", 30, []))

    def test_a_byte_in_each_block(self):
        d = self.files[DIALOGUE]
        start = d["data"] + 12 + d["bundle"]["head"]
        data = self.path.read_bytes()
        edge = 0
        for k, size in enumerate(BLOCKS):
            for offset in (start + edge, start + edge + size // 2, start + edge + size - 1):
                hit = self.one(offset)
                self.assertEqual((hit["name"], hit["kind"], hit["blocks"]), (DIALOGUE, "bundle", [k]))
                self.assertEqual(data[offset], k + 1)  # the fake block bytes carry their index
            edge += size

    def test_a_read_across_two_blocks_names_both(self):
        d = self.files[DIALOGUE]
        start = d["data"] + 12 + d["bundle"]["head"]
        hit = self.one(start + BLOCKS[0] - 10, size=20)
        self.assertEqual(hit["blocks"], [0, 1])

    def test_a_read_spanning_two_records_returns_both(self):
        readme = self.files["/readme.txt"]
        free = [r for r in self.tree["records"] if r["kind"] == "free"][0]
        self.assertEqual(readme["data"] + readme["size"], free["offset"])
        hits = self.index.lookup(readme["data"] + readme["size"] - 2, size=6)
        self.assertEqual([(h["name"], h["kind"]) for h in hits],
                         [("/readme.txt", "file"), (None, "free")])
        self.assertEqual(hits[0]["rel_offset"], readme["size"] - 2)
        self.assertEqual(hits[1]["rel_offset"], 0)

    def test_bytes_past_the_end_are_unknown(self):
        size = self.path.stat().st_size
        hits = self.index.lookup(size - 1, size=10)
        self.assertEqual([h["kind"] for h in hits][-1], "unknown")
        self.assertEqual(hits[-1]["rel_offset"], size)

    def test_lookup_command_prints_entries(self):
        out = self.tmp / "tree.json"
        ggpk.write_tree(out, self.tree)
        bank = self.files[BANK]
        code, stdout, _ = self.run_main(["lookup", "--tree", str(out), str(bank["data"] + 5)])
        self.assertEqual(code, 0)
        self.assertEqual(json.loads(stdout.strip()),
                         {"name": BANK, "kind": "file", "rel_offset": 5, "blocks": []})


# --------------------------------------------------------------------------
# Target: diff and copy

class DiffAndCopy(TempDir):
    def test_identical_trees_exit_0(self):
        a, b = self.tmp / "a.json", self.tmp / "b.json"
        ggpk.write_tree(a, self.tree)
        ggpk.write_tree(b, ggpk.walk_ggpk(self.path))
        code, stdout, _ = self.run_main(["diff", str(a), str(b)])
        self.assertEqual(code, 0, stdout)
        self.assertIn("identical", stdout)

    def test_a_moved_record_is_reported(self):
        other = self.tmp / "moved.ggpk"
        build_sample(other, moved=True)
        a, b = self.tmp / "a.json", self.tmp / "b.json"
        ggpk.write_tree(a, self.tree)
        ggpk.write_tree(b, ggpk.walk_ggpk(other))
        code, stdout, _ = self.run_main(["diff", str(a), str(b)])
        self.assertEqual(code, 1)
        self.assertRegex(stdout, r"moved file /readme\.txt: offset \d+ -> \d+")
        self.assertIn("added free", stdout)

    def test_copy_writes_the_record_data(self):
        out = self.tmp / "live" / "ggpk" / "index.raw"
        ggpk.copy_record(self.path, self.tree, INDEX, out)
        rec = self.files[INDEX]
        self.assertEqual(out.read_bytes(), self.path.read_bytes()[rec["data"]:rec["data"] + rec["size"]])

    def test_copy_refuses_a_destination_inside_a_git_checkout(self):
        checkout = self.tmp / "checkout"
        (checkout / ".git").mkdir(parents=True)
        tree = self.tmp / "tree.json"
        ggpk.write_tree(tree, self.tree)
        code, _, err = self.run_main(["copy", "--ggpk", str(self.path), "--tree", str(tree),
                                      "--name", INDEX, "--out", str(checkout / "index.raw")])
        self.assertEqual(code, 1)
        self.assertIn("git checkout", err)
        self.assertFalse((checkout / "index.raw").exists())

    def test_copy_refuses_a_stale_tree(self):
        other = self.tmp / "moved.ggpk"
        build_sample(other, moved=True)
        with self.assertRaises(common.ToolError) as ctx:
            ggpk.copy_record(other, self.tree, BANK, self.tmp / "bank.out")
        self.assertIn("stale", str(ctx.exception))


# --------------------------------------------------------------------------
# Target: selfcheck

class Selfcheck(TempDir):
    def test_passes_on_a_ggpk_with_the_session_records(self):
        code, stdout, _ = self.run_main(["selfcheck", "--ggpk", str(self.path)])
        self.assertEqual(code, 0, stdout)
        self.assertIn("ggpk selfcheck: ok", stdout)
        self.assertIn("1 dialogue bundles", stdout)

    def test_names_what_is_missing(self):
        b = Builder()
        readme = b.file("readme.txt", b"x")
        root = b.dir("", [readme])
        path = self.tmp / "small.ggpk"
        b.build(root, path)
        code, stdout, _ = self.run_main(["selfcheck", "--ggpk", str(path)])
        self.assertEqual(code, 1)
        self.assertIn("FAIL no /FMOD/Desktop/CarrionCrone.bank", stdout)
        self.assertIn("FAIL no /Bundles2/_.index.bin", stdout)


if __name__ == "__main__":
    unittest.main()
