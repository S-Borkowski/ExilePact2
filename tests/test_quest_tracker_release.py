"""tools/quest_tracker/release.py: check, notes and cut, on synthetic files.

Every case builds a minimal copy of the files that carry a version and of the
guide's "Release notes" in a temporary folder; `cut` runs against a git
repository there whose `origin` is a bare repository beside it, so nothing
touches this checkout or the network.

The baseline is a consistent tree at 1.2.0 that `check` accepts and `cut`
tags. The targets are the refusals: a file that disagrees (named), a version
with no or empty notes, uncommitted changes, a HEAD behind the remote's
branch, and a tag that already exists here or on the remote. The real tree
is checked too, so a version bump that misses a file fails here first.
"""
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from tools.quest_tracker import release  # noqa: E402

GUIDE = """# Quest tracker

Intro.

## Release notes

### 1.2.0

- First item, wrapped
  onto a second line.
- Second item.

  A paragraph after the list,
  wrapped too.

### 1.1.0 (older)

- Old item.

## After the notes
"""


def write_tree(root: Path, version: str = "1.2.0", guide: str = GUIDE, lag: dict | None = None) -> None:
    """A minimal app tree at `version`; `lag` overrides one file's version."""
    lag = lag or {}
    app = root / "apps" / "quest-tracker"
    (app / "src-tauri").mkdir(parents=True)
    (root / "docs" / "quest-tracker").mkdir(parents=True)

    def v(key: str) -> str:
        return lag.get(key, version)

    (app / "package.json").write_text(json.dumps({"name": "ui", "version": v("package")}), encoding="utf-8")
    (app / "package-lock.json").write_text(json.dumps(
        {"name": "ui", "version": v("lock"), "packages": {"": {"version": v("lock-root")}}}), encoding="utf-8")
    (app / "src-tauri" / "tauri.conf.json").write_text(json.dumps({"version": v("tauri")}), encoding="utf-8")
    (app / "Cargo.toml").write_text('[workspace.package]\nversion = "%s"\n' % v("cargo"), encoding="utf-8")
    (app / "Cargo.lock").write_text(
        'version = 4\n\n[[package]]\nname = "qt-engine"\nversion = "%s"\n\n'
        '[[package]]\nname = "serde"\nversion = "9.9.9"\nsource = "registry+https://example.invalid"\n'
        % v("lock-member"), encoding="utf-8")
    (root / "docs" / "quest-tracker" / "README.md").write_text(guide, encoding="utf-8")


def git(cwd: Path, *args: str) -> str:
    return subprocess.run(["git", *args], cwd=cwd, check=True, capture_output=True, text=True).stdout.strip()


class Check(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)

    def tearDown(self):
        self.tmp.cleanup()

    def test_a_consistent_tree_passes(self):
        write_tree(self.root)
        self.assertEqual(release.check(None, self.root), "1.2.0")
        self.assertEqual(release.check("1.2.0", self.root), "1.2.0")

    def test_a_lagging_file_is_named(self):
        for key, where in (("package", "package.json"), ("lock-root", 'packages[""]'),
                           ("tauri", "tauri.conf.json"), ("cargo", "Cargo.toml"),
                           ("lock-member", "Cargo.lock qt-engine")):
            with self.subTest(key=key), tempfile.TemporaryDirectory() as d:
                write_tree(Path(d), lag={key: "1.1.0"})
                with self.assertRaises(release.ReleaseError) as e:
                    release.check("1.2.0", Path(d))
                self.assertIn(where, str(e.exception))

    def test_a_registry_package_is_not_a_member(self):
        write_tree(self.root)
        self.assertNotIn("Cargo.lock serde", release.versions(self.root))

    def test_a_version_the_files_do_not_carry_fails(self):
        write_tree(self.root)
        with self.assertRaises(release.ReleaseError):
            release.check("1.3.0", self.root)

    def test_missing_or_empty_notes_fail(self):
        write_tree(self.root, version="1.3.0")
        with self.assertRaises(release.ReleaseError) as e:
            release.check(None, self.root)
        self.assertIn("no release notes for 1.3.0", str(e.exception))
        empty = GUIDE.replace("### 1.2.0\n\n- First item, wrapped\n  onto a second line.\n- Second item.\n\n"
                              "  A paragraph after the list,\n  wrapped too.\n", "### 1.2.0\n\n")
        with tempfile.TemporaryDirectory() as d:
            write_tree(Path(d), guide=empty)
            with self.assertRaises(release.ReleaseError) as e:
                release.check(None, Path(d))
            self.assertIn("empty", str(e.exception))

    def test_the_real_tree_is_consistent(self):
        version = release.check()
        self.assertTrue(release.notes(version).startswith("Quest Tracker " + version))


