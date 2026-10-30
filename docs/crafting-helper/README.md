# Crafting Helper

A small window for Path of Exile 2 that helps you craft items. You copy an
item in the game with the game's own `Ctrl+Alt+C`, then press the app's
hotkey (`Ctrl+Alt+X` by default) or `Ctrl+V` in its window, and the window
becomes the craft window: it shows the item and its modifiers, lets you pick
the modifiers you want as targets (each with a tier floor), and previews the
route a crafting blueprint takes to them, each step with its currency, its
omens and its rough odds, or "odds unknown" and why. *Start crafting* turns
it into a guide: you take the step in the game and hover the item, and the
app reads the item's tooltip from the game's window with Windows' own OCR
and says whether the step hit or missed and what to do next. A read it
cannot trust changes nothing and says to grab the item again, which always
works. While a craft is guided and the game is in front, a click-through
overlay over the game shows the current step, where a hit and a miss go
and the last outcome. The app reads the clipboard once per grab and never
watches it; copies of the game's window stay in memory. It uses no
keyboard hook, sends the game no keys, reads no game memory, moves no
mouse and clicks nothing. A button opens Craft of Exile in your web
browser for the crafting this app does not cover.

- UI direction, windows and states: [`ui-design.md`](ui-design.md)
- The design reviews: the craft window's direction, before it was built
  ([`design/review-direction.md`](design/review-direction.md)), the built
  pages ([`design/review-craft-window.md`](design/review-craft-window.md)),
  and the overlay's direction, before it was built
  ([`design/review-overlay.md`](design/review-overlay.md))
- What each crafting item does to an item's modifiers:
  [`crafting-mechanics.md`](crafting-mechanics.md)
- Research note (poe2db's modifier pages, the data extractor, the data
  files, the item tooltip on screen):
  [`crafting-helper-research.md`](crafting-helper-research.md)
- Writing and checking a crafting blueprint: [`blueprints.md`](blueprints.md)
- How a state-ordered route uses an item's free slots, and the facts it
  rests on with their labels:
  [`docs/models/crafting-slots-spec.md`](../models/crafting-slots-spec.md);
  why routes are planned that way:
  [`docs/adr/0001-crafting-helper-route-design.md`](../adr/0001-crafting-helper-route-design.md)
- The item text parser the app uses:
  [`apps/crafting-helper/crates/ch-item/README.md`](../../apps/crafting-helper/crates/ch-item/README.md)
- How the app updates itself, shared with the quest tracker and the
  waystone roller: [`docs/app-updates.md`](../app-updates.md)

## Overview

- **One window, `crafting-helper.exe`.** A Tauri app: the app's logic
  (`ch-app`) is a Rust crate, and the UI is Svelte and TypeScript in an
  ordinary window titled "Crafting Helper", resizable and in the taskbar.
  Closing the window quits the app. Only one copy runs; starting a second
  brings the first to the front.
- **Grabbing an item.** In the game, hover the item and press
  `Ctrl+Alt+C`, the game's own copy of the item's text. Then either:
  - press the app's **hotkey**, `Ctrl+Alt+X` by default, a global shortcut
    that works while the game is in front. Each press reads the clipboard's
    text once; or
  - press **`Ctrl+V` in the app's window**. The window takes the text the
    paste carries. A paste into a text field of the window is that field's,
    not a grab.
