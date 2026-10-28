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
- **Small and out of the way.** Placed at the right edge of the screen by
  default (below the minimap area), and dragged anywhere by its header in
  edit mode; it is not docked to an edge, and the place it is dragged to is
  kept (owner, 2026-10-25, after Live 2). Collapsed it must be no bigger
  than ~48×48 px; expanded, about 320 px wide and at most 60% of screen
  height.
- **Only over the game.** In play mode the overlay is visible only while
  the game or the overlay itself is the window in front; when another
  window comes to the front it hides, and it comes back, without taking
  focus from the game, when the game is in front again. In edit mode it is
  always visible (owner, 2026-10-25, after Live 2).
- **Scales.** Design at 1920×1080 (1×). It will be scaled to 1440p (1.33×)
  and 4K (2×) by the webview's zoom, or to any factor from 0.75× to 2× the
  player sets, so keep icons as vectors and sizes on a 4 px grid. The zoom
  scales the content only: the player sizes the window, by dragging the
  panel's edges and corners in edit mode, and the panel fills it (owner,
  2026-10-28, for 0.1.1). Lay the panel out to stretch in both directions,
  its body scrolling, down to the window's minimum of 320×240.
- **Sliders apply while dragged.** The opacity and scale sliders change the
  panel as the thumb moves, not only when it is let go (owner, 2026-10-25,
  after Live 3), so the panel's own look is the slider's preview.
- **A tray icon.** While the overlay runs it has an icon in the Windows
  notification area: the app's own icon (no icon drawn for it), tooltip
  "Quest Tracker", or "Quest Tracker (1.3.0 available)" while a newer
  version is known. Its menu, top to bottom: "Update available: 1.3.0"
  (only while a newer version is known; opens that release's page in the
  default browser), "Check for updates" (opens the Settings window and
  starts a check), "Settings" (opens the
  [Settings window](#the-settings-window)), a separator, and "Quit Quest
  Tracker", which closes the overlay even while it is hidden (owner,
  2026-10-25, after Live 3; the update entries and Settings, owner,
  2026-10-27). Nothing in the panel changes for it: an update is never
  announced inside the overlay.
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

"Probably done" (owner, 2026-10-25, after launch 6): a quest or a step
shown done because of a later event the log recorded, not its own signal,
carries a small mark beside its status mark (the row) or its step mark (the
detail). A quest gets it when its data says its done is such a later event
(for example a quest done on entering the next area) and the player did not
mark it done; a step gets it when it shows done but its own signal never
fired and the player did not tick it (a step a later step implies, or the
"Use ..." step of a boss quest finished by its reward line). The mark
differs from the done mark in shape, not only in colour; its tooltip, and
the same words as its accessible text, say it was inferred from a later
step the log recorded. It is not a control: it adds no toggle and no tick
action, and the manual-tick marker does not change. No new icon id or frame
id; a token for it, if the design needs one, goes under `Optional token
keys`.

A find-it tip (owner, 2026-10-25, after Live 3): a step may carry the
owner's short "how to find it" text, one line in the quest data (for
example "Follow up river."). The row shows the tip of the step it shows
next on its own line under that step, in play and edit mode, in the
secondary text style, wrapping to at most two lines; the detail shows each
step's tip under its text. Most steps have none, and then nothing shows.
No new icon id, frame id or token key.

