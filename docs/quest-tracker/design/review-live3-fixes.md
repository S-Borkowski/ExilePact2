# Design review: the fixes after Live 3

Date: 2026-10-26. Scope: a quest row with a find-it tip under its step
(`apps/quest-tracker/src/components/QuestRow.svelte`, `.tip`), the step tips
in a quest's detail (`QuestDetail.svelte`, `.step-tip`), a skipped row in
Next up with the step its tracked progress reached, and the settings
sliders that apply while dragged (`Settings.svelte`, with `src/slider.ts`).
Tool: the `impeccable` skill 4.7.1, `audit` command, with its bundled
detector (`impeccable detect --json` over the three components). The audit
was code-level: no browser or running overlay was available in the session,
so no rendered contrast read, screenshot or real touch gesture was made; the
drag was exercised by synthesized pointer events in jsdom (`live3.test.ts`).
Contrast was computed from the harvested tokens in `theme/tokens.css` (WCAG 2
relative luminance), over the panel background at full opacity.

audit: 18/20 (Excellent) after the fixes below. Accessibility 4, Performance 4, Theming 4, Responsive 3, Implementation integrity 3. The detector reported no finding (`[]`).

findings: 2 fixed in this item (P2 the scale slider's pointer drag had no `touch-action` and nothing ended it on a lost pointer capture; P2 closing the settings frame mid-drag dropped the value still waiting for its frame), 2 left open with reasons (P3 a tip longer than two lines is clipped with no way to read the rest; P3 the Auto switch reads on until the shell's first view after a drag arrives).

## Findings

- **[P2, fixed] The scale slider's own drag could be lost.** The scale
  slider tracks the pointer itself (its value follows the pointer's travel
  in screen pixels, because each update re-zooms the page), so the range's
  native drag is turned off. Without `touch-action: none` a touch drag on a
  touch screen would be claimed as a pan, and a drag the system took away
  (lost pointer capture, a focus change mid-drag) left the drag state set,
  so the next pointer move over the slider would still have moved the
  value. Fix: `.slider.tracked { touch-action: none; }`, and
  `lostpointercapture` ends the drag like a release (`endScale`).
  `live3.test.ts` (`scale_slider_sends_at_most_one_update_per_frame`) holds
  both.
- **[P2, fixed] Closing the frame mid-drag dropped the last value.** Each
  slider sends at most one update per animation frame; a value moved to in
  the frame the settings closed was cancelled with the frame. Fix: the
  component flushes both senders on teardown, so the last value always
  reaches the shell. Held by the same test.
- **[P3, open] A long tip is clipped at two lines.** The row clamps the tip
  to two lines (the brief's "at most two lines"), and play mode has no hover
  to show the rest. The owner's five tips are one short line each at 320 px;
  the detail shows every tip in full. Left open: a later, longer tip should
  be shortened in the data rather than given a second surface.
- **[P3, open] The Auto switch lags a drag by one view.** Moving the scale
  slider sends a fixed factor; the shell turns Auto off and the switch
  follows the next `qt:view`, a frame or so later. The UI keeps no copy of a
  setting of its own (the frame's rule since Live 2), so the switch is not
  turned off locally.

## Checked and passing

- **Contrast.** The tip is `color.text-secondary` at 12/16: 9.16:1 on the
  panel background and 8.50:1 on the in-progress band. A skipped row's step
  and tip take `color.text-muted`, like the rest of a finished row: 5.80:1
  on the panel background, and 4.81:1 under a highlight's 14% skipped tint
  (`review-live2-fixes.md`). The step stays unstruck (only the name carries
  the skipped line-through), so it reads.
- **Tokens.** The new rules (`.tip`, `.step-tip`, `.slider.tracked`) read
  existing tokens only; no icon, token key or frame was added. Tests hold the
  `.tip` and `.step-tip` token uses to `tokens.css` and `defaults.css`.
- **Motion.** The tip leaves with the completed step and comes in with the
  new one on the row's existing transition (opacity and a 5 px rise, strong
  ease-out); under reduced motion the change is instant, by the existing
  `.entering` rule. The sliders themselves do not animate: the value, the
  fill and the readout follow the pointer 1:1 within the frame.
- **Feedback while dragging.** Both sliders show the pointer's value at once
  (thumb, fill, the value readout and `aria-valuetext`) and hold it against
  an older view still in flight, so the thumb never steps back under the
  pointer; the view takes over once it shows the value, or a second after the
  release. A press on the scale thumb does not jump it; a press on the track
  jumps there, as a range does. The keyboard steps both as ranges.
- **Play mode.** The tip lines add no control; `play_mode_has_no_controls`
  passes over every frame.
