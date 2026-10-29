# Crafting Helper — UI Design

The Crafting Helper is an ordinary desktop window (Tauri 2, Svelte 5,
TypeScript) with a Settings window and a click-through overlay. The main
window is the craft window: the player copies an item in the game with
`Ctrl+Alt+C` and grabs it with the app's hotkey (`Ctrl+Alt+X` by default) or
`Ctrl+V` in the window; the window shows the parsed item, lets the player
pick the modifiers they want with a tier floor each, previews the route the
engine chose (each step's currency, omens and odds, and the fallbacks taken
because something is blocked), and once accepted guides the craft one step
at a time, classifying each re-grab as a hit, a miss or unexpected. While
a craft is guided, the app can also read the hovered item from the game's
window (the screen watch, `screen_watch`) and advance the guide by itself,
and a click-through overlay over the game shows the current step. A button
in every state opens Craft of Exile in the default browser, and another
opens the *Find a base on trade* panel, which builds a trade site search
for a magic base to start a craft from and opens it in the browser. This
document is the direction the components follow.

What each window does, and every command and event, is in the guide's
Command reference ([`README.md`](README.md)). The direction was critiqued
before any component existed: [`design/review-direction.md`](design/review-direction.md)
for the craft window, [`design/review-overlay.md`](design/review-overlay.md)
for the overlay, the screen watch's lines and the Settings window's
Overlay section.

## Principles

