#!/usr/bin/env python3
"""Single PostToolUse dispatcher.

Runs every tree check in one interpreter, sharing one git plan
(`_common.TreeState`) instead of one `py -3` launch and one `git status` per
check. AGENTS.md § "Some of These Rules Are Enforced, Not Just Written" names
the checks; today `TREE_CHECKS` holds one, `decompiled_output`. This file only
changes how many processes run them, never what they check. Each check keeps
a `check(payload, tree) -> (rc, message)` body its own standalone `main()`
still calls, so `py -3 .claude/hooks/decompiled_output.py`, for example,
behaves exactly as it does through here -- see `tests/test_claude_hooks.py`'s
`TestPostToolUseDispatcher` and `TestDispatcherEquivalence`. Adding a check
means adding a module with that `check` shape and naming it in `TREE_CHECKS`.

Gating reproduces the PostToolUse matcher group `settings.json` runs this
hook under, `Edit|Write|Bash|PowerShell|NotebookEdit`:
  - TREE_TOOLS = {Edit, Write, Bash, PowerShell, NotebookEdit, apply_patch},
    or an unrecognised `tool_name` (a malformed payload runs the tree checks
    too -- the fail-safe direction) -> the tree checks.
`apply_patch` is Codex's file-edit tool: `.codex/hooks.json` runs this same
dispatcher, and Codex names its edits that way.
`EP2_SKIP_HOOKS` short-circuits the tree checks.

Each check runs inside its own `try/except`: a crash is reported as its own
non-blocking line (rc 1) and never masks, or blocks for, the others -- Claude
Code semantics are exit 2 = block, other non-zero = non-blocking stderr note,
0 = silent. Every part's stderr is forwarded verbatim, in the order run;
stdout stays empty; the exit code is the max over every part.
"""

import importlib
import json
import sys
import traceback
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import _common  # noqa: E402

TREE_TOOLS = {"Edit", "Write", "Bash", "PowerShell", "NotebookEdit", "apply_patch"}

# Run in this order; each name is a module in this directory exposing
# `check(payload, tree) -> (rc, message)`.
TREE_CHECKS = (
    "decompiled_output",
)


def _run_check(name: str, payload, tree: "_common.TreeState") -> tuple[str, int, str]:
    """(name, rc, message) for one check, isolating a bug in that check's own
    code to its own line. A crash must never turn into a block (rc 2) --
    that would mean one check's exception silently gains the enforcement
    power of every check -- and must never swallow the checks after it."""
    try:
        module = importlib.import_module(name)
        rc, message = module.check(payload, tree)
        return name, rc or 0, message or ""
    except Exception:  # noqa: BLE001 - isolate the crash, do not re-raise
        return name, 1, f"{name} crashed:\n{traceback.format_exc()}"


def main() -> int:
    raw = sys.stdin.read()
    try:
        payload = json.loads(raw)
        if not isinstance(payload, dict):
            payload = None
    except ValueError:
        payload = None
    tool = payload.get("tool_name") if payload else None

    parts: list[tuple[str, int, str]] = []

    # Tree checks -- for the tools the matcher names, or an unrecognised
    # tool_name (a malformed payload runs them too: the fail-safe direction).
    if tool is None or tool in TREE_TOOLS:
        if not _common.skip_requested():
            root = _common.repo_root()
            if root is not None:
                tree = _common.TreeState(root)
                for name in TREE_CHECKS:
                    parts.append(_run_check(name, payload, tree))

    worst = 0
    for _name, rc, message in parts:
        if message:
            sys.stderr.write(message if message.endswith("\n") else message + "\n")
        worst = max(worst, rc)
    return worst


if __name__ == "__main__":
    sys.exit(main())
