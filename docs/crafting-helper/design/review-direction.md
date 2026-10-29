# Review of the craft window's direction (impeccable critique)

Run: 2026-10-29, `impeccable` 4.7.1, `critique` against
[`../ui-design.md`](../ui-design.md) § "Craft window" (and the parts of
"Windows", "Settings window", "States" and "Accessibility" it changed)
before any component existed, with the `taste-skill` 2.1.5 app-screen rules
(`references/app-screens.md`: one question per screen, the most frequent
job one click away, forms and settings, empty states, the app house rules)
applied alongside.

**DEGRADED: single-context** (the critique ran inside one implementer
session that has no sub-agent tool and no way to ask the owner; the design
review and the detector run were done one after the other in that session).
`impeccable context` found no `PRODUCT.md` or `DESIGN.md`; the direction
document, the workorder's craft state and command contract, and M3's built
pages were the brief. The detector (`impeccable detect --json
docs/crafting-helper/ui-design.md`) returned `[]` with exit 0: there was no
markup to scan yet, so the deterministic scan says nothing about this run.
No browser was opened (there is no page yet), and the run's snapshot was
not persisted under `.impeccable/critique/`, which is outside this change's
files. The owner's review of the built window comes later, as fixes on this
branch (the workorder's decision of 2026-10-29), so the critique's closing
questions were skipped.

Mode: **Operate** (a tool window beside the game: the player plans a craft,
then works through it step by step).

The screen's one question, in the app-screen rules' terms: in planning,
*"what will get this item the modifiers I want?"*, answered by the route
preview; in guiding, *"what do I do next, and did the last try work?"*,
answered by the current step card. The most frequent job is the re-grab,
which is the hotkey or `Ctrl+V` from any mode, so it is never more than one
key away.

## Design health (Nielsen, 0–4)

Scored on the direction as first written, before the changes below.

| # | Heuristic | Score | Key issue |
|---|---|---|---|
| 1 | Visibility of system status | 2 | After a re-grab the outcome is told only in the status line at the window's foot; the player is looking at the step card |
| 2 | Match with the real world | 3 | Prefix, suffix, desecrated, tiers and the currency names are the game's; "T1 is best" is assumed knowledge |
| 3 | User control and freedom | 2 | End craft in an active guide clears the history on one click, with nothing to take it back |
| 4 | Consistency and standards | 3 | Native controls, M3's frame and the quest tracker's theme; one currency picker for Settings and the craft |
| 5 | Error prevention | 3 | Floors come only from Rust's options; Start crafting is disabled with its reason; no typed ids remain |
| 6 | Recognition rather than recall | 3 | The Chosen list keeps the targets in view; the blocked list sits under the route, so a fallback's cause is out of sight |
| 7 | Flexibility and efficiency | 3 | The filter, the hotkey and the paste from any mode; no shortcut for Start crafting beyond focus and Enter |
| 8 | Aesthetic and minimalist design | 3 | Two columns, one primary action; the header's long Craft of Exile button risks competing with it |
| 9 | Error recovery | 3 | Each rejected blueprint and each fallback says why in words; the fix (the blocked list) is a scroll away |
| 10 | Help and documentation | 3 | Empty prompts tell the next step; the take-this-step prompt spells out the loop |
| **Total** | | **28/40** | Good; gaps in status after a re-grab and in control over ending a craft |

## Design specificity

The direction is built from the craft itself rather than from a generic
tool layout: the item panel shows the game's own sides and tiers with
"target" marked on the modifiers that already meet one, the route is the
blueprint's steps with the engine's odds in its own words ("odds unknown"
never hidden behind a dash), and the guide's two branch lines tell the
player before they click in the game what a hit and a miss will lead to.
The frame (header, two columns, status line) is a standard tool window,
which suits an Operate surface used beside a game; the app-screen rules'
tells (KPI tiles, charts, icons in tinted squares, gradients) are absent.

## What works

- **Rust's words reach the player unedited.** Odds, fallbacks, branches and
  summaries are quoted forms pinned by tests, so the preview cannot drift
  from what the engine decided.
- **The action is never scrolled away.** Start crafting and End craft sit
  in fixed bars at the column's foot, and a disabled Start says why.
- **One picker for one concept.** Settings and the craft's override use the
  same by-name, filterable catalogue list, and the catalogue itself is the
  limit, so there is no number to explain.

## Priority issues and what changed

1. **[P1] The outcome of a re-grab is easy to miss** (heuristic 1). The
   player tries a step in the game, grabs the item, and looks back at the
   step card; the hit or miss is only in the status line at the foot and,
   oldest first, at the bottom of History.
   *Changed:* the Current step part shows a "Last:" line under its heading,
   the newest History entry's outcome word and summary, once a change has
   been observed. The page only shows that entry; Rust still judges it.
2. **[P2] Ending an active craft throws its history away on one click**
   (heuristic 3). *Changed:* in active and stuck, End craft is a secondary
   button that asks in place, "End this craft? Its history is cleared.",
   with End craft and Keep crafting, focus on Keep crafting; in done and
   replan, where ending is the expected next step, it stays primary and acts
   at once. No modal dialog, following the app-screen rules' confirmation
   that names its object.
3. **[P2] The blocked list is out of sight when it is the fix** (heuristics
   6 and 9). With a fallback line or no route, the cause is often a blocked
   currency, and the list sat under the whole route.
   *Changed:* Blocked for this craft moves above the route as a disclosure
   whose summary always shows the count ("Blocked for this craft: 3"),
   closed by default and open when the route has a fallback or there is no
   route.
4. **[P3] "T1 is the best tier" is assumed** (heuristic 2). A player new to
   PoE 2's tier numbering may read "At least T3" backwards.
   *Changed:* Targets carries the help line "T1 is the best tier; a floor
   keeps a roll at that tier or better."
5. **[P3] The Craft of Exile button competes with Start crafting**
   (heuristic 8). Its text is long and it is in every state.
   *Changed:* it is styled as a secondary button, as Settings is; Start
   crafting is the window's one primary action (End craft in done and
   replan). The owner's text for it is kept as is.

## Considered and not changed

- **History newest first.** The workorder's contract orders it oldest
  first, which reads as the craft's story; the "Last:" line covers the need
  to see the newest at a glance.
- **An undo for Clear targets.** Re-picking from the Chosen list's filter is
  quick, and Rust keeps no target history to undo from; the button is
  disabled with none chosen, as the waystone roller's Clear choices.
- **A keyboard shortcut for Start crafting.** The hotkey and `Ctrl+V` are
  the window's shortcuts; a third global key would compete with the game's.
  Start crafting is reachable by Tab and Enter.
- **Unblocking a currency from its fallback line.** The line is Rust's
  string and the page does not know which ids it names; opening the blocked
  list beside it (issue 3) gives the same reach without the page parsing
  Rust's words.
- **Showing the overlay during a craft.** Left hidden until M6 (the owner's
  decision of 2026-10-29).

## Personas

- **Power user (Alex):** knows the targets and wants a route fast — the
  modifier filter, the Chosen list and the hotkey re-grab from any mode
  serve; the floor select defaults to the best tier, so most targets need
  one click.
- **First-timer (Jordan):** does not know what a floor or a blueprint is —
  the empty prompts say the next step, the tier help line explains the
  numbering, every unknown odds says why, and the branch lines say what
  happens before the click in the game.
- **Accessibility (Sam):** native checkboxes, selects and buttons in
  labelled groups, every floor select and Remove named for its modifier,
  outcomes and statuses as words, one polite live region, focus moved to
  the new mode's heading on a mode change and to Keep crafting when End
  craft asks.
