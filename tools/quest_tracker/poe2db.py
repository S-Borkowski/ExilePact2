"""poe2db.tw quest pages: fetch them politely, draft quest data, measure log coverage.

Developer-only. This is the one piece of the quest tracker that touches the
network, and it only ever asks poe2db.tw:

    py -3 -m tools.quest_tracker.poe2db fetch [--cache DIR] [--delay S] [--only SLUG ...]
                                             [--max-age DAYS] [--include-endgame]
    py -3 -m tools.quest_tracker.poe2db extract [--cache DIR] [--out FILE] [--with-dialogue]
    py -3 -m tools.quest_tracker.poe2db coverage --log PATH [--cache DIR]

`fetch` reads robots.txt first and skips whatever it disallows (through
`urllib.robotparser`, plus the `*` and `$` patterns that module does not
understand), sends one request at a time at least `--delay` seconds apart
(default 3, never under 2), names this tool and repository in its User-Agent,
and leaves pages cached within `--max-age` days alone. The cache, by default
`%LOCALAPPDATA%\\ExilePact2\\quest-tracker\\poe2db-cache\\`, holds the raw pages
and `index.json` (slug, URL, fetch date, HTTP status per page); it is refused
inside a git checkout, because raw pages are never tracked. Only campaign
quests are fetched (no Act 10, no act-less MasterMission) unless
`--include-endgame`.

`extract` turns the cached pages into a draft JSON file, one entry per quest,
for a person to curate into `apps/quest-tracker/data/quests.json`. It stops
with a non-zero exit, naming the slug, when the index yields no quest or a
page has no steps table: a changed site fails loudly rather than drafting
nothing.

`coverage` measures how far a page's dialogue matches the lines a saved
`Client.txt` logged for the same NPC, and prints counts only, never text.
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import os
import re
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
import urllib.robotparser
from html.parser import HTMLParser
from pathlib import Path

SITE_HOST = "poe2db.tw"
SITE = "https://" + SITE_HOST
ROBOTS_URL = SITE + "/robots.txt"
INDEX_SLUG = "Quest"
USER_AGENT = (
    "ExilePact2-quest-tracker-poe2db/1 (developer data tool, one request at a time; "
    "+https://github.com/S-Borkowski/ExilePact2)"
)
MIN_DELAY = 2.0
DEFAULT_DELAY = 3.0
DEFAULT_MAX_AGE = 7
CACHE_INDEX = "index.json"
DRAFT_NAME = "quests-draft.json"
QUEST_TYPES = ("Normal", "Optional", "Important", "MasterMission")
ENDGAME_ACTS = frozenset({10})
# A quest slug is one path segment as the index links it, e.g. `Reaching_Clearfell`.
SLUG_RE = re.compile(r"^[A-Za-z0-9_%.,'()\-]+$")

# Client.txt: `YYYY/MM/DD HH:MM:SS <ms> <hex> [<LEVEL> Client <pid>] <message>`.
LOG_LINE_RE = re.compile(
    r"^\d{4}/\d{2}/\d{2} \d{2}:\d{2}:\d{2} \d+ [0-9A-Fa-f]+ \[[A-Z]+ Client \d+\] (.*)$"
)
# Chat channels (local, trade, global, whisper, guild) carry one of these
# before the speaker; they are other people, never NPCs.
CHAT_PREFIXES = "#$%@&"


class ToolError(Exception):
    """A refusal or failure the command line reports in one line, exit 1."""


class ExtractError(ToolError):
    """The site's structure is not what the parser knows; the message starts with the slug."""


def page_url(slug: str) -> str:
    return "%s/us/%s" % (SITE, slug)


def default_cache_dir() -> Path:
    base = os.environ.get("LOCALAPPDATA") or str(Path.home() / "AppData" / "Local")
    return Path(base) / "ExilePact2" / "quest-tracker" / "poe2db-cache"


# --------------------------------------------------------------------------
# Refusing a git checkout

