# Agent Rule Stories

`AGENTS.md` states each rule as an imperative plus one sentence of why. The
incident that made the rule worth writing — the dated research session, the
review finding, the "learned the expensive way" narrative — lives here
instead, one file per `AGENTS.md` section that has one. Each file opens with
a link back to the section it backs.

Every story here comes from the sister project
[hero-siege-offline-toolkit](https://github.com/falorfrozen-cmd/hero-siege-offline-toolkit)
(Hero Siege, a GameMaker game), where this tooling was built. The engine
names in them (ForgePact, YYToolkit, GML, `hs-game-sdk`, the hub) are that
project's, not this repository's; the rules they back are engine-independent.

| `AGENTS.md` section | Story and evidence |
|---|---|
| [`Some of These Rules Are Enforced, Not Just Written`](../../AGENTS.md#some-of-these-rules-are-enforced-not-just-written) | [rules-enforced.md](rules-enforced.md); [workorder-calibration.md](workorder-calibration.md) — the recalibration of `/workorder`'s audit budgets and model tiers on real runs |
| [`Spend Each Check Once, and Overlap What Does Not Wait`](../../AGENTS.md#spend-each-check-once-and-overlap-what-does-not-wait) | [workorder-calibration.md](workorder-calibration.md) |
| [`Mod Development Workflow: Test Before / After, Then Build to It`](../../AGENTS.md#mod-development-workflow-test-before--after-then-build-to-it) | [static-model-workflow.md](static-model-workflow.md) — the pilot that modelled a drop roll from a written spec, and the live rounds it would have saved |
| [`Limit Rebuilds & Reruns During Development`](../../AGENTS.md#limit-rebuilds--reruns-during-development) | [limit-rebuilds-reruns.md](limit-rebuilds-reruns.md) |
| [`Prove the Instrument Before Trusting a Negative Result`](../../AGENTS.md#prove-the-instrument-before-trusting-a-negative-result) | [prove-the-instrument.md](prove-the-instrument.md) |
| [`Check a Permission Where It Is Used, Not Where It Is Convenient`](../../AGENTS.md#check-a-permission-where-it-is-used-not-where-it-is-convenient) | [check-a-permission.md](check-a-permission.md) |
| [`Never Call an Address You Resolved by Hand`](../../AGENTS.md#never-call-an-address-you-resolved-by-hand) | [never-call-resolved-address.md](never-call-resolved-address.md) |
| [`Don't Suspend the Game's Own Runtime`](../../AGENTS.md#dont-suspend-the-games-own-runtime) | [dont-suspend-runtime.md](dont-suspend-runtime.md) |

Sections not listed here (`Shared Agent Tooling…`, `Offer /workorder…`,
`Legal`, `Check for a Named Ghidra Project…`, `Documentation & Instructions
Maintenance`, `Fold What a Mod Learned…`, `One Branch and One Pull Request
per Feature` and `Every Pull Request Gets ai-review…`) carry no incident
narrative worth splitting out — the rule and its one-sentence why are the
whole story.
