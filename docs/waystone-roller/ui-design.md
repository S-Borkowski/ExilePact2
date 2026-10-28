# Waystone Roller — UI Design

The Waystone Roller is one ordinary desktop window (Tauri 2, Svelte 5,
TypeScript), not an overlay. The player sets rules in it — mods to want,
mods to avoid, a tier range, rarities and corruption — and the app turns
them into one Path of Exile 2 search string. Pressing the shortcut
(`Ctrl+Alt+W` by default) while playing copies that string; the player
pastes it into the stash or a vendor's search box. The window is where the
rules are made and checked; it is not looked at while playing.

This document is the direction the components follow. The critique of it is
in [design/review-direction.md](design/review-direction.md), the audit of the
built page in [design/review-ui.md](design/review-ui.md).

## Principles

- **Rust decides, the UI renders.** The shell sends one `View` (event
  `wr:view`, command `get_view`); the page shows it and sends commands
  (`update_selection`, `copy_now`, `set_hotkey`). The page never builds,
  measures or shortens the search itself. The one thing it keeps of its own
  is the text typed in the mod list's filter.
- **The string is the product.** Everything else on the page exists to make
  the string at the bottom right. The footer that shows it is always in
  view.
- **Never colour alone.** Every state is also told by text or shape: the
  selected choice of a row is a filled segment with its word, a wanted row
  carries a `+` mark and an avoided one a `−`, the meter says "over by N",
  an error status starts with `!`.
- **Refuse, never truncate.** Over 250 characters, the page shows what each
  part of the search costs so the player can see what to drop; it never
  offers a shortened string.

## Theme

A byte-identical copy of the quest tracker's `theme/tokens.css` and
`theme/fonts/` (Alegreya Sans, Alegreya Sans SC; SIL Open Font Licence), so
the two apps read as one family. The test `theme_tokens_match_quest_tracker`
holds the copy to the original. Tokens used:

| Use | Token |
|---|---|
| Window background | `--color-panel-bg` |
| Raised surfaces (filter row, footer) | `--color-panel-raised` |
| Sunken surfaces (search string box, inputs) | `--color-panel-sunken` |
| Dividers, borders | `--color-panel-divider`, `--color-panel-border` |
| Text | `--color-text-primary`, `-secondary`, `-muted` |
| Want (selected segment, `+` mark) | `--color-status-available` |
| Avoid (selected segment, `−` mark), over the limit, error status | `--color-action-danger` / `--color-warning` |
| Focus ring, active filter | `--color-edit-accent` |
| Primary button (Copy) | `--color-action-primary-*`, `--color-text-on-light` |
| Disabled | `--color-action-disabled-bg`, `--color-action-disabled-fg` |
| Headings | `--font-family-caps` (small caps), `--font-weight-title` |
| Corners | `--chamfer-button`, `--radius-button` (square, as the tracker) |

Monospace (the search string, tokens, part texts): the system's
`ui-monospace, "Cascadia Mono", Consolas, monospace` — no font is added.
Sizes from the tokens: body `--size-row`, secondary `--size-step`, captions
`--size-caption`; spacing on the 4 px grid (`--space-unit`).

## Window

Titled "Waystone Roller", resizable, in the taskbar; about 800 × 860 by
default (the list then shows eight or more rows even with the over-limit
parts open), usable from 560 × 480. The page is a column: header, filter row,
mod list (the part that scrolls), footer (sticky at the bottom). The list
never shrinks below about four rows; on a window too short for that, the
whole page scrolls and the footer stays pinned at the bottom.

## Header

- Title "Waystone Roller" in small caps.
- One line under it: "Mods from poe2db.tw, fetched <date>" (`data.source`'s
  host, `data.fetched` as written).

## Filter row

Four groups on one row, wrapping on a narrow window; each group has a
visible label.

- **Tier**: two selects, "from" and "to", 1–16. The "from" list offers no
  value above the current "to", the "to" list none below "from", so the
  page cannot ask for an empty range.
- **Rarity**: three toggle buttons, Normal, Magic, Rare (`aria-pressed`,
  a check mark in the pressed state). All three off and all three on both
  mean "any rarity"; the shell decides that.
- **Corruption**: a segmented choice of Any / Corrupted / Not corrupted.
- **Wanted mods**: a segmented choice of Any / All, with a short caption
  under it that follows the choice: "any one is enough" or "all must be
  there".

