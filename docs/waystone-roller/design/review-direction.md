# Review of the UI direction (impeccable critique)

Run: 2026-10-28, `impeccable` 4.7.1, `critique` against
[`../ui-design.md`](../ui-design.md) before any component existed.

**DEGRADED: single-context** (the critique ran inside one implementer
session that has no sub-agent tool and no way to ask the owner; the design
review and the detector run were done one after the other in that session).
`impeccable context` found no `PRODUCT.md` or `DESIGN.md`; the direction
document and the plan's UI direction were the brief. The detector
(`impeccable detect --json docs/waystone-roller/ui-design.md`) returned `[]`:
there was no markup to scan yet, so the deterministic scan says nothing about
this run. The built page is scanned in [review-ui.md](review-ui.md).

Mode: **Operate** (a tool window: the player completes a task — make one
string — and leaves).

## Design health (Nielsen, 0–4)

| # | Heuristic | Score | Key issue |
|---|---|---|---|
| 1 | Visibility of system status | 3 | The meter and the always-visible string are right; the status line can go stale after the selection changes (the shell's call) |
| 2 | Match with the real world | 3 | "Prefixes / Suffixes / Desecrated", tiers and rarities are the game's words; "—" for "neither" is terse |
| 3 | User control and freedom | 2 | No way to clear every choice at once; with 60+ mods that is many clicks |
| 4 | Consistency and standards | 3 | Native controls, the quest tracker's theme |
| 5 | Error prevention | 3 | The tier selects cannot form an empty range; Want is disabled where it cannot work |
| 6 | Recognition rather than recall | 2 | Nothing shows which mods are chosen without scrolling the whole list |
| 7 | Flexibility and efficiency | 3 | Text filter, the shortcut; no "chosen only" view |
| 8 | Aesthetic and minimalist design | 3 | One column, one sticky footer; the footer holds many items and must stay compact |
| 9 | Error recovery | 3 | Over the limit, each part's length is shown; which part to cut first is left to arithmetic |
| 10 | Help and documentation | 2 | The shortcut field gives no format example |
| **Total** | | **27/40** | Good, with gaps in control and recall |

## Design specificity

The direction is grounded in the product: the three-way row control, the
token shown on each row, the 250-character meter and the part breakdown are
specific to building a PoE 2 search, and the copied theme ties it to the
quest tracker. The layout itself (header, filters, list, footer) is a
standard tool window, which is right for an Operate surface.

## What works

- The string and the meter are always in view; the player never scrolls to
  see what will be copied.
- "Never colour alone": `+` / `−` marks, filled segments, "over by N" and the
  `!` on errors carry every state in text or shape.
- Refusing instead of truncating, with the cost of each part shown.

## Priority issues and what changed

1. **[P1] Chosen mods are hard to review** (heuristic 6). With a long list,
   checking what is wanted and avoided means scrolling everything.
   *Changed:* the list gets a "Chosen only" toggle beside the filter, and
   each group heading counts its wanted and avoided mods.
2. **[P1] No way to start over** (heuristic 3). *Changed:* a "Clear choices"
   button in the list's toolbar sends `update_selection` with both lists
   empty (tier, rarity and corruption stay); it is disabled while nothing is
   chosen.
3. **[P2] Over the limit, the player must work out which part to cut**
   (heuristic 9). *Changed:* the parts list marks its longest part
   "longest", in text, not only by weight.
4. **[P2] The shortcut field has no format** (heuristic 10). *Changed:* the
   field carries the placeholder and caption "e.g. Ctrl+Alt+W".
5. **[P3] "Any / All" alone is ambiguous** (heuristic 2). *Changed:* the
   group is labelled "Wanted mods" with the caption "highlight a waystone
   with any one / with all of them" under the choice; the words Any and All
   stay, as the plan's direction names them.

## Considered and not changed

- **Replacing "—" with a word.** The plan's direction names the control
  Want / — / Avoid; the middle choice keeps "—" on screen, with "Neither" as
  its accessible name and tooltip.
- **Sorting the parts by length.** They stay in the search's own order, so
  the list reads like the string above it; the "longest" mark covers the
  need.
- **Clearing a stale status line in the page.** The status is the shell's;
  the page shows what the view carries (Rust decides, the UI renders).

## Personas

- **Power user (Alex):** wants to set five mods fast — the text filter and
  the radio groups (arrow keys move within a row) serve; "Chosen only"
  covers the review.
- **First-timer (Jordan):** does not know what a token is — the token is
  secondary, small, and the row's label leads; the hint line tells how to
  use the string.
- **Accessibility (Sam):** native radios and selects, a named radio group
  per mod, `role="status"` for the shell's message, a visible focus ring.
