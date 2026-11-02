# Crafting Helper — requested art

The Crafting Helper's Exile theme is drawn from the Quest Tracker's
steel/masonry kit (`docs/quest-tracker/design/steel-masonry-kit/`): its
neutral steel frame, its charcoal masonry, its palette and its control
geometry. The elements below have no counterpart in the kit. Each is drawn
today by a marked CSS placeholder in the kit palette
(`[data-art-placeholder="<id>"]` in `apps/crafting-helper/src/steel.css`),
so the app ships complete; this file asks for the real art.

**Who makes it.** The owner generates each file outside this repository,
from the entry alone: every entry names the exact size, slicing, format,
states, palette, material, safe area and reference images it needs. Nothing
in this workorder drew, cropped, recoloured or extracted art.

**Where it goes.** Save each file under
`apps/crafting-helper/theme/materials/requested/` with the file name the
entry gives, set that entry's `status` to `delivered` in
`apps/crafting-helper/theme/materials/requested/manifest.json`, and rebuild.
No code changes: `src/requested-art.ts` finds the files through
`import.meta.glob`, sets one `--art-<file stem>` URL property per file, and,
once every file of an entry is there, marks the document `data-art-<id>`;
the Exile rules after each placeholder in `src/steel.css` then draw the file
in its place.

**How it is checked.** Each entry's Acceptance line: the dimensions and
alpha, read with any image tool, then `npm --prefix apps/crafting-helper
run corner-safety`, which must still report 0 failures (no content over
frame art at any size, scale, text size or scroll position; readability
floors; a flush frame), and `npm --prefix apps/crafting-helper test`, whose
`src/steel-kit.test.ts` holds the manifest, the placeholders and the
loader together.

The palette is `docs/quest-tracker/design/steel-masonry-kit/palette.json`:
well `#090D10`, steel `#8D979E`, edge `#B8C0C5`, muted `#A9AFB3`, text
`#E4E6E7`, gold `#D0BF94`, ivory `#E6E5DE`, control `#424D54`, hover
`#536169`, line `#5D686F`, disabled `#535D64`, error `#D0A3A0`, warn
`#CAB990`. Gold is for titles and short headings only; controls are steel
grey, never gold or cyan.

## ch-radio — radio-off.svg, radio-on.svg, radio-focus.svg, radio-disabled.svg

