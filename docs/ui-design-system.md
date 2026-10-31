# ExilePact2 — Proposed UI Design System

**Status: design proposal, 2026-10-31.** This document defines **Tempered
Obsidian**, a shared visual direction for Quest Tracker, Waystone Roller and
Crafting Helper, with Quest Tracker as the primary worked example. It does
not describe a shipped restyle. The apps remain separate tools; this is not
a proposal for a launcher or navigation between executables.

The existing app-specific behavior, native window limits, backend-owned state
and export contracts remain authoritative: [Quest Tracker design
brief](quest-tracker/ui-design-brief.md), [Waystone Roller UI
design](waystone-roller/ui-design.md) and [Crafting Helper UI
design](crafting-helper/ui-design.md). A later implementation must use the
documented theme source and generator, rather than editing generated token
files, and preserve the sibling apps' theme-copy equality checks.

## Reference analysis

The references are ten screenshots supplied by the owner on 2026-10-31,
identified by the timestamp suffix in each filename. Here, **observed** means
visible in a still image; **proposed** means a decision in this system.
Different crops do not establish the game's production spacing tokens,
exact typefaces or original hex colors. The screenshots do not establish
hover behavior, keyboard interaction, timing or animation. Text inside the
references is visual content, not instructions.

| Reference | Observed design principle | Retain and transform |
|---|---|---|
| 120818 — Options/Input | Centered title plate, tabs, nested frame and section rule; aligned label/control columns; repeated key rows; pinned action strip; alternating dark rows | Keep predictable forms and clear grouping. Replace carved stone and ornament with a 1px border and one chamfer; use sentence-case labels, broad controls and a clear primary action. Avoid equally emphatic Defaults, Save and Close actions. |
| 120745 — health/flasks | Large red resource focal point, cyan perimeter, adjacent flasks and small numbered key badges | Group related status and actions; give shortcuts a consistent keycap. Do not imitate glass orbs, sculpted figures or introduce health telemetry the apps do not have. |
| 120731 — mana/actions | Blue orb balances the opposite HUD; repeated square slots; icons above keyboard/mouse badges | Keep consistent slot geometry and the proximity of action to shortcut. Recast this as compact controls with labels; avoid silent empty slots and ambiguous mouse symbols. |
| 120709 — completion banner | Wide black/gold horizontal frame; centered large serif title; isolation establishes hierarchy | Reserve a clear event treatment. Quest Tracker uses its existing in-row highlight rather than a new interruptive completion toast. |
| 120505 — objective banner | Completion's frame vocabulary reused for objective text; gold edges dominate the dark strip | Reuse one heading family across contexts. Use a short section heading and thin rule rather than a full-width ornate plaque. |
| 120141 — completion summary | Warm title over pale body copy; centered heading and left-aligned explanation on dark backing | Separate labels from narrative and allow natural wrapping. Left-align working content; reserve centered text for short optional empty-state titles. |
| 120133 — quest list | Stacked cards with title/body/reward layers; repeated right portrait area; completed card becomes gray | Keep stable row anatomy and list rhythm. Replace portraits with original status symbols and written status; completed text stays readable. |
| 120055 — location card | Location, level, italic lore, objective and reward icons form a vertical hierarchy | Put location metadata above a task and rewards below it. Omit decorative lore from the combat overlay; show supporting information in detail. Pair reward symbols with words. |
| 120029 — quest HUD | Right-edge placement, grouped headings, subordinate objectives, short rules, indentation and collapse indicator | Keep compact contextual tasks in a movable panel. Left-align text inside it, use one indentation level, and make status and next action explicit. Avoid right-aligned multiline reading. |
| 120759 — world map | Dominant spatial canvas, top-center mode label, act selector, top-right mode switch and salient circular markers on subdued terrain | Keep foreground/background discipline and clear selection. Transfer these to route progression and list selection, rather than introducing a world-map capability. Prefer a keyboard-readable ordered route to a sprawling node canvas. |

