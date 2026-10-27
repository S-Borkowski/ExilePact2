# Design review: the fixes after Live 2

Date: 2026-10-25. Scope: the Settings frame as changed after Live 2
(`apps/quest-tracker/src/components/Settings.svelte`: opacity slider from 0%,
an Auto scale switch plus a scale slider, no Dock side row), the highlighted
quest row (`QuestRow.svelte`, `data-highlight`) and the collapsed badge's
highlight (`Collapsed.svelte`), and the edit-mode header drag
(`Header.svelte`). Tool: the `impeccable` skill 4.7.1, `audit` command, with
its bundled detector (`impeccable detect --json` over the four components).
The audit was code-level: no browser or running overlay was available in the
session, so no rendered contrast read or screenshot was made. Contrast was
computed from the harvested tokens in `theme/tokens.css` (WCAG 2 relative
luminance), over the panel background at full opacity.

audit: 17/20 (Good) after the fixes below. Accessibility 3, Performance 4, Theming 4, Responsive 3, Implementation integrity 3. The detector reported no finding (`[]`).

findings: 2 fixed in this item (P1 muted-text contrast on a highlighted finished row; P2 slider values read as bare numbers), 1 fixed P3 (badge tint margin), 3 left open with reasons (P2 solid row bands at opacity 0, P3 no keyboard way to move the window, P3 the badge's own easing curve).

## Findings

- **[P1, fixed] Muted text on a highlighted finished row fell under AA.**
  A done, skipped or locked row draws its name and rewards in
  `color.text-muted`. Under the highlight's 20% tint of its status colour
  that text measured 4.12:1 (done), 4.35:1 (skipped) and 4.48:1 (locked),
  under 4.5:1 for 13-14 px text. Fix: a finished row's tint is 14%, giving
  4.64, 4.81 and 4.90:1. Unfinished rows keep 20%: their lowest text is
  `color.text-secondary`, at 5.71:1 or more on every tint (in progress on its
  active band included). `live2.test.ts` holds the 0.14 rule.
- **[P2, fixed] The sliders announced bare numbers.** The scale slider
  rests at 1 while Auto is on, so a screen reader would have read "1" for
  Auto; the opacity slider read "0.35" for 35%. Fix: `aria-valuetext` carries
  the shown value ("Auto", "1.25×", "35%").
- **[P3, fixed] The badge's tint left a thin margin.** At 30% the available
  colour took the gold count to 4.61:1 and secondary glyphs to 4.52:1. The
  badge now uses the row's 20% (6.05:1 and 5.94:1 or more).
- **[P2, open] At 0% opacity the in-progress band and the edit-mode header
  tint stay solid.** They are row and header fills
  (`color.row-active-bg`, `color.edit-tint`), not the panel background, and
  they carry meaning (the quest in progress, edit mode), so they were left as
  they are. Whether they should fade with the panel is for the owner to judge
  in Live procedure 3.
- **[P3, open] The window moves by mouse only.** The edit-mode header drag
  is a pointer gesture on the window's own title bar, as in any desktop
  window; the header's controls stay keyboard-reachable buttons. Moving the
  window from the keyboard would be a shell feature, outside this item.
- **[P3, open] The badge writes its fade curve out**
  (`cubic-bezier(0.23, 1, 0.32, 1)`, the row transition's curve), because
  the row's `--row-ease` is local to `QuestRow.svelte`. No theme token holds
  it; the harvested `motion.easing` is the controls' curve. Left as is.

## What holds

- The highlight is colour only (an opacity-faded overlay under the row's
  content, and a background colour on the badge), so it shows with Windows'
  animations off, and nothing in it moves. It comes on at once, holds 2.5 s
  and fades out over 220 ms on the strong ease-out; a second change restarts
  the hold without blinking off, so it never flashes.
- Every colour in the new rules is an existing token: the status colours,
  `color.panel-bg` and `opacity.default`. No token key was added.
- The Settings rows keep the frame's 40 px rows, the 36 × 24 switch and the
  slider of the opacity row; each control sends one field and shows what the
  next view says. Play mode lists the same settings as text, with no control.
- At 0% opacity the panel's ring (border, and the 2 px edit accent) is drawn
  by `.cut::after`, which the opacity setting does not touch.
