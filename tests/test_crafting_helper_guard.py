"""The crafting helper touches nothing but its windows, one clipboard read per
grab and, through the shared `ep-net` alone, GitHub's release list and
assets. It sends no input, watches nothing and reaches no game file.

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
  another program's window, the game's files);
- the one exception is the overlay's show and hide: `src-tauri/src/lib.rs`
  alone may name `ShowWindow`, and every `SW_` name it holds is
  `SW_SHOWNOACTIVATE` or `SW_HIDE`, so the game keeps the focus;
  `ShowWindowAsync` stays an offence everywhere;
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
  crate, or on a Tauri plugin but the global shortcut and single instance,
  and `package.json` on no Tauri plugin; the `windows-sys` features are the
  waystone roller's set;
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
external dependency from the workspace (`ch-item` writes its own, std-only);
every directory the `crates/*` member glob finds has a `Cargo.toml`; the
executable's name, library name and identifier are the pinned ones. The
guide's `### <version>` release heading is pinned by the docs criterion of
the workorder that wrote the guide, not here.

`LockedToQuestTrackerPins`: every registry package in the app's
`Cargo.lock` but `minisign-verify` and every package in its
`package-lock.json` is one the quest tracker's lock already holds, at the
same version; the local packages are the workspace's members (`ch-item`
included when it exists) and the shared `ep-net` and `ep-update`; and the
npm and workspace versions equal the quest tracker's.

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


def input_offences(name: str, text: str, glue: bool) -> list[str]:
    """The input, watching, window, process and game names one Rust source
    may not name. `glue` says whether it is `src-tauri/src/lib.rs`: there
    `ShowWindow` is allowed and every `SW_` name is one of the overlay's two
    show commands; anywhere else `ShowWindow` is an offence."""
    code = wr.blank_rust_comments(text)
    offences = wr.text_offences(name, code, NO_INPUT_NAMES + WATCH_NAMES + SHOW_WINDOW_ASYNC)
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


def member_dirs() -> list[Path]:
    """Every directory the workspace's `crates/*` glob finds, and `src-tauri`."""
    return sorted(d for d in CRATES.iterdir() if d.is_dir()) + [TAURI]


def local_packages(lock: dict) -> dict[str, str]:
    """name -> version of every local (sourceless) package in a Cargo.lock."""
    return {p["name"]: p["version"] for p in lock.get("package", []) if "source" not in p}


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

        offences = []
        for path in self.rust_sources():
            offences += input_offences(wr.rel(path), wr.read(path), glue=path == GLUE_LIB)
        self.assertEqual(offences, [])

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
            offences += wr.manifest_offences(wr.rel(m), tomllib.loads(wr.read(m)))
        self.assertEqual(offences, [])
        workspace = tomllib.loads(wr.read(APP / "Cargo.toml"))
        self.assertEqual(
            set(workspace["workspace"]["dependencies"]["windows-sys"]["features"]),
            wr.WINDOWS_SYS_FEATURES,
        )
        self.assertEqual(
            {d for d in workspace["workspace"]["dependencies"] if d.startswith("tauri-plugin-")},
            wr.ALLOWED_TAURI_PLUGINS,
        )

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

    def test_names_are_pinned(self):
        conf = json.loads(wr.read(TAURI / "tauri.conf.json"))
        self.assertEqual(conf["identifier"], "net.exilepact2.crafting-helper")
        self.assertEqual(conf["mainBinaryName"], "crafting-helper")
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
        self.assertEqual(set(local_packages(lock)), members | wr.SHARED_CRATES)

    def test_workspace_pins_equal_the_quest_trackers(self):
        ours = tomllib.loads(wr.read(APP / "Cargo.toml"))["workspace"]["dependencies"]
        theirs = tomllib.loads(wr.read(QT / "Cargo.toml"))["workspace"]["dependencies"]

        def version(spec):
            return spec if isinstance(spec, str) else spec.get("version")

        for dep, spec in ours.items():
            self.assertIn(dep, theirs, f"{dep} is not a quest tracker pin")
            self.assertTrue(str(version(spec)).startswith("="), f"{dep} is not pinned exactly")
            self.assertEqual(version(spec), version(theirs[dep]), dep)

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