Across the set, near-black absorbs detail, gold assigns importance, material
layers imply depth, and repeated frames establish a family. Proximity groups
related information; framing separates gameplay from interface; small caps
mark headings; imagery carries theme. The modern system removes visual noise,
low-contrast inactive text, tiny targets, ambiguous icons, long all-caps copy,
excessive nested borders and decoration that competes with the game.

## Design rationale

Tempered Obsidian combines dark mineral surfaces, warm ivory reading text,
restrained antique-gold hierarchy and cyan keyboard focus. Proportion,
warm/cool balance and fine engraved-looking lines carry the references'
weight; concise words, measured density and predictable controls suit the
tools' everyday work.

Use three visual layers: a quiet page or overlay field, one enclosing panel,
and selected or interactive content. Allow at most one visible accent border
per region; use dividers for the remaining grouping. A small original
diagonal cut on major containers provides identity. Keep reading areas free
of ornamental texture. Do not reproduce the references' stone textures,
statues, gothic arches, skulls, logos, fonts, parchment title plates or
resource globes. All new artwork must be original.

Gold marks headings, primary actions and importance rather than every border.
Semantic colors belong to data; cyan identifies focus and edit mode so that
keyboard focus remains distinct from gold selection. The information order
is **title → current next action → rationale/reward → metadata**. The overlay
presents one useful action at a glance; ordinary windows expose controls and
explanation. Prefer a flat list with a selected detail region to a dashboard
of unrelated cards. Preserve backend-owned truth and refusal messages;
styling must not conceal uncertainty, imply automation or invent progress.

## Color palette

These are proposed values, not sampled game colors or values already applied
to the generated theme. A later restyle maps the semantic roles to the
existing theme source.

| Role | Hex | Application |
|---|---|---|
| canvas | #111416 | Ordinary window field; dark text on filled gold buttons |
| surface | #171A1C | Main panels and default overlay backing |
| raised | #202528 | Header/footer and hover surfaces |
| selected | #24292C | Selected list row surface |
| sunken | #0C0F11 | Inputs, copied-string output and raw item text |
| divider | #343B3E | Nonessential separators only |
| control-border | #727C80 | Essential input and secondary-button outlines |
| text-primary | #EEE9DD | Main task and body copy |
| text-secondary | #C8C3B8 | Supporting explanation |
| text-muted | #9C9D96 | Metadata and completed labels; no opacity-reduced body copy |
| gold | #D0B477 | Headings, importance marks and primary fill |
| gold-hover / gold-pressed | #DEC58F / #BEA16A | Primary hover / press |
| focus-edit | #83CED2 | Focus ring and explicit edit-mode edge |
| available-success | #9AC9A7 | Available/check/positive result, paired with words or a distinct shape |
| in-progress-info | #A3C7E7 | Current quest arrow and informational line |
| warning | #E5BD76 | Attention/limit-near notice with triangle and explanation |
| error-danger | #E59A95 | Error/refusal/miss, with ! or written status |
| unknown | #C1AFE2 | Unknown status with question symbol |
| disabled | #9C9D96 | Disabled words on raised surface; no blanket container opacity |

Calculated with sRGB relative luminance on opaque pairs during design:

| Pair | Contrast ratio |
|---|---|
| text-primary / surface | 14.43:1 |
| text-secondary / surface | 9.95:1 |
| text-muted / surface | 6.39:1 |
| gold / surface | 8.72:1 |
| available-success / surface | 9.40:1 |
| warning / surface | 9.89:1 |
| error-danger / surface | 7.82:1 |
| focus-edit / selected | 8.21:1 |
| canvas / gold | 9.23:1 |
| control-border / surface | 4.09:1 |

These calculations validate those pairs only, not every token combination
or a rendered screen. The adoption target is at least 4.5:1 for normal text
and 3:1 for essential control/focus boundaries against adjacent surfaces,
including hover, press and each backing choice. Divider is decorative and
must never be the sole cue identifying a control.

