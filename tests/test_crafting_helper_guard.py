"""The crafting helper touches nothing but its windows, one clipboard read per
grab and, through the shared `ep-net` alone, GitHub's release list and
assets and the trade site's league list: that list is the one request
beside GitHub's, sent only when the player clicks the League refresh icon.
It sends no input, watches nothing but, while a craft is guiding, the
game's window (which window is in front, and a copy of the game's own in
memory, read with Windows' OCR) and reaches no game file.

Read from the files only (no build, no network), these pin the rules of
the app (`docs/crafting-helper/README.md`, Privacy). They are the waystone
roller's guard (`tests/test_waystone_roller_guard.py`) pointed at
`apps/crafting-helper`, its helpers imported from that module rather than
copied, with these differences:

`NoInputNoWatchingNoNetworkNoGame`:

- no Rust source of the app (comments blanked, strings kept) sends input:
  the crafting helper has no paste, so each name the waystone roller allows
  in its `paste.rs` alone (`SendInput`, `KEYBDINPUT`, `GetForegroundWindow`,
  `KeyboardAndMouse` and the rest) is an offence in every Rust source here,
  as is every name that guard forbids everywhere (mouse and hardware input,
  the cursor, window messages, focus and activation, another process, hooks,
  another program's window, the game's files); the trade site's domain,
  `www.pathofexile.com` in lower case, is a web address and not a game
  file, and so is the bare `"pathofexile"` a Rust test checks stays out of
  the log, in a test under `tests/` alone; `PathOfExile.exe` stays an
  offence everywhere, tests included;
- the one exception is the overlay's show and hide: `src-tauri/src/lib.rs`
  alone may name `ShowWindow`, and every `SW_` name it holds is
  `SW_SHOWNOACTIVATE` or `SW_HIDE`, so the game keeps the focus;
  `ShowWindowAsync` stays an offence everywhere; the overlay is moved with
  Tauri's own `set_position`, and `SetWindowPos` and `MoveWindow` stay
  offences everywhere;
- the other exception (M6) is the window in front:
  `src-tauri/src/game_window.rs` alone may name `GetForegroundWindow` and
  `GetWindowText` / `GetWindowTextLength`, to tell whether the game is in
  front by its title, and names no other banned name; every other Rust
  source, the glue's `lib.rs` and the tests included, still may not;
- the quest tracker's `qt-screen` and `qt-capture`, which this app takes by
  path and unchanged for the screen watch (M6), send no input either: every
  Rust source of theirs (comments blanked) names none of the input,
  watching, show, message, focus, hook, process or game-file names above,
  but `EnumWindows` and `GetForegroundWindow` in `qt-capture/src/win.rs`,
  its window lookup; `qt-capture` is a Windows-target dependency of
  `src-tauri` and a Windows-target dev-dependency of `ch-watch` (its
  screenshot tests), by path, and of no other manifest;
- `ch-watch`'s library (outside `tests/`, `examples/` and `#[cfg(test)]`
  items, comments blanked) names no `std::fs`, `File` or `OpenOptions`: no
  copy, crop or OCR line is written anywhere;
- no Rust source watches the clipboard or polls the keys:
  `AddClipboardFormatListener`, `SetClipboardViewer`,
  `GetClipboardSequenceNumber`, `GetAsyncKeyState`, `GetKeyState` and
  `RegisterRawInputDevices` are each an offence (a grab reads the clipboard
  once, on demand);
- no UI source but the tests names `navigator.clipboard`, an input call or
  the network (`fetch(`, `XMLHttpRequest`, `WebSocket`, `EventSource`,
  `sendBeacon`);
- no Rust source names a socket or an HTTP client, only `src-tauri` names
  `ep_net` or depends on `ep-net`, and `ch-app` takes the shared
  `ep-update` and nothing else of it;
- no manifest depends on an HTTP, socket, TLS, WebSocket, input or process
  crate, or on a Tauri plugin but the global shortcut, single instance and
  opener, and `package.json` on no Tauri plugin; the `windows-sys` features
  are the waystone roller's set. The opener (`tauri-plugin-opener`, the
  quest tracker's pin) is the crafting helper's own addition to the
  waystone roller's two plugins, so `wr.ALLOWED_TAURI_PLUGINS` is not
  loosened: it is allowed in the workspace's `[workspace.dependencies]` and
  `src-tauri`'s `[dependencies]` alone;
- the app opens two web addresses, Craft of Exile's and the trade site's
  search: in the Rust sources outside tests (comments blanked) they are the
  only two `http://` or `https://` literals, each once, in `ch-app`, the
  values of `CRAFT_OF_EXILE_URL` and `TRADE_SEARCH_URL`; the glue alone
  names `tauri_plugin_opener`, once, registers it with
  `open_js_links_on_click(false)` (no link handler in the pages) and never
  with `init`, and calls `.opener()` once; the `open_craft_of_exile` and
  `open_trade_search` commands each take nothing but the app state, and no
  command takes a URL or path argument;
- the one request beside GitHub's is the trade site's league list:
  `ep-net`'s `TRADE_LEAGUES_HOST` is `www.pathofexile.com` and its
  `TRADE_LEAGUES_PATH` `/api/trade2/data/leagues`, each defined once
  outside tests (comments and `#[cfg(test)]` items blanked);
  `ep_net::fetch_trade_leagues` is named in the crafting helper by the
  glue's `lib.rs` alone, once, and `fetch_trade_leagues` by no Rust source
  of the quest tracker, the waystone roller or `ep-update`; the
  `refresh_trade_leagues` command, the one that asks for it, takes nothing
  but the app state;
- the pages hold no address and open nothing themselves: no UI source but
  the tests (`//`, `/* */` and `<!-- -->` comments blanked, strings kept)
  names an `http://` or `https://` literal, `window.open`, `location.href`,
  `.assign` or `.replace`, `target="_blank"`, `plugin:opener` or
  `@tauri-apps/plugin-opener`, and the bridge declares
  `open_craft_of_exile`, `open_trade_search` and `refresh_trade_leagues`
  once each, taking `Record<string, never>`;
- the windows' CSP is the pinned string; the capabilities are three, the
  `main` window's (`core:event:default`, `core:window:default`), the
  `settings` window's and the `overlay` window's (`core:event:default`
  each), and none grants an `http:`, `shell:`, `fs:`, `opener:`,
  `process:`, `websocket:`, `upload:` or `updater:` permission or a
  `remote` URL;
- `tauri.conf.json` declares the `main` and `overlay` windows, and the
  overlay is pinned: `overlay.html`, transparent, undecorated, always on
  top, off the taskbar, unfocused and unfocusable, shadowless and hidden.

`Versions`: the workspace, `tauri.conf.json`, `package.json`, `ch-item`'s
literal `version` key and every workspace member's entry in `Cargo.lock`
carry one version; `ch-app` and `src-tauri` take their version and every
external dependency from the workspace (`ch-item` writes its own, std-only),
and so does every other member crate (`ch-data`, `ch-blueprint`,
`ch-engine` and any later one): its `version`, `edition` and `publish` keys
and each external dependency come from the workspace, a local or shared
crate by path; every directory the `crates/*` member glob finds has a `Cargo.toml`; the
executable's name, library name and identifier are the pinned ones. The
guide's `### <version>` release heading is pinned by the docs criterion of
the workorder that wrote the guide, not here.

`LockedToQuestTrackerPins`: every registry package in the app's
`Cargo.lock` but `minisign-verify` and every package in its
`package-lock.json` is one the quest tracker's lock already holds, at the
same version; the local packages are the workspace's members (`ch-item`
included when it exists), the shared `ep-net` and `ep-update`, and the quest
tracker's `qt-screen` and `qt-capture` (M6; `wr.SHARED_CRATES` is not
widened for them); and the
npm and workspace versions equal the quest tracker's. One workspace pin may
be one the quest tracker's workspace lacks: `toml`, for the blueprint files.
Its exact version must be one the quest tracker's `Cargo.lock` already holds
(build metadata aside), so the pin adds no registry package; every other pin
still equals the quest tracker's.

Every matcher runs on a synthetic offender first, so a matcher that sees
nothing cannot pass, and every walk must find files. A failure names the
file and the offending entry.
"""
import json
import re
import sys
import tomllib
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

# The module, never its `TestCase` classes: a class bound here would be run
# a second time from this file.
from tests import test_waystone_roller_guard as wr  # noqa: E402

APP = ROOT / "apps" / "crafting-helper"
CRATES = APP / "crates"
TAURI = APP / "src-tauri"
QT = ROOT / "apps" / "quest-tracker"
GLUE_LIB = TAURI / "src" / "lib.rs"
CH_APP = CRATES / "ch-app"
CH_ITEM = CRATES / "ch-item"
CH_WATCH = CRATES / "ch-watch"
# The one Rust source that reads the window in front and its title, to tell
# whether it is the game's (M6).
GAME_WINDOW_FILE = TAURI / "src" / "game_window.rs"
# The quest tracker's screen crates, taken by path and unchanged (M6).
QT_SCREEN = QT / "crates" / "qt-screen"
QT_CAPTURE = QT / "crates" / "qt-capture"
LINKED_CRATES = {"qt-screen", "qt-capture"}
CAPTURE_CRATE = "qt-capture"
# Where `qt-capture` may be taken: the glue's Windows dependencies and the
# watch crate's Windows dev-dependencies (its screenshot tests), nowhere
# else.
CAPTURE_TABLES = {
    (TAURI / "Cargo.toml", "target.cfg(windows).dependencies"),
    (CH_WATCH / "Cargo.toml", "target.cfg(windows).dev-dependencies"),
}
# `qt-capture`'s window lookup, the two banned names it may hold, in
# `win.rs` alone (checked 2026-10-29: the only two there).
QT_CAPTURE_WIN = QT_CAPTURE / "src" / "win.rs"
QT_CAPTURE_ALLOWED = {"EnumWindows", "GetForegroundWindow"}

