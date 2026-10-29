"""tools/crafting_helper/release.py and its workflow: check, notes, cut, on synthetic files.

Every case in `Check`, `Notes`, `Cut` and `Workflow` builds a minimal copy of
the files that carry the crafting helper's version and of its guide's
"Release notes" in a temporary folder; `cut` runs against a git repository
there whose `origin` is a bare repository beside it, so nothing touches this
checkout or the network.

The baseline is a consistent tree at 1.2.0 that `check` accepts and `cut`
tags `crafting-helper-v1.2.0`. The targets are the refusals: a file that
disagrees (named), a version with no or empty notes, uncommitted changes, a
HEAD behind the remote's branch, and a tag that already exists here or on
the remote. The workspace lists its members as `["crates/*", "src-tauri"]`,
so the tool expands the glob the way cargo does: every directory it matches
is a member (`ch-item` too, which writes its version literally), a matched
directory with no `Cargo.toml` fails, a plain file is skipped, and a pattern
that matches nothing fails. A path package in `Cargo.lock` that is not a
member of the app's workspace (the shared `apps/shared/` crates) carries its
own version and is not read.

`Workflow` reads `.github/workflows/crafting-helper-release.yml` as text and
checks the properties the release path relies on, signing included, with a
mutated copy of the file as each check's negative control; the signing
checks are the waystone roller test's, applied to this app. The workflow
names neither sibling app.

`RealTree` checks the real checkout, so a version bump that misses a file
fails here first. It needs the app and its guide in place.
"""
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from tools.crafting_helper import release  # noqa: E402
# The helpers only, as a module: binding its TestCase classes here would run them twice.
from tests import test_waystone_roller_release as wr_test  # noqa: E402

WORKFLOW = ROOT / ".github" / "workflows" / "crafting-helper-release.yml"

