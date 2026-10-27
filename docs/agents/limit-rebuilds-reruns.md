Story and evidence behind `AGENTS.md` § ["Limit Rebuilds & Reruns During Development"](../../AGENTS.md#limit-rebuilds--reruns-during-development).

> From the sister project [hero-siege-offline-toolkit](https://github.com/falorfrozen-cmd/hero-siege-offline-toolkit)
> (Hero Siege, a GameMaker game): the names below are that project's, kept as evidence.

## Pet Quest Collector's Phase 0 research (2026-09-10)

**This was not respected closely enough during Pet Quest Collector's Phase 0
research (2026-09-10)**: candidate interaction hooks were added and tested
one small batch at a time - a named script, then five more named scripts,
then seven anonymous closures on one object, then two builtins, then nine
more anonymous closures on a second object - each round costing its own
rebuild, DLL swap, full game relaunch, and a live collect from the tester.
Several of those rounds could have been one round: `hs-game-sdk`'s static
name/hierarchy search (`grep` over `scripts.hpp`/`objects.hpp`, or the
Python/C++ bindings) can enumerate *every* plausibly-relevant script or
object *before* touching the game at all, and costs nothing to run
repeatedly. (In that project, for instance, "every script-table entry inside
`<object>`'s own Create event" found every anonymous closure GameMaker split
out of that object, cheaply, with no live session.)

What carries over to this repository, when a live research session's goal is
"find which of several unknown candidates does X" (not "verify one
already-suspected mechanism"):
- Exhaust the static search first: every name or string matching the concept
  (by substring, by shared class or vtable, by shared caller), through the
  `ghidra` MCP's search and xref tools and `tools/decomp_index.py`, not just
  the one name the plan or a prior guess assumed. Read the existing
  `*-research.md` docs for a technique already proven there.
- Hook every candidate that search turns up in the *same* build, gated
  together behind one research command, before asking for a single relaunch
  — or arm them as `x64dbg` logpoints in one live session
  ([docs/tools/x64dbg-mcp.md](../tools/x64dbg-mcp.md)). A hook that turns out
  irrelevant costs one install call and a few log lines - far cheaper than a
  round trip that could have included it.
- Only fall back to a narrower, more expensive technique (e.g. hooking hot
  shared helpers instead of named functions) after the broad static-search
  round has been exhausted and come back empty, and even then, hook every
  plausible candidate at once rather than one per relaunch.