Used in: the main window, Find a base on trade panel (`trade/TradePanel.svelte`), the Match group's two radio buttons ("Both slots", "At least one"): the player picks whether the trade search needs a prefix and a suffix or any one pick. The kit has a checkbox and a toggle but no radio.
Size: 16 x 16 px at scale 1, drawn at that CSS size; scales 0.75, 1, 1.33, 2: an SVG on a 16 x 16 viewBox whose ring and dot stay crisp at 12, 16, 21 and 32 device px (strokes on whole or half pixels, no hairlines under 1 px at 0.75).
Slicing: none (drawn whole, `background-size: 16px 16px`).
Format: SVG, transparent outside the ring (alpha 0 in the four corners), no embedded fonts or raster images.
States: normal off (`radio-off.svg`), selected on (`radio-on.svg`), keyboard focus (`radio-focus.svg`, on or off alike), disabled (`radio-disabled.svg`); hover reuses `radio-off.svg`.
Palette: ring steel `#8D979E` on a `#151E24` face (the kit checkbox's), selected dot ivory `#E6E5DE`, focus ring ivory `#E6E5DE` 1 px outside the steel ring, disabled ring and dot `#535D64`.
Material: steel; light from the top; edges: a single crisp 1 px ring, no bevel, no glow.
Safe area: the whole 16 x 16 is the control; nothing else sits inside it. The label starts 8 px to its right; never paint outside the 16 x 16 box.
References: `docs/quest-tracker/design/steel-masonry-kit/components/checkbox-off.svg` and `docs/quest-tracker/design/steel-masonry-kit/components/checkbox-on.svg` (match the face, edge and tick colours with a round ring and dot in place of the square and tick), `docs/quest-tracker/design/steel-masonry-kit/components/checkbox-focus.svg` (the focus ring), `docs/quest-tracker/design/steel-masonry-kit/previews/components.png` (the "Controls" row: checkbox and toggle states, to sit beside them).
Current: `docs/crafting-helper/design/reference/ch-radio.png` (today's element, from the base tag)
Placeholder: a 16 x 16 CSS circle, 1 px steel `#8D979E` edge on `#151E24`, an ivory `#E6E5DE` 7 px dot when selected.
Wiring: `apps/crafting-helper/theme/materials/requested/radio-off.svg`, `apps/crafting-helper/theme/materials/requested/radio-on.svg`, `apps/crafting-helper/theme/materials/requested/radio-focus.svg`, `apps/crafting-helper/theme/materials/requested/radio-disabled.svg`; manifest entry `ch-radio` in `apps/crafting-helper/theme/materials/requested/manifest.json`
Acceptance: each file a 16 x 16 viewBox SVG; corner alpha 0; all four present before the entry is marked; `npm --prefix apps/crafting-helper run corner-safety` reports 0 failures and the trade cases' readability passes.

## ch-overlay-frame — frame-overlay-compact.png

Used in: the overlay window (`overlay/Overlay.svelte`), the step panel's border: a click-through 480 x 168 window over the game that shows the guided craft's current step in at most seven one-line rows. The kit's frame has 48 px corner knots that do not fit a 168 px window with seven rows.
Size: drawn as a nine-slice over the panel's box, 480 px wide and 140-164 px tall at scale 1 (it follows its rows); source PNG 960 x 336 px (2x), so it holds at scales 0.75, 1, 1.33, 2 without visible softening at 2.
Slicing: nine-slice, 24 source px on each side (drawn 12 px at scale 1), edges stretched, centre not drawn.
Format: PNG RGBA; the centre (inside 24 source px of every edge) alpha 0; art alpha 255 within the band, a 1-2 source px anti-aliased outer edge.
States: single state.
Palette: steel `#8D979E` rails with an edge highlight `#B8C0C5` on the lit side and a line `#5D686F` shadow, the well `#090D10` behind; no gold.
Material: steel; light from the top left; edges: thin forged rails like the kit frame's, with small square corner plates instead of knots.
Safe area: text starts 8 px from the top and bottom and 12 px from the left and right of the panel at scale 1; the art must stay within the outer 6 px (12 source px) of every edge and never paint further in. The masonry well shows through the centre.
References: `docs/quest-tracker/design/steel-masonry-kit/assets/frame-neutral-steel.png` (the rail's material and colour, at a far smaller corner), `docs/quest-tracker/design/steel-masonry-kit/previews/overlay-settings.png` (how the frame sits on a panel over the game), `docs/quest-tracker/design/steel-masonry-kit/components/collapsed-badge.svg` (a compact steel edge at small size), `docs/quest-tracker/design/frames/steel/play-default.png` (the rails' weight against the well).
Current: `docs/crafting-helper/design/reference/ch-overlay-frame.png` (today's element, from the base tag)
Placeholder: a 3 px double steel border (`#8D979E`) around the panel, over the masonry well.
Wiring: `apps/crafting-helper/theme/materials/requested/frame-overlay-compact.png`; manifest entry `ch-overlay-frame` in `apps/crafting-helper/theme/materials/requested/manifest.json`
Acceptance: 960 x 336 px RGBA; alpha 0 at every pixel more than 12 source px from an edge; `npm --prefix apps/crafting-helper run corner-safety` reports 0 failures and the overlay's readability passes over both scenes.

## ch-item-card — item-card-frame.png

Used in: the main window's craft area (`craft/ItemPanel.svelte`), the card around the grabbed item: its name, base, item level, prefixes and suffixes with their tiers, and the grabbed text below. It shows while planning and while guiding.
Size: drawn as a nine-slice on the card's box, about 300-430 px wide and 150-400 px tall at scale 1 (382 x 204 at the default window); source PNG 512 x 512 px, so it holds at scales 0.75, 1, 1.33, 2.
Slicing: nine-slice, 32 source px on each side (drawn 16 px at scale 1), edges stretched, centre not drawn.
Format: PNG RGBA; the centre alpha 0 (the masonry well shows through); art alpha 255 in the border band with a 1-2 px anti-aliased edge.
States: single state.
Palette: steel `#8D979E` edge, highlight `#B8C0C5`, line `#5D686F`, a `#121B20` inner shadow at most; no gold.
Material: steel; light from the top; edges: a plain forged border with small corner plates, quieter than the window frame.
Safe area: the card's text sits 10 px from its top and bottom and 12 px from its sides at scale 1; the art must stay within the outer 8 px (16 source px) of every edge.
References: `docs/quest-tracker/design/steel-masonry-kit/assets/frame-neutral-steel.png` (material), `docs/quest-tracker/design/steel-masonry-kit/previews/detail.png` (the quest detail's reading block: the weight a card within a window should have), `docs/quest-tracker/design/steel-masonry-kit/components/message-info.svg` (the slate panel colour behind text).
Current: `docs/crafting-helper/design/reference/ch-item-card.png` (today's element, from the base tag)
Placeholder: a `#121B20` card with a 1 px line `#5D686F` edge and 10 / 12 px padding.
Wiring: `apps/crafting-helper/theme/materials/requested/item-card-frame.png`; manifest entry `ch-item-card` in `apps/crafting-helper/theme/materials/requested/manifest.json`
Acceptance: 512 x 512 px RGBA; centre alpha 0 (every pixel more than 32 source px from an edge); `npm --prefix apps/crafting-helper run corner-safety` reports 0 failures.

## ch-step-marker — step-marker.svg

Used in: the main window's route preview (`craft/RoutePreview.svelte`, `ol.steps`), the plate behind each step's number (1, 2, 3 ...) at the start of every step of the chosen route, while planning.
Size: 22 x 22 px at scale 1, growing with the number's text (at least 22 x 22); scales 0.75, 1, 1.33, 2: an SVG that stays crisp from 17 to 44 device px.
Slicing: none (drawn whole, stretched to the plate's box).
Format: SVG, transparent outside the plate, no embedded fonts or text (the number is live text drawn over it).
States: single state.
Palette: face `#1E272D`, edge steel `#8D979E`, a fine top highlight `#74818A`; the number over it is ivory `#E4E6E7`.
Material: steel; light from the top; edges: a square plate with a 1 px edge and the kit button's top highlight, no bevel.
Safe area: the number sits centred in the inner 16 x 16 px; never paint anything in that area but the flat face.
References: `docs/quest-tracker/design/steel-masonry-kit/components/button-default.svg` (face, edge and top highlight), `docs/quest-tracker/design/steel-masonry-kit/components/keycap.svg` (a small plate around text).
Current: `docs/crafting-helper/design/reference/ch-step-marker.png` (today's element, from the base tag)
Placeholder: a 22 x 22 CSS square, `#1E272D` face, 1 px steel `#8D979E` edge, the number in ivory.
Wiring: `apps/crafting-helper/theme/materials/requested/step-marker.svg`; manifest entry `ch-step-marker` in `apps/crafting-helper/theme/materials/requested/manifest.json`
Acceptance: a 22 x 22 viewBox SVG; alpha 0 outside the plate; `npm --prefix apps/crafting-helper run corner-safety` reports 0 failures and the step numbers' contrast passes.

## ch-update-dot — update-dot.svg

Used in: the main window's header (`App.svelte`), inside the Settings button: the mark beside "Update 1.3.0" when a check found a newer version. (The Waystone Roller asks for the same mark under its own path; one drawing may serve both.)
Size: 10 x 10 px at scale 1; scales 0.75, 1, 1.33, 2: an SVG that reads as a mark from 8 to 20 device px.
Slicing: none.
Format: SVG, transparent around the mark.
States: single state.
Palette: ivory `#E6E5DE` fill with a steel `#8D979E` edge; no gold, no green.
Material: gold accent is not used; a small steel-edged ivory jewel; light from the top; edges: crisp, no glow.
Safe area: the whole 10 x 10; the label's text starts 6 px to its right.
References: `docs/quest-tracker/design/steel-masonry-kit/components/status-in-progress.svg` (the ivory fill and steel edge of a small mark; not its diamond shape), `docs/quest-tracker/design/steel-masonry-kit/previews/components.png` (the Status diamonds row, for weight).
Current: `docs/crafting-helper/design/reference/ch-update-dot.png` (today's element, from the base tag)
Placeholder: an 8 x 8 ivory `#E6E5DE` square with a 1 px steel edge.
Wiring: `apps/crafting-helper/theme/materials/requested/update-dot.svg`; manifest entry `ch-update-dot` in `apps/crafting-helper/theme/materials/requested/manifest.json`
Acceptance: a 10 x 10 viewBox SVG; alpha 0 in its corners; `npm --prefix apps/crafting-helper run corner-safety` reports 0 failures.
