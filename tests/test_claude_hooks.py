"""The PostToolUse hooks in .claude/hooks/ must actually block.

`AGENTS.md` § "Prove the Instrument Before Trusting a Negative Result" is the
reason this file exists. A hook that exits 0 on a file it should have flagged
is indistinguishable from a hook that is working, and in the sister project
these hooks were ported from, every one of them did exactly that during
development: they exited 0 against a harness feeding them paths Windows Python
could not resolve, and looked healthy while guarding nothing.

So every test here is a pair. A *positive control* proves the hook fires on a
real violation, and a negative control proves it stays quiet on a clean tree.
A suite with only the negative half would pass against hooks that do nothing.

The rig is a throwaway git repository in the system temp directory, not the
session scratchpad -- `git init` there fails with `Filename too long`.
"""

import importlib.util
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
HOOKS = REPO / ".claude" / "hooks"
SETTINGS = REPO / ".claude" / "settings.json"

SKIP_VAR = "EP2_SKIP_HOOKS"


def _hooks_source_dir():
    """The directory `HookRig` and `_load_hook_module` load hook scripts from.

    `EP2_HOOK_UNDER_TEST_DIR` swaps in a scratchpad copy -- a mutant, or an
    older revision fetched with `git show` -- without ever overwriting the
    live `.claude/hooks/` the running session's own hooks execute from."""
    override = os.environ.get("EP2_HOOK_UNDER_TEST_DIR")
    return Path(override) if override else HOOKS