Overlay backing is proposed at the existing theme token value of 0.92;
Quest Tracker’s current opacity setting defaults to 0.9. Opacity remains a
user setting, including 0%; the opaque ratios do not certify text
over gameplay. Before a restyle release, check composites over white, black
and busy captured or fixture backgrounds. A minimal dark text shadow can
support reading but does not guarantee contrast at 0% backing. Explain the
low-opacity tradeoff without removing the setting. No frosted glass or
backdrop blur.

## Typography scale

Keep the already bundled, licensed **Alegreya Sans** and **Alegreya Sans SC**;
no new font download or game font is needed. The SC face is for short titles
and section names; body copy stays mixed case. Use system `ui-monospace`,
`Cascadia Mono`, `Consolas` for copied search strings, raw item text and
hotkeys. Use tabular numerals for odds, tiers and counts. Never force user
text to uppercase.

| Style | Size / line height, px | Weight | Use |
|---|---|---|---|
| Display | 28 / 36 | 700 SC | Optional large documentation mockup title; not the overlay |
| Window title | 24 / 32 | 700 SC | Crafting/Waystone page title |
| Section | 20 / 28 | 700 SC | Detail heading and main-window section |
| Overlay title / row title | 16 / 20 | 700 SC / 500 Sans | Header / quest name |
| Body | 16 / 24 | 400 Sans | Main-window guidance |
| Compact step | 14 / 20 | 400 Sans | Overlay next step and helper text |
| Label / button | 14 / 20 | 500 / 700 Sans | Controls and filter labels |
| Caption | 12 / 16 | 400 Sans | Optional metadata, never the sole error/privacy explanation |
| Output / keycap | 14 / 20 | System mono normal | String preview / shortcut |

Default tracking is 0; SC headings may use at most +0.02em. Prose in detail
and main windows should run 45–75 characters per line; the overlay wraps
earlier. Do not clip quest names or guidance to fixed heights. Long names
and paths wrap. A labelled horizontal code scroller is acceptable for raw
strings whose literal spacing matters. Never ellipsize the next action or
an actionable error.

## Spacing system

The base unit is **4px** at content zoom 1×. Tokens are **4, 8, 12, 16, 24,
32, 48, 64px**: 4 inside icon/text microgroups, 8 between related labels, 12
for overlay padding and row gaps, 16 for main-window card padding, 24 between
sections and for page padding, and 32 between major areas. Avoid arbitrary
one-off gaps. Borders of 1px and a 2px focus outline are optical exceptions.

| Region | Dimensions and rhythm |
|---|---|
| Overlay shell | 320px nominal width; 12px horizontal insets; 40px header; 28px act/group header |
| Quest row | At least 48px, with 12px top/bottom padding; grows for a 20px next-step line, tips and wrapped titles |
| Status column | 16px icon plus 8px gap, aligning every title |
| Substeps | At most 24px indentation; one indentation level |
| Overlay edit targets | At least 32×32px with 8px separation |
| Collapsed overlay | 48×48px |
| Ordinary windows | 24px outer padding, 16px grid gutter and section inset; 16px outer padding in narrow windows |
| Buttons and inputs | At least 40px in ordinary windows; 44px targets in touch-oriented mockups; multiline rows grow |
| Icons | Original vectors on a 16/20/24px grid; single color except semantic marks |

Keep Quest Tracker's existing 320×240 native minimum and scroll the body
independently. Use square corners with a proposed 2px bevel/chamfer on
buttons and 6px cut on major panels. These geometric dimensions are design
values, not instructions to change generated chamfers in this documentation
change. Shadows indicate elevation only: dark `0 8px 24px` at approximately
35% opacity, subtle enough not to form another border.

## Component library

Every component inherits semantic tokens. This inventory does not add app
capabilities. Every interactive control needs default, hover, pressed,
focus-visible and disabled variants; hover never substitutes for selection.
The focus ring is a 2px cyan outline with 2px offset, kept visible outside
chamfers and overflow clips. Where a refusal or limit disables a control,
place the reason beside it.