def git_tree_of(path: Path) -> Path | None:
    """The nearest directory at or above `path` holding a `.git` entry, if any.

    A worktree's `.git` is a file, so any entry counts. No git process is run.
    """
    p = Path(path).expanduser().resolve()
    for d in (p, *p.parents):
        if (d / ".git").exists():
            return d
    return None


def refuse_inside_git(path: Path, what: str) -> None:
    tree = git_tree_of(path)
    if tree is not None:
        raise ToolError(
            "refusing to write the %s at %s: it is inside the git checkout %s. Raw "
            "poe2db pages and drafts stay outside every repository; pass a folder "
            "elsewhere." % (what, path, tree)
        )


# --------------------------------------------------------------------------
# Network: one host, one request at a time, a fixed minimum gap

def urllib_fetcher(url: str, user_agent: str) -> tuple[int, bytes]:
    """The real fetcher: GET `url`, returning (HTTP status, body)."""
    request = urllib.request.Request(url, headers={"User-Agent": user_agent})
    try:
        with urllib.request.urlopen(request, timeout=60) as response:
            return response.status, response.read()
    except urllib.error.HTTPError as err:
        return err.code, err.read() or b""


class SystemClock:
    def monotonic(self) -> float:
        return time.monotonic()

    def sleep(self, seconds: float) -> None:
        time.sleep(seconds)

    def today(self) -> _dt.date:
        return _dt.date.today()


class Session:
    """Sends one request at a time, at least `delay` seconds apart, and only to poe2db.tw."""

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


# --------------------------------------------------------------------------
# robots.txt

def _robots_rules(text: str) -> list[tuple[bool, str]]:
    """The Allow/Disallow rules of the group that applies to this tool's User-Agent."""
    groups: list[tuple[list[str], list[tuple[bool, str]]]] = []
    agents: list[str] = []
    rules: list[tuple[bool, str]] = []
    in_rules = False
    for raw in text.splitlines():
        line = raw.split("#", 1)[0].strip()
        if ":" not in line:
            continue
        key, value = line.split(":", 1)
        key, value = key.strip().lower(), value.strip()
        if key == "user-agent":
            if in_rules:
                groups.append((agents, rules))
                agents, rules, in_rules = [], [], False
            agents.append(value.lower())
        elif key in ("allow", "disallow"):
            in_rules = True
            if agents and value:
                rules.append((key == "allow", value))
    if agents:
        groups.append((agents, rules))
    token = USER_AGENT.split("/")[0].lower()
    mine = [r for names, group in groups for r in group if any(n != "*" and n in token for n in names)]
    if mine:
        return mine
    return [r for names, group in groups for r in group if "*" in names]


def _pattern_matches(pattern: str, path: str) -> bool:
    anchored = pattern.endswith("$")
    body = pattern[:-1] if anchored else pattern
    regex = ".*".join(re.escape(part) for part in body.split("*"))
    return re.match(regex + ("$" if anchored else ""), path) is not None


class Robots:
    """robots.txt as `urllib.robotparser` reads it, and its wildcard patterns too.

    `urllib.robotparser` matches rules as plain prefixes, so poe2db's
    `Disallow: /*/marked` would never match there. A URL is fetched only when
    both readings allow it; among wildcard rules the longest match wins and an
    Allow wins a tie, as the robots.txt standard (RFC 9309) says.
    """

    def __init__(self, text: str = "", allow_all: bool = False, disallow_all: bool = False):
        self.parser = urllib.robotparser.RobotFileParser(ROBOTS_URL)
        if allow_all:
            self.parser.allow_all = True
        elif disallow_all:
            self.parser.disallow_all = True
        self.parser.parse(text.splitlines())
        self.rules = _robots_rules(text)

    def allows(self, url: str) -> bool:
        if not self.parser.can_fetch(USER_AGENT, url):
            return False
        parts = urllib.parse.urlsplit(url)
        path = (parts.path or "/") + ("?" + parts.query if parts.query else "")
        best: tuple[int, bool] | None = None
        for allow, pattern in self.rules:
            if _pattern_matches(pattern, path):
                key = (len(pattern), allow)
                if best is None or key > best:
                    best = key
        return best is None or best[1]

    def crawl_delay(self) -> float | None:
        value = self.parser.crawl_delay(USER_AGENT)
        return float(value) if value is not None else None


