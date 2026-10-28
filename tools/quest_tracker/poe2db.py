"""poe2db.tw quest pages: fetch them politely, draft quest data, measure log coverage.

Developer-only. This is the one piece of the quest tracker that touches the
network, and it only ever asks poe2db.tw:

    py -3 -m tools.quest_tracker.poe2db fetch [--cache DIR] [--delay S] [--only SLUG ...]
                                             [--max-age DAYS] [--include-endgame]
    py -3 -m tools.quest_tracker.poe2db fetch-classes CLASS ... [--cache DIR] [--delay S] [--max-age DAYS]
    py -3 -m tools.quest_tracker.poe2db extract [--cache DIR] [--out FILE] [--with-dialogue]
    py -3 -m tools.quest_tracker.poe2db remarks [--cache DIR] [--data FILE] [--class CLASS ...]
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
with a non-zero exit, naming the slug, when the index yields no quest, a
steps table has no or malformed step rows, or no page in the cache has a
steps table: a changed site fails loudly rather than drafting nothing. A
single page without a steps table (some real campaign pages have none) is
drafted with zero steps and its slug named on stderr; `coverage` counts it
the same way.

`fetch-classes` fetches class pages (e.g. `Warrior`, `Mercenary`) by the same
rules, recording them under the cache index's `classes` so they are never
read as quests. `remarks` prints a draft of the kill rows of their "Text
Audio" tables whose boss the quest data names (`bar_gone` bosses and `npcs`):
class, boss, key, text, the page's URL and fetch date, never a whole table,
for a person to curate into the data's `remarks` table. A class page with no
such table stops it with a non-zero exit naming the class.

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
DEFAULT_DATA = Path(__file__).resolve().parents[2] / "apps" / "quest-tracker" / "data" / "quests.json"
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


def fetch_classes(cache: Path, classes: list[str], delay: float = DEFAULT_DELAY,
                  max_age: int = DEFAULT_MAX_AGE, fetcher=None, clock=None, out=None) -> dict:
    """Fetch class pages (e.g. `Warrior`) into `cache`, by the same rules as `fetch`.

    They are recorded under the index's `classes`, never its `pages`, so
    `extract` and `coverage` never read a class page as a quest. No quest
    index is fetched. Returns the run's counts.
    """
    out = out or sys.stdout
    if not delay >= MIN_DELAY:
        raise ToolError("refusing --delay %s: the minimum is %s seconds between requests" % (delay, MIN_DELAY))
    if max_age < 0:
        raise ToolError("--max-age cannot be negative")
    if not classes:
        raise ToolError("name at least one class page, e.g. Warrior")
    for name in classes:
        if not SLUG_RE.match(name):
            raise ToolError("not a class page name: %r (use the last part of the page URL)" % name)
    cache = Path(cache)
    refuse_inside_git(cache, "poe2db cache")
    clock = clock or SystemClock()
    session = Session(fetcher or urllib_fetcher, clock, float(delay))
    cache.mkdir(parents=True, exist_ok=True)
    index = load_cache_index(cache)
    index.setdefault("classes", {})
    today = clock.today()
    counts = {"fetched": 0, "cached": 0, "robots": 0, "failed": 0}

    names = list(dict.fromkeys(classes))
    stale = [n for n in names if not _is_fresh(index["classes"].get(n), cache, today, max_age)]
    counts["cached"] = len(names) - len(stale)
    if stale:
        robots = load_robots(session)
        crawl_delay = robots.crawl_delay()
        if crawl_delay is not None and crawl_delay > session.delay:
            session.delay = crawl_delay
    for name in stale:
        url = page_url(name)
        if not robots.allows(url):
            counts["robots"] += 1
            print("skip  %s: disallowed by robots.txt" % name, file=out)
            continue
        status, body = session.get(url)
        if status != 200:
            counts["failed"] += 1
            print("fail  %s: HTTP %s" % (name, status), file=out)
            continue
        file_name = "class-" + cache_file_name(name)
        (cache / file_name).write_bytes(body)
        index["classes"][name] = {"slug": name, "url": url, "fetched": today.isoformat(),
                                  "status": status, "file": file_name}
        save_cache_index(cache, index)
        counts["fetched"] += 1
        print("fetch %s" % name, file=out)
    print("class pages: %d fetched, %d cached, %d disallowed by robots.txt, %d failed; cache %s"
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
    """One quest page, https://poe2db.tw/us/<slug>. Raises ExtractError naming the slug.

    A page with no steps table (a table headed '#' and 'Name') is a quest with
    zero steps and `steps_table` false: some real campaign pages carry none.
    The callers fail loudly when no page in the cache has one.
    """
    root = parse_html(text)
    table = _steps_table(root)
    name_node = root.find("div", cls="itemName")
    name = name_node.text() if name_node is not None else ""
    if not name:
        raise ExtractError("%s: the page has no quest name (`div.itemName`)" % slug)
    stats = root.find("div", cls="Stats")
    info = _properties([p for p in stats.elements() if p.has_class("property")] if stats else [])
    description = stats.find("div", cls="implicitMod") if stats is not None else None
    steps = []
    for row in table.find_all("tr") if table is not None else ():
        cells = [c for c in row.elements() if c.tag == "td"]
        if len(cells) < 2:
            continue
        number = cells[0].text()
        if not number.isdigit():
            raise ExtractError("%s: a steps-table row starts with %r, not a step number" % (slug, number))
        steps.append(_parse_step(int(number), cells[1], with_dialogue))
    if table is not None and not steps:
        raise ExtractError("%s: the steps table has no step rows" % slug)
    return {
        "name": name,
        "type": info["type"],
        "acts": info["acts"],
        "rewards": info["rewards"],
        "description": description.text() if description is not None else None,
        "steps": steps,
        "steps_table": table is not None,
    }


def _check_steps_tables(pages: list[tuple[str, dict]], stderr) -> list[str]:
    """Name each page without a steps table on stderr; fail when none has one."""
    missing = [slug for slug, page in pages if not page["steps_table"]]
    if pages and len(missing) == len(pages):
        raise ExtractError(
            "%s: no page in the cache has a steps table (a table headed '#' and 'Name'); "
            "the site's structure may have changed" % INDEX_SLUG)
    for slug in missing:
        print("%s: no steps table; recorded with zero steps" % slug, file=stderr)
    return missing


def _cached_quest_pages(cache: Path, index: dict) -> list[tuple[str, dict]]:
    return [(slug, rec) for slug, rec in index["pages"].items()
            if slug != INDEX_SLUG and rec.get("status") == 200 and rec.get("file")]


def _require_cache_index(cache: Path) -> dict:
    if not (Path(cache) / CACHE_INDEX).is_file():
        raise ToolError("no cache at %s (no %s); run `fetch` first" % (cache, CACHE_INDEX))
    return load_cache_index(cache)


def extract(cache: Path, out: Path | None = None, with_dialogue: bool = False,
            clock=None, stdout=None, stderr=None) -> dict:
    """Draft one JSON entry per cached quest page and write it to `out`."""
    stdout = stdout or sys.stdout
    stderr = stderr or sys.stderr
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
    pages = [(slug, parse_quest_page(slug, _read_cached(cache, cached[slug]), with_dialogue))
             for slug in order]
    no_steps_table = _check_steps_tables(pages, stderr)
    quests = []
    for slug, page in pages:
        record = cached[slug]
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
        "no_steps_table": no_steps_table,
        "quests": quests,
    }
    out.parent.mkdir(parents=True, exist_ok=True)
    with open(out, "w", encoding="utf-8", newline="\n") as fh:
        json.dump(draft, fh, indent=2, ensure_ascii=False)
        fh.write("\n")
    print("extracted %d quests to %s" % (len(quests), out), file=stdout)
    return draft


# --------------------------------------------------------------------------
# Remarks: a class's kill lines for the bosses the quest data names

# A key that ends on a kill: `MillerKillComment`, `BlackjawKilled`,
# `RavineBlackjawBossDefeated`, `RedValeBossSlainComment`. Not a sighting, and
# not `IncursionDefeatBlackjawUnique`, a league mechanic's line.
KILL_KEY_RE = re.compile(r"(?:Kill|Killed|Defeat|Defeated|Slain)(?:Comment)?$")


def _key_words(key: str) -> list[str]:
    """`RavineBlackjawBossDefeated` -> ['ravine', 'blackjaw', 'boss', 'defeated']."""
    return [w.lower() for w in re.findall(r"[A-Z][a-z]+|[A-Z]+(?![a-z])|[a-z]+|\d+", key)]


def _boss_word(name: str) -> str:
    """The word of a boss's name a page key carries: the last word of the name
    before its epithet. `The Bloated Miller` -> miller, `Ignagduk, the Bog
    Witch` -> ignagduk, `Beira of the Rotten Pack` -> beira, `BlackJaw` -> blackjaw."""
    head = re.split(r",| of ", name, maxsplit=1)[0]
    words = re.findall(r"[A-Za-z']+", head)
    return words[-1].lower() if words else ""


def data_bosses(data: dict) -> list[str]:
    """The bosses a remark may be for, as the app's validator allows them: each
    `bar_gone` signal's boss, then `npcs`, in file order, each once."""
    names = [s["boss"] for q in data.get("quests", []) for step in q.get("steps", [])
             for s in step.get("signals", [])
             if s.get("kind") == "screen" and s.get("what") == "bar_gone" and s.get("boss")]
    return list(dict.fromkeys(names + list(data.get("npcs", []))))