| Component | Anatomy and variants | Behavior and limits |
|---|---|---|
| Panel / window shell | Flat field, one border, heading, scroll body, optional pinned footer; overlay/edit/ordinary | Edit mode has an edge and explicit text; no interactive play-mode chrome |
| Section heading | SC name, optional count, thin rule | Left aligned; count secondary; no ornate plaque |
| Button | Gold primary, outlined secondary, text tertiary, explicit-verb danger; optional leading icon | One primary task per region; loading keeps width and names the operation; no icon-only destructive action |
| Icon button | Original 20px glyph in 32/40px target with accessible name | Tooltip on hover/focus in interactive windows; essential meaning cannot depend on a tooltip |
| Tabs / segmented choice | Label, selected underline/filled segment and selected semantics | Existing local views only; no suite-wide app tabs |
| Quest row | Status, title, next step, optional tip and worth marker; available/in progress/done/skipped/locked/unknown/manual | Shape plus color; status words in detail/icon key; completed text stays legible; edit click opens detail, play click passes to game |
| Detail panel | Title, zone/giver, worth reason, steps, rewards, source, existing mark/skip/reset actions | In-panel detail with contextual return on narrow overlay; confirmations only for existing destructive behavior |
| Status mark / badge | Available open diamond, in-progress arrow, done tick, skipped dash, locked lock, unknown ?; optional manual tick | Preserve semantic distinctions using original assets; never infer reward from status |
| Keycap / shortcut row | Mono key label, modifier grouping and action label | Read current binding, not historical F-key examples; explicitly show capture/error/reserved/conflict states |
| Text field / select / filter | Visible label, input, helper/error, optional clear | 40px ordinary height; no placeholder-only labels; validation follows intentional edit/submit as the current app dictates |
| Toggle / checkbox / slider | Label, actual state and helper; slider numeric value | Toggle for immediate settings, checkbox for selection; keyboard increments and live preview where currently supported |
| Filter/modifier row | Modifier words, tier, Want/Ignore/Avoid segments and +/− marks | Backend is the source of truth; no color-only meaning or local search-string reconstruction |
| Output footer | Full search string, count/250, Copy and refusal explanation | Always reachable; over-limit output never truncates or copies; Copy differs from the paste hotkey |
| Item summary / target row | Base/rarity, modifier groups, target flag and measured numeric columns | No apparent grab before clipboard/hotkey input; preserve the last-good item after a refusal |
| Route step | Ordered number, action/currency words, odds, hit/miss destinations and current marker | Ordered text is primary; connectors decorative; no graph editor or automatic game action |
| Banner / inline status | Icon, sentence and contextual recovery; info/success/warning/error | Persistent failures in ordinary windows; Quest Tracker changes use existing row highlights, not new toasts |
| Progress / update line | Operation, determinate value if known and Cancel where available | Unknown total stays unknown; no invented percentages; preserve separate update states |
| Empty state | Small original icon, concise title, cause and existing next action | Distinguish waiting/no character/no log/all done/no matches; never present an error as empty |
| Tooltip / icon key | Brief explanation / complete legend | Keyboard accessible in edit mode; play mode needs inline meaning or its existing hotkey legend |
| Scroll / disclosure | Usable native scrollbar; heading, chevron and expanded state | One list scroll area where possible; expanded content remains in reading order |

## Wireframe descriptions

Dimensions are content pixels at 1×, excluding native title bars. These
describe composition, not replicas of the screenshots.

1. **Quest Tracker — Next up, play, 320×480.** A 40px top row places the
   title left and current character in secondary type right; act/zone
   grouping follows. Each body row aligns a 16px status column with quest
   title and next action. Keep the existing quest order; show reward/why
   only where already part of the view. An optional 3px in-progress edge
   stays within the one panel boundary. Text is left aligned and the body
   scrolls. No buttons or hover states appear in play mode. Idle/error
   headers use words and preserve rows as current behavior permits. The
   composition is `[Act 1 · Clearfell] → [arrow · Quest name / next step /
   find-it tip] → [diamond · next available quest]`; it does not define new
   game content or ordering logic.

