# Quest tracker component spec

Harvested from the Figma comps on 2026-10-25. Sizes are px at 1× (1920×1080);
the app scales them by Auto (from the monitor) or a fixed factor from 0.75×
to 2× the player sets. Token names are the keys of
`apps/quest-tracker/theme/theme.json` (CSS custom properties in `tokens.css`
use the same name with the dot as a dash: `color.panel-bg` →
`--color-panel-bg`). Every variant has a reference image under
`components/<Component>/`, and every frame under `frames/`.

## Global rules

- **Grid:** 4 px (`space.unit`). Panel width 320 (`size.panel-width`),
  height at most 60% of the screen (648 at 1×); the panel scrolls beyond it.
- **Shape:** radius 0 everywhere (`radius`, `radius.row`, `radius.button`).
  Corners are cut at 45° on the top-left and bottom-right only: panel 6 px
  (`chamfer.panel`), buttons 3 px, the badge 4 px. In CSS, a
  `clip-path: polygon(...)` on the element plus an inner pseudo-element for
  the border.
- **Surface:** panel `color.panel-bg`, border 1 px `color.panel-border`,
  shadow `shadow.panel`, a 1 px inner top highlight of white at 5%. Edit
  mode: border 2 px `color.edit-accent` (solid), header `color.edit-tint`.
- **Opacity:** the Settings slider sets the opacity of the panel background
  only (`opacity.default` 0.92, from `opacity.min` 0 to 1 in 0.05 steps;
  the 0.3 floor went after Live 2, owner, 2026-10-25). At 0 the background
  is gone; text, icons, the border and the edit-mode accent stay opaque.
- **Type:** Alegreya Sans (`font.family`), act headers in Alegreya Sans SC
  Bold (`font.family-caps`). Nothing under 12 px.

| Style | Font | Size / line | Use |
|---|---|---|---|
| Title | Bold 700 | 16 / 20 | Character name, panel titles |
| Row | Medium 500 | 16 / 20 | Quest name (14 / 20 on finished rows) |
| Step | Regular 400 | 14 / 18 | In-progress step line, body copy |
| Label | Medium 500 | 13 / 16 | Reward text, settings labels |
| Button | Bold 700, +0.5 tracking | 13 / 16 | Buttons |
| Act | SC Bold 700, +3% tracking | 13 / 16 | Act headers, group labels |
| Caption | Medium 500, +0.2 tracking | 12 / 16 | Hints, footers, helper text |
| Badge count | ExtraBold 800 | 22 / 24 | Collapsed badge |
| Display | Bold 700 | 20 / 24 | Quest detail title |

- **Status colours:** locked `color.status-locked`, available
  `…-available`, in progress `…-in-progress`, done `…-done`, skipped
  `…-skipped`, unknown `…-unknown` (violet). Shape always differs too (see
  the icon set). Errors and warnings use `color.warning`.
- **Worth:** worth it = solid star `color.worth-gold`; optional = outline
  star `color.text-secondary`; not worth it = dash `color.worth-none`. On
  done, skipped and locked rows every marker is `color.text-muted`.
- **Motion:** the row transition (below, after QuestRow): the completed
  step out over ~150 ms, the next step in over ~220 ms, ease-out; instant
  under `prefers-reduced-motion: reduce`. Reset confirm 3 s
  (`motion.confirm-ms`); its countdown drains a 2 px bar. There are no
  toasts and no Undo toast (owner, 2026-10-25, after Live 1).

## Components

### PanelHeader — 320×36
Padding 0/12, gap 8 (6 in edit). Variants:
- `mode=play`: status/character icon 16, name (Title), class·level
  (Caption, `color.text-muted`, fills), view label (Caption,
  `color.text-secondary`, right).
- `mode=edit`: background `color.edit-tint`; EDIT chip (teal fill
  `color.edit-accent`, pencil 12 + "EDIT" 12 ExtraBold in
  `color.panel-sunken`, padding 2/6/2/5); name + 12 px chevron (opens the
  character picker); ViewSwitch; collapse and settings IconButtons (24).
- `mode=log-idle`: as play, with a 14 px warning glyph (`color.warning`)
  and "No log update · 5 min" (Caption, `color.text-secondary`). Rows below
  it render at 55% opacity (`opacity.stale-rows`).
- With no character: name reads "Quest tracker" in `color.text-muted`, no
  view switch, no chevron.

### ActHeader — 320×28
Padding 0/12. Chevron 12, "Act 2" (Act style), right side: available icon
12 + "4 available ·" + "1/9 done" (Caption). Background `color.panel-raised`.
Variants: expanded/collapsed × available yes/none. Only the current act is
listed.

