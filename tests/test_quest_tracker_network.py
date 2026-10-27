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
- no `Cargo.toml` under the app depends on an HTTP client, a socket, TLS or
  WebSocket crate, or a Tauri network plugin, and `package.json` on no
  Tauri network plugin;
- the binary's compiled graph (`cargo tree -i`, skipped when cargo is not on
  PATH) holds none of those crates but `socket2` through tokio, Tauri's
  runtime: `Cargo.lock` lists optional ones that are not compiled, and a
  feature turned on in a manifest could link them;
- no Rust source outside `crates/qt-net/` (the transport crate) names a
  socket (std's, tokio's, or Winsock through the `windows` crates),
  WinHTTP, WinINet, urlmon's `URLDownloadToFile`, an HTTP client crate, or a
  webview on a remote page (`WebviewUrl::External`);
- no UI source but the tests, and no root `*.html` entry page of a window,
  calls `fetch(`, `XMLHttpRequest`, `WebSocket`, `EventSource` or
  `sendBeacon`.

Every matcher is first run on a synthetic offender in the same test, so a
matcher that sees nothing cannot pass, and every walk must find files. A
failure names the file and the offending entry.

`UpdateEndpoint` (the target, added with the update check) is layer 2,
reachability, and pins what the one request names:

- the transport's one method, `fetch_releases`, is named at exactly one
  place in non-test code, inside `crates/qt-app/src/updates.rs`, and
  `qt-net`'s `fetch_release_list` only by the glue in `src-tauri/src/lib.rs`;
- `qt_net` is named in Rust code only by `src-tauri/src/lib.rs` and the
  crate itself, a manifest depends on it only in `src-tauri/Cargo.toml`
  (the workspace manifest lists it as a member), and in `Cargo.lock` only
  the `src-tauri` package depends on it;
- the Settings window's capability, `capabilities/settings.json`, is the
  `settings` window's alone, with `core:event:default` (and at most
  `core:window:default`), no other permission and no `remote`;
- `qt-net`'s host, port, scheme and path are `api.github.com`, 443, HTTPS
  and the release list of `release.REPO_URL`'s owner and repository, and
  `updates.rs`'s tag prefix and release-page prefix are
  `release.TAG_PREFIX` and `release.REPO_URL + "/releases/tag/"`.

