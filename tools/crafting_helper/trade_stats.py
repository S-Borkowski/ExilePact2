"""The trade site's stat ids for the crafting helper's modifiers: fetch the public list, map, check.

Developer-only. The app never asks the trade site for its stats; this tool
does, once per refresh, and ships the result as data:

    py -3 -m tools.crafting_helper.trade_stats fetch [--cache DIR]
    py -3 -m tools.crafting_helper.trade_stats map [--cache DIR] [--data DIR]
    py -3 -m tools.crafting_helper.trade_stats check [--data DIR]

`fetch` sends one GET to the trade site's public stat list
(`https://www.pathofexile.com/api/trade2/data/stats`, no login) with a
User-Agent naming this tool and repository, and keeps the answer in a cache
outside every git checkout, by default
`%LOCALAPPDATA%\\ExilePact2\\crafting-helper\\trade-cache\\` (`stats.json`, the
raw list, and `index.json`, its URL, fetch date and HTTP status). An answer
that is not HTTP 200 or not the list's shape stops the run and leaves the
cache as it was.

`map` works offline from the cache and the class files under
`apps/crafting-helper/data/`, and writes `trade-stats.json` beside them. For
every class and every ordinary-pool (`normal`) modifier it records either the
trade text and the stat ids to search for, or that the modifier is not
searchable and why:

- a modifier matches by its first line with one leading `+` removed, against
  the texts of the list's `explicit` group, exactly;
- a modifier with more than one line is a hybrid (the site lists each line as
  its own stat, and a hybrid's first line is often a plain modifier's text),
  so it is recorded as not searchable, reason `hybrid`, never mapped by a line;
- a line the site does not list is not searchable, reason `no trade stat`;
  nothing is rewritten to make it match (a `reduced` is not turned into an
  `increased`): a guess is a wrong search;
- a text the site lists under several ids keeps every id;
- an armour class (except for Accuracy Rating) and a martial weapon take the
  `<text> (Local)` id when the site lists one; every other class, and a text
  with no `(Local)` entry, takes the plain id. This is a static reading of the
  site's list and the game's local-modifier convention, not a measurement.

The file keeps only the matched entries' text and ids, the source URL and the
fetch date, never the rest of the list. `map` prints its coverage counts.

`check` holds the shipped file to that schema and to the class files: every
class of `index.json`, every normal modifier of each class file and nothing
else, ids that start `explicit.stat_`, the source URL and an ISO fetch date.
It prints each problem and exits 1 on any, 0 when the file is clean.
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import os
import sys
from pathlib import Path

from tools.quest_tracker.poe2db import ToolError, git_tree_of, urllib_fetcher

STATS_HOST = "www.pathofexile.com"
STATS_URL = "https://%s/api/trade2/data/stats" % STATS_HOST
USER_AGENT = (
    "ExilePact2-crafting-helper-trade-stats/1 (developer data tool, one request per refresh; "
    "+https://github.com/S-Borkowski/ExilePact2)"
)
CACHE_INDEX = "index.json"
CACHE_FILE = "stats.json"
DATA_INDEX = "index.json"
DATA_FILE = "trade-stats.json"
DEFAULT_DATA = Path(__file__).resolve().parents[2] / "apps" / "crafting-helper" / "data"

SCHEMA = 1
SOURCE_NOTE = (
    "Trade stat ids and texts from the Path of Exile trade site's public stat list; the texts are "
    "game text by Grinding Gear Games, quoted under the owner's 2026-10-24 decision. Only the "
    "entries matched to the crafting helper's modifiers are kept."
)

EXPLICIT_PREFIX = "explicit.stat_"
LOCAL_SUFFIX = " (Local)"
REASON_HYBRID = "hybrid"
REASON_NO_STAT = "no trade stat"
REASONS = (REASON_HYBRID, REASON_NO_STAT)
SECTION = "normal"

# The classes whose own defences are local modifiers, and the martial weapons,
# whose damage and accuracy modifiers are. Accuracy on an armour piece is global.
ARMOUR_CLASSES = frozenset(("Gloves", "Boots", "Helmets", "Body Armours", "Shields", "Bucklers", "Foci"))
MARTIAL_WEAPONS = frozenset(("Bows", "Crossbows", "One Hand Maces", "Two Hand Maces", "Quarterstaves",
                             "Spears", "Talismans"))
GLOBAL_ON_ARMOUR = "Accuracy Rating"


def default_cache_dir() -> Path:
    base = os.environ.get("LOCALAPPDATA") or str(Path.home() / "AppData" / "Local")
    return Path(base) / "ExilePact2" / "crafting-helper" / "trade-cache"


def dumps(data: dict) -> str:
    return json.dumps(data, indent=2, ensure_ascii=False) + "\n"


def _read_json(path: Path):
    return json.loads(Path(path).read_bytes().decode("utf-8").replace("\r\n", "\n"))


def _write(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_name(path.name + ".tmp")
    with open(tmp, "w", encoding="utf-8", newline="\n") as fh:
        fh.write(text)
    os.replace(tmp, path)


# --------------------------------------------------------------------------
# The rules

def trade_text(line: str) -> str:
    """A modifier line as the trade site writes it: one leading `+` removed, nothing else changed."""
    return line[1:] if line.startswith("+") else line


def uses_local(class_name: str, text: str) -> bool:
    """Whether this class's modifier with this text is a local stat on the trade site."""
    if class_name in MARTIAL_WEAPONS:
        return True
    return class_name in ARMOUR_CLASSES and GLOBAL_ON_ARMOUR not in text


