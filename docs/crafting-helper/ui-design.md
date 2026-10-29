# Crafting Helper — UI Design

The Crafting Helper is an ordinary desktop window (Tauri 2, Svelte 5,
TypeScript) with a Settings window and a click-through overlay. The main
window is the craft window: the player copies an item in the game with
`Ctrl+Alt+C` and grabs it with the app's hotkey (`Ctrl+Alt+X` by default) or
`Ctrl+V` in the window; the window shows the parsed item, lets the player
pick the modifiers they want with a tier floor each, previews the route the
engine chose (each step's currency, omens and odds, and the fallbacks taken
because something is blocked), and once accepted guides the craft one step
at a time, classifying each re-grab as a hit, a miss or unexpected. A button
in every state opens Craft of Exile in the default browser. The overlay
stays hidden in this version; placing it over the game is later work. This
document is the direction the components follow.

What each window does, and every command and event, is in the guide's
Command reference ([`README.md`](README.md)). The direction was critiqued
before any component existed: [`design/review-direction.md`](design/review-direction.md).

## Principles

- **Rust decides, the UI renders.** The shell sends one `View` (event
  `ch:view`, command `get_view`) to the main window and the overlay, and
  one `SettingsView` (event `ch:app-settings`) to the Settings window. A
  page shows what it was sent and sends commands; it never checks a grab,
  validates a hotkey, plans a route, judges an outcome or edits a list
  itself, and waits for the next view instead of patching its copy. The
  only state a page keeps is the text of its filter fields (the target
  filter and the currency filter), as the waystone roller's mod filter.
- **The clipboard is the player's.** No page reads it on its own: the main
  window takes the text of a paste the player made (the paste event's
  `clipboardData`), and nothing else. There is no "Read clipboard" button.
  A paste into a text field is that field's, never a grab.
- **Never colour alone.** Every state is also told by text or shape: an
  error status starts with `!`, an available update says its version in
  words beside the green dot, the grab's source is a word ("from the
  hotkey", "from a paste"), a modifier that meets a target says "target",
  an outcome is the word Hit, Miss or Unexpected, and the guide's status is
  a word.
- **Refuse, never guess.** A refused grab, hotkey, list or craft command
  leaves the last good one in place and says why in a status line.
- **One address, from Rust.** The only outside page the app opens is Craft
  of Exile, at one address held in Rust; the page sends a command that
  carries no address.
- **The overlay never gets in the way.** It does not take focus, lets
  every click through, and is hidden unless a craft is active (in this
  version it stays hidden during a craft too).

## Theme

A byte-identical copy of the quest tracker's `theme/tokens.css` and
`theme/fonts/` (Alegreya Sans, Alegreya Sans SC; SIL Open Font Licence), so
the three apps read as one family. The test `theme_tokens_match_quest_tracker`
(`src/theme.test.ts`) holds the copy to the original. The pages use the
waystone roller's token mapping ([`docs/waystone-roller/ui-design.md`](../waystone-roller/ui-design.md)
§ "Theme"):

| Use | Token |
|---|---|
| Window background | `--color-panel-bg` |
| Sunken surfaces (the grabbed text, inputs, the current step, lists that scroll) | `--color-panel-sunken` |
| Dividers, borders | `--color-panel-divider`, `--color-panel-border` |
| Text | `--color-text-primary`, `-secondary`, `-muted` |
| An available update (the dot and its words) | `--color-status-available` |
| Error status, a Miss or Unexpected outcome's word, the "stuck" and "replan" status words | `--color-warning` |
| Focus ring (2 px), the download bar, the primary button's border | `--color-edit-accent` |
| Headings, group labels, the status word | `--font-family-caps` (small caps) |
| A section's heading (Targets, Route, History) / a group label inside it (Prefixes, Chosen) | `--color-text-secondary` / `--color-text-muted` |
| Corners | square, as the tracker |

Monospace (the grabbed text, the hotkey): the system's
`ui-monospace, "Cascadia Mono", Consolas, monospace`; no font is added.
Spacing on the 4 px grid (`--space-unit`). Odds, tiers and try counts use
tabular figures (`font-variant-numeric: tabular-nums`) so a column of them
lines up.

