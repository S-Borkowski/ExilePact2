"""The shared update workspace, `apps/shared/`, has one door to the network,
names the releases the release tooling makes, and checks them with the
owner's key.

The three apps take their update code from this workspace: `ep-net`, the
only network code of any of them, and `ep-update`, the check, the signature
check and the swap of the running exe. Each app's own guard
(`tests/test_quest_tracker_network.py`, `tests/test_waystone_roller_guard.py`,
`tests/test_crafting_helper_guard.py`) pins what that app reaches; this file
pins what the shared crates are, by reading files only (no build, no
network), because a mistake here reaches every app at once and an install
runs whatever these crates accept.

`OneNetworkCrate`:

- no Rust source under `apps/shared/` outside `crates/ep-net/` names a
  socket (std's, tokio's, or Winsock), WinHTTP, WinINet, urlmon's
  `URLDownloadToFile`, an HTTP client crate or a remote webview page;
- none of it is `unsafe` (comments and strings blanked): `ep-update` has no
  reason to be, and the transport's `unsafe` stays in the transport;
- no manifest of the shared workspace or of any app's `src-tauri`, nor
  the quest tracker's workspace manifest, turns on a `Win32_Networking_*`
  feature but `ep-net`'s.

`ReleaseContract`:

- `ep-net`'s release-list path and download path prefix are those of the
  repository the three `release.py` modules name (`REPO_URL`), and its
  three tag prefixes are their `TAG_PREFIX`es, as are those of
  `ep-update`'s `QUEST_TRACKER`, `WAYSTONE_ROLLER` and `CRAFTING_HELPER`
  products;
- the two crates agree on each product's exe name and on the signature
  suffix, `.minisig`, the asset the release workflows publish beside each
  exe;
- each product's exe name is the asset its release workflow uploads, as
  GitHub stores an asset name (a run of other characters, a space
  included, becomes one dot): the name its Collect step copies the built
  exe (`mainBinaryName` and `.exe`) to, or the built name when it keeps
  it. The crafting helper builds `Crafting Helper.exe` and fetches
  `Crafting.Helper.exe`; the quest tracker builds `Quest Tracker.exe` and
  its workflow copies it to `quest-tracker.exe`, the name 0.1.x installs
  fetch; the waystone roller keeps its kebab name;
- a download's one redirect may go to exactly two hosts,
  `release-assets.githubusercontent.com` and `objects.githubusercontent.com`.

`KeyAndPins`:

- `ep-update`'s `RELEASE_PUBLIC_KEY` is defined once, as a `&str` (never an
  `Option`: there is no build without a key), holding a minisign public key
  line (base64 of `Ed`, an 8-byte key ID and a 32-byte key) that is not the
  test fixture's key, nor another line of the fixture's key ID;
- no app's glue (`src-tauri`) names the key source,
  `UpdateConfig::public_key`, outside its tests: the built-in key is the
  only one a shipped build checks against;
- every registry package of the shared `Cargo.lock` is in the quest
  tracker's lock at the same version, source and checksum, and its local
  packages are `ep-net` and `ep-update`, which the quest tracker's lock
  lists as local too;
- every `[workspace.dependencies]` pin of the shared workspace is exact
  (`=`) and equals the quest tracker's, except `minisign-verify =0.2.5`,
  the one crate no app carried before, at the version the quest
  tracker's lock holds; the members take every dependency with
  `workspace = true` and none depends on a `qt-*`, `wr-*` or `ch-*` crate.

Every matcher runs on a synthetic offender (and a clean source) first, so a
matcher that sees nothing cannot pass, and every walk must find files. Rust
is read with the quest tracker guard's reader (comments and, where a name is
looked for, strings blanked; `#[cfg(test)]` items removed), imported from
its module rather than copied.
"""
import base64
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
from tests import test_quest_tracker_network as qtnet  # noqa: E402
from tools.quest_tracker import release as qt_release  # noqa: E402
from tools.waystone_roller import release as wr_release  # noqa: E402
from tools.crafting_helper import release as ch_release  # noqa: E402

SHARED = qtnet.SHARED
NET_CRATE = qtnet.NET_CRATE
EP_UPDATE = qtnet.EP_UPDATE
NET_LIB_RS = NET_CRATE / "src" / "lib.rs"
VERSION_RS = EP_UPDATE / "src" / "version.rs"
CHECK_RS = EP_UPDATE / "src" / "check.rs"
UPDATER_RS = EP_UPDATE / "src" / "updater.rs"
RELEASE_KEY_RS = EP_UPDATE / "src" / "release_key.rs"
FIXTURE_KEY = SHARED / "fixtures" / "signing" / "test-key.pub"
SHARED_MANIFEST = SHARED / "Cargo.toml"
SHARED_LOCK = SHARED / "Cargo.lock"
MEMBERS = {"ep-net": NET_CRATE / "Cargo.toml", "ep-update": EP_UPDATE / "Cargo.toml"}

QT = ROOT / "apps" / "quest-tracker"
WR = ROOT / "apps" / "waystone-roller"
CH = ROOT / "apps" / "crafting-helper"
QT_MANIFEST = QT / "Cargo.toml"
QT_LOCK = QT / "Cargo.lock"
GLUES = [QT / "src-tauri", WR / "src-tauri", CH / "src-tauri"]
# The prefix of every app's own crates, none of which a shared crate takes.
APP_CRATE_PREFIXES = ("qt-", "wr-", "ch-")

