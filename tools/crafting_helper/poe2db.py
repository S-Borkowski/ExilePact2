"""poe2db.tw's item-class modifier pages: fetch them politely and write the crafting helper's data.

Developer-only. This is the one piece of the crafting helper that touches the
network, and it only ever asks poe2db.tw:

    py -3 -m tools.crafting_helper.poe2db fetch [--cache DIR] [--delay S] [--max-age DAYS]
    py -3 -m tools.crafting_helper.poe2db extract [--cache DIR] [--out DIR]
    py -3 -m tools.crafting_helper.poe2db check [--data DIR]

`fetch` reads robots.txt, then every page of the class table below that is not
fresh in the cache: one request at a time, at least `--delay` seconds apart
(default 3, never under 2, raised to robots.txt's `Crawl-delay` when that is
longer), with a User-Agent naming this tool and repository. A page fetched
within `--max-age` days (default 7) is not asked again; when every page is
fresh nothing is asked at all, robots.txt included. A page that answers
anything but HTTP 200 stops the run with an error and leaves its cache entry
as it was. The cache, by default
`%LOCALAPPDATA%\\ExilePact2\\crafting-helper\\poe2db-cache\\`, holds the raw
pages and `index.json` (one record per page: URL, fetch date, HTTP status,
file); it is refused inside a git checkout, because raw pages are never
tracked.

`extract` works offline from the cache. Each item-class page embeds one
`new ModsView({...})` call whose argument is JSON; from it the tool keeps the
`normal` and `desecrated` modifier rows, the `essence` and `perfect_essence`
rows (essences and alloys) and the `liquid` rows (liquid emotions, jewels
only), and counts every other non-empty group under the page's `ignored`
(corruption outcomes, rune and soul-core augments, league pools). The base
types come from the page's `...Item` tab (or, on a jewel page, the page's own
item). Modifier text keeps the displayed lines only: hidden stats
(`span.secondary`) are dropped, keyword links keep their text, `<br>` starts a
new line, and every number or range becomes `#`, its values kept beside the
template in order. Rows with the same section, kind, family list and lines are
one modifier, with tiers numbered per attribute variant from the highest item
level down (equal levels: higher values first; two rows alike in every field
are one tier). Essence and liquid rows alike after this reduction are one
entry listing the variants it appeared on, and each is matched, per variant,
to the modifier tier it adds: same kind and family, a tier of the same item
level, the values deciding between several, and null when none fits. Every class file is checked with `check_data` before
anything is written, and nothing at all is written unless every class passes;
a page whose structure the tool does not know (no ModsView, an unexpected
item class or variant list, an unknown non-empty row group, a missing key, an
unparsable value, a base count that differs from its tab header) stops it with
a non-zero exit naming the page. The output, by default
`apps/crafting-helper/data/`, is `index.json` and `classes/<class>.json`;
every list in it is sorted by content, so a refresh diffs cleanly.

`check` loads `index.json` and every class file it names, prints each problem
and exits 1 on any, 0 when the files keep the schema.

The data files keep modifier lines as `#` templates, affix and essence names,
and the source URL and fetch date of every page, nothing else, under the
owner's 2026-10-24 decision on short game text (AGENTS.md § "Legal").
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import os
import re
import sys
import urllib.parse
from collections import namedtuple
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
USER_AGENT = (
    "ExilePact2-crafting-helper-poe2db/1 (developer data tool, one request at a time; "
    "+https://github.com/S-Borkowski/ExilePact2)"
)
MIN_DELAY = 2.0
DEFAULT_DELAY = 3.0
DEFAULT_MAX_AGE = 7
CACHE_INDEX = "index.json"
DATA_INDEX = "index.json"
DEFAULT_DATA = Path(__file__).resolve().parents[2] / "apps" / "crafting-helper" / "data"

SCHEMA = 1
SOURCE_NOTE = (
    "Item class modifier data from poe2db (poe2db.tw; its wiki content is CC BY-NC-SA 3.0); "
    "modifier lines, affix names, base names and essence names are game text by Grinding Gear "
    "Games, quoted under the owner's 2026-10-24 decision. Values are replaced by # and kept "
    "beside each line; hidden stats are left out."
)


# --------------------------------------------------------------------------
# The class table: the item classes in scope and the pages that carry them

Page = namedtuple("Page", "variant slug")
ItemClass = namedtuple("ItemClass", "name code pages jewel")

ARMOUR_SIX = ("str", "dex", "int", "str_dex", "str_int", "dex_int")


def _armour(name: str, code: str, prefix: str, variants) -> ItemClass:
    return ItemClass(name, code, tuple(Page(v, "%s_%s" % (prefix, v)) for v in variants), False)


def _single(name: str, code: str, slug: str) -> ItemClass:
    return ItemClass(name, code, (Page("all", slug),), False)


def _jewels(name: str, code: str, slugs) -> ItemClass:
    return ItemClass(name, code, tuple(Page(slug.lower().replace("_", "-"), slug) for slug in slugs), True)


# `name` is the game's "Item Class:" text (the key M2 and M4 join on), `code`
# poe2db's `opt.ItemClassesCode` for the class, in the roadmap's order.
CLASS_TABLE = (
    _armour("Gloves", "Gloves", "Gloves", ARMOUR_SIX),
    _armour("Boots", "Boots", "Boots", ARMOUR_SIX),
    _armour("Helmets", "Helmet", "Helmets", ARMOUR_SIX),
    _armour("Body Armours", "Body Armour", "Body_Armours", ARMOUR_SIX + ("str_dex_int",)),
    _armour("Shields", "Shield", "Shields", ("str", "str_dex", "str_int")),
    _single("Quivers", "Quiver", "Quivers"),
    _single("Bucklers", "Buckler", "Bucklers"),
    _single("Foci", "Focus", "Foci"),
    _single("Wands", "Wand", "Wands"),
    _single("One Hand Maces", "One Hand Mace", "One_Hand_Maces"),
    _single("Sceptres", "Sceptre", "Sceptres"),
    _single("Spears", "Spear", "Spears"),
    _single("Bows", "Bow", "Bows"),
    _single("Staves", "Staff", "Staves"),
    _single("Two Hand Maces", "Two Hand Mace", "Two_Hand_Maces"),
    _single("Quarterstaves", "Warstaff", "Quarterstaves"),
    _single("Crossbows", "Crossbow", "Crossbows"),
    _single("Talismans", "Talisman", "Talismans"),
    _single("Belts", "Belt", "Belts"),
    _single("Amulets", "Amulet", "Amulets"),
    _single("Rings", "Ring", "Rings"),
    _jewels("Jewels", "Jewel", ("Ruby", "Emerald", "Sapphire", "Diamond", "Time-Lost_Ruby",
                                 "Time-Lost_Emerald", "Time-Lost_Sapphire", "Time-Lost_Diamond")),
)


def page_url(slug: str) -> str:
    return "%s/us/%s" % (SITE, slug)


def kebab(text: str) -> str:
    return "-".join(re.findall(r"[a-z0-9]+", text.lower()))


def class_file(item_class: ItemClass) -> str:
    return "classes/%s.json" % kebab(item_class.name)


def default_cache_dir() -> Path:
    base = os.environ.get("LOCALAPPDATA") or str(Path.home() / "AppData" / "Local")
    return Path(base) / "ExilePact2" / "crafting-helper" / "poe2db-cache"


class ExtractError(ToolError):
    """A page's structure is not what the parser knows; the message starts with the page."""


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
        return {"schema": 1, "pages": {}}
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