2. **Quest Tracker — edit/detail, 320×480.** Reuse row geometry with a cyan
   outer edge and Edit mode label. Header controls for the existing views,
   collapse and settings use 32px targets beside the drag region. Extra
   controls wrap into an edit-only row instead of colliding with character
   text. Quest selection replaces the list body with Back/title, reason,
   steps, rewards/giver/zone, source and the existing mark/skip/reset actions.
   At 320×240, header/footer remain reachable while only the body scrolls.
   The 48×48 collapsed badge shows count and original status mark, with its
   existing expand affordance available in edit mode.

3. **Quest Tracker — ordinary Settings, approximately 420×328.** Keep the
   native title bar, 16px padded Settings heading, Updates section,
   automatic-check switch and privacy helper. Follow with status/install
   lines, progress, Check now, the app-owned install/cancel/retry action,
   version, divider and Created by ST4H footer. This small ordinary window
   stays distinct from overlay settings. The overlay settings retain
   opacity, scale, filters, read-screen, hotkey and log controls in stacked
   groups under their current contract.

4. **Waystone Roller — 800×860.** Header: title and Settings. Below: compact
   labelled filters, then the modifier list with `[modifier words | tier |
   Want / Ignore / Avoid]`. Pin the output footer with the full search
   string, count/250, over-limit breakdown when necessary and Copy. Search
   remains local filtering. Avoid turning the header/footer into unrelated
   dashboard cards. At 560×480, filters wrap and segments move beneath row
   text; the footer must not cover the final rows.

5. **Crafting Helper — planning, 960×680.** Header: title and the existing
   trade, Craft of Exile and Settings actions. Body: `[Item, 240px] [Targets,
   flexible] [Route, at least 280px]`, with 16px gutters; footer: existing
   status line. Inner width and column minima decide when to use two columns
   or a stack. Present route steps as ordered action words with a clear
   current marker. Start crafting belongs to the route region. Initial,
   refused and planning states reuse these slots with explicit messages,
   preserving the last-good item where the app does.

6. **Crafting Helper — narrow/guiding.** At 600×480, stack item summary,
   targets and route in source order; expand full details only where current
   controls permit. Keep the guiding current step above long history and
   label hit/miss destinations with text. Its separate click-through game
   overlay shows current action, destinations, last outcome and screen-watch
   status only: no controls, new game input or fake live resource meter.
   Existing native minima and overlay placement remain authoritative.

7. **Shared error/empty/update comparison.** Use the same shell for waiting
   (neutral mark, reason and expected trigger), refused/error (!, precise
   cause and existing recovery), all done (tick, completion words and any
   existing view switch), and update states (checking/available/downloading/
   unsigned/failed, operation-specific text and available actions). Offer
   no action in click-through play mode; point to the current edit-mode
   hotkey instead. Persistent failures remain visible until recovery.

## Responsive behavior

These are resizable desktop companions, not a mobile product. Breakpoints
below are proposed content-layout rules; existing native minimum sizes
remain. Assess effective content width at 200% zoom rather than physical
display pixels. Quest Tracker content zoom of 0.75×–2× remains independent
of native window resizing; changing zoom does not resize the native window.

| Effective content width | Layout |
|---|---|
| 320–479px | Overlay and narrow Settings: one column, labels above controls, wrapped actions, 12–16px padding, complete text and body scrolling. The 320×240 overlay remains usable. This does not give Waystone or Crafting a supported 320px native minimum. |
| 480–759px | Compact ordinary-window rules where native minima allow: 16px padding, stacked sections, wrapping filters and 40px targets. Keep Crafting's existing below-760 header wrapping. |
| 760–959px | 24px page inset; two-column item/targets with route below when its minimum cannot fit. Waystone rows may align controls horizontally. |
| 960px and above | Crafting may use three columns if inner width fits 240px + flexible ≥240px + route ≥280px + 32px gutters. Wider views grow reading areas to their measure cap instead of stretching text indefinitely. |

