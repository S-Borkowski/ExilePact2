"""The waystone roller touches nothing but its windows, the clipboard and,
through the shared `ep-net` alone, GitHub's release list and assets.

Read from the files only (no build, no network), these pin the rules of
the app (`docs/waystone-roller/README.md`, Privacy):

`NoInputNoNetworkNoGame`:

- no Rust source of the app (comments blanked, strings kept) names a Win32
  call that sends input, posts or sends a window message, opens, reads or
  writes another process, hooks, or finds another program's window, nor
  the game's files (`PathOfExile`, `Client.txt`, `Content.ggpk`);
- no network but `ep-net` (the shared workspace's transport crate,
  `apps/shared/`, guarded by `tests/test_app_updates_guard.py`), reached
  only by the glue: no Rust source of the app names a socket or an HTTP
  client, only `src-tauri` names `ep_net` or depends on `ep-net`, `wr-app`
  takes the shared `ep-update` (no network) and nothing else of it, and no
  UI source but the tests calls `fetch(`, `XMLHttpRequest`, `WebSocket`,
  `EventSource` or `sendBeacon`;
- no manifest depends on an HTTP, socket, TLS or WebSocket crate, on a
  Tauri plugin but the global shortcut and single instance (no
  `tauri-plugin-opener`: nothing opens a browser), or on an input or
  process crate, and `package.json` on no Tauri plugin;
- the `windows-sys` features are the pinned clipboard-and-beep set;
- the windows' CSP is the pinned string; the capabilities are the `main`
  window's with `core:event:default` and `core:window:default` only and the
  `settings` window's with `core:event:default` only, and no capability
  grants an `http:`, `shell:`, `fs:`, `opener:`, `process:`, `websocket:`,
  `upload:` or `updater:` permission or a `remote` URL.

`Versions`: the workspace, `tauri.conf.json` and `package.json` carry one
version, and the guide's Release notes have a heading for it; the
executable's name and identifier are the pinned ones.

`LockedToQuestTrackerPins`: every registry package in the app's
`Cargo.lock` but `minisign-verify` (the shared update crate's signature
check, the one registry crate this workorder adds) and every package in its
`package-lock.json` is one the quest tracker's lock already holds, at the
same version; the local packages are the app's three and the shared
`ep-net` and `ep-update`; and the npm and workspace versions equal the
quest tracker's.

Every matcher runs on a synthetic offender first, so a matcher that sees
nothing cannot pass, and every walk must find files. A failure names the
file and the offending entry.
"""
import json
import os
import re
import tomllib
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
APP = ROOT / "apps" / "waystone-roller"
TAURI = APP / "src-tauri"
QT = ROOT / "apps" / "quest-tracker"
GUIDE = ROOT / "docs" / "waystone-roller" / "README.md"

SKIP_DIRS = {"target", "node_modules", "gen", "dist"}

PINNED_CSP = (
    "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; "
    "img-src 'self' data:; font-src 'self' data:; connect-src ipc: http://ipc.localhost"
)
CAPABILITY_WINDOWS = ["main"]
CAPABILITY_PERMISSIONS = ["core:event:default", "core:window:default"]
# The Settings window is built at run time (not in tauri.conf.json); its
# capability only listens for `wr:app-settings`.
SETTINGS_WINDOWS = ["settings"]
SETTINGS_PERMISSIONS = ["core:event:default"]
# The shared crates: `ep-net` is the only network code and only the glue
# takes it; `wr-app` takes `ep-update`, which has none.
SHARED_CRATES = {"ep-net", "ep-update"}
NETWORK_CRATE = "ep-net"
NETWORK_CRATE_NAME = re.compile(r"\bep[-_]net\b")
# The registry crate the shared update code adds, pinned in apps/shared.
NEW_REGISTRY_PACKAGES = {("minisign-verify", "0.2.5")}
FORBIDDEN_PERMISSION_PREFIXES = (
    "http:", "shell:", "fs:", "opener:", "process:", "websocket:", "upload:", "updater:",
)
WINDOWS_SYS_FEATURES = {
    "Win32_Foundation",
    "Win32_UI_WindowsAndMessaging",
    "Win32_System_DataExchange",
    "Win32_System_Memory",
    "Win32_System_Ole",
    "Win32_System_Diagnostics_Debug",
}
ALLOWED_TAURI_PLUGINS = {"tauri-plugin-global-shortcut", "tauri-plugin-single-instance"}
FORBIDDEN_CRATES = (
    "reqwest", "ureq", "hyper", "isahc", "curl", "attohttpc", "minreq", "surf",
    "socket2", "tungstenite", "tokio-tungstenite", "native-tls", "rustls",
    "enigo", "rdev", "inputbot", "winput", "sysinfo", "process-memory", "read-process-memory",
    "arboard", "clipboard-win",
)

