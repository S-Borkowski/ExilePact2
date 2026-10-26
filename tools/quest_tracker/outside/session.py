"""The live folder: prepare it, anchor the clock, ask the instruments to stop.

Developer-only. Usage:

    py -3 -m tools.quest_tracker.outside.session prep [--live L] [--client-txt P] [--ggpk G]
    py -3 -m tools.quest_tracker.outside.session clock [--live L]
    py -3 -m tools.quest_tracker.outside.session stop [--live L] [--only i1,i2,i3,marker]

`prep` creates `L` and its subfolders, refusing an `L` that already holds
anything, and writes `session.json`: when it was made, the `Client.txt` path
and its size now (the offset the analysis reads from), and the `Content.ggpk`
path, size and modification time. It only looks at the two files' sizes and
times; it reads neither. It also records the first clock anchor.

`clock` appends one anchor, `{"utc": ..., "tick_ms": <GetTickCount64>}`, to
`L\\clock.jsonl`, the best of five samples.

`stop` creates the empty files `stop-i1`, `stop-i2`, `stop-i3` and
`stop-marker` in `L` (or only those `--only` names); each instrument ends
when its file appears.

`L` defaults to `%USERPROFILE%\\quest-tracker-live-5\\beira-defeat\\`, and
every command refuses one inside a git checkout.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from tools.quest_tracker.outside import common
from tools.quest_tracker.outside.common import ToolError


def prep(live: Path, client_txt: Path, ggpk: Path, sampler=common.sample_anchor,
         now=common.now_utc) -> dict:
    live = Path(live)
    common.refuse_inside_git(live, "live folder")
    if live.exists():
        if not live.is_dir():
            raise ToolError("the live folder %s exists and is not a folder" % live)
        if any(live.iterdir()):
            raise ToolError("refusing to prepare %s: it is not empty. Each session gets a "
                            "fresh folder; move the old one away first." % live)
    client_txt, ggpk = Path(client_txt), Path(ggpk)
    if not client_txt.is_file():
        raise ToolError("no Client.txt at %s" % client_txt)
    if not ggpk.is_file():
        raise ToolError("no Content.ggpk at %s" % ggpk)
    log_stat, ggpk_stat = client_txt.stat(), ggpk.stat()

    for sub in common.SUBFOLDERS:
        (live / sub).mkdir(parents=True, exist_ok=True)
    session = {
        "created_utc": common.format_utc(now()),
        "client_txt": str(client_txt),
        "client_txt_offset": log_stat.st_size,
        "ggpk": str(ggpk),
        "ggpk_size": ggpk_stat.st_size,
        "ggpk_mtime": common.format_utc(common.utc_from_epoch_ns(ggpk_stat.st_mtime_ns)),
    }
    common.write_json(live / common.SESSION_JSON, session)
    clock(live, sampler=sampler)
    return session


def clock(live: Path, sampler=common.sample_anchor) -> dict:
    live = Path(live)
    common.refuse_inside_git(live, "clock anchor")
    if not live.is_dir():
        raise ToolError("no live folder at %s: run `session prep` first" % live)
    anchor = sampler()
    common.append_jsonl(live / common.CLOCK_JSONL, anchor)
    return anchor


def parse_only(values: list[str] | None) -> list[str]:
    """`--only i1,i3` or `--only i1 i3` as a list of instrument names."""
    if not values:
        return list(common.INSTRUMENTS)
    names = [n.strip() for v in values for n in v.split(",") if n.strip()]
    for name in names:
        if name not in common.INSTRUMENTS:
            raise ToolError("unknown instrument %r for --only: expected %s"
                            % (name, ", ".join(common.INSTRUMENTS)))
    return names


def stop(live: Path, only: list[str] | None = None) -> list[Path]:
    live = Path(live)
    common.refuse_inside_git(live, "stop file")
    if not live.is_dir():
        raise ToolError("no live folder at %s" % live)
    made = []
    for name in parse_only(only):
        path = common.stop_file(live, name)
        path.touch(exist_ok=True)
        made.append(path)
    return made


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="session", description="Prepare the live folder, anchor the clock, stop the instruments.")
    sub = parser.add_subparsers(dest="command", required=True)
    p = sub.add_parser("prep", help="create the live folder and session.json")
    p.add_argument("--live", type=Path, default=None)
    p.add_argument("--client-txt", type=Path, default=common.DEFAULT_CLIENT_TXT)
    p.add_argument("--ggpk", type=Path, default=common.DEFAULT_GGPK)
    c = sub.add_parser("clock", help="append one clock anchor to clock.jsonl")
    c.add_argument("--live", type=Path, default=None)
    s = sub.add_parser("stop", help="create the stop files")
    s.add_argument("--live", type=Path, default=None)
    s.add_argument("--only", nargs="+", default=None, metavar="NAME",
                   help="instruments to stop: %s (comma- or space-separated)"
                   % ", ".join(common.INSTRUMENTS))
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    live = args.live if args.live is not None else common.default_live_dir()
    try:
        if args.command == "prep":
            session = prep(live, args.client_txt, args.ggpk)
            print("session: prepared %s (client_txt_offset %d, ggpk_size %d)"
                  % (live, session["client_txt_offset"], session["ggpk_size"]))
        elif args.command == "clock":
            anchor = clock(live)
            print("session: clock %s tick_ms %d" % (anchor["utc"], anchor["tick_ms"]))
        else:
            for path in stop(live, args.only):
                print("session: stop %s" % path.name)
    except ToolError as err:
        print("session: error: %s" % err, file=sys.stderr)
        return 1
    except (OSError, ValueError, json.JSONDecodeError) as err:
        print("session: error: %s" % err, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