The body takes the remaining height after intrinsic header/footer heights.
At minimum window heights, allow whole-page vertical scrolling when needed;
keep actions reachable without fixed footers occluding content. Prefer an
overlay height of at most 60% of the game viewport where the current resize
contract allows, without forcing a change to saved user sizes.

Long translations, 200% text zoom, long character names and wrapped errors
must not overlap controls. Avoid horizontal page scrolling; confine any
unavoidable literal-string scrolling to its labelled output box. A narrow
layout presents the same capabilities in sequence rather than removing them.

## Interaction and accessibility

The stills show affordances for tabs, selects, key bindings, checkboxes,
buttons, collapse markers, map nodes and status differences. The following
behaviors are proposed or retained from existing app documentation; the
screenshots do not prove them.

Ordinary-control hover raises the surface one level; pressing darkens it or
uses gold-pressed. Selection persists through fill plus marker/text. Focus
has its own visible ring. Use native button/input/select/switch semantics;
tabs expose selection and follow their established keyboard pattern. Tab
order follows reading order, includes every existing control and never
traps focus. Keep Escape behavior specific to each window: Quest Tracker's
ordinary Settings explicitly does not close on Esc. Existing dialogs and
disclosures restore focus to their trigger where appropriate.

Quest Tracker play mode stays click-through; edit affordances exist only in
edit mode. Helpers display the current binding. Do not turn historic F8/F7
examples in the design brief into new global bindings: the current README
documents Ctrl+Alt+Q for mode, and current app settings are the authority.

Preserve the documented row-change model: approximately 150ms exit then
220ms entrance, 4–6px movement and about 2.5s of nonflashing highlight.
With reduced-motion, swap steps instantly without translated or pulsing
motion, retaining a static highlight. Repeated changes restart cleanly,
without a queue or toast stack. Announce each meaningful change once in a
polite live region instead of rereading the full list. Other control
transitions use 100–150ms of opacity/color only; no continuous ornamental
animation.

Pair status color with words or a distinct shape. Completed rows use a tick
and metadata rather than global opacity. Associate visible field labels and
helpers; sliders expose their values and keyboard increments. Targets are
at least 32px in overlay edit mode and 40px in ordinary windows, separated
spatially. Important error/privacy helpers use at least 14px, never Caption.
Updates, errors and loading states need meaningful words; unknown progress
must not appear determinate. This proposal does not certify the apps'
accessibility: screen-reader semantics and actual contrast need rendered
implementation checks.

## Adoption boundaries

This document is a proposal alongside existing app specifications. It does
not change product code, generated themes, fonts, assets, dependencies,
view-model contracts, native minimum sizes or game interaction. No Figma
file, interactive prototype, build or release is produced by publishing it.
Current shipped specifications remain intact. Release notes do not claim an
unshipped restyle; a later player-visible implementation must update its
relevant guides and release notes in the same change.

Use this checklist before adoption:

- Review the visual direction and the choice of Quest Tracker as the primary
  example with the owner before implementing a restyle or bundling assets.
- Implement one representative overlay, list, form and output view, then
  review the results before extending the system across the app family.
- Capture actual visual snapshots at desktop minimum sizes and zoom levels,
  with long text, reduced-motion and light/dark/busy overlay backdrops.
- Verify each essential text/control/focus contrast pairing in its rendered
  states, keyboard traversal, accessible semantics and refusal states.
- Preserve the shared theme source/generator and copy-equality checks, the
  app export contracts, backend-owned state and existing app tests.
- Run an available impeccable design audit when UI changes. Do not install
  design plugins into the repository.
- Keep reference screenshots, copied game art and extracted assets out of
  tracked files. Create original vector ornament if the adopted design
  needs it; do not add remote asset or font dependencies.

Documentation coverage and link checks can establish that the proposal is
complete and connected. They do not establish aesthetic quality or that a
future interface matches it. Actual visual and behavior acceptance belongs
to the separately scoped implementation.
