# Approved UI adoption review

**Latest:** the Exile theme is wired to the [neutral steel/masonry kit](steel-masonry-kit/HANDOFF.md) and the minimum-size whole-scroll corner overlap is fixed; the first section below records how that was measured, its Fix 2 part (2026-11-02) the readability floors, the inside-only diamond fill and the hidden play scrollbar that followed the owner's Live 2 remarks, its Fix 3 part (2026-11-02) the Next up sections that followed the owner's Live 3 remark, its Fix 4 part (2026-11-02) the inset line inside a section the owner asked for after Live 4, and its owner-check part what the four native-app sessions found. Every later section is history: it describes the warm prototype the owner rejected as too yellow, and is not evidence about the kit in the application.

## Steel/masonry kit wired, measured by artwork — 2026-11-01

### What changed

The Exile frame is the kit's `frame-neutral-steel.png`, unchanged, as a nine-slice (224 source px, drawn 48 px, constant on screen at every scale) over one `cover` crop of `masonry-charcoal.png`. It is flush with the window edge: Live 1 found the rails floating about 13 px inside the window, in the 12 px transparent margin the overlay kept for Claude's shadow, and the owner asked for the frame to reach the edge. In Exile the overlay now has no margin, and in the overlay and the ordinary Settings window each side's outset (9 / 8 / 13.5 / 8.5 px, top, right, bottom, left) sits just under the PNG's own transparent padding at 48 px, so only that padding leaves the window. The eight edit-mode resize handles lie just inside the window's edge, and the drag strip sits below the top one. Claude keeps its margin and shadow. Palette, diamonds and controls come from the kit's `palette.json`, `components/*.svg` and previews; the diamonds are the kit's six status SVGs reproduced as inline markup, with the same accessible names as before. No art was generated, edited or extracted, and no asset request was raised. The compact title bar and the footer sit between the corner knots and only the body scrolls inside the rails; when that leaves too little room, the whole content scrolls in a well clear of the knots.

### Fix 2: readability, inside-only diamonds, no play scrollbar — 2026-11-02

The owner's Live 2 remarks: text at the smaller scales was hard to read, a diamond's status colour should fill only its inside, and play's scrollbar cannot be used because play is click-through. Each is now a rule the runner checks on every case, scale and scroll position, Exile only.

- **Readability.** Each Exile size is the larger of its scale-1 size and a floor divided by the page zoom, so text never renders under 14 px for quest names and titles, 13 px for objectives, tips and detail text, or 12 px for anything else; the 11 px metadata, rewards, group labels, mode tag and Settings credit went to 12-13 px, and the shared size tokens carry the same floors into the components' own rules. Cinzel is kept to 13 px and up. The runner reads each text run's computed size, zoom, caps, weight and tracking, and its contrast against the worst pixel behind it in a render with every text run made transparent (4.5:1; a disabled control's text is exempt).
- **Diamonds.** The ring of every status keeps the kit's steel stroke, unfilled; in progress and done carry the kit's ivory and silver fill on an inner diamond with a gap of the well between them.
- **Play scrollbar.** In Exile play every scroller of the panel hides its scrollbar and still scrolls; edit, the overlay Settings frame, the picker and the ordinary Settings window keep the kit's 8 px bar. The runner now starts Edge without Playwright's headless `--hide-scrollbars` switch: before Fix 2 no scrollbar was drawn in any run, so the scrollbar-as-content check had nothing to see.

Baseline, the same runner with the new checks before any Fix 2 style change: 140 cases, 108 failures (54 readability, 54 play scrollbar), control detected. At scale 0.75 every case failed readability (titles 12.8 px, objectives 12 px, tips 10.5 px, metadata 8.25 px); at scale 1 the 11 px metadata and the 11 px Cinzel labels failed; 1.33 passed; at 2 one ordinary Settings label failed contrast; every play case with an overflowing scroller showed its 8 px bar. After: 0 failures.

Design pass: `impeccable` (audit, its detector found nothing, then polish) and `taste-skill` (its app-screen rules) reviewed the Exile overlay and the Settings window from the screenshots. Applied: the header tools' gap (6 to 4 px), which kept the settings gear off a row of its own at 320×240; text selection and the text fields' caret in kit colours; and the 12 px floor the type rules ask for. Declined, as outside the kit or this round: the disabled colour's 2.4:1 (WCAG-exempt and the kit's own value), token names for the kit literals, the kit's 2 px message-strip edge, its Cinzel capitals headings and toggle size, and the minimum-size detail density.

