"""Cut a crafting helper release: check the version, draft its notes, tag it.

Developer-only. A release is one annotated tag, `crafting-helper-v<version>`;
pushing it starts `.github/workflows/crafting-helper-release.yml`, which
builds `Crafting Helper.exe`, signs it with the release key (and fails
without it) and attaches it as `Crafting.Helper.exe` (GitHub stores a space
as a dot), with `Crafting.Helper.exe.minisig`, `SHA256SUMS.txt` and the same
exe under its 0.1.x name, `crafting-helper.exe` with its own
`crafting-helper.exe.minisig`, to a draft release for the owner to publish
(and mark as latest) by hand:

    py -3 -m tools.crafting_helper.release check [VERSION]
    py -3 -m tools.crafting_helper.release notes VERSION [--out FILE]
    py -3 -m tools.crafting_helper.release cut [--push] [--remote NAME] [--branch NAME]

`check` reads the version from every file under `apps/crafting-helper/` that
carries one (`package.json`, `package-lock.json`, `src-tauri/tauri.conf.json`,
the workspace's `Cargo.toml`, a member's own `Cargo.toml` when it writes its
version literally, and the workspace members in `Cargo.lock`) and fails,
naming each file, unless they all agree, and agree with VERSION when it is
given. The members are the ones the workspace's `Cargo.toml` lists, with its
globs (`crates/*`) expanded the way cargo expands them: every directory a
pattern matches is a member and must have a `Cargo.toml`, a plain file is
skipped, and a pattern that matches nothing fails. The shared crates of
`apps/shared/`, which the lock also lists without a source, keep their own
workspace's version and are not read. It also fails when the guide's
"Release notes" (`docs/crafting-helper/README.md`) has no section for the
version, or an empty one.

`notes` prints the release body: a short header, then the version's section
of the guide's "Release notes", with each hard-wrapped list item joined onto
one line (a release body renders a single newline as a line break). The
header names each asset, `Crafting.Helper.exe.minisig` (the release key's
signature, which the app checks before it installs an update) among them,
and says the legacy `crafting-helper.exe` is for in-app updates from 0.1.x.

`cut` runs `check` on the files' own version, then refuses unless the work
tree has no uncommitted change to a tracked file, HEAD is the tip of
`<remote>/<branch>` (default `origin/main`) after a fetch, and the tag exists
neither here nor on the remote. It then creates the annotated tag, and
pushes it only with `--push`; without, it prints the push command.

The quest tracker's and the waystone roller's tools work the same way for
their own apps. Their functions read that app's paths and tag from module
globals, so only the app-independent pieces are imported from the quest
tracker's: the error type, the git runner and the unwrapping of release
notes. Its reading of a workspace's members opens `<member>/Cargo.toml` for
each `members` entry as written and cannot expand a glob, so this app's
members are read here.
"""
from __future__ import annotations

import argparse
import json
import re
import sys
import tomllib
from pathlib import Path

from tools.quest_tracker.release import ReleaseError, _git, unwrap

ROOT = Path(__file__).resolve().parents[2]
APP = Path("apps") / "crafting-helper"
GUIDE = Path("docs") / "crafting-helper" / "README.md"
TAG_PREFIX = "crafting-helper-v"
REPO_URL = "https://github.com/S-Borkowski/ExilePact2"

__all__ = ["ReleaseError", "tag_for", "members", "versions", "changelog", "notes", "check", "cut", "main"]


def tag_for(version: str) -> str:
    return TAG_PREFIX + version


def members(app: Path) -> list[str]:
    """The workspace's member directories, relative to `app`, globs expanded.

    Cargo reads each `members` entry as a glob: every directory it matches
    is a member and must carry a `Cargo.toml`, a plain file is ignored, and
    an entry that matches nothing fails to load. The same holds here, as a
    `ReleaseError` naming the entry or the directory.
    """
    with open(app / "Cargo.toml", "rb") as f:
        patterns = tomllib.load(f)["workspace"]["members"]
    found: list[str] = []
    for pattern in patterns:
        matched = sorted(path for path in app.glob(pattern) if path.is_dir())
        if not matched:
            raise ReleaseError("Cargo.toml: the workspace member %r matches no directory under %s"
                               % (pattern, APP.as_posix()))
        for path in matched:
            rel = path.relative_to(app).as_posix()
            if not (path / "Cargo.toml").is_file():
                raise ReleaseError("the workspace member %s/%s has no Cargo.toml" % (APP.as_posix(), rel))
            if rel not in found:
                found.append(rel)
    return found


