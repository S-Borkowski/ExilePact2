# Quest Tracker — steel and masonry design handoff

**Status: design assets and specifications only. Application wiring is deferred to another session.**

The owner's latest instruction was: “focus on design only, implementation (wiring it in)/coding will be done later. prepare all components so other session can continue without having to generate any art.”

Start with [overview](previews/overview.png), [component sheet](previews/components.png), [secondary views](previews/secondary-views.png) and [safe-area diagram](previews/safe-areas.png). Editable SVG versions sit beside every PNG. This kit is the latest material/palette handoff; the original [approved close-up](reference/approved-close-up.png) remains the authority for the compact tracker composition. That local reference is a byte-identical copy of `docs/ui-design-system/quest-tracker-overlay-approved.png` and is documentation only.

## What is settled

- Thin weathered **neutral steel/iron** frame; charcoal masonry interior. The preceding rendered app was rejected as “way too yellow.” No warm brass frame, brown/gold fleck texture, gold-filled form controls, statues, lantern, crest or large journal banner.
- Compact live **Quest Tracker / actual Act** bar, flat quest sections, fine steel dividers, serif quest titles/objectives, small configured-hotkey footer. The custom artwork is the outer border; no second CSS ring or raised cards.
- Keep character name/class/level, Worth it/Optional headings and reward labels visible in play. They are compact secondary information.
- All six quest statuses use distinct diamond treatments, **without inner status icons** in Exile. Separate worth/manual/inferred marks remain.
- Exile is the default theme; Claude retains its existing presentation and icons. A selected theme remains the same across play, edit and both Settings surfaces.
- Edit/settings mode remains freely draggable. Keep the existing native-window bounds, independent scale/opacity preferences, resize directions, commands, persistence, privacy and all [22 feature groups](../approved-ui-preservation.md).

The neutral palette shown here incorporates the latest “steel/masonry” correction. It is a finished design deliverable for review, not a claim that the owner has approved a newly wired application. Do not silently substitute the old yellow prototype if a detail is unclear; ask the owner about genuine ambiguity.

## Complete asset inventory

Everything needed for artwork is included locally. **Do not generate, download or extract more art to implement this design.**

| Files | Use |
|---|---|
| `assets/frame-neutral-steel.png` | Final neutral frame, 1189×1323 RGBA, transparent centre/exterior. Production artwork. |
| `assets/masonry-charcoal.png` | Final 1254×1254 cool slate field. Production artwork; use a single cover crop with darkening, not an unverified repeated tile. |
| `components/status-*.svg` | Six finished 24×24 diamond assets. Production geometry/patterns; existing exact accessible names are included. |
| `components/quest-divider.svg`, `header-rail.svg`, `resize-grip.svg`, `loading.svg` | Ready vector linework. Keep actual labels/content live. |
| `components/button-*.svg` | Default, hover, pressed, focus and disabled visual specifications. |
| `components/toggle-*.svg`, `checkbox-*.svg` | Off/on, keyboard focus, disabled; checkbox mixed state included. |
| `components/text-input-*.svg`, `select-*.svg` | Text field default/focus/invalid/disabled; select default/focus/open. |
| `components/segmented-*.svg`, `slider.svg`, `scrollbar.svg` | View switch, range control and narrow scroll treatment. |
| `components/tooltip.svg`, `keycap.svg`, `progressbar.svg`, `message-*.svg`, `collapsed-badge.svg` | Supporting states and feedback. |
| `assets/icons/*.svg` | Byte-identical snapshot of all 46 existing repository icons: worth/reward/manual/navigation/settings/warnings plus Claude legacy status icons. |
| `assets/fonts/*` | Existing local Cinzel and Alegreya Sans font files and both OFL notices, copied unchanged. No new font dependency. |
| `previews/*.svg` and matching PNGs | Editable design masters and browser-rendered review images. Not application backgrounds. |
| `palette.json` | Design colours, separate from runtime token files. |
| `reference/approved-close-up.png` | Original approved composition, included for the next session; never a runtime background. |
| `asset-manifest.json`, `ART-PROVENANCE.md` | Exact file hashes, origin and generation prompts. |

There are **42 component SVGs**, 46 existing icon SVGs and 10 preview masters. Widget SVGs are design specifications, not replacements for real accessible buttons, text fields or selectors. Implement their appearance using the existing live controls. Full-screen mockups contain illustrative text and must never be imported as runtime UI.

Keep the folder structure intact: SVGs reference the supplied sibling `assets/` directory for raster art and local fonts. The packet is portable as a whole; moving an SVG alone will break those relative links. PNG previews can be viewed independently.

## Material and palette recipe

