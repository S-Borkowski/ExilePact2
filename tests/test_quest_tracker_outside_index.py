"""tools/quest_tracker/outside/bundle_index: the decoded-index reader and the block-to-file annotation.

Nothing here reads the real `Content.ggpk` or a real index. The synthetic
index in `tests/fixtures/quest_tracker/outside/index/layout.json` (two
bundles, files crossing a 256 KiB block boundary) is packed by this test into
the decoded-index layout the reader assumes, so the fixture pins the reader's
field order; a real decode is the check of that order, once a decoder is
approved.

The baseline is the reader: the bundles, files and directories come back in
the order and with the values the layout holds, paths resolve through the
path hash the index turns out to use, and an index whose hashes match no
known function (the negative control) keeps its files and leaves their paths
unknown instead of guessing. The targets are the session's uses: a read in
each block is annotated with exactly the files that block holds, a read of a
bundle header, a bank file or the index bundle itself gets none, `files`
lists one block's files, and `extract-plan` lists the bundle ranges that hold
a path prefix.
"""
import contextlib
import io
import json
import shutil
import struct
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from tools.quest_tracker.outside import bundle_index, common  # noqa: E402
from tools.quest_tracker.outside.common import ToolError  # noqa: E402

FIXTURES = ROOT / "tests" / "fixtures" / "quest_tracker" / "outside" / "index"
LAYOUT = json.loads((FIXTURES / "layout.json").read_text(encoding="utf-8"))

DIALOGUE = "/Bundles2/Streaming/Folders/audio/dialogue/4.0/monsters.3.bundle.bin"
METADATA = "/Bundles2/Folders/metadata/monsters.bundle.bin"
NESTED = b"NESTED-PATH-BUNDLE"


# --------------------------------------------------------------------------
# Packing the synthetic layout

def _murmur(path: str) -> int:
    return bundle_index.murmur64a(path.lower().encode("utf-8"))


def _fnv(path: str) -> int:
    return bundle_index.fnv1a64((path.lower() + "++").encode("utf-8"))


def _cstring(text: str) -> bytes:
    return text.encode("utf-8") + b"\0"


def pack_paths(files) -> tuple[bytes, list[tuple[str, int, int]]]:
    """The path section, one directory record per folder, and its (folder, offset, size) list.

    Folders under `Audio/` and `Metadata/Monsters/CarrionCrone` are written
    through a base string (the base phase, then references to it); the others
    are written as whole literal paths, so both forms are exercised.
    """
    folders: dict[str, list[str]] = {}
    for f in files:
        folder, _, name = f["path"].rpartition("/")
        folders.setdefault(folder, []).append(name)
    data = bytearray()
    out = []
    for folder, names in folders.items():
        start = len(data)
        if folder.startswith("Audio/") or folder.endswith("CarrionCrone"):
            data += struct.pack("<I", 0)                      # base phase on
            data += struct.pack("<I", 1) + _cstring(folder + "/")
            data += struct.pack("<I", 0)                      # base phase off
            for name in names:
                data += struct.pack("<I", 1) + _cstring(name)  # base 0 + name
        else:
            for name in names:
                data += struct.pack("<I", 1) + _cstring(folder + "/" + name)
        out.append((folder, start, len(data) - start))
    return bytes(data), out


def pack_index(layout=LAYOUT, hash_fn=_murmur) -> tuple[bytes, bytes]:
    """The decoded index (bundles, files, directories, then the nested path bundle) and its paths."""
    paths, folders = pack_paths(layout["files"])
    out = bytearray()
    out += struct.pack("<i", len(layout["bundles"]))
    for b in layout["bundles"]:
        name = b["name"].encode("utf-8")
        out += struct.pack("<i", len(name)) + name + struct.pack("<I", b["uncompressed"])
    out += struct.pack("<i", len(layout["files"]))
    for f in layout["files"]:
        out += struct.pack("<QiiI", hash_fn(f["path"]), f["bundle"], f["offset"], f["size"])
    out += struct.pack("<i", len(folders))
    for folder, offset, size in folders:
        out += struct.pack("<QIII", hash_fn(folder), offset, size, size)
    out += NESTED
    return bytes(out), paths


