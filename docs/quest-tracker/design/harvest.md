# Figma harvest

## Exile adoption, 2026-10-31

The original harvest below is retained as the **Claude** theme. The default **Exile** theme pairs QT-local `theme/exile.json` and `theme/exile.css` manually; no theme generator exists. Its selectors are scoped to the received theme. The final rich materials use three new original raster assets below. The earlier outline-only SVG approximation was rejected during rendered review and is not shipped. The concept PNG remains documentation only. Optional per-quest illustration niches were omitted because there is no authoritative art mapping. Rich quest names use system Georgia with a serif fallback for title-case cream lettering; Cinzel remains the engraved display face and Alegreya Sans remains prose. No Georgia font file is redistributed.

Cinzel SemiBold is bundled unmodified as `fonts/Cinzel-SemiBold.woff2`: Fontsource's [complete family ZIP](https://api.fontsource.org/v1/download/cinzel), fetched2026-10-31 from the [family download page](https://fontsource.org/fonts/cinzel/use), entry `static/cinzel-latin-600-normal.woff2`. Family ZIP companion TTF name table reports **Cinzel SemiBold, Version2.000**; OS/2 weight600; bundled file begins `wOF2`, length15204bytes, SHA256 `61efc27ceb7d08bb841652fa8979c0b6833b92548b7f9544c07968a6c02f28e4`. Latin coverage is the source subset; other characters use the serif fallback.

The ZIP's original full license ships as `fonts/Cinzel-OFL.txt`: copyright2020 The Cinzel Project Authors, SIL Open Font License1.1, matching [upstream OFL](https://raw.githubusercontent.com/google/fonts/main/ofl/cinzel/OFL.txt). Existing Alegreya fonts/license remain. Cinzel/font-license copies are byte-identical in all three app font directories for the shared-resource contract; sibling apps continue selecting their original fonts and palette. Exile artwork/token overrides stay Quest-Tracker-local.

## Original production material assets

Generated 2026-10-31 UTC with the built-in image-generation tool from this project's original AI concept as a material/sculpture reference. No game pixels, screenshots, extracted assets, letters or controls are baked in. The tool exposed no model identifier. Selected outputs were copied byte-for-byte, without pixel edits, resizing or conversion.

| Asset under `apps/quest-tracker/theme/materials/raster/` | Dimensions / mode | Bytes | SHA-256 |
|---|---|---:|---|
| `journal-frame.png` | 1254×1254 RGBA | 1237083 | `0745d20818040cf3b5796acab4d54de981b38ddf53b47a611437992dae57a3c2` |
| `lantern-astrolabe-crest.png` | 1983×793 RGBA | 1811176 | `1ff2e55ca20f94f2614b929543f7148f3a997261e1d51660974ebcae3ca443d6` |
| `charcoal-stone-well.png` | 1254×1254 RGB | 3736261 | `72cdd6a41d7f5f7ea89dc76de917407c3533f12df8733cddc4e41bee6f7cb225` |

Frame and crest have genuine alpha spanning 0–255; frame center is fully transparent. The frame uses nine-slice 208/190/208/190 with 34px/30px rails, horizontal stretch and vertical rounding, preserving niche proportions. Short windows reduce the overlay rails to 28px top, 12px sides and 14px bottom; ordinary Settings retains 20px/18px rails. Content scrolls in a separately inset well; rail art never paints over it. The owner removed the separate Campaign journal/crest header on 2026-11-01. Its crest PNG and exact prompt remain here as source provenance, but the current overlay does not import or render it. The frame and stone texture remain runtime art; no replacement art was generated. Stone is subdued by a dark CSS layer; tile seamlessness was requested, not mathematically established. All artwork is decorative, unfocusable and takes no pointer input.

Exact generation prompts:

### Frame prompt