### Fix 3: Next up sections — 2026-11-02

The owner's Live 3 remark: the quest separators and the lack of clear sections made it hard to tell which quests are worth it and which optional. Before, Next up was one flat list: a quiet grey caps label above each worth group, and the kit's full divider (dark band, steel line, centre diamond) under every quest but the list's last, so the quest that ended a group drew the same divider right above the next label. Now, in Exile only and in the kit preview's order, each worth group is a section element labelled by its heading; the heading is kit gold, led by the group's worth icon; the kit divider appears once above every heading but the first, as the section break; and between two quests of one section only the divider's fine steel line is drawn, with no band and no diamond, and none under a section's last quest. Claude keeps the flat list, and the All view's act sections keep the full divider. No art was added and no colour outside the kit palette is used.

The fixtures are frozen and none has two quests in one worth group or a "Not worth it" group, so the browser preview gained a dev-only view, `?frame=next-up-sections`: all three groups with two quests each, taken from the campaign fixture's rows with their worth set to the group. The runner covers it in play and edit across the whole overlay matrix, and its sections check fails a section without a visible heading named by `aria-labelledby`, a separator under a section's last quest, a section after the first without exactly one section break above its heading, an in-section separator that draws the diamond or is more than half a break's height, and a heading that touches a separator or break.

Baseline, the runner with the sections check and the new view before any markup or style change: 160 cases, 40 failures, control detected, sections checked on 39 cases: 39 sections failures, and one case (Next up play, 320×240, scale 0.75) that did not load because a source edit landed mid-run; run again alone on the unchanged tree it failed the sections check the same way. Every Exile Next up case, 40 in all, failed: the two-group fixture with a missing heading for each group, a separator before the second heading and a missing section break; the three-section view with the same and, in addition, the full diamond divider between two quests of one group. Every other check passed. After: 0 failures.

### Fix 4: the line inside a section is inset — 2026-11-02

After Live 4 the owner asked whether the lines between quests of one section should be shorter, and chose to inset them. The fine steel line between two quests of one Exile section now starts at the quest name's left edge, after the status diamond (38 px in at scale 1: the row's 4 px padding, the 24 px mark column and the 10 px gap), and stops at the row's 4 px right padding. The section break (band, line, centre diamond) still spans the width, and so do the All view's act-section dividers; Claude is unchanged. No colour, token or art was added.

The runner's sections check gained a sixth reason, `separator not inset`: an in-section separator that starts left of its row's `.name` box (0.5 px tolerance), whose row shows no name, or that is not narrower than the narrowest section break on the page. Baseline, the runner with that reason before the style change: 160 cases, 20 failures, control detected, sections checked on 40 cases; the failures were exactly the 20 `next-up-sections` cases (play and edit, the whole matrix), each `separator not inset`, and nothing else. The two-group Next up fixture has one quest per group, so it draws no line inside a section and passed. After: 0 failures.

### The corner-safety runner

`npm --prefix apps/quest-tracker run corner-safety` replaces the rectangle checks of the earlier reviews, which passed while text crossed the knots. It renders each preview in headless Microsoft Edge, takes the frame artwork alone as a mask (alpha 64 and above), and intersects it with the painted boxes of every text line, mark, icon, control, divider and scrollbar, each clipped by its scrolling ancestors. Its matrix covers the Exile overlay surfaces (Next up and All in play and edit, quest detail, character picker, overlay Settings, the four empty states) at 364×384 and 320×240 and the ordinary Settings window at 420×328 (idle, update available, downloading, failed), at scales 0.75, 1, 1.33 and 2 with normal text and at scale 1 with text doubled, each at the top, middle and end of every scroller. It also fails a box that can never be fully seen, sideways overflow, any control in play, an edit drag strip that is covered, a focused control that is clipped or over the art, and a header or footer that moves or clips in the fixed-chrome layout. On every case it checks that the frame is flush: the rendered art comes within 2 px of each window edge, and no art pixel is cut off by the window (computed from the frame element's box, its outset and width per side, and the PNG's own padding, decoded in Node); in edit, each resize handle must be what the page hits 2 px inside the middle of its window edge or inside its corner. As a positive control, a probe placed over a real frame knot must be reported at every scale.

Results on this tree:

