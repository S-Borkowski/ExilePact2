# Quest tracker component spec

## Exile and Claude adoption

**Exile is the default theme**; **Claude** preserves the original specifications below. Both overlay Settings and the ordinary Settings window offer Theme → Exile / Claude. The preference is saved by Rust, sent to both windows and applied without changing mode, open frame, progress, opacity, zoom or saved geometry.

Exile follows the [approved compact steel close-up](../../ui-design-system/quest-tracker-overlay-approved.png). Thin weathered double steel rails and intertwined corner knots surround a quiet charcoal field. A narrow Cinzel Quest Tracker / actual Roman Act bar precedes compact character/class/level context. Act is shown only for exactly one received act; it remains truthful when Next up is empty. There is no current-player-zone field. Existing edit controls follow the context line. Flat full-width quest sections use gold mixed-case Georgia titles (19/25px), white Georgia current objectives (16/22px), secondary tips (14/20px), fine dividers with small center diamonds and diamond status marks. Character details, Worth it / Optional headings and reward labels remain visible in play by the owner's explicit decision. No boxed card fills, bevels, rounded card wells, gold architectural surround or crest are used.

The owner's latest status choice uses no inner icons in Exile rows or detail headings: in-progress solid gold/single outline; available hollow gold/double outline; locked hollow/thin muted single outline; done silver-filled/double outline; skipped diagonal hatch/single outline; unknown hollow/dotted outline. Fill and edge pattern distinguish every pair beyond colour alone. Each mark exposes its exact existing status label once and repeats it as a hover title. Status meanings and separate worth/manual/inferred marks are unchanged. Claude retains all six original icons. Since Live 2 (owner, 2026-11-02) a status colour fills only the inside of the diamond: every outer ring keeps the kit's steel stroke and is never filled, and in progress (ivory) and done (silver) carry the kit's fill on an inner diamond (radius 7 of the ring's 10), with a gap of the well between the two; in progress keeps its upper-facet highlight inside that fill and done its inner line.

Every expanded Exile frame and ordinary Settings retains this same steel family. Mode changes controls/input only. Play has a noninteractive configured-hotkey · Edit footer; edit retains the original return-to-play command and full accessible description in a compact footer. Small metadata/rewards/forms retain Alegreya Sans; toggles/sliders/focus use warm gold. The collapsed badge keeps its small shape and selected palette. Decorative art is hidden from assistive technology and takes no input. All controls/actions and 22 feature groups remain: [preservation map](approved-ui-preservation.md).

The Exile custom art frame stays fixed and is the only outer border: the legacy CSS ring and synthetic inset frame shadows are disabled. Its content scrolls in a separately inset, clipped well. The existing functional header and a fixed noninteractive top-rail grab area use the existing header-drag bridge in edit mode. The rail remains reachable after scrolling Settings to the end. Buttons, selects, fields, right clicks and passive play start no drag; all eight resize edges remain separate. Control focus outlines and Claude's original border/header gesture remain unchanged.

Exile wraps titles/steps/tips and permits vertical scrolling. The title bar and footer sit between the frame's corner knots and only the body scrolls, its clip inside the rails; when the body would be shorter than 72 px, or than its tallest row or control plus 8 px, the whole content scrolls instead, in a well inset 24 px from the sides, 16 px from the top and 33 px from the bottom, clear of the knots. Controls wrap at narrow effective widths. The frame is the kit's `frame-neutral-steel.png`: nine-slice 224, drawn 48 px, flush with the window edge: per-side outsets of 9 / 8 / 13.5 / 8.5 px (top, right, bottom, left), just under the PNG's own transparent padding, carry only that padding past the edge, all constant on screen at every scale; the reading inset is 20 px at the sides, 14 px top and 21 px bottom, in the overlay and the ordinary Settings window alike. `npm --prefix apps/quest-tracker run corner-safety` checks every case against the rendered art, and that the frame is flush. Native default364×384 is all paint in Exile, with no transparent margin (Claude keeps its12px margin around340×360 paint); saved sizes win, minimum320×240 remains. Theme selection never changes native bounds. [Rendered review](review-approved-ui.md) records evidence and limits. This UI adoption introduces no new game behavior.

Exile readability (owner, Live 2, 2026-11-02), checked by `corner-safety` on every case, scale and scroll position: text renders at least 14 px on screen for quest names and frame or window titles, 13 px for objectives, tips and detail text, and 12 px for everything else, at every scale; each size is the larger of its scale-1 size and the floor divided by the scale's zoom, so the floor only bites below scale 1. Every text run reaches 4.5:1 contrast against the worst pixel behind it, over the masonry well at the default opacity (a disabled control's text is exempt, as in WCAG). No faux small caps; the caps face (Cinzel) only at 13 px rendered and up; weight 400 or more; tracking at most 0.05em under 13 px and 0.1em above. Play-mode scrollbar: play is click-through, so in Exile play every scroller of the panel hides its scrollbar and still scrolls (a new view or a restored position moves it); edit, the overlay Settings frame, the character picker and the ordinary Settings window keep the kit's 8 px scrollbar.

