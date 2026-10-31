"""tools/disk_hygiene.py (issue #38): the report, the worktree cleanup, the
target trim and the free-space check, each on a temporary git repository.

Nothing here touches this checkout. Every fixture is a fresh repository in a
temporary directory: a main checkout with one commit, worktrees made with
`git worktree add`, and the merge target a local branch passed with `--into`.
Each "kept" case has a control beside it -- the same worktree with the
blocking condition taken away is acted on -- so a tool that kept everything
could not pass. Build output is a few small files under `apps/<app>/target/`,
ignored by the fixture's `.gitignore`, measured against a cap of a few
kilobytes given in GiB.
"""

import contextlib
import io
import os
import shutil
import stat
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TOOLS = ROOT / "tools"
sys.path.insert(0, str(TOOLS))

import disk_hygiene  # noqa: E402
import workorder_lock  # noqa: E402

GIB = 1024 ** 3


def git(repo, *args, check=True):
    return subprocess.run(["git", "-C", str(repo), *args], check=check, capture_output=True, text=True)


def _force_remove(func, path, _exc):
    # git marks its object files read-only, which Windows refuses to delete.
    os.chmod(path, stat.S_IWRITE)
    func(path)


def write(path, size=10):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(b"x" * size)


def listed(repo):
    """The normalised paths `git worktree list` registers."""
    out = git(repo, "worktree", "list", "--porcelain").stdout
    return {os.path.normcase(os.path.normpath(line[len("worktree "):]))
            for line in out.splitlines() if line.startswith("worktree ")}


def norm(path):
    return os.path.normcase(os.path.normpath(str(path)))


class Fixture(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix="disk-hygiene-"))
        self.addCleanup(shutil.rmtree, self.tmp, onexc=_force_remove)
        self.main = self.tmp / "main"
        self.main.mkdir()
        git(self.main, "init", "-q", "-b", "main")
        git(self.main, "config", "user.name", "test")
        git(self.main, "config", "user.email", "test@example.invalid")
        git(self.main, "config", "core.autocrlf", "false")
        (self.main / ".gitignore").write_text("apps/*/target/\napps/*/node_modules/\n", encoding="utf-8")
        (self.main / "README.md").write_text("fixture\n", encoding="utf-8")
        git(self.main, "add", "-A")
        git(self.main, "commit", "-q", "-m", "init")

    def worktree(self, name, parent=None):
        path = (parent or self.tmp) / name
        git(self.main, "worktree", "add", "-q", str(path), "-b", name)
        return path

    def run_tool(self, *argv, repo=None):
        out = io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(out):
            try:
                rc = disk_hygiene.main(["--repo", str(repo or self.main), *argv])
            except SystemExit as exc:
                rc = exc.code
        return rc, out.getvalue()

    def actions(self, *argv, repo=None):
        """{normalised path: action word} from a `worktrees` run."""
        rc, out = self.run_tool("worktrees", "--into", "main", *argv, repo=repo)
        self.assertIn(rc, (0, 1), out)
        found = {}
        for line in out.splitlines():
            word, _, rest = line.partition(" ")
            if word in ("keep", "remove", "prune", "prune-locked", "unregistered"):
                path = rest.split(" (")[0].split(": ")[0]
                found[norm(path)] = word
        return found, rc, out


