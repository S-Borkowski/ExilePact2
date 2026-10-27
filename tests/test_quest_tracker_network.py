"""The quest tracker makes no network request the user did not order.

The update check is the app's only network code, and it runs only when the
user presses *Check for updates* or turns on *Automatically check for
updates*. The behaviour is proven by `crates/qt-app/tests/updates.rs` with a
recording fake transport; this file pins, by reading files only, that there
is no other door through which the app could reach the internet.

`NoNetworkByDefault` (the baseline) holds before the update check exists and
must keep holding after it:

- the overlay's CSP is the pinned string;
- `capabilities/default.json` keeps its one window, `overlay`, and its three
  permissions, and no capability file grants an `http:`, `updater:`,
  `websocket:`, `upload:`, `shell:`, `fs:`, `process:` or `opener:`
  permission or a `remote` URL;
- no `Cargo.toml` under the app depends on an HTTP client or a Tauri network
  plugin, and `package.json` on no Tauri network plugin;
- no Rust source outside `crates/qt-net/` (the transport crate) names a
  socket, WinHTTP, WinINet or an HTTP client crate;
- no UI source but the tests calls `fetch(`, `XMLHttpRequest`, `WebSocket`,
  `EventSource` or `sendBeacon`.

Every matcher is first run on a synthetic offender in the same test, so a
matcher that sees nothing cannot pass, and every walk must find files. A
failure names the file and the offending entry.

`UpdateEndpoint` (the target, added with the update check) pins where the
one network call is made and which host and release tags it names.
"""
import json
import os
import re
import tomllib
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
APP = ROOT / "apps" / "quest-tracker"
TAURI = APP / "src-tauri"
CAPABILITIES = TAURI / "capabilities"
UI_SRC = APP / "src"

# Directories a walk never enters: build output, dependencies, generated
# schemas. The transport crate is the one place allowed to name sockets.
SKIP_DIRS = {"target", "node_modules", "gen", "dist", ".svelte-kit"}
NET_CRATE = APP / "crates" / "qt-net"

PINNED_CSP = (
    "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; "
    "img-src 'self' data:; font-src 'self' data:; connect-src ipc: http://ipc.localhost"
)
DEFAULT_CAPABILITY_WINDOWS = ["overlay"]
DEFAULT_CAPABILITY_PERMISSIONS = [
    "core:event:default",
    "core:window:default",
    "core:window:allow-start-dragging",
]