class Notes(unittest.TestCase):
    def test_the_section_stops_at_the_next_version(self):
        with tempfile.TemporaryDirectory() as d:
            write_tree(Path(d))
            text = release.changelog("1.2.0", Path(d))
            older = release.changelog("1.1.0", Path(d))
        self.assertNotIn("Old item", text)
        self.assertEqual(older, "- Old item.")

    def test_items_and_paragraphs_are_unwrapped(self):
        with tempfile.TemporaryDirectory() as d:
            write_tree(Path(d))
            body = release.notes("1.2.0", Path(d))
        self.assertIn("\n- First item, wrapped onto a second line.\n- Second item.\n\n"
                      "  A paragraph after the list, wrapped too.\n", body)
        self.assertIn("/blob/quest-tracker-v1.2.0/docs/quest-tracker/README.md", body)


class Cut(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        base = Path(self.tmp.name)
        self.remote = base / "remote.git"
        self.root = base / "work"
        git(base, "init", "--quiet", "--bare", str(self.remote))
        git(base, "init", "--quiet", "-b", "main", str(self.root))
        git(self.root, "config", "user.email", "test@example.invalid")
        git(self.root, "config", "user.name", "test")
        git(self.root, "config", "core.autocrlf", "false")
        write_tree(self.root)
        git(self.root, "add", "-A")
        git(self.root, "commit", "--quiet", "-m", "tree")
        git(self.root, "remote", "add", "origin", str(self.remote))
        git(self.root, "push", "--quiet", "origin", "main")

    def tearDown(self):
        self.tmp.cleanup()

    def remote_tags(self) -> str:
        return git(self.root, "ls-remote", "--tags", "origin")

    def test_cut_tags_and_pushes_only_when_asked(self):
        self.assertEqual(release.cut(root=self.root), "quest-tracker-v1.2.0")
        self.assertEqual(git(self.root, "cat-file", "-t", "quest-tracker-v1.2.0"), "tag")
        self.assertEqual(self.remote_tags(), "")
        git(self.root, "push", "--quiet", "origin", "refs/tags/quest-tracker-v1.2.0")
        git(self.root, "tag", "-d", "quest-tracker-v1.2.0")
        with self.assertRaises(release.ReleaseError) as e:
            release.cut(root=self.root)
        self.assertIn("already exists on origin", str(e.exception))

    def test_cut_with_push_reaches_the_remote(self):
        release.cut(push=True, root=self.root)
        self.assertIn("refs/tags/quest-tracker-v1.2.0", self.remote_tags())

    def test_a_local_tag_is_refused(self):
        git(self.root, "tag", "quest-tracker-v1.2.0")
        with self.assertRaises(release.ReleaseError) as e:
            release.cut(root=self.root)
        self.assertIn("already exists here", str(e.exception))

    def test_uncommitted_changes_are_refused(self):
        (self.root / "docs" / "quest-tracker" / "README.md").write_text(GUIDE + "\nmore\n", encoding="utf-8")
        with self.assertRaises(release.ReleaseError) as e:
            release.cut(root=self.root)
        self.assertIn("uncommitted", str(e.exception))
        self.assertEqual(git(self.root, "tag", "--list"), "")

    def test_a_head_off_the_remote_tip_is_refused(self):
        (self.root / "extra.txt").write_text("x\n", encoding="utf-8")
        git(self.root, "add", "extra.txt")
        git(self.root, "commit", "--quiet", "-m", "unpushed")
        with self.assertRaises(release.ReleaseError) as e:
            release.cut(root=self.root)
        self.assertIn("is not the tip of origin/main", str(e.exception))
        self.assertEqual(git(self.root, "tag", "--list"), "")

    def test_an_inconsistent_tree_is_refused_before_git(self):
        (self.root / "apps" / "quest-tracker" / "package.json").write_text(
            json.dumps({"name": "ui", "version": "1.3.0"}), encoding="utf-8")
        with self.assertRaises(release.ReleaseError) as e:
            release.cut(root=self.root)
        self.assertIn("version mismatch", str(e.exception))


if __name__ == "__main__":
    unittest.main()