class WorktreeTests(Fixture):
    def test_git_prune_alone_keeps_a_locked_missing_entry(self):
        # Baseline: what the owner met on 2026-10-31. A locked worktree whose
        # folder is gone survives `git worktree prune`.
        gone = self.worktree("gone")
        git(self.main, "worktree", "lock", str(gone))
        shutil.rmtree(gone, onexc=_force_remove)
        git(self.main, "worktree", "prune")
        self.assertIn(norm(gone), listed(self.main))
        # Control: the same entry unlocked is pruned.
        git(self.main, "worktree", "unlock", str(gone))
        git(self.main, "worktree", "prune")
        self.assertNotIn(norm(gone), listed(self.main))

    def test_dry_run_is_the_default_and_changes_nothing(self):
        merged = self.worktree("merged")
        missing = self.worktree("missing")
        shutil.rmtree(missing, onexc=_force_remove)
        before = git(self.main, "worktree", "list", "--porcelain").stdout
        found, rc, out = self.actions()
        self.assertEqual(rc, 0, out)
        self.assertEqual(found[norm(merged)], "remove", out)
        self.assertEqual(found[norm(missing)], "prune", out)
        self.assertIn("--apply", out)
        self.assertEqual(git(self.main, "worktree", "list", "--porcelain").stdout, before)
        self.assertTrue(merged.is_dir())

    def test_merged_clean_worktree_is_removed_on_apply(self):
        merged = self.worktree("merged")
        found, rc, out = self.actions("--apply")
        self.assertEqual(rc, 0, out)
        self.assertEqual(found[norm(merged)], "remove", out)
        self.assertFalse(merged.exists(), out)
        self.assertNotIn(norm(merged), listed(self.main))
        # Branches are never deleted.
        self.assertEqual(git(self.main, "branch", "--list", "merged").stdout.strip(), "merged")

    def test_worktree_with_modified_file_is_kept(self):
        wt = self.worktree("modified")
        (wt / "README.md").write_text("changed\n", encoding="utf-8")
        found, rc, out = self.actions("--apply")
        self.assertEqual(found[norm(wt)], "keep", out)
        self.assertIn("modified", out)
        self.assertTrue(wt.is_dir())
        # Control: the same worktree with the change undone is removed.
        git(wt, "checkout", "--", "README.md")
        found, rc, out = self.actions("--apply")
        self.assertEqual(found[norm(wt)], "remove", out)
        self.assertFalse(wt.exists(), out)

    def test_worktree_with_untracked_file_is_kept(self):
        wt = self.worktree("untracked")
        write(wt / "notes.txt")
        found, rc, out = self.actions("--apply")
        self.assertEqual(found[norm(wt)], "keep", out)
        self.assertTrue((wt / "notes.txt").is_file())
        # Control.
        (wt / "notes.txt").unlink()
        found, rc, out = self.actions("--apply")
        self.assertEqual(found[norm(wt)], "remove", out)
        self.assertFalse(wt.exists(), out)

    def test_ignored_build_output_does_not_block_removal(self):
        wt = self.worktree("built")
        write(wt / "apps" / "app" / "target" / "debug" / "app.exe", 2000)
        write(wt / "apps" / "app" / "node_modules" / "pkg" / "index.js", 100)
        self.assertEqual(git(wt, "status", "--porcelain").stdout, "")
        found, rc, out = self.actions("--apply")
        self.assertEqual(rc, 0, out)
        self.assertEqual(found[norm(wt)], "remove", out)
        self.assertFalse(wt.exists(), out)

    def test_unmerged_worktree_is_kept(self):
        wt = self.worktree("feature")
        (wt / "feature.txt").write_text("work\n", encoding="utf-8")
        git(wt, "add", "feature.txt")
        git(wt, "commit", "-q", "-m", "work")
        found, rc, out = self.actions("--apply")
        self.assertEqual(found[norm(wt)], "keep", out)
        self.assertIn("not merged", out)
        self.assertTrue(wt.is_dir())
        # Control: once main has the commit, the worktree goes.
        git(self.main, "merge", "-q", "--ff-only", "feature")
        found, rc, out = self.actions("--apply")
        self.assertEqual(found[norm(wt)], "remove", out)
        self.assertFalse(wt.exists(), out)

    def test_locked_worktree_is_kept(self):
        wt = self.worktree("locked")
        git(self.main, "worktree", "lock", str(wt))
        found, rc, out = self.actions("--apply")
        self.assertEqual(found[norm(wt)], "keep", out)
        self.assertIn("locked", out)
        self.assertTrue(wt.is_dir())
        # Control.
        git(self.main, "worktree", "unlock", str(wt))
        found, rc, out = self.actions("--apply")
        self.assertEqual(found[norm(wt)], "remove", out)
        self.assertFalse(wt.exists(), out)

    def test_locked_worktree_with_missing_folder_is_pruned(self):
        gone = self.worktree("gone")
        git(self.main, "worktree", "lock", str(gone))
        shutil.rmtree(gone, onexc=_force_remove)
        found, rc, out = self.actions()
        self.assertEqual(found[norm(gone)], "prune-locked", out)
        self.assertIn(norm(gone), listed(self.main))
        found, rc, out = self.actions("--apply")
        self.assertEqual(rc, 0, out)
        self.assertEqual(found[norm(gone)], "prune-locked", out)
        self.assertNotIn(norm(gone), listed(self.main))

    def test_locked_worktree_on_missing_parent_is_kept(self):
        drive = self.tmp / "unplugged"
        wt = self.worktree("away", parent=drive)
        git(self.main, "worktree", "lock", str(wt))
        shutil.rmtree(drive, onexc=_force_remove)
        found, rc, out = self.actions("--apply")
        self.assertEqual(found[norm(wt)], "keep", out)
        self.assertIn(norm(wt), listed(self.main))
        # Control: with the parent back (the drive mounted) and the folder
        # still gone, the entry is stale and is pruned.
        drive.mkdir()
        found, rc, out = self.actions("--apply")
        self.assertEqual(found[norm(wt)], "prune-locked", out)
        self.assertNotIn(norm(wt), listed(self.main))

    def test_missing_folder_is_pruned(self):
        gone = self.worktree("gone")
        shutil.rmtree(gone, onexc=_force_remove)
        found, rc, out = self.actions("--apply")
        self.assertEqual(rc, 0, out)
        self.assertEqual(found[norm(gone)], "prune", out)
        self.assertNotIn(norm(gone), listed(self.main))

    def test_main_and_current_checkout_are_kept(self):
        wt = self.worktree("merged")
        found, rc, out = self.actions("--apply", repo=wt)
        self.assertEqual(found[norm(self.main)], "keep", out)
        self.assertEqual(found[norm(wt)], "keep", out)
        self.assertIn("main worktree", out)
        self.assertIn("current checkout", out)
        self.assertTrue(wt.is_dir())
        self.assertTrue(self.main.is_dir())
        # Control: run from main, the same worktree is removed.
        found, rc, out = self.actions("--apply")
        self.assertEqual(found[norm(self.main)], "keep", out)
        self.assertEqual(found[norm(wt)], "remove", out)
        self.assertFalse(wt.exists(), out)

    def test_unregistered_folder_is_reported_not_deleted(self):
        orphan = self.main / ".claude" / "worktrees" / "orphan"
        write(orphan / "apps" / "app" / "src.txt", 500)
        registered = self.worktree("kept-one", parent=self.main / ".claude" / "worktrees")
        found, rc, out = self.actions("--apply")
        self.assertEqual(found[norm(orphan)], "unregistered", out)
        self.assertTrue((orphan / "apps" / "app" / "src.txt").is_file())
        # Control: a registered folder in the same place is not "unregistered".
        self.assertNotEqual(found.get(norm(registered)), "unregistered", out)

    def test_unknown_into_ref_exits_2(self):
        wt = self.worktree("merged")
        r = subprocess.run([sys.executable, str(TOOLS / "disk_hygiene.py"), "--repo", str(self.main),
                            "worktrees", "--into", "no-such-ref", "--apply"], capture_output=True, text=True)
        self.assertEqual(r.returncode, 2, r.stdout + r.stderr)
        self.assertIn("no-such-ref", r.stderr)
        self.assertTrue(wt.is_dir())
        # Not a repository is a usage error too.
        r = subprocess.run([sys.executable, str(TOOLS / "disk_hygiene.py"), "--repo", str(self.tmp),
                            "worktrees"], capture_output=True, text=True)
        self.assertEqual(r.returncode, 2, r.stdout + r.stderr)
        # Control: a ref that exists works.
        r = subprocess.run([sys.executable, str(TOOLS / "disk_hygiene.py"), "--repo", str(self.main),
                            "worktrees", "--into", "main"], capture_output=True, text=True)
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)


