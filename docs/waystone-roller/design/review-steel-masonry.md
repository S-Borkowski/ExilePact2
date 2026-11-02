# Waystone Roller — the steel/masonry restyle, reviewed

The Exile theme (the Quest Tracker's steel/masonry kit) on both windows,
with the previous look kept as the Claude theme. What it is and how it is
built: [`../ui-design.md`](../ui-design.md) § "Theme". This page records
what the checks printed, the screenshots, the requested art and the design
pass, on 2026-11-02.

## Corner safety

`npm --prefix apps/waystone-roller run corner-safety` (the runner,
`apps/waystone-roller/scripts/corner-safety.mjs`, adapted from the Quest
Tracker's): every view fixture (`empty`, `ok-any`, `ok-all`, `too-long`,
`hotkey-taken`, `unmatchable`) in the main window at 800 × 860 and
560 × 480, and the Settings window's `idle`, `available`,
`install-downloading` and `install-failed` at 420 × 364; device scales
0.75, 1, 1.33 and 2 at text 100 %, and text 200 % at scale 1; every scroll
position; focus through the main window (`ok-any`) and the Settings window
(`idle`); the knot-probe control at every scale.

```
corner-safety: 80 cases, 0 failures, control detected, flush checked on 80 cases, readability checked on 80 cases
```

`--self-check` (synthetic pages: a box on a knot reported and one in the
centre not, a 12 px margin reported not flush and a flush frame passing, a
9 px run, a low-contrast run and a small-caps run reported and a compliant
run passing, at every scale):

```
corner-safety: 4 cases, 0 failures, control detected, flush checked on 4 cases, readability checked on 4 cases
```

## The previous look, pixel for pixel

`npm --prefix apps/waystone-roller run corner-safety -- --legacy-base
waystone-crafting-steel-masonry-base`: each fixture at 800 × 860 and
`too-long` at 560 × 480, and the four Settings states, rendered in the base
tag's tree and in this one with `theme=claude`, compared on decoded pixels
(the Settings window without its new Appearance section, which the base
does not have):

```
legacy-pixels: 11 cases, 0 differ
```

## Screenshots

In [`frames/`](frames/), at device scale 1, written by `--screenshots`:

- `exile-main-default.png` — `ok-any`, 800 × 860
- `exile-main-min.png` — `too-long`, 560 × 480
- `exile-settings-idle.png` — `idle`, 420 × 364
- `exile-settings-downloading.png` — `install-downloading`, 420 × 364
- `claude-main-default.png` — `ok-any`, 800 × 860, the previous look
- `claude-settings-idle.png` — `idle`, 420 × 364, the previous look

## Requested art

Three elements have no counterpart in the kit; each is a marked CSS
placeholder in Exile until the owner delivers its file
([`ASSET-REQUESTS.md`](ASSET-REQUESTS.md), with today's element in
[`reference/`](reference/)):

| Id | File | Element | Status |
|---|---|---|---|
| `wr-mark-want` | `mark-want.svg` | the Want mark of a row and of the parts list | placeholder |
| `wr-mark-avoid` | `mark-avoid.svg` | the Avoid mark of a row and of the parts list | placeholder |
| `wr-update-dot` | `update-dot.svg` | the header Settings button's update mark | placeholder |

## Design pass

Both skills were run on the Exile screenshots above: `impeccable` (audit,
then polish) and `taste-skill:design-taste-frontend` (its app-screen
review). Neither skill's own tooling replaced the runner: the impeccable
detector (`impeccable detect`) found nothing in the changed files; the
taste-skill inspector was not run, its review was done on the screenshots.
Each finding, with what was done:

- impeccable audit, implementation integrity: the footer's string box sat
  3 px left of the filter field above it (the page area keeps 3 px inside
  its scroller, the footer did not) — applied: the footer's side padding
  is 11 px, so the two share one left edge.
- impeccable audit, accessibility: text contrast, focus rings, the
  disabled Want segment — no change needed: the runner's readability and
  focus checks pass on every case; an earlier draft's struck-through Want on
  the steel fill (2.9:1) was already fixed to the kit's edge colour.
- impeccable audit, theming: `steel.css` carries hex values rather than
  tokens — declined: `theme/tokens.css` and `theme/exile.css` are byte
  copies of the Quest Tracker's and may not gain tokens; every literal is a
  kit colour, which `steel-kit.test.ts` enforces.
- impeccable polish, the minimum window: the pinned footer takes three
  fifths of the 560 × 480 window when the search is too long — declined:
  the footer stays in view by design (unchanged behaviour), and it scrolls
  itself instead of covering the list.
- taste-skill, one left edge / dead space: "Wanted mods" wraps to a second
  filter row at 800 px, leaving its right side empty — declined: a tighter
  gap was tried and still wrapped; refitting the row is a layout change,
  not a theme.
- taste-skill, colour only on data, status and the one primary action:
  Copy shares the steel fill of the chosen segments — declined: the kit
  draws controls in steel grey and never fills one in gold.
- taste-skill house rules (no thin rules, no monospace, no small caps
  labels) — declined: the brief pins the kit, whose rails, dividers and
  Cinzel labels these are, and the search string stays the system
  monospace by the workorder's own rule.

After the applied change the runner, the legacy comparison and the
screenshots were run again; the summaries above are from that run.
