"""tools/quest_tracker/outside/fileio: the trace parser, reads, map, rehearsal-check and export.

Nothing here starts a trace or reads the real `Content.ggpk`. The traces are
synthetic `tracerpt -of XML` exports under
`tests/fixtures/quest_tracker/outside/tracerpt/`, with invented names, in the
shape tracerpt writes kernel events on this machine (the class GUID in
`ExtendedTracingInfo`, the class name in `RenderingInfo`, fields padded or in
hex, and local times with the minute-short offset tracerpt prints). `export`
runs an injected stand-in for tracerpt. The one test that touches the disk
directly is the probe's, on a 1 MiB file in a temporary folder.

The baseline is a trace with other processes' reads and other files' reads
in it: `reads` keeps only the given pid's reads of the given file. The
targets are the routes the session depends on: a name resolved through the
FileObject's Create and, failing that, through the FileKey's rundown name; a
pid taken from the header or, when the header has none, through the thread
map; a read whose fields tracerpt left undecoded; the mapped read list; and
the two rehearsal lines, which pass with both probe reads and the mapped page
and fail as cache-blind without the second read.
"""
import contextlib
import datetime
import io
import json
import shutil
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from tools.quest_tracker.outside import common, fileio, ggpk  # noqa: E402

FIXTURES = ROOT / "tests" / "fixtures" / "quest_tracker" / "outside" / "tracerpt"
UTC = datetime.timezone.utc

GAME_PID = 4242
PROBE_PID = 3131
PROBE = {
    "pid": PROBE_PID,
    "file": r"C:\Users\example\live\i1\rehearsal\probe.bin",
    "offset_a": 4 * 2 ** 20,
    "size_a": 64 * 2 ** 10,
    "reads_utc": ["2026-10-26T10:00:02.000Z", "2026-10-26T10:00:03.000Z"],
    "offset_b": 32 * 2 ** 20,
    "mapped_utc": "2026-10-26T10:00:04.000Z",
}


def t(seconds: float) -> str:
    when = datetime.datetime(2026, 10, 26, 10, 0, 0, tzinfo=UTC) + datetime.timedelta(seconds=seconds)
    return common.format_utc(when)