- **The craft window** is the main window's craft area, in one of four
  modes:
  - **Nothing grabbed**: how to grab.
  - **Can't plan this item**: the last grab is not an item a blueprint
    plans for (a text that is not an item copied with `Ctrl+Alt+C`, or the
    engine's refusal, such as a corrupted item), with the reason in words
    and the grabbed text shown.
  - **Planning**: the item (name, rarity, base, class, item level, and its
    prefixes and suffixes with their tiers), the **targets** (the
    modifiers this base can roll at its item level, as prefixes, suffixes
    and desecrated, filterable by text; each chosen one with a floor, "As
    high as possible" by default or "At least T<n>"; a choice that would
    take its side past the item's limit is greyed out, below), **Blocked for this
    craft** (the Settings list, which this craft can add to or allow
    from, without changing Settings), and the **route** the engine chose:
    the blueprint's source, the chosen targets with their floors, then
    each step's instruction, the target modifiers a hit of it may place,
    its currency and omens, its rough odds or "odds unknown" with the
    reason, a fallback line where a blocked item made it take another,
    and other applicable routes to choose from, as `Route 1`, `Route 2`
    and so on. The blueprint's name is not shown: the technique is the
    app's to follow, not the player's to read. A step that places a
    target says so in the app's own words, naming the currency, its omens,
    the side and the modifiers (`Chaos Orb for +#% to Fire Resistance
    (suffix) or +#% to Cold Resistance (suffix)`). A removal step names
    the currency with its omens and the modifier it removes, by its text
    on the item (`Orb of Annulment to remove +95(90-104) to maximum Mana
    or +200(168-236) to Accuracy Rating; it takes one modifier at
    random`). Where a listed blueprint lets a clean-up step either whittle
    (Chaos Orb with Omen of Whittling, which takes the lowest-level
    modifier) or annul (a plain Orb of Annulment, which takes one at
    random), the app decides from the modifier levels on the item: the
    whittle when the engine's whittle is a certain hit, every modifier at
    the lowest level being one you do not want (when several share that
    level, the step names each), otherwise the annulment, and the whittle after all when Orb of
    Annulment is blocked for the craft. A step that neither places nor
    removes a modifier (a fill, a quality or a manual step) keeps the
    blueprint's text. The blueprint's notes, the reasoning behind its
    technique, are not shown. With no route, one line per blueprint says
    why it does not apply. *Start crafting* starts the guide.

    Four of the blueprints are **state-ordered** (since 0.1.1): the
    engine plans them from each side's free slots rather than a fixed
    order, taking the first step whose condition holds on the item as it
    stands, so an item part-way through a craft (a rare with some of the
    targets, a fractured target, a magic item with one good modifier)
    starts where it is. Their route is the path the craft takes if each
    step hits, from this item, so a step may appear twice. On such a
    route no step offers an Exalted Orb without a side omen while a side's
    last free slots are kept for its missing targets; a step names Omen
    of Greater Exaltation where two free slots take two targets; a step
    that closes a finished side says so in the app's words (`Exalted Orb
    with Omen of Greater Exaltation and Omen of Dextral Exaltation on the
    suffix side, for any modifier`); a removal is a whittle only when it
    is certain to take a modifier you do not want, and names Omen of
    Sinistral or Dextral Annulment when the other side holds a target;
    and the engine's choice is final (the app does not decide again by
    the modifier levels). How they work: [`blueprints.md`](blueprints.md)
    § "State-ordered blueprints".

    **The last target is desecrated** (since 0.1.2, issue #32). When the
    item is rare, carries no desecrated modifier (an unrevealed one
    included) and exactly one target is missing, a state-ordered route
    places that last target with a desecration on its side instead of an
    Exalted Orb or an essence: the bone for the item's class with Omen of
    Sinistral or Dextral Necromancy (`Preserved Collarbone with Omen of
    Sinistral Necromancy on the prefix side for +# to Armour`), then the
    reveal with Omen of Abyssal Echoes. A reveal that offers no target is
    taken off with Omen of Light and an Orb of Annulment, and the bone is
    used again. Both steps say "odds unknown" with the reason. When the
    last target cannot be desecrated, the route keeps the step it took
    before, and that step's Fallback line says why (`Perfect Exalted Orb
    instead of a desecration: Omen of Sinistral Necromancy blocked`): the
    item's class has no desecrated modifier on that side at the item's
    level (body armours, boots, gloves, helmets, shields and bucklers
    have no desecrated prefix, sceptres none at all, and every equipment
    desecrated modifier needs item level 65), or the bone or the
    Necromancy omen is blocked. A plain bone without the omen is never
    offered in its place, since it could land on the other side. An item
    that already has its desecrated modifier, two or more missing
    targets, and a magic or normal item keep the route as before.

    **Target choices respect the item's prefix and suffix limits** (since
    0.1.2, issue #32). The limit is the item's as a rare, read from the
    item: three and three, two and two on a jewel, changed by a line on
    the item that allows one more or one fewer prefix or suffix (the Dusk
    and Gloam rings and amulets, the Distorted and Twisted amulets, a
    Contempt liquid emotion), so a rare Dusk Ring takes four prefixes and
    two suffixes. A side counts its chosen targets (a desecrated choice
    on the side its modifier belongs to) and the item's modifiers that no
    target names and a route cannot take off (fractured, unrevealed). An
    unchosen choice on a side that has reached its limit is greyed out,
    still listed and filtered, its checkbox disabled, and says why on
    hover (`Prefixes full: 2 on the item + 1 chosen = 3 of 3`). A chosen
    one is never disabled: unticking it frees its side and re-enables
    the rest. Since a grab ticks the item's own modifiers, a rare with
    three prefixes arrives with its prefix side full; untick one to pick
    another prefix.

    A grab that enters planning arrives with the item's own explicit
    prefixes and suffixes already ticked as targets, each with its floor
    at the tier it has, so you only add the modifier you want next; one
    the craft cannot keep at that tier is left unticked, without a
    message, and an item with no explicit modifier starts with none.
    After another grab while planning, the list is the targets chosen
    before that still fit the new item, in their order while their side
    has room under the new item's limit, then the new item's own
    modifiers not already in it that still fit. Untick any of them as
    you would any target. A list that would take a side past its limit
    is refused whole, and the status line says why (`Prefixes over the
    limit: 3 on the item + 1 chosen = 4 of 3`).
  - **Guiding**: the current step with its odds and two lines, where a
    hit and where a miss go, and the screen watch's line (below); the item
    as last grabbed or read; and the history of each step's outcome, each
    marked "from the screen" or "from a grab". After taking the step in
    the game, hover the item: the app reads it from the screen, classifies
    the change as a hit or a miss and moves on. Or copy the item and grab
    it again, which the app classifies the same way as a hit, a miss or
    unexpected. A clean-up step the app named as a whittle or an
    annulment is judged against the one it named, and decided again on
    the next item when it repeats: a whittle taken where the app said to
    annul (one modifier out and one in) reads as unexpected, and the craft
    asks to plan again, as with any currency other than the one shown.
    On a state-ordered route the step is picked from the item after each
    outcome, so "Step <n>" counts the steps in the order this craft takes
    them; the hit line names the preview's next step and the miss line
    says the next step is picked from the item after the miss.
    *End craft* goes back to planning on the latest item, keeping the
    targets.

  Each grab's text, as copied, is under a *Grabbed text* disclosure with
  its line count and where it came from ("from the hotkey" or "from a
  paste"). A text whose first line does not start with `Item Class: ` is
  still shown, with a warning that it does not look like an item copied
  with `Ctrl+Alt+C`. A grab and the craft are kept in memory only: nothing
  of either is saved, and the next grab replaces the item.
- **A grab is refused** when the clipboard or the paste holds no text, only
  whitespace, or more than 65,536 bytes (UTF-8), or when the clipboard could
  not be read. The status line says why and the last grab stays. A refused
  grab from the hotkey also plays a short tone, since the window may be
  behind the game; a refused paste does not.
- **The hotkey can be changed** in the Settings window. It can never be
  `Ctrl+Alt+C`, in any spelling, because that is the game's own item copy.
  Windows lets one program hold a key combination: when another program
  holds the one you chose, the app says so, keeps running and keeps the old
  hotkey.
- **Explore more crafting on Craft of Exile**, a button in the main
  window's header, opens one fixed address,
  `https://beta.craftofexile.com/?game=poe2`, in your default web browser.
  It is the only address the app opens (Privacy).
- **A Settings window**, opened from the *Settings* button at the right of
  the main window's header, holds four sections, *Grab* (the hotkey),
  *Overlay* (where the overlay sits, and whether the app reads the item
  from the screen), *Blocked currencies* and *Updates*, and the line
  "Created by ST4H". While a
  check has found a newer version, the *Settings* button says so in words
  (`Update <version>`, beside a green dot).
- **Blocked currencies**: a saved list of crafting items (currencies,
  omens, bones, fluxes, catalysts, essences, alloys and liquid emotions)
  the helper is never to suggest, picked by name from the whole catalogue.
  A route avoids them: a step whose first choice is blocked falls back to
  an alternative and says so, and a blueprint that cannot do without one
  does not apply. Each craft starts from this list and can change it for
  itself alone.
- **An update check, only when you ask.** *Check now* asks GitHub once
  whether a newer Crafting Helper release is published; with *Automatically
  check for updates* on (off by default), the app also asks at launch and
  then once a day. A check only checks: nothing is downloaded until you
  press the install button.
- **The update installs in the app.** *Install <version> and restart*
  downloads the release's `crafting-helper.exe`, checks its signature
  against the key built into the app, puts it in place of the running exe
  and restarts into it, with a progress bar and *Cancel*. A failed step says
  why in one line, with *Try again*, and leaves the running app untouched;
  a newer release that is not signed says it cannot be installed and offers
  no button. An update never opens a release page or a browser
  ([`docs/app-updates.md`](../app-updates.md)).
- **Reading the item from the screen.** While a craft is guiding, the game
  is the window in front (its title is `Path of Exile 2`) and *Read the
  item from the screen* is on (the default), the app copies the game's
  client area at most twice a second, finds the item tooltips in the copy
  and reads each with Windows' own OCR (en-US), only when its crop
  changed. Hover the item after taking a step, without Alt: the plain
  tooltip is the one the app reads best. A change it reads twice alike, at
  least 0.4 s apart, is the step's result: a hit or a miss advances the
  guide as a grab does, with the screen as its source. Nothing else
  changes the guide: no tooltip, another item, the item unchanged, a read
  it cannot trust (a line it cannot account for, a value that fits two
  tiers, an unclear rarity; it says why and to grab the item), a tooltip
  under the overlay, or a change the guide did not expect (it says to grab
  the item, and does not apply that change again). A grab always works and wins over a read in
  flight; a grab of the item the screen just read is not a second outcome
  ("This is the item the screen read: the grab replaces it, and the craft
  is unchanged."). One line in the guiding card and the overlay says what
  the watch sees and what to do ([`ui-design.md`](ui-design.md) § "The
  screen watch's line"). With the setting off, on a Windows without English
  OCR, or with the game not in front, nothing is read and grabs drive the
  guide.
- **An overlay over the game**, a click-through, always-on-top panel, 480
  by 168 at 1x, that shows the guide's status, the step and its try, the
  instruction, what it uses and its odds, where a hit and a miss
  go, the last outcome with its source, and the screen watch's line. It
  shows while a craft is guiding (in any guide status: crafting, stuck,
  replan or done) **and** the game's window is in front; it hides at once
  when the craft ends, another window comes to the front (the app's own
  windows included) or the game is minimized. It sits inside the game's
  client area, 12 physical pixels from its edges, at the place chosen in
  Settings: `top-center` by default, or top left, top right, bottom left,
  bottom center or bottom right. It never takes focus and every click and
  the cursor pass through it to the game ([`ui-design.md`](ui-design.md)
  § "Overlay").
- **What the craft window is built on**: the modifier data's lookups and
  odds (`ch-data`), the hand-edited blueprints under
  `apps/crafting-helper/blueprints/` and their format
  ([`blueprints.md`](blueprints.md)), and the engine that picks a blueprint
  route for an item and classifies each step as hit, miss or unexpected
  (`ch-engine`), all built into the exe. The rules the engine follows on a
  state-ordered blueprint are written down first, in
  [`crafting-slots-spec.md`](../models/crafting-slots-spec.md), and
  modelled in stdlib Python; the engine is held to that model by tests
  (Working on the app).

## Platforms and prerequisites

- **Windows 10 or 11, 64-bit.**
- **The Microsoft Edge WebView2 runtime.** Windows 11 ships it, and most
  Windows 10 machines have it through Edge. `crafting-helper.exe` does not
  bundle it; without it the window cannot start.
- **The game in English.** The app's check of the first line,
  `Item Class: `, is English text, and the screen watch reads English.
- **For reading the item from the screen: Windows' English OCR**
  (`Windows.Media.Ocr`, en-US), as for the quest tracker. Without it
  the guiding card and the overlay say so (`Can't read the screen (no English OCR): grab the
  item after each step.`) and the guide takes grabs alone. The game in a
  window or borderless; exclusive fullscreen is not supported.

To build it from this checkout you also need the Rust toolchain for the MSVC
target (`cargo`; built here with cargo 1.98.1), the Visual Studio C++ build
tools, and Node.js with npm. Every crate and npm package is pinned exactly,
to the quest tracker's versions; build with `--locked` and `npm ci`.

## Command reference

### `crafting-helper.exe`

Takes no arguments.

| Environment variable | Effect |
|---|---|
| `CRAFTING_HELPER_DIR` | The folder for `settings.json` and `diag.log`. Unset or empty: `%LOCALAPPDATA%\ExilePact2\crafting-helper\`. A folder inside a git checkout is refused: the app then runs without saving, and the status line says why. |

- `settings.json`: `schema` (1), `hotkey` (default `Ctrl+Alt+X`),
  `check_updates` (*Automatically check for updates*; any value but `true`,
  or none, loads it off), `blocked_currencies` (a list of catalogue
  ids), `overlay_position` (where the overlay sits: `top-left`,
  `top-center`, `top-right`, `bottom-left`, `bottom-center` or
  `bottom-right`; any other value, or none, loads the default,
  `top-center`) and `screen_watch` (*Read the item from the screen*; any
  value but `false`, or none, loads it on). A craft's own changes to the
  blocked list are never saved. Written a
  moment after each change, through a temporary file renamed over the old
  one, and when the app quits. A saved hotkey of `Ctrl+Alt+C` is replaced by
  the default at launch, and the status line says so. A saved list of
  blocked currencies keeps its valid ids and drops the rest (see the
  Settings window below for what is valid), and the status line says how
  many were dropped.
- `diag.log`: one line per event, with the UTC time. Every line the app
  writes:
  - `settings: <n>` at launch, for each note about the store or the saved
    settings that the status line also shows: the folder refused, the
    settings unreadable, the saved hotkey replaced, saved ids dropped, or
    the shipped blueprints not loading (then no craft can be planned).
  - `start: crafting-helper <version>; store <folder>` (`store none` when
    the folder was refused).
  - `hotkey <hk> registered` or `hotkey <hk> could not be registered:
    <error>`, at launch and when the hotkey is changed; `hotkey <hk>
    refused: it is the game's item copy` for `Ctrl+Alt+C`.
  - `grab from the shortcut: N lines`, `grab from a paste: N lines` or
    `grab refused: <reason>` (for example `the clipboard holds no text`,
    `the paste is empty`, `the clipboard holds N bytes of text, over
    65536`, `the clipboard could not be read (<error>)`). **The grabbed
    text is never written**, to the log or anywhere else.
  - After a refused grab from the hotkey, `refusal beep: <sound> ok` or
    `refusal beep: <sound> failed (<error>)` for each sound tried, or
    `refusal beep: none on this platform` where the app has no beep
    (outside Windows).
  - `overlay: click-through` at launch, or `error: overlay window: <e>`
    (`click-through: <e>`, `place: <e>`, `handle: <e>`, `not created`)
    when the overlay could not be made click-through, placed, shown or
    hidden. Each time it changes: `overlay: shown`, `overlay: hidden (not
    guiding)` or `overlay: hidden (game not in front)`, and `overlay:
    placed <position>` when the game's client area, the setting or the
    overlay's size moved it; `error: overlay: its size is unknown: <e>`
    once, when the overlay's size could not be read (it is shown where it
    is).
  - The craft window's commands, one line each: `craft: <n> targets` (`1 target`),
    `craft: <n> blocked for this craft`, `craft: route <blueprint id>`,
    `craft: accepted <blueprint id>`, `craft: ended`, or `craft: refused:
    <reason>` for one the app did not take. While guiding, each grab that
    is a step's result adds `craft: hit at <step id>`, `craft: miss at
    <step id>` or `craft: unexpected at <step id>`, and a grab of the item
    the screen just read `craft: grab matches the screen`. Each change read
    from the screen adds `craft: hit at <step id> (from the screen)` or
    `craft: miss at <step id> (from the screen)`, or `watch: unexpected
    change at <step id> not applied`. Only counts, ids (blueprints, steps,
    modifiers) and reasons: no item text.
  - The screen watch's lines, each written when what it says changes,
    never per read. None holds OCR text, a window title or a pixel:
    - `watch: on (OCR en-US)` or `watch: refused: no English OCR`, at the
      first read;
    - `watch: idle (<reason>)`: `not guiding`, `guide not active` (stuck,
      replan or done), `screen watch off`, `game not in front` or
      `minimized`; `watch: idle (not guiding)` is also written once at
      start, before any craft, when the app has a screen backend (on
      Windows);
    - `watch: game window <w>x<h>`, the game's client area in physical
      pixels, when the game comes to the front or its size changes;
    - `watch: tooltip <state>`: `none`, `unchanged`, `another item`,
      `unreadable`, `covered by the overlay` or `changed`;
    - `watch: black: 10 copies in a row`, when the copy of the window
      comes back black;
    - `watch: minute: <n> copies, <n> tooltips, <n> OCR reads, busy <n>
      ms`, once a minute while guiding (a failed OCR call is not an OCR
      read: it is the `error: watch:` line below);
    - `error: watch: <call> failed: <system error>`, once until that call
      succeeds; `error: the watch thread did not start: <e>`; and `error:
      watch: did not end within <n> ms` when the app quits during a slow
      read.
  - `overlay position: <position>` and `screen watch: on` or `screen watch:
    off` when the Settings window changes them.
  - `opened Craft of Exile` after the button, or `error: opening Craft of
    Exile: <e>` when the browser could not be asked to open it.
  - `blocked currencies: N ids` when the list changes; `settings refused:
    <reason>` when the Settings window sends a change the app does not
    take; `error: settings window: <reason>` when that window could not be
    opened; `error: settings could not be saved: <e>`.
  - `updates: ...` lines, the same as the waystone roller's (its guide's
    Command reference): the setting turned on or off, each check's start
    and outcome, each install step, failure, cancel or refusal, the restart,
    and at every start one `updates: cleanup: ...` line
    (`removed crafting-helper.exe.old` when an install left one). No line
    holds a URL or anything of GitHub's answer.
  - `stop` when the app quits.

  Rotated at 1 MB into `diag.log.1`.

**The main window**, titled "Crafting Helper", 960 by 680 at 1x by default
and usable from 600 by 480. Its header holds the title, the *Explore more
crafting on Craft of Exile* button and the *Settings* button; under it, the
craft area, in two columns from 880 px wide and one below; last, the status
line. With nothing grabbed, the craft area says how to grab (`Ctrl+Alt+C`
in the game, then the current hotkey or `Ctrl+V` here). The status line is
the app's last message: `Grabbed an item: N lines.` (with how many targets
the new item dropped, when it cannot roll them), `Not grabbed: <reason>.`,
the warning for a text that does not look like an item, what a craft
command did or why it was refused (`Targets not changed: <reason>.` and the
like, with nothing changed), a step's outcome while guiding (ending
`Read from the screen.` when the screen watch read it), an unexpected
change read from the screen that was not applied, why a grab
while guiding is not a step's result (the same item again, the item the
screen just read, or not an item a blueprint plans for; the craft is
unchanged), or a problem with the hotkey,
the saved settings or opening Craft of Exile. Each target refers to a
modifier this base can roll at its item level, with no floor ("As high as
possible", the best tier) or a tier the item level reaches; a list with
anything else is refused whole. The window's parts and their words are
[`ui-design.md`](ui-design.md) § "Craft window".

**The Settings window**, titled "Crafting Helper Settings", opens from the
main window's *Settings* button (shown and focused when it is already
open). It is decorated, dark and not resizable, 440 by 720 at 1x, centred;
closing the main window quits the app with it. From the top:

- **Grab**: the current hotkey, a field to type a new one (`Ctrl+Alt+G`,
  for example) with *Set*, and what the app said about the last change:
  `The hotkey is now <hk>.`, `<hk> is the game's own item copy, so it
  cannot be the hotkey. The hotkey is still <old>.`, `Hotkey <hk> is taken
  by another program. The hotkey is still <old>.` and the like.
- **Overlay**: *Position*, a list of the six places ("Top left", "Top
  center", "Top right", "Bottom left", "Bottom center", "Bottom right";
  "Top center" by default), with the help line "Where the step panel sits
  in the game's window while a craft is guided."; and *Read the item from
  the screen*, a switch, on by default, with the help line "After each
  step, hover the item in the game and the helper reads it. Copies of the
  screen stay in memory; nothing is saved or sent. Off: grab the item
  after each step." Each saves itself, and the overlay is placed again
  when its position changes. With this section the page is taller than the
  window and scrolls as a whole.
- **Blocked currencies**: the count, then the catalogue as checkboxes by
  name, grouped by kind (Currency, Omens, Bones, Fluxes, Catalysts,
  Essences, Alloys, Liquid emotions), with a *Filter currencies* field and a
  *Blocked only* checkbox; there is no field to type an id. Each change
  sends the whole list of ids; the app trims each id, drops empty ones and
  repeats, and saves the list it kept. The catalogue is the list's limit:
  every id must be one of its ids (about 220), so the list can hold the
  whole catalogue and never more. An id outside the catalogue, an id over
  64 characters, or anything but a list of text is refused whole, with
  nothing changed. A saved id the catalogue no longer holds is dropped at
  launch and counted in the status line. The craft window's *Blocked for
  this craft* uses the same picker over the craft's own list.
- **Updates**, the waystone roller's section: *Automatically check for
  updates* (off), with the helper text "Asks GitHub once a day whether a
  newer version is out. Downloads nothing: an update is installed only when
  you press its button. Off: Crafting Helper never connects unless you ask
  it to."; what the last check said (`Not checked yet.`, `Checking for
  updates…`, `You have the latest version (<running>).`, `Crafting Helper
  <new> is available. You have <running>.`, `No Crafting Helper release is
  published yet.` or `Could not check for updates: <reason>.`); the
  install's line and its one button (*Install <new> and restart*, the
  download with its bar and *Cancel*, *Try again* after a failure, or none
  for an unsigned release); **Check now**, disabled while a check or an
  install runs; and the running version. The reasons are the quest
  tracker's (its guide's Command reference). With the check on, it runs at
  launch, again when it is turned on, and then a day after the last check
  started, or an hour after an automatic check failed.
- "Created by ST4H" at the bottom.

The main window's commands are `get_view`, `grab_text` (the paste's text),
`open_settings`, and the craft window's `set_targets` (the whole list of
targets, each a modifier id and a floor or none), `set_craft_blocked` (the
craft's whole blocked list, catalogue ids only; never saved),
`choose_route` (a blueprint id the preview offers), `accept_craft`,
`end_craft` and `open_craft_of_exile`, which takes no argument: the
address is the app's own. Each craft command answers with the view. Both
windows may send `get_catalogue`, which answers the catalogue grouped by
kind, each entry's id and English name. The main window is sent `ch:view`,
which carries the craft part (its mode and what the mode shows), the
hotkey, the last grab, the status line, the overlay's state (whether it
shows, its position and, while guiding, the step it shows) and
`update_available`; while guiding, the craft part carries the screen
watch's line. The Settings window's commands are `get_app_settings`,
`set_hotkey`,
`update_settings` (an object of `check_updates`, `blocked_currencies`,
`overlay_position` and `screen_watch`, any of them, and nothing else; a
position the app does not know is refused whole), `check_for_updates`, `install_update`
(the only way an install starts) and `cancel_update`; it is sent
`ch:app-settings`. The overlay listens to `ch:view` and sends nothing but
`get_view`. The windows load their pages while the app still sets itself
up, so a page's first `get_view`, `get_app_settings` or `get_catalogue`
can be refused with Tauri's "state not managed"; the page asks again every
150 ms, for up to 10 seconds, and passes any other refusal on at once.
The newest release is the highest published, non-prerelease
release tagged `crafting-helper-v<major>.<minor>.<patch>`.

### npm scripts

Run from `apps/crafting-helper/` (or with `npm --prefix apps/crafting-helper`).

| Script | What it does |
|---|---|
| `npm run dev` | The pages alone in a browser at `http://127.0.0.1:1422/`, over the view fixtures (`?state=empty`, `?state=craft-planning`, `settings.html?state=overlay-settings`, `overlay.html?state=overlay-active`; [`ui-design.md`](ui-design.md) § "Browser preview"). |
| `npm run build` | Type-checks the pages and writes `dist/`, which the executable embeds. Needed before any `cargo` build of the `crafting-helper` package. |
| `npm test` | The pages' tests, in jsdom. |
| `npm run start` | The app under `tauri dev`. |
| `npm run release` | `tauri build --no-bundle`: `target/release/crafting-helper.exe`. |

The data extractor, `py -3 -m tools.crafting_helper.poe2db`, is described in
the research note:
[`crafting-helper-research.md#data-extractor`](crafting-helper-research.md#data-extractor).

## Privacy

- **The clipboard is read only when you grab.** A press of the app's hotkey
  reads the clipboard's text once (if another program holds the clipboard,
  the app tries again for about 100 ms, then refuses). A `Ctrl+V` in the
  app's window hands the page the text of that paste, which it sends to the
  app; nothing else reads the clipboard for it. The app **never watches the
  clipboard**: no read at launch, on a timer, on focus or from any other
  command, no clipboard listener and no change counter. It never writes to
  the clipboard.
- **No keyboard hook.** The hotkey is a registered system hotkey (Windows'
  `RegisterHotKey`, through Tauri's global-shortcut plugin): Windows tells
  the app when that one combination is pressed, and the app sees no other
  key. It does not poll the keyboard's state either.
- **The game's window is copied only while a craft is guiding, the game
  is in front and *Read the item from the screen* is on.** While a craft is
  guiding, the app asks Windows which window is in front, four times a
  second, and compares that window's title with `Path of Exile 2` where it
  reads it: the title of any other window is dropped at once, never kept,
  logged or passed on. Only when it is the game, the guide is active and
  the setting is on does it copy the game's client area (a GDI copy, at
  most twice a second), and read the tooltips in it with Windows' own OCR,
  on this machine. The copies, the tooltip crops and the OCR text are
  kept in memory while a read is processed (the last read's crops and
  lines, to skip an unchanged one), never written to a file and never
  logged: `diag.log` gets only states, counts, ids and reasons. Nothing of
  them is sent anywhere. With no craft guiding, the app looks at no window
  at all. It reads no window but the game's, and never the cursor's
  position.
- **It sends the game nothing.** No key, no mouse input or movement, no
  click, no typed text, no window message, no focus or activation change,
  no simulated Alt, no other process opened (but the web browser Windows
  starts when you press the Craft of Exile button, below), no game memory
  read and none of the game's files. Hovering the item is yours to do
  (without Alt: the plain tooltip reads best). The overlay never takes
  focus and lets every click and the cursor through.
- **What it keeps of a grab**: the text, in memory, until the next grab or
  until the app quits, and the craft built on it (the item, the targets,
  the craft's blocked changes, the guide and its history, and the item the
  screen last read, as modifiers, not text), in memory until the app
  quits. None of it is saved, and none of the text is logged; `diag.log`
  gets only the line count or the reason a grab was refused, and the
  craft's counts, ids, refusal reasons and outcome words.
- **Its other effects outside its windows** are the overlay over the game
  while a craft is guided, the refusal tone, the
  update check's requests to GitHub, opening Craft of Exile in the default
  browser when you press its button and, when you press the install
  button, replacing its own exe.
- **One web address, opened only by its button.** *Explore more crafting
  on Craft of Exile* asks Windows to open
  `https://beta.craftofexile.com/?game=poe2` in your default browser, and
  nothing else: the address is fixed in the app (`ch-app`'s
  `CRAFT_OF_EXILE_URL`), no command takes an address, and the pages hold
  none and cannot open one themselves (no link, no `window.open`; the
  opener plugin's own commands are granted to no window). The app itself
  sends nothing for it, and nothing of your item, targets or craft goes
  with it; the page loads in your browser, which connects to Craft of
  Exile as for any page you open (Craft of Exile is not part of this
  project). The guard pins each of these
  (`tests.test_crafting_helper_guard`).
- **Network: only the update check and the install, only when you ask.**
  With *Automatically check for updates* off (the default) and no *Check
  now* pressed, the app sends and receives nothing (the Craft of Exile page
  is your browser's connection, not the app's). A check is one `GET
  https://api.github.com/repos/S-Borkowski/ExilePact2/releases?per_page=30`,
  port 443 only, from Rust through Windows' own WinHTTP, with three
  headers: `Accept: application/vnd.github+json`, `X-GitHub-Api-Version:
  2022-11-28` and `User-Agent: ExilePact2-crafting-helper` (no version, no
  machine detail). The answer is read up to 2 MiB within 20 s, used only to
  find the newest release, and never stored or logged.
- **An install** (the button only) makes two downloads, the signature, then
  the exe: `GET
  https://github.com/S-Borkowski/ExilePact2/releases/download/crafting-helper-v<version>/crafting-helper.exe.minisig`
  and the same path ending `crafting-helper.exe`, each with two headers,
  `Accept: application/octet-stream` and the same `User-Agent`. GitHub
  answers each with a redirect to a signed address on
  `release-assets.githubusercontent.com`; the app follows that one redirect,
  only over HTTPS on port 443 and only to that host or
  `objects.githubusercontent.com`, with the same two headers, and never logs
  or keeps the address. Each download stops at the size the release lists
  (at most 64 MiB for the exe, 4 KiB for the signature), after 30 s without
  a byte, or after 15 minutes in all.
- For every request: no cookie and no credential is sent, the proxy is the
  system's, and TLS and its certificate checks are Windows'. GitHub sees
  your IP address and those headers; nothing else leaves. The app's code
  outside the shared `ep-net` crate has no network code
  (`tests.test_crafting_helper_guard`). (WebView2, Microsoft's runtime that
  draws the windows, may connect on its own, as for the quest tracker.) The
  data extractor asks poe2db.tw for pages only when a developer runs it.
- **What it writes**: `settings.json` and `diag.log` in its folder (Command
  reference). The log holds the hotkey, the grabs' line counts, refusals,
  the blocked list's length, the `craft:` lines, the `overlay:` and
  `watch:` lines (states, counts and the game's client size), the Craft of
  Exile line and the `updates:` lines; nothing about your account or
  character, no item text, no OCR text, no window title, no image, and
  nothing of GitHub's answers. An
  install also writes beside the exe: `crafting-helper.exe.download` while
  it downloads (deleted when the install fails or is cancelled) and
  `crafting-helper.exe.old`, the replaced exe, which the next start removes.

## Working on the app

| Path | What it holds |
|---|---|
| `apps/crafting-helper/crates/ch-app/` | The app's logic without Tauri: the store, the grab's checks and normalisation (`grab.rs`), the hotkey's rules (`hotkey.rs`), the saved settings, the blocked currencies and the catalogue grouped for the pickers (`settings.rs`), the craft window (`craft.rs`: its four modes in memory, what a grab does to it, its commands' rules, its view from the engine's plan, guide and outcomes, the copy the design marks Rust's, and `CRAFT_OF_EXILE_URL`), the views (`view.rs`, the IPC contract `src/ipc.ts` mirrors), the commands, the overlay over the game (`overlay.rs`: where it sits from the game's client area, the setting and its size, and the step it shows), the screen watch (`screen.rs`: the `ScreenBackend` trait the glue implements, `GAME_WINDOW_TITLE`, the watch thread with its timings, and the watch's lines; `craft.rs`'s `Craft::screen_change` applies a confirmed read), and the update commands and ticker (`updates.rs`, over the shared `ep-update`). Tested through a recording fake shell, a fake screen backend, a fake transport and a temporary store. |
| `apps/crafting-helper/crates/ch-watch/` | The screen watch's pure part, with no thread, clock or Windows call: the locator that finds item tooltips in a copy of the game's client (`locator.rs`), the reader that OCRs each tooltip's crop through the calls it is given and reports one under the overlay as covered (`reader.rs`), the matcher that tells no tooltip, covered, another item, unreadable (with the reason), unchanged or changed from the read lines and the item the step was shown on (`matcher.rs`, `text.rs`), and `item_key`, what "the same item" means for the guide. Takes the quest tracker's `qt-screen`, and `qt-capture` for its screenshot tests alone. |
| `apps/quest-tracker/crates/qt-screen/`, `qt-capture/` | The quest tracker's screen crates, taken **by path and unchanged**: `qt-screen` (the image, the boxes, the OCR line, the fuzzy match) by `ch-watch` and the glue, `qt-capture` (the game's client area, the GDI copy, the overlay's own rect and Windows' OCR) by the glue and by `ch-watch`'s tests. Each takes its package keys from its own workspace; a change there is a change to this app ([Maintenance triggers](#maintenance-triggers)). |
| `apps/crafting-helper/crates/ch-item/` | The item text parser, a std-only crate; the craft window parses each grab with it: [`crates/ch-item/README.md`](../../apps/crafting-helper/crates/ch-item/README.md). |
| `apps/crafting-helper/crates/ch-data/` | The modifier data under `data/`, embedded at build time: an item's variant from its base type, what it can roll at its item level, the join of each parsed modifier to its data modifier and tier, the affix limits (`AFFIX_LIMITS`), and one step's add and removal odds, or `unknown` with the reason. |
| `apps/crafting-helper/crates/ch-blueprint/` | The blueprint format: the model (`model.rs`, listed and state order), the static catalogue of currencies, omens, bones, fluxes and catalysts with their ids, and the not-established fact ids `relies_on` may name (`catalogue.rs`), the structural validator whose every error is `<path>:<line>:<col>: <message>` (`validate.rs`), and the shipped blueprints, embedded by `build.rs`. |
| `apps/crafting-helper/crates/ch-engine/` | The engine: blueprints resolved against the data, route choice for an item, its targets and the blocked currencies (`route.rs`), the slot accounting every state-ordered rule reads (`slots.rs`), the state-ordered next step, side rules, placeholders and preview (`state_order.rs`), the guide's outcomes (`guide.rs`), the full catalogue with the data-backed essences, alloys and liquids, and the `check_blueprints` example. `ch-app` re-exports it and plans, guides and fills the blocked list's catalogue with it. |
| `docs/models/crafting-slots-spec.md` | The mechanism behind state-ordered routes, in our own words, each fact labelled documented, measured or not established with its source: the slot accounting, what each crafting item does to the slots, the route rules, the preview and the representative crafts. The model and the engine follow it; a gap is fixed there first. |
| `apps/crafting-helper/blueprints/` | The shipped blueprints, one TOML file each: how to write one is [`blueprints.md`](blueprints.md). |
| `apps/crafting-helper/src-tauri/` | The Tauri glue: the global hotkey, single instance, the Win32 clipboard read and the refusal tone, the commands, the Settings window (label `settings`) and the overlay (label `overlay`, made click-through at start, moved where `ch-app` places it and shown without taking focus), the screen watch's real backend (`screen.rs`, over `qt-capture` and `ch-watch`'s reader) and the one file that reads the window in front and its title (`game_window.rs`), each window with its capability under `capabilities/`, opening Craft of Exile's address through `tauri-plugin-opener` from Rust alone (registered without its link handler, no `opener:` permission in any capability), the update transport over the shared `ep-net`, and the restart into an installed update. |
| `apps/crafting-helper/src/` | The main page (`App.svelte`; the shared parts under `components/`: the grab prompt, the grabbed text, the status line and `CurrencyPicker.svelte`; the craft area under `craft/`, one component per part, from `CraftArea.svelte`; the commands `bridge.ts`, the paste route `paste.ts`, the view's types `ipc.ts`), the Settings window's page under `src/settings/` (`settings.html`, one component per section, listed in `sections.ts`) and the overlay's under `src/overlay/` (`overlay.html`, `Overlay.svelte`: the step panel). |
| `apps/crafting-helper/theme/` | The quest tracker's theme, copied byte for byte (`theme.test.ts`). |
| `apps/crafting-helper/data/` | The modifier data the extractor writes ([research note](crafting-helper-research.md#data-files)), embedded by `ch-data` when the app is built. |
| `apps/crafting-helper/fixtures/` | `views/`, one main window view per state (the craft window's included, [`views/README.md`](../../apps/crafting-helper/fixtures/views/README.md)), `app-settings/`, one Settings window view per update, install and settings state, and `catalogue.json`, the `get_catalogue` answer, shared by the page's tests and `ch-app`'s. |
| `apps/shared/` | The update code the three apps share, its own Cargo workspace: `ep-net` (the only network code) and `ep-update`: [`docs/app-updates.md`](../app-updates.md). |
| `tools/crafting_helper/` | The poe2db data extractor (`poe2db.py`), the release cutter (`release.py`) and the slot model (`slot_model.py`, stdlib only, built from the spec alone: `py -3 -m tools.crafting_helper.slot_model check` plans every case of `cases.json` and names each difference). |
| `.github/workflows/crafting-helper-release.yml` | The release build: from a pushed tag to a draft release. |

Tests (none opens a window, reads a real clipboard, copies the screen or
reaches the network, but the three marked as reading images or OCR):

```sh
npm --prefix apps/crafting-helper run build                                            # needed before the workspace build
cargo test --manifest-path apps/crafting-helper/Cargo.toml --workspace --locked         # every crate, ch-item included
cargo test --manifest-path apps/crafting-helper/Cargo.toml -p ch-app --locked --test grab     # the grab, fake shell
cargo test --manifest-path apps/crafting-helper/Cargo.toml -p ch-app --locked --test hotkey   # the hotkey's rules
cargo test --manifest-path apps/crafting-helper/Cargo.toml -p ch-app --locked --test updates  # the update commands, fake transport
cargo test --manifest-path apps/crafting-helper/Cargo.toml -p ch-app --locked --test craft_window  # the craft window, fake shell
cargo test --manifest-path apps/crafting-helper/Cargo.toml -p ch-app --locked --test remarks  # whittle or annul, removal and branch copy, fake shell
cargo test --manifest-path apps/crafting-helper/Cargo.toml -p ch-app --locked --test state_order  # the craft window on state-ordered routes, fake shell
cargo test --manifest-path apps/crafting-helper/Cargo.toml -p ch-app --locked --test settings # the saved settings, the blocked list, the catalogue
cargo test --manifest-path apps/crafting-helper/Cargo.toml -p ch-app --locked --test overlay  # the overlay: when it shows, where it sits, what it says
cargo test --manifest-path apps/crafting-helper/Cargo.toml -p ch-app --locked --test watch    # the screen watch, fake backend
cargo test --manifest-path apps/crafting-helper/Cargo.toml -p ch-watch --locked        # the locator, reader and matcher, synthetic images and lines
CH_TOOLTIP_SHOTS="$USERPROFILE/crafting-helper-shots" cargo test --manifest-path apps/crafting-helper/Cargo.toml -p ch-watch --locked --test shots -- --include-ignored  # reads images and OCR: the owner's screenshots
cargo run --release --manifest-path apps/crafting-helper/Cargo.toml -p ch-watch --locked --example shot_timings -- <image.png> [left top right bottom]  # reads an image and OCR: one shot's times
cargo test --manifest-path apps/quest-tracker/Cargo.toml -p qt-capture --locked -- --include-ignored  # reads OCR: qt-capture's own tests and its OCR self-test
cargo test --manifest-path apps/crafting-helper/Cargo.toml -p ch-data --locked         # the data: join, valid modifiers, odds
cargo test --manifest-path apps/crafting-helper/Cargo.toml -p ch-blueprint --locked    # the format, the catalogue, the validator
cargo test --manifest-path apps/crafting-helper/Cargo.toml -p ch-engine --locked       # route choice, outcomes, the seeds' paths, state order, the parity test
cargo test --manifest-path apps/crafting-helper/Cargo.toml -p ch-engine --locked --test slots  # the parity test: the engine against the slot model's cases
py -3 -m unittest tests.test_crafting_helper_slot_model                                # the slot model: its baselines, cases and invariants
cargo run --manifest-path apps/crafting-helper/Cargo.toml -p ch-engine --locked --example check_blueprints -- apps/crafting-helper/blueprints  # check blueprints
npm --prefix apps/crafting-helper test                                                  # the pages, in jsdom
py -3 -m unittest tests.test_crafting_helper_guard                                      # the guard
py -3 -m unittest tests.test_crafting_helper_release                                    # the release cutter and workflow
py -3 -m unittest tests.test_crafting_helper_poe2db tests.test_crafting_helper_data     # the data extractor and the shipped data
cargo test --manifest-path apps/shared/Cargo.toml --locked --workspace                   # the shared update crates, 127.0.0.1 only
py -3 -m unittest tests.test_app_updates_guard                                          # the shared crates' guard
```

- `ch-app`'s `tests/grab.rs` counts the fake shell's clipboard reads: one
  per hotkey press (`grab_reads_the_clipboard_once_per_press`) and none for
  a paste (`grab_from_paste_reads_no_clipboard`). `tests/hotkey.rs` covers
  the default and `Ctrl+Alt+C` in every spelling; `tests/settings.rs` the
  saved settings, the blocked list (catalogue ids only, the whole
  catalogue allowed) and the catalogue's groups, `overlay_position` and
  `screen_watch` included; `tests/overlay.rs` the overlay over the game:
  hidden until a craft is guiding with the game in front
  (`overlay_hidden_until_a_craft_is_active`,
  `overlay_shows_the_step_while_guiding_with_the_game_in_front`,
  `overlay_hides_when_the_game_is_not_in_front`), its lines
  (`overlay_lines_follow_the_step_and_the_last_outcome`), its place at
  each position (`overlay_is_placed_at_the_chosen_position`), no backend
  call unless guiding (`no_backend_call_unless_guiding`), the watch
  ended at shutdown (`shutdown_stops_the_watch`), the overlay's step
  naming its target modifiers (`overlay_step_names_the_mods_it_targets`)
  and one `watch: idle (not guiding)` at start
  (`watch_idle_is_logged_once_at_start`).
- `ch-app`'s `tests/watch.rs` drives the screen watch over the whittled
  ring with a fake screen backend and ticks driven by the test: a hit read
  twice alike advances (`screen_hit_advances_after_two_equal_reads`), a
  miss repeats the step, an unexpected change is not applied, an unchanged
  tooltip or another item changes nothing, a grab wins over a read in
  flight (`a_grab_wins_over_a_read_in_flight`), a grab matching the screen
  read is not an outcome, the backend is called without the app's lock
  held, no English OCR stops the watch with a reason, a tooltip under the
  overlay is reported, the watch line follows the game and the guide, the
  black and minute lines are logged, an unreadable reason that lies in the
  item says to grab after each step while another keeps the hover line
  (`an_unreadable_item_says_to_grab_after_each_step`), and with
  `screen_watch` off nothing is read and a grab still advances
  (`screen_watch_off_reads_nothing_and_a_grab_still_advances`).
- `ch-watch`'s tests: `tests/matcher.rs` over tooltip lines built in the
  test from the ring texts (the plain and Alt forms, OCR confusions,
  hybrid lines, unchanged, another base, an unaccounted line or an
  ambiguous tier read as unreadable, rarity, the join, the lines
  measured on the screenshots, and a rare belt with a charm slot line, a
  flask implicit and the requirement line as OCR broke it on shot 13,
  `a_belt_with_charm_slots_and_a_flask_implicit_reads_unchanged`);
  `tests/reader.rs` over synthetic tooltips
  drawn in the test (the locator finds one and nothing on a plain scene,
  a box under the overlay is covered, an unchanged crop is OCR read once);
  no image file is committed. `tests/shots.rs` reads the owner's
  screenshots and their item copies from the folder `CH_TOOLTIP_SHOTS`
  names (the command above): the set is whole, whole-image OCR reads the
  copied modifier lines (the positive control), the located boxes match
  the hand-measured ones in `tests/shots.txt`, each plain read gives its
  copy or says unreadable, never another item, the belt Live 1 could not
  read (shot 13) reads as its copy (`the_belt_shot_reads_unchanged`), and
  a shot without a tooltip finds none. Its tests are `#[ignore]`d, so they run only on a
  machine that holds the shots; the images and copies stay there, outside
  every repository. The research note's
  [Item tooltip on screen](crafting-helper-research.md#item-tooltip-on-screen)
  records what they measured.
- `cargo test --workspace` here runs no `qt-*` test: `qt-capture`'s own
  tests, with `--include-ignored` for its OCR self-test
  (`ocr_selftest_reads_rendered_names`, which renders names with GDI and
  reads them back), run from the quest tracker's workspace (the command
  above). It is the OCR's positive control on a machine.
- The glue's `build.rs` links the Visual C++ runtime statically into the
  app's exe alone, so the exe needs no `VCRUNTIME140.dll`. tauri-build's
  own way of doing it puts a stand-in `msvcrt.lib` on a search path that
  Cargo hands to every doctest of a `cargo test --workspace` run, where
  it broke the `ch-blueprint` doctest's link (`LNK1120`); the build
  script turns that off and passes the same libraries to the binary's
  link alone.
- `ch-app`'s `tests/craft_window.rs` drives the craft window over the
  whittled ring of `ch-engine`'s tests
  (`crates/ch-engine/tests/items/gear/rings-resist-whittle/`, read by
  path): the targets a base offers and their floors, the item's own
  modifiers ticked as targets after a grab
  (`a_grab_ticks_the_items_own_mods_as_targets`, none for an item without
  explicit modifiers, the list after a re-grab), refusals, a route with
  odds in words and without the blueprint's notes
  (`the_route_view_drops_the_blueprint_notes`), each step naming its
  target modifiers in an instruction of the app's own
  (`a_targeted_step_says_its_currency_side_and_mods`; a removal step
  names the unwanted modifiers it may take, and a step with no target
  left or a manual step keeps the blueprint's text,
  `a_step_without_targets_keeps_the_blueprint_text`), a blocked
  currency's fallback and a craft's own blocked list, no route,
  accepting, a hit, a miss, an unexpected change, an unchanged re-grab
  and the walk to done; and that `open_craft_of_exile` asks the shell for
  the one fixed address
  (`open_craft_of_exile_opens_only_the_fixed_address`).
- `ch-app`'s `tests/remarks.rs` drives the shipped state-ordered
  `close-a-side` over the owner's 2P/1S belt (`crates/ch-app/tests/items/`,
  constructed; its README lists the states), with Life, Mana and the
  missing Armour as target prefixes and Fire Resistance as a target
  suffix, after the suffix side was closed and the slam on the open
  prefix side missed: the removal is the annulment with Omen of
  Sinistral Annulment when the lowest-level modifier is a target
  (`a_missed_exalt_above_the_lowest_level_is_annulled`) and the whittle
  when it is not (`a_missed_exalt_at_the_lowest_level_is_whittled`), a
  removal step names the modifiers it removes
  (`a_removal_step_names_the_modifier_it_removes`), a miss branch into it
  offers no choice, the branch lines name the next step by its composed
  instruction, the desecrate step names the hardest target (on issue
  #28's ring, `ch-engine`'s slot case `rare-one-target`), an annulment
  the app named is judged as one from a grab or the screen while a
  whittle there reads unexpected
  (`an_annulment_the_app_named_is_judged_as_an_annulment`,
  `an_annulment_read_from_the_screen_is_judged_as_an_annulment`), a
  missed annulment shows the step again with no fallback line, no status
  line names the blueprint (`status_lines_do_not_name_the_blueprint`),
  and the last outcome says its word once
  (`the_last_outcome_says_its_word_once`). These moved off the retired
  `fill-desecrate-exalt` in 0.1.1; the listed whittle-or-annul by level,
  and that the whittle stays when Orb of Annulment is blocked
  (`a_blocked_annulment_keeps_the_whittle`), are `craft.rs`'s unit tests,
  on that blueprint's test copy in `ch-engine`.
- `ch-app`'s `tests/state_order.rs` drives the craft window on
  state-ordered routes: a fill step names its currency, omens and side
  (`a_fill_step_names_its_currency_omens_and_side`), two target prefixes
  and a suffix never get an Exalted Orb without a side omen on any route
  (`two_prefixes_and_a_suffix_never_get_an_unrestricted_exalt`), a rare
  with one target gets a route (`a_rare_with_one_target_gets_a_route`,
  issue #28), the hit line names the preview's next step and the miss
  line says the item picks it, no fallback line for the engine's own
  pick, a removal names its side annulment, and the route's source line
  carries no web address
  (`the_route_source_carries_no_web_address`). Since 0.1.2 the belt's
  Armour, its last target missing, is desecrated on `close-a-side`, so
  where the close, the slam or the clean-up is a test's subject (here and
  in `tests/remarks.rs`) Omen of Sinistral Necromancy is blocked for the
  craft, which keeps that route.
- `ch-app`'s `tests/affix_limits.rs` holds the target choices to the
  item's prefix and suffix limits (issue #32), over the owner's belt and
  `ch-engine`'s whittled ring edited in the test (a modifier taken off, a
  fractured header, the base swapped for a Dusk or Gloam Ring with the
  data's implicit lines): an item with room disables nothing, a full side
  disables every other choice on it with the reason
  (`two_on_the_item_and_one_chosen_disable_the_rest_with_the_reason`),
  unticking re-enables the side (`unticking_re_enables_the_side`), an
  implicit changes the count (`an_implicit_changes_the_count`), a
  desecrated choice counts on its side, a fractured or unrevealed
  modifier no target names holds a slot unticked, `set_targets` refuses
  a list over a limit whole
  (`set_targets_refuses_a_side_over_its_limit`), and a re-grab drops the
  targets past the limit in list order
  (`a_regrab_drops_targets_past_the_limit`).
- `ch-app`'s `tests/last_desecration.rs` drives the last-target
  desecration through the craft window: the belt's last step is the
  desecration and the reveal (`the_last_step_is_a_desecration_and_reveal`),
  an item that already has its desecrated modifier keeps its route
  (`an_already_desecrated_item_keeps_its_route`), and the step taken
  instead of a desecration says why on its Fallback line, for a blocked
  Necromancy omen, an item below level 65 and a body armour missing a
  prefix, and with a blocked alternative of the step's own too, both
  reasons, the desecration's first (`the_desecration_fallback_says_why`).
  The page's tests render `craft-side-full` and `craft-last-desecration`
  (`a_full_side_greys_its_other_choices_with_the_reason`,
  `the_last_target_is_desecrated_or_the_fallback_says_why`).
- `ch-app`'s `tests/fixtures.rs` (`fixtures_are_current`) writes the
  views under `fixtures/views/` and `fixtures/app-settings/` and
  `fixtures/catalogue.json`, the overlay's and the screen watch's views
  made with the fake shell and the fake screen backend, ticks driven by
  the test, and fails when a file differs. After a deliberate change, regenerate them with
  `CH_FIXTURES=1` before `cargo test --manifest-path
  apps/crafting-helper/Cargo.toml -p ch-app --locked --test fixtures`, and
  review the diff. The page's tests render each one.
- `tests.test_crafting_helper_guard` reads the files only: no clipboard
  watching (no clipboard listener, viewer or sequence number) and no
  keyboard polling in any Rust source; no `navigator.clipboard` in the UI
  outside its tests; no input sent, no game access and no network but the
  shared `ep-net`, reached only from `src-tauri`; the window in front and
  a window's title read in the glue's `game_window.rs` alone (with a
  positive control that it does); the quest tracker's `qt-screen` and
  `qt-capture` walked as the app's own code (no input sent; their two
  window-lookup names allowed in `qt-capture`'s `win.rs` alone), and
  `qt-capture` taken only by the glue and `ch-watch`'s tests; no file
  written by `ch-watch`'s library; no Tauri plugin but the
  global shortcut, single instance and opener; one web address in the Rust
  sources, Craft of Exile's, once, in `ch-app`, the opener named and called
  once in the glue and registered without its link handler, and no command
  taking an address or a path (`open_craft_of_exile` takes only the app
  state); no address, `window.open`, change of `location`, new-window link
  or opener command in the pages; the pinned CSP, the three windows'
  capabilities and the overlay window's settings; one version everywhere
  (`ch-item`'s included); every member crate but `ch-item` taking its
  package keys and external dependencies from the workspace; and every
  package already in the quest tracker's locks but `minisign-verify`. The
  one workspace pin the quest tracker's workspace lacks, `toml` (for the
  blueprint files), must be a version its `Cargo.lock` already holds.
- The item parser's own tests and fixtures are described in
  [`crates/ch-item/README.md`](../../apps/crafting-helper/crates/ch-item/README.md).
- `ch-data`'s `tests/data.rs` joins the owner's five real items (the
  parser's `fixtures/real/`) to the data and checks the anchored odds
  (`ring_life_exalt_odds_match_the_weights`); the rest of its items are
  constructed in the test. Its `tests/limits.rs` holds
  `item_affix_limits` to the table (`rare_limits_are_three_and_three`),
  a Contempt emotion's extra slot
  (`contempt_adds_a_slot_on_the_other_side`) and the base implicits that
  raise or lower a side, with their sign
  (`an_implicit_raises_or_lowers_a_side`), over items whose implicit
  lines come from the data's base rows. `ch-blueprint`'s `tests/format.rs` reads the
  files under `tests/fixtures/valid/` and `tests/fixtures/invalid/` and
  checks each error's line and column, and reads the spec's facts table
  against the fact ids `relies_on` may name (`facts_match_the_spec_table`).
  `ch-engine`'s `tests/engine.rs`
  covers route choice and outcomes, and the listed-order baseline: the
  retired `fill-desecrate-exalt`, kept as a test blueprint under
  `tests/items/gear/fill-desecrate-exalt/`, still names a plain Exalted
  Orb and still rejects a rare with one target; `tests/state_order.rs`
  covers state order over its `test-state-*` blueprints; `tests/seeds_gear.rs`,
  `tests/seeds_jewels_weapons.rs` and `tests/seeds_v2.rs` walk each shipped
  blueprint on a hit and
  a miss path, over the item texts under `tests/items/` (each folder's
  `README.md` says which are real and which constructed);
  `tests/last_desecration.rs` holds the last-target desecration on the
  shipped state-ordered blueprints, over the owner's belt and its later
  states under `tests/items/last-desecration/`: the last target on a
  rare with no desecrated modifier is desecrated and revealed
  (`the_last_target_on_an_undesecrated_rare_is_desecrated`), a wrong
  reveal is taken off with Omen of Light and the bone used again
  (`a_wrong_reveal_is_lit_off_and_desecrated_again`), an item with a
  desecrated target and one with two targets missing keep their route,
  and no pool or a blocked Necromancy omen falls back with the reason
  (`a_last_target_outside_the_desecration_pool_falls_back_with_why`,
  `a_blocked_necromancy_omen_falls_back_with_why`); and
  `tests/shipped.rs` validates and resolves every file under
  `blueprints/`. After editing a blueprint, run `check_blueprints` (above)
  and the `ch-engine` tests: [`blueprints.md`](blueprints.md).
- **The slot model and the parity test.** The state-ordered rules are
  written down in
  [`docs/models/crafting-slots-spec.md`](../models/crafting-slots-spec.md)
  first (the decision:
  [`docs/adr/0001-crafting-helper-route-design.md`](../adr/0001-crafting-helper-route-design.md)),
  then modelled from the spec alone in `tools/crafting_helper/slot_model.py`.
  `tests/test_crafting_helper_slot_model.py` holds the model's baselines
  (an Exalted Orb with no side omen can take the prefix slot a 2P/1S craft
  still needs; the listed `fill-desecrate-exalt` names a plain one there
  and rejects a rare with one target) and its targets (every case projects
  as recorded, and the spec's invariants hold, each with a control). The
  model owns
  `apps/crafting-helper/crates/ch-engine/tests/items/slots/cases.json`,
  each case an abstract item, its targets and what each state-ordered
  blueprint must preview on it; `ch-engine`'s `tests/slots.rs`
  (`every_case_projects_as_the_model_records`) builds a constructed item
  per case and asserts every expectation, beside a negative control that
  a changed expectation is reported. A disagreement is settled by the
  spec, never by editing `cases.json` to match the engine. A new
  state-ordered blueprint is held to the model only once the model's
  `BLUEPRINTS`, an expectation in every case and `tests/slots.rs`'s list
  of the four (`V2`) name it. The last-target desecration (0.1.2) is in
  the spec's § "The last-target desecration" and the model's
  `BLUEPRINTS`; a case may name the sides with a desecration pool
  (`desecration_pool`, both when absent), and four cases cover it beside
  the changed expectations of the 2P/1S rares: a rare with a desecrated
  target (unchanged slam), a body armour missing a prefix with a suffix
  pool only (slam), a revealed unwanted desecrated modifier (Omen of
  Light, then the bone again) and the 2P/1S rare with Omen of Sinistral
  Necromancy blocked (as before the rule). The parity test does not
  compare odds or Fallback lines.

**Refreshing the modifier data**: see the research note's
[Data extractor](crafting-helper-research.md#data-extractor).

**Changing the icon**: `src-tauri/icons/source.png` is the one source.
Regenerate the icon files with
`npm --prefix apps/crafting-helper exec tauri icon apps/crafting-helper/src-tauri/icons/source.png -- --output apps/crafting-helper/src-tauri/icons`,
delete what it writes beyond the files `src-tauri/tauri.conf.json`'s
`bundle.icon` names (plus `64x64.png` and `icon.png`), then run
`cargo clean --manifest-path apps/crafting-helper/Cargo.toml --release -p crafting-helper`
before `npm --prefix apps/crafting-helper run release`: an incremental build
can keep the old `icon.ico` in the exe (seen for the waystone roller).

**Cutting a release.** A release is the annotated tag
`crafting-helper-v<version>` on the tip of `main`. The version is the one in
`apps/crafting-helper/`'s `package.json`, `package-lock.json`,
`src-tauri/tauri.conf.json`, the workspace's `Cargo.toml`,
`crates/ch-item/Cargo.toml` (which writes its version literally) and
`Cargo.lock` (every workspace member: the crates under `crates/`,
`ch-watch` included, and `crafting-helper`; the quest tracker's `qt-screen`
and `qt-capture` are in the lock by path but are not members, and keep
their own version). These must agree, and "Release notes" below needs a
`### <version>` section for it:

1. Bump the version in those files (build once so `Cargo.lock` follows),
   and write the version's release notes. Land both on `main` through a
   pull request.
2. On an up-to-date `main`:

   ```sh
   py -3 -m tools.crafting_helper.release check            # every version agrees; the notes exist
   py -3 -m tools.crafting_helper.release notes 0.1.0      # the release body, as it will read
   py -3 -m tools.crafting_helper.release cut --push       # tag HEAD and push the tag
   ```

   `cut` refuses uncommitted changes to tracked files, a HEAD that is not
   the tip of `origin/main`, and a tag that already exists here or on the
   remote. Without `--push` it only creates the tag and prints the push
   command.
3. The pushed tag starts `.github/workflows/crafting-helper-release.yml`. It
   checks the tag against the files, builds with Rust 1.98.1 and Node
   24.19.0 on a Windows runner, signs the exe with the release key, and
   creates a **draft** release. The draft holds `crafting-helper.exe`,
   `SHA256SUMS.txt`, `crafting-helper.exe.minisig` (the signature the app
   checks before it installs an update) and the body from step 2. Without
   the `TAURI_SIGNING_PRIVATE_KEY` secret the sign step fails the run and
   no release is made ([`docs/app-updates.md`](../app-updates.md)).
4. Open the draft on GitHub, check it, publish it, and tick "Set as the
   latest release". The workflow never publishes.

To rebuild a tag, for a failed run or a lost asset, run the workflow by
hand (Actions › Crafting Helper Release › Run workflow) with the tag. If the
release already exists, only its assets are replaced.

## Known limitations

- **Reading the item from the screen was measured in one live session.**
  How the tooltip looks, where it draws and what OCR reads of it were
  measured on the owner's screenshots (2026-10-29: 2560 x 1440
  fullscreen and 1920 x 1080 windowed, HUD scale middle, UI text size
  medium), then in the game in Live 1 (2026-10-29, the exe built from
  commit `0ca9d7f8`, a rare belt on `fill-desecrate-exalt`, a route
  retired in 0.1.1; the research
  note's
  [Item tooltip on screen](crafting-helper-research.md#item-tooltip-on-screen)
  § "Live 1"). There the app's copy of the window read the client as
  2560 x 1440 physical pixels, the crafted belt hovered with nothing used
  read `unchanged`, another item read `another item`, and an Exalted Orb's
  miss was read from the screen and applied about 1 s after the tooltip
  changed, with no grab; a grab after it logged `craft: grab matches the
  screen` and did not apply the outcome a second time. The time from hover to outcome as the player sees it,
  and what the watch reads while Alt is held, were not observed (neither
  has a timestamp of its own). Only that one resolution, at HUD scale
  middle and UI text size medium, ran live; another display scale,
  resolution, HUD scale or text size has not been tried live, and the
  `watch: game window` and `watch: minute:` lines are what to read there.
- **The watch's cost while guiding.** In Live 1, with the game in front
  and no tooltip in view, a minute held 61 to 98 copies at 3.2 to 5.0 s
  busy, about 50 ms for a copy and its locating; with tooltips hovered,
  the busiest minutes were 82 copies, 19 OCR reads and 7.9 s, and 73
  copies, 13 OCR reads and 8.1 s, so up to about 8 s of copy and OCR in a
  minute, on one machine. With the game not in front, a minute counted no
  copies. With the watch off it counts none by inference: the first minute
  line after the switch counted 2 copies and the next 0, and the line
  gives no time per copy, so placing those 2 before the switch is an
  inference, not a measurement (the research note's § "Live 1",
  watch-off). A slower machine has not been measured.
- **What the watch cannot read** it calls unreadable and asks for a grab:
  a modifier the data has no modifier for, on the tooltip or on the item
  the step was shown on (the quarterstaff's `of the Parasite` suffix in
  the shots), an unrevealed desecrated modifier (a row of glyphs OCR reads
  nothing of), a fractured modifier (in no shot), new values that fit two
  tiers, a name colour far from every rarity's, an item with no base
  type, and a tooltip reaching the bottom edge of the game's window, where
  it may be cut off (only the bottom edge is checked). A unique item is in
  no shot. With Alt held, OCR lost some values in the shots: hover without
  Alt, as the watch line says. When the reason lies in the item being
  crafted (a modifier the data does not know, or a tier that cannot be
  told on an item whose own copy has an unrevealed or fractured modifier),
  hovering differently cannot help, and the line says `Can't read the item
  (<reason>): grab the item after each step.` instead. Eight of the ten
  plain equipment shots read as their item's copy and the rest unreadable,
  never another item.
- **A requirement line whose first word OCR breaks** is placed by what
  follows its colon (`Level` and its value, then attributes). A rare
  Utility Belt whose requirement line OCR broke so (the owner's shot 13)
  reads as the belt's copy on that screenshot, and in Live 1 the crafted
  belt hovered with nothing used read `unchanged`. If OCR breaks the
  first word of a requirement line that also reads an `(unmet)` (an
  attribute the character does not meet), the line is not accounted for
  and the item reads as unreadable, never as another item; whether the
  game draws that word on the tooltip at all is not established. Why OCR
  broke the word on shot 13 alone is not established either.
- **Another item of the same base** with the same modifiers and tiers
  looks like the item being crafted: the watch tells items by base type,
  rarity and explicit modifiers (ids and tiers), not by the values it
  rolled. Hover only the item you are crafting.
- **The overlay can cover the tooltip.** A tooltip under the overlay is
  not read (the watch line says to move the overlay in Settings or grab
  the item); the game draws the tooltip beside the hovered item, often at
  the window's left edge, so choose a place clear of where you hover.
- **The overlay does not hide under the game's panels**, has no drag or
  edit mode and no scale setting: its place is one of the six in Settings,
  and its size is fixed.
- **Exclusive fullscreen is not supported** and was not tried: run the
  game windowed or borderless. Copies of the borderless window were never
  black for the quest tracker; should ten copies in a row come back black,
  the watch says `Can't copy the game's window: grab the item after each
  step.`
- **The game's window is known by its title**, `Path of Exile 2`
  (measured with the waystone roller, 2026-10-28). Another window with
  exactly that title, in front, would be taken for the game.
- **A craft is not saved.** The item, the targets, the craft's blocked
  changes, the guide and its history live in memory: quitting the app ends
  the craft, and the next start begins with nothing grabbed.
- **The crafting engine's limits** show in the craft window:
  - **Odds are often unknown.** Every jewel (poe2db shows no spawn weights
    for jewels), every desecration and reveal (no established weights; a
    reveal may also offer normal-pool modifiers, whose share is not
    established), every flux (how it maps tiers is not established), and a
    target whose weight is the site's placeholder 1 give `unknown` with the
    reason. Known odds are one step's rough chance, never a simulation or
    an expected cost.
  - **The affix limits are assumed**: Magic 1 prefix and 1 suffix, Rare 3
    and 3, Rare jewel 2 and 2, one more on the other side with a Contempt
    liquid emotion, and one more or one fewer on a side for each base
    implicit that says so: Dusk Ring and Dusk Amulet one more prefix and
    one fewer suffix, Gloam Ring and Gloam Amulet the reverse, Distorted
    Amulet one fewer suffix, Twisted Amulet one fewer prefix (poe2db's base
    rows in the shipped data, fetched 2026-10-28;
    [`crafting-mechanics.md`](crafting-mechanics.md) § "Item basics"). The
    Magic and Rare limits are documented (poe2db's item text, the spec's
    `magic-slots` and `rare-slots`), the jewel and Contempt ones are the
    owner's; none is measured from the game, and every state-ordered
    route's slot accounting and the target choices' greying rest on them.
    How the game writes a base's limit implicit in a `Ctrl+Alt+C` copy is
    not measured either (no copied item has one): the app reads it in the
    form of the data's line, `+1 Prefix Modifier allowed` or `-1 Suffix
    Modifier allowed`, and `ch-data`'s `tests/limits.rs` builds its items
    that way.
  - **The last-target desecration rests on the data's level and pool.**
    Whether the class has a desecrated modifier on the target's side at
    the item's level is read from the shipped data (every equipment
    desecrated modifier is level 65 there), not measured; nor is what
    share of a reveal's options a normal-pool target gets, which is why
    the rule offers any target on a side with a pool and its odds are
    unknown. The Necromancy and Abyssal Echoes omens are assumed active
    before use (`omens-active-before-use`, now all four state-ordered
    routes').
  - **The tier join is measured on five items.** The tier the engine
    derives from poe2db's levels equalled the game's `Tier:` on all 18
    joined modifiers of the owner's five real items
    (`real_fixture_tiers_equal_game_tiers`); a class or modifier none of
    them carries has not been compared.
  - **A blueprint cannot branch into another**, and steps resting on a
    mechanic not established (alloys with Crystallisation omens, flux tier
    mapping, catalyst quality, rune sockets) are kept and said so in the
    blueprint file's `notes`, which the craft window does not show;
    catalyst and rune steps are manual.
  - **The state-ordered routes rely on facts that are not established.**
    Each of the four names them in its `relies_on`, by their id in the
    spec's facts table
    ([`crafting-slots-spec.md`](../models/crafting-slots-spec.md)
    § "Facts"); community guides state them or they are inferred, and
    none was measured in the game: Omen of Greater Exaltation with a
    Sinistral or Dextral Exaltation omen adding both modifiers on that
    side (`greater-with-side-omen`, all four), Greater Exaltation needing
    two free slots (`greater-needs-two-free`, all four), an exalt or
    augment with no side omen landing on the open side when the other is
    full (`open-side-landing`, `close-a-side` and `magic-augment-essence`),
    Whittling taking the lowest modifier level (`whittling-mod-level`,
    all four), a fractured modifier never being annulled or chaosed
    (`fractured-not-removed`, all four), the Necromancy and Abyssal
    Echoes omens needing to be active before use
    (`omens-active-before-use`, all four since 0.1.2's last-target
    desecration), and what Sinistral Necromancy does on a class with no
    desecrated prefixes (`necromancy-without-pool`,
    `essence-desecrate-double-slam`'s alone: the last-target desecration
    is never offered without a pool, so it does not rely on it). The craft window does
    not show `relies_on`: the blueprint files and
    [`blueprints.md`](blueprints.md) § "The seed blueprints" carry it. If
    one of them is false in the game, the step is still shown as the
    route has it, and what the currency did is classified from the item
    as usual, as a miss or as unexpected.
  - **A state-ordered preview assumes every step hits** and nothing else
    changes: no modifier lands where the step did not aim, no omen is
    wasted, and every fact above holds. A whittle's added modifier is not
    shown; a filler that closes a side is shown as "any modifier"; a later
    step's odds are read off that projected item ("if the earlier steps
    hit"), so they are rougher than the first step's. A preview longer
    than 16 steps is rejected as one that cannot finish. The odds of an
    uncertain removal assume each modifier it may take is equally likely
    to go (`removal-equally-likely`, not established).
- **The "looks like an item" check is the first line only.** A text is
  taken for an item when its first line starts with `Item Class: `; every
  item text the owner copied for the parser (`ch-item`'s
  `fixtures/real/`) does, but the check reads nothing else. A text that
  fails it is still shown, with the warning.
- **The hotkey reached the app with the game in front, in Live 1.** A
  `Ctrl+Alt+X` grab arrived with Path of Exile 2 in front (2026-10-29),
  as `Ctrl+Alt+W` did for the waystone roller (2026-10-28;
  [`docs/RUNTIME_DATA_MODELS.md`](../RUNTIME_DATA_MODELS.md) § "Item search
  box (stash and vendor)"). Whether the game binds `Ctrl+Alt+X` itself, or
  still sees it while the app holds it, was not checked. If a game binding
  stops working while the app runs, choose another hotkey in Settings.
- **A grab from the hotkey needs the copy first.** The hotkey reads what is
  on the clipboard now: press it after the game's `Ctrl+Alt+C` has copied
  the item. Pressed before, it shows whatever text was there, with the
  warning when that is not an item.
- **The refusal tone** is the waystone roller's: an 880 Hz `Beep`, with
  Windows' simple beep as the fallback. A muted "System Sounds" level or
  another output device could silence it; that was not checked.
- **English client only.**
- **The update check reads the 30 most recent releases** of the
  repository, all three apps' together; a Crafting Helper release older
  than the 30 newest is not seen. A network that needs a proxy Windows is
  not set up for, or that blocks `api.github.com`, `github.com` or GitHub's
  asset host, makes a check or an install fail with a one-line reason.
  GitHub answers at most 60 unauthenticated requests an hour from one IP
  address (GitHub's documentation, not measured here). Windows only.
- **The install needs a folder it can write.** It downloads into
  `crafting-helper.exe.download` beside the exe and renames the running exe
  to `crafting-helper.exe.old`; in a folder the app cannot write (under
  `Program Files`, for example) the install fails at once with "the app's
  folder cannot be written", before any download.
- **The `.old` file stays until the next start.** Windows does not let a
  running exe be deleted, so the replaced exe is left as
  `crafting-helper.exe.old` and removed by the next start, which retries
  for about 30 s while the old process ends. Until then, to roll back by
  hand: quit the app, rename `crafting-helper.exe` away and
  `crafting-helper.exe.old` back to `crafting-helper.exe`.
- **Antivirus and SmartScreen behaviour is not established.** The exe has
  no Authenticode signature, and how an antivirus treats an exe that
  downloads and renames its own file was not measured.
- **The built exe has run with the game once.** 0.1.0 is proven by tests
  with fakes (a fake shell for the clipboard, the hotkey, the browser and
  the overlay, a fake screen backend, a fake transport and loopback
  servers for updates), by the screen reader's run over the owner's
  screenshots, and by Live 1 (2026-10-29): one guided craft of a rare
  belt, its first exalt read from the screen as a miss, with the overlay
  shown at the top center over the game and hidden whenever another
  window came in front. The craft window's walks to the end of a route
  are the engine's test items (constructed ring texts), not crafts taken
  in the game. The changes made after Live 1 (whittle or annul decided by
  the app, a removal step naming its modifiers, the branch lines naming
  the next step, no route name on screen, the outcome's word said once)
  are proven by tests only, and so are 0.1.1's state-ordered routes: the
  engine's walks, the slot model and the parity test over constructed
  items, with no craft taken in the game. 0.1.2's last-target desecration
  and greyed target choices are proven the same way; the page's tests
  prove a greyed choice's disabled state and tooltip, not how it looks,
  which is for a look in the browser preview (`craft-side-full`).
- **Craft of Exile's page is outside this project.** The button opens its
  beta address with `?game=poe2`; if the site moves or drops that address,
  the button opens whatever the site serves there until the app is
  updated.

## Maintenance triggers

- **The game's copied item text changing** (its first line, or the
  `Ctrl+Alt+C` binding): the "looks like an item" check in `ch-app`'s
  `grab.rs` and the parser in `ch-item`.
- **The item tooltip's look changing** (a patch that redraws its header,
  frame lines or name colours, moves it, changes its fonts or the key
  hints under it, or a new UI scale): take a new screenshot set with each
  item's copy, as the research note's
  [Item tooltip on screen](crafting-helper-research.md#item-tooltip-on-screen)
  describes, measure the boxes into `ch-watch`'s `tests/shots.txt`, and
  run the real-shot tests with `CH_TOOLTIP_SHOTS` (Working on the app)
  until each plain shot reads as its copy or unreadable again; retune the
  locator, the reader or the matcher's `INTERFACE_STRINGS` in `ch-watch`
  to get there, and fold what changed into the research note and
  [`docs/RUNTIME_DATA_MODELS.md`](../RUNTIME_DATA_MODELS.md) § "Item
  tooltip (on screen)".
- **A change to the quest tracker's `qt-screen` or `qt-capture`**: both are
  built into this app by path. Run `ch-watch`'s tests and its real-shot run,
  `ch-app`'s `watch` and `overlay` tests, `qt-capture`'s tests with its OCR
  self-test, and the guard (`tests.test_crafting_helper_guard`, which walks
  both crates), before the change lands for either app.
- **The game's window title changing** from `Path of Exile 2`:
  `GAME_WINDOW_TITLE` in `ch-app`'s `screen.rs` (and the waystone roller's
  own copy).
- **A game patch** that adds, removes or rewords modifiers or crafting
  items: refresh the data with the extractor and update
  [`crafting-mechanics.md`](crafting-mechanics.md). The screen watch
  matches tooltip lines against the same data, so run `ch-watch`'s tests
  (and the real-shot run where the shots are) after a refresh.
- **A data refresh**: an essence, alloy or liquid emotion renamed or
  removed breaks every blueprint that names its id, and a new crafting item
  needs a catalogue entry in `ch-blueprint`'s `catalogue.rs`. Run
  `check_blueprints` on `apps/crafting-helper/blueprints/` and the
  `ch-data` and `ch-engine` tests ([`blueprints.md`](blueprints.md)); the
  odds anchors in `ch-data`'s tests change with the weights. A catalogue
  id that goes away is also dropped from saved blocked lists at the next
  start, and `fixtures/catalogue.json` and the craft window's fixtures
  change with it (`CH_FIXTURES=1`, Working on the app).
- **A patch that changes slots or omens** (how many prefixes and suffixes
  a magic or rare item holds, what an Exaltation, Annulment or Necromancy
  omen does, Omen of Greater Exaltation or Whittling, or a crafting item
  made available or unobtainable): update the facts table and the rules in
  [`docs/models/crafting-slots-spec.md`](../models/crafting-slots-spec.md)
  first, then the model (`tools/crafting_helper/slot_model.py`) and the
  cases it records in `cases.json` (`py -3 -m unittest
  tests.test_crafting_helper_slot_model`), then the engine until the
  parity test (`ch-engine`'s `tests/slots.rs`) passes. A fact relabelled
  in the table also means `ch-blueprint`'s list of not-established ids
  (`facts_match_the_spec_table` fails until they agree) and the
  `relies_on` of each blueprint that names it; a fact a session measures
  is relabelled measured there.
- **poe2db's pages changing**: the extractor stops with an error naming the
  page; see the research note.
- **Craft of Exile's address changing** (the site leaving
  `beta.craftofexile.com`, or `?game=poe2` no longer choosing Path of
  Exile 2): change `CRAFT_OF_EXILE_URL` in `ch-app`'s `craft.rs` and the
  guard's `CRAFT_OF_EXILE_URL` in `tests/test_crafting_helper_guard.py`
  together, with this guide's Privacy section; it stays the one address.
- **A dependency bump**: keep the versions equal to the quest tracker's
  (`LockedToQuestTrackerPins`); bump them together.
- **GitHub changes its API or its release flow**: the release list's
  endpoint or fields, the `/releases/download/` redirect, or its asset host;
  or the repository, the `crafting-helper-v` tag prefix or the asset names
  change. Update the shared `ep-net` and `ep-update` with
  `tests/test_app_updates_guard.py` and `tools/crafting_helper/release.py`
  together ([`docs/app-updates.md`](../app-updates.md)).
- **The signing key changes** (rotated or lost): follow
  [`docs/app-updates.md`](../app-updates.md) § "Rotating the key"; copies
  built with the old key cannot install a release signed with the new one.

## Release notes

### 0.1.2

- The last target is desecrated (#32). When a rare has every target but
  one and no desecrated modifier yet, the route now places that last one
  with a bone on its side, with Omen of Sinistral or Dextral Necromancy,
  then reveals it with Omen of Abyssal Echoes, instead of an Exalted Orb
  or essence slam. If the reveal offers none of your targets, the route
  takes the wrong modifier off with Omen of Light and an Orb of
  Annulment and uses the bone again. Desecration and reveals still say
  "odds unknown".
- When the last target cannot be desecrated, the route keeps its usual
  step and its Fallback line says why: the item's class has no
  desecrated modifier on that side at the item's level (body armours,
  boots, gloves, helmets, shields and bucklers have no desecrated prefix,
  and every equipment desecrated modifier needs item level 65), or the
  bone or the Necromancy omen is blocked.
- Target choices now respect the item's prefix and suffix limits. A
  choice that would go past a side's limit is greyed out, not hidden,
  and its tooltip says why (`Prefixes full: 2 on the item + 1 chosen = 3
  of 3`); untick a target and the side's choices come back.
- The limits are read from the item, not a fixed three: Dusk and Gloam
  rings and amulets, and Distorted and Twisted amulets, allow one more or
  one fewer prefix or suffix, and a Contempt emotion still adds one.
  Fractured and unrevealed modifiers you have not chosen still hold
  their slot.
- The app still sends the game nothing.

### 0.1.1

- Crafts are planned from each side's free slots. Four new routes,
  drawn from published 0.5 crafting guides, pick their next step from the
  item as it stands, so an item already part-way there (a rare with some
  of your targets, a fractured target, a magic item with one good
  modifier) starts where it is, and the route preview shows the steps the
  craft takes if each one lands:
  - **close a side**: fill the side that already has its targets with
    cheap modifiers, so the last exalt can only go where the missing
    target is;
  - **essence, desecrate, then side slams**: an essence on a magic item,
    a desecration for the hardest target on its side, then Exalted Orbs
    restricted to each target's side;
  - **a magic item with two good modifiers, then an essence**: transmute
    and augment, annul a wrong one, then an essence makes it rare with a
    third;
  - **clean a rare, then slam**: for a rare with few or none of your
    targets, such as a dropped one (#28): remove what is in the way, then
    side-restricted slams.
- The routes never offer an Exalted Orb without a side omen while one
  side's last free slots are kept for its missing targets, so an exalt can
  no longer take the prefix slot your last prefix needs.
- Every omen a step needs is named: the Sinistral or Dextral omen for its
  side, and Omen of Greater Exaltation where two free slots take two
  targets.
- A removal says Omen of Sinistral or Dextral Annulment when the other
  side holds a target, so the annul cannot take it, and uses a whittle
  only when it is certain to take a modifier you do not want.
- The old "fill, desecrate, slam" route is gone; the new ones cover its
  crafts and the rare with one target it refused.
- Some of what these routes do is how community guides describe the
  game, not something GGG has stated or this project has measured (such
  as Greater Exaltation with a side omen putting both modifiers on that
  side). Each route's file lists those facts (Known limitations).
- The app still sends the game nothing.

### 0.1.0 (first version)

- The crafting helper's first version: copy an item in the game with
  `Ctrl+Alt+C`, then press `Ctrl+Alt+X` (changeable, never `Ctrl+Alt+C`)
  or `Ctrl+V` in the window, and the window becomes the craft window.
- The craft window shows the item and its modifiers with their tiers. The
  item's own prefixes and suffixes start ticked as targets at the tier
  they have, so you only add the modifier you want next; pick it from the
  prefixes, suffixes and desecrated modifiers the item's base can roll,
  each "as high as possible" or "at least" a tier, and it previews the
  route a blueprint takes: the targets, then each step naming the
  modifiers it aims for, or the ones it removes, with its currency and
  omens, its rough odds or "odds unknown" and why, the fallbacks taken
  because something is blocked, or why no blueprint applies. Where a
  clean-up step could whittle (Chaos Orb with Omen of Whittling) or annul
  (Orb of Annulment), the app picks from the modifier levels on the item
  and judges the result against the one it named.
- *Start crafting* guides you step by step: take the step in the game and
  hover the item, and the app reads the item's tooltip from the screen with
  Windows' own OCR, says whether it was a hit or a miss, moves to the next
  step and keeps a history. When it cannot read the item, or the change is
  not one the guide expected, it changes nothing and says to grab the item:
  a grab (`Ctrl+Alt+C`, then the hotkey or `Ctrl+V`) always works and says
  whether it was a hit, a miss or unexpected. A craft is kept only until
  the app quits.
- An overlay over the game shows the current step, where a hit and a miss
  go, the last outcome and what the screen reading is doing, while a craft
  is guided and the game is in front. It lets every click through and
  never takes focus.
- An *Explore more crafting on Craft of Exile* button opens Craft of Exile
  in your web browser, the only page the app ever opens.
- The app reads the clipboard once per grab and only then. It never watches
  the clipboard, uses no keyboard hook and sends the game nothing. It copies
  the game's window only while a craft is guided with the game in front;
  the copies and what OCR reads stay in memory and are never saved, logged
  or sent.
- A Settings window with the hotkey, the overlay's position (top center by
  default, or another edge or corner) and *Read the item from the screen*
  (on by default; off, you grab the item after each step), a saved list of
  blocked currencies picked by name from every crafting item (each craft
  can change it for itself), *Automatically check for updates* (off by
  default; it only checks), *Check now*, the in-app install of a signed
  newer version, the version and "Created by ST4H".
- Reading the screen needs Windows' English OCR and the game windowed or
  borderless in English. It reads the hovered equipment's tooltip, belts
  included, with their charm slot and flask lines, and was measured on
  the owner's screenshots and in one live session at 2560 x 1440 (Known
  limitations).
- Download `crafting-helper.exe` from the repository's
  [crafting helper releases](https://github.com/S-Borkowski/ExilePact2/releases?q=crafting-helper)
  and run it; there is no installer.