Rust is read with its comments (and, for names, its strings) blanked and
its `#[cfg(test)]` items removed, so a doc comment or a test fake neither
satisfies nor breaks a check; each reader runs on a synthetic offender and
a synthetic clean source first, as in the baseline.
"""
import fnmatch
import json
import os
import re
import shutil
import subprocess
import sys
import tomllib
import unittest
from pathlib import Path
from urllib.parse import urlparse

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from tools.quest_tracker import release  # noqa: E402

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
    "socket2", "tungstenite", "tokio-tungstenite", "native-tls",
    "tauri-plugin-http", "tauri-plugin-updater", "tauri-plugin-websocket", "tauri-plugin-upload",
)
# The compiled graph of the app's binary, read with `cargo tree`. A
# forbidden crate may be in it only through the dependents listed here:
# `socket2` comes with tokio, Tauri's async runtime, which the app does not
# call for the network (the Rust name check below sees `tokio::net`).
GRAPH_PACKAGE = "quest-tracker"
GRAPH_ALLOWED_DEPENDENTS = {"socket2": {"tokio"}}
# A crate the binary is known to compile, with the app as a direct
# dependent: the graph reader's positive control.
GRAPH_CONTROL = "tauri"
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
    # Raw sockets through the `windows`/`windows-sys` crates
    # (`Win32::Networking::WinSock`), urlmon's one-call download, tokio's
    # sockets, and a webview whose page is a remote URL.
    ("WinSock", re.compile(r"\bWinSock\b|\bWSAStartup\b")),
    ("URLDownloadToFile", re.compile(r"\bURLDownloadToFile")),
    ("tokio::net", re.compile(r"\btokio\s*::\s*net\b")),
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


def ui_entry_pages() -> list[Path]:
    """Every root `*.html` of the app: one entry page per window."""
    return sorted(APP.glob("*.html"))


def lock_forbidden(lock: dict) -> list[str]:
    """`name@version` of each package of a `Cargo.lock` that is a forbidden
    crate, for `cargo tree -i`."""
    return sorted(
        f"{package['name']}@{package['version']}" for package in lock.get("package", [])
        if forbidden_crate(package["name"])
    )


def graph_dependents(stdout: str) -> list[str]:
    """The direct dependents `cargo tree -i <crate> --depth 1 --prefix none`
    printed: every line after the crate's own, by package name. Nothing
    printed (the crate is not compiled) is none."""
    lines = [line.split()[0] for line in stdout.splitlines() if line.strip()]
    return lines[1:]


def graph_offences(spec: str, stdout: str, allowed: dict[str, set[str]]) -> list[str]:
    """Each direct dependent through which a forbidden crate is compiled
    into the binary that `allowed` does not list for it."""
    name = spec.split("@")[0]
    return [
        f"{spec} <- {dependent}" for dependent in graph_dependents(stdout)
        if dependent not in allowed.get(name, set())
    ]


def cargo_tree_dependents(spec: str) -> subprocess.CompletedProcess:
    """`cargo tree` of the binary's compiled graph (normal and build edges,
    the host target), inverted on `spec`, direct dependents only."""
    return subprocess.run(
        [
            "cargo", "tree", "--manifest-path", str(APP / "Cargo.toml"), "--locked", "--offline",
            "-p", GRAPH_PACKAGE, "-e", "normal,build", "-i", spec,
            "--depth", "1", "--prefix", "none", "--format", "{p}",
        ],
        capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=180,
    )


# --- Layer 2: reading Rust without its comments, strings and test items ---

QT_APP = APP / "crates" / "qt-app"
UPDATES_RS = QT_APP / "src" / "updates.rs"
NET_LIB_RS = NET_CRATE / "src" / "lib.rs"
GLUE_RS = TAURI / "src" / "lib.rs"
SETTINGS_CAPABILITY = CAPABILITIES / "settings.json"
SETTINGS_REQUIRED_PERMISSIONS = {"core:event:default"}
SETTINGS_ALLOWED_PERMISSIONS = {"core:event:default", "core:window:default"}
TRANSPORT_METHOD = "fetch_releases"
NET_FUNCTION = "fetch_release_list"
NET_CRATE_NAME = "qt-net"
NET_IDENT = re.compile(r"\bqt_net\b")

_RAW_STRING = re.compile(r'b?r(#*)"')
_CHAR = re.compile(r"b?'(?:\\(?:x[0-9a-fA-F]{2}|u\{[0-9a-fA-F]{1,6}\}|.)|[^\\'\n])'")
_CFG_TEST = re.compile(r"#\s*(!?)\s*\[\s*cfg\s*\(\s*(?:all\s*\(\s*)?test\b")


def _ident_char(c: str) -> bool:
    return c.isalnum() or c == "_"


def blank_rust(text: str, keep_strings: bool = False) -> str:
    """`text` with comments (and strings and char literals unless
    `keep_strings`) turned to spaces. Newlines and offsets are kept, so a
    position or line number in the result is one in `text`."""
    out = list(text)

    def blank(a: int, b: int) -> None:
        for k in range(a, b):
            if out[k] != "\n":
                out[k] = " "

    i, n = 0, len(text)
    while i < n:
        c = text[i]
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
        elif c in "rb" and (i == 0 or not _ident_char(text[i - 1])) and _RAW_STRING.match(text, i):
            m = _RAW_STRING.match(text, i)
            close = '"' + m.group(1)
            end = text.find(close, m.end())
            end = n if end < 0 else end + len(close)
            if not keep_strings:
                blank(i, end)
            i = end
        elif c == '"' or (c == "b" and text.startswith('b"', i) and (i == 0 or not _ident_char(text[i - 1]))):
            j = i + (2 if c == "b" else 1)
            while j < n and text[j] != '"':
                j += 2 if text[j] == "\\" else 1
            end = min(j + 1, n)
            if not keep_strings:
                blank(i, end)
            i = end
        elif c in "'b" and (i == 0 or not _ident_char(text[i - 1])) and _CHAR.match(text, i):
            end = _CHAR.match(text, i).end()
            if not keep_strings:
                blank(i, end)
            i = end
        else:
            i += 1
    return "".join(out)


def cfg_test_spans(code: str) -> list[tuple[int, int]]:
    """The spans of the items a `#[cfg(test)]` (or `#[cfg(all(test, ...))]`)
    applies to, attribute included, in comment- and string-blanked `code`.
    An item ends at its closing brace, or at a `;` outside any bracket."""
    spans: list[tuple[int, int]] = []
    for m in _CFG_TEST.finditer(code):
        if spans and m.start() < spans[-1][1]:
            continue
        if m.group(1):  # `#![cfg(test)]`: the whole file or module
            spans.append((m.start(), len(code)))
            break
        k = code.find("]", m.end()) + 1
        depth = 0
        while k < len(code):
            c = code[k]
            if c in "{([":
                depth += 1
            elif c in "})]":
                depth -= 1
                if depth == 0 and c == "}":
                    break
            elif c == ";" and depth == 0:
                break
            k += 1
        spans.append((m.start(), min(k + 1, len(code))))
    return spans


def non_test_rust(text: str) -> tuple[str, str]:
    """(code, code with its strings): `text` without comments and without
    its `#[cfg(test)]` items, the first with strings blanked too."""
    code = blank_rust(text)
    with_strings = blank_rust(text, keep_strings=True)
    spans = cfg_test_spans(code)

    def cut(s: str) -> str:
        parts, last = [], 0
        for a, b in spans:
            parts.append(s[last:a])
            parts.append(re.sub(r"[^\n]", " ", s[a:b]))
            last = b
        parts.append(s[last:])
        return "".join(parts)

    return cut(code), cut(with_strings)


def line_of(text: str, pos: int) -> int:
    return text.count("\n", 0, pos) + 1


def name_uses(name: str, code: str, ident: str) -> list[str]:
    """Each `name:line` where `ident` occurs in `code` other than as the
    name of a `fn` being declared: a call, or a path passed as a value."""
    uses = []
    for m in re.finditer(r"\b" + re.escape(ident) + r"\b", code):
        if re.search(r"\bfn\s+$", code[:m.start()][-40:]):
            continue
        uses.append(f"{name}:{line_of(code, m.start())}")
    return uses


def is_test_source(path: Path) -> bool:
    """A Rust file of a crate's `tests/` (integration tests)."""
    return "tests" in path.relative_to(APP).parts[:-1]