def _all_pages(table) -> list[Page]:
    return [page for item_class in table for page in item_class.pages]


def fetch(cache: Path, delay: float = DEFAULT_DELAY, max_age: int = DEFAULT_MAX_AGE,
          fetcher=None, clock=None, out=None, table=CLASS_TABLE) -> dict:
    """Fetch robots.txt and every stale page of `table` into `cache`.

    Returns {"fetched": [slug, ...], "cached": [slug, ...]}.
    """
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
    pages = _all_pages(table)
    stale = [p for p in pages if not _fresh(index["pages"].get(p.slug), cache, today, max_age)]
    cached = [p.slug for p in pages if p not in stale]
    if not stale:
        print("all %d pages cached within %d days; nothing asked" % (len(pages), max_age), file=out)
        return {"fetched": [], "cached": cached}

    session = Session(fetcher or urllib_fetcher, clock, float(delay))
    robots = _load_robots(session)
    crawl_delay = robots.crawl_delay()
    if crawl_delay is not None and crawl_delay > session.delay:
        session.delay = crawl_delay
    refused = [p.slug for p in stale if not robots.allows(page_url(p.slug))]
    if refused:
        raise ToolError("robots.txt disallows %s; nothing fetched" % ", ".join(refused))
    fetched: list[str] = []
    for page in stale:
        url = page_url(page.slug)
        status, body = session.get(url)
        if status != 200:
            raise ToolError("%s answered HTTP %s; its cache entry is unchanged (%d page(s) fetched before it)"
                            % (url, status, len(fetched)))
        cache.mkdir(parents=True, exist_ok=True)
        name = page.slug + ".html"
        (cache / name).write_bytes(body)
        index["pages"][page.slug] = {"slug": page.slug, "url": url, "fetched": today.isoformat(),
                                     "status": status, "file": name}
        _save_index(cache, index)
        fetched.append(page.slug)
        print("fetch %s: %d bytes" % (page.slug, len(body)), file=out)
    print("fetched %d page(s), %d already cached; cache %s" % (len(fetched), len(cached), cache), file=out)
    return {"fetched": fetched, "cached": cached}


# --------------------------------------------------------------------------
# Reading one page

KEPT_GROUPS = ("normal", "desecrated", "essence", "perfect_essence", "liquid")
SECTIONS = ("normal", "desecrated")
ESSENCE_GROUPS = ("essence", "perfect_essence")
# Dropped on purpose, counted under the page's `ignored`: corruption outcomes,
# rune and soul-core augments, and league pools whose crafting source is not
# established.
IGNORED_GROUPS = frozenset({
    "corrupted", "corruption_upgrade", "socketable", "bonded", "marksman", "decay", "soul",
    "destruction", "berserking", "chronomancy", "rotmother",
})
IGNORED_PREFIXES = ("breach_",)
# Legacy groups every page carries empty; a non-empty one is a surprise.
EMPTY_GROUPS = frozenset({
    "master", "delve", "incursion", "elder", "shaper", "crusader", "redeemer", "hunter", "warlord",
    "veiled", "scourgeup", "scourgedown", "bestiary", "haunted", "sentinel", "synthesis",
    "synthesis_corrupted", "infamous", "searing", "eater", "enchant", "warbands", "graft_corrupted",
})
# Top-level lists that are not row groups (a jewel page's `ids`).
NOT_GROUPS = frozenset({"ids"})
KINDS = {"1": "prefix", "2": "suffix"}
ROW_KEYS = {
    "normal": ("Name", "Level", "ModGenerationTypeID", "ModFamilyList", "str"),
    "desecrated": ("Name", "Level", "ModGenerationTypeID", "ModFamilyList", "str"),
    "essence": ("Name", "Level", "ModGenerationTypeID", "ModFamilyList", "str", "IsPerfect", "IsAlloy",
                "Removes"),
    "perfect_essence": ("Name", "Level", "ModGenerationTypeID", "ModFamilyList", "str", "IsPerfect",
                        "IsAlloy", "Removes"),
    "liquid": ("Name", "Level", "ModGenerationTypeID", "ModFamilyList", "str"),
}
WEIGHTS = ("spawn", "not-established", "none")

_MODS_VIEW = "new ModsView("
_NUM = r"[+-]?\d+(?:\.\d+)?"
# `(5–8)`, `(0.1—0.5)`, `(-10—-5)`: a range as poe2db prints it, then any lone number.
_VALUE_RE = re.compile(r"\(\s*(%s)\s*[–—-]\s*(%s)\s*\)|(\d+(?:\.\d+)?)" % (_NUM, _NUM))
# What a `span.mod-value` may hold: a signed range or number, maybe with a `%`.
_MOD_VALUE_RE = re.compile(r"^[+-]?(?:\(\s*%s\s*[–—-]\s*%s\s*\)|\d+(?:\.\d+)?)%%?$" % (_NUM, _NUM))
_COUNT_RE = re.compile(r"/\s*(\d+)\s*$")
_LEVEL_RE = re.compile(r"\bLevel\s+(\d+)\b")
_SKIP_TEXT_TAGS = frozenset({"script", "style", "template"})