- **Rust decides, the UI renders.** The shell sends one `View` (event
  `ch:view`, command `get_view`) to the main window and the overlay, and
  one `SettingsView` (event `ch:app-settings`) to the Settings window. A
  page shows what it was sent and sends commands; it never checks a grab,
  validates a hotkey, plans a route, judges an outcome or edits a list
  itself, and waits for the next view instead of patching its copy. The
  only state a page keeps is the text of its filter fields (the target
  filter, the currency filter and the trade panel's modifier filter), as
  the waystone roller's mod filter.
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
- **Fixed addresses, from Rust.** The app opens two outside pages, Craft
  of Exile and the trade site's search, each from an address held in Rust
  (the trade link is Rust's fixed prefix with the league and the query
  appended); a page sends a command that carries no address, and shows the
  trade link as text, never as a link it follows. Besides the update check
  (Settings, Updates), the app sends one request of its own: the trade
  site's league list, and only when the player clicks the refresh icon
  beside League in Settings.
- **The overlay never gets in the way.** It does not take focus, lets
  every click through, and shows only while a craft is guided and the
  game's window is in front.
- **The screen is read, never touched.** The app sends the game nothing:
  the player takes the step and hovers the item themselves, without Alt
  (the plain tooltip is the one the app reads best). A
  read the app cannot trust changes nothing and says to grab the item,
  which stays the fallback in every state.

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
Spacing on the 4 px grid (`--space-unit`). Odds, tiers, try counts and the
trade panel's weights use tabular figures (`font-variant-numeric: tabular-nums`) so a column of them
lines up.

## Windows

### Main window

Titled "Crafting Helper", resizable, in the taskbar. **960 × 680 by
default, usable from 600 × 480** (M3's 720 × 640 and 480 × 400 grow: the
item, the target list and the route do not fit side by side in 720, and a
route of eight steps with their odds does not fit in 400; 680 high still
fits a 1366 × 768 screen with the taskbar). A column of three parts:

- **The header**: the title "Crafting Helper" in small caps, then at the
  right end the **Find a base on trade** button (§ "Find a base on
  trade"), the **Explore more crafting on Craft of Exile** button and the
  **Settings** button, which sends `open_settings`. When a check this run
  found a newer version (the view's `update_available`), the Settings
  button names it after a divider, "Settings | Update 1.3.0", with a small
  green square beside the version; its accessible name is then "Settings,
  update 1.3.0 available". Below 760 px wide the header wraps: the title
  and Settings keep the first row, and the Find a base on trade and Craft
  of Exile buttons take their own row under it, in that order.
- **The craft area**, which takes the rest of the window and shows one of
  the craft part's four modes (§ "Craft window"), or, while it is open,
  the trade panel in its place (§ "Find a base on trade").
- **The status line** last (`role="status"`, polite): the app's last
  message; an error starts with `!` and takes the error colour.

A `Ctrl+V` anywhere in the window but a text field sends `grab_text` with
the paste's text, and nothing else. The hotkey's grab and a paste are the
same grab in every mode.

### Overlay

`overlay.html`: the current step of a guided craft, over the game, so the
player reads what to do and where a hit or a miss goes without looking
away from the game. Its one question is *"what do I do now, and did the
last try work?"*; everything else (the route's steps, the fallbacks, the item, the
history, every control) stays in the main window. This direction was
critiqued before the page was built:
[`design/review-overlay.md`](design/review-overlay.md).

**The window.** Transparent, undecorated, always on top, off the taskbar,
not resizable, **480 × 168** at scale 1 (the glue writes this size to
`tauri.conf.json`). It never takes focus (it is shown with
`SW_SHOWNOACTIVATE`, and is not focusable) and lets every click and the
cursor through to the game; nothing on the page can be selected, focused
or clicked. M3's placeholder line, "Next step: shown here while a craft is
active.", is gone.

**When it shows.** While the craft is guiding, in any guide status
(Crafting, Stuck, Replan, Done), **and** the game's window is in front
(its title is `Path of Exile 2`); hidden otherwise. It hides at once when
guiding ends (End craft) and when the game is not in front: another
window, the app's own windows included, or the game minimized. Rust
decides (`view.overlay.visible`) and the shell shows or hides the window;
the page never decides it. Each change is one diagnostic line:
`overlay: shown`, `overlay: hidden (not guiding)`, `overlay: hidden (game
not in front)`.

**Where it sits.** Inside the game's client area, at the edge or corner
the player chose in Settings (`overlay_position`): `top-left`,
`top-center` (the default), `top-right`, `bottom-left`, `bottom-center`,
`bottom-right`. The window is inset 12 physical pixels from the client's
edges (12 from the top and centred across for `top-center`, and so on),
from the overlay's size as the shell reports it in physical pixels, so the
inset holds at every display scale. A client smaller than the overlay and
its insets puts it at the client's top-left corner. Rust computes the
place (a pure `ch-app` function, tested) and the shell only moves the
window; it is placed again when the client rect or the setting changes,
never on every tick, with `overlay: placed <position>`.

**The panel.** One panel of the theme, as wide as the window and only as
tall as its rows, anchored to the window's edge nearest the game's edge
(its top for the three `top-*` positions, its bottom for the `bottom-*`
ones; the view carries the position), so the inset is the panel's visible
inset and the rest of the window is empty and click-through. The ground is
`--color-panel-bg` at `--opacity-default` (0.92), the tracker's overlay
ground: no blur, no glass, no gradient. A 1 px `--color-panel-border` edge
keeps it apart from a dark scene. Over a white scene the ground blends to
`#252729` (computed), which keeps primary text at 12:1, secondary at 7.4:1,
the warning colour at 6.3:1 and muted text at 4.7:1; over black they are
higher still. So the panel reads over a bright scene and a dark one alike;
muted text is kept to labels and the source words. Square
corners, `--space-panel-pad` (8 px) above and below and `--space-row-pad`
(12 px) at the sides, so no line starts at the panel's edge over a busy
scene (the audit's finding 1), no shadow (the window has none to draw it
in).

**The rows**, top to bottom, at most seven, each one line high and cut
with an ellipsis at its end when too long (the page never wraps a row, so
the panel never grows past 168):

1. **Header**: the guide's status word ("Crafting", "Stuck", "Replan",
   "Done") as the main window's outlined caption-size label, then "Step
   <n> · try <n>" in tabular figures (`--color-text-secondary`). No
   blueprint name: the route is the app's recipe, not the player's.
2. **Instruction**: the step's instruction as Rust sends it, which for a
   step that places a target names the currency, its omens, the side and
   the target modifiers, and for a removal step the currency (Omen of
   Whittling or Orb of Annulment, as the app decided) and the modifier it
   removes (§ "Rust-produced copy"); `--size-title` (16 px)
   at `--font-weight-title`, `--color-text-primary`: the row the eye lands
   on. The overlay has no separate targets row: the instruction carries
   them.
3. **Uses**: the label "Uses" (small caps, muted), the currency and omen
   names, then the odds as Rust sends them (`~12% (8000 of 69500
   weight)`, `odds unknown: <reason>`); the odds lead with the percent, so
   an ellipsis keeps the number. No fallback line: it is in the main
   window.
4. `On a hit: <where it goes>`, as Rust sends it.
5. `On a miss: <where it goes>`, as Rust sends it.
6. **Last**: once a change has been observed, "Last:", the outcome word
   (Hit, Miss, Unexpected; Miss and Unexpected in `--color-warning`, the
   word carries it), its source in muted words ("from the screen" or "from
   a grab") and the summary as sent, which carries no outcome word, so the
   word is said once. Before the first: "No step taken yet." in muted
   text.
7. **Screen**: the label "Screen" (small caps, muted) and the watch line
   as Rust sends it (§ "The screen watch's line"). In
   `--color-text-secondary` while it only reports, in `--color-warning`
   when it asks the player to do something (Rust says which); the words
   say what to do either way.

Rows 1–2 and 3–5 and 6–7 are three groups, 4 px apart; rows inside a group
sit on `--line-step` (18 px), the header and the instruction on
`--line-title` (20 px).
Body text is `--size-step` (14 px); labels and the status word are
`--font-family-caps` at `--size-caption`.

**Each status:**

- **Crafting** (active): all seven rows.
- **Stuck**: the header says "Stuck"; row 3 becomes "Why:" and the first
  reason as Rust sends it, and rows 4–7 stay, so the step and both branches
  stay in view while the player fixes it in the app.
- **Replan**: header "Replan"; row 2 "End the craft in the app to plan
  again from this item."; row 3 "Why:" and the first reason; then Last.
  The branch rows and the Screen row are left out (nothing is read).
- **Done**: header "Done"; row 2 "The craft is done."; then Last. Nothing
  else.

The screen watch reads the hovered item's tooltip, and **the overlay can
cover it**: a tooltip the overlay's window overlaps is never read in part
(a hidden line would read as a lost modifier), and the Screen row says so
and names the fix, moving the overlay in Settings or grabbing the item.
The reader counts the overlay's whole window as covering, its empty part
included (it reads the window's rect, not the panel's), which is why the
window is no taller than seven rows need (about 156 px of panel in 168).
Where the game draws a
tooltip depends on where the item sits and is not established here;
`top-center` is the owner's default. In Live 1 (2026-10-29, 2560 x 1440)
the overlay showed there, at the top middle over the game, and the
crafted belt's tooltip was read with it in place; no `tooltip covered by
the overlay` line was logged, so whether `top-center` ever covers a
crafted item's tooltip, at that or another resolution, is not observed.

**Not in this version:** hiding under the game's panels (a craft is taken
with the stash and the inventory open, so it would hide the overlay
throughout), dragging or editing it in place, a scale setting, motion of
any kind (rows change in place, without a transition, so nothing moves
over the game), and exclusive fullscreen.

The page is not a live region and has no accessible role beyond its
text: the main window's status line already announces each outcome, and a
second announcement from a window the player cannot focus would repeat it.

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

**Item** (left). The item's title heads the panel (its name, else its
base type), then one line of facts:
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

**Prefilled targets.** A grab that enters planning arrives with the
item's own explicit prefixes and suffixes already chosen, each with its
floor at the tier it has ("At least T5" for a T5 modifier), so each is
ticked in its group, its count is in the group heading, and the item panel
tags it "target": the player only adds the new modifier. A modifier the
craft cannot keep at that tier is left unticked, without a message. After a
grab that enters planning again, the list is the targets chosen before that
still fit the new item, then the new item's own modifiers not already in
it. The list is Rust's; the page sends nothing for it, and unticking works
as for any target. An item with no explicit modifier starts with none.

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
- With a route: its source (the blueprint's source line, muted:
  attribution; Rust sends it without the web addresses a blueprint's
  source names, so no view carries an address). The blueprint's name is
  not shown: the route ("close the finished side, then slam the open
  one") is the app's recipe, not something the player has to read. With other applicable routes, a select
  labelled "Route" whose options read "Route 1", "Route 2", ... in Rust's
  order, the previewed one first (so the route on screen is always Route
  1), each option's value its blueprint id; its change sends
  `choose_route { blueprint }`. Then what the
  craft targets: the sub-heading "Targets" and one row per chosen target,
  its text and its floor in Rust's words (the floor select's label, "At
  least T3"). The panel says what is targeted and what to do; the
  blueprint's notes (the reasoning behind the technique) are not sent and
  not shown. Then the steps as a
  numbered list (for a state-ordered route, the projection: the steps the
  craft takes if each one hits, from the item as it stands, so a step may
  come twice); each step shows its instruction (Rust's, naming the
  step's target modifiers, § "Rust-produced copy"), a "Targets" line with
  the target modifiers a hit of the step may place (left out for a step
  that places none, such as a quality or removal step), a "Uses" line with
  the currency and omen names, its odds as sent (`~12% (8000 of 69500
  weight)`, or `odds unknown: <reason>` with the words "odds unknown" kept,
  never a blank or a dash), and, when an earlier alternative was passed
  over, the fallback line under the label "Fallback" (small caps), as sent:
  `<used name> instead of <first alternative's name>: <names> blocked`
  (§ "Rust-produced copy" for a state-ordered step's).
- With no route: "No route reaches these targets." and one line per
  rejected blueprint, the rejection alone in Rust's words (no blueprint
  name), in Rust's order. The blocked list above is open, so the
  player can unblock what a rejection names.

**The Start bar** (fixed at the right column's foot): the primary button
**Start crafting**, which sends `accept_craft {}`, enabled only when the
view's `can_accept` is true; while it is disabled, a line beside it says
why ("Pick a target first." with no target, "No route to start." with no
route).

### Mode guiding

**Current step** (left). The item's title heads it, as the Item panel
titles it (its name, else its base type), with the guide's status as a
word beside it: "Crafting" (active), "Stuck", "Replan" or "Done". The
blueprint's name is not shown here or anywhere in the guide. Then, once a
change has been observed, the **last outcome** line, where the player's
eyes already are after a re-grab: "Last:" and the newest History entry's
outcome word, its source ("from the screen" or "from a grab", muted) and
summary (the page shows that entry; it judges nothing). The summary
carries no outcome word of its own, so the word is said once. Then, for
active and stuck, a sunken card with:

- "Step <n>" and the try count ("try 2"); the instruction, larger than the
  body text, which names the step's target modifiers, or for a removal
  step the currency and the modifier it removes (a whittle-or-annul step
  of a listed route names Omen of Whittling or Orb of Annulment, as the app
  decided from the modifier levels, § "Rust-produced copy"); the "Targets" line
  (the target modifiers a hit may place, as in the route's step list); the
  "Uses" line; the odds as sent; the fallback line when there is one.
  In a state-ordered route the engine picks each step from the item, so
  "Step <n>" counts the steps in the order this craft takes them, not
  their place in the blueprint or the preview: the first is 1, a step taken
  again right after itself keeps its number (the try count goes up), and
  any other step is the next number. A History entry keeps the number its
  step had when it was shown.
- The two branch lines as sent: `On a hit: <where it goes>` and `On a miss:
  <where it goes>`, where Rust names the next step by its instruction or
  says this step again, start over, give the item up, or the craft is done
  (a state-ordered route's lines: § "Rust-produced copy").
- The **watch line**: the label "Screen" (small caps, muted) and the
  screen watch's line as Rust sends it (§ "The screen watch's line"), in
  `--color-text-secondary` while it only reports and `--color-warning`
  when it asks the player to act. It is the same line the overlay's Screen
  row shows, so the two never disagree.
- The prompt "Take this step in the game, copy the item with `Ctrl+Alt+C`,
  then press `<hotkey>` or `Ctrl+V` here." It stays in every watch state:
  the grab is the fallback the watch line points to.

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
sets it the same way), the entry's **source** in muted words after it,
"from the screen" or "from a grab" (the view's source for the entry; the
page maps it to these words and judges nothing), and the summary
as sent, which carries no outcome word (the label before it says it once)
and names the targets gained or lost with their tier, a below-floor roll,
or the reasons.
With none yet: "No step taken yet." The history is not a live region; the
status line announces each outcome.

A grab while guiding is the step's result; a grab of the same item as the
step was shown on, or of text that is not a plannable item, changes nothing
and the status line says why (Rust's words). With the screen watch on, the
player can skip the grab: after the step, hovering the item lets the app
read it, and a hit or a miss read twice alike is kept as a grab's would be
(a History entry from the screen, the item now replaced). A change the
guide did not expect is **not** kept from the screen: the guide and the
item stay, and the status line and the watch line say to grab the item to
check. A grab always wins over a read in flight. A grab of the item a
screen read already gave (same rarity and explicit modifiers) is not a new
outcome: `craft: grab matches the screen`, and the grabbed text replaces
the read item.

### The screen watch's line

One line, Rust's copy, the same in the guiding card and the overlay's
Screen row: what the watch sees and what to do. Each form names the next
action in words, and the ones that ask the player to act are marked by
Rust to take the warning colour. Rust's words; the
`ch-app` tests pin each form.

| The watch | The line | Warning colour |
|---|---|---|
| Reading, no tooltip | `Watching: after the step, hover the item in the game.` | |
| The item unchanged | `Watching: the item has not changed yet.` | |
| Another item hovered | `Watching: that is another item; hover the one you are crafting.` | |
| A change, read once | `Checking the change: keep hovering the item.` | |
| Unreadable | `Can't read the item (<reason>): hover it without Alt, or grab it.` | yes |
| Unreadable because of the item itself | `Can't read the item (<reason>): grab the item after each step.` | yes |
| Under the overlay | `The overlay covers the item: move it in Settings, or grab the item.` | yes |
| An unexpected change | `The item changed in a way the guide did not expect: grab it to check.` | yes |
| 10 black copies in a row | `Can't copy the game's window: grab the item after each step.` | yes |
| No English OCR | `Can't read the screen (no English OCR): grab the item after each step.` | yes |
| The game's window call fails | `Can't read the screen (<call> failed): grab the item after each step.`, in place of the paused lines | yes |
| A copy or the OCR call fails | `Can't read the screen (<call> failed): grab the item after each step.` | yes |
| `screen_watch` off | `Screen reading is off: grab the item after each step.` | |
| No screen backend | `Screen reading is not available: grab the item after each step.` | |
| The game not in front | `Paused: the game is not in front.` | |
| The game minimized | `Paused: the game is minimized.` | |
| Stuck | `Paused: the guide is stuck.` | |
| Replan or done | none: the row and the line are left out | |

`<reason>` is the matcher's reason in words: `a line it does not know`,
`two tiers fit`, `the rarity is unclear`, `the item has no base type`,
`it may be cut off at the window's edge`, `a modifier it does not know`.
The reason comes from the item itself, so hovering differently cannot
help, when it is `a modifier it does not know`, or when the tier cannot be
told and the crafted item's own copy has an unrevealed or a fractured
modifier; then the line says to grab the item instead. Rust decides which
from the item it is guiding; a reason the read itself brought keeps the
first form.
`<call>` is the system call that failed, as the one-time `error: watch:
<call> failed: <error>` line names it; the line stays until a call
succeeds or the craft stops guiding. A failed window call never reads as
`Paused: the game is not in front.`: whether the game is in front is not
known. None of these lines, nor any other, carries OCR text, a pixel or
a window title.

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
| Fallback | `<used name> instead of <first alternative's name>: <names> blocked`, or `<used name> instead of <first alternative's name>: does not fit this item` when no passed-over alternative names a blocked id. None on a whittle-or-annul step the app named by the modifier levels: the whittle it passed over is not blocked. A state-ordered step's engine passes alternatives over by its own rules too (a whittle that is not certain, a Sinistral or Dextral omen with nothing on the other side to protect), and that is no fallback: the line shows only when a blocked id made it pass one over, which Rust tells by asking the engine for the same step with those ids unblocked, and reads `<used, with its omens> instead of <that alternative, with its omens>: <names> blocked` (`Orb of Annulment instead of Orb of Annulment with Omen of Sinistral Annulment: Omen of Sinistral Annulment blocked`) |
| Whittle or annul | in a listed blueprint, a removal step that offers Chaos Orb with Omen of Whittling and then a plain Orb of Annulment (the retired fill route's two clean-up steps, kept as `ch-engine`'s listed-order test blueprint) is named by the app from the modifier levels, on the item the step is shown on: Chaos Orb with Omen of Whittling when the whittle is a certain hit (the single lowest-level removable modifier is unwanted), else Orb of Annulment, its action and odds the engine's. With Orb of Annulment blocked for the craft the whittle stays, with its odds as they are; with Whittling or the Chaos Orb blocked the engine itself takes the annulment and the Fallback line shows. A grab or a screen read is judged against the way the app named the step, and the next step is decided again on the next item. Only in a listed blueprint in which every step that names Omen of Whittling is such a step. A state-ordered step is the engine's pick, never renamed: the engine takes the whittle only when it is certain to take a non-target on the blocked side and an annulment that fits could take a target, else the annulment, with Omen of Sinistral or Dextral Annulment when the other side holds a target |
| Floor options | `As high as possible (T<best>)` (the default), then `At least T<n>` per reachable tier |
| Step instruction | for a step that places a target: `<currency> for <targets>` with each target as `<text> (<side>)` (`Chaos Orb for +#% to Fire Resistance (suffix) or +#% to Cold Resistance (suffix)`); with omens, `<currency> with <omen> and <omen> for …`; for a step tied to one side, `<currency> on the <prefix or suffix> side for <targets>`. Lists read `a`, `a or b`, `a, b or c`. A removal step names the currency with its omens and the modifier it removes, by its text as the item shows it: the whittle `Chaos Orb with Omen of Whittling to remove <modifier>, the lowest-level modifier` (or `to remove the lowest-level modifier: <a> or <b>` when several share the level), the annulment `Orb of Annulment to remove <a> or <b>; it takes one modifier at random`, naming the unwanted modifiers; a side annulment names only those on its side, `Orb of Annulment with Omen of Sinistral Annulment to remove <a>; it takes one modifier at random`. A `fill` step with a currency names it, its omens and the side it fills, any modifier being its aim: `Exalted Orb with Omen of Greater Exaltation and Omen of Dextral Exaltation on the suffix side, for any modifier` (`<currency>, for any modifier` with no side), in every blueprint, listed included. Every other step keeps the blueprint's `say` text: a `remove-desecrated`, quality or manual step, a removal step with no action, and a step that places a target when no target is left for it. The same function makes it for the route's step list, the current step, the overlay and a History entry |
| Step targets | each step line's target modifiers, as the planning list writes them; empty when the step places none |
| Branches | `On a hit: …` and `On a miss: …`, naming the next step by its instruction as composed for the route on this item (a step the route left out by the blueprint's text), or this step again, start over, give the item up, or the craft is done; a step with a condition, `this step again until its condition holds, then <next>`. A miss that leads to a removal step says `the new modifier is removed next; the step says how once it is shown`: the modifier does not exist yet, so whittle or annul is decided when that step is shown. A state-ordered route has no transitions: the hit line names the projection's next step by its composed instruction, or says `the craft is done` when the step is the projection's last. The projection is the route the engine previews from the item the step is shown on; when no start of the route fits that item (both sides full, a magic route's item made rare), the one made when the craft started, while every step so far hit; with neither, `On a hit: the next step is picked from the item after the hit`. The miss line is always `On a miss: the next step is picked from the item after the miss` |
| History summary | no outcome word: the targets gained or lost with their tier, a below-floor roll, or the reasons (`gained +#% to Fire Resistance (T3)`). The page puts the word before it; the status line, which shows the summary alone, adds it itself (`Hit: gained +#% to Fire Resistance (T3).`) |
| Refusal, rejection, stuck and replan reasons, the status line | Rust's words. No status line names the blueprint (`Route chosen.`, `Craft started.`) |
| Catalogue group labels and currency names | Rust's (English) |
| The watch line (guiding card and overlay) | § "The screen watch's line", with whether it takes the warning colour |
| The overlay's rows | the step number and try count, the instruction, the uses, the odds, both branch lines, the last outcome's word, source and summary, the stuck and replan reason and the watch line: Rust's, in the overlay part of the view; whether it shows and its position too |

### UI-owned copy

Headings: "Can't plan this item", "Grab another item:", "Grabbed text",
"Prefixes", "Suffixes", "Desecrated", "Targets", "Chosen", "Route",
"Blocked for this craft: <n>", "History", "Why:". Buttons: "Explore more
crafting on Craft of Exile", "Settings", "Start crafting", "End craft",
"Keep crafting", "Remove", "Clear targets". Labels: "Filter modifiers",
"Filter currencies", "Blocked only", "Floor for <modifier text>", "Route",
the route select's options "Route <n>", "Uses", "Targets" (a step's line), "Fallback", "Last:", "best T<n>", "Step <n>", "try <n>", the tags "fractured", "desecrated",
"unrevealed", "target", "Settings", "this craft", "allowed for this craft",
the status words "Crafting", "Stuck", "Replan", "Done", the watch line's
label "Screen", the sources "from the screen" and "from a grab", and the
overlay's "Step <n> · try <n>", "Why:" and "End the craft in the app to
plan again from this item.". The Settings window's Overlay section:
"Overlay", "Position", the six position names, "Read the item from the
screen" and their help lines (§ "Settings window"). Empty prompts
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
competes with the area's one primary action (Start crafting, or the trade
panel's search button while that panel is open). Its click sends
`open_craft_of_exile {}` with no arguments, and Rust opens its one fixed
address, `https://beta.craftofexile.com/?game=poe2`, in the default
browser. The page holds no address: no `<a href>` to an outside page, no
`window.open`, no change of `location`. A failure to open shows in the
status line with `!`.

## Find a base on trade

A panel of the main window for starting a craft from a bought base: the
player picks an item class, a base type, an optional minimum item level and
the modifiers they want, sees how rare each is to roll on that base, and
one button opens a trade site search for a **magic** item of that base
carrying them, ranked by their rarity. Its one question is *"which magic
bases on trade already carry what I want?"*. The app sends the trade site
nothing for it: Rust builds the link from its fixed prefix, the league
(Settings, Trade) and the query, and the default browser loads it. The
panel never buys, whispers, copies to the clipboard or touches the game.

### Opening and closing

The header's **Find a base on trade** button (§ "Main window") is a
secondary button, as Settings is, with no outward-arrow mark: it opens a
panel in the app, not the browser. It sends `set_trade_search` with the
view's current selection and `open: true`; Rust sends a view whose trade
part is open, and the page shows **the panel in place of the craft area**.
While the panel is open the header button carries `aria-pressed="true"`
and pressing it again closes the panel.

**Back to the craft**, the panel's first control (a secondary button with
a left-arrow mark hidden from screen readers), sends `set_trade_search`
with `open: false`; the craft area comes back as the craft now stands.

**The craft is untouched underneath.** Opening the panel changes nothing
of the craft, its guide, the screen watch or the overlay: a guided craft
stays guided, the overlay keeps showing over the game, the screen watch
keeps reading and advancing the guide, and a grab (the hotkey, or
`Ctrl+V` outside a text field) goes to the craft as in any mode, the
status line saying what it did; the panel stays open. The selection lives
in Rust's memory for this run, kept while the panel is closed (reopening
shows it as it was) and never saved.

Focus: opening moves focus to the panel's heading; closing, either way,
moves it to the header's Find a base on trade button.

### Layout

The panel takes the craft area's place and its rules: from 880 px wide,
**two columns**, about 45 % and 55 %, each scrolling on its own; below,
**one column** scrolling as a whole. Across the top, above the columns:
**Back to the craft**, then the heading "Find a base on trade" (small caps,
as the craft area's headings).

| Width | Left column | Right column |
|---|---|---|
| from 880 px (960 × 680 by default) | Base, then Match, then League and the Trade link; the Search bar fixed at the column's foot | Modifiers |
| below 880 px (down to 600 × 480) | Base, Modifiers, Match, League and the Trade link, in that order; the Search bar fixed at the window's foot above the status line | |

At 960 × 680 the left column's groups (three fields, two choices, the
league line and a three-line link) fit without scrolling above the Search
bar, and the right column holds the modifier list, which is the long part.
At 600 × 480 the Modifiers groups scroll together in a sunken box past
200 px, so Match and the link are not a whole base's list away, and the
Search bar stays in view, as the craft's Start bar does.

### The controls

Every change sends `set_trade_search { search: { open, class, base,
item_level, picks: [{ mod_id, floor }], both_slots } }` with the **whole
selection**, as `set_targets` sends the whole target list: `class` and
`base` are `null` until chosen, `item_level` is `null` for none, `floor`
is `null` for "Any tier". No field names an address, and the page sends
no link. Rust validates and sends the next view; a refusal leaves the
selection as it was and says why in the status line with `!`.

**Base** (a group headed "Base"):

- **Item class**, a native select labelled "Item class": first the
  placeholder option "Choose a class", then the view's class list in its
  order (Rust's names: "Rings", "Gloves", "Jewels", …). Choosing a class
  clears the base and the picks (Rust does it, and the status line says
  how many picks were cleared, so a slip of the select is not silent).
- **Base type**, a native select labelled "Base type": "Choose a base",
  then the chosen class's bases in the view's order ("Sapphire Ring").
  Disabled until a class is chosen. Choosing another base keeps the picks
  the new base offers and drops the rest; the status line says how many
  were dropped (Rust's words).
- **Minimum item level**, a number field labelled "Minimum item level",
  with the help line "Optional. Empty searches any item level; a higher
  level reaches better tiers." It sends on change (Enter, or leaving the
  field), not on every key; empty sends `null`. Rust takes a whole number
  from 1 to 100 and refuses anything else; on a refusal the field shows
  the view's value again. The level also sets which tiers each modifier's
  floor offers.

**Modifiers** (right column, heading "Modifiers"): the help line "Weight:
higher is rarer. 12.0 means about 1 in 12 modifiers added to that side of
this base is this one, at the floor or better. The search ranks results by
these weights." (the design critique's finding: a bare number has no
scale, and 1 in N is what the weight is) and a filter field "Filter modifiers" that narrows the rows by
text as the player types (the page's own state, as the craft's filter;
with no match, "No modifier matches “<text>”."). Then two groups,
"Prefixes" and "Suffixes", each heading with its count of picks
("Prefixes · 1 picked"), the rows in the view's order. Each searchable row
is one line:

- a **checkbox** labelled with the modifier's text as the data writes it
  (`+# to maximum Life`); checking picks it with floor "Any tier",
  unchecking drops it;
- at the row's right end, the **weight column**: the rarity weight as
  Rust sends it, one decimal (`12.4`), right-aligned in tabular figures, so
  the column lines up; unpicked rows show theirs too, so the player can
  compare before picking;
- when the weight is the **default** (the data gives this modifier no
  usable spawn weight on this base: every jewel, some tiers), the word
  "default" after the number, as an outlined caption-size tag (the
  "target" tag's style), never a colour or a mark alone.

A picked row adds, under its text:

- its **floor select**, labelled "Floor for <modifier text>", whose
  options are Rust's: `Any tier` (the default), then `At least T<n>` per
  tier reachable at the item level. A stricter floor raises the row's
  weight (a better tier is rarer), so the number changes with the floor;
- for a default weight, the line `default: <reason>` in muted text, the
  reason as Rust sends it (why the data has no weight here). Above the
  groups, when any row's weight is the default, one muted line explains
  the word: "“default” marks a weight the data does not give; a picked
  modifier says why."

At most **8 picks**: with 8, the view says no further pick is allowed, the
unpicked checkboxes are disabled and Rust's line `At most 8 modifiers in
one search.` shows above the groups.

**Not searchable** rows (the view marks them, with a reason: a modifier
with more than one line, whose lines the trade site lists as separate
stats, or one with no trade site stat) are not mixed into the groups. They
sit in one disclosure after the two groups, "Not searchable on trade: <n>",
closed by default and narrowed by the same filter, each entry its text,
its side ("prefix", "suffix") and
Rust's reason, with no checkbox, so the keyboard never stops on a row it
cannot pick. A base with none leaves the disclosure out.

**Match** (a group headed "Match", a native radio group): "Both slots"
(the default) and "At least one", each with a muted line under its label
that says what it asks for, "A prefix and a suffix from your picks." and
"Any one of your picks.", so the choice reads without the help. A magic
item holds one prefix and one suffix, so "Both slots" asks for two of the
picks, one on each side; "At least one" asks for one. Under the group,
Rust's **count line**, which says what the search will ask for and, when
it differs from the choice, why: with picks on both sides and Both slots,
two; with At least one, one; with picks on one side only, one, because a
magic item holds a single modifier of that side (§ "The trade panel's
Rust-produced copy"). No count line before the first pick.

**League**: one line, "League" (small caps, muted) and the view's league,
then a small secondary **Change** button that sends `open_settings {}`
(the league is set in Settings, Trade; the panel holds no league field).

**Trade link**, shown only when the view carries a link: the label "Trade
link", then the link in a **read-only** multi-line field (a `textarea` with
`readonly`, three rows, the system monospace, a sunken surface, breaking
anywhere, focusable and selectable so the keyboard can scroll it), and the
help line "Select it and press Ctrl+C to copy it." The page never makes
it a link: no `<a href>`, no `window.open`, no change of `location`; the
app writes nothing to the clipboard.

**The Search bar**: the window's primary button for as long as the panel
is open, **Search on the trade site**, with an outward-arrow mark after
the text (hidden from screen readers) and the accessible description
"Opens the trade site in your web browser". It sends `open_trade_search
{}`, with no arguments; Rust opens the link it built from its own state in
the default browser. It is enabled only when the view says a search can
be opened; while disabled, the line beside it says why, as Rust sends it.
A failure to open shows in the status line with `!`.

### Each state

- **No class** (the panel just opened, the default): Item class on
  "Choose a class", Base type disabled; the Modifiers column says "Choose
  an item class to see its modifiers."; no count line, no link; the
  Search button disabled beside Rust's `Choose an item class.`
- **No base**: a class chosen, Base type on "Choose a base"; Modifiers
  says "Choose a base type to see its modifiers."; disabled beside
  `Choose a base type.`
- **No pick**: the base's rows, each with its weight, none checked; no
  count line, no link; disabled beside `Pick at least one modifier.`
- **Picks on one side only**: the link shown and the button enabled; the
  count line says the search asks for one, and why.
- **A refusal**: a `set_trade_search` Rust refuses (a base the class does
  not have, a modifier the base does not offer or cannot search, a ninth
  pick, an item level out of range) changes nothing: the panel shows the
  last view, and the status line says why with `!`. A selection Rust
  cannot make a link for (the link would pass 2,000 characters) shows no
  link, and the line beside the disabled button is Rust's reason, starting
  with `!` and in the warning colour.
- **Link ready**: a prefix and a suffix picked, Both slots on: the count
  line asks for two, the link is shown, the Search button is enabled. Its
  click writes `trade: opened, <n> modifiers, at least <k>` to the
  diagnostic log (counts only) and the status line says the search opened
  in the browser.

### The trade panel's Rust-produced copy

The page renders these as sent; `ch-app`'s tests pin each form.

| What | Form |
|---|---|
| Class and base names | Rust's, from the shipped data (`Rings`, `Sapphire Ring`) |
| Modifier text | as the data writes it, a line each (`+# to maximum Life`) |
| Weight | one decimal, `12.4`; the source `spawn` or `default`, and for a default its reason |
| Floor options | `Any tier` (the default), then `At least T<n>` per reachable tier |
| Not searchable reason | `more than one line; the trade site lists each line as its own stat`, `no trade site stat for this text` |
| Count line | `The search asks for two: one prefix and one suffix.`, `The search asks for at least one of your picks.`, `Your picks are all suffixes and a magic item holds one, so the search asks for one.` (prefixes alike) |
| Why there is no link | `Choose an item class.`, `Choose a base type.`, `Pick at least one modifier.`, `! Too many modifiers for one link: remove one.` |
| Pick limit | `At most 8 modifiers in one search.` |
| League | the saved league's id (`Forbidden Rites`) |
| The link | `https://www.pathofexile.com/trade2/search/poe2/<league>?q=<query>`, both percent-encoded |
| Status lines | `Opened the trade search in your browser.`, `! Couldn't open the browser: <error>`, the refusal reasons, the count of picks a new base dropped |
| Settings, Trade note | `Refreshed: 6 leagues from the trade site.`, `Refreshed: 5 leagues from the trade site; 1 skipped.`, `! Not refreshed: <reason>. The list and your league are unchanged.`, `! League not saved: <reason>.` |

### The trade panel's UI-owned copy

Buttons: "Find a base on trade", "Back to the craft", "Search on the trade
site", "Change". Headings and labels: "Find a base on trade", "Base",
"Item class", "Base type", "Minimum item level", "Modifiers", "Filter
modifiers", "Prefixes", "Suffixes", "<n> picked", "Floor for <modifier
text>", the tag "default", "Not searchable on trade: <n>", "prefix",
"suffix", "Match", "Both slots", "At least one", "League", "Trade link".
Placeholders, hints and help: "Choose a class", "Choose a base", "Optional.
Empty searches any item level; a higher level reaches better tiers.",
"A prefix and a suffix from your picks.", "Any one of your picks.",
"Weight: higher is rarer. 12.0 means about 1 in 12 modifiers added to that
side of this base is this one, at the floor or better. The search ranks
results by these weights.", "“default” marks a weight the data does not give; a
picked modifier says why.", "Choose an item class to see its modifiers.",
"Choose a base type to see its modifiers.", "No modifier matches
“<text>”.", "Select it and press Ctrl+C to copy it.", and the accessible
description "Opens the trade site in your web browser". The Settings
window's Trade section: "Trade", "League", the refresh button's
accessible name "Refresh the league list from the trade site", "Refreshing
the league list…" and its help line.

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
- **Overlay**: two controls, each saving itself (one save model, as the
  rest of the window):
  - **Position**, a native select labelled "Position" with the six places
    in reading order, "Top left", "Top center", "Top right", "Bottom
    left", "Bottom center", "Bottom right", sending `update_settings {
    overlay_position }` with `top-left` … `bottom-right`; the view's value
    is selected (`top-center` by default). Help line: "Where the step
    panel sits in the game's window while a craft is guided."
  - **Read the item from the screen**, a switch styled as Updates' switch,
    sending `update_settings { screen_watch }`; on by default. Help line:
    "After each step, hover the item in the game and the helper reads it.
    Copies of the screen stay in memory; nothing is saved or sent. Off:
    grab the item after each step."
  The select offers only the six names the app takes, so the page cannot
  send a position the app refuses; should the shell refuse a change all the
  same, the saved value stays, the next view shows it and the bridge logs
  Rust's reason, as it does for the blocked list (the view carries no
  status line for this section; the audit's finding 3). The section sits
  after Grab because both shape
  the craft loop; the page grows past the window's 720 px with it, and
  scrolls as a whole (as it already does in `install-downloading`), so
  Updates, the least-used section, is the one below the fold.
- **Blocked currencies**: the help line "Currencies the helper never
  suggests. Pick them by name.", the count ("3 blocked", or "No currency is
  blocked."), and the currency picker (§ "The currency picker") over the
  saved list, without tags. Every change sends the whole list of ids with
  `update_settings`. Every id must be one of the catalogue's, with no
  repeats, so the list holds at most the whole catalogue; a saved id the
  catalogue no longer holds is dropped at load and counted in the status.
- **Trade**: the league the trade panel's search opens in (§ "Find a base
  on trade"). One row, then a help line and a note line:
  - **League**, a native select labelled "League", and beside it, on the
    same row, the **refresh icon button**. The select's options are the
    view's offered leagues, in the view's order, each option's text and
    value the league's id as the site writes it ("Forbidden Rites", "HC
    Forbidden Rites"); the saved `trade_league` is selected. A change
    sends `update_settings { trade_league }`.
  - What the select offers, all Rust's: **before any refresh**, the six
    leagues this version shipped with (`SHIPPED_TRADE_LEAGUES`: Forbidden
    Rites, HC Forbidden Rites, Runes of Aldur, HC Runes of Aldur, Standard,
    Hardcore, in the site's order); **after a refresh**, the site's poe2
    leagues from the answer, in its order, for this run only (the list is
    never saved; the next start offers the shipped six again and sends
    nothing). In both, **the saved league is always offered**: appended
    last when the list lacks it, so a league saved from an older list
    never disappears and is never refused for being missing.
  - The **refresh icon button**: a secondary button, 32 × 32 px, square,
    with a circular-arrow mark drawn in SVG (hidden from screen readers;
    no icon font) and no visible text; its accessible name is "Refresh
    the league list from the trade site", and the same words are its
    `title`, so a mouse user who does not know the mark can hover it. Its click sends
    `refresh_trade_leagues {}`, with no arguments. **Busy**: while the
    view says the list is refreshing, the button is disabled and carries
    `aria-busy="true"`, the mark stays still (no spinner; § "Accessibility",
    motion), and the note line reads "Refreshing the league list…". Rust
    returns the view at once marked refreshing and sends the next one when
    the answer is in; a click while one runs sends no second request (Rust
    refuses it even if the page let it through). The select stays usable
    meanwhile.
  - The help line: "The league the trade search opens in. The refresh
    button asks the trade site (pathofexile.com) for its current leagues;
    nothing is sent until you click it, and the new list lasts until the
    app closes."
  - The **note line** (polite live region, as Grab's status): empty until
    something happens this run; after a refresh, Rust's note, how many
    leagues the list now holds and how many entries were skipped (§ "The
    trade panel's Rust-produced copy"); after a refused refresh,
    Rust's reason starting with `!` and in the warning colour, saying the
    list and the league are unchanged. A refused `trade_league` patch (a
    name over 64 characters, empty after trimming, or with a control
    character; the select cannot send one, but the shell refuses it all
    the same) shows its reason here too, and the saved league stays
    selected.
  **Nothing is requested until the icon is clicked**: not at start, not
  on opening Settings or the trade panel, not on a timer, after an update
  check or on a settings patch. The section sits after Blocked
  currencies and before Updates, so Updates stays the last section.
- **Updates**: the waystone roller's section, word for word with the app's
  name: the *Automatically check for updates* switch (off), its help line,
  the check's and the install's status lines, the download bar with its
  numbers and **Cancel**, the install's one button, **Check now**, and
  "Version <running version>"
  ([`docs/waystone-roller/ui-design.md`](../waystone-roller/ui-design.md)
  § "Settings window" has each state).
- **Created by ST4H**, the window's footer, plain text.

No button, link or state of the Settings window opens a release page or a
browser. The app opens two pages, both from the main window: Craft of
Exile, from its header button, and the trade site's search, from the trade
panel's search button (§ "Find a base on trade"). Two sections here send
requests: Updates (its check and its install), and Trade, whose refresh
icon asks the trade site for its league list and nothing else.

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
| `overlay-active` | Guiding with the game in front: the overlay is visible with every row of an active guide, `top-center` |
| `overlay-not-in-front` | Guiding with another window in front: the overlay is hidden |
| `craft-prefilled` | Planning right after the grab: the item's own two modifiers ticked as targets at their tier and tagged "target", the route the engine previews for them |
| `craft-planning` | Targets chosen with their floors, listed again above the route's steps; steps that name their target modifiers and show known and unknown odds; Start crafting enabled |
| `craft-fallback` | A blocked currency and the step's fallback line; Blocked for this craft open, the entry tagged with its source |
| `craft-no-route` | Targets no blueprint reaches (Chaos Orbs and Orbs of Annulment blocked): one line per rejected blueprint; Start crafting disabled with its reason |
| `craft-close-side` | The owner's 2P/1S belt on the state-ordered `close-a-side`: the fill step `Exalted Orb with Omen of Greater Exaltation and Omen of Dextral Exaltation on the suffix side, for any modifier`, then the slam on the one open side with no side omen; the source line without its web addresses |
| `craft-refused` | A grab the engine refuses: "Can't plan this item", the reason, the grabbed text |
| `craft-guiding` | The guide on its first step: instruction, odds, both branch lines, try 1, "No step taken yet." |
| `craft-miss` | After a miss: the "Last:" line and History's Miss entry with its summary, the step the miss led to |
| `craft-miss-clean` | The belt's craft after the slam on its open side missed: Step 3, the engine's Orb of Annulment with Omen of Sinistral Annulment (a whittle would take a target), its odds, no Fallback line, both branch lines saying the item picks the next step |
| `craft-replan` | Status Replan with its reasons; End craft primary |
| `craft-done` | Status Done: "The craft is done.", the whole history |
| `craft-watching` | Guiding with the screen watch reading and no tooltip: the watch line `Watching: …` |
| `craft-screen-hit` | A hit read from the screen: "Last:" and History's entry "from the screen", the next step |
| `craft-screen-unexpected` | An unexpected change read and not applied: the step and item kept, the watch line in the warning colour |
| `craft-watch-off` | `screen_watch` off: `Screen reading is off: …`; grabs drive the guide as in M5 |
| `craft-watch-unavailable` | No English OCR: `Can't read the screen (no English OCR): …` |
| `trade-empty` | The trade panel just opened over a planning craft: no class chosen, the Search button disabled with `Choose an item class.` |
| `trade-picking` | Rings, Sapphire Ring, item level 75, one prefix and one suffix picked at Any tier, Both slots: the count line asks for two, every row's weight, the link shown, Search enabled |
| `trade-one-side` | Two suffixes picked: the count line says the search asks for one and why; the link shown |
| `trade-default-weight` | A jewel base: every weight tagged "default", the picked rows' `default: <reason>` lines, the legend above the groups |
| `trade-refused` | After `trade-picking`, a `set_trade_search` naming a base the class does not have: `!` the reason in the status line, the selection and the link kept |

| Settings window (`fixtures/app-settings/`) | What it shows |
|---|---|
| `idle`, `checking`, `up-to-date`, `available`, `no-release`, `failed` | Each state of the update check |
| `install-unsigned`, `install-downloading`, `install-verifying`, `install-restarting`, `install-failed` | Each state of the install |
| `hotkey-refused` | `Ctrl+Alt+C` asked for: `!` the refusal, the old hotkey kept |
| `blocked-currencies` | Three catalogue items blocked, checked by name in the picker |
| `overlay-settings` | The Overlay section with a position other than `top-center` and the screen watch off |
| `trade-league` | The Trade section with a league other than the default selected, the shipped list offered, no note |
| `trade-league-refreshing` | A refresh running: the refresh button disabled and busy, "Refreshing the league list…" |
| `trade-league-refresh-refused` | A refused refresh: `!` the reason in the note line, the list offered before and the saved league kept |

Every view fixture carries the overlay part, the watch line and the trade
part (closed in every fixture but the `trade-*` ones), and every settings
fixture `overlay_position`, `screen_watch`, `trade_league` and the offered
leagues, so the page tests render the new fields in every state. The overlay page renders any view
fixture: `overlay-active` for an active guide, `craft-replan` and
`craft-done` for those statuses.

## Browser preview

Outside Tauri (`npm run dev`, port 1422) each page renders a fixture chosen
by the query's `state`: `http://127.0.0.1:1422/?state=empty` (default
`grabbed-item`; the craft window's states are `?state=craft-planning` and
the other `craft-*` names),
`http://127.0.0.1:1422/overlay.html?state=overlay-active` (the default; any
view fixture's name works, `craft-done` for instance, and `scene=bright` or
`scene=dark` paints a flat white or near-black stand-in for the game behind
the panel, in the preview only, so its legibility over both can be
checked; the preview draws the page at the window's 480 × 168 inside a
dashed outline, so the panel's height and anchoring show), and
`http://127.0.0.1:1422/settings.html?state=install-downloading&scale=2`
(default `idle`; `scale` zooms the page as the webview's scale would). The
pickers read `fixtures/catalogue.json`. Commands are logged, not acted on: a
paste is not shown as a grab, a target change does not re-plan, the
Craft of Exile button logs `open_craft_of_exile` and opens nothing, the
trade panel's states are `?state=trade-picking` and the other `trade-*`
names (a pick logs `set_trade_search`, the Search button logs
`open_trade_search` and opens nothing), and the Settings window's
`?state=trade-league` and the other `trade-league-*` names show the Trade
section, whose refresh button logs `refresh_trade_leagues` and requests
nothing. The
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
  mark and its accessible description), and so does the trade panel's
  Search on the trade site; Find a base on trade, which opens a panel in
  the app, carries no such mark and says whether the panel is open
  (`aria-pressed`).
- Opening the trade panel moves focus to its heading; closing it moves
  focus back to the header's Find a base on trade button.
- The trade panel's modifier rows are checkboxes in two labelled groups;
  each row's weight sits inside its label, so a screen reader hears "+# to
  maximum Life, weight 12.4" (", default" after a default weight). Rows
  that cannot be searched carry no checkbox. The trade link is a read-only
  field the keyboard can reach and select, never a link.
- The Trade section's refresh button is an icon with an accessible name
  ("Refresh the league list from the trade site"), disabled and
  `aria-busy` while refreshing; its note line is a polite live region.
- A disabled Start crafting says why in text beside it.
- End craft's in-place question is a group with its sentence as the label;
  focus moves to its Keep crafting button when it opens, so a second
  Enter does not end the craft.
- The watch line and the overlay's warning rows are told by their words
  (each says what to do), not by the warning colour alone; a History
  entry's source is a word, not a mark.
- The overlay cannot take focus and is not a live region; everything it
  shows is also in the main window, where the status line announces it.
- The Settings window's Position and League selects and screen-reading
  switch are native controls with visible labels; the switch's state is its checked
  state, as Updates' switch.
- Text contrast from the tracker's tokens; on the overlay over a white
  scene, 12:1 for primary text and 4.7:1 for muted text (computed from
  the tokens at the panel's 0.92 opacity).
- No motion beyond transitions of 120 ms (colours, the switch, the
  download bar), and none under `prefers-reduced-motion`.