# The local packages of the shared lock: its two members.
SHARED_LOCAL_PACKAGES = {"ep-net", "ep-update"}
# The one registry crate the shared workspace pins that the quest tracker's
# workspace does not: the signature check.
NEW_CRATE, NEW_CRATE_PIN = "minisign-verify", "=0.2.5"
# The only hosts a download's one redirect may lead to.
REDIRECT_HOSTS = ["release-assets.githubusercontent.com", "objects.githubusercontent.com"]
SIGNATURE_SUFFIX = ".minisig"
# Each product: the release module that cuts its releases, `ep-net`'s
# constants for it and `ep-update`'s `Product` constant.
PRODUCTS = [
    ("quest tracker", qt_release, "QUEST_TRACKER_TAG_PREFIX", "QUEST_TRACKER_EXE", "QUEST_TRACKER"),
    ("waystone roller", wr_release, "WAYSTONE_ROLLER_TAG_PREFIX", "WAYSTONE_ROLLER_EXE", "WAYSTONE_ROLLER"),
    ("crafting helper", ch_release, "CRAFTING_HELPER_TAG_PREFIX", "CRAFTING_HELPER_EXE", "CRAFTING_HELPER"),
]

UNSAFE = re.compile(r"\bunsafe\b")
NETWORKING_FEATURE = "Win32_Networking"
KEY_SOURCE = re.compile(r"\bpublic_key\b")
# A minisign public key: the algorithm `Ed`, an 8-byte key ID, a 32-byte
# Ed25519 key.
KEY_BYTES, KEY_ALGORITHM = 42, b"Ed"


def rel(path: Path) -> str:
    return qtnet.rel(path)


def shared_rust() -> list[Path]:
    """Every Rust file of the shared workspace, tests included."""
    return qtnet.walk(SHARED, suffixes={".rs"})


def glue_rust() -> list[Path]:
    """Every Rust file of every app's `src-tauri`."""
    return [path for glue in GLUES for path in qtnet.walk(glue, suffixes={".rs"})]


def unsafe_offences(name: str, text: str) -> list[str]:
    """Each `name:line` where `text`, comments and strings blanked, uses
    `unsafe` (`#![forbid(unsafe_code)]` is not a use)."""
    code = qtnet.blank_rust(text)
    return [f"{name}:{qtnet.line_of(code, m.start())}" for m in UNSAFE.finditer(code)]


def networking_features(name: str, text: str) -> list[str]:
    """Each `Win32_Networking*` feature a Cargo manifest turns on."""
    offences = []
    for label, table in qtnet.dependency_tables(tomllib.loads(text)):
        for dep, spec in table.items():
            features = spec.get("features", []) if isinstance(spec, dict) else []
            for feature in features:
                if feature.startswith(NETWORKING_FEATURE):
                    offences.append(f"{name}: [{label}] {dep} feature {feature}")
    return offences


def key_source_offences(name: str, text: str) -> list[str]:
    """Each `name:line` where non-test `text`, comments and strings blanked,
    names `public_key`: the key source of `UpdateConfig`."""
    code, _ = qtnet.non_test_rust(text)
    return [f"{name}:{qtnet.line_of(code, m.start())}" for m in KEY_SOURCE.finditer(code)]


def str_arrays(text: str, const: str) -> list[list[str]]:
    """The strings of each `const <const>: [&str; N] = [...];` in
    comment-blanked non-test `text`."""
    pattern = (
        r"\bconst\s+" + re.escape(const)
        + r"\s*:\s*\[\s*&\s*(?:'static\s+)?str\s*;\s*[0-9_]+\s*\]\s*=\s*\[(.*?)\]\s*;"
    )
    return [
        re.findall(r'"((?:[^"\\]|\\.)*)"', m.group(1))
        for m in re.finditer(pattern, text, re.DOTALL)
    ]


def product_fields(text: str, const: str, field: str) -> list[str]:
    """Each `field` a `const <const>: Product = Product { ... }` defines in
    comment-blanked non-test `text`."""
    pattern = (
        r"\bconst\s+" + re.escape(const) + r"\s*:\s*Product\s*=\s*Product\s*\{[^}]*?"
        r"\b" + re.escape(field) + r"\s*:\s*\"((?:[^\"\\]|\\.)*)\""
    )
    return [m.group(1) for m in re.finditer(pattern, text)]


