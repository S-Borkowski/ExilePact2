"""poe2db.tw's Waystones page: fetch it politely and write the waystone mod data file.

Developer-only. This is the one piece of the waystone roller that touches the
network, and it only ever asks poe2db.tw:

    py -3 -m tools.waystone_roller.poe2db fetch [--cache DIR] [--delay S] [--max-age DAYS]
    py -3 -m tools.waystone_roller.poe2db extract [--cache DIR] [--out FILE]

`fetch` reads robots.txt, then fetches only `/us/Waystones`: two requests, one
at a time, at least `--delay` seconds apart (default 3, never under 2), with a
User-Agent naming this tool and repository. A page cached within `--max-age`
days (default 7) is left alone and nothing is asked at all. The cache, by
default `%LOCALAPPDATA%\\ExilePact2\\waystone-roller\\poe2db-cache\\`, holds the
raw page and `index.json` (URL, fetch date, HTTP status); it is refused inside
a git checkout, because raw pages are never tracked. robots.txt is read with
the quest tracker's reader, which applies the `*` group's rules (poe2db has no
other group) and the `*` and `$` patterns `urllib.robotparser` ignores.

`extract` turns the cached page into the app's data file, by default
`apps/waystone-roller/data/waystone-mods.json`, which is tracked: unlike the
quest tracker's draft, this output ships as it is. It reads only two tabs,
`WaystonesMods` (Level | Pre/Suf | Description) and `DesecratedWaystoneMods`
(Name | Pre/Suf | Description), finding each table by its tab's id and its
header text, never by its random tbody id. From a Description cell it keeps
the displayed lines only: hidden internal stats (`span.secondary`) are
dropped, keyword links keep their text, `<br>` (between stats) and `<br/>`
(inside one stat whose text has two lines, e.g. `#% more Magic and Rare
Monsters` / `Rare Monsters have #% more chance of Monster Modifiers`) both
start a new line, as the item shows them, and every number or range
becomes `#`. Rows whose displayed lines are equal become one family, `tiers`
counting the rows merged; a row with no displayed line is listed under
`skipped` with its reason. Families are sorted and given ids the same way
every run, so a refresh diffs cleanly. A missing tab, unexpected column
headers, an unknown Pre/Suf value, an empty table or a row count that differs
from the tab header's `/N` stops it with a non-zero exit naming the tab, and
leaves the existing file alone.

The data file keeps modifier lines as templates and desecrated affix names,
nothing else, under the owner's 2026-10-24 decision on short game text
(AGENTS.md § "Legal"); `check_data` is the rule the shipped file is held to.
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import os
import re
import sys
import urllib.parse
from pathlib import Path

from tools.quest_tracker.poe2db import (
    Node,
    Robots,
    SystemClock,
    ToolError,
    parse_html,
    refuse_inside_git,
    urllib_fetcher,
)

SITE_HOST = "poe2db.tw"
SITE = "https://" + SITE_HOST
ROBOTS_URL = SITE + "/robots.txt"
PAGE_SLUG = "Waystones"
PAGE_URL = "%s/us/%s" % (SITE, PAGE_SLUG)
PAGE_FILE = PAGE_SLUG + ".html"
USER_AGENT = (
    "ExilePact2-waystone-roller-poe2db/1 (developer data tool, one request at a time; "
    "+https://github.com/S-Borkowski/ExilePact2)"
)
MIN_DELAY = 2.0
DEFAULT_DELAY = 3.0
DEFAULT_MAX_AGE = 7
CACHE_INDEX = "index.json"
DEFAULT_OUT = Path(__file__).resolve().parents[2] / "apps" / "waystone-roller" / "data" / "waystone-mods.json"

SCHEMA = 1
# Tab id -> (group, the table's column headers).
SECTIONS = {
    "WaystonesMods": ("waystone", ("Level", "Pre/Suf", "Description")),
    "DesecratedWaystoneMods": ("desecrated", ("Name", "Pre/Suf", "Description")),
}
GROUPS = ("waystone", "desecrated")
KINDS = {"prefix": "prefix", "suffix": "suffix"}
# The reward lines a regular waystone mod carries besides its own effect.
BONUS_TEMPLATES = (
    "#% more Waystones found in Area",
    "#% increased Waystones found in Area",
    "Monsters have #% more Effectiveness",
    "#% more Rarity of Items found in this Area",
    "#% more Pack size",
    "#% more Magic and Rare Monsters",
    "Rare Monsters have #% more chance of Monster Modifiers",
)
_BONUS = frozenset(t.lower() for t in BONUS_TEMPLATES)
SOURCE_NOTE = (
    "Waystone modifier data from poe2db (poe2db.tw; its wiki content is CC BY-NC-SA 3.0); "
    "modifier lines and desecrated affix names are game text by Grinding Gear Games, quoted "
    "under the owner's 2026-10-24 decision. Values are replaced by #; hidden stats are left out."
)
MAX_LINE = 200
FAMILY_KEYS = ["id", "group", "kind", "name", "lines", "bonus", "tiers"]
TOP_KEYS = ["schema", "source", "sections", "fetched", "source_note", "families", "skipped"]

# `(5–9)`, `(0.5-1.5)`, `(-10–-5)`: a range as poe2db prints it, then any lone number.
_RANGE_RE = re.compile(r"\(\s*[+-]?\d+(?:\.\d+)?\s*[–—-]\s*[+-]?\d+(?:\.\d+)?\s*\)")
_NUMBER_RE = re.compile(r"\d+(?:\.\d+)?")
_COUNT_RE = re.compile(r"/\s*(\d+)\s*$")
_ID_RE = re.compile(r"^[a-z]+(?:-[a-z0-9]+)*$")
_SKIP_TEXT_TAGS = frozenset({"script", "style", "template"})


class ExtractError(ToolError):
    """The page's structure is not what the parser knows; the message starts with the tab id."""