### QuestRow — 320×40 (in progress 56, finished 32)
Auto layout, padding 8/12 (in progress 8/12/10/12; finished 6/12), gap 8.
Status mark 16 · body (fills): name (Row) + manual marker 12 after the name
+ step line (Step, `color.text-primary`, in progress only) · reward text
(Label, `color.text-secondary`, muted on finished rows) · worth marker 16.
- In progress: background `color.row-active-bg`, 3 px bar `color.status-
  in-progress` inset 3 px from the left edge, so it shows inside the edit
  border.
- Not worth it (live rows): name in `color.text-secondary`.
- Finished (done, skipped, locked): 32 high, name 14, all text
  `color.text-muted`; skipped names are struck through.
- Hover (edit mode only): 1 px inner outline `color.hover-outline`, no fill
  change; click opens quest detail.
Variants: status (6) × worth (3) × detection (auto/manual). Manual covers
"partial".
- Tip line (after Live 3, not in the Figma comps): when the step the row
  shows has a find-it tip (`current_tip`), it sits on its own line under
  the step line, in play and edit mode: Step style, `color.text-secondary`,
  wrapping to at most two lines (ellipsis after), gap 2 above. The row
  grows by the tip's lines. No tip, no line.
- Skipped with progress (after Live 3): a skipped row whose tracked
  progress reaches a step shows that step line and its tip line, in
  `color.text-muted` like the rest of a finished row; it keeps the skipped
  mark and the struck-through name.

### Row transition (QuestRow, in place of the toasts)
Not a component of its own: a live change of a quest's row (`qt:row-change`:
it became available, a step was reached, or it is done) plays on that row.
- Step out: the completed step's line fades to 0 over ~150 ms.
- Step in: then the next step's line fades in from ~0.2 opacity, moving up
  4-6 px into place, over ~220 ms, ease-out. A quest done has no next step:
  the row settles into its finished variant.
- A second change to the same row restarts the transition cleanly; nothing
  queues or stacks.
- Highlight (after Live 2): every change also tints the row's band or
  background with an existing token (the new status colour or
  `color.edit-accent`) for ~2.5 s, then clears; a second change restarts
  the hold. Colour or opacity only, never a flash. The collapsed badge
  shows it too.
- `prefers-reduced-motion: reduce` (Windows animations off): the step swap
  is instant and nothing moves; the highlight still shows.
- Every change is announced once in a polite live region
  (`aria-live="polite"`), the change's one sentence. A row not on screen
  (its act folded, or filtered out) is announced and not animated.

### StatusMark 16, WorthMarker 16, ManualTick 12
Icon instances, colour from the tokens above. ManualTick is the eye with a
slash (`manual-tick.svg`), `color.text-muted`.

### RewardChip — icon 20, with label 24 high
Icon only, or icon + label (Label) with padding 4/8 on `color.panel-raised`.
Used in quest detail only. Resistances use their `color.res-*`.

### StepLine — 288×20
Pending: empty box 14, `color.text-secondary`. Current: in-progress glyph,
text `color.text-primary`. Done: check-in-box, `color.text-muted`.
A step with a find-it tip (`tip`, after Live 3) shows it under the step's
text, aligned with it: Step style, `color.text-secondary`, wrapping; the
line grows to fit.

### "Probably done" mark (QuestRow, StepLine), added after launch 6
Not in the Figma comps (owner, 2026-10-25, after launch 6): the view marks a
done row or a done step as inferred (`inferred: true`) when it shows done
because of a later event the log recorded, not its own signal; the brief's
"Data the UI shows" says when.
- Placement: right after the row's status mark 16, and right after the
  step's done mark 14, gap 4; it never replaces the done mark.
- Shape: a 10 px dashed ring, 1.5 px stroke, `color.text-muted`, drawn by
  the UI (no icon id). It differs from the done check in shape, so it reads
  without colour.
- Tooltip and accessible text, the same words: "Probably done: inferred from
  a later step the log recorded."
- Not a control: no hover outline, no click, no toggle, no tick action; in
  play mode it is click-through like the rest of the row. The manual-tick
  marker and the step's tick behave exactly as without it.
