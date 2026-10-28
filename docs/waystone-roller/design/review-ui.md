# Audit of the built page (impeccable audit)

Run: 2026-10-28, `impeccable` 4.7.1, `audit` of the page in the browser
preview (`npm run dev`, `http://127.0.0.1:1421/?view=<fixture>`), against
the direction in [`../ui-design.md`](../ui-design.md) and its critique in
[review-direction.md](review-direction.md).

**DEGRADED: single-context** (one implementer session, no sub-agent tool and
no browser automation tool). How the evidence was gathered:

- **Detector, source scan:** `impeccable detect --json apps/waystone-roller/src
  apps/waystone-roller/index.html` returned `[]` (exit 0), before and after
  the fixes below. The design hook scanned each component as it was written
  and reported nothing.
- **Detector, URL scan:** `impeccable detect --json --viewport 760x720
  http://127.0.0.1:1421/?view=too-long` could not start its browser
  ("Failed to launch the browser process!"), so there is no in-page scan.
- **Looking at the render:** headless Microsoft Edge screenshots of the
  preview, read one by one: `ok-any`, `too-long` and `unmatchable` at
  760 × 720 and 800 × 860, `hotkey-taken` at the minimum 560 × 480. They
  stay on the implementer's machine (no image is committed).

## Findings and what changed

| # | Severity | Finding | Change |
|---|---|---|---|
| 1 | P1 | The "Wanted mods" choice stretched to the width of its column and its caption wrapped, so the filter row broke into two lines at 760 px and the mod list showed only two or three rows. | The filter groups no longer stretch their controls; the caption is short ("any one is enough" / "all must be there") and sits under the choice. The row now fits one line from about 740 px. |
| 2 | P1 | At the minimum window (560 × 480) the mod list collapsed to nothing: header, filters and footer took the whole height. | The list never shrinks below 200 px; a shorter window scrolls the page, with the footer pinned to the bottom (`position: sticky`). The direction's default size became about 800 × 860. |
| 3 | P2 | A row's neutral choice "—" was filled with the same bright fill as a chosen rarity or corruption, so every untouched row shouted as loudly as a wanted one. | The chosen "—" is now a quiet outlined segment; Want (filled green) and Avoid (filled red) are the only loud states, and each keeps its word and its `+` / `−` mark. |
| 4 | P2 | "Cannot be matched" and the status's `!` ran into the following word: the space inside the hidden `!` span was trimmed. | The mark keeps its gap by a margin, not a space character. |
| 5 | P3 | The avoided part of an over-long search is cut with an ellipsis in the parts list. | Its full text is in the element's tooltip; the length column (the number the player acts on) is never cut. |

## Checked and left as is

- **Accessibility:** native radios (one named radio group per mod, arrow
  keys move within it), selects, toggle buttons with `aria-pressed`, a
  labelled text filter, `role="status"` for the shell's message and a
  polite count for the filter. A 2 px focus ring in the edit-accent colour
  on every control, drawn on the segment around a hidden radio. Primary and
  secondary text are well above 4.5:1 on the panel colours; muted text
  (`#959084` on `#121416`) is about 5.6:1. Disabled controls (`#6A665E`) are
  below that, which WCAG exempts; their state is also said in words (the
  reason beside Copy, the struck-through Want).
- **Never colour alone:** wanted `+`, avoided `−`, chosen segments filled
  and bold, the meter's "over by N", the "longest" word in the parts list,
  `!` before an error, a drawn check or empty box on the rarity toggles.
- **A wanted mod that cannot be wanted** (the `unmatchable` fixture) shows
  Want checked, disabled and struck through: that is the true state (it is
  in the list, and the search cannot use it), and Avoid and "—" stay
  available to change it.
- **Theming:** every colour, size and spacing comes from the copied tokens;
  the two literal values are the footer's upward shadow and the monospace
  stack (no font is added).
- **Motion:** only 120 ms colour transitions on hover and choice, removed
  under `prefers-reduced-motion`; no state depends on them.
- **Performance:** one list of at most a few hundred rows, rendered from
  the view; no animation of layout, no images.
- **Browser surfaces:** text selection, caret, scrollbar and focus ring take
  the theme's colours; numerals in the tier selects, meter, counts and part
  lengths are tabular.