def _load_hook_module(name):
    """Exec a `.claude/hooks/<name>.py` in isolation, for unit-level tests of
    its pure functions (as opposed to `HookRig`, which drives the hook as a
    subprocess the way `settings.json` actually invokes it). Not added to
    `sys.modules` -- each call gets its own copy."""
    spec = importlib.util.spec_from_file_location(name, _hooks_source_dir() / f"{name}.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


CLEAN_SOURCE = """\
// Hook installed at a measured offset; see docs/research for the byte pattern.
#include <windows.h>
DWORD WINAPI worker(LPVOID) { return 0; }
"""

# A line every positive control below can reuse: a Ghidra auto-generated
# symbol inside a decompiler-shaped statement.
GHIDRA_LISTING = "clean line\n    iVar1 = FUN_00b489070(param_1);\n"

CRASHING_CHECK = "def check(payload, tree):\n    raise RuntimeError('boom')\n"


def _git(*args, cwd):
    return subprocess.run(
        ["git", *args], cwd=cwd, capture_output=True, text=True, check=True
    )


class HookRig:
    """A minimal git repo laid out like this repository, with the hooks copied in."""

    def __init__(self):
        self.root = Path(tempfile.mkdtemp(prefix="ep2-hooks-"))
        shutil.copytree(_hooks_source_dir(), self.root / ".claude" / "hooks",
                        ignore=shutil.ignore_patterns("__pycache__"))
        # The real settings.json, so the rig is laid out like the repository.
        shutil.copy2(SETTINGS, self.root / ".claude" / "settings.json")

        # Real files so the clean-tree controls are meaningful.
        self.write("src/mod.cpp", CLEAN_SOURCE)
        self.write("docs/notes.md", "# Notes\n")

        _git("init", "-q", cwd=self.root)
        _git("config", "user.email", "t@example.com", cwd=self.root)
        _git("config", "user.name", "T", cwd=self.root)
        _git("config", "core.autocrlf", "false", cwd=self.root)
        _git("add", "-A", cwd=self.root)
        _git("commit", "-qm", "base", cwd=self.root)

    def write(self, rel, text):
        path = self.root / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(text.encode("utf-8"))

    def install_check(self, name, body, position=0):
        """Add a check module `name` to the rig's copy of the hooks and splice
        it into the rig's copy of `post_tool_use.TREE_CHECKS` at `position`.

        With one real check in the repository, this is how the dispatcher's
        multi-check behaviour (ordering, crash isolation, worst exit code)
        stays proven. Only the rig's throwaway copy is ever rewritten."""
        hooks = self.root / ".claude" / "hooks"
        (hooks / f"{name}.py").write_bytes(body.encode("utf-8"))
        dispatcher = hooks / "post_tool_use.py"
        text = dispatcher.read_text(encoding="utf-8")
        module = _load_hook_module("post_tool_use")
        checks = list(module.TREE_CHECKS)
        checks.insert(position, name)
        anchor = "TREE_CHECKS = ("
        start = text.index(anchor)
        end = text.index(")", start)
        replacement = "TREE_CHECKS = (" + "".join(f"\n    {c!r}," for c in checks) + "\n"
        dispatcher.write_text(text[:start] + replacement + text[end:], encoding="utf-8")

    def run(self, hook, payload=None, env=None, args=(), raw_input=None):
        environ = dict(os.environ)
        environ.pop(SKIP_VAR, None)
        environ.update(env or {})
        if raw_input is None:
            raw_input = json.dumps(payload if payload is not None else {"tool_name": "Bash"})
        return subprocess.run(
            [sys.executable, str(self.root / ".claude" / "hooks" / hook), *args],
            cwd=self.root,
            input=raw_input,
            capture_output=True,
            text=True,
            env=environ,
        )

    def destroy(self):
        shutil.rmtree(self.root, ignore_errors=True)


class HookTestCase(unittest.TestCase):
    def setUp(self):
        self.rig = HookRig()
        self.addCleanup(self.rig.destroy)


def _add_submodule(test, rig, name="vendor-mod"):
    """Register a throwaway repository as a submodule of `rig` and return its
    checkout. Skips the test when this git refuses file-protocol submodules."""
    origin = Path(tempfile.mkdtemp(prefix="ep2-sub-"))
    test.addCleanup(shutil.rmtree, origin, ignore_errors=True)
    _git("init", "-q", cwd=origin)
    _git("config", "user.email", "t@example.com", cwd=origin)
    _git("config", "user.name", "T", cwd=origin)
    (origin / "docs").mkdir()
    (origin / "docs" / "research.md").write_bytes(b"clean\n")
    _git("add", "-A", cwd=origin)
    _git("commit", "-qm", "base", cwd=origin)
    try:
        _git(
            "-c", "protocol.file.allow=always",
            "submodule", "add", "-q", origin.as_uri(), name,
            cwd=rig.root,
        )
        _git("commit", "-qm", "add submodule", cwd=rig.root)
    except subprocess.CalledProcessError as exc:
        raise unittest.SkipTest(f"git submodule add unavailable: {exc.stderr}")
    return rig.root / name


class TestDecompiledOutput(HookTestCase):
    """The one hook whose failure is not fixable by a later release: once
    decompiled game code has reached a remote it cannot be taken back. So this
    suite carries more negative controls than positive ones -- a hook that
    flags ordinary research notes (names, offsets, byte patterns) would be
    switched off within a day, and a switched-off hook guards nothing.
    """

    HOOK = "decompiled_output.py"

    # Every one of these is explicitly blessed by AGENTS.md as an
    # interoperability fact. If the patterns ever start matching them, the hook
    # has become worse than useless.
    ALLOWED = """\
# Flask charge research

`ItemFlask::OnUse` is reached from the player's action handler.
We hook it by byte pattern `48 89 5C 24 ? 57 48 83 EC 20` at a measured RVA.
Measured: 1204 calls, 0 with a null owner, before the null-check fix.
Offsets: +0x18 is the charge count (DWORD); calling convention is __fastcall.
Our own code: `auto* p = reinterpret_cast<DWORD*>(base + 0x18);`
Schema: `Flasks.datc64` row is 0x40 bytes; `undefined behaviour` is avoided.
A stack var named in_stack is fine, and so is `unaff` in prose.
"""

    def test_clean_tree_is_silent(self):
        """Negative control: nothing changed, nothing said."""
        result = self.rig.run(self.HOOK)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_names_offsets_and_measurements_do_not_block(self):
        """The load-bearing negative control -- see this class's docstring."""
        self.rig.write("docs/flask-research.md", self.ALLOWED)
        result = self.rig.run(self.HOOK)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_ghidra_symbol_blocks(self):
        self.rig.write(
            "docs/notes.md",
            "The handler:\n\n    iVar1 = FUN_00b489070(param_1);\n",
        )
        result = self.rig.run(self.HOOK)
        self.assertEqual(result.returncode, 2, result.stdout)
        self.assertIn("Ghidra", result.stderr)

    def test_ida_symbol_blocks(self):
        self.rig.write("docs/notes.md", "call    sub_140A3B7C0\n")
        result = self.rig.run(self.HOOK)
        self.assertEqual(result.returncode, 2, result.stdout)
        self.assertIn("IDA", result.stderr)

    def test_ghidra_type_declaration_blocks(self):
        self.rig.write("docs/notes.md", "undefined8 result;\n")
        result = self.rig.run(self.HOOK)
        self.assertEqual(result.returncode, 2, result.stdout)

    def test_ghidra_register_pseudo_variable_blocks(self):
        self.rig.write("docs/notes.md", "  lVar2 = *(longlong *)(unaff_RBX + 0x18);\n")
        result = self.rig.run(self.HOOK)
        self.assertEqual(result.returncode, 2, result.stdout)
        self.assertIn("register/stack pseudo-variable", result.stderr)

    def test_ghidra_stack_pseudo_variable_blocks(self):
        self.rig.write("docs/notes.md", "  uVar3 = in_stack_00000028;\n")
        result = self.rig.run(self.HOOK)
        self.assertEqual(result.returncode, 2, result.stdout)

    def test_hex_rays_typed_cast_blocks(self):
        self.rig.write("docs/notes.md", "  v4 = *(_QWORD *)(a1 + 24);\n")
        result = self.rig.run(self.HOOK)
        self.assertEqual(result.returncode, 2, result.stdout)
        self.assertIn("Hex-Rays", result.stderr)

    def test_windows_typed_cast_does_not_block(self):
        """Negative control for the Hex-Rays cast: Windows' own `DWORD` has no
        leading underscore, and our code uses it."""
        self.rig.write("src/mod.cpp", CLEAN_SOURCE + "auto v = *(DWORD *)(base + 24);\n")
        result = self.rig.run(self.HOOK)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_listing_in_source_blocks_not_only_markdown(self):
        """A comment in a .cpp is as tracked as a docs page."""
        self.rig.write(
            "src/note.cpp",
            "// original: iVar1 = FUN_00b489070(param_1);\n",
        )
        result = self.rig.run(self.HOOK)
        self.assertEqual(result.returncode, 2, result.stdout)

    def test_guard_machinery_is_not_flagged_by_its_own_patterns(self):
        """`.claude/` holds the patterns themselves; see the hook's docstring."""
        self.rig.write(".claude/agents/note.md", "matches FUN_00b489070 too\n")
        result = self.rig.run(self.HOOK)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_binary_asset_is_not_scanned(self):
        self.rig.write("docs/shot.png", "FUN_00b489070\n")
        result = self.rig.run(self.HOOK)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_skip_switch_silences_it(self):
        self.rig.write("docs/notes.md", "call    sub_140A3B7C0\n")
        blocked = self.rig.run(self.HOOK)
        self.assertEqual(blocked.returncode, 2, blocked.stdout)
        self.assertIn(SKIP_VAR, blocked.stderr, "the blocking message names its way out")
        allowed = self.rig.run(self.HOOK, env={SKIP_VAR: "1"})
        self.assertEqual(allowed.returncode, 0, allowed.stderr)

    # -- added lines only -------------------------------------------------
    #
    # The first version of this hook (in the sister project) matched whole
    # file contents, and committed files there already carried matches.
    # Appending a paragraph to one of them made every subsequent tool call
    # exit 2 until someone set the skip variable -- the exact outcome the
    # hook's docstring says it avoids. These tests are a pair: the hook must
    # stop wedging, and grandfathering must not become a loophole.

    LEGACY = "Old finding\n\n    iVar1 = FUN_00b489070(param_1);\n"

    def _commit_legacy(self):
        self.rig.write("docs/legacy-research.md", self.LEGACY)
        _git("add", "-A", cwd=self.rig.root)
        _git("commit", "-qm", "legacy research", cwd=self.rig.root)

    def test_appending_to_a_file_that_already_contains_a_listing_is_silent(self):
        self._commit_legacy()
        self.rig.write(
            "docs/legacy-research.md",
            self.LEGACY + "\nMeasured 1204 calls through the same path.\n",
        )
        result = self.rig.run(self.HOOK)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_a_new_listing_added_to_such_a_file_still_blocks(self):
        """Grandfathering is scoped to what is in HEAD, not to the file."""
        self._commit_legacy()
        self.rig.write(
            "docs/legacy-research.md",
            self.LEGACY + "\nand then: call    sub_140A3B7C0\n",
        )
        result = self.rig.run(self.HOOK)
        self.assertEqual(result.returncode, 2, result.stdout)
        self.assertIn("IDA", result.stderr)

    def test_deleting_a_line_containing_a_listing_is_silent(self):
        self._commit_legacy()
        self.rig.write("docs/legacy-research.md", "Old finding\n")
        result = self.rig.run(self.HOOK)
        self.assertEqual(result.returncode, 0, result.stderr)


class TestDecompiledOutputInSubmodules(HookTestCase):
    """A top-level `git status` reports a dirty submodule as one changed
    pointer -- never as the files inside it. Should a submodule ever be added,
    scanning only the top level would leave its `docs/` unguarded, which is
    precisely the "looked installed and healthy while guarding nothing"
    failure this suite exists to prevent.
    """

    HOOK = "decompiled_output.py"

    def setUp(self):
        super().setUp()
        self.sub = _add_submodule(self, self.rig)

    def test_clean_submodule_is_silent(self):
        """Negative control: a registered, unmodified submodule says nothing."""
        result = self.rig.run(self.HOOK)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_listing_inside_a_dirty_submodule_blocks(self):
        (self.sub / "docs" / "research.md").write_bytes(
            b"the collect path:\n\n    iVar1 = FUN_00b489070(param_1);\n"
        )
        result = self.rig.run(self.HOOK)
        self.assertEqual(result.returncode, 2, result.stdout)
        self.assertIn("vendor-mod/docs/research.md", result.stderr)

    def test_clean_change_inside_a_submodule_does_not_block(self):
        (self.sub / "docs" / "research.md").write_bytes(
            b"Measured 1204 calls through ItemFlask::OnUse.\n"
        )
        result = self.rig.run(self.HOOK)
        self.assertEqual(result.returncode, 0, result.stderr)


class GrandfatheredWholeFileInventory(unittest.TestCase):
    """Pins the whole-file-match inventory `decompiled_output.py`'s docstring
    describes ("Added lines only, and why that is not a loophole") to a
    checked fact. Runs the hook's own `SIGNATURES`, via its own `inspect`,
    over whole file contents (not added lines: the claim is about what already
    sits in committed files, which is exactly what the hook itself no longer
    reads) of this repository's tree at HEAD, excluding `EXCLUDED_PREFIXES`.

    The expected inventory is empty. If a committed file ever matches, either
    it is a violation to remove, or it is a deliberate grandfathered path to
    add to `EXPECTED` together with the docstring -- citing this test by name
    rather than a bare count.
    """

    EXPECTED = frozenset()

    @classmethod
    def setUpClass(cls):
        cls.hook = _load_hook_module("decompiled_output")

    def _matches(self, display, blob_bytes):
        text = blob_bytes.decode("utf-8", "replace")
        lines = list(enumerate(text.splitlines(), start=1))
        return bool(self.hook.inspect(display, lines))

    def _scan_tree(self):
        found = set()
        listing = subprocess.run(
            ["git", "ls-tree", "-r", "--name-only", "HEAD"],
            cwd=REPO, capture_output=True, text=True)
        if listing.returncode != 0:
            self.skipTest("no HEAD to scan")
        for rel in listing.stdout.splitlines():
            if rel.startswith(self.hook.EXCLUDED_PREFIXES):
                continue
            if not rel.endswith(self.hook.WATCHED_SUFFIXES):
                continue
            blob = subprocess.run(
                ["git", "show", f"HEAD:{rel}"], cwd=REPO, capture_output=True)
            if blob.returncode != 0:
                continue
            if self._matches(rel, blob.stdout):
                found.add(rel)
        return found

    def test_the_committed_tree_carries_no_unexpected_whole_file_match(self):
        found = self._scan_tree()
        self.assertEqual(
            found, set(self.EXPECTED),
            f"whole-file decompiled-output signature matches in committed files: "
            f"{sorted(found)}; expected: {sorted(self.EXPECTED)}")


class TestPostToolUseDispatcher(HookTestCase):
    """`.claude/hooks/post_tool_use.py`: one process running every tree
    check. Each pair here mirrors the positive/negative shape this file opens
    with -- a clean tree stays silent, and the real check's violation still
    blocks when reached through the dispatcher, not just standalone. The
    multi-check behaviour (crash isolation, ordering, worst exit code) is
    proven with a fake check the rig splices into its own copy of
    `TREE_CHECKS`.
    """

    HOOK = "post_tool_use.py"

    def test_every_tree_check_is_a_module_with_a_check_function(self):
        dispatcher = _load_hook_module("post_tool_use")
        self.assertIn("decompiled_output", dispatcher.TREE_CHECKS)
        for name in dispatcher.TREE_CHECKS:
            module = _load_hook_module(name)
            self.assertTrue(callable(getattr(module, "check", None)), name)

    # -- negative control -----------------------------------------------

    def test_clean_tree_is_silent(self):
        result = self.rig.run(self.HOOK)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, "")
        self.assertEqual(result.stderr, "")

    # -- the real check, run through the dispatcher ---------------------

    def test_decompiled_output_violation_blocks_through_it(self):
        self.rig.write("docs/notes.md", GHIDRA_LISTING)
        result = self.rig.run(self.HOOK)
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertIn("Decompiled or disassembled game source", result.stderr)
        self.assertEqual(result.stdout, "")

    # -- tool_name gating -------------------------------------------------

    def test_edit_tool_runs_the_tree_checks(self):
        self.rig.write("docs/notes.md", GHIDRA_LISTING)
        result = self.rig.run(self.HOOK, payload={"tool_name": "Edit"})
        self.assertEqual(result.returncode, 2, result.stderr)

    def test_codex_apply_patch_runs_the_tree_checks(self):
        self.rig.write("docs/notes.md", GHIDRA_LISTING)
        result = self.rig.run(self.HOOK, payload={"tool_name": "apply_patch"})
        self.assertEqual(result.returncode, 2, result.stderr)

    def test_read_tool_runs_nothing(self):
        self.rig.write("docs/notes.md", GHIDRA_LISTING)
        result = self.rig.run(self.HOOK, payload={"tool_name": "Read"})
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stderr, "")

    def test_malformed_stdin_still_runs_the_tree_checks(self):
        """A `tool_name` the dispatcher cannot read runs the tree checks
        anyway -- the fail-safe direction."""
        clean = self.rig.run(self.HOOK, raw_input="{not json")
        self.assertEqual(clean.returncode, 0, clean.stderr)
        self.rig.write("docs/notes.md", GHIDRA_LISTING)
        dirty = self.rig.run(self.HOOK, raw_input="{not json")
        self.assertEqual(dirty.returncode, 2, dirty.stderr)

    def test_skip_hooks_silences_the_tree_checks(self):
        self.rig.write("docs/notes.md", GHIDRA_LISTING)
        result = self.rig.run(self.HOOK, env={SKIP_VAR: "1"})
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stderr, "")

    # -- a crash in one check must not mask, or block for, the others ---

    def test_crash_in_one_check_does_not_mask_a_real_violation(self):
        self.rig.install_check("crashing_check", CRASHING_CHECK, position=0)
        self.rig.write("docs/notes.md", GHIDRA_LISTING)
        result = self.rig.run(self.HOOK)
        self.assertEqual(result.returncode, 2, result.stderr)  # the real violation still blocks
        self.assertIn("crashing_check crashed", result.stderr)
        self.assertIn("RuntimeError", result.stderr)
        self.assertIn("Decompiled or disassembled game source", result.stderr)
        # Forwarded in the order run.
        self.assertLess(result.stderr.index("crashing_check crashed"),
                        result.stderr.index("Decompiled or disassembled"))

    def test_crash_alone_is_non_blocking(self):
        """A check's own bug is an error (rc 1), not a block (rc 2) -- it must
        never gain the enforcement power of a real finding."""
        self.rig.install_check("crashing_check", CRASHING_CHECK)
        result = self.rig.run(self.HOOK)
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertIn("crashing_check crashed", result.stderr)

    def test_exit_code_is_the_worst_over_every_check(self):
        """A non-blocking note from one check plus a block from another is a
        block, and both messages reach stderr."""
        self.rig.install_check(
            "noting_check",
            "def check(payload, tree):\n    return 1, 'a non-blocking note'\n",
            position=1,
        )
        self.rig.write("docs/notes.md", GHIDRA_LISTING)
        result = self.rig.run(self.HOOK)
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertIn("a non-blocking note", result.stderr)
        self.assertLess(result.stderr.index("Decompiled or disassembled"),
                        result.stderr.index("a non-blocking note"))