GAME_NAMES = [
    # Input sent to another program.
    ("SendInput", re.compile(r"\bSendInput\b")),
    ("keybd_event", re.compile(r"\bkeybd_event\b")),
    ("mouse_event", re.compile(r"\bmouse_event\b")),
    ("PostMessage", re.compile(r"\bPostMessage[AW]?\b")),
    ("SendMessage", re.compile(r"\bSendMessage[AW]?\b")),
    ("PostThreadMessage", re.compile(r"\bPostThreadMessage[AW]?\b")),
    ("KeyboardAndMouse", re.compile(r"KeyboardAndMouse")),
    # Another process's memory, threads or hooks.
    ("OpenProcess", re.compile(r"\bOpenProcess\b")),
    ("ReadProcessMemory", re.compile(r"\bReadProcessMemory\b")),
    ("WriteProcessMemory", re.compile(r"\bWriteProcessMemory\b")),
    ("VirtualAllocEx", re.compile(r"\bVirtualAllocEx\b")),
    ("CreateRemoteThread", re.compile(r"\bCreateRemoteThread\b")),
    ("SetWindowsHookEx", re.compile(r"\bSetWindowsHookEx[AW]?\b")),
    ("DebugActiveProcess", re.compile(r"\bDebugActiveProcess\b")),
    # Another program's window.
    ("FindWindow", re.compile(r"\bFindWindow(Ex)?[AW]?\b")),
    ("GetForegroundWindow", re.compile(r"\bGetForegroundWindow\b")),
    ("SetForegroundWindow", re.compile(r"\bSetForegroundWindow\b")),
    # The game's files.
    ("PathOfExile", re.compile(r"PathOfExile", re.IGNORECASE)),
    ("Client.txt", re.compile(r"Client\.txt", re.IGNORECASE)),
    ("Content.ggpk", re.compile(r"\.ggpk", re.IGNORECASE)),
    ("Grinding Gear Games", re.compile(r"Grinding Gear Games", re.IGNORECASE)),
]
RUST_NETWORK_NAMES = [
    ("std::net", re.compile(r"\bstd\s*::\s*net\b")),
    ("TcpStream", re.compile(r"TcpStream")),
    ("UdpSocket", re.compile(r"UdpSocket")),
    ("TcpListener", re.compile(r"TcpListener")),
    ("WinHttp", re.compile(r"WinHttp", re.IGNORECASE)),
    ("InternetOpen", re.compile(r"InternetOpen")),
    ("WinSock", re.compile(r"\bWinSock\b|\bWSAStartup\b")),
    ("URLDownloadToFile", re.compile(r"\bURLDownloadToFile")),
    ("tokio::net", re.compile(r"\btokio\s*::\s*net\b")),
    ("reqwest", re.compile(r"\breqwest\b")),
    ("WebviewUrl::External", re.compile(r"\bWebviewUrl\s*::\s*External\b")),
]
UI_NETWORK_NAMES = [
    ("fetch(", re.compile(r"\bfetch\s*\(")),
    ("XMLHttpRequest", re.compile(r"XMLHttpRequest")),
    ("WebSocket", re.compile(r"WebSocket")),
    ("EventSource", re.compile(r"EventSource")),
    ("sendBeacon", re.compile(r"sendBeacon")),
]
UI_SUFFIXES = {".ts", ".js", ".mjs", ".svelte", ".html"}