class TrimTests(Fixture):
    def build_output(self, incremental=4000, deps=1000):
        target = self.main / "apps" / "app" / "target"
        write(target / "debug" / "incremental" / "app-1" / "s-1" / "dep-graph.bin", incremental)
        write(target / "debug" / "deps" / "libapp.rlib", deps)
        return target

    def trim(self, cap_bytes, *argv):
        return self.run_tool("trim", "--cap-gib", repr(cap_bytes / GIB), *argv)

    def test_trim_under_cap_changes_nothing(self):
        target = self.build_output()
        rc, out = self.trim(10000, "--apply")
        self.assertEqual(rc, 0, out)
        self.assertTrue((target / "debug" / "incremental").is_dir(), out)
        self.assertTrue((target / "debug" / "deps" / "libapp.rlib").is_file(), out)
        # Control: the same target with a cap below its size is trimmed.
        rc, out = self.trim(2000, "--apply")
        self.assertFalse((target / "debug" / "incremental").exists(), out)

    def test_trim_drops_incremental_first(self):
        target = self.build_output()
        rc, out = self.trim(2000)
        self.assertEqual(rc, 0, out)
        self.assertTrue((target / "debug" / "incremental").is_dir(), "dry run deleted: " + out)
        rc, out = self.trim(2000, "--apply")
        self.assertEqual(rc, 0, out)
        self.assertFalse((target / "debug" / "incremental").exists(), out)
        self.assertTrue((target / "debug" / "deps" / "libapp.rlib").is_file(), out)

    def test_trim_drops_target_still_over_cap(self):
        target = self.build_output()
        rc, out = self.trim(500)
        self.assertTrue(target.is_dir(), "dry run deleted: " + out)
        rc, out = self.trim(500, "--apply")
        self.assertEqual(rc, 0, out)
        self.assertFalse(target.exists(), out)
        self.assertTrue((self.main / "apps" / "app").is_dir())

    def test_trim_skips_a_checkout_whose_build_lock_is_held(self):
        target = self.build_output()
        argv = [sys.executable, str(TOOLS / "disk_hygiene.py"), "--repo", str(self.main),
                "trim", "--cap-gib", repr(500 / GIB), "--apply"]
        with workorder_lock.held("build", workorder_lock.lock_dir(self.main), timeout=0) as name:
            self.assertEqual(name, "build")
            r = subprocess.run(argv, capture_output=True, text=True)
        self.assertIn("busy", r.stdout, r.stdout + r.stderr)
        self.assertTrue((target / "debug" / "incremental").is_dir(), r.stdout)
        # Control: with the lock free, the same run trims.
        r = subprocess.run(argv, capture_output=True, text=True)
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        self.assertNotIn("busy", r.stdout)
        self.assertFalse(target.exists(), r.stdout)

    @unittest.skipUnless(os.name == "nt", "directory junctions exist only on Windows (_winapi.CreateJunction)")
    def test_trim_never_follows_a_junction(self):
        import _winapi
        outside = self.tmp / "elsewhere"
        write(outside / "big.bin", 8000)
        target = self.main / "apps" / "app" / "target"
        write(target / "debug" / "deps" / "libapp.rlib", 100)
        _winapi.CreateJunction(str(outside), str(target / "debug" / "linked"))
        self.assertTrue((target / "debug" / "linked" / "big.bin").is_file())
        # Sizing does not follow it: 100 bytes is under a 1000-byte cap.
        rc, out = self.trim(1000, "--apply")
        self.assertEqual(rc, 0, out)
        self.assertTrue((target / "debug" / "deps" / "libapp.rlib").is_file(), out)
        # Deleting does not follow it: the target goes, what it pointed at stays.
        rc, out = self.trim(50, "--apply")
        self.assertEqual(rc, 0, out)
        self.assertFalse(target.exists(), out)
        self.assertTrue((outside / "big.bin").is_file(), out)
        self.assertEqual((outside / "big.bin").stat().st_size, 8000)