def _number(text: str):
    value = float(text)
    return int(value) if value.is_integer() and "." not in text else value


def template(text: str) -> tuple[str, list[list]]:
    """One displayed line as a `#` template, and one [min, max] per `#`."""
    values: list[list] = []

    def swap(match: re.Match) -> str:
        if match.group(3) is not None:
            n = _number(match.group(3))
            values.append([n, n])
        else:
            values.append([_number(match.group(1)), _number(match.group(2))])
        return "#"

    line = _VALUE_RE.sub(swap, text)
    return " ".join(line.split()), values


def mod_text(html: str) -> tuple[list[str], list[list]]:
    """A modifier's displayed lines as templates and their values, in order.

    Raises ValueError for a `span.mod-value` that holds something other than a
    number or a range.
    """
    return _node_text(parse_html(html))


def _node_text(node: Node) -> tuple[list[str], list[list]]:
    pieces: list[str] = []

    def walk(n: Node) -> None:
        for child in n.children:
            if isinstance(child, str):
                pieces.append(child)
            elif child.tag == "br":
                pieces.append("\n")
            elif child.has_class("secondary"):
                continue
            elif child.tag in _SKIP_TEXT_TAGS:
                continue
            else:
                if child.has_class("mod-value"):
                    shown = "".join(child.text().split())
                    if not _MOD_VALUE_RE.match(shown):
                        raise ValueError("a value reads %r" % shown)
                walk(child)

    walk(node)
    lines: list[str] = []
    values: list[list] = []
    for raw in "".join(pieces).split("\n"):
        line, found = template(raw)
        if line:
            lines.append(line)
            values.extend(found)
    return lines, values


def mods_view(html: str, where: str) -> dict:
    """The JSON object a page passes to `new ModsView(...)`."""
    count = html.count(_MODS_VIEW)
    if count != 1:
        raise ExtractError("%s: the page holds %d `new ModsView(` calls, not 1" % (where, count))
    start = html.find("{", html.index(_MODS_VIEW))
    try:
        data, _ = json.JSONDecoder().raw_decode(html, start)
    except (ValueError, json.JSONDecodeError) as err:
        raise ExtractError("%s: the ModsView argument is not JSON (%s)" % (where, err)) from None
    if not isinstance(data, dict):
        raise ExtractError("%s: the ModsView argument is not an object" % where)
    return data


def _int(value, what: str, where: str) -> int:
    try:
        text = str(value).strip()
        if isinstance(value, bool) or not re.fullmatch(r"-?\d+", text):
            raise ValueError
        return int(text)
    except ValueError:
        raise ExtractError("%s: %s %r is not a whole number" % (where, what, value)) from None


def _kind(value, where: str) -> str:
    kind = KINDS.get(str(value).strip())
    if kind is None:
        raise ExtractError("%s: generation type %r is neither 1 (prefix) nor 2 (suffix)" % (where, value))
    return kind


def _family(value, where: str) -> list[str]:
    if not isinstance(value, list) or not all(isinstance(f, str) and f for f in value):
        raise ExtractError("%s: ModFamilyList %r is not a list of names" % (where, value))
    return list(value)


def _plain(value, where: str) -> str:
    if not isinstance(value, str):
        raise ExtractError("%s: Name %r is not text" % (where, value))
    text = parse_html(value).text() if "<" in value else value
    return " ".join(text.split())


def _anchor(value, where: str) -> tuple[str, str]:
    """(name, slug) of the item link an essence or liquid row names."""
    link = parse_html(value).find("a") if isinstance(value, str) else None
    href = link.attrs.get("href", "") if link is not None else ""
    name = " ".join(link.text().split()) if link is not None else ""
    slug = href.rsplit("/", 1)[-1]
    if not name or not slug:
        raise ExtractError("%s: Name does not link an item (%r)" % (where, str(value)[:80]))
    return name, slug


def _lines(row: dict, where: str) -> tuple[list[str], list[list]]:
    if not isinstance(row["str"], str):
        raise ExtractError("%s: str is not text" % where)
    try:
        lines, values = mod_text(row["str"])
    except ValueError as err:
        raise ExtractError("%s: %s" % (where, err)) from None
    if not lines:
        raise ExtractError("%s: no displayed line" % where)
    return lines, values


def _row(group: str, row, where: str) -> dict:
    if not isinstance(row, dict):
        raise ExtractError("%s: the row is not an object" % where)
    missing = [key for key in ROW_KEYS[group] if key not in row]
    if missing:
        raise ExtractError("%s: the row has no %s" % (where, ", ".join(missing)))
    lines, values = _lines(row, where)
    out = {"kind": _kind(row["ModGenerationTypeID"], where), "family": _family(row["ModFamilyList"], where),
           "lines": lines, "values": values, "ilvl": _int(row["Level"], "Level", where)}
    if group in SECTIONS:
        out["name"] = _plain(row["Name"], where)
        chance = row.get("DropChance")
        out["weight"] = None if chance is None else _int(chance, "DropChance", where)
    else:
        out["name"], out["slug"] = _anchor(row["Name"], where)
    if group in ESSENCE_GROUPS:
        for key in ("IsPerfect", "IsAlloy", "Removes"):
            if row[key] not in (0, 1, "0", "1", True, False):
                raise ExtractError("%s: %s %r is not a flag" % (where, key, row[key]))
        out["perfect"] = row["IsPerfect"] in (1, "1", True)
        out["alloy"] = row["IsAlloy"] in (1, "1", True)
        out["removes"] = row["Removes"] in (1, "1", True)
    return out


def _weights(rows: list[dict]) -> str:
    if not rows:
        return "none"
    if any(r["weight"] is not None and r["weight"] > 1 for r in rows):
        return "spawn"
    return "not-established"


