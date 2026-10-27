@AGENTS.md

# CLAUDE.md

The line above imports [`AGENTS.md`](AGENTS.md), which holds this repository's
actual rules. Claude Code loads `CLAUDE.md` automatically; nearly every other
coding agent reads `AGENTS.md`. Keeping the rules in one file and importing them
here means there is never a second copy to drift.

**Do not add rules to this file.** They belong in `AGENTS.md`, where every agent
will see them.