def explicit_texts(stats: dict) -> dict[str, list[str]]:
    """The `explicit` group's texts, each with every id the site lists it under, in list order."""
    groups = stats.get("result") if isinstance(stats, dict) else None
    if not isinstance(groups, list):
        raise ToolError("the stat list has no `result` list")
    texts: dict[str, list[str]] = {}
    found = False
    for group in groups:
        if not isinstance(group, dict) or group.get("id") != "explicit":
            continue
        found = True
        for entry in group.get("entries") or []:
            if not isinstance(entry, dict) or not isinstance(entry.get("id"), str) \
                    or not isinstance(entry.get("text"), str):
                raise ToolError("an explicit entry without a string id and text: %r" % (entry,))
            ids = texts.setdefault(entry["text"], [])
            if entry["id"] not in ids:
                ids.append(entry["id"])
    if not found or not texts:
        raise ToolError("the stat list has no `explicit` group with entries")
    return texts


def map_mod(class_name: str, mod: dict, texts: dict[str, list[str]]) -> dict:
    lines = mod.get("lines") or []
    if len(lines) != 1:
        return {"searchable": False, "reason": REASON_HYBRID}
    text = trade_text(lines[0])
    if text not in texts:
        return {"searchable": False, "reason": REASON_NO_STAT}
    local = text + LOCAL_SUFFIX
    if local in texts and uses_local(class_name, text):
        text = local
    return {"text": text, "ids": list(texts[text])}


def build_mapping(stats: dict, class_files: list[dict], fetched: str) -> dict:
    """The trade-stats.json document for these class files, in their order."""
    texts = explicit_texts(stats)
    classes: dict[str, dict] = {}
    for data in class_files:
        name = data["class"]
        classes[name] = {mod["id"]: map_mod(name, mod, texts)
                         for mod in data["mods"] if mod.get("section") == SECTION}
    return {"schema": SCHEMA, "source": STATS_URL, "fetched": fetched, "source_note": SOURCE_NOTE,
            "classes": classes}


def _counts(entries: dict) -> dict[str, int]:
    counts = {"modifiers": len(entries), "searchable": 0, "local": 0, REASON_HYBRID: 0, REASON_NO_STAT: 0}
    for entry in entries.values():
        if "ids" in entry:
            counts["searchable"] += 1
            counts["local"] += entry["text"].endswith(LOCAL_SUFFIX)
        else:
            counts[entry["reason"]] += 1
    return counts


def coverage(mapping: dict) -> dict[str, int]:
    """Totals over every class: modifiers, searchable, local, hybrid, no trade stat."""
    total = _counts({})
    for entries in mapping["classes"].values():
        for key, n in _counts(entries).items():
            total[key] += n
    return total


# --------------------------------------------------------------------------
# Check

def _is_iso_date(value) -> bool:
    if not isinstance(value, str):
        return False
    try:
        return _dt.date.fromisoformat(value).isoformat() == value
    except ValueError:
        return False


def _check_entry(where: str, mod: dict, entry) -> list[str]:
    if not isinstance(entry, dict):
        return ["%s: not an object" % where]
    lines = mod.get("lines") or []
    if "ids" in entry or "text" in entry:
        problems = []
        if set(entry) != {"text", "ids"}:
            problems.append("%s: keys %s, want text and ids" % (where, sorted(entry)))
        text, ids = entry.get("text"), entry.get("ids")
        if not isinstance(text, str) or not text:
            problems.append("%s: text is not a non-empty string" % where)
        if not isinstance(ids, list) or not ids:
            problems.append("%s: no ids" % where)
        else:
            for stat_id in ids:
                if not isinstance(stat_id, str) or not stat_id.startswith(EXPLICIT_PREFIX):
                    problems.append("%s: id %r does not start %s" % (where, stat_id, EXPLICIT_PREFIX))
            if len(set(map(str, ids))) != len(ids):
                problems.append("%s: an id repeats" % where)
        if len(lines) != 1:
            problems.append("%s: has %d lines, and a hybrid is never searchable" % (where, len(lines)))
        return problems
    if set(entry) != {"searchable", "reason"} or entry.get("searchable") is not False:
        return ["%s: keys %s, want text and ids, or searchable false and a reason" % (where, sorted(entry))]
    reason = entry.get("reason")
    if reason not in REASONS:
        return ["%s: unknown reason %r (want one of %s)" % (where, reason, ", ".join(REASONS))]
    if (reason == REASON_HYBRID) != (len(lines) != 1):
        return ["%s: reason %r for a modifier of %d line(s)" % (where, reason, len(lines))]
    return []