The row fits one line from about 740 px wide; the groups never stretch their
controls.

Every change sends `update_selection` with only the keys it changes (the
whole `rarity` object for a rarity toggle).

## Mod list

- A text filter above the list ("Filter mods"), matching a mod's lines, its
  affix name and its token, case-insensitive. A count says how many mods
  match. Filtering only hides rows; it never changes a selection.
- Beside the filter, a **Chosen only** toggle (shows only wanted and
  avoided mods) and a **Clear choices** button (sends both lists empty;
  tier, rarity and corruption stay; disabled while nothing is chosen).
- Three groups, each with a heading, its count and how many of its mods are
  wanted and avoided: **Prefixes**,
  **Suffixes** (`group: waystone`, by `kind`) and **Desecrated**
  (`group: desecrated`; each row also shows its affix name and whether it is
  a prefix or a suffix).
- A row: its label (the first line, numbers shown as `#`), any further
  lines below it in secondary text, its tier count ("3 tiers"), its token in
  small monospace, and a three-way choice **Want / — / Avoid** (a radio
  group named after the mod; "—" reads "Neither" to a screen reader).
- A wanted row shows `+` before its label, an avoided row `−`, so the
  choice is visible in the label column too. Want and Avoid are filled
  (green, red) when chosen; a chosen "—" is only outlined, so untouched
  rows stay quiet.
- A mod with no token (`token: null`) cannot be told apart from another mod
  on its own: its Want is disabled and `why_not` is shown under the label.
  Avoid stays available. A mod the result lists in `unmatchable` is marked
  "Cannot be matched" on its row.
- Choosing Want or Avoid sends both lists in one `update_selection`, the
  mod moved out of the other list, so a mod is never in both.

## Footer (always in view)

- **The search string**, monospace, selectable, in a sunken box; "Nothing
  selected yet" (muted) when the result is empty.
- **The meter** "N / 250": the string's length, or the over-long length
  when too long, then the limit. Over the limit it turns to the error
  colour and adds "over by N".
- **Parts**, only when too long: one row per part (rarity, tier, want,
  avoid, corruption), its text in monospace and its length, in the search's
  order; the longest part is marked "longest", so the player sees which rule
  costs the most.
- **Unmatchable**, only then: "Some wanted mods cannot be told apart from
  others" and the mods' labels.
- **Copy** (primary button). Enabled only when the result is `ok`; when it
  is not, the reason sits beside it. Copy and the shortcut run the same
  path in the shell.
- **The hint** "<shortcut> copies this while you play" (the view's
  `hotkey`, `Ctrl+Alt+W` by default).
- **The status line** (`role="status"`, polite), the shell's last message;
  an error starts with `!` and takes the error colour.
- **Shortcut**: a text field holding the current shortcut (caption "e.g.
  Ctrl+Alt+W") and a "Set" button; Enter in the field sets it too. The shell answers with a status
  when the shortcut is taken and keeps the old one.

## States (one view fixture each, `fixtures/views/`)

| Fixture | What it shows |
|---|---|
| `empty` | Nothing selected: empty-string placeholder, 0 / 250, Copy disabled |
| `ok-any` | Want-any with tier, rarity and avoid: a string, Copy enabled |
| `ok-all` | Want-all with corruption |
| `too-long` | Over the limit: error meter, every part with its length, Copy disabled |
| `unmatchable` | Want-all including a mod with no token: disabled Want, `why_not`, the unmatchable notice |
| `hotkey-taken` | The shortcut refused: the error status naming it |

## Browser preview

Outside Tauri (`npm run dev`, port 1421) the page renders a fixture chosen
by the query, `http://127.0.0.1:1421/?view=too-long` (default `ok-any`).
Commands are logged; a selection patch is echoed into the shown selection so
the controls respond, but the string and the meter stay the fixture's — the
preview has no engine.

## Accessibility

- Native controls throughout (select, radio, button, text input) so
  keyboard and screen reader behaviour comes from the platform.
- A visible focus ring (`--color-edit-accent`, 2 px) on every control.
- Text contrast from the tracker's tokens (primary and secondary text on
  the panel background are above 4.5:1).
- Hit targets at least 28 px tall (`--size-button`).
- No motion beyond colour transitions under 150 ms, and none under
  `prefers-reduced-motion`.