def contract_offences(net_name: str, net_text: str, product_name: str, product_text: str,
                      products: list[tuple[str, str, str, str, str]]) -> list[str]:
    """Where `ep-net` (`net_text`) or `ep-update`'s products (`product_text`)
    disagree with the release contract. `products` holds, per product, its
    label, its release module's `REPO_URL` and `TAG_PREFIX`, and the names
    of `ep-net`'s tag-prefix and exe constants and `ep-update`'s product.
    Each constant must be defined exactly once outside tests."""
    offences = []
    _, net = qtnet.non_test_rust(net_text)
    _, product = qtnet.non_test_rust(product_text)

    def one(name: str, values: list, what: str):
        if len(values) != 1:
            offences.append(f"{name}: {what} defined {len(values)} times outside tests")
            return None
        return values[0]

    path = one(net_name, qtnet.rust_const(net, "PATH", "str"), "const PATH")
    prefix = one(net_name, qtnet.rust_const(net, "DOWNLOAD_PATH_PREFIX", "str"), "const DOWNLOAD_PATH_PREFIX")
    for label, repo_url, tag_prefix, tag_const, exe_const, product_const in products:
        repo = qtnet.repo_of(repo_url)
        releases = f"/repos/{repo}/releases"
        if path is not None and not (path == releases or path.startswith(releases + "?")):
            offences.append(f"{net_name}: PATH {path!r} is not {releases!r} (the {label}'s {repo_url})")
        downloads = f"/{repo}/releases/download/"
        if prefix is not None and prefix != downloads:
            offences.append(
                f"{net_name}: DOWNLOAD_PATH_PREFIX {prefix!r} is not {downloads!r} (the {label}'s {repo_url})"
            )
        net_tag = one(net_name, qtnet.rust_const(net, tag_const, "str"), f"const {tag_const}")
        if net_tag is not None and net_tag != tag_prefix:
            offences.append(f"{net_name}: {tag_const} {net_tag!r}, not the {label}'s TAG_PREFIX {tag_prefix!r}")
        product_tag = one(product_name, product_fields(product, product_const, "tag_prefix"),
                          f"{product_const}.tag_prefix")
        if product_tag is not None and product_tag != tag_prefix:
            offences.append(
                f"{product_name}: {product_const}.tag_prefix {product_tag!r}, "
                f"not the {label}'s TAG_PREFIX {tag_prefix!r}"
            )
        net_exe = one(net_name, qtnet.rust_const(net, exe_const, "str"), f"const {exe_const}")
        product_exe = one(product_name, product_fields(product, product_const, "exe_name"),
                          f"{product_const}.exe_name")
        if net_exe is not None and product_exe is not None and net_exe != product_exe:
            offences.append(
                f"{net_name}: {exe_const} {net_exe!r}, not {product_name}'s {product_const}.exe_name {product_exe!r}"
            )
    for name, text in ((net_name, net), (product_name, product)):
        suffix = one(name, qtnet.rust_const(text, "SIGNATURE_SUFFIX", "str"), "const SIGNATURE_SUFFIX")
        if suffix is not None and suffix != SIGNATURE_SUFFIX:
            offences.append(f"{name}: SIGNATURE_SUFFIX {suffix!r}, not {SIGNATURE_SUFFIX!r}")
    hosts = one(net_name, str_arrays(net, "REDIRECT_HOSTS"), "const REDIRECT_HOSTS")
    if hosts is not None and hosts != REDIRECT_HOSTS:
        offences.append(f"{net_name}: REDIRECT_HOSTS {hosts!r}, not {REDIRECT_HOSTS!r}")
    return offences


def key_line(text: str) -> str:
    """The key line of a minisign public key file: the line after its
    `untrusted comment:`."""
    lines = [line.strip() for line in text.splitlines() if line.strip()]
    keys = [line for line in lines if not line.startswith("untrusted comment:")]
    if len(keys) != 1:
        raise ValueError(f"not one key line in a public key file: {keys!r}")
    return keys[0]


def key_id(line: str) -> bytes:
    """The key ID of a minisign public key line, or a `ValueError`."""
    try:
        raw = base64.b64decode(line, validate=True)
    except ValueError as error:
        raise ValueError(f"not base64: {line!r}") from error
    if len(raw) != KEY_BYTES or raw[:2] != KEY_ALGORITHM:
        raise ValueError(f"not a minisign Ed25519 public key ({len(raw)} bytes, {raw[:2]!r}): {line!r}")
    return raw[2:10]


def release_key_offences(name: str, text: str, fixture_line: str) -> list[str]:
    """Where `RELEASE_PUBLIC_KEY` in non-test `text` is not one `&str`
    holding a minisign key line other than the fixture's key."""
    _, code = qtnet.non_test_rust(text)
    values = qtnet.rust_const(code, "RELEASE_PUBLIC_KEY", "str")
    if len(values) != 1:
        return [f"{name}: RELEASE_PUBLIC_KEY defined as a &str {len(values)} times outside tests"]
    value = values[0]
    try:
        ident = key_id(value)
    except ValueError as error:
        return [f"{name}: RELEASE_PUBLIC_KEY is {error}"]
    if value == fixture_line or ident == key_id(fixture_line):
        return [f"{name}: RELEASE_PUBLIC_KEY is the test fixture's key {ident[::-1].hex().upper()}"]
    return []


def lock_packages(lock: dict) -> dict[tuple[str, str], dict]:
    return {(p["name"], p["version"]): p for p in lock.get("package", [])}


def lock_offences(shared: dict, quest: dict, local: set[str]) -> list[str]:
    """Where the shared lock holds a package the quest tracker's lock does
    not, at the same version, source and checksum, or a local package other
    than `local`."""
    offences = []
    quest_packages = lock_packages(quest)
    seen_local = set()
    for (name, version), package in sorted(lock_packages(shared).items()):
        spec = f"{name}@{version}"
        theirs = quest_packages.get((name, version))
        if "source" not in package:
            seen_local.add(name)
            if name not in local:
                offences.append(f"shared lock: local package {spec} is not one of {sorted(local)}")
            elif theirs is None or "source" in theirs:
                offences.append(f"shared lock: local package {spec} is not a local package of the quest tracker's")
            continue
        if theirs is None:
            offences.append(f"shared lock: {spec} is not in the quest tracker's lock")
            continue
        for key in ("source", "checksum"):
            if package.get(key) != theirs.get(key):
                offences.append(f"shared lock: {spec} {key} {package.get(key)!r}, the quest tracker's {theirs.get(key)!r}")
    for missing in sorted(local - seen_local):
        offences.append(f"shared lock: no local package {missing}")
    return offences


def pin_of(spec) -> str | None:
    return spec if isinstance(spec, str) else spec.get("version") if isinstance(spec, dict) else None