def _text_before_break(cell: Node) -> str:
    """A cell's text up to its first `<br>` (or the key's `<strong>`), in document
    order: an element left open across the break (an `<i>` the site never
    closes) must not carry the key into the text."""
    pieces: list[str] = []

    def walk(node: Node) -> bool:
        for child in node.children:
            if isinstance(child, str):
                pieces.append(child)
            elif child.tag in ("br", "strong"):
                return True
            elif child.tag not in SKIP_TEXT_TAGS and walk(child):
                return True
        return False

    walk(cell)
    return " ".join("".join(pieces).split())


def parse_text_audio(name: str, text: str) -> list[dict]:
    """The rows of a class page's "Text Audio" table: {key, text, audio}.

    A row is `<td>text<br/> marker <strong>Key</strong> <a href=...ogg></a></td>`;
    `text` is what comes before the first `<br>`, and `audio` the file name the
    link ends in (its name says the act, e.g. `_a1_`, `_i3_`).
    """
    root = parse_html(text)
    pane = root.find(id=name + "TextAudio")
    if pane is None:
        pane = next((n for n in root.find_all("div") if n.attrs.get("id", "").endswith("TextAudio")), None)
    table = pane.find("table") if pane is not None else None
    body = table.find("tbody") if table is not None else None
    if body is None:
        raise ExtractError("%s: no Text Audio table (no `#%sTextAudio` tab with a table body)" % (name, name))
    rows = []
    for tr in body.find_all("tr"):
        td, key = tr.find("td"), tr.find("strong")
        if td is None or key is None:
            raise ExtractError("%s: a Text Audio row without a cell or a key" % name)
        link = tr.find("a")
        rows.append({"key": key.text(), "text": _text_before_break(td),
                     "audio": link.attrs.get("href", "").rsplit("/", 1)[-1] if link is not None else ""})
    if not rows:
        raise ExtractError("%s: the Text Audio table has no rows" % name)
    return rows


