"""tools/quest_tracker/release.py: check, notes and cut, on synthetic files.

Every case builds a minimal copy of the files that carry a version and of the
guide's "Release notes" in a temporary folder; `cut` runs against a git
repository there whose `origin` is a bare repository beside it, so nothing
touches this checkout or the network.

The baseline is a consistent tree at 1.2.0 that `check` accepts and `cut`
tags. The targets are the refusals: a file that disagrees (named), a version
with no or empty notes, uncommitted changes, a HEAD behind the remote's
branch, and a tag that already exists here or on the remote. The real tree
is checked too, so a version bump that misses a file fails here first. A
path package in `Cargo.lock` that is not a member of the app's workspace
(the shared `apps/shared/` crates) carries its own version and is not read.
`Workflow` reads `.github/workflows/quest-tracker-release.yml` as text (no
YAML parser is installed) and checks the properties the release path relies
on, signing included, with a mutated copy of the file as each check's
negative control. `CollectStep` runs the workflow's "Collect the assets"
lines with bash on a fake build, `Quest Tracker.exe` since 0.2.0, and
checks that the release still gets `quest-tracker.exe`; its negative
control is the copy line from before 0.2.0. `TauriConf` pins the build's
name and the identifier that must not follow it.
"""
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from tools.quest_tracker import release  # noqa: E402

WORKFLOW = ROOT / ".github" / "workflows" / "quest-tracker-release.yml"

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


MEMBERS = {"crates/qt-engine": "qt-engine", "src-tauri": "quest-tracker"}


def write_tree(root: Path, version: str = "1.2.0", guide: str = GUIDE, lag: dict | None = None,
               extra_lock: str = "") -> None:
    """A minimal app tree at `version`; `lag` overrides one file's version.

    The workspace has two members, `qt-engine` and `quest-tracker`;
    `extra_lock` is appended to `Cargo.lock` as written.
    """
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
    (app / "Cargo.toml").write_text(
        '[workspace]\nmembers = [%s]\n\n[workspace.package]\nversion = "%s"\n'
        % (", ".join('"%s"' % m for m in MEMBERS), v("cargo")), encoding="utf-8")
    for member, name in MEMBERS.items():
        (app / member).mkdir(parents=True, exist_ok=True)
        (app / member / "Cargo.toml").write_text(
            '[package]\nname = "%s"\nversion.workspace = true\n' % name, encoding="utf-8")
    (app / "Cargo.lock").write_text(
        'version = 4\n\n[[package]]\nname = "qt-engine"\nversion = "%s"\n\n'
        '[[package]]\nname = "quest-tracker"\nversion = "%s"\n\n'
        '[[package]]\nname = "serde"\nversion = "9.9.9"\nsource = "registry+https://example.invalid"\n'
        % (v("lock-member"), v("lock-app")) + extra_lock, encoding="utf-8")
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
                           ("lock-member", "Cargo.lock qt-engine"), ("lock-app", "Cargo.lock quest-tracker")):
            with self.subTest(key=key), tempfile.TemporaryDirectory() as d:
                write_tree(Path(d), lag={key: "1.1.0"})
                with self.assertRaises(release.ReleaseError) as e:
                    release.check("1.2.0", Path(d))
                self.assertIn(where, str(e.exception))

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
        for name in MEMBERS.values():
            self.assertEqual(found["Cargo.lock " + name], "1.2.0")

    def test_a_member_missing_from_the_lock_is_named(self):
        # Negative control for reading members from Cargo.toml: a member the
        # lock does not list fails rather than going unchecked.
        write_tree(self.root)
        lock = self.root / "apps" / "quest-tracker" / "Cargo.lock"
        lock.write_text(lock.read_text(encoding="utf-8").replace('name = "quest-tracker"', 'name = "other"'),
                        encoding="utf-8")
        with self.assertRaises(release.ReleaseError) as e:
            release.check("1.2.0", self.root)
        self.assertIn("quest-tracker", str(e.exception))

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

    def test_the_header_names_the_signature(self):
        with tempfile.TemporaryDirectory() as d:
            write_tree(Path(d))
            body = release.notes("1.2.0", Path(d))
        header = body.split("## Changelog")[0]
        for needed in ("`quest-tracker.exe`", "`quest-tracker-replay.exe`", "`SHA256SUMS.txt`",
                       "`quest-tracker.exe.minisig`"):
            self.assertIn(needed, header)


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


