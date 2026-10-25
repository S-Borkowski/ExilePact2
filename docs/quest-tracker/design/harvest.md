# Figma harvest

Harvested once, on 2026-10-25. Nothing after this reads Figma.

- **File:** https://www.figma.com/design/OCm0ix5nud2HU18LNYKf3g
  ("ExilePact2 – Quest Tracker"), pages Foundations `0:1`, Icons `2:80`,
  Components `2:81`, Frames `2:82`.
- **Tools:** Claude Code with the claude.ai Figma MCP (`get_screenshot`,
  `download_assets`, `get_design_context`, `use_figma` for variables and
  bounds), impeccable 4.7.1 and taste-skill 1.0.0 for the review, Pillow for
  cropping, fontTools for WOFF2 conversion.

## What was written

| Output | Source |
|---|---|
| `apps/quest-tracker/theme/theme.json` | Figma variables (74 + 3 added in review) and text styles, mapped to the contract's token keys; extras listed below |
| `apps/quest-tracker/theme/tokens.css` | Generated from `theme.json`, plus `@font-face` for the five bundled faces (local URLs only) |
| `apps/quest-tracker/theme/icons/*.svg` | The 41 contract ids copied from `docs/quest-tracker/icons/` per the brief's `Icon sources:` table (the Figma icons are imports of the same files), title ids made unique; 5 extras |
| `apps/quest-tracker/theme/fonts/` | Alegreya Sans Regular, Medium, Bold, ExtraBold and Alegreya Sans SC Bold, TTF from google/fonts (`ofl/alegreyasans`, `ofl/alegreyasanssc`) converted to WOFF2; `OFL.txt` |
| `docs/quest-tracker/design/frames/` | One PNG per frame id at 1× over the bright backdrop, `--dark` twins over the dark one; panel-level ids cropped from the "modes" and "empty-states" frames |
| `docs/quest-tracker/design/components/` | Each component set exported at 2× and cropped per variant (143 PNGs) |
| `docs/quest-tracker/design/reference/` | `get_design_context` output per frame, reference only |
| `component-spec.md`, `review-comps.md` | Written from the file and the five review rounds |

## Frame ids and nodes

| Frame id | Image | Node (bright / dark) |
|---|---|---|
| panel-campaign | full frame | `10:2` / `10:4`, panel `11:2` |
| panel-next-up | full frame | `10:6` / `10:8`, panel `11:441` |
| collapsed | full frame | `10:10` / `10:12`, badges `11:651` + idle + warning |
| quest-detail | full frame | `10:14` / `10:16`, panel `12:663` |
| character-picker | full frame | `10:18` / `10:20`, panel `12:796` |
| settings | full frame | `10:22` / `10:24`, panel `12:881` |
| mode-play | crop of "modes" | panel `32:2901` in `10:26` / `32:3075` in `10:28` |
| mode-edit | crop of "modes" (with the Undo toast) | panel `32:2920` / `32:3094` |
| empty-no-log | crop of "empty-states" | `13:1169` in `10:30` / `14:1529` in `10:32` |
| empty-waiting | crop | `13:1203` / `14:1539` |
| empty-no-character | crop | `13:1233` / `14:1547` |
| empty-act-done | crop | `13:1266` / `14:1557` |
| toast | full frame | `10:34` / `10:36` |

Extra frames: `icon-key` (`28:2747` / `28:2923`), `log-idle`,
`empty-no-log-play`, `empty-waiting-play`, `empty-no-character-play`, and
the whole `modes` and `empty-states` boards.

Component sets: StatusMark `5:33`, WorthMarker `5:47`, ManualTick `5:48`,
RewardChip `5:228`, StepLine `6:147`, Button `6:184`, Toggle `6:203`,
Slider `6:220`, Dropdown `6:241`, CollapsedBadge `6:252`, Toast `6:271`,
IconButton `7:181`, ViewSwitch `7:192`, PanelHeader `7:224`, ActHeader
`7:261`, QuestRow `7:1300`, CharacterRow `10:944`, SettingRow `10:964`.

## Extras beyond the contract

- **Icons:** `ui-chevron-down`, `ui-chevron-up`, `ui-chevron-left`,
  `ui-check` (step-done toast, undo), `ui-warning` (errors, log idle,
  warning badge).
- **Tokens:** surface, action, edit-tint, row and selection colours; the
  caps font and per-weight font files; line heights; component sizes; bar
  widths; chamfer sizes; `opacity.min`, `opacity.stale-rows`; toast, undo
  and confirm durations.
- **Variants:** PanelHeader `mode=log-idle`; CollapsedBadge
  `state=warning`; Toast `type=undo`; Button `variant=danger,state=confirm`.

**Note, 2026-10-25, after Live 1.** This record is left as harvested. The
owner replaced toasts with the row's own transition after Live 1, so the
toast parts were removed from the app: the `toast` frame id and the
`color.toast-bg` token key left the brief's export contract, and
`color.toast-bg`, `shadow.toast`, `size.toast-width`, `size.toast-height`,
`bar.toast`, `chamfer.toast`, `motion.toast-ms` and `motion.undo-ms` left
`theme.json` and `tokens.css`. No UI uses the Toast component or its
`type=undo` variant. `ui-check` stays, for the tracked character and the
`ok` status line. The `toast` frame images and the Toast variant images
remain under `frames/` and `components/Toast/` as the record of what the
comps held.

## Not harvested, and why

- **Backdrops:** the bright desert and dark crypt images are placeholders
  generated with Figma AI to test legibility; they are in the frame
  screenshots only and not bundled.
- **Icons from Figma:** not re-exported. The Figma icons are imports of the
  repository's SVGs, so the repository files are the masters.
- **PNG icon scales:** not needed under Tauri (the contract asks for SVG
  only).
- **Hover, focus and interaction flows** that the review left open (see
  `review-comps.md`): to be judged on the real build in `review-finish.md`.
- **Reference code:** all 15 pulls succeeded (the 13 frame ids plus
  `icon-key` and `log-idle`, 224 KB of TSX). Their asset URLs point at
  Figma's short-lived MCP asset store and are already dead links; use
  `theme/icons/` instead. The edit-mode header's empty class/level slot
  holds a zero-width space (U+200B); the real header leaves that slot out.
