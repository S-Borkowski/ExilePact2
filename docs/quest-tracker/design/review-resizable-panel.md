# Review of the resizable panel

The design check of 0.1.1's resizable overlay (owner's request, 2026-10-28:
"it should be possible to resize window freely like any other windows
window can be resized (by dragging on edge)"):
`apps/quest-tracker/src/components/Panel.svelte` and its root
`apps/quest-tracker/src/App.svelte`. The scale setting now zooms the
content only; the player sizes the window, by the panel's edges and corners
in edit mode, and the panel fills the window inside its 12 px shadow margin
with its body scrolling.

It was reviewed on the browser preview (`npm --prefix apps/quest-tracker
run dev`, then `http://127.0.0.1:1420/?frame=<frame>&mode=<mode>`), with
the `panel-campaign` and `panel-next-up` fixtures in play and edit mode, at
three window sizes in logical pixels: narrow, the minimum (320 × 240);
default, a first start at zoom 1 (344 × 408); and wide (560 × 720). One
more render at `scale=2` and 688 × 816 (the default size at zoom 2) checked
the zoom, and one of the collapsed badge in edit mode checked that it has no
handles. Renders were taken with headless Edge over its DevTools protocol
and read by eye.

Tools: impeccable 4.7.1 (`impeccable context`, `impeccable detect`, then its
`audit` procedure). It ran inside the item's implementer, a subagent with
no owner to ask: its interview step was replaced by the brief
(`docs/quest-tracker/ui-design-brief.md`), the component spec and the
owner's request above. Its rendered scan did not run (`detect` on the
preview URL could not launch its own browser); the source scan did.

detector: `impeccable detect --json` over `Panel.svelte` and `App.svelte`
printed `[]`.
audit: accessibility 3 (the handles are a pointer affordance on the
window's own frame, hidden from assistive technology and out of the tab
order; the window keeps its size without them, and nothing else changed for
the keyboard or a screen reader); performance 4 (no motion, no layout read
in script; the handles are eight empty boxes, rendered in edit mode only);
theming 4 (no new token, colour or icon; the handles draw nothing);
responsive 3 (the panel follows the window in both directions, the header
and the way back to play stay on one line down to the minimum; the edit
header's name keeps its cap at any width, below); integrity 4 (the resize
is the window's own, through Tauri's `startResizeDragging`, as the header's
move is `startDragging`).

## Findings and what was done

- **F1, fixed** (audit, responsive): the first version of the fill made the
  overlay `100vh` tall, which in the preview's stand-in for the webview
  zoom (a CSS `zoom` on the page) is scaled with the zoom, so at `scale=2`
  the panel ran to twice the window's height. The overlay's height is now the window's height divided
  by the preview's scale (`--preview-scale`, unset and so 1 inside the
  app), and at `scale=2` the panel is exactly the window minus its margin.
- **F2, fixed** (audit, responsive): with the panel as tall as the window,
  the status line and the way back to play could be squeezed by a long
  list. Both are now fixed-height (`flex: none`) like the header, and only
  the body gives up height and scrolls.
- **F3, fixed** (the minimum size): the minimum was chosen from the
  renders: at 320 × 240 the edit header keeps the EDIT chip, about five
  letters of the name, the view switch and both buttons on one line, the
  way back to play stays on one line (it wraps to two below about 300 px),
  and the body keeps about 165 px for rows. Play mode's header fits there
  too, the class and level shortened first.
- **F4, fixed** (Live 1, scale away from 1): the owner saw the panel's
  frame grow a little as the scale fell and shrink as it rose, with the
  window's size unchanged. Measured in the built app (the edit accent's
  ring located on a screen capture, the layout read over WebView2's
  remote debugging) on an 818 × 808 window: the webview zoom does lay the
  overlay out over the whole window at every scale, but its 12 px margin
  is in CSS pixels, which the zoom draws larger, so the frame was
  800 × 790 at 0.75, 794 × 784 at 1, 782 × 772 at 1.5 and 770 × 760 at 2.
  The margin is now divided by the zoom the page measures (the window's
  outer size over its viewport), so it stays 12 window pixels and the
  frame is 794 × 784 at every scale; only the content zooms. The
  preview's `scale` sets the same divisor for its CSS stand-in. The
  shadow still zooms with the content: by its token (8 px blur, 2 px
  down) it needs more than the 12 px above about 1.2, so its faint outer
  edge is cut at the window's border there (read from the token, not
  measured).

## Kept, with the reason

- **N1** — the edit header's character name is capped at 104 px
  (`Header.svelte`, `.who`), so on a wide window it still truncates
  ("Corisande_F…") with spare room beside it. The cap belongs to the
  header, outside this change's files; widening it is a follow-up for the
  header, not a part of the resize.
- **N2** — a short list leaves empty panel below it on a tall window
  (Needs human judgement 5's default: the panel fills the window). A
  `max-height` on `.panel` restores shrink-to-content if the owner prefers
  it.
- **N3** — no visible resize grip. The plan keeps the look as the owner's
  design, with no new token, icon or control: the cursor over each edge
  band (8 px, half over the border) and corner square (14 px) is the
  affordance, as on any Windows window. In play mode nothing is drawn or
  hit, so the whole window stays click-through.
- **N4** — the east band covers the outer 4 px of the body's thin
  scrollbar. The scrollbar keeps the rest of its width, and the wheel
  scrolls the body as before.
- **N5** — touch. The overlay is a desktop window driven by a mouse; the
  handles take a left-button press only and the gesture is the window
  manager's from there, so no touch path was exercised.
