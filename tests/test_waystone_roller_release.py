"""tools/waystone_roller/release.py and its workflow: check, notes, cut, on synthetic files.

Every case builds a minimal copy of the files that carry the waystone
roller's version and of its guide's "Release notes" in a temporary folder;
`cut` runs against a git repository there whose `origin` is a bare
repository beside it, so nothing touches this checkout or the network.

The baseline is a consistent tree at 1.2.0 that `check` accepts and `cut`
tags `waystone-roller-v1.2.0`. The targets are the refusals: a file that
disagrees (named), a version with no or empty notes, uncommitted changes, a
HEAD behind the remote's branch, and a tag that already exists here or on
the remote. The real tree is checked too, so a version bump that misses a
file fails here first. `Workflow` reads
`.github/workflows/waystone-roller-release.yml` as text (no YAML parser is
installed) and checks the properties the release path relies on, with a
mutated copy of the file as each check's negative control.
"""
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from tools.waystone_roller import release  # noqa: E402

WORKFLOW = ROOT / ".github" / "workflows" / "waystone-roller-release.yml"

GUIDE = """# Waystone Roller

Intro.

## Release notes

### 1.2.0

- First item, wrapped
  onto a second line.
- Second item.

  A paragraph after the list,
  wrapped too.

### 1.2.0-rc1

- A pre-release whose heading starts with the same text.

### 1.1.0 (older)

- Old item.

## After the notes
"""

MEMBERS = ("wr-regex", "wr-app", "waystone-roller")


def write_tree(root: Path, version: str = "1.2.0", guide: str = GUIDE, lag: dict | None = None) -> None:
    """A minimal app tree at `version`; `lag` overrides one file's version."""
    lag = lag or {}
    app = root / "apps" / "waystone-roller"
    (app / "src-tauri").mkdir(parents=True)
    (root / "docs" / "waystone-roller").mkdir(parents=True)

    def v(key: str) -> str:
        return lag.get(key, version)

    (app / "package.json").write_text(json.dumps({"name": "waystone-roller-ui", "version": v("package")}),
                                      encoding="utf-8")
    (app / "package-lock.json").write_text(json.dumps(
        {"name": "waystone-roller-ui", "version": v("lock"), "packages": {"": {"version": v("lock-root")}}}),
        encoding="utf-8")
    (app / "src-tauri" / "tauri.conf.json").write_text(json.dumps({"version": v("tauri")}), encoding="utf-8")
    (app / "Cargo.toml").write_text('[workspace.package]\nversion = "%s"\n' % v("cargo"), encoding="utf-8")
    lock = "version = 4\n\n"
    for name in MEMBERS:
        lock += '[[package]]\nname = "%s"\nversion = "%s"\n\n' % (name, v("lock-" + name))
    lock += '[[package]]\nname = "serde"\nversion = "9.9.9"\nsource = "registry+https://example.invalid"\n'
    (app / "Cargo.lock").write_text(lock, encoding="utf-8")
    (root / "docs" / "waystone-roller" / "README.md").write_text(guide, encoding="utf-8")


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

    def test_every_member_of_the_workspace_is_read(self):
        write_tree(self.root)
        found = release.versions(self.root)
        for name in MEMBERS:
            self.assertEqual(found["Cargo.lock " + name], "1.2.0")

    def test_a_lagging_file_is_named(self):
        cases = [("package", "package.json"), ("lock", "package-lock.json is"),
                 ("lock-root", 'packages[""]'), ("tauri", "tauri.conf.json"), ("cargo", "Cargo.toml")]
        cases += [("lock-" + name, "Cargo.lock " + name) for name in MEMBERS]
        for key, where in cases:
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
        self.assertIn("docs/waystone-roller/README.md", str(e.exception))
        empty = GUIDE.replace("### 1.2.0\n\n- First item, wrapped\n  onto a second line.\n- Second item.\n\n"
                              "  A paragraph after the list,\n  wrapped too.\n", "### 1.2.0\n\n")
        with tempfile.TemporaryDirectory() as d:
            write_tree(Path(d), guide=empty)
            with self.assertRaises(release.ReleaseError) as e:
                release.check(None, Path(d))
            self.assertIn("empty", str(e.exception))

    def test_a_guide_without_release_notes_fails(self):
        write_tree(self.root, guide=GUIDE.replace("## Release notes", "## Notes"))
        with self.assertRaises(release.ReleaseError) as e:
            release.check(None, self.root)
        self.assertIn('no "## Release notes" section', str(e.exception))

    def test_the_real_tree_is_consistent(self):
        version = release.check()
        self.assertTrue(release.notes(version).startswith("Waystone Roller " + version))


