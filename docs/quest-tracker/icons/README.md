# Quest tracker icons

46 original SVG icons for the [design brief](../ui-design-brief.md).
These are the masters: the Figma harvest copies them, under the brief's
icon ids, to `apps/quest-tracker/theme/icons/`.

[manifest.json](manifest.json) maps every icon ID to its file and label.
[preview.html](preview.html), [preview.png](preview.png) and
[quest-tracker-icons.zip](quest-tracker-icons.zip) show the first delivery
of 38 icons (2026-10-24). They predate the 2026-10-25 additions and the
redrawn manual-tick, worth and unknown marks, so the SVGs win where they
differ.

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
| `reward/` | 21 | Passive point, weapon-set point, spirit, fire/cold/lightning/chaos/all-elemental resistance, life, mana, attribute, charm, flask, defences, other permanent stat, skill gem, support gem, ascendancy, item/currency, unlock, story-only |
| `status/` | 6 | Locked, available, in-progress, done, skipped, unknown |
| `worth/` | 3 | Full (worth it), half (optional), none (not worth it) |
| `ui/` | 16 | Four chevrons, collapse, expand, settings, close, edit, click-through, external-link, character, refresh, manual-tick, check, warning |

The four chevrons are directional variants of one mark. Collapse and expand
are separate exports. Manual-tick is the manual-detection marker named in the
brief's component table: an eye with a slash ("the log can't see this"),
redrawn on 2026-10-25 so it no longer looks like a checkbox. Refresh also
serves the Reset action. Check marks a completed step (toasts, undo);
warning marks errors and an idle log.

Statuses have distinct shapes: pointed lock, open exclamation frame,
hourglass, framed check, struck arrow, and question diamond (with a bolder
"?" since 2026-10-25). Worth levels use a solid star, an outline star and a
plain dash; the half star and the dash in a diamond were dropped because
they read too close to the solid star and to the unknown diamond at 16 px.
Colour is supplementary. "Other permanent stat" uses an octagon for the same
reason.

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