def default_cache_dir() -> Path:
    base = os.environ.get("LOCALAPPDATA") or str(Path.home() / "AppData" / "Local")
    return Path(base) / "ExilePact2" / "waystone-roller" / "poe2db-cache"


# --------------------------------------------------------------------------
# Network: one host, one request at a time, a fixed minimum gap

class Session:
    """Sends one request at a time, at least `delay` seconds apart, and only to poe2db.tw.

    The quest tracker's session has the same rules but sends its own
    User-Agent, so this tool keeps its own.
    """

    def __init__(self, fetcher, clock, delay: float):
        self.fetcher = fetcher
        self.clock = clock
        self.delay = delay
        self.last: float | None = None
        self.requests: list[str] = []

    def get(self, url: str) -> tuple[int, bytes]:
        parts = urllib.parse.urlsplit(url)
        if parts.scheme != "https" or parts.netloc != SITE_HOST:
            raise ToolError("refusing a request to %s: this tool only asks %s" % (url, SITE_HOST))
        if self.last is not None:
            wait = self.delay - (self.clock.monotonic() - self.last)
            if wait > 0:
                self.clock.sleep(wait)
        try:
            status, body = self.fetcher(url, USER_AGENT)
        finally:
            self.last = self.clock.monotonic()
        self.requests.append(url)
        return status, body


def _load_robots(session: Session) -> Robots:
    status, body = session.get(ROBOTS_URL)
    if status == 200:
        return Robots(body.decode("utf-8", "replace"))
    if status in (401, 403):
        return Robots(disallow_all=True)
    if 400 <= status < 500:
        return Robots(allow_all=True)
    raise ToolError("robots.txt answered HTTP %s; fetching nothing until it can be read" % status)


# --------------------------------------------------------------------------
# Cache

def _load_index(cache: Path) -> dict:
    path = Path(cache) / CACHE_INDEX
    if not path.is_file():
        return {"schema": 1, "source": PAGE_URL, "pages": {}}
    with open(path, encoding="utf-8") as fh:
        data = json.load(fh)
    data.setdefault("pages", {})
    return data


