# Crafting Helper — steel/masonry review

The Exile theme (the Quest Tracker's steel/masonry kit) on the Crafting
Helper's three windows, and the Claude theme kept as today's look. How the
theme is built: [`../ui-design.md`](../ui-design.md) § "Theme". The art the
kit does not have: [`ASSET-REQUESTS.md`](ASSET-REQUESTS.md).

## Corner safety

`npm --prefix apps/crafting-helper run corner-safety`
(`apps/crafting-helper/scripts/corner-safety.mjs`, adapted from the Quest
Tracker's runner; its header comment is the specification), on the final
tree:

```
corner-safety: 120 cases, 0 failures, control detected, flush checked on 110 cases, readability checked on 120 cases
```

The matrix: the main window's eight states at 960 x 680 and 600 x 480, the
Settings window's six states at 440 x 720 and the overlay over a bright and
a dark scene at 480 x 168, each at device scales 0.75, 1, 1.33 and 2 and at
text 200 % at scale 1, at the top, middle and end of every scroller. The
overlay carries no frame, so its 10 cases are not flush-checked. The
`--self-check` (synthetic pages, no app) reports
`corner-safety: 4 cases, 0 failures, control detected, flush checked on 4
cases, readability checked on 4 cases`: at every scale the knot probe is
seen and the centre box is not, a 12 px-inset frame is reported not flush
and a flush one passes, and a 9 px run, a low-contrast run and a
small-caps run are reported while a compliant run passes.

Two adaptations of the Quest Tracker's check, both for structures the
Quest Tracker does not have, neither relaxing a threshold:

- **A scroller inside a scroller** (the Settings window's currency list
  inside its scrolling sections) is not reported as cut off when it fits
  the outer scroller's window, since some position of the outer one shows
  it whole; its contents are then held to its own box. A nested scroller
  taller than the outer window is still reported.
- **A one-line row cut with an ellipsis** inside a scroller (the overlay's
  rows, once text 200 % makes the panel scroll) is a design choice, as the
  Quest Tracker's check already treats such a cut when nothing scrolls it.
  The row's height is still held to the scroller.

Layout choices the runner drove, all under Exile only:

- The main window's title bar wraps its buttons below the title when they
  do not fit (Settings beside the title below 760 px, as under Claude).
- When text is so large that the panes would keep under 160 px (text 200 %
  at 600 x 480), the main window's content scrolls as a whole inside the
  frame, clear of the corner knots.
- The overlay's panel scrolls, with no bar, when its text outgrows the
  168 px window.
- The grabbed text wraps and grows with its pane, rather than scrolling in
  a block of its own inside a scrolling pane.
- A chosen target's floor and Remove wrap below its text when they do not
  fit beside it.

## The previous theme

`npm --prefix apps/crafting-helper run corner-safety -- --legacy-base
waystone-crafting-steel-masonry-base` renders each case in the base tag's
tree and here with `theme=claude`, at device scale 1, and compares decoded
pixels (the Theme row removed in both):

```
legacy-pixels: 18 cases, 0 differ
```

Its control, the first case rendered under Exile, differed by 652353
pixels, so the comparison sees a change when there is one.

## Screenshots

Written by `--screenshots docs/crafting-helper/design/frames`, at device
scale 1, under `frames/`:

- `exile-main-planning.png` — planning (`craft-planning`), 960 x 680
- `exile-main-guiding.png` — guiding (`craft-guiding`), 960 x 680
- `exile-main-min.png` — the trade panel (`trade-picking`), 600 x 480
- `exile-settings-idle.png` — Settings, 440 x 720
- `exile-overlay-bright.png`, `exile-overlay-dark.png` — the overlay over both scenes, 480 x 168
- `claude-main-planning.png`, `claude-settings-idle.png`, `claude-overlay-bright.png` — the same windows under Claude

## Requested art

Five elements have no counterpart in the kit; each draws a marked CSS
placeholder in the kit palette until its file arrives, and
[`ASSET-REQUESTS.md`](ASSET-REQUESTS.md) holds the full request:

| Id | Files | Element | Placeholder |
|---|---|---|---|
| `ch-radio` | `radio-off.svg`, `radio-on.svg`, `radio-focus.svg`, `radio-disabled.svg` | the trade panel's Match radios | a 16 px steel ring, an ivory dot when on |
| `ch-overlay-frame` | `frame-overlay-compact.png` | the overlay panel's edge | a 3 px double steel border |
| `ch-item-card` | `item-card-frame.png` | the grabbed item's card | a slate card with a fine steel line |
| `ch-step-marker` | `step-marker.svg` | the route's step number plate | a slate square with a steel edge |
| `ch-update-dot` | `update-dot.svg` | the header's update mark | an 8 px ivory square, steel edge |

Today's elements, from the base tag, are under `reference/` (one
`<id>.png` each), written by `--references docs/crafting-helper/design/reference
--root ref:waystone-crafting-steel-masonry-base`.

## Design pass

Both skills judged the Exile screenshots above: `impeccable` (audit,
detector and polish) and `taste-skill:design-taste-frontend` (its
app-screen review, listed below as taste-skill). Findings and what was
done:

- **impeccable (audit):** the guide's End craft bar sat on a flat
  `#090D10` slab under the masonry, a visible seam the start bar does not
  have. — applied: the bar is transparent over the well under Exile
  (`.end-bar` in `steel.css`).
- **impeccable (audit):** the browser surfaces (selection, caret,
  scrollbars, focus ring) carried defaults. — applied: all four come from
  the kit palette (selection `#424D54`, ivory caret and 2 px ring drawn
  inside the control, the kit's 8 px slate scrollbar).
- **impeccable (audit):** contrast, small type and faux small caps. —
  applied: held by the runner's readability check on every case (12 px
  floor, Cinzel at 13 px and up, 4.5:1); caption-sized labels moved from
  the caps face to bold Alegreya Sans.
- **impeccable (audit):** text scaling broke the title bar (the buttons
  covered the title at text 200 %). — applied: the title bar wraps
  (§ "Corner safety").
- **impeccable (detector):** `impeccable detect` over `steel.css` and the
  changed components reported no findings.
- **impeccable (polish):** the overlay's masonry is barely visible in a
  156 px panel. — declined: the kit sets the masonry at 38 % behind live
  content; raising it there would trade the overlay's legibility over a
  bright scene for texture.
- **taste-skill (app screen):** the section headings are tracked caps
  (Cinzel at 0.06em), and app screens allow at most one tracked capital
  label. — declined: Cinzel section headings are the kit's committed
  treatment (the Quest Tracker sets them at 0.08em), and the tracking stays
  within the readability rule.
- **taste-skill (app screen):** three equal-weight outlined buttons in the
  header with no single primary. — declined: which actions the header
  offers and their order are not the theme's to change; Start crafting
  stays the one primary, in steel grey.
- **taste-skill (app screen):** boxed panels (the chosen targets, the
  current step, blocked for this craft, the item card) with 1px edges. —
  declined: allowed on app screens, and each box groups one control set;
  their edges take the kit's line colour, one step from the well.
- **taste-skill (app screen):** "Step 2" reads as index numbering. —
  declined: it is data (the guide's step number from Rust), and wording is
  out of scope.
- **taste-skill (app screen):** secondary and muted text share one grey
  (`#A9AFB3`), so the screen carries two text inks, not three. — applied
  as found: the kit's tokens already do this; nothing to change.