def remarks(cache: Path, data_path: Path, classes: list[str] | None = None,
            clock=None, stdout=None) -> dict:
    """Print a draft of the cached class pages' kill rows for bosses `data_path` names.

    One row per line whose key ends on a kill and carries a boss's name word
    (`_boss_word`), never the whole table. The draft is for a person to curate
    into the quest data's `remarks` table; `audio` helps tell which act's
    fight a line belongs to and is not a field of that table.
    """
    stdout = stdout or sys.stdout
    cache = Path(cache)
    index = _require_cache_index(cache)
    cached = {n: r for n, r in index.get("classes", {}).items() if r.get("status") == 200 and r.get("file")}
    if not cached:
        raise ToolError("no class page in the cache at %s; run `fetch-classes` first" % cache)
    names = list(dict.fromkeys(classes)) if classes else list(cached)
    for name in names:
        if name not in cached:
            raise ToolError("%s: not in the cache; run `fetch-classes %s` first" % (name, name))
    with open(data_path, encoding="utf-8") as fh:
        bosses = [(b, _boss_word(b)) for b in data_bosses(json.load(fh))]
    rows = []
    for name in names:
        record = cached[name]
        for row in parse_text_audio(name, _read_cached(cache, record)):
            if not row["text"] or not KILL_KEY_RE.search(row["key"]):
                continue
            words = _key_words(row["key"])
            boss = next((b for b, w in bosses if w and w in words), None)
            if boss is None:
                continue
            entry = {"class": name, "boss": boss, "key": row["key"], "text": row["text"],
                     "source_url": record["url"], "fetched": record["fetched"], "audio": row["audio"]}
            if entry not in rows:
                rows.append(entry)
    draft = {"schema": 1, "extracted": (clock or SystemClock()).today().isoformat(), "remarks": rows}
    json.dump(draft, stdout, indent=2, ensure_ascii=False)
    stdout.write("\n")
    return draft


# --------------------------------------------------------------------------
# The remark ledger: every campaign remark key of the class pages, accounted for
#
# `apps/quest-tracker/data/remark-ledger/<part>.json` names each key whose
# sound belongs to a campaign act, with the quest steps it is attached to or
# the reason it is not. The ledger holds keys, ids and our own words; the
# classes' lines reach the data only as the draft rows `--apply` writes.

# The playable classes, in the order the draft writes their rows.
CLASSES = ("Warrior", "Mercenary", "Ranger", "Huntress", "Witch", "Sorceress", "Monk", "Druid")
# A part and the sound tags (`warrior_gameplay_a1_06.ogg` -> a1) that put a key in it.
PARTS = (("act1", ("a1",)), ("act2", ("a2",)), ("act3", ("a3",)), ("act4", ("a4",)),
         ("interludes", ("i1", "i2", "i3", "p1", "p2", "p3")))
PART_NAMES = tuple(name for name, _ in PARTS)
# The quest act a part's steps belong to; the interludes are the data's act 6.
PART_ACTS = {"act1": 1, "act2": 2, "act3": 3, "act4": 4, "interludes": 6}
SKIP_CODES = ("log-covers", "no-step", "no-area", "not-established", "too-short")
SOUND_TAG_RE = re.compile(r"(?<=_)([aip]\d+)(?=_)")
MIN_LETTERS = 12
DNT = "[DNT]"
MAX_WHY = 200
MAX_CANDIDATES = 5


def sound_tags(audio: str) -> list[str]:
    """`ranger_gameplay_a1_11.ogg` -> ['a1']; a wild or endgame sound has none."""
    return SOUND_TAG_RE.findall(audio)


def part_of_tags(tags) -> str | None:
    """The first part, in PARTS order, that one of `tags` falls in."""
    for name, part_tags in PARTS:
        if any(tag in part_tags for tag in tags):
            return name
    return None


def clean_text(text: str) -> str:
    """The text rules' first half: `{` and `}` removed, whitespace collapsed."""
    return " ".join(text.replace("{", "").replace("}", "").split())


def _letters(text: str) -> str:
    return "".join(ch for ch in text.lower() if ch.isalpha())


def _too_short(text: str) -> bool:
    return DNT in text.upper() or len(_letters(text)) < MIN_LETTERS