Skipped quests stay tracked (owner, 2026-10-25, after Live 3): the log
keeps moving a skipped quest on. Its row keeps the skipped status mark and
muted style, and, when "show skipped quests" lists it, shows the step its
progress has reached (and that step's tip) as an in-progress row would;
its detail shows the steps done and current by that progress. Unskipping
shows exactly that progress. A skipped quest's progress plays no row
transition and announces nothing. With "show skipped quests" on, the
"next up" view also lists a skipped quest that is available or in progress
by its progress, whatever "only worth-it" says, after the rows not skipped
of the same worth; with it off, "next up" lists no skipped quest. Neither
changes an act's counts.

Detection values, per quest: **auto** (the log records the quest's
completion), **partial** (the log records some of its unlocks or steps but
not the completion), **manual** (the log records nothing usable). The quest
row's manual variant covers both partial and manual; the manual-tick marker
shows on both.

Live changes: when a line the game writes while the overlay runs makes a
quest available, reaches one of its steps or completes it, the quest's own
row shows the change in place (the row transition, frame 9): the step just
completed, the step now current, and one sentence naming the quest and what
changed, for the screen reader. Lines read at start-up to catch up change
the rows without a transition. There are no toasts (owner, 2026-10-25,
after Live 1).

A new character: a level-1 entry to The Riverbank starts a new character
that the log does not name until its first level-up, unless it is a relog.
A relog inside The Riverbank enters it again at level 1; when the client
names a character the overlay already knows within a few seconds of that
entry, the entry is that character's relog and it stays the active one
(owner, 2026-10-25, after launch 6). Deciding which it is takes a
moment: for up to 5 s after the entry (the relog window; up to about 7.5 s
when the log goes quiet, as the guide says), the previous frame stays on
screen, with the character and rows it showed before, and nothing about a
new character is shown yet. Otherwise, once the window closes and until its
first level-up, the header shows the new character as "New character" at
level 1, with its Act 1 rows; it is not in the character picker's list, and
it survives a relaunch of the overlay.

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
   act's quests that are available or in progress, worth-it ones first,
   and with "show skipped quests" on the skipped ones that are too (Data
   the UI shows). This is the default view while playing.

In every view, an in-progress quest always shows its next step as a second
line under its name (owner, 2026-10-25), and that step's find-it tip, if it
has one, on a line under the step. A quest whose every step is done but
which is not done yet has no next step, and shows none (Live 9): only its
name, until it is done.
3. **Collapsed state** [`collapsed`]. The small button/badge, with a
   count of worth-it quests currently available. A live change of a row
   shows the same highlight on the badge (frame 9).
4. **Quest detail** [`quest-detail`]. What opens when a row is clicked: all
   steps with a tick per step, every reward, giver and zone, the "worth it"
   reason, the poe2db link, and the buttons Mark done / Skip / Reset.
5. **Character picker** [`character-picker`]. The overlay tracks progress
   per character; a list of detected characters (name, class, level), the
   active one highlighted, and "new character".
6. **Settings** [`settings`]. Background opacity slider from 0% to 100% in
   5% steps (it fades the panel background only; at 0% the background is
   gone, and text, icons, the panel border and the edit-mode accent stay
   fully opaque, so edit mode still reads as edit mode), scale as an "Auto" switch plus a slider for a fixed factor from
   0.75× to 2× in 0.05 steps (moving the slider turns Auto off), "show
   skipped quests" (a skipped quest shows even with "only worth-it" on),
   "only worth-it quests", a Hotkeys section, log file path with a status
   line, and a link to the icon key (owner, 2026-10-25). There is no
   dock-side setting: the panel is dragged in edit mode (owner, 2026-10-25,
   after Live 2). A "Read the screen" toggle, off by default, follows "only
   worth-it quests", with a helper line saying it reads the game's window
   for boss bars and quest dialogue and saves nothing (owner, 2026-10-26).
   Since 0.1.1 there is no click-through switch and no switch for hiding
   under game panels (owner, 2026-10-28): play mode is always
   click-through and edit mode always takes clicks, and the overlay always
   hides in play mode while a game panel is open under it.
7. **Edit mode vs. play mode** [`mode-play`, `mode-edit`]. The same panel in
   both. Play mode is click-through (mouse goes to the game) and shows no
   buttons; edit mode (hotkey) accepts clicks and shows the controls. The
   two must be distinguishable at a glance — e.g. a coloured frame or header
   tint in edit mode.
8. **Empty and error states.** No log found [`empty-no-log`]; log found but
   the game hasn't written anything yet [`empty-waiting`]; no character
   detected yet [`empty-no-character`]; all quests in this act done
   [`empty-act-done`].
9. **Row transition** (no frame id: it happens inside the quest row; owner,
   2026-10-25, after Live 1, in place of the toasts). When a quest becomes
   available, a step is reached or the quest is done, the row itself
   changes: the completed step fades out over about 150 ms, then the next
   step fades in from about 0.2 opacity with a 4-6 px upward move over
   about 220 ms, ease-out. A second change to the same row restarts the
   transition cleanly; nothing queues and nothing stacks. A quest started
   with no step reached yet makes the same change, with nothing going out
   and its first step coming in. Every change also highlights the row: its
   band or background is tinted with an existing token (the new status
   colour or `color.edit-accent`) for about 2.5 s and then clears; a second
   change restarts the hold. The highlight is colour or opacity only, never
   a flash (one per change, never more than three a second). With
   `prefers-reduced-motion: reduce` (Windows animations off) the step swap
   is instant and nothing moves, but the highlight still shows, so the
   change is seen with animations off (owner, 2026-10-25, after Live 2:
   Live 2's change went unseen on a machine with animations off). Every
   change is announced once in a polite live region (`aria-live="polite"`),
   one sentence naming the quest and what changed. In the `collapsed`
   frame the badge shows the highlight; a row that is not on screen (its
   act folded, or filtered out) is announced only. Mark done and Skip are
   undone from the quest detail (Unmark done, Unskip); there is no Undo
   toast.

Also designed, beyond the frame ids above: an icon key (every status, worth
and manual mark, opened from Settings or with F1), play-mode versions of the
empty states ("Press F8 to …"), and a log-idle header ("No log update ·
5 min", rows dimmed). Hotkeys, all rebindable in Settings and working in both
modes: F8 edit mode, F7 collapse/expand, F6 switch view, F1 icon key; Esc
closes detail, settings, the picker and the icon key (owner, 2026-10-25).

## Components (make each one a Figma component with variants)

| Component | Variants | Approx. size @1× | Notes |
|---|---|---|---|
| Panel header | play / edit | 320×36 | Title, active character, view switch, collapse button, settings button (edit only) |
| Act section header | expanded / collapsed; has-available / none | 320×28 | "Act 2", progress like `5/9`, chevron |
| Quest row | status × worth-it (yes/no/optional) × detection (auto/manual; manual also covers partial) | 320×40 | Status mark, name, short reward text (e.g. "Spirit", "Passive · Cold res"; owner, 2026-10-25), worth-it marker; current step as a second line when in progress, always shown (row grows to ~56) |
| Status mark | locked, available, in progress, done, skipped, unknown | 16×16 | Must differ in shape, not just colour |
| Worth-it marker | worth it / optional / not worth it | 16×16 | Solid star / outline star / dash |
| Manual-tick marker | — | 12×12 | Shows the row needs a manual tick (detection partial or manual); an eye with a slash after the quest name, explained in quest detail |
| Reward chip | one per reward category | 20×20 icon, optional label | Icon+label in quest detail only; rows show reward text instead |
| Step line | pending / current / done | 288×20 | Used in quest detail |
| Button | primary / secondary / danger; default / hover / pressed / disabled; danger also confirm | h 28 | Mark done, Skip, Reset. Reset asks twice: the first click relabels it "Confirm reset" for 3 s |
| Toggle, slider, dropdown | standard states | h 24 | Settings |
| Collapsed badge | idle / has-new (count) | 48×48 | Count of available worth-it quests |
| Row transition | step out / step in; highlight; reduced motion (instant swap, highlight kept) | the quest row | Not a component of its own: the quest row's step line and band. The completed step fades out (~150 ms), the next fades in from ~0.2 opacity rising 4-6 px (~220 ms, ease-out); the row is highlighted with an existing token for ~2.5 s, also under `prefers-reduced-motion: reduce`, where the swap is instant; the collapsed badge shows the highlight too; announced in a polite live region |

## The Settings window

The app's own settings, apart from the overlay (owner, 2026-10-27). It is
not an overlay frame: it is a small ordinary window of its own, opened from
the tray's "Settings" and "Check for updates", so the
[Export contract](#export-contract)'s frame ids do not change. The panel's
Settings frame stays as it is; nothing moves between the two and neither
links to the other.

- **Purpose.** For now it holds one setting, "Automatically check for
  updates", off by default, and the line "Created by ST4H". It is built to
  take more: its body is a list of sections drawn in order, so a later
  setting is a new section, not a new layout.
- **Size.** About 420 px wide at 1×, as tall as its content (328 px with
  one section, which fits its tallest state, a download in progress), not
  resizable, centred on the screen. A native,
  dark title bar ("Quest Tracker Settings") with the system's close
  button; it is in the taskbar while open and not always on top.
- **Layout, top to bottom.** The title "Settings" (Title style); then each
  section: its name as a group label (Act style, `color.text-muted`), its
  rows; sections are divided by a 1 px `color.panel-divider` line. A
  footer under a divider: "Created by ST4H" (Caption, `color.text-muted`),
  plain text, no link.
- **The Updates section.** A setting row "Automatically check for updates"
  with a toggle (on as the app says), and under it a helper line (Label
  size, not Caption: it is the privacy promise): it asks
  GitHub once a day whether a newer version is out and downloads nothing;
  off, Quest Tracker never connects unless you ask it to check. Then the
  check's status line (Step style, with a 16 px icon), the install's own
  line under it (the same style, only while an install has something to
  say), the download's progress bar while one runs, the buttons "Check
  now" (secondary) and the install part's one button, and "Version 1.2.0"
  (Caption).
- **Its states** are the update check's: *not checked yet* (no icon);
  *checking* ("Checking for updates…", the refresh icon, "Check now"
  disabled); *up to date* (the check icon); *available* ("Quest Tracker
  1.3.0 is available. You have 1.2.0.", the available mark in
  `color.status-available`); *no release published* (no icon); *failed*
  ("Could not check for updates: <reason>.", the warning icon in
  `color.warning`; a newer version found earlier is kept, and so is its
  Install button). The text says what happened in every state; the icon
  and its colour only repeat it.
- **Install, in the window.** A newer version is installed here, never
  handed to a browser: no state, button or link opens a release page. The
  install part's states, each written by the app:
  *ready*: the primary button "Install 1.3.0 and restart", disabled while
  a check runs; *unsigned*: the line "1.3.0 is available but cannot be
  installed: the release is not signed." (warning icon), no button and no
  *Try again*, since it would fail the same way; *downloading*:
  "Downloading Quest Tracker 1.3.0…" (refresh icon), a 4 px determinate
  bar (the slider's track, `color.panel-divider`, filled in
  `color.edit-accent`) with the amount in words beside it ("5.2 MB of
  12.0 MB", Caption), and *Cancel* (secondary), which goes back to
  *ready*; with no total known the bar is the bare track and the amount
  stands alone; *verifying*: "Checking the signature of Quest Tracker
  1.3.0…" (refresh icon); *restarting*: "Quest Tracker 1.3.0 is installed.
  Restarting…" (check icon); *failed*: "Could not install Quest Tracker
  1.3.0: <reason>." (warning icon in `color.warning`) and *Try again*
  (secondary), which runs the install again. "Check now" stays in every
  state but is disabled while a download, the signature check or the
  restart runs. Nothing moves: the bar steps with each view, and there is
  no animation to turn off under `prefers-reduced-motion`.
- **Tokens and components it reuses.** The panel's tokens
  (`color.panel-bg` as the window's background, the text colours, the type
  styles, the 4 px grid) and the styled layer's Toggle, Button, group label
  and Caption, unchanged. No new token, icon or component variant: the
  download's progress bar is the Slider's track without its thumb.
- **Keyboard and screen reader.** Tab moves from the toggle to "Check now"
  to the install part's button (Install, *Cancel* or *Try again*); Space
  flips the toggle, Enter or Space presses a button; focus shows as the
  edit accent's ring, as in the panel. The toggle is a `switch` named by
  its label and described by its helper line; each section is a region
  named by its heading; the check's status line and the install's line are
  each a polite live region, the install's present (empty) in every state
  so that its first words are read out when they arrive. The progress bar
  is a `progressbar` named by the install's line, with the amount in words
  as its value text. The window is closed by its title bar (or Alt+F4);
  Esc does nothing.
- **No remote load.** Like the overlay, the window is a page of the app's
  own bundle: fonts and icons are local, the content security policy blocks
  every remote load, and the page itself never calls the network. The
  check and the install are made by the app, not the page: a check only
  when asked or by the setting, an install only when its button or *Try
  again* is pressed.

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
- Worth-it: solid star, outline star, plain dash.
- UI: chevron, collapse/expand, settings gear, close, edit (pencil),
  click-through (cursor with slash), external link, character, refresh,
  manual tick.

The six reward icons added to the list above (all elemental resistances,
attribute, charm, flask, defences, other permanent stat) were delivered on
2026-10-25 with two extras (`ui/check.svg`, `ui/warning.svg`);
[`icons/manifest.json`](icons/manifest.json) lists all 46 files, and the
table in `icons/README.md` lists them. Which file serves which icon id is
in [Export contract](#export-contract).

## Tokens to define as Figma variables

Name each variable after its token key in [Export contract](#export-contract).

- **Colours:** panel background, panel border, text primary/secondary/muted,
  edit-mode accent, one colour per status, worth-it gold, four resistance
  colours. Check text contrast at least 4.5:1 against the
  panel background.
- **Type:** one free font with a licence that allows bundling (e.g. from
  Google Fonts; a variable font is fine); sizes for title, row name, step
  text, caption; optionally the weights for title and row.
- **Spacing:** 4 px grid; row padding, section gap, panel padding.
- **Radius and border width**, and the default window opacity.
- **Optional, when designed:** the panel shadow, panel and header
  gradients, row and button radii, and one motion duration and easing.

## Export contract

These names are fixed: the harvest, the Rust crates, the shell and the UI
all read them from here. Extras beyond them (more icons, frames or tokens)
are welcome; the harvest lists them. The Figma file is harvested once, the
moment the comps exist, and nothing after the harvest reads Figma.

Icon ids: reward-passive, reward-weapon-set, reward-spirit, reward-res-fire, reward-res-cold, reward-res-lightning, reward-res-chaos, reward-res-all, reward-life, reward-mana, reward-attribute, reward-charm, reward-flask, reward-defences, reward-other, reward-skill-gem, reward-support-gem, reward-ascendancy, reward-item, reward-unlock, reward-story, status-locked, status-available, status-in-progress, status-done, status-skipped, status-unknown, worth-yes, worth-optional, worth-no, manual-tick, ui-chevron, ui-collapse, ui-expand, ui-settings, ui-close, ui-edit, ui-click-through, ui-external-link, ui-character, ui-refresh

Frame ids: panel-campaign, panel-next-up, collapsed, quest-detail, character-picker, settings, mode-play, mode-edit, empty-no-log, empty-waiting, empty-no-character, empty-act-done

Token keys: color.panel-bg, color.panel-border, color.text-primary, color.text-secondary, color.text-muted, color.edit-accent, color.status-locked, color.status-available, color.status-in-progress, color.status-done, color.status-skipped, color.status-unknown, color.worth-gold, color.res-fire, color.res-cold, color.res-lightning, color.res-chaos, font.family, font.file, size.title, size.row, size.step, size.caption, space.unit, space.row-pad, space.section-gap, space.panel-pad, radius, border-width, opacity.default

Optional token keys: shadow.panel, gradient.panel, gradient.header, radius.row, radius.button, font.weight-title, font.weight-row, motion.duration, motion.easing

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
