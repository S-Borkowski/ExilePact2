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
browser for the crafting this app does not cover, and the *Find a base on
trade* panel opens a trade site search, in the same browser, for a magic
base that already carries the modifiers you want, so a craft can start
from a bought base.

- UI direction, windows and states: [`ui-design.md`](ui-design.md)
- The design reviews: the craft window's direction, before it was built
  ([`design/review-direction.md`](design/review-direction.md)), the built
  pages ([`design/review-craft-window.md`](design/review-craft-window.md)),
  and the overlay's direction, before it was built
  ([`design/review-overlay.md`](design/review-overlay.md))
- What each crafting item does to an item's modifiers:
  [`crafting-mechanics.md`](crafting-mechanics.md)
- Research note (poe2db's modifier pages, the data extractor, the data
  files, the trade site's stat ids, the item tooltip on screen):
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

- **One window, `Crafting Helper.exe`** (since 0.2.0; `crafting-helper.exe`
  before, and a 0.1.x copy that updates itself keeps that old name: the
  update bullet below). A Tauri app: the app's logic
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
    and, when more than one route applies, a select listing every one as
    `Route 1`, `Route 2` and so on in the engine's order, the previewed one
    selected. A route keeps its number whichever is previewed, so choosing
    `Route 2` shows route 2 and leaves it selected (since 0.2.0; before,
    the previewed route was always listed first as `Route 1`, so a second
    choice could flip back). The blueprint's name is not shown: the technique is the
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
    why it does not apply, and above those lines the app may propose
    lowering one target's tier (below). *Start crafting* starts the guide.

    Seven of the seventeen blueprints are **state-ordered** (four since
    0.1.1, two more since 0.1.4, one since 0.2.0): the
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
    Sinistral or Dextral Necromancy (`Use Omen of Sinistral Necromancy,
    then apply the Preserved Collarbone; the desecrated modifier will be a
    prefix, for +# to Armour`), then the
    reveal with Omen of Abyssal Echoes. A reveal that offers no target is
    taken off with Omen of Light and an Orb of Annulment, and the bone is
    used again (since 0.2.0, only once no Exalted Orb step can still add a
    target on that side: below). Both steps say "odds unknown" with the reason. When the
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

    **What the crafting guides teach** (since 0.1.4, issue #35). The
    maxroll and Mobalytics crafting guides add four routes: a physical
    weapon built on a good magic prefix and Greater Essence of Abrasion
    (Bows, Crossbows, One Hand Maces, Quarterstaves, Spears, Talismans,
    Two Hand Maces), a body armour with one top defence prefix whose other
    prefixes are added together once only that side is open, a rare with
    four modifiers whose target is fractured first, after Essence of the
    Abyss, and the rescue of a full rare that lacks one target, by a
    desecration on that target's side. Every route keeps to 0.5's one
    crafted modifier per item (an essence's, an alloy's or a liquid
    emotion's), so `weapons-alloy-budget`, which used two alloys, no
    longer ships. And every route follows the guides' good practice:
    - a step whose odds its exclusions settle says so, 100% or 0%, even
      where the weights are unknown (a jewel's isolation);
    - on a state-ordered route, a removal certain to take only modifiers
      you do not want comes before a step that gambles;
    - a magic item whose target prefix is below the two best tiers it can
      roll at its item level is not upgraded to rare: the route is
      refused, and the reason says a new base is worth more;
    - a jewel is never offered a Greater or Perfect currency, which only
      costs more there: the plain orb is offered instead.
    How they work: [`blueprints.md`](blueprints.md) § "The seed
    blueprints".

    **Desecrations name their Necromancy omen** (since 0.2.0, issue #37).
    Every desecration that has to land on one side tells you to use Omen
    of Sinistral Necromancy (prefix) or Dextral Necromancy (suffix) first,
    and says which side the desecrated modifier will be (`Use Omen of
    Dextral Necromancy, then apply the Preserved Collarbone; the
    desecrated modifier will be a suffix, for +#% to Cold Resistance`).
    `rings-resist-whittle`'s desecration, which used the plain bone, now
    takes the Necromancy omen of the last target's side: while targets on
    both sides are still missing the preview names both omens (Sinistral
    if the last target is a prefix, Dextral if it is a suffix), and the
    guide names the one once a single target is left. When that omen is
    blocked it falls back to the plain bone, with a Fallback line naming
    the omen. A desecration tied to no side (a blocker on either side)
    still names none.

    **A list with no route may be offered one lowered tier** (since 0.2.0,
    issue #37). When no route reaches the targets, but lowering one
    target's floor to the tier an essence, alloy or liquid emotion
    guarantees lets a route through that currency exist, the Route panel
    says so under "No route reaches these targets" (`Lower +#% to Fire
    Resistance from T2 to T3: Greater Essence of Insulation adds it at T3,
    which unlocks a route.`, for a magic Heavy Belt with T2 Fire, Cold and
    Lightning Resistance), with two buttons: **Lower to T3** changes that
    one floor and shows the route, **Keep T2** hides the proposal and
    leaves the targets as they are (it shows again only when the app
    proposes something else). Only one target is ever lowered, a floor is
    never raised, and nothing is proposed while a route exists.

    **A wrong reveal is kept through the same-side exalt** (since 0.2.0,
    issue #37). On a state-ordered route, when a reveal offers none of your
    targets and the route still has an Exalted Orb step that can add a
    missing target on the same side, with a free slot there, the revealed
    modifier stays on the item through that exalt: it keeps its modifier
    family out of the exalt's pool, so it never lowers the exalt's chance
    and can raise it. Omen of Light takes it off once that side is done,
    as before. A side already full takes Omen of Light first, as in 0.1.x.
    And the reveal step that such an exalt follows names a **blocker**,
    in the route preview and on the guide's step (the overlay does not
    show it), under its odds: `Blocker: If +#% to Cold Resistance is not offered,
    take +# to Strength and Dexterity or +# to Strength and Intelligence
    (keeps out +# to Strength): the suffix exalt's chance of a target goes
    from ~17% to ~20%`. It names at most three desecrated modifiers, best
    first, those of equal chance together, and only those that raise the
    exalt's chance; with none, there is no Blocker line. Only desecrated
    modifiers are candidates, and the app does not read the Well of Souls
    screen, so it cannot tell which of them were offered (Known
    limitations). Listed-order routes give no blocker advice.

    **A fractured modifier narrows an essence's removal** (since 0.2.0,
    issue #37). On a rare whose prefix or suffix side is full and holds
    a fractured modifier, the state-ordered fractured essence swap adds
    an essence's target there (Essence of Hysteria's, the one essence
    whose modifiers are in the class lists): the essence removes one of
    that side's other modifiers first, never the fractured one, so with
    no target among them its modifier is certain. Its step names what the
    removal can take and what stays (`Essence of Hysteria for #% increased
    Mana Regeneration Rate (suffix); it removes the unrevealed desecrated
    suffix or +10(9-12) to Dexterity, one at random; +23(21-25)% to Fire
    Resistance is fractured and stays`). When one of those is an
    unrevealed desecrated modifier, a **Survival** line under the odds,
    in the route preview and on the guide's step (the overlay does not
    show it), gives its chance of surviving the swap and the next step on
    each outcome: `The unrevealed desecrated suffix survives the swap 1
    in 2 (…). If it stays, Reveal at the Well of Souls with Omen of
    Abyssal Echoes for … If it goes, Orb of Annulment with Omen of
    Dextral Annulment to remove +10(9-12) to Dexterity; it takes one
    modifier at random, so it spares #% increased Mana Regeneration Rate
    1 in 2; if it takes it, the route re-plans when the item is read
    again.` After the annulment the route desecrates the last target on
    that side with its Necromancy omen and reveals it (Known
    limitations).

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
  It is one of the two addresses the app opens; the trade search below is
  the other (Privacy).
- **Find a base on trade** (since 0.1.3), a button in the main window's
  header, shows a panel in place of the craft area for starting a craft
  from a bought base. Pick an **item class**, a **base type**, an optional
  **minimum item level** (1 to 100; a higher level reaches better tiers)
  and up to 8 **modifiers** from the base's ordinary-pool prefixes and
  suffixes, each "Any tier" or "At least T<n>". Once a modifier has a
  floor, the blank item-level field suggests the level the most demanding
  floor tier needs (since 0.1.4; at least T2 movement speed on boots
  needs 65), the help line says the search uses it, and the search asks
  for that level and up; a level you type replaces it. Each modifier shows its
  **weight**, higher the rarer it is to roll on that base (12.0: about 1
  in 12 modifiers added to that side of the base is this one, at the floor
  or better), from the modifier data's spawn weights; where the data has
  none for that base (every jewel, some tiers), the weight is a stated
  default, tagged "default" with its reason. **Match** chooses *Both
  slots* (the default: a prefix and a suffix from your picks, two in all,
  since a magic item holds one of each) or *At least one*; a line says
  what the search will ask for, and why when it is one. **Search on the
  trade site** opens `www.pathofexile.com/trade2` in your default browser,
  searching for a **magic** item of that base, online and not corrupted,
  carrying those modifiers, and the search carries a weighted sum of them
  (each modifier's value times its weight over its top value, the best
  roll it can have at that item level, so a modifier counts for at most
  its weight and a rarer modifier counts for more). The site opens the
  results sorted by price: click the Weighted Sum column to rank them by
  that sum. The link is also shown,
  read-only, to select and copy yourself. A modifier with more than one line (a hybrid), or one
  the trade site has no stat for, is listed under *Not searchable on
  trade* with the reason. The panel never buys, whispers, copies to the
  clipboard or touches the game, and the craft goes on underneath: *Back
  to the craft* returns to it as it stands. The selection is kept in
  memory for this run, never saved. How it looks:
  [`ui-design.md`](ui-design.md) § "Find a base on trade".
- **The trade league**: the search opens in the league chosen in
  Settings, *Trade* (`Forbidden Rites` by default). The **League**
  dropdown offers the six leagues this version shipped with, and the
  **refresh icon** beside it asks the trade site for its current leagues
  (one request, only on that click; Privacy). The refreshed list lasts
  until the app closes and is never saved; the league you chose is saved
  and always offered, even when a list lacks it.
- **A Settings window**, opened from the *Settings* button at the right of
  the main window's header, holds five sections, *Grab* (the hotkey),
  *Overlay* (where the overlay sits, and whether the app reads the item
  from the screen), *Blocked currencies*, *Trade* (the league) and
  *Updates*, and the line "Created by ST4H". While a
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
  downloads the release's exe (`Crafting.Helper.exe`, the name GitHub
  stores for `Crafting Helper.exe`), checks its signature
  against the key built into the app, puts it in place of the running exe
  and restarts into it, with a progress bar and *Cancel*. The file keeps
  the name it has: a 0.1.x copy, `crafting-helper.exe`, fetches the
  release's legacy `crafting-helper.exe` asset (every release from 0.2.0
  carries it, signed under that name) and stays `crafting-helper.exe`
  after the update; rename it by hand if you like, or download
  `Crafting Helper.exe` afresh. A failed step says
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
  Windows 10 machines have it through Edge. `Crafting Helper.exe` does not
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

### `Crafting Helper.exe`

Takes no arguments. A 0.1.x copy updated in place keeps its old name,
`crafting-helper.exe`, and behaves the same.

| Environment variable | Effect |
|---|---|
| `CRAFTING_HELPER_DIR` | The folder for `settings.json` and `diag.log`. Unset or empty: `%LOCALAPPDATA%\ExilePact2\crafting-helper\`. A folder inside a git checkout is refused: the app then runs without saving, and the status line says why. |

- `settings.json`: `schema` (1), `hotkey` (default `Ctrl+Alt+X`),
  `check_updates` (*Automatically check for updates*; any value but `true`,
  or none, loads it off), `blocked_currencies` (a list of catalogue
  ids), `overlay_position` (where the overlay sits: `top-left`,
  `top-center`, `top-right`, `bottom-left`, `bottom-center` or
  `bottom-right`; any other value, or none, loads the default,
  `top-center`), `screen_watch` (*Read the item from the screen*; any
  value but `false`, or none, loads it on) and `trade_league` (the league
  the trade search opens in, as the trade site writes it: 1 to 64
  characters once trimmed, with no control character; anything else, or
  none, loads the default, `Forbidden Rites`). A craft's own changes to the
  blocked list, the trade panel's selection and a refreshed league list
  are never saved. Written a
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
  - The trade panel's and the league list's `trade:` lines, counts and
    reasons only, never a league, a base, a modifier or the link:
    - `trade: opened, <n> modifiers, at least <k>` (`1 modifier`) after
      *Search on the trade site* opened the search, `trade: open failed:
      <error>` when the browser could not be asked to open it (the link in
      the error replaced by `the trade search`), and `trade: refused:
      <reason>` for a selection the app did not take or a search it could
      not open (no class, base type or pick, a link over 2,000
      characters);
    - `trade: leagues refreshed, <n> leagues, <s> skipped` (`1 league`)
      when a refresh's answer was taken, and `trade: league refresh
      refused: <reason>` when it was not (the request failed, the site
      answered another status than 200, the answer was over 64 KiB, not a
      league list, over 50 entries, or left no league).
  - `blocked currencies: N ids` when the list changes; `settings refused:
    <reason>` when the Settings window sends a change the app does not
    take; `error: settings window: <reason>` when that window could not be
    opened; `error: settings could not be saved: <e>`.
  - `updates: ...` lines, the same as the waystone roller's (its guide's
    Command reference): the setting turned on or off, each check's start
    and outcome, each install step, failure, cancel or refusal, the restart,
    and at every start one `updates: cleanup: ...` line
    (`removed Crafting Helper.exe.old` when an install left one, the
    running exe's name with `.old`). No line
    holds a URL or anything of GitHub's answer.
  - `stop` when the app quits.

  Rotated at 1 MB into `diag.log.1`.

**The main window**, titled "Crafting Helper", 960 by 680 at 1x by default
and usable from 600 by 480. Its header holds the title, the *Explore more
crafting on Craft of Exile* button, the *Find a base on trade* button and
the *Settings* button; under it, the craft area (or the trade panel in its
place), in two columns from 880 px wide and one below; last, the status
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
- **Trade** (since 0.1.3): *League*, a dropdown of the leagues offered,
  the saved one selected, with the refresh icon beside it ("Refresh the
  league list from the trade site"), and the help line "The league the
  trade search opens in. The refresh button asks the trade site
  (pathofexile.com) for its current leagues; nothing is sent until you
  click it, and the new list lasts until the app closes." Before a
  refresh the dropdown offers the six leagues this version shipped with
  (Forbidden Rites, HC Forbidden Rites, Runes of Aldur, HC Runes of Aldur,
  Standard, Hardcore); after one, the trade site's Path of Exile 2
  leagues in its order, leaving out another realm's, a private league
  and a name the app would not take. The saved league is always offered,
  last when the list lacks it. While a refresh runs the icon is disabled
  and the note reads "Refreshing the league list…"; then `Refreshed: <n>
  leagues from the trade site.` (with `; <s> skipped`), or `! Not
  refreshed: <reason>. The list and your league are unchanged.` A second
  click while one runs sends nothing.
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
address is the app's own; and the trade panel's `set_trade_search` (the
panel's whole selection: whether it is open, the class, the base type,
the minimum item level, the picks, each a modifier id and a floor or
none, and *Both slots*; no field names an address) and
`open_trade_search`, which takes no argument: the app builds the link from
its own state. Each craft and trade command answers with the view. Both
windows may send `get_catalogue`, which answers the catalogue grouped by
kind, each entry's id and English name. The main window is sent `ch:view`,
which carries the craft part (its mode and what the mode shows), the
hotkey, the last grab, the status line, the overlay's state (whether it
shows, its position and, while guiding, the step it shows) and
`update_available`; while guiding, the craft part carries the screen
watch's line. The Settings window's commands are `get_app_settings`,
`set_hotkey`,
`update_settings` (an object of `check_updates`, `blocked_currencies`,
`overlay_position`, `screen_watch` and `trade_league`, any of them, and
nothing else; a position the app does not know, or a league name it
would not load, is refused whole), `refresh_trade_leagues` (no argument;
the only way the league request is sent), `check_for_updates`, `install_update`
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
| `npm run release` | `tauri build --no-bundle`: `target/release/Crafting Helper.exe` (Cargo builds `crafting-helper.exe`; Tauri names the result from `mainBinaryName`). |

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
  starts when you press the Craft of Exile or trade search button,
  below), no game memory
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
  update check's requests to GitHub, the trade site's league list when you
  click the refresh icon, opening Craft of Exile or the trade search in
  the default browser when you press its button and, when you press the
  install button, replacing its own exe.
- **Two web addresses, each opened only by its button.** *Explore more
  crafting on Craft of Exile* asks Windows to open
  `https://beta.craftofexile.com/?game=poe2` in your default browser, and
  *Search on the trade site* (the trade panel) asks it to open
  `https://www.pathofexile.com/trade2/search/poe2/` followed by the
  league and `?q=` and the search, both percent-encoded, so the league
  cannot leave its place in the address; nothing else. Both prefixes are
  fixed in the app (`ch-app`'s `CRAFT_OF_EXILE_URL` and
  `TRADE_SEARCH_URL`), no command takes an address (the trade search's
  command, `open_trade_search`, takes no argument: its link is built from
  the app's own state), and the pages hold none and
  cannot open one themselves (no link, no `window.open`; the opener
  plugin's own commands are granted to no window). The app itself sends
  nothing for either; the page loads in your browser, which connects to
  the site as for any page you open (neither site is part of this
  project). Nothing of your item, targets or craft goes with either. **The
  trade search's link carries** the base type, the minimum item level if
  you set one, the league and the trade site's ids of the modifiers you
  picked, with a minimum value where you chose a tier floor and each
  one's weight; the site sees them, with
  your browser's own cookies and session there, as for any search you
  make. The app sends the trade site no search itself (it never asks the
  site for a search id: the link is built on this machine) and writes
  nothing to the clipboard: the link is shown for you to select and copy.
  The guard pins each of these (`tests.test_crafting_helper_guard`).
- **Network: only the update check, the install and the league list, only
  when you ask.** With *Automatically check for updates* off (the
  default), no *Check now* pressed and the league refresh icon not
  clicked, the app sends and receives nothing (the Craft of Exile page
  and the trade search are your browser's connections, not the app's). A
  check is one `GET
  https://api.github.com/repos/S-Borkowski/ExilePact2/releases?per_page=30`,
  port 443 only, from Rust through Windows' own WinHTTP, with three
  headers: `Accept: application/vnd.github+json`, `X-GitHub-Api-Version:
  2022-11-28` and `User-Agent: ExilePact2-crafting-helper` (no version, no
  machine detail). The answer is read up to 2 MiB within 20 s, used only to
  find the newest release, and never stored or logged.
- **An install** (the button only) makes two downloads, the signature, then
  the exe: `GET
  https://github.com/S-Borkowski/ExilePact2/releases/download/crafting-helper-v<version>/Crafting.Helper.exe.minisig`
  and the same path ending `Crafting.Helper.exe` (a 0.1.x copy asks for
  `crafting-helper.exe.minisig` and `crafting-helper.exe`, which every
  release from 0.2.0 also carries), each with two headers,
  `Accept: application/octet-stream` and the same `User-Agent`. GitHub
  answers each with a redirect to a signed address on
  `release-assets.githubusercontent.com`; the app follows that one redirect,
  only over HTTPS on port 443 and only to that host or
  `objects.githubusercontent.com`, with the same two headers, and never logs
  or keeps the address. Each download stops at the size the release lists
  (at most 64 MiB for the exe, 4 KiB for the signature), after 30 s without
  a byte, or after 15 minutes in all.
- **The league list** (since 0.1.3), only when you click the refresh icon
  beside *League* in Settings, *Trade* (the `refresh_trade_leagues`
  command, which takes no argument): one `GET
  https://www.pathofexile.com/api/trade2/data/leagues`, port 443 only,
  from Rust through the same shared `ep-net` and WinHTTP, with two
  headers, `Accept: application/json` and the same `User-Agent`, and
  nothing of your settings, item or searches. Never at start, on a timer,
  on opening Settings or the trade panel, after an update check or on a
  settings change; a click while one runs sends nothing. The answer is
  read up to 64 KiB within 20 s (the release list's timeouts), only its
  Path of Exile 2, non-private league names are kept, in memory for this
  run, and the answer is never saved or logged: the next start offers the
  shipped list again and sends nothing.
- For every request: no cookie and no credential is sent, the proxy is the
  system's, and TLS and its certificate checks are Windows'. GitHub, or
  for the league list the trade site, sees your IP address and those
  headers; nothing else leaves. The app's code
  outside the shared `ep-net` crate has no network code
  (`tests.test_crafting_helper_guard`). (WebView2, Microsoft's runtime that
  draws the windows, may connect on its own, as for the quest tracker.) The
  data extractor asks poe2db.tw for pages only when a developer runs it.
- **What it writes**: `settings.json` and `diag.log` in its folder (Command
  reference). The log holds the hotkey, the grabs' line counts, refusals,
  the blocked list's length, the `craft:` lines, the `overlay:` and
  `watch:` lines (states, counts and the game's client size), the Craft of
  Exile line, the `trade:` lines (counts and reasons) and the `updates:`
  lines; nothing about your account or character, no item text, no OCR
  text, no window title, no image, no league, base, modifier or trade
  link, and nothing of GitHub's or the trade site's answers.
  `settings.json` holds the league you chose. An
  install also writes beside the exe, under the running exe's own name:
  `Crafting Helper.exe.download` while
  it downloads (deleted when the install fails or is cancelled) and
  `Crafting Helper.exe.old`, the replaced exe, which the next start removes.

## Working on the app

| Path | What it holds |
|---|---|
| `apps/crafting-helper/crates/ch-app/` | The app's logic without Tauri: the store, the grab's checks and normalisation (`grab.rs`), the hotkey's rules (`hotkey.rs`), the saved settings, the blocked currencies and the catalogue grouped for the pickers (`settings.rs`), the craft window (`craft.rs`: its four modes in memory, what a grab does to it, its commands' rules, its view from the engine's plan, guide and outcomes, the copy the design marks Rust's, and `CRAFT_OF_EXILE_URL`), the views (`view.rs`, the IPC contract `src/ipc.ts` mirrors), the commands, the overlay over the game (`overlay.rs`: where it sits from the game's client area, the setting and its size, and the step it shows), the screen watch (`screen.rs`: the `ScreenBackend` trait the glue implements, `GAME_WINDOW_TITLE`, the watch thread with its timings, and the watch's lines; `craft.rs`'s `Craft::screen_change` applies a confirmed read), and the update commands and ticker (`updates.rs`, over the shared `ep-update`), the *Find a base on trade* panel (`trade.rs`: the selection, its rules and view, `TRADE_SEARCH_URL`, the percent-encoding and the link) and the trade league (`league.rs`: the `trade_league` setting's rules, the shipped list and `DEFAULT_TRADE_LEAGUE`, and the refresh answer read and filtered; the request itself is the shell's `fetch_trade_leagues`). Tested through a recording fake shell, a fake screen backend, a fake transport and a temporary store. |
| `apps/crafting-helper/crates/ch-watch/` | The screen watch's pure part, with no thread, clock or Windows call: the locator that finds item tooltips in a copy of the game's client (`locator.rs`), the reader that OCRs each tooltip's crop through the calls it is given and reports one under the overlay as covered (`reader.rs`), the matcher that tells no tooltip, covered, another item, unreadable (with the reason), unchanged or changed from the read lines and the item the step was shown on (`matcher.rs`, `text.rs`), and `item_key`, what "the same item" means for the guide. Takes the quest tracker's `qt-screen`, and `qt-capture` for its screenshot tests alone. |
| `apps/quest-tracker/crates/qt-screen/`, `qt-capture/` | The quest tracker's screen crates, taken **by path and unchanged**: `qt-screen` (the image, the boxes, the OCR line, the fuzzy match) by `ch-watch` and the glue, `qt-capture` (the game's client area, the GDI copy, the overlay's own rect and Windows' OCR) by the glue and by `ch-watch`'s tests. Each takes its package keys from its own workspace; a change there is a change to this app ([Maintenance triggers](#maintenance-triggers)). |
| `apps/crafting-helper/crates/ch-item/` | The item text parser, a std-only crate; the craft window parses each grab with it: [`crates/ch-item/README.md`](../../apps/crafting-helper/crates/ch-item/README.md). |
| `apps/crafting-helper/crates/ch-data/` | The modifier data under `data/`, embedded at build time: an item's variant from its base type, what it can roll at its item level, the join of each parsed modifier to its data modifier and tier, the affix limits (`AFFIX_LIMITS`), and one step's add and removal odds, or `unknown` with the reason; and the `trade` module, pure and offline: the trade stat ids (`trade-stats.json`, embedded), a modifier's start weight or the stated default, what a base offers the trade panel, and the trade site's search JSON ([`crates/ch-data/README.md`](../../apps/crafting-helper/crates/ch-data/README.md) § "Trade: start weights and the search query"). |
| `apps/crafting-helper/crates/ch-blueprint/` | The blueprint format: the model (`model.rs`, listed and state order), the static catalogue of currencies, omens, bones, fluxes and catalysts with their ids, and the not-established fact ids `relies_on` may name (`catalogue.rs`), the structural validator whose every error is `<path>:<line>:<col>: <message>` (`validate.rs`), and the shipped blueprints, embedded by `build.rs`. |
| `apps/crafting-helper/crates/ch-engine/` | The engine: blueprints resolved against the data, route choice for an item, its targets and the blocked currencies (`route.rs`), the slot accounting every state-ordered rule reads (`slots.rs`), the state-ordered next step, side rules, placeholders and preview (`state_order.rs`), the guide's outcomes (`guide.rs`), the full catalogue with the data-backed essences, alloys and liquids, and the `check_blueprints` example. `ch-app` re-exports it and plans, guides and fills the blocked list's catalogue with it. |
| `docs/models/crafting-slots-spec.md` | The mechanism behind state-ordered routes, in our own words, each fact labelled documented, measured or not established with its source: the slot accounting, what each crafting item does to the slots, the route rules, the preview and the representative crafts. The model and the engine follow it; a gap is fixed there first. |
| `apps/crafting-helper/blueprints/` | The shipped blueprints, one TOML file each: how to write one is [`blueprints.md`](blueprints.md). |
| `apps/crafting-helper/src-tauri/` | The Tauri glue: the global hotkey, single instance, the Win32 clipboard read and the refusal tone, the commands, the Settings window (label `settings`) and the overlay (label `overlay`, made click-through at start, moved where `ch-app` places it and shown without taking focus), the screen watch's real backend (`screen.rs`, over `qt-capture` and `ch-watch`'s reader) and the one file that reads the window in front and its title (`game_window.rs`), each window with its capability under `capabilities/`, opening Craft of Exile's address and the trade search's link through `tauri-plugin-opener` from Rust alone (registered without its link handler, no `opener:` permission in any capability), the update transport and the league list's fetch (`ep_net::fetch_trade_leagues`) over the shared `ep-net`, and the restart into an installed update. |
| `apps/crafting-helper/src/` | The main page (`App.svelte`; the shared parts under `components/`: the grab prompt, the grabbed text, the status line and `CurrencyPicker.svelte`; the craft area under `craft/`, one component per part, from `CraftArea.svelte`; the trade panel under `trade/`, `TradePanel.svelte` and `TradeModifiers.svelte`; the commands `bridge.ts`, the paste route `paste.ts`, the view's types `ipc.ts`), the Settings window's page under `src/settings/` (`settings.html`, one component per section, listed in `sections.ts`, the league's `TradeSection.svelte` among them) and the overlay's under `src/overlay/` (`overlay.html`, `Overlay.svelte`: the step panel). |
| `apps/crafting-helper/theme/` | The quest tracker's theme, copied byte for byte (`theme.test.ts`). |
| `apps/crafting-helper/data/` | The modifier data the extractor writes ([research note](crafting-helper-research.md#data-files)), embedded by `ch-data` when the app is built. |
| `apps/crafting-helper/data/trade-stats.json` | The trade site's stat ids for every class's ordinary-pool modifiers, or why one is not searchable (`hybrid`, `no trade stat`), with the source URL and fetch date; written by `trade_stats.py map`, embedded by `ch-data` ([research note](crafting-helper-research.md#trade-stat-ids)). |
| `apps/crafting-helper/fixtures/` | `views/`, one main window view per state (the craft window's included, [`views/README.md`](../../apps/crafting-helper/fixtures/views/README.md)), `app-settings/`, one Settings window view per update, install and settings state, and `catalogue.json`, the `get_catalogue` answer, shared by the page's tests and `ch-app`'s. |
| `apps/shared/` | The update code the three apps share, its own Cargo workspace: `ep-net` (the only network code, the league list's fixed request included) and `ep-update`: [`docs/app-updates.md`](../app-updates.md). |
| `tools/crafting_helper/` | The poe2db data extractor (`poe2db.py`), the trade stat mapper (`trade_stats.py`: `fetch` the trade site's public stat list into a cache outside the checkout, `map` it to `trade-stats.json`, `check` the shipped file; developer-only), the release cutter (`release.py`) and the slot model (`slot_model.py`, stdlib only, built from the spec alone: `py -3 -m tools.crafting_helper.slot_model check` plans every case of `cases.json` and names each difference). |
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
cargo test --manifest-path apps/crafting-helper/Cargo.toml -p ch-app --locked --test trade    # the trade panel and its link, fake shell
cargo test --manifest-path apps/crafting-helper/Cargo.toml -p ch-app --locked --test trade_leagues  # the league setting and its refresh, fake shell
cargo test --manifest-path apps/crafting-helper/Cargo.toml -p ch-watch --locked        # the locator, reader and matcher, synthetic images and lines
CH_TOOLTIP_SHOTS="$USERPROFILE/crafting-helper-shots" cargo test --manifest-path apps/crafting-helper/Cargo.toml -p ch-watch --locked --test shots -- --include-ignored  # reads images and OCR: the owner's screenshots
cargo run --release --manifest-path apps/crafting-helper/Cargo.toml -p ch-watch --locked --example shot_timings -- <image.png> [left top right bottom]  # reads an image and OCR: one shot's times
cargo test --manifest-path apps/quest-tracker/Cargo.toml -p qt-capture --locked -- --include-ignored  # reads OCR: qt-capture's own tests and its OCR self-test
cargo test --manifest-path apps/crafting-helper/Cargo.toml -p ch-data --locked         # the data: join, valid modifiers, odds
cargo test --manifest-path apps/crafting-helper/Cargo.toml -p ch-data --locked --test trade  # start weights, the offer, the trade query
cargo test --manifest-path apps/crafting-helper/Cargo.toml -p ch-blueprint --locked    # the format, the catalogue, the validator
cargo test --manifest-path apps/crafting-helper/Cargo.toml -p ch-engine --locked       # route choice, outcomes, the seeds' paths, state order, the parity test
cargo test --manifest-path apps/crafting-helper/Cargo.toml -p ch-engine --locked --test slots  # the parity test: the engine against the slot model's cases
cargo test --manifest-path apps/crafting-helper/Cargo.toml -p ch-engine --locked --test seeds_guides  # the four blueprints from the guides: hit, miss, refused start
cargo test --manifest-path apps/crafting-helper/Cargo.toml -p ch-engine --locked --test rules  # jewels, weak-magic-start, certain removals first, essence swaps
cargo test --manifest-path apps/crafting-helper/Cargo.toml -p ch-engine --locked --test fracture  # the fracture goal, the count conditions, a full-side desecration
cargo test --manifest-path apps/crafting-helper/Cargo.toml -p ch-engine --locked --test relax  # the one-target tier relaxation when no route exists
cargo test --manifest-path apps/crafting-helper/Cargo.toml -p ch-engine --locked --test blocker  # the reveal step's blocker advice
cargo test --manifest-path apps/crafting-helper/Cargo.toml -p ch-engine --locked --test essence_swap  # the fractured essence swap: survival, both outcomes, the Dusk Ring
py -3 -m unittest tests.test_crafting_helper_slot_model                                # the slot model: its baselines, cases and invariants
py -3 -m unittest tests.test_crafting_helper_mechanics_doc                             # crafting-mechanics.md: headings, sources, labels
cargo run --manifest-path apps/crafting-helper/Cargo.toml -p ch-engine --locked --example check_blueprints -- apps/crafting-helper/blueprints  # check blueprints
npm --prefix apps/crafting-helper test                                                  # the pages, in jsdom
node apps/crafting-helper/scripts/scroll-height.mjs                                     # headless Edge: no view scrolls past its content
py -3 -m unittest tests.test_crafting_helper_guard                                      # the guard
py -3 -m unittest tests.test_crafting_helper_release                                    # the release cutter and workflow
py -3 -m unittest tests.test_crafting_helper_poe2db tests.test_crafting_helper_data     # the data extractor and the shipped data
py -3 -m unittest tests.test_crafting_helper_trade_stats                               # the trade stat mapper, offline
py -3 -m tools.crafting_helper.trade_stats check                                       # the shipped trade-stats.json against the class files
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
  `a_belt_with_charm_slots_and_a_flask_implicit_reads_unchanged`; since
  0.1.4, over `ch-item`'s constructed crossbow with a hybrid increased
  Physical Damage and Accuracy prefix beside the plain one, a tooltip
  showing the summed line is never read as another item
  (`a_summed_hybrid_line_never_reads_as_another_item`), and an item that
  already held a plain pair carrying a hybrid's lines still reads as a
  change, the guard looking at the item before the step
  (`a_held_plain_pair_with_a_hybrids_lines_reads_as_a_change`);
  `ch-item`'s `constructed_crossbow_hybrid_beside_plain_physical` parses
  that crossbow);
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
  modifier no target names holds a slot unticked, an unrevealed modifier
  holds the slot of the missing target it waits to become, as the
  engine counts it
  (`an_unrevealed_modifier_holds_the_slot_of_a_missing_target`), an
  unticked fractured target stays selectable
  (`an_unticked_fractured_target_stays_selectable`), `set_targets` refuses
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
- **The trade panel and the league.** `ch-data`'s `tests/trade.rs` holds
  the baseline (the add pool's odds unchanged,
  `trade_baseline_add_pool_odds_are_unchanged`) and the targets on the
  shipped data: a rarer modifier and a stricter floor weigh more, a base
  without spawn weights takes the default and says why, the offer lists
  the ordinary pool with what can be searched, and the query asks for a
  magic item of the base with one entry per modifier, weights ordered by
  rarity and over each modifier's top value, so Live 2's ring with
  Accuracy at T1 outranks Life and Cold Resistance at their best rolls
  (`trade_weight_rarer_floor_outranks_commoner_best_rolls`), two only
  when both sides are picked, the `(Local)` stat on
  armour and martial weapons, a value floor only for a chosen tier, and
  refuses a modifier with no trade stat. `ch-app`'s `tests/trade.rs`
  drives the panel through `App` with the recording fake shell: closed and
  opening nothing by default
  (`trade_panel_is_closed_and_opens_nothing_by_default_baseline`), only
  the trade address built from the app's own state opened
  (`open_trade_search_opens_only_the_trade_address_built_from_the_state`),
  the league percent-encoded into one path segment, refusals that keep
  the state, no link past 2,000 characters, the craft untouched
  underneath, and log lines with counts only; since 0.1.4 a blank item
  level searches the suggested one
  (`trade_search_with_a_blank_item_level_uses_the_suggestion`) and a typed
  one wins (`trade_search_keeps_a_typed_item_level`), and `ch-data`'s
  `trade_suggested_item_level_is_none_without_floors_baseline` and
  `trade_suggested_item_level_is_the_highest_floor_tier_level` hold the
  suggestion itself. `tests/trade_leagues.rs`
  holds the league list: nothing asked for at start or by default
  (`trade_leagues_are_not_requested_at_startup_or_by_default_baseline`),
  a refresh fills the list for this run, a failed, malformed or oversized
  answer keeps the list and the league, other realms, private leagues and
  bad names skipped, a second refresh while one runs sends no request,
  the saved league always offered, and log lines with counts only; the
  fake shell answers, so no test reaches the trade site, and `ep-net`'s
  own tests send the league request to their 127.0.0.1 server.
  `tests.test_crafting_helper_trade_stats` runs the mapper's rules
  (match, hybrid, no trade stat, duplicate ids, local) on a synthetic stat
  list and class files, each with a negative control, with no network.
  The page's `src/trade/trade-panel.test.ts` and the Settings window's
  tests render the trade fixtures (`fixtures/views/trade-*.json`,
  `fixtures/app-settings/trade-league*.json`); `trade-floor` (0.1.4) shows
  the suggestion in the blank field and the help line
  (`trade_item_level_suggestion_shows_in_the_blank_field`).
- **Spacing and alignment** (0.1.4's UI review). `src/ui-spacing.test.ts`
  reads the declarations jsdom receives from the shipped styles
  (`vite.config.ts` keeps the components' styles and `app.css` under
  Vitest) and pins each fix: a checkbox centred in its label row with no
  native margin, in the currency picker
  (`currency_filter_checkboxes_align_with_their_labels`) and the target
  choices (`target_checkboxes_align_with_their_labels`), and buttons and
  text controls keeping their inner padding
  (`button_text_keeps_its_inner_padding`), beside a control that the
  style reader sees what a rule sets. jsdom does no layout, so no box is
  measured ([`ui-design.md`](ui-design.md) § "Spacing and alignment").
- **The window's scroll height** (0.2.0). `scripts/scroll-height.mjs`
  measures it in a real layout: it serves the pages with the app's own
  Vite, drives headless Edge (WebView2's engine) over the DevTools
  protocol, and for every view fixture at 960x680 and 600x480 fails when
  the document scrolls at all or a scroll pane scrolls past its own
  content; it exits 2, never 0, when Edge or the page cannot start (Edge
  from its usual paths, or `MSEDGE`). It found the main window scrolling
  up to 2,222 px past the views with long lists: each visually hidden
  span sat at its own place far down a list, outside the pane's clipping.
  `src/scroll.test.ts` pins the fix in jsdom, in the style of
  `ui-spacing.test.ts`: every element taken out of the flow is held by a
  positioned box inside its scroll pane or pinned to its containing
  block's top left corner (`no_positioned_element_escapes_its_scroll_pane`,
  with the negative control
  `an_unpinned_absolute_box_in_a_scroll_pane_is_caught`).
- **0.2.0's craft changes.** `ch-app`'s `tests/craft_window.rs`: a chosen
  route keeps its number and stays selected
  (`route_numbers_stay_with_their_routes`, beside
  `choosing_a_route_previews_it_baseline`); every desecration bound to a
  side names its Necromancy omen
  (`every_side_desecration_names_its_necromancy_omen`, beside
  `an_unsided_desecration_names_no_omen_baseline`), the whittle's names
  both while targets on both sides are missing and a blocked omen names
  itself in the Fallback line; the magic belt's T2 resistances show the
  relaxation and accepting it previews a route
  (`no_route_shows_the_tier_relaxation`,
  `accepting_the_relaxation_previews_a_route`); and the reveal step
  names the blocker with the exalt's chance before and after
  (`reveal_step_names_the_blocker_with_before_and_after`, beside
  `no_blocker_line_on_a_last_desecration_baseline`). `ch-engine`'s
  `tests/relax.rs` (the belt's baseline with no route, Greater Essence of
  Insulation proposed, a blocked essence moving the proposal to the next
  target, none for a selection that already routes) and
  `tests/blocker.rs` (the Strength blockers ranked, none on a last
  desecration); the slot model's case
  `rare-wrong-reveal-same-side-exalt` keeps a wrong reveal through the
  same-side exalt, while `rare-wrong-reveal` (side full) keeps Light
  first. The page's `src/app.test.ts` renders `craft-relaxation` and
  `craft-blocker` (`relaxation_proposal_offers_lower_and_keep`,
  `reveal_step_shows_the_blocker_line`) and keeps each route's number in
  the select (`route_select_keeps_each_routes_number`).
- `tests.test_crafting_helper_mechanics_doc` reads
  [`crafting-mechanics.md`](crafting-mechanics.md): its headings, a
  source for every entry, and since 0.1.4 every guide fact labelled
  community guide, documented or not established
  (`test_guide_facts_are_labelled`).
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
  global shortcut, single instance and opener; two web addresses in the
  Rust sources, Craft of Exile's and the trade search's, each once, in
  `ch-app` (`CRAFT_OF_EXILE_URL`, `TRADE_SEARCH_URL`), the opener named and
  called once in the glue and registered without its link handler, and no
  command taking an address or a path (`open_craft_of_exile`,
  `open_trade_search` and `refresh_trade_leagues` take only the app
  state); the one request beside GitHub's, the trade site's league list:
  `ep-net`'s `TRADE_LEAGUES_HOST` and `TRADE_LEAGUES_PATH` pinned and
  defined once outside tests, and `ep_net::fetch_trade_leagues` named by
  the glue alone, once, and by no source of the quest tracker, the
  waystone roller or `ep-update`; no address, `window.open`, change of
  `location`, new-window link or opener command in the pages, and the
  bridge declaring each of the three commands once, taking
  `Record<string, never>`; the pinned CSP, the three windows'
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
  checks each error's line and column, the one-crafted and jewel rules
  among them (0.1.4: `crafted-twice.toml`, `crafted-repeat.toml`,
  `jewel-grades.toml`, `fracture-currency.toml`), and reads the spec's
  facts table against the fact ids `relies_on` may name
  (`facts_match_the_spec_table`). `ch-data`'s `tests/isolation.rs` (0.1.4)
  holds an add its exclusions make certain, or certain not to hit, without
  weights, beside a weighed pool that keeps its percent
  (`a_weighed_pool_keeps_its_percent_baseline`), and `tests/hybrid.rs`
  (0.1.4) a hybrid prefix beside its plain family on `ch-item`'s
  constructed crossbow (`crossbows-rare-hybrid-physical.txt`): one
  modifier, one prefix slot.
  `ch-engine`'s `tests/engine.rs`
  covers route choice and outcomes, and the listed-order baseline: the
  retired `fill-desecrate-exalt`, kept as a test blueprint under
  `tests/items/gear/fill-desecrate-exalt/`, still names a plain Exalted
  Orb and still rejects a rare with one target; `tests/state_order.rs`
  covers state order over its `test-state-*` blueprints; `tests/seeds_gear.rs`,
  `tests/seeds_jewels_weapons.rs`, `tests/seeds_v2.rs` and (0.1.4)
  `tests/seeds_guides.rs` walk each shipped blueprint on a hit and a miss
  path, over the item texts under `tests/items/` (each folder's
  `README.md` says which are real and which constructed), and
  `seeds_guides.rs` each guide blueprint on a start it refuses too
  (`<id>_start_refuses`: a weak magic prefix, or a rare with the wrong
  modifier count); `seeds_gear.rs`'s
  `gloves_crit_melee_essence_miss_goes_on` holds that the gloves' essence
  is used once; `tests/rules.rs` (0.1.4) holds the guides' route rules
  over test blueprints under `tests/items/rules/`, each beside a
  baseline: a jewel offered the plain Exalted Orb where a ring keeps the
  Perfect one, `weak-magic-start` on magic boots with T3 movement speed at
  item level 82 and not with T2, a certain removal before a gamble, and an
  essence swap's odds, side and order; `tests/fracture.rs` (0.1.4) the
  Fracturing Orb's odds and outcome, the modifier count conditions and a
  desecration on a full side, over constructed boots in
  `tests/items/fracture/`;
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
  and rejects a rare with one target; an uncertain removal keeps its
  file-order place, `test_uncertain_removal_keeps_file_order`) and its
  targets (every case projects as recorded, the spec's invariants hold,
  each with a control, and a certain removal goes first,
  `test_certain_removal_goes_first`, since 0.1.4). The
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
  of the four (`V2`) name it; the two from the guides (0.1.4) are not
  among them, since they use only rules the cases already pin, and
  `ch-engine`'s `tests/seeds_guides.rs` pins them instead. Two cases
  (0.1.4) cover Certain removals first: `rare-certain-removal-first` and
  its control, `rare-uncertain-removal-keeps-order`. The last-target
  desecration (0.1.2) is in
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

**Refreshing the trade stat ids** (after a data refresh, or when the trade
site changes its stat list): `py -3 -m tools.crafting_helper.trade_stats
fetch` (one request, into the cache outside the checkout), then `map`
(offline; prints the coverage per class) and `check`, then the `ch-data`
and `ch-app` trade tests, whose anchors move with an id. The rules and the
last coverage are the research note's
[Trade stat ids](crafting-helper-research.md#trade-stat-ids).

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
   creates a **draft** release. The draft holds the built `Crafting
   Helper.exe` as `Crafting.Helper.exe` (GitHub stores a space in an
   asset name as a dot), `Crafting.Helper.exe.minisig` (the signature the
   app checks before it installs an update), `SHA256SUMS.txt` (both
   names' hashes), the same exe as `crafting-helper.exe` with its own
   `crafting-helper.exe.minisig` (signed under that name, for 0.1.x
   copies, which look up only those names; every release from 0.2.0
   carries them) and the body from step 2. Without
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
  (<reason>): grab the item after each step.` instead.
  **A hybrid modifier's summed line** cannot be read either. Where a
  hybrid (two lines under one modifier, such as increased Physical Damage
  with Accuracy) shares a line with a plain modifier on the item, the
  plain tooltip may show one line with the two values added up (not
  measured), which no tier accounts for. Since 0.1.4 the watch refuses a
  read (unreadable, as for a value that fits two tiers) when two different
  modifiers on the read carry two lines of a hybrid the base can roll and
  at least one of them is not on the item before the step with the same
  tier and values: only then can a summed line hide in it. It never reads
  one as another item. An item that already held such a plain pair, with
  the same values, (an attack weapon with increased Physical Damage and an
  Accuracy prefix, an armour with a defence and Life or Mana, a caster
  with spell damage and Mana) reads as before; a step that rerolls the
  pair's values (a Divine Orb) is refused. The narrow case where both
  sums stay inside their tiers reads as unchanged, and the refusal reuses the ambiguous-tier wording rather
  than naming the hybrid; grab the item after a step that may have added
  one. Eight of the ten
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
    before use (`omens-active-before-use`, now all seven state-ordered
    routes').
  - **The fractured essence swap rests on statements, not measurements**
    (since 0.2.0). That a Perfect or special essence on a full side
    removes from that side (`essence-full-side-removal`), never a
    fractured modifier (`fractured-not-removed`), each modifier it may
    take equally likely (`removal-equally-likely`), is what community
    guides and the owner say, not established and not measured; that the
    removal may take an unrevealed desecrated modifier is the owner's
    statement alone. The Survival line's chance and the annulment's
    chance of sparing the essence's modifier rest on them. No Perfect
    essence's own modifier is in the class lists, so it cannot be a
    target: the route uses Essence of Hysteria. What the route plans
    after an annulment that took the essence's modifier is the engine's
    reading, pinned only as far as the essence step coming back; the
    Python slot model has no essence swap.
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
  - **One crafted modifier per item is checked on the blueprints, not
    on the item.** Since 0.5 an item holds at most one crafted modifier
    (an essence's of any grade, a special essence's, a liquid emotion's
    or an alloy's). Every shipped blueprint places at most one, and the
    check refuses one that places two with no annulment or Chaos Orb
    step between them, or repeats a crafted step on a miss. That is a
    plausibility check: the removal between two crafted steps may take
    another modifier. The app cannot see a crafted modifier the item
    already had before the craft (whether a `Ctrl+Alt+C` copy marks one
    is not measured), so a route may still offer an essence on such an
    item. What the game does then was not tried here; the outcome is
    classified from the item as usual.
  - **Alloys count as the crafted modifier.** The 0.5.0 patch notes have
    an alloy add a crafted modifier by replacing one already on the item,
    and an item holds one, so an alloy's modifier is the item's one
    (`alloy-counts-as-crafted`, documented). `boots-runic-ward` keeps its
    Sovereign Alloy and drops the post's Essence of Horror for it, and
    `weapons-alloy-budget`, built on two alloys, was retired.
  - **The Fracturing Orb's odds rest on two community readings.** The
    odds count the explicit modifiers that are not fractured, not
    desecrated (revealed or not) and not the Mark of the Abyssal Lord
    (Essence of the Abyss's modifier), and take each as equally likely.
    That the orb never picks the mark (`abyss-mark-unfracturable`) or a
    desecrated modifier (`desecrated-unfracturable`; that it skips an
    unrevealed one too is our reading) is what the Mobalytics guides say,
    not established and not measured; the step's basis names both. If
    either is false, `fracture-first`'s one in three is one in four.
    `fracture-first` uses no Crystallisation omen on its Essence of the
    Abyss, so the essence can take the target off; that ends the craft.
  - **A desecration on a full side** (`full-rare-desecration-rescue`)
    assumes the Necromancy omen makes the desecration take its random
    modifier from that side (`full-desecration-same-side`, not
    established), each one equally likely (`removal-equally-likely`); its
    odds are that removal's and leave out the reveal's own chance.
  - **The blocker advice names desecrated modifiers only, and the app
    does not read the reveal screen** (since 0.2.0). The Well of Souls
    shows three modifiers to choose from, which the app never sees (the
    screen watch reads item tooltips alone), so the reveal step's Blocker
    line is advice given before the reveal: if the target is not offered,
    take one of these if it is. Its candidates are the class's desecrated
    modifiers on that side at the item level; an ordinary modifier a
    reveal may also offer (`reveal-offers-base`, not established) is never
    named, since which ones is not known. Each candidate's gain is the
    engine's reading that a revealed modifier's families leave the exalt's
    pool (the same reading the blocker blueprints rely on), not measured;
    the chances are the step odds' usual rough figures. A listed-order
    route gives no advice and keeps its written miss transition.
  - **The tier relaxation lowers one target, to one currency's tier.** It
    is proposed only when no route exists, lowers exactly one target's
    floor to the tier an unblocked essence, alloy or liquid emotion adds
    it at, and only when a route through that currency then exists; it
    never raises a floor or lowers two targets together, so a list that
    would need two lowered gets no proposal.
  - **The state-ordered routes rely on facts that are not established.**
    Each of the seven names them in its `relies_on`, by their id in the
    spec's facts table
    ([`crafting-slots-spec.md`](../models/crafting-slots-spec.md)
    § "Facts"); community guides state them or they are inferred, and
    none was measured in the game. "The first four" below are
    `close-a-side`, `essence-desecrate-double-slam`,
    `magic-augment-essence` and `rare-clean-then-slam`: Omen of Greater
    Exaltation with a Sinistral or Dextral Exaltation omen adding both
    modifiers on that side (`greater-with-side-omen`, the first four and
    `weapons-physical-generic`), Greater Exaltation needing two free
    slots (`greater-needs-two-free`, the same and
    `body-armours-single-prefix`), an exalt or augment with no side omen
    landing on the open side when the other is full
    (`open-side-landing`, `close-a-side`, `magic-augment-essence`,
    `weapons-physical-generic`'s augment and
    `body-armours-single-prefix`'s isolating Greater exalt), Whittling
    taking the lowest modifier level (`whittling-mod-level`, the first
    four), a fractured modifier never being annulled or chaosed
    (`fractured-not-removed`, the first four), the Necromancy and Abyssal
    Echoes omens needing to be active before use
    (`omens-active-before-use`, all seven since 0.1.2's last-target
    desecration, 0.1.4 and 0.2.0), and what Sinistral Necromancy does on a class
    with no desecrated prefixes (`necromancy-without-pool`,
    `essence-desecrate-double-slam`'s alone among them: the last-target
    desecration is never offered without a pool, so it does not rely on
    it). The two listed guide blueprints of 0.1.4 name theirs too:
    `fracture-first` relies on `fractured-not-removed`,
    `omens-active-before-use`, `abyss-mark-unfracturable` and
    `desecrated-unfracturable`, and `full-rare-desecration-rescue` on
    `omens-active-before-use`, `necromancy-without-pool`,
    `removal-equally-likely` and `full-desecration-same-side`. The
    state-ordered `fractured-essence-swap` (0.2.0) relies on
    `essence-full-side-removal`, `fractured-not-removed`,
    `removal-equally-likely` and `omens-active-before-use`. The craft window does
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
  `Crafting Helper.exe.download` beside the exe and renames the running exe
  to `Crafting Helper.exe.old` (the running exe's own name, so
  `crafting-helper.exe.download` and `.old` for a copy that kept its 0.1.x
  name); in a folder the app cannot write (under
  `Program Files`, for example) the install fails at once with "the app's
  folder cannot be written", before any download.
- **The `.old` file stays until the next start.** Windows does not let a
  running exe be deleted, so the replaced exe is left as
  `Crafting Helper.exe.old` and removed by the next start, which retries
  for about 30 s while the old process ends. Until then, to roll back by
  hand: quit the app, rename `Crafting Helper.exe` away and
  `Crafting Helper.exe.old` back to `Crafting Helper.exe` (the same with
  `crafting-helper.exe` for a copy that kept its 0.1.x name).
- **An updated 0.1.x copy keeps its old name.** An update replaces the
  running file where it is, so a 0.1.x `crafting-helper.exe` stays
  `crafting-helper.exe` after updating to 0.2.0 and later; it is the same
  program. A 0.1.x copy reaches 0.2.0 or later through the legacy
  `crafting-helper.exe` asset (a copy still at 0.1.x would stop updating
  if a release left that asset out); once updated, it looks up
  `Crafting.Helper.exe` like any 0.2.0, whatever its file is called.
  Rename it by hand, or download `Crafting Helper.exe` afresh, to take
  the new name.
- **The release asset's name has a dot for the space.** GitHub stores
  `Crafting Helper.exe` as `Crafting.Helper.exe`, so a browser download
  saves that name; the program runs under either.
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
  which is for a look in the browser preview (`craft-side-full`). 0.1.4's
  four guide routes, its route rules, the trade suggestion and the hybrid
  guard are proven the same way, by the engine's walks and tests over
  constructed items and the page's tests, with no craft taken in the game;
  its spacing fixes are pinned by the declarations the page's tests read,
  not by a measured layout.
- **Craft of Exile's page is outside this project.** The button opens its
  beta address with `?game=poe2`; if the site moves or drops that address,
  the button opens whatever the site serves there until the app is
  updated.
- **Not every modifier can be searched on trade.** A modifier with more
  than one line (a hybrid, 77 of the 900 ordinary-pool modifiers) is not
  searchable: the trade site lists each line as a stat of its own, and a
  search by one line would also find the plain modifier with that line.
  Neither is a modifier whose text the site's stat list does not carry
  (32, such as `#% reduced Attribute Requirements`, which the site lists
  only as `increased`); the app does not guess a rewrite. Both are listed
  under *Not searchable on trade* with the reason
  ([research note](crafting-helper-research.md#trade-stat-ids)).
- **The weighted sum ranks by value times weight.** The trade site sums
  each modifier's rolled value times its weight; the app divides each
  weight by the modifier's top value (the maximum roll of the best tier
  it can have at the item level, tier 1 with none) so that a modifier
  adds at most its rarity weight and a large-valued one does not win by
  its numbers alone. A high roll of a common modifier can still outrank a
  low roll of a rare one, most of all when the rare one's floor is far
  below its best tier. The ranking is a guide, not the odds. (Until Live
  2, 2026-10-30, the weight was over the floor's minimum roll, which let a
  high roll of a commoner modifier outrank the rarer one at its floor.)
- **The trade page opens sorted by price.** The link asks for the
  weighted sum as the sort (`sort: {"statgroup.1":"desc"}`), yet in Live
  3 (2026-10-30) the trade site opened the search sorted by price;
  whether the site honours a sort carried in the link is not
  established. Click the Weighted Sum column to rank the results by
  rarity.
- **A weight is one add's odds**, from poe2db's spawn weights: about 1 in
  N modifiers added to that side of that base is this one, at the floor
  or better. Where the data has no usable spawn weight (every jewel page,
  the `str_dex_int` body armour page, a tier without one), the
  weight is a stated default (the median of that side's weighed
  modifiers, or 1.0), tagged "default" with its reason, not a measurement.
- **The league goes stale each league.** The default (`Forbidden Rites`)
  and the six shipped leagues are the trade site's list of 2026-10-29 and
  2026-10-30. When a new league starts, the refresh icon fetches the
  current list for that run (it is not saved: the next start offers the
  shipped six again, and your chosen league, which is saved); the default
  and the shipped list change only with a new version.
- **The local-stat rule is a reading.** Armour classes (but for Accuracy
  Rating) and martial weapons search the trade site's `(Local)` stat ids,
  the others its plain ids. This follows the site's list and the game's
  local-modifier convention and was not measured; a wrong choice finds
  the wrong items, or none.
- **The trade site's interface can change.** The search link's `?q=`
  form, the query's `count` and `weight` groups, the stat ids and the
  league list's shape are the site's, read from its public lists and not
  confirmed by GGG; a change can make the link open an empty or refused
  search, or a refresh refused, until the app is updated (Maintenance
  triggers). The app never sends a search itself, so a broken link shows
  in your browser, not in the app.
- **The link is copied by hand.** The app writes nothing to the
  clipboard; select the link in the panel and press `Ctrl+C`.
- **0.1.3's trade panel and league list are proven by tests.** The panel,
  the link it builds and the league refresh run in `ch-app`'s tests with a
  fake shell, and the league request in `ep-net`'s against a 127.0.0.1
  server; whether the trade site answers the built link as intended is
  checked in a live session, not by the tests.

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
  together, with this guide's Privacy section.
- **The trade site changing its stat list, its search or its leagues**: a
  stat id moved, renamed or added, or a modifier's text changed, means
  rerunning `trade_stats.py fetch` and `map` (Working on the app,
  "Refreshing the trade stat ids") and the `ch-data` and `ch-app` trade
  tests; a data refresh means `map` again too, since `check` refuses a
  `trade-stats.json` whose modifier ids are not the class files'. A change
  to the search address or the query's form: `TRADE_SEARCH_URL` in
  `ch-app`'s `trade.rs` and the guard's `TRADE_SEARCH_URL` together, or
  `ch-data`'s `trade_query`, with this guide's Privacy section. A change
  to the league list's address or shape: `ep-net`'s `TRADE_LEAGUES_HOST`
  and `TRADE_LEAGUES_PATH` with the guard's pins, or `ch-app`'s
  `league.rs`.
- **A new league**: `DEFAULT_TRADE_LEAGUE` and `SHIPPED_TRADE_LEAGUES` in
  `ch-app`'s `league.rs` (the refresh icon covers a run until then), with
  the tests and fixtures that name them (`CH_FIXTURES=1`) and this guide.
- **A dependency bump**: keep the versions equal to the quest tracker's
  (`LockedToQuestTrackerPins`); bump them together.
- **GitHub changes its API or its release flow**: the release list's
  endpoint or fields, the `/releases/download/` redirect, or its asset host;
  or the repository, the `crafting-helper-v` tag prefix or the asset names
  change. Update the shared `ep-net` and `ep-update` with
  `tests/test_app_updates_guard.py` and `tools/crafting_helper/release.py`
  together ([`docs/app-updates.md`](../app-updates.md)). A renamed exe
  keeps the legacy assets: the release workflow attaches the exe under
  every name an older copy looks up, each signed under that name, as it
  attaches `crafting-helper.exe` for 0.1.x since 0.2.0.
- **The signing key changes** (rotated or lost): follow
  [`docs/app-updates.md`](../app-updates.md) § "Rotating the key"; copies
  built with the old key cannot install a release signed with the new one.

## Release notes

### 0.2.0

Route choice, Necromancy omens, a tier proposal, blocker advice, the
fractured essence swap and the new name (#37).

- **Picking a route sticks.** The route list now numbers every route the
  same way whichever one you look at, so choosing Route 2 shows Route 2
  and leaves it selected; before, it could flip back to the first.
- **Every desecration that must land on one side names its Necromancy
  omen**: "Use Omen of Dextral Necromancy, then apply the Preserved
  Collarbone; the desecrated modifier will be a suffix, for …"
  (Sinistral for a prefix). The ring whittling route's desecration now
  uses the omen of the last target's side too: while targets on both
  sides are left it names both, and the one once a single target is
  left. If that omen is blocked it uses the plain bone and says why.
- **A target list no route reaches may get a proposal.** If lowering one
  target's tier lets an essence (or an alloy or liquid emotion) add it
  for certain, and a route then
  exists, the app says so, with **Lower to T<n>** and **Keep T<n>**
  buttons. A magic belt with T2 Fire, Cold and Lightning Resistance gets
  "Lower +#% to Fire Resistance from T2 to T3: Greater Essence of
  Insulation adds it at T3, which unlocks a route." Keep hides the
  proposal and leaves your targets as they were.
- **Blocker advice at the reveal.** When an exalt on the same side comes
  after a desecration's reveal, the reveal step says which desecrated
  modifier to take if your target is not offered, what ordinary modifier
  it keeps out of the exalt, and how much it raises the exalt's chance
  ("If +#% to Cold Resistance is not offered, take +# to Strength and
  Dexterity or +# to Strength and Intelligence (keeps out +# to
  Strength): the suffix exalt's chance of a target goes from ~17% to
  ~20%"). Only desecrated modifiers are suggested, and the app does not
  read the Well of Souls screen, so it cannot see what was offered.
- **A wrong reveal is kept through that exalt.** When a reveal offers
  none of your targets and an exalt can still add one on the same side,
  the route keeps the revealed modifier for that exalt (it can only help
  it) and takes it off with Omen of Light once that side is done, instead
  of right away. A side that is already full takes Omen of Light first,
  as before.
- **A fractured modifier can make an essence's modifier certain.** On a
  rare whose prefix or suffix side is full, an essence that adds a
  modifier there (Essence of Hysteria) has to remove one of that side's
  modifiers first, and never the fractured one, so the fractured
  modifier narrows what it can take. When none of what is left is one
  of your targets, the essence's modifier is certain; with only one
  modifier left for it to take (a Dusk or Gloam ring's smaller side, for
  one), which modifier goes is certain too. A new route takes that
  craft, and its essence step says what it can remove and what stays
  ("Essence of Hysteria for #% increased Mana Regeneration Rate
  (suffix); it removes the unrevealed desecrated suffix or +10(9-12) to
  Dexterity, one at random; +23(21-25)% to Fire Resistance is fractured
  and stays"). When an unrevealed desecrated modifier sits on that
  side, the step's **Survival** line gives its chance of surviving the
  swap (1 in 2 with two modifiers it can take) and what to do either
  way: if it stays, reveal it; if it goes, annul on that side with Omen
  of Sinistral or Dextral Annulment (it may take the essence's modifier
  instead, with the chance shown, and the route then plans again from
  the item), then desecrate with that side's Necromancy omen and reveal.
  Perfect essences' own modifiers cannot be targets (none adds a
  modifier from the item's own list), so Essence of Hysteria is the
  essence the route uses; and how often the essence or the annulment
  takes each modifier is not measured: the chances assume each is
  equally likely.
- **The main window no longer scrolls past its content.** With long
  lists the window could scroll far below the last line into empty
  space.
- **The program is now `Crafting Helper.exe`.** GitHub lists the download
  as `Crafting.Helper.exe` (it turns the space into a dot); rename it if
  you like, it runs either way. A 0.1.x copy updates in place as before
  and keeps its old name, `crafting-helper.exe`: it is the same program,
  and you can rename it or download the new file whenever you like. Your
  settings carry over.
- The app still sends the game nothing.

### 0.1.4

What the maxroll and Mobalytics crafting guides teach (#35).

- **Four new routes:**
  - a physical weapon (bows, crossbows, maces, quarterstaves, spears,
    talismans): roll a magic weapon until its increased Physical Damage is
    one of its two best tiers, then Greater Essence of Abrasion for flat
    physical damage, a Greater Exalted Orb with the Greater and Dextral
    Exaltation omens for two suffixes, and a desecration for the last
    target;
  - a body armour with one top defence prefix: a Greater resistance
    essence, a desecration that fills the suffixes and stays unrevealed,
    then one Greater Exalted Orb with Omen of Greater Exaltation that can
    only land on the prefixes;
  - fracture first: on a rare with four modifiers, Essence of the Abyss,
    then a Fracturing Orb with one chance in three to fix a target, then
    annulments, Chaos Orbs, a desecration and side-restricted Perfect
    exalts;
  - the rescue of a full rare that lacks one target: a desecration on
    that target's side, revealed with Abyssal Echoes, and Omen of Light
    and another bone for a wrong reveal.
- **One crafted modifier per item**, as 0.5 has it: no route places a
  second essence, alloy or liquid emotion on an item. The weapon alloy
  route that used two alloys is retired; the Runic Ward boots route no
  longer uses Essence of Horror (the Sovereign Alloy's Runic Ward is its
  one crafted modifier); the critical gloves route uses its Essence of
  Hysteria once and moves on after a miss.
- **A weak magic start is sent back.** A magic item whose wanted prefix
  is below the two best tiers it can have at its item level is not turned
  rare: the route says a new base is worth more than an upgrade.
- **Certain steps say so.** A step whose result is settled shows 100% (or
  0%) even where the odds would otherwise be unknown, such as an exalt
  that can roll nothing but the target, on a jewel too. On the routes that pick their
  steps from the item, a removal that can only take modifiers you do not
  want now comes before a step that gambles.
- **Jewels are never offered a Greater or Perfect currency**, which only
  costs more there: the plain orb instead.
- **Find a base on trade suggests an item level.** Once a modifier has a
  tier floor, the empty item-level field shows the level that floor needs
  (65 for at least T2 movement speed on boots), the search asks for that
  level and up, and the help line says so. Type a level to use your own.
- **The screen watch no longer risks misreading a hybrid modifier.** If
  the tooltip adds a hybrid modifier's line (such as increased Physical
  Damage with Accuracy) to a plain modifier's line, the sum could be read
  as a different tier. The watch now asks for a grab when a step could
  have brought such a line in. An item that already held, say, increased
  Physical Damage and an Accuracy prefix with the same values is read as
  in 0.1.3.
- **Spacing fixes**: the checkboxes of the currency filter (Settings and
  *Blocked for this craft*) and of the target choices line up with their
  labels, and button and field text no longer touches their edges.
- The app still sends the game nothing.

### 0.1.3

- **Find a base on trade** (#30): a new button in the header opens a panel
  for starting a craft from a bought base. Pick an item class, a base
  type, an optional minimum item level and the modifiers you want (up to
  8, each any tier or at least a tier you choose), and *Search on the
  trade site* opens the official trade site in your browser, searching
  for a magic item of that base carrying two of them, a prefix and a
  suffix (or at least one, if you choose). The search carries a weighted
  sum of them in which the rarer modifiers count for more; the site opens
  the results sorted by price, so click the Weighted Sum column to rank
  them by it.
- Each modifier shows its weight, higher the rarer it is to roll on that
  base: 12.0 means about 1 in 12 modifiers added to that side is this
  one. Where the data has no weight for it (every jewel, some tiers), the
  weight is a stated default, marked "default" with the reason.
- Modifiers with more than one line, and the few the trade site has no
  stat for, are listed as not searchable, with the reason.
- The search link is shown in the panel, for you to select and copy. The
  app copies nothing to your clipboard, never buys or whispers for you,
  and sends the trade site no search itself: your browser opens the link.
- **A League setting** in Settings, *Trade*: the league the search opens
  in (Forbidden Rites by default), with a refresh icon beside it that asks
  the trade site for its current leagues. That click is the only time the
  app asks; the new list lasts until the app closes, and your choice is
  saved.
- The craft is untouched while the panel is open: a guided craft, the
  overlay and the screen watch carry on, and *Back to the craft* returns
  to it.
- The app still sends the game nothing.

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
  their slot; an unrevealed one counts as the missing target on its side
  it will be revealed as, and an unticked fractured target stays
  selectable, since ticking it again takes no new slot.
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