class RemarkPages:
    """The eight class pages' Text Audio rows, by key.

    `keys` holds the campaign keys in page order (classes in CLASSES order,
    rows in page order) with their sound tags and part; `texts[key][class]`
    every distinct cleaned text that class's page gives the key, whatever the
    row's own sound; `by_letters` every key, campaign or not, a text's letters
    belong to, for the rule that drops a text two keys share; `voiced` every
    key with a sound file on at least one page, the only keys that rule counts
    (a key whose rows all lack one, an older voice set, is never heard).
    """

    def __init__(self, pages: dict):
        self.pages = pages
        self.keys: dict[str, dict] = {}
        self.texts: dict[str, dict[str, list[str]]] = {}
        self.by_letters: dict[str, set[str]] = {}
        self.voiced: set[str] = set()
        tags_of: dict[str, set[str]] = {}
        for name in CLASSES:
            for row in pages[name]["rows"]:
                key, text = row["key"], row["text"]
                if row["voiced"]:
                    self.voiced.add(key)
                tags_of.setdefault(key, set()).update(row["tags"])
                texts = self.texts.setdefault(key, {}).setdefault(name, [])
                if text not in texts:
                    texts.append(text)
                if _letters(text):
                    self.by_letters.setdefault(_letters(text), set()).add(key)
        for key, tags in tags_of.items():
            part = part_of_tags(tags)
            if part is not None:
                self.keys[key] = {"tags": sorted(tags), "part": part}

    def part_keys(self, part: str) -> list[str]:
        return [key for key, info in self.keys.items() if info["part"] == part]

    def class_texts(self, key: str, name: str) -> list[str]:
        return self.texts.get(key, {}).get(name, [])

    def all_texts(self, key: str) -> list[str]:
        return [t for name in CLASSES for t in self.class_texts(key, name)]

    def shared_with(self, key: str, text: str) -> set[str]:
        """The other voiced keys any page gives this text (letters only, lower case)."""
        return (self.by_letters.get(_letters(text), set()) & self.voiced) - {key}


def load_class_pages(cache: Path) -> RemarkPages:
    """The eight playable classes' pages from the cache; every one must be there."""
    cache = Path(cache)
    index = _require_cache_index(cache)
    cached = {n: r for n, r in index.get("classes", {}).items() if r.get("status") == 200 and r.get("file")}
    missing = [name for name in CLASSES if name not in cached]
    if missing:
        raise ToolError("the remark ledger reads all eight class pages; not in the cache: %s; "
                        "run `fetch-classes %s` first" % (", ".join(missing), " ".join(missing)))
    pages = {}
    for name in CLASSES:
        rows = [{"key": row["key"], "text": clean_text(row["text"]), "tags": sound_tags(row["audio"]),
                 "voiced": bool(row["audio"])}
                for row in parse_text_audio(name, _read_cached(cache, cached[name]))]
        pages[name] = {"record": cached[name], "rows": rows}
    return RemarkPages(pages)


def _load_data(data_path: Path) -> dict:
    with open(data_path, encoding="utf-8") as fh:
        return json.load(fh)


def _words4(text: str) -> set[str]:
    return {w for w in re.findall(r"[a-z]+", text.lower()) if len(w) >= 4}


def remark_keys(cache: Path, data_path: Path, part: str, stdout=None) -> list[dict]:
    """Print one JSON line per campaign key of `part`, in page order.

    Each line: the key, its sound tags, each class's texts after the text rules
    a ledger is not needed for (braces, whitespace, under 12 letters, [DNT]),
    the other voiced keys any of those texts is also a text of (the rule that needs a
    ledger), and up to five candidate `quest/step` ids: steps of the part's act
    whose title or text shares a word of four or more letters with the key's.
    """
    stdout = stdout or sys.stdout
    pages = load_class_pages(cache)
    data = _load_data(data_path)
    steps = [(q["id"], s["id"], _words4(s.get("title", "") + " " + s.get("text", "")))
             for q in data.get("quests", []) if q.get("act") == PART_ACTS[part] for s in q.get("steps", [])]
    lines = []
    for key in pages.part_keys(part):
        words = {w for w in _key_words(key) if len(w) >= 4}
        ranked = sorted(((-len(words & step_words), n, "%s/%s" % (quest, step))
                         for n, (quest, step, step_words) in enumerate(steps) if words & step_words))
        texts = {name: [t for t in pages.class_texts(key, name) if not _too_short(t)] for name in CLASSES}
        shared = sorted({other for name in CLASSES for t in texts[name] for other in pages.shared_with(key, t)})
        line = {"key": key, "tags": pages.keys[key]["tags"], "texts": texts, "shared": shared,
                "candidates": [c for _, _, c in ranked[:MAX_CANDIDATES]]}
        lines.append(line)
        stdout.write(json.dumps(line) + "\n")
    return lines


def _signal_area(signal: dict):
    return signal.get("code") if signal.get("kind") == "area" else signal.get("area")


def _act_fits(tags: list[str], quest: dict) -> bool:
    """Rule 2: `a<n>` is act n; `i<n>`/`p<n>` an act-6 quest in `P<n>_*` zones."""
    for tag in tags:
        kind, n = tag[0], int(tag[1:])
        if kind == "a" and quest.get("act") == n:
            return True
        if kind in "ip" and quest.get("act") == 6 and \
                any(str(z).startswith("P%d_" % n) for z in quest.get("zones") or []):
            return True
    return False