CAPABILITIES = {
    "default.json": (["main"], ["core:event:default", "core:window:default"]),
    "settings.json": (["settings"], ["core:event:default"]),
    "overlay.json": (["overlay"], ["core:event:default"]),
}
CONF_WINDOWS = ["main", "overlay"]
# The overlay as `tauri.conf.json` declares it: click-through is set at run
# time (`set_ignore_cursor_events`); here it never takes the focus and
# starts hidden.
OVERLAY_PINS = {
    "url": "overlay.html",
    "transparent": True,
    "decorations": False,
    "alwaysOnTop": True,
    "skipTaskbar": True,
    "focus": False,
    "focusable": False,
    "shadow": False,
    "visible": False,
}

# No paste: what the waystone roller allows in `paste.rs` is an offence in
# every Rust source here. `GetAsyncKeyState` is listed with the watching
# names below, once.
SHOW_WINDOW = "ShowWindow"
NO_INPUT_NAMES = [m for m in wr.PASTE_ONLY_NAMES if m[0] != "GetAsyncKeyState"] + [
    m for m in wr.GAME_NAMES if m[0] != SHOW_WINDOW
]
SHOW_WINDOW_NAMES = [(SHOW_WINDOW, re.compile(r"\bShowWindow\b"))]
# The window in front and its title: allowed in `GAME_WINDOW_FILE` alone.
WINDOW_IN_FRONT_LABELS = {"GetForegroundWindow", "GetWindowText"}
WINDOW_IN_FRONT_NAMES = [m for m in NO_INPUT_NAMES if m[0] in WINDOW_IN_FRONT_LABELS]
SHOW_WINDOW_ASYNC = [("ShowWindowAsync", re.compile(r"\bShowWindowAsync\b"))]
# In the glue, every show command is one of the overlay's two. No word
# character before it, so `WS_EX_...` or `MSW_X` is not one.
SW_NAME = re.compile(r"(?<!\w)SW_\w+")
OVERLAY_SHOW_COMMANDS = {"SW_SHOWNOACTIVATE", "SW_HIDE"}
# No clipboard watching and no key polling: a grab reads once, on demand.
WATCH_NAMES = [
    ("AddClipboardFormatListener", re.compile(r"\bAddClipboardFormatListener\b")),
    ("SetClipboardViewer", re.compile(r"\bSetClipboardViewer\b")),
    ("GetClipboardSequenceNumber", re.compile(r"\bGetClipboardSequenceNumber\b")),
    ("GetAsyncKeyState", re.compile(r"\bGetAsyncKeyState\b")),
    ("GetKeyState", re.compile(r"\bGetKeyState\b")),
    ("RegisterRawInputDevices", re.compile(r"\bRegisterRawInputDevices\b")),
]
UI_CLIPBOARD_NAMES = [("navigator.clipboard", re.compile(r"\bnavigator\s*\??\.\s*clipboard\b"))]
# The app's Tauri plugins: the waystone roller's two and the opener, which
# opens Craft of Exile's one address. The opener is allowed in these
# manifests' tables alone (`wr.manifest_offences` labels).
OPENER_PLUGIN = "tauri-plugin-opener"
ALLOWED_TAURI_PLUGINS = wr.ALLOWED_TAURI_PLUGINS | {OPENER_PLUGIN}
OPENER_TABLES = {
    APP / "Cargo.toml": "workspace.dependencies",
    TAURI / "Cargo.toml": "dependencies",
}
# The two web addresses the app opens, by the `ch-app` constant that holds
# each: Craft of Exile's and the trade site's search (the league and the
# query follow it).
CRAFT_OF_EXILE_URL = "https://beta.craftofexile.com/?game=poe2"
TRADE_SEARCH_URL = "https://www.pathofexile.com/trade2/search/poe2/"
ADDRESS_CONSTS = {"CRAFT_OF_EXILE_URL": CRAFT_OF_EXILE_URL, "TRADE_SEARCH_URL": TRADE_SEARCH_URL}
URL_LITERAL = re.compile(r"\bhttps?://[^\"\s]*", re.IGNORECASE)
# A `&str` constant and its value (no escaped quote in any pinned value).
STR_CONST = re.compile(r"\bconst\s+(\w+)\s*:\s*&\s*(?:'static\s+)?str\s*=\s*\"([^\"]*)\"\s*;")
# The trade site's domain, a web address and not one of the game's files
# (`wr.GAME_NAMES`' `PathOfExile`): lower case, as the addresses write it.
# A Rust test may also name the bare `"pathofexile"`, to check it stays out
# of the log.
TRADE_SITE_DOMAIN = re.compile(r"(?<![\w.])www\.pathofexile\.com\b")
TRADE_SITE_TEST_NEEDLE = re.compile(r"\"pathofexile\"")
# The one request beside GitHub's: `ep-net`'s league list, its host and path
# pinned, asked for through `ep_net::fetch_trade_leagues` by the glue alone.
EP_NET = ROOT / "apps" / "shared" / "crates" / "ep-net"
EP_UPDATE = ROOT / "apps" / "shared" / "crates" / "ep-update"
OTHER_APPS = [ROOT / "apps" / "quest-tracker", ROOT / "apps" / "waystone-roller"]
TRADE_LEAGUES_PINS = {"TRADE_LEAGUES_HOST": "www.pathofexile.com", "TRADE_LEAGUES_PATH": "/api/trade2/data/leagues"}
EP_NET_LEAGUES_CALL = re.compile(r"\bep_net\s*::\s*(?:\{[^}]*\b)?fetch_trade_leagues\b")
LEAGUES_FETCH_NAME = re.compile(r"\bfetch_trade_leagues\b")
OPENER_CRATE = re.compile(r"\btauri_plugin_opener\b")
OPENER_INIT = re.compile(r"\btauri_plugin_opener\s*::\s*(?:\{[^}]*\b)?init\b")
OPENER_CALL = re.compile(r"\.\s*opener\s*\(\s*\)")
NO_LINK_HANDLER = re.compile(r"\bopen_js_links_on_click\s*\(\s*false\s*\)")
ANY_LINK_HANDLER = re.compile(r"\bopen_js_links_on_click\b")
# A Tauri command and its parameter list (no parameter type here holds a
# `)`, so the list ends at the first one).
COMMAND = re.compile(
    r"#\[\s*tauri\s*::\s*command\b[^\]]*\]\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+(\w+)\s*(?:<[^>]*>)?\s*\(([^)]*)\)"
)
# The commands that take the app state alone: the two that open an address
# and the one that asks the trade site for its league list.
STATE_ONLY_COMMANDS = ("open_craft_of_exile", "open_trade_search", "refresh_trade_leagues")
# The app state, the one parameter each of those takes.
STATE_TYPE = re.compile(r"^(?:Shared<'_>|(?:tauri\s*::\s*)?State<'_,\s*Arc<App>>)$")
# A parameter that would carry an address or a file: by its name or its type.
ADDRESS_PARAM_NAME = re.compile(r"url|uri|href|link|address|path|file|dir|location", re.IGNORECASE)
ADDRESS_PARAM_TYPE = re.compile(r"\b(?:PathBuf|Path|Url|OsString|Uri)\b")
# What a page could open a page or a browser with, which no UI source
# outside the tests names (comments blanked): any web address, `window.open`,
# a change of `location`, a link to a new window, or the opener plugin's own
# commands or package. The pages send `open_craft_of_exile` or
# `open_trade_search` instead.
UI_OPEN_NAMES = [
    ("http(s):// literal", re.compile(r"\bhttps?://", re.IGNORECASE)),
    ("window.open", re.compile(r"\bwindow\s*\??\.\s*open\b")),
    ("location.href", re.compile(r"\blocation\s*\??\.\s*href\b")),
    ("location.assign", re.compile(r"\blocation\s*\??\.\s*assign\b")),
    ("location.replace", re.compile(r"\blocation\s*\??\.\s*replace\b")),
    ('target="_blank"', re.compile(r"(?<![\w-])_blank\b")),
    ("plugin:opener", re.compile(r"\bplugin:opener\b")),
    ("@tauri-apps/plugin-opener", re.compile(r"@tauri-apps/plugin-opener\b")),
]
UI_COMMENT_SUFFIXES = {".ts", ".js", ".mjs", ".svelte", ".html", ".css"}
# The bridge declares each command's arguments (`CommandArgs`,
# `SettingsCommandArgs`); each state-only command's must be the empty record.
BRIDGE = APP / "src" / "bridge.ts"
UI_OPEN_COMMAND_ARGS = "Record<string, never>"
# The workspace pins the quest tracker's workspace lacks: `ch-blueprint`'s
# TOML reader, a package already in its lock (through the Tauri build).
PINS_OUTSIDE_THE_QUEST_TRACKER = {"toml"}
# The package keys every member but `ch-item` (std-only, literal keys) takes
# from the workspace.
PACKAGE_KEYS = ("version", "edition", "publish")
# What writing a file takes, which the watch crate's library never names.
FILE_WRITE_NAMES = [
    ("std::fs", re.compile(r"\bstd\s*::\s*(?:fs\b|\{[^}]*\bfs\b)")),
    ("File", re.compile(r"\bFile\b")),
    ("OpenOptions", re.compile(r"\bOpenOptions\b")),
]
CFG_TEST = re.compile(r"#\[\s*cfg\s*\(\s*test\s*\)\s*\]")


def blank_trade_site(code: str, rust_test: bool) -> str:
    """`code` with each lower-case `www.pathofexile.com` (the trade site's
    domain) turned to spaces, and, in a Rust test (`rust_test`), each bare
    `"pathofexile"` string; newlines are kept. Neither is a game file."""
    out = list(code)
    spans = [m.span() for m in TRADE_SITE_DOMAIN.finditer(code)]
    if rust_test:
        spans += [m.span() for m in TRADE_SITE_TEST_NEEDLE.finditer(code)]
    for a, b in spans:
        out[a:b] = " " * (b - a)
    return "".join(out)


