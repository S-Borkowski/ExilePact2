"""tools/quest_tracker/outside/run.py: how `spawn_detached` starts an instrument.

Live 6 was blind at G2: `run start` launched the three instruments with
`DETACHED_PROCESS`, and none of them wrote a ready line. A process started
with that flag has no console at all, and Windows ignores `CREATE_NO_WINDOW`
beside it. The fix starts each one with `CREATE_NO_WINDOW` and its own
process group instead, still breaking away from the caller's job when the
job allows it.

The baseline is the flag set `Popen` receives, read from a recording double:
no `DETACHED_PROCESS` bit, the `CREATE_NO_WINDOW` and `CREATE_NEW_PROCESS_GROUP`
bits set, with the breakaway bit on the first try and dropped on the retry
after a job refuses it. The positive control on the spawn itself is a real
`powershell.exe` started through `spawn_detached` that writes a file in a
temporary folder outside the repository; it runs only on Windows and needs no
game.

`CaptureBands` covers `run start --top/--bottom`: the baseline is the capture
command with neither option (today's, no `-Top`/`-Bottom`), the target is
`--bottom 0,0,1,1` reaching the capture as its `-Bottom`, and a malformed band
is refused, by name, before anything is spawned.
"""
import io
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from tools.quest_tracker.outside import run  # noqa: E402

CREATE_NO_WINDOW = 0x08000000
DETACHED_PROCESS = 0x00000008
CREATE_NEW_PROCESS_GROUP = 0x00000200
CREATE_BREAKAWAY_FROM_JOB = 0x01000000


class _Proc:
    pid = 4242