def pin_offences(shared: dict, quest: dict, quest_lock: dict, new: tuple[str, str]) -> list[str]:
    """Where a `[workspace.dependencies]` pin of the shared manifest is not
    exact, differs from the quest tracker's pin, or is not the one new crate
    at its pin and at the version of the quest tracker's lock."""
    offences = []
    ours = shared.get("workspace", {}).get("dependencies", {})
    theirs = quest.get("workspace", {}).get("dependencies", {})
    new_name, new_pin = new
    for dep, spec in sorted(ours.items()):
        pin = pin_of(spec)
        if not pin or not pin.startswith("="):
            offences.append(f"shared manifest: {dep} pin {pin!r} is not exact")
            continue
        if dep in theirs:
            if pin_of(theirs[dep]) != pin:
                offences.append(f"shared manifest: {dep} {pin}, the quest tracker's {pin_of(theirs[dep])}")
            continue
        if dep != new_name:
            offences.append(f"shared manifest: {dep} is not a dependency of the quest tracker's workspace")
            continue
        if pin != new_pin:
            offences.append(f"shared manifest: {dep} {pin}, not {new_pin}")
        locked = sorted(p["version"] for p in quest_lock.get("package", []) if p["name"] == dep)
        if locked != [pin[1:]]:
            offences.append(f"shared manifest: {dep} {pin}, the quest tracker's lock holds {locked!r}")
    if new_name not in ours:
        offences.append(f"shared manifest: no pin for {new_name}")
    return offences


def member_offences(name: str, manifest: dict, members: set[str]) -> list[str]:
    """Where a member manifest names a version of its own, or depends on a
    `qt-*`, `wr-*` or `ch-*` crate."""
    offences = []
    for label, table in qtnet.dependency_tables(manifest):
        for dep, spec in table.items():
            real = spec.get("package", dep) if isinstance(spec, dict) else dep
            if real.startswith(APP_CRATE_PREFIXES):
                offences.append(f"{name}: [{label}] {real} is an app's crate")
            if isinstance(spec, dict) and "path" in spec and real in members and "version" not in spec:
                continue
            if not (isinstance(spec, dict) and spec.get("workspace") is True and "version" not in spec):
                offences.append(f"{name}: [{label}] {dep} is not taken from the workspace")
    return offences


class OneNetworkCrate(unittest.TestCase):
    def test_no_shared_rust_outside_ep_net_names_the_network(self):
        names = qtnet.RUST_NETWORK_NAMES
        self.assertEqual(
            qtnet.text_offences("s.rs", "use std::fs;\nlet h = WinHttpOpen(ua, 0, None, None, 0);\n", names),
            ["s.rs:2: WinHttp"],
        )
        self.assertEqual(qtnet.text_offences("c.rs", "use std::fs;\nlet requested = 1;\n", names), [])

        sources = [path for path in shared_rust() if NET_CRATE not in path.parents]
        for expected in (UPDATER_RS, CHECK_RS, RELEASE_KEY_RS):
            self.assertIn(expected, sources)
        self.assertNotIn(NET_LIB_RS, sources)
        self.assertGreaterEqual(len(sources), 10, [rel(p) for p in sources])
        offences = []
        for path in sources:
            offences += qtnet.text_offences(rel(path), path.read_text(encoding="utf-8"), names)
        self.assertEqual(
            offences, [],
            "shared Rust outside apps/shared/crates/ep-net names network code:\n" + "\n".join(offences),
        )
        # Positive control: the transport itself does.
        self.assertTrue(qtnet.text_offences(rel(NET_LIB_RS), NET_LIB_RS.read_text(encoding="utf-8"), names))

    def test_no_shared_rust_outside_ep_net_is_unsafe(self):
        synthetic = (
            "#![forbid(unsafe_code)]\n"
            "// unsafe in a comment\n"
            'const S: &str = "unsafe";\n'
            "fn f() { unsafe { g() } }\n"
            "unsafe fn g() {}\n"
        )
        self.assertEqual(unsafe_offences("s.rs", synthetic), ["s.rs:4", "s.rs:5"])
        self.assertEqual(unsafe_offences("c.rs", "fn f() -> u8 { 1 }\n"), [])

        sources = [path for path in shared_rust() if NET_CRATE not in path.parents]
        self.assertIn(UPDATER_RS, sources)
        offences = []
        for path in sources:
            offences += unsafe_offences(rel(path), path.read_text(encoding="utf-8"))
        self.assertEqual(offences, [], "shared Rust outside ep-net is unsafe:\n" + "\n".join(offences))
        net = [path for path in shared_rust() if NET_CRATE in path.parents]
        self.assertTrue(
            any(unsafe_offences(rel(p), p.read_text(encoding="utf-8")) for p in net),
            "the matcher sees no unsafe in ep-net itself",
        )

    def test_only_ep_net_turns_on_win32_networking(self):
        synthetic = (
            '[dependencies]\nwindows-sys = { version = "=1", features = ["Win32_Foundation"] }\n'
            '[target.\'cfg(windows)\'.dependencies]\n'
            'windows = { workspace = true, features = ["Win32_Networking_WinSock"] }\n'
            '[workspace.dependencies]\nw = { version = "1", features = ["Win32_Networking_WinInet"] }\n'
        )
        self.assertEqual(
            networking_features("Cargo.toml", synthetic),
            ["Cargo.toml: [workspace.dependencies] w feature Win32_Networking_WinInet",
             "Cargo.toml: [target.cfg(windows).dependencies] windows feature Win32_Networking_WinSock"],
        )
        self.assertEqual(networking_features("c.toml", '[dependencies]\nserde = "1"\n'), [])

        manifests = qtnet.walk(SHARED, names={"Cargo.toml"}) + [
            glue / "Cargo.toml" for glue in GLUES
        ] + [QT_MANIFEST]
        self.assertIn(MEMBERS["ep-net"], manifests)
        self.assertIn(SHARED_MANIFEST, manifests)
        for path in manifests:
            self.assertTrue(path.is_file(), rel(path))
        offences = []
        for path in manifests:
            if path == MEMBERS["ep-net"]:
                continue
            offences += networking_features(rel(path), path.read_text(encoding="utf-8"))
        self.assertEqual(offences, [], "a manifest but ep-net's turns on networking:\n" + "\n".join(offences))
        # Positive control: ep-net's does.
        self.assertTrue(networking_features("ep-net", MEMBERS["ep-net"].read_text(encoding="utf-8")))


