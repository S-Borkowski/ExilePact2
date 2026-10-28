# Review of the Settings window (impeccable critique and audit)

Run: 2026-10-28, `impeccable` 4.7.1, `critique` then `audit` of the
waystone roller's Settings window: `apps/waystone-roller/settings.html`, its
root `src/settings/SettingsWindow.svelte` and its one section,
`src/settings/UpdatesSection.svelte`, plus the main window's header button
(`src/App.svelte`). The spec is [`../ui-design.md`](../ui-design.md) §
"Settings window" and § "Header"; the window is adapted from the quest
tracker's ([`../../quest-tracker/design/review-settings-window.md`](../../quest-tracker/design/review-settings-window.md)).

**DEGRADED: single-context** (one implementer session with no sub-agent
tool and no browser automation tool, and no owner to interview): the
critique's two assessments ran one after the other in one context, the
design review first, and its interview was replaced by the spec and the
owner's request of 2026-10-28. How the evidence was gathered:

- **Detector, source scan:** `impeccable detect --json
  apps/waystone-roller/src/settings apps/waystone-roller/settings.html
  apps/waystone-roller/src/App.svelte` printed `[]` (exit 0), after the
  fixes below. The design hook scanned each file as it was written and
  reported nothing.
- **Detector, URL scan:** `impeccable detect --json
  "http://127.0.0.1:1421/settings.html?state=install-failed"` could not
  start its browser ("Failed to launch the browser process!"), so there is
  no in-page scan and no overlay.
- **Looking at the render:** the browser preview (`npm run dev`, then
  `http://127.0.0.1:1421/settings.html?state=<fixture>`), each of the
  eleven fixtures in a 420 px frame (the window's width; a headless window
  cannot be made that narrow) in headless Microsoft Edge, at 1× and, for
  `available`, `install-downloading`, `install-failed` and
  `install-unsigned`, at 2× (`&scale=2`). The screenshots were read by eye
  and stay on the implementer's machine (no image is committed). The header
  button was looked at on the main page's preview without an update; the
  preview's view fixtures carry none, so the marked button is pinned by
  `settings_button_marks_an_available_update`, not by a render.

critique: Nielsen 31/40 (single context). Visibility of status 4 (every
check and install state is a line of words in a polite live region; the
download has a determinate bar and its numbers; Check now is disabled while
anything runs); match 3 (the app's own sentences; "Install 1.3.0 and
restart" says what the click will do, including the restart); control 3
(Cancel during a download, Try again after a failure; no undo once the
restart starts, by design); consistency 4 (the main window's buttons, focus
ring and `!` error mark; the quest tracker's switch, group label and
layout); error prevention 3 (the install button exists only for a signed
release and is disabled while a check runs); recognition 3; flexibility 2
(one control path); minimalism 3 (an unsigned release says "1.3.0" twice,
see N3); error recovery 4 (a failure names its reason in one line and
offers Try again; the running app is untouched); help 2 (the help line says
what the switch does and does not do; nothing explains a signature failure
beyond its reason).

audit: accessibility 4 (`lang`, a `main` named by its `h1`, one region per
section named by its `h2`, the credit as the page's own footer, the switch a
`switch` named by its label and described by its help, the progress bar
named "Download", the `!` marks hidden from assistive technology because
the reason's words carry it, focus rings from the main page's base rules;
text contrast from the shared tokens, muted grey 5.8:1 and above on the
panel colour); performance 4 (no remote load; the only motion is the
indeterminate bar, stopped under reduced motion); theming 4 (tokens only,
no new token); responsive 3 (a fixed 420 px window; the text wraps and
nothing overflows at 1× and 2×); integrity 4.

## Findings and what was done

- **F1, fixed** (critique, layout): a failed install's reason that wrapped
  ran its second line back under the `!` mark, so the mark no longer led
  the reason. Each status line is now a row: the mark stays in its own
  column and the reason's lines hang after it.
- **F2, fixed** (critique, consistency): a failed check ("Could not check
  for updates: GitHub answered 500.") was told by its colour alone, unlike
  a failed install and unlike the main window's status, whose errors start
  with `!`. A failed check now starts with `!` too; a test pins both.
- **F3, fixed** (audit, browser surfaces): the download's count used the
  font's default old-style figures ("o KB of 39 KB"), which read as letters
  and change width as they climb. The count now uses lining, even-width
  figures.
- **F4, fixed** (audit, layout): the window's height (300 px from the glue)
  cut off its tallest state. Measured on the preview at 1×: 303 px for the
  plain states, 325 for unsigned, verifying and restarting, 343 for a
  failed install with a two-line reason, 349 while downloading. The window
  is now 420 × 364, enough for those and a three-line reason; the page is
  as tall as its content, so spare height falls below the credit.

## Kept, with the reason

- **N1** — the switch's "on" colour is the teal accent
  (`color.edit-accent`), not a semantic green: the quest tracker's switch,
  so the two apps' Settings windows read as one.
- **N2** — the title "Settings" repeats the title bar's "Waystone Roller
  Settings": it names `main` for a screen reader, as in the quest tracker.
- **N3** — with an unsigned release, the check's line ("Waystone Roller
  1.3.0 is available. You have 1.2.0.") and the install's line ("1.3.0 is
  available but cannot be installed: the release is not signed.") both say
  1.3.0 is out. Both are the app's text (`ep-update`, shared with the quest
  tracker); the second explains why there is no install button.
- **N4** — no Esc to close: closing needs a window permission beyond
  `core:event:default`, which the window's capability keeps to the minimum.
  The title bar's close button and Alt+F4 close it.
- **N5** — the `!` mark is a character, not a drawn icon: it is the main
  window's own error mark (`ui-design.md` § "Footer"), so the two windows
  mark an error the same way.
- **N6** — the indeterminate bar (no size known) slides a quarter-width
  block. The release list always gives the size, so the case is a guard;
  under reduced motion it is a still tinted track.

## Open for the owner

Whether the reviewed design is what you want (default: ship it). A restyle
touches `src/settings/` and the header button only.