class Notes(unittest.TestCase):
    def test_the_section_stops_at_the_next_version(self):
        with tempfile.TemporaryDirectory() as d:
            write_tree(Path(d))
            text = release.changelog("1.2.0", Path(d))
            older = release.changelog("1.1.0", Path(d))
        self.assertNotIn("Old item", text)
        self.assertNotIn("pre-release", text)
        self.assertEqual(older, "- Old item.")

    def test_a_heading_matches_the_version_then_space_or_end_only(self):
        # Negative control for the heading rule: "1.2.0" must not pick up
        # "### 1.2.0-rc1", and "1.2" names no section at all.
        with tempfile.TemporaryDirectory() as d:
            write_tree(Path(d))
            self.assertIn("pre-release", release.changelog("1.2.0-rc1", Path(d)))
            with self.assertRaises(release.ReleaseError):
                release.changelog("1.2", Path(d))

    def test_items_and_paragraphs_are_unwrapped(self):
        with tempfile.TemporaryDirectory() as d:
            write_tree(Path(d))
            body = release.notes("1.2.0", Path(d))
        self.assertIn("\n- First item, wrapped onto a second line.\n- Second item.\n\n"
                      "  A paragraph after the list, wrapped too.\n", body)

    def test_the_header_names_the_app_its_file_and_the_guide_at_the_tag(self):
        with tempfile.TemporaryDirectory() as d:
            write_tree(Path(d))
            body = release.notes("1.2.0", Path(d))
        self.assertTrue(body.startswith("Waystone Roller 1.2.0"))
        for needed in ("`waystone-roller.exe`", "`SHA256SUMS.txt`", "WebView2", "64-bit", "English",
                       "/blob/waystone-roller-v1.2.0/docs/waystone-roller/README.md"):
            self.assertIn(needed, body)
        # Negative control: nothing of the quest tracker's header leaks in.
        self.assertNotIn("quest-tracker", body)
        self.assertNotIn("Quest Tracker", body)


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
        self.assertEqual(release.cut(root=self.root), "waystone-roller-v1.2.0")
        self.assertEqual(git(self.root, "cat-file", "-t", "waystone-roller-v1.2.0"), "tag")
        self.assertEqual(git(self.root, "tag", "-l", "--format=%(contents:subject)", "waystone-roller-v1.2.0"),
                         "Waystone Roller 1.2.0")
        self.assertEqual(self.remote_tags(), "")
        git(self.root, "push", "--quiet", "origin", "refs/tags/waystone-roller-v1.2.0")
        git(self.root, "tag", "-d", "waystone-roller-v1.2.0")
        with self.assertRaises(release.ReleaseError) as e:
            release.cut(root=self.root)
        self.assertIn("already exists on origin", str(e.exception))

    def test_cut_with_push_reaches_the_remote(self):
        release.cut(push=True, root=self.root)
        self.assertIn("refs/tags/waystone-roller-v1.2.0", self.remote_tags())

    def test_another_apps_tag_does_not_block_the_cut(self):
        # Negative control for the tag checks: the quest tracker's tag at the
        # same version is a different tag.
        git(self.root, "tag", "quest-tracker-v1.2.0")
        git(self.root, "push", "--quiet", "origin", "refs/tags/quest-tracker-v1.2.0")
        self.assertEqual(release.cut(root=self.root), "waystone-roller-v1.2.0")

    def test_a_local_tag_is_refused(self):
        git(self.root, "tag", "waystone-roller-v1.2.0")
        with self.assertRaises(release.ReleaseError) as e:
            release.cut(root=self.root)
        self.assertIn("already exists here", str(e.exception))

    def test_uncommitted_changes_are_refused(self):
        (self.root / "docs" / "waystone-roller" / "README.md").write_text(GUIDE + "\nmore\n", encoding="utf-8")
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
        (self.root / "apps" / "waystone-roller" / "package.json").write_text(
            json.dumps({"name": "waystone-roller-ui", "version": "1.3.0"}), encoding="utf-8")
        with self.assertRaises(release.ReleaseError) as e:
            release.cut(root=self.root)
        self.assertIn("version mismatch", str(e.exception))


