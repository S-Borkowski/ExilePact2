# /workorder: model tiers, and why

Read this when choosing or changing a tier, or when a cost question comes up. `SKILL.md` § "Model tiers" keeps the spawn table every launch needs.

Incidents, dates and figures cited here (ForgePact, `forgepact-*` workorder slugs, the UI redesign, the bug batch) are from the sister project hero-siege-offline-toolkit, where this pipeline was calibrated; the procedure itself is engine-independent.

Set in each agent's frontmatter, with an `effort:` beside every tier that
takes one: `planner` fable/medium, `implementer` fable/medium, `consultant`
fable/medium, `instrument-blindness-reviewer` fable/medium, the other reviewers
opus/medium, `live-operator` opus/medium, `verifier` haiku/medium, `scribe`
haiku/low.
Each fable agent has a generated `-opus` twin at the opus tier it replaced
(`fallback:` in its frontmatter), spawned when fable is unavailable.
Override the model for one run by passing `model` on the Agent call.
Effort has no per-call override on the Agent tool, which is why it is pinned
— an agent without one inherits whatever the session runs at.

**Effort variants choose effort per spawn.** An agent's
`effort-variants:` line lists the other levels /workorder runs it at, and
`py -3 tools/sync_agent_tooling.py` writes one `<agent>-<level>.md` for each,
same body, own `name:` and `effort:`; `tests/test_agent_tooling_sync.py`
fails a variant that drifted from its source. Today: `planner-medium`,
`planner-xhigh`, `planner-max`, `implementer-medium`, `consultant-max`,
and an `-opus` fallback twin for every agent or variant on `fable`.
Spawn the variant by name, in a driver's `Agent` call and in
`workorder-rounds.js` alike. A Workflow `agent()` call does accept an
`effort` option, but whether it beats the agent's own pinned `effort:` is
not documented, and a transcript records no effort level at all, so the
name is the one spelling that both runs at that effort and lets
`workorder_audit.py` see it (`split_effort_variant`; `--calibrate` reports
per role, model *and* effort). To add a level, add it to the source's
`effort-variants:`, re-run the sync, and name it in a table here.



