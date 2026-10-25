# Design review of the Figma comps

Five review rounds ran on the comps before the harvest, each by a fresh
reviewer that had only the screenshots, the brief and `PRODUCT.md`. Every
round used impeccable 4.7.1's critique procedure plus a taste-skill 1.0.0
pass. The owner read each round's findings and chose what to fix while the
Figma file was still theirs.

detector: unavailable — impeccable's detector reads HTML/CSS markup or a live page, and the comps were Figma PNG exports, so there was nothing for it to scan in any round. It runs at the finish review on the real build (`review-finish.md`).
critique: Nielsen 30/40, "Good" (round 5, before its fixes). Every heuristic scored 3. Earlier rounds: 27, 27, 28, 30.
taste: passes. Radius 0 with 45° cut corners throughout, warm off-white text on off-black, no glow, gradient or decorative purple; colour carries meaning (teal = edit mode only).

## How the score moved

| Round | Score | What the round changed |
|---|---:|---|
| 1 | 27 | Toasts off the rows; distinct glyphs for manual-tick, optional and not-worth; error triangle; in-progress highlight; bigger badge count and type; solid edit border; panel shadow |
| 2 | 27 | Owner choices: cut corners, EDIT chip, play-mode empty states. Then: muted stars on finished rows, inset in-progress bar, real empty-state buttons, violet unknown, Reset confirm, per-type toast glyphs |
| 3 | 28 | One fixture across frames, "Not worth it", Undo toast, log-idle header, recoverable no-log error (path, Browse/Retry, ticks kept), Settings guards, icon key |
| 4 | 30 | Owner choices below; grey optional star, dimmed not-worth names, eye-slash manual marker, labelled act count, picker chevron, idle badge, Esc notes |
| 5 | 30 | Warning badge, distinct Unknown/Manual copy, compact finished rows (32 px), countdown bars, consistent next-up frames, darker-scene border |

## Owner decisions (2026-10-25)

- Rows show a short reward text ("Spirit", "Passive · Cold res"), not icons.
  Icons with labels stay in quest detail.
- Only the current act is listed, in both views.
- An in-progress quest always shows its next step as a second line.
- "Go with recommended" on round 1's open options: a 45° cut-corner system
  (6 px panel, 3 px buttons, 4 px toasts and badge, radius 0 elsewhere), an
  EDIT chip in the edit header, and play-mode empty states that say "Press
  F8 …" instead of showing buttons.
- Toasts stack below the panel, never above it (the minimap sits above).
- Hotkeys, rebindable in Settings and working in both modes: F8 edit mode,
  F7 collapse/expand, F6 switch view, F1 icon key. Esc closes detail,
  settings, the picker and the icon key.
- The opacity slider fades the background only; text and icons stay fully
  opaque.
- The icon key opens from Settings and with F1.
- After round 5: fix the two P1s and the small copy and consistency items,
  then harvest; leave interaction states for the finish review.

findings:

Open at the harvest, from round 5. They are left for the finish review on
the real build, because they are behaviour a static comp shows poorly:

- Keyboard path through rows in edit mode (↑/↓, Enter opens detail, D done,
  S skip).
- Hotkey rebinding states: listening ("Press a key… Esc cancels") and a
  conflict prompt; whether a key PoE2 uses may be bound.
- Hover tooltips on the edit-header buttons showing their hotkeys
  ("Collapse · F7").
- A one-time first-run hint that lists the hotkeys.
- A "New character" flow frame, and renaming or removing a character.
- More than three toasts at once (overflow).
- The log-idle state offers no fix yet (for example "Check the log path").
- Three neighbouring cool hues (teal edit accent, blue in progress, green
  available): watch them on real scenes.

The edit header shows the character name with a picker chevron but no
class or level, for width. Reviewers noted it; it is open for the owner.