def rel(path: Path) -> str:
    return path.relative_to(ROOT).as_posix()


def walk(top: Path, suffixes: set[str] | None = None, names: set[str] | None = None) -> list[Path]:
    """Files under `top`, skipping build output, by suffix or by exact name."""
    found = []
    for folder, dirs, files in os.walk(top):
        dirs[:] = [d for d in dirs if d not in SKIP_DIRS]
        for file in files:
            path = Path(folder) / file
            if (suffixes and path.suffix in suffixes) or (names and file in names):
                found.append(path)
    return sorted(found)


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8").replace("\r\n", "\n")


def blank_rust_comments(text: str) -> str:
    """`text` with its `//` and `/* */` comments turned to spaces; strings
    are kept (a forbidden name in a string is still an offence) and are
    skipped over, so a `//` inside one is not a comment. Newlines are kept."""
    out = list(text)
    i, n = 0, len(text)

    def blank(a: int, b: int) -> None:
        for k in range(a, b):
            if out[k] != "\n":
                out[k] = " "

    while i < n:
        if text.startswith("//", i):
            end = text.find("\n", i)
            end = n if end < 0 else end
            blank(i, end)
            i = end
        elif text.startswith("/*", i):
            depth, j = 1, i + 2
            while j < n and depth:
                if text.startswith("/*", j):
                    depth, j = depth + 1, j + 2
                elif text.startswith("*/", j):
                    depth, j = depth - 1, j + 2
                else:
                    j += 1
            blank(i, j)
            i = j
        elif text[i] == '"':
            j = i + 1
            while j < n and text[j] != '"':
                j += 2 if text[j] == "\\" else 1
            i = j + 1
        else:
            i += 1
    return "".join(out)


def text_offences(name: str, text: str, matchers) -> list[str]:
    """`name:line: label` for every matcher hit in `text`."""
    offences = []
    for label, pattern in matchers:
        for m in pattern.finditer(text):
            line = text.count("\n", 0, m.start()) + 1
            offences.append(f"{name}:{line}: {label}")
    return offences


def rust_offences(name: str, text: str, matchers) -> list[str]:
    return text_offences(name, blank_rust_comments(text), matchers)


def forbidden_crate(dep: str) -> bool:
    return any(dep == bad or dep.startswith(bad + "-") for bad in FORBIDDEN_CRATES)


def dependency_tables(manifest: dict):
    """Every dependency table of a Cargo manifest, with a label for it."""
    kinds = ("dependencies", "dev-dependencies", "build-dependencies")
    for kind in kinds:
        if kind in manifest:
            yield kind, manifest[kind]
    workspace = manifest.get("workspace", {})
    if "dependencies" in workspace:
        yield "workspace.dependencies", workspace["dependencies"]
    for target, table in manifest.get("target", {}).items():
        for kind in kinds:
            if kind in table:
                yield f"target.{target}.{kind}", table[kind]


def manifest_offences(name: str, manifest: dict) -> list[str]:
    """Forbidden crates, Tauri plugins beyond the two allowed, and
    `windows-sys` features beyond the pinned set, in one manifest."""
    offences = []
    for label, table in dependency_tables(manifest):
        for dep, spec in table.items():
            if forbidden_crate(dep):
                offences.append(f"{name}: {label}.{dep}")
            if dep.startswith("tauri-plugin-") and dep not in ALLOWED_TAURI_PLUGINS:
                offences.append(f"{name}: {label}.{dep} (a Tauri plugin)")
            if dep == "windows-sys" and isinstance(spec, dict):
                extra = set(spec.get("features", [])) - WINDOWS_SYS_FEATURES
                for f in sorted(extra):
                    offences.append(f"{name}: {label}.windows-sys feature {f}")
    return offences


