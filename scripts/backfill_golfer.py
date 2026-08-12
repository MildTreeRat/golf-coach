"""Dev CLI: attribute already-uploaded swings to a golfer. [Career mode, step 1]

Usage:
    python scripts/backfill_golfer.py --name Aaron --handedness right   # every unlabeled swing
    python scripts/backfill_golfer.py --name Aaron --session 2026-08-10 # one session
    python scripts/backfill_golfer.py --name Aaron --dry-run            # show, change nothing

Swings uploaded before `player_id` existed carry no golfer. Nothing can derive one for them —
the clips do not say who is in them — so this is the one-time human answer, applied in bulk.

**Unlabeled swings only, always.** A swing that already names a golfer is left untouched, which
makes the script idempotent and makes a second run over a mixed session safe. Correcting a swing
that names the *wrong* golfer is a different operation with a different blast radius, and it lives
on the upload page as an explicit per-swing override (`POST .../swings/{id}/golfer`).

Creating a golfer needs `--handedness`; naming one that already exists does not, and the stored
value wins either way.
"""

from __future__ import annotations

import argparse
import sys

from golf_coach.config import settings
from golf_coach.contracts.golfer import Handedness, slugify
from golf_coach.storage.bundle_store import SwingBundleStore
from golf_coach.storage.golfer_store import GolferStore


def _sessions(store: SwingBundleStore, only: str | None) -> list[str]:
    return [only] if only is not None else store.list_session_ids()


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--name", required=True, help="Golfer's name, e.g. 'Aaron'.")
    parser.add_argument(
        "--handedness",
        choices=[h.value for h in Handedness],
        help="Required only when creating a golfer the registry has never seen.",
    )
    parser.add_argument("--session", help="Limit to one session id; default is every session.")
    parser.add_argument("--dry-run", action="store_true", help="Report, change nothing.")
    args = parser.parse_args(argv)

    store = SwingBundleStore(settings.sessions_dir)
    golfers = GolferStore(settings.golfers_dir)

    player_id = slugify(args.name)
    if not player_id:
        print(f"'{args.name}' has no usable characters for an id.")
        return 2

    golfer = golfers.get(player_id)
    if golfer is None:
        if args.handedness is None:
            print(
                f"'{args.name}' is new here. Pass --handedness right|left to create them.\n"
                "It is not guessable and not recoverable later: it is the frame of reference for "
                "every signed metric, so a wrong value silently inverts them."
            )
            return 2
        if args.dry_run:
            print(f"would create golfer {player_id} ({args.name}, {args.handedness}-handed)")
        else:
            golfer = golfers.get_or_create(args.name, Handedness(args.handedness))
            print(f"created golfer {golfer.player_id} ({golfer.handedness.value}-handed)")

    total = 0
    for session_id in _sessions(store, args.session):
        manifests = store.get_session(session_id)
        unlabeled = [m.swing_id for m in manifests if m.player_id is None]
        already = {m.player_id for m in manifests if m.player_id is not None}
        if not unlabeled:
            note = f" (all {len(manifests)} already attributed)" if manifests else ""
            print(f"{session_id}: nothing to do{note}")
            continue

        if args.dry_run:
            changed = unlabeled
        else:
            changed = store.attribute_unlabeled(session_id, player_id)
        total += len(changed)
        other = f", leaving {sorted(already)} alone" if already else ""
        verb = "would attribute" if args.dry_run else "attributed"
        print(f"{session_id}: {verb} swing(s) {', '.join(changed)} to {player_id}{other}")

    print(f"\n{'Would attribute' if args.dry_run else 'Attributed'} {total} swing(s).")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