- Tokens: existing ones only (`color.text-muted`); a new one,
  if the styled layer needs it, goes under the brief's `Optional token
  keys`.

### Button — h 28, padding 0/12, gap 8, label Button style, 3 px cut corners
- primary: fill `color.action-primary-bg`, text `color.text-on-light`;
  hover `…-primary-hover`; pressed `…-primary-pressed`.
- secondary: fill `color.action-secondary-bg`, 1 px `color.panel-border`;
  hover/pressed accordingly.
- danger: outline and text `color.action-danger`. `state=confirm`: filled
  `color.action-danger`, text `color.panel-sunken`, "Confirm reset", 120
  wide, 2 px countdown bar along the bottom (3 s); a second click resets.
- disabled: `color.action-disabled-bg` / `-fg`.

### Toggle 36×24, Slider 200×24, Dropdown 140×24
Square (radius 0). On = `color.edit-accent` track. Slider track
`color.panel-divider`, fill `color.edit-accent`, thumb `color.text-primary`.

### SettingRow — 320×40
Padding 0/12; label (Label) left, control right. Helper text below a row
(Caption, `color.text-muted`, padding 0/12/6): under opacity "Text and icons
always stay fully opaque."; under click-through "Off: the panel
blocks clicks to the game."

Rows after Live 2 (owner, 2026-10-25): opacity is a Slider from 0% to 100%
in 5% steps with its value as a percentage; scale is a Toggle "Auto" and a
Slider for a fixed factor from 0.75× to 2× in 0.05 steps, its value shown
as `1.25×`, and moving the slider turns Auto off; "Show skipped quests" is a
Toggle whose helper says a skipped quest shows even with "Only worth-it" on.
There is no dock-side row: the panel is dragged by its header in edit mode.
"Read the screen" (owner, 2026-10-26) is a Toggle, off by default, right
after "Only worth-it quests", whose helper says it reads the game's window
for boss bars and quest dialogue and saves nothing.
"Hide under game panels" (owner, 2026-10-27) is a Toggle, off by default,
right after "Read the screen" and its helper, whose helper says it hides
the tracker while a game panel is open under it, reads a few pixels of the
game's window and saves nothing; play mode lists it as On or Off after "Read the screen".

### Hotkeys block (Settings)
Heading "Hotkeys · click a key to rebind" (Label); rows label (Label,
`color.text-secondary`) + keycap (12 Bold, `color.panel-sunken` fill, 1 px
`color.panel-border`, padding 1/6): Edit mode F8, Collapse / expand F7,
Switch view F6, Icon key F1.

### CharacterRow — 320×48
Padding 0/12, gap 12: character icon 20, name (Row) over class·level
(Caption). Active: `color.select-active-bg` with a 2 px `color.edit-accent`
left bar and "Tracking". Hover: `#181B1E`.

### CollapsedBadge — 48×48, 4 px cut corners
- `idle`: 1 px `color.panel-border`, expand glyph `color.text-secondary`.
- `has-new`: 2 px `color.worth-gold`, solid star 14 + count (22 ExtraBold,
  gold).
- `warning` (log missing or idle): 1 px `color.warning`, warning glyph 20.

### Empty and error states
Body padding 16, gap 8: glyph 24, title (Title), body (Step,
`color.text-secondary`), then captions. "Can't find Client.txt" adds
"Looked in: <path>" and "Your manual ticks are kept." (Caption); edit mode
shows Browse… (primary) and Retry (secondary); play mode shows "Press [F8]
then ⚙ › Log file". "No character yet": Add character / "Press [F8] then
add a character". "Waiting for the game": "Press [F8] then ⚙ › Log file".

### Icon key panel
Opened from Settings or F1, closed by Esc or ✕. Groups (Act style): Status,
Worth it, Detection; each row an instance + name (14 Medium) over meaning
(Caption).

### SettingsWindow — 420 wide, its own window (owner, 2026-10-27)
Not an overlay frame: the tray's Settings window (the brief's
"The Settings window"), built from the components above, no new token.
- Window: fill `color.panel-bg` edge to edge, no border, no cut corners or
  shadow (the native title bar frames it). Padding 16 at the sides.
- Title "Settings" (Title), 16 above and 4 below. Each section: a group
  label (Act, `color.text-muted`), then SettingRows (40 high, padding
  0/16 here) and their helper lines (13 / 18, `color.text-muted`: larger
  than the panel's Caption, as the help states what the window connects
  to). The window is as tall as its content, about 280 at 1×. A 1 px
  `color.panel-divider` line between sections and above the footer.
- Updates: SettingRow "Automatically check for updates" + Toggle; helper
  "Asks GitHub once a day whether a newer version is out. Downloads
  nothing. Off: Quest Tracker never connects unless you ask it to check.";
  status line (Step, `color.text-primary`, 16 icon 8 before it:
  `ui-refresh` checking, `ui-check` up to date, `status-available` in
  `color.status-available` available, `ui-warning` in `color.warning`
  failed, none otherwise); buttons 8 apart: "Check now" (secondary,
  disabled while checking), "Open the release page" (primary,
  `ui-external-link` 14 after the label, only while a newer version is
  known); "Version 1.2.0" (Caption, `color.text-muted`, tabular figures).
- Footer: "Created by ST4H" (Caption, `color.text-muted`), plain text,
  padding 12/16.