def check_mapping(mapping, class_files: list[dict]) -> list[str]:
    """Every problem with a trade-stats.json document against these class files."""
    if not isinstance(mapping, dict):
        return ["not a JSON object"]
    problems = []
    if mapping.get("schema") != SCHEMA:
        problems.append("schema is %r, want %d" % (mapping.get("schema"), SCHEMA))
    if "source" not in mapping:
        problems.append("no source")
    elif mapping["source"] != STATS_URL:
        problems.append("source URL is %r, want %s" % (mapping["source"], STATS_URL))
    if "fetched" not in mapping:
        problems.append("no fetched date")
    elif not _is_iso_date(mapping["fetched"]):
        problems.append("fetched %r is not an ISO date" % (mapping["fetched"],))
    note = mapping.get("source_note")
    if not isinstance(note, str) or "Grinding Gear Games" not in note:
        problems.append("source_note is missing or does not credit Grinding Gear Games")
    extra = set(mapping) - {"schema", "source", "fetched", "source_note", "classes"}
    if extra:
        problems.append("unexpected top-level keys %s" % sorted(extra))
    classes = mapping.get("classes")
    if not isinstance(classes, dict):
        return problems + ["classes is not an object"]
    want = [data["class"] for data in class_files]
    if list(classes) != want:
        problems.append("classes are %s, want the class files' %s" % (list(classes), want))
    by_name = {data["class"]: data for data in class_files}
    for name, entries in classes.items():
        data = by_name.get(name)
        if data is None:
            continue
        if not isinstance(entries, dict):
            problems.append("%s: not an object" % name)
            continue
        mods = {mod["id"]: mod for mod in data["mods"]}
        normal = [mod["id"] for mod in data["mods"] if mod.get("section") == SECTION]
        for mod_id, entry in entries.items():
            where = "%s %s" % (name, mod_id)
            mod = mods.get(mod_id)
            if mod is None:
                problems.append("%s: not in the class file" % where)
            elif mod.get("section") != SECTION:
                problems.append("%s: a %s modifier, not an ordinary-pool one" % (where, mod.get("section")))
            else:
                problems.extend(_check_entry(where, mod, entry))
        for mod_id in normal:
            if mod_id not in entries:
                problems.append("%s %s: missing" % (name, mod_id))
        if [m for m in entries if m in normal] != normal:
            problems.append("%s: modifiers are not in the class file's order" % name)
    return problems


def load_class_files(data_dir: Path) -> list[dict]:
    index = _read_json(Path(data_dir) / DATA_INDEX)
    return [_read_json(Path(data_dir) / entry["file"]) for entry in index["classes"]]


def check_dir(data_dir: Path) -> list[str]:
    """Every problem with `trade-stats.json` under `data_dir`, against the class files beside it."""
    data_dir = Path(data_dir)
    path = data_dir / DATA_FILE
    if not path.is_file():
        return ["%s is missing; run `fetch` and `map` first" % path]
    try:
        class_files = load_class_files(data_dir)
    except (OSError, UnicodeDecodeError, ValueError, KeyError, TypeError) as err:
        return ["the class files under %s cannot be read: %s" % (data_dir, err)]
    try:
        text = path.read_bytes().decode("utf-8").replace("\r\n", "\n")
        mapping = json.loads(text)
    except (UnicodeDecodeError, ValueError) as err:
        return ["%s: %s" % (path, err)]
    problems = check_mapping(mapping, class_files)
    if text != dumps(mapping):
        problems.append("%s is not written as map writes it" % DATA_FILE)
    return problems


# --------------------------------------------------------------------------
# Fetch, map, check

