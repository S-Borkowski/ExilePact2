"""Launch one of this repository's Tauri apps with its WebView2 debuggable.

The `app-webview` MCP server (`.mcp.json`) is Playwright attached over CDP to
`http://127.0.0.1:9333`. Nothing listens there unless an app was started by
this tool: it sets `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` for that one
process only, so an app started any other way stays undebuggable. It drives
the app's own window, never the game (`AGENTS.md` § "Legal").

    py -3 tools/app_webview.py launch <path to app .exe>
    py -3 tools/app_webview.py status
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import time
import urllib.error
import urllib.request

PORT = 9333
LIST_URL = f"http://127.0.0.1:{PORT}/json/list"
ENV_VAR = "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS"
WAIT_SECONDS = 20


def child_env(base: dict[str, str]) -> dict[str, str]:
    env = dict(base)
    env[ENV_VAR] = f"--remote-debugging-port={PORT}"
    return env


def pages() -> list[dict] | None:
    """The debuggable pages, or None when nothing listens on the port."""
    try:
        with urllib.request.urlopen(LIST_URL, timeout=2) as answer:
            return json.load(answer)
    except (urllib.error.URLError, OSError, ValueError):
        return None


def print_pages(found: list[dict]) -> None:
    for page in found:
        if page.get("type") == "page":
            print(f"page: {page.get('title')!r} {page.get('url')}")


def launch(exe: str) -> int:
    if not (os.path.isfile(exe) and exe.lower().endswith(".exe")):
        print(f"refused: not an .exe file: {exe}")
        return 2
    if pages() is not None:
        print(f"refused: port {PORT} already answers; close the app holding it first")
        return 2
    process = subprocess.Popen([exe], env=child_env(os.environ), cwd=os.path.dirname(exe) or None)
    deadline = time.monotonic() + WAIT_SECONDS
    while time.monotonic() < deadline:
        found = pages()
        if found:
            print(f"launched: pid {process.pid}, CDP on 127.0.0.1:{PORT}")
            print_pages(found)
            return 0
        if process.poll() is not None:
            print(f"failed: the app exited with {process.returncode} before CDP answered")
            return 1
        time.sleep(0.5)
    print(f"failed: no CDP answer on port {PORT} within {WAIT_SECONDS}s (pid {process.pid} left running)")
    return 1


def status() -> int:
    found = pages()
    if found is None:
        print(f"down: nothing answers on 127.0.0.1:{PORT}")
        return 1
    print(f"up: CDP on 127.0.0.1:{PORT}")
    print_pages(found)
    return 0


def main(argv: list[str]) -> int:
    if len(argv) == 2 and argv[0] == "launch":
        return launch(argv[1])
    if argv == ["status"]:
        return status()
    print(__doc__)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