def rust_const(text: str, const: str, ty: str) -> list[str]:
    """Each value `const <const>: <ty> = <value>;` defines in comment-blanked
    non-test `text`: a string's contents, or a number's digits."""
    if ty == "str":
        pattern = (
            r"\bconst\s+" + re.escape(const)
            + r"\s*:\s*&\s*(?:'static\s+)?str\s*=\s*\"((?:[^\"\\]|\\.)*)\"\s*;"
        )
    else:
        pattern = r"\bconst\s+" + re.escape(const) + r"\s*:\s*" + re.escape(ty) + r"\s*=\s*([0-9_]+)\s*;"
    return [m.group(1).replace("_", "") if ty != "str" else m.group(1) for m in re.finditer(pattern, text)]


def repo_of(url: str) -> str:
    """`owner/repo` of a `https://github.com/<owner>/<repo>` URL."""
    parsed = urlparse(url)
    parts = [p for p in parsed.path.split("/") if p]
    if parsed.scheme != "https" or parsed.netloc != "github.com" or len(parts) != 2:
        raise ValueError(f"not a GitHub repository URL: {url!r}")
    return "/".join(parts)


def endpoint_offences(net_name: str, net_text: str, updates_name: str, updates_text: str,
                      repo_url: str, tag_prefix: str) -> list[str]:
    """Where `qt-net`'s endpoint constants or `updates.rs`'s release
    constants disagree with the release contract (`repo_url`, `tag_prefix`).
    Each constant must be defined exactly once in non-test code."""
    offences = []
    _, net = non_test_rust(net_text)
    _, updates = non_test_rust(updates_text)

    def one(name: str, text: str, const: str, ty: str):
        values = rust_const(text, const, ty)
        if len(values) != 1:
            offences.append(f"{name}: const {const} defined {len(values)} times outside tests")
            return None
        return values[0]

    repo = repo_of(repo_url)
    host = one(net_name, net, "HOST", "str")
    if host is not None and host != "api.github.com":
        offences.append(f"{net_name}: HOST {host!r}, not 'api.github.com'")
    port = one(net_name, net, "PORT", "u16")
    if port is not None and port != "443":
        offences.append(f"{net_name}: PORT {port}, not 443")
    scheme = one(net_name, net, "SCHEME", "str")
    if scheme is not None and scheme != "https":
        offences.append(f"{net_name}: SCHEME {scheme!r}, not 'https'")
    path = one(net_name, net, "PATH", "str")
    releases = f"/repos/{repo}/releases"
    if path is not None and not (path == releases or path.startswith(releases + "?")):
        offences.append(f"{net_name}: PATH {path!r} is not {releases!r} (of {repo_url})")
    prefix = one(updates_name, updates, "TAG_PREFIX", "str")
    if prefix is not None and prefix != tag_prefix:
        offences.append(f"{updates_name}: TAG_PREFIX {prefix!r}, not release.TAG_PREFIX {tag_prefix!r}")
    page = one(updates_name, updates, "RELEASE_PAGE_PREFIX", "str")
    if page is not None and page != repo_url + "/releases/tag/":
        offences.append(
            f"{updates_name}: RELEASE_PAGE_PREFIX {page!r}, not release.REPO_URL + '/releases/tag/'"
        )
    return offences


