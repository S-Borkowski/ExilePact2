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
"""
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