def _dispatcher_order():
    return tuple(f"{name}.py" for name in _load_hook_module("post_tool_use").TREE_CHECKS)


class TestDispatcherEquivalence(HookTestCase):
    """Permanent equivalence check, not a one-off measurement: every tree
    check run standalone, in `TREE_CHECKS` order, must produce byte-identical
    stderr and the same exit code as running them through `post_tool_use.py`.
    This is what keeps each check's `check(payload, tree)` body and the
    dispatcher's own concatenation from drifting silently apart from what
    each script's standalone `main()` still does.
    """

    def _run_originals(self, payload=None, env=None, raw_input=None):
        out, rc = "", 0
        for hook in _dispatcher_order():
            result = self.rig.run(hook, payload=payload, env=env, raw_input=raw_input)
            self.assertEqual(result.stdout, "", (hook, result.stdout))
            out += result.stderr
            rc = max(rc, result.returncode)
        return rc, out

    def _run_dispatcher(self, payload=None, env=None, raw_input=None):
        result = self.rig.run("post_tool_use.py", payload=payload, env=env, raw_input=raw_input)
        self.assertEqual(result.stdout, "", result.stdout)
        return result.returncode, result.stderr

    def _assert_equivalent(self, payload=None, env=None, raw_input=None):
        original = self._run_originals(payload, env, raw_input)
        dispatched = self._run_dispatcher(payload, env, raw_input)
        self.assertEqual(original, dispatched)
        return original[0]

    def test_clean_tree_is_equivalent(self):
        self.assertEqual(self._assert_equivalent(), 0)

    def test_several_violations_at_once_is_equivalent(self):
        self.rig.write("docs/notes.md", GHIDRA_LISTING)
        self.rig.write("docs/new note.md", "call    sub_140A3B7C0\n")
        self.rig.write("src/hook.cpp", "// v4 = *(_QWORD *)(a1 + 24);\n")
        self.assertEqual(self._assert_equivalent(), 2)

    def test_skip_hooks_is_equivalent(self):
        self.rig.write("docs/notes.md", GHIDRA_LISTING)
        self.assertEqual(self._assert_equivalent(env={SKIP_VAR: "1"}), 0)

    def test_malformed_stdin_is_equivalent(self):
        self.rig.write("docs/notes.md", GHIDRA_LISTING)
        self.assertEqual(self._assert_equivalent(raw_input="{not json"), 2)


