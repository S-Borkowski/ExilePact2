# Quest tracker icons

38 original SVG icons for the [design brief](../ui-design-brief.md).
This delivery covers icons only. The specimen colours and typography do not
define the later overlay UI.

Open [preview.html](preview.html) for the labelled specimen, or view
[preview.png](preview.png). [manifest.json](manifest.json) maps every icon ID
to its file and label. [quest-tracker-icons.zip](quest-tracker-icons.zip)
contains the SVG folders, manifest, this guide and both previews.

## Export specification

- `width="24"`, `height="24"`, `viewBox="0 0 24 24"` on every SVG.
- Transparent background; 1.8 px strokes with rounded joins and caps.
- Angular silhouettes, cut corners, faceted stones and restrained ornament.
- Single-colour `currentColor`, except the four resistance defaults below.
- No fonts, raster images, external references, scripts, filters or gradients
  inside the SVGs. Each file includes an accessible title.
- The preview shows enlarged artwork and actual 24, 20 and 16 px samples.
  Its light swatches show the same shapes in dark monochrome.

## Contents

| Folder | Count | Icons |
|---|---:|---|
| `reward/` | 15 | Passive point, weapon-set point, spirit, fire/cold/lightning/chaos resistance, life, mana, skill gem, support gem, ascendancy, item/currency, unlock, story-only |
| `status/` | 6 | Locked, available, in-progress, done, skipped, unknown |
| `worth/` | 3 | Full (worth it), half (optional), none (not worth it) |
| `ui/` | 14 | Four chevrons, collapse, expand, settings, close, edit, click-through, external-link, character, refresh, manual-tick |

The four chevrons are directional variants of one mark. Collapse and expand
are separate exports. Manual-tick is the manual-detection marker named in the
brief's component table. Refresh also serves the Reset action.

Statuses have distinct shapes: pointed lock, open exclamation frame,
hourglass, framed check, struck arrow, and question diamond. Worth levels use
a full star, half star, and diamond with a dash. Colour is supplementary.

## Art references

Published PoE2 art was inspected for subject and silhouette reference:

- [Uncut Skill Gem](https://poe2db.tw/us/Uncut_Skill_Gem): a broad, mounted
  stone, simplified into an irregular faceted outline with two small claws.
- [Uncut Support Gem](https://poe2db.tw/us/Uncut_Support_Gem): an elongated,
  pointed stone, kept visibly different from the skill gem in monochrome.
- [Quest reward art](https://poe2db.tw/us/Quest): the book, spirit skull and
  coin motifs inform weapon-set point, spirit and item/currency respectively.
- [Book of Specialisation](https://poe2db.tw/us/Book_of_Specialisation) and
  [Gembloom Skull](https://poe2db.tw/us/Gembloom_Skull) provide the corresponding
  reward context.

All exported paths are newly drawn. The set contains no copied or embedded
game images. Remaining category symbols use original, compact abstractions:
connected nodes, elemental wards, heart, mana drop, rising altar, gateway and
open storybook. These represent reward categories rather than specific items.

## Colour and import

| Resistance | Default colour |
|---|---|
| Fire | `#E99568` |
| Cold | `#91C9DE` |
| Lightning | `#E3C76F` |
| Chaos | `#BA9ACD` |

Drag the individual SVG files into Figma. For inline SVG, set CSS `color` on
the SVG or its parent; resistance icons have their own root colour, so
override `color` on those SVGs directly. For an external image or a renderer
that does not resolve `currentColor`, resolve it to the desired colour when
importing. An external `<img>` does not inherit the parent page's text colour.

When inlining multiple SVGs, give the title and `aria-labelledby` a unique
ID per instance, or replace them with the host component's accessible label.
Collapse/expand assume the default right dock; mirror them horizontally for
a left dock. Strokes scale normally with the viewBox.

The specimen uses warm ivory on charcoal, with antique gold for worth levels.
Use a dark solid panel behind the default resistance colours; on a light
surface, use darker theme colours or the monochrome treatment. Actual overlay
compositing and toolkit integration belong to the UI session.
