# Driving our own apps: the `app-webview` MCP server

A live session of one of this repository's Tauri apps (Crafting Helper, Quest
Tracker, Waystone Roller) used to hand every click to the owner. The apps'
windows are WebView2 pages, so an agent can click them itself through the
Chrome DevTools Protocol (CDP) — the same way Playwright tests WebView2 apps.

## How it fits together

- **`tools/app_webview.py launch <exe>`** starts the app with
  `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9333`, set
  for that one process only, waits until `http://127.0.0.1:9333/json/list`
  answers, and prints the app's pages (an app may also have an overlay page).
  It refuses when the port already answers. `status` prints the same list.
  An app started any other way — the shipped exe a player runs — exposes no
  port: nothing in the apps themselves changed.
- **`app-webview`** in `.mcp.json` is the pinned `@playwright/mcp@0.0.82`
  with `--cdp-endpoint http://127.0.0.1:9333`. It connects on its first tool
  call, so it stays idle until an app was launched by the tool above.
- **`playwright`** (headless Edge) stays for ordinary web pages. A page an app
  opens in the default browser — a trade search, a guide — is read there, by
  navigating to the URL the app shows or logged.

Measured 2026-10-30 with Crafting Helper 0.1.2 (release build): the
environment variable is honoured; `/json/list` listed `Crafting Helper`
(`http://tauri.localhost/`) and `Crafting Helper Overlay`
(`http://tauri.localhost/overlay.html`).

## Who uses it

`live-operator` carries the `mcp__app-webview__*` tools (tabs, snapshot,
click, type, select, key press, hover, wait, screenshot, evaluate, console)
and a few read-only `mcp__playwright__*` ones. It never navigates the app's
page away from `tauri.localhost`, and closes the app with its window close at
teardown, before restoring the app's backed-up settings and log.

## What it must never do

Drive the game. These tools reach the app's own window only; Path of Exile 2
gets no click, key or movement from an agent (`AGENTS.md` § "Legal"). A step
that needs one stays the person's.

The port listens on `127.0.0.1` only, for as long as that app runs. Close the
app when the session ends.