def _save_index(cache: Path, index: dict) -> None:
    path = Path(cache) / CACHE_INDEX
    tmp = path.with_suffix(".json.tmp")
    with open(tmp, "w", encoding="utf-8", newline="\n") as fh:
        json.dump(index, fh, indent=2, sort_keys=True, ensure_ascii=False)
        fh.write("\n")
    os.replace(tmp, path)


def _fresh(record: dict | None, cache: Path, today: _dt.date, max_age: int) -> bool:
    if not record or record.get("status") != 200 or not record.get("file"):
        return False
    if not (Path(cache) / record["file"]).is_file():
        return False
    try:
        fetched = _dt.date.fromisoformat(record["fetched"])
    except (KeyError, TypeError, ValueError):
        return False
    # A record dated after today (a clock that moved back) is not fresh either.
    return 0 <= (today - fetched).days < max_age


def fetch(cache: Path, delay: float = DEFAULT_DELAY, max_age: int = DEFAULT_MAX_AGE,
          fetcher=None, clock=None, out=None) -> str:
    """Fetch robots.txt and the Waystones page into `cache`. Returns "fetched" or "cached"."""
    out = out or sys.stdout
    if not delay >= MIN_DELAY:
        raise ToolError("refusing --delay %s: the minimum is %s seconds between requests" % (delay, MIN_DELAY))
    if max_age < 0:
        raise ToolError("--max-age cannot be negative")
    cache = Path(cache)
    refuse_inside_git(cache, "poe2db cache")
    clock = clock or SystemClock()
    today = clock.today()
    index = _load_index(cache)
    record = index["pages"].get(PAGE_SLUG)
    if _fresh(record, cache, today, max_age):
        print("cached %s (fetched %s, within %d days); nothing asked" % (PAGE_SLUG, record["fetched"], max_age),
              file=out)
        return "cached"

    session = Session(fetcher or urllib_fetcher, clock, float(delay))
    robots = _load_robots(session)
    crawl_delay = robots.crawl_delay()
    if crawl_delay is not None and crawl_delay > session.delay:
        session.delay = crawl_delay
    if not robots.allows(PAGE_URL):
        raise ToolError("%s is disallowed by robots.txt; nothing fetched" % PAGE_URL)
    status, body = session.get(PAGE_URL)
    if status != 200:
        raise ToolError("%s answered HTTP %s; the cache is unchanged" % (PAGE_URL, status))
    cache.mkdir(parents=True, exist_ok=True)
    (cache / PAGE_FILE).write_bytes(body)
    index["pages"][PAGE_SLUG] = {"slug": PAGE_SLUG, "url": PAGE_URL, "fetched": today.isoformat(),
                                 "status": status, "file": PAGE_FILE}
    _save_index(cache, index)
    print("fetch %s: %d bytes; cache %s" % (PAGE_SLUG, len(body), cache), file=out)
    return "fetched"


# --------------------------------------------------------------------------
# Extracting the two mod tables

def _tab(root: Node, section: str) -> Node:
    for node in root.find_all(id=section):
        if node.has_class("tab-pane"):
            return node
    raise ExtractError("%s: the tab is missing from the page (no div.tab-pane with this id)" % section)


def _headers(table: Node) -> tuple[str, ...]:
    head = table.find("thead") or table
    row = head.find("tr")
    if row is None:
        return ()
    return tuple(cell.text() for cell in row.elements() if cell.tag in ("th", "td"))


def _mods_table(tab: Node, section: str) -> Node:
    want = SECTIONS[section][1]
    tables = list(tab.find_all("table"))
    found = [t for t in tables if _headers(t) == want]
    if len(found) == 1:
        return found[0]
    seen = "; ".join(" | ".join(_headers(t)) or "(no headers)" for t in tables) or "no table"
    if not found:
        raise ExtractError("%s: no table with the columns %s; found: %s" % (section, " | ".join(want), seen))
    raise ExtractError("%s: %d tables have the columns %s; cannot tell which holds the mods"
                       % (section, len(found), " | ".join(want)))


def _header_count(tab: Node) -> int | None:
    header = tab.find(cls="card-header")
    if header is None:
        return None
    match = _COUNT_RE.search(header.text())
    return int(match.group(1)) if match else None