def input_offences(
    name: str, text: str, glue: bool, game_window: bool = False, rust_test: bool = False
) -> list[str]:
    """The input, watching, window, process and game names one Rust source
    may not name. `glue` says whether it is `src-tauri/src/lib.rs`: there
    `ShowWindow` is allowed and every `SW_` name is one of the overlay's two
    show commands; anywhere else `ShowWindow` is an offence. `game_window`
    says whether it is `GAME_WINDOW_FILE`: there, and nowhere else,
    `GetForegroundWindow` and `GetWindowText(Length)` are allowed. The trade
    site's domain is no game file (`blank_trade_site`; `rust_test` says
    whether the source is under a crate's `tests/`)."""
    code = blank_trade_site(wr.blank_rust_comments(text), rust_test)
    names = [m for m in NO_INPUT_NAMES if not (game_window and m[0] in WINDOW_IN_FRONT_LABELS)]
    offences = wr.text_offences(name, code, names + WATCH_NAMES + SHOW_WINDOW_ASYNC)
    if not glue:
        return offences + wr.text_offences(name, code, SHOW_WINDOW_NAMES)
    for m in SW_NAME.finditer(code):
        if m.group(0) not in OVERLAY_SHOW_COMMANDS:
            line = code.count("\n", 0, m.start()) + 1
            offences.append(f"{name}:{line}: {m.group(0)}")
    return offences


def overlay_offences(conf: dict) -> list[str]:
    """Where `tauri.conf.json`'s `overlay` window differs from its pins."""
    windows = [w for w in conf.get("app", {}).get("windows", []) if w.get("label") == "overlay"]
    if len(windows) != 1:
        return [f"overlay: {len(windows)} windows labelled overlay"]
    window = windows[0]
    return [
        f"overlay: {key} {window.get(key)!r}, not {want!r}"
        for key, want in OVERLAY_PINS.items()
        if window.get(key) != want
    ]


def workspace_dependency_offences(name: str, manifest: dict) -> list[str]:
    """Each dependency of a member manifest that is neither a path to a
    local or shared crate nor taken with `workspace = true` (no version of
    its own)."""
    offences = []
    for label, table in wr.dependency_tables(manifest):
        for dep, spec in table.items():
            if isinstance(spec, dict) and "version" not in spec and ("path" in spec or spec.get("workspace") is True):
                continue
            offences.append(f"{name}: {label}.{dep} is not taken from the workspace")
    return offences


def package_key_offences(name: str, manifest: dict) -> list[str]:
    """Each of a member manifest's `version`, `edition` and `publish` keys
    not taken from the workspace."""
    package = manifest.get("package", {})
    return [
        f"{name}: package.{key} is not taken from the workspace"
        for key in PACKAGE_KEYS
        if package.get(key) != {"workspace": True}
    ]


def pin_version(spec) -> str | None:
    """The version of a workspace dependency, written as a string or a table."""
    return spec if isinstance(spec, str) else spec.get("version")


def lock_pin_offences(pins: dict, lock: dict, names: set[str]) -> list[str]:
    """Each of `names` among the workspace `pins` that is not pinned exactly
    to a version the parsed `lock` holds for that package, build metadata
    aside (Cargo matches `=1.1.8` to `1.1.8+spec-1.1.0`)."""
    offences = []
    for name in sorted(names & set(pins)):
        version = str(pin_version(pins[name]))
        if not version.startswith("="):
            offences.append(f"{name}: {version!r} is not pinned exactly")
            continue
        held = {p["version"].split("+")[0] for p in lock.get("package", []) if p["name"] == name}
        if version[1:] not in held:
            offences.append(f"{name}: {version[1:]} is not in the quest tracker's Cargo.lock")
    return offences


def member_dirs() -> list[Path]:
    """Every directory the workspace's `crates/*` glob finds, and `src-tauri`."""
    return sorted(d for d in CRATES.iterdir() if d.is_dir()) + [TAURI]


def local_packages(lock: dict) -> dict[str, str]:
    """name -> version of every local (sourceless) package in a Cargo.lock."""
    return {p["name"]: p["version"] for p in lock.get("package", []) if "source" not in p}


def linked_crate_offences(name: str, text: str, allowed: set[str]) -> list[str]:
    """The input, watching, show, message, focus, hook, process and game-file
    names one Rust source of a linked quest tracker crate names (comments
    blanked), but those in `allowed`."""
    matchers = NO_INPUT_NAMES + WATCH_NAMES + SHOW_WINDOW_ASYNC + SHOW_WINDOW_NAMES
    return [o for o in wr.rust_offences(name, text, matchers) if o.rsplit(": ", 1)[1] not in allowed]


def capture_crate_uses(path: Path, manifest: dict) -> set[tuple[Path, str]]:
    """`(manifest, table label)` for each table of `manifest` that takes the
    capture crate, and `(manifest, "<label> (path)")` when it is not taken by
    path from the quest tracker's crate."""
    uses = set()
    for label, table in wr.dependency_tables(manifest):
        spec = table.get(CAPTURE_CRATE)
        if spec is None:
            continue
        uses.add((path, label))
        target = spec.get("path") if isinstance(spec, dict) else None
        if target is None or (path.parent / target).resolve() != QT_CAPTURE.resolve():
            uses.add((path, f"{label} (path)"))
    return uses


def blank_cfg_test(code: str) -> str:
    """`code` (comments already blanked) with each `#[cfg(test)]` item turned
    to spaces: the attribute and the item after it, to its `;` or its
    matching `}`, whichever ends it first. Strings are skipped over when
    counting braces. Newlines are kept."""
    out = list(code)
    for m in CFG_TEST.finditer(code):
        i, depth, n = m.end(), 0, len(code)
        while i < n:
            c = code[i]
            if c == '"':
                i += 1
                while i < n and code[i] != '"':
                    i += 2 if code[i] == "\\" else 1
            elif c == "{":
                depth += 1
            elif c == "}":
                depth -= 1
                if depth == 0:
                    break
            elif c == ";" and depth == 0:
                break
            i += 1
        for k in range(m.start(), min(i + 1, n)):
            if out[k] != "\n":
                out[k] = " "
    return "".join(out)


def file_write_offences(name: str, text: str) -> list[str]:
    """The file names (`std::fs`, `File`, `OpenOptions`) one library source
    names outside comments and `#[cfg(test)]` items."""
    return wr.text_offences(name, blank_cfg_test(wr.blank_rust_comments(text)), FILE_WRITE_NAMES)


def app_manifest_offences(name: str, manifest: dict, opener_table: str | None) -> list[str]:
    """`wr.manifest_offences` for one of this app's manifests, the opener
    plugin allowed in the table labelled `opener_table` alone (`None`:
    nowhere)."""
    allowed = f"{name}: {opener_table}.{OPENER_PLUGIN} (a Tauri plugin)" if opener_table else None
    return [o for o in wr.manifest_offences(name, manifest) if o != allowed]


def blank_ui_comments(text: str) -> str:
    """`text` (TypeScript, JavaScript, a Svelte component, an HTML page or a
    style sheet) with its `//`, `/* */` and `<!-- -->` comments turned to
    spaces; newlines are kept. Strings (`'`, `"`, and template literals) are
    kept, so a name in one is still an offence, and are skipped over, so a
    `//` inside one is not a comment; a `'` or `"` string ends at its line's
    end, as an apostrophe in a page's text does. A `//` right after a `:` is
    an address (`https://`), never a comment, so markup text cannot hide
    one."""
    out = list(text)
    i, n = 0, len(text)

    def blank(a: int, b: int) -> None:
        for k in range(a, b):
            if out[k] != "\n":
                out[k] = " "

    def skip(start: int, close: str, multiline: bool) -> int:
        j = start + 1
        while j < n and text[j] != close and (multiline or text[j] != "\n"):
            j += 2 if text[j] == "\\" else 1
        return j + 1

    while i < n:
        if text.startswith("//", i) and (i == 0 or text[i - 1] != ":"):
            end = text.find("\n", i)
            end = n if end < 0 else end
            blank(i, end)
            i = end
        elif text.startswith("/*", i) or text.startswith("<!--", i):
            close = "*/" if text[i] == "/" else "-->"
            end = text.find(close, i + 2)
            end = n if end < 0 else end + len(close)
            blank(i, end)
            i = end
        elif text[i] in "'\"":
            i = skip(i, text[i], False)
        elif text[i] == "`":
            i = skip(i, "`", True)
        else:
            i += 1
    return "".join(out)


def ui_open_offences(name: str, text: str) -> list[str]:
    return wr.text_offences(name, blank_ui_comments(text), UI_OPEN_NAMES)


def ui_command_offences(name: str, text: str, command: str) -> list[str]:
    """The bridge declares `command` once, taking the empty record."""
    found = re.findall(rf"\b{command}\s*:\s*([^;]*);", blank_ui_comments(text))
    if len(found) != 1:
        return [f"{name}: {command} declared {len(found)} times"]
    args = re.sub(r"\s+", " ", found[0]).strip()
    return [] if args == UI_OPEN_COMMAND_ARGS else [f"{name}: {command} takes {args}, not {UI_OPEN_COMMAND_ARGS}"]


def is_rust_test(path: Path) -> bool:
    """A Rust source under a crate's `tests/` directory."""
    return "tests" in path.relative_to(APP).parts


def url_literals(name: str, text: str) -> list[tuple[str, str]]:
    """`(name:line, literal)` for each `http://` or `https://` in `text`,
    comments blanked (strings kept)."""
    code = wr.blank_rust_comments(text)
    return [(f"{name}:{code.count(chr(10), 0, m.start()) + 1}", m.group(0)) for m in URL_LITERAL.finditer(code)]


