# Agent Guidelines

ExilePact2 is a Path of Exile 2 modding project. Its agent tooling — the
`/workorder` pipeline, the phase and review agents, the hooks and the MCP
servers — was ported from the sister project
[hero-siege-offline-toolkit](https://github.com/falorfrozen-cmd/hero-siege-offline-toolkit),
where every rule below was learned. The stories under
[`docs/agents/`](docs/agents/README.md) are that project's incidents; they are
kept because the rules they back are engine-independent, and the names in them
(ForgePact, YYToolkit, GML) are Hero Siege's, not this repository's.

## Some of These Rules Are Enforced, Not Just Written

[`.claude/`](.claude/README.md) carries the subset of this file that a machine
can check or run, so those rules stop depending on whether the right section
was read first. One dispatcher, `.claude/hooks/post_tool_use.py`, runs the
`PostToolUse` checks (today: decompiled output reaching a tracked file), three
review agents (`decompile-output-guard`, `docs-sync-reviewer`,
`instrument-blindness-reviewer`), three phase agents that run a change through
plan → implement → verify, each at its own pinned model tier and effort
(`planner`, `implementer`, `verifier`), a `consultant` a phase can put one hard
decision to without escalating the whole phase, a `live-operator` that runs a
workorder's written live-game procedure so the driver only relays to the
person, a `scribe` for the round bookkeeping, and the `/workorder` skill (drives
those phases and routes defects back to the phase that caused them). MCP
servers are in [`.mcp.json`](.mcp.json).

The phase split exists so an implementer that meets something the plan did not
anticipate returns `PLAN-DEFECT` with evidence instead of improvising, and the
verifier reports what the acceptance criteria actually printed instead of
judging whether the code looks right — "evidence before assertions" applied to
a handover.

A phase merely *stuck* on one decision uses `ADVICE-NEEDED` to put that
question to the `consultant` and resume with the answer, stating what it would
do unaided so the reply confirms or corrects a position rather than replacing
the thinking; which problems start a tier higher is decided from observable
properties of the task, never a model's self-reported confidence, because a
model that cannot solve something is also badly calibrated about whether it
can.

A defect whose fix is already written down does not buy a whole phase. A
reviewer finding that carries its exact `fix` gets a patch round, and a
`PLAN-DEFECT` that states its correction gets an amendment rather than a
replan. Neither is counted against the pipeline's caps, and the files decide
whether one applied (`round_delta.py size`, `tools/amend_check.py`), not the
agent that took it.

Codex runs the same hook dispatcher and gets the same agents and skills, from
`.codex/` and `.agents/skills/` (§ "Shared Agent Tooling" below).

If a hook blocks an edit, it is quoting a rule from this file — read what it
printed rather than working around it. If you add a rule here that is
mechanically checkable, add the check too; `.claude/README.md` says how.

Story and evidence: [docs/agents/rules-enforced.md](docs/agents/rules-enforced.md);
where `/workorder`'s budgets and tiers come from:
[docs/agents/workorder-calibration.md](docs/agents/workorder-calibration.md)

## Shared Agent Tooling, for Claude Code and Codex Alike

This project is developed with both Claude Code and ChatGPT's Codex, and every
rule and tool here must work for both. This file is the rule file for both
(Claude Code reads it through `CLAUDE.md`'s import). Each piece of tooling has
one source and a derived copy for the other agent:

| What | Claude Code reads | Codex reads |
|---|---|---|
| Skills | `.claude/skills/` (mirror) | `.agents/skills/` (**source**) |
| Review and phase agents | `.claude/agents/*.md` (**source**) | `.codex/agents/*.toml` |
| MCP servers | `.mcp.json` (**source**) | `.codex/config.toml` |
| `PostToolUse` checks | `.claude/settings.json` | `.codex/hooks.json` |

Edit the source, then run `py -3 tools/sync_agent_tooling.py`;
`tests/test_agent_tooling_sync.py` fails on a copy edited by hand or left
stale. The effort variants in `.claude/agents/` (`planner-xhigh.md` and the
rest, how `/workorder` picks an effort per spawn) are derived the same way,
from their source agent's `effort-variants:` line. Never add a skill, agent or
server for one agent only without saying why in `.claude/README.md` § "Codex";
`/workorder` is the one current exception, because it drives Claude Code's own
subagent and workflow tools.

- **Design plugins**: `impeccable` (the frontend design audit and polish
  skill) and the `taste-skill` design skills. Claude Code enables them from
  `.claude/settings.json` and offers to install them the first time a
  contributor trusts the folder. Codex users install them once per machine,
  user-wide: `npx impeccable install --providers=codex --scope=global` and
  `npx skills add https://github.com/Leonxlnx/taste-skill -g -a codex`. Never
  install them into this project, where they would land in `.agents/skills/`
  or overwrite `.codex/hooks.json`. `impeccable` also adds a design-detector
  hook under Claude Code; set `IMPECCABLE_HOOK_DISABLED=1` to switch it off
  for yourself. Under Codex, run `$impeccable audit` on changed UI instead.
- **MCP servers**: `ghidra`, `x64dbg`, `context7`, `github`, `playwright`
  and `app-webview` (Playwright attached to one of our apps' WebView2, started
  by `tools/app_webview.py`; [docs/tools/app-webview-mcp.md](docs/tools/app-webview-mcp.md)).
  `ghidra` reads the local Ghidra project (§ "Check for a Named Ghidra Project"
  below). `x64dbg` attaches a headless debugger to the running game for
  `live-operator` alone, with non-breaking hardware logging breakpoints, under
  the machine-wide game lease (`tools/game_lease.py`;
  [docs/tools/x64dbg-mcp.md](docs/tools/x64dbg-mcp.md)). `context7` serves
  library documentation. `github` needs a personal access token; on a new
  machine, run `py -3 tools/setup_agent_secrets.py` once. On Windows it asks
  for each missing token and saves it as a user environment variable that both
  agents read; elsewhere it prints the `export` line to add to your shell
  profile. The Python servers need `py -3 -m pip install -r
  tools/requirements-mcp.txt`.

Codex loads `.codex/` only for a trusted project, asks you to approve
`.codex/hooks.json` under `/hooks`, and should be started at the repository
root. `.claude/README.md` describes each of these, and § "Codex" there lists
what does not carry over.

## Offer `/workorder` When the Work Has Shape, and Respect "Plan Only"

`/workorder` is user-invoked only. The skill sets `disable-model-invocation:
true`, because it spawns at least three agents, which is the wrong response to
a typo — so reaching for it through the `Skill` tool returns a refusal, not a
pipeline. You cannot start it; you can only **offer** it, in one line, before
starting, when the request matches any of these:

- the change spans several files or modules;
- the user asks to **implement, execute or carry out a plan** — theirs, or one
  from an earlier session;
- it introduces concurrency, changes how a hook attaches, alters a contract two
  language bindings must agree on, or would ship a player-visible change;
- there are three or more distinct steps and a later one depends on an earlier
  one being right.

Offer, then wait. One line — *"This spans the hook and the panel; want me to
run it through `/workorder`, or just make the change?"* — and if the answer is
no, do the work directly and drop it. Do not re-offer within a session, and do
not ask about a one-line fix.

**"Plan it" means plan it, and nothing else.** When the request is for a plan,
a design, an approach, or an assessment of how something should be done, the
mode that fits is `/workorder plan <task>` — so name it and ask the user to run
it, the same way you offer the full pipeline. Do not try to invoke it yourself;
that is the refusal above, and it costs a round to discover.

Once it is running, **stop when the plan exists**. Report it, name the
`resume` command, write no code, and edit nothing. The user is splitting the
work across sessions on purpose: implementation starts from the file, not from
the conversation, and the plan not surviving that split is exactly the signal
they are trying to get.

## Spend Each Check Once, and Overlap What Does Not Wait

A check shared between two changes is cheaper than the same check run after
each one, and work that does not depend on a phase should not wait for it.
So, in `/workorder` and anything run like it:

- **Changes that are ready together get one round and one full verify**, not
  one each. Plan fewer, larger workorders, with lanes and one join verify,
  rather than a chain of small ones.
- **Start read-only or disjoint-file work as soon as its input is
  committed**, beside the verifier rather than behind it. Stream a review's
  findings to fixers one by one against a pinned snapshot, and verify all the
  fixes once, after the stream drains.
- **Name commits by tag or merge-base in criteria, never by a hash of a
  moving head.** `tools/plan_lint.py` flags a bare hash as `pinned-sha`.
- **Merge a clean `origin/main` that misses the plan's files in place**; stop
  only when it conflicts or touches them.
- **Batch the owner's decisions before the next plan, and fix a flaky test
  in the round that saw it**, rather than retrying it.
- **During development run only the relevant subset; the full suite runs
  once, as the last step before the pull request.** A first verify and an
  items gate run every criterion but the whole suites and the ones marked
  `(final)` (`tools/run_criteria.py --dev`); a fix round runs the criteria
  whose `(reads ...)` the fix touches plus the failed ones
  (`--changed-since`). The full set runs at the final gate before the PR, or
  whenever the delta is unknown, touches a shared contract, or the verifier
  cannot tell. So every module a change touches needs a targeted criterion
  beside its whole suite.
- **After a write, read the diff, not the file**: an agent checks its own edit
  with `git diff -- <path>`, a grep or a ranged read, and an agent after it
  reads the diff since its base or the one section it needs, never the whole
  file again. `workorder_audit.py` R26 fails an implementer or planner that
  does.
- **A question never idles the pipeline.** Before asking the owner, start
  everything the answer cannot change. Spin off an out-of-scope bug instead
  of asking about it. Apply the default to a reversible choice and say how
  to undo it.
- **The owner's own scope is not a failure.** A plan change or round that
  exists only to carry the owner's new decision costs no replan, no tier
  step and no round of the cap (`amend_check.py`'s `SCOPE`, State's `scope
  rounds:`, audit R25).
- **Plan when the inputs exist, and review the design before building it.**
  A plan that waits on another workorder's result stays `status: DRAFT`,
  which `plan_lint` refuses.

Independence is not traded for speed: the verifier and every reviewer still
run fresh, and a check is shared, never skipped.

Procedure: `.claude/skills/workorder/SKILL.md` § "Spend each check once, and
overlap what does not wait" and `.claude/agents/planner.md` § "Spend each
check once". Story and evidence:
[docs/agents/workorder-calibration.md](docs/agents/workorder-calibration.md)

## Legal: Decompiled Output Never Reaches Any Origin

This project reverse-engineers Path of Exile 2's client (memory layout, hooked
functions, data-file schemas) to build mods. **The constraint is entirely about
output, not technique**: decompiling or disassembling the game locally — reading
a function in Ghidra/IDA to understand what a mechanism does — is a legitimate
research step when static name search and live measurement (hooking, tracing,
before/after diffing) run out. What must never happen is for that output to
land in a repository that gets pushed to its origin — see the hard rule below.

**Hard rule: never commit, paste, or embed decompiled or disassembled game code
(IDA/Ghidra listings or exports, pseudo-C, disassembly dumps) or extracted
game assets into any tracked file.** This applies to production code, `docs/`
research notes, commit messages, and comments alike — write up *what was
learned*, in your own words, never the decompiled text itself.

What is fine to commit — because it documents *interoperability facts*, not the
game's copyrightable expression:
- **Names, data-file schemas and numeric identifiers** needed to read or hook
  the game.
- **Measured runtime behavior**: what a function does when called, what it reads
  or writes, observed crashes and their signatures, before/after values.
- **Our own** code that reacts to that behavior — original work, not derived
  from the game's source text.
- Function/struct *offsets, signatures (byte patterns) and calling
  conventions* needed to hook or read memory.
- **Short user-facing game text** — quest names, step text, NPC lines and
  reward text, only the fields a tool needs for matching or display, stored
  with its source URL and fetch date — is not an "extracted game asset" under
  the hard rule above (the owner's decision, 2026-10-24); a whole-page or bulk
  dump still is.

What must stay local and out of version control:
- Full or partial decompiled/disassembled function bodies, however they were
  produced (Ghidra, IDA, Binary Ninja, manual transcription).
- Exported decompiler project files or listings (`.gpr`, `.i64`, `.idb`) and
  extracted game data (`Content.ggpk`/bundle contents) — kept on the
  researcher's machine only.
- Screenshots or copy-pasted excerpts of decompiled code in issues, PRs, or
  docs.

If a research note needs to explain *why* something behaves a certain way,
paraphrase the mechanism rather than quoting the code. `.gitignore` excludes
common decompiler artifact paths as a mechanical backstop; extend it rather
than working around it if a new tool produces a new artifact type.

**Path of Exile 2 is an online game.** Client modification and attaching a
debugger can break Grinding Gear Games' terms of service and get an account
banned. Research against a client you can afford to lose, and never ship
anything that gives one player an advantage over another.

## Check for a Named Ghidra Project Before Researching a Game Mechanism

Without a decompiler, a question about how a game function behaves — what it is
really called, what arguments it wants, what it leaves behind — can only be
answered by guessing, building and asking the owner for a live session. Each
guess costs a whole round. Before planning or researching an unknown game
mechanism, check what this machine can read, and say what you found.

- **The `ghidra` MCP server comes first.** Run
  `py -3 -m tools.ghidra_mcp status`; when it prints no `missing` line,
  answer the question through the `mcp__ghidra__*` read tools (search and
  list functions and strings, decompile, xrefs, callers, callees and call
  graph, function info). It serves a copy of the research project, so it never
  locks the original. The planner and its effort variants, `implementer`,
  `implementer-medium`, `consultant` and `consultant-max` carry those read tools
  and no write or debugger tool; the ban on writes and debugging through a
  Bash `curl` to the REST port or under Codex is a prompt rule, not something
  the tooling enforces ([docs/tools/ghidra-mcp.md](docs/tools/ghidra-mcp.md)).
- **An empty callers or xrefs answer is "not observed", not a negative.** They
  come from Ghidra's reference table, which an import without full analysis
  leaves sparse, and which never sees calls through a vtable or a function
  pointer (§ "Prove the Instrument" below).
- **If the MCP is unavailable but the project is present**, say what `status`
  printed and offer `py -3 -m tools.ghidra_mcp setup` (its download needs the
  owner's go-ahead).
- **Record the check.** A plan that researches a game mechanism carries one
  `## State` line, `ghidra mcp: used — <what it answered>`, `ghidra mcp:
  unavailable — <what status printed>; offered setup` or `ghidra mcp: skipped —
  <reason>`. `tools/plan_lint.py` refuses a research plan without it
  (`ghidra-unchecked`) or with another value (`ghidra-bad-value`).
- **Whichever route answers, decompiled text never reaches a tracked file**,
  a commit message, an issue or a PR (§ "Legal" above). Paraphrase.
- **If there is no Ghidra project at all, offer the owner two options** and let
  them choose:
  1. **Set up Ghidra first (recommended).** Install a JDK 21 and Ghidra, and
     import a copy of the game executable from a plain path (the headless
     launcher cannot take a path containing `(x86)`). Downloads need the
     owner's go-ahead.
  2. **Continue without decompilation (not recommended).** Every mechanism
     question then has to be settled by live measurement, one round each.

**Ask the decompile index before a headless decompile.** A function someone
already decompiled for this build is a file on this machine, not a new
headless run, but only if it can be found. Before decompiling, run
`py -3 tools/decomp_index.py has <name>` (add `--contains` for a partial name)
and reuse what it lists for the current build; after a headless run, `scan`
its output directory. The index and every decompile stay outside any
repository; the tool refuses to write either inside a git checkout. Detail:
[docs/tools/decomp-index.md](docs/tools/decomp-index.md).

## Mod Development Workflow: Test Before / After, Then Build to It

When developing a mod, hook, or any gameplay-affecting change, don't start by
editing the hook and checking it live in-game. Establish both ends of the
behavior change as tests first, then write the mod to close the gap:

1. **Baseline test** — capture how the game behaves *without* the mod (vanilla
   read, unmodified value, default probability/roll).
2. **Target test** — capture how it *should* behave once the mod is applied.
3. **Implement the mod** to turn the target test green without breaking the
   baseline test's assumptions about the unmodified path (a toggle that's off
   by default should still reproduce baseline behavior).

Where a real game process is required to observe the behavior, prefer a
fixture or mock over a live game session for the baseline/target tests, and
reserve actual in-game runs for final verification.

**Choose representative cases, not every case.** A feature that applies to many
skills, items or monsters does not need every case exercised; pick one or two
ordinary cases plus the outliers that take a different code path or read a
different kind of value. This applies most to live sessions, where the owner's
time in the game is the cost. It does not relax "Prove the Instrument" below.

**Where a mechanism can be written down, model it before you launch.** Write
the mechanism in your own words with each claim labelled static reading,
measured or not established (specs in `docs/models/`), build a stdlib-only
model from that spec and the recorded measurements, and write the baseline and
target tests against the model. Whoever builds the model works from the spec
alone and returns a gap to the reader instead of opening the decompiler. A
model cannot catch whether a hook attaches, frame timing, stale addresses or
how a change feels in play; those still need the live run.

Story and evidence: [docs/agents/static-model-workflow.md](docs/agents/static-model-workflow.md)

## Limit Rebuilds & Reruns During Development

Full recompiles and relaunching the game for every change are slow. Before or
alongside mod development:
- Look for an existing tool that lets you exercise the change without a full
  rebuild/relaunch (a standalone test harness, a replay of captured state,
  hot-reloadable hook code). If none exists, build one under `tools/` and
  document it, so the shortcut is reusable rather than one-off.
- `tools/freeze_probe.ps1` is an example of the pattern applied to diagnostics:
  observe the running game from outside instead of print-and-relaunch.
- Reserve full rebuild + in-game relaunch cycles for final confirmation.

When the goal is "find which of several unknown candidates does X," exhaust
the static search before any live session, then hook every candidate it turns
up in one build behind one research command — several rounds have repeatedly
turned out to be one round done broad enough the first time.

When the question is "does this function fire, with what arguments, how
often", a live debugger can answer it without a research build or a relaunch.
The `x64dbg` MCP server attaches headless x64dbg to the running game, under
the game lease, and logs each hit through a non-breaking hardware breakpoint:
at most four at once, a positive control first, and an address mapped from
Ghidra stays a research finding. Only `live-operator` carries its tools, so a
debugger question becomes a step of a written live procedure. See
[docs/tools/x64dbg-mcp.md](docs/tools/x64dbg-mcp.md).

Story and evidence: [docs/agents/limit-rebuilds-reruns.md](docs/agents/limit-rebuilds-reruns.md)

## Keep Build Outputs From Filling the Disk

Every checkout, the main one and each `git worktree`, builds into its own
`apps/<app>/target/`, and a debug `target/` grows by gigabytes per build; on
2026-10-30 that filled C: during a workorder (issue #38). Build output stays in
each checkout: do not set `CARGO_TARGET_DIR`, `build.target-dir` or
`build.build-dir`, because one shared dir serialises every build on the machine
and out-of-tree output outlives its worktree. Reclaim the space instead with
`py -3 tools/disk_hygiene.py`: `report` shows what each checkout costs, and
after a pull request merges, run the `worktrees` dry run, read it, then
`py -3 tools/disk_hygiene.py worktrees --apply` to remove the merged worktrees
(never a locked, dirty or unmerged one; branches are never deleted). `trim`
cuts an app's `target/` that grew past a cap. `/workorder` refuses to launch
with `DISK-LOW` when the drive has less than 30 GiB free; free space with the
tool, and launch anyway only on the owner's word.

Commands, safety rules and measurements: [docs/tools/disk-hygiene.md](docs/tools/disk-hygiene.md)

## Prove the Instrument Before Trusting a Negative Result

Before a "0 calls" / "no effect" / "never fires" result is allowed to close a
line of investigation:
- **Run a positive control through the same instrument.** Point it at
  something you already know fires, in the same build, in the same session.
  An instrument that cannot produce a non-zero anywhere has told you nothing
  about your target.
- **Know how your instrument attaches, and whether the code under test can
  reach it.** A vtable or import-table swap only sees calls routed through
  that table; an inline detour at the function's own address sees the call
  itself. Prefer the latter whenever a table-based hook reports zero.

The same blindness is a shipping bug: a hook that reports "installed" and then
silently changes nothing is a feature that reports armed and does nothing. So
**put the interception in the installer, not in whichever call sites a review
happened to verify** — and a correction is not done until every shared
installer has it too.

Story and evidence: [docs/agents/prove-the-instrument.md](docs/agents/prove-the-instrument.md)

## Check a Permission Where It Is Used, Not Where It Is Convenient

A flag that a consumer reads earlier in the frame cannot be validated at the
end of the frame: whatever the consumer did this frame, it already did.

- **Validate at the point of use**, with the thing being acted on. "May I do
  this to *this* object" is usually answerable from the object itself, and
  that answer is correct regardless of when any cached state was refreshed.
- **A sentinel that means "unknown" must never compare equal to a real
  value.** Prefer a read that fails as a unit over a magic value.
- **Frame-boundary work is for budgets and housekeeping**, not for anything
  another system's correctness depends on within the same frame.
- **Write the negative down as "not observed", not "does not happen"**, until
  a control backs it.

Story and evidence: [docs/agents/check-a-permission.md](docs/agents/check-a-permission.md)

## Never Call an Address You Resolved by Hand

Reading a function's address in Ghidra is a legitimate research step. **Shipping
that address as a constant is not.** A game build's RVAs are not an interface:
the next patch moves every one of them, and the constant then names whatever
bytes happen to sit at that offset. A *read* through a stale address returns
garbage; a *call* through one transfers control into arbitrary code on a
player's machine.

So, before a pointer is called or dereferenced in code that reaches a player:

- **Resolve by something that survives a patch**: an export, a vtable slot
  reached from a known object, or a byte-pattern signature scan with enough
  context to be unique — and fail if it matches zero or more than one site.
- **Treat a struct layout as an assumption to measure**, not a fact: it fails
  silently (a plausible zero) instead of loudly. Find a positive control on the
  same target first.
- **A measured address is a research finding, not an implementation.** Keep it
  in `docs/` and behind a research-build flag, where a wrong value costs a
  session, not a release.
- **If an address genuinely cannot be avoided, validate it and refuse.**
  Confirm the target is committed, executable, and inside the intended
  module's image before calling, and on failure disable the feature with a
  message instead of falling through to the call. Count the refusal so it
  surfaces instead of as silence.

When a call shape is rejected, record *what was supplied* alongside the result:
a rejected call shape is a labelling problem to revisit, not a settled dead
end. And make a refusal say why — a mod that fails silently is a mod nobody can
debug from a bug report.

Story and evidence: [docs/agents/never-call-resolved-address.md](docs/agents/never-call-resolved-address.md)

## Don't Suspend the Game's Own Runtime

A mod that works does one of two things: it **reads** game state, or it
**changes one value inside a call the game is already making**. The game stays
in charge of its own loop throughout.

**Features that take the loop away from the game are not recommended**: pause,
time scaling, save-state/rewind, forced state restore, freezing threads —
anything whose contract is "suspend the world and give it back unchanged".
Their failure mode inverts (ordinary mods fail by doing nothing; these leave
the session stuck), the claim "everything stops" cannot be verified, and they
tax every future game patch. In an online game they also desync the client
from the server.

Prefer the alternatives: read-only tooling outside the game, a change to one
value the game is about to use, or leaving the behaviour alone.

Story and evidence: [docs/agents/dont-suspend-runtime.md](docs/agents/dont-suspend-runtime.md)

## Documentation & Instructions Maintenance

Upon completing any task or making changes to features, workflows,
architecture, or dependencies, update the documentation and `README.md` files
where relevant, and keep links and commands accurate.

**Record what was built, not what you meant to build.** `*-plan.md` is in this
repository's `.gitignore`. A plan is a working note: it is out of date the
moment the thing exists. When a plan's work lands, fold the reasoning that is
still true into the document that describes the result — a design doc, or a
`docs/adr/` entry for a decision that outlived its discussion — and leave the
plan on your own machine.

## Fold What a Mod Learned About the Game Into the Shared References

A mod's research doc is where its measurements are argued out. Facts about the
game itself — where a structure lives, what a container holds, what a function
reads and writes, which call crashes the game — belong to every mod, so when a
research phase is recorded, those facts go into `docs/RUNTIME_DATA_MODELS.md`
in the same feature, not "later". Copy only what was measured or read, label a
static reading as a reading, in your own words, per the Legal section, and
change the label to measured when a later session measures it. Leave the
argument, the negative results and the session history in the research doc,
and link to it.

## One Branch and One Pull Request per Feature

A feature gets exactly **one branch and one pull request**. Follow-on work for
that feature (a fix a live gate found, a doc correction, a verification result)
is committed onto that same branch and rides that same pull request. Before
pushing, look for the feature's existing branch on the remote (`git branch -r`,
`gh pr list --state all`); if one exists, fast-forward or merge onto it rather
than pushing a sibling. Different features stay apart even when they touch the
same files. Link the issue the work belongs to (`Closes #<n>`), and open one
first if there is none.

## Every Pull Request Gets `ai-review`, and Its Comments Are Fixed Before Merge

1. **Add the `ai-review` label when you open it** (`gh pr create --label
   ai-review`). The label is what starts the AI review
   (`.github/workflows/ai-review.yml`); it needs the Claude GitHub App
   installed, the `ai-review` label created and the `CLAUDE_CODE_OAUTH_TOKEN`
   secret, which only the owner can provide.
2. **Wait for the review, then fix what it found** on the same branch. Reply to
   each comment with what changed, or why nothing did. Then request a
   re-review with a comment whose body **starts with** `@claude review`, as its
   own comment — the workflow checks `startsWith`, so a trigger anywhere else
   is skipped without a word.
3. **Do not merge, and do not report the pull request ready to merge, while
   the review has not posted or a comment of it is unaddressed** — nor while CI
   is red. Merging is the owner's call.

A pull request that edits `ai-review.yml` itself cannot be reviewed by it;
say so in the pull request instead of waiting for a review that will not come.