def _template(line: str) -> str:
    line = _RANGE_RE.sub("#", line)
    line = _NUMBER_RE.sub("#", line)
    return " ".join(line.split())


def description_lines(cell: Node) -> tuple[list[str], bool]:
    """The cell's displayed lines as templates, and whether it held hidden stats."""
    pieces: list[str] = []
    hidden = False

    def walk(node: Node) -> None:
        nonlocal hidden
        for child in node.children:
            if isinstance(child, str):
                pieces.append(child)
            elif child.tag == "br":
                pieces.append("\n")
            elif child.has_class("secondary"):
                hidden = True
            elif child.tag not in _SKIP_TEXT_TAGS:
                walk(child)

    walk(cell)
    lines = [_template(line) for line in "".join(pieces).split("\n")]
    return [line for line in lines if line], hidden


def _rows(table: Node) -> list[Node]:
    body = table.find("tbody")
    if body is None:
        return []
    return [row for row in body.elements() if row.tag == "tr"]


def _section_rows(root: Node, section: str):
    """Yield (row number, kind, name, lines, hidden) for each row of the tab's mods table."""
    group = SECTIONS[section][0]
    tab = _tab(root, section)
    rows = _rows(_mods_table(tab, section))
    if not rows:
        raise ExtractError("%s: the mods table has no rows" % section)
    count = _header_count(tab)
    if count is not None and count != len(rows):
        raise ExtractError("%s: the tab header says /%d but the table has %d rows" % (section, count, len(rows)))
    for number, row in enumerate(rows, 1):
        cells = [c for c in row.elements() if c.tag in ("td", "th")]
        if len(cells) != 3:
            raise ExtractError("%s: row %d has %d cells, not 3" % (section, number, len(cells)))
        affix = cells[1].text()
        kind = KINDS.get(affix.strip().lower())
        if kind is None:
            raise ExtractError("%s: row %d has an unknown Pre/Suf value %r" % (section, number, affix))
        name = None
        if group == "desecrated":
            name = " ".join(cells[0].text().split())
            if not name:
                raise ExtractError("%s: row %d has no affix name" % (section, number))
        lines, hidden = description_lines(cells[2])
        yield number, kind, name, lines, hidden


def family_sort_key(family: dict):
    return (GROUPS.index(family["group"]), [line.lower() for line in family["lines"]], family["lines"],
            family["kind"], family["name"] or "")


def sorted_families(families: list[dict]) -> list[dict]:
    return sorted(families, key=family_sort_key)


def _slug(line: str) -> str:
    return "-".join(re.findall(r"[a-z]+", line.lower())) or "mod"


def parse_page(html: str, fetched: str) -> dict:
    """The data file for one Waystones page fetched on `fetched` (ISO date)."""
    root = parse_html(html)
    families: dict[tuple, dict] = {}
    skipped: list[dict] = []
    for section, (group, _) in SECTIONS.items():
        for number, kind, name, lines, hidden in _section_rows(root, section):
            if not lines:
                skipped.append({"section": section, "row": number,
                                "reason": "only hidden stats" if hidden else "empty description"})
                continue
            key = (group, tuple(lines))
            family = families.get(key)
            if family is None:
                families[key] = {"group": group, "kind": kind, "name": name, "lines": lines,
                                 "bonus": [line.lower() in _BONUS for line in lines], "tiers": 1}
            elif (family["kind"], family["name"]) != (kind, name):
                raise ExtractError("%s: row %d has the lines of a %s %s but is a %s %s"
                                   % (section, number, family["kind"], family["name"] or "mod",
                                      kind, name or "mod"))
            else:
                family["tiers"] += 1
    ordered: list[dict] = []
    used: dict[str, int] = {}
    for family in sorted_families(list(families.values())):
        base = _slug(family["lines"][0])
        used[base] = used.get(base, 0) + 1
        ident = base if used[base] == 1 else "%s-%d" % (base, used[base])
        ordered.append({"id": ident, **family})
    return {"schema": SCHEMA, "source": PAGE_URL, "sections": list(SECTIONS), "fetched": fetched,
            "source_note": SOURCE_NOTE, "families": ordered, "skipped": skipped}