class TempLive(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = Path(self._tmp.name)
        self.assertIsNone(common.git_tree_of(self.tmp), "the temp folder sits inside a git checkout")
        self.live = self.tmp / "live"
        for sub in common.SUBFOLDERS:
            (self.live / sub).mkdir(parents=True, exist_ok=True)

    def tearDown(self):
        self._tmp.cleanup()

    def put_session(self):
        shutil.copy(FIXTURES / "session.xml", self.live / "i1" / "session.xml")

    def put_rehearsal(self, fixture: str, probe=None):
        folder = self.live / "i1" / "rehearsal"
        shutil.copy(FIXTURES / fixture, folder / "rehearsal.xml")
        common.write_json(folder / "probe.json", probe or PROBE)


# --------------------------------------------------------------------------
# Times

class TraceTime(unittest.TestCase):
    def test_offset_is_rounded_to_the_quarter_hour(self):
        # tracerpt wrote +01:59 for CEST (measured): the true offset is +02:00.
        got = fileio.parse_trace_time("2025-06-23T23:57:02.186084400+01:59")
        self.assertEqual(got, datetime.datetime(2025, 6, 23, 21, 57, 2, 186084, tzinfo=UTC))
        got = fileio.parse_trace_time("2026-10-26T11:00:00.000000000+00:59")
        self.assertEqual(got, datetime.datetime(2026, 10, 26, 10, 0, 0, tzinfo=UTC))

    def test_z_and_negative_offsets(self):
        self.assertEqual(fileio.parse_trace_time("2026-10-26T10:00:00.5Z"),
                         datetime.datetime(2026, 10, 26, 10, 0, 0, 500000, tzinfo=UTC))
        self.assertEqual(fileio.parse_trace_time("2026-10-26T05:00:00.000-05:00"),
                         datetime.datetime(2026, 10, 26, 10, 0, 0, tzinfo=UTC))

    def test_header_start_time_agrees_with_the_rounded_offset(self):
        scan = fileio.scan_trace(FIXTURES / "session.xml")
        self.assertEqual(scan.stats["clock_check_ms"], 0.0)


# --------------------------------------------------------------------------
# Baseline and targets: the scan and `reads`

class Reads(TempLive):
    def setUp(self):
        super().setUp()
        self.put_session()
        self.result = fileio.reads(self.live, GAME_PID)
        self.rows = common.read_jsonl(self.live / "i1" / "reads.jsonl")
        self.by_offset = {r["offset"]: r for r in self.rows}

    def test_keeps_only_the_pid_and_the_path(self):
        self.assertEqual(sorted(self.by_offset), [0x1000, 0x8000, 0x40000, 0x200000, 0x300000])
        self.assertNotIn(0x2000, self.by_offset)    # pid 5151's read of the same file
        self.assertNotIn(0x0, self.by_offset)       # the game's read of another file
        self.assertNotIn(0x310000, self.by_offset)  # a thread no event maps to a process
        for r in self.rows:
            self.assertEqual(r["pid"], GAME_PID)
            self.assertTrue(r["file"].lower().endswith("\\content.ggpk"))
            self.assertEqual(r["op"], "read")

    def test_rows_carry_the_contract_fields(self):
        for key in ("utc", "pid", "pid_source", "tid", "op", "file", "offset", "size", "flags"):
            self.assertIn(key, self.rows[0])
        first = self.by_offset[0x1000]
        self.assertEqual((first["utc"], first["size"], first["flags"], first["tid"]),
                         (t(2), 4096, 0x60900, 777))

    def test_resolves_a_name_by_file_object_and_by_file_key(self):
        self.assertEqual(self.by_offset[0x1000]["name_source"], "FileObject")
        # FileObject 0xD1 has no Create; its FileKey is named only by the end-of-trace rundown.
        self.assertEqual(self.by_offset[0x40000]["name_source"], "FileKey")
        self.assertTrue(self.by_offset[0x40000]["file"].endswith("Content.ggpk"))

    def test_takes_the_pid_from_the_header_or_the_thread_map(self):
        self.assertEqual(self.by_offset[0x1000]["pid_source"], "header")
        self.assertEqual(self.by_offset[0x8000]["pid_source"], "thread")    # DCStart
        self.assertEqual(self.by_offset[0x300000]["pid_source"], "thread")  # Start
        self.assertEqual(self.result["pid_sources"], {"header": 3, "thread": 2})

    def test_an_undecoded_read_takes_the_binary_route(self):
        row = self.by_offset[0x200000]
        self.assertEqual((row["size"], row["tid"], row["flags"]), (65536, 777, 0))
        self.assertEqual(self.result["stats"]["route_binary"], 1)

    def test_counts_writes_to_the_ggpk_from_any_pid(self):
        self.assertEqual(self.result["writes"], 1)
        self.assertEqual(self.result["write_pids"], [6262])

    def test_command_prints_counts(self):
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            code = fileio.main(["reads", "--live", str(self.live), "--pid", str(GAME_PID)])
        self.assertEqual(code, 0)
        self.assertIn("5 reads by pid 4242", out.getvalue())
        self.assertIn("binary 1", out.getvalue())

    def test_refuses_an_output_inside_a_git_checkout(self):
        checkout = self.tmp / "checkout"
        (checkout / ".git").mkdir(parents=True)
        live = checkout / "live"
        (live / "i1").mkdir(parents=True)
        shutil.copy(FIXTURES / "session.xml", live / "i1" / "session.xml")
        with self.assertRaises(common.ToolError):
            fileio.reads(live, GAME_PID)
        self.assertFalse((live / "i1" / "reads.jsonl").exists())


# --------------------------------------------------------------------------
# Target: map

class Map(TempLive):
    def test_each_read_gets_its_records(self):
        rows = [
            {"utc": t(1), "pid": GAME_PID, "pid_source": "header", "tid": 1, "op": "read",
             "file": "x", "offset": 100, "size": 50, "flags": 0},
            {"utc": t(2), "pid": GAME_PID, "pid_source": "header", "tid": 1, "op": "read",
             "file": "x", "offset": 1190, "size": 20, "flags": 0},
        ]
        with open(self.live / "i1" / "reads.jsonl", "w", encoding="utf-8") as f:
            for r in rows:
                f.write(json.dumps(r) + "\n")
        tree = {"format": ggpk.TREE_FORMAT, "ggpk_size": 4000, "version": 3, "root": 0, "free_head": 0,
                "records": [
                    {"kind": "file", "name": "/a.bank", "offset": 60, "length": 1000, "data": 100,
                     "size": 960},
                    {"kind": "file", "name": "/Bundles2/b.bundle.bin", "offset": 1060, "length": 2000,
                     "data": 1100, "size": 1960,
                     "bundle": {"head": 52, "uncompressed": 262144, "payload": 100,
                                "granularity": 262144, "blocks": [100]}},
                ]}
        ggpk.write_tree(self.live / "ggpk" / "tree-before.json", tree)
        res = fileio.map_reads(self.live)
        self.assertEqual(res["tree"], self.live / "ggpk" / "tree-before.json")
        mapped = common.read_jsonl(self.live / "i1" / "mapped.jsonl")
        self.assertEqual(mapped[0]["records"],
                         [{"name": "/a.bank", "kind": "file", "rel_offset": 0, "blocks": []}])
        # 1190..1209: the bundle header ends at 1100 + 12 + 52 = 1164, so this read is block 0.
        self.assertEqual(mapped[1]["records"],
                         [{"name": "/Bundles2/b.bundle.bin", "kind": "bundle", "rel_offset": 90,
                           "blocks": [0]}])
        self.assertEqual(res["kinds"], {"file": 1, "bundle": 1})

    def test_prefers_the_after_walk(self):
        (self.live / "ggpk" / "tree-after.json").write_text("{}", encoding="utf-8")
        self.assertEqual(fileio.default_tree(self.live), self.live / "ggpk" / "tree-after.json")


# --------------------------------------------------------------------------
# Target: rehearsal-check

class RehearsalCheck(TempLive):
    def lines(self):
        def no_tracerpt(*a, **k):
            raise AssertionError("the XML is present; tracerpt must not run")
        return fileio.rehearsal_check(self.live, run=no_tracerpt)

    def verdicts(self, lines):
        return {line.split(" | ")[0][2:]: line.rsplit(" | ", 1)[1] for line in lines}

    def test_both_probe_reads_and_the_mapped_page_pass(self):
        self.put_rehearsal("rehearsal-pass.xml")
        lines = self.lines()
        self.assertEqual(self.verdicts(lines), {"i1-rehearsal": "pass", "i1-rehearsal-mapped": "pass"})
        self.assertTrue(lines[0].startswith("- i1-rehearsal | expected: "))
        self.assertIn("| observed: ", lines[0])
        self.assertIn("flags 0x43 (paging bit set), pid from thread", lines[1])

    def test_a_missing_second_read_is_a_cache_blind_fail(self):
        self.put_rehearsal("rehearsal-cache-blind.xml")
        lines = self.lines()
        self.assertEqual(self.verdicts(lines)["i1-rehearsal"], "fail")
        self.assertIn("cache-blind", lines[0])
        # The cache manager's own read at A (another size) does not stand in for the second read.
        self.assertEqual(self.verdicts(lines)["i1-rehearsal-mapped"], "pass")

    def test_a_wrong_pid_fails_naming_the_pid_seen(self):
        probe = dict(PROBE, pid=9999)
        self.put_rehearsal("rehearsal-pass.xml", probe)
        lines = self.lines()
        self.assertEqual(self.verdicts(lines),
                         {"i1-rehearsal": "fail", "i1-rehearsal-mapped": "not-observed"})
        self.assertIn("by pid 3131, not the probe's pid 9999", lines[0])

    def test_no_events_fails(self):
        self.put_rehearsal("rehearsal-empty.xml")
        lines = self.lines()
        self.assertEqual(self.verdicts(lines),
                         {"i1-rehearsal": "fail", "i1-rehearsal-mapped": "not-observed"})
        self.assertIn("no FileIo events", lines[0])

    def test_no_probe_is_not_run(self):
        lines = self.lines()
        self.assertEqual([v.split(" ")[0] for v in self.verdicts(lines).values()], ["not-run", "not-run"])

    def test_exports_the_rehearsal_etl_when_its_xml_is_missing(self):
        folder = self.live / "i1" / "rehearsal"
        (folder / "rehearsal.etl").write_bytes(b"etl")
        common.write_json(folder / "probe.json", PROBE)
        calls = []

        def fake_tracerpt(cmd, cwd=None, capture_output=None, text=None):
            calls.append(cmd)
            shutil.copy(FIXTURES / "rehearsal-pass.xml", cmd[3])
            return SimpleNamespace(returncode=0, stdout="The command completed successfully.", stderr="")

        lines = fileio.rehearsal_check(self.live, run=fake_tracerpt)
        self.assertEqual(calls, [["tracerpt", str(folder / "rehearsal.etl"), "-o",
                                  str(folder / "rehearsal.xml"), "-of", "XML", "-lr", "-y"]])
        self.assertEqual(self.verdicts(lines)["i1-rehearsal"], "pass")


class Export(TempLive):
    def test_a_failing_tracerpt_is_an_error(self):
        (self.live / "i1" / "session.etl").write_bytes(b"etl")

        def failing(cmd, **kw):
            return SimpleNamespace(returncode=5, stdout="", stderr="Error: bad file")

        with self.assertRaises(common.ToolError) as ctx:
            fileio.export(self.live, "session", run=failing)
        self.assertIn("bad file", str(ctx.exception))

    def test_refuses_an_export_inside_a_git_checkout(self):
        checkout = self.tmp / "checkout"
        (checkout / ".git").mkdir(parents=True)
        (checkout / "i1").mkdir()
        (checkout / "i1" / "session.etl").write_bytes(b"etl")
        with self.assertRaises(common.ToolError):
            fileio.export(checkout, "session", run=lambda *a, **k: self.fail("tracerpt ran"))


# --------------------------------------------------------------------------
# The probe, on a small file

@unittest.skipUnless(sys.platform == "win32", "the probe calls the Windows file API")
class Probe(TempLive):
    def test_writes_reads_maps_and_records(self):
        rec = fileio.probe(self.live, total=1 << 20, offset_a=1 << 18, size_a=1 << 16,
                           offset_b=3 << 18, pause=0.0)
        path = self.live / "i1" / "rehearsal" / "probe.bin"
        self.assertEqual(path.stat().st_size, 1 << 20)
        saved = common.read_json(self.live / "i1" / "rehearsal" / "probe.json")
        self.assertEqual(saved, rec)
        self.assertEqual(set(saved), {"pid", "file", "offset_a", "size_a", "reads_utc", "offset_b",
                                      "mapped_utc"})
        self.assertEqual(len(saved["reads_utc"]), 2)
        for stamp in saved["reads_utc"] + [saved["mapped_utc"]]:
            common.parse_utc(stamp)
        self.assertEqual(fileio.suffix_of(saved["file"]).lower(),
                         str(path).split(":", 1)[1].lower())


if __name__ == "__main__":
    unittest.main()
