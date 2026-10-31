#!/usr/bin/env python3
"""See and reclaim the disk space this repository's checkouts cost (issue #38).

Every checkout -- the main one and each `git worktree` -- builds its apps into
its own `apps/<app>/target/` and installs its own `apps/<app>/node_modules/`.
A debug `target/` grows by gigabytes per build, so a handful of worktrees can
fill a drive. This reports what each checkout costs, removes worktrees whose
work is already merged, clears stale registrations, and trims a `target/`
that grew past a cap. Nothing runs it automatically.

Usage:
    py -3 tools/disk_hygiene.py [--repo DIR] report   [--into REF]
    py -3 tools/disk_hygiene.py [--repo DIR] worktrees [--into REF] [--apply]
    py -3 tools/disk_hygiene.py [--repo DIR] trim      [--cap-gib N] [--apply]
    py -3 tools/disk_hygiene.py [--repo DIR] check     [--min-free-gib N]

`--repo` defaults to the current directory's checkout; `--into` defaults to
`origin/main`. `worktrees` and `trim` are dry runs unless `--apply`: they
print what they would do and change nothing.

`worktrees` prints one line per registered worktree, starting with its action:
    keep <path> (<reason>)  the main worktree, the checkout this runs from, a
                            locked worktree whose folder exists, one with
                            modified, staged or untracked files, one whose
                            HEAD is not merged into REF, or any git error
    remove <path>           clean apart from ignored files, unlocked, merged;
                            applied with `git worktree remove` (never --force)
    prune <path>            registered, folder gone, unlocked
    prune-locked <path>     registered, folder gone, locked, and the folder's
                            parent exists (so the drive is mounted); applied
                            as unlock then prune. With the parent gone too it
                            is `keep`: that is what git's lock is for.
    unregistered <path>     a folder under <main>/.claude/worktrees/ git does
                            not list; reported with its size, never deleted
Branches are never deleted.

`trim` checks each present checkout's `apps/*/target` against the cap
(default 15 GiB): over it, the `<profile>/incremental` folders go first, and
the whole `target` only if it is still over. It holds that checkout's `build`
lock (tools/workorder_lock.py) while deleting, and prints `busy` and skips
the checkout when a build holds it. Sizing and deleting never follow a
symlink or a junction.

`check` exits 1 when the drive has less than N GiB free (default
DEFAULT_MIN_FREE_GIB), naming the commands that free space.

Exit code: 0 success; 1 an applied action failed, or `check` is below its
threshold; 2 usage (not a repository, unknown --into ref).
"""

from __future__ import annotations

import argparse
import os
import shutil
import stat
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import workorder_lock  # noqa: E402

DEFAULT_MIN_FREE_GIB = 30
DEFAULT_CAP_GIB = 15.0
DEFAULT_INTO = "origin/main"
GIB = 1024 ** 3
_REPARSE_POINT = getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0x400)
_NAME_SURROGATE = 0x20000000  # the reparse-tag bit Windows sets on every kind of link


class UsageError(Exception):
    pass


# --- sizes and free space -------------------------------------------------

def free_gib(path) -> float:
    """Free GiB on the drive holding `path` (its nearest existing ancestor)."""
    p = Path(path).resolve()
    while not p.exists() and p.parent != p:
        p = p.parent
    return shutil.disk_usage(str(p)).free / GIB


def human(size: int) -> str:
    value = float(size)
    for unit in ("B", "KiB", "MiB", "GiB"):
        if value < 1024 or unit == "GiB":
            return f"{int(value)} B" if unit == "B" else f"{value:.1f} {unit}"
        value /= 1024
    return f"{value:.1f} GiB"


