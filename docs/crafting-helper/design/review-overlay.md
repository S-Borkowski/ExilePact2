# Review of the overlay's direction (impeccable critique)

Run: 2026-10-29, `impeccable` 4.7.1, `critique` against
[`../ui-design.md`](../ui-design.md) § "Overlay" (and the parts of "Mode
guiding", "The screen watch's line", "Settings window", "States", "Browser
preview" and "Accessibility" it changed) before any component existed,
with the `taste-skill` 2.1.5 app-screen rules (`references/app-screens.md`:
one question per screen, the most frequent job one click away, forms and
settings, colour on dark screens, empty and error states, the house rules
for app screens) applied alongside.

**DEGRADED: single-context** (the critique ran inside one implementer
session that has no sub-agent tool and no way to ask the owner; the design
review and the detector run were done one after the other in that
session, the review first). `impeccable context` found no `PRODUCT.md` or
`DESIGN.md` and no visual implementation of the new overlay; the direction
document, the workorder's overlay, watch and settings contract, M3's
overlay page and the theme's tokens were the brief. The detector
(`impeccable detect --json docs/crafting-helper/ui-design.md`) returned
`[]` with exit 0: there is no markup for the new overlay yet, so the
deterministic scan says nothing about this run. No browser was opened
(there is no page yet), and the run's snapshot was not persisted under
`.impeccable/critique/`, which is outside this change's files. The owner's
review of the built overlay comes later, as fixes on this branch (the
workorder's decision of 2026-10-29). Questions skipped: the critique lists
six priority issues, but the owner cannot be asked in this session and has
already chosen to review the built overlay instead; each issue's change is
below.

Mode: **Operate** (a heads-up panel over the game while a craft is
guided, read at a glance between clicks in the game).

The screen's one question, in the app-screen rules' terms: *"what do I do
now, and did the last try work?"*, answered by the instruction row and the
Last row. Its one finding is the last outcome, or what the screen watch
needs from the player. The most frequent job, taking the step and having
it counted, is zero clicks away when the screen watch reads the item, and
one key (the grab hotkey) when it cannot; the overlay has no control of its
own, since it lets every click through.

## Design health (Nielsen, 0–4)

Scored on the direction's first outline: the workorder's content list for
the overlay (the route and step, the instruction, the uses and odds, both
branch lines, the try count, the last outcome and the watch line) poured
into M3's one-line, 420 × 64 panel, with the watch line and History as M5
left them. The critique and the writing ran in the same session, so
`ui-design.md` as committed already holds every change below.

| # | Heuristic | Score | Key issue |
|---|---|---|---|
| 1 | Visibility of system status | 2 | A screen read and a grab gave the same Last row and History entry, and the watch line was a muted line at the panel's foot in every state, the ones that need the player included |
| 2 | Match with the real world | 3 | The game's words (hit, miss, tier, omen, currency names) and the player's own action ("hover the item"); "OCR" appears only in the one state that is about it |
| 3 | User control and freedom | 3 | The screen watch is a switch; a grab always wins over a read in flight; an unexpected change is never kept from the screen |
| 4 | Consistency and standards | 3 | The main window's status word, outcome label, branch lines and tokens; the overlay's watch line and the card's are one string |
| 5 | Error prevention | 2 | The overlay can sit over the very tooltip it reads, and nothing on the panel said why reading stopped |
| 6 | Recognition rather than recall | 3 | The step, both branches and the last outcome stay in view over the game; a stuck guide's reason was only in the main window |
| 7 | Flexibility and efficiency | 3 | Six positions and the grab hotkey as the fallback; no drag or scale (out of scope) |
| 8 | Aesthetic and minimalist design | 2 | Nine pieces of content in a one-line panel: cut to its first words, or grown without a limit over the game; the Settings section pushed the window past its 720 px |
| 9 | Error recovery | 3 | Every refusal keeps the guide as it was and points to the grab |
| 10 | Help and documentation | 3 | Each watch state says the next step; the Settings help line says what reading the screen does and keeps |
| **Total** | | **27/40** | Acceptable; gaps in telling a screen read from a grab, in what the panel covers, and in its height |

## Design specificity

The direction is built from the craft loop, not a generic heads-up
display: the rows are the guide's own (the step, its uses and odds, where
a hit and a miss go), the source of each outcome is named because the
whole milestone is about trusting a read, and every state of the screen
watch ends in the player's next action in the game's terms. The panel is
the quest tracker's overlay ground and square corners, so the family of
apps reads as one over the game. The app-screen rules' tells (KPI tiles,
icons in tinted squares, glass, gradients, a tinted accent on chrome) are
absent; colour is the warning hue on the one row that needs the player.

