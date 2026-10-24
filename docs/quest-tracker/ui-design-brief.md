# Quest Tracker Overlay — Figma Design Brief

The owner designs the overlay's look in Figma from this brief; the code then
consumes the Figma file through the Figma MCP design-to-code workflow. The
overlay is a separate always-on-top window drawn over Path of Exile 2
(windowed or borderless fullscreen), fed only by the game's `Client.txt`.

Status: toolkit settled. The overlay is a Tauri app (Rust and the WebView2
runtime, with an HTML/CSS/TypeScript UI), shipped as one `quest-tracker.exe`
— the owner's decision of 2026-10-25. Everything below is designed with the
web's means; the names in [Export contract](#export-contract) are final.

## Design constraints

- **Over a moving game.** The panel sits on top of bright, dark and busy
  scenes. Every text and icon must stay legible on pure white and pure black
  backgrounds alike: use a solid or near-solid panel background, not
  see-through text.
- **Small and out of the way.** Default docked to a screen edge (right side,
  below the minimap area). Collapsed it must be no bigger than ~48×48 px;
  expanded, about 320 px wide and at most 60% of screen height.
- **Scales.** Design at 1920×1080 (1×). It will be scaled to 1440p (1.33×)
  and 4K (2×) by the webview's zoom, so keep icons as vectors and sizes on a
  4 px grid.
- **What the web UI can do over the game.** The window is transparent per
  pixel, so these all work and may be designed for: rounded corners, soft
  shadows (drawn by CSS inside a transparent margin around the panel; the
  window itself has no system shadow), CSS gradients, translucency per
  element, SVG used directly and coloured by CSS, variable fonts,
  letter-spacing and CSS motion. Fonts and images are bundled with the app:
  no remote font or image (the app's content security policy blocks every
  remote load), and a font needs a licence that allows bundling. Hover
  states exist only in edit mode; in play mode the mouse passes to the game.
- **One firm no: no frosted glass.** `backdrop-filter`, or any blur of the
  game behind the panel, cannot work: it only sees the overlay's own pixels,
  never the game's window, so a frosted panel would blur nothing. Do not
  design for it.
- **Readable at a glance.** The player looks for under a second mid-fight:
  status must read from colour *and* shape, never colour alone.
- **Don't mimic GGG's UI exactly** — no game logos, fonts or art taken from
  the game files. A dark, gothic-adjacent style is fine; original work only.

## Data the UI shows

Per quest: name, act, giver NPC, zone, reward(s), "worth it" rating, status,
current step text, and whether progress is detected automatically from the
log or must be ticked by hand.

Status values, per character (the first that applies wins):

- **skipped** — the player chose not to do it.
- **done** — the log recorded its completion, or the player marked it done.
- **unknown** — the quest's window has passed (the story moved on) with no
  completion in the log, and the log cannot tell whether it was done: needs
  a manual tick.
- **in progress** — a step was seen in the log or ticked by hand; the
  current step is the furthest one reached.
- **available** — it can be picked up: its unlock appeared in the log, or it
  needs none.
- **locked** — not yet available.

Detection values, per quest: **auto** (the log records the quest's
completion), **partial** (the log records some of its unlocks or steps but
not the completion), **manual** (the log records nothing usable). The quest
row's manual variant covers both partial and manual; the manual-tick marker
shows on both.

Reward categories (one icon each), as measured on poe2db when the quest data
was planned: passive skill points, weapon-set passive points, spirit,
resistance (fire / cold / lightning / chaos, four icons, plus one for all
elemental resistances), maximum life, maximum mana (two separate icons),
attribute, charm (slot), flask (charges or slot), defences, other permanent
stat, skill gem, support gem, ascendancy trial, item or currency, unlock
(waypoint, vendor, feature), story only (no reward).

## Frames to design

Each frame's id from the export contract is in brackets; name the Figma
frame exactly that.

1. **Expanded panel — campaign view** [`panel-campaign`]. Header, the
   current act's header and its quest rows. Only the current act is shown;
   other acts are not listed (owner, 2026-10-25). Show at least one row in
   each status.
