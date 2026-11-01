# Approved UI adoption review

## First-screen critique — 2026-10-31

Reviewed the rendered 364×384 compact Next up/play and campaign/edit screens before extending the material system. The compact frame has the specified 3px steel/masonry trim and a quiet uniform reading field. Existing status shapes, rewards, steps and tips remain visible. The rich shell has original carved corners, a steel inner rail, gold inlay and lantern-astrolabe relief; it is visibly distinct from the gameplay instrument.

Two classes needed correction: the crest/title plate used too much of the default window's height, and journal rows still had the flat compact anatomy. Reduce the crest before content at short heights; add recessed row wells/title rails only to rich surfaces. Long titles and steps must wrap, and edit targets need 32px minimums. First local app-only evidence: `.claude/workorders/approved-first-compact.png` and `approved-first-rich.png` (local review captures, not durable links).

The user-wide impeccable/taste skills were absent during planning; no plugin was installed into this repository. This explicit first-screen critique and the final matrix below are the manual substitute.

## Reference-fidelity correction — 2026-11-01

The owner rejected the initial vector/gray-tile interpretation above as unlike the approved original and reported the lower frame floating over scrolled content. That feedback was correct: the first pass preserved functions but did not reproduce the physical material language, and its decoration sat above a whole-panel scroll area.

Reopened the original concept and compared the new representative journal beside it before expanding checks. The final screen now uses original generated black carved masonry/niche rails, worn warm-gold inlay, a dimensional amber lantern/astrolabe crown, subdued physical stone, rectangular recessed wells with double fine gold lines, cream Georgia title-case quest names and readable Sans prose. Header/act/control bands now share dark recessed stone/blackened metal and selected warm-gold treatment; the old flat gray bands and rounded gray cards are gone. The illustrative act tabs, fictional Track button and quest art mapping remain absent. Quiet play stays a restrained 3px instrument.