def _base_tab(root: Node, where: str) -> Node:
    tabs = [n for n in root.find_all(cls="tab-pane") if n.attrs.get("id", "").endswith("Item")]
    if len(tabs) != 1:
        raise ExtractError("%s: %d base-item tabs (div.tab-pane#...Item), not 1" % (where, len(tabs)))
    return tabs[0]


def _bases(root: Node, variant: str, where: str) -> list[dict]:
    tab = _base_tab(root, where)
    tab_id = tab.attrs["id"]
    header = tab.find(cls="card-header")
    match = _COUNT_RE.search(header.text()) if header is not None else None
    if match is None:
        raise ExtractError("%s: the %s tab header has no /N count" % (where, tab_id))
    bases: list[dict] = []
    for card in tab.find_all(cls="flex-grow-1"):
        link = card.find("a", cls="whiteitem")
        if link is None:
            continue
        name = " ".join(link.text().split())
        if not name:
            raise ExtractError("%s: a base in %s has no name" % (where, tab_id))
        req = card.find(cls="requirements")
        level = _LEVEL_RE.search(req.text()) if req is not None else None
        implicits = []
        for node in card.find_all(cls="implicitMod"):
            try:
                lines, values = _node_text(node)
            except ValueError as err:
                raise ExtractError("%s: base %s: %s" % (where, name, err)) from None
            if lines:
                implicits.append({"lines": lines, "values": values})
        bases.append({"name": name, "variant": variant, "req_level": int(level.group(1)) if level else None,
                      "implicits": implicits})
    if len(bases) != int(match.group(1)):
        raise ExtractError("%s: the %s tab header says /%s but it lists %d bases"
                           % (where, tab_id, match.group(1), len(bases)))
    return bases


def parse_page(html: str, item_class: ItemClass, page: Page, fetched: str) -> dict:
    """What one cached page holds, reduced: its record, bases and kept rows by group."""
    where = page.slug
    view = mods_view(html, where)
    opt = view.get("opt") if isinstance(view.get("opt"), dict) else {}
    if opt.get("ItemClassesCode") != item_class.code:
        raise ExtractError("%s: opt.ItemClassesCode is %r, not %r" % (where, opt.get("ItemClassesCode"),
                                                                    item_class.code))
    baseitem = view.get("baseitem") if isinstance(view.get("baseitem"), dict) else {}
    opts = baseitem.get("opts")
    variants = [p.variant for p in item_class.pages]
    if opts and (not isinstance(opts, dict) or sorted(opts) != sorted(variants)):
        raise ExtractError("%s: baseitem.opts lists %s, not the table's %s"
                           % (where, sorted(opts) if isinstance(opts, dict) else opts, variants))
    rows: dict[str, list[dict]] = {}
    ignored: dict[str, int] = {}
    for key, value in view.items():
        if not isinstance(value, list) or key in NOT_GROUPS:
            continue
        if key in KEPT_GROUPS:
            rows[key] = [_row(key, row, "%s %s row %d" % (where, key, n)) for n, row in enumerate(value, 1)]
        elif not value:
            continue
        elif key in IGNORED_GROUPS or key.startswith(IGNORED_PREFIXES):
            ignored[key] = len(value)
        else:
            raise ExtractError("%s: the row group %r holds %d rows and is neither kept nor known to be "
                               "dropped" % (where, key, len(value)))
    if not rows.get("normal"):
        raise ExtractError("%s: the normal group is missing or empty" % where)
    if item_class.jewel:
        code = baseitem.get("Code")
        if not isinstance(code, str) or code.replace(" ", "_") != page.slug:
            raise ExtractError("%s: baseitem.Code is %r, not this jewel" % (where, code))
        bases = [{"name": code, "variant": page.variant, "req_level": None, "implicits": []}]
    else:
        bases = _bases(parse_html(html), page.variant, where)
    return {
        "page": {"variant": page.variant, "url": page_url(page.slug), "fetched": fetched,
                 "weights": {s: _weights(rows.get(s, [])) for s in SECTIONS},
                 "ignored": dict(sorted(ignored.items()))},
        "bases": bases,
        "rows": {g: rows.get(g, []) for g in KEPT_GROUPS},
    }


# --------------------------------------------------------------------------
# Building one class file

def _desc(values: list[list]) -> tuple:
    return tuple(-x for pair in values for x in pair)


def _text_key(lines: list[str]) -> tuple:
    return tuple(line.lower() for line in lines), tuple(lines)


def mod_sort_key(mod: dict):
    return (SECTIONS.index(mod["section"]), mod["kind"], _text_key(mod["lines"]), mod["family"])


def base_sort_key(base: dict):
    return (base["name"].lower(), base["name"], base["variant"], base["req_level"] is None,
            base["req_level"] or 0, json.dumps(base["implicits"]))


def entry_sort_key(entry: dict):
    return (entry["name"].lower(), entry["name"], entry["kind"], _text_key(entry["lines"]), entry["family"],
            json.dumps(entry["values"]), entry["ilvl"], entry.get("group", ""), entry.get("perfect", False),
            entry.get("alloy", False), entry.get("removes", False))


def _slug(lines: list[str]) -> str:
    return "-".join(re.findall(r"[a-z]+", lines[0].lower())) or "mod"


def _matches(entry: dict, mods: list[dict]) -> dict:
    """Per variant, the modifier tier the essence or liquid row adds, or None."""
    found = {}
    for variant in entry["variants"]:
        pairs = [(mod, tier) for mod in mods if mod["kind"] == entry["kind"] and mod["family"] == entry["family"]
                 for tier in mod["tiers"].get(variant, []) if tier["ilvl"] == entry["ilvl"]]
        if len(pairs) > 1:
            pairs = [(mod, tier) for mod, tier in pairs if tier["values"] == entry["values"]]
        found[variant] = {"mod": pairs[0][0]["id"], "tier": pairs[0][1]["tier"]} if len(pairs) == 1 else None
    return found