def shared_dependency_offences(name: str, manifest: dict, glue: bool) -> list[str]:
    """A dependency on a shared crate that is not a path into
    `apps/shared/crates/`, and one on `ep-net` outside the glue."""
    offences = []
    for label, table in dependency_tables(manifest):
        for dep, spec in table.items():
            if dep not in SHARED_CRATES:
                continue
            path = spec.get("path", "") if isinstance(spec, dict) else ""
            if not path.replace("\\", "/").endswith(f"shared/crates/{dep}"):
                offences.append(f"{name}: {label}.{dep} is not the shared crate's path")
            if dep == NETWORK_CRATE and not glue:
                offences.append(f"{name}: {label}.{dep} outside the glue")
    return offences


def capability_offences(name: str, capability: dict) -> list[str]:
    offences = []
    for entry in capability.get("permissions", []):
        ident = entry.get("identifier", "") if isinstance(entry, dict) else str(entry)
        if ident.startswith(FORBIDDEN_PERMISSION_PREFIXES):
            offences.append(f"{name}: permission {ident!r}")
    if "remote" in capability:
        offences.append(f"{name}: remote {capability['remote']!r}")
    return offences


def package_offences(name: str, package: dict) -> list[str]:
    offences = []
    for table in ("dependencies", "devDependencies", "optionalDependencies"):
        for dep in package.get(table, {}):
            if dep.startswith("@tauri-apps/plugin-"):
                offences.append(f"{name}: {table}.{dep}")
    return offences


def is_ui_test(path: Path) -> bool:
    return path.name.endswith((".test.ts", ".test.js")) or path.name == "test-support.ts"


def registry_packages(lock: dict) -> set[tuple[str, str]]:
    """(name, version) of every registry package in a parsed Cargo.lock."""
    return {
        (p["name"], p["version"])
        for p in lock.get("package", [])
        if str(p.get("source", "")).startswith("registry+")
    }


def npm_packages(lock: dict) -> set[tuple[str, str]]:
    """(path, version) of every installed package in a package-lock.json."""
    return {
        (path, info.get("version", ""))
        for path, info in lock.get("packages", {}).items()
        if path.startswith("node_modules/")
    }


def version_offences(versions: dict[str, str]) -> list[str]:
    """Every source whose version differs from the workspace's."""
    want = versions.get("Cargo.toml")
    return [f"{k}: {v!r}, the workspace has {want!r}" for k, v in versions.items() if v != want]


def release_headings(guide: str) -> list[str]:
    """The `### <version>` headings of the guide's `## Release notes`."""
    s = guide.find("\n## Release notes\n")
    if s < 0:
        return []
    e = guide.find("\n## ", s + 1)
    section = guide[s:] if e < 0 else guide[s:e]
    return re.findall(r"^### (\S+)", section, re.MULTILINE)