## Windows

### Main window

Titled "Crafting Helper", resizable, in the taskbar. **960 × 680 by
default, usable from 600 × 480** (M3's 720 × 640 and 480 × 400 grow: the
item, the target list and the route do not fit side by side in 720, and a
route of eight steps with their odds does not fit in 400; 680 high still
fits a 1366 × 768 screen with the taskbar). A column of three parts:

- **The header**: the title "Crafting Helper" in small caps, then at the
  right end the **Explore more crafting on Craft of Exile** button and the
  **Settings** button, which sends `open_settings`. When a check this run
  found a newer version (the view's `update_available`), the Settings
  button names it after a divider, "Settings | Update 1.3.0", with a small
  green square beside the version; its accessible name is then "Settings,
  update 1.3.0 available". Below 760 px wide the header wraps and the
  Craft of Exile button takes its own row under the title.
- **The craft area**, which takes the rest of the window and shows one of
  the craft part's four modes (§ "Craft window").
- **The status line** last (`role="status"`, polite): the app's last
  message; an error starts with `!` and takes the error colour.

A `Ctrl+V` anywhere in the window but a text field sends `grab_text` with
the paste's text, and nothing else. The hotkey's grab and a paste are the
same grab in every mode.

### Overlay

`overlay.html`: one line, `view.overlay.line`, on a transparent,
undecorated, always-on-top window off the taskbar, about 420 × 64. The line
sits in a slightly see-through panel of the theme, one line high, cut with
an ellipsis when too long; nothing on the page can be selected or clicked,
and the window ignores the cursor. The shell shows it without activating it
only while `view.overlay.visible` is set and hides it otherwise. Nothing in
this version sets it: the craft window's guide does not show the overlay
(placing it over the game, hiding it when the game is not in front and
filling its line with the current step are later work), so its line stays
the placeholder "Next step: shown here while a craft is active."

## Craft window

The craft area of the main window. The view's craft part is always present
and in one of four modes; the page picks its layout from the mode and never
infers one from the other fields. Everything below that is in quotes as
`code` is sent by Rust (§ "Rust-produced copy"); plain quotes are the page's
own words (§ "UI-owned copy").

### Layout

From 880 px of window width the craft area is **two columns**, about 45 %
and 55 %, each scrolling on its own; below 880 px it is **one column** that
scrolls as a whole, in the order the columns list them, left first.

| Mode | Left column | Right column |
|---|---|---|
| none | The grab prompt, across both columns | |
| refused | The refusal and the grab prompt, across both columns | |
| planning | Item, then Targets | Blocked for this craft (a disclosure), then Route preview; the Start bar fixed at the column's foot |
| guiding | Current step (with its End bar at the foot) | Item now, then History |

At the minimum size (600 × 480, one column) planning reads Item, Targets,
Blocked for this craft, Route preview, and the Start bar stays fixed at the
window's foot above the status line, so the action is never scrolled away.
Guiding reads Current step, Item now, History, with the End bar fixed the
same way.

### Mode none: nothing grabbed

"Nothing grabbed yet. To grab an item:" and two numbered steps, "In the
game, hover the item and press `Ctrl+Alt+C`" and "Press `<hotkey>`, or press
`Ctrl+V` in this window", the hotkey taken from the view. This is M3's empty
state, unchanged.

### Mode refused: the grab cannot be planned

The heading "Can't plan this item", the reason as Rust sent it (a parse
failure, a text that is not `Ctrl+Alt+C`'s, or the engine's refusal, in
words), then the two grab steps of mode none headed "Grab another item:",
and the **Grabbed text** disclosure open: the line count and source ("27
lines, from a paste") and the text in M3's sunken monospace block, which
scrolls and is focusable so the keyboard can scroll it.

### Mode planning

**Item** (left). The item's name as the heading, then one line of facts:
rarity, base type, class and "item level 82" ("Rare · Sapphire Ring · Ring ·
item level 82"). The explicit modifiers follow in two labelled lists,
"Prefixes" and "Suffixes", each modifier as its lines (a hybrid modifier's
lines stay together), its tier at the right ("T3", or "—" with the
accessible name "no tier" when the data has none), and tags in words after
the lines: "fractured", "desecrated", "unrevealed", and "target" when it
meets a chosen target at or above its floor (the tag carries a check mark
shape too). An item with no explicit modifier says "No explicit modifiers."
Under the lists, the **Grabbed text** disclosure, closed.

**Targets** (left, under the item). The heading "Targets" and the help
line "T1 is the best tier; a floor keeps a roll at that tier or better.",
then:

- **Chosen**: the chosen targets, each on one row with its text (which
  wraps when long), its floor select and a **Remove** button ("Remove
  <modifier text>" to a screen reader), and a
  **Clear targets** button (disabled with none chosen). With none: "Pick
  the modifiers you want from the list below."
- A filter field, "Filter modifiers", which narrows the list below by text
  as the player types; the page's own state. With no match: "No modifier
  matches “<text>”."
- The choices for this base in three groups, "Prefixes", "Suffixes" and
  "Desecrated", each heading with its count of chosen targets ("Prefixes ·
  1 chosen"); a group with no choice for this base is left out. Each choice
  is a checkbox labelled with the modifier's text and "best T<n>" after it.
  Checking adds it with the default floor; unchecking removes it. A checked
  choice also shows its **floor select**, labelled "Floor for <modifier
  text>", whose options are Rust's: `As high as possible (T<best>)` first
  (the default), then `At least T<n>` per tier this item level reaches.
  In one column (below 880 px) the three groups scroll together in a
  sunken box past 240 px, so the route under them is not a whole base's
  modifier list away.

Every change (check, uncheck, a floor, Remove, Clear targets) sends
`set_targets { targets: [{ mod_id, floor }] }` with the whole list, `floor`
none for "as high as possible". A refusal leaves the list as it was and
says why in the status line. When a re-grab drops targets that do not fit
the new item, the status line says how many (Rust's words).

**Blocked for this craft** (right, above the route). A disclosure whose
summary always shows the count, "Blocked for this craft: 3", so the player
sees what the route was planned around without opening it. It is closed by
default and open when the previewed route has a fallback line or there is
no route, the two cases where the blocked list is the likely fix; the
player's own opening or closing then holds until the mode changes. Inside:
the help line "Changes here apply to this craft only; Settings keeps your
usual list." and the **currency picker** (§ "The currency picker") over the
effective list: each blocked entry is tagged "Settings" or "this craft",
and each Settings entry the player allowed for this craft is listed
unchecked under its name with the tag "allowed for this craft". Every
change sends `set_craft_blocked { blocked: [ids] }` with the whole
effective list; Settings is never changed from here.

**Route preview** (right, under the blocked list). The heading "Route",
then:

- With no target: "Pick a target to see a route." and nothing else.
- With a route: the route's name, its source (the blueprint's source line,
  muted) and its notes; with other applicable routes, a select labelled
  "Route" listing the previewed one first and then the others by name,
  whose change sends `choose_route { blueprint }`. Then the steps as a
  numbered list; each step shows its instruction, a "Uses" line with the
  currency and omen names, its odds as sent (`~12% (8000 of 69500
  weight)`, or `odds unknown: <reason>` with the words "odds unknown" kept,
  never a blank or a dash), and, when an earlier alternative was passed
  over, the fallback line under the label "Fallback" (small caps), as sent:
  `<used name> instead of <first alternative's name>: <names> blocked`.
- With no route: "No route reaches these targets." and one line per
  rejected blueprint, its name and the rejection in words ("<name>:
  <rejection>"), in Rust's order. The blocked list above is open, so the
  player can unblock what a rejection names.

**The Start bar** (fixed at the right column's foot): the primary button
**Start crafting**, which sends `accept_craft {}`, enabled only when the
view's `can_accept` is true; while it is disabled, a line beside it says
why ("Pick a target first." with no target, "No route to start." with no
route).

### Mode guiding

**Current step** (left). The route's name as the heading and the guide's
status as a word beside it: "Crafting" (active), "Stuck", "Replan" or
"Done". Then, once a change has been observed, the **last outcome** line,
where the player's eyes already are after a re-grab: "Last:" and the
newest History entry's outcome word and summary (the page shows that
entry; it judges nothing). Then, for active and stuck, a sunken card with:

- "Step <n>" and the try count ("try 2"); the instruction, larger than the
  body text; the "Uses" line; the odds as sent; the fallback line when there
  is one.
- The two branch lines as sent: `On a hit: <where it goes>` and `On a miss:
  <where it goes>`, where Rust names the next step by its instruction or
  says this step again, start over, give the item up, or the craft is done.
- The prompt "Take this step in the game, copy the item with `Ctrl+Alt+C`,
  then press `<hotkey>` or `Ctrl+V` here."

When stuck or replan, the reasons follow as a list under "Why:"; replan
adds "End the craft to plan again from this item." Done replaces the card
with "The craft is done." and the last summary.

**The End bar** (fixed at the left column's foot): **End craft**, which
sends `end_craft {}` in any status and returns to planning on the latest
item with the targets and this craft's blocked changes kept and the
history cleared. In done and replan it is the primary button and acts at
once. In active and stuck, where it would throw away a craft in progress,
it is a secondary button that first asks in place, without a dialog: "End
this craft? Its history is cleared." with **End craft** (sends the
command) and **Keep crafting** (closes the question; focus returns to the
first button). A view that changes the mode closes the question.

**Item now** (right): the item panel of planning, for the item as last
grabbed, with the same "target" tags.

**History** (right, under the item): "History", then an ordered list,
oldest first, one entry per observed change: "Step <n>", the instruction
(muted), the outcome as a word ("Hit", "Miss", "Unexpected"; Miss and
Unexpected take the warning colour, the word carries it), set as an
outlined caption-size label like the guide's status word (the "Last:" line
sets it the same way), and the summary
as sent, which starts `Hit:`, `Miss:` or `Unexpected:` and names the
targets gained or lost with their tier, a below-floor roll, or the reasons.
With none yet: "No step taken yet." The history is not a live region; the
status line announces each outcome.

A grab while guiding is the step's result; a grab of the same item as the
step was shown on, or of text that is not a plannable item, changes nothing
and the status line says why (Rust's words).

### The currency picker

One component, used by the craft window's Blocked for this craft and the
Settings window's Blocked currencies: the player picks by name from the
catalogue (`get_catalogue`: groups by kind, each with its heading label and
its entries, id and name, sorted by name); there is no field to type an id.

- A filter field, "Filter currencies", narrowing the list by name as the
  player types (the page's own state); with no match, "No currency matches
  “<text>”."
- A **Blocked only** checkbox that hides the unchecked entries.
- The list, scrolling past 240 px in the craft window and past 136 px
  (about six names) in the Settings window, whose fixed 440 × 720 must
  also hold Updates: each group's heading (Rust's label, small caps), then
  one checkbox per entry labelled with its name, checked when blocked, and
  the tag the caller gives (none in Settings).
- Each time the list comes into view (the window opens, or Blocked for
  this craft opens), it scrolls itself to its first checked entry when
  that entry is out of sight, so what is blocked shows without a search;
  once per showing, and never after the player has scrolled it.

The list can hold the whole catalogue; there is no other limit.

### Rust-produced copy

The page renders these as sent; `ch-app`'s tests pin each form.

| What | Form |
|---|---|
| Known odds | `~12% (8000 of 69500 weight)`: the percent as an integer from 10 up, with one decimal from 1 to under 10, `under 1%` above 0 and under 1; then the basis in brackets. 100 % is `certain (<basis>)` |
| Unknown odds | `odds unknown: <reason>` |
| Fallback | `<used name> instead of <first alternative's name>: <names> blocked`, or `<used name> instead of <first alternative's name>: does not fit this item` when no passed-over alternative names a blocked id |
| Floor options | `As high as possible (T<best>)` (the default), then `At least T<n>` per reachable tier |
| Branches | `On a hit: …` and `On a miss: …`, naming the next step by its instruction, or this step again, start over, give the item up, or the craft is done |
| History summary | starts `Hit:`, `Miss:` or `Unexpected:`, then the targets gained or lost with their tier, a below-floor roll, or the reasons |
| Refusal, rejection, stuck and replan reasons, the status line | Rust's words |
| Catalogue group labels and currency names | Rust's (English) |

### UI-owned copy

Headings: "Can't plan this item", "Grab another item:", "Grabbed text",
"Prefixes", "Suffixes", "Desecrated", "Targets", "Chosen", "Route",
"Blocked for this craft: <n>", "History", "Why:". Buttons: "Explore more
crafting on Craft of Exile", "Settings", "Start crafting", "End craft",
"Keep crafting", "Remove", "Clear targets". Labels: "Filter modifiers",
"Filter currencies", "Blocked only", "Floor for <modifier text>", "Route",
"Uses", "Fallback", "Last:", "best T<n>", "Step <n>", "try <n>", the tags "fractured", "desecrated",
"unrevealed", "target", "Settings", "this craft", "allowed for this craft",
and the status words "Crafting", "Stuck", "Replan", "Done". Empty prompts
and hints: "Nothing grabbed yet. To grab an item:", "Pick the modifiers you
want from the list below.", "Pick a target to see a route.", "No route
reaches these targets.", "Pick a target first.", "No route to start.", "No
modifier matches “<text>”.", "No currency matches “<text>”.", "No explicit
modifiers.", "No step taken yet.", "The craft is done.", "End the craft to
plan again from this item.", "Changes here apply to this craft only;
Settings keeps your usual list.", "T1 is the best tier; a floor keeps a
roll at that tier or better.", "End this craft? Its history is cleared."
and the take-this-step prompt.

### Explore more crafting on Craft of Exile

A native button in the header, in every main-window state, with the visible
text `Explore more crafting on Craft of Exile` (the owner's words) and an
outward-arrow mark after it (hidden from screen readers); its accessible
description is "Opens Craft of Exile in your web browser". It is styled as
a secondary button (border, no accent fill), as Settings is, so it never
competes with Start crafting, the window's one primary action. Its click sends
`open_craft_of_exile {}` with no arguments, and Rust opens its one fixed
address, `https://beta.craftofexile.com/?game=poe2`, in the default
browser. The page holds no address: no `<a href>` to an outside page, no
`window.open`, no change of `location`. A failure to open shows in the
status line with `!`.

## Settings window

A second, small window (`settings.html`, `src/settings/`), adapted from the
waystone roller's and drawn from the same tokens: 440 × 720 at scale 1, not
resizable, titled "Crafting Helper Settings", opened from the header's
button. It renders the shell's `SettingsView` and keeps no copy of its own;
every control sends one command and waits for the next view. Its sections
are listed in `sections.ts`, in order, each with its heading, its region and
the divider before it; a later setting is a new entry there.

From the top:

- **Settings** (the window's `h1`, small caps).
- **Grab**: the current hotkey in a `kbd`, the help line "Press Ctrl+Alt+C
  in the game to copy an item, then this hotkey to grab it. It cannot be
  Ctrl+Alt+C, the game's own copy.", a text field ("e.g. Ctrl+Alt+G") with
  **Set** (Enter sets it too; disabled while the field is empty), and the
  status of the last change (`hotkey_status`, polite live region; an error
  starts with `!`). The field is cleared on Set; the hotkey shown is always
  the view's.
- **Blocked currencies**: the help line "Currencies the helper never
  suggests. Pick them by name.", the count ("3 blocked", or "No currency is
  blocked."), and the currency picker (§ "The currency picker") over the
  saved list, without tags. Every change sends the whole list of ids with
  `update_settings`. Every id must be one of the catalogue's, with no
  repeats, so the list holds at most the whole catalogue; a saved id the
  catalogue no longer holds is dropped at load and counted in the status.
- **Updates**: the waystone roller's section, word for word with the app's
  name: the *Automatically check for updates* switch (off), its help line,
  the check's and the install's status lines, the download bar with its
  numbers and **Cancel**, the install's one button, **Check now**, and
  "Version <running version>"
  ([`docs/waystone-roller/ui-design.md`](../waystone-roller/ui-design.md)
  § "Settings window" has each state).
- **Created by ST4H**, the window's footer, plain text.

No button, link or state of the Settings window opens a release page or a
browser; the only page the app opens is Craft of Exile, from the main
window's button.

## States

Each state is a fixture, written by `ch-app`'s `tests/fixtures.rs` by
driving the app, and rendered by the page's tests. The catalogue the
pickers read is `fixtures/catalogue.json`, written by the same test.

| Main window (`fixtures/views/`) | What it shows |
|---|---|
| `empty` | Nothing grabbed: the two steps, the default hotkey, no status; craft mode none |
| `grabbed-item` | A hotkey grab of an item's text, "from the hotkey"; the craft part as that grab leaves it |
| `grab-refused` | A later press found an empty clipboard: the last grab kept, `!` the reason |
| `grabbed-not-item` | A paste of text that is not an item, "from a paste", `!` the warning; craft mode refused |
| `hotkey-taken` | Another program holds `Ctrl+Alt+X` at the start: `!` the status naming it |
| `overlay-active` | The overlay's line is visible (the shell's switch, which nothing in this version sets) |
| `craft-planning` | Targets chosen with their floors; a route whose steps show known and unknown odds; Start crafting enabled |
| `craft-fallback` | A blocked currency and the step's fallback line; Blocked for this craft open, the entry tagged with its source |
| `craft-no-route` | Targets no blueprint reaches: one line per rejected blueprint; Start crafting disabled with its reason |
| `craft-refused` | A grab the engine refuses: "Can't plan this item", the reason, the grabbed text |
| `craft-guiding` | The guide on its first step: instruction, odds, both branch lines, try 1, "No step taken yet." |
| `craft-miss` | After a miss: the "Last:" line and History's Miss entry with its summary, the step the miss led to |
| `craft-replan` | Status Replan with its reasons; End craft primary |
| `craft-done` | Status Done: "The craft is done.", the whole history |

| Settings window (`fixtures/app-settings/`) | What it shows |
|---|---|
| `idle`, `checking`, `up-to-date`, `available`, `no-release`, `failed` | Each state of the update check |
| `install-unsigned`, `install-downloading`, `install-verifying`, `install-restarting`, `install-failed` | Each state of the install |
| `hotkey-refused` | `Ctrl+Alt+C` asked for: `!` the refusal, the old hotkey kept |
| `blocked-currencies` | Three catalogue items blocked, checked by name in the picker |

## Browser preview

Outside Tauri (`npm run dev`, port 1422) each page renders a fixture chosen
by the query's `state`: `http://127.0.0.1:1422/?state=empty` (default
`grabbed-item`; the craft window's states are `?state=craft-planning` and
the other `craft-*` names),
`http://127.0.0.1:1422/overlay.html?state=overlay-active` (the default), and
`http://127.0.0.1:1422/settings.html?state=install-downloading&scale=2`
(default `idle`; `scale` zooms the page as the webview's scale would). The
pickers read `fixtures/catalogue.json`. Commands are logged, not acted on: a
paste is not shown as a grab, a target change does not re-plan, and the
Craft of Exile button logs `open_craft_of_exile` and opens nothing. The
fixtures are reached only on the dev server, so the release bundle carries
none.

## Accessibility

- Native controls throughout (button, checkbox, select, text input, list,
  `details`) so keyboard and screen reader behaviour comes from the
  platform. The target choices and the currency picker are checkbox lists
  in labelled groups, not custom widgets.
- Every floor select is labelled with its modifier ("Floor for <modifier
  text>"); every Remove button names what it removes.
- A visible focus ring (`--color-edit-accent`) on every control, the
  grabbed text block included.
- The status line is the one polite live region in the main window (and
  the Settings window's status lines in theirs); an error is told by `!`
  and words, an outcome and a guide status by a word, not colour alone.
- When the mode changes (Start crafting, End craft, a grab that starts
  planning), focus moves to the new mode's first heading, so a keyboard
  user is not left on a control that is gone.
- The Craft of Exile button says it opens the web browser (its visible
  mark and its accessible description).
- A disabled Start crafting says why in text beside it.
- End craft's in-place question is a group with its sentence as the label;
  focus moves to its Keep crafting button when it opens, so a second
  Enter does not end the craft.
- Text contrast from the tracker's tokens.
- No motion beyond transitions of 120 ms (colours, the switch, the
  download bar), and none under `prefers-reduced-motion`.