| Role | Colour / treatment |
|---|---|
| Base well | `#090D10` |
| Masonry | Supplied texture at about 38% opacity over the base; one cover crop per well. No warm filter. |
| Steel / bright edge | `#8D979E` / `#B8C0C5` |
| Divider | `#5D686F` with a restrained highlight, no glow |
| Objective / body | `#E4E6E7` |
| Secondary metadata / footer | `#A9AFB3` |
| Restrained antique-gold accent | `#D0BF94`, chiefly quest titles, short title bar and the available diamond edge |
| Form active fill / hover | `#424D54` / `#536169` — steel grey, not cyan or yellow |
| Focus / active diamond | `#E6E5DE` |

The main difference from the rejected prototype is the **neutral structure and masonry**, with much less gold. The in-progress diamond now uses the reference-like ivory fill rather than the earlier yellow fill. Gold remains a limited accent rather than the frame or control surface.

Typography at 1×: local Cinzel SemiBold 17px/24px for the compact bar; Georgia/Times/serif 18–19px/25px for quest names and 15–16px/22px for objectives; muted tips 13–14px/20px; existing Alegreya Sans 11–14px for secondary metadata/forms. Body text stays mixed case. Long text wraps.

## Diamond status mapping

| Existing enum | Visual | Exact existing status name |
|---|---|---|
| `in_progress` | Ivory filled diamond, single steel edge; subtle upper facet | In progress |
| `available` | Hollow diamond, double muted antique-gold edge | Available |
| `locked` | Hollow diamond, thin dim steel edge | Locked |
| `done` | Muted silver fill, double bright steel edge | Done |
| `skipped` | Diagonal steel hatch, single edge | Skipped |
| `unknown` | Hollow diamond, dotted steel edge | Unknown: tick it by hand |

Fill, line count and pattern distinguish the states beyond colour. Use the **same treatment in rows and detail**; switching themes must update both. Display at 20–24px in a consistent 24px status column. Keep one accessible status name and truthful tooltip per mark; decorative facets/patterns must not announce themselves. The large six-state legend in the design sheet is documentation, not an additional play-panel section.

`assets/icons/status-*.svg` are the **old Claude icons**. Do not overwrite them with the new diamonds. If importing diamond files, use a separate namespace/directory. If recreating these SVG shapes as live UI markup, copy their supplied geometry rather than designing new marks. Give repeated inline pattern/clip IDs unique instance names.

## Frame slicing and content safety

The preview masters use the supplied neutral PNG in eight nine-slice segments:

- source dimensions 1189×1323; slice **224px on each side**, centre unfilled;
- destination corner blocks **48px at 1×**, **8px outset**, within the existing 12px transparent native-window margin;
- source columns: 224 / 741 / 224; source rows: 224 / 875 / 224;
- frame is decorative, fixed, pointer-events none, unfocusable and aria-hidden;
- draw masonry separately behind live content. Do not tint the full panel or desaturate its text.

The art's alpha bounds are `(33,40)–(1157,1273)`; centre alpha is zero. No processing or new crop generation is needed. Use the PNG directly with nine-slice rendering. Header/footer text sits clear of the corner knots; the preview has a 20px general reading inset and approximately 40px corner reach.

**Known unfinished application defect:** the current prototype's rectangular whole-scroll well lets quest diamonds/text pass through the lower corner artwork at minimum size. Upper corners can be crossed after the header scrolls away too. The earlier rectangular overflow checks did not prove safe paint.

The [safe-area diagram](previews/safe-areas.png) proposes a 44px top/bottom clearance at 1× for a 296×216 painted panel. This is a **design specification, not a tested app fix**. Do not blindly multiply large vertical insets at 2× and leave controls taller than the remaining viewport. During wiring, use actual rendered corner extents and either safe shaped clipping or appropriately constrained ornament sizing/clearance. All content and focused controls must become fully readable at some scroll position. Default fixed-header/footer panels should keep their compact density.

Keep a fixed, usable drag region above/outside the scrolling content in edit/settings, plus the functional header drag path. Exclude buttons, links, inputs, selectors, role-buttons and resize handles from drag dispatch. Preserve all eight resize directions. Play remains passive. Neither this kit nor prior browser hit tests establish native OS movement; verify that separately if claiming it.

## Views and content rules

The supplied masters cover play, edit, quest detail, character picker, overlay Settings and ordinary Settings; supporting states cover collapsed, empty, warning/error, loading and updates. State samples show the material family, not an exhaustive form or replacement wording contract. Preserve every existing control and exact runtime message even when it is below the illustrated fold.

