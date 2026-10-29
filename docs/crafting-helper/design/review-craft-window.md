# Audit of the built craft window and Settings (impeccable audit)

Run: 2026-10-29, `impeccable` 4.7.1, `audit` of the main window's craft
states and the Settings window's blocked currencies in the browser preview
(`npm run dev`, `http://127.0.0.1:1422/?state=<fixture>` and
`http://127.0.0.1:1422/settings.html?state=blocked-currencies`), against the
direction in [`../ui-design.md`](../ui-design.md) and its critique in
[review-direction.md](review-direction.md), with the `taste-skill` 2.1.5
app-screen rules (`references/app-screens.md`: § 2.7 "The check", § 4.2 "One
left edge, one right edge", § 8 "Forms and settings", § 11 "Empty, zero,
loading and error states", § 13 "House rules") applied alongside.

**DEGRADED: single-context** (one implementer session, no sub-agent tool and
no way to ask the owner; the owner's own review of the window comes later, as
fixes on this branch, by the workorder's decision of 2026-10-29).
`impeccable context` found no `PRODUCT.md` or `DESIGN.md`; the direction
document and M3's built pages were the brief. How the evidence was gathered:

- **The states looked at:** `craft-planning`, `craft-fallback`,
  `craft-no-route`, `craft-refused`, `craft-guiding`, `craft-miss`,
  `craft-replan` and `craft-done`, each at the main window's default
  960 × 680 and its minimum 600 × 480, and Settings' `blocked-currencies`
  at its fixed 440 × 720 (with `idle` and `install-downloading` beside it
  for the window's height).
- **Looking at the render:** headless Microsoft Edge 156, driven over the
  DevTools protocol on `127.0.0.1` with the viewport set exactly
  (`Emulation.setDeviceMetricsOverride`; plain `--headless --window-size`
  gave a 492 × 628 viewport for a 440 × 720 request, so its pictures were
  discarded), one screenshot per state and size, read one by one before the
  fixes and again after. The layout figures below (scroll heights, where the
  route starts) were read from the same pages with `Runtime.evaluate`. The
  pictures stay on the implementer's machine; no image is committed.
- **Detector, source scan:** `impeccable detect --json` over `src/App.svelte`,
  `src/app.css`, `src/components`, `src/craft` and `src/settings` returned
  `[]` (exit 0), before and after the fixes; the design hook, on the edit
  it scanned, reported nothing.
- **Detector, page scan:** `impeccable detect --cdp <port> --json` on each
  of the eight `craft-*` states at 960 × 680 and on Settings, after the
  fixes. Settings, `craft-refused`, `craft-guiding`, `craft-miss` and
  `craft-done` returned `[]`; the others returned only warnings, listed under
  "Checked and left as is". The detector noted that each page was a
  background tab, so its pixel contrast pass read nothing: the contrast
  figures below are computed from the tokens instead.
- **taste-skill:** its `lint.mjs` and `inspect.mjs` were not run (they open
  the screen at 1440, 768 and 390 wide, sizes this window never has); its
  § 2.7 questions were answered against the screenshots instead.

## Audit health

| # | Dimension | Score | Key finding |
|---|---|---|---|
| 1 | Accessibility | 3 | Native controls, labelled groups and focus moves hold; two floor selects per chosen target share one name (left as is, below) |
| 2 | Performance | 4 | Lists of a few hundred rows from the view, no layout animation, no images |
| 3 | Theming | 4 | Every colour and size from the copied tokens |
| 4 | Responsive | 2 → 3 | At the minimum size the route sat about 1,900 px down, below the whole choice list (finding 1) |
| 5 | Implementation integrity | 3 | The page renders what Rust sends; two places fought the fixed window sizes (findings 2 and 4) |
| **Total** | | **16/20 → 17/20** | **Good** |

The screen's question in planning, *"what will get this item the modifiers
I want?"*, is answered by the route, the right column's main block at the
default size. In guiding, *"what do I do next, and did the last try
work?"*, is answered by the step card and the "Last:" line above it, the
first things in the left column.

## Findings and what changed

| # | Severity | Finding | Change |
|---|---|---|---|
| 1 | P1 | At the minimum window (600 × 480, one column) `craft-planning`'s route began 1,896 px into a 299 px scroll area, under all 53 of the ring's modifier choices: the answer to the screen's question was six screens away from the targets that change it. | In one column the three choice groups scroll together in a sunken box past 240 px, as the currency list does. The route now begins at 814 px (scroll height 2,378 → 1,296 px). The two-column layout is unchanged; there the route has its own column. |
| 2 | P2 | `craft-fallback` opens Blocked for this craft because the blocked list is the likely fix, but the list opened at "Arcanist's Etcher" and the one blocked entry, Omen of Whittling, was scrolled out of its 240 px: the open disclosure showed ten unchecked names and not the cause. | Each time the list comes into view it scrolls itself to its first checked entry when that entry is out of sight; once per showing, and never after the player has scrolled it, so a tick never makes it jump. The fallback state now opens on Omen of Whittling with its "this craft" tag. |
| 3 | P2 | A chosen target's row wrapped at the default size: the text and floor select on one line, **Remove** alone on the next, so each target took about 70 px and the three buttons sat under the text instead of beside the select. | Each chosen row is one grid row: the text (which wraps when long), the floor select, Remove. Three targets now take about 45 px each, and the selects and buttons line up in columns. |
| 4 | P2 | The Settings window is a fixed 440 × 720, not resizable, and its page measured 819 px with the new picker: **Check now**, the version and the footer were below the window's edge and needed a scroll in every state. | The Settings picker's list scrolls past 136 px (about six names) instead of 240 px, set by the section through `--picker-list-height`; the page measures 720 px in `idle` and `blocked-currencies`. `install-downloading` still runs 41 px over (its progress lines), and the page scrolls there. The section's stale comment that the shell sizes the window to the page now says the window is fixed. |
| 5 | P3 | The outcome word repeats the summary's first word ("Miss Miss: +#% to Fire Resistance (T5) below its floor T3"), in History and in the "Last:" line, and read as a stutter. Both halves are pinned (the summary is Rust's, the word is the design's, and the page tests read both). | The word is set as an outlined caption-size label, as the guide's status word is, so it reads as the entry's mark before the sentence. The text is unchanged. Dropping the leading word from Rust's summary is left to the owner's review. |
| 6 | P3 | Section headings (Targets, Route, History) and the group labels inside them (Prefixes, Suffixes, Chosen) had the same muted small caps, so the left column read as one run of equal labels. | A section's `h2` heading takes `--color-text-secondary`; group labels stay muted. |

`ui-design.md` records findings 1–6 where they change the direction: the
Chosen row, the one-column choice box, the picker's two list heights and its
scroll to the first checked entry, the outcome word's label, and the
heading colours in § "Theme".

## Checked and left as is

- **Accessibility:** native buttons, checkboxes, selects, search fields and
  `details` throughout; the target choices and the currency picker are
  checkbox lists in fieldsets with legends; every floor select is named
  "Floor for <modifier text>" and every Remove "Remove <modifier text>"; the
  status line is the one polite live region; the mode's first heading takes
  the focus on a mode change; End craft's question is a labelled group that
  takes the focus on Keep crafting. A 2 px focus ring in the edit-accent
  colour on every control. Contrast from the tokens, on the window, sunken
  and raised surfaces: primary text 13.6–15.6:1, secondary 8.4–9.7:1, muted
  5.3–6.1:1, the warning colour 7.1–8.1:1, the hit green 9.7–11.2:1, the
  in-progress blue 7.9–9.1:1. Disabled text (`#6A665E`, about 3.2:1) is
  below 4.5:1, which WCAG exempts; Start crafting says why it is disabled in
  words beside it.
- **Two floor selects with one name:** a chosen target's floor shows in the
  Chosen list and again under its checked choice, both named "Floor for
  <modifier text>". That is the direction (the Chosen list keeps the
  targets in view; the choice list keeps the floor beside the checkbox), and
  both send the same command with the same options, so either one is right.
- **Never colour alone:** an error is `!` and words; an outcome and a guide
  status are words; a met target is the word "target" with a drawn check;
  a missing tier is "—" named "no tier".
- **Long lines (detector warning):** the route's notes in `craft-planning`
  and `craft-fallback`, and the longest rejection in `craft-no-route`, run
  past 80 characters a line in the right column at 960 px. Capping them at
  about 70 characters would add a line or two above the route's steps in a
  window whose height is the scarcer room; the column itself is the
  measure.
- **Tight leading (detector warning):** `--line-step` is 18 px on 14 px text
  (1.29), one hundredth under the detector's 1.3. It is the quest tracker's
  token, copied byte for byte and held to the original by
  `theme_tokens_match_quest_tracker`; the three apps keep one rhythm.
- **Odds text:** a basis can carry its own brackets, so a line reads
  `~4.5% ((6000 of 132355 weight) times (…))`. That is Rust's form, pinned
  by `ch-app`'s tests, and the page renders it as sent.
- **The route name twice:** with other routes, the route's name shows as a
  line and as the selected option of the select labelled "Route" beside
  it. The line names the route when the select is closed; the select is how
  another is chosen.
- **The minimum window's header:** below 760 px the Craft of Exile button
  takes its own row under the title (36 px of a 480 px window), as the
  direction says; the Start and End bars stay pinned above the status line.
- **Theming and browser surfaces:** every colour, size and spacing comes
  from the copied tokens; selection, caret, scrollbars and the focus ring
  take the theme's colours; odds, tiers and try counts use tabular figures.
- **Motion:** only 120 ms colour transitions on hover, removed under
  `prefers-reduced-motion`; no state depends on them.
- **App-screen rules:** one filled primary per mode (Start crafting, or End
  craft in done and replan); borders, no shadows; no icon chips, pills
  above headings or emoji, and no gradient but the download bar's fill, one
  flat colour drawn with `linear-gradient`; empty states keep their frame and say one
  sentence ("Pick a target to see a route.", "No step taken yet.").