def build_class(item_class: ItemClass, pages: list[dict]) -> dict:
    """The class file from the parsed pages of `item_class`, in table order."""
    order = [p.variant for p in item_class.pages]
    groups: dict[tuple, dict] = {}
    bases: dict[str, dict] = {}
    entries = {"essence": {}, "liquid": {}}
    for parsed in pages:
        variant = parsed["page"]["variant"]
        for base in parsed["bases"]:
            bases.setdefault(json.dumps(base, sort_keys=True), base)
        for section in SECTIONS:
            for row in parsed["rows"][section]:
                key = (section, row["kind"], tuple(row["family"]), tuple(row["lines"]))
                group = groups.setdefault(key, {"section": section, "kind": row["kind"], "family": row["family"],
                                                "lines": row["lines"], "tiers": {}})
                group["tiers"].setdefault(variant, []).append(
                    {"name": row["name"], "ilvl": row["ilvl"], "weight": row["weight"], "values": row["values"]})
        for source, target in (("essence", "essence"), ("perfect_essence", "essence"), ("liquid", "liquid")):
            for row in parsed["rows"][source]:
                entry = {"name": row["name"], "slug": row["slug"]}
                if target == "essence":
                    entry.update({"group": source, "perfect": row["perfect"], "alloy": row["alloy"],
                                  "removes": row["removes"]})
                entry.update({"kind": row["kind"], "ilvl": row["ilvl"], "family": row["family"],
                              "lines": row["lines"], "values": row["values"]})
                kept = entries[target].setdefault(json.dumps(entry, sort_keys=True), dict(entry, variants=[]))
                if variant not in kept["variants"]:
                    kept["variants"].append(variant)

    mods: list[dict] = []
    used: dict[str, int] = {}
    for group in sorted(groups.values(), key=mod_sort_key):
        base = ("desecrated-" if group["section"] == "desecrated" else "") + group["kind"] + "-" + _slug(group["lines"])
        used[base] = used.get(base, 0) + 1
        ident = base if used[base] == 1 else "%s-%d" % (base, used[base])
        tiers = {}
        for variant in order:
            rows = group["tiers"].get(variant)
            if not rows:
                continue
            # Two rows alike in every field (poe2db lists some desecrated mods
            # twice) are one tier: nothing on the item tells them apart.
            rows = list({json.dumps(r, sort_keys=True): r for r in rows}.values())
            rows = sorted(rows, key=lambda r: (-r["ilvl"], _desc(r["values"]), r["name"],
                                               -1 if r["weight"] is None else -r["weight"]))
            tiers[variant] = [{"tier": n, "name": r["name"], "ilvl": r["ilvl"], "weight": r["weight"],
                               "values": r["values"]} for n, r in enumerate(rows, 1)]
        mods.append({"id": ident, "section": group["section"], "kind": group["kind"], "family": group["family"],
                     "lines": group["lines"], "tiers": tiers})

    def finish(found: dict) -> list[dict]:
        done = []
        for entry in sorted(found.values(), key=entry_sort_key):
            entry["variants"] = [v for v in order if v in entry["variants"]]
            entry["matches"] = _matches(entry, mods)
            done.append(entry)
        return done

    return {
        "schema": SCHEMA, "class": item_class.name, "code": item_class.code, "source_note": SOURCE_NOTE,
        "pages": [p["page"] for p in pages],
        "bases": sorted(bases.values(), key=base_sort_key),
        "mods": mods,
        "essences": finish(entries["essence"]),
        "liquid": finish(entries["liquid"]),
    }


def build_index(table=CLASS_TABLE) -> dict:
    return {"schema": SCHEMA, "source_note": SOURCE_NOTE,
            "classes": [{"class": c.name, "file": class_file(c), "variants": [p.variant for p in c.pages]}
                        for c in table]}


# --------------------------------------------------------------------------
# The rules the data files are held to

INDEX_KEYS = ["schema", "source_note", "classes"]
INDEX_CLASS_KEYS = ["class", "file", "variants"]
CLASS_KEYS = ["schema", "class", "code", "source_note", "pages", "bases", "mods", "essences", "liquid"]
PAGE_KEYS = ["variant", "url", "fetched", "weights", "ignored"]
BASE_KEYS = ["name", "variant", "req_level", "implicits"]
MOD_KEYS = ["id", "section", "kind", "family", "lines", "tiers"]
TIER_KEYS = ["tier", "name", "ilvl", "weight", "values"]
ESSENCE_KEYS = ["name", "slug", "group", "perfect", "alloy", "removes", "kind", "ilvl", "family", "lines",
                "values", "variants", "matches"]
LIQUID_KEYS = ["name", "slug", "kind", "ilvl", "family", "lines", "values", "variants", "matches"]
MAX_LINE = 200
_ID_RE = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")
_BAD_TEXT_RE = re.compile(r"[\[<>0-9]")
_BAD_NAME_RE = re.compile(r"[\[<>]")


def _is_int(value) -> bool:
    return isinstance(value, int) and not isinstance(value, bool)


def _is_number(value) -> bool:
    return isinstance(value, (int, float)) and not isinstance(value, bool)


def _check_text(where: str, lines, values, problems: list[str]) -> None:
    if not isinstance(lines, list) or not lines or not all(isinstance(x, str) and x for x in lines):
        problems.append("%s: lines is not a list of text" % where)
        return
    for line in lines:
        if _BAD_TEXT_RE.search(line):
            problems.append("%s: line %r holds a digit, `[` or markup" % (where, line))
        if len(line) > MAX_LINE:
            problems.append("%s: a line is %d characters, over %d" % (where, len(line), MAX_LINE))
        if line != " ".join(line.split()):
            problems.append("%s: line %r has stray whitespace" % (where, line))
    if not isinstance(values, list) or not all(
            isinstance(v, list) and len(v) == 2 and all(_is_number(x) for x in v) for v in values):
        problems.append("%s: values is not a list of [min, max] numbers" % where)
    elif len(values) != sum(line.count("#") for line in lines):
        problems.append("%s: %d values for %d `#` in the lines"
                        % (where, len(values), sum(line.count("#") for line in lines)))


def _check_family(where: str, family, problems: list[str]) -> None:
    if not isinstance(family, list) or not all(isinstance(f, str) and f for f in family):
        problems.append("%s: family is not a list of names" % where)