def member_versions(app: Path) -> dict[str, str]:
    """Each member's literal version and its `Cargo.lock` version.

    A member that inherits the workspace's version (`version.workspace =
    true`) is read from the lock only; one that writes its own (`ch-item`)
    is read from its `Cargo.toml` too. Any other package without a source is
    a path dependency from another workspace (the shared crates in
    `apps/shared/`), which carries that workspace's version and is not this
    app's to check. A member the lock does not list fails.
    """
    found: dict[str, str] = {}
    names: list[str] = []
    for rel in members(app):
        with open(app / rel / "Cargo.toml", "rb") as f:
            package = tomllib.load(f)["package"]
        names.append(package["name"])
        if isinstance(package.get("version"), str):
            found["%s/Cargo.toml [package]" % rel] = package["version"]
    with open(app / "Cargo.lock", "rb") as f:
        local = {p["name"]: p["version"] for p in tomllib.load(f)["package"] if "source" not in p}
    missing = [name for name in names if name not in local]
    if missing:
        raise ReleaseError("Cargo.lock lists no workspace member %s; run cargo to update the lock"
                           % ", ".join(missing))
    found.update({"Cargo.lock " + name: local[name] for name in names})
    return found


def versions(root: Path = ROOT) -> dict[str, str]:
    """Every version the app's files carry, keyed by where it was read."""
    app = root / APP
    found: dict[str, str] = {}
    package = json.loads((app / "package.json").read_text(encoding="utf-8"))
    found["package.json"] = package["version"]
    lock = json.loads((app / "package-lock.json").read_text(encoding="utf-8"))
    found["package-lock.json"] = lock["version"]
    found["package-lock.json packages[\"\"]"] = lock["packages"][""]["version"]
    tauri = json.loads((app / "src-tauri" / "tauri.conf.json").read_text(encoding="utf-8"))
    found["src-tauri/tauri.conf.json"] = tauri["version"]
    with open(app / "Cargo.toml", "rb") as f:
        found["Cargo.toml [workspace.package]"] = tomllib.load(f)["workspace"]["package"]["version"]
    found.update(member_versions(app))
    return found


def changelog(version: str, root: Path = ROOT) -> str:
    """The guide's "Release notes" section for `version`, as written."""
    lines = (root / GUIDE).read_text(encoding="utf-8").splitlines()
    try:
        start = lines.index("## Release notes")
    except ValueError:
        raise ReleaseError("%s has no \"## Release notes\" section" % GUIDE.as_posix()) from None
    # The version, then a space or the end: "### 0.1.0 (first version)".
    heading = re.compile(r"### %s(\s|$)" % re.escape(version))
    body: list[str] | None = None
    for line in lines[start + 1:]:
        if line.startswith("## "):
            break
        if line.startswith("### "):
            if body is not None:
                break
            if heading.match(line):
                body = []
            continue
        if body is not None:
            body.append(line)
    if body is None:
        raise ReleaseError("%s has no release notes for %s (a \"### %s\" heading under \"## Release notes\")"
                           % (GUIDE.as_posix(), version, version))
    text = "\n".join(body).strip()
    if not text:
        raise ReleaseError("%s: the release notes for %s are empty" % (GUIDE.as_posix(), version))
    return text