def _area_fits(area: str, quest: dict, step: dict, key: str, boss, texts: list[str], areas: dict) -> bool:
    """Rule 3: the area of a signal the step already has (not this key's own),
    else an area of the data, inside the quest's zones when it has any, whose
    name the step's title or text, the key or a class's line names."""
    for signal in step.get("signals", []):
        if signal.get("what") == "remark" and signal.get("key") == key:
            continue
        if signal.get("what") == "kill_remark" and boss is not None and signal.get("boss") == boss:
            continue
        if _signal_area(signal) == area:
            return True
    if area not in areas:
        return False
    zones = quest.get("zones") or []
    if zones and area not in zones:
        return False
    name = re.sub(r"^the\s+", "", str(areas[area].get("name", "")).strip().lower())
    if not name:
        return False
    said = " ".join([step.get("title", ""), step.get("text", "")] + texts).lower()
    return bool(re.search(r"\b%s\b" % re.escape(name), said)) or _letters(name) in key.lower()


class LedgerReport:
    def __init__(self, parts: tuple):
        self.parts = parts
        self.findings: list[str] = []
        self.gaps: list[str] = []
        self.entries: dict[str, list[dict]] = {part: [] for part in parts}
        self.rows: list[dict] = []
        self.counts = {"keys": 0, "entries": 0, "attached": 0, "attachments": 0,
                       "rows": 0, "dropped-short": 0, "dropped-shared": 0}
        self.counts.update({code: 0 for code in SKIP_CODES})

    def find(self, where: str, problem: str) -> None:
        self.findings.append("finding: %s: %s" % (where, problem))

    def summary(self) -> str:
        c = self.counts
        return ("summary: parts=%s keys=%d entries=%d attached=%d attachments=%d %s rows=%d dropped=%d "
                "dropped-short=%d dropped-shared=%d gaps=%d findings=%d"
                % (",".join(self.parts), c["keys"], c["entries"], c["attached"], c["attachments"],
                   " ".join("%s=%d" % (code, c[code]) for code in SKIP_CODES), c["rows"],
                   c["dropped-short"] + c["dropped-shared"], c["dropped-short"], c["dropped-shared"],
                   len(self.gaps), len(self.findings)))

    def write(self, stdout) -> None:
        for line in self.gaps + self.findings + [self.summary()]:
            print(line, file=stdout)


def _entry_problems(entry) -> list[str]:
    """What makes an entry malformed, in the ledger's own terms."""
    if not isinstance(entry, dict):
        return ["an entry is an object"]
    problems = []
    if not isinstance(entry.get("key"), str) or not entry.get("key"):
        problems.append("an entry names a key")
    if not isinstance(entry.get("why"), str) or not entry["why"].strip():
        problems.append("an entry says why, in one short sentence")
    elif len(entry["why"]) > MAX_WHY:
        problems.append("`why` is over %d characters" % MAX_WHY)
    if ("attach" in entry) == ("skip" in entry):
        problems.append("an entry is either attached or skipped")
    elif "skip" in entry:
        if entry["skip"] not in SKIP_CODES:
            problems.append("unknown skip code %r" % entry["skip"])
        if "boss" in entry:
            problems.append("only an attached entry names a boss")
    else:
        attach = entry["attach"]
        if not isinstance(attach, list) or not attach:
            problems.append("`attach` is a non-empty list")
        else:
            for item in attach:
                if not isinstance(item, dict) or set(item) != {"quest", "step", "area"} or \
                        not all(isinstance(v, str) and v for v in item.values()):
                    problems.append("an attachment is {quest, step, area}")
                    break
        if "boss" in entry and (not isinstance(entry["boss"], str) or not entry["boss"]):
            problems.append("`boss` names a boss")
    extra = set(entry) - {"key", "attach", "skip", "boss", "why"}
    if extra:
        problems.append("unknown field(s) %s" % ", ".join(sorted(extra)))
    return problems


def _read_ledger_file(folder: Path, part: str, report: LedgerReport) -> list:
    path = Path(folder) / (part + ".json")
    if not path.is_file():
        report.find(part, "no ledger file %s" % path)
        return []
    try:
        with open(path, encoding="utf-8") as fh:
            ledger = json.load(fh)
    except (OSError, ValueError) as err:
        report.find(part, "%s is not readable JSON (%s)" % (path.name, err))
        return []
    if not isinstance(ledger, dict) or ledger.get("schema") != 1 or ledger.get("part") != part or \
            not isinstance(ledger.get("keys"), list):
        report.find(part, '%s is not {"schema": 1, "part": "%s", "keys": [...]}' % (path.name, part))
        return []
    return ledger["keys"]