def _check_name(where: str, name, problems: list[str], empty_ok: bool = False) -> None:
    if not isinstance(name, str) or (not empty_ok and not name.strip()) or _BAD_NAME_RE.search(name) \
            or name != " ".join(name.split()):
        problems.append("%s: name %r is not plain text" % (where, name))


def _sorted_by(items: list, key) -> bool:
    try:
        return items == sorted(items, key=key)
    except (TypeError, KeyError):
        return False


def check_data(data: dict, item_class: ItemClass | None = None) -> list[str]:
    """Every way one class file breaks the data rules; empty when it keeps them."""
    problems: list[str] = []
    if not isinstance(data, dict):
        return ["the class file is not an object"]
    if list(data) != CLASS_KEYS:
        return ["top-level keys are %s, not %s" % (list(data), CLASS_KEYS)]
    if data["schema"] != SCHEMA:
        problems.append("schema is %r, not %d" % (data["schema"], SCHEMA))
    if not isinstance(data["class"], str) or not data["class"]:
        problems.append("class is %r" % (data["class"],))
    if item_class is not None and (data["class"], data["code"]) != (item_class.name, item_class.code):
        problems.append("class and code are %r, %r, not %r, %r" % (data["class"], data["code"], item_class.name,
                                                                 item_class.code))
    if not isinstance(data["source_note"], str) or "poe2db" not in data["source_note"]:
        problems.append("source_note does not name poe2db")

    pages = data["pages"]
    variants: list[str] = []
    if not isinstance(pages, list) or not pages:
        problems.append("pages is empty or not a list")
        pages = []
    for n, page in enumerate(pages):
        where = "page %d" % n
        if not isinstance(page, dict) or list(page) != PAGE_KEYS:
            problems.append("%s: keys are not %s" % (where, PAGE_KEYS))
            continue
        if not isinstance(page["variant"], str) or not page["variant"] or page["variant"] in variants:
            problems.append("%s: variant %r is empty or repeated" % (where, page["variant"]))
        variants.append(page["variant"])
        if not isinstance(page["url"], str) or not page["url"].startswith(SITE + "/us/"):
            problems.append("%s: url %r is not a poe2db page" % (where, page["url"]))
        try:
            _dt.date.fromisoformat(page["fetched"])
        except (TypeError, ValueError):
            problems.append("%s: fetched %r is not an ISO date" % (where, page["fetched"]))
        weights = page["weights"]
        if not isinstance(weights, dict) or list(weights) != list(SECTIONS) \
                or not all(w in WEIGHTS for w in weights.values()):
            problems.append("%s: weights %r" % (where, weights))
        ignored = page["ignored"]
        if not isinstance(ignored, dict) or not all(_is_int(c) and c > 0 for c in ignored.values()) \
                or list(ignored) != sorted(ignored):
            problems.append("%s: ignored %r" % (where, ignored))
    if item_class is not None and variants != [p.variant for p in item_class.pages]:
        problems.append("page variants are %s, not %s" % (variants, [p.variant for p in item_class.pages]))

    bases = data["bases"]
    if not isinstance(bases, list) or not bases:
        problems.append("bases is empty or not a list")
        bases = []
    for n, base in enumerate(bases):
        where = "base %d" % n
        if not isinstance(base, dict) or list(base) != BASE_KEYS:
            problems.append("%s: keys are not %s" % (where, BASE_KEYS))
            continue
        _check_name(where, base["name"], problems)
        if base["variant"] not in variants:
            problems.append("%s: variant %r has no page" % (where, base["variant"]))
        if base["req_level"] is not None and not (_is_int(base["req_level"]) and base["req_level"] > 0):
            problems.append("%s: req_level %r" % (where, base["req_level"]))
        if not isinstance(base["implicits"], list):
            problems.append("%s: implicits is not a list" % where)
            continue
        for k, implicit in enumerate(base["implicits"]):
            if not isinstance(implicit, dict) or list(implicit) != ["lines", "values"]:
                problems.append("%s implicit %d: keys are not lines, values" % (where, k))
            else:
                _check_text("%s implicit %d" % (where, k), implicit["lines"], implicit["values"], problems)
    if bases and all(isinstance(b, dict) and list(b) == BASE_KEYS for b in bases) \
            and not _sorted_by(bases, base_sort_key):
        problems.append("bases are not sorted by content")

    mods = data["mods"]
    tiers_of: dict[str, dict] = {}
    if not isinstance(mods, list) or not mods:
        problems.append("mods is empty or not a list")
        mods = []
    keys: set[tuple] = set()
    for n, mod in enumerate(mods):
        where = "mod %d (%s)" % (n, mod.get("id") if isinstance(mod, dict) else "?")
        if not isinstance(mod, dict) or list(mod) != MOD_KEYS:
            problems.append("%s: keys are not %s" % (where, MOD_KEYS))
            continue
        ident = mod["id"]
        if not isinstance(ident, str) or not _ID_RE.match(ident):
            problems.append("%s: id is not a kebab slug" % where)
        elif ident in tiers_of:
            problems.append("%s: id is used twice" % where)
        if mod["section"] not in SECTIONS:
            problems.append("%s: section %r" % (where, mod["section"]))
        if mod["kind"] not in KINDS.values():
            problems.append("%s: kind %r" % (where, mod["kind"]))
        _check_family(where, mod["family"], problems)
        lines = mod["lines"]
        # The line rules of _check_text; a mod's values live on its tiers.
        if not isinstance(lines, list) or not lines or not all(isinstance(x, str) and x for x in lines):
            problems.append("%s: lines is not a list of text" % where)
            continue
        hashes = sum(line.count("#") for line in lines)
        for line in lines:
            if _BAD_TEXT_RE.search(line):
                problems.append("%s: line %r holds a digit, `[` or markup" % (where, line))
            if len(line) > MAX_LINE or line != " ".join(line.split()):
                problems.append("%s: line %r is too long or has stray whitespace" % (where, line))
        key = (mod["section"], mod["kind"], json.dumps(mod["family"]), tuple(lines))
        if key in keys:
            problems.append("%s: the same section, kind, family and lines as another mod" % where)
        keys.add(key)
        tiers = mod["tiers"]
        if not isinstance(tiers, dict) or not tiers:
            problems.append("%s: tiers is empty or not an object" % where)
            continue
        tiers_of[ident] = tiers
        if [v for v in variants if v in tiers] != list(tiers):
            problems.append("%s: tier variants %s are not page variants in page order" % (where, list(tiers)))
        for variant, rows in tiers.items():
            at = "%s %s" % (where, variant)
            if not isinstance(rows, list) or not rows:
                problems.append("%s: no tiers" % at)
                continue
            if not all(isinstance(r, dict) and list(r) == TIER_KEYS for r in rows):
                problems.append("%s: tier keys are not %s" % (at, TIER_KEYS))
                continue
            if [r["tier"] for r in rows] != list(range(1, len(rows) + 1)):
                problems.append("%s: tiers are numbered %s, not 1..%d without gaps"
                                % (at, [r["tier"] for r in rows], len(rows)))
            for r in rows:
                if not _is_int(r["ilvl"]) or r["ilvl"] < 0:
                    problems.append("%s tier %s: ilvl %r" % (at, r["tier"], r["ilvl"]))
                if r["weight"] is not None and not (_is_int(r["weight"]) and r["weight"] >= 0):
                    problems.append("%s tier %s: weight %r" % (at, r["tier"], r["weight"]))
                _check_name("%s tier %s" % (at, r["tier"]), r["name"], problems, empty_ok=True)
                values = r["values"]
                if not isinstance(values, list) or not all(
                        isinstance(v, list) and len(v) == 2 and all(_is_number(x) for x in v) for v in values):
                    problems.append("%s tier %s: values is not a list of [min, max] numbers" % (at, r["tier"]))
                elif len(values) != hashes:
                    problems.append("%s tier %s: %d values for %d `#` in the lines"
                                    % (at, r["tier"], len(values), hashes))
            ilvls = [r["ilvl"] for r in rows if _is_int(r["ilvl"])]
            if ilvls != sorted(ilvls, reverse=True):
                problems.append("%s: tier 1 is not the highest item level" % at)
    if mods and all(isinstance(m, dict) and list(m) == MOD_KEYS for m in mods) \
            and not _sorted_by(mods, mod_sort_key):
        problems.append("mods are not sorted by content")

    for field, keys_wanted in (("essences", ESSENCE_KEYS), ("liquid", LIQUID_KEYS)):
        entries = data[field]
        if not isinstance(entries, list):
            problems.append("%s is not a list" % field)
            continue
        for n, entry in enumerate(entries):
            where = "%s %d" % (field, n)
            if not isinstance(entry, dict) or list(entry) != keys_wanted:
                problems.append("%s: keys are not %s" % (where, keys_wanted))
                continue
            _check_name(where, entry["name"], problems)
            if not isinstance(entry["slug"], str) or not re.fullmatch(r"[A-Za-z0-9_'%.()-]+", entry["slug"]):
                problems.append("%s: slug %r" % (where, entry["slug"]))
            if field == "essences":
                if entry["group"] not in ESSENCE_GROUPS:
                    problems.append("%s: group %r" % (where, entry["group"]))
                for flag in ("perfect", "alloy", "removes"):
                    if not isinstance(entry[flag], bool):
                        problems.append("%s: %s is not true or false" % (where, flag))
            if entry["kind"] not in KINDS.values():
                problems.append("%s: kind %r" % (where, entry["kind"]))
            if not _is_int(entry["ilvl"]) or entry["ilvl"] < 0:
                problems.append("%s: ilvl %r" % (where, entry["ilvl"]))
            _check_family(where, entry["family"], problems)
            _check_text(where, entry["lines"], entry["values"], problems)
            found = entry["variants"]
            if not isinstance(found, list) or not found or [v for v in variants if v in found] != found:
                problems.append("%s: variants %r are not page variants in page order" % (where, found))
                continue
            matches = entry["matches"]
            if not isinstance(matches, dict) or list(matches) != found:
                problems.append("%s: matches does not name exactly its variants" % where)
                continue
            for variant, match in matches.items():
                if match is None:
                    continue
                if not isinstance(match, dict) or list(match) != ["mod", "tier"]:
                    problems.append("%s: match for %s is not {mod, tier} or null" % (where, variant))
                elif match["mod"] not in tiers_of:
                    problems.append("%s: match for %s names no mod %r" % (where, variant, match["mod"]))
                elif match["tier"] not in [r.get("tier") for r in tiers_of[match["mod"]].get(variant, [])
                                           if isinstance(r, dict)]:
                    problems.append("%s: match for %s names tier %r, which %s does not have there"
                                    % (where, variant, match["tier"], match["mod"]))
        if entries and all(isinstance(e, dict) and list(e) == keys_wanted for e in entries) \
                and not _sorted_by(entries, entry_sort_key):
            problems.append("%s are not sorted by content" % field)
    return problems


