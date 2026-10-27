# Review of the Settings window

The design check of the tray's Settings window (owner, 2026-10-27):
`apps/quest-tracker/settings.html`, its root
`apps/quest-tracker/src/settings/SettingsWindow.svelte` and its one section,
`UpdatesSection.svelte`. The spec is the brief's § "The Settings window" and
`component-spec.md` § "SettingsWindow". It was reviewed on the browser
preview (`npm --prefix apps/quest-tracker run dev`, then
`http://127.0.0.1:1420/settings.html?state=<state>`), every one of the six
states rendered side by side at 420 px wide (the window's width), at 1× and
2×, in headless Edge.

Tools: impeccable 4.7.1 (`impeccable detect`, then its `critique` and
`audit` procedures) and taste-skill 2.1.5 (`design-taste-frontend`, the app
screen run: `lint.mjs --app` and `inspect.mjs --app`). Both ran inside the
item's implementer, a subagent with no owner to ask and no subagents of its
own: impeccable's critique therefore ran in a single context rather than as
its two isolated assessments, and its interview step was replaced by the
brief, the component spec and the owner's request of 2026-10-27. Its
browser overlay did not run (no browser automation exposed); the
screenshots were taken with headless Edge and read by eye.

detector: `impeccable detect --json` over `src/settings/` and `settings.html` printed `[]`.
critique: Nielsen 32/40 (single context). Visibility of status 4 (the status line names every state in words, a polite live region reads it out, "Check now" is disabled while a check runs); match 3; control 3 (every action is undone by the next click; no Esc, see N3); consistency 4 (the panel's own toggle, buttons, group label and type); error prevention 3; recognition 3; flexibility 2 (one control path, nothing to accelerate); minimalism 4; error recovery 3 (a failure says why in one line and "Check now" stays enabled); help 3 (the helper line says what the setting does and does not do).
audit: accessibility 4 (`lang`, a `main` named by its `h1`, one region per section named by its `h2`, the credit as the page's footer outside `main`, the toggle a `switch` named by its label and described by its help, focus rings from the styled layer, text contrast 5.8:1 for the muted grey and above on `color.panel-bg`); performance 4 (no motion of its own, no remote load); theming 4 (tokens only, no new token); responsive 3 (a fixed 420 px window; the content wraps, nothing overflows); integrity 4.
taste: `lint.mjs --app`: 0 FAIL, 0 WARN. `inspect.mjs --app` (state `failed`): 1 FAIL before the fixes (`blank-gap`), 0 FAIL after; 3 WARN kept, below.

## Findings and what was done

- **F1, fixed** (taste `blank-gap`, impeccable layout): the window filled
  the viewport (`min-height: 100vh`) and pushed the credit to its bottom, so
  any window taller than the content opened a blank band between "Version"
  and "Created by ST4H". The page is now as tall as its content; the shell
  sizes the window to it (about 280 px at 1×), and spare height falls below
  the credit.
- **F2, fixed** (taste `tiny-text`, impeccable critique): over half the
  text was under 13 px, most of it the helper line, which is the window's
  privacy promise ("Downloads nothing. Off: Quest Tracker never connects
  unless you ask it to check."). The overlay's 12 px Caption suits a glance
  over a game; this window is read at a desk. The helper is now 13 / 18 in
  `color.text-muted`; the version and the credit stay Caption.
- **F3, fixed** (impeccable audit): the credit sat inside `main`. It is
  now the page's own `footer`, beside `main`, so it is a separate landmark;
  a test pins it.

## Kept, with the reason

- **N1** — taste `palette-cream`: the text colour `#ECE6D9` is the
  harvest's `color.text-primary`, the owner's design (`review-comps.md`);
  the window reuses the panel's tokens and invents none.
- **N2** — taste `hero-void` and the 768 / 390 px renders: the inspector
  measures 1440, 768 and 390 px wide; the window is 420 px and not
  resizable, so only the 420 px renders were judged. `dev-server`: the
  preview is the dev server by design (its fixtures exist only there).
- **N3** — no Esc to close. Closing would need a window permission beyond
  `core:event:default` in the window's capability, which the plan keeps to
  the minimum; the title bar's close button and Alt+F4 close it.
- **N4** — the title "Settings" repeats the title bar's "Quest Tracker
  Settings". Kept: it names `main` for a screen reader, and it is where a
  later section's window starts reading.
- **N5** — the status line's text starts 24 px further right in the states
  that carry an icon (checking, up to date, available, failed) than in the
  two that do not (not checked yet, no release). Reserving the slot would
  leave an unexplained gap in those two; the text is what carries the state.
- **N6** — the toggle's "on" colour is the panel's teal accent
  (`color.edit-accent`), not a semantic green as taste-skill's app rules
  prefer: the same toggle as the panel's Settings frame, by the spec.

## Open for the owner

Whether the reviewed design is what you want (the plan's Needs human
judgement › "The Settings window's look"; default: ship it). A restyle
touches `src/settings/` only.