GUIDE = """# Crafting Helper

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

# Member directory -> package name. `crates/*` matches the first two.
MEMBER_DIRS = {"crates/ch-app": "ch-app", "crates/ch-item": "ch-item", "src-tauri": "crafting-helper"}
MEMBERS = tuple(MEMBER_DIRS.values())
LITERAL = "crates/ch-item"  # writes `version = "..."`, not `version.workspace = true`


def write_tree(root: Path, version: str = "1.2.0", guide: str = GUIDE, lag: dict | None = None,
               extra_lock: str = "") -> None:
    """A minimal app tree at `version`; `lag` overrides one file's version.

    `extra_lock` is appended to `Cargo.lock` as written.
    """
    lag = lag or {}
    app = root / "apps" / "crafting-helper"
    (app / "src-tauri").mkdir(parents=True)
    (root / "docs" / "crafting-helper").mkdir(parents=True)

    def v(key: str) -> str:
        return lag.get(key, version)

    (app / "package.json").write_text(json.dumps({"name": "crafting-helper-ui", "version": v("package")}),
                                      encoding="utf-8")
    (app / "package-lock.json").write_text(json.dumps(
        {"name": "crafting-helper-ui", "version": v("lock"), "packages": {"": {"version": v("lock-root")}}}),
        encoding="utf-8")
    (app / "src-tauri" / "tauri.conf.json").write_text(json.dumps({"version": v("tauri")}), encoding="utf-8")
    (app / "Cargo.toml").write_text(
        '[workspace]\nmembers = ["crates/*", "src-tauri"]\n\n[workspace.package]\nversion = "%s"\n'
        % v("cargo"), encoding="utf-8")
    for member, name in MEMBER_DIRS.items():
        (app / member).mkdir(parents=True, exist_ok=True)
        own = 'version = "%s"' % v("literal") if member == LITERAL else "version.workspace = true"
        (app / member / "Cargo.toml").write_text('[package]\nname = "%s"\n%s\n' % (name, own), encoding="utf-8")
    lock = "version = 4\n\n"
    for name in MEMBERS:
        lock += '[[package]]\nname = "%s"\nversion = "%s"\n\n' % (name, v("lock-" + name))
    lock += '[[package]]\nname = "serde"\nversion = "9.9.9"\nsource = "registry+https://example.invalid"\n'
    (app / "Cargo.lock").write_text(lock + extra_lock, encoding="utf-8")
    (root / "docs" / "crafting-helper" / "README.md").write_text(guide, encoding="utf-8")


def git(cwd: Path, *args: str) -> str:
    return subprocess.run(["git", *args], cwd=cwd, check=True, capture_output=True, text=True).stdout.strip()


class Check(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.app = self.root / "apps" / "crafting-helper"

    def tearDown(self):
        self.tmp.cleanup()

    def test_a_consistent_tree_passes(self):
        write_tree(self.root)
        self.assertEqual(release.check(None, self.root), "1.2.0")
        self.assertEqual(release.check("1.2.0", self.root), "1.2.0")

    def test_the_member_glob_is_expanded(self):
        write_tree(self.root)
        self.assertEqual(release.members(self.app), ["crates/ch-app", "crates/ch-item", "src-tauri"])
        found = release.versions(self.root)
        for name in MEMBERS:
            self.assertEqual(found["Cargo.lock " + name], "1.2.0")
        self.assertEqual(found[LITERAL + "/Cargo.toml [package]"], "1.2.0")

    def test_a_member_that_inherits_its_version_is_read_from_the_lock_only(self):
        write_tree(self.root)
        found = release.versions(self.root)
        self.assertNotIn("crates/ch-app/Cargo.toml [package]", found)
        self.assertNotIn("src-tauri/Cargo.toml [package]", found)

    def test_a_lagging_file_is_named(self):
        cases = [("package", "package.json"), ("lock", "package-lock.json is"),
                 ("lock-root", 'packages[""]'), ("tauri", "tauri.conf.json"), ("cargo", "Cargo.toml [workspace"),
                 ("literal", LITERAL + "/Cargo.toml [package]")]
        cases += [("lock-" + name, "Cargo.lock " + name) for name in MEMBERS]
        for key, where in cases:
            with self.subTest(key=key), tempfile.TemporaryDirectory() as d:
                write_tree(Path(d), lag={key: "1.1.0"})
                with self.assertRaises(release.ReleaseError) as e:
                    release.check("1.2.0", Path(d))
                self.assertIn(where, str(e.exception))

    def test_a_glob_directory_without_a_manifest_is_named(self):
        # Cargo fails to load such a workspace; so does the check.
        write_tree(self.root)
        (self.app / "crates" / "ch-half").mkdir()
        with self.assertRaises(release.ReleaseError) as e:
            release.check("1.2.0", self.root)
        self.assertIn("crates/ch-half has no Cargo.toml", str(e.exception))

    def test_a_plain_file_the_glob_matches_is_skipped(self):
        # Negative control for the rule above: cargo ignores a file in crates/.
        write_tree(self.root)
        (self.app / "crates" / "notes.txt").write_text("x\n", encoding="utf-8")
        self.assertEqual(release.members(self.app), ["crates/ch-app", "crates/ch-item", "src-tauri"])
        self.assertEqual(release.check("1.2.0", self.root), "1.2.0")

    def test_a_pattern_that_matches_nothing_fails(self):
        write_tree(self.root)
        for member in ("crates/ch-app", "crates/ch-item"):
            for path in sorted((self.app / member).iterdir()):
                path.unlink()
            (self.app / member).rmdir()
        with self.assertRaises(release.ReleaseError) as e:
            release.members(self.app)
        self.assertIn("'crates/*' matches no directory", str(e.exception))

    def test_a_registry_package_is_not_a_member(self):
        write_tree(self.root)
        self.assertNotIn("Cargo.lock serde", release.versions(self.root))

    def test_a_shared_path_crate_is_not_a_member(self):
        # A path dependency on another workspace (apps/shared/) is in the lock
        # with no source, at that workspace's own version.
        write_tree(self.root, extra_lock='\n[[package]]\nname = "ep-update"\nversion = "0.9.0"\n')
        self.assertEqual(release.check("1.2.0", self.root), "1.2.0")
        found = release.versions(self.root)
        self.assertNotIn("Cargo.lock ep-update", found)
        for name in MEMBERS:
            self.assertEqual(found["Cargo.lock " + name], "1.2.0")

    def test_a_member_missing_from_the_lock_is_named(self):
        # Negative control for reading members from Cargo.toml: a member the
        # glob finds but the lock does not list fails rather than going unchecked.
        write_tree(self.root)
        lock = self.app / "Cargo.lock"
        lock.write_text(lock.read_text(encoding="utf-8").replace('name = "ch-item"', 'name = "other"'),
                        encoding="utf-8")
        with self.assertRaises(release.ReleaseError) as e:
            release.check("1.2.0", self.root)
        self.assertIn("ch-item", str(e.exception))

    def test_a_version_the_files_do_not_carry_fails(self):
        write_tree(self.root)
        with self.assertRaises(release.ReleaseError):
            release.check("1.3.0", self.root)

    def test_missing_or_empty_notes_fail(self):
        write_tree(self.root, version="1.3.0")
        with self.assertRaises(release.ReleaseError) as e:
            release.check(None, self.root)
        self.assertIn("no release notes for 1.3.0", str(e.exception))
        self.assertIn("docs/crafting-helper/README.md", str(e.exception))
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
        self.assertTrue(body.startswith("Crafting Helper 1.2.0"))
        for needed in ("`crafting-helper.exe`", "`SHA256SUMS.txt`", "`crafting-helper.exe.minisig`",
                       "`Ctrl+Alt+C`", "`Ctrl+Alt+X`", "WebView2", "64-bit", "English",
                       "/blob/crafting-helper-v1.2.0/docs/crafting-helper/README.md"):
            self.assertIn(needed, body)
        # Negative control: nothing of either sibling's header leaks in.
        for sibling in ("quest-tracker", "Quest Tracker", "waystone-roller", "Waystone Roller"):
            self.assertNotIn(sibling, body)


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
        self.assertEqual(release.cut(root=self.root), "crafting-helper-v1.2.0")
        self.assertEqual(git(self.root, "cat-file", "-t", "crafting-helper-v1.2.0"), "tag")
        self.assertEqual(git(self.root, "tag", "-l", "--format=%(contents:subject)", "crafting-helper-v1.2.0"),
                         "Crafting Helper 1.2.0")
        self.assertEqual(self.remote_tags(), "")
        git(self.root, "push", "--quiet", "origin", "refs/tags/crafting-helper-v1.2.0")
        git(self.root, "tag", "-d", "crafting-helper-v1.2.0")
        with self.assertRaises(release.ReleaseError) as e:
            release.cut(root=self.root)
        self.assertIn("already exists on origin", str(e.exception))

    def test_cut_with_push_reaches_the_remote(self):
        release.cut(push=True, root=self.root)
        self.assertIn("refs/tags/crafting-helper-v1.2.0", self.remote_tags())

    def test_another_apps_tag_does_not_block_the_cut(self):
        # Negative control for the tag checks: a sibling's tag at the same
        # version is a different tag.
        for tag in ("quest-tracker-v1.2.0", "waystone-roller-v1.2.0"):
            git(self.root, "tag", tag)
            git(self.root, "push", "--quiet", "origin", "refs/tags/" + tag)
        self.assertEqual(release.cut(root=self.root), "crafting-helper-v1.2.0")

    def test_a_local_tag_is_refused(self):
        git(self.root, "tag", "crafting-helper-v1.2.0")
        with self.assertRaises(release.ReleaseError) as e:
            release.cut(root=self.root)
        self.assertIn("already exists here", str(e.exception))

    def test_uncommitted_changes_are_refused(self):
        (self.root / "docs" / "crafting-helper" / "README.md").write_text(GUIDE + "\nmore\n", encoding="utf-8")
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
        (self.root / "apps" / "crafting-helper" / "package.json").write_text(
            json.dumps({"name": "crafting-helper-ui", "version": "1.3.0"}), encoding="utf-8")
        with self.assertRaises(release.ReleaseError) as e:
            release.cut(root=self.root)
        self.assertIn("version mismatch", str(e.exception))


SIGN_STEP = wr_test.SIGN_STEP
RELEASE_STEP = wr_test.RELEASE_STEP
SIBLINGS = ("quest-tracker", "quest_tracker", "quest tracker", "waystone-roller", "waystone_roller",
            "waystone roller")


def workflow_problems(text: str) -> list[str]:
    """Each property of the release workflow that `text` lacks, by name."""
    lines = [line.strip() for line in text.splitlines()]
    needs = {
        "tag trigger": '- "crafting-helper-v*"' in lines,
        "manual run with a tag input": "workflow_dispatch:" in lines and "tag:" in lines,
        "contents: write": "contents: write" in lines,
        "concurrency group": "group: crafting-helper-release-${{ github.event.inputs.tag || github.ref_name }}" in lines,
        "repository guard": "if: github.repository == 'S-Borkowski/ExilePact2'" in lines,
        "windows runner": "runs-on: windows-latest" in lines,
        "checks the tag": 'python -m tools.crafting_helper.release check "$version"' in lines,
        "builds the app": "npm --prefix apps/crafting-helper run release" in lines,
        "takes the exe": any("apps/crafting-helper/target/release/crafting-helper.exe" in line for line in lines),
        "hashes it": any(line.startswith("(cd dist-release && sha256sum crafting-helper.exe > SHA256SUMS.txt")
                         for line in lines),
        "notes from the guide": any(line.startswith("python -m tools.crafting_helper.release notes")
                                    for line in lines),
        "a draft release": any("gh release create" in line for line in lines) and "--draft" in text,
        "its title": any('--title "Crafting Helper $VERSION"' in line for line in lines),
        "attaches the exe, the sums and the signature":
            "assets=(dist-release/crafting-helper.exe dist-release/SHA256SUMS.txt "
            "dist-release/crafting-helper.exe.minisig)" in lines,
        "npm cache of this app": "cache-dependency-path: apps/crafting-helper/package-lock.json" in lines,
        "rust cache of this app": "workspaces: apps/crafting-helper" in lines,
        "nothing publishes": "--draft=false" not in text and "--latest" not in text
                             and "gh release edit" not in text,
        "nothing of either sibling": not any(name in text.lower() for name in SIBLINGS),
    }
    needs.update(wr_test.signing_problems(text, "crafting-helper.exe", "apps/crafting-helper"))
    return [name for name, ok in needs.items() if not ok]


class Workflow(unittest.TestCase):
    def setUp(self):
        self.text = WORKFLOW.read_text(encoding="utf-8")

    def test_the_workflow_has_every_property(self):
        self.assertEqual(workflow_problems(self.text), [])

    def test_each_property_is_caught_when_missing(self):
        mutations = {
            "tag trigger": ('- "crafting-helper-v*"', '- "v*"'),
            "contents: write": ("contents: write", "contents: read"),
            "concurrency group": ("group: crafting-helper-release-", "group: release-"),
            "repository guard": ("if: github.repository == 'S-Borkowski/ExilePact2'", "if: true"),
            "windows runner": ("runs-on: windows-latest", "runs-on: ubuntu-latest"),
            "checks the tag": ('release check "$version"', 'release check'),
            "builds the app": ("run release\n", "run build\n"),
            "takes the exe": ("cp apps/crafting-helper/target/release/crafting-helper.exe",
                              "cp apps/crafting-helper/target/debug/crafting-helper.exe"),
            "a draft release": ("--draft --verify-tag", "--verify-tag"),
            "its title": ('--title "Crafting Helper $VERSION"', '--title "$VERSION"'),
            "notes from the guide": ("python -m tools.crafting_helper.release notes",
                                     "python -m tools.crafting_helper.release changes"),
            "attaches the exe, the sums and the signature": (" dist-release/crafting-helper.exe.minisig)", ")"),
            "rust cache of this app": ("workspaces: apps/crafting-helper", "workspaces: apps"),
            "signs with the app's own CLI and the version": ('tauri signer sign --app-version "$VERSION" ',
                                                             "tauri signer sign "),
            "the secrets reach only the sign step's env": (
                "GH_TOKEN: ${{ github.token }}",
                "GH_TOKEN: ${{ github.token }}\n      TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}"),
            "fails without the key": ("\n            exit 1\n          fi\n          npm --prefix",
                                      "\n          fi\n          npm --prefix"),
            "writes the .minisig": ("base64 -d dist-release/crafting-helper.exe.sig",
                                    "cp dist-release/crafting-helper.exe.sig"),
        }
        for name, (old, new) in mutations.items():
            with self.subTest(name=name):
                self.assertIn(old, self.text)
                self.assertIn(name, workflow_problems(self.text.replace(old, new, 1)))
        published = self.text + "\n      - run: gh release edit \"$TAG\" --draft=false\n"
        self.assertIn("nothing publishes", workflow_problems(published))
        # The sign step moved after the release step: a failed sign would
        # come too late to stop the release.
        start = self.text.index("      - name: " + SIGN_STEP)
        end = self.text.index("      - name: " + RELEASE_STEP)
        moved = self.text[:start] + self.text[end:] + "\n" + self.text[start:end]
        self.assertIn("signs before the release", workflow_problems(moved))

    def test_a_mention_of_either_sibling_is_caught(self):
        # One mention, anywhere (a comment counts), of each sibling's name.
        for mention in ("apps/quest-tracker/package-lock.json", "tools.quest_tracker.release",
                        "# as the Quest Tracker does", "apps/waystone-roller/package-lock.json",
                        "tools.waystone_roller.release", "# as the Waystone Roller does"):
            with self.subTest(mention=mention):
                problems = workflow_problems(self.text + "\n" + mention + "\n")
                self.assertEqual(problems, ["nothing of either sibling"])


class RealTree(unittest.TestCase):
    def test_the_real_tree_is_consistent(self):
        version = release.check()
        self.assertTrue(release.notes(version).startswith("Crafting Helper " + version))


if __name__ == "__main__":
    unittest.main()