SIGN_STEP = "Sign the exe"
RELEASE_STEP = "Create or update the draft release"
COLLECT_STEP = "Collect the assets"
# The build is `Quest Tracker.exe` (tauri.conf.json's `mainBinaryName`); the
# release asset stays `quest-tracker.exe`, the name every install asks for
# and the one its signature names, so the Collect step copies it under that.
COLLECT_GUI = 'cp "apps/quest-tracker/target/release/Quest Tracker.exe" dist-release/quest-tracker.exe'
# What the Collect step copied before 0.2.0, when the build had the asset's name.
COLLECT_GUI_BEFORE = "cp apps/quest-tracker/target/release/quest-tracker.exe dist-release/"


def workflow_steps(text: str) -> list[tuple[str, list[str]]]:
    """The job's steps in order: each step's name (or its `uses:`) and its stripped lines."""
    steps: list[tuple[str, list[str]]] = []
    for line in text.splitlines():
        if line.startswith("      - "):
            head = line[8:].strip()
            steps.append((head[len("name: "):] if head.startswith("name: ") else head, []))
        elif steps and line.startswith("        "):
            steps[-1][1].append(line.strip())
    return steps


def signing_problems(text: str, exe: str, app: str) -> dict[str, bool]:
    """The signing properties, shared in shape with the waystone roller's test."""
    steps = workflow_steps(text)
    names = [name for name, _ in steps]
    sign = next((body for name, body in steps if name == SIGN_STEP), [])
    env = sign[sign.index("env:") + 1:sign.index("run: |")] if "env:" in sign and "run: |" in sign else []
    command = 'npm --prefix %s exec -- tauri signer sign --app-version "$VERSION" ' \
              '"$GITHUB_WORKSPACE/dist-release/%s"' % (app, exe)
    guard = 'if [ -z "${TAURI_SIGNING_PRIVATE_KEY:-}" ]; then'
    fails = (guard in sign and command in sign and sign.index(guard) < sign.index(command)
             and sign[sign.index(guard) + 1].startswith('echo "::error::')
             and sign[sign.index(guard) + 2] == "exit 1")
    return {
        "signs with the app's own CLI and the version": command in sign,
        "the secrets reach only the sign step's env":
            "TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}" in env
            and "TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}" in env
            and text.count("secrets.") == 2,
        "fails without the key": fails,
        "writes the .minisig": "base64 -d dist-release/%s.sig > dist-release/%s.minisig" % (exe, exe) in sign,
        "signs before the release": SIGN_STEP in names and RELEASE_STEP in names
                                    and names.index(SIGN_STEP) < names.index(RELEASE_STEP),
    }