def check_data(data: dict) -> list[str]:
    """Every way `data` breaks the data file's rules; empty when it keeps them."""
    problems: list[str] = []
    if list(data) != TOP_KEYS:
        problems.append("top-level keys are %s, not %s" % (list(data), TOP_KEYS))
    if data.get("schema") != SCHEMA:
        problems.append("schema is %r, not %d" % (data.get("schema"), SCHEMA))
    if data.get("source") != PAGE_URL:
        problems.append("source is %r, not %s" % (data.get("source"), PAGE_URL))
    if data.get("sections") != list(SECTIONS):
        problems.append("sections are %r" % (data.get("sections"),))
    try:
        _dt.date.fromisoformat(data.get("fetched"))
    except (TypeError, ValueError):
        problems.append("fetched is %r, not an ISO date" % (data.get("fetched"),))
    if not isinstance(data.get("source_note"), str) or "poe2db" not in data.get("source_note", ""):
        problems.append("source_note does not name poe2db")
    families = data.get("families")
    if not isinstance(families, list) or not families:
        return problems + ["families is empty or not a list"]
    ids: set[str] = set()
    keys: set[tuple] = set()
    for n, family in enumerate(families):
        where = "family %d (%s)" % (n, family.get("id") if isinstance(family, dict) else "?")
        if not isinstance(family, dict) or list(family) != FAMILY_KEYS:
            problems.append("%s: keys are not %s" % (where, FAMILY_KEYS))
            continue
        ident = family["id"]
        if not isinstance(ident, str) or not _ID_RE.match(ident):
            problems.append("%s: id is not a kebab slug" % where)
        elif ident in ids:
            problems.append("%s: id is used twice" % where)
        ids.add(ident)
        if family["group"] not in GROUPS:
            problems.append("%s: group %r" % (where, family["group"]))
        if family["kind"] not in KINDS.values():
            problems.append("%s: kind %r" % (where, family["kind"]))
        name = family["name"]
        if family["group"] == "waystone" and name is not None:
            problems.append("%s: a waystone mod has a name" % where)
        if family["group"] == "desecrated" and not (isinstance(name, str) and name.strip()):
            problems.append("%s: a desecrated mod has no name" % where)
        if isinstance(name, str) and re.search(r"[\[<>0-9]", name):
            problems.append("%s: name %r is not plain text" % (where, name))
        lines = family["lines"]
        if not isinstance(lines, list) or not lines or not all(isinstance(x, str) and x for x in lines):
            problems.append("%s: lines is not a list of text" % where)
            continue
        for line in lines:
            if re.search(r"[\[<>0-9]", line):
                problems.append("%s: line %r holds a digit, `[` or markup" % (where, line))
            if len(line) > MAX_LINE:
                problems.append("%s: a line is %d characters, over %d" % (where, len(line), MAX_LINE))
            if line != " ".join(line.split()):
                problems.append("%s: line %r has stray whitespace" % (where, line))
        bonus = family["bonus"]
        if not isinstance(bonus, list) or len(bonus) != len(lines) or not all(isinstance(b, bool) for b in bonus):
            problems.append("%s: bonus is not one bool per line" % where)
        elif bonus != [line.lower() in _BONUS for line in lines]:
            problems.append("%s: bonus does not follow the reward templates" % where)
        tiers = family["tiers"]
        if not isinstance(tiers, int) or isinstance(tiers, bool) or tiers < 1:
            problems.append("%s: tiers %r" % (where, tiers))
        key = (family["group"], tuple(lines))
        if key in keys:
            problems.append("%s: the same lines as another family" % where)
        keys.add(key)
    skipped = data.get("skipped")
    if not isinstance(skipped, list):
        problems.append("skipped is not a list")
    else:
        for entry in skipped:
            if (not isinstance(entry, dict) or list(entry) != ["section", "row", "reason"]
                    or entry["section"] not in SECTIONS or not isinstance(entry["row"], int)
                    or entry["row"] < 1 or not entry["reason"]):
                problems.append("skipped entry %r is malformed" % (entry,))
    return problems