2. **Expanded panel — "next up" view** [`panel-next-up`]. Only the current
   act's quests that are available or in progress, worth-it ones first.
   This is the default view while playing.

In every view, an in-progress quest always shows its next step as a second
line under its name (owner, 2026-10-25).
3. **Collapsed state** [`collapsed`]. The small docked button/badge, with a
   count of worth-it quests currently available.
4. **Quest detail** [`quest-detail`]. What opens when a row is clicked: all
   steps with a tick per step, every reward, giver and zone, the "worth it"
   reason, the poe2db link, and the buttons Mark done / Skip / Reset.
5. **Character picker** [`character-picker`]. The overlay tracks progress
   per character; a list of detected characters (name, class, level), the
   active one highlighted, and "new character".
6. **Settings** [`settings`]. Opacity slider, click-through toggle, dock
   side (left/right), scale (auto/1×/1.33×/2×), "show skipped quests", "only
   worth-it quests", log file path with a status line.
7. **Edit mode vs. play mode** [`mode-play`, `mode-edit`]. The same panel in
   both. Play mode is click-through (mouse goes to the game) and shows no
   buttons; edit mode (hotkey) accepts clicks and shows the controls. The
   two must be distinguishable at a glance — e.g. a coloured frame or header
   tint in edit mode.
8. **Empty and error states.** No log found [`empty-no-log`]; log found but
   the game hasn't written anything yet [`empty-waiting`]; no character
   detected yet [`empty-no-character`]; all quests in this act done
   [`empty-act-done`].
9. **Toast** [`toast`]. A short notification when a quest becomes available
   or a step completes (~3 s, stacks at most 3, top of the panel or near
   it).

## Components (make each one a Figma component with variants)

| Component | Variants | Approx. size @1× | Notes |
|---|---|---|---|
| Panel header | play / edit | 320×36 | Title, active character, view switch, collapse button, settings button (edit only) |
| Act section header | expanded / collapsed; has-available / none | 320×28 | "Act 2", progress like `5/9`, chevron |
| Quest row | status × worth-it (yes/no/optional) × detection (auto/manual; manual also covers partial) | 320×40 | Status mark, name, short reward text (e.g. "Spirit", "Passive · Cold res"; owner, 2026-10-25), worth-it marker; current step as a second line when in progress, always shown (row grows to ~56) |
| Status mark | locked, available, in progress, done, skipped, unknown | 16×16 | Must differ in shape, not just colour |
| Worth-it marker | worth it / optional / not worth it | 16×16 | E.g. star / half / dash |
| Manual-tick marker | — | 12×12 | Shows the row needs a manual tick (detection partial or manual) |
| Reward chip | one per reward category | 20×20 icon, optional label | Icon+label in quest detail only; rows show reward text instead |
| Step line | pending / current / done | 288×20 | Used in quest detail |
| Button | primary / secondary / danger; default / hover / pressed / disabled; danger also confirm | h 28 | Mark done, Skip, Reset. Reset asks twice: the first click relabels it "Confirm reset" for 3 s |
| Toggle, slider, dropdown | standard states | h 24 | Settings |
| Collapsed badge | idle / has-new (count) | 48×48 | Count of available worth-it quests |
| Toast | quest available / step done / quest done | 300×44 | Icon + one line |

## Icons (24×24 grid, SVG only, single colour unless noted)

The original SVG set is in [`icons/`](icons/README.md), with a labelled
[specimen sheet](icons/preview.html). This completes the icon deliverable;
the overlay frames, components and theme remain a separate design task.
Every path in the set is newly drawn; it contains no game art.

Icons are SVG only. The UI draws each SVG directly at any scale and colours
it with CSS (`currentColor`), so no PNG scales are exported.

- Reward icons: passive point, weapon-set point, spirit, resistance (×4
  colours, plus all elemental resistances), life, mana, attribute, charm,
  flask, defences, other permanent stat, skill gem, support gem, ascendancy,
  item/currency, unlock, story-only.
- Status: lock, available (open/exclamation), in progress, done (check),
  skipped (strike/arrow), unknown (question).