def load_robots(session: Session) -> Robots:
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

def load_cache_index(cache: Path) -> dict:
    path = Path(cache) / CACHE_INDEX
    if not path.is_file():
        return {"schema": 1, "source": page_url(INDEX_SLUG), "pages": {}}
    with open(path, encoding="utf-8") as fh:
        data = json.load(fh)
    data.setdefault("pages", {})
    return data


def save_cache_index(cache: Path, index: dict) -> None:
    path = Path(cache) / CACHE_INDEX
    tmp = path.with_suffix(".json.tmp")
    with open(tmp, "w", encoding="utf-8", newline="\n") as fh:
        json.dump(index, fh, indent=2, sort_keys=True, ensure_ascii=False)
        fh.write("\n")
    os.replace(tmp, path)


def cache_file_name(slug: str) -> str:
    return re.sub(r"[^A-Za-z0-9_.\-]", "_", slug) + ".html"


def _is_fresh(record: dict | None, cache: Path, today: _dt.date, max_age: int) -> bool:
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


def _read_cached(cache: Path, record: dict) -> str:
    with open(Path(cache) / record["file"], encoding="utf-8", errors="replace") as fh:
        return fh.read()


def is_campaign(entry: dict) -> bool:
    """Acts 1-6. Endgame is Act 10 and the act-less MasterMission entries; a
    MasterMission with a campaign act (an ascendancy trial) is campaign."""
    return entry["act"] is not None and entry["act"] not in ENDGAME_ACTS


def fetch(cache: Path, delay: float = DEFAULT_DELAY, only: list[str] | None = None,
          max_age: int = DEFAULT_MAX_AGE, include_endgame: bool = False,
          fetcher=None, clock=None, out=None) -> dict:
    """Fetch the quest index and the quest pages into `cache`. Returns the run's counts."""
    out = out or sys.stdout
    if not delay >= MIN_DELAY:
        raise ToolError("refusing --delay %s: the minimum is %s seconds between requests" % (delay, MIN_DELAY))
    if max_age < 0:
        raise ToolError("--max-age cannot be negative")
    cache = Path(cache)
    refuse_inside_git(cache, "poe2db cache")
    for slug in only or []:
        if not SLUG_RE.match(slug):
            raise ToolError("not a quest slug: %r (use the last part of the page URL)" % slug)
    clock = clock or SystemClock()
    session = Session(fetcher or urllib_fetcher, clock, float(delay))
    cache.mkdir(parents=True, exist_ok=True)
    index = load_cache_index(cache)
    today = clock.today()
    counts = {"fetched": 0, "cached": 0, "robots": 0, "failed": 0}

    robots = load_robots(session)
    crawl_delay = robots.crawl_delay()
    if crawl_delay is not None and crawl_delay > session.delay:
        session.delay = crawl_delay

    def get_page(slug: str) -> str | None:
        record = index["pages"].get(slug)
        if _is_fresh(record, cache, today, max_age):
            counts["cached"] += 1
            return _read_cached(cache, record)
        url = page_url(slug)
        if not robots.allows(url):
            counts["robots"] += 1
            print("skip  %s: disallowed by robots.txt" % slug, file=out)
            return None
        status, body = session.get(url)
        if status != 200:
            counts["failed"] += 1
            print("fail  %s: HTTP %s" % (slug, status), file=out)
            if not (record and record.get("status") == 200):
                index["pages"][slug] = {"slug": slug, "url": url, "fetched": today.isoformat(),
                                        "status": status, "file": None}
                save_cache_index(cache, index)
            return None
        name = cache_file_name(slug)
        (cache / name).write_bytes(body)
        index["pages"][slug] = {"slug": slug, "url": url, "fetched": today.isoformat(),
                                "status": status, "file": name}
        save_cache_index(cache, index)
        counts["fetched"] += 1
        print("fetch %s" % slug, file=out)
        return body.decode("utf-8", "replace")

    index_html = get_page(INDEX_SLUG)
    if index_html is None:
        raise ToolError("%s: the quest index page could not be fetched" % INDEX_SLUG)
    if only:
        slugs = [s for s in dict.fromkeys(only) if s != INDEX_SLUG]
    else:
        entries = parse_index(index_html)
        slugs = list(dict.fromkeys(e["slug"] for e in entries if include_endgame or is_campaign(e)))
    for slug in slugs:
        get_page(slug)
    print("pages: %d fetched, %d cached, %d disallowed by robots.txt, %d failed; cache %s"
          % (counts["fetched"], counts["cached"], counts["robots"], counts["failed"], cache), file=out)
    return counts