def ledger_report(pages: RemarkPages, data: dict, folder: Path, parts: tuple, with_data: bool) -> LedgerReport:
    """Check the ledger files of `parts` against the pages (and the data), and
    build the draft rows of their attached keys."""
    report = LedgerReport(parts)
    quests = {q.get("id"): q for q in data.get("quests", [])}
    areas = data.get("areas", {})
    bosses = set(data_bosses(data))
    seen: dict[str, str] = {}
    valid: dict[str, dict] = {}  # key -> its well-formed entry in the right part
    for part in parts:
        for entry in _read_ledger_file(folder, part, report):
            report.counts["entries"] += 1
            key = entry.get("key") if isinstance(entry, dict) else None
            where = "%s %s" % (part, key) if isinstance(key, str) and key else part
            problems = _entry_problems(entry)
            for problem in problems:
                report.find(where, problem)
            if not isinstance(key, str) or not key:
                continue
            if key in seen:
                report.find(where, "in more than one entry (also in %s)" % seen[key])
                continue
            seen[key] = part
            if key not in pages.texts:
                report.find(where, "not on any class page")
                continue
            if key not in pages.keys:
                report.find(where, "no campaign sound on any class page; not a ledger key")
                continue
            if pages.keys[key]["part"] != part:
                report.find(where, "belongs to %s, not %s" % (pages.keys[key]["part"], part))
                continue
            if problems:
                continue
            valid[key] = entry
            report.entries[part].append(entry)
    for part in parts:
        for key in pages.part_keys(part):
            report.counts["keys"] += 1
            if key not in seen:
                report.find("%s %s" % (part, key), "no entry")

    attached = {key: frozenset((a["quest"], a["step"]) for a in e["attach"])
                for key, e in valid.items() if "attach" in e}
    expected_signals = set()
    for part in parts:
        for entry in report.entries[part]:
            key, where = entry["key"], "%s %s" % (part, entry["key"])
            texts = pages.all_texts(key)
            why = _letters(entry["why"])
            if any(len(_letters(t)) >= MIN_LETTERS and _letters(t) in why for t in texts):
                report.find(where, "`why` quotes a class's line; say it in our own words")
            if "skip" in entry:
                report.counts[entry["skip"]] += 1
                if entry["skip"] == "too-short":
                    usable = [name for name in CLASSES
                              if any(not _too_short(t) for t in pages.class_texts(key, name))]
                    if usable:
                        report.find(where, "skipped as too-short, but %s has a usable text" % ", ".join(usable))
                continue
            report.counts["attached"] += 1
            boss = entry.get("boss")
            if boss is not None and boss not in bosses:
                report.find(where, "boss %r is not a boss the data names" % boss)
            for item in entry["attach"]:
                report.counts["attachments"] += 1
                quest = quests.get(item["quest"])
                step = next((s for s in (quest or {}).get("steps", []) if s.get("id") == item["step"]), None)
                if step is None:
                    report.find(where, "no step %s/%s" % (item["quest"], item["step"]))
                    continue
                if not _act_fits(pages.keys[key]["tags"], quest):
                    report.find(where, "%s is act %s, not the sound's act (rule 2)" % (item["quest"], quest.get("act")))
                if not _area_fits(item["area"], quest, step, key, boss, texts, areas):
                    report.find(where, "area %s is not %s/%s's (rule 3)" % (item["area"], item["quest"], item["step"]))
                expected_signals.add((item["quest"], item["step"], "kill_remark" if boss else "remark",
                                      boss or key, item["area"]))
            # The draft: each class's texts that survive the text rules.
            kept_any = False
            for name in CLASSES:
                kept = []
                for text in pages.class_texts(key, name):
                    if _too_short(text):
                        report.counts["dropped-short"] += 1
                    elif any(seen.get(other) != part or attached.get(other) != attached[key]
                             for other in pages.shared_with(key, text)):
                        report.counts["dropped-shared"] += 1
                    else:
                        kept.append(text)
                record = pages.pages[name]["record"]
                for text in kept:
                    row = {"class": name}
                    if boss is not None:
                        row["boss"] = boss
                    row.update({"key": key, "text": text, "source_url": record["url"], "fetched": record["fetched"]})
                    report.rows.append(row)
                if kept:
                    kept_any = True
                else:
                    report.gaps.append("gap: %s %s: no usable text" % (where, name))
            if not kept_any:
                report.find(where, "no class has a usable text (rule 4)")
    report.counts["rows"] = len(report.rows)
    if with_data:
        _compare_data(report, pages, data, parts, expected_signals)
    return report


def _compare_data(report: LedgerReport, pages: RemarkPages, data: dict, parts: tuple, expected: set) -> None:
    """The data's `remarks` rows against the draft, and its remark and
    kill_remark signals against the attachments, within `parts`."""
    every = tuple(parts) == PART_NAMES
    in_parts = lambda key: every or pages.keys.get(key, {}).get("part") in parts  # noqa: E731
    rows = [row for row in data.get("remarks", []) if in_parts(row.get("key"))]
    if rows != report.rows:
        draft = [json.dumps(r, sort_keys=True) for r in report.rows]
        have = [json.dumps(r, sort_keys=True) for r in rows]
        for row, text in zip(report.rows, draft):
            if text not in have:
                report.find("data", "row missing: %s %s" % (row["class"], row["key"]))
        for row, text in zip(rows, have):
            if text not in draft:
                report.find("data", "row not in the draft: %s %s" % (row.get("class"), row.get("key")))
        if sorted(draft) == sorted(have):
            report.find("data", "the remarks rows are not in the draft's order; run --apply")
    entry_bosses = {e["boss"] for part in parts for e in report.entries[part] if e.get("boss")}
    acts = {PART_ACTS[part] for part in parts}
    actual = set()
    for quest in data.get("quests", []):
        for step in quest.get("steps", []):
            for signal in step.get("signals", []):
                what = signal.get("what")
                if what == "remark" and in_parts(signal.get("key")):
                    actual.add((quest.get("id"), step.get("id"), what, signal.get("key"), signal.get("area")))
                elif what == "kill_remark" and (every or signal.get("boss") in entry_bosses or quest.get("act") in acts):
                    actual.add((quest.get("id"), step.get("id"), what, signal.get("boss"), signal.get("area")))
    for quest, step, what, name, area in sorted(expected - actual, key=str):
        report.find("data", "signal missing: %s/%s %s %s %s" % (quest, step, what, name, area))
    for quest, step, what, name, area in sorted(actual - expected, key=str):
        report.find("data", "signal not in the ledger: %s/%s %s %s %s" % (quest, step, what, name, area))


