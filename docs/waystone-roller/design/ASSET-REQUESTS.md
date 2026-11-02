# Waystone Roller — requested art

Waystone Roller's default look, Exile, is the Quest Tracker's steel/masonry
kit (`docs/quest-tracker/design/steel-masonry-kit/`): its neutral steel
frame, its charcoal masonry and its control treatments drawn as CSS. The
elements below have no counterpart in that kit. Until their art existed, the
Exile theme drew each one as a marked placeholder: a dashed outline in the
kit palette, `[data-art-placeholder="<id>"]` in
`apps/waystone-roller/src/steel.css`, which stays as the fallback for a
missing file. Nothing in this workorder generated, drew, cropped or
recoloured art; every file below was for the owner to make.

**Delivered.** The owner delivered all three files on 2026-11-02. They sit
in `apps/waystone-roller/theme/materials/requested/` exactly as delivered
(byte for byte; `.gitattributes` keeps their LF line endings, so the hashes
the workorder's criteria pin hold on a fresh checkout), every manifest entry
reads `delivered`, and the Exile theme draws the files in place of the
placeholders. Each entry below ends with its `Status:` line.

**Who makes it.** The owner, from this page alone: each entry says where
the element sits, its exact size, its states, its colours (hex values from
the kit's `palette.json`), its material and lighting, what must stay clear,
which reference images to match, and how it is checked.

**Where it goes.** Save each file under
`apps/waystone-roller/theme/materials/requested/` with exactly the name its
heading gives, then set its entry's `status` to `delivered` in
`apps/waystone-roller/theme/materials/requested/manifest.json`, and rebuild.
No code changes: `src/requested-art.ts` finds the file through
`import.meta.glob` and marks the page (`data-art-<id>` on the root element
and a `--art-<file name without extension>` URL property), and the Exile
rules in `src/steel.css` then draw the file in place of the placeholder.
The Claude theme never shows any of it.

**How it is checked.** The dimensions and the transparency the entry
states, then `npm --prefix apps/waystone-roller run corner-safety` (nothing
paints over the frame, readability, flush frame) and `npm --prefix
apps/waystone-roller test` (the loader and the placeholders) both pass.

Kit palette by role (`docs/quest-tracker/design/steel-masonry-kit/palette.json`):
well `#090D10`, steel `#8D979E`, edge `#B8C0C5`, muted `#A9AFB3`, text
`#E4E6E7`, gold `#D0BF94`, ivory `#E6E5DE`, control `#424D54`, hover
`#536169`, line `#5D686F`, disabled `#535D64`, error `#D0A3A0`, warn
`#CAB990`.

## wr-mark-want — mark-want.svg

Used in: the main window. (1) The mod list (`src/components/ModRow.svelte`,
`.choice-mark` of a row whose choice is Want): the mark in the 16 px column
left of the mod's text, telling the row is wanted by shape, not only by
colour. (2) The footer's parts list when the search is too long
(`src/components/SearchFooter.svelte`, `.part-what` of the Wanted part): a
small mark before the word "Wanted". Today it is the text glyph `+` in the
want colour, 9 × 18 px (measured at scale 1 in the base build) in its 16 px
column.
Size: 16 × 16 px at scale 1 in the list (the row's mark column is 16 px
wide), drawn at 12 × 12 px in the parts list, from the same file; scales
0.75, 1, 1.33, 2: an SVG, so it stays crisp; strokes at least 1.5 px at 16 px
so they still read at 12 px and at 0.75 (9 px).
Slicing: none (drawn whole, `contain`).
Format: SVG, transparent background (no filled square behind the mark);
the mark itself fully opaque, edges anti-aliased.
States: single state (the row's segmented Want / — / Avoid control shows the
choice; the mark only repeats it).
Palette: gold `#D0BF94` for the mark (the want colour of the Exile theme),
with at most a thin edge in `#B8C0C5` or a shadow in well `#090D10`; no other
colour.
Material: gold accent; light from the top left; edges: a crisp 1 px inner
bevel at most, no glow.
Safe area: the whole 16 × 16 box is the mark's; keep 2 px clear inside each
edge so it never touches the row's text column, which starts 12 px to its
right. Nothing outside the box is ever painted.
References: `docs/quest-tracker/design/steel-masonry-kit/components/status-available.svg` (match the gold edge weight and the diamond's restraint), `docs/quest-tracker/design/steel-masonry-kit/previews/components.png` (the "Status diamonds" row: the size and stroke weight of a 24 px mark), `docs/quest-tracker/design/steel-masonry-kit/assets/icons/worth-yes.svg` (an existing positive mark's drawing style), `docs/waystone-roller/design/frames/exile-main-default.png` (the two wanted rows at the top of the list, where the placeholder sits).
Current: `docs/waystone-roller/design/reference/wr-mark-want.png` (today's element, from the base tag)
Placeholder: a 16 × 16 px box with a 1 px dashed gold `#D0BF94` outline around the `+` glyph (13 px); in the parts list a 12 × 12 px dashed gold square before the word.
Wiring: `apps/waystone-roller/theme/materials/requested/mark-want.svg`; manifest entry `wr-mark-want` in `apps/waystone-roller/theme/materials/requested/manifest.json`
Acceptance: the SVG's `viewBox` is square (16 × 16 or a multiple); its background is transparent (no element covers the whole box); it uses only the colours listed above; dropped in and flipped to `delivered`, `npm --prefix apps/waystone-roller test` and `npm --prefix apps/waystone-roller run corner-safety` exit 0.
Status: delivered (owner, 2026-11-02; the file is in place and the Exile theme draws it)

## wr-mark-avoid — mark-avoid.svg

Used in: the main window. (1) The mod list (`src/components/ModRow.svelte`,
`.choice-mark` of a row whose choice is Avoid): the mark in the 16 px column
left of the mod's text, telling the row is avoided by shape. (2) The
footer's parts list when the search is too long
(`src/components/SearchFooter.svelte`, `.part-what` of the Avoided part).
Today it is the text glyph `−` (minus) in the avoid colour, 9 × 18 px
(measured at scale 1 in the base build) in its 16 px column.
Size: 16 × 16 px at scale 1 in the list, 12 × 12 px in the parts list, from
the same file; scales 0.75, 1, 1.33, 2: an SVG; strokes at least 1.5 px at
16 px. It must read as a different shape from `mark-want.svg` (not only a
different colour): for example a bar or a crossed form against the want
mark's plus.
Slicing: none (drawn whole, `contain`).
Format: SVG, transparent background; the mark fully opaque, edges
anti-aliased.
States: single state.
Palette: error `#D0A3A0` for the mark (the avoid colour of the Exile
theme), with at most a thin edge in `#B8C0C5` or a shadow in well `#090D10`.
Material: steel with a muted red accent; light from the top left; edges: a
crisp 1 px inner bevel at most, no glow.
Safe area: the whole 16 × 16 box; keep 2 px clear inside each edge. Nothing
outside the box is ever painted.
References: `docs/quest-tracker/design/steel-masonry-kit/components/message-error.svg` (the kit's use of `#D0A3A0`: a quiet accent, not a fill), `docs/quest-tracker/design/steel-masonry-kit/previews/components.png` (the "Status diamonds" row: size and stroke weight), `docs/quest-tracker/design/steel-masonry-kit/assets/icons/worth-no.svg` (an existing negative mark's drawing style), `docs/waystone-roller/design/frames/exile-main-default.png` (the avoided row under "Suffixes").
Current: `docs/waystone-roller/design/reference/wr-mark-avoid.png` (today's element, from the base tag)
Placeholder: a 16 × 16 px box with a 1 px dashed `#D0A3A0` outline around the `−` glyph (13 px); in the parts list a 12 × 12 px dashed square before the word.
Wiring: `apps/waystone-roller/theme/materials/requested/mark-avoid.svg`; manifest entry `wr-mark-avoid` in `apps/waystone-roller/theme/materials/requested/manifest.json`
Acceptance: the SVG's `viewBox` is square; its background is transparent; it uses only the colours listed above; its shape differs from `mark-want.svg`; dropped in and flipped to `delivered`, `npm --prefix apps/waystone-roller test` and `npm --prefix apps/waystone-roller run corner-safety` exit 0.
Status: delivered (owner, 2026-11-02; the file is in place and the Exile theme draws it)

## wr-update-dot — update-dot.svg

Used in: the main window's header (`src/App.svelte`, the Settings button,
`.update-mark .dot`): a small mark before the words "Update 1.3.0" when a
check found a newer version, so the button says an update is waiting. Today
it is a plain 6 × 6 px square in the want colour.
Size: 8 × 8 px at scale 1; scales 0.75, 1, 1.33, 2: an SVG; at 0.75 it is 6
px, so its shape must still read at that size (a solid form, no fine
detail).
Slicing: none (drawn whole, `contain`).
Format: SVG, transparent outside the mark; the mark fully opaque.
States: single state (the button's own hover, press and focus are drawn by
the button around it).
Palette: gold `#D0BF94` for the mark, optionally edged in `#B8C0C5`; it sits
on the button's face, steel `#1E272D`, or `#536169` while hovered, so it
must read on both.
Material: gold accent; light from the top left; edges: crisp, no glow.
Safe area: the whole 8 × 8 box; the words "Update …" start 6 px to its
right and must not be touched. Nothing outside the box is ever painted.
References: `docs/quest-tracker/design/steel-masonry-kit/components/status-available.svg` (the gold diamond: a small diamond at this size is a good fit), `docs/quest-tracker/design/steel-masonry-kit/components/quest-divider.svg` (the centre diamond's proportions), `docs/quest-tracker/design/steel-masonry-kit/components/button-hover.svg` (the hovered face it must read on), `docs/waystone-roller/design/frames/exile-main-default.png` (the header's Settings button, top right).
Current: `docs/waystone-roller/design/reference/wr-update-dot.png` (today's element, the mark and its words, from the base tag)
Placeholder: an 8 × 8 px square filled with control `#424D54` and outlined by a 1 px dashed gold `#D0BF94` line.
Wiring: `apps/waystone-roller/theme/materials/requested/update-dot.svg`; manifest entry `wr-update-dot` in `apps/waystone-roller/theme/materials/requested/manifest.json`
Acceptance: the SVG's `viewBox` is square (8 × 8 or a multiple); transparent outside the mark; uses only the colours listed above; dropped in and flipped to `delivered`, `npm --prefix apps/waystone-roller test` and `npm --prefix apps/waystone-roller run corner-safety` exit 0.
Status: delivered (owner, 2026-11-02; the file is in place and the Exile theme draws it)