# --------------------------------------------------------------------------
# HTML: a small tolerant tree, enough for the pages' shape

VOID_TAGS = frozenset({"area", "base", "br", "col", "embed", "hr", "img", "input", "link",
                       "meta", "param", "source", "track", "wbr",
                       # poe2db splits dialogue with a bare `<continue>`, never closed.
                       "continue"})
SKIP_TEXT_TAGS = frozenset({"script", "style", "template"})


class Node:
    __slots__ = ("tag", "attrs", "children")

    def __init__(self, tag: str, attrs):
        self.tag = tag
        self.attrs = {k: (v or "") for k, v in attrs}
        self.children: list = []

    def has_class(self, name: str) -> bool:
        return name in self.attrs.get("class", "").split()

    def iter(self):
        yield self
        for child in self.children:
            if isinstance(child, Node):
                yield from child.iter()

    def find_all(self, tag: str | None = None, cls: str | None = None, id: str | None = None):
        for node in self.iter():
            if node is self:
                continue
            if tag is not None and node.tag != tag:
                continue
            if cls is not None and not node.has_class(cls):
                continue
            if id is not None and node.attrs.get("id") != id:
                continue
            yield node

    def find(self, tag: str | None = None, cls: str | None = None, id: str | None = None):
        return next(self.find_all(tag, cls, id), None)

    def elements(self):
        return [c for c in self.children if isinstance(c, Node)]

    def text(self) -> str:
        """Visible text, whitespace collapsed, a `<br>` kept as a newline."""
        pieces: list[str] = []

        def walk(node: Node) -> None:
            for child in node.children:
                if isinstance(child, str):
                    pieces.append(child)
                elif child.tag == "br":
                    pieces.append("\n")
                elif child.tag not in SKIP_TEXT_TAGS:
                    walk(child)

        walk(self)
        return _clean("".join(pieces))


def _clean(text: str) -> str:
    lines = [" ".join(line.split()) for line in text.split("\n")]
    return "\n".join(line for line in lines if line)