def ledger_check(cache: Path, data_path: Path, folder: Path, part: str | None = None,
                 ledger_only: bool = False, stdout=None) -> LedgerReport:
    """`remarks --ledger DIR --check`: findings one per line, then a summary line."""
    stdout = stdout or sys.stdout
    pages = load_class_pages(cache)
    parts = (part,) if part else PART_NAMES
    report = ledger_report(pages, _load_data(data_path), folder, parts, with_data=not ledger_only)
    report.write(stdout)
    return report


def _remarks_span(text: str) -> tuple[int, int, int]:
    """Where the top-level `remarks` array sits in a JSON text: the offset of
    its key, of its `[` and just past its `]`. A nested `remarks` is never it."""
    depth, i, key_at, start = 0, 0, None, None
    colon = re.compile(r"\s*:\s*")
    while i < len(text):
        ch = text[i]
        if ch == '"':
            j = i + 1
            while text[j] != '"':
                j += 2 if text[j] == "\\" else 1
            if start is None and depth == 1 and text[i + 1:j] == "remarks":
                m = colon.match(text, j + 1)
                if m and text.startswith("[", m.end()):
                    key_at, start, i = i, m.end(), m.end()
                    continue
            i = j + 1
            continue
        if ch in "[{":
            depth += 1
        elif ch in "]}":
            depth -= 1
            if start is not None and depth == 1:
                return key_at, start, i + 1
        i += 1
    raise ToolError("the data has no top-level `remarks` array")


def render_remarks(text: str, rows: list[dict]) -> str:
    """`text` with its top-level `remarks` array replaced by `rows`, one per
    line, indented one step past the key, in the file's own line endings."""
    key_at, start, end = _remarks_span(text)
    eol = "\r\n" if "\r\n" in text else "\n"
    indent = text[text.rfind("\n", 0, key_at) + 1:key_at]
    if indent.strip():
        indent = ""
    if not rows:
        block = "[]"
    else:
        block = "[" + eol + ("," + eol).join(indent + "  " + json.dumps(row) for row in rows) + eol + indent + "]"
    return text[:start] + block + text[end:]


def ledger_apply(cache: Path, data_path: Path, folder: Path, stdout=None) -> LedgerReport:
    """`remarks --ledger DIR --apply`: rewrite only the data's `remarks` array
    with the draft rows, if the whole ledger passes its `--ledger-only` check."""
    stdout = stdout or sys.stdout
    pages = load_class_pages(cache)
    data_path = Path(data_path)
    raw = data_path.read_bytes()
    text = raw.decode("utf-8")
    report = ledger_report(pages, json.loads(text), folder, PART_NAMES, with_data=False)
    report.write(stdout)
    if report.findings:
        print("apply: the ledger has findings; %s is not written" % data_path, file=stdout)
        return report
    new = render_remarks(text, report.rows)
    if new == text:
        print("apply: %d rows; %s unchanged" % (len(report.rows), data_path), file=stdout)
    else:
        data_path.write_bytes(new.encode("utf-8"))
        print("apply: %d rows written to %s" % (len(report.rows), data_path), file=stdout)
    return report


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


