# Crafting Helper — UI Design

The Crafting Helper is an ordinary desktop window (Tauri 2, Svelte 5,
TypeScript) with a Settings window and a click-through overlay. In 0.1.0 the
main window shows one thing: the last item the player grabbed, after copying
it in the game with `Ctrl+Alt+C` and pressing the app's hotkey (`Ctrl+Alt+X`
by default) or `Ctrl+V` in the window. The craft itself, its steps and odds,
comes in later versions, in the main window and the overlay; this document
is the direction the shell's components follow and the frame those versions
build in.

What each window does, and every command and event, is in the guide's
Command reference ([`README.md`](README.md)).

## Principles

- **Rust decides, the UI renders.** The shell sends one `View` (event
  `ch:view`, command `get_view`) to the main window and the overlay, and
  one `SettingsView` (event `ch:app-settings`) to the Settings window. A
  page shows what it was sent and sends commands; it never checks a grab,
  validates a hotkey or edits a list itself, and waits for the next view
  instead of patching its copy.
- **The clipboard is the player's.** No page reads it on its own: the main
  window takes the text of a paste the player made (the paste event's
  `clipboardData`), and nothing else. There is no "Read clipboard" button.
  A paste into a text field is that field's, never a grab.
- **Never colour alone.** Every state is also told by text or shape: an
  error status starts with `!`, an available update says its version in
  words beside the green dot, the grab's source is a word ("from the
  hotkey", "from a paste").
- **Refuse, never guess.** A refused grab, hotkey or list leaves the last
  good one in place and says why in a status line.
- **The overlay never gets in the way.** It does not take focus, lets
  every click through, and is hidden unless a craft is active.

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
| Sunken surfaces (the grabbed text, inputs) | `--color-panel-sunken` |
| Dividers, borders | `--color-panel-divider`, `--color-panel-border` |
| Text | `--color-text-primary`, `-secondary`, `-muted` |
| An available update (the dot and its words) | `--color-status-available` |
| Error status | `--color-warning` |
| Focus ring (2 px), the download bar | `--color-edit-accent` |
| Headings, group labels | `--font-family-caps` (small caps) |
| Corners | square, as the tracker |

Monospace (the grabbed text, the hotkey): the system's
`ui-monospace, "Cascadia Mono", Consolas, monospace`; no font is added.
Spacing on the 4 px grid (`--space-unit`).

## Windows

### Main window

Titled "Crafting Helper", resizable, in the taskbar; 720 × 640 by default,
usable from 480 × 400. A column of two parts:

- **The header**: the title "Crafting Helper" in small caps, and at the
  right end the **Settings** button, which sends `open_settings`. When a
  check this run found a newer version (the view's `update_available`), the
  button names it after a divider, "Settings | Update 1.3.0", with a small
  green square beside the version; its accessible name is then "Settings,
  update 1.3.0 available".
- **The "Last grabbed item" panel**, which takes the rest of the window:
  - With nothing grabbed: "Nothing grabbed yet. To grab an item:" and two
    numbered steps, "In the game, hover the item and press `Ctrl+Alt+C`"
    and "Press `<hotkey>`, or press `Ctrl+V` in this window", the hotkey
    taken from the view.
  - With a grab: the line count ("27 lines") and its source ("from the
    hotkey" or "from a paste"), then the text as the app sent it, in a
    sunken monospace block that scrolls inside the panel. The block is
    focusable, so the keyboard can scroll a long item.
  - **The status line** last (`role="status"`, polite): the app's last
    message; an error starts with `!` and takes the error colour.

A `Ctrl+V` anywhere in the window but a text field sends `grab_text` with
the paste's text, and nothing else.

### Overlay

`overlay.html`: one line, `view.overlay.line`, on a transparent,
undecorated, always-on-top window off the taskbar, about 420 × 64. The line
sits in a slightly see-through panel of the theme, one line high, cut with
an ellipsis when too long; nothing on the page can be selected or clicked,
and the window ignores the cursor. The shell shows it without activating it
only while a craft is active (`view.overlay.visible`) and hides it
otherwise. In 0.1.0 no craft is ever active, so it stays hidden; its line is
a placeholder, "Next step: shown here while a craft is active." Placing it
over the game and hiding it by foreground window are later work.

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
  suggests. Each is a currency id.", a field ("Currency id", at most 64
  characters) with **Add**, and the list, each id with its **Remove**
  button ("Remove <id>" to a screen reader). With no id: "No currency is
  blocked." The list box scrolls past 200 px. At 200 ids the field and Add
  are disabled, with "The list holds at most 200 ids." Every change sends
  the whole list with `update_settings`.
- **Updates**: the waystone roller's section, word for word with the app's
  name: the *Automatically check for updates* switch (off), its help line,
  the check's and the install's status lines, the download bar with its
  numbers and **Cancel**, the install's one button, **Check now**, and
  "Version <running version>"
  ([`docs/waystone-roller/ui-design.md`](../waystone-roller/ui-design.md)
  § "Settings window" has each state).
- **Created by ST4H**, the window's footer, plain text.

No button, link or state opens a release page or a browser.

## States

Each state is a fixture, written by `ch-app`'s `tests/fixtures.rs` and
rendered by the page's tests.

| Main window (`fixtures/views/`) | What it shows |
|---|---|
| `empty` | Nothing grabbed: the two steps, the default hotkey, no status |
| `grabbed-item` | A hotkey grab of an item's text: its line count, "from the hotkey", the text |
| `grab-refused` | A later press found an empty clipboard: the last grab kept, `!` the reason |
| `grabbed-not-item` | A paste of text that is not an item: shown, "from a paste", `!` the warning |
| `hotkey-taken` | Another program holds `Ctrl+Alt+X` at the start: `!` the status naming it |
| `overlay-active` | A craft is active: the overlay's line is visible |

| Settings window (`fixtures/app-settings/`) | What it shows |
|---|---|
| `idle`, `checking`, `up-to-date`, `available`, `no-release`, `failed` | Each state of the update check |
| `install-unsigned`, `install-downloading`, `install-verifying`, `install-restarting`, `install-failed` | Each state of the install |
| `hotkey-refused` | `Ctrl+Alt+C` asked for: `!` the refusal, the old hotkey kept |
| `blocked-currencies` | A list of three ids, each with Remove |

## Browser preview

Outside Tauri (`npm run dev`, port 1422) each page renders a fixture chosen
by the query's `state`: `http://127.0.0.1:1422/?state=empty` (default
`grabbed-item`), `http://127.0.0.1:1422/overlay.html?state=overlay-active`
(the default), and
`http://127.0.0.1:1422/settings.html?state=install-downloading&scale=2`
(default `idle`; `scale` zooms the page as the webview's scale would).
Commands are logged, not acted on: a paste is not shown as a grab. The
fixtures are reached only on the dev server, so the release bundle carries
none.

## Accessibility

- Native controls throughout (button, text input, list) so keyboard and
  screen reader behaviour comes from the platform.
- A visible focus ring (`--color-edit-accent`) on every control, the
  grabbed text block included.
- Status lines are polite live regions; an error is told by `!` and words,
  not colour alone.
- Text contrast from the tracker's tokens.
- No motion beyond transitions of 120 ms (colours, the switch, the
  download bar), and none under `prefers-reduced-motion`.
