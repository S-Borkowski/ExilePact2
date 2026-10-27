#!/usr/bin/env python3
"""Shared plumbing for the PostToolUse hooks.

Every PostToolUse check keys off the **working tree**, not off the tool payload. That
choice is the whole reason this module exists, and it is worth stating once:

A payload-shaped hook only sees `Edit` and `Write`, and only sees them when it
can resolve `tool_input.file_path` against the repository root. Both halves of
that leak. A `sed -i` or a heredoc under `Bash` rewrites a file without ever
producing a `file_path`, and a path the hook cannot resolve returns "not my
file" -- which is indistinguishable from "nothing wrong". During development
exactly that happened (in the sister project these hooks were ported from):
every hook exited 0 against a harness feeding them POSIX-style paths Windows
Python could not resolve, and looked installed and healthy while guarding
nothing.

Asking git what changed costs one subprocess (~40ms) and has neither hole.
"""

import os
import re
import subprocess
from pathlib import Path


def _parse_porcelain(blob: bytes) -> list[tuple[str, str]]:
    """(status, path) pairs out of one `git status --porcelain -z` blob.

    Shared by `changed_entries` and `TreeState` below so the two callers of
    `-z`'s NUL-delimited, rename-aware format agree on how to read it.
    """
    fields = blob.decode("utf-8", "replace").split("\0")
    entries: list[tuple[str, str]] = []
    index = 0
    while index < len(fields):
        entry = fields[index]
        index += 1
        if len(entry) < 4:
            continue
        status, path = entry[:2], entry[3:]
        # A rename entry is followed by its source path in the next field.
        if "R" in status or "C" in status:
            index += 1
        if path:
            entries.append((status, path))
    return entries


def _under_pathspec(rel: str, pathspec: str) -> bool:
    """True when `rel` is `pathspec` itself or something inside it, matching
    git's own pathspec semantics on directory-component boundaries -- so a
    pathspec of `src` selects `src/thing.cpp` but not `src-gen/thing.cpp`,
    exactly as a narrow `git status -- <pathspec>` call would."""
    if pathspec in (".", ""):
        return True
    spec = pathspec.rstrip("/")
    return rel == spec or rel.startswith(spec + "/")


class TreeState:
    """One top-level `git status` plus one parallel round of per-submodule
    statuses (none when the repository has no `.gitmodules`), memoised for
    the life of this object.

    This is the dispatcher's explicit injection point: `.claude/hooks/post_tool_use.py`
    builds exactly one of these per PostToolUse call and hands it to every
    check, so the checks share one git plan instead of each running its own
    `git status`. A check's own standalone `main()` builds one just for
    itself, so its query surface -- not its per-call process count -- is
    unchanged when run outside the dispatcher.

    `hub_entries()` ("hub" = the top-level repository) runs
    `--ignore-submodules=all` so it never spawns a child `git status` inside
    each submodule (measured at ~150-200ms extra in the sister project);
    `submodule_entries()` recovers that information itself, as one parallel
    round of per-submodule statuses, which is also what lets
    `dirty_submodules()` answer without a second full-tree scan.
    """

    def __init__(self, root: Path):
        self.root = root
        self._hub: list[tuple[str, str]] | None = None
        self._subs: dict[str, list[tuple[str, str]]] | None = None

    def hub_entries(self) -> list[tuple[str, str]]:
        if self._hub is None:
            out = subprocess.run(
                ["git", "status", "--porcelain", "-z", "-uall",
                 "--ignore-submodules=all", "--", "."],
                cwd=self.root,
                capture_output=True,
                check=False,
            )
            self._hub = _parse_porcelain(out.stdout) if out.returncode == 0 else []
        return self._hub

    def _submodule_dirs(self) -> list[str]:
        try:
            text = (self.root / ".gitmodules").read_bytes().decode("utf-8", "replace")
        except OSError:
            return []
        return re.findall(r"^\s*path\s*=\s*(.+?)\s*$", text, re.MULTILINE)

    def submodule_entries(self) -> dict[str, list[tuple[str, str]]]:
        if self._subs is None:
            procs = []
            for rel in self._submodule_dirs():
                if not (self.root / rel / ".git").exists():
                    continue
                procs.append((rel, subprocess.Popen(
                    ["git", "status", "--porcelain", "-z", "-uall", "--", "."],
                    cwd=self.root / rel,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.DEVNULL,
                )))
            subs: dict[str, list[tuple[str, str]]] = {}
            for rel, proc in procs:
                out, _ = proc.communicate()
                subs[rel] = _parse_porcelain(out) if proc.returncode == 0 else []
            self._subs = subs
        return self._subs

    def changed_entries(self, tree: Path, pathspec: str) -> list[tuple[str, str]]:
        """Same result `changed_entries(tree, pathspec)` below would return
        for `tree`, filtered from the memoised status instead of a fresh
        subprocess. `tree` must be `self.root` or a submodule directory under
        it; anything else -- an untracked or uninitialised submodule -- has no
        status to report and returns `[]`, same as a narrow query would for a
        pathspec outside the working tree."""
        if tree == self.root:
            return [e for e in self.hub_entries() if _under_pathspec(e[1], pathspec)]
        for rel, entries in self.submodule_entries().items():
            if self.root / rel == tree:
                return [e for e in entries if _under_pathspec(e[1], pathspec)]
        return []

    def changed_paths(self, tree: Path, pathspec: str) -> list[str]:
        return [path for _, path in self.changed_entries(tree, pathspec)]

    def dirty_submodules(self) -> list[str]:
        """Submodules whose own working tree differs from their HEAD --
        output-equivalent to a 'top-level status reports it changed' test, because a
        submodule whose pointer moved but whose tree is clean scans to
        nothing either way."""
        return [rel for rel, entries in self.submodule_entries().items() if entries]


