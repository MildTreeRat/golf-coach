"""Dev CLI: flag a topped shot so it stops dragging down a club's carry average. [M16 P5]

Usage:
    python scripts/flag_mishit.py 2026-08-10/3 --confirm   # yes, a mishit -- hold it out
    python scripts/flag_mishit.py 2026-08-10/3 --clear     # no, a real shot -- count it
    python scripts/flag_mishit.py 2026-08-10/3 --auto      # remove the verdict; the rule decides
    python scripts/flag_mishit.py --list                   # every golfer's per-club mishit tally
    python scripts/flag_mishit.py --list --name Aaron      # one golfer

`analysis/club_profile.py` flags a carry below half its club's median automatically, once the club
has five clean shots. This CLI is for the shots that rule cannot see: a heavier miss the golfer
wants gone, or a punch shot the rule flagged that should stay. One swing at a time, because a
session has many shots and nothing but memory says which was a top (ADR-028, ADR-024 §5).

Exits 0 on a successful flag or a listing; 2 on input this cannot use.
"""

from __future__ import annotations

import argparse
import sys

from golf_coach.analysis.club_profile import build_bag_profile
from golf_coach.config import settings
from golf_coach.contracts.golfer import slugify
from golf_coach.contracts.mishit import MishitVerdict
from golf_coach.storage.bundle_store import SwingBundleStore
from golf_coach.storage.corpus import read_corpus
from golf_coach.storage.golfer_store import GolferStore


def _flag(store: SwingBundleStore, ref: str, verdict: MishitVerdict | None) -> int:
    if ref.count("/") != 1 or not all(ref.split("/")):
        print(f"'{ref}' is not a SESSION/SWING reference, e.g. 2026-08-10/3")
        return 2
    session_id, swing_id = ref.split("/")
    manifest = store.set_mishit(session_id, swing_id, verdict)
    if manifest is None:
        print(f"no swing {ref}")
        return 2
    said = verdict.value if verdict else "auto (no verdict -- the rule decides)"
    print(f"{ref}: mishit = {said}")
    return 0


def _list(store: SwingBundleStore, golfers: GolferStore, only: str | None) -> int:
    if only is not None and golfers.get(only) is None:
        print(f"no golfer '{only}'")
        return 2
    ids = [only] if only else [g.player_id for g in golfers.list_all()]
    for player_id in ids:
        bag = build_bag_profile(read_corpus(settings.sessions_dir, player_id))
        rows = [club for club in bag.clubs if club.n_shots > 0]
        print(f"\n{player_id} -- {bag.mishits_excluded} shot(s) held out across the bag")
        if not rows:
            print("  (no tagged shots yet)")
            continue
        for club in rows:
            if club.mishits:
                ruled = club.mishits - club.mishits_unconfirmed
                note = (
                    f"{club.mishits} mishit(s): {ruled} confirmed, "
                    f"{club.mishits_unconfirmed} auto and unconfirmed"
                )
            else:
                note = "clean"
            print(f"  {club.club.value:<6} {club.n_shots:>3} shots   {note}")
            for ref in club.mishit_refs:
                print(f"           {ref}")
    return 0


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("ref", nargs="?", help="SESSION/SWING, e.g. 2026-08-10/3")
    parser.add_argument("--confirm", action="store_true", help="mark it a mishit")
    parser.add_argument("--clear", action="store_true", help="mark it a real shot")
    parser.add_argument("--auto", action="store_true", help="remove any verdict; the rule decides")
    parser.add_argument("--list", action="store_true", help="show every club's mishit tally")
    parser.add_argument("--name", help="with --list, limit to one golfer by name")
    args = parser.parse_args(argv)

    store = SwingBundleStore(settings.sessions_dir)

    if args.list:
        only = slugify(args.name) if args.name else None
        return _list(store, GolferStore(settings.golfers_dir), only)

    chosen = [flag for flag in ("confirm", "clear", "auto") if getattr(args, flag)]
    if args.ref is None or len(chosen) != 1:
        parser.error("give a SESSION/SWING ref and exactly one of --confirm / --clear / --auto")
    verdict = {
        "confirm": MishitVerdict.CONFIRMED,
        "clear": MishitVerdict.CLEARED,
        "auto": None,
    }[chosen[0]]
    return _flag(store, args.ref, verdict)


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