class NoInputNoNetworkNoGame(unittest.TestCase):
    def rust_sources(self) -> list[Path]:
        files = walk(APP / "crates", {".rs"}) + walk(TAURI, {".rs"})
        self.assertGreater(len(files), 5, "the walk found no Rust sources")
        self.assertIn(TAURI / "src" / "lib.rs", files)
        return files

    def test_no_rust_source_sends_input_or_reaches_the_game(self):
        offender = (
            "// SendInput in a comment is fine\n"
            "let s = \"http://example\"; unsafe { SendInput(1, p, 4) };\n"
            "/* ReadProcessMemory */ let h = OpenProcess(0, 0, pid);\n"
            "let p = \"C:/Games/PathOfExile.exe\";\n"
        )
        hits = rust_offences("offender.rs", offender, GAME_NAMES)
        self.assertEqual(
            hits,
            ["offender.rs:2: SendInput", "offender.rs:3: OpenProcess", "offender.rs:4: PathOfExile"],
        )
        self.assertEqual(rust_offences("clean.rs", "let x = \"// SendInput\"; // OpenProcess\n", GAME_NAMES[:1]), ["clean.rs:1: SendInput"])
        self.assertEqual(rust_offences("clean.rs", "// SendInput OpenProcess\nfn f() {}\n", GAME_NAMES), [])

        offences = []
        for path in self.rust_sources():
            offences += rust_offences(rel(path), read(path), GAME_NAMES)
        self.assertEqual(offences, [])

    def test_no_rust_source_names_the_network(self):
        self.assertEqual(
            rust_offences("o.rs", "use std::net::TcpStream;\n", RUST_NETWORK_NAMES),
            ["o.rs:1: std::net", "o.rs:1: TcpStream"],
        )
        offences = []
        for path in self.rust_sources():
            offences += rust_offences(rel(path), read(path), RUST_NETWORK_NAMES)
        self.assertEqual(offences, [])

    def test_only_the_glue_reaches_ep_net(self):
        matcher = [("ep_net", NETWORK_CRATE_NAME)]
        self.assertEqual(
            rust_offences("o.rs", "// ep_net in a comment\nlet r = ep_net::fetch_release_list(p);\n", matcher),
            ["o.rs:2: ep_net"],
        )
        offender = {
            "dependencies": {
                "ep-net": {"path": "../../shared/crates/ep-net"},
                "ep-update": {"path": "../vendor/ep-update"},
            },
        }
        self.assertEqual(
            shared_dependency_offences("o", offender, glue=False),
            [
                "o: dependencies.ep-net outside the glue",
                "o: dependencies.ep-update is not the shared crate's path",
            ],
        )
        self.assertEqual(
            shared_dependency_offences("o", offender, glue=True),
            ["o: dependencies.ep-update is not the shared crate's path"],
        )

        # Rust: only the glue names it.
        offences = []
        named = []
        for path in self.rust_sources():
            hits = rust_offences(rel(path), read(path), matcher)
            if path.is_relative_to(TAURI):
                named += hits
            else:
                offences += hits
        self.assertEqual(offences, [])
        self.assertTrue(named, "the glue never names ep_net: the walk or the matcher is blind")

        # Manifests: only the glue depends on it; both shared crates by path.
        offences = []
        for m in walk(APP, names={"Cargo.toml"}):
            offences += shared_dependency_offences(rel(m), tomllib.loads(read(m)), glue=m.parent == TAURI)
        self.assertEqual(offences, [])
        glue = tomllib.loads(read(TAURI / "Cargo.toml"))["dependencies"]
        self.assertIn(NETWORK_CRATE, glue)
        app = tomllib.loads(read(APP / "crates" / "wr-app" / "Cargo.toml"))["dependencies"]
        self.assertEqual(sorted(set(app) & SHARED_CRATES), ["ep-update"])

    def test_no_ui_source_calls_the_network(self):
        self.assertEqual(
            text_offences("o.ts", "await fetch('https://x');\nnew WebSocket(u);\n", UI_NETWORK_NAMES),
            ["o.ts:1: fetch(", "o.ts:2: WebSocket"],
        )
        files = [p for p in walk(APP / "src", UI_SUFFIXES) if not is_ui_test(p)]
        # Every root page: the main window's and the Settings window's.
        pages = sorted(APP.glob("*.html"))
        self.assertIn(APP / "index.html", pages)
        files += pages
        self.assertGreater(len(files), 5, "the walk found no UI sources")
        offences = []
        for path in files:
            offences += text_offences(rel(path), read(path), UI_NETWORK_NAMES + GAME_NAMES[:7])
        self.assertEqual(offences, [])

    def test_no_manifest_depends_on_network_input_or_process_crates(self):
        offender = {
            "dependencies": {"reqwest": "0.12", "tauri-plugin-http": "2", "tauri-plugin-global-shortcut": "2"},
            "target": {"cfg(windows)": {"dependencies": {
                "windows-sys": {"version": "0.61", "features": ["Win32_Foundation", "Win32_UI_Input_KeyboardAndMouse"]},
                "enigo": "0.2",
            }}},
        }
        self.assertEqual(
            sorted(manifest_offences("o", offender)),
            sorted([
                "o: dependencies.reqwest",
                "o: dependencies.tauri-plugin-http (a Tauri plugin)",
                "o: target.cfg(windows).dependencies.windows-sys feature Win32_UI_Input_KeyboardAndMouse",
                "o: target.cfg(windows).dependencies.enigo",
            ]),
        )
        manifests = walk(APP, names={"Cargo.toml"})
        self.assertEqual(
            {rel(m) for m in manifests},
            {
                "apps/waystone-roller/Cargo.toml",
                "apps/waystone-roller/crates/wr-regex/Cargo.toml",
                "apps/waystone-roller/crates/wr-app/Cargo.toml",
                "apps/waystone-roller/src-tauri/Cargo.toml",
            },
        )
        offences = []
        for m in manifests:
            offences += manifest_offences(rel(m), tomllib.loads(read(m)))
        self.assertEqual(offences, [])
        workspace = tomllib.loads(read(APP / "Cargo.toml"))
        self.assertEqual(
            set(workspace["workspace"]["dependencies"]["windows-sys"]["features"]),
            WINDOWS_SYS_FEATURES,
        )

    def test_package_json_has_no_tauri_plugin(self):
        self.assertEqual(
            package_offences("o", {"dependencies": {"@tauri-apps/plugin-http": "2", "@tauri-apps/api": "2"}}),
            ["o: dependencies.@tauri-apps/plugin-http"],
        )
        package = json.loads(read(APP / "package.json"))
        self.assertEqual(package_offences("package.json", package), [])

    def test_csp_and_capability_are_pinned(self):
        self.assertEqual(
            capability_offences("o", {"permissions": ["core:event:default", {"identifier": "http:default"}], "remote": {"urls": ["https://x"]}}),
            ["o: permission 'http:default'", "o: remote {'urls': ['https://x']}"],
        )
        conf = json.loads(read(TAURI / "tauri.conf.json"))
        self.assertEqual(conf["app"]["security"]["csp"], PINNED_CSP)
        self.assertFalse(conf["app"].get("withGlobalTauri"), "withGlobalTauri must stay off")
        self.assertEqual([w["label"] for w in conf["app"]["windows"]], CAPABILITY_WINDOWS)
        capabilities = sorted((TAURI / "capabilities").glob("*.json"))
        self.assertEqual([c.name for c in capabilities], ["default.json", "settings.json"])
        default = json.loads(read(TAURI / "capabilities" / "default.json"))
        self.assertEqual(default["windows"], CAPABILITY_WINDOWS)
        self.assertEqual(default["permissions"], CAPABILITY_PERMISSIONS)
        settings = json.loads(read(TAURI / "capabilities" / "settings.json"))
        self.assertEqual(settings["windows"], SETTINGS_WINDOWS)
        self.assertEqual(settings["permissions"], SETTINGS_PERMISSIONS)
        for c in capabilities:
            self.assertEqual(capability_offences(c.name, json.loads(read(c))), [])