def settings_capability_offences(name: str, capability: dict) -> list[str]:
    """Where the Settings window's capability is more than `settings`'s
    window listening for events."""
    offences = capability_offences(name, capability)
    if capability.get("identifier") != "settings":
        offences.append(f"{name}: identifier {capability.get('identifier')!r}")
    if capability.get("windows") != ["settings"]:
        offences.append(f"{name}: windows {capability.get('windows')!r}")
    if "webviews" in capability:
        offences.append(f"{name}: webviews {capability['webviews']!r}")
    permissions = {permission_id(p) for p in capability.get("permissions", [])}
    for missing in sorted(SETTINGS_REQUIRED_PERMISSIONS - permissions):
        offences.append(f"{name}: missing permission {missing!r}")
    for extra in sorted(permissions - SETTINGS_ALLOWED_PERMISSIONS):
        offences.append(f"{name}: permission {extra!r}")
    return offences


def depends_on_net(manifest: dict) -> list[str]:
    """The dependency tables of a Cargo manifest that take `qt-net`, by
    name, by `package` or by a path ending in `qt-net`."""
    tables = []
    for label, table in dependency_tables(manifest):
        for key, spec in table.items():
            names = {key}
            if isinstance(spec, dict):
                names.add(spec.get("package", key))
                if "path" in spec:
                    names.add(Path(spec["path"]).name)
            if NET_CRATE_NAME in names:
                tables.append(label)
    return tables