def notes(version: str, root: Path = ROOT) -> str:
    guide = "%s/blob/%s/%s" % (REPO_URL, tag_for(version), GUIDE.as_posix())
    header = (
        "Crafting Helper %s, a small window for Path of Exile 2 that shows the item you grab: copy it "
        "in the game with the game's own `Ctrl+Alt+C`, then press the app's hotkey (`Ctrl+Alt+X` by "
        "default) or `Ctrl+V` in its window. It reads the clipboard once per grab and never watches it, "
        "sends the game no keys, reads no game memory, moves no mouse and clicks nothing.\n\n"
        "Download `Crafting.Helper.exe` below and run it; there is no installer. GitHub lists "
        "`Crafting Helper.exe` with a dot for the space, so rename it back if you like. It needs "
        "Windows 10 or 11, 64-bit, with the Microsoft Edge WebView2 runtime, and the game in English. "
        "`SHA256SUMS.txt` lists the files' hashes. `Crafting.Helper.exe.minisig` is the release key's "
        "signature of it, which the app checks before it installs an update. `crafting-helper.exe` and "
        "`crafting-helper.exe.minisig` are the same program and its signature under the old name, for "
        "in-app updates from 0.1.x; there is no need to download them.\n\n"
        "Guide: [docs/crafting-helper/README.md](%s).\n\n"
        "## Changelog\n\n" % (version, guide)
    )
    return header + unwrap(changelog(version, root)) + "\n"


def check(version: str | None = None, root: Path = ROOT) -> str:
    """The one version every file agrees on; raises naming each disagreement."""
    found = versions(root)
    expected = version or found["package.json"]
    wrong = ["%s is %s" % (where, v) for where, v in found.items() if v != expected]
    if wrong:
        raise ReleaseError("version mismatch, expected %s: %s" % (expected, "; ".join(wrong)))
    changelog(expected, root)
    return expected


def cut(push: bool = False, remote: str = "origin", branch: str = "main", root: Path = ROOT) -> str:
    """Tag the checked-out commit as the files' version; returns the tag."""
    version = check(None, root)
    tag = tag_for(version)
    if _git(root, "status", "--porcelain", "--untracked-files=no"):
        raise ReleaseError("the work tree has uncommitted changes to tracked files; commit or stash them first")
    _git(root, "fetch", "--quiet", remote, branch)
    head = _git(root, "rev-parse", "HEAD")
    tip = _git(root, "rev-parse", "%s/%s" % (remote, branch))
    if head != tip:
        raise ReleaseError("HEAD %s is not the tip of %s/%s (%s); a release is cut from %s as pushed"
                           % (head[:12], remote, branch, tip[:12], branch))
    if _git(root, "tag", "--list", tag):
        raise ReleaseError("the tag %s already exists here" % tag)
    if _git(root, "ls-remote", "--tags", remote, "refs/tags/" + tag):
        raise ReleaseError("the tag %s already exists on %s" % (tag, remote))
    _git(root, "tag", "-a", tag, "-m", "Crafting Helper " + version)
    if push:
        _git(root, "push", remote, "refs/tags/" + tag)
    return tag


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="py -3 -m tools.crafting_helper.release",
                                     description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="command", required=True)
    p_check = sub.add_parser("check", help="every version agrees, and the guide has its notes")
    p_check.add_argument("version", nargs="?")
    p_notes = sub.add_parser("notes", help="print the release body for VERSION")
    p_notes.add_argument("version")
    p_notes.add_argument("--out", type=Path)
    p_cut = sub.add_parser("cut", help="tag HEAD of main as the files' version")
    p_cut.add_argument("--push", action="store_true")
    p_cut.add_argument("--remote", default="origin")
    p_cut.add_argument("--branch", default="main")
    args = parser.parse_args(argv)
    try:
        if args.command == "check":
            print("release: %s is consistent" % check(args.version))
        elif args.command == "notes":
            body = notes(args.version)
            if args.out:
                args.out.write_text(body, encoding="utf-8", newline="\n")
            else:
                sys.stdout.write(body)
        else:
            tag = cut(push=args.push, remote=args.remote, branch=args.branch)
            if args.push:
                print("release: pushed %s; the release workflow builds it into a draft release" % tag)
            else:
                print("release: tagged %s; push it with: git push %s refs/tags/%s" % (tag, args.remote, tag))
    except ReleaseError as e:
        print("release: error: %s" % e, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
