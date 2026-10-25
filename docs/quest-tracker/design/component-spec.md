# Quest tracker component spec

Harvested from the Figma comps on 2026-10-25. Sizes are px at 1× (1920×1080);
the app scales them by 1.33× and 2×. Token names are the keys of
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
  only (`opacity.default` 0.92, floor `opacity.min` 0.3). Text, icons and
  borders stay opaque.
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

### Row transition (QuestRow, in place of the toasts)
Not a component of its own: a live change of a quest's row (`qt:row-change`:
it became available, a step was reached, or it is done) plays on that row.
- Step out: the completed step's line fades to 0 over ~150 ms.
- Step in: then the next step's line fades in from ~0.2 opacity, moving up
  4-6 px into place, over ~220 ms, ease-out. A quest done has no next step:
  the row settles into its finished variant.
- A second change to the same row restarts the transition cleanly; nothing
  queues or stacks.
- `prefers-reduced-motion: reduce` (Windows animations off): the change is
  instant.
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
always stay fully opaque. Floor 30%."; under click-through "Off: the panel
blocks clicks to the game."

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