def commands(text: str) -> dict[str, list[tuple[str, str]]]:
    """Each `#[tauri::command]` function in `text` (comments blanked) and its
    parameters as `(name, type)`."""
    found = {}
    for m in COMMAND.finditer(wr.blank_rust_comments(text)):
        params = []
        for part in m.group(2).split(","):
            if part.strip():
                pname, _, ptype = part.partition(":")
                params.append((pname.strip().removeprefix("mut ").strip(), " ".join(ptype.split())))
        found[m.group(1)] = params
    return found


def address_parameter_offences(name: str, found: dict[str, list[tuple[str, str]]]) -> list[str]:
    """Each command parameter that could carry a web address or a file: by
    its name or its type."""
    return [
        f"{name}: {command}({pname}: {ptype})"
        for command, params in found.items()
        for pname, ptype in params
        if ADDRESS_PARAM_NAME.search(pname) or ADDRESS_PARAM_TYPE.search(ptype)
    ]


def state_only_offences(found: dict[str, list[tuple[str, str]]], command: str) -> list[str]:
    """Where `command` is missing or takes more than the app state."""
    params = found.get(command)
    if params is None:
        return [f"{command}: no such command"]
    if len(params) != 1 or not STATE_TYPE.match(params[0][1]):
        return [f"{command}: takes {params}, not the app state alone"]
    return []


def address_const_offences(name: str, text: str, literals: list[str]) -> list[str]:
    """Each address literal in one Rust source (comments blanked) that is not
    the value of the `ch-app` constant pinned to it (`ADDRESS_CONSTS`)."""
    consts = {value: const for const, value in STR_CONST.findall(wr.blank_rust_comments(text))}
    pinned = {value: const for const, value in ADDRESS_CONSTS.items()}
    offences = []
    for literal in literals:
        want = pinned.get(literal)
        if want is None or consts.get(literal) != want:
            offences.append(f"{name}: {literal} is not {want or 'a pinned address'}")
    return offences


def str_consts_outside_tests(text: str) -> list[tuple[str, str]]:
    """`(name, value)` of each `&str` constant one Rust source defines,
    comments and `#[cfg(test)]` items blanked."""
    return STR_CONST.findall(blank_cfg_test(wr.blank_rust_comments(text)))


def league_pin_offences(sources: dict[str, str]) -> list[str]:
    """Where `ep-net`'s sources (`name -> text`) do not define each of
    `TRADE_LEAGUES_PINS` once, outside tests, with its pinned value."""
    defined: dict[str, list[str]] = {}
    for text in sources.values():
        for const, value in str_consts_outside_tests(text):
            defined.setdefault(const, []).append(value)
    offences = []
    for const, want in TRADE_LEAGUES_PINS.items():
        values = defined.get(const, [])
        if len(values) != 1:
            offences.append(f"{const}: defined {len(values)} times outside tests")
        elif values[0] != want:
            offences.append(f"{const}: {values[0]!r}, not {want!r}")
    return offences


def league_fetch_names(name: str, text: str, matcher: re.Pattern) -> list[str]:
    """`name:line` of each `matcher` hit in one Rust source, comments
    blanked: `EP_NET_LEAGUES_CALL` in the crafting helper (where ch-app's
    `Shell` has a method of the same name), `LEAGUES_FETCH_NAME` in another
    app or shared crate."""
    code = wr.blank_rust_comments(text)
    return [f"{name}:{code.count(chr(10), 0, m.start()) + 1}" for m in matcher.finditer(code)]