def workflow_problems(text: str) -> list[str]:
    """Each property of the release workflow that `text` lacks, by name."""
    lines = [line.strip() for line in text.splitlines()]
    needs = {
        "tag trigger": '- "waystone-roller-v*"' in lines,
        "manual run with a tag input": "workflow_dispatch:" in lines and "tag:" in lines,
        "contents: write": "contents: write" in lines,
        "concurrency group": "group: waystone-roller-release-${{ github.event.inputs.tag || github.ref_name }}" in lines,
        "repository guard": "if: github.repository == 'S-Borkowski/ExilePact2'" in lines,
        "windows runner": "runs-on: windows-latest" in lines,
        "checks the tag": 'python -m tools.waystone_roller.release check "$version"' in lines,
        "builds the app": "npm --prefix apps/waystone-roller run release" in lines,
        "takes the exe": any("apps/waystone-roller/target/release/waystone-roller.exe" in line for line in lines),
        "hashes it": any(line.startswith("(cd dist-release && sha256sum waystone-roller.exe > SHA256SUMS.txt")
                         for line in lines),
        "notes from the guide": any(line.startswith("python -m tools.waystone_roller.release notes")
                                    for line in lines),
        "a draft release": any("gh release create" in line for line in lines) and "--draft" in text,
        "its title": any('--title "Waystone Roller $VERSION"' in line for line in lines),
        "attaches the exe and the sums": "assets=(dist-release/waystone-roller.exe dist-release/SHA256SUMS.txt)"
                                         in lines,
        "npm cache of this app": "cache-dependency-path: apps/waystone-roller/package-lock.json" in lines,
        "rust cache of this app": "workspaces: apps/waystone-roller" in lines,
        "nothing publishes": "--draft=false" not in text and "--latest" not in text
                             and "gh release edit" not in text,
        "nothing of the quest tracker": "quest-tracker" not in text and "quest_tracker" not in text,
    }
    return [name for name, ok in needs.items() if not ok]


class Workflow(unittest.TestCase):
    def setUp(self):
        self.text = WORKFLOW.read_text(encoding="utf-8")

    def test_the_workflow_has_every_property(self):
        self.assertEqual(workflow_problems(self.text), [])

    def test_each_property_is_caught_when_missing(self):
        mutations = {
            "tag trigger": ('- "waystone-roller-v*"', '- "v*"'),
            "contents: write": ("contents: write", "contents: read"),
            "repository guard": ("if: github.repository == 'S-Borkowski/ExilePact2'", "if: true"),
            "windows runner": ("runs-on: windows-latest", "runs-on: ubuntu-latest"),
            "a draft release": ("--draft --verify-tag", "--verify-tag"),
            "notes from the guide": ("python -m tools.waystone_roller.release notes",
                                     "python -m tools.waystone_roller.release changes"),
            "attaches the exe and the sums": ("dist-release/SHA256SUMS.txt)", ")"),
            "nothing of the quest tracker": ("apps/waystone-roller/package-lock.json",
                                             "apps/quest-tracker/package-lock.json"),
        }
        for name, (old, new) in mutations.items():
            with self.subTest(name=name):
                self.assertIn(old, self.text)
                self.assertIn(name, workflow_problems(self.text.replace(old, new, 1)))
        published = self.text + "\n      - run: gh release edit \"$TAG\" --draft=false\n"
        self.assertIn("nothing publishes", workflow_problems(published))


if __name__ == "__main__":
    unittest.main()