def check_index(index: dict, table=CLASS_TABLE) -> list[str]:
    """Every way `index.json` breaks the data rules; empty when it keeps them."""
    if not isinstance(index, dict) or list(index) != INDEX_KEYS:
        return ["index.json keys are not %s" % INDEX_KEYS]
    problems: list[str] = []
    if index["schema"] != SCHEMA:
        problems.append("index.json schema is %r, not %d" % (index["schema"], SCHEMA))
    if not isinstance(index["source_note"], str) or "poe2db" not in index["source_note"]:
        problems.append("index.json source_note does not name poe2db")
    if index["classes"] != build_index(table)["classes"]:
        problems.append("index.json classes are not the class table's %d classes, in order" % len(table))
    return problems


def dumps(data: dict) -> str:
    return json.dumps(data, indent=2, ensure_ascii=False) + "\n"


def check_dir(data_dir: Path, table=CLASS_TABLE) -> list[str]:
    """Every problem with the data files under `data_dir`."""
    data_dir = Path(data_dir)
    path = data_dir / DATA_INDEX
    if not path.is_file():
        return ["%s is missing; run `extract` first" % path]
    try:
        text = path.read_bytes().decode("utf-8").replace("\r\n", "\n")
        index = json.loads(text)
    except (UnicodeDecodeError, ValueError) as err:
        return ["%s: %s" % (path, err)]
    problems = check_index(index, table)
    if text != dumps(index):
        problems.append("index.json is not written as extract writes it")
    by_name = {c.name: c for c in table}
    for entry in index.get("classes") or []:
        if not isinstance(entry, dict) or not isinstance(entry.get("file"), str):
            continue
        file = data_dir / entry["file"]
        if not file.is_file():
            problems.append("%s: missing" % entry["file"])
            continue
        try:
            text = file.read_bytes().decode("utf-8").replace("\r\n", "\n")
            data = json.loads(text)
        except (UnicodeDecodeError, ValueError) as err:
            problems.append("%s: %s" % (entry["file"], err))
            continue
        problems.extend("%s: %s" % (entry["file"], p) for p in check_data(data, by_name.get(entry.get("class"))))
        if text != dumps(data):
            problems.append("%s: not written as extract writes it" % entry["file"])
    return problems