class NoInputNoWatchingNoNetworkNoGame(unittest.TestCase):
    def rust_sources(self) -> list[Path]:
        files = wr.walk(CRATES, {".rs"}) + wr.walk(TAURI, {".rs"})
        self.assertGreater(len(files), 5, "the walk found no Rust sources")
        self.assertIn(GLUE_LIB, files)
        self.assertIn(CH_APP / "src" / "lib.rs", files)
        return files

    def test_no_rust_source_sends_input_watches_or_reaches_the_game(self):
        offender = (
            "// SendInput and AddClipboardFormatListener in a comment are fine\n"
            "let s = \"http://example\"; unsafe { SendInput(1, p, 4) };\n"
            "/* ReadProcessMemory */ let h = OpenProcess(0, 0, pid);\n"
            "let p = \"C:/Games/PathOfExile.exe\";\n"
            "unsafe { AddClipboardFormatListener(hwnd) };\n"
            "let w = GetForegroundWindow();\n"
        )
        self.assertEqual(
            sorted(input_offences("offender.rs", offender, glue=True)),
            [
                "offender.rs:2: SendInput",
                "offender.rs:3: OpenProcess",
                "offender.rs:4: PathOfExile",
                "offender.rs:5: AddClipboardFormatListener",
                "offender.rs:6: GetForegroundWindow",
            ],
        )
        self.assertEqual(input_offences("clean.rs", "// SendInput OpenProcess GetKeyState\nfn f() {}\n", glue=False), [])
        # The trade site's domain is no game file; the game's files still are,
        # in a test too, and a mixed-case domain or the bare name outside a
        # test is one.
        site = (
            f'pub const TRADE_SEARCH_URL: &str = "{TRADE_SEARCH_URL}";\n'
            'let needle = "pathofexile";\n'
            'let a = "https://www.PathOfExile.com/trade2";\n'
            'let b = "https://www.pathofexile.com/../PathOfExile.exe";\n'
            'let c = "C:/games/pathofexile.exe";\n'
        )
        self.assertEqual(
            input_offences("t.rs", site, glue=False, rust_test=True),
            ["t.rs:3: PathOfExile", "t.rs:4: PathOfExile", "t.rs:5: PathOfExile"],
        )
        self.assertEqual(
            input_offences("s.rs", site, glue=False),
            ["s.rs:2: PathOfExile", "s.rs:3: PathOfExile", "s.rs:4: PathOfExile", "s.rs:5: PathOfExile"],
        )
        self.assertEqual(input_offences("s.rs", site.splitlines()[0], glue=False), [])

        offences = []
        sources = self.rust_sources()
        for path in sources:
            offences += input_offences(
                wr.rel(path),
                wr.read(path),
                glue=path == GLUE_LIB,
                game_window=path == GAME_WINDOW_FILE,
                rust_test=is_rust_test(path),
            )
        self.assertEqual(offences, [])
        # Positive control: the domain is there to be allowed (else the
        # allowance is stale, or the walk blind).
        self.assertTrue(any(TRADE_SITE_DOMAIN.search(wr.read(p)) for p in sources if not is_rust_test(p)))

    def test_only_the_game_window_file_reads_the_window_in_front(self):
        reader = (
            "// SetForegroundWindow in a comment is fine\n"
            "let h = unsafe { GetForegroundWindow() };\n"
            "let n = unsafe { GetWindowTextLengthW(h) };\n"
            "let got = unsafe { GetWindowTextW(h, buf.as_mut_ptr(), len) };\n"
        )
        self.assertEqual(input_offences("game_window.rs", reader, glue=False, game_window=True), [])
        # Anywhere else each is an offence, the glue's lib.rs included.
        want = ["{0}:2: GetForegroundWindow", "{0}:3: GetWindowText", "{0}:4: GetWindowText"]
        self.assertEqual(sorted(input_offences("app.rs", reader, glue=False)), [w.format("app.rs") for w in want])
        self.assertEqual(sorted(input_offences("lib.rs", reader, glue=True)), [w.format("lib.rs") for w in want])
        # The window file names no other banned name: focus, the show call,
        # input, another process.
        self.assertEqual(
            input_offences(
                "game_window.rs",
                reader + "unsafe { SetForegroundWindow(h) };\nShowWindow(h, SW_HIDE);\nSendInput(1, p, 4);\n"
                "OpenProcess(0, 0, pid);\n",
                glue=False,
                game_window=True,
            ),
            [
                "game_window.rs:7: SendInput",
                "game_window.rs:5: SetForegroundWindow",
                "game_window.rs:8: OpenProcess",
                "game_window.rs:6: ShowWindow",
            ],
        )

        # The real tree: the file exists and reads the window in front (a
        # positive control), and no other Rust source names either call.
        files = self.rust_sources()
        self.assertIn(GAME_WINDOW_FILE, files)
        found = {}
        for path in files:
            hits = wr.rust_offences(wr.rel(path), wr.read(path), WINDOW_IN_FRONT_NAMES)
            if hits:
                found[path] = {h.rsplit(": ", 1)[1] for h in hits}
        self.assertEqual(found, {GAME_WINDOW_FILE: WINDOW_IN_FRONT_LABELS})
        self.assertEqual(
            input_offences(wr.rel(GAME_WINDOW_FILE), wr.read(GAME_WINDOW_FILE), glue=False, game_window=True), []
        )

    def test_linked_quest_tracker_crates_send_no_input(self):
        offender = (
            "// SendInput in a comment is fine\n"
            "let h = unsafe { GetForegroundWindow() };\n"
            "unsafe { EnumWindows(Some(visit), l) };\n"
            "unsafe { SetForegroundWindow(h); PostMessageW(h, 0, 0, 0) };\n"
            "let p = \"PathOfExile.exe\";\n"
        )
        self.assertEqual(
            linked_crate_offences("win.rs", offender, QT_CAPTURE_ALLOWED),
            ["win.rs:4: PostMessage", "win.rs:4: SetForegroundWindow", "win.rs:5: PathOfExile"],
        )
        self.assertEqual(
            linked_crate_offences("o.rs", offender, set()),
            [
                "o.rs:2: GetForegroundWindow",
                "o.rs:4: PostMessage",
                "o.rs:4: SetForegroundWindow",
                "o.rs:3: EnumWindows",
                "o.rs:5: PathOfExile",
            ],
        )

        screen = wr.walk(QT_SCREEN, {".rs"})
        capture = wr.walk(QT_CAPTURE, {".rs"})
        self.assertGreater(len(screen), 5, "the walk found no qt-screen sources")
        self.assertIn(QT_CAPTURE_WIN, capture)
        offences = []
        allowed_seen = set()
        for path in screen + capture:
            text = wr.read(path)
            allowed = QT_CAPTURE_ALLOWED if path == QT_CAPTURE_WIN else set()
            offences += linked_crate_offences(wr.rel(path), text, allowed)
            if allowed:
                hits = linked_crate_offences(wr.rel(path), text, set())
                allowed_seen |= {h.rsplit(": ", 1)[1] for h in hits}
        self.assertEqual(offences, [])
        # Both allowances are used (else they are stale, or the walk blind).
        self.assertEqual(allowed_seen, QT_CAPTURE_ALLOWED)

    def test_the_watch_crate_writes_no_file(self):
        offender = (
            "// std::fs::write in a comment is fine\n"
            "use std::fs;\n"
            "use std::{io, fs::File};\n"
            "let o = OpenOptions::new();\n"
            "#[cfg(test)]\n"
            "mod tests {\n"
            "    fn f() { let s = \"}\"; std::fs::read(\"x\"); }\n"
            "}\n"
            "#[cfg(test)]\nuse std::fs::File;\n"
            "fn g() -> Profile { Profile }\n"
        )
        self.assertEqual(
            file_write_offences("o.rs", offender),
            ["o.rs:2: std::fs", "o.rs:3: std::fs", "o.rs:3: File", "o.rs:4: OpenOptions"],
        )
        self.assertEqual(file_write_offences("c.rs", "#[cfg(test)]\nmod t { use std::fs::File; }\n"), [])

        files = [
            p
            for p in wr.walk(CH_WATCH, {".rs"})
            if not {"tests", "examples"} & set(p.relative_to(CH_WATCH).parts)
        ]
        self.assertIn(CH_WATCH / "src" / "lib.rs", files)
        self.assertIn(CH_WATCH / "src" / "reader.rs", files)
        offences = []
        for path in files:
            offences += file_write_offences(wr.rel(path), wr.read(path))
        self.assertEqual(offences, [])

    def test_only_the_glue_and_the_watch_tests_take_the_capture_crate(self):
        here = TAURI / "Cargo.toml"
        offender = tomllib.loads(
            '[dependencies]\nqt-capture = { path = "../../quest-tracker/crates/qt-capture" }\n'
            "[target.'cfg(windows)'.dependencies]\nqt-capture = { path = \"../../elsewhere/qt-capture\" }\n"
            "[dev-dependencies]\nqt-capture = \"0.1\"\n"
        )
        self.assertEqual(
            capture_crate_uses(here, offender),
            {
                (here, "dependencies"),
                (here, "target.cfg(windows).dependencies"),
                (here, "target.cfg(windows).dependencies (path)"),
                (here, "dev-dependencies"),
                (here, "dev-dependencies (path)"),
            },
        )

        manifests = wr.walk(APP, names={"Cargo.toml"})
        self.assertIn(CH_WATCH / "Cargo.toml", manifests)
        uses = set()
        for m in manifests:
            uses |= capture_crate_uses(m, tomllib.loads(wr.read(m)))
        self.assertEqual(
            sorted((wr.rel(p), label) for p, label in uses),
            sorted((wr.rel(p), label) for p, label in CAPTURE_TABLES),
        )

    def test_every_input_and_watching_matcher_sees_its_name(self):
        matchers = NO_INPUT_NAMES + WATCH_NAMES + SHOW_WINDOW_ASYNC + SHOW_WINDOW_NAMES
        offender = "".join(f"let x = {label}(0);\n" for label, _ in matchers)
        self.assertEqual(
            wr.rust_offences("o.rs", offender, matchers),
            [f"o.rs:{line}: {label}" for line, (label, _) in enumerate(matchers, 1)],
        )
        # Every name the waystone roller's guard bans or allows in paste.rs
        # alone is banned here.
        labels = {label for label, _ in matchers}
        self.assertLessEqual({label for label, _ in wr.PASTE_ONLY_NAMES + wr.GAME_NAMES}, labels)
        # `GetKeyState` is not a substring hit of `GetAsyncKeyState`, nor
        # `ShowWindow` of `ShowWindowAsync`: one offence each.
        self.assertEqual(
            wr.rust_offences("o.rs", "GetAsyncKeyState(k); ShowWindowAsync(h, 0);\n", matchers),
            ["o.rs:1: GetAsyncKeyState", "o.rs:1: ShowWindowAsync"],
        )

    def test_only_the_glue_shows_the_overlay_without_activating(self):
        clean = (
            "use windows_sys::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE, SW_SHOWNOACTIVATE};\n"
            "// SW_SHOW in a comment is fine\n"
            "unsafe { ShowWindow(h, if shown { SW_SHOWNOACTIVATE } else { SW_HIDE }) };\n"
        )
        self.assertEqual(input_offences("lib.rs", clean, glue=True), [])
        # Anywhere else it is an offence, each `ShowWindow` named.
        self.assertEqual(
            input_offences("app.rs", clean, glue=False),
            ["app.rs:1: ShowWindow", "app.rs:3: ShowWindow"],
        )
        # In the glue, a show command that activates, or the async call.
        self.assertEqual(
            input_offences("lib.rs", clean + "unsafe { ShowWindow(h, SW_SHOW) };\n", glue=True),
            ["lib.rs:4: SW_SHOW"],
        )
        self.assertEqual(
            input_offences("lib.rs", clean + "unsafe { ShowWindowAsync(h, SW_HIDE) };\n", glue=True),
            ["lib.rs:4: ShowWindowAsync"],
        )
        self.assertEqual(
            input_offences("lib.rs", clean + "unsafe { SetForegroundWindow(h) };\n", glue=True),
            ["lib.rs:4: SetForegroundWindow"],
        )
        # Positive control: the real glue shows the overlay this way.
        self.assertTrue(wr.text_offences("lib.rs", wr.blank_rust_comments(wr.read(GLUE_LIB)), SHOW_WINDOW_NAMES))

    def test_no_rust_source_names_the_network(self):
        self.assertEqual(
            wr.rust_offences("o.rs", "use std::net::TcpStream;\n", wr.RUST_NETWORK_NAMES),
            ["o.rs:1: std::net", "o.rs:1: TcpStream"],
        )
        offences = []
        for path in self.rust_sources():
            offences += wr.rust_offences(wr.rel(path), wr.read(path), wr.RUST_NETWORK_NAMES)
        self.assertEqual(offences, [])

    def test_only_the_glue_reaches_ep_net(self):
        matcher = [("ep_net", wr.NETWORK_CRATE_NAME)]
        self.assertEqual(
            wr.rust_offences("o.rs", "// ep_net in a comment\nlet r = ep_net::fetch_release_list(p);\n", matcher),
            ["o.rs:2: ep_net"],
        )
        self.assertEqual(
            wr.shared_dependency_offences("o", {"dependencies": {"ep-net": {"path": "../../shared/crates/ep-net"}}}, glue=False),
            ["o: dependencies.ep-net outside the glue"],
        )

        offences = []
        named = []
        for path in self.rust_sources():
            hits = wr.rust_offences(wr.rel(path), wr.read(path), matcher)
            if path.is_relative_to(TAURI):
                named += hits
            else:
                offences += hits
        self.assertEqual(offences, [])
        self.assertTrue(named, "the glue never names ep_net: the walk or the matcher is blind")

        offences = []
        for m in wr.walk(APP, names={"Cargo.toml"}):
            offences += wr.shared_dependency_offences(wr.rel(m), tomllib.loads(wr.read(m)), glue=m.parent == TAURI)
        self.assertEqual(offences, [])
        glue = tomllib.loads(wr.read(TAURI / "Cargo.toml"))["dependencies"]
        self.assertIn(wr.NETWORK_CRATE, glue)
        app = tomllib.loads(wr.read(CH_APP / "Cargo.toml"))["dependencies"]
        self.assertEqual(sorted(set(app) & wr.SHARED_CRATES), ["ep-update"])

    def test_no_ui_source_calls_the_network_or_reads_the_clipboard(self):
        names = wr.UI_NETWORK_NAMES + wr.UI_INPUT_NAMES + UI_CLIPBOARD_NAMES
        self.assertEqual(
            wr.text_offences(
                "o.ts",
                "await fetch('https://x');\nconst t = await navigator.clipboard.readText();\nnavigator?.clipboard;\n",
                names,
            ),
            ["o.ts:1: fetch(", "o.ts:2: navigator.clipboard", "o.ts:3: navigator.clipboard"],
        )
        self.assertEqual(wr.text_offences("c.ts", "const c = event.clipboardData;\n", names), [])
        files = [p for p in wr.walk(APP / "src", wr.UI_SUFFIXES) if not wr.is_ui_test(p)]
        # Every root page: the main window's, the Settings window's and the
        # overlay's.
        pages = sorted(APP.glob("*.html"))
        self.assertEqual([p.name for p in pages], ["index.html", "overlay.html", "settings.html"])
        files += pages
        self.assertGreater(len(files), 5, "the walk found no UI sources")
        offences = []
        for path in files:
            offences += wr.text_offences(wr.rel(path), wr.read(path), names)
        self.assertEqual(offences, [])

    def test_ui_has_no_free_form_open(self):
        offender = (
            "// http://127.0.0.1:1422/?state=x in a comment is fine\n"
            "const a = 'https://beta.craftofexile.com/?game=poe2';\n"
            "window.open(a);\n"
            "location.href = a; window.location.assign(a); location?.replace(a);\n"
            '<a href="#" target="_blank">x</a>\n'
            "await invoke('plugin:opener|open_url', { url });\n"
            "import { openUrl } from '@tauri-apps/plugin-opener';\n"
            "<!-- <a href=\"http://in.an.html.comment\"> -->\n"
            "/* window.open(b) in a block comment */\n"
            "<p>Can't open; see HTTP://markup.text</p>\n"
            "const s = `${x}//not a comment https://in.a.template`;\n"
        )
        self.assertEqual(
            ui_open_offences("o.svelte", offender),
            [
                "o.svelte:2: http(s):// literal",
                "o.svelte:10: http(s):// literal",
                "o.svelte:11: http(s):// literal",
                "o.svelte:3: window.open",
                "o.svelte:4: location.href",
                "o.svelte:4: location.assign",
                "o.svelte:4: location.replace",
                'o.svelte:5: target="_blank"',
                "o.svelte:6: plugin:opener",
                "o.svelte:7: @tauri-apps/plugin-opener",
            ],
        )
        # Negative controls: what the pages do name is no offence.
        self.assertEqual(
            ui_open_offences("c.ts", "const q = window.location.search;\nconst t = `a ${b}` // https://x\n"),
            [],
        )
        self.assertEqual(blank_ui_comments("a // b\nc"), "a     \nc")
        self.assertEqual(
            ui_command_offences("b.ts", "interface C {\n  open_craft_of_exile: { url: string };\n}\n", "open_craft_of_exile"),
            ["b.ts: open_craft_of_exile takes { url: string }, not Record<string, never>"],
        )
        self.assertEqual(
            ui_command_offences("b.ts", "// open_craft_of_exile: Record<string, never>;\n", "open_craft_of_exile"),
            ["b.ts: open_craft_of_exile declared 0 times"],
        )
        self.assertEqual(
            ui_command_offences("b.ts", "  open_craft_of_exile: Record<string,  never>;\n", "open_craft_of_exile"), []
        )
        # The trade search and the league refresh: a field added, a second
        # declaration; `set_trade_search` is not `open_trade_search`.
        bridge = (
            "  set_trade_search: { search: TradeSelection };\n"
            "  open_trade_search: { league: string };\n"
            "  refresh_trade_leagues: Record<string, never>;\n"
            "  refresh_trade_leagues: Record<string, never>;\n"
        )
        self.assertEqual(
            [o for c in STATE_ONLY_COMMANDS[1:] for o in ui_command_offences("b.ts", bridge, c)],
            [
                "b.ts: open_trade_search takes { league: string }, not Record<string, never>",
                "b.ts: refresh_trade_leagues declared 2 times",
            ],
        )

        files = [p for p in wr.walk(APP / "src", UI_COMMENT_SUFFIXES) if not wr.is_ui_test(p)]
        pages = sorted(APP.glob("*.html"))
        self.assertEqual([p.name for p in pages], ["index.html", "overlay.html", "settings.html"])
        files += pages
        self.assertIn(BRIDGE, files)
        self.assertIn(APP / "src" / "App.svelte", files)
        self.assertGreater(len(files), 10, "the walk found no UI sources")
        offences = []
        for path in files:
            offences += ui_open_offences(wr.rel(path), wr.read(path))
        self.assertEqual(offences, [])
        bridge = wr.read(BRIDGE)
        self.assertEqual([o for c in STATE_ONLY_COMMANDS for o in ui_command_offences(wr.rel(BRIDGE), bridge, c)], [])

    def test_no_manifest_depends_on_network_input_or_process_crates(self):
        offender = {
            "dependencies": {"reqwest": "0.12", "tauri-plugin-clipboard-manager": "2"},
            "target": {"cfg(windows)": {"dependencies": {
                "windows-sys": {"version": "0.61", "features": ["Win32_Foundation", "Win32_UI_Input"]},
            }}},
        }
        self.assertEqual(
            sorted(wr.manifest_offences("o", offender)),
            sorted([
                "o: dependencies.reqwest",
                "o: dependencies.tauri-plugin-clipboard-manager (a Tauri plugin)",
                "o: target.cfg(windows).dependencies.windows-sys feature Win32_UI_Input",
            ]),
        )
        manifests = wr.walk(APP, names={"Cargo.toml"})
        self.assertEqual(
            set(manifests),
            {APP / "Cargo.toml"} | {d / "Cargo.toml" for d in member_dirs()},
        )
        self.assertIn(CH_APP / "Cargo.toml", manifests)
        offences = []
        for m in manifests:
            offences += app_manifest_offences(wr.rel(m), tomllib.loads(wr.read(m)), OPENER_TABLES.get(m))
        self.assertEqual(offences, [])
        workspace = tomllib.loads(wr.read(APP / "Cargo.toml"))
        self.assertEqual(
            set(workspace["workspace"]["dependencies"]["windows-sys"]["features"]),
            wr.WINDOWS_SYS_FEATURES,
        )
        self.assertEqual(
            {d for d in workspace["workspace"]["dependencies"] if d.startswith("tauri-plugin-")},
            ALLOWED_TAURI_PLUGINS,
        )

    def test_only_the_glue_takes_the_opener_plugin(self):
        offender = tomllib.loads(
            '[dependencies]\ntauri-plugin-opener = { workspace = true }\n'
            "[target.'cfg(windows)'.dependencies]\ntauri-plugin-opener = { workspace = true }\n"
            'tauri-plugin-shell = "2"\n'
        )
        # Nowhere allowed: every opener is an offence.
        self.assertEqual(
            app_manifest_offences("o", offender, None),
            [
                "o: dependencies.tauri-plugin-opener (a Tauri plugin)",
                "o: target.cfg(windows).dependencies.tauri-plugin-opener (a Tauri plugin)",
                "o: target.cfg(windows).dependencies.tauri-plugin-shell (a Tauri plugin)",
            ],
        )
        # Allowed in one table: that one alone, and no other plugin.
        self.assertEqual(
            app_manifest_offences("o", offender, "dependencies"),
            [
                "o: target.cfg(windows).dependencies.tauri-plugin-opener (a Tauri plugin)",
                "o: target.cfg(windows).dependencies.tauri-plugin-shell (a Tauri plugin)",
            ],
        )
        # The waystone roller's set stays its own two.
        self.assertNotIn(OPENER_PLUGIN, wr.ALLOWED_TAURI_PLUGINS)

        # The workspace pins it and the glue takes it; no other member does.
        takers = []
        for m in wr.walk(APP, names={"Cargo.toml"}):
            manifest = tomllib.loads(wr.read(m))
            for label, table in wr.dependency_tables(manifest):
                if OPENER_PLUGIN in table:
                    takers.append((m, label))
        self.assertEqual(sorted(takers), sorted(OPENER_TABLES.items()))
        workspace = tomllib.loads(wr.read(APP / "Cargo.toml"))["workspace"]["dependencies"]
        quest_tracker = tomllib.loads(wr.read(QT / "Cargo.toml"))["workspace"]["dependencies"]
        self.assertEqual(pin_version(workspace[OPENER_PLUGIN]), pin_version(quest_tracker[OPENER_PLUGIN]))
        glue = tomllib.loads(wr.read(TAURI / "Cargo.toml"))["dependencies"]
        self.assertEqual(glue[OPENER_PLUGIN], {"workspace": True})

    def test_only_the_two_pinned_addresses_are_opened(self):
        offender = (
            "// https://in.a.comment is fine\n"
            f'pub const A: &str = "{CRAFT_OF_EXILE_URL}";\n'
            'let b = "http://example.com/x";\n'
            "use tauri_plugin_opener::OpenerExt;\n"
            ".plugin(tauri_plugin_opener::init())\n"
            "app.opener().open_url(u, None::<&str>);\n"
            "#[tauri::command(async)]\nfn open_page(app: Shared<'_>, url: String) -> View { todo!() }\n"
            "#[tauri::command]\npub fn reveal(app: Shared<'_>, at: PathBuf) {}\n"
            "#[tauri::command]\nfn open_craft_of_exile(app: Shared<'_>, page: String) -> View { todo!() }\n"
            "#[tauri::command(async)]\nfn open_trade_search(app: Shared<'_>, league: String) -> View { todo!() }\n"
            "#[tauri::command(async)]\nfn refresh_trade_leagues(app: Shared<'_>, host: String) -> SettingsView { todo!() }\n"
        )
        self.assertEqual(
            url_literals("o.rs", offender),
            [("o.rs:2", CRAFT_OF_EXILE_URL), ("o.rs:3", "http://example.com/x")],
        )
        # Each address is the value of the constant pinned to it; a literal
        # under another name, or any other address, is an offence.
        self.assertEqual(
            address_const_offences("o.rs", offender, [CRAFT_OF_EXILE_URL, "http://example.com/x"]),
            ["o.rs: https://beta.craftofexile.com/?game=poe2 is not CRAFT_OF_EXILE_URL",
             "o.rs: http://example.com/x is not a pinned address"],
        )
        self.assertEqual(
            address_const_offences(
                "t.rs", f'pub const TRADE_SEARCH_URL: &str = "{TRADE_SEARCH_URL}";\n', [TRADE_SEARCH_URL]
            ),
            [],
        )
        self.assertEqual(len(OPENER_CRATE.findall(wr.blank_rust_comments(offender))), 2)
        self.assertTrue(OPENER_INIT.search(offender))
        self.assertTrue(OPENER_INIT.search("use tauri_plugin_opener::{init, OpenerExt};"))
        self.assertFalse(OPENER_INIT.search("use tauri_plugin_opener::{Builder as OpenerPlugin, OpenerExt};"))
        self.assertFalse(NO_LINK_HANDLER.search(offender))
        self.assertTrue(NO_LINK_HANDLER.search(".open_js_links_on_click( false )"))
        self.assertEqual(len(OPENER_CALL.findall(offender)), 1)
        found = commands(offender)
        self.assertEqual(
            found,
            {
                "open_page": [("app", "Shared<'_>"), ("url", "String")],
                "reveal": [("app", "Shared<'_>"), ("at", "PathBuf")],
                "open_craft_of_exile": [("app", "Shared<'_>"), ("page", "String")],
                "open_trade_search": [("app", "Shared<'_>"), ("league", "String")],
                "refresh_trade_leagues": [("app", "Shared<'_>"), ("host", "String")],
            },
        )
        self.assertEqual(
            address_parameter_offences("o.rs", found),
            ["o.rs: open_page(url: String)", "o.rs: reveal(at: PathBuf)"],
        )
        # A parameter added to any state-only command is caught, though no
        # name or type of it reads as an address.
        self.assertEqual(
            [o for c in STATE_ONLY_COMMANDS for o in state_only_offences(found, c)],
            [
                "open_craft_of_exile: takes [('app', \"Shared<'_>\"), ('page', 'String')], not the app state alone",
                "open_trade_search: takes [('app', \"Shared<'_>\"), ('league', 'String')], not the app state alone",
                "refresh_trade_leagues: takes [('app', \"Shared<'_>\"), ('host', 'String')], not the app state alone",
            ],
        )
        self.assertEqual(state_only_offences({}, "open_trade_search"), ["open_trade_search: no such command"])
        self.assertEqual(state_only_offences({"open_trade_search": [("app", "Shared<'_>")]}, "open_trade_search"), [])

        sources = [p for p in self.rust_sources() if not is_rust_test(p)]
        self.assertIn(CH_APP / "src" / "craft.rs", sources)
        self.assertIn(CH_APP / "src" / "trade.rs", sources)
        self.assertFalse(any(is_rust_test(p) for p in sources))
        literals, crate_names, calls, found, misnamed = [], [], [], {}, []
        for path in sources:
            text = wr.read(path)
            code = wr.blank_rust_comments(text)
            here = [literal for _, literal in url_literals(wr.rel(path), text)]
            literals += [(path, literal) for literal in here]
            misnamed += address_const_offences(wr.rel(path), text, here)
            crate_names += [path for _ in OPENER_CRATE.finditer(code)]
            calls += [path for _ in OPENER_CALL.finditer(code)]
            self.assertFalse(OPENER_INIT.search(code), wr.rel(path))
            for command, params in commands(text).items():
                self.assertNotIn(command, found, f"{command} declared twice")
                found[command] = params
                self.assertEqual(path, GLUE_LIB, f"{command}: a command outside the glue")
        # The two addresses, each once, in ch-app, each its constant's value;
        # no other web address.
        self.assertEqual(
            sorted(lit for _, lit in literals), sorted(ADDRESS_CONSTS.values()), [(wr.rel(p), lit) for p, lit in literals]
        )
        for path, _ in literals:
            self.assertTrue(path.is_relative_to(CH_APP / "src"), wr.rel(path))
        self.assertEqual(misnamed, [])
        # The opener: named once, called once, both in the glue.
        self.assertEqual(crate_names, [GLUE_LIB])
        self.assertEqual(calls, [GLUE_LIB])
        glue = wr.blank_rust_comments(wr.read(GLUE_LIB))
        self.assertEqual(len(NO_LINK_HANDLER.findall(glue)), 1)
        self.assertEqual(len(ANY_LINK_HANDLER.findall(glue)), 1)
        # The commands: the walk sees them, the two open commands and the
        # league refresh take the state alone, and none takes an address or
        # a path.
        self.assertIn("grab_text", found)
        self.assertIn("set_trade_search", found)
        self.assertEqual([o for c in STATE_ONLY_COMMANDS for o in state_only_offences(found, c)], [])
        self.assertEqual(address_parameter_offences("glue", found), [])

    def test_the_league_list_is_the_one_trade_request(self):
        # The host and path: a changed path, a second definition, one in a
        # comment or a test (not counted), one missing.
        lib = (
            '// pub const TRADE_LEAGUES_PATH: &str = "/api/trade2/data/leagues";\n'
            'pub const TRADE_LEAGUES_HOST: &str = "www.pathofexile.com";\n'
            'pub const TRADE_LEAGUES_PATH: &str = "/api/trade2/data/stats";\n'
            "#[cfg(test)]\nmod tests {\n"
            '    const TRADE_LEAGUES_HOST: &str = "127.0.0.1";\n'
            '    fn f() { let s = "}"; }\n'
            "}\n"
        )
        self.assertEqual(
            league_pin_offences({"lib.rs": lib}),
            ["TRADE_LEAGUES_PATH: '/api/trade2/data/stats', not '/api/trade2/data/leagues'"],
        )
        self.assertEqual(
            league_pin_offences({"lib.rs": lib, "win.rs": 'const TRADE_LEAGUES_HOST: &\'static str = "x";\n'}),
            [
                "TRADE_LEAGUES_HOST: defined 2 times outside tests",
                "TRADE_LEAGUES_PATH: '/api/trade2/data/stats', not '/api/trade2/data/leagues'",
            ],
        )
        self.assertEqual(
            league_pin_offences({"lib.rs": ""}),
            ["TRADE_LEAGUES_HOST: defined 0 times outside tests", "TRADE_LEAGUES_PATH: defined 0 times outside tests"],
        )
        # The callers: the glue's one, a second in it, one through a `use`,
        # and the Shell method of the same name (no call of ep-net).
        glue = (
            "// ep_net::fetch_trade_leagues() in a comment is fine\n"
            "fn fetch_trade_leagues(&self) -> Result<Answer, String> {\n"
            "    ep_net::fetch_trade_leagues()\n"
            "}\n"
            "let again = ep_net :: fetch_trade_leagues();\n"
            "use ep_net::{fetch_release_list, fetch_trade_leagues};\n"
        )
        self.assertEqual(
            league_fetch_names("lib.rs", glue, EP_NET_LEAGUES_CALL), ["lib.rs:3", "lib.rs:5", "lib.rs:6"]
        )
        # In another app, any mention of the name is a caller.
        other = "// fetch_trade_leagues in a comment is fine\nlet r = net::fetch_trade_leagues();\n"
        self.assertEqual(league_fetch_names("o.rs", other, LEAGUES_FETCH_NAME), ["o.rs:2"])

        # The real tree: ep-net defines each pin once, outside tests.
        net = wr.walk(EP_NET / "src", {".rs"})
        self.assertIn(EP_NET / "src" / "lib.rs", net)
        self.assertEqual(league_pin_offences({wr.rel(p): wr.read(p) for p in net}), [])
        # The crafting helper names the call once, in the glue.
        callers = []
        for path in wr.walk(CRATES, {".rs"}) + wr.walk(TAURI, {".rs"}):
            callers += [(path, hit) for hit in league_fetch_names(wr.rel(path), wr.read(path), EP_NET_LEAGUES_CALL)]
        self.assertEqual([p for p, _ in callers], [GLUE_LIB], [hit for _, hit in callers])
        # No other app, nor the other shared crate, names it at all.
        others = []
        for top in OTHER_APPS + [EP_UPDATE]:
            files = wr.walk(top, {".rs"})
            self.assertGreater(len(files), 0 if top == EP_UPDATE else 5, f"the walk found no Rust sources in {wr.rel(top)}")
            for path in files:
                others += league_fetch_names(wr.rel(path), wr.read(path), LEAGUES_FETCH_NAME)
        self.assertEqual(others, [])

    def test_package_json_has_no_tauri_plugin(self):
        self.assertEqual(
            wr.package_offences("o", {"dependencies": {"@tauri-apps/plugin-clipboard-manager": "2"}}),
            ["o: dependencies.@tauri-apps/plugin-clipboard-manager"],
        )
        package = json.loads(wr.read(APP / "package.json"))
        self.assertEqual(wr.package_offences("package.json", package), [])

    def test_csp_and_capabilities_are_pinned(self):
        self.assertEqual(
            wr.capability_offences("o", {"permissions": [{"identifier": "clipboard-manager:default"}, "fs:default"]}),
            ["o: permission 'fs:default'"],
        )
        conf = json.loads(wr.read(TAURI / "tauri.conf.json"))
        self.assertEqual(conf["app"]["security"]["csp"], wr.PINNED_CSP)
        self.assertFalse(conf["app"].get("withGlobalTauri"), "withGlobalTauri must stay off")
        self.assertEqual([w["label"] for w in conf["app"]["windows"]], CONF_WINDOWS)
        capabilities = sorted((TAURI / "capabilities").glob("*.json"))
        self.assertEqual([c.name for c in capabilities], sorted(CAPABILITIES))
        for c in capabilities:
            capability = json.loads(wr.read(c))
            windows, permissions = CAPABILITIES[c.name]
            self.assertEqual(capability["windows"], windows, c.name)
            self.assertEqual(capability["permissions"], permissions, c.name)
            self.assertEqual(wr.capability_offences(c.name, capability), [])

    def test_overlay_window_is_pinned(self):
        good = {"app": {"windows": [{"label": "main"}, dict(OVERLAY_PINS, label="overlay", width=420)]}}
        self.assertEqual(overlay_offences(good), [])
        bad = json.loads(json.dumps(good))
        bad["app"]["windows"][1].update(focusable=True, visible=True)
        del bad["app"]["windows"][1]["transparent"]
        self.assertEqual(
            overlay_offences(bad),
            [
                "overlay: transparent None, not True",
                "overlay: focusable True, not False",
                "overlay: visible True, not False",
            ],
        )
        self.assertEqual(overlay_offences({"app": {"windows": [{"label": "main"}]}}), ["overlay: 0 windows labelled overlay"])
        conf = json.loads(wr.read(TAURI / "tauri.conf.json"))
        self.assertEqual(overlay_offences(conf), [])