def fetch(cache: Path, fetcher=None, today: _dt.date | None = None, out=None) -> dict:
    """One GET of the stat list into the cache. Returns the cache record."""
    out = out or sys.stdout
    cache = Path(cache)
    tree = git_tree_of(cache)
    if tree is not None:
        raise ToolError("refusing to write the trade cache at %s: it is inside the git checkout %s. The raw "
                        "stat list stays outside every repository; pass a folder elsewhere." % (cache, tree))
    status, body = (fetcher or urllib_fetcher)(STATS_URL, USER_AGENT)
    if status != 200:
        raise ToolError("%s answered HTTP %s; the cache is left as it was" % (STATS_URL, status))
    try:
        stats = json.loads(body.decode("utf-8"))
        texts = explicit_texts(stats)
    except (UnicodeDecodeError, ValueError) as err:
        raise ToolError("%s did not answer JSON (%s); the cache is left as it was" % (STATS_URL, err))
    except ToolError as err:
        raise ToolError("%s is not the stat list's shape (%s); the cache is left as it was" % (STATS_URL, err))
    record = {"url": STATS_URL, "fetched": (today or _dt.date.today()).isoformat(), "status": status,
              "file": CACHE_FILE}
    cache.mkdir(parents=True, exist_ok=True)
    _write(cache / CACHE_FILE, body.decode("utf-8"))
    _write(cache / CACHE_INDEX, dumps(record))
    print("fetched %s: %d explicit texts, into %s" % (STATS_URL, len(texts), cache), file=out)
    return record


def write_map(cache: Path, data_dir: Path | None = None, out=None) -> dict:
    """Write trade-stats.json from the cache and the class files. Returns the document written."""
    out = out or sys.stdout
    cache = Path(cache)
    data_dir = Path(data_dir) if data_dir is not None else DEFAULT_DATA
    if not (cache / CACHE_INDEX).is_file() or not (cache / CACHE_FILE).is_file():
        raise ToolError("no cached stat list in %s; run `fetch` first" % cache)
    record = _read_json(cache / CACHE_INDEX)
    if not _is_iso_date(record.get("fetched")):
        raise ToolError("the cache record has no ISO fetch date; run `fetch` again")
    class_files = load_class_files(data_dir)
    mapping = build_mapping(_read_json(cache / CACHE_FILE), class_files, record["fetched"])
    problems = check_mapping(mapping, class_files)
    if problems:
        raise ToolError("the mapping breaks the file rules, nothing written: " + "; ".join(problems[:8]))
    _write(data_dir / DATA_FILE, dumps(mapping))
    for name, entries in mapping["classes"].items():
        c = _counts(entries)
        print("%s: %d modifiers, %d searchable (%d local), %d hybrid, %d no trade stat"
              % (name, c["modifiers"], c["searchable"], c["local"], c[REASON_HYBRID], c[REASON_NO_STAT]),
              file=out)
    c = coverage(mapping)
    print("total: %d modifiers, %d searchable (%d local), %d hybrid, %d no trade stat; wrote %s"
          % (c["modifiers"], c["searchable"], c["local"], c[REASON_HYBRID], c[REASON_NO_STAT],
             data_dir / DATA_FILE), file=out)
    return mapping


def check(data_dir: Path | None = None, out=None) -> list[str]:
    out = out or sys.stdout
    data_dir = Path(data_dir) if data_dir is not None else DEFAULT_DATA
    problems = check_dir(data_dir)
    for problem in problems:
        print(problem, file=out)
    print("%s: %s" % (data_dir / DATA_FILE, "%d problem(s)" % len(problems) if problems else "clean"), file=out)
    return problems


# --------------------------------------------------------------------------
# Command line

def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="py -3 -m tools.crafting_helper.trade_stats",
        description="Developer-only: fetch the trade site's public stat list, map the crafting helper's "
                    "modifiers to its stat ids, and check the mapping.")
    sub = parser.add_subparsers(dest="command", required=True, metavar="{fetch,map,check}")
    cache_help = ("cache folder, outside any git checkout "
                  "(default %%LOCALAPPDATA%%\\ExilePact2\\crafting-helper\\trade-cache)")
    data_help = "data folder (default apps/crafting-helper/data)"

    f = sub.add_parser("fetch", help="one GET of the trade site's stat list into the cache")
    f.add_argument("--cache", type=Path, default=None, help=cache_help)

    m = sub.add_parser("map", help="write trade-stats.json from the cached list and the class files")
    m.add_argument("--cache", type=Path, default=None, help=cache_help)
    m.add_argument("--data", type=Path, default=None, help=data_help)

    c = sub.add_parser("check", help="hold trade-stats.json to the schema and the class files")
    c.add_argument("--data", type=Path, default=None, help=data_help)
    return parser


def main(argv: list[str] | None = None, fetcher=None, today: _dt.date | None = None) -> int:
    args = build_parser().parse_args(argv)
    try:
        if args.command == "check":
            return 1 if check(args.data) else 0
        cache = args.cache if args.cache is not None else default_cache_dir()
        if args.command == "fetch":
            fetch(cache, fetcher=fetcher, today=today)
        else:
            write_map(cache, args.data)
    except (ToolError, OSError, ValueError, KeyError) as err:
        print("trade_stats: error: %s" % err, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