**The driver is the session itself**, so no frontmatter pins it: run the
session that runs /workorder on `opus` at `medium` effort, and raise it to
`high` for a session whose product is the driver's own judgement (triaging a
bug batch into items, deciding how to split a plan). Drivers were 30% of the
calibration set's spend over hundreds of turns, many above 300K tokens, and
effort is paid on every one of them. Their job is routing by written rules;
the hard calls already go to `consultant` and replans to the planner, and the
driver failures the audit recorded (R10) were discipline, not reasoning,
which more effort does not fix. Chosen 2026-10-06 (#436), not measured.

## Tiers from score against cost (owner, 2026-11-10)

Every tier is read off the Artificial Analysis Engineering Index against
weighted average cost per task (read 2026-11-10; values estimated from the
chart, `*` marks the points the chart itself stars):

| Model (effort) | Engineering Index | Cost per task |
|---|---|---|
| Fable 5.2 (high) | ~69.5 | ~$6.6 |
| Fable 5.2 (medium) | ~64.5 | ~$2.75 |
| Fable 5.2 (low) | ~61.5 | ~$2.3 |
| Opus 5.5 (medium) | ~53.5 | ~$1.85 |
| Sonnet 5.5 (high) | ~46 | ~$1.3 |
| Haiku 5.5 (max)* | ~42.5 | ~$0.31 |
| Haiku 5.5 (xhigh)* | ~40.5 | ~$0.18 |
| Haiku 5.5 (high)* | ~36.5 | ~$0.12 |
| Haiku 5.5 (medium)* | ~32 | ~$0.065 |
| Haiku 5.5 (low)* | ~27 | ~$0.03 |

The rule: a role whose product is its own judgement buys score; a role that
applies a written correction, follows a written procedure or runs commands
buys the cheapest point that does the job.

| Role | Tier | Why |
|---|---|---|
| `planner`, `implementer`, `consultant`, `instrument-blindness-reviewer` | fable / medium | judgement every later phase is held to; +11 points over Opus medium for +49% a task, where Fable high costs 2.4x Fable medium for +5 |
| `planner-xhigh`, `planner-max`, `consultant-max` | fable / high | the hard triage rows and the second replan, where cheaper reasoning has failed or a wrong model costs whole sessions; rare enough that 2.4x a task is the cheapest place to buy score |
| `planner-medium`, `implementer-medium`, the driver | opus / medium | amendments and patch rounds apply a correction already written down, and the driver routes by written rules; Fable low scores 8 more for +24%, which these jobs do not need |
| `docs-sync-reviewer`, `decompile-output-guard` | opus / medium | a checklist against a diff; Haiku max was tried for one workorder and cost more than this, ran 2-5x longer and broke the R7 budget on 14 of 17 runs (§ "Haiku max reviewers, measured" below) |
| `live-operator` | opus / medium | a written live procedure; its prompts live above 100K (measured below), where Haiku bills 5x, and on its own tokens Opus costs 1.26x Sonnet for +6 Intelligence Index points (below), so not Sonnet either |
| `verifier` | haiku / medium | runs the criteria and reports what printed |
| `scribe` | haiku / low | pastes precomputed text (retired from the loop) |

Measured 2026-11-10 on every local transcript of these roles (this repo
and both sister projects; tokens as the opus/sonnet/fable runs spent them,
repriced at list):

| Role | Runs | Prompt/turn p50 / p90 / p99 | Turns > 100K | Runs touching > 100K | Sonnet 5.5 / run | Haiku 5.5 / run |
|---|---|---|---|---|---|---|
| `docs-sync-reviewer` | 919 | 43K / 86K / 126K | 4.8% | 6% | $0.237 | $0.015 |
| `decompile-output-guard` | 891 | 38K / 85K / 152K | 5.6% | 6% | $0.175 | $0.012 |
| `live-operator` | 178 | 127K / 344K / 579K | 62.7% | 78% | $2.92 | $0.60 |
| `verifier` (control, already haiku) | 2612 | 32K / 58K / 79K | 0.1% | 0% | — | $0.006 |

The two reviewers stay under 100K on 95% of their turns, and the long-prompt
card adds about a quarter to their Haiku bill, so Haiku is still about 15x
cheaper than Sonnet for them. `live-operator` lives above 100K (its p50 is
127K, peaks to 700K): Haiku would bill most of its turns at 5x and run them
in the long-context regime the reviewers avoid, for about 5x saving on a role
that spends the owner's time in the game, so it stays off Haiku. Haiku max is
used rather than xhigh because it is the chart's closest point to Sonnet high
(~42.5 against ~46) for $0.31 against $0.18 a task. Not established: whether
Haiku's findings hold up on the 5% of reviewer turns past 100K; a reviewer
miss there shows up in `--calibrate` and in the AI review that follows every
pull request, and `decompile-output-guard` has the mechanical
`decompiled_output.py` hook behind it.

**Haiku max reviewers, measured (MegaBolt's `arena-slot-table`, 2026-11-11).** The
forecast above repriced other models' token counts; Haiku spent its own, and
the forecast did not hold:

| Role | Runs | Turns > 100K | Turns / tokens vs Opus-medium average | Actual $ / run | Opus-medium $ / run (historical) |
|---|---|---|---|---|---|
| `docs-sync-reviewer` | 10 | 71% (forecast 4.8%) | 2.4x / 4.5x | $0.79 | ~$0.35 |
| `decompile-output-guard` | 7+1 | 50% (forecast 5.6%) | 2.4x / 3.8x | $0.29 | ~$0.26 |

At max effort Haiku explored instead of checking: up to 107 turns, 26M
tokens and a 423K peak prompt in one run, 10-18 minutes of wall clock per
review, and 14 of 17 runs over the R7 budget. Its long prompts put most
turns on the 5x card, so it cost more than Opus medium, not 15x less. Both
reviewers are back on opus/medium. Lesson for the next tier move: a
cheaper model is judged on the tokens *it* spends on the task, measured on
one real workorder, not on another model's tokens repriced.

**Checked against the Intelligence Index too (2026-11-10).** Artificial
Analysis's release comparison of the four models at every effort (Intelligence
Index v5.0.1 against cost per task) gives:

| Effort | Fable 5.2 | Opus 5.5 | Sonnet 5.5 | Haiku 5.5 |
|---|---|---|---|---|
| max | 65 / $26.54 | 59 / $10.04 | 55 / $10.02 | 36 / $0.41* |
| xhigh | 62 / $14.83 | 57 / $6.05 | 50 / $4.09 | 35 / $0.25 |
| high | 62 / $9.08 | 52 / $3.46 | 42 / $1.64 | 30 / $0.16 |
| medium | 56 / $3.80 | 48 / $2.45 | 31 / $0.78 | 25 / $0.09 |
| low | 54 / $3.10 | 35 / $0.98 | 26 / $0.51 | 19 / $0.04 |

Four of Sonnet's five levels are beaten on both score and cost: medium by
Haiku max, xhigh by Opus high and Fable low, max by Fable medium. Sonnet
high (42 / $1.64) is the one point nothing beats outright, and Opus medium
gives 6 more for 49% more a task. On this pipeline's tokens the gap is
smaller: repriced at list, `live-operator`'s measured runs cost 1.26x on
Opus what they cost on Sonnet (97.6% of its prompt tokens are cache reads,
which both bill at $0.20), so no agent runs Sonnet. The same table says
why some points are never used: Fable xhigh scores what Fable high does for
63% more, and Opus high is beaten by Fable low (54 / $3.10 against 52 /
$3.46), so opus/high survives only as the fallback twin of the fable
agents.

Every agent on `fable` keeps an `-opus` twin at the opus tier it replaced
(`fallback:`, and the `|` half of an `effort-variants:` entry), spawned once
when fable is unavailable. Opus and sonnet agents have no twin. The chart's
cost per task is mostly output and thinking; here 92-99% of tokens are
cache reads, so the real gaps between models are smaller than the chart's
(§ "Where Opus 5.5 fits"), and `--calibrate` per role and model is what
should move a tier next.

Fable everywhere (2026-11-10, while the owner had a free Fable token reset)
lasted one session and was reverted the same day to this table.

**Fable left this pipeline on 2026-10-05**, when the owner's license
stopped carrying it, and **came back on 2026-11-09** (the owner's call,
for Fable 5.2 through the `fable` alias): what ran opus/max (`planner-max`,
`consultant-max`) now runs fable/high, and what ran opus/high (`planner`,
`implementer`, `instrument-blindness-reviewer`) runs fable/medium. Every
other tier is unchanged. Each of the five keeps its old opus tier as a
generated twin (`planner-opus`, `planner-max-opus`, `implementer-opus`,
`consultant-max-opus`, `instrument-blindness-reviewer-opus`) that the driver
and `workorder-rounds.js` spawn once when a fable spawn fails for its model,
so a lapsed license costs one failed spawn, not a stuck pipeline. A source
names its variants' models in `effort-variants:` (`max=fable/high|opus/max`:
level, its model/effort, its twin) and its own twin in `fallback:`. Not
measured: `--calibrate` will say whether fable at one effort level down
earns its price. `workorder_audit.py` has no Fable 5.2 price row yet (the
price was not in the `claude-api` model table on 2026-11-09), so those
transcripts go unpriced until one is added to `MODEL_PRICES`.