FORBIDDEN_PERMISSION_PREFIXES = (
    "http:", "updater:", "websocket:", "upload:", "shell:", "fs:", "process:", "opener:",
)
FORBIDDEN_CRATES = (
    "reqwest", "ureq", "hyper", "isahc", "curl", "attohttpc", "minreq", "surf",
    "tauri-plugin-http", "tauri-plugin-updater", "tauri-plugin-websocket", "tauri-plugin-upload",
)
FORBIDDEN_NPM = (
    "@tauri-apps/plugin-http", "@tauri-apps/plugin-updater",
    "@tauri-apps/plugin-websocket", "@tauri-apps/plugin-upload",
)
RUST_NETWORK_NAMES = [
    ("std::net", re.compile(r"\bstd\s*::\s*net\b")),
    ("TcpStream", re.compile(r"TcpStream")),
    ("UdpSocket", re.compile(r"UdpSocket")),
    ("TcpListener", re.compile(r"TcpListener")),
    ("WinHttp", re.compile(r"WinHttp", re.IGNORECASE)),
    ("InternetOpen", re.compile(r"InternetOpen")),
    ("reqwest", re.compile(r"\breqwest\b")),
    ("ureq", re.compile(r"\bureq\b")),
    ("hyper::", re.compile(r"\bhyper\s*::")),
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


def permission_id(entry) -> str:
    """A capability permission is a string or an object with `identifier`."""
    if isinstance(entry, dict):
        return str(entry.get("identifier", ""))
    return str(entry)


def capability_offences(name: str, capability: dict) -> list[str]:
    """Network-granting permissions or remote URLs in one capability."""
    offences = []
    for entry in capability.get("permissions", []):
        ident = permission_id(entry)
        if ident.startswith(FORBIDDEN_PERMISSION_PREFIXES):
            offences.append(f"{name}: permission {ident!r}")
    if "remote" in capability:
        offences.append(f"{name}: remote {capability['remote']!r}")
    return offences


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


def manifest_offences(name: str, text: str) -> list[str]:
    """HTTP clients or Tauri network plugins a Cargo manifest depends on."""
    offences = []
    for label, table in dependency_tables(tomllib.loads(text)):
        for key, spec in table.items():
            real = spec.get("package", key) if isinstance(spec, dict) else key
            for dep in {key, real}:
                if forbidden_crate(dep):
                    offences.append(f"{name}: [{label}] {dep}")
    return offences


def package_offences(name: str, package: dict) -> list[str]:
    """Tauri network plugins an npm manifest depends on."""
    offences = []
    for kind in ("dependencies", "devDependencies", "optionalDependencies", "peerDependencies"):
        for dep in package.get(kind, {}):
            if dep in FORBIDDEN_NPM:
                offences.append(f"{name}: {kind} {dep}")
    return offences


def text_offences(name: str, text: str, matchers) -> list[str]:
    """Each (label, line) where one of `matchers` occurs in `text`."""
    offences = []
    for number, line in enumerate(text.splitlines(), 1):
        for label, pattern in matchers:
            if pattern.search(line):
                offences.append(f"{name}:{number}: {label}")
    return offences


def is_ui_test(path: Path) -> bool:
    return ".test." in path.name or path.name.startswith("test-support.")


class NoNetworkByDefault(unittest.TestCase):
    """Layer 3 of the privacy invariant: no door to the internet but qt-net."""

    def test_csp_is_pinned(self):
        conf = json.loads((TAURI / "tauri.conf.json").read_text(encoding="utf-8"))
        csp = conf.get("app", {}).get("security", {}).get("csp")
        self.assertEqual(
            csp, PINNED_CSP,
            f"{rel(TAURI / 'tauri.conf.json')}: app.security.csp changed to {csp!r}",
        )

    def test_default_capability_is_unchanged(self):
        path = CAPABILITIES / "default.json"
        capability = json.loads(path.read_text(encoding="utf-8"))
        self.assertEqual(
            capability.get("windows"), DEFAULT_CAPABILITY_WINDOWS,
            f"{rel(path)}: windows {capability.get('windows')!r}",
        )
        self.assertEqual(
            [permission_id(p) for p in capability.get("permissions", [])],
            DEFAULT_CAPABILITY_PERMISSIONS,
            f"{rel(path)}: permissions {capability.get('permissions')!r}",
        )

    def test_no_capability_grants_network(self):
        # Negative control: the matcher sees a synthetic offender.
        synthetic = {
            "windows": ["overlay"],
            "permissions": ["core:event:default", {"identifier": "http:default", "allow": []}],
            "remote": {"urls": ["https://example.com/*"]},
        }
        self.assertEqual(len(capability_offences("synthetic.json", synthetic)), 2)
        for prefix in FORBIDDEN_PERMISSION_PREFIXES:
            self.assertTrue(
                capability_offences("synthetic.json", {"permissions": [prefix + "default"]}),
                f"matcher misses {prefix}",
            )
        self.assertEqual(capability_offences("synthetic.json", {"permissions": ["core:window:default"]}), [])

        files = walk(CAPABILITIES, suffixes={".json", ".toml"})
        self.assertIn(CAPABILITIES / "default.json", files)
        offences = []
        for path in files:
            if path.suffix == ".toml":
                capability = tomllib.loads(path.read_text(encoding="utf-8"))
            else:
                capability = json.loads(path.read_text(encoding="utf-8"))
            offences += capability_offences(rel(path), capability)
        conf = json.loads((TAURI / "tauri.conf.json").read_text(encoding="utf-8"))
        for inline in conf.get("app", {}).get("security", {}).get("capabilities", []):
            if isinstance(inline, dict):
                offences += capability_offences(rel(TAURI / "tauri.conf.json"), inline)
        self.assertEqual(offences, [], "capabilities grant network access:\n" + "\n".join(offences))

    def test_no_manifest_depends_on_an_http_client(self):
        synthetic = """
[dependencies]
serde = "1"
reqwest = { version = "0.12" }
web = { package = "ureq", version = "2" }

[target.'cfg(windows)'.dependencies]
tauri-plugin-updater = "2"

[workspace.dependencies]
hyper-util = "0.1"
"""
        self.assertEqual(
            manifest_offences("synthetic/Cargo.toml", synthetic),
            [
                "synthetic/Cargo.toml: [dependencies] reqwest",
                "synthetic/Cargo.toml: [dependencies] ureq",
                "synthetic/Cargo.toml: [workspace.dependencies] hyper-util",
                "synthetic/Cargo.toml: [target.cfg(windows).dependencies] tauri-plugin-updater",
            ],
        )
        self.assertEqual(manifest_offences("clean/Cargo.toml", '[dependencies]\nserde = "1"\n'), [])

        manifests = walk(APP, names={"Cargo.toml"})
        self.assertIn(APP / "Cargo.toml", manifests)
        self.assertIn(TAURI / "Cargo.toml", manifests)
        self.assertGreaterEqual(len(manifests), 11, [rel(p) for p in manifests])
        offences = []
        for path in manifests:
            offences += manifest_offences(rel(path), path.read_text(encoding="utf-8"))
        self.assertEqual(offences, [], "Cargo manifests depend on network crates:\n" + "\n".join(offences))

    def test_package_json_has_no_network_plugin(self):
        synthetic = {
            "dependencies": {"@tauri-apps/api": "2", "@tauri-apps/plugin-http": "2"},
            "devDependencies": {"@tauri-apps/plugin-websocket": "2"},
        }
        self.assertEqual(
            package_offences("synthetic/package.json", synthetic),
            [
                "synthetic/package.json: dependencies @tauri-apps/plugin-http",
                "synthetic/package.json: devDependencies @tauri-apps/plugin-websocket",
            ],
        )

        path = APP / "package.json"
        offences = package_offences(rel(path), json.loads(path.read_text(encoding="utf-8")))
        self.assertEqual(offences, [], "package.json depends on network plugins:\n" + "\n".join(offences))

    def test_no_rust_source_outside_qt_net_names_a_socket(self):
        synthetic = "use std::net::TcpStream;\nfn f() { let _ = reqwest::get(\"x\"); }\n"
        self.assertEqual(
            text_offences("synthetic.rs", synthetic, RUST_NETWORK_NAMES),
            ["synthetic.rs:1: std::net", "synthetic.rs:1: TcpStream", "synthetic.rs:2: reqwest"],
        )
        for label, _ in RUST_NETWORK_NAMES:
            self.assertTrue(text_offences("synthetic.rs", label, RUST_NETWORK_NAMES), f"matcher misses {label}")
        self.assertEqual(text_offences("clean.rs", "use std::fs;\nlet requested = 1;\n", RUST_NETWORK_NAMES), [])

        sources = [
            path for path in walk(APP, suffixes={".rs"})
            if NET_CRATE not in path.parents
        ]
        self.assertIn(TAURI / "src" / "lib.rs", sources)
        self.assertGreaterEqual(len(sources), 40, [rel(p) for p in sources])
        offences = []
        for path in sources:
            offences += text_offences(rel(path), path.read_text(encoding="utf-8"), RUST_NETWORK_NAMES)
        self.assertEqual(
            offences, [],
            "Rust sources outside crates/qt-net name network code:\n" + "\n".join(offences),
        )

    def test_no_ui_source_calls_the_network(self):
        synthetic = "const r = await fetch('https://x');\nnew WebSocket(u);\nnavigator.sendBeacon(u);\n"
        self.assertEqual(
            text_offences("synthetic.ts", synthetic, UI_NETWORK_NAMES),
            ["synthetic.ts:1: fetch(", "synthetic.ts:2: WebSocket", "synthetic.ts:3: sendBeacon"],
        )
        for label, _ in UI_NETWORK_NAMES:
            probe = label if label.endswith("(") else f"new {label}(u)"
            self.assertTrue(text_offences("synthetic.ts", probe, UI_NETWORK_NAMES), f"matcher misses {label}")
        self.assertEqual(text_offences("clean.ts", "const prefetched = 1;\n", UI_NETWORK_NAMES), [])
        self.assertTrue(is_ui_test(UI_SRC / "app.test.ts"))
        self.assertFalse(is_ui_test(UI_SRC / "main.ts"))

        sources = [path for path in walk(UI_SRC, suffixes=UI_SUFFIXES) if not is_ui_test(path)]
        sources.append(APP / "index.html")
        self.assertIn(UI_SRC / "main.ts", sources)
        self.assertIn(UI_SRC / "App.svelte", sources)
        offences = []
        for path in sources:
            offences += text_offences(rel(path), path.read_text(encoding="utf-8"), UI_NETWORK_NAMES)
        self.assertEqual(offences, [], "UI sources call the network:\n" + "\n".join(offences))


if __name__ == "__main__":
    unittest.main()