def _is_link(path) -> bool:
    """A symlink, a junction or another name-surrogate reparse point (one that
    names somewhere else): never followed. Other reparse points, such as a
    OneDrive placeholder, are ordinary files and folders here."""
    try:
        st = os.lstat(path)
    except OSError:
        return False
    if stat.S_ISLNK(st.st_mode):
        return True
    if getattr(st, "st_file_attributes", 0) & _REPARSE_POINT and \
            getattr(st, "st_reparse_tag", 0) & _NAME_SURROGATE:
        return True
    isjunction = getattr(os.path, "isjunction", None)
    return bool(isjunction and isjunction(path))


def size_of(path) -> int:
    """Bytes under `path`, never following a link (a link counts as nothing)."""
    path = str(path)
    if _is_link(path):
        return 0
    if os.path.isfile(path):
        try:
            return os.lstat(path).st_size
        except OSError:
            return 0
    total = 0
    stack = [path]
    while stack:
        current = stack.pop()
        try:
            entries = list(os.scandir(current))
        except OSError:
            continue
        for entry in entries:
            if _is_link(entry.path):
                continue
            try:
                if entry.is_dir(follow_symlinks=False):
                    stack.append(entry.path)
                else:
                    total += entry.stat(follow_symlinks=False).st_size
            except OSError:
                continue
    return total


def delete_tree(path, errors: list) -> None:
    """Delete `path` without following any link inside it: a link is removed
    itself, never what it points at. Each failure is appended to `errors`."""
    path = str(path)

    def remove_link(p):
        try:
            if os.path.isdir(p):
                os.rmdir(p)  # a directory junction or symlink: removes the link only
            else:
                os.unlink(p)
        except OSError as exc:
            errors.append(f"{p}: {exc}")

    if _is_link(path):
        remove_link(path)
        return
    try:
        entries = list(os.scandir(path))
    except OSError as exc:
        errors.append(f"{path}: {exc}")
        return
    for entry in entries:
        if _is_link(entry.path):
            remove_link(entry.path)
        elif entry.is_dir(follow_symlinks=False):
            delete_tree(entry.path, errors)
        else:
            try:
                os.unlink(entry.path)
            except PermissionError:
                try:
                    os.chmod(entry.path, stat.S_IWRITE)
                    os.unlink(entry.path)
                except OSError as exc:
                    errors.append(f"{entry.path}: {exc}")
            except OSError as exc:
                errors.append(f"{entry.path}: {exc}")
    try:
        os.rmdir(path)
    except OSError as exc:
        errors.append(f"{path}: {exc}")


# --- git ------------------------------------------------------------------

def git(cwd, *args) -> subprocess.CompletedProcess:
    return subprocess.run(["git", "-C", str(cwd), *args], capture_output=True, text=True)


def norm(path) -> str:
    return os.path.normcase(os.path.normpath(os.path.abspath(str(path))))


def toplevel(repo) -> Path:
    r = git(repo, "rev-parse", "--show-toplevel")
    if r.returncode != 0 or not r.stdout.strip():
        raise UsageError(f"not a git repository: {repo}")
    return Path(r.stdout.strip())


def resolve_ref(repo, ref: str) -> str:
    r = git(repo, "rev-parse", "--verify", "--quiet", f"{ref}^{{commit}}")
    if r.returncode != 0 or not r.stdout.strip():
        raise UsageError(f"unknown --into ref: {ref}")
    return r.stdout.strip()


def list_worktrees(repo) -> list:
    """`git worktree list --porcelain -z` as dicts; the main worktree first."""
    r = git(repo, "worktree", "list", "--porcelain", "-z")
    if r.returncode != 0:
        raise UsageError(f"git worktree list failed: {r.stderr.strip()}")
    entries, current = [], {}
    for field in r.stdout.split("\0"):
        if not field:
            if current:
                entries.append(current)
                current = {}
            continue
        key, _, value = field.partition(" ")
        current[key] = value
    if current:
        entries.append(current)
    return [e for e in entries if "worktree" in e]