class Versions(unittest.TestCase):
    def test_matcher_names_a_lagging_file(self):
        self.assertEqual(
            version_offences({"Cargo.toml": "0.2.0", "tauri.conf.json": "0.2.0", "package.json": "0.1.0"}),
            ["package.json: '0.1.0', the workspace has '0.2.0'"],
        )
        self.assertEqual(release_headings("x\n## Release notes\n\n### 0.2.0 (y)\n\n### 0.1.0\n\n## Next\n### 9\n"), ["0.2.0", "0.1.0"])

    def test_one_version_everywhere(self):
        workspace = tomllib.loads(read(APP / "Cargo.toml"))
        conf = json.loads(read(TAURI / "tauri.conf.json"))
        package = json.loads(read(APP / "package.json"))
        versions = {
            "Cargo.toml": workspace["workspace"]["package"]["version"],
            "tauri.conf.json": conf["version"],
            "package.json": package["version"],
        }
        self.assertEqual(version_offences(versions), [])
        self.assertIn(versions["Cargo.toml"], release_headings(read(GUIDE)))
        for m in walk(APP, names={"Cargo.toml"}):
            pkg = tomllib.loads(read(m)).get("package")
            if pkg:
                self.assertEqual(pkg.get("version"), {"workspace": True}, rel(m))

    def test_names_are_pinned(self):
        conf = json.loads(read(TAURI / "tauri.conf.json"))
        self.assertEqual(conf["identifier"], "net.exilepact2.waystone-roller")
        self.assertEqual(conf["mainBinaryName"], "waystone-roller")
        self.assertEqual(conf["app"]["windows"][0]["title"], "Waystone Roller")
        self.assertIs(conf["bundle"]["active"], False)
        tauri = tomllib.loads(read(TAURI / "Cargo.toml"))
        self.assertEqual(tauri["package"]["name"], "waystone-roller")