class _TreeBuilder(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.root = Node("#root", [])
        self.stack = [self.root]

    def handle_starttag(self, tag, attrs):
        node = Node(tag, attrs)
        self.stack[-1].children.append(node)
        if tag not in VOID_TAGS:
            self.stack.append(node)

    def handle_startendtag(self, tag, attrs):
        self.stack[-1].children.append(Node(tag, attrs))

    def handle_endtag(self, tag):
        for i in range(len(self.stack) - 1, 0, -1):
            if self.stack[i].tag == tag:
                del self.stack[i:]
                return

    def handle_data(self, data):
        self.stack[-1].children.append(data)


def parse_html(text: str) -> Node:
    builder = _TreeBuilder()
    builder.feed(text)
    builder.close()
    return builder.root


# --------------------------------------------------------------------------
# Extracting the index and the quest pages

def _reward_items(node: Node) -> list[str]:
    """`Reward: <a>Item</a>, 100 Gold` -> ['Item', '100 Gold']; a link is one item."""
    items: list[str] = []

    def walk(n: Node) -> None:
        for child in n.children:
            if isinstance(child, str):
                for i, piece in enumerate(child.replace("Reward:", "").split(",")):
                    piece = " ".join(piece.split())
                    if not piece:
                        continue
                    if i == 0 and piece.startswith("(") and items:
                        items[-1] += " " + piece  # `Uncut Support Gem(iLv4)`
                    else:
                        items.append(piece)
            elif child.tag == "a":
                text = " ".join(child.text().split())
                if text:
                    items.append(text)
            elif child.tag not in SKIP_TEXT_TAGS:
                walk(child)

    walk(node)
    return items


def _properties(props: list[Node]) -> dict:
    """The `<div class="property">` lines: type, act(s) and rewards."""
    quest_type = None
    acts: list[int] = []
    rewards: list[str] = []
    for prop in props:
        text = prop.text()
        act = re.fullmatch(r"Act (\d+)", text)
        if act:
            acts.append(int(act.group(1)))
        elif text.startswith("Reward:"):
            rewards = _reward_items(prop)
        elif quest_type is None and re.fullmatch(r"[A-Za-z]+", text):
            quest_type = text
    return {"type": quest_type, "acts": acts, "rewards": rewards}


def _slug_of(href: str) -> str | None:
    href = href.strip()
    if href.startswith("/us/"):
        href = href[len("/us/"):]
    return href if SLUG_RE.match(href) else None


def parse_index(text: str) -> list[dict]:
    """The `#Quest` tab of https://poe2db.tw/us/Quest, one dict per entry, in page order.

    A quest that repeats per act appears once per act, under the same slug.
    """
    root = parse_html(text)
    tab = root.find(id="Quest")
    entries: list[dict] = []
    if tab is not None:
        for box in tab.find_all("div", cls="flex-grow-1"):
            link = next((a for a in box.find_all("a", cls="Quest") if a.text()), None)
            if link is None:
                continue
            slug = _slug_of(link.attrs.get("href", ""))
            if slug is None:
                print("warning: index entry %r has an unexpected link %r; skipped"
                      % (link.text(), link.attrs.get("href", "")), file=sys.stderr)
                continue
            info = _properties([p for p in box.elements() if p.has_class("property")])
            description = box.find("div", cls="implicitMod")
            entries.append({
                "slug": slug,
                "name": link.text(),
                "type": info["type"],
                "act": info["acts"][0] if info["acts"] else None,
                "rewards": info["rewards"],
                "description": description.text() if description is not None else None,
            })
    if not entries:
        raise ExtractError(
            "%s: the index page yields no quest (no `#Quest` tab with quest entries); "
            "the site's structure may have changed" % INDEX_SLUG)
    return entries


def _steps_table(root: Node) -> Node | None:
    for table in root.find_all("table"):
        header = [th.text() for th in table.find_all("th")]
        if header[:2] == ["#", "Name"]:
            return table
    return None


def _npc_entry(item: Node) -> dict | None:
    link = item.find("a", cls="NPCs")
    if link is None:
        return None
    speech = item.find("span", cls="d-block")
    topic: list[str] = []
    seen_link = False
    for child in item.children:
        if child is link:
            seen_link = True
        elif child is speech:
            break
        elif seen_link:
            topic.append(child if isinstance(child, str) else child.text())
    lines: list[str] = []
    if speech is not None:
        current: list[str] = []
        for child in speech.children:
            if isinstance(child, Node) and child.tag == "continue":
                lines.append(" ".join("".join(current).split()))
                current = []
            elif isinstance(child, str):
                current.append(child)
            elif child.tag == "br":
                current.append(" ")
            else:
                current.append(child.text())
        lines.append(" ".join("".join(current).split()))
    return {
        "npc": link.text(),
        "topic": " ".join(" ".join(topic).split()).rstrip(":").strip() or None,
        "lines": [line for line in lines if line],
    }


def _parse_step(number: int, cell: Node, with_dialogue: bool) -> dict:
    segments: list[list[str]] = [[]]
    areas: list[str] = []
    dialogue: list[dict] = []

    def walk(node: Node) -> None:
        for child in node.children:
            if isinstance(child, str):
                segments[-1].append(child)
            elif child.tag == "br":
                segments.append([])
            elif child.has_class("MapPins"):
                areas.append(child.text())
            elif child.has_class("NPCs") or child.find("a", cls="NPCs") is not None:
                holders = [li for li in child.find_all("li") if li.find("a", cls="NPCs") is not None]
                for holder in holders or [child]:
                    entry = _npc_entry(holder)
                    if entry is not None:
                        dialogue.append(entry)
            elif child.tag not in SKIP_TEXT_TAGS:
                walk(child)

    walk(cell)
    texts = [" ".join("".join(seg).split()) for seg in segments]
    title = texts[0] if texts else ""
    body = [t for t in texts[1:] if t and not t.startswith("Area:")]
    step = {
        "number": number,
        "title": title,
        "text": "\n".join(body),
        "area": areas[0] if areas else None,
        "areas": areas,
        "npcs": list(dict.fromkeys(d["npc"] for d in dialogue)),
    }
    if with_dialogue:
        step["dialogue"] = dialogue
    return step


def parse_quest_page(slug: str, text: str, with_dialogue: bool = False) -> dict:
    """One quest page, https://poe2db.tw/us/<slug>. Raises ExtractError naming the slug."""
    root = parse_html(text)
    table = _steps_table(root)
    if table is None:
        raise ExtractError(
            "%s: the page has no steps table (a table headed '#' and 'Name'); "
            "the site's structure may have changed" % slug)
    name_node = root.find("div", cls="itemName")
    name = name_node.text() if name_node is not None else ""
    if not name:
        raise ExtractError("%s: the page has no quest name (`div.itemName`)" % slug)
    stats = root.find("div", cls="Stats")
    info = _properties([p for p in stats.elements() if p.has_class("property")] if stats else [])
    description = stats.find("div", cls="implicitMod") if stats is not None else None
    steps = []
    for row in table.find_all("tr"):
        cells = [c for c in row.elements() if c.tag == "td"]
        if len(cells) < 2:
            continue
        number = cells[0].text()
        if not number.isdigit():
            raise ExtractError("%s: a steps-table row starts with %r, not a step number" % (slug, number))
        steps.append(_parse_step(int(number), cells[1], with_dialogue))
    if not steps:
        raise ExtractError("%s: the steps table has no step rows" % slug)
    return {
        "name": name,
        "type": info["type"],
        "acts": info["acts"],
        "rewards": info["rewards"],
        "description": description.text() if description is not None else None,
        "steps": steps,
    }


def _cached_quest_pages(cache: Path, index: dict) -> list[tuple[str, dict]]:
    return [(slug, rec) for slug, rec in index["pages"].items()
            if slug != INDEX_SLUG and rec.get("status") == 200 and rec.get("file")]


def _require_cache_index(cache: Path) -> dict:
    if not (Path(cache) / CACHE_INDEX).is_file():
        raise ToolError("no cache at %s (no %s); run `fetch` first" % (cache, CACHE_INDEX))
    return load_cache_index(cache)


def extract(cache: Path, out: Path | None = None, with_dialogue: bool = False,
            clock=None, stdout=None) -> dict:
    """Draft one JSON entry per cached quest page and write it to `out`."""
    stdout = stdout or sys.stdout
    cache = Path(cache)
    out = Path(out) if out is not None else cache / DRAFT_NAME
    refuse_inside_git(out, "draft")
    index = _require_cache_index(cache)
    index_record = index["pages"].get(INDEX_SLUG)
    if not index_record or index_record.get("status") != 200 or not index_record.get("file"):
        raise ExtractError("%s: the quest index page is not in the cache; run `fetch` first" % INDEX_SLUG)
    entries = parse_index(_read_cached(cache, index_record))
    by_slug: dict[str, list[dict]] = {}
    for entry in entries:
        by_slug.setdefault(entry["slug"], []).append(entry)
    cached = dict(_cached_quest_pages(cache, index))
    order = [s for s in by_slug if s in cached] + [s for s in cached if s not in by_slug]
    quests = []
    for slug in order:
        record = cached[slug]
        page = parse_quest_page(slug, _read_cached(cache, record), with_dialogue)
        listed = by_slug.get(slug, [])
        acts = sorted(set(page["acts"]) | {e["act"] for e in listed if e["act"] is not None})
        first = listed[0] if listed else {}
        quests.append({
            "slug": slug,
            "url": record["url"],
            "fetched": record["fetched"],
            "name": page["name"],
            "type": page["type"] or first.get("type"),
            "act": page["acts"][0] if page["acts"] else (acts[0] if acts else None),
            "acts": acts,
            "rewards": page["rewards"] or first.get("rewards", []),
            "description": page["description"] or first.get("description"),
            "steps": page["steps"],
        })
    if not quests:
        raise ExtractError("%s: the index yields no quest with a cached page; run `fetch` first" % INDEX_SLUG)
    draft = {
        "schema": 1,
        "source": page_url(INDEX_SLUG),
        "index_fetched": index_record["fetched"],
        "extracted": (clock or SystemClock()).today().isoformat(),
        "with_dialogue": with_dialogue,
        "quests": quests,
    }
    out.parent.mkdir(parents=True, exist_ok=True)
    with open(out, "w", encoding="utf-8", newline="\n") as fh:
        json.dump(draft, fh, indent=2, ensure_ascii=False)
        fh.write("\n")
    print("extracted %d quests to %s" % (len(quests), out), file=stdout)
    return draft


# --------------------------------------------------------------------------
# Coverage: does a page's dialogue reach the log?

def _norm(text: str) -> str:
    return " ".join(text.lower().split())


def _four_word_runs(text: str) -> set[tuple[str, ...]]:
    words = re.findall(r"[a-z0-9']+", text.lower())
    return set(zip(words, words[1:], words[2:], words[3:]))


def npc_lines_from_log(log: Path, speakers: set[str]) -> dict[str, set[str]]:
    """The distinct lines each of `speakers` said in a Client.txt, read-only.

    Only `<Speaker>: <text>` messages count; chat (a channel prefix) never does.
    """
    said: dict[str, set[str]] = {s: set() for s in speakers}
    with open(log, "r", encoding="utf-8", errors="replace") as fh:
        for raw in fh:
            match = LOG_LINE_RE.match(raw.rstrip("\r\n"))
            if not match:
                continue
            message = match.group(1)
            if not message or message[0] in CHAT_PREFIXES or message[0] == ":":
                continue
            speaker, sep, line = message.partition(": ")
            if sep and speaker in said and line.strip():
                said[speaker].add(line.strip())
    return said


def coverage(cache: Path, log: Path, stdout=None) -> dict:
    """Per cached page: dialogue snippets, how many equal or contain a logged line
    of the same NPC, how many share a 4-word run with one. Counts only."""
    stdout = stdout or sys.stdout
    cache = Path(cache)
    index = _require_cache_index(cache)
    pages = sorted(_cached_quest_pages(cache, index))
    if not pages:
        raise ExtractError("%s: no quest page in the cache; run `fetch` first" % INDEX_SLUG)
    parsed = [(slug, parse_quest_page(slug, _read_cached(cache, rec), with_dialogue=True))
              for slug, rec in pages]
    speakers = {d["npc"] for _, page in parsed for step in page["steps"] for d in step["dialogue"]}
    said = npc_lines_from_log(Path(log), speakers)
    logged = {s: {_norm(line) for line in lines} for s, lines in said.items()}
    runs = {s: set().union(*(_four_word_runs(line) for line in lines)) if lines else set()
            for s, lines in said.items()}
    totals = {"pages": 0, "snippets": 0, "equal_or_contain": 0, "shared_4word_run": 0}
    rows = []
    for slug, page in parsed:
        snippets = equal = shared = 0
        for step in page["steps"]:
            for block in step["dialogue"]:
                npc = block["npc"]
                for line in block["lines"]:
                    snippets += 1
                    snippet = _norm(line)
                    if any(l == snippet or l in snippet for l in logged.get(npc, ()) if l):
                        equal += 1
                    if _four_word_runs(line) & runs.get(npc, set()):
                        shared += 1
        rows.append({"slug": slug, "snippets": snippets, "equal_or_contain": equal,
                     "shared_4word_run": shared})
        totals["pages"] += 1
        totals["snippets"] += snippets
        totals["equal_or_contain"] += equal
        totals["shared_4word_run"] += shared
        print("%s\tsnippets=%d\tequal_or_contain=%d\tshared_4word_run=%d"
              % (slug, snippets, equal, shared), file=stdout)
    totals["npcs_with_logged_lines"] = sum(1 for lines in said.values() if lines)
    print("TOTAL\tpages=%d\tsnippets=%d\tequal_or_contain=%d\tshared_4word_run=%d\tnpcs_with_logged_lines=%d"
          % (totals["pages"], totals["snippets"], totals["equal_or_contain"],
             totals["shared_4word_run"], totals["npcs_with_logged_lines"]), file=stdout)
    return {"pages": rows, "totals": totals}


# --------------------------------------------------------------------------
# Command line

def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="py -3 -m tools.quest_tracker.poe2db",
        description="Developer-only poe2db.tw quest tooling: fetch, extract, coverage.")
    sub = parser.add_subparsers(dest="command", required=True, metavar="{fetch,extract,coverage}")

    f = sub.add_parser("fetch", help="fetch the quest index and campaign quest pages into the cache, politely")
    f.add_argument("--cache", type=Path, default=None,
                   help="cache folder, outside any git checkout (default %%LOCALAPPDATA%%\\ExilePact2\\quest-tracker\\poe2db-cache)")
    f.add_argument("--delay", type=float, default=DEFAULT_DELAY,
                   help="seconds between requests (default %s, minimum %s)" % (DEFAULT_DELAY, MIN_DELAY))
    f.add_argument("--only", nargs="+", metavar="SLUG", default=None,
                   help="fetch only these quest pages (and the index)")
    f.add_argument("--max-age", type=int, default=DEFAULT_MAX_AGE, metavar="DAYS",
                   help="leave pages fetched within this many days alone (default %d)" % DEFAULT_MAX_AGE)
    f.add_argument("--include-endgame", action="store_true",
                   help="also fetch Act 10 and act-less MasterMission quests")

    e = sub.add_parser("extract", help="draft one JSON entry per cached quest page")
    e.add_argument("--cache", type=Path, default=None, help="cache folder (default as for fetch)")
    e.add_argument("--out", type=Path, default=None,
                   help="draft file, outside any git checkout (default <cache>\\%s)" % DRAFT_NAME)
    e.add_argument("--with-dialogue", action="store_true", help="include each step's NPC dialogue")

    c = sub.add_parser("coverage", help="count how far cached dialogue matches a Client.txt (counts only)")
    c.add_argument("--log", type=Path, required=True, help="a Client.txt, read-only")
    c.add_argument("--cache", type=Path, default=None, help="cache folder (default as for fetch)")
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
            fetch(cache, delay=args.delay, only=args.only, max_age=args.max_age,
                  include_endgame=args.include_endgame, fetcher=fetcher, clock=clock)
        elif args.command == "extract":
            extract(cache, out=args.out, with_dialogue=args.with_dialogue, clock=clock)
        else:
            coverage(cache, args.log)
    except ToolError as err:
        print("poe2db: error: %s" % err, file=sys.stderr)
        return 1
    except (OSError, json.JSONDecodeError) as err:
        print("poe2db: error: %s" % err, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