class Versions(unittest.TestCase):
    def test_matchers_see_a_stray_dependency(self):
        offender = tomllib.loads(
            '[dependencies]\nserde = { workspace = true }\nch-app = { path = "../crates/ch-app" }\n'
            'regex = "=1.13.1"\nserde_json = { workspace = true, version = "1" }\n'
            "[target.'cfg(windows)'.dependencies]\nwindows-sys = { workspace = true }\n"
        )
        self.assertEqual(
            workspace_dependency_offences("o", offender),
            ["o: dependencies.regex is not taken from the workspace",
             "o: dependencies.serde_json is not taken from the workspace"],
        )
        self.assertEqual(
            wr.version_offences({"Cargo.toml": "0.1.0", "crates/ch-item/Cargo.toml": "0.2.0"}),
            ["crates/ch-item/Cargo.toml: '0.2.0', the workspace has '0.1.0'"],
        )

    def test_every_member_glob_match_is_a_crate(self):
        workspace = tomllib.loads(wr.read(APP / "Cargo.toml"))["workspace"]
        self.assertEqual(workspace["members"], ["crates/*", "src-tauri"])
        dirs = member_dirs()
        self.assertIn(CH_APP, dirs)
        for d in dirs:
            self.assertTrue((d / "Cargo.toml").is_file(), f"{wr.rel(d)} has no Cargo.toml")

    def test_one_version_everywhere(self):
        workspace = tomllib.loads(wr.read(APP / "Cargo.toml"))
        conf = json.loads(wr.read(TAURI / "tauri.conf.json"))
        package = json.loads(wr.read(APP / "package.json"))
        versions = {
            "Cargo.toml": workspace["workspace"]["package"]["version"],
            "tauri.conf.json": conf["version"],
            "package.json": package["version"],
        }
        lock = local_packages(tomllib.loads(wr.read(APP / "Cargo.lock")))
        for d in member_dirs():
            pkg = tomllib.loads(wr.read(d / "Cargo.toml"))["package"]
            if d == CH_ITEM:
                # M2's crate writes its version literally.
                versions[wr.rel(d / "Cargo.toml")] = pkg.get("version")
            else:
                self.assertEqual(pkg.get("version"), {"workspace": True}, wr.rel(d))
            versions[f"Cargo.lock {pkg['name']}"] = lock.get(pkg["name"])
        if CH_ITEM.is_dir():
            self.assertIn("apps/crafting-helper/crates/ch-item/Cargo.toml", versions)
            self.assertIn("Cargo.lock ch-item", versions)
        self.assertEqual(wr.version_offences(versions), [])

    def test_ch_app_and_the_glue_take_everything_from_the_workspace(self):
        offences = []
        for d in (CH_APP, TAURI):
            manifest = tomllib.loads(wr.read(d / "Cargo.toml"))
            for key in ("version", "edition", "publish"):
                self.assertEqual(manifest["package"].get(key), {"workspace": True}, f"{wr.rel(d)} {key}")
            offences += workspace_dependency_offences(wr.rel(d / "Cargo.toml"), manifest)
        self.assertEqual(offences, [])

    def test_every_new_crate_takes_everything_from_the_workspace(self):
        offender = tomllib.loads(
            '[package]\nname = "ch-new"\nversion.workspace = true\nedition = "2021"\n'
            '[dependencies]\nch-item = { path = "../ch-item" }\nserde = { workspace = true }\n'
            'toml = "=1.1.8"\n'
        )
        self.assertEqual(
            package_key_offences("o", offender) + workspace_dependency_offences("o", offender),
            [
                "o: package.edition is not taken from the workspace",
                "o: package.publish is not taken from the workspace",
                "o: dependencies.toml is not taken from the workspace",
            ],
        )
        crates = [d for d in member_dirs() if d not in (CH_ITEM, TAURI)]
        # The walk finds the M4 crates, so a blind walk cannot pass.
        self.assertLessEqual({"ch-app", "ch-data", "ch-blueprint", "ch-engine"}, {d.name for d in crates})
        offences = []
        for d in crates:
            name = wr.rel(d / "Cargo.toml")
            manifest = tomllib.loads(wr.read(d / "Cargo.toml"))
            offences += package_key_offences(name, manifest) + workspace_dependency_offences(name, manifest)
        self.assertEqual(offences, [])

    def test_names_are_pinned(self):
        conf = json.loads(wr.read(TAURI / "tauri.conf.json"))
        self.assertEqual(conf["identifier"], "net.exilepact2.crafting-helper")
        # The program is `Crafting Helper.exe` (0.2.0); the identifier, the
        # Cargo names and the window titles stay.
        self.assertEqual(conf["productName"], "Crafting Helper")
        self.assertEqual(conf["mainBinaryName"], "Crafting Helper")
        self.assertEqual(conf["app"]["windows"][0]["title"], "Crafting Helper")
        self.assertIs(conf["bundle"]["active"], False)
        tauri = tomllib.loads(wr.read(TAURI / "Cargo.toml"))
        self.assertEqual(tauri["package"]["name"], "crafting-helper")
        self.assertEqual(tauri["lib"]["name"], "crafting_helper_lib")