## What works

- **One string, two places.** The watch line in the guiding card and on
  the overlay is the same Rust string, so the overlay and the app never
  disagree about what the watch is doing.
- **Refuse, then point to the grab.** No read the app cannot trust changes
  the guide; every such state says to grab the item, which keeps the M5
  loop as the fallback in every case.
- **Rust places it.** The six positions and the inset are a pure, tested
  function in physical pixels, and the shell only moves the window, so a
  display scale or a resized client cannot drift the panel off the game.

## Priority issues and what changed

1. **[P1] A screen read and a grab looked the same** (heuristics 1 and 6).
   The milestone asks the player to trust an outcome the app read from the
   screen; with no source shown, a wrong read cannot be noticed and
   checked.
   *Changed:* the overlay's Last row, the guiding "Last:" line and every
   History entry name the source in words, "from the screen" or "from a
   grab".
2. **[P1] The watch states that need the player were easy to miss**
   (heuristics 1 and 9). Covered, unreadable, an unexpected change, black
   copies and no English OCR were a muted line at the panel's foot.
   *Changed:* Rust marks those lines to take `--color-warning`, and each
   ends in the action in words ("hold Alt over it, or grab it", "move it
   in Settings, or grab the item", "grab it to check"); the reporting
   states stay secondary text.
3. **[P2] The overlay can cover the tooltip it reads** (heuristic 5). The
   reader treats the overlay's whole window as covering, and nothing said
   why reading stopped.
   *Changed:* the covered state has its own line naming both fixes; the
   window is no taller than seven rows need (480 × 168), and the panel is
   anchored to the game's edge. Where the game draws a crafted item's
   tooltip is not established; Live 1 shows whether `top-center` covers
   it.
4. **[P2] The content does not fit M3's panel, and nothing bounds it**
   (heuristic 8). M3's one line, 420 × 64, cuts everything after the
   instruction's first words; letting it wrap grows it over the game
   without a limit.
   *Changed:* a 480 × 168 window with at most seven rows in three groups,
   each row one line with an ellipsis at its end; the instruction is the
   one larger row; no fallback line (it is in the main window); the odds
   lead with the percent, so an ellipsis keeps the number.
5. **[P2] The Settings window outgrows its fixed 720 px** (heuristic 8).
   *Changed:* the Overlay section is two native controls with one help
   line each, placed after Grab; the page scrolls as a whole, so Updates,
   the least-used section, is the part below the fold. The audit of the
   built window measures it.
6. **[P3] A click-through panel cannot be acted on** (heuristic 6). The
   content list says only that stuck, replan and done "say so"; a status
   word over the active rows leaves the player looking for a control that
   is not there.
   *Changed:* stuck shows "Why:" and its first reason in place of the uses;
   replan says "End the craft in the app to plan again from this item."
   and drops the branch and Screen rows; done says "The craft is done."
   with the last outcome. Each points to the app, where the action is.

## Considered and not changed

- **A see-through or blurred panel, so more of the game shows.** The
  app-screen rules ban glass and blur, and legibility over a bright scene
  is the overlay's first job; the tracker's 0.92 ground keeps muted text at
  4.7:1 over white (computed from the tokens).
- **A flash or slide when an outcome lands.** Motion over the game draws
  the eye away from the next click and repeats on every read; the row
  changes in place, and the main window's status line announces it.
- **Hiding the overlay under the game's panels**, as the quest tracker
  does. A craft is taken with the stash and the inventory open, so it would
  hide throughout.
- **A drag mode or a scale setting.** Out of this milestone's scope; the
  six positions cover the edges the game leaves free.
- **The fallback line on the overlay.** It is about the route's plan, read
  once in the main window; the overlay is for the step being taken.
- **Making the overlay a live region.** It cannot take focus and repeats
  the main window, whose status line already announces each outcome.

## Personas

- **Power user (Alex):** wants to roll without leaving the game — with the
  screen watch on, the step is counted by hovering the item; the hotkey
  grab still works from any state, and the branch rows say what comes next
  before the click.
- **First-timer (Jordan):** does not know what the app can see — the
  Settings help line says what is copied and that nothing is kept or sent,
  and every watch line says the next action in plain words.
- **Accessibility (Sam):** the overlay is a mirror, never the only place a
  fact appears; outcomes, sources and watch states are words, the warning
  colour is never the only signal, and the Settings controls are native
  and labelled.