- `corner-safety -- --self-check` (the kit frame alone at each scale: a probe on a knot and one in the centre; a frame 12 px in from the window, reported not flush with its art 13 / 14 / 12 / 18 px from the left, top, right and bottom edges; a flush frame, which passes; since Fix 2 also a 9 px run, a `#535D64` run (2.89:1) and a small-caps run, each reported, a compliant run that passes, and a play scroller whose 8 px bar is reported and whose hidden bar passes; since Fix 3 also a two-section Next up laid out as the old flat list, reported for its missing headings, the separator before a heading, the missing break and the diamond between two quests of one group, and the section layout, which passes; since Fix 4 the section pages' rows carry a mark-sized box and a `.name`, the Fix 3 layout with its full-width fine line is reported `separator not inset`, and the same layout with the line inset to the name passes): 4 cases, 0 failures, control detected, flush checked on 4 cases, readability checked on 4 cases, play scrollbar checked on 4 cases, sections checked on 4 cases.
- `corner-safety` after Fix 2: 140 cases, 0 failures, control detected, flush checked on 140 cases, readability checked on 140 cases, play scrollbar checked on 54 cases.
- `corner-safety` after Fix 3 (the matrix now includes the three-section view in play and edit): 160 cases, 0 failures, control detected, flush checked on 160 cases, readability checked on 160 cases, play scrollbar checked on 64 cases, sections checked on 40 cases.
- `corner-safety` after Fix 4 (the inset line inside a section): 160 cases, 0 failures, control detected, flush checked on 160 cases, readability checked on 160 cases, play scrollbar checked on 64 cases, sections checked on 40 cases.
- Before the flush change, the same runner at scale 1 reported every one of its 28 cases not flush: the overlay's art 13 / 14 / 12 / 18 px from the window edges, the Settings window's 9 / 10 / 8 / 14 px, and no edit resize handle on the window's edge.
- Baseline, the same runner against the warm prototype (tag `quest-tracker-steel-masonry-base`) at 320×240: 60 cases, 204 failures, control detected. The failures are the known defect: the title bar, the footer, rows, diamonds and divider decorations painted over the frame art at every scale. (That tree's preview ignores scale 0.75, so its 0.75 cases render at 1.)

Limitation: an edit-mode quest row is a button, and at scale 2 or with text doubled its wrapped title can make it taller than the reading well, so it can never be seen whole. Such a row is checked line by line: each of its text lines and marks must fit the scroller and stay off the art, and when focused only its ring's left and right edges must be unclipped. Every other box is checked whole.

### Claude unchanged

The Claude Next up play, overlay Settings edit and quest detail edit at 364×384, and the Claude ordinary Settings window at 420×328, all at scale 1, rendered byte-identical screenshots at the base tag and on this tree, before and again after the flush change, again after Fix 2 (readability, diamonds, play scrollbar and the design pass are all Exile-scoped), again after Fix 3 (the sections are Exile-scoped; Claude's Next up keeps its flat list), and again after Fix 4 (the inset is one Exile rule).

### Screenshots

Written by `corner-safety -- --screenshots`, scale 1, in [frames/steel/](frames/steel/): [play-default](frames/steel/play-default.png) and [edit-all-default](frames/steel/edit-all-default.png) at 364×384, [detail-min](frames/steel/detail-min.png) and [all-min](frames/steel/all-min.png) in edit at 320×240 (the whole content scrolls there; the images show the top), [overlay-settings-default](frames/steel/overlay-settings-default.png), [ordinary-settings](frames/steel/ordinary-settings.png) (idle), [claude-play-default](frames/steel/claude-play-default.png), and since Fix 3 [sections-min](frames/steel/sections-min.png), the three-section view in play at 320×240. Compared beside the kit's `previews/play.png`, `edit.png`, `overlay-settings.png`, `ordinary-settings.png` and `secondary-views.png`, they show the same frame, well, palette, title bar, diamonds and controls; the text is the fixture's, not the previews' illustrative strings. Regenerated after the flush change: in every Exile image the steel rails touch the image's four edges. Regenerated again after Fix 2: the metadata, group labels and footer render at 12 px or more, the in-progress and done diamonds are filled only inside their steel ring, and at 320×240 the header tools fit on one row, so the first quest shows at the minimum size; the Claude image is byte-identical. Regenerated after Fix 3: `play-default` shows the gold Worth it and Optional headings with their icons and the kit divider between the two sections, and `sections-min` the first section at the minimum size, its heading at the top and a fine steel line between its two quests; beside the kit's `previews/play.png` the section order matches. Only these two images changed; the others are byte-identical. Regenerated after Fix 4: in `sections-min` the fine line between the two Worth it quests now starts under the quest name, clear of the diamond, and stops at the row's right padding; `play-default` still shows the section break across the full width, and it is byte-identical, because its fixture has one quest per section and so no line inside one. Only `sections-min` changed.

### Owner check in the native app

Four live sessions ran the release build in place against the owner's own game data, with no input sent to the game. A failed check below is not an open item: each was the start of the round that fixed it, and the next session checked that fix.

- **Live 1 (2026-11-01).** The title-bar drag moved the window and a second drag from the overlay Settings top strip moved it again; a native resize took it down to the 320×240 minimum and not below; switching to Claude showed no kit art at the same bounds, and switching back brought the frame back; the ordinary Settings window opened in the Exile theme with the kit art. The owner found the style right but the frame not reaching the window edge (it sat in the 12 px margin), which failed the look check and led to the flush change. The minimum-size corner screenshots could not be taken: with no game running there was no quest list.
- **Live 2 (2026-11-01).** With the game running, the frame met the window edge exactly (its art box equal to the window). At 320×240 the owner saw no text, diamond or control over the frame's corners in the six screenshots. The look check failed on the owner's remarks: small-scale text was hard to read, a diamond's status colour filled the whole mark rather than its inside, play's scrollbar is of no use in a click-through mode, and a design pass was wanted; Fix 2 answered all four.
- **Live 3 (2026-11-02).** The diamond's outer ring was unfilled with the status colour only inside it, and in edit at the default size the owner confirmed both that and that the text was easy to read; at 320×240 in play at the owner's own scale the scroller drew no scrollbar while its content still overflowed, and the owner confirmed the bar was gone and the text readable. The owner's remark that the separators and the missing sections made worth hard to read led to Fix 3.
- **Live 4 (2026-11-02).** In edit, Next up showed three labelled sections (Worth it, Optional, Not worth it), two section breaks and no line under a section's last quest; at 320×240 in play the same held, every heading rendered at 13 px or more and the first heading was whole at the top. The owner passed both. Afterwards the owner asked whether the lines between quests of one section should be shorter and chose to inset them; that inset (Fix 4, above) is an owner decision checked by `corner-safety` and the regenerated screenshots, not a look a live session checked.

In every session the owner's settings were restored byte-identical afterwards.

## Current compact steel implementation — 2026-11-01

### Reference fidelity

The authority is [quest-tracker-overlay-approved.png](../../ui-design-system/quest-tracker-overlay-approved.png), the owner's original approved AI close-up, SHA256 `ec36548bdbf2dcd33ef41cf984c84ffc3a8b51ffba04202963faa7c65cbdd874`. Earlier passes selected the wrong ornate journal study. Their visual approvals did not establish fidelity to this steel design; this correction addresses our error, not a new owner design direction. The full reference is documentation only. Runtime uses the separately generated transparent steel-frame reconstruction and quiet darkened stone texture; all text, status symbols, dividers and controls are live UI. Exact asset/prompt provenance is in [harvest](harvest.md#approved-steel-frame-prompt).

The first-screen checkpoint was captured with Playwright MCP at364×384 before expanding the remaining states: [first-screen-compact-steel.png](frames/approved/first-screen-compact-steel.png) and [edit checkpoint](frames/approved/first-screen-compact-steel-edit.png). A fresh independent reviewer compared both to the canonical close-up and returned PASS with no blocking findings; the driver independently agreed. Those checkpoint captures establish steel/layout fidelity; their inner status icons are superseded by the owner's latest all-six diamond-treatment decision below. Character/class/level, worth headings and reward labels remain visible in play. Long current tips continue through scrolling; future detail steps are not invented as active objectives.

| Reference feature | Implemented and observed |
|---|---|
| Thin weathered double steel rails and intertwined corner knots | Original transparent `tracker-steel-frame.png`; slice224/destination64px/stretch/outset12px, no fill. No runtime gold architecture, crest or full concept image. Ordinary Settings uses the same frame with0 outset. |
| Compact Quest Tracker / Act bar | Narrow live Cinzel bar. Roman Act is the sole received progress-derived act, including empty Next up; absent with zero/multiple acts. No player-zone field is invented. Compact character context follows it. |
| Flat separated quest sections | Computed row background transparent, box-shadow none, top/left/right borders0; fine1px bottom rule and small central diamond. No rounded boxed cards or inset card bevels. |
| Gold serif titles / white serif current objectives | Georgia19/25px weight400 titles, gold `#f0d779`; Georgia16/22px current objectives, white `#eeece5`; secondary Georgia14/20px tips. Metadata/rewards remain secondary. |
| Diamond status marks | Six distinct live CSS diamond fill/outline/pattern treatments without inner icons in Exile rows/detail; exact status names/tooltips and existing worth/manual/inferred marks retained. Claude keeps its six original icons. |
| Small footer | Passive actual configured-shortcut · Edit in play, zero controls. Edit keeps the same return command, compact shortcut · Play text and full accessible description. |
| Art-only outer border | Exile legacy panel `::after` ring is hidden and outer box-shadow none. Claude keeps its original ring, fonts and layout. |

Final app-only views: [Exile play](frames/approved/exile-play.png), [All/edit](frames/approved/exile-journal.png) (the filename is historical), [ordinary Settings](frames/approved/exile-settings.png), [ordinary Settings end](frames/approved/steel-ordinary-settings-end.png), [characters](frames/approved/steel-characters.png), [minimum detail](frames/approved/steel-detail-min.png). These replace the old gold-journal screenshots at the established current filenames.

### Diamond status revision — 2026-11-01

The owner replaced the earlier inner-icon choice: “use the diamond fill instead of icons” and “Use distinct diamond treatments for all statuses.” Status meanings remain the six existing enum values. Each Exile mark has one exact `STATUS_LABEL` accessible name and matching hover title, no inner icon/SVG/glyph, and a decorative16px diamond inside a22px status slot. Claude's original labelled icons remain unchanged; separate manual, inferred and worth marks remain.

| Status | Diamond treatment |
|---|---|
| In progress | Solid gold, single outline |
| Available | Hollow gold, double outline |
| Locked / not yet available | Hollow, thin muted single outline |
| Done | Muted silver fill, double outline |
| Skipped | Diagonal hatch, single outline |
| Unknown | Hollow, dotted outline |

The [six-status app comparison](frames/approved/exile-status-diamonds.png) uses the unchanged campaign fixture at364×1024, so every mark fits in one actual app capture. Top to bottom: done, skipped, locked, in progress, available, unknown. Filled versus hollow, single versus double/dotted edges and hatching distinguish the states beyond colour alone. Current default play/edit captures were refreshed, plus [minimum All/edit](frames/approved/steel-diamonds-min.png) and minimum detail. The previous18-case geometry and53-control evidence below is retained for unchanged layout rather than repeated.

Playwright MCP measured all six marks with zero inner icons, exact accessible names and matching titles. Default play remains340×360 at12,12, no horizontal body overflow, zero controls and footer at29,324,306×32. Minimum320×240 detail and All/edit have no horizontal body overflow and a hittable fixed drag region; after whole-well scrolling to462/544px respectively, both footers fit at y180–212 inside the y22–212 reading well, and the fixed drag grip remains hittable. Claude comparison at364×384 has zero diamond elements and the six original status icon IDs/labels. The new row/detail/theme-transition target was red before implementation; focused approved/row/detail/inferred checks then passed22 tests in4 files, and the production build reported0 errors/0 warnings. The only browser console error was the development server's missing favicon404. No native movement or in-game measurement is inferred.

### Frame and scroll checks

Playwright MCP exercised18 focused overlay combinations after font loading and two animation frames for layout/ResizeObserver settling. All expanded cases had no horizontal well/page overflow and a reachable footer inside the inset well at scroll end. The outer frame stayed fixed; the fixed edit grab region remained hit-testable after scrolling. Play had zero controls and no drag region. Scrolling uses the body normally and the whole clipped well when enlarged chrome leaves insufficient body space. Header/tool groups and settings input rows wrap at narrow effective widths.

| Surface / viewport / scale | Frame x,y,w,h | Well x,y,w,h | Scroll start / middle / end; footer at end |
|---|---|---|---|
| Next up/play364×384,1× |12,12,340,360|29,22,306,334|Body0/22/43; footer29,324,306,32 |
| All/edit364×384,1× |12,12,340,360|29,22,306,334|Body0/176/351; same fixed footer |
| Settings/edit320×240,1× |12,12,296,216|29,22,262,190|Whole0/266/531; footer29,180,262,32 |
| Detail/edit320×240,1× |12,12,296,216|29,22,262,190|Whole0/231/462; footer29,180,262,32 |
| Next up / All play320×240,1× |12,12,296,216|29,22,262,190|Body0/106/212 and0/244/487; footer29,180,262,32; controls0 |
| No log / no character edit320×240,1× |12,12,296,216|29,22,262,190|Body0/69/138 and0/47/94; footer inside well |
| Picker / act done edit364×384,1× |12,12,340,360|29,22,306,334|No scrolling required; footer inside well. Collapsed separately retained its badge at320×240. |
| Settings/edit364×384,2× |12,12,340,360|46,32,272,308|Whole0/484/968; footer46,236,272,104 |
| Next up/play364×384,2× |12,12,340,360|46,32,272,308|Whole0/250/499; same footer rectangle; controls0 |
| Detail/edit364×384,1.33× |12,12,340,360|34.6,25.3,294.8,325.4|Whole0/267/534; footer34.6,309.2,294.8,41.5 |
| Settings/edit364×384,0.75× runtime override |12,12,340,360|24.8,19.5,314.5,340.5|Body0/109/220; footer24.8,336.3,314.5,23.8 |
| Settings/edit364×384,text200% |12,12,340,360|29,22,306,334|Whole0/394/788; footer29,304,306,52 |
| Settings/edit320×240,text200% |12,12,296,216|29,22,262,190|Whole0/636/1272; footer29,120,262,92 |
| Detail/edit364×384,text200% |12,12,340,360|29,22,306,334|Whole0/508/1015; footer29,304,306,52 |
| Ordinary Settings420×328,normal/text200% |0,0,420,328|28,28,364,256|Well0/68/136 and0/246/492; credit28,243,364,41 and28,227,364,57; frame fixed |

Text200% doubles every HTML element's original computed font size and numeric line height, not page zoom. The ordinary Settings title measured15→30px. Scale0.75 was an explicit runtime CSS/page-zoom override; no unsupported query is claimed. Representative [2×](frames/approved/steel-settings-scale2.png), [200% default](frames/approved/steel-settings-text200.png) and [200% minimum](frames/approved/steel-settings-min-text200.png) captures show whole-well scrolling. The final ordinary Settings paint check found status text crossing the lower-left knot before scroll; increasing only its bottom clearance from28 to44px fixed it. Its controls/credit remain reachable.

Across minimum overlay Settings/detail, default picker and ordinary available/failed/unsigned/downloading states, all53 enabled controls accepted focus and could be scrolled into their clipping area; no unreachable focus target was observed. Existing deterministic tests cover exact commands, reversal/refusal states, drag dispatch/control and right-click exclusions, eight resize directions, long content, passive play, and reduced-motion2.5s feedback. Browser hit tests establish a reachable drag surface, not successful native movement. Claude comparison at identical overlay bounds retained Alegreya Sans, no Exile artwork/footer and its original ring; ordinary Claude retained its original page scrolling.

White, black and striped busy backings at opacity0/.9/1 gave matching backing/texture alpha while text stayed opacity1. Transparency does not promise contrast over arbitrary scenes. Opaque CSS-color contrast against `#090b0c`: gold quest13.81:1, objective white16.68:1, secondary9.65:1, footer8.88:1; essential divider4.11:1 and diamond11.45:1. These calculations cover the stated flat color pairs, not every textured pixel.

First-screen targeted checks passed20 tests; broader targeted checks passed44 in8 files before the last responsive/selector fixes. Fresh verification caught the old first-`h1` character assertion after the intentional static tracker heading was introduced. It now checks the actual `.header .name` for both provisional and named-negative cases, retaining level/class/act/picker assertions. Row-change and ordinary Settings targeted rerun passed18 in2 files. The production build reported0 errors/0 warnings and both root UI network/release guards passed. Final independent verification and final reference/interaction review are recorded by the workorder driver; no full-backend rerun is implied here.

No new native or in-game measurement was made for this frontend correction. The native app was closed during this matrix; previous native evidence below is historical. Free window dragging still uses the existing native bridge; successful OS movement and simultaneous two-native-window propagation are not newly claimed. No game store/settings/input, permissions or native-HUD operation was introduced.

## Historical implementation evidence — superseded visual authority

Everything below records earlier passes, including their then-current claims and screenshots. Those passes compared the wrong ornate concept and cannot establish current reference fidelity. Current screenshots at reused filenames are the steel implementation linked above; older numeric observations remain history only.

## Compact functional header and art-only border — 2026-11-01

The owner removed the added Campaign journal/crest header because it occupied quest space, and removed the extra CSS border because Exile already has a custom art frame. The attached compact tracker close-up guides this space-efficient layout. No reference image was copied into runtime artwork, no new art was generated, and the existing custom frame and stone texture remain unchanged. The crest source PNG/prompt remains historical provenance; the current panel does not import or render it.

The functional character/view/settings/collapse Header and fixed edit-mode top-rail drag area remain. The extra plaque is absent in every expanded mode/frame. Exile disables only the outer panel's legacy `::after` ring and synthetic inset frame shadows; its custom art frame remains. Ordinary Exile Settings also removes its synthetic inset frame shadows. Button/control focus outlines, the clipped scroll well, resize edges and Claude's original ring/shadow remain intact.

Playwright MCP measured the same browser fixture before and after at364×384:

| Surface | Before | After |
|---|---|---|
| Exile campaign/edit | Decorative header84px; body starts y218; outer frame12,12,340,360 | No plaque/crest; body starts y134; same outer frame;84px header space reclaimed |
| Exile play/edit outer CSS | `::after` displayed and three inset frame shadows | `::after` display none; box-shadow none; custom art present in both modes |
| Claude play/edit | Original ring displayed and white5% inset highlight | Exact same ring/shadow; no Exile art |
| Ordinary Exile Settings | Two inset frame shadows behind custom art | Box-shadow none; custom art retained |

At320×240, Next up/Settings/detail kept frame12,12,296,216 and well24,40,272,174 fixed. Their scroll start/middle/end values were0/173/346,0/277/554 and0/276/551 respectively; every footer reached y174–214 inside the well at the end. At364×384, the frame stayed12,12,340,360 and well42,46,280,292; body scroll values were0/122/244,0/218/436 and0/205/409, with footer y298–338 visible throughout. All six cases had no horizontal page overflow, and the fixed drag grip remained hit-testable after scrolling. Updated journal/play/ordinary Settings/minimum scroll-end screenshots show the compact functional header and art-only outer border.

Fresh headerless Settings checks used Playwright MCP at `/?frame=settings&mode=edit&theme=exile` with every HTML element's computed font size and numeric line height doubled from its unmodified value. Normal line heights remained proportional to the doubled font. The Theme label measured13/16px before and26/32px after; help text and the functional Header measured14/20px before and28/40px after. Browser scale, root CSS zoom and panel CSS zoom all stayed1. Measurements followed font loading and responsive layout settling; this is200% text, not page zoom.

| Current headerless Settings at200% text | Scroll start / middle / end | Fixed frame and inset well (x,y,w,h) | Footer at scroll end | Overflow and fixed drag grip |
|---|---|---|---|---|
| Exile edit,320×240 | Whole well0 /576 /1151 | Frame12,12,296,216; well24,40,272,174 at all three positions |24,99,272,115; fully inside well and hit-testable | Page width320; well/body scroll width equals client width272; grip hit-testable at all three positions |
| Exile edit,364×384 | Whole well0 /504 /1008 | Frame12,12,340,360; well42,46,280,292 at all three positions |42,223,280,115; fully inside well and hit-testable | Page width364; well/body scroll width equals client width280; grip hit-testable at all three positions |

In both200% cases the footer scrolls with the whole well, becomes fully reachable at the end, and stays clipped while below the viewport at the start/middle. Both retained custom art, zero plaque/crest elements, `::after` display none and outer box-shadow none. These fresh browser checks leave the native app closed and do not touch its store.

The new header-absence assertion failed before the change and passed afterward. All existing functional Header, mode/theme, passive-play and drag/control-exclusion assertions remain. Native app-webview endpoint9333 was unavailable and no tracker process was running; the application was left closed. These are browser observations, not a new native or in-game measurement. The earlier native/crest captures below describe the previous UI stages.

## Selected-theme mode consistency — 2026-11-01

The selected theme now chooses the expanded-panel material independently of mode and frame. Exile retains its carved stone/gold frame, textured wells, ornament and serif quest titles in play/edit, Next up/All, Settings/detail/picker and empty states. Claude retains its original presentation. Mode changes interaction and controls only; returning to play still closes edit-only frames as before. Earlier compact-play captures and counts below are historical, not the current mode contract.

The new regression failed before the correction (`expected journal, received compact`) and passed after removing mode/frame from material selection. It checks both themes across every frame/mode, preserved commands and control-free play, and no `update_settings` request during mode/frame changes.

Actual Tauri app-webview MCP observed the following in the disposable test profile after hot reload:

| Selected theme | Mode / frame | Expanded material | Decorative frame | Controls |
|---|---|---|---|---|
| Exile | edit / settings | journal | present | 13 |
| Exile | play / empty-waiting | journal | present | 0 |
| Claude | edit / settings | claude | absent | 13 |
| Claude | play / empty-waiting | claude | absent | 0 |

`window.outerWidth/outerHeight` stayed582×703 throughout; opacity0.9, scale1, disabled screen reading, hotkey and synthetic log path were unchanged. The app was restored to Exile edit/Settings after inspection. No live game or installed/user store was changed.

Playwright fixture checks at that stage covered Next up and All at320×240 and364×384: the first quest title remained fully inside the well, there were no play controls and no horizontal overflow. Before the later header removal, reducing the crown to48px at short viewports left the first Next up card fully visible (y224–336 inside the well y46–338). Those crown measurements are historical; the crown is now removed uniformly across modes. The [current Exile play screenshot](frames/approved/exile-play.png) records the retained custom frame without the extra header.

## First-screen critique — 2026-10-31

Reviewed the rendered 364×384 compact Next up/play and campaign/edit screens before extending the material system. The compact frame has the specified 3px steel/masonry trim and a quiet uniform reading field. Existing status shapes, rewards, steps and tips remain visible. The rich shell has original carved corners, a steel inner rail, gold inlay and lantern-astrolabe relief; it is visibly distinct from the gameplay instrument.

Two classes needed correction: the crest/title plate used too much of the default window's height, and journal rows still had the flat compact anatomy. Reduce the crest before content at short heights; add recessed row wells/title rails only to rich surfaces. Long titles and steps must wrap, and edit targets need 32px minimums. First local app-only evidence: `.claude/workorders/approved-first-compact.png` and `approved-first-rich.png` (local review captures, not durable links).

The user-wide impeccable/taste skills were absent during planning; no plugin was installed into this repository. This explicit first-screen critique and the final matrix below are the manual substitute.

## Reference-fidelity correction — 2026-11-01

The owner rejected the initial vector/gray-tile interpretation above as unlike the approved original and reported the lower frame floating over scrolled content. That feedback was correct: the first pass preserved functions but did not reproduce the physical material language, and its decoration sat above a whole-panel scroll area.

Reopened the original concept and compared the new representative journal beside it before expanding checks. The screen uses original generated black carved masonry/niche rails, worn warm-gold inlay, a dimensional amber lantern/astrolabe crown, subdued physical stone, rectangular recessed wells with double fine gold lines, cream Georgia title-case quest names and readable Sans prose. Header/act/control bands share dark recessed stone/blackened metal and selected warm-gold treatment; the old flat gray bands and rounded gray cards are gone. The illustrative act tabs, fictional Track button and quest art mapping remain absent. The later mode-consistency correction above retains these Exile materials in play as well.

The generated frame uses nine-slicing with vertically rounded side segments, preserving sculpture proportions. The outer frame is fixed; a separately inset well clips its scrolling contents. At this stage the crown belonged to a title plate inside that well; that extra header is now removed. The fixed top-rail and functional Header retain the existing edit-mode drag bridge. Art takes no input; controls and all resize edges retain their actions. Exact provenance and hashes are in [harvest](harvest.md#original-production-material-assets).

Current app-only screenshots: [rich journal](frames/approved/exile-journal.png), [Exile play](frames/approved/exile-play.png), [ordinary Settings](frames/approved/exile-settings.png), and [minimum overlay Settings scrolled to the end](frames/approved/exile-settings-scroll-end.png). Claude screenshots retain the unchanged baseline cascade. These are rendered live controls, not the concept PNG used as a runtime mockup.

### Frame and scroll checks

Each case below was measured at scroll start, middle and end using actual app-webview/Playwright MCP. Frame and inset-well rectangles remained fixed, outer scroll stayed zero, and no horizontal page overflow occurred. The footer is reached at the end of a whole-well scroll; it need not be visible at its start.

| Surface / tool | Fixed frame x,y,w,h | Fixed inset well x,y,w,h | Observed scroll / outcome |
|---|---|---|---|
| Native overlay Settings, app-webview, viewport511×571 | 12,12,487,547 | 42,46,427,479 | Inner body0/181/362; footer reachable; fixed drag region y21 throughout |
| Same actual native window, text metrics doubled | 12,12,487,547 | 42,46,427,479 | Inner well0/416/831; footer reachable; drag region y21 unchanged; font overrides restored afterward |
| Browser overlay minimum320×240 | 12,12,296,216 | 24,40,272,174 | Inner well0/277/554; footer bottom inside the well at end |
| Browser overlay default364×384, after mode-consistency correction | 12,12,340,360 | 42,46,280,292 | Inner body0/260/520; footer fixed at y298–338 and reachable at every position |
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