class LockedToQuestTrackerPins(unittest.TestCase):
    def test_matchers_see_a_new_package(self):
        ours = {"package": [
            {"name": "serde", "version": "1.0.229", "source": "registry+https://github.com/rust-lang/crates.io-index"},
            {"name": "leftpad", "version": "0.1.0", "source": "registry+https://github.com/rust-lang/crates.io-index"},
            {"name": "ch-app", "version": "0.1.0"},
        ]}
        theirs = {"package": [ours["package"][0]]}
        self.assertEqual(wr.registry_packages(ours) - wr.registry_packages(theirs), {("leftpad", "0.1.0")})
        self.assertEqual(local_packages(ours), {"ch-app": "0.1.0"})

    def test_every_crate_is_in_the_quest_trackers_lock(self):
        lock = tomllib.loads(wr.read(APP / "Cargo.lock"))
        ours = wr.registry_packages(lock)
        theirs = wr.registry_packages(tomllib.loads(wr.read(QT / "Cargo.lock")))
        self.assertGreater(len(ours), 100, "the app's lock lists too few packages")
        self.assertEqual(sorted(ours - theirs - wr.NEW_REGISTRY_PACKAGES), [])
        # The one new crate is there, at its pin (else the allowance is stale).
        self.assertEqual(ours & wr.NEW_REGISTRY_PACKAGES, wr.NEW_REGISTRY_PACKAGES)
        members = {tomllib.loads(wr.read(d / "Cargo.toml"))["package"]["name"] for d in member_dirs()}
        self.assertLessEqual({"ch-app", "crafting-helper"}, members)
        if CH_ITEM.is_dir():
            self.assertIn("ch-item", members)
        self.assertIn("ch-watch", members)
        # The members, the shared crates and the quest tracker's two screen
        # crates (M6); `wr.SHARED_CRATES` stays the shared crates alone.
        self.assertEqual(set(local_packages(lock)), members | wr.SHARED_CRATES | LINKED_CRATES)

    def test_workspace_pins_equal_the_quest_trackers(self):
        ours = tomllib.loads(wr.read(APP / "Cargo.toml"))["workspace"]["dependencies"]
        theirs = tomllib.loads(wr.read(QT / "Cargo.toml"))["workspace"]["dependencies"]
        for dep, spec in ours.items():
            if dep in PINS_OUTSIDE_THE_QUEST_TRACKER:
                # Checked against the quest tracker's lock, below.
                continue
            self.assertIn(dep, theirs, f"{dep} is not a quest tracker pin")
            self.assertTrue(str(pin_version(spec)).startswith("="), f"{dep} is not pinned exactly")
            self.assertEqual(pin_version(spec), pin_version(theirs[dep]), dep)

    def test_workspace_pins_outside_the_quest_tracker_are_in_its_lock(self):
        lock = {"package": [
            {"name": "toml", "version": "1.1.8+spec-1.1.0", "source": "registry+https://github.com/rust-lang/crates.io-index"},
        ]}
        names = {"toml", "leftpad"}
        self.assertEqual(lock_pin_offences({"toml": "=1.1.8"}, lock, names), [])
        self.assertEqual(
            lock_pin_offences({"toml": "=1.1.9", "leftpad": {"version": "=0.1.0"}, "serde": "=9"}, lock, names),
            [
                "leftpad: 0.1.0 is not in the quest tracker's Cargo.lock",
                "toml: 1.1.9 is not in the quest tracker's Cargo.lock",
            ],
        )
        self.assertEqual(lock_pin_offences({"toml": "1.1.8"}, lock, names), ["toml: '1.1.8' is not pinned exactly"])

        ours = tomllib.loads(wr.read(APP / "Cargo.toml"))["workspace"]["dependencies"]
        theirs = tomllib.loads(wr.read(QT / "Cargo.toml"))["workspace"]["dependencies"]
        # Each allowance is used and still needed (else it is stale).
        self.assertEqual(set(ours) - set(theirs), PINS_OUTSIDE_THE_QUEST_TRACKER)
        qt_lock = tomllib.loads(wr.read(QT / "Cargo.lock"))
        self.assertEqual(lock_pin_offences(ours, qt_lock, PINS_OUTSIDE_THE_QUEST_TRACKER), [])

    def test_npm_versions_equal_the_quest_trackers(self):
        ours = json.loads(wr.read(APP / "package.json"))
        theirs = json.loads(wr.read(QT / "package.json"))
        for table in ("dependencies", "devDependencies"):
            for dep, version in ours.get(table, {}).items():
                self.assertEqual(version, theirs.get(table, {}).get(dep), f"{table}.{dep}")
        ours_lock = wr.npm_packages(json.loads(wr.read(APP / "package-lock.json")))
        theirs_lock = wr.npm_packages(json.loads(wr.read(QT / "package-lock.json")))
        self.assertGreater(len(ours_lock), 20, "the app's package-lock lists too few packages")
        self.assertEqual(sorted(ours_lock - theirs_lock), [])


if __name__ == "__main__":
    unittest.main()