Exile Next up sections (owner, Live 3, 2026-11-02; the kit's `previews/play.png` order), checked by `corner-safety`: each worth group (Worth it, Optional, Not worth it) is one section element labelled by its section heading through `aria-labelledby`. The section heading is the existing label in Cinzel caps (13 px rendered or more, 0.08em tracking) in kit gold `color.worth-gold`, led by that group's worth icon at 16 px (decoration; the text names the section), with 8 px above and 4 px below at scale 1. The section break is the kit quest divider as drawn elsewhere (3 px dark band, 1 px steel line, 7 px centre diamond, 10 px high, full width), once above the heading of every section but the first. Between two quests of one section only the divider's fine 1 px steel line (`color.panel-divider`) is drawn, with no band and no diamond; there is none under a section's last quest, so no divider sits right above a heading. Claude's Next up keeps its flat list, and the All view's act sections keep the full divider between quests.

Harvested from the Figma comps on 2026-10-25. Sizes are px at 1× (1920×1080);
the app scales them by Auto (from the monitor) or a fixed factor from 0.75×
to 2× the player sets. Token names are the keys of
`apps/quest-tracker/theme/theme.json` (CSS custom properties in `tokens.css`
use the same name with the dot as a dash: `color.panel-bg` →
`--color-panel-bg`). Every variant has a reference image under
`components/<Component>/`, and every frame under `frames/`.

## Global rules

- **Grid:** 4 px (`space.unit`). Panel width 320 (`size.panel-width`) in
  the comps. Since 0.1.1 the panel fills the window inside its 12 px
  shadow margin, in both directions, and its body scrolls: the player sizes
  the window by the panel's edges in edit mode. The original comp paint was
  320×384; current new-store paint is340×360 in a364×384 native window.
  Saved sizes win, minimum320×240 remains, and scale only zooms content.
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
Both themes add Theme first in overlay Settings: Exile (default) / Claude,
using the existing saved settings command. All previous settings follow.
Padding 0/12; label (Label) left, control right. Helper text below a row
(Caption, `color.text-muted`, padding 0/12/6): under opacity "Text and icons
always stay fully opaque."; under "Read the screen" "Reads the game's
window for boss bars and quest dialogue. Saves nothing it reads." Since
0.1.1 there is no click-through row and no "Hide under game panels" row.

Rows after Live 2 (owner, 2026-10-25): opacity is a Slider from 0% to 100%
in 5% steps with its value as a percentage; scale is a Toggle "Auto" and a
Slider for a fixed factor from 0.75× to 2× in 0.05 steps, its value shown
as `1.25×`, and moving the slider turns Auto off; "Show skipped quests" is a
Toggle whose helper says a skipped quest shows even with "Only worth-it" on.
There is no dock-side row: the panel is dragged by its header in edit mode.
"Read the screen" (owner, 2026-10-26) is a Toggle, off by default, right
after "Only worth-it quests", whose helper says it reads the game's window
for boss bars and quest dialogue and saves nothing.
"Hide under game panels" (owner, 2026-10-27) was a Toggle after "Read the
screen" in 0.1.0; since 0.1.1 (owner, 2026-10-28) the row is gone, and
hiding under game panels is always on in play mode.

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
  to). The window is as tall as its content, 328 at 1× (its tallest
  state, a download, fits). A 1 px
  `color.panel-divider` line between sections and above the footer.
- Updates: SettingRow "Automatically check for updates" + Toggle; helper
  "Asks GitHub once a day whether a newer version is out. Downloads
  nothing. Off: Quest Tracker never connects unless you ask it to check.";
  status line (Step, `color.text-primary`, 16 icon 8 before it:
  `ui-refresh` checking, `ui-check` up to date, `status-available` in
  `color.status-available` available, `ui-warning` in `color.warning`
  failed, none otherwise); the install line 8 under it, same style, only
  while an install has something to say (`ui-warning` in `color.warning`
  unsigned and failed, `ui-refresh` downloading and verifying, `ui-check`
  restarting); while downloading, a progress row 8 under that, starting
  at the line's text (24 in): a 4 high bar, `color.panel-divider` filled
  with `color.edit-accent` up to the bytes received (bare track with no
  total), and the amount 8 after it (Caption, tabular figures, "0 KB of
  4 KB"); buttons 8 apart: "Check now" (secondary, disabled while
  checking, downloading, verifying or restarting), then at most one of
  "Install 1.3.0 and restart" (primary, the app's label, ready only,
  disabled while checking), "Cancel" (secondary, downloading) and "Try
  again" (secondary, failed). No button or link opens a release page.
  "Version 1.2.0" (Caption, `color.text-muted`, tabular figures).
- Footer: "Created by ST4H" (Caption, `color.text-muted`), plain text,
  padding 12/16.