**Where Opus 5.5 fits (2026-09-22).** `opus` resolves to Claude Opus 5.5; it
did so already for the last sessions of the calibration set, with no file
changed. What it changes is the price, and price is what the tiers were
chosen by. List prices per million tokens (input / output / cache read):

| Model | In | Out | Cache read |
|---|---|---|---|
| Fable 5.1 (`fable`) | $10 | $50 | $0.25 |
| Opus 5.5 (`opus`) | $4 | $20 | $0.20 |
| Opus 5 (was `opus`) | $5 | $25 | $0.50 |
| Sonnet 5.5 (`sonnet`) | $2 | $10 | $0.20 |
| Sonnet 5 (was `sonnet`) | $2 | $10 | $0.20 |
| Haiku 5.5 (`haiku`), prompt ≤ 100K tokens | $0.10 | $0.50 | $0.01 |
| Haiku 5.5 (`haiku`), prompt > 100K tokens | $0.50 | $2.50 | $0.05 |
| Haiku 4.5 (was `haiku`) | $1 | $5 | $0.10 |

92-99% of every role's tokens here are cache reads, so for this pipeline
Opus 5.5 costs about what Sonnet 5 does, and 40-60% less than Opus 5 did:
the 20 Opus 5 drivers and the Opus-5 planners of the calibration set would
have cost about half as much. Hence:

- **implementer → `opus`.** Measured on the 22 sessions: Sonnet 5
  implementers averaged $4.65 a run with a tail to $18.90 (p90 46.8M tokens,
  max 78.9M — the runs that hit round caps); Opus-tier ones used 36% fewer
  tokens on average (11.2M against 17.4M) while carrying the hard triage
  rows, and the four Opus 5.5 runs averaged $4.82. Same money, no tail.
- **planner, consultant, instrument-blindness-reviewer stay `opus`**, now
  cheaper. Until 2026-10-05 `fable` kept the two hard triage rows plus the
  second replan; those are now `planner-xhigh` and `planner-max`.
- **amendment planner → always `planner-medium`** (with `model: opus` passed
  explicitly until 2026-11-10; now its own pin, opus/medium, which R27 checks).
  An amendment planner always runs as planner-medium, never escalated.
  It applies one correction someone already stated, so the replan ladder
  does not reach it: spawn it as `planner-medium` whatever `planner-tier=`
  the State records. In the 18 sessions after 2026-10-02 one amendment ran
  fable because the driver carried an escalated tier over; fable planners
  averaged 8.49M input tokens a run there. `workorder_audit.py` R27
  (`amendment-tier`) fails an `amendment:` planner whose transcript ran
  fable or that ran as `planner-xhigh`/`planner-max`, and
  `workorder-rounds.js` spawns its own amendments as `planner-medium` with
  `model: 'opus'`.