class TestDispatcherEquivalenceInSubmodules(HookTestCase):
    """The submodule-scanning half of the equivalence claim: a listing inside
    a dirty submodule, and a submodule whose pointer moved but whose own tree
    is clean (the case `TreeState.dirty_submodules` must not over- or
    under-report).
    """

    def setUp(self):
        super().setUp()
        self.sub = _add_submodule(self, self.rig)

    def _assert_equivalent(self):
        out_o, rc_o = "", 0
        for hook in _dispatcher_order():
            result = self.rig.run(hook)
            out_o += result.stderr
            rc_o = max(rc_o, result.returncode)
        result_p = self.rig.run("post_tool_use.py")
        self.assertEqual((rc_o, out_o), (result_p.returncode, result_p.stderr))
        return rc_o

    def test_listing_inside_dirty_submodule_is_equivalent(self):
        (self.sub / "docs" / "research.md").write_bytes(
            b"the collect path:\n\n    iVar1 = FUN_00b489070(param_1);\n"
        )
        self.assertEqual(self._assert_equivalent(), 2)

    def test_submodule_pointer_moved_but_clean_is_equivalent(self):
        (self.sub / "docs" / "research.md").write_bytes(b"clean v2\n")
        _git("-c", "user.email=t@example.com", "-c", "user.name=T", "commit", "-qam", "bump", cwd=self.sub)
        self.assertEqual(self._assert_equivalent(), 0)


if __name__ == "__main__":
    unittest.main()
