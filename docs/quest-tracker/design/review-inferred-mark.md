# Review of the "probably done" mark

The design check of the mark added after launch 6 (owner, 2026-10-25): a
quest row or a detail step the view flags `inferred` (shown done because of
a later event the log recorded, not its own signal) carries a small dashed
ring beside its status or step mark. Built in
`apps/quest-tracker/src/components/InferredMark.svelte` and placed by
`QuestRow.svelte` (after the row's status mark), `QuestDetail.svelte` (after
each flagged step's mark, and after the detail head's status mark when the
row itself is flagged). The spec is `component-spec.md` § "'Probably done'
mark" and the brief's "Data the UI shows". Tools: impeccable 4.7.1
(`impeccable detect`, then its `audit` checklist, applied by the agent that
built the change). The skill ran as a subagent with no owner to ask: its
interview step was replaced by the brief, the component spec and the
Decisions entry of 2026-10-25 ("Show an inferred mark").

audit: ran (source only). The detector found nothing over `InferredMark.svelte`, `QuestRow.svelte` and `QuestDetail.svelte` (`detect --json` printed `[]`). Checklist, 19/20: accessibility 3 (the ring is a named image, `role="img"` with an `aria-label` and a `title` of the same words, its SVG `aria-hidden`; it differs from the done check in shape, a dashed ring against a check, so it reads without colour; `color.text-muted` #959084 on the panel's #121416 is 5.8:1, over SC 1.4.11's 3:1 for a graphic; it is no control and no focus stop, so play mode stays free of controls; the point lost: the tooltip is a `title`, which a keyboard user cannot open, see F1); performance 4 (one inline 10 px SVG circle, no animation of its own, no image request); theming 4 (its only colour is the `color.text-muted` token through `currentColor`; the 10 px size and 1.5 px stroke are the component spec's; no token added, so the brief's `Optional token keys` is unchanged); responsive 4 (a vector drawing, so the webview zoom at 1.33x and 2x keeps it sharp; no width, no new target); integrity 4 (no icon id or frame id added; the words live once, in `labels.ts`'s `INFERRED_LABEL`). The rendered check (`detect --cdp` against the running WebView2) did not run: this item builds no app, so the placement was checked by the DOM tests in `src/inferred.test.ts` and the spacing by reading the CSS, not by eye.
findings: none blocking; nothing to fix in this item's files. Two notes, decided rather than fixed:

- **F1** — the tooltip is the browser's `title`, shown on hover in edit
  mode only: play mode is click-through, so nothing there hovers, and the
  ring is deliberately no focus stop (the spec: not a control), so a
  keyboard user does not open it. The same words are its accessible name,
  read inside the row's button and beside the step's tick, so no reader
  loses them. Making the ring focusable would add a control the spec and
  `play_mode_has_no_controls` forbid.
- **F2** — in edit mode a step's tick is a 24 px hit area around its 14 px
  glyph, with negative margins so the glyph stays where play mode draws it.
  Its hover border therefore reaches 1 px into the ring's 4 px gap, at the
  ring's left edge. Moving the ring 2 px further out in edit mode only would
  make it jump on every mode toggle, while the glyph beside it stays still;
  the 1 px touch, visible only while the tick is hovered, is the lesser cost.

As built: a 10 px ring, radius 4.25, 1.5 px stroke in `currentColor`
(`color.text-muted`), dashed `2 1.34` (eight dashes round the
circumference). On a row it leads the title, 4 px after the 16 px status
mark (the row's 8 px column gap less 4) and 4 px before the name, centred on
the 20 px line. In a step it sits 4 px after the 14 px step mark, 4 px down,
so both centre 9 px into the 18 px step line in either mode. Tooltip and
accessible text: "Probably done: inferred from a later step the log
recorded." It changes nothing else: the status mark stays the done mark,
the manual-tick marker follows `needs_tick` as before, and in edit mode a
click on the ring is a click on the row (it opens the detail) or nothing in
a step (the tick beside it is the step's one toggle).

What to check by eye at the next live run (Live procedure 2's inferred-mark
check): the ring beside a done row at 1x and 2x, over a bright scene and a
dark one, at the panel's lowest opacity.