class ReleaseContract(unittest.TestCase):
    def test_ep_net_and_ep_update_name_the_releases_the_tooling_makes(self):
        repo = "https://github.com/owner/repo"
        products = [
            ("tool", repo, "tool-v", "TOOL_TAG_PREFIX", "TOOL_EXE", "TOOL"),
            ("other", repo, "other-v", "OTHER_TAG_PREFIX", "OTHER_EXE", "OTHER"),
        ]
        net = (
            '//! pub const PATH: &str = "/repos/doc/doc/releases";\n'
            'pub const PATH: &str = "/repos/owner/repo/releases?per_page=30";\n'
            'pub const DOWNLOAD_PATH_PREFIX: &str = "/owner/repo/releases/download/";\n'
            'pub const TOOL_TAG_PREFIX: &str = "tool-v";\n'
            'pub const OTHER_TAG_PREFIX: &str = "other-v";\n'
            'pub const TOOL_EXE: &str = "tool.exe";\n'
            'pub const OTHER_EXE: &str = "other.exe";\n'
            'pub const SIGNATURE_SUFFIX: &str = ".minisig";\n'
            "pub const REDIRECT_HOSTS: [&str; 2] = [\n"
            '    "release-assets.githubusercontent.com",\n'
            '    "objects.githubusercontent.com",\n'
            "];\n"
            '#[cfg(test)]\nmod tests { const REDIRECT_HOSTS: [&str; 1] = ["127.0.0.1"]; }\n'
        )
        product = (
            'pub const SIGNATURE_SUFFIX: &str = ".minisig";\n'
            '/// pub const TOOL: Product = Product { tag_prefix: "doc-v", exe_name: "doc.exe" };\n'
            'pub const TOOL: Product = Product {\n    name: "Tool",\n    tag_prefix: "tool-v",\n'
            '    exe_name: "tool.exe",\n};\n'
            'pub const OTHER: Product = Product { name: "Other", tag_prefix: "other-v", exe_name: "other.exe" };\n'
        )
        self.assertEqual(qtnet.non_test_rust(net)[1].count("127.0.0.1"), 0)
        self.assertEqual(contract_offences("net.rs", net, "version.rs", product, products), [])
        # Each product's release module must name the repository ep-net asks:
        # one that names another is two offences, its list and its downloads.
        forked = products[:1] + [(products[1][0], "https://github.com/owner/fork") + products[1][2:]]
        self.assertEqual(
            contract_offences("net.rs", net, "version.rs", product, forked),
            ["net.rs: PATH '/repos/owner/repo/releases?per_page=30' is not '/repos/owner/fork/releases' "
             "(the other's https://github.com/owner/fork)",
             "net.rs: DOWNLOAD_PATH_PREFIX '/owner/repo/releases/download/' is not "
             "'/owner/fork/releases/download/' (the other's https://github.com/owner/fork)"],
        )
        wrong = {
            "net.rs: OTHER_TAG_PREFIX 'tool-v', not the other's TAG_PREFIX 'other-v'":
                (net.replace('OTHER_TAG_PREFIX: &str = "other-v"', 'OTHER_TAG_PREFIX: &str = "tool-v"'), product),
            "version.rs: TOOL.tag_prefix 'tool2-v', not the tool's TAG_PREFIX 'tool-v'":
                (net, product.replace('    tag_prefix: "tool-v"', '    tag_prefix: "tool2-v"')),
            "net.rs: OTHER_EXE 'other2.exe', not version.rs's OTHER.exe_name 'other.exe'":
                (net.replace('"other.exe"', '"other2.exe"'), product),
            "version.rs: SIGNATURE_SUFFIX '.sig', not '.minisig'":
                (net, product.replace('".minisig"', '".sig"')),
            "net.rs: REDIRECT_HOSTS ['release-assets.githubusercontent.com', 'evil.example'], "
            f"not {REDIRECT_HOSTS!r}":
                (net.replace('"objects.githubusercontent.com"', '"evil.example"'), product),
            "net.rs: REDIRECT_HOSTS ['release-assets.githubusercontent.com'], not " f"{REDIRECT_HOSTS!r}":
                (net.replace('[&str; 2]', '[&str; 1]').replace('    "objects.githubusercontent.com",\n', ""),
                 product),
            "net.rs: const TOOL_TAG_PREFIX defined 0 times outside tests":
                (net.replace('pub const TOOL_TAG_PREFIX: &str = "tool-v";\n', ""), product),
            "net.rs: const PATH defined 2 times outside tests":
                (net + 'const PATH: &str = "/repos/owner/repo/releases";\n', product),
        }
        # A product renamed away drops both of its fields: two offences.
        renamed = product.replace("pub const OTHER: Product", "pub const OTHER2: Product")
        self.assertEqual(
            contract_offences("net.rs", net, "version.rs", renamed, products),
            ["version.rs: OTHER.tag_prefix defined 0 times outside tests",
             "version.rs: OTHER.exe_name defined 0 times outside tests"],
        )
        for expected, (net_text, product_text) in wrong.items():
            self.assertEqual(
                contract_offences("net.rs", net_text, "version.rs", product_text, products), [expected],
            )

        for path in (NET_LIB_RS, VERSION_RS):
            self.assertTrue(path.is_file(), rel(path))
        real = [
            (label, module.REPO_URL, module.TAG_PREFIX, tag_const, exe_const, product_const)
            for label, module, tag_const, exe_const, product_const in PRODUCTS
        ]
        self.assertEqual([qtnet.repo_of(p[1]) for p in real], ["S-Borkowski/ExilePact2"] * 3)
        self.assertEqual([p[2] for p in real], ["quest-tracker-v", "waystone-roller-v", "crafting-helper-v"])
        offences = contract_offences(
            rel(NET_LIB_RS), NET_LIB_RS.read_text(encoding="utf-8"),
            rel(VERSION_RS), VERSION_RS.read_text(encoding="utf-8"), real,
        )
        self.assertEqual(offences, [], "the shared crates disagree with the release contract:\n" + "\n".join(offences))

    def test_each_exe_asset_is_the_built_exe_as_github_stores_it(self):
        # GitHub stores an asset name with each run of characters outside
        # `[A-Za-z0-9_+@-]` as one dot (the rule `gh release upload` mirrors
        # for `--clobber`), so `Crafting Helper.exe` is `Crafting.Helper.exe`.
        self.assertEqual(github_asset_name("Crafting Helper.exe"), "Crafting.Helper.exe")
        self.assertEqual(github_asset_name("quest-tracker.exe"), "quest-tracker.exe")
        self.assertEqual(github_asset_name("a  b (c).exe"), "a.b.c.exe")
        # The asset is the Collect step's copy of the built exe: its target
        # name, or the built name when the copy goes to a directory or there
        # is none (the negative control: no rename means the built name).
        built_exe = "apps/tool/target/release/Tool App.exe"
        self.assertEqual(released_asset_name("run: echo\n", "tool", "Tool App.exe"), "Tool.App.exe")
        self.assertEqual(
            released_asset_name(f'  cp "{built_exe}" dist-release/\n', "tool", "Tool App.exe"), "Tool.App.exe",
        )
        self.assertEqual(
            released_asset_name(f'  cp "{built_exe}" dist-release/tool.exe\n', "tool", "Tool App.exe"), "tool.exe",
        )
        # Another app's copy, or a second copy of the copy, names nothing.
        self.assertEqual(
            released_asset_name('  cp "apps/other/target/release/Tool App.exe" dist-release/o.exe\n'
                                "  cp dist-release/Tool.App.exe dist-release/tool.exe\n", "tool", "Tool App.exe"),
            "Tool.App.exe",
        )
        with self.assertRaises(ValueError):
            released_asset_name(f'cp "{built_exe}" dist-release/a.exe\ncp "{built_exe}" dist-release/b.exe\n',
                                "tool", "Tool App.exe")
        _, net = qtnet.non_test_rust(NET_LIB_RS.read_text(encoding="utf-8"))
        _, product = qtnet.non_test_rust(VERSION_RS.read_text(encoding="utf-8"))
        built, released = {}, {}
        for app, (_, _, _, exe_const, product_const) in zip((QT, WR, CH), PRODUCTS):
            conf = json.loads((app / "src-tauri" / "tauri.conf.json").read_text(encoding="utf-8"))
            built[product_const] = conf["mainBinaryName"] + ".exe"
            workflow = ROOT / ".github" / "workflows" / f"{app.name}-release.yml"
            asset = released_asset_name(workflow.read_text(encoding="utf-8"), app.name, built[product_const])
            released[product_const] = asset
            with self.subTest(app=app.name):
                self.assertEqual(qtnet.rust_const(net, exe_const, "str"), [asset])
                self.assertEqual(product_fields(product, product_const, "exe_name"), [asset])
        # The crafting helper and the quest tracker build title-case exes;
        # the quest tracker's workflow uploads its build as the kebab name
        # 0.1.x installs fetch, and the waystone roller keeps its kebab name.
        self.assertEqual(built, {
            "QUEST_TRACKER": "Quest Tracker.exe",
            "WAYSTONE_ROLLER": "waystone-roller.exe",
            "CRAFTING_HELPER": "Crafting Helper.exe",
        })
        self.assertEqual(released, {
            "QUEST_TRACKER": "quest-tracker.exe",
            "WAYSTONE_ROLLER": "waystone-roller.exe",
            "CRAFTING_HELPER": "Crafting.Helper.exe",
        })