class LockedToQuestTrackerPins(unittest.TestCase):
    def test_matchers_see_a_new_package(self):
        ours = {"package": [
            {"name": "serde", "version": "1.0.229", "source": "registry+https://github.com/rust-lang/crates.io-index"},
            {"name": "leftpad", "version": "0.1.0", "source": "registry+https://github.com/rust-lang/crates.io-index"},
            {"name": "wr-app", "version": "0.1.0"},
        ]}
        theirs = {"package": [ours["package"][0]]}
        self.assertEqual(registry_packages(ours) - registry_packages(theirs), {("leftpad", "0.1.0")})
        self.assertEqual(
            npm_packages({"packages": {"": {}, "node_modules/vite": {"version": "8.3.4"}}})
            - npm_packages({"packages": {"node_modules/vite": {"version": "8.3.3"}}}),
            {("node_modules/vite", "8.3.4")},
        )

    def test_every_crate_is_in_the_quest_trackers_lock(self):
        ours = registry_packages(tomllib.loads(read(APP / "Cargo.lock")))
        theirs = registry_packages(tomllib.loads(read(QT / "Cargo.lock")))
        self.assertGreater(len(ours), 100, "the app's lock lists too few packages")
        self.assertEqual(sorted(ours - theirs - NEW_REGISTRY_PACKAGES), [])
        # The one new crate is there, at its pin (else the allowance is stale).
        self.assertEqual(ours & NEW_REGISTRY_PACKAGES, NEW_REGISTRY_PACKAGES)
        lock = tomllib.loads(read(APP / "Cargo.lock"))
        local = {p["name"] for p in lock["package"] if "source" not in p}
        self.assertEqual(local, {"wr-regex", "wr-app", "waystone-roller"} | SHARED_CRATES)

    def test_workspace_pins_equal_the_quest_trackers(self):
        ours = tomllib.loads(read(APP / "Cargo.toml"))["workspace"]["dependencies"]
        theirs = tomllib.loads(read(QT / "Cargo.toml"))["workspace"]["dependencies"]

        def version(spec):
            return spec if isinstance(spec, str) else spec.get("version")

        for dep, spec in ours.items():
            self.assertIn(dep, theirs, f"{dep} is not a quest tracker pin")
            self.assertEqual(version(spec), version(theirs[dep]), dep)

    def test_npm_versions_equal_the_quest_trackers(self):
        ours = json.loads(read(APP / "package.json"))
        theirs = json.loads(read(QT / "package.json"))
        for table in ("dependencies", "devDependencies"):
            for dep, version in ours.get(table, {}).items():
                self.assertEqual(version, theirs.get(table, {}).get(dep), f"{table}.{dep}")
        ours_lock = npm_packages(json.loads(read(APP / "package-lock.json")))
        theirs_lock = npm_packages(json.loads(read(QT / "package-lock.json")))
        self.assertGreater(len(ours_lock), 20, "the app's package-lock lists too few packages")
        self.assertEqual(sorted(ours_lock - theirs_lock), [])


if __name__ == "__main__":
    unittest.main()