def coverage(cache: Path, log: Path, stdout=None, stderr=None) -> dict:
    """Per cached page: dialogue snippets, how many equal or contain a logged line
    of the same NPC, how many share a 4-word run with one. Counts only."""
    stdout = stdout or sys.stdout
    stderr = stderr or sys.stderr
    cache = Path(cache)
    index = _require_cache_index(cache)
    pages = sorted(_cached_quest_pages(cache, index))
    if not pages:
        raise ExtractError("%s: no quest page in the cache; run `fetch` first" % INDEX_SLUG)
    parsed = [(slug, parse_quest_page(slug, _read_cached(cache, rec), with_dialogue=True))
              for slug, rec in pages]
    _check_steps_tables(parsed, stderr)
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
        description="Developer-only poe2db.tw quest tooling: fetch, fetch-classes, extract, remarks, coverage.")
    sub = parser.add_subparsers(dest="command", required=True, metavar="{fetch,fetch-classes,extract,remarks,coverage}")

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

    fc = sub.add_parser("fetch-classes", help="fetch class pages (for their remarks) into the cache, politely")
    fc.add_argument("classes", nargs="+", metavar="CLASS", help="class page names, e.g. Warrior Mercenary")
    fc.add_argument("--cache", type=Path, default=None, help="cache folder (default as for fetch)")
    fc.add_argument("--delay", type=float, default=DEFAULT_DELAY,
                    help="seconds between requests (default %s, minimum %s)" % (DEFAULT_DELAY, MIN_DELAY))
    fc.add_argument("--max-age", type=int, default=DEFAULT_MAX_AGE, metavar="DAYS",
                    help="leave pages fetched within this many days alone (default %d)" % DEFAULT_MAX_AGE)

    e = sub.add_parser("extract", help="draft one JSON entry per cached quest page")
    e.add_argument("--cache", type=Path, default=None, help="cache folder (default as for fetch)")
    e.add_argument("--out", type=Path, default=None,
                   help="draft file, outside any git checkout (default <cache>\\%s)" % DRAFT_NAME)
    e.add_argument("--with-dialogue", action="store_true", help="include each step's NPC dialogue")

    r = sub.add_parser("remarks", help="print a kill-remark draft, or with --keys/--ledger curate and check the remark ledger")
    r.add_argument("--cache", type=Path, default=None, help="cache folder (default as for fetch)")
    r.add_argument("--data", type=Path, default=DEFAULT_DATA,
                   help="the quest data whose bosses to keep (default apps/quest-tracker/data/quests.json)")
    r.add_argument("--class", dest="classes", nargs="+", metavar="CLASS", default=None,
                   help="only these cached class pages (default every one)")
    r.add_argument("--keys", action="store_true",
                   help="print one JSON line per campaign key of --part: its sound tags, each class's usable "
                        "texts, the keys sharing a text, up to five candidate steps")
    r.add_argument("--part", choices=PART_NAMES, default=None,
                   help="the part --keys lists, or the one ledger file --check reads")
    r.add_argument("--ledger", type=Path, default=None, metavar="DIR",
                   help="the remark ledger folder, one <part>.json per part (with --check or --apply)")
    r.add_argument("--check", action="store_true",
                   help="check the ledger against the class pages and the data; exit 0 only with no finding")
    r.add_argument("--ledger-only", action="store_true",
                   help="with --check: leave out the data's remarks rows and remark signals")
    r.add_argument("--apply", action="store_true",
                   help="rewrite only the data's remarks array with the ledger's draft rows")

    c = sub.add_parser("coverage", help="count how far cached dialogue matches a Client.txt (counts only)")
    c.add_argument("--log", type=Path, required=True, help="a Client.txt, read-only")
    c.add_argument("--cache", type=Path, default=None, help="cache folder (default as for fetch)")
    return parser


def _check_remarks_options(parser: argparse.ArgumentParser, args) -> None:
    """`remarks` alone drafts kill rows as it always has; the ledger options
    combine only as `--keys --part P`, `--ledger DIR --check [--part P]
    [--ledger-only]` and `--ledger DIR --apply`."""
    ledger_mode = args.keys or args.check or args.apply or args.ledger_only or args.ledger is not None
    if not ledger_mode:
        if args.part is not None:
            parser.error("remarks: --part goes with --keys or --check")
        return
    if args.classes:
        parser.error("remarks: --class does not go with the ledger options; they read all eight classes")
    if args.keys:
        if args.part is None:
            parser.error("remarks: --keys needs --part")
        if args.check or args.apply or args.ledger_only or args.ledger is not None:
            parser.error("remarks: --keys does not go with --ledger, --check, --ledger-only or --apply")
        return
    if args.ledger is None:
        parser.error("remarks: --check and --apply need --ledger DIR")
    if args.check == args.apply:
        parser.error("remarks: --ledger needs exactly one of --check and --apply")
    if args.ledger_only and not args.check:
        parser.error("remarks: --ledger-only goes with --check")
    if args.apply and args.part is not None:
        parser.error("remarks: --apply writes every part's rows; it takes no --part")


def main(argv: list[str] | None = None, fetcher=None, clock=None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    if args.command == "remarks":
        _check_remarks_options(parser, args)
    cache = args.cache if args.cache is not None else default_cache_dir()
    try:
        if args.command == "fetch":
            if not args.delay >= MIN_DELAY:
                print("poe2db: error: refusing --delay %s: the minimum is %s seconds between requests"
                      % (args.delay, MIN_DELAY), file=sys.stderr)
                return 2
            fetch(cache, delay=args.delay, only=args.only, max_age=args.max_age,
                  include_endgame=args.include_endgame, fetcher=fetcher, clock=clock)
        elif args.command == "fetch-classes":
            if not args.delay >= MIN_DELAY:
                print("poe2db: error: refusing --delay %s: the minimum is %s seconds between requests"
                      % (args.delay, MIN_DELAY), file=sys.stderr)
                return 2
            fetch_classes(cache, args.classes, delay=args.delay, max_age=args.max_age,
                          fetcher=fetcher, clock=clock)
        elif args.command == "extract":
            extract(cache, out=args.out, with_dialogue=args.with_dialogue, clock=clock)
        elif args.command == "remarks":
            if args.keys:
                remark_keys(cache, args.data, args.part)
            elif args.check:
                report = ledger_check(cache, args.data, args.ledger, part=args.part, ledger_only=args.ledger_only)
                return 1 if report.findings else 0
            elif args.apply:
                return 1 if ledger_apply(cache, args.data, args.ledger).findings else 0
            else:
                remarks(cache, args.data, classes=args.classes, clock=clock)
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