class TempDir(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = Path(self._tmp.name)
        self.assertIsNone(common.git_tree_of(self.tmp), "the temp folder sits inside a git checkout")
        raw, paths = pack_index()
        self.bin = self.tmp / "index.dec"
        self.paths = self.tmp / "paths.dec"
        self.bin.write_bytes(raw)
        self.paths.write_bytes(paths)

    def tearDown(self):
        self._tmp.cleanup()

    def run_main(self, argv):
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = bundle_index.main(argv)
        return code, out.getvalue(), err.getvalue()

    def checkout(self) -> Path:
        checkout = self.tmp / "checkout"
        (checkout / ".git").mkdir(parents=True)
        return checkout


# --------------------------------------------------------------------------
# Baseline: the reader

class Hashes(unittest.TestCase):
    def test_murmur64a_of_the_empty_path(self):
        # The root directory's path hash public bundle readers test for to
        # recognise this hash: MurmurHash64A, seed 0x1337B33F, of "".
        self.assertEqual(bundle_index.murmur64a(b""), 0xF42A94E69CFF42FE)

    def test_murmur64a_tail_lengths_differ(self):
        seen = {bundle_index.murmur64a(b"abcdefghijklmnop"[:n]) for n in range(17)}
        self.assertEqual(len(seen), 17)

    def test_fnv1a64_of_the_empty_string_is_the_offset_basis(self):
        self.assertEqual(bundle_index.fnv1a64(b""), 0xCBF29CE484222325)


class Reader(TempDir):
    def test_records_come_back_in_the_layout_order(self):
        idx = bundle_index.parse_index_bin(self.bin.read_bytes())
        self.assertEqual(idx["bundles"], LAYOUT["bundles"])
        self.assertEqual([(f["bundle"], f["offset"], f["size"]) for f in idx["files"]],
                         [(f["bundle"], f["offset"], f["size"]) for f in LAYOUT["files"]])
        self.assertEqual([f["hash"] for f in idx["files"]],
                         ["%016x" % _murmur(f["path"]) for f in LAYOUT["files"]])
        self.assertEqual(len(idx["directories"]), 3)
        tail = self.bin.read_bytes()[idx["path_bundle"]["offset"]:]
        self.assertEqual(tail, NESTED)
        self.assertEqual(idx["path_bundle"]["size"], len(NESTED))

    def test_without_the_path_section_paths_are_unknown(self):
        idx = bundle_index.parse_index_bin(self.bin.read_bytes())
        self.assertEqual(idx["hash"], "none")
        self.assertTrue(all(f["path"] is None for f in idx["files"]))
        self.assertEqual(idx["counts"]["paths_resolved"], 0)

    def test_paths_resolve_through_murmur(self):
        idx = bundle_index.parse_index_bin(self.bin.read_bytes(), self.paths.read_bytes())
        self.assertEqual(idx["hash"], "murmur64a")
        self.assertEqual([f["path"] for f in idx["files"]], [f["path"] for f in LAYOUT["files"]])
        self.assertEqual(idx["counts"]["paths_resolved"], len(LAYOUT["files"]))

    def test_paths_resolve_through_fnv(self):
        raw, paths = pack_index(hash_fn=_fnv)
        idx = bundle_index.parse_index_bin(raw, paths)
        self.assertEqual(idx["hash"], "fnv1a64")
        self.assertEqual([f["path"] for f in idx["files"]], [f["path"] for f in LAYOUT["files"]])

    def test_hashes_no_function_matches_leave_paths_unknown(self):
        # Negative control: the same paths, hashed by neither function.
        raw, paths = pack_index(hash_fn=lambda p: _murmur(p) ^ 0x5A5A5A5A5A5A5A5A)
        idx = bundle_index.parse_index_bin(raw, paths)
        self.assertEqual(idx["hash"], "none")
        self.assertEqual(idx["counts"]["paths_listed"], len(LAYOUT["files"]))
        self.assertEqual(idx["counts"]["paths_resolved"], 0)
        self.assertTrue(all(f["path"] is None for f in idx["files"]))

    def test_a_truncated_index_is_refused(self):
        raw = self.bin.read_bytes()
        for cut in (2, 30, 100, 220, 290):
            with self.assertRaises(ToolError):
                bundle_index.parse_index_bin(raw[:cut])

    def test_a_file_naming_a_missing_bundle_is_refused(self):
        layout = json.loads(json.dumps(LAYOUT))
        layout["files"][0]["bundle"] = 9
        raw, _ = pack_index(layout)
        with self.assertRaises(ToolError):
            bundle_index.parse_index_bin(raw)

    def test_read_command_writes_an_index_that_loads_back(self):
        out = self.tmp / "index.json"
        code, stdout, err = self.run_main(["read", "--bin", str(self.bin), "--paths", str(self.paths),
                                           "--out", str(out)])
        self.assertEqual(code, 0, err)
        self.assertIn("2 bundles, 7 files", stdout)
        self.assertIn("7 of 7 paths resolved (murmur64a)", stdout)
        index = bundle_index.load_index(out)
        self.assertEqual(sorted(f["path"] for f in index.files), sorted(f["path"] for f in LAYOUT["files"]))
        # The binary loads the same way, without the JSON step.
        direct = bundle_index.load_index(self.bin, self.paths)
        self.assertEqual(direct.files, index.files)

    def test_read_refuses_an_output_inside_a_git_checkout(self):
        target = self.checkout() / "index.json"
        code, _, err = self.run_main(["read", "--bin", str(self.bin), "--out", str(target)])
        self.assertEqual(code, 1)
        self.assertIn("git checkout", err)
        self.assertFalse(target.exists())


# --------------------------------------------------------------------------
# Target: blocks to files

class Blocks(TempDir):
    def setUp(self):
        super().setUp()
        self.index = bundle_index.load_index(self.bin, self.paths)

    def names(self, files):
        return [f["path"].rpartition("/")[2] for f in files]

    def test_bundle_names_match_the_ggpk_names(self):
        self.assertEqual(self.index.bundle_for(DIALOGUE), 0)
        self.assertEqual(self.index.bundle_for(DIALOGUE.upper()), 0)
        self.assertEqual(self.index.bundle_for(METADATA), 1)
        self.assertIsNone(self.index.bundle_for("/Bundles2/_.index.bin"))
        self.assertIsNone(self.index.bundle_for("/FMOD/Desktop/CarrionCrone.bank"))

    def test_a_streaming_bundle_listed_without_its_streaming_folder_matches(self):
        layout = json.loads(json.dumps(LAYOUT))
        layout["bundles"][0]["name"] = "Folders/audio/dialogue/4.0/monsters.3"
        raw, paths = pack_index(layout)
        index = bundle_index.BundleIndex(bundle_index.parse_index_bin(raw, paths))
        self.assertEqual(index.bundle_for(DIALOGUE), 0)

    def test_a_read_in_each_block_names_that_blocks_files(self):
        self.assertEqual(self.names(self.index.files_in_blocks(0, [0])), ["a.ogg", "b.ogg"])
        self.assertEqual(self.names(self.index.files_in_blocks(0, [1])), ["b.ogg", "c.ogg"])
        self.assertEqual(self.names(self.index.files_in_blocks(0, [2])), ["c.ogg"])
        self.assertEqual(self.names(self.index.files_in_blocks(1, [1])), ["x.ao", "death.ogg"])

    def test_each_file_carries_the_blocks_it_was_found_in(self):
        hits = self.index.files_in_blocks(1, [0, 1])
        self.assertEqual(self.names(hits), ["CarrionCrone.ao", "CarrionCrone.otc", "x.ao", "death.ogg"])
        self.assertEqual([h["blocks"] for h in hits], [[0], [0], [0, 1], [1]])
        self.assertEqual(hits[2]["bundle"], "Folders/metadata/monsters")

    def test_a_block_past_the_bundles_end_holds_nothing(self):
        self.assertEqual(self.index.files_in_blocks(1, [2]), [])

    def test_a_file_ending_on_a_block_boundary_stays_in_its_block(self):
        layout = json.loads(json.dumps(LAYOUT))
        layout["files"][0]["size"] = 262144
        layout["files"][1]["offset"] = 262144
        layout["files"][1]["size"] = 37856
        raw, paths = pack_index(layout)
        index = bundle_index.BundleIndex(bundle_index.parse_index_bin(raw, paths))
        self.assertEqual(self.names(index.files_in_blocks(0, [0])), ["a.ogg"])
        self.assertEqual(self.names(index.files_in_blocks(0, [1])), ["b.ogg", "c.ogg"])


class Annotate(TempDir):
    def setUp(self):
        super().setUp()
        self.live = self.tmp / "live"
        (self.live / "i1").mkdir(parents=True)
        (self.live / "ggpk").mkdir()
        shutil.copyfile(FIXTURES / "mapped.jsonl", self.live / "i1" / "mapped.jsonl")
        code, _, err = self.run_main(["read", "--bin", str(self.bin), "--paths", str(self.paths),
                                      "--out", str(self.live / "ggpk" / "index.json")])
        self.assertEqual(code, 0, err)

    def annotated(self):
        return common.read_jsonl(self.live / "i1" / "mapped.jsonl")

    def files_of(self, rec):
        return [(f["path"].rpartition("/")[2], f["blocks"]) for f in rec["files"]]

    def test_annotate_adds_each_reads_files(self):
        code, stdout, err = self.run_main(["annotate", "--live", str(self.live)])
        self.assertEqual(code, 0, err)
        self.assertIn("8 reads", stdout)
        recs = self.annotated()
        self.assertEqual(self.files_of(recs[0]), [("a.ogg", [0]), ("b.ogg", [0])])
        self.assertEqual(self.files_of(recs[1]), [("b.ogg", [1]), ("c.ogg", [1])])
        self.assertEqual(self.files_of(recs[2]), [("c.ogg", [2])])
        self.assertEqual(self.files_of(recs[4]), [("CarrionCrone.ao", [0]), ("CarrionCrone.otc", [0]),
                                                  ("x.ao", [0, 1]), ("death.ogg", [1])])
        # A read across two bundles gets the files of both.
        self.assertEqual(self.files_of(recs[7]), [("c.ogg", [2]), ("x.ao", [1]), ("death.ogg", [1])])

    def test_reads_outside_a_known_bundles_blocks_get_no_files(self):
        self.assertEqual(self.run_main(["annotate", "--live", str(self.live)])[0], 0)
        recs = self.annotated()
        self.assertEqual(recs[3]["files"], [])        # a bundle header
        self.assertEqual(recs[5]["files"], [])        # the index bundle itself
        self.assertEqual(recs[5]["files_unmatched"], ["/Bundles2/_.index.bin"])
        self.assertEqual(recs[6]["files"], [])        # the FMOD bank, a plain file
        self.assertNotIn("files_unmatched", recs[6])

    def test_annotate_keeps_every_other_field_and_runs_twice_the_same(self):
        before = common.read_jsonl(FIXTURES / "mapped.jsonl")
        self.assertEqual(self.run_main(["annotate", "--live", str(self.live)])[0], 0)
        once = self.annotated()
        for a, b in zip(before, once):
            self.assertEqual({k: v for k, v in b.items() if k not in ("files", "files_unmatched")}, a)
        self.assertEqual(self.run_main(["annotate", "--live", str(self.live)])[0], 0)
        self.assertEqual(self.annotated(), once)

    def test_annotate_without_an_index_names_it(self):
        (self.live / "ggpk" / "index.json").unlink()
        code, _, err = self.run_main(["annotate", "--live", str(self.live)])
        self.assertEqual(code, 1)
        self.assertIn("index.json", err)

    def test_annotate_refuses_an_output_inside_a_git_checkout(self):
        target = self.checkout() / "mapped.jsonl"
        code, _, err = self.run_main(["annotate", "--live", str(self.live), "--out", str(target)])
        self.assertEqual(code, 1)
        self.assertIn("git checkout", err)
        self.assertFalse(target.exists())

    def test_files_command_lists_one_blocks_files(self):
        code, stdout, err = self.run_main(["files", "--index", str(self.live / "ggpk" / "index.json"),
                                           "--bundle", DIALOGUE, "--block", "1"])
        self.assertEqual(code, 0, err)
        lines = [l for l in stdout.splitlines() if l.strip()]
        self.assertEqual(len(lines), 2, stdout)
        self.assertTrue(lines[0].endswith("Audio/Dialogue/4.0/Monsters/b.ogg"), lines)
        self.assertTrue(lines[1].endswith("Audio/Dialogue/4.0/Monsters/c.ogg"), lines)

    def test_files_command_refuses_an_unknown_bundle(self):
        code, _, err = self.run_main(["files", "--index", str(self.live / "ggpk" / "index.json"),
                                      "--bundle", "/Bundles2/_.index.bin"])
        self.assertEqual(code, 1)
        self.assertIn("not in the index", err)


class ExtractPlan(TempDir):
    def setUp(self):
        super().setUp()
        self.index = bundle_index.load_index(self.bin, self.paths)

    def test_a_prefix_lists_its_bundle_ranges_and_blocks(self):
        plan = self.index.extract_plan("metadata/monsters/carrioncrone/")
        self.assertEqual(len(plan), 1)
        entry = plan[0]
        self.assertEqual(entry["bundle"], "Folders/metadata/monsters")
        self.assertEqual(entry["ggpk_name"], METADATA)
        self.assertEqual(entry["ranges"], [[0, 3000], [270000, 290000]])
        self.assertEqual(entry["blocks"], [0, 1])
        self.assertEqual([f["path"] for f in entry["files"]],
                         ["Metadata/Monsters/CarrionCrone/CarrionCrone.ao",
                          "Metadata/Monsters/CarrionCrone/CarrionCrone.otc",
                          "Metadata/Monsters/CarrionCrone/death.ogg"])

    def test_a_prefix_across_bundles_lists_each(self):
        plan = self.index.extract_plan("/")
        self.assertEqual([e["bundle"] for e in plan], [b["name"] for b in LAYOUT["bundles"]])
        self.assertEqual(plan[0]["ranges"], [[0, 600000]])
        self.assertEqual(plan[0]["blocks"], [0, 1, 2])

    def test_a_prefix_nothing_has_is_empty(self):
        self.assertEqual(self.index.extract_plan("metadata/monsters/nobody/"), [])

    def test_extract_plan_needs_paths(self):
        index = bundle_index.load_index(self.bin)
        with self.assertRaises(ToolError):
            index.extract_plan("metadata/")

    def test_extract_plan_command_prints_json(self):
        code, stdout, err = self.run_main(["extract-plan", "--index", str(self.bin), "--paths", str(self.paths),
                                           "--prefix", "Metadata\\Monsters\\CarrionCrone\\"])
        self.assertEqual(code, 0, err)
        plan = json.loads(stdout)
        self.assertEqual(plan["prefix"], "metadata/monsters/carrioncrone/")
        self.assertEqual(plan["bundles"][0]["ranges"], [[0, 3000], [270000, 290000]])


if __name__ == "__main__":
    unittest.main()