def dumps(data: dict) -> str:
    return json.dumps(data, indent=2, ensure_ascii=False) + "\n"


def extract(cache: Path, out_path: Path | None = None, out=None) -> dict:
    """Write the data file from the cached page. Returns the data written."""
    out = out or sys.stdout
    cache = Path(cache)
    record = _load_index(cache)["pages"].get(PAGE_SLUG)
    if not record or record.get("status") != 200 or not record.get("file") \
            or not (cache / record["file"]).is_file():
        raise ToolError("no cached %s page in %s; run `fetch` first" % (PAGE_SLUG, cache))
    with open(cache / record["file"], encoding="utf-8", errors="replace") as fh:
        html = fh.read()
    data = parse_page(html, record["fetched"])
    problems = check_data(data)
    if problems:
        raise ExtractError("the extracted data breaks the file's rules: " + "; ".join(problems[:5]))
    out_path = Path(out_path) if out_path is not None else DEFAULT_OUT
    out_path.parent.mkdir(parents=True, exist_ok=True)
    tmp = out_path.with_name(out_path.name + ".tmp")
    with open(tmp, "w", encoding="utf-8", newline="\n") as fh:
        fh.write(dumps(data))
    os.replace(tmp, out_path)
    groups = [f["group"] for f in data["families"]]
    print("families: %d (%d waystone, %d desecrated)"
          % (len(groups), groups.count("waystone"), groups.count("desecrated")), file=out)
    print("skipped rows: %d" % len(data["skipped"]), file=out)
    for entry in data["skipped"]:
        print("  %s row %d: %s" % (entry["section"], entry["row"], entry["reason"]), file=out)
    print("wrote %s (page fetched %s)" % (out_path, data["fetched"]), file=out)
    return data


# --------------------------------------------------------------------------
# Command line

def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="py -3 -m tools.waystone_roller.poe2db",
        description="Developer-only: fetch poe2db.tw's Waystones page and write the waystone roller's mod data.")
    sub = parser.add_subparsers(dest="command", required=True, metavar="{fetch,extract}")

    f = sub.add_parser("fetch", help="fetch robots.txt and the Waystones page into the cache, politely")
    f.add_argument("--cache", type=Path, default=None,
                   help="cache folder, outside any git checkout "
                        "(default %%LOCALAPPDATA%%\\ExilePact2\\waystone-roller\\poe2db-cache)")
    f.add_argument("--delay", type=float, default=DEFAULT_DELAY,
                   help="seconds between requests (default %s, minimum %s)" % (DEFAULT_DELAY, MIN_DELAY))
    f.add_argument("--max-age", type=int, default=DEFAULT_MAX_AGE, metavar="DAYS",
                   help="leave a page fetched within this many days alone (default %d)" % DEFAULT_MAX_AGE)

    e = sub.add_parser("extract", help="write the mod data file from the cached page")
    e.add_argument("--cache", type=Path, default=None, help="cache folder (default as for fetch)")
    e.add_argument("--out", type=Path, default=None,
                   help="data file (default apps/waystone-roller/data/waystone-mods.json)")
    return parser


def main(argv: list[str] | None = None, fetcher=None, clock=None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    cache = args.cache if args.cache is not None else default_cache_dir()
    try:
        if args.command == "fetch":
            if not args.delay >= MIN_DELAY:
                print("poe2db: error: refusing --delay %s: the minimum is %s seconds between requests"
                      % (args.delay, MIN_DELAY), file=sys.stderr)
                return 2
            fetch(cache, delay=args.delay, max_age=args.max_age, fetcher=fetcher, clock=clock)
        else:
            extract(cache, out_path=args.out)
    except ToolError as err:
        print("poe2db: error: %s" % err, file=sys.stderr)
        return 1
    except (OSError, json.JSONDecodeError) as err:
        print("poe2db: error: %s" % err, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