# --------------------------------------------------------------------------
# Extract and check

def extract(cache: Path, out_dir: Path | None = None, out=None, table=CLASS_TABLE) -> dict:
    """Write index.json and every class file from the cache. Returns {file: data} as written."""
    out = out or sys.stdout
    cache = Path(cache)
    index = _load_index(cache)
    missing = []
    for page in _all_pages(table):
        record = index["pages"].get(page.slug)
        if not record or record.get("status") != 200 or not record.get("file") \
                or not (cache / record["file"]).is_file():
            missing.append(page.slug)
    if missing:
        raise ToolError("no cached page for %s in %s; run `fetch` first" % (", ".join(missing), cache))
    files: dict[str, dict] = {}
    problems: list[str] = []
    for item_class in table:
        parsed = []
        for page in item_class.pages:
            record = index["pages"][page.slug]
            with open(cache / record["file"], encoding="utf-8", errors="replace") as fh:
                parsed.append(parse_page(fh.read(), item_class, page, record["fetched"]))
        data = build_class(item_class, parsed)
        problems.extend("%s: %s" % (item_class.name, p) for p in check_data(data, item_class))
        files[class_file(item_class)] = data
    if problems:
        raise ExtractError("the extracted data breaks the file rules, nothing written: " + "; ".join(problems[:8]))
    files[DATA_INDEX] = build_index(table)
    out_dir = Path(out_dir) if out_dir is not None else DEFAULT_DATA
    for name, data in files.items():
        path = out_dir / name
        path.parent.mkdir(parents=True, exist_ok=True)
        tmp = path.with_name(path.name + ".tmp")
        with open(tmp, "w", encoding="utf-8", newline="\n") as fh:
            fh.write(dumps(data))
        os.replace(tmp, path)
    for item_class in table:
        data = files[class_file(item_class)]
        print("%s: %d bases, %d mods, %d essences, %d liquid"
              % (item_class.name, len(data["bases"]), len(data["mods"]), len(data["essences"]),
                 len(data["liquid"])), file=out)
    print("wrote %d class files and %s under %s" % (len(table), DATA_INDEX, out_dir), file=out)
    return files


def check(data_dir: Path | None = None, out=None, table=CLASS_TABLE) -> list[str]:
    out = out or sys.stdout
    data_dir = Path(data_dir) if data_dir is not None else DEFAULT_DATA
    problems = check_dir(data_dir, table)
    for problem in problems:
        print(problem, file=out)
    print("%s: %s" % (data_dir, "%d problem(s)" % len(problems) if problems else "clean"), file=out)
    return problems


# --------------------------------------------------------------------------
# Command line

def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="py -3 -m tools.crafting_helper.poe2db",
        description="Developer-only: fetch poe2db.tw's item-class modifier pages, write the crafting "
                    "helper's data files from them, and check those files.")
    sub = parser.add_subparsers(dest="command", required=True, metavar="{fetch,extract,check}")

    f = sub.add_parser("fetch", help="fetch robots.txt and every stale class page into the cache, politely")
    f.add_argument("--cache", type=Path, default=None,
                   help="cache folder, outside any git checkout "
                        "(default %%LOCALAPPDATA%%\\ExilePact2\\crafting-helper\\poe2db-cache)")
    f.add_argument("--delay", type=float, default=DEFAULT_DELAY,
                   help="seconds between requests (default %s, minimum %s)" % (DEFAULT_DELAY, MIN_DELAY))
    f.add_argument("--max-age", type=int, default=DEFAULT_MAX_AGE, metavar="DAYS",
                   help="leave a page fetched within this many days alone (default %d)" % DEFAULT_MAX_AGE)

    e = sub.add_parser("extract", help="write index.json and the class files from the cached pages")
    e.add_argument("--cache", type=Path, default=None, help="cache folder (default as for fetch)")
    e.add_argument("--out", type=Path, default=None,
                   help="data folder (default apps/crafting-helper/data)")

    c = sub.add_parser("check", help="hold index.json and every class file to the schema")
    c.add_argument("--data", type=Path, default=None, help="data folder (default apps/crafting-helper/data)")
    return parser


def main(argv: list[str] | None = None, fetcher=None, clock=None, table=CLASS_TABLE) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    try:
        if args.command == "check":
            return 1 if check(args.data, table=table) else 0
        cache = args.cache if args.cache is not None else default_cache_dir()
        if args.command == "fetch":
            if not args.delay >= MIN_DELAY:
                print("poe2db: error: refusing --delay %s: the minimum is %s seconds between requests"
                      % (args.delay, MIN_DELAY), file=sys.stderr)
                return 2
            fetch(cache, delay=args.delay, max_age=args.max_age, fetcher=fetcher, clock=clock, table=table)
        else:
            extract(cache, out_dir=args.out, table=table)
    except ToolError as err:
        print("poe2db: error: %s" % err, file=sys.stderr)
        return 1
    except (OSError, json.JSONDecodeError) as err:
        print("poe2db: error: %s" % err, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