```text
Use case: stylized-concept. Asset type: production nine-slice UI border PNG, not a mockup.
Use the attached ORIGINAL AI concept only as a material and sculpture reference, especially the RIGHT journal window. Create a new original front-facing square rectangular gothic UI surround, 1024 by 1024 pixels, real transparent alpha outside AND throughout the large empty central rectangular opening. Center must be totally transparent. No text, letters, icons, controls, interface content, landscapes or background.
The border should occupy about 9 percent of the canvas on each side: straight horizontal and vertical rails joining four architectural corner caps, with a large rectangular empty opening about 80 percent canvas width and height. Keep long rails geometrically straight so CSS nine-slice can stretch the central segments. No perspective.
Match the reference's tactile sculptural depth: chipped dark charcoal carved masonry, weathered black steel interior rail, worn warm antique-gold inlay and restrained filigree, gothic stone ribs, small carved recesses and layered bevels. Richly rendered original physical material, photorealistic 3D relief with raking warm light and deep recess shadows. Four substantial corners and continuous edges; ornament stays entirely inside image bounds. The rails can have shallow carved vertical niche details but must remain narrow. Old stone must be richly textured and nearly black, gold worn and warm, no glossy chrome, no flat vector strokes, no modern sci-fi outline. Uniform dimensional detail around all four sides. Transparent isolated object, no drop-shadow rectangle or checkerboard painted into the image.
```

### Crest prompt

```text
Use case: stylized-concept. Asset type: production UI decorative header crest transparent PNG, not a mockup.
Use the attached ORIGINAL AI concept as material reference, especially its LEFT lantern in an astrolabe ring and architectural top gables. Create a new original centered ornamental crest for a dark gothic journal UI. Transparent alpha everywhere outside the single isolated object. No lettering, UI controls, cards, scene, background, landscape, people or logos.
A warmly glowing amber lantern hangs inside a beautifully etched worn antique-gold astrolabe/compass spoked ring, seated above a low broad black carved-stone architectural title-plate crown. Subtle Gothic stone gables/niches, deeply carved weathered charcoal stone, black steel inner trim and worn gold inlay. Broad horizontal footprint with central lantern-ring rising modestly above, approximately 2.5 to 1 width-to-height object proportions, fully inside image bounds with small transparent margin. It will be displayed around 160 by 65 CSS pixels, so the central lantern and ring must be clean and readable with no hairline-only structure.
Highly tactile sculptural 3D relief, aged black stone pores, chipped ridges, hammered gold, restrained warm rim lighting and deep recessed shadows, matching the lush original concept's physical material depth. Not a flat vector logo, not a modern minimalist outline, no large opaque rectangular backdrop. Keep the lower title-plate crown narrow so live HTML text can be placed below separately.
```

### Stone prompt

```text
Use case: stylized-concept. Asset type: seamless repeating dark stone UI well texture, 1024 square, opaque.
Use the attached original AI journal concept as a MATERIAL reference only: the rich near-black rough carved stone in the journal/card recesses. Generate a subtle seamless/tileable charcoal-black weathered masonry surface for live text to sit over. Straight-on orthographic material sample only. Low-contrast fine pores, faint erosion and tiny chisel marks, black mineral flecks and restrained subdued warm edge glints. Overall nearly uniform dark charcoal (#141414 to #242321), no large cracks, no borders, no ornaments, no writing, no symbols, no scenery, no objects, no bright highlights, no vignettes, no directional gradient, no perspective. Physical tactile aged stone at an appropriate small scale, quiet enough for legible ivory text, not a flat vector fill and not noisy high-contrast gravel. Opposite edges must join without visible seams.
```

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
| `apps/quest-tracker/theme/tokens.css` | Original values paired manually with `theme.json`, plus `@font-face` for the five original bundled faces (local URLs only); there is no generator |
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

**Note, 2026-10-25, after Live 2.** `opacity.min` changed from 0.3 to 0 in
`theme.json` and `tokens.css`, by the owner's decision after Live 2: the
background opacity slider now runs from 0%, where the panel's background is
gone and its text, icons, border and edit-mode accent stay fully opaque.
The Settings frame's harvested reference (`reference/settings.tsx`) and
image stay as the record of the comps, with the old "Floor 30%" helper text,
the Dock side row and a single Scale choice; the app now has an Auto scale
switch plus a scale slider and no dock setting. No other token changed: the row
highlight added after Live 2 uses the existing status colours.

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
