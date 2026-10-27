"""tools/game_lease.py: the machine-wide lease x64dbg_mcp gates attaching on.

Every test points `GAME_LEASE_DIR` at a temp directory, so none touches the
real record under `%LOCALAPPDATA%\\ExilePact2`.
"""

import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))

from tools import game_lease as gl  # noqa: E402


class LeaseTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.env = mock.patch.dict(os.environ, {"GAME_LEASE_DIR": self.tmp.name})
        self.env.start()

    def tearDown(self):
        self.env.stop()
        self.tmp.cleanup()

    def test_no_record_is_free(self):
        self.assertIsNone(gl.read_record())
        self.assertEqual(gl.status()["state"], "free")

    def test_acquire_names_a_live_holder_and_release_frees_it(self):
        rc, record = gl.acquire("demo-live-1", os.getpid())
        self.assertEqual(rc, 0, record)
        self.assertEqual(gl.status()["state"], "held")
        self.assertEqual(gl.release()[0], 0)
        self.assertEqual(gl.status()["state"], "free")

    def test_a_held_lease_refuses_a_second_acquire_without_force(self):
        gl.acquire("first", os.getpid())
        rc, out = gl.acquire("second", os.getpid())
        self.assertEqual(rc, 1)
        self.assertIn("held", out["error"])
        self.assertEqual(gl.acquire("second", os.getpid(), force=True)[0], 0)

    def test_a_dead_holder_reads_as_stale(self):
        child = subprocess.Popen([sys.executable, "-c", "pass"])
        child.wait()
        gl._write({"schema": gl.SCHEMA, "state": gl.HELD, "lease_id": "x", "label": "gone",
                   "holder": {"pid": child.pid, "pid_start": 1}})
        self.assertEqual(gl.status()["state"], "stale")

    def test_an_unreadable_record_is_never_free(self):
        Path(self.tmp.name, gl.RECORD_NAME).write_text("{not json", encoding="utf-8")
        self.assertTrue(gl.read_record()["refused"])
        self.assertEqual(gl.status()["state"], "unavailable")
        self.assertEqual(gl.acquire("x", os.getpid())[0], 1)

    def test_a_bad_label_is_refused(self):
        self.assertEqual(gl.acquire("has space", os.getpid())[0], 2)


if __name__ == "__main__":
    unittest.main()