- Main overlay: 364×384 native default, including existing transparent margins; minimum 320×240. Respect saved size/position. Mode/theme changes must not resize or move it.
- Ordinary Settings has its own existing window and live update states. The 420×360 design sample demonstrates spacing, not a change to that window's saved/default size.
- The compact Act label comes from the **single received `view.acts` entry**. Use Roman formatting only when exactly one act exists; omit it for zero/multiple acts. “All” is the current act's statuses, not all campaign acts.
- The metadata strip represents existing character/class/level. There is no current player-zone binding; do not invent one from quest-location data.
- Render the received current step/tip/rewards. The fictional quest/character strings in previews are illustrative. Do not turn future detail steps into active objectives or invent new progress.
- Keep rewards, worth, manual and inferred information and all current actions/refusals. “Probably done” must not silently become confirmed done.
- Footer uses the actual configured shortcut. Play footer is text with no control. Edit footer retains its existing return-to-play command and full accessible name.
- The empty-state examples use existing action labels; keep all four original empty states and their original recovery behavior. Update checking/available/unsigned/downloading/failure states reuse the supplied control/feedback parts, not new artwork.
- No Apply/save workflow is added. Settings continue using the existing updater/persistence path.

## What the next session starts with

Checkout: `C:/Users/stann/.codex/worktrees/reference-ui-design-system/ExilePact2`.

Existing feature branch: `codex/reference-inspired-ui-design-system`; PR45/issue44 already exist. Do not create a sibling feature branch/PR or merge it automatically.

There is substantial **uncommitted earlier prototype code** in `apps/quest-tracker/src/` and earlier docs/screens. Preserve it and inspect its diff before changing anything. It already contains the preference, compact header, six diamond mechanics and many preservation tests, but still uses the rejected warm artwork/palette and has the whole-scroll corner defect. It is not the finished design. Old release executables were not rebuilt for this kit and do not show it.

Useful existing files: `Panel.svelte`, `Header.svelte`, `MaterialFrame.svelte`, `ActSection.svelte`, `QuestRow.svelte`, `QuestDetail.svelte`, `StatusMark.svelte`, `Settings.svelte`, `src/settings/`, `approved.css`, and QT-local `theme/exile.json`/`exile.css`. No Rust/IPC/engine work is required by this design.

When implementation is explicitly requested:

1. Read this file and view all three sheets before coding. Use these assets; no art generation phase remains.
2. Inspect the current prototype diff and existing behavior/tests. Preserve all 22 feature groups and the Claude baseline.
3. Copy the two finished raster assets into the app's material directory with distinct names; retire warm runtime imports. Retain historical files/provenance and update the explicit curated-asset list/hash/count rather than weakening guards. With four existing PNGs retained plus these two new files, the intended curated set is six. Keep shared tokens/fonts and sibling apps unchanged.
4. Wire the neutral palette and supplied vector treatment through all views and both Settings surfaces, using live text and existing commands. Verify play/edit theme consistency.
5. Solve the corner-safe scroll layout as real application work. Capture a default play panel and minimum detail/All panels, compare to this kit, then cover the remaining states.
6. Use the requested **Tauri/app-webview or Playwright MCP** for app interaction and visual verification. Check minimum/default bounds, scale0.75/1/1.33/2, text200%, scroll top/middle/end, focus reachability, drag exclusions, passive play and independent Settings. Compare artwork intersections, not only rectangle overflow.
7. Run relevant frontend checks and build, with independent final review. The last pre-kit frontend run passed82 tests; this is historical standing, not verification of newly wired art. Unchanged backend standing was430 Rust tests/1 ignored and1648 root Python tests/8 skipped; avoid repeating unrelated suites without a contract change.
8. Only after acceptance, build a new standalone release if requested. Do not call an older executable “the new design.” Update the existing feature PR through the normal project process; publishing/merge remains separate.

The old ignored workorder was marked SUPERSEDED when the owner redirected to design-only. Create an implementation plan for the actual current tree if needed; do not resume an old visual PASS or silently skip the known corner blocker.

### Copyable next-session request

> Implement the Quest Tracker design from `docs/quest-tracker/design/steel-masonry-kit/HANDOFF.md` in the current worktree. Use the finished local art and SVG components; do not generate new art. Preserve the22 existing feature groups, Exile default/Claude choice, all status semantics and free edit/settings dragging. Inspect the existing uncommitted prototype first. Follow the neutral steel/masonry previews, fix whole-scroll corner overlap, verify through Tauri/Playwright MCP and keep the feature on its existing branch/PR45. Ask about genuine ambiguity instead of inventing a different visual design.

## Validation of this deliverable

Both raster outputs were inspected. Frame alpha includes a fully transparent centre and exterior. SVG/PNG design sheets were rendered with Playwright after local font loading. The packet is checked for valid XML, complete local references and manifest hashes. This validates design-file completeness, not application behavior, live dragging or in-game performance. No application source was edited after the design-only redirect, and no release, push or publication was performed for this kit.