def lock_dependents(lock: dict, crate: str) -> list[str]:
    """The packages of a `Cargo.lock` whose dependencies include `crate`."""
    return sorted(
        package["name"] for package in lock.get("package", [])
        if any(dep.split()[0] == crate for dep in package.get("dependencies", []))
    )


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
        sockets = '[dependencies]\nsocket2 = "0.6"\ntungstenite = "0.27"\ntokio-tungstenite = "0.27"\nnative-tls = "0.2"\n'
        self.assertEqual(
            manifest_offences("sockets/Cargo.toml", sockets),
            [f"sockets/Cargo.toml: [dependencies] {dep}"
             for dep in ("socket2", "tungstenite", "tokio-tungstenite", "native-tls")],
        )

        manifests = walk(APP, names={"Cargo.toml"})
        self.assertIn(APP / "Cargo.toml", manifests)
        self.assertIn(TAURI / "Cargo.toml", manifests)
        self.assertGreaterEqual(len(manifests), 11, [rel(p) for p in manifests])
        offences = []
        for path in manifests:
            offences += manifest_offences(rel(path), path.read_text(encoding="utf-8"))
        self.assertEqual(offences, [], "Cargo manifests depend on network crates:\n" + "\n".join(offences))

    def test_compiled_graph_has_no_network_crate(self):
        # The manifests name direct dependencies only; `Cargo.lock` already
        # lists optional clients (tauri's reqwest) that are not compiled.
        # Only the compiled graph tells a feature that links one apart.
        lock = tomllib.loads(
            '[[package]]\nname = "reqwest"\nversion = "0.13.5"\n'
            '[[package]]\nname = "hyper-util"\nversion = "0.1.21"\n'
            '[[package]]\nname = "serde"\nversion = "1.0.0"\n'
        )
        self.assertEqual(lock_forbidden(lock), ["hyper-util@0.1.21", "reqwest@0.13.5"])
        allowed = {"socket2": {"tokio"}}
        self.assertEqual(graph_offences("reqwest@0.13.5", "", allowed), [])
        self.assertEqual(graph_offences("socket2@0.6.5", "socket2 v0.6.5\ntokio v1.54.1\n", allowed), [])
        self.assertEqual(
            graph_offences("socket2@0.6.5", "socket2 v0.6.5\ntokio v1.54.1\nqt-app v0.1.0 (C:\\x)\n", allowed),
            ["socket2@0.6.5 <- qt-app"],
        )
        self.assertEqual(
            graph_offences("reqwest@0.13.5", "reqwest v0.13.5\ntauri v2.12.3\n", allowed),
            ["reqwest@0.13.5 <- tauri"],
        )

        if shutil.which("cargo") is None:
            self.skipTest("cargo is not on PATH: the compiled graph was not read")
        # Positive control: the reader sees a crate the binary does compile.
        control = cargo_tree_dependents(GRAPH_CONTROL)
        self.assertEqual(control.returncode, 0, control.stderr)
        self.assertIn(GRAPH_PACKAGE, graph_dependents(control.stdout), control.stdout)

        specs = lock_forbidden(tomllib.loads((APP / "Cargo.lock").read_text(encoding="utf-8")))
        offences = []
        for spec in specs:
            tree = cargo_tree_dependents(spec)
            self.assertEqual(tree.returncode, 0, f"cargo tree -i {spec}: {tree.stderr}")
            offences += graph_offences(spec, tree.stdout, GRAPH_ALLOWED_DEPENDENTS)
        self.assertEqual(
            offences, [],
            f"network crates compiled into {GRAPH_PACKAGE}:\n" + "\n".join(offences),
        )

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
        doors = (
            "use windows::Win32::Networking::WinSock::{connect, socket};\n"
            "unsafe { windows_sys::Win32::Networking::WinSock::WSAStartup(0x202, &mut data) };\n"
            "URLDownloadToFileW(None, url, path, 0, None);\n"
            "let addrs = tokio::net::lookup_host(host).await;\n"
            "WebviewWindowBuilder::new(&app, \"w\", WebviewUrl::External(url));\n"
        )
        self.assertEqual(
            text_offences("doors.rs", doors, RUST_NETWORK_NAMES),
            ["doors.rs:1: WinSock", "doors.rs:2: WinSock", "doors.rs:3: URLDownloadToFile",
             "doors.rs:4: tokio::net", "doors.rs:5: WebviewUrl::External"],
        )
        self.assertEqual(
            text_offences("clean.rs", "use tokio::sync::mpsc;\nWebviewUrl::App(page.into());\n", RUST_NETWORK_NAMES),
            [],
        )

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

        # Every window's entry page is read, not one named here: an inline
        # script in a page the walk missed would pass unseen.
        self.assertEqual(
            text_offences("synthetic.html", '<body>\n<script>fetch("https://x")</script>\n', UI_NETWORK_NAMES),
            ["synthetic.html:2: fetch("],
        )
        pages = ui_entry_pages()
        self.assertIn(APP / "index.html", pages)
        self.assertIn(APP / "settings.html", pages)
        sources = [path for path in walk(UI_SRC, suffixes=UI_SUFFIXES) if not is_ui_test(path)] + pages
        self.assertIn(UI_SRC / "main.ts", sources)
        self.assertIn(UI_SRC / "App.svelte", sources)
        self.assertIn(UI_SRC / "settings" / "main.ts", sources)
        offences = []
        for path in sources:
            offences += text_offences(rel(path), path.read_text(encoding="utf-8"), UI_NETWORK_NAMES)
        self.assertEqual(offences, [], "UI sources call the network:\n" + "\n".join(offences))