- Worth-it: full, half, none.
- UI: chevron, collapse/expand, settings gear, close, edit (pencil),
  click-through (cursor with slash), external link, character, refresh,
  manual tick.

The six reward icons added to the list above (all elemental resistances,
attribute, charm, flask, defences, other permanent stat) were delivered on
2026-10-25 with two extras (`ui/check.svg`, `ui/warning.svg`);
[`icons/manifest.json`](icons/manifest.json) lists all 46 files, and the
table in `icons/README.md` predates them. Which file serves which icon id is
in [Export contract](#export-contract).

## Tokens to define as Figma variables

Name each variable after its token key in [Export contract](#export-contract).

- **Colours:** panel background, panel border, text primary/secondary/muted,
  edit-mode accent, one colour per status, worth-it gold, four resistance
  colours, toast background. Check text contrast at least 4.5:1 against the
  panel background.
- **Type:** one free font with a licence that allows bundling (e.g. from
  Google Fonts; a variable font is fine); sizes for title, row name, step
  text, caption; optionally the weights for title and row.
- **Spacing:** 4 px grid; row padding, section gap, panel padding.
- **Radius and border width**, and the default window opacity.
- **Optional, when designed:** panel and toast shadows, panel and header
  gradients, row and button radii, and one motion duration and easing.

## Export contract

These names are fixed: the harvest, the Rust crates, the shell and the UI
all read them from here. Extras beyond them (more icons, frames or tokens)
are welcome; the harvest lists them. The Figma file is harvested once, the
moment the comps exist, and nothing after the harvest reads Figma.

Icon ids: reward-passive, reward-weapon-set, reward-spirit, reward-res-fire, reward-res-cold, reward-res-lightning, reward-res-chaos, reward-res-all, reward-life, reward-mana, reward-attribute, reward-charm, reward-flask, reward-defences, reward-other, reward-skill-gem, reward-support-gem, reward-ascendancy, reward-item, reward-unlock, reward-story, status-locked, status-available, status-in-progress, status-done, status-skipped, status-unknown, worth-yes, worth-optional, worth-no, manual-tick, ui-chevron, ui-collapse, ui-expand, ui-settings, ui-close, ui-edit, ui-click-through, ui-external-link, ui-character, ui-refresh

Frame ids: panel-campaign, panel-next-up, collapsed, quest-detail, character-picker, settings, mode-play, mode-edit, empty-no-log, empty-waiting, empty-no-character, empty-act-done, toast

Token keys: color.panel-bg, color.panel-border, color.text-primary, color.text-secondary, color.text-muted, color.edit-accent, color.status-locked, color.status-available, color.status-in-progress, color.status-done, color.status-skipped, color.status-unknown, color.worth-gold, color.res-fire, color.res-cold, color.res-lightning, color.res-chaos, color.toast-bg, font.family, font.file, size.title, size.row, size.step, size.caption, space.unit, space.row-pad, space.section-gap, space.panel-pad, radius, border-width, opacity.default

Optional token keys: shadow.panel, shadow.toast, gradient.panel, gradient.header, radius.row, radius.button, font.weight-title, font.weight-row, motion.duration, motion.easing

Icon sources: each icon id and the owner's delivered file (under
`docs/quest-tracker/icons/`) the harvest copies to it. A Figma version of an
icon, when the owner makes one, replaces the file at the harvest. Still to
draw: none (the six once listed here were delivered on 2026-10-25).

| Icon id | Owner's file |
|---|---|
| reward-passive | `reward/passive-point.svg` |
| reward-weapon-set | `reward/weapon-set-point.svg` |
| reward-spirit | `reward/spirit.svg` |
| reward-res-fire | `reward/resistance-fire.svg` |
| reward-res-cold | `reward/resistance-cold.svg` |
| reward-res-lightning | `reward/resistance-lightning.svg` |
| reward-res-chaos | `reward/resistance-chaos.svg` |
| reward-res-all | `reward/resistance-all.svg` |
| reward-life | `reward/life.svg` |
| reward-mana | `reward/mana.svg` |
| reward-attribute | `reward/attribute.svg` |
| reward-charm | `reward/charm.svg` |
| reward-flask | `reward/flask.svg` |
| reward-defences | `reward/defences.svg` |
| reward-other | `reward/other.svg` |
| reward-skill-gem | `reward/skill-gem.svg` |
| reward-support-gem | `reward/support-gem.svg` |
| reward-ascendancy | `reward/ascendancy.svg` |
| reward-item | `reward/item.svg` |
| reward-unlock | `reward/unlock.svg` |
| reward-story | `reward/story-only.svg` |
| status-locked | `status/locked.svg` |
| status-available | `status/available.svg` |
| status-in-progress | `status/in-progress.svg` |
| status-done | `status/done.svg` |
| status-skipped | `status/skipped.svg` |
| status-unknown | `status/unknown.svg` |
| worth-yes | `worth/full.svg` |
| worth-optional | `worth/half.svg` |
| worth-no | `worth/none.svg` |
| manual-tick | `ui/manual-tick.svg` |
| ui-chevron | `ui/chevron-right.svg` |
| ui-collapse | `ui/collapse.svg` |
| ui-expand | `ui/expand.svg` |
| ui-settings | `ui/settings.svg` |
| ui-close | `ui/close.svg` |
| ui-edit | `ui/edit.svg` |
| ui-click-through | `ui/click-through.svg` |
| ui-external-link | `ui/external-link.svg` |
| ui-character | `ui/character.svg` |
| ui-refresh | `ui/refresh.svg` |

Extras in the delivery, kept and listed at the harvest: `ui/chevron-down.svg`,
`ui/chevron-up.svg`, `ui/chevron-left.svg`, `ui/check.svg`,
`ui/warning.svg`.

Files the harvest writes:

- `apps/quest-tracker/theme/theme.json` — a flat object, token key to
  value: colours as any CSS colour (`#rrggbb`, or with alpha), sizes in px
  at 1×, `font.file` relative to `theme/`, `opacity.default` from 0 to 1,
  optional keys only when designed.
- `apps/quest-tracker/theme/tokens.css` — `:root` with one custom property
  per token key, the dot replaced by a dash (e.g. `--color-panel-bg:`), plus
  the `@font-face` rule for the bundled font, with no remote URL.
- `apps/quest-tracker/theme/icons/<id>.svg` — one per icon id, `currentColor`
  where single-colour, no external reference, script or raster.
- `apps/quest-tracker/theme/fonts/` — the font as WOFF2 (variable allowed)
  and its licence text.
- `docs/quest-tracker/design/frames/<frame id>.png` and
  `docs/quest-tracker/design/components/<Component>/<variant>.png` —
  reference screenshots.
- `docs/quest-tracker/design/reference/<frame id>.tsx` — design-to-code
  output, reference only, never imported.
- `docs/quest-tracker/design/component-spec.md` — per component: size,
  padding, gaps, states, tokens used.
- `docs/quest-tracker/design/harvest.md` — file URL, node ids, date, tools
  and versions, extras, anything not harvested.
- `docs/quest-tracker/design/review-comps.md` — the design review of the
  comps (impeccable and taste-skill), its findings and the owner's
  decisions; later `review-finish.md` for the first styled build.

## How the code will consume it

1. Build the file with **components, variants and variables** named as in
   the tables above and in [Export contract](#export-contract) (e.g.
   `QuestRow/status=available,worth=yes`), with auto layout, so
   `get_design_context` returns usable structure.
2. Send the file URL (and node URLs of the frames) to the session; it runs
   the design-to-code workflow once and writes the files listed in
   [Export contract](#export-contract): tokens to `theme.json` and
   `tokens.css`, the frames and components as reference screenshots.
3. Icons are taken as SVG into `apps/quest-tracker/theme/icons/<id>.svg`,
   from Figma or from the owner's files in `icons/`.

Until the Figma file exists, the overlay runs with a plain functional UI
built from the same components, so the parser and quest data can be tested
in-game without waiting on the design.