class Worktree:
    def __init__(self, entry: dict, index: int, current: str):
        self.path = entry["worktree"]
        self.head = entry.get("HEAD", "")
        branch = entry.get("branch", "")
        self.label = branch.removeprefix("refs/heads/") if branch else (self.head[:10] or "detached")
        self.main = index == 0 or "bare" in entry
        self.current = norm(self.path) == current
        self.locked = "locked" in entry
        self.present = os.path.isdir(self.path)
        self.parent_present = os.path.isdir(os.path.dirname(os.path.normpath(self.path)))

    def dirty(self):
        """True, False, or an error string."""
        r = git(self.path, "status", "--porcelain", "--untracked-files=normal")
        if r.returncode != 0:
            return f"git status failed: {r.stderr.strip() or r.returncode}"
        return bool(r.stdout.strip())

    def merged(self, repo, into_sha: str):
        """True, False, or an error string."""
        if not self.head:
            return "no HEAD"
        r = git(repo, "merge-base", "--is-ancestor", self.head, into_sha)
        if r.returncode in (0, 1):
            return r.returncode == 0
        return f"git merge-base failed: {r.stderr.strip() or r.returncode}"


def load(repo):
    """(checkout this runs from, main worktree path, [Worktree])."""
    top = toplevel(repo)
    entries = list_worktrees(top)
    current = norm(top)
    trees = [Worktree(e, i, current) for i, e in enumerate(entries)]
    main_path = Path(trees[0].path) if trees else top
    return top, main_path, trees


def unregistered(main_path: Path, trees: list) -> list:
    root = main_path / ".claude" / "worktrees"
    if not root.is_dir():
        return []
    known = {norm(t.path) for t in trees}
    out = []
    for entry in sorted(os.scandir(root), key=lambda e: e.name):
        if entry.is_dir(follow_symlinks=False) and norm(entry.path) not in known:
            out.append(Path(entry.path))
    return out


# --- worktrees --------------------------------------------------------------

def decide(tree: Worktree, repo, into_sha: str, into: str):
    """(action, reason) for one worktree."""
    if tree.main:
        return "keep", "main worktree"
    if tree.current:
        return "keep", "current checkout"
    if not tree.present:
        if not tree.locked:
            return "prune", "folder missing"
        if tree.parent_present:
            return "prune-locked", "locked, folder missing"
        return "keep", "locked, folder and its parent missing: drive not mounted?"
    if tree.locked:
        return "keep", "locked"
    dirty = tree.dirty()
    if isinstance(dirty, str):
        return "keep", dirty
    if dirty:
        return "keep", "modified, staged or untracked files"
    merged = tree.merged(repo, into_sha)
    if isinstance(merged, str):
        return "keep", merged
    if not merged:
        return "keep", f"not merged into {into}"
    return "remove", f"clean, merged into {into}"


def still_listed(repo, path: str) -> bool:
    return any(norm(e["worktree"]) == norm(path) for e in list_worktrees(repo))


def apply_action(action: str, tree: Worktree, main_path: Path):
    """None on success, else the failure text."""
    if action == "remove":
        r = git(main_path, "worktree", "remove", tree.path)
        return None if r.returncode == 0 else (r.stderr.strip() or f"exit {r.returncode}")
    if action == "prune-locked":
        r = git(main_path, "worktree", "unlock", tree.path)
        if r.returncode != 0:
            return r.stderr.strip() or f"unlock exit {r.returncode}"
    if action in ("prune", "prune-locked"):
        r = git(main_path, "worktree", "prune")
        if r.returncode != 0:
            return r.stderr.strip() or f"prune exit {r.returncode}"
        return "still registered after prune" if still_listed(main_path, tree.path) else None
    return None