class UpdateEndpoint(unittest.TestCase):
    """Layer 2 of the privacy invariant: the one request is made in one
    place, reached by one crate, and names the release contract's endpoint."""

    def test_rust_reader_skips_comments_strings_and_test_items(self):
        source = (
            "// fetch_releases in a comment\n"
            "/* nested /* fetch_releases */ */\n"
            "const A: &str = \"fetch_releases }\";\n"
            "const C: char = '{';\n"
            "fn g<'a>(x: &'a str) {}\n"
            "#[cfg(test)]\n"
            "mod tests {\n"
            "    fn f() { t.fetch_releases(); let s = \"}\"; }\n"
            "}\n"
            "#[cfg(all(test, windows))]\n"
            "const B: [u8; 2] = [0; 2];\n"
            "fn real() { t.fetch_releases() }\n"
        )
        code, with_strings = non_test_rust(source)
        self.assertEqual(len(code), len(source))
        self.assertEqual(code.count("\n"), source.count("\n"))
        self.assertEqual(name_uses("s.rs", code, TRANSPORT_METHOD), ["s.rs:12"])
        self.assertEqual(name_uses("s.rs", blank_rust(source), TRANSPORT_METHOD), ["s.rs:8", "s.rs:12"])
        self.assertEqual(rust_const(with_strings, "A", "str"), ["fetch_releases }"])
        self.assertEqual(rust_const(with_strings, "B", "[u8; 2]"), [])
        self.assertIn("fn g<'a>(x: &'a str)", code)
        # A declaration is not a use; a path passed as a value is.
        self.assertEqual(name_uses("d.rs", "trait T { fn fetch_releases(&self); }\n", TRANSPORT_METHOD), [])
        self.assertEqual(
            name_uses("p.rs", "let f = T::fetch_releases;\n", TRANSPORT_METHOD), ["p.rs:1"],
        )

    def test_the_transport_is_called_in_one_place(self):
        sources = [p for p in walk(APP, suffixes={".rs"}) if not is_test_source(p)]
        self.assertIn(UPDATES_RS, sources)
        self.assertIn(GLUE_RS, sources)
        self.assertNotIn(QT_APP / "tests" / "updates.rs", sources)
        self.assertTrue(is_test_source(QT_APP / "tests" / "updates.rs"))
        self.assertFalse(is_test_source(UPDATES_RS))
        method_uses, net_uses = [], []
        for path in sources:
            code, _ = non_test_rust(path.read_text(encoding="utf-8"))
            method_uses += name_uses(rel(path), code, TRANSPORT_METHOD)
            if NET_CRATE not in path.parents:
                net_uses += name_uses(rel(path), code, NET_FUNCTION)
        self.assertEqual(
            [use.rsplit(":", 1)[0] for use in method_uses], [rel(UPDATES_RS)],
            "the transport's method is called outside updates.rs, or more than once:\n"
            + "\n".join(method_uses),
        )
        self.assertEqual(
            [use.rsplit(":", 1)[0] for use in net_uses], [rel(GLUE_RS)],
            f"qt-net's {NET_FUNCTION} is called outside the glue, or more than once:\n"
            + "\n".join(net_uses),
        )

    def test_qt_net_is_reached_only_by_the_glue(self):
        # Negative controls: each reader sees a synthetic offender.
        self.assertEqual(
            name_uses("s.rs", blank_rust("use qt_net::fetch_release_list; // qt_net\nlet s = \"qt_net\";\n"),
                      "qt_net"),
            ["s.rs:1"],
        )
        synthetic = """
[dependencies]
serde = "1"
web = { path = "../crates/qt-net" }

[dev-dependencies]
net = { package = "qt-net", path = "../x" }

[target.'cfg(windows)'.dependencies]
qt-net = { path = "../qt-net" }
"""
        self.assertEqual(
            depends_on_net(tomllib.loads(synthetic)),
            ["dependencies", "dev-dependencies", "target.cfg(windows).dependencies"],
        )
        self.assertEqual(depends_on_net(tomllib.loads('[dependencies]\nqt-app = { path = "../qt-app" }\n')), [])
        lock = tomllib.loads(
            '[[package]]\nname = "a"\ndependencies = ["qt-net 0.1.0", "serde"]\n'
            '[[package]]\nname = "b"\ndependencies = ["qt-net"]\n'
            '[[package]]\nname = "qt-netx"\n'
            '[[package]]\nname = "c"\ndependencies = ["qt-netx"]\n'
        )
        self.assertEqual(lock_dependents(lock, NET_CRATE_NAME), ["a", "b"])

        # Rust code, tests included: only the glue and the crate itself.
        sources = walk(APP, suffixes={".rs"})
        self.assertIn(NET_LIB_RS, sources)
        naming = sorted({
            rel(path) for path in sources
            if NET_CRATE not in path.parents
            and NET_IDENT.search(blank_rust(path.read_text(encoding="utf-8")))
        })
        self.assertEqual(naming, [rel(GLUE_RS)], f"Rust code naming qt_net: {naming}")

        # Manifests: only src-tauri depends on it; the workspace lists it.
        manifests = walk(APP, names={"Cargo.toml"})
        self.assertIn(NET_CRATE / "Cargo.toml", manifests)
        dependents = sorted(
            rel(path) for path in manifests
            if depends_on_net(tomllib.loads(path.read_text(encoding="utf-8")))
        )
        self.assertEqual(dependents, [rel(TAURI / "Cargo.toml")], f"manifests taking qt-net: {dependents}")
        workspace = tomllib.loads((APP / "Cargo.toml").read_text(encoding="utf-8"))["workspace"]
        self.assertIn("crates/qt-net", workspace.get("members", []))
        net_manifest = tomllib.loads((NET_CRATE / "Cargo.toml").read_text(encoding="utf-8"))
        net_deps = [key for _, table in dependency_tables(net_manifest) for key in table]
        self.assertEqual(
            [dep for dep in net_deps if dep.startswith("qt-")], [],
            f"qt-net depends on another qt-* crate: {net_deps}",
        )

        # The lock: the src-tauri package is the one dependent.
        glue_package = tomllib.loads((TAURI / "Cargo.toml").read_text(encoding="utf-8"))["package"]["name"]
        cargo_lock = tomllib.loads((APP / "Cargo.lock").read_text(encoding="utf-8"))
        self.assertIn(NET_CRATE_NAME, [p["name"] for p in cargo_lock.get("package", [])])
        self.assertEqual(lock_dependents(cargo_lock, NET_CRATE_NAME), [glue_package])

    def test_settings_window_capability(self):
        clean = {
            "identifier": "settings",
            "windows": ["settings"],
            "permissions": ["core:event:default"],
        }
        self.assertEqual(settings_capability_offences("synthetic.json", clean), [])
        self.assertEqual(
            settings_capability_offences(
                "synthetic.json", dict(clean, permissions=["core:event:default", "core:window:default"]),
            ),
            [],
        )
        self.assertEqual(
            settings_capability_offences(
                "synthetic.json",
                {
                    "identifier": "other",
                    "windows": ["settings", "overlay"],
                    "permissions": [{"identifier": "opener:default"}, "core:webview:default"],
                    "remote": {"urls": ["https://example.com/*"]},
                },
            ),
            [
                "synthetic.json: permission 'opener:default'",
                "synthetic.json: remote {'urls': ['https://example.com/*']}",
                "synthetic.json: identifier 'other'",
                "synthetic.json: windows ['settings', 'overlay']",
                "synthetic.json: missing permission 'core:event:default'",
                "synthetic.json: permission 'core:webview:default'",
                "synthetic.json: permission 'opener:default'",
            ],
        )

        capability = json.loads(SETTINGS_CAPABILITY.read_text(encoding="utf-8"))
        offences = settings_capability_offences(rel(SETTINGS_CAPABILITY), capability)
        self.assertEqual(offences, [], "the Settings window's capability:\n" + "\n".join(offences))

        # No other capability reaches the settings window.
        files = walk(CAPABILITIES, suffixes={".json", ".toml"})
        self.assertIn(SETTINGS_CAPABILITY, files)
        reaching = []
        for path in files:
            if path.suffix == ".toml":
                other = tomllib.loads(path.read_text(encoding="utf-8"))
            else:
                other = json.loads(path.read_text(encoding="utf-8"))
            if any(fnmatch.fnmatchcase("settings", w) for w in other.get("windows", [])):
                reaching.append(rel(path))
        self.assertEqual(reaching, [rel(SETTINGS_CAPABILITY)], f"capabilities for the settings window: {reaching}")

    def test_endpoint_matches_the_release_contract(self):
        self.assertEqual(repo_of(release.REPO_URL), "S-Borkowski/ExilePact2")
        with self.assertRaises(ValueError):
            repo_of("https://example.com/S-Borkowski/ExilePact2")
        repo_url, prefix = "https://github.com/owner/repo", "tool-v"
        net = (
            '//! pub const HOST: &str = "doc.example";\n'
            'pub const HOST: &str = "api.github.com";\n'
            "pub const PORT: u16 = 443;\n"
            'pub const SCHEME: &str = "https";\n'
            'pub const PATH: &str = "/repos/owner/repo/releases?per_page=30";\n'
            "#[cfg(test)]\nmod tests { const HOST: &str = \"127.0.0.1\"; }\n"
        )
        updates = (
            'pub const TAG_PREFIX: &str = "tool-v";\n'
            'pub const RELEASE_PAGE_PREFIX: &str = "https://github.com/owner/repo/releases/tag/";\n'
        )
        self.assertEqual(endpoint_offences("net.rs", net, "updates.rs", updates, repo_url, prefix), [])
        wrong = {
            "net.rs: HOST 'evil.example', not 'api.github.com'":
                (net.replace('"api.github.com"', '"evil.example"'), updates),
            "net.rs: PORT 80, not 443": (net.replace("443", "80"), updates),
            "net.rs: SCHEME 'http', not 'https'": (net.replace('"https"', '"http"'), updates),
            "net.rs: PATH '/repos/other/repo/releases?per_page=30' is not '/repos/owner/repo/releases' "
            "(of https://github.com/owner/repo)":
                (net.replace("/repos/owner/", "/repos/other/"), updates),
            "net.rs: const HOST defined 0 times outside tests":
                (net.replace('pub const HOST: &str = "api.github.com";\n', ""), updates),
            "net.rs: const PORT defined 2 times outside tests":
                (net + "const PORT: u16 = 443;\n", updates),
            "updates.rs: TAG_PREFIX 'other-v', not release.TAG_PREFIX 'tool-v'":
                (net, updates.replace('"tool-v"', '"other-v"')),
            "updates.rs: RELEASE_PAGE_PREFIX 'https://github.com/owner/fork/releases/tag/', "
            "not release.REPO_URL + '/releases/tag/'":
                (net, updates.replace("owner/repo/", "owner/fork/")),
        }
        for expected, (net_text, updates_text) in wrong.items():
            self.assertEqual(
                endpoint_offences("net.rs", net_text, "updates.rs", updates_text, repo_url, prefix),
                [expected],
            )

        offences = endpoint_offences(
            rel(NET_LIB_RS), NET_LIB_RS.read_text(encoding="utf-8"),
            rel(UPDATES_RS), UPDATES_RS.read_text(encoding="utf-8"),
            release.REPO_URL, release.TAG_PREFIX,
        )
        self.assertEqual(offences, [], "the endpoint disagrees with the release contract:\n" + "\n".join(offences))


if __name__ == "__main__":
    unittest.main()