class CheckAndReportTests(Fixture):
    def test_check_exit_code_follows_threshold(self):
        self.assertEqual(disk_hygiene.DEFAULT_MIN_FREE_GIB, 30)
        free = disk_hygiene.free_gib(self.main)
        self.assertGreater(free, 0)
        rc, out = self.run_tool("check", "--min-free-gib", "0")
        self.assertEqual(rc, 0, out)
        rc, out = self.run_tool("check", "--min-free-gib", "1000000")
        self.assertEqual(rc, 1, out)
        for word in ("report", "worktrees", "trim"):
            self.assertIn(word, out)

    def test_report_lists_sizes_and_free_space(self):
        target = self.main / "apps" / "app" / "target"
        write(target / "debug" / "app.exe", 3000)
        write(self.main / "apps" / "app" / "node_modules" / "pkg" / "index.js", 700)
        wt = self.worktree("side")
        orphan = self.main / ".claude" / "worktrees" / "orphan"
        write(orphan / "a.txt", 300)
        rc, out = self.run_tool("report", "--into", "main")
        self.assertEqual(rc, 0, out)
        lines = out.splitlines()
        self.assertIn("main", lines[0])
        self.assertIn(norm(self.main), norm(lines[0]))
        self.assertIn(norm(wt), norm(out))
        self.assertIn("merged", out)
        target_line = next(l for l in lines if "target" in l)
        self.assertIn(disk_hygiene.human(3000), target_line)
        nm_line = next(l for l in lines if "node_modules" in l)
        self.assertIn(disk_hygiene.human(700), nm_line)
        self.assertIn("unregistered", out)
        self.assertIn(disk_hygiene.human(300), next(l for l in lines if "unregistered" in l))
        self.assertIn("GiB free", out)
        # Nothing was changed by the report.
        self.assertTrue((target / "debug" / "app.exe").is_file())
        self.assertTrue(orphan.is_dir())


if __name__ == "__main__":
    unittest.main()