def cmd_worktrees(args) -> int:
    top, main_path, trees = load(args.repo)
    into_sha = resolve_ref(top, args.into)
    failed = 0
    for tree in trees:
        action, reason = decide(tree, top, into_sha, args.into)
        if action == "keep":
            print(f"keep {tree.path} ({reason})")
            continue
        line = f"{action} {tree.path} ({tree.label}; {reason})"
        if not args.apply:
            print(line)
            continue
        error = apply_action(action, tree, main_path)
        if error is None:
            print(f"{line}: done")
        else:
            failed += 1
            print(f"{line}: failed: {error}")
    for folder in unregistered(main_path, trees):
        print(f"unregistered {folder} ({human(size_of(folder))}; not a registered worktree, left alone)")
    if not args.apply:
        print("dry run: nothing changed; rerun with --apply to act on the lines above. Branches are never deleted.")
    elif failed:
        print(f"{failed} action(s) failed")
    return 1 if failed else 0


# --- report -----------------------------------------------------------------

def app_dirs(checkout, name: str) -> list:
    apps = Path(checkout) / "apps"
    if not apps.is_dir():
        return []
    out = []
    for app in sorted(apps.iterdir(), key=lambda p: p.name):
        candidate = app / name
        if app.is_dir() and not _is_link(app) and candidate.is_dir():
            out.append(candidate)
    return out


def cmd_report(args) -> int:
    top, main_path, trees = load(args.repo)
    into_sha = resolve_ref(top, args.into)
    for tree in trees:
        flags = []
        if tree.main:
            flags.append("main")
        if tree.current:
            flags.append("current")
        if tree.locked:
            flags.append("locked")
        if not tree.present:
            flags.append("folder missing")
        else:
            dirty = tree.dirty()
            if dirty is True:
                flags.append("dirty")
            elif isinstance(dirty, str):
                flags.append("status error")
        merged = tree.merged(top, into_sha)
        if merged is True:
            flags.append(f"merged into {args.into}")
        elif merged is False:
            flags.append(f"not merged into {args.into}")
        print(f"{tree.path} [{tree.label}] {', '.join(flags)}")
        if not tree.present:
            continue
        total = 0
        for name in ("target", "node_modules"):
            for folder in app_dirs(tree.path, name):
                size = size_of(folder)
                total += size
                print(f"  {folder.relative_to(tree.path).as_posix()}: {human(size)}")
        print(f"  build output and node_modules: {human(total)}")
    for folder in unregistered(main_path, trees):
        print(f"unregistered {folder}: {human(size_of(folder))}")
    anchor = Path(top).resolve().anchor or str(top)
    print(f"{free_gib(top):.1f} GiB free on {anchor}")
    return 0


# --- trim -------------------------------------------------------------------

def incremental_dirs(target: Path) -> list:
    """`<profile>/incremental` and `<triple>/<profile>/incremental` folders."""
    out = []
    for level1 in sorted(target.iterdir(), key=lambda p: p.name) if target.is_dir() else []:
        if _is_link(level1) or not level1.is_dir():
            continue
        if level1.name == "incremental":
            continue
        inc = level1 / "incremental"
        if inc.is_dir() and not _is_link(inc):
            out.append(inc)
        for level2 in sorted(level1.iterdir(), key=lambda p: p.name):
            inc2 = level2 / "incremental"
            if level2.is_dir() and not _is_link(level2) and inc2.is_dir() and not _is_link(inc2):
                out.append(inc2)
    return out


def plan_trim(checkout, cap: int) -> list:
    """[(target, size, [(path, size) to delete], whole)] for targets over cap."""
    plans = []
    for target in app_dirs(checkout, "target"):
        if _is_link(target):
            continue
        size = size_of(target)
        if size <= cap:
            plans.append((target, size, [], False))
            continue
        inc = [(p, size_of(p)) for p in incremental_dirs(target)]
        remaining = size - sum(s for _, s in inc)
        plans.append((target, size, inc, remaining > cap))
    return plans