def workflow_problems(text: str) -> list[str]:
    """Each property of the release workflow that `text` lacks, by name."""
    lines = [line.strip() for line in text.splitlines()]
    needs = {
        "tag trigger": '- "quest-tracker-v*"' in lines,
        "manual run with a tag input": "workflow_dispatch:" in lines and "tag:" in lines,
        "contents: write": "contents: write" in lines,
        "concurrency group": "group: quest-tracker-release-${{ github.event.inputs.tag || github.ref_name }}" in lines,
        "repository guard": "if: github.repository == 'S-Borkowski/ExilePact2'" in lines,
        "windows runner": "runs-on: windows-latest" in lines,
        "checks the tag": 'python -m tools.quest_tracker.release check "$version"' in lines,
        "builds the app": "npm --prefix apps/quest-tracker run release" in lines,
        "takes both exes": COLLECT_GUI in lines
                           and "cp apps/quest-tracker/target/release/quest-tracker-replay.exe dist-release/" in lines,
        "hashes them": any(line.startswith("(cd dist-release && sha256sum quest-tracker.exe "
                                           "quest-tracker-replay.exe > SHA256SUMS.txt") for line in lines),
        "notes from the guide": any(line.startswith("python -m tools.quest_tracker.release notes")
                                    for line in lines),
        "a draft release": any("gh release create" in line for line in lines) and "--draft" in text,
        "its title": any('--title "Quest Tracker $VERSION"' in line for line in lines),
        "attaches the exes, the sums and the signature":
            "assets=(dist-release/quest-tracker.exe dist-release/quest-tracker-replay.exe "
            "dist-release/SHA256SUMS.txt dist-release/quest-tracker.exe.minisig)" in lines,
        "npm cache of this app": "cache-dependency-path: apps/quest-tracker/package-lock.json" in lines,
        "rust cache of this app": "workspaces: apps/quest-tracker" in lines,
        "nothing publishes": "--draft=false" not in text and "--latest" not in text
                             and "gh release edit" not in text,
        "nothing of the waystone roller": "waystone-roller" not in text and "waystone_roller" not in text,
    }
    needs.update(signing_problems(text, "quest-tracker.exe", "apps/quest-tracker"))
    return [name for name, ok in needs.items() if not ok]