class SpawnFlags(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.log = Path(self._tmp.name) / "instrument.log"

    def _spawn(self, popen):
        with mock.patch.object(run.sys, "platform", "win32"), \
                mock.patch.object(run.subprocess, "Popen", popen):
            return run.spawn_detached(["powershell.exe", "-NoProfile"], self.log, cwd=Path(self._tmp.name))

    def test_constants_match_windows(self):
        self.assertEqual(run.CREATE_NO_WINDOW, CREATE_NO_WINDOW)
        self.assertEqual(run.CREATE_NEW_PROCESS_GROUP, CREATE_NEW_PROCESS_GROUP)
        self.assertEqual(run.CREATE_BREAKAWAY_FROM_JOB, CREATE_BREAKAWAY_FROM_JOB)
        if sys.platform == "win32":
            self.assertEqual(run.CREATE_NO_WINDOW, subprocess.CREATE_NO_WINDOW)
            self.assertEqual(DETACHED_PROCESS, subprocess.DETACHED_PROCESS)

    def test_flags_have_no_window_and_no_detached(self):
        calls = []

        def popen(command, **kw):
            calls.append(kw)
            return _Proc()

        self.assertEqual(self._spawn(popen), 4242)
        self.assertEqual(len(calls), 1)
        flags = calls[0]["creationflags"]
        self.assertFalse(flags & DETACHED_PROCESS, hex(flags))
        self.assertTrue(flags & CREATE_NO_WINDOW, hex(flags))
        self.assertTrue(flags & CREATE_NEW_PROCESS_GROUP, hex(flags))
        self.assertTrue(flags & CREATE_BREAKAWAY_FROM_JOB, hex(flags))
        self.assertIs(calls[0]["stdin"], subprocess.DEVNULL)
        self.assertIs(calls[0]["stderr"], subprocess.STDOUT)

    def test_retry_without_breakaway_keeps_no_window(self):
        calls = []

        def popen(command, **kw):
            calls.append(kw["creationflags"])
            if kw["creationflags"] & CREATE_BREAKAWAY_FROM_JOB:
                raise PermissionError("the job forbids breakaway")
            return _Proc()

        self.assertEqual(self._spawn(popen), 4242)
        self.assertEqual(len(calls), 2)
        self.assertTrue(calls[0] & CREATE_BREAKAWAY_FROM_JOB)
        retry = calls[1]
        self.assertFalse(retry & CREATE_BREAKAWAY_FROM_JOB, hex(retry))
        self.assertFalse(retry & DETACHED_PROCESS, hex(retry))
        self.assertTrue(retry & CREATE_NO_WINDOW, hex(retry))
        self.assertTrue(retry & CREATE_NEW_PROCESS_GROUP, hex(retry))


TODAY_CAPTURE_TAIL = ["-TargetPid", "1234", "-OutDir", None, "-StopFile", None, "-Hz", "2",
                      "-MaxMinutes", "90.0"]


class CaptureBands(unittest.TestCase):
    """`run start --top/--bottom`: each band reaches the capture as `-Top` or
    `-Bottom`, without them the capture command is today's, and a malformed band
    is refused before anything is spawned. Nothing here starts a process: the
    spawn is a recording double."""

    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.live = Path(self._tmp.name) / "live"
        self.live.mkdir()
        run.common.write_json(self.live / run.common.SESSION_JSON, {"client_txt_offset": 0})
        self.spawned = []

    def capture(self, **bands):
        return {t.name: t for t in run.instruments(self.live, 1234, **bands)}["capture"].command

    def fake_spawn(self, command, log):
        self.spawned.append(list(command))
        return 4242

    def main(self, *extra):
        real_start = run.start

        def start(*args, **kw):
            return real_start(*args, spawn=self.fake_spawn, created=lambda pid: None, **kw)

        err = io.StringIO()
        with mock.patch.object(run, "start", side_effect=start), \
                mock.patch.object(run.sys, "stdout", io.StringIO()), \
                mock.patch.object(run.sys, "stderr", err):
            code = run.main(["start", "--live", str(self.live), "--pid", "1234", "--timeout", "0", *extra])
        return code, err.getvalue()

    def test_baseline_no_option_keeps_todays_capture_command(self):
        cap = self.capture()
        self.assertNotIn("-Top", cap)
        self.assertNotIn("-Bottom", cap)
        tail = cap[cap.index("-TargetPid"):]
        expected = list(TODAY_CAPTURE_TAIL)
        expected[3] = str(self.live / "i3")
        expected[5] = str(run.common.stop_file(self.live, "i3"))
        self.assertEqual(tail, expected)
        self.assertEqual(self.capture(top=None, bottom=None), cap)
        # Through the command line too: no option, no band argument.
        code, err = self.main()
        self.assertEqual(code, 1, err)          # nothing real ran, so nothing is ready
        spawned = [c for c in self.spawned if "-OutDir" in c]
        self.assertEqual(spawned, [cap])

    def test_target_bottom_reaches_the_capture(self):
        code, err = self.main("--bottom", "0,0,1,1")
        self.assertEqual(err, "")
        cap = [c for c in self.spawned if "-OutDir" in c]
        self.assertEqual(len(cap), 1)
        cap = cap[0]
        self.assertEqual(cap[cap.index("-Bottom") + 1], "0,0,1,1")
        self.assertNotIn("-Top", cap)
        recorded = run.common.read_json(self.live / run.RUN_JSON)["instruments"]["capture"]["command"]
        self.assertEqual(recorded, cap)
        # The other instruments do not take a band.
        for command in self.spawned:
            if command is not cap:
                self.assertNotIn("-Bottom", command)

    def test_top_and_off_reach_the_capture(self):
        cap = self.capture(top="0.2, 0, 0.8, 0.1", bottom="off")
        self.assertEqual(cap[cap.index("-Top") + 1], "0.2, 0, 0.8, 0.1")
        self.assertEqual(cap[cap.index("-Bottom") + 1], "off")
        cap = self.capture(top="OFF")
        self.assertEqual(cap[cap.index("-Top") + 1], "off")
        self.assertNotIn("-Bottom", cap)

    def test_malformed_band_is_refused_before_anything_spawns(self):
        code, err = self.main("--bottom", "0,0,1")
        self.assertEqual(code, 1)
        self.assertIn("--bottom", err)
        self.assertIn("0,0,1", err)
        self.assertEqual(self.spawned, [])
        self.assertFalse((self.live / run.RUN_JSON).exists())

        code, err = self.main("--top", "0.5,0,0.5,1")
        self.assertEqual(code, 1)
        self.assertIn("--top", err)
        self.assertEqual(self.spawned, [])
        self.assertFalse((self.live / run.RUN_JSON).exists())

    def test_each_malformed_shape_is_refused_by_name(self):
        for text in ("0,0,1", "0,0,1,1,1", "a,0,1,1", "0,0,1.5,1", "-0.1,0,1,1", "0,0.5,1,0.5",
                     "0.6,0,0.4,1", "nan,0,1,1", "0,0,inf,1", ""):
            with self.subTest(text=text):
                with self.assertRaises(run.ToolError) as caught:
                    self.capture(bottom=text)
                self.assertIn("--bottom", str(caught.exception))
                with self.assertRaises(run.ToolError) as caught:
                    self.capture(top=text)
                self.assertIn("--top", str(caught.exception))

    def test_both_bands_off_is_refused(self):
        with self.assertRaises(run.ToolError) as caught:
            self.capture(top="off", bottom="off")
        self.assertIn("both", str(caught.exception))
        # One band off alone leaves the other at the capture's default.
        self.capture(top="off")
        self.capture(bottom="off")


@unittest.skipUnless(sys.platform == "win32", "spawns Windows PowerShell")
class SpawnSmoke(unittest.TestCase):
    """A real spawn, no game: the child must run its command."""

    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.dir = Path(self._tmp.name)

    def test_spawned_powershell_writes_its_file(self):
        out = self.dir / "ok.txt"
        log = self.dir / "spawn.log"
        command = ["powershell.exe", "-NoProfile", "-Command",
                   "'ok' | Out-File -FilePath '{}'".format(str(out).replace("'", "''"))]
        pid = run.spawn_detached(command, log, cwd=self.dir)
        self.assertGreater(pid, 0)
        deadline = time.monotonic() + 10.0
        while time.monotonic() < deadline and not out.exists():
            time.sleep(0.1)
        self.assertTrue(out.exists(), "the spawned powershell.exe wrote nothing in 10 s; log: "
                        + (log.read_text(errors="replace") if log.exists() else "<none>"))
        # Let the child finish writing and exit before the folder is removed.
        deadline = time.monotonic() + 10.0
        while time.monotonic() < deadline and run.alive(pid, None):
            time.sleep(0.1)
        # Windows PowerShell 5.1's Out-File writes UTF-16 LE with a BOM.
        self.assertEqual(out.read_text(encoding="utf-16").strip(), "ok")


if __name__ == "__main__":
    unittest.main()