The generated frame uses nine-slicing with vertically rounded side segments, preserving sculpture proportions. The outer frame is fixed; a separately inset well clips its scrolling contents. The crown belongs to the title plate inside that well. A fixed top-rail drag region remains available after Settings scrolls down; the rich title plate also uses the existing drag bridge. Art takes no input; controls and all resize edges retain their actions. Exact provenance and hashes are in [harvest](harvest.md#original-production-material-assets).

Current app-only screenshots: [rich journal](frames/approved/exile-journal.png), [quiet play](frames/approved/exile-play.png), [ordinary Settings](frames/approved/exile-settings.png), and [minimum overlay Settings scrolled to the end](frames/approved/exile-settings-scroll-end.png). Claude screenshots retain the unchanged baseline cascade. These are rendered live controls, not the concept PNG used as a runtime mockup.

### Frame and scroll checks

Each case below was measured at scroll start, middle and end using actual app-webview/Playwright MCP. Frame and inset-well rectangles remained fixed, outer scroll stayed zero, and no horizontal page overflow occurred. The footer is reached at the end of a whole-well scroll; it need not be visible at its start.

| Surface / tool | Fixed frame x,y,w,h | Fixed inset well x,y,w,h | Observed scroll / outcome |
|---|---|---|---|
| Native overlay Settings, app-webview, viewport511×571 | 12,12,487,547 | 42,46,427,479 | Inner body0/181/362; footer reachable; fixed drag region y21 throughout |
| Same actual native window, text metrics doubled | 12,12,487,547 | 42,46,427,479 | Inner well0/416/831; footer reachable; drag region y21 unchanged; font overrides restored afterward |
| Browser overlay minimum320×240 | 12,12,296,216 | 24,40,272,174 | Inner well0/277/554; footer bottom inside the well at end |
| Browser overlay default364×384 | 12,12,340,360 | 42,46,280,292 | Inner well0/272/544; footer reachable |
| Browser ordinary Settings420×328 | 0,0,420,328 | 18,20,384,288 | Inner well0/91/182; credit reachable |
| Browser ordinary Settings420×280, normal/doubled text | 0,0,420,280 | 18,20,384,240 | Inner well0/115/230 and0/313/626; credit reachable. Ordinary plaque rails remain20px/18px; the small-overlay media rule excludes this window. |

Native MCP clicking Settings changed Rust's frame to `settings`, a positive control for the same browser input instrument. Synthetic MCP dragging delivered a pointerdown but did not change native position2021,419. No successful OS movement is claimed. The host's invoke property is immutable, so a temporary interception probe could not observe native calls and supplies no negative conclusion. Deterministic tests verify rail/title/header drag dispatch, right-click/control exclusions, passive play and the retained Claude header path. Existing native movement/persistence code was unchanged; earlier shell persistence tests pass. Actual simultaneous two-native-window propagation was not observed because only the overlay page was available; Rust both-view and both frontend selector tests cover that contract. Neither limitation adds a game-facing requirement.

The affected reference/frame/drag checks passed:7 frontend test files,41 tests, with no behavioral assertions removed. The final frontend build reports zero Svelte errors/warnings. Fresh independent browser review compared the final render with the original concept and found no blocking material mismatch:12 combinations had no horizontal overflow,4 Exile200% text cases retained reachable footers, and all32 enabled controls across minimum Settings/detail/ordinary views were reachable by focus/scroll. The scrolled Settings grip remained hit-testable with a move cursor and pointer-transparent art. These are browser observations, not successful native OS dragging. Final suites are reported by the workorder driver.

## Earlier structural matrix

The following broader counts record the earlier pass before the owner's reference-fidelity correction. They are retained as scoped history; they are not presented as a second final run against the new raster materials. The current frame/scroll evidence above covers the corrected architecture.

Browser review used the existing Vite fixture previews and Exile/Claude theme queries. App-only evidence is committed under [frames/approved](frames/approved/): [Exile journal](frames/approved/exile-journal.png), [Claude journal](frames/approved/claude-journal.png), [Exile play](frames/approved/exile-play.png), [Exile Settings](frames/approved/exile-settings.png) and [Claude Settings](frames/approved/claude-settings.png). The synthetic fixture text is deliberate. No concept PNG or game pixels are runtime artwork.

| Check | Evidence and outcome |
|---|---|
| All frames/modes | 80 Exile combinations: campaign, Next up, detail, settings, characters, collapsed and four empty states, play/edit, at320×240,364×384,480×640,760×800. No horizontal page overflow, zero-height body or unreachable footer. Whole-panel cases were scrolled before checking footer reachability. |
| Content scales | 24 edit combinations at320×240/364×384, supported query scales1/1.33/2, across Next up/campaign/settings/detail. Six more cases exercised0.75 and2 by explicit runtime CSS override, not an unsupported0.75 query. No overflow or stranded footer. |
| Text enlargement | Four frames at364×384 with actual text font/line metrics doubled. Settings/detail initially clipped the footer; fixed with a presentation-only ResizeObserver selecting whole-panel scrolling when chrome leaves under72px. Retested successfully. The independent visual review also caught plaque order changing in block flow; the decorative plaque now precedes Header in both layouts, with meaningful control order preserved. |
| Long content | Invented636-character character/quest/error text at320×240 retained in full, wrapped/scrolled without horizontal overflow; footer reachable. No lorem/game text or additional state fields introduced. |
| Ordinary Settings | All six update and seven install fixtures in both themes:26 combinations at420×328. No horizontal overflow; Exile native/custom controls at least40px, content and footer reached by page scroll. Claude retains original dimensions/type. |
| Controls and focus | Checked settings/detail/character controls. Fixed small All and manual-tick targets to at least32px in Exile; ordinary select/toggle/button targets40px. Focus uses an inset independent outline within clipping. Exile slider/toggle/input/action accents are warm gold; Claude stays cyan. Existing reduced-motion tests retain the static2.5s highlight and one announcement. |
| Opacity/backing | White, black and a generated busy striped backing at opacity0/.9/1: nine combinations. Computed panel alpha changed while title text remained opaque. At0 the field is transparent; no contrast guarantee is made for arbitrary backgrounds. Reading fields remain quiet at the normal backing. |
| Claude isolation | Same five primary frames in both themes at identical364×384 bounds: original Sans/SC and no Exile art in Claude; Cinzel/material art only Exile. Shared baseline token values and original component/style rules remain intact. Sibling font/CSS parity tests pass; added Cinzel files do not change their font selection or palette. |

That pass fixed scoped-CSS specificity, minimum targets, large-text scrolling and plaque order. Its independent review did not catch the material mismatch subsequently identified by the owner; the correction and actual fixed-frame checks are recorded above. Optional quest portrait niches remain omitted because there is no authoritative mapping. These scoped checks do not certify every font, display or state.

## Contrast calculations

Ratios use sRGB relative luminance for the actual opaque Exile token pairs. Text target4.5:1 and essential boundary target3:1 are met by these representative worst/state pairs:

| Pair | Ratio |
|---|---:|
| Bone primary / reading well | 13.32:1 |
| Secondary / raised surface | 8.38:1 |
| Muted / hover surface | 5.51:1 |
| Gold title / raised surface | 6.15:1 |
| Dark primary-action text / pressed gold | 6.01:1 |
| Steel essential outline / hover surface | 3.63:1 |
| Gold control/focus / reading well | 7.22:1 |

## Native UI evidence and instrument limits

The local debug binary was discovered from Cargo compiler-artifact JSON, not the release display name: `apps/quest-tracker/target/debug/quest-tracker.exe`, SHA256 `ec33309fa94bc2a7c44fe92663f8085be25982074bc17ea5940fdbee27979be5`. It was launched with CDP and a disposable external TEMP store and empty synthetic log, with screen reading and update checks off. No installed executable or user store was replaced. This debug app reads the current frontend from the existing local Vite server; the final frontend build passes with zero Svelte errors/warnings.

Actual WebView2 observations: Exile default after an unavailable/invalid test store; Claude saved through the normal command, normal close and restart; clean reload retained Claude, opacity0.9, scale1, disabled screen/update preferences and the synthetic log. Switching Exile↔Claude retained the settings frame and364×384 native bounds, changed the title font and removed Exile art in Claude. A saved320×240 window opened at320×240; Exile Settings used whole-panel scrolling with no horizontal overflow and reachable footer.

Setup corrections are retained as limitations: an initial store inside Git was correctly refused, and a raw settings object was correctly quarantined because the store requires its existing schema/data envelope. The final external store was written by the app and reloaded cleanly. A native resize gesture did not change bounds and is not recorded as a successful resize test. Existing capabilities correctly refused resize/close IPC; no capability was added. Native screenshots remain local because the desktop backing is outside this app's artwork.

The hash above identifies the native debug host used with the live Vite frontend, not a published release containing the final frontend. Final production frontend compilation is verified separately. Simultaneous two-native-window propagation and synthetic OS dragging remain unmeasured as described above; no new app permissions or test-only commands were added to force them. Only Tauri/app-webview and Playwright MCP were used for the final reference/frame checks. No game-facing check is part of this UI-only scope.
