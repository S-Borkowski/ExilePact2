# `.claude/` — the automated half of `AGENTS.md`

`AGENTS.md` holds this repository's rules in prose. This directory holds the
subset that a machine can enforce or execute, so they stop depending on whether
an agent happened to read the right section of a long file first. `AGENTS.md`
itself states each rule plus one sentence of why; the incident or review
finding that made a rule worth writing lives in `docs/agents/<section>.md`,
linked from the rule, so it stops being loaded into every subagent's context
on every turn.

All of this was ported from the sister project
[hero-siege-offline-toolkit](https://github.com/falorfrozen-cmd/hero-siege-offline-toolkit)
(a GameMaker game's modding hub), where it was built and measured. Only the
engine-independent machinery came across: the hub, the release catalog and its
signing, the Tauri and SDK reviewers, the game-driving MCP server and the
submodules stayed behind. Dates, issue numbers and workorder names quoted
below (`forgepact-issue-14`, "hub #485" and the like) are that project's
history, kept where they explain why a rule exists.

Everything here is committed on purpose. Only `settings.local.json`
(per-machine overrides) is gitignored.

## Hooks — `settings.json` + `hooks/`

`settings.json` fires one `PostToolUse` command per `Edit`, `Write`, `Bash`,
`PowerShell` or `NotebookEdit` call, `.claude/hooks/post_tool_use.py`. The
dispatcher runs its checks in-process against one shared `git status`
(`_common.TreeState`, its explicit injection point) rather than launching a
separate `py -3` and `git status` per check. Today it runs one check:

| Check | Fires when | Catches |
|---|---|---|
| `decompiled_output.py` | any changed text file | decompiler and disassembler output (Ghidra/IDA auto-generated symbols, pseudo-C and listing shapes) reaching a tracked file |

The check keeps its own `check(payload, tree) -> (rc, message)` body and its
own standalone `main()`, so `py -3 .claude/hooks/decompiled_output.py` behaves
the same with or without the dispatcher; `tests/test_claude_hooks.py`'s
`TestDispatcherEquivalence` proves the two paths agree. A crash in a check is
caught and reported as its own non-blocking line rather than masking, or
blocking for, any other. A check exits 0 silently when nothing is wrong and 2
with an explanation when something is. They run through `py -3`; on a
non-Windows machine change that to `python3` in `settings.json`.

**Adding a check:** give it a `check(payload, tree: TreeState) -> (rc,
message)` function, add its module name to `post_tool_use.py`'s
`TREE_CHECKS` tuple, keep a standalone `main()` for its own CLI, and add a
positive/negative pair to both the check's own test class and
`TestDispatcherEquivalence` in `tests/test_claude_hooks.py`.

**Checks key off the working tree, not the tool payload**, and that is the
single most important thing to preserve when editing them. A payload-shaped
hook only sees `Edit` and `Write`, and only when it can resolve
`tool_input.file_path` against the repository root. Both halves leak: a `sed -i`
or heredoc under `Bash` never produces a `file_path`, and a path the hook cannot
resolve returns "not my file" — which is indistinguishable from "nothing
wrong". In the sister project, payload-shaped versions exited 0 against a
harness feeding them POSIX-style paths Windows Python could not resolve, and
looked installed and healthy while guarding nothing. `git status --porcelain`
costs ~40 ms and has neither hole. `.claude/hooks/_common.py` holds the shared
parts.

**Escape hatch: `EP2_SKIP_HOOKS=1`.** Every blocking message names it. Because
the trigger is the working tree, a state the hook objects to would otherwise
fail *every* later tool call, including the ones needed to finish or undo the
work. A blocking hook with no way out is worse than no hook.

Why this check, when the bar for a hook is otherwise "guards a failure that
has already shipped": the failure it guards is not a bug a later release fixes.
Once decompiled game text reaches a pushed origin it cannot be taken back
(`AGENTS.md` § "Legal: Decompiled Output Never Reaches Any Origin").
`.gitignore` already excludes decompiler *artifact paths* (`*.i64`, `*.gpr`,
`ghidra_projects/` and so on); this covers the likelier accident, a listing
pasted into a tracked `.md` or `.cpp` while writing up a finding. It reads only
the lines a change *adds*, so a committed file that legitimately names an
address or a symbol does not wedge every later edit to it.

It deliberately does **not** match plain function names, offsets or
byte-pattern signatures. `AGENTS.md` names those as interoperability facts that
are fine to commit, so matching them would flag the research notes they belong
in and teach everyone to set `EP2_SKIP_HOOKS`.

## Agents — `agents/`

There are two kinds here, and the distinction is the whole design.

**Phase agents run in sequence**, each at its own model tier, and each hands the
next one a document rather than a conversation. They are driven by
`/workorder`, never spawned by hand.

| Agent | Model / effort | Does |
|---|---|---|
| `planner` | opus / high | researches the change and writes `.claude/workorders/<slug>-plan.md`, whose acceptance criteria are commands and files, never prose |
| `implementer` | opus / high | executes the steps; returns `PLAN-DEFECT` with evidence rather than improvising around a plan that turns out to be wrong |
| `verifier` | haiku / medium | runs the acceptance criteria (first through `tools/run_criteria.py --jobs auto`, which runs every command-shaped criterion in one call, independent ones at once; after a fix, only the criteria the fix reaches plus the failed ones), or one item's `checks:` in a streamed plan, and reports what they actually printed; read-only, and judges nothing it cannot execute |
| `consultant` | opus / xhigh | answers **one** narrow question from a phase that hit a decision above its tier, then stops; never implements, plans or reviews |
| `live-operator` | opus / medium | runs a workorder's written `### Live procedure <n>` against the real game — the positive control first, raw output to `<slug>-live-<n>.md` — under the machine-wide game lease, and hands every in-game action a person must take back to the driver; never installs a build, never judges the mechanism |
| `scribe` | haiku / low | **retired from the round loop.** No path spawns it now: `tools/workorder_record.py` applies State/Log from the run record. It used to paste a precomputed round Log entry and State lines into the workorder files, with `Read`/`Grep`/`Edit` only (no shell). The file stays until a follow-up removes it with its pinned-tier test and audit fixtures |

Why these tiers: planning carries the most judgement that is written down
nowhere, so it gets the strongest model. Implementation is *not* the easy part —
a plan never fully survives contact, and a weak model follows a wrong plan off
the cliff instead of stopping — so it gets a capable one too
(`skills/workorder/model-tiers.md` has the prices and the measurement).
Verification against mechanical criteria is genuinely cheap, so it gets the
cheapest. The session that drives `/workorder` should run opus/medium
(`model-tiers.md` says why). Every agent pins `effort:` for the same reason it
pins `model:` — left out, it silently follows the session.

Where `/workorder` runs one agent at more than one effort, the agent lists the
extra levels in `effort-variants:` and `tools/sync_agent_tooling.py` generates
`<agent>-<level>.md` for each: `planner-medium`, `planner-xhigh`,
`planner-max`, `implementer-medium` and `consultant-max`. The `Agent` tool
takes a model per call but no effort, so the agent's name is how a spawn picks
one. Never edit a variant; edit its source and re-run the sync.

The verifier never sees the implementer's reasoning, because sharing that
context would mean sharing its blind spots.

The `planner` (and its variants), `implementer`, `implementer-medium`,
`consultant` and `consultant-max` reach the `ghidra` MCP server's read tools,
named `mcp__ghidra__<tool>` in their `tools:` lines: search, list, decompile,
xrefs, callers, callees, call graph and function info. The read set is
`AGENT_READ_TOOLS` in `tools/ghidra_mcp.py`, and
`tests/test_ghidra_agent_tools.py` pins those lines to it and fails any write
or debugger tool on them. Every other agent (verifier, scribe, `live-operator`,
the reviewers) gets none. Their prompts put the MCP first for a game-mechanism
question and treat an empty callers or xrefs answer as "not observed": Ghidra's
reference table never sees calls through a vtable or a function pointer
(`AGENTS.md` § "Check for a Named Ghidra Project Before Researching a Game
Mechanism").

`live-operator` alone carries the `x64dbg` MCP server's tools, all eight,
named `mcp__x64dbg__<tool>` on its `tools:` line: `status`, `logpoint`,
`command`, `bplist`, `log`, `modules`, `disasm` and `detach`. The set is
`LIVE_OPERATOR_TOOLS` in `tools/x64dbg_mcp.py`, and
`tests/test_x64dbg_agent_tools.py` pins the line to it and fails any
`mcp__x64dbg__*` tool on another agent. Its prompt uses them only when a live
procedure names debugger steps: attach under the game lease, a debugger
positive control first, hardware logging breakpoints only, verified with
`bplist`, and `detach` before the game is closed
([`docs/tools/x64dbg-mcp.md`](../docs/tools/x64dbg-mcp.md)). A session that
has not loaded those tools reaches the same eight, through the same gates, as
`py -3 -m tools.x64dbg_mcp tool <name> '<json>'`.

It also carries the `app-webview` server's tools (tabs, snapshot, click,
type, select, key press, hover, wait, screenshot, evaluate, console) to
drive one of our own apps started by `tools/app_webview.py launch`, and
read-only `mcp__playwright__*` tools for pages an app opens in the browser
([`docs/tools/app-webview-mcp.md`](../docs/tools/app-webview-mcp.md)); no
test pins these, unlike the x64dbg set.

There is no game-driving MCP server here (the sister project's launched,
screenshotted and sent input to its game). The lease is taken by hand or by
the live operator with `py -3 -m tools.game_lease acquire --label
<slug>-live-<n>`, which names the running game process as its holder, and
every in-game action is a step the person performs.

### Getting a harder model onto a harder problem

Three mechanisms, because they cover three different failures. A pipeline with
only the last one learns that a task was hard by failing at it twice.

**Triage, before anything is spawned.** `/workorder` matches the request against
a table of observable properties — does it introduce concurrency, does it have
to establish an unknown mechanism rather than verify a suspected one, does it
change how a hook attaches, does it change a contract two bindings must agree
on — and starts the planner and implementer a tier up when it matches. The
signals are deliberately *properties of the task*, never a model's own sense of
how confident it feels: a model that cannot solve something is also poorly
calibrated about whether it can. The driver states which row it matched, so the
user can correct it for free.

**Consultation, mid-phase.** A phase can return `ADVICE-NEEDED` with one narrow
question; the driver spawns `consultant` (`consultant-max` for the hardest
rows), then re-enters the phase with the answer appended to the workorder's
`## Log`. Same-phase, same-tier re-entry is a `SendMessage` to that phase's own
agent id (recorded in `## State` › `agents:` when it was spawned). A fresh
spawn happens only when that is not possible, and carries a `PROGRESS SO FAR`
block instead. Either way, one hard decision costs one short answer rather than
re-running the whole phase at a higher tier. An agent cannot spawn another
agent, so this is a return-and-redispatch through the driver rather than a call.

The guardrail is one required field: **`WHAT I WOULD DO WITHOUT HELP`**. The
consultant confirms or corrects a position, which is fast and precise, instead
of solving from nothing. A question without it is sent back. Cap is two per
round; a third means triage was wrong and the *phase* escalates instead.

**Escalation, on repeated failure.** The planner moves to `planner-max` on a
second `PLAN-DEFECT` and stops on a third. This is the backstop, not the router.
It works because planning is the cheapest phase by token volume — a plan is a
few thousand output tokens against an implementation's hundred thousand — so one
maximum-effort replan costs less than the implement round it saves.

The reviewers never escalate and never consult. Neither does the verifier, and
that one is a design choice rather than economy — it is scoped to what it can
execute, and a route to a judgement call would reopen the door that scoping
closes. An uncertain verifier reports `UNATTEMPTED` and lets a human decide.

**These are tier aliases, not pinned version IDs**, and deliberately. The tier
is the design decision; the version is not. The acceptance criteria are
mechanical, so a model change that breaks something fails a *test*, not a
review; pinning version IDs would buy a stale-ID sweep every generation, ending
in a retired model that breaks the agent outright. This is not the
hand-resolved-address case `AGENTS.md` forbids: an alias is a documented moving
pointer, where a hardcoded address is a constant whose meaning moved underneath
it.

**Every agent here sets `model:` explicitly — none inherits.** That is the
difference between a tier being a design decision and a tier being an accident
of whatever the operator last picked in `/model`. In the sister project two
reviewers omitted the field, which silently meant "whatever the session is":
running the session on Haiku would have quietly dropped their review to the
cheapest tier while still printing a clean report. Neither is visible in a
diff — something that reports itself working while doing something other than
what it says.

So: set `model:` on every new agent. `inherit` is a valid value when following
the session is genuinely the intent — but say why in the agent's **body**, not
as a YAML comment beside the value. A trailing ` # because …` makes the field
read as `inherit # because …`, which is neither a known tier nor quoted, and
fails both `test_every_pinned_model_is_a_known_tier` and
`test_no_unquoted_hash_in_any_definition` (the ` #` trap described further
down).

The one thing that cannot be pinned is the **`/workorder` driver itself** — a
skill runs in the main session, at the session's model. That is why its routing
is written as an explicit table of verdicts and destinations rather than as
judgement: the loop has to be followable at any tier. If you run the session on
Haiku, the phases still run at their own pinned tiers; only the routing between
them gets cheaper.

One limit behind the tiers: Haiku 5.5 bills any turn whose prompt is over 100K
tokens at 5x. Acceptance criteria plus the one context section a criterion
cites, which is all the verifier opens, stay well under it, and that is part of
why the verifier's job is scoped to what it can execute rather than to
reviewing the change.

### Domain reviewers

**Domain reviewers run in parallel**, are read-only, and each covers one bug
class. Wall-clock is one agent; only tokens add up.

| Agent | Model / effort | Reviews |
|---|---|---|
| `decompile-output-guard` | opus / medium | whether a change carries the game's own expression rather than facts about it — the judgement half of `decompiled_output.py` |
| `docs-sync-reviewer` | opus / medium | which README, guide, index entry or ADR the change just made wrong |
| `instrument-blindness-reviewer` | opus / high | table-only hook installs, hand-resolved addresses, struct-layout assumptions, and negatives recorded without a positive control |

`instrument-blindness-reviewer` exists because the sister project once had 34
hooks reporting zero calls against a game that was calling them; its file
records the incidents, because the reason these are hard to catch in ordinary
review is that the code reads as correct. `docs-sync-reviewer` enforces
`AGENTS.md` § "Documentation & Instructions Maintenance", which nothing else
does.

Hook and agent overlap deliberately on decompiled output: the hook takes text
that is unmistakably a listing, and the agent takes the question no pattern
answers — has a paraphrase crossed from describing behaviour into reproducing
expression?

**Which reviewer runs when.** No reviewer is given the workorder path — each
dispatch pastes `## Goal`, `## Out of scope`, the diff commands, and the paths
this round touched, found with `.claude/skills/workorder/round_delta.py`
(`snapshot <slug> <round>` before the round, `delta <slug> <round>` after; the
snapshot records HEAD too, so work the implementer commits mid-round lands in
the delta; exit 3 means the snapshot is missing or untrustworthy, and
everything is treated as changed). In workflow mode only
`instrument-blindness-reviewer` runs every round; `docs-sync-reviewer` runs
once at the `finalGate: true` launch, which is verifier plus deferred reviewers
with no implementer, and `decompile-output-guard` runs there only when the
branch touches docs or `*.md` — every round, `tools/decompile_check.py` runs
the hook's signatures over the lines added since the round's base, as the
mechanical half. A reviewer that was `BLOCKING` last round is always re-run.
The diffs reviewers read are built from `round_delta.py heads`'s recorded base
commits rather than `HEAD`, because a `HEAD`-relative diff after the round's
own commits is empty.

## Skills — `skills/`

| Skill | Invocation | Purpose |
|---|---|---|
| `workorder` | user-only (`/workorder`) | drives plan → implement → verify across the phase agents, routing defects back to the phase that caused them |

`.claude/workflows/workorder-rounds.js` also shows up as the `workorder-rounds`
skill entry (from its `meta`); it is launched by `/workorder`, not invoked on
its own.

Any shared skill added later is written in `.agents/skills/` (the location
Codex and most other agents scan; empty today but for a tracked `.gitkeep`,
because git drops an empty folder and the sync tool and its test read this one
in every checkout), and `.claude/skills/<name>/` is
a byte-for-byte copy made by `tools/sync_agent_tooling.py`, because Claude Code
scans only `.claude/skills/`. `tests/test_agent_tooling_sync.py` fails on a
mirror that differs or a skill added under `.claude/skills/` alone.
`workorder` is the exception (`CLAUDE_ONLY_SKILLS`; see § "Codex" below). A
user-only shared skill also gets a generated `agents/openai.yaml` with
`allow_implicit_invocation: false`, Codex's equivalent of
`disable-model-invocation: true`.

`workorder` is user-only because it spawns at least three agents and is the
wrong tool for a one-line fix — the cost should be a deliberate choice, not
something that fires on a typo. `AGENTS.md` § "Offer `/workorder` When the Work
Has Shape" carries the trigger list that makes Claude *suggest* it. Suggest,
wait, and drop it if the answer is no.

### How `/workorder` runs

`workorder` owns the loop the phase agents cannot run themselves: an agent
returns a verdict to whoever spawned it, so the routing — `PLAN-DEFECT` back to
the planner, `IMPL-DEFECT` back to the implementer — lives in the driver. Two
caps keep that loop from burning tokens on a problem it has lost: **2 replans**
and **3 implement→verify rounds**, then it stops and asks a human.

It has three modes: `/workorder <task>` runs everything, `/workorder plan
<task>` stops after the plan, and `/workorder resume <slug>` picks up at
implementation — in a **different session**, which is the point. Splitting there
buys a human checkpoint where a plan can be argued with before any code exists,
and a real test of whether the plan is self-sufficient, since a fresh session
has none of the conversation that produced it. A plan that cannot survive a
fresh session was never a plan; `resume` checks for exactly that before
spawning anything, and routes `PLAN-DEFECT` if a step says "as discussed" or
leans on a decision that was never written down.

The detailed procedure lives in `skills/workorder/SKILL.md` and its siblings:
`amendments.md`, `checks-and-overlap.md`, `items.md`, `live-gate.md`,
`model-tiers.md` and `verify-mechanics.md`.

**A workorder is two files**: `.claude/workorders/<slug>-plan.md`
(frontmatter, `## State`, `## Goal`, `## Out of scope`, `## Acceptance
criteria`, `## Steps` — what everyone who touches the workorder reads) and
`<slug>-context.md` (`## Context the implementer needs` by stable `###`
subsection, `## Needs human judgement`, `## Log` — read by section, only when
a step or a reviewer is pointed at it). Both, and the
`.claude/workorders/.rounds/` round snapshots, are gitignored: they are working
notes for the run, and what is still true once the work lands belongs in
`docs/` (`AGENTS.md` § "Documentation & Instructions Maintenance").

`## State` carries `round:`, `phase:`, `gates:` (the tokens a conditional
criterion or step names), `gates pending:`, `route tokens:`, `round base:`
(the `round_delta.py` snapshot for this round), `agents:` (each phase's agent
id, for the resume mechanism above), `reviewers:`, `open defects:`,
`patch rounds:`, `decisions in force:` and, on a plan that researches a game
mechanism, `ghidra mcp:` (`used — <what it answered>`, `unavailable — <what
status printed>; offered setup` or `skipped — <reason>`). `gates:` lists only
the gates that are set, or `none`; every gate a criterion may later need sits
on `gates pending:`, and possible research outcomes on `route tokens:`. A
`gates:` value holding `|` or "or" alternatives, or a `<placeholder>`, is a
template and sets nothing — a planner once wrote one, the verifier read every
gate as set, and the launch burned three rounds to `CAP` with no real defect.
The verifier now reports a criterion whose gate is not set as `UNATTEMPTED
(gate <x> not set)`, the workflow routes a round as `PASS-PENDING-HUMAN` when
every failure is gated on an unset gate, and `tools/workorder_audit.py` R19
flags whoever wrote a template line.

Rules learned from runs that hit the cap:

- **Only `BLOCKING` findings spend a round.** Every reviewer returns
  `blocking` findings, `non_blocking` findings, and a separate `plan_defect`
  flag — set only when no implementation of the plan as written could satisfy
  its Goal. A missing assert or pin the plan didn't forbid is a `BLOCKING`
  finding for the implementer, not a costly replan. Non-blocking findings ride
  along as context and surface in the final report.
- **A fix that is already known does not buy a full round.** When every
  BLOCKING finding of a round carries its reviewer's exact `fix`, and nothing
  else failed, the next round is a *patch round*: a `patch-implementer`
  applies the fixes, the verifier re-verifies by reach, and only the finding
  reviewers re-run. If `round_delta.py size` then shows at most 20 changed
  lines, no new file and no instrument path, the round is not counted against
  the cap. The same idea covers plans: a `PLAN-DEFECT` that states its own
  correction goes to an `amendment:` planner, and `tools/amend_check.py`
  decides from the files whether it stayed an amendment (Goal, scope and human
  questions untouched, at most 20 lines) or counts as a replan.
- **At the cap, split rather than raise.** The recovery is a new workorder
  carrying only the still-open findings, with its own fresh three rounds.
- **Triage warns on size.** The caps are per *workorder*, so an oversized plan
  puts unrelated work on one shared budget. Above 30 criteria or 3 independent
  findings, the driver says so before spawning.

**A workorder cannot be run against another checkout, and the driver stops
rather than trying** (`SKILL.md` Step 0.25). A session opened in a worktree
has every `Edit` and `Write` outside that worktree refused by the harness. In
the sister project an implementer that met the refusal routed every edit
through scratch byte-patch scripts instead of returning `PLAN-DEFECT` — 57 of
142 turns, double the round budget. The driver now names the two ways forward
(open the session in that checkout, or bring the work here), `planner.md`
refuses to write such a plan (`status: BLOCKED`), `implementer.md` makes the
refusal a `PLAN-DEFECT`, and audit R15 fails any run that carried on after it.

`.claude/skills/workorder/ensure_submodule.py` gives each worktree its own
submodule checkout and copies the gitignored build prerequisites that
`local_prereqs.json` names (empty, `{}`, today). ExilePact2 has no submodules,
so `SKILL.md`'s submodule step never applies here until one is added.

### Reading a workorder by section — `section.py` and `workorder_brief.py`

**`.claude/skills/workorder/section.py <file> '<heading>'` prints one section
of a workorder file** — heading to the next heading of the same or a higher
level, fenced code ignored by CommonMark's rule, CRLF and LF alike. The
heading is single-quoted because headings carry backticks, which Bash runs as
a command inside double quotes. A citation matches strictest first: the exact
text, then the text with backticks ignored, then a prefix (the way to cite a
heading with an apostrophe: stop before it). Exit 3 lists the file's headings,
4 names an ambiguous one, 5 refuses a file with an unclosed fence rather than
printing to its end, and 6 refuses `## Log` and everything under it unless
`--log` is passed. That is how the verifier follows a criterion's citation into
the context file without reading the implementer's reasoning. It reads any
markdown file: `--toc` lists every heading with its line and size, and `--grep
'<regex>'` prints only the list items and paragraphs of a section that match,
windowing an item over 4,000 characters around each match — guides with a few
enormous lines otherwise defeat a ranged `Read`.

**Code mode.** `section.py <file> --toc [--grep REGEX]` and `section.py <file>
'<symbol>' [--grep REGEX]` take a `.cpp .cc .c .hpp .h .py .js .mjs .ts` file,
built on `tools/source_index.py`'s symbol index: C/C++ functions at file scope
and in namespaces, out-of-line `Class::Method` definitions and
`struct`/`class`/`enum` bodies; Python through `ast`; JS/TS through a lexer
that knows strings, template literals, comments and regex literals. A symbol
prints `-- <path> lines <start>-<end> (<kind> <qualified name>)` and then its
lines exactly. Exit 3 lists near names, 4 every match of an ambiguous one, and
7 means `--grep` matched nothing. `implementer.md` and `planner.md` send their
agents here for any source file over about 200 KB: `--toc --grep`, then one
call per symbol, never `grep -n` followed by `sed -n`.

**`tools/workorder_brief.py <plan> <selector>` prints one implementer's slice
of a workorder in one call**, built on `section.py`'s heading matching,
`plan_lint.py`'s lane and item parsers and `run_criteria.py`'s criterion
numbering. The selector is `--round N`, `--lane NAME`, `--join`, `--item ID`
or `--paths P[,P...]`, with `--criteria`, `--context`, `--base REF`,
`--since-round K` and `--amended` adding to it. Every brief prints its size
against the plan's and context's, `## Goal`, `## Out of scope`, `## State`,
the selection, each Context subsection the printed steps cite by `ctx:`,
`### Decisions`, the `git diff` commands for the selection's files when a base
is given (printed, never run), and a footer naming what it left out. Exit 0
printed, 2 a usage error, 3 a lane, item or criterion that does not exist, 5 an
unclosed fence. `--launch-args <slug>` prints the JSON the driver passes to the
workflow. Every implementer prompt the workflow writes names its brief command
first, with the full files as the fallback: the Workflow tool cannot read a
file, so the prompt names a command rather than carrying the slice.

### Workflow mode — `workflows/workorder-rounds.js`

**Workflow mode is the default way steps 2–4 run.** `/workorder <task>` (after
the plan is approved) and `/workorder resume <slug>` call
`.claude/workflows/workorder-rounds.js` instead of driver turns — the user's
own `/workorder` invocation is the opt-in the Workflow tool requires. **Driver
mode — steps 2–4 as the driver's own turns — is the documented fallback**, used
when the Workflow tool is unavailable, the launch fails, or the user says
"driver mode" or "no workflow".

The round loop spawns no bookkeeping agents:

- **Bookkeeping:** the implementer runs `round_delta.py snapshot`/`heads`
  first and `delta`, the greps and `tools/decompile_check.py` last, and
  reports them in its schema.
- **Recording:** the launch returns `record {rounds:[{stateLines, logBlock}]}`.
  The driver applies it with `py -3 tools/workorder_record.py apply --run
  <runId>`, which is idempotent per run and round. (The scribe this replaced
  measurably lost State lines and damaged the Log in about a quarter of
  launches.)
- **Reviewers:** per § "Which reviewer runs when" above.
- **Fix rounds** continue inside one launch up to the caps.
- **Launch args:** the driver builds them with `py -3 tools/workorder_brief.py
  --launch-args <slug>`. It refuses with exit 6 and a `DISK-LOW` line when the
  checkout's drive has less than `--min-free-gib` (default 30) GiB free,
  pointing at `py -3 tools/disk_hygiene.py report`.
- **Criteria reuse:** a development verify passes `run_criteria.py --reuse
  <implementer criteria dir>`.

```
Workflow({ scriptPath: ".claude/workflows/workorder-rounds.js",
           args: { slug, planPath, contextPath, goalExcerpt, implementerModel, round,
                   reviewers: { '<name>': 'never' | 'clean' | 'blocking', ... },
                   researchHeadings, baseHeads, priorFindings, state,
                   lanes, join, items: [{ id, files, checks, after, shares, owner, default, reversible, build_reads }, ...],
                   streaming, answered, reviewScopes, finalGate,
                   maxParallel /* DEFAULT_MAX_PARALLEL, 4; 1-16 */, maxAgents, tokenCeiling, itemAttempts, reviewPassCap } })
```

`reviewers` is a map, one entry per applicable reviewer. `researchHeadings`
names the context file's `###` heading(s) `instrument-blindness-reviewer`
should read. `baseHeads` is `{ '.': sha }`, copied from `## State` › `round
base:`. `priorFindings` is `{ '<reviewer>': [{ where, problem }] }` for a
reviewer entering as `blocking`. `state` is the plan's `## State` section as
`section.py <plan> 'State'` prints it. A re-run reviewer that was blocking is
handed its own finding and asked whether the delta resolves it, and every
reviewer is told the Out-of-scope list is not a checklist. The script still
accepts a `submodules` list and per-repo base heads; with no submodules here,
only `.` is used.

**Lanes** are step groups a plan declares as `### Lane: <name>` headings with a
`files:` line, plus one `### Join`. `lanes` and `join` are pasted from `py -3
tools/plan_lint.py <plan> --lanes-json`, and only for the first implementation
of the plan's steps. The script runs one implementer per lane
(`implementer:<name>:r<n>`) in parallel; each stays inside its file set, runs
no git write and no full build or suite, and checks the cooperative stop marker
(`round_delta.py stopped`) before each step. A lane about to return
`PLAN-DEFECT` or `ADVICE-NEEDED` writes the marker first (`round_delta.py
stop`), and the others return `STOPPED`. When every lane returns `IMPL-DONE`,
the join commits each lane's file set as its own commit by pathspec and does
the `### Join` steps. Lanes that could not have come from `--lanes-json` are
`BAD-ARGS`.

**Items** replace rounds with a streamed pipeline for a plan whose `## Steps`
declares `### Item: <id>` groups, pasted from `plan_lint.py <plan>
--items-json`. Each item gets its own implementer the moment no running item
holds its files; items sharing a file queue in plan order. The implementer
commits only its own paths through `tools/item_commit.py`, then an independent
item verifier runs its `checks:`; a failure sends it back with the evidence, up
to three attempts. Reviewers read committed ranges pinned to the `HEAD` they
start from, and each finished reviewer's `BLOCKING` findings become a fixer on
the files they name. An item that stops parks alone, holding only what depends
on it. When nothing is left, the acceptance criteria run once, and a failure
there becomes one fix before the gate runs again. Which item may start is a set
of pure functions between the script's `@scheduler` markers, which the test
file cuts out and tests directly. An unanswered owner item with `reversible:
true` and a default runs on that default and is listed under `defaulted` with
how to undo it.

**Amendments run inside the launch.** An implementer's or fixer's `PLAN-DEFECT`
whose `CORRECTION:` is not `none` goes through `tools/amend_check.py save`, a
fresh planner labelled `amendment: <slug> <id>:r<n>`, and `amend_check.py
check`. Only exit 0 with `AMENDMENT` re-runs the work, one amendment at a time;
anything else goes back to the driver as a replan, after `amend_check.py
restore` has put the files back. A lane's and a reviewer's plan defect stay the
driver's.

The whole-tree verifiers start `run_criteria.py` in the background and poll it
with `--status <out> --wait 220`; one item's checks stay in the foreground.

It returns to the driver on anything needing judgement — `PASS`,
`PASS-PENDING-HUMAN`, `PLAN-DEFECT`, `ADVICE-NEEDED`, `AGENT-FAILED`, `PARKED`,
`CEILING` or `CAP` — so replans, consultations, human questions and the step-5
report stay with the driver (only a confirmed amendment runs inside). Every
re-entry inside the script is a fresh spawn, never the `SendMessage` resume,
because the script has no agent id to send to; measured in the sister project
as no worse than a resumed implementer.
`.claude/workflows/workorder-rounds.test.mjs` (`node --test`) dry-runs the
routing against stub agents.

**Step 5 audits the run's own cost.** The report step runs `py -3
tools/workorder_audit.py --latest` and prints every `FAIL` line verbatim
beside the round summary — a workorder that passes its acceptance criteria and
still breaks a cost/behavior rule says so.

### `tools/workorder_audit.py` — did this run actually save time and tokens

This makes "did a `/workorder` run save time and tokens, and did it break a
rule" runnable and repeatable, against the rules this page states:

```
py -3 tools/workorder_audit.py [--latest | --session <id-prefix> | --calibrate <list>]
    [--projects-dir DIR] [--project NAME] [--json]
```

`--session` looks in every checkout of the repository (the main checkout's
project directory and each `.claude/worktrees/<name>` one). `--calibrate
<file>` takes a list of session prefixes and prints each role's turns, tokens,
context per turn and list-price cost as p50/p75/p90/max, plus how many sessions
each rule fails at the current constants. The budgets are set from that output
([docs/agents/workorder-calibration.md](../docs/agents/workorder-calibration.md)).

It streams a session's transcripts: the driver's own
`~/.claude/projects/<project>/<session>.jsonl`, ad-hoc subagents at
`<session>/subagents/agent-*.jsonl`, and workflow-mode agents at
`<session>/subagents/workflows/wf_*/agent-*.jsonl`, each paired with a
`.meta.json` carrying `agentType` and a label. `--projects-dir`/`--project`
exist so tests never touch the real `~/.claude/projects`. Per agent it reports
turns (deduped by `message.id`), tokens, output tokens, context per turn, peak
context, wall minutes, the longest tool call, the model actually run, its
list-price cost (`MODEL_PRICES`), and KB of `Read` results by kind; for rounds
with lanes, a lane summary. Then it prints twenty-seven rules as `PASS`/`FAIL`
with evidence:

| Rule | Checks |
|---|---|
| R1 reviewer-reads-workorder | a reviewer `Read`/grep of a `-plan.md` (`instrument-blindness-reviewer` may read a `-context.md`) |
| R2 verifier-scope | a verifier whole-file `Read` or shell read of a `-context.md` or an oversized plan, or `section.py` with `--log` |
| R3 guide-whole | an agent whose guide `Read` results exceed a KB budget |
| R4 batching | an implementer's share of small sequential shell calls over budget |
| R5 blocking-call | a tool call over the time budget (except `Agent`/`Task` and `AskUserQuestion`) |
| R6 planner-rewrite | a planner `Write` to a plan/context path it already wrote |
| R7 / R8 / R9 reviewer- / implementer- / verifier-budget | turns, tokens, or (implementer only) context-per-turn over that role's budget |
| R10 driver-discipline | a driver that builds or tests, edits outside `.claude/workorders/`, or takes too many turns in one round — judged only while it is driving |
| R11 replans | two or more planner runs, not counting a confirmed `amendment:` planner |
| R12 plan-size | a plan or context file whose planner-authored part is over budget |
| R13 round-budget | one round's total subagent tokens over budget (round 0 larger; k implementers get k times) |
| R14 reviewer-reruns-suite | a reviewer running suites or builds more than twice |
| R15 edit-guard-workaround | a subagent that worked around the worktree edit guard instead of returning `PLAN-DEFECT` |
| R16 scribe-scope | a scribe that edited outside `.claude/workorders/`, touched git, or wrote through a shell |
| R17 live-operator-scope | a `live-operator` that wrote anything but its own `<slug>-live-<n>.md`, installed a build, ran a writing git command, restored a backup it did not take, force-stopped the game, or forced another holder's game lease |
| R18 scribe-state-preserved | a scribe `Edit` that dropped a `key:` State entry |
| R19 gates-template | a `gates:` line written as a template of alternatives or a `<placeholder>` |
| R20 live-capture-author | anyone but `live-operator` editing a `-live-<n>.md` capture |
| R21 verifier-interpreter | a verifier running `python`/`python3` instead of `py -3` |
| R22 verifier-suite-once | a verifier running the same suite more than once |
| R23 lane-git-mutation | a lane implementer (not the join) running a writing git command |
| R24 cheap-routes | an amendment without its `save`/`check`, two amendments with no implementer between, or two patch rounds back to back |
| R25 owner-scope | an `amend_check.py` `SCOPE:` verdict with no user message behind it |
| R26 reread-after-write | an implementer or planner re-reading a file it wrote, whole, more than twice — read `git diff -- <file>` instead |
| R27 amendment-tier | an `amendment:` planner run on an escalated variant or a non-opus model |

Every budget is a named module-level constant in the tool
(`IMPLEMENTER_MAX_TURNS`, `VERIFIER_MAX_TOKENS` and so on), each with a comment
naming the measurement it was set from — read those for the current number.
Each sits at about the 90th percentile of the sister project's measured runs,
so a `FAIL` means "this run is in the slowest tenth"; re-run `--calibrate` on
this project's own sessions once there are enough, and move a constant when its
percentile has moved. Exit 0 when every rule passes, 1 when any fails, 2 on a
usage error. Tests (`tests/test_workorder_audit.py`) give every rule a failing
fixture and a passing control.

### `tools/workorder_speed.py` — where the wall time went

The audit answers "did one session break a rule"; this answers "where did the
time go", for one session or many, using the audit's parser:

```
py -3 tools/workorder_speed.py [--transcript <driver .jsonl> ...] [--project-dir <dir> ...]
    [--plan <plan.md>] [--until <UTC>] [--json]
```

Each session and the aggregate carry span, busy and serial minutes,
concurrency, minutes per phase, each launch's item windows, verifies by kind,
owner waits, routes (amendments, replans, consultations) and `workorder_reads`
— how each role read the workorder's own files, which shows whether the brief
and the digest moved reading off the whole files. The module docstring defines
each figure. Tests: `tests/test_workorder_speed.py`.

### `tools/plan_lint.py`, `run_criteria.py`, `live_checks.py`, `amend_check.py` — criteria that read, not guess

**`plan_lint.py <plan>`** checks `## Acceptance criteria` for defects that
each cost a round: a prose criterion, a heading slice not anchored on `\n`, a
grep over a live capture, `python` where the repository runs `py -3`,
`pinned-sha` (a bare commit hash where a tag or merge-base belongs),
`eof-slice`, `merge-walk` (a `git log` range without `--first-parent` in a plan
that merges main) and `plan-draft`. It reads lanes (`lane-overlap`,
`lane-no-files`, `lane-no-join`, `lane-dup-name`, `lane-bad-name`; `--lanes-json`
prints only when clean) and items (`item-overlap`, `item-no-files`,
`item-no-checks`, `item-dup-id`, `item-bad-id`, `item-unknown-ref`,
`item-cycle`, `items-and-lanes`; `--items-json`, with `--known a,b --wait S`
for a streaming planner). Owner questions carry a default: `owner-no-default`,
`owner-no-reversible`, `owner-reversible-no-default`, `owner-legal-default`
(a legal or decompile-output question is never reversible). A plan that
researches a game mechanism must carry a `ghidra mcp:` State line
(`ghidra-unchecked`, `ghidra-bad-value`); the trigger is broad on purpose (a
`### Live procedure`, `route tokens:`, `analyzeHeadless`, `decomp_index.py
has`, `mcp__ghidra__`), and a plan that only mentions these tools clears it
with one `skipped — <reason>` line. It warns `no-reads` and `reads-nothing`
about a criterion's `(reads ...)` map. The planner runs it before
`PLAN-READY`; the driver runs it again before spawning an implementer.

**`run_criteria.py <plan>`** is the verifier's first call. It runs every
command-shaped criterion exactly as written, in bash from the checkout root,
once per distinct command, skips a criterion whose gate is not set, prints each
exit code with the output's tail, keeps full logs as `cmd-<n>.log`, and judges
nothing. `--jobs N|auto` runs independent commands at once by resource class
(builds first and alone, at most one whole suite and `--browser-jobs` browser
suites), declared per criterion as `(class <c>)`/`(after <k>)` or recognised
from the command. `--item <id>` runs one item's `checks:`. `--dev` skips whole
suites and `(final)` criteria; `--changed-since <ref>`/`--changed-from <file>`
plus `--failed <k,...>` run only the criteria a fix can reach, falling back to
all when the delta is unknown; `--reuse <dir>` reuses an implementer's
results. Every run keeps `<out>/status.json` and `<out>/report.txt`;
`--status <out> [--wait S]` exits 0 finished, 3 running, 4 stale, 2 no status
(S ≤ 220 keeps a poll under audit R5), and `--digest <out>` prints one line per
criterion that passed as expected and the full block for the rest, so verifiers
stop reading 100–200 KB reports whole.

**`live_checks.py <capture> --expect <names> [--require-pass <names>]`** reads
a live capture's `## Checks` block: the verdict is the first word after a
line's last `|`. It exits 1 on a missing, renamed, duplicated or unreadable
check, or a `--require-pass` check that did not pass; a research check's
`fail`, `not-observed` or `not-run (instrument: …)` is a finding and exits 0.
It replaces punctuation-sensitive greps that once cost three rounds.

**`amend_check.py save|check|restore <plan> [<context>]`** decides whether a
plan change was an amendment, the owner's own scope (`SCOPE:`, when it follows
an `owner, <date>:` decision line the context gained since `save` — exempt from
the replan cap and the tier ladder, and audit R25 checks the owner did say
something), or a replan. `restore` copies the saved files back.

**`item_commit.py --message <m> -- <paths>`** commits one item's paths under
the checkout's commit lock; **`workorder_lock.py <name> -- <cmd>`** is that
lock (an OS file lock dropped when its holder exits). Tests:
`tests/test_workorder_plan_tools.py`, `test_workorder_plan_lint_owner.py`,
`test_workorder_plan_lint_ghidra.py`, `test_workorder_run_criteria_status.py`,
`test_run_criteria_reuse.py`, `test_workorder_record.py` and
`test_workorder_launch_args.py`.

### `tools/source_index.py` — go to the range, don't grep around

Generic, stdlib-only, read-only index for one large C/C++ file, built around
banner comments and `#ifndef <guard>` research spans:

```
py -3 tools/source_index.py <file> [--regions] [--functions]
                                    [--find NAME] [--at LINE]
                                    [--guard MACRO] [--json]
```

`--regions` (the default) prints one line per banner-delimited region —
`start-end  KB  [R]  title`; `[R]` marks a region inside a `--guard`-named
`#ifndef` span. `--functions` finds file-scope definitions with a
brace-matching heuristic (its docstring lists what it misses). `--find NAME`
returns every region and function whose name contains `NAME`, with ranges, so
the next call is a ranged `Read`. `--at LINE` returns the region and function
containing a line. It only prints identifiers and banner titles, never the
file's text. It also backs `section.py`'s code mode (above). Tests:
`tests/test_source_index.py`, `tests/test_section_code_mode.py`.

### Research and live-session tools

- **`tools/decomp_index.py`** — `has <name>` (`--contains`, or an address)
  answers whether a function was already decompiled for this build and where
  the file is; `scan <outdir>` records a headless run's output. The index and
  every decompile stay outside any repository; the tool refuses to write
  inside a git checkout ([docs/tools/decomp-index.md](../docs/tools/decomp-index.md)).
- **`tools/decompile_check.py <slug> <round>`** (or `--paths`) — the
  per-round half of `decompile-output-guard`: runs the hook's own signatures
  over the lines added since a recorded base, because by the end of a round the
  implementer's commits have emptied the hook's working-tree view.
- **`tools/game_lease.py`** — one machine-wide game lease under
  `%LOCALAPPDATA%\ExilePact2\lease.json`, so two sessions never drive one
  install at once: `py -3 -m tools.game_lease acquire|status|release`. A held
  record names its holder by PID and process start time, so a crashed holder
  reads as gone and an unreadable record never reads as free (`acquire
  --force` is the way out). `tools/x64dbg_mcp.py` attaches only under a held
  lease.
- **`tools/thread_state.py <pid>`** — reads one process's threads from the OS
  with query-limited access and touches nothing; tells a running game from a
  held one, and a clean debugger detach from one that left threads suspended.
- **`tools/freeze_probe.ps1`** — samples the machine once a second while the
  game window stops responding, to find what is busy during a stall from
  outside the game. Its text still describes the sister project's game.

## Plugins — `settings.json`

`settings.json` registers one marketplace (`extraKnownMarketplaces`) and
enables one plugin from it (`enabledPlugins`). Claude Code offers to install it
when a contributor trusts the repository.

| Plugin | Source | For |
|---|---|---|
| `claude-code-setup` | `anthropics/claude-plugins-official` | recommending Claude Code automations for this codebase |

## MCP servers — `../.mcp.json`

| Server | For |
|---|---|
| `ghidra` | the local Ghidra project as tools (search functions, decompile by address, callers, callees, xrefs, strings), from [bethington/ghidra-mcp](https://github.com/bethington/ghidra-mcp) v6.0.0 pinned by sha256: one shared headless server on `127.0.0.1:8089` over a *copy* of the research project, plus a stdio bridge per session, started by `tools/ghidra_mcp.py` |
| `x64dbg` | a live debugger on the running game, for `live-operator` only: headless x64dbg with the [AgentSmithers/x64DbgMCPServer](https://github.com/AgentSmithers/x64DbgMCPServer) plugin, both pinned, attached under the game lease, and eight tools of our own served over stdio by `tools/x64dbg_mcp.py`, which forwards to the plugin on `127.0.0.1:50300` — see [`docs/tools/x64dbg-mcp.md`](../docs/tools/x64dbg-mcp.md) |
| `context7` | live library documentation, pinned to `@upstash/context7-mcp@4.1.1` |
| `playwright` | driving a browser; pinned to `@playwright/mcp@0.0.82`, run headless on Microsoft Edge (`--browser msedge --headless`) because Edge ships with Windows and the default, Chrome, often is not installed, and with `--isolated` (an in-memory profile per server) because concurrent sessions in `.claude/worktrees/` otherwise collide on one shared profile |
| `app-webview` | driving one of this repository's own Tauri apps, for `live-operator`: the same pinned `@playwright/mcp@0.0.82` attached over CDP to `127.0.0.1:9333` (`--cdp-endpoint`), where only an app started by `py -3 tools/app_webview.py launch <exe>` listens (it sets `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` for that process alone); never the game — see [`docs/tools/app-webview-mcp.md`](../docs/tools/app-webview-mcp.md) |
| `github` | issues, pull requests and Actions for this repository, over GitHub's hosted MCP endpoint |

`ghidra` is a local entry, `py -3 -m tools.ghidra_mcp`. It needs Ghidra, a JDK
21 and the research project, plus `py -3 -m tools.ghidra_mcp setup` once per
machine, which downloads the pinned release (ask the owner first). The first
session to use it starts a shared headless server, and later sessions,
worktrees and Codex reuse it, because a Ghidra project allows one lock. It
binds to loopback, has script execution off, and serves a copy of the project,
so headless runs on the original keep working. What it decompiles stays out of
tracked files (`AGENTS.md` § "Legal"). Under Codex, whose MCP startup timeout
is 10 s, run `py -3 -m tools.ghidra_mcp start` first if the server is cold.
`py -3 -m tools.ghidra_mcp status` says what is missing.
[`docs/tools/ghidra-mcp.md`](../docs/tools/ghidra-mcp.md) explains the choice
of server, the overrides and the sharp edges. Which agents may call which of
its tools is in § "Agents" above.

`x64dbg` is the other local entry, `py -3 -m tools.x64dbg_mcp`, a stdio proxy
that is connected from session start and answers "not attached" until a
debugger session exists. It needs `py -3 -m tools.x64dbg_mcp setup` once per
machine, which downloads the pinned x64dbg snapshot, clones the plugin at its
pinned commit, applies our two edits (loopback bind, braces in log lines) and
builds it with MSBuild and a .NET 8 SDK (ask the owner first). Everything it
installs stays under `%USERPROFILE%\tools\`, outside any checkout. Attach and
teardown run through a detached keeper (`attach --game`, `detach`), under the
game lease (`tools/game_lease.py`), and the debugger is detached **before** the
game is closed. It is not an HTTP entry pointing at the plugin's own server,
because that server exists only after the attach and exposes the plugin's
whole surface (`StopDebug`, memory writes, stepping) to every session; our
eight tools are the boundary in code. What x64dbg disassembles stays in the
live capture (`AGENTS.md` § "Legal"). Remember that attaching a debugger to an
online game can breach its terms of service (`AGENTS.md` § "Legal").

The Python servers need `py -3 -m pip install -r tools/requirements-mcp.txt`
once, and assume Claude Code starts a project-scoped stdio server with the
project root as its working directory.

`github` does **not** authenticate interactively. Claude Code tries OAuth
dynamic client registration, the endpoint does not support it, and the session
reports *"Incompatible auth server: does not support dynamic client
registration"*. A POST with a bearer token returns 200, without one 401. So
`.mcp.json` sends `Bearer ${GITHUB_MCP_PAT}`, expanded from the environment —
no token in the repository.

### Tokens: `tools/setup_agent_secrets.py`, once per machine

`github` reads a personal access token from `GITHUB_MCP_PAT`, so no token is in
the repository and Claude Code and Codex read the same one. On a new machine
run:

```bash
py -3 tools/setup_agent_secrets.py
```

On Windows it asks, with input hidden, for the token if it is not saved yet and
saves it as a persistent user variable in `HKCU\Environment`. A value set only
in the current shell does not count, because newly started programs will not
see it. `--list` shows `set`, `SESSION` or `MISSING`, and `--force` asks again.
Pressing Enter at the prompt uses `gh auth token` instead. On macOS and Linux
it saves nothing: it prints the `export` line to add to your shell profile. A
test fails if a server in `.mcp.json` reads a variable the script does not ask
for.

**Claude Code and Codex must be restarted afterwards.** A process reads its
environment at launch, so the session that sets the variable is never the
session that can use it.

Three things worth knowing about the `gh auth token` route:

- **It is a copy, and copies go stale.** `gh auth refresh`, `gh auth logout` or
  a re-login rotates `gh`'s token, and this copy then 401s while `gh` itself
  keeps working. Re-run `tools/setup_agent_secrets.py --force` to resync.
- **It is plaintext at rest**, in the user's registry environment, readable by
  anything running as that user — a deliberate downgrade from `gh`'s keyring,
  accepted for convenience.
- **It is broadly scoped** across every repository the account can reach. A
  fine-grained PAT restricted to this repository narrows the blast radius and
  drops into the same variable.

Until the variable is set the entry simply fails to connect, which is
harmless — the `gh` CLI covers the same ground.

## Codex — `../.codex/` and `../.agents/`

The same rules and tooling reach Codex. `AGENTS.md` is the rule file for both
agents (Claude Code reads it through `CLAUDE.md`'s import).

| Claude Code | Codex | How the two stay in step |
|---|---|---|
| `.claude/skills/` | `.agents/skills/` (the source; only a `.gitkeep` today) | `tools/sync_agent_tooling.py` mirrors it |
| `.claude/agents/*.md` (the source) | `.codex/agents/*.toml` | generated by the same tool |
| `.mcp.json` (the source) | `.codex/config.toml` `[mcp_servers.*]` | generated by the same tool |
| `settings.json` `PostToolUse` | `.codex/hooks.json` `PostToolUse` | both run `hooks/post_tool_use.py`; a test pins it |

Run `py -3 tools/sync_agent_tooling.py` after editing any source; `--check`
only reports. Codex loads `.codex/` only once you trust the project, and asks
you to approve `.codex/hooks.json` in `/hooks` (again whenever it changes).
Start Codex at the repository root: the hook command is a relative path.

The generated agents keep each Claude agent's instructions verbatim, with a
preamble translating tool names. A Claude agent without write tools becomes
`sandbox_mode = "read-only"`. Its `effort:` becomes `model_reasoning_effort`
(`max` becomes `xhigh`); a Haiku-tier agent that pins no `effort:` falls back
to `low` (`TIER_DEFAULT_EFFORT`). Effort variants get a Codex twin like any
other agent. The Claude model tier is not translated: Codex has no fixed model
per tier, so the agent inherits the session's model, and the tier is recorded
in a comment.

What does not carry over:

- **`/workorder` and `workorder-rounds`** drive Claude Code's `Agent` and
  `Workflow` tools and their scripts are called by `.claude/skills/workorder/`
  path, so they stay Claude-only. Under Codex, run the phases by hand with the
  generated `planner`, `implementer` and `verifier` agents.
- **The `ai-review` CI job** (`.github/workflows/ai-review.yml`) runs Claude
  Code in GitHub Actions whichever agent opened the pull request; the label
  works the same for both.
- **The `tools:` allowlist, and with it the `mcp__ghidra__*` read set.**
  `tools/sync_agent_tooling.py` reads an agent's `tools:` line only to decide
  `sandbox_mode`, so every Codex agent sees every server in
  `.codex/config.toml` with all its tools, the `ghidra` server's writes and
  debugger tools included. A server-wide `disabled_tools` would strip the
  owner's own Codex session too, so the restriction there is the ban written
  in the agent bodies. The `x64dbg` server is narrower by construction: its
  stdio proxy serves only its eight tools.
- **`omitClaudeMd: true`** on the verifier, the scribe and the three reviewers
  has no Codex equivalent. `tools/sync_agent_tooling.py` drops it and notes
  this in the header comment, so under Codex those agents still load
  `AGENTS.md`. Their bodies restate the rules they apply, so they behave the
  same either way. The sister project's measurement of what loading
  `AGENTS.md` costs per spawn lives in its own
  `docs/agents/workorder-audit-2026-10/`, which was not copied here.

## A trap worth knowing: `#` in frontmatter

An agent or skill `description:` is a **plain YAML scalar**, so a space followed
by `#` opens a comment and everything after it is silently dropped — no error,
no warning, and the file still loads. A sister-project reviewer shipped with
`adds or edits a #[tauri::command]` in its description and lost the last 86
characters, which were the trigger conditions that make the agent get picked at
all. The fix is a **double-quoted scalar**: the attribute stays readable in the
trigger text, and the quoting stops YAML treating the `#` as a comment. Do the
same for any `description:` containing attribute syntax, a shell flag, or a URL
with a fragment — quote the value rather than rewording around the character,
and never put authoring notes in `description:` itself, since that text is what
gets matched against.

To check a file: strip the frontmatter and look for an unquoted ` #` in it.

## Changing any of this

Twenty-seven suites cover this page's tooling. Twenty-six are Python and run
under the first command below; the workflow script's routing is JavaScript and
runs separately, under Node:

```bash
py -3 -m unittest discover -s tests
py -3 -m unittest tests.test_claude_hooks -v              # the hook actually blocks, and the dispatcher agrees with it
py -3 -m unittest tests.test_claude_agents -v             # the agent definitions are well-formed
py -3 -m unittest tests.test_agent_tooling_sync -v        # the Codex copies (.agents/, .codex/) match their sources
py -3 -m unittest tests.test_ai_review_workflow -v        # the ai-review workflow's trigger and label rules
py -3 -m unittest tests.test_claude_workorder -v          # round_delta.py (and ensure_submodule.py), as subprocesses
py -3 -m unittest tests.test_claude_workorder_section -v  # section.py, plus the agent and SKILL.md sentences that carry the same lesson
py -3 -m unittest tests.test_section_code_mode -v         # section.py's code mode: one fixture per language, the size test, the toc cap
py -3 -m unittest tests.test_source_index -v              # source_index.py against a synthetic fixture
py -3 -m unittest tests.test_workorder_audit -v           # workorder_audit.py's rules, each with a failing fixture and a passing control
py -3 -m unittest tests.test_workorder_speed -v           # workorder_speed.py's measures on synthetic sessions
py -3 -m unittest tests.test_workorder_brief -v           # workorder_brief.py's selectors, each with a control
py -3 -m unittest tests.test_workorder_launch_args -v     # workorder_brief.py --launch-args
py -3 -m unittest tests.test_workorder_record -v          # workorder_record.py apply, idempotent per run and round
py -3 -m unittest tests.test_workorder_plan_tools -v      # live_checks.py, plan_lint.py, amend_check.py, run_criteria.py
py -3 -m unittest tests.test_workorder_plan_lint_owner -v # plan_lint.py's owner-question rules
py -3 -m unittest tests.test_workorder_plan_lint_ghidra -v  # plan_lint.py's ghidra-unchecked / ghidra-bad-value
py -3 -m unittest tests.test_workorder_run_criteria_status -v  # run_criteria.py's status.json, report.txt, --status and --digest
py -3 -m unittest tests.test_run_criteria_reuse -v        # run_criteria.py --reuse
py -3 -m unittest tests.test_decomp_index -v              # decomp_index.py, including its refusal inside a git checkout
py -3 -m unittest tests.test_decompile_check -v           # decompile_check.py over added lines since a base
py -3 -m unittest tests.test_ghidra_mcp -v                # the ghidra launcher: git refusal, loopback, stripped env, version check
py -3 -m unittest tests.test_ghidra_agent_tools -v        # the phase agents' mcp__ghidra__* tools match AGENT_READ_TOOLS
py -3 -m unittest tests.test_x64dbg_mcp -v                # the x64dbg launcher, keeper, lease gate, holds, teardown and allowlist
py -3 -m unittest tests.test_x64dbg_agent_tools -v        # live-operator alone carries mcp__x64dbg__*, exactly LIVE_OPERATOR_TOOLS
py -3 -m unittest tests.test_game_lease -v                # the machine-wide game lease
py -3 -m unittest tests.test_thread_state -v              # the read-only outside check of a process's threads
node --test .claude/workflows/workorder-rounds.test.mjs   # workflow mode's routing
```

The `test_x64dbg_*` suites and `test_thread_state` run on fixtures and fakes: a
stand-in plugin on `127.0.0.1`, a fake headless (`tests/x64dbg_fake_headless.py`),
a fake probe behind the outside check's seam, and a guard that fails any
non-loopback fetch or unexpected subprocess. Without the `mcp` package or off
Windows, the stdio round trip and the Windows-only `real_os` tests skip, each
naming why; everything else runs.

**`.claude/workflows/*.js` and `*.mjs` must stay LF.** `.gitattributes` forces
`text eol=lf` on both globs: the Workflow tool's permission handler refuses to
schedule a script containing any CR byte at all — "script contains control
characters that would be hidden in the approval dialog" — so a CRLF
`workorder-rounds.js` could never be launched, however correct its content.
Check with `file` after editing either script rather than assuming the
checkout's usual `core.autocrlf` behavior carried over.

`test_claude_agents.py` enforces the two rules on this page that a machine can
check: every agent pins `model:` to a known tier alias, and no frontmatter
carries an unquoted ` #` that would silently truncate the value. Both are
invisible in a diff. It self-tests its own parser, for the same reason
everything else here does.

`test_claude_workorder.py` drives `round_delta.py` as the subprocess the driver
and the workflow script actually invoke, not by importing its functions — the
same discipline `test_claude_hooks.py` uses for the hooks.

Every hook test is a **pair**: a positive control proving the hook fires on a
real violation, and a negative control proving it stays quiet on a clean tree.
A suite with only the negative half passes against hooks that do nothing, which
is exactly the failure `AGENTS.md` § "Prove the Instrument Before Trusting a
Negative Result" warns about. If you add a check, add both halves.

The rig builds a throwaway git repository in the system temp directory rather
than the session scratchpad, where `git init` fails with `Filename too long`.
