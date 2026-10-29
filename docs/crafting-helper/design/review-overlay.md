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

## Audit of the built overlay

Run: 2026-10-29, `impeccable` 4.7.1, `audit` of the built overlay page
(`src/overlay/`), the Settings window's Overlay section
(`src/settings/OverlaySection.svelte`) and the guiding view's watch line and
outcome sources (`src/craft/CurrentStep.svelte`, `History.svelte`) in the
browser preview, against [`../ui-design.md`](../ui-design.md) and the
critique above, with the `taste-skill` app-screen rules applied alongside as
in the critique.

**DEGRADED: single-context** (one implementer session, no sub-agent tool and
no way to ask the owner; the owner reviews the built overlay later, as fixes
on this branch, by the workorder's decision of 2026-10-29).
`impeccable context` found no `PRODUCT.md` or `DESIGN.md`; the direction
document was the brief. How the evidence was gathered:

- **The states looked at:** the overlay at its window's 480 × 168 (the
  preview draws it so) for every view fixture the milestone added and the
  guide's other statuses: `overlay-active`, `overlay-not-in-front`,
  `craft-watching`, `craft-screen-hit`, `craft-screen-unexpected`,
  `craft-watch-off`, `craft-watch-unavailable`, `craft-replan` and
  `craft-done`, each over `scene=bright` or `scene=dark`, and
  `overlay-active` with its position set to a `bottom-*` place; Settings'
  `overlay-settings` (with `install-downloading` beside it for the height)
  at its fixed 440 × 720; the main window's `craft-watching`,
  `craft-screen-hit` and `craft-screen-unexpected` at 960 × 680 and
  `craft-screen-hit` at the minimum 600 × 480.
- **Looking at the render:** headless Microsoft Edge 156 over the DevTools
  protocol on `127.0.0.1`, the viewport set exactly
  (`Emulation.setDeviceMetricsOverride`), one screenshot per state, read
  before the fix and again after; the panel's height, which rows are cut
  and the Settings page's height were read from the same pages with
  `Runtime.evaluate`. The pictures stay on the implementer's machine; no
  image is committed.
- **Detector, source scan:** `impeccable detect --json` over `src/overlay`,
  `src/settings/OverlaySection.svelte`, `src/craft/CurrentStep.svelte` and
  `src/craft/History.svelte` returned `[]` (exit 0); the design hook
  reported nothing on each edit it scanned.
- **Detector, page scan:** `impeccable detect --cdp <port> --json` on
  `settings.html?state=overlay-settings` returned `[]`; on
  `overlay.html?state=overlay-active` it returned five warnings, before and
  after the fix (finding 1 and "Checked and left as is"). Each page was a
  background tab, so the pixel contrast pass read nothing; the contrast
  figures are the critique's, computed from the tokens.
- **taste-skill:** its `lint.mjs` and `inspect.mjs` were not run (they open
  the page at 1440, 768 and 390 wide, sizes neither window has); its
  app-screen questions were answered against the screenshots.

### Audit health

| # | Dimension | Score | Key finding |
|---|---|---|---|
| 1 | Accessibility | 3 | Settings' two controls are native and labelled, each described by its help line; the overlay is a mirror of the main window, with outcome, source and watch state in words, the warning colour never the only signal |
| 2 | Performance | 4 | Seven text rows from the view, no motion, no images, no layout read in a loop |
| 3 | Theming | 4 | Every colour and size from the copied tokens; the only literal colours are the preview's stand-in scenes |
| 4 | Responsive | 3 | Both windows have fixed sizes; every overlay row is one line with an ellipsis, so the panel stays 156 px of 168 in every state |
| 5 | Implementation integrity | 3 → 4 | The pages render what Rust sends; the direction promised a refusal line the view cannot carry (finding 3), and the text ran 8 px from the panel's edge (finding 1) |
| **Total** | | **17/20 → 18/20** | **Excellent** |

The overlay answers its question, *"what do I do now, and did the last try
work?"*, at a glance: the instruction is the one 16 px bold row, and the
Last row names the outcome and where it came from. Over the white stand-in
the 0.92 ground holds every row legible; over the near-black one the 1 px
edge keeps the panel apart from the scene. The watch line in the guiding
card and on the overlay is one string, and the warning ones take the warning
colour in both.

### Findings and what changed

1. **[P2] Text started 8 px from the panel's edge** (Implementation
   integrity; the detector's `body-text-viewport-edge`, five rows). Over a
   busy game scene the first letters sat almost on the border.
   *Changed:* the panel's sides take `--space-row-pad` (12 px); top and
   bottom keep `--space-panel-pad` (8 px), so the panel's height (156 px)
   did not change. `ui-design.md` § "Overlay" says so.
2. **[P2] The screen-read source and the watch line had to be checked in
   both windows.** The guiding card's "Last:" line, each History entry and
   the overlay's Last row were read for `craft-screen-hit` ("from the
   screen") and `craft-guiding` ("from a grab"), and the watch line for
   every watch fixture, in the colour Rust marks. Nothing to change; the
   page tests pin each (`history_marks_outcomes_read_from_the_screen`,
   `guiding_shows_the_watch_line`, `overlay_renders_the_watch_line`).
3. **[P2] The direction promised a refusal line the view cannot carry.**
   `ui-design.md` asked the Overlay section to show a refused position with
   `!` in a status line, as the Grab section does; but `SettingsView` has a
   status only for the hotkey, and the select offers only the six names the
   app takes, so the page cannot send a refused one.
   *Changed:* the direction now says so: a refusal from the shell (which
   the page cannot cause) keeps the saved value, the next view shows it and
   the bridge logs Rust's reason, as for the blocked list. No status line
   was invented.

### Checked and left as is

- **The odds are cut in the Uses row** in every active state at 480 px
  (`~3.0% ((4000 of 133355 weight) times (the l…`). The percent leads, as
  the direction planned, so the cut keeps the number; the whole basis is in
  the main window.
- **Replan's Why and Last rows are cut** (the reason is long). Replan
  points to the app, where the full reasons are listed under "Why:".
- **The detector's five `body-text-viewport-edge` warnings remain** at
  13 px: the scan's tab was wider than 480, where the rule asks 16 px; at
  the overlay's own 480 px width it asks 12, which the panel now meets. The
  rows are cut by design, not overflowing.
- **"Last:" and "Why:" are plain muted words while "Uses" and "Screen" are
  small caps**, as the direction sets them: the first two read as the start
  of a sentence, the other two as a row's label.
- **The Settings page is 904 px tall** in `overlay-settings` (950 in
  `install-downloading`) in its 720 px window, so Updates is below the fold
  and the page scrolls as a whole: the direction's choice, Updates being
  the least-used section.
- **No focus, no live region on the overlay**: it cannot be focused or
  clicked, and the main window's status line announces each outcome.