def cmd_trim(args) -> int:
    top, _main_path, trees = load(args.repo)
    cap = int(args.cap_gib * GIB)
    errors_total = 0
    for tree in trees:
        if not tree.present:
            continue
        plans = plan_trim(tree.path, cap)
        over = [p for p in plans if p[2] or p[3]]
        for target, size, _inc, _whole in plans:
            if not _inc and not _whole:
                print(f"ok {target} ({human(size)}, cap {human(cap)})")
        if not over:
            continue
        if not args.apply:
            for target, size, inc, whole in over:
                for path, s in inc:
                    print(f"trim {path} ({human(s)}; {target} is {human(size)}, over cap {human(cap)})")
                if whole:
                    print(f"trim {target} (still over cap {human(cap)} without incremental)")
            continue
        with workorder_lock.held("build", workorder_lock.lock_dir(tree.path), timeout=0) as name:
            if name is None:
                print(f"busy {tree.path} (its build lock is held; skipped)")
                continue
            for target, size, _inc, _whole in over:
                errors = []
                for path in incremental_dirs(target):
                    s, before = size_of(path), len(errors)
                    delete_tree(path, errors)
                    print(f"trim {path} ({human(s)}): {'failed' if len(errors) > before else 'done'}")
                after = size_of(target) if target.exists() else 0
                if after > cap:
                    before = len(errors)
                    delete_tree(target, errors)
                    print(f"trim {target} ({human(after)}, still over cap {human(cap)}): "
                          f"{'failed' if len(errors) > before else 'done'}")
                for error in errors:
                    print(f"  failed: {error}")
                errors_total += len(errors)
    if not args.apply:
        print("dry run: nothing changed; rerun with --apply to delete the lines marked trim.")
    elif errors_total:
        print(f"{errors_total} deletion error(s)")
    return 1 if errors_total else 0


# --- check ------------------------------------------------------------------

def cmd_check(args) -> int:
    top = toplevel(args.repo)
    free = free_gib(top)
    anchor = Path(top).resolve().anchor or str(top)
    if free >= args.min_free_gib:
        print(f"ok: {free:.1f} GiB free on {anchor} (threshold {args.min_free_gib:g} GiB)")
        return 0
    print(f"low: {free:.1f} GiB free on {anchor}, below {args.min_free_gib:g} GiB. "
          f"See what is using it with `py -3 tools/disk_hygiene.py report`, then free it with "
          f"`py -3 tools/disk_hygiene.py worktrees` and `py -3 tools/disk_hygiene.py trim` "
          f"(dry runs; add --apply).")
    return 1


# --- main -------------------------------------------------------------------

def parser() -> argparse.ArgumentParser:
    common = argparse.ArgumentParser(add_help=False)
    common.add_argument("--repo", default=argparse.SUPPRESS, help="a directory in the checkout (default: .)")
    p = argparse.ArgumentParser(prog="disk_hygiene.py", description=__doc__.split("\n\n")[0])
    p.add_argument("--repo", default=".", help="a directory in the checkout (default: .)")
    sub = p.add_subparsers(dest="command", required=True)
    s = sub.add_parser("report", parents=[common], help="sizes, flags and free space")
    s.add_argument("--into", default=DEFAULT_INTO)
    s = sub.add_parser("worktrees", parents=[common], help="remove merged worktrees, prune stale ones")
    s.add_argument("--into", default=DEFAULT_INTO)
    s.add_argument("--apply", action="store_true")
    s = sub.add_parser("trim", parents=[common], help="trim apps/*/target over a cap")
    s.add_argument("--cap-gib", type=float, default=DEFAULT_CAP_GIB)
    s.add_argument("--apply", action="store_true")
    s = sub.add_parser("check", parents=[common], help="exit 1 below a free-space threshold")
    s.add_argument("--min-free-gib", type=float, default=DEFAULT_MIN_FREE_GIB)
    return p


def main(argv=None) -> int:
    args = parser().parse_args(argv)
    handler = {"report": cmd_report, "worktrees": cmd_worktrees, "trim": cmd_trim, "check": cmd_check}
    try:
        return handler[args.command](args)
    except UsageError as exc:
        print(f"disk_hygiene: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