- **Reviewers, live-operator and the docs row → `opus`/`medium`
  (2026-10-06, sister-project #436).** Until then the reviewers
  (`docs-sync-reviewer`, `decompile-output-guard` and two the port dropped)
  and `live-operator` ran sonnet/high, and the docs triage row's implementer
  ran `sonnet`. Artificial Analysis's Intelligence Index against cost per
  task (read 2026-10-06) puts Opus 5.5 ahead of Sonnet 5.5 at every effort
  level for about the same price: Opus medium about 51 at $1.40 a task,
  Sonnet high about 47 at $1.10, Sonnet xhigh about 52 at $2.75 against Opus
  high's 53.5 at $1.85. That benchmark prices mostly output and thinking;
  here 92-99% of tokens are cache reads at $0.20 on both, so the model step
  costs less still, and effort is what moves the bill, hence `medium`
  rather than the `high` they had. On the calibration set's tokens Opus 5.5
  costs the reviewers about 1.2× what Sonnet did. No reviewer finding was
  ever measured as missed on `sonnet`, so this is a capability bet chosen
  from a benchmark, not a measured fix; `--calibrate` will say whether it
  holds.
- **verifier and scribe stay `haiku`, now with an effort pinned
  (2026-10-08).** They run commands and paste text. Haiku 5.5 (released
  2026-10-07) honours `effort` where Haiku 4.5 ignored it, so an unpinned
  one would run at the driver's level: `verifier` is pinned `medium`
  (Haiku 5.5's own default; a false verdict costs a round, and at about
  $0.02 a run the step down to `low` saves nothing worth that), `scribe`
  `low` (it pastes precomputed text). Haiku 5.5 bills a turn whose prompt
  is over 100K tokens at 5×, and both stay under it: on 87 sessions since
  2026-09-22 the verifier's p90 prompt was 64K with no turn over 100K, the
  scribe's 70K with 1%. Repriced on those tokens a verifier run costs
  about $0.02 against $0.55 on Opus 5.5. Not moved to Haiku: the
  reviewers, live-operator, planner and implementer. Artificial
  Analysis (read 2026-10-08) puts Haiku 5.5's best Coding Agent Index (41,
  at `xhigh`; `max` scores 36 at four times the cost) below Sonnet 5.5 at
  `low` (42), and its Intelligence Index at `xhigh` (41) well below Opus
  5.5 at `medium` (51); those roles' prompts also sit mostly over 100K
  (implementer and planner 67% of turns). The `haiku` alias still
  resolved to Haiku 4.5 on 2026-10-07; it moves by itself.
- **Effort.** Opus 5.5 defaults to `medium` and thinks more per turn than
  Opus 5 at the same level; `high` is pinned for the phases that carry long
  agentic work and `xhigh` for the one narrow question `consultant` answers.
  `medium` is used where the correction is already written (amendments,
  patch rounds) or the change is docs only, and `max` only where cheaper
  reasoning has failed twice. None of these levels is measured yet:
  `tools/workorder_audit.py --calibrate` reports per role, model *and*
  effort, so the next recalibration says whether they hold.

`tools/workorder_audit.py` prices every transcript at these rates
(`MODEL_PRICES`), so a report's cost line tracks a tier change without anyone
redoing this arithmetic.

**Tier aliases, not pinned version IDs.** `opus` names the tier, not a
version. Pinning `claude-opus-5` across eight files buys reproducibility this
pipeline doesn't need (a broken model fails a *test*, not a review) and costs
a stale-ID sweep every generation — not the `*Rva*` case from `AGENTS.md`,
since an alias is a documented moving pointer, not a silently-drifted
constant.

**When to reach for `planner-xhigh` or `consultant-max` yourself,** beyond the
automatic escalation above (these were the reasons to reach for `fable`):

- The plan must establish an **unknown** game mechanism, not verify a
  suspected one — the "which of 34 candidates does X" shape, where a wrong
  mechanism model has cost whole sessions rather than one round.
- The change falls in the class `AGENTS.md` § "Don't Suspend the Game's Own
  Runtime" warns about — the failure mode inverts, and the blast radius is the
  player's session.

Not on a routine change, or the implementer or a reviewer — those run at high
volume, where more thinking per turn is real money for no measured gain —
and not as the default planner, which is 22% of spend already.