class Workflow(unittest.TestCase):
    def setUp(self):
        self.text = WORKFLOW.read_text(encoding="utf-8")

    def test_the_workflow_has_every_property(self):
        self.assertEqual(workflow_problems(self.text), [])

    def test_each_property_is_caught_when_missing(self):
        mutations = {
            "tag trigger": ('- "quest-tracker-v*"', '- "v*"'),
            "contents: write": ("contents: write", "contents: read"),
            "repository guard": ("if: github.repository == 'S-Borkowski/ExilePact2'", "if: true"),
            "windows runner": ("runs-on: windows-latest", "runs-on: ubuntu-latest"),
            "a draft release": ("--draft --verify-tag", "--verify-tag"),
            "notes from the guide": ("python -m tools.quest_tracker.release notes",
                                     "python -m tools.quest_tracker.release changes"),
            "takes both exes": ("cp apps/quest-tracker/target/release/quest-tracker-replay.exe dist-release/\n",
                                ""),
            "attaches the exes, the sums and the signature": (" dist-release/quest-tracker.exe.minisig)", ")"),
            "nothing of the waystone roller": ("workspaces: apps/quest-tracker", "workspaces: apps/waystone-roller"),
            "signs with the app's own CLI and the version": ('tauri signer sign --app-version "$VERSION" ',
                                                             "tauri signer sign "),
            "the secrets reach only the sign step's env": (
                "GH_TOKEN: ${{ github.token }}",
                "GH_TOKEN: ${{ github.token }}\n      TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}"),
            "fails without the key": ("\n            exit 1\n          fi\n          npm --prefix",
                                      "\n          fi\n          npm --prefix"),
            "writes the .minisig": ("base64 -d dist-release/quest-tracker.exe.sig", "cp dist-release/quest-tracker.exe.sig"),
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


class TauriConf(unittest.TestCase):
    def test_tauri_conf_names_the_build_quest_tracker(self):
        # `mainBinaryName` names the built exe, `Quest Tracker.exe`. The
        # identifier keys the WebView2 data folder and the single instance,
        # so it must not follow the product name.
        conf = json.loads((ROOT / "apps" / "quest-tracker" / "src-tauri" / "tauri.conf.json")
                          .read_text(encoding="utf-8"))
        overlay = [w for w in conf["app"]["windows"] if w["label"] == "overlay"]
        self.assertEqual(len(overlay), 1)
        self.assertEqual(
            (conf["productName"], conf["mainBinaryName"], conf["identifier"], overlay[0]["title"]),
            ("Quest Tracker", "Quest Tracker", "net.exilepact2.quest-tracker", "Quest Tracker"))


def find_bash() -> str:
    """Git for Windows' bash on Windows (never WSL's System32 one), else `bash` on PATH."""
    if os.name != "nt":
        found = shutil.which("bash")
        if not found:
            raise AssertionError("no bash on PATH: the Collect-step dry run needs one")
        return found
    roots = []
    exec_path = subprocess.run(["git", "--exec-path"], capture_output=True, text=True).stdout.strip()
    if exec_path:
        roots += list(Path(exec_path).parents)
    git_exe = shutil.which("git")
    if git_exe:
        roots += list(Path(git_exe).resolve().parents)
    for root in roots:
        for candidate in (root / "bin" / "bash.exe", root / "usr" / "bin" / "bash.exe"):
            if candidate.is_file() and "system32" not in str(candidate).lower():
                return str(candidate)
    raise AssertionError("no Git for Windows bash found from `git --exec-path` or the git on PATH: "
                         "the Collect-step dry run needs one")


def collect_script(text: str) -> list[str]:
    """The Collect step's run lines, without the release-notes line (`Notes` covers it)."""
    body = next((body for name, body in workflow_steps(text) if name == COLLECT_STEP), [])
    run = body[body.index("run: |") + 1:] if "run: |" in body else []
    return [line for line in run if not line.startswith("python -m tools.quest_tracker.release notes")]


class CollectStep(unittest.TestCase):
    """The Collect step run with bash on fake build outputs.

    The `.minisig` cannot be made here (it needs the release key);
    `signing_problems` pins that the sign step signs
    `dist-release/quest-tracker.exe`.
    """
    GUI = b"the built Quest Tracker.exe of the test"
    REPLAY = b"the built quest-tracker-replay.exe of the test"

    def run_collect(self, lines: list[str], root: Path) -> subprocess.CompletedProcess:
        release_dir = root / "apps" / "quest-tracker" / "target" / "release"
        release_dir.mkdir(parents=True)
        (release_dir / "Quest Tracker.exe").write_bytes(self.GUI)
        (release_dir / "quest-tracker-replay.exe").write_bytes(self.REPLAY)
        script = "\n".join(lines) + "\n"
        if os.name == "nt":
            script = 'export PATH="/usr/bin:$PATH"\n' + script
        # The flags GitHub Actions runs a `shell: bash` step with.
        return subprocess.run([find_bash(), "--noprofile", "--norc", "-eo", "pipefail", "-c", script],
                              cwd=root, capture_output=True, text=True)

    def test_collect_step_dry_run(self):
        lines = collect_script(WORKFLOW.read_text(encoding="utf-8"))
        self.assertIn(COLLECT_GUI, lines)
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            done = self.run_collect(lines, root)
            self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
            dist = root / "dist-release"
            self.assertEqual(sorted(p.name for p in dist.iterdir()),
                             ["SHA256SUMS.txt", "quest-tracker-replay.exe", "quest-tracker.exe"])
            self.assertEqual((dist / "quest-tracker.exe").read_bytes(), self.GUI)
            self.assertEqual((dist / "quest-tracker-replay.exe").read_bytes(), self.REPLAY)
            sums = [re.fullmatch(r"([0-9a-f]{64}) [ *](.+)", line)
                    for line in (dist / "SHA256SUMS.txt").read_text(encoding="utf-8").splitlines()]
            self.assertTrue(all(sums), (dist / "SHA256SUMS.txt").read_text(encoding="utf-8"))
            self.assertEqual([(m.group(1), m.group(2)) for m in sums],
                             [(hashlib.sha256(self.GUI).hexdigest(), "quest-tracker.exe"),
                              (hashlib.sha256(self.REPLAY).hexdigest(), "quest-tracker-replay.exe")])
        # Negative control: the copy from before 0.2.0 finds no
        # `quest-tracker.exe` in a build that is `Quest Tracker.exe`.
        before = [COLLECT_GUI_BEFORE if line == COLLECT_GUI else line for line in lines]
        self.assertNotEqual(before, lines)
        with tempfile.TemporaryDirectory() as d:
            done = self.run_collect(before, Path(d))
            self.assertNotEqual(done.returncode, 0, done.stdout + done.stderr)
            self.assertFalse((Path(d) / "dist-release" / "SHA256SUMS.txt").exists())


if __name__ == "__main__":
    unittest.main()