def github_asset_name(name: str) -> str:
    """The name GitHub stores for a release asset uploaded as `name`."""
    return re.sub(r"[^A-Za-z0-9_+@-]+", ".", name).strip(".")


# A release workflow's copy of a file into `dist-release/`: the source (quoted
# or bare) and the name after the slash, empty when it keeps its own.
DIST_COPY = re.compile(r'^\s*cp\s+(?:"([^"\n]+)"|(\S+))\s+dist-release/(\S*)\s*$', re.MULTILINE)


def released_asset_name(workflow: str, app: str, built: str) -> str:
    """The asset name, as GitHub stores it, that a release workflow uploads
    for `app`'s built exe `built`: the name its one copy of
    `apps/<app>/target/release/<built>` into `dist-release/` gives it, or
    `built` when no copy renames it. Two such copies are an error."""
    source = f"apps/{app}/target/release/{built}"
    names = [m[3] or built for m in DIST_COPY.finditer(workflow) if (m[1] or m[2]) == source]
    if len(names) > 1:
        raise ValueError(f"{len(names)} copies of {source} into dist-release/: {names}")
    return github_asset_name(names[0] if names else built)


class KeyAndPins(unittest.TestCase):
    def fixture_line(self) -> str:
        line = key_line(FIXTURE_KEY.read_text(encoding="utf-8"))
        key_id(line)  # Positive control: the fixture key reads as a key.
        return line

    def test_release_key_is_a_real_key_and_not_the_fixtures(self):
        fixture = self.fixture_line()
        # A valid line of another key (the fixture's bytes with a new key ID).
        raw = bytearray(base64.b64decode(fixture))
        raw[2:10] = bytes(range(1, 9))
        other = base64.b64encode(bytes(raw)).decode()
        good = f'//! doc\npub const RELEASE_PUBLIC_KEY: &str = "{other}";\n'
        self.assertEqual(release_key_offences("k.rs", good, fixture), [])
        self.assertEqual(
            release_key_offences("k.rs", good.replace("&str", "&'static str"), fixture), [],
        )
        wrong = {
            f'pub const RELEASE_PUBLIC_KEY: Option<&str> = Some("{other}");\n':
                "k.rs: RELEASE_PUBLIC_KEY defined as a &str 0 times outside tests",
            f'// pub const RELEASE_PUBLIC_KEY: &str = "{other}";\n':
                "k.rs: RELEASE_PUBLIC_KEY defined as a &str 0 times outside tests",
            f'#[cfg(test)]\nconst RELEASE_PUBLIC_KEY: &str = "{other}";\n':
                "k.rs: RELEASE_PUBLIC_KEY defined as a &str 0 times outside tests",
            good + f'pub const RELEASE_PUBLIC_KEY: &str = "{other}";\n':
                "k.rs: RELEASE_PUBLIC_KEY defined as a &str 2 times outside tests",
            f'pub const RELEASE_PUBLIC_KEY: &str = "{fixture}";\n':
                f"k.rs: RELEASE_PUBLIC_KEY is the test fixture's key {key_id(fixture)[::-1].hex().upper()}",
            'pub const RELEASE_PUBLIC_KEY: &str = "";\n':
                "k.rs: RELEASE_PUBLIC_KEY is not a minisign Ed25519 public key (0 bytes, b''): ''",
            'pub const RELEASE_PUBLIC_KEY: &str = "not a key!";\n':
                "k.rs: RELEASE_PUBLIC_KEY is not base64: 'not a key!'",
        }
        for text, expected in wrong.items():
            self.assertEqual(release_key_offences("k.rs", text, fixture), [expected], text)
        # The fixture's key ID under different bytes is still the fixture's.
        same_id = bytearray(base64.b64decode(fixture))
        same_id[10] ^= 0xFF
        self.assertTrue(release_key_offences(
            "k.rs", f'pub const RELEASE_PUBLIC_KEY: &str = "{base64.b64encode(bytes(same_id)).decode()}";\n', fixture,
        ))

        self.assertTrue(RELEASE_KEY_RS.is_file(), rel(RELEASE_KEY_RS))
        offences = release_key_offences(rel(RELEASE_KEY_RS), RELEASE_KEY_RS.read_text(encoding="utf-8"), fixture)
        self.assertEqual(offences, [], "\n".join(offences))

    def test_neither_glue_sets_the_key_source(self):
        synthetic = (
            "// public_key: the built-in one\n"
            'const S: &str = "public_key";\n'
            "let config = UpdateConfig { public_key: Some(k), ..UpdateConfig::default() };\n"
            "config.public_key = Some(k);\n"
            "#[cfg(test)]\nmod tests { fn f() { UpdateConfig { public_key: None, ..Default::default() }; } }\n"
        )
        self.assertEqual(key_source_offences("s.rs", synthetic), ["s.rs:3", "s.rs:4"])
        self.assertEqual(
            key_source_offences("c.rs", "let config = UpdateConfig { exe_path: p, ..UpdateConfig::default() };\n"),
            [],
        )

        sources = glue_rust()
        for glue in GLUES:
            self.assertIn(glue / "src" / "lib.rs", sources)
        offences = []
        for path in sources:
            offences += key_source_offences(rel(path), path.read_text(encoding="utf-8"))
        self.assertEqual(offences, [], "a glue sets the update key source:\n" + "\n".join(offences))
        # Positive control: the source the glue would set is the config's.
        self.assertTrue(key_source_offences(rel(CHECK_RS), CHECK_RS.read_text(encoding="utf-8")))

    def test_shared_lock_is_the_quest_trackers(self):
        reg = "registry+https://github.com/rust-lang/crates.io-index"
        quest = {"package": [
            {"name": "ep-net", "version": "0.1.0"},
            {"name": "ep-update", "version": "0.1.0"},
            {"name": "serde", "version": "1.0.0", "source": reg, "checksum": "aa"},
            {"name": "qt-app", "version": "0.1.0"},
        ]}
        shared = {"package": [
            {"name": "ep-net", "version": "0.1.0"},
            {"name": "ep-update", "version": "0.1.0"},
            {"name": "serde", "version": "1.0.0", "source": reg, "checksum": "aa"},
        ]}
        self.assertEqual(lock_offences(shared, quest, SHARED_LOCAL_PACKAGES), [])

        def with_package(**package):
            return {"package": shared["package"] + [package]}

        wrong = {
            "shared lock: evil@1.0.0 is not in the quest tracker's lock":
                with_package(name="evil", version="1.0.0", source=reg, checksum="bb"),
            "shared lock: serde@1.0.1 is not in the quest tracker's lock":
                {"package": shared["package"][:2] + [dict(shared["package"][2], version="1.0.1")]},
            "shared lock: serde@1.0.0 checksum 'bb', the quest tracker's 'aa'":
                {"package": shared["package"][:2] + [dict(shared["package"][2], checksum="bb")]},
            "shared lock: serde@1.0.0 source 'git+https://example.com/serde', the quest tracker's " f"{reg!r}":
                {"package": shared["package"][:2] + [dict(shared["package"][2], source="git+https://example.com/serde")]},
            "shared lock: local package qt-app@0.1.0 is not one of ['ep-net', 'ep-update']":
                with_package(name="qt-app", version="0.1.0"),
            "shared lock: no local package ep-update":
                {"package": [shared["package"][0], shared["package"][2]]},
        }
        for expected, lock in wrong.items():
            self.assertEqual(lock_offences(lock, quest, SHARED_LOCAL_PACKAGES), [expected])
        self.assertEqual(
            lock_offences(shared, {"package": [dict(quest["package"][0], source=reg)] + quest["package"][1:]},
                          SHARED_LOCAL_PACKAGES),
            ["shared lock: local package ep-net@0.1.0 is not a local package of the quest tracker's"],
        )

        shared_lock = tomllib.loads(SHARED_LOCK.read_text(encoding="utf-8"))
        quest_lock = tomllib.loads(QT_LOCK.read_text(encoding="utf-8"))
        self.assertGreaterEqual(len(shared_lock.get("package", [])), 3)
        self.assertIn((NEW_CRATE, NEW_CRATE_PIN[1:]), lock_packages(shared_lock))
        offences = lock_offences(shared_lock, quest_lock, SHARED_LOCAL_PACKAGES)
        self.assertEqual(offences, [], "the shared lock is not the quest tracker's:\n" + "\n".join(offences))

    def test_shared_pins_are_the_quest_trackers(self):
        quest = tomllib.loads(
            '[workspace.dependencies]\nserde = { version = "=1.0.0", features = ["derive"] }\n'
            'tauri = "=2.0.0"\n'
        )
        quest_lock = {"package": [{"name": "minisign-verify", "version": "0.2.5"}]}
        new = (NEW_CRATE, NEW_CRATE_PIN)
        good = '[workspace.dependencies]\nserde = { version = "=1.0.0" }\nminisign-verify = "=0.2.5"\n'
        self.assertEqual(pin_offences(tomllib.loads(good), quest, quest_lock, new), [])
        wrong = {
            "shared manifest: serde pin '1.0.0' is not exact": good.replace('"=1.0.0"', '"1.0.0"'),
            "shared manifest: serde =1.0.1, the quest tracker's =1.0.0": good.replace('"=1.0.0"', '"=1.0.1"'),
            "shared manifest: reqwest is not a dependency of the quest tracker's workspace":
                good + 'reqwest = "=0.13.0"\n',
            "shared manifest: no pin for minisign-verify": good.replace('minisign-verify = "=0.2.5"\n', ""),
            "shared manifest: serde pin None is not exact": good.replace('{ version = "=1.0.0" }', '{ path = "x" }'),
        }
        # A wrong new-crate pin also disagrees with the lock: two offences.
        self.assertEqual(
            pin_offences(tomllib.loads(good.replace('"=0.2.5"', '"=0.2.4"')), quest, quest_lock, new),
            ["shared manifest: minisign-verify =0.2.4, not =0.2.5",
             "shared manifest: minisign-verify =0.2.4, the quest tracker's lock holds ['0.2.5']"],
        )
        for expected, text in wrong.items():
            self.assertEqual(pin_offences(tomllib.loads(text), quest, quest_lock, new), [expected], text)

        members = set(MEMBERS)
        member_good = (
            '[dependencies]\nserde = { workspace = true }\nep-net = { path = "../ep-net" }\n'
            "[target.'cfg(windows)'.dependencies]\nwindows-sys = { workspace = true, features = [\"x\"] }\n"
        )
        self.assertEqual(member_offences("m", tomllib.loads(member_good), members), [])
        self.assertEqual(
            member_offences("m", tomllib.loads(member_good.replace(
                "serde = { workspace = true }", 'serde = { workspace = true }\nregex = "=1.0.0"')), members),
            ["m: [dependencies] regex is not taken from the workspace"],
        )
        self.assertEqual(
            member_offences("m", tomllib.loads(member_good.replace(
                "serde = { workspace = true }", 'serde = { workspace = true, version = "1" }')), members),
            ["m: [dependencies] serde is not taken from the workspace"],
        )
        self.assertEqual(
            member_offences("m", tomllib.loads(
                member_good + '[dev-dependencies]\nqt-app = { path = "../../../quest-tracker/crates/qt-app" }\n'),
                members),
            ["m: [dev-dependencies] qt-app is an app's crate",
             "m: [dev-dependencies] qt-app is not taken from the workspace"],
        )
        self.assertEqual(
            member_offences("m", tomllib.loads(
                member_good + '[dependencies.item]\npackage = "ch-item"\nworkspace = true\n'), members),
            ["m: [dependencies] ch-item is an app's crate"],
        )

        shared = tomllib.loads(SHARED_MANIFEST.read_text(encoding="utf-8"))
        offences = pin_offences(
            shared, tomllib.loads(QT_MANIFEST.read_text(encoding="utf-8")),
            tomllib.loads(QT_LOCK.read_text(encoding="utf-8")), new,
        )
        self.assertEqual(sorted(shared["workspace"]["members"]), ["crates/ep-net", "crates/ep-update"])
        for name, path in MEMBERS.items():
            self.assertTrue(path.is_file(), rel(path))
            manifest = tomllib.loads(path.read_text(encoding="utf-8"))
            self.assertEqual(manifest["package"]["name"], name)
            offences += member_offences(rel(path), manifest, members)
        self.assertEqual(offences, [], "the shared workspace's pins:\n" + "\n".join(offences))


if __name__ == "__main__":
    unittest.main()