def repo_root() -> Path | None:
    """The top of the working tree, or None if git cannot say."""
    try:
        out = subprocess.run(
            ["git", "rev-parse", "--show-toplevel"],
            capture_output=True,
            text=True,
            check=True,
        )
    except (OSError, subprocess.CalledProcessError):
        return None
    root = out.stdout.strip()
    if not root:
        return None
    # Resolve both sides of any later comparison the same way: a checkout
    # reached through a junction or symlink otherwise never matches a resolved
    # payload path.
    try:
        return Path(root).resolve()
    except OSError:
        return Path(root)


def changed_entries(root: Path, pathspec: str) -> list[tuple[str, str]]:
    """(status, path) for everything under `pathspec` that differs from HEAD.

    `status` is git's two-character porcelain code, so a caller can tell an
    untracked file (`??`) from a modified one. `decompiled_output.py` needs
    that distinction: for a tracked file only the *added* lines are its
    business, while an untracked file is new in its entirety.
    """
    out = subprocess.run(
        ["git", "status", "--porcelain", "-z", "-uall", "--", pathspec],
        cwd=root,
        capture_output=True,
        check=False,
    )
    if out.returncode != 0:
        return []
    return _parse_porcelain(out.stdout)


def changed_paths(root: Path, pathspec: str) -> list[str]:
    """Repo-relative paths under `pathspec` that differ from HEAD.

    Includes untracked files, which is deliberate -- a brand new `.cpp` file is
    exactly as capable of carrying a violation as an edited one.

    `-uall` (in `changed_entries`, which this wraps) is load-bearing, not
    tidiness. By default git *collapses* an untracked directory to a single
    entry (`?? docs/`) and never names the files inside it, so every hook built
    on this helper was blind to anything in a newly created folder -- it saw a
    directory it had no suffix rule for and moved on. That is the same "returns
    'not my file', which is indistinguishable from 'nothing wrong'" hole this
    module exists to close, reached by a different route. It was found by
    `decompiled_output.py`'s tests, where a listing written to a fresh `docs/`
    went unflagged; the fix belongs here rather than in that hook, because
    every check built on this helper shares the blindness.

    Uses `-z` so paths containing spaces, quotes or non-ASCII arrive verbatim
    instead of in git's quoted form.
    """
    return [path for _, path in changed_entries(root, pathspec)]


def skip_requested() -> bool:
    """True when the operator has deliberately switched the hooks off.

    There are legitimate states these hooks would otherwise wedge -- for
    example re-vendoring a third-party dependency whose sources carry
    IDA-style symbols, which `decompiled_output.py` cannot tell from a pasted
    listing. Without an escape hatch a blocking check would then fail every
    subsequent tool call in the repository, including the ones needed to
    finish or undo the work.

    A blocking hook with no way out is worse than no hook, so every blocking
    message names this variable.
    """
    return os.environ.get("EP2_SKIP_HOOKS", "").strip() not in ("", "0", "false")


SKIP_HINT = (
    "If this state is deliberate, set EP2_SKIP_HOOKS=1 for the session to "
    "silence this repository's PostToolUse hooks."
)
